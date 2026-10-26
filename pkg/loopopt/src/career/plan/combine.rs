//! Continuation fits and candidate combinations.
use crate::career::plan::cont::{CEIL_SHARE, Cont, ScoreBase, Trio, TrioSideVeteran};
use crate::career::plan::rows::{PlanRow, RowOrigin};
use crate::career::plan::search::{FullLoop, legal_pair, loop_ceiling, val};
use crate::career::plan::{CandidateIndex, PlanCandidates, PlanContext, found_lineage};
use crate::career::pool::Pool;
use crate::career::sim::Sim;
use crate::career::{Legacy, Lineage};
use serde_json::Value;
/// A and B index `PlanCandidates::trainees`; the finisher indexes `Data::umas` directly.
/// Each side uses either an existing full veteran (a free step) or a newly built one.
#[derive(Clone, Copy)]
pub(crate) struct PlanCombination {
    pub(crate) candidate_a_index: CandidateIndex,
    pub(crate) candidate_b_index: CandidateIndex,
    /// Expected careers for the finisher, after A and B are full veterans.
    pub(crate) finisher_cost: f64,
    pub(crate) finisher_trainee: crate::uma::UmaIndex,
    pub(crate) a_uses_existing: bool,
    pub(crate) b_uses_existing: bool,
}

impl PlanCombination {
    pub(crate) fn total_cost(&self, stage_costs: &[f64]) -> f64 {
        let a_cost = if self.a_uses_existing { 0.0 } else { stage_costs[self.candidate_a_index.0] };
        let b_cost = if self.b_uses_existing { 0.0 } else { stage_costs[self.candidate_b_index.0] };
        a_cost + b_cost + self.finisher_cost
    }

    pub(crate) const fn as_trio(&self) -> Trio {
        Trio {
            candidate_a: self.candidate_a_index,
            candidate_b: self.candidate_b_index,
            finisher_cost: self.finisher_cost,
            a_uses_existing: self.a_uses_existing,
            b_uses_existing: self.b_uses_existing,
        }
    }
}

/// Continuation and trio values fitted together for scoring the plan's rows.
/// Per-candidate buffers use the fixed order in `PlanCandidates::trainees`.
pub(crate) struct PlanFit {
    pub(crate) continuations: Vec<Cont>,
    /// Each candidate's step: the expected careers to a fresh full veteran of hers under her continuation.
    pub(crate) step_costs: Vec<f64>,
    pub(crate) trios: Vec<Trio>,
    /// Per candidate, the trios' sides (`ScoreBase::trio_sides`).
    pub(crate) sides: Vec<Vec<TrioSideVeteran>>,
    /// V: the cheapest trio's careers.
    pub(crate) current_value: f64,
}

impl PlanFit {
    pub(crate) fn score_base<'fit>(&'fit self, candidates: &'fit PlanCandidates) -> ScoreBase<'fit> {
        ScoreBase {
            candidates: &candidates.trainees,
            step_costs: &self.step_costs,
            later_stage_costs: &candidates.later_stage_costs,
            current_value: self.current_value,
            current_odds: &candidates.current_odds,
            continuations: &self.continuations,
            trios: &self.trios,
            trio_sides: &self.sides,
            finisher_umas: &candidates.allowed_trainees,
        }
    }
}

/// Pass 2's completed fit and reporting results, used by the proxy check and output assembly.
pub(crate) struct PlanScores {
    pub(crate) fit: PlanFit,
    pub(crate) best_combo: Option<PlanCombination>,
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
    /// P(her veteran lifts the candidate): lam.
    lift_chance: f64,
    /// The mean odds lifted to.
    lifted_odds: f64,
}

