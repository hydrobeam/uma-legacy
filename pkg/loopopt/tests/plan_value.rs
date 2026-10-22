//! The obsolete `next.value_only` field must not change the plan's V, continuation fit or chosen trio.
//! The boot_mob fixture exercises a continuation that builds before grinding.
use serde_json::{Value, json};
use std::{fs, process::Command};

fn run(job: &Value, tag: &str) -> Value {
    let path = std::env::temp_dir().join(format!("uma-plan-value-{}-{tag}.json", std::process::id()));
    fs::write(&path, job.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_loopopt"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .args(["data.json", path.to_str().unwrap()])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice::<Value>(&output.stdout).unwrap()["next"].clone()
}

#[test]
fn v_does_not_depend_on_the_output() {
    let mut job: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/boot_mob.json")).unwrap(),
    )
    .unwrap();
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
