//! Policy simulator on a surrogate (`simulate` on a career job; docs/SOLVER.md "Policy simulator").
//! Shared simulator records, construction and deck evaluation live here; policies, draws, audits and path execution
//! live in `sim/policies.rs`, `sim/draw.rs`, `sim/audit.rs` and `sim/play.rs`.
//!
//! Under the model, the expected number of careers until the first career whose veteran lands every target, for a
//! decision policy on a fixed starting pool. One process retains the loaded data and deck cache between careers;
//! policies may still search for new decks or careers. Per run (derived):
//!
//!   E = sum over careers t of S_t,   S_0 = 1,   S_{t+1} = S_t (1 - p_t)
//!
//! with p_t the exact P(all) of career t; the path continues through failures only (each failed veteran drawn from the
//! career's exact landed-set distribution with the all-landed set removed), and stops when S_t < eps or at
//! `max_careers`, where the tail S_t / p_t (career t repeated forever) is added.
//!
//! The modules call each other on purpose: `run_career` (career.rs) runs `simulate` for a job with `simulate`, and the
//! `exact` and `index` policies (`exact_cur`, `index_cur`) call `run_career` again on that job with `simulate` removed,
//! so the simulator tests the same path the page runs.
//!
//! The surrogate (policy `greedy`): the career with the highest P(all), where each candidate (trainee, parents) is
//! valued by its trainee's cached decks only (`Cache`: per deck and outfit the corner table, exact at any inheritance
//! chances, `Env::corners`). A candidate is first bounded by the search's ceiling and by `rise` from each fully checked
//! deck of hers (`Ref`, proven best at its chances) and by her product-search proofs (`ruled`); a candidate whose bound
//! rb clears the best so far B and exceeds her table value tv by more than the margin m gets an exact deck search
//! there (`best_deck` at bar B, the per-uma search), and the deck found joins the cache ("refresh"). Derived: for the
//! exact best career c* (value E*), either rb(c*) < B (so E* < B), or it was searched (tv = E* when the search
//! finished), or tv >= rb / (1 + m) >= E* / (1 + m); B only rises. So the career picked has an exact P(all), with the
//! cached deck it is run with, of at least E* / (1 + m) (m = 0: the exact greedy pick up to ties and searches that ran
//! out of work). A candidate is valued once, when its last parent arrives: a new veteran only creates candidates with
//! her as a parent, and each trainee's static ceiling (q = 1, every spark on six legacies) skips her outright once it
//! is below B.
//!
//! `margin: -1` searches every candidate no bound rules out (exact-backed: the career search's own proofs, nothing
//! trusted from the cache). `audit` records the surrogate against fresh exact searches (`audit_row`); `check` compares
//! it with the exact greedy pick every few veterans. The job's `fix` is ignored; its `trainees` are the trainees.
//!
//! Draws (`u01`): keyed by (seed, run, career index, stream, item), so a run index faces the same uniforms under every
//! policy (common random numbers). Landed set: inverse CDF over the failed sets ordered fewest-landed first; stars per
//! landed target from `star_dist`; G1 wins per race from the schedule's chance of at least one win. Pruning: a new
//! veteran is dropped when an owned veteran of the same character with the same two parents has at least her stars
//! on every target and every G1 she won (exact for P(all)).
//!
//! Policies (`simulate`'s match on `policy`): `greedy` (surrogate), `greedy_exact` (the career search in process per
//! decision: the best career by P(all)), `loop` (a fixed rotation of `loop_members`: each career's parents are the
//! latest veterans of the previous two members in rotation order; bootstrap below), and `trace` (building for
//! `loop_members[0]` with the others as builders: the measurement `Cont`'s shape is fitted to), and `index` (the page's
//! plan: `index_rule` on the step's pool, run as the page's plan job in process, its `choice` the career run). A policy
//! picks the next career (`Cur`) from the state (`PathState`) in `play`; `trace` runs its own loop and returns its rows.
//!
//! `index` and the worker's kept state: a plan job reads and leaves the kept plans (`plan::kept::BESTS`), proofs (`SEEN`),
//! main searches (`LAST_SOLVE`) and deck memos (`DECK_KEEP`). Each run starts with all four empty (`kept_swap`; the
//! state before the simulation is put back after it), so a run's plans read only what its own earlier steps left: a
//! run's careers are a function of the job and the run index, not of the runs before it or their order. Within a run
//! a step's pool is the last one plus her veteran at the end (or the same pool, when she is pruned), the case the kept
//! plans serve (`Bests`).
//!
//! Loop bootstrap: a member whose two parents (the latest rotation veterans of the previous two members) both exist
//! runs with them; otherwise her parents are the best pair for her by exact P(all) from the pool (the career search
//! with her as the only trainee) with every existing one of those rotation veterans fixed as a parent. A loop career
//! keeps her own schedule (the career model's); the loop planner coordinates schedules across members, this does not.
//! The loop adds every veteran without dominance pruning. Its rotation keeps the better of the previous veteran and
//! the new one for the next two loop careers (`rot_update`), keeping the previous veteran on a tie.
use crate::career::cache::Cache;
use crate::career::inherit::{ExactOptions, Inheritance, RelOf, SlotSink, aff_factor, inherit, insp_miss, spark_scale};
use crate::career::outcomes::interp_in;
/// the page's next career (`next` on a career job)
use crate::career::plan;
use crate::career::pool::{Pool, Who};
use crate::career::search::{Bar, DeckRes, Ref};
use crate::career::sim::play::PathState;
use crate::career::state::ProductProof;
use crate::career::{Career, Env, JobInputs, Legacy, Lineage};
use crate::data::Data;
use crate::deck::space::SelectedDeck;
use crate::deck::{CardsRaw, d_true};
use crate::skills::{GEN_GOLD, SkillsRaw};
use serde::Deserialize;
use serde_json::{Value, json};
use std::rc::Rc;

