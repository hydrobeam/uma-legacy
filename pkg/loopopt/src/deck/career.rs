//! Career planner: the best next career from the veterans a player actually has (their own, plus borrowed ones).
//!
//! The loop planner assumes a running loop: every legacy carries every target spark (stars from the rank mix) and
//! race overlaps come from planned schedules. Here each candidate career (trainee T, parents L1 and L2 from the
//! veteran list, at most one borrowed) uses what those veterans really carry:
//!   - affinity per legacy from relation points and the G1 wins on record: a parent P (other parent Q, P's own
//!     parents G) counts pair(T,P) + pair(P,Q) + 3 per G1 shared by P and Q + sum over G of [tri(T,P,G) + 3 per G1
//!     shared by P and G]; a grandparent G under P counts tri(T,P,G) + 3 per G1 shared by P and G (skills.rs, same
//!     fit); a veteran whose parents are unknown adds no grandparent slots;
//!   - per target k, the inheritance chance q_k = 1 - prod over the six legacies (1 - min(1, 0.03 s (1 + A/100)))^2,
//!     s = the stars of that legacy's spark of k (0 if it has none);
//!   - spark generation 0.2 / 0.4 x 1.1^n_k for a white / gold, n_k = how many of the six carry a spark of k.
//! Everything else is the loop planner's model. q_k and the trainee's own event hint chance ew enter every formula
//! only as (1 - ew)(1 - q_k), so T's event hint chances are replaced by 1 - (1 - ew)(1 - q_k) and the deck search
//! runs at q = 0: its products, bounds and exact odds are then the career's, apart from the factor 1.1^(n_k - 6) per
//! target, which is the same for every deck and multiplies the result (`JointExt` applies it per target where the
//! number is not a plain product: P(all but one), the per-target chances).
//!
//! Search: every legal candidate gets a ceiling, EXACT_RHO x C x the product value at each target's best possible
//! deck odds (the deck_size lowest miss chances of any legal entries, links firing for sure); candidates are solved
//! best ceiling first (best product deck, then every deck whose product value times EXACT_RHO reaches the best exact
//! value, scored exactly, as in the per-uma search) until the next ceiling is below the top-k exact value. The top k
//!
//! Schedule: the trainee races to win the G1s her parents won (the race weights of `best_response` are the parents'
//! wins), since each G1 the new veteran shares with a parent adds 3 to her affinity and 3 to the parent's when she is
//! later a parent herself (the loop planner's schedule rule with the other members' wins replaced by the parents').
//!
//! Lookahead (`lookahead: true`, for the best career): the new veteran V lands each set S of targets with its exact
//! chance (`JointExt::sets`), each landed spark's stars from the rank mix (`star_dist`); her G1 wins are her schedule's
//! win chances, her parents the career's. f(S) = the best P(all) of a next career with V in the pool: the best
//! setups with V as a parent are found for V landing every target (the career search with V fixed as a parent), and
//! each S is scored on them (stars averaged: every landing factor is affine in q_k and the stars are independent, so
//! that average is exact for a fixed setup); f(S) is at least today's best P(all) (`p_now`). A lower bound: another
//! setup could do better for some S. Then, counting careers until a veteran lands every target, with a setup run again
//! and again once chosen: moving on with V costs 1/f(S); running this career once more and keeping the better
//! veteran costs 1 + sum over S' (not all) P(S') / max(f(S), f(S')). Reroll when that is lower.
use super::*;

#[derive(Deserialize, Clone, Default)]
pub(crate) struct Ancestor {
    pub chara: u32,
    /// (skill group, stars 1..=3) of its white skill sparks
    #[serde(default)] pub sparks: Vec<(u32, u8)>,
    /// race indices (data.json `races`) of the single-race G1s it won
    #[serde(default)] pub wins: Vec<usize>,
    /// (aptitude factor group, stars) of its pink sparks: 11 turf, 12 dirt, 21-24 front runner .. end closer, 31-34
    /// sprint .. long; reported (inheritance chance per aptitude), not part of the odds
    #[serde(default)] pub pinks: Vec<(u32, u8)>,
}
#[derive(Deserialize, Clone, Default)]
pub(crate) struct Legacy {
    pub chara: u32,
    #[serde(default)] pub borrowed: bool,
    #[serde(default)] pub sparks: Vec<(u32, u8)>,
    #[serde(default)] pub wins: Vec<usize>,
    #[serde(default)] pub pinks: Vec<(u32, u8)>,
    /// its own parents (the new trainee's grandparents); empty when unknown
    #[serde(default)] pub parents: Vec<Ancestor>,
    /// internal (the lookahead's new veteran): win chance per race index in place of `wins`, and the star mix her
    /// sparks are drawn from in place of fixed stars
    #[serde(skip)] win_p: Vec<f64>,
    #[serde(skip)] mix: Option<[f64; 3]>,
}
fn d_top() -> usize { 5 }
#[derive(Deserialize)]
struct CareerJob {
    legacies: Vec<Legacy>,
    /// trainees to consider (chara ids); empty = every owned uma
    #[serde(default)] trainees: Vec<u32>,
    #[serde(default = "d_top")] top: usize,
    /// legacies (indices) that must be among the two parents
    #[serde(default)] fix: Vec<usize>,
    /// one result per trainee (false: the best careers whoever trains, e.g. with the trainee fixed: the best parents)
    #[serde(default = "d_true")] per_trainee: bool,
    /// the reroll-or-move-on lookahead for the best career
    #[serde(default)] lookahead: bool,
    /// today's best P(all) over every career (the lookahead's floor); computed when absent
    #[serde(default)] p_now: Option<f64>,
}

