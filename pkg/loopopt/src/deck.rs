//! Joint deck + loop search for skill targets.
//!
//! Deck model (per training run):
//!   - training hints: each card gives h = 1 + hint_frequency/25 hints, each a uniform draw over its hint options
//!     (skills + one stat option), so P(card hints skill s) = 1 - (1 - 1/opts)^h when s is in its pool;
//!   - chain events: the number of chain events in a run is Poisson(lambda); each one advances a chain picked
//!     uniformly among the remaining chain slots of the non-pal cards (a card with more links left is picked
//!     more often; links fire in order). Pals have no chain slots. lambda = 7.1 fits 6 slots -> 81%,
//!     8 -> 59%, 9 -> 50% full-chain completion;
//!   - event rewards come from the job (`events`): (card, skill group, 1 white | 2 gold, link; 0 = whenever the
//!     card is in the deck, e.g. pal golds).
//! Deck hint odds per target: 1 - prod(1 - p) over sources, separately for white and gold.
//!
//! Search (exact for the model): race schedules maximise affinity and do not depend on the deck, so each
//! foursome's per-lineage proc rates are computed once and every deck on the Pareto frontier is scored against
//! them. Foursomes are visited in descending order of a ceiling computed with the best odds any deck gives per
//! target; a foursome is solved only while its ceiling for some pal count can still beat that pal count's best.
use super::*;
use crate::skills::{atleast, chart_rows, lineages, member_json, Model, SkillJob, SkillsRaw, GEN_GOLD};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Deserialize, Clone)]
pub(crate) struct Card { pub id: u32, pub name: String, pub chara: u32, pub rarity: u8, pub pal: bool, pub links: u8, pub hf: f64, pub opts: u32, pub whites: Vec<u32> }
#[derive(Deserialize)]
pub(crate) struct CardsRaw { pub cards: Vec<Card> }

#[derive(Deserialize)]
pub(crate) struct DeckJob {
    #[serde(default = "d_lambda")] pub lambda: f64,
    #[serde(default = "d_size")] pub deck_size: usize,
    /// (card id, skill group, 1 white | 2 gold, chain link 1..; 0 = whenever the card is in the deck)
    #[serde(default)] pub events: Vec<(u32, u32, u8, u8)>,
    /// chain events whose reward depends on the outcome: (card, group, level, link, chance given the link fires)
    #[serde(default)] pub events_p: Vec<(u32, u32, u8, u8, f64)>,
    /// non-chain events with a fixed chance per run when the card is in the deck: (card, group, 1 | 2, chance)
    #[serde(default)] pub odds_events: Vec<(u32, u32, u8, f64)>,
    /// choices that end a card's chain: (card, link) for every such choice, and their skill rewards
    /// (card, link, group, 1 | 2, chance given the link fires)
    #[serde(default)] pub end_links: Vec<(u32, u8)>,
    #[serde(default)] pub ends: Vec<(u32, u8, u32, u8, f64)>,
    #[serde(default)] pub owned_cards: Vec<u32>,
    /// allow one card the player does not own (a friend's card) in the deck
    #[serde(default)] pub borrow: bool,
    #[serde(default)] pub lock_cards: Vec<u32>,
    #[serde(default)] pub exclude_cards: Vec<u32>,
    /// lowest rarity allowed for cards with chain events (R cards have none and always count: no chain slots)
    #[serde(default = "d_rar")] pub min_rarity: u8,
    /// only decks with exactly this many pals
    #[serde(default)] pub pals: Option<usize>,
    /// how many distinct decks to return (each with its best loop)
    #[serde(default = "d_one")] pub top_decks: usize,
    /// experiment: fixed per-uma race schedules instead of loop-optimised ones. "solo" = each uma maximises her
    /// own expected race wins; "goals" = career objectives only; "none" = shared wins count for nothing.
    #[serde(default)] pub schedule: Option<String>,
}

/// Fixed schedule for one uma (see DeckJob::schedule).
fn prebuilt(d: &Data, u: &Uma, mode: &str) -> Sched {
    let solo = best_response(d, u, &vec![1.0; d.nr], None, None).expect("infeasible goals").0;
    if mode != "goals" { return solo; }
    let mut s = solo; let mut mask = 0u32;
    for t in 0..NT {
        if let Choice::Opt(o) = s[t] {
            let b = u.bits[t][o as usize];
            if u.forced[t].is_some() || b & !mask != 0 { mask |= b; } else { s[t] = Choice::Rest; }
        }
    }
    if feasible(u, &s) { s } else { solo }
}
fn d_one() -> usize { 1 }
fn kmax_top(dj: &DeckJob) -> usize { dj.top_decks.max(1) }
fn d_lambda() -> f64 { 7.1 }
fn d_size() -> usize { 6 }
fn d_rar() -> u8 { 2 }   // SR: R cards have no chain events, so this only filters chain cards

/// Chain kind of a deck entry: (chain length, link the chain stops at). A card played normally stops at its last
/// link; a card whose chain the player ends early (a choice that ends the chain) stops sooner. While it runs, an
/// entry is picked with weight = links it has left (length - done); once it reaches its stop it is out.
type Kind = (u8, u8);

/// P(link k of an entry fires), k = 1..=stop, for every chain kind present, given the deck's chain kinds.
fn chain_marginals(kinds: &[Kind], lambda: f64) -> HashMap<Kind, Vec<f64>> {
    let n = kinds.len();
    let mut out: HashMap<Kind, Vec<f64>> = HashMap::new();
    if n == 0 { return out; }
    let emax = 60usize;
    let mut pois = vec![0.0f64; emax + 1];
    pois[0] = (-lambda).exp();
    for e in 1..=emax { pois[e] = pois[e - 1] * lambda / e as f64; }
    let total: usize = kinds.iter().map(|&(_, stop)| stop as usize).sum();
    let width = kinds.iter().map(|k| k.1 as usize).max().unwrap_or(0).max(1);
    let mut fired = vec![vec![0.0f64; width]; n];      // P(entry i has >= k+1 links done), accumulated over E
    let mut st: HashMap<Vec<u8>, f64> = HashMap::new();
    st.insert(vec![0; n], 1.0);
    for e in 1..=emax.min(total) {
        let mut ns: HashMap<Vec<u8>, f64> = HashMap::new();
        for (s, p) in &st {
            let w: Vec<f64> = (0..n).map(|i| if s[i] < kinds[i].1 { (kinds[i].0 - s[i]) as f64 } else { 0.0 }).collect();
            let wsum: f64 = w.iter().sum();
            if wsum == 0.0 { *ns.entry(s.clone()).or_insert(0.0) += p; continue; }
            for i in 0..n { if w[i] > 0.0 { let mut t = s.clone(); t[i] += 1; *ns.entry(t).or_insert(0.0) += p * w[i] / wsum; } }
        }
        st = ns;
        let mut prev_done = vec![vec![0.0f64; width]; n];
        for (s, p) in &st { for i in 0..n { for k in 0..s[i] as usize { prev_done[i][k] += p; } } }
        // P(link done) = sum_e P(E = e) * P(done after e events); accumulate with P(E = e)
        let pe = pois[e] + if e == emax.min(total) { 1.0 - pois[..=e].iter().sum::<f64>() } else { 0.0 };
        for i in 0..n { for k in 0..width { fired[i][k] += pe * prev_done[i][k]; } }
    }
    for i in 0..n {
        let kd = kinds[i];
        out.entry(kd).or_insert_with(|| (0..kd.1 as usize).map(|k| fired[i][k]).collect());
    }
    out
}

/// Target sources of one deck entry (a card, or a card whose chain the player ends early): training hints
/// (target, chance), chain rewards grouped per (target, level) as (link, chance given the link fires) sorted by
/// link, and rewards that do not depend on the chain (target, level, chance).
#[derive(Clone, Default)]
struct Src { train: Vec<(usize, f64)>, chains: Vec<(usize, u8, Vec<(u8, f64)>)>, fixed: Vec<(usize, u8, f64)> }

/// Multiply an entry's miss chances into mw (white) / mg (gold); m[l - 1] = P(chain link l fires).
/// Links fire in order, so a skill offered at several links comes from the earliest one that fires:
/// P(miss) = 1 - sum_l P(link l fires) * (P(not from links < l) - P(not from links <= l)).
/// It falls with every m[l], so the lowest possible link odds bound the miss chance from above.
fn src_miss(s: &Src, m: &[f64], mw: &mut [f64], mg: &mut [f64]) {
    for &(k, p) in &s.train { mw[k] *= 1.0 - p; }
    for &(k, lvl, p) in &s.fixed { if lvl >= 2 { mg[k] *= 1.0 - p; } else { mw[k] *= 1.0 - p; } }
    for (k, lvl, evs) in &s.chains {
        let (mut cum, mut miss, mut i) = (1.0f64, 1.0f64, 0usize);
        while i < evs.len() {
            let l = evs[i].0; let mut nc = cum;
            while i < evs.len() && evs[i].0 == l { nc *= 1.0 - evs[i].1; i += 1; }
            let ml = m.get(l as usize - 1).copied().unwrap_or(0.0);
            miss -= ml * (cum - nc); cum = nc;
        }
        if *lvl >= 2 { mg[*k] *= miss; } else { mw[*k] *= miss; }
    }
}

