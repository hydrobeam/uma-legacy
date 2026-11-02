//! The deck searches' bounds where the class caps are known to miss, end to end on the real card data, and what a cap
//! breach leaves behind. A Seiun Sky [Paint the Sky Red] career whose deck may hold no pal (`nested_nopal`: her
//! card's chain gives the two targets at links 2 and 3, the best product deck misleads by a factor 4), the same in
//! three-card decks (`nested_three_cards`), and with veterans carrying the targets at different stars, so each pair
//! has its own chances and the later deck searches start from the earlier ones' decks (`nested_seeded`): the capped
//! search finds the cap-free search's answer, scores no deck past a cap it relies on, covers the nested deck by its
//! proven bound and is proven (under the two-feeder cap where it pruned such decks). Then cap breaches forced by
//! lowered caps (`CAPS_TEST`) on val/plan p16: one in the main search, and one first found while planning on a main
//! search kept from a proven run; the searches
//! and plans after it, the kept search reused alone and both after the worker's state goes out and comes back into a
//! process with the normal caps, all say so.
use crate::common;
use common::{State, TempFile, loopopt, run_with, val_job};
use serde_json::{Value, json};

/// The class caps lowered (`CAPS_TEST`) so that a search breaches them.
const LOWERED_CAPS: &str = "1.0,1.0,1.0,1.0";

/// one process running `jobs` in turn (one output line each), the caps lowered to `caps` (`CAPS_TEST`) where given
fn run(tag: &str, jobs: &[&Value], caps: Option<&str>, state: State) -> Vec<Value> {
    let mut command = loopopt("global", state);
    command.env_remove("CAPS_TEST");
    if let Some(caps) = caps {
        command.env("CAPS_TEST", caps);
    }
    run_with(command, tag, jobs).outputs
}

/// The fixture `tests/fixtures/{name}.json`.
fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn nested_decks_match_the_cap_free_search() {
    for (name, assumes) in [("nested_nopal", false), ("nested_three_cards", true), ("nested_seeded", false)] {
        let job = fixture(name);
        let mut cap_free = job.clone();
        cap_free["job"]["deck"]["cap_free"] = json!(true);
        let capped = &run(&format!("{name}-capped"), &[&job], None, State::NONE)[0];
        let free = &run(&format!("{name}-free"), &[&cap_free], None, State::NONE)[0];
        let p_all = |out: &Value| out["results"][0]["p_all"].as_f64().unwrap();
        assert!((p_all(capped) - p_all(free)).abs() <= 1e-12 * p_all(free), "{name}: {capped} vs {free}");
        assert_eq!(capped["results"][0]["deck"], free["results"][0]["deck"], "{name}: another deck");
        assert!(capped["certificates"].get("cap_exceeded").is_none(), "{name}: a deck past a cap the search relies on");
        assert!(
            capped["certificates"]["nested_max_ratio"].as_f64().is_some_and(|ratio| ratio > 1.05),
            "{name}: {capped}"
        );
        assert_eq!((&capped["complete"], &capped["proven_top"]), (&json!(true), &json!(true)), "{name}");
        assert!(
            capped.get("unproven").is_none() && capped["certificates"].get("job_unproven").is_none(),
            "{name}: {capped}"
        );
        let two_feeder = Some(json!(["two_feeder_cap"]));
        assert_eq!(capped.get("assumes").cloned(), assumes.then(|| two_feeder.clone()).flatten(), "{name}");
        assert_eq!(
            capped["certificates"].get("job_assumes").cloned(),
            assumes.then_some(two_feeder).flatten(),
            "{name}"
        );
        // (the seeded fixture's pairs differ in chances: two deck searches, the second started from the first's deck)
        if name == "nested_seeded" {
            assert!(capped["solved"].as_u64().is_some_and(|solved| solved >= 2), "{name}: {}", capped["solved"]);
        }
    }
}

/// val/plan p16: (the plan job with `no_reports`, the search job)
fn p16_jobs() -> (Value, Value) {
    let mut job = val_job("plan/p16.json");
    let mut search = job.clone();
    search["job"]["career"].as_object_mut().unwrap().remove("next");
    job["job"]["career"]["no_reports"] = json!(true);
    (job, search)
}

