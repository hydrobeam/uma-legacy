//! The robustness check on the pick (`next.robust`; docs/SOLVER.md "Next career"): the rows the page shows rescored
//! under model variants that need no career search, and which careers the plan's rule selects under each. A plan keeps
//! what the check reads (`CheckKeep`: its shown rows, fit, candidates and deck cache) and the check runs on it later, a
//! variant per job if asked (`CheckRequest::Next`), so the pick is shown first. It reports and changes nothing.
use crate::career::cache::Cache;
use crate::career::clock::{PROF, now_ms};
use crate::career::plan::combine::{ContEvidence, ContModel, PlanFit, fit_continuation};
use crate::career::plan::combine::{PairCareer, TrioSides, exact_pair_career, rechoose_finishers};
use crate::career::plan::cont::{
    Cont, ContShape, FITTED_SHAPE, OddsScales, ScoreBase, ScoringKnobs, SideStep, Trio, TrioSideVeteran, Want, trio_v,
};
use crate::career::plan::kept::career_id;
use crate::career::plan::pair_bound::{PAIR_SEARCH_PAIRS, trainee_bounded_pairs};
use crate::career::plan::rows::{PickRows, PlanPick, PlanRow, near_floor, pick_of};
use crate::career::plan::score::FinisherChoice;
use crate::career::plan::score::score_career;
use crate::career::plan::search::{OPT_SHOW, sim_for};
use crate::career::plan::sweep::deck_odds;
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

/// A variant the check rescores the rows under: its kind (`VariantKind`), Cont's shape and every uma's odds scale
/// (`OddsScales`). Every variant refits each candidate's continuation from the plan's evidence at her own uma's scale and
/// drops the plan's step caps (`caps`: a kept fit's cap is a price under the plan's model, not the variant's)
#[derive(Clone, Copy)]
struct CheckVariant {
    kind: VariantKind,
    shape: ContShape,
    scales: OddsScales,
}

/// What a variant changes.
#[derive(Clone, Copy)]
enum VariantKind {
    /// The plan's own model with its step caps dropped: the control that separates the caps' removal, which every
    /// variant makes, from the variants' own changes (where the plan read no cap, its scores are the plan's)
    Nominal,
    /// Cont's shape at a corner of its fitted range (`SHAPE_CORNERS`).
    Shape,
    /// Every career's P(all) times an end of the all-target ratio's interval.
    Ratio,
    /// The plan's strict argmax's uma's P(all) down and one competitor's up (`differential_variants`).
    Differential,
}

impl VariantKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::Shape => "shape",
            Self::Ratio => "ratio",
            Self::Differential => "differential",
        }
    }
}

/// The variants every check runs, after the plan's own model: the uncapped nominal control, Cont's shape at each corner
/// (each candidate's continuation refitted as the plan fits it), then every career's P(all) times each end of the
/// all-target obs/pred ratio's 95% interval (docs/RESEARCH.md section 13): the candidates' odds now, their lifted odds,
/// C's, the scored career's own full set (`build::scaled_landed_sets`), the continuations refitted on them. A uniform
/// scale leaves a finishing career's score p V - 1 unchanged (p and V scale inversely), so the ratio variants test the
/// continuations' and sides' response, not the calibration of one career against another: the differential ones do
fn fixed_variants() -> [CheckVariant; 7] {
    let shape = |shape: ContShape| CheckVariant { kind: VariantKind::Shape, shape, scales: OddsScales::ONE };
    let ratio =
        |scale: f64| CheckVariant { kind: VariantKind::Ratio, shape: FITTED_SHAPE, scales: OddsScales::uniform(scale) };
    [
        CheckVariant { kind: VariantKind::Nominal, shape: FITTED_SHAPE, scales: OddsScales::ONE },
        shape(SHAPE_CORNERS[0]),
        shape(SHAPE_CORNERS[1]),
        shape(SHAPE_CORNERS[2]),
        shape(SHAPE_CORNERS[3]),
        ratio(P_ALL_RATIO_LOW),
        ratio(P_ALL_RATIO_HIGH),
    ]
}

/// A differential scenario's multipliers on P(all): the strict argmax's uma's down, one competitor's up. Measured spread,
/// inferred adversarial split (m = 1.386 tau), weakly identified (.review/planner/calib/SUMMARY.md, a scratch measurement
/// on the run data, not a research finding): the spread of log obs/pred between outfits, tau about 0.2, split between
/// two careers to cover 95% of random pairs' relative error. Per uma, which is per outfit here: the data model one outfit per uma
/// (`outfit_aptitudes` empty for every uma in both eras)
const DIFFERENTIAL_DOWN: f64 = 0.76;
const DIFFERENTIAL_UP: f64 = 1.32;

/// Competitors a check raises against its strict argmax, one differential scenario each.
const DIFFERENTIAL_COMPETITORS: usize = 3;

/// The differential scenarios (`VariantKind::Differential`): for each of the DIFFERENTIAL_COMPETITORS best-scoring other
/// umas among the checked rows (by their best row's score under the plan's model), the plan's model with the strict
/// argmax's uma's P(all) times DIFFERENTIAL_DOWN and that competitor's times DIFFERENTIAL_UP: her odds now and lifted,
/// as C (each trio's C chosen again at the priced odds, `rechoose_finishers`; each route's C), her own landed sets, her
/// continuation refitted at her scaled odds, every other uma's as is. Approximation: the builders' lift evidence (their
/// lift chances and mean lifted odds, `ContEvidence`) is the plan's, not reweighted for the scaled umas' outcomes
fn differential_variants(draft: &CheckDraft, argmax: usize) -> Vec<CheckVariant> {
    let pick_uma = draft.rows[argmax].trainee;
    let mut competitors: Vec<(crate::uma::UmaIndex, f64)> = vec![];
    for row in draft.rows.iter().filter(|row| row.trainee != pick_uma) {
        match competitors.iter_mut().find(|(uma, _)| *uma == row.trainee) {
            Some(entry) => entry.1 = entry.1.max(row.score),
            None => competitors.push((row.trainee, row.score)),
        }
    }
    // (a stable sort: equal scores in row order)
    competitors.sort_by(|first, second| second.1.total_cmp(&first.1));
    competitors
        .iter()
        .take(DIFFERENTIAL_COMPETITORS)
        .map(|&(uma, _)| CheckVariant {
            kind: VariantKind::Differential,
            shape: FITTED_SHAPE,
            scales: OddsScales { common: 1.0, scaled: [(pick_uma, DIFFERENTIAL_DOWN), (uma, DIFFERENTIAL_UP)] },
        })
        .collect()
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
    /// The checked rows' careers (`kept::career_id`), in the plan's row order, their own odds (P(all)), and the plan's
    /// pick among them.
    row_ids: Vec<String>,
    row_odds: Vec<f64>,
    plan_place: usize,
    /// Per variant done, its report (the plan's model first; its `pick` a place in the rows), and the rows the
    /// variants picked (the plan's first).
    entries: Vec<Value>,
    contenders: Vec<usize>,
    /// The variants this check runs, in order (`fixed_variants`, then `differential_variants`).
    variants: Vec<CheckVariant>,
    /// The rows the differential scenarios picked (places in the rows), in the order first picked.
    differential_picks: Vec<usize>,
    /// The within-trainee pair search's report (`CheckWork::pair_search`), once run.
    pairs: Option<Value>,
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
    hash_text(&json_without(job, &["no_reports", "compare", "roll"], &["roll", "robust", "oracle", "pair_search"]))
}

/// A variant's base: the plan's trios and sides (the same careers, veterans and C's) priced under the variant's
/// continuations; per-candidate vectors in candidate order
struct VariantFit {
    continuations: Vec<Cont>,
    step_costs: Vec<f64>,
    sides: Vec<Vec<TrioSideVeteran>>,
    trios: Vec<Trio>,
    current_value: f64,
    odds_scales: OddsScales,
}

