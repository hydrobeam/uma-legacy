//! Continuation fits and candidate combinations.
use crate::career::plan::cont::{
    Cont, ContShape, FITTED_SHAPE, ScoreBase, ScoringKnobs, SideStep, Trio, TrioSideVeteran, TrioSlot, anchored_cost,
    trio_value,
};
use crate::career::plan::rows::{PlanRow, RowOrigin};
use crate::career::plan::search::{FullLoop, alternative_careers, legal_pair, loop_ceiling, val};
use crate::career::plan::{CandidateIndex, PlanCandidates, PlanContext, PlanSearches, found_lineage};
use crate::career::sim::Sim;
use crate::career::{Legacy, Lineage};
use serde_json::Value;
/// Fresh lineages a trio side may take per candidate unless the job says (`PlanJob::fresh_lineages`): the veteran her best
/// career now would leave and the ones her next best careers would (other pairs ranked by her cached decks,
/// `alternative_careers`, each searched, `exact_pair_career`), each at its own step; the trios take the cheapest choice
/// of step + C's step, so a veteran that moves her best career onto a lineage worse for C leaves the old one there while
/// it stays among her top ones (not guaranteed: a veteran can open as many better pairs)
pub(crate) const FRESH_LINEAGES: usize = 3;

/// Trios the plan reports beside V's (`trios` in the output), cheapest first.
pub(crate) const CHEAPEST_TRIOS: usize = 3;

/// A trio within this share of V's value counts as tied with it (`near` in the output's `trios`). Assumed: the joint-lift
/// oracle reorders p16's trios by 1.6% (V_trios_second F4), an error that differs between trios; the structural errors
/// common to every trio (Jensen, sides that reuse each other) do not set it.
pub(crate) const NEAR_TRIO_SHARE: f64 = 0.02;

/// A trio and its finisher's uma (`Data::umas`), as the plan reports its best trio.
#[derive(Clone, Copy)]
pub(crate) struct PlanCombination {
    pub(crate) trio: Trio,
    pub(crate) finisher_trainee: crate::uma::UmaIndex,
}

/// Continuation and trio values fitted together for scoring the plan's rows.
/// Per-candidate buffers use the fixed order in `PlanCandidates::trainees`.
pub(crate) struct PlanFit {
    pub(crate) continuations: Vec<Cont>,
    /// What each candidate's continuation was fitted from (`continuation_evidence`; the robustness check refits it).
    pub(crate) evidence: Vec<Option<ContEvidence>>,
    /// Each candidate's step: the expected careers to a fresh full veteran of hers under her continuation.
    pub(crate) step_costs: Vec<f64>,
    pub(crate) trios: Vec<Trio>,
    /// Per candidate, the trios' sides (`ScoreBase::trio_sides`).
    pub(crate) sides: Vec<Vec<TrioSideVeteran>>,
    /// V: the cheapest trio's careers.
    pub(crate) current_value: f64,
    /// The job's scoring knobs.
    pub(crate) knobs: ScoringKnobs,
}

impl PlanFit {
    pub(crate) fn score_base<'fit>(&'fit self, candidates: &'fit PlanCandidates) -> ScoreBase<'fit> {
        ScoreBase {
            candidates: &candidates.trainees,
            step_costs: &self.step_costs,
            current_value: self.current_value,
            current_odds: &candidates.current_odds,
            certified: &candidates.certified,
            continuations: &self.continuations,
            step_caps: &candidates.step_caps,
            trios: &self.trios,
            trio_sides: &self.sides,
            finisher_umas: &candidates.allowed_trainees,
            knobs: self.knobs,
            odds_scales: crate::career::plan::cont::OddsScales::ONE,
        }
    }
    /// A trio's careers at the fit's steps.
    pub(crate) fn trio_value(&self, trio: &Trio) -> f64 {
        trio_value(&self.step_costs, &self.sides, trio)
    }
}

/// Pass 2's completed fit and reporting results, used by the proxy check and output assembly.
pub(crate) struct PlanScores {
    pub(crate) fit: PlanFit,
    pub(crate) best_combo: Option<PlanCombination>,
    /// The CHEAPEST_TRIOS cheapest trios, cheapest first (`best_combo` the first): the near-tied routes the page names.
    pub(crate) cheapest_combos: Vec<PlanCombination>,
    pub(crate) any_scored: bool,
    pub(crate) combo_ms: f64,
    pub(crate) index_ms: f64,
    pub(crate) work: Value,
}

