//! The planner's pick on the surrogate simulator (`plan` on a career job; docs/SOLVER.md "Solver-only career fields"; `next`: "Next career").
//!
//! The objective is the one the simulator measures: the expected number of careers until a career's veteran lands
//! every target. Per candidate career c (trainee, parents, her exact best deck), base policy B and sample j (derived,
//! as `sim`):
//!
//!   X_j(c, B) = 1 + sum over careers t >= 1 of S_t,   S_1 = 1 - p_c,   S_{t+1} = S_t (1 - p_t)
//!
//! career 0 is c itself (p_c its exact P(all); its failed veteran drawn from its exact landed-set distribution), every
//! later career is B's, stopping when S_t < eps or at t = horizon, where the tail S_t / p is added. Two bases, on the
//! same uniforms:
//!   - `greedy` (`Sim::play`): the best P(all) on the cached decks, re-chosen after every career; p in the tail is the
//!     last career's. Greedy only ever moves to a higher P(all), so the tail is at least what greedy would still need
//!     (derived): an upper estimate of "c, then greedy for ever".
//!   - `loop`: the simulator's loop policy over the rotation members (the loop planner's best loop for the job's
//!     settings, `loop_members`): member i trains with the latest rotation veterans of the previous two as parents; c's
//!     veteran enters the rotation when her character is a member (the next member trains next), else the rotation
//!     starts at the member whose best career now has the highest P(all). p in the tail is the best P(all) of the last
//!     rotation's careers (the player repeats that one; the last career's own p would make the tail depend on which
//!     member happens to train at the horizon). A member without both parent veterans gets the best pair from the pool
//!     with those she has fixed (the career search); when no legal pair exists the sample ends there with the tail.
//! The candidate's estimate is the lower of the two bases' means (a rollout over several base heuristics is no worse
//! than the best of them: Bertsekas); per sample the series of that base is used for the comparisons.
//!
//! Common random numbers: sample j uses `Keys::Main { seed, run: j }` for every candidate and both bases (the same
//! uniforms at the same career index), so differences are paired.
//!
//! Allocation (successive doubling): every candidate gets `samples_min` samples (sample index outer, candidate inner, so
//! a budget that runs out leaves the counts even); the leader L is the eligible candidate with the lowest estimate; a
//! candidate is separated from L when the paired difference X_j(c) - X_j(L) (each with its own winning base) over their
//! common samples has its 95% Student-t interval above 0. While some eligible candidate is not separated, L and those
//! candidates double their samples (to `samples_max`); stop when none is left ("separated"), all of them are at
//! `samples_max` ("samples_max"), or `budget_ms` of wall clock has passed ("budget"; checked before each sample). Pick:
//! among L and the candidates not separated from it, the one with the highest P(all) now ("about the same" when there
//! are several).
use super::*;

fn p_budget() -> f64 { 20000.0 }

/// Each trainee's best career (`Env::solve`, one trainee, top 1) and the decks that search checked, per plan: kept for
/// the next plan of the same setup and places whose pool is this one plus one veteran at the end (the page's Compare
/// and Reroll: your veterans and her). Every career of the larger pool is one of this pool's or has her as a parent, so
/// the best is the better of the kept one and the best with her as a parent (a search with her fixed): exact, the same
/// value; on an exact tie the kept career stands.
pub(in super::super) struct Bests { key: (u64, u64), best: Vec<Option<Option<Found>>>, refs: Vec<Vec<Vec<Ref>>> }
thread_local! { static BESTS: std::cell::RefCell<Vec<Bests>> = const { std::cell::RefCell::new(Vec::new()) }; }
/// the kept plans for the page to store (`state_out`): per plan its key and every uma's kept best (her decks' refs are
/// not kept: nothing reads them back)
pub(in super::super) fn bests_out() -> Value {
    BESTS.with(|c| Value::Array(c.borrow().iter().map(|b| json!({"k": [super::super::ux(b.key.0), super::super::ux(b.key.1)], "n": b.best.len(),
        "b": b.best.iter().enumerate().filter_map(|(u, x)| x.as_ref().map(|f| json!([u, f.as_ref().map(super::super::found_out)]))).collect::<Vec<_>>()})).collect()))
}
pub(in super::super) fn bests_in(v: &Value) -> Option<Vec<Bests>> {
    use super::super::{found_in, uxr};
    v.as_array()?.iter().map(|e| {
        let k = e["k"].as_array()?; let n = e["n"].as_u64()? as usize;
        let mut best: Vec<Option<Option<Found>>> = vec![None; n];
        for x in e["b"].as_array()? {
            let a = x.as_array()?; let u = a.first()?.as_u64()? as usize;
            if u >= n { return None; }
            best[u] = Some(if a.get(1)?.is_null() { None } else { Some(found_in(a.get(1)?)?) });
        }
        Some(Bests { key: (uxr(k.first()?)?, uxr(k.get(1)?)?), best, refs: vec![vec![]; n] })
    }).collect()
}
pub(in super::super) fn bests_set(b: Vec<Bests>) { BESTS.with(|c| *c.borrow_mut() = b); }
fn h64(s: &str) -> u64 { use std::hash::{Hash, Hasher}; let mut h = std::collections::hash_map::DefaultHasher::new(); s.hash(&mut h); h.finish() }
/// (setup and places, the first `n` legacies)
fn bests_key(job: &Value, fix: &[usize], n: usize) -> (u64, u64) {
    let ls = job.get("legacies").and_then(|l| l.as_array()).map(|a| Value::Array(a[..n.min(a.len())].to_vec()).to_string()).unwrap_or_default();
    (crate::deck::career::setup_fp(job) ^ h64(&format!("{fix:?}")).rotate_left(1), h64(&ls))
}
/// trainee u's best career in `pool` (and the refs to absorb), reusing a kept plan of the pool less its last veteran
fn best_of(env: &mut Env, pool: &Pool, fix: &[usize], u: usize, same: Option<&Bests>, prev: Option<&Bests>, nl: usize) -> (Option<Found>, Vec<Vec<Ref>>) {
    // a kept plan of this very pool (the same job planned again: a reload, a setting changed and back): her best, as kept
    if let Some(sm) = same { if let Some(Some(x)) = sm.best.get(u) { return (x.clone(), vec![]); } }
    if let Some(pr) = prev { if let Some(Some(old)) = pr.best.get(u) {
        let mut refs = pr.refs.get(u).cloned().unwrap_or_default();
        if fix.len() >= 2 { return (old.clone(), refs); }   // both parents fixed: she can't be one
        let mut fx = fix.to_vec(); fx.push(nl - 1);
        // no career with her can beat the kept one (each one's ceiling, as `solve` takes them, is below it): kept, exact
        if let Some(o) = old {
            let (her, tid) = (nl - 1, env.d.umas[u].id);
            let mut ub = 0.0f64;
            for y in 0..her {
                let (ca, cb) = (pool.me[y].chara, pool.me[her].chara);
                if ca == cb || ca == tid || cb == tid || (pool.borrowed[y] && pool.borrowed[her]) || !fix.iter().all(|&f| f == y) { continue; }
                let (q, _, c) = env.lineage(pool, u, y, her);
                ub = ub.max(env.ceiling(u, &q, c).unwrap_or(0.0));
            }
            if ub * (1.0 + 1e-9) < o.val { return (Some(o.clone()), refs); }
        }
        // only a career above the kept one matters: the search's bar starts there (exact; a tie keeps the kept one)
        env.floor = old.as_ref().map_or(f64::NEG_INFINITY, |o| o.val); let sr = env.solve(pool, &fx, &[u], false, 1); env.floor = f64::NEG_INFINITY;
        if refs.len() < sr.refs.len() { refs.resize(sr.refs.len(), vec![]); }
        for (t, r) in sr.refs.into_iter().enumerate() { refs[t].extend(r); }
        let new = sr.found.into_iter().next();
        let best = match (old, new) { (Some(o), Some(n)) => Some(if n.val > o.val { n } else { o.clone() }), (o, n) => o.clone().or(n) };
        return (best, refs);
    } }
    let sr = env.solve(pool, fix, &[u], false, 1);
    (sr.found.into_iter().next(), sr.refs)
}
fn p_smin() -> usize { 4 }
fn p_smax() -> usize { 16 }
fn p_eps() -> f64 { 1e-3 }
fn p_horizon() -> usize { 200 }
fn p_seed() -> u32 { 1 }
fn p_top() -> usize { 6 }
fn p_recent() -> usize { 3 }
fn p_trainees() -> usize { 12 }
fn p_margin() -> f64 { 1e9 }
fn p_swork() -> usize { 5_000_000 }
fn p_rwork() -> usize { 0 }

#[derive(Deserialize, Clone)]
pub(in super::super) struct PlanJob {
    /// wall clock for the sampling (ms; the candidate searches and the initial state come first and are not counted)
    #[serde(default = "p_budget")] budget_ms: f64,
    #[serde(default = "p_smin")] samples_min: usize,
    #[serde(default = "p_smax")] samples_max: usize,
    #[serde(default = "p_eps")] eps: f64,
    #[serde(default = "p_horizon")] horizon_max: usize,
    #[serde(default = "p_seed")] seed: u32,
    /// candidates: the `top` best careers by P(all), the best career with each of the `recent` newest own veterans, the
    /// best career of each of the `trainees` best trainees, the best career of each rotation member
    #[serde(default = "p_top")] top: usize,
    #[serde(default = "p_recent")] recent: usize,
    #[serde(default = "p_trainees")] trainees: usize,
    /// the surrogate's refresh margin (`sim` module notes)
    #[serde(default = "p_margin")] margin: f64,
    /// work budget (career-search units) of each candidate search beyond the job's own (also each loop bootstrap
    /// search), and of each deck search a continuation makes (a refresh); 0 = the job's budget
    #[serde(default = "p_swork")] search_work: usize,
    #[serde(default = "p_rwork")] refresh_work: usize,
    /// the rotation (chara ids, in order); absent: the loop planner's best loop for the job's settings; empty: no loop base
    #[serde(default)] loop_members: Option<Vec<u32>>,
    /// `index_rule`: only the frozen value V0 (no index per candidate)
    #[serde(default)] value_only: bool,
    /// `index_rule`: each uma's best career through the surrogate (`best_sur`) instead of the career search (1), or
    /// both, reporting where they differ (2); the surrogate's refresh margin there
    #[serde(default)] sur: u8,
    #[serde(default)] sur_margin: f64,
    /// `index_rule` measurement: each builder's career also from her deck class's winning decks alone (no career search),
    /// its lifts and score beside the exact ones (`proxy_check` in the output); changes nothing else
    #[serde(default)] proxy_check: bool,
    /// `index_rule`: every uma's best career searched exactly (the rule before builders went through their deck class)
    #[serde(default)] exact_all: bool,
    /// `index_rule`: the stage candidates (chara ids, in order) and their continuations (lam, phi, pinf) taken as given,
    /// not fitted to this pool: a comparison prices a pool with one more veteran under the fit of the plan it compares
    /// with (the plan's `fit`), so the two values differ only by what she changes, not by a refit
    #[serde(default)] fit: Option<Vec<(u32, f64, f64, f64)>>,
    /// `index_rule` test knob: PROXY_EFF in place of 0.06 (-1 leaves pass 1's builders to their proxies, so the pass-2
    /// searches refit Cont: tests/plan_memo.rs runs that path)
    #[serde(default)] proxy_eff: Option<f64>,
}

/// wall clock in ms: the host's `env.clock_ms` in wasm (performance.now()), else the process clock
#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" { fn clock_ms() -> f64; }
    unsafe { clock_ms() }
}
#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> f64 {
    static T0: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    T0.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
}

fn mean(x: &[f64]) -> f64 { x.iter().sum::<f64>() / x.len() as f64 }
/// mean and 95% Student-t half-width (None below two samples)
fn ci(x: &[f64]) -> (f64, Option<f64>) {
    let n = x.len();
    if n == 0 { return (f64::NAN, None); }
    let m = mean(x);
    if n < 2 { return (m, None); }
    let sd = (x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (n as f64 - 1.0)).sqrt();
    (m, Some(tq975(n as f64 - 1.0) * sd / (n as f64).sqrt()))
}
/// the paired difference X(i) - X(l) over their common samples: (mean, half-width), None below two
fn paired(xi: &[f64], xl: &[f64]) -> Option<(f64, f64)> {
    let n = xi.len().min(xl.len());
    if n < 2 { return None; }
    let d: Vec<f64> = (0..n).map(|j| xi[j] - xl[j]).collect();
    let (m, h) = ci(&d);
    Some((m, h.unwrap()))
}
/// i is separated from (significantly worse than) the leader
fn separated(xi: &[f64], xl: &[f64]) -> bool { paired(xi, xl).map_or(false, |(m, h)| m - h > 0.0) }

/// The loop planner's best loop for the job's settings (a deck + loop search, top 1, per-uma decks): its members as
/// {id, name, short}, in rotation order. Cached per setup (the job without its pool and places).
fn best_loop(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, job: &Value) -> Result<Vec<Value>, String> {
    thread_local! { static LOOPS: std::cell::RefCell<Vec<(u64, Result<Vec<Value>, String>)>> = const { std::cell::RefCell::new(Vec::new()) }; }
    let mut j = job.clone();
    let Some(o) = j.as_object_mut() else { return Err("bad job".into()) };
    for k in ["legacies", "trainees", "fix", "per_trainee", "lookahead", "p_now", "p_minus1_now", "policy", "rollout", "plan", "simulate", "top", "next", "compare"] { o.remove(k); }
    o.insert("mode".into(), json!("deck")); o.insert("cmd".into(), json!("deck"));
    o.insert("top".into(), json!(1)); o.insert("top_decks".into(), json!(1)); o.insert("per_uma".into(), json!(true));
    let fp = setup_fp(&j);
    if let Some(x) = LOOPS.with(|c| c.borrow().iter().find(|e| e.0 == fp).map(|e| e.1.clone())) { return x; }
    let r = crate::deck::run_deck(d, sk, cr, &j);
    let x = match r["top"][0]["loop"]["members"].as_array() {
        Some(m) if !m.is_empty() => Ok(m.clone()),
        _ => Err(r["error"].as_str().unwrap_or("the loop search found no loop").to_string()),
    };
    LOOPS.with(|c| { let mut c = c.borrow_mut(); c.push((fp, x.clone())); if c.len() > 4 { let _ = c.remove(0); } });
    x
}

