//! A per-uma career search that runs out of its work cap (`next.search_work`) gives the best career it found, not a
//! proven best: the plan counts those umas (`umas_unproven`) and marks their rows (`proven: false`), which the page
//! notes under its table. On the val/plan p16 pool with a cap of 1 unit every searched uma's search stops early; with the
//! default cap none does.
use serde_json::{Value, json};
use std::{fs, process::Command};

fn plan(job: &Value) -> Value {
    let p = std::env::temp_dir().join(format!("uma-plan-unproven-{}.json", std::process::id()));
    fs::write(&p, job.to_string()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_loopopt")).current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .arg("data.json").arg(&p).env_remove("LOOPOPT_STATE_IN").env_remove("LOOPOPT_STATE_OUT").output().unwrap();
    fs::remove_file(&p).unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice::<Value>(&out.stdout).unwrap()["next"].clone()
}

#[test]
fn capped_searches_are_reported_unproven() {
    let job: Value = serde_json::from_str(&fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap()).unwrap();
    let unproven = |n: &Value| n["options"].as_array().unwrap().iter().filter(|o| o["proven"] == json!(false)).count();
    let n = plan(&job);
    assert_eq!(n["umas_unproven"], 0);
    assert_eq!(unproven(&n), 0);
    let mut capped = job.clone(); capped["next"]["search_work"] = json!(1);
    let n = plan(&capped);
    assert!(n["umas_unproven"].as_u64().unwrap() > 0, "no search reported unproven under a cap of 1");
    assert!(unproven(&n) > 0, "no row marked unproven under a cap of 1");
    // only searched rows are marked (a proxy row was never searched)
    assert!(n["options"].as_array().unwrap().iter().all(|o| o["proven"] != json!(false) || o["exact"] == json!(true)));
}