/// A builder row's lift of one candidate.
#[derive(Clone, Copy)]
struct BuilderLift {
    /// `LiftStats::efficiency` at the candidate's odds now.
    efficiency: f64,
    /// The builder's uma (`Data::umas`).
    builder: crate::uma::UmaIndex,
    /// The builder's row (its place in the plan's rows).
    row: usize,
    /// P(her veteran lifts the candidate): lam.
    lift_chance: f64,
    /// The mean odds lifted to.
    lifted_odds: f64,
}

/// What a candidate's continuation is fitted from (`fit_continuation`): her odds now, the lifts of her exactly searched
/// builder rows (other umas' careers whose veteran lifts her: their best and, with `builder_careers` > 1, their next
/// ones), most efficient first, and her full-loop ceiling with her
/// two most efficient builders of different umas (None: one builder's uma, her mean lifted odds the ceiling)
pub(crate) struct ContEvidence {
    odds: f64,
    lifts: Vec<BuilderLift>,
    loop_ceiling: Option<f64>,
    /// The two builders (umas) behind the full-loop ceiling, where there is one.
    loop_builders: Option<[crate::uma::UmaIndex; 2]>,
}

impl ContEvidence {
    /// The two builders behind her full-loop ceiling (pinf's pair), most efficient first.
    pub(crate) const fn loop_builders(&self) -> Option<[crate::uma::UmaIndex; 2]> {
        self.loop_builders
    }
}

/// The model a continuation is fitted under: her shape (`ContShape`) and every career's P(all) as a multiple of the
/// one the plan computes (1 but in the robustness check's all-target ratio variants, `robust`)
#[derive(Clone, Copy, Debug)]
pub(crate) struct ContModel {
    pub(crate) shape: ContShape,
    pub(crate) odds_scale: f64,
}

/// The model the plan prices with.
pub(crate) const FITTED_MODEL: ContModel = ContModel { shape: FITTED_SHAPE, odds_scale: 1.0 };

/// Per candidate, what her continuation is fitted from (`ContEvidence`): None where she has no odds now or no builder
/// lifts her
pub(crate) fn continuation_evidence(
    simulator: &mut Sim,
    context: &PlanContext,
    runs: &[PlanRow],
    wins_by_trainee: &crate::uma::UmaTable<Vec<f64>>,
    buf: &mut Vec<f64>,
) -> Vec<Option<ContEvidence>> {
    let PlanContext { input, candidates } = *context;
    let data = context.data();
    let wins = |uma: crate::uma::UmaIndex| wins_by_trainee[uma].clone();
    (0..candidates.trainees.len())
        .map(|candidate| {
            if candidates.current_odds[candidate] <= 0.0 || runs.is_empty() {
                return None;
            }
            let trainee = candidates.trainees[candidate];
            let p = candidates.current_odds[candidate];
            // (a builder's best career and her next ones, `builder_careers`: each a career she can run)
            let mut lifts: Vec<BuilderLift> = runs
                .iter()
                .enumerate()
                .filter(|(_, row)| {
                    matches!(row.origin, RowOrigin::Base | RowOrigin::Alternative)
                        && row.exact
                        && data.umas[row.trainee].id != data.umas[trainee].id
                        && row.lift_stats[candidate].lift_chance > 0.0
                })
                .map(|(place, row)| {
                    let lift = row.lift_stats[candidate];
                    BuilderLift {
                        efficiency: lift.efficiency(p),
                        builder: row.trainee,
                        row: place,
                        lift_chance: lift.lift_chance,
                        lifted_odds: lift.lifted_odds_sum / lift.lift_chance,
                    }
                })
                .collect();
            lifts.sort_by(|first, second| second.efficiency.partial_cmp(&first.efficiency).unwrap());
            let most_efficient = *lifts.first()?;
            let second_builder = lifts
                .iter()
                .skip(1)
                .map(|lift| lift.builder)
                .find(|&builder| data.umas[builder].id != data.umas[most_efficient.builder].id);
            let loop_ceiling = second_builder.map(|second_builder| {
                let full_loop = FullLoop { trainee, first_builder: most_efficient.builder, second_builder };
                loop_ceiling(simulator, input.pool, full_loop, &wins, buf)
            });
            let loop_builders = second_builder.map(|second_builder| [most_efficient.builder, second_builder]);
            Some(ContEvidence { odds: p, lifts, loop_ceiling, loop_builders })
        })
        .collect()
}