/// What a lineage counts of one legacy: relation index, G1 wins as (race, chance) sorted by race, stars per job target,
/// pink sparks, and the star mix when the stars are random.
struct Who { ix: Option<usize>, chara: u32, wins: Vec<(usize, f64)>, stars: Vec<u8>, pinks: Vec<(u32, u8)>, mix: Option<[f64; 3]> }
/// The veterans the parents come from, each with her own parents.
struct Pool { me: Vec<Who>, gp: Vec<Vec<Who>>, borrowed: Vec<bool> }
/// One legacy slot of a lineage: affinity parts and per-target stars.
#[derive(Clone)]
struct Slot { parent: bool, legacy: usize, gp: Option<usize>, chara: u32, rel: f64, race: f64, stars: Vec<u8>, pinks: Vec<(u32, u8)>, mix: Option<[f64; 3]> }
/// G1s two legacies both won (expected count when a win is a chance)
fn shared(a: &Who, b: &Who) -> f64 {
    let (mut i, mut j, mut s) = (0, 0, 0.0f64);
    while i < a.wins.len() && j < b.wins.len() {
        match a.wins[i].0.cmp(&b.wins[j].0) { std::cmp::Ordering::Less => i += 1, std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => { s += a.wins[i].1 * b.wins[j].1; i += 1; j += 1; } }
    }
    s
}
/// Per job target: the inheritance chance and how many of the six carry the spark. A slot with random stars (`mix`)
/// enters as the average of its squared miss factor (its stars are independent of everything else).
fn inherit(sl: &[Slot], ntar: usize) -> (Vec<f64>, Vec<i32>) {
    let mut q = vec![0.0f64; ntar]; let mut nc = vec![0i32; ntar];
    for k in 0..ntar {
        let (mut miss, mut mix2) = (1.0f64, 1.0f64);
        for s in sl { let st = s.stars[k]; if st == 0 { continue; } nc[k] += 1;
            let m = |st: f64| 1.0 - (0.03 * st * (1.0 + (s.rel + s.race) / 100.0)).min(1.0);
            match s.mix { None => miss *= m(st as f64), Some(w) => mix2 *= (0..3).map(|i| w[i] * m(i as f64 + 1.0).powi(2)).sum::<f64>() } }
        q[k] = 1.0 - miss * miss * mix2;
    }
    (q, nc)
}
/// A candidate the search solved: trainee, parents (legacy indices), value C x E, its ceiling, its best deck.
struct Found { t: usize, a: usize, b: usize, val: f64, ub: f64, sel: Sel }
struct Solved { found: Vec<Found>, cands: usize, solved: usize, bounded: usize, scored: usize, skipped: usize, next_bound: f64, complete: bool, proven: bool }

