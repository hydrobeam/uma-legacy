//! WebAssembly entry points. The host writes a UTF-8 JSON request into memory from `alloc`, calls `init` (once, with
//! the era's data) or `call` (a job), then reads `result_len()` bytes at the returned pointer. Requests:
//!   `init`: {"data":<data.json>,"skills":<web_skills.json>,"cards":<web_cards.json>,"events":<card_events.json>}
//!   `call`: {"cmd":"deck", ...SkillJob + DeckJob}  (needs "cards" and "events" in init)
//!           {"cmd":"career", ...SkillJob + DeckJob + CareerJob}  (same)
//!           {"cmd":"state_out"}, {"cmd":"state_in","state":...}  the career planner's kept work out and back in
//!           (page reloads)
#![allow(unsafe_code, reason = "the wasm exports take the host's pointers: `alloc`/`dealloc`, `init` and `call`")]
use crate::career::reply::{CareerAnswer, CareerReply};
use crate::data::{Data, DataRaw, load_raw};
use crate::refusal::Refusal;
use crate::skills::SkillsRaw;
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
    OUT.with(|output| output.borrow().len())
}

/// Load the era's data (`InitReq`) without taking ownership of its bytes; the response is read as after `call`.
///
/// # Safety
/// As for `call`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init(request_bytes: *const u8, len: usize) -> *const u8 {
    // SAFETY: the caller guarantees `request_bytes` is valid for reading `len` initialized bytes of UTF-8 that stay
    // unchanged during the call (`call`'s `# Safety`); `request_text` does not outlive it.
    let request_text = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(request_bytes, len)) };
    respond(&load(request_text))
}

/// Handle a job request without taking ownership of its bytes. The returned response is owned by this module;
/// copy `result_len()` bytes before the next `init` or `call`, which may overwrite or reallocate the output buffer.
///
/// # Safety
/// `request_bytes` must be non-null and valid for reading `len` initialized bytes in a single allocation, even for an
/// empty request. The bytes must be valid UTF-8 and remain unchanged for the duration of the call. The host must not
/// reenter `init` or `call` while either is running or modify the module-owned response buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call(request_bytes: *const u8, len: usize) -> *const u8 {
    // SAFETY: the caller guarantees `request_bytes` is valid for reading `len` initialized bytes of UTF-8 that stay
    // unchanged during the call (`# Safety` above); `request_text` does not outlive it.
    let request_text = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(request_bytes, len)) };
    handle(request_text);
    OUT.with(|output| output.borrow().as_ptr())
}

/// `reply` written into the output buffer, which `result_len` measures; its start.
fn respond(reply: &impl serde::Serialize) -> *const u8 {
    OUT.with(|output| {
        let mut buffer = output.borrow_mut();
        buffer.clear();
        serde_json::to_writer(&mut *buffer, reply).unwrap();
        buffer.as_ptr()
    })
}

/// The init request, parsed straight into the solver's input types (no intermediate `Value` tree for the large data
/// table: d97c26d measured init 42 -> 13 ms on Global). `cards` stays a `Value`: a card table that does not parse
/// leaves `DECK` unchanged, so an initial request can still load the schedule data without card data.
#[derive(serde::Deserialize)]
struct InitReq {
    data: DataRaw,
    skills: SkillsRaw,
    #[serde(default)]
    cards: serde_json::Value,
    #[serde(default)]
    events: serde_json::Value,
}

/// `init`'s work: the era's data kept for the jobs; answers `{"ok":true,"umas":n}` or an error.
fn load(request_text: &str) -> serde_json::Value {
    let request = match serde_json::from_str::<InitReq>(request_text) {
        Ok(request) => request,
        Err(e) => return serde_json::json!({"error": format!("bad init request: {e}")}),
    };
    let data = load_raw(request.data);
    let uma_count = data.umas.len();
    if let Ok(cards) = <crate::deck::CardsRaw as serde::Deserialize>::deserialize(&request.cards) {
        let events = crate::deck::sources::event_lists(&request.skills, &request.events);
        DECK.with(|deck| *deck.borrow_mut() = Some((cards, events)));
    }
    STATE.with(|state| *state.borrow_mut() = Some((data, request.skills)));
    crate::career::forget_kept_decks();
    serde_json::json!({"ok": true, "umas": uma_count})
}

/// `call`'s work: one job run on the data `init` kept, its reply written into the output buffer (`respond`; a reply
/// may borrow the data, so it is written where the data is borrowed).
fn handle(request_text: &str) {
    let mut job = match serde_json::from_str::<serde_json::Value>(request_text) {
        Ok(job) => job,
        Err(e) => {
            respond(&Refusal::new(format!("bad request: {e}")));
            return;
        }
    };
    let cmd = job["cmd"].as_str().unwrap_or("").to_owned();
    match cmd.as_str() {
        "deck" => STATE.with(|stored_state| {
            DECK.with(|stored_deck| {
                let stored_data = stored_state.borrow();
                let Some((data, skills)) = stored_data.as_ref() else {
                    return respond(&Refusal::new("not initialised"));
                };
                let stored_cards = stored_deck.borrow();
                let Some((cards, events)) = stored_cards.as_ref() else {
                    return respond(&Refusal::new("no card data"));
                };
                crate::deck::sources::apply_event_lists(&mut job, events);
                respond(&crate::loop_planner::job::run_deck(data, skills, cards, &job))
            })
        }),
        // state_out / state_in read no events
        "career" | "state_out" | "state_in" => STATE.with(|stored_state| {
            DECK.with(|stored_deck| {
                let stored_data = stored_state.borrow();
                let Some((data, skills)) = stored_data.as_ref() else {
                    return respond(&Refusal::new("not initialised"));
                };
                let stored_cards = stored_deck.borrow();
                let Some((cards, events)) = stored_cards.as_ref() else {
                    return respond(&Refusal::new("no card data"));
                };
                // (a carried roll's plan job too: its kept roll state is found by the plan job as it ran, events applied)
                if cmd == "career" {
                    crate::deck::sources::apply_event_lists(&mut job, events);
                    if job["roll"]["job"].is_object() {
                        crate::deck::sources::apply_event_lists(&mut job["roll"]["job"], events);
                    }
                }
                respond(&crate::career::state::KEPT.with(|kept| match cmd.as_str() {
                    "state_out" => {
                        CareerReply::Answer(CareerAnswer::StateOut(crate::career::state::state_out(&kept.borrow())))
                    }
                    "state_in" => {
                        let state_in = crate::career::state::state_in(&mut kept.borrow_mut(), &job["state"]);
                        CareerReply::Answer(CareerAnswer::StateIn(state_in))
                    }
                    _ => crate::career::run_career_in(data, skills, cards, &job, &mut kept.borrow_mut()),
                }))
            })
        }),
        other => respond(&Refusal::new(format!("unknown command {other}"))),
    };
}
