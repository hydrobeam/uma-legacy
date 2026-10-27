//! Building a career's scoring: the places her veteran may be a parent (`prepare_setups`), the scoring over her
//! outcomes (`evaluate_career`) and its tables, collapsed per uncertain target (`prepare_scoring`). The kernels it runs
//! are in `kernel`.
use crate::career::clock::{now_ms, prof};
use crate::career::plan::cont::{ScoreBase, Want};
use crate::career::plan::fold::{All, Leaves, OneLeaf};
use crate::career::plan::score::kernel::{FOLD_SCRATCH, LiftStats, Scored, SetupParts};
use crate::career::plan::score::routes::{LiftSide, LiftSlots, PairedAt, RouteInputs, RouteSet, lift_slots, routes};
use crate::career::plan::score::{CareerScore, KeptScoring, completes_goal, fm_masks};
use crate::career::plan::search::{legal_pair, val};
use crate::career::plan::step::{LiftRead, NO_SLOT, Route, ShadowRoute, WhileBuilding};
use crate::career::plan::{OutcomeIndex, PoolProvenance, ScoringContext};
use crate::career::pool::Pool;
use crate::career::sim::Sim;
use crate::career::{Career, Lineage};
use std::rc::Rc;
/// One place a career's new veteran may be in a lineage, one slot of the scoring: a stage-1 candidate's parent beside a
/// partner (a partner setup), or in a route's C's lineage (a finisher setup, `routes`); her slot's miss and spark parts
/// there (`HerSlot`), and how many of the lineage trainee's cached decks it reads.
#[derive(Clone)]
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
    /// The odds below the setup's in its slot (`Scored::current_probabilities`), which it lifts past: a partner setup's
    /// candidate's odds now, a lift setup's the odds of the trio side it lifts (her best career's, or an Alternative
    /// side's own), a finisher setup's 0.
    pub(super) odds_below: f64,
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
    /// The most the setup's odds can be at any outcome as the fold computes them, in f64: the most any table it reads
    /// holds times the most spark scale (`conservative_odds`: each outcome's odds is a table interpolated at miss chances
    /// in [0, 1], a convex combination of its entries, times the scale of her landed set, each up to rounding); +inf
    /// where a miss factor lies outside [0, 1]
    pub(super) fn odds_bound(&self, simulator: &Sim) -> f64 {
        // (outside `conservative_odds`' domain, an x or a spark factor not in [0, 1]: no bound)
        let in_unit = |value: f64| (0.0..=1.0).contains(&value);
        if !self.misses_without_her.iter().all(|&miss| self.miss_by_stars.iter().all(|&factor| in_unit(miss * factor)))
            || !self.target_spark_scales.iter().flatten().all(|&factor| in_unit(factor))
        {
            return f64::INFINITY;
        }
        let most_table = self.tables(simulator).flatten().fold(0.0f64, |most, &value| most.max(value));
        let most_scale: f64 = self.target_spark_scales.iter().map(|scales| scales[0].max(scales[1])).product();
        conservative_odds(most_table, most_scale)
    }
}

/// An upper bound in f64 on any odds the fold computes from tables at most `most_table` and spark scales whose
/// product is at most `most_scale`: their product raised by 1e-12 of itself, plus 64 eta (eta the least subnormal).
///
/// Domain (`Setup::odds_bound` gives +inf, no bound, outside it): table entries in [0, most_table], at most 10
/// uncertain targets each with x = x0 xm in [0, 1], and at most 12 spark factors each in [0, 1] (1.1^(n - 6) for n
/// carriers of six slots, `inherit::spark_scale`), `most_scale` the product of each factor's most. Scales above 1
/// would amplify the absolute error below, and are outside it. Every f64 operation rounds to within u = 2^-53 of its
/// exact result plus eta / 2 (the latter only below the normal range, where the spacing is eta).
///
/// Error recurrence. Let V_l be a level-l value in exact arithmetic (a convex combination of entries, so V_l <= M =
/// most_table) and e_l the most its computed value exceeds it (e_0 = 0: the entries are read as stored). A level computes
/// fl(fl(a w) + fl(b x)) with w = fl(1 - x) <= (1 - x)(1 + u) (1 - x is exact for x >= 1/2 and normal otherwise), from
/// children computed at most V + e. Its weights are nonnegative with w + x <= 1 + u, and each of its three roundings
/// adds at most u of its result plus eta / 2. So the level computes at most
/// V_l (1 + u)^3 + e_(l-1) (1 + u)^3 + (3/2 + u) eta, that is
///     e_l <= ((1 + u)^3 - 1) M + (1 + u)^3 e_(l-1) + (3/2 + u) eta,
/// and a computed value is at most M + e_l, the additive term included: a flat table of 3 eta read at 0.5 computes
/// 4 eta, above (1 + u) 3 eta, within 3 eta + 3/2 eta. Over 10 levels,
///     e_10 <= ((1 + u)^30 - 1) M + (3/2 + u) eta (sum over l < 10 of (1 + u)^(3 l)) < 31 u M + 16 eta.
/// At x = 0 or 1 a level selects an endpoint exactly (w and the products exact, one term 0), adding nothing.
/// The spark scale needs no error term: `Scored::derive` multiplies a setup's actual factors in target order from 1.0,
/// as `Setup::odds_bound` multiplies their maxima, so (rounding being monotone) its computed scale is at most the
/// computed `most_scale`, itself at most 1. The odds are then fl(value x scale), at most
/// (M + e_10) most_scale (1 + u) + eta / 2 <= M most_scale (1 + 33 u) + 17 eta (a scale at most 1 cannot amplify the
/// absolute term). The bound computes fl(fl(M most_scale)(1 + 1e-12)) + 64 eta, rounded. Where that sum is
/// subnormal, every step is exact or within eta / 2, so it is at least M most_scale (1 + 1e-12) (1 - u)^2 + 62 eta.
/// Where it is normal and the 64 eta rounds away, M most_scale is near 2^-1022 or above, so its relative slack,
/// (1e-12 - 35 u) M most_scale, is above 4000 eta, more than the 17 eta it must cover. Either way it is at least the
/// odds.
///
/// So 1 / it is at most every C step the routes read, 1 / fl(odds x scale) included (`ShadowRoute`'s floor, the
/// routes' lower bounds). A flat table of 0.1 read at x = 0.2 computes 0.1 * 0.8 + 0.1 * 0.2 = 0.10000000000000002,
/// above the entry; a flat table of 3 eta read at 0.5 computes 4 eta, which the relative slack alone would miss
/// (`the_odds_bound_holds_in_f64`)
pub(super) fn conservative_odds(most_table: f64, most_scale: f64) -> f64 {
    (most_table * most_scale) * (1.0 + 1e-12) + 64.0 * f64::from_bits(1)
}