mod audit;
mod draw;
pub(super) mod play;
mod policies;

fn d_policy() -> String {
    "greedy".into()
}
const fn d_runs() -> usize {
    16
}
const fn d_seed() -> u32 {
    1
}
const fn d_eps() -> f64 {
    1e-3
}
const fn d_maxc() -> usize {
    100_000
}
const fn d_frac() -> f64 {
    0.3
}
const fn d_asample() -> usize {
    3
}
const fn d_aevery() -> usize {
    50
}

#[derive(Deserialize, Clone)]
pub(super) struct SimJob {
    /// greedy | greedy_exact | loop | trace | index
    #[serde(default = "d_policy")]
    policy: String,
    #[serde(default = "d_runs")]
    runs: usize,
    #[serde(default)]
    first: usize,
    #[serde(default = "d_seed")]
    seed: u32,
    #[serde(default = "d_eps")]
    eps: f64,
    #[serde(default = "d_maxc")]
    max_careers: usize,
    /// refresh margin m (module notes); 0 = refresh whenever a bound exceeds the table value
    #[serde(default)]
    pub(super) margin: f64,
    #[serde(default = "d_true")]
    prune: bool,
    /// keep the deck cache across runs (faster; with m > 0 a pick can depend on what earlier runs cached)
    #[serde(default = "d_true")]
    share_cache: bool,
    /// every `check` new veterans (and at the start), compare the surrogate with exact searches
    #[serde(default)]
    check: usize,
    /// lineages checked per check (the newest veteran's candidates, ceiling at least `check_frac` x B, best first)
    #[serde(default)]
    check_lineages: usize,
    #[serde(default = "d_frac")]
    check_frac: f64,
    /// loop: rotation members (chara ids) in order; trace: the trainee built for, then her builders
    #[serde(default)]
    loop_members: Vec<u32>,
    /// fidelity audit (`audit_row`): at every pick change of greedy (the exact argmax, the pick and `audit_sample`
    /// random candidates with the newest veteran), and every `audit_every` careers of the loop (its career, and its
    /// keep-or-replace decision under exact values)
    #[serde(default)]
    audit: bool,
    #[serde(default = "d_asample")]
    audit_sample: usize,
    #[serde(default = "d_aevery")]
    audit_every: usize,
    /// log progress to stderr every this many careers (0: off)
    #[serde(default)]
    progress: usize,
    /// index: the plan job's `next` (docs/SOLVER.md "Next career"); the page sends `{}`
    #[serde(default = "d_next")]
    next: Value,
}
fn d_next() -> Value {
    json!({})
}
/// index: the page's `top` for its plan job (`ROLL_K` in webapp/src/career.js)
const PAGE_TOP: usize = 6;
/// The worker's kept state across jobs: the kept plans, proofs, main searches and deck memos
#[derive(Default)]
struct KeptState {
    bests: Vec<plan::kept::Bests>,
    seen: Vec<(u64, crate::uma::UmaTable<Vec<Ref>>)>,
    searches: Vec<crate::career::state::KeptSolve>,
    decks: Vec<(u64, crate::career::decks::DeckSearchCache)>,
}
/// Replace the worker's kept state, returning the previous state (`index`: module notes).
fn kept_swap(replacement: KeptState) -> KeptState {
    let bests = plan::kept::bests_take();
    plan::kept::bests_set(replacement.bests);
    let seen = crate::career::state::SEEN.with(|cache| std::mem::replace(&mut *cache.borrow_mut(), replacement.seen));
    let searches = crate::career::state::LAST_SOLVE
        .with(|cache| std::mem::replace(&mut *cache.borrow_mut(), replacement.searches));
    let decks =
        crate::career::decks::DECK_KEEP.with(|cache| std::mem::replace(&mut *cache.borrow_mut(), replacement.decks));
    KeptState { bests, seen, searches, decks }
}
/// The best career so far by the surrogate (`Sim::consider_at`).
#[derive(Clone, Copy)]
struct SurrogatePick {
    /// Spark scale C times her best cached deck's table value.
    value: f64,
    /// Trainee and parents, first parent the lower legacy index.
    lineage: Lineage,
    /// The deck's index among her cached decks (`Cache::decks`).
    deck: usize,
}
/// A career to run, with its exact numbers
pub(crate) struct Cur {
    lineage: Lineage,
    p_all: f64,
    /// Chance of each landed set (bit i = job target i).
    pub(super) os: Vec<f64>,
    /// Her chance of at least one win, per race index.
    pub(super) wp: Rc<Vec<f64>>,
    /// A loop rotation career: her veteran may replace her member's rotation veteran (`rot_update`).
    rotation: bool,
}
/// The simulator's work counters (`Sim::n`).
#[derive(Default, Clone, Copy)]
pub(super) struct Counters {
    /// Candidate careers valued.
    candidates: usize,
    /// Candidates whose ceiling cleared the bar.
    ceiling_passes: usize,
    /// Cached-table valuations (`deck_for`).
    tables: usize,
    /// Deck searches ("refreshes").
    pub(super) searches: usize,
    /// Deck searches that did not finish (no `Ref`).
    partial_searches: usize,
    /// Veterans added to a path's pool.
    veterans_added: usize,
}

