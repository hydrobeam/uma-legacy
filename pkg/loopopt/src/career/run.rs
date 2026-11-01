//! A career job's answer once its `Env` is built: the main search's output (`search_output`), the page's next career
//! (`plan_next`), rolls of a kept plan (`run_carried_roll`, `answer_kept_roll`), the kept proofs (`keep_seen`) and the
//! certificates every search of the job made (`note_job_certificates`).
use crate::career::decks::CfStats;
use crate::career::plan::report::{PlanReply, RollReply};
use crate::career::pool::Pool;
use crate::career::reply::{CapRerun, CareerSearch, CarriedRoll, JobCertificates, SearchWork};
use crate::career::search::{Assumes, CareerSearchResult, SearchRequest, Unproven, solve_kept};
use crate::career::state::{KeptSeen, KeptState, SEEN_KEEP, hash_text, json_without};
use crate::career::{
    CareerJob, CareerSetup, ClassFlags, DeckSearchAudit, Env, JobInputs, SearchState, answer_job, run_career_in,
};
use crate::data::Data;
use crate::deck::CardsRaw;
use crate::refusal::Refusal;
use crate::skills::SkillsRaw;
use crate::uma::UmaIndex;
use serde_json::Value;
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
) -> CarriedRoll {
    let mut search_job = job.clone();
    if let Some(object) = search_job.as_object_mut() {
        object.remove("roll");
    }
    let reply = answer_job(data, skills, cards, &search_job, kept);
    let mut plan_job = roll["job"].clone();
    let roll = match plan_job.get_mut("next").and_then(|next| next.as_object_mut()) {
        Some(next) => {
            next.insert("roll".into(), roll["stars"].clone());
            run_career_in(data, skills, cards, &plan_job, kept).into_roll()
        }
        None => RollReply::Refusal(Refusal::new("roll: the job is not a plan job (no `next`)")),
    };
    CarriedRoll { reply, roll }
}

/// Keep this job's fully checked decks for the next job on its setup, trainees and pool (`seen_fingerprint`; the last
/// `SEEN_KEEP` per class, four keys).
pub(super) fn keep_seen(kept: &mut KeptState, state: &SearchState, seen_fingerprint: u64) {
    let cache = &mut kept.seen;
    cache.retain(|entry| entry.key != seen_fingerprint);
    cache.push(KeptSeen {
        key: seen_fingerprint,
        refs: state.seen.iter().map(|refs| refs[refs.len().saturating_sub(SEEN_KEEP)..].to_vec()).collect(),
    });
    if cache.len() > 4 {
        cache.remove(0);
    }
}

/// Keep this job's class flags for every later job (`KeptState::flags`, by class content `CareerSetup::class_key`):
/// the job started from the kept ones (`career_setup`), so its own are their union with what it flagged.
pub(super) fn keep_flags(kept: &mut KeptState, setup: &CareerSetup, state: &SearchState) {
    for (key, flags) in setup.keyed_flags(state) {
        let slot = kept.flags.entry(key).or_default();
        *slot = slot.union(flags);
    }
}

/// A roll of a plan this worker keeps (`next.roll`): answered from its roll state, no search (`plan::roll::roll_kept`);
/// None when the job carries no roll or the worker keeps no state for it (the job then runs, its plan answering it).
pub(super) fn answer_kept_roll<'data>(
    env: &mut Env<'data>,
    kept: &mut KeptState,
    run: &CareerRun<'_, 'data>,
) -> Option<RollReply> {
    let career_job = run.job;
    let plan_job = career_job
        .next
        .as_ref()
        .filter(|next_job| next_job.get("roll").is_some_and(|roll_value| !roll_value.is_null()))
        .and_then(|next_job| serde_json::from_value::<crate::career::plan::PlanJob>(next_job.clone()).ok())?;
    crate::career::plan::roll::roll_kept(env, kept, run.input, &plan_job)
}

