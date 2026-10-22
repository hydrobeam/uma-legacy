//! Career scoring over the possible veterans and their continuations.
use crate::career::clock::{now_ms, prof};
use crate::career::plan::cont::{OwnTrio, ScoreBase, TrioSideVeteran, Want, trio_v};
use crate::career::plan::fold::{
    FOLD_SCRATCH, FoldContext, FoldCursor, FoldScratch, LeafLayout, SlotTables, fold_rec, leaf_set,
};
use crate::career::plan::roll::{ActiveCandidate, Scored};
use crate::career::plan::search::{IDX_PARTNERS, Setup, is_full, legal_pair, val};
use crate::career::plan::step::{
    All, CandidateLift, EstimateBuffers, Leaves, One, OneLeaf, OwnFullLanding, StepEstimate, StepScratch, one_of,
    v_after,
};
use crate::career::plan::{CandidateIndex, OutcomeIndex, PoolProvenance, ScoringContext, TargetMask};
use crate::career::pool::Pool;
use crate::career::sim::{Cur, Sim};
use crate::career::{Career, Env, Legacy, Lineage};
/// Cached leaf probabilities depend only on the star mix and uncertain-target order.
#[derive(Default)]
struct LeafCache {
    star_probabilities: Vec<f64>,
    uncertain_landed_masks: Vec<usize>,
    /// Three probability bit patterns followed by job-target indices; empty means partial leaves.
    key: Vec<u64>,
}

/// Reused outcome buffers. Every representative is written before a later leaf reads it.
#[derive(Default)]
pub(crate) struct OutcomeScratch {
    leaf_cache: LeafCache,
    workspace: OutcomeWorkspace,
}

#[derive(Default)]
struct OutcomeWorkspace {
    representative_savings: Vec<f64>,
    /// Offset into lift_records, or u32::MAX when the representative was not recorded.
    lift_offsets: Vec<u32>,
    lift_records: Vec<LiftRecord>,
    estimate: EstimateBuffers,
}

/// A representative's candidate odds; usize::MAX ends a record.
struct LiftRecord {
    candidate: usize,
    probability: f64,
}

/// A candidate whose odds stay fixed because no setup was assigned to it.
struct UnslottedCandidate {
    candidate: usize,
    threshold: f64,
    current_probability: f64,
}

struct EvaluationCounters {
    outcomes: usize,
    changed_outcomes: usize,
    cost_evaluations: usize,
    cache_hits: usize,
    single_candidate_shortcuts: usize,
}

struct ScoringEvaluation {
    expected_saving: f64,
    improvement_chance: f64,
    outcome_savings: Vec<f64>,
    lift_stats: Vec<LiftStats>,
    full_landing_below_threshold: bool,
    counters: EvaluationCounters,
}

struct FinisherCeiling {
    upper_bound: f64,
    lineage: Lineage,
}

/// A career's own 8/8 as a trio side when her uma is not a stage candidate (her outcome lands every target): the replan
/// would take her uma in (a veteran of hers) with that real 8/8 a free side. Per candidate side (real or fresh) her 8/8 can
/// pair with, C's step: 1 / the best allowed C's value with the two 8/8s as parents (every allowed uma but hers and the
/// partner's, best ceiling first, the value taken until no ceiling can beat the best, as `plan_combinations` does).
/// Her 8/8 is the new veteran in `extended_pool` (`Env::veteran`: every target at the rank's star mix), valued as
/// every fresh side is
/// (the trios' own 8/8s use the rank's mix too), so the trios with her compare like for like with the others; one per
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
    let data = simulator.d;
    let trainee_id = data.umas[trainee].id;
    let mut trios = vec![];
    // the pool with her (`her_pool`: `Env::pool` of the legacies and hers), with each side pushed in turn (the same Pool
    // as `Env::pool` of those and the side) and popped after
    let partner_index = extended_pool.veterans.len();
    let (mut inheritance_chances, mut finishers): (Vec<f64>, Vec<FinisherCeiling>) =
        (Vec::with_capacity(simulator.env.target_count), vec![]);
    for candidate in 0..score_base.candidates.len() {
        let partner_trainee = score_base.candidates[candidate];
        if data.umas[partner_trainee].id == trainee_id {
            continue;
        }
        for TrioSideVeteran { uses_existing, legacy } in
            score_base.trio_sides.get(candidate).map_or(&[][..], |value| &**value)
        {
            simulator.env.pool_push(extended_pool, legacy);
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
                        let scale = simulator.env.lineage_into(&*extended_pool, lineage, &mut inheritance_chances);
                        simulator
                            .env
                            .ceiling(lineage.trainee, &inheritance_chances, scale)
                            .map(|upper_bound| FinisherCeiling { upper_bound, lineage })
                    }),
            );
            finishers.sort_by(|a, b| b.upper_bound.partial_cmp(&a.upper_bound).unwrap());
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
            Env::pool_pop(extended_pool);
        }
    }
    trios
}

/// V after her with her own 8/8 a trio side (`her_trios`), at the candidates' step costs c1 (a real partner side free)
pub(crate) fn with_her(v: f64, her: &[OwnTrio], c1: &[f64]) -> f64 {
    her.iter()
        .map(|&OwnTrio { candidate: CandidateIndex(ib), uses_existing: real, finisher_cost: c3 }| {
            (if real { 0.0 } else { c1[ib] }) + c3
        })
        .fold(v, f64::min)
}

/// How a career's own 8/8 ends her part of the plan (`her_ends`).
#[derive(Clone, Copy)]
pub(crate) struct HerEnds {
    /// Her place among the stage candidates; None: her uma is not one.
    pub(crate) own_candidate: Option<usize>,
    /// Her parents are both 8/8s: her own 8/8 a fully looped parent, V after her 0 (the goal).
    pub(crate) completes_goal: bool,
}