impl VariantFit {
    /// The plan's fit refitted under `variant`: each continuation from the plan's evidence at her uma's scale
    /// (`fit_continuation`; a candidate without any grinds), her step and her other careers' steps at their odds so
    /// scaled, C's step at her odds times her uma's scale (`1 / (odds x scale)`), V the cheapest trio
    fn new(fit: &CheckFit, candidates: &CheckCandidates, variant: CheckVariant) -> Self {
        let odds_scales = variant.scales;
        let scale_of = |candidate: usize| odds_scales.of(candidates.trainees[candidate]);
        let continuations: Vec<Cont> = fit
            .evidence
            .iter()
            .enumerate()
            .map(|(candidate, evidence)| {
                let model = ContModel { shape: variant.shape, odds_scale: scale_of(candidate) };
                evidence.as_ref().map_or_else(Cont::default, |evidence| fit_continuation(evidence, model).0)
            })
            .collect();
        let step_costs: Vec<f64> = continuations
            .iter()
            .zip(&candidates.current_odds)
            .enumerate()
            .map(|(candidate, (continuation, &odds))| continuation.cost(odds * scale_of(candidate)))
            .collect();
        let sides: Vec<Vec<TrioSideVeteran>> = fit
            .sides
            .iter()
            .zip(&continuations)
            .enumerate()
            .map(|(candidate, (sides, continuation))| {
                let odds_scale = scale_of(candidate);
                sides
                    .iter()
                    .map(|side| match side.step {
                        // (admitted only where she grinds at once, `plan_sides`: one that builds under the variant is
                        // out of its trios, at no finite cost)
                        SideStep::Alternative(_) => {
                            let plan = continuation.plan(side.odds * odds_scale);
                            let step = if plan.building_careers == 0.0 { plan.expected_careers } else { f64::INFINITY };
                            side.at_step(SideStep::Alternative(step))
                        }
                        step @ (SideStep::Existing(_) | SideStep::Current) => side.at_step(step),
                    })
                    .collect()
            })
            .collect();
        let trios: Vec<Trio> = fit
            .trios
            .iter()
            .map(|trio| Trio { finisher_cost: trio.finisher_cost / odds_scales.of(trio.finisher), ..*trio })
            .collect();
        let current_value = trio_v(&step_costs, &sides, &trios);
        Self { continuations, step_costs, sides, trios, current_value, odds_scales }
    }
    fn score_base<'fit>(&'fit self, candidates: &'fit CheckCandidates, knobs: ScoringKnobs) -> ScoreBase<'fit> {
        ScoreBase {
            candidates: &candidates.trainees,
            step_costs: &self.step_costs,
            current_value: self.current_value,
            current_odds: &candidates.current_odds,
            certified: &candidates.certified,
            continuations: &self.continuations,
            // (no step caps: a kept fit's cap is a price under the plan's model, not the variant's)
            step_caps: &[],
            trios: &self.trios,
            trio_sides: &self.sides,
            finisher_umas: &candidates.allowed_trainees,
            knobs,
            odds_scales: self.odds_scales,
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
    // (the plan's model's own strict argmax: the plan's pick where no row within NEAR has better odds, `PickRows::pick`)
    let planned_pick = pick_among(&draft, &as_planned);
    let row_odds = draft.rows.iter().map(|row| row.career.value).collect();
    let check = CheckKeep {
        key,
        row_ids,
        row_odds,
        plan_place: draft.plan_place,
        entries: vec![variant_entry("plan", FITTED_SHAPE, OddsScales::ONE, draft.current_value, &planned_pick)],
        variants: fixed_variants().into_iter().chain(differential_variants(&draft, planned_pick.pick)).collect(),
        contenders: vec![planned_pick.pick],
        differential_picks: vec![],
        pairs: None,
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

/// One variant's pick among the shown rows and the scorings' memory-budget truncations behind it.
struct VariantPick {
    /// The shown row picked (its place in the shown rows).
    pick: usize,
    score: f64,
    /// The pick's score less the best other shown row's (at least -1e-9: a strict argmax).
    margin: f64,
    /// Every shown row's score, in the plan's row order.
    scores: Vec<f64>,
    /// The scorings' truncations (`build::score_career`'s `sc_routes_dropped` and `sc_partner_setups_dropped`: routes
    /// and partner setups the memory budget left out, each lowering a row's saving, never raising it), summed over the
    /// rows; None for the plan's own scores, made during the plan. A variant's pick that moves with them can be the
    /// budget's, not the model's
    truncations: Option<Truncations>,
}

/// The memory budget's truncations over a variant's scorings (`VariantPick::truncations`).
#[derive(Clone, Copy)]
struct Truncations {
    routes_dropped: f64,
    partner_setups_dropped: f64,
}

/// The strict argmax (`PickRows::argmax`, no NEAR band) over the shown rows at `scores`, in the plan's row order (every
/// shown row can be the pick): each variant's own best row, so that variants that disagree show the pick varying
fn pick_among(draft: &CheckDraft, scores: &[f64]) -> VariantPick {
    let rows = PickRows {
        count: draft.rows.len(),
        score: |k: usize| scores[k],
        odds: |k: usize| draft.rows[k].career.value,
        can_be_pick: |_: usize| true,
    };
    let pick = rows.argmax();
    let runner_up = (0..scores.len()).filter(|&k| k != pick).map(|k| scores[k]).fold(f64::NEG_INFINITY, f64::max);
    VariantPick {
        pick,
        score: scores[pick],
        margin: scores[pick] - runner_up,
        scores: scores.to_vec(),
        truncations: None,
    }
}

/// A variant's report entry: `kind` "plan" for the plan's own scores, else the variant's (`VariantKind::name`).
fn variant_entry(
    kind: &str,
    variant_shape: ContShape,
    scales: OddsScales,
    current_value: f64,
    pick: &VariantPick,
) -> Value {
    let truncations = pick.truncations.map_or(
        json!({"routes_dropped": null, "partner_setups_dropped": null}),
        |t| json!({"routes_dropped": t.routes_dropped, "partner_setups_dropped": t.partner_setups_dropped}),
    );
    // (the scaled umas: uma indices, `Data::umas`, with their scales; none for a uniform scale)
    let scaled: Vec<Value> = scales
        .scaled
        .iter()
        .filter(|&&(uma, _)| uma != crate::uma::NONE)
        .map(|&(uma, scale)| json!([uma.0, scale]))
        .collect();
    json!({"kind": kind, "ceil_share": variant_shape.ceil_share, "lam_decay": variant_shape.lam_decay, "odds_scale": scales.common, "scaled": scaled,
        "v0": current_value, "pick": pick.pick, "score": pick.score, "margin": pick.margin, "scores": pick.scores,
        "routes_dropped": truncations["routes_dropped"], "partner_setups_dropped": truncations["partner_setups_dropped"]})
}

/// `run` beside the plan: its scorings read the cached decks only (`Evaluator::frozen`: no deck search, so no deck,
/// proof or flag the plan's own output reads changes), and the plan's profile counters are put back after it (its
/// scorings are not the plan's work)
pub(crate) fn aside<T>(simulator: &mut Sim, run: impl FnOnce(&mut Sim) -> T) -> T {
    aside_counted(simulator, run).0
}

/// `aside`, and the profile counters `run` added (`PROF`), which the plan's are not.
fn aside_counted<T>(simulator: &mut Sim, run: impl FnOnce(&mut Sim) -> T) -> (T, ProfileCounters) {
    let plan_profile = PROF.with(|counters| std::mem::take(&mut *counters.borrow_mut()));
    simulator.evaluator.frozen = true;
    let result = run(simulator);
    simulator.evaluator.frozen = false;
    let run_profile = PROF.with(|counters| std::mem::replace(&mut *counters.borrow_mut(), plan_profile));
    (result, run_profile)
}

/// A run's profile counters by name (`clock::PROF`'s map).
type ProfileCounters = std::collections::BTreeMap<&'static str, f64>;

impl CheckWork {
    /// The plan's fit refitted under `check_variant` (`VariantFit::new`). A differential scenario prices one or two
    /// umas apart from the rest, so a trio's best C can change: each trio's C is chosen again at the scenario's odds
    /// (`rechoose_finishers`, on the plan's cache), and V with it; a uniform scale keeps every C's order. Reads the
    /// deck cache: call it within `aside`
    fn variant_fit(&self, simulator: &mut Sim, pool: ScoringPool, check_variant: CheckVariant) -> VariantFit {
        let mut variant = VariantFit::new(&self.fit, &self.candidates, check_variant);
        if matches!(check_variant.kind, VariantKind::Differential) {
            let mut buf = vec![];
            let sides = TrioSides { trio_sides: &variant.sides, candidates: &self.candidates.trainees };
            let finishers = FinisherChoice { umas: &self.candidates.allowed_trainees, scales: variant.odds_scales };
            let trios = rechoose_finishers(simulator, pool.pool, sides, finishers, &self.fit.trios, &mut buf);
            variant.current_value = trio_v(&variant.step_costs, &variant.sides, &trios);
            variant.trios = trios;
        }
        variant
    }
    /// A variant: the checked rows rescored exactly (`score_career`, `Want::Score`) under `model` on the plan's cache
    /// (in the simulator), its V and its pick
    fn run_variant(&self, simulator: &mut Sim, pool: ScoringPool, check_variant: CheckVariant) -> (f64, VariantPick) {
        let variant = aside(simulator, |simulator| self.variant_fit(simulator, pool, check_variant));
        let score_base = variant.score_base(&self.candidates, self.fit.knobs);
        let scoring = ScoringContext { score_base: &score_base, legacies: pool.legacies, pool: pool.pool };
        let mut buf = vec![];
        let (scores, counters): (Vec<f64>, _) = aside_counted(simulator, |simulator| {
            self.draft
                .rows
                .iter()
                .map(|row| {
                    let career = found_career(row.trainee, &row.career);
                    score_career(simulator, &scoring, career, Want::Score, &mut buf).expected_saving - 1.0
                })
                .collect()
        });
        let counter = |name: &str| counters.get(name).copied().unwrap_or(0.0);
        let truncations = Truncations {
            routes_dropped: counter("sc_routes_dropped"),
            partner_setups_dropped: counter("sc_partner_setups_dropped"),
        };
        let pick = VariantPick { truncations: Some(truncations), ..pick_among(&self.draft, &scores) };
        (variant.current_value, pick)
    }
}

/// One trainee's within-trainee pair search (`CheckWork::pair_search`).
struct TraineePairs {
    /// Her shown career (her best-scoring checked row) and its score under the nominal model.
    shown: Found,
    shown_score: f64,
    /// Her pairs' careers (her best cached deck on each) with their nominal scores, best first (a stable sort: equal
    /// scores in pair order); the pairs scored, her runnable pairs, and whether the bound left some unscored
    /// (`trainee_bounded_pairs`).
    ranked: Vec<(f64, Found)>,
    pairs_scored: usize,
    pairs_runnable: usize,
    pairs_truncated: bool,
}

/// The pairs the pair search searches exactly and scores again, best first by cached deck: the best and its next three.
const PAIR_FINALISTS: usize = 4;

/// One finalist pair of the pair search (`CheckWork::finalist`): her career kept on it, the better by score (nominal) of
/// her best-scoring cached deck there and the exact P(all) search's career (ties: the search's), with the scores behind
/// the choice.
struct Finalist {
    found: Found,
    score: f64,
    /// The exact search (`exact_pair_career`: how it ended), its career's score, and whether its career was kept.
    searched: PairCareer,
    searched_score: f64,
    kept_searched: bool,
    /// The score of her P(all)-best cached deck on the pair (the ranking's) and of her best-scoring cached deck there,
    /// and whether those are different decks.
    p_all_deck_score: f64,
    best_cached_score: f64,
    score_deck_differs: bool,
}

impl Finalist {
    /// How its career was chosen: its P(all)-best cached deck's score, its best-scoring cached deck's (and whether that is
    /// another deck), the exact search's career's, and which was kept.
    fn deck_choice(&self) -> Value {
        json!({"p_all_deck_score": self.p_all_deck_score, "best_cached_score": self.best_cached_score,
            "score_deck_differs": self.score_deck_differs, "searched_score": self.searched_score,
            "kept": if self.kept_searched { "searched" } else { "cached" }})
    }
    /// Its report: its career (`id`), score, how its exact search ended and its deck choice.
    fn json(&self, id: String) -> Value {
        json!({"career": id, "score": self.score, "search": self.searched.status.name(), "deck_choice": self.deck_choice()})
    }
}

/// The best trainees by score the pair search runs for beside the pick's and those the per-uma scenarios tie with it
/// (`CheckKeep::pair_search_trainees`).
const PAIR_SEARCH_TOP: usize = 4;

/// The estimated outcomes (`sc_out`) phase 1 of the pair search admits (`admitted_trainees`): a "top" trainee runs only
/// where the estimates of the trainees admitted before her plus her own stay within it. An estimate is the pairs she is
/// scored on (her runnable pairs, at most `PAIR_SEARCH_PAIRS`) times the outcomes one scoring of her shown career
/// evaluates (`CheckWork::estimated_outcomes`), so it bounds neither the work done (her finalists, their exact searches
/// and the scorings' own spread are not in it), nor the pick's and the tied trainees' (they always run, their estimates
/// counted against it; each is scored on at most `PAIR_SEARCH_PAIRS` pairs), nor phase 2's (at most `PAIR_SEARCH_PAIRS`
/// pairs per trainee, of the few shared pairs phase 1 found). Measured
/// 2026-10-26 (native, about 3.9 million a second on p16 and gm16_nu8): gm16_nu8's four trainees take 24.5 million,
/// p16_t10's pick 19.6 million (a second trainee there would take the pair search past 15 s)
const PAIR_SEARCH_OUTCOMES: f64 = 30e6;

/// A trainee the pair search runs for, and why (`CheckKeep::pair_search_trainees`): "pick" (the plan's pick's), "tied"
/// (a differential scenario's pick's) or "top" (among the `PAIR_SEARCH_TOP` best by score).
#[derive(Clone, Copy)]
struct PairSearchTrainee {
    trainee: crate::uma::UmaIndex,
    reason: &'static str,
}

/// What the pair search's scorings read (`CheckWork::pair_search`): the nominal fit (the plan's, uncapped: one V and one
/// fit for every career it scores, the shown ones rescored under it too), Cont's shape corners, and the pool.
struct PairSearchFits<'fits> {
    nominal: &'fits VariantFit,
    corners: &'fits [VariantFit],
    pool: ScoringPool<'fits>,
}

/// A pair-search report (phase 1's per trainee, phase 2's per finalist) and the career it names, its `career`: what
/// the recommendation reads (`CheckWork::verdict`).
struct ReportedCareer {
    report: Value,
    career: Found,
}

impl ReportedCareer {
    fn score(&self) -> f64 {
        self.report["score"].as_f64().unwrap_or(f64::NEG_INFINITY)
    }
}

/// Phase 2 of the pair search (`CheckWork::shared_pair_report`): its finalists, best first, and its coverage: the
/// trainees it tried the shared pairs for, the careers it scored, the shared pairs they could run (summed over them),
/// and how many of them the bound left pairs unscored for (`trainee_bounded_pairs`).
struct SharedPairs {
    finalists: Vec<ReportedCareer>,
    trainees_tried: usize,
    careers_tried: usize,
    pairs_runnable: usize,
    trainees_truncated: usize,
}

/// Which of `trainees` phase 1 of the pair search searches (`CheckWork::pair_search`), given each one's estimated
/// outcomes (`estimates`, in the same order): the pick's and the tied always; each "top" one, in order, where the
/// estimates of the trainees admitted before her plus her own stay within PAIR_SEARCH_OUTCOMES (one skipped does not
/// stop a cheaper one after her). A function of the trainees, their order and their estimates, not of any search's work
fn admitted_trainees(trainees: &[PairSearchTrainee], estimates: &[f64]) -> Vec<bool> {
    let mut admitted_outcomes = 0.0;
    trainees
        .iter()
        .zip(estimates)
        .map(|(trainee, &estimate)| {
            let admitted = trainee.reason != "top" || admitted_outcomes + estimate <= PAIR_SEARCH_OUTCOMES;
            if admitted {
                admitted_outcomes += estimate;
            }
            admitted
        })
        .collect()
}

/// The trainees phase 2 of the pair search tries the shared pairs for (`CheckWork::shared_pair_report`): every allowed
/// trainee, once, in `allowed` order, but those with a phase-1 report (`reported`). A trainee nominated for phase 1 that
/// the work limit skipped, or that had no checked row, is tried here
fn phase_two_trainees(
    allowed: &[crate::uma::UmaIndex],
    reported: &[crate::uma::UmaIndex],
) -> Vec<crate::uma::UmaIndex> {
    let mut trainees: Vec<crate::uma::UmaIndex> = vec![];
    for &trainee in allowed {
        if !trainees.contains(&trainee) && !reported.contains(&trainee) {
            trainees.push(trainee);
        }
    }
    trainees
}

/// What both phases of the pair search found (`CheckWork::verdict`): phase 1's reports, then phase 2's.
#[derive(Clone, Copy)]
struct PairSearchFound<'found> {
    reports: &'found [ReportedCareer],
    shared: &'found [ReportedCareer],
}

/// The pair search's verdict (`CheckWork::verdict`): its best (trainee, pair) by score (`best`, null with none) and
/// the career the page recommends running (`recommendation`).
struct PairSearchVerdict {
    best: Value,
    recommendation: Value,
}

/// The career the pair search recommends (`CheckWork::verdict`), why (`source`), and its score in the pair search
/// (None for the plan's pick, which the pair search did not score as such).
#[derive(Clone, Copy)]
struct Recommended<'careers> {
    career: &'careers Found,
    source: &'static str,
    score: Option<f64>,
}

/// A career and the one its margin is taken over (`CheckWork::scenario_margins`): the best (trainee, pair) and the
/// pick's side.
#[derive(Clone, Copy)]
struct CareerAgainst<'careers> {
    career: &'careers Found,
    against: &'careers Found,
}

