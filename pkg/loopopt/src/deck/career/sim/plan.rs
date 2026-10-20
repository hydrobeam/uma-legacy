//! The page's next career (`next` on a career job, `index_rule`; docs/SOLVER.md "Next career"): the career that brings
//! a fully looped parent closest per run, scored exactly over every result she can roll, with each candidate's
//! continuation (`Cont`) fitted to the pool. It values careers through the simulator's deck cache (`Sim`, `Cache`).
use super::*;


/// Each trainee's best career (`Env::solve`, one trainee, top 1) and the decks that search checked, per plan: kept for
/// the next plan of the same setup and places whose pool is this one plus one veteran at the end (the page's Compare
/// and Reroll: your veterans and her). Every career of the larger pool is one of this pool's or has her as a parent, so
/// the best is the better of the kept one and the best with her as a parent (a search with her fixed): exact, the same
/// value; on an exact tie the kept career stands.
pub(in super::super) struct Bests { key: (u64, u64), best: Vec<Option<Option<Found>>>, refs: Vec<Vec<Vec<Ref>>>,
    /// her best looped career (`index_rule`), where searched: kept the same way (a pool plus one veteran adds looped pairs
    /// only when she is an 8/8)
    looped: Vec<Option<Option<Found>>>,
    /// per uma whether the search behind her kept best (and her looped best) ran out of work (`Solved::complete` false)
    unp: Vec<bool>, lunp: Vec<bool> }
thread_local! { static BESTS: std::cell::RefCell<Vec<Bests>> = const { std::cell::RefCell::new(Vec::new()) }; }
/// the kept plans for the page to store (`state_out`): per plan its key and every uma's kept best (her decks' refs are
/// not kept: nothing reads them back)
pub(in super::super) fn bests_out() -> Value {
    BESTS.with(|c| Value::Array(c.borrow().iter().map(|b| json!({"k": [super::super::ux(b.key.0), super::super::ux(b.key.1)], "n": b.best.len(),
        "b": b.best.iter().enumerate().filter_map(|(u, x)| x.as_ref().map(|f| json!([u, f.as_ref().map(super::super::found_out)]))).collect::<Vec<_>>(),
        "l": b.looped.iter().enumerate().filter_map(|(u, x)| x.as_ref().map(|f| json!([u, f.as_ref().map(super::super::found_out)]))).collect::<Vec<_>>(),
        "u": (0..b.unp.len()).filter(|&u| b.unp[u]).collect::<Vec<_>>(), "lu": (0..b.lunp.len()).filter(|&u| b.lunp[u]).collect::<Vec<_>>()})).collect()))
}
pub(in super::super) fn bests_in(v: &Value) -> Option<Vec<Bests>> {
    use super::super::{found_in, uxr};
    v.as_array()?.iter().map(|e| {
        let k = e["k"].as_array()?; let n = e["n"].as_u64()? as usize;
        let per_uma = |key: &str| -> Option<Vec<Option<Option<Found>>>> {
            let mut v: Vec<Option<Option<Found>>> = vec![None; n];
            for x in e[key].as_array()? {
                let a = x.as_array()?; let u = a.first()?.as_u64()? as usize;
                if u >= n { return None; }
                v[u] = Some(if a.get(1)?.is_null() { None } else { Some(found_in(a.get(1)?)?) });
            }
            Some(v)
        };
        // (absent in a state stored before them: proven)
        let flags = |key: &str| -> Vec<bool> { let mut v = vec![false; n]; for x in e[key].as_array().into_iter().flatten() { if let Some(u) = x.as_u64() { if (u as usize) < n { v[u as usize] = true; } } } v };
        Some(Bests { key: (uxr(k.first()?)?, uxr(k.get(1)?)?), best: per_uma("b")?, refs: vec![vec![]; n], looped: per_uma("l")?, unp: flags("u"), lunp: flags("lu") })
    }).collect()
}
pub(in super::super) fn bests_set(b: Vec<Bests>) { BESTS.with(|c| *c.borrow_mut() = b); }
/// the kept plans, taken out (the simulator's `index` policy starts each run without them, `sim::kept_swap`)
pub(in super::super) fn bests_take() -> Vec<Bests> { BESTS.with(|c| std::mem::take(&mut *c.borrow_mut())) }
fn h64(s: &str) -> u64 { use std::hash::{Hash, Hasher}; let mut h = std::collections::hash_map::DefaultHasher::new(); s.hash(&mut h); h.finish() }
/// (setup and places, the first `n` legacies)
fn bests_key(job: &Value, fix: &[usize], n: usize) -> (u64, u64) {
    let ls = job.get("legacies").and_then(|l| l.as_array()).map(|a| Value::Array(a[..n.min(a.len())].to_vec()).to_string()).unwrap_or_default();
    (crate::deck::career::setup_fp(job) ^ h64(&format!("{fix:?}")).rotate_left(1), h64(&ls))
}
/// trainee u's best career in `pool` (and the refs to absorb, and whether every search behind it was complete: false
/// when one ran out of its work cap, `Solved::complete`), reusing a kept plan of the pool less its last veteran
fn best_of(env: &mut Env, pool: &Pool, fix: &[usize], u: usize, same: Option<&Bests>, prev: Option<&Bests>, nl: usize) -> (Option<Found>, Vec<Vec<Ref>>, bool) {
    // a kept plan of this very pool (the same job planned again: a reload, a setting changed and back): her best, as kept
    if let Some(sm) = same { if let Some(Some(x)) = sm.best.get(u) { return (x.clone(), vec![], !sm.unp.get(u).copied().unwrap_or(false)); } }
    if let Some(pr) = prev { if let Some(Some(old)) = pr.best.get(u) {
        let mut refs = pr.refs.get(u).cloned().unwrap_or_default();
        let kept_ok = !pr.unp.get(u).copied().unwrap_or(false);
        if fix.len() >= 2 { return (old.clone(), refs, kept_ok); }   // both parents fixed: she can't be one
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
            if ub * (1.0 + 1e-9) < o.val { return (Some(o.clone()), refs, kept_ok); }
        }
        // only a career above the kept one matters: the search's bar starts there (exact; a tie keeps the kept one)
        env.floor = old.as_ref().map_or(f64::NEG_INFINITY, |o| o.val); let sr = env.solve(pool, &fx, &[u], false, 1); env.floor = f64::NEG_INFINITY;
        if refs.len() < sr.refs.len() { refs.resize(sr.refs.len(), vec![]); }
        for (t, r) in sr.refs.into_iter().enumerate() { refs[t].extend(r); }
        let ok = kept_ok && sr.complete;
        let new = sr.found.into_iter().next();
        let best = match (old, new) { (Some(o), Some(n)) => Some(if n.val > o.val { n } else { o.clone() }), (o, n) => o.clone().or(n) };
        return (best, refs, ok);
    } }
    let sr = env.solve(pool, fix, &[u], false, 1);
    let ok = sr.complete;
    (sr.found.into_iter().next(), sr.refs, ok)
}
/// What `score_career` reads that grows while a plan runs: the deck cache's decks and fully checked decks (`Cache`,
/// per trainee, both only grow) summed over the trainees `us` (None: every trainee), and the deck searches run so far
/// (`deck_for`'s: the only work in a scoring that changes what a later one reads)
fn sver(s: &Sim, us: Option<&[usize]>) -> (usize, usize, usize) {
    let (a, b) = match us {
        Some(us) => us.iter().fold((0, 0), |(a, b), &u| (a + s.cache.decks[u].len(), b + s.cache.refs[u].len())),
        None => (s.cache.n_decks(), s.cache.refs.iter().map(|r| r.len()).sum()),
    };
    (a, b, s.n.searches)
}
/// A scoring with its stamp: the cache it read (`sver` over `us`, the trainees whose decks it reads) when it ran no deck
/// search and added nothing, else None. A scoring of the same career against the same base, run when `sver` is again
/// the stamp, reads the same decks and tables, takes the same branches (whether `deck_for` searches is decided by those
/// alone) and gives the same f64s (`stamp_ok`), so it need not run
fn stamped<R>(s: &mut Sim, us: Option<&[usize]>, f: impl FnOnce(&mut Sim) -> R) -> (R, Option<(usize, usize)>) {
    let v0 = sver(s, us); let r = f(s); let v1 = sver(s, us);
    (r, (v0 == v1).then_some((v1.0, v1.1)))
}
fn stamp_ok(s: &Sim, us: Option<&[usize]>, st: Option<(usize, usize)>) -> bool { let v = sver(s, us); st == Some((v.0, v.1)) }
/// the same career as `score_career` reads it: trainee, parents in order, every field of the deck (floats by bits). Not
/// its value (`Found::val`, nor `ub`): `score_career` never reads them, and a proxy's value (the surrogate's corner-table
/// interpolation) and the search's exact one differ in their last bits
fn same_career(f: &Found, g: &Found) -> bool {
    let bits = |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits());
    f.t == g.t && f.a == g.a && f.b == g.b && f.sel.ents == g.sel.ents && f.sel.plain == g.sel.plain
        && f.sel.filler == g.sel.filler && bits(&f.sel.dw, &g.sel.dw) && bits(&f.sel.dg, &g.sel.dg)
}
fn p_margin() -> f64 { 1e9 }
fn p_swork() -> usize { 5_000_000 }

