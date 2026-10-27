//! The career planner's memos (per Cont, by the bits of p) return what the functions compute, through a refit of Cont
//! as well: the val/plan p11_noagnes pool (p11 itself no longer refits since the stage candidates are ranked by the
//! main search and the full-loop ceiling) with its pass-1 builders left to their proxies (`next.proxy_eff` -1: none searched
//! there), so the pass-2 searches change a candidate's best builders and index_rule fits Cont again (new Conts, new
//! memos). With `PLAN_MEMO_CHECK` every hit is computed
//! again and compared bit for bit (the plan panics on a stale value; a memo carried across the refit fails here, while
//! the output alone need not show it), with the cap lifted (`PLAN_MEMO_CAP`) so clearing cannot hide stale entries.
//! The output with the memos on must equal the output with them off (`PLAN_MEMO_OFF`) but for timings.
use serde_json::{Value, json};
use std::{fs, process::Command};

fn run(job: &Value, off: bool) -> (Value, String) {
    let path = std::env::temp_dir().join(format!("uma-plan-memo-{}-{}.json", std::process::id(), u8::from(off)));
    fs::write(&path, job.to_string()).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_loopopt"));
    c.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .args(["data.json", path.to_str().unwrap()])
        .env("PLAN_MEMO", "1");
    // (memos on: no cap in practice, so entries made before the refit are still there to be read after it)
    if off {
        c.env("PLAN_MEMO_OFF", "1").env_remove("PLAN_MEMO_CHECK");
    } else {
        c.env_remove("PLAN_MEMO_OFF").env("PLAN_MEMO_CHECK", "1").env("PLAN_MEMO_CAP", "100000000");
    }
    let output = c.output().unwrap();
    fs::remove_file(path).unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    (serde_json::from_slice(&output.stdout).unwrap(), String::from_utf8_lossy(&output.stderr).into_owned())
}
/// timings out (keys `timing_s` and `*_ms`)
fn strip(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.retain(|k, _| k != "timing_s" && !k.ends_with("_ms"));
            for x in m.values_mut() {
                strip(x);
            }
        }
        Value::Array(a) => {
            for x in a {
                strip(x);
            }
        }
        _ => {}
    }
}
/// a number after `key ` in the PLAN_MEMO line
fn stat(err: &str, key: &str) -> u64 {
    let l = err.lines().find(|l| l.starts_with("PLAN_MEMO")).expect("no PLAN_MEMO line");
    let i = l.find(&format!("{key} ")).unwrap() + key.len() + 1;
    l.get(i..).unwrap().split_whitespace().next().unwrap().parse().unwrap()
}

#[test]
fn memos_exact_through_a_refit() {
    let mut job: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p11_noagnes.json")).unwrap(),
    )
    .unwrap();
    // (`builder_careers` 1: with the default 3 the alternative rows' lifts already give Cont its exact builders, and no
    // refit follows the pass-2 searches)
    job["next"] = json!({"proxy_eff": -1.0, "builder_careers": 1});
    let (mut on, err_on) = run(&job, false);
    let (mut off, err_off) = run(&job, true);
    // the refit path ran, with the memos looked up (and hit) in one run and not at all in the other
    assert!(stat(&err_on, "refits") >= 1, "no refit: {err_on}");
    assert!(stat(&err_on, "hits") > 0, "{err_on}");
    assert_eq!(stat(&err_off, "lookups"), 0, "{err_off}");
    assert!(on["next"]["v0"].as_f64().is_some());
    strip(&mut on);
    strip(&mut off);
    assert!(on == off, "outputs differ with the memos on and off");
}
