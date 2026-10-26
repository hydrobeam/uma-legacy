//! The surrogate's career valuation, shared by the policy simulator (`sim`) and the plan (`plan`): per trainee her
//! cached decks (`Cache`), the tables built once per job for the allocation-free ceiling, the win-chance memo, the
//! work counters and the planner's working copy of its pool (docs/SOLVER.md "Policy simulator").
//!
//! The simulator owns one `Evaluator` beside its `&mut Env` and its policies borrow both: a valuation that searches for
//! decks takes `&mut Env` (the search state, its audit and memo), one that only folds a trainee's chances in takes the
//! setup and `&mut SearchState`, one that only reads takes `&CareerSetup`. The evaluator never sees a path's state:
//! callers pass the pool and the bar, and keep the best pick themselves (`Sim::consider`).
use crate::career::cache::Cache;
use crate::career::inherit::{ExactOptions, Inheritance, RelOf, SlotSink, aff_factor, inherit, insp_miss, spark_scale};
use crate::career::outcomes::interp_in;
use crate::career::pool::{Pool, Who};
use crate::career::schedule::{ExpectedRecord, ScheduleKey, ScheduleOutfits};
use crate::career::search::{Bar, DeckRes, Ref};
use crate::career::state::ProductProof;
use crate::career::{Career, CareerSetup, Env, Legacy, Lineage, SearchState};
use crate::deck::space::SelectedDeck;
use crate::skills::GEN_GOLD;
use crate::uma::{UmaIndex, UmaTable};
use std::rc::Rc;

/// The best career so far by the surrogate (`Evaluator::consider`).
#[derive(Clone, Copy)]
pub(super) struct SurrogatePick {
    /// Spark scale C times her best cached deck's table value.
    pub(super) value: f64,
    /// Trainee and parents, first parent the lower legacy index.
    pub(super) lineage: Lineage,
    /// The deck's index among her cached decks (`Cache::decks`).
    pub(super) deck: usize,
}
/// A career to run, with its exact numbers
pub(crate) struct Cur {
    pub(super) lineage: Lineage,
    pub(super) p_all: f64,
    /// Chance of each landed set (bit i = job target i).
    pub(super) outcome_sets: Vec<f64>,
    /// What her schedule and aptitudes leave her veteran: her win chance per race, her possible pinks, the outfit
    /// her schedule runs.
    pub(super) record: ExpectedRecord,
    /// A loop rotation career: her veteran may replace her member's rotation veteran (`rot_update`).
    pub(super) rotation: bool,
}
/// The evaluator's work counters (`Evaluator::counters`).
#[derive(Default, Clone, Copy)]
pub(super) struct Counters {
    /// Candidate careers valued.
    pub(super) candidates: usize,
    /// Candidates whose ceiling cleared the bar.
    pub(super) ceiling_passes: usize,
    /// Cached-table valuations (`deck_for`).
    pub(super) tables: usize,
    /// Deck searches ("refreshes").
    pub(super) searches: usize,
    /// Deck searches that did not finish (no `Ref`).
    pub(super) partial_searches: usize,
}
/// A trainee's inheritance chances and the bounds for choosing her cached or searched deck.
#[derive(Clone, Copy)]
pub(super) struct DeckRequest<'chances> {
    /// Trainee's position in Data::umas.
    pub(super) trainee: UmaIndex,
    /// Inheritance probability per job target.
    pub(super) inheritance_chances: &'chances [f64],
    /// Product of inheritance spark multipliers.
    pub(super) spark_scale: f64,
    /// Upper bound on this lineage's success probability.
    pub(super) ceiling: f64,
    /// Minimum success probability worth searching.
    pub(super) threshold: f64,
}
/// Which pool a working copy was made from (`Evaluator::take_pool`): its address, length and borrowing rule.
#[derive(Clone, Copy, PartialEq, Eq)]
struct PoolIdentity {
    address: usize,
    veterans: usize,
    max_borrowed: u8,
}
impl PoolIdentity {
    fn of(pool: &Pool) -> Self {
        Self { address: pool as *const Pool as usize, veterans: pool.veterans.len(), max_borrowed: pool.max_borrowed }
    }
}
/// The pool with her veteran as its last entry (`Evaluator::push_her`), given back with `Evaluator::pop_her`.
#[must_use = "give it back with `Evaluator::pop_her`"]
pub(super) struct HerPool {
    pub(super) pool: Pool,
    /// The evaluator's working copy lent out with her entry pushed (`pop_her` pops it and keeps the copy), rather than
    /// a pool built for her.
    lent: bool,
}
/// What a career's schedule leaves her veteran (`ExpectedRecord`), per what the schedule reads (`ScheduleKey`: the
/// trainee, her outfits' grades, the race weights' bits, her legacies' pink inputs) (`Evaluator::wins`).
#[derive(Default)]
pub(super) struct WinChances(crate::FxMap<ScheduleKey, ExpectedRecord>);
impl WinChances {
    /// her schedule's record in the best of `outfits` for it (`CareerSetup::plan_schedule`), cached by what it reads
    fn of(&mut self, setup: &CareerSetup, pool: &Pool, lineage: Lineage, outfits: ScheduleOutfits) -> ExpectedRecord {
        let key = setup.schedule_key(pool, lineage, outfits);
        if let Some(cached) = self.0.get(&key) {
            return cached.clone();
        }
        let planned = setup.plan_schedule(pool, lineage, outfits);
        let record = ExpectedRecord {
            pink_chances: Rc::new(planned.pink.as_ref().map_or_else(Vec::new, CareerSetup::pink_chances)),
            outfit: planned.outfit.card,
            win_chances: Rc::new(planned.win_chances),
        };
        self.0.insert(key, record.clone());
        record
    }
    /// Schedules computed so far.
    pub(super) fn schedules(&self) -> usize {
        self.0.len()
    }
}