/// Career x's place among the stage candidates and whether its parents a, b are both 8/8s: what `score_career`,
/// `outcome_hi` and `pick_her` read of her first
pub(crate) fn her_ends(s: &Sim, bs: &ScoreBase, legs: &[Legacy], lineage: Lineage) -> HerEnds {
    let Lineage { trainee: x, first_parent: a, second_parent: b } = lineage;
    HerEnds {
        own_candidate: bs.candidates.iter().position(|&u| u == x),
        completes_goal: is_full(s.env.skill_job, &legs[a].sparks) && is_full(s.env.skill_job, &legs[b].sparks),
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

/// `Env::pool` of `legs` and her veteran (career x with parents a, b, schedule `c`): with `of_legs` (`pool` is
/// `Env::pool(legs)`) `pool` with her entry pushed, the same Pool (`Env::pool_push` builds an entry as `Env::pool` does)
pub(crate) fn her_pool(s: &mut Sim, legs: &[Legacy], pool: &Pool, of_legs: bool, lineage: Lineage, c: &Cur) -> Pool {
    let veteran = s.env.veteran(legs, lineage, c.wp.to_vec());
    if of_legs {
        debug_assert_eq!(pool.veterans.len(), legs.len(), "one legacy per pool member");
        let mut extended = s.take_pool(pool);
        s.env.pool_push(&mut extended, &veteran);
        extended
    } else {
        let mut extended = legs.to_vec();
        extended.push(veteran);
        s.env.pool(&extended)
    }
}

/// `her_pool`'s pool done with `of_legs`, her entry popped and the working copy kept (`Sim::put_pool`)
pub(crate) fn her_pool_done(s: &mut Sim, pool: &Pool, of_legs: bool, mut pv: Pool) {
    if of_legs {
        Env::pool_pop(&mut pv);
        s.put_pool(pool, pv);
    }
}

/// Prepared scoring retained for the chosen career's later rolls.
pub(crate) struct KeptScoring {
    pub(crate) scored: Scored,
    /// At least one all-targets-landed outcome has a saving below 0.5; used when preparing the roll bound.
    pub(crate) full_landing_below_threshold: bool,
}

/// One candidate's lift statistics from a career's scoring (`score_career`'s stats): over her outcomes, the chance her
/// veteran lifts the candidate's odds and the lifted odds summed at each outcome's chance.
#[derive(Clone, Copy, Default)]
pub(crate) struct LiftStats {
    /// pl: P(her veteran lifts the candidate).
    pub(crate) lift_chance: f64,
    /// sp: the candidate's odds summed over the outcomes that lift her, each at its chance (mean lifted odds sp / pl).
    pub(crate) lifted_odds_sum: f64,
}

impl LiftStats {
    /// A builder's lift efficiency for the candidate at odds p: her veteran lifts the candidate with chance pl to odds
    /// sp / pl on average, so pl (1/p - 1/(sp/pl)) careers saved per career (Cont's builders, the proxies' admission
    /// and `proxy_check`; PROXY_EFF was calibrated on it)
    pub(crate) fn efficiency(self, p: f64) -> f64 {
        self.lift_chance * (1.0 / p - 1.0 / (self.lifted_odds_sum / self.lift_chance))
    }
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
/// decks' corner tables, folded one target at a time); her own 8/8 ends her stage when she is a candidate.
///
/// `context.pool` must be `Env::pool(context.legacies)` (the planner's callers): her veteran is then an entry
/// pushed (`Env::pool_push` builds an entry as `Env::pool` does). `score_career_of` takes any pool.
pub(crate) fn score_career(
    s: &mut Sim,
    context: &ScoringContext,
    career: Career,
    want: Want,
    buf: &mut Vec<f64>,
) -> CareerScore {
    let started = now_ms();
    let score = score_career_(s, context, career, want, PoolProvenance::RecordedLegacies, buf);
    prof("score_ms", now_ms() - started);
    prof("score_calls", 1.0);
    score
}

/// `score_career` for a pool built otherwise (`sim`'s): rebuild the pool from `context.legacies` and her veteran.
pub(crate) fn score_career_of(
    s: &mut Sim,
    context: &ScoringContext,
    career: Career,
    want: Want,
    buf: &mut Vec<f64>,
) -> CareerScore {
    let started = now_ms();
    let score = score_career_(s, context, career, want, PoolProvenance::Extended, buf);
    prof("score_ms", now_ms() - started);
    prof("score_calls", 1.0);
    score
}

#[derive(Clone, Copy)]
struct RankedPartner {
    value: f64,
    lineage: Lineage,
}

/// Value legal partners in pool order, then keep the first two under strict comparisons.
/// val may populate the simulator's deck cache, which later candidates can read.
fn prepare_setups(
    simulator: &mut Sim,
    score_base: &ScoreBase,
    pool: &Pool,
    trainee: crate::uma::UmaIndex,
    score_buffer: &mut Vec<f64>,
) -> Vec<Setup> {
    let data = simulator.d;
    let candidate_count = score_base.candidates.len();
    let new_veteran = pool.veterans.len() - 1;
    let mut setups: Vec<Setup> = vec![];
    for candidate in 0..candidate_count {
        let candidate_trainee = score_base.candidates[candidate];
        if data.umas[candidate_trainee].id == data.umas[trainee].id
            || score_base.step_costs[candidate] == 0.0
            || !score_base.step_costs[candidate].is_finite()
        {
            continue;
        }
        // her best IDX_PARTNERS (2) partners by value, ties in partner order: every legal partner valued in order (each
        // `val` as before), the two kept by strict comparisons, as the stable sort's first two
        const _: () = assert!(IDX_PARTNERS == 2, "the partner pick below keeps exactly two");
        let mut partners: [Option<RankedPartner>; 2] = [None, None];
        for partner in 0..new_veteran {
            let lineage = Lineage { trainee: candidate_trainee, first_parent: partner, second_parent: new_veteran };
            if !legal_pair(data, pool, lineage) {
                continue;
            }
            let value = val(simulator, pool, lineage, score_buffer);
            match partners {
                [None, _] => partners[0] = Some(RankedPartner { value, lineage }),
                [Some(first), _] if value > first.value => {
                    partners[1] = partners[0];
                    partners[0] = Some(RankedPartner { value, lineage });
                }
                [_, None] => partners[1] = Some(RankedPartner { value, lineage }),
                [_, Some(second)] if value > second.value => partners[1] = Some(RankedPartner { value, lineage }),
                _ => {}
            }
        }
        for RankedPartner { lineage, .. } in partners.into_iter().flatten() {
            let Some(slot) = simulator.env.her_slot(pool, lineage) else {
                continue;
            };
            let deck_count = simulator.cache.decks[candidate_trainee].len();
            if simulator.cache.decks[candidate_trainee].iter().all(|deck| deck.tab.is_empty()) {
                continue;
            }
            setups.push(Setup {
                candidate,
                misses_without_her: slot.misses_without_her,
                target_spark_scales: slot.target_spark_scales,
                miss_by_stars: slot.miss_by_stars,
                deck_count,
            });
        }
    }
    setups
}

fn score_career_(
    simulator: &mut Sim,
    context: &ScoringContext,
    career: Career,
    want: Want,
    provenance: PoolProvenance,
    score_buffer: &mut Vec<f64>,
) -> CareerScore {
    let trainee = career.lineage.trainee;
    let recorded_pool = provenance == PoolProvenance::RecordedLegacies;

    let candidate_count = context.score_base.candidates.len();
    let target_count = simulator.env.target_count;
    let uncertain_target_count = simulator.env.uncertain_targets.len();
    let star_mix = simulator.env.skill_job.star_dist;
    let HerEnds { own_candidate, completes_goal } =
        her_ends(simulator, context.score_base, context.legacies, career.lineage);
    let current_started = now_ms();
    let current = simulator.make_cur(context.pool, career, false);
    let pool_started = now_ms();
    prof("sc_cur_ms", pool_started - current_started);
    let mut extended_pool =
        her_pool(simulator, context.legacies, context.pool, recorded_pool, career.lineage, &current);
    let new_veteran = extended_pool.veterans.len() - 1;
    // where she can be a parent: per stage-1 candidate (stage 1 still to run) her best partners
    let setups_started = now_ms();
    prof("sc_pool_ms", setups_started - pool_started);
    let mut setups = prepare_setups(simulator, context.score_base, &extended_pool, trainee, score_buffer);
    prof("sc_setup_ms", now_ms() - setups_started);
    let fold_started = now_ms();
    // per setup, per star pattern of the uncertain targets (base 4, position k = uncv[k]; 0 absent), the best table value
    // at scale 1: the tables folded one coordinate at a time (interp's order and arithmetic), every setup's tables at once
    // (`fold_rec`); per outcome (leaf, guaranteed golds fm) and lifted candidate, that value times the setup's spark scale,
    // the best over her setups and her odds now (`fold_emit`): the products and maxima the outcome loop took per outcome
    let fixed_target_count = simulator.env.fixed_targets.len();
    let fixed_landed_masks = fm_masks(&simulator.env.fixed_targets);
    // the candidates with setups (candidate order): her slot per candidate, per setup, and odds now per slot
    let mut slot_by_candidate: Vec<Option<usize>> = vec![None; candidate_count];
    let mut current_probabilities: Vec<f64> = vec![];
    for setup in &setups {
        if slot_by_candidate[setup.candidate].is_none() {
            slot_by_candidate[setup.candidate] = Some(current_probabilities.len());
            current_probabilities.push(context.score_base.current_odds[setup.candidate]);
        }
    }
    let slot_count = current_probabilities.len();
    let slot_by_setup: Vec<usize> = setups.iter().map(|setup| slot_by_candidate[setup.candidate].unwrap()).collect();
    // what the evaluation reads (`Scored`; the spark scales per setup and landed set m: the product the loop took per
    // outcome, the same order, `Scored::derive`)
    let mut scored = Scored {
        uncertain_target_count,
        fixed_target_count,
        target_count,
        slot_count,
        own_candidate,
        completes_goal,
        star_mix,
        uncertain_targets: simulator.env.uncertain_targets.clone(),
        fixed_landed_masks,
        misses_without_her: setups.iter_mut().map(|setup| std::mem::take(&mut setup.misses_without_her)).collect(),
        miss_by_stars: setups.iter().map(|setup| setup.miss_by_stars).collect(),
        target_spark_scales: setups.iter_mut().map(|setup| std::mem::take(&mut setup.target_spark_scales)).collect(),
        setup_by_table: vec![],
        initial_tables: vec![],
        collapsed_targets: vec![true; uncertain_target_count],
        slot_by_setup,
        current_probabilities,
        landed_set_chances: current.os,
        active_candidates: vec![],
        own_trios: vec![],
        present_factors: vec![],
        absent_factors: vec![],
        spark_scales: vec![],
    };
    scored.derive();
    // the kept tables and, per uncertain target, collapsed (`sc_prep`; with no setup slot no outcome reads a table: every
    // target)
    if slot_count > 0 {
        sc_prep(simulator, context.score_base, &setups, &mut scored);
    }
    prof("sc_fold_ms", now_ms() - fold_started);
    // the candidates the loop below can lift, in candidate order: those with setups, and (never, as the odds are not
    // negative) one whose odds now already clear her threshold pnow (1 + 1e-9); every other keeps up = pnow, below it
    let thresholds: Vec<f64> =
        (0..candidate_count).map(|candidate| context.score_base.current_odds[candidate] * (1.0 + 1e-9)).collect();
    let active: Vec<usize> = (0..candidate_count)
        .filter(|&candidate| {
            slot_by_candidate[candidate].is_some() || context.score_base.current_odds[candidate] > thresholds[candidate]
        })
        .collect();
    // her own 8/8 as a trio side when her uma is not a stage candidate (the full set only; `her_trios`)
    let full_mask = (1usize << target_count) - 1;
    let own_trios_started = now_ms();
    scored.own_trios = if own_candidate.is_none()
        && !completes_goal
        && !context.score_base.trio_sides.is_empty()
        && scored.landed_set_chances[full_mask] > 0.0
    {
        her_trios(simulator, context.score_base, &mut extended_pool, new_veteran, trainee, score_buffer)
    } else {
        vec![]
    };
    prof("sc_her_ms", now_ms() - own_trios_started);
    // per active candidate: (index, her slot or none, her threshold, her odds now)
    scored.active_candidates = active
        .iter()
        .map(|&candidate| ActiveCandidate {
            candidate: CandidateIndex(candidate),
            slot: slot_by_candidate[candidate],
            threshold: thresholds[candidate],
            current_chance: context.score_base.current_odds[candidate],
        })
        .collect();
    her_pool_done(simulator, context.pool, recorded_pool, extended_pool);
    if want == Want::Touch {
        return CareerScore {
            expected_saving: 0.0,
            improvement_chance: 0.0,
            kept: None,
            lift_stats: vec![LiftStats::default(); candidate_count],
        };
    }
    let mut scratch = FOLD_SCRATCH.with(|current| std::mem::take(&mut *current.borrow_mut()));
    let evaluation =
        sc_eval(context.score_base, &scored, &All, if want == Want::Keep { Want::Score } else { want }, &mut scratch);
    FOLD_SCRATCH.with(|current| *current.borrow_mut() = scratch);
    prof("sc_out", evaluation.counters.outcomes as f64);
    prof("sc_trio", evaluation.counters.changed_outcomes as f64);
    prof("sc_cost", evaluation.counters.cost_evaluations as f64);
    prof("sc_same", evaluation.counters.cache_hits as f64);
    prof("sc_one", evaluation.counters.single_candidate_shortcuts as f64);
    CareerScore {
        expected_saving: evaluation.expected_saving,
        improvement_chance: evaluation.improvement_chance,
        kept: (want == Want::Keep)
            .then_some(KeptScoring { scored, full_landing_below_threshold: evaluation.full_landing_below_threshold }),
        lift_stats: evaluation.lift_stats,
    }
}

impl Scored {
    /// Both evaluation phases borrow the same target layout and slot tables.
    fn fold_context<'score, L: Leaves>(&'score self, leaves: &'score L) -> FoldContext<'score, L> {
        FoldContext {
            leaf_layout: LeafLayout {
                uncertain_targets: &self.uncertain_targets,
                fixed_landed_masks: &self.fixed_landed_masks,
                fixed_target_count: self.fixed_target_count,
            },
            slot_tables: SlotTables {
                spark_scales: &self.spark_scales,
                slot_by_setup: &self.slot_by_setup,
                current_probabilities: &self.current_probabilities,
                setup_count: self.setup_count(),
                slot_count: self.slot_count,
            },
            initial_tables: &self.initial_tables,
            setup_by_table: &self.setup_by_table,
            present_factors: &self.present_factors,
            absent_factors: &self.absent_factors,
            collapsed_targets: &self.collapsed_targets,
            collapsed_star_levels: [0, 1],
            leaves,
            emit_setup_products: self.emits_setup_products(),
        }
    }

    fn own_full_landing(&self) -> OwnFullLanding<'_> {
        OwnFullLanding {
            candidate_index: self.own_candidate,
            completes_goal: self.completes_goal,
            trios: &self.own_trios,
            full_mask: self.full_mask().bits(),
        }
    }
}

/// A scoring's evaluation (`Scored`) at the leaves `leaves` produces: the fold of its tables into the outcome rows
/// (with
/// any setup slots: `fold_rec`), then the outcome loop (`sc_outcomes`): what `sc_outcomes` returns, with whether an
/// outcome
/// that lands every target reads y < 0.5 (with y). The plan's scorings take `All`; a roll `OneLeaf` (`sc_roll`), the same
/// arithmetic in the same order at the leaves it produces
#[inline(never)]
fn sc_eval<L: Leaves>(
    score_base: &ScoreBase,
    scored: &Scored,
    leaves: &L,
    want: Want,
    scratch: &mut FoldScratch,
) -> ScoringEvaluation {
    let (uncertain_target_count, fixed_target_count, setup_count, slot_count) =
        (scored.uncertain_target_count, scored.fixed_target_count, scored.setup_count(), scored.slot_count);
    let emit_setup_products = scored.emits_setup_products();
    let fold_context = scored.fold_context(leaves);
    let leaf_rows = leaves.nrows(1usize << (2 * uncertain_target_count));
    let outcome_value_count =
        if emit_setup_products { leaf_rows * setup_count } else { (leaf_rows << fixed_target_count) * slot_count };
    // (reused across calls: every row is written before it is read)
    if scratch.outcome_values.len() < outcome_value_count {
        scratch.outcome_values.resize(outcome_value_count, 0.0);
    }
    if slot_count > 0 {
        let table_count = scored.setup_by_table.len();
        if scratch.fold.levels.len() < uncertain_target_count + 1 {
            scratch.fold.levels.resize(uncertain_target_count + 1, vec![]);
        }
        for k in 1..=uncertain_target_count {
            let l = table_count << (uncertain_target_count - k);
            if scratch.fold.levels[k].len() < l {
                scratch.fold.levels[k].resize(l, 0.0);
            }
        }
        if scratch.fold.setup_factors.len() < 16 * setup_count {
            scratch.fold.setup_factors.resize(16 * setup_count, 0.0);
        }
        if scratch.fold.block_accumulators.len() < 16 * slot_count {
            scratch.fold.block_accumulators.resize(16 * slot_count, 0.0);
        }
        let t_fo = now_ms();
        fold_rec(
            &fold_context,
            FoldCursor { target: 0, leaf: 0, landed_mask: 0 },
            &mut scratch.fold,
            &mut scratch.outcome_values[..outcome_value_count],
        );
        prof("sc_fold_ms", now_ms() - t_fo);
    }
    let t_lo = now_ms();
    let r = sc_outcomes(
        score_base,
        scored,
        &fold_context,
        &scratch.outcome_values[..outcome_value_count],
        want,
        &mut scratch.outcomes,
    );
    prof("sc_loop_ms", now_ms() - t_lo);
    r
}

/// A roll's y (`roll_answer`): the kept scoring evaluated at one outcome (her stars `leaf`, the guaranteed targets
/// landed `fm`), `sc_eval` at `OneLeaf` (the leaf and its representative); 0 where the outcome can't happen, as the
/// plan's full table held
pub(super) fn sc_roll(score_base: &ScoreBase, scored: &Scored, outcome: OutcomeIndex) -> f64 {
    let leaf = outcome.leaf(scored.fixed_target_count);
    let fm = outcome.fixed_mask(scored.fixed_target_count).bits();
    let leaves = OneLeaf::new(leaf, scored.representative_of(leaf));
    let mut scratch = FOLD_SCRATCH.with(|c| std::mem::take(&mut *c.borrow_mut()));
    let evaluation = sc_eval(score_base, scored, &leaves, Want::Ys, &mut scratch);
    FOLD_SCRATCH.with(|c| *c.borrow_mut() = scratch);
    evaluation.outcome_savings[(leaves.row(leaf) << scored.fixed_target_count) | fm]
}

/// `score_career_`'s tables (with setup slots): every setup's tables, the dominated ones dropped, as level 0 of
/// the fold (`Scored::initial_tables`, table j at j 2^nu, its setup `setup_by_table[j]`); per uncertain target whether
/// it is collapsed (below, `collapsed_targets`). Its own function, entered once per scoring, so V8 compiles it
/// optimised after the first few (`docs/PERF.md`, *In the page*).
///
/// Collapse: target k is collapsed when, for every setup, either her x is the same f64 at 1, 2 and 3 stars (x0 = 0: then
/// the fold takes the same f64s at those levels, so it gives the same values bit for bit), or every kept table of the
/// setup has the same value with target k landed and not (`t[i] == t[i | (1 << k)]`): the table's interpolation then reads
/// no x_k, t (1 - x) + t x = t, and the fold's f64s differ by at most rounding (a few units in the last place) between
/// her star levels. A collapsed target is folded at 0 and 1 star only; the outcome loop reads 1 star's values for her 2
/// and 3 stars there (`sc_outcomes`). Every value folded is the fold's value at a leaf of the
/// full fold (the same operations on the same f64s), so the dominated-table drop stays exact at every leaf folded; and
/// the tables dropped are none that could set a maximum, so deciding the collapse on the kept tables decides it on the
/// values the outcome loop reads.
#[inline(never)]
fn sc_prep(simulator: &Sim, score_base: &ScoreBase, setups: &[Setup], scored: &mut Scored) {
    let uncertain_target_count = scored.uncertain_target_count;
    let (present_factors, absent_factors) = (&scored.present_factors, &scored.absent_factors);
    // every setup's tables, level 0 of the fold (table j at j 2^nu), and the setup of each
    let (mut setup_by_table, mut initial_tables): (Vec<usize>, Vec<f64>) = (vec![], vec![]);
    let mut dominated_tables = 0usize;
    for (si, setup) in setups.iter().enumerate() {
        let tables: Vec<&Vec<f64>> = simulator.cache.decks[score_base.candidates[setup.candidate]][..setup.deck_count]
            .iter()
            .flat_map(|deck| deck.tab.iter())
            .collect();
        // a table at most another everywhere (an equal one: the later) folds to at most that one's value at every leaf
        // (every fold coefficient is >= 0 and rounding is monotone), so the maximum is the same without it
        let nonnegative_factors = present_factors[si]
            .iter()
            .chain(&absent_factors[si])
            .all(|star_factors| star_factors.iter().all(|&v| v >= 0.0));
        for (j, table) in tables.iter().enumerate() {
            debug_assert_eq!(
                table.len(),
                1usize << uncertain_target_count,
                "one table entry per subset of the uncertain targets"
            );
            if nonnegative_factors
                && tables.iter().enumerate().any(|(k, context)| {
                    k != j
                        && table.iter().zip(context.iter()).all(|(a, b)| a <= b)
                        && (k < j || table.iter().zip(context.iter()).any(|(a, b)| a < b))
                })
            {
                dominated_tables += 1;
                continue;
            }
            initial_tables.extend_from_slice(table);
            setup_by_table.push(si);
        }
    }
    prof("sc_dom", dominated_tables as f64);
    let table_count = setup_by_table.len();
    // the collapse (above): per target, every setup's x the same at 1-3 stars, or her tables the same either side of bit k
    let table_size = 1usize << uncertain_target_count;
    let mut collapsed_targets = vec![true; uncertain_target_count];
    let mut table_collapses = 0usize;
    for (k, collapsed) in collapsed_targets.iter_mut().enumerate() {
        let mut collapsed_by_table = false;
        for (si, setup_factors) in present_factors.iter().enumerate() {
            let star_factors = &setup_factors[k];
            if star_factors[1].to_bits() == star_factors[2].to_bits()
                && star_factors[2].to_bits() == star_factors[3].to_bits()
            {
                continue;
            }
            let flat = (0..table_count).filter(|&j| setup_by_table[j] == si).all(|j| {
                let t = &initial_tables[j * table_size..(j + 1) * table_size];
                (0..table_size).all(|i| i >> k & 1 == 1 || t[i] == t[i | (1 << k)])
            });
            if !flat {
                *collapsed = false;
                break;
            }
            collapsed_by_table = true;
        }
        if *collapsed && collapsed_by_table {
            table_collapses += 1;
        }
    }
    prof("sc_col", collapsed_targets.iter().filter(|&&c| c).count() as f64);
    prof("sc_col_tab", table_collapses as f64);
    (scored.setup_by_table, scored.initial_tables, scored.collapsed_targets) =
        (setup_by_table, initial_tables, collapsed_targets);
}

/// `score_career_`'s outcome loop: every outcome `fold_context.leaves` produces (her stars per uncertain target,
/// the guaranteed golds present or not), its chance, V after her and y; (E[y], P(y > 1e-9), y per outcome (at its
/// leaf's row), per candidate
/// stats: what `want` asks for; whether an outcome landing every target reads y < 0.5; counters). Its own function,
/// entered once per scoring, so V8 compiles it optimised after the first few (`docs/PERF.md`, *In the page*). On a
/// collapsed target (`collapsed_targets`, `sc_prep`: folded at 0 and 1 star only) a leaf with 2 or 3 stars there reads
/// its representative's values (`representative_of`: those digits set to 1, so a smaller leaf, met first): y as
/// computed there (`representative_savings`) and each candidate's odds for the stats; every sum (E[y], the lift,
/// the stats) still takes
/// every outcome in the loop's order at its own chance, the same sums as without the collapse wherever the
/// representative's values are the leaf's own.
///
/// What is fixed per call is decided here, once: the loop (`sc_loop`) is specialised by whether it takes the products
/// itself (`emits_setup_products`), whether any target is collapsed and what is wanted (y or the stats), and the
/// candidates it can lift are split into those with a slot (slot j is the j-th of them: slots are given in candidate
/// order) and those without one (odds fixed at their odds now)
#[inline(never)]
fn sc_outcomes<L: Leaves>(
    score_base: &ScoreBase,
    scored: &Scored,
    fold_context: &FoldContext<L>,
    u: &[f64],
    want: Want,
    scratch: &mut OutcomeScratch,
) -> ScoringEvaluation {
    let leaves = fold_context.leaves;
    let n = score_base.candidates.len();
    let (uncertain_target_count, slot_count) = (scored.uncertain_target_count, scored.slot_count);
    // V after her when one candidate's step alone moved (`one_of`, `single_candidate_value`: trio_v's minimum,
    // split by the trios that read her step)
    let single_candidate_costs: Vec<One> = if score_base.trios.is_empty() || want == Want::Stats {
        vec![]
    } else {
        (0..n).map(|i| one_of(score_base.trios, score_base.step_costs, i)).collect()
    };
    prepare_leaf_cache(scored, leaves, &mut scratch.leaf_cache);
    // A leaf's representative has each nonzero collapsed-target digit at 1.
    let collapse_mask: usize =
        (0..uncertain_target_count).filter(|&k| scored.collapsed_targets[k]).map(|k| 3usize << (2 * k)).sum();
    let low_digit_bits = collapse_mask & (usize::MAX / 3);
    // the candidates it can lift: with a slot (in slot order) and without
    let (mut candidate_indices, mut thresholds, mut unslotted_candidates) = (vec![], vec![], vec![]);
    for &ActiveCandidate {
        candidate: CandidateIndex(candidate),
        slot: sl,
        threshold,
        current_chance: current_probability,
    } in &scored.active_candidates
    {
        match sl {
            Some(sl) => {
                assert_eq!(sl, candidate_indices.len(), "slotted candidates come in slot order");
                candidate_indices.push(candidate);
                thresholds.push(threshold);
            }
            None => unslotted_candidates.push(UnslottedCandidate { candidate, threshold, current_probability }),
        }
    }
    assert_eq!(candidate_indices.len(), slot_count, "one slotted candidate per slot");
    // Setups are in slot order; slot j owns setup_offsets[j]..setup_offsets[j + 1].
    let mut setup_offsets = vec![0usize; slot_count + 1];
    for (si, &sl) in scored.slot_by_setup.iter().enumerate() {
        assert!(si == 0 || scored.slot_by_setup[si - 1] <= sl, "setups come in slot order");
        setup_offsets[sl + 1] = si + 1;
    }
    // V after her found lazily (`sc_loop`): no step, stage 2-3 cost or C's step NaN, -0.0 or below 0, so no sum of them is
    // NaN or -0.0 and equal minima are the same bits
    let ok = |c: f64| c >= 0.0 && c.to_bits() != (-0.0f64).to_bits();
    let lazy_costs = score_base.step_costs.iter().all(|&c| ok(c))
        && score_base.later_stage_costs.iter().all(|&c| ok(c))
        && score_base.trios.iter().all(|t| ok(t.finisher_cost));
    let context = OutcomeContext {
        score_base,
        leaf_layout: &fold_context.leaf_layout,
        slot_tables: &fold_context.slot_tables,
        candidates: CandidateSlots {
            values: u,
            setup_offsets: &setup_offsets,
            candidate_indices: &candidate_indices,
            thresholds: &thresholds,
            unslotted_candidates: &unslotted_candidates,
        },
        own_full_landing: scored.own_full_landing(),
        probabilities: OutcomeProbabilities {
            outcome_probabilities: &scored.landed_set_chances,
            star_probabilities: &scratch.leaf_cache.star_probabilities,
            uncertain_landed_masks: &scratch.leaf_cache.uncertain_landed_masks,
            collapse_mask,
            low_digit_bits,
        },
        retain_outcome_savings: want == Want::Ys,
        single_candidate_costs: &single_candidate_costs,
        lazy_costs,
        leaves,
    };
    let OutcomeWorkspace { representative_savings, lift_offsets, lift_records, estimate } = &mut scratch.workspace;
    // (Y: y and its sums, S: the stats)
    match (scored.emits_setup_products(), collapse_mask != 0, want == Want::Stats) {
        (false, false, false) => sc_loop::<false, false, true, false, L>(
            &context,
            representative_savings,
            lift_offsets,
            lift_records,
            estimate,
        ),
        (false, true, false) => sc_loop::<false, true, true, false, L>(
            &context,
            representative_savings,
            lift_offsets,
            lift_records,
            estimate,
        ),
        (true, false, false) => sc_loop::<true, false, true, false, L>(
            &context,
            representative_savings,
            lift_offsets,
            lift_records,
            estimate,
        ),
        (true, true, false) => sc_loop::<true, true, true, false, L>(
            &context,
            representative_savings,
            lift_offsets,
            lift_records,
            estimate,
        ),
        (false, false, true) => sc_loop::<false, false, false, true, L>(
            &context,
            representative_savings,
            lift_offsets,
            lift_records,
            estimate,
        ),
        (false, true, true) => sc_loop::<false, true, false, true, L>(
            &context,
            representative_savings,
            lift_offsets,
            lift_records,
            estimate,
        ),
        (true, false, true) => sc_loop::<true, false, false, true, L>(
            &context,
            representative_savings,
            lift_offsets,
            lift_records,
            estimate,
        ),
        (true, true, true) => sc_loop::<true, true, false, true, L>(
            &context,
            representative_savings,
            lift_offsets,
            lift_records,
            estimate,
        ),
    }
}

fn prepare_leaf_cache<L: Leaves>(scored: &Scored, leaves: &L, cache: &mut LeafCache) {
    let (uncertain_target_count, w) = (scored.uncertain_target_count, scored.star_mix);
    // per leaf (at its row) her landed set on the uncertain targets and her stars' chance: the product over k of w[her
    // stars on target k - 1] where she has any, in k order from 1.0 (every leaf: the leaves with digit k > 0 extend those
    // below 4^k; else per leaf, `leaf_set`, the same f64s)
    // (they depend on w and uncv alone: built for every leaf, they are kept until those change, `cache.key`)
    if L::ALL {
        let same = cache.key.len() == 3 + uncertain_target_count
            && cache.key[..3].iter().zip(&w).all(|(&k, x)| k == x.to_bits())
            && cache.key[3..].iter().zip(&scored.uncertain_targets).all(|(&k, &i)| k == i as u64);
        if !same {
            cache.star_probabilities.clear();
            cache.uncertain_landed_masks.clear();
            cache.star_probabilities.push(1.0);
            cache.uncertain_landed_masks.push(0usize);
            for k in 0..uncertain_target_count {
                let lo = 1usize << (2 * k);
                for dg in 1..4usize {
                    for i in 0..lo {
                        cache.star_probabilities.push(cache.star_probabilities[i] * w[dg - 1]);
                        cache
                            .uncertain_landed_masks
                            .push(cache.uncertain_landed_masks[i] | (1 << scored.uncertain_targets[k]));
                    }
                }
            }
            cache.key.clear();
            cache.key.extend(w.iter().map(|x| x.to_bits()));
            cache.key.extend(scored.uncertain_targets.iter().map(|&i| i as u64));
        }
    } else {
        cache.key.clear();
        cache.star_probabilities.clear();
        cache.uncertain_landed_masks.clear();
        for &l in leaves.list() {
            let outcome = leaf_set(l, &scored.uncertain_targets, w);
            cache.star_probabilities.push(outcome.star_probability);
            cache.uncertain_landed_masks.push(outcome.landed_mask);
        }
    }
}

/// Outcome-only rows and candidate lookup, alongside the shared SlotTables view.
struct CandidateSlots<'slots> {
    values: &'slots [f64],
    /// Slot j owns setups offsets[j]..offsets[j + 1].
    setup_offsets: &'slots [usize],
    candidate_indices: &'slots [usize],
    thresholds: &'slots [f64],
    unslotted_candidates: &'slots [UnslottedCandidate],
}

struct OutcomeProbabilities<'out> {
    /// Probability indexed by the job-target landed mask, before the star mix.
    outcome_probabilities: &'out [f64],
    /// Both leaf arrays use Leaves::row, including partial-leaf scorings.
    star_probabilities: &'out [f64],
    uncertain_landed_masks: &'out [usize],
    collapse_mask: usize,
    low_digit_bits: usize,
}

