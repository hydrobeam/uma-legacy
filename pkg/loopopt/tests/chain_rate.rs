//! The pal-count chain-event rate (`chain_budget`, `ChainModel`) on the shipped Global data: without a slope it is the
//! fixed law it stands for, and a positive slope raises the odds of decks with pals.
mod common;
use common::{run_job, val_job, without_clocks};
use serde_json::{Value, json};

/// pal_U's deck job with deck settings `fields`.
fn job(fields: Value) -> Value {
    let mut job = val_job("jobs/pal_U.json");
    for (key, value) in fields.as_object().unwrap() {
        job["job"]["deck"][key] = value.clone();
    }
    job
}
/// logit(0.35): the pal-free rate the fixed law used.
fn intercept() -> f64 {
    (0.35f64 / 0.65).ln()
}

/// A rate without slope gives every deck one distribution: the whole output equals the fixed law's at that rate.
#[test]
fn flat_rate_is_the_fixed_law() {
    let a = intercept();
    let rate = 1.0 / (1.0 + (-a).exp());
    let flat = run_job("chain-rate", &job(json!({"chain_budget": [a, 0.0]})));
    let fixed = run_job("chain-rate", &job(json!({"chain_events": [21, rate]})));
    assert_eq!(without_clocks(&flat), without_clocks(&fixed));
}

/// With four pals in every deck, a positive slope raises each deck's chain-event rate, so the best deck's odds rise.
#[test]
fn slope_raises_four_pal_odds() {
    let a = intercept();
    let p_all = |slope: f64| {
        let out = run_job("chain-rate", &job(json!({"pals": 4, "chain_budget": [a, slope]})));
        assert_eq!(out["top"][0]["deck"]["pals"], json!(4), "{out}");
        out["top"][0]["loop"]["p_all"].as_f64().unwrap()
    };
    let (flat, sloped) = (p_all(0.0), p_all(0.168_072));
    assert!(sloped > flat, "{sloped} <= {flat}");
}

/// Both laws at once are refused.
#[test]
fn both_laws_refused() {
    let out = run_job("chain-rate", &job(json!({"chain_events": [21, 0.35], "chain_budget": [intercept(), 0.1]})));
    assert!(out["error"].as_str().is_some_and(|error| error.contains("chain_budget")), "{out}");
}
