//! Building a career's scoring: the places her veteran may be a parent (`prepare_setups`), the scoring over her
//! outcomes (`evaluate_career`) and its tables, collapsed per uncertain target (`prepare_scoring`). The kernels it runs
//! are in `kernel`.
use crate::career::clock::{now_ms, prof};
use crate::career::plan::cont::{ScoreBase, Want};
use crate::career::plan::fold::{All, Leaves, OneLeaf};
use crate::career::plan::score::kernel::{FOLD_SCRATCH, LiftStats, Scored};
use crate::career::plan::score::routes::{RouteInputs, RouteSet, routes};
use crate::career::plan::score::{CareerScore, KeptScoring, completes_goal, fm_masks};
use crate::career::plan::search::{legal_pair, val};
use crate::career::plan::{OutcomeIndex, PoolProvenance, ScoringContext};
use crate::career::pool::Pool;
use crate::career::sim::Sim;
use crate::career::{Career, Lineage};
/// One place a career's new veteran may be in a lineage, one slot of the scoring: a stage-1 candidate's parent beside a
/// partner (a partner setup), or in a route's C's lineage (a finisher setup, `routes`); her slot's miss and spark parts
/// there (`HerSlot`), and how many of the lineage trainee's cached decks it reads.
pub(super) struct Setup {
    /// The candidate she may lift, as an index into `ScoreBase::candidates` (a finisher setup: C's route's, unread).
    pub(super) candidate: usize,
    /// The lineage's trainee, whose cached decks' tables the setup reads.
    pub(super) trainee: crate::uma::UmaIndex,
    /// Her partner there (a legacy index; a finisher setup: unread).
    pub(super) partner: usize,
    /// x0: per uncertain target (`CareerSetup::uncertain_targets` order), the miss product without her.
    pub(super) misses_without_her: Vec<f64>,
    /// pw: per job target, the spark scale without and with her spark.
    pub(super) target_spark_scales: Vec<[f64; 2]>,
    /// xm: her miss factor x per star count (0: absent).
    pub(super) miss_by_stars: [f64; 4],
    /// The trainee's cached decks it reads (their corner tables): a trainee's cached decks only grow, so a range of
    /// them names the same decks later.
    pub(super) decks: std::ops::Range<usize>,
    /// The one outfit option's table it reads of each deck (a deck holds one per option); None: every option's.
    pub(super) outfit: Option<usize>,
}

impl Setup {
    /// The corner tables it reads: per deck of `decks`, its `outfit` option's table, or every one.
    pub(super) fn tables<'cache>(&self, simulator: &'cache Sim) -> impl Iterator<Item = &'cache Vec<f64>> {
        let outfit = self.outfit;
        simulator.evaluator.cache.decks[self.trainee][self.decks.clone()].iter().flat_map(move |deck| {
            deck.tables
                .iter()
                .enumerate()
                .filter(move |(option, _)| outfit.is_none_or(|kept| kept == *option))
                .map(|(_, table)| table)
        })
    }
    /// The most the setup's odds can be at any outcome: the most any table it reads holds times the most spark scale
    /// (each outcome's odds is a table interpolated at miss chances in [0, 1], a convex combination of its entries,
    /// times the scale of her landed set); +inf where a miss factor lies outside [0, 1]
    pub(super) fn odds_bound(&self, simulator: &Sim) -> f64 {
        if !self
            .misses_without_her
            .iter()
            .all(|&miss| self.miss_by_stars.iter().all(|&factor| (0.0..=1.0).contains(&(miss * factor))))
        {
            return f64::INFINITY;
        }
        let most_table = self.tables(simulator).flatten().fold(0.0f64, |most, &value| most.max(value));
        let most_scale: f64 = self.target_spark_scales.iter().map(|scales| scales[0].max(scales[1])).product();
        most_table * most_scale
    }
}

/// The outcome rows' memory a scoring may take, in f64 entries (256 MiB, the same natively and in wasm so the page
/// answers as the native validation does; a page's linear memory never shrinks and phones hold less): the fold's
/// per-leaf values (4^nu leaves times the setups, `Scored::evaluate`) and its levels (twice the tables' corners). The
/// partner and lift slots take at most 2 x partners x IDX_CANDS of the setups: a job whose leaf rows alone for them
/// pass the budget is refused before it runs (`sim_for`); past it with their corner tables (at 10 uncertain targets
/// the default 2 partners' leaf rows take all of it), the setups ranked last are left out (`fit_partner_setups`), so an
/// admitted plan never fails for memory; the finisher slots fill what is left (`routes`: the routes past it left out,
/// highest lower bounds first, counted in `routes_dropped`)
pub(crate) const OUTCOME_BUDGET: usize = 1 << 25;