/// The check of a plan this worker keeps (`next.robust`): answered from its check state, no search
/// (`plan::robust::check_kept`); None when the job asks none or the worker keeps no state for it (the job then runs, its
/// plan answering it).
pub(super) fn answer_kept_check<'data>(
    env: &mut Env<'data>,
    kept: &mut KeptState,
    run: &CareerRun<'_, 'data>,
) -> Option<crate::career::plan::report::CheckReply> {
    let career_job = run.job;
    let plan_job = career_job
        .next
        .as_ref()
        .filter(|next_job| next_job.get("robust").is_some_and(|request| !request.is_null()))
        .and_then(|next_job| serde_json::from_value::<crate::career::plan::PlanJob>(next_job.clone()).ok())?;
    let pool = crate::career::plan::ScoringPool {
        legacies: &career_job.legacies,
        pool: run.pool,
        pick_pool: run.pick_pool.unwrap_or(run.pool),
        fixed_parents: &career_job.fixed_parents,
    };
    crate::career::plan::check_before_plan(env, kept, run.input, pool, &plan_job)
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
    work: SearchWork,
    rho_top: f64,
) -> CareerSearch {
    let career_job = run.job;
    let results = if career_job.no_reports {
        vec![]
    } else {
        search
            .found
            .iter()
            .enumerate()
            .map(|(i, found)| env.setup.report(&mut env.state, run.pool, found, i == 0))
            .collect()
    };
    CareerSearch {
        results,
        candidates: search.candidates,
        solved: search.solved,
        bounded: search.bounded,
        scored: search.scored,
        skipped: search.skipped,
        next_bound: Some(search.next_bound),
        // a deck band over EXACT_CAP decks (in the main search or any later one of this job) was certified at a lower
        // cap only: `cap` says which, `complete` is false
        complete: search.complete && !(search.certified_cap < rho_top),
        proven_top: search.proven,
        cap: rho_top.min(search.certified_cap),
        class_caps: env.setup.space.caps(),
        work,
        given: None,
        // why the main search is not complete (`Unproven`: "work_cap", "band_overflow", "cap_breach"); `job_unproven`
        // (`note_job_certificates`) has every search of the job's
        unproven: search.unproven.any().then_some(search.unproven),
        // the named assumptions its proof rests on beyond the derived bounds and the measured class caps (`Assumes`:
        // "two_feeder_cap", "multi_feeder_cap"; docs/SOLVER.md, *What "proven" rests on*)
        assumes: (search.assumes != Assumes::default()).then_some(search.assumes),
        compare: career_job.compare.then_some(true),
        next: None,
        certificates: JobCertificates::default(),
        timing_s: 0.0,
    }
}

