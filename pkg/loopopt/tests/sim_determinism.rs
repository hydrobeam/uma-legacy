//! The policy simulator's `index` runs depend on the job and the run index alone (docs/SOLVER.md *Policy simulator*):
//! on the val/sim pool4 pool, run 1 of a two-run job (`runs` 2, `first` 0) and the same run alone (`runs` 1, `first` 1),
//! each in its own process, under goal `looped`, give the same record, wall clocks aside. Each run starts from an empty
//! kept state (`kept_swap`), so the first run's plans cannot reach the second's.
use serde_json::{Value, json};
use std::{fs, process::Command};

/// The simulation's output for pool4 with this `simulate` block.
fn simulate(block: &Value, tag: &str) -> Value {
    let mut job: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/sim/pools/pool4.json")).unwrap(),
    )
    .unwrap();
    job["simulate"] = block.clone();
    let path = std::env::temp_dir().join(format!("uma-sim-determinism-{tag}-{}.json", std::process::id()));
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

/// A run's record without its wall clocks (`wall_s`, the plans' `plan_s`).
fn without_clocks(run: &Value) -> Value {
    let mut run = run.clone();
    let fields = run.as_object_mut().unwrap();
    fields.remove("wall_s");
    if let Some(index) = fields.get_mut("index").and_then(Value::as_object_mut) {
        index.remove("plan_s");
    }
    run
}

#[test]
fn sim_runs_depend_on_job_and_index() {
    let block = |runs: usize, first: usize| json!({"policy": "index", "goal": "looped", "seed": 1, "runs": runs, "first": first, "max_careers": 2});
    let both = simulate(&block(2, 0), "both");
    let alone = simulate(&block(1, 1), "alone");
    let second_of_both = &both["runs"][1];
    assert_eq!(second_of_both["run"], 1, "{both}");
    assert_eq!(alone["runs"][0]["run"], 1, "{alone}");
    assert_eq!(without_clocks(second_of_both), without_clocks(&alone["runs"][0]));
    assert_eq!(both["goal"], "looped");
}
