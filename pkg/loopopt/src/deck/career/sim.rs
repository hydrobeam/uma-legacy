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
//! picks the next career (`Cur`) from the state (`St`) in `play`; `trace` runs its own loop and returns its rows.
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
//! The loop adds every veteran (no pruning): it never scans the pool, and the latest veteran is the one it breeds from.
use super::*;
use super::cache::Cache;
use std::rc::Rc;
/// the page's next career (`next` on a career job)
pub(super) mod plan;

fn d_policy() -> String { "greedy".into() }
fn d_runs() -> usize { 16 }
fn d_seed() -> u32 { 1 }
fn d_eps() -> f64 { 1e-3 }
fn d_maxc() -> usize { 100_000 }
fn d_frac() -> f64 { 0.3 }
fn d_asample() -> usize { 3 }
fn d_aevery() -> usize { 50 }

#[derive(Deserialize, Clone)]
pub(super) struct SimJob {
    /// greedy | greedy_exact | loop | trace | index
    #[serde(default = "d_policy")] policy: String,
    #[serde(default = "d_runs")] runs: usize,
    #[serde(default)] first: usize,
    #[serde(default = "d_seed")] seed: u32,
    #[serde(default = "d_eps")] eps: f64,
    #[serde(default = "d_maxc")] max_careers: usize,
    /// refresh margin m (module notes); 0 = refresh whenever a bound exceeds the table value
    #[serde(default)] margin: f64,
    #[serde(default = "d_true")] prune: bool,
    /// keep the deck cache across runs (faster; with m > 0 a pick can depend on what earlier runs cached)
    #[serde(default = "d_true")] share_cache: bool,
    /// every `check` new veterans (and at the start), compare the surrogate with exact searches
    #[serde(default)] check: usize,
    /// lineages checked per check (the newest veteran's candidates, ceiling at least `check_frac` x B, best first)
    #[serde(default)] check_lineages: usize,
    #[serde(default = "d_frac")] check_frac: f64,
    /// loop: rotation members (chara ids) in order; trace: the trainee built for, then her builders
    #[serde(default)] loop_members: Vec<u32>,
    /// fidelity audit (`audit_row`): at every pick change of greedy (the exact argmax, the pick and `audit_sample`
    /// random candidates with the newest veteran), and every `audit_every` careers of the loop (its career, and its
    /// keep-or-replace decision under exact values)
    #[serde(default)] audit: bool,
    #[serde(default = "d_asample")] audit_sample: usize,
    #[serde(default = "d_aevery")] audit_every: usize,
    /// log progress to stderr every this many careers (0: off)
    #[serde(default)] progress: usize,
    /// index: the plan job's `next` (docs/SOLVER.md "Next career"); the page sends `{}`
    #[serde(default = "d_next")] next: Value,
}
fn d_next() -> Value { json!({}) }
/// index: the page's `top` for its plan job (`ROLL_K` in webapp/src/career.js)
const PAGE_TOP: usize = 6;
/// The worker's kept state across jobs: the kept plans, proofs, main searches and deck memos
type KeptState = (Vec<plan::Bests>, Vec<(u64, Vec<Vec<Ref>>)>, Vec<(u64, Solved, super::Ruled)>, Vec<(u64, super::Kept)>);
/// replaces the worker's kept state with `k`, returning what it was (`index`: module notes)
fn kept_swap(k: KeptState) -> KeptState {
    let b = plan::bests_take(); plan::bests_set(k.0);
    let s = super::SEEN.with(|c| std::mem::replace(&mut *c.borrow_mut(), k.1));
    let l = super::LAST_SOLVE.with(|c| std::mem::replace(&mut *c.borrow_mut(), k.2));
    let d = super::DECK_KEEP.with(|c| std::mem::replace(&mut *c.borrow_mut(), k.3));
    (b, s, l, d)
}

// ---- random numbers ----
fn fmix(mut h: u32) -> u32 { h ^= h >> 16; h = h.wrapping_mul(0x85eb_ca6b); h ^= h >> 13; h = h.wrapping_mul(0xc2b2_ae35); h ^= h >> 16; h }
/// 53 bits in [0, 1) from the keys
pub(super) fn u01(keys: &[u32]) -> f64 {
    let (mut a, mut b) = (0x9e37_79b9u32, 0x7f4a_7c15u32);
    for &k in keys { a = fmix((a ^ k).wrapping_add(0x632b_e5ab)); b = fmix(b.wrapping_add(k.wrapping_mul(0x27d4_eb2f)) ^ a); }
    (a as f64 * 2f64.powi(21) + (b >> 11) as f64) / 2f64.powi(53)
}
const STREAM_SET: u32 = 1;
const STREAM_STAR: u32 = 2;
const STREAM_WIN: u32 = 3;
/// Whose uniforms: a run's own path
#[derive(Clone, Copy)]
enum Keys { Main { seed: u32, run: u32 } }
impl Keys {
    fn u(&self, t: usize, stream: u32, item: usize) -> f64 {
        match *self {
            Keys::Main { seed, run } => u01(&[seed, run, t as u32, stream, item as u32]),
        }
    }
}

/// The best career so far by the surrogate: value (C x table), trainee, parents (a < b), deck index in the cache
#[derive(Clone, Copy)]
struct Pick { val: f64, t: usize, a: usize, b: usize, d: usize }
/// A career to run, with its exact numbers: P(all), landed-set chances (bit i = job target i), win chance per race
struct Cur { t: usize, a: usize, b: usize, p: f64, os: Vec<f64>, wp: Rc<Vec<f64>>, rot: bool }
/// Loop rotation: the next member's position and each member's latest rotation veteran
#[derive(Clone, Default)]
struct Rot { pos: usize, latest: Vec<Option<usize>> }
/// One path's state: veterans, the solver's pool, owned veterans per family (dominance), the surrogate's best
#[derive(Clone)]
struct St { legs: Vec<Legacy>, pool: Pool, fam: crate::FxMap<String, Vec<(Vec<u8>, Vec<usize>)>>, best: Option<Pick>, cur: Option<Rc<Cur>>, rot: Rot,
            /// `next`: each loop member's best career (by member index)
            mb: Vec<Option<Pick>> }

#[derive(Clone, Copy, PartialEq)]
enum Base { Greedy, Loop, Trace, Index }
#[derive(Default, Clone, Copy)]
struct Count { cands: usize, ceil_pass: usize, tables: usize, searches: usize, partial: usize, added: usize }

