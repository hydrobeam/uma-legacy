//! The career planner offers looped careers: an uma's best career on two real 8/8 parents, beside her best-odds one.
//! The val/plan p16 pool with two of its veterans (El Condor Pasa #5 and Tamamo Cross #15) given their missing targets
//! at 1 star: two real 8/8s, so the cheapest trio has both sides real and its last step (Agnes Digital on those two) is a
//! career you can run now. Its full result alone saves one career per run in V's own units (V = 1/p of it), so the pick
//! saves at least one.
use serde_json::Value;
use std::{fs, process::Command};

#[test]
fn looped_career_is_offered() {
    let mut job: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap(),
    )
    .unwrap();
    let targets: Vec<u64> = job["targets"].as_array().unwrap().iter().map(|t| t["group"].as_u64().unwrap()).collect();
    for i in [5, 15] {
        let sp = job["legacies"][i]["sparks"].as_array_mut().unwrap();
        let have: Vec<u64> = sp.iter().map(|s| s[0].as_u64().unwrap()).collect();
        for &g in &targets {
            if !have.contains(&g) {
                sp.push(serde_json::json!([g, 1]));
            }
        }
    }
    let path = std::env::temp_dir().join(format!("uma-plan-looped-{}.json", std::process::id()));
    fs::write(&path, job.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_loopopt"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .args(["data.json", path.to_str().unwrap()])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let out: Value = serde_json::from_slice(&output.stdout).unwrap();
    let n = &out["next"];
    // the setup: both sides of the cheapest trio are the real 8/8s (no careers), so V is the last step alone
    assert!(n["members"].as_array().unwrap().iter().all(|m| m["real"] == true), "{}", n["members"]);
    let v0 = n["v0"].as_f64().unwrap();
    let c3 = n["v0_plan"]["careers"][2].as_f64().unwrap();
    assert!((v0 - c3).abs() < 1e-9 * v0, "V {v0} is not the last step's {c3}");
    let opts = n["options"].as_array().unwrap();
    let pick = opts.iter().find(|o| o["chosen"] == true).unwrap();
    assert!(
        pick["looped"] == true && pick["role"] == "finish",
        "pick {} on {}: role {}, looped {}",
        pick["trainee"]["name"],
        pick["parents"],
        pick["role"],
        pick["looped"]
    );
    assert!(pick["saving"].as_f64().unwrap() >= 1.0 - 1e-9, "the pick saves {} careers per run", pick["saving"]);
    // the trio's last run is on two 8/8 parents; the best-odds tag is on one row
    assert!(opts.iter().filter(|o| o["role"] == "finish").all(|o| o["looped"] == true));
    assert_eq!(opts.iter().filter(|o| o["best_odds"] == true).count(), 1);
}