/// The partner setups within `OUTCOME_BUDGET` (each taking `factor` times its entries, `setup_entries`; one past usize
/// takes the whole budget): where they all fit, every one; else, deterministically, the setups ranked last among their
/// candidate's partners are left out first (the last candidate's first, then rank by rank), until the rest fit, so a
/// scoring never fails for memory (the setups left out only lower what it credits: fewer partners lift fewer
/// candidates; counted in `sc_partner_setups_dropped`). Returns the entries the kept ones take
fn fit_partner_setups(
    simulator: &Sim,
    setups: &mut Vec<Setup>,
    leaf_count: usize,
    table_size: usize,
    factor: usize,
) -> usize {
    let entries: Vec<usize> = setups
        .iter()
        .map(|setup| {
            setup_entries(simulator, setup, leaf_count, table_size)
                .and_then(|entries| entries.checked_mul(factor))
                .unwrap_or(usize::MAX)
        })
        .collect();
    let candidates: Vec<usize> = setups.iter().map(|setup| setup.candidate).collect();
    let (kept, taken) = within_budget(&entries, &candidates, OUTCOME_BUDGET);
    let dropped = kept.iter().filter(|&&keep| !keep).count();
    if dropped > 0 {
        prof("sc_partner_setups_dropped", dropped as f64);
    }
    let mut index = 0;
    setups.retain(|_| {
        index += 1;
        kept[index - 1]
    });
    taken
}

/// `fit_partner_setups`' choice: which setups (their entries, their candidates, in `prepare_setups`' order: candidate
/// by candidate, best partner first) to keep within `budget`, and the entries the kept ones take (sums saturating)
fn within_budget(entries: &[usize], candidates: &[usize], budget: usize) -> (Vec<bool>, usize) {
    let total =
        |kept: &[bool]| (0..entries.len()).filter(|&i| kept[i]).fold(0usize, |sum, i| sum.saturating_add(entries[i]));
    let mut kept = vec![true; entries.len()];
    let mut taken = total(&kept);
    // (each setup's rank among its candidate's partners; the highest rank first, the last candidate's first)
    let rank: Vec<usize> = (0..entries.len())
        .map(|i| candidates[..i].iter().rev().take_while(|&&earlier| earlier == candidates[i]).count())
        .collect();
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by(|&first, &second| rank[second].cmp(&rank[first]).then(second.cmp(&first)));
    for index in order {
        if taken <= budget {
            break;
        }
        kept[index] = false;
        // (recomputed from the kept ones: a saturated sum can't be undone by subtraction)
        taken = total(&kept);
    }
    (kept, taken)
}

/// A setup's outcome entries: its leaf row (4^nu) and two corner tables (2^nu each) per table it reads, checked (None:
/// past usize)
pub(super) fn setup_entries(simulator: &Sim, setup: &Setup, leaf_count: usize, table_size: usize) -> Option<usize> {
    setup.tables(simulator).count().checked_mul(2 * table_size)?.checked_add(leaf_count)
}

#[derive(Clone, Copy)]
struct RankedPartner {
    value: f64,
    lineage: Lineage,
}