/// whether an output's reasons `field` (`unproven`, `job_unproven`) hold `reason`
fn lists(out: &Value, field: &str, reason: &str) -> bool {
    out[field].as_array().is_some_and(|reasons| reasons.contains(&json!(reason)))
}

#[test]
fn a_breach_stays_unproven_through_a_reload() {
    let (job, search) = p16_jobs();
    let state = TempFile::new("cap-shapes-state");
    let breached = run("breach", &[&search, &job], Some(LOWERED_CAPS), State { load: None, save: Some(state.path()) });
    let reloaded = run("breach-reload", &[&search, &job], None, State { load: Some(state.path()), save: None });
    for (when, outs) in [("breached", &breached), ("after the reload", &reloaded)] {
        for out in outs {
            assert_eq!(out["complete"], json!(false), "{when}: {}", out["certificates"]["job_unproven"]);
            assert!(
                out["certificates"]["job_unproven"]
                    .as_array()
                    .is_some_and(|reasons| reasons.contains(&json!("cap_breach"))),
                "{when}: {}",
                out["certificates"]["job_unproven"]
            );
            assert!(out["certificates"]["cap_breached"].as_array().is_some_and(|umas| !umas.is_empty()), "{when}");
            assert_eq!(out["proven_top"], json!(false), "{when}");
            assert!(lists(out, "unproven", "cap_breach"), "{when}: {}", out["unproven"]);
        }
    }
}

#[test]
fn a_breach_found_while_planning_unproves_the_kept_search() {
    let (job, search) = p16_jobs();
    let (proven_state, planned_state) = (TempFile::new("cap-shapes-proven"), TempFile::new("cap-shapes-planned"));
    // a proven main search, kept; then, under lowered caps, the plan on it (the main search kept: its deck searches
    // do not run again, so the breach is the plan's) and the search alone; then both after a reload with normal caps
    let proven = run("plan-breach-search", &[&search], None, State { load: None, save: Some(proven_state.path()) });
    let planned = run(
        "plan-breach",
        &[&job, &search],
        Some(LOWERED_CAPS),
        State { load: Some(proven_state.path()), save: Some(planned_state.path()) },
    );
    let reloaded =
        run("plan-breach-reload", &[&search, &job], None, State { load: Some(planned_state.path()), save: None });
    let first = &proven[0];
    assert_eq!((&first["complete"], &first["proven_top"]), (&json!(true), &json!(true)), "{first}");
    assert!(first.get("unproven").is_none() && first["certificates"].get("job_unproven").is_none(), "{first}");
    // (p16's nested-chain card, 30116, beside chain cards feeding its other targets: proven under the named assumption)
    assert!(
        lists(first, "assumes", "multi_feeder_cap") && lists(&first["certificates"], "job_assumes", "multi_feeder_cap"),
        "{first}"
    );
    assert_eq!(planned[0]["work"]["nodes"], json!(0), "the plan job searched its main search again");
    assert!(
        planned[0]["certificates"]["cap_breaches"].as_u64().is_some_and(|breaches| breaches > 0),
        "no breach while planning"
    );
    let rows = planned[0]["next"]["options"].as_array().unwrap();
    assert!(rows.iter().any(|row| row["proven"] == json!(false)), "no row unproven after the plan's breach");
    for (when, outs) in [("after the plan's breach", &planned), ("after the reload", &reloaded)] {
        for out in outs {
            assert_eq!((&out["complete"], &out["proven_top"]), (&json!(false), &json!(false)), "{when}");
            assert!(lists(out, "unproven", "cap_breach"), "{when}: {}", out["unproven"]);
            assert!(
                lists(&out["certificates"], "job_unproven", "cap_breach"),
                "{when}: {}",
                out["certificates"]["job_unproven"]
            );
            assert!(out["certificates"]["cap_breached"].as_array().is_some_and(|umas| !umas.is_empty()), "{when}");
        }
    }
}