/// The job's `next`: the page's next career (`plan::index_rule`) on the main search `search`.
pub(super) fn plan_next<'data>(
    env: &mut Env<'data>,
    kept: &mut KeptState,
    run: &CareerRun<'_, 'data>,
    search: &CareerSearchResult,
    next: &Value,
) -> PlanReply {
    let career_job = run.job;
    let settings = match serde_json::from_value::<crate::career::plan::PlanJob>(next.clone()) {
        Ok(settings) => settings,
        Err(e) => return PlanReply::Refusal(Refusal::new(format!("next: {e}"))),
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

/// The main search's output (`search_output`) qualified by its trainees' class flags `flags` as they stand at the end
/// of the job (`CareerSearchResult::qualify`): a class a later search flagged (the plan's, or one of an earlier job on
/// the deck space) unproves it (`unproven`, `complete` and `proven_top` false), and an assuming one adds to `assumes`.
fn qualify_output(out: &mut CareerSearch, flags: ClassFlags) {
    let mut unproven = out.unproven.unwrap_or_default();
    unproven.cap_breach |= flags.breached;
    if unproven.any() {
        out.unproven = Some(unproven);
        out.complete = false;
        out.proven_top = false;
    }
    let assumes = out.assumes.unwrap_or_default().union(flags.assumes);
    if assumes != Assumes::default() {
        out.assumes = Some(assumes);
    }
}

/// What every search of the job certified, into its answer: band overflows (`cap`, `complete`), decks past their
/// class's assumed cap, the cap-free searches, the main search qualified by its trainees' (`trainees`) class flags as
/// they stand now (`qualify_output`), and the reasons the job's deck searches (the main search's and the plan's) and
/// the flags of every class it answers for (its trainees', with a plan `CareerSetup::owned`) leave an answer
/// incomplete (`job_unproven`: band overflows and cap breaches; a work budget that ran out shows in the search's own
/// `unproven` and the plan's rows only, a `cap_free` job's cap-free searches aside; then `complete` is false), or
/// rest on a named assumption (`job_assumes`).
pub(super) fn note_job_certificates(
    setup: &CareerSetup,
    state: &SearchState,
    audit: &DeckSearchAudit,
    out: &mut CareerSearch,
    rho_top: f64,
    trainees: &[UmaIndex],
) {
    qualify_output(out, setup.trainee_flags(state, trainees));
    // (the classes the job answers for: the main search's trainees', and with a plan every uma it may pick)
    let answered = if out.next.is_some() { setup.owned.as_slice() } else { trainees };
    let job_flags = setup.trainee_flags(state, answered);
    let certificates = &mut out.certificates;
    if audit.overflows > 0 {
        out.complete = false;
        out.cap = rho_top.min(audit.certified_cap);
        certificates.band_overflows = Some(audit.overflows);
    }
    // a scored deck past its class's assumed cap (in any search of the job): per class [class, ratio, cap]
    if audit.cap_breach > 0 {
        let caps = setup.space.caps();
        certificates.cap_exceeded = Some(
            (0..4)
                .filter(|&k| audit.max_ratio[k] > caps[k] * (1.0 + 1e-9))
                .map(|k| (k, audit.max_ratio[k], caps[k]))
                .collect(),
        );
    }
    // the deck searches that saw a breach and were answered by the cap-free search (`best_deck_cf`; every search of the
    // job, the plan's included), how many of those (or of a `cap_free` job's searches) proved nothing (then `complete`
    // is false), breaches left unanswered (`CAP_RERUN=0`), and the cap-free work
    if setup.space.deck_job.cap_free {
        certificates.cap_free = Some(true);
    }
    let cap_free = &audit.cap_free_stats;
    if *cap_free != CfStats::default() {
        certificates.cap_rerun = Some(CapRerun {
            searches: cap_free.reruns,
            unproven: cap_free.open,
            unanswered: cap_free.unhandled,
            work: cap_free.work,
        });
    }
    // capped deck searches that breached their caps (any search of the job): each such career search is not complete;
    // and the trainees (chara ids) whose deck class has breached in this job or a kept earlier one: no search or plan
    // row of theirs is proven (a row certified before the breach within this job included)
    if audit.capped_breaches > 0 {
        certificates.cap_breaches = Some(audit.capped_breaches);
    }
    let breached: Vec<u32> = setup
        .owned
        .iter()
        .filter(|&&uma| state.class_flags[setup.deck_class[uma]].breached)
        .map(|&uma| setup.data.umas[uma].id)
        .collect();
    if !breached.is_empty() {
        certificates.cap_breached = Some(breached);
    }
    // a scored deck past its class's cap within its nested-chain bound (no breach: proven where the bound is, else
    // under `multi_feeder_cap`)
    if audit.nested_max_ratio > f64::NEG_INFINITY {
        certificates.nested_max_ratio = Some(audit.nested_max_ratio);
    }
    // (a `cap_free` job's cap-free search that proved nothing ran out of its work; elsewhere it is a rerun's)
    // (and the main search's own reasons but its work cap, a kept search's included: it ran in an earlier job)
    let cap_free_job = setup.space.deck_job.cap_free;
    let main_unproven = out.unproven.unwrap_or_default();
    let job_unproven = Unproven {
        work_cap: cap_free_job && cap_free.open > 0,
        band_overflow: audit.overflows > 0 || main_unproven.band_overflow,
        cap_breach: audit.capped_breaches > 0
            || job_flags.breached
            || main_unproven.cap_breach
            || (!cap_free_job && cap_free.open > 0)
            || cap_free.unhandled > 0,
    };
    // (every search of the job and every class it answers for: what `assumes` says of the main search, which a kept
    // search carries without running)
    let job_assumes = Assumes { two_feeder: audit.assumed_searches > 0, multi_feeder: audit.multi_searches > 0 }
        .union(job_flags.assumes)
        .union(out.assumes.unwrap_or_default());
    if job_assumes != Assumes::default() {
        out.certificates.job_assumes = Some(job_assumes);
    }
    if job_unproven.any() {
        out.certificates.job_unproven = Some(job_unproven);
        out.complete = false;
    }
}
