//! The planner's pick on the surrogate simulator (`plan` on a career job; docs/SOLVER.md "Plan").
//!
//! The objective is the one the simulator measures: the expected number of careers until a career's veteran lands
//! every target. Per candidate career c (trainee, parents, her exact best deck) and sample j (derived, as `sim`):
//!
//!   X_j(c) = 1 + sum over careers t >= 1 of S_t,   S_1 = 1 - p_c,   S_{t+1} = S_t (1 - p_t)
//!
//! career 0 is c itself (p_c its exact P(all); its failed veteran drawn from its exact landed-set distribution), every
//! later career is the surrogate greedy's (`Sim::play`, policy `greedy`: the best P(all) on the cached decks, re-chosen
//! after every career), stopping when S_t < eps or at t = horizon, where the tail S_t / p_t (career t repeated forever)
//! is added. The greedy continuation only ever moves to a higher P(all), so on every path the tail is at least what
//! greedy would still need after t (derived): X_j(c) is an upper bound of "c, then greedy for ever" on its path, and
//! the bound tightens with the horizon. Common random numbers: sample j uses `Keys::Main { seed, run: j }` for every
//! candidate (the same uniforms at the same career index), so differences between candidates are paired.
//!
//! Allocation (successive doubling): every candidate gets `samples_min` samples (sample index outer, candidate inner, so
//! a budget that runs out leaves the counts even); the leader L is the eligible candidate with the lowest mean; a
//! candidate is separated from L when the paired difference X_j(c) - X_j(L) over their common samples has its 95%
//! Student-t interval above 0. While some eligible candidate is not separated, L and those candidates double their
//! samples (to `samples_max`); stop when none is left ("separated"), all of them are at `samples_max` ("samples_max"),
//! or `budget_ms` of wall clock has passed ("budget"; checked before each sample). Pick: among L and the candidates
//! not separated from it, the one with the highest P(all) now ("about the same" when there are several).
use super::*;

fn p_budget() -> f64 { 6000.0 }
fn p_smin() -> usize { 4 }
fn p_smax() -> usize { 16 }
fn p_eps() -> f64 { 1e-3 }
fn p_horizon() -> usize { 50 }
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
    /// best career of each of the `trainees` best trainees
    #[serde(default = "p_top")] top: usize,
    #[serde(default = "p_recent")] recent: usize,
    #[serde(default = "p_trainees")] trainees: usize,
    /// the surrogate's refresh margin (`sim` module notes)
    #[serde(default = "p_margin")] margin: f64,
    /// work budget (career-search units) of each candidate search beyond the job's own, and of each deck search a
    /// continuation makes (a refresh); 0 = the job's budget
    #[serde(default = "p_swork")] search_work: usize,
    #[serde(default = "p_rwork")] refresh_work: usize,
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

/// What the sampling found: per candidate its samples; why it stopped; time; careers simulated
struct Est { xs: Vec<Vec<f64>>, stop: &'static str, ms: f64, careers: usize, leader: Option<usize> }

impl<'s, 'a> Sim<'s, 'a> {
    /// one sample of candidate c from state st0 (module notes)
    fn plan_sample(&mut self, st0: &St, c: &Cur, j: usize, seed: u32, eps: f64, careers: &mut usize) -> f64 {
        let keys = Keys::Main { seed, run: j as u32 };
        let mut st = st0.clone();
        let (vet, _) = self.draw(c, &st.legs, &keys, 0);
        let mut buf = vec![];
        if !(self.cfg.prune && self.dominated(&st, &vet)) { self.add_vet(&mut st, vet, true, &mut buf); }
        let (e, t, ..) = self.play(&mut st, Base::Greedy, keys, 1.0 - c.p, 1, eps, None, &mut vec![], &mut vec![]);
        *careers += t;
        1.0 + e
    }
    /// the allocation (module notes) over `cs`; only `eligible` ones lead or count as contenders
    fn plan_estimate(&mut self, st0: &St, cs: &[Cur], eligible: &[bool], pj: &PlanJob) -> Est {
        let t0 = now_ms();
        let n = cs.len();
        let smax = pj.samples_max.max(1);
        let mut xs: Vec<Vec<f64>> = vec![vec![]; n];
        let mut want = vec![pj.samples_min.clamp(1, smax); n];
        let mut careers = 0usize;
        let leader_of = |xs: &[Vec<f64>]| (0..n).filter(|&i| eligible[i] && !xs[i].is_empty()).min_by(|&x, &y| mean(&xs[x]).partial_cmp(&mean(&xs[y])).unwrap());
        let stop;
        'rounds: loop {
            let top = *want.iter().max().unwrap();
            for j in 0..top {
                for i in 0..n {
                    if xs[i].len() != j || j >= want[i] { continue; }
                    if now_ms() - t0 > pj.budget_ms { stop = "budget"; break 'rounds; }
                    let x = self.plan_sample(st0, &cs[i], j, pj.seed, pj.eps, &mut careers);
                    xs[i].push(x);
                }
            }
            let Some(l) = leader_of(&xs) else { stop = "no candidate"; break };
            let open: Vec<usize> = (0..n).filter(|&i| i != l && eligible[i] && !separated(&xs[i], &xs[l])).collect();
            if open.is_empty() { stop = "separated"; break; }
            let target = (2 * xs[l].len()).min(smax);
            let mut grew = false;
            for &i in open.iter().chain([&l]) { if xs[i].len() < target { want[i] = want[i].max(target); grew = true; } }
            if !grew { stop = "samples_max"; break; }
        }
        let leader = leader_of(&xs);
        Est { xs, stop, ms: now_ms() - t0, careers, leader }
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
             fastc, fixed, main: false, audits: vec![], fill: true })
}

