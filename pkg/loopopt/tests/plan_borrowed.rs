//! `borrowed_parent` on the val/plan p16 pool with veteran #14 marked borrowed.
//! "later": no career the job offers has a borrowed parent (the search's results, the plan's rows and its pick), while
//! the plan's look-ahead is the one with borrowing (V and Cont's fit are the "allow" job's, to the bit on this pool);
//! the page's run as two jobs, after an "allow" run in the same worker, plans as the combined job does (each setting's
//! main search is kept under its own key). "never" answers as the pool without the borrowed veteran.
use serde_json::{Value, json};
use std::{fs, process::Command};

const BOR: usize = 14;

/// one process running `jobs` in turn (one output line each); `tag` names its job files
fn run(tag: &str, jobs: &[&Value]) -> Vec<Value> {
    let dir = std::env::temp_dir();
    let ps: Vec<_> = jobs
        .iter()
        .enumerate()
        .map(|(i, j)| {
            let p = dir.join(format!("uma-plan-borrowed-{}-{tag}-{i}.json", std::process::id()));
            fs::write(&p, j.to_string()).unwrap();
            p
        })
        .collect();
    let output = Command::new(env!("CARGO_BIN_EXE_loopopt"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .arg("data.json")
        .args(&ps)
        .env_remove("LOOPOPT_STATE_IN")
        .env_remove("LOOPOPT_STATE_OUT")
        .output()
        .unwrap();
    for p in &ps {
        fs::remove_file(p).unwrap();
    }
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}
fn job(bp: Option<&str>) -> Value {
    let mut j: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap(),
    )
    .unwrap();
    j["legacies"][BOR]["borrowed"] = json!(true);
    if let Some(b) = bp {
        j["borrowed_parent"] = json!(b);
    }
    j
}
/// timings and work counters out
fn strip(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(
            m.iter()
                .filter(|(k, _)| {
                    !["timing_s", "work", "umas_searched", "umas_certified"].contains(&k.as_str())
                        && !k.ends_with("_ms")
                })
                .map(|(k, x)| (k.clone(), strip(x)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(strip).collect()),
        x => x.clone(),
    }
}
/// the legacy indices (`parents`, a slot's `legacy`) as they are once veteran `gone` is left out of the pool
fn renumber(v: &Value, gone: usize) -> Value {
    let ix = |x: &Value| match x.as_u64() {
        Some(i) if !x.is_boolean() => json!(if i as usize > gone { i - 1 } else { i }),
        _ => x.clone(),
    };
    match v {
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, x)| {
                    (
                        k.clone(),
                        match (k.as_str(), x) {
                            ("parents", Value::Array(a)) => Value::Array(
                                a.iter().map(|y| if y.is_object() { renumber(y, gone) } else { ix(y) }).collect(),
                            ),
                            ("legacy", _) => ix(x),
                            // a career id, "uma:a,b:deck" (a report's, a member's builder): its parents are legacy indices too
                            ("career" | "builder", Value::String(id)) => {
                                let mut parts: Vec<String> = id.splitn(3, ':').map(str::to_owned).collect();
                                if let Some(pair) = parts.get_mut(1) {
                                    *pair = pair
                                        .split(',')
                                        .map(|p| p.parse::<u64>().map_or(p.to_owned(), |i| ix(&json!(i)).to_string()))
                                        .collect::<Vec<_>>()
                                        .join(",");
                                }
                                Value::String(parts.join(":"))
                            }
                            // a trio side's identity, "kind:i[,j]" (output.rs `side_key`): its legacy indices too
                            ("sides", Value::Array(a)) => Value::Array(
                                a.iter()
                                    .map(|side| match side.as_str().and_then(|text| text.split_once(':')) {
                                        Some((kind, places)) => json!(format!(
                                            "{kind}:{}",
                                            places
                                                .split(',')
                                                .map(|p| p
                                                    .parse::<u64>()
                                                    .map_or(p.to_owned(), |i| ix(&json!(i)).to_string()))
                                                .collect::<Vec<_>>()
                                                .join(",")
                                        )),
                                        None => side.clone(),
                                    })
                                    .collect(),
                            ),
                            _ => renumber(x, gone),
                        },
                    )
                })
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(|x| renumber(x, gone)).collect()),
        x => x.clone(),
    }
}
fn has_bor(r: &Value) -> bool {
    r["parents"].as_array().unwrap().iter().any(|p| p.as_u64() == Some(BOR as u64))
}