/// The outcome entries a scoring's batches may take: `OUTCOME_BUDGET` (a test lowers it on its own thread to make the
/// batches run, `with_route_budget`)
#[cfg(not(test))]
const fn route_budget() -> usize {
    OUTCOME_BUDGET
}

/// `route_budget` under test: the test's own budget where it set one (`with_route_budget`).
#[cfg(test)]
fn route_budget() -> usize {
    ROUTE_BUDGET.with(std::cell::Cell::get).unwrap_or(OUTCOME_BUDGET)
}

#[cfg(test)]
thread_local! {
    /// A test's route budget (`route_budget`).
    static ROUTE_BUDGET: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// `run` with the scorings on this thread at a route budget of `budget` entries (`route_budget`).
#[cfg(test)]
pub(crate) fn with_route_budget<T>(budget: usize, run: impl FnOnce() -> T) -> T {
    ROUTE_BUDGET.with(|cell| cell.set(Some(budget)));
    let result = run();
    ROUTE_BUDGET.with(|cell| cell.set(None));
    result
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
/// candidates; counted in `sc_partner_setups_dropped`, and the plan then says its scoring is incomplete,
/// `scoring_complete`). Returns the entries the kept ones take
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
    let ranks = pool.content_ranks();
    for candidate in 0..candidate_count {
        let candidate_trainee = score_base.candidates[candidate];
        if data.umas[candidate_trainee].id == data.umas[trainee].id
            || score_base.step_costs[candidate] == 0.0
            || !score_base.step_costs[candidate].is_finite()
        {
            continue;
        }
        // her best `partners` partners by value, ties by the partner's content rank (`Pool::content_ranks`, so not by her place
        // in the pool), then in partner order: every legal partner valued in order (each `val` as before), each inserted
        // before the first kept one it beats
        let mut partners: Vec<RankedPartner> = Vec::with_capacity(score_base.knobs.partners + 1);
        for partner in 0..new_veteran {
            let lineage = Lineage { trainee: candidate_trainee, first_parent: partner, second_parent: new_veteran };
            if !legal_pair(data, pool, lineage) {
                continue;
            }
            let value = val(simulator, pool, lineage, score_buffer);
            let rank = ranks[partner];
            let beats = |kept: &RankedPartner| {
                value > kept.value || (value == kept.value && rank < ranks[kept.lineage.first_parent])
            };
            let place = partners.iter().position(beats).unwrap_or(partners.len());
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
                odds_below: score_base.current_odds[candidate],
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
    let full_possible = !completes_goal && current.outcome_sets[full_mask] > 0.0;
    let frame = CareerFrame {
        uncertain_targets: simulator.env.setup.uncertain_targets.as_slice().into(),
        fixed_target_count: simulator.env.setup.fixed_targets.len(),
        fixed_landed_masks: fm_masks(&simulator.env.setup.fixed_targets).into(),
        target_count,
        completes_goal,
        star_mix,
        landed_set_chances: scaled_landed_sets(
            current.outcome_sets,
            full_mask,
            context.score_base.odds_scales.of(trainee),
        )
        .into(),
    };
    // the routes through her veteran: their lift setups (slots after the partner slots) and finisher setups (after
    // those), for every scoring that reads V after her; within the memory budget (`OUTCOME_BUDGET`: the routes left
    // out past it are the ones with the highest lower bounds, `routes`)
    let route_set = builds_routes.then(|| {
        let routes_started = now_ms();
        let her = simulator.env.setup.veteran(context.legacies, career.lineage, &current.record);
        let inputs = RouteInputs {
            her: &her,
            new_veteran,
            partner_setups: &setups,
            full_possible,
            trainee,
            entry_budget: route_budget().saturating_sub(taken),
            leaf_count,
            table_size,
        };
        let lifts = lift_slots(simulator, context, &extended_pool.pool, &inputs);
        prof("sc_route_wins_ms", now_ms() - routes_started);
        let reads = lift_reads(simulator, context, &frame, &lifts);
        let route_set =
            routes(simulator, context, &mut extended_pool.pool, &inputs, LiftSlots { lifts, reads }, score_buffer);
        prof("sc_route_ms", now_ms() - routes_started);
        prof("sc_routes", route_set.routes.len() as f64);
        route_set
    });
    let fold_started = now_ms();
    let mut first = route_set.as_ref().map(FirstBatch::of);
    let mut scored = if let (Some(route_set), Some(first)) = (&route_set, &mut first) {
        {
            let mut batch_setups: Vec<&Setup> = setups.iter().collect();
            batch_setups.extend(first.lifts.iter().map(|&slot| &route_set.lift_setups[slot]));
            batch_setups.extend(&route_set.finisher_setups);
            let slots = SlotCounts { partner: partner_slot_count, lift: first.lifts.len() };
            // (the routes and checks move into the scoring; the first batch keeps its lift slots and check places)
            let route_lists = RouteLists {
                routes: std::mem::take(&mut first.routes),
                shadows: std::mem::take(&mut first.shadows),
                collapse_setups: route_set.collapse_setups(),
            };
            let scoring_use = ScoringUse::of(want == Want::Keep);
            let mut scored = frame.scored(simulator, &batch_setups, slots, route_lists, scoring_use);
            scored.retain_least = !route_set.left_out.is_empty();
            scored
        }
    } else {
        let batch_setups: Vec<&Setup> = setups.iter().collect();
        let slots = SlotCounts { partner: partner_slot_count, lift: 0 };
        frame.scored(simulator, &batch_setups, slots, RouteLists::default(), ScoringUse::of(want == Want::Keep))
    };
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
    let evaluation_want = if want == Want::Keep { Want::Score } else { want };
    let mut scratch = FOLD_SCRATCH.with(|current| std::mem::take(&mut *current.borrow_mut()));
    let mut evaluation = scored.evaluate(context.score_base, &All, evaluation_want, &mut scratch);
    FOLD_SCRATCH.with(|current| *current.borrow_mut() = scratch);
    // the routes left out that may undercut V after her somewhere (their check failed, or they read a lift slot the
    // first batch does not hold): taken in later batches, each beside V after her so far (`LaterBatches`), so the
    // scoring is the one with every priced route kept
    let mut later_batches = vec![];
    if let (Some(route_set), Some(first)) = (&route_set, &first) {
        let undercut: Vec<usize> = (0..route_set.left_out.len())
            .filter(|&left| first.shadow_of_left[left].is_none_or(|shadow| evaluation.undercuts[shadow]))
            .collect();
        scored.retain_least = false;
        if !undercut.is_empty() {
            let later = LaterBatches {
                frame: &frame,
                route_set,
                collapsed_targets: &scored.collapsed_targets,
                want: evaluation_want,
                entry_budget: route_budget(),
                leaf_count,
                table_size,
            };
            let batched = later.evaluate(simulator, context.score_base, &undercut, evaluation, want == Want::Keep);
            evaluation = batched.evaluation;
            later_batches = batched.kept;
            prof("sc_routes_batched", (undercut.len() - batched.unbatched) as f64);
            // (a route no batch can hold, its lift slots and C past the whole budget: the scoring is incomplete, and
            // says so, `routes_dropped_uncertified`)
            if batched.unbatched > 0 {
                prof("sc_routes_dropped_uncertified", batched.unbatched as f64);
            }
        }
    }
    prof("sc_out", evaluation.counters.outcomes as f64);
    prof("sc_trio", evaluation.counters.changed_outcomes as f64);
    prof("sc_cost", evaluation.counters.cost_evaluations as f64);
    prof("sc_same", evaluation.counters.cache_hits as f64);
    CareerScore {
        expected_saving: evaluation.expected_saving,
        improvement_chance: evaluation.improvement_chance,
        kept: (want == Want::Keep).then_some(KeptScoring { scored, later: later_batches }),
        lift_stats: evaluation.lift_stats,
        joint_lifts: evaluation.joint_lifts,
        classes: evaluation.classes,
    }
}

/// A scoring's first batch (`evaluate_career`): its lift slots (the drafted ones it takes, in order), its routes and
/// checks with their lifts renumbered to those places, and per left-out route its check's place (None: unchecked, a lift
/// slot of it is not the first batch's)
struct FirstBatch {
    lifts: Vec<usize>,
    routes: Vec<Route>,
    shadows: Vec<ShadowRoute>,
    shadow_of_left: Vec<Option<usize>>,
}

impl FirstBatch {
    fn of(route_set: &RouteSet) -> Self {
        let lifts: Vec<usize> = (0..route_set.first_lifts.len()).filter(|&slot| route_set.first_lifts[slot]).collect();
        let mut place = vec![NO_SLOT; route_set.first_lifts.len()];
        for (index, &slot) in lifts.iter().enumerate() {
            place[slot] = index;
        }
        let renumber = |lifts: [usize; 2]| lifts.map(|slot| if slot == NO_SLOT { NO_SLOT } else { place[slot] });
        let routes = route_set.routes.iter().map(|route| Route { lifts: renumber(route.lifts), ..*route }).collect();
        let (mut shadows, mut shadow_of_left) = (vec![], vec![]);
        for shadow in &route_set.shadows {
            shadow_of_left.push(shadow.map(|shadow| {
                shadows.push(ShadowRoute { lifts: renumber(shadow.lifts), ..shadow });
                shadows.len() - 1
            }));
        }
        Self { lifts, routes, shadows, shadow_of_left }
    }
}

/// What every scoring of one career reads beside its setups and routes (`CareerFrame::scored`): the job's targets,
/// whether her parents complete the goal, the rank's star mix and her landed sets' chances as the base prices them
/// (`scaled_landed_sets`). The lists are shared (`Rc`): every scoring of the career, a lift-read chunk, a batch or a
/// kept roll's, holds the frame's one copy, none its own.
struct CareerFrame {
    /// The uncertain targets as job-target indices, ascending (`CareerSetup::uncertain_targets`).
    uncertain_targets: Rc<[usize]>,
    fixed_target_count: usize,
    /// Per mask of the guaranteed targets landed, its landed set (`fm_masks`).
    fixed_landed_masks: Rc<[crate::career::plan::TargetMask]>,
    target_count: usize,
    completes_goal: bool,
    star_mix: [f64; 3],
    landed_set_chances: Rc<[f64]>,
}

/// A scoring's routes (`CareerFrame::scored`): the routes kept, the routes the budget left out (`ShadowRoute`, each
/// checked at every outcome) and the setups left out, which its collapse reads with its own (`prepare_scoring`).
#[derive(Default)]
struct RouteLists<'setups> {
    routes: Vec<Route>,
    shadows: Vec<ShadowRoute>,
    collapse_setups: Vec<&'setups Setup>,
}

/// How a scoring's setups divide into slots (`CareerFrame::scored`): the partner slots first, then the lift slots, then
/// one finisher slot per route.
#[derive(Clone, Copy)]
struct SlotCounts {
    partner: usize,
    lift: usize,
}

/// Whether a scoring outlives its setups (`CareerFrame::scored`): kept for rolls, it holds copies of their parts
/// (`Scored::misses_without_her` and the next two, which a roll stores); evaluated once, it borrows them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScoringUse {
    Once,
    Kept,
}

impl ScoringUse {
    const fn of(kept: bool) -> Self {
        if kept { Self::Kept } else { Self::Once }
    }
}

impl CareerFrame {
    /// A scoring of `setups` (one slot each, in `slots`' order) and `routes` as `Scored::evaluate` reads it: per setup
    /// its slot's odds below (`Setup::odds_below`) and the factors derived from its miss and spark parts, borrowed
    /// (`Scored::derive_from`; a scoring kept for rolls, `ScoringUse::Kept`, holds copies of them, the one thing it
    /// keeps of the setups), and with any slot its kept tables and collapsed targets (`prepare_scoring`)
    fn scored(
        &self,
        simulator: &Sim,
        setups: &[&Setup],
        slots: SlotCounts,
        route_lists: RouteLists,
        scoring_use: ScoringUse,
    ) -> Scored {
        let RouteLists { routes, shadows, collapse_setups } = route_lists;
        // per setup, per star pattern of the uncertain targets (base 4, position k = uncertain_targets[k]; 0 absent),
        // the best table value at scale 1: the tables folded one coordinate at a time (interp's order and arithmetic),
        // every setup's tables at once (`FoldContext::fold`); per outcome (leaf, guaranteed golds fm) and slot, that
        // value times the setup's spark scale, the best of it and the slot's odds below (`FoldContext::emit`): the
        // products and maxima the outcome loop took per outcome
        let uncertain_target_count = self.uncertain_targets.len();
        let slot_count = setups.len();
        let lifting_slots = slots.partner + slots.lift;
        // what the evaluation reads (`Scored`; the spark scales per setup and landed set m: the product the loop took
        // per outcome, the same order, `Scored::derive`)
        let mut scored = Scored {
            uncertain_target_count,
            fixed_target_count: self.fixed_target_count,
            target_count: self.target_count,
            slot_count,
            slot_candidates: setups[..slots.partner].iter().map(|setup| setup.candidate).collect(),
            lift_candidates: setups[slots.partner..lifting_slots].iter().map(|setup| setup.candidate).collect(),
            completes_goal: self.completes_goal,
            star_mix: self.star_mix,
            uncertain_targets: Rc::clone(&self.uncertain_targets),
            fixed_landed_masks: Rc::clone(&self.fixed_landed_masks),
            misses_without_her: vec![],
            miss_by_stars: vec![],
            target_spark_scales: vec![],
            setup_by_table: vec![],
            initial_tables: vec![],
            collapsed_targets: vec![true; uncertain_target_count],
            slot_by_setup: (0..slot_count).collect(),
            // (one slot per setup: a partner slot's odds below are her candidate's odds now, a lift slot's its side's
            // odds, a finisher slot's 0)
            current_probabilities: setups.iter().map(|setup| setup.odds_below).collect(),
            landed_set_chances: Rc::clone(&self.landed_set_chances),
            routes,
            finisher_umas: setups[lifting_slots..].iter().map(|setup| setup.trainee).collect(),
            present_factors: vec![],
            absent_factors: vec![],
            spark_scales: vec![],
            shadows,
            prior_least: vec![],
            retain_least: false,
        };
        let parts: Vec<SetupParts> = setups
            .iter()
            .map(|setup| SetupParts {
                misses_without_her: &setup.misses_without_her,
                miss_by_stars: setup.miss_by_stars,
                target_spark_scales: &setup.target_spark_scales,
            })
            .collect();
        scored.derive_from(&parts);
        if scoring_use == ScoringUse::Kept {
            // (the copy is the point: a roll outlives the setups and stores these)
            scored.misses_without_her = setups.iter().map(|setup| setup.misses_without_her.clone()).collect();
            scored.miss_by_stars = setups.iter().map(|setup| setup.miss_by_stars).collect();
            scored.target_spark_scales = setups.iter().map(|setup| setup.target_spark_scales.clone()).collect();
        }
        // the kept tables and, per uncertain target, collapsed (`prepare_scoring`; with no setup slot no outcome reads a
        // table: every target)
        if slot_count > 0 {
            prepare_scoring(simulator, setups, &collapse_setups, &mut scored);
        }
        scored
    }
}

/// The routes the budget left out of a scoring's first batch that may undercut V after her somewhere (their check,
/// `ShadowRoute`, failed, or a lift slot of theirs is not the first batch's), taken in later batches: each batch the
/// lift slots its routes read and their finisher setups (with their paired routes'), within the budget and MAX_ROUTES,
/// its collapsed targets the first batch's (so every outcome reads the representative it did), and V after her the
/// least of its routes' and the batches' before it at every outcome (`Scored::prior_least`). V after her is a least,
/// exact in any order, so the last batch's y at every outcome, and its sums in the loop's order, are the scoring's
/// with every priced route kept
struct LaterBatches<'batch> {
    frame: &'batch CareerFrame,
    route_set: &'batch RouteSet,
    collapsed_targets: &'batch [bool],
    want: Want,
    entry_budget: usize,
    leaf_count: usize,
    table_size: usize,
}

