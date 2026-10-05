//! Joint deck + loop search for skill targets.
//!
//! Deck model (per training run):
//!   - training hints: each card gives h = base(scenario) x (1 + hint_frequency/100) draws, each uniform over its
//!     skill options still below Lv 5 (the stat option is never drawn; a skill at Lv 5 leaves the pool, so a card
//!     giving +5 levels per draw never repeats one), `hint_miss`; a card's draws hint two of its targets together less
//!     often than independent odds would say;
//!   - chain events: the number in a run is Binomial(CHAIN_N, CHAIN_P); each advances a chain picked among the
//!     remaining chain slots of the non-pal cards (a card with more links left is picked more often; links fire in
//!     order). Pals and R cards have no chain slots. Both fitted on uma-it Independent Training runs (docs/MODEL.md);
//!   - event rewards come from the job (`events`): (card, skill group, 1 white | 2 gold, link; 0 = whenever the
//!     card is in the deck, e.g. pal golds).
//! Deck hint odds per target: 1 - prod(1 - p) over sources, separately for white and gold.
//!
//! Search (exact for the model): race schedules maximise affinity and do not depend on the deck, so each
//! foursome's per-lineage proc rates are computed once and every deck on the Pareto frontier is scored against
//! them. Foursomes are visited in descending order of a ceiling computed with the best odds any deck gives per
//! target; a foursome is solved only while its ceiling for some pal count can still beat that pal count's best.
use super::*;
use crate::skills::{chart_rows, lineages, member_json, Model, SkillJob, SkillsRaw, GEN_GOLD};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Deserialize, Clone)]
pub(crate) struct Card { pub id: u32, pub name: String, pub chara: u32, pub rarity: u8, pub pal: bool, pub links: u8, pub hf: f64,
    /// training-hint options that are skills (the stat option is never drawn in Independent Training)
    #[serde(default)] pub nskill: u32, pub whites: Vec<u32>,
    /// hint levels per training-hint draw at max limit break (1 + hint-level effect); a skill at Lv 5 leaves the pool
    #[serde(default = "d_hinc")] pub hinc: u32,
    /// characters who cannot train with this card: its own character and, for a group card, every member
    #[serde(default)] pub charas: Vec<u32> }
impl Card {
    pub(crate) fn blocks(&self, uma: u32) -> bool { self.chara == uma || self.charas.contains(&uma) }
}
#[derive(Deserialize)]
pub(crate) struct CardsRaw { pub cards: Vec<Card> }

#[derive(Deserialize)]
pub(crate) struct DeckJob {
    /// chain events per run: Binomial(n, p); defaults CHAIN_N, CHAIN_P (for experiments)
    #[serde(default)] pub chain_events: Option<(u32, f64)>,
    /// Independent Training scenario: "unity" (default), "gc" (Grand Concert), "ura", "trackblazer", "grandmasters" (JP); sets how many
    /// training-hint draws a card gives (`hint_base`)
    #[serde(default = "d_scenario")] pub scenario: String,
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
    /// links where IT's default choice ends a card's chain on some outcomes: (card, link, k outcomes, of n)
    #[serde(default)] pub stop_risk: Vec<(u32, u8, u8, u8)>,
    /// events that name two or more skills, in full: (card, link, [choice][outcome] -> (skill group, 1 | 2)), the
    /// choices that keep the chain going, outcomes equally likely. At these links they replace `events` /
    /// `events_p`, which credit every skill on its own.
    #[serde(default)] pub alts: Vec<(u32, u8, Vec<Vec<Vec<(u32, u8)>>>)>,
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
    /// one deck per uma instead of one shared deck: each trainee runs her own best legal deck (across her three
    /// lineages); `top_decks` then counts loops
    #[serde(default)] pub per_uma: bool,
    /// internal: return the skyline decks only (used for a uma's own-card-free deck set in per-uma mode)
    #[serde(default)] pub deck_only: bool,
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
fn kmax_top(dj: &DeckJob) -> usize { if dj.per_uma || dj.deck_only { 1 } else { dj.top_decks.max(1) } }
fn d_scenario() -> String { "unity".into() }
fn d_hinc() -> u32 { 1 }
/// Chain events per run ~ Binomial(CHAIN_N, CHAIN_P): fitted on 879 uma-it Independent Training runs whose chain
/// progress is readable from event hints (mean 7.35, variance 4.8; no difference between scenarios; a Poisson count,
/// variance = mean, fits much worse). It slightly underestimates decks with two chain cards: both chains finish in
/// 82% of such runs, 75% predicted.
const CHAIN_N: u32 = 21;
const CHAIN_P: f64 = 0.35;
/// Training-hint draws per run for a card with hint frequency 0, by scenario (h = base x (1 + hf/100)); fitted on the
/// same runs' training hints with the Lv-5 cap (`hint_miss`; max-limit-break card-runs, 92 uploaders; 90%: Unity
/// 2.66-2.75, Grand Concert 2.03-2.13, URA 1.48-1.73; Trackblazer has too few runs and Grand Masters (JP) none, both use
/// the pooled 2.30).
pub(crate) fn hint_base(scenario: &str) -> f64 {
    match scenario { "gc" => 2.08, "ura" => 1.60, "trackblazer" | "grandmasters" => 2.30, _ => 2.70 }
}
/// P(none of `k` given skills of a card's pool is drawn) in `t` training-hint draws over its `n` skills, each draw
/// uniform over the skills still below Lv 5; a skill leaves the pool after `cap` draws (ceil(5 / hint levels per
/// draw)), so a +5 card never draws a skill twice. Exact over how many draws each other skill has taken.
fn miss_draws(k: usize, n: usize, t: usize, cap: usize) -> f64 {
    if k == 0 { return 1.0; }
    if k >= n { return if t == 0 { 1.0 } else { 0.0 }; }
    if t <= cap { return (1.0 - k as f64 / n as f64).powi(t as i32); }   // no skill can leave the pool before the last draw
    // state: how many of the n - k other skills have taken 0, 1, .., cap draws (cap: left the pool)
    let mut st: HashMap<Vec<u8>, f64> = HashMap::new();
    let mut s0 = vec![0u8; cap + 1]; s0[0] = (n - k) as u8; st.insert(s0, 1.0);
    for _ in 0..t {
        let mut nx: HashMap<Vec<u8>, f64> = HashMap::new();
        for (s, p) in &st {
            let avail = (n - k - s[cap] as usize + k) as f64;   // k > 0: never empty
            for j in 0..cap {
                if s[j] == 0 { continue; }
                let mut u = s.clone(); u[j] -= 1; u[j + 1] += 1;
                *nx.entry(u).or_insert(0.0) += p * s[j] as f64 / avail;
            }
        }
        st = nx;
    }
    st.values().sum()
}
/// `miss_draws` at a fractional draw count h: IT gives a near-fixed whole number of draws per setup, so h is read as
/// floor(h) or floor(h) + 1 draws with the fractional part as the weight of the second.
pub(crate) fn hint_miss(k: usize, n: usize, h: f64, hinc: u32) -> f64 {
    let cap = (5 + hinc.max(1) as usize - 1) / hinc.max(1) as usize;
    let t0 = h.max(0.0).floor(); let f = h - t0; let t0 = t0 as usize;
    (1.0 - f) * miss_draws(k, n, t0, cap) + if f > 0.0 { f * miss_draws(k, n, t0 + 1, cap) } else { 0.0 }
}
fn d_size() -> usize { 6 }
fn d_rar() -> u8 { 2 }   // SR: R cards have no chain events, so this only filters chain cards

/// Chain kind of a deck entry: (chain length, link the chain stops at). A card played normally stops at its last
/// link; a card whose chain the player ends early (a choice that ends the chain) stops sooner. While it runs, an
/// entry is picked with weight = links it has left (length - done); once it reaches its stop it is out.
/// A deck entry's chain kind: (chain links, link it stops after, stop risk). Stop risk: links where IT's default
/// choice ends the chain on k of n outcomes, one byte per link 1..=4 ((k << 4) | n; 0 = none), only links before the stop.
type Kind = (u8, u8, u32);
fn stop_risk_code(risk: &[(u8, u8, u8)], stop: u8) -> u32 {
    risk.iter().filter(|r| r.0 >= 1 && r.0 < stop && r.0 <= 4 && r.2 > 0).fold(0u32, |c, r| c | ((((r.1 as u32) << 4) | r.2 as u32) << (8 * (r.0 as u32 - 1))))
}

thread_local! { static CHAIN_EVENTS: std::cell::Cell<(u32, f64)> = std::cell::Cell::new((CHAIN_N, CHAIN_P)); }
/// P(N = e) chain events per run for e = 0..=n, N ~ Binomial of the job in progress (default CHAIN_N, CHAIN_P).
fn event_pmf(n: usize) -> Vec<f64> {
    let (bn, bp) = CHAIN_EVENTS.with(|c| c.get());
    (0..=n).map(|e| if e as u32 <= bn { binom_coef(bn, e as u32) * bp.powi(e as i32) * (1.0 - bp).powi((bn - e as u32) as i32) } else { 0.0 }).collect()
}
fn binom_coef(n: u32, k: u32) -> f64 { (0..k).fold(1.0f64, |a, i| a * (n - i) as f64 / (i + 1) as f64) }
/// Cache key of the event-count distribution in use.
fn pmf_key() -> u64 { CHAIN_EVENTS.with(|c| { let (n, p) = c.get(); ((n as u64) << 52) ^ p.to_bits() }) }
/// P(link k of an entry fires), k = 1..=stop, for every chain kind present, given the deck's chain kinds.
/// Dense, deterministic, memoised version of `chain_marginals_ref`. States are mixed-radix indices over the
/// entries' progress; every transition raises the index, so one pass in index order is a topological order.
/// Entries are sorted first so the floating-point summation order is canonical (the HashMap reference sums in
/// iteration order, which varied from run to run in the last bits).
fn chain_marginals(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    thread_local! { static CM: std::cell::RefCell<crate::FxMap<(Vec<Kind>, u64), HashMap<Kind, Vec<f64>>>> = std::cell::RefCell::new(Default::default()); }
    let mut ks: Vec<Kind> = kinds.to_vec(); ks.sort();
    let key = (ks, pmf_key());
    if let Some(r) = CM.with(|c| c.borrow().get(&key).cloned()) { return r; }
    let r = if key.0.iter().any(|k| k.2 != 0) { chain_marginals_risk(&key.0) } else { chain_marginals_dense(&key.0) };
    CM.with(|c| c.borrow_mut().insert(key, r.clone()));
    r
}
/// Kinds with a stop risk: the chance that IT's default choice ends a chain at a link is independent of how many
/// chain events the run gets, so the run is a mixture over where each risky entry would stop. Each realisation is a
/// plain kind (links, realised stop): the entry competes for events with its full chain until it stops, then frees
/// its remaining slots, exactly as a chosen early end does. Exact.
fn chain_marginals_risk(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    // per entry: [(realised stop, probability)]
    let alts: Vec<Vec<(u8, f64)>> = kinds.iter().map(|&(links, stop, risk)| {
        let mut v = vec![]; let mut go = 1.0f64;
        for l in 1..stop.min(5) {
            let b = (risk >> (8 * (l as u32 - 1))) & 0xff;
            if b == 0 { continue; }
            let q = (b >> 4) as f64 / (b & 15) as f64;
            v.push((l, go * q)); go *= 1.0 - q;
        }
        v.push((stop, go)); let _ = links; v
    }).collect();
    let n = kinds.len();
    let mut acc: Vec<Vec<f64>> = kinds.iter().map(|k| vec![0.0; k.1 as usize]).collect();
    let mut pick = vec![0usize; n];
    loop {
        let w: f64 = (0..n).map(|i| alts[i][pick[i]].1).product();
        if w > 0.0 {
            let real: Vec<Kind> = (0..n).map(|i| (kinds[i].0, alts[i][pick[i]].0, 0)).collect();
            let m = chain_marginals(&real);
            for i in 0..n { if let Some(v) = m.get(&real[i]) { for (k, x) in v.iter().enumerate() { acc[i][k] += w * x; } } }
        }
        let mut i = 0;
        while i < n { pick[i] += 1; if pick[i] < alts[i].len() { break; } pick[i] = 0; i += 1; }
        if i == n { break; }
    }
    let mut out: HashMap<Kind, Vec<f64>> = HashMap::new();
    for i in 0..n { out.entry(kinds[i]).or_insert_with(|| acc[i].clone()); }
    out
}
fn chain_marginals_dense(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    let n = kinds.len();
    let mut out: HashMap<Kind, Vec<f64>> = HashMap::new();
    if n == 0 { return out; }
    let emax = 60usize;
    let pois = event_pmf(emax);
    let total: usize = kinds.iter().map(|&(_, stop, _)| stop as usize).sum();
    let last = emax.min(total);
    let width = kinds.iter().map(|k| k.1 as usize).max().unwrap_or(0).max(1);
    let mut stride = vec![0usize; n]; let mut nst = 1usize;
    for i in 0..n { stride[i] = nst; nst *= kinds[i].1 as usize + 1; }
    let mut p = vec![0.0f64; nst]; p[0] = 1.0;
    // done[e][i][k] = P(after e events entry i has >= k+1 links done)
    let mut done = vec![0.0f64; (last + 1) * n * width];
    let mut s = vec![0u8; n]; let mut lvl = 0usize;
    for idx in 0..nst {
        let pi = p[idx];
        if pi != 0.0 && lvl <= last {
            if lvl >= 1 { let b = lvl * n * width; for i in 0..n { for k in 0..s[i] as usize { done[b + i * width + k] += pi; } } }
            if lvl < last {
                let mut wsum = 0.0f64;
                for i in 0..n { if s[i] < kinds[i].1 { wsum += (kinds[i].0 - s[i]) as f64; } }
                if wsum > 0.0 {
                    for i in 0..n { if s[i] < kinds[i].1 { let w = (kinds[i].0 - s[i]) as f64; p[idx + stride[i]] += pi * w / wsum; } }
                }
            }
        }
        // odometer increment of s, tracking the level
        for i in 0..n { if s[i] < kinds[i].1 { s[i] += 1; lvl += 1; break; } lvl -= s[i] as usize; s[i] = 0; }
    }
    let mut fired = vec![vec![0.0f64; width]; n];
    for e in 1..=last {
        let pe = pois[e] + if e == last { 1.0 - pois[..=e].iter().sum::<f64>() } else { 0.0 };
        let b = e * n * width;
        for i in 0..n { for k in 0..width { fired[i][k] += pe * done[b + i * width + k]; } }
    }
    for i in 0..n {
        let kd = kinds[i];
        out.entry(kd).or_insert_with(|| (0..kd.1 as usize).map(|k| fired[i][k]).collect());
    }
    out
}

/// P(link k of an entry fires), k = 1..=stop, for every chain kind present, given the deck's chain kinds
/// (reference implementation, for the tests).
#[cfg(test)]
fn chain_marginals_ref(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    let n = kinds.len();
    let mut out: HashMap<Kind, Vec<f64>> = HashMap::new();
    if n == 0 { return out; }
    let emax = 60usize;
    let pois = event_pmf(emax);
    let total: usize = kinds.iter().map(|&(_, stop, _)| stop as usize).sum();
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

/// One random event outcome per firing: (chance, target rewards (target, 1 white | 2 gold)) per outcome. The chances
/// sum to at most 1 (the rest gives nothing); a reward with chance p on its own is [(p, [(k, lvl)])].
type Group = Vec<(f64, Vec<(usize, u8)>)>;
/// Per target a group touches: (k, P(gold for k), P(any hint for k)).
fn group_gh(g: &Group) -> Vec<(usize, f64, f64)> {
    let mut v: Vec<(usize, f64, f64)> = vec![];
    for (w, out) in g {
        let mut seen: Vec<(usize, u8)> = vec![];
        for &(k, lvl) in out { match seen.iter_mut().find(|x| x.0 == k) { Some(x) => x.1 = x.1.max(lvl), None => seen.push((k, lvl)) } }
        for (k, lvl) in seen {
            let e = match v.iter().position(|x| x.0 == k) { Some(i) => &mut v[i], None => { v.push((k, 0.0, 0.0)); v.last_mut().unwrap() } };
            if lvl >= 2 { e.1 += w; }
            e.2 += w;
        }
    }
    v.sort_by_key(|x| x.0);
    v
}

/// Final-state distribution of the chain process behind `chain_marginals`: (links done per entry, probability).
/// Memoised per sorted kind list; `kinds` must be sorted. Exact: a state at level L (L events taken) is final
/// with P(N = L) unless every entry is at its stop, then with P(N >= L); stop-risk entries are the mixture over
/// where they stop, as in `chain_marginals_risk`.
fn chain_states(kinds: &[Kind]) -> std::rc::Rc<Vec<(Vec<u8>, f64)>> {
    thread_local! { static CS: std::cell::RefCell<crate::FxMap<(Vec<Kind>, u64), std::rc::Rc<Vec<(Vec<u8>, f64)>>>> = std::cell::RefCell::new(Default::default()); }
    let key = (kinds.to_vec(), pmf_key());
    if let Some(r) = CS.with(|c| c.borrow().get(&key).cloned()) { return r; }
    let n = kinds.len();
    let alts: Vec<Vec<(u8, f64)>> = kinds.iter().map(|&(_, stop, risk)| {
        let mut v = vec![]; let mut go = 1.0f64;
        for l in 1..stop.min(5) {
            let b = (risk >> (8 * (l as u32 - 1))) & 0xff;
            if b == 0 { continue; }
            let q = (b >> 4) as f64 / (b & 15) as f64;
            v.push((l, go * q)); go *= 1.0 - q;
        }
        v.push((stop, go)); v
    }).collect();
    let mut acc: crate::FxMap<Vec<u8>, f64> = Default::default();
    let mut pick = vec![0usize; n];
    loop {
        let w: f64 = (0..n).map(|i| alts[i][pick[i]].1).product();
        if w > 0.0 {
            let stops: Vec<u8> = (0..n).map(|i| alts[i][pick[i]].0).collect();
            let total: usize = stops.iter().map(|&x| x as usize).sum();
            let pois = event_pmf(total);
            let mut stride = vec![0usize; n]; let mut nst = 1usize;
            for i in 0..n { stride[i] = nst; nst *= stops[i] as usize + 1; }
            let mut p = vec![0.0f64; nst]; p[0] = 1.0;
            let mut s = vec![0u8; n];
            for idx in 0..nst {
                let mut r = idx; let mut lvl = 0usize;
                for i in 0..n { s[i] = (r % (stops[i] as usize + 1)) as u8; r /= stops[i] as usize + 1; lvl += s[i] as usize; }
                let pi = p[idx];
                if pi == 0.0 { continue; }
                let mut wsum = 0.0f64;
                for i in 0..n { if s[i] < stops[i] { wsum += (kinds[i].0 - s[i]) as f64; } }
                let fin = if wsum > 0.0 {
                    for i in 0..n { if s[i] < stops[i] { p[idx + stride[i]] += pi * (kinds[i].0 - s[i]) as f64 / wsum; } }
                    pois[lvl]
                } else { (1.0 - pois[..lvl].iter().sum::<f64>()).max(0.0) };
                *acc.entry(s.clone()).or_insert(0.0) += w * pi * fin;
            }
        }
        let mut i = 0;
        while i < n { pick[i] += 1; if pick[i] < alts[i].len() { break; } pick[i] = 0; i += 1; }
        if i == n { break; }
    }
    let mut v: Vec<(Vec<u8>, f64)> = acc.into_iter().collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    let r = std::rc::Rc::new(v);
    CS.with(|c| c.borrow_mut().insert(key, r.clone()));
    r
}

/// Exact chances that all targets land, and all but one, for trainee `u` with one deck, at each inheritance chance in
/// `qs`: (P(all), P(all but one), outfit option index in `model.opts[u]`) per q, with the outfit that gives the
/// higher P(all).
/// `ents`: the deck's entries (sources, chain kind); `filler`: cards without target hints, which still take chain
/// slots (as 2-link SRs). The per-target odds (`src_miss`) are the marginals of the same model; the metric the
/// search ranks by multiplies them as if targets were independent. They are not: chain cards share one run's
/// chain events (a run with many completes several chains, a short one starves them all), and one event's
/// outcomes can give one target or another. Here: the distribution of chain progress (`chain_states`); given it,
/// sources are independent, except outcome groups that touch two or more targets, whose outcomes are enumerated.
/// Training hints stay independent per target, as in the marginal model.
fn joint_trainee(model: &Model, u: usize, qs: &[f64], ents: &[(Src, Kind)], filler: usize, n_targets: usize) -> Vec<(f64, f64, usize)> {
    use crate::skills::GEN_WHITE;
    let nu = model.uncertain.len();
    // per-target miss factors (no gold, no hint) of the groups that touch one target; the others are "coupled"
    fn take<'g>(g: &'g Group, coupled: &mut Vec<&'g Group>, ng: &mut [f64], nh: &mut [f64]) {
        let gh = group_gh(g);
        if gh.len() >= 2 { coupled.push(g); return; }
        for (k, pg, ph) in gh { ng[k] *= 1.0 - pg; nh[k] *= 1.0 - ph; }
    }
    // sources that do not depend on the chain
    let (mut bg, mut bh) = (vec![1.0f64; nu], vec![1.0f64; nu]);
    let mut bc: Vec<&Group> = vec![];
    for (src, _) in ents {
        for &(k, p) in &src.train { bh[k] *= 1.0 - p; }
        for g in &src.fixed { take(g, &mut bc, &mut bg, &mut bh); }
    }
    // chain entries sorted by kind (with fillers) for the memoised state distribution; per entry and links done,
    // the cumulative factors and coupled groups of the links fired so far
    let mut ch: Vec<(Kind, Option<usize>)> = ents.iter().enumerate().filter(|(_, e)| e.1 != (0, 0, 0)).map(|(i, e)| (e.1, Some(i))).collect();
    for _ in 0..filler { ch.push(((2, 2, 0), None)); }
    ch.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let kinds: Vec<Kind> = ch.iter().map(|x| x.0).collect();
    let states = if kinds.is_empty() { std::rc::Rc::new(vec![(vec![], 1.0)]) } else { chain_states(&kinds) };
    let lv: Vec<Option<Vec<(Vec<f64>, Vec<f64>, Vec<&Group>)>>> = ch.iter().map(|&(kd, e)| e.map(|i| {
        let src = &ents[i].0;
        let (mut g, mut h, mut c) = (vec![1.0f64; nu], vec![1.0f64; nu], vec![]);
        let mut out = vec![(g.clone(), h.clone(), c.clone())];
        for l in 1..=kd.1 {
            for (cl, grp) in &src.chains { if *cl == l { take(grp, &mut c, &mut g, &mut h); } }
            out.push((g.clone(), h.clone(), c.clone()));
        }
        out
    })).collect();
    let opts = &model.opts[u];
    let no = opts.len(); let nq = qs.len();
    let nfix = model.n_fixed_gold as usize;
    let fixed = model.fixed_factor();
    let mut acc = vec![(0.0f64, 0.0f64); nq * no];
    let (mut ng, mut nh) = (vec![1.0f64; nu], vec![1.0f64; nu]);
    let mut coupled: Vec<&Group> = vec![];
    let mut alts: Vec<Vec<(f64, &[(usize, u8)])>> = vec![];
    let mut pick: Vec<usize> = vec![];
    let mut dist = [0.0f64; 40];
    for (prog, w) in states.iter() {
        ng.copy_from_slice(&bg); nh.copy_from_slice(&bh);
        coupled.clear(); coupled.extend(bc.iter().copied());
        for (pos, l) in lv.iter().enumerate() {
            let Some(l) = l else { continue };
            let (g, h, c) = &l[prog[pos] as usize];
            for k in 0..nu { ng[k] *= g[k]; nh[k] *= h[k]; }
            coupled.extend(c.iter().copied());
        }
        // outcomes of the coupled groups: each group's outcomes plus "nothing" (the rest of its weight)
        alts.clear();
        for g in &coupled {
            let mut v: Vec<(f64, &[(usize, u8)])> = g.iter().map(|(p, o)| (*p, &o[..])).collect();
            let rest = 1.0 - g.iter().map(|x| x.0).sum::<f64>();
            if rest > 1e-15 { v.push((rest, &[][..])); }
            alts.push(v);
        }
        pick.clear(); pick.resize(alts.len(), 0);
        loop {
            let wc: f64 = (0..alts.len()).map(|i| alts[i][pick[i]].0).product::<f64>() * w;
            let mut fg = [false; 32]; let mut fh = [false; 32];
            for i in 0..alts.len() { for &(k, lvl) in alts[i][pick[i]].1 { fh[k] = true; if lvl >= 2 { fg[k] = true; } } }
            for (qi, &q) in qs.iter().enumerate() {
                for (o, (st, _)) in opts.iter().enumerate() {
                    let ew = &model.ews[u][o];
                    // P(all land) and the distribution of how many of the uncertain targets land (fixed golds after)
                    let mut all = fixed;
                    dist[..=nu + nfix].iter_mut().for_each(|x| *x = 0.0); dist[0] = 1.0;
                    for k in 0..nu {
                        let pg = if fg[k] { 1.0 } else { 1.0 - ng[k] };
                        let pnh = if fh[k] { 0.0 } else { nh[k] };
                        let f = match st[k] { 2 => GEN_GOLD, 1 => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg),
                            _ => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg - pnh * (1.0 - ew[k]) * (1.0 - q)) };
                        all *= f;
                        for j in (0..=k).rev() { dist[j + 1] += dist[j] * f; dist[j] *= 1.0 - f; }
                    }
                    for _ in 0..nfix { for j in (0..=nu + nfix).rev() { if j + 1 < dist.len() { dist[j + 1] += dist[j] * GEN_GOLD; } dist[j] *= 1.0 - GEN_GOLD; } }
                    let m1: f64 = dist[n_targets.saturating_sub(1).min(nu + nfix)..=nu + nfix].iter().sum();
                    let a = &mut acc[qi * no + o];
                    a.0 += wc * all; a.1 += wc * m1;
                }
            }
            let mut i = 0;
            while i < alts.len() { pick[i] += 1; if pick[i] < alts[i].len() { break; } pick[i] = 0; i += 1; }
            if i == alts.len() { break; }
        }
    }
    (0..nq).map(|qi| {
        let mut b = 0;
        for o in 1..no { if acc[qi * no + o].0 > acc[qi * no + b].0 { b = o; } }
        (acc[qi * no + b].0, acc[qi * no + b].1, b)
    }).collect()
}

