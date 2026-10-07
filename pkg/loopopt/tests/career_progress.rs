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
    // over her stars too: exact for the checked setups (the multilinear form reproduces a direct evaluation)
    assert!(ahead["stars_exact"] == true && ahead["multilinear_check"].as_f64().unwrap() < 1e-10);
    let by: Vec<&Value> = ahead["improve_by_count"].as_array().unwrap().iter().collect();
    assert!((by.iter().map(|c| c["p"].as_f64().unwrap()).sum::<f64>() - 1.0).abs() < 1e-9, "every outcome, stars included, once");
    let imp = ahead["improve_stars"].as_f64().unwrap();
    assert!((imp - by.iter().map(|c| c["p"].as_f64().unwrap() * c["improve"].as_f64().unwrap()).sum::<f64>()).abs() < 1e-12);
    assert!(ahead["best_case_with_her"].as_f64().unwrap() >= ahead["f_if_improve"].as_f64().unwrap_or(0.0));
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

/// The planner's pick as a rollout on the weak pool (career_U5 = career_U4 with `rollout: 6`): reproducible, the corner
/// tables equal a direct exact evaluation, and the pick is the one the simulations were run from (DESIGN: Inari One
/// with parents 2 and 6, 1 in 835, over the 1-in-587 career).
#[test]
fn rollout_on_weak_pool() {
    let job: Value = serde_json::from_str(&fs::read_to_string("jobs/career_U5.json").unwrap()).unwrap();
    let (a, b) = (run(&job), run(&job));
    let (ra, rb) = (&a["rollout"], &b["rollout"]);
    assert!(ra["error"].is_null(), "{}", ra);
    let qs = |r: &Value| r["candidates"].as_array().unwrap().iter().map(|c| c["q"].as_f64().unwrap().to_bits()).collect::<Vec<_>>();
    assert_eq!(ra["pick"], rb["pick"]);
    assert_eq!(qs(ra), qs(rb), "Q is reproducible to the bit");
    assert!(ra["table_check"].as_f64().unwrap() < 1e-10, "table value vs direct: {}", ra["table_check"]);
    let cands = ra["candidates"].as_array().unwrap();
    assert_eq!(cands.len(), 6);
    let pick = ra["pick"].as_u64().unwrap() as usize;
    let c = &cands[pick];
    assert_eq!(c["trainee"]["id"], 1034);
    assert_eq!(c["parents"], json!([2, 6]));
    assert!(ra["differs"] == true && c["chosen"] == true);
    assert!((c["q"].as_f64().unwrap() - 567.98).abs() < 0.01, "Q {}", c["q"]);
    let p_star = ra["p_star"].as_f64().unwrap();
    assert!((ra["repeat_careers"].as_f64().unwrap() - 1.0 / p_star).abs() < 1e-9);
    assert!((1.0 / p_star - 587.48).abs() < 0.01);
    for (i, x) in cands.iter().enumerate() {
        let (q, p, ps) = (x["q"].as_f64().unwrap(), x["p_all"].as_f64().unwrap(), x["p_all_sets"].as_f64().unwrap());
        assert!((p - ps).abs() < 1e-12 * p.max(1e-300) + 1e-15, "the landed-set distribution gives the career's odds");
        assert!(p <= p_star * (1.0 + 1e-12) && x["gain"].as_f64().unwrap() >= 1.0 - 1e-12);
        // f >= p* for every failed set, so Q <= 1 + (1 - p) / p*
        assert!(q >= 1.0 && q <= 1.0 + (1.0 - p) / p_star + 1e-9);
        assert!(i == pick || q >= c["q"].as_f64().unwrap());
    }
    assert_eq!(ra["choice"]["trainee"]["id"], 1034);
    assert!(ra["choice"]["schedule"].is_array() && ra["choice"]["outcome_sets"].is_array());
    let (lb, pbar) = (ra["lower_bound"].as_f64().unwrap(), ra["p_bar"].as_f64().unwrap());
    assert!(pbar >= p_star && lb <= 1.0 / p_star && (lb - (1.0 + (1.0 - p_star) / pbar)).abs() < 1e-9);
    assert!(ra["p_bar_proven"] == true && ra["shortlist_proven"] == true && ra["candidates_proven"] == true);
    // the main results are untouched by the rollout
    let mut plain = job.clone(); plain.as_object_mut().unwrap().remove("rollout");
    let p0 = run(&plain);
    assert_eq!(p0["results"], a["results"]);
}