/// What the sampling found: per candidate its samples under each base (greedy, loop) and their tail shares; why it
/// stopped; time; careers simulated
/// and, per sample, the exact P(all) of the best career (greedy's) right after the option's first career (`nx`) against
/// today's best (`p0`)
struct Est { g: Vec<Vec<f64>>, l: Vec<Vec<f64>>, tg: Vec<Vec<f64>>, tl: Vec<Vec<f64>>, nx: Vec<Vec<f64>>, p0: f64, stop: &'static str, ms: f64, careers: usize, leader: Option<usize> }
impl Est {
    /// whether the loop base wins for candidate i (its mean lower, on as many samples)
    fn loop_wins(&self, i: usize) -> bool { !self.l[i].is_empty() && self.l[i].len() == self.g[i].len() && mean(&self.l[i]) < mean(&self.g[i]) }
    /// candidate i's series: its winning base's samples
    fn x(&self, i: usize) -> &[f64] { if self.loop_wins(i) { &self.l[i] } else { &self.g[i] } }
}

impl<'s, 'a> Sim<'s, 'a> {
    /// one greedy sample of candidate c from state st0 (module notes): (X, tail share, the exact P(all) of the best career
    /// after c's veteran joins: greedy's next career, the surrogate's choice with its cached deck)
    fn plan_sample(&mut self, st0: &St, c: &Cur, j: usize, seed: u32, eps: f64, careers: &mut usize) -> (f64, f64, f64) {
        let keys = Keys::Main { seed, run: j as u32 };
        let mut st = st0.clone();
        let (vet, _) = self.draw(c, &st.legs, &keys, 0);
        let mut buf = vec![];
        if !(self.cfg.prune && self.dominated(&st, &vet)) { self.add_vet(&mut st, vet, true, &mut buf); }
        let pn = self.greedy_cur(&mut st).p;
        let (e, t, tail, ..) = self.play(&mut st, Base::Greedy, keys, 1.0 - c.p, 1, eps, None, &mut vec![], &mut vec![]);
        *careers += t;
        (1.0 + e, tail / (1.0 + e), pn)
    }
    /// the loop's next career as `loop_cur` makes it, None when a member has no legal pair (bootstrap)
    fn plan_loop_cur(&mut self, st: &mut St) -> Option<Rc<Cur>> {
        let nm = self.members.len();
        let pos = st.rot.pos % nm;
        let t = self.members[pos];
        let prev: Vec<usize> = [1, 2].iter().filter_map(|&k| st.rot.latest[(pos + nm - k) % nm]).collect();
        if prev.len() == 2 { return Some(self.loop_cur(st)); }
        self.env.use_seen = true;
        let sv = self.env.solve(&st.pool, &prev, &[t], false, 1);
        let f = sv.found.first()?;
        self.cache.absorb(self.env, &sv.refs);
        let c = Rc::new(self.make_cur(&st.pool, f.t, f.a, f.b, &f.sel.clone(), true));
        st.cur = Some(c.clone());
        Some(c)
    }
    /// one loop sample of candidate c (module notes): (X, tail share); `start` the member that trains first when c's
    /// trainee is no member
    fn plan_sample_loop(&mut self, st0: &St, c: &Cur, j: usize, seed: u32, eps: f64, start: usize, careers: &mut usize) -> (f64, f64) {
        let keys = Keys::Main { seed, run: j as u32 };
        let mut st = st0.clone();
        let (vet, _) = self.draw(c, &st.legs, &keys, 0);
        let mut buf = vec![];
        let nm = self.members.len();
        self.add_vet(&mut st, vet, false, &mut buf);
        let vi = st.legs.len() - 1;
        st.rot = Rot { pos: start, latest: vec![None; nm] };
        if let Some(m) = self.members.iter().position(|&u| u == c.t) { st.rot.latest[m] = Some(vi); st.rot.pos = m + 1; }
        st.cur = None;
        let (mut s, mut t, mut e) = (1.0 - c.p, 1usize, 0.0f64);
        let mut ps: Vec<f64> = vec![c.p];
        loop {
            let cur = self.plan_loop_cur(&mut st);
            let done = cur.is_none() || s < eps || t >= self.cfg.max_careers;
            if let Some(cu) = &cur { if done { ps.push(cu.p); } }
            if done {
                let pb = ps.iter().rev().take(nm).cloned().fold(0.0f64, f64::max);
                let tail = if pb > 0.0 { s / pb } else { f64::INFINITY };
                *careers += t;
                let x = 1.0 + e + tail;
                return (x, tail / x);
            }
            let cur = cur.unwrap();
            e += s; s *= 1.0 - cur.p; ps.push(cur.p);
            let (vet, _) = self.draw(&cur, &st.legs, &keys, t);
            t += 1;
            self.add_vet(&mut st, vet, false, &mut buf);
            if cur.rot { self.rot_update(&mut st); }
            st.cur = None;
        }
    }
    /// the allocation (module notes) over `cs`; only `eligible` ones lead or count as contenders; `starts[i]`: where the
    /// rotation starts for candidate i (None: no loop base)
    fn plan_estimate(&mut self, st0: &St, cs: &[Cur], eligible: &[bool], starts: &[Option<usize>], pj: &PlanJob) -> Est {
        let t0 = now_ms();
        let n = cs.len();
        let smax = pj.samples_max.max(1);
        let p0 = st0.cur.as_ref().map_or(0.0, |c| c.p);
        let mut est = Est { g: vec![vec![]; n], l: vec![vec![]; n], tg: vec![vec![]; n], tl: vec![vec![]; n], nx: vec![vec![]; n], p0, stop: "", ms: 0.0, careers: 0, leader: None };
        let mut want = vec![pj.samples_min.clamp(1, smax); n];
        let leader_of = |est: &Est| (0..n).filter(|&i| eligible[i] && !est.g[i].is_empty()).min_by(|&x, &y| mean(est.x(x)).partial_cmp(&mean(est.x(y))).unwrap());
        'rounds: loop {
            let top = *want.iter().max().unwrap();
            for j in 0..top {
                for i in 0..n {
                    if est.g[i].len() != j || j >= want[i] { continue; }
                    if now_ms() - t0 > pj.budget_ms { est.stop = "budget"; break 'rounds; }
                    let (x, sh, pn) = self.plan_sample(st0, &cs[i], j, pj.seed, pj.eps, &mut est.careers);
                    est.nx[i].push(pn);
                    if let Some(sp) = starts[i] {
                        let (xl, shl) = self.plan_sample_loop(st0, &cs[i], j, pj.seed, pj.eps, sp, &mut est.careers);
                        est.l[i].push(xl); est.tl[i].push(shl);
                    }
                    est.g[i].push(x); est.tg[i].push(sh);
                }
            }
            let Some(l) = leader_of(&est) else { est.stop = "no candidate"; break };
            let open: Vec<usize> = (0..n).filter(|&i| i != l && eligible[i] && !separated(est.x(i), est.x(l))).collect();
            if open.is_empty() { est.stop = "separated"; break; }
            let target = (2 * est.g[l].len()).min(smax);
            let mut grew = false;
            for &i in open.iter().chain([&l]) { if est.g[i].len() < target { want[i] = want[i].max(target); grew = true; } }
            if !grew { est.stop = "samples_max"; break; }
        }
        est.leader = leader_of(&est);
        est.ms = now_ms() - t0;
        est
    }
}

/// a career as a candidate: its exact numbers (`make_cur`) and the search result it came from
struct Cand { f: Found, cur: Cur, eligible: bool, greedy: bool, from: Vec<&'static str> }

fn sim_for<'s, 'a>(env: &'s mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, pj: &PlanJob) -> Result<Sim<'s, 'a>, Value> {
    let ntar = env.ntar;
    if ntar > 12 || env.nu > 10 { return Err(json!({"error": "The plan takes at most 12 targets, 10 of them not guaranteed by the scenario."})); }
    let cfg: SimJob = serde_json::from_value(json!({"policy": "greedy", "eps": pj.eps, "max_careers": pj.horizon_max, "margin": pj.margin})).unwrap();
    // as `simulate` builds it (the continuations are the player's later careers: every owned trainee)
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
    let trainees = env.owned.clone();
    Ok(Sim { env, d, sk, cr, job, cfg, cache: Cache::new(n), trainees, hi, order, star, sched: Default::default(), n: Count::default(), members: vec![],
             fastc, fixed, main: false, audits: vec![], fill: true, mi: vec![], t_now: 0, run_now: 0, dcache: None })
}

/// The rotation for the loop base: (members as uma indices, their {id, name, short}, or why there is none). Members
/// must be owned umas with a legal deck, at least three of them.
fn rotation(env: &Env, d: &Data, sk: &SkillsRaw, cr: &CardsRaw, job: &Value, pj: &PlanJob) -> (Vec<usize>, Vec<Value>, Option<String>) {
    let info: Result<Vec<Value>, String> = match &pj.loop_members {
        Some(ids) => Ok(ids.iter().map(|&id| { let u = d.umas.iter().find(|u| u.id == id); json!({"id": id, "name": u.map_or(format!("Uma {id}"), |u| u.name.clone())}) }).collect()),
        None => best_loop(d, sk, cr, job),
    };
    let info = match info { Ok(x) => x, Err(e) => return (vec![], vec![], Some(e)) };
    if info.is_empty() { return (vec![], vec![], Some("no loop base (loop_members empty)".into())); }
    let idx: Vec<Option<usize>> = info.iter().map(|m| m["id"].as_u64().and_then(|id| env.uix(id as u32))).collect();
    if idx.len() < 3 || idx.iter().any(|x| x.map_or(true, |t| !env.owned.contains(&t) || env.dmax[t].is_none())) {
        return (vec![], info, Some("the loop's members must be at least three owned umas with a legal deck".into()));
    }
    (idx.into_iter().map(|x| x.unwrap()).collect(), info, None)
}

impl<'s, 'a> Sim<'s, 'a> {
    /// `initial` without its own search: the cache starts from `refs` (searches already made), then every candidate of
    /// the pool is valued best ceiling first
    fn plan_initial(&mut self, legs: &[Legacy], refs: &[&[Vec<Ref>]]) -> St {
        let mut st = St { legs: vec![], pool: Pool { me: vec![], gp: vec![], borrowed: vec![], gpc: vec![] }, gpc: vec![], fam: Default::default(), best: None, cur: None,
                          rot: Rot { pos: 0, latest: vec![None; self.members.len()] }, mb: vec![], since: vec![] };
        let mut buf = vec![];
        for l in legs { self.add_vet(&mut st, l.clone(), false, &mut buf); }
        for r in refs { self.cache.absorb(self.env, r); }
        let trainees = self.trainees.clone();
        let mut cs = vec![];
        let n = st.legs.len();
        for &t in &trainees { for a in 0..n { for b in a + 1..n { if self.legal(&st, t, a, b) {
            let (q, _, c) = self.env.lineage(&st.pool, t, a, b);
            if let Some(ub) = self.env.ceiling(t, &q, c) { cs.push((ub, t, a, b)); }
        } } } }
        cs.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        for (ub, t, a, b) in cs {
            if ub * (1.0 + 1e-9) < st.best.map_or(0.0, |p| p.val) { break; }
            self.consider(&mut st, t, a, b, &mut buf);
        }
        if st.best.is_some() { self.greedy_cur(&mut st); }
        st
    }
    fn cand(&mut self, pool: &Pool, f: Found, eligible: bool, greedy: bool, from: &'static str) -> Cand {
        let cur = self.make_cur(pool, f.t, f.a, f.b, &f.sel, false);
        Cand { f, cur, eligible, greedy, from: vec![from] }
    }
    fn add_cand(&mut self, cs: &mut Vec<Cand>, pool: &Pool, f: Found, eligible: bool, from: &'static str) {
        let k = (f.t, f.a.min(f.b), f.a.max(f.b));
        if let Some(c) = cs.iter_mut().find(|c| (c.f.t, c.f.a.min(c.f.b), c.f.a.max(c.f.b)) == k) { if !c.from.contains(&from) { c.from.push(from); } return; }
        let c = self.cand(pool, f, eligible, false, from);
        cs.push(c);
    }
    /// the greedy pick of state st (today's best by the surrogate) as a candidate, with the deck the surrogate runs
    fn greedy_cand(&mut self, st: &mut St, eligible: bool) -> Option<Cand> {
        let pk = st.best?;
        let cur = self.greedy_cur(st);
        let sel = self.cache.decks[pk.t][pk.d].sel.clone();
        let (q, _, c) = self.env.lineage(&st.pool, pk.t, pk.a, pk.b);
        let ub = self.env.ceiling(pk.t, &q, c).unwrap_or(0.0);
        let f = Found { t: pk.t, a: pk.a, b: pk.b, val: cur.p, ub, sel };
        Some(Cand { f, cur: Cur { t: cur.t, a: cur.a, b: cur.b, p: cur.p, os: cur.os.clone(), wp: cur.wp.clone(), rot: false }, eligible, greedy: true, from: vec!["greedy"] })
    }
}

/// candidate i's numbers: the estimate (the winning base's mean), its interval, both bases, tail shares
fn est_json(est: &Est, i: usize) -> serde_json::Map<String, Value> {
    let mut o = serde_json::Map::new();
    let x = est.x(i);
    let (m, h) = ci(x);
    let lw = est.loop_wins(i);
    let num = |v: &[f64]| { let (m, h) = ci(v); (if v.is_empty() || !m.is_finite() { Value::Null } else { json!(m) }, json!(h.filter(|h| h.is_finite()))) };
    let (gm, gh) = num(&est.g[i]); let (lm, lh) = num(&est.l[i]);
    let share = |v: &[f64]| if v.is_empty() { Value::Null } else { json!(mean(v)) };
    o.insert("expected".into(), if x.is_empty() || !m.is_finite() { Value::Null } else { json!(m) });
    o.insert("ci".into(), json!(h.filter(|h| h.is_finite())));
    o.insert("samples".into(), json!(x.len()));
    o.insert("base".into(), json!(if lw { "loop" } else { "greedy" }));
    o.insert("expected_greedy".into(), gm); o.insert("ci_greedy".into(), gh);
    o.insert("expected_loop".into(), lm); o.insert("ci_loop".into(), lh);
    o.insert("tail_share".into(), share(if lw { &est.tl[i] } else { &est.tg[i] }));
    o.insert("tail_share_greedy".into(), share(&est.tg[i])); o.insert("tail_share_loop".into(), share(&est.tl[i]));
    // does running it improve the next run: the share of samples whose best career right after beats today's best, and
    // that career's mean P(all) over those samples
    let up: Vec<f64> = est.nx[i].iter().cloned().filter(|&p| p > est.p0 * (1.0 + 1e-9)).collect();
    o.insert("improve_next".into(), if est.nx[i].is_empty() { Value::Null } else { json!(up.len() as f64 / est.nx[i].len() as f64) });
    o.insert("next_if_improved".into(), if up.is_empty() { Value::Null } else { json!(mean(&up)) });
    o
}

fn row(env: &mut Env, pool: &Pool, c: &Cand, est: &Est, i: usize, lead: Option<usize>) -> Value {
    let mut r = env.report(pool, &c.f, false);
    let o = r.as_object_mut().unwrap();
    for (k, v) in est_json(est, i) { o.insert(k, v); }
    o.insert("eligible".into(), json!(c.eligible));
    o.insert("greedy".into(), json!(c.greedy));
    o.insert("from".into(), json!(c.from));
    o.insert("vs_leader".into(), match lead.and_then(|l| paired(est.x(i), est.x(l))) { Some((m, h)) => json!([m, h]), None => Value::Null });
    r
}

/// The plan for a career job (module notes): candidates, the estimate per candidate, the pick. `sv` is the job's main
/// search (its refs seed the cache), `fix`/`trainees` the job's places.
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn plan<'a>(env: &mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, legs: &[Legacy], pool: &Pool,
                                 sv: &Solved, reuse_top: bool, fix: &[usize], trainees: &[usize], pj: &PlanJob) -> Value {
    let t_all = now_ms();
    let t1 = now_ms();
    let (members, minfo, mwhy) = rotation(env, d, sk, cr, job, pj);
    let loop_ms = now_ms() - t1;
    let mut s = match sim_for(env, d, sk, cr, job, pj) { Ok(s) => s, Err(e) => return e };
    s.members = members.clone();
    // the candidates' searches (fixed places respected), their fully checked decks into the cache
    s.env.use_seen = true;
    let mut tm = vec![];
    let t1 = now_ms();
    let budget0 = s.env.budget;
    if pj.search_work > 0 { s.env.budget = pj.search_work; s.env.hard = pj.search_work; }
    let top = if reuse_top && sv.found.len() >= pj.top.max(1) { Solved { found: sv.found.iter().take(pj.top.max(1)).map(|f| Found { t: f.t, a: f.a, b: f.b, val: f.val, ub: f.ub, sel: f.sel.clone() }).collect(), refs: vec![vec![]; d.umas.len()],
        work: 0, cands: 0, solved: 0, bounded: 0, scored: 0, skipped: 0, next_bound: 0.0, complete: sv.complete, proven: sv.proven, cert: sv.cert } }
        else { s.env.solve(pool, fix, trainees, false, pj.top.max(1)) };
    tm.push(now_ms() - t1); let t1 = now_ms();
    let per = if pj.trainees > 0 { Some(s.env.solve(pool, fix, trainees, true, pj.trainees)) } else { None };
    tm.push(now_ms() - t1); let t1 = now_ms();
    let mut rec: Vec<Solved> = vec![];
    if fix.len() < 2 {
        for v in (0..legs.len()).rev().filter(|&v| !legs[v].borrowed && !fix.contains(&v)).take(pj.recent) {
            let mut f2 = fix.to_vec(); f2.push(v);
            rec.push(s.env.solve(pool, &f2, trainees, false, 1));
        }
    }
    tm.push(now_ms() - t1); let t1 = now_ms();
    // each rotation member's best career, whatever its odds now
    let mut mem: Vec<Solved> = vec![];
    for &m in &members { if trainees.contains(&m) { mem.push(s.env.solve(pool, fix, &[m], false, 1)); } }
    tm.push(now_ms() - t1);
    // the sampling: deck searches with the refresh budget, loop bootstraps under the hard cap
    s.env.budget = if pj.refresh_work > 0 { pj.refresh_work } else { budget0 };
    let mut refs: Vec<&[Vec<Ref>]> = vec![&sv.refs, &top.refs];
    if let Some(p) = &per { refs.push(&p.refs); }
    for r in rec.iter().chain(&mem) { refs.push(&r.refs); }
    let t_init = now_ms();
    let mut st0 = s.plan_initial(legs, &refs);
    let init_ms = now_ms() - t_init;
    let mut cs: Vec<Cand> = vec![];
    let top_key = top.found.first().map(|f| (f.t, f.a.min(f.b), f.a.max(f.b)));
    let g_ok = st0.best.map_or(false, |p| trainees.contains(&p.t) && fix.iter().all(|&x| x == p.a || x == p.b));
    if let Some(g) = s.greedy_cand(&mut st0, g_ok) { cs.push(g); }
    for f in top.found { s.add_cand(&mut cs, pool, f, true, "top"); }
    for r in rec { for f in r.found { s.add_cand(&mut cs, pool, f, true, "recent"); } }
    if let Some(p) = per { for f in p.found { s.add_cand(&mut cs, pool, f, true, "trainee"); } }
    for r in mem { for f in r.found { s.add_cand(&mut cs, pool, f, true, "member"); } }
    // a searched candidate equal to the greedy pick replaces it (the search's deck is the exact best there)
    if cs.len() > 1 && cs[0].greedy {
        let k = (cs[0].f.t, cs[0].f.a.min(cs[0].f.b), cs[0].f.a.max(cs[0].f.b));
        if let Some(i) = (1..cs.len()).find(|&i| (cs[i].f.t, cs[i].f.a.min(cs[i].f.b), cs[i].f.a.max(cs[i].f.b)) == k) {
            let mut c = cs.remove(i); c.greedy = true; c.from.insert(0, "greedy"); cs[0] = c;
        }
    }
    if cs.iter().all(|c| !c.eligible) { return json!({"error": "No legal career to plan."}); }
    // where the rotation starts for an option whose trainee is no member: the member whose best career is the best now
    let start = members.iter().enumerate().max_by(|x, y| {
        let p = |t: usize| cs.iter().filter(|c| c.f.t == t).map(|c| c.cur.p).fold(0.0f64, f64::max);
        p(*x.1).partial_cmp(&p(*y.1)).unwrap().then(y.0.cmp(&x.0)) }).map(|x| x.0);
    let starts: Vec<Option<usize>> = cs.iter().map(|_| start).collect();
    let curs: Vec<Cur> = cs.iter().map(|c| Cur { t: c.cur.t, a: c.cur.a, b: c.cur.b, p: c.cur.p, os: c.cur.os.clone(), wp: c.cur.wp.clone(), rot: false }).collect();
    let elig: Vec<bool> = cs.iter().map(|c| c.eligible).collect();
    if pj.search_work > 0 { s.env.hard = pj.search_work; }
    let est = s.plan_estimate(&st0, &curs, &elig, &starts, pj);
    s.env.hard = 0;
    // the pick: among the leader and the eligible candidates not separated from it, the highest P(all) now
    let lead = est.leader;
    let same: Vec<usize> = match lead { None => vec![], Some(l) => (0..cs.len()).filter(|&i| elig[i] && !est.g[i].is_empty() && (i == l || !separated(est.x(i), est.x(l)))).collect() };
    // ties: when the leader's estimate comes from the loop future, the loop's own next career (the base policy's action:
    // rollout with no candidate clearly better follows its base); the rotation continues after the newest veteran when
    // she is a member (Pasa -> Agnes -> Zephyr -> Pasa ...), else it starts where the loop future starts; otherwise, or
    // when that member has no candidate among the tied ones, the highest P(all) now
    let loop_next: Option<usize> = if members.is_empty() { None } else {
        let newest = legs.last().and_then(|l| s.env.uix(l.chara)).and_then(|u| members.iter().position(|&m| m == u));
        match newest { Some(k) => Some(members[(k + 1) % members.len()]), None => start.map(|k| members[k]) } };
    let by_p = |x: &usize, y: &usize| cs[*x].cur.p.partial_cmp(&cs[*y].cur.p).unwrap().then(y.cmp(x));
    let loop_led = lead.map_or(false, |l| est.loop_wins(l)) && same.len() > 1;
    // among the tied careers of loop members other than the newest veteran's (in a three-member rotation both others are
    // "next"; the listed order is arbitrary), the lowest estimate; the listed next member only when that leaves none
    let newest_m = legs.last().and_then(|l| s.env.uix(l.chara)).filter(|u| members.contains(u));
    let by_est = |x: &usize, y: &usize| mean(est.x(*x)).partial_cmp(&mean(est.x(*y))).unwrap_or(std::cmp::Ordering::Equal);
    let loop_pick = if !loop_led { None } else {
        same.iter().copied().filter(|&i| members.contains(&cs[i].cur.t) && Some(cs[i].cur.t) != newest_m).min_by(|x, y| by_est(x, y))
            .or_else(|| loop_next.and_then(|m| same.iter().copied().filter(|&i| cs[i].cur.t == m).max_by(|x, y| by_p(x, y)))) };
    let pick_rule = if same.len() <= 1 { "lowest" } else if loop_pick.is_some() { "loop" } else { "odds" };
    let pick = loop_pick.or_else(|| same.iter().copied().max_by(|x, y| by_p(x, y)))
        .or_else(|| (0..cs.len()).filter(|&i| elig[i]).max_by(|&x, &y| cs[x].cur.p.partial_cmp(&cs[y].cur.p).unwrap()));
    // fidelity: the chosen career's table value (the surrogate) against its exact value (the search's best deck)
    let mut buf = vec![];
    let fid = pick.map(|i| {
        let f = &cs[i].f;
        let (q, _, c) = s.env.lineage(pool, f.t, f.a, f.b);
        let tv = s.tval(f.t, &q, c, &mut buf).0;
        json!({"table": tv, "exact": cs[i].cur.p, "rel_gap": (cs[i].cur.p - tv) / cs[i].cur.p.max(1e-300)})
    });
    // the surrogate's pick against the exact search's best (when nothing is fixed, the top search is over everything)
    let greedy_vs_exact = if fix.is_empty() && trainees.len() == s.env.owned.len() { cs.iter().find(|c| c.greedy).map(|g| json!({
        "surrogate": st0.best.map(|p| p.val), "its_exact": g.cur.p, "agrees": top_key == Some((g.f.t, g.f.a.min(g.f.b), g.f.a.max(g.f.b)))})) } else { None };
    s.env.budget = budget0;
    let sim_n = s.n;
    let ndecks = s.cache.n_decks();
    drop(s);
    // rows, best estimate first (unsampled last)
    let mut ord: Vec<usize> = (0..cs.len()).collect();
    let key = |i: usize| if est.g[i].is_empty() { f64::INFINITY } else { mean(est.x(i)) };
    ord.sort_by(|&x, &y| key(x).partial_cmp(&key(y)).unwrap_or(std::cmp::Ordering::Equal).then(est.g[x].is_empty().cmp(&est.g[y].is_empty())));
    let opts: Vec<Value> = ord.iter().map(|&i| {
        let mut r = row(env, pool, &cs[i], &est, i, lead);
        let o = r.as_object_mut().unwrap();
        o.insert("chosen".into(), json!(pick == Some(i)));
        o.insert("about_same".into(), json!(same.len() > 1 && same.contains(&i)));
        o.insert("leader".into(), json!(lead == Some(i)));
        o.insert("loop_start".into(), json!(if members.contains(&cs[i].cur.t) { None } else { start }));
        r
    }).collect();
    let pos = |i: Option<usize>| i.and_then(|i| ord.iter().position(|&x| x == i));
    let gi = cs.iter().position(|c| c.greedy);
    let greedy_expected = gi.map(|i| { let mut o = est_json(&est, i); o.insert("option".into(), json!(pos(Some(i)))); o.insert("p_all".into(), json!(cs[i].cur.p)); Value::Object(o) });
    let choice = pick.map(|i| env.report(pool, &cs[i].f, true));
    json!({"options": opts, "pick": pos(pick), "leader": pos(lead), "about_same": same.len() > 1, "pick_rule": pick_rule,
           "loop_next": loop_next.map(|m| json!({"id": env.d.umas[m].id, "name": env.d.umas[m].name})), "choice": choice, "greedy_expected": greedy_expected,
           "loop_members": minfo, "loop_base": mwhy.is_none(), "loop_note": mwhy, "loop_ms": loop_ms,
           "stop": est.stop, "sampling_ms": est.ms, "init_ms": init_ms, "search_ms": tm, "total_ms": now_ms() - t_all, "continuations": est.g.iter().chain(&est.l).map(|x| x.len()).sum::<usize>(),
           "careers_simulated": est.careers, "deck_searches": sim_n.searches, "decks": ndecks, "fidelity": fid, "greedy_check": greedy_vs_exact,
           "settings": {"budget_ms": pj.budget_ms, "samples_min": pj.samples_min, "samples_max": pj.samples_max, "eps": pj.eps, "horizon_max": pj.horizon_max,
                        "seed": pj.seed, "top": pj.top, "recent": pj.recent, "trainees": pj.trainees, "margin": pj.margin, "search_work": pj.search_work, "refresh_work": pj.refresh_work}})
}

/// The veteran analysis on the same estimator: (a) the stay career again, (b) `advance` (the best career with the new
/// veteran), the same samples for both. The pool already holds her either way.
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn plan_policy<'a>(env: &mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, legs: &[Legacy], pool: &Pool,
                                        sv: &Solved, stay: Found, stay_refs: &[Vec<Ref>], pj: &PlanJob) -> Value {
    let t_all = now_ms();
    let Some(adv) = sv.found.first() else { return json!({"error": "No next career with her."}) };
    let adv = Found { t: adv.t, a: adv.a, b: adv.b, val: adv.val, ub: adv.ub, sel: adv.sel.clone() };
    let (members, minfo, mwhy) = rotation(env, d, sk, cr, job, pj);
    let mut s = match sim_for(env, d, sk, cr, job, pj) { Ok(s) => s, Err(e) => return e };
    s.members = members.clone();
    let budget0 = s.env.budget;
    if pj.refresh_work > 0 { s.env.budget = pj.refresh_work; }
    let st0 = s.plan_initial(legs, &[&sv.refs, stay_refs]);
    let cs = vec![s.cand(pool, stay, true, false, "stay"), s.cand(pool, adv, true, false, "advance")];
    let curs: Vec<Cur> = cs.iter().map(|c| Cur { t: c.cur.t, a: c.cur.a, b: c.cur.b, p: c.cur.p, os: c.cur.os.clone(), wp: c.cur.wp.clone(), rot: false }).collect();
    // the rotation starts at the member with the best of these two careers (0 when neither trains a member)
    let start = if members.is_empty() { None } else { Some(cs.iter().filter_map(|c| members.iter().position(|&m| m == c.cur.t).map(|i| (c.cur.p, i)))
        .max_by(|x, y| x.0.partial_cmp(&y.0).unwrap()).map_or(0, |x| x.1)) };
    if pj.search_work > 0 { s.env.hard = pj.search_work; }
    let est = s.plan_estimate(&st0, &curs, &[true, true], &[start, start], pj);
    s.env.hard = 0;
    s.env.budget = budget0;
    drop(s);
    let one = |i: usize| { let mut o = est_json(&est, i); o.insert("p_all".into(), json!(cs[i].cur.p)); Value::Object(o) };
    let diff = paired(est.x(1), est.x(0));
    let about_same = diff.map_or(true, |(m, h)| m.abs() <= h);
    let better = if about_same { Value::Null } else if diff.unwrap().0 < 0.0 { json!("advance") } else { json!("stay") };
    json!({"stay": one(0), "advance": one(1), "advance_minus_stay": diff.map(|(m, h)| json!([m, h])), "about_same": about_same, "better": better,
           "loop_members": minfo, "loop_base": mwhy.is_none(), "loop_note": mwhy,
           "stop": est.stop, "sampling_ms": est.ms, "total_ms": now_ms() - t_all, "careers_simulated": est.careers,
           "horizon_max": pj.horizon_max, "eps": pj.eps})
}