/// Each candidate's continuation: lam and the first step from her most efficient exactly searched base builder, pinf
/// from her full-loop ceiling with her two most efficient builders (of different umas), phi the share of the log-odds
/// gap the first step closes; a given fit (`PlanJob::fit`) replaces the pool's
pub(crate) fn plan_continuations(
    simulator: &mut Sim,
    context: &PlanContext,
    runs: &[PlanRow],
    wins_by_trainee: &crate::uma::UmaTable<Vec<usize>>,
    buf: &mut Vec<f64>,
) -> Vec<Cont> {
    let PlanContext { input, candidates } = *context;
    let data = context.data();
    let candidate_count = candidates.trainees.len();
    let wins = |uma: crate::uma::UmaIndex| wins_by_trainee[uma].clone();
    let mut continuations = vec![Cont::default(); candidate_count];
    for candidate in 0..candidate_count {
        if candidates.current_odds[candidate] <= 0.0 || runs.is_empty() {
            continue;
        }
        let trainee = candidates.trainees[candidate];
        let p = candidates.current_odds[candidate];
        let mut lifts: Vec<BuilderLift> = runs
            .iter()
            .filter(|row| {
                row.origin == RowOrigin::Base
                    && row.exact
                    && data.umas[row.trainee].id != data.umas[trainee].id
                    && row.lift_stats[candidate].lift_chance > 0.0
            })
            .map(|row| {
                let lift = row.lift_stats[candidate];
                BuilderLift {
                    efficiency: lift.efficiency(p),
                    builder: row.trainee,
                    lift_chance: lift.lift_chance,
                    lifted_odds: lift.lifted_odds_sum / lift.lift_chance,
                }
            })
            .collect();
        lifts.sort_by(|first, second| second.efficiency.partial_cmp(&first.efficiency).unwrap());
        let Some(&best) = lifts.first() else { continue };
        let lam = best.lift_chance;
        let p1 = best.lifted_odds;
        let second_builder = lifts
            .iter()
            .skip(1)
            .map(|lift| lift.builder)
            .find(|&builder| data.umas[builder].id != data.umas[best.builder].id);
        let pinf = match second_builder {
            Some(second_builder) => {
                let full_loop = FullLoop { trainee, first_builder: best.builder, second_builder };
                (CEIL_SHARE * loop_ceiling(simulator, input.pool, full_loop, &wins, buf)).max(p1)
            }
            None => p1,
        };
        let phi = if pinf > p && p1 > p { ((p1 / p).ln() / (pinf / p).ln()).clamp(0.0, 1.0) } else { 0.0 };
        continuations[candidate] = Cont::new(lam, phi, pinf);
    }
    if let Some(fit) = &input.settings.fit {
        for candidate in 0..candidate_count {
            if let Some(&(_, lam, phi, pinf)) =
                fit.iter().find(|entry| data.umas[candidates.trainees[candidate]].id == entry.0)
            {
                continuations[candidate] = Cont::new(lam, phi, pinf);
            }
        }
    }
    continuations
}

/// One side of a trio: a real full veteran of the candidate's, or the fresh one her best career now would leave.
#[derive(Clone, Copy)]
struct TrioSide {
    uses_existing: bool,
    /// The veteran's index in the working pool.
    legacy: usize,
}

/// Each side choice's best finisher: her odds and uma.
#[derive(Clone, Copy)]
struct Finisher {
    odds: f64,
    trainee: crate::uma::UmaIndex,
}

