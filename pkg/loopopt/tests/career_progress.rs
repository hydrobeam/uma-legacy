use serde_json::{Value, json};
use std::{fs, process::Command};

fn run(job: &Value) -> Value {
    let path = std::env::temp_dir().join(format!("uma-career-progress-{}-{}.json", std::process::id(), rand_tag(job)));
    fs::write(&path, job.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_loopopt")).args(["data.json", path.to_str().unwrap()]).output().unwrap();
    fs::remove_file(path).unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}
fn rand_tag(job: &Value) -> u64 { job.to_string().bytes().fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64)) }

/// The weak pool (each veteran keeps about half the target sparks): the landed-set distribution agrees with the
/// career's own odds, the schedule lists every G1 she can enter, and the look ahead's facts are consistent.
#[test]
fn weak_pool_outcomes_schedule_and_look_ahead() {
    let mut job: Value = serde_json::from_str(&fs::read_to_string("jobs/career_U4.json").unwrap()).unwrap();
    job["trainees"] = json!([1034]); // Inari One
    job["fix"] = json!([2, 3]);
    job["per_trainee"] = json!(false);
    job["top"] = json!(1);
    job["lookahead"] = json!(true);
    job["star_dist"] = json!([0.0, 0.0, 1.0]);
    let result = run(&job);
    let career = &result["results"][0];
    let p: Vec<f64> = career["outcome_sets"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
    assert_eq!(p.len(), 256);
    assert!((p.iter().sum::<f64>() - 1.0).abs() < 1e-10);
    assert!((p[255] - career["p_all"].as_f64().unwrap()).abs() < 1e-10);
    let near: f64 = p.iter().enumerate().filter(|(m, _)| m.count_ones() >= 7).map(|(_, x)| x).sum();
    assert!((near - career["p_minus1"].as_f64().unwrap()).abs() < 1e-10);
    // the schedule: every G1 she can enter; those she runs have a win chance, the others say what she does instead
    let rows = career["schedule"].as_array().unwrap();
    let ran = rows.iter().filter(|r| r["run"] == true).count();
    assert!(ran > 0 && rows.len() > ran, "every G1 she can enter is listed, not only those she runs");
    assert!((career["g1_wins"].as_f64().unwrap() - rows.iter().filter_map(|r| r["p"].as_f64()).sum::<f64>()).abs() < ran as f64, "expected wins come from the races she runs");
    assert!(rows.iter().all(|r| (r["run"] == true) == r["p"].is_number() && (r["run"] == true) == r["instead"].is_null()));
    // look ahead: a floor at today's best, "no gain" exactly where the next career doesn't beat it
    let ahead = &result["lookahead"];
    let p_now = ahead["p_now"].as_f64().unwrap();
    assert!(p_now > 0.0);
    for m in ahead["miss_one"].as_array().unwrap() {
        let f = m["f"].as_f64().unwrap();
        assert!(f >= p_now * (1.0 - 1e-12));
        assert_eq!(m["no_gain"].as_bool().unwrap(), f <= p_now * (1.0 + 1e-9));
    }
    let shares: f64 = ahead["by_count"].as_array().unwrap().iter().map(|c| c["p"].as_f64().unwrap()).sum();
    assert!((shares + ahead["p_all"].as_f64().unwrap() - 1.0).abs() < 1e-9, "outcome classes cover every result");
    assert!((0.0..=1.0).contains(&ahead["improve"].as_f64().unwrap()));
}

/// The comparison for one veteran: both choices are counted the same way and both estimates are finite.
#[test]
fn policy_compares_both_choices() {
    let mut job: Value = serde_json::from_str(&fs::read_to_string("jobs/career_U4.json").unwrap()).unwrap();
    let mut v = job["legacies"][2].clone();
    v["chara"] = json!(1034);
    v["parents"] = json!([job["legacies"][2].clone(), job["legacies"][3].clone()]);
    for p in v["parents"].as_array_mut().unwrap() { p.as_object_mut().unwrap().remove("parents"); p.as_object_mut().unwrap().remove("borrowed"); }
    let n = job["legacies"].as_array().unwrap().len();
    job["legacies"].as_array_mut().unwrap().push(v);
    job["fix"] = json!([n]);
    job["per_trainee"] = json!(false);
    job["top"] = json!(1);
    job["policy"] = json!({"stay_trainee": 1034, "stay_parents": [2, 3], "p_floor": 0.0017});
    let result = run(&job);
    let policy = &result["policy"];
    for a in ["stay", "advance"] {
        let c = policy[a]["expected_careers_estimate"].as_f64().unwrap();
        let pa = policy[a]["p_all"].as_f64().unwrap();
        assert!(c >= 1.0 && pa > 0.0 && policy[a]["p_within_two"].as_f64().unwrap() >= pa);
    }
    assert!(policy["continuation_floor"].as_f64().unwrap() >= 0.0017);
}