/// An outsider (a trainee no loop member) is picked over the loop's career only with at least this many times its odds
pub(super) const NEXT_OUTSIDE: f64 = 1.5;
/// Candidates the index rule weighs at most, and partners kept per trainee where a candidate's veteran is a parent
const IDX_CANDS: usize = 8;
const IDX_PARTNERS: usize = 2;


/// legal parents a, b for trainee u: different characters, neither hers, at most one borrowed
pub(super) fn legal_pair(d: &Data, p: &Pool, u: usize, a: usize, b: usize) -> bool {
    let id = d.umas[u].id; let (ca, cb) = (p.me[a].chara, p.me[b].chara);
    a != b && ca != cb && ca != id && cb != id && !(p.borrowed[a] && p.borrowed[b])
}
/// Whether builder x's career f (her deck class's proxy) is provably her best, without a career search: over every legal
/// pair the bounds `solve` would prune with (the ceiling; each fully checked deck of her class through `rise`, exact for
/// the class; each product proof of her class through `rise` and the cap) stay at or below f's value, so no deck and pair
/// beats it (a different deck or pair of equal value may exist; the value is the search's). With mob or affinity-free
/// parents every classmate has the same chances, so one classmate's search proves the rest of her class.
fn certified(env: &mut Env, d: &Data, pool: &Pool, fix: &[usize], x: usize, f: &Found) -> bool {
    let nl = pool.me.len(); let ct = env.cls[x];
    for a in 0..nl { for b in a + 1..nl {
        if !legal_pair(d, pool, x, a, b) || !fix.iter().all(|&g| g == a || g == b) { continue; }
        let (q, _, c) = env.lineage(pool, x, a, b);
        let Some(mut ub) = env.ceiling(x, &q, c) else { continue };
        for r in &env.seen[ct] { ub = ub.min(c * r.val * env.rise(x, &q, &r.q)); }
        for (rq, sv) in &env.ruled[ct] { ub = ub.min(env.space.rho_top() * c * sv * env.rise(x, &q, rq)); }
        if ub > f.val * (1.0 + 1e-9) { return false; }
    } }
    true
}
/// trainee u's P(all) with parents a, b (cached decks; a deck search where she has none)
fn val(s: &mut Sim, p: &Pool, u: usize, a: usize, b: usize, buf: &mut Vec<f64>) -> f64 {
    let (q, _, c) = s.env.lineage(p, u, a, b);
    let Some(ub) = s.env.ceiling(u, &q, c) else { return 0.0 };
    s.deck_for(u, &q, c, ub, 0.0, buf).map_or(0.0, |x| x.0)
}
/// Trainee u's best career through the surrogate (`deck_for`, sim module notes): every legal pair (all of `fix` among
/// the parents), highest ceiling first, valued by her cached decks, an exact deck search where the bound beats the table
/// by more than `margin`; stops when the next ceiling is below the best. margin 0: P(all) at least the exact best, up
/// to deck searches that run out of work.
fn best_sur(s: &mut Sim, d: &Data, pool: &Pool, fix: &[usize], u: usize, margin: f64, buf: &mut Vec<f64>) -> Option<Found> {
    let m0 = s.cfg.margin; s.cfg.margin = margin;
    let n = pool.me.len();
    let mut cs: Vec<(f64, usize, usize, Vec<f64>, f64)> = vec![];
    for a in 0..n { for b in a + 1..n {
        if !legal_pair(d, pool, u, a, b) || !fix.iter().all(|&f| f == a || f == b) { continue; }
        let (q, _, c) = s.env.lineage(pool, u, a, b);
        let Some(ub) = s.env.ceiling(u, &q, c) else { continue };
        cs.push((ub, a, b, q, c));
    } }
    cs.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
    let mut best: Option<Found> = None;
    for (ub, a, b, q, c) in cs {
        let bar = best.as_ref().map_or(0.0, |f| f.val);
        if ub <= bar { break; }
        if let Some((v, di)) = s.deck_for(u, &q, c, ub, bar, buf) {
            if v > bar { best = Some(Found { t: u, a, b, val: v, ub, sel: s.cache.decks[u][di].sel.clone() }); }
        }
    }
    s.cfg.margin = m0;
    best
}
/// an 8/8 veteran (a spark of every job target), yours or borrowed
fn is_full(sj: &SkillJob, sparks: &[(u32, u8)]) -> bool { sj.targets.iter().all(|t| sparks.iter().any(|x| x.0 == t.group && x.1 > 0)) }


/// One place a builder's new veteran may be a parent: the stage-1 candidate she may lift (index), per uncertain target
/// (env.uncv order) the miss product without her, per job target the spark scale without and with her spark, her miss
/// factor x per star count (0: absent), how many of the trainee's cached decks it reads (their corner tables; a trainee's
/// cached decks only grow, so the first `nd` are the ones there when the setup was made)
struct Setup { ui: usize, x0: Vec<f64>, pw: Vec<[f64; 2]>, xm: [f64; 4], nd: usize }

/// Two deep: candidates given the term (the highest by index, plus the best career's trainee), and builders tried
const TWO_TOP: usize = 3;
/// Scores within this many careers per run of the best (or 5% of it) count as tied
const NEAR: f64 = 0.25;
const TWO_BUILDERS: usize = 3;
/// Builders through their deck class (`index_rule`): a proxy is searched exactly when her lift efficiency is within this
/// share of a candidate's second best, or her saving within this share of the lowest saving the shown rows, the pick's
/// near-tie band or two deep take. Recalibrated 2026-10-09 (val/plan `proxy_check`, with the own-8/8 credit): the proxy is
/// the exact career for every builder on five of the seven jobs; it reads at most 2.79% low on a contender's efficiency
/// (exact at least 0.7 of a candidate's second best; p11) and 2.67% low on a builder's saving (p11): 0.06 is 2.15x and
/// 2.25x those. (The first measurement, on the planner as it was before the class winners and the merges since, had 11%
/// and 9%, with 0.2.) The errors move with the model: run `proxy_check` again after a change to what builders' careers
/// are made of
const PROXY_EFF: f64 = 0.06;
const PROXY_SAVE: f64 = 0.06;
/// the rows the page shows (career.js `OPT_SHOW`): their careers are searched exactly
const OPT_SHOW: usize = 8;
/// Two deep follows rolls at most this many targets short of 8/8 (on the owner's 10-veteran pool rolls with 4 of 8 never
/// opened a builder better than the best career now; 6 and 7 of 8 carried the term)
const TWO_MISS: usize = 1;
/// `val` with the deck it found (its index in the cache)
fn val_d(s: &mut Sim, p: &Pool, u: usize, a: usize, b: usize, buf: &mut Vec<f64>) -> Option<(f64, usize)> {
    let (q, _, c) = s.env.lineage(p, u, a, b);
    let ub = s.env.ceiling(u, &q, c)?;
    s.deck_for(u, &q, c, ub, 0.0, buf)
}
/// Trainee u's odds with a full loop lineage: parents hypothetical veterans of b1 and b2 (every target, stars at the
/// rank's mix, G1 wins `w1` / `w2`), each bred from the other and from u (grandparents every target at 2 stars)
#[allow(clippy::too_many_arguments)]
fn loop_ceiling(s: &mut Sim, d: &Data, legs: &[Legacy], u: usize, b1: usize, b2: usize, wins: &dyn Fn(usize) -> Vec<usize>, buf: &mut Vec<f64>) -> f64 {
    let sj = s.env.sj; let star = s.env.sj.star_dist;
    let full: Vec<(u32, u8)> = sj.targets.iter().map(|t| (t.group, 2)).collect();
    let anc = |x: usize| Ancestor { chara: d.umas[x].id, sparks: full.clone(), wins: wins(x), pinks: vec![] };
    let mk = |x: usize, p1: usize, p2: usize| Legacy { chara: d.umas[x].id, borrowed: false, sparks: full.clone(), wins: wins(x), pinks: vec![],
        parents: vec![anc(p1), anc(p2)], win_p: vec![], mix: Some(star) };
    let mut l = legs.to_vec(); l.push(mk(b1, b2, u)); l.push(mk(b2, b1, u));
    let p = s.env.pool(&l); let n = l.len();
    if !legal_pair(d, &p, u, n - 2, n - 1) { return 0.0; }
    val(s, &p, u, n - 2, n - 1, buf)
}
/// Below this many stage candidates the plan adds umas by `hint_umas` (the loop planner's search added up to three; a
/// pool with veterans of its loop's umas gives four or more by the other routes: 6-8 on the val/plan pools)
const HINT_FLOOR: usize = 4;
/// The owned umas (a legal deck, not among `cands`) with the highest odds under a full loop lineage, at most `k`, best
/// first: her parents hypothetical veterans of the first two candidates' umas (every target, stars at the rank's mix),
/// each bred from the other and from her, as `loop_ceiling` builds p_inf's lineage (a missing candidate: an ancestor of
/// no uma, so no relation points); no G1 wins on the hypothetical ancestors (the race part of their affinity is the
/// same for every uma ranked, and the wins come from schedules the pool doesn't have yet). Valued by `Env::ceiling` (the
/// per-target best deck odds: no deck search), which ranks the umas the way a full loop would before any building.
fn hint_umas(env: &mut Env, d: &Data, legs: &[Legacy], cands: &[usize], trainees: &[usize], k: usize) -> Vec<(usize, f64)> {
    let sj = env.sj; let star = sj.star_dist;
    let full: Vec<(u32, u8)> = sj.targets.iter().map(|t| (t.group, 2)).collect();
    let mut sides: Vec<u32> = vec![];
    for &c in cands { let id = d.umas[c].id; if !sides.contains(&id) { sides.push(id); } if sides.len() == 2 { break; } }
    while sides.len() < 2 { sides.push(0); }
    let (ca, cb) = (sides[0], sides[1]);
    let anc = |chara: u32| Ancestor { chara, sparks: full.clone(), wins: vec![], pinks: vec![] };
    let mk = |chara: u32, p1: u32, p2: u32| Legacy { chara, borrowed: false, sparks: full.clone(), wins: vec![], pinks: vec![],
        parents: vec![anc(p1), anc(p2)], win_p: vec![], mix: Some(star) };
    let mut out: Vec<(usize, f64)> = vec![];
    for &u in trainees {
        let id = d.umas[u].id;
        if cands.contains(&u) || env.dmax[u].is_none() || id == ca || id == cb { continue; }
        let mut l = legs.to_vec(); l.push(mk(ca, cb, id)); l.push(mk(cb, ca, id));
        let p = env.pool(&l); let n = l.len();
        if !legal_pair(d, &p, u, n - 2, n - 1) { continue; }
        let (q, _, c) = env.lineage(&p, u, n - 2, n - 1);
        if let Some(v) = env.ceiling(u, &q, c) { if v > 0.0 { out.push((u, v)); } }
    }
    // best first; equal odds in uma order (stable)
    out.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap());
    out.truncate(k);
    out
}
/// What a career is scored against: stage-1 costs per candidate (index into `cands`; 0 = her 8/8 exists), stages 2-3 per
/// A, V0, and the stage-1 odds now
pub(super) struct ScoreBase<'b> { pub(super) cands: &'b [usize], pub(super) c1: Vec<f64>, pub(super) m23: &'b [f64], pub(super) v0: f64, pub(super) pnow: Vec<f64>, pub(super) cont: Vec<Cont>,
    /// the trios (A, B as candidate indices, C's step, A and B from a real 8/8 of hers): V = min over them of A's step +
    /// B's step + C's step, a side from a real 8/8 costing 0, else c1; empty: min over a of c1[a] + m23[a]
    pub(super) combos: Vec<Trio>,
    /// per candidate, the trios' sides: (a real 8/8 veteran of hers, the legacy): her real 8/8s and the 8/8 her best career
    /// now would leave; with `all` (the umas a trio's C may be), what `her_trios` pairs a new real 8/8 with. Empty: none
    pub(super) sides: &'b [Vec<(bool, Legacy)>], pub(super) all: &'b [usize] }
