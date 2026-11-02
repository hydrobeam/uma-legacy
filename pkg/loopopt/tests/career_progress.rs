mod common;
use common::{run_job, val_job};
use serde_json::json;

/// A job whose targets are all the scenario's gold (`guaranteed` 2, no uncertain target) answers: no card changes the
/// odds, so each career takes the deck of filler with no search (the deck search, which takes at least one target,
/// panicked on it), and the plan prices its trios on those careers. val/plan p10 with its two guaranteed targets.
#[test]
fn a_job_whose_targets_are_all_guaranteed_answers() {
    let mut job = val_job("plan/p10.json");
    let targets = job["job"]["skills"]["targets"].take();
    let guaranteed: Vec<_> =
        targets.as_array().unwrap().iter().filter(|target| target["guaranteed"] == 2).cloned().collect();
    assert_eq!(guaranteed.len(), 2, "p10 has two guaranteed targets");
    job["job"]["skills"]["targets"] = json!(guaranteed);
    let output = run_job("career-guaranteed", &job);
    assert_eq!(output["complete"], true, "the search is proven");
    let results = output["results"].as_array().unwrap();
    assert!(!results.is_empty(), "careers are found");
    for career in results {
        assert_eq!(career["deck"]["card_ids"], json!([]), "no card is needed: filler only");
        let p_all = career["p_all"].as_f64().unwrap();
        assert!(p_all > 0.0 && p_all <= 1.0, "P(all) {p_all}");
    }
    let next = &output["next"];
    assert_eq!(next["umas_unproven"], 0);
    assert!(next["v0"].as_f64().is_some_and(f64::is_finite), "the plan has a V: {}", next["v0"]);
}

/// The weak pool: the landed-set distribution agrees with the career's own odds, and the schedule includes
/// both run and skipped G1s.
#[test]
fn weak_pool_outcomes_and_schedule() {
    let mut job = val_job("jobs/career_U5.json");
    job["job"]["career"]["trainees"] = json!([1034]); // Inari One
    job["job"]["career"]["fix"] = json!([2, 3]);
    job["job"]["career"]["per_trainee"] = json!(false);
    job["job"]["career"]["top"] = json!(1);
    job["job"]["skills"]["rank_class"] = json!("ss_and_up");
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
