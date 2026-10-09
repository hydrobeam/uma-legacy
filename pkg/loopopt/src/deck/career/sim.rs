//! Policy simulator on a surrogate (`simulate` on a career job; docs/SOLVER.md "Policy simulator").
//!
//! The quantity is the one val/policy_sim.mjs measures: under the model, the expected number of careers until the first
//! career whose veteran lands every target, for a decision policy on a fixed starting pool. One process runs every
//! career of every run, so nothing is re-read or re-searched between careers. Per run (derived, as in policy_sim):
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
//! trusted from the cache). `audit` records the surrogate against fresh exact searches on the main path (`audit_row`).
//! The job's `fix` is ignored; its `trainees` are the trainees.
//!
//! Draws are val/policy_sim.mjs's (`u01`, bit for bit): keyed by (seed, run, career index, stream, item), so a run
//! index faces the same uniforms under every policy (common random numbers), and a greedy path equals policy_sim's
//! greedy path for as long as the picks agree. Landed set: inverse CDF over the failed sets ordered fewest-landed first;
//! stars per landed target from `star_dist`; G1 wins per race from the schedule's chance of at least one win. Pruning
//! as in policy_sim: a new veteran is dropped when an owned veteran of the same character with the same two parents
//! has at least her stars on every target and every G1 she won (exact for P(all), see policy_sim's README).
//!
//! Policies: `greedy` (surrogate), `greedy_exact` and `planner` (the solver in process per decision: the best career
//! by P(all), and the page's rollout pick), `loop` (a fixed rotation of `loop_members`: each career's parents are the
//! latest veterans of the previous two members in rotation order; bootstrap below), and `rollout_greedy` /
//! `rollout_loop`: at the decision careers (the first; then every `redecide` careers) a candidate set is scored by
//! running each candidate career and then the base policy (greedy or loop) until S < `cont_eps`, averaged over
//! `samples` paired continuations (sample j draws the same uniforms for every candidate); the lowest wins, the other
//! careers follow the base policy.
//!
//! Loop bootstrap: a member whose two parents (the latest rotation veterans of the previous two members) both exist
//! runs with them; otherwise her parents are the best pair for her by exact P(all) from the pool (the career search
//! with her as the only trainee) with every existing one of those rotation veterans fixed as a parent. A loop career
//! keeps her own schedule (the career model's); the loop planner coordinates schedules across members, this does not.
//! The loop adds every veteran (no pruning): it never scans the pool, and the latest veteran is the one it breeds from.
use super::*;
use super::rollout::Cache;
use std::rc::Rc;
/// the planner's pick on this simulator (`plan` on a career job)
pub(super) mod plan;

fn d_policy() -> String { "greedy".into() }
fn d_runs() -> usize { 16 }
fn d_seed() -> u32 { 1 }
fn d_eps() -> f64 { 1e-3 }
fn d_maxc() -> usize { 100_000 }
fn d_samples() -> usize { 8 }
fn d_ctop() -> usize { 6 }
fn d_cpt() -> usize { 4 }
fn d_crec() -> usize { 2 }
fn d_k() -> usize { 6 }
fn d_frac() -> f64 { 0.3 }
fn d_asample() -> usize { 3 }
fn d_inner() -> usize { 10 }
fn d_aevery() -> usize { 50 }

#[derive(Deserialize, Clone)]
pub(super) struct SimJob {
    /// greedy | greedy_exact | planner | loop | rollout_greedy | rollout_loop
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
    /// E1: every `check` new veterans (and at the start), compare the surrogate with exact searches
    #[serde(default)] check: usize,
    /// E1: lineages checked per check (the newest veteran's candidates, ceiling at least `check_frac` x B, best first)
    #[serde(default)] check_lineages: usize,
    #[serde(default = "d_frac")] check_frac: f64,
    /// rollout_*: paired continuations per candidate, candidates (K best whoever trains, best career of the N best
    /// trainees, best career with each of the R most recent veterans), re-decide every this many careers (0: first only)
    #[serde(default = "d_samples")] samples: usize,
    /// `index2`: builder veterans drawn per veteran, and the builder (a loop member's chara id)
    #[serde(default = "d_inner")] inner: usize,
    #[serde(default)] builder: u32,
    /// `cycle`: the members' chara ids in the order they train, repeated
    #[serde(default)] cycle: Vec<u32>,
    #[serde(default = "d_ctop")] cand_top: usize,
    #[serde(default = "d_cpt")] cand_per_trainee: usize,
    #[serde(default = "d_crec")] cand_recent: usize,
    #[serde(default)] redecide: usize,
    /// survival at which a continuation stops (tail S/p added); default eps
    #[serde(default)] cont_eps: Option<f64>,
    /// planner: rollout width
    #[serde(default = "d_k")] planner_k: usize,
    /// loop: rotation members (chara ids) in order
    #[serde(default)] loop_members: Vec<u32>,
    /// fidelity audit on the main path (`audit_row`): at every pick change of greedy (the exact argmax, the pick and
    /// `audit_sample` random candidates with the newest veteran), at every rollout decision (the chosen and
    /// `audit_sample` random candidates), and every `audit_every` careers of the loop (its career, and its keep-or-replace
    /// decision under exact values)
    #[serde(default)] audit: bool,
    #[serde(default = "d_asample")] audit_sample: usize,
    #[serde(default = "d_aevery")] audit_every: usize,
    /// log progress to stderr every this many careers (0: off)
    #[serde(default)] progress: usize,
}

