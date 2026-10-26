//! The robustness check on the pick (`next.robust`; docs/SOLVER.md "Next career"): the rows the page shows rescored
//! under model variants that need no career search, and which careers the plan's rule selects under each. A plan keeps
//! what the check reads (`CheckKeep`: its shown rows, fit, candidates and deck cache) and the check runs on it later, a
//! variant per job if asked (`CheckRequest::Next`), so the pick is shown first. It reports and changes nothing.
use crate::career::cache::Cache;
use crate::career::clock::{PROF, now_ms};
use crate::career::plan::combine::{ContEvidence, ContModel, FITTED_MODEL, PlanFit, fitted_continuations};
use crate::career::plan::cont::{
    Cont, ContShape, FITTED_SHAPE, ScoreBase, ScoringKnobs, SideStep, Trio, TrioSideVeteran, Want, trio_v,
};
use crate::career::plan::kept::career_id;
use crate::career::plan::rows::{PickRows, PlanPick, PlanRow, near_floor, pick_of};
use crate::career::plan::score::score_career;
use crate::career::plan::search::{OPT_SHOW, sim_for};
use crate::career::plan::{CheckRequest, PlanCandidates, PlanContext, PlanJob, ScoringContext, found_career};
use crate::career::pool::Pool;
use crate::career::search::Found;
use crate::career::sim::Sim;
use crate::career::state::{KeptState, hash_text, json_without};
use crate::career::{Env, JobInputs, Legacy};
use serde_json::{Value, json};

include!(concat!(env!("OUT_DIR"), "/plan_constants.rs"));

/// Cont's shape at the corners of the range its leave-one-pool-out refits on cost span (val/sim/fit_cont.py on the
/// integrated solver, 2026-10-26: `CEIL_SHARE` 0.5 to 0.65, `LAM_DECAY` 0.6 to 1.3; the plan's 0.6 / 1.5 was kept, as
/// good out of sample)
const SHAPE_CORNERS: [ContShape; 4] = [
    ContShape { ceil_share: 0.5, lam_decay: 0.6 },
    ContShape { ceil_share: 0.5, lam_decay: 1.3 },
    ContShape { ceil_share: 0.65, lam_decay: 0.6 },
    ContShape { ceil_share: 0.65, lam_decay: 1.3 },
];

/// Check states kept (the most recent last): each holds its plan's deck cache.
const CHECK_KEEP: usize = 2;

/// The variants, after the plan's own model: Cont's shape at each corner (each candidate's continuation refitted as
/// the plan fits it), then every career's P(all) times each end of the all-target obs/pred ratio's 95% interval
/// (docs/RESEARCH.md section 13): the candidates' odds now, their lifted odds, C's, the scored career's own full set
/// (`build::scaled_landed_sets`), the continuations refitted on them
fn variants() -> [ContModel; 6] {
    let shape = |shape: ContShape| ContModel { shape, odds_scale: 1.0 };
    let ratio = |odds_scale: f64| ContModel { shape: FITTED_SHAPE, odds_scale };
    [
        shape(SHAPE_CORNERS[0]),
        shape(SHAPE_CORNERS[1]),
        shape(SHAPE_CORNERS[2]),
        shape(SHAPE_CORNERS[3]),
        ratio(P_ALL_RATIO_LOW),
        ratio(P_ALL_RATIO_HIGH),
    ]
}

/// The candidates as the check reads them (`PlanCandidates`' fields of the same names).
struct CheckCandidates {
    trainees: Vec<crate::uma::UmaIndex>,
    current_odds: Vec<f64>,
    certified: Vec<bool>,
    allowed_trainees: Vec<crate::uma::UmaIndex>,
}

/// The plan's fit as the check reads it (`PlanFit`'s fields of the same names).
struct CheckFit {
    evidence: Vec<Option<ContEvidence>>,
    sides: Vec<Vec<TrioSideVeteran>>,
    trios: Vec<Trio>,
    knobs: ScoringKnobs,
}

/// A shown row as the check rescores it: her career and its score under the plan's model.
struct CheckRow {
    trainee: crate::uma::UmaIndex,
    career: Found,
    score: f64,
}

