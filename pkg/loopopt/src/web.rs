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

#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len); let p = v.as_mut_ptr(); std::mem::forget(v); p
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(p: *mut u8, len: usize) { unsafe { drop(Vec::from_raw_parts(p, 0, len)); } }
#[unsafe(no_mangle)]
pub extern "C" fn result_len() -> usize { OUT.with(|o| o.borrow().len()) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn call(p: *const u8, len: usize) -> *const u8 {
    let req = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(p, len)) };
    let out = handle(req).to_string();
    OUT.with(|o| { *o.borrow_mut() = out.into_bytes(); o.borrow().as_ptr() })
}

/// The init request, parsed straight into the solver's input types (no intermediate `Value` tree for the large
/// data table). `cards` stays a `Value` so a card table that does not parse only disables the deck search.
#[derive(serde::Deserialize)]
struct InitReq { data: DataRaw, skills: SkillsRaw, #[serde(default)] cards: serde_json::Value, #[serde(default)] events: serde_json::Value }
#[derive(serde::Deserialize)]
struct Head { #[serde(default)] cmd: String }

fn handle(req: &str) -> serde_json::Value {
    if req.starts_with("{\"cmd\":\"init\"") || matches!(serde_json::from_str::<Head>(req), Ok(h) if h.cmd == "init") {
        let r: InitReq = match serde_json::from_str(req) { Ok(r) => r, Err(e) => return serde_json::json!({"error": format!("bad init request: {e}")}) };
        let d = load_raw(r.data);
        let n = d.umas.len();
        if let Ok(cr) = <super::deck::CardsRaw as serde::Deserialize>::deserialize(&r.cards) { let ev = super::deck::event_lists(&r.skills, &r.events); DECK.with(|x| *x.borrow_mut() = Some((cr, ev))); }
        STATE.with(|s| *s.borrow_mut() = Some((d, r.skills)));
        super::deck::forget_kept_decks();
        return serde_json::json!({"ok": true, "umas": n});
    }
    let mut v: serde_json::Value = match serde_json::from_str(req) { Ok(v) => v, Err(e) => return serde_json::json!({"error": format!("bad request: {e}")}) };
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
            if cmd == "career" { super::deck::apply_event_lists(&mut v, ev); }
            super::deck::run_career(d, sk, cr, &v)
        })),
        c => serde_json::json!({"error": format!("unknown command {c}")}),
    }
}
