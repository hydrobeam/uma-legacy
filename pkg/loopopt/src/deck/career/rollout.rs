//! The planner's pick of the next career: a one-step rollout with a stationary tail over the K best careers by P(all)
//! (`rollout: K`; docs/SOLVER.md "Rollout").
//!
//! For each candidate career a (trainee, parents, her best deck):
//!
//!   Q(a) = 1 + sum over S != all of P_a(S) / f_a(S)
//!
//! P_a(S): the exact chance her new veteran lands target set S (`outcome_sets`). f_a(S): the best P(all) of a next career
//! once that veteran (stars from the rank mix, G1 wins her schedule's chances, parents a's parents) joins the pool,
//! at least p* (today's best P(all); the player can always run that career). Derived: Q(a) is the expected number of
//! careers of "run a, then repeat the best career it leaves", and when no career is improved Q = 1/p* for every a
//! that is today's best. The rollout runs the candidate with the lowest Q. Q counts every improvement as permanent and
//! no later one, so it ranks careers; it is no forecast of careers (under the model the policy's measured value on
//! career_U4 is 230 careers where Q at the start is 568).
//!
//! f_a(S) through corner tables: for a trainee and a fixed deck, P(all) per outfit is multilinear in the per-target
//! inheritance chances (derived; `corners`), so 2^nu exact evaluations per (trainee, deck) give it at any chances.
//! Decks: the search's fully checked decks (`Ref`), plus a deck search wherever a setup's bound says the cached decks
//! may fall short. Per candidate, the best SHORTLIST setups with her landing every target (stars averaged: exact for a
//! fixed deck, since the stars are independent of everything else and P(all) is multilinear in each target's chance)
//! are found best first: every legal (trainee, partner) gets a ceiling (the search's, at each target's best possible
//! deck odds) and, from each fully checked deck of that trainee, the `rise` bound; setups are taken by ceiling; where a
//! bound exceeds the cached decks' table value, her deck is searched at that setup's chances. The shortlist is then
//! proven at S = all under EXACT_RHO (`shortlist_proven`) unless a deck search ran out of work. For S != all, f_a(S)
//! is the best table value over the shortlist's setups with her stars set by S: exact for those setups' cached decks,
//! a floor otherwise (another deck or another setup could do better for a veteran that lands only part of the set).
use super::*;

/// Setups kept per candidate (DESIGN: 4 and 12 gave the same decision at every step of 40 simulated U4 trajectories)
pub(super) const SHORTLIST: usize = 4;

/// A cached deck of one trainee and per outfit its corner table (P(all), every spark scale 1, at q_k in {0, 1} per
/// uncertain target; index bit k set = q_k = 0, as `corners` builds them from q0 = 0)
#[derive(Clone)]
pub(super) struct CDeck { pub(super) sel: Sel, pub(super) tab: Vec<Vec<f64>> }
/// The rollout's deck cache: per trainee her decks with corner tables and her fully checked decks (`Ref`, for `rise`)
#[derive(Clone)]
pub(super) struct Cache { pub(super) decks: Vec<Vec<CDeck>>, pub(super) refs: Vec<Vec<Ref>>, pub tables: usize, pub searches: usize, pub scored: usize }

impl Cache {
    pub(super) fn new(n: usize) -> Cache { Cache { decks: (0..n).map(|_| vec![]).collect(), refs: vec![vec![]; n], tables: 0, searches: 0, scored: 0 } }
    /// a deck of trainee t (its corner table built now); a deck already cached is skipped
    pub(super) fn add(&mut self, env: &mut Env, t: usize, sel: &Sel) {
        if self.decks[t].iter().any(|d| d.sel.ents == sel.ents && d.sel.plain == sel.plain && d.sel.filler == sel.filler) { return; }
        let tab = env.corners(t, sel, &vec![0.0; env.ntar]);
        self.tables += 1;
        self.decks[t].push(CDeck { sel: sel.clone(), tab });
    }
    /// a search's fully checked decks
    pub(super) fn absorb(&mut self, env: &mut Env, refs: &[Vec<Ref>]) {
        for (t, rs) in refs.iter().enumerate() { for r in rs { self.add(env, t, &r.sel); self.refs[t].push(r.clone()); } }
    }
    pub(super) fn n_decks(&self) -> usize { self.decks.iter().map(|d| d.len()).sum() }
}

impl<'a> Env<'a> {
    /// P(all) of trainee t at chances q (job-target order) with spark scale c, through her cached decks' tables (the
    /// best deck and outfit); 0 without a cached deck
    fn tval(&self, cache: &Cache, t: usize, q: &[f64], c: f64, buf: &mut Vec<f64>) -> f64 {
        let x: Vec<f64> = self.uncv.iter().map(|&i| 1.0 - q[i]).collect();
        c * cache.decks[t].iter().flat_map(|d| d.tab.iter()).map(|tab| interp_in(tab, &x, buf)).fold(0.0f64, f64::max)
    }