/// (A, B, C's step, A from a real 8/8, B from a real 8/8)
pub(super) type Trio = (usize, usize, f64, bool, bool);
/// V over the trios at step costs c1
fn trio_v(c1: &[f64], combos: &[Trio]) -> f64 {
    // (a plain comparison: no value here is NaN, so the same minimum as f64::min)
    let mut best = f64::INFINITY;
    for &(ia, ib, c3, ra, rb) in combos { let v = (if ra { 0.0 } else { c1[ia] }) + (if rb { 0.0 } else { c1[ib] }) + c3; if v < best { best = v; } }
    best
}
/// An exact memo of a pure function of its key, owned (through an `Rc`) by the one object whose fixed parameters the
/// function reads; a hit returns the value a miss computed for the same key bits, so outputs are the same by
/// construction. The gate only decides whether to look up: after MEMO_PROBE lookups with fewer than one hit in
/// MEMO_MIN_HIT it stops (the hash and insert would cost more than the misses save). Past MEMO_CAP entries it is cleared
/// (the wasm page's memory).
pub(super) struct Memo<K, V> { map: FxMap<K, V>, look: u64, hit: u64, off: bool, kind: usize, cap: usize }
const MEMO_PROBE: u64 = 4096;
const MEMO_MIN_HIT: u64 = 8;
/// (measured on val/plan: a Cont memo without a cap would hit 0.2-1.5% more often; 1 << 14 entries hold about 0.6 MB)
const MEMO_CAP: usize = 1 << 14;
/// bit equality of memoised values (the audit's test)
pub(super) trait SameBits { fn same_bits(&self, o: &Self) -> bool; }
impl SameBits for f64 { fn same_bits(&self, o: &Self) -> bool { self.to_bits() == o.to_bits() } }
impl SameBits for (f64, i32, f64) { fn same_bits(&self, o: &Self) -> bool { self.0.same_bits(&o.0) && self.1 == o.1 && self.2.same_bits(&o.2) } }
impl<K: std::hash::Hash + Eq, V: Copy + SameBits + std::fmt::Debug> Memo<K, V> {
    // (`PLAN_MEMO_OFF`: every memo off from the start, for the test that compares against no memo)
    // (`PLAN_MEMO_CAP`: another cap, for the test: entries kept long enough to be read after a refit)
    fn new(kind: usize) -> Self { let e = memo_env(); Memo { map: FxMap::default(), look: 0, hit: 0, off: e.0, kind, cap: e.2 } }
    #[inline]
    fn get_or(&mut self, k: impl FnOnce() -> K, f: impl FnOnce() -> V) -> V {
        if self.off { return f(); }
        self.look += 1;
        let k = k();
        if let Some(&v) = self.map.get(&k) {
            self.hit += 1;
            // (`PLAN_MEMO_CHECK`: every hit computed again and compared bit for bit; tests/plan_memo.rs)
            if memo_env().1 { let w = f(); assert!(v.same_bits(&w), "memo kind {}: hit {v:?}, computed {w:?}", self.kind); }
            return v;
        }
        let v = f();
        if self.look >= MEMO_PROBE && self.hit * MEMO_MIN_HIT < self.look { self.off = true; self.map = FxMap::default(); return v; }
        if self.map.len() >= self.cap { self.map.clear(); memo_stat(7); }
        self.map.insert(k, v);
        v
    }
}
// (`PLAN_MEMO`: per plan, each memo kind's lookups, hits and memos gated off, the Cont refits and the cap's clears, on
// stderr)
thread_local! { static MEMO_STATS: std::cell::Cell<[u64; 8]> = const { std::cell::Cell::new([0; 8]) }; }
impl<K, V> Drop for Memo<K, V> {
    fn drop(&mut self) { MEMO_STATS.with(|c| { let mut s = c.get(); s[3 * self.kind] += self.look; s[3 * self.kind + 1] += self.hit; s[3 * self.kind + 2] += self.off as u64; c.set(s); }); }
}
/// (PLAN_MEMO_OFF, PLAN_MEMO_CHECK, PLAN_MEMO_CAP or MEMO_CAP)
fn memo_env() -> (bool, bool, usize) {
    static E: std::sync::OnceLock<(bool, bool, usize)> = std::sync::OnceLock::new();
    *E.get_or_init(|| (std::env::var("PLAN_MEMO_OFF").is_ok(), std::env::var("PLAN_MEMO_CHECK").is_ok(),
                       std::env::var("PLAN_MEMO_CAP").ok().and_then(|v| v.parse().ok()).unwrap_or(MEMO_CAP)))
}
fn memo_stat(i: usize) { MEMO_STATS.with(|c| { let mut s = c.get(); s[i] += 1; c.set(s); }); }
struct MemoReport;
impl Drop for MemoReport {
    fn drop(&mut self) {
        let s = MEMO_STATS.with(|c| c.replace([0; 8]));
        if std::env::var("PLAN_MEMO").is_ok() {
            let r = |l: u64, h: u64| if l == 0 { 0.0 } else { 100.0 * h as f64 / l as f64 };
            eprintln!("PLAN_MEMO plan lookups {} hits {} ({:.1}%) off {} | cost_lo lookups {} hits {} ({:.1}%) off {} | refits {} clears {}",
                s[0], s[1], r(s[0], s[1]), s[2], s[3], s[4], r(s[3], s[4]), s[5], s[6], s[7]);
        }
    }
}
/// A step's continuation (form assumed; the rate and step fitted to the pool now, the shape to traces): builder careers
/// lift her odds, each lift closing `phi` of the remaining gap in log odds to `pinf`, her effective ceiling; the lift
/// chance falls with the remaining gap: lam_j = lam ((1 - phi)^j)^LAM_DECAY. pinf = CEIL_SHARE x her full-loop odds.
/// Both constants fitted (val/sim/README.md E5) to traces of building for Agnes Digital on the owner's 2-, 5-, 11- and
/// 19-veteran pools (the best builder each career, 6 runs x 400 careers): within 10-20% on 11 and 19, building
/// underestimated on 5 and overestimated early on 2; leave-one-pool-out fits ranged 0.6-1.0 and 1-3. lam = 0: none.
#[derive(Clone)]
pub(super) struct Cont { lam: f64, phi: f64, pinf: f64,
    /// per n (0..=CONT_N): the careers built through the n-th success (the sum `plan` takes, in its order), and (1 - phi)^n:
    /// `plan` reads them instead of evaluating the powers per call (the same expressions, so the same values)
    tab: std::rc::Rc<Vec<(f64, f64)>>,
    /// `plan`'s and `cost_lo`'s results by the bits of p. Exact: both read p and lam, phi, pinf and tab alone, which are
    /// set once in `new` and never written after (private fields; a refit makes new Conts). A clone shares the memos and
    /// carries the same parameters; a new or default Cont gets its own
    memo: std::rc::Rc<ContMemo> }