/// Immutable outcome inputs, separate from the reusable caches and accumulators.
struct OutcomeContext<'out, 'base, L: Leaves> {
    score_base: &'out ScoreBase<'base>,
    leaf_layout: &'out LeafLayout<'out>,
    slot_tables: &'out SlotTables<'out>,
    candidates: CandidateSlots<'out>,
    probabilities: OutcomeProbabilities<'out>,
    own_full_landing: OwnFullLanding<'out>,
    leaves: &'out L,
    retain_outcome_savings: bool,
    lazy_costs: bool,
    single_candidate_costs: &'out [One],
}

/// Each slot's odds after her at row r (outcome m, fm): with PL the best of her odds now and her setups' spark scale times
/// table value, per slot over its setups in setup order (the same strict maxima `fold_emit` takes), else the folded row
#[inline(always)]
fn sc_row<'out, const PL: bool, L: Leaves>(
    context: &'out OutcomeContext<L>,
    leaf_row: usize,
    landed_mask: usize,
    fixed_mask: usize,
    probability_buffer: &'out mut [f64],
) -> &'out [f64] {
    if PL {
        let (folded_row, scale_row) = (
            &context.candidates.values[leaf_row * context.slot_tables.setup_count..][..context.slot_tables.setup_count],
            &context.slot_tables.spark_scales[landed_mask * context.slot_tables.setup_count..]
                [..context.slot_tables.setup_count],
        );
        for ((probability, &current_probability), setup_range) in probability_buffer
            .iter_mut()
            .zip(context.slot_tables.current_probabilities)
            .zip(context.candidates.setup_offsets.windows(2))
        {
            let mut best = current_probability;
            for (&scratch, &factor) in
                scale_row[setup_range[0]..setup_range[1]].iter().zip(&folded_row[setup_range[0]..setup_range[1]])
            {
                let value = scratch * factor;
                if value > best {
                    best = value;
                }
            }
            *probability = best;
        }
        probability_buffer
    } else {
        &context.candidates.values
            [((leaf_row << context.leaf_layout.fixed_target_count) | fixed_mask) * context.slot_tables.slot_count..]
            [..context.slot_tables.slot_count]
    }
}

