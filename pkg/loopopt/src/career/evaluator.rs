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
    pub(super) os: Vec<f64>,
    /// Her chance of at least one win, per race index.
    pub(super) wp: Rc<Vec<f64>>,
    /// A loop rotation career: her veteran may replace her member's rotation veteran (`rot_update`).
    pub(super) rotation: bool,
}
/// The evaluator's work counters (`Evaluator::n`).
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
/// Her chance per race of at least one win, per (trainee, her parents' wins and pink inputs) (`Evaluator::wins`).
#[derive(Default)]
pub(super) struct WinChances(crate::FxMap<(UmaIndex, Vec<usize>), Rc<Vec<f64>>>);
impl WinChances {
    /// her chance per race of at least one win, cached per (trainee, the parents' wins)
    fn of(&mut self, setup: &CareerSetup, pool: &Pool, lineage: Lineage) -> Rc<Vec<f64>> {
        let mut k: Vec<usize> = pool.veterans[lineage.first_parent]
            .wins
            .iter()
            .chain(&pool.veterans[lineage.second_parent].wins)
            .map(|w| w.race)
            .collect();
        k.sort_unstable();
        // her pink sparks change her win chances: their inputs join the key (start grades, each spark's aptitude and chance)
        if let Some(pc) = setup.pink_career(pool, lineage) {
            k.push(usize::MAX);
            k.extend(pc.start.iter().map(|&g| g as usize));
            for f in &pc.fire {
                k.push(f.apt);
                k.push((f.p * 1e9).round() as usize);
            }
        }
        if let Some(x) = self.0.get(&(lineage.trainee, k.clone())) {
            return Rc::clone(x);
        }
        let w = Rc::new(setup.schedule(pool, lineage).win_chances);
        self.0.insert((lineage.trainee, k), Rc::clone(&w));
        w
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
    pub(super) fastc: UmaTable<Vec<Vec<(f64, f64)>>>,
    /// The model's fixed gold factor (`Model::fixed_factor`).
    pub(super) fixed: f64,
    pub(super) win_chances: WinChances,
    pub(super) n: Counters,
    /// a lineage's chances, reused (`CareerSetup::lineage_into`); empty between uses
    pub(super) qs: Vec<f64>,
    /// the planner's working copy of its pool (`take_pool`), with the identity of the pool it copies
    working_pool: Option<(PoolIdentity, Pool)>,
}

/// Per trainee and outfit, per uncertain target (A, B): her ceiling factor is A - B (1 - q) (`Evaluator::fastc`; none
/// without legal deck odds). `outfit_fast` at q = 0 on the hint chances `set_trainee` would write, split: A = dg GOLD +
/// (1 - dg) WHITE and B = (1 - dg) WHITE (1 - dw)(1 - e) where she lacks the skill, B = 0 where she has it
fn fastc(setup: &CareerSetup) -> UmaTable<Vec<Vec<(f64, f64)>>> {
    setup
        .data
        .umas
        .indices()
        .map(|t| match &setup.deck_odds_ceilings[t] {
            None => vec![],
            Some((dw, dg)) => setup.outfit_coverage[t]
                .iter()
                .zip(&setup.model.ews[t])
                .map(|(sv, ew)| {
                    (0..setup.uncertain_count)
                        .map(|k| match sv[k] {
                            2 => (GEN_GOLD, 0.0),
                            1 => (dg[k] * GEN_GOLD + (1.0 - dg[k]) * crate::skills::GEN_WHITE, 0.0),
                            _ => (
                                dg[k] * GEN_GOLD + (1.0 - dg[k]) * crate::skills::GEN_WHITE,
                                (1.0 - dg[k]) * crate::skills::GEN_WHITE * (1.0 - dw[k]) * (1.0 - ew[k]),
                            ),
                        })
                        .collect()
                })
                .collect(),
        })
        .collect()
}

/// The career with its deck: its exact numbers; `rotation`: a loop rotation career (`Cur::rotation`). Her win chances
/// come from (and fill) `win_chances`.
fn career_numbers(
    setup: &CareerSetup,
    state: &mut SearchState,
    win_chances: &mut WinChances,
    pool: &Pool,
    career: Career,
    rotation: bool,
) -> Cur {
    let lineage = career.lineage;
    let t = lineage.trainee;
    let Inheritance { chances: q, carriers: nc } = inherit(&setup.slots(pool, lineage), setup.target_count);
    setup.set_trainee(state, t, &q);
    let exact = setup.exact(state, t, career.deck, &nc, ExactOptions { all_but_one: true, landed_sets: true });
    let os = setup.outcome_sets(exact.outfit, &exact.ext);
    let wp = win_chances.of(setup, pool, lineage);
    Cur { lineage, p_all: exact.p_all, os, wp, rotation }
}

impl Evaluator {
    /// The job's evaluator: an empty deck cache, `fastc` and the fixed gold factor
    pub(super) fn new(setup: &CareerSetup, margin: f64, fill_missing_decks: bool) -> Self {
        Self {
            cache: Cache::new(setup.data.umas.len()),
            margin,
            fill_missing_decks,
            fastc: fastc(setup),
            fixed: setup.model.fixed_factor(),
            win_chances: WinChances::default(),
            n: Counters::default(),
            qs: vec![],
            working_pool: None,
        }
    }
    /// A working copy of `pool` (the planner's `CareerSetup::pool(legs)`), equal to it: the one kept by `put_pool` when
    /// it was made from this pool (the same address, length and borrowing rule: the pool a caller passes is borrowed,
    /// so unchanged, or, as the simulator's, only grown), else a clone. A user pushes its extra entries
    /// (`CareerSetup::pool_push` builds an entry as `CareerSetup::pool` does, so the pool of legs and those) and pops
    /// them before `put_pool`
    pub(super) fn take_pool(&mut self, pool: &Pool) -> Pool {
        match self.working_pool.take() {
            Some((identity, copy)) if identity == PoolIdentity::of(pool) => copy,
            _ => pool.clone(),
        }
    }
    /// `take_pool`'s copy back, its extra entries popped
    pub(super) fn put_pool(&mut self, pool: &Pool, p: Pool) {
        debug_assert_eq!(p.veterans.len(), pool.veterans.len(), "put_pool takes back the pool take_pool lent");
        self.working_pool = Some((PoolIdentity::of(pool), p));
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
        let veteran = setup.veteran(legacies, her.lineage, her.wp.to_vec());
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
    pub(super) fn tval(&self, setup: &CareerSetup, t: UmaIndex, q: &[f64], c: f64, buf: &mut Vec<f64>) -> (f64, usize) {
        // (1 - q per uncertain target, on the stack up to 32 of them)
        let nu = setup.uncertain_targets.len();
        let (mut xa, xv): ([f64; 32], Vec<f64>);
        let x: &[f64] = if nu <= 32 {
            xa = [0.0; 32];
            for (k, &i) in setup.uncertain_targets.iter().enumerate() {
                xa[k] = 1.0 - q[i];
            }
            &xa[..nu]
        } else {
            xv = setup.uncertain_targets.iter().map(|&i| 1.0 - q[i]).collect();
            &xv
        };
        let (mut best, mut bd) = (0.0f64, usize::MAX);
        for (di, dk) in self.cache.decks[t].iter().enumerate() {
            for tab in &dk.tab {
                let v = interp_in(tab, x, buf);
                if bd == usize::MAX || v > best {
                    best = v;
                    bd = di;
                }
            }
        }
        (c * best, bd)
    }
    /// trainee t's best cached deck at chances q (spark scale c, ceiling ub) after the refresh rule (sim module notes),
    /// when it can reach the bar th: (value, deck index); None when it provably cannot. A refresh searches in `env`
    /// (her chances folded in, its product proof kept in `env.state.ruled`) and caches the deck found.
    pub(super) fn deck_for(&mut self, env: &mut Env, request: DeckRequest, buf: &mut Vec<f64>) -> Option<(f64, usize)> {
        let Env { ref setup, ref mut state, ref mut audit, ref mut memo } = *env;
        let mut rb = request.ceiling;
        for r in &self.cache.refs[request.trainee] {
            rb = rb.min(setup.ref_bound(request.trainee, request.inheritance_chances, request.spark_scale, r));
            if rb * (1.0 + 1e-9) < request.threshold {
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
        self.n.tables += 1;
        let (mut tv, mut di) = self.tval(setup, request.trainee, request.inheritance_chances, request.spark_scale, buf);
        if di == usize::MAX || rb > tv * (1.0 + self.margin) {
            setup.set_trainee(state, request.trainee, request.inheritance_chances);
            let sr: Vec<&SelectedDeck> = self.cache.decks[request.trainee].iter().map(|x| &x.sel).collect();
            state.work = 0;
            let mut sc = 0;
            let r = setup.best_deck(
                state,
                audit,
                memo,
                request.trainee,
                request.spark_scale,
                if self.fill_missing_decks && di == usize::MAX { 0.0 } else { request.threshold },
                &sr,
                &mut sc,
            );
            if let Some(s) = state.last_product_bound.take() {
                let ct = setup.deck_class[request.trainee];
                state.ruled[ct]
                    .push(ProductProof { inheritance_chances: request.inheritance_chances.to_vec(), product_bound: s });
            }
            self.n.searches += 1;
            if let DeckRes::Done(v, sel, full) = r {
                self.cache.add(setup, state, memo, request.trainee, &sel);
                if full {
                    self.cache.refs[request.trainee].push(Ref {
                        q: request.inheritance_chances.to_vec(),
                        val: v,
                        deck: sel,
                    });
                } else {
                    self.n.partial_searches += 1;
                }
                (tv, di) = self.tval(setup, request.trainee, request.inheritance_chances, request.spark_scale, buf);
            }
        }
        if di == usize::MAX { None } else { Some((tv, di)) }
    }
    /// `CareerSetup::ceiling` of a candidate lineage on `pool` without allocations (the same slots, affinities and
    /// formula as `slots`, `inherit` and `outfit_fast`; equal up to rounding, so it only filters, with a slack)
    pub(super) fn fast_ub(&self, setup: &CareerSetup, pool: &Pool, lineage: Lineage) -> f64 {
        let Lineage { trainee: t, first_parent: a, second_parent: b } = lineage;
        let ntar = setup.target_count;
        // `walk_slots`' slots and affinities, each slot's misses multiplied in as it
        // comes but a parent's after her grandparent slots' (this filter's own order, so its own rounding); a
        // random-star slot counts at its fixed stars here, where `inherit` averages its squared miss over the mix
        struct Ub<'pool> {
            ntar: usize,
            miss: [f64; 12],
            nc: [i32; 12],
            parent: Option<(&'pool [u8], f64)>,
        }
        impl Ub<'_> {
            #[inline(always)]
            fn add(&mut self, stars: &[u8], fa: f64) {
                for k in 0..self.ntar {
                    let s = stars[k];
                    if s > 0 {
                        self.nc[k] += 1;
                        self.miss[k] *= insp_miss(f64::from(s), fa);
                    }
                }
            }
            #[inline(always)]
            fn flush(&mut self) {
                if let Some((s, fa)) = self.parent.take() {
                    self.add(s, fa);
                }
            }
        }
        impl<'pool> SlotSink<'pool> for Ub<'pool> {
            #[inline(always)]
            fn slot(&mut self, _: usize, gi: Option<usize>, w: &'pool Who, rel: f64, race: f64) {
                if gi.is_none() {
                    self.flush();
                    self.parent = Some((&w.stars[..], aff_factor(rel, race)));
                } else {
                    self.add(&w.stars, aff_factor(rel, race));
                }
            }
        }
        let mut ub = Ub { ntar, miss: [1.0f64; 12], nc: [0i32; 12], parent: None };
        setup.walk_slots(pool, a, b, &RelOf { setup, p: pool, t }, &mut ub);
        ub.flush();
        let (miss, nc) = (ub.miss, ub.nc);
        let mut c = 1.0f64;
        for k in 0..ntar {
            c *= spark_scale(nc[k]);
        }
        let mut best = 0.0f64;
        for ab in &self.fastc[t] {
            let mut v = 1.0f64;
            for (j, &i) in setup.uncertain_targets.iter().enumerate() {
                v *= ab[j].0 - ab[j].1 * miss[i] * miss[i];
            }
            if v > best {
                best = v;
            }
        }
        setup.space.rho_top() * c * self.fixed * best
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
        self.n.candidates += 1;
        let t = lineage.trainee;
        let mut q = std::mem::take(&mut self.qs);
        let c = env.setup.lineage_into(pool, lineage, &mut q);
        let pick = 'value: {
            let Some(ub) = env.setup.ceiling(&mut env.state, t, &q, c) else { break 'value None };
            if env.setup.space.deck_job.checks.sim_check_ub {
                let f = self.fast_ub(&env.setup, pool, lineage);
                assert!((f - ub).abs() <= 1e-12 * ub, "fast_ub {f} vs ceiling {ub}");
            }
            if ub * (1.0 + 1e-9) < bar {
                break 'value None;
            }
            self.n.ceiling_passes += 1;
            self.deck_for(
                env,
                DeckRequest { trainee: t, inheritance_chances: &q, spark_scale: c, ceiling: ub, threshold: bar },
                buf,
            )
            .map(|(tv, di)| SurrogatePick { value: tv, lineage: lineage.ordered(), deck: di })
        };
        self.qs = q;
        pick
    }
    /// her chance per race of at least one win, cached per (trainee, the parents' wins)
    pub(super) fn wins(&mut self, setup: &CareerSetup, pool: &Pool, lineage: Lineage) -> Rc<Vec<f64>> {
        self.win_chances.of(setup, pool, lineage)
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
        let deck = &self.cache.decks[pick.lineage.trainee][pick.deck].sel;
        career_numbers(setup, state, &mut self.win_chances, pool, Career { lineage: pick.lineage, deck }, rotation)
    }
}