impl<'s, 'a> Sim<'s, 'a> {
    /// `initial` without its own search: the cache starts from `refs` (searches already made), then every candidate of
    /// the pool is valued best ceiling first
    fn plan_initial(&mut self, legs: &[Legacy], refs: &[&[Vec<Ref>]]) -> St {
        let mut st = St { legs: vec![], pool: Pool { me: vec![], gp: vec![], borrowed: vec![] }, gpc: vec![], fam: Default::default(), best: None, cur: None, rot: Rot::default() };
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

fn row(env: &mut Env, pool: &Pool, c: &Cand, x: &[f64], lead: Option<&[f64]>) -> Value {
    let mut r = env.report(pool, &c.f, false);
    let (m, h) = ci(x);
    let o = r.as_object_mut().unwrap();
    o.insert("expected".into(), if x.is_empty() { Value::Null } else { json!(m) });
    o.insert("ci".into(), json!(h));
    o.insert("samples".into(), json!(x.len()));
    o.insert("eligible".into(), json!(c.eligible));
    o.insert("greedy".into(), json!(c.greedy));
    o.insert("from".into(), json!(c.from));
    o.insert("vs_leader".into(), match lead.and_then(|l| paired(x, l)) { Some((m, h)) => json!([m, h]), None => Value::Null });
    r
}

/// The plan for a career job (module notes): candidates, the estimate per candidate, the pick. `sv` is the job's main
/// search (its refs seed the cache), `fix`/`trainees` the job's places.
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn plan<'a>(env: &mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, legs: &[Legacy], pool: &Pool,
                                 sv: &Solved, reuse_top: bool, fix: &[usize], trainees: &[usize], pj: &PlanJob) -> Value {
    let t_all = now_ms();
    let mut s = match sim_for(env, d, sk, cr, job, pj) { Ok(s) => s, Err(e) => return e };
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
    let mut rec: Vec<(usize, Solved)> = vec![];
    if fix.len() < 2 {
        for v in (0..legs.len()).rev().filter(|&v| !legs[v].borrowed && !fix.contains(&v)).take(pj.recent) {
            let mut f2 = fix.to_vec(); f2.push(v);
            rec.push((v, s.env.solve(pool, &f2, trainees, false, 1)));
        }
    }
    tm.push(now_ms() - t1);
    s.env.hard = 0;
    s.env.budget = if pj.refresh_work > 0 { pj.refresh_work } else { budget0 };
    let mut refs: Vec<&[Vec<Ref>]> = vec![&sv.refs, &top.refs];
    if let Some(p) = &per { refs.push(&p.refs); }
    for (_, r) in &rec { refs.push(&r.refs); }
    let t_init = now_ms();
    let mut st0 = s.plan_initial(legs, &refs);
    let init_ms = now_ms() - t_init;
    let mut cs: Vec<Cand> = vec![];
    let top_key = top.found.first().map(|f| (f.t, f.a.min(f.b), f.a.max(f.b)));
    let g_ok = st0.best.map_or(false, |p| trainees.contains(&p.t) && fix.iter().all(|&x| x == p.a || x == p.b));
    if let Some(g) = s.greedy_cand(&mut st0, g_ok) { cs.push(g); }
    for f in top.found { s.add_cand(&mut cs, pool, f, true, "top"); }
    for (_, r) in rec { for f in r.found { s.add_cand(&mut cs, pool, f, true, "recent"); } }
    if let Some(p) = per { for f in p.found { s.add_cand(&mut cs, pool, f, true, "trainee"); } }
    // a searched candidate equal to the greedy pick replaces it (the search's deck is the exact best there)
    if cs.len() > 1 && cs[0].greedy {
        let k = (cs[0].f.t, cs[0].f.a.min(cs[0].f.b), cs[0].f.a.max(cs[0].f.b));
        if let Some(i) = (1..cs.len()).find(|&i| (cs[i].f.t, cs[i].f.a.min(cs[i].f.b), cs[i].f.a.max(cs[i].f.b)) == k) {
            let mut c = cs.remove(i); c.greedy = true; c.from.insert(0, "greedy"); cs[0] = c;
        }
    }
    if cs.iter().all(|c| !c.eligible) { return json!({"error": "No legal career to plan."}); }
    let curs: Vec<Cur> = cs.iter().map(|c| Cur { t: c.cur.t, a: c.cur.a, b: c.cur.b, p: c.cur.p, os: c.cur.os.clone(), wp: c.cur.wp.clone(), rot: false }).collect();
    let elig: Vec<bool> = cs.iter().map(|c| c.eligible).collect();
    let est = s.plan_estimate(&st0, &curs, &elig, pj);
    // the pick: among the leader and the eligible candidates not separated from it, the highest P(all) now
    let lead = est.leader;
    let same: Vec<usize> = match lead { None => vec![], Some(l) => (0..cs.len()).filter(|&i| elig[i] && !est.xs[i].is_empty() && (i == l || !separated(&est.xs[i], &est.xs[l]))).collect() };
    let pick = same.iter().copied().max_by(|&x, &y| cs[x].cur.p.partial_cmp(&cs[y].cur.p).unwrap().then(y.cmp(&x)))
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
    ord.sort_by(|&x, &y| {
        let (a, b) = (&est.xs[x], &est.xs[y]);
        match (a.is_empty(), b.is_empty()) { (false, false) => mean(a).partial_cmp(&mean(b)).unwrap(), (e1, e2) => e1.cmp(&e2) }
    });
    let lx: Option<&[f64]> = lead.map(|l| est.xs[l].as_slice());
    let opts: Vec<Value> = ord.iter().map(|&i| {
        let mut r = row(env, pool, &cs[i], &est.xs[i], lx);
        let o = r.as_object_mut().unwrap();
        o.insert("chosen".into(), json!(pick == Some(i)));
        o.insert("about_same".into(), json!(same.len() > 1 && same.contains(&i)));
        o.insert("leader".into(), json!(lead == Some(i)));
        r
    }).collect();
    let pos = |i: Option<usize>| i.and_then(|i| ord.iter().position(|&x| x == i));
    let gi = cs.iter().position(|c| c.greedy);
    let greedy_expected = gi.map(|i| { let (m, h) = ci(&est.xs[i]); json!({"option": pos(Some(i)), "expected": if est.xs[i].is_empty() { Value::Null } else { json!(m) }, "ci": h, "samples": est.xs[i].len(), "p_all": cs[i].cur.p}) });
    let choice = pick.map(|i| env.report(pool, &cs[i].f, true));
    json!({"options": opts, "pick": pos(pick), "leader": pos(lead), "about_same": same.len() > 1, "choice": choice, "greedy_expected": greedy_expected,
           "stop": est.stop, "sampling_ms": est.ms, "init_ms": init_ms, "search_ms": tm, "total_ms": now_ms() - t_all, "continuations": est.xs.iter().map(|x| x.len()).sum::<usize>(),
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
    let mut s = match sim_for(env, d, sk, cr, job, pj) { Ok(s) => s, Err(e) => return e };
    let budget0 = s.env.budget;
    if pj.refresh_work > 0 { s.env.budget = pj.refresh_work; }
    let st0 = s.plan_initial(legs, &[&sv.refs, stay_refs]);
    let cs = vec![s.cand(pool, stay, true, false, "stay"), s.cand(pool, adv, true, false, "advance")];
    let curs: Vec<Cur> = cs.iter().map(|c| Cur { t: c.cur.t, a: c.cur.a, b: c.cur.b, p: c.cur.p, os: c.cur.os.clone(), wp: c.cur.wp.clone(), rot: false }).collect();
    let est = s.plan_estimate(&st0, &curs, &[true, true], pj);
    s.env.budget = budget0;
    drop(s);
    let one = |i: usize| { let (m, h) = ci(&est.xs[i]); json!({"expected": if est.xs[i].is_empty() { Value::Null } else { json!(m) }, "ci": h, "samples": est.xs[i].len(), "p_all": cs[i].cur.p}) };
    let diff = paired(&est.xs[1], &est.xs[0]);
    let about_same = diff.map_or(true, |(m, h)| m.abs() <= h);
    let better = if about_same { Value::Null } else if diff.unwrap().0 < 0.0 { json!("advance") } else { json!("stay") };
    json!({"stay": one(0), "advance": one(1), "advance_minus_stay": diff.map(|(m, h)| json!([m, h])), "about_same": about_same, "better": better,
           "stop": est.stop, "sampling_ms": est.ms, "total_ms": now_ms() - t_all, "careers_simulated": est.careers,
           "horizon_max": pj.horizon_max, "eps": pj.eps})
}
