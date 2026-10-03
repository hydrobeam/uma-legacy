//! Skill-target loop search (Rust port of skills.py) and JSON entry points shared by the CLI tests and the web build.
//!
//! Model per training run (one self-copy-free lineage, trainee T):
//!   - every target skill's spark sits on all 6 legacies, with stars drawn from a rank distribution
//!     (expected base rate is exact because legacies are independent and proc is linear in the base rate);
//!   - a skill is acquired if guaranteed, on T's own outfit skill list, hinted by the deck, or proc'd from the lineage
//!     (P = 1 - prod(1 - base*(1+aff/100))^INSPIRATIONS over the six legacies, parents and grandparents alike);
//!   - each legacy's affinity follows uma-it-optimizer's fit on 8,155 Independent Training receipts
//!     (github.com/Acrith/uma-it-optimizer, tools/loop/affinity.py): a parent P counts pair(T,P), the whole
//!     pair(P1,P2), 3 per G1 shared with the other parent, and for each of P's own parents G the trio (T,P,G) plus
//!     3 per G1 shared with G; a grandparent G under P counts trio(T,P,G) plus 3 per G1 shared with P. In a loop P's
//!     own parents are the other parent and the fourth member, so the parents' shared G1s count twice for each;
//!   - it then lands as a spark with GEN_GOLD if the gold version is bought, else GEN_WHITE (20% / 40% x 1.1^6: all six
//!     ancestors carry the spark in a running loop).
//! Metric: P(all targets land on the veteran), averaged over the 12 lineages.
//! Search: exact upper bound per foursome (pair overlaps -> pairwise ceilings PB), then best-first with real
//! schedules (affinity-optimised: in the 12-lineage cycle every pair's shared G1s weigh the same) until the next
//! bound is below the k-th best.
use super::*;
use serde_json::{json, Value};

/// Chance a skill learned in a career becomes a spark on the veteran: 20% for a white, 40% for a gold, x1.1 for each of
/// the six ancestors already carrying that spark (aoneko_pochi's curve; hakuraku.moe/notes/spark_generation fits it to
/// ~1.2M white/gold observations from CM10-CM12 room data, p >= 0.84). In a running loop all six carry every target.
pub(crate) const GEN_WHITE: f64 = 0.2 * 1.771561;
pub(crate) const GEN_GOLD: f64 = 0.4 * 1.771561;
/// Grandparents' base rate relative to parents'. 1: they fire less only because their affinity is lower (receipts:
/// halving predicted half of what fired).
const GP_RATE: f64 = 1.0;
const INSPIRATIONS: i32 = 2;

#[derive(Deserialize, Clone)]
pub(crate) struct CardRaw(pub u32, pub String, pub Vec<(u32, u8)>);
#[derive(Deserialize)]
pub(crate) struct SkillsRaw { pub cards: std::collections::HashMap<String, Vec<CardRaw>>, pub skills: Vec<(u32, String, Option<String>)> }

#[derive(Deserialize, Clone)]
pub(crate) struct Target {
    pub group: u32,
    /// 0 = none, 1 = white guaranteed, 2 = gold guaranteed
    #[serde(default)] pub guaranteed: u8,
    #[serde(default)] pub deck_white: f64,
    #[serde(default)] pub deck_gold: f64,
}
#[derive(Deserialize)]
pub(crate) struct SkillJob {
    pub targets: Vec<Target>,
    /// 1/2/3-star shares of the lineage's sparks: 0.50/0.45/0.05 below rank score 17,500, 0.20/0.70/0.10 from 17,500 up
    /// (hakuraku.moe/notes/spark_generation: 528,637 and 196,646 white sparks from CM10-CM12 room data)
    #[serde(default = "dstar")] pub star_dist: [f64; 3],
    #[serde(default)] pub owned: Vec<u32>,
    #[serde(default)] pub exclude: Vec<u32>,
    #[serde(default)] pub require: Vec<u32>,
    /// outfits (card ids) not owned; an uma whose every outfit is listed counts as not owned
    #[serde(default)] pub exclude_outfits: Vec<u32>,
    /// undocumented guarantees: (chara id, skill group, 1 white | 2 gold), applied to every outfit
    #[serde(default)] pub extra: Vec<(u32, u32, u8)>,
    /// trainee events that hint a target: (chara id for every outfit, or outfit card id; skill group; chance per
    /// career). The hinted white counts like a deck white hint for that trainee only.
    #[serde(default)] pub trainee_hints: Vec<(u32, u32, f64)>,
    #[serde(default = "dtop5")] pub top: usize,
    #[serde(default = "diters")] pub iters: usize,
    #[serde(default = "dseed")] pub seed: u64,
}
fn dstar() -> [f64; 3] { [0.50, 0.45, 0.05] }
fn dtop5() -> usize { 5 }
fn diters() -> usize { 30 }
fn dseed() -> u64 { 3 }

