//! Career scoring over the possible veterans and their continuations.
use crate::career::clock::{now_ms, prof};
use crate::career::plan::cont::{OwnTrio, ScoreBase, TrioSideVeteran, Want};
use crate::career::plan::fold::{All, Leaves, OneLeaf};
use crate::career::plan::score::kernel::{ActiveCandidate, FOLD_SCRATCH, LiftStats, Scored};
use crate::career::plan::search::{IDX_PARTNERS, is_full, legal_pair, val};
use crate::career::plan::{CandidateIndex, OutcomeIndex, PoolProvenance, ScoringContext, TargetMask};
use crate::career::pool::Pool;
use crate::career::sim::{Cur, Sim};
use crate::career::{Career, Env, Legacy, Lineage};
pub(crate) mod kernel;

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
pub(crate) fn her_ends(simulator: &Sim, score_base: &ScoreBase, legacies: &[Legacy], lineage: Lineage) -> HerEnds {
    let Lineage { trainee: x, first_parent: a, second_parent: b } = lineage;
    HerEnds {
        own_candidate: score_base.candidates.iter().position(|&u| u == x),
        completes_goal: is_full(simulator.env.skill_job, &legacies[a].sparks)
            && is_full(simulator.env.skill_job, &legacies[b].sparks),
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

/// Build the pool with this career's veteran. A recorded pool is the pool of `legacies`, so reuse its
/// working copy and push one entry; otherwise rebuild from the recorded legacies and the new veteran.
pub(crate) fn her_pool(
    simulator: &mut Sim,
    legacies: &[Legacy],
    pool: &Pool,
    recorded_pool: bool,
    lineage: Lineage,
    outcome: &Cur,
) -> Pool {
    let veteran = simulator.env.veteran(legacies, lineage, outcome.wp.to_vec());
    if recorded_pool {
        debug_assert_eq!(pool.veterans.len(), legacies.len(), "one legacy per pool member");
        let mut extended = simulator.take_pool(pool);
        simulator.env.pool_push(&mut extended, &veteran);
        extended
    } else {
        let mut extended = legacies.to_vec();
        extended.push(veteran);
        simulator.env.pool(&extended)
    }
}

/// Return a recorded pool's working copy after popping the new veteran (`Sim::put_pool`).
pub(crate) fn her_pool_done(simulator: &mut Sim, pool: &Pool, recorded_pool: bool, mut extended_pool: Pool) {
    if recorded_pool {
        Env::pool_pop(&mut extended_pool);
        simulator.put_pool(pool, extended_pool);
    }
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
/// decks' corner tables, folded one target at a time); her own 8/8 ends her stage when she is a candidate.
///
/// `context.pool` must be `Env::pool(context.legacies)` (the planner's callers): her veteran is then an entry
/// pushed (`Env::pool_push` builds an entry as `Env::pool` does). `score_career_of` takes any pool.
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

/// One place a builder's new veteran may be a parent: the stage-1 candidate she may lift, her slot's miss and spark
/// parts there (`HerSlot`), and how many of the candidate's cached decks it reads.
struct Setup {
    /// The candidate she may lift, as an index into `ScoreBase::candidates`.
    candidate: usize,
    /// x0: per uncertain target (`Env::uncertain_targets` order), the miss product without her.
    misses_without_her: Vec<f64>,
    /// pw: per job target, the spark scale without and with her spark.
    target_spark_scales: Vec<[f64; 2]>,
    /// xm: her miss factor x per star count (0: absent).
    miss_by_stars: [f64; 4],
    /// The candidate's cached decks it reads (their corner tables): a trainee's cached decks only grow, so the first
    /// `deck_count` are the ones there when the setup was made.
    deck_count: usize,
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

fn evaluate_career(
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
    // (`FoldContext::fold`); per outcome (leaf, guaranteed golds fm) and lifted candidate, that value times the setup's spark scale,
    // the best over her setups and her odds now (`FoldContext::emit`): the products and maxima the outcome loop took per outcome
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
    // the kept tables and, per uncertain target, collapsed (`prepare_scoring`; with no setup slot no outcome reads a table: every
    // target)
    if slot_count > 0 {
        prepare_scoring(simulator, context.score_base, &setups, &mut scored);
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
        scored.evaluate(context.score_base, &All, if want == Want::Keep { Want::Score } else { want }, &mut scratch);
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
    /// A roll's y (`roll_answer`): the kept scoring evaluated at one outcome (her stars `leaf`, the guaranteed targets
    /// landed `fm`), `Scored::evaluate` at `OneLeaf` (the leaf and its representative); 0 where the outcome can't happen, as the
    /// plan's full table held
    pub(super) fn roll(&self, score_base: &ScoreBase, outcome: OutcomeIndex) -> f64 {
        let leaf = outcome.leaf(self.fixed_target_count);
        let fixed_mask = outcome.fixed_mask(self.fixed_target_count).bits();
        let leaves = OneLeaf::new(leaf, self.representative_of(leaf));
        let mut scratch = FOLD_SCRATCH.with(|c| std::mem::take(&mut *c.borrow_mut()));
        let evaluation = self.evaluate(score_base, &leaves, Want::Ys, &mut scratch);
        FOLD_SCRATCH.with(|c| *c.borrow_mut() = scratch);
        evaluation.outcome_savings[(leaves.row(leaf) << self.fixed_target_count) | fixed_mask]
    }
}

/// `evaluate_career`'s tables (with setup slots): every setup's tables, the dominated ones dropped, as level 0 of
/// the fold (`Scored::initial_tables`, table j at j 2^nu, its setup `setup_by_table[j]`); per uncertain target whether
/// it is collapsed (below, `collapsed_targets`). Its own function, entered once per scoring, so V8 compiles it
/// optimised after the first few (`docs/PERF.md`, *In the page*).
///
/// Collapse: target k is collapsed when, for every setup, either her x is the same f64 at 1, 2 and 3 stars (x0 = 0: then
/// the fold takes the same f64s at those levels, so it gives the same values bit for bit), or every kept table of the
/// setup has the same value with target k landed and not (`t[i] == t[i | (1 << k)]`): the table's interpolation then reads
/// no x_k, t (1 - x) + t x = t, and the fold's f64s differ by at most rounding (a few units in the last place) between
/// her star levels. A collapsed target is folded at 0 and 1 star only; the outcome loop reads 1 star's values for her 2
/// and 3 stars there (`evaluate_outcomes`). Every value folded is the fold's value at a leaf of the
/// full fold (the same operations on the same f64s), so the dominated-table drop stays exact at every leaf folded; and
/// the tables dropped are none that could set a maximum, so deciding the collapse on the kept tables decides it on the
/// values the outcome loop reads.
#[inline(never)]
fn prepare_scoring(simulator: &Sim, score_base: &ScoreBase, setups: &[Setup], scored: &mut Scored) {
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
