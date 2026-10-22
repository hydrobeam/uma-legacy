//! WebAssembly entry points. The host writes a UTF-8 JSON request into memory from `alloc`, calls `call`,
//! then reads `result_len()` bytes at the returned pointer. Requests:
//!   {"cmd":"init","data":<data.json>,"skills":<web_skills.json>}
//!   {"cmd":"deck", ...SkillJob + DeckJob}  (needs "cards" and "events" in init)
//!   {"cmd":"career", ...SkillJob + DeckJob + CareerJob}  (same)
//!   {"cmd":"state_out"}, {"cmd":"state_in","state":...}  the career planner's kept work out and back in (page reloads)
#![allow(unsafe_code, reason = "the wasm exports take the host's pointers: `alloc`/`dealloc` and `call`")]
use crate::skills::SkillsRaw;
use crate::{Data, DataRaw, load_raw};
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<Option<(Data, SkillsRaw)>> = const { RefCell::new(None) };
    // the card table and the job lists its event cache adds (`events` at init)
    static DECK: RefCell<Option<(crate::deck::CardsRaw, crate::deck::sources::EventLists)>> = const { RefCell::new(None) };
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Allocate an uninitialized request buffer. The host writes its bytes and later releases it with `dealloc`.
/// Neither `call` nor `result_len` transfers ownership of this allocation.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let pointer = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    pointer
}
/// Release a request buffer allocated by this module.
///
/// # Safety
/// `p` must be a live allocation returned by `alloc(len)` in this instance, with the same `len`.
/// It must not have been freed, and no caller may access it after this call. A pointer returned by `call`
/// belongs to the output buffer and must never be passed here.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(p: *mut u8, len: usize) {
    // SAFETY: the caller guarantees `p` came from `alloc(len)` in this instance and is still live, so it is a
    // `Vec<u8>` allocation of capacity `len`; length 0 means no element is read or dropped.
    unsafe {
        drop(Vec::from_raw_parts(p, 0, len));
    }
}
/// Length of the latest response, to be read together with the pointer returned by `call`.
#[unsafe(no_mangle)]
pub extern "C" fn result_len() -> usize {
    OUT.with(|o| o.borrow().len())
}

/// Handle a request without taking ownership of its bytes. The returned response is owned by this module;
/// copy `result_len()` bytes before the next `call`, which may overwrite or reallocate the output buffer.
///
/// # Safety
/// `p` must be non-null and valid for reading `len` initialized bytes in a single allocation, even for an empty
/// request. The bytes must be valid UTF-8 and remain unchanged for the duration of the call. The host must not
/// reenter `call` while it is running or modify the module-owned response buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call(p: *const u8, len: usize) -> *const u8 {
    // SAFETY: the caller guarantees `p` is valid for reading `len` initialized bytes of UTF-8 that stay unchanged
    // during the call (`# Safety` above); `req` does not outlive it.
    let req = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(p, len)) };
    let out = handle(req);
    // (written into the last call's buffer: `Value`'s `Display` is this compact serializer, the same bytes)
    OUT.with(|output| {
        let mut buffer = output.borrow_mut();
        buffer.clear();
        serde_json::to_writer(&mut *buffer, &out).unwrap();
        buffer.as_ptr()
    })
}

/// The init request, parsed straight into the solver's input types (no intermediate `Value` tree for the large
/// data table). `cards` stays a `Value`: a card table that does not parse leaves `DECK` unchanged,
/// so an initial request can still load the schedule data without card data.
#[derive(serde::Deserialize)]
struct InitReq {
    data: DataRaw,
    skills: SkillsRaw,
    #[serde(default)]
    cards: serde_json::Value,
    #[serde(default)]
    events: serde_json::Value,
}

fn handle(req: &str) -> serde_json::Value {
    // the request parsed once: an init request with the page's prefix straight into `InitReq`, every other one into a
    // `Value` (an init request without the prefix then from it; on an error the text is parsed again for the message)
    let init = |parsed: Result<InitReq, serde_json::Error>| -> Result<InitReq, serde_json::Value> {
        parsed
            .or_else(|_| serde_json::from_str::<InitReq>(req))
            .map_err(|e| serde_json::json!({"error": format!("bad init request: {e}")}))
    };
    let mut job: Option<serde_json::Value> = None;
    let initialization = if req.starts_with("{\"cmd\":\"init\"") {
        Some(init(serde_json::from_str(req)))
    } else {
        match serde_json::from_str::<serde_json::Value>(req) {
            Ok(value) if value.get("cmd").and_then(|c| c.as_str()) == Some("init") => {
                Some(init(serde_json::from_value(value)))
            }
            Ok(value) => {
                job = Some(value);
                None
            }
            Err(e) => return serde_json::json!({"error": format!("bad request: {e}")}),
        }
    };
    if let Some(initialization) = initialization {
        let request = match initialization {
            Ok(request) => request,
            Err(error) => return error,
        };
        let data = load_raw(request.data);
        let uma_count = data.umas.len();
        if let Ok(cards) = <crate::deck::CardsRaw as serde::Deserialize>::deserialize(&request.cards) {
            let events = crate::deck::sources::event_lists(&request.skills, &request.events);
            DECK.with(|deck| *deck.borrow_mut() = Some((cards, events)));
        }
        STATE.with(|state| *state.borrow_mut() = Some((data, request.skills)));
        crate::career::forget_kept_decks();
        return serde_json::json!({"ok": true, "umas": uma_count});
    }
    let mut job = job.unwrap();
    let cmd = job["cmd"].as_str().unwrap_or("").to_owned();
    match cmd.as_str() {
        "deck" => STATE.with(|s| {
            DECK.with(|dk| {
                let st = s.borrow();
                let Some((d, sk)) = st.as_ref() else { return serde_json::json!({"error": "not initialised"}) };
                let dkb = dk.borrow();
                let Some((cr, ev)) = dkb.as_ref() else { return serde_json::json!({"error": "no card data"}) };
                crate::deck::sources::apply_event_lists(&mut job, ev);
                crate::deck::loops::run_deck(d, sk, cr, &job)
            })
        }),
        // state_out / state_in read no events
        "career" | "state_out" | "state_in" => STATE.with(|s| {
            DECK.with(|dk| {
                let st = s.borrow();
                let Some((d, sk)) = st.as_ref() else { return serde_json::json!({"error": "not initialised"}) };
                let dkb = dk.borrow();
                let Some((cr, ev)) = dkb.as_ref() else { return serde_json::json!({"error": "no card data"}) };
                // (a carried roll's plan job too: its kept roll state is found by the plan job as it ran, events applied)
                if cmd == "career" {
                    crate::deck::sources::apply_event_lists(&mut job, ev);
                    if job["roll"]["job"].is_object() {
                        crate::deck::sources::apply_event_lists(&mut job["roll"]["job"], ev);
                    }
                }
                crate::career::run_career(d, sk, cr, &job)
            })
        }),
        c => serde_json::json!({"error": format!("unknown command {c}")}),
    }
}