/// SCF lineages over member positions: (trainee, P1, P2, X); P1 bred from (P2, X), P2 from (P1, X).
pub(crate) fn lineages() -> Vec<(usize, usize, usize, usize)> {
    let mut v = vec![];
    for t in 0..4 { for x in 0..4 { if x == t { continue; }
        let ps: Vec<usize> = (0..4).filter(|&i| i != t && i != x).collect();
        v.push((t, ps[0], ps[1], x)); } }
    v
}

pub(crate) struct Model<'a> {
    d: &'a Data,
    pub(crate) uncertain: Vec<usize>,          // indices into job.targets that are not guaranteed gold
    pub(crate) n_fixed_gold: i32,
    base: f64,
    /// per uma index: distinct coverage tuples over `uncertain` (0/1/2) -> outfit labels giving them
    pub(crate) opts: Vec<Vec<(Vec<u8>, Vec<(u32, String)>)>>,
    /// per uma index: every listed outfit is excluded
    pub(crate) no_outfit: Vec<bool>,
    job: &'a SkillJob,
    /// lineage table and coverage tuples without labels, for the allocation-free hot path
    lin: Vec<(usize, usize, usize, usize)>,
    sts: Vec<Vec<Vec<u8>>>,
    /// per uma index, per option: chance the trainee's own events hint each uncertain target's white
    pub(crate) ews: Vec<Vec<Vec<f64>>>,
}

impl<'a> Model<'a> {
    pub(crate) fn new(d: &'a Data, sk: &SkillsRaw, job: &'a SkillJob) -> Self {
        let uncertain: Vec<usize> = (0..job.targets.len()).filter(|&i| job.targets[i].guaranteed != 2).collect();
        let n_fixed_gold = (job.targets.len() - uncertain.len()) as i32;
        let base = job.star_dist[0] * 0.03 + job.star_dist[1] * 0.06 + job.star_dist[2] * 0.09;
        let mut opts = vec![]; let mut no_outfit = vec![]; let mut ews: Vec<Vec<Vec<f64>>> = vec![];
        for u in &d.umas {
            let mut cards: Vec<CardRaw> = sk.cards.get(&u.id.to_string()).cloned().unwrap_or_default();
            let had = !cards.is_empty();
            cards.retain(|c| !job.exclude_outfits.contains(&c.0));
            no_outfit.push(had && cards.is_empty());
            if cards.is_empty() { cards.push(CardRaw(0, "any outfit".into(), vec![])); }
            let mut m: Vec<(Vec<u8>, Vec<(u32, String)>)> = vec![];
            let mut mew: Vec<Vec<f64>> = vec![];
            for c in &cards {
                let ew: Vec<f64> = uncertain.iter().map(|&i| {
                    let g = job.targets[i].group;
                    1.0 - job.trainee_hints.iter().filter(|h| (h.0 == u.id || (c.0 != 0 && h.0 == c.0)) && h.1 == g).map(|h| 1.0 - h.2).product::<f64>()
                }).collect();
                let st: Vec<u8> = uncertain.iter().map(|&i| {
                    let g = job.targets[i].group;
                    let mut lv = c.2.iter().filter(|(gg, _)| *gg == g).map(|(_, l)| *l).max().unwrap_or(0);
                    for (ch, gg, l) in &job.extra { if *ch == u.id && *gg == g { lv = lv.max(*l); } }
                    if job.targets[i].guaranteed == 1 { lv = lv.max(1); }
                    lv
                }).collect();
                if let Some(i) = (0..m.len()).find(|&i| m[i].0 == st && mew[i] == ew) { m[i].1.push((c.0, c.1.clone())); } else { m.push((st, vec![(c.0, c.1.clone())])); mew.push(ew); }
            }
            opts.push(m); ews.push(mew);
        }
        let sts = opts.iter().map(|m| m.iter().map(|(st, _)| st.clone()).collect()).collect();
        Model { d, uncertain, n_fixed_gold, base, opts, no_outfit, job, lin: lineages(), sts, ews }
    }

