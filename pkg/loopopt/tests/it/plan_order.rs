//! The next-career plan does not read the veterans' order: on the val/plan p10 pool, the legacies reversed give the same
//! stage candidates (`fit`'s umas, in order, and `hint`) and V (Astra's review, 2026-10-26: parent-position order seeded
//! the ceiling lineage, so the reversed pool took other candidates and V moved 2653.99 -> 2668.33); and a step cap kept
//! by a plan whose fixed parent was another veteran at the same position is not read (the same review: p10 with legacies
//! 1 and 6 swapped and fix [6], then in order keeping fix [6], read five caps fitted with another mandatory parent and V
//! fell 2447.02 -> 2345.70).
use crate::common;
use common::{run_jobs, val_job};
use serde_json::Value;

/// One process running `jobs` in turn (the worker's kept state carried between them), their outputs' `next`.
fn plans(name: &str, jobs: &[&Value]) -> Vec<Value> {
    run_jobs(&format!("plan-order-{name}"), jobs).into_iter().map(|output| output["next"].clone()).collect()
}

/// p16_borrow: borrowed veterans usable, so every uma's best career is searched before the stage candidates are chosen
/// and the free slots rank by the careers found (candidates.rs `presearch`), a path p10 (proxies) does not take
fn p16_borrow() -> Value {
    val_job("plan/p16_borrow.json")
}

/// The stage candidates (`fit`'s umas, in order) and the ceiling ranking's picks (`hint`).
fn candidates(next: &Value) -> (Vec<Value>, Vec<Value>) {
    let fit = next["fit"].as_array().unwrap().iter().map(|entry| entry[0].clone()).collect();
    let hint = next["hint"].as_array().unwrap().iter().map(|entry| entry["id"].clone()).collect();
    (fit, hint)
}

#[test]
fn the_veterans_order_changes_no_candidate() {
    let in_order = val_job("plan/p10.json");
    let mut reversed = in_order.clone();
    reversed["job"]["career"]["legacies"].as_array_mut().unwrap().reverse();
    let next = [&plans("in-order", &[&in_order])[0], &plans("reversed", &[&reversed])[0]];
    assert_eq!(candidates(next[0]), candidates(next[1]));
    assert_eq!(next[0]["v0"].as_f64().unwrap().to_bits(), next[1]["v0"].as_f64().unwrap().to_bits());
}

#[test]
fn the_veterans_order_changes_no_presearched_candidate() {
    let in_order = p16_borrow();
    let mut reversed = in_order.clone();
    reversed["job"]["career"]["legacies"].as_array_mut().unwrap().reverse();
    let next = [&plans("borrow-in-order", &[&in_order])[0], &plans("borrow-reversed", &[&reversed])[0]];
    assert!(
        next[0]["hint"].as_array().unwrap().iter().all(|entry| entry["basis"] == "searched"),
        "{}",
        next[0]["hint"]
    );
    assert_eq!(candidates(next[0]), candidates(next[1]));
    assert_eq!(next[0]["v0"].as_f64().unwrap().to_bits(), next[1]["v0"].as_f64().unwrap().to_bits());
}

#[test]
fn a_cap_kept_under_another_fixed_veteran_is_not_read() {
    let mut in_order = val_job("plan/p10.json");
    in_order["job"]["career"]["fix"] = serde_json::json!([6]);
    let mut swapped = in_order.clone();
    swapped["job"]["career"]["legacies"].as_array_mut().unwrap().swap(1, 6);
    let chained = plans("chained", &[&swapped, &in_order]);
    let plain = &plans("plain", &[&in_order])[0];
    assert_eq!(chained[1]["v0"].as_f64().unwrap().to_bits(), plain["v0"].as_f64().unwrap().to_bits());
    assert!(chained[1]["step_caps"].as_object().unwrap().is_empty(), "{}", chained[1]["step_caps"]);
}
