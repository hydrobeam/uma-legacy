//! Policy simulator on a surrogate (`simulate` on a career job; docs/SOLVER.md "Policy simulator").
//!
//! Under the model, the expected number of careers until the first career whose veteran lands every target, for a
//! decision policy on a fixed starting pool. One process runs every career of every run, so nothing is re-read or
//! re-searched between careers. Per run (derived):
//!
//!   E = sum over careers t of S_t,   S_0 = 1,   S_{t+1} = S_t (1 - p_t)
//!
//! with p_t the exact P(all) of career t; the path continues through failures only (each failed veteran drawn from the
//! career's exact landed-set distribution with the all-landed set removed), and stops when S_t < eps or at
//! `max_careers`, where the tail S_t / p_t (career t repeated forever) is added.
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
//! `index` and the worker's kept state: a plan job reads and leaves the kept plans (`plan::BESTS`), proofs (`SEEN`),
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
use super::cache::Cache;
use super::inherit::{ExactOptions, Inheritance, LineageChances};
use super::{
    Ancestor, Bar, CardsRaw, Data, DeckRes, Deserialize, Env, Found, FxMap, GEN_GOLD, HerSlot, Legacy, PROF, Pool, Ref,
    RelOf, Sel, SkillJob, SkillsRaw, SlotSink, Solved, Value, Who, aff_factor, d_true, holds_fix, inherit, insp_miss,
    interp_in, json, now_ms, prof, progress, rise_factor, run_career, spark_scale,
};
use crate::deck::career::sim::plan::ScoringContext;
use crate::deck::career::{Career, JobInputs, Lineage};
use std::rc::Rc;
/// the page's next career (`next` on a career job)
pub(super) mod plan;

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
    margin: f64,
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
    bests: Vec<plan::Bests>,
    seen: Vec<(u64, Vec<Vec<Ref>>)>,
    searches: Vec<(u64, Solved, super::Ruled)>,
    decks: Vec<(u64, super::Kept)>,
}
/// Replace the worker's kept state, returning the previous state (`index`: module notes).
fn kept_swap(replacement: KeptState) -> KeptState {
    let bests = plan::bests_take();
    plan::bests_set(replacement.bests);
    let seen = super::SEEN.with(|cache| std::mem::replace(&mut *cache.borrow_mut(), replacement.seen));
    let searches = super::LAST_SOLVE.with(|cache| std::mem::replace(&mut *cache.borrow_mut(), replacement.searches));
    let decks = super::DECK_KEEP.with(|cache| std::mem::replace(&mut *cache.borrow_mut(), replacement.decks));
    KeptState { bests, seen, searches, decks }
}

// ---- random numbers ----
const fn fmix(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}
/// 53 bits in [0, 1) from the keys
pub(super) fn u01(keys: &[u32]) -> f64 {
    let (mut a, mut b) = (0x9e37_79b9u32, 0x7f4a_7c15u32);
    for &k in keys {
        a = fmix((a ^ k).wrapping_add(0x632b_e5ab));
        b = fmix(b.wrapping_add(k.wrapping_mul(0x27d4_eb2f)) ^ a);
    }
    (f64::from(a) * 2f64.powi(21) + f64::from(b >> 11)) / 2f64.powi(53)
}
const STREAM_SET: u32 = 1;
const STREAM_STAR: u32 = 2;
const STREAM_WIN: u32 = 3;
/// Whose uniforms: a run's own path
#[derive(Clone, Copy)]
enum Keys {
    Main { seed: u32, run: u32 },
}
impl Keys {
    fn u(self, t: usize, stream: u32, item: usize) -> f64 {
        match self {
            Self::Main { seed, run } => u01(&[seed, run, t as u32, stream, item as u32]),
        }
    }
}

