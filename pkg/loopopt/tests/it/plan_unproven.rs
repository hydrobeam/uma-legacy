//! A per-uma career search that runs out of its work cap (`next.search_work`) gives the best career it found, not a
//! proven best: the plan counts those umas (`umas_unproven`) and marks their rows (`proven: false`), which the page
//! notes under its table. The val/plan p16 regression checks that a cap of 1 unit leaves unproven rows and searches,
//! while the default cap leaves none. (An alternative row from her cached decks, never searched on its pair, is
//! `proven: false` with `searched: false`: no search behind it ran out, so it is not counted here.)
use crate::common;
use common::{run_job, val_job};
use serde_json::{Value, json};

/// The plan of `job` (its `next`).
fn plan(job: &Value) -> Value {
    run_job("plan-unproven", job)["next"].clone()
}

#[test]
fn capped_searches_are_reported_unproven() {
    let job = val_job("plan/p16.json");
    let unproven = |n: &Value| {
        n["options"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["proven"] == json!(false) && o["searched"] != json!(false))
            .count()
    };
    let n = plan(&job);
    assert_eq!(n["umas_unproven"], 0);
    assert_eq!(unproven(&n), 0);
    let mut capped = job;
    capped["job"]["career"]["next"]["search_work"] = json!(1);
    let n = plan(&capped);
    assert!(n["umas_unproven"].as_u64().unwrap() > 0, "no search reported unproven under a cap of 1");
    assert!(unproven(&n) > 0, "no row marked unproven under a cap of 1");
    // only searched rows are marked (a proxy row was never searched)
    assert!(n["options"].as_array().unwrap().iter().all(|o| o["proven"] != json!(false) || o["exact"] == json!(true)));
}