/// A career's margin over the pick's side (`CheckWork::verdict`): under the nominal model, and in the two per-uma
/// scenarios, the pick's uma's P(all) x DIFFERENTIAL_DOWN against the career's uma's x DIFFERENTIAL_UP (`pick_down`)
/// and the reverse (`pick_up`), each trio's C chosen again (`variant_fit`).
#[derive(Clone, Copy)]
struct ScenarioMargins {
    nominal: f64,
    pick_down: f64,
    pick_up: f64,
}

impl ScenarioMargins {
    /// Better under the nominal model and both scenarios.
    fn hold(self) -> bool {
        self.nominal > 0.0 && self.pick_down > 0.0 && self.pick_up > 0.0
    }
    fn json(self) -> Value {
        json!({"nominal": self.nominal, "pick_down": self.pick_down, "pick_up": self.pick_up})
    }
}

/// The pair search's deterministic work so far (`CheckWork::pair_search`): the outcomes its scorings evaluated
/// (`sc_out`, in the profile counters `aside` keeps apart from the plan's) and the deck searches' nodes.
#[derive(Clone, Copy)]
struct PairSearchWork {
    outcomes: f64,
    nodes: f64,
}

impl PairSearchWork {
    fn now(simulator: &Sim) -> Self {
        let outcomes = PROF.with(|counters| counters.borrow().get("sc_out").copied().unwrap_or(0.0));
        Self { outcomes, nodes: simulator.env.setup.space.nodes.get() as f64 }
    }
    fn since(self, start: Self) -> Self {
        Self { outcomes: self.outcomes - start.outcomes, nodes: self.nodes - start.nodes }
    }
    fn json(self) -> Value {
        json!({"outcomes": self.outcomes, "nodes": self.nodes})
    }
}