/// `LaterBatches::evaluate`'s result: the last batch's evaluation, each batch's scoring where a kept scoring asks for
/// them (`KeptScoring::later`: a roll takes the least over the batches), and how many routes fit no batch.
struct Batched {
    evaluation: super::kernel::ScoringEvaluation,
    kept: Vec<Scored>,
    unbatched: usize,
}

/// One left-out route as a later batch takes it (`LaterBatches::unit`): its lift slots (drafted places), its routes
/// (the route, then its paired route where it has one, finisher slots from 0, lifts as drafted) and their finisher
/// setups in that order.
struct Unit<'batch> {
    lifts: Vec<usize>,
    routes: Vec<Route>,
    setups: Vec<&'batch Setup>,
}

impl<'batch> LaterBatches<'batch> {
    /// Left-out route `left` with its paired route (`PairedAt`)
    fn unit(&self, left: usize) -> Unit<'batch> {
        let route_set = self.route_set;
        let left_route = &route_set.left_out[left];
        let mut setups: Vec<&Setup> = vec![];
        let mut route = left_route.route;
        if route.finisher_slot != NO_SLOT {
            route.finisher_slot = 0;
            setups.push(&left_route.setup);
        }
        let paired = left_route.paired.map(|paired| match paired {
            PairedAt::First(first_route) => (first_route, &route_set.finisher_setups[first_route.finisher_slot]),
            PairedAt::LeftOut(other) => (route_set.left_out[other].route, &route_set.left_out[other].setup),
        });
        route.while_building = match (route.while_building, paired) {
            (WhileBuilding::Slot(_), Some(_)) => WhileBuilding::Slot(setups.len()),
            (WhileBuilding::Slot(_), None) => WhileBuilding::Skip,
            (other, _) => other,
        };
        let mut routes = vec![route];
        if let Some((paired_route, paired_setup)) = paired {
            routes.push(Route { finisher_slot: setups.len(), while_building: WhileBuilding::Skip, ..paired_route });
            setups.push(paired_setup);
        }
        let lifts = route.lifts.iter().copied().filter(|&slot| slot != NO_SLOT).collect();
        Unit { lifts, routes, setups }
    }
    /// Every batch after the first (`first`, its evaluation with V after her per outcome kept): the routes in order,
    /// as many per batch as their lift slots' and finisher setups' entries fit, at most MAX_ROUTES; the last batch's
    /// evaluation, and with `keep` each batch's scoring
    fn evaluate(
        &self,
        simulator: &Sim,
        score_base: &ScoreBase,
        undercut: &[usize],
        first: super::kernel::ScoringEvaluation,
        keep: bool,
    ) -> Batched {
        let entries =
            |setup: &Setup| setup_entries(simulator, setup, self.leaf_count, self.table_size).unwrap_or(usize::MAX);
        let lift_setups = &self.route_set.lift_setups;
        // (the batches: consecutive routes while their setups fit; one whose setups alone do not fit is left out)
        let (mut batches, mut unbatched): (Vec<Vec<Unit>>, usize) = (vec![], 0);
        let (mut current, mut current_lifts, mut used, mut routes): (Vec<Unit>, Vec<usize>, usize, usize) =
            (vec![], vec![], 0, 0);
        for &left in undercut {
            let unit = self.unit(left);
            let unit_size = |held: &[usize]| {
                let lifts = unit.lifts.iter().filter(|slot| !held.contains(slot));
                let finishers = unit.setups.iter().map(|setup| entries(setup));
                lifts.map(|&slot| entries(&lift_setups[slot])).chain(finishers).fold(0usize, usize::saturating_add)
            };
            if unit_size(&[]) > self.entry_budget || unit.routes.len() > crate::career::plan::step::MAX_ROUTES {
                unbatched += 1;
                continue;
            }
            let added = unit_size(&current_lifts);
            if used.saturating_add(added) > self.entry_budget
                || routes + unit.routes.len() > crate::career::plan::step::MAX_ROUTES
            {
                batches.push(std::mem::take(&mut current));
                (current_lifts, used, routes) = (vec![], 0, 0);
            }
            let added = unit_size(&current_lifts);
            used = used.saturating_add(added);
            routes += unit.routes.len();
            current_lifts
                .extend(unit.lifts.iter().filter(|slot| !current_lifts.contains(slot)).copied().collect::<Vec<_>>());
            current.push(unit);
        }
        if !current.is_empty() {
            batches.push(current);
        }
        let (mut evaluation, mut kept) = (first, vec![]);
        let batch_count = batches.len();
        for (batch_index, batch) in batches.into_iter().enumerate() {
            // (the batch's lift slots in order of first use, its routes' lifts and finisher slots renumbered)
            let mut lifts: Vec<usize> = vec![];
            for unit in &batch {
                for &slot in &unit.lifts {
                    if !lifts.contains(&slot) {
                        lifts.push(slot);
                    }
                }
            }
            let mut batch_setups: Vec<&Setup> = lifts.iter().map(|&slot| &lift_setups[slot]).collect();
            let mut batch_routes = vec![];
            for unit in &batch {
                let offset = batch_setups.len() - lifts.len();
                let place = |slot: usize| {
                    if slot == NO_SLOT {
                        NO_SLOT
                    } else {
                        lifts.iter().position(|&held| held == slot).expect("a held lift")
                    }
                };
                for route in &unit.routes {
                    let shift = |slot: usize| if slot == NO_SLOT { NO_SLOT } else { slot + offset };
                    let while_building = match route.while_building {
                        WhileBuilding::Slot(slot) => WhileBuilding::Slot(slot + offset),
                        other => other,
                    };
                    batch_routes.push(Route {
                        lifts: route.lifts.map(place),
                        finisher_slot: shift(route.finisher_slot),
                        while_building,
                        ..*route
                    });
                }
                batch_setups.extend(&unit.setups);
            }
            let slots = SlotCounts { partner: 0, lift: lifts.len() };
            let route_lists = RouteLists { routes: batch_routes, ..RouteLists::default() };
            let mut scored = self.frame.scored(simulator, &batch_setups, slots, route_lists, ScoringUse::of(keep));
            scored.collapsed_targets = self.collapsed_targets.to_vec();
            scored.prior_least = std::mem::take(&mut evaluation.outcome_least);
            scored.retain_least = batch_index + 1 < batch_count;
            let mut scratch = FOLD_SCRATCH.with(|current| std::mem::take(&mut *current.borrow_mut()));
            evaluation = scored.evaluate(score_base, &All, self.want, &mut scratch);
            FOLD_SCRATCH.with(|current| *current.borrow_mut() = scratch);
            if keep {
                (scored.prior_least, scored.retain_least) = (vec![], false);
                kept.push(scored);
            }
        }
        Batched { evaluation, kept, unbatched }
    }
}