/// A candidate's continuation under `model`, anchored at her odds now, and the builder row its lam and phi are from: pinf
/// from her full-loop ceiling (`ContShape::ceil_share` of it, or her builder's mean lifted odds where higher), and lam and
/// the first step (phi, the share of the log-odds gap it closes) from the exactly searched builder row (a base row or an
/// alternative one, `builder_careers`) whose
/// continuation costs her the least under that pinf (ties: the more efficient), so that a builder added beside the
/// others, the pinf pair unchanged, can only lower her step. Every odds it reads is scaled by `model.odds_scale` (the
/// lift chances are not odds)
pub(crate) fn fit_continuation(evidence: &ContEvidence, model: ContModel) -> (Cont, usize) {
    let ContModel { shape: ContShape { ceil_share, lam_decay }, odds_scale } = model;
    let p = evidence.odds * odds_scale;
    let loop_share = evidence.loop_ceiling.map(|ceiling| ceil_share * (odds_scale * ceiling));
    let shape = |lift: &BuilderLift| {
        let lifted_odds = lift.lifted_odds * odds_scale;
        let pinf = loop_share.map_or(lifted_odds, |share| share.max(lifted_odds));
        let phi =
            if pinf > p && lifted_odds > p { ((lifted_odds / p).ln() / (pinf / p).ln()).clamp(0.0, 1.0) } else { 0.0 };
        // (a lift chance is a probability: a sum of outcome chances can pass 1 by rounding)
        (lift.lift_chance.min(1.0), phi, pinf)
    };
    let most_efficient = &evidence.lifts[0];
    let (mut best, mut builder) = (shape(most_efficient), most_efficient.row);
    let mut best_cost = anchored_cost(best.0, best.1, best.2, p, lam_decay);
    for lift in &evidence.lifts[1..] {
        let (lam, phi, pinf) = shape(lift);
        let cost = anchored_cost(lam, phi, pinf, p, lam_decay);
        if cost < best_cost {
            (best, best_cost, builder) = ((lam, phi, pinf), cost, lift.row);
        }
    }
    (Cont::with_decay(best.0, best.1, best.2, p, lam_decay), builder)
}

/// Each candidate's continuation under `model` (`fit_continuation`; a candidate without evidence grinds: lam 0)
pub(crate) fn fitted_continuations(evidence: &[Option<ContEvidence>], model: ContModel) -> Vec<Cont> {
    evidence
        .iter()
        .map(|evidence| evidence.as_ref().map_or_else(Cont::default, |evidence| fit_continuation(evidence, model).0))
        .collect()
}

/// Each candidate's continuation as the plan prices it (`fitted_continuations` under `FITTED_MODEL`); a given fit
/// (`PlanJob::fit`) replaces the pool's, anchored at its own odds (`FitEntry`)
pub(crate) fn plan_continuations(context: &PlanContext, evidence: &[Option<ContEvidence>]) -> Vec<Cont> {
    let PlanContext { input, candidates } = *context;
    let data = context.data();
    let mut continuations = fitted_continuations(evidence, FITTED_MODEL);
    if let Some(fit) = &input.settings.fit {
        for (candidate, continuation) in continuations.iter_mut().enumerate() {
            if let Some(entry) =
                fit.iter().find(|entry| data.umas[candidates.trainees[candidate]].id == entry.character_id())
            {
                let (lam, phi, pinf) = entry.shape();
                let anchor = entry.anchor_odds().unwrap_or(candidates.current_odds[candidate]);
                *continuation = Cont::new(lam.min(1.0), phi, pinf, anchor);
            }
        }
    }
    continuations
}