struct Sim<'s, 'a> {
    env: &'s mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value,
    cfg: SimJob, cache: Cache, trainees: Vec<usize>, hi: Vec<f64>, order: Vec<usize>, star: [f64; 3],
    sched: crate::FxMap<(usize, Vec<usize>), Rc<Vec<f64>>>, n: Count, members: Vec<usize>,
    /// per trainee and outfit, per uncertain target (A, B): her ceiling factor is A - B (1 - q) (`fast_ub`)
    fastc: Vec<Vec<Vec<(f64, f64)>>>, fixed: f64,
    /// trace: trainee → loop member index (empty under the other policies)
    mi: Vec<Option<usize>>,
    /// the simulator's own path (false when the plan uses this machinery): audits are taken only there
    main: bool, audits: Vec<Value>,
    /// the plan only (false for every policy here): a trainee without a cached deck gets her best deck searched without
    /// a bar (so she has one from then on), instead of a search at the bar that caches nothing when it ends Bounded
    fill: bool,
    /// a lineage's chances, reused (`lineage_into`)
    qs: Vec<f64>,
    /// the planner's working copy of its pool (`take_pool`), with the address, length and borrowing rule of the pool it
    /// copies
    wpool: Option<((usize, usize, u8), Pool)>,
}
/// index, this run: plan jobs run, each one's seconds, the picks' roles (`next.options[].role`), picks that were the best
/// odds career (`best_odds`)
type IxCount = (usize, Vec<f64>, std::collections::BTreeMap<String, usize>, usize);
thread_local! { static IX: std::cell::RefCell<IxCount> = std::cell::RefCell::new(Default::default()); }

fn canon(chara: u32, sparks: &[(u32, u8)], pinks: &[(u32, u8)], wins: &[usize]) -> String {
    let (mut s, mut p, mut w) = (sparks.to_vec(), pinks.to_vec(), wins.to_vec());
    s.sort(); p.sort(); w.sort();
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
    let z = 1.959964f64; let (z3, z5, z7) = (z.powi(3), z.powi(5), z.powi(7));
    z + (z3 + z) / (4.0 * df) + (5.0 * z5 + 16.0 * z3 + 3.0 * z) / (96.0 * df * df) + (3.0 * z7 + 19.0 * z5 + 17.0 * z3 - 15.0 * z) / (384.0 * df.powi(3))
}
fn stats(xs: &[f64]) -> Value {
    let n = xs.len() as f64;
    if xs.is_empty() { return Value::Null; }
    let mean = xs.iter().sum::<f64>() / n;
    let sd = if xs.len() > 1 { (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt() } else { 0.0 };
    let half = if xs.len() > 1 { tq975(n - 1.0) * sd / n.sqrt() } else { 0.0 };
    json!({"n": xs.len(), "mean": mean, "sd": sd, "ci95": [mean - half, mean + half], "min": xs.iter().cloned().fold(f64::INFINITY, f64::min), "max": xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max)})
}