impl CheckWork {
    /// The pair search (`PlanJob::pair_search`), on the cached decks only (`aside`); it changes no pick. Every career is
    /// scored under the nominal model (`PairSearchFits`).
    ///
    /// Phase 1, per trainee of `trainees` (`trainee_report`): her runnable pairs, at most `PAIR_SEARCH_PAIRS` of them
    /// (`trainee_bounded_pairs`), with her best cached deck, the
    /// `PAIR_FINALISTS` best of them with every cached deck scored and searched exactly (`finalist`), her best pair
    /// against her shown career under the shape corners and her own per-uma scales. Phase 2 (`shared_pair_report`):
    /// the pairs phase 1 found best, each tried for every other allowed trainee with her best cached deck (within the
    /// same bound), the best
    /// `PAIR_FINALISTS` of those as finalists. The verdict (`verdict`): the best (trainee, pair) by score over both
    /// phases (`best`), and the career the page recommends (`recommendation`)
    fn pair_search(&self, simulator: &mut Sim, pool: ScoringPool, trainees: &[PairSearchTrainee]) -> Value {
        let started_ms = now_ms();
        let data = simulator.data;
        let variants = fixed_variants();
        let nominal = VariantFit::new(&self.fit, &self.candidates, variants[0]);
        let corners: Vec<VariantFit> = variants
            .iter()
            .filter(|variant| matches!(variant.kind, VariantKind::Shape))
            .map(|&variant| VariantFit::new(&self.fit, &self.candidates, variant))
            .collect();
        let fits = PairSearchFits { nominal: &nominal, corners: &corners, pool };
        let mut buf = vec![];
        let mut memo: crate::career::plan::combine::PairMemo = crate::FxMap::default();
        let (reports, shared, skipped, verdict) = aside(simulator, |simulator| {
            // Phase 1: the trainees in order, a "top" one only where the estimated outcomes admitted stay within
            // PAIR_SEARCH_OUTCOMES (`admitted_trainees`: deterministic estimates, not time or the order work was done)
            let estimates: Vec<f64> = trainees
                .iter()
                .map(|searched| self.estimated_outcomes(simulator, &fits, searched.trainee, &mut buf))
                .collect();
            let (mut reports, mut skipped): (Vec<ReportedCareer>, Vec<&str>) = (vec![], vec![]);
            for (&searched, admitted) in trainees.iter().zip(admitted_trainees(trainees, &estimates)) {
                if !admitted {
                    skipped.push(data.umas[searched.trainee].name.as_str());
                    continue;
                }
                reports.extend(self.trainee_report(simulator, &fits, searched, &mut memo, &mut buf));
            }
            // Phase 2: the pairs phase 1 found best, for every trainee without a phase-1 report
            let shared = self.shared_pair_report(simulator, &fits, &reports, &mut memo, &mut buf);
            // The verdict: the best over both phases, and what the page recommends
            let found = PairSearchFound { reports: &reports, shared: &shared.finalists };
            let verdict = self.verdict(simulator, &fits, found, &mut buf);
            (reports, shared, skipped, verdict)
        });
        let reports_json = |reported: Vec<ReportedCareer>| reported.into_iter().map(|r| r.report).collect::<Vec<_>>();
        let shared_coverage = json!({"trainees": shared.trainees_tried, "careers": shared.careers_tried,
            "pairs_runnable": shared.pairs_runnable, "trainees_truncated": shared.trainees_truncated});
        json!({"trainees": reports_json(reports), "shared": reports_json(shared.finalists),
            "shared_coverage": shared_coverage, "best": verdict.best, "recommendation": verdict.recommendation,
            "skipped_for_work": skipped, "pair_bound": PAIR_SEARCH_PAIRS, "pairs_ms": now_ms() - started_ms})
    }
    /// The pair search's verdict (`pair_search`, the page's `pairSearchNotes`). `best`: the best (trainee, pair) by score
    /// over both phases (ties: the earlier), with its margin over the pick's trainee's shown career. The pick's side: the
    /// pick's trainee's best pair where the page names it as her better parents (`better_parents`), else the plan's pick.
    /// Where `best` is another trainee's, its margin over the pick's side under the nominal model and the two per-uma
    /// scenarios (`ScenarioMargins`, `versus_pick_side`); it is recommended only where all three are positive, and is
    /// otherwise tied with the pick's side, which stays the recommendation (`recommendation`: its `career`, `trainee`,
    /// `trainee_id`, `parents`, `source`: "cross_trainee", "pick_trainee_pair" or "plan_pick", `score`: the pair
    /// search's, null for the plan's pick, and `report`: the career as a row reports one, its deck with it, for the page
    /// to offer it as it is). Within `aside`
    fn verdict(
        &self,
        simulator: &mut Sim,
        fits: &PairSearchFits,
        found: PairSearchFound,
        buf: &mut Vec<f64>,
    ) -> PairSearchVerdict {
        let data = simulator.data;
        let id = |simulator: &Sim, career: &Found| {
            career_id(&simulator.env.setup.space, data.umas[career.trainee].id, career)
        };
        let recommendation = |simulator: &mut Sim, recommended: Recommended| {
            let career = recommended.career;
            let env = &mut simulator.env;
            let report = env.setup.report(&mut env.state, fits.pool.pool, career, false);
            json!({"career": id(simulator, career), "trainee": data.umas[career.trainee].name,
                "trainee_id": data.umas[career.trainee].id, "parents": [career.first_parent, career.second_parent],
                "source": recommended.source, "score": recommended.score, "report": report})
        };
        let plan_pick = &self.draft.rows[self.draft.plan_place].career;
        let pick_report = found.reports.iter().find(|reported| reported.career.trainee == plan_pick.trainee);
        let pick_side = match pick_report {
            Some(reported) if reported.report["better_parents"] == true => {
                Recommended { career: &reported.career, source: "pick_trainee_pair", score: Some(reported.score()) }
            }
            _ => Recommended { career: plan_pick, source: "plan_pick", score: None },
        };
        let Some(best) = found
            .reports
            .iter()
            .chain(found.shared)
            .reduce(|kept, reported| if reported.score() > kept.score() { reported } else { kept })
        else {
            return PairSearchVerdict { best: Value::Null, recommendation: recommendation(simulator, pick_side) };
        };
        let mut best_json = json!({"trainee": best.report["trainee"], "career": best.report["career"],
            "best_parents": best.report["best_parents"], "score": best.report["score"], "reason": best.report["reason"],
            "pick_trainee_shown": pick_report.map(|shown| shown.report["shown"].clone()),
            "margin_over_pick_trainee_shown": pick_report
                .and_then(|shown| shown.report["shown_score"].as_f64())
                .map(|shown_score| best.score() - shown_score)});
        let mut recommended = pick_side;
        if best.career.trainee != plan_pick.trainee {
            let against = CareerAgainst { career: &best.career, against: pick_side.career };
            let margins = self.scenario_margins(simulator, fits, against, buf);
            best_json["versus_pick_side"] = json!({"career": id(simulator, pick_side.career),
                "source": pick_side.source, "margins": margins.json(), "holds": margins.hold()});
            if margins.hold() {
                recommended = Recommended { career: &best.career, source: "cross_trainee", score: Some(best.score()) };
            }
        }
        PairSearchVerdict { best: best_json, recommendation: recommendation(simulator, recommended) }
    }
    /// `against.career`'s score less `against.against`'s (`ScenarioMargins`): under the nominal model, then with the
    /// latter's uma's P(all) x DIFFERENTIAL_DOWN and the former's x DIFFERENTIAL_UP, then the reverse (each trio's C
    /// chosen again, `variant_fit`). Within `aside`
    fn scenario_margins(
        &self,
        simulator: &mut Sim,
        fits: &PairSearchFits,
        against: CareerAgainst,
        buf: &mut Vec<f64>,
    ) -> ScenarioMargins {
        let pool = fits.pool;
        let mut margin_under = |simulator: &mut Sim, base: &ScoreBase| {
            let scoring = ScoringContext { score_base: base, legacies: pool.legacies, pool: pool.pool };
            let mut score = |simulator: &mut Sim, career: &Found| {
                score_career(simulator, &scoring, found_career(career.trainee, career), Want::Score, buf)
                    .expected_saving
                    - 1.0
            };
            score(simulator, against.career) - score(simulator, against.against)
        };
        let mut scenario_margin = |simulator: &mut Sim, against_scale: f64, career_scale: f64| {
            let scenario = CheckVariant {
                kind: VariantKind::Differential,
                shape: FITTED_SHAPE,
                scales: OddsScales {
                    common: 1.0,
                    scaled: [(against.against.trainee, against_scale), (against.career.trainee, career_scale)],
                },
            };
            let fit = self.variant_fit(simulator, pool, scenario);
            margin_under(simulator, &fit.score_base(&self.candidates, self.fit.knobs))
        };
        let pick_down = scenario_margin(simulator, DIFFERENTIAL_DOWN, DIFFERENTIAL_UP);
        let pick_up = scenario_margin(simulator, DIFFERENTIAL_UP, DIFFERENTIAL_DOWN);
        let nominal = margin_under(simulator, &fits.nominal.score_base(&self.candidates, self.fit.knobs));
        ScenarioMargins { nominal, pick_down, pick_up }
    }
    /// One trainee's report (`pair_search` phase 1): her pairs (`trainee_pairs`), the best `PAIR_FINALISTS` as finalists
    /// (`finalist`), the best of them by score against her shown career (nominal, under each shape corner and at her own
    /// odds x 0.76 / x 1.32, C chosen again, `variant_fit`) and against the best other pair evaluated; None where she has
    /// no checked row
    fn trainee_report(
        &self,
        simulator: &mut Sim,
        fits: &PairSearchFits,
        searched: PairSearchTrainee,
        memo: &mut crate::career::plan::combine::PairMemo,
        buf: &mut Vec<f64>,
    ) -> Option<ReportedCareer> {
        let PairSearchTrainee { trainee, reason } = searched;
        let (trainee_started_ms, work_at_start) = (now_ms(), PairSearchWork::now(simulator));
        let pool = fits.pool;
        let base = fits.nominal.score_base(&self.candidates, self.fit.knobs);
        let scoring = ScoringContext { score_base: &base, legacies: pool.legacies, pool: pool.pool };
        let found = self.trainee_pairs(simulator, &scoring, pool, trainee, buf)?;
        let score = |simulator: &mut Sim, buf: &mut Vec<f64>, base: &ScoreBase, career: &Found| {
            let scoring = ScoringContext { score_base: base, legacies: pool.legacies, pool: pool.pool };
            score_career(simulator, &scoring, found_career(trainee, career), Want::Score, buf).expected_saving - 1.0
        };
        // the best PAIR_FINALISTS pairs by cached deck, each with every cached deck scored and searched exactly, ranked
        // again by score
        let mut finalists: Vec<Finalist> = found
            .ranked
            .iter()
            .take(PAIR_FINALISTS)
            .map(|ranked| self.finalist(simulator, &scoring, memo, ranked, buf))
            .collect();
        let winner =
            (0..finalists.len()).fold(0, |best, k| if finalists[k].score > finalists[best].score { k } else { best });
        let best = &finalists[winner];
        let mut margin_under = |simulator: &mut Sim, base: &ScoreBase| {
            score(simulator, buf, base, &best.found) - score(simulator, buf, base, &found.shown)
        };
        let corner_margins: Vec<f64> = fits
            .corners
            .iter()
            .map(|corner| margin_under(simulator, &corner.score_base(&self.candidates, self.fit.knobs)))
            .collect();
        // her own P(all) at each end of the per-uma spread, both careers scaled alike
        let scale_margins: Vec<f64> = [DIFFERENTIAL_DOWN, DIFFERENTIAL_UP]
            .iter()
            .map(|&scale| {
                let own = CheckVariant {
                    kind: VariantKind::Differential,
                    shape: FITTED_SHAPE,
                    scales: OddsScales { common: 1.0, scaled: [(trainee, scale), (crate::uma::NONE, 1.0)] },
                };
                let fit = self.variant_fit(simulator, pool, own);
                margin_under(simulator, &fit.score_base(&self.candidates, self.fit.knobs))
            })
            .collect();
        // the best other pair evaluated (one entry per pair): a finalist's kept career, or past the finalists its
        // P(all)-best cached deck's (`ranked` best first, so the first past them is their best)
        let other_finalists = finalists
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != winner)
            .map(|(_, finalist)| (finalist.score, &finalist.found));
        let first_past_finalists =
            found.ranked.get(PAIR_FINALISTS).map(|(cached_score, career)| (*cached_score, career));
        let best_other =
            other_finalists.chain(first_past_finalists).max_by(|first, second| first.0.total_cmp(&second.0));
        let id = |career: &Found| career_id(&simulator.env.setup.space, simulator.data.umas[trainee].id, career);
        let alternatives: Vec<Value> = finalists
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != winner)
            .map(|(_, finalist)| finalist.json(id(&finalist.found)))
            .collect();
        // (the page's better parents: a gain over her shown career on another pair; a gain on her shown pair itself is
        // a better deck, not better parents)
        let same_pair = |first: &Found, second: &Found| {
            (first.first_parent.min(first.second_parent), first.first_parent.max(first.second_parent))
                == (second.first_parent.min(second.second_parent), second.first_parent.max(second.second_parent))
        };
        let better_parents = best.score - found.shown_score > 0.0 && !same_pair(&best.found, &found.shown);
        let report = json!({
            "trainee": simulator.data.umas[trainee].name, "reason": reason,
            "shown": id(&found.shown), "shown_score": found.shown_score,
            "best_parents": [best.found.first_parent, best.found.second_parent],
            "career": id(&best.found), "score": best.score, "search": best.searched.status.name(),
            "proven": best.searched.proven && best.kept_searched, "deck_choice": best.deck_choice(),
            "margin_over_shown": best.score - found.shown_score, "better_parents": better_parents,
            "margin_over_other_pairs": best_other.map(|(score, _)| best.score - score),
            "best_other_pair": best_other.map(|(_, career)| id(career)),
            "corner_margins": corner_margins, "scale_margins": scale_margins,
            "alternatives": alternatives, "pairs_scored": found.pairs_scored,
            "pairs_runnable": found.pairs_runnable, "pairs_truncated": found.pairs_truncated,
            "trainee_ms": now_ms() - trainee_started_ms,
            "work": PairSearchWork::now(simulator).since(work_at_start).json(),
        });
        Some(ReportedCareer { report, career: finalists.swap_remove(winner).found })
    }
    /// Phase 2 of the pair search (`pair_search`): the best pairs phase 1 found (its reports' `best_parents`, each once)
    /// tried for every allowed trainee without a phase-1 report (`phase_two_trainees`: those the work limit skipped
    /// among them), where she can run them, with her best cached deck there; the `PAIR_FINALISTS` best by score each made
    /// a finalist (`finalist`), reported best first ("reason": "shared_pair"; no shown career, so no margins)
    fn shared_pair_report(
        &self,
        simulator: &mut Sim,
        fits: &PairSearchFits,
        reports: &[ReportedCareer],
        memo: &mut crate::career::plan::combine::PairMemo,
        buf: &mut Vec<f64>,
    ) -> SharedPairs {
        let (started_ms, work_at_start) = (now_ms(), PairSearchWork::now(simulator));
        let pool = fits.pool;
        let data = simulator.data;
        let mut shared_pairs: Vec<(usize, usize)> = vec![];
        for reported in reports {
            let career = &reported.career;
            let pair = (career.first_parent.min(career.second_parent), career.first_parent.max(career.second_parent));
            if !shared_pairs.contains(&pair) {
                shared_pairs.push(pair);
            }
        }
        let base = fits.nominal.score_base(&self.candidates, self.fit.knobs);
        let scoring = ScoringContext { score_base: &base, legacies: pool.legacies, pool: pool.pool };
        let (mut chances, mut scratch) = (vec![], vec![]);
        let mut tried: Vec<(f64, Found)> = vec![];
        let reported: Vec<crate::uma::UmaIndex> = reports.iter().map(|reported| reported.career.trainee).collect();
        let trainees_tried = phase_two_trainees(&self.candidates.allowed_trainees, &reported);
        let (mut pairs_runnable, mut trainees_truncated) = (0, 0);
        for &trainee in &trainees_tried {
            let shared = |first: usize, second: usize| shared_pairs.contains(&(first.min(second), first.max(second)));
            let bounded = trainee_bounded_pairs(simulator.env, pool, trainee, shared, PAIR_SEARCH_PAIRS);
            pairs_runnable += bounded.runnable;
            trainees_truncated += usize::from(bounded.truncated());
            for (first_parent, second_parent) in bounded.pairs {
                let lineage = crate::career::Lineage { trainee, first_parent, second_parent };
                let odds = deck_odds(simulator, pool.pool, lineage, &mut chances, &mut scratch);
                let Some(deck) = (0..odds.len()).max_by(|&first, &second| odds[first].total_cmp(&odds[second])) else {
                    continue;
                };
                let career = Found {
                    trainee,
                    first_parent,
                    second_parent,
                    value: odds[deck],
                    ceiling: odds[deck],
                    deck: simulator.evaluator.cache.decks[trainee][deck].deck.clone(),
                };
                let career_score = score_career(simulator, &scoring, found_career(trainee, &career), Want::Score, buf)
                    .expected_saving
                    - 1.0;
                tried.push((career_score, career));
            }
        }
        tried.sort_by(|first, second| second.0.total_cmp(&first.0));
        let pairs_tried = tried.len();
        let mut finalists: Vec<Finalist> = tried
            .iter()
            .take(PAIR_FINALISTS)
            .map(|ranked| self.finalist(simulator, &scoring, memo, ranked, buf))
            .collect();
        finalists.sort_by(|first, second| second.score.total_cmp(&first.score));
        let work = PairSearchWork::now(simulator).since(work_at_start).json();
        let elapsed_ms = now_ms() - started_ms;
        let finalists = finalists
            .into_iter()
            .map(|finalist| {
                let trainee = finalist.found.trainee;
                let id = career_id(&simulator.env.setup.space, data.umas[trainee].id, &finalist.found);
                let mut report = finalist.json(id);
                report["trainee"] = json!(data.umas[trainee].name);
                report["reason"] = json!("shared_pair");
                report["best_parents"] = json!([finalist.found.first_parent, finalist.found.second_parent]);
                report["pairs_tried"] = json!(pairs_tried);
                report["phase_ms"] = json!(elapsed_ms);
                report["work"] = work.clone();
                ReportedCareer { report, career: finalist.found }
            })
            .collect();
        SharedPairs {
            finalists,
            trainees_tried: trainees_tried.len(),
            careers_tried: pairs_tried,
            pairs_runnable,
            trainees_truncated,
        }
    }
    /// A finalist pair (`pair_search`, `Finalist`): `ranked` (her P(all)-best cached deck on the pair and its score) and
    /// every other cached deck of hers on the pair scored under `scoring` (the cache read frozen, `aside`), the pair
    /// searched exactly, and the better by score kept
    fn finalist(
        &self,
        simulator: &mut Sim,
        scoring: &ScoringContext,
        memo: &mut crate::career::plan::combine::PairMemo,
        ranked: &(f64, Found),
        buf: &mut Vec<f64>,
    ) -> Finalist {
        let (p_all_deck_score, p_all_career) = ranked;
        let trainee = p_all_career.trainee;
        let mut score = |simulator: &mut Sim, career: &Found| {
            score_career(simulator, scoring, found_career(trainee, career), Want::Score, buf).expected_saving - 1.0
        };
        let lineage = crate::career::Lineage {
            trainee,
            first_parent: p_all_career.first_parent,
            second_parent: p_all_career.second_parent,
        };
        let odds = deck_odds(simulator, scoring.pool, lineage, &mut vec![], &mut vec![]);
        let (mut best_cached, mut best_cached_score) = (p_all_career.clone(), *p_all_deck_score);
        for (deck, &deck_odds) in odds.iter().enumerate() {
            let cached = &simulator.evaluator.cache.decks[trainee][deck].deck;
            // (a deck with no chance of the full set is scored too: its partial veterans can lift sides)
            if !(deck_odds >= 0.0) || crate::career::plan::kept::same_deck(cached, &p_all_career.deck) {
                continue;
            }
            let career = Found { value: deck_odds, ceiling: deck_odds, deck: cached.clone(), ..p_all_career.clone() };
            let career_score = score(simulator, &career);
            if career_score > best_cached_score {
                (best_cached, best_cached_score) = (career, career_score);
            }
        }
        let score_deck_differs = !crate::career::plan::kept::same_deck(&best_cached.deck, &p_all_career.deck);
        let searched = exact_pair_career(simulator, memo, scoring.pool, p_all_career);
        let searched_score = score(simulator, &searched.found);
        let kept_searched = searched_score >= best_cached_score;
        let (found, score) =
            if kept_searched { (searched.found.clone(), searched_score) } else { (best_cached, best_cached_score) };
        Finalist {
            found,
            score,
            searched,
            searched_score,
            kept_searched,
            p_all_deck_score: *p_all_deck_score,
            best_cached_score,
            score_deck_differs,
        }
    }
    /// Her shown career: her best-scoring checked row's (the last of equal scores); None where she has none
    fn shown_career(&self, trainee: crate::uma::UmaIndex) -> Option<&Found> {
        let rows = self.draft.rows.iter().filter(|row| row.trainee == trainee);
        rows.max_by(|first, second| first.score.total_cmp(&second.score)).map(|row| &row.career)
    }
    /// Her phase-1 work estimate (`admitted_trainees`): the pairs she is scored on (`trainee_bounded_pairs`) times the
    /// outcomes (`sc_out`) one scoring of her shown career evaluates under the nominal model; 0 where she has no checked
    /// row (she gets no report). Within `aside`
    fn estimated_outcomes(
        &self,
        simulator: &mut Sim,
        fits: &PairSearchFits,
        trainee: crate::uma::UmaIndex,
        buf: &mut Vec<f64>,
    ) -> f64 {
        let Some(shown) = self.shown_career(trainee) else { return 0.0 };
        let pool = fits.pool;
        let pairs = trainee_bounded_pairs(simulator.env, pool, trainee, |_, _| true, PAIR_SEARCH_PAIRS).pairs.len();
        let base = fits.nominal.score_base(&self.candidates, self.fit.knobs);
        let scoring = ScoringContext { score_base: &base, legacies: pool.legacies, pool: pool.pool };
        let before = PairSearchWork::now(simulator).outcomes;
        score_career(simulator, &scoring, found_career(trainee, shown), Want::Score, buf);
        pairs as f64 * (PairSearchWork::now(simulator).outcomes - before)
    }
    /// One trainee's pairs (`pair_search`): her shown career and her runnable pairs within the bound
    /// (`trainee_bounded_pairs`), each with her best cached deck, scored under `scoring`; None where she has no checked
    /// row
    fn trainee_pairs(
        &self,
        simulator: &mut Sim,
        scoring: &ScoringContext,
        pool: ScoringPool,
        trainee: crate::uma::UmaIndex,
        buf: &mut Vec<f64>,
    ) -> Option<TraineePairs> {
        let shown = self.shown_career(trainee)?.clone();
        let mut score = |simulator: &mut Sim, career: &Found| {
            score_career(simulator, scoring, found_career(trainee, career), Want::Score, buf).expected_saving - 1.0
        };
        let shown_score = score(simulator, &shown);
        let bounded = trainee_bounded_pairs(simulator.env, pool, trainee, |_, _| true, PAIR_SEARCH_PAIRS);
        let (pairs_runnable, pairs_truncated) = (bounded.runnable, bounded.truncated());
        let (mut chances, mut scratch) = (vec![], vec![]);
        let mut ranked: Vec<(f64, Found)> = vec![];
        for (first_parent, second_parent) in bounded.pairs {
            let lineage = crate::career::Lineage { trainee, first_parent, second_parent };
            let odds = deck_odds(simulator, pool.pool, lineage, &mut chances, &mut scratch);
            let Some(deck) = (0..odds.len()).max_by(|&first, &second| odds[first].total_cmp(&odds[second])) else {
                continue;
            };
            let career = Found {
                trainee,
                first_parent,
                second_parent,
                value: odds[deck],
                ceiling: odds[deck],
                deck: simulator.evaluator.cache.decks[trainee][deck].deck.clone(),
            };
            ranked.push((score(simulator, &career), career));
        }
        if ranked.is_empty() {
            return None;
        }
        let pairs_scored = ranked.len();
        ranked.sort_by(|first, second| second.0.total_cmp(&first.0));
        Some(TraineePairs { shown, shown_score, ranked, pairs_scored, pairs_runnable, pairs_truncated })
    }
}