// ---- val/policy_sim.mjs's random numbers ----
fn fmix(mut h: u32) -> u32 { h ^= h >> 16; h = h.wrapping_mul(0x85eb_ca6b); h ^= h >> 13; h = h.wrapping_mul(0xc2b2_ae35); h ^= h >> 16; h }
/// policy_sim's `u01` bit for bit: 53 bits in [0, 1) from the keys
pub(super) fn u01(keys: &[u32]) -> f64 {
    let (mut a, mut b) = (0x9e37_79b9u32, 0x7f4a_7c15u32);
    for &k in keys { a = fmix((a ^ k).wrapping_add(0x632b_e5ab)); b = fmix(b.wrapping_add(k.wrapping_mul(0x27d4_eb2f)) ^ a); }
    (a as f64 * 2f64.powi(21) + (b >> 11) as f64) / 2f64.powi(53)
}
const STREAM_SET: u32 = 1;
const STREAM_STAR: u32 = 2;
const STREAM_WIN: u32 = 3;
/// Whose uniforms: a run's own path (policy_sim's keys), or continuation sample j of the decision at career t0
#[derive(Clone, Copy)]
enum Keys { Main { seed: u32, run: u32 }, Sample { seed: u32, run: u32, t0: u32, j: u32 } }
impl Keys {
    fn u(&self, t: usize, stream: u32, item: usize) -> f64 {
        match *self {
            Keys::Main { seed, run } => u01(&[seed, run, t as u32, stream, item as u32]),
            Keys::Sample { seed, run, t0, j } => u01(&[seed, run, t as u32, stream, item as u32, 0x5a3d, t0, j]),
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
struct St { legs: Vec<Legacy>, pool: Pool, gpc: Vec<Vec<f64>>, fam: crate::FxMap<String, Vec<(Vec<u8>, Vec<usize>)>>, best: Option<Pick>, cur: Option<Rc<Cur>>, rot: Rot,
            /// `next`: each loop member's best career (by member index)
            mb: Vec<Option<Pick>>,
            /// careers each candidate trained since her best career last changed (`stale`)
            since: Vec<usize> }

#[derive(Clone, Copy, PartialEq)]
enum Base { Greedy, Loop, Next }
#[derive(Default, Clone, Copy)]
struct Count { cands: usize, ceil_pass: usize, tables: usize, searches: usize, partial: usize, added: usize }

struct Sim<'s, 'a> {
    env: &'s mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value,
    cfg: SimJob, cache: Cache, trainees: Vec<usize>, hi: Vec<f64>, order: Vec<usize>, star: [f64; 3],
    sched: crate::FxMap<(usize, Vec<usize>), Rc<Vec<f64>>>, n: Count, members: Vec<usize>,
    /// per trainee and outfit, per uncertain target (A, B): her ceiling factor is A - B (1 - q) (`fast_ub`)
    fastc: Vec<Vec<Vec<(f64, f64)>>>, fixed: f64,
    /// `next`: trainee → loop member index (empty under the other policies)
    mi: Vec<Option<usize>>,
    /// the career index and run of the decision being made, and `pick_index`'s last decision (keyed by the best values)
    t_now: usize, run_now: u32, dcache: Option<(Vec<u64>, Option<usize>)>,
    /// on the main path (not inside a rollout's continuations): audits are taken only there
    main: bool, audits: Vec<Value>,
    /// `plan` only (false for every policy here): a trainee without a cached deck gets her best deck searched without
    /// a bar (so she has one from then on), instead of a search at the bar that caches nothing when it ends Bounded
    fill: bool,
}

fn canon(chara: u32, sparks: &[(u32, u8)], pinks: &[(u32, u8)], wins: &[usize]) -> String {
    let (mut s, mut p, mut w) = (sparks.to_vec(), pinks.to_vec(), wins.to_vec());
    s.sort(); p.sort(); w.sort();
    format!("{chara}|{s:?}|{p:?}|{w:?}")
}
/// policy_sim's family: the character and her two parents as recorded
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

impl<'s, 'a> Sim<'s, 'a> {
    fn key(&self, c: &Cur) -> String { format!("{}:{},{}", self.d.umas[c.t].id, c.a, c.b) }
    /// P(all) at chances q with spark scale c through trainee t's cached decks: (value, deck index; usize::MAX: none)
    fn tval(&self, t: usize, q: &[f64], c: f64, buf: &mut Vec<f64>) -> (f64, usize) {
        let x: Vec<f64> = self.env.uncv.iter().map(|&i| 1.0 - q[i]).collect();
        let (mut best, mut bd) = (0.0f64, usize::MAX);
        for (di, dk) in self.cache.decks[t].iter().enumerate() {
            for tab in &dk.tab { let v = interp_in(tab, &x, buf); if bd == usize::MAX || v > best { best = v; bd = di; } }
        }
        (c * best, bd)
    }
    /// trainee t's best cached deck at chances q (spark scale c, ceiling ub) after the refresh rule (module notes), when
    /// it can reach the bar th: (value, deck index); None when it provably cannot
    fn deck_for(&mut self, t: usize, q: &[f64], c: f64, ub: f64, th: f64, buf: &mut Vec<f64>) -> Option<(f64, usize)> {
        let mut rb = ub;
        for r in &self.cache.refs[t] { rb = rb.min(c * r.val * self.env.rise(t, q, &r.q)); if rb * (1.0 + 1e-9) < th { return None; } }
        if th > 0.0 && self.env.ruled[self.env.cls[t]].iter().any(|(rq, s)| EXACT_RHO * c * s * self.env.rise(t, q, rq) * (1.0 + 2e-9) < th) { return None; }
        self.n.tables += 1;
        let (mut tv, mut di) = self.tval(t, q, c, buf);
        if di == usize::MAX || rb > tv * (1.0 + self.cfg.margin) {
            self.env.set_trainee(t, q);
            let starts: Vec<Sel> = self.cache.decks[t].iter().map(|x| x.sel.clone()).collect();
            let sr: Vec<&Sel> = starts.iter().collect();
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
        let ti = Some(t);
        let mut miss = [1.0f64; 12]; let mut nc = [0i32; 12];
        let sh = 3.0 * shared(&p.me[a], &p.me[b]);
        let mut slot = |stars: &[u8], aff: f64| { for k in 0..ntar { let s = stars[k]; if s > 0 { nc[k] += 1; miss[k] *= 1.0 - (0.03 * s as f64 * (1.0 + aff / 100.0)).min(1.0); } } };
        for (x, y) in [(a, b), (b, a)] {
            let px = &p.me[x];
            let (mut rel, mut race) = (env.pair(ti, px.ix) + env.pair(px.ix, p.me[y].ix), sh);
            for (gi, g) in p.gp[x].iter().enumerate() {
                let (gr, gc) = (env.tri(ti, px.ix, g.ix), st.gpc[x][gi]);
                rel += gr; race += gc;
                slot(&g.stars, gr + gc);
            }
            slot(&px.stars, rel + race);
        }
        let mut c = 1.0f64;
        for k in 0..ntar { c *= 1.1f64.powi(nc[k] - 6); }
        let mut best = 0.0f64;
        for ab in &self.fastc[t] {
            let mut v = 1.0f64;
            for (j, &i) in env.uncv.iter().enumerate() { v *= ab[j].0 - ab[j].1 * miss[i] * miss[i]; }
            if v > best { best = v; }
        }
        EXACT_RHO * c * self.fixed * best
    }
    /// value candidate (t, a, b) against the best so far
    /// the value a career of trainee t must beat to matter: the best overall, or under `next` a member's own best
    fn bar(&self, st: &St, t: usize) -> f64 {
        match self.mi.get(t).copied().flatten() { Some(k) => st.mb[k].map_or(0.0, |p| p.val), None => st.best.map_or(0.0, |p| p.val) }
    }
    fn consider(&mut self, st: &mut St, t: usize, a: usize, b: usize, buf: &mut Vec<f64>) {
        let bv = self.bar(st, t);
        self.n.cands += 1;
        let (q, _, c) = self.env.lineage(&st.pool, t, a, b);
        let Some(ub) = self.env.ceiling(t, &q, c) else { return };
        static CHECK_UB: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *CHECK_UB.get_or_init(|| std::env::var("SIM_CHECK_UB").is_ok()) { let f = self.fast_ub(st, t, a, b); assert!((f - ub).abs() <= 1e-12 * ub, "fast_ub {f} vs ceiling {ub}"); }
        if ub * (1.0 + 1e-9) < bv { return; }
        self.n.ceil_pass += 1;
        let Some((tv, di)) = self.deck_for(t, &q, c, ub, bv, buf) else { return };
        let pk = Pick { val: tv, t, a: a.min(b), b: a.max(b), d: di };
        if let Some(k) = self.mi.get(t).copied().flatten() { if tv > bv { st.mb[k] = Some(pk); st.cur = None; } }
        if tv > st.best.map_or(0.0, |p| p.val) { st.best = Some(pk); st.cur = None; }
    }
    fn legal(&self, st: &St, t: usize, a: usize, b: usize) -> bool {
        let tid = self.d.umas[t].id; let (ca, cb) = (st.pool.me[a].chara, st.pool.me[b].chara);
        a != b && ca != cb && ca != tid && cb != tid && !(st.pool.borrowed[a] && st.pool.borrowed[b])
    }
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
    /// `next`: the page's rule (sim::plan::next_rule) on the surrogate: the best career of each loop member, except the
    /// newest veteran's member (always under `next_noretry`; under `next` only when another member's best career has
    /// that veteran as a parent); the best of those, or the best career overall when its trainee is no member and it
    /// has at least NEXT_OUTSIDE times those odds
    fn next_cur(&mut self, st: &mut St) -> Rc<Cur> {
        if let Some(c) = &st.cur { return c.clone(); }
        let pk = match self.cfg.policy.as_str() {
            "pick_index" => self.pick_index(st),
            "cycle" => { let id = self.cfg.cycle[self.t_now % self.cfg.cycle.len()];
                let k = self.env.uix(id).and_then(|u| self.mi[u]).expect("cycle: ids must be loop members"); st.mb[k].or(st.best).expect("no legal career") }
            // the best career's trainee counts too (a candidate when she is in the list): fresh, she has the best odds
            "stale" => { let g = st.best.expect("no legal career");
                (0..self.members.len()).filter(|&k| st.since[k] == 0).filter_map(|k| st.mb[k])
                    .fold(None::<Pick>, |m, p| if m.map_or(true, |m| p.val > m.val) { Some(p) } else { m }).unwrap_or(g) }
            _ => self.next_pick(st),
        };
        let sel = self.cache.decks[pk.t][pk.d].sel.clone();
        let c = Rc::new(self.make_cur(&st.pool, pk.t, pk.a, pk.b, &sel, false));
        st.cur = Some(c.clone());
        c
    }
    /// `pick_index`: the careers-saved index per candidate builder (`samples` veterans of her best career, then the best
    /// career forever); the highest above 0 trains, else the best career. Decided again only when a best value changes.
    fn pick_index(&mut self, st: &mut St) -> Pick {
        let g = st.best.expect("no legal career");
        let key: Vec<u64> = std::iter::once(g.val.to_bits()).chain(st.mb.iter().map(|p| p.map_or(0, |p| p.val.to_bits()))).collect();
        if let Some((k, ch)) = &self.dcache { if *k == key { return ch.map_or(g, |i| st.mb[i].unwrap()); } }
        let (p0, n) = (g.val, self.cfg.samples);
        let mut best: Option<(f64, usize)> = None;
        for k in 0..self.members.len() {
            let Some(mk) = st.mb[k] else { continue };
            if mk.t == g.t { continue; }
            let sel = self.cache.decks[mk.t][mk.d].sel.clone();
            let c = self.make_cur(&st.pool, mk.t, mk.a, mk.b, &sel, false);
            let mut s1 = 0.0f64;
            for j in 0..n {
                let keys = Keys::Sample { seed: self.cfg.seed, run: self.run_now, t0: self.t_now as u32, j: (k * 1000 + j) as u32 };
                let (vet, _) = self.draw(&c, &st.legs, &keys, 0);
                let mut st2 = st.clone(); let mut buf = vec![];
                self.add_vet(&mut st2, vet, true, &mut buf);
                s1 += 1.0 / st2.best.unwrap().val;
            }
            let idx = 1.0 / p0 - (1.0 + (1.0 - c.p) * s1 / n as f64);
            if idx > 0.0 && best.map_or(true, |b| idx > b.0) { best = Some((idx, k)); }
        }
        let ch = best.map(|b| b.1);
        self.dcache = Some((key, ch));
        ch.map_or(g, |i| st.mb[i].unwrap())
    }
    /// the `next*` policies' pick (sim::plan's former loop rule)
    fn next_pick(&mut self, st: &St) -> Pick {
        let last = st.legs.len() - 1;
        let nk = self.env.uix(st.legs[last].chara).and_then(|u| self.mi[u]);
        // `next_retry1`: at most one retry per step (none when the career before the newest trained her member too)
        let again = last > 0 && self.env.uix(st.legs[last - 1].chara).and_then(|u| self.mi[u]) == nk;
        let retry = self.cfg.policy == "next" || (self.cfg.policy == "next_retry1" && !again);
        let excl = nk.filter(|&k| !retry || st.mb.iter().enumerate().any(|(j, p)| j != k && p.map_or(false, |p| p.a == last || p.b == last)));
        let lb = st.mb.iter().enumerate().filter(|&(j, _)| Some(j) != excl).filter_map(|(_, p)| *p)
            .fold(None::<Pick>, |m, p| if m.map_or(true, |m| p.val > m.val) { Some(p) } else { m });
        let g = st.best.expect("no legal career");
        match lb { Some(l) if self.mi[g.t].is_none() && g.val >= plan::NEXT_OUTSIDE * l.val => g, Some(l) => l, None => g }
    }
    /// `index`: per loop member X, her best career run once and then the best career forever (stationary):
    /// E_X = 1 + (1 - p_X) E_v[1 / p*(pool + v)], v her veteran (`samples` draws); index = 1/p* - E_X (careers saved
    /// against running today's best forever); `record` = P(v raises the best odds); `member_improved`: P(v raises each
    /// member's best)
    fn index(&mut self, st0: &St) -> Value {
        let g = st0.best.expect("no legal career");
        let sel = self.cache.decks[g.t][g.d].sel.clone();
        let p0 = self.make_cur(&st0.pool, g.t, g.a, g.b, &sel, false).p;
        let (n, nm) = (self.cfg.samples, self.members.len());
        let mut rows = vec![];
        for k in 0..nm {
            let Some(mk) = st0.mb[k] else { rows.push(json!({"member": self.d.umas[self.members[k]].name, "error": "no career"})); continue };
            let sel = self.cache.decks[mk.t][mk.d].sel.clone();
            let c = self.make_cur(&st0.pool, mk.t, mk.a, mk.b, &sel, false);
            let tm = std::time::Instant::now();
            let (mut s1, mut s2, mut imp) = (0.0f64, 0.0f64, 0usize);
            let mut mimp = vec![0usize; nm];
            for j in 0..n {
                let keys = Keys::Sample { seed: self.cfg.seed, run: k as u32, t0: 0, j: j as u32 };
                let (vet, _) = self.draw(&c, &st0.legs, &keys, 0);
                let mut st = st0.clone(); let mut buf = vec![];
                self.add_vet(&mut st, vet, true, &mut buf);
                let b = st.best.unwrap();
                let p1 = if (b.t, b.a, b.b) == (g.t, g.a, g.b) { p0 } else { let sel = self.cache.decks[b.t][b.d].sel.clone(); self.make_cur(&st.pool, b.t, b.a, b.b, &sel, false).p };
                if p1 > p0 * (1.0 + 1e-12) { imp += 1; }
                for i in 0..nm { if st.mb[i].map(|x| x.val) > st0.mb[i].map(|x| x.val).map(|v| v * (1.0 + 1e-12)) { mimp[i] += 1; } }
                let x = 1.0 / p1; s1 += x; s2 += x * x;
            }
            let nf = n as f64; let mean = s1 / nf; let sd = ((s2 / nf - mean * mean).max(0.0) * nf / (nf - 1.0)).sqrt();
            let ex = 1.0 + (1.0 - c.p) * mean;
            rows.push(json!({"member": self.d.umas[mk.t].name, "parents": [c.a, c.b], "p": c.p, "careers_if_now": ex, "se": (1.0 - c.p) * sd / nf.sqrt(),
                "index": 1.0 / p0 - ex, "gain": (1.0 - p0) * (1.0 / p0 - mean), "cost": (p0 - c.p) * mean, "record": imp as f64 / nf,
                "member_improved": mimp.iter().enumerate().map(|(i, &m)| json!({"member": self.d.umas[self.members[i]].name, "frac": m as f64 / nf})).collect::<Vec<_>>(),
                "seconds": tm.elapsed().as_secs_f64()}));
        }
        json!({"index": rows, "p_best": p0, "best": {"trainee": self.d.umas[g.t].name, "parents": [g.a, g.b]}, "samples": n, "baseline_careers": 1.0 / p0})
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
                    let base = plan::ScoreBase { cands: &one, c1: vec![1.0], m23: &[0.0], v0: 0.0, pnow: vec![pa], cont: vec![], combos: vec![] };
                    let (_, _, _, stt, _) = plan::score_career(self, self.d, &base, &st.legs, &st.pool, pk.t, pk.a, pk.b, &sel, &mut buf);
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
    /// `index2`: the grandparent route. Per loop member X other than the builder B: her veteran v (`samples` draws from
    /// her best career), then B's best career with v as a parent, her veteran w (`inner` draws), then the best career
    /// forever: E = 1 + (1 - p_X)(1 + (1 - p_B|v) E_w[1 / p*(pool + v + w)]). Paired with the same w draws from B's
    /// usual best career (v in the pool, not her parent): the difference is v's value as a grandparent.
    fn index2(&mut self, st0: &St) -> Value {
        let g = st0.best.expect("no legal career");
        let sel = self.cache.decks[g.t][g.d].sel.clone();
        let p0 = self.make_cur(&st0.pool, g.t, g.a, g.b, &sel, false).p;
        let bu = self.env.uix(self.cfg.builder).expect("index2: builder");
        let bk = self.mi[bu].expect("index2: the builder must be a loop member");
        let pb = st0.mb[bk].expect("index2: no builder career");
        let sel = self.cache.decks[bu][pb.d].sel.clone();
        let cb0 = self.make_cur(&st0.pool, bu, pb.a, pb.b, &sel, false);
        let (n1, n2, nm) = (self.cfg.samples, self.cfg.inner, self.members.len());
        let mut rows = vec![];
        for k in 0..nm {
            if k == bk { continue; }
            let Some(mk) = st0.mb[k] else { continue };
            let sel = self.cache.decks[mk.t][mk.d].sel.clone();
            let c = self.make_cur(&st0.pool, mk.t, mk.a, mk.b, &sel, false);
            let tm = std::time::Instant::now();
            let (mut ew, mut eo, mut dif, mut dif2, mut rw, mut ro, mut pbv) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0usize, 0usize, 0.0f64);
            let mut buf = vec![];
            for i in 0..n1 {
                let kv = Keys::Sample { seed: self.cfg.seed, run: 100 + k as u32, t0: 1, j: i as u32 };
                let (v, _) = self.draw(&c, &st0.legs, &kv, 0);
                let mut st1 = st0.clone();
                self.add_vet(&mut st1, v, true, &mut buf);
                let vi = st1.legs.len() - 1;
                // B's best career with v as a parent
                let mut best: Option<(f64, usize, usize)> = None;
                for b in 0..vi {
                    if !self.legal(&st1, bu, b, vi) { continue; }
                    let (q, _, cc) = self.env.lineage(&st1.pool, bu, b, vi);
                    let Some(ub) = self.env.ceiling(bu, &q, cc) else { continue };
                    let th = best.map_or(0.0, |x| x.0);
                    if let Some((tv, di)) = self.deck_for(bu, &q, cc, ub, th, &mut buf) { if tv > th { best = Some((tv, b, di)); } }
                }
                let Some((_, b, di)) = best else { continue };
                let sel = self.cache.decks[bu][di].sel.clone();
                let cv = self.make_cur(&st1.pool, bu, b, vi, &sel, false);
                pbv += cv.p;
                let mut m = [0.0f64; 2];
                for (x, cb) in [&cv, &cb0].into_iter().enumerate() {
                    for jj in 0..n2 {
                        let kw = Keys::Sample { seed: self.cfg.seed, run: 100 + k as u32, t0: 2 + i as u32, j: jj as u32 };
                        let (w, _) = self.draw(cb, &st1.legs, &kw, 1);
                        let mut st2 = st1.clone();
                        self.add_vet(&mut st2, w, true, &mut buf);
                        let bb = st2.best.unwrap();
                        let p2 = if (bb.t, bb.a, bb.b) == (g.t, g.a, g.b) { p0 } else { let sel = self.cache.decks[bb.t][bb.d].sel.clone(); self.make_cur(&st2.pool, bb.t, bb.a, bb.b, &sel, false).p };
                        if p2 > p0 * (1.0 + 1e-12) { if x == 0 { rw += 1 } else { ro += 1 } }
                        m[x] += 1.0 / p2 / n2 as f64;
                    }
                }
                let e_w = 1.0 + (1.0 - c.p) * (1.0 + (1.0 - cv.p) * m[0]);
                let e_o = 1.0 + (1.0 - c.p) * (1.0 + (1.0 - cb0.p) * m[1]);
                ew += e_w; eo += e_o; dif += e_o - e_w; dif2 += (e_o - e_w) * (e_o - e_w);
            }
            let nf = n1 as f64; let md = dif / nf; let sd = ((dif2 / nf - md * md).max(0.0) * nf / (nf - 1.0)).sqrt();
            rows.push(json!({"member": self.d.umas[mk.t].name, "p": c.p, "builder_p_with_v": pbv / nf, "builder_p_usual": cb0.p,
                "careers_gp_route": ew / nf, "careers_usual_builder": eo / nf, "gp_value": md, "gp_value_se": sd / nf.sqrt(),
                "index_gp_route": 1.0 / p0 - ew / nf, "record_gp_route": rw as f64 / (nf * n2 as f64), "record_usual": ro as f64 / (nf * n2 as f64),
                "seconds": tm.elapsed().as_secs_f64()}));
        }
        json!({"index2": rows, "p_best": p0, "baseline_careers": 1.0 / p0, "builder": self.d.umas[bu].name, "samples": n1, "inner": n2})
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
            let (q, _, c) = self.env.lineage(&st.pool, t, a, b);
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
                let (q, _, c) = s.env.lineage(&st.pool, t, v.min(o), v.max(o));
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
                    let (q, _, c) = s.env.lineage(&st.pool, t, v.min(o), v.max(o));
                    tot += s.exact_best(t, &q, c).0;
                }
                tot
            };
            let (eo, en) = (ex(self, old), ex(self, vi));
            self.audits.push(json!({"t": st.rot.pos, "rotation": [so, sn, eo, en, (sn > so) == (en > eo)]}));
        }
        if sn > so { st.rot.latest[m] = Some(vi); }
    }
    /// the in-process solver's pick (greedy_exact: the best career by P(all); planner: the page's rollout pick)
    fn exact_cur(&mut self, st: &St, planner: bool) -> Cur {
        let mut job = self.job.clone();
        let o = job.as_object_mut().unwrap();
        for k in ["simulate", "policy", "p_now", "p_minus1_now"] { o.remove(k); }
        o.insert("legacies".into(), Value::Array(st.legs.iter().map(leg_json).collect()));
        o.insert("lookahead".into(), json!(false));
        if planner { o.insert("rollout".into(), json!(self.cfg.planner_k)); }
        else { o.insert("rollout".into(), json!(0)); o.insert("top".into(), json!(1)); o.insert("per_trainee".into(), json!(false)); }
        let out = run_career(self.d, self.sk, self.cr, &job);
        let pick = if planner { out["rollout"]["choice"].clone() } else { out["results"][0].clone() };
        assert!(pick.is_object(), "solver gave no pick: {}", out.to_string().chars().take(300).collect::<String>());
        let t = self.env.uix(pick["trainee"]["id"].as_u64().unwrap() as u32).unwrap();
        let (a, b) = (pick["parents"][0].as_u64().unwrap() as usize, pick["parents"][1].as_u64().unwrap() as usize);
        let os: Vec<f64> = pick["outcome_sets"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        let wp = self.wins(&st.pool, t, a, b);
        Cur { t, a, b, p: pick["p_all"].as_f64().unwrap(), os, wp, rot: false }
    }
    /// the veteran a failed career leaves (policy_sim's `drawVeteran`) and her landed mask
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
        let env = &*self.env;
        st.pool.me.push(env.who(l.chara, &l.sparks, &l.wins, &[], &l.pinks, None));
        st.pool.gp.push(l.parents.iter().map(|g| env.who(g.chara, &g.sparks, &g.wins, &[], &g.pinks, None)).collect());
        st.pool.borrowed.push(l.borrowed);
        let x = st.pool.me.len() - 1;
        st.gpc.push(st.pool.gp[x].iter().map(|g| 3.0 * shared(&st.pool.me[x], g)).collect());
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
        let pool = self.env.pool(legs);
        let mut st = St { legs: vec![], pool: Pool { me: vec![], gp: vec![], borrowed: vec![] }, gpc: vec![], fam: Default::default(), best: None, cur: None,
                          rot: Rot { pos: 0, latest: vec![None; self.members.len()] }, mb: vec![None; self.members.len()], since: vec![0; self.members.len()] };
        let mut buf = vec![];
        for l in legs { self.add_vet(&mut st, l.clone(), false, &mut buf); }
        if base != Base::Loop {
            // seed the cache with the career search's fully checked decks
            self.env.use_seen = false;
            let trainees = self.trainees.clone();
            let sv = self.env.solve(&pool, &[], &trainees, false, 1);
            self.env.use_seen = true;
            self.cache.absorb(self.env, &sv.refs);
            let mut cs = vec![];
            let n = st.legs.len();
            for &t in &trainees { for a in 0..n { for b in a + 1..n { if self.legal(&st, t, a, b) {
                let (q, _, c) = self.env.lineage(&st.pool, t, a, b);
                if let Some(ub) = self.env.ceiling(t, &q, c) { cs.push((ub, t, a, b)); }
            } } } }
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
    /// between the two decks, search finished]. Kinds: 0 the pick, 1 a random candidate, 2 a rollout candidate, 3 a
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
    /// E1: the surrogate against exact searches on this pool (the argmax career, and the newest veteran's lineages)
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
            let mut cs = vec![];
            for &t in &trainees { for a in 0..n { for b in a + 1..n {
                if !all && b != vi { continue; }
                if !self.legal(st, t, a, b) { continue; }
                let (q, _, c) = self.env.lineage(&st.pool, t, a, b);
                if let Some(ub) = self.env.ceiling(t, &q, c) { if ub >= self.cfg.check_frac * bv { cs.push((ub, t, a, b)); } }
            } } }
            cs.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
            cs.truncate(self.cfg.check_lineages);
            if let Some(p) = st.best { if !cs.iter().any(|x| (x.1, x.2, x.3) == (p.t, p.a, p.b)) { let (q, _, c) = self.env.lineage(&st.pool, p.t, p.a, p.b); cs.push((self.env.ceiling(p.t, &q, c).unwrap(), p.t, p.a, p.b)); } }
            let mut buf = vec![];
            for (ub, t, a, b) in cs {
                let (q, nc, c) = self.env.lineage(&st.pool, t, a, b);
                let rb = self.cache.refs[t].iter().map(|r| c * r.val * self.env.rise(t, &q, &r.q)).fold(ub, f64::min);
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

    /// Follow `base` (from state st at career t with survival s) until s < eps or tmax careers; with `run` given, the
    /// rollout decisions too (main path). Returns (sum of S including the tail, careers, tail, final p).
    fn play(&mut self, st: &mut St, base: Base, keys: Keys, mut s: f64, mut t: usize, eps: f64, decide: Option<u32>, rec: &mut Vec<Value>, checks: &mut Vec<Value>) -> (f64, usize, f64, f64, usize) {
        let mut e = 0.0f64; let mut pruned = 0usize; let mut buf = vec![];
        let mut last_key = String::new();
        let exact = matches!(self.cfg.policy.as_str(), "greedy_exact" | "planner");
        let mut ecur: Option<Rc<Cur>> = None;
        let mut since_check = 0usize;
        if decide.is_some() && self.cfg.check > 0 && base == Base::Greedy { checks.push(self.check(st, true)); }
        loop {
            self.t_now = t; if let Some(r) = decide { self.run_now = r; }
            let due = decide.is_some() && self.cfg.policy.starts_with("rollout") && (t == 0 || (self.cfg.redecide > 0 && t % self.cfg.redecide == 0));
            let cur: Rc<Cur> = if due { Rc::new(self.decide(st, base, decide.unwrap(), t, rec)) }
                else if exact { match &ecur { Some(c) => c.clone(), None => { let c = Rc::new(self.exact_cur(st, self.cfg.policy == "planner")); ecur = Some(c.clone()); c } } }
                else { match base { Base::Greedy => self.greedy_cur(st), Base::Loop => self.loop_cur(st), Base::Next => self.next_cur(st) } };
            if decide.is_some() {
                let k = self.key(&cur);
                if k != last_key {
                    rec.push(json!({"t": t, "pick": k, "trainee": self.d.umas[cur.t].name, "p": cur.p, "pool": st.legs.len(), "s": s})); last_key = k;
                    if self.cfg.audit && base == Base::Greedy && !exact && !due { let a = self.audit_greedy(st, &keys, t); self.audits.push(a); }
                }
                if self.cfg.audit && base == Base::Loop && !due && t % self.cfg.audit_every == 0 {
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
                Base::Next => {
                    let before: Vec<Option<(usize, usize, usize, u64)>> = st.mb.iter().map(|p| p.map(|p| (p.t, p.a, p.b, p.val.to_bits()))).collect();
                    let tk = self.mi.get(cur.t).copied().flatten();
                    self.add_vet(st, vet, true, &mut buf);
                    if let Some(k) = tk { st.since[k] += 1; }
                    for (k, b) in before.iter().enumerate() { if st.mb[k].map(|p| (p.t, p.a, p.b, p.val.to_bits())) != *b { st.since[k] = 0; } }
                    st.cur = None; ecur = None;
                }
                Base::Greedy => {
                    if self.cfg.prune && self.dominated(st, &vet) { pruned += 1; continue; }
                    self.add_vet(st, vet, !exact, &mut buf);
                    ecur = None;
                    since_check += 1;
                    if decide.is_some() && self.cfg.check > 0 && since_check >= self.cfg.check { since_check = 0; checks.push(self.check(st, false)); }
                }
            }
        }
    }

    /// candidate careers for a rollout decision: (trainee, parents, deck), the base policy's own pick first
    fn candidates(&mut self, st: &mut St, base: Base) -> Vec<Rc<Cur>> {
        let own = match base { Base::Greedy => self.greedy_cur(st), Base::Loop => self.loop_cur(st), Base::Next => self.next_cur(st) };
        let mut out = vec![own];
        let trainees = self.trainees.clone();
        let mut fs: Vec<Found> = vec![];
        self.env.use_seen = true;
        let sv = self.env.solve(&st.pool, &[], &trainees, false, self.cfg.cand_top); self.cache.absorb(self.env, &sv.refs); fs.extend(sv.found);
        let sv = self.env.solve(&st.pool, &[], &trainees, true, self.cfg.cand_per_trainee); self.cache.absorb(self.env, &sv.refs); fs.extend(sv.found);
        let n = st.legs.len();
        for v in (0..n).rev().take(self.cfg.cand_recent) {
            let sv = self.env.solve(&st.pool, &[v], &trainees, false, 1); self.cache.absorb(self.env, &sv.refs); fs.extend(sv.found);
        }
        for f in fs {
            if out.iter().any(|c| (c.t, c.a.min(c.b), c.a.max(c.b)) == (f.t, f.a.min(f.b), f.a.max(f.b))) { continue; }
            let c = self.make_cur(&st.pool, f.t, f.a, f.b, &f.sel, false);
            out.push(Rc::new(c));
        }
        out
    }
    /// a rollout decision at career t0 (module notes)
    fn decide(&mut self, st: &mut St, base: Base, run: u32, t0: usize, rec: &mut Vec<Value>) -> Cur {
        let tm = std::time::Instant::now();
        let cands = self.candidates(st, base);
        let ce = self.cfg.cont_eps.unwrap_or(self.cfg.eps);
        let m = self.cfg.samples;
        let mut scores = vec![vec![0.0f64; m]; cands.len()];
        let was_main = self.main; self.main = false;
        for (ci, c) in cands.iter().enumerate() {
            for j in 0..m {
                let keys = Keys::Sample { seed: self.cfg.seed, run, t0: t0 as u32, j: j as u32 };
                let mut st2 = st.clone();
                let (vet, _) = self.draw(c, &st2.legs, &keys, 0);
                let mut buf = vec![];
                match base {
                    Base::Greedy => { if !(self.cfg.prune && self.dominated(&st2, &vet)) { self.add_vet(&mut st2, vet, true, &mut buf); } }
                    Base::Next => { self.add_vet(&mut st2, vet, true, &mut buf); st2.cur = None; }
                    Base::Loop => { self.add_vet(&mut st2, vet, false, &mut buf);
                        if c.rot { self.rot_update(&mut st2); }
                        st2.cur = None; }
                }
                let (e, ..) = self.play(&mut st2, base, keys, 1.0 - c.p, 1, ce, None, &mut vec![], &mut vec![]);
                scores[ci][j] = 1.0 + e;
            }
        }
        self.main = was_main;
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let pick = (0..cands.len()).min_by(|&x, &y| mean(&scores[x]).partial_cmp(&mean(&scores[y])).unwrap()).unwrap();
        let rows: Vec<Value> = cands.iter().zip(&scores).map(|(c, s)| {
            let diff: Vec<f64> = s.iter().zip(&scores[0]).map(|(x, y)| x - y).collect();
            json!({"pick": self.key(c), "p": c.p, "score": mean(s), "vs_base": stats(&diff)})
        }).collect();
        if self.cfg.audit && self.main {
            let mut idx: Vec<usize> = (0..cands.len()).filter(|&i| i != pick).collect();
            let keys = Keys::Sample { seed: self.cfg.seed, run, t0: t0 as u32, j: 999 };
            let mut chosen = vec![pick];
            for i in 0..self.cfg.audit_sample.min(idx.len()) { let j = i + (keys.u(0, 9, i) * (idx.len() - i) as f64) as usize; let last = idx.len() - 1; idx.swap(i, j.min(last)); chosen.push(idx[i]); }
            let pool = st.pool.clone();
            let rows: Vec<Value> = chosen.iter().map(|&i| { let c = &cands[i]; self.audit_row(&pool, c.t, c.a, c.b, 2) }).collect();
            self.audits.push(json!({"t": t0, "pool": st.legs.len(), "decision": true, "rows": rows}));
        }
        rec.push(json!({"decision": t0, "chosen": pick, "differs": pick != 0, "candidates": rows, "seconds": tm.elapsed().as_secs_f64()}));
        let c = &cands[pick];
        Cur { t: c.t, a: c.a, b: c.b, p: c.p, os: c.os.clone(), wp: c.wp.clone(), rot: c.rot }
    }
}

pub(super) fn simulate<'a>(env: &mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, legs: &[Legacy], trainees: &[usize], sim: &Value) -> Value {
    let cfg: SimJob = match serde_json::from_value(sim.clone()) { Ok(x) => x, Err(e) => return json!({"error": format!("simulate: {e}")}) };
    let ntar = env.ntar;
    if ntar > 12 || env.nu > 10 { return json!({"error": "simulate takes at most 12 targets, 10 of them not guaranteed"}); }
    let base = match cfg.policy.as_str() {
        "greedy" | "greedy_exact" | "planner" | "rollout_greedy" => Base::Greedy,
        "loop" | "rollout_loop" => Base::Loop,
        "next" | "next_noretry" | "next_retry1" | "index" | "index2" | "pick_index" | "stale" | "cycle" | "trace" => Base::Next,
        p => return json!({"error": format!("simulate: unknown policy {p}")}),
    };
    let members: Vec<usize> = cfg.loop_members.iter().filter_map(|&id| env.uix(id)).collect();
    if base != Base::Greedy && (members.len() < 3 || members.len() != cfg.loop_members.len() || members.iter().any(|m| !env.owned.contains(m))) {
        return json!({"error": "loop: loop_members must be at least three owned umas"});
    }
    let full = (1usize << ntar) - 1;
    let mut order: Vec<usize> = (0..full).collect();
    order.sort_by_key(|&m| (m.count_ones(), m));
    let n = d.umas.len();
    let one = vec![1.0f64; ntar];
    let hi: Vec<f64> = (0..n).map(|t| if env.owned.contains(&t) { env.ceiling(t, &one, 1.0).unwrap_or(0.0) } else { 0.0 }).collect();
    let star = env.sj.star_dist;
    let fastc: Vec<Vec<Vec<(f64, f64)>>> = (0..n).map(|t| match &env.dmax[t] { None => vec![], Some((dw, dg)) =>
        env.sts0[t].iter().zip(&env.ew0[t]).map(|(sv, ew)| (0..env.nu).map(|k| match sv[k] {
            2 => (GEN_GOLD, 0.0),
            1 => (dg[k] * GEN_GOLD + (1.0 - dg[k]) * crate::skills::GEN_WHITE, 0.0),
            _ => (dg[k] * GEN_GOLD + (1.0 - dg[k]) * crate::skills::GEN_WHITE, (1.0 - dg[k]) * crate::skills::GEN_WHITE * (1.0 - dw[k]) * (1.0 - ew[k])),
        }).collect()).collect() }).collect();
    let fixed = env.model.fixed_factor();
    let mut mi = vec![None; if base == Base::Next { n } else { 0 }];
    if base == Base::Next { for (k, &m) in members.iter().enumerate() { mi[m] = Some(k); } }
    let mut s = Sim { env, d, sk, cr, job, cfg: cfg.clone(), cache: Cache::new(n), trainees: trainees.to_vec(), hi, order, star, sched: Default::default(), n: Count::default(), members, fastc, fixed, main: true, audits: vec![], fill: false, mi, t_now: 0, run_now: 0, dcache: None };
    let t_init = std::time::Instant::now();
    let st0 = s.initial(legs, base);
    if cfg.policy == "index" { return s.index(&st0); }
    if cfg.policy == "index2" { return s.index2(&st0); }
    if cfg.policy == "trace" { return s.trace(&st0); }
    let init_s = t_init.elapsed().as_secs_f64();
    let mut runs = vec![]; let mut es = vec![];
    let eps = cfg.eps;
    for run in cfg.first..cfg.first + cfg.runs {
        if !cfg.share_cache && run > cfg.first { s.cache = Cache::new(n); }
        s.dcache = None;
        let tm = std::time::Instant::now();
        let n0 = s.n;
        let mut st = st0.clone();
        let (mut rec, mut checks) = (vec![], vec![]);
        let keys = Keys::Main { seed: cfg.seed, run: run as u32 };
        let (e, careers, tail, fp, pruned) = s.play(&mut st, base, keys, 1.0, 0, eps, Some(run as u32), &mut rec, &mut checks);
        let n1 = s.n;
        es.push(e);
        let (segs, decisions): (Vec<Value>, Vec<Value>) = rec.into_iter().partition(|r| r.get("decision").is_none());
        let finalk = segs.last().map(|x| x["pick"].clone()).unwrap_or(Value::Null);
        let wall = tm.elapsed().as_secs_f64();
        eprintln!("{} run {}: E {:.1}, {} careers, tail {:.2}, final p {:.3e}, pool {}, {:.1} s", cfg.policy, run, e, careers, tail, fp, st.legs.len(), wall);
        runs.push(json!({"run": run, "E": e, "careers": careers, "tail": tail, "final_p": fp, "final": finalk, "pool": st.legs.len(), "pruned": pruned,
            "wall_s": wall, "candidates": n1.cands - n0.cands, "ceiling_pass": n1.ceil_pass - n0.ceil_pass, "tables": n1.tables - n0.tables,
            "refreshes": n1.searches - n0.searches, "partial_searches": n1.partial - n0.partial, "segs": segs, "decisions": decisions, "checks": checks, "audits": std::mem::take(&mut s.audits)}));
    }
    json!({"simulate": sim, "policy": cfg.policy, "runs": runs, "E": stats(&es), "init_s": init_s, "decks": s.cache.n_decks(), "deck_tables": s.cache.tables,
           "refreshes": s.n.searches, "schedules": s.sched.len()})
}

#[cfg(test)]
mod tests {
    #[test]
    fn u01_matches_policy_sim() {
        // node: (await import('./val/policy_sim.mjs')).u01(...)
        assert_eq!(super::u01(&[1, 3, 0, 1, 0]), 0.006101473502044041);
        assert_eq!(super::u01(&[1, 0, 17, 2, 5]), 0.786756654747679);
        assert_eq!(super::u01(&[7, 123456, 99999, 3, 33]), 0.9235248915231418);
    }
}