/// What a plan leaves for its check, before the plan's candidates and fit are moved in (`check_draft`).
pub(crate) struct CheckDraft {
    /// The shown rows in the plan's row order (`shown_rows`).
    rows: Vec<CheckRow>,
    /// The plan's pick among them.
    plan_place: usize,
    current_value: f64,
}

impl CheckDraft {
    /// The shown rows' careers (`kept::career_id`), in the plan's row order: `keep_check`'s `row_ids`
    pub(crate) fn row_ids(&self, data: &crate::data::Data, space: &crate::deck::space::DeckSpace) -> Vec<String> {
        self.rows.iter().map(|row| career_id(space, data.umas[row.trainee].id, &row.career)).collect()
    }
}

/// A plan's check state (`KeptState::checks`, by `check_key`; not stored by `state_out`): the checked rows' careers,
/// the variants done and, while any is left, what the next one reads (`CheckWork`), released after the last.
pub(crate) struct CheckKeep {
    key: u64,
    /// The checked rows' careers (`kept::career_id`), in the plan's row order, and the plan's pick among them.
    row_ids: Vec<String>,
    plan_place: usize,
    /// Per variant done, its report (the plan's model first; its `pick` a place in the rows), and the rows the
    /// variants picked (the plan's first).
    entries: Vec<Value>,
    contenders: Vec<usize>,
    check_ms: f64,
    work: Option<CheckWork>,
}

/// What a variant left reads: the checked rows, the fit and candidates they are scored against, and the plan's deck
/// cache (read frozen).
struct CheckWork {
    draft: CheckDraft,
    candidates: CheckCandidates,
    fit: CheckFit,
    cache: Cache,
}

/// A plan job's check key: the whole job but its roll, its check and oracle requests and the page's flags that change
/// no plan (`roll_key`'s, with `next.robust` and `next.oracle`)
pub(crate) fn check_key(job: &Value) -> u64 {
    hash_text(&json_without(job, &["no_reports", "compare", "roll"], &["roll", "robust", "oracle"]))
}

/// A variant's base: the plan's trios and sides (the same careers, veterans and C's) priced under the variant's
/// continuations; per-candidate vectors in candidate order
struct VariantFit {
    continuations: Vec<Cont>,
    step_costs: Vec<f64>,
    sides: Vec<Vec<TrioSideVeteran>>,
    trios: Vec<Trio>,
    current_value: f64,
    odds_scale: f64,
}

impl VariantFit {
    /// The plan's fit refitted under `model`: each continuation from the plan's evidence (`fit_continuation`; a
    /// candidate without any grinds), her step and her other careers' steps at their odds, C's step at her odds scaled
    /// (`1 / (odds x scale)`), V the cheapest trio
    fn new(fit: &CheckFit, candidates: &CheckCandidates, model: ContModel) -> Self {
        let odds_scale = model.odds_scale;
        let continuations = fitted_continuations(&fit.evidence, model);
        let step_costs: Vec<f64> = continuations
            .iter()
            .zip(&candidates.current_odds)
            .map(|(continuation, &odds)| continuation.cost(odds * odds_scale))
            .collect();
        let sides: Vec<Vec<TrioSideVeteran>> = fit
            .sides
            .iter()
            .zip(&continuations)
            .map(|(sides, continuation)| {
                sides
                    .iter()
                    .map(|side| match side.step {
                        SideStep::Alternative(_) => {
                            side.at_step(SideStep::Alternative(continuation.cost(side.odds * odds_scale)))
                        }
                        step @ (SideStep::Existing(_) | SideStep::Current) => side.at_step(step),
                    })
                    .collect()
            })
            .collect();
        let trios: Vec<Trio> =
            fit.trios.iter().map(|trio| Trio { finisher_cost: trio.finisher_cost / odds_scale, ..*trio }).collect();
        let current_value = trio_v(&step_costs, &sides, &trios);
        Self { continuations, step_costs, sides, trios, current_value, odds_scale }
    }
    fn score_base<'fit>(&'fit self, candidates: &'fit CheckCandidates, knobs: ScoringKnobs) -> ScoreBase<'fit> {
        ScoreBase {
            candidates: &candidates.trainees,
            step_costs: &self.step_costs,
            current_value: self.current_value,
            current_odds: &candidates.current_odds,
            certified: &candidates.certified,
            continuations: &self.continuations,
            trios: &self.trios,
            trio_sides: &self.sides,
            finisher_umas: &candidates.allowed_trainees,
            knobs,
            odds_scale: self.odds_scale,
        }
    }
}