    /// The rollout over `cands` (module notes). Its deck searches share one work budget; `p_star` is today's best.
    pub(super) fn rollout(&mut self, p: &Pool, legs: &[Legacy], cands: &[Found], p_star: f64, cache: &mut Cache) -> (Vec<Value>, Option<usize>, bool, f64) {
        let ntar = self.ntar;
        let full = (1usize << ntar) - 1;
        self.work = 0;
        let owned = self.owned.clone();
        let mut buf = vec![];
        let mut proven = true;
        let mut rows = vec![];
        let mut pick: Option<(f64, usize)> = None;
        let mut check = 0.0f64;
        for (ci, f) in cands.iter().enumerate() {
            let (t, a, b) = (f.t, f.a, f.b);
            // the exact chance of each landed set (bit i = job target i)
            let (q, nc) = inherit(&self.slots(p, t, a, b), ntar);
            self.set_trainee(t, &q);
            let (j, ext) = self.exact(t, &f.sel, &nc, false, true);
            let ps = self.outcome_sets(j.2, &ext);
            // her veteran, landing every target, in the pool
            let (_, wp) = self.schedule(p, t, a, b);
            let mut legs2 = legs.to_vec(); legs2.push(self.veteran(legs, t, a, b, wp)); let vi = legs2.len() - 1;
            let p2 = self.pool(&legs2);
            let vc = p2.me[vi].chara;
            // every legal next career with her as a parent, by ceiling
            struct Su { t: usize, b: usize, q: Vec<f64>, c: f64, hi: f64 }
            let mut su: Vec<Su> = vec![];
            for &t2 in &owned {
                let id2 = self.d.umas[t2].id;
                if id2 == vc { continue; }
                for b2 in 0..vi {
                    let cb = p2.me[b2].chara;
                    if cb == id2 || cb == vc { continue; }
                    let (q2, _, c2) = self.lineage(&p2, t2, b2, vi);
                    let Some(hi) = self.ceiling(t2, &q2, c2) else { continue };
                    su.push(Su { t: t2, b: b2, q: q2, c: c2, hi });
                }
            }
            su.sort_by(|x, y| y.hi.partial_cmp(&x.hi).unwrap());
            // best first: the SHORTLIST best setups with her landing every target
            let mut top: Vec<(f64, usize)> = vec![];
            let (mut searched, mut visited) = (0usize, 0usize);
            for i in 0..su.len() {
                let thr = if top.len() >= SHORTLIST { top[SHORTLIST - 1].0.max(p_star) } else { p_star };
                if su[i].hi * (1.0 + 1e-9) < thr { break; }
                visited += 1;
                let (t2, c2) = (su[i].t, su[i].c);
                let rb = cache.refs[t2].iter().map(|r| c2 * r.val * self.rise(t2, &su[i].q, &r.q)).fold(su[i].hi, f64::min);
                if rb * (1.0 + 1e-9) < thr { continue; }
                let mut lo = self.tval(cache, t2, &su[i].q, c2, &mut buf);
                if rb > lo * (1.0 + 1e-9) {
                    // her cached decks may fall short here: her best deck at these chances
                    self.set_trainee(t2, &su[i].q);
                    let starts: Vec<Sel> = cache.decks[t2].iter().map(|d| d.sel.clone()).collect();
                    let sr: Vec<&Sel> = starts.iter().collect();
                    let mut sc = 0;
                    let r = self.best_deck(t2, c2, thr, &sr, &mut sc);
                    cache.scored += sc;
                    if let DeckRes::Done(v, sel, fl) = r {
                        searched += 1; cache.searches += 1;
                        cache.add(self, t2, &sel);
                        if fl { cache.refs[t2].push(Ref { q: su[i].q.clone(), val: v, sel }); } else { proven = false; }
                        lo = self.tval(cache, t2, &su[i].q, c2, &mut buf);
                    }
                }
                if lo > p_star {
                    let pos = top.iter().position(|x| x.0 < lo).unwrap_or(top.len());
                    top.insert(pos, (lo, i)); top.truncate(SHORTLIST);
                }
            }
            // f(S) per landed set over the shortlist: her stars set by S (averaged over the rank mix)
            let mut fs = vec![p_star; 1 << ntar];
            for &(_, i) in &top {
                let (t2, b2) = (su[i].t, su[i].b);
                let mut sl = self.slots(&p2, t2, b2, vi);
                let iv = sl.iter().position(|x| x.parent && x.legacy == vi).unwrap();
                for m in 0..full {
                    if ps[m] == 0.0 { continue; }
                    for k in 0..ntar { sl[iv].stars[k] = if m >> k & 1 == 1 { 2 } else { 0 }; }
                    let (q2, nc2) = inherit(&sl, ntar);
                    let c2: f64 = nc2.iter().map(|&n| 1.1f64.powi(n - 6)).product();
                    let v = self.tval(cache, t2, &q2, c2, &mut buf);
                    if v > fs[m] { fs[m] = v; }
                }
            }
            // the check: the best setup's table value against a direct exact evaluation over her cached decks, with her
            // landing every target and with the likeliest failed set
            if let Some(&(_, i0)) = top.first() {
                let t2 = su[i0].t;
                let mut sl = self.slots(&p2, t2, su[i0].b, vi);
                let iv = sl.iter().position(|x| x.parent && x.legacy == vi).unwrap();
                let worst = (0..full).max_by(|&x, &y| ps[x].partial_cmp(&ps[y]).unwrap()).unwrap_or(0);
                let sels: Vec<Sel> = cache.decks[t2].iter().map(|d| d.sel.clone()).collect();
                for m in [full, worst] {
                    for k in 0..ntar { sl[iv].stars[k] = if m >> k & 1 == 1 { 2 } else { 0 }; }
                    let (q2, nc2) = inherit(&sl, ntar);
                    let c2: f64 = nc2.iter().map(|&n| 1.1f64.powi(n - 6)).product();
                    let via = self.tval(cache, t2, &q2, c2, &mut buf);
                    self.set_trainee(t2, &q2);
                    let direct = sels.iter().map(|sel| self.exact(t2, sel, &nc2, false, false).0 .0).fold(0.0f64, f64::max);
                    check = check.max((via - direct).abs() / direct.max(1e-300));
                }
            }
            let (mut qv, mut fail, mut ef, mut imp) = (1.0f64, 0.0f64, 0.0f64, 0.0f64);
            for m in 0..full {
                if ps[m] == 0.0 { continue; }
                qv += ps[m] / fs[m]; fail += ps[m]; ef += ps[m] * fs[m];
                if fs[m] > p_star * (1.0 + 1e-9) { imp += ps[m]; }
            }
            let f_fail = if fail > 0.0 { ef / fail } else { p_star };
            if pick.map_or(true, |(bq, _)| qv < bq) { pick = Some((qv, ci)); }
            let best = top.first().map(|&(v, i)| json!({"trainee": {"id": self.d.umas[su[i].t].id, "name": self.d.umas[su[i].t].name}, "partner": su[i].b, "p_all": v}));
            rows.push(json!({"q": qv, "p_all": f.val, "p_all_sets": ps[full], "f_fail": f_fail, "gain": f_fail / p_star,
                "improve": if fail > 0.0 { imp / fail } else { 0.0 }, "best_next": best, "setups": su.len(), "visited": visited, "searched": searched}));
        }
        (rows, pick.map(|x| x.1), proven, check)
    }

