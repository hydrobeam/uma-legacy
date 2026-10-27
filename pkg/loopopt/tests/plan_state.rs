//! The page's run as two jobs (the search, then the same job with `next`) and across a page reload (the worker's kept
//! work out with `state_out`, back in with `state_in`), on the val/plan p16 pool: the plan job takes the search the
//! worker keeps with what its product searches proved, so its plan does the work of one combined job and gives its
//! output; after a reload the stored main search is taken as it is (no search nodes) and both jobs answer as before.
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

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

/// A job after a job on another pool of the same setup (val/plan p16_88, then p16, in one process) answers as p16 alone:
/// what the first job proved is kept under its pool (`KeptState::seen`), so the second takes none of it (a proof from
/// another pool could let `certified` name another career of equal value). Timings and work counts may differ.
#[test]
fn a_job_after_another_pool_answers_as_alone() {
    let read = |name: &str| -> Value {
        let path = format!("{}/../../val/plan/{name}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    };
    let (other, job) = (read("p16_88"), read("p16"));
    assert_ne!(other["legacies"], job["legacies"], "the two jobs must differ in their pools");
    let dir = std::env::temp_dir();
    let tag = std::process::id();
    let [po, pj] = ["other", "job"].map(|n| dir.join(format!("uma-cross-pool-{tag}-{n}.json")));
    fs::write(&po, other.to_string()).unwrap();
    fs::write(&pj, job.to_string()).unwrap();
    let (after, _) = run(&[&po, &pj], None, None);
    let (alone, _) = run(&[&pj], None, None);
    for p in [&po, &pj] {
        fs::remove_file(p).unwrap();
    }
    assert!(strip(&after[1]) == strip(&alone[0]), "p16 after p16_88 differs from p16 alone");
}

/// A kept plan is reused only on its own deck space (`bests_key`'s space key; that the keys differ in this case is
/// `career::tests::a_trainee_filter_that_changes_a_class_changes_the_plan_key`): the nested_nopal pool with cards
/// 30067 and 30080 as plain pals, a [1004] plan after a [1003, 1004] one searches as many careers and answers as it
/// does alone. The same plan twice does reuse (the count this reads sees reuse).
#[test]
fn a_plan_on_another_trainee_filter_reuses_none_of_its_careers() {
    let mut job: Value = serde_json::from_str(
        &fs::read_to_string(format!("{}/tests/fixtures/nested_nopal.json", env!("CARGO_MANIFEST_DIR"))).unwrap(),
    )
    .unwrap();
    job["exclude_cards"].as_array_mut().unwrap().retain(|card| card != 30067 && card != 30080);
    job["next"] = json!({});
    let dir = std::env::temp_dir();
    let tag = std::process::id();
    let [wide, narrow] = ["wide", "narrow"].map(|n| dir.join(format!("uma-plan-filter-{tag}-{n}.json")));
    job["trainees"] = json!([1003, 1004]);
    fs::write(&wide, job.to_string()).unwrap();
    job["trainees"] = json!([1004]);
    fs::write(&narrow, job.to_string()).unwrap();
    let (after, _) = run(&[&wide, &narrow], None, None);
    let (alone, _) = run(&[&narrow], None, None);
    let (twice, _) = run(&[&narrow, &narrow], None, None);
    for p in [&wide, &narrow] {
        fs::remove_file(p).unwrap();
    }
    let solves = |out: &Value| out["next"]["work"]["prof"]["solves"].as_f64().unwrap();
    assert!(solves(&twice[1]) < solves(&twice[0]), "the same plan twice reused nothing: the check reads no reuse");
    assert_eq!(solves(&after[1]), solves(&alone[0]), "the [1004] plan reused careers of the [1003, 1004] plan");
    assert!(strip(&after[1]) == strip(&alone[0]), "the [1004] plan after the [1003, 1004] plan differs from it alone");
}

/// The pair search asked only after a check's variants finished without it (astra-review-V5 #5): the plan, its check
/// with `robust: "all"` and no `pair_search`, then `robust: "next"` with it: the working state went with the last
/// variant, so `next` says why it has no pairs (`pairs_error`); `"all"` with it then runs the plan and its check again
/// and brings the pairs
#[test]
fn a_pair_search_asked_after_its_check_finished_says_why_or_runs_again() {
    let job: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap(),
    )
    .unwrap();
    let with_next = |next: Value| {
        let mut follow_up = job.clone();
        follow_up["next"] = next;
        follow_up
    };
    let dir = std::env::temp_dir();
    let tag = std::process::id();
    let jobs = [
        job.clone(),
        with_next(json!({"robust": "all"})),
        with_next(json!({"robust": "next", "pair_search": true})),
        with_next(json!({"robust": "all", "pair_search": true})),
    ];
    let paths: Vec<_> = (0..jobs.len()).map(|k| dir.join(format!("uma-late-pairs-{tag}-{k}.json"))).collect();
    for (path, job) in paths.iter().zip(&jobs) {
        fs::write(path, job.to_string()).unwrap();
    }
    let (outputs, _) = run(&paths.iter().map(PathBuf::as_path).collect::<Vec<_>>(), None, None);
    for path in &paths {
        fs::remove_file(path).unwrap();
    }
    let check = |k: usize| &outputs[k]["next"]["robust"];
    assert_eq!(check(1)["pending"], json!(0));
    assert!(check(1)["pairs"].is_null() && check(1)["pairs_error"].is_null(), "no pair search asked yet");
    assert!(check(2)["pairs"].is_null());
    assert!(check(2)["pairs_error"].is_string(), "the late request says why it has no pairs: {}", check(2));
    assert!(check(3)["pairs"]["trainees"].as_array().is_some_and(|trainees| !trainees.is_empty()), "{}", check(3));
}