/// Combinations in candidate-pair order, then A's side, then B's side (`ScoreBase::trio_sides` order): per pair of
/// sides the best C (the allowed uma, not of A's or B's, at her best odds on the two as parents)
pub(crate) fn plan_combinations(
    simulator: &mut Sim,
    context: &PlanContext,
    sides: &[Vec<TrioSideVeteran>],
    buf: &mut Vec<f64>,
) -> Vec<PlanCombination> {
    let PlanContext { input, candidates } = *context;
    let pool = input.pool;
    let data = context.data();
    let candidate_count = candidates.trainees.len();
    let mut combos: Vec<PlanCombination> = vec![];
    for first_candidate in 0..candidate_count {
        for second_candidate in first_candidate + 1..candidate_count {
            let (first_trainee, second_trainee) =
                (candidates.trainees[first_candidate], candidates.trainees[second_candidate]);
            if data.umas[first_trainee].id == data.umas[second_trainee].id {
                continue;
            }
            // (`CareerSetup::pool` of the legacies and the fresh full veterans: the working copy of `pool` with them
            // pushed, A's first, popped below; each side's place in it)
            let mut working_pool = simulator.evaluator.take_pool(pool);
            let mut places = |candidate: usize| -> Vec<usize> {
                sides[candidate]
                    .iter()
                    .map(|side| match side.step {
                        SideStep::Existing(legacy) => legacy,
                        SideStep::Current | SideStep::Alternative(_) => {
                            simulator.env.setup.pool_push(&mut working_pool, &side.legacy);
                            working_pool.veterans.len() - 1
                        }
                    })
                    .collect()
            };
            let (first_places, second_places) = (places(first_candidate), places(second_candidate));
            for (first_side, &first_parent) in first_places.iter().enumerate() {
                for (second_side, &second_parent) in second_places.iter().enumerate() {
                    let mut best: Option<(f64, crate::uma::UmaIndex)> = None;
                    for &finisher_trainee in &candidates.allowed_trainees {
                        if data.umas[finisher_trainee].id == data.umas[first_trainee].id
                            || data.umas[finisher_trainee].id == data.umas[second_trainee].id
                        {
                            continue;
                        }
                        let lineage = Lineage { trainee: finisher_trainee, first_parent, second_parent };
                        if legal_pair(data, &working_pool, lineage) {
                            let odds = val(simulator, &working_pool, lineage, buf);
                            if odds > best.map_or(0.0, |(best_odds, _)| best_odds) {
                                best = Some((odds, finisher_trainee));
                            }
                        }
                    }
                    if let Some((odds, finisher_trainee)) = best {
                        let slot =
                            |candidate: usize, side: usize| TrioSlot { candidate: CandidateIndex(candidate), side };
                        combos.push(PlanCombination {
                            trio: Trio {
                                sides: [slot(first_candidate, first_side), slot(second_candidate, second_side)],
                                finisher: finisher_trainee,
                                finisher_cost: 1.0 / odds,
                            },
                            finisher_trainee,
                        });
                    }
                }
            }
            while working_pool.veterans.len() > pool.veterans.len() {
                working_pool.pop_veteran();
            }
            simulator.evaluator.put_pool(pool, working_pool);
        }
    }
    combos
}

/// The trios' sides as `rechoose_finishers` reads them: per candidate her sides (`ScoreBase::trio_sides`) and the
/// candidates' umas (candidate order).
#[derive(Clone, Copy)]
pub(crate) struct TrioSides<'sides> {
    pub(crate) trio_sides: &'sides [Vec<TrioSideVeteran>],
    pub(crate) candidates: &'sides [crate::uma::UmaIndex],
}

/// The trios with each C chosen again at odds as `scales` prices them (`OddsScales::of`): per trio the allowed uma
/// (`allowed`), not of A's or B's character, at her best priced odds (`val` x her scale) on the trio's two sides as
/// `plan_combinations` takes them (a real full veteran in place, a fresh side pushed), the first such on a tie; her
/// step 1 / (odds x scale). At every scale 1 the C `plan_combinations` chose. A trio with no legal C keeps its own
pub(crate) fn rechoose_finishers(
    simulator: &mut Sim,
    pool: &crate::career::pool::Pool,
    sides: TrioSides,
    finishers: crate::career::plan::score::FinisherChoice,
    trios: &[Trio],
    buf: &mut Vec<f64>,
) -> Vec<Trio> {
    let data = simulator.data;
    let TrioSides { trio_sides, candidates } = sides;
    let crate::career::plan::score::FinisherChoice { umas: allowed, scales } = finishers;
    trios
        .iter()
        .map(|trio| {
            let mut working_pool = simulator.evaluator.take_pool(pool);
            let mut place = |slot: TrioSlot| {
                let side = &trio_sides[slot.candidate.0][slot.side];
                match side.step {
                    SideStep::Existing(legacy) => legacy,
                    SideStep::Current | SideStep::Alternative(_) => {
                        simulator.env.setup.pool_push(&mut working_pool, &side.legacy);
                        working_pool.veterans.len() - 1
                    }
                }
            };
            let [first, second] = trio.sides;
            let (first_parent, second_parent) = (place(first), place(second));
            let excluded = [data.umas[candidates[first.candidate.0]].id, data.umas[candidates[second.candidate.0]].id];
            let mut best: Option<(f64, crate::uma::UmaIndex)> = None;
            for &finisher in allowed {
                if excluded.contains(&data.umas[finisher].id) {
                    continue;
                }
                let lineage = Lineage { trainee: finisher, first_parent, second_parent };
                if legal_pair(data, &working_pool, lineage) {
                    let priced = val(simulator, &working_pool, lineage, buf) * scales.of(finisher);
                    if priced > best.map_or(0.0, |(best_priced, _)| best_priced) {
                        best = Some((priced, finisher));
                    }
                }
            }
            while working_pool.veterans.len() > pool.veterans.len() {
                working_pool.pop_veteran();
            }
            simulator.evaluator.put_pool(pool, working_pool);
            best.map_or(*trio, |(priced, finisher)| Trio { finisher, finisher_cost: 1.0 / priced, ..*trio })
        })
        .collect()
}

