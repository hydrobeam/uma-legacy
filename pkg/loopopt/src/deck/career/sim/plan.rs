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

/// The frozen three-stage value (module notes of `index_rule`): per candidate uma, her stage-1 cost and the cheapest
/// stages 2-3 with her as A; V0 and its plan
struct Frozen { c1: Vec<f64>, m23: Vec<f64>, plan: Vec<(usize, usize, usize, [f64; 3])>, v0: f64, at: usize }

/// legal parents a, b for trainee u: different characters, neither hers, at most one borrowed
fn legal_pair(d: &Data, p: &Pool, u: usize, a: usize, b: usize) -> bool {
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

/// V0: the cheapest plan A -> B -> C over the candidates, each stage repeated until its 8/8 (an existing 8/8 of that uma
/// costs nothing): 1/p_A + 1/p_B(A8 available) + 1/p_C(parents A8, B8). A8 and B8 are hypothetical 8/8 veterans
/// (`veteran`: stars at the rank's mix, G1 wins at her schedule's chances, parents her stage's parents). B's parents: the
/// best for B with A8 available. Frozen: no veteran other than A8 and B8 changes any stage (an upper bound on the
/// optimum's careers, for this plan family).
fn frozen(s: &mut Sim, d: &Data, legs: &[Legacy], cands: &[usize], fb: &[Option<Found>], buf: &mut Vec<f64>) -> Frozen {
    let n = cands.len(); let sj = s.env.sj;
    let full_of = |legs: &[Legacy], u: usize| legs.iter().enumerate().filter(|(_, l)| l.chara == d.umas[u].id && is_full(sj, &l.sparks))
        .max_by_key(|(_, l)| l.sparks.iter().map(|x| x.1 as u32).sum::<u32>()).map(|(i, _)| i);
    let pool0 = s.env.pool(legs);
    let mut c1 = vec![f64::INFINITY; n]; let mut m23 = vec![f64::INFINITY; n];
    let mut plan = vec![(usize::MAX, usize::MAX, usize::MAX, [f64::INFINITY; 3]); n];
    for ai in 0..n {
        let a = cands[ai];
        let (legs_a, ia) = match full_of(legs, a) {
            Some(i) => { c1[ai] = 0.0; (legs.to_vec(), i) }
            None => { let Some(f) = &fb[ai] else { continue }; if f.val <= 0.0 { continue; }
                c1[ai] = 1.0 / f.val;
                let wp = s.wins(&pool0, a, f.a, f.b).to_vec();
                let mut l = legs.to_vec(); l.push(s.env.veteran(legs, a, f.a, f.b, wp)); let i = l.len() - 1; (l, i) }
        };
        let pool_a = s.env.pool(&legs_a);
        for bi in 0..n {
            let b = cands[bi];
            if d.umas[b].id == d.umas[a].id { continue; }
            let (legs_ab, ib, cb) = match full_of(&legs_a, b) {
                Some(i) => (legs_a.clone(), i, 0.0),
                None => {
                    // B's best parents with A8 available: A8 and her best partner, or her best career without A8
                    let (mut best, mut pp) = (fb[bi].as_ref().map_or(0.0, |f| f.val), fb[bi].as_ref().map(|f| (f.a, f.b)));
                    for x in 0..legs_a.len() { if x != ia && legal_pair(d, &pool_a, b, ia, x) {
                        let v = val(s, &pool_a, b, ia, x, buf); if v > best { best = v; pp = Some((ia, x)); } } }
                    let Some((pa, pb)) = pp else { continue }; if best <= 0.0 { continue; }
                    let wp = s.wins(&pool_a, b, pa, pb).to_vec();
                    let mut l = legs_a.clone(); l.push(s.env.veteran(&legs_a, b, pa, pb, wp)); let i = l.len() - 1;
                    (l, i, 1.0 / best)
                }
            };
            if cb >= m23[ai] { continue; }
            let pool_ab = s.env.pool(&legs_ab);
            for ci in 0..n {
                let c = cands[ci];
                if !legal_pair(d, &pool_ab, c, ia, ib) { continue; }
                let v = val(s, &pool_ab, c, ia, ib, buf); if v <= 0.0 { continue; }
                let cc = 1.0 / v;
                if cb + cc < m23[ai] { m23[ai] = cb + cc; plan[ai] = (bi, ci, 0, [c1[ai], cb, cc]); }
            }
        }
    }
    let (mut v0, mut at) = (f64::INFINITY, usize::MAX);
    for ai in 0..n { if c1[ai] + m23[ai] < v0 { v0 = c1[ai] + m23[ai]; at = ai; } }
    Frozen { c1, m23, plan, v0, at }
}

/// One place a candidate's new veteran may be a parent: trainee (candidate index), per uncertain target (env.uncv
/// order) the miss product without her, per job target the spark scale without and with her spark, her miss factor
/// x per star count (0: absent), the trainee's cached decks' corner tables
struct Setup { ui: usize, x0: Vec<f64>, pw: Vec<[f64; 2]>, xm: [f64; 4], tabs: Vec<Vec<f64>> }

/// The page's next career, exact and without simulation: the career that saves the most careers to a FULLY LOOPED
/// veteran. Value V0 (`frozen`): the cheapest plan A -> B -> C if no veteran but its 8/8s mattered (an upper bound).
/// Index of candidate X (her best career, odds p_X): I(X) = E_v[V0 - V0(+v)] - 1, v her veteran, the expectation
/// exact over her landed set (`outcome_sets`) and her stars per uncertain target (the rank's mix; stars on a
/// guaranteed gold change no P(all), only its carriers). V0(+v): each candidate's stage-1 odds raised where v as a
/// parent beats them (her best partners, the trainee's cached decks: a floor), X's own stage 1 done when v is 8/8;
/// stages 2-3 kept (first order). Derived: the stage career itself scores about 0 (its 8/8 saves 1/p with chance p);
/// a roll that changes nobody's odds leaves every index as it was, so the same career is picked again (repeat); the
/// highest index is run. Reservation value r (Weitzman): E[(Y - r)+] = 1, "repeat until a roll saves at least r".
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn index_rule<'a>(env: &mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, legs: &[Legacy],
                                       pool: &Pool, sv: &Solved, fix: &[usize], trainees: &[usize], pj: &PlanJob) -> Value {
    let t_all = now_ms();
    let (members, minfo, mwhy) = rotation(env, d, sk, cr, job, pj);
    let loop_ms = now_ms() - t_all;
    let g = sv.found[0].clone();
    // candidates: the best career's trainee, the loop's members, the umas of the best career's lineage, the umas you have
    // veterans of, the search's other trainees; each once, allowed, at most IDX_CANDS
    let mut cands: Vec<usize> = vec![g.t];
    let push = |u: usize, c: &mut Vec<usize>| if !c.contains(&u) && trainees.contains(&u) { c.push(u) };
    for &m in &members { push(m, &mut cands); }
    for x in [g.a, g.b] { for c in std::iter::once(legs[x].chara).chain(legs[x].parents.iter().map(|p| p.chara)) { if let Some(u) = env.uix(c) { push(u, &mut cands); } } }
    for l in legs.iter().rev() { if !l.borrowed { if let Some(u) = env.uix(l.chara) { push(u, &mut cands); } } }
    for f in &sv.found { push(f.t, &mut cands); }
    cands.truncate(IDX_CANDS);
    let t1 = now_ms();
    let budget0 = env.budget;
    if pj.search_work > 0 { env.budget = pj.search_work; env.hard = pj.search_work; }
    env.use_seen = true;
    let mut fb: Vec<Option<Found>> = vec![]; let mut refs = vec![];
    for &u in &cands {
        if u == g.t { fb.push(Some(g.clone())); continue; }
        let s = env.solve(pool, fix, &[u], false, 1);
        refs.push(s.refs.clone());
        fb.push(s.found.into_iter().next());
    }
    env.hard = 0; env.budget = budget0;
    let search_ms = now_ms() - t1;
    let mut s = match sim_for(env, d, sk, cr, job, pj) { Ok(s) => s, Err(e) => return e };
    s.cache.absorb(s.env, &sv.refs);
    for r in &refs { s.cache.absorb(s.env, r); }
    let mut buf = vec![];
    let t2 = now_ms();
    let fr = frozen(&mut s, d, legs, &cands, &fb, &mut buf);
    let v0_ms = now_ms() - t2;
    let names = |i: usize| if i == usize::MAX { Value::Null } else { json!(d.umas[cands[i]].name) };
    let plan_json = if fr.at == usize::MAX { Value::Null } else {
        let (bi, ci, _, c) = fr.plan[fr.at];
        json!({"a": names(fr.at), "b": names(bi), "c": names(ci), "careers": c}) };
    let n = cands.len(); let ntar = s.env.ntar; let nu = s.env.uncv.len();
    let uncv = s.env.uncv.clone(); let fixed_t = s.env.fixed_t.clone();
    let w = s.env.sj.star_dist;
    // per candidate: index, savings, chance her roll saves anything, reservation value
    let mut idx: Vec<Option<(f64, f64, f64, Option<(f64, f64)>)>> = vec![None; n];
    let t3 = now_ms();
    if !pj.value_only && fr.v0.is_finite() {
        let pnow: Vec<f64> = fb.iter().map(|f| f.as_ref().map_or(0.0, |f| f.val)).collect();
        for xi in 0..n {
            let Some(f) = fb[xi].clone() else { continue };
            let x = cands[xi];
            let c = s.make_cur(pool, x, f.a, f.b, &f.sel, false);
            let mut lv = legs.to_vec(); lv.push(s.env.veteran(legs, x, f.a, f.b, c.wp.to_vec())); let iv = lv.len() - 1;
            let pv = s.env.pool(&lv);
            // where she can be a parent: per other candidate (stage 1 still to run) her best partners
            let mut setups: Vec<Setup> = vec![];
            for ui in 0..n {
                let u = cands[ui];
                if d.umas[u].id == d.umas[x].id || fr.c1[ui] == 0.0 || !fr.c1[ui].is_finite() { continue; }
                let mut ps: Vec<(f64, usize)> = (0..iv).filter(|&b| legal_pair(d, &pv, u, b, iv)).map(|b| (val(&mut s, &pv, u, b, iv, &mut buf), b)).collect();
                ps.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap()); ps.truncate(IDX_PARTNERS);
                for (_, b) in ps {
                    let sl = s.env.slots(&pv, u, b, iv);
                    let Some(pos) = sl.iter().position(|x| x.parent && x.legacy == iv) else { continue };
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
            // per setup, per star pattern of the uncertain targets (base 4, position k = uncv[k]; 0 absent), the best
            // table value at scale 1: the tables folded one coordinate at a time (interp's order and arithmetic)
            let nleaf = 1usize << (2 * nu);
            let folds: Vec<Vec<f64>> = setups.iter().map(|st| {
                let mut out = vec![0.0f64; nleaf];
                for tab in &st.tabs {
                    // depth-first over coordinates, a stack of partially folded tables
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
            let mut c1 = fr.c1.clone();
            for leaf in 0..nleaf {
                let mut mu = 0usize; let mut pr_st = 1.0f64;
                for k in 0..nu { let sg = (leaf >> (2 * k)) & 3; if sg > 0 { mu |= 1 << uncv[k]; pr_st *= w[sg - 1]; } }
                for fm in 0..(1usize << nf) {
                    let mut m = mu; for (j, &i) in fixed_t.iter().enumerate() { if fm >> j & 1 == 1 { m |= 1 << i; } }
                    let pr = c.os[m] * pr_st;
                    if pr <= 0.0 { continue; }
                    c1.copy_from_slice(&fr.c1);
                    for (si, st) in setups.iter().enumerate() {
                        let sc: f64 = (0..ntar).map(|k| st.pw[k][(m >> k & 1) as usize]).product();
                        let v = sc * folds[si][leaf];
                        if v > pnow[st.ui] * (1.0 + 1e-9) && 1.0 / v < c1[st.ui] { c1[st.ui] = 1.0 / v; }
                    }
                    if m == (1usize << ntar) - 1 { c1[xi] = 0.0; }
                    let v0n = (0..n).map(|a| c1[a] + fr.m23[a]).fold(f64::INFINITY, f64::min);
                    let y = (fr.v0 - v0n).max(0.0);
                    ey += pr * y; if y > 1e-9 { lift += pr; }
                    ys.push((y, pr));
                }
            }
            // reservation value: E[(Y - r)+] = 1 (none when E[Y] <= 1)
            let r = if ey > 1.0 {
                let (mut lo, mut hi) = (0.0f64, ys.iter().map(|x| x.0).fold(0.0, f64::max));
                for _ in 0..60 { let mid = 0.5 * (lo + hi); let e: f64 = ys.iter().map(|&(y, p)| p * (y - mid).max(0.0)).sum(); if e > 1.0 { lo = mid } else { hi = mid } }
                let r = 0.5 * (lo + hi);
                Some((r, ys.iter().filter(|x| x.0 >= r).map(|x| x.1).sum::<f64>())) } else { None };
            idx[xi] = Some((ey - 1.0, ey, lift, r));
        }
    }
    let index_ms = now_ms() - t3;
    drop(s);
    // pick: the highest index (ties to the best career)
    let mut pick = 0usize;
    for i in 0..n { if let (Some(a), Some(b)) = (idx[i], idx[pick]) { if a.0 > b.0 + 1e-9 { pick = i; } } }
    let role = |u: usize| if u == g.t { "best" } else if members.contains(&u) { "loop" } else { "lineage" };
    let mut opts: Vec<Value> = vec![];
    for i in 0..n {
        let Some(f) = &fb[i] else { continue };
        let mut r = env.report(pool, f, false);
        let o = r.as_object_mut().unwrap();
        o.insert("role".into(), json!(role(cands[i]))); o.insert("best_odds".into(), json!(i == 0)); o.insert("chosen".into(), json!(i == pick && !pj.value_only));
        o.insert("stage_cost".into(), json!(fr.c1[i]));
        if let Some((ix, ey, lift, r)) = idx[i] { o.insert("index".into(), json!(ix)); o.insert("saving".into(), json!(ey)); o.insert("lift".into(), json!(lift)); o.insert("reservation".into(), json!(r.map(|x| x.0))); o.insert("reservation_chance".into(), json!(r.map(|x| x.1))); }
        opts.push(r);
    }
    let choice = match &fb[pick] { Some(f) => env.report(pool, f, true), None => Value::Null };
    json!({"options": opts, "choice": if pj.value_only { Value::Null } else { choice }, "rule": "index", "v0": if fr.v0.is_finite() { json!(fr.v0) } else { Value::Null },
           "v0_plan": plan_json, "loop_members": minfo, "loop_note": mwhy,
           "loop_ms": loop_ms, "search_ms": search_ms, "v0_ms": v0_ms, "index_ms": index_ms, "total_ms": now_ms() - t_all})
}