    /// p_bar: the best P(all) any pool could give, max over the owned trainees of P(all) with every target inherited
    /// for sure (q = 1) and all six legacies carrying every spark (scale 1), each with her best deck there. Best first
    /// by the search's ceiling; `proven` when every trainee searched finished and the next ceiling is below the best.
    pub(super) fn p_bar(&mut self) -> (f64, Option<usize>, bool) {
        self.work = 0;
        let one = vec![1.0f64; self.ntar];
        let owned = self.owned.clone();
        let mut cs: Vec<(f64, usize)> = owned.iter().filter_map(|&t| self.ceiling(t, &one, 1.0).map(|u| (u, t))).collect();
        cs.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        let (mut best, mut who, mut proven) = (0.0f64, None, true);
        let mut sc = 0;
        for &(ub, t) in &cs {
            if ub * (1.0 + 1e-9) < best { break; }
            self.set_trainee(t, &one);
            if let DeckRes::Done(v, _, full) = self.best_deck(t, 1.0, best, &[], &mut sc) {
                if !full { proven = false; }
                if v > best { best = v; who = Some(t); }
            }
        }
        (best, who, proven)
    }
}

/// p_bar per job fingerprint (targets, roster, cards, scenario, ...: everything but the pool and the places), so the
/// page's searches on one setup pay for it once per solver instance
pub(super) fn p_bar_cached(env: &mut Env, v: &Value) -> (f64, Option<u32>, bool) {
    thread_local! { static PBAR: std::cell::RefCell<Vec<(u64, (f64, Option<u32>, bool))>> = const { std::cell::RefCell::new(Vec::new()) }; }
    let fp = setup_fp(v);
    if let Some(x) = PBAR.with(|c| c.borrow().iter().find(|e| e.0 == fp).map(|e| e.1)) { return x; }
    let (p, t, proven) = env.p_bar();
    let x = (p, t.map(|t| env.d.umas[t].id), proven);
    PBAR.with(|c| { let mut c = c.borrow_mut(); c.push((fp, x)); if c.len() > 8 { c.remove(0); } });
    x
}