/// Per trainee and outfit, per uncertain target (A, B): her ceiling factor is A - B (1 - q) (`Sim::fastc`; none
/// without legal deck odds). `outfit_fast` at q = 0 on the hint chances `set_trainee` would write, split: A = dg GOLD +
/// (1 - dg) WHITE and B = (1 - dg) WHITE (1 - dw)(1 - e) where she lacks the skill, B = 0 where she has it
fn fastc(env: &Env) -> Vec<Vec<Vec<(f64, f64)>>> {
    (0..env.d.umas.len()).map(|t| match &env.dmax[t] { None => vec![], Some((dw, dg)) =>
        env.sts0[t].iter().zip(&env.ew0[t]).map(|(sv, ew)| (0..env.nu).map(|k| match sv[k] {
            2 => (GEN_GOLD, 0.0),
            1 => (dg[k] * GEN_GOLD + (1.0 - dg[k]) * crate::skills::GEN_WHITE, 0.0),
            _ => (dg[k] * GEN_GOLD + (1.0 - dg[k]) * crate::skills::GEN_WHITE, (1.0 - dg[k]) * crate::skills::GEN_WHITE * (1.0 - dw[k]) * (1.0 - ew[k])),
        }).collect()).collect() }).collect()
}
impl<'s, 'a> Sim<'s, 'a> {
    /// The simulator's machinery for a job (`simulate`'s, and the plan's through `plan::sim_for`): the landed sets in
    /// order of size, each owned trainee's ceiling at q = 1 (`hi`), the rank's star mix, `fastc`, the fixed gold factor
    #[allow(clippy::too_many_arguments)]
    fn new(env: &'s mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, cfg: SimJob, trainees: Vec<usize>, members: Vec<usize>,
           mi: Vec<Option<usize>>, main: bool, fill: bool) -> Self {
        let ntar = env.ntar;
        let full = (1usize << ntar) - 1;
        let mut order: Vec<usize> = (0..full).collect();
        order.sort_by_key(|&m| (m.count_ones(), m));
        let n = d.umas.len();
        let one = vec![1.0f64; ntar];
        let hi: Vec<f64> = (0..n).map(|t| if env.owned.contains(&t) { env.ceiling(t, &one, 1.0).unwrap_or(0.0) } else { 0.0 }).collect();
        let star = env.sj.star_dist;
        let fastc = fastc(env);
        let fixed = env.model.fixed_factor();
        Sim { env, d, sk, cr, job, cfg, cache: Cache::new(n), trainees, hi, order, star, sched: Default::default(), n: Count::default(), members, fastc, fixed, main, audits: vec![], fill, mi, qs: vec![], wpool: None }
    }
    /// A working copy of `pool` (the planner's `Env::pool(legs)`), equal to it: the one kept by `put_pool` when it was
    /// made from this pool (the same address, length and borrowing rule: the pool a caller passes is borrowed, so
    /// unchanged, or, as the simulator's, only grown), else a clone. A user pushes its extra entries (`Env::pool_push`
    /// builds an entry as `Env::pool` does, so the pool of legs and those) and pops them before `put_pool`
    pub(super) fn take_pool(&mut self, pool: &Pool) -> Pool {
        let id = (pool as *const Pool as usize, pool.me.len(), pool.max_bor);
        match self.wpool.take() { Some((k, p)) if k == id => p, _ => pool.clone() }
    }
    /// `take_pool`'s copy back, its extra entries popped
    pub(super) fn put_pool(&mut self, pool: &Pool, p: Pool) {
        debug_assert_eq!(p.me.len(), pool.me.len());
        self.wpool = Some(((pool as *const Pool as usize, pool.me.len(), pool.max_bor), p));
    }
    fn key(&self, c: &Cur) -> String { format!("{}:{},{}", self.d.umas[c.t].id, c.a, c.b) }
    /// P(all) at chances q with spark scale c through trainee t's cached decks: (value, deck index; usize::MAX: none)
    fn tval(&self, t: usize, q: &[f64], c: f64, buf: &mut Vec<f64>) -> (f64, usize) {
        // (1 - q per uncertain target, on the stack up to 32 of them)
        let nu = self.env.uncv.len();
        let (mut xa, xv): ([f64; 32], Vec<f64>);
        let x: &[f64] = if nu <= 32 {
            xa = [0.0; 32]; for (k, &i) in self.env.uncv.iter().enumerate() { xa[k] = 1.0 - q[i]; } &xa[..nu]
        } else { xv = self.env.uncv.iter().map(|&i| 1.0 - q[i]).collect(); &xv };
        let (mut best, mut bd) = (0.0f64, usize::MAX);
        for (di, dk) in self.cache.decks[t].iter().enumerate() {
            for tab in &dk.tab { let v = interp_in(tab, x, buf); if bd == usize::MAX || v > best { best = v; bd = di; } }
        }
        (c * best, bd)
    }
    /// trainee t's best cached deck at chances q (spark scale c, ceiling ub) after the refresh rule (module notes), when
    /// it can reach the bar th: (value, deck index); None when it provably cannot
    fn deck_for(&mut self, t: usize, q: &[f64], c: f64, ub: f64, th: f64, buf: &mut Vec<f64>) -> Option<(f64, usize)> {
        let mut rb = ub;
        for r in &self.cache.refs[t] { rb = rb.min(self.env.ref_bound(t, q, c, r)); if rb * (1.0 + 1e-9) < th { return None; } }
        if th > 0.0 && self.env.ruled_out(t, q, c, &self.env.ruled[self.env.cls[t]], Bar::Below(th)) { return None; }
        self.n.tables += 1;
        let (mut tv, mut di) = self.tval(t, q, c, buf);
        if di == usize::MAX || rb > tv * (1.0 + self.cfg.margin) {
            self.env.set_trainee(t, q);
            // (her cached decks borrowed: the cache and the Env are separate fields)
            let sr: Vec<&Sel> = self.cache.decks[t].iter().map(|x| &x.sel).collect();
            self.env.work = 0;
            let mut sc = 0;
            let r = self.env.best_deck(t, c, if self.fill && di == usize::MAX { 0.0 } else { th }, &sr, &mut sc);
            if let Some(s) = self.env.last_pv.take() { let ct = self.env.cls[t]; self.env.ruled[ct].push((q.to_vec(), s)); }
            self.n.searches += 1;
            if let DeckRes::Done(v, sel, full) = r {
                self.cache.add(self.env, t, &sel);
                if full { self.cache.refs[t].push(Ref { q: q.to_vec(), val: v, sel }); } else { self.n.partial += 1; }
                (tv, di) = self.tval(t, q, c, buf);
            }
        }
        if di == usize::MAX { None } else { Some((tv, di)) }
    }
    /// `Env::ceiling` of candidate (t, a, b) without allocations (the same slots, affinities and formula as `slots`,
    /// `inherit` and `outfit_fast`; equal up to rounding, so it only filters, with a slack)
    fn fast_ub(&self, st: &St, t: usize, a: usize, b: usize) -> f64 {
        let env = &*self.env; let p = &st.pool; let ntar = env.ntar;
        // `walk_slots`' slots and affinities, each slot's misses multiplied in as it
        // comes but a parent's after her grandparent slots' (this filter's own order, so its own rounding); a
        // random-star slot counts at its fixed stars here, where `inherit` averages its squared miss over the mix
        struct Ub<'p> { ntar: usize, miss: [f64; 12], nc: [i32; 12], parent: Option<(&'p [u8], f64)> }
        impl Ub<'_> {
            #[inline(always)]
            fn add(&mut self, stars: &[u8], fa: f64) { for k in 0..self.ntar { let s = stars[k]; if s > 0 { self.nc[k] += 1; self.miss[k] *= insp_miss(s as f64, fa); } } }
            #[inline(always)]
            fn flush(&mut self) { if let Some((s, fa)) = self.parent.take() { self.add(s, fa); } }
        }
        impl<'p> SlotSink<'p> for Ub<'p> {
            #[inline(always)]
            fn slot(&mut self, _: usize, gi: Option<usize>, w: &'p Who, rel: f64, race: f64) {
                if gi.is_none() { self.flush(); self.parent = Some((&w.stars[..], aff_factor(rel, race))); } else { self.add(&w.stars, aff_factor(rel, race)); }
            }
        }
        let mut ub = Ub { ntar, miss: [1.0f64; 12], nc: [0i32; 12], parent: None };
        env.walk_slots(p, a, b, &RelOf { env, p, t }, &mut ub);
        ub.flush();
        let (miss, nc) = (ub.miss, ub.nc);
        let mut c = 1.0f64;
        for k in 0..ntar { c *= spark_scale(nc[k]); }
        let mut best = 0.0f64;
        for ab in &self.fastc[t] {
            let mut v = 1.0f64;
            for (j, &i) in env.uncv.iter().enumerate() { v *= ab[j].0 - ab[j].1 * miss[i] * miss[i]; }
            if v > best { best = v; }
        }
        env.space.rho_top() * c * self.fixed * best
    }
    /// value candidate (t, a, b) against the best so far
    /// the value a career of trainee t must beat to matter: the best overall, or under `next` a member's own best
    fn bar(&self, st: &St, t: usize) -> f64 {
        match self.mi.get(t).copied().flatten() { Some(k) => st.mb[k].map_or(0.0, |p| p.val), None => st.best.map_or(0.0, |p| p.val) }
    }
    fn consider(&mut self, st: &mut St, t: usize, a: usize, b: usize, buf: &mut Vec<f64>) {
        let bv = self.bar(st, t);
        self.n.cands += 1;
        let mut q = std::mem::take(&mut self.qs);
        let c = self.env.lineage_into(&st.pool, t, a, b, &mut q);
        self.consider_at(st, t, a, b, &q, c, bv, buf);
        self.qs = q;
    }
    /// `consider` at the candidate's chances q and spark scale c
    #[allow(clippy::too_many_arguments)]
    fn consider_at(&mut self, st: &mut St, t: usize, a: usize, b: usize, q: &[f64], c: f64, bv: f64, buf: &mut Vec<f64>) {
        let Some(ub) = self.env.ceiling(t, q, c) else { return };
        static CHECK_UB: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *CHECK_UB.get_or_init(|| std::env::var("SIM_CHECK_UB").is_ok()) { let f = self.fast_ub(st, t, a, b); assert!((f - ub).abs() <= 1e-12 * ub, "fast_ub {f} vs ceiling {ub}"); }
        if ub * (1.0 + 1e-9) < bv { return; }
        self.n.ceil_pass += 1;
        let Some((tv, di)) = self.deck_for(t, q, c, ub, bv, buf) else { return };
        let pk = Pick { val: tv, t, a: a.min(b), b: a.max(b), d: di };
        if let Some(k) = self.mi.get(t).copied().flatten() { if tv > bv { st.mb[k] = Some(pk); st.cur = None; } }
        if tv > st.best.map_or(0.0, |p| p.val) { st.best = Some(pk); st.cur = None; }
    }
    fn legal(&self, st: &St, t: usize, a: usize, b: usize) -> bool { st.pool.pair_ok(self.d.umas[t].id, a, b) }
    /// her chance per race of at least one win, cached per (trainee, the parents' wins)
    pub(super) fn wins(&mut self, pool: &Pool, t: usize, a: usize, b: usize) -> Rc<Vec<f64>> {
        let mut k: Vec<usize> = pool.me[a].wins.iter().chain(&pool.me[b].wins).map(|w| w.0).collect(); k.sort();
        // her pink sparks change her win chances: their inputs join the key (start grades, each spark's aptitude and chance)
        if let Some(pc) = self.env.pink_career(pool, t, a, b) {
            k.push(usize::MAX); k.extend(pc.start.iter().map(|&g| g as usize));
            for f in &pc.fire { k.push(f.apt); k.push((f.p * 1e9).round() as usize); }
        }
        if let Some(x) = self.sched.get(&(t, k.clone())) { return x.clone(); }
        let w = Rc::new(self.env.schedule(pool, t, a, b).1);
        self.sched.insert((t, k), w.clone());
        w
    }
    /// career (t, a, b) with deck `sel`: its exact numbers
    pub(super) fn make_cur(&mut self, pool: &Pool, t: usize, a: usize, b: usize, sel: &Sel, rot: bool) -> Cur {
        let (q, nc) = inherit(&self.env.slots(pool, t, a, b), self.env.ntar);
        self.env.set_trainee(t, &q);
        let (j, ext) = self.env.exact(t, sel, &nc, true, true);
        let os = self.env.outcome_sets(j.2, &ext);
        let wp = self.wins(pool, t, a, b);
        Cur { t, a, b, p: j.0, os, wp, rot }
    }
    fn greedy_cur(&mut self, st: &mut St) -> Rc<Cur> {
        if let Some(c) = &st.cur { return c.clone(); }
        let pk = st.best.expect("no legal career");
        let sel = self.cache.decks[pk.t][pk.d].sel.clone();
        let c = Rc::new(self.make_cur(&st.pool, pk.t, pk.a, pk.b, &sel, false));
        st.cur = Some(c.clone());
        c
    }
    /// `trace`: building for loop_members[0] (A) with the others as builders. Each career: per builder her best career's
    /// exact chance of raising A's best odds (lam) and mean odds raised to (p1); the most efficient (lam (1/p - 1/p1)) runs,
    /// her veteran joins; recorded per career: A's odds before, the builder, lam, p1, and A's odds after
    fn trace(&mut self, st0: &St) -> Value {
        let a = self.members[0]; let nb = self.members.len();
        let mut out = vec![];
        for run in self.cfg.first..self.cfg.first + self.cfg.runs {
            let mut st = st0.clone(); let keys = Keys::Main { seed: self.cfg.seed, run: run as u32 };
            let mut buf = vec![]; let mut rows = vec![];
            let tm = std::time::Instant::now();
            for t in 0..self.cfg.max_careers {
                let Some(pka) = st.mb[0] else { break }; let pa = pka.val;
                let one = [a];
                let mut bestb: Option<(f64, Pick, f64, f64)> = None;
                for k in 1..nb {
                    let Some(pk) = st.mb[k] else { continue };
                    let sel = self.cache.decks[pk.t][pk.d].sel.clone();
                    let pw = [pa];
                    let base = plan::ScoreBase { cands: &one, c1: &[1.0], m23: &[0.0], v0: 0.0, pnow: &pw, cont: &[], combos: &[], sides: &[], all: &[] };
                    let (_, _, _, stt) = plan::score_career_of(self, self.d, &base, &st.legs, &st.pool, pk.t, pk.a, pk.b, &sel, plan::Want::Stats, &mut buf);
                    let (pl, sp) = stt[0];
                    let (e, p1) = if pl > 0.0 { let p1 = sp / pl; (pl * (1.0 / pa - 1.0 / p1), p1) } else { (0.0, pa) };
                    if bestb.map_or(true, |b| e > b.0) { bestb = Some((e, pk, pl, p1)); }
                }
                let Some((_, pk, lam, p1)) = bestb else { break };
                let sel = self.cache.decks[pk.t][pk.d].sel.clone();
                let cur = self.make_cur(&st.pool, pk.t, pk.a, pk.b, &sel, false);
                let (vet, _) = self.draw(&cur, &st.legs, &keys, t);
                self.add_vet(&mut st, vet, true, &mut buf);
                let after = st.mb[0].map_or(pa, |p| p.val);
                rows.push(json!([t, pa, self.d.umas[pk.t].name, lam, p1, after]));
            }
            eprintln!("trace run {}: {} careers, A from {:.3e} to {:.3e}, {:.1} s", run, rows.len(), st0.mb[0].map_or(0.0, |p| p.val), st.mb[0].map_or(0.0, |p| p.val), tm.elapsed().as_secs_f64());
            out.push(json!({"run": run, "rows": rows}));
        }
        json!({"trace": out, "trainee": self.d.umas[a].name, "builders": self.members[1..].iter().map(|&u| self.d.umas[u].name.clone()).collect::<Vec<_>>()})
    }
    /// the loop's next career (module notes: rotation and bootstrap)
    fn loop_cur(&mut self, st: &mut St) -> Rc<Cur> {
        if let Some(c) = &st.cur { return c.clone(); }
        let nm = self.members.len();
        let pos = st.rot.pos % nm;
        let t = self.members[pos];
        let prev: Vec<usize> = [1, 2].iter().filter_map(|&k| st.rot.latest[(pos + nm - k) % nm]).collect();
        let c = if prev.len() == 2 {
            let (a, b) = (prev[0].min(prev[1]), prev[0].max(prev[1]));
            let (q, c) = self.env.lineage_q(&st.pool, t, a, b);
            let ub = self.env.ceiling(t, &q, c).unwrap_or(f64::INFINITY);
            let mut buf = vec![];
            let (_, di) = self.deck_for(t, &q, c, ub, 0.0, &mut buf).expect("loop career without a deck");
            let sel = self.cache.decks[t][di].sel.clone();
            self.make_cur(&st.pool, t, a, b, &sel, true)
        } else {
            self.env.use_seen = true;
            let sv = self.env.solve(&st.pool, &prev, &[t], false, 1);
            let f = sv.found.first().expect("loop bootstrap: no legal career for a member");
            self.cache.absorb(self.env, &sv.refs);
            self.make_cur(&st.pool, f.t, f.a, f.b, &f.sel.clone(), true)
        };
        let c = Rc::new(c);
        st.cur = Some(c.clone());
        c
    }
    /// The loop's rotation after a member's career (her veteran the pool's last): the member keeps the better of her
    /// rotation veteran and the new one, by the sum of the surrogate P(all) of the two loop careers she is a parent of
    /// (members pos + 1 and pos + 2, the other parent their current rotation veteran; a term without one is left out
    /// for both); the first one she has is kept as is. Then the next member trains.
    fn rot_update(&mut self, st: &mut St) {
        let nm = self.members.len(); let m = st.rot.pos % nm; let vi = st.legs.len() - 1;
        st.rot.pos += 1; st.cur = None;
        let Some(old) = st.rot.latest[m] else { st.rot.latest[m] = Some(vi); return };
        let mut buf = vec![];
        let mut score = |s: &mut Self, v: usize| -> f64 {
            let mut tot = 0.0;
            for k in 1..=2 {
                let p = (m + k) % nm; let t = s.members[p];
                let other = if k == 1 { st.rot.latest[(m + nm - 1) % nm] } else { st.rot.latest[(m + 1) % nm] };
                let Some(o) = other else { continue };
                if o == v || st.pool.me[o].chara == st.pool.me[v].chara { continue; }
                let (q, c) = s.env.lineage_q(&st.pool, t, v.min(o), v.max(o));
                let ub = s.env.ceiling(t, &q, c).unwrap_or(f64::INFINITY);
                tot += s.deck_for(t, &q, c, ub, 0.0, &mut buf).map_or(0.0, |x| x.0);
            }
            tot
        };
        let (so, sn) = (score(self, old), score(self, vi));
        if self.cfg.audit && self.main && st.rot.pos % self.cfg.audit_every == 0 {
            // the keep-or-replace decision under exact values (a full deck search per term)
            let ex = |s: &mut Self, v: usize| -> f64 {
                let mut tot = 0.0;
                for k in 1..=2 {
                    let p = (m + k) % nm; let t = s.members[p];
                    let other = if k == 1 { st.rot.latest[(m + nm - 1) % nm] } else { st.rot.latest[(m + 1) % nm] };
                    let Some(o) = other else { continue };
                    if o == v || st.pool.me[o].chara == st.pool.me[v].chara { continue; }
                    let (q, c) = s.env.lineage_q(&st.pool, t, v.min(o), v.max(o));
                    tot += s.exact_best(t, &q, c).0;
                }
                tot
            };
            let (eo, en) = (ex(self, old), ex(self, vi));
            self.audits.push(json!({"t": st.rot.pos, "rotation": [so, sn, eo, en, (sn > so) == (en > eo)]}));
        }
        if sn > so { st.rot.latest[m] = Some(vi); }
    }
    /// the in-process career search's pick (greedy_exact: the best career by P(all))
    fn exact_cur(&mut self, st: &St) -> Cur {
        let mut job = self.job.clone();
        let o = job.as_object_mut().unwrap();
        o.remove("simulate");
        o.insert("legacies".into(), Value::Array(st.legs.iter().map(leg_json).collect()));
        o.insert("top".into(), json!(1)); o.insert("per_trainee".into(), json!(false));
        let out = run_career(self.d, self.sk, self.cr, &job);
        let pick = out["results"][0].clone();
        assert!(pick.is_object(), "solver gave no pick: {}", out.to_string().chars().take(300).collect::<String>());
        let t = self.env.uix(pick["trainee"]["id"].as_u64().unwrap() as u32).unwrap();
        let (a, b) = (pick["parents"][0].as_u64().unwrap() as usize, pick["parents"][1].as_u64().unwrap() as usize);
        let os: Vec<f64> = pick["outcome_sets"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        let wp = self.wins(&st.pool, t, a, b);
        Cur { t, a, b, p: pick["p_all"].as_f64().unwrap(), os, wp, rot: false }
    }
    /// `index`: the page's plan for this pool (the career job with the pool as legacies, no places, the page's `top`,
    /// `next`, `no_reports`: the plan job the page sends, docs/SOLVER.md "Next career"), in process; its `choice` runs.
    /// The job's `trainees` stay (the page sends none: every owned uma)
    fn index_cur(&mut self, st: &St) -> Cur {
        let mut job = self.job.clone();
        let o = job.as_object_mut().unwrap();
        o.remove("simulate");
        o.insert("legacies".into(), Value::Array(st.legs.iter().map(leg_json).collect()));
        o.insert("fix".into(), json!([])); o.insert("top".into(), json!(PAGE_TOP)); o.insert("per_trainee".into(), json!(false));
        o.insert("next".into(), self.cfg.next.clone()); o.insert("no_reports".into(), json!(true));
        let tm = std::time::Instant::now();
        let out = run_career(self.d, self.sk, self.cr, &job);
        let pick = out["next"]["choice"].clone();
        assert!(pick.is_object(), "index: the plan gave no pick: {}", out.to_string().chars().take(300).collect::<String>());
        IX.with(|ix| { let mut ix = ix.borrow_mut();
            ix.0 += 1; ix.1.push((tm.elapsed().as_secs_f64() * 1e3).round() / 1e3);
            if let Some(c) = out["next"]["options"].as_array().and_then(|v| v.iter().find(|x| x["chosen"].as_bool() == Some(true))) {
                *ix.2.entry(c["role"].as_str().unwrap_or("?").to_string()).or_default() += 1;
                if c["best_odds"].as_bool() == Some(true) { ix.3 += 1; }
            } });
        let t = self.env.uix(pick["trainee"]["id"].as_u64().unwrap() as u32).unwrap();
        let (a, b) = (pick["parents"][0].as_u64().unwrap() as usize, pick["parents"][1].as_u64().unwrap() as usize);
        let os: Vec<f64> = pick["outcome_sets"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        let wp = self.wins(&st.pool, t, a, b);
        Cur { t, a, b, p: pick["p_all"].as_f64().unwrap(), os, wp, rot: false }
    }
    /// the veteran a failed career leaves and her landed mask
    fn draw(&self, c: &Cur, legs: &[Legacy], keys: &Keys, t: usize) -> (Legacy, usize) {
        let tot: f64 = self.order.iter().map(|&m| c.os[m]).sum();
        let mut u = keys.u(t, STREAM_SET, 0) * tot;
        let (mut mask, mut last) = (usize::MAX, usize::MAX);
        for &m in &self.order { if c.os[m] <= 0.0 { continue; } last = m; if u < c.os[m] { mask = m; break; } u -= c.os[m]; }
        if mask == usize::MAX { mask = last; }
        let mut sparks = vec![];
        for (i, tg) in self.env.sj.targets.iter().enumerate() {
            if mask >> i & 1 == 0 { continue; }
            let x = keys.u(t, STREAM_STAR, i);
            let s = if x < self.star[0] { 1 } else if x < self.star[0] + self.star[1] { 2 } else { 3 };
            sparks.push((tg.group, s));
        }
        let wins: Vec<usize> = c.wp.iter().enumerate().filter(|&(ri, &p)| p > 0.0 && keys.u(t, STREAM_WIN, ri) < p).map(|(ri, _)| ri).collect();
        let anc = |l: &Legacy| Ancestor { chara: l.chara, sparks: l.sparks.clone(), wins: l.wins.clone(), pinks: l.pinks.clone() };
        (Legacy { chara: self.d.umas[c.t].id, borrowed: false, sparks, wins, pinks: vec![], parents: vec![anc(&legs[c.a]), anc(&legs[c.b])], ..Default::default() }, mask)
    }
    fn stars_wins(&self, l: &Legacy) -> (Vec<u8>, Vec<usize>) {
        let st = self.env.sj.targets.iter().map(|t| l.sparks.iter().filter(|s| s.0 == t.group).map(|s| s.1).max().unwrap_or(0)).collect();
        let mut w = l.wins.clone(); w.sort(); w.dedup();
        (st, w)
    }
    fn dominated(&self, st: &St, l: &Legacy) -> bool {
        let (s, w) = self.stars_wins(l);
        st.fam.get(&family(l)).map_or(false, |v| v.iter().any(|(s2, w2)| s2.iter().zip(&s).all(|(x, y)| x >= y) && w.iter().all(|r| w2.binary_search(r).is_ok())))
    }
    /// a veteran joins the pool; `eval`: the surrogate values every new candidate (she is one of its parents)
    fn add_vet(&mut self, st: &mut St, l: Legacy, eval: bool, buf: &mut Vec<f64>) {
        // (her entry as `Env::pool` builds it, win chances and star mix kept)
        self.env.pool_push(&mut st.pool, &l);
        if !l.borrowed { let sw = self.stars_wins(&l); st.fam.entry(family(&l)).or_default().push(sw); }
        st.legs.push(l);
        self.n.added += 1;
        if !eval { return; }
        let vi = st.legs.len() - 1;
        let trainees = self.trainees.clone();
        for &t in &trainees {
            if self.hi[t] * (1.0 + 1e-9) < self.bar(st, t) { continue; }
            for b in 0..vi {
                if !self.legal(st, t, b, vi) { continue; }
                // the allocation-free ceiling first (`fast_ub`, the same bound to rounding)
                if self.fast_ub(st, t, b, vi) * (1.0 + 1e-6) < self.bar(st, t) { self.n.cands += 1; continue; }
                self.consider(st, t, b, vi, buf);
            }
        }
    }
    /// the initial state: the job's pool, every candidate valued (best ceiling first)
    fn initial(&mut self, legs: &[Legacy], base: Base) -> St {
        let mut st = St { legs: vec![], pool: self.env.pool(&[]), fam: Default::default(), best: None, cur: None,
                          rot: Rot { pos: 0, latest: vec![None; self.members.len()] }, mb: vec![None; self.members.len()] };
        let mut buf = vec![];
        for l in legs { self.add_vet(&mut st, l.clone(), false, &mut buf); }
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
            let mut cs = self.env.gen_cands(&st.pool, &[], &trainees, |_, _| true, |ub, _, t, a, b| (ub, t, a, b));
            cs.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
            for (ub, t, a, b) in cs {
                let floor = st.mb.iter().fold(st.best.map_or(0.0, |p| p.val), |m, p| m.min(p.map_or(0.0, |p| p.val)));
                if ub * (1.0 + 1e-9) < floor { break; }
                if ub * (1.0 + 1e-9) < self.bar(&st, t) { continue; }
                self.consider(&mut st, t, a, b, &mut buf);
            }
        }
        st
    }

    /// One audit row: career (t, a, b) valued by the surrogate's table (her cached decks, no refresh) against a fresh
    /// exact deck search there: [kind, table value, exact value, (exact - table) / exact, landed-set total variation
    /// between the two decks, search finished]. Kinds: 0 the pick, 1 a random candidate, 3 a
    /// loop career.
    fn audit_row(&mut self, pool: &Pool, t: usize, a: usize, b: usize, kind: u8) -> Value {
        let (q, nc, c) = self.env.lineage(pool, t, a, b);
        let mut buf = vec![];
        let (tv, di) = self.tval(t, &q, c, &mut buf);
        let (exv, sel, full) = self.exact_best(t, &q, c);
        let sets = |env: &mut Env, sel: &Sel| { env.set_trainee(t, &q); let (j, ext) = env.exact(t, sel, &nc, false, true); env.outcome_sets(j.2, &ext) };
        let tvd = if di == usize::MAX { 1.0 } else {
            let s1 = sets(self.env, &self.cache.decks[t][di].sel.clone()); let s2 = sets(self.env, &sel);
            0.5 * s1.iter().zip(&s2).map(|(x, y)| (x - y).abs()).sum::<f64>() };
        json!([kind, tv, exv, if exv > 0.0 { (exv - tv) / exv } else { 0.0 }, tvd, full])
    }
    /// trainee t's exact best deck at chances q (a full deck search): (value with spark scale c, deck, finished)
    fn exact_best(&mut self, t: usize, q: &[f64], c: f64) -> (f64, Sel, bool) {
        self.env.set_trainee(t, q);
        self.env.work = 0;
        let mut sc = 0;
        let r = self.env.best_deck(t, c, 0.0, &[], &mut sc);
        self.env.last_pv = None;
        match r { DeckRes::Done(v, sel, full) => (c * v, sel, full), _ => panic!("audit: no legal deck") }
    }
    /// The greedy audit at a pick change: the exact career search's pick against the surrogate's, and audit rows for
    /// the pick and `audit_sample` random candidates with the newest veteran as a parent (ceiling at least 0.3 x B; every
    /// candidate of the pool at the start)
    fn audit_greedy(&mut self, st: &mut St, keys: &Keys, t_career: usize) -> Value {
        let cur = self.greedy_cur(st);
        let trainees = self.trainees.clone();
        let sv = self.env.solve(&st.pool, &[], &trainees, false, 1);
        let ex = sv.found.first().map(|f| (f.val, format!("{}:{},{}", self.d.umas[f.t].id, f.a.min(f.b), f.a.max(f.b))));
        let sk = self.key(&cur);
        let differs = ex.as_ref().map_or(true, |x| x.1 != sk);
        let mut rows = vec![self.audit_row(&st.pool.clone(), cur.t, cur.a, cur.b, 0)];
        let bv = st.best.map_or(0.0, |p| p.val);
        let n = st.legs.len(); let vi = n - 1; let all = t_career == 0;
        let mut cs = vec![];
        for &t in &trainees { for a in 0..n { for b in a + 1..n {
            if !all && b != vi { continue; }
            if !self.legal(st, t, a, b) || (t, a, b) == (cur.t, cur.a.min(cur.b), cur.a.max(cur.b)) { continue; }
            if self.fast_ub(st, t, a, b) >= 0.3 * bv { cs.push((t, a, b)); }
        } } }
        for i in 0..self.cfg.audit_sample.min(cs.len()) {
            let j = i + (keys.u(t_career, 9, i) * (cs.len() - i) as f64) as usize;
            let last = cs.len() - 1; cs.swap(i, j.min(last));
            let (t, a, b) = cs[i];
            rows.push(self.audit_row(&st.pool.clone(), t, a, b, 1));
        }
        json!({"t": t_career, "pool": n, "sur": sk, "sur_p": cur.p, "ex": ex.as_ref().map(|x| x.1.clone()), "ex_p": ex.as_ref().map(|x| x.0),
               "differs": differs, "proven": sv.proven, "rows": rows})
    }
    /// the surrogate against exact searches on this pool (the argmax career, and the newest veteran's lineages)
    fn check(&mut self, st: &mut St, all: bool) -> Value {
        let cur = self.greedy_cur(st);
        let trainees = self.trainees.clone();
        let sv = self.env.solve(&st.pool, &[], &trainees, false, 1);
        let ex = sv.found.first().map(|f| (f.val, format!("{}:{},{}", self.d.umas[f.t].id, f.a, f.b)));
        let argmax = json!({"pool": st.legs.len(), "sur": self.key(&cur), "sur_p": cur.p, "sur_val": st.best.map(|p| p.val),
                            "ex": ex.as_ref().map(|x| x.1.clone()), "ex_p": ex.as_ref().map(|x| x.0), "proven": sv.proven});
        let mut rows = vec![];
        if self.cfg.check_lineages > 0 {
            let bv = st.best.map_or(0.0, |p| p.val);
            let n = st.legs.len(); let vi = n - 1;
            // (the legal careers with her, or every one, `gen_cands`' ceilings, at least check_frac x the best)
            let mut cs: Vec<(f64, usize, usize, usize)> = self.env.gen_cands(&st.pool, &[], &trainees, |_, b| all || b == vi, |ub, _, t, a, b| (ub, t, a, b))
                .into_iter().filter(|x| x.0 >= self.cfg.check_frac * bv).collect();
            cs.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
            cs.truncate(self.cfg.check_lineages);
            if let Some(p) = st.best { if !cs.iter().any(|x| (x.1, x.2, x.3) == (p.t, p.a, p.b)) { let (q, c) = self.env.lineage_q(&st.pool, p.t, p.a, p.b); cs.push((self.env.ceiling(p.t, &q, c).unwrap(), p.t, p.a, p.b)); } }
            let mut buf = vec![];
            for (ub, t, a, b) in cs {
                let (q, nc, c) = self.env.lineage(&st.pool, t, a, b);
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
                let sets = |env: &mut Env, sel: &Sel| { env.set_trainee(t, &q); let (j, ext) = env.exact(t, sel, &nc, false, true); env.outcome_sets(j.2, &ext) };
                let tvd = if di == usize::MAX { 1.0 } else {
                    let s1 = sets(self.env, &self.cache.decks[t][di].sel.clone()); let s2 = sets(self.env, &sel);
                    0.5 * s1.iter().zip(&s2).map(|(x, y)| (x - y).abs()).sum::<f64>() };
                let pick = st.best.map_or(false, |p| (p.t, p.a, p.b) == (t, a, b));
                rows.push(json!([ub, rb, tv, exv, (exv - tv) / exv, rb <= tv * (1.0 + self.cfg.margin), tvd, full, pick]));
            }
        }
        json!({"argmax": argmax, "lineages": rows})
    }

    /// Follow `base` (from state st at career t with survival s) until s < eps or tmax careers; with `run` given (the
    /// run index), record pick changes, checks and audits. Returns (sum of S including the tail, careers, tail, final p).
    fn play(&mut self, st: &mut St, base: Base, keys: Keys, mut s: f64, mut t: usize, eps: f64, decide: Option<u32>, rec: &mut Vec<Value>, checks: &mut Vec<Value>) -> (f64, usize, f64, f64, usize) {
        let mut e = 0.0f64; let mut pruned = 0usize; let mut buf = vec![];
        let mut last_key = String::new();
        let exact = self.cfg.policy == "greedy_exact";
        let mut ecur: Option<Rc<Cur>> = None;
        let mut since_check = 0usize;
        if decide.is_some() && self.cfg.check > 0 && base == Base::Greedy { checks.push(self.check(st, true)); }
        loop {
            let cur: Rc<Cur> = if exact { match &ecur { Some(c) => c.clone(), None => { let c = Rc::new(self.exact_cur(st)); ecur = Some(c.clone()); c } } }
                else { match base { Base::Greedy => self.greedy_cur(st), Base::Loop => self.loop_cur(st), Base::Trace => unreachable!("trace runs its own loop"),
                    Base::Index => match &ecur { Some(c) => c.clone(), None => { let c = Rc::new(self.index_cur(st)); ecur = Some(c.clone()); c } } } };
            if decide.is_some() {
                let k = self.key(&cur);
                if k != last_key {
                    rec.push(json!({"t": t, "pick": k, "trainee": self.d.umas[cur.t].name, "p": cur.p, "pool": st.legs.len(), "s": s})); last_key = k;
                    if self.cfg.audit && base == Base::Greedy && !exact { let a = self.audit_greedy(st, &keys, t); self.audits.push(a); }
                }
                if self.cfg.audit && base == Base::Loop && t % self.cfg.audit_every == 0 {
                    let r = self.audit_row(&st.pool.clone(), cur.t, cur.a, cur.b, 3); self.audits.push(json!({"t": t, "pool": st.legs.len(), "rows": [r]}));
                }
                if self.cfg.progress > 0 && t % self.cfg.progress == 0 && t > 0 { eprintln!("  {} run {}: {} careers, pool {}, S {:.4}, p {:.3e}", self.cfg.policy, decide.unwrap(), t, st.legs.len(), s, cur.p); }
            }
            if s < eps || t >= self.cfg.max_careers { let tail = s / cur.p; return (e + tail, t, tail, cur.p, pruned); }
            e += s; s *= 1.0 - cur.p;
            let (vet, _) = self.draw(&cur, &st.legs, &keys, t);
            t += 1;
            match base {
                Base::Loop => {
                    self.add_vet(st, vet, false, &mut buf);
                    if cur.rot { self.rot_update(st); }
                    st.cur = None; ecur = None;
                }
                Base::Trace => unreachable!("trace runs its own loop"),
                // (index: a pruned veteran leaves the pool as it was, so the plan, and its pick, are kept)
                Base::Greedy | Base::Index => {
                    if self.cfg.prune && self.dominated(st, &vet) { pruned += 1; continue; }
                    self.add_vet(st, vet, !exact && base == Base::Greedy, &mut buf);
                    ecur = None;
                    since_check += 1;
                    if decide.is_some() && base == Base::Greedy && self.cfg.check > 0 && since_check >= self.cfg.check { since_check = 0; checks.push(self.check(st, false)); }
                }
            }
        }
    }

}

