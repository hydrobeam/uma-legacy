//! Career scoring over the possible veterans and their continuations.
//!
//! `score_career` and `score_career_of` (here, with what they return) are the entries; `her_trios`, `her_ends` and
//! `fm_masks` are what `anchors` and `roll` read of a career's own full veteran and landed sets.
//! - `build`: building a scoring: her setups (`prepare_setups`), `evaluate_career` and the tables (`prepare_scoring`).
//! - `kernel`: the scoring kernels that fold and evaluate a prepared scoring (`Scored`).
use crate::career::clock::{now_ms, prof};
use crate::career::plan::cont::{OwnTrio, ScoreBase, TrioSideVeteran, Want};
use crate::career::plan::score::build::evaluate_career;
use crate::career::plan::score::kernel::{LiftStats, Scored};
use crate::career::plan::search::{is_full, legal_pair, val};
use crate::career::plan::{CandidateIndex, PoolProvenance, ScoringContext, TargetMask};
use crate::career::pool::Pool;
use crate::career::sim::Sim;
use crate::career::{Career, Legacy, Lineage};
mod build;
pub(crate) mod kernel;

struct FinisherCeiling {
    upper_bound: f64,
    lineage: Lineage,
}

/// A career's own full veteran as a trio side when her uma is not a stage candidate (her outcome lands every target): the replan
/// would take her uma in (a veteran of hers) with that real full veteran a free side. Per candidate side (real or fresh) her full veteran can
/// pair with, C's step: 1 / the best allowed C's value with the two full veterans as parents (every allowed uma but hers and the
/// partner's, best ceiling first, the value taken until no ceiling can beat the best, as `plan_combinations` does).
/// Her full veteran is the new veteran in `extended_pool` (`CareerSetup::veteran`: every target at the rank's star
/// mix), valued as every fresh side is
/// (the trios' own full veterans use the rank's mix too), so the trios with her compare like for like with the others; one per
/// call, not per star pattern (the full set's stars vary per leaf: per leaf it would be 4^nu searches for C).
/// (B's candidate index, partner side real, C's step)
pub(crate) fn her_trios(
    simulator: &mut Sim,
    score_base: &ScoreBase,
    extended_pool: &mut Pool,
    new_veteran: usize,
    trainee: crate::uma::UmaIndex,
    score_buffer: &mut Vec<f64>,
) -> Vec<OwnTrio> {
    let data = simulator.data;
    let trainee_id = data.umas[trainee].id;
    let mut trios = vec![];
    // the pool with her (`Evaluator::push_her`: `CareerSetup::pool` of the legacies and hers), with each side pushed in
    // turn (the same Pool as `CareerSetup::pool` of those and the side) and popped after
    let partner_index = extended_pool.veterans.len();
    let (mut inheritance_chances, mut finishers): (Vec<f64>, Vec<FinisherCeiling>) =
        (Vec::with_capacity(simulator.env.setup.target_count), vec![]);
    for candidate in 0..score_base.candidates.len() {
        let partner_trainee = score_base.candidates[candidate];
        if data.umas[partner_trainee].id == trainee_id {
            continue;
        }
        for TrioSideVeteran { uses_existing, legacy } in
            score_base.trio_sides.get(candidate).map_or(&[][..], |value| &**value)
        {
            simulator.env.setup.pool_push(extended_pool, legacy);
            // (C ranges over the trainees on one pair: `lineage` and `ceiling` per trainee, not `gen_cands`, whose
            // per-trainee tables cover every legacy of the pool)
            finishers.clear();
            finishers.extend(
                score_base
                    .finisher_umas
                    .iter()
                    .copied()
                    .map(|trainee| Lineage { trainee, first_parent: new_veteran, second_parent: partner_index })
                    .filter(|lineage| {
                        data.umas[lineage.trainee].id != trainee_id
                            && data.umas[lineage.trainee].id != data.umas[partner_trainee].id
                            && legal_pair(data, &*extended_pool, *lineage)
                    })
                    .filter_map(|lineage| {
                        let scale =
                            simulator.env.setup.lineage_into(&*extended_pool, lineage, &mut inheritance_chances);
                        simulator
                            .env
                            .setup
                            .ceiling(&mut simulator.env.state, lineage.trainee, &inheritance_chances, scale)
                            .map(|upper_bound| FinisherCeiling { upper_bound, lineage })
                    }),
            );
            finishers.sort_by(|first, second| second.upper_bound.partial_cmp(&first.upper_bound).unwrap());
            let mut best = 0.0f64;
            for &FinisherCeiling { upper_bound, lineage } in &finishers {
                if upper_bound * (1.0 + 1e-9) <= best {
                    break;
                }
                let value = val(simulator, &*extended_pool, lineage, score_buffer);
                if value > best {
                    best = value;
                }
            }
            if best > 0.0 {
                trios.push(OwnTrio {
                    candidate: CandidateIndex(candidate),
                    uses_existing: *uses_existing,
                    finisher_cost: 1.0 / best,
                });
            }
            extended_pool.pop_veteran();
        }
    }
    trios
}