#[derive(Deserialize, Clone)]
pub(in super::super) struct PlanJob {
    /// the surrogate's refresh margin (`sim` module notes)
    #[serde(default = "p_margin")] margin: f64,
    /// work budget (career-search units) of each candidate search beyond the job's own (also each loop bootstrap
    /// search), and of each deck search a continuation makes (a refresh); 0 = the job's budget
    #[serde(default = "p_swork")] search_work: usize,
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



/// The loop planner's best loop for the job's settings (a deck + loop search, top 1, per-uma decks): its members as
/// {id, name, short}, in rotation order. Cached per setup (the job without its pool and places).
fn best_loop(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, job: &Value) -> Result<Vec<Value>, String> {
    thread_local! { static LOOPS: std::cell::RefCell<Vec<(u64, Result<Vec<Value>, String>)>> = const { std::cell::RefCell::new(Vec::new()) }; }
    let mut j = job.clone();
    let Some(o) = j.as_object_mut() else { return Err("bad job".into()) };
    for k in ["legacies", "trainees", "fix", "per_trainee", "simulate", "top", "next", "compare"] { o.remove(k); }
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

impl<'s, 'a> Sim<'s, 'a> {
}


fn sim_for<'s, 'a>(env: &'s mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, pj: &PlanJob) -> Result<Sim<'s, 'a>, Value> {
    let ntar = env.ntar;
    if ntar > 12 || env.nu > 10 { return Err(json!({"error": "The plan takes at most 12 targets, 10 of them not guaranteed by the scenario."})); }
    let cfg: SimJob = serde_json::from_value(json!({"margin": pj.margin})).unwrap();
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
             fastc, fixed, main: false, audits: vec![], fill: true, mi: vec![] })
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
}





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
    let bar = f.val * (1.0 + 1e-9);
    // per pair: the lowest of the bounds above the bar, as none of them at or below it (each bound is computed alone, so
    // stopping at the first that proves changes no number); f64::min passes over a NaN, so a NaN bound proves nothing
    // and the pair fails only when some bound is a number (`num`)
    for a in 0..nl { 'pairs_b: for b in a + 1..nl {
        if !legal_pair(d, pool, x, a, b) || !fix.iter().all(|&g| g == a || g == b) { continue; }
        let (q, _, c) = env.lineage(pool, x, a, b);
        let Some(ub) = env.ceiling(x, &q, c) else { continue };
        if ub <= bar { continue; }
        let mut num = !ub.is_nan();
        for r in &env.seen[ct] { let v = c * r.val * env.rise(x, &q, &r.q); if v <= bar { continue 'pairs_b; } num |= !v.is_nan(); }
        for (rq, sv) in &env.ruled[ct] { let v = env.space.rho_top() * c * sv * env.rise(x, &q, rq); if v <= bar { continue 'pairs_b; } num |= !v.is_nan(); }
        if num { return false; }
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

/// Scores within this many careers per run of the best (or 5% of it) count as tied
const NEAR: f64 = 0.25;
/// Builders through their deck class (`index_rule`): a proxy is searched exactly when her lift efficiency is within this
/// share of a candidate's second best, or her saving within this share of the lowest saving the shown rows or the pick's
/// near-tie band take. Recalibrated 2026-10-09 (val/plan `proxy_check`, with the own-8/8 credit): the proxy is
/// the exact career for every builder on five of the seven jobs; it reads at most 2.79% low on a contender's efficiency
/// (exact at least 0.7 of a candidate's second best; p11) and 2.67% low on a builder's saving (p11): 0.06 is 2.15x and
/// 2.25x those. (The first measurement, on the planner as it was before the class winners and the merges since, had 11%
/// and 9%, with 0.2.) The errors move with the model: run `proxy_check` again after a change to what builders' careers
/// are made of
const PROXY_EFF: f64 = 0.06;
const PROXY_SAVE: f64 = 0.06;
/// the rows the page shows (career.js `OPT_SHOW`): their careers are searched exactly
const OPT_SHOW: usize = 8;
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
/// Both constants fitted (val/sim/README.md E5) to traces of building for Agnes Digital on 2-, 5-, 11- and
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
fn her_trios(s: &mut Sim, d: &Data, bs: &ScoreBase, p8: &mut Pool, iv: usize, x: usize, buf: &mut Vec<f64>) -> Vec<(usize, bool, f64)> {
    let her_id = d.umas[x].id; let mut out = vec![];
    // the pool with her (`her_pool`: `Env::pool` of the legacies and hers), with each side pushed in turn (the same Pool
    // as `Env::pool` of those and the side) and popped after
    let il = p8.me.len();
    for ib in 0..bs.cands.len() {
        let pu = bs.cands[ib]; if d.umas[pu].id == her_id { continue; }
        for (real, leg) in bs.sides.get(ib).map_or(&[][..], |v| &v[..]) {
            s.env.pool_push(p8, leg);
            let mut cs: Vec<(f64, usize)> = bs.all.iter().copied()
                .filter(|&cu| d.umas[cu].id != her_id && d.umas[cu].id != d.umas[pu].id && legal_pair(d, &*p8, cu, iv, il))
                .filter_map(|cu| { let (q, _, c) = s.env.lineage(&*p8, cu, iv, il); s.env.ceiling(cu, &q, c).map(|ub| (ub, cu)) }).collect();
            cs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let mut best = 0.0f64;
            for (ub, cu) in cs { if ub * (1.0 + 1e-9) <= best { break; } let v = val(s, &*p8, cu, iv, il, buf); if v > best { best = v; } }
            if best > 0.0 { out.push((ib, *real, 1.0 / best)); }
            s.env.pool_pop(p8);
        }
    }
    out
}
/// V after her with her own 8/8 a trio side (`her_trios`), at the candidates' step costs c1 (a real partner side free)
fn with_her(v: f64, her: &[(usize, bool, f64)], c1: &[f64]) -> f64 {
    her.iter().map(|&(ib, real, c3)| (if real { 0.0 } else { c1[ib] }) + c3).fold(v, f64::min)
}
/// Score of career x (parents a, b of `legs`, deck `sel`) against `bs`: E[V0 - V0 with her veteran], the chance it is
/// above 0, Y per outcome (only with `want_ys`; else empty), and per candidate (P(her veteran lifts her), the summed lifted odds). Exact over her landed set (`outcome_sets`) and her stars per uncertain target (the
/// rank's mix): her veteran as a parent of each stage-1 candidate (her best IDX_PARTNERS partners, the trainee's cached
/// decks' corner tables, folded one target at a time); her own 8/8 ends her stage when she is a candidate.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
/// `Env::pool` of `legs` and her veteran (career x with parents a, b, schedule `c`): with `of_legs` (`pool` is
/// `Env::pool(legs)`) `pool` with her entry pushed, the same Pool (`Env::pool_push` builds an entry as `Env::pool` does)
#[allow(clippy::too_many_arguments)]
fn her_pool(s: &Sim, legs: &[Legacy], pool: &Pool, of_legs: bool, x: usize, a: usize, b: usize, c: &Cur) -> Pool {
    let v = s.env.veteran(legs, x, a, b, c.wp.to_vec());
    if of_legs { debug_assert_eq!(pool.me.len(), legs.len()); let mut pv = pool.clone(); s.env.pool_push(&mut pv, &v); pv }
    else { let mut lv = legs.to_vec(); lv.push(v); s.env.pool(&lv) }
}
/// `pool` must be `Env::pool(legs)` (the planner's callers): the pool with her veteran is then that pool with her entry
/// pushed (`Env::pool_push` builds an entry as `Env::pool` does). `score_career_of` takes any pool.
pub(super) fn score_career(s: &mut Sim, d: &Data, bs: &ScoreBase, legs: &[Legacy], pool: &Pool, x: usize, a: usize, b: usize, sel: &Sel, want_ys: bool, buf: &mut Vec<f64>) -> (f64, f64, Vec<f64>, Vec<(f64, f64)>) {
    let t_sc = now_ms(); let r = score_career_(s, d, bs, legs, pool, true, x, a, b, sel, want_ys, buf); prof("score_ms", now_ms() - t_sc); prof("score_calls", 1.0); r
}
/// `score_career` for a pool built otherwise (`sim`'s): the pool with her veteran built from `legs` and her
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub(super) fn score_career_of(s: &mut Sim, d: &Data, bs: &ScoreBase, legs: &[Legacy], pool: &Pool, x: usize, a: usize, b: usize, sel: &Sel, want_ys: bool, buf: &mut Vec<f64>) -> (f64, f64, Vec<f64>, Vec<(f64, f64)>) {
    let t_sc = now_ms(); let r = score_career_(s, d, bs, legs, pool, false, x, a, b, sel, want_ys, buf); prof("score_ms", now_ms() - t_sc); prof("score_calls", 1.0); r
}
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn score_career_(s: &mut Sim, d: &Data, bs: &ScoreBase, legs: &[Legacy], pool: &Pool, of_legs: bool, x: usize, a: usize, b: usize, sel: &Sel, want_ys: bool, buf: &mut Vec<f64>) -> (f64, f64, Vec<f64>, Vec<(f64, f64)>) {
    let n = bs.cands.len(); let ntar = s.env.ntar; let nu = s.env.uncv.len();
    let uncv = s.env.uncv.clone(); let fixed_t = s.env.fixed_t.clone(); let w = s.env.sj.star_dist;
    let xi = bs.cands.iter().position(|&u| u == x);
    // two 8/8 parents: her own 8/8 is a fully looped parent (the goal: V after is 0)
    let goal = is_full(s.env.sj, &legs[a].sparks) && is_full(s.env.sj, &legs[b].sparks);
    let t_mc = now_ms();
    let c = s.make_cur(pool, x, a, b, sel, false);
    let t_lv = now_ms(); prof("sc_cur_ms", t_lv - t_mc);
    let mut pv = her_pool(s, legs, pool, of_legs, x, a, b, &c); let iv = pv.me.len() - 1;
    // where she can be a parent: per stage-1 candidate (stage 1 still to run) her best partners
    let t_su = now_ms(); prof("sc_pool_ms", t_su - t_lv);
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
    prof("sc_setup_ms", now_ms() - t_su); let t_fo = now_ms();
    // per setup, per star pattern of the uncertain targets (base 4, position k = uncv[k]; 0 absent), the best table value
    // at scale 1: the tables folded one coordinate at a time (interp's order and arithmetic), every setup's tables at once
    // (`fold_rec`); per outcome (leaf, guaranteed golds fm) and lifted candidate, that value times the setup's spark scale,
    // the best over her setups and her odds now (`fold_emit`): the products and maxima the outcome loop took per outcome
    let nleaf = 1usize << (2 * nu);
    let ns = setups.len();
    let nf = fixed_t.len();
    // per setup and landed set m, the spark scale (the product the loop below took per outcome, the same order)
    let nm = 1usize << ntar;
    // (m-major: scf[m * ns + si])
    let mut scf: Vec<f64> = Vec::with_capacity(nm * ns);
    for m in 0..nm { scf.extend(setups.iter().map(|st| (0..ntar).map(|k| st.pw[k][(m >> k & 1) as usize]).product::<f64>())); }
    let fmm: Vec<usize> = (0..(1usize << nf)).map(|fm| { let mut m = 0; for (j, &i) in fixed_t.iter().enumerate() { if fm >> j & 1 == 1 { m |= 1 << i; } } m }).collect();
    // the candidates with setups (candidate order): her slot per candidate, per setup, and odds now per slot
    let mut aslot: Vec<Option<usize>> = vec![None; n]; let mut p0: Vec<f64> = vec![];
    for st in &setups { if aslot[st.ui].is_none() { aslot[st.ui] = Some(p0.len()); p0.push(bs.pnow[st.ui]); } }
    let na = p0.len();
    let sslot: Vec<usize> = setups.iter().map(|st| aslot[st.ui].unwrap()).collect();
    // (reused across calls: every row is written before it is read)
    let mut sc = FOLD_SCRATCH.with(|c| std::mem::take(&mut *c.borrow_mut()));
    let pl = (1usize << nf) * na > ns;
    let nrow = if pl { nleaf * ns } else { (nleaf << nf) * na };
    if sc.u.len() < nrow { sc.u.resize(nrow, 0.0); }
    if na > 0 { sc_fold(s, bs, &setups, &mut sc, pl, nu, nf, ns, na, nrow, &uncv, &fmm, &scf, &sslot, &p0); }
    prof("sc_fold_ms", now_ms() - t_fo);
    // the candidates the loop below can lift, in candidate order: those with setups, and (never, as the odds are not
    // negative) one whose odds now already clear her threshold pnow (1 + 1e-9); every other keeps up = pnow, below it
    let thr: Vec<f64> = (0..n).map(|ui| bs.pnow[ui] * (1.0 + 1e-9)).collect();
    let active: Vec<usize> = (0..n).filter(|&ui| aslot[ui].is_some() || bs.pnow[ui] > thr[ui]).collect();
    // her own 8/8 as a trio side when her uma is not a stage candidate (the full set only; `her_trios`)
    let full_m = (1usize << ntar) - 1;
    let t_h = now_ms();
    let her: Vec<(usize, bool, f64)> = if xi.is_none() && !goal && !bs.sides.is_empty() && c.os[full_m] > 0.0 { her_trios(s, d, bs, &mut pv, iv, x, buf) } else { vec![] };
    prof("sc_her_ms", now_ms() - t_h); let t_lo = now_ms();
    // per active candidate: (index, her slot or none, her threshold, her odds now)
    let act: Vec<(usize, Option<usize>, f64, f64)> = active.iter().map(|&ui| (ui, aslot[ui], thr[ui], bs.pnow[ui])).collect();
    let (ey, lift, ys, stats, [n_out, n_ch, n_cost, n_same, n_one]) = {
        let FoldScratch { u, pst, mus, .. } = &mut sc;
        sc_outcomes(bs, &c.os, w, &uncv, &fmm, &u[..nrow], &scf, &sslot, &p0, &act, &her, xi, goal, pl, want_ys, nu, nf, ns, na, full_m, pst, mus)
    };
    let t_end = now_ms();
    FOLD_SCRATCH.with(|c| *c.borrow_mut() = sc);
    prof("sc_loop_ms", t_end - t_lo); prof("sc_out", n_out as f64); prof("sc_trio", n_ch as f64); prof("sc_cost", n_cost as f64);
    prof("sc_same", n_same as f64); prof("sc_one", n_one as f64);
    (ey, lift, ys, stats)
}

/// `score_career_`'s fold (with na > 0 setup slots): every setup's tables, the dominated ones dropped, folded (`fold_rec`)
/// into the outcome rows `sc.u`. Its own function, entered once per scoring, so V8 compiles it optimised after the first
/// few (`docs/PERF.md`, *In the page*)
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn sc_fold(s: &Sim, bs: &ScoreBase, setups: &[Setup], sc: &mut FoldScratch, pl: bool, nu: usize, nf: usize, ns: usize, na: usize, nrow: usize,
    uncv: &[usize], fmm: &[usize], scf: &[f64], sslot: &[usize], p0: &[f64]) {
    // per setup, per target and star level: x and 1 - x (the same expressions the fold took per branch)
    let xs: Vec<Vec<[f64; 4]>> = setups.iter().map(|st| (0..nu).map(|k| [0usize, 1, 2, 3].map(|sg| st.x0[k] * st.xm[sg])).collect()).collect();
    let xc: Vec<Vec<[f64; 4]>> = xs.iter().map(|v| v.iter().map(|r| r.map(|x| 1.0 - x)).collect()).collect();
    // every setup's tables, level 0 of the fold (table j at j 2^nu), and the setup of each
    let mut tsi: Vec<usize> = vec![];
    if sc.lv.len() < nu + 1 { sc.lv.resize(nu + 1, vec![]); }
    sc.lv[0].clear();
    let mut n_dom = 0usize;
    for (si, st) in setups.iter().enumerate() {
        let tabs: Vec<&Vec<f64>> = s.cache.decks[bs.cands[st.ui]][..st.nd].iter().flat_map(|dk| dk.tab.iter()).collect();
        // a table at most another everywhere (an equal one: the later) folds to at most that one's value at every leaf
        // (every fold coefficient is >= 0 and rounding is monotone), so the maximum is the same without it
        let nonneg = xs[si].iter().chain(&xc[si]).all(|r| r.iter().all(|&v| v >= 0.0));
        for (j, tab) in tabs.iter().enumerate() {
            debug_assert_eq!(tab.len(), 1usize << nu);
            if nonneg && tabs.iter().enumerate().any(|(k, o)| k != j && tab.iter().zip(o.iter()).all(|(a, b)| a <= b) && (k < j || tab.iter().zip(o.iter()).any(|(a, b)| a < b))) { n_dom += 1; continue; }
            sc.lv[0].extend_from_slice(tab); tsi.push(si);
        }
    }
    prof("sc_dom", n_dom as f64);
    let nt = tsi.len();
    for k in 1..=nu { let l = nt << (nu - k); if sc.lv[k].len() < l { sc.lv[k].resize(l, 0.0); } }
    if sc.fv.len() < 16 * ns { sc.fv.resize(16 * ns, 0.0); }
    if sc.acc.len() < 16 * na { sc.acc.resize(16 * na, 0.0); }
    let cx = FoldCx { pl, nu, nf, ns, na, tsi: &tsi, xs: &xs, xc: &xc, uncv, fmm, scf, sslot, p0 };
    let FoldScratch { lv, fv, acc, u, .. } = sc;
    fold_rec(&cx, 0, 0, 0, lv, fv, &mut acc[..16 * na], &mut u[..nrow]);
}
/// `score_career_`'s outcome loop: every outcome (her stars per uncertain target, the guaranteed golds present or not),
/// its chance, V after her and y; (E[y], P(y > 1e-9), y per outcome with `want_ys`, per candidate stats, counters). Its
/// own function, entered once per scoring, so V8 compiles it optimised after the first few (`docs/PERF.md`, *In the page*)
#[inline(never)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn sc_outcomes(bs: &ScoreBase, os: &[f64], w: [f64; 3], uncv: &[usize], fmm: &[usize], u: &[f64], scf: &[f64], sslot: &[usize], p0: &[f64],
    act: &[(usize, Option<usize>, f64, f64)], her: &[(usize, bool, f64)], xi: Option<usize>, goal: bool, pl: bool, want_ys: bool,
    nu: usize, nf: usize, ns: usize, na: usize, full_m: usize, pst: &mut Vec<f64>, mus: &mut Vec<usize>) -> (f64, f64, Vec<f64>, Vec<(f64, f64)>, [usize; 5]) {
    let n = bs.cands.len(); let nleaf = 1usize << (2 * nu);
    let v0_after = |c1: &[f64]| if bs.combos.is_empty() { (0..n).map(|a| c1[a] + bs.m23[a]).fold(f64::INFINITY, f64::min) }
        else { trio_v(c1, &bs.combos) };
    // every outcome: uncertain targets' star pattern, guaranteed golds present or not
    let (mut ey, mut lift) = (0.0f64, 0.0f64);
    // y per outcome (careers saved), at leaf << nf | fm (0 where the outcome can't happen); only when the caller reads it
    let mut ys: Vec<f64> = if want_ys { vec![0.0; nleaf << nf] } else { vec![] };
    let mut c1 = bs.c1.clone();
    // per stage-1 candidate: the chance her veteran raises that candidate's odds, and the odds raised to (summed, weighted)
    let mut stats = vec![(0.0f64, 0.0f64); n];
    // V after her when no candidate's step moves (most outcomes lift no one): once
    let v_same = v0_after(&bs.c1);
    let mut ch: Vec<usize> = vec![];
    let (mut n_out, mut n_ch, mut n_cost, mut n_same, mut n_one) = (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut ubuf = vec![0.0f64; na];
    // V after her when one candidate's step alone moved (`one_of`, `one_v`: trio_v's minimum, split by the trios that read
    // her step)
    let one: Vec<One> = if bs.combos.is_empty() { vec![] } else { (0..n).map(|i| one_of(&bs.combos, &bs.c1, i)).collect() };
    // per candidate, odds costed and their cost (`bs.cost` is a pure function of them), direct-mapped by the odds' bits
    const CC: usize = 256;
    let mut last: Vec<(u64, f64)> = vec![(f64::NAN.to_bits(), 0.0); act.len() * CC];
    // per leaf her landed set on the uncertain targets and her stars' chance: the product over k of w[her stars on target
    // k - 1] where she has any, in k order from 1.0 (the leaves with digit k > 0 extend those below 4^k)
    pst.clear(); pst.push(1.0); mus.clear(); mus.push(0usize);
    for k in 0..nu {
        let lo = 1usize << (2 * k);
        for dg in 1..4usize { for i in 0..lo { pst.push(pst[i] * w[dg - 1]); mus.push(mus[i] | 1 << uncv[k]); } }
    }
    for leaf in 0..nleaf {
        let (mu, pr_st) = (mus[leaf], pst[leaf]);
        for (fm, &mf) in fmm.iter().enumerate() {
            let m = mu | mf;
            let pr = os[m] * pr_st;
            if pr <= 0.0 { continue; }
            n_out += 1;
            // each candidate's odds after her: the best of her setups' and her odds now (`fold_emit`; with pl, here)
            let ur: &[f64] = if pl {
                ubuf.copy_from_slice(p0);
                let (fr, sr) = (&u[leaf * ns..(leaf + 1) * ns], &scf[m * ns..(m + 1) * ns]);
                for ((&sc, &f), &sl) in sr.iter().zip(fr).zip(sslot) { let v = sc * f; if v > ubuf[sl] { ubuf[sl] = v; } }
                &ubuf
            } else { &u[(leaf << nf | fm) * na..][..na] };
            // the candidates whose step moved (if none, c1 is the base's and V after is v_same)
            ch.clear();
            for (ai, &(ui, sl, th, pn)) in act.iter().enumerate() {
                let upv = match sl { Some(sl) => ur[sl], None => pn };
                if upv > th {
                    stats[ui].0 += pr; stats[ui].1 += pr * upv;
                    let ub = upv.to_bits(); let ci = ai * CC + (ub.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as usize;
                    let cc = if last[ci].0 == ub { n_same += 1; last[ci].1 } else { n_cost += 1; let cc = bs.cost(ui, upv); last[ci] = (ub, cc); cc };
                    if cc < c1[ui] { c1[ui] = cc; ch.push(ui); } }
            }
            if m == full_m { if let Some(xi) = xi { c1[xi] = 0.0; ch.push(xi); } }
            if !ch.is_empty() { n_ch += 1; }
            let mut va = if ch.is_empty() { v_same }
                else if ch.len() == 1 && !one.is_empty() && one[ch[0]].ok { n_one += 1; one_v(&one[ch[0]], c1[ch[0]]) }
                else { v0_after(&c1) };
            if m == full_m && !her.is_empty() { va = with_her(va, her, &c1); }
            // c1 back to the base's
            for &i in &ch { c1[i] = bs.c1[i]; }
            let y = if goal && m == full_m { bs.v0 } else { (bs.v0 - va).max(0.0) };
            ey += pr * y; if y > 1e-9 { lift += pr; }
            if want_ys { ys[leaf << nf | fm] = y; }
        }
    }
    (ey, lift, ys, stats, [n_out, n_ch, n_cost, n_same, n_one])
}
/// `trio_v` with one candidate's step changed (to x, from the base's c1), per candidate i: the minimum over the trios that
/// do not read her step (their values at the base's steps, trio_v's expression), and the trios that read it once, as
/// (key = rest + C's step, rest, C's step), key ascending: a trio's value (x + rest) + C's step, the same float as trio_v's
/// sum (addition commutes exactly). Not `ok` when a trio reads her step twice
struct One { not: f64, rd: Vec<(f64, f64, f64)>, ok: bool }
fn one_of(combos: &[Trio], c1: &[f64], i: usize) -> One {
    let (mut not, mut rd, mut ok) = (f64::INFINITY, vec![], true);
    for &(ia, ib, c3, ra, rb) in combos {
        let (av, bv) = (if ra { 0.0 } else { c1[ia] }, if rb { 0.0 } else { c1[ib] });
        match (ia == i && !ra, ib == i && !rb) {
            (false, false) => { let v = av + bv + c3; if v < not { not = v; } }
            (true, false) => rd.push((bv + c3, bv, c3)),
            (false, true) => rd.push((av + c3, av, c3)),
            (true, true) => ok = false,
        }
    }
    rd.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap());
    One { not, rd, ok }
}
/// trio_v at the base's steps with candidate i's at x (`One`): the minimum, the reading trios scanned by key until no
/// later one can be below it. Derived: every term is >= 0, so a trio's value is at least fl(x + key) (1 - u)^5 (u the
/// unit roundoff; key is rounded once, x + key once, the value's two sums once each) > fl(x + key) (1 - 1e-9), and keys
/// only rise; a trio at or above the minimum leaves it unchanged (min is order-free)
fn one_v(o: &One, x: f64) -> f64 {
    let mut best = o.not;
    for &(key, rest, c3) in &o.rd {
        if (x + key) * (1.0 - 1e-9) >= best { break; }
        let v = (x + rest) + c3; if v < best { best = v; }
    }
    best
}
/// `score_career`'s fold buffers, kept between calls (each call writes what it reads): per level the folded tables, the
/// last two levels' 16 leaves per setup and per candidate slot, the outcome rows, per leaf her stars' chance and set
#[derive(Default)]
struct FoldScratch { lv: Vec<Vec<f64>>, fv: Vec<f64>, acc: Vec<f64>, u: Vec<f64>, pst: Vec<f64>, mus: Vec<usize> }
thread_local! { static FOLD_SCRATCH: std::cell::RefCell<FoldScratch> = std::cell::RefCell::new(FoldScratch::default()); }
/// What `fold_rec` reads: per table its setup, per setup x and 1 - x per uncertain target and star level; for
/// `fold_emit`: whether it writes per leaf the setups' table values (pl: fewer rows than per outcome and slot; the outcome
/// loop then takes the products and maxima itself), else per outcome the slots' odds; the guaranteed golds' sets, the spark scales (m-major), each setup's candidate slot, the slots' odds now
struct FoldCx<'f> { pl: bool, nu: usize, nf: usize, ns: usize, na: usize, tsi: &'f [usize], xs: &'f [Vec<[f64; 4]>], xc: &'f [Vec<[f64; 4]>],
    uncv: &'f [usize], fmm: &'f [usize], scf: &'f [f64], sslot: &'f [usize], p0: &'f [f64] }
/// One leaf's rows (each fm; her landed set mu on the uncertain targets): per candidate slot the best of her odds now and
/// each setup's spark scale at m times its table value f (the products and the strict maxima the outcome loop took, in
/// any setup order: a maximum)
fn fold_emit(cx: &FoldCx, leaf: usize, mu: usize, f: &[f64], u: &mut [f64]) {
    if cx.pl { u[leaf * cx.ns..(leaf + 1) * cx.ns].copy_from_slice(f); return; }
    for (fm, &mf) in cx.fmm.iter().enumerate() {
        let m = mu | mf;
        let row = &mut u[((leaf << cx.nf) | fm) * cx.na..][..cx.na];
        row.copy_from_slice(cx.p0);
        let sr = &cx.scf[m * cx.ns..(m + 1) * cx.ns];
        for ((&sc, &fv), &sl) in sr.iter().zip(f).zip(cx.sslot) { let v = sc * fv; if v > row[sl] { row[sl] = v; } }
    }
}
/// `fold_emit` for the 16 leaves of the last two levels (f[si * 16 + b], b = digit k | digit k+1 << 2), through a slot-major
/// block acc[slot * 16 + b]
fn fold_emit16(cx: &FoldCx, k: usize, leaf: usize, mu: usize, f: &[f64], acc: &mut [f64], u: &mut [f64]) {
    let (ns, na) = (cx.ns, cx.na);
    if cx.pl {
        for b in 0..16usize {
            let li = leaf | (b & 3) << (2 * k) | (b >> 2) << (2 * (k + 1));
            for (si, r) in u[li * ns..(li + 1) * ns].iter_mut().enumerate() { *r = f[si * 16 + b]; }
        }
        return;
    }
    let (ba, bb) = (1usize << cx.uncv[k], 1usize << cx.uncv[k + 1]);
    for (fm, &mf) in cx.fmm.iter().enumerate() {
        // per leaf of the block, where its spark scales start (m-major)
        let mo: [usize; 16] = std::array::from_fn(|b| (mu | mf | if b & 3 > 0 { ba } else { 0 } | if b >> 2 > 0 { bb } else { 0 }) * ns);
        for (sl, a) in acc.chunks_exact_mut(16).enumerate() { a.fill(cx.p0[sl]); }
        for (si, &sl) in cx.sslot.iter().enumerate() {
            let (fs, a) = (&f[si * 16..si * 16 + 16], &mut acc[sl * 16..sl * 16 + 16]);
            for b in 0..16 { let v = cx.scf[mo[b] + si] * fs[b]; if v > a[b] { a[b] = v; } }
        }
        for b in 0..16usize {
            let li = leaf | (b & 3) << (2 * k) | (b >> 2) << (2 * (k + 1));
            let row = &mut u[((li << cx.nf) | fm) * na..][..na];
            for (sl, r) in row.iter_mut().enumerate() { *r = acc[sl * 16 + b]; }
        }
    }
}
/// Every table folded at level k (lv[k]: table j at j 2^(nu-k)) down each branch of her stars on uncertain target k,
/// with the arithmetic of the one-table fold (`t[2i] c + t[2i+1] x`, the last two levels unrolled); per leaf and setup the
/// best of 0 and her tables' values (a maximum: any order), then `fold_emit`. mu: her landed set on the targets so far
#[allow(clippy::too_many_arguments)]
fn fold_rec(cx: &FoldCx, k: usize, leaf: usize, mu: usize, lv: &mut [Vec<f64>], fv: &mut [f64], acc: &mut [f64], u: &mut [f64]) {
    let (nu, ns) = (cx.nu, cx.ns);
    if k == nu {
        // (nu < 2: one value per table)
        let f = &mut fv[..ns]; f.fill(0.0);
        for (j, &si) in cx.tsi.iter().enumerate() { let v = lv[k][j]; if v > f[si] { f[si] = v; } }
        fold_emit(cx, leaf, mu, f, u);
        return;
    }
    if nu - k == 2 {
        // the last two levels: 4 values per table, 16 leaves per setup (f[si * 16 + (digit k | digit k+1 << 2)])
        let f = &mut fv[..16 * ns]; f.fill(0.0);
        let t = &lv[k];
        for (j, &si) in cx.tsi.iter().enumerate() {
            let (t0, t1, t2, t3) = (t[4 * j], t[4 * j + 1], t[4 * j + 2], t[4 * j + 3]);
            let (xa, ca, xb, cb) = (&cx.xs[si][k], &cx.xc[si][k], &cx.xs[si][k + 1], &cx.xc[si][k + 1]);
            let f0: [f64; 4] = std::array::from_fn(|sg| t0 * ca[sg] + t1 * xa[sg]);
            let f1: [f64; 4] = std::array::from_fn(|sg| t2 * ca[sg] + t3 * xa[sg]);
            let o = &mut f[si * 16..si * 16 + 16];
            for sg2 in 0..4usize { for sg in 0..4usize {
                let v = f0[sg] * cb[sg2] + f1[sg] * xb[sg2];
                let oo = &mut o[sg | sg2 << 2]; if v > *oo { *oo = v; }
            } }
        }
        fold_emit16(cx, k, leaf, mu, f, acc, u);
        return;
    }
    let h = 1usize << (nu - k - 1);
    for sg in 0..4usize {
        {
            let (lo, hi) = lv.split_at_mut(k + 1);
            let (t, f) = (&lo[k], &mut hi[0]);
            for (j, &si) in cx.tsi.iter().enumerate() {
                let (x, c) = (cx.xs[si][k][sg], cx.xc[si][k][sg]);
                let (tj, fj) = (&t[2 * h * j..2 * h * (j + 1)], &mut f[h * j..h * (j + 1)]);
                for i in 0..h { fj[i] = tj[2 * i] * c + tj[2 * i + 1] * x; }
            }
        }
        fold_rec(cx, k + 1, leaf | sg << (2 * k), if sg > 0 { mu | 1 << cx.uncv[k] } else { mu }, lv, fv, acc, u);
    }
}

/// Per leaf (base-4 digits, digit k at bits 2k..2k+1) the product of g[k][digit k] over k, into `out`: level by level,
/// each level's products the previous level's (at the leaf's lower digits) times g[k][digit k], from 1.0. The same
/// operations in the same order as `let mut pr = 1.0; for k in 0..nu { pr *= g[k][digit k]; }` per leaf, so the same
/// f64s; each prefix is computed once instead of once per leaf (4/3 4^nu products for nu 4^nu)
fn leaf_products(g: &[[f64; 4]], out: &mut Vec<f64>, tmp: &mut Vec<f64>) {
    out.clear(); out.push(1.0);
    for (k, gk) in g.iter().enumerate() {
        let low = (1usize << (2 * k)) - 1;
        tmp.clear(); for &gd in gk { tmp.extend(out[..=low].iter().map(|&o| o * gd)); }
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
/// `outcome_hi`'s bound for one (candidate, partner), bd per leaf: the least of the ceiling rfc = rf x the best fast
/// outfit's product and, over the anchors (f, qa), f x `rise_leaves`(qa); the same f64s as `rfc.min(anc)` with anc
/// every anchor's value folded in at every leaf (a min may be taken in any order; a NaN rfc counts as +inf there, as
/// `f64::min` drops it). Found by an exact branch and bound over the leaves' digit prefixes (digit k at bits 2k..2k+1,
/// the products' order). An anchor's factors are all > 0 and f > 0 (checked; else it goes through `rise_leaves` whole),
/// so a leaf's product is at least its prefix's product times the least factor of each remaining digit; with slack
/// BB_SLACK for rounding (far more than the (1 + 2^-53)^(2 nu + 4) the chains can drift) that bounds f x the leaf's
/// value from below at every leaf under the prefix (correctly rounded products are monotone). Where the bound reaches
/// the most bd holds under the prefix (`tmax`), the anchor lowers no leaf there: the subtree is skipped.
struct AncBb { nu: usize, no: usize, tmax: Vec<Vec<f64>>, pp: Vec<f64>, smin: Vec<f64>, nodes: u64, roots: u64 }
const BB_SLACK: f64 = 1.0 - 1e-12;
impl AncBb {
    fn new() -> Self { AncBb { nu: 0, no: 0, tmax: vec![], pp: vec![], smin: vec![], nodes: 0, roots: 0 } }
    /// bd per leaf (level nu of `tmax`)
    fn bd(&self) -> &[f64] { &self.tmax[self.nu] }
    /// a fresh (candidate, partner) with `no` outfits for the anchors and the ceiling's fast outfits gc ([outfit][target
    /// k][her stars], nfo of them): bd = rfc (per leaf rf x, from 0.0, the most of each outfit's product in
    /// `leaf_products`' order: the old ceiling's f64s) at the marked leaves (`Marks`), -inf at the others (so no anchor
    /// is folded in where nothing reads the bound: every test below compares with a most that holds marked leaves only),
    /// `tmax` built
    #[allow(clippy::too_many_arguments)]
    fn reset(&mut self, nu: usize, no: usize, gc: &[f64], nfo: usize, rf: f64, mk: &Marks, pre: &mut Vec<f64>, tmp: &mut Vec<f64>) {
        self.nu = nu; self.no = no;
        self.pp.clear(); self.pp.resize((nu + 1) * no, 1.0);
        self.smin.clear(); self.smin.resize((nu + 1) * no, 1.0);
        self.tmax.resize(nu + 1, vec![]);
        let nan_inf = |b: f64| if b.is_nan() { f64::INFINITY } else { b };
        let mut bd = std::mem::take(&mut self.tmax[nu]);
        bd.clear();
        if nu == 0 {
            bd.push(if mk.lv[0][0] { nan_inf(rf * if nfo > 0 { 1.0 } else { 0.0 }) } else { f64::NEG_INFINITY });
            self.tmax[0] = bd; return;
        }
        let np = 1usize << (2 * (nu - 1));
        bd.resize(np * 4, 0.0);
        let mut g: Vec<[f64; 4]> = Vec::with_capacity(nu);
        if mk.dense {
            for o in 0..nfo {
                g.clear(); g.extend((0..nu).map(|k| { let b = (o * nu + k) * 4; [gc[b], gc[b + 1], gc[b + 2], gc[b + 3]] }));
                leaf_products(&g[..nu - 1], pre, tmp);
                // the last digit's level straight into the running most
                for (dg, &gl) in g[nu - 1].iter().enumerate() {
                    for (c, &pv) in bd[dg * np..(dg + 1) * np].iter_mut().zip(pre.iter()) { let pr = pv * gl; if pr > *c { *c = pr; } }
                }
            }
        } else {
            // few marked leaves: each outfit's products down the marked prefixes only, the same products in the same
            // order (1.0 times digit 0's factor, times digit 1's, ...: `leaf_products`' f64s), the same most
            for o in 0..nfo {
                pre.clear(); pre.resize(nu + 1, 1.0);
                ceil_dfs(&mk.lv, &gc[o * nu * 4..(o + 1) * nu * 4], nu, 0, 0, pre, &mut bd);
            }
        }
        for c in bd.iter_mut() { *c = nan_inf(rf * *c); }
        for &l in &mk.unm { bd[l] = f64::NEG_INFINITY; }
        self.tmax[nu] = bd;
        self.levels();
    }
    /// per prefix the most bd holds under it (level j: the 4^j prefixes of j digits; prefix p of j digits holds leaves
    /// p + 4^j r: the most of level j + 1's four quarters; no NaN here)
    fn levels(&mut self) {
        for j in (0..self.nu).rev() {
            let np = 1usize << (2 * j);
            let mut lv = std::mem::take(&mut self.tmax[j]);
            let up = &self.tmax[j + 1];
            lv.clear(); lv.extend((0..np).map(|p| up[p].max(up[np + p]).max(up[2 * np + p]).max(up[3 * np + p])));
            self.tmax[j] = lv;
        }
    }
    /// an anchor's suffix products of its least factors (`smin`), and whether its bound at the root is below the most bd
    /// holds (tmax only falls, so a no now is a no later too); f > 0, g its factors as `run`'s
    fn root_open(&mut self, f: f64, g: &[f64]) -> bool {
        let (nu, no) = (self.nu, self.no);
        if nu == 0 { return true; }
        let mut lb = 0.0f64;
        for o in 0..no {
            for j in (0..nu).rev() {
                let gk = &g[(o * nu + j) * 4..(o * nu + j) * 4 + 4];
                let mn = gk[0].min(gk[1]).min(gk[2]).min(gk[3]);
                self.smin[o * (nu + 1) + j] = self.smin[o * (nu + 1) + j + 1] * mn;
            }
            let l = self.smin[o * (nu + 1)]; if l > lb { lb = l; }
        }
        f * BB_SLACK * lb < self.tmax[0][0]
    }
    /// `root_open`'s answer for an anchor (f > 0, chances qa) from its 0-star factors alone (`rise_factors`' expressions;
    /// the same products in the same order; `smin` is not written, `run` writes it). Premises (the caller's): e, qa, x0 in
    /// [0, 1], xms falling and >= 0, nu > 0. Derived: qn = 1 - x0 xms[s] rises with s (rounding is monotone); a factor is 1
    /// where qn <= qa, else N(qn) / D, D = N(qa) = 1 - (1 - e)(1 - qa) in [0, 1] and N rising with qn: N >= D, so it is
    /// >= 1 (+inf where D = 0, as N > 0 there), never NaN, and rises with qn: each digit's least is its 0-star factor
    #[allow(clippy::too_many_arguments)]
    fn root_min(&self, f: f64, sts: &[Vec<u8>], ews: &[Vec<f64>], x0: &[f64], xms: &[f64; 4], qa: &[f64], uncv: &[usize]) -> bool {
        let nu = self.nu;
        let mut lb = 0.0f64;
        for (st, ew) in sts.iter().zip(ews) {
            let mut l = 1.0f64;
            for k in (0..nu).rev() {
                let i = uncv[k]; let qn = 1.0 - x0[k] * xms[0];
                l *= if st[k] > 0 || qn <= qa[i] { 1.0 } else { (1.0 - (1.0 - ew[k]) * (1.0 - qn)) / (1.0 - (1.0 - ew[k]) * (1.0 - qa[i])) };
            }
            if l > lb { lb = l; }
        }
        f * BB_SLACK * lb < self.tmax[0][0]
    }
    /// fold one anchor in: f > 0, g its factors [outfit][target k][her stars], all > 0
    fn run(&mut self, f: f64, g: &[f64]) {
        let (nu, no) = (self.nu, self.no);
        if nu == 0 {
            let v = f * if no > 0 { 1.0 } else { 0.0 }; if v < self.tmax[0][0] { self.tmax[0][0] = v; }
            return;
        }
        if !self.root_open(f, g) { return; }
        for o in 0..no { self.pp[o] = 1.0; }
        self.roots += 1;
        self.dfs(0, 0, f, f * BB_SLACK, g);
    }
    fn dfs(&mut self, j: usize, p: usize, f: f64, fs: f64, g: &[f64]) {
        let (nu, no) = (self.nu, self.no);
        self.nodes += 1;
        for dg in 0..4usize {
            let pc = p | dg << (2 * j);
            for o in 0..no { self.pp[(j + 1) * no + o] = self.pp[j * no + o] * g[(o * nu + j) * 4 + dg]; }
            if j + 1 == nu {
                let mut rv = 0.0f64; for o in 0..no { let pr = self.pp[nu * no + o]; if pr > rv { rv = pr; } }
                let v = f * rv;
                let b = &mut self.tmax[nu][pc];
                if v < *b { *b = v; }
            } else {
                let mut lb = 0.0f64;
                for o in 0..no { let l = self.pp[(j + 1) * no + o] * self.smin[o * (nu + 1) + j + 1]; if l > lb { lb = l; } }
                if fs * lb >= self.tmax[j + 1][pc] { continue; }
                self.dfs(j + 1, pc, f, fs, g);
            }
        }
        let up = &self.tmax[j + 1]; let s = 1usize << (2 * j);
        self.tmax[j][p] = up[p].max(up[p + s]).max(up[p + 2 * s]).max(up[p + 3 * s]);
    }
}
/// The leaves (and outcomes) whose bound something reads: `lv[j][p]`, prefix p of j digits holds a marked leaf (level nu:
/// the leaves); `dense` when most leaves are marked (the ceiling is then computed at every leaf, vectorised)
struct Marks { lv: Vec<Vec<bool>>, dense: bool, unm: Vec<usize> }
/// `AncBb::reset`'s ceiling for one outfit (g [target k][her stars]) down the marked prefixes: pp[j] the product of the
/// first j digits' factors from 1.0, into the running most at each marked leaf
fn ceil_dfs(mk: &[Vec<bool>], g: &[f64], nu: usize, j: usize, p: usize, pp: &mut [f64], bd: &mut [f64]) {
    for dg in 0..4usize {
        let pc = p | dg << (2 * j);
        if !mk[j + 1][pc] { continue; }
        let v = pp[j] * g[j * 4 + dg];
        if j + 1 == nu { if v > bd[pc] { bd[pc] = v; } } else { pp[j + 1] = v; ceil_dfs(mk, g, nu, j + 1, pc, pp, bd); }
    }
}
/// `rise_leaves`'s factors for chances qa, [outfit][target k][her stars] (the same expressions), into g
#[allow(clippy::too_many_arguments)]
fn rise_factors(sts: &[Vec<u8>], ews: &[Vec<f64>], x0: &[f64], xms: &[f64; 4], qa: &[f64], uncv: &[usize], g: &mut Vec<f64>) {
    let nu = uncv.len(); g.clear();
    for (st, ew) in sts.iter().zip(ews) {
        for k in 0..nu { let i = uncv[k]; for sg in 0..4 {
            let qn = 1.0 - x0[k] * xms[sg];
            g.push(if st[k] > 0 || qn <= qa[i] { 1.0 } else { (1.0 - (1.0 - ew[k]) * (1.0 - qn)) / (1.0 - (1.0 - ew[k]) * (1.0 - qa[i])) });
        } }
    }
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
/// Off by default (not worth the plan's +6-11%); 10 gives the numbers above.
const HI_SEARCHES: usize = 0;
/// ... and only for band outcomes whose partners' bounds are within this factor of p_now: a cost rule, not part of the
/// bound. It skips p10 and p11, whose band needs partners at 2.5 times p_now and more: there 15 searches (at 1, 2 and 3
/// stars) freed nothing (p11: 233 ms native spent), and only 63 (805 ms) moved p10 (0.474 -> 0.378); 1.3 and 1.5 chose
/// the same
const HI_RISE: f64 = 1.5;
/// `outcome_hi` for one (candidate ui, partner y): her slot's miss factors, the bound per leaf (`AncBb`), each marked
/// outcome's most of the partners' (`tu`: candidate ui's row of the table). Its own function, entered once per partner,
/// so V8 compiles it optimised after the first few (`docs/PERF.md`, *In the page*)
struct HiCx<'h> { bs: &'h ScoreBase<'h>, d: &'h Data, legs: &'h [Legacy], pool: &'h Pool, fix: &'h [usize], uncv: &'h [usize], mk: &'h Marks,
    mc: &'h [usize], lm: &'h [u32], nu: usize, ntar: usize, nm: usize, nf: usize, ncell: usize, nmc: usize, iv: usize, rf: f64 }
/// `hi_partner`'s scratch, kept across partners (each call writes what it reads)
struct HiScr { bb: AncBb, g1: Vec<f64>, gc: Vec<f64>, lp: Vec<f64>, lp_tmp: Vec<f64>, qz: Vec<f64>, tg: Vec<f64> }
#[inline(never)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn hi_partner(s: &Sim, hx: &HiCx, pv: &Pool, ui: usize, u: usize, y: usize, sts: &[Vec<u8>], ews: &[Vec<f64>], ew_ok: bool, fr: &[usize],
    fr_of: &[Option<Option<(Vec<f64>, f64)>>], tu: &mut [f64], hp: &mut HiScr, hs: &mut Vec<(usize, Vec<f64>, Vec<f64>)>, hq: &mut Vec<(usize, Vec<f64>, Vec<f64>, [f64; 4])>) {
    let HiCx { bs, d, legs, pool, fix, uncv, mk, mc, lm, nu, ntar, nm, nf, ncell, nmc, iv, rf } = *hx;
    let HiScr { bb, g1, gc, lp, lp_tmp, qz, tg } = hp;
    let sl = s.env.slots(pv, u, y, iv);
    let Some(pos) = sl.iter().position(|z| z.parent && z.legacy == iv) else { return };
    let a_v = sl[pos].rel + sl[pos].race;
    let mut sl0 = sl.clone(); { let s = std::rc::Rc::make_mut(&mut sl0[pos].stars); for k in 0..ntar { s[k] = 0; } } sl0[pos].mix = None;
    let (q0, nc0) = inherit(&sl0, ntar);
    let xm = |st: u8| { let m = 1.0 - (0.03 * st as f64 * (1.0 + a_v / 100.0)).min(1.0); m * m };
    let xms = [1.0, xm(1), xm(2), xm(3)];
    let x0: Vec<f64> = uncv.iter().map(|&i| 1.0 - q0[i]).collect();
    // the best outfit's product per leaf: per outfit and target the factor A - B (1 - q') at each of her four star
    // levels, 1 - q' = x0 xm (`outfit_fast` at her chances folded in, as `Sim::fast_ub` takes it)
    let t0 = now_ms();
    gc.clear();
    for o in &s.fastc[u] { for k in 0..nu { let (aa, bb) = o[k]; for sg in 0..4 { gc.push(aa - bb * (x0[k] * xms[sg])); } } }
    // the anchored rise bound per leaf: the least over today's pairs (y, z) keeping the places of
    // (pnow / c_yz) rise(q'(leaf), q_yz), q'_k(leaf) = 1 - x0[k] xm[her stars], and over the main search's proven
    // careers of her deck class (E_best(q_f) = val_f there: val_f rise(q', q_f)); at most the ceiling too (under
    // EXACT_RHO): bd = min(rf ceil, anc) per leaf, by `AncBb` (anchors in any order: a min)
    bb.reset(nu, sts.len(), gc, s.fastc[u].len(), rf, mk, lp, lp_tmp); let t1 = now_ms();
    // the anchors whose bound at the root is open (f, factors), and whether any went in whole
    let mut direct = false;
    let (mut gs, mut fs): (Vec<f64>, Vec<f64>) = (vec![], vec![]);
    // (`AncBb::root_min`'s premises but the anchor's chances)
    let safe = ew_ok && nu > 0 && xms[0] >= xms[1] && xms[1] >= xms[2] && xms[2] >= xms[3] && xms[3] >= 0.0 && x0.iter().all(|v| (0.0..=1.0).contains(v));
    let mut add = |bb: &mut AncBb, f: f64, qa: &[f64], gs: &mut Vec<f64>, fs: &mut Vec<f64>, direct: &mut bool| {
        if f > 0.0 && safe && uncv.iter().all(|&i| (0.0..=1.0).contains(&qa[i])) {
            // every factor >= 1, each digit's least its 0-star one: the root test on those alone, the rest only
            // for an anchor it keeps (as the test below would: all factors > 0, `root_open` the same answer)
            if !bb.root_min(f, sts, ews, &x0, &xms, qa, uncv) { return; }
            rise_factors(sts, ews, &x0, &xms, qa, uncv, g1);
            debug_assert!(g1.iter().all(|&v| v > 0.0) && bb.root_open(f, g1));
            fs.push(f); gs.extend_from_slice(g1); return;
        }
        rise_factors(sts, ews, &x0, &xms, qa, uncv, g1);
        if f > 0.0 && g1.iter().all(|&v| v > 0.0) { if bb.root_open(f, g1) { fs.push(f); gs.extend_from_slice(g1); } } else {
            // outside the bound's premises: every leaf, as before
            *direct = true;
            let rz = rise_leaves(sts, ews, &x0, &xms, qa, uncv, lp, lp_tmp);
            for (av, rv) in bb.tmax[bb.nu].iter_mut().zip(&rz) { let v = f * rv; if v < *av { *av = v; } }
        }
    };
    if bs.pnow[ui] > 0.0 {
        let mut nz = 0usize;
        for z in (0..legs.len()).filter(|&z| z != y && legal_pair(d, pool, u, y, z) && fix.iter().all(|&f| f == y || f == z)) {
            let cz = s.env.lineage_qc(pool, u, y, z, uncv, qz, tg);
            if cz <= 0.0 { continue; }
            nz += 1;
            add(bb, bs.pnow[ui] / cz, qz, &mut gs, &mut fs, &mut direct);
        }
        if nz > 0 { prof("hi_z", nz as f64); }
    }
    for &i in fr { let (fq, fv) = fr_of[i].as_ref().unwrap().as_ref().unwrap(); add(bb, *fv, fq, &mut gs, &mut fs, &mut direct); }
    if direct { bb.levels(); }
    // the likeliest lowest first (f x the best outfit's product at 2 stars on every target): only speed
    let ga = sts.len() * nu * 4;
    let mut ord: Vec<(f64, usize)> = fs.iter().enumerate().map(|(i, &f)| {
        let gi = &gs[i * ga..(i + 1) * ga];
        (f * (0..sts.len()).map(|o| (0..nu).map(|k| gi[(o * nu + k) * 4 + 2]).product::<f64>()).fold(0.0f64, f64::max), i) }).collect();
    ord.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.cmp(&q.1)));
    let t3 = now_ms(); let nodes0 = bb.nodes;
    for &(_, i) in &ord { bb.run(fs[i], &gs[i * ga..(i + 1) * ga]); }
    prof("hi_nodes", (bb.nodes - nodes0) as f64); prof("hi_open", fs.len() as f64); prof("hi_roots", bb.roots as f64); bb.roots = 0; let t4 = now_ms();
    let pw: Vec<[f64; 2]> = (0..ntar).map(|k| [1.1f64.powi(nc0[k] - 6), 1.1f64.powi(nc0[k] + 1 - 6)]).collect();
    let sc: Vec<f64> = (0..nm).map(|m| (0..ntar).map(|k| pw[k][(m >> k & 1) as usize]).product()).collect();
    // each outcome's most of the partners' sc[m] bd[leaf] (from pnow; in partner order per candidate, as `hi_at`)
    let bd = bb.bd();
    if nmc == ncell && nf == 0 {
        for ((t, &m), &b) in tu.iter_mut().zip(lm).zip(bd) { let v = sc[m as usize] * b; if v > *t { *t = v; } }
    } else {
        for ((t, &i), &m) in tu.iter_mut().zip(mc).zip(lm) { let v = sc[m as usize] * bd[i >> nf]; if v > *t { *t = v; } }
    }
    prof("hi_ceil_ms", t1 - t0); prof("hi_anc_ms", t3 - t1); prof("hi_bb_ms", t4 - t3); prof("hi_fold_ms", now_ms() - t4); prof("hi_pairs", 1.0);
    if HI_SEARCHES > 0 { hs.push((ui, bb.bd().to_vec(), sc)); }
    hq.push((u, q0, x0, xms));
}
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
///     saving. Not bounded (channel 2, out of scope by design): what the replan's refits do (Cont fitted
///     again, other careers' best careers with her as a parent, the stage candidates chosen again beyond her own 8/8).
#[allow(clippy::too_many_arguments)]
fn outcome_hi(s: &mut Sim, d: &Data, bs: &ScoreBase, legs: &[Legacy], pool: &Pool, fix: &[usize], msv: &Solved, ytab: &[f64], x: usize, a: usize, b: usize, sel: &Sel, buf: &mut Vec<f64>) -> (Vec<f64>, Vec<f64>) {
    let n = bs.cands.len(); let ntar = s.env.ntar; let nu = s.env.uncv.len();
    let uncv = s.env.uncv.clone(); let fixed_t = s.env.fixed_t.clone(); let w = s.env.sj.star_dist;
    let xi = bs.cands.iter().position(|&u| u == x);
    let goal = is_full(s.env.sj, &legs[a].sparks) && is_full(s.env.sj, &legs[b].sparks);
    let c = s.make_cur(pool, x, a, b, sel, false);
    let mut pv = her_pool(s, legs, pool, true, x, a, b, &c); let iv = pv.me.len() - 1;
    // per candidate and EVERY legal partner: her slot's miss factors and the spark scale, as `score_career`'s setups
    let nleaf = 1usize << (2 * nu); let nm = 1usize << ntar;
    // (candidate, bound at scale 1 per leaf, spark scale per m): kept only for the exact anchors (`HI_SEARCHES` > 0); else
    // each outcome reads `tab`, the same maxima folded in as each partner is done
    let mut hs: Vec<(usize, Vec<f64>, Vec<f64>)> = vec![];
    let mut hq: Vec<(usize, Vec<f64>, Vec<f64>, [f64; 4])> = vec![];   // per partner: (trainee, q0, x0, xms), for the anchors
    let rf = s.env.space.rho_top() * s.fixed;
    let (lp, lp_tmp): (Vec<f64>, Vec<f64>) = (vec![], vec![]);
    // the outcomes (her stars `leaf`, the fixed targets landed fm: cell leaf << nf | fm) with their chances and landed sets
    let nf = fixed_t.len(); let ncell = nleaf << nf;
    let mut prs: Vec<f64> = vec![0.0; ncell]; let mut ms: Vec<usize> = vec![0; ncell];
    for leaf in 0..nleaf {
        let mut mu = 0usize; let mut pr_st = 1.0f64;
        for k in 0..nu { let sg = (leaf >> (2 * k)) & 3; if sg > 0 { mu |= 1 << uncv[k]; pr_st *= w[sg - 1]; } }
        for fm in 0..(1usize << nf) {
            let mut m = mu; for (j, &i) in fixed_t.iter().enumerate() { if fm >> j & 1 == 1 { m |= 1 << i; } }
            let pr = c.os[m] * pr_st;
            if pr <= 0.0 { continue; }
            prs[leaf << nf | fm] = pr; ms[leaf << nf | fm] = m;
        }
    }
    // the outcomes whose bound is read: the page reads y_hi only where its y (rounded) is under 0.5, `provisional` sums
    // only cells with y < 0.5, the band (`HI_SEARCHES`) is within them; elsewhere the bound is not computed (no cell's
    // value depends on another's: each is the most over partners of a per-leaf least, computed alone)
    let marked = |i: usize| prs[i] > 0.0 && ytab[i] < 0.5;
    let mc: Vec<usize> = (0..ncell).filter(|&i| marked(i)).collect();
    // (her own 8/8 counts only on the full set: needed only when a marked outcome lands it)
    let her_need = mc.iter().any(|&i| ms[i] == nm - 1);
    let mut mk = Marks { lv: vec![vec![]; nu + 1], dense: false, unm: vec![] };
    mk.lv[nu] = vec![false; nleaf]; for &i in &mc { mk.lv[nu][i >> nf] = true; }
    for j in (0..nu).rev() { let np = 1usize << (2 * j); let up = &mk.lv[j + 1]; mk.lv[j] = (0..np).map(|p| up[p] || up[np + p] || up[2 * np + p] || up[3 * np + p]).collect(); }
    mk.unm = (0..nleaf).filter(|&l| !mk.lv[nu][l]).collect();
    let nml = nleaf - mk.unm.len(); mk.dense = 4 * nml >= nleaf;
    prof("hi_cells", mc.len() as f64); prof("hi_mleaves", nml as f64);
    if mc.is_empty() { return (vec![0.0; ncell], prs); }
    // the cells the table holds: the marked ones, or with most leaves marked every possible one, folded as before
    // (vectorised where every cell is possible; an unmarked leaf's bound is -inf, so its cells keep p_now, never read)
    let mc: Vec<usize> = if mk.dense { (0..ncell).filter(|&i| prs[i] > 0.0).collect() } else { mc };
    let lm: Vec<u32> = mc.iter().map(|&i| ms[i] as u32).collect();
    let nmc = mc.len();
    // per candidate and marked outcome her odds' bound so far: pnow, then the most of every partner's (`hi_at` without `hs`)
    let mut tab: Vec<f64> = (0..n).flat_map(|ui| std::iter::repeat_n(bs.pnow[ui], nmc)).collect();
    // per found career of the main search, (q_f, val_f) when fully checked (a Ref at its own chances): once each
    let mut fr_of: Vec<Option<Option<(Vec<f64>, f64)>>> = vec![None; msv.found.len()];
    // an anchor's chances (`Env::lineage_qc`: only the uncertain targets' entries, the ones the factors read)
    let mut hp = HiScr { bb: AncBb::new(), g1: vec![], gc: vec![], lp, lp_tmp, qz: vec![0.0f64; ntar], tg: vec![] };
    let hx = HiCx { bs, d, legs, pool, fix, uncv: &uncv, mk: &mk, mc: &mc, lm: &lm, nu, ntar, nm, nf, ncell, nmc, iv, rf };
    for ui in 0..n {
        let u = bs.cands[ui];
        if d.umas[u].id == d.umas[x].id || bs.c1[ui] == 0.0 || s.fastc[u].is_empty() { continue; }
        // her outfits' skill coverage and own chances (as `Env::rise` reads them)
        let (sts, ews) = (s.env.sts0[u].clone(), s.env.ew0[u].clone());
        hp.tg.clear();   // (`Env::lineage_qc`'s table: another trainee)
        // the main search's found careers of her deck class that it fully checked (a Ref at the career's own chances):
        // (q_f, val_f), the best value of any deck at q_f for every trainee of the class (`Env::seen`), scale 1
        let ct = s.env.cls[u];
        let fr: Vec<usize> = (0..msv.found.len()).filter(|&i| s.env.cls[msv.found[i].t] == ct).filter(|&i| fr_of[i].get_or_insert_with(|| {
            let f = &msv.found[i]; let (fq, _, _) = s.env.lineage(pool, f.t, f.a, f.b);
            msv.refs.get(f.t).and_then(|rs| rs.iter().find(|r| r.q == fq)).map(|r| (r.q.clone(), r.val)) }).is_some()).collect();
        // `AncBb::root_min`'s premises on her own chances
        let ew_ok = ews.iter().all(|ew| ew.iter().all(|e| (0.0..=1.0).contains(e)));
        for y in (0..iv).filter(|&y| legal_pair(d, &pv, u, y, iv)) {
            hi_partner(s, &hx, &pv, ui, u, y, &sts, &ews, ew_ok, &fr, &fr_of, &mut tab[ui * nmc..(ui + 1) * nmc], &mut hp, &mut hs, &mut hq);
        }
    }
    let HiScr { mut lp, mut lp_tmp, .. } = hp;
    let full_m = nm - 1;
    let her: Vec<(usize, bool, f64)> = if xi.is_none() && !goal && !bs.sides.is_empty() && c.os[full_m] > 0.0 && her_need { her_trios(s, d, bs, &mut pv, iv, x, buf) } else { vec![] };
    let mut ys: Vec<f64> = vec![0.0; ncell];
    let mut c1 = bs.c1.clone(); let mut up = vec![0.0f64; n];
    let t5 = now_ms();
    for (j, &i) in mc.iter().enumerate() {
        if !marked(i) { continue; }
        for ui in 0..n { up[ui] = tab[ui * nmc + j]; }
        ys[i] = saving_lo(bs, &up, ms[i], full_m, xi, &her, goal, &mut c1);
    }
    drop(tab); prof("hi_ys_ms", now_ms() - t5);
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
/// step; a roll that changes nothing leaves every score as it was (repeat). Pick: the highest;
/// within NEAR, the better own odds. The goal is always the next fully looped parent, any uma: one you have already is a
/// real 8/8 (free) with 8/8 parents, a strong side of the trios.
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn index_rule<'a>(env: &mut Env<'a>, d: &'a Data, sk: &'a SkillsRaw, cr: &'a CardsRaw, job: &'a Value, legs: &[Legacy],
                                       pool: &Pool, sv: &Solved, fix: &[usize], trainees: &[usize], pj: &PlanJob) -> Value {
    let t_all = now_ms();
    progress(1, 0, 0);   // the page's status line: the plan begins (its candidates' best careers)
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
    // per uma: her best career's search ran out of work (`best_of`), and her looped one's
    let mut unp: Vec<bool> = vec![false; d.umas.len()]; let mut lunp: Vec<bool> = vec![false; d.umas.len()];
    if pj.sur != 1 { for &u in &cands { if best[u].is_none() { let (f, r, ok) = best_of(env, pool, fix, u, same.as_ref(), prev.as_ref(), nl); urefs[u] = r; best[u] = Some(f); unp[u] = !ok; } } }
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
    let n = cands.len();
    let pnow: Vec<f64> = fb.iter().map(|f| f.as_ref().map_or(0.0, |f| f.val)).collect();
    let sj = s.env.sj;
    // per candidate whether an 8/8 of hers exists (a free side of the trios)
    let have8: Vec<bool> = cands.iter().map(|&u| legs.iter().any(|l| l.chara == d.umas[u].id && is_full(sj, &l.sparks))).collect();
    let zeros = vec![0.0f64; n];
    let mut exact_n = 0usize;
    let t3 = now_ms();
    // Builders through their deck class: a builder's career is first taken from her class's winning decks (the main
    // search's and the candidates' best careers of her class: legal for her, the same deck problem, valued at her own
    // chances through the surrogate, `best_sur`), no career search. Exact searches only where the plan can turn on her:
    // a lift efficiency within PROXY_EFF of a candidate's second best (Cont's two builders come from the exactly
    // searched careers), or a saving within PROXY_SAVE of the shown rows (the page's top OPT_SHOW) or the pick's near-tie
    // band. Measured on val/plan (`proxy_check`): the proxy's odds only ever read low (it is a
    // career she can run); its efficiency on contenders for a candidate's top two at most 2.8% low (p11; it can also
    // read high, which only searches more), its saving at most 2.7% low; exact on the other five jobs.
    // `exact_all` restores a search for every uma.
    let proxy_on = pj.sur == 0 && !pj.exact_all && !pj.proxy_check && !pj.value_only;
    let wins: Vec<(usize, Sel)> = sv.found.iter().map(|f| (f.t, f.sel.clone()))
        .chain(cands.iter().filter_map(|&u| best[u].clone().flatten().map(|f| (u, f.sel)))).collect();
    // pass 1: every allowed uma's best career; per candidate, the chance her veteran raises that candidate's odds and the
    // mean odds raised to (exact over her outcomes)
    let mut runs: Vec<(usize, Found, Vec<(f64, f64)>)> = vec![];
    // per run: a proxy's lifts' stamp (`stamped`; None for every other run)
    let mut st_ver: Vec<Option<(usize, usize)>> = vec![];
    // per run: an exact best career (else her class's proxy)
    let mut exact_run: Vec<bool> = vec![];
    // admitted proxies the class's proofs certified (`certified`), so not searched
    let mut n_cert = 0usize;
    let mut wtab: Vec<Vec<usize>> = vec![vec![]; d.umas.len()];
    // (placeholder step costs: 1, so every candidate's lifts are gathered; an 8/8 of hers may be a poorer parent than a
    // fresh one)
    let base_s = ScoreBase { cands: &cands, c1: vec![1.0; n], m23: &zeros, v0: 0.0, pnow: pnow.clone(), cont: vec![], combos: vec![], sides: &[], all: &[] };
    // uma x's exact best career (searched now if not yet) and its lifts; `reuse`: her proxy career, its lifts and their
    // stamp (`stamped`, over the candidates' decks: this base reads no other's), taken as they are when the search found
    // the same career and nothing they read has changed
    let exact_one = |s: &mut Sim, best: &mut Vec<Option<Option<Found>>>, urefs: &mut Vec<Vec<Vec<Ref>>>, unp: &mut Vec<bool>, x: usize, buf: &mut Vec<f64>,
                     reuse: Option<(&Found, &[(f64, f64)], Option<(usize, usize)>)>| -> Option<(Found, Vec<(f64, f64)>)> {
        if best[x].is_none() {
            if pj.search_work > 0 { s.env.budget = pj.search_work; s.env.hard = pj.search_work; }
            let (f, r, ok) = best_of(s.env, pool, fix, x, same.as_ref(), prev.as_ref(), nl);
            if let Some(f) = &f { s.cache.add(s.env, x, &f.sel); }
            urefs[x] = r; best[x] = Some(f); unp[x] = !ok;
            s.env.hard = 0; s.env.budget = budget0;
        }
        let f = best[x].clone().flatten()?;
        if let Some((pf, pst, stamp)) = reuse { if same_career(pf, &f) && stamp_ok(s, Some(&cands), stamp) { prof("reuse_st", 1.0); return Some((f, pst.to_vec())); } }
        let (_, _, _, st) = score_career(s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, false, buf);
        Some((f, st))
    };
    let set_wins = |s: &mut Sim, wtab: &mut Vec<Vec<usize>>, x: usize, f: &Found| { let w = s.wins(pool, x, f.a, f.b); wtab[x] = w.iter().enumerate().filter(|x| *x.1 > 0.5).map(|x| x.0).collect(); };
    if !pj.value_only {
        let mut order: Vec<usize> = vec![g.t]; for &x in &all { if x != g.t { order.push(x); } }
        for &x in &order {
            progress(2, order.iter().position(|&y| y == x).unwrap_or(0), order.len());   // pass 1: uma k of n
            if proxy_on && best[x].is_none() {
                let ts = now_ms();
                let cx = s.env.cls[x];
                for (t, sel) in &wins { if *t != x && s.env.cls[*t] == cx { s.cache.add(s.env, x, sel); } }
                let pf = best_sur(&mut s, d, pool, fix, x, 1e9, &mut buf);
                search_ms += now_ms() - ts;
                let Some(f) = pf else { continue };
                let ((_, _, _, st), sv) = stamped(&mut s, Some(&cands), |s| score_career(s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, false, &mut buf));
                runs.push((x, f, st)); exact_run.push(false); st_ver.push(sv);
                continue;
            }
            if best[x].is_none() || (pj.sur == 2 && !cands.contains(&x)) {
                let ts = now_ms();
                if pj.search_work > 0 { s.env.budget = pj.search_work; s.env.hard = pj.search_work; }
                if pj.sur != 1 && best[x].is_none() {
                    let (f, r, ok) = best_of(s.env, pool, fix, x, same.as_ref(), prev.as_ref(), nl);
                    if let Some(f) = &f { s.cache.add(s.env, x, &f.sel); }
                    urefs[x] = r; best[x] = Some(f); unp[x] = !ok;
                }
                if pj.sur > 0 { sur_one(&mut s, &mut best, x, &mut buf, &mut sur_diff); }
                s.env.hard = 0; s.env.budget = budget0;
                search_ms += now_ms() - ts;
            }
            let Some(Some(f)) = best[x].clone() else { continue };
            let (_, _, _, st) = score_career(&mut s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, false, &mut buf);
            runs.push((x, f, st)); exact_run.push(true); st_ver.push(None);
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
                let r = { let ri = &runs[i]; exact_one(&mut s, &mut best, &mut urefs, &mut unp, x, &mut buf, Some((&ri.1, &ri.2[..], st_ver[i]))) };
                if let Some((f, st)) = r { runs[i] = (x, f, st); } exact_run[i] = true; st_ver[i] = None;
            }
            search_ms += now_ms() - ts;
        }
        for r in &runs { set_wins(&mut s, &mut wtab, r.0, &r.1); }
    }
    // Looped careers: per uma whose career above has parents that are not both 8/8s, her best career on two real 8/8
    // veterans (a legal pair, all of `fix` among them), a row of its own beside it. Only there is her own 8/8 a fully
    // looped parent (the goal), and the trios' last step is such a career: without these rows the career V itself
    // prices (C on A's and B's 8/8s) was never among the options, and a uma whose best-odds pair happened to be two 8/8s
    // took the goal's credit alone (inputs of 2026-10-17: Taiki Shuttle 0.454 over Inari One's looped career, 1.000).
    // Exact (the surrogate's under `sur` 1); pairs by ceiling, stopping below the best found. Not among Cont's builders
    // (Cont is fitted on each uma's best career) and not added to the cache (what the tables read stays the job's
    // winners), so V and every other row are as before.
    let nbase = runs.len();
    let looped = |f: &Found| is_full(sj, &legs[f.a].sparks) && is_full(sj, &legs[f.b].sparks);
    // per uma her best looped career where searched (None: not searched; Some(None): no legal looped pair), kept with
    // the plan's best careers: a kept plan of this very pool gives it as kept; one of this pool less its last veteran
    // gives it as kept unless she is an 8/8 (then only her pairs are new: the better of the kept one and those, a tie
    // keeping the kept one)
    let mut lbest: Vec<Option<Option<Found>>> = vec![None; d.umas.len()];
    progress(3, 0, 0);   // the looped careers
    if !pj.value_only && fix.len() < 2 {
        let full8: Vec<usize> = (0..nl).filter(|&i| is_full(sj, &legs[i].sparks)).collect();
        let her8 = nl > 0 && full8.contains(&(nl - 1));
        let ts = now_ms(); let lnodes0 = s.env.space.nodes.get();
        if pj.search_work > 0 { s.env.budget = pj.search_work; s.env.hard = pj.search_work; }
        for i in 0..nbase {
            let x = runs[i].0;
            if looped(&runs[i].1) { continue; }
            let kept = |b: &Option<Bests>| b.as_ref().and_then(|b| b.looped.get(x).cloned().flatten());
            let kept_unp = |b: &Option<Bests>| b.as_ref().is_some_and(|b| b.lunp.get(x).copied().unwrap_or(false));
            let lf: Option<Found> = if let Some(v) = kept(&same) { lunp[x] = kept_unp(&same); v } else {
                let start = kept(&prev);
                let only_her = start.is_some();
                if only_her { lunp[x] = kept_unp(&prev); }
                if only_her && !her8 { start.flatten() } else {
                    let mut cs: Vec<(f64, usize, usize)> = vec![];
                    for (j, &a) in full8.iter().enumerate() { for &b in &full8[j + 1..] {
                        if only_her && b != nl - 1 { continue; }
                        if !legal_pair(d, pool, x, a, b) || !fix.iter().all(|&g| g == a || g == b) { continue; }
                        let (q, _, c) = s.env.lineage(pool, x, a, b);
                        if let Some(ub) = s.env.ceiling(x, &q, c) { cs.push((ub, a, b)); }
                    } }
                    cs.sort_by(|p, q| q.0.partial_cmp(&p.0).unwrap());
                    let mut lf: Option<Found> = start.flatten();
                    for (ub, a, b) in cs {
                        if ub <= lf.as_ref().map_or(0.0, |f| f.val) { break; }
                        // (the exact search's bar starts at the best looped career so far: only a career above it can
                        // replace it, as in `best_of`; a tie keeps the earlier pair either way)
                        let f = if pj.sur == 1 { best_sur(&mut s, d, pool, &[a, b], x, pj.sur_margin, &mut buf) }
                                else {
                                    s.env.floor = lf.as_ref().map_or(f64::NEG_INFINITY, |f| f.val);
                                    let sr = s.env.solve(pool, &[a, b], &[x], false, 1);
                                    if !sr.complete { lunp[x] = true; }
                                    s.env.floor = f64::NEG_INFINITY; sr.found.into_iter().next()
                                };
                        if let Some(f) = f { if f.val > lf.as_ref().map_or(0.0, |g| g.val) { lf = Some(f); } }
                    }
                    lf
                }
            };
            lbest[x] = Some(lf.clone());
            if let Some(f) = lf {
                let (_, _, _, st) = score_career(&mut s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, false, &mut buf);
                runs.push((x, f, st)); exact_run.push(true); st_ver.push(None);
            }
        }
        s.env.hard = 0; s.env.budget = budget0;
        search_ms += now_ms() - ts; prof("looped_ms", now_ms() - ts); prof("looped_nodes", (s.env.space.nodes.get() - lnodes0) as f64);
    }
    // Cont's builders: the careers above the looped ones
    let cmask = |exact: &[bool]| -> Vec<bool> { exact.iter().enumerate().map(|(i, &e)| e && i < nbase).collect() };
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
    let mut cont = cont_of(&mut s, &runs, &cmask(&exact_run), &wtab, &mut buf);
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
        let (ey, lift, ys, _) = score_career(s, d, &base, legs, pool, r.0, r.1.a, r.1.b, &r.1.sel, true, buf);
        (ey - 1.0, lift, ys)
    };
    let mut score: Vec<f64> = vec![]; let mut lifts: Vec<f64> = vec![]; let mut ytabs: Vec<Vec<f64>> = vec![];
    // Each row's per-outcome table (4^nu 2^nf floats; about 700 MB over every row at 10 uncertain targets) is kept only
    // while the row can still be the pick. The pick (below) is a row within NEAR.max(0.05 |top|) of top, top at least the
    // highest score less 1e-9 (the argmax's tolerance), or the base row a looped pick repeats (`undup`). A row's score
    // changes only when it is scored again, which writes a fresh table, and an exactly searched row is scored again only
    // when every row is (a refit). So lo, the highest score of an exact row among the rows scored since every row last
    // was, is at most the final top; thr(t) = t - NEAR.max(0.05 |t|) rises with t; a row below thr(lo) (less a slack far
    // above its rounding) is not near the final top unless it is scored again. Kept also: every base row some looped row
    // repeats (same uma and pair; a base row whose career changes is scored again). Only the pick's table is read.
    let prune = |score: &[f64], exact: &[bool], runs: &[(usize, Found, Vec<(f64, f64)>)], ytabs: &mut [Vec<f64>]| {
        let lo = (0..score.len()).filter(|&i| exact[i]).map(|i| score[i]).fold(f64::NEG_INFINITY, f64::max);
        if !lo.is_finite() { return; }
        let thr = lo - NEAR.max(0.05 * lo.abs()) - 1e-6 * (1.0 + lo.abs());
        let pr = |f: &Found| (f.a.min(f.b), f.a.max(f.b));
        for i in 0..score.len() {
            if ytabs[i].is_empty() || !(score[i] < thr) { continue; }
            if i < nbase && (nbase..runs.len()).any(|k| runs[k].0 == runs[i].0 && pr(&runs[k].1) == pr(&runs[i].1)) { continue; }
            ytabs[i] = vec![];
        }
    };
    // per row its score's stamp (`stamped`, over every trainee's decks: `her_trios` reads any C's): a row whose career
    // and base are as they were, scored again while the stamp holds, keeps its score, lift and table
    let mut sc_ver: Vec<Option<(usize, usize)>> = vec![];
    progress(4, 0, runs.len());   // pass 2: every career against V
    if !runs.is_empty() && vnow.is_finite() {
        for r in &runs { let ((sc, lift, ys), v) = stamped(&mut s, None, |s| score_one(s, r, &c1, vnow, &cont, &combos3, &sides, &mut buf)); score.push(sc); lifts.push(lift); ytabs.push(ys); sc_ver.push(v); prune(&score, &exact_run, &runs, &mut ytabs); }
        // the proxies a shown row or the pick's near-tie band can turn on: searched exactly (their
        // saving, score + 1, within PROXY_SAVE of the lowest saving those take), then scored again; when one of them joins a
        // candidate's two most efficient builders, Cont, the trios, V and every score are computed again
        while proxy_on {
            let mut sv: Vec<f64> = score.clone(); sv.sort_by(|x, y| y.partial_cmp(x).unwrap());
            let (top, k8) = (sv[0], sv[(OPT_SHOW - 1).min(sv.len() - 1)]);
            let t = k8.min(top - NEAR.max(0.05 * top.abs())) + 1.0;
            let add: Vec<usize> = (0..runs.len()).filter(|&i| !exact_run[i] && score[i] + 1.0 >= t * (1.0 - PROXY_SAVE)).collect();
            if add.is_empty() { break; }
            let ts = now_ms();
            // (the careers scored: a row whose search found the same one, or that was certified, may keep its score)
            let old: Vec<Found> = add.iter().map(|&i| runs[i].1.clone()).collect();
            for &i in &add {
                let x = runs[i].0;
                if certified(s.env, d, pool, fix, x, &runs[i].1) { best[x] = Some(Some(runs[i].1.clone())); exact_run[i] = true; n_cert += 1; continue; }
                let r = { let ri = &runs[i]; exact_one(&mut s, &mut best, &mut urefs, &mut unp, x, &mut buf, Some((&ri.1, &ri.2[..], st_ver[i]))) };
                if let Some((f, st)) = r { set_wins(&mut s, &mut wtab, x, &f); runs[i] = (x, f, st); } exact_run[i] = true; st_ver[i] = None;
            }
            search_ms += now_ms() - ts;
            let cont2 = cont_of(&mut s, &runs, &cmask(&exact_run), &wtab, &mut buf);
            let same = cont2.iter().zip(&cont).all(|(a, b)| a.lam == b.lam && a.phi == b.phi && a.pinf == b.pinf);
            if same {
                for (k, &i) in add.iter().enumerate() {
                    // the same career against the same base (Cont, so c1, V, the trios and sides, as when it was scored),
                    // nothing it reads changed: its score, lift and table stand (a dropped table stays dropped: its score
                    // is as low as it was, and lo only rises here)
                    if same_career(&old[k], &runs[i].1) && stamp_ok(&s, None, sc_ver[i]) { prof("reuse_sc", 1.0); continue; }
                    let ((sc, lift, ys), v) = stamped(&mut s, None, |s| score_one(s, &runs[i], &c1, vnow, &cont, &combos3, &sides, &mut buf)); score[i] = sc; lifts[i] = lift; ytabs[i] = ys; sc_ver[i] = v;
                }
                prune(&score, &exact_run, &runs, &mut ytabs);
            } else {
                memo_stat(6);
                cont = cont2;
                c1 = (0..n).map(|ui| cont[ui].cost(pnow[ui])).collect();
                let tc = now_ms(); combos = combos_of(&mut s, &c1, &mut buf); sides = sides_of(&mut s, &c1); combo_ms += now_ms() - tc;
                combos3 = combos.iter().map(|c| (c.0, c.1, c.2, c.4, c.5)).collect();
                vnow = trio_v(&c1, &combos3);
                if !vnow.is_finite() { score.clear(); lifts.clear(); ytabs.clear(); break; }
                // (every row scored again: the old tables go first, the new ones kept as above)
                score.clear(); lifts.clear(); ytabs.clear(); sc_ver.clear();
                for r in &runs { let ((sc, lift, ys), v) = stamped(&mut s, None, |s| score_one(s, r, &c1, vnow, &cont, &combos3, &sides, &mut buf)); score.push(sc); lifts.push(lift); ytabs.push(ys); sc_ver.push(v); prune(&score, &exact_run, &runs, &mut ytabs); }
            }
        }
        exact_n = score.len();
    }
    let (c1, cont, combos, combos3, vnow, sides) = (c1, cont, combos, combos3, vnow, sides);
    let step = |ui: usize, real: bool| if real { 0.0 } else { c1[ui] };
    let best_combo = combos.iter().min_by(|x, y| (step(x.0, x.4) + step(x.1, x.5) + x.2).partial_cmp(&(step(y.0, y.4) + step(y.1, y.5) + y.2)).unwrap()).copied();
    let index_ms = now_ms() - t3;
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
                        let (_, _, _, st) = score_career(&mut s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, false, &mut buf);
                        let pe = effs(x, &st);
                        let psc = if score.is_empty() { None } else { Some(score_career(&mut s, d, &base, legs, pool, x, f.a, f.b, &f.sel, false, &mut buf).0 - 1.0) };
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
    // a looped row that repeats another row's career (a builder's exact search can land on the same 8/8 pair): left out,
    // the pick moved to the other (the same career, the same score)
    let pair = |f: &Found| (f.a.min(f.b), f.a.max(f.b));
    let dup: Vec<bool> = (0..runs.len()).map(|i| i >= nbase && (0..nbase).any(|j| runs[j].0 == runs[i].0 && pair(&runs[j].1) == pair(&runs[i].1))).collect();
    let undup: Vec<usize> = (0..runs.len()).map(|i| if dup[i] { (0..nbase).find(|&j| runs[j].0 == runs[i].0 && pair(&runs[j].1) == pair(&runs[i].1)).unwrap() } else { i }).collect();
    // the pick's upper bound per outcome (`outcome_hi`; the pick as below, with the same scores)
    progress(6, 0, 0);   // the pick's outcome bound
    let ys_hi: Option<(usize, (Vec<f64>, Vec<f64>))> = if !runs.is_empty() && !score.is_empty() && vnow.is_finite() {
        let total = |i: usize| score.get(i).copied().unwrap_or(f64::NEG_INFINITY);
        let mut pk = 0usize; for i in 0..runs.len() { if total(i) > total(pk) + 1e-9 { pk = i; } }
        let top = total(pk); let near = |i: usize| total(i) >= top - NEAR.max(0.05 * top.abs());
        for i in 0..runs.len() { if near(i) && runs[i].1.val > runs[pk].1.val { pk = i; } }
        pk = undup[pk];
        let base = ScoreBase { cands: &cands, c1: c1.clone(), m23: &zeros, v0: vnow, pnow: pnow.clone(), cont: cont.clone(), combos: combos3.clone(), sides: &sides, all: &all };
        let (x, f) = (runs[pk].0, runs[pk].1.clone());
        assert!(!ytabs[pk].is_empty(), "the pick's outcome table was dropped");
        let th = now_ms(); let r = outcome_hi(&mut s, d, &base, legs, pool, fix, sv, &ytabs[pk], x, f.a, f.b, &f.sel, &mut buf); prof("hi_ms", now_ms() - th);
        Some((pk, r))
    } else { None };
    drop(s);
    { let key = bests_key(job, fix, nl); let mut kb = best.clone(); kb[g.t] = None;   // the main search's best is per job, searched again (cheap) next time
      BESTS.with(|c| { let mut c = c.borrow_mut(); c.retain(|b| b.key != key); c.push(Bests { key, best: kb, refs: urefs, looped: lbest, unp: unp.clone(), lunp: lunp.clone() }); if let Some(p) = prev { c.push(p); } while c.len() > 4 { c.remove(0); } }); }
    // nothing scored (no trio: fewer than two candidates of different umas): the best odds is the one option and the pick
    if runs.is_empty() && !pj.value_only { for f in &sv.found { runs.push((f.t, f.clone(), vec![])); } }
    // pick: the highest score; within NEAR of it (the model's noise), the better odds of her own 8/8
    let total = |i: usize| score.get(i).copied().unwrap_or(f64::NEG_INFINITY);
    let mut pick = 0usize;
    for i in 0..runs.len() { if total(i) > total(pick) + 1e-9 { pick = i; } }
    let top = if runs.is_empty() { 0.0 } else { total(pick) };
    let near = |i: usize| total(i) >= top - NEAR.max(0.05 * top.abs());
    for i in 0..runs.len() { if near(i) && runs[i].1.val > runs[pick].1.val { pick = i; } }
    if let Some(&u) = undup.get(pick) { pick = u; }
    // per career: the trios' last step (C on A's and B's real 8/8s) only on two 8/8 parents
    let role = |u: usize, f: &Found| match best_combo { Some((ia, ib, _, _, _, _)) if u == cands[ia] || u == cands[ib] => "plan",
        Some((_, _, _, cu, ra, rb)) if u == cu && ra && rb && looped(f) => "finish",
        _ => if members.contains(&u) { "loop" } else if cands.contains(&u) { "candidate" } else { "builder" } };
    let mut opts: Vec<Value> = vec![];
    for (i, (x, f, _)) in runs.iter().enumerate() {
        if dup.get(i).copied().unwrap_or(false) { continue; }
        let mut rp = env.report(pool, f, false);
        let o = rp.as_object_mut().unwrap();
        o.insert("role".into(), json!(role(*x, f))); o.insert("best_odds".into(), json!(*x == g.t && pair(f) == pair(&g))); o.insert("chosen".into(), json!(i == pick));
        // both parents 8/8s: her 8/8 here is a fully looped parent
        o.insert("looped".into(), json!(looped(f)));
        // false: a builder the plan could not turn on, her career from her deck class's winning decks (no career search)
        o.insert("exact".into(), json!(exact_run.get(i).copied().unwrap_or(true)));
        // her career search ran out of its work cap (`search_work`): the best it found, not proven her best
        if exact_run.get(i).copied().unwrap_or(false) && (if i < nbase { unp[*x] } else { lunp[*x] }) { o.insert("proven".into(), json!(false)); }
        o.insert("stage_cost".into(), json!(cands.iter().position(|&c| c == *x).map(|ui| c1[ui])));
        o.insert("index".into(), json!(score.get(i))); o.insert("saving".into(), json!(score.get(i).map(|v| v + 1.0)));
        o.insert("lift".into(), json!(lifts.get(i))); o.insert("near".into(), json!(near(i) && i != pick));
        o.insert("score".into(), json!(if score.is_empty() { None } else { Some(total(i)) }));
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
        // per outcome where y < 0.5, at least y: the most her veteran's direct lifts could save (`outcome_hi`; rounded
        // up); null where y >= 0.5 (the page reads it only below 0.5: there it checks the saving exactly)
        "y_hi": ys_hi.as_ref().filter(|(pk, (h, _))| *pk == pick && h.len() == ys.len()).map(|(_, (h, _))| h.iter().zip(ys).map(|(hv, y)| if *y < 0.5 { json!((hv.max(*y) * 100.0).ceil() / 100.0) } else { Value::Null }).collect::<Vec<Value>>()),
        // the chance a roll reads no saving from y but could save at least half a career by y_hi (the page checks those)
        "provisional": ys_hi.as_ref().filter(|(pk, (h, _))| *pk == pick && h.len() == ys.len()).map(|(_, (h, pr))| (0..ys.len()).filter(|&i| ys[i] < 0.5 && h[i].max(ys[i]) >= 0.5).map(|i| pr[i]).sum::<f64>())}), _ => Value::Null };
    let mut out = json!({"options": opts, "choice": choice, "rule": "index", "mode": "trio", "v0": if vnow.is_finite() { json!(vnow) } else { Value::Null }, "outcomes": outcomes,
           "members": members_json,
           "v0_plan": plan_json, "fit": (0..n).map(|ui| json!([d.umas[cands[ui]].id, cont[ui].lam, cont[ui].phi, cont[ui].pinf])).collect::<Vec<_>>(), "loop_members": minfo, "loop_note": mwhy, "hint": hint, "umas_scored": exact_n, "umas_searched": exact_run.iter().filter(|&&e| e).count() - n_cert, "umas_certified": n_cert, "umas_unproven": (0..d.umas.len()).filter(|&u| unp[u] || lunp[u]).count(), "work": work,
           "sur": pj.sur, "sur_diff": sur_diff, "loop_ms": loop_ms, "search_ms": search_ms, "combo_ms": combo_ms, "index_ms": index_ms, "total_ms": now_ms() - t_all});
    if !proxy_check.is_null() { out["proxy_check"] = proxy_check; }
    out["work"]["prof"] = json!(PROF.with(|p| std::mem::take(&mut *p.borrow_mut())));
    out
}