/// Target sources of one deck entry (a card, a card whose chain the player ends early, or a card taking a given
/// choice where an event offers targets on different choices): training hints (target, chance), outcome groups that
/// do not depend on the chain, and chain outcome groups with their link, sorted by link. `fm` / `cm` are the same
/// per target as miss factors (1 - P(gold), 1 - P(any hint)), the chain ones per link in link order.
#[derive(Clone, Default)]
struct Src { train: Vec<(usize, f64)>, fixed: Vec<Group>, chains: Vec<(u8, Group)>,
             fm: Vec<(usize, f64, f64)>, cm: Vec<(usize, Vec<(u8, f64, f64)>)> }
impl Src {
    fn finish(mut self) -> Self {
        self.chains.sort_by_key(|x| x.0);
        self.fm = self.fixed.iter().flat_map(|g| group_gh(g)).map(|(k, g, h)| (k, 1.0 - g, 1.0 - h)).collect();
        let mut cm: Vec<(usize, Vec<(u8, f64, f64)>)> = vec![];
        for (l, g) in &self.chains {
            for (k, pg, ph) in group_gh(g) {
                let e = match cm.iter().position(|x| x.0 == k) { Some(i) => &mut cm[i].1, None => { cm.push((k, vec![])); &mut cm.last_mut().unwrap().1 } };
                match e.last_mut() { Some(x) if x.0 == *l => { x.1 *= 1.0 - pg; x.2 *= 1.0 - ph; } _ => e.push((*l, 1.0 - pg, 1.0 - ph)) }
            }
        }
        cm.sort_by_key(|x| x.0);
        self.cm = cm;
        self
    }
    fn is_empty(&self) -> bool { self.train.is_empty() && self.fixed.is_empty() && self.chains.is_empty() }
}

/// Multiply an entry's miss chances into mw / mg; m[l - 1] = P(chain link l fires). mg gets P(no gold hint) and mw
/// P(no white hint | no gold hint) = P(no hint) / P(no gold hint), both exact for the entry: its sources are
/// independent of each other, while a skill's gold and white from one event (e.g. on different outcomes) are not.
/// Links fire in order, so a skill offered at several links comes from the earliest one that fires:
/// P(miss) = 1 - sum_l P(link l fires) * (P(not from links < l) - P(not from links <= l)).
/// Both misses fall with every m[l], so the lowest possible link odds bound them from above.
fn src_miss(s: &Src, m: &[f64], mw: &mut [f64], mg: &mut [f64]) {
    let mut eg = [1.0f64; 32]; let mut eh = [1.0f64; 32];
    for &(k, p) in &s.train { eh[k] *= 1.0 - p; }
    for &(k, ng, nh) in &s.fm { eg[k] *= ng; eh[k] *= nh; }
    for (k, links) in &s.cm {
        let (mut cg, mut ch, mut xg, mut xh) = (1.0f64, 1.0f64, 1.0f64, 1.0f64);
        for &(l, ng, nh) in links {
            let ml = m.get(l as usize - 1).copied().unwrap_or(0.0);
            let (ncg, nch) = (cg * ng, ch * nh);
            xg -= ml * (cg - ncg); xh -= ml * (ch - nch); cg = ncg; ch = nch;
        }
        eg[*k] *= xg; eh[*k] *= xh;
    }
    for k in 0..mw.len() { mg[k] *= eg[k]; mw[k] *= if eg[k] > 0.0 { eh[k] / eg[k] } else { 1.0 }; }
}