/// Merge a card-event cache (card_events.json) into a deck job: skill names -> (group, level) via the skills table.
pub(crate) fn apply_events_db(v: &mut Value, sk: &SkillsRaw, db: &Value) {
    let look = |name: &str| sk.skills.iter().find_map(|(g, w, gd)| if w == name { Some((*g, 1u8)) } else if gd.as_deref() == Some(name) { Some((*g, 2u8)) } else { None });
    let mut ev: Vec<Value> = v["events"].as_array().cloned().unwrap_or_default();
    let mut evp: Vec<Value> = v["events_p"].as_array().cloned().unwrap_or_default();
    let mut od: Vec<Value> = v["odds_events"].as_array().cloned().unwrap_or_default();
    let mut el: Vec<Value> = v["end_links"].as_array().cloned().unwrap_or_default();
    let mut en: Vec<Value> = v["ends"].as_array().cloned().unwrap_or_default();
    if let Some(cards) = db["cards"].as_object() {
        for (id, c) in cards {
            let Ok(id) = id.parse::<u32>() else { continue };
            for link in c["chain"].as_array().into_iter().flatten() {
                let l = link[0].as_u64().unwrap_or(0);
                for n in link[1].as_array().into_iter().flatten() {
                    let (name, p) = if let Some(a) = n.as_array() { (a[0].as_str().unwrap_or(""), a[1].as_f64().unwrap_or(0.0)) } else { (n.as_str().unwrap_or(""), 1.0) };
                    if let Some((g, lv)) = look(name) { if p >= 1.0 { ev.push(json!([id, g, lv, l])); } else { evp.push(json!([id, g, lv, l, p])); } }
                }
            }
            for link in c["end"].as_array().into_iter().flatten() {
                let l = link[0].as_u64().unwrap_or(0);
                el.push(json!([id, l]));
                for n in link[1].as_array().into_iter().flatten() {
                    let (name, p) = if let Some(a) = n.as_array() { (a[0].as_str().unwrap_or(""), a[1].as_f64().unwrap_or(0.0)) } else { (n.as_str().unwrap_or(""), 1.0) };
                    if let Some((g, lv)) = look(name) { en.push(json!([id, l, g, lv, p])); }
                }
            }
            for n in c["always"].as_array().into_iter().flatten() { if let Some((g, lv)) = look(n.as_str().unwrap_or("")) { ev.push(json!([id, g, lv, 0])); } }
            for e in c["chance"].as_array().into_iter().flatten() { if let Some((g, lv)) = look(e[0].as_str().unwrap_or("")) { od.push(json!([id, g, lv, e[1].as_f64().unwrap_or(0.0)])); } }
        }
    }
    v["events"] = Value::Array(ev); v["events_p"] = Value::Array(evp); v["odds_events"] = Value::Array(od);
    v["end_links"] = Value::Array(el); v["ends"] = Value::Array(en);
}

/// Slack for float noise in dominance tests: per coordinate, and summed over all coordinates.
const DOM_EPS: f64 = 1e-12;
const SUM_EPS: f64 = 1e-9;

#[inline]
fn dominates(by: &[f64], x: &[f64]) -> bool { by.iter().zip(x).all(|(b, a)| *b >= *a - DOM_EPS) }

/// Coarse signature for a fast necessary test of dominance: the first SIG_FIELDS coordinates as levels
/// floor(LEVELS * v) (capped at LEVELS - 1), each in a field of FB bits whose top bit is a guard. `entry`
/// signatures use the value itself, `query` ones the value less DOM_EPS, so `by` dominating `x` implies every
/// field of entry(by) >= the matching field of query(x). A 64-bit word holds 16 fields of 8 levels, enough for
/// 8 targets x (gold, any hint), and is cheap in WebAssembly as well as natively.
type Sig = u64;
const FB: usize = 4;
const SIG_FIELDS: usize = 64 / FB;
const LEVELS: Sig = 1 << (FB - 1);
const SIG_GUARD: Sig = {
    let mut g: Sig = 0; let mut c = 0;
    while c < SIG_FIELDS { g |= (1 as Sig) << (FB * c + FB - 1); c += 1; }
    g
};
fn sig_of(v: &[f64], query: bool) -> Sig {
    let mut s: Sig = 0;
    for (c, &x) in v.iter().take(SIG_FIELDS).enumerate() {
        let y = if query { x - DOM_EPS } else { x };
        let l = (((y * LEVELS as f64).floor().max(0.0) as u32) as Sig).min(LEVELS - 1);   // via u32: wasm2js lacks the 64-bit float conversion
        s |= l << (FB * c);
    }
    s
}
/// every field of `e` >= the matching field of `q` (SWAR: guard bits survive the subtraction)
#[inline]
fn sig_ge(e: Sig, q: Sig) -> bool { ((e | SIG_GUARD) - q) & SIG_GUARD == SIG_GUARD }

/// Field-wise max / min of two signatures (levels only, guard bits clear).
fn sig_max(a: Sig, b: Sig) -> Sig {
    let mut m: Sig = 0;
    for c in 0..SIG_FIELDS { let sh = FB * c; m |= ((a >> sh) & (LEVELS - 1)).max((b >> sh) & (LEVELS - 1)) << sh; }
    m
}
fn sig_min(a: Sig, b: Sig) -> Sig {
    let mut m: Sig = 0;
    for c in 0..SIG_FIELDS { let sh = FB * c; m |= ((a >> sh) & (LEVELS - 1)).min((b >> sh) & (LEVELS - 1)) << sh; }
    m
}

const BLOCK: usize = 32;
const NBUCKET: usize = 128;

/// Decks whose coordinate sums fall in one slice of [0, nd]: flat arrays (entry and query signatures,
/// coordinates, ids) plus, per block of BLOCK decks, the field-wise max entry signature, so a whole block that
/// cannot hold a dominator is skipped with one test.
#[derive(Default)]
struct Bucket { list: Vec<DeckEval>, esig: Vec<Sig>, qsig: Vec<Sig>, flat: Vec<f64>, uid: Vec<u64>, bmax: Vec<Sig>, bmin: Vec<Sig> }

/// Kept decks of one pal count. A dominator's coordinate sum is at least the deck's (less float slack), so a
/// query only visits buckets at or above the deck's sum.
/// `recent`: coordinates of the last RECENT decks that dominated a new deck.
///   top_decks = 1: a deck on it may since have been dropped, but then a kept deck dominates it, so it still
///   proves that any deck it dominates is not needed.
///   top_decks > 1: kept decks are only dropped at compaction, which starts a fresh `Kept`, so every deck on it
///   is still kept; `ruid` holds their ids so each counts once.
struct Kept {
    nd: usize, bw: f64, buckets: Vec<Bucket>, n: usize, next_uid: u64,
    lastrow: Vec<f64>, has_last: bool,
    cap: usize, recent: Vec<f64>, rpos: usize, rlen: usize, ruid: Vec<u64>, found: Vec<(usize, usize)>,
}
const RECENT: usize = 32;
impl Kept {
    fn new(nd: usize) -> Self {
        Kept { nd, bw: (nd.max(1) as f64) / NBUCKET as f64, buckets: (0..NBUCKET).map(|_| Bucket::default()).collect(), n: 0, next_uid: 0,
               lastrow: vec![0.0; nd], has_last: false, cap: 512, recent: vec![0.0; RECENT * nd], rpos: 0, rlen: 0,
               ruid: vec![u64::MAX; RECENT], found: vec![] }
    }
    fn len(&self) -> usize { self.n }
    #[inline] fn bucket_of(&self, s: f64) -> usize { ((s / self.bw).floor().max(0.0) as usize).min(NBUCKET - 1) }
    /// number of kept decks dominating x (stops at `need`)
    fn count(&mut self, x: &[f64], xq: Sig, s: f64, need: usize, tests: &mut usize) -> usize {
        let nd = self.nd;
        let b0 = self.bucket_of(s - SUM_EPS);
        let mut n = 0;
        self.found.clear();
        for bi in (b0..NBUCKET).rev() {
            let bk = &self.buckets[bi];
            for (blk, &m) in bk.bmax.iter().enumerate() {
                if !sig_ge(m, xq) { continue; }
                let end = ((blk + 1) * BLOCK).min(bk.list.len());
                for i in blk * BLOCK..end {
                    if !sig_ge(bk.esig[i], xq) { continue; }
                    *tests += 1;
                    let row = &bk.flat[i * nd..(i + 1) * nd];
                    if dominates(row, x) {
                        n += 1; self.found.push((bi, i));
                        if n >= need {
                            self.lastrow.copy_from_slice(row); self.has_last = true;
                            return n;
                        }
                    }
                }
            }
        }
        n
    }
    /// like `count`, without touching the remembered-dominator state (for a fixed reference set)
    fn count_ro(&self, x: &[f64], xq: Sig, s: f64, need: usize) -> usize {
        let nd = self.nd;
        let b0 = self.bucket_of(s - SUM_EPS);
        let mut n = 0;
        for bi in (b0..NBUCKET).rev() {
            let bk = &self.buckets[bi];
            for (blk, &m) in bk.bmax.iter().enumerate() {
                if !sig_ge(m, xq) { continue; }
                let end = ((blk + 1) * BLOCK).min(bk.list.len());
                for i in blk * BLOCK..end {
                    if sig_ge(bk.esig[i], xq) && dominates(&bk.flat[i * nd..(i + 1) * nd], x) { n += 1; if n >= need { return n; } }
                }
            }
        }
        n
    }
    fn count_recent(&self, x: &[f64]) -> usize {
        let nd = self.nd;
        (0..self.rlen).filter(|&r| dominates(&self.recent[r * nd..(r + 1) * nd], x)).count()
    }
    /// put the dominators the last count found on `recent`
    fn remember_found(&mut self) {
        let nd = self.nd;
        for (bi, i) in std::mem::take(&mut self.found) {
            let id = self.buckets[bi].uid[i];
            if self.ruid[..self.rlen].contains(&id) { continue; }
            let r = self.rpos;
            let (rec, flat) = (&mut self.recent, &self.buckets[bi].flat);
            rec[r * nd..(r + 1) * nd].copy_from_slice(&flat[i * nd..(i + 1) * nd]);
            self.ruid[r] = id;
            self.rpos = (r + 1) % RECENT; self.rlen = (self.rlen + 1).min(RECENT);
        }
    }
    fn insert(&mut self, x: DeckEval) {
        let bi = self.bucket_of(x.s);
        let (e, q) = (sig_of(&x.dv, false), sig_of(&x.dv, true));
        let id = self.next_uid; self.next_uid += 1;
        let bk = &mut self.buckets[bi];
        let i = bk.list.len();
        if i % BLOCK == 0 { bk.bmax.push(e); bk.bmin.push(q); } else { let l = bk.bmax.len() - 1; bk.bmax[l] = sig_max(bk.bmax[l], e); bk.bmin[l] = sig_min(bk.bmin[l], q); }
        bk.esig.push(e); bk.qsig.push(q); bk.flat.extend_from_slice(&x.dv); bk.uid.push(id); bk.list.push(x);
        self.n += 1;
    }
    /// drop kept decks that x dominates (only decks with sum <= s + slack can be)
    fn remove_dominated_by(&mut self, x: &[f64], xe: Sig, s: f64) {
        let nd = self.nd;
        let top = self.bucket_of(s + SUM_EPS);
        for bi in 0..=top {
            let hit = |bk: &Bucket, i: usize| sig_ge(xe, bk.qsig[i]) && dominates(x, &bk.flat[i * nd..(i + 1) * nd]);
            let bk = &self.buckets[bi];
            if !(0..bk.list.len()).any(|i| hit(bk, i)) { continue; }
            let old = std::mem::take(&mut self.buckets[bi]);
            let keep: Vec<bool> = (0..old.list.len()).map(|i| !hit(&old, i)).collect();
            let mut nb = Bucket::default();
            for (i, d) in old.list.into_iter().enumerate() {
                if !keep[i] { self.n -= 1; continue; }
                let e = old.esig[i];
                let j = nb.list.len();
                let q = old.qsig[i];
                if j % BLOCK == 0 { nb.bmax.push(e); nb.bmin.push(q); } else { let l = nb.bmax.len() - 1; nb.bmax[l] = sig_max(nb.bmax[l], e); nb.bmin[l] = sig_min(nb.bmin[l], q); }
                nb.esig.push(e); nb.qsig.push(q); nb.flat.extend_from_slice(&old.flat[i * nd..(i + 1) * nd]); nb.uid.push(old.uid[i]); nb.list.push(d);
            }
            self.buckets[bi] = nb;
        }
    }
    fn into_list(self) -> Vec<DeckEval> { self.buckets.into_iter().flat_map(|b| b.list).collect() }
}