/// The rows the check rescores, in the plan's row order (the order its pick rule reads): the scored rows that can be
/// the pick, less those repeating an earlier row's career, that the page shows by score (the first OPT_SHOW, ties in row
/// order) or that are within `near_floor` of the top
fn shown_rows(context: &PlanContext, runs: &[PlanRow]) -> Vec<usize> {
    let mut rows: Vec<usize> = (0..runs.len())
        .filter(|&i| {
            context.input.is_runnable(&runs[i].career)
                && runs[i].net_saving().is_some_and(f64::is_finite)
                && !(0..i).any(|j| runs[j].repeats(&runs[i]))
        })
        .collect();
    let mut by_score = rows.clone();
    by_score.sort_by(|&first, &second| runs[second].total().total_cmp(&runs[first].total()));
    let Some(&top_row) = by_score.first() else { return rows };
    let floor = near_floor(runs[top_row].total());
    let shown: Vec<usize> = by_score.iter().take(OPT_SHOW).copied().collect();
    rows.retain(|&i| shown.contains(&i) || runs[i].total() >= floor);
    rows
}

/// What the plan's check will read of its rows (`CheckDraft`): None where the check does not apply, the plan pricing a
/// given fit (`PlanJob::fit`: refitting it would not be the plan's model) or scoring nothing against a finite V
pub(crate) fn check_draft(
    context: &PlanContext,
    runs: &[PlanRow],
    fit: &PlanFit,
    any_scored: bool,
) -> Option<CheckDraft> {
    let input = context.input;
    if input.settings.fit.is_some() || !any_scored || !fit.current_value.is_finite() {
        return None;
    }
    let shown = shown_rows(context, runs);
    let undup: Vec<usize> = (0..runs.len()).map(|i| (0..i).find(|&j| runs[j].repeats(&runs[i])).unwrap_or(i)).collect();
    let PlanPick { row: plan_pick, .. } = pick_of(runs, |career| input.is_runnable(career), &undup);
    let plan_place = shown.iter().position(|&i| i == plan_pick)?;
    // (the check state outlives the plan's rows)
    let rows = shown
        .iter()
        .map(|&i| CheckRow { trainee: runs[i].trainee, career: runs[i].career.clone(), score: runs[i].total() })
        .collect();
    Some(CheckDraft { rows, plan_place, current_value: fit.current_value })
}

/// What a plan's check is kept from (`keep_check`): the plan's candidates and fit, moved in, and its deck cache.
pub(crate) struct CheckSource {
    pub(crate) candidates: PlanCandidates,
    pub(crate) fit: PlanFit,
    pub(crate) cache: Cache,
}

/// Keep a plan's check state (`CheckKeep`) under `key`: its draft, and its source moved in.
pub(crate) fn keep_check(
    worker: &mut KeptState,
    row_ids: Vec<String>,
    key: u64,
    draft: CheckDraft,
    source: CheckSource,
) {
    let CheckSource { candidates, fit, cache } = source;
    let PlanCandidates { trainees, current_odds, certified, allowed_trainees, .. } = candidates;
    let PlanFit { evidence, sides, trios, knobs, .. } = fit;
    let as_planned = draft.rows.iter().map(|row| row.score).collect::<Vec<_>>();
    let planned_pick = pick_among(&draft, &as_planned);
    debug_assert_eq!(planned_pick.pick, draft.plan_place, "the rule over the shown rows is the plan's pick");
    let check = CheckKeep {
        key,
        row_ids,
        plan_place: draft.plan_place,
        entries: vec![variant_entry(FITTED_MODEL, draft.current_value, &planned_pick)],
        contenders: vec![draft.plan_place],
        check_ms: 0.0,
        work: Some(CheckWork {
            candidates: CheckCandidates { trainees, current_odds, certified, allowed_trainees },
            fit: CheckFit { evidence, sides, trios, knobs },
            cache,
            draft,
        }),
    };
    worker.checks.retain(|kept| kept.key != check.key);
    worker.checks.push(check);
    while worker.checks.len() > CHECK_KEEP {
        worker.checks.remove(0);
    }
}

