//! The planner's pick on the surrogate simulator (`plan` on a career job; docs/SOLVER.md "Plan").
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
    for k in ["legacies", "trainees", "fix", "per_trainee", "lookahead", "p_now", "p_minus1_now", "policy", "rollout", "plan", "simulate", "top"] { o.remove(k); }
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
        let mut st = St { legs: vec![], pool: Pool { me: vec![], gp: vec![], borrowed: vec![] }, gpc: vec![], fam: Default::default(), best: None, cur: None,
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
        work: 0, cands: 0, solved: 0, bounded: 0, scored: 0, skipped: 0, next_bound: 0.0, complete: sv.complete, proven: sv.proven } }
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
/// trainee u's P(all) with parents a, b (cached decks; a deck search where she has none)
fn val(s: &mut Sim, p: &Pool, u: usize, a: usize, b: usize, buf: &mut Vec<f64>) -> f64 {
    let (q, _, c) = s.env.lineage(p, u, a, b);
    let Some(ub) = s.env.ceiling(u, &q, c) else { return 0.0 };
    s.deck_for(u, &q, c, ub, 0.0, buf).map_or(0.0, |x| x.0)
}
/// an 8/8 veteran (a spark of every job target), yours or borrowed
fn is_full(sj: &SkillJob, sparks: &[(u32, u8)]) -> bool { sj.targets.iter().all(|t| sparks.iter().any(|x| x.0 == t.group && x.1 > 0)) }


/// One place a builder's new veteran may be a parent: the stage-1 candidate she may lift (index), per uncertain target
/// (env.uncv order) the miss product without her, per job target the spark scale without and with her spark, her miss
/// factor x per star count (0: absent), the trainee's cached decks' corner tables
struct Setup { ui: usize, x0: Vec<f64>, pw: Vec<[f64; 2]>, xm: [f64; 4], tabs: Vec<Vec<f64>> }

/// Two deep: candidates given the term (the highest by index, plus the best career's trainee), and builders tried
const TWO_TOP: usize = 3;
/// Scores within this many careers per run of the best (or 5% of it) count as tied
const NEAR: f64 = 0.25;
const TWO_BUILDERS: usize = 3;
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
/// What a career is scored against: stage-1 costs per candidate (index into `cands`; 0 = her 8/8 exists), stages 2-3 per
/// A, V0, and the stage-1 odds now
pub(super) struct ScoreBase<'b> { pub(super) cands: &'b [usize], pub(super) c1: Vec<f64>, pub(super) m23: &'b [f64], pub(super) v0: f64, pub(super) pnow: Vec<f64>, pub(super) cont: Vec<Cont>,
    /// the trios (A, B as candidate indices, C's step): V = min over them of c1[A] + c1[B] + C's step; empty: min over a of
    /// c1[a] + m23[a]
    pub(super) combos: Vec<(usize, usize, f64)> }