/// Value legal partners in pool order, then keep the first `ScoringKnobs::partners` under strict comparisons (best first).
/// val may populate the simulator's deck cache, which later candidates can read.
fn prepare_setups(
    simulator: &mut Sim,
    score_base: &ScoreBase,
    pool: &Pool,
    trainee: crate::uma::UmaIndex,
    score_buffer: &mut Vec<f64>,
) -> Vec<Setup> {
    let data = simulator.data;
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
        // her best `partners` partners by value, ties in partner order: every legal partner valued in order (each `val` as
        // before), each inserted before the first kept one it beats strictly, as the stable sort's first `partners`
        let mut partners: Vec<RankedPartner> = Vec::with_capacity(score_base.knobs.partners + 1);
        for partner in 0..new_veteran {
            let lineage = Lineage { trainee: candidate_trainee, first_parent: partner, second_parent: new_veteran };
            if !legal_pair(data, pool, lineage) {
                continue;
            }
            let value = val(simulator, pool, lineage, score_buffer);
            let place = partners.iter().position(|kept| value > kept.value).unwrap_or(partners.len());
            if place < score_base.knobs.partners {
                partners.insert(place, RankedPartner { value, lineage });
                partners.truncate(score_base.knobs.partners);
            }
        }
        for RankedPartner { lineage, .. } in partners {
            let Some(slot) = simulator.env.setup.her_slot(pool, lineage) else {
                continue;
            };
            let deck_count = simulator.evaluator.cache.decks[candidate_trainee].len();
            if simulator.evaluator.cache.decks[candidate_trainee].iter().all(|deck| deck.tables.is_empty()) {
                continue;
            }
            setups.push(Setup {
                candidate,
                trainee: candidate_trainee,
                partner: lineage.first_parent,
                misses_without_her: slot.misses_without_her,
                target_spark_scales: slot.target_spark_scales,
                miss_by_stars: slot.miss_by_stars,
                decks: 0..deck_count,
                outfit: None,
            });
        }
    }
    setups
}