/// Owned roster tables moved into a simulator.
pub(super) struct SimulationRoster {
    /// Trainees considered by later simulated careers, in search order.
    pub(super) trainees: Vec<crate::uma::UmaIndex>,
    /// Rotation members in the supplied order.
    pub(super) members: Vec<crate::uma::UmaIndex>,
    /// Rotation position for each Data::umas index, when tracing members.
    pub(super) member_by_trainee: crate::uma::UmaTable<Option<usize>>,
}

/// Independent simulator behaviours selected by its caller.
#[derive(Clone, Copy)]
pub(super) struct SimulationOptions {
    /// Record audit output for the main simulation.
    pub(super) record_audits: bool,
    /// Search for decks absent from the simulator cache.
    pub(super) fill_missing_decks: bool,
}

pub(crate) struct Sim<'env, 'data> {
    pub(super) env: &'env mut Env<'data>,
    pub(super) d: &'data Data,
    skills: &'data SkillsRaw,
    cards: &'data CardsRaw,
    job: &'data Value,
    pub(super) cfg: SimJob,
    pub(super) cache: Cache,
    trainees: Vec<crate::uma::UmaIndex>,
    /// Per uma, her static ceiling (q = 1 on every target; 0 when she is not owned)
    static_ceilings: crate::uma::UmaTable<f64>,
    /// The failed landed sets (every set but the full one), fewest landed first, ties by mask: the draw's order
    failed_sets: Vec<usize>,
    /// The rank's 1/2/3-star shares of a landed target's spark
    star_mix: [f64; 3],
    /// Win chances per (trainee, her parents' wins and pink inputs) (`wins`)
    win_chances: crate::FxMap<(crate::uma::UmaIndex, Vec<usize>), Rc<Vec<f64>>>,
    pub(super) n: Counters,
    members: Vec<crate::uma::UmaIndex>,
    /// per trainee and outfit, per uncertain target (A, B): her ceiling factor is A - B (1 - q) (`fast_ub`)
    pub(super) fastc: crate::uma::UmaTable<Vec<Vec<(f64, f64)>>>,
    pub(super) fixed: f64,
    /// trace: trainee → loop member index (empty under the other policies)
    member_by_trainee: crate::uma::UmaTable<Option<usize>>,
    /// the simulator's own path (false when the plan uses this machinery): audits are taken only there
    record_audits: bool,
    audits: Vec<Value>,
    /// the plan only (false for every policy here): a trainee without a cached deck gets her best deck searched without
    /// a bar (so she has one from then on), instead of a search at the bar that caches nothing when it ends Bounded
    fill_missing_decks: bool,
    /// a lineage's chances, reused (`lineage_into`)
    pub(super) qs: Vec<f64>,
    /// the planner's working copy of its pool (`take_pool`), with the identity of the pool it copies
    working_pool: Option<(PoolIdentity, Pool)>,
}
/// Which pool a working copy was made from (`Sim::take_pool`): its address, length and borrowing rule.
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
fn canon(chara: u32, sparks: &[(u32, u8)], pinks: &[(u32, u8)], wins: &[usize]) -> String {
    let (mut s, mut p, mut w) = (sparks.to_vec(), pinks.to_vec(), wins.to_vec());
    s.sort_unstable();
    p.sort_unstable();
    w.sort_unstable();
    format!("{chara}|{s:?}|{p:?}|{w:?}")
}
/// a veteran's family: the character and her two parents as recorded
fn family(l: &Legacy) -> String {
    let mut ps: Vec<String> = l.parents.iter().map(|g| canon(g.chara, &g.sparks, &g.pinks, &g.wins)).collect();
    ps.sort();
    format!("{}#{}", l.chara, ps.join("#"))
}
fn leg_json(l: &Legacy) -> Value {
    json!({"chara": l.chara, "borrowed": l.borrowed, "sparks": l.sparks, "wins": l.wins, "pinks": l.pinks,
           "parents": l.parents.iter().map(|g| json!({"chara": g.chara, "sparks": g.sparks, "wins": g.wins, "pinks": g.pinks})).collect::<Vec<_>>()})
}
/// Per trainee and outfit, per uncertain target (A, B): her ceiling factor is A - B (1 - q) (`Sim::fastc`; none
/// without legal deck odds). `outfit_fast` at q = 0 on the hint chances `set_trainee` would write, split: A = dg GOLD +
/// (1 - dg) WHITE and B = (1 - dg) WHITE (1 - dw)(1 - e) where she lacks the skill, B = 0 where she has it
fn fastc(env: &Env) -> crate::uma::UmaTable<Vec<Vec<(f64, f64)>>> {
    env.data
        .umas
        .indices()
        .map(|t| match &env.deck_odds_ceilings[t] {
            None => vec![],
            Some((dw, dg)) => env.outfit_coverage[t]
                .iter()
                .zip(&env.own_event_chances[t])
                .map(|(sv, ew)| {
                    (0..env.uncertain_count)
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
/// A trainee's inheritance chances and the bounds for choosing her cached or searched deck.
#[derive(Clone, Copy)]
pub(super) struct DeckRequest<'chances> {
    /// Trainee's position in Data::umas.
    pub(super) trainee: crate::uma::UmaIndex,
    /// Inheritance probability per job target.
    pub(super) inheritance_chances: &'chances [f64],
    /// Product of inheritance spark multipliers.
    pub(super) spark_scale: f64,
    /// Upper bound on this lineage's success probability.
    pub(super) ceiling: f64,
    /// Minimum success probability worth searching.
    pub(super) threshold: f64,
}

impl<'env, 'data> Sim<'env, 'data> {
    /// The simulator's machinery for a job (`simulate`'s, and the plan's through `plan::search::sim_for`): the landed sets in
    /// order of size, each owned trainee's ceiling at q = 1 (`static_ceilings`), the rank's star mix, `fastc`, the
    /// fixed gold factor
    pub(super) fn new(
        env: &'env mut Env<'data>,
        input: &JobInputs<'data>,
        cfg: SimJob,
        roster: SimulationRoster,
        options: SimulationOptions,
    ) -> Self {
        let ntar = env.target_count;
        let full = (1usize << ntar) - 1;
        let mut failed_sets: Vec<usize> = (0..full).collect();
        failed_sets.sort_by_key(|&m| (m.count_ones(), m));
        let n = input.data.umas.len();
        let one = vec![1.0f64; ntar];
        let static_ceilings: crate::uma::UmaTable<f64> = env
            .data
            .umas
            .indices()
            .map(|t| if env.owned.contains(&t) { env.ceiling(t, &one, 1.0).unwrap_or(0.0) } else { 0.0 })
            .collect();
        let star_mix = env.skill_job.star_dist;
        let fastc = fastc(env);
        let fixed = env.model.fixed_factor();
        Sim {
            env,
            d: input.data,
            skills: input.skills,
            cards: input.cards,
            job: input.job,
            cfg,
            cache: Cache::new(n),
            trainees: roster.trainees,
            static_ceilings,
            failed_sets,
            star_mix,
            win_chances: std::collections::HashMap::default(),
            n: Counters::default(),
            members: roster.members,
            fastc,
            fixed,
            record_audits: options.record_audits,
            audits: vec![],
            fill_missing_decks: options.fill_missing_decks,
            member_by_trainee: roster.member_by_trainee,
            qs: vec![],
            working_pool: None,
        }
    }
    /// A working copy of `pool` (the planner's `Env::pool(legs)`), equal to it: the one kept by `put_pool` when it was
    /// made from this pool (the same address, length and borrowing rule: the pool a caller passes is borrowed, so
    /// unchanged, or, as the simulator's, only grown), else a clone. A user pushes its extra entries (`Env::pool_push`
    /// builds an entry as `Env::pool` does, so the pool of legs and those) and pops them before `put_pool`
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
    fn key(&self, c: &Cur) -> String {
        let Lineage { trainee, first_parent, second_parent } = c.lineage;
        format!("{}:{},{}", self.d.umas[trainee].id, first_parent, second_parent)
    }
    /// P(all) at chances q with spark scale c through trainee t's cached decks: (value, deck index; usize::MAX: none)
    fn tval(&self, t: crate::uma::UmaIndex, q: &[f64], c: f64, buf: &mut Vec<f64>) -> (f64, usize) {
        // (1 - q per uncertain target, on the stack up to 32 of them)
        let nu = self.env.uncertain_targets.len();
        let (mut xa, xv): ([f64; 32], Vec<f64>);
        let x: &[f64] = if nu <= 32 {
            xa = [0.0; 32];
            for (k, &i) in self.env.uncertain_targets.iter().enumerate() {
                xa[k] = 1.0 - q[i];
            }
            &xa[..nu]
        } else {
            xv = self.env.uncertain_targets.iter().map(|&i| 1.0 - q[i]).collect();
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
    /// trainee t's best cached deck at chances q (spark scale c, ceiling ub) after the refresh rule (module notes), when
    /// it can reach the bar th: (value, deck index); None when it provably cannot
    pub(super) fn deck_for(&mut self, request: DeckRequest, buf: &mut Vec<f64>) -> Option<(f64, usize)> {
        let mut rb = request.ceiling;
        for r in &self.cache.refs[request.trainee] {
            rb = rb.min(self.env.ref_bound(request.trainee, request.inheritance_chances, request.spark_scale, r));
            if rb * (1.0 + 1e-9) < request.threshold {
                return None;
            }
        }
        if request.threshold > 0.0
            && self.env.ruled_out(
                request.trainee,
                request.inheritance_chances,
                request.spark_scale,
                &self.env.ruled[self.env.deck_class[request.trainee]],
                Bar::Below(request.threshold),
            )
        {
            return None;
        }
        self.n.tables += 1;
        let (mut tv, mut di) = self.tval(request.trainee, request.inheritance_chances, request.spark_scale, buf);
        if di == usize::MAX || rb > tv * (1.0 + self.cfg.margin) {
            self.env.set_trainee(request.trainee, request.inheritance_chances);
            // (her cached decks borrowed: the cache and the Env are separate fields)
            let sr: Vec<&SelectedDeck> = self.cache.decks[request.trainee].iter().map(|x| &x.sel).collect();
            self.env.work = 0;
            let mut sc = 0;
            let r = self.env.best_deck(
                request.trainee,
                request.spark_scale,
                if self.fill_missing_decks && di == usize::MAX { 0.0 } else { request.threshold },
                &sr,
                &mut sc,
            );
            if let Some(s) = self.env.last_product_bound.take() {
                let ct = self.env.deck_class[request.trainee];
                self.env.ruled[ct]
                    .push(ProductProof { inheritance_chances: request.inheritance_chances.to_vec(), product_bound: s });
            }
            self.n.searches += 1;
            if let DeckRes::Done(v, sel, full) = r {
                self.cache.add(self.env, request.trainee, &sel);
                if full {
                    self.cache.refs[request.trainee].push(Ref {
                        q: request.inheritance_chances.to_vec(),
                        val: v,
                        deck: sel,
                    });
                } else {
                    self.n.partial_searches += 1;
                }
                (tv, di) = self.tval(request.trainee, request.inheritance_chances, request.spark_scale, buf);
            }
        }
        if di == usize::MAX { None } else { Some((tv, di)) }
    }
    /// `Env::ceiling` of a candidate lineage without allocations (the same slots, affinities and formula as `slots`,
    /// `inherit` and `outfit_fast`; equal up to rounding, so it only filters, with a slack)
    fn fast_ub(&self, st: &PathState, lineage: Lineage) -> f64 {
        let Lineage { trainee: t, first_parent: a, second_parent: b } = lineage;
        let env = &*self.env;
        let p = &st.pool;
        let ntar = env.target_count;
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
        env.walk_slots(p, a, b, &RelOf { env, p, t }, &mut ub);
        ub.flush();
        let (miss, nc) = (ub.miss, ub.nc);
        let mut c = 1.0f64;
        for k in 0..ntar {
            c *= spark_scale(nc[k]);
        }
        let mut best = 0.0f64;
        for ab in &self.fastc[t] {
            let mut v = 1.0f64;
            for (j, &i) in env.uncertain_targets.iter().enumerate() {
                v *= ab[j].0 - ab[j].1 * miss[i] * miss[i];
            }
            if v > best {
                best = v;
            }
        }
        env.space.rho_top() * c * self.fixed * best
    }
    /// the value a career of trainee t must beat to matter: the best overall, or under `trace` a member's own best
    fn bar(&self, st: &PathState, t: crate::uma::UmaIndex) -> f64 {
        match self.member_by_trainee.get(t).copied().flatten() {
            Some(k) => st.member_best[k].map_or(0.0, |p| p.value),
            None => st.best.map_or(0.0, |p| p.value),
        }
    }
    /// value a candidate lineage against the best so far
    fn consider(&mut self, st: &mut PathState, lineage: Lineage, buf: &mut Vec<f64>) {
        let bar = self.bar(st, lineage.trainee);
        self.n.candidates += 1;
        let mut q = std::mem::take(&mut self.qs);
        let c = self.env.lineage_into(&st.pool, lineage, &mut q);
        self.consider_at(st, lineage, &q, c, bar, buf);
        self.qs = q;
    }
    /// `consider` at the candidate's chances q and spark scale c, against `bar`
    fn consider_at(&mut self, st: &mut PathState, lineage: Lineage, q: &[f64], c: f64, bar: f64, buf: &mut Vec<f64>) {
        let t = lineage.trainee;
        let Some(ub) = self.env.ceiling(t, q, c) else { return };
        static CHECK_UB: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *CHECK_UB.get_or_init(|| std::env::var("SIM_CHECK_UB").is_ok()) {
            let f = self.fast_ub(st, lineage);
            assert!((f - ub).abs() <= 1e-12 * ub, "fast_ub {f} vs ceiling {ub}");
        }
        if ub * (1.0 + 1e-9) < bar {
            return;
        }
        self.n.ceiling_passes += 1;
        let Some((tv, di)) = self.deck_for(
            DeckRequest { trainee: t, inheritance_chances: q, spark_scale: c, ceiling: ub, threshold: bar },
            buf,
        ) else {
            return;
        };
        let pick = SurrogatePick { value: tv, lineage: lineage.ordered(), deck: di };
        if let Some(k) = self.member_by_trainee.get(t).copied().flatten()
            && tv > bar
        {
            st.member_best[k] = Some(pick);
            st.current = None;
        }
        if tv > st.best.map_or(0.0, |p| p.value) {
            st.best = Some(pick);
            st.current = None;
        }
    }
    fn legal(&self, st: &PathState, lineage: Lineage) -> bool {
        st.pool.pair_ok(self.d.umas[lineage.trainee].id, lineage.first_parent, lineage.second_parent)
    }
    /// her chance per race of at least one win, cached per (trainee, the parents' wins)
    pub(super) fn wins(&mut self, pool: &Pool, lineage: Lineage) -> Rc<Vec<f64>> {
        let mut k: Vec<usize> = pool.veterans[lineage.first_parent]
            .wins
            .iter()
            .chain(&pool.veterans[lineage.second_parent].wins)
            .map(|w| w.race)
            .collect();
        k.sort_unstable();
        // her pink sparks change her win chances: their inputs join the key (start grades, each spark's aptitude and chance)
        if let Some(pc) = self.env.pink_career(pool, lineage) {
            k.push(usize::MAX);
            k.extend(pc.start.iter().map(|&g| g as usize));
            for f in &pc.fire {
                k.push(f.apt);
                k.push((f.p * 1e9).round() as usize);
            }
        }
        if let Some(x) = self.win_chances.get(&(lineage.trainee, k.clone())) {
            return Rc::clone(x);
        }
        let w = Rc::new(self.env.schedule(pool, lineage).win_chances);
        self.win_chances.insert((lineage.trainee, k), Rc::clone(&w));
        w
    }
    /// the career with its deck: its exact numbers; `rotation`: a loop rotation career (`Cur::rotation`)
    pub(super) fn make_cur(&mut self, pool: &Pool, career: Career, rotation: bool) -> Cur {
        let lineage = career.lineage;
        let t = lineage.trainee;
        let Inheritance { chances: q, carriers: nc } = inherit(&self.env.slots(pool, lineage), self.env.target_count);
        self.env.set_trainee(t, &q);
        let exact = self.env.exact(t, career.deck, &nc, ExactOptions { all_but_one: true, landed_sets: true });
        let os = self.env.outcome_sets(exact.outfit, &exact.ext);
        let wp = self.wins(pool, lineage);
        Cur { lineage, p_all: exact.p_all, os, wp, rotation }
    }
}