pub(super) struct ContMemo { plan: std::cell::RefCell<Memo<u64, (f64, i32, f64)>>, lo: std::cell::RefCell<Memo<u64, f64>> }
fn cont_memo() -> std::rc::Rc<ContMemo> { std::rc::Rc::new(ContMemo { plan: std::cell::RefCell::new(Memo::new(0)), lo: std::cell::RefCell::new(Memo::new(1)) }) }
impl Default for Cont {
    fn default() -> Self { Cont { lam: 0.0, phi: 0.0, pinf: 0.0, tab: Default::default(), memo: cont_memo() } }
}
/// `plan`'s cap on successes before grinding
const CONT_N: usize = 500;
/// Effective ceiling as a share of the full-loop odds, and the lift chance's decline exponent (val/sim/README.md E5)
const CEIL_SHARE: f64 = 0.6;
const LAM_DECAY: f64 = 1.5;
impl Cont {
    fn new(lam: f64, phi: f64, pinf: f64) -> Cont {
        let r = 1.0 - phi;
        let mut tab = Vec::with_capacity(CONT_N + 1); let mut built = 0.0f64;
        for n in 0..=CONT_N as i32 { let b = built + 1.0 / (lam * r.powi(n).powf(LAM_DECAY)); tab.push((b, r.powi(n))); built = b; }
        Cont { lam, phi, pinf, tab: std::rc::Rc::new(tab), memo: cont_memo() }
    }
    /// (expected careers to her 8/8 from odds p, the best number of successes before grinding, the odds then): build n
    /// successes (sum of 1 / lam_j careers), then grind at p_n; the best n
    fn plan(&self, p: f64) -> (f64, i32, f64) {
        if p <= 0.0 { return (f64::INFINITY, 0, p); }
        if self.lam <= 0.0 || self.phi <= 0.0 || self.pinf <= p { return (1.0 / p, 0, p); }
        self.memo.plan.borrow_mut().get_or(|| p.to_bits(), || self.plan_n(p))
    }
    /// `plan` past its early returns (the first stop of the building loop)
    fn plan_n(&self, p: f64) -> (f64, i32, f64) {
        if self.tab.len() > CONT_N {
            let lr = (p / self.pinf).ln(); let t = &self.tab;
            let (mut n, mut best) = (0usize, 1.0 / p);
            while n < CONT_N {
                let x = t[n].0 + 1.0 / (self.pinf * (lr * t[n + 1].1).exp());
                if x >= best { break; }
                best = x; n += 1;
            }
            return (best, n as i32, self.pinf * (lr * t[n].1).exp());
        }
        let r = 1.0 - self.phi; let lr = (p / self.pinf).ln();
        let pn = |n: i32| self.pinf * (lr * r.powi(n)).exp();
        let (mut n, mut built, mut best) = (0i32, 0.0f64, 1.0 / p);
        while n < 500 {
            let b = built + 1.0 / (self.lam * r.powi(n).powf(LAM_DECAY));
            let x = b + 1.0 / pn(n + 1);
            if x >= best { break; }
            best = x; built = b; n += 1;
        }
        (best, n, pn(n))
    }
    fn cost(&self, p: f64) -> f64 { self.plan(p).0 }
    /// a lower bound on `cost` at every odds up to p: the minimum over every n (not the first local one `plan` stops at).
    /// Each n's cost B_n + 1 / p_n falls as p rises (p_n = pinf exp(ln(p / pinf) r^n) rises with p), so this minimum falls
    /// with p, and at any p' <= p it is at most the minimum at p', which is at most `cost(p')`
    fn cost_lo(&self, p: f64) -> f64 {
        if p <= 0.0 { return f64::INFINITY; }
        if self.lam <= 0.0 || self.phi <= 0.0 || self.pinf <= p { return 1.0 / p; }
        self.memo.lo.borrow_mut().get_or(|| p.to_bits(), || self.cost_lo_n(p))
    }
    /// `cost_lo` past its early returns (every n: CONT_N exp; `outcome_hi` asks for it per outcome and candidate)
    fn cost_lo_n(&self, p: f64) -> f64 {
        let lr = (p / self.pinf).ln(); let mut best = 1.0 / p;
        // every n's cost is its building (rising with n: each term 1/lam_j > 0) plus a positive grind, so once the
        // building alone reaches the best found no later n can be lower: the scan stops there, the same minimum
        if self.tab.len() > CONT_N { let t = &self.tab; for n in 0..CONT_N { if t[n].0 >= best { break; } best = best.min(t[n].0 + 1.0 / (self.pinf * (lr * t[n + 1].1).exp())); } return best; }
        let r = 1.0 - self.phi; let mut built = 0.0f64;
        for n in 0..500i32 { built += 1.0 / (self.lam * r.powi(n).powf(LAM_DECAY)); if built >= best { break; } best = best.min(built + 1.0 / (self.pinf * (lr * r.powi(n + 1)).exp())); }
        best
    }
}
impl ScoreBase<'_> {
    // (no Cont: a default one's, lam = 0: grinding at p)
    fn cost(&self, ui: usize, p: f64) -> f64 { match self.cont.get(ui) { Some(c) => c.cost(p), None => if p <= 0.0 { f64::INFINITY } else { 1.0 / p } } }
    fn cost_lo(&self, ui: usize, p: f64) -> f64 { match self.cont.get(ui) { Some(c) => c.cost_lo(p), None => if p <= 0.0 { f64::INFINITY } else { 1.0 / p } } }
}
/// A career's own 8/8 as a trio side when her uma is not a stage candidate (her outcome lands every target): the replan
/// would take her uma in (a veteran of hers) with that real 8/8 a free side. Per candidate side (real or fresh) her 8/8 can
/// pair with, C's step: 1 / the best allowed C's value with the two 8/8s as parents (every allowed uma but hers and the
/// partner's, best ceiling first, the value taken until no ceiling can beat the best: the best C by value, as `combos_of`
/// finds one). Her 8/8 is `lv[iv]` (`Env::veteran`: every target at the rank's star mix), valued as every fresh side is
/// (the trios' own 8/8s use the rank's mix too), so the trios with her compare like for like with the others; one per
/// call, not per star pattern (the full set's stars vary per leaf: per leaf it would be 4^nu searches for C).
/// (B's candidate index, partner side real, C's step)
fn her_trios(s: &mut Sim, d: &Data, bs: &ScoreBase, lv: &[Legacy], iv: usize, x: usize, buf: &mut Vec<f64>) -> Vec<(usize, bool, f64)> {
    let her_id = d.umas[x].id; let mut out = vec![];
    // the pool of lv, with each side pushed in turn (the same Pool as `Env::pool` of lv and the side)
    let mut p8 = s.env.pool(lv); let il = lv.len();
    for ib in 0..bs.cands.len() {
        let pu = bs.cands[ib]; if d.umas[pu].id == her_id { continue; }
        for (real, leg) in bs.sides.get(ib).map_or(&[][..], |v| &v[..]) {
            s.env.pool_push(&mut p8, leg);
            let mut cs: Vec<(f64, usize)> = bs.all.iter().copied()
                .filter(|&cu| d.umas[cu].id != her_id && d.umas[cu].id != d.umas[pu].id && legal_pair(d, &p8, cu, iv, il))
                .filter_map(|cu| { let (q, _, c) = s.env.lineage(&p8, cu, iv, il); s.env.ceiling(cu, &q, c).map(|ub| (ub, cu)) }).collect();
            cs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let mut best = 0.0f64;
            for (ub, cu) in cs { if ub * (1.0 + 1e-9) <= best { break; } let v = val(s, &p8, cu, iv, il, buf); if v > best { best = v; } }
            if best > 0.0 { out.push((ib, *real, 1.0 / best)); }
            s.env.pool_pop(&mut p8);
        }
    }
    out
}
/// V after her with her own 8/8 a trio side (`her_trios`), at the candidates' step costs c1 (a real partner side free)
fn with_her(v: f64, her: &[(usize, bool, f64)], c1: &[f64]) -> f64 {
    her.iter().map(|&(ib, real, c3)| (if real { 0.0 } else { c1[ib] }) + c3).fold(v, f64::min)
}
/// Score of career x (parents a, b of `legs`, deck `sel`) against `bs`: E[V0 - V0 with her veteran], the chance it is
/// above 0, and (Y, P) per outcome. Exact over her landed set (`outcome_sets`) and her stars per uncertain target (the
/// rank's mix): her veteran as a parent of each stage-1 candidate (her best IDX_PARTNERS partners, the trainee's cached
/// decks' corner tables, folded one target at a time); her own 8/8 ends her stage when she is a candidate.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub(super) fn score_career(s: &mut Sim, d: &Data, bs: &ScoreBase, legs: &[Legacy], pool: &Pool, x: usize, a: usize, b: usize, sel: &Sel, buf: &mut Vec<f64>) -> (f64, f64, Vec<f64>, Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let n = bs.cands.len(); let ntar = s.env.ntar; let nu = s.env.uncv.len();
    let uncv = s.env.uncv.clone(); let fixed_t = s.env.fixed_t.clone(); let w = s.env.sj.star_dist;
    let v0_after = |c1: &[f64]| if bs.combos.is_empty() { (0..n).map(|a| c1[a] + bs.m23[a]).fold(f64::INFINITY, f64::min) }
        else { trio_v(c1, &bs.combos) };
    let xi = bs.cands.iter().position(|&u| u == x);
    // two 8/8 parents: her own 8/8 is a fully looped parent (the goal: V after is 0)
    let goal = is_full(s.env.sj, &legs[a].sparks) && is_full(s.env.sj, &legs[b].sparks);
    let c = s.make_cur(pool, x, a, b, sel, false);
    let mut lv = legs.to_vec(); lv.push(s.env.veteran(legs, x, a, b, c.wp.to_vec())); let iv = lv.len() - 1;
    let pv = s.env.pool(&lv);
    // where she can be a parent: per stage-1 candidate (stage 1 still to run) her best partners
    let mut setups: Vec<Setup> = vec![];
    for ui in 0..n {
        let u = bs.cands[ui];
        if d.umas[u].id == d.umas[x].id || bs.c1[ui] == 0.0 || !bs.c1[ui].is_finite() { continue; }
        let mut ps: Vec<(f64, usize)> = (0..iv).filter(|&y| legal_pair(d, &pv, u, y, iv)).map(|y| (val(s, &pv, u, y, iv, buf), y)).collect();
        ps.sort_by(|p, q| q.0.partial_cmp(&p.0).unwrap()); ps.truncate(IDX_PARTNERS);
        for (_, y) in ps {
            let sl = s.env.slots(&pv, u, y, iv);
            let Some(pos) = sl.iter().position(|z| z.parent && z.legacy == iv) else { continue };
            let a_v = sl[pos].rel + sl[pos].race;
            let mut sl0 = sl.clone(); { let s = std::rc::Rc::make_mut(&mut sl0[pos].stars); for k in 0..ntar { s[k] = 0; } } sl0[pos].mix = None;
            let (q0, nc0) = inherit(&sl0, ntar);
            let xm = |st: u8| { let m = 1.0 - (0.03 * st as f64 * (1.0 + a_v / 100.0)).min(1.0); m * m };
            let nd = s.cache.decks[u].len();
            if s.cache.decks[u].iter().all(|dk| dk.tab.is_empty()) { continue; }
            setups.push(Setup { ui, x0: uncv.iter().map(|&i| 1.0 - q0[i]).collect(),
                pw: (0..ntar).map(|k| [1.1f64.powi(nc0[k] - 6), 1.1f64.powi(nc0[k] + 1 - 6)]).collect(), xm: [1.0, xm(1), xm(2), xm(3)], nd });
        }
    }
    // per setup, per star pattern of the uncertain targets (base 4, position k = uncv[k]; 0 absent), the best table value
    // at scale 1: the tables folded one coordinate at a time (interp's order and arithmetic)
    let nleaf = 1usize << (2 * nu);
    // (one scratch buffer per level, reused by the four branches below it: the same values in the same order)
    let mut scratch: Vec<Vec<f64>> = vec![vec![]; nu];
    // (setup-major, one buffer: fl[si * nleaf + leaf])
    let ns = setups.len();
    let mut fl = vec![0.0f64; ns * nleaf];
    for (st, out) in setups.iter().zip(fl.chunks_exact_mut(nleaf.max(1))) {
        // per target and star level: x and 1 - x (the same expressions the fold took per branch)
        let xs: Vec<[f64; 4]> = (0..nu).map(|k| [0usize, 1, 2, 3].map(|sg| st.x0[k] * st.xm[sg])).collect();
        let xc: Vec<[f64; 4]> = xs.iter().map(|r| r.map(|x| 1.0 - x)).collect();
        for tab in s.cache.decks[bs.cands[st.ui]][..st.nd].iter().flat_map(|dk| dk.tab.iter()) {
            fn rec(t: &[f64], k: usize, leaf: usize, xs: &[[f64; 4]], xc: &[[f64; 4]], nu: usize, out: &mut [f64], sc: &mut [Vec<f64>]) {
                if k == nu { if t[0] > out[leaf] { out[leaf] = t[0]; } return; }
                if nu - k == 2 {
                    // the last two levels unrolled: t holds 4 values
                    let (t0, t1, t2, t3) = (t[0], t[1], t[2], t[3]);
                    let (xa, ca, xb, cb) = (&xs[k], &xc[k], &xs[k + 1], &xc[k + 1]);
                    for sg in 0..4usize {
                        let f0 = t0 * ca[sg] + t1 * xa[sg]; let f1 = t2 * ca[sg] + t3 * xa[sg];
                        let l1 = leaf | sg << (2 * k);
                        for sg2 in 0..4usize {
                            let v = f0 * cb[sg2] + f1 * xb[sg2];
                            let li = l1 | sg2 << (2 * (k + 1));
                            if v > out[li] { out[li] = v; }
                        }
                    }
                    return;
                }
                let (f, rest) = sc.split_first_mut().unwrap();
                for sg in 0..4usize {
                    let (x, c) = (xs[k][sg], xc[k][sg]);
                    let h = t.len() / 2;
                    f.clear(); f.extend((0..h).map(|i| t[2 * i] * c + t[2 * i + 1] * x));
                    rec(f, k + 1, leaf | sg << (2 * k), xs, xc, nu, out, rest);
                }
            }
            rec(tab, 0, 0, &xs, &xc, nu, out, &mut scratch);
        }
    }
    // leaf-major for the outcome loop (each leaf's setups contiguous): fol[leaf * ns + si]
    let mut fol: Vec<f64> = Vec::with_capacity(nleaf * ns);
    for leaf in 0..nleaf { fol.extend((0..ns).map(|si| fl[si * nleaf + leaf])); }
    drop(fl);
    // every outcome: uncertain targets' star pattern, guaranteed golds present or not
    let nf = fixed_t.len();
    let (mut ey, mut lift) = (0.0f64, 0.0f64);
    // y per outcome (careers saved), at leaf << nf | fm (0 where the outcome can't happen)
    let mut ys: Vec<f64> = vec![0.0; nleaf << nf];
    let mut c1 = bs.c1.clone();
    // per stage-1 candidate: the chance her veteran raises that candidate's odds, and the odds raised to (summed, weighted)
    let mut stats = vec![(0.0f64, 0.0f64); n];
    let mut dist0: Vec<(f64, f64)> = vec![];
    // per setup and landed set m, the spark scale (the product the loop below took per outcome, the same order); V after
    // her when no candidate's step moves (most outcomes lift no one): once
    let nm = 1usize << ntar;
    // (m-major: scf[m * ns + si])
    let mut scf: Vec<f64> = Vec::with_capacity(nm * ns);
    for m in 0..nm { scf.extend(setups.iter().map(|st| (0..ntar).map(|k| st.pw[k][(m >> k & 1) as usize]).product::<f64>())); }
    let sui: Vec<usize> = setups.iter().map(|st| st.ui).collect();
    let v_same = v0_after(&bs.c1);
    // the candidates the loop below can lift, in candidate order: those with setups, and (never, as the odds are not
    // negative) one whose odds now already clear her threshold pnow (1 + 1e-9); every other keeps up = pnow, below it
    let thr: Vec<f64> = (0..n).map(|ui| bs.pnow[ui] * (1.0 + 1e-9)).collect();
    let active: Vec<usize> = (0..n).filter(|&ui| sui.contains(&ui) || bs.pnow[ui] > thr[ui]).collect();
    let mut up = bs.pnow.clone();
    let fmm: Vec<usize> = (0..(1usize << fixed_t.len())).map(|fm| { let mut m = 0; for (j, &i) in fixed_t.iter().enumerate() { if fm >> j & 1 == 1 { m |= 1 << i; } } m }).collect();
    let mut ch: Vec<usize> = vec![];
    // her own 8/8 as a trio side when her uma is not a stage candidate (the full set only; `her_trios`)
    let full_m = (1usize << ntar) - 1;
    let her: Vec<(usize, bool, f64)> = if xi.is_none() && !goal && !bs.sides.is_empty() && c.os[full_m] > 0.0 { her_trios(s, d, bs, &lv, iv, x, buf) } else { vec![] };
    for leaf in 0..nleaf {
        let mut mu = 0usize; let mut pr_st = 1.0f64;
        for k in 0..nu { let sg = (leaf >> (2 * k)) & 3; if sg > 0 { mu |= 1 << uncv[k]; pr_st *= w[sg - 1]; } }
        let fr = &fol[leaf * ns..(leaf + 1) * ns];
        for (fm, &mf) in fmm.iter().enumerate() {
            let m = mu | mf;
            let pr = c.os[m] * pr_st;
            if pr <= 0.0 { continue; }
            let sr = &scf[m * ns..(m + 1) * ns];
            // up and c1 are the base's here (pnow, bs.c1)
            for ((&sc, &f), &ui) in sr.iter().zip(fr).zip(&sui) { let v = sc * f; if v > up[ui] { up[ui] = v; } }
            // the candidates whose step moved (if none, c1 is the base's and V after is v_same); up back to pnow
            ch.clear();
            for &ui in &active {
                let upv = up[ui]; up[ui] = bs.pnow[ui];
                if upv > thr[ui] {
                    stats[ui].0 += pr; stats[ui].1 += pr * upv;
                    if ui == 0 { dist0.push((pr, upv)); }
                    let cc = bs.cost(ui, upv); if cc < c1[ui] { c1[ui] = cc; ch.push(ui); } }
            }
            if m == full_m { if let Some(xi) = xi { c1[xi] = 0.0; ch.push(xi); } }
            let mut va = if !ch.is_empty() { v0_after(&c1) } else { v_same };
            if m == full_m && !her.is_empty() { va = with_her(va, &her, &c1); }
            // c1 back to the base's
            for &i in &ch { c1[i] = bs.c1[i]; }
            let y = if goal && m == full_m { bs.v0 } else { (bs.v0 - va).max(0.0) };
            ey += pr * y; if y > 1e-9 { lift += pr; }
            ys[leaf << nf | fm] = y;
        }
    }
    (ey, lift, ys, stats, dist0)
}