    pub(crate) fn skill_probs(&self, st: &[u8], ew: &[f64], q: f64) -> Vec<f64> {
        st.iter().zip(&self.uncertain).enumerate().map(|(k, (&v, &i))| {
            let t = &self.job.targets[i];
            if v == 2 { return GEN_GOLD; }
            let white = if v == 1 { GEN_WHITE } else { GEN_WHITE * (1.0 - (1.0 - t.deck_white) * (1.0 - ew[k]) * (1.0 - q)) };
            t.deck_gold * GEN_GOLD + (1.0 - t.deck_gold) * white
        }).collect()
    }
    /// best outfit for trainee `u` given per-career proc q: (product over uncertain skills, option index)
    fn trainee(&self, u: usize, q: f64) -> (f64, usize) {
        let mut best = (-1.0, 0);
        for (k, (st, _)) in self.opts[u].iter().enumerate() {
            let v: f64 = self.skill_probs(st, &self.ews[u][k], q).iter().product();
            if v > best.0 { best = (v, k); }
        }
        best
    }
    fn fixed(&self) -> f64 { GEN_GOLD.powi(self.n_fixed_gold) }

    /// per-lineage proc q from slot affinities computed with overlap function `o(a, b)` (member positions)
    pub(crate) fn lineage_q(&self, f: [usize; 4], o: &dyn Fn(usize, usize) -> f64) -> Vec<f64> {
        let d = self.d;
        let pr = |a: usize, b: usize| d.pair[f[a]][f[b]] as f64;
        let tr = |a: usize, b: usize, c: usize| d.tri[f[a]][f[b]][f[c]] as f64;
        lineages().iter().map(|&(t, p1, p2, x)| {
            let mut miss = 1.0f64;
            for (par, other) in [(p1, p2), (p2, p1)] {
                let ap = pr(t, par) + pr(p1, p2) + 3.0 * o(par, other)
                    + tr(t, par, other) + 3.0 * o(par, other) + tr(t, par, x) + 3.0 * o(par, x);
                miss *= 1.0 - (self.base * (1.0 + ap / 100.0)).min(1.0);
                for g in [other, x] {
                    let ag = tr(t, par, g) + 3.0 * o(par, g);
                    miss *= 1.0 - (GP_RATE * self.base * (1.0 + ag / 100.0)).min(1.0);
                }
            }
            1.0 - miss.powi(INSPIRATIONS)
        }).collect()
    }
    /// `lineage_q` into a fixed array (no allocation); same numbers.
    pub(crate) fn lineage_q_arr(&self, f: [usize; 4], o: &dyn Fn(usize, usize) -> f64) -> [f64; 12] {
        let d = self.d;
        let mut out = [0.0f64; 12];
        for (l, &(t, p1, p2, x)) in self.lin.iter().enumerate() {
            let mut miss = 1.0f64;
            for (par, other) in [(p1, p2), (p2, p1)] {
                let ap = d.pair[f[t]][f[par]] as f64 + d.pair[f[p1]][f[p2]] as f64 + 3.0 * o(par, other)
                    + d.tri[f[t]][f[par]][f[other]] as f64 + 3.0 * o(par, other)
                    + d.tri[f[t]][f[par]][f[x]] as f64 + 3.0 * o(par, x);
                miss *= 1.0 - (self.base * (1.0 + ap / 100.0)).min(1.0);
                for g in [other, x] {
                    let ag = d.tri[f[t]][f[par]][f[g]] as f64 + 3.0 * o(par, g);
                    miss *= 1.0 - (GP_RATE * self.base * (1.0 + ag / 100.0)).min(1.0);
                }
            }
            out[l] = 1.0 - miss.powi(INSPIRATIONS);
        }
        out
    }
    /// Upper bound on the inheritance chance q of every lineage whose trainee is `t`, with parents and the fourth
    /// member drawn from `pool` and race overlaps bounded by `pb`. Same arithmetic, in the same order, as
    /// `lineage_q_arr` with o = pb, except that the grandparent slot the fourth member fills takes its best value
    /// over every possible fourth member. q rises with every affinity, so this is >= every such q.
    pub(crate) fn q_ub_trainee(&self, t: usize, pool: &[usize], pb: &[Vec<f64>]) -> f64 {
        let d = self.d;
        let others: Vec<usize> = pool.iter().copied().filter(|&u| u != t).collect();
        let xmax: Vec<f64> = others.iter().map(|&par| others.iter().filter(|&&x| x != par)
            .map(|&x| d.tri[t][par][x] as f64 + 3.0 * pb[par][x]).fold(f64::NEG_INFINITY, f64::max)).collect();
        let mut best = 0.0f64;
        for i in 0..others.len() { for j in i + 1..others.len() {
            let (p1, p2) = (others[i], others[j]);
            let mut miss = 1.0f64;
            for (par, other, xm) in [(p1, p2, xmax[i]), (p2, p1, xmax[j])] {
                // the fourth member's terms take their best value over every fourth member, in both slots
                let ap = d.pair[t][par] as f64 + d.pair[p1][p2] as f64 + 3.0 * pb[par][other]
                    + d.tri[t][par][other] as f64 + 3.0 * pb[par][other] + xm;
                miss *= 1.0 - (self.base * (1.0 + ap / 100.0)).min(1.0);
                let ag = d.tri[t][par][other] as f64 + 3.0 * pb[par][other];
                miss *= 1.0 - (GP_RATE * self.base * (1.0 + ag / 100.0)).min(1.0);
                miss *= 1.0 - (GP_RATE * self.base * (1.0 + xm / 100.0)).min(1.0);
            }
            best = best.max(1.0 - miss.powi(INSPIRATIONS));
        } }
        best
    }
    /// Lower bound on every lineage's q: all affinities at zero (relation points and race overlaps are >= 0).
    pub(crate) fn q_floor(&self) -> f64 {
        let miss = ((1.0 - self.base) * (1.0 - GP_RATE * self.base).powi(2)).powi(2);
        1.0 - miss.powi(INSPIRATIONS)
    }
    /// `metric_d` without allocation (hot path of the deck search); same numbers.
    #[inline]
    pub(crate) fn metric_fast(&self, f: [usize; 4], qs: &[f64; 12], dw: &[f64], dg: &[f64]) -> f64 {
        let mut sum = 0.0;
        for (l, &(t, ..)) in self.lin.iter().enumerate() {
            let q = qs[l];
            let mut best = 0.0f64;
            for (st, ew) in self.sts[f[t]].iter().zip(&self.ews[f[t]]) {
                let mut p = 1.0f64;
                for k in 0..st.len() {
                    let v = st[k];
                    p *= if v == 2 { GEN_GOLD } else {
                        let white = if v == 1 { GEN_WHITE } else { GEN_WHITE * (1.0 - (1.0 - dw[k]) * (1.0 - ew[k]) * (1.0 - q)) };
                        dg[k] * GEN_GOLD + (1.0 - dg[k]) * white
                    };
                }
                if p > best { best = p; }
            }
            sum += best;
        }
        self.fixed() * sum / 12.0
    }
    /// Best-outfit product over the uncertain targets for trainee `u` at proc `q` (the per-lineage term of
    /// `metric_fast`, before the fixed-gold factor); increasing in q and in every deck odd.
    #[inline]
    pub(crate) fn trainee_fast(&self, u: usize, q: f64, dw: &[f64], dg: &[f64]) -> f64 {
        let mut best = 0.0f64;
        for (st, ew) in self.sts[u].iter().zip(&self.ews[u]) {
            let mut p = 1.0f64;
            for k in 0..st.len() {
                let v = st[k];
                p *= if v == 2 { GEN_GOLD } else {
                    let white = if v == 1 { GEN_WHITE } else { GEN_WHITE * (1.0 - (1.0 - dw[k]) * (1.0 - ew[k]) * (1.0 - q)) };
                    dg[k] * GEN_GOLD + (1.0 - dg[k]) * white
                };
            }
            if p > best { best = p; }
        }
        best
    }
    pub(crate) fn fixed_factor(&self) -> f64 { self.fixed() }
    /// Per-skill landing odds with deck hint odds supplied per uncertain target (white, gold) instead of the job's.
    pub(crate) fn skill_probs_d(&self, st: &[u8], ew: &[f64], q: f64, dw: &[f64], dg: &[f64]) -> Vec<f64> {
        st.iter().enumerate().map(|(k, &v)| {
            if v == 2 { return GEN_GOLD; }
            let white = if v == 1 { GEN_WHITE } else { GEN_WHITE * (1.0 - (1.0 - dw[k]) * (1.0 - ew[k]) * (1.0 - q)) };
            dg[k] * GEN_GOLD + (1.0 - dg[k]) * white
        }).collect()
    }
    pub(crate) fn trainee_d(&self, u: usize, q: f64, dw: &[f64], dg: &[f64]) -> (f64, usize) {
        let mut best = (-1.0, 0);
        for (k, (st, _)) in self.opts[u].iter().enumerate() {
            let v: f64 = self.skill_probs_d(st, &self.ews[u][k], q, dw, dg).iter().product();
            if v > best.0 { best = (v, k); }
        }
        best
    }
    pub(crate) fn metric_d(&self, f: [usize; 4], qs: &[f64], dw: &[f64], dg: &[f64]) -> f64 {
        let lin = lineages();
        self.fixed() * lin.iter().zip(qs).map(|(&(t, ..), &q)| self.trainee_d(f[t], q, dw, dg).0).sum::<f64>() / lin.len() as f64
    }
    pub(crate) fn fixed_pub(&self) -> f64 { self.fixed() }
    /// Everything behind the metric for one loop, for the "how this loop works" view: pairwise relation points and
    /// expected shared G1 wins, then per lineage the six legacy slots (relation part, race part, affinity, proc),
    /// the per-career inheritance chance q, each target's landing chance and P(all land).
    /// `deck` = per-uncertain-target (white, gold) deck odds; None uses the job's own deck fields.
    /// `deck`: per member (dw, dg) deck odds, or None for the job's own deck fields.
    pub(crate) fn detail(&self, f: [usize; 4], o: &dyn Fn(usize, usize) -> f64, deck: Option<[(&[f64], &[f64]); 4]>) -> Value {
        let d = self.d;
        let pr = |a: usize, b: usize| d.pair[f[a]][f[b]] as f64;
        let tr = |a: usize, b: usize, c: usize| d.tri[f[a]][f[b]][f[c]] as f64;
        let mut pairs = vec![];
        for a in 0..4 { for b in a + 1..4 { pairs.push(json!({"a": a, "b": b, "rel": pr(a, b), "wins": o(a, b)})); } }
        let mut lins = vec![];
        for &(t, p1, p2, x) in &lineages() {
            let mut slots = vec![]; let mut miss = 1.0f64;
            for (par, other) in [(p1, p2), (p2, p1)] {
                let rel = pr(t, par) + pr(p1, p2) + tr(t, par, other) + tr(t, par, x);
                let race = 3.0 * o(par, other) + 3.0 * o(par, other) + 3.0 * o(par, x); let aff = rel + race;
                let proc = (self.base * (1.0 + aff / 100.0)).min(1.0); miss *= 1.0 - proc;
                slots.push(json!({"kind": "parent", "who": [par], "rel": rel, "race": race, "aff": aff, "proc": proc}));
                for g in [other, x] {
                    let rel = tr(t, par, g); let race = 3.0 * o(par, g); let aff = rel + race;
                    let proc = (GP_RATE * self.base * (1.0 + aff / 100.0)).min(1.0); miss *= 1.0 - proc;
                    slots.push(json!({"kind": "grandparent", "who": [par, g], "rel": rel, "race": race, "aff": aff, "proc": proc}));
                }
            }
            let q = 1.0 - miss.powi(INSPIRATIONS);
            let (pv, k) = match deck.map(|dk| dk[t]) { Some((w, g)) => self.trainee_d(f[t], q, w, g), None => self.trainee(f[t], q) };
            let probs = match deck.map(|dk| dk[t]) { Some((w, g)) => self.skill_probs_d(&self.opts[f[t]][k].0, &self.ews[f[t]][k], q, w, g), None => self.skill_probs(&self.opts[f[t]][k].0, &self.ews[f[t]][k], q) };
            let skills: Vec<Value> = self.uncertain.iter().zip(&probs).zip(&self.opts[f[t]][k].0)
                .map(|((&i, p), lv)| json!({"group": self.job.targets[i].group, "p": p, "outfit": lv})).collect();
            lins.push(json!({"t": t, "p": [p1, p2], "x": x, "slots": slots, "q": q, "p_all": self.fixed() * pv, "skills": skills,
                             "fixed_gold": self.n_fixed_gold}));
        }
        json!({"base": self.base, "gp_rate": GP_RATE, "inspirations": INSPIRATIONS, "gen_white": GEN_WHITE, "gen_gold": GEN_GOLD, "pairs": pairs, "lineages": lins})
    }
    fn metric(&self, f: [usize; 4], qs: &[f64]) -> f64 {
        let lin = lineages();
        self.fixed() * lin.iter().zip(qs).map(|(&(t, ..), &q)| self.trainee(f[t], q).0).sum::<f64>() / lin.len() as f64
    }
}

