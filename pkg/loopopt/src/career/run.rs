//! A career job's answer once its `Env` is built: the main search's output (`search_output`), the page's next career
//! (`plan_next`), rolls of a kept plan (`run_carried_roll`, `answer_kept_roll`), the kept proofs (`keep_seen`) and the
//! certificates every search of the job made (`note_job_certificates`).
use crate::career::decks::CfStats;
use crate::career::pool::Pool;
use crate::career::search::{CareerSearchResult, SearchRequest, solve_kept};
use crate::career::state::{KeptState, SEEN_KEEP, hash_text, json_without};
use crate::career::{CareerJob, CareerSetup, DeckSearchAudit, Env, JobInputs, SearchState, run_career_in};
use crate::data::Data;
use crate::deck::CardsRaw;
use crate::skills::SkillsRaw;
use crate::uma::UmaIndex;
use serde_json::{Value, json};
/// A roll of a kept plan carried by another job (`roll`: {job: the plan job, stars}; the page's Reroll now sends the
/// plan job with `next.roll` on its own first, so its saving shows before the search): the job runs without it, then
/// the plan job with `next.roll` (its kept roll state, or the plan run again when the worker has none), its answer as
/// `roll`
pub(super) fn run_carried_roll(
    data: &Data,
    skills: &SkillsRaw,
    cards: &CardsRaw,
    job: &Value,
    roll: &Value,
    kept: &mut KeptState,
) -> Value {
    let mut search_job = job.clone();
    if let Some(object) = search_job.as_object_mut() {
        object.remove("roll");
    }
    let mut out = run_career_in(data, skills, cards, &search_job, kept);
    let mut plan_job = roll["job"].clone();
    out["roll"] = match plan_job.get_mut("next").and_then(|next| next.as_object_mut()) {
        Some(next) => {
            next.insert("roll".into(), roll["stars"].clone());
            let plan_out = run_career_in(data, skills, cards, &plan_job, kept);
            if plan_out["next"]["roll"].is_null() {
                json!({"error": plan_out["error"].as_str().or_else(|| plan_out["next"]["error"].as_str()).unwrap_or("the plan has no roll")})
            } else {
                plan_out["next"]["roll"].clone()
            }
        }
        None => json!({"error": "roll: the job is not a plan job (no `next`)"}),
    };
    out
}

/// Keep this job's fully checked decks for the next job on its setup (the last `SEEN_KEEP` per class, four setups).
pub(super) fn keep_seen(kept: &mut KeptState, state: &SearchState, setup_fingerprint: u64) {
    let cache = &mut kept.seen;
    cache.retain(|entry| entry.0 != setup_fingerprint);
    cache.push((
        setup_fingerprint,
        state.seen.iter().map(|refs| refs[refs.len().saturating_sub(SEEN_KEEP)..].to_vec()).collect(),
    ));
    if cache.len() > 4 {
        cache.remove(0);
    }
}

/// A roll of a plan this worker keeps (`next.roll`): answered from its roll state, no search (`plan::roll::roll_kept`);
/// None when the job carries no roll or the worker keeps no state for it (the job then runs, its plan answering it).
pub(super) fn answer_kept_roll<'data>(
    env: &mut Env<'data>,
    kept: &mut KeptState,
    run: &CareerRun<'_, 'data>,
) -> Option<Value> {
    let career_job = run.job;
    let plan_job = career_job
        .next
        .as_ref()
        .filter(|next_job| next_job.get("roll").is_some_and(|roll_value| !roll_value.is_null()))
        .and_then(|next_job| serde_json::from_value::<crate::career::plan::PlanJob>(next_job.clone()).ok())?;
    crate::career::plan::roll::roll_kept(
        env,
        kept,
        run.input,
        &career_job.legacies,
        run.pool,
        &career_job.fixed_parents,
        &plan_job,
    )
}

/// What the job's main search and its plan read.
pub(super) struct CareerRun<'run, 'data> {
    pub(super) input: &'run JobInputs<'data>,
    pub(super) job: &'run CareerJob,
    /// The job's trainees (uma indices).
    pub(super) trainees: &'run [UmaIndex],
    /// The job's pool of legacies, borrowing as `borrowed_parent` allows in the plan's look-ahead.
    pub(super) pool: &'run Pool,
    /// `borrowed_parent` "later": `pool` without borrowed parents, where the career picked comes from.
    pub(super) pick_pool: Option<&'run Pool>,
}

impl CareerRun<'_, '_> {
    /// The main search over `pool` with the job's places, trainees and result count.
    fn search_request(&self) -> SearchRequest<'_> {
        SearchRequest {
            pool: self.pool,
            fixed_parents: &self.job.fixed_parents,
            trainees: self.trainees,
            per_trainee: self.job.per_trainee,
            top: self.job.top,
        }
    }
    /// The main search of the career picked: over `pick_pool` when there is one.
    pub(super) fn pick_search_request(&self) -> SearchRequest<'_> {
        SearchRequest { pool: self.pick_pool.unwrap_or(self.pool), ..self.search_request() }
    }
    /// The key a main search is kept under (`KeptState::searches`): the hash of the job's text without the keys `drop`
    /// (`json_without`: no clone). The page sends a career job, then the same job with `next`: the plan job reuses
    /// that job's main search (a search is deterministic for its job, so the result is the one it would compute).
    pub(super) fn search_key(&self, drop: &[&str]) -> u64 {
        hash_text(&json_without(self.input.job, drop, &[]))
    }
}

