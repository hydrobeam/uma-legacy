//! The simulator's `index` under `act_on` "pair_search" runs the page's final recommendation (docs/SOLVER.md *Policy
//! simulator*). On three small cuts of val/plan p16, one per kind of recommendation (the pick's trainee's better parents,
//! on three veterans; another trainee's career that scores above the pick as fitted, on two; the pick, on two), a
//! simulated run's first career is, by its id (trainee, parents and deck), the career the page's own job sequence
//! recommends: the plan job, then the check one variant per job and the
//! pair search (career/run.ts `checkPhase`), read as the page's lines read it (`pairSearchNotes`). What it runs is
//! observed apart from what it was asked to run: the lineage its first segment ran (`segs`: trainee and both parents),
//! the career the report it built that career from names (`index.executed`: the `career` job's result for a named
//! career, its deck as named, no deck search; the plan's `choice` otherwise), and that career's landed-set chances and
//! P(all), each equal to the page's career's to the bit (the `career` job the page's Use this sends for it, or the plan's
//! pick).
use crate::common;
use common::{check_of, run_jobs, val_cut};
use serde_json::{Value, json};

/// The check jobs the page sends after the plan, at most: one per variant (7 fixed, up to 3 per-uma scenarios), then
/// one for the pair search; a job after the check is complete answers its report again.
const CHECK_JOBS: usize = 12;

/// A pool that shows one kind of recommendation: the smallest cut of val/plan p16 found that gives it (its veterans,
/// targets and owned umas, `val_cut`), named by the recommendation's `source`.
struct Pool {
    source: &'static str,
    legacies: &'static [usize],
    targets: &'static [usize],
    owned: &'static [u32],
}

const POOLS: [Pool; 3] = [
    Pool { source: "pick_trainee_pair", legacies: &[6, 13, 15], targets: &[3, 7], owned: &[1014, 1021, 1029, 1034] },
    Pool { source: "cross_trainee", legacies: &[6, 11], targets: &[7], owned: &[1021, 1038] },
    Pool { source: "plan_pick", legacies: &[13, 15], targets: &[7], owned: &[1021, 1029, 1034] },
];

/// The pool's job as the page's plan job sends it (`no_reports`; p16 already sets `top` 6, no places).
fn plan_job(pool: &Pool) -> Value {
    let mut job = val_cut("plan/p16.json", pool.legacies, pool.targets, pool.owned);
    job["job"]["career"]["next"] = json!({});
    job["job"]["career"]["no_reports"] = json!(true);
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

fn page_recommendation(pool: &Pool) -> PageRecommendation {
    let source = pool.source;
    let job = plan_job(pool);
    let mut check_job = job.clone();
    check_job["job"]["career"]["next"] = json!({"robust": "next", "pair_search": true});
    let jobs: Vec<Value> = std::iter::once(job).chain(std::iter::repeat_n(check_job, CHECK_JOBS)).collect();
    let outputs = run_jobs(source, &jobs);
    let plan = &outputs[0]["next"];
    let check = check_of(&outputs[CHECK_JOBS]);
    assert_eq!(check["pending"], json!(0), "{source}: the check is complete");
    assert_eq!(check["plan"], plan["outcomes"]["career"], "{source}: the check is of the plan's pick");
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

fn simulated_first_career(pool: &Pool) -> SimulatedCareer {
    let source = pool.source;
    let mut job = plan_job(pool);
    job["job"]["career"]["simulate"] = json!({"policy": "index", "act_on": "pair_search", "runs": 1, "max_careers": 1});
    let output = &run_jobs(&format!("{source}-sim"), &[job])[0];
    let run = &output["runs"][0];
    let text = |value: &Value| value.as_str().unwrap_or_else(|| panic!("{source}: not recorded: {run}")).to_string();
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
fn named_career(pool: &Pool, id: &str) -> LandedSets {
    let source = pool.source;
    let (trainee, rest) = id.split_once(':').unwrap();
    let (first, rest) = rest.split_once(',').unwrap();
    let second = rest.split_once(':').unwrap().0;
    let mut job = plan_job(pool);
    job["job"]["career"].as_object_mut().unwrap().remove("next");
    job["job"]["career"]["trainees"] = json!([trainee.parse::<u32>().unwrap()]);
    job["job"]["career"]["fix"] = json!([first.parse::<usize>().unwrap(), second.parse::<usize>().unwrap()]);
    job["job"]["career"]["given"] = json!(id);
    job["job"]["career"]["top"] = json!(1);
    let output = &run_jobs(&format!("{source}-named"), &[job])[0];
    assert_eq!(output["results"][0]["career"], json!(id), "{source}: the career job reports the career named");
    LandedSets::of(&output["results"][0])
}

#[test]
fn index_acting_on_the_pair_search_runs_the_pages_recommendation() {
    for pool in &POOLS {
        let source = pool.source;
        let page = page_recommendation(pool);
        assert_eq!(page.source, source, "{source}: the kind of recommendation this pool tests");
        assert_eq!(page.solver_says, page.career, "{source}: the solver's recommendation is the page's");
        let simulated = simulated_first_career(pool);
        assert_eq!(simulated.asked, page.career, "{source}: the simulator is asked to run the page's recommendation");
        assert_eq!(simulated.ran_lineage, lineage_of(&page.career), "{source}: it runs her on both its parents");
        assert_eq!(
            simulated.ran_from, page.career,
            "{source}: the report it runs from is of that career, deck included"
        );
        let expected = if source == "plan_pick" { page.plan_pick } else { named_career(pool, &page.career) };
        assert_eq!(simulated.ran, expected, "{source}: it runs with that career's own landed sets and odds");
        assert_eq!(simulated.segment_odds, expected.p_all, "{source}: its segment runs at those odds");
    }
}