/// Per leaf (base-4 digits, digit k at bits 2k..2k+1) the product of g[k][digit k] over k, into `out`: level by level,
/// each level's products the previous level's (at the leaf's lower digits) times g[k][digit k], from 1.0. The same
/// operations in the same order as `let mut pr = 1.0; for k in 0..nu { pr *= g[k][digit k]; }` per leaf, so the same
/// f64s; each prefix is computed once instead of once per leaf (4/3 4^nu products for nu 4^nu)
fn leaf_products(g: &[[f64; 4]], out: &mut Vec<f64>, tmp: &mut Vec<f64>) {
    out.clear(); out.push(1.0);
    for (k, gk) in g.iter().enumerate() {
        let low = (1usize << (2 * k)) - 1;
        tmp.clear(); tmp.extend((0..4usize << (2 * k)).map(|i| out[i & low] * gk[i >> (2 * k)]));
        std::mem::swap(out, tmp);
    }
}
/// Per leaf (as `leaf_products`) the best outfit's `Env::rise` of trainee (sts, ews: her outfits' coverage and own
/// chances) from chances qa (job-target order) to q'(leaf), q'_k = 1 - x0[k] xms[her stars on uncertain target k]:
/// per outfit and target the factor at each of her four star levels (1 where it can't rise), their product per leaf
#[allow(clippy::too_many_arguments)]
fn rise_leaves(sts: &[Vec<u8>], ews: &[Vec<f64>], x0: &[f64], xms: &[f64; 4], qa: &[f64], uncv: &[usize], lp: &mut Vec<f64>, lp_tmp: &mut Vec<f64>) -> Vec<f64> {
    let nu = uncv.len(); let mut rz = vec![0.0f64; 1usize << (2 * nu)];
    for (st, ew) in sts.iter().zip(ews) {
        let g: Vec<[f64; 4]> = (0..nu).map(|k| { let i = uncv[k]; [0, 1, 2, 3].map(|sg| {
            let qn = 1.0 - x0[k] * xms[sg];
            if st[k] > 0 || qn <= qa[i] { 1.0 } else { (1.0 - (1.0 - ew[k]) * (1.0 - qn)) / (1.0 - (1.0 - ew[k]) * (1.0 - qa[i])) } }) }).collect();
        leaf_products(&g, lp, lp_tmp);
        for (rv, &pr) in rz.iter_mut().zip(lp.iter()) { if pr > *rv { *rv = pr; } }
    }
    rz
}
/// `outcome_hi` at one outcome (her stars `leaf`, landed set m): each candidate's odds `up` at the most any partner's
/// bound allows (into `up`), then `saving_lo`; `c1` scratch
#[allow(clippy::too_many_arguments)]
fn hi_at(bs: &ScoreBase, hs: &[(usize, Vec<f64>, Vec<f64>)], leaf: usize, m: usize, full_m: usize, xi: Option<usize>, her: &[(usize, bool, f64)], goal: bool, c1: &mut [f64], up: &mut [f64]) -> f64 {
    up.copy_from_slice(&bs.pnow);
    for (ui, bd, sc) in hs { let v = sc[m] * bd[leaf]; if v > up[*ui] { up[*ui] = v; } }
    saving_lo(bs, up, m, full_m, xi, her, goal, c1)
}
/// V - V_lo at candidate odds `up`: each step that rose at `Cont::cost_lo`, her own 8/8 (`xi`, `her_trios`) on the full
/// set, the goal's V; `c1` scratch
#[allow(clippy::too_many_arguments)]
fn saving_lo(bs: &ScoreBase, up: &[f64], m: usize, full_m: usize, xi: Option<usize>, her: &[(usize, bool, f64)], goal: bool, c1: &mut [f64]) -> f64 {
    let n = bs.cands.len();
    c1.copy_from_slice(&bs.c1);
    for ui in 0..n { if up[ui] > bs.pnow[ui] * (1.0 + 1e-9) { let cc = bs.cost_lo(ui, up[ui]); if cc < c1[ui] { c1[ui] = cc; } } }
    if m == full_m { if let Some(xi) = xi { c1[xi] = 0.0; } }
    let mut va = if bs.combos.is_empty() { (0..n).map(|a| c1[a] + bs.m23[a]).fold(f64::INFINITY, f64::min) } else { trio_v(c1, &bs.combos) };
    if m == full_m && !her.is_empty() { va = with_her(va, her, c1); }
    if goal && m == full_m { bs.v0 } else { (bs.v0 - va).max(0.0) }
}
/// The pick's bound's exact anchors (`outcome_hi`): her best deck searched with her at each of these stars on every
/// uncertain target, for at most HI_SEARCHES / 2 partners (with no limit p16 took 5 and left the same). Callgrind per
/// search: p16 55M, gm16 151M instructions (the val/plan jobs +6.0% and +11.1% in all). A leaf whose stars are all at
/// most 1 is rare at the rank's mix: adding 1 star moved p16's provisional 0.0275 -> 0.0272, gm16 0.0796 -> 0.0742 for
/// +8.8% and +16.5%
const HI_STARS: [usize; 2] = [2, 3];
/// Off by default (owner, 2026-10-09: not worth the plan's +6-11%); 10 gives the numbers above.
const HI_SEARCHES: usize = 0;
/// ... and only for band outcomes whose partners' bounds are within this factor of p_now: a cost rule, not part of the
/// bound. It skips p10 and p11, whose band needs partners at 2.5 times p_now and more: there 15 searches (at 1, 2 and 3
/// stars) freed nothing (p11: 233 ms native spent), and only 63 (805 ms) moved p10 (0.474 -> 0.378); 1.3 and 1.5 chose
/// the same
const HI_RISE: f64 = 1.5;
/// The pick's upper bound per outcome (`outcomes.y_hi`, read by the page's reroll): the most V can fall through her
/// veteran's DIRECT lifts. The outcomes, base and own-8/8 credit (`her_trios`) are `score_career`'s; but each stage
/// candidate's odds with her as a parent, with EVERY legal partner y (not the table value of her IDX_PARTNERS best), are
/// bounded from the candidate's exact best now (pnow): every pair (y, z) of today's pool that keeps the plan's places is
/// a career of hers worth at most pnow, so (`Env::rise`: E(q') <= E(q) x rise(q', q) for every deck)
///     value(y, her) = c' E_best(q') <= c' (pnow / c_yz) rise(q', q_yz),
/// the least over z (the veteran most like her: rise 1 when one already carries what she does), and over the main
/// search's found careers f of her deck class that it fully checked (a `Ref` at q_f: every deck of every classmate is
/// worth at most val_f there, `Env::seen`, under the caps every search assumes; the found careers are the search's
/// answer, so the anchors depend on the job alone)
///     value(y, her) <= c' val_f rise(q', q_f);
/// and, for the partners behind the band's likeliest outcomes (y < 0.5 <= the bound so far; `HI_SEARCHES`, `HI_RISE`),
/// over her best deck searched at her s = 2, 3 stars on every uncertain target (q^s, `best_deck` fully checked: E_best
/// there, under the caps every search assumes)
///     value(y, her) <= c' E_best(q^s) rise(q', q^s),
/// rise 1 where her stars are at most s (the band's outcomes are bounded again after); also at most the ceiling
/// (`Env::ceiling` through `Sim::fastc`, under EXACT_RHO: per outfit and target A - B (1 - q'), `outfit_fast`'s factor at
/// q'; before 2026-10-09 the bound took A + B q', B more per target, valid and on p16 3.3 times as high at the pick's
/// partners). A candidate's step costs the lowest cost at those odds
/// (`Cont::cost_lo`). Derived, per outcome:
/// (1) For each partner, the bound >= the best career's exact value >= the table value `score_career` reads. A cached
///     deck's corner table is multilinear in the targets' miss chances, as P(all) of a fixed deck is (each chance enters
///     one target's factor linearly), so its interpolation is that deck's exact value; a real deck is at most the best.
///     Her best partners are among every partner. So up_hi >= up for every candidate.
/// (2) cost_lo(up_hi) <= cost(up) (`Cont::cost_lo`), a step cost never rises from the base, and V over the trios (and
///     `with_her`) does not fall when a step cost rises; so V_lo <= V_table and y_hi >= y.
/// (3) Each anchor A (a pair (y, z), a found career, a search at q^s) has a value E_A with E_best(q_A) <= E_A (a career
///     of today's pool is worth at most pnow; a fully checked deck search is the best under the caps). Every deck D legal
///     for the candidate has E_D(q') <= E_D(q_A) rise(q', q_A) (`Env::rise`) <= E_A rise(q', q_A), so her best career
///     with partner y is at most c' E_A rise(q', q_A) for every A, and at most its ceiling. In the exact replan with her
///     in the pool, with today's Cont and trios, her step then costs at least cost_lo(up_hi): y_hi bounds the direct-lift
///     saving. Not bounded (channel 2, out of scope by the owner's decision): what the replan's refits do (Cont fitted
///     again, other careers' best careers with her as a parent, the stage candidates chosen again beyond her own 8/8).
#[allow(clippy::too_many_arguments)]
fn outcome_hi(s: &mut Sim, d: &Data, bs: &ScoreBase, legs: &[Legacy], pool: &Pool, fix: &[usize], msv: &Solved, ytab: &[f64], x: usize, a: usize, b: usize, sel: &Sel, buf: &mut Vec<f64>) -> (Vec<f64>, Vec<f64>) {
    let n = bs.cands.len(); let ntar = s.env.ntar; let nu = s.env.uncv.len();
    let uncv = s.env.uncv.clone(); let fixed_t = s.env.fixed_t.clone(); let w = s.env.sj.star_dist;
    let xi = bs.cands.iter().position(|&u| u == x);
    let goal = is_full(s.env.sj, &legs[a].sparks) && is_full(s.env.sj, &legs[b].sparks);
    let c = s.make_cur(pool, x, a, b, sel, false);
    let mut lv = legs.to_vec(); lv.push(s.env.veteran(legs, x, a, b, c.wp.to_vec())); let iv = lv.len() - 1;
    let pv = s.env.pool(&lv);
    // per candidate and EVERY legal partner: her slot's miss factors and the spark scale, as `score_career`'s setups
    let nleaf = 1usize << (2 * nu); let nm = 1usize << ntar;
    let mut hs: Vec<(usize, Vec<f64>, Vec<f64>)> = vec![];   // (candidate, bound at scale 1 per leaf, spark scale per m)
    let mut hq: Vec<(usize, Vec<f64>, Vec<f64>, [f64; 4])> = vec![];   // per partner: (trainee, q0, x0, xms), for the anchors
    let rf = s.env.space.rho_top() * s.fixed;
    let (mut lp, mut lp_tmp): (Vec<f64>, Vec<f64>) = (vec![], vec![]);
    for ui in 0..n {
        let u = bs.cands[ui];
        if d.umas[u].id == d.umas[x].id || bs.c1[ui] == 0.0 || s.fastc[u].is_empty() { continue; }
        // her outfits' skill coverage and own chances (as `Env::rise` reads them)
        let (sts, ews) = (s.env.sts0[u].clone(), s.env.ew0[u].clone());
        // the main search's found careers of her deck class that it fully checked (a Ref at the career's own chances):
        // (q_f, val_f), the best value of any deck at q_f for every trainee of the class (`Env::seen`), scale 1
        let ct = s.env.cls[u];
        let fr: Vec<(Vec<f64>, f64)> = msv.found.iter().filter(|f| s.env.cls[f.t] == ct).filter_map(|f| {
            let (fq, _, _) = s.env.lineage(pool, f.t, f.a, f.b);
            msv.refs.get(f.t)?.iter().find(|r| r.q == fq).map(|r| (r.q.clone(), r.val)) }).collect();
        for y in (0..iv).filter(|&y| legal_pair(d, &pv, u, y, iv)) {
            let sl = s.env.slots(&pv, u, y, iv);
            let Some(pos) = sl.iter().position(|z| z.parent && z.legacy == iv) else { continue };
            let a_v = sl[pos].rel + sl[pos].race;
            let mut sl0 = sl.clone(); { let s = std::rc::Rc::make_mut(&mut sl0[pos].stars); for k in 0..ntar { s[k] = 0; } } sl0[pos].mix = None;
            let (q0, nc0) = inherit(&sl0, ntar);
            let xm = |st: u8| { let m = 1.0 - (0.03 * st as f64 * (1.0 + a_v / 100.0)).min(1.0); m * m };
            let xms = [1.0, xm(1), xm(2), xm(3)];
            let x0: Vec<f64> = uncv.iter().map(|&i| 1.0 - q0[i]).collect();
            // the best outfit's product per leaf: per outfit and target the factor A - B (1 - q') at each of her four star
            // levels, 1 - q' = x0 xm (`outfit_fast` at her chances folded in, as `Sim::fast_ub` takes it)
            let mut ceil = vec![0.0f64; nleaf];
            for o in &s.fastc[u] {
                let g: Vec<[f64; 4]> = (0..nu).map(|k| { let (aa, bb) = o[k]; [0, 1, 2, 3].map(|sg| aa - bb * (x0[k] * xms[sg])) }).collect();
                leaf_products(&g, &mut lp, &mut lp_tmp);
                for (cv, &pr) in ceil.iter_mut().zip(&lp) { if pr > *cv { *cv = pr; } }
            }
            // the anchored rise bound per leaf: the least over today's pairs (y, z) keeping the places of
            // (pnow / c_yz) rise(q'(leaf), q_yz), q'_k(leaf) = 1 - x0[k] xm[her stars]
            let mut anc = vec![f64::INFINITY; nleaf];
            if bs.pnow[ui] > 0.0 {
                for z in (0..legs.len()).filter(|&z| z != y && legal_pair(d, pool, u, y, z) && fix.iter().all(|&f| f == y || f == z)) {
                    let (qz, _, cz) = s.env.lineage(pool, u, y, z);
                    if cz <= 0.0 { continue; }
                    let rz = rise_leaves(&sts, &ews, &x0, &xms, &qz, &uncv, &mut lp, &mut lp_tmp);
                    let f = bs.pnow[ui] / cz;
                    for (av, rv) in anc.iter_mut().zip(&rz) { let v = f * rv; if v < *av { *av = v; } }
                }
            }
            // and over the main search's proven careers of her deck class: E_best(q_f) = val_f there
            for (fq, fv) in &fr {
                let rz = rise_leaves(&sts, &ews, &x0, &xms, fq, &uncv, &mut lp, &mut lp_tmp);
                for (av, rv) in anc.iter_mut().zip(&rz) { let v = fv * rv; if v < *av { *av = v; } }
            }
            // at most the ceiling too (under EXACT_RHO)
            for (cv, av) in ceil.iter_mut().zip(&anc) { *cv = (rf * *cv).min(*av); }
            let pw: Vec<[f64; 2]> = (0..ntar).map(|k| [1.1f64.powi(nc0[k] - 6), 1.1f64.powi(nc0[k] + 1 - 6)]).collect();
            let sc: Vec<f64> = (0..nm).map(|m| (0..ntar).map(|k| pw[k][(m >> k & 1) as usize]).product()).collect();
            hs.push((ui, ceil, sc)); hq.push((u, q0, x0, xms));
        }
    }
    let full_m = nm - 1;
    let her: Vec<(usize, bool, f64)> = if xi.is_none() && !goal && !bs.sides.is_empty() && c.os[full_m] > 0.0 { her_trios(s, d, bs, &lv, iv, x, buf) } else { vec![] };
    let nf = fixed_t.len();
    let mut ys: Vec<f64> = vec![0.0; nleaf << nf]; let mut prs: Vec<f64> = vec![0.0; nleaf << nf]; let mut ms: Vec<usize> = vec![0; nleaf << nf];
    let mut c1 = bs.c1.clone(); let mut up = vec![0.0f64; n];
    for leaf in 0..nleaf {
        let mut mu = 0usize; let mut pr_st = 1.0f64;
        for k in 0..nu { let sg = (leaf >> (2 * k)) & 3; if sg > 0 { mu |= 1 << uncv[k]; pr_st *= w[sg - 1]; } }
        for fm in 0..(1usize << nf) {
            let mut m = mu; for (j, &i) in fixed_t.iter().enumerate() { if fm >> j & 1 == 1 { m |= 1 << i; } }
            let pr = c.os[m] * pr_st;
            if pr <= 0.0 { continue; }
            prs[leaf << nf | fm] = pr; ms[leaf << nf | fm] = m;
            ys[leaf << nf | fm] = hi_at(bs, &hs, leaf, m, full_m, xi, &her, goal, &mut c1, &mut up);
        }
    }
    // exact anchors (`HI_SEARCHES`) for the partners that hold the most likely outcomes of the band (y < 0.5 <= y_hi) in
    // it: per band outcome, most likely first, the partners that must all fall for it to leave the band (every partner
    // above p_now of each candidate whose odds move V; none further than HI_RISE above p_now), taken whole while they fit
    let mut band: Vec<usize> = (0..ys.len()).filter(|&i| prs[i] > 0.0 && ytab[i] < 0.5 && ys[i].max(ytab[i]) >= 0.5).collect();
    band.sort_by(|&i, &j| prs[j].partial_cmp(&prs[i]).unwrap().then(i.cmp(&j)));
    let cap = HI_SEARCHES / HI_STARS.len();
    let mut chosen: Vec<usize> = vec![];
    for &i in &band {
        if chosen.len() >= cap { break; }
        let (leaf, m) = (i >> nf, ms[i]);
        let h = hi_at(bs, &hs, leaf, m, full_m, xi, &her, goal, &mut c1, &mut up);
        let (mut need, mut far): (Vec<usize>, bool) = (vec![], false);
        for ui in 0..n {
            if up[ui] <= bs.pnow[ui] * (1.0 + 1e-9) { continue; }
            let keep = up[ui]; up[ui] = bs.pnow[ui];
            let h2 = saving_lo(bs, &up, m, full_m, xi, &her, goal, &mut c1); up[ui] = keep;
            if h2 >= h - 1e-9 { continue; }
            for (pi, (pu, bd, sc)) in hs.iter().enumerate() {
                if *pu != ui { continue; }
                let v = sc[m] * bd[leaf];
                if v > bs.pnow[ui] * (1.0 + 1e-9) { need.push(pi); if v > HI_RISE * bs.pnow[ui] { far = true; } }
            }
        }
        if far || need.is_empty() { continue; }
        need.retain(|pi| !chosen.contains(pi));
        if chosen.len() + need.len() <= cap { chosen.extend(need); }
    }
    // each one: her best deck searched at her s stars on every uncertain target (q^s, `HI_STARS`), proven (`best_deck`
    // full, under the caps): value(y, her) <= c' E_best(q^s) rise(q', q^s) for every outcome (rise 1 where her stars <= s)
    for &pi in &chosen {
        let (u, q0, x0, xms) = &hq[pi];
        let (sts, ews) = (s.env.sts0[*u].clone(), s.env.ew0[*u].clone());
        for &st in &HI_STARS {
            let mut q = q0.clone(); for k in 0..nu { q[uncv[k]] = 1.0 - x0[k] * xms[st]; }
            s.env.set_trainee(*u, &q); s.env.work = 0; let mut nsc = 0;
            if let DeckRes::Done(v, _, true) = s.env.best_deck(*u, 1.0, 0.0, &[], &mut nsc) {
                let rz = rise_leaves(&sts, &ews, x0, xms, &q, &uncv, &mut lp, &mut lp_tmp);
                for (bv, r) in hs[pi].1.iter_mut().zip(&rz) { let v2 = v * r; if v2 < *bv { *bv = v2; } }
            }
        }
    }
    if !chosen.is_empty() { for &i in &band { ys[i] = hi_at(bs, &hs, i >> nf, ms[i], full_m, xi, &her, goal, &mut c1, &mut up); } }
    (ys, prs)
}