/// The fresh full veteran a career of candidate `candidate` would leave (`CareerSetup::veteran`: her G1 wins and
/// possible pinks from her schedule in the outfit her deck runs, `Evaluator::career_wins`)
fn fresh_veteran(simulator: &mut Sim, context: &PlanContext, career: crate::career::Career) -> Legacy {
    let record =
        simulator.evaluator.career_wins(&simulator.env.setup, &mut simulator.env.state, context.input.pool, career);
    simulator.env.setup.veteran(context.input.legacies, career.lineage, &record)
}

/// The trios' sides per candidate: her real full veterans (free), the fresh full veteran her best career now would leave
/// (her step, where finite) and those of her next `fresh_lineages` - 1 careers (the pairs ranked by her cached decks,
/// `alternative_careers`, each then searched exactly, `exact_pair_career`; each at her step at its odds, admitted only
/// where she grinds at once there, `BuildPlan::building_careers` 0)
pub(crate) fn plan_sides(
    simulator: &mut Sim,
    context: &PlanContext,
    full_veterans: &[Vec<usize>],
    continuations: &[Cont],
    step_costs: &[f64],
    searches: &mut PlanSearches,
    buf: &mut Vec<f64>,
) -> Vec<Vec<TrioSideVeteran>> {
    let PlanContext { input, candidates } = *context;
    let legacies = input.legacies;
    (0..candidates.trainees.len())
        .map(|candidate| {
            let mut sides: Vec<TrioSideVeteran> = full_veterans[candidate]
                .iter()
                .map(|&i| TrioSideVeteran {
                    step: SideStep::Existing(i),
                    legacy: legacies[i].clone(),
                    odds: 0.0,
                    parents: None,
                    deck: None,
                    proven: true,
                })
                .collect();
            let trainee = candidates.trainees[candidate];
            let Some(career) = candidates.careers[candidate].as_ref() else { return sides };
            if !step_costs[candidate].is_finite() {
                return sides;
            }
            let lineage = found_lineage(trainee, career);
            sides.push(TrioSideVeteran {
                step: SideStep::Current,
                legacy: fresh_veteran(simulator, context, crate::career::Career { lineage, deck: &career.deck }),
                odds: candidates.current_odds[candidate],
                parents: Some((career.first_parent, career.second_parent)),
                deck: Some(career.deck.clone()),
                proven: candidates.certified[candidate],
            });
            let pair = career.unordered_parents();
            let ranked = alternative_careers(
                simulator,
                context.parents(),
                trainee,
                pair,
                context.settings().fresh_lineages - 1,
                buf,
            );
            for ranked_career in ranked {
                // (each chosen pair at its best found searching it, the value another plan's complete search of the same
                // pair finds: the cached decks only rank the pairs)
                let PairCareer { found: alternative, proven, .. } =
                    exact_pair_career(simulator, &mut searches.pair_careers, context.input.pool, &ranked_career);
                let (lineage, odds) = (found_lineage(trainee, &alternative), alternative.value);
                let plan = continuations[candidate].plan(odds);
                // (only where she grinds at once on it: no scored career's veteran lifts an Alternative side, so one that
                // builds would be priced on a build no route advances, and her building would replace the lineage it
                // wins by, C's pedigree)
                if plan.expected_careers.is_finite() && plan.building_careers == 0.0 {
                    sides.push(TrioSideVeteran {
                        step: SideStep::Alternative(plan.expected_careers),
                        legacy: fresh_veteran(
                            simulator,
                            context,
                            crate::career::Career { lineage, deck: &alternative.deck },
                        ),
                        odds,
                        parents: Some((lineage.first_parent, lineage.second_parent)),
                        deck: Some(alternative.deck.clone()),
                        proven,
                    });
                }
            }
            sides
        })
        .collect()
}

