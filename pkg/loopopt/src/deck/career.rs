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
//! Schedule: every G1 win counts one, plus one per parent who won that race (`best_response` weights), under the same
//! goals, race clashes and streak penalties as the loop scheduler; the report lists every G1 she could enter.
//!
//! Lookahead (`lookahead: true`, for the best career): the new veteran V lands each set S of targets with its exact
//! chance (`JointExt::sets`), each landed spark's stars from the rank mix (`star_dist`); her G1 wins are her schedule's
//! win chances, her parents the career's. f(S) = the best P(all) of a next career with V in the pool: the best
//! setups with V as a parent are found for V landing every target (the career search with V fixed as a parent), and
//! each S is scored on them (stars averaged: every landing factor is affine in q_k and the stars are independent, so
//! that average is exact for a fixed setup); f(S) is at least today's best P(all) (`p_now`). A lower bound: another
//! setup could do better for some S. The look ahead reports these facts and decides nothing: whether to use V or run
//! the career again is `policy`'s question, for one veteran, both choices counted the same way (one more career, then
//! the best setup it leaves, repeated). P(at least n-1) is tracked over the same setups, today's best-all career as
//! its floor.
use super::*;
mod rollout;
mod sim;

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
/// The work budget of one career search (units as in the per-uma search: 2 per deck node tested, 200 per exact
/// scoring, 10,000 per deck search). Measured need for a proven result (2026-10-07, synthetic pools of 12-400
/// veterans, default 8 targets): 10M-85M, the most at 200 veterans (12 s native); the loop planner's 30M
/// (EXACT_BUDGET) left pools of 12, 100 and 200 veterans unproven with the same picks. 300M leaves 3.5x headroom
/// and stops a runaway search after about a minute of native work. `EXACT_BUDGET` in the environment overrides it.
const CAREER_BUDGET: usize = 300_000_000;
#[derive(Deserialize)]
struct CareerPolicy {
    stay_trainee: u32,
    stay_parents: Vec<usize>,
    /// Best known all-target career already available in this roster, without the new draft.
    #[serde(default)] p_floor: f64,
}
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
    /// the next-step lookahead for the best career
    #[serde(default)] lookahead: bool,
    /// today's best P(all) over every career (the lookahead's floor); computed when absent
    #[serde(default)] p_now: Option<f64>,
    /// P(at least n-1) on today's best-all career, supplied with its all-target floor.
    #[serde(default)] p_minus1_now: Option<f64>,
    /// Compare rerolling a particular career with using the new parent, over an equal continuation horizon.
    #[serde(default)] policy: Option<CareerPolicy>,
    /// the planner's pick as a one-step rollout over this many best careers (0: off; `rollout` in the output)
    #[serde(default)] rollout: usize,
    /// the policy simulator (`sim`): runs instead of the search when present
    #[serde(default)] simulate: Option<Value>,
    /// the planner's pick on the surrogate simulator (`sim::plan`, `plan` in the output; with `policy`, `policy.plan`)
    #[serde(default)] plan: Option<Value>,
}

/// What a lineage counts of one legacy: relation index, G1 wins as (race, chance) sorted by race, stars per job target,
/// pink sparks, and the star mix when the stars are random.
#[derive(Clone)]
struct Who { ix: Option<usize>, chara: u32, wins: Vec<(usize, f64)>, stars: Vec<u8>, pinks: Vec<(u32, u8)>, mix: Option<[f64; 3]> }
/// The veterans the parents come from, each with her own parents.
#[derive(Clone)]
struct Pool { me: Vec<Who>, gp: Vec<Vec<Who>>, borrowed: Vec<bool> }
/// One legacy slot of a lineage: affinity parts and per-target stars.
#[derive(Clone)]
struct Slot { parent: bool, legacy: usize, gp: Option<usize>, chara: u32, rel: f64, race: f64, stars: Vec<u8>, pinks: Vec<(u32, u8)>, mix: Option<[f64; 3]> }
/// Whether legacy `x` is at least as good a parent as legacy `y` in every career (`Env::solve`, dominated veterans):
/// the same character, stars at least y's on every job target, G1 wins a superset of y's, and the same number of
/// parents, each (in order) the same character as y's, with stars and wins at least as good the same way. Only fixed
/// stars and certain wins (the lookahead's new veteran, with random stars and win chances, never compares).
/// Derived: with partner z and trainee t, swapping y for x keeps every relation term (same characters, grandparents
/// in the same slots) and raises or keeps every shared-win count (x with z, x with her parents; nothing else changes),
/// so every slot's affinity and stars are at least as high: each q_k is at least as high (1 - prod (1 - min(1,
/// 0.03 s (1 + A/100)))^2 rises with every s and A), each carrier count n_k too (so the spark scale C), and with them
/// the ceiling, the `rise` bounds and every deck's product and exact value (each landing factor rises with q_k). The
/// same sums and products in the same order, so this holds for the floats as well (correctly rounded operations are
/// monotone).
fn dominates(p: &Pool, x: usize, y: usize) -> bool {
    let ge = |a: &Who, b: &Who| a.chara == b.chara && a.mix.is_none() && b.mix.is_none() && a.stars.iter().zip(&b.stars).all(|(s, t)| s >= t)
        && a.wins.iter().chain(&b.wins).all(|w| w.1 == 1.0) && b.wins.iter().all(|w| a.wins.iter().any(|v| v.0 == w.0));
    ge(&p.me[x], &p.me[y]) && p.gp[x].len() == p.gp[y].len() && p.gp[x].iter().zip(&p.gp[y]).all(|(a, b)| ge(a, b))
}
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
/// Outcomes are ordered by target bitmask; the last entry is success and requires no continuation.
fn continuation_estimate(outcomes: &[f64], next: &[f64]) -> (Option<f64>, f64) {
    let full = outcomes.len() - 1;
    let (mut remaining, mut finite, mut two_runs) = (1.0f64, true, outcomes[full]);
    for m in 0..full {
        if outcomes[m] == 0.0 { continue; }
        two_runs += outcomes[m] * next[m];
        if next[m] <= 0.0 { finite = false; } else { remaining += outcomes[m] / next[m]; }
    }
    (if finite && remaining.is_finite() { Some(remaining) } else { None }, two_runs)
}
/// A job's setup fingerprint: everything but the pool and the places (targets, roster, cards, scenario, ...), the
/// key of what one job's search proves that holds for the next (p_bar, fully checked decks)
pub(super) fn setup_fp(v: &Value) -> u64 {
    let mut key = v.clone();
    if let Some(o) = key.as_object_mut() {
        for k in ["legacies", "trainees", "fix", "top", "per_trainee", "lookahead", "p_now", "p_minus1_now", "policy", "rollout", "cmd", "mode"] { o.remove(k); }
    }
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.to_string().hash(&mut h);
    h.finish()
}
thread_local! {
    /// per setup fingerprint (the last few), every fully checked deck found so far (`Env::seen`): what the page's
    /// earlier jobs proved bounds the next job's searches. A Ref holds a trainee's chances and her best exact value
    /// there, which depend on the setup only (the job's trainees change no deck value: they only order plain pals).
    static SEEN: std::cell::RefCell<Vec<(u64, Vec<Vec<Ref>>)>> = const { std::cell::RefCell::new(Vec::new()) };
}
/// Fully checked decks kept per trainee across jobs (the most recent): the bound loop over them stays cheap
const SEEN_KEEP: usize = 256;
/// Multilinear interpolation of corner values v (index bit k = coordinate k at 1) at the point x.
fn interp(v: &[f64], x: &[f64]) -> f64 { interp_in(v, x, &mut vec![]) }
/// `interp` with a buffer the caller keeps (the rollout evaluates tables millions of times)
/// Folds coordinate 0 first, halving the table each step: entry i after k folds is the full sweep's entry i << k (the
/// same multiply-adds on the same values, so the same bits), and only those reach entry 0.
fn interp_in(v: &[f64], x: &[f64], cur: &mut Vec<f64>) -> f64 {
    cur.clear(); cur.extend_from_slice(v);
    let mut n = cur.len();
    for &xk in x {
        n /= 2;
        for i in 0..n { cur[i] = cur[2 * i] * (1.0 - xk) + cur[2 * i + 1] * xk; }
    }
    cur[0]
}
/// A candidate the search solved: trainee, parents (legacy indices), value C x E, its ceiling, its best deck.
struct Found { t: usize, a: usize, b: usize, val: f64, ub: f64, sel: Sel }
/// A trainee's fully checked deck: the chances it was found at (job-target order), its exact value there before the
/// spark scale (the best of every legal deck, under the EXACT_RHO certificate), the deck. Its value bounds every deck
/// at other chances through `rise`.
#[derive(Clone)]
struct Ref { q: Vec<f64>, val: f64, sel: Sel }
enum DeckRes { NoDeck, Bounded, Done(f64, Sel, bool) }
struct Solved { found: Vec<Found>, refs: Vec<Vec<Ref>>, work: usize, cands: usize, solved: usize, bounded: usize, scored: usize, skipped: usize, next_bound: f64, complete: bool, proven: bool }

