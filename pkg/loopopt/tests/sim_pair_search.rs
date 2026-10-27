//! The simulator's `index` under `act_on` "pair_search" runs the page's final recommendation (docs/SOLVER.md *Policy
//! simulator*). On three val/plan pools, one per kind of recommendation (p16: the pick's trainee's better parents;
//! p10_pink: another trainee's career that scores above the pick as fitted; p11: the pick), a simulated run's first
//! career is, by its id (trainee, parents and
//! deck), the career the page's own job sequence recommends: the plan job, then the check one variant per job and the
//! pair search (webapp/src/career.js `checkPhase`), read as the page's lines read it (`pairSearchNotes`). What it runs is
//! observed apart from what it was asked to run: the lineage its first segment ran (`segs`: trainee and both parents),
//! the career the report it built that career from names (`index.executed`: the `career` job's result for a named
//! career, its deck as named, no deck search; the plan's `choice` otherwise), and that career's landed-set chances and
//! P(all), each equal to the page's career's to the bit (the `career` job the page's Use this sends for it, or the plan's
//! pick).
use serde_json::{Value, json};
use std::{fs, process::Command};

/// The check jobs the page sends after the plan, at most: one per variant (7 fixed, up to 3 per-uma scenarios), then
/// one for the pair search; a job after the check is complete answers its report again.
const CHECK_JOBS: usize = 12;

/// One process running `jobs` in turn (the worker's kept state carried between them, as the page's worker carries it),
/// in era global; one output per job.
fn run(tag: &str, jobs: &[Value]) -> Vec<Value> {
    let dir = std::env::temp_dir();
    let paths: Vec<_> =
        (0..jobs.len()).map(|k| dir.join(format!("uma-sim-pairs-{tag}-{}-{k}.json", std::process::id()))).collect();
    for (path, job) in paths.iter().zip(jobs) {
        fs::write(path, job.to_string()).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_loopopt"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .arg("data.json")
        .args(&paths)
        .env_remove("LOOPOPT_STATE_IN")
        .env_remove("LOOPOPT_STATE_OUT")
        .output()
        .unwrap();
    for path in &paths {
        fs::remove_file(path).unwrap();
    }
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).lines().map(|line| serde_json::from_str(line).unwrap()).collect()
}

/// The val/plan pool's job as the page's plan job sends it (`no_reports`; the pools already set `top` 6, no places).
fn plan_job(pool: &str) -> Value {
    let path = format!("{}/../../val/plan/{pool}.json", env!("CARGO_MANIFEST_DIR"));
    let mut job: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    job["next"] = json!({});
    job["no_reports"] = json!(true);
    job
}

/// The career the page recommends after its check, from the fields its lines read: the pair search's best where it
/// scores above the plan's pick as fitted (`versus_pick`'s nominal margin; "Better career found", or for the pick's
/// trainee "Better parents found"), else the pick; the per-uma scenarios decide nothing. Also the solver's own
/// `recommendation` and the pick's odds.
struct PageRecommendation {
    career: String,
    solver_says: String,
    source: String,
    plan_pick: LandedSets,
}

/// A career's landed-set chances (bit i: job target i) and P(all), to the bit.
#[derive(Debug, PartialEq, Eq)]
struct LandedSets {
    outcome_sets: Vec<u64>,
    p_all: u64,
}

impl LandedSets {
    fn of(report: &Value) -> Self {
        let bits = |chance: &Value| chance.as_f64().unwrap().to_bits();
        let outcome_sets = report["outcome_sets"].as_array().unwrap_or_else(|| panic!("no landed sets: {report}"));
        Self { outcome_sets: outcome_sets.iter().map(bits).collect(), p_all: bits(&report["p_all"]) }
    }
}