struct Env<'a> {
    d: &'a Data, sj: &'a SkillJob, model: Model<'a>, ew0: Vec<Vec<Vec<f64>>>, uncv: Vec<usize>, fixed_t: Vec<usize>, ntar: usize, nu: usize,
    unc: Vec<u32>, space: DeckSpace<'a>, dmax: Vec<Option<(Vec<f64>, Vec<f64>)>>, sts0: Vec<Vec<Vec<u8>>>, owned: Vec<usize>, budget: usize, work: usize,
}
impl<'a> Env<'a> {
    fn uix(&self, id: u32) -> Option<usize> { self.d.umas.iter().position(|u| u.id == id) }
    fn stars_of(&self, sp: &[(u32, u8)]) -> Vec<u8> { self.sj.targets.iter().map(|t| sp.iter().filter(|s| s.0 == t.group).map(|s| s.1.min(3)).max().unwrap_or(0)).collect() }
    fn who(&self, chara: u32, sparks: &[(u32, u8)], wins: &[usize], win_p: &[f64], pinks: &[(u32, u8)], mix: Option<[f64; 3]>) -> Who {
        let mut w: Vec<(usize, f64)> = if win_p.is_empty() { let mut v = wins.to_vec(); v.sort(); v.dedup(); v.into_iter().map(|r| (r, 1.0)).collect() }
            else { win_p.iter().enumerate().filter(|x| *x.1 > 0.0).map(|(r, &p)| (r, p)).collect() };
        w.sort_by_key(|x| x.0);
        Who { ix: self.uix(chara), chara, wins: w, stars: self.stars_of(sparks), pinks: pinks.to_vec(), mix }
    }
    fn pool(&self, legs: &[Legacy]) -> Pool {
        Pool { me: legs.iter().map(|l| self.who(l.chara, &l.sparks, &l.wins, &l.win_p, &l.pinks, l.mix)).collect(),
               gp: legs.iter().map(|l| l.parents.iter().map(|g| self.who(g.chara, &g.sparks, &g.wins, &[], &g.pinks, None)).collect()).collect(),
               borrowed: legs.iter().map(|l| l.borrowed).collect() }
    }
    fn pair(&self, a: Option<usize>, b: Option<usize>) -> f64 { match (a, b) { (Some(a), Some(b)) => self.d.pair[a][b] as f64, _ => 0.0 } }
    fn tri(&self, a: Option<usize>, b: Option<usize>, c: Option<usize>) -> f64 { match (a, b, c) { (Some(a), Some(b), Some(c)) => self.d.tri[a][b][c] as f64, _ => 0.0 } }
    /// the six legacy slots of trainee t with parents a and b
    fn slots(&self, p: &Pool, t: usize, a: usize, b: usize) -> Vec<Slot> {
        let ti = Some(t); let mut v = vec![];
        for (x, y) in [(a, b), (b, a)] {
            let px = &p.me[x];
            let mut rel = self.pair(ti, px.ix) + self.pair(px.ix, p.me[y].ix); let mut race = 3.0 * shared(px, &p.me[y]);
            let mut gs = vec![];
            for (gi, g) in p.gp[x].iter().enumerate() {
                let (gr, gc) = (self.tri(ti, px.ix, g.ix), 3.0 * shared(px, g));
                rel += gr; race += gc;
                gs.push(Slot { parent: false, legacy: x, gp: Some(gi), chara: g.chara, rel: gr, race: gc, stars: g.stars.clone(), pinks: g.pinks.clone(), mix: g.mix });
            }
            v.push(Slot { parent: true, legacy: x, gp: None, chara: px.chara, rel, race, stars: px.stars.clone(), pinks: px.pinks.clone(), mix: px.mix });
            v.extend(gs);
        }
        v
    }
    fn scale(&self, nc: &[i32]) -> (Vec<f64>, Vec<f64>) {
        (self.uncv.iter().map(|&i| 1.1f64.powi(nc[i] - 6)).collect(), self.fixed_t.iter().map(|&i| 1.1f64.powi(nc[i] - 6)).collect())
    }
    /// trainee t's event hint chances with her lineage's inheritance chances folded in (the search then runs at q = 0)
    fn set_trainee(&mut self, t: usize, q: &[f64]) {
        let uncv = &self.uncv;
        self.model.ews[t] = self.ew0[t].iter().map(|o| o.iter().zip(uncv).map(|(e, &i)| 1.0 - (1.0 - e) * (1.0 - q[i])).collect()).collect();
    }
    fn ents_of(&self, sel: &Sel) -> Vec<(Src, Kind)> { sel.ents.iter().map(|&j| (self.space.srcs[j].clone(), self.space.kind[j])).collect() }
    /// exact P(all) of trainee t (her inheritance already folded in) with deck `sel`, scaled per target; `sets`: also the
    /// chance of each set of uncertain targets landing
    fn exact(&self, t: usize, sel: &Sel, nc: &[i32], m1: bool, sets: bool) -> ((f64, f64, usize), JointExt) {
        let (su, sf) = self.scale(nc);
        let mut ext = JointExt { su, sf, marg: vec![], sets: if sets { vec![0.0] } else { vec![] } };
        let j = joint_fast(&self.model, t, &[0.0], &self.ents_of(sel), sel.filler, self.ntar, m1, Some(&mut ext))[0];
        (j, ext)
    }