/// A pair's career for a trio side (`exact_pair_career`): the career, whether it is the pair's proven best, and how the
/// search ended.
#[derive(Clone)]
pub(crate) struct PairCareer {
    pub(crate) found: crate::career::search::Found,
    /// The search completed (its best is the pair's best at every deck, so another plan's complete search of the same
    /// pair finds the same value); false: the search's incumbent or, where it found none above them, the cached decks'
    /// career (a real career, its value no certificate).
    pub(crate) proven: bool,
    pub(crate) status: PairSearchStatus,
}

/// How a pair's exact search ended (`PairCareer::status`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PairSearchStatus {
    /// Complete: its best is the pair's best (P(all)), the career taken.
    Proven,
    /// Complete, and its best is below the cached decks' value for the pair: that value was the corner tables'
    /// interpolation, above the exact best; the search's career is taken (on gm16 one ulp: the same deck).
    ProvenBelowCached,
    /// Not complete (its work ran out, or a breached class): the better of its incumbent and the cached career.
    Incomplete,
    /// Complete with no career on the pair: the cached career kept, unproven.
    NoneFound,
}

impl PairSearchStatus {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Proven => "proven",
            Self::ProvenBelowCached => "proven_below_cached",
            Self::Incomplete => "incomplete",
            Self::NoneFound => "none_found",
        }
    }
}

/// Trainee `ranked.trainee`'s best career on `ranked`'s parent pair, searched with both parents fixed (`Env::solve`) under
/// the job's own budget, not `search_work` (one pair: its search completes on every val/plan pool; a budget that one job
/// sets the same for every plan, so whether it completes depends on the job and the proofs the worker kept, never on
/// `search_work`), memoised (`PairMemo`: per plan, `PlanSearches::pair_careers`, or per check). A complete search's
/// best is taken, proven, even below the cached decks' career's value (`ranked`; that value is the corner tables'
/// interpolation: `ProvenBelowCached`); an incomplete search's incumbent where it is at least that value, else the
/// cached career (`Incomplete`, unproven); a complete search that finds none keeps the cached career, unproven
/// (`NoneFound`). Proven certifies the pair's best deck for P(all), not its saving's rank among pairs
pub(crate) fn exact_pair_career(
    simulator: &mut Sim,
    memo: &mut PairMemo,
    pool: &crate::career::pool::Pool,
    ranked: &crate::career::search::Found,
) -> PairCareer {
    let (first_parent, second_parent) = ranked.unordered_parents();
    let key = (ranked.trainee, first_parent, second_parent);
    if let Some(career) = memo.get(&key) {
        return career.clone();
    }
    let pair = [first_parent, second_parent];
    let searched = simulator.env.solve(crate::career::search::SearchRequest {
        pool,
        fixed_parents: &pair,
        trainees: &[ranked.trainee],
        per_trainee: false,
        top: 1,
    });
    let career = match searched.found.into_iter().next() {
        // (a complete search's best is the pair's: taken even below the cached value, an interpolation)
        Some(found) if searched.complete => {
            let status =
                if found.value < ranked.value { PairSearchStatus::ProvenBelowCached } else { PairSearchStatus::Proven };
            PairCareer { found, proven: true, status }
        }
        Some(found) if found.value >= ranked.value => {
            PairCareer { found, proven: false, status: PairSearchStatus::Incomplete }
        }
        _ => PairCareer {
            found: ranked.clone(),
            proven: false,
            status: if searched.complete { PairSearchStatus::NoneFound } else { PairSearchStatus::Incomplete },
        },
    };
    memo.insert(key, career.clone());
    career
}

/// Pair searches by (trainee, parent pair ascending) (`exact_pair_career`).
pub(crate) type PairMemo = crate::FxMap<(crate::uma::UmaIndex, usize, usize), PairCareer>;