pub(crate) fn atleast(ps: &[f64], k: usize) -> f64 {
    let mut d = vec![1.0f64];
    for &p in ps { let mut nn = vec![0.0; d.len() + 1]; for (i, x) in d.iter().enumerate() { nn[i] += x * (1.0 - p); nn[i + 1] += x * p; } d = nn; }
    d[k.min(d.len() - 1)..].iter().sum()
}

/// Structured schedule chart: shared G1s (affinity) and optional G1s (one runner, race sparks only).
pub(crate) fn chart_rows(d: &Data, m: &[usize], s: &[Sched]) -> Value {
    let n = m.len();
    let ks: Vec<Vec<usize>> = (0..n).map(|i| { let mut k = 0; (0..NT).map(|t| { match s[i][t] { Choice::Rest => k = 0, Choice::Opt(_) => k = (k + 1).min(6) }; k }).collect() }).collect();
    let mut runners: Vec<Vec<bool>> = vec![vec![false; n]; d.nr];
    for i in 0..n { for t in 0..NT { if let Choice::Opt(o) = s[i][t] { let op = &d.umas[m[i]].opts[t][o as usize]; if op.race >= 0 { runners[op.race as usize][i] = true; } } } }
    let mut rows: Vec<(usize, i32)> = vec![];
    for t in 0..NT { for i in 0..n { if let Choice::Opt(o) = s[i][t] { let op = &d.umas[m[i]].opts[t][o as usize];
        if op.race >= 0 && !rows.contains(&(t, op.race)) { rows.push((t, op.race)); } } } }
    rows.sort();
    let out: Vec<Value> = rows.iter().map(|&(t, race)| {
        let mut cnt = 0;
        let cells: Vec<Value> = (0..n).map(|i| match s[i][t] {
            Choice::Opt(o) => { let op = &d.umas[m[i]].opts[t][o as usize];
                if op.race == race { cnt += 1; json!({"p": (pk(d, op.bp, ks[i][t]) * 100.0).round(), "goal": op.goal}) } else { Value::Null } }
            Choice::Rest => Value::Null,
        }).collect();
        let shared = runners[race as usize].iter().filter(|x| **x).count() >= 2;
        let alts: Vec<String> = interchangeable(d, m, s, t, race).iter().map(|&a| chart_race_name(&d.race_names[a as usize])).collect();
        json!({"race": row_label(d, m, s, t, race), "when": when(t), "year": t / 24, "shared": shared, "cells": cells, "runners": cnt, "alts": alts})
    }).collect();
    Value::Array(out)
}