/// Combinations in candidate-pair order, then fresh/existing side order. `full_veterans`: per candidate her real full veterans
/// (legacy indices); `step_costs`: per candidate, a fresh full veteran's step (a fresh side only where it is finite).
pub(crate) fn plan_combinations(
    simulator: &mut Sim,
    context: &PlanContext,
    full_veterans: &[Vec<usize>],
    step_costs: &[f64],
    buf: &mut Vec<f64>,
) -> Vec<PlanCombination> {
    let PlanContext { input, candidates } = *context;
    let pool = input.pool;
    let data = context.data();
    let candidate_count = candidates.trainees.len();
    let fresh_veterans: Vec<Option<Legacy>> =
        (0..candidate_count).map(|candidate| fresh_veteran(simulator, context, step_costs, candidate)).collect();
    let mut combos: Vec<PlanCombination> = vec![];
    for first_candidate in 0..candidate_count {
        for second_candidate in first_candidate + 1..candidate_count {
            let (first_trainee, second_trainee) =
                (candidates.trainees[first_candidate], candidates.trainees[second_candidate]);
            if data.umas[first_trainee].id == data.umas[second_trainee].id {
                continue;
            }
            // (`CareerSetup::pool` of the legacies and the fresh full veterans: the working copy of `pool` with them
            // pushed, A's first, popped below)
            let mut working_pool = simulator.evaluator.take_pool(pool);
            let sides = |simulator: &mut Sim, working_pool: &mut Pool, candidate: usize| -> Vec<TrioSide> {
                let mut sides: Vec<TrioSide> =
                    full_veterans[candidate].iter().map(|&legacy| TrioSide { uses_existing: true, legacy }).collect();
                if let Some(fresh) = &fresh_veterans[candidate] {
                    simulator.env.setup.pool_push(working_pool, fresh);
                    sides.push(TrioSide { uses_existing: false, legacy: working_pool.veterans.len() - 1 });
                }
                sides
            };
            let (a_sides, b_sides) = (
                sides(simulator, &mut working_pool, first_candidate),
                sides(simulator, &mut working_pool, second_candidate),
            );
            // per side choice ([A existing][B existing]): the best C
            let mut finishers: [[Option<Finisher>; 2]; 2] = [[None; 2]; 2];
            for &finisher_trainee in &candidates.allowed_trainees {
                if data.umas[finisher_trainee].id == data.umas[first_trainee].id
                    || data.umas[finisher_trainee].id == data.umas[second_trainee].id
                {
                    continue;
                }
                for a_side in &a_sides {
                    for b_side in &b_sides {
                        let lineage = Lineage {
                            trainee: finisher_trainee,
                            first_parent: a_side.legacy,
                            second_parent: b_side.legacy,
                        };
                        if legal_pair(data, &working_pool, lineage) {
                            let best =
                                &mut finishers[usize::from(a_side.uses_existing)][usize::from(b_side.uses_existing)];
                            let odds = val(simulator, &working_pool, lineage, buf);
                            if odds > best.map_or(0.0, |finisher| finisher.odds) {
                                *best = Some(Finisher { odds, trainee: finisher_trainee });
                            }
                        }
                    }
                }
            }
            for (a_uses_existing, row) in [false, true].into_iter().zip(finishers) {
                for (b_uses_existing, finisher) in [false, true].into_iter().zip(row) {
                    if let Some(finisher) = finisher {
                        combos.push(PlanCombination {
                            candidate_a_index: CandidateIndex(first_candidate),
                            candidate_b_index: CandidateIndex(second_candidate),
                            finisher_cost: 1.0 / finisher.odds,
                            finisher_trainee: finisher.trainee,
                            a_uses_existing,
                            b_uses_existing,
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

/// The fresh full veteran the best career of candidate `candidate` would now leave (`CareerSetup::veteran`), where
/// her step is finite
fn fresh_veteran(simulator: &mut Sim, context: &PlanContext, step_costs: &[f64], candidate: usize) -> Option<Legacy> {
    if !step_costs[candidate].is_finite() {
        return None;
    }
    let candidates = context.candidates;
    let career = candidates.careers[candidate].as_ref()?;
    let lineage = found_lineage(candidates.trainees[candidate], career);
    let record = simulator.evaluator.career_wins(
        &simulator.env.setup,
        &mut simulator.env.state,
        context.input.pool,
        crate::career::Career { lineage, deck: &career.deck },
    );
    Some(simulator.env.setup.veteran(context.input.legacies, lineage, &record))
}

/// The trios' sides per candidate (her real full veterans, then the fresh full veteran her best career now would leave), for `her_trios`
pub(crate) fn plan_sides(
    simulator: &mut Sim,
    context: &PlanContext,
    full_veterans: &[Vec<usize>],
    step_costs: &[f64],
) -> Vec<Vec<TrioSideVeteran>> {
    let legacies = context.input.legacies;
    (0..context.candidates.trainees.len())
        .map(|candidate| {
            let mut sides: Vec<TrioSideVeteran> = full_veterans[candidate]
                .iter()
                .map(|&i| TrioSideVeteran { uses_existing: true, legacy: legacies[i].clone() })
                .collect();
            if let Some(fresh) = fresh_veteran(simulator, context, step_costs, candidate) {
                sides.push(TrioSideVeteran { uses_existing: false, legacy: fresh });
            }
            sides
        })
        .collect()
}
