//! Career scoring over the possible veterans and their continuations.
//!
//! `score_career` and `score_career_of` (here, with what they return) are the entries; `fm_masks` is what `anchors` and
//! `roll` read of a career's landed sets.
//! - `build`: building a scoring: her setups (`prepare_setups`), `evaluate_career` and the tables (`prepare_scoring`).
//! - `kernel`: the scoring kernels that fold and evaluate a prepared scoring (`Scored`).
//! - `routes`: the routes through her veteran (`routes`): each trio with her in it, priced on its own lineage.
use crate::career::clock::{now_ms, prof};
use crate::career::plan::cont::Want;
use crate::career::plan::score::build::evaluate_career;
use crate::career::plan::score::kernel::{LiftStats, Scored};
use crate::career::plan::search::{is_full, legal_pair, val};
use crate::career::plan::{PoolProvenance, ScoringContext, TargetMask};
use crate::career::pool::Pool;
use crate::career::sim::Sim;
use crate::career::{Career, Legacy, Lineage};
pub(crate) mod build;
pub(crate) mod kernel;
mod routes;

struct FinisherCeiling {
    upper_bound: f64,
    lineage: Lineage,
}

/// The two parents (pool indices) of a trio's C and the umas she may not be (theirs).
#[derive(Clone, Copy)]
pub(crate) struct FinisherParents {
    pub(crate) first_parent: usize,
    pub(crate) second_parent: usize,
    pub(crate) excluded: [u32; 2],
}

/// The umas a C may be and the scales their odds are priced at (`ScoreBase::finisher_umas`, `ScoreBase::odds_scales`).
#[derive(Clone, Copy)]
pub(crate) struct FinisherChoice<'umas> {
    pub(crate) umas: &'umas [crate::uma::UmaIndex],
    pub(crate) scales: crate::career::plan::cont::OddsScales,
}

/// The best allowed C (`finishers.umas`, not of `excluded`'s umas, a legal pair) on `parents` in `pool` at her odds as
/// the base prices them (her odds times her uma's scale, `OddsScales::of`), with her odds (unscaled): every candidate
/// ranked by her ceiling times her scale, best first, valued (`val`) until no scaled ceiling can beat the best scaled
/// odds so far or `scaled_floor`; None where no C's scaled odds pass `scaled_floor` (every C's scaled ceiling is then at
/// most it, or her scaled value is). At scale 1 for every uma, the ranking and the comparisons are the unscaled ones
pub(crate) fn best_finisher(
    simulator: &mut Sim,
    finishers: FinisherChoice,
    pool: &Pool,
    parents: FinisherParents,
    scaled_floor: f64,
    score_buffer: &mut Vec<f64>,
) -> Option<(f64, crate::uma::UmaIndex)> {
    let data = simulator.data;
    let scale = |trainee: crate::uma::UmaIndex| finishers.scales.of(trainee);
    let mut inheritance_chances = Vec::with_capacity(simulator.env.setup.target_count);
    // (C ranges over the trainees on one pair: `lineage` and `ceiling` per trainee, not `gen_cands`, whose per-trainee
    // tables cover every legacy of the pool)
    let mut finishers: Vec<FinisherCeiling> = finishers
        .umas
        .iter()
        .copied()
        .map(|trainee| Lineage { trainee, first_parent: parents.first_parent, second_parent: parents.second_parent })
        .filter(|lineage| {
            !parents.excluded.contains(&data.umas[lineage.trainee].id) && legal_pair(data, pool, *lineage)
        })
        .filter_map(|lineage| {
            let spark_scale = simulator.env.setup.lineage_into(pool, lineage, &mut inheritance_chances);
            simulator
                .env
                .setup
                .ceiling(&mut simulator.env.state, lineage.trainee, &inheritance_chances, spark_scale)
                .map(|upper_bound| FinisherCeiling { upper_bound: upper_bound * scale(lineage.trainee), lineage })
        })
        .collect();
    finishers.sort_by(|first, second| second.upper_bound.partial_cmp(&first.upper_bound).unwrap());
    // (the best so far: her scaled odds, her odds, her uma)
    let mut best: Option<(f64, f64, crate::uma::UmaIndex)> = None;
    for &FinisherCeiling { upper_bound, lineage } in &finishers {
        if upper_bound * (1.0 + 1e-9) <= best.map_or(scaled_floor, |(scaled, _, _)| scaled.max(scaled_floor)) {
            break;
        }
        let value = val(simulator, pool, lineage, score_buffer);
        let scaled = value * scale(lineage.trainee);
        if scaled > best.map_or(scaled_floor, |(best_scaled, _, _)| best_scaled) {
            best = Some((scaled, value, lineage.trainee));
        }
    }
    best.map(|(_, value, trainee)| (value, trainee))
}

/// Whether a career's parents are both full veterans: then her own full veteran is a fully looped parent, V after her
/// 0 (the goal)
pub(crate) fn completes_goal(simulator: &Sim, legacies: &[Legacy], lineage: Lineage) -> bool {
    is_full(simulator.env.setup.skill_job, &legacies[lineage.first_parent].sparks)
        && is_full(simulator.env.setup.skill_job, &legacies[lineage.second_parent].sparks)
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
    /// With `Want::Joint`: per set of candidates her veteran lifts at once (bit c: candidate c), its chance.
    pub(crate) joint_lifts: Vec<(u64, f64)>,
    /// With `Want::Classes`: the saving split by outcome class (`kernel::ClassSaving`).
    pub(crate) classes: Vec<kernel::ClassSaving>,
}

/// Score of `career` against `context.score_base`: E[V0 - V0 with her veteran], the chance it exceeds 1e-9,
/// the scoring kept for a roll (`Want::Keep`: `Scored`), and per candidate (P(her veteran lifts her), the summed lifted
/// odds): what `want` asks for (`Want`). Exact over her landed set (`outcome_sets`) and her stars per uncertain target
/// (the rank's mix): her veteran as a parent of each stage-1 candidate (her best `ScoringKnobs::partners` partners, the
/// trainee's cached decks' corner tables, folded one target at a time: the lift statistics), and V after her over the
/// routes through her veteran (`routes`: a lifted side's one career, its C priced with her in C's lineage at each
/// outcome, and her own full veteran as a side, the trios at the base staying as they were).
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