/// The surrogate's valuation state (module notes): built once per job by `Evaluator::new`, owned by the simulator.
pub(crate) struct Evaluator {
    /// Per trainee, her cached decks with their corner tables and her fully checked decks.
    pub(super) cache: Cache,
    /// The refresh margin m (sim module notes); 0 = refresh whenever a bound exceeds the table value.
    pub(super) margin: f64,
    /// the plan only (false for every policy of the simulator): a trainee without a cached deck gets her best deck
    /// searched without a bar (so she has one from then on), instead of a search at the bar that caches nothing when it
    /// ends Bounded
    fill_missing_decks: bool,
    /// per trainee and outfit, per uncertain target (A, B): her ceiling factor is A - B (1 - q) (`fast_ub`)
    pub(super) ceiling_factors: UmaTable<Vec<Vec<(f64, f64)>>>,
    /// The model's fixed gold factor (`Model::fixed_factor`).
    pub(super) fixed: f64,
    pub(super) win_chances: WinChances,
    pub(super) counters: Counters,
    /// a lineage's chances, reused (`CareerSetup::lineage_into`); empty between uses
    pub(super) inheritance_buffer: Vec<f64>,
    /// the planner's working copy of its pool (`take_pool`), with the identity of the pool it copies
    working_pool: Option<(PoolIdentity, Pool)>,
}

/// Per trainee and outfit, per uncertain target (A, B): her ceiling factor is A - B (1 - q)
/// (`Evaluator::ceiling_factors`; none without legal deck odds). `outfit_fast` at q = 0 on the hint chances
/// `set_trainee` would write, split: A = dg GOLD + (1 - dg) WHITE and B = (1 - dg) WHITE (1 - dw)(1 - e) where she
/// lacks the skill, B = 0 where she has it
fn ceiling_factors(setup: &CareerSetup) -> UmaTable<Vec<Vec<(f64, f64)>>> {
    setup
        .data
        .umas
        .indices()
        .map(|trainee| match &setup.deck_odds_ceilings[trainee] {
            None => vec![],
            Some((white_hint_chances, gold_hint_chances)) => setup.outfit_coverage[trainee]
                .iter()
                .zip(&setup.model.event_chances[trainee])
                .map(|(coverage, event_chances)| {
                    (0..setup.uncertain_count)
                        .map(|k| match coverage[k] {
                            2 => (GEN_GOLD, 0.0),
                            1 => (
                                gold_hint_chances[k] * GEN_GOLD
                                    + (1.0 - gold_hint_chances[k]) * crate::skills::GEN_WHITE,
                                0.0,
                            ),
                            _ => (
                                gold_hint_chances[k] * GEN_GOLD
                                    + (1.0 - gold_hint_chances[k]) * crate::skills::GEN_WHITE,
                                (1.0 - gold_hint_chances[k])
                                    * crate::skills::GEN_WHITE
                                    * (1.0 - white_hint_chances[k])
                                    * (1.0 - event_chances[k]),
                            ),
                        })
                        .collect()
                })
                .collect(),
        })
        .collect()
}