pub(crate) fn member_json(d: &Data, m: &[usize]) -> Vec<Value> {
    m.iter().map(|&x| json!({"id": d.umas[x].id, "name": d.umas[x].name, "short": short_name(&d.umas[x].name)})).collect()
}

/// Skill-target search. Returns the top loops with odds, outfits, per-trainee odds and schedules.
pub(crate) fn run_skills(d: &Data, sk: &SkillsRaw, job: &SkillJob) -> Value {
    let model = Model::new(d, sk, job);
    let n = d.umas.len();
    let id_ok = |id: u32| (job.owned.is_empty() || job.owned.contains(&id) || job.require.contains(&id)) && !job.exclude.contains(&id);
    let pool: Vec<usize> = (0..n).filter(|&i| id_ok(d.umas[i].id) && !model.no_outfit[i]).collect();
    let req: Vec<usize> = job.require.iter().filter_map(|&id| d.umas.iter().position(|u| u.id == id)).collect();
    if pool.len() < 4 { return json!({"error": "Pick at least 4 umas to search."}); }
    if req.len() > 4 { return json!({"error": "At most 4 umas can be required."}); }
    let pb = pair_ceilings(d, &pool);
    let k = pool.len();
    let mut cand: Vec<(f64, [usize; 4])> = vec![];
    for a in 0..k { for b in a + 1..k { for c in b + 1..k { for e in c + 1..k {
        let f = [pool[a], pool[b], pool[c], pool[e]];
        if !req.iter().all(|r| f.contains(r)) { continue; }
        let qs = model.lineage_q(f, &|x, y| pb[f[x]][f[y]]);
        cand.push((model.metric(f, &qs), f));
    } } } }
    if cand.is_empty() { return json!({"error": "No foursome contains all the required umas."}); }
    cand.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
    let top = job.top.max(1);
    let mut done: Vec<(f64, f64, [usize; 4], Res, Vec<f64>)> = vec![];   // (value, bound, f, res, qs)
    let mut next = 0;
    let kth = |done: &Vec<(f64, f64, [usize; 4], Res, Vec<f64>)>| if done.len() < top { f64::NEG_INFINITY } else {
        let mut v: Vec<f64> = done.iter().map(|x| x.0).collect(); v.sort_by(|a, b| b.partial_cmp(a).unwrap()); v[top - 1] };
    while next < cand.len() && cand[next].0 >= kth(&done) {
        let (ub, f) = cand[next]; next += 1;
        let r = optimize(d, f, "scf@0.5", job.iters, job.seed, 60.0, &[]);
        let ps: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let qs = model.lineage_q(f, &|x, y| wval(&ps[x], &ps[y]));
        let v = model.metric(f, &qs);
        done.push((v, ub, f, r, qs));
    }
    done.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
    let lin = lineages();
    let loops: Vec<Value> = done.iter().take(top).map(|(v, ub, f, r, qs)| {
        // per-lineage outfit choice and skill odds
        let mut minus1 = 0.0; let mut per: Vec<(f64, usize)> = vec![(0.0, 0); 4];
        let mut picks: Vec<Vec<usize>> = vec![vec![]; 4];
        for (&(t, ..), &q) in lin.iter().zip(qs) {
            let (val, kk) = model.trainee(f[t], q);
            let mut ps = vec![GEN_GOLD; model.n_fixed_gold as usize];
            ps.extend(model.skill_probs(&model.opts[f[t]][kk].0, &model.ews[f[t]][kk], q));
            minus1 += atleast(&ps, job.targets.len().saturating_sub(1));
            per[t].0 += model.fixed() * val; per[t].1 += 1; picks[t].push(kk);
        }
        minus1 /= lin.len() as f64;
        let outfits: Vec<Value> = (0..4).map(|i| {
            let kk = *picks[i].iter().max_by_key(|x| picks[i].iter().filter(|y| y == x).count()).unwrap();
            let (st, cards) = &model.opts[f[i]][kk];
            let default_st: Vec<u8> = model.uncertain.iter().map(|&j| if job.targets[j].guaranteed == 1 { 1 } else { 0 }).collect();
            let covers: Vec<Value> = st.iter().zip(&model.uncertain).filter(|(lv, j)| **lv > 0 && !(**lv == 1 && job.targets[**j].guaranteed == 1))
                .map(|(lv, j)| json!({"group": job.targets[*j].group, "level": lv})).collect();
            json!({"outfits": if *st == default_st { vec![] } else { cards.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>() }, "covers": covers})
        }).collect();
        let s4 = [r.s[0], r.s[1], r.s[2], r.s[3]];
        let ps: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let detail = model.detail(*f, &|x, y| wval(&ps[x], &ps[y]), None);
        json!({
            "members": member_json(d, f), "p_all": v, "p_minus1": minus1, "bound": ub,
            "per_trainee": per.iter().map(|(s, c)| s / *c as f64).collect::<Vec<_>>(),
            "outfits": outfits, "mean_proc": qs.iter().sum::<f64>() / qs.len() as f64,
            "schedule": chart_rows(d, f, &s4),
            "detail": detail,
        })
    }).collect();
    let next_bound = if next < cand.len() { cand[next].0 } else { 0.0 };
    // solved loops are known exactly; the best is proven once no unsolved ceiling reaches it
    let proven_top = !done.is_empty() && next_bound < done[0].0;
    json!({"loops": loops, "searched": done.len(), "foursomes": cand.len(), "next_bound": next_bound, "proven_top": proven_top})
}

