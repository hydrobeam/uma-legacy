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
/// career's own odds, and the schedule lists every G1 she can enter.
#[test]
fn weak_pool_outcomes_and_schedule() {
    let mut job: Value = serde_json::from_str(&fs::read_to_string("jobs/career_U5.json").unwrap()).unwrap();
    job["trainees"] = json!([1034]); // Inari One
    job["fix"] = json!([2, 3]);
    job["per_trainee"] = json!(false);
    job["top"] = json!(1);
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
}
