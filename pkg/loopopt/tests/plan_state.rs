//! The page's run as two jobs (the search, then the same job with `next`) and across a page reload (the worker's kept
//! work out with `state_out`, back in with `state_in`), on the val/plan p16 pool: the plan job takes the search the
//! worker keeps with what its product searches proved, so its plan does the work of one combined job and gives its
//! output; after a reload the stored main search is taken as it is (no search nodes) and both jobs answer as before.
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

/// one process running `jobs` in turn (one output line each), the state in from / out to the given files
fn run(jobs: &[&Path], state_in: Option<&Path>, state_out: Option<&Path>) -> (Vec<Value>, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_loopopt"));
    c.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global")).arg("data.json").args(jobs);
    c.env_remove("LOOPOPT_STATE_IN").env_remove("LOOPOPT_STATE_OUT");
    if let Some(p) = state_in {
        c.env("LOOPOPT_STATE_IN", p);
    }
    if let Some(p) = state_out {
        c.env("LOOPOPT_STATE_OUT", p);
    }
    let output = c.output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    (
        String::from_utf8_lossy(&output.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}
/// timings and work counters out (keys `timing_s`, `*_ms`, `work`, and the plan's counts of its own searches: a plan of
/// a pool it kept searches less)
fn strip(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(
            m.iter()
                .filter(|(k, _)| {
                    !["timing_s", "work", "umas_searched", "umas_certified"].contains(&k.as_str())
                        && !k.ends_with("_ms")
                })
                .map(|(k, x)| (k.clone(), strip(x)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(strip).collect()),
        x => x.clone(),
    }
}

#[test]
fn two_jobs_and_a_reload_plan_as_one_job() {
    let job: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap(),
    )
    .unwrap();
    let mut search = job.clone();
    search.as_object_mut().unwrap().remove("next");
    let mut plan = job.clone();
    plan["no_reports"] = json!(true);
    let dir = std::env::temp_dir();
    let tag = std::process::id();
    let [pj, ps, pp, st] =
        ["job", "search", "plan", "state"].map(|n| dir.join(format!("uma-plan-state-{tag}-{n}.json")));
    fs::write(&pj, job.to_string()).unwrap();
    fs::write(&ps, search.to_string()).unwrap();
    fs::write(&pp, plan.to_string()).unwrap();
    let (one, _) = run(&[&pj], None, None);
    let (two, _) = run(&[&ps, &pp], None, Some(&st));
    let (re, err) = run(&[&ps, &pp], Some(&st), None);
    for p in [&pj, &ps, &pp, &st] {
        fs::remove_file(p).unwrap();
    }
    let (one, two, re) = (&one[0], &two, &re);
    // the plan job: the kept search (no search of its own), the combined job's plan and the same plan work
    assert_eq!(two[1]["work"]["nodes"], 0);
    assert!(strip(&two[1]["next"]) == strip(&one["next"]), "the plan job's plan differs from the combined job's");
    assert_eq!(
        two[1]["next"]["work"]["nodes"], one["next"]["work"]["nodes"],
        "the plan job's plan does other work than the combined job's"
    );
    // after the reload: the stored search is restored and taken by both jobs, which answer as before
    assert!(err.contains("\"solve\":1"), "state_in: {err}");
    assert_eq!(re[0]["work"]["nodes"], 0, "the main search ran again after the reload");
    assert!(strip(&re[0]) == strip(&two[0]), "the search job's answer differs after the reload");
    assert!(strip(&re[1]["next"]) == strip(&one["next"]), "the plan differs after the reload");
}