/// One variant's pick among the shown rows.
struct VariantPick {
    /// The shown row picked (its place in the shown rows).
    pick: usize,
    score: f64,
    /// The pick's score less the best other shown row's (negative where the rule took her by odds within NEAR).
    margin: f64,
}

/// The plan's pick rule (`PickRows`) over the shown rows at `scores`, in the plan's row order (every shown row can be
/// the pick); under the plan's own scores it is the plan's pick (the rows it skips are below `near_floor`)
fn pick_among(draft: &CheckDraft, scores: &[f64]) -> VariantPick {
    let rows = PickRows {
        count: draft.rows.len(),
        score: |k: usize| scores[k],
        odds: |k: usize| draft.rows[k].career.value,
        can_be_pick: |_: usize| true,
    };
    let PlanPick { row: pick, .. } = rows.pick();
    let runner_up = (0..scores.len()).filter(|&k| k != pick).map(|k| scores[k]).fold(f64::NEG_INFINITY, f64::max);
    VariantPick { pick, score: scores[pick], margin: scores[pick] - runner_up }
}

fn variant_entry(model: ContModel, current_value: f64, pick: &VariantPick) -> Value {
    json!({"ceil_share": model.shape.ceil_share, "lam_decay": model.shape.lam_decay, "odds_scale": model.odds_scale,
        "v0": current_value, "pick": pick.pick, "score": pick.score, "margin": pick.margin})
}

/// `run` beside the plan: its scorings read the cached decks only (`Evaluator::frozen`: no deck search, so no deck,
/// proof or flag the plan's own output reads changes), and the plan's profile counters are put back after it (its
/// scorings are not the plan's work)
pub(crate) fn aside<T>(simulator: &mut Sim, run: impl FnOnce(&mut Sim) -> T) -> T {
    let plan_profile = PROF.with(|counters| std::mem::take(&mut *counters.borrow_mut()));
    simulator.evaluator.frozen = true;
    let result = run(simulator);
    simulator.evaluator.frozen = false;
    PROF.with(|counters| *counters.borrow_mut() = plan_profile);
    result
}

impl CheckWork {
    /// A variant: the checked rows rescored exactly (`score_career`, `Want::Score`) under `model` on the plan's cache
    /// (in the simulator), its V and its pick
    fn run_variant(&self, simulator: &mut Sim, pool: ScoringPool, model: ContModel) -> (f64, VariantPick) {
        let variant = VariantFit::new(&self.fit, &self.candidates, model);
        let score_base = variant.score_base(&self.candidates, self.fit.knobs);
        let scoring = ScoringContext { score_base: &score_base, legacies: pool.legacies, pool: pool.pool };
        let mut buf = vec![];
        let scores: Vec<f64> = aside(simulator, |simulator| {
            self.draft
                .rows
                .iter()
                .map(|row| {
                    let career = found_career(row.trainee, &row.career);
                    score_career(simulator, &scoring, career, Want::Score, &mut buf).expected_saving - 1.0
                })
                .collect()
        });
        (variant.current_value, pick_among(&self.draft, &scores))
    }
}

impl CheckKeep {
    /// The variants left (none once every one is done).
    fn pending(&self) -> usize {
        variants().len() + 1 - self.entries.len()
    }
    /// Record a variant done.
    fn record(&mut self, model: ContModel, current_value: f64, pick: &VariantPick) {
        if !self.contenders.contains(&pick.pick) {
            self.contenders.push(pick.pick);
        }
        self.entries.push(variant_entry(model, current_value, pick));
    }
    /// The check's report so far (`next.robust`): `pending` the variants left; `stable` and `contenders` over those done.
    fn report(&self) -> Value {
        let id_of = |place: usize| self.row_ids[place].clone();
        let entries: Vec<Value> = self
            .entries
            .iter()
            .map(|entry| {
                let mut entry = entry.clone();
                entry["pick"] = json!(id_of(entry["pick"].as_u64().unwrap_or(0) as usize));
                entry
            })
            .collect();
        json!({
            "plan": id_of(self.plan_place),
            "rows": self.row_ids,
            "variants": entries,
            "pending": self.pending(),
            "stable": self.contenders.len() == 1,
            "contenders": self.contenders.iter().map(|&place| id_of(place)).collect::<Vec<_>>(),
            "check_ms": self.check_ms,
        })
    }
}