/// How a career's own full veteran ends her part of the plan (`her_ends`).
#[derive(Clone, Copy)]
pub(crate) struct HerEnds {
    /// Her place among the stage candidates; None: her uma is not one.
    pub(crate) own_candidate: Option<usize>,
    /// Her parents are both full veterans: her own full veteran a fully looped parent, V after her 0 (the goal).
    pub(crate) completes_goal: bool,
}

/// The career's place among the stage candidates and whether its parents are both full veterans: what `score_career`,
/// `outcome_hi` and `pick_her` read of her first
pub(crate) fn her_ends(simulator: &Sim, score_base: &ScoreBase, legacies: &[Legacy], lineage: Lineage) -> HerEnds {
    let Lineage { trainee, first_parent, second_parent } = lineage;
    HerEnds {
        own_candidate: score_base.candidates.iter().position(|&candidate| candidate == trainee),
        completes_goal: is_full(simulator.env.setup.skill_job, &legacies[first_parent].sparks)
            && is_full(simulator.env.setup.skill_job, &legacies[second_parent].sparks),
    }
}

/// Per mask fm of the scenario-guaranteed targets landed (bit j: `fixed_t[j]`), its landed set over the job targets: an
/// outcome's cell is (leaf << nf) | fm, its landed set the leaf's (`leaf_set`) | this (`score_career`, `outcome_hi`)
pub(crate) fn fm_masks(fixed_t: &[usize]) -> Vec<TargetMask> {
    (0..(1usize << fixed_t.len()))
        .map(|bits| {
            let fixed = TargetMask::from_bits(bits);
            let mut landed = TargetMask::default();
            for bit in fixed.iter() {
                landed.insert(fixed_t[bit]);
            }
            landed
        })
        .collect()
}

/// Prepared scoring retained for the chosen career's later rolls.
pub(crate) struct KeptScoring {
    pub(crate) scored: Scored,
    /// At least one all-targets-landed outcome has a saving below 0.5; used when preparing the roll bound.
    pub(crate) full_landing_below_threshold: bool,
}

/// Named outputs of career scoring. `Want` determines which fields are computed; Stats leaves the two
/// scalar fields at zero, Score leaves `lift_stats` empty, Touch returns zeros everywhere (no evaluation), and only Keep
/// retains the scoring tables.
pub(crate) struct CareerScore {
    /// Expected reduction in remaining careers, before subtracting the current career's cost.
    pub(crate) expected_saving: f64,
    pub(crate) improvement_chance: f64,
    pub(crate) kept: Option<KeptScoring>,
    /// Per candidate (candidate order): her lift statistics.
    pub(crate) lift_stats: Vec<LiftStats>,
}

/// Score of `career` against `context.score_base`: E[V0 - V0 with her veteran], the chance it exceeds 1e-9,
/// the scoring kept for a roll (`Want::Keep`: `Scored`, with whether an outcome landing every target reads y < 0.5),
/// and per candidate (P(her veteran lifts her), the summed lifted odds): what `want` asks for (`Want`).
/// Exact over her landed set (`outcome_sets`) and her stars per uncertain target (the
/// rank's mix): her veteran as a parent of each stage-1 candidate (her best IDX_PARTNERS partners, the trainee's cached
/// decks' corner tables, folded one target at a time); her own full veteran ends her stage when she is a candidate.
///
/// `context.pool` must be `CareerSetup::pool(context.legacies)` (the planner's callers): her veteran is then an entry
/// pushed (`CareerSetup::pool_push` builds an entry as `CareerSetup::pool` does). `score_career_of` takes any pool.
pub(crate) fn score_career(
    simulator: &mut Sim,
    context: &ScoringContext,
    career: Career,
    want: Want,
    buf: &mut Vec<f64>,
) -> CareerScore {
    let started = now_ms();
    let score = evaluate_career(simulator, context, career, want, PoolProvenance::RecordedLegacies, buf);
    prof("score_ms", now_ms() - started);
    prof("score_calls", 1.0);
    score
}

/// `score_career` for a pool built otherwise (`sim`'s): rebuild the pool from `context.legacies` and her veteran.
pub(crate) fn score_career_of(
    simulator: &mut Sim,
    context: &ScoringContext,
    career: Career,
    want: Want,
    buf: &mut Vec<f64>,
) -> CareerScore {
    let started = now_ms();
    let score = evaluate_career(simulator, context, career, want, PoolProvenance::Extended, buf);
    prof("score_ms", now_ms() - started);
    prof("score_calls", 1.0);
    score
}