/// Class flags follow the class's content, not the job's trainee list: a proven p16 plan kept, then (lowered caps) a
/// search with only some of its trainees breaches their classes, then the first plan again (normal caps) reuses its
/// kept plan and main search, searched before the breach, and reports it.
#[test]
fn a_breach_under_another_trainee_filter_reaches_the_kept_plan() {
    let (job, mut narrow) = p16_jobs();
    let (kept_state, breached_state) =
        (TempFile::new("cap-shapes-filter-kept"), TempFile::new("cap-shapes-filter-breached"));
    let first = run("filter-plan", &[&job], None, State { load: None, save: Some(kept_state.path()) });
    assert_eq!(first[0]["proven_top"], json!(true), "{}", first[0]["unproven"]);
    let mut ids: Vec<Value> = vec![];
    for row in first[0]["next"]["options"].as_array().unwrap() {
        let id = row["report"]["trainee"]["id"].clone();
        if ids.len() < 4 && !ids.contains(&id) {
            ids.push(id);
        }
    }
    narrow["job"]["career"]["trainees"] = json!(ids);
    let narrowed = run(
        "filter-narrow",
        &[&narrow],
        Some(LOWERED_CAPS),
        State { load: Some(kept_state.path()), save: Some(breached_state.path()) },
    );
    let again = run("filter-again", &[&job], None, State { load: Some(breached_state.path()), save: None });
    assert!(
        narrowed[0]["certificates"]["cap_breaches"].as_u64().is_some_and(|breaches| breaches > 0),
        "no breach: {}",
        narrowed[0]
    );
    let out = &again[0];
    assert_eq!(out["work"]["nodes"], json!(0), "the plan job searched its main search again");
    assert_eq!((&out["complete"], &out["proven_top"]), (&json!(false), &json!(false)), "{}", out["unproven"]);
    assert!(
        lists(out, "unproven", "cap_breach") && lists(&out["certificates"], "job_unproven", "cap_breach"),
        "{}",
        out["unproven"]
    );
    assert!(out["certificates"]["cap_breached"].as_array().is_some_and(|umas| !umas.is_empty()));
    let rows = out["next"]["options"].as_array().unwrap();
    assert!(rows.iter().any(|row| row["proven"] == json!(false)), "no row unproven after the breach");
}

/// A nested card beside another chain feeder of its targets, which no bound covers: nested_nopal with Mejiro Palmer
/// [Go Ahead and Laugh] (30027) allowed, whose chain feeds Vanguard Spirit at link 3 beside Seiun Sky [Paint the Sky
/// Red]. The pair's exact / product is 2.71, past the class-1 cap 1.05: the capped search holds such decks at their
/// configuration's nested cap (leaf and `BestCovered` value), and a scored deck past its cap within its nested bound
/// makes the cap-free search answer the call. The capped search answers as the cap-free one does (Palmer and Seiun Sky,
/// P(all) 0.005884; the nested bound's pre-review answer was Seiun Sky alone at 0.005472, reported proven).
#[test]
fn a_nested_card_beside_another_feeder_matches_the_cap_free_search() {
    let mut job = fixture("nested_nopal");
    job["job"]["deck"]["exclude_cards"].as_array_mut().unwrap().retain(|card| card != 30027);
    let mut cap_free = job.clone();
    cap_free["job"]["deck"]["cap_free"] = json!(true);
    let capped = &run("palmer-capped", &[&job], None, State::NONE)[0];
    let free = &run("palmer-free", &[&cap_free], None, State::NONE)[0];
    let p_all = |out: &Value| out["results"][0]["p_all"].as_f64().unwrap();
    assert!((p_all(capped) - p_all(free)).abs() <= 1e-12 * p_all(free), "{} vs {}", p_all(capped), p_all(free));
    assert!((p_all(free) - 0.005_884).abs() < 5e-7, "the cap-free answer moved: {}", p_all(free));
    assert_eq!(capped["results"][0]["deck"], free["results"][0]["deck"], "another deck");
    let cards = capped["results"][0]["deck"]["card_ids"].as_array().unwrap();
    assert!(cards.contains(&json!(30027)) && cards.contains(&json!(30055)), "{cards:?}");
    assert_eq!((&capped["complete"], &capped["proven_top"]), (&json!(true), &json!(true)), "{}", capped["unproven"]);
    assert!(
        capped["certificates"].get("cap_breaches").is_none() && capped["certificates"].get("cap_breached").is_none(),
        "{capped}"
    );
}