fn page_recommendation(pool: &str) -> PageRecommendation {
    let job = plan_job(pool);
    let mut check_job = job.clone();
    check_job["next"] = json!({"robust": "next", "pair_search": true});
    let jobs: Vec<Value> = std::iter::once(job).chain(std::iter::repeat_n(check_job, CHECK_JOBS)).collect();
    let outputs = run(pool, &jobs);
    let plan = &outputs[0]["next"];
    let check = &outputs[CHECK_JOBS]["next"]["robust"];
    assert_eq!(check["pending"], json!(0), "{pool}: the check is complete");
    assert_eq!(check["plan"], plan["outcomes"]["career"], "{pool}: the check is of the plan's pick");
    let pairs = &check["pairs"];
    let best = &pairs["best"];
    let career = if best["versus_pick"]["margins"]["nominal"].as_f64().is_some_and(|margin| margin > 0.0) {
        &best["career"]
    } else {
        &plan["outcomes"]["career"]
    };
    PageRecommendation {
        career: career.as_str().unwrap().to_string(),
        solver_says: pairs["recommendation"]["career"].as_str().unwrap().to_string(),
        source: pairs["recommendation"]["source"].as_str().unwrap().to_string(),
        plan_pick: LandedSets::of(&plan["choice"]),
    }
}

/// A simulated `index` run's first career under `act_on` "pair_search": the career it was asked to run (the
/// recommendation as recorded before it ran), the lineage its first segment ran ("trainee:first,second"), the career
/// the report it was built from names, that career's landed sets as it ran, and the first segment's odds.
struct SimulatedCareer {
    asked: String,
    ran_lineage: String,
    ran_from: String,
    ran: LandedSets,
    segment_odds: u64,
}

fn simulated_first_career(pool: &str) -> SimulatedCareer {
    let mut job = plan_job(pool);
    job["simulate"] = json!({"policy": "index", "act_on": "pair_search", "runs": 1, "max_careers": 1});
    let output = &run(&format!("{pool}-sim"), &[job])[0];
    let run = &output["runs"][0];
    let text = |value: &Value| value.as_str().unwrap_or_else(|| panic!("{pool}: not recorded: {run}")).to_string();
    let executed = &run["index"]["executed"][0];
    SimulatedCareer {
        asked: text(&run["index"]["careers"][0]),
        ran_lineage: text(&run["segs"][0]["pick"]),
        ran_from: text(&executed["career"]),
        ran: LandedSets::of(executed),
        segment_odds: run["segs"][0]["p"].as_f64().unwrap().to_bits(),
    }
}

/// A career id's lineage: "trainee:first,second" (`kept::career_id`).
fn lineage_of(id: &str) -> String {
    let (trainee, rest) = id.split_once(':').unwrap();
    format!("{trainee}:{}", rest.split_once(':').unwrap().0)
}

/// The landed sets the `career` job gives the named career (its deck as named): the job the page's Use this sends for
/// the recommendation's row, and the one the simulator runs.
fn named_career(pool: &str, id: &str) -> LandedSets {
    let (trainee, rest) = id.split_once(':').unwrap();
    let (first, rest) = rest.split_once(',').unwrap();
    let second = rest.split_once(':').unwrap().0;
    let mut job = plan_job(pool);
    job.as_object_mut().unwrap().remove("next");
    job["trainees"] = json!([trainee.parse::<u32>().unwrap()]);
    job["fix"] = json!([first.parse::<usize>().unwrap(), second.parse::<usize>().unwrap()]);
    job["career"] = json!(id);
    job["top"] = json!(1);
    let output = &run(&format!("{pool}-named"), &[job])[0];
    assert_eq!(output["results"][0]["career"], json!(id), "{pool}: the career job reports the career named");
    LandedSets::of(&output["results"][0])
}

#[test]
fn index_acting_on_the_pair_search_runs_the_pages_recommendation() {
    for (pool, source) in [("p16", "pick_trainee_pair"), ("p10_pink", "cross_trainee"), ("p11", "plan_pick")] {
        let page = page_recommendation(pool);
        assert_eq!(page.source, source, "{pool}: the kind of recommendation this pool tests");
        assert_eq!(page.solver_says, page.career, "{pool}: the solver's recommendation is the page's");
        let simulated = simulated_first_career(pool);
        assert_eq!(simulated.asked, page.career, "{pool}: the simulator is asked to run the page's recommendation");
        assert_eq!(simulated.ran_lineage, lineage_of(&page.career), "{pool}: it runs her on both its parents");
        assert_eq!(simulated.ran_from, page.career, "{pool}: the report it runs from is of that career, deck included");
        let expected = if source == "plan_pick" { page.plan_pick } else { named_career(pool, &page.career) };
        assert_eq!(simulated.ran, expected, "{pool}: it runs with that career's own landed sets and odds");
        assert_eq!(simulated.segment_odds, expected.p_all, "{pool}: its segment runs at those odds");
    }
}