pub(super) fn evaluate_career(
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
    let target_count = simulator.env.setup.target_count;
    let uncertain_target_count = simulator.env.setup.uncertain_targets.len();
    let star_mix = simulator.env.setup.skill_job.star_dist;
    let completes_goal = completes_goal(simulator, context.legacies, career.lineage);
    let current_started = now_ms();
    let current =
        simulator.evaluator.make_cur(&simulator.env.setup, &mut simulator.env.state, context.pool, career, false);
    let pool_started = now_ms();
    prof("sc_cur_ms", pool_started - current_started);
    let mut extended_pool =
        simulator.evaluator.push_her(&simulator.env.setup, context.legacies, context.pool, recorded_pool, &current);
    let new_veteran = extended_pool.pool.veterans.len() - 1;
    // where she can be a parent: per stage-1 candidate (stage 1 still to run) her best partners, one partner slot each
    let setups_started = now_ms();
    prof("sc_pool_ms", setups_started - pool_started);
    let mut setups = prepare_setups(simulator, context.score_base, &extended_pool.pool, trainee, score_buffer);
    prof("sc_setup_ms", now_ms() - setups_started);
    let full_mask = (1usize << target_count) - 1;
    let (leaf_count, table_size) = (1usize << (2 * uncertain_target_count), 1usize << uncertain_target_count);
    let builds_routes = matches!(want, Want::Score | Want::Keep | Want::Ys | Want::Classes)
        && !context.score_base.trios.is_empty()
        && context.score_base.current_value.is_finite();
    // the partner setups' entries, twice where lift setups follow (each reads at most what its partner setup does),
    // checked against the budget before anything is allocated (`fit_partner_setups`)
    let taken = fit_partner_setups(simulator, &mut setups, leaf_count, table_size, if builds_routes { 2 } else { 1 });
    let partner_slot_count = setups.len();
    // the routes through her veteran: their lift setups (slots after the partner slots) and finisher setups (after
    // those), for every scoring that reads V after her; within the memory budget (`OUTCOME_BUDGET`: the routes left
    // out past it are the ones with the highest lower bounds, `routes`)
    let (lift_slot_count, route_list) = if builds_routes {
        let routes_started = now_ms();
        let her = simulator.env.setup.veteran(context.legacies, career.lineage, &current.record);
        let entry_budget = OUTCOME_BUDGET - taken;
        let inputs = RouteInputs {
            her: &her,
            new_veteran,
            partner_setups: &setups,
            full_possible: !completes_goal && current.outcome_sets[full_mask] > 0.0,
            trainee,
            entry_budget,
            leaf_count,
            table_size,
        };
        let RouteSet { lift_setups, finisher_setups, routes: route_list, dropped } =
            routes(simulator, context, &mut extended_pool.pool, &inputs, score_buffer);
        let lift_slot_count = lift_setups.len();
        setups.extend(lift_setups);
        setups.extend(finisher_setups);
        prof("sc_route_ms", now_ms() - routes_started);
        prof("sc_routes", route_list.len() as f64);
        prof("sc_routes_dropped", dropped as f64);
        (lift_slot_count, route_list)
    } else {
        (0, vec![])
    };
    let fold_started = now_ms();
    // per setup, per star pattern of the uncertain targets (base 4, position k = uncertain_targets[k]; 0 absent), the best table value
    // at scale 1: the tables folded one coordinate at a time (interp's order and arithmetic), every setup's tables at once
    // (`FoldContext::fold`); per outcome (leaf, guaranteed golds fm) and slot, that value times the setup's spark scale,
    // the best of it and the slot's odds below (`FoldContext::emit`): the products and maxima the outcome loop took per outcome
    let fixed_target_count = simulator.env.setup.fixed_targets.len();
    let fixed_landed_masks = fm_masks(&simulator.env.setup.fixed_targets);
    // one slot per setup: a partner or lift slot's odds below are her candidate's odds now, a finisher slot's 0
    let slot_count = setups.len();
    let lifting_slots = partner_slot_count + lift_slot_count;
    let current_probabilities: Vec<f64> = setups
        .iter()
        .enumerate()
        .map(|(slot, setup)| if slot < lifting_slots { context.score_base.current_odds[setup.candidate] } else { 0.0 })
        .collect();
    // what the evaluation reads (`Scored`; the spark scales per setup and landed set m: the product the loop took per
    // outcome, the same order, `Scored::derive`)
    let mut scored = Scored {
        uncertain_target_count,
        fixed_target_count,
        target_count,
        slot_count,
        slot_candidates: setups[..partner_slot_count].iter().map(|setup| setup.candidate).collect(),
        lift_candidates: setups[partner_slot_count..lifting_slots].iter().map(|setup| setup.candidate).collect(),
        completes_goal,
        star_mix,
        uncertain_targets: simulator.env.setup.uncertain_targets.clone(),
        fixed_landed_masks,
        misses_without_her: setups.iter_mut().map(|setup| std::mem::take(&mut setup.misses_without_her)).collect(),
        miss_by_stars: setups.iter().map(|setup| setup.miss_by_stars).collect(),
        target_spark_scales: setups.iter_mut().map(|setup| std::mem::take(&mut setup.target_spark_scales)).collect(),
        setup_by_table: vec![],
        initial_tables: vec![],
        collapsed_targets: vec![true; uncertain_target_count],
        slot_by_setup: (0..slot_count).collect(),
        current_probabilities,
        landed_set_chances: scaled_landed_sets(
            current.outcome_sets,
            full_mask,
            context.score_base.odds_scales.of(trainee),
        ),
        routes: route_list,
        finisher_umas: setups[lifting_slots..].iter().map(|setup| setup.trainee).collect(),
        present_factors: vec![],
        absent_factors: vec![],
        spark_scales: vec![],
    };
    scored.derive();
    // the kept tables and, per uncertain target, collapsed (`prepare_scoring`; with no setup slot no outcome reads a table: every
    // target)
    if slot_count > 0 {
        prepare_scoring(simulator, &setups, &mut scored);
    }
    prof("sc_fold_ms", now_ms() - fold_started);
    simulator.evaluator.pop_her(context.pool, extended_pool);
    if want == Want::Touch {
        return CareerScore {
            expected_saving: 0.0,
            improvement_chance: 0.0,
            kept: None,
            lift_stats: vec![LiftStats::default(); candidate_count],
            joint_lifts: vec![],
            classes: vec![],
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
    CareerScore {
        expected_saving: evaluation.expected_saving,
        improvement_chance: evaluation.improvement_chance,
        kept: (want == Want::Keep).then_some(KeptScoring { scored }),
        lift_stats: evaluation.lift_stats,
        joint_lifts: evaluation.joint_lifts,
        classes: evaluation.classes,
    }
}

/// Her landed sets' chances as a base pricing her P(all) at `odds_scale` x the model's takes them (`ScoreBase::odds_scales`
/// at her uma,
/// the robustness check's all-target ratio variants): the full set's chance f times the scale, the mass it loses (or
/// gains) spread over the other sets in proportion to their chances, each times (1 - scale f) / (1 - f), so the sets
/// still sum to one and P(all) is scaled as every other career's. Unchanged at scale 1 (every plan scoring), or where
/// f is 0 or 1 (nothing to scale, or no other set to take the mass)
fn scaled_landed_sets(mut chances: Vec<f64>, full_mask: usize, odds_scale: f64) -> Vec<f64> {
    let full = chances[full_mask];
    if odds_scale == 1.0 || !(full > 0.0 && full < 1.0) {
        return chances;
    }
    let others_scale = (1.0 - odds_scale * full) / (1.0 - full);
    for (landed, chance) in chances.iter_mut().enumerate() {
        *chance *= if landed == full_mask { odds_scale } else { others_scale };
    }
    chances
}

impl Scored {
    /// A roll's y (`roll_answer`): the kept scoring evaluated at one outcome (her stars `leaf`, the guaranteed targets
    /// landed `fm`), `Scored::evaluate` at `OneLeaf` (the leaf and its representative); 0 where the outcome can't happen, as the
    /// plan's full table held
    pub(in crate::career::plan) fn roll(&self, score_base: &ScoreBase, outcome: OutcomeIndex) -> f64 {
        let leaf = outcome.leaf(self.fixed_target_count);
        let fixed_mask = outcome.fixed_mask(self.fixed_target_count).bits();
        let leaves = OneLeaf::new(leaf, self.representative_of(leaf));
        let mut scratch = FOLD_SCRATCH.with(|cell| std::mem::take(&mut *cell.borrow_mut()));
        let evaluation = self.evaluate(score_base, &leaves, Want::Ys, &mut scratch);
        FOLD_SCRATCH.with(|cell| *cell.borrow_mut() = scratch);
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
fn prepare_scoring(simulator: &Sim, setups: &[Setup], scored: &mut Scored) {
    let uncertain_target_count = scored.uncertain_target_count;
    let (present_factors, absent_factors) = (&scored.present_factors, &scored.absent_factors);
    // every setup's tables, level 0 of the fold (table j at j 2^nu), and the setup of each
    let (mut setup_by_table, mut initial_tables): (Vec<usize>, Vec<f64>) = (vec![], vec![]);
    let mut dominated_tables = 0usize;
    for (setup_index, setup) in setups.iter().enumerate() {
        let tables: Vec<&Vec<f64>> = setup.tables(simulator).collect();
        // a table at most another everywhere (an equal one: the later) folds to at most that one's value at every leaf
        // (every fold coefficient is >= 0 and rounding is monotone), so the maximum is the same without it
        let nonnegative_factors = present_factors[setup_index]
            .iter()
            .chain(&absent_factors[setup_index])
            .all(|star_factors| star_factors.iter().all(|&factor| factor >= 0.0));
        for (j, table) in tables.iter().enumerate() {
            debug_assert_eq!(
                table.len(),
                1usize << uncertain_target_count,
                "one table entry per subset of the uncertain targets"
            );
            if nonnegative_factors
                && tables.iter().enumerate().any(|(k, other_table)| {
                    k != j
                        && table.iter().zip(other_table.iter()).all(|(value, other_value)| value <= other_value)
                        && (k < j
                            || table.iter().zip(other_table.iter()).any(|(value, other_value)| value < other_value))
                })
            {
                dominated_tables += 1;
                continue;
            }
            initial_tables.extend_from_slice(table);
            setup_by_table.push(setup_index);
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
        for (setup_index, setup_factors) in present_factors.iter().enumerate() {
            let star_factors = &setup_factors[k];
            if star_factors[1].to_bits() == star_factors[2].to_bits()
                && star_factors[2].to_bits() == star_factors[3].to_bits()
            {
                continue;
            }
            let flat = (0..table_count).filter(|&j| setup_by_table[j] == setup_index).all(|j| {
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
    prof("sc_col", collapsed_targets.iter().filter(|&&collapsed| collapsed).count() as f64);
    prof("sc_col_tab", table_collapses as f64);
    (scored.setup_by_table, scored.initial_tables, scored.collapsed_targets) =
        (setup_by_table, initial_tables, collapsed_targets);
}

#[cfg(test)]
mod tests {
    use super::{OUTCOME_BUDGET, scaled_landed_sets};

    /// The full set's chance scaled, the other sets taking what it lost in proportion, the total still one; scale 1
    /// the same f64s
    #[test]
    fn a_scaled_full_set_moves_its_mass_to_the_others_in_proportion() {
        let chances = vec![0.1, 0.2, 0.3, 0.4];
        assert_eq!(scaled_landed_sets(chances.clone(), 3, 1.0), chances);
        let scaled = scaled_landed_sets(chances, 3, 0.75);
        assert!((scaled[3] - 0.3).abs() < 1e-15);
        assert!((scaled.iter().sum::<f64>() - 1.0).abs() < 1e-15);
        assert!((scaled[1] / scaled[0] - 2.0).abs() < 1e-12 && (scaled[2] / scaled[0] - 3.0).abs() < 1e-12);
    }

    /// At 10 uncertain targets, 8 candidates x 2 partners (16 partner setups, each with its lift setup: twice its
    /// entries) and 3 tables per setup, the job is admitted (`sim_for` reads the leaf rows alone: 2 x 2 x 8 x 4^10 =
    /// 2^25, the budget) but the setups with their corner tables pass it: the last candidates' second partners are left
    /// out until the rest fit, never the scoring refused; where all fit, all are kept
    #[test]
    fn partner_setups_past_the_budget_drop_the_lowest_ranked_first() {
        let (nu, tables) = (10, 3);
        let (leaf_count, table_size) = (1usize << (2 * nu), 1usize << nu);
        assert!((2 * 2 * crate::career::plan::search::IDX_CANDS) << (2 * nu) <= OUTCOME_BUDGET, "admitted");
        let entries = vec![2 * (leaf_count + tables * 2 * table_size); 16];
        let candidates: Vec<usize> = (0..16).map(|setup| setup / 2).collect();
        assert!(entries.iter().sum::<usize>() > OUTCOME_BUDGET);
        let (kept, taken) = super::within_budget(&entries, &candidates, OUTCOME_BUDGET);
        assert!(taken <= OUTCOME_BUDGET && taken == (0..16).filter(|&i| kept[i]).map(|i| entries[i]).sum::<usize>());
        // (one setup's 2 x 4^10 + 12 x 2^10 entries: 15 fit in 2^25 = 16 x 2 x 4^10 only with one left out)
        assert_eq!(kept.iter().filter(|&&keep| !keep).count(), 1);
        assert!(!kept[15] && kept[..15].iter().all(|&keep| keep), "the last candidate's second partner first");
        // an entry past usize (`setup_entries`' None) is left out, whatever its rank
        let mut huge = entries.clone();
        huge[0] = usize::MAX;
        let (kept, taken) = super::within_budget(&huge, &candidates, OUTCOME_BUDGET);
        assert!(!kept[0] && taken <= OUTCOME_BUDGET);
        let (kept, _) = super::within_budget(&entries[..4], &candidates[..4], OUTCOME_BUDGET);
        assert!(kept.iter().all(|&keep| keep));
    }

    /// A roll's y (`Scored::roll`: the fold at `OneLeaf`, the outcome and its representative) is the f64 the plan's
    /// full table holds at that outcome (`Scored::evaluate` at `All`, `Want::Ys`), both computed in this process from
    /// val/plan p16's kept roll state, at 512 outcomes spread over every one (the first and last among them)
    #[test]
    fn a_roll_is_the_full_tables_value_at_its_outcome() {
        use crate::career::plan::OutcomeIndex;
        use crate::career::plan::cont::Want;
        use crate::career::plan::fold::All;
        use crate::career::plan::score::kernel::FoldScratch;
        let folder = concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/");
        let read = |file: &str| std::fs::read_to_string(format!("{folder}{file}")).unwrap();
        let data = crate::data::load_str(&read("data.json"));
        let skills: crate::skills::SkillsRaw = serde_json::from_str(&read("web_skills.json")).unwrap();
        let cards: crate::deck::CardsRaw = serde_json::from_str(&read("web_cards.json")).unwrap();
        let mut job: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap(),
        )
        .unwrap();
        let events: serde_json::Value = serde_json::from_str(&read("card_events.json")).unwrap();
        crate::deck::sources::apply_events_db(&mut job, &skills, &events);
        job["no_reports"] = serde_json::json!(true);
        let mut kept = crate::career::state::KeptState::default();
        let out = crate::career::run_career_in(&data, &skills, &cards, &job, &mut kept);
        assert!(out["next"]["v0"].is_number(), "the plan ran: {}", out["next"]["error"]);
        let roll = kept.rolls.last().expect("the plan kept its pick's roll state");
        let base = roll.base.base();
        let table = roll.scored.evaluate(&base, &All, Want::Ys, &mut FoldScratch::default()).outcome_savings;
        let step = (table.len() / 512).max(1);
        let mut indices: Vec<usize> = (0..table.len()).step_by(step).collect();
        indices.push(table.len() - 1);
        for index in indices {
            let y = roll.scored.roll(&base, OutcomeIndex(index));
            assert_eq!(y.to_bits(), table[index].to_bits(), "outcome {index}: roll {y}, table {}", table[index]);
        }
    }
}