/// The best career so far by the surrogate (`Sim::consider_at`).
#[derive(Clone, Copy)]
struct Pick {
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
    os: Vec<f64>,
    /// Her chance of at least one win, per race index.
    wp: Rc<Vec<f64>>,
    /// A loop rotation career: her veteran may replace her member's rotation veteran (`rot_update`).
    rotation: bool,
}
/// Loop rotation: the next member's position and each member's latest rotation veteran
#[derive(Clone, Default)]
struct Rotation {
    /// Careers run so far; the next member is this modulo the member count.
    position: usize,
    /// Per member, her rotation veteran's legacy index.
    latest: Vec<Option<usize>>,
}
/// A veteran's stars per job target and her distinct G1 wins, sorted: what dominance pruning compares.
#[derive(Clone)]
struct VeteranResult {
    stars: Vec<u8>,
    wins: Vec<usize>,
}
/// One path's state: veterans, the solver's pool, owned veterans per family (dominance), the surrogate's best
#[derive(Clone)]
struct PathState {
    legacies: Vec<Legacy>,
    pool: Pool,
    /// Per family (`family`), the results of the owned veterans in it.
    families: crate::FxMap<String, Vec<VeteranResult>>,
    best: Option<Pick>,
    /// The career the policy runs next, kept until the state changes.
    current: Option<Rc<Cur>>,
    rotation: Rotation,
    /// `trace`: each loop member's best career (by member index)
    member_best: Vec<Option<Pick>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Base {
    Greedy,
    Loop,
    Trace,
    Index,
}
/// The simulator's work counters (`Sim::n`).
#[derive(Default, Clone, Copy)]
struct Counters {
    /// Candidate careers valued.
    candidates: usize,
    /// Candidates whose ceiling cleared the bar.
    ceiling_passes: usize,
    /// Cached-table valuations (`deck_for`).
    tables: usize,
    /// Deck searches ("refreshes").
    searches: usize,
    /// Deck searches that did not finish (no `Ref`).
    partial_searches: usize,
    /// Veterans added to a path's pool.
    veterans_added: usize,
}

/// `trace`: a builder's best career and what it does for the trainee built for (A, best odds p).
#[derive(Clone, Copy)]
struct BuilderCareer {
    /// lam (1/p - 1/p1): the expected fall in A's careers needed, per career run.
    efficiency: f64,
    pick: Pick,
    /// lam: the chance her veteran raises A's best odds.
    lift_chance: f64,
    /// p1: A's mean odds when raised (p when never).
    lifted_odds: f64,
}

/// Owned roster tables moved into a simulator.
struct SimulationRoster {
    /// Trainees considered by later simulated careers, in search order.
    trainees: Vec<usize>,
    /// Rotation members in the supplied order.
    members: Vec<usize>,
    /// Rotation position for each Data::umas index, when tracing members.
    member_by_trainee: Vec<Option<usize>>,
}

/// Independent simulator behaviours selected by its caller.
#[derive(Clone, Copy)]
struct SimulationOptions {
    /// Record audit output for the main simulation.
    record_audits: bool,
    /// Search for decks absent from the simulator cache.
    fill_missing_decks: bool,
}

pub(crate) struct Sim<'env, 'data> {
    env: &'env mut Env<'data>,
    d: &'data Data,
    skills: &'data SkillsRaw,
    cards: &'data CardsRaw,
    job: &'data Value,
    cfg: SimJob,
    cache: Cache,
    trainees: Vec<usize>,
    /// Per uma, her static ceiling (q = 1 on every target; 0 when she is not owned)
    static_ceilings: Vec<f64>,
    /// The failed landed sets (every set but the full one), fewest landed first, ties by mask: the draw's order
    failed_sets: Vec<usize>,
    /// The rank's 1/2/3-star shares of a landed target's spark
    star_mix: [f64; 3],
    /// Win chances per (trainee, her parents' wins and pink inputs) (`wins`)
    win_chances: crate::FxMap<(usize, Vec<usize>), Rc<Vec<f64>>>,
    n: Counters,
    members: Vec<usize>,
    /// per trainee and outfit, per uncertain target (A, B): her ceiling factor is A - B (1 - q) (`fast_ub`)
    fastc: Vec<Vec<Vec<(f64, f64)>>>,
    fixed: f64,
    /// trace: trainee → loop member index (empty under the other policies)
    member_by_trainee: Vec<Option<usize>>,
    /// the simulator's own path (false when the plan uses this machinery): audits are taken only there
    record_audits: bool,
    audits: Vec<Value>,
    /// the plan only (false for every policy here): a trainee without a cached deck gets her best deck searched without
    /// a bar (so she has one from then on), instead of a search at the bar that caches nothing when it ends Bounded
    fill_missing_decks: bool,
    /// a lineage's chances, reused (`lineage_into`)
    qs: Vec<f64>,
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
        Self { address: pool as *const Pool as usize, veterans: pool.me.len(), max_borrowed: pool.max_bor }
    }
}
/// `index`, this run: what its plan jobs did.
#[derive(Default)]
struct IndexRunCounts {
    /// Plan jobs run.
    plans: usize,
    /// Each plan job's seconds, rounded to the millisecond.
    plan_seconds: Vec<f64>,
    /// Per role of the chosen option (`next.options[].role`), how many picks had it.
    roles: std::collections::BTreeMap<String, usize>,
    /// Picks that were the best-odds career (`best_odds`).
    best_odds_picks: usize,
}
thread_local! { static IX: std::cell::RefCell<IndexRunCounts> = std::cell::RefCell::new(IndexRunCounts::default()); }

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
fn tq975(df: f64) -> f64 {
    let z = 1.959_964_f64;
    let (z3, z5, z7) = (z.powi(3), z.powi(5), z.powi(7));
    z + (z3 + z) / (4.0 * df)
        + (5.0 * z5 + 16.0 * z3 + 3.0 * z) / (96.0 * df * df)
        + (3.0 * z7 + 19.0 * z5 + 17.0 * z3 - 15.0 * z) / (384.0 * df.powi(3))
}
fn stats(xs: &[f64]) -> Value {
    let n = xs.len() as f64;
    if xs.is_empty() {
        return Value::Null;
    }
    let mean = xs.iter().sum::<f64>() / n;
    let sd = if xs.len() > 1 { (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt() } else { 0.0 };
    let half = if xs.len() > 1 { tq975(n - 1.0) * sd / n.sqrt() } else { 0.0 };
    json!({"n": xs.len(), "mean": mean, "sd": sd, "ci95": [mean - half, mean + half], "min": xs.iter().copied().fold(f64::INFINITY, f64::min), "max": xs.iter().copied().fold(f64::NEG_INFINITY, f64::max)})
}

/// Per trainee and outfit, per uncertain target (A, B): her ceiling factor is A - B (1 - q) (`Sim::fastc`; none
/// without legal deck odds). `outfit_fast` at q = 0 on the hint chances `set_trainee` would write, split: A = dg GOLD +
/// (1 - dg) WHITE and B = (1 - dg) WHITE (1 - dw)(1 - e) where she lacks the skill, B = 0 where she has it
fn fastc(env: &Env) -> Vec<Vec<Vec<(f64, f64)>>> {
    (0..env.d.umas.len())
        .map(|t| match &env.dmax[t] {
            None => vec![],
            Some((dw, dg)) => env.sts0[t]
                .iter()
                .zip(&env.ew0[t])
                .map(|(sv, ew)| {
                    (0..env.nu)
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
struct DeckRequest<'chances> {
    /// Trainee's position in Data::umas.
    trainee: usize,
    /// Inheritance probability per job target.
    inheritance_chances: &'chances [f64],
    /// Product of inheritance spark multipliers.
    spark_scale: f64,
    /// Upper bound on this lineage's success probability.
    ceiling: f64,
    /// Minimum success probability worth searching.
    threshold: f64,
}

impl<'env, 'data> Sim<'env, 'data> {
    /// The simulator's machinery for a job (`simulate`'s, and the plan's through `plan::sim_for`): the landed sets in
    /// order of size, each owned trainee's ceiling at q = 1 (`static_ceilings`), the rank's star mix, `fastc`, the
    /// fixed gold factor
    fn new(
        env: &'env mut Env<'data>,
        input: &JobInputs<'data>,
        cfg: SimJob,
        roster: SimulationRoster,
        options: SimulationOptions,
    ) -> Self {
        let ntar = env.ntar;
        let full = (1usize << ntar) - 1;
        let mut failed_sets: Vec<usize> = (0..full).collect();
        failed_sets.sort_by_key(|&m| (m.count_ones(), m));
        let n = input.data.umas.len();
        let one = vec![1.0f64; ntar];
        let static_ceilings: Vec<f64> = (0..n)
            .map(|t| if env.owned.contains(&t) { env.ceiling(t, &one, 1.0).unwrap_or(0.0) } else { 0.0 })
            .collect();
        let star_mix = env.sj.star_dist;
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
        debug_assert_eq!(p.me.len(), pool.me.len(), "put_pool takes back the pool take_pool lent");
        self.working_pool = Some((PoolIdentity::of(pool), p));
    }
    fn key(&self, c: &Cur) -> String {
        let Lineage { trainee, first_parent, second_parent } = c.lineage;
        format!("{}:{},{}", self.d.umas[trainee].id, first_parent, second_parent)
    }
    /// P(all) at chances q with spark scale c through trainee t's cached decks: (value, deck index; usize::MAX: none)
    fn tval(&self, t: usize, q: &[f64], c: f64, buf: &mut Vec<f64>) -> (f64, usize) {
        // (1 - q per uncertain target, on the stack up to 32 of them)
        let nu = self.env.uncv.len();
        let (mut xa, xv): ([f64; 32], Vec<f64>);
        let x: &[f64] = if nu <= 32 {
            xa = [0.0; 32];
            for (k, &i) in self.env.uncv.iter().enumerate() {
                xa[k] = 1.0 - q[i];
            }
            &xa[..nu]
        } else {
            xv = self.env.uncv.iter().map(|&i| 1.0 - q[i]).collect();
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
    fn deck_for(&mut self, request: DeckRequest, buf: &mut Vec<f64>) -> Option<(f64, usize)> {
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
                &self.env.ruled[self.env.cls[request.trainee]],
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
            let sr: Vec<&Sel> = self.cache.decks[request.trainee].iter().map(|x| &x.sel).collect();
            self.env.work = 0;
            let mut sc = 0;
            let r = self.env.best_deck(
                request.trainee,
                request.spark_scale,
                if self.fill_missing_decks && di == usize::MAX { 0.0 } else { request.threshold },
                &sr,
                &mut sc,
            );
            if let Some(s) = self.env.last_pv.take() {
                let ct = self.env.cls[request.trainee];
                self.env.ruled[ct].push((request.inheritance_chances.to_vec(), s));
            }
            self.n.searches += 1;
            if let DeckRes::Done(v, sel, full) = r {
                self.cache.add(self.env, request.trainee, &sel);
                if full {
                    self.cache.refs[request.trainee].push(Ref { q: request.inheritance_chances.to_vec(), val: v, sel });
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
        let ntar = env.ntar;
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
            for (j, &i) in env.uncv.iter().enumerate() {
                v *= ab[j].0 - ab[j].1 * miss[i] * miss[i];
            }
            if v > best {
                best = v;
            }
        }
        env.space.rho_top() * c * self.fixed * best
    }
    /// the value a career of trainee t must beat to matter: the best overall, or under `trace` a member's own best
    fn bar(&self, st: &PathState, t: usize) -> f64 {
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
        let pick = Pick { value: tv, lineage: lineage.ordered(), deck: di };
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
        let mut k: Vec<usize> = pool.me[lineage.first_parent]
            .wins
            .iter()
            .chain(&pool.me[lineage.second_parent].wins)
            .map(|w| w.0)
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
        let Inheritance { chances: q, carriers: nc } = inherit(&self.env.slots(pool, lineage), self.env.ntar);
        self.env.set_trainee(t, &q);
        let exact = self.env.exact(t, career.deck, &nc, ExactOptions { all_but_one: true, landed_sets: true });
        let os = self.env.outcome_sets(exact.outfit, &exact.ext);
        let wp = self.wins(pool, lineage);
        Cur { lineage, p_all: exact.p_all, os, wp, rotation }
    }
    /// a cached pick's career (her cached deck cloned out of the cache)
    fn pick_cur(&mut self, pool: &Pool, pick: Pick, rotation: bool) -> Cur {
        let deck = self.cache.decks[pick.lineage.trainee][pick.deck].sel.clone();
        self.make_cur(pool, Career { lineage: pick.lineage, deck: &deck }, rotation)
    }
    fn greedy_cur(&mut self, st: &mut PathState) -> Rc<Cur> {
        if let Some(c) = &st.current {
            return Rc::clone(c);
        }
        let pick = st.best.expect("no legal career");
        let c = Rc::new(self.pick_cur(&st.pool, pick, false));
        st.current = Some(Rc::clone(&c));
        c
    }
    /// `trace`: building for loop_members[0] (A) with the others as builders. Each career: per builder her best career's
    /// exact chance of raising A's best odds (lam) and mean odds raised to (p1); the most efficient (lam (1/p - 1/p1)) runs,
    /// her veteran joins; recorded per career: A's odds before, the builder, lam, p1, and A's odds after
    fn trace(&mut self, st0: &PathState) -> Value {
        let a = self.members[0];
        let nb = self.members.len();
        let mut out = vec![];
        for run in self.cfg.first..self.cfg.first + self.cfg.runs {
            let mut st = st0.clone();
            let keys = Keys::Main { seed: self.cfg.seed, run: run as u32 };
            let mut buf = vec![];
            let mut rows = vec![];
            let tm = std::time::Instant::now();
            for t in 0..self.cfg.max_careers {
                let Some(pka) = st.member_best[0] else { break };
                let pa = pka.value;
                // this career's scoring of the builders: A alone, at her best odds now, on this career's pool
                let one = [a];
                let pw = [pa];
                let base = plan::ScoreBase {
                    cands: &one,
                    c1: &[1.0],
                    m23: &[0.0],
                    v0: 0.0,
                    pnow: &pw,
                    cont: &[],
                    combos: &[],
                    sides: &[],
                    all: &[],
                };
                let scoring = ScoringContext { score_base: &base, legacies: &st.legacies, pool: &st.pool };
                let mut best_builder: Option<BuilderCareer> = None;
                for k in 1..nb {
                    let Some(pk) = st.member_best[k] else { continue };
                    let sel = self.cache.decks[pk.lineage.trainee][pk.deck].sel.clone();
                    let plan::CareerScore { lift_stats: stt, .. } = plan::score_career_of(
                        self,
                        &scoring,
                        Career { lineage: pk.lineage, deck: &sel },
                        plan::Want::Stats,
                        &mut buf,
                    );
                    let (pl, sp) = stt[0];
                    let (e, p1) = if pl > 0.0 {
                        let p1 = sp / pl;
                        (pl * (1.0 / pa - 1.0 / p1), p1)
                    } else {
                        (0.0, pa)
                    };
                    if best_builder.is_none_or(|b| e > b.efficiency) {
                        best_builder =
                            Some(BuilderCareer { efficiency: e, pick: pk, lift_chance: pl, lifted_odds: p1 });
                    }
                }
                let Some(builder) = best_builder else { break };
                let cur = self.pick_cur(&st.pool, builder.pick, false);
                let vet = self.draw(&cur, &st.legacies, keys, t);
                self.add_vet(&mut st, vet, true, &mut buf);
                let after = st.member_best[0].map_or(pa, |p| p.value);
                rows.push(json!([
                    t,
                    pa,
                    self.d.umas[builder.pick.lineage.trainee].name,
                    builder.lift_chance,
                    builder.lifted_odds,
                    after
                ]));
            }
            eprintln!(
                "trace run {}: {} careers, A from {:.3e} to {:.3e}, {:.1} s",
                run,
                rows.len(),
                st0.member_best[0].map_or(0.0, |p| p.value),
                st.member_best[0].map_or(0.0, |p| p.value),
                tm.elapsed().as_secs_f64()
            );
            out.push(json!({"run": run, "rows": rows}));
        }
        json!({"trace": out, "trainee": self.d.umas[a].name, "builders": self.members[1..].iter().map(|&u| self.d.umas[u].name.clone()).collect::<Vec<_>>()})
    }
    /// the loop's next career (module notes: rotation and bootstrap)
    fn loop_cur(&mut self, st: &mut PathState) -> Rc<Cur> {
        if let Some(c) = &st.current {
            return Rc::clone(c);
        }
        let nm = self.members.len();
        let pos = st.rotation.position % nm;
        let t = self.members[pos];
        let prev: Vec<usize> = [1, 2].iter().filter_map(|&k| st.rotation.latest[(pos + nm - k) % nm]).collect();
        let c = if prev.len() == 2 {
            let lineage =
                Lineage { trainee: t, first_parent: prev[0].min(prev[1]), second_parent: prev[0].max(prev[1]) };
            let (q, c) = self.env.lineage_q(&st.pool, lineage);
            let ub = self.env.ceiling(t, &q, c).unwrap_or(f64::INFINITY);
            let mut buf = vec![];
            let (_, di) = self
                .deck_for(
                    DeckRequest { trainee: t, inheritance_chances: &q, spark_scale: c, ceiling: ub, threshold: 0.0 },
                    &mut buf,
                )
                .expect("loop career without a deck");
            let sel = self.cache.decks[t][di].sel.clone();
            self.make_cur(&st.pool, Career { lineage, deck: &sel }, true)
        } else {
            self.env.use_seen = true;
            let sv = self.env.solve(&st.pool, &prev, &[t], false, 1);
            let f = sv.found.first().expect("loop bootstrap: no legal career for a member");
            self.cache.absorb(self.env, &sv.refs);
            self.make_cur(&st.pool, Career { lineage: f.lineage(), deck: &f.sel }, true)
        };
        let c = Rc::new(c);
        st.current = Some(Rc::clone(&c));
        c
    }
    /// The loop careers veteran v of member m is a parent of (members m + 1 and m + 2, the other parent their current
    /// rotation veteran; a term without one, or with her own character, is left out), in that order
    fn rotation_careers(&self, st: &PathState, m: usize, v: usize) -> Vec<Lineage> {
        let nm = self.members.len();
        let rotation = &st.rotation;
        let mut careers = vec![];
        for k in 1..=2 {
            let t = self.members[(m + k) % nm];
            let other = if k == 1 { rotation.latest[(m + nm - 1) % nm] } else { rotation.latest[(m + 1) % nm] };
            let Some(o) = other else { continue };
            if o == v || st.pool.me[o].chara == st.pool.me[v].chara {
                continue;
            }
            careers.push(Lineage { trainee: t, first_parent: v.min(o), second_parent: v.max(o) });
        }
        careers
    }
    /// The loop's rotation after a member's career (her veteran the pool's last): the member keeps the better of her
    /// rotation veteran and the new one, by the sum of the surrogate P(all) of the two loop careers she is a parent of
    /// (`rotation_careers`, for both); the first one she has is kept as is. Then the next member trains.
    fn rot_update(&mut self, st: &mut PathState) {
        let nm = self.members.len();
        let m = st.rotation.position % nm;
        let vi = st.legacies.len() - 1;
        st.rotation.position += 1;
        st.current = None;
        let Some(old) = st.rotation.latest[m] else {
            st.rotation.latest[m] = Some(vi);
            return;
        };
        let mut buf = vec![];
        let mut surrogate = |s: &mut Self, v: usize| -> f64 {
            let mut tot = 0.0;
            for lineage in s.rotation_careers(st, m, v) {
                let t = lineage.trainee;
                let (q, c) = s.env.lineage_q(&st.pool, lineage);
                let ub = s.env.ceiling(t, &q, c).unwrap_or(f64::INFINITY);
                tot += s
                    .deck_for(
                        DeckRequest {
                            trainee: t,
                            inheritance_chances: &q,
                            spark_scale: c,
                            ceiling: ub,
                            threshold: 0.0,
                        },
                        &mut buf,
                    )
                    .map_or(0.0, |x| x.0);
            }
            tot
        };
        let (so, sn) = (surrogate(self, old), surrogate(self, vi));
        if self.cfg.audit && self.record_audits && st.rotation.position.is_multiple_of(self.cfg.audit_every) {
            // the keep-or-replace decision under exact values (a full deck search per term)
            let exact = |s: &mut Self, v: usize| -> f64 {
                let mut tot = 0.0;
                for lineage in s.rotation_careers(st, m, v) {
                    let (q, c) = s.env.lineage_q(&st.pool, lineage);
                    tot += s.exact_best(lineage.trainee, &q, c).value;
                }
                tot
            };
            let (eo, en) = (exact(self, old), exact(self, vi));
            self.audits.push(json!({"t": st.rotation.position, "rotation": [so, sn, eo, en, (sn > so) == (en > eo)]}));
        }
        if sn > so {
            st.rotation.latest[m] = Some(vi);
        }
    }
    /// Builds a runnable career from a selected internal solver result.
    /// The caller checks that `pick` is an object. Its required fields supply the
    /// exact probabilities; resolving the win chances also fills the schedule cache.
    fn cur_from_solver_pick(&mut self, pool: &Pool, pick: &Value) -> Cur {
        let trainee = self.env.uix(pick["trainee"]["id"].as_u64().unwrap() as u32).unwrap();
        let (parent_a, parent_b) =
            (pick["parents"][0].as_u64().unwrap() as usize, pick["parents"][1].as_u64().unwrap() as usize);
        let outcome_sets: Vec<f64> =
            pick["outcome_sets"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        let lineage = Lineage { trainee, first_parent: parent_a, second_parent: parent_b };
        let win_probs = self.wins(pool, lineage);
        Cur { lineage, p_all: pick["p_all"].as_f64().unwrap(), os: outcome_sets, wp: win_probs, rotation: false }
    }
    /// the in-process career search's pick (greedy_exact: the best career by P(all))
    fn exact_cur(&mut self, st: &PathState) -> Cur {
        let mut job = self.job.clone();
        let o = job.as_object_mut().unwrap();
        o.remove("simulate");
        o.insert("legacies".into(), Value::Array(st.legacies.iter().map(leg_json).collect()));
        o.insert("top".into(), json!(1));
        o.insert("per_trainee".into(), json!(false));
        let out = run_career(self.d, self.skills, self.cards, &job);
        let pick = out["results"][0].clone();
        assert!(pick.is_object(), "solver gave no pick: {}", out.to_string().chars().take(300).collect::<String>());
        self.cur_from_solver_pick(&st.pool, &pick)
    }
    /// `index`: the page's plan for this pool (the career job with the pool as legacies, no places, the page's `top`,
    /// `next`, `no_reports`: the plan job the page sends, docs/SOLVER.md "Next career"), in process; its `choice` runs.
    /// The job's `trainees` stay (the page sends none: every owned uma)
    fn index_cur(&mut self, st: &PathState) -> Cur {
        let mut job = self.job.clone();
        let o = job.as_object_mut().unwrap();
        o.remove("simulate");
        o.insert("legacies".into(), Value::Array(st.legacies.iter().map(leg_json).collect()));
        o.insert("fix".into(), json!([]));
        o.insert("top".into(), json!(PAGE_TOP));
        o.insert("per_trainee".into(), json!(false));
        o.insert("next".into(), self.cfg.next.clone());
        o.insert("no_reports".into(), json!(true));
        let tm = std::time::Instant::now();
        let out = run_career(self.d, self.skills, self.cards, &job);
        let pick = out["next"]["choice"].clone();
        assert!(
            pick.is_object(),
            "index: the plan gave no pick: {}",
            out.to_string().chars().take(300).collect::<String>()
        );
        IX.with(|ix| {
            let mut ix = ix.borrow_mut();
            ix.plans += 1;
            ix.plan_seconds.push((tm.elapsed().as_secs_f64() * 1e3).round() / 1e3);
            if let Some(c) =
                out["next"]["options"].as_array().and_then(|v| v.iter().find(|x| x["chosen"].as_bool() == Some(true)))
            {
                *ix.roles.entry(c["role"].as_str().unwrap_or("?").to_string()).or_default() += 1;
                if c["best_odds"].as_bool() == Some(true) {
                    ix.best_odds_picks += 1;
                }
            }
        });
        self.cur_from_solver_pick(&st.pool, &pick)
    }
    /// the veteran career number t leaves when it fails: her landed set drawn from the failed sets, her stars from the
    /// rank's mix, her wins from her schedule's chances
    fn draw(&self, c: &Cur, legs: &[Legacy], keys: Keys, t: usize) -> Legacy {
        let tot: f64 = self.failed_sets.iter().map(|&m| c.os[m]).sum();
        let mut u = keys.u(t, STREAM_SET, 0) * tot;
        let (mut mask, mut last) = (usize::MAX, usize::MAX);
        for &m in &self.failed_sets {
            if c.os[m] <= 0.0 {
                continue;
            }
            last = m;
            if u < c.os[m] {
                mask = m;
                break;
            }
            u -= c.os[m];
        }
        if mask == usize::MAX {
            mask = last;
        }
        let mut sparks = vec![];
        for (i, tg) in self.env.sj.targets.iter().enumerate() {
            if mask >> i & 1 == 0 {
                continue;
            }
            let x = keys.u(t, STREAM_STAR, i);
            let s = if x < self.star_mix[0] {
                1
            } else if x < self.star_mix[0] + self.star_mix[1] {
                2
            } else {
                3
            };
            sparks.push((tg.group, s));
        }
        let wins: Vec<usize> =
            c.wp.iter()
                .enumerate()
                .filter(|&(ri, &p)| p > 0.0 && keys.u(t, STREAM_WIN, ri) < p)
                .map(|(ri, _)| ri)
                .collect();
        let Lineage { trainee, first_parent, second_parent } = c.lineage;
        Legacy {
            chara: self.d.umas[trainee].id,
            borrowed: false,
            sparks,
            wins,
            pinks: vec![],
            parents: vec![Ancestor::of_legacy(&legs[first_parent]), Ancestor::of_legacy(&legs[second_parent])],
            ..Default::default()
        }
    }
    fn veteran_result(&self, l: &Legacy) -> VeteranResult {
        let stars = self
            .env
            .sj
            .targets
            .iter()
            .map(|t| l.sparks.iter().filter(|s| s.0 == t.group).map(|s| s.1).max().unwrap_or(0))
            .collect();
        let mut wins = l.wins.clone();
        wins.sort_unstable();
        wins.dedup();
        VeteranResult { stars, wins }
    }
    /// whether an owned veteran of her family has at least her stars on every target and every G1 she won
    fn dominated(&self, st: &PathState, l: &Legacy) -> bool {
        let new = self.veteran_result(l);
        st.families.get(&family(l)).is_some_and(|owned| {
            owned.iter().any(|old| {
                old.stars.iter().zip(&new.stars).all(|(x, y)| x >= y)
                    && new.wins.iter().all(|r| old.wins.binary_search(r).is_ok())
            })
        })
    }
    /// a veteran joins the pool; `eval`: the surrogate values every new candidate (she is one of its parents)
    fn add_vet(&mut self, st: &mut PathState, l: Legacy, eval: bool, buf: &mut Vec<f64>) {
        // (her entry as `Env::pool` builds it, win chances and star mix kept)
        self.env.pool_push(&mut st.pool, &l);
        if !l.borrowed {
            let result = self.veteran_result(&l);
            st.families.entry(family(&l)).or_default().push(result);
        }
        st.legacies.push(l);
        self.n.veterans_added += 1;
        if !eval {
            return;
        }
        let vi = st.legacies.len() - 1;
        let trainees = self.trainees.clone();
        for &t in &trainees {
            if self.static_ceilings[t] * (1.0 + 1e-9) < self.bar(st, t) {
                continue;
            }
            for b in 0..vi {
                let lineage = Lineage { trainee: t, first_parent: b, second_parent: vi };
                if !self.legal(st, lineage) {
                    continue;
                }
                // the allocation-free ceiling first (`fast_ub`, the same bound to rounding)
                if self.fast_ub(st, lineage) * (1.0 + 1e-6) < self.bar(st, t) {
                    self.n.candidates += 1;
                    continue;
                }
                self.consider(st, lineage, buf);
            }
        }
    }
    /// the initial state: the job's pool, every candidate valued (best ceiling first)
    fn initial(&mut self, legs: &[Legacy], base: Base) -> PathState {
        let mut st = PathState {
            legacies: vec![],
            pool: self.env.pool(&[]),
            families: std::collections::HashMap::default(),
            best: None,
            current: None,
            rotation: Rotation { position: 0, latest: vec![None; self.members.len()] },
            member_best: vec![None; self.members.len()],
        };
        let mut buf = vec![];
        for l in legs {
            self.add_vet(&mut st, l.clone(), false, &mut buf);
        }
        if !matches!(base, Base::Loop | Base::Index) {
            // seed the cache with the career search's fully checked decks
            self.env.use_seen = false;
            let trainees = self.trainees.clone();
            // (on the path's pool, `Env::pool(legs)`: `add_vet` builds each entry as it does)
            let sv = self.env.solve(&st.pool, &[], &trainees, false, 1);
            self.env.use_seen = true;
            self.cache.absorb(self.env, &sv.refs);
            // (every legal career's ceiling, `gen_cands`: `ceiling` at `lineage`'s chances, to the bit; in (trainee,
            // pair) order, then best first)
            let mut candidates = self.env.gen_cands(&st.pool, &[], &trainees, |_, _| true, RankedCareer::new);
            candidates.sort_by(|x, y| y.ceiling.partial_cmp(&x.ceiling).unwrap());
            for candidate in candidates {
                let floor = st
                    .member_best
                    .iter()
                    .fold(st.best.map_or(0.0, |p| p.value), |m, p| m.min(p.map_or(0.0, |p| p.value)));
                if candidate.ceiling * (1.0 + 1e-9) < floor {
                    break;
                }
                if candidate.ceiling * (1.0 + 1e-9) < self.bar(&st, candidate.lineage.trainee) {
                    continue;
                }
                self.consider(&mut st, candidate.lineage, &mut buf);
            }
        }
        st
    }

    /// One audit row: a career valued by the surrogate's table (her cached decks, no refresh) against a fresh exact
    /// deck search there: [kind, table value, exact value, (exact - table) / exact, landed-set total variation between
    /// the two decks, search finished].
    fn audit_row(&mut self, pool: &Pool, lineage: Lineage, kind: AuditKind) -> Value {
        let t = lineage.trainee;
        let LineageChances { chances: q, carriers: nc, spark_scale: c } = self.env.lineage(pool, lineage);
        let mut buf = vec![];
        let (tv, di) = self.tval(t, &q, c, &mut buf);
        let best = self.exact_best(t, &q, c);
        let tvd = if di == usize::MAX {
            1.0
        } else {
            let cached = self.cache.decks[t][di].sel.clone();
            self.landed_set_distance(t, &q, &nc, &cached, &best.deck)
        };
        let exv = best.value;
        json!([kind as u8, tv, exv, if exv > 0.0 { (exv - tv) / exv } else { 0.0 }, tvd, best.finished])
    }
    /// total variation distance between the landed-set distributions of two decks of trainee t at chances q (carriers
    /// nc)
    fn landed_set_distance(&mut self, t: usize, q: &[f64], nc: &[i32], first: &Sel, second: &Sel) -> f64 {
        let mut sets = |sel: &Sel| {
            self.env.set_trainee(t, q);
            let exact = self.env.exact(t, sel, nc, ExactOptions { all_but_one: false, landed_sets: true });
            self.env.outcome_sets(exact.outfit, &exact.ext)
        };
        let s1 = sets(first);
        let s2 = sets(second);
        0.5 * s1.iter().zip(&s2).map(|(x, y)| (x - y).abs()).sum::<f64>()
    }
    /// trainee t's exact best deck at chances q (a full deck search), its value with spark scale c
    fn exact_best(&mut self, t: usize, q: &[f64], c: f64) -> ExactDeck {
        self.env.set_trainee(t, q);
        self.env.work = 0;
        let mut sc = 0;
        let r = self.env.best_deck(t, c, 0.0, &[], &mut sc);
        self.env.last_pv = None;
        match r {
            DeckRes::Done(v, deck, finished) => ExactDeck { value: c * v, deck, finished },
            _ => panic!("audit: no legal deck"),
        }
    }
    /// The greedy audit at a pick change: the exact career search's pick against the surrogate's, and audit rows for
    /// the pick and `audit_sample` random candidates with the newest veteran as a parent (ceiling at least 0.3 x B; every
    /// candidate of the pool at the start)
    fn audit_greedy(&mut self, st: &mut PathState, keys: Keys, t_career: usize) -> Value {
        let cur = self.greedy_cur(st);
        let trainees = self.trainees.clone();
        let sv = self.env.solve(&st.pool, &[], &trainees, false, 1);
        let ex =
            sv.found.first().map(|f| (f.val, format!("{}:{},{}", self.d.umas[f.t].id, f.a.min(f.b), f.a.max(f.b))));
        let sk = self.key(&cur);
        let differs = ex.as_ref().is_none_or(|x| x.1 != sk);
        let mut rows = vec![self.audit_row(&st.pool, cur.lineage, AuditKind::Pick)];
        let bv = st.best.map_or(0.0, |p| p.value);
        let n = st.legacies.len();
        let vi = n - 1;
        let all = t_career == 0;
        let picked = cur.lineage.ordered();
        let mut cs = vec![];
        for &t in &trainees {
            for a in 0..n {
                for b in a + 1..n {
                    if !all && b != vi {
                        continue;
                    }
                    let lineage = Lineage { trainee: t, first_parent: a, second_parent: b };
                    if !self.legal(st, lineage) || lineage == picked {
                        continue;
                    }
                    if self.fast_ub(st, lineage) >= 0.3 * bv {
                        cs.push(lineage);
                    }
                }
            }
        }
        for i in 0..self.cfg.audit_sample.min(cs.len()) {
            let j = i + (keys.u(t_career, 9, i) * (cs.len() - i) as f64) as usize;
            let last = cs.len() - 1;
            cs.swap(i, j.min(last));
            rows.push(self.audit_row(&st.pool, cs[i], AuditKind::Random));
        }
        json!({"t": t_career, "pool": n, "sur": sk, "sur_p": cur.p_all, "ex": ex.as_ref().map(|x| x.1.clone()), "ex_p": ex.as_ref().map(|x| x.0),
               "differs": differs, "proven": sv.proven, "rows": rows})
    }
    /// the surrogate against exact searches on this pool (the argmax career, and the newest veteran's lineages)
    fn check(&mut self, st: &mut PathState, all: bool) -> Value {
        let cur = self.greedy_cur(st);
        let trainees = self.trainees.clone();
        let sv = self.env.solve(&st.pool, &[], &trainees, false, 1);
        let ex = sv.found.first().map(|f| (f.val, format!("{}:{},{}", self.d.umas[f.t].id, f.a, f.b)));
        let argmax = json!({"pool": st.legacies.len(), "sur": self.key(&cur), "sur_p": cur.p_all, "sur_val": st.best.map(|p| p.value),
                            "ex": ex.as_ref().map(|x| x.1.clone()), "ex_p": ex.as_ref().map(|x| x.0), "proven": sv.proven});
        let mut rows = vec![];
        if self.cfg.check_lineages > 0 {
            let bv = st.best.map_or(0.0, |p| p.value);
            let n = st.legacies.len();
            let vi = n - 1;
            // (the legal careers with her, or every one, `gen_cands`' ceilings, at least check_frac x the best)
            let mut cs: Vec<RankedCareer> = self
                .env
                .gen_cands(&st.pool, &[], &trainees, |_, b| all || b == vi, RankedCareer::new)
                .into_iter()
                .filter(|x| x.ceiling >= self.cfg.check_frac * bv)
                .collect();
            cs.sort_by(|x, y| y.ceiling.partial_cmp(&x.ceiling).unwrap());
            cs.truncate(self.cfg.check_lineages);
            if let Some(p) = st.best
                && !cs.iter().any(|x| x.lineage == p.lineage)
            {
                let (q, c) = self.env.lineage_q(&st.pool, p.lineage);
                cs.push(RankedCareer {
                    ceiling: self.env.ceiling(p.lineage.trainee, &q, c).unwrap(),
                    lineage: p.lineage,
                });
            }
            let mut buf = vec![];
            for RankedCareer { ceiling: ub, lineage } in cs {
                let t = lineage.trainee;
                let LineageChances { chances: q, carriers: nc, spark_scale: c } = self.env.lineage(&st.pool, lineage);
                let rb = self.cache.refs[t].iter().map(|r| self.env.ref_bound(t, &q, c, r)).fold(ub, f64::min);
                let (tv, di) = self.tval(t, &q, c, &mut buf);
                self.env.set_trainee(t, &q);
                self.env.work = 0;
                let mut sc = 0;
                let r = self.env.best_deck(t, c, 0.0, &[], &mut sc);
                self.env.last_pv = None;
                let DeckRes::Done(v, sel, full) = r else { continue };
                let exv = c * v;
                // landed sets: the cached argmax deck against the exact best deck (total variation distance)
                let tvd = if di == usize::MAX {
                    1.0
                } else {
                    let cached = self.cache.decks[t][di].sel.clone();
                    self.landed_set_distance(t, &q, &nc, &cached, &sel)
                };
                let pick = st.best.is_some_and(|p| p.lineage == lineage);
                rows.push(json!([
                    ub,
                    rb,
                    tv,
                    exv,
                    (exv - tv) / exv,
                    rb <= tv * (1.0 + self.cfg.margin),
                    tvd,
                    full,
                    pick
                ]));
            }
        }
        json!({"argmax": argmax, "lineages": rows})
    }

    /// Follow `options.base` from state st (survival 1, career 0) until the survival S < eps or max_careers, recording
    /// pick changes, checks and audits into `trace`.
    fn play(&mut self, st: &mut PathState, options: PlayOptions, trace: &mut PathTrace) -> PathResult {
        let PlayOptions { base, keys, eps, run } = options;
        let (mut s, mut t) = (1.0f64, 0usize);
        let mut e = 0.0f64;
        let mut pruned = 0usize;
        let mut buf = vec![];
        let mut last_key = String::new();
        let exact = self.cfg.policy == "greedy_exact";
        // Exact and index policies keep their decision until the pool changes.
        let mut kept_decision: Option<Rc<Cur>> = None;
        let mut since_check = 0usize;
        if self.cfg.check > 0 && base == Base::Greedy {
            trace.checks.push(self.check(st, true));
        }
        loop {
            let cur: Rc<Cur> = if exact {
                if let Some(career) = &kept_decision {
                    Rc::clone(career)
                } else {
                    let career = Rc::new(self.exact_cur(st));
                    kept_decision = Some(Rc::clone(&career));
                    career
                }
            } else {
                match base {
                    Base::Greedy => self.greedy_cur(st),
                    Base::Loop => self.loop_cur(st),
                    Base::Trace => unreachable!("trace runs its own loop"),
                    Base::Index => {
                        if let Some(career) = &kept_decision {
                            Rc::clone(career)
                        } else {
                            let career = Rc::new(self.index_cur(st));
                            kept_decision = Some(Rc::clone(&career));
                            career
                        }
                    }
                }
            };
            let k = self.key(&cur);
            if k != last_key {
                trace.pick_changes.push(json!({"t": t, "pick": k, "trainee": self.d.umas[cur.lineage.trainee].name, "p": cur.p_all, "pool": st.legacies.len(), "s": s}));
                last_key = k;
                if self.cfg.audit && base == Base::Greedy && !exact {
                    let a = self.audit_greedy(st, keys, t);
                    self.audits.push(a);
                }
            }
            if self.cfg.audit && base == Base::Loop && t.is_multiple_of(self.cfg.audit_every) {
                let r = self.audit_row(&st.pool, cur.lineage, AuditKind::Loop);
                self.audits.push(json!({"t": t, "pool": st.legacies.len(), "rows": [r]}));
            }
            if self.cfg.progress > 0 && t.is_multiple_of(self.cfg.progress) && t > 0 {
                eprintln!(
                    "  {} run {}: {} careers, pool {}, S {:.4}, p {:.3e}",
                    self.cfg.policy,
                    run,
                    t,
                    st.legacies.len(),
                    s,
                    cur.p_all
                );
            }
            if s < eps || t >= self.cfg.max_careers {
                let tail = s / cur.p_all;
                return PathResult { expected_careers: e + tail, careers: t, tail, final_p_all: cur.p_all, pruned };
            }
            e += s;
            s *= 1.0 - cur.p_all;
            let vet = self.draw(&cur, &st.legacies, keys, t);
            t += 1;
            match base {
                Base::Loop => {
                    self.add_vet(st, vet, false, &mut buf);
                    if cur.rotation {
                        self.rot_update(st);
                    }
                    st.current = None;
                    kept_decision = None;
                }
                Base::Trace => unreachable!("trace runs its own loop"),
                // (index: a pruned veteran leaves the pool as it was, so the plan, and its pick, are kept)
                Base::Greedy | Base::Index => {
                    if self.cfg.prune && self.dominated(st, &vet) {
                        pruned += 1;
                        continue;
                    }
                    self.add_vet(st, vet, !exact && base == Base::Greedy, &mut buf);
                    kept_decision = None;
                    since_check += 1;
                    if base == Base::Greedy && self.cfg.check > 0 && since_check >= self.cfg.check {
                        since_check = 0;
                        trace.checks.push(self.check(st, false));
                    }
                }
            }
        }
    }
}

/// A legal career and its ceiling (`Env::gen_cands`' order).
#[derive(Clone, Copy)]
struct RankedCareer {
    ceiling: f64,
    lineage: Lineage,
}
impl RankedCareer {
    /// `gen_cands`' constructor: (ceiling, trainee position, trainee, a, b)
    const fn new(ceiling: f64, _: usize, trainee: usize, first_parent: usize, second_parent: usize) -> Self {
        Self { ceiling, lineage: Lineage { trainee, first_parent, second_parent } }
    }
}

/// What an audit row is of (`Sim::audit_row`; its number is the row's first entry).
#[derive(Clone, Copy)]
enum AuditKind {
    /// The surrogate's pick at a pick change.
    Pick = 0,
    /// A random candidate with the newest veteran.
    Random = 1,
    /// A loop career.
    Loop = 3,
}

/// A trainee's exact best deck (`Sim::exact_best`).
struct ExactDeck {
    /// Its exact value times the spark scale.
    value: f64,
    deck: Sel,
    /// The deck search finished (proven best).
    finished: bool,
}

/// How a run's path is followed (`Sim::play`).
#[derive(Clone, Copy)]
struct PlayOptions {
    base: Base,
    keys: Keys,
    /// Stop once the survival S falls below this.
    eps: f64,
    /// The run index (progress lines).
    run: u32,
}

/// What a path records as it goes (`Sim::play`).
#[derive(Default)]
struct PathTrace {
    /// Each change of the career picked: career index, pick, trainee, P(all), pool size, survival.
    pick_changes: Vec<Value>,
    /// The surrogate's checks against exact searches (`Sim::check`).
    checks: Vec<Value>,
}

/// One run's result (`Sim::play`).
struct PathResult {
    /// E: the sum of the survivals S_t, the tail S / p included (module notes).
    expected_careers: f64,
    /// Careers run before the path stopped.
    careers: usize,
    /// The tail S / p added at the stop.
    tail: f64,
    /// P(all) of the career the path stopped at.
    final_p_all: f64,
    /// New veterans dropped as dominated.
    pruned: usize,
}

pub(super) fn simulate<'data>(
    env: &mut Env<'data>,
    input: &JobInputs<'data>,
    legs: &[Legacy],
    trainees: &[usize],
    sim: &Value,
) -> Value {
    let cfg: SimJob = match serde_json::from_value(sim.clone()) {
        Ok(x) => x,
        Err(e) => return json!({"error": format!("simulate: {e}")}),
    };
    let ntar = env.ntar;
    if ntar > 12 || env.nu > 10 {
        return json!({"error": "simulate takes at most 12 targets, 10 of them not guaranteed"});
    }
    let base = match cfg.policy.as_str() {
        "greedy" | "greedy_exact" => Base::Greedy,
        "loop" => Base::Loop,
        "trace" => Base::Trace,
        "index" => Base::Index,
        p => return json!({"error": format!("simulate: unknown policy {p}")}),
    };
    let members: Vec<usize> = cfg.loop_members.iter().filter_map(|&id| env.uix(id)).collect();
    if matches!(base, Base::Loop | Base::Trace)
        && (members.len() < 3
            || members.len() != cfg.loop_members.len()
            || members.iter().any(|m| !env.owned.contains(m)))
    {
        return json!({"error": "loop: loop_members must be at least three owned umas"});
    }
    let n = input.data.umas.len();
    let mut mi = vec![None; if base == Base::Trace { n } else { 0 }];
    if base == Base::Trace {
        for (k, &m) in members.iter().enumerate() {
            mi[m] = Some(k);
        }
    }
    let mut s = Sim::new(
        env,
        input,
        cfg.clone(),
        SimulationRoster { trainees: trainees.to_vec(), members, member_by_trainee: mi },
        SimulationOptions { record_audits: true, fill_missing_decks: false },
    );
    let t_init = std::time::Instant::now();
    let st0 = s.initial(legs, base);
    if base == Base::Trace {
        return s.trace(&st0);
    }
    let init_s = t_init.elapsed().as_secs_f64();
    let mut runs = vec![];
    let mut es = vec![];
    let eps = cfg.eps;
    // index: the kept state before the simulation, put back after it (module notes)
    let kept_before_simulation = (base == Base::Index).then(|| kept_swap(KeptState::default()));
    for run in cfg.first..cfg.first + cfg.runs {
        if !cfg.share_cache && run > cfg.first {
            s.cache = Cache::new(n);
        }
        if base == Base::Index {
            kept_swap(KeptState::default());
            IX.with(|ix| *ix.borrow_mut() = IndexRunCounts::default());
        }
        let tm = std::time::Instant::now();
        let n0 = s.n;
        let mut st = st0.clone();
        let mut trace = PathTrace::default();
        let keys = Keys::Main { seed: cfg.seed, run: run as u32 };
        let PathResult { expected_careers: e, careers, tail, final_p_all: fp, pruned } =
            s.play(&mut st, PlayOptions { base, keys, eps, run: run as u32 }, &mut trace);
        let n1 = s.n;
        es.push(e);
        let PathTrace { pick_changes: segs, checks } = trace;
        let finalk = segs.last().map_or(Value::Null, |x| x["pick"].clone());
        let wall = tm.elapsed().as_secs_f64();
        eprintln!(
            "{} run {}: E {:.1}, {} careers, tail {:.2}, final p {:.3e}, pool {}, {:.1} s",
            cfg.policy,
            run,
            e,
            careers,
            tail,
            fp,
            st.legacies.len(),
            wall
        );
        runs.push(json!({"run": run, "E": e, "careers": careers, "tail": tail, "final_p": fp, "final": finalk, "pool": st.legacies.len(), "pruned": pruned,
            "wall_s": wall, "candidates": n1.candidates - n0.candidates, "ceiling_pass": n1.ceiling_passes - n0.ceiling_passes, "tables": n1.tables - n0.tables,
            "refreshes": n1.searches - n0.searches, "partial_searches": n1.partial_searches - n0.partial_searches, "segs": segs, "checks": checks, "audits": std::mem::take(&mut s.audits)}));
        if base == Base::Index {
            let ix = IX.with(|ix| std::mem::take(&mut *ix.borrow_mut()));
            runs.last_mut().unwrap()["index"] = json!({"plans": ix.plans, "plan_s": ix.plan_seconds, "roles": ix.roles, "best_odds": ix.best_odds_picks});
        }
    }
    if let Some(kept) = kept_before_simulation {
        kept_swap(kept);
    }
    json!({"simulate": sim, "policy": cfg.policy, "runs": runs, "E": stats(&es), "init_s": init_s, "decks": s.cache.n_decks(), "deck_tables": s.cache.tables,
           "refreshes": s.n.searches, "schedules": s.win_chances.len()})
}

#[cfg(test)]
mod tests {
    #[test]
    /// the random numbers are pinned: changing them would change every recorded run
    fn u01_values() {
        assert_eq!(super::u01(&[1, 3, 0, 1, 0]), 0.006_101_473_502_044_041);
        assert_eq!(super::u01(&[1, 0, 17, 2, 5]), 0.786_756_654_747_679);
        assert_eq!(super::u01(&[7, 123_456, 99_999, 3, 33]), 0.923_524_891_523_141_8);
    }
}