/// The pool a check's scorings read: the plan job's own.
#[derive(Clone, Copy)]
pub(crate) struct ScoringPool<'pool> {
    pub(crate) legacies: &'pool [Legacy],
    pub(crate) pool: &'pool Pool,
}

/// A check job answered before the job would run (`career::run::answer_kept_check`): `check_kept`, but a job that
/// also asks the oracle (`next.oracle`) is never answered from the kept state, which has none: `Next` is refused,
/// `All` left to the plan (None: it runs, answering both)
pub(crate) fn check_before_plan<'data>(
    env: &mut Env<'data>,
    worker: &mut KeptState,
    input: &JobInputs<'data>,
    pool: ScoringPool,
    plan_job: &PlanJob,
) -> Option<Value> {
    if plan_job.oracle.is_some() {
        return (plan_job.robust == Some(CheckRequest::Next))
            .then(|| json!({"error": "next.robust \"next\" answers from the kept plan, which has no oracle: ask next.oracle with \"all\" or alone"}));
    }
    check_kept(env, worker, input, pool, plan_job)
}

/// The plan's check (`next.robust`) from its kept state (`CheckKeep`, by `check_key`), with no search: the next
/// variant (`CheckRequest::Next`) or every one left (`All`), on a simulator holding the plan's deck cache, and the
/// report so far. None when the job asks no check, or asks `All` and the worker keeps no state for it (the plan then
/// runs and keeps one, `index_rule`); `Next` without one is answered with an error, no plan run
pub(crate) fn check_kept<'data>(
    env: &mut Env<'data>,
    worker: &mut KeptState,
    input: &JobInputs<'data>,
    pool: ScoringPool,
    plan_job: &PlanJob,
) -> Option<Value> {
    let request = plan_job.robust?;
    let key = check_key(input.job);
    let Some(index) = worker.checks.iter().position(|check| check.key == key) else {
        // (the page's one-variant job never runs the plan behind the player's next job)
        return (request == CheckRequest::Next).then(|| json!({"error": "no kept plan to check", "unavailable": true}));
    };
    let mut check = worker.checks.remove(index);
    let started_ms = now_ms();
    if let Some(mut work) = check.work.take() {
        match sim_for(env, worker, input, plan_job) {
            Ok(mut simulator) => {
                std::mem::swap(&mut simulator.evaluator.cache, &mut work.cache);
                while check.pending() > 0 {
                    let model = variants()[check.entries.len() - 1];
                    let (current_value, pick) = work.run_variant(&mut simulator, pool, model);
                    check.record(model, current_value, &pick);
                    if request == CheckRequest::Next {
                        break;
                    }
                }
                std::mem::swap(&mut simulator.evaluator.cache, &mut work.cache);
            }
            Err(error) => return Some(error),
        }
        // (put back while a variant is left; after the last it goes, the report alone kept)
        if check.pending() > 0 {
            check.work = Some(work);
        }
    }
    check.check_ms += now_ms() - started_ms;
    let report = check.report();
    worker.checks.push(check);
    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pick rule over a variant's scores: the top, or within NEAR of it the better odds; equal scores and odds
    /// keep the earlier row, as the plan's rule does
    #[test]
    fn a_variant_picks_by_the_plans_rule() {
        let odds = [0.01, 0.02, 0.05];
        let pick = |scores: [f64; 3]| {
            PickRows { count: 3, score: |k: usize| scores[k], odds: |k: usize| odds[k], can_be_pick: |_: usize| true }
                .pick()
                .row
        };
        // (2.9 is within NEAR 0.25 of 3.0, with the better odds)
        assert_eq!(pick([3.0, 2.9, 1.0]), 1);
        assert_eq!(pick([3.0, 2.0, 1.0]), 0);
        let even = [0.02, 0.02, 0.02];
        let tied = PickRows { count: 3, score: |_: usize| 1.0, odds: |k: usize| even[k], can_be_pick: |_: usize| true };
        assert_eq!(tied.pick().row, 0);
    }
}