impl CheckKeep {
    /// The trainees the pair search runs for (`CheckWork::pair_search`), each once: the plan's pick's, those of the rows
    /// the differential scenarios picked (the trainees they tie with the pick), then the `PAIR_SEARCH_TOP` best trainees
    /// by their best checked row's score under the plan's model (ties by uma index)
    fn pair_search_trainees(&self, draft: &CheckDraft) -> Vec<PairSearchTrainee> {
        let mut by_score: Vec<usize> = (0..draft.rows.len()).collect();
        // (equal scores by uma index: the order the work limit admits them in, `admitted_trainees`)
        by_score.sort_by(|&first, &second| {
            let (first, second) = (&draft.rows[first], &draft.rows[second]);
            second.score.total_cmp(&first.score).then(first.trainee.cmp(&second.trainee))
        });
        let mut top: Vec<crate::uma::UmaIndex> = vec![];
        for place in by_score {
            let trainee = draft.rows[place].trainee;
            if top.len() < PAIR_SEARCH_TOP && !top.contains(&trainee) {
                top.push(trainee);
            }
        }
        let pick = PairSearchTrainee { trainee: draft.rows[self.plan_place].trainee, reason: "pick" };
        let tied = self
            .differential_picks
            .iter()
            .map(|&place| PairSearchTrainee { trainee: draft.rows[place].trainee, reason: "tied" });
        let top = top.into_iter().map(|trainee| PairSearchTrainee { trainee, reason: "top" });
        let mut trainees: Vec<PairSearchTrainee> = vec![];
        for candidate in std::iter::once(pick).chain(tied).chain(top) {
            if !trainees.iter().any(|kept| kept.trainee == candidate.trainee) {
                trainees.push(candidate);
            }
        }
        trainees
    }
    /// The contenders, best odds first (a stable sort: equal odds in contender order).
    fn tied(&self) -> Vec<usize> {
        let mut tied = self.contenders.clone();
        tied.sort_by(|&first, &second| self.row_odds[second].total_cmp(&self.row_odds[first]));
        tied
    }
    /// The variants left (none once every one is done).
    const fn pending(&self) -> usize {
        self.variants.len() + 1 - self.entries.len()
    }
    /// Record a variant done.
    fn record(&mut self, variant: CheckVariant, current_value: f64, pick: &VariantPick) {
        if !self.contenders.contains(&pick.pick) {
            self.contenders.push(pick.pick);
        }
        if matches!(variant.kind, VariantKind::Differential) && !self.differential_picks.contains(&pick.pick) {
            self.differential_picks.push(pick.pick);
        }
        self.entries.push(variant_entry(variant.kind.name(), variant.shape, variant.scales, current_value, pick));
    }
    /// The check's report so far (`next.robust`): `pending` the variants left; `stable` and `contenders` (each variant's
    /// strict argmax, in the order first picked) over those done; where they disagree, `pick_varies` and `tied`, the
    /// contenders best odds first (equal odds in contender order), the row the check names over the plan's. Where they
    /// agree on another row than the plan's pick, `near_override` and `score_winner`: every variant scores that row
    /// highest and the plan's rule took its pick for better odds within NEAR of it (`PickRows::pick`); the variants do
    /// not disagree there
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
            // (stable: every variant's strict argmax is the plan's pick, NEAR's)
            "stable": self.contenders == [self.plan_place],
            "contenders": self.contenders.iter().map(|&place| id_of(place)).collect::<Vec<_>>(),
            "pick_varies": self.contenders.len() > 1,
            "near_override": self.contenders.len() == 1 && self.contenders[0] != self.plan_place,
            "score_winner": if self.contenders.len() == 1 { json!(id_of(self.contenders[0])) } else { Value::Null },
            "tied": if self.contenders.len() > 1 {
                json!(self.tied().into_iter().map(id_of).collect::<Vec<_>>())
            } else {
                Value::Null
            },
            "check_ms": self.check_ms,
            "pairs": self.pairs,
        })
    }
}