/// Merge a card-event cache (card_events.json) into a deck job: skill names -> (group, level) via the skills table.
pub(crate) fn apply_events_db(v: &mut Value, sk: &SkillsRaw, db: &Value) {
    let look = |name: &str| sk.skills.iter().find_map(|(g, w, gd)| if w == name { Some((*g, 1u8)) } else if gd.as_deref() == Some(name) { Some((*g, 2u8)) } else { None });
    let mut ev: Vec<Value> = v["events"].as_array().cloned().unwrap_or_default();
    let mut evp: Vec<Value> = v["events_p"].as_array().cloned().unwrap_or_default();
    let mut od: Vec<Value> = v["odds_events"].as_array().cloned().unwrap_or_default();
    let mut el: Vec<Value> = v["end_links"].as_array().cloned().unwrap_or_default();
    let mut en: Vec<Value> = v["ends"].as_array().cloned().unwrap_or_default();
    let mut sr: Vec<Value> = v["stop_risk"].as_array().cloned().unwrap_or_default();
    let mut al: Vec<Value> = v["alts"].as_array().cloned().unwrap_or_default();
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
            for r in c["stop_risk"].as_array().into_iter().flatten() { sr.push(json!([id, r[0], r[1], r[2]])); }
            for a in c["alts"].as_array().into_iter().flatten() {
                let chs: Vec<Vec<Vec<(u32, u8)>>> = a[1].as_array().into_iter().flatten().map(|ch| ch.as_array().into_iter().flatten()
                    .map(|out| out.as_array().into_iter().flatten().filter_map(|n| look(n.as_str().unwrap_or(""))).collect()).collect()).collect();
                al.push(json!([id, a[0], chs]));
            }
            for n in c["always"].as_array().into_iter().flatten() { if let Some((g, lv)) = look(n.as_str().unwrap_or("")) { ev.push(json!([id, g, lv, 0])); } }
            for e in c["chance"].as_array().into_iter().flatten() { if let Some((g, lv)) = look(e[0].as_str().unwrap_or("")) { od.push(json!([id, g, lv, e[1].as_f64().unwrap_or(0.0)])); } }
        }
    }
    v["events"] = Value::Array(ev); v["events_p"] = Value::Array(evp); v["odds_events"] = Value::Array(od);
    v["end_links"] = Value::Array(el); v["ends"] = Value::Array(en); v["stop_risk"] = Value::Array(sr); v["alts"] = Value::Array(al);
}

/// The search ranks decks and loops by the product of per-target odds; the reported ones are then scored exactly
/// (`joint_trainee`), and so are the decks and loops whose ranking value is at least this share of the best. On the
/// reference setups the exact value was 1.00-1.19 times the product (the most for 0-1-pal decks), so 0.8 covers a
/// gap well beyond that.
const JOINT_BAND: f64 = 0.8;
thread_local! { static JCOUNT: std::cell::Cell<usize> = std::cell::Cell::new(0); }

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

const NBUCKET: usize = 128;