/// `sc_outcomes`' loop, specialised: PL (the products taken here, `sc_row`), COL (a target collapsed), Y (y and its sums
/// computed; else E[y] and the lift 0.0, no step costed), S (the stats; else none), L (the leaves taken, in order; rows,
/// y and the representatives' records at each leaf's row). With S a representative outcome that
/// other leaves read records the candidates it lifts and their odds (`lift_records` from `lift_offsets[outcome]`, ended by usize::MAX;
/// u32::MAX: not recorded, past the cap), which the leaves reading it add to their stats (each candidate's sums still in
/// outcome order: the same f64s); unrecorded, they take its row again. Separate mutable buffer
/// borrows preserve nonaliasing information across this hot function boundary.
#[inline(never)]
fn sc_loop<const PL: bool, const COL: bool, const Y: bool, const S: bool, L: Leaves>(
    context: &OutcomeContext<L>,
    representative_savings: &mut Vec<f64>,
    lift_offsets: &mut Vec<u32>,
    lift_records: &mut Vec<LiftRecord>,
    estimate_buffers: &mut EstimateBuffers,
) -> ScoringEvaluation {
    let score_base = context.score_base;
    // Keep immutable hot slices independent of writes to the reusable outcome buffers.
    let candidate_indices = context.candidates.candidate_indices;
    let thresholds = context.candidates.thresholds;
    let unslotted_candidates = context.candidates.unslotted_candidates;
    let outcome_probabilities = context.probabilities.outcome_probabilities;
    let fixed_landed_masks = context.leaf_layout.fixed_landed_masks;
    let candidate_count = score_base.candidates.len();
    let (fixed_target_count, slot_count, full_mask) =
        (context.leaf_layout.fixed_target_count, context.slot_tables.slot_count, context.own_full_landing.full_mask);
    let (leaf_count, leaves) = ((1usize << (2 * context.leaf_layout.uncertain_targets.len())), context.leaves);
    let leaf_rows = leaves.nrows(leaf_count);
    // every outcome: uncertain targets' star pattern, guaranteed golds present or not
    let (mut expected_saving, mut improvement_chance) = (0.0f64, 0.0f64);
    // y per outcome (careers saved), at (row << nf) | fm (0 where the outcome can't happen); only when the caller reads it
    let mut outcome_savings: Vec<f64> =
        if context.retain_outcome_savings { vec![0.0; leaf_rows << fixed_target_count] } else { vec![] };
    // an outcome that can happen, lands every target and reads y < 0.5 (`pick_her`)
    let mut full_landing_below_threshold = false;
    let mut step_costs = score_base.step_costs.to_vec();
    // per stage-1 candidate: the chance her veteran raises that candidate's odds, and the odds raised to (summed, weighted)
    let mut lift_stats = if S { vec![LiftStats::default(); candidate_count] } else { vec![] };
    let mut changed_candidates: Vec<usize> = vec![];
    let mut step_scratch = StepScratch { costs: &mut step_costs, changed_candidates: &mut changed_candidates };
    let mut outcome_count = 0usize;
    let mut probability_buffer = vec![0.0f64; slot_count];
    // V after her (`v_after` at the estimate's step costs, `Est`), from the candidates each outcome lifts (row, candidate,
    // odds: `lifted`)
    let mut estimate = if Y {
        StepEstimate::new(
            score_base,
            slot_count + unslotted_candidates.len(),
            context.lazy_costs,
            context.single_candidate_costs,
            std::mem::take(estimate_buffers),
        )
    } else {
        StepEstimate::default()
    };
    let mut lifted: Vec<CandidateLift> = vec![];
    let (collapse_mask, low_digit_bits) = (context.probabilities.collapse_mask, context.probabilities.low_digit_bits);
    let rep_of = |l: usize| {
        let z = l & collapse_mask;
        (l & !collapse_mask) | ((z | z >> 1) & low_digit_bits)
    };
    // Bound the representative records to about twice the savings buffer's memory.
    let record_capacity = (leaf_count << fixed_target_count).clamp(1 << 16, u32::MAX as usize - 1);
    // Buffers only grow. A reused representative precedes this leaf and has the same landed set,
    // so its positive outcome probability made this call write both its saving and record offset.
    if COL {
        if Y && representative_savings.len() < leaf_rows << fixed_target_count {
            representative_savings.resize(leaf_rows << fixed_target_count, 0.0);
        }
        if S {
            if lift_offsets.len() < leaf_rows << fixed_target_count {
                lift_offsets.resize(leaf_rows << fixed_target_count, u32::MAX);
            }
            lift_records.clear();
            lift_records.push(LiftRecord { candidate: usize::MAX, probability: 0.0 });
        }
    }
    let visited_leaves = if L::ALL { leaf_count } else { leaves.list().len() };
    for li in 0..visited_leaves {
        let leaf = if L::ALL { li } else { leaves.list()[li] };
        let leaf_row = leaves.row(leaf);
        let (uncertain_mask, star_probability) = (
            context.probabilities.uncertain_landed_masks[leaf_row],
            context.probabilities.star_probabilities[leaf_row],
        );
        let representative = if COL { rep_of(leaf) } else { leaf };
        // (a representative that other leaves read: y computed wherever her landed set can happen, even at chance 0 itself)
        let is_representative = COL && representative == leaf && leaf & collapse_mask != 0;
        for (fixed_mask, &fixed_landed_mask) in fixed_landed_masks.iter().enumerate() {
            let landed_mask = uncertain_mask | fixed_landed_mask.bits();
            let outcome_probability = outcome_probabilities[landed_mask] * star_probability;
            if outcome_probability <= 0.0 && !(is_representative && outcome_probabilities[landed_mask] > 0.0) {
                continue;
            }
            if COL && representative != leaf {
                // the representative's y, the stats at the representative's odds and this outcome's chance
                outcome_count += 1;
                let representative_row = leaves.row(representative);
                let representative_outcome = (representative_row << fixed_target_count) | fixed_mask;
                if S {
                    let record_start = lift_offsets[representative_outcome];
                    if record_start == u32::MAX {
                        let probabilities = sc_row::<PL, L>(
                            context,
                            representative_row,
                            landed_mask,
                            fixed_mask,
                            &mut probability_buffer,
                        );
                        for ((&lifted_probability, &threshold), &candidate) in
                            probabilities.iter().zip(thresholds).zip(candidate_indices)
                        {
                            if lifted_probability > threshold {
                                lift_stats[candidate].lift_chance += outcome_probability;
                                lift_stats[candidate].lifted_odds_sum += outcome_probability * lifted_probability;
                            }
                        }
                        for &UnslottedCandidate { candidate, threshold, current_probability } in unslotted_candidates {
                            if current_probability > threshold {
                                lift_stats[candidate].lift_chance += outcome_probability;
                                lift_stats[candidate].lifted_odds_sum += outcome_probability * current_probability;
                            }
                        }
                    } else {
                        for &LiftRecord { candidate, probability: lifted_probability } in
                            lift_records[record_start as usize..].iter().take_while(|e| e.candidate != usize::MAX)
                        {
                            lift_stats[candidate].lift_chance += outcome_probability;
                            lift_stats[candidate].lifted_odds_sum += outcome_probability * lifted_probability;
                        }
                    }
                }
                if Y {
                    let saving = representative_savings[representative_outcome];
                    expected_saving += outcome_probability * saving;
                    if saving > 1e-9 {
                        improvement_chance += outcome_probability;
                    }
                    if landed_mask == full_mask && saving < 0.5 {
                        full_landing_below_threshold = true;
                    }
                    if context.retain_outcome_savings {
                        outcome_savings[(leaf_row << fixed_target_count) | fixed_mask] = saving;
                    }
                }
                continue;
            }
            // each candidate's odds after her: the best of her setups' and her odds now (`fold_emit`; with pl, here)
            let probabilities = sc_row::<PL, L>(context, leaf_row, landed_mask, fixed_mask, &mut probability_buffer);
            let possible = outcome_probability > 0.0;
            if possible {
                outcome_count += 1;
            }
            // (a representative records what it lifts, within the cap)
            let record_lifts = S
                && is_representative
                && lift_records.len() + slot_count + unslotted_candidates.len() < record_capacity;
            let record_start = lift_records.len();
            // the candidates it lifts: their stats and records here, V after her from the list (in this order)
            lifted.clear();
            macro_rules! lifted {
                ($ci:expr, $candidate:expr, $lifted_probability:expr) => {{
                    let (candidate, lifted_probability): (usize, f64) = ($candidate, $lifted_probability);
                    if S && possible {
                        lift_stats[candidate].lift_chance += outcome_probability;
                        lift_stats[candidate].lifted_odds_sum += outcome_probability * lifted_probability;
                    }
                    if record_lifts {
                        lift_records.push(LiftRecord { candidate, probability: lifted_probability });
                    }
                    if Y {
                        lifted.push(CandidateLift { slot: $ci, candidate, probability: lifted_probability });
                    }
                }};
            }
            for (j, ((&lifted_probability, &threshold), &candidate)) in
                probabilities.iter().zip(thresholds).zip(candidate_indices).enumerate()
            {
                if lifted_probability > threshold {
                    lifted!(j, candidate, lifted_probability);
                }
            }
            for (j, &UnslottedCandidate { candidate, threshold, current_probability }) in
                unslotted_candidates.iter().enumerate()
            {
                if current_probability > threshold {
                    lifted!(slot_count + j, candidate, current_probability);
                }
            }
            if S && is_representative {
                let outcome_index = (leaf_row << fixed_target_count) | fixed_mask;
                lift_offsets[outcome_index] = if !record_lifts {
                    u32::MAX
                } else if lift_records.len() == record_start {
                    0
                } else {
                    lift_records.push(LiftRecord { candidate: usize::MAX, probability: 0.0 });
                    record_start as u32
                };
            }
            if !Y {
                continue;
            }
            let saving =
                v_after(score_base, &mut estimate, &mut step_scratch, &lifted, landed_mask, &context.own_full_landing);
            if is_representative {
                representative_savings[(leaf_row << fixed_target_count) | fixed_mask] = saving;
            }
            if !possible {
                continue;
            }
            expected_saving += outcome_probability * saving;
            if saving > 1e-9 {
                improvement_chance += outcome_probability;
            }
            if landed_mask == full_mask && saving < 0.5 {
                full_landing_below_threshold = true;
            }
            if context.retain_outcome_savings {
                outcome_savings[(leaf_row << fixed_target_count) | fixed_mask] = saving;
            }
        }
    }
    if Y {
        *estimate_buffers = EstimateBuffers {
            probability_costs: std::mem::take(&mut estimate.workspace.probability_costs),
            cell_costs: std::mem::take(&mut estimate.workspace.cell_costs),
        };
    }
    ScoringEvaluation {
        expected_saving,
        improvement_chance,
        outcome_savings,
        lift_stats,
        full_landing_below_threshold,
        counters: EvaluationCounters {
            outcomes: outcome_count,
            changed_outcomes: estimate.workspace.changed_outcomes,
            cost_evaluations: estimate.workspace.cost_evaluations,
            cache_hits: estimate.workspace.cache_hits,
            single_candidate_shortcuts: estimate.workspace.single_candidate_shortcuts,
        },
    }
}

/// V after her before her own 8/8 and the goal, at the candidates' step costs c1: the trios' least cost (`trio_v`; with no
/// trio, the least of each candidate's step plus her stages 2-3)
pub(crate) fn v0_after(bs: &ScoreBase, c1: &[f64]) -> f64 {
    if bs.trios.is_empty() {
        (0..bs.candidates.len()).map(|a| c1[a] + bs.later_stage_costs[a]).fold(f64::INFINITY, f64::min)
    } else {
        trio_v(c1, bs.trios)
    }
}