/// Per lift slot of `lifts`, what V after her reads of it at the outcomes of this career (`LiftRead`: the least step,
/// whether any outcome leaves her grinding at once after it): the lift slots alone scored, with no route
/// (`Want::Score`: V after her costs every lifted slot's step at every outcome, `StepWorkspace::lift_reads`), its
/// targets collapsed only where that is exact, so every leaf's odds are the full fold's bit for bit (`prepare_scoring`):
/// every odds the full scoring reads of a lift slot, at a leaf or at its representative, is read here too (each setup's
/// tables fold alone, the same f64s), so its least step is at most the full scoring's and any step there that grinds
/// grinds here. The routes' lower bounds and dominance read them (`routes`)
fn lift_reads(simulator: &Sim, context: &ScoringContext, frame: &CareerFrame, lifts: &[LiftSide]) -> Vec<LiftRead> {
    if lifts.is_empty() {
        return vec![];
    }
    let started = now_ms();
    // (in chunks of lift slots within the route budget: each slot's reads are its own, so chunks change no read)
    let leaf_count = 1usize << (2 * frame.uncertain_targets.len());
    let table_size = 1usize << frame.uncertain_targets.len();
    let mut reads = Vec::with_capacity(lifts.len());
    let mut start = 0;
    while start < lifts.len() {
        let mut used = 0usize;
        let mut end = start;
        while end < lifts.len() {
            let entries = setup_entries(simulator, &lifts[end].setup, leaf_count, table_size).unwrap_or(usize::MAX);
            if end > start && used.saturating_add(entries) > route_budget() {
                break;
            }
            used = used.saturating_add(entries);
            end += 1;
        }
        let setups: Vec<&Setup> = lifts[start..end].iter().map(|lift| &lift.setup).collect();
        let slots = SlotCounts { partner: 0, lift: setups.len() };
        let scored = frame.scored(simulator, &setups, slots, RouteLists::default(), ScoringUse::Once);
        reads.extend(reads_of(scored, context.score_base));
        start = end;
    }
    prof("sc_lift_steps_ms", now_ms() - started);
    reads
}