    /// The best careers: trainees from `trainees` (uma indices), two legacies of `p` as parents (all of `fix` among
    /// them, at most one borrowed), each with her best deck; `top` of them, one per trainee when `per_trainee`.
    fn solve(&mut self, p: &Pool, fix: &[usize], trainees: &[usize], per_trainee: bool, top: usize) -> Solved {
        let d = self.d; let ntar = self.ntar;
        self.work = 0;   // each search has its own work budget
        struct Cand { t: usize, a: usize, b: usize, ub: f64, c: f64, q: Vec<f64> }
        let mut cands: Vec<Cand> = vec![];
        let nl = p.me.len();
        for &t in trainees {
            let tid = d.umas[t].id;
            let Some((dw, dg)) = self.dmax[t].clone() else { continue };
            for a in 0..nl { for b in a + 1..nl {
                let (ca, cb) = (p.me[a].chara, p.me[b].chara);
                if ca == cb || ca == tid || cb == tid || (p.borrowed[a] && p.borrowed[b]) || !fix.iter().all(|&f| f == a || f == b) { continue; }
                let (q, nc) = inherit(&self.slots(p, t, a, b), ntar);
                let (su, sf) = self.scale(&nc);
                let c = su.iter().chain(&sf).product::<f64>();
                self.set_trainee(t, &q);
                let pv = (0..self.model.n_outfits(t)).map(|o| self.model.outfit_fast(t, o, 0.0, &dw, &dg)).fold(0.0f64, f64::max);
                cands.push(Cand { t, a, b, ub: EXACT_RHO * c * self.model.fixed_factor() * pv, c, q });
            } }
        }
        cands.sort_by(|x, y| y.ub.partial_cmp(&x.ub).unwrap());
        let top = top.max(1);
        let mut complete = true;
        let mut done: Vec<(f64, usize, Sel)> = vec![];
        let kth = |done: &[(f64, usize, Sel)]| if done.len() < top { f64::NEG_INFINITY } else { done[top - 1].0 };
        let (mut n_solved, mut n_bounded, mut n_scored) = (0usize, 0usize, 0usize);
        let mut next = 0;
        // one result per trainee: a candidate only matters if it beats the k-th best and its trainee's best so far
        let thr = |done: &[(f64, usize, Sel)], cands: &[Cand], t: usize| if !per_trainee { kth(done) } else { done.iter().find(|x| cands[x.1].t == t).map_or(kth(done), |x| x.0.max(kth(done))) };
        // How far trainee t's best exact value (before the spark scale) can rise from chances q0 to q. A target the
        // outfit lacks lands with GG g + GW (1 - g - n (1 - e)(1 - q_k)) per chain state (g: gold hinted, n: no hint,
        // e: her own event hint chance); from q0 to q it grows at most by (1 - (1 - e)(1 - q_k)) / (1 - (1 - e)(1 - q0_k))
        // (n = 1, g = 0), and not at all when q_k <= q0_k or the outfit has the skill. P(all) is a sum of products of
        // these factors, so it grows at most by the product, for every deck: E(q) <= E(q0) x max over outfits of it.
        let rise = |env: &Env, t: usize, q: &[f64], q0: &[f64]| -> f64 {
            env.sts0[t].iter().zip(&env.ew0[t]).map(|(st, ew)| env.uncv.iter().enumerate().map(|(k, &i)| {
                if st[k] > 0 || q[i] <= q0[i] { 1.0 } else { (1.0 - (1.0 - ew[k]) * (1.0 - q[i])) / (1.0 - (1.0 - ew[k]) * (1.0 - q0[i])) }
            }).product::<f64>()).fold(0.0f64, f64::max)
        };
        // per trainee, her fully checked candidates: (candidate, exact best value before the spark scale, its deck)
        let mut refs: Vec<Vec<(usize, f64, Sel)>> = vec![vec![]; d.umas.len()];
        while next < cands.len() && cands[next].ub * (1.0 + 1e-9) >= kth(&done) {
            let ci = next; next += 1;
            let (t, c) = (cands[ci].t, cands[ci].c);
            let th = thr(&done, &cands, t);
            if cands[ci].ub * (1.0 + 1e-9) < th { n_bounded += 1; continue; }
            let rb = refs[t].iter().map(|r| c * r.1 * rise(self, t, &cands[ci].q, &cands[r.0].q)).fold(f64::INFINITY, f64::min);
            if rb * (1.0 + 1e-9) < th { n_bounded += 1; continue; }
            let uid = d.umas[t].id;
            self.set_trainee(t, &cands[ci].q);
            let n0 = self.space.nodes.get();
            let (r, _) = self.space.search(&self.model, &[t], &[uid], &[(vec![(0, 0.0)], 0.0)], true, f64::NEG_INFINITY, 1.0, 1);
            self.work += 2 * (self.space.nodes.get() - n0) + 10_000;
            let Some((pv, sel)) = r.into_iter().next() else { continue };
            // the candidate's own ceiling from its best product deck
            if EXACT_RHO * c * pv * (1.0 + 1e-9) < th { n_bounded += 1; continue; }
            n_solved += 1;
            let score = |env: &Env, sel: &Sel| joint(&env.model, t, &[0.0], &env.ents_of(sel), sel.filler, ntar, false)[0].0;
            let mut best = (score(self, &sel), sel); n_scored += 1; self.work += 200;
            // her other checked candidates' decks are good starts (a higher start leaves fewer decks to check)
            for r in &refs[t] { let x = score(self, &r.2); n_scored += 1; self.work += 200; if x > best.0 { best = (x, r.2.clone()); } }
            let mut full = false;
            if self.work <= self.budget {
                let n0 = self.space.nodes.get();
                let (mut more, over) = self.space.search(&self.model, &[t], &[uid], &[(vec![(0, 0.0)], best.0)], false, f64::NEG_INFINITY, EXACT_RHO, EXACT_CAP);
                self.work += 2 * (self.space.nodes.get() - n0) + 5 * more.len() + 10_000;
                full = !over;
                more.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
                if full { for (v, s) in more {
                    if EXACT_RHO * v * (1.0 + 1e-9) < best.0 { break; }
                    if self.work > self.budget { full = false; break; }
                    if s.ents == best.1.ents && s.plain == best.1.plain && s.filler == best.1.filler { continue; }
                    let e: Vec<(&Src, Kind)> = s.ents.iter().map(|&j| (&self.space.srcs[j], self.space.kind[j])).collect();
                    if let Some(ub) = upper_na(&self.model, t, &[0.0], &e, s.filler) { self.work += 25; if ub[0] * (1.0 + 1e-9) < best.0 { continue; } }
                    let x = score(self, &s); n_scored += 1; self.work += 200;
                    if x > best.0 { best = (x, s); }
                } }
            }
            if full { refs[t].push((ci, best.0, best.1.clone())); } else { complete = false; }
            let val = c * best.0;
            if per_trainee { if let Some(i) = done.iter().position(|x| cands[x.1].t == t) { if done[i].0 >= val { continue; } done.remove(i); } }
            let pos = done.iter().position(|x| x.0 < val).unwrap_or(done.len());
            done.insert(pos, (val, ci, best.1));
        }
        let next_bound = if next < cands.len() { cands[next].ub } else { 0.0 };
        let proven = complete && !done.is_empty() && (done.len() >= top.min(cands.len()) || next >= cands.len()) && next_bound < done[0].0;
        let n = cands.len();
        Solved { found: done.into_iter().take(top).map(|(val, ci, sel)| Found { t: cands[ci].t, a: cands[ci].a, b: cands[ci].b, val, ub: cands[ci].ub, sel }).collect(),
                 cands: n, solved: n_solved, bounded: n_bounded, scored: n_scored, skipped: n - next, next_bound, complete, proven }
    }

