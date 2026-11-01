//! The career planner's memos (per Cont, by the bits of p) return what the functions compute, through a refit of Cont
//! as well: the val/plan p11_noagnes pool (p11 itself no longer refits since the stage candidates are ranked by the
//! main search and the full-loop ceiling) with its pass-1 builders left to their proxies (`next.proxy_eff` -1: none searched
//! there), so the pass-2 searches change a candidate's best builders and index_rule fits Cont again (new Conts, new
//! memos). With `PLAN_MEMO_CHECK` every hit is computed
//! again and compared bit for bit (the plan panics on a stale value; a memo carried across the refit fails here, while
//! the output alone need not show it), with the cap lifted (`PLAN_MEMO_CAP`) so clearing cannot hide stale entries.
//! The output with the memos on must equal the output with them off (`PLAN_MEMO_OFF`) but for timings.
mod common;
use common::{State, loopopt, run_with, val_job, without_clocks};
use serde_json::{Value, json};

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
    let mut job = val_job("plan/p11_noagnes.json");
    // (`builder_careers` 1: with the default 3 the alternative rows' lifts already give Cont its exact builders, and no
    // refit follows the pass-2 searches)
    job["next"] = json!({"proxy_eff": -1.0, "builder_careers": 1});
    let (on, err_on) = run(&job, false);
    let (off, err_off) = run(&job, true);
    // the refit path ran, with the memos looked up (and hit) in one run and not at all in the other
    assert!(stat(&err_on, "refits") >= 1, "no refit: {err_on}");
    assert!(stat(&err_on, "hits") > 0, "{err_on}");
    assert_eq!(stat(&err_off, "lookups"), 0, "{err_off}");
    assert!(on["next"]["v0"].as_f64().is_some());
    assert!(without_clocks(&on) == without_clocks(&off), "outputs differ with the memos on and off");
}
