//! WebAssembly entry points. The host writes a UTF-8 JSON request into memory from `alloc`, calls `call`,
//! then reads `result_len()` bytes at the returned pointer. Requests:
//!   {"cmd":"init","data":<data.json>,"skills":<web_skills.json>}
//!   {"cmd":"deck", ...SkillJob + DeckJob}  (needs "cards" and "events" in init)
//!   {"cmd":"career", ...SkillJob + DeckJob + CareerJob}  (same)
//!   {"cmd":"state_out"}, {"cmd":"state_in","state":...}  the career planner's kept work out and back in (page reloads)
use super::*;
use super::skills::*;
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<Option<(Data, SkillsRaw)>> = const { RefCell::new(None) };
    // the card table and the job lists its event cache adds (`events` at init)
    static DECK: RefCell<Option<(super::deck::CardsRaw, super::deck::EventLists)>> = const { RefCell::new(None) };
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Allocate an uninitialized request buffer. The host writes its bytes and later releases it with `dealloc`.
/// Neither `call` nor `result_len` transfers ownership of this allocation.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len); let p = v.as_mut_ptr(); std::mem::forget(v); p
}
/// Release a request buffer allocated by this module.
///
/// # Safety
/// `p` must be a live allocation returned by `alloc(len)` in this instance, with the same `len`.
/// It must not have been freed, and no caller may access it after this call. A pointer returned by `call`
/// belongs to the output buffer and must never be passed here.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(p: *mut u8, len: usize) { unsafe { drop(Vec::from_raw_parts(p, 0, len)); } }
/// Length of the latest response, to be read together with the pointer returned by `call`.
#[unsafe(no_mangle)]
pub extern "C" fn result_len() -> usize { OUT.with(|o| o.borrow().len()) }

/// Handle a request without taking ownership of its bytes. The returned response is owned by this module;
/// copy `result_len()` bytes before the next `call`, which may overwrite or reallocate the output buffer.
///
/// # Safety
/// `p` must be non-null and valid for reading `len` initialized bytes in a single allocation, even for an empty
/// request. The bytes must be valid UTF-8 and remain unchanged for the duration of the call. The host must not
/// reenter `call` while it is running or modify the module-owned response buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call(p: *const u8, len: usize) -> *const u8 {
    let req = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(p, len)) };
    let out = handle(req);
    // (written into the last call's buffer: `Value`'s `Display` is this compact serializer, the same bytes)
    OUT.with(|o| { let mut b = o.borrow_mut(); b.clear(); serde_json::to_writer(&mut *b, &out).unwrap(); b.as_ptr() })
}

/// The init request, parsed straight into the solver's input types (no intermediate `Value` tree for the large
/// data table). `cards` stays a `Value` so a card table that does not parse only disables the deck search.
#[derive(serde::Deserialize)]
struct InitReq { data: DataRaw, skills: SkillsRaw, #[serde(default)] cards: serde_json::Value, #[serde(default)] events: serde_json::Value }

fn handle(req: &str) -> serde_json::Value {
    // the request parsed once: an init request with the page's prefix straight into `InitReq`, every other one into a
    // `Value` (an init request without the prefix then from it; on an error the text is parsed again for the message)
    let init = |r: Result<InitReq, serde_json::Error>| -> Result<InitReq, serde_json::Value> {
        r.or_else(|_| serde_json::from_str::<InitReq>(req)).map_err(|e| serde_json::json!({"error": format!("bad init request: {e}")}))
    };
    let mut v: Option<serde_json::Value> = None;
    let ir = if req.starts_with("{\"cmd\":\"init\"") { Some(init(serde_json::from_str(req))) } else {
        match serde_json::from_str::<serde_json::Value>(req) {
            Ok(x) if x.get("cmd").and_then(|c| c.as_str()) == Some("init") => Some(init(serde_json::from_value(x))),
            Ok(x) => { v = Some(x); None }
            Err(e) => return serde_json::json!({"error": format!("bad request: {e}")}),
        }
    };
    if let Some(r) = ir {
        let r = match r { Ok(r) => r, Err(e) => return e };
        let d = load_raw(r.data);
        let n = d.umas.len();
        if let Ok(cr) = <super::deck::CardsRaw as serde::Deserialize>::deserialize(&r.cards) { let ev = super::deck::event_lists(&r.skills, &r.events); DECK.with(|x| *x.borrow_mut() = Some((cr, ev))); }
        STATE.with(|s| *s.borrow_mut() = Some((d, r.skills)));
        super::deck::forget_kept_decks();
        return serde_json::json!({"ok": true, "umas": n});
    }
    let mut v = v.unwrap();
    let cmd = v["cmd"].as_str().unwrap_or("").to_owned();
    match cmd.as_str() {
        "deck" => STATE.with(|s| DECK.with(|dk| {
            let st = s.borrow(); let Some((d, sk)) = st.as_ref() else { return serde_json::json!({"error": "not initialised"}) };
            let dkb = dk.borrow(); let Some((cr, ev)) = dkb.as_ref() else { return serde_json::json!({"error": "no card data"}) };
            super::deck::apply_event_lists(&mut v, ev);
            super::deck::run_deck(d, sk, cr, &v)
        })),
        // state_out / state_in read no events
        "career" | "state_out" | "state_in" => STATE.with(|s| DECK.with(|dk| {
            let st = s.borrow(); let Some((d, sk)) = st.as_ref() else { return serde_json::json!({"error": "not initialised"}) };
            let dkb = dk.borrow(); let Some((cr, ev)) = dkb.as_ref() else { return serde_json::json!({"error": "no card data"}) };
            // (a carried roll's plan job too: its kept roll state is found by the plan job as it ran, events applied)
            if cmd == "career" { super::deck::apply_event_lists(&mut v, ev); if v["roll"]["job"].is_object() { super::deck::apply_event_lists(&mut v["roll"]["job"], ev); } }
            super::deck::run_career(d, sk, cr, &v)
        })),
        c => serde_json::json!({"error": format!("unknown command {c}")}),
    }
}