pub(super) fn simulate<'a>(env: &mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, legs: &[Legacy], trainees: &[usize], sim: &Value) -> Value {
    let cfg: SimJob = match serde_json::from_value(sim.clone()) { Ok(x) => x, Err(e) => return json!({"error": format!("simulate: {e}")}) };
    let ntar = env.ntar;
    if ntar > 12 || env.nu > 10 { return json!({"error": "simulate takes at most 12 targets, 10 of them not guaranteed"}); }
    let base = match cfg.policy.as_str() {
        "greedy" | "greedy_exact" => Base::Greedy,
        "loop" => Base::Loop,
        "trace" => Base::Trace,
        "index" => Base::Index,
        p => return json!({"error": format!("simulate: unknown policy {p}")}),
    };
    let members: Vec<usize> = cfg.loop_members.iter().filter_map(|&id| env.uix(id)).collect();
    if matches!(base, Base::Loop | Base::Trace) && (members.len() < 3 || members.len() != cfg.loop_members.len() || members.iter().any(|m| !env.owned.contains(m))) {
        return json!({"error": "loop: loop_members must be at least three owned umas"});
    }
    let n = d.umas.len();
    let mut mi = vec![None; if base == Base::Trace { n } else { 0 }];
    if base == Base::Trace { for (k, &m) in members.iter().enumerate() { mi[m] = Some(k); } }
    let mut s = Sim::new(env, d, sk, cr, job, cfg.clone(), trainees.to_vec(), members, mi, true, false);
    let t_init = std::time::Instant::now();
    let st0 = s.initial(legs, base);
    if base == Base::Trace { return s.trace(&st0); }
    let init_s = t_init.elapsed().as_secs_f64();
    let mut runs = vec![]; let mut es = vec![];
    let eps = cfg.eps;
    // index: the kept state before the simulation, put back after it (module notes)
    let kept0 = if base == Base::Index { Some(kept_swap(Default::default())) } else { None };
    for run in cfg.first..cfg.first + cfg.runs {
        if !cfg.share_cache && run > cfg.first { s.cache = Cache::new(n); }
        if base == Base::Index { kept_swap(Default::default()); IX.with(|ix| *ix.borrow_mut() = Default::default()); }
        let tm = std::time::Instant::now();
        let n0 = s.n;
        let mut st = st0.clone();
        let (mut rec, mut checks) = (vec![], vec![]);
        let keys = Keys::Main { seed: cfg.seed, run: run as u32 };
        let (e, careers, tail, fp, pruned) = s.play(&mut st, base, keys, 1.0, 0, eps, Some(run as u32), &mut rec, &mut checks);
        let n1 = s.n;
        es.push(e);
        let segs = rec;
        let finalk = segs.last().map(|x| x["pick"].clone()).unwrap_or(Value::Null);
        let wall = tm.elapsed().as_secs_f64();
        eprintln!("{} run {}: E {:.1}, {} careers, tail {:.2}, final p {:.3e}, pool {}, {:.1} s", cfg.policy, run, e, careers, tail, fp, st.legs.len(), wall);
        runs.push(json!({"run": run, "E": e, "careers": careers, "tail": tail, "final_p": fp, "final": finalk, "pool": st.legs.len(), "pruned": pruned,
            "wall_s": wall, "candidates": n1.cands - n0.cands, "ceiling_pass": n1.ceil_pass - n0.ceil_pass, "tables": n1.tables - n0.tables,
            "refreshes": n1.searches - n0.searches, "partial_searches": n1.partial - n0.partial, "segs": segs, "checks": checks, "audits": std::mem::take(&mut s.audits)}));
        if base == Base::Index { let ix = IX.with(|ix| std::mem::take(&mut *ix.borrow_mut()));
            runs.last_mut().unwrap()["index"] = json!({"plans": ix.0, "plan_s": ix.1, "roles": ix.2, "best_odds": ix.3}); }
    }
    if let Some(k) = kept0 { kept_swap(k); }
    json!({"simulate": sim, "policy": cfg.policy, "runs": runs, "E": stats(&es), "init_s": init_s, "decks": s.cache.n_decks(), "deck_tables": s.cache.tables,
           "refreshes": s.n.searches, "schedules": s.sched.len()})
}

#[cfg(test)]
mod tests {
    #[test]
    /// the random numbers are pinned: changing them would change every recorded run
    fn u01_values() {
        assert_eq!(super::u01(&[1, 3, 0, 1, 0]), 0.006101473502044041);
        assert_eq!(super::u01(&[1, 0, 17, 2, 5]), 0.786756654747679);
        assert_eq!(super::u01(&[7, 123456, 99999, 3, 33]), 0.9235248915231418);
    }
}
