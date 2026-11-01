mod common;
use common::{run_job, val_job};
use serde_json::json;

/// The weak pool: the landed-set distribution agrees with the career's own odds, and the schedule includes
/// both run and skipped G1s.
#[test]
fn weak_pool_outcomes_and_schedule() {
    let mut job = val_job("jobs/career_U5.json");
    job["trainees"] = json!([1034]); // Inari One
    job["fix"] = json!([2, 3]);
    job["per_trainee"] = json!(false);
    job["top"] = json!(1);
    job["star_dist"] = json!([0.0, 0.0, 1.0]);
    let result = run_job("career-progress", &job);
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
    assert!(
        (career["g1_wins"].as_f64().unwrap() - rows.iter().filter_map(|r| r["p"].as_f64()).sum::<f64>()).abs()
            < ran as f64,
        "expected wins come from the races she runs"
    );
    assert!(
        rows.iter().all(|r| (r["run"] == true) == r["p"].is_number() && (r["run"] == true) == r["instead"].is_null())
    );
}
