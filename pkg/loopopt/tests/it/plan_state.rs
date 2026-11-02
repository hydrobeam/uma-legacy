//! The page's run as two jobs (the search, then the same job with `next`) and across a page reload (the worker's kept
//! work out with `state_out`, back in with `state_in`), on the val/plan p16 pool: the plan job takes the search the
//! worker keeps with what its product searches proved, so its plan does the work of one combined job and gives its
//! output; after a reload the stored main search is taken as it is (no search nodes) and both jobs answer as before.
mod common;
use common::{Ran, State, TempFile, check_of, loopopt, run_with, val_job, without_work};
use serde_json::{Value, json};

/// one process running `jobs` in turn (one output line each), with `state`'s files
fn run(tag: &str, jobs: &[&Value], state: State) -> Ran {
    run_with(loopopt("global", state), tag, jobs)
}

#[test]
fn two_jobs_and_a_reload_plan_as_one_job() {
    let job = val_job("plan/p16.json");
    let mut search = job.clone();
    search["job"]["career"].as_object_mut().unwrap().remove("next");
    let mut plan = job.clone();
    plan["job"]["career"]["no_reports"] = json!(true);
    let state = TempFile::new("plan-state");
    let one = run("one", &[&job], State::NONE).outputs;
    let two = run("two", &[&search, &plan], State { load: None, save: Some(state.path()) }).outputs;
    let Ran { outputs: re, stderr: err } =
        run("reload", &[&search, &plan], State { load: Some(state.path()), save: None });
    let (one, two, re) = (&one[0], &two, &re);
    // the plan job: the kept search (no search of its own), the combined job's plan and the same plan work
    assert_eq!(two[1]["work"]["nodes"], 0);
    assert!(
        without_work(&two[1]["next"]) == without_work(&one["next"]),
        "the plan job's plan differs from the combined job's"
    );
    assert_eq!(
        two[1]["next"]["work"]["nodes"], one["next"]["work"]["nodes"],
        "the plan job's plan does other work than the combined job's"
    );
    // after the reload: the stored search is restored and taken by both jobs, which answer as before
    assert!(err.contains("\"solve\":1"), "state_in: {err}");
    assert_eq!(re[0]["work"]["nodes"], 0, "the main search ran again after the reload");
    assert!(without_work(&re[0]) == without_work(&two[0]), "the search job's answer differs after the reload");
    assert!(without_work(&re[1]["next"]) == without_work(&one["next"]), "the plan differs after the reload");
}

/// A job after a job on another pool of the same setup (val/plan p16_88, then p16, in one process) answers as p16 alone:
/// what the first job proved is kept under its pool (`KeptState::seen`), so the second takes none of it (a proof from
/// another pool could let `certified` name another career of equal value). Timings and work counts may differ.
#[test]
fn a_job_after_another_pool_answers_as_alone() {
    let (other, job) = (val_job("plan/p16_88.json"), val_job("plan/p16.json"));
    assert_ne!(
        other["job"]["career"]["legacies"], job["job"]["career"]["legacies"],
        "the two jobs must differ in their pools"
    );
    let after = run("cross-pool-after", &[&other, &job], State::NONE).outputs;
    let alone = run("cross-pool-alone", &[&job], State::NONE).outputs;
    assert!(without_work(&after[1]) == without_work(&alone[0]), "p16 after p16_88 differs from p16 alone");
}

/// A kept plan is reused only on its own deck space (`bests_key`'s space key; that the keys differ in this case is
/// `career::tests::a_trainee_filter_that_changes_a_class_changes_the_plan_key`): the nested_nopal pool with cards
/// 30067 and 30080 as plain pals, a [1004] plan after a [1003, 1004] one searches as many careers and answers as it
/// does alone. The same plan twice does reuse (the count this reads sees reuse).
#[test]
fn a_plan_on_another_trainee_filter_reuses_none_of_its_careers() {
    let path = format!("{}/tests/fixtures/nested_nopal.json", env!("CARGO_MANIFEST_DIR"));
    let mut job: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    job["job"]["deck"]["exclude_cards"].as_array_mut().unwrap().retain(|card| card != 30067 && card != 30080);
    job["job"]["career"]["next"] = json!({});
    job["job"]["career"]["trainees"] = json!([1003, 1004]);
    let wide = job.clone();
    job["job"]["career"]["trainees"] = json!([1004]);
    let narrow = job;
    let after = run("filter-after", &[&wide, &narrow], State::NONE).outputs;
    let alone = run("filter-alone", &[&narrow], State::NONE).outputs;
    let twice = run("filter-twice", &[&narrow, &narrow], State::NONE).outputs;
    let solves = |out: &Value| out["next"]["work"]["prof"]["solves"].as_f64().unwrap();
    assert!(solves(&twice[1]) < solves(&twice[0]), "the same plan twice reused nothing: the check reads no reuse");
    assert_eq!(solves(&after[1]), solves(&alone[0]), "the [1004] plan reused careers of the [1003, 1004] plan");
    assert!(
        without_work(&after[1]) == without_work(&alone[0]),
        "the [1004] plan after the [1003, 1004] plan differs from it alone"
    );
}

/// The pair search asked only after a check's variants finished without it (astra-review-V5 #5): the plan, its check
/// with `robust: "all"` and no `pair_search`, then `robust: "next"` with it: the working state went with the last
/// variant, so `next` says why it has no pairs (`pairs_error`); `"all"` with it then runs the plan and its check again
/// and brings the pairs
#[test]
fn a_pair_search_asked_after_its_check_finished_says_why_or_runs_again() {
    let job = val_job("plan/p16.json");
    let with_next = |next: Value| {
        let mut follow_up = job.clone();
        follow_up["job"]["career"]["next"] = next;
        follow_up
    };
    let jobs = [
        job.clone(),
        with_next(json!({"robust": "all"})),
        with_next(json!({"robust": "next", "pair_search": true})),
        with_next(json!({"robust": "all", "pair_search": true})),
    ];
    let outputs = run_with(loopopt("global", State::NONE), "late-pairs", &jobs).outputs;
    let check = |k: usize| check_of(&outputs[k]);
    assert_eq!(check(1)["pending"], json!(0));
    assert!(check(1)["pairs"].is_null() && check(1)["pairs_error"].is_null(), "no pair search asked yet");
    assert!(check(2)["pairs"].is_null());
    assert!(check(2)["pairs_error"].is_string(), "the late request says why it has no pairs: {}", check(2));
    assert!(check(3)["pairs"]["trainees"].as_array().is_some_and(|trainees| !trainees.is_empty()), "{}", check(3));
}