/// A step's continuation (form assumed; the rate and step fitted to the pool now, the shape to traces): builder careers
/// lift her odds, each lift closing `phi` of the remaining gap in log odds to `pinf`, her effective ceiling; the lift
/// chance falls with the remaining gap: lam_j = lam ((1 - phi)^j)^LAM_DECAY. pinf = CEIL_SHARE x her full-loop odds.
/// Both constants fitted (val/sim/README.md E5) to traces of building for Agnes Digital on the owner's 2-, 5-, 11- and
/// 19-veteran pools (the best builder each career, 6 runs x 400 careers): within 10-20% on 11 and 19, building
/// underestimated on 5 and overestimated early on 2; leave-one-pool-out fits ranged 0.6-1.0 and 1-3. lam = 0: none.
#[derive(Clone, Copy, Default)]
pub(super) struct Cont { lam: f64, phi: f64, pinf: f64 }
/// Effective ceiling as a share of the full-loop odds, and the lift chance's decline exponent (val/sim/README.md E5)
const CEIL_SHARE: f64 = 0.6;
const LAM_DECAY: f64 = 1.5;
impl Cont {
    /// (expected careers to her 8/8 from odds p, the best number of successes before grinding, the odds then): build n
    /// successes (sum of 1 / lam_j careers), then grind at p_n; the best n
    fn plan(&self, p: f64) -> (f64, i32, f64) {
        if p <= 0.0 { return (f64::INFINITY, 0, p); }
        if self.lam <= 0.0 || self.phi <= 0.0 || self.pinf <= p { return (1.0 / p, 0, p); }
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
}
impl ScoreBase<'_> { fn cost(&self, ui: usize, p: f64) -> f64 { self.cont.get(ui).copied().unwrap_or_default().cost(p) } }
/// Score of career x (parents a, b of `legs`, deck `sel`) against `bs`: E[V0 - V0 with her veteran], the chance it is
/// above 0, and (Y, P) per outcome. Exact over her landed set (`outcome_sets`) and her stars per uncertain target (the
/// rank's mix): her veteran as a parent of each stage-1 candidate (her best IDX_PARTNERS partners, the trainee's cached
/// decks' corner tables, folded one target at a time); her own 8/8 ends her stage when she is a candidate.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub(super) fn score_career(s: &mut Sim, d: &Data, bs: &ScoreBase, legs: &[Legacy], pool: &Pool, x: usize, a: usize, b: usize, sel: &Sel, buf: &mut Vec<f64>) -> (f64, f64, Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let n = bs.cands.len(); let ntar = s.env.ntar; let nu = s.env.uncv.len();
    let uncv = s.env.uncv.clone(); let fixed_t = s.env.fixed_t.clone(); let w = s.env.sj.star_dist;
    let v0_after = |c1: &[f64]| if bs.combos.is_empty() { (0..n).map(|a| c1[a] + bs.m23[a]).fold(f64::INFINITY, f64::min) }
        else { bs.combos.iter().map(|&(ia, ib, c3)| c1[ia] + c1[ib] + c3).fold(f64::INFINITY, f64::min) };
    let xi = bs.cands.iter().position(|&u| u == x);
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
            let mut sl0 = sl.clone(); for k in 0..ntar { sl0[pos].stars[k] = 0; } sl0[pos].mix = None;
            let (q0, nc0) = inherit(&sl0, ntar);
            let xm = |st: u8| { let m = 1.0 - (0.03 * st as f64 * (1.0 + a_v / 100.0)).min(1.0); m * m };
            let tabs: Vec<Vec<f64>> = s.cache.decks[u].iter().flat_map(|dk| dk.tab.iter().cloned()).collect();
            if tabs.is_empty() { continue; }
            setups.push(Setup { ui, x0: uncv.iter().map(|&i| 1.0 - q0[i]).collect(),
                pw: (0..ntar).map(|k| [1.1f64.powi(nc0[k] - 6), 1.1f64.powi(nc0[k] + 1 - 6)]).collect(), xm: [1.0, xm(1), xm(2), xm(3)], tabs });
        }
    }
    // per setup, per star pattern of the uncertain targets (base 4, position k = uncv[k]; 0 absent), the best table value
    // at scale 1: the tables folded one coordinate at a time (interp's order and arithmetic)
    let nleaf = 1usize << (2 * nu);
    let folds: Vec<Vec<f64>> = setups.iter().map(|st| {
        let mut out = vec![0.0f64; nleaf];
        for tab in &st.tabs {
            fn rec(t: &[f64], k: usize, leaf: usize, st: &Setup, nu: usize, out: &mut [f64]) {
                if k == nu { if t[0] > out[leaf] { out[leaf] = t[0]; } return; }
                for sg in 0..4usize {
                    let x = st.x0[k] * st.xm[sg];
                    let h = t.len() / 2;
                    let f: Vec<f64> = (0..h).map(|i| t[2 * i] * (1.0 - x) + t[2 * i + 1] * x).collect();
                    rec(&f, k + 1, leaf | sg << (2 * k), st, nu, out);
                }
            }
            rec(tab, 0, 0, st, nu, &mut out);
        }
        out
    }).collect();
    // every outcome: uncertain targets' star pattern, guaranteed golds present or not
    let nf = fixed_t.len();
    let (mut ey, mut lift) = (0.0f64, 0.0f64);
    let mut ys: Vec<(f64, f64)> = vec![];
    let mut c1 = bs.c1.clone();
    // per stage-1 candidate: the chance her veteran raises that candidate's odds, and the odds raised to (summed, weighted)
    let mut stats = vec![(0.0f64, 0.0f64); n];
    let mut dist0: Vec<(f64, f64)> = vec![];
    let mut up = vec![0.0f64; n];
    for leaf in 0..nleaf {
        let mut mu = 0usize; let mut pr_st = 1.0f64;
        for k in 0..nu { let sg = (leaf >> (2 * k)) & 3; if sg > 0 { mu |= 1 << uncv[k]; pr_st *= w[sg - 1]; } }
        for fm in 0..(1usize << nf) {
            let mut m = mu; for (j, &i) in fixed_t.iter().enumerate() { if fm >> j & 1 == 1 { m |= 1 << i; } }
            let pr = c.os[m] * pr_st;
            if pr <= 0.0 { continue; }
            c1.copy_from_slice(&bs.c1);
            up.copy_from_slice(&bs.pnow);
            for (si, st) in setups.iter().enumerate() {
                let sc: f64 = (0..ntar).map(|k| st.pw[k][(m >> k & 1) as usize]).product();
                let v = sc * folds[si][leaf];
                if v > up[st.ui] { up[st.ui] = v; }
            }
            for ui in 0..n { if up[ui] > bs.pnow[ui] * (1.0 + 1e-9) {
                stats[ui].0 += pr; stats[ui].1 += pr * up[ui];
                if ui == 0 { dist0.push((pr, up[0])); }
                let cc = bs.cost(ui, up[ui]); if cc < c1[ui] { c1[ui] = cc; } } }
            if m == (1usize << ntar) - 1 { if let Some(xi) = xi { c1[xi] = 0.0; } }
            let y = (bs.v0 - v0_after(&c1)).max(0.0);
            ey += pr * y; if y > 1e-9 { lift += pr; }
            ys.push((y, pr));
        }
    }
    (ey, lift, ys, stats, dist0)
}

