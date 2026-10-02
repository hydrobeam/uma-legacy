//! WebAssembly entry points. The host writes a UTF-8 JSON request into memory from `alloc`, calls `call`,
//! then reads `result_len()` bytes at the returned pointer. Requests:
//!   {"cmd":"init","data":<data.json>,"skills":<web_skills.json>}
//!   {"cmd":"skills", ...SkillJob}   {"cmd":"affinity", ...AffinityJob}
//!   {"cmd":"deck", ...SkillJob + DeckJob}  (needs "cards" and "events" in init)
use super::*;
use super::skills::*;
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<Option<(Data, SkillsRaw)>> = const { RefCell::new(None) };
    static DECK: RefCell<Option<(super::deck::CardsRaw, serde_json::Value)>> = const { RefCell::new(None) };
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

fn handle(req: &str) -> serde_json::Value {
    let v: serde_json::Value = match serde_json::from_str(req) { Ok(v) => v, Err(e) => return serde_json::json!({"error": format!("bad request: {e}")}) };
    match v["cmd"].as_str().unwrap_or("") {
        "init" => {
            let d = load_str(&v["data"].to_string());
            let sk: SkillsRaw = match serde_json::from_value(v["skills"].clone()) { Ok(s) => s, Err(e) => return serde_json::json!({"error": format!("bad skills table: {e}")}) };
            let n = d.umas.len();
            if let Ok(cr) = serde_json::from_value::<super::deck::CardsRaw>(v["cards"].clone()) { DECK.with(|x| *x.borrow_mut() = Some((cr, v["events"].clone()))); }
            STATE.with(|s| *s.borrow_mut() = Some((d, sk)));
            serde_json::json!({"ok": true, "umas": n})
        }
        "skills" => STATE.with(|s| {
            let st = s.borrow(); let Some((d, sk)) = st.as_ref() else { return serde_json::json!({"error": "not initialised"}) };
            match serde_json::from_value::<SkillJob>(v.clone()) { Ok(job) => run_skills(d, sk, &job), Err(e) => serde_json::json!({"error": format!("bad job: {e}")}) }
        }),
        "affinity" => STATE.with(|s| {
            let st = s.borrow(); let Some((d, _)) = st.as_ref() else { return serde_json::json!({"error": "not initialised"}) };
            match serde_json::from_value::<AffinityJob>(v.clone()) { Ok(job) => run_affinity(d, &job), Err(e) => serde_json::json!({"error": format!("bad job: {e}")}) }
        }),
        "deck" => STATE.with(|s| DECK.with(|dk| {
            let st = s.borrow(); let Some((d, sk)) = st.as_ref() else { return serde_json::json!({"error": "not initialised"}) };
            let dkb = dk.borrow(); let Some((cr, db)) = dkb.as_ref() else { return serde_json::json!({"error": "no card data"}) };
            let mut job = v.clone();
            super::deck::apply_events_db(&mut job, sk, db);
            super::deck::run_deck(d, sk, cr, &job)
        })),
        c => serde_json::json!({"error": format!("unknown command {c}")}),
    }
}
