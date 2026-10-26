//! The policy simulator's goal "looped" on a constructed pool (docs/SOLVER.md *Policy simulator*): val/jobs/career_U's
//! Oguri Cap, Agnes Digital, Tamamo Cross and Inari One veterans, two targets (the scenario's gold 20049 and 20166),
//! only Oguri Cap carrying 20166, so she is the one full veteran and no career is looped at the start. Greedy's best
//! careers land both targets about half the time. Under goal "looped" the first careers do not finish: their veterans
//! are drawn from every landed set, so a full veteran arrives within a few careers; greedy then runs the best looped
//! career (on her and Oguri Cap), with no higher odds than the pick it replaces, and those careers finish: S falls, and the run stops on
//! the tail S / p. Under goal "first" the same pool runs its first pick to the end. Stopped while its career still
//! does not finish (before the switch), the looped run has run no finishing career and is censored.
use serde_json::{Value, json};
use std::{fs, process::Command};

const TARGET_ONE_VETERAN_LACKS: u64 = 20166;

/// The constructed pool's simulation under `goal`, stopped at `max_careers`.
fn simulate(goal: &str, max_careers: usize) -> Value {
    let career_job: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/jobs/career_U.json")).unwrap(),
    )
    .unwrap();
    let mut legacies: Vec<Value> = [0, 3, 6, 9].iter().map(|&i| career_job["legacies"][i].clone()).collect();
    for legacy in &mut legacies[1..] {
        legacy["sparks"].as_array_mut().unwrap().retain(|spark| spark[0] != TARGET_ONE_VETERAN_LACKS);
    }
    let mut job = career_job;
    job["targets"] = json!([{"group": 20049, "guaranteed": 2}, {"group": TARGET_ONE_VETERAN_LACKS}]);
    job["legacies"] = json!(legacies);
    job["simulate"] =
        json!({"policy": "greedy", "goal": goal, "seed": 1, "runs": 1, "max_careers": max_careers, "margin": 0});
    let path = std::env::temp_dir().join(format!("uma-sim-looped-{goal}-{max_careers}-{}.json", std::process::id()));
    fs::write(&path, job.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_loopopt"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .args(["data.json", path.to_str().unwrap()])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}

/// A pick key's parents ("chara:a,b").
fn parents(pick: &Value) -> (usize, usize) {
    let (_, pair) = pick.as_str().unwrap().split_once(':').unwrap();
    let (first, second) = pair.split_once(',').unwrap();
    (first.parse().unwrap(), second.parse().unwrap())
}

#[test]
fn sim_looped_goal_on_a_constructed_pool() {
    let initial_pool = 4;
    let looped = simulate("looped", 40);
    let run = &looped["runs"][0];
    let segs = run["segs"].as_array().unwrap();
    // the first pick is not looped (one full veteran); greedy switches to a career on a drawn veteran before any
    // career finishes (S still 1)
    assert_eq!(segs[0]["t"], 0, "{run}");
    let switch = segs.iter().find(|seg| parents(&seg["pick"]).1 >= initial_pool).expect("no switch to a drawn veteran");
    let switched_at = switch["t"].as_u64().unwrap() as usize;
    assert!(switched_at > 0 && switch["s"] == 1.0, "{switch}");
    assert!(run["full_veterans"].as_u64().unwrap() >= 2, "{run}");
    assert_eq!(run["censored"], false);
    let careers = run["careers"].as_u64().unwrap() as usize;
    let finishing = run["finishing"].as_u64().unwrap() as usize;
    assert_eq!(finishing, careers - switched_at, "every career from the switch on finishes: {run}");
    // with one looped pick to the end: E = (careers before it) + sum over its f careers of (1 - p)^i + the tail
    // (1 - p)^f / p, the run stopping at S < eps
    if segs.last().unwrap()["t"].as_u64().unwrap() as usize == switched_at {
        let p = run["final_p"].as_f64().unwrap();
        let survival = (1.0 - p).powi(finishing as i32);
        let tail = run["tail"].as_f64().unwrap();
        assert!((tail - survival / p).abs() <= 1e-9 * tail, "tail {tail} against S / p {}", survival / p);
        let expected = switched_at as f64 + (1.0 - survival) / p + tail;
        let e = run["E"].as_f64().unwrap();
        assert!((e - expected).abs() <= 1e-9 * e, "E {e} against {expected}");
        assert!(survival < 1e-3, "stopped at S {survival}");
    }
    // goal "first": the first pick runs to the end, every career finishing
    let first = simulate("first", 40);
    let first_segs = first["runs"][0]["segs"].as_array().unwrap();
    assert_eq!(first_segs.len(), 1, "{first}");
    assert_eq!(first_segs[0]["pick"], segs[0]["pick"]);
    // stopped at the career before the switch (one that does not finish): no finishing career ran, so no tail, and E is
    // the careers run
    assert!(switched_at >= 2, "{run}");
    let cut = simulate("looped", switched_at - 1);
    let cut_run = &cut["runs"][0];
    assert_eq!(cut_run["censored"], true, "{cut_run}");
    assert_eq!(cut_run["tail"], 0.0);
    assert_eq!(cut_run["finishing"], 0);
    assert_eq!(cut_run["E"].as_f64().unwrap(), (switched_at - 1) as f64);
    assert_eq!(cut["censored"], 1);
}