/// The main search's answer: the reports of its careers (the race schedule only for the best one, the one the page
/// shows; each takes a schedule search) and its counters.
pub(super) fn search_output(
    env: &mut Env,
    run: &CareerRun,
    search: &CareerSearchResult,
    search_work: Value,
    rho_top: f64,
) -> Value {
    let career_job = run.job;
    let results: Vec<Value> = if career_job.no_reports {
        vec![]
    } else {
        search
            .found
            .iter()
            .enumerate()
            .map(|(i, found)| env.setup.report(&mut env.state, run.pool, found, i == 0))
            .collect()
    };
    let mut out = json!({"results": results, "candidates": search.candidates, "solved": search.solved, "bounded": search.bounded, "scored": search.scored, "skipped": search.skipped,
           "next_bound": search.next_bound, "complete": search.complete, "proven_top": search.proven, "cap": rho_top.min(search.certified_cap), "class_caps": env.setup.space.caps(), "work": search_work});
    // a deck band over EXACT_CAP decks (in the main search or any later one of this job) was certified at a lower cap
    // only: `cap` says which, `complete` is false
    if search.certified_cap < rho_top {
        out["complete"] = json!(false);
    }
    if career_job.compare {
        out["compare"] = json!(true);
    }
    out
}

/// The job's `next`: the page's next career (`plan::index_rule`) on the main search `search`.
pub(super) fn plan_next<'data>(
    env: &mut Env<'data>,
    kept: &mut KeptState,
    run: &CareerRun<'_, 'data>,
    search: &CareerSearchResult,
    next: &Value,
) -> Value {
    let career_job = run.job;
    let settings = match serde_json::from_value::<crate::career::plan::PlanJob>(next.clone()) {
        Ok(settings) => settings,
        Err(e) => return json!({"error": format!("next: {e}")}),
    };
    let borrowing;
    let (solved, pick_search) = match run.pick_pool {
        None => (search, None),
        // "later": the look-ahead's main search is the one that borrows: the same job's as "allow", kept under its
        // key (so a plan after an "allow" run takes it as it is)
        Some(pick_pool) => {
            borrowing = solve_kept(
                env,
                kept,
                run.search_key(&["next", "no_reports", "borrowed_parent"]),
                &run.search_request(),
            );
            (&borrowing, Some(crate::career::plan::PickSearch { pool: pick_pool, solved: search }))
        }
    };
    crate::career::plan::index_rule(
        env,
        kept,
        &crate::career::plan::PlanInputs {
            resources: run.input,
            legacies: &career_job.legacies,
            pool: run.pool,
            solved,
            fixed_parents: &career_job.fixed_parents,
            trainees: run.trainees,
            settings: &settings,
            pick_search,
        },
    )
}

/// What every search of the job certified, into its answer: band overflows (`cap`, `complete`), decks past their
/// class's assumed cap, and the cap-free searches.
pub(super) fn note_job_certificates(setup: &CareerSetup, audit: &DeckSearchAudit, out: &mut Value, rho_top: f64) {
    if audit.overflows > 0 {
        out["complete"] = json!(false);
        out["cap"] = json!(rho_top.min(audit.certified_cap));
        out["band_overflows"] = json!(audit.overflows);
    }
    // a scored deck past its class's assumed cap (in any search of the job): per class [class, ratio, cap]
    if audit.cap_breach > 0 {
        let caps = setup.space.caps();
        out["cap_exceeded"] = json!(
            (0..4)
                .filter(|&k| audit.max_ratio[k] > caps[k] * (1.0 + 1e-9))
                .map(|k| json!([k, audit.max_ratio[k], caps[k]]))
                .collect::<Vec<_>>()
        );
    }
    // the deck searches that saw a breach and were answered by the cap-free search (`best_deck_cf`; every search of the
    // job, the plan's included), how many of those (or of a `cap_free` job's searches) proved nothing (then `complete`
    // is false), breaches left unanswered (`CAP_RERUN=0`), and the cap-free work
    if setup.space.deck_job.cap_free {
        out["cap_free"] = json!(true);
    }
    let cap_free = &audit.cap_free_stats;
    if *cap_free != CfStats::default() {
        out["cap_rerun"] = json!({"searches": cap_free.reruns, "unproven": cap_free.open, "unanswered": cap_free.unhandled, "work": cap_free.work});
        if cap_free.open > 0 || cap_free.unhandled > 0 {
            out["complete"] = json!(false);
        }
    }
}
