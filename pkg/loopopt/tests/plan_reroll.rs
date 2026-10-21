//! `next.reroll` (the game's end-of-career spark reroll) on the val/plan p16 pool. "never" is the job without the field;
//! "all_bought" and "always" plan alike (V counts careers: a career short of a target can't be the goal however it is
//! rolled) and differ only in the TP they report; with the reroll every fully looped step costs at most what it did and
//! at least half (P(all land) per career rises by k = 2 - G, 1 < k <= 2, builders unchanged).
use serde_json::{Value, json};
use std::{fs, process::Command};

fn run(tag: &str, j: &Value) -> Value {
    let p = std::env::temp_dir().join(format!("uma-plan-reroll-{}-{tag}.json", std::process::id()));
    fs::write(&p, j.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_loopopt")).current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .arg("data.json").arg(&p).env_remove("LOOPOPT_STATE_IN").env_remove("LOOPOPT_STATE_OUT").output().unwrap();
    fs::remove_file(&p).unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_str(String::from_utf8_lossy(&output.stdout).lines().next().unwrap()).unwrap()
}
fn job(rr: Option<&str>) -> Value {
    let mut j: Value = serde_json::from_str(&fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap()).unwrap();
    if let Some(r) = rr { j["next"]["reroll"] = json!(r); }
    j
}
/// timings and work counters out, and the reroll's own block
fn strip(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(m.iter().filter(|(k, _)| !["timing_s", "work", "reroll"].contains(&k.as_str()) && !k.ends_with("_ms")).map(|(k, x)| (k.clone(), strip(x))).collect()),
        Value::Array(a) => Value::Array(a.iter().map(strip).collect()),
        x => x.clone(),
    }
}

#[test]
fn reroll_settings() {
    let none = run("none", &job(None));
    let never = run("never", &job(Some("never")));
    assert_eq!(strip(&none), strip(&never), "\"never\" is not the job without the field");
    assert!(never["next"].get("reroll").is_none());
    let (all, always) = (run("all", &job(Some("all_bought"))), run("always", &job(Some("always"))));
    assert_eq!(strip(&all), strip(&always), "the two reroll settings plan differently");
    let v = |o: &Value| o["next"]["v0"].as_f64().unwrap();
    let (v0, v1) = (v(&never), v(&all));
    assert!(v1 < v0 && v1 >= v0 / 2.0 * (1.0 - 1e-12), "V {v0} -> {v1}");
    for o in [&all, &always] {
        let r = &o["next"]["reroll"];
        for k in r["k"].as_array().unwrap() { let k = k.as_f64().unwrap(); assert!(k >= 1.0 && k <= 2.0, "k {k}"); }
        assert_eq!(r["careers"].as_f64().unwrap(), v1);
    }
    // TP: "always" rerolls every career; "all_bought" at most every career, and not none
    let (ra, rb) = (&always["next"]["reroll"], &all["next"]["reroll"]);
    assert_eq!(ra["setting"], "always"); assert_eq!(rb["setting"], "all_bought");
    assert_eq!(ra["rerolls"].as_f64().unwrap(), v1);
    let n = rb["rerolls"].as_f64().unwrap();
    assert!(n > 0.0 && n <= v1, "all_bought rerolls {n} of {v1} careers");
}
