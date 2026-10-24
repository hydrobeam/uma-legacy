//! The pal-count chain-event rate (`chain_budget`, `ChainModel`) on the shipped Global data: without a slope it is the
//! fixed law it stands for, and a positive slope raises the odds of decks with pals.
use serde_json::{Value, json};
use std::{fs, process::Command};

fn run(job: &Value) -> Value {
    let tag = job.to_string().bytes().fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b)));
    let path = std::env::temp_dir().join(format!("uma-chain-rate-{}-{tag}.json", std::process::id()));
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
fn job(fields: Value) -> Value {
    let mut job: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/jobs/pal_U.json")).unwrap(),
    )
    .unwrap();
    for (key, value) in fields.as_object().unwrap() {
        job[key] = value.clone();
    }
    job
}
/// The output without its wall clocks (`timing_s`, `*_ms`), as val/same.py compares it.
fn strip(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(key, _)| *key != "timing_s" && !key.ends_with("_ms"))
                .map(|(key, value)| (key.clone(), strip(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(strip).collect()),
        other => other.clone(),
    }
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
    let flat = run(&job(json!({"chain_budget": [a, 0.0]})));
    let fixed = run(&job(json!({"chain_events": [21, rate]})));
    assert_eq!(strip(&flat), strip(&fixed));
}

/// With four pals in every deck, a positive slope raises each deck's chain-event rate, so the best deck's odds rise.
#[test]
fn slope_raises_four_pal_odds() {
    let a = intercept();
    let p_all = |slope: f64| {
        let out = run(&job(json!({"pals": 4, "chain_budget": [a, slope]})));
        assert_eq!(out["top"][0]["deck"]["pals"], json!(4), "{out}");
        out["top"][0]["loop"]["p_all"].as_f64().unwrap()
    };
    let (flat, sloped) = (p_all(0.0), p_all(0.168_072));
    assert!(sloped > flat, "{sloped} <= {flat}");
}

/// Both laws at once are refused.
#[test]
fn both_laws_refused() {
    let out = run(&job(json!({"chain_events": [21, 0.35], "chain_budget": [intercept(), 0.1]})));
    assert!(out["error"].as_str().is_some_and(|error| error.contains("chain_budget")), "{out}");
}