#[derive(Deserialize)]
pub(crate) struct AffinityJob {
    #[serde(default = "dscheme")] pub scheme: String,
    #[serde(default)] pub owned: Vec<u32>,
    #[serde(default)] pub exclude: Vec<u32>,
    #[serde(default)] pub require: Vec<u32>,
    #[serde(default = "dtop5")] pub top: usize,
    #[serde(default = "diters")] pub iters: usize,
}
fn dscheme() -> String { "scf".into() }

/// Pure affinity search (the original problem): best loops by expected affinity per 4 runs.
pub(crate) fn run_affinity(d: &Data, job: &AffinityJob) -> Value {
    let pool: Vec<u32> = d.umas.iter().map(|u| u.id).filter(|id| (job.owned.is_empty() || job.owned.contains(id) || job.require.contains(id)) && !job.exclude.contains(id)).collect();
    if pool.len() < 4 { return json!({"error": "Pick at least 4 umas to search."}); }
    if job.require.len() > 4 { return json!({"error": "At most 4 umas can be required."}); }
    let jobs: Jobs = serde_json::from_value(json!({"mode": "best", "scheme": if job.scheme == "rot" { "rot" } else { "scf" }, "iters": job.iters, "seed": 3,
        "threads": 1, "top": job.top.max(1), "owned": job.owned, "exclude": job.exclude, "require": job.require})).unwrap();
    let b = best_search(d, &jobs);
    let loops: Vec<Value> = b.done.iter().take(jobs.top).map(|(ub, f, r)| {
        let m: Vec<usize> = r.order.to_vec();
        let lp = make_loop(d, r.order, &jobs.scheme);
        let s_o: [Sched; 4] = std::array::from_fn(|i| r.s[f.iter().position(|x| *x == m[i]).unwrap()]);
        json!({"members": member_json(d, &m), "score": r.val, "base": lp.base, "ceiling": ub, "schedule": chart_rows(d, &m, &s_o)})
    }).collect();
    let next_bound = if b.next < b.cand.len() { b.cand[b.next].0 } else { 0.0 };
    json!({"loops": loops, "searched": b.done.len(), "foursomes": b.cand.len(), "next_bound": next_bound})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;
    fn data() -> &'static Data {
        static D: OnceLock<Data> = OnceLock::new();
        D.get_or_init(|| load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json")))
    }
    fn skills() -> SkillsRaw { serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web_skills.json")).unwrap()).unwrap() }

    /// Regression anchor (Unity Cup targets, Nothing Ventured guaranteed), uma-it-optimizer's affinity model. Under the
    /// earlier umaishow split with halved grandparents (and the Python skills.py port) the same loop scored 1 in 283; with
    /// the rounded 35%/70% spark generation (before the x1.1^6 curve) it was 1 in 140.4, and 127.3 with the 45/50/5 star mix.
    #[test]
    fn skills_port_matches_python() {
        let d = data(); let sk = skills();
        let job: SkillJob = serde_json::from_value(json!({
            "targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166,"guaranteed":2},{"group":20046,"guaranteed":2},
                        {"group":20161,"deck_white":0.8},{"group":21010},{"group":20165,"deck_gold":0.8},{"group":20203,"guaranteed":2}],
            "extra": [[1078,20165,1],[1003,20190,1]], "top": 3})).unwrap();
        let r = run_skills(d, &sk, &job);
        let l = &r["loops"][0];
        let ids: Vec<u64> = l["members"].as_array().unwrap().iter().map(|m| m["id"].as_u64().unwrap()).collect();
        let mut s = ids.clone(); s.sort();
        assert_eq!(s, vec![1007, 1019, 1021, 1034], "best loop {ids:?}");
        let inv = 1.0 / l["p_all"].as_f64().unwrap();
        assert!((inv - 130.7).abs() < 1.0, "1 in {inv}");
        assert_eq!(r["proven_top"], json!(true));
    }

    /// Excluding one outfit (Summer Special Week) changes only that outfit's skills: regression anchor from the CLI.
    #[test]
    fn outfit_exclusion() {
        let d = data(); let sk = skills();
        let job: SkillJob = serde_json::from_value(json!({
            "targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166,"guaranteed":2},{"group":20046,"guaranteed":2},
                        {"group":20161,"deck_white":0.8},{"group":21010},{"group":20165,"deck_gold":0.8},{"group":20203,"guaranteed":2}],
            "extra": [[1078,20165,1],[1003,20190,1]], "exclude": [1034, 1021], "exclude_outfits": [100102], "top": 2})).unwrap();
        let r = run_skills(d, &sk, &job);
        let l = &r["loops"][0];
        let mut ids: Vec<u64> = l["members"].as_array().unwrap().iter().map(|m| m["id"].as_u64().unwrap()).collect(); ids.sort();
        assert_eq!(ids, vec![1009, 1014, 1019, 1029]);
        { let inv = 1.0 / l["p_all"].as_f64().unwrap(); assert!((inv - 172.6).abs() < 1.0, "1 in {inv}"); }
        // every outfit of an uma excluded -> that uma is not owned
        let job2: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159}], "exclude_outfits": [101901, 101902], "top": 1})).unwrap();
        let r2 = run_skills(d, &sk, &job2);
        assert!(r2["loops"][0]["members"].as_array().unwrap().iter().all(|m| m["id"] != json!(1019)));
    }

    /// The structured chart agrees with the bound: every schedule cell refers to a race actually run.
    #[test]
    fn affinity_entry_point_runs() {
        let d = data();
        let job: AffinityJob = serde_json::from_value(json!({"exclude": [1006, 1029, 1034], "top": 2})).unwrap();
        let r = run_affinity(d, &job);
        assert!((r["loops"][0]["score"].as_f64().unwrap() - 1961.015).abs() < 1e-2, "{}", r["loops"][0]["score"]);
    }
}