    /// Trainee t's race schedule with parents a and b: the G1s they won weigh one each (a G1 she shares with a parent
    /// adds affinity when she is a parent herself); the rows of the races she runs, and her win chance per race index.
    fn schedule(&self, p: &Pool, t: usize, a: usize, b: usize) -> (Value, Vec<f64>) {
        let d = self.d; let u = &d.umas[t];
        let mut wr = vec![0.0f64; d.nr];
        for x in [a, b] { for &(r, pr) in &p.me[x].wins { if r < d.nr { wr[r] += pr; } } }
        let Some((s, _)) = best_response(d, u, &wr, None, None) else { return (json!([]), vec![0.0; d.nr]) };
        let wp = probs(d, u, &s);
        let won = |x: usize, r: usize| p.me[x].wins.iter().find(|w| w.0 == r).map_or(0.0, |w| w.1);
        let mut rows = vec![]; let mut k = 0usize;
        for tt in 0..NT {
            match s[tt] {
                Choice::Rest => k = 0,
                Choice::Opt(o) => { k = (k + 1).min(6); let op = &u.opts[tt][o as usize];
                    if op.race >= 0 { let r = op.race as usize;
                        rows.push(json!({"turn": tt, "when": when(tt), "year": tt / 24, "race": d.race_names[r], "p": pk(d, op.bp, k), "goal": op.goal,
                                         "parents": [won(a, r) > 0.0, won(b, r) > 0.0]})); } }
            }
        }
        (Value::Array(rows), wp)
    }