/// Keep each deck unless `kmax` decks scoring at least as high in every coordinate are kept before it (sorted
/// pass). A dominator's coordinate sum is at least the deck's, so only kept decks above that sum are checked.
fn k_filter(mut g: Vec<DeckEval>, kmax: usize, nd: usize) -> Kept {
    g.sort_by(|a, b| b.s.partial_cmp(&a.s).unwrap());
    let mut kept = Kept::new(nd);
    let mut tests = 0usize;
    for x in g {
        let xq = sig_of(&x.dv, true);
        if kept.count(&x.dv, xq, x.s, kmax, &mut tests) < kmax { kept.insert(x); }
    }
    kept.has_last = false;
    kept
}

/// One candidate deck. `dw`/`dg` are the per-target white/gold hint odds the metric uses; `dv` are the
/// coordinates used for dominance (see `run_deck`), all higher-is-better, and `s` is their sum.
#[derive(Clone)]
struct DeckEval { pals: usize, borrowed: Option<usize>, cards: Vec<usize>, ends: Vec<u8>, plain: usize, filler: usize, dw: Vec<f64>, dg: Vec<f64>, dv: Vec<f64>, s: f64 }

pub(crate) fn run_deck(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    let sj: SkillJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    let dj: DeckJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    let model = Model::new(d, sk, &sj);
    let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
    let nu = unc.len();
    let is_owned = |id: u32| dj.owned_cards.is_empty() || dj.owned_cards.contains(&id) || dj.lock_cards.contains(&id);
    let owned_c = |id: u32| (is_owned(id) || dj.borrow) && !dj.exclude_cards.contains(&id);

    // ---- card sources ----
    let ev_for = |id: u32| -> Vec<(usize, u8, u8, f64)> {
        let mut v: Vec<(usize, u8, u8, f64)> = dj.events.iter().filter(|e| e.0 == id).filter_map(|e| unc.iter().position(|&g| g == e.1).map(|k| (k, e.2, e.3, 1.0))).collect();
        v.extend(dj.events_p.iter().filter(|e| e.0 == id).filter_map(|e| unc.iter().position(|&g| g == e.1).map(|k| (k, e.2, e.3, e.4))));
        v };
    let odds_for = |id: u32| -> Vec<(usize, u8, f64)> { dj.odds_events.iter().filter(|e| e.0 == id).filter_map(|e| unc.iter().position(|&g| g == e.1).map(|k| (k, e.2, e.3))).collect() };
    let train = |c: &Card| -> Vec<(usize, f64)> {
        if c.opts == 0 { return vec![]; }
        let h = 1.0 + c.hf / 25.0;
        let p = 1.0 - (1.0 - 1.0 / c.opts as f64).powf(h);
        unc.iter().enumerate().filter(|(_, g)| c.whites.contains(g)).map(|(k, _)| (k, p)).collect()
    };
    let cards: Vec<&Card> = cr.cards.iter().filter(|c| owned_c(c.id)).collect();
    // early-end options: links where a card has a choice that ends its chain, and the target rewards of that choice
    let end_links_for = |id: u32| -> Vec<u8> { let mut v: Vec<u8> = dj.end_links.iter().filter(|e| e.0 == id).map(|e| e.1).collect(); v.sort(); v.dedup(); v };
    let end_rew = |id: u32, l: u8| -> Vec<(usize, u8, f64)> { dj.ends.iter().filter(|e| e.0 == id && e.1 == l).filter_map(|e| unc.iter().position(|&g| g == e.2).map(|k| (k, e.3, e.4))).collect() };
    // sources of a card played normally (stop = None) or ending its chain at link `stop` (links before it as usual,
    // then the ending choice's rewards at that link)
    let make_src = |c: &Card, stop: Option<u8>| -> Src {
        let mut s = Src { train: train(c), chains: vec![], fixed: odds_for(c.id) };
        let mut ch: Vec<(usize, u8, u8, f64)> = vec![];
        for (k, lvl, link, f) in ev_for(c.id) {
            if link == 0 { s.fixed.push((k, lvl, f)); continue; }
            if c.pal { continue; }                       // pals have no chain slots
            match stop { Some(st) if link >= st => {}, _ => ch.push((k, lvl, link, f)) }
        }
        if let Some(st) = stop { for (k, lvl, f) in end_rew(c.id, st) { ch.push((k, lvl, st, f)); } }
        ch.sort_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));
        for (k, lvl, link, f) in ch {
            match s.chains.last_mut() { Some(g) if g.0 == k && g.1 == lvl => g.2.push((link, f)), _ => s.chains.push((k, lvl, vec![(link, f)])) }
        }
        s
    };
    let has_src = |s: &Src| !s.train.is_empty() || !s.chains.is_empty() || !s.fixed.is_empty();
    // deck entries: card index (`rel`), chain kind, early-end link (0 = played normally) and sources
    let mut rel: Vec<usize> = vec![];
    let mut kind: Vec<Kind> = vec![];
    let mut endk: Vec<u8> = vec![];
    let mut srcs: Vec<Src> = vec![];
    let mut plain_names: Vec<String> = vec![];
    let mut plain_chars: Vec<u32> = vec![];
    for (i, c) in cards.iter().enumerate() {
        let useful = !ev_for(c.id).is_empty() || !odds_for(c.id).is_empty() || !train(c).is_empty();
        let locked = dj.lock_cards.contains(&c.id);
        if c.pal {
            if useful || locked { rel.push(i); kind.push((0, 0)); endk.push(0); srcs.push(make_src(c, None)); }
        } else if c.links == 0 {
            // a card without chain events (R cards): training hints only, and like a pal it takes no chain slot
            if useful || locked { rel.push(i); kind.push((0, 0)); endk.push(0); srcs.push(make_src(c, None)); }
        } else if c.rarity >= dj.min_rarity {
            if useful || locked { rel.push(i); kind.push((c.links, c.links)); endk.push(0); srcs.push(make_src(c, None)); }
            // the same card played to end its chain at link l, as its own entry (one character, so a deck holds
            // at most one of a card's entries)
            for l in end_links_for(c.id) {
                if l == 0 || l > c.links { continue; }
                let v = make_src(c, Some(l));
                if has_src(&v) || locked { rel.push(i); kind.push((c.links, l)); endk.push(l); srcs.push(v); }
            }
        }
    }
    // plain pals: owned pal characters with no relevant card in `rel` (best-rarity card name for display).
    // With borrowing, a pal character the player owns no card of can still fill one slot as the borrowed card.
    for c in cards.iter().filter(|c| c.pal && is_owned(c.id)) {
        if rel.iter().any(|&r| cards[r].chara == c.chara) || plain_chars.contains(&c.chara) { continue; }
        plain_chars.push(c.chara);
        let best = cards.iter().filter(|x| x.pal && x.chara == c.chara && is_owned(x.id)).max_by_key(|x| x.rarity).unwrap();
        plain_names.push(best.name.clone());
    }
    if dj.borrow {
        let mut seen: Vec<u32> = vec![];
        for c in cards.iter() {
            if !c.pal || is_owned(c.id) || plain_chars.contains(&c.chara) || seen.contains(&c.chara) || rel.iter().any(|&r| cards[r].chara == c.chara) { continue; }
            let best = cards.iter().enumerate().filter(|(_, x)| x.pal && x.chara == c.chara).max_by_key(|(_, x)| x.rarity).unwrap().0;
            seen.push(c.chara); rel.push(best); kind.push((0, 0)); endk.push(0); srcs.push(make_src(cards[best], None));
        }
    }
    let mut locked: Vec<usize> = rel.iter().copied().filter(|&r| dj.lock_cards.contains(&cards[r].id)).collect();
    locked.sort(); locked.dedup();

    // ---- card dominance (exact) ----
    // Card A can stand in for card B in any deck when both are pals or both have the same chain length (so the
    // deck's chain odds do not change), A is owned whenever B is, and A at its weakest still beats B at its
    // strongest on every target: A's miss chances with its chain links at their lowest possible odds (the most
    // crowded deck) are no higher than B's with every link firing, for both "no gold hint" and "no hint at all".
    // Swapping B for A then gives a deck at least as good on every dominance coordinate, with the same pal
    // count. The swap is blocked only if A is already in the deck or shares a character with one of the other
    // deck_size - 1 cards, so B is dropped when kept dominators with at least deck_size - 1 + top_decks
    // distinct characters exist: every deck with B then has top_decks distinct kept decks at least as good
    // (swap the deck's other dropped cards first, then B for each unblocked dominator).
    let mut n_card_dropped = 0usize;
    // pals and R cards both take no chain slot, so they can stand in for each other unless the job fixes the pal
    // count
    let pal_fixed = dj.pals.is_some();
    // per kept card, the kept cards that beat it whatever the deck (same test as above), for subtree pruning
    let mut robust: Vec<Vec<usize>> = vec![];
    {
        let nrel0 = rel.len();
        let mut kindset: Vec<Kind> = (0..nrel0).filter(|&j| kind[j] != (0, 0)).map(|j| kind[j]).collect();
        kindset.push((2, 2)); kindset.sort(); kindset.dedup();
        let others = dj.deck_size.saturating_sub(1);
        fn multisets(vals: &[Kind], start: usize, left: usize, cur: &mut Vec<Kind>, out: &mut dyn FnMut(&[Kind])) {
            out(cur);
            if left == 0 { return; }
            for i in start..vals.len() { cur.push(vals[i]); multisets(vals, i, left - 1, cur, out); cur.pop(); }
        }
        // lowest odds of each link over every possible set of other chain cards
        let needs_marg = |kd: Kind| (0..nrel0).any(|j| kind[j] != (0, 0) && kind[j] == kd && !srcs[j].chains.is_empty());
        let mut min_marg: HashMap<Kind, Vec<f64>> = HashMap::new();
        for &kd in &kindset {
            if !needs_marg(kd) { continue; }
            let mut mins = vec![1.0f64; kd.1 as usize];
            multisets(&kindset, 0, others, &mut vec![], &mut |o| {
                let mut key = vec![kd]; key.extend_from_slice(o);
                let m = chain_marginals(&key, dj.lambda);
                for k in 0..kd.1 as usize { mins[k] = mins[k].min(m[&kd][k]); }
            });
            min_marg.insert(kd, mins);
        }
        // per card: (miss gold, miss any) per target, at its strongest (lo) and weakest (hi)
        let miss = |j: usize, strongest: bool| -> (Vec<f64>, Vec<f64>) {
            let stop = kind[j].1 as usize;
            let m: Vec<f64> = if strongest { vec![1.0; stop] } else { min_marg.get(&kind[j]).cloned().unwrap_or(vec![0.0; stop]) };
            let mut mw = vec![1.0f64; nu]; let mut mg = vec![1.0f64; nu];
            src_miss(&srcs[j], &m, &mut mw, &mut mg);
            let mh: Vec<f64> = (0..nu).map(|k| mg[k] * mw[k]).collect();
            (mg, mh)
        };
        let lo: Vec<(Vec<f64>, Vec<f64>)> = (0..nrel0).map(|j| miss(j, true)).collect();
        let hi: Vec<(Vec<f64>, Vec<f64>)> = (0..nrel0).map(|j| miss(j, false)).collect();
        let strength = |j: usize| (0..nu).map(|k| 2.0 - lo[j].0[k] - lo[j].1[k]).sum::<f64>();
        let mut order: Vec<usize> = (0..nrel0).collect();
        order.sort_by(|&a, &b| strength(b).partial_cmp(&strength(a)).unwrap().then(a.cmp(&b)));
        let need = dj.deck_size.saturating_sub(1) + kmax_top(&dj);
        let mut keep = vec![true; nrel0];
        let mut done: Vec<usize> = vec![];
        for &b in &order {
            let cb = cards[rel[b]];
            if !locked.contains(&rel[b]) {
                let mut charas: Vec<u32> = vec![];
                for &a in &done {
                    if !keep[a] { continue; }
                    let ca = cards[rel[a]];
                    if kind[a] != kind[b] || (pal_fixed && ca.pal != cb.pal) { continue; }
                    if is_owned(cb.id) && !is_owned(ca.id) { continue; }
                    if !(0..nu).all(|k| hi[a].0[k] <= lo[b].0[k] && hi[a].1[k] <= lo[b].1[k]) { continue; }
                    if !charas.contains(&ca.chara) { charas.push(ca.chara); }
                    if charas.len() >= need { break; }
                }
                if charas.len() >= need { keep[b] = false; n_card_dropped += 1; }
            }
            done.push(b);
        }
        if std::env::var("DECK_DEBUG").is_ok() {
            for j in 0..nrel0 { if !keep[j] { eprintln!("  drop {}", cards[rel[j]].name); } }
        }
        let kept_old: Vec<usize> = (0..nrel0).filter(|&j| keep[j]).collect();
        robust = kept_old.iter().map(|&b| {
            let cb = cards[rel[b]];
            kept_old.iter().enumerate().filter(|&(_, &a)| {
                if a == b { return false; }
                let ca = cards[rel[a]];
                if kind[a] != kind[b] || (pal_fixed && ca.pal != cb.pal) { return false; }
                let _ = (ca, cb);
                let le = (0..nu).all(|k| hi[a].0[k] <= lo[b].0[k] && hi[a].1[k] <= lo[b].1[k]);
                let eq = (0..nu).all(|k| hi[a].0[k] == lo[b].0[k] && hi[a].1[k] == lo[b].1[k]);
                le && (!eq || a < b)
            }).map(|(ni, _)| ni).collect()
        }).collect();
        if std::env::var("DECK_DEBUG_PAIRS").is_ok() {
            for (bi, &b) in kept_old.iter().enumerate() {
                let names: Vec<String> = robust[bi].iter().map(|&a| cards[rel[kept_old[a]]].name.clone()).collect();
                if !names.is_empty() { eprintln!("ROBUST {} <= {:?}", cards[rel[b]].name, names); }
            }
        }
        let mut j = 0;
        rel.retain(|_| { let k = keep[j]; j += 1; k });
        let mut j = 0;
        srcs.retain(|_| { let k = keep[j]; j += 1; k });
        let mut j = 0;
        kind.retain(|_| { let k = keep[j]; j += 1; k });
        let mut j = 0;
        endk.retain(|_| { let k = keep[j]; j += 1; k });
    }

    // ---- dominance coordinates ----
    // For a target the trainee's outfit does not give, the metric's landing chance is
    //   p = 0.35 + 0.35*g - 0.35*(1 - q)*(1 - h),   g = P(gold hint), h = P(any hint) = 1 - (1 - w)*(1 - g);
    // with the white on the outfit p = 0.35 + 0.35*g, and with the gold it is constant. So the metric only
    // rises with g and h, and a deck at least as high in every (g, h) is never worse. (w, g) dominance implies
    // (g, h) dominance, so this prunes at least as much. Coordinates no relevant card can move are dropped,
    // and h == g when no card gives the white.
    let mut has_w = vec![false; nu]; let mut has_g = vec![false; nu];
    for sr in &srcs {
        for &(k, _) in &sr.train { has_w[k] = true; }
        for (k, lvl, _) in &sr.chains { if *lvl >= 2 { has_g[*k] = true; } else { has_w[*k] = true; } }
        for &(k, lvl, _) in &sr.fixed { if lvl >= 2 { has_g[k] = true; } else { has_w[k] = true; } }
    }
    let coords: Vec<(usize, bool)> = (0..nu).flat_map(|k| {
        let mut v = vec![];
        if has_g[k] { v.push((k, true)); }
        if has_w[k] { v.push((k, false)); }
        v
    }).collect();

    // ---- enumerate decks ----
    let allowed_borrow = if dj.borrow { 1usize } else { 0 };
    let mut n_swapped = 0usize;
    let nrel = rel.len();
    // characters as bits (one deck holds one entry per character)
    let mut chara_ix: HashMap<u32, usize> = HashMap::new();
    for &r in &rel { let n = chara_ix.len(); chara_ix.entry(cards[r].chara).or_insert(n); }
    if chara_ix.len() > 128 { return json!({"error": "too many distinct characters among relevant cards"}); }
    let ebit: Vec<u128> = rel.iter().map(|&r| 1u128 << chara_ix[&cards[r].chara]).collect();
    let ebor: Vec<bool> = rel.iter().map(|&r| !is_owned(cards[r].id)).collect();
    // masks of the deck-independent dominators (`robust`), split by whether the dominator is owned
    // Chain-kind key of a deck = multiset of its chain entries' kinds plus fillers (2, 2), coded as counts in base 7.
    let mut kinds_all: Vec<Kind> = (0..nrel).filter(|&j| kind[j] != (0, 0)).map(|j| kind[j]).collect();
    kinds_all.push((2, 2)); kinds_all.sort(); kinds_all.dedup();
    let kpos: Vec<u64> = (0..kinds_all.len()).map(|i| 7u64.pow(i as u32)).collect();
    // Per key: each entry's miss chances (white, gold) under the key's chain odds, and for each entry B the
    // entries that beat it on every target under those odds (same kind, so swapping keeps the odds; per target no
    // higher chance of missing the gold and no higher chance of missing every hint; exact ties go to the lower
    // index), as owned / borrowed character masks.
    struct KeyData { mw: Vec<f64>, mg: Vec<f64>, d_own: Vec<u128>, d_bor: Vec<u128> }
    let mut key_cache: HashMap<u64, KeyData> = HashMap::new();
    let make_key = |code: u64| -> KeyData {
        let mut ks: Vec<Kind> = vec![]; let mut c = code;
        for &kd in &kinds_all { for _ in 0..(c % 7) { ks.push(kd); } c /= 7; }
        let marg = chain_marginals(&ks, dj.lambda);
        let empty: Vec<f64> = vec![];
        let mut mw = vec![1.0f64; nrel * nu]; let mut mg = vec![1.0f64; nrel * nu];
        for j in 0..nrel { src_miss(&srcs[j], marg.get(&kind[j]).unwrap_or(&empty), &mut mw[j * nu..(j + 1) * nu], &mut mg[j * nu..(j + 1) * nu]); }
        let mh: Vec<f64> = (0..nrel * nu).map(|i| mg[i] * mw[i]).collect();
        let mut d_own = vec![0u128; nrel]; let mut d_bor = vec![0u128; nrel];
        for b in 0..nrel { for a in 0..nrel {
            // same kind keeps the chain odds; with a fixed pal count (`pals`), same pal status keeps it
            if a == b || kind[a] != kind[b] || (pal_fixed && cards[rel[a]].pal != cards[rel[b]].pal) { continue; }
            let (ra, rb) = (a * nu..(a + 1) * nu, b * nu..(b + 1) * nu);
            let le = mg[ra.clone()].iter().zip(&mg[rb.clone()]).all(|(x, y)| x <= y) && mh[ra.clone()].iter().zip(&mh[rb.clone()]).all(|(x, y)| x <= y);
            if !le { continue; }
            let eq = mg[ra.clone()] == mg[rb.clone()] && mh[ra] == mh[rb];
            if eq && a > b { continue; }
            if ebor[a] { d_bor[b] |= ebit[a]; } else { d_own[b] |= ebit[a]; }
        } }
        KeyData { mw, mg, d_own, d_bor }
    };
    let mut n_decks = 0usize;
    let size = dj.deck_size;
    let kmax = dj.top_decks.max(1);
    let ident = |cards: &[usize], ends: &[u8], plain: usize, filler: usize| -> Vec<u64> {
        let mut v: Vec<u64> = cards.iter().zip(ends).map(|(&c, &e)| ((c as u64) << 8) | e as u64).collect();
        v.sort(); v.push((1 << 40) + plain as u64); v.push((2 << 40) + filler as u64); v
    };
    let mut st_s1 = 0usize;
    assert!(nu <= 32, "too many uncertain targets");
    let is_locked: Vec<bool> = (0..nrel).map(|j| locked.contains(&rel[j])).collect();
    // Decks are enumerated one chain-kind key at a time (the key fixes every entry's chain odds), so each deck is
    // generated once, under its own key, with that key's exact dominance relation:
    //  - Swap rule. If card A outside the deck beats deck card B on every target under the key's odds and can
    //    legally replace it (same kind, so the key is unchanged; not a second entry of a character already in
    //    the deck; within the borrow limit), the swapped deck is generated too and is at least as good. A deck
    //    with top_decks such swaps is not needed: by induction over the swap order (ties go to the lower index,
    //    so it has no cycles), each swapped deck is kept or itself has top_decks kept decks at least as good.
    //    Distinct (B, A) pairs give distinct decks; counting A's distinct characters undercounts pairs, which
    //    only skips fewer decks.
    //  - Partial decks. While a deck is built, each entry still to be added blocks at most one more character,
    //    so a partial deck where one card B already has (entries still to add) + top_decks unblocked legal
    //    swaps can only grow into decks the swap rule skips; the whole branch is dropped.
    //  - Dead entries. The same with nothing chosen yet: an entry whose legal dominators under the key span
    //    deck_size - 1 + top_decks characters never appears.
    // Owned cards only count borrowed dominators when the deck has room for a borrowed card.
    let nk = kinds_all.len();
    let ki22 = kinds_all.iter().position(|&k| k == (2, 2)).unwrap();
    let by_kind: Vec<Vec<usize>> = (0..nk).map(|i| (0..nrel).filter(|&j| kind[j] != (0, 0) && kind[j] == kinds_all[i]).collect()).collect();
    // entries without chain slots: pals, group cards and R cards
    let pal_ents: Vec<usize> = (0..nrel).filter(|&j| kind[j] == (0, 0)).collect();
    let mut n_subtrees = 0usize;
    let mut dv = vec![0.0f64; coords.len()];
    let (mut st_tests, mut st_full, mut st_dom, mut st_ring) = (0usize, 0usize, 0usize, 0usize);
    // Pick `cnt` entries from each group in turn (one entry per character, at most `allowed` borrowed).
    // Passed-over dominators: when entry j is added at position p of its group, a dominator of j whose character
    // is not in the deck and belongs to no entry that can still be added (`later[gi][p]`) will never be in the
    // finished deck, so it is a legal swap for j there. `acc` counts such guaranteed swaps along the branch; at
    // top_decks the branch is cut (the swap rule would skip every deck in it).
    struct Ctx<'a> { gs: &'a [(Vec<usize>, usize)], later: &'a [Vec<u128>], dm: &'a [u128], ebit: &'a [u128], ebor: &'a [bool], allowed: usize, kq: u32 }
    fn pick(cx: &Ctx, gi: usize, start: usize, left: usize, stack: &mut Vec<usize>, chm: u128, nb: usize, acc: u32, cut: &mut usize,
            prune: &mut dyn FnMut(&[usize], u128, usize) -> bool, out: &mut dyn FnMut(&[usize], u128, usize)) {
        if left == 0 {
            if gi + 1 >= cx.gs.len() { out(stack, chm, nb); } else { pick(cx, gi + 1, 0, cx.gs[gi + 1].1, stack, chm, nb, acc, cut, prune, out); }
            return;
        }
        let list = &cx.gs[gi].0;
        for p in start..list.len() {
            if list.len() - p < left { break; }
            let j = list[p];
            if chm & cx.ebit[j] != 0 || (cx.ebor[j] && nb >= cx.allowed) { continue; }
            let (c2, n2) = (chm | cx.ebit[j], nb + cx.ebor[j] as usize);
            let acc2 = acc + (cx.dm[j] & !c2 & !cx.later[gi][p]).count_ones();
            if acc2 >= cx.kq { *cut += 1; continue; }
            stack.push(j);
            if !prune(stack, c2, n2) { pick(cx, gi, p + 1, left - 1, stack, c2, n2, acc2, cut, prune, out); }
            stack.pop();
        }
    }
    let t0 = Instant::now();
    // One enumeration pass keeping the decks that fewer than `kq` others beat. With `s1` (the kq = 1 result and
    // its deck identities) a deck is dropped at once when kq decks of that set, other than itself, beat it.
    let mut run_enum = |kq: usize, s1: Option<&(Kept, std::collections::HashSet<Vec<u64>>)>| -> Vec<DeckEval> {
    let mut groups: HashMap<(usize, usize), Kept> = HashMap::new();
    let kmax = kq;
    // every chain-kind count vector (fillers included in the (2, 2) count) with at most deck_size chain slots
    let mut counts: Vec<Vec<usize>> = vec![vec![]];
    for _ in 0..nk { counts = counts.into_iter().flat_map(|c| { let used: usize = c.iter().sum(); (0..=size - used).map(move |x| { let mut d = c.clone(); d.push(x); d }) }).collect(); }
    for cnt in &counts {
        let csum: usize = cnt.iter().sum();
        // a key needs real entries of each non-filler kind
        if (0..nk).any(|i| i != ki22 && cnt[i] > by_kind[i].len()) { continue; }
        let code: u64 = cnt.iter().zip(&kpos).map(|(&c, &p)| c as u64 * p).sum();
        let kd = key_cache.entry(code).or_insert_with_key(|&c| make_key(c));
        let kd: &KeyData = kd;
        let dmask = |b: usize, nb: usize| -> u128 { kd.d_own[b] | if nb - ebor[b] as usize + 1 <= allowed_borrow { kd.d_bor[b] } else { 0 } };
        let alive = |j: usize| is_locked[j] || (kd.d_own[j] | if ebor[j] { kd.d_bor[j] } else { 0 }).count_ones() < (size - 1 + kmax) as u32;
        let alive_k: Vec<Vec<usize>> = (0..nk).map(|i| by_kind[i].iter().copied().filter(|&j| alive(j)).collect()).collect();
        let alive_p: Vec<usize> = pal_ents.iter().copied().filter(|&j| alive(j)).collect();
        for real22 in 0..=cnt[ki22].min(by_kind[ki22].len()) {
            let filler = cnt[ki22] - real22;
            if (0..nk).any(|i| (if i == ki22 { real22 } else { cnt[i] }) > alive_k[i].len()) { continue; }
            let rem = size - csum;
            for np in 0..=rem.min(alive_p.len()) {
                let plain = rem - np;
                if plain > plain_chars.len() { continue; }
                let target = csum - filler + np;      // entries in the finished deck
                let mut gs: Vec<(Vec<usize>, usize)> = vec![];
                for i in 0..nk { let c = if i == ki22 { real22 } else { cnt[i] }; if c > 0 { gs.push((alive_k[i].clone(), c)); } }
                if np > 0 { gs.push((alive_p.clone(), np)); }
                // (a card added later could be the deck's borrowed card, so before the deck is complete an owned
                // card only counts owned dominators)
                let mut prune = |sel: &[usize], chm: u128, _nb: usize| -> bool {
                    let need = (target - sel.len() + kmax) as u32;
                    for &b in sel {
                        if is_locked[b] { continue; }
                        let m = kd.d_own[b] | if ebor[b] { kd.d_bor[b] } else { 0 };
                        if (m & !(chm & !ebit[b])).count_ones() >= need { n_subtrees += 1; return true; }
                    }
                    false
                };
                let mut out = |sel: &[usize], chm: u128, nb: usize| {
                    let pals = sel.iter().filter(|&&j| cards[rel[j]].pal).count() + plain;
                    if let Some(p) = dj.pals { if p != pals { return; } }
                    if !locked.iter().all(|l| sel.iter().any(|&j| rel[j] == *l)) { return; }
                    // swap rule over all deck cards (pairs, see above)
                    let mut swaps = 0u32;
                    for &b in sel {
                        if is_locked[b] { continue; }
                        swaps += (dmask(b, nb) & !(chm & !ebit[b])).count_ones();
                        if swaps as usize >= kmax { break; }
                    }
                    if swaps as usize >= kmax { n_swapped += 1; return; }
                    let mut mw = [1.0f64; 32]; let mut mg = [1.0f64; 32];
                    for &j in sel {
                        let (w, g) = (&kd.mw[j * nu..(j + 1) * nu], &kd.mg[j * nu..(j + 1) * nu]);
                        for t in 0..nu { mw[t] *= w[t]; mg[t] *= g[t]; }
                    }
                    for t in 0..nu { mw[t] = 1.0 - mw[t]; mg[t] = 1.0 - mg[t]; }
                    let borrowed = sel.iter().copied().find(|&j| ebor[j]).map(|j| rel[j]);
                    n_decks += 1;
                    let mut s = 0.0;
                    for (c, &(k, is_g)) in coords.iter().enumerate() {
                        dv[c] = if is_g { mg[k] } else { 1.0 - (1.0 - mw[k]) * (1.0 - mg[k]) };
                        s += dv[c];
                    }
                    // One group for every pal count: the result is the top decks overall, and a deck that top_decks
                    // others match or beat on every coordinate is at or below each of them for every loop.
                    let nd = coords.len();
                    let g = groups.entry((0, 0)).or_insert_with(|| Kept::new(nd));
                    if kmax == 1 && g.has_last && dominates(&g.lastrow, &dv) { return; }
                    if kmax == 1 && nd > 0 && (0..g.rlen).any(|r| dominates(&g.recent[r * nd..(r + 1) * nd], &dv)) { st_ring += 1; return; }
                    if kmax > 1 && g.rlen >= kmax && g.count_recent(&dv) >= kmax { st_ring += 1; return; }
                    let xq = sig_of(&dv, true);
                    if let Some((s1k, ids)) = s1 {
                        // kq decks of the set beat it: drop it, unless it is one of them (then it needs one more)
                        if s1k.count_ro(&dv, xq, s, kmax) >= kmax {
                            let own = ids.contains(&ident(&sel.iter().map(|&j| rel[j]).collect::<Vec<_>>(), &sel.iter().map(|&j| endk[j]).collect::<Vec<_>>(), plain, filler));
                            if !own || s1k.count_ro(&dv, xq, s, kmax + 1) > kmax { st_s1 += 1; return; }
                        }
                    }
                    let n = g.count(&dv, xq, s, kmax, &mut st_tests);
                    st_full += 1;
                    if n > 0 && (kmax > 1 || n >= kmax) && nd > 0 { g.remember_found(); }
                    if n >= kmax { st_dom += 1; return; }
                    let mut so: Vec<usize> = sel.to_vec(); so.sort();       // entry order, for display
                    let x = DeckEval { pals, borrowed, cards: so.iter().map(|&j| rel[j]).collect(), ends: so.iter().map(|&j| endk[j]).collect(), plain, filler, dw: mw[..nu].to_vec(), dg: mg[..nu].to_vec(), dv: dv.clone(), s };
                    if kmax == 1 { g.remove_dominated_by(&dv, sig_of(&dv, false), s); }
                    g.insert(x);
                    if kmax > 1 && g.len() > g.cap {
                        // compaction drops decks, so the remembered dominators are forgotten (Kept::new clears them)
                        let list = std::mem::replace(g, Kept::new(nd)).into_list();
                        *g = k_filter(list, kmax, nd);
                        g.cap = (2 * g.len()).max(512);
                    }
                };
                let mut stack: Vec<usize> = Vec::with_capacity(size);
                if gs.is_empty() { out(&stack, 0, 0); } else {
                    // later[gi][p]: characters of entries after position p in group gi and of every later group
                    let mut later: Vec<Vec<u128>> = vec![vec![]; gs.len()];
                    let mut tail = 0u128;
                    for gi in (0..gs.len()).rev() {
                        let l = &gs[gi].0; let mut v = vec![0u128; l.len()]; let mut m = tail;
                        for p in (0..l.len()).rev() { v[p] = m; m |= ebit[l[p]]; }
                        tail = m; later[gi] = v;
                    }
                    // legal dominators whatever else the deck holds: owned ones, or any for a borrowed card
                    let dm: Vec<u128> = (0..nrel).map(|j| if is_locked[j] { 0 } else { kd.d_own[j] | if ebor[j] { kd.d_bor[j] } else { 0 } }).collect();
                    let cx = Ctx { gs: &gs, later: &later, dm: &dm, ebit: &ebit, ebor: &ebor, allowed: allowed_borrow, kq: kmax as u32 };
                    let mut cut = 0usize;
                    pick(&cx, 0, 0, gs[0].1, &mut stack, 0, 0, 0, &mut cut, &mut prune, &mut out);
                    n_subtrees += cut;
                }
            }
        }
    }
    let mut front: Vec<DeckEval> = vec![];
    let mut gkeys: Vec<(usize, usize)> = groups.keys().copied().collect(); gkeys.sort();
    for pc in &gkeys {
        let kept = k_filter(groups.remove(pc).unwrap().into_list(), kmax, coords.len()).into_list();
        front.extend(kept);
    }
    front
    };
    // top_decks > 1: first the decks no other deck beats (fast), then the full pass filtered against them
    let front: Vec<DeckEval> = if kmax == 1 { run_enum(1, None) } else {
        let s1 = run_enum(1, None);
        let nd = coords.len();
        let mut sorted: Vec<&DeckEval> = s1.iter().collect();
        sorted.sort_by_key(|x| std::cmp::Reverse(sig_of(&x.dv, false)));      // similar decks share blocks
        let mut kk = Kept::new(nd);
        for x in sorted { kk.insert(x.clone()); }
        let ids: std::collections::HashSet<Vec<u64>> = s1.iter().map(|x| ident(&x.cards, &x.ends, x.plain, x.filler)).collect();
        if std::env::var("DECK_DEBUG").is_ok() { eprintln!("pass 1: {} decks no other beats, {:.2}s", s1.len(), t0.elapsed().as_secs_f64()); }
        run_enum(kmax, Some(&(kk, ids)))
    };
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("tests {} scans {} avg_lim {:.0} dom_in_scan {} ring {} swapped {} subtrees {} s1 {}", st_tests, st_full, 0.0, st_dom, st_ring, n_swapped, n_subtrees, st_s1); }
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("rel {} decks {} enum {:.2}s", nrel, n_decks, t0.elapsed().as_secs_f64()); }
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("front {} pareto {:.2}s", front.len(), t0.elapsed().as_secs_f64()); }
    if front.is_empty() { return json!({"error": "No deck can be built from the allowed cards."}); }
    // Ceiling hierarchy over the frontier: super-clusters (pal count + which odds are >= 50%) contain clusters
    // (pal count + odds in quarter buckets) contain decks. A group's componentwise max odds give a valid ceiling
    // for every deck in it, because P(all land) only increases with each deck odd.
    // Decks that no other frontier deck (of any pal count) dominates. The metric rises with every dominance
    // coordinate, so for any loop the best value over the frontier is reached on this skyline; the ceilings
    // below (which only bound that best value) are built from it. Every frontier deck is still scored when a
    // loop is solved.
    let sky: Vec<usize> = {
        let tagged: Vec<DeckEval> = front.iter().enumerate().map(|(i, x)| { let mut y = x.clone(); y.cards = vec![i]; y }).collect();
        let mut v: Vec<usize> = k_filter(tagged, 1, coords.len()).into_list().into_iter().map(|y| y.cards[0]).collect();
        v.sort();
        v
    };
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("skyline {} of {}", sky.len(), front.len()); }
    struct Grp { w: Vec<f64>, g: Vec<f64>, kids: Vec<usize>, gk: (usize, usize) }
    let mut clusters: Vec<Grp> = vec![];
    let mut supers: Vec<Grp> = vec![];
    {
        let mut cidx: HashMap<Vec<u32>, usize> = HashMap::new();
        let mut sidx: HashMap<Vec<u32>, usize> = HashMap::new();
        for &i in &sky { let x = &front[i];
            let bk = x.borrowed.map_or(u32::MAX, |b| b as u32);
            let mut ck: Vec<u32> = vec![x.pals as u32, bk];
            let mut sk2: Vec<u32> = vec![x.pals as u32, bk];
            for k in 0..nu {
                ck.push((x.dw[k] * 4.0).floor() as u32); ck.push((x.dg[k] * 4.0).floor() as u32);
                sk2.push((x.dw[k] >= 0.5) as u32); sk2.push((x.dg[k] >= 0.5) as u32);
            }
            let new_c = !cidx.contains_key(&ck);
            let ci = *cidx.entry(ck).or_insert_with(|| { clusters.push(Grp { w: vec![0.0; nu], g: vec![0.0; nu], kids: vec![], gk: (x.pals, x.borrowed.unwrap_or(usize::MAX)) }); clusters.len() - 1 });
            let c = &mut clusters[ci];
            for k in 0..nu { c.w[k] = c.w[k].max(x.dw[k]); c.g[k] = c.g[k].max(x.dg[k]); }
            c.kids.push(i);
            let si = *sidx.entry(sk2).or_insert_with(|| { supers.push(Grp { w: vec![0.0; nu], g: vec![0.0; nu], kids: vec![], gk: (x.pals, x.borrowed.unwrap_or(usize::MAX)) }); supers.len() - 1 });
            let sg = &mut supers[si];
            for k in 0..nu { sg.w[k] = sg.w[k].max(x.dw[k]); sg.g[k] = sg.g[k].max(x.dg[k]); }
            if new_c { sg.kids.push(ci); }
        }
    }

    // ---- loop search ----
    let n = d.umas.len();
    let id_ok = |id: u32| (sj.owned.is_empty() || sj.owned.contains(&id) || sj.require.contains(&id)) && !sj.exclude.contains(&id);
    let pool: Vec<usize> = (0..n).filter(|&i| id_ok(d.umas[i].id) && !model.no_outfit[i]).collect();
    let req: Vec<usize> = sj.require.iter().filter_map(|&id| d.umas.iter().position(|u| u.id == id)).collect();
    if pool.len() < 4 { return json!({"error": "Pick at least 4 umas to search."}); }
    let gmax = |pc: Option<usize>| -> (Vec<f64>, Vec<f64>) {
        let mut w = vec![0.0f64; nu]; let mut g = vec![0.0f64; nu];
        for x in front.iter().filter(|x| pc.map_or(true, |p| x.pals == p)) { for k in 0..nu { w[k] = w[k].max(x.dw[k]); g[k] = g[k].max(x.dg[k]); } }
        (w, g)
    };
    let (allw, allg) = gmax(None);
    // coarse groups: pal count x borrowed card (decks with different borrowed cards cannot share odds)
    let mut gvec: Vec<((usize, usize), Vec<f64>, Vec<f64>)> = vec![];
    for x in sky.iter().map(|&i| &front[i]) {
        let k = (x.pals, x.borrowed.unwrap_or(usize::MAX));
        let gi = match gvec.iter().position(|e| e.0 == k) { Some(i) => i, None => { gvec.push((k, vec![0.0; nu], vec![0.0; nu])); gvec.len() - 1 } };
        for t in 0..nu { gvec[gi].1[t] = gvec[gi].1[t].max(x.dw[t]); gvec[gi].2[t] = gvec[gi].2[t].max(x.dg[t]); }
    }
    let _ = gmax;
    let tq = Instant::now();
    let fixed_s: Option<Vec<Option<Sched>>> = dj.schedule.as_deref().map(|mode| (0..n).map(|i| if pool.contains(&i) { Some(prebuilt(d, &d.umas[i], mode)) } else { None }).collect());
    let no_race = dj.schedule.as_deref() == Some("none");
    let pb = match &fixed_s {
        // fixed schedules: the overlaps are known exactly, so the ceilings are exact
        Some(fs) => {
            let ps: Vec<Option<Vec<f64>>> = fs.iter().enumerate().map(|(i, s)| s.as_ref().map(|s| probs(d, &d.umas[i], s))).collect();
            let mut m = vec![vec![0.0f64; n]; n];
            if !no_race { for &a in &pool { for &b in &pool { if a != b { m[a][b] = wval(ps[a].as_ref().unwrap(), ps[b].as_ref().unwrap()); } } } }
            m
        }
        None => pair_ceilings(d, &pool),
    };
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("supers {} clusters {} pb {:.2}s", supers.len(), clusters.len(), tq.elapsed().as_secs_f64()); }
    let k = pool.len();
    let lin_t: Vec<usize> = lineages().iter().map(|l| l.0).collect();
    let fixed = model.fixed_factor();
    // q range for the per-trainee table: the zero-affinity floor and, per trainee, an analytic ceiling over every
    // lineage it can head (no pass over the foursomes needed)
    let qmax: Vec<f64> = (0..n).map(|u| if pool.contains(&u) { model.q_ub_trainee(u, &pool, &pb) } else { 0.0 }).collect();
    let qlo = model.q_floor();
    let qhi = pool.iter().map(|&u| qmax[u]).fold(qlo, f64::max);
    // Per-trainee ceiling table. The metric is the mean over the 12 lineages of a per-trainee term
    // T(u, q, deck) (`trainee_fast`), and T rises with q. Letting every lineage pick its own best skyline deck can
    // only raise the sum, so  sum_l max_deck T(t_l, q_l, deck)  bounds the best deck for the foursome.
    // B[u][i] = max over skyline decks of T(u, grid[i], deck), with each q rounded up to the next grid point
    // (any q at or below qlo uses grid[0] = qlo; no q exceeds qhi = grid[NG-1]).
    const NG: usize = 64;
    let step = if qhi > qlo { (qhi - qlo) / (NG - 1) as f64 } else { 0.0 };
    let grid: Vec<f64> = (0..NG).map(|i| if i == NG - 1 { qhi } else { qlo + step * i as f64 }).collect();
    let gidx = |q: f64| -> usize {
        if step == 0.0 { return NG - 1; }
        let mut i = (((q - qlo) / step).ceil().max(0.0) as usize).min(NG - 1);
        while grid[i] < q && i < NG - 1 { i += 1; }
        i
    };
    // filled on demand: only the umas and grid points the generated foursomes touch are ever computed
    let btab_c: std::cell::RefCell<Vec<[f64; NG]>> = std::cell::RefCell::new(vec![[f64::NAN; NG]; n]);
    let btab = |u: usize, i: usize| -> f64 {
        let v = btab_c.borrow()[u][i];
        if !v.is_nan() { return v; }
        let q = grid[i];
        let v = sky.iter().map(|&j| model.trainee_fast(u, q, &front[j].dw, &front[j].dg)).fold(0.0, f64::max);
        btab_c.borrow_mut()[u][i] = v; v
    };
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("q range {:.4}..{:.4} table {:.2}s", qlo, qhi, tq.elapsed().as_secs_f64()); }
    // Separable bound. Each uma heads 3 of the 12 lineages and every one of its q is <= qmax[u], so
    //   ceiling(f) <= fixed/12 * sum_u 3 * M[u],  M[u] = B[u][grid(qmax[u])].
    // Foursomes are generated in bands of that bound, highest first; only those whose bound can still beat the
    // k-th best deck value ever get their real ceiling computed.
    let mval: Vec<f64> = (0..n).map(|u| if pool.contains(&u) { btab(u, gidx(qmax[u])) } else { 0.0 }).collect();
    let mut ord: Vec<usize> = pool.clone();
    ord.sort_by(|&a, &b| mval[b].partial_cmp(&mval[a]).unwrap());
    let mv: Vec<f64> = ord.iter().map(|&u| mval[u]).collect();
    let sep = |s: f64| fixed * 3.0 * s / 12.0 * (1.0 + 1e-12);           // bound from a sum of M (with float slack)
    let nreq = req.len();
    let total_foursomes: usize = { let m = k - nreq.min(k); let c = 4usize.saturating_sub(nreq);
        (0..c).fold(1usize, |acc, i| acc * (m - i) / (i + 1)) };
    // generate foursomes with  lo <= sum M < hi  (same summation order for both tests, so bands never overlap)
    let band = |lo: f64, hi: f64, out: &mut Vec<(f64, [usize; 4])>| {
        for a in 0..k {
            if a + 3 >= k || mv[a] + mv[a + 1] + mv[a + 2] + mv[a + 3] < lo { break; }
            for b in a + 1..k {
                if b + 2 >= k || mv[a] + mv[b] + mv[b + 1] + mv[b + 2] < lo { break; }
                for c in b + 1..k {
                    if c + 1 >= k || mv[a] + mv[b] + mv[c] + mv[c + 1] < lo { break; }
                    for e in c + 1..k {
                        let sm = mv[a] + mv[b] + mv[c] + mv[e];
                        if sm < lo { break; }
                        if sm >= hi { continue; }
                        let mut f = [ord[a], ord[b], ord[c], ord[e]]; f.sort();   // roster order, as before
                        if !req.iter().all(|r| f.contains(r)) { continue; }
                        let qs = model.lineage_q_arr(f, &|x, y| pb[f[x]][f[y]]);
                        // 1e-12 slack: the bound sums in a different order than the metric
                        let ub = fixed * (0..12).map(|l| btab(f[lin_t[l]], gidx(qs[l]))).sum::<f64>() / 12.0 * (1.0 + 1e-12);
                        out.push((ub, f));
                    }
                }
            }
        }
    };
    let smax = if k >= 4 { mv[0] + mv[1] + mv[2] + mv[3] } else { 0.0 };
    let mut hi_sum = f64::INFINITY;          // foursomes with sum M >= hi_sum are generated
    let lo_sum = smax * 0.85;                // first band
    let mut cand: Vec<(f64, [usize; 4])> = vec![];
    band(lo_sum, hi_sum, &mut cand);
    hi_sum = lo_sum;
    cand.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("ceilings {:.2}s first band {}", tq.elapsed().as_secs_f64(), cand.len()); }
    // best per pal count: (value, foursome idx in solved, deck idx)
    let mut best: HashMap<usize, (f64, usize, usize)> = HashMap::new();
    let mut solved: Vec<([usize; 4], Res, Vec<f64>)> = vec![];
    let mut next = 0; let mut scanned = 0; let mut generated = cand.len();
    // Solve foursomes in ceiling order until no unsolved ceiling, generated or not, can beat the k-th best deck.
    let kdeck = dj.top_decks.max(1);
    let mut deck_best: Vec<(f64, usize)> = vec![(f64::NEG_INFINITY, 0); front.len()];
    let mut kth_val = f64::NEG_INFINITY;
    loop {
        if !(next < cand.len() && cand[next].0 >= kth_val) {
            // every generated candidate is done; foursomes not generated yet are bounded by sep(hi_sum)
            // a foursome whose bound is at most the k-th best value cannot beat it
            if hi_sum <= 0.0 || sep(hi_sum) <= kth_val { break; }
            // next band: down to where the bound reaches the k-th best value (a hair below, so the band after it
            // is never needed), but at most 15% of the remaining range at a time while that value is still low
            let kth_lo = if kth_val > f64::NEG_INFINITY { kth_val * 12.0 / (3.0 * fixed) * (1.0 - 1e-9) } else { 0.0 };
            let lo = kth_lo.max(hi_sum * 0.85);
            let lo = if lo < smax * 1e-6 { 0.0 } else { lo };
            let mut more: Vec<(f64, [usize; 4])> = vec![];
            band(lo, hi_sum, &mut more);
            hi_sum = lo; generated += more.len();
            let mut rest: Vec<(f64, [usize; 4])> = cand.split_off(next);
            rest.extend(more);
            rest.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
            cand.extend(rest);
            continue;
        }
        let (_, f) = cand[next]; next += 1; scanned += 1;
        // tight ceiling: each skyline deck scored on its own (still with the pairwise-ceiling overlaps), through
        // the group / super-cluster / cluster maxima first
        let qpb = model.lineage_q_arr(f, &|x, y| pb[f[x]][f[y]]);
        let cur = kth_val;
        let gpass: Vec<(usize, usize)> = gvec.iter().filter(|(_, w, g)| model.metric_fast(f, &qpb, w, g) >= cur).map(|(k, ..)| *k).collect();
        let pass = supers.iter().any(|sg| gpass.contains(&sg.gk) && model.metric_fast(f, &qpb, &sg.w, &sg.g) >= cur
            && sg.kids.iter().any(|&ci| { let c = &clusters[ci]; model.metric_fast(f, &qpb, &c.w, &c.g) >= cur
                && c.kids.iter().any(|&i| model.metric_fast(f, &qpb, &front[i].dw, &front[i].dg) >= cur) }));
        if !pass { continue; }
        let r = match &fixed_s {
            Some(fs) => Res { val: 0.0, s: std::array::from_fn(|i| fs[f[i]].unwrap()), order: [0, 1, 2, 3], evals: 0, secs: 0.0 },
            None => optimize(d, f, "scf@0.5", sj.iters, sj.seed, 60.0, &[]),
        };
        let ps: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let qs = model.lineage_q(f, &|x, y| if no_race { 0.0 } else { wval(&ps[x], &ps[y]) });
        let qa: [f64; 12] = std::array::from_fn(|i| qs[i]);
        let si = solved.len();
        for (di, x) in front.iter().enumerate() {
            let val = model.metric_fast(f, &qa, &x.dw, &x.dg);
            let e = best.entry(x.pals).or_insert((f64::NEG_INFINITY, 0, 0));
            if val > e.0 { *e = (val, si, di); }
            if val > deck_best[di].0 { deck_best[di] = (val, si); }
        }
        solved.push((f, r, qs));
        let mut vals: Vec<f64> = deck_best.iter().map(|x| x.0).collect();
        vals.sort_by(|a, b| b.partial_cmp(a).unwrap());
        kth_val = if vals.len() >= kdeck { vals[kdeck - 1] } else { f64::NEG_INFINITY };
    }
    if solved.is_empty() { return json!({"error": "No foursome contains all the required umas."}); }
    let _ = (&allw, &allg);
    let _ = &gvec;
    let minbest = |_b: &HashMap<usize, (f64, usize, usize)>| kth_val;
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("search {:.2}s scanned {} solved {}", tq.elapsed().as_secs_f64(), scanned, solved.len()); }
    // ---- output ----
    let lin = lineages();
    let deck_json = |x: &DeckEval| -> Value {
        let mut names: Vec<String> = x.cards.iter().zip(&x.ends).map(|(&i, &e)| {
            let mut n = cards[i].name.clone();
            if e > 0 { n = format!("{} (end chain at link {})", n, e); }
            if Some(i) == x.borrowed { n = format!("{} (borrowed)", n); }
            n
        }).collect();
        names.extend(plain_names.iter().take(x.plain).cloned());
        for _ in 0..x.filler { names.push("(any SR without target hints)".into()); }
        let odds: Vec<Value> = (0..nu).map(|k| json!({"group": unc[k], "white": x.dw[k], "gold": x.dg[k]})).collect();
        let pal_names: Vec<String> = x.cards.iter().filter(|&&i| cards[i].pal).map(|&i| cards[i].name.clone()).chain(plain_names.iter().take(x.plain).cloned()).collect();
        json!({"pals": x.pals, "cards": names, "odds": odds, "pal_cards": pal_names, "borrowed": x.borrowed.map(|i| cards[i].name.clone()),
               "card_ids": x.cards.iter().map(|&i| cards[i].id).collect::<Vec<_>>(), "end_chain_at": x.ends.clone()})
    };
    let loop_json = |f: [usize; 4], r: &Res, qs: &[f64], x: &DeckEval, v: f64| -> Value {
        let mut minus1 = 0.0; let mut per = vec![(0.0, 0usize); 4]; let mut picks: Vec<Vec<usize>> = vec![vec![]; 4];
        for (&(t, ..), &q) in lin.iter().zip(qs) {
            let (val, kk) = model.trainee_d(f[t], q, &x.dw, &x.dg);
            let mut ps = vec![GEN_GOLD; model.n_fixed_gold as usize];
            ps.extend(model.skill_probs_d(&model.opts[f[t]][kk].0, &model.ews[f[t]][kk], q, &x.dw, &x.dg));
            minus1 += atleast(&ps, sj.targets.len().saturating_sub(1));
            per[t].0 += model.fixed_pub() * val; per[t].1 += 1; picks[t].push(kk);
        }
        minus1 /= lin.len() as f64;
        let outfits: Vec<Value> = (0..4).map(|i| {
            let kk = *picks[i].iter().max_by_key(|x| picks[i].iter().filter(|y| y == x).count()).unwrap();
            let (st, cs) = &model.opts[f[i]][kk];
            let covers: Vec<Value> = st.iter().enumerate().filter(|(_, lv)| **lv > 0).map(|(j, lv)| json!({"group": unc[j], "level": lv})).collect();
            json!({"outfits": if covers.is_empty() { vec![] } else { cs.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>() }, "covers": covers})
        }).collect();
        let s4 = [r.s[0], r.s[1], r.s[2], r.s[3]];
        let ps: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let detail = model.detail(f, &|a, b| wval(&ps[a], &ps[b]), Some((&x.dw, &x.dg)));
        json!({"members": member_json(d, &f), "p_all": v, "p_minus1": minus1, "outfits": outfits,
               "per_trainee": per.iter().map(|(s, c)| s / *c as f64).collect::<Vec<_>>(),
               "mean_proc": qs.iter().sum::<f64>() / qs.len() as f64, "schedule": chart_rows(d, &f, &s4), "detail": detail})
    };
    // best deck found per pal count, among the kept decks and solved loops: indicative only (decks of one pal
    // count can be pruned by decks of another); for an exact answer at a given pal count, rerun with `pals`
    let mut pal_counts: Vec<usize> = best.keys().copied().collect(); pal_counts.sort();
    let mut by_pals: Vec<Value> = vec![];
    for p in &pal_counts {
        if let Some(&(v, si, di)) = best.get(p) {
            let (f, r, qs) = &solved[si];
            by_pals.push(json!({"pals": p, "deck": deck_json(&front[di]), "loop": loop_json(*f, r, qs, &front[di], v)}));
        }
    }
    by_pals.sort_by(|a, b| b["loop"]["p_all"].as_f64().partial_cmp(&a["loop"]["p_all"].as_f64()).unwrap());
    let mut order: Vec<usize> = (0..front.len()).filter(|&i| deck_best[i].0 > f64::NEG_INFINITY).collect();
    order.sort_by(|&a, &b| deck_best[b].0.partial_cmp(&deck_best[a].0).unwrap());
    let top: Vec<Value> = order.iter().take(kdeck).map(|&di| { let (v, si) = deck_best[di]; let (f, r, qs) = &solved[si];
        json!({"deck": deck_json(&front[di]), "loop": loop_json(*f, r, qs, &front[di], v)}) }).collect();
    let next_bound = (if next < cand.len() { cand[next].0 } else { 0.0 }).max(if hi_sum > 0.0 { sep(hi_sum) } else { 0.0 });
    json!({"top": top, "best": by_pals, "decks_built": n_decks, "decks_on_frontier": front.len(), "foursomes": total_foursomes,
           "foursomes_ranked": generated, "foursomes_scanned": scanned, "foursomes_solved": solved.len(), "next_bound": next_bound,
           "proven": next_bound < minbest(&best), "relevant_cards": rel.len(), "cards_dropped": n_card_dropped, "decks_swapped": n_swapped, "plain_pals": plain_names})
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The allocation-free metric used in the deck search equals the reference metric.
    #[test]
    fn fast_metric_matches_reference() {
        let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
        let sk: SkillsRaw = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web_skills.json")).unwrap()).unwrap();
        let sj: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":21010},{"group":20165}]})).unwrap();
        let m = Model::new(&d, &sk, &sj);
        let nu = m.uncertain.len();
        for (fi, f) in [[0usize, 5, 9, 20], [3, 7, 11, 40], [1, 2, 3, 4]].iter().enumerate() {
            let qa = m.lineage_q_arr(*f, &|_, _| 3.0);
            let qv = m.lineage_q(*f, &|_, _| 3.0);
            assert!(qa.iter().zip(&qv).all(|(a, b)| (a - b).abs() < 1e-15));
            let dw: Vec<f64> = (0..nu).map(|k| ((k + fi) as f64 * 0.17) % 1.0).collect();
            let dg: Vec<f64> = (0..nu).map(|k| ((k * 3 + fi) as f64 * 0.11) % 1.0).collect();
            let a = m.metric_fast(*f, &qa, &dw, &dg); let b = m.metric_d(*f, &qv, &dw, &dg);
            assert!((a - b).abs() < 1e-15 * b.max(1.0), "{a} vs {b}");
        }
    }
    /// Calibration: lambda 7.1 reproduces the observed full-chain completion (6 -> 81%, 9 -> 50%) within 3 points,
    /// and first links fire far more often than last links.
    #[test]
    fn chain_model_calibration() {
        let m = chain_marginals(&[(3, 3), (3, 3)], 7.1);
        assert!((m[&(3, 3)][2] - 0.81).abs() < 0.03, "{:?}", m);
        assert!(m[&(3, 3)][0] > 0.97);
        let m9 = chain_marginals(&[(3, 3), (3, 3), (3, 3)], 7.1);
        assert!((m9[&(3, 3)][2] - 0.50).abs() < 0.03, "{:?}", m9);
        let one = chain_marginals(&[(3, 3)], 7.1);
        assert!(one[&(3, 3)][2] > 0.95);
        // ending a chain early frees its slot: the other card's links fire more often
        let early = chain_marginals(&[(3, 3), (3, 2)], 7.1);
        assert!(early[&(3, 3)][2] > m[&(3, 3)][2] && early[&(3, 2)].len() == 2);
    }
}