/// Decks whose coordinate sums fall in one slice of [0, nd]: flat arrays (entry and query signatures,
/// coordinates, ids) plus a level index of the entry signatures: for each word of 64 decks and each signature
/// field f and level l >= 1, the bits of the decks whose field f is at least l. ANDing one word per field of a
/// query gives the decks that pass the signature test, 64 at a time; they are then visited in index order, the
/// same order as a plain scan.
#[derive(Default)]
/// `lq` is the mirror index of the query signatures (bit set when field f is at most l, l < LEVELS - 1), for the
/// removal test "x's entry signature over the deck's query one".
struct Bucket { list: Vec<DeckEval>, esig: Vec<Sig>, qsig: Vec<Sig>, flat: Vec<f64>, uid: Vec<u64>, lv: Vec<u64>, lq: Vec<u64> }
const LVW: usize = SIG_FIELDS * (LEVELS as usize - 1);   // index words per 64 decks
impl Bucket {
    #[inline]
    fn index_set(&mut self, i: usize, e: Sig, q: Sig) {
        let w = i / 64;
        if self.lv.len() < (w + 1) * LVW { self.lv.resize((w + 1) * LVW, 0); self.lq.resize((w + 1) * LVW, 0); }
        let bit = 1u64 << (i % 64);
        let base = w * LVW;
        const L1: usize = LEVELS as usize - 1;
        for f in 0..SIG_FIELDS {
            let l = ((e >> (FB * f)) & (LEVELS - 1)) as usize;
            for k in 0..l { self.lv[base + f * L1 + k] |= bit; }
            let l = ((q >> (FB * f)) & (LEVELS - 1)) as usize;
            for k in l..L1 { self.lq[base + f * L1 + k] |= bit; }
        }
    }
    /// rebuild the level index from deck `from` on (after the arrays were compacted)
    fn reindex_from(&mut self, from: usize) {
        let w0 = from / 64;
        self.lv.truncate((w0 + 1) * LVW); self.lq.truncate((w0 + 1) * LVW);
        if self.lv.len() == (w0 + 1) * LVW {
            let keep = if from % 64 == 0 { 0 } else { u64::MAX >> (64 - from % 64) };
            for x in &mut self.lv[w0 * LVW..] { *x &= keep; }
            for x in &mut self.lq[w0 * LVW..] { *x &= keep; }
        }
        if self.esig.len() <= w0 * 64 { self.lv.truncate(w0 * LVW); self.lq.truncate(w0 * LVW); }
        for i in from..self.esig.len() { let (e, q) = (self.esig[i], self.qsig[i]); self.index_set(i, e, q); }
    }
    /// decks of word w that pass the signature test for the query whose level-index offsets are `qo`
    #[inline]
    fn cand_word(&self, w: usize, qo: &[usize]) -> u64 {
        let n = self.esig.len() - w * 64;
        let mut m = if n >= 64 { u64::MAX } else { (1u64 << n) - 1 };
        let base = w * LVW;
        for &o in qo { m &= self.lv[base + o]; if m == 0 { break; } }
        m
    }
}
/// Level-index offsets of a query signature: one per field whose level is at least 1.
fn query_offsets(q: Sig, out: &mut [usize; SIG_FIELDS]) -> usize {
    let mut n = 0;
    for f in 0..SIG_FIELDS {
        let l = ((q >> (FB * f)) & (LEVELS - 1)) as usize;
        if l > 0 { out[n] = f * (LEVELS as usize - 1) + l - 1; n += 1; }
    }
    n
}

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
        let mut qo = [0usize; SIG_FIELDS]; let nq = query_offsets(xq, &mut qo);
        for bi in (b0..NBUCKET).rev() {
            let bk = &self.buckets[bi];
            for w in 0..bk.esig.len().div_ceil(64) {
                let mut cm = bk.cand_word(w, &qo[..nq]);
                while cm != 0 {
                    let i = w * 64 + cm.trailing_zeros() as usize; cm &= cm - 1;
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
        let mut qo = [0usize; SIG_FIELDS]; let nq = query_offsets(xq, &mut qo);
        for bi in (b0..NBUCKET).rev() {
            let bk = &self.buckets[bi];
            for w in 0..bk.esig.len().div_ceil(64) {
                let mut cm = bk.cand_word(w, &qo[..nq]);
                while cm != 0 {
                    let i = w * 64 + cm.trailing_zeros() as usize; cm &= cm - 1;
                    if dominates(&bk.flat[i * nd..(i + 1) * nd], x) { n += 1; if n >= need { return n; } }
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
        bk.index_set(i, e, q);
        bk.esig.push(e); bk.qsig.push(q); bk.flat.extend_from_slice(&x.dv); bk.uid.push(id); bk.list.push(x);
        self.n += 1;
    }
    /// drop kept decks that x dominates (only decks with sum <= s + slack can be), in place, keeping the order of
    /// the rest
    fn remove_dominated_by(&mut self, x: &[f64], xe: Sig, s: f64) {
        let nd = self.nd;
        let top = self.bucket_of(s + SUM_EPS);
        // lq offsets of x's entry signature: fields below the top level constrain the deck's query field
        let mut xo = [0usize; SIG_FIELDS]; let mut nx = 0;
        for f in 0..SIG_FIELDS {
            let l = ((xe >> (FB * f)) & (LEVELS - 1)) as usize;
            if l < LEVELS as usize - 1 { xo[nx] = f * (LEVELS as usize - 1) + l; nx += 1; }
        }
        for bi in 0..=top {
            let bk = &mut self.buckets[bi];
            let len = bk.qsig.len();
            // the first deck x dominates: entry test (x's entry signature over the deck's query one) first
            let mut first = None;
            'scan: for start in (0..len).step_by(64) {
                let n = len - start;
                let mut m = if n >= 64 { u64::MAX } else { (1u64 << n) - 1 };
                let base = start / 64 * LVW;
                for &o in &xo[..nx] { if m == 0 { break; } m &= bk.lq[base + o]; }
                while m != 0 {
                    let i = start + m.trailing_zeros() as usize; m &= m - 1;
                    if dominates(x, &bk.flat[i * nd..(i + 1) * nd]) { first = Some(i); break 'scan; }
                }
            }
            let Some(f) = first else { continue };
            // compact [f, len): kept decks move down in order, dropped ones end up past `w` and are truncated
            let mut w = f;
            for r in f + 1..len {
                if sig_ge(xe, bk.qsig[r]) && dominates(x, &bk.flat[r * nd..(r + 1) * nd]) { continue; }
                bk.list.swap(w, r);
                bk.esig[w] = bk.esig[r]; bk.qsig[w] = bk.qsig[r]; bk.uid[w] = bk.uid[r];
                bk.flat.copy_within(r * nd..(r + 1) * nd, w * nd);
                w += 1;
            }
            self.n -= len - w;
            bk.list.truncate(w); bk.esig.truncate(w); bk.qsig.truncate(w); bk.uid.truncate(w); bk.flat.truncate(w * nd);
            bk.reindex_from(f);
        }
    }
    fn into_list(self) -> Vec<DeckEval> { self.buckets.into_iter().flat_map(|b| b.list).collect() }
}

/// The order k_filter(list, 1, nd).into_list() produces when it keeps every deck: stable by
/// (bucket of the sum ascending, sum descending), ties in input order.
fn kfilter_order(mut g: Vec<DeckEval>, nd: usize) -> Vec<DeckEval> {
    g.sort_by(|a, b| b.s.partial_cmp(&a.s).unwrap());
    let kept = Kept::new(nd);
    g.sort_by_key(|x| kept.bucket_of(x.s));
    g
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
/// Deck frontiers already enumerated inside one `run_deck` (its branches change the uma pool, which the
/// enumeration does not depend on: the pool only orders the plain pals, a display matter), keyed by the job
/// without its uma-pool fields. Each entry: the frontier and the counters the output reports.
type EnumEntry = (String, Vec<DeckEval>, usize, usize);
thread_local! { static ENUM_CACHE: std::cell::RefCell<Option<Vec<EnumEntry>>> = const { std::cell::RefCell::new(None) }; }
struct EnumScope(Option<Vec<EnumEntry>>);
impl EnumScope { fn new() -> Self { EnumScope(ENUM_CACHE.with(|c| c.borrow_mut().replace(vec![]))) } }
impl Drop for EnumScope { fn drop(&mut self) { let prev = self.0.take(); ENUM_CACHE.with(|c| *c.borrow_mut() = prev); } }
fn enum_key(v: &Value) -> String {
    let mut k = v.clone();
    if let Some(o) = k.as_object_mut() { for f in ["exclude", "require", "owned"] { o.remove(f); } }
    k.to_string()
}

#[derive(Clone)]
struct DeckEval { pals: usize, borrowed: Option<usize>, cards: Vec<usize>, ends: Vec<u8>, vars: Vec<u32>, plain: usize, filler: usize, dw: Vec<f64>, dg: Vec<f64>, dv: Vec<f64>, s: f64 }

/// A deck's per-target (white, gold) hint odds.
trait Odds { fn dw(&self) -> &[f64]; fn dg(&self) -> &[f64]; }
impl Odds for DeckEval { fn dw(&self) -> &[f64] { &self.dw } fn dg(&self) -> &[f64] { &self.dg } }
impl Odds for PDeck { fn dw(&self) -> &[f64] { &self.dw } fn dg(&self) -> &[f64] { &self.dg } }
/// A cluster of decks (indices into the list it was built from) with their componentwise maximum odds.
struct Cl { mdw: Vec<f64>, mdg: Vec<f64>, mem: Vec<usize> }
/// Clusters of at most 16 decks by recursive median splits on the widest odds coordinate.
fn split<D: Odds>(sky: &[D], ix: &mut [usize], nu: usize, out: &mut Vec<Cl>) {
    if ix.len() <= 16 {
        let mut mdw = vec![0.0f64; nu]; let mut mdg = vec![0.0f64; nu];
        for &j in ix.iter() { for k in 0..nu { mdw[k] = mdw[k].max(sky[j].dw()[k]); mdg[k] = mdg[k].max(sky[j].dg()[k]); } }
        out.push(Cl { mdw, mdg, mem: ix.to_vec() });
        return;
    }
    let coord = |j: usize, c: usize| if c < nu { sky[j].dw()[c] } else { sky[j].dg()[c - nu] };
    let mut lo = vec![f64::INFINITY; 2 * nu]; let mut hi = vec![f64::NEG_INFINITY; 2 * nu];
    for &j in ix.iter() { for k in 0..nu {
        lo[k] = lo[k].min(sky[j].dw()[k]); hi[k] = hi[k].max(sky[j].dw()[k]);
        lo[nu + k] = lo[nu + k].min(sky[j].dg()[k]); hi[nu + k] = hi[nu + k].max(sky[j].dg()[k]);
    } }
    let c = (0..2 * nu).max_by(|&a, &b| (hi[a] - lo[a]).partial_cmp(&(hi[b] - lo[b])).unwrap()).unwrap();
    let mid = ix.len() / 2;
    ix.select_nth_unstable_by(mid, |&a, &b| coord(a, c).partial_cmp(&coord(b, c)).unwrap());
    let (l, r) = ix.split_at_mut(mid);
    split(sky, l, nu, out); split(sky, r, nu, out);
}

/// A deck detached from the search that built it (names and ids instead of indices), for per-uma mode.
#[derive(Clone, Serialize, Deserialize)]
struct PDeck { names: Vec<String>, ids: Vec<u32>, ends: Vec<u8>, #[serde(default)] vars: Vec<u32>, #[serde(default)] choices: Vec<String>, plain_ids: Vec<u32>, pal_names: Vec<String>, pals: usize, borrowed: Option<String>, dw: Vec<f64>, dg: Vec<f64> }
impl PDeck {
    fn out(&self, unc: &[u32]) -> Value {
        let odds: Vec<Value> = (0..unc.len()).map(|k| json!({"group": unc[k], "white": self.dw[k], "gold": self.dg[k]})).collect();
        json!({"pals": self.pals, "cards": self.names, "odds": odds, "pal_cards": self.pal_names, "borrowed": self.borrowed,
               "card_ids": self.ids, "end_chain_at": self.ends, "choices": self.choices, "plain_ids": self.plain_ids})
    }
    fn blocks(&self, uma: u32, card_of: &HashMap<u32, &Card>) -> bool {
        self.ids.iter().chain(&self.plain_ids).any(|id| card_of.get(id).map_or(false, |c| c.blocks(uma)))
    }
}

/// Order for exact ties between deck entries: card order, except that one character's entries take her positions in
/// order of rarity (highest first; stable), so an SSR wins an exact tie with her own R card ([Planned Perfection] over
/// [Tracen Academy] Riko Kashimoto, the same event) and ties between different characters keep card order.
fn tie_rank(rel: &[usize], cards: &[&Card]) -> Vec<usize> {
    let mut rank: Vec<usize> = (0..rel.len()).collect();
    let mut by: Vec<(u32, Vec<usize>)> = vec![];
    for (j, &r) in rel.iter().enumerate() {
        match by.iter_mut().find(|x| x.0 == cards[r].chara) { Some(x) => x.1.push(j), None => by.push((cards[r].chara, vec![j])) }
    }
    for (_, js) in &by {
        let mut s = js.clone(); s.sort_by_key(|&j| std::cmp::Reverse(cards[rel[j]].rarity));
        for (&pos, &j) in js.iter().zip(&s) { rank[j] = pos; }
    }
    rank
}

/// Builds a deck entry's target sources and chain kind from the deck job; shared by the deck search and the exact
/// evaluation of the decks it reports.
struct Sources<'a> { dj: &'a DeckJob, unc: &'a [u32] }
impl<'a> Sources<'a> {
    fn ev_for(&self, id: u32) -> Vec<(usize, u8, u8, f64)> {
        let unc = self.unc;
        let mut v: Vec<(usize, u8, u8, f64)> = self.dj.events.iter().filter(|e| e.0 == id).filter_map(|e| unc.iter().position(|&g| g == e.1).map(|k| (k, e.2, e.3, 1.0))).collect();
        v.extend(self.dj.events_p.iter().filter(|e| e.0 == id).filter_map(|e| unc.iter().position(|&g| g == e.1).map(|k| (k, e.2, e.3, e.4))));
        v
    }
    fn odds_for(&self, id: u32) -> Vec<(usize, u8, f64)> { self.dj.odds_events.iter().filter(|e| e.0 == id).filter_map(|e| self.unc.iter().position(|&g| g == e.1).map(|k| (k, e.2, e.3))).collect() }
    /// Training-hint draws per run: base(scenario) x (1 + hint frequency / 100), each uniform over the card's skill
    /// options still below Lv 5. Per target in the pool: P(hinted) = 1 - hint_miss(1).
    fn draws(&self, c: &Card) -> f64 { hint_base(&self.dj.scenario) * (1.0 + c.hf / 100.0) }
    fn train(&self, c: &Card) -> Vec<(usize, f64)> {
        if c.nskill == 0 { return vec![]; }
        let p = 1.0 - hint_miss(1, c.nskill as usize, self.draws(c), c.hinc);
        self.unc.iter().enumerate().filter(|(_, g)| c.whites.contains(g)).map(|(k, _)| (k, p)).collect()
    }
    /// A card's training hints over two or more of its targets as one outcome group: one outcome per non-empty set S
    /// of targets hinted, P(exactly S) = sum over U in S of (-1)^(|S|-|U|) hint_miss(m - |U|) (m targets in the
    /// pool), so the draws that hint one target cannot also hint another. Each target's own chance is unchanged.
    fn train_group(&self, c: &Card, tr: &[(usize, f64)]) -> Group {
        let (m, n, h) = (tr.len(), c.nskill as usize, self.draws(c));
        let miss = |unhit: usize| hint_miss(unhit, n, h, c.hinc);   // P(none of `unhit` given targets hinted)
        let mut g: Group = vec![];
        for sm in 1u32..(1 << m) {
            let mut w = 0.0f64; let ssz = sm.count_ones() as i32;
            let mut u = sm;
            loop {                                                    // every subset U of S (including the empty set)
                let usz = u.count_ones() as i32;
                w += if (ssz - usz) % 2 == 0 { 1.0 } else { -1.0 } * miss(m - usz as usize);
                if u == 0 { break; }
                u = (u - 1) & sm;
            }
            g.push((w.max(0.0), (0..m).filter(|i| sm >> i & 1 == 1).map(|i| (tr[i].0, 1u8)).collect()));
        }
        g
    }
    /// early-end options: links where a card has a choice that ends its chain, and the target rewards of that choice
    fn end_links(&self, id: u32) -> Vec<u8> { let mut v: Vec<u8> = self.dj.end_links.iter().filter(|e| e.0 == id).map(|e| e.1).collect(); v.sort(); v.dedup(); v }
    fn end_rew(&self, id: u32, l: u8) -> Vec<(usize, u8, f64)> { self.dj.ends.iter().filter(|e| e.0 == id && e.1 == l).filter_map(|e| self.unc.iter().position(|&g| g == e.2).map(|k| (k, e.3, e.4))).collect() }
    /// The choices worth taking at an event listed in `alts` (None: not listed), as outcome groups over the targets:
    /// those with a target that no other choice beats on every target (P(gold) and P(any hint)); of two with the same
    /// odds, both are kept unless their outcomes are the same (then the first).
    fn choices(&self, id: u32, link: u8) -> Option<Vec<Group>> {
        let a = self.dj.alts.iter().find(|a| a.0 == id && a.1 == link)?;
        let nu = self.unc.len();
        let gs: Vec<Group> = a.2.iter().map(|ch| {
            let n = ch.len().max(1) as f64;
            ch.iter().map(|out| {
                let mut r: Vec<(usize, u8)> = out.iter().filter_map(|&(g, lv)| self.unc.iter().position(|&x| x == g).map(|k| (k, lv))).collect();
                r.sort(); r.dedup(); (1.0 / n, r)
            }).filter(|(_, r)| !r.is_empty()).collect()
        }).collect();
        let prof: Vec<Vec<(f64, f64)>> = gs.iter().map(|g| { let mut p = vec![(0.0, 0.0); nu]; for (k, pg, ph) in group_gh(g) { p[k] = (pg, ph); } p }).collect();
        let beats = |j: usize, i: usize| prof[j].iter().zip(&prof[i]).all(|(a, b)| a.0 >= b.0 && a.1 >= b.1) && (prof[j] != prof[i] || (gs[j] == gs[i] && j < i));
        Some((0..gs.len()).filter(|&i| !gs[i].is_empty() && !(0..gs.len()).any(|j| j != i && !gs[j].is_empty() && beats(j, i))).map(|i| gs[i].clone()).collect())
    }
    fn in_range(link: u8, stop: Option<u8>) -> bool { match stop { Some(st) => link < st, None => true } }
    /// links (before the entry's stop) where two or more choices are worth taking, with how many
    fn var_links(&self, c: &Card, stop: Option<u8>) -> Vec<(u8, usize)> {
        if c.pal { return vec![]; }
        let mut v: Vec<(u8, usize)> = self.dj.alts.iter().filter(|a| a.0 == c.id && Self::in_range(a.1, stop))
            .filter_map(|a| { let n = self.choices(c.id, a.1).map_or(0, |x| x.len()); (n >= 2).then_some((a.1, n)) }).collect();
        v.sort(); v.dedup(); v
    }
    /// The entry's choice variants: the choice taken at each `var_links` link, 4 bits per link in link order.
    fn variants(&self, c: &Card, stop: Option<u8>) -> Vec<u32> {
        let mut out = vec![0u32];
        for (pos, &(_, n)) in self.var_links(c, stop).iter().enumerate() {
            out = out.iter().flat_map(|&v| (0..n.min(16)).map(move |i| v | ((i as u32) << (4 * pos)))).collect();
        }
        out
    }
    fn has_alts(&self, id: u32) -> bool { self.dj.alts.iter().any(|a| a.0 == id && self.choices(id, a.1).map_or(false, |x| !x.is_empty())) }
    /// Sources of a card played normally (stop = None) or ending its chain at link `stop` (links before it as usual,
    /// then the ending choice's rewards at that link), taking choice variant `var`.
    fn src(&self, c: &Card, stop: Option<u8>, var: u32) -> Src {
        let one = |k: usize, lvl: u8, p: f64| -> Group { vec![(p, vec![(k, lvl)])] };
        let mut s = Src::default();
        s.fixed = self.odds_for(c.id).into_iter().map(|(k, lvl, p)| one(k, lvl, p)).collect();
        let tr = self.train(c);
        if tr.len() >= 2 { s.fixed.push(self.train_group(c, &tr)); } else { s.train = tr; }
        let alt_links: Vec<u8> = self.dj.alts.iter().filter(|a| a.0 == c.id).map(|a| a.1).collect();
        for (k, lvl, link, f) in self.ev_for(c.id) {
            if link == 0 { s.fixed.push(one(k, lvl, f)); continue; }
            if c.pal || alt_links.contains(&link) || !Self::in_range(link, stop) { continue; }   // pals have no chain slots
            s.chains.push((link, one(k, lvl, f)));
        }
        if !c.pal {
            let vl = self.var_links(c, stop);
            let mut ls = alt_links.clone(); ls.sort(); ls.dedup();
            for l in ls {
                if !Self::in_range(l, stop) { continue; }
                let ch = self.choices(c.id, l).unwrap_or_default();
                let i = vl.iter().position(|x| x.0 == l).map_or(0, |pos| ((var >> (4 * pos)) & 15) as usize);
                if let Some(g) = ch.get(i) { s.chains.push((l, g.clone())); }
            }
        }
        if let Some(st) = stop { for (k, lvl, f) in self.end_rew(c.id, st) { s.chains.push((st, one(k, lvl, f))); } }
        s.finish()
    }
    fn kind(&self, c: &Card, stop: Option<u8>) -> Kind {
        if c.pal || c.links == 0 { return (0, 0, 0); }
        let risk: Vec<(u8, u8, u8)> = self.dj.stop_risk.iter().filter(|r| r.0 == c.id).map(|r| (r.1, r.2, r.3)).collect();
        let st = stop.unwrap_or(c.links);
        (c.links, st, stop_risk_code(&risk, st))
    }
    /// "link 1: Uma Stan" for each link where the variant picks between choices ("" when there is none)
    fn label(&self, c: &Card, stop: Option<u8>, var: u32, sk: &SkillsRaw) -> String {
        let name = |k: usize, lvl: u8| sk.skills.iter().find(|x| x.0 == self.unc[k]).map(|x| if lvl >= 2 { x.2.clone().unwrap_or(x.1.clone()) } else { x.1.clone() }).unwrap_or_default();
        self.var_links(c, stop).iter().enumerate().map(|(pos, &(l, _))| {
            let i = ((var >> (4 * pos)) & 15) as usize;
            let g = self.choices(c.id, l).unwrap_or_default().get(i).cloned().unwrap_or_default();
            let mut ns: Vec<String> = vec![];
            for (_, out) in &g { for &(k, lvl) in out { let n = name(k, lvl); if !ns.contains(&n) { ns.push(n); } } }
            format!("link {}: {}", l, ns.join(", "))
        }).collect::<Vec<_>>().join("; ")
    }
}

/// Deck + loop search with the character rule: a trainee cannot run a card of her own character, nor a group card
/// she is a member of (`Card::blocks`).
///
/// `run_deck_relaxed` ignores the rule. Its answer is exact whenever every returned (deck, loop) obeys it; otherwise
/// the search space is split on the first offending trainee u and both halves are searched again:
///   1. loops without u (u added to `exclude`), every deck allowed;
///   2. loops with u (u added to `require`), decks without any card that blocks u (added to `exclude_cards`).
/// Every legal (deck, loop) lies in exactly one half, so recursing until each half's answer is legal and merging
/// (a deck keeps its best legal loop) gives the exact top `top_decks`. A half whose relaxed best cannot beat the
/// current k-th legal result is not split further: the relaxed value bounds every legal value inside it.
pub(crate) fn run_deck(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    if v["per_uma"].as_bool() == Some(true) { return run_deck_relaxed(d, sk, cr, v); }   // the rule is applied per uma there
    if std::env::var("DECK_RELAXED").is_ok() { return run_deck_relaxed(d, sk, cr, v); }   // test hook: ignore the rule
    let k = v["top_decks"].as_u64().unwrap_or(1).max(1) as usize;
    let ids = |x: &Value| -> Vec<u32> { x.as_array().map(|a| a.iter().filter_map(|e| e.as_u64().map(|n| n as u32)).collect()).unwrap_or_default() };
    let card_of: HashMap<u32, &Card> = cr.cards.iter().map(|c| (c.id, c)).collect();
    let locked = ids(&v["lock_cards"]);
    // the first trainee in `t`'s loop that one of its deck's cards blocks
    let offender = |t: &Value| -> Option<u32> {
        let deck: Vec<u32> = ids(&t["deck"]["card_ids"]).into_iter().chain(ids(&t["deck"]["plain_ids"])).collect();
        t["loop"]["members"].as_array()?.iter().filter_map(|m| m["id"].as_u64().map(|n| n as u32))
            .find(|&u| deck.iter().any(|id| card_of.get(id).map_or(false, |c| c.blocks(u))))
    };
    let deck_key = |t: &Value| -> String { let mut n: Vec<String> = t["cards"].as_array().map(|a| a.iter().map(|x| x.to_string()).collect()).unwrap_or_default(); n.sort(); n.join("|") };
    // the search's ranking value (product of per-target odds) drives the split; the result is ordered by exact odds
    let pv = |t: &Value| t["loop"]["rank_value"].as_f64().or(t["loop"]["p_all"].as_f64()).unwrap_or(0.0);
    let pj = |t: &Value| t["loop"]["p_all"].as_f64().unwrap_or(0.0);

    let mut legal: Vec<(String, Value)> = vec![];                 // distinct decks, best legal loop each
    let mut by_pals: HashMap<u64, Value> = HashMap::new();
    let mut root: Option<Value> = None;
    let (mut searches, mut decks_built, mut proven) = (0usize, 0u64, true);
    let mut stack: Vec<Value> = vec![v.clone()];
    let _enum_cache = EnumScope::new();
    while let Some(job) = stack.pop() {
        let out = run_deck_relaxed(d, sk, cr, &job);
        if out.get("error").is_some() { if root.is_none() { return out; } continue; }   // e.g. a half with < 4 umas
        searches += 1;
        decks_built += out["decks_built"].as_u64().unwrap_or(0);
        proven &= out["proven"].as_bool().unwrap_or(false);
        let top = out["top"].as_array().cloned().unwrap_or_default();
        for b in out["best"].as_array().into_iter().flatten() {
            if offender(b).is_some() { continue; }
            let p = b["pals"].as_u64().unwrap_or(0);
            if by_pals.get(&p).map_or(true, |o| pv(o) < pv(b)) { by_pals.insert(p, b.clone()); }
        }
        if root.is_none() { root = Some(out); }
        let kth = |legal: &Vec<(String, Value)>| if legal.len() >= k { let mut x: Vec<f64> = legal.iter().map(|e| pv(&e.1)).collect(); x.sort_by(|a, b| b.partial_cmp(a).unwrap()); x[k - 1] } else { f64::NEG_INFINITY };
        if top.first().map_or(true, |t| pv(t) <= kth(&legal)) { continue; }
        let mut split = None;
        for t in &top {
            if let Some(u) = offender(t) { split = Some(u); break; }
            let key = deck_key(&t["deck"]);
            match legal.iter_mut().find(|e| e.0 == key) {
                Some(e) => { if pv(&e.1) < pv(t) { e.1 = t.clone(); } }
                None => legal.push((key, t.clone())),
            }
        }
        let Some(u) = split else { continue };
        let require = ids(&job["require"]);
        let blockers: Vec<u32> = cr.cards.iter().filter(|c| c.blocks(u)).map(|c| c.id).collect();
        if !require.contains(&u) {
            let mut a = job.clone();
            let mut ex = ids(&job["exclude"]); ex.push(u); a["exclude"] = json!(ex);
            stack.push(a);
        }
        if require.len() < 4 || require.contains(&u) {
            if !blockers.iter().any(|b| locked.contains(b)) {
                let mut b = job.clone();
                let mut rq = require.clone(); if !rq.contains(&u) { rq.push(u); } b["require"] = json!(rq);
                let mut ex = ids(&job["exclude_cards"]); for c in &blockers { if !ex.contains(c) { ex.push(*c); } } b["exclude_cards"] = json!(ex);
                stack.push(b);
            }
        }
    }
    let mut out = root.unwrap_or_else(|| json!({"error": "no search ran"}));
    legal.sort_by(|a, b| pv(&b.1).partial_cmp(&pv(&a.1)).unwrap());
    let mut top: Vec<Value> = legal.into_iter().take(k).map(|e| e.1).collect();
    top.sort_by(|a, b| pj(b).partial_cmp(&pj(a)).unwrap());
    out["top"] = Value::Array(top);
    let mut best: Vec<Value> = by_pals.into_values().collect();
    best.sort_by(|a, b| pj(b).partial_cmp(&pj(a)).unwrap());
    out["best"] = Value::Array(best);
    out["decks_built"] = json!(decks_built);
    out["searches"] = json!(searches);
    out["proven"] = json!(proven);
    out
}

fn run_deck_relaxed(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    let sj: SkillJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    let dj: DeckJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    CHAIN_EVENTS.with(|c| c.set(dj.chain_events.unwrap_or((CHAIN_N, CHAIN_P))));
    let model = Model::new(d, sk, &sj);
    let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
    let nu = unc.len();
    let is_owned = |id: u32| dj.owned_cards.is_empty() || dj.owned_cards.contains(&id) || dj.lock_cards.contains(&id);
    let owned_c = |id: u32| (is_owned(id) || dj.borrow) && !dj.exclude_cards.contains(&id);

    // ---- card sources ----
    let srcb = Sources { dj: &dj, unc: &unc };
    let cards: Vec<&Card> = cr.cards.iter().filter(|c| owned_c(c.id)).collect();
    // deck entries: card index (`rel`), chain kind, early-end link (0 = played normally), choice variant and sources
    let mut rel: Vec<usize> = vec![];
    let mut kind: Vec<Kind> = vec![];
    let mut endk: Vec<u8> = vec![];
    let mut vark: Vec<u32> = vec![];
    let mut srcs: Vec<Src> = vec![];
    let mut plain_names: Vec<String> = vec![];
    let mut plain_ids: Vec<u32> = vec![];
    let mut plain_chars: Vec<u32> = vec![];
    for (i, c) in cards.iter().enumerate() {
        let useful = !srcb.ev_for(c.id).is_empty() || !srcb.odds_for(c.id).is_empty() || !srcb.train(c).is_empty() || srcb.has_alts(c.id);
        let locked = dj.lock_cards.contains(&c.id);
        if c.pal || c.links == 0 {
            // pals, and cards without chain events (R cards): like a pal, an R card takes no chain slot
            if useful || locked { rel.push(i); kind.push((0, 0, 0)); endk.push(0); vark.push(0); srcs.push(srcb.src(c, None, 0)); }
        } else if c.rarity >= dj.min_rarity {
            // one entry per choice variant (an event offering targets on different choices: the player prioritises
            // one); the same card played to end its chain at link l is its own entry too. One character, so a deck
            // holds at most one of a card's entries.
            if useful || locked {
                for v in srcb.variants(c, None) { rel.push(i); kind.push(srcb.kind(c, None)); endk.push(0); vark.push(v); srcs.push(srcb.src(c, None, v)); }
            }
            for l in srcb.end_links(c.id) {
                if l == 0 || l > c.links { continue; }
                for v in srcb.variants(c, Some(l)) {
                    let x = srcb.src(c, Some(l), v);
                    if !x.is_empty() || locked { rel.push(i); kind.push(srcb.kind(c, Some(l))); endk.push(l); vark.push(v); srcs.push(x); }
                }
            }
        }
    }
    // plain pals: owned pal characters with no relevant card in `rel` (best-rarity card name for display).
    // With borrowing, a pal character the player owns no card of can still fill one slot as the borrowed card.
    // A deck with p plain pals uses the first p, so those blocking no uma in the pool come first (a plain pal that
    // blocks a loop member makes the result illegal, and `run_deck` then searches without it).
    let pool_ids: Vec<u32> = d.umas.iter().map(|u| u.id).filter(|&id| (sj.owned.is_empty() || sj.owned.contains(&id) || sj.require.contains(&id)) && !sj.exclude.contains(&id)).collect();
    let mut plain: Vec<(usize, u32, String)> = vec![];
    for c in cards.iter().filter(|c| c.pal && is_owned(c.id)) {
        if rel.iter().any(|&r| cards[r].chara == c.chara) || plain_chars.contains(&c.chara) { continue; }
        plain_chars.push(c.chara);
        let best = cards.iter().filter(|x| x.pal && x.chara == c.chara && is_owned(x.id)).max_by_key(|x| x.rarity).unwrap();
        plain.push((pool_ids.iter().filter(|&&u| best.blocks(u)).count(), best.id, best.name.clone()));
    }
    plain.sort_by_key(|x| x.0);   // stable: ties keep card order
    for (_, id, name) in plain { plain_ids.push(id); plain_names.push(name); }
    if dj.borrow {
        let mut seen: Vec<u32> = vec![];
        for c in cards.iter() {
            if !c.pal || is_owned(c.id) || plain_chars.contains(&c.chara) || seen.contains(&c.chara) || rel.iter().any(|&r| cards[r].chara == c.chara) { continue; }
            let best = cards.iter().enumerate().filter(|(_, x)| x.pal && x.chara == c.chara).max_by_key(|(_, x)| x.rarity).unwrap().0;
            seen.push(c.chara); rel.push(best); kind.push((0, 0, 0)); endk.push(0); vark.push(0); srcs.push(srcb.src(cards[best], None, 0));
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
        let mut kindset: Vec<Kind> = (0..nrel0).filter(|&j| kind[j] != (0, 0, 0)).map(|j| kind[j]).collect();
        kindset.push((2, 2, 0)); kindset.sort(); kindset.dedup();
        let others = dj.deck_size.saturating_sub(1);
        fn multisets(vals: &[Kind], start: usize, left: usize, cur: &mut Vec<Kind>, out: &mut dyn FnMut(&[Kind])) {
            out(cur);
            if left == 0 { return; }
            for i in start..vals.len() { cur.push(vals[i]); multisets(vals, i, left - 1, cur, out); cur.pop(); }
        }
        // lowest odds of each link over every possible set of other chain cards
        let needs_marg = |kd: Kind| (0..nrel0).any(|j| kind[j] != (0, 0, 0) && kind[j] == kd && !srcs[j].chains.is_empty());
        let mut min_marg: HashMap<Kind, Vec<f64>> = HashMap::new();
        for &kd in &kindset {
            if !needs_marg(kd) { continue; }
            let mut mins = vec![1.0f64; kd.1 as usize];
            multisets(&kindset, 0, others, &mut vec![], &mut |o| {
                let mut key = vec![kd]; key.extend_from_slice(o);
                let m = chain_marginals(&key);
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
        let tr = tie_rank(&rel, &cards);
        order.sort_by(|&a, &b| strength(b).partial_cmp(&strength(a)).unwrap().then(tr[a].cmp(&tr[b])));
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
                let le = (0..nu).all(|k| hi[a].0[k] <= lo[b].0[k] && hi[a].1[k] <= lo[b].1[k]);
                let eq = (0..nu).all(|k| hi[a].0[k] == lo[b].0[k] && hi[a].1[k] == lo[b].1[k]);
                le && (!eq || tr[a] < tr[b])
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
        let mut j = 0;
        vark.retain(|_| { let k = keep[j]; j += 1; k });
    }

    // ---- dominance coordinates ----
    // For a target the trainee's outfit does not give, the metric's landing chance is
    //   p = W + W*g - W*(1 - q)*(1 - h)  (W = GEN_WHITE),   g = P(gold hint), h = P(any hint) = 1 - (1 - w)*(1 - g);
    // with the white on the outfit p = W + W*g, and with the gold it is constant. So the metric only
    // rises with g and h, and a deck at least as high in every (g, h) is never worse. (w, g) dominance implies
    // (g, h) dominance, so this prunes at least as much. Coordinates no relevant card can move are dropped,
    // and h == g when no card gives the white.
    let mut has_w = vec![false; nu]; let mut has_g = vec![false; nu];
    for sr in &srcs {
        for &(k, _) in &sr.train { has_w[k] = true; }
        for g in sr.fixed.iter().chain(sr.chains.iter().map(|x| &x.1)) {
            for (_, out) in g { for &(k, lvl) in out { if lvl >= 2 { has_g[k] = true; } else { has_w[k] = true; } } }
        }
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
    let mut kinds_all: Vec<Kind> = (0..nrel).filter(|&j| kind[j] != (0, 0, 0)).map(|j| kind[j]).collect();
    kinds_all.push((2, 2, 0)); kinds_all.sort(); kinds_all.dedup();
    let kpos: Vec<u64> = (0..kinds_all.len()).map(|i| 7u64.pow(i as u32)).collect();
    // Per key: each entry's miss chances (white, gold) under the key's chain odds, and for each entry B the
    // entries that beat it on every target under those odds (same kind, so swapping keeps the odds; per target no
    // higher chance of missing the gold and no higher chance of missing every hint; exact ties go by `tie_rank`), as
    // owned / borrowed character masks.
    struct KeyData { mw: Vec<f64>, mg: Vec<f64>, d_own: Vec<u128>, d_bor: Vec<u128> }
    let tr2 = tie_rank(&rel, &cards);
    let mut key_cache: crate::FxMap<u64, KeyData> = Default::default();
    let make_key = |code: u64| -> KeyData {
        let mut ks: Vec<Kind> = vec![]; let mut c = code;
        for &kd in &kinds_all { for _ in 0..(c % 7) { ks.push(kd); } c /= 7; }
        let marg = chain_marginals(&ks);
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
            if eq && tr2[a] > tr2[b] { continue; }
            if ebor[a] { d_bor[b] |= ebit[a]; } else { d_own[b] |= ebit[a]; }
        } }
        KeyData { mw, mg, d_own, d_bor }
    };
    let mut n_decks = 0usize;
    let size = dj.deck_size;
    let kmax = kmax_top(&dj);
    let ident_of = |c: usize, e: u8, x: u32| ((c as u64 + 3) << 40) | ((x as u64) << 8) | e as u64;
    let ident = |cards: &[usize], ends: &[u8], vars: &[u32], plain: usize, filler: usize| -> Vec<u64> {
        let mut v: Vec<u64> = cards.iter().zip(ends).zip(vars).map(|((&c, &e), &x)| ident_of(c, e, x)).collect();
        v.sort(); v.push((1 << 40) + plain as u64); v.push((2 << 40) + filler as u64); v
    };
    // the same identity straight from a selection of entries
    let ident_sel = |sel: &[usize], plain: usize, filler: usize| -> Vec<u64> {
        let mut v: Vec<u64> = Vec::with_capacity(sel.len() + 2);
        v.extend(sel.iter().map(|&j| ident_of(rel[j], endk[j], vark[j])));
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
    let ki22 = kinds_all.iter().position(|&k| k == (2, 2, 0)).unwrap();
    let by_kind: Vec<Vec<usize>> = (0..nk).map(|i| (0..nrel).filter(|&j| kind[j] != (0, 0, 0) && kind[j] == kinds_all[i]).collect()).collect();
    // entries without chain slots: pals, group cards and R cards
    let pal_ents: Vec<usize> = (0..nrel).filter(|&j| kind[j] == (0, 0, 0)).collect();
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
    let mut run_enum = |kq: usize, s1: Option<&(Kept, crate::FxSet<Vec<u64>>)>| -> Vec<DeckEval> {
    let mut groups: crate::FxMap<(usize, usize), Kept> = Default::default();
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
                            let own = ids.contains(&ident_sel(sel, plain, filler));
                            if !own || s1k.count_ro(&dv, xq, s, kmax + 1) > kmax { st_s1 += 1; return; }
                        }
                    }
                    let n = g.count(&dv, xq, s, kmax, &mut st_tests);
                    st_full += 1;
                    if n > 0 && (kmax > 1 || n >= kmax) && nd > 0 { g.remember_found(); }
                    if n >= kmax { st_dom += 1; return; }
                    let mut so: Vec<usize> = sel.to_vec(); so.sort();       // entry order, for display
                    let x = DeckEval { pals, borrowed, cards: so.iter().map(|&j| rel[j]).collect(), ends: so.iter().map(|&j| endk[j]).collect(), vars: so.iter().map(|&j| vark[j]).collect(), plain, filler, dw: mw[..nu].to_vec(), dg: mg[..nu].to_vec(), dv: dv.clone(), s };
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
        let list = groups.remove(pc).unwrap().into_list();
        // with kmax = 1 the online set is already an antichain (every insert drops what it dominates and is
        // rejected when dominated), so k_filter would keep every deck and only reorder: reproduce its order
        let kept = if kmax == 1 { kfilter_order(list, coords.len()) } else { k_filter(list, kmax, coords.len()).into_list() };
        front.extend(kept);
    }
    front
    };
    // top_decks > 1: first the decks no other deck beats (fast), then the full pass filtered against them
    let ekey = ENUM_CACHE.with(|c| c.borrow().is_some()).then(|| enum_key(v));
    let cached = ekey.as_ref().and_then(|k| ENUM_CACHE.with(|c| c.borrow().as_ref().unwrap().iter().find(|e| &e.0 == k).map(|e| (e.1.clone(), e.2, e.3))));
    let hit = cached.is_some();
    let front: Vec<DeckEval> = if let Some((f, nd_, ns_)) = cached { n_decks = nd_; n_swapped = ns_; f } else if kmax == 1 { run_enum(1, None) } else {
        let s1 = run_enum(1, None);
        let nd = coords.len();
        let mut sorted: Vec<&DeckEval> = s1.iter().collect();
        sorted.sort_by_key(|x| std::cmp::Reverse(sig_of(&x.dv, false)));      // similar decks share blocks
        let mut kk = Kept::new(nd);
        for x in sorted { kk.insert(x.clone()); }
        let ids: crate::FxSet<Vec<u64>> = s1.iter().map(|x| ident(&x.cards, &x.ends, &x.vars, x.plain, x.filler)).collect();
        if std::env::var("DECK_DEBUG").is_ok() { eprintln!("pass 1: {} decks no other beats, {:.2}s", s1.len(), t0.elapsed().as_secs_f64()); }
        run_enum(kmax, Some(&(kk, ids)))
    };
    if let (Some(k), false) = (ekey, hit) {
        ENUM_CACHE.with(|c| { let mut c = c.borrow_mut(); let l = c.as_mut().unwrap(); if l.len() >= 4 { l.remove(0); } l.push((k, front.clone(), n_decks, n_swapped)); });
    }
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
    // (with top_decks = 1 the frontier is already an antichain, so it is its own skyline)
    let sky: Vec<usize> = if kmax_top(&dj) == 1 { (0..front.len()).collect() } else {
        let tagged: Vec<DeckEval> = front.iter().enumerate().map(|(i, x)| { let mut y = x.clone(); y.cards = vec![i]; y }).collect();
        let mut v: Vec<usize> = k_filter(tagged, 1, coords.len()).into_list().into_iter().map(|y| y.cards[0]).collect();
        v.sort();
        v
    };
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("skyline {} of {}", sky.len(), front.len()); }
    if dj.deck_only || dj.per_uma {
        let pd: Vec<PDeck> = sky.iter().map(|&i| { let x = &front[i];
            let mut names: Vec<String> = x.cards.iter().zip(&x.ends).zip(&x.vars).map(|((&i, &e), &v)| {
                let mut n = cards[i].name.clone();
                let lb = srcb.label(cards[i], (e > 0).then_some(e), v, sk);
                if !lb.is_empty() { n = format!("{} ({})", n, lb); }
                if e > 0 { n = format!("{} (end chain at link {})", n, e); }
                if Some(i) == x.borrowed { n = format!("{} (borrowed)", n); }
                n
            }).collect();
            names.extend(plain_names.iter().take(x.plain).cloned());
            for _ in 0..x.filler { names.push("(any SR without target hints)".into()); }
            let choices: Vec<String> = x.cards.iter().zip(&x.ends).zip(&x.vars).map(|((&i, &e), &v)| srcb.label(cards[i], (e > 0).then_some(e), v, sk)).collect();
            PDeck { names, ids: x.cards.iter().map(|&i| cards[i].id).collect(), ends: x.ends.clone(), vars: x.vars.clone(), choices, plain_ids: plain_ids.iter().take(x.plain).copied().collect(),
                    pal_names: x.cards.iter().filter(|&&i| cards[i].pal).map(|&i| cards[i].name.clone()).chain(plain_names.iter().take(x.plain).cloned()).collect(),
                    pals: x.pals, borrowed: x.borrowed.map(|i| cards[i].name.clone()), dw: x.dw.clone(), dg: x.dg.clone() } }).collect();
        if dj.deck_only { return json!({"sky": pd}); }
        let stats = json!({"decks_built": n_decks, "decks_on_frontier": front.len(), "relevant_cards": nrel, "cards_dropped": n_card_dropped,
                           "decks_swapped": n_swapped, "plain_pals": plain_names});
        return run_multi(d, sk, cr, v, &model, &sj, &dj, pd, stats);
    }
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
    let bcl: Vec<Cl> = { let mut ix = sky.clone(); let mut out = vec![]; if nu > 0 && !ix.is_empty() { split(&front, &mut ix, nu, &mut out); } out };
    let btab_c: std::cell::RefCell<Vec<[f64; NG]>> = std::cell::RefCell::new(vec![[f64::NAN; NG]; n]);
    let btab = |u: usize, i: usize| -> f64 {
        let v = btab_c.borrow()[u][i];
        if !v.is_nan() { return v; }
        let q = grid[i];
        // T rises with every odds coordinate (0 <= q <= 1), so T at a cluster's maximum odds bounds its decks:
        // clusters go best bound first and stop once a bound (with 1e-9 relative slack for rounding) is below the
        // best value; a maximum does not depend on visit order, so the value is the same bit for bit
        let v = if (0.0..=1.0).contains(&q) {
            let mut bs: Vec<(f64, usize)> = bcl.iter().enumerate().map(|(ci, c)| (model.trainee_fast(u, q, &c.mdw, &c.mdg), ci)).collect();
            bs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let mut best = 0.0f64;
            for (b, ci) in bs {
                if b * (1.0 + 1e-9) < best { break; }
                for &j in &bcl[ci].mem { best = best.max(model.trainee_fast(u, q, &front[j].dw, &front[j].dg)); }
            }
            best
        } else { sky.iter().map(|&j| model.trainee_fast(u, q, &front[j].dw, &front[j].dg)).fold(0.0, f64::max) };
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
        let mut names: Vec<String> = x.cards.iter().zip(&x.ends).zip(&x.vars).map(|((&i, &e), &v)| {
            let mut n = cards[i].name.clone();
            let lb = srcb.label(cards[i], (e > 0).then_some(e), v, sk);
            if !lb.is_empty() { n = format!("{} ({})", n, lb); }
            if e > 0 { n = format!("{} (end chain at link {})", n, e); }
            if Some(i) == x.borrowed { n = format!("{} (borrowed)", n); }
            n
        }).collect();
        names.extend(plain_names.iter().take(x.plain).cloned());
        for _ in 0..x.filler { names.push("(any SR without target hints)".into()); }
        let odds: Vec<Value> = (0..nu).map(|k| json!({"group": unc[k], "white": x.dw[k], "gold": x.dg[k]})).collect();
        let pal_names: Vec<String> = x.cards.iter().filter(|&&i| cards[i].pal).map(|&i| cards[i].name.clone()).chain(plain_names.iter().take(x.plain).cloned()).collect();
        json!({"pals": x.pals, "cards": names, "odds": odds, "pal_cards": pal_names, "borrowed": x.borrowed.map(|i| cards[i].name.clone()),
               "card_ids": x.cards.iter().map(|&i| cards[i].id).collect::<Vec<_>>(), "end_chain_at": x.ends.clone(),
               "choices": x.cards.iter().zip(&x.ends).zip(&x.vars).map(|((&i, &e), &v)| srcb.label(cards[i], (e > 0).then_some(e), v, sk)).collect::<Vec<_>>(),
               "plain_ids": plain_ids.iter().take(x.plain).copied().collect::<Vec<_>>()})
    };
    // exact odds for a reported (deck, loop); `v` is the search's ranking value (the product of per-target odds)
    let loop_json = |f: [usize; 4], r: &Res, qs: &[f64], x: &DeckEval, v: f64| -> Value {
        let ents: Vec<(Src, Kind)> = x.cards.iter().zip(&x.ends).zip(&x.vars).map(|((&i, &e), &vv)| {
            let st = (e > 0).then_some(e); (srcb.src(cards[i], st, vv), srcb.kind(cards[i], st)) }).collect();
        let mut per_l = vec![(0.0, 0.0, 0usize); 12];
        for t in 0..4 {
            let ls: Vec<usize> = (0..12).filter(|&l| lin[l].0 == t).collect();
            let qt: Vec<f64> = ls.iter().map(|&l| qs[l]).collect();
            let j = joint_trainee(&model, f[t], &qt, &ents, x.filler, sj.targets.len());
            for (i, &l) in ls.iter().enumerate() { per_l[l] = j[i]; }
        }
        let pj = per_l.iter().map(|y| y.0).sum::<f64>() / 12.0;
        let minus1 = per_l.iter().map(|y| y.1).sum::<f64>() / 12.0;
        let mut per = vec![(0.0, 0usize); 4]; let mut picks: Vec<Vec<usize>> = vec![vec![]; 4];
        for (l, &(t, ..)) in lin.iter().enumerate() { per[t].0 += per_l[l].0; per[t].1 += 1; picks[t].push(per_l[l].2); }
        let outfits: Vec<Value> = (0..4).map(|i| {
            let kk = *picks[i].iter().max_by_key(|x| picks[i].iter().filter(|y| y == x).count()).unwrap();
            let (st, cs) = &model.opts[f[i]][kk];
            let covers: Vec<Value> = st.iter().enumerate().filter(|(_, lv)| **lv > 0).map(|(j, lv)| json!({"group": unc[j], "level": lv})).collect();
            json!({"outfits": if covers.is_empty() { vec![] } else { cs.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>() }, "covers": covers})
        }).collect();
        let s4 = [r.s[0], r.s[1], r.s[2], r.s[3]];
        let ps: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let mut detail = model.detail(f, &|a, b| wval(&ps[a], &ps[b]), Some([(&x.dw[..], &x.dg[..]); 4]));
        if let Some(ls) = detail["lineages"].as_array_mut() { for (l, y) in ls.iter_mut().enumerate() { y["rank_value"] = y["p_all"].clone(); y["p_all"] = json!(per_l[l].0); } }
        json!({"members": member_json(d, &f), "p_all": pj, "p_minus1": minus1, "rank_value": v, "outfits": outfits,
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
    // the top decks by the search's ranking, reported with exact odds and in their order
    let mut top: Vec<Value> = order.iter().take(kdeck).map(|&di| { let (v, si) = deck_best[di]; let (f, r, qs) = &solved[si];
        json!({"deck": deck_json(&front[di]), "loop": loop_json(*f, r, qs, &front[di], v)}) }).collect();
    top.sort_by(|a, b| b["loop"]["p_all"].as_f64().partial_cmp(&a["loop"]["p_all"].as_f64()).unwrap());
    let next_bound = (if next < cand.len() { cand[next].0 } else { 0.0 }).max(if hi_sum > 0.0 { sep(hi_sum) } else { 0.0 });
    json!({"top": top, "best": by_pals, "decks_built": n_decks, "decks_on_frontier": front.len(), "foursomes": total_foursomes,
           "foursomes_ranked": generated, "foursomes_scanned": scanned, "foursomes_solved": solved.len(), "next_bound": next_bound,
           "proven": next_bound < minbest(&best), "relevant_cards": rel.len(), "cards_dropped": n_card_dropped, "decks_swapped": n_swapped, "plain_pals": plain_names})
}


/// Per-uma decks: each trainee runs her own best legal deck, the same one in all three lineages she heads.
///   value(f) = fixed/12 * sum_t max over decks D legal for f[t] of sum over her lineages l of T(f[t], q_l, D)
/// The skyline (decks no deck beats on every coordinate) holds every uma's best deck as long as none of its decks
/// carries a card that blocks her; for a uma it does, her legal decks come from a second deck search without
/// those cards, run the first time a foursome with her is solved. Bounds use the whole skyline (legal or not),
/// which no legal deck beats, so the search is exact.
fn run_multi(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value, model: &Model, sj: &SkillJob, dj: &DeckJob, sky: Vec<PDeck>, stats: Value) -> Value {
    let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
    let nu = unc.len();
    let card_of: HashMap<u32, &Card> = cr.cards.iter().map(|c| (c.id, c)).collect();
    let n = d.umas.len();
    let id_ok = |id: u32| (sj.owned.is_empty() || sj.owned.contains(&id) || sj.require.contains(&id)) && !sj.exclude.contains(&id);
    // a locked card goes in every deck, so a uma it blocks cannot train at all
    let lock_blocked = |u: usize| dj.lock_cards.iter().any(|id| card_of.get(id).map_or(false, |c| c.blocks(d.umas[u].id)));
    let pool: Vec<usize> = (0..n).filter(|&i| id_ok(d.umas[i].id) && !model.no_outfit[i] && !lock_blocked(i)).collect();
    let req: Vec<usize> = sj.require.iter().filter_map(|&id| d.umas.iter().position(|u| u.id == id)).collect();
    if req.iter().any(|r| !pool.contains(r)) { return json!({"error": "A required uma cannot train with a locked card of her own."}); }
    if pool.len() < 4 { return json!({"error": "Pick at least 4 umas to search."}); }
    let tq = Instant::now();
    let fixed_s: Option<Vec<Option<Sched>>> = dj.schedule.as_deref().map(|mode| (0..n).map(|i| if pool.contains(&i) { Some(prebuilt(d, &d.umas[i], mode)) } else { None }).collect());
    let no_race = dj.schedule.as_deref() == Some("none");
    let pb = match &fixed_s {
        Some(fs) => {
            let ps: Vec<Option<Vec<f64>>> = fs.iter().enumerate().map(|(i, s)| s.as_ref().map(|s| probs(d, &d.umas[i], s))).collect();
            let mut m = vec![vec![0.0f64; n]; n];
            if !no_race { for &a in &pool { for &b in &pool { if a != b { m[a][b] = wval(ps[a].as_ref().unwrap(), ps[b].as_ref().unwrap()); } } } }
            m
        }
        None => pair_ceilings(d, &pool),
    };
    let k = pool.len();
    let lin = lineages();
    let lin_t: Vec<usize> = lin.iter().map(|l| l.0).collect();
    let fixed = model.fixed_factor();
    let qmax: Vec<f64> = (0..n).map(|u| if pool.contains(&u) { model.q_ub_trainee(u, &pool, &pb) } else { 0.0 }).collect();
    let qlo = model.q_floor();
    let qhi = pool.iter().map(|&u| qmax[u]).fold(qlo, f64::max);
    const NG: usize = 64;
    let step = if qhi > qlo { (qhi - qlo) / (NG - 1) as f64 } else { 0.0 };
    let grid: Vec<f64> = (0..NG).map(|i| if i == NG - 1 { qhi } else { qlo + step * i as f64 }).collect();
    let gidx = |q: f64| -> usize {
        if step == 0.0 { return NG - 1; }
        let mut i = (((q - qlo) / step).ceil().max(0.0) as usize).min(NG - 1);
        while grid[i] < q && i < NG - 1 { i += 1; }
        i
    };
    let zero = vec![0.0f64; nu];
    // A uma's value depends only on the targets some outfit of hers lacks: (g, h) = (P(gold), P(any hint)) where
    // an outfit lacks the skill, g alone where her outfits have at most the white. Decks are thinned per class of
    // umas with the same coordinates: a deck another beats on all of them never wins for those umas.
    let ucoords = |u: usize| -> Vec<(usize, bool)> {
        let mut c = vec![];
        for k in 0..nu {
            if model.opts[u].iter().any(|(st, _)| st[k] == 0) { c.push((k, true)); c.push((k, false)); }
            else if model.opts[u].iter().any(|(st, _)| st[k] == 1) { c.push((k, true)); }
        }
        c
    };
    let thin = |list: &[PDeck], co: &[(usize, bool)]| -> Vec<usize> {
        if list.is_empty() { return vec![]; }
        if co.is_empty() { return vec![0]; }
        let tagged: Vec<DeckEval> = list.iter().enumerate().map(|(i, x)| {
            let dv: Vec<f64> = co.iter().map(|&(k, g)| if g { x.dg[k] } else { 1.0 - (1.0 - x.dw[k]) * (1.0 - x.dg[k]) }).collect();
            let s = dv.iter().sum();
            DeckEval { pals: 0, borrowed: None, cards: vec![i], ends: vec![], vars: vec![], plain: 0, filler: 0, dw: vec![], dg: vec![], dv, s }
        }).collect();
        let mut v: Vec<usize> = k_filter(tagged, 1, co.len()).into_list().into_iter().map(|y| y.cards[0]).collect();
        v.sort(); v
    };
    // the full coordinate set needs no thinning: the skyline is already thin on it
    let full: Vec<(usize, bool)> = (0..nu).flat_map(|k| [(k, true), (k, false)]).collect();
    let mut class_ix: crate::FxMap<Vec<(usize, bool)>, usize> = Default::default();
    let mut class_sky: Vec<Vec<usize>> = vec![];
    let mut ucls: Vec<usize> = vec![usize::MAX; n];
    for &u in &pool {
        let co = ucoords(u);
        let ci = match class_ix.get(&co) { Some(&c) => c, None => {
            class_sky.push(if co == full { (0..sky.len()).collect() } else { thin(&sky, &co) });
            class_ix.insert(co, class_sky.len() - 1); class_sky.len() - 1 } };
        ucls[u] = ci;
    }
    let uview = |u: usize| -> &Vec<usize> { &class_sky[ucls[u]] };
    // Clusters of each class's decks (recursive median splits on the widest odds coordinate) with their
    // componentwise maximum odds. T(u, q, deck) rises with every deck odds coordinate (for 0 <= q <= 1, since a
    // white is worth less than a gold), so T at a cluster's maximum bounds every member; maxima over a class are
    // found cluster by cluster, best bound first, skipping clusters whose bound (with 1e-9 relative slack for
    // rounding) is below the best value so far. The maximum of a set does not depend on visit order, so the
    // result is the same value bit for bit.
    let class_cl: Vec<Vec<Cl>> = class_sky.iter().map(|v| { let mut ix = v.clone(); let mut out = vec![]; if nu > 0 && !ix.is_empty() { split(&sky, &mut ix, nu, &mut out); } out }).collect();
    // max(init, max over u's class of val(deck)), with bound(cluster maxima) >= val of every member
    let bb_max = |u: usize, init: f64, val: &dyn Fn(&[f64], &[f64]) -> f64| -> f64 {
        let cls = &class_cl[ucls[u]];
        if cls.is_empty() { return uview(u).iter().map(|&j| val(&sky[j].dw, &sky[j].dg)).fold(init, f64::max); }
        let mut bs: Vec<(f64, usize)> = cls.iter().enumerate().map(|(i, c)| (val(&c.mdw, &c.mdg), i)).collect();
        bs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let mut best = init;
        for (b, i) in bs {
            if b * (1.0 + 1e-9) < best { break; }
            for &j in &cls[i].mem { best = best.max(val(&sky[j].dw, &sky[j].dg)); }
        }
        best
    };
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("classes {} sizes {:?} ({:.2}s)", class_sky.len(), class_sky.iter().map(|c| c.len()).collect::<Vec<_>>(), tq.elapsed().as_secs_f64()); }
    // B[u][i] = max over all skyline decks (legal for u or not) of T(u, grid[i], deck): no legal deck beats it
    let btab_c: std::cell::RefCell<Vec<[f64; NG]>> = std::cell::RefCell::new(vec![[f64::NAN; NG]; n]);
    let btab = |u: usize, i: usize| -> f64 {
        let v = btab_c.borrow()[u][i];
        if !v.is_nan() { return v; }
        let q = grid[i];
        let init = model.trainee_fast(u, q, &zero, &zero);
        let v = if (0.0..=1.0).contains(&q) { bb_max(u, init, &|dw, dg| model.trainee_fast(u, q, dw, dg)) }
            else { uview(u).iter().map(|&j| model.trainee_fast(u, q, &sky[j].dw, &sky[j].dg)).fold(init, f64::max) };
        btab_c.borrow_mut()[u][i] = v; v
    };
    // legal deck sets: the skyline itself unless one of its decks blocks the uma
    let needs_alt: Vec<bool> = (0..n).map(|u| pool.contains(&u) && uview(u).iter().any(|&j| sky[j].blocks(d.umas[u].id, &card_of))).collect();
    let alt: std::cell::RefCell<HashMap<usize, Vec<PDeck>>> = std::cell::RefCell::new(HashMap::new());
    let alt_built = std::cell::Cell::new(0u64);
    let ensure = |u: usize| {
        if !needs_alt[u] || alt.borrow().contains_key(&u) { return; }
        let uid = d.umas[u].id;
        let mut job = v.clone();
        job["deck_only"] = json!(true); job["per_uma"] = json!(false); job["top_decks"] = json!(1);
        let mut ex: Vec<u32> = dj.exclude_cards.clone();
        for c in cr.cards.iter().filter(|c| c.blocks(uid)) { if !ex.contains(&c.id) { ex.push(c.id); } }
        job["exclude_cards"] = json!(ex);
        let out = run_deck_relaxed(d, sk, cr, &job);
        let list: Vec<PDeck> = serde_json::from_value(out["sky"].clone()).unwrap_or_default();
        let list: Vec<PDeck> = thin(&list, &ucoords(u)).into_iter().map(|j| list[j].clone()).collect();
        alt_built.set(alt_built.get() + 1);
        alt.borrow_mut().insert(u, list);
    };
    // best legal deck for uma u over her lineages' q: (sum of T, the deck; None = no legal deck, odds 0).
    // The best skyline deck beats every deck; when it is legal it is the answer, and only otherwise are her
    // legal decks searched for.
    let scan = |u: usize, qs: &[f64], list: &[&PDeck]| -> (f64, Option<usize>) {
        let mut best = (qs.iter().map(|&q| model.trainee_fast(u, q, &zero, &zero)).sum::<f64>(), None::<usize>);
        for (j, x) in list.iter().enumerate() {
            let s: f64 = qs.iter().map(|&q| model.trainee_fast(u, q, &x.dw, &x.dg)).sum();
            if s > best.0 { best = (s, Some(j)); }
        }
        best
    };
    let best_for = |u: usize, qs: &[f64]| -> (f64, Option<PDeck>) {
        let view: Vec<&PDeck> = uview(u).iter().map(|&j| &sky[j]).collect();
        let (s, j) = scan(u, qs, &view);
        if !needs_alt[u] || j.map_or(true, |j| !view[j].blocks(d.umas[u].id, &card_of)) { return (s, j.map(|j| view[j].clone())); }
        ensure(u);
        let a = alt.borrow();
        let list: Vec<&PDeck> = a[&u].iter().collect();
        let (s, j) = scan(u, qs, &list);
        (s, j.map(|j| list[j].clone()))
    };
    // ceiling with each uma's best skyline deck (legal or not), the same deck across her lineages
    let relaxed = |f: [usize; 4], qa: &[f64; 12]| -> f64 {
        let mut tot = 0.0;
        for t in 0..4 {
            let qs: Vec<f64> = (0..12).filter(|&l| lin_t[l] == t).map(|l| qa[l]).collect();
            let init = qs.iter().map(|&q| model.trainee_fast(f[t], q, &zero, &zero)).sum::<f64>();
            let val = |dw: &[f64], dg: &[f64]| qs.iter().map(|&q| model.trainee_fast(f[t], q, dw, dg)).sum::<f64>();
            tot += if qs.iter().all(|q| (0.0..=1.0).contains(q)) { bb_max(f[t], init, &val) }
                else { uview(f[t]).iter().map(|&j| val(&sky[j].dw, &sky[j].dg)).fold(init, f64::max) };
        }
        fixed * tot / 12.0 * (1.0 + 1e-12)
    };
    let mval: Vec<f64> = (0..n).map(|u| if pool.contains(&u) { btab(u, gidx(qmax[u])) } else { 0.0 }).collect();
    let mut ord: Vec<usize> = pool.clone();
    ord.sort_by(|&a, &b| mval[b].partial_cmp(&mval[a]).unwrap());
    let mv: Vec<f64> = ord.iter().map(|&u| mval[u]).collect();
    let sep = |s: f64| fixed * 3.0 * s / 12.0 * (1.0 + 1e-12);
    let nreq = req.len();
    let total_foursomes: usize = { let m = k - nreq.min(k); let c = 4usize.saturating_sub(nreq); (0..c).fold(1usize, |acc, i| acc * (m - i) / (i + 1)) };
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
                        let mut f = [ord[a], ord[b], ord[c], ord[e]]; f.sort();
                        if !req.iter().all(|r| f.contains(r)) { continue; }
                        let qs = model.lineage_q_arr(f, &|x, y| pb[f[x]][f[y]]);
                        let ub = fixed * (0..12).map(|l| btab(f[lin_t[l]], gidx(qs[l]))).sum::<f64>() / 12.0 * (1.0 + 1e-12);
                        out.push((ub, f));
                    }
                }
            }
        }
    };
    let smax = if k >= 4 { mv[0] + mv[1] + mv[2] + mv[3] } else { 0.0 };
    let mut hi_sum = f64::INFINITY;
    let lo_sum = smax * 0.85;
    let mut cand: Vec<(f64, [usize; 4])> = vec![];
    band(lo_sum, hi_sum, &mut cand);
    hi_sum = lo_sum;
    cand.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
    let kloops = dj.top_decks.max(1);
    let mut solved: Vec<([usize; 4], Res, Vec<f64>, Vec<Option<PDeck>>, f64)> = vec![];
    let mut vals: Vec<f64> = vec![];
    let mut next = 0; let mut scanned = 0; let mut generated = cand.len();
    let mut kth_val = f64::NEG_INFINITY;
    loop {
        // Ceilings carry a (1 + 1e-12) guard against rounding, so a loop whose ceiling beats the k-th value by no more
        // than that only ties it and cannot change the result. Without this, a job whose targets are all guaranteed
        // (every loop has the same value) solved every foursome.
        let beats = |c: f64| c > kth_val * (1.0 + 4e-12);
        if !(next < cand.len() && beats(cand[next].0)) {
            if hi_sum <= 0.0 || !beats(sep(hi_sum)) { break; }
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
        let qpb = model.lineage_q_arr(f, &|x, y| pb[f[x]][f[y]]);
        if !beats(relaxed(f, &qpb)) { continue; }
        let r = match &fixed_s {
            Some(fs) => Res { val: 0.0, s: std::array::from_fn(|i| fs[f[i]].unwrap()), order: [0, 1, 2, 3], evals: 0, secs: 0.0 },
            None => optimize(d, f, "scf@0.5", sj.iters, sj.seed, 60.0, &[]),
        };
        let ps: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let qs = model.lineage_q(f, &|x, y| if no_race { 0.0 } else { wval(&ps[x], &ps[y]) });
        let qa: [f64; 12] = std::array::from_fn(|i| qs[i]);
        // the exact ceiling only needs the skyline: skip the legal-deck searches when it cannot win
        if !beats(relaxed(f, &qa)) { continue; }
        let mut tot = 0.0; let mut decks: Vec<Option<PDeck>> = vec![];
        for t in 0..4 {
            let qt: Vec<f64> = (0..12).filter(|&l| lin_t[l] == t).map(|l| qa[l]).collect();
            let (s, x) = best_for(f[t], &qt); tot += s; decks.push(x);
        }
        let val = fixed * tot / 12.0;
        solved.push((f, r, qs, decks, val));
        vals.push(val);
        let mut vs = vals.clone(); vs.sort_by(|a, b| b.partial_cmp(a).unwrap());
        kth_val = if vs.len() >= kloops { vs[kloops - 1] } else { f64::NEG_INFINITY };
    }
    if solved.is_empty() { return json!({"error": "No foursome contains all the required umas."}); }
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("per-uma search {:.2}s scanned {} solved {} legal-deck searches {}", tq.elapsed().as_secs_f64(), scanned, solved.len(), alt_built.get()); }
    let mut order: Vec<usize> = (0..solved.len()).collect();
    order.sort_by(|&a, &b| solved[b].4.partial_cmp(&solved[a].4).unwrap());
    let empty_deck = PDeck { names: vec![], ids: vec![], ends: vec![], vars: vec![], choices: vec![], plain_ids: vec![], pal_names: vec![], pals: 0, borrowed: None, dw: zero.clone(), dg: zero.clone() };
    // ---- exact odds for the loops reported ----
    // The search ranked by the product of per-target odds. The loops within JOINT_BAND of the k-th best are scored
    // exactly, each trainee with the legal deck (among those within JOINT_BAND of her best by the product) that
    // gives her lineages the highest exact P(all); they are then ordered by it.
    let srcb = Sources { dj, unc: &unc };
    let ntar = sj.targets.len();
    let ents_of = |x: &PDeck| -> (Vec<(Src, Kind)>, usize) {
        let e: Vec<(Src, Kind)> = x.ids.iter().enumerate().filter_map(|(i, id)| card_of.get(id).map(|c| {
            let st = x.ends.get(i).copied().filter(|&e| e > 0); let v = x.vars.get(i).copied().unwrap_or(0);
            (srcb.src(c, st, v), srcb.kind(c, st)) })).collect();
        (e, dj.deck_size.saturating_sub(x.ids.len() + x.plain_ids.len()))
    };
    let joint_pick = |u: usize, qt: &[f64], chosen: &Option<PDeck>| -> (Vec<(f64, f64, usize)>, Option<PDeck>) {
        let pv = |x: &PDeck| qt.iter().map(|&q| model.trainee_fast(u, q, &x.dw, &x.dg)).sum::<f64>();
        let a = alt.borrow();
        let list: Vec<&PDeck> = match a.get(&u) { Some(l) => l.iter().collect(), None => uview(u).iter().map(|&j| &sky[j]).filter(|x| !x.blocks(d.umas[u].id, &card_of)).collect() };
        let bpv = chosen.as_ref().map_or(0.0, |x| pv(x));
        let mut cands: Vec<(f64, &PDeck)> = list.into_iter().map(|x| (pv(x), x)).filter(|(v, _)| *v >= JOINT_BAND * bpv).collect();
        cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        if let Some(c) = chosen { if !cands.iter().any(|x| x.1.ids == c.ids && x.1.ends == c.ends && x.1.vars == c.vars) { cands.insert(0, (bpv, c)); } }
        let mut best: Option<(f64, Vec<(f64, f64, usize)>, Option<PDeck>)> = None;
        if cands.is_empty() { return (joint_trainee(model, u, qt, &[], 0, ntar), None); }
        for (_, x) in cands {
            let (e, fl) = ents_of(x);
            JCOUNT.with(|c| c.set(c.get() + 1));
            let j = joint_trainee(model, u, qt, &e, fl, ntar);
            let sv: f64 = j.iter().map(|y| y.0).sum();
            if best.as_ref().map_or(true, |b| sv > b.0) { best = Some((sv, j, Some(x.clone()))); }
        }
        let b = best.unwrap(); (b.1, b.2)
    };
    let kv = order.get(kloops.min(order.len()) - 1).map_or(0.0, |&si| solved[si].4);
    let mut scored: Vec<(f64, usize, Vec<(f64, f64, usize)>, Vec<Option<PDeck>>)> = order.iter().take(kloops + 8)
        .filter(|&&si| solved[si].4 >= JOINT_BAND * kv).map(|&si| {
        let (f, _, qs, decks, _) = &solved[si];
        let mut per_l = vec![(0.0, 0.0, 0usize); 12]; let mut dks = vec![];
        for t in 0..4 {
            let ls: Vec<usize> = (0..12).filter(|&l| lin_t[l] == t).collect();
            let qt: Vec<f64> = ls.iter().map(|&l| qs[l]).collect();
            let (j, x) = joint_pick(f[t], &qt, &decks[t]);
            for (i, &l) in ls.iter().enumerate() { per_l[l] = j[i]; }
            dks.push(x);
        }
        (per_l.iter().map(|x| x.0).sum::<f64>() / 12.0, si, per_l, dks)
    }).collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("exact scoring: {} loops, {} decks scored, {:.3}s total", scored.len(), JCOUNT.with(|c| c.get()), tq.elapsed().as_secs_f64()); }
    let top: Vec<Value> = scored.iter().take(kloops).map(|(pj, si, per_l, jdecks)| {
        let (f, r, qs, _, v) = &solved[*si];
        let dk: Vec<&PDeck> = jdecks.iter().map(|x| x.as_ref().unwrap_or(&empty_deck)).collect();
        let mut per = vec![(0.0, 0usize); 4]; let mut picks: Vec<Vec<usize>> = vec![vec![]; 4];
        for (l, &(t, ..)) in lin.iter().enumerate() { per[t].0 += per_l[l].0; per[t].1 += 1; picks[t].push(per_l[l].2); }
        let minus1 = per_l.iter().map(|x| x.1).sum::<f64>() / 12.0;
        let outfits: Vec<Value> = (0..4).map(|i| {
            let kk = *picks[i].iter().max_by_key(|x| picks[i].iter().filter(|y| y == x).count()).unwrap();
            let (st, cs) = &model.opts[f[i]][kk];
            let covers: Vec<Value> = st.iter().enumerate().filter(|(_, lv)| **lv > 0).map(|(j, lv)| json!({"group": unc[j], "level": lv})).collect();
            json!({"outfits": if covers.is_empty() { vec![] } else { cs.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>() }, "covers": covers})
        }).collect();
        let s4 = [r.s[0], r.s[1], r.s[2], r.s[3]];
        let pss: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let mut detail = model.detail(*f, &|a, b| wval(&pss[a], &pss[b]), Some(std::array::from_fn(|t| (&dk[t].dw[..], &dk[t].dg[..]))));
        if let Some(ls) = detail["lineages"].as_array_mut() { for (l, x) in ls.iter_mut().enumerate() { x["rank_value"] = x["p_all"].clone(); x["p_all"] = json!(per_l[l].0); } }
        let lp = json!({"members": member_json(d, f), "p_all": pj, "p_minus1": minus1, "rank_value": v, "outfits": outfits,
               "per_trainee": per.iter().map(|(s, c)| s / *c as f64).collect::<Vec<_>>(),
               "mean_proc": qs.iter().sum::<f64>() / qs.len() as f64, "schedule": chart_rows(d, f, &s4), "detail": detail});
        // the deck most members share, for displays that show one deck
        let base = (0..4).max_by_key(|&i| (dk.iter().filter(|x| x.names == dk[i].names).count(), 4 - i)).unwrap();
        json!({"loop": lp, "deck": dk[base].out(&unc), "decks": dk.iter().map(|x| x.out(&unc)).collect::<Vec<_>>()})
    }).collect();
    let next_bound = (if next < cand.len() { cand[next].0 } else { 0.0 }).max(if hi_sum > 0.0 { sep(hi_sum) } else { 0.0 });
    let mut out = stats;
    for (key, val) in [("top", json!(top)), ("best", json!([])), ("per_uma", json!(true)), ("foursomes", json!(total_foursomes)),
                       ("foursomes_ranked", json!(generated)), ("foursomes_scanned", json!(scanned)), ("foursomes_solved", json!(solved.len())),
                       ("next_bound", json!(next_bound)), ("proven", json!(next_bound < kth_val)), ("searches", json!(1 + alt_built.get()))] {
        out[key] = val;
    }
    out
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
    /// dense chain_marginals equals the reference to float noise
    #[test]
    fn chain_marginals_dense_matches_ref() {
        let kinds: Vec<Kind> = vec![(2, 2, 0),(3, 3, 0),(3, 2, 0),(3, 1, 0),(4, 4, 0),(4, 3, 0),(5, 5, 0),(5, 4, 0),(5, 2, 0)];
        let mut worst = 0.0f64; let mut cnt = 0;
        fn ms(v: &[Kind], st: usize, left: usize, cur: &mut Vec<Kind>, f: &mut dyn FnMut(&[Kind])) { if !cur.is_empty() { f(cur); } if left == 0 { return; } for i in st..v.len() { cur.push(v[i]); ms(v, i, left - 1, cur, f); cur.pop(); } }
        ms(&kinds, 0, 6, &mut vec![], &mut |ks| {
            let a = chain_marginals_ref(ks); let b = chain_marginals_dense(ks); cnt += 1;
            for (k, v) in &a { for (x, y) in v.iter().zip(&b[k]) { worst = worst.max((x - y).abs()); } }
        });
        eprintln!("{cnt} multisets, max |dense - ref| = {worst:e}");
        assert!(worst < 1e-14);
    }
    /// The character rule: a card blocks its own character and, for a group card, every member.
    #[test]
    fn card_blocks_own_character_and_group_members() {
        let cr: CardsRaw = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web_cards.json")).unwrap()).unwrap();
        let sirius = cr.cards.iter().find(|c| c.id == 30081).expect("Team Sirius");
        assert!(sirius.blocks(1001) && sirius.blocks(1002) && !sirius.blocks(1008));    // Special Week, Suzuka; not Vodka
        let halo = cr.cards.iter().find(|c| c.name.contains("Tonight, We Waltz")).unwrap();
        assert!(halo.blocks(halo.chara) && !halo.blocks(1001));
    }
    /// Calibration against the uma-it runs the event count was fitted on: a 3-link card next to one other finishes its
    /// chain 89% of the time (observed; the binomial gives 86.6%), next to two others 60%, and alone almost always.
    #[test]
    fn chain_model_calibration() {
        let m = chain_marginals(&[(3, 3, 0), (3, 3, 0)]);
        assert!((m[&(3, 3, 0)][2] - 0.866).abs() < 0.01, "{:?}", m);
        assert!(m[&(3, 3, 0)][0] > 0.99);
        let m9 = chain_marginals(&[(3, 3, 0), (3, 3, 0), (3, 3, 0)]);
        assert!(m9[&(3, 3, 0)][2] > 0.45 && m9[&(3, 3, 0)][2] < 0.62, "{:?}", m9);
        let one = chain_marginals(&[(3, 3, 0)]);
        assert!(one[&(3, 3, 0)][2] > 0.99);
        // ending a chain early frees its slot: the other card's links fire more often
        let early = chain_marginals(&[(3, 3, 0), (3, 2, 0)]);
        assert!(early[&(3, 3, 0)][2] > m[&(3, 3, 0)][2] && early[&(3, 2, 0)].len() == 2);
        let pm = event_pmf(30);
        assert!((pm.iter().sum::<f64>() - 1.0).abs() < 1e-12 && (pm.iter().enumerate().map(|(e, p)| e as f64 * p).sum::<f64>() - 7.35).abs() < 1e-9);
    }
    /// A card's training draws over two targets in its pool: each target keeps its own chance 1 - (1 - 1/n)^h, both
    /// are hinted together less often than independent chances would say, and the outcomes are a distribution.
    #[test]
    fn training_draws_over_two_targets() {
        let cr: CardsRaw = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web_cards.json")).unwrap()).unwrap();
        let c = cr.cards.iter().find(|c| c.nskill > 0 && c.whites.contains(&20046) && c.whites.contains(&20161)).expect("a card with Ramp Up and Tail Held High");
        let dj = job_with(json!({}));
        let unc = [20046u32, 20161];
        let sb = Sources { dj: &dj, unc: &unc };
        let tr = sb.train(c); assert_eq!(tr.len(), 2);
        let g = sb.train_group(c, &tr);
        assert!(g.iter().map(|x| x.0).sum::<f64>() <= 1.0 + 1e-12);
        let gh = group_gh(&g);
        for (k, _, ph) in &gh { assert!((ph - tr[*k].1).abs() < 1e-12, "{gh:?} vs {tr:?}"); }
        let both = g.iter().filter(|x| x.1.len() == 2).map(|x| x.0).sum::<f64>();
        assert!(both < tr[0].1 * tr[1].1);
        let h = 2.70 * (1.0 + c.hf / 100.0);
        assert!((tr[0].1 - (1.0 - hint_miss(1, c.nskill as usize, h, c.hinc))).abs() < 1e-12);        // the card's assembled sources carry the same odds (the group is not lost next to its other rewards)
        let (mut mw, mut mg) = (vec![1.0; 2], vec![1.0; 2]);
        src_miss(&sb.src(c, None, 0), &[1.0, 1.0, 1.0], &mut mw, &mut mg);
        for k in 0..2 { assert!((1.0 - mw[k] * mg[k] - tr[k].1).abs() < 1e-9 || mg[k] < 1.0, "{mw:?} {mg:?} {tr:?}"); assert!(mw[k] * mg[k] < 1.0); }
    }
    /// The Lv-5 cap: a skill leaves a card's pool after ceil(5 / levels per draw) draws. Checked against brute-force
    /// enumeration of every draw sequence, and on cases worked by hand (a +5 card draws without replacement).
    #[test]
    fn training_draws_leave_maxed_skills() {
        fn brute(k: usize, n: usize, t: usize, cap: usize, cnt: &mut Vec<usize>) -> f64 {
            if t == 0 { return 1.0; }
            let avail: Vec<usize> = (0..n).filter(|&i| cnt[i] < cap).collect();
            if avail.is_empty() { return 1.0; }
            let mut p = 0.0;
            for &i in &avail {
                if i < k { continue; }                       // skills 0..k are the given ones: drawing one fails
                cnt[i] += 1; p += brute(k, n, t - 1, cap, cnt) / avail.len() as f64; cnt[i] -= 1;
            }
            p
        }
        for n in 2..=6 { for k in 1..=n { for t in 0..=6 { for cap in 1..=5 {
            let b = brute(k, n, t, cap, &mut vec![0; n]);
            assert!((miss_draws(k, n, t, cap) - b).abs() < 1e-12, "k {k} n {n} t {t} cap {cap}: {} vs {b}", miss_draws(k, n, t, cap));
        }}}}
        assert!((miss_draws(1, 5, 4, 1) - 0.2).abs() < 1e-12);           // 4 of 5 skills, no repeats
        assert_eq!(miss_draws(2, 5, 4, 1), 0.0);                          // only 3 others to absorb 4 draws
        assert!((miss_draws(2, 5, 3, 1) - 0.1).abs() < 1e-12);           // 1 / C(5, 3)
        assert!((miss_draws(1, 7, 2, 2) - (6.0f64 / 7.0).powi(2)).abs() < 1e-12);   // 2 draws, cap 2: no skill leaves before the last
        // fractional h: floor / ceil mixture; levels per draw 1 -> cap 5, 3 or 4 -> 2, 5 -> 1
        assert!((hint_miss(1, 5, 4.25, 5) - (0.75 * 0.2 + 0.25 * 0.0)).abs() < 1e-12);
        assert!((hint_miss(1, 5, 2.0, 1) - 0.64).abs() < 1e-12);
    }
    /// Stop risk (IT's default choice ends the chain on some outcomes) is the exact mixture over where it stops.
    #[test]
    fn chain_stop_risk_is_a_mixture() {
        let r12 = stop_risk_code(&[(1, 1, 2), (2, 1, 2)], 3);          // ends on 1 of 2 outcomes at links 1 and 2
        let r2 = stop_risk_code(&[(2, 1, 2)], 3);
        assert_eq!(stop_risk_code(&[(2, 1, 2), (3, 1, 2)], 2), 0);     // links at or past the stop don't matter
        let full = chain_marginals(&[(3, 3, 0)])[&(3, 3, 0)].clone();
        let alone = chain_marginals(&[(3, 3, r12)])[&(3, 3, r12)].clone();
        assert!((alone[0] - full[0]).abs() < 1e-15 && (alone[1] - 0.5 * full[1]).abs() < 1e-15 && (alone[2] - 0.25 * full[2]).abs() < 1e-15, "{alone:?}");
        // with a partner: 50% the deck is (full, full), 50% the risky card stops after link 2 and frees its slot
        let mix = chain_marginals(&[(3, 3, 0), (3, 3, r2)]);
        let a = chain_marginals(&[(3, 3, 0), (3, 3, 0)]); let b = chain_marginals(&[(3, 3, 0), (3, 2, 0)]);
        for k in 0..3 { assert!((mix[&(3, 3, 0)][k] - (0.5 * a[&(3, 3, 0)][k] + 0.5 * b[&(3, 3, 0)][k])).abs() < 1e-15); }
        assert!((mix[&(3, 3, r2)][2] - 0.5 * a[&(3, 3, 0)][2]).abs() < 1e-15);
        assert!(mix[&(3, 3, 0)][2] > a[&(3, 3, 0)][2]);
    }

    fn job_with(v: Value) -> DeckJob { serde_json::from_value(v).unwrap() }
    fn halo() -> Card {
        let cr: CardsRaw = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web_cards.json")).unwrap()).unwrap();
        cr.cards.into_iter().find(|c| c.id == 30056).unwrap()
    }
    /// An event offering targets on different choices gives one deck entry per choice, each with one choice's
    /// rewards; with only one of them a target, the card keeps a single entry with that choice.
    #[test]
    fn choices_become_variants() {
        let c = halo();
        // King Halo [Tonight, We Waltz], link 1: Stop Right There! (choice 1) or Uma Stan (choice 2)
        let dj = job_with(json!({"alts": [[30056, 1, [[[[20102, 1]]], [[[20159, 1]]]]]], "events": [[30056, 20102, 1, 1], [30056, 20159, 1, 1]]}));
        let unc = [20159u32, 20102];
        let sb = Sources { dj: &dj, unc: &unc };
        let vs = sb.variants(&c, None);
        assert_eq!(vs.len(), 2);
        for (i, &v) in vs.iter().enumerate() {
            let (mut mw, mut mg) = (vec![1.0; 2], vec![1.0; 2]);
            src_miss(&sb.src(&c, None, v), &[1.0, 1.0, 1.0], &mut mw, &mut mg);
            // with every link firing, exactly one of the two whites is certain
            let got: Vec<bool> = mw.iter().map(|&x| x < 1e-12).collect();
            assert_eq!(got.iter().filter(|&&x| x).count(), 1, "variant {i}: {mw:?}");
        }
        let one = [20159u32];
        let sb1 = Sources { dj: &dj, unc: &one };
        assert_eq!(sb1.variants(&c, None), vec![0]);
        let (mut mw, mut mg) = (vec![1.0], vec![1.0]);
        src_miss(&sb1.src(&c, None, 0), &[1.0, 1.0, 1.0], &mut mw, &mut mg);
        assert!(mw[0] < 1e-12 && mg[0] == 1.0);
    }
    /// One event's outcomes exclude each other: a gold on two of four outcomes and its white on the other two give
    /// a hint whenever the link fires (independent 50% chances would give 75%).
    #[test]
    fn outcomes_of_one_event_are_exclusive() {
        let c = halo();
        let dj = job_with(json!({"alts": [[30056, 3, [[[[20203, 2]], [[20203, 1]], [[20203, 2]], [[20203, 1]]]]]]}));
        let unc = [20203u32];
        let sb = Sources { dj: &dj, unc: &unc };
        let (mut mw, mut mg) = (vec![1.0], vec![1.0]);
        src_miss(&sb.src(&c, None, 0), &[1.0, 1.0, 0.8], &mut mw, &mut mg);
        assert!((mg[0] - 0.6).abs() < 1e-12, "{mg:?}");                  // P(no gold) = 1 - 0.8 * 0.5
        assert!((mg[0] * mw[0] - 0.2).abs() < 1e-12, "{mw:?}");          // P(no hint) = 1 - 0.8
    }
    /// The final-state distribution has the same per-link marginals as `chain_marginals`, stop risk included.
    #[test]
    fn chain_states_match_marginals() {
        let r = stop_risk_code(&[(2, 1, 2)], 3);
        for ks in [vec![(3, 3, 0)], vec![(2, 2, 0), (3, 3, 0), (3, 3, 0)], vec![(3, 2, 0), (3, 3, r), (4, 4, 0)], vec![(2, 2, 0); 6]] {
            let mut ks: Vec<Kind> = ks; ks.sort();
            let st = chain_states(&ks);
            assert!((st.iter().map(|x| x.1).sum::<f64>() - 1.0).abs() < 1e-12);
            let m = chain_marginals(&ks);
            for (i, kd) in ks.iter().enumerate() {
                for l in 1..=kd.1 {
                    let p: f64 = st.iter().filter(|x| x.0[i] >= l).map(|x| x.1).sum();
                    assert!((p - m[kd][l as usize - 1]).abs() < 1e-12, "{ks:?} entry {i} link {l}: {p} vs {}", m[kd][l as usize - 1]);
                }
            }
        }
    }
    /// With no chain cards and no event touching two targets, every source is independent: the exact value equals
    /// the product of per-target odds the search ranks by.
    #[test]
    fn joint_equals_product_without_correlation() {
        let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
        let sk: SkillsRaw = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web_skills.json")).unwrap()).unwrap();
        let sj: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
        let m = Model::new(&d, &sk, &sj);
        let nu = m.uncertain.len();
        let g1 = |k: usize, lvl: u8, p: f64| -> Group { vec![(p, vec![(k, lvl)])] };
        let a = Src { train: vec![(0, 0.3), (2, 0.2)], fixed: vec![g1(1, 2, 0.5), g1(3, 2, 0.5), g1(3, 1, 1.0)], ..Default::default() }.finish();
        let b = Src { train: vec![(1, 0.4)], fixed: vec![g1(0, 2, 0.25)], ..Default::default() }.finish();
        let (mut mw, mut mg) = (vec![1.0; nu], vec![1.0; nu]);
        for x in [&a, &b] { src_miss(x, &[], &mut mw, &mut mg); }
        let dw: Vec<f64> = mw.iter().map(|x| 1.0 - x).collect(); let dg: Vec<f64> = mg.iter().map(|x| 1.0 - x).collect();
        for u in [0usize, 5, 20] {
            for q in [0.3, 0.75] {
                let j = joint_trainee(&m, u, &[q], &[(a.clone(), (0, 0, 0)), (b.clone(), (0, 0, 0))], 0, sj.targets.len());
                let p = m.fixed_factor() * m.trainee_fast(u, q, &dw, &dg);
                assert!((j[0].0 - p).abs() < 1e-14, "uma {u} q {q}: {} vs {p}", j[0].0);
            }
        }
    }
}