    /// One career's report: trainee and outfit, parents, the six legacy slots, per target odds, aptitudes, deck, schedule.
    fn report(&mut self, p: &Pool, f: &Found, with_schedule: bool) -> Value {
        let d = self.d; let ntar = self.ntar; let nu = self.nu;
        let name = |id: u32| d.umas.iter().find(|u| u.id == id).map_or(format!("Uma {id}"), |u| u.name.clone());
        let (t, a, b) = (f.t, f.a, f.b);
        let uid = d.umas[t].id;
        let sl = self.slots(p, t, a, b);
        let (q, nc) = inherit(&sl, ntar);
        self.set_trainee(t, &q);
        let (j, ext) = self.exact(t, &f.sel, &nc, true, false);
        let o = j.2;
        let mut pt = vec![0.0f64; ntar];
        for (pos, &i) in self.uncv.iter().enumerate() { pt[i] = ext.marg[o * nu + pos]; }
        for (pos, &i) in self.fixed_t.iter().enumerate() { pt[i] = GEN_GOLD * ext.sf[pos]; }
        let sj = self.sj;
        let (st, cards) = &self.model.opts[t][o];
        let covers: Vec<Value> = st.iter().zip(&self.uncv).filter(|(lv, i)| **lv > 0 && !(**lv == 1 && sj.targets[**i].guaranteed == 1))
            .map(|(lv, i)| json!({"group": sj.targets[*i].group, "level": lv})).collect();
        let default_st: Vec<u8> = self.uncv.iter().map(|&i| if sj.targets[i].guaranteed == 1 { 1 } else { 0 }).collect();
        let outfits = if *st == default_st { vec![] } else { cards.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>() };
        let slots: Vec<Value> = sl.iter().map(|s| json!({"kind": if s.parent { "parent" } else { "grandparent" }, "legacy": s.legacy, "gp": s.gp,
            "chara": s.chara, "name": name(s.chara), "rel": s.rel, "race": s.race, "aff": s.rel + s.race, "stars": s.stars, "pinks": s.pinks})).collect();
        let targets: Vec<Value> = (0..ntar).map(|i| json!({"group": sj.targets[i].group, "q": q[i], "carriers": nc[i], "p": pt[i],
            "gold": sj.targets[i].guaranteed == 2})).collect();
        let pd = self.space.pdeck(&f.sel, &[uid]);
        // pink sparks: per aptitude the chance at least one of the two inheritance rolls passes it on (1/3/5% per star)
        let mut pg: Vec<u32> = sl.iter().flat_map(|s| s.pinks.iter().map(|x| x.0)).collect(); pg.sort(); pg.dedup();
        let pinks: Vec<Value> = pg.iter().map(|&g| {
            let (mut miss, mut n) = (1.0f64, 0);
            for s in &sl { if let Some(st) = s.pinks.iter().filter(|x| x.0 == g).map(|x| x.1.min(3)).max() { if st > 0 { n += 1;
                miss *= 1.0 - ([0.0, 0.01, 0.03, 0.05][st as usize] * (1.0 + (s.rel + s.race) / 100.0)).min(1.0); } } }
            json!({"group": g, "q": 1.0 - miss * miss, "carriers": n})
        }).collect();
        let (sched, wp) = if with_schedule { self.schedule(p, t, a, b) } else { (Value::Null, vec![]) };
        json!({"trainee": {"id": uid, "name": d.umas[t].name, "outfits": outfits, "covers": covers},
               "parents": [a, b], "slots": slots, "targets": targets, "p_all": j.0, "p_minus1": j.1, "expected": pt.iter().sum::<f64>(),
               "bound": f.ub, "deck": pd.out(&self.unc), "pinks": pinks, "schedule": sched, "g1_wins": if with_schedule { json!(wp.iter().sum::<f64>()) } else { Value::Null }})
    }