/// The pool a check's scorings read: the plan job's own, the pool its pick comes from (`PlanInputs::pick_pool`) and its
/// fixed parents (what the within-trainee pair search may pair her with).
#[derive(Clone, Copy)]
pub(crate) struct ScoringPool<'pool> {
    pub(crate) legacies: &'pool [Legacy],
    pub(crate) pool: &'pool Pool,
    pub(crate) pick_pool: &'pool Pool,
    pub(crate) fixed_parents: &'pool [usize],
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
/// runs and keeps one, `index_rule`); `Next` without one is answered with an error, no plan run. The pair search
/// (`next.pair_search`) needs the check's working state, which goes after the last variant where no job had asked for
/// it: asked then, `All` is treated as having no state (the plan runs again, its check with it), and `Next` reports
/// `pairs_error`, saying so
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
    let kept_check = &worker.checks[index];
    let pairs_lost = plan_job.pair_search && kept_check.pairs.is_none() && kept_check.work.is_none();
    if pairs_lost && request == CheckRequest::All {
        return None;
    }
    let mut check = worker.checks.remove(index);
    let started_ms = now_ms();
    if let Some(mut work) = check.work.take() {
        match sim_for(env, worker, input, plan_job) {
            Ok(mut simulator) => {
                std::mem::swap(&mut simulator.evaluator.cache, &mut work.cache);
                let mut ran_variant = false;
                while check.pending() > 0 {
                    let variant = check.variants[check.entries.len() - 1];
                    let (current_value, pick) = work.run_variant(&mut simulator, pool, variant);
                    check.record(variant, current_value, &pick);
                    ran_variant = true;
                    if request == CheckRequest::Next {
                        break;
                    }
                }
                // (the pair search after the last variant: in the same job under "all", as its own step under "next")
                if plan_job.pair_search
                    && check.pending() == 0
                    && check.pairs.is_none()
                    && (request == CheckRequest::All || !ran_variant)
                {
                    let trainees = check.pair_search_trainees(&work.draft);
                    check.pairs = Some(work.pair_search(&mut simulator, pool, &trainees));
                }
                std::mem::swap(&mut simulator.evaluator.cache, &mut work.cache);
            }
            Err(error) => return Some(error),
        }
        // (put back while a variant, or the pair search asked for, is left; after the last it goes, the report alone kept)
        if check.pending() > 0 || (plan_job.pair_search && check.pairs.is_none()) {
            check.work = Some(work);
        }
    }
    check.check_ms += now_ms() - started_ms;
    let mut report = check.report();
    if pairs_lost {
        report["pairs_error"] = json!(
            "no pair search: this check's variants finished before a job asked for it, and its working state went with \
             the last; ask next.robust \"all\" with next.pair_search to run the plan, its check and the pair search again"
        );
    }
    worker.checks.push(check);
    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A variant's pick over its scores: the strict argmax (a row within NEAR with better odds is not taken); equal
    /// scores keep the earlier row
    #[test]
    fn a_variant_picks_its_strict_argmax() {
        let odds = [0.01, 0.02, 0.05];
        let pick = |scores: [f64; 3]| {
            PickRows { count: 3, score: |k: usize| scores[k], odds: |k: usize| odds[k], can_be_pick: |_: usize| true }
                .argmax()
        };
        assert_eq!(pick([3.0, 2.9, 1.0]), 0);
        assert_eq!(pick([2.9, 3.0, 1.0]), 1);
        assert_eq!(pick([1.0, 1.0, 1.0]), 0);
    }

    /// Variants that disagree: the check reports the pick as varying and names the contenders best odds first.
    #[test]
    fn disagreeing_variants_name_the_best_odds_first() {
        let check = |contenders: Vec<usize>| CheckKeep {
            key: 0,
            row_ids: vec!["a".into(), "b".into(), "c".into()],
            row_odds: vec![0.01, 0.03, 0.02],
            plan_place: 0,
            entries: vec![],
            contenders,
            variants: vec![],
            differential_picks: vec![],
            pairs: None,
            check_ms: 0.0,
            work: None,
        };
        let varying = check(vec![0, 2, 1]).report();
        assert_eq!(varying["pick_varies"], json!(true));
        assert_eq!(varying["tied"], json!(["b", "c", "a"]));
        assert_eq!(varying["near_override"], json!(false));
        assert!(varying["score_winner"].is_null());
        let stable = check(vec![0]).report();
        assert_eq!(stable["pick_varies"], json!(false));
        assert!(stable["tied"].is_null());
        assert_eq!((stable["stable"].clone(), stable["near_override"].clone()), (json!(true), json!(false)));
        // (p11, 2026-10-26: every variant scores one career highest, NEAR took another for its odds: no variance)
        let overridden = check(vec![2]).report();
        assert_eq!(overridden["pick_varies"], json!(false));
        assert_eq!(overridden["stable"], json!(false));
        assert_eq!(overridden["near_override"], json!(true));
        assert_eq!(overridden["score_winner"], json!("c"));
    }

    /// A differential scenario scales the argmax's uma down and each of the best other umas up, one per scenario, the
    /// best first; `OddsScales::of` gives the common scale to every other uma
    #[test]
    fn differential_scenarios_scale_the_argmax_against_each_competitor() {
        let row = |uma: usize, score: f64| CheckRow {
            trainee: crate::uma::UmaIndex(uma),
            career: Found {
                trainee: crate::uma::UmaIndex(uma),
                first_parent: 0,
                second_parent: 1,
                value: 0.01,
                ceiling: 0.01,
                deck: crate::deck::space::SelectedDeck {
                    entries: vec![],
                    plain_pals: 0,
                    filler_cards: 0,
                    white_hint_chances: vec![],
                    gold_hint_chances: vec![],
                },
            },
            score,
        };
        let draft = CheckDraft {
            rows: vec![row(5, 2.0), row(7, 1.5), row(5, 1.9), row(9, 1.8), row(7, 1.7), row(3, 0.1), row(4, 0.0)],
            plan_place: 0,
            current_value: 100.0,
        };
        let scenarios = differential_variants(&draft, 0);
        let raised: Vec<usize> = scenarios.iter().map(|variant| variant.scales.scaled[1].0.0).collect();
        assert_eq!(raised, [9, 7, 3], "the best other umas, one each, best first");
        let scales = scenarios[0].scales;
        assert_eq!(scales.of(crate::uma::UmaIndex(5)), DIFFERENTIAL_DOWN);
        assert_eq!(scales.of(crate::uma::UmaIndex(9)), DIFFERENTIAL_UP);
        assert_eq!(scales.of(crate::uma::UmaIndex(7)), 1.0);
        assert_eq!(OddsScales::uniform(0.9).of(crate::uma::NONE), 0.9);
    }

    /// The work limit admits by estimates alone: the pick's and the tied always (their estimates counted), a "top" one
    /// where the estimates admitted before her plus hers fit, one too large skipped without stopping a cheaper one after
    /// her, whatever work any search did
    #[test]
    fn the_work_limit_admits_top_trainees_by_their_estimates_alone() {
        let trainee =
            |uma: usize, reason: &'static str| PairSearchTrainee { trainee: crate::uma::UmaIndex(uma), reason };
        let trainees =
            [trainee(1, "pick"), trainee(2, "tied"), trainee(3, "top"), trainee(4, "top"), trainee(5, "top")];
        let budget = PAIR_SEARCH_OUTCOMES;
        // the pick and the tied over the budget alone still run; no top one fits after them, not even at no estimate
        assert_eq!(
            admitted_trainees(&trainees, &[0.6 * budget, 0.6 * budget, 0.0, 1.0, 1.0]),
            [true, true, false, false, false]
        );
        // a top one too large is skipped, a cheaper one after her admitted
        assert_eq!(
            admitted_trainees(&trainees, &[0.2 * budget, 0.2 * budget, 0.7 * budget, 0.5 * budget, 0.1 * budget]),
            [true, true, false, true, true]
        );
    }

    /// Phase 2 tries every allowed trainee without a phase-1 report, once: one the work limit skipped (or with no
    /// checked row) is tried, as an uma nominated for phase 1 is not excluded for being nominated
    #[test]
    fn phase_two_tries_every_trainee_phase_one_did_not_report_on() {
        let uma = crate::uma::UmaIndex;
        let allowed = [uma(1), uma(2), uma(3), uma(2), uma(4), uma(5)];
        // umas 1 and 3 reported; 4 (say) nominated but skipped for work
        assert_eq!(phase_two_trainees(&allowed, &[uma(1), uma(3)]), [uma(2), uma(4), uma(5)]);
    }
}