/// `lift_reads` of a scoring of the lift slots alone: its targets collapsed only where every setup's x is the same f64
/// at 1, 2 and 3 stars (the fold then takes the same f64s at those levels, so every leaf's odds are the full fold's bit
/// for bit; not where the tables are flat, which the full scoring may read at a representative only up to rounding),
/// evaluated at every outcome whose landed set can happen (`Want::LiftReads`)
fn reads_of(mut scored: Scored, score_base: &ScoreBase) -> Vec<LiftRead> {
    for (k, collapsed) in scored.collapsed_targets.iter_mut().enumerate() {
        *collapsed = scored.present_factors.iter().all(|setup_factors| {
            let star_factors = setup_factors[k];
            star_factors[1].to_bits() == star_factors[2].to_bits()
                && star_factors[2].to_bits() == star_factors[3].to_bits()
        });
    }
    let mut scratch = FOLD_SCRATCH.with(|current| std::mem::take(&mut *current.borrow_mut()));
    let evaluation = scored.evaluate(score_base, &All, Want::LiftReads, &mut scratch);
    FOLD_SCRATCH.with(|current| *current.borrow_mut() = scratch);
    evaluation.lift_reads
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
fn prepare_scoring(simulator: &Sim, setups: &[&Setup], collapse_setups: &[&Setup], scored: &mut Scored) {
    let uncertain_target_count = scored.uncertain_target_count;
    // every setup's tables, level 0 of the fold (table j at j 2^nu), and the setup of each
    let (mut setup_by_table, mut initial_tables): (Vec<usize>, Vec<f64>) = (vec![], vec![]);
    let mut dominated_tables = 0usize;
    for (setup_index, setup) in setups.iter().enumerate() {
        let factors =
            SetupFactors { present: &scored.present_factors[setup_index], absent: &scored.absent_factors[setup_index] };
        let (kept, dominated) = kept_tables(simulator, setup, factors);
        dominated_tables += dominated;
        for table in kept {
            initial_tables.extend_from_slice(table);
            setup_by_table.push(setup_index);
        }
    }
    prof("sc_dom", dominated_tables as f64);
    let table_count = setup_by_table.len();
    // the collapse (above): per target, every setup's x the same at 1-3 stars, or her tables the same either side of bit
    // k; the collapse-only setups (the routes and lift slots left out, `RouteSet::collapse_setups`) the same, so the
    // representatives read are those of the scoring with them kept
    let table_size = 1usize << uncertain_target_count;
    let left_out: Vec<CollapseOnly> = collapse_setups
        .iter()
        .map(|setup| {
            let present: Vec<[f64; 4]> = (0..uncertain_target_count)
                .map(|k| {
                    [0usize, 1, 2, 3].map(|her_stars| setup.misses_without_her[k] * setup.miss_by_stars[her_stars])
                })
                .collect();
            let absent: Vec<[f64; 4]> = present.iter().map(|star_factors| star_factors.map(|x| 1.0 - x)).collect();
            let (kept, _) = kept_tables(simulator, setup, SetupFactors { present: &present, absent: &absent });
            CollapseOnly { present, tables: kept }
        })
        .collect();
    let mut collapsed_targets = vec![true; uncertain_target_count];
    let mut table_collapses = 0usize;
    for (k, collapsed) in collapsed_targets.iter_mut().enumerate() {
        let mut collapsed_by_table = false;
        let scored_setups = scored.present_factors.iter().enumerate().map(|(setup_index, setup_factors)| {
            let tables: Vec<&[f64]> = (0..table_count)
                .filter(|&j| setup_by_table[j] == setup_index)
                .map(|j| &initial_tables[j * table_size..(j + 1) * table_size])
                .collect();
            collapse_at(k, setup_factors[k], &tables)
        });
        let left_out_setups = left_out.iter().map(|setup| {
            let tables: Vec<&[f64]> = setup.tables.iter().map(|table| table.as_slice()).collect();
            collapse_at(k, setup.present[k], &tables)
        });
        for permit in scored_setups.chain(left_out_setups) {
            match permit {
                Collapse::Exact => {}
                Collapse::ByTable => collapsed_by_table = true,
                Collapse::No => {
                    *collapsed = false;
                    break;
                }
            }
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

/// A collapse-only setup as `prepare_scoring` reads it: its x per uncertain target and star level, its kept tables.
struct CollapseOnly<'cache> {
    present: Vec<[f64; 4]>,
    tables: Vec<&'cache Vec<f64>>,
}

/// A setup's x = x0 xm and 1 - x per uncertain target and star level (`Scored::derive`'s).
#[derive(Clone, Copy)]
struct SetupFactors<'factors> {
    present: &'factors [[f64; 4]],
    absent: &'factors [[f64; 4]],
}

/// A setup's tables less the dominated ones, and how many those are: a table at most another everywhere (an equal one:
/// the later) folds to at most that one's value at every leaf (every fold coefficient is >= 0 and rounding is
/// monotone), so the maximum is the same without it
fn kept_tables<'cache>(simulator: &'cache Sim, setup: &Setup, factors: SetupFactors) -> (Vec<&'cache Vec<f64>>, usize) {
    let tables: Vec<&Vec<f64>> = setup.tables(simulator).collect();
    let nonnegative_factors = factors
        .present
        .iter()
        .chain(factors.absent)
        .all(|star_factors| star_factors.iter().all(|&factor| factor >= 0.0));
    let (mut kept, mut dominated) = (vec![], 0usize);
    for (j, table) in tables.iter().enumerate() {
        debug_assert_eq!(
            table.len(),
            1usize << factors.present.len(),
            "one table entry per subset of the uncertain targets"
        );
        if nonnegative_factors
            && tables.iter().enumerate().any(|(k, other_table)| {
                k != j
                    && table.iter().zip(other_table.iter()).all(|(value, other_value)| value <= other_value)
                    && (k < j || table.iter().zip(other_table.iter()).any(|(value, other_value)| value < other_value))
            })
        {
            dominated += 1;
            continue;
        }
        kept.push(*table);
    }
    (kept, dominated)
}

/// How one setup lets target k collapse (`prepare_scoring`).
enum Collapse {
    /// Its x is the same f64 at 1, 2 and 3 stars: the fold takes the same f64s there.
    Exact,
    /// Every kept table of it reads the same with target k landed and not.
    ByTable,
    No,
}

/// Setup's collapse of target k from its x at each star level and its kept tables
fn collapse_at(k: usize, star_factors: [f64; 4], tables: &[&[f64]]) -> Collapse {
    if star_factors[1].to_bits() == star_factors[2].to_bits() && star_factors[2].to_bits() == star_factors[3].to_bits()
    {
        return Collapse::Exact;
    }
    let flat = tables.iter().all(|t| (0..t.len()).all(|i| i >> k & 1 == 1 || t[i] == t[i | (1 << k)]));
    if flat { Collapse::ByTable } else { Collapse::No }
}

#[cfg(test)]
mod tests {
    use super::{OUTCOME_BUDGET, scaled_landed_sets};

    /// Astra (astra-iv2.out 2): the odds bound holds in f64. A flat table of 0.1 read at a miss chance of 0.2 computes
    /// 0.1 * 0.8 + 0.1 * 0.2 = 0.10000000000000002, above every entry, so 1 / 0.1 = 10 is no floor on that C's step
    /// (9.999999999999998); the bound (`conservative_odds`) is at least the fold's value, and its inverse at most the step
    #[test]
    fn the_odds_bound_holds_in_f64() {
        use crate::career::plan::score::kernel::Scored;
        let mut scored = Scored {
            uncertain_target_count: 1,
            fixed_target_count: 0,
            target_count: 1,
            slot_count: 1,
            slot_candidates: vec![],
            lift_candidates: vec![],
            completes_goal: false,
            star_mix: [0.5, 0.45, 0.05],
            uncertain_targets: vec![0].into(),
            fixed_landed_masks: crate::career::plan::score::fm_masks(&[]).into(),
            misses_without_her: vec![vec![0.2]],
            miss_by_stars: vec![[1.0; 4]],
            target_spark_scales: vec![vec![[1.0, 1.0]]],
            setup_by_table: vec![0],
            initial_tables: vec![0.1, 0.1],
            collapsed_targets: vec![false],
            slot_by_setup: vec![0],
            current_probabilities: vec![0.0],
            landed_set_chances: vec![0.5, 0.5].into(),
            routes: vec![],
            finisher_umas: vec![],
            present_factors: vec![],
            absent_factors: vec![],
            spark_scales: vec![],
            shadows: vec![],
            prior_least: vec![],
            retain_least: false,
        };
        scored.derive();
        let odds = scored.setup_odds_at(0, 0, 0);
        assert_eq!(0.1f64 * 0.8 + 0.1 * 0.2, 0.100_000_000_000_000_02, "Astra's arithmetic");
        assert!(odds > 0.1, "the fold's value passes the entry: {odds}");
        let bound = super::conservative_odds(0.1, 1.0);
        assert!(bound >= odds && 1.0 / bound <= 1.0 / odds, "bound {bound}, odds {odds}");
        // (below the normal range: a flat table of 3 eta read at 0.5 computes 4 eta, and 3 eta (1 + 1e-12) = 3 eta)
        let eta = f64::from_bits(1);
        let mut subnormal = scored_of(vec![0.5], vec![3.0 * eta, 3.0 * eta]);
        subnormal.derive();
        let odds = subnormal.setup_odds_at(0, 0, 0);
        assert_eq!(odds, 4.0 * eta, "the fold rounds up past the entry");
        assert_eq!(3.0 * eta * (1.0 + 1e-12), 3.0 * eta, "a relative slack alone adds nothing there");
        assert!(super::conservative_odds(3.0 * eta, 1.0) >= odds);
    }

    /// A one-target, one-setup scoring of a table (`initial_tables`, missed then landed) with her miss product without
    /// her `misses` (her stars' factor 1 at every level)
    fn scored_of(misses: Vec<f64>, table: Vec<f64>) -> crate::career::plan::score::kernel::Scored {
        crate::career::plan::score::kernel::Scored {
            uncertain_target_count: 1,
            fixed_target_count: 0,
            target_count: 1,
            slot_count: 1,
            slot_candidates: vec![],
            lift_candidates: vec![],
            completes_goal: false,
            star_mix: [0.5, 0.45, 0.05],
            uncertain_targets: vec![0].into(),
            fixed_landed_masks: crate::career::plan::score::fm_masks(&[]).into(),
            misses_without_her: vec![misses],
            miss_by_stars: vec![[1.0; 4]],
            target_spark_scales: vec![vec![[1.0, 1.0]]],
            setup_by_table: vec![0],
            initial_tables: table,
            collapsed_targets: vec![false],
            slot_by_setup: vec![0],
            current_probabilities: vec![0.0],
            landed_set_chances: vec![0.5, 0.5].into(),
            routes: vec![],
            finisher_umas: vec![],
            present_factors: vec![],
            absent_factors: vec![],
            spark_scales: vec![],
            shadows: vec![],
            prior_least: vec![],
            retain_least: false,
        }
    }

    /// The batches (Astra, astra-iv2.out 2): the val/plan p16 plan with the route budget cut to 2^15 entries (the first
    /// batch holds few lift slots and finisher setups, so most routes are left out, many unchecked, and run in later
    /// batches; the lift reads run in chunks) gives every row's score, V and the pick to the bit, and the pick's kept
    /// roll state answers every outcome (512 spread over all) with the same y as the plan run at the full budget: the
    /// rolls read the later batches too
    #[test]
    fn the_batches_change_no_score_and_no_roll() {
        use crate::career::plan::OutcomeIndex;
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
        let plan = |kept: &mut crate::career::state::KeptState| {
            crate::career::run_career_in(&data, &skills, &cards, &job, kept)
        };
        let mut full_kept = crate::career::state::KeptState::default();
        let full = plan(&mut full_kept);
        let mut cut_kept = crate::career::state::KeptState::default();
        let cut = super::with_route_budget(1 << 15, || plan(&mut cut_kept));
        let (full, cut) = (&full["next"], &cut["next"]);
        assert!(cut["work"]["prof"]["sc_routes_batched"].as_f64().unwrap_or(0.0) > 0.0, "the batches ran");
        assert_eq!(cut["routes_dropped_uncertified"], 0.0);
        assert_eq!(full["v0"].as_f64().unwrap().to_bits(), cut["v0"].as_f64().unwrap().to_bits());
        assert_eq!(full["choice"]["career"], cut["choice"]["career"]);
        let scores = |plan: &serde_json::Value| -> Vec<(String, u64)> {
            plan["options"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|row| Some((row["career"].as_str()?.to_owned(), row["score"].as_f64()?.to_bits())))
                .collect()
        };
        assert_eq!(scores(full), scores(cut), "every row's score");
        let (full_roll, cut_roll) = (full_kept.rolls.last().unwrap(), cut_kept.rolls.last().unwrap());
        assert!(!cut_roll.later.is_empty(), "the pick's kept scoring holds later batches");
        let base = full_roll.base.base();
        let outcome_count =
            1usize << (2 * full_roll.scored.uncertain_target_count + full_roll.scored.fixed_target_count);
        for index in (0..outcome_count).step_by((outcome_count / 512).max(1)).chain([outcome_count - 1]) {
            let (full_y, cut_y) =
                (full_roll.roll(&base, OutcomeIndex(index)), cut_roll.roll(&base, OutcomeIndex(index)));
            assert_eq!(full_y.to_bits(), cut_y.to_bits(), "outcome {index}: {full_y} at the full budget, {cut_y} cut");
        }
    }

    /// The lift reads cover every outcome whose landed set can happen, whatever her stars' own chance (Astra,
    /// astra-iv.out 1): one uncertain target, a lift slot whose odds rise with her stars, the rank's mix [0, 1, 0] (legal:
    /// skills.rs), so she lands it at 2 stars only. The full scoring may still read a 1- or 3-star leaf, as the
    /// representative of a collapsed target; the reads take every leaf's step, so the least is the 3-star leaf's,
    /// below the one the possible outcomes alone give (`Want::Score`, what the reads were before: they then said the
    /// slot's least step was 1 / q2 where the scoring can read 1 / q3, and a route through it could be pruned wrongly)
    #[test]
    fn the_lift_reads_take_the_impossible_leaves_too() {
        use crate::career::plan::cont::{OddsScales, ScoreBase, ScoringKnobs, Want};
        use crate::career::plan::fold::All;
        use crate::career::plan::score::kernel::{FoldScratch, Scored};
        let (table_missed, table_landed, base_miss) = (0.001, 0.004, 0.9);
        let mut scored = Scored {
            uncertain_target_count: 1,
            fixed_target_count: 0,
            target_count: 1,
            slot_count: 1,
            slot_candidates: vec![],
            lift_candidates: vec![0],
            completes_goal: false,
            star_mix: [0.0, 1.0, 0.0],
            uncertain_targets: vec![0].into(),
            fixed_landed_masks: crate::career::plan::score::fm_masks(&[]).into(),
            misses_without_her: vec![vec![base_miss]],
            miss_by_stars: vec![[1.0, 0.8, 0.5, 0.2]],
            target_spark_scales: vec![vec![[1.0, 1.0]]],
            setup_by_table: vec![0],
            initial_tables: vec![table_landed, table_missed],
            collapsed_targets: vec![false],
            slot_by_setup: vec![0],
            current_probabilities: vec![table_missed * 1.01],
            landed_set_chances: vec![0.5, 0.5].into(),
            routes: vec![],
            finisher_umas: vec![],
            present_factors: vec![],
            absent_factors: vec![],
            spark_scales: vec![],
            shadows: vec![],
            prior_least: vec![],
            retain_least: false,
        };
        scored.derive();
        let leaf_odds: Vec<f64> = (0..4).map(|leaf| scored.setup_odds_at(0, leaf, 1)).collect();
        let odds = |leaf: usize| leaf_odds[leaf];
        scored.current_probabilities = vec![odds(0) * (1.0 + 1e-6)];
        assert!(
            odds(3) > odds(2) && odds(2) > odds(1),
            "her odds rise with her stars: {} {} {}",
            odds(1),
            odds(2),
            odds(3)
        );
        let base = ScoreBase {
            candidates: &[crate::uma::UmaIndex(0)],
            step_costs: &[1000.0],
            current_value: 2000.0,
            current_odds: &[table_missed],
            continuations: &[],
            step_caps: &[],
            trios: &[],
            trio_sides: &[],
            finisher_umas: &[],
            knobs: ScoringKnobs::default(),
            odds_scales: OddsScales::ONE,
        };
        // (no continuation: she grinds, her step 1 / odds, cheapest at the highest odds)
        let possible_only = scored.evaluate(&base, &All, Want::Score, &mut FoldScratch::default()).lift_reads[0];
        let reads = super::reads_of(scored, &base)[0];
        assert_eq!(possible_only.least_step.to_bits(), (1.0 / odds(2)).to_bits(), "the possible outcomes: 2 stars");
        assert!((reads.least_step - 1.0 / odds(3)).abs() < 1e-9 * reads.least_step, "every leaf: 3 stars");
        assert!(reads.least_step < possible_only.least_step);
    }

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