    /// The reroll-or-move-on lookahead for career f (module notes).
    fn lookahead(&mut self, p: &Pool, legs: &[Legacy], f: &Found, p_now: f64) -> Value {
        let d = self.d; let ntar = self.ntar; let nu = self.nu;
        if ntar > 12 { return json!({"error": "the lookahead takes at most 12 targets"}); }
        let (t, a, b) = (f.t, f.a, f.b);
        // the chance of each set of targets landing (bit i = job target i): uncertain targets jointly, guaranteed golds apart
        let (q, nc) = inherit(&self.slots(p, t, a, b), ntar);
        self.set_trainee(t, &q);
        let (j, ext) = self.exact(t, &f.sel, &nc, false, true);
        let su = &ext.sets[(j.2 << nu)..((j.2 + 1) << nu)];
        let full = (1usize << ntar) - 1;
        let mut ps = vec![0.0f64; 1 << ntar];
        for (mu, &x) in su.iter().enumerate() {
            if x == 0.0 { continue; }
            let mut base = 0usize; for (k, &i) in self.uncv.iter().enumerate() { if mu >> k & 1 == 1 { base |= 1 << i; } }
            let mut part = vec![(base, x)];
            for (pos, &i) in self.fixed_t.iter().enumerate() { let g = GEN_GOLD * ext.sf[pos];
                part = part.into_iter().flat_map(|(m, w)| [(m, w * (1.0 - g)), (m | 1 << i, w * g)]).collect(); }
            for (m, w) in part { ps[m] += w; }
        }
        // the new veteran: this trainee, every target carried (stars from the rank mix), her schedule's win chances, the
        // career's parents as her parents
        let (_, wp) = self.schedule(p, t, a, b);
        let anc = |l: &Legacy| Ancestor { chara: l.chara, sparks: l.sparks.clone(), wins: l.wins.clone(), pinks: l.pinks.clone() };
        let v = Legacy { chara: d.umas[t].id, borrowed: false, sparks: self.sj.targets.iter().map(|x| (x.group, 2)).collect(), wins: vec![], pinks: vec![],
            parents: vec![anc(&legs[a]), anc(&legs[b])], win_p: wp, mix: Some(self.sj.star_dist) };
        let mut legs2 = legs.to_vec(); legs2.push(v); let vi = legs2.len() - 1;
        let p2 = self.pool(&legs2);
        let owned = self.owned.clone();
        let next = self.solve(&p2, &[vi], &owned, true, 4);
        // each setup scored for every landed set
        let mut fs = vec![p_now; 1 << ntar];
        let mut setups = vec![];
        for s in &next.found {
            let sl = self.slots(&p2, s.t, s.a, s.b);
            let vs: Vec<usize> = sl.iter().enumerate().filter(|(_, x)| x.parent && x.legacy == vi).map(|(i, _)| i).collect();
            let mut at_full = 0.0;
            for m in 0..=full {
                if ps[m] == 0.0 && m != full { continue; }
                let mut sl2 = sl.clone();
                for &i in &vs { for k in 0..ntar { sl2[i].stars[k] = if m >> k & 1 == 1 { 2 } else { 0 }; } }
                let (q2, nc2) = inherit(&sl2, ntar);
                self.set_trainee(s.t, &q2);
                let val = self.exact(s.t, &s.sel, &nc2, false, false).0 .0;
                if m == full { at_full = val; }
                if val > fs[m] { fs[m] = val; }
            }
            let partner = if s.a == vi { s.b } else { s.a };
            setups.push(json!({"trainee": {"id": d.umas[s.t].id, "name": d.umas[s.t].name}, "partner": partner, "p_all": at_full}));
        }
        // careers until a veteran lands every target: move on (1/f) or run this career again and keep the better veteran
        let inv = |x: f64| if x > 0.0 { 1.0 / x } else { f64::INFINITY };
        let pfull = ps[full];
        let reroll = |m: usize| 1.0 + (0..full).map(|m2| ps[m2] * inv(fs[m].max(fs[m2]))).sum::<f64>();
        let mut this = 1.0; let mut by: Vec<(f64, f64, f64)> = vec![(0.0, 0.0, 0.0); ntar];   // per landed count: (P, P x f, P where reroll)
        let mut improve = 0.0;
        let mut dec = vec![false; full];
        for m in 0..full {
            if ps[m] == 0.0 { continue; }
            let (mo, rr) = (inv(fs[m]), reroll(m));
            dec[m] = rr < mo;
            this += ps[m] * mo.min(rr);
            let c = (m as u32).count_ones() as usize;
            by[c].0 += ps[m]; by[c].1 += ps[m] * fs[m]; if dec[m] { by[c].2 += ps[m]; }
            if fs[m] > p_now * (1.0 + 1e-9) { improve += ps[m]; }
        }
        let by_count: Vec<Value> = (0..ntar).rev().filter(|&c| by[c].0 > 0.0).map(|c| json!({"landed": c, "p": by[c].0, "f": by[c].1 / by[c].0, "reroll": by[c].2 / by[c].0})).collect();
        let miss_one: Vec<Value> = (0..ntar).map(|k| { let m = full & !(1 << k);
            json!({"group": self.sj.targets[k].group, "p": ps[m], "f": fs[m], "reroll": dec[m], "move_on": inv(fs[m]), "careers_reroll": reroll(m)}) }).collect();
        json!({"p_now": p_now, "p_all": pfull, "careers_now": inv(p_now), "careers_this": this, "improve": improve, "setups": setups,
               "next_proven": next.proven, "by_count": by_count, "miss_one": miss_one, "star_dist": self.sj.star_dist})
    }
}

