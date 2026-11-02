//! The obsolete `next.value_only` field is refused (a job's records refuse a field they do not know), so it cannot
//! change the plan's V, continuation fit or chosen trio. The boot_mob fixture exercises a continuation that builds
//! before grinding.
use crate::common;
use common::{run_job, val_job};
use serde_json::{Value, json};

/// The plan of `job` (its `next`).
fn run(job: &Value, tag: &str) -> Value {
    run_job(&format!("plan-value-{tag}"), job)["next"].clone()
}

#[test]
fn v_does_not_depend_on_the_output() {
    let mut job = val_job("plan/boot_mob.json");
    job["job"]["career"]["next"] = json!({});
    let full = run(&job, "full");
    // (the setup: V's sides build before they grind, so Cont is in V)
    assert!(
        full["members"].as_array().unwrap().iter().any(|m| m["building_careers"].as_f64().unwrap() > 0.0),
        "{}",
        full["members"]
    );
    job["job"]["career"]["next"] = json!({"value_only": true});
    let file = common::TempFile::job("plan-value-vo", &job);
    let output = common::loopopt("global", common::State::NONE).arg("data.json").arg(file.path()).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.code() == Some(2) && stderr.contains("unknown field `value_only`"), "{stderr}");
}