/// The page's next career, exact where it can be and without simulation: the career that brings a FULLY LOOPED veteran
/// (an 8/8 whose parents are 8/8s) the closest. V = the cheapest trio: A and B two stage candidates, each step "build
/// while it pays, then run her" (`Cont`, fitted to the pool now; free when an 8/8 of hers exists), C the best allowed
/// uma with A's and B's 8/8s as parents (1 / her full-loop odds). A career is scored E[V - V with her veteran] - 1, exact
/// over her outcomes: her veteran as a parent raising any candidate's odds lowers that step, her own 8/8 ends her own
/// step; a roll that changes nothing leaves every score as it was (repeat). Two deep for the 3 highest and the best
/// odds: her rolls one target short, as a parent of the 3 highest builders, over today's best score. Pick: the highest;
/// within NEAR, the better own odds. The goal is always the next fully looped parent, any uma: one you have already is a
/// real 8/8 (free) with 8/8 parents, a strong side of the trios.
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn index_rule<'a>(env: &mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, legs: &[Legacy],
                                       pool: &Pool, sv: &Solved, fix: &[usize], trainees: &[usize], pj: &PlanJob) -> Value {
    let t_all = now_ms();
    // (first in, dropped last: after every memo of this plan)
    let _memo_report = MemoReport;
    // the work counters at the start (deck-search nodes, exact scorings): reported as this plan's own
    let (nodes0, joints0) = (env.space.nodes.get(), crate::deck::JCOUNT.with(|c| c.get()));
    // the loop planner's members only when the job names them (`loop_members`, an override; the plan no longer runs the
    // loop planner's search itself: on the val/plan pools its members always entered the candidates by the routes below)
    let (members, minfo, mwhy) = if pj.loop_members.is_some() { rotation(env, d, sk, cr, job, pj) } else { (vec![], vec![], None) };
    let g = sv.found[0].clone();
    // stage-1 candidates (who A can be; whose odds a builder may lift): the best career's trainee, the named loop members,
    // the umas of the best career's lineage, the umas you have veterans of, the search's other trainees; at most IDX_CANDS
    let mut cands: Vec<usize> = vec![g.t];
    let push = |u: usize, c: &mut Vec<usize>| if !c.contains(&u) && trainees.contains(&u) { c.push(u) };
    for &m in &members { push(m, &mut cands); }
    for x in [g.a, g.b] { for c in std::iter::once(legs[x].chara).chain(legs[x].parents.iter().map(|p| p.chara)) { if let Some(u) = env.uix(c) { push(u, &mut cands); } } }
    for l in legs.iter().rev() { if !l.borrowed { if let Some(u) = env.uix(l.chara) { push(u, &mut cands); } } }
    for f in &sv.found { push(f.t, &mut cands); }
    cands.truncate(IDX_CANDS);
    if let Some(f) = &pj.fit { cands = f.iter().filter_map(|&(c, ..)| env.uix(c)).filter(|u| trainees.contains(u)).collect(); }
    // a pool that gives fewer than HINT_FLOOR candidates (a loop being started: veterans of umas that won't be in it, mob
    // parents) is filled from the planner's own model: the owned umas ranked by their odds with a full loop lineage
    // (`hint_umas`), no deck search
    let mut hint: Vec<Value> = vec![];
    if pj.loop_members.is_none() && pj.fit.is_none() && cands.len() < HINT_FLOOR {
        for (u, v) in hint_umas(env, d, legs, &cands, trainees, HINT_FLOOR - cands.len()) {
            cands.push(u); hint.push(json!({"id": d.umas[u].id, "name": d.umas[u].name, "ceiling": v}));
        }
    }
    let loop_ms = now_ms() - t_all;
    let all: Vec<usize> = trainees.iter().copied().filter(|&u| env.dmax[u].is_some()).collect();
    let t1 = now_ms();
    let budget0 = env.budget;
    if pj.search_work > 0 { env.budget = pj.search_work; env.hard = pj.search_work; }
    env.use_seen = true;
    // best careers, searched on demand (stage-1 candidates now, builders when scored exactly)
    let mut best: Vec<Option<Option<Found>>> = vec![None; d.umas.len()];
    best[g.t] = Some(Some(g.clone()));
    // a kept plan of this pool less its last veteran (`Bests`), and this plan's own, kept at the end
    let nl = legs.len();
    // (taken out while this plan runs and put back after it: every reroll of a career reuses the same one)
    let same: Option<Bests> = { let k = bests_key(job, fix, nl); BESTS.with(|c| { let mut c = c.borrow_mut(); let i = c.iter().position(|b| b.key == k); i.map(|i| c.remove(i)) }) };
    let prev: Option<Bests> = if nl > 1 { let k = bests_key(job, fix, nl - 1); BESTS.with(|c| { let mut c = c.borrow_mut(); let i = c.iter().position(|b| b.key == k); i.map(|i| c.remove(i)) }) } else { None };
    let mut urefs: Vec<Vec<Vec<Ref>>> = vec![vec![]; d.umas.len()];
    if pj.sur != 1 { for &u in &cands { if best[u].is_none() { let (f, r) = best_of(env, pool, fix, u, same.as_ref(), prev.as_ref(), nl); urefs[u] = r; best[u] = Some(f); } } }
    let mut search_ms = now_ms() - t1;
    let mut s = match sim_for(env, d, sk, cr, job, pj) { Ok(s) => s, Err(e) => return e };
    // the cache the planner's table values read (trios' third members, candidates' lifted odds): each search's exact
    // winners only, the main search's best careers and each uma's best one, so what it holds (and every number read from
    // it) depends on the job alone, not on what the pruning happened to check fully (earlier jobs' proofs, the order, a
    // kept plan). Before, every fully checked deck went in: a candidate then had several decks, and her lifted odds
    // read the best of them; with one the planner's numbers moved once (val/plan: picks unchanged)
    for f in &sv.found { s.cache.add(s.env, f.t, &f.sel); }
    for &u in &cands { if let Some(Some(f)) = &best[u] { s.cache.add(s.env, u, &f.sel); } }
    // (`proxy_check`: the cache as it is before any builder's search, for the proxies at the end)
    let cache0 = if pj.proxy_check { Some(s.cache.clone()) } else { None };
    let mut buf = vec![];
    // the surrogate's best careers (sur 1: in place of the searches; 2: beside them, differences reported)
    let mut sur_diff: Vec<Value> = vec![];
    let sur_one = |s: &mut Sim, best: &mut Vec<Option<Option<Found>>>, u: usize, buf: &mut Vec<f64>, sur_diff: &mut Vec<Value>| {
        let f = best_sur(s, d, pool, fix, u, pj.sur_margin, buf);
        if pj.sur == 2 {
            let e = best[u].clone().flatten();
            let (ve, vs) = (e.as_ref().map_or(0.0, |f| f.val), f.as_ref().map_or(0.0, |f| f.val));
            let same = e.as_ref().map(|f| (f.a.min(f.b), f.a.max(f.b))) == f.as_ref().map(|f| (f.a.min(f.b), f.a.max(f.b)));
            if !same || (ve - vs).abs() > 1e-9 * ve.max(vs) {
                sur_diff.push(json!({"trainee": d.umas[u].name, "search": [e.as_ref().map(|f| [f.a, f.b]), ve], "sur": [f.as_ref().map(|f| [f.a, f.b]), vs]}));
            }
        } else { best[u] = Some(f); }
    };
    if pj.sur > 0 {
        let ts = now_ms();
        if pj.search_work > 0 { s.env.budget = pj.search_work; s.env.hard = pj.search_work; }
        for &u in &cands { if pj.sur == 2 || best[u].is_none() { sur_one(&mut s, &mut best, u, &mut buf, &mut sur_diff); } }
        s.env.hard = 0; s.env.budget = budget0;
        search_ms += now_ms() - ts;
    }
    let fb: Vec<Option<Found>> = cands.iter().map(|&u| best[u].clone().flatten()).collect();
    let ntar = s.env.ntar; let n = cands.len();
    let pnow: Vec<f64> = fb.iter().map(|f| f.as_ref().map_or(0.0, |f| f.val)).collect();
    let sj = s.env.sj;
    // per candidate whether an 8/8 of hers exists (a free side of the trios)
    let have8: Vec<bool> = cands.iter().map(|&u| legs.iter().any(|l| l.chara == d.umas[u].id && is_full(sj, &l.sparks))).collect();
    let zeros = vec![0.0f64; n];
    let mut exact_n = 0usize;
    let t3 = now_ms();
    // Builders through their deck class (owner decision, 2026-10-08): a builder's career is first taken from her class's
    // winning decks (the main search's and the candidates' best careers of her class: legal for her, the same deck
    // problem, valued at her own chances through the surrogate, `best_sur`), no career search. Exact searches only where
    // the plan can turn on her: a lift efficiency within PROXY_EFF of a candidate's second best (Cont's two builders come
    // from the exactly searched careers), or a saving within PROXY_SAVE of the shown rows (the page's top OPT_SHOW), the
    // pick's near-tie band and two deep's builders. Measured on val/plan (`proxy_check`): the proxy's odds only ever read
    // low (it is a career she can run); its efficiency on contenders for a candidate's top two at most 2.8% low (p11; it
    // can also read high, which only searches more), its saving at most 2.7% low; exact on the other five jobs.
    // `exact_all` restores a search for every uma.
    let proxy_on = pj.sur == 0 && !pj.exact_all && !pj.proxy_check && !pj.value_only;
    let wins: Vec<(usize, Sel)> = sv.found.iter().map(|f| (f.t, f.sel.clone()))
        .chain(cands.iter().filter_map(|&u| best[u].clone().flatten().map(|f| (u, f.sel)))).collect();
    // pass 1: every allowed uma's best career; per candidate, the chance her veteran raises that candidate's odds and the
    // mean odds raised to (exact over her outcomes)
    let mut runs: Vec<(usize, Found, Vec<(f64, f64)>)> = vec![];
    // per run: an exact best career (else her class's proxy)
    let mut exact_run: Vec<bool> = vec![];
    // admitted proxies the class's proofs certified (`certified`), so not searched
    let mut n_cert = 0usize;
    let mut wtab: Vec<Vec<usize>> = vec![vec![]; d.umas.len()];
    // (placeholder step costs: 1, so every candidate's lifts are gathered; an 8/8 of hers may be a poorer parent than a
    // fresh one)
    let base_s = ScoreBase { cands: &cands, c1: vec![1.0; n], m23: &zeros, v0: 0.0, pnow: pnow.clone(), cont: vec![], combos: vec![], sides: &[], all: &[] };
    // uma x's exact best career (searched now if not yet) and its lifts
    let exact_one = |s: &mut Sim, best: &mut Vec<Option<Option<Found>>>, urefs: &mut Vec<Vec<Vec<Ref>>>, x: usize, buf: &mut Vec<f64>| -> Option<(Found, Vec<(f64, f64)>)> {
        if best[x].is_none() {
            if pj.search_work > 0 { s.env.budget = pj.search_work; s.env.hard = pj.search_work; }
            let (f, r) = best_of(s.env, pool, fix, x, same.as_ref(), prev.as_ref(), nl);
            if let Some(f) = &f { s.cache.add(s.env, x, &f.sel); }
            urefs[x] = r; best[x] = Some(f);
            s.env.hard = 0; s.env.budget = budget0;
        }
        let f = best[x].clone().flatten()?;
        let (_, _, _, st, _) = score_career(s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, buf);
        Some((f, st))
    };
    let set_wins = |s: &mut Sim, wtab: &mut Vec<Vec<usize>>, x: usize, f: &Found| { let w = s.wins(pool, x, f.a, f.b); wtab[x] = w.iter().enumerate().filter(|x| *x.1 > 0.5).map(|x| x.0).collect(); };
    if !pj.value_only {
        let mut order: Vec<usize> = vec![g.t]; for &x in &all { if x != g.t { order.push(x); } }
        for &x in &order {
            if proxy_on && best[x].is_none() {
                let ts = now_ms();
                let cx = s.env.cls[x];
                for (t, sel) in &wins { if *t != x && s.env.cls[*t] == cx { s.cache.add(s.env, x, sel); } }
                let pf = best_sur(&mut s, d, pool, fix, x, 1e9, &mut buf);
                search_ms += now_ms() - ts;
                let Some(f) = pf else { continue };
                let (_, _, _, st, _) = score_career(&mut s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, &mut buf);
                runs.push((x, f, st)); exact_run.push(false);
                continue;
            }
            if best[x].is_none() || (pj.sur == 2 && !cands.contains(&x)) {
                let ts = now_ms();
                if pj.search_work > 0 { s.env.budget = pj.search_work; s.env.hard = pj.search_work; }
                if pj.sur != 1 && best[x].is_none() {
                    let (f, r) = best_of(s.env, pool, fix, x, same.as_ref(), prev.as_ref(), nl);
                    if let Some(f) = &f { s.cache.add(s.env, x, &f.sel); }
                    urefs[x] = r; best[x] = Some(f);
                }
                if pj.sur > 0 { sur_one(&mut s, &mut best, x, &mut buf, &mut sur_diff); }
                s.env.hard = 0; s.env.budget = budget0;
                search_ms += now_ms() - ts;
            }
            let Some(Some(f)) = best[x].clone() else { continue };
            let (_, _, _, st, _) = score_career(&mut s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, &mut buf);
            runs.push((x, f, st)); exact_run.push(true);
        }
        // the near-top builders searched exactly: per candidate every proxy whose efficiency reaches (1 - PROXY_EFF) of the
        // candidate's second best (exact and proxy careers together), until none is left
        while proxy_on {
            let mut add: Vec<usize> = vec![];
            for ui in 0..n {
                if pnow[ui] <= 0.0 { continue; }
                let (u, p) = (cands[ui], pnow[ui]);
                let mut e: Vec<(f64, usize)> = runs.iter().enumerate().filter(|(_, r)| d.umas[r.0].id != d.umas[u].id && r.2[ui].0 > 0.0)
                    .map(|(i, r)| { let (pl, sp) = r.2[ui]; (pl * (1.0 / p - 1.0 / (sp / pl)), i) }).collect();
                e.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
                let Some(&(e1, _)) = e.first() else { continue };
                let second = e.get(1).map_or(e1, |x| x.0);
                for &(v, i) in &e { if v < second * (1.0 - pj.proxy_eff.unwrap_or(PROXY_EFF)) { break; } if !exact_run[i] && !add.contains(&i) { add.push(i); } }
            }
            if add.is_empty() { break; }
            add.sort();
            let ts = now_ms();
            for i in add {
                let x = runs[i].0;
                // a proxy the class's proofs certify needs no search (it is her best career's value)
                if certified(s.env, d, pool, fix, x, &runs[i].1) { best[x] = Some(Some(runs[i].1.clone())); exact_run[i] = true; n_cert += 1; continue; }
                if let Some((f, st)) = exact_one(&mut s, &mut best, &mut urefs, x, &mut buf) { runs[i] = (x, f, st); } exact_run[i] = true;
            }
            search_ms += now_ms() - ts;
        }
        for r in &runs { set_wins(&mut s, &mut wtab, r.0, &r.1); }
    }
    // each candidate's step with building: her most efficient builder now (rate, log-odds step), her full-loop ceiling with
    // her two most efficient builders (among the exactly searched careers)
    let cont_of = |s: &mut Sim, runs: &[(usize, Found, Vec<(f64, f64)>)], exact: &[bool], wtab: &[Vec<usize>], buf: &mut Vec<f64>| -> Vec<Cont> {
        let wt = |x: usize| wtab[x].clone();
        let mut cont = vec![Cont::default(); n];
        for ui in 0..n {
            if pnow[ui] <= 0.0 || runs.is_empty() { continue; }
            let u = cands[ui]; let p = pnow[ui];
            let mut eff: Vec<(f64, usize, f64, f64)> = runs.iter().zip(exact).filter(|(r, ex)| **ex && d.umas[r.0].id != d.umas[u].id && r.2[ui].0 > 0.0)
                .map(|(r, _)| { let (pl, sp) = r.2[ui]; let p1 = sp / pl; (pl * (1.0 / p - 1.0 / p1), r.0, pl, p1) }).collect();
            eff.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
            let Some(&(_, b1, lam, p1)) = eff.first() else { continue };
            let b2 = eff.iter().skip(1).map(|e| e.1).find(|&b| d.umas[b].id != d.umas[b1].id);
            let pinf = match b2 { Some(b2) => (CEIL_SHARE * loop_ceiling(s, d, legs, u, b1, b2, &wt, buf)).max(p1), None => p1 };
            let phi = if pinf > p && p1 > p { ((p1 / p).ln() / (pinf / p).ln()).clamp(0.0, 1.0) } else { 0.0 };
            cont[ui] = Cont::new(lam, phi, pinf);
        }
        // a given fit replaces the pool's (`PlanJob::fit`)
        if let Some(f) = &pj.fit { for ui in 0..n { if let Some(&(_, lam, phi, pinf)) = f.iter().find(|x| d.umas[cands[ui]].id == x.0) { cont[ui] = Cont::new(lam, phi, pinf); } } }
        cont
    };
    let mut cont = cont_of(&mut s, &runs, &exact_run, &wtab, &mut buf);
    // a fresh 8/8 of hers (her step); a real 8/8 of hers costs nothing (the trios' side choice)
    let mut c1: Vec<f64> = (0..n).map(|ui| cont[ui].cost(pnow[ui])).collect();
    // the trios: A and B two candidates (unordered), C the best allowed uma with an 8/8 of A and one of B as parents. Each
    // side: a real 8/8 veteran of hers (free), or a fresh one, her step, the 8/8 her best career now would leave
    // (`Env::veteran`: its parents, every target at the rank's star mix, her G1 wins at their chances), whichever trio is
    // cheaper; C's 8/8 is then a fully looped parent
    let t_c = now_ms();
    let real8: Vec<Vec<usize>> = cands.iter().map(|&u| (0..legs.len()).filter(|&i| legs[i].chara == d.umas[u].id && is_full(sj, &legs[i].sparks)).collect()).collect();
    // (A, B, C's step, C, A real, B real)
    let combos_of = |s: &mut Sim, c1: &[f64], buf: &mut Vec<f64>| -> Vec<(usize, usize, f64, usize, bool, bool)> {
        let hyp8: Vec<Option<Legacy>> = (0..n).map(|ui| if !c1[ui].is_finite() { None } else {
            fb[ui].as_ref().map(|f| { let wp = s.wins(pool, cands[ui], f.a, f.b); s.env.veteran(legs, cands[ui], f.a, f.b, wp.to_vec()) }) }).collect();
        let mut combos: Vec<(usize, usize, f64, usize, bool, bool)> = vec![];
        for ia in 0..n { for ib in ia + 1..n {
            let (ua, ub) = (cands[ia], cands[ib]);
            if d.umas[ua].id == d.umas[ub].id { continue; }
            let mut l = legs.to_vec();
            // per side: (real, legacy index)
            let mut side = |ui: usize| -> Vec<(bool, usize)> { let mut v: Vec<(bool, usize)> = real8[ui].iter().map(|&i| (true, i)).collect();
                if let Some(h) = &hyp8[ui] { l.push(h.clone()); v.push((false, l.len() - 1)); } v };
            let (xs, ys) = (side(ia), side(ib));
            let p8 = s.env.pool(&l);
            // per side choice (fresh/real for A, fresh/real for B): the best C
            let mut bc = [(0.0f64, usize::MAX); 4];
            for &c in &all { if d.umas[c].id == d.umas[ua].id || d.umas[c].id == d.umas[ub].id { continue; }
                for &(ra, x) in &xs { for &(rb, y) in &ys { if legal_pair(d, &p8, c, x, y) {
                    let k = (ra as usize) * 2 + rb as usize;
                    let v = val(s, &p8, c, x, y, buf); if v > bc[k].0 { bc[k] = (v, c); } } } } }
            for (k, &(v, cu)) in bc.iter().enumerate() { if cu != usize::MAX { combos.push((ia, ib, 1.0 / v, cu, k >= 2, k % 2 == 1)); } }
        } }
        combos
    };
    // the trios' sides per candidate (her real 8/8s, then the fresh 8/8 her best career now would leave), for `her_trios`
    let sides_of = |s: &mut Sim, c1: &[f64]| -> Vec<Vec<(bool, Legacy)>> { (0..n).map(|ui| {
        let mut v: Vec<(bool, Legacy)> = real8[ui].iter().map(|&i| (true, legs[i].clone())).collect();
        if c1[ui].is_finite() { if let Some(f) = fb[ui].as_ref() { let wp = s.wins(pool, cands[ui], f.a, f.b); v.push((false, s.env.veteran(legs, cands[ui], f.a, f.b, wp.to_vec()))); } }
        v }).collect() };
    let mut combos = combos_of(&mut s, &c1, &mut buf);
    let mut sides = sides_of(&mut s, &c1);
    let mut combo_ms = now_ms() - t_c;
    let mut combos3: Vec<Trio> = combos.iter().map(|c| (c.0, c.1, c.2, c.4, c.5)).collect();
    let mut vnow = trio_v(&c1, &combos3);
    // pass 2: every career scored against V (exact over her outcomes): E[V - V with her veteran] - 1
    let score_one = |s: &mut Sim, r: &(usize, Found, Vec<(f64, f64)>), c1: &[f64], vnow: f64, cont: &[Cont], combos3: &[Trio], sides: &[Vec<(bool, Legacy)>], buf: &mut Vec<f64>| {
        let base = ScoreBase { cands: &cands, c1: c1.to_vec(), m23: &zeros, v0: vnow, pnow: pnow.clone(), cont: cont.to_vec(), combos: combos3.to_vec(), sides, all: &all };
        let (ey, lift, ys, _, _) = score_career(s, d, &base, legs, pool, r.0, r.1.a, r.1.b, &r.1.sel, buf);
        (ey - 1.0, lift, ys)
    };
    let mut score: Vec<f64> = vec![]; let mut lifts: Vec<f64> = vec![]; let mut ytabs: Vec<Vec<f64>> = vec![];
    if !runs.is_empty() && vnow.is_finite() {
        for r in &runs { let (sc, lift, ys) = score_one(&mut s, r, &c1, vnow, &cont, &combos3, &sides, &mut buf); score.push(sc); lifts.push(lift); ytabs.push(ys); }
        // the proxies a shown row, the pick's near-tie band or two deep's builders can turn on: searched exactly (their
        // saving, score + 1, within PROXY_SAVE of the lowest saving those take), then scored again; when one of them joins a
        // candidate's two most efficient builders, Cont, the trios, V and every score are computed again
        while proxy_on {
            let mut sv: Vec<f64> = score.clone(); sv.sort_by(|x, y| y.partial_cmp(x).unwrap());
            let (top, k8, k3) = (sv[0], sv[(OPT_SHOW - 1).min(sv.len() - 1)], sv[(TWO_BUILDERS - 1).min(sv.len() - 1)]);
            let t = k8.min(k3).min(top - NEAR.max(0.05 * top.abs())) + 1.0;
            let add: Vec<usize> = (0..runs.len()).filter(|&i| !exact_run[i] && score[i] + 1.0 >= t * (1.0 - PROXY_SAVE)).collect();
            if add.is_empty() { break; }
            let ts = now_ms();
            for &i in &add {
                let x = runs[i].0;
                if certified(s.env, d, pool, fix, x, &runs[i].1) { best[x] = Some(Some(runs[i].1.clone())); exact_run[i] = true; n_cert += 1; continue; }
                if let Some((f, st)) = exact_one(&mut s, &mut best, &mut urefs, x, &mut buf) { set_wins(&mut s, &mut wtab, x, &f); runs[i] = (x, f, st); } exact_run[i] = true;
            }
            search_ms += now_ms() - ts;
            let cont2 = cont_of(&mut s, &runs, &exact_run, &wtab, &mut buf);
            let same = cont2.iter().zip(&cont).all(|(a, b)| a.lam == b.lam && a.phi == b.phi && a.pinf == b.pinf);
            if same {
                for &i in &add { let (sc, lift, ys) = score_one(&mut s, &runs[i], &c1, vnow, &cont, &combos3, &sides, &mut buf); score[i] = sc; lifts[i] = lift; ytabs[i] = ys; }
            } else {
                memo_stat(6);
                cont = cont2;
                c1 = (0..n).map(|ui| cont[ui].cost(pnow[ui])).collect();
                let tc = now_ms(); combos = combos_of(&mut s, &c1, &mut buf); sides = sides_of(&mut s, &c1); combo_ms += now_ms() - tc;
                combos3 = combos.iter().map(|c| (c.0, c.1, c.2, c.4, c.5)).collect();
                vnow = trio_v(&c1, &combos3);
                if !vnow.is_finite() { score.clear(); lifts.clear(); ytabs.clear(); break; }
                for i in 0..runs.len() { let (sc, lift, ys) = score_one(&mut s, &runs[i], &c1, vnow, &cont, &combos3, &sides, &mut buf); score[i] = sc; lifts[i] = lift; ytabs[i] = ys; }
            }
        }
        exact_n = score.len();
    }
    let (c1, cont, combos, combos3, vnow, sides) = (c1, cont, combos, combos3, vnow, sides);
    let vof = |c: &[f64]| trio_v(c, &combos3);
    let step = |ui: usize, real: bool| if real { 0.0 } else { c1[ui] };
    let best_combo = combos.iter().min_by(|x, y| (step(x.0, x.4) + step(x.1, x.5) + x.2).partial_cmp(&(step(y.0, y.4) + step(y.1, y.5) + y.2)).unwrap()).copied();
    let index_ms = now_ms() - t3;
    // two deep, in the same units: her strong rolls (at most TWO_MISS targets short of 8/8, stars at the rank's mix) as a
    // parent of the TWO_BUILDERS highest builders; the next career's gain over today's best score
    let t4 = now_ms();
    let mut two: Vec<Option<f64>> = vec![None; runs.len()];
    if !pj.value_only && !score.is_empty() {
        let istar = score.iter().cloned().fold(0.0f64, f64::max);
        let mut ord: Vec<usize> = (0..runs.len()).collect();
        ord.sort_by(|&p, &q| score[q].partial_cmp(&score[p]).unwrap());
        let builders: Vec<usize> = ord.iter().take(TWO_BUILDERS).map(|&i| runs[i].0).collect();
        let mut who: Vec<usize> = ord.iter().take(TWO_TOP).copied().collect();
        if let Some(gi) = runs.iter().position(|r| r.0 == g.t) { if !who.contains(&gi) { who.push(gi); } }
        let full = (1usize << ntar) - 1;
        for &wi in &who {
            let (x, f) = (runs[wi].0, runs[wi].1.clone());
            let c = s.make_cur(pool, x, f.a, f.b, &f.sel, false);
            let mut gain = 0.0f64;
            for m in (0..full).filter(|m| m.count_ones() as usize + TWO_MISS >= ntar) {
                let pm = c.os[m]; if pm <= 0.0 { continue; }
                let keep: Vec<u32> = (0..ntar).filter(|&k| m >> k & 1 == 1).map(|k| s.env.sj.targets[k].group).collect();
                let mut v = s.env.veteran(legs, x, f.a, f.b, c.wp.to_vec()); v.sparks.retain(|sp| keep.contains(&sp.0));
                let mut lv = legs.to_vec(); lv.push(v); let iv = lv.len() - 1; let pv = s.env.pool(&lv);
                // her direct lifts: the base after her
                let mut pvn = pnow.clone(); let mut c1v = c1.clone();
                for ui in 0..n {
                    let u = cands[ui];
                    if d.umas[u].id == d.umas[x].id { continue; }
                    for y in 0..iv { if legal_pair(d, &pv, u, y, iv) { let vv = val(&mut s, &pv, u, y, iv, &mut buf); if vv > pvn[ui] { pvn[ui] = vv; } } }
                    if pvn[ui] > pnow[ui] { c1v[ui] = c1v[ui].min(cont[ui].cost(pvn[ui])); }
                }
                let v0v = vof(&c1v);
                let basev = ScoreBase { cands: &cands, c1: c1v, m23: &zeros, v0: v0v, pnow: pvn, cont: cont.clone(), combos: combos3.clone(), sides: &sides, all: &all };
                let mut bestg = 0.0f64;
                for &bd in &builders {
                    if d.umas[bd].id == d.umas[x].id { continue; }
                    let mut bp = (0.0f64, usize::MAX, usize::MAX);
                    for y in 0..iv { if legal_pair(d, &pv, bd, y, iv) { if let Some((vv, di)) = val_d(&mut s, &pv, bd, y, iv, &mut buf) { if vv > bp.0 { bp = (vv, y, di); } } } }
                    if bp.1 == usize::MAX { continue; }
                    let sel = s.cache.decks[bd][bp.2].sel.clone();
                    let (ey2, _, _, _, _) = score_career(&mut s, d, &basev, &lv, &pv, bd, bp.1, iv, &sel, &mut buf);
                    bestg = bestg.max(ey2 - 1.0 - istar);
                }
                gain += pm * bestg;
            }
            two[wi] = Some(gain);
        }
    }
    let two_ms = now_ms() - t4;
    let work = json!({"nodes": s.env.space.nodes.get() - nodes0, "exact": crate::deck::JCOUNT.with(|c| c.get()) - joints0, "max_ratio": s.env.max_ratio.iter().copied().fold(0.0f64, f64::max), "class_max_ratio": s.env.max_ratio, "class_max_deck": s.env.max_decks(), "cap": s.env.space.rho_top().min(s.env.cert), "class_caps": s.env.space.caps()});
    // measurement (`proxy_check`): each builder's career from her deck class's winning decks (the main search's and the
    // candidates' best careers of her class, legal for her and valued at her own chances) through the surrogate, no career
    // search; its lifts per candidate (efficiency, as Cont takes them) and its score beside the exact ones. Runs after
    // everything else, on the cache as it was before the builders' searches, so the plan's own numbers are unchanged
    let proxy_check: Value = match (pj.proxy_check, cache0) { (true, Some(mut c0)) if !runs.is_empty() => {
        std::mem::swap(&mut s.cache, &mut c0);
        let wins: Vec<(usize, Sel)> = sv.found.iter().map(|f| (f.t, f.sel.clone()))
            .chain(cands.iter().filter_map(|&u| best[u].clone().flatten().map(|f| (u, f.sel)))).collect();
        let base_s = ScoreBase { cands: &cands, c1: vec![1.0; n], m23: &zeros, v0: 0.0, pnow: pnow.clone(), cont: vec![], combos: vec![], sides: &[], all: &[] };
        let base = ScoreBase { cands: &cands, c1: c1.clone(), m23: &zeros, v0: vnow, pnow: pnow.clone(), cont: cont.clone(), combos: combos3.clone(), sides: &sides, all: &all };
        let effs = |x: usize, st: &[(f64, f64)]| -> Vec<f64> { (0..n).map(|ui| {
            let (pl, sp) = st[ui];
            if d.umas[cands[ui]].id == d.umas[x].id || pl <= 0.0 || pnow[ui] <= 0.0 { 0.0 } else { pl * (1.0 / pnow[ui] - 1.0 / (sp / pl)) } }).collect() };
        let mut rows = vec![];
        for (i, r) in runs.iter().enumerate() {
            let x = r.0; let is_c = cands.contains(&x);
            let ee = effs(x, &r.2);
            let (pf, pe, psc) = if is_c { (Some(r.1.clone()), ee.clone(), score.get(i).copied()) } else {
                let cx = s.env.cls[x];
                for (t, sel) in &wins { if *t != x && s.env.cls[*t] == cx { s.cache.add(s.env, x, sel); } }
                match best_sur(&mut s, d, pool, fix, x, 1e9, &mut buf) {
                    None => (None, vec![0.0; n], None),
                    Some(f) => {
                        let (_, _, _, st, _) = score_career(&mut s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, &mut buf);
                        let pe = effs(x, &st);
                        let psc = if score.is_empty() { None } else { Some(score_career(&mut s, d, &base, legs, pool, x, f.a, f.b, &f.sel, &mut buf).0 - 1.0) };
                        (Some(f), pe, psc)
                    }
                }
            };
            let same_pair = pf.as_ref().map_or(false, |f| (f.a.min(f.b), f.a.max(f.b)) == (r.1.a.min(r.1.b), r.1.a.max(r.1.b)));
            let same_deck = pf.as_ref().map_or(false, |f| f.sel.ents == r.1.sel.ents && f.sel.plain == r.1.sel.plain && f.sel.filler == r.1.sel.filler);
            rows.push(json!({"trainee": d.umas[x].name, "cand": is_c, "class": s.env.cls[x], "exact_val": r.1.val, "proxy_val": pf.as_ref().map(|f| f.val),
                "same_pair": same_pair, "same_deck": same_deck, "score": score.get(i), "proxy_score": psc, "eff": ee, "proxy_eff": pe}));
        }
        std::mem::swap(&mut s.cache, &mut c0);
        json!({"cands": cands.iter().map(|&u| d.umas[u].name.clone()).collect::<Vec<_>>(), "rows": rows})
    } _ => Value::Null };
    // the pick's upper bound per outcome (`outcome_hi`; the pick as below, with the same scores)
    let ys_hi: Option<(usize, (Vec<f64>, Vec<f64>))> = if !runs.is_empty() && !score.is_empty() && vnow.is_finite() {
        let total = |i: usize| score.get(i).copied().unwrap_or(f64::NEG_INFINITY) + two[i].unwrap_or(0.0);
        let mut pk = 0usize; for i in 0..runs.len() { if total(i) > total(pk) + 1e-9 { pk = i; } }
        let top = total(pk); let near = |i: usize| total(i) >= top - NEAR.max(0.05 * top.abs());
        for i in 0..runs.len() { if near(i) && runs[i].1.val > runs[pk].1.val { pk = i; } }
        let base = ScoreBase { cands: &cands, c1: c1.clone(), m23: &zeros, v0: vnow, pnow: pnow.clone(), cont: cont.clone(), combos: combos3.clone(), sides: &sides, all: &all };
        let (x, f) = (runs[pk].0, runs[pk].1.clone());
        Some((pk, outcome_hi(&mut s, d, &base, legs, pool, fix, sv, &ytabs[pk], x, f.a, f.b, &f.sel, &mut buf)))
    } else { None };
    drop(s);
    { let key = bests_key(job, fix, nl); let mut kb = best.clone(); kb[g.t] = None;   // the main search's best is per job, searched again (cheap) next time
      BESTS.with(|c| { let mut c = c.borrow_mut(); c.retain(|b| b.key != key); c.push(Bests { key, best: kb, refs: urefs }); if let Some(p) = prev { c.push(p); } while c.len() > 4 { c.remove(0); } }); }
    // nothing scored (no trio: fewer than two candidates of different umas): the best odds is the one option and the pick
    if runs.is_empty() && !pj.value_only { for f in &sv.found { runs.push((f.t, f.clone(), vec![])); two.push(None); } }
    // pick: the highest score; within NEAR of it (the model's noise), the better odds of her own 8/8
    let total = |i: usize| score.get(i).copied().unwrap_or(f64::NEG_INFINITY) + two[i].unwrap_or(0.0);
    let mut pick = 0usize;
    for i in 0..runs.len() { if total(i) > total(pick) + 1e-9 { pick = i; } }
    let top = if runs.is_empty() { 0.0 } else { total(pick) };
    let near = |i: usize| total(i) >= top - NEAR.max(0.05 * top.abs());
    for i in 0..runs.len() { if near(i) && runs[i].1.val > runs[pick].1.val { pick = i; } }
    let role = |u: usize| match best_combo { Some((ia, ib, _, _, _, _)) if u == cands[ia] || u == cands[ib] => "plan", Some((_, _, _, cu, _, _)) if u == cu => "finish",
        _ => if members.contains(&u) { "loop" } else if cands.contains(&u) { "candidate" } else { "builder" } };
    let mut opts: Vec<Value> = vec![];
    for (i, (x, f, _)) in runs.iter().enumerate() {
        let mut rp = env.report(pool, f, false);
        let o = rp.as_object_mut().unwrap();
        o.insert("role".into(), json!(role(*x))); o.insert("best_odds".into(), json!(*x == g.t)); o.insert("chosen".into(), json!(i == pick));
        // false: a builder the plan could not turn on, her career from her deck class's winning decks (no career search)
        o.insert("exact".into(), json!(exact_run.get(i).copied().unwrap_or(true)));
        o.insert("stage_cost".into(), json!(cands.iter().position(|&c| c == *x).map(|ui| c1[ui])));
        o.insert("index".into(), json!(score.get(i))); o.insert("saving".into(), json!(score.get(i).map(|v| v + 1.0)));
        o.insert("lift".into(), json!(lifts.get(i))); o.insert("near".into(), json!(near(i) && i != pick));
        o.insert("two_step".into(), json!(two[i])); o.insert("score".into(), json!(if score.is_empty() { None } else { Some(total(i)) }));
        opts.push(rp);
    }
    opts.sort_by(|a, b| b["score"].as_f64().unwrap_or(f64::NEG_INFINITY).partial_cmp(&a["score"].as_f64().unwrap_or(f64::NEG_INFINITY)).unwrap());
    let choice = if pj.value_only || runs.is_empty() { Value::Null } else { env.report(pool, &runs[pick].1, true) };
    let uname = |u: usize| if u == usize::MAX { Value::Null } else { json!(d.umas[u].name) };
    // a member: `real` when the trio uses a real 8/8 of hers (her step free), else her fresh 8/8's step
    let member = |ui: usize, real: bool| { let (_, nb, ps) = cont[ui].plan(pnow[ui]);
        json!({"trainee": uname(cands[ui]), "p": pnow[ui], "have8": have8[ui], "real": real, "p_inf": if cont[ui].lam > 0.0 { Some(cont[ui].pinf) } else { None }, "lam": cont[ui].lam,
               "phi": cont[ui].phi, "builds": nb, "p_switch": ps, "careers": step(ui, real)}) };
    let (plan_json, members_json) = match best_combo { Some((ia, ib, c3, cu, ra, rb)) => {
        // A: the cheaper of the two steps first
        let ((fa, fra), (fb2, frb)) = if step(ia, ra) <= step(ib, rb) { ((ia, ra), (ib, rb)) } else { ((ib, rb), (ia, ra)) };
        (json!({"a": uname(cands[fa]), "b": uname(cands[fb2]), "c": uname(cu), "careers": [step(fa, fra), step(fb2, frb), c3]}), json!([member(fa, fra), member(fb2, frb)])) }
        None => (Value::Null, json!([])) };
    // the pick's careers saved per outcome (the page reads a reroll's from it): her uncertain targets' stars (base 4 per
    // target in `uncv` order, 0 absent) then the scenario's guaranteed golds present or not, at leaf << nf | fm
    let outcomes = match (ytabs.get(pick), runs.get(pick)) { (Some(ys), Some((x, f, _))) => json!({"trainee": d.umas[*x].id, "parents": [f.a, f.b],
        "uncv": env.uncv, "fixed": env.fixed_t, "y": ys.iter().map(|y| (y * 100.0).round() / 100.0).collect::<Vec<f64>>(),
        // per outcome, at least y: the most her veteran's direct lifts could save (`outcome_hi`; rounded up)
        "y_hi": ys_hi.as_ref().filter(|(pk, (h, _))| *pk == pick && h.len() == ys.len()).map(|(_, (h, _))| h.iter().zip(ys).map(|(hv, y)| (hv.max(*y) * 100.0).ceil() / 100.0).collect::<Vec<f64>>()),
        // the chance a roll reads no saving from y but could save at least half a career by y_hi (the page checks those)
        "provisional": ys_hi.as_ref().filter(|(pk, (h, _))| *pk == pick && h.len() == ys.len()).map(|(_, (h, pr))| (0..ys.len()).filter(|&i| ys[i] < 0.5 && h[i].max(ys[i]) >= 0.5).map(|i| pr[i]).sum::<f64>())}), _ => Value::Null };
    let mut out = json!({"options": opts, "choice": choice, "rule": "index", "mode": "trio", "v0": if vnow.is_finite() { json!(vnow) } else { Value::Null }, "outcomes": outcomes,
           "members": members_json,
           "v0_plan": plan_json, "fit": (0..n).map(|ui| json!([d.umas[cands[ui]].id, cont[ui].lam, cont[ui].phi, cont[ui].pinf])).collect::<Vec<_>>(), "loop_members": minfo, "loop_note": mwhy, "hint": hint, "umas_scored": exact_n, "umas_searched": exact_run.iter().filter(|&&e| e).count() - n_cert, "umas_certified": n_cert, "work": work,
           "sur": pj.sur, "sur_diff": sur_diff, "loop_ms": loop_ms, "search_ms": search_ms, "combo_ms": combo_ms, "index_ms": index_ms, "two_ms": two_ms, "total_ms": now_ms() - t_all});
    if !proxy_check.is_null() { out["proxy_check"] = proxy_check; }
    out
}