/// The page's next career, exact where it can be and without simulation: the career that brings a FULLY LOOPED veteran
/// (an 8/8 whose parents are 8/8s) the closest. V = the cheapest trio: A and B two stage candidates, each step "build
/// while it pays, then run her" (`Cont`, fitted to the pool now; free when an 8/8 of hers exists), C the best allowed
/// uma with A's and B's 8/8s as parents (1 / her full-loop odds). A career is scored E[V - V with her veteran] - 1, exact
/// over her outcomes: her veteran as a parent raising any candidate's odds lowers that step, her own 8/8 ends her own
/// step; a roll that changes nothing leaves every score as it was (repeat). Two deep for the 3 highest and the best
/// odds: her rolls one target short, as a parent of the 3 highest builders, over today's best score. Pick: the highest;
/// within NEAR, the better own odds. With a fully looped veteran already, the best odds (unscored).
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn index_rule<'a>(env: &mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, legs: &[Legacy],
                                       pool: &Pool, sv: &Solved, fix: &[usize], trainees: &[usize], pj: &PlanJob) -> Value {
    let t_all = now_ms();
    let (members, minfo, mwhy) = rotation(env, d, sk, cr, job, pj);
    let loop_ms = now_ms() - t_all;
    let g = sv.found[0].clone();
    // stage-1 candidates (who A can be; whose odds a builder may lift): the best career's trainee, the loop's members, the
    // umas of the best career's lineage, the umas you have veterans of, the search's other trainees; at most IDX_CANDS
    let mut cands: Vec<usize> = vec![g.t];
    let push = |u: usize, c: &mut Vec<usize>| if !c.contains(&u) && trainees.contains(&u) { c.push(u) };
    for &m in &members { push(m, &mut cands); }
    for x in [g.a, g.b] { for c in std::iter::once(legs[x].chara).chain(legs[x].parents.iter().map(|p| p.chara)) { if let Some(u) = env.uix(c) { push(u, &mut cands); } } }
    for l in legs.iter().rev() { if !l.borrowed { if let Some(u) = env.uix(l.chara) { push(u, &mut cands); } } }
    for f in &sv.found { push(f.t, &mut cands); }
    cands.truncate(IDX_CANDS);
    let all: Vec<usize> = trainees.iter().copied().filter(|&u| env.dmax[u].is_some()).collect();
    let t1 = now_ms();
    let budget0 = env.budget;
    if pj.search_work > 0 { env.budget = pj.search_work; env.hard = pj.search_work; }
    env.use_seen = true;
    // best careers, searched on demand (stage-1 candidates now, builders when scored exactly)
    let mut best: Vec<Option<Option<Found>>> = vec![None; d.umas.len()];
    best[g.t] = Some(Some(g.clone()));
    let mut refs = vec![];
    for &u in &cands { if best[u].is_none() { let s = env.solve(pool, fix, &[u], false, 1); refs.push(s.refs.clone()); best[u] = Some(s.found.into_iter().next()); } }
    let fb: Vec<Option<Found>> = cands.iter().map(|&u| best[u].clone().flatten()).collect();
    let mut search_ms = now_ms() - t1;
    let mut s = match sim_for(env, d, sk, cr, job, pj) { Ok(s) => s, Err(e) => return e };
    s.cache.absorb(s.env, &sv.refs);
    for r in &refs { s.cache.absorb(s.env, r); }
    let mut buf = vec![];
    let ntar = s.env.ntar; let n = cands.len();
    let pnow: Vec<f64> = fb.iter().map(|f| f.as_ref().map_or(0.0, |f| f.val)).collect();
    let sj = s.env.sj;
    // the goal reached already (a fully looped veteran), and per candidate whether an 8/8 of hers exists (her step free)
    let looped = legs.iter().any(|l| is_full(sj, &l.sparks) && l.parents.len() == 2 && l.parents.iter().all(|p| is_full(sj, &p.sparks)));
    let have8: Vec<bool> = cands.iter().map(|&u| legs.iter().any(|l| l.chara == d.umas[u].id && is_full(sj, &l.sparks))).collect();
    let zeros = vec![0.0f64; n];
    let mut exact_n = 0usize;
    let t3 = now_ms();
    // pass 1: every allowed uma's best career; per candidate, the chance her veteran raises that candidate's odds and the
    // mean odds raised to (exact over her outcomes)
    let mut runs: Vec<(usize, Found, Vec<(f64, f64)>)> = vec![];
    let mut wtab: Vec<Vec<usize>> = vec![vec![]; d.umas.len()];
    if !pj.value_only && !looped {
        // (placeholder step costs: 1 where her step is open, 0 where her 8/8 exists, so only open steps gather lifts)
        let open: Vec<f64> = have8.iter().map(|&h| if h { 0.0 } else { 1.0 }).collect();
        let base_s = ScoreBase { cands: &cands, c1: open, m23: &zeros, v0: 0.0, pnow: pnow.clone(), cont: vec![], combos: vec![] };
        let mut order: Vec<usize> = vec![g.t]; for &x in &all { if x != g.t { order.push(x); } }
        for &x in &order {
            if best[x].is_none() {
                let ts = now_ms();
                if pj.search_work > 0 { s.env.budget = pj.search_work; s.env.hard = pj.search_work; }
                let sr = s.env.solve(pool, fix, &[x], false, 1);
                s.env.hard = 0; s.env.budget = budget0;
                s.cache.absorb(s.env, &sr.refs);
                best[x] = Some(sr.found.into_iter().next());
                search_ms += now_ms() - ts;
            }
            let Some(Some(f)) = best[x].clone() else { continue };
            let (_, _, _, st, _) = score_career(&mut s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, &mut buf);
            runs.push((x, f, st));
        }
        for r in &runs { let w = s.wins(pool, r.0, r.1.a, r.1.b); wtab[r.0] = w.iter().enumerate().filter(|x| *x.1 > 0.5).map(|x| x.0).collect(); }
    }
    let wt = |x: usize| wtab[x].clone();
    // each candidate's step with building: her most efficient builder now (rate, log-odds step), her full-loop ceiling with
    // her two most efficient builders
    let mut cont = vec![Cont::default(); n];
    for ui in 0..n {
        if have8[ui] || pnow[ui] <= 0.0 || runs.is_empty() { continue; }
        let u = cands[ui]; let p = pnow[ui];
        let mut eff: Vec<(f64, usize, f64, f64)> = runs.iter().filter(|r| d.umas[r.0].id != d.umas[u].id && r.2[ui].0 > 0.0)
            .map(|r| { let (pl, sp) = r.2[ui]; let p1 = sp / pl; (pl * (1.0 / p - 1.0 / p1), r.0, pl, p1) }).collect();
        eff.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        let Some(&(_, b1, lam, p1)) = eff.first() else { continue };
        let b2 = eff.iter().skip(1).map(|e| e.1).find(|&b| d.umas[b].id != d.umas[b1].id);
        let pinf = match b2 { Some(b2) => (CEIL_SHARE * loop_ceiling(&mut s, d, legs, u, b1, b2, &wt, &mut buf)).max(p1), None => p1 };
        let phi = if pinf > p && p1 > p { ((p1 / p).ln() / (pinf / p).ln()).clamp(0.0, 1.0) } else { 0.0 };
        cont[ui] = Cont { lam, phi, pinf };
    }
    let c1: Vec<f64> = (0..n).map(|ui| if have8[ui] { 0.0 } else { cont[ui].cost(pnow[ui]) }).collect();
    // the trios: A and B two candidates (unordered), C the best allowed uma with their 8/8s as parents (her odds with a
    // full loop lineage of A and B)
    let t_c = now_ms();
    let mut combos: Vec<(usize, usize, f64, usize)> = vec![];
    if !looped { for ia in 0..n { for ib in ia + 1..n {
        let (ua, ub) = (cands[ia], cands[ib]);
        if d.umas[ua].id == d.umas[ub].id || !c1[ia].is_finite() || !c1[ib].is_finite() { continue; }
        let mut bc = (0.0f64, usize::MAX);
        for &c in &all { if d.umas[c].id == d.umas[ua].id || d.umas[c].id == d.umas[ub].id { continue; }
            let v = loop_ceiling(&mut s, d, legs, c, ua, ub, &wt, &mut buf); if v > bc.0 { bc = (v, c); } }
        if bc.1 != usize::MAX { combos.push((ia, ib, 1.0 / bc.0, bc.1)); }
    } } }
    let combo_ms = now_ms() - t_c;
    let vof = |c: &[f64]| combos.iter().map(|&(ia, ib, c3, _)| c[ia] + c[ib] + c3).fold(f64::INFINITY, f64::min);
    let vnow = if looped { 0.0 } else { vof(&c1) };
    let best_combo = combos.iter().min_by(|x, y| (c1[x.0] + c1[x.1] + x.2).partial_cmp(&(c1[y.0] + c1[y.1] + y.2)).unwrap()).copied();
    // pass 2: every career scored against V (exact over her outcomes): E[V - V with her veteran] - 1
    let combos3: Vec<(usize, usize, f64)> = combos.iter().map(|c| (c.0, c.1, c.2)).collect();
    let mut score: Vec<f64> = vec![]; let mut lifts: Vec<f64> = vec![];
    if !runs.is_empty() && vnow.is_finite() {
        let base = ScoreBase { cands: &cands, c1: c1.clone(), m23: &zeros, v0: vnow, pnow: pnow.clone(), cont: cont.clone(), combos: combos3.clone() };
        for r in &runs {
            let (ey, lift, _, _, _) = score_career(&mut s, d, &base, legs, pool, r.0, r.1.a, r.1.b, &r.1.sel, &mut buf);
            exact_n += 1; score.push(ey - 1.0); lifts.push(lift);
        }
    }
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
                    if d.umas[u].id == d.umas[x].id || have8[ui] { continue; }
                    for y in 0..iv { if legal_pair(d, &pv, u, y, iv) { let vv = val(&mut s, &pv, u, y, iv, &mut buf); if vv > pvn[ui] { pvn[ui] = vv; } } }
                    if pvn[ui] > pnow[ui] { c1v[ui] = c1v[ui].min(cont[ui].cost(pvn[ui])); }
                }
                let v0v = vof(&c1v);
                let basev = ScoreBase { cands: &cands, c1: c1v, m23: &zeros, v0: v0v, pnow: pvn, cont: cont.clone(), combos: combos3.clone() };
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
    drop(s);
    // nothing scored (the goal reached, or no trio): the best odds is the one option and the pick
    if runs.is_empty() && !pj.value_only { for f in &sv.found { runs.push((f.t, f.clone(), vec![])); two.push(None); } }
    // pick: the highest score; within NEAR of it (the model's noise), the better odds of her own 8/8
    let total = |i: usize| score.get(i).copied().unwrap_or(f64::NEG_INFINITY) + two[i].unwrap_or(0.0);
    let mut pick = 0usize;
    for i in 0..runs.len() { if total(i) > total(pick) + 1e-9 { pick = i; } }
    let top = if runs.is_empty() { 0.0 } else { total(pick) };
    let near = |i: usize| total(i) >= top - NEAR.max(0.05 * top.abs());
    for i in 0..runs.len() { if near(i) && runs[i].1.val > runs[pick].1.val { pick = i; } }
    if looped { if let Some(i) = runs.iter().position(|r| r.0 == g.t) { pick = i; } }
    let role = |u: usize| match best_combo { Some((ia, ib, _, _)) if u == cands[ia] || u == cands[ib] => "plan", Some((_, _, _, cu)) if u == cu => "finish",
        _ => if members.contains(&u) { "loop" } else if cands.contains(&u) { "candidate" } else { "builder" } };
    let mut opts: Vec<Value> = vec![];
    for (i, (x, f, _)) in runs.iter().enumerate() {
        let mut rp = env.report(pool, f, false);
        let o = rp.as_object_mut().unwrap();
        o.insert("role".into(), json!(role(*x))); o.insert("best_odds".into(), json!(*x == g.t)); o.insert("chosen".into(), json!(i == pick));
        o.insert("stage_cost".into(), json!(cands.iter().position(|&c| c == *x).map(|ui| c1[ui])));
        o.insert("index".into(), json!(score.get(i))); o.insert("saving".into(), json!(score.get(i).map(|v| v + 1.0)));
        o.insert("lift".into(), json!(lifts.get(i))); o.insert("near".into(), json!(near(i) && i != pick));
        o.insert("two_step".into(), json!(two[i])); o.insert("score".into(), json!(if score.is_empty() { None } else { Some(total(i)) }));
        opts.push(rp);
    }
    opts.sort_by(|a, b| b["score"].as_f64().unwrap_or(f64::NEG_INFINITY).partial_cmp(&a["score"].as_f64().unwrap_or(f64::NEG_INFINITY)).unwrap());
    let choice = if pj.value_only || runs.is_empty() { Value::Null } else { env.report(pool, &runs[pick].1, true) };
    let uname = |u: usize| if u == usize::MAX { Value::Null } else { json!(d.umas[u].name) };
    let member = |ui: usize| { let (_, nb, ps) = cont[ui].plan(pnow[ui]);
        json!({"trainee": uname(cands[ui]), "p": pnow[ui], "have8": have8[ui], "p_inf": if cont[ui].lam > 0.0 { Some(cont[ui].pinf) } else { None }, "lam": cont[ui].lam,
               "phi": cont[ui].phi, "builds": nb, "p_switch": ps, "careers": c1[ui], "careers_frozen": if pnow[ui] > 0.0 { 1.0 / pnow[ui] } else { f64::INFINITY }}) };
    let (plan_json, members_json) = match best_combo { Some((ia, ib, c3, cu)) => {
        // A: the cheaper of the two steps first
        let (fa, fb2) = if c1[ia] <= c1[ib] { (ia, ib) } else { (ib, ia) };
        (json!({"a": uname(cands[fa]), "b": uname(cands[fb2]), "c": uname(cu), "careers": [c1[fa], c1[fb2], c3]}), json!([member(fa), member(fb2)])) }
        None => (Value::Null, json!([])) };
    // the same trios with every step at today's odds (no building), for reference
    let cf: Vec<f64> = (0..n).map(|ui| if have8[ui] { 0.0 } else if pnow[ui] > 0.0 { 1.0 / pnow[ui] } else { f64::INFINITY }).collect();
    let v0f = { let v = vof(&cf); if v.is_finite() { json!(v) } else { Value::Null } };
    json!({"options": opts, "choice": choice, "rule": "index", "mode": if looped { "done" } else { "trio" }, "v0": if vnow.is_finite() { json!(vnow) } else { Value::Null },
           "v0_frozen": v0f, "members": members_json,
           "v0_plan": plan_json, "loop_members": minfo, "loop_note": mwhy, "umas_scored": exact_n,
           "loop_ms": loop_ms, "search_ms": search_ms, "combo_ms": combo_ms, "index_ms": index_ms, "two_ms": two_ms, "total_ms": now_ms() - t_all})
}
