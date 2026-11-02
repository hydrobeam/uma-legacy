//! The career planner's memos (per Cont, by the bits of p) return what the functions compute, through a refit of Cont
//! as well: a small cut of the val/plan p11_noagnes pool (three veterans, three targets, Curren Chan, King Halo and
//! Yamanin Zephyr owned) where the pass-2 searches change a candidate's best builders and index_rule fits Cont again
//! (new Conts, new memos), and the refitted Conts look up odds the old ones had memoised. With `PLAN_MEMO_CHECK` every
//! hit is computed again and compared bit for bit (the plan panics on a stale value; a memo carried across the refit
//! fails here, while the output alone need not show it), with the cap lifted (`PLAN_MEMO_CAP`) so clearing cannot hide
//! stale entries. The output with the memos on must equal the output with them off (`PLAN_MEMO_OFF`) but for timings.
use crate::common;
use common::{State, loopopt, run_with, val_cut, without_clocks};
use serde_json::Value;

/// `job`'s output and stderr, with the plan's memos on (`PLAN_MEMO`) and checked (`PLAN_MEMO_CHECK`) or `off`
fn run(job: &Value, off: bool) -> (Value, String) {
    let mut command = loopopt("global", State::NONE);
    command.env("PLAN_MEMO", "1");
    // (memos on: no cap in practice, so entries made before the refit are still there to be read after it)
    if off {
        command.env("PLAN_MEMO_OFF", "1").env_remove("PLAN_MEMO_CHECK");
    } else {
        command.env_remove("PLAN_MEMO_OFF").env("PLAN_MEMO_CHECK", "1").env("PLAN_MEMO_CAP", "100000000");
    }
    let ran = run_with(command, &format!("plan-memo-{}", u8::from(off)), &[job]);
    (ran.outputs.into_iter().next().expect("one output"), ran.stderr)
}
/// a number after `key ` in the PLAN_MEMO line
fn stat(err: &str, key: &str) -> u64 {
    let l = err.lines().find(|l| l.starts_with("PLAN_MEMO")).expect("no PLAN_MEMO line");
    let i = l.find(&format!("{key} ")).unwrap() + key.len() + 1;
    l.get(i..).unwrap().split_whitespace().next().unwrap().parse().unwrap()
}

#[test]
fn memos_exact_through_a_refit() {
    let job = val_cut("plan/p11_noagnes.json", &[0, 6, 10], &[5, 6, 7], &[1038, 1061, 1078]);
    let (on, err_on) = run(&job, false);
    let (off, err_off) = run(&job, true);
    // the refit path ran, with the memos looked up (and hit) in one run and not at all in the other
    assert!(stat(&err_on, "refits") >= 1, "no refit: {err_on}");
    assert!(stat(&err_on, "hits") > 0, "{err_on}");
    assert_eq!(stat(&err_off, "lookups"), 0, "{err_off}");
    assert!(on["next"]["v0"].as_f64().is_some());
    assert!(without_clocks(&on) == without_clocks(&off), "outputs differ with the memos on and off");
}
