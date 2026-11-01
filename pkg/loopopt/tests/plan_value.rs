//! The obsolete `next.value_only` field must not change the plan's V, continuation fit or chosen trio.
//! The boot_mob fixture exercises a continuation that builds before grinding.
mod common;
use common::{run_job, val_job};
use serde_json::{Value, json};

/// The plan of `job` (its `next`).
fn run(job: &Value, tag: &str) -> Value {
    run_job(&format!("plan-value-{tag}"), job)["next"].clone()
}

#[test]
fn v_does_not_depend_on_the_output() {
    let mut job = val_job("plan/boot_mob.json");
    job["next"] = json!({});
    let full = run(&job, "full");
    // (the setup: V's sides build before they grind, so Cont is in V)
    assert!(
        full["members"].as_array().unwrap().iter().any(|m| m["building_careers"].as_f64().unwrap() > 0.0),
        "{}",
        full["members"]
    );
    job["next"] = json!({"value_only": true});
    let vo = run(&job, "vo");
    for k in ["v0", "fit", "members", "v0_plan"] {
        assert_eq!(vo[k], full[k], "{k}");
    }
}