/// The career with its deck: its exact numbers; `rotation`: a loop rotation career (`Cur::rotation`). Her schedule's
/// record, in the outfit option her exact odds pick, comes from (and fills) `win_chances`.
fn career_numbers(
    setup: &CareerSetup,
    state: &mut SearchState,
    win_chances: &mut WinChances,
    pool: &Pool,
    career: Career,
    rotation: bool,
) -> Cur {
    let lineage = career.lineage;
    let trainee = lineage.trainee;
    let Inheritance { chances: q, carriers } = inherit(&setup.slots(pool, lineage), setup.target_count);
    setup.set_trainee(state, trainee, &q);
    let exact =
        setup.exact(state, trainee, career.deck, &carriers, ExactOptions { all_but_one: true, landed_sets: true });
    let outcome_sets = setup.outcome_sets(exact.outfit, &exact.extras);
    let record = win_chances.of(setup, pool, lineage, ScheduleOutfits::DeckOption(exact.outfit));
    Cur { lineage, p_all: exact.p_all, outcome_sets, record, rotation }
}

impl Evaluator {
    /// The job's evaluator: an empty deck cache, `ceiling_factors` and the fixed gold factor
    pub(super) fn new(setup: &CareerSetup, margin: f64, fill_missing_decks: bool) -> Self {
        Self {
            cache: Cache::new(setup.data.umas.len()),
            margin,
            fill_missing_decks,
            ceiling_factors: ceiling_factors(setup),
            fixed: setup.model.fixed_factor(),
            win_chances: WinChances::default(),
            counters: Counters::default(),
            inheritance_buffer: vec![],
            working_pool: None,
        }
    }
    /// A working copy of `pool` (the planner's `CareerSetup::pool(legacies)`), equal to it: the one kept by `put_pool` when
    /// it was made from this pool (the same address, length and borrowing rule: the pool a caller passes is borrowed,
    /// so unchanged, or, as the simulator's, only grown), else a clone. A user pushes its extra entries
    /// (`CareerSetup::pool_push` builds an entry as `CareerSetup::pool` does, so the pool of legacies and those) and pops
    /// them before `put_pool`
    pub(super) fn take_pool(&mut self, pool: &Pool) -> Pool {
        match self.working_pool.take() {
            Some((identity, copy)) if identity == PoolIdentity::of(pool) => copy,
            _ => pool.clone(),
        }
    }
    /// `take_pool`'s copy back, its extra entries popped
    pub(super) fn put_pool(&mut self, pool: &Pool, working_pool: Pool) {
        debug_assert_eq!(
            working_pool.veterans.len(),
            pool.veterans.len(),
            "put_pool takes back the pool take_pool lent"
        );
        self.working_pool = Some((PoolIdentity::of(pool), working_pool));
    }
    /// Build the pool with career `her`'s veteran. A recorded pool is the pool of `legacies`, so lend its working copy
    /// with one entry pushed (`CareerSetup::pool_push` builds an entry as `CareerSetup::pool` does); otherwise rebuild
    /// from the recorded legacies and the new veteran.
    pub(super) fn push_her(
        &mut self,
        setup: &CareerSetup,
        legacies: &[Legacy],
        pool: &Pool,
        recorded_pool: bool,
        her: &Cur,
    ) -> HerPool {
        let veteran = setup.veteran(legacies, her.lineage, &her.record);
        if recorded_pool {
            debug_assert_eq!(pool.veterans.len(), legacies.len(), "one legacy per pool member");
            let mut extended = self.take_pool(pool);
            setup.pool_push(&mut extended, &veteran);
            HerPool { pool: extended, lent: true }
        } else {
            let mut extended = legacies.to_vec();
            extended.push(veteran);
            HerPool { pool: setup.pool(&extended), lent: false }
        }
    }
    /// Take back `push_her`'s pool: a lent working copy has the new veteran popped and is kept (`put_pool`).
    pub(super) fn pop_her(&mut self, pool: &Pool, her: HerPool) {
        let HerPool { pool: mut extended, lent } = her;
        if lent {
            extended.pop_veteran();
            self.put_pool(pool, extended);
        }
    }
    /// P(all) at chances q with spark scale c through trainee t's cached decks: (value, deck index; usize::MAX: none)
    pub(super) fn tval(
        &self,
        setup: &CareerSetup,
        trainee: UmaIndex,
        q: &[f64],
        c: f64,
        scratch: &mut Vec<f64>,
    ) -> (f64, usize) {
        // (1 - q per uncertain target, on the stack up to 32 of them)
        let uncertain_count = setup.uncertain_targets.len();
        let (mut stack_misses, heap_misses): ([f64; 32], Vec<f64>);
        let misses: &[f64] = if uncertain_count <= 32 {
            stack_misses = [0.0; 32];
            for (k, &i) in setup.uncertain_targets.iter().enumerate() {
                stack_misses[k] = 1.0 - q[i];
            }
            &stack_misses[..uncertain_count]
        } else {
            heap_misses = setup.uncertain_targets.iter().map(|&i| 1.0 - q[i]).collect();
            &heap_misses
        };
        let (mut best, mut best_deck) = (0.0f64, usize::MAX);
        for (deck_index, cached_deck) in self.cache.decks[trainee].iter().enumerate() {
            for table in &cached_deck.tables {
                let value = interp_in(table, misses, scratch);
                if best_deck == usize::MAX || value > best {
                    best = value;
                    best_deck = deck_index;
                }
            }
        }
        (c * best, best_deck)
    }
    /// trainee t's best cached deck at chances q (spark scale c, ceiling ub) after the refresh rule (sim module notes),
    /// when it can reach the bar th: (value, deck index); None when it provably cannot. A refresh searches in `env`
    /// (her chances folded in, its product proof kept in `env.state.ruled`) and caches the deck found.
    pub(super) fn deck_for(
        &mut self,
        env: &mut Env,
        request: DeckRequest,
        scratch: &mut Vec<f64>,
    ) -> Option<(f64, usize)> {
        let Env { setup, state, .. } = env;
        let mut reference_bound = request.ceiling;
        for reference in &self.cache.refs[request.trainee] {
            reference_bound = reference_bound.min(setup.ref_bound(
                request.trainee,
                request.inheritance_chances,
                request.spark_scale,
                reference,
            ));
            if reference_bound * (1.0 + 1e-9) < request.threshold {
                return None;
            }
        }
        if request.threshold > 0.0
            && setup.ruled_out(
                request.trainee,
                request.inheritance_chances,
                request.spark_scale,
                &state.ruled[setup.deck_class[request.trainee]],
                Bar::Below(request.threshold),
            )
        {
            return None;
        }
        self.counters.tables += 1;
        let (mut table_value, mut deck_index) =
            self.tval(setup, request.trainee, request.inheritance_chances, request.spark_scale, scratch);
        if deck_index == usize::MAX || reference_bound > table_value * (1.0 + self.margin) {
            setup.set_trainee(state, request.trainee, request.inheritance_chances);
            let start_decks: Vec<&SelectedDeck> =
                self.cache.decks[request.trainee].iter().map(|cached| &cached.deck).collect();
            state.work = 0;
            let mut scored = 0;
            let result = env.best_deck(
                crate::career::decks::DeckSearch {
                    trainee: request.trainee,
                    spark_scale: request.spark_scale,
                    bar: if self.fill_missing_decks && deck_index == usize::MAX { 0.0 } else { request.threshold },
                },
                &start_decks,
                &mut scored,
            );
            let Env { setup, state, memo, .. } = env;
            if let Some(product_bound) = state.last_product_bound.take() {
                let deck_class = setup.deck_class[request.trainee];
                state.ruled[deck_class]
                    .push(ProductProof { inheritance_chances: request.inheritance_chances.to_vec(), product_bound });
            }
            self.counters.searches += 1;
            if let DeckRes::Done(value, deck, full) = result {
                self.cache.add(setup, state, memo, request.trainee, &deck);
                if full {
                    self.cache.refs[request.trainee].push(Ref {
                        inheritance_chances: request.inheritance_chances.to_vec(),
                        value,
                        deck,
                    });
                } else {
                    self.counters.partial_searches += 1;
                }
                (table_value, deck_index) =
                    self.tval(setup, request.trainee, request.inheritance_chances, request.spark_scale, scratch);
            }
        }
        if deck_index == usize::MAX { None } else { Some((table_value, deck_index)) }
    }
    /// `CareerSetup::ceiling` of a candidate lineage on `pool` without allocations (the same slots, affinities and
    /// formula as `slots`, `inherit` and `outfit_fast`; equal up to rounding, so it only filters, with a slack)
    pub(super) fn fast_ub(&self, setup: &CareerSetup, pool: &Pool, lineage: Lineage) -> f64 {
        let Lineage { trainee, first_parent, second_parent } = lineage;
        let target_count = setup.target_count;
        // `walk_slots`' slots and affinities, each slot's misses multiplied in as it
        // comes but a parent's after her grandparent slots' (this filter's own order, so its own rounding); a
        // random-star slot counts at its fixed stars here, where `inherit` averages its squared miss over the mix
        struct CeilingAccumulator<'pool> {
            target_count: usize,
            miss: [f64; 12],
            carriers: [i32; 12],
            parent: Option<(&'pool [u8], f64)>,
        }
        impl CeilingAccumulator<'_> {
            #[inline(always)]
            fn add(&mut self, stars: &[u8], affinity_factor: f64) {
                for k in 0..self.target_count {
                    let target_stars = stars[k];
                    if target_stars > 0 {
                        self.carriers[k] += 1;
                        self.miss[k] *= insp_miss(f64::from(target_stars), affinity_factor);
                    }
                }
            }
            #[inline(always)]
            fn flush(&mut self) {
                if let Some((parent_stars, affinity_factor)) = self.parent.take() {
                    self.add(parent_stars, affinity_factor);
                }
            }
        }
        impl<'pool> SlotSink<'pool> for CeilingAccumulator<'pool> {
            #[inline(always)]
            fn slot(
                &mut self,
                _: usize,
                grandparent_slot: Option<usize>,
                who: &'pool Who,
                relation_points: f64,
                race: f64,
            ) {
                if grandparent_slot.is_none() {
                    self.flush();
                    self.parent = Some((&who.stars[..], aff_factor(relation_points, race)));
                } else {
                    self.add(&who.stars, aff_factor(relation_points, race));
                }
            }
        }
        let mut accumulator =
            CeilingAccumulator { target_count, miss: [1.0f64; 12], carriers: [0i32; 12], parent: None };
        setup.walk_slots(pool, first_parent, second_parent, &RelOf { setup, pool, trainee }, &mut accumulator);
        accumulator.flush();
        let (miss, carriers) = (accumulator.miss, accumulator.carriers);
        let mut lineage_scale = 1.0f64;
        for k in 0..target_count {
            lineage_scale *= spark_scale(carriers[k]);
        }
        let mut best = 0.0f64;
        for factors in &self.ceiling_factors[trainee] {
            let mut value = 1.0f64;
            for (j, &i) in setup.uncertain_targets.iter().enumerate() {
                value *= factors[j].0 - factors[j].1 * miss[i] * miss[i];
            }
            if value > best {
                best = value;
            }
        }
        setup.space.rho_top() * lineage_scale * self.fixed * best
    }
    /// value a candidate lineage on `pool` against `bar`: her best cached deck after the refresh rule (`deck_for`), as
    /// a pick, when her ceiling clears the bar and a deck can reach it
    pub(super) fn consider(
        &mut self,
        env: &mut Env,
        pool: &Pool,
        lineage: Lineage,
        bar: f64,
        buf: &mut Vec<f64>,
    ) -> Option<SurrogatePick> {
        self.counters.candidates += 1;
        let trainee = lineage.trainee;
        let mut q = std::mem::take(&mut self.inheritance_buffer);
        let c = env.setup.lineage_into(pool, lineage, &mut q);
        let pick = 'value: {
            let Some(ub) = env.setup.ceiling(&mut env.state, trainee, &q, c) else { break 'value None };
            if crate::checks::get().sim_check_ub {
                let fast_bound = self.fast_ub(&env.setup, pool, lineage);
                assert!((fast_bound - ub).abs() <= 1e-12 * ub, "fast_ub {fast_bound} vs ceiling {ub}");
            }
            if ub * (1.0 + 1e-9) < bar {
                break 'value None;
            }
            self.counters.ceiling_passes += 1;
            self.deck_for(
                env,
                DeckRequest { trainee, inheritance_chances: &q, spark_scale: c, ceiling: ub, threshold: bar },
                buf,
            )
            .map(|(value, deck_index)| SurrogatePick {
                value,
                lineage: lineage.ordered(),
                deck: deck_index,
            })
        };
        self.inheritance_buffer = q;
        pick
    }
    /// what her schedule leaves her veteran in the best of `outfits` for it (`ExpectedRecord`), cached by what the
    /// schedule reads
    pub(super) fn wins(
        &mut self,
        setup: &CareerSetup,
        pool: &Pool,
        lineage: Lineage,
        outfits: ScheduleOutfits,
    ) -> ExpectedRecord {
        self.win_chances.of(setup, pool, lineage, outfits)
    }
    /// `wins` of a career with its deck: in the outfit option her exact odds pick (`CareerSetup::exact`, after folding
    /// her chances in, as `make_cur` does), found only where her outfits' grades differ (any option reads the same
    /// grades otherwise)
    pub(super) fn career_wins(
        &mut self,
        setup: &CareerSetup,
        state: &mut SearchState,
        pool: &Pool,
        career: Career,
    ) -> ExpectedRecord {
        let lineage = career.lineage;
        let trainee = lineage.trainee;
        let option = if setup.data.umas[trainee].outfit_aptitudes.is_empty() {
            0
        } else {
            let Inheritance { chances: q, carriers } = inherit(&setup.slots(pool, lineage), setup.target_count);
            setup.set_trainee(state, trainee, &q);
            let options = ExactOptions { all_but_one: false, landed_sets: false };
            setup.exact(state, trainee, career.deck, &carriers, options).outfit
        };
        self.win_chances.of(setup, pool, lineage, ScheduleOutfits::DeckOption(option))
    }
    /// the career with its deck: its exact numbers; `rotation`: a loop rotation career (`Cur::rotation`)
    pub(super) fn make_cur(
        &mut self,
        setup: &CareerSetup,
        state: &mut SearchState,
        pool: &Pool,
        career: Career,
        rotation: bool,
    ) -> Cur {
        career_numbers(setup, state, &mut self.win_chances, pool, career, rotation)
    }
    /// `make_cur` for a cached pick (her deck read in place from the cache)
    pub(super) fn cached_cur(
        &mut self,
        setup: &CareerSetup,
        state: &mut SearchState,
        pool: &Pool,
        pick: SurrogatePick,
        rotation: bool,
    ) -> Cur {
        let deck = &self.cache.decks[pick.lineage.trainee][pick.deck].deck;
        career_numbers(setup, state, &mut self.win_chances, pool, Career { lineage: pick.lineage, deck }, rotation)
    }
}