struct Env<'a> {
    d: &'a Data, sj: &'a SkillJob, model: Model<'a>, ew0: Vec<Vec<Vec<f64>>>, uncv: Vec<usize>, fixed_t: Vec<usize>, ntar: usize, nu: usize,
    unc: Vec<u32>, space: DeckSpace<'a>, dmax: Vec<Option<(Vec<f64>, Vec<f64>)>>, sts0: Vec<Vec<Vec<u8>>>, owned: Vec<usize>, budget: usize, work: usize,
    /// every earlier search's fully checked decks per trainee (same model, so each is a proven best at its chances):
    /// later searches bound with them (`rise`), but don't start from them, so ties resolve as in a lone search
    seen: Vec<Vec<Ref>>,
    /// whether `solve` bounds with `seen`: off for the searches whose fully checked decks feed the rollout's cache
    /// (its floors f(S) come from those decks, so they must not depend on what earlier searches proved)
    use_seen: bool,
    /// per trainee, what her product searches proved (`best_deck` pass 1): (chances q, S) with every legal deck's product
    /// value at q (fixed factor included, before the spark scale) at most S. Bounds her other candidates (`solve`).
    ruled: Vec<Vec<(Vec<f64>, f64)>>,
    /// what the last `best_deck` call's product search proved: Some(S) as in `ruled`, None when it proved nothing
    last_pv: Option<f64>,
    /// a hard work cap for `solve` (0: none): past it the search stops taking candidates (`plan`'s extra searches)
    hard: usize,
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
        let mut ext = JointExt { su, sf, marg: vec![], sets: if sets { vec![0.0] } else { vec![] }, per_outfit: vec![] };
        let j = joint_fast(&self.model, t, &[0.0], &self.ents_of(sel), sel.filler, self.ntar, m1, Some(&mut ext))[0];
        (j, ext)
    }

    /// How far trainee t's best exact value (before the spark scale) can rise from chances q0 to q. A target the
    /// outfit lacks lands with GG g + GW (1 - g - n (1 - e)(1 - q_k)) per chain state (g: gold hinted, n: no hint,
    /// e: her own event hint chance); from q0 to q it grows at most by (1 - (1 - e)(1 - q_k)) / (1 - (1 - e)(1 - q0_k))
    /// (n = 1, g = 0), and not at all when q_k <= q0_k or the outfit has the skill. P(all) is a sum of products of
    /// these factors, so it grows at most by the product, for every deck: E(q) <= E(q0) x max over outfits of it.
    fn rise(&self, t: usize, q: &[f64], q0: &[f64]) -> f64 {
        self.sts0[t].iter().zip(&self.ew0[t]).map(|(st, ew)| self.uncv.iter().enumerate().map(|(k, &i)| {
            if st[k] > 0 || q[i] <= q0[i] { 1.0 } else { (1.0 - (1.0 - ew[k]) * (1.0 - q[i])) / (1.0 - (1.0 - ew[k]) * (1.0 - q0[i])) }
        }).product::<f64>()).fold(0.0f64, f64::max)
    }
    /// The inheritance chances, carriers and spark scale C of trainee t with parents a and b.
    fn lineage(&self, p: &Pool, t: usize, a: usize, b: usize) -> (Vec<f64>, Vec<i32>, f64) {
        let (q, nc) = inherit(&self.slots(p, t, a, b), self.ntar);
        let (su, sf) = self.scale(&nc);
        let c = su.iter().chain(&sf).product::<f64>();
        (q, nc, c)
    }
    /// The ceiling of trainee t at chances q (her inheritance folded in here) with spark scale c: EXACT_RHO x C x the
    /// product value at each target's best possible deck odds (None: no legal deck odds for her).
    fn ceiling(&mut self, t: usize, q: &[f64], c: f64) -> Option<f64> {
        let (dw, dg) = self.dmax[t].clone()?;
        self.set_trainee(t, q);
        let pv = (0..self.model.n_outfits(t)).map(|o| self.model.outfit_fast(t, o, 0.0, &dw, &dg)).fold(0.0f64, f64::max);
        Some(EXACT_RHO * c * self.model.fixed_factor() * pv)
    }
    /// Trainee t's best deck at the chances already folded in (`set_trainee`), as the per-uma search finds it: the best
    /// product deck, then every deck whose product value times EXACT_RHO reaches the best exact value, scored exactly
    /// (`upper_na` filters). `c` the spark scale, `th` the value (C x E) the career must beat: below it the search stops
    /// after the product deck. `starts`: decks scored first (a higher start leaves fewer decks to check). Counts the
    /// exact scorings in `scored`, its work in `self.work`; `full` is false when the work budget ran out.
    fn best_deck(&mut self, t: usize, c: f64, th: f64, starts: &[&Sel], scored: &mut usize) -> DeckRes {
        let ntar = self.ntar; let uid = self.d.umas[t].id;
        let n0 = self.space.nodes.get();
        // the product search starts at the bar the career must clear (a hair below the cut below): it prunes every
        // branch that cannot reach it and finds nothing when no deck can, and a deck that clears it is the one an
        // unseeded search finds (the seed is below the maximum, so the first deck reaching it is never pruned)
        let seed = if th.is_finite() && th > 0.0 { th / (EXACT_RHO * c * (1.0 + 1e-9)) * (1.0 - 1e-12) } else { f64::NEG_INFINITY };
        let (r, over) = self.space.search(&self.model, &[t], &[uid], &[(vec![(0, 0.0)], 0.0)], true, seed, 1.0, 1);
        self.work += 2 * (self.space.nodes.get() - n0) + 10_000;
        // what it proves (`ruled`): the deck found is the best (the first deck above the seed the search keeps is only
        // ever replaced by a better one, and a branch is pruned only below the best so far); none found, every deck is
        // at most the seed. Nothing when the search gave up (`over`, more than 128 card characters).
        self.last_pv = if over { None } else if let Some(x) = r.first() { Some(x.0) } else if seed > f64::NEG_INFINITY { Some(seed) } else { None };
        let Some((pv, sel)) = r.into_iter().next() else { return if seed > f64::NEG_INFINITY { DeckRes::Bounded } else { DeckRes::NoDeck } };
        // the candidate's own ceiling from its best product deck
        if EXACT_RHO * c * pv * (1.0 + 1e-9) < th { return DeckRes::Bounded; }
        let score = |env: &Env, sel: &Sel| joint(&env.model, t, &[0.0], &env.ents_of(sel), sel.filler, ntar, false)[0].0;
        let mut best = (score(self, &sel), sel); *scored += 1; self.work += 200;
        for s in starts { let x = score(self, s); *scored += 1; self.work += 200; if x > best.0 { best = (x, (*s).clone()); } }
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
                let x = score(self, &s); *scored += 1; self.work += 200;
                if x > best.0 { best = (x, s); }
            } }
        }
        DeckRes::Done(best.0, best.1, full)
    }

    /// The best careers: trainees from `trainees` (uma indices), two legacies of `p` as parents (all of `fix` among
    /// them, at most one borrowed), each with her best deck; `top` of them, one per trainee when `per_trainee`.
    /// Candidates are kept as (ceiling, trainee, parents) only (a pool of 400 veterans has millions); their chances
    /// are recomputed when one is solved (the same numbers, so the same results).
    fn solve(&mut self, p: &Pool, fix: &[usize], trainees: &[usize], per_trainee: bool, top: usize) -> Solved {
        let d = self.d;
        self.work = 0;   // each search has its own work budget
        struct Cand { ub: f64, t: u16, a: u16, b: u16 }
        let mut cands: Vec<Cand> = vec![];
        let nl = p.me.len();
        assert!(nl <= u16::MAX as usize && d.umas.len() <= u16::MAX as usize);
        for &t in trainees {
            let tid = d.umas[t].id;
            if self.dmax[t].is_none() { continue; }
            for a in 0..nl { for b in a + 1..nl {
                let (ca, cb) = (p.me[a].chara, p.me[b].chara);
                if ca == cb || ca == tid || cb == tid || (p.borrowed[a] && p.borrowed[b]) || !fix.iter().all(|&f| f == a || f == b) { continue; }
                let (q, _, c) = self.lineage(p, t, a, b);
                let ub = self.ceiling(t, &q, c).unwrap();
                cands.push(Cand { ub, t: t as u16, a: a as u16, b: b as u16 });
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
        let thr = |done: &[(f64, usize, Sel)], cands: &[Cand], t: usize| if !per_trainee { kth(done) } else { done.iter().find(|x| cands[x.1].t as usize == t).map_or(kth(done), |x| x.0.max(kth(done))) };
        // per trainee, her fully checked candidates (`Ref`: chances, exact best value before the spark scale, deck)
        let mut refs: Vec<Vec<Ref>> = vec![vec![]; d.umas.len()];
        // per trainee, where this search's own entries in `ruled` start
        let ruled0: Vec<usize> = self.ruled.iter().map(|r| r.len()).collect();
        // dominated veterans (`dominates`): per legacy y, the legacies x at least as good in every career; and the
        // candidates this search took and did not solve (bounded, no deck)
        let dom: Vec<Vec<usize>> = (0..nl).map(|y| (0..nl).filter(|&x| x != y && dominates(p, x, y)).collect()).collect();
        let prune = dom.iter().any(|v| !v.is_empty());
        let mut unsolved: crate::FxSet<(u16, u16, u16)> = Default::default();
        while next < cands.len() && cands[next].ub * (1.0 + 1e-9) >= kth(&done) {
            if self.hard > 0 && self.work > self.hard { complete = false; break; }
            let ci = next; next += 1;
            let (t, a, b) = (cands[ci].t as usize, cands[ci].a as usize, cands[ci].b as usize);
            let th = thr(&done, &cands, t);
            if prune { unsolved.insert((cands[ci].t, cands[ci].a, cands[ci].b)); }
            if cands[ci].ub * (1.0 + 1e-9) < th { n_bounded += 1; continue; }
            // A candidate whose parent has a dominating veteran x, where (t, x, other parent) was taken earlier and not
            // solved, would end unsolved too: that one was ruled out by its ceiling, a Ref's or a product search's bound,
            // a search that found no deck clearing the bar, or (in turn) this test, at a bar no higher than now (the bar
            // only rises), and each of those bounds is at most as high for this candidate (`dominates`: q, C and every
            // deck's value no higher), so it fails the same test (no deck at all: none here either). So no Ref, no
            // result, the same counters bar `work`; exact for any `top` and either `per_trainee`, and it uses nothing but
            // this search's own candidates. A taken-and-solved dominator proves nothing of the kind (this one could still
            // come out solved, a Ref the later searches start from), so only unsolved ones count.
            if prune {
                let key = |x: usize, y: usize| (t as u16, x.min(y) as u16, x.max(y) as u16);
                if [(a, b), (b, a)].iter().any(|&(x, y)| dom[x].iter().any(|&v| v != y && unsolved.contains(&key(v, y)))) { n_bounded += 1; continue; }
            }
            let (q, _, c) = self.lineage(p, t, a, b);
            let seen: &[Ref] = if self.use_seen { &self.seen[t] } else { &[] };
            let rb = refs[t].iter().chain(seen).map(|r| c * r.val * self.rise(t, &q, &r.q)).fold(f64::INFINITY, f64::min);
            if rb * (1.0 + 1e-9) < th { n_bounded += 1; continue; }
            // what her earlier product searches proved (`ruled`). Derived: with her chances folded in, target k's factor
            // in the product value of one outfit and one deck (odds dw, dg) is f(n) = A - B n, n = (1 - e)(1 - q_k),
            // A = dg G + (1 - dg) W, B = (1 - dg) W (1 - dw) <= A (`outfit_fast` at q = 0; a target the outfit has is
            // q-free). From q to q' with q'_k > q_k: n' < n, and A >= B gives f(n') / f(n) <= (1 - n') / (1 - n), the
            // factor `rise` takes ((A - B n')(1 - n) <= (A - B n)(1 - n') <=> A (n' - n) <= B (n' - n)); with q'_k <= q_k
            // the factor does not grow. So per outfit the product at q' is at most the product at q times that outfit's
            // ratio product, the best outfit at q' at most S x rise(t, q', q) for every deck, and the fixed gold factor
            // and the spark scale c' multiply both sides alike. Then the search below would end `Bounded` on either path:
            // no deck above its seed th / (EXACT_RHO c' (1 + 1e-9)), or its best deck pv' with EXACT_RHO c' pv' (1 + 1e-9)
            // < th. The slack 2e-9 covers the float rounding of pv' and rise (about 1e-15 each). A skipped candidate is
            // one that would have been `Bounded`: no Ref, no result, the same counters bar `work` (less of it, so a search
            // that ran out of its budget before can now get further; none of the checked jobs comes near it). A search whose
            // decks feed the rollout (`use_seen` off) takes only the proofs of its own candidates, so its work does not
            // depend on what earlier searches did either.
            let lo = if self.use_seen { 0 } else { ruled0[t] };
            if self.ruled[t][lo..].iter().any(|(rq, s)| EXACT_RHO * c * s * self.rise(t, &q, rq) * (1.0 + 2e-9) < th) { n_bounded += 1; continue; }
            self.set_trainee(t, &q);
            let starts: Vec<&Sel> = refs[t].iter().map(|r| &r.sel).collect();
            let res = self.best_deck(t, c, th, &starts, &mut n_scored);
            if let Some(s) = self.last_pv.take() { self.ruled[t].push((q.clone(), s)); }
            let (val, sel, full) = match res {
                DeckRes::NoDeck => continue,
                DeckRes::Bounded => { n_bounded += 1; continue }
                DeckRes::Done(v, sel, full) => (v, sel, full),
            };
            n_solved += 1;
            if prune { unsolved.remove(&(cands[ci].t, cands[ci].a, cands[ci].b)); }
            if full { refs[t].push(Ref { q, val, sel: sel.clone() }); } else { complete = false; }
            let val = c * val;
            if per_trainee { if let Some(i) = done.iter().position(|x| cands[x.1].t as usize == t) { if done[i].0 >= val { continue; } done.remove(i); } }
            let pos = done.iter().position(|x| x.0 < val).unwrap_or(done.len());
            done.insert(pos, (val, ci, sel));
        }
        for (t, rs) in refs.iter().enumerate() { for r in rs { if !self.seen[t].iter().any(|x| x.val == r.val && x.q == r.q) { self.seen[t].push(r.clone()); } } }
        let next_bound = if next < cands.len() { cands[next].ub } else { 0.0 };
        let proven = complete && !done.is_empty() && (done.len() >= top.min(cands.len()) || next >= cands.len()) && next_bound < done[0].0;
        let n = cands.len();
        Solved { found: done.into_iter().take(top).map(|(val, ci, sel)| Found { t: cands[ci].t as usize, a: cands[ci].a as usize, b: cands[ci].b as usize, val, ub: cands[ci].ub, sel }).collect(),
                 refs, cands: n, solved: n_solved, bounded: n_bounded, scored: n_scored, skipped: n - next, next_bound, complete, proven, work: self.work }
    }

    /// Trainee t's race schedule with parents a and b: every G1 win counts one (a G1 on her record can be shared with a
    /// later partner), plus one per parent who won that race (a win she shares with a parent adds 3 affinity to each of
    /// them when she is a parent herself), so she runs the G1s that win the most and, where two clash, the shared one.
    /// Rows: every G1 she could enter, in turn order; `run` false for those she skips, with what she does that turn.
    /// Also her win chance per race index.
    fn schedule(&self, p: &Pool, t: usize, a: usize, b: usize) -> (Value, Vec<f64>) {
        let d = self.d; let u = &d.umas[t];
        let mut wr = vec![1.0f64; d.nr];
        for x in [a, b] { for &(r, pr) in &p.me[x].wins { if r < d.nr { wr[r] += pr; } } }
        let Some((s, _)) = best_response(d, u, &wr, None, None) else { return (json!([]), vec![0.0; d.nr]) };
        let wp = probs(d, u, &s);
        let won = |x: usize, r: usize| p.me[x].wins.iter().find(|w| w.0 == r).map_or(0.0, |w| w.1);
        let mut rows = vec![]; let mut k = 0usize;
        for tt in 0..NT {
            let chosen = match s[tt] { Choice::Rest => { k = 0; None } Choice::Opt(o) => { k = (k + 1).min(6); Some(o as usize) } };
            for (o, op) in u.opts[tt].iter().enumerate() {
                if op.race < 0 { continue; }
                let r = op.race as usize; let run = chosen == Some(o);
                let instead = match chosen { _ if run => Value::Null, None => json!("rest"), Some(c) => json!(u.opts[tt][c].name) };
                // the loop planner's chart row (one member: `cells`, `shared` = a parent won it, so it builds affinity),
                // plus `run`, `p`, `instead`, and `name` (the race's full name)
                let par = [won(a, r) > 0.0, won(b, r) > 0.0];
                let cell = if run { json!({"p": (pk(d, op.bp, k) * 100.0).round(), "goal": op.goal, "apt": 110 - op.bp, "row": k, "pen": d.pen.get(k.max(1) - 1).copied().unwrap_or(0)}) } else { Value::Null };
                rows.push(json!({"turn": tt, "when": when(tt), "year": tt / 24, "race": chart_race_name(&d.race_names[r]), "name": d.race_names[r],
                                 "run": run, "skipped": !run, "instead": instead, "p": if run { json!(pk(d, op.bp, k)) } else { Value::Null }, "goal": op.goal,
                                 "parents": par, "shared": par[0] || par[1], "cells": [cell], "runners": run as u8, "alts": []}));
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
        let (j, ext) = self.exact(t, &f.sel, &nc, true, with_schedule && ntar <= 12);
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
        let outcomes = if with_schedule && ntar <= 12 { json!(self.outcome_sets(j.2, &ext)) } else { Value::Null };
        let (sched, wp) = if with_schedule { self.schedule(p, t, a, b) } else { (Value::Null, vec![]) };
        json!({"trainee": {"id": uid, "name": d.umas[t].name, "outfits": outfits, "covers": covers},
               "parents": [a, b], "slots": slots, "targets": targets, "p_all": j.0, "p_minus1": j.1, "expected": pt.iter().sum::<f64>(),
               "bound": f.ub, "deck": pd.out(&self.unc), "pinks": pinks, "schedule": sched, "outcome_sets": outcomes, "g1_wins": if with_schedule { json!(wp.iter().sum::<f64>()) } else { Value::Null }})
    }

    /// Corner table of trainee t with deck `sel`: per outfit, P(all) with every spark-generation scale 1 at the 2^nu
    /// points q_k = 1 - (1 - q0_k) x_k, x_k in {0, 1} (index bit k = uncertain target k at x_k = 1). Derived: q_k enters
    /// every landing factor only through (1 - e)(1 - q_k), affinely, and P(all) per outfit is a sum of products with
    /// one factor per target, so it is multilinear in x and `interp` at any x in [0, 1]^nu gives it exactly.
    fn corners(&mut self, t: usize, sel: &Sel, q0: &[f64]) -> Vec<Vec<f64>> {
        let (nu, ntar) = (self.nu, self.ntar);
        let no = self.model.n_outfits(t);
        let mut corners = vec![vec![0.0f64; 1 << nu]; no];
        for c in 0..(1usize << nu) {
            let mut q = q0.to_vec();
            for (k, &i) in self.uncv.iter().enumerate() { let x = if c >> k & 1 == 1 { 1.0 } else { 0.0 }; q[i] = 1.0 - (1.0 - q0[i]) * x; }
            self.set_trainee(t, &q);
            let (_, ext) = self.exact(t, sel, &vec![6; ntar], false, false);
            for o in 0..no { corners[o][c] = ext.per_outfit[o]; }
        }
        corners
    }

    /// Joint distribution of full target sets, including guaranteed gold generation.
    fn outcome_sets(&self, outfit: usize, ext: &JointExt) -> Vec<f64> {
        let su = &ext.sets[(outfit << self.nu)..((outfit + 1) << self.nu)];
        let mut ps = vec![0.0f64; 1 << self.ntar];
        for (mu, &x) in su.iter().enumerate() {
            if x == 0.0 { continue; }
            let mut base = 0usize; for (k, &i) in self.uncv.iter().enumerate() { if mu >> k & 1 == 1 { base |= 1 << i; } }
            let mut part = vec![(base, x)];
            for (pos, &i) in self.fixed_t.iter().enumerate() { let g = GEN_GOLD * ext.sf[pos];
                part = part.into_iter().flat_map(|(m, w)| [(m, w * (1.0 - g)), (m | 1 << i, w * g)]).collect(); }
            for (m, w) in part { ps[m] += w; }
        }
        ps
    }

    /// The veteran career (t, a, b) leaves, before her result is known: trainee t, a spark of every target with stars
    /// drawn from the rank mix (`star_dist`), her schedule's win chances `wp`, parents a and b.
    fn veteran(&self, legs: &[Legacy], t: usize, a: usize, b: usize, wp: Vec<f64>) -> Legacy {
        let anc = |l: &Legacy| Ancestor { chara: l.chara, sparks: l.sparks.clone(), wins: l.wins.clone(), pinks: l.pinks.clone() };
        Legacy { chara: self.d.umas[t].id, borrowed: false, sparks: self.sj.targets.iter().map(|x| (x.group, 2)).collect(), wins: vec![], pinks: vec![],
            parents: vec![anc(&legs[a]), anc(&legs[b])], win_p: wp, mix: Some(self.sj.star_dist) }
    }
    /// The next-step lookahead for career f (module notes).
    fn lookahead(&mut self, p: &Pool, legs: &[Legacy], f: &Found, p_now: f64, p_minus1_now: f64, policy: bool) -> Value {
        let d = self.d; let ntar = self.ntar;
        if ntar > 12 { return json!({"error": "the lookahead takes at most 12 targets"}); }
        let (t, a, b) = (f.t, f.a, f.b);
        // the chance of each set of targets landing (bit i = job target i): uncertain targets jointly, guaranteed golds apart
        let (q, nc) = inherit(&self.slots(p, t, a, b), ntar);
        self.set_trainee(t, &q);
        let (j, ext) = self.exact(t, &f.sel, &nc, false, true);
        let full = (1usize << ntar) - 1;
        let ps = self.outcome_sets(j.2, &ext);
        // the new veteran: this trainee, every target carried (stars from the rank mix), her schedule's win chances, the
        // career's parents as her parents
        let (_, wp) = self.schedule(p, t, a, b);
        let mut legs2 = legs.to_vec(); legs2.push(self.veteran(legs, t, a, b, wp)); let vi = legs2.len() - 1;
        let p2 = self.pool(&legs2);
        let owned = self.owned.clone();
        let next = self.solve(&p2, &[vi], &owned, true, 4);
        // each setup scored for every landed set
        let mut fs = vec![p_now; 1 << ntar];
        let mut near = vec![p_minus1_now; 1 << ntar];
        let mut near_all = vec![p_now; 1 << ntar];
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
                let score = self.exact(s.t, &s.sel, &nc2, !policy, false).0;
                let val = score.0;
                if score.1 > near[m] { near[m] = score.1; near_all[m] = val; }
                if m == full { at_full = val; }
                if val > fs[m] { fs[m] = val; }
            }
            let partner = if s.a == vi { s.b } else { s.a };
            setups.push(json!({"trainee": {"id": d.umas[s.t].id, "name": d.umas[s.t].name}, "partner": partner, "p_all": at_full}));
        }
        // the facts per landed set, the old roster still available
        let pfull = ps[full];
        if policy {
            // One run now; on failure, repeat the best checked continuation without further upgrades.
            // Both actions use this same terminal policy. This is a horizon estimate, not an optimal Bellman value.
            let (remaining, two_runs) = continuation_estimate(&ps, &fs);
            return json!({"expected_careers_estimate": remaining,
                "p_all": pfull, "p_within_two": two_runs, "next_proven": next.proven, "setups": setups.len(),
                "star_dist": self.sj.star_dist});
        }
        // The improvement odds over her stars too (not only stars averaged): for a fixed setup, P(all) per outfit is
        // multilinear in x_k = m(s_k)^2, m(s) = 1 - min(1, 0.03 s (1 + A_V/100)) her miss factor for target k at s stars
        // (each landing factor is affine in (1 - q_k) = (the other legacies' miss)^2 x_k), times the spark-generation
        // scale 1.1^(n_k - 6). So P(all) at the 2^nu corners x in {0, 1} (uncertain targets) gives it for every set
        // she lands and every star count: exact for these setups, over 4^n outcomes (n <= 8 targets).
        let stars_exact = ntar <= 8;
        let (mut improve_x, mut ef, mut ef_imp, mut fmax) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let mut by_x = vec![(0.0f64, 0.0f64); ntar + 1];   // per number landed: (P, P where she improves)
        let mut check = 0.0f64;
        if stars_exact && !policy {
            let w = self.sj.star_dist;
            // per setup: per outfit the corner values (scale 1), the new veteran's affinity, the carriers without her
            struct Lin { corners: Vec<Vec<f64>>, xm: [f64; 4], pw: Vec<[f64; 2]> }
            let mut lins = vec![];
            for s in &next.found {
                let sl = self.slots(&p2, s.t, s.a, s.b);
                let iv = sl.iter().position(|x| x.parent && x.legacy == vi).unwrap();
                let a_v = sl[iv].rel + sl[iv].race;
                let mut sl0 = sl.clone(); for k in 0..ntar { sl0[iv].stars[k] = 0; } sl0[iv].mix = None;
                let (q0, nc0) = inherit(&sl0, ntar);
                let corners = self.corners(s.t, &s.sel, &q0);
                // the check: every target at 2 stars, against a direct evaluation
                let mut sl2 = sl0.clone(); for k in 0..ntar { sl2[iv].stars[k] = 2; }
                let (q2, nc2) = inherit(&sl2, ntar);
                self.set_trainee(s.t, &q2);
                let direct = self.exact(s.t, &s.sel, &nc2, false, false).0 .0;
                let xm = |st: u8| { let m = 1.0 - (0.03 * st as f64 * (1.0 + a_v / 100.0)).min(1.0); m * m };
                let xs: Vec<f64> = self.uncv.iter().map(|_| xm(2)).collect();
                let sc: f64 = (0..ntar).map(|k| 1.1f64.powi(nc0[k] + 1 - 6)).product();
                let via = sc * corners.iter().map(|cv| interp(cv, &xs)).fold(0.0f64, f64::max);
                check = check.max((via - direct).abs() / direct.max(1e-300));
                // per star count her x factor, per target the spark scale without and with her spark (the loop's terms)
                let xm4 = [1.0, xm(1), xm(2), xm(3)];
                let pw: Vec<[f64; 2]> = (0..ntar).map(|k| [1.1f64.powi(nc0[k] - 6), 1.1f64.powi(nc0[k] + 1 - 6)]).collect();
                lins.push(Lin { corners, xm: xm4, pw });
            }
            // every outcome: per target absent or 1-3 stars
            let mut sig = vec![0u8; ntar];
            let (mut xs, mut cur) = (vec![0.0f64; self.uncv.len()], vec![]);
            loop {
                let mut m = 0usize; let mut pr = 1.0f64;
                for k in 0..ntar { if sig[k] > 0 { m |= 1 << k; pr *= w[sig[k] as usize - 1]; } }
                pr *= ps[m];
                if pr > 0.0 {
                    let mut f = p_now;
                    for l in &lins {
                        for (x, &i) in xs.iter_mut().zip(&self.uncv) { *x = l.xm[sig[i] as usize]; }
                        let sc: f64 = (0..ntar).map(|k| l.pw[k][(sig[k] > 0) as usize]).product();
                        let v = sc * l.corners.iter().map(|cv| interp_in(cv, &xs, &mut cur)).fold(0.0f64, f64::max);
                        if v > f { f = v; }
                        if v > fmax { fmax = v; }
                    }
                    let up = f > p_now * (1.0 + 1e-9);
                    ef += pr * f; if up { improve_x += pr; ef_imp += pr * f; }
                    let c = m.count_ones() as usize; by_x[c].0 += pr; if up { by_x[c].1 += pr; }
                }
                let mut k = 0;
                while k < ntar { sig[k] += 1; if sig[k] <= 3 { break; } sig[k] = 0; k += 1; }
                if k == ntar { break; }
            }
        }
        // (probability, weighted all, weighted near, no-gain mass, near-only improvement mass)
        let mut by = vec![(0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64); ntar];
        let mut improve = 0.0;
        let mut partial_improve = 0.0;
        let mut stay = vec![true; full];
        for m in 0..full {
            if ps[m] == 0.0 { continue; }
            stay[m] = fs[m] <= p_now * (1.0 + 1e-9);
            let near_only = stay[m] && near[m] > p_minus1_now * (1.0 + 1e-9);
            let c = m.count_ones() as usize;
            by[c].0 += ps[m]; by[c].1 += ps[m] * fs[m]; by[c].2 += ps[m] * near[m];
            if stay[m] { by[c].3 += ps[m]; } else { improve += ps[m]; }
            if near_only { by[c].4 += ps[m]; partial_improve += ps[m]; }
        }
        let by_count: Vec<Value> = (0..ntar).rev().filter(|&c| by[c].0 > 0.0).map(|c| json!({
            "landed": c, "p": by[c].0, "f": by[c].1 / by[c].0, "f_minus1": by[c].2 / by[c].0,
            "no_gain": by[c].3 / by[c].0, "partial_only": by[c].4 / by[c].0})).collect();
        let miss_one: Vec<Value> = (0..ntar).map(|k| { let m = full & !(1 << k);
            json!({"group": self.sj.targets[k].group, "p": ps[m], "f": fs[m], "f_minus1": near[m],
                "near_p_all": near_all[m], "no_gain": stay[m], "partial_only": stay[m] && near[m] > p_minus1_now * (1.0 + 1e-9)}) }).collect();
        json!({"p_now": p_now, "p_minus1_now": p_minus1_now, "p_all": pfull, "improve": improve,
               "partial_improve": partial_improve, "setups": setups, "next_proven": next.proven,
               "stars_exact": stars_exact, "improve_stars": if stars_exact { json!(improve_x) } else { Value::Null },
               "f_mean": if stars_exact { json!(ef) } else { Value::Null }, "f_if_improve": if stars_exact && improve_x > 0.0 { json!(ef_imp / improve_x) } else { Value::Null },
               "improve_by_count": if stars_exact { json!((0..=ntar).rev().filter(|&c| by_x[c].0 > 0.0).map(|c| json!({"landed": c, "p": by_x[c].0, "improve": by_x[c].1 / by_x[c].0})).collect::<Vec<_>>()) } else { Value::Null },
               "multilinear_check": check, "best_case_with_her": if stars_exact { json!(fmax) } else { Value::Null },
               "best_with_her": next.found.first().map(|f| json!({"trainee": d.umas[f.t].id, "p_all": f.val})),
               "by_count": by_count, "miss_one": miss_one, "star_dist": self.sj.star_dist})
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
    let budget = std::env::var("EXACT_BUDGET").ok().and_then(|s| s.parse().ok()).unwrap_or(CAREER_BUDGET);
    let mut env = Env { d, sj: &sj, model, ew0, uncv, fixed_t, ntar, nu, unc, space, dmax, sts0, owned, budget, work: 0, seen: vec![vec![]; n], use_seen: true, ruled: vec![vec![]; n], last_pv: None, hard: 0 };
    // what earlier jobs on this setup proved (the page's worker keeps it between jobs)
    let fp = setup_fp(v);
    if let Some(s) = SEEN.with(|c| c.borrow().iter().find(|e| e.0 == fp).map(|e| e.1.clone())) { if s.len() == n { env.seen = s; } }
    let keep = |env: &Env| SEEN.with(|c| {
        let mut c = c.borrow_mut(); c.retain(|e| e.0 != fp);
        c.push((fp, env.seen.iter().map(|rs| rs[rs.len().saturating_sub(SEEN_KEEP)..].to_vec()).collect()));
        if c.len() > 4 { c.remove(0); }
    });
    if let Some(sm) = &cj.simulate { return sim::simulate(&mut env, d, sk, cr, v, legs, &trainees, sm); }
    let pj: Option<sim::plan::PlanJob> = match &cj.plan { None => None, Some(x) => match serde_json::from_value(x.clone()) { Ok(p) => Some(p), Err(e) => return json!({"error": format!("plan: {e}")}) } };
    let pool = env.pool(legs);
    // the rollout's cache takes the main search's decks when it uses its candidates (see `use_seen`)
    env.use_seen = cj.rollout == 0;
    let sv = env.solve(&pool, &cj.fix, &trainees, cj.per_trainee, cj.top);
    env.use_seen = true;
    if std::env::var("CAREER_WORK").is_ok() { eprintln!("career search: work {} of {}, solved {}, proven {}", sv.work, env.budget, sv.solved, sv.proven); }
    if sv.cands == 0 { keep(&env); return json!({"error": "No legal career: a trainee needs two veterans of other characters (not her own, not the same as each other), at most one of them borrowed."}); }
    // the race schedule only for the best career (the one the page shows; each takes a schedule search)
    let results: Vec<Value> = sv.found.iter().enumerate().map(|(i, f)| env.report(&pool, f, i == 0)).collect();
    let mut out = json!({"results": results, "candidates": sv.cands, "solved": sv.solved, "bounded": sv.bounded, "scored": sv.scored, "skipped": sv.skipped,
           "next_bound": sv.next_bound, "complete": sv.complete, "proven_top": sv.proven, "cap": EXACT_RHO});
    if let Some(policy) = &cj.policy {
        let stay_t = env.uix(policy.stay_trainee).filter(|t| env.owned.contains(t));
        if ntar > 12 {
            out["policy"] = json!({"error": "The remaining-careers estimate supports at most 12 targets."});
        } else if stay_t.is_none() || policy.stay_parents.len() != 2 || policy.stay_parents.iter().any(|&i| i >= legs.len()) {
            out["policy"] = json!({"error": "The reroll career needs an eligible trainee and two existing parents."});
        } else if let Some(advance) = sv.found.first() {
            let stay = env.solve(&pool, &policy.stay_parents, &[stay_t.unwrap()], false, 1);
            if let (Some(pj), Some(stay_f)) = (&pj, stay.found.first()) {
                let sf = Found { t: stay_f.t, a: stay_f.a, b: stay_f.b, val: stay_f.val, ub: stay_f.ub, sel: stay_f.sel.clone() };
                let pl = sim::plan::plan_policy(&mut env, d, sk, cr, v, legs, &pool, &sv, sf, &stay.refs, pj);
                out["policy"] = json!({"plan": pl, "actions_proven": stay.proven && sv.proven});
            } else if let Some(stay_f) = stay.found.first() {
                let floor = policy.p_floor.max(stay_f.val).max(advance.val);
                let stay_estimate = env.lookahead(&pool, legs, stay_f, floor, 0.0, true);
                let advance_estimate = env.lookahead(&pool, legs, advance, floor, 0.0, true);
                out["policy"] = json!({"stay": stay_estimate, "advance": advance_estimate,
                    "actions_proven": stay.proven && sv.proven, "continuation_floor": floor,
                    "horizon": "one run, then repeat the best checked continuation; no further upgrades"});
            } else {
                out["policy"] = json!({"error": "No legal reroll career found for those parents."});
            }
        }
    }
    if cj.rollout > 0 && !sv.found.is_empty() {
        let t_r = Instant::now();
        let k = cj.rollout;
        out["rollout"] = if ntar > 12 || nu > 10 { json!({"error": "The rollout takes at most 12 targets, 10 of them not guaranteed by the scenario."}) } else {
            // the K best careers whoever trains (the job's trainees and fixed parents respected)
            let own;
            env.use_seen = false;
            let rsv: &Solved = if !cj.per_trainee && cj.top >= k { &sv } else { own = env.solve(&pool, &cj.fix, &trainees, false, k); &own };
            let mut cache = rollout::Cache::new(n);
            cache.absorb(&mut env, &rsv.refs);
            // today's best career over everything: the floor of every next career, and what repeating it costs
            let open = cj.fix.is_empty() && trainees.len() == env.owned.len();
            let (p_star, from, p_proven) = match cj.p_now {
                Some(x) => (x.max(rsv.found[0].val), "p_now", Value::Null),
                None if open => (rsv.found[0].val, "candidates", json!(rsv.proven)),
                None => { let all = env.owned.clone(); let b = env.solve(&pool, &[], &all, false, 1); cache.absorb(&mut env, &b.refs);
                    (b.found.first().map_or(0.0, |f| f.val).max(rsv.found[0].val), "search", json!(b.proven)) }
            };
            env.use_seen = true;
            let cands = &rsv.found[..k.min(rsv.found.len())];
            let (rows, pick, sl_proven, check) = env.rollout(&pool, legs, cands, p_star, &mut cache);
            let (pbar, pbar_t, pbar_proven) = rollout::p_bar_cached(&mut env, v);
            let list: Vec<Value> = cands.iter().zip(rows).enumerate().map(|(i, (f, row))| {
                let mut r = env.report(&pool, f, false);
                if let (Some(o), Some(x)) = (r.as_object_mut(), row.as_object()) { for (key, val) in x { if key != "p_all" { o.insert(key.clone(), val.clone()); } } o.insert("chosen".into(), json!(pick == Some(i))); }
                r
            }).collect();
            let choice = pick.map(|i| env.report(&pool, &cands[i], true));
            json!({"k": k, "shortlist": rollout::SHORTLIST, "pick": pick, "differs": pick.map_or(false, |i| i != 0), "choice": choice, "candidates": list,
                   "p_star": p_star, "repeat_careers": 1.0 / p_star, "p_star_from": from, "p_star_proven": p_proven,
                   "p_bar": pbar, "p_bar_trainee": pbar_t, "p_bar_proven": pbar_proven, "lower_bound": 1.0 + (1.0 - p_star) / pbar,
                   "candidates_proven": rsv.proven, "shortlist_proven": sl_proven, "decks": cache.n_decks(), "deck_searches": cache.searches, "table_check": check,
                   "timing_s": t_r.elapsed().as_secs_f64()})
        };
    }
    if let (Some(pj), None) = (&pj, &cj.policy) { if !sv.found.is_empty() {
        out["plan"] = sim::plan::plan(&mut env, d, sk, cr, v, legs, &pool, &sv, !cj.per_trainee, &cj.fix, &trainees, pj);
    } }
    if cj.lookahead { if let Some(f) = sv.found.first() {
        // today's best career over everything (the floor of what the next career can do)
        let (p_now, p_minus1_now) = match (cj.p_now, cj.p_minus1_now) {
            (Some(all), Some(near)) => (all, near),
            (Some(all), None) if (all - f.val).abs() <= all.abs() * 1e-9 => {
                let r = env.report(&pool, f, false); (all, r["p_minus1"].as_f64().unwrap())
            }
            _ => {
                let all = env.owned.clone();
                let best = env.solve(&pool, &[], &all, true, 1);
                let r = best.found.first().map(|f| env.report(&pool, f, false));
                (cj.p_now.unwrap_or_else(|| r.as_ref().map_or(0.0, |r| r["p_all"].as_f64().unwrap())),
                 r.as_ref().map_or(0.0, |r| r["p_minus1"].as_f64().unwrap()))
            }
        };
        out["lookahead"] = env.lookahead(&pool, legs, f, p_now, p_minus1_now, false);
    } }
    keep(&env);
    out["timing_s"] = json!(t_all.elapsed().as_secs_f64());
    out
}

#[cfg(test)]
mod policy_tests {
    use super::continuation_estimate;
    #[test]
    fn stronger_parent_outputs_can_outweigh_immediate_success() {
        // Shared tail floor 0.3. Rerolling succeeds less often now, but can produce a much better parent.
        let (stay, stay_two) = continuation_estimate(&[0.4, 0.4, 0.0, 0.2], &[0.8, 0.3, 0.3, 0.8]);
        let (advance, advance_two) = continuation_estimate(&[0.7, 0.0, 0.0, 0.3], &[0.3; 4]);
        assert!((stay.unwrap() - 17.0 / 6.0).abs() < 1e-12);
        assert!((advance.unwrap() - 10.0 / 3.0).abs() < 1e-12);
        assert!(stay < advance);
        assert!(stay_two > advance_two);
    }
    #[test]
    fn failure_without_a_continuation_is_not_a_finite_estimate() {
        assert_eq!(continuation_estimate(&[0.5, 0.5], &[0.0, 0.8]).0, None);
        assert_eq!(continuation_estimate(&[0.0, 1.0], &[0.0, 1.0]).0, Some(1.0));
    }
}