#[test]
fn later_picks_without_a_borrowed_parent_and_plans_with_one() {
    let (allow, later) = (job(None), job(Some("later")));
    let a = &run("a", &[&allow])[0];
    let l = &run("l", &[&later])[0];
    // the premise: with borrowing the best careers and the pick use the borrowed veteran
    assert!(
        has_bor(&a["results"][0]) && has_bor(&a["next"]["choice"]),
        "the borrowed veteran is not in the allow job's pick"
    );
    // the pick: no result, row or pick with a borrowed parent
    assert!(
        !l["results"].as_array().unwrap().is_empty() && l["results"].as_array().unwrap().iter().all(|r| !has_bor(r))
    );
    let opts = l["next"]["options"].as_array().unwrap();
    assert!(!opts.is_empty() && opts.iter().all(|o| !has_bor(o)), "a row with a borrowed parent");
    assert!(!has_bor(&l["next"]["choice"]));
    assert_eq!(opts.iter().filter(|o| o["chosen"] == true).count(), 1);
    // the look-ahead: the allow job's
    for k in ["v0", "fit", "members", "v0_plan"] {
        assert_eq!(l["next"][k], a["next"][k], "{k}");
    }
    // and it matters: without borrowing later (never) V is another
    let n = &run("n", &[&job(Some("never"))])[0];
    assert_ne!(n["next"]["v0"], a["next"]["v0"]);
    // the page's run as two jobs, after an allow run in the same worker: the combined later job's answer
    let strip_next = |j: &Value| {
        let mut s = j.clone();
        s.as_object_mut().unwrap().remove("next");
        s
    };
    let mut lp = later.clone();
    lp["no_reports"] = json!(true);
    let mut ap = allow.clone();
    ap["no_reports"] = json!(true);
    let two = run("two", &[&strip_next(&allow), &ap, &strip_next(&later), &lp]);
    assert!(strip(&two[2]["results"]) == strip(&l["results"]), "the later search job after an allow run differs");
    assert!(
        strip(&two[3]["next"]) == strip(&l["next"]),
        "the later plan job after an allow run differs from the combined job's"
    );
}

/// "later" on the page's path: the plan job, then its check one job at a time from the worker's kept state
/// (`next.robust: "next"` with the pair search), as `checkPhase` sends them. The check answers before the job builds
/// its pools, so it must read the pick's pool there too. On p16 the pair search's best pairs all take veteran #15, so
/// it is the one marked borrowed here: "allow" pairs with her (the premise), "later" never may.
#[test]
fn later_checks_from_the_kept_plan_without_a_borrowed_parent() {
    const PAIRED: u64 = 15;
    let checked = |bp: Option<&str>| {
        let mut plan = job(bp);
        plan["legacies"][BOR]["borrowed"] = json!(false);
        plan["legacies"][PAIRED as usize]["borrowed"] = json!(true);
        plan["no_reports"] = json!(true);
        let mut check = plan.clone();
        check["next"] = json!({"robust": "next", "pair_search": true});
        let mut jobs = vec![&plan];
        // (more than the check's variants: the last ones answer its finished report again)
        jobs.extend(std::iter::repeat_n(&check, 14));
        let outputs = run(&format!("kept-{}", bp.unwrap_or("allow")), &jobs);
        let pairs = outputs
            .iter()
            .rev()
            .find_map(|output| output["next"]["robust"]["pairs"].as_object().cloned())
            .expect("no check answered with its pair search");
        let paired = |parents: &[Value]| parents.iter().any(|p| p.as_u64() == Some(PAIRED));
        let any_pair =
            pairs["trainees"].as_array().unwrap().iter().any(|found| paired(found["best_parents"].as_array().unwrap()));
        let recommended = pairs["recommendation"]["career"].as_str().unwrap().to_string();
        let rec_parents: Vec<Value> =
            recommended.split(':').nth(1).unwrap().split(',').map(|p| json!(p.parse::<u64>().unwrap())).collect();
        (any_pair, paired(&rec_parents), recommended)
    };
    let (allow_pairs, _, _) = checked(None);
    assert!(allow_pairs, "the premise: with borrowing the pair search pairs a trainee with the borrowed veteran");
    let (later_pairs, later_recommends, recommended) = checked(Some("later"));
    assert!(!later_pairs, "the pair search paired a trainee with the borrowed veteran under \"later\"");
    assert!(!later_recommends, "the recommendation {recommended} takes the borrowed veteran under \"later\"");
}

#[test]
fn never_is_the_pool_without_borrowed_veterans() {
    let never = job(Some("never"));
    let mut gone = job(None);
    gone["legacies"].as_array_mut().unwrap().remove(BOR);
    let n = &run("nv", &[&never])[0];
    let g = &run("g", &[&gone])[0];
    assert!(strip(&renumber(&n["results"], BOR)) == strip(&g["results"]), "results differ");
    assert!(strip(&renumber(&n["next"], BOR)) == strip(&g["next"]), "plans differ");
}