pub(crate) fn run_career(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    let sj: SkillJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    let dj: DeckJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    let cj: CareerJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    CHAIN_EVENTS.with(|c| c.set(dj.chain_events.unwrap_or((CHAIN_N, CHAIN_P))));
    let t_all = Instant::now();
    let model = Model::new(d, sk, &sj);
    let ew0 = model.ews.clone();
    let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
    let nu = unc.len();
    let ntar = sj.targets.len();
    let uncv = model.uncertain.clone();
    let fixed_t: Vec<usize> = (0..ntar).filter(|i| !uncv.contains(i)).collect();
    let n = d.umas.len();
    let card_of: HashMap<u32, &Card> = cr.cards.iter().map(|c| (c.id, c)).collect();
    let lock_blocked = |u: usize| dj.lock_cards.iter().any(|id| card_of.get(id).map_or(false, |c| c.blocks(d.umas[u].id)));
    let id_ok = |id: u32| (sj.owned.is_empty() || sj.owned.contains(&id)) && !sj.exclude.contains(&id);
    // every uma that can train (the lookahead's next career takes any of them); the job's trainees are among these
    let owned: Vec<usize> = (0..n).filter(|&i| id_ok(d.umas[i].id) && !model.no_outfit[i] && !lock_blocked(i)).collect();
    let trainees: Vec<usize> = owned.iter().copied().filter(|&i| cj.trainees.is_empty() || cj.trainees.contains(&d.umas[i].id)).collect();
    if trainees.is_empty() { return json!({"error": "No trainee to search: pick at least one owned uma."}); }
    let legs = &cj.legacies;
    if legs.len() < 2 { return json!({"error": "Add at least two veterans to choose parents from."}); }
    let pool_ids: Vec<u32> = trainees.iter().map(|&i| d.umas[i].id).collect();
    let ent = match deck_entries(cr, &dj, &unc, &pool_ids) { Ok(e) => e, Err(e) => return e };
    let space = deck_space(cr, &dj, sk, &unc, &ent);

    // ---- per uma: the best possible deck odds per target (a ceiling for every legal deck) ----
    let size = dj.deck_size;
    let strong: Vec<(Vec<f64>, Vec<f64>)> = (0..space.rel.len()).map(|j| {
        let (mut mw, mut mg) = (vec![1.0f64; nu], vec![1.0f64; nu]);
        src_miss(&space.srcs[j], &vec![1.0; space.kind[j].1 as usize], &mut mw, &mut mg);
        let mh: Vec<f64> = (0..nu).map(|k| mg[k] * mw[k]).collect();
        (mg, mh)
    }).collect();
    let dmax: Vec<Option<(Vec<f64>, Vec<f64>)>> = (0..n).map(|u| {
        if !owned.contains(&u) { return None; }
        let uid = d.umas[u].id;
        let legal: Vec<usize> = (0..space.rel.len()).filter(|&j| !space.cards[space.rel[j]].blocks(uid)).collect();
        let lowest = |f: &dyn Fn(usize) -> f64| { let mut m: Vec<f64> = legal.iter().map(|&j| f(j)).collect(); m.sort_by(|a, b| a.partial_cmp(b).unwrap()); m.iter().take(size).product::<f64>() };
        let (mut dw, mut dg) = (vec![0.0f64; nu], vec![0.0f64; nu]);
        for k in 0..nu {
            let (g, h) = (1.0 - lowest(&|j| strong[j].0[k]), 1.0 - lowest(&|j| strong[j].1[k]));
            // (g, h) = (P(gold hint), P(any hint)); the product formula takes dw with (1 - dw)(1 - dg) = 1 - h
            dg[k] = g; dw[k] = if g < 1.0 { (1.0 - (1.0 - h) / (1.0 - g)).max(0.0) } else { 1.0 };
        }
        Some((dw, dg))
    }).collect();
    let sts0: Vec<Vec<Vec<u8>>> = model.opts.iter().map(|m| m.iter().map(|x| x.0.clone()).collect()).collect();
    let budget = std::env::var("EXACT_BUDGET").ok().and_then(|s| s.parse().ok()).unwrap_or(EXACT_BUDGET);
    let mut env = Env { d, sj: &sj, model, ew0, uncv, fixed_t, ntar, nu, unc, space, dmax, sts0, owned, budget, work: 0 };
    let pool = env.pool(legs);
    let sv = env.solve(&pool, &cj.fix, &trainees, cj.per_trainee, cj.top);
    if sv.cands == 0 { return json!({"error": "No legal career: a trainee needs two veterans of other characters (not her own, not the same as each other), at most one of them borrowed."}); }
    // the race schedule only for the best career (the one the page shows; each takes a schedule search)
    let results: Vec<Value> = sv.found.iter().enumerate().map(|(i, f)| env.report(&pool, f, i == 0)).collect();
    let mut out = json!({"results": results, "candidates": sv.cands, "solved": sv.solved, "bounded": sv.bounded, "scored": sv.scored, "skipped": sv.skipped,
           "next_bound": sv.next_bound, "complete": sv.complete, "proven_top": sv.proven, "cap": EXACT_RHO});
    if cj.lookahead { if let Some(f) = sv.found.first() {
        // today's best career over everything (the floor of what the next career can do)
        let p_now = match cj.p_now { Some(x) => x, None => { let all = env.owned.clone(); env.solve(&pool, &[], &all, true, 1).found.first().map_or(0.0, |x| x.val) } };
        out["lookahead"] = env.lookahead(&pool, legs, f, p_now);
    } }
    out["timing_s"] = json!(t_all.elapsed().as_secs_f64());
    out
}
