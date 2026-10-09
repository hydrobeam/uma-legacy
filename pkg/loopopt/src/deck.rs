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
use crate::skills::{chart_rows, check_sizes, lins, member_json, Model, SkillJob, SkillsRaw, GEN_GOLD};
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
    /// The character rule. `RULE_OFF=1` (brute-force checks only, val/permcheck.py and owncheck.py) switches it off.
    pub(crate) fn blocks(&self, uma: u32) -> bool {
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        !*OFF.get_or_init(|| std::env::var("RULE_OFF").is_ok()) && (self.chara == uma || self.charas.contains(&uma))
    }
}
#[derive(Deserialize)]
pub(crate) struct CardsRaw { pub cards: Vec<Card>,
    /// cards the game bans from a scenario's deck, by scenario name (master.mdb single_mode_restrict_support via
    /// export_cards.py: Team Sirius in "unity" and "gc")
    #[serde(default)] pub restrict: HashMap<String, Vec<u32>> }

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
    /// measured outcome weights for `alts` events: (card, link, [choice][outcome] -> share); an event not listed here
    /// keeps equally likely outcomes (event_shares.json, applied by the event builders)
    #[serde(default)] pub alt_w: Vec<(u32, u8, Vec<Vec<f64>>)>,
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
    /// per-uma: rank loops and decks by exact odds (EXACT_RHO); false = prune by product odds, as a fast first pass
    /// (what is reported is still scored exactly, `exact.complete` false)
    #[serde(default = "d_true")] pub exact: bool,
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
fn d_true() -> bool { true }
/// Skyline depth of the deck enumeration: 1 (the exact search scores every deck it needs from `DeckSpace`)
fn kmax_top(_dj: &DeckJob) -> usize { 1 }
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
fn scenario_name(s: &str) -> &str {
    match s { "unity" => "Unity Cup", "gc" => "Grand Concert", "ura" => "URA Finale", "trackblazer" => "Trackblazer", "grandmasters" => "Grand Masters", x => x }
}
pub(crate) fn hint_base(scenario: &str) -> f64 {
    // fitted on uma-it IT runs (docs/HINT_RATE_FIT.md, refit 2026-10-05 on 2,402 runs); Grand Masters has no data: a placeholder
    match scenario { "gc" => 2.03, "ura" => 1.60, "trackblazer" => 1.81, "grandmasters" => 2.30, _ => 2.70 }
}
/// P(none of `k` given skills of a card's pool is drawn) in `t` training-hint draws over its `n` skills, each draw
/// uniform over the skills still below Lv 5; a skill leaves the pool after `cap` draws (ceil(5 / hint levels per
/// draw)), so a +5 card never draws a skill twice. Exact over how many draws each other skill has taken.
fn miss_draws(k: usize, n: usize, t: usize, cap: usize) -> f64 {
    if k == 0 { return 1.0; }
    if k >= n { return if t == 0 { 1.0 } else { 0.0 }; }
    if t <= cap { return (1.0 - k as f64 / n as f64).powi(t as i32); }   // no skill can leave the pool before the last draw
    // state: how many of the n - k other skills have taken 0, 1, .., cap draws (cap: left the pool); a sorted map, so
    // sums run in one order (a std HashMap's order changes from run to run, and so did the last bits of every result)
    let mut st: std::collections::BTreeMap<Vec<u8>, f64> = Default::default();
    let mut s0 = vec![0u8; cap + 1]; s0[0] = (n - k) as u8; st.insert(s0, 1.0);
    for _ in 0..t {
        let mut nx: std::collections::BTreeMap<Vec<u8>, f64> = Default::default();
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
/// Cache key of the event-count distribution in use: (n, bits of p). (An XOR of the two collided: (21, 0.35) and
/// (22, 0.7) gave one key.)
fn pmf_key() -> (u32, u64) { CHAIN_EVENTS.with(|c| { let (n, p) = c.get(); (n, p.to_bits()) }) }
/// P(link k of an entry fires), k = 1..=stop, for every chain kind present, given the deck's chain kinds.
/// Dense, deterministic, memoised version of `chain_marginals_ref`. States are mixed-radix indices over the
/// entries' progress; every transition raises the index, so one pass in index order is a topological order.
/// Entries are sorted first so the floating-point summation order is canonical (the HashMap reference sums in
/// iteration order, which varied from run to run in the last bits).
fn chain_marginals(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    thread_local! { static CM: std::cell::RefCell<crate::FxMap<(Vec<Kind>, (u32, u64)), HashMap<Kind, Vec<f64>>>> = std::cell::RefCell::new(Default::default()); }
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
    thread_local! { static CS: std::cell::RefCell<crate::FxMap<(Vec<Kind>, (u32, u64)), std::rc::Rc<Vec<(Vec<u8>, f64)>>>> = std::cell::RefCell::new(Default::default()); }
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
fn joint(model: &Model, u: usize, qs: &[f64], ents: &[(Src, Kind)], filler: usize, n_targets: usize, m1: bool) -> Vec<(f64, f64, usize)> {
    let r = joint_fast(model, u, qs, ents, filler, n_targets, m1, None);
    static CHECK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *CHECK.get_or_init(|| std::env::var("JOINT_CHECK").is_ok()) {
        let x = joint_trainee_ref(model, u, qs, ents, filler, n_targets);
        for (a, b) in r.iter().zip(&x) {
            let rel = |p: f64, q: f64| (p - q).abs() / q.abs().max(1e-300);
            assert!(rel(a.0, b.0) < 1e-11 && (!m1 || rel(a.1, b.1) < 1e-11), "joint mismatch: {:?} vs {:?}", a, b);
        }
    }
    r
}

pub(crate) struct JointExt { pub su: Vec<f64>, pub sf: Vec<f64>, pub marg: Vec<f64>, pub sets: Vec<f64>, pub per_outfit: Vec<f64> }
/// `joint_trainee` without the reference check; `m1` = also compute P(all but one) (else 0).
/// Faster form of the same sum: chain states are collapsed to the progress that changes some target's factors (a
/// filler's progress, and links that add nothing, drop out), and outcome groups touching two or more targets are
/// enumerated per cluster of targets they connect (clusters are independent given the chain state).
/// `ext` (the career planner): each target's landing factor times its spark-generation scale (`su` per uncertain
/// target, `sf` per guaranteed gold, in job order), and per (chance, outfit, uncertain target) the exact chance it
/// lands, summed into `marg`; with `sets` non-empty on entry, per (chance, outfit) the chance of each set of uncertain
/// targets landing (bit k = uncertain target k), 2^nu entries each. None leaves every number as it is.
fn joint_fast(model: &Model, u: usize, qs: &[f64], ents: &[(Src, Kind)], filler: usize, n_targets: usize, m1: bool, mut ext: Option<&mut JointExt>) -> Vec<(f64, f64, usize)> {
    use crate::skills::GEN_WHITE;
    let nu = model.uncertain.len();
    fn take<'g>(g: &'g Group, coupled: &mut Vec<&'g Group>, ng: &mut [f64], nh: &mut [f64]) {
        let gh = group_gh(g);
        if gh.len() >= 2 { coupled.push(g); return; }
        for (k, pg, ph) in gh { ng[k] *= 1.0 - pg; nh[k] *= 1.0 - ph; }
    }
    let (mut bg, mut bh) = (vec![1.0f64; nu], vec![1.0f64; nu]);
    let mut bc: Vec<&Group> = vec![];
    for (src, _) in ents {
        for &(k, p) in &src.train { bh[k] *= 1.0 - p; }
        for g in &src.fixed { take(g, &mut bc, &mut bg, &mut bh); }
    }
    let mut ch: Vec<(Kind, Option<usize>)> = ents.iter().enumerate().filter(|(_, e)| e.1 != (0, 0, 0)).map(|(i, e)| (e.1, Some(i))).collect();
    for _ in 0..filler { ch.push(((2, 2, 0), None)); }
    ch.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let kinds: Vec<Kind> = ch.iter().map(|x| x.0).collect();
    let states = if kinds.is_empty() { std::rc::Rc::new(vec![(vec![], 1.0)]) } else { chain_states(&kinds) };
    // per position with target sources: per links done, (cumulative miss gold, miss any, coupled groups so far)
    let mut lv: Vec<(usize, Vec<(Vec<f64>, Vec<f64>, Vec<&Group>)>, Vec<u8>)> = vec![];
    for (pos, &(kd, e)) in ch.iter().enumerate() {
        let Some(i) = e else { continue };
        let src = &ents[i].0;
        if src.chains.is_empty() { continue; }
        let (mut g, mut h, mut c) = (vec![1.0f64; nu], vec![1.0f64; nu], vec![]);
        let mut out = vec![(g.clone(), h.clone(), c.clone())];
        // class of each links-done count: the first count with the same factors
        let mut cls = vec![0u8];
        for l in 1..=kd.1 {
            let mut changed = false;
            for (cl, grp) in &src.chains { if *cl == l { take(grp, &mut c, &mut g, &mut h); changed = true; } }
            if changed { out.push((g.clone(), h.clone(), c.clone())); }
            cls.push((out.len() - 1) as u8);
        }
        if out.len() > 1 { lv.push((pos, out, cls)); }
    }
    // collapsed states: classes of the positions in `lv`, weights summed in state order
    let mut coll: crate::FxMap<Vec<u8>, f64> = Default::default();
    let mut order: Vec<Vec<u8>> = vec![];
    for (prog, w) in states.iter() {
        let key: Vec<u8> = lv.iter().map(|(pos, _, cls)| cls[prog[*pos] as usize]).collect();
        match coll.get_mut(&key) { Some(x) => *x += w, None => { coll.insert(key.clone(), *w); order.push(key); } }
    }
    let opts = &model.opts[u];
    let no = opts.len(); let nq = qs.len();
    let nfix = model.n_fixed_gold as usize;
    let fixed = match &ext { Some(x) => x.sf.iter().fold(1.0, |a, s| a * GEN_GOLD * s), None => model.fixed_factor() };
    let want_sets = ext.as_ref().map_or(false, |x| !x.sets.is_empty()) && nu <= 16;
    if let Some(x) = ext.as_deref_mut() { x.marg = vec![0.0; nq * no * nu]; if want_sets { x.sets = vec![0.0; nq * no << nu]; } }
    let mut dist: Vec<Vec<f64>> = if want_sets { vec![vec![]; nq * no] } else { vec![] };
    let mut acc = vec![(0.0f64, 0.0f64); nq * no];
    // per (q, outfit, target): factor with no coupled outcome, with a coupled white (any hint), with a coupled gold
    let nqo = nq * no;
    let mut fv = vec![[0.0f64; 3]; nqo * nu];
    let (mut ng, mut nh) = (vec![1.0f64; nu], vec![1.0f64; nu]);
    let mut coupled: Vec<&Group> = vec![];
    let mut cl_of = vec![usize::MAX; nu];
    let mut lvl = vec![0u8; nu];
    for key in &order {
        let w = coll[key];
        ng.copy_from_slice(&bg); nh.copy_from_slice(&bh);
        coupled.clear(); coupled.extend(bc.iter().copied());
        for (j, (_, out, _)) in lv.iter().enumerate() {
            let (g, h, c) = &out[key[j] as usize];
            for k in 0..nu { ng[k] *= g[k]; nh[k] *= h[k]; }
            coupled.extend(c.iter().copied());
        }
        for (qi, &q) in qs.iter().enumerate() {
            for (o, (st, _)) in opts.iter().enumerate() {
                let ew = &model.ews[u][o];
                let base = (qi * no + o) * nu;
                for k in 0..nu {
                    let f = |pg: f64, pnh: f64| match st[k] { 2 => GEN_GOLD, 1 => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg),
                        _ => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg - pnh * (1.0 - ew[k]) * (1.0 - q)) };
                    fv[base + k] = [f(1.0 - ng[k], nh[k]), f(1.0 - ng[k], 0.0), f(1.0, 0.0)];
                    if let Some(x) = &ext { for v in fv[base + k].iter_mut() { *v *= x.su[k]; } }
                }
            }
        }
        // clusters of targets joined by coupled groups (union-find over targets)
        let mut par: Vec<usize> = (0..nu).collect();
        fn root(p: &mut [usize], mut x: usize) -> usize { while p[x] != x { p[x] = p[p[x]]; x = p[x]; } x }
        for g in &coupled {
            let ts = group_gh(g);
            for w2 in ts.windows(2) { let (a, b) = (root(&mut par, w2[0].0), root(&mut par, w2[1].0)); if a != b { par[a.max(b)] = a.min(b); } }
        }
        cl_of.iter_mut().for_each(|x| *x = usize::MAX);
        let mut clusters: Vec<(Vec<usize>, Vec<&Group>)> = vec![];
        for g in &coupled {
            let r = root(&mut par, group_gh(g)[0].0);
            let ci = match cl_of[r] { usize::MAX => { clusters.push((vec![], vec![])); cl_of[r] = clusters.len() - 1; clusters.len() - 1 } c => c };
            clusters[ci].1.push(g);
        }
        for k in 0..nu { if cl_of[root(&mut par, k)] != usize::MAX { let ci = cl_of[root(&mut par, k)]; clusters[ci].0.push(k); } }
        let in_cl: Vec<bool> = (0..nu).map(|k| cl_of[root(&mut par, k)] != usize::MAX).collect();
        // per (q, outfit): (P(all land), P(exactly one misses)) of each independent unit, combined
        let mut ua = vec![1.0f64; nqo]; let mut ub = vec![0.0f64; nqo];
        let push = |ua: &mut [f64], ub: &mut [f64], i: usize, a: f64, b: f64| { ub[i] = ub[i] * a + ua[i] * b; ua[i] *= a; };
        for i in 0..nqo {
            for k in 0..nu { if !in_cl[k] { let f = fv[i * nu + k][0]; push(&mut ua, &mut ub, i, f, 1.0 - f);
                if let Some(x) = ext.as_deref_mut() { x.marg[i * nu + k] += w * f; } } }
        }
        // the landed sets: independent targets first, clusters below (their targets join through `cdist`)
        if want_sets { for i in 0..nqo {
            let d = &mut dist[i]; d.clear(); d.resize(1 << nu, 0.0); d[0] = 1.0;
            for k in 0..nu { if in_cl[k] { continue; } let (f, b) = (fv[i * nu + k][0], 1usize << k);
                for m in 0..(1usize << nu) { if m & b == 0 && d[m] != 0.0 { let x = d[m]; d[m] = x * (1.0 - f); d[m | b] += x * f; } } }
        } }
        for (ts, gs) in &clusters {
            let alts: Vec<Vec<(f64, &[(usize, u8)])>> = gs.iter().map(|g| {
                let mut v: Vec<(f64, &[(usize, u8)])> = g.iter().map(|(p, o)| (*p, &o[..])).collect();
                let rest = 1.0 - g.iter().map(|x| x.0).sum::<f64>();
                if rest > 1e-15 { v.push((rest, &[][..])); }
                v
            }).collect();
            let mut ca = vec![0.0f64; nqo]; let mut cb = vec![0.0f64; nqo];
            // per (chance, outfit): distribution over the subsets of this cluster's targets (as full masks)
            let mut cdist: Vec<crate::FxMap<usize, f64>> = if want_sets { vec![Default::default(); nqo] } else { vec![] };
            let mut pick = vec![0usize; alts.len()];
            loop {
                let wc: f64 = (0..alts.len()).map(|i| alts[i][pick[i]].0).product();
                for &k in ts { lvl[k] = 0; }
                for i in 0..alts.len() { for &(k, l) in alts[i][pick[i]].1 { lvl[k] = lvl[k].max(if l >= 2 { 2 } else { 1 }); } }
                for i in 0..nqo {
                    let (mut a, mut b) = (1.0f64, 0.0f64);
                    for &k in ts { let f = fv[i * nu + k][lvl[k] as usize]; b = b * f + a * (1.0 - f); a *= f; }
                    ca[i] += wc * a; cb[i] += wc * b;
                    if let Some(x) = ext.as_deref_mut() { for &k in ts { x.marg[i * nu + k] += w * wc * fv[i * nu + k][lvl[k] as usize]; } }
                    if want_sets {
                        let mut part: Vec<(usize, f64)> = vec![(0, wc)];
                        for &k in ts { let f = fv[i * nu + k][lvl[k] as usize];
                            part = part.into_iter().flat_map(|(m, p)| [(m, p * (1.0 - f)), (m | 1 << k, p * f)]).collect(); }
                        for (m, p) in part { *cdist[i].entry(m).or_insert(0.0) += p; }
                    }
                }
                let mut i = 0;
                while i < alts.len() { pick[i] += 1; if pick[i] < alts[i].len() { break; } pick[i] = 0; i += 1; }
                if i == alts.len() { break; }
            }
            for i in 0..nqo { push(&mut ua, &mut ub, i, ca[i], cb[i]); }
            if want_sets { for i in 0..nqo {
                let d = &mut dist[i]; let mut nd = vec![0.0f64; 1 << nu];
                for (m0, &x) in d.iter().enumerate() { if x == 0.0 { continue; } for (&m, &p) in &cdist[i] { nd[m0 | m] += x * p; } }
                *d = nd;
            } }
        }
        for i in 0..nqo {
            let (mut a, mut b) = (ua[i], ub[i]);
            if m1 { for j in 0..nfix { let g = match &ext { Some(x) => GEN_GOLD * x.sf[j], None => GEN_GOLD }; b = b * g + a * (1.0 - g); a *= g; } } else { a *= fixed; }
            // P(at least n_targets - 1 land) over nu + nfix targets
            let m = if n_targets <= nu + nfix { if n_targets == 0 { 1.0 } else { a + b } } else { 0.0 };
            acc[i].0 += w * a; acc[i].1 += w * if m1 { m } else { 0.0 };
            if want_sets { if let Some(x) = ext.as_deref_mut() { for (m, &p) in dist[i].iter().enumerate() { x.sets[(i << nu) + m] += w * p; } } }
        }
    }
    if let Some(x) = ext.as_deref_mut() { x.per_outfit = acc.iter().map(|a| a.0).collect(); }
    (0..nq).map(|qi| {
        let mut b = 0;
        for o in 1..no { if acc[qi * no + o].0 > acc[qi * no + b].0 { b = o; } }
        (acc[qi * no + b].0, acc[qi * no + b].1, b)
    }).collect()
}

/// Reference form of `joint_trainee`: every chain state, every combination of coupled outcomes. `JOINT_CHECK=1` runs
/// it beside the fast form on every call and panics on a relative difference above 1e-11.
fn joint_trainee_ref(model: &Model, u: usize, qs: &[f64], ents: &[(Src, Kind)], filler: usize, n_targets: usize) -> Vec<(f64, f64, usize)> {
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

/// Per chain-kind key (sorted kinds, fillers as (2, 2)): for each position, P(links done = l | N = n), n = 0..=nmax,
/// and whether the progress vector given every n is negatively upper-orthant dependent:
///   P(X_i >= t_i for all i) <= prod_i P(X_i >= t_i)   for every threshold vector t,
/// checked exactly (to 1e-12) over the key's state distribution. Early ends and stop risk are included (stop risk as
/// the mixture over where each risky card stops, which does not depend on N).
struct KeyLevel { marg: Vec<Vec<Vec<f64>>>, nuod: bool, lev: Vec<Vec<f64>>, dims: Vec<usize>, gstride: Vec<usize>,
                  pairs: std::cell::RefCell<crate::FxMap<(usize, usize), bool>> }
impl KeyLevel {
    /// P(X_i = a, X_j = b | N = m)
    fn pair_marg(&self, i: usize, j: usize, m: usize) -> Vec<Vec<f64>> {
        let mut v = vec![vec![0.0f64; self.dims[j]]; self.dims[i]];
        for (gi, &p) in self.lev[m].iter().enumerate() { if p != 0.0 { v[(gi / self.gstride[i]) % self.dims[i]][(gi / self.gstride[j]) % self.dims[j]] += p; } }
        v
    }
    /// The orthant check with positions i and j as one block: for every up-set A of the (i, j) grid and every
    /// threshold vector t on the other positions, P((X_i, X_j) in A, X_k >= t_k) <= P((X_i, X_j) in A) prod P(X_k >= t_k).
    /// Any rising function of (X_i, X_j) is a positive combination of up-set indicators, so this is what the bound
    /// needs when one group holds both cards.
    fn pair_ok(&self, i: usize, j: usize) -> bool {
        if let Some(&r) = self.pairs.borrow().get(&(i, j)) { return r; }
        let (di, dj) = (self.dims[i], self.dims[j]);
        // up-sets of the di x dj grid: for each row a, the first column c_a in the set, non-increasing in a
        let mut ups: Vec<Vec<usize>> = vec![];
        fn upsets(a: usize, di: usize, dj: usize, hi: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
            if a == di { out.push(cur.clone()); return; }
            for c in 0..=hi { cur.push(c); upsets(a + 1, di, dj, c, cur, out); cur.pop(); }
        }
        upsets(0, di, dj, dj, &mut vec![], &mut ups);
        let n = self.dims.len();
        let others: Vec<usize> = (0..n).filter(|&k| k != i && k != j).collect();
        let mut ok = true;
        'outer: for m in 0..self.lev.len() {
            let d = &self.lev[m];
            for up in &ups {
                // indicator of the up-set (column c >= up[row]) over the grid, times the distribution
                let mut arr: Vec<f64> = d.iter().enumerate().map(|(gi, &p)| { let a = (gi / self.gstride[i]) % di; let c = (gi / self.gstride[j]) % dj; if c >= up[a] { p } else { 0.0 } }).collect();
                let pa: f64 = arr.iter().sum();
                for &k in &others {
                    for gi in (0..arr.len()).rev() { let c = (gi / self.gstride[k]) % self.dims[k]; if c + 1 < self.dims[k] { arr[gi] += arr[gi + self.gstride[k]]; } }
                }
                for gi in 0..arr.len() {
                    if (gi / self.gstride[i]) % di != 0 || (gi / self.gstride[j]) % dj != 0 { continue; }
                    let prod: f64 = others.iter().map(|&k| self.marg[k][m][(gi / self.gstride[k]) % self.dims[k]..].iter().sum::<f64>()).product();
                    if arr[gi] > pa * prod * (1.0 + 1e-12) + 1e-15 { ok = false; break 'outer; }
                }
            }
        }
        self.pairs.borrow_mut().insert((i, j), ok);
        ok
    }
}
fn key_level(kinds: &[Kind]) -> std::rc::Rc<KeyLevel> {
    thread_local! { static KL: std::cell::RefCell<crate::FxMap<(Vec<Kind>, (u32, u64)), std::rc::Rc<KeyLevel>>> = std::cell::RefCell::new(Default::default()); }
    let key = (kinds.to_vec(), pmf_key());
    if let Some(r) = KL.with(|c| c.borrow().get(&key).cloned()) { return r; }
    let (bn, _) = CHAIN_EVENTS.with(|c| c.get());
    let nmax = bn as usize;
    let n = kinds.len();
    let dims: Vec<usize> = kinds.iter().map(|k| k.1 as usize + 1).collect();
    let mut gstride = vec![1usize; n]; let mut gsize = 1usize;
    for i in 0..n { gstride[i] = gsize; gsize *= dims[i]; }
    // per level, the distribution over the grid of progress vectors
    let mut lev = vec![vec![0.0f64; gsize]; nmax + 1];
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
    let mut pick = vec![0usize; n];
    loop {
        let w: f64 = (0..n).map(|i| alts[i][pick[i]].1).product();
        if w > 0.0 {
            let stops: Vec<u8> = (0..n).map(|i| alts[i][pick[i]].0).collect();
            let total: usize = stops.iter().map(|&x| x as usize).sum();
            let mut stride = vec![0usize; n]; let mut nst = 1usize;
            for i in 0..n { stride[i] = nst; nst *= stops[i] as usize + 1; }
            let mut p = vec![0.0f64; nst]; p[0] = 1.0;
            let mut st = vec![0u8; n];
            for idx in 0..nst {
                let mut r = idx; let mut lvl = 0usize; let mut gi = 0usize;
                for i in 0..n { st[i] = (r % (stops[i] as usize + 1)) as u8; r /= stops[i] as usize + 1; lvl += st[i] as usize; gi += st[i] as usize * gstride[i]; }
                let pi = p[idx];
                if pi == 0.0 { continue; }
                let mut wsum = 0.0f64;
                for i in 0..n { if st[i] < stops[i] { wsum += (kinds[i].0 - st[i]) as f64; } }
                if wsum > 0.0 { for i in 0..n { if st[i] < stops[i] { p[idx + stride[i]] += pi * (kinds[i].0 - st[i]) as f64 / wsum; } } }
                if lvl <= nmax { lev[lvl][gi] += w * pi; }
                if lvl == total { for m in total + 1..=nmax { lev[m][gi] += w * pi; } }
            }
        }
        let mut i = 0;
        while i < n { pick[i] += 1; if pick[i] < alts[i].len() { break; } pick[i] = 0; i += 1; }
        if i == n { break; }
    }
    let mut marg = vec![vec![vec![0.0f64; 0]; nmax + 1]; n];
    let mut nuod = true;
    for m in 0..=nmax {
        let d = &lev[m];
        for i in 0..n {
            let mut v = vec![0.0f64; dims[i]];
            for gi in 0..gsize { v[(gi / gstride[i]) % dims[i]] += d[gi]; }
            marg[i][m] = v;
        }
        if !nuod { continue; }
        // upper-orthant sums by suffix sums along every axis
        let mut up = d.clone();
        for i in 0..n {
            for gi in (0..gsize).rev() { let c = (gi / gstride[i]) % dims[i]; if c + 1 < dims[i] { up[gi] += up[gi + gstride[i]]; } }
        }
        let tail: Vec<Vec<f64>> = (0..n).map(|i| { let mut t = marg[i][m].clone(); for c in (0..dims[i] - 1).rev() { t[c] += t[c + 1]; } t }).collect();
        for gi in 0..gsize {
            let prod: f64 = (0..n).map(|i| tail[i][(gi / gstride[i]) % dims[i]]).product();
            if up[gi] > prod * (1.0 + 1e-12) + 1e-15 { nuod = false; break; }
        }
    }
    let r = std::rc::Rc::new(KeyLevel { marg, nuod, lev, dims, gstride, pairs: Default::default() });
    KL.with(|c| c.borrow_mut().insert(key, r.clone()));
    r
}

/// Proven upper bound on the exact P(all) of trainee `u` with one deck, per inheritance chance in `qs` (fixed factor
/// included), for decks the bound covers; None otherwise. Every card is grouped with the targets its chain feeds;
/// each group's factor is a rising function of that card's progress alone. Given N = n, when the progress vector is
/// negatively upper-orthant dependent (`key_level`, checked exactly per key), the expected product of such
/// functions of distinct coordinates is at most the product of their expectations (write each as a positive
/// combination of indicators {X_i >= t}). Event outcomes and training hints are independent of the allocation, and
/// exclusive outcomes only lower the product (outcome indicators are negatively associated), so each target's
/// outcomes are taken at their marginal chances.
///   U = sum_n P(N = n) prod_cards E[prod_{k fed by its chain} phi_k(X_card) | n] prod_{other k} phi_k
/// Not covered (None): a target fed by two cards' chains, or a key that fails the orthant check.
/// A group's target indicators (which targets one firing hints) are negatively upper-orthant dependent: for every set
/// S of its targets, P(all of S hinted) <= prod over S of P(k hinted). True for outcomes that each give one target
/// (they exclude each other); not for an outcome giving two targets together. Training draws over several targets
/// are checked the same way. Cached per group (by its bits).
fn group_nuod(g: &Group) -> bool {
    thread_local! { static GN: std::cell::RefCell<crate::FxMap<Vec<(u64, Vec<(usize, u8)>)>, bool>> = std::cell::RefCell::new(Default::default()); }
    let key: Vec<(u64, Vec<(usize, u8)>)> = g.iter().map(|(p, o)| (p.to_bits(), o.clone())).collect();
    if let Some(r) = GN.with(|c| c.borrow().get(&key).copied()) { return r; }
    let mut ts: Vec<usize> = g.iter().flat_map(|(_, o)| o.iter().map(|x| x.0)).collect(); ts.sort(); ts.dedup();
    // per outcome, the targets it hints as a bit set
    let outs: Vec<(f64, u32)> = g.iter().map(|(p, o)| (*p, o.iter().fold(0u32, |m, x| m | 1 << ts.iter().position(|&t| t == x.0).unwrap()))).collect();
    let marg: Vec<f64> = (0..ts.len()).map(|i| outs.iter().filter(|o| o.1 >> i & 1 == 1).map(|o| o.0).sum()).collect();
    let ok = ts.len() > 20 || (1u32..1 << ts.len()).filter(|s| s.count_ones() >= 2).all(|s| {
        let joint: f64 = outs.iter().filter(|o| o.1 & s == s).map(|o| o.0).sum();
        joint <= (0..ts.len()).filter(|i| s >> i & 1 == 1).map(|i| marg[i]).product::<f64>() * (1.0 + 1e-12) + 1e-15
    });
    GN.with(|c| c.borrow_mut().insert(key, ok));
    ok
}

fn upper_na(model: &Model, u: usize, qs: &[f64], ents: &[(&Src, Kind)], filler: usize) -> Option<Vec<f64>> {
    use crate::skills::GEN_WHITE;
    let nu = model.uncertain.len();
    // every group touching two or more targets must be negatively upper-orthant dependent, or the per-target
    // marginals below could understate a group whose outcomes hint targets together
    for (src, _) in ents {
        for g in src.fixed.iter().chain(src.chains.iter().map(|x| &x.1)) {
            if group_gh(g).len() >= 2 && !group_nuod(g) { if std::env::var("UPPER_WHY").is_ok() { eprintln!("why: group {:?}", g); } return None; }
        }
    }
    let (mut ng0, mut nh0) = (vec![1.0f64; nu], vec![1.0f64; nu]);
    for (src, _) in ents {
        for &(k, p) in &src.train { nh0[k] *= 1.0 - p; }
        for g in &src.fixed { for (k, pg, ph) in group_gh(g) { ng0[k] *= 1.0 - pg; nh0[k] *= 1.0 - ph; } }
    }
    // chain cards in key order (fillers included): (kind, per target: cumulative (miss gold, miss any) after l links)
    let mut chains: Vec<(Kind, Vec<(usize, Vec<(f64, f64)>)>)> = vec![];
    for (src, kd) in ents {
        if *kd == (0, 0, 0) { continue; }
        let stop = kd.1 as usize;
        let ts: Vec<(usize, Vec<(f64, f64)>)> = src.cm.iter().map(|(k, per)| {
            let mut cum = vec![(1.0f64, 1.0f64); stop + 1];
            for l in 1..=stop { cum[l] = cum[l - 1]; for &(lk, g, h) in per { if lk as usize == l { cum[l].0 *= g; cum[l].1 *= h; } } }
            (*k, cum) }).collect();
        chains.push((*kd, ts));
    }
    for _ in 0..filler { chains.push(((2, 2, 0), vec![])); }
    chains.sort_by(|a, b| a.0.cmp(&b.0));
    let kinds: Vec<Kind> = chains.iter().map(|c| c.0).collect();
    // groups: cards joined by a target their chains both feed; at most one group of two cards, none larger
    let mut feeders: Vec<Vec<usize>> = vec![vec![]; nu];
    for (ci, (_, ts)) in chains.iter().enumerate() { for (k, _) in ts { feeders[*k].push(ci); } }
    let mut pair: Option<(usize, usize)> = None;
    for k in 0..nu {
        match feeders[k].len() {
            0 | 1 => {}
            2 => { let p = (feeders[k][0], feeders[k][1]); if pair.map_or(false, |x| x != p) { return None; } pair = Some(p); }
            _ => { return None; }
        }
    }
    let fed: Vec<bool> = (0..nu).map(|k| !feeders[k].is_empty()).collect();
    let kl = if kinds.is_empty() { None } else { Some(key_level(&kinds)) };
    if let Some(k) = &kl {
        let ok = match pair { None => k.nuod, Some((a, b)) => k.pair_ok(a, b) };
        if !ok { return None; }
    }
    let (bn, _) = CHAIN_EVENTS.with(|c| c.get());
    let pmf = event_pmf(bn as usize);
    let opts = &model.opts[u];
    let fixed = model.fixed_factor();
    let phi = |o: usize, q: f64, k: usize, ng: f64, nh: f64| -> f64 {
        let (st, ew) = (&opts[o].0, &model.ews[u][o]);
        let pg = 1.0 - ng;
        match st[k] { 2 => GEN_GOLD, 1 => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg), _ => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg - nh * (1.0 - ew[k]) * (1.0 - q)) }
    };
    let pm: Option<Vec<Vec<Vec<f64>>>> = match (pair, &kl) { (Some((a, b)), Some(k)) => Some((0..pmf.len()).map(|m| k.pair_marg(a, b, m)).collect()), _ => None };
    Some(qs.iter().map(|&q| {
        let mut best = 0.0f64;
        for o in 0..opts.len() {
            let mut other = 1.0;
            for k in 0..nu { if !fed[k] { other *= phi(o, q, k, ng0[k], nh0[k]); } }
            // per single card and links done: the product over the targets only it feeds
            let single = |ci: usize, l: usize| -> f64 { chains[ci].1.iter().filter(|(k, _)| feeders[*k].len() == 1).map(|(k, cum)| phi(o, q, *k, ng0[*k] * cum[l].0, nh0[*k] * cum[l].1)).product() };
            let comp: Vec<Vec<f64>> = (0..chains.len()).map(|ci| (0..=chains[ci].0 .1 as usize).map(|l| single(ci, l)).collect()).collect();
            // the pair's group: both cards' single targets and the shared ones, over (la, lb)
            let pairv: Option<Vec<Vec<f64>>> = pair.map(|(a, b)| (0..=chains[a].0 .1 as usize).map(|la| (0..=chains[b].0 .1 as usize).map(|lb| {
                let mut v = comp[a][la] * comp[b][lb];
                for (k, cum_a) in &chains[a].1 { if feeders[*k].len() == 2 {
                    let cum_b = &chains[b].1.iter().find(|x| x.0 == *k).unwrap().1;
                    v *= phi(o, q, *k, ng0[*k] * cum_a[la].0 * cum_b[lb].0, nh0[*k] * cum_a[la].1 * cum_b[lb].1);
                } }
                v }).collect()).collect());
            let mut tot = 0.0;
            for (m, &pn) in pmf.iter().enumerate() {
                if pn == 0.0 { continue; }
                let mut prod = other;
                if let Some(k) = &kl {
                    for ci in 0..chains.len() {
                        if chains[ci].1.is_empty() || pair.map_or(false, |(a, b)| ci == a || ci == b) { continue; }
                        prod *= k.marg[ci][m].iter().zip(&comp[ci]).map(|(p, c)| p * c).sum::<f64>();
                    }
                    if let (Some(pv), Some(pmm)) = (&pairv, &pm) {
                        prod *= pmm[m].iter().zip(pv).map(|(r, s)| r.iter().zip(s).map(|(p, c)| p * c).sum::<f64>()).sum::<f64>();
                    }
                }
                tot += pn * prod;
            }
            best = best.max(tot);
        }
        fixed * best * (1.0 + 1e-9)
    }).collect())
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
    let mut aw: Vec<Value> = v["alt_w"].as_array().cloned().unwrap_or_default();
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
            for a in c["alt_w"].as_array().into_iter().flatten() { aw.push(json!([id, a[0], a[1]])); }
            for n in c["always"].as_array().into_iter().flatten() { if let Some((g, lv)) = look(n.as_str().unwrap_or("")) { ev.push(json!([id, g, lv, 0])); } }
            for e in c["chance"].as_array().into_iter().flatten() { if let Some((g, lv)) = look(e[0].as_str().unwrap_or("")) { od.push(json!([id, g, lv, e[1].as_f64().unwrap_or(0.0)])); } }
        }
    }
    v["events"] = Value::Array(ev); v["events_p"] = Value::Array(evp); v["odds_events"] = Value::Array(od);
    v["end_links"] = Value::Array(el); v["ends"] = Value::Array(en); v["stop_risk"] = Value::Array(sr); v["alts"] = Value::Array(al); v["alt_w"] = Value::Array(aw);
}

/// The search ranks decks and loops by the product of per-target odds; the reported ones are then scored exactly
/// (`joint_trainee`), and so are the decks and loops whose ranking value is at least this share of the best. On the
/// reference setups the exact value was 1.00-1.19 times the product (the most for 0-1-pal decks), so 0.8 covers a
/// gap well beyond that.
/// Assumed cap on a deck's exact odds over its product odds, for the per-uma search: decks and loops are scored exactly
/// wherever their product-odds ceiling times this could beat the best exact value. Largest ratio seen: 1.17 (Grand
/// Masters, all owned, 0-1-pal decks); every deck scored reports its ratio (`exact.max_ratio`).
const EXACT_RHO: f64 = 1.25;
/// Caps tried in turn, highest first: the answer is certified under the first whose pass finishes within budget.
const EXACT_STEPS: [f64; 3] = [EXACT_RHO, 1.12, 1.05];
/// Most decks one exact deck search may return; beyond it the answer is the best found and `exact.complete` is false.
const EXACT_CAP: usize = 50_000;
/// Work budget of the exact per-uma search, in units of about 0.1 us natively (2 per partial deck tested, 25 per
/// bound, 200 per exact score, 10,000 per deck search): about 3 s for the pass at EXACT_RHO. A pass that runs out
/// certifies nothing; the next lower step gets a quarter more (see `run_multi`). Jobs where many decks lie within
/// the cap of each other (high inheritance chances, few open targets, borrowing) can exhaust it.
const EXACT_BUDGET: usize = 30_000_000;
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
        let w = self.dj.alt_w.iter().find(|w| w.0 == id && w.1 == link).map(|w| &w.2);   // measured shares, else equal
        let nu = self.unc.len();
        let gs: Vec<Group> = a.2.iter().enumerate().map(|(ci, ch)| {
            let n = ch.len().max(1) as f64;
            ch.iter().enumerate().map(|(oi, out)| {
                let mut r: Vec<(usize, u8)> = out.iter().filter_map(|&(g, lv)| self.unc.iter().position(|&x| x == g).map(|k| (k, lv))).collect();
                r.sort(); r.dedup();
                (w.and_then(|w| w.get(ci)).and_then(|c| c.get(oi)).copied().unwrap_or(1.0 / n), r)
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

thread_local! { static UCHK: std::cell::Cell<(usize, f64, usize)> = std::cell::Cell::new((0, 0.0, 0)); static SPT: std::cell::Cell<(f64, f64, usize, usize)> = std::cell::Cell::new((0.0, 0.0, 0, 0)); }
/// Every legal deck of one trainee, without the skyline's pruning (which compares product odds only). Used to make
/// the per-uma search exact: decks are enumerated by chain-kind key, like the skyline search, with an upper bound on
/// a partial deck's product value (each remaining slot of a kind group at that group's best miss chances per target,
/// ignoring the one-per-character rule), so only the decks that can reach a threshold are built.
struct DeckSpace<'a> {
    dj: &'a DeckJob, unc: Vec<u32>, sk: &'a SkillsRaw,
    cards: Vec<&'a Card>,
    // entries (as in run_deck_relaxed, before card dominance drops any): card index, chain kind, early end, variant
    rel: Vec<usize>, kind: Vec<Kind>, endk: Vec<u8>, vark: Vec<u32>, srcs: Vec<Src>, ebor: Vec<bool>,
    // pal characters with no relevant card: (id, name, card)
    plain: Vec<(u32, String, &'a Card)>,
    locked: Vec<usize>, allowed_borrow: usize,
    // every chain kind of any entry plus fillers (2, 2): key codes are counts in base 7 over this list
    kinds_all: Vec<Kind>,
    keys: std::cell::RefCell<crate::FxMap<u64, std::rc::Rc<(Vec<f64>, Vec<f64>)>>>,
    red: std::cell::RefCell<crate::FxMap<Vec<usize>, (std::rc::Rc<Vec<usize>>, usize)>>,
    gmc: std::cell::RefCell<crate::FxMap<(usize, u64), std::rc::Rc<Vec<Vec<(Vec<f64>, Vec<f64>)>>>>>,
    /// partial decks tested, over every search (the search's cost)
    nodes: std::cell::Cell<usize>,
}
#[derive(Clone)]
struct Sel { ents: Vec<usize>, plain: usize, filler: usize, dw: Vec<f64>, dg: Vec<f64> }
impl<'a> DeckSpace<'a> {
    /// per-entry miss chances (white given no gold, gold), entry-major, under the chain odds of key `code`
    fn key_miss(&self, code: u64) -> std::rc::Rc<(Vec<f64>, Vec<f64>)> {
        if let Some(r) = self.keys.borrow().get(&code) { return r.clone(); }
        let nu = self.unc.len(); let n = self.rel.len();
        let mut ks: Vec<Kind> = vec![]; let mut c = code;
        for &kd in &self.kinds_all { for _ in 0..(c % 7) { ks.push(kd); } c /= 7; }
        let marg = chain_marginals(&ks);
        let empty: Vec<f64> = vec![];
        let mut mw = vec![1.0f64; n * nu]; let mut mg = vec![1.0f64; n * nu];
        for j in 0..n { src_miss(&self.srcs[j], marg.get(&self.kind[j]).unwrap_or(&empty), &mut mw[j * nu..(j + 1) * nu], &mut mg[j * nu..(j + 1) * nu]); }
        let r = std::rc::Rc::new((mw, mg));
        self.keys.borrow_mut().insert(code, r.clone());
        r
    }
    /// Entries that matter for trainees `us` (one per-uma trainee, or a loop's four with a shared deck): legal ones (no
    /// card of any of them), with entries that are interchangeable for them merged. Targets every outfit of each has as
    /// a gold are constant in both the product and the exact value, so an entry is judged by its rewards for the other
    /// targets: entries of one character with the same such rewards and chain kind keep the first; entries with none
    /// (they only take a slot) keep, per chain kind, pal status and borrowing, deck_size of them, characters no
    /// rewarding entry uses first. Any deck has an equal-valued deck from these.
    fn reduced(&self, model: &Model, us: &[usize], ids: &[u32]) -> Vec<usize> {
        let nu = self.unc.len();
        let relt: Vec<bool> = (0..nu).map(|k| us.iter().any(|&u| model.opts[u].iter().any(|(st, _)| st[k] < 2))).collect();
        let pg = |g: &Group| -> Group { g.iter().map(|(p, o)| (*p, o.iter().copied().filter(|x| relt[x.0]).collect::<Vec<_>>())).collect() };
        let proj = |x: &Src| -> String {
            let tr: Vec<(usize, u64)> = x.train.iter().filter(|t| relt[t.0]).map(|t| (t.0, t.1.to_bits())).collect();
            let fx: Vec<Group> = x.fixed.iter().map(|g| pg(g)).filter(|g| g.iter().any(|o| !o.1.is_empty())).collect();
            let ch: Vec<(u8, Group)> = x.chains.iter().map(|(l, g)| (*l, pg(g))).filter(|(_, g)| g.iter().any(|o| !o.1.is_empty())).collect();
            let bits = |g: &Group| -> Vec<(u64, Vec<(usize, u8)>)> { g.iter().map(|(p, o)| (p.to_bits(), o.clone())).collect() };
            format!("{:?}|{:?}|{:?}", tr, fx.iter().map(bits).collect::<Vec<_>>(), ch.iter().map(|(l, g)| (*l, bits(g))).collect::<Vec<_>>())
        };
        let legal: Vec<usize> = (0..self.rel.len()).filter(|&j| ids.iter().all(|&id| !self.cards[self.rel[j]].blocks(id))).collect();
        let keys: Vec<String> = legal.iter().map(|&j| proj(&self.srcs[j])).collect();
        let empty = proj(&Src::default());
        let is_locked = |j: usize| self.locked.contains(&self.rel[j]);
        let mut keep: Vec<usize> = vec![];
        let mut seen: Vec<(u32, Kind, &String)> = vec![];
        let mut used_chara: Vec<u32> = vec![];
        for (i, &j) in legal.iter().enumerate() {
            if keys[i] == empty && !is_locked(j) { continue; }
            let ch = self.cards[self.rel[j]].chara;
            if !is_locked(j) && seen.iter().any(|x| x.0 == ch && x.1 == self.kind[j] && x.2 == &keys[i]) { continue; }
            seen.push((ch, self.kind[j], &keys[i])); keep.push(j); used_chara.push(ch);
        }
        let mut cls: Vec<((Kind, bool, bool), Vec<usize>)> = vec![];
        for (i, &j) in legal.iter().enumerate() {
            if keys[i] != empty || is_locked(j) { continue; }
            let c = (self.kind[j], self.cards[self.rel[j]].pal, self.ebor[j]);
            match cls.iter_mut().find(|x| x.0 == c) { Some(x) => x.1.push(j), None => cls.push((c, vec![j])) }
        }
        let size = self.dj.deck_size;
        for (_, mem) in &cls {
            let mut pick: Vec<usize> = vec![]; let mut chs: Vec<u32> = vec![];
            for pass in 0..2 {
                for &j in mem {
                    if pick.len() >= size { break; }
                    let ch = self.cards[self.rel[j]].chara;
                    if chs.contains(&ch) || pick.contains(&j) { continue; }
                    if pass == 0 && used_chara.contains(&ch) { continue; }
                    pick.push(j); chs.push(ch);
                }
            }
            keep.extend(pick);
        }
        keep.sort();
        keep
    }
    /// (reduced entries, id of the trainee set for other caches)
    fn reduced_cached(&self, model: &Model, us: &[usize], ids: &[u32]) -> (std::rc::Rc<Vec<usize>>, usize) {
        if let Some(r) = self.red.borrow().get(us) { return r.clone(); }
        let id = self.red.borrow().len();
        let r = (std::rc::Rc::new(self.reduced(model, us, ids)), id);
        self.red.borrow_mut().insert(us.to_vec(), r.clone());
        r
    }
    fn gm_cached(&self, u: usize, code: u64, groups: &[&Vec<usize>], mw: &[f64], mg: &[f64], nu: usize, size: usize) -> std::rc::Rc<Vec<Vec<(Vec<f64>, Vec<f64>)>>> {
        if let Some(r) = self.gmc.borrow().get(&(u, code)) { return r.clone(); }
        let r = std::rc::Rc::new(groups.iter().map(|l| {
            let mut pre = vec![(vec![1.0f64; nu], vec![1.0f64; nu]); size + 1];
            for t in 0..nu {
                let mut a: Vec<f64> = l.iter().map(|&j| mw[j * nu + t]).collect(); a.sort_by(|x, y| x.partial_cmp(y).unwrap());
                let mut b: Vec<f64> = l.iter().map(|&j| mg[j * nu + t]).collect(); b.sort_by(|x, y| x.partial_cmp(y).unwrap());
                for r in 1..=size { pre[r].0[t] = pre[r - 1].0[t] * a.get(r - 1).copied().unwrap_or(1.0); pre[r].1[t] = pre[r - 1].1[t] * b.get(r - 1).copied().unwrap_or(1.0); }
            }
            pre }).collect::<Vec<_>>());
        self.gmc.borrow_mut().insert((u, code), r.clone());
        r
    }
    fn legal_plain(&self, ids: &[u32]) -> Vec<&(u32, String, &'a Card)> { self.plain.iter().filter(|x| ids.iter().all(|&id| !x.2.blocks(id))).collect() }
    /// Decks legal for trainees `us` (uma indices; `ids` their ids). `clauses`: ((trainee position in `us`,
    /// inheritance chance) pairs, threshold); a deck's clause value is the sum of its product values (fixed factor
    /// included) over those pairs. `best`: return the one deck
    /// with the highest value of the single clause above `seed` (none if no deck beats it; ties: the first found).
    /// Otherwise return every deck
    /// for which some clause has rho * value >= threshold; at most `cap` decks (the flag says the cap was hit).
    fn search(&self, model: &Model, us: &[usize], ids: &[u32], clauses: &[(Vec<(usize, f64)>, f64)], best: bool, seed: f64, rho: f64, cap: usize) -> (Vec<(f64, Sel)>, bool) {
        let nu = self.unc.len(); let size = self.dj.deck_size;
        if self.locked.iter().any(|&l| ids.iter().any(|&id| self.cards[l].blocks(id))) { return (vec![], false); }
        let (legal, set_id) = self.reduced_cached(model, us, ids);
        let legal: Vec<usize> = legal.to_vec();
        let nplain = self.legal_plain(ids).len();
        let mut chara_ix: HashMap<u32, usize> = HashMap::new();
        for &j in &legal { let n = chara_ix.len(); chara_ix.entry(self.cards[self.rel[j]].chara).or_insert(n); }
        if chara_ix.len() > 128 { return (vec![], true); }
        let ebit: Vec<u128> = (0..self.rel.len()).map(|j| chara_ix.get(&self.cards[self.rel[j]].chara).map_or(0, |&c| 1u128 << c)).collect();
        let nk = self.kinds_all.len();
        let ki22 = self.kinds_all.iter().position(|&k| k == (2, 2, 0)).unwrap();
        let kpos: Vec<u64> = (0..nk).map(|i| 7u64.pow(i as u32)).collect();
        let by_kind: Vec<Vec<usize>> = (0..nk).map(|i| legal.iter().copied().filter(|&j| self.kind[j] != (0, 0, 0) && self.kind[j] == self.kinds_all[i]).collect()).collect();
        let pal_ents: Vec<usize> = legal.iter().copied().filter(|&j| self.kind[j] == (0, 0, 0)).collect();
        let fixed = model.fixed_factor();
        let val = |cl: &(Vec<(usize, f64)>, f64), dw: &[f64], dg: &[f64]| fixed * cl.0.iter().map(|&(k, q)| model.trainee_fast(us[k], q, dw, dg)).sum::<f64>();
        let t_cfg = Instant::now();
        let mut out: Vec<(f64, Sel)> = vec![];
        let mut bestv = if best { seed } else { f64::NEG_INFINITY };
        let mut over = false;
        let mut counts: Vec<Vec<usize>> = vec![vec![]];
        for _ in 0..nk { counts = counts.into_iter().flat_map(|c| { let used: usize = c.iter().sum(); (0..=size - used).map(move |x| { let mut d = c.clone(); d.push(x); d }) }).collect(); }
        // every (key, real (2, 2) entries, kind-0 entries) with its groups and root bound; visited best bound first
        struct Cfg { root: f64, code: u64, filler: usize, plain: usize, gs: Vec<(usize, usize)>, gm: std::rc::Rc<Vec<Vec<(Vec<f64>, Vec<f64>)>>>, suf: Vec<(Vec<f64>, Vec<f64>)>, rpk: std::rc::Rc<Vec<Vec<Vec<f64>>>>, sp: Vec<Vec<f64>> }
        let groups_all: Vec<&Vec<usize>> = by_kind.iter().chain(std::iter::once(&pal_ents)).collect();
        // locked cards each group can hold (a configuration whose groups cannot hold them all is skipped)
        let lb = |j: usize| self.locked.iter().position(|&l| l == self.rel[j]).map_or(0u8, |b| 1u8 << b);
        let all_lk: u8 = if self.locked.is_empty() { 0 } else { ((1u16 << self.locked.len()) - 1) as u8 };
        let glk: Vec<u8> = groups_all.iter().map(|l| l.iter().fold(0u8, |m, &j| m | lb(j))).collect();
        let mut cfgs: Vec<Cfg> = vec![];
        let mut dw = vec![0.0f64; nu]; let mut dg = vec![0.0f64; nu];
        // (clause, chance, outfit) triples; the product value of one outfit at one chance is submodular in the set of
        // cards (log of each target's factor gains less from a card the more the deck already has), so a deck's value
        // is at most a partial deck's value times the best ratios P({card}) / P({}) of the cards still to add
        // (clause, trainee, chance, outfit), each (clause, trainee, chance) a run of its trainee's outfits: og
        let oq: Vec<(usize, usize, f64, usize)> = clauses.iter().enumerate().flat_map(|(ci, cl)| cl.0.iter().flat_map(move |&(k, q)| (0..model.n_outfits(us[k])).map(move |o| (ci, us[k], q, o)))).collect();
        let og: Vec<(usize, usize, usize)> = { let mut v = vec![]; let mut i = 0; while i < oq.len() { let no = model.n_outfits(oq[i].1); v.push((oq[i].0, i, no)); i += no; } v };
        let zero = vec![0.0f64; nu];
        let p0: Vec<f64> = oq.iter().map(|&(_, u, q, o)| model.outfit_fast(u, o, q, &zero, &zero)).collect();
        for cnt in &counts {
            let csum: usize = cnt.iter().sum();
            if (0..nk).any(|i| i != ki22 && cnt[i] > by_kind[i].len()) { continue; }
            let code: u64 = cnt.iter().zip(&kpos).map(|(&c, &p)| c as u64 * p).sum();
            let km = self.key_miss(code);
            let (mw, mg) = (&km.0, &km.1);
            // per kind group, the lowest miss chances of any entry under this key
            // per kind group and target: products of the r lowest miss chances of distinct entries, r = 0..=size
            // (any r entries of the group miss at least that much)
            // per kind group and target: products of the r lowest miss chances of distinct entries, r = 0..=size
            // (any r entries of the group miss at least that much); cached per trainee and key
            let gm = self.gm_cached(set_id, code, &groups_all, mw, mg, nu, size);
            // per kind group and (clause, chance, outfit): products of the r best single-card ratios, r = 0..=size;
            // built only for keys with a configuration the per-target bound lets through
            let mut rpk: Option<std::rc::Rc<Vec<Vec<Vec<f64>>>>> = None;
            let make_rpk = || -> Vec<Vec<Vec<f64>>> { groups_all.iter().map(|l| {
                let mut x = vec![0.0f64; nu]; let mut y = vec![0.0f64; nu];
                let rat: Vec<Vec<f64>> = l.iter().map(|&j| {
                    for t in 0..nu { x[t] = 1.0 - mw[j * nu + t]; y[t] = 1.0 - mg[j * nu + t]; }
                    oq.iter().enumerate().map(|(i, &(_, u, q, o))| if p0[i] > 0.0 { model.outfit_fast(u, o, q, &x, &y) / p0[i] } else { f64::INFINITY }).collect()
                }).collect();
                (0..oq.len()).map(|i| {
                    let mut r: Vec<f64> = rat.iter().map(|v| v[i]).collect(); r.sort_by(|a, b| b.partial_cmp(a).unwrap());
                    let mut pre = vec![1.0f64; size + 1];
                    for k in 1..=size { pre[k] = pre[k - 1] * r.get(k - 1).copied().unwrap_or(1.0); }
                    pre
                }).collect()
            }).collect() };
            for real22 in 0..=cnt[ki22].min(by_kind[ki22].len()) {
                let filler = cnt[ki22] - real22;
                let rem = size - csum;
                for np in 0..=rem.min(pal_ents.len()) {
                    let plain = rem - np;
                    if plain > nplain { continue; }
                    if let Some(p) = self.dj.pals { if np + plain < p || plain > p { continue; } }
                    let mut gs: Vec<(usize, usize)> = vec![];
                    for i in 0..nk { let c = if i == ki22 { real22 } else { cnt[i] }; if c > 0 { gs.push((i, c)); } }
                    if np > 0 { gs.push((nk, np)); }
                    if all_lk & !gs.iter().fold(0u8, |m, &(i, _)| m | glk[i]) != 0 { continue; }
                    let ng = gs.len();
                    // the per-target bound first (suf[0], the same products in the same order as the suffix table below),
                    // the tables only for configurations it lets through
                    for t in 0..nu {
                        let (mut a, mut b) = (1.0f64, 1.0f64);
                        for g in (0..ng).rev() { a *= gm[gs[g].0][gs[g].1].0[t]; b *= gm[gs[g].0][gs[g].1].1[t]; }
                        dw[t] = 1.0 - a; dg[t] = 1.0 - b;
                    }
                    let ideal = if best { val(&clauses[0], &dw, &dg) * (1.0 + 1e-9) >= bestv } else { clauses.iter().any(|cl| rho * val(cl, &dw, &dg) * (1.0 + 1e-9) >= cl.1) };
                    if !ideal { continue; }
                    let mut suf: Vec<(Vec<f64>, Vec<f64>)> = vec![(vec![1.0; nu], vec![1.0; nu]); ng + 1];
                    for g in (0..ng).rev() { for t in 0..nu { suf[g].0[t] = suf[g + 1].0[t] * gm[gs[g].0][gs[g].1].0[t]; suf[g].1[t] = suf[g + 1].1[t] * gm[gs[g].0][gs[g].1].1[t]; } }
                    if rpk.is_none() { rpk = Some(std::rc::Rc::new(make_rpk())); }
                    let rpk = rpk.as_ref().unwrap();
                    let mut sp: Vec<Vec<f64>> = vec![vec![1.0; oq.len()]; ng + 1];
                    for g in (0..ng).rev() { for i in 0..oq.len() { sp[g][i] = sp[g + 1][i] * rpk[gs[g].0][i][gs[g].1]; } }
                    // root bound: per (chance, outfit) the lower of the per-target bound and the ratio bound
                    let mut cv = vec![0.0f64; clauses.len()];
                    for &(ci, i, no) in &og {
                        let (_, u, q, _) = oq[i]; let mut m = 0.0f64;
                        for o in 0..no { m = m.max(model.outfit_fast(u, o, q, &dw, &dg).min(p0[i + o] * sp[0][i + o])); }
                        cv[ci] += fixed * m;
                    }
                    let root = if best { cv[0] } else { (0..clauses.len()).map(|ci| rho * cv[ci] / clauses[ci].1.max(1e-300)).fold(f64::NEG_INFINITY, f64::max) };
                    cfgs.push(Cfg { root, code, filler, plain, gs, gm: gm.clone(), suf, rpk: rpk.clone(), sp });
                }
            }
        }
        // stable: equal bounds keep enumeration order
        cfgs.sort_by(|a, b| b.root.partial_cmp(&a.root).unwrap());
        SPT.with(|c| { let mut v = c.get(); v.0 += t_cfg.elapsed().as_secs_f64(); v.2 += cfgs.len(); v.3 += counts.len(); c.set(v); });
        let t_dfs = Instant::now();
        // A deck holds at most `allowed_borrow` (0 or 1) borrowed cards; the tables above take each group's best entries
        // whatever they are, so with borrowing they count several borrowed cards at once (every card the player lacks
        // is one), which leaves the bound loose. Per (key, group): the products of its r best owned entries, and of
        // r - 1 owned with its best borrowed one, for the miss chances (lowest first) and the ratios (highest first).
        struct GB { own: Vec<(Vec<f64>, Vec<f64>)>, one: Vec<(Vec<f64>, Vec<f64>)>, rown: Vec<Vec<f64>>, rone: Vec<Vec<f64>> }
        let mut gbc: HashMap<(u64, usize), std::rc::Rc<GB>> = HashMap::new();
        let gb_of = |code: u64, gi: usize, mw: &[f64], mg: &[f64]| -> GB {
            let l = groups_all[gi];
            let (own, bor): (Vec<usize>, Vec<usize>) = l.iter().partition(|&&j| !self.ebor[j]);
            let mut om = vec![(vec![1.0f64; nu], vec![1.0f64; nu]); size + 1];
            let mut bm = (vec![1.0f64; nu], vec![1.0f64; nu]);
            for t in 0..nu {
                let mut a: Vec<f64> = own.iter().map(|&j| mw[j * nu + t]).collect(); a.sort_by(|x, y| x.partial_cmp(y).unwrap());
                let mut b: Vec<f64> = own.iter().map(|&j| mg[j * nu + t]).collect(); b.sort_by(|x, y| x.partial_cmp(y).unwrap());
                for r in 1..=size { om[r].0[t] = om[r - 1].0[t] * a.get(r - 1).copied().unwrap_or(1.0); om[r].1[t] = om[r - 1].1[t] * b.get(r - 1).copied().unwrap_or(1.0); }
                for &j in &bor { bm.0[t] = bm.0[t].min(mw[j * nu + t]); bm.1[t] = bm.1[t].min(mg[j * nu + t]); }
            }
            let mut one = om.clone();
            for r in 1..=size { for t in 0..nu { one[r].0[t] = om[r].0[t].min(om[r - 1].0[t] * bm.0[t]); one[r].1[t] = om[r].1[t].min(om[r - 1].1[t] * bm.1[t]); } }
            let (mut x, mut y) = (vec![0.0f64; nu], vec![0.0f64; nu]);
            let mut ratio = |j: usize| -> Vec<f64> {
                for t in 0..nu { x[t] = 1.0 - mw[j * nu + t]; y[t] = 1.0 - mg[j * nu + t]; }
                oq.iter().enumerate().map(|(i, &(_, u, q, o))| if p0[i] > 0.0 { model.outfit_fast(u, o, q, &x, &y) / p0[i] } else { f64::INFINITY }).collect()
            };
            let ro: Vec<Vec<f64>> = own.iter().map(|&j| ratio(j)).collect();
            let rb: Vec<Vec<f64>> = bor.iter().map(|&j| ratio(j)).collect();
            let (mut rown, mut rone) = (vec![], vec![]);
            for i in 0..oq.len() {
                let mut r: Vec<f64> = ro.iter().map(|v| v[i]).collect(); r.sort_by(|a, b| b.partial_cmp(a).unwrap());
                let best_b = rb.iter().map(|v| v[i]).fold(f64::NEG_INFINITY, f64::max);
                let mut pre = vec![1.0f64; size + 1];
                for k in 1..=size { pre[k] = pre[k - 1] * r.get(k - 1).copied().unwrap_or(1.0); }
                let mut p1 = pre.clone();
                if best_b > f64::NEG_INFINITY { for k in 1..=size { p1[k] = pre[k].max(pre[k - 1] * best_b); } }
                rown.push(pre); rone.push(p1);
            }
            let _ = code;
            GB { own: om, one, rown, rone }
        };
        for cf in &cfgs {
            if best && cf.root * (1.0 + 1e-9) < bestv { break; }
            if !best && cf.root * (1.0 + 1e-9) < 1.0 { break; }
            let km = self.key_miss(cf.code);
            let (mw, mg) = (&km.0, &km.1);
            let (filler, plain) = (cf.filler, cf.plain);
            let gs: Vec<(&[usize], usize)> = cf.gs.iter().map(|&(i, c)| (&groups_all[i][..], c)).collect();
            // per group of this configuration: its key's tables (gmin[g] = gm[group], rp[g] = rpk[group])
            let gmin: Vec<&Vec<(Vec<f64>, Vec<f64>)>> = cf.gs.iter().map(|&(i, _)| &cf.gm[i]).collect();
            let rp: Vec<&Vec<Vec<f64>>> = cf.gs.iter().map(|&(i, _)| &cf.rpk[i]).collect();
            let (suf, sp) = (&cf.suf, &cf.sp);
            // the one-borrowed-card tables of this configuration's groups and their suffixes (`own`: no borrowed card
            // left to place; `one`: at most one, in one group)
            let one_b = self.allowed_borrow == 1 && gs.iter().any(|(l, _)| l.iter().any(|&j| self.ebor[j]));
            let gbs: Vec<std::rc::Rc<GB>> = if one_b { cf.gs.iter().map(|&(i, _)| gbc.entry((cf.code, i)).or_insert_with(|| std::rc::Rc::new(gb_of(cf.code, i, mw, mg))).clone()).collect() } else { vec![] };
            let ngs = cf.gs.len();
            let (mut so, mut s1) = (vec![(vec![1.0f64; nu], vec![1.0f64; nu]); ngs + 1], vec![(vec![1.0f64; nu], vec![1.0f64; nu]); ngs + 1]);
            let (mut po, mut p1) = (vec![vec![1.0f64; oq.len()]; ngs + 1], vec![vec![1.0f64; oq.len()]; ngs + 1]);
            if one_b { for g in (0..ngs).rev() { let (gb, c) = (&gbs[g], cf.gs[g].1);
                for t in 0..nu {
                    so[g].0[t] = gb.own[c].0[t] * so[g + 1].0[t]; so[g].1[t] = gb.own[c].1[t] * so[g + 1].1[t];
                    s1[g].0[t] = (gb.one[c].0[t] * so[g + 1].0[t]).min(gb.own[c].0[t] * s1[g + 1].0[t]);
                    s1[g].1[t] = (gb.one[c].1[t] * so[g + 1].1[t]).min(gb.own[c].1[t] * s1[g + 1].1[t]);
                }
                for i in 0..oq.len() { po[g][i] = gb.rown[i][c] * po[g + 1][i]; p1[g][i] = (gb.rone[i][c] * po[g + 1][i]).max(gb.rown[i][c] * p1[g + 1][i]); }
            } }
            let mut cv = vec![0.0f64; clauses.len()];
            let mut pdw = vec![0.0f64; nu]; let mut pdg = vec![0.0f64; nu];
            let ng = gs.len();
            {
                {
                    struct St { sel: Vec<usize>, pw: Vec<[f64; 32]>, pg: Vec<[f64; 32]> }
                    let mut st = St { sel: vec![], pw: vec![[1.0; 32]], pg: vec![[1.0; 32]] };
                    // can a deck from here (group g with `left` still to pick, `pw`/`pg` the partial miss) pass?
                    let mut hopeful = |g: usize, left: usize, pw: &[f64; 32], pg: &[f64; 32], bestv: f64, nb: usize| -> bool {
                        // borrowed cards still allowed (only with the one-borrowed tables)
                        let can = nb < 1;
                        for t in 0..nu {
                            let (a, b) = if g >= ng { (1.0, 1.0) }
                                else if !one_b { (gmin[g][left].0[t] * suf[g + 1].0[t], gmin[g][left].1[t] * suf[g + 1].1[t]) }
                                else if can { let gb = &gbs[g];
                                    ((gb.one[left].0[t] * so[g + 1].0[t]).min(gb.own[left].0[t] * s1[g + 1].0[t]), (gb.one[left].1[t] * so[g + 1].1[t]).min(gb.own[left].1[t] * s1[g + 1].1[t])) }
                                else { (gbs[g].own[left].0[t] * so[g + 1].0[t], gbs[g].own[left].1[t] * so[g + 1].1[t]) };
                            dw[t] = 1.0 - pw[t] * a; dg[t] = 1.0 - pg[t] * b;
                            pdw[t] = 1.0 - pw[t]; pdg[t] = 1.0 - pg[t];
                        }
                        cv.iter_mut().for_each(|x| *x = 0.0);
                        for &(ci, i, no) in &og {
                            let (_, u, q, _) = oq[i]; let mut m = 0.0f64;
                            for o in 0..no {
                                let ix = i + o;
                                let r = if g >= ng { 1.0 } else if !one_b { rp[g][ix][left] * sp[g + 1][ix] }
                                    else if can { (gbs[g].rone[ix][left] * po[g + 1][ix]).max(gbs[g].rown[ix][left] * p1[g + 1][ix]) }
                                    else { gbs[g].rown[ix][left] * po[g + 1][ix] };
                                m = m.max(model.outfit_fast(u, o, q, &dw, &dg).min(model.outfit_fast(u, o, q, &pdw, &pdg) * r));
                            }
                            cv[ci] += fixed * m;
                        }
                        if best { cv[0] * (1.0 + 1e-9) >= bestv }
                        else { (0..clauses.len()).any(|ci| rho * cv[ci] * (1.0 + 1e-9) >= clauses[ci].1) }
                    };
                    // locked cards as bits; avail[g][p]: locked cards with an entry at position >= p of group g or in a
                    // later group (a branch that can no longer place every locked card is dropped)
                    let lbit: Vec<u8> = (0..self.rel.len()).map(|j| self.locked.iter().position(|&l| l == self.rel[j]).map_or(0, |b| 1u8 << b)).collect();
                    let all_locked: u8 = if self.locked.is_empty() { 0 } else { ((1u16 << self.locked.len()) - 1) as u8 };
                    let mut avail: Vec<Vec<u8>> = vec![vec![]; ng + 1];
                    avail[ng] = vec![0];
                    for g in (0..ng).rev() {
                        let l = gs[g].0; let mut v = vec![0u8; l.len() + 1]; v[l.len()] = avail[g + 1][0];
                        for p in (0..l.len()).rev() { v[p] = v[p + 1] | lbit[l[p]]; }
                        avail[g] = v;
                    }
                    if all_locked & !avail[0][0] != 0 { continue; }
                    fn rec(sp: &DeckSpace, gs: &[(&[usize], usize)], g: usize, start: usize, left: usize, chm: u128, nb: usize, have: u8, lk: (&[u8], u8, &[Vec<u8>]), st: &mut St, ebit: &[u128],
                           mw: &[f64], mg: &[f64], nu: usize, hopeful: &mut dyn FnMut(usize, usize, &[f64; 32], &[f64; 32], f64, usize) -> bool,
                           leaf: &mut dyn FnMut(&[usize], &[f64; 32], &[f64; 32]) -> bool, bestv: &dyn Fn() -> f64) -> bool {
                        let (lbit, all_locked, avail) = lk;
                        if left == 0 {
                            if g + 1 >= gs.len() { let (pw, pg) = (*st.pw.last().unwrap(), *st.pg.last().unwrap()); return leaf(&st.sel, &pw, &pg); }
                            return rec(sp, gs, g + 1, 0, gs[g + 1].1, chm, nb, have, lk, st, ebit, mw, mg, nu, hopeful, leaf, bestv);
                        }
                        if all_locked & !have & !avail[g][start] != 0 { return false; }
                        let list = gs[g].0;
                        for p in start..list.len() {
                            if list.len() - p < left { break; }
                            let j = list[p];
                            if chm & ebit[j] != 0 || (sp.ebor[j] && nb >= sp.allowed_borrow) { continue; }
                            sp.nodes.set(sp.nodes.get() + 1);
                            let (mut w2, mut g2) = (*st.pw.last().unwrap(), *st.pg.last().unwrap());
                            for t in 0..nu { w2[t] *= mw[j * nu + t]; g2[t] *= mg[j * nu + t]; }
                            if !hopeful(g, left - 1, &w2, &g2, bestv(), nb + sp.ebor[j] as usize) { continue; }
                            st.sel.push(j); st.pw.push(w2); st.pg.push(g2);
                            let stop = rec(sp, gs, g, p + 1, left - 1, chm | ebit[j], nb + sp.ebor[j] as usize, have | lbit[j], lk, st, ebit, mw, mg, nu, hopeful, leaf, bestv);
                            st.sel.pop(); st.pw.pop(); st.pg.pop();
                            if stop { return true; }
                        }
                        false
                    }
                    let bcell = std::cell::Cell::new(bestv);
                    let mut leaf = |sel: &[usize], pw: &[f64; 32], pg: &[f64; 32]| -> bool {
                        let pals = sel.iter().filter(|&&j| self.cards[self.rel[j]].pal).count() + plain;
                        if let Some(p) = self.dj.pals { if p != pals { return false; } }
                        if !self.locked.iter().all(|l| sel.iter().any(|&j| self.rel[j] == *l)) { return false; }
                        let dw: Vec<f64> = (0..nu).map(|t| 1.0 - pw[t]).collect(); let dg: Vec<f64> = (0..nu).map(|t| 1.0 - pg[t]).collect();
                        let mk = |v: f64| { let mut e = sel.to_vec(); e.sort(); (v, Sel { ents: e, plain, filler, dw: dw.clone(), dg: dg.clone() }) };
                        if best {
                            let v = val(&clauses[0], &dw, &dg);
                            if v > bcell.get() { bcell.set(v); out.clear(); out.push(mk(v)); }
                            false
                        } else {
                            let v = clauses.iter().map(|cl| val(cl, &dw, &dg)).fold(f64::NEG_INFINITY, f64::max);
                            if clauses.iter().any(|cl| rho * val(cl, &dw, &dg) * (1.0 + 1e-9) >= cl.1) {
                                out.push(mk(v));
                                if out.len() >= cap { return true; }
                            }
                            false
                        }
                    };
                    let bget = || bcell.get();
                    let stop = if gs.is_empty() { leaf(&[], &[1.0; 32], &[1.0; 32]) }
                        else { rec(self, &gs, 0, 0, gs[0].1, 0, 0, 0, (&lbit, all_locked, &avail), &mut st, &ebit, mw, mg, nu, &mut hopeful, &mut leaf, &bget) };
                    bestv = bcell.get();
                    if stop { over = true; return (out, over); }
                }
            }
        }
        SPT.with(|c| { let mut v = c.get(); v.1 += t_dfs.elapsed().as_secs_f64(); c.set(v); });
        (out, over)
    }
    fn pdeck(&self, x: &Sel, ids: &[u32]) -> PDeck {
        let srcb = Sources { dj: self.dj, unc: &self.unc };
        let lp = self.legal_plain(ids);
        let ci: Vec<usize> = x.ents.iter().map(|&j| self.rel[j]).collect();
        let borrowed = x.ents.iter().copied().find(|&j| self.ebor[j]).map(|j| self.rel[j]);
        let mut names: Vec<String> = x.ents.iter().map(|&j| {
            let c = self.cards[self.rel[j]]; let e = self.endk[j];
            let mut n = c.name.clone();
            let lb = srcb.label(c, (e > 0).then_some(e), self.vark[j], self.sk);
            if !lb.is_empty() { n = format!("{} ({})", n, lb); }
            if e > 0 { n = format!("{} (end chain at link {})", n, e); }
            if Some(self.rel[j]) == borrowed { n = format!("{} (borrowed)", n); }
            n
        }).collect();
        names.extend(lp.iter().take(x.plain).map(|p| p.1.clone()));
        for _ in 0..x.filler { names.push("(any SR without target hints)".into()); }
        PDeck { names, ids: ci.iter().map(|&i| self.cards[i].id).collect(), ends: x.ents.iter().map(|&j| self.endk[j]).collect(),
                vars: x.ents.iter().map(|&j| self.vark[j]).collect(),
                choices: x.ents.iter().map(|&j| srcb.label(self.cards[self.rel[j]], (self.endk[j] > 0).then_some(self.endk[j]), self.vark[j], self.sk)).collect(),
                plain_ids: lp.iter().take(x.plain).map(|p| p.0).collect(),
                pal_names: ci.iter().filter(|&&i| self.cards[i].pal).map(|&i| self.cards[i].name.clone()).chain(lp.iter().take(x.plain).map(|p| p.1.clone())).collect(),
                pals: x.ents.iter().filter(|&&j| self.cards[self.rel[j]].pal).count() + x.plain,
                borrowed: borrowed.map(|i| self.cards[i].name.clone()), dw: x.dw.clone(), dg: x.dg.clone() }
    }
}

/// Deck entries of a job: per entry the card (`rel`, an index into `cards`, the cards a deck may hold), chain kind,
/// early-end link (0 = played normally), choice variant and sources; the plain pals (owned pal characters with no
/// relevant card, best-rarity card) and the locked cards. `pool_ids`: the umas that may train (plain pals that block
/// none of them come first).
struct Entries<'a> { cards: Vec<&'a Card>, rel: Vec<usize>, kind: Vec<Kind>, endk: Vec<u8>, vark: Vec<u32>, srcs: Vec<Src>,
                     plain_ids: Vec<u32>, plain_names: Vec<String>, plain_chars: Vec<u32>, locked: Vec<usize> }
fn deck_entries<'a>(cr: &'a CardsRaw, dj: &DeckJob, unc: &[u32], pool_ids: &[u32]) -> Result<Entries<'a>, Value> {
    let is_owned = |id: u32| dj.owned_cards.is_empty() || dj.owned_cards.contains(&id) || dj.lock_cards.contains(&id);
    // a card the scenario bans can't be in the deck, owned, borrowed or locked
    let banned: &[u32] = cr.restrict.get(&dj.scenario).map_or(&[], |v| v.as_slice());
    if let Some(c) = dj.lock_cards.iter().find(|id| banned.contains(id)).and_then(|id| cr.cards.iter().find(|c| c.id == *id)) {
        return Err(json!({"error": format!("{} can't be used in {}: the game bans it from that scenario's deck.", c.name, scenario_name(&dj.scenario))}));
    }
    let owned_c = |id: u32| (is_owned(id) || dj.borrow) && !dj.exclude_cards.contains(&id) && !banned.contains(&id);
    // ---- card sources ----
    let srcb = Sources { dj, unc };
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
    Ok(Entries { cards, rel, kind, endk, vark, srcs, plain_ids, plain_names, plain_chars, locked })
}
fn deck_space<'a>(cr: &'a CardsRaw, dj: &'a DeckJob, sk: &'a SkillsRaw, unc: &[u32], e: &Entries<'a>) -> DeckSpace<'a> {
    let is_owned = |id: u32| dj.owned_cards.is_empty() || dj.owned_cards.contains(&id) || dj.lock_cards.contains(&id);
    let card_of: HashMap<u32, &Card> = cr.cards.iter().map(|c| (c.id, c)).collect();
    let mut kinds_all: Vec<Kind> = e.kind.iter().copied().filter(|&k| k != (0, 0, 0)).collect();
    kinds_all.push((2, 2, 0)); kinds_all.sort(); kinds_all.dedup();
    DeckSpace { dj, unc: unc.to_vec(), sk, cards: e.cards.clone(), rel: e.rel.clone(), kind: e.kind.clone(), endk: e.endk.clone(), vark: e.vark.clone(),
        srcs: e.srcs.clone(), ebor: e.rel.iter().map(|&r| !is_owned(e.cards[r].id)).collect(),
        plain: e.plain_ids.iter().zip(&e.plain_names).map(|(&id, n)| (id, n.clone(), card_of[&id])).collect(),
        locked: e.locked.clone(), allowed_borrow: if dj.borrow { 1 } else { 0 }, kinds_all, keys: Default::default(), red: Default::default(), gmc: Default::default(), nodes: Default::default() }
}

mod career;
pub(crate) use career::run_career;

/// Deck + loop search. The character rule (a trainee cannot run a card of her own character, nor a group card she is
/// a member of, `Card::blocks`) is part of which decks are legal in the exact search (`DeckSpace`): per trainee with
/// per-uma decks, for all four members with a shared deck.
pub(crate) fn run_deck(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    run_deck_relaxed(d, sk, cr, v)
}

fn run_deck_relaxed(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    let sj: SkillJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    let dj: DeckJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    CHAIN_EVENTS.with(|c| c.set(dj.chain_events.unwrap_or((CHAIN_N, CHAIN_P))));
    let model = Model::new(d, sk, &sj);
    let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
    let nu = unc.len();
    let is_owned = |id: u32| dj.owned_cards.is_empty() || dj.owned_cards.contains(&id) || dj.lock_cards.contains(&id);
    let pool_ids: Vec<u32> = d.umas.iter().map(|u| u.id).filter(|&id| (sj.owned.is_empty() || sj.owned.contains(&id) || sj.require.contains(&id)) && !sj.exclude.contains(&id)).collect();
    let ent = match deck_entries(cr, &dj, &unc, &pool_ids) { Ok(e) => e, Err(e) => return e };
    // every entry, before card dominance drops any: the exact per-uma search enumerates decks from these
    let space = if !dj.deck_only { Some(deck_space(cr, &dj, sk, &unc, &ent)) } else { None };
    let Entries { cards, mut rel, mut kind, mut endk, mut vark, mut srcs, plain_ids, plain_names, plain_chars, locked } = ent;
    let srcb = Sources { dj: &dj, unc: &unc };

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
    let ident_of = |c: usize, e: u8, x: u32| ((c as u64 + 3) << 40) | ((x as u64) << 8) | e as u64;
    // a deck's identity from a selection of entries
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
    // the decks no other deck beats on every dominance coordinate (the exact search scores every deck it needs from
    // `DeckSpace`, so one skyline serves any `top_decks`)
    let front: Vec<DeckEval> = run_enum(1, None);
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
    {
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
        return run_multi(d, cr, &model, &sj, &dj, pd, stats, space.as_ref().unwrap());
    }
}

/// Per-uma decks: each trainee runs her own best legal deck, the same one in all the lineages she heads (three in a
/// four-uma loop, one in a three-uma rotation).
///   value(f) = fixed/L * sum_t max over decks D legal for f[t] of sum over her lineages l of T(f[t], q_l, D), L = 12 or 3
/// The skyline (decks no deck beats on every coordinate) holds every uma's best deck as long as none of its decks
/// carries a card that blocks her; for a uma it does, her legal decks come from a second deck search without
/// those cards, run the first time a foursome with her is solved. Bounds use the whole skyline (legal or not),
/// which no legal deck beats, so the search is exact.
fn run_multi(d: &Data, cr: &CardsRaw, model: &Model, sj: &SkillJob, dj: &DeckJob, sky: Vec<PDeck>, stats: Value, space: &DeckSpace) -> Value {
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
    if let Err(e) = check_sizes(&sj.sizes, pool.len(), req.len()) { return json!({"error": e}); }
    let (with3, with4) = (sj.sizes.contains(&3), sj.sizes.contains(&4));
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
    // Structures: four-uma loops (12 lineages) and three-uma rotations (3 lineages; `lins`), members as [usize; 4]
    // with `NONE` last for three; a structure's value is the mean over its lineages, so both sizes rank together.
    let lin_of = |f: &[usize; 4]| lins(members(f).len());
    let fixed = model.fixed_factor();
    let qmax: Vec<f64> = (0..n).map(|u| if pool.contains(&u) { model.q_ub_trainee(u, &pool, &pb, with3) } else { 0.0 }).collect();
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
    // ---- exact per-uma values ----
    // Loops are ranked by exact odds. A trainee's exact value at her lineages' inheritance chances is the best exact
    // P(all) over every legal deck (`DeckSpace`, not only the skyline). It is seeded with her best legal skyline decks
    // by product odds; then every legal deck whose product value times EXACT_RHO reaches the best exact value is scored
    // exactly. That is exact under one assumption: no deck's exact odds exceed EXACT_RHO times its product odds.
    let srcb = Sources { dj, unc: &unc };
    let ntar = sj.targets.len();
    let ents_of = |x: &PDeck| -> (Vec<(Src, Kind)>, usize) {
        let e: Vec<(Src, Kind)> = x.ids.iter().enumerate().filter_map(|(i, id)| card_of.get(id).map(|c| {
            let st = x.ends.get(i).copied().filter(|&e| e > 0); let v = x.vars.get(i).copied().unwrap_or(0);
            (srcb.src(c, st, v), srcb.kind(c, st)) })).collect();
        (e, dj.deck_size.saturating_sub(x.ids.len() + x.plain_ids.len()))
    };
    struct Best { sum: f64, deck: Option<PDeck>, per: Vec<(f64, f64, usize)> }
    // per (uma, chances): the best found, decks already scored or bounded, and the cap it is exact under
    struct ExEntry { best: Option<(f64, PDeck)>, done: crate::FxSet<(Vec<u32>, Vec<u8>, Vec<u32>, usize)>, seen: crate::FxSet<(Vec<usize>, usize, usize)>, level: f64, result: Option<std::rc::Rc<Best>> }
    let ex_cache: std::cell::RefCell<crate::FxMap<(usize, Vec<u64>), ExEntry>> = Default::default();
    let n_scored = std::cell::Cell::new(0usize); let n_band = std::cell::Cell::new(0usize);
    let cap_hit = std::cell::Cell::new(false); let max_ratio = std::cell::Cell::new(0.0f64);
    let (t_band, t_score, t_seed) = (std::cell::Cell::new(0.0f64), std::cell::Cell::new(0.0f64), std::cell::Cell::new(0.0f64));
    let work = std::cell::Cell::new(0usize); let n_ufilt = std::cell::Cell::new(0usize); let t_ub = std::cell::Cell::new(0.0f64);
    let budget = std::env::var("EXACT_BUDGET").ok().and_then(|s| s.parse().ok()).unwrap_or(EXACT_BUDGET);
    let budget_c = std::cell::Cell::new(budget);
    let spend = |w: usize| work.set(work.get() + w);
    // set when a pass runs out of budget (or a deck search out of room): that pass certifies nothing
    let aborted = std::cell::Cell::new(false);
    // Exact value of trainee u at chances qs under cap lv: the best exact P(all) over her best legal skyline decks
    // and every legal deck whose product value times lv reaches it (each first through the proven bound).
    let exact_best = |u: usize, qs: &[f64], lv: f64| -> std::rc::Rc<Best> {
        let key = (u, qs.iter().map(|q| q.to_bits()).collect::<Vec<u64>>());
        if let Some(e) = ex_cache.borrow().get(&key) { if e.level >= lv { if let Some(r) = &e.result { return r.clone(); } } }
        let mut ent = ex_cache.borrow_mut().remove(&key).unwrap_or(ExEntry { best: None, done: Default::default(), seen: Default::default(), level: -1.0, result: None });
        let uid = d.umas[u].id;
        let tsum = |x: &PDeck| fixed * qs.iter().map(|&q| model.trainee_fast(u, q, &x.dw, &x.dg)).sum::<f64>();
        let score = |x: &PDeck, ent: &mut ExEntry| {
            if !ent.done.insert((x.ids.clone(), x.ends.clone(), x.vars.clone(), x.plain_ids.len())) { return; }
            let t0 = Instant::now();
            let (e, fl) = ents_of(x);
            let j = joint(model, u, qs, &e, fl, ntar, false);
            t_score.set(t_score.get() + t0.elapsed().as_secs_f64());
            if std::env::var("UPPER_CHECK").is_ok() {
                let er: Vec<(&Src, Kind)> = e.iter().map(|(s, k)| (s, *k)).collect();
                if let Some(ub) = upper_na(model, u, qs, &er, fl) {
                    for (i, y) in j.iter().enumerate() {
                        assert!(ub[i] >= y.0 * (1.0 - 1e-12), "upper bound below exact: {} < {} for {:?}", ub[i], y.0, x.names);
                        UCHK.with(|c| { let mut v = c.get(); v.0 += 1; v.1 = v.1.max(ub[i] / y.0); c.set(v); });
                    }
                } else { UCHK.with(|c| { let mut v = c.get(); v.2 += 1; c.set(v); }); }
            }
            n_scored.set(n_scored.get() + 1); spend(200);
            for (i, &q) in qs.iter().enumerate() {
                let p = fixed * model.trainee_fast(u, q, &x.dw, &x.dg);
                if p > 0.0 { max_ratio.set(max_ratio.get().max(j[i].0 / p)); }
            }
            let sv: f64 = j.iter().map(|y| y.0).sum();
            if ent.best.as_ref().map_or(true, |b| sv > b.0) { ent.best = Some((sv, x.clone())); }
        };
        if ent.level < 0.0 {
            // seed: her best legal skyline decks by product value (or the best legal deck when none is on the skyline)
            let t0 = Instant::now();
            let mut seeds: Vec<(f64, &PDeck)> = uview(u).iter().map(|&j| &sky[j]).filter(|x| !x.blocks(uid, &card_of)).map(|x| (tsum(x), x)).collect();
            seeds.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            t_seed.set(t_seed.get() + t0.elapsed().as_secs_f64());
            for (_, x) in seeds.iter().take(3) { score(x, &mut ent); }
            if ent.best.is_none() {
                let (r, _) = space.search(model, &[u], &[uid], &[(qs.iter().map(|&q| (0, q)).collect(), 0.0)], true, f64::NEG_INFINITY, 1.0, 1);
                if let Some((_, sel)) = r.first() { let x = space.pdeck(sel, &[uid]); score(&x, &mut ent); }
            }
            ent.level = 1.0;
        }
        // every legal deck that might beat it under the cap lv
        let mut complete = true;
        if lv > ent.level { if let Some((thr, _)) = ent.best.clone() {
            n_band.set(n_band.get() + 1);
            let t0 = Instant::now();
            let n0 = space.nodes.get();
            let (mut cands, over) = space.search(model, &[u], &[uid], &[(qs.iter().map(|&q| (0, q)).collect(), thr)], false, f64::NEG_INFINITY, lv, EXACT_CAP);
            spend(2 * (space.nodes.get() - n0) + 5 * cands.len() + 10_000);
            t_band.set(t_band.get() + t0.elapsed().as_secs_f64());
            if over { cap_hit.set(true); complete = false; }
            cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            if complete { for (v, sel) in &cands {
                if lv * v * (1.0 + 1e-9) < ent.best.as_ref().unwrap().0 { break; }
                if work.get() > budget_c.get() { complete = false; break; }
                if !ent.seen.insert((sel.ents.clone(), sel.plain, sel.filler)) { continue; }
                // the proven bound first: a deck it puts below the best exact value cannot beat it
                let e: Vec<(&Src, Kind)> = sel.ents.iter().map(|&j| (&space.srcs[j], space.kind[j])).collect();
                let t1 = Instant::now();
                let ubo = upper_na(model, u, qs, &e, sel.filler);
                t_ub.set(t_ub.get() + t1.elapsed().as_secs_f64());
                if let Some(ub) = ubo {
                    spend(25);
                    if ub.iter().sum::<f64>() * (1.0 + 1e-9) < ent.best.as_ref().unwrap().0 { n_ufilt.set(n_ufilt.get() + 1); continue; }
                }
                let x = space.pdeck(sel, &[uid]);
                score(&x, &mut ent);
            } }
            if complete { ent.level = lv; } else { aborted.set(true); }
        } else { ent.level = lv; } }
        let b = match &ent.best {
            Some((sum, x)) => { let (e, fl) = ents_of(x); let per = joint(model, u, qs, &e, fl, ntar, true); Best { sum: *sum, deck: Some(x.clone()), per } }
            None => { let per = joint(model, u, qs, &[], 0, ntar, true); Best { sum: per.iter().map(|y| y.0).sum(), deck: None, per } }
        };
        let b = std::rc::Rc::new(b);
        ent.result = Some(b.clone());
        ex_cache.borrow_mut().insert(key, ent);
        b
    };
    // exact ceiling table: X[u][i] = best exact value over every legal deck at chance grid[i] under the cap it was
    // computed for (rises with the chance, so a lineage with chance q is bounded at grid[gidx(q)]; each lineage may
    // take its own deck)
    let xtab_c: std::cell::RefCell<Vec<[(f64, f64); NG]>> = std::cell::RefCell::new(vec![[(f64::NAN, 0.0); NG]; n]);
    let xtab = |u: usize, i: usize, lv: f64| -> f64 {
        let (v, l) = xtab_c.borrow()[u][i];
        if !v.is_nan() && l >= lv { return v; }
        let v = exact_best(u, &[grid[i]], lv).sum;
        // a value from a deck search the budget cut short is not exact under lv: not kept
        if !aborted.get() { xtab_c.borrow_mut()[u][i] = (v, lv); }
        v
    };
    // Ceiling of a cell (uma, grid point) under cap lv: the exact value if computed under lv or more, else the lowest
    // such value at a higher grid point, else the product-odds bound times lv; (bound, exact?)
    let cell = |u: usize, i: usize, lv: f64| -> (f64, bool) {
        let row = xtab_c.borrow()[u];
        let mut b = lv * fixed * btab(u, i) * (1.0 + 1e-12);
        for j in i..NG { if !row[j].0.is_nan() && row[j].1 >= lv { b = b.min(row[j].0 * (1.0 + 1e-12)); return (b, j == i); } }
        (b, false)
    };
    // Clusters of the whole skyline, and the best m skyline decks by `val` among those passing `ok` (cluster maxima
    // bound their members, best bound first; `val` must rise with every odds coordinate)
    let full_cl: Vec<Cl> = if dj.per_uma { vec![] } else { let mut ix: Vec<usize> = (0..sky.len()).collect(); let mut out = vec![]; if nu > 0 && !ix.is_empty() { split(&sky, &mut ix, nu, &mut out); } else { out.push(Cl { mdw: vec![], mdg: vec![], mem: ix }); } out };
    let sky_top = |m: usize, val: &dyn Fn(&[f64], &[f64]) -> f64, ok: &dyn Fn(&PDeck) -> bool| -> Vec<(f64, usize)> {
        let mut bs: Vec<(f64, usize)> = full_cl.iter().enumerate().map(|(i, c)| (if nu > 0 { val(&c.mdw, &c.mdg) } else { f64::INFINITY }, i)).collect();
        bs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let mut top: Vec<(f64, usize)> = vec![];
        for (b, ci) in bs {
            if top.len() >= m && b * (1.0 + 1e-9) < top[m - 1].0 { break; }
            for &j in &full_cl[ci].mem {
                if !ok(&sky[j]) { continue; }
                let v = val(&sky[j].dw, &sky[j].dg);
                if top.len() < m || v > top[m - 1].0 {
                    let pos = top.iter().position(|x| v > x.0).unwrap_or(top.len());
                    top.insert(pos, (v, j)); top.truncate(m);
                }
            }
        }
        top
    };
    // shared-deck ceiling: the best single skyline deck (legal or not) over the loop's twelve lineages, product odds
    let relaxed_shared = |f: [usize; 4], qa: &[f64; 12]| -> f64 {
        let ln = lin_of(&f);
        let val = |dw: &[f64], dg: &[f64]| (0..ln.len()).map(|l| model.trainee_fast(f[ln[l].0], qa[l], dw, dg)).sum::<f64>();
        sky_top(1, &val, &|_| true).first().map_or(0.0, |x| x.0) * fixed / ln.len() as f64 * (1.0 + 1e-12)
    };
    // ---- shared deck ----
    // With one deck for the whole loop, a loop's value is the best exact sum over its twelve lineages among decks legal
    // for all four members. Per loop: every such deck whose exact sum is at least `floor` (the k-th best distinct
    // deck so far, times 12) or among the loop's own best `top_decks`, under cap lv; sorted, best first. Ceilings
    // stay the per-uma ones (each trainee's best deck bounds her share of any shared deck).
    let shared = !dj.per_uma;
    let kloops = dj.top_decks.max(1);
    type SList = std::rc::Rc<Vec<(f64, PDeck)>>;
    let sh_cache: std::cell::RefCell<crate::FxMap<([usize; 4], Vec<u64>), (f64, f64, SList)>> = Default::default();
    let exact_shared = |f: [usize; 4], qa: &[f64], floor: f64, lv: f64| -> SList {
        let key = (f, qa.iter().map(|q| q.to_bits()).collect::<Vec<u64>>());
        if let Some((l, fl, r)) = sh_cache.borrow().get(&key) { if *l >= lv && *fl <= floor { return r.clone(); } }
        let ln = lin_of(&f); let fm = members(&f); let nm = fm.len();
        let us: Vec<usize> = fm.to_vec(); let ids: Vec<u32> = fm.iter().map(|&u| d.umas[u].id).collect();
        let pairs: Vec<(usize, f64)> = (0..ln.len()).map(|l| (ln[l].0, qa[l])).collect();
        let qt: Vec<Vec<f64>> = (0..nm).map(|t| (0..ln.len()).filter(|&l| ln[l].0 == t).map(|l| qa[l]).collect()).collect();
        let mut done: crate::FxSet<(Vec<u32>, Vec<u8>, Vec<u32>, usize)> = Default::default();
        let mut list: Vec<(f64, PDeck)> = vec![];
        let mut score = |x: &PDeck, list: &mut Vec<(f64, PDeck)>| {
            if !done.insert((x.ids.clone(), x.ends.clone(), x.vars.clone(), x.plain_ids.len())) { return; }
            let t0 = Instant::now();
            let (e, fl) = ents_of(x);
            let mut sv = 0.0;
            for t in 0..nm {
                let j = joint(model, f[t], &qt[t], &e, fl, ntar, false);
                for (i, &q) in qt[t].iter().enumerate() {
                    let p = fixed * model.trainee_fast(f[t], q, &x.dw, &x.dg);
                    if p > 0.0 { max_ratio.set(max_ratio.get().max(j[i].0 / p)); }
                    sv += j[i].0;
                }
            }
            t_score.set(t_score.get() + t0.elapsed().as_secs_f64());
            n_scored.set(n_scored.get() + 1); spend(800);
            list.push((sv, x.clone()));
        };
        // the threshold a deck must reach: the floor, or the loop's k-th best so far when that is higher
        let need = |list: &Vec<(f64, PDeck)>| -> f64 {
            let mut v: Vec<f64> = list.iter().map(|x| x.0).collect(); v.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let kl = if v.len() >= kloops { v[kloops - 1] } else { f64::NEG_INFINITY };
            let t = floor.max(kl);
            if t.is_finite() { t } else { v.last().copied().unwrap_or(f64::NEG_INFINITY) }
        };
        let t0 = Instant::now();
        let val = |dw: &[f64], dg: &[f64]| fixed * pairs.iter().map(|&(t, q)| model.trainee_fast(f[t], q, dw, dg)).sum::<f64>();
        let seeds = sky_top(kloops + 2, &val, &|x| ids.iter().all(|&id| !x.blocks(id, &card_of)));
        t_seed.set(t_seed.get() + t0.elapsed().as_secs_f64());
        for &(_, j) in &seeds { score(&sky[j], &mut list); }
        if list.is_empty() {
            let (r, _) = space.search(model, &us, &ids, &[(pairs.clone(), 0.0)], true, f64::NEG_INFINITY, 1.0, 1);
            if let Some((_, sel)) = r.first() { let x = space.pdeck(sel, &ids); score(&x, &mut list); }
        }
        let mut complete = true;
        if lv > 1.0 && !list.is_empty() {
            n_band.set(n_band.get() + 1);
            let t0 = Instant::now();
            let n0 = space.nodes.get();
            let (mut cands, over) = space.search(model, &us, &ids, &[(pairs.clone(), need(&list))], false, f64::NEG_INFINITY, lv, EXACT_CAP);
            spend(2 * (space.nodes.get() - n0) + 5 * cands.len() + 10_000);
            t_band.set(t_band.get() + t0.elapsed().as_secs_f64());
            if over { cap_hit.set(true); complete = false; }
            cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let mut seen: crate::FxSet<(Vec<usize>, usize, usize)> = Default::default();
            if complete { for (v, sel) in &cands {
                let thr = need(&list);
                if lv * v * (1.0 + 1e-9) < thr { break; }
                if work.get() > budget_c.get() { complete = false; break; }
                if !seen.insert((sel.ents.clone(), sel.plain, sel.filler)) { continue; }
                let e: Vec<(&Src, Kind)> = sel.ents.iter().map(|&j| (&space.srcs[j], space.kind[j])).collect();
                let t1 = Instant::now();
                let ub: Option<f64> = (0..nm).map(|t| upper_na(model, f[t], &qt[t], &e, sel.filler).map(|b| b.iter().sum::<f64>())).sum();
                t_ub.set(t_ub.get() + t1.elapsed().as_secs_f64());
                if let Some(ub) = ub { spend(100); if ub * (1.0 + 1e-9) < thr { n_ufilt.set(n_ufilt.get() + 1); continue; } }
                let x = space.pdeck(sel, &ids);
                score(&x, &mut list);
            } }
            if !complete { aborted.set(true); }
        }
        let thr = need(&list);
        list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        list.retain(|x| x.0 >= thr);
        let r: SList = std::rc::Rc::new(list);
        if complete { sh_cache.borrow_mut().insert(key, (lv, floor, r.clone())); }
        r
    };
    // Exact ceiling of a foursome at chances `qa`, refined lazily: while it beats `kth`, the loosest inexact term is
    // tightened (first computed at the uma's highest grid point, then halfway down towards its own) until the ceiling
    // stops beating `kth` or every term is exact. Each step is one exact deck search for one uma.
    let exact_ceiling = |f: [usize; 4], qa: &[f64; 12], kth: f64, lv: f64| -> f64 {
        loop {
            let mut tot = 0.0; let mut worst: Option<(f64, usize, usize)> = None;
            let ln = lin_of(&f);
            for l in 0..ln.len() {
                let (u, i) = (f[ln[l].0], gidx(qa[l]));
                let (b, ex) = cell(u, i, lv); tot += b;
                if !ex && worst.map_or(true, |w| b > w.0) { worst = Some((b, u, i)); }
            }
            let ce = tot / ln.len() as f64;
            let Some((_, u, i)) = worst else { return ce };
            if ce <= kth * (1.0 + 4e-12) || aborted.get() { return ce; }
            let row = xtab_c.borrow()[u];
            let j = (i..NG).find(|&j| !row[j].0.is_nan() && row[j].1 >= lv);
            let tgt = match j { None => i.max(gidx(qmax[u])), Some(j) => i + (j - i) / 2 };
            xtab(u, tgt, lv);
        }
    };
    // ceiling with each uma's best skyline deck (legal or not), the same deck across her lineages
    let relaxed = |f: [usize; 4], qa: &[f64; 12]| -> f64 {
        let mut tot = 0.0;
        let ln = lin_of(&f);
        for t in 0..members(&f).len() {
            let qs: Vec<f64> = (0..ln.len()).filter(|&l| ln[l].0 == t).map(|l| qa[l]).collect();
            let init = qs.iter().map(|&q| model.trainee_fast(f[t], q, &zero, &zero)).sum::<f64>();
            let val = |dw: &[f64], dg: &[f64]| qs.iter().map(|&q| model.trainee_fast(f[t], q, dw, dg)).sum::<f64>();
            tot += if qs.iter().all(|q| (0.0..=1.0).contains(q)) { bb_max(f[t], init, &val) }
                else { uview(f[t]).iter().map(|&j| val(&sky[j].dw, &sky[j].dg)).fold(init, f64::max) };
        }
        fixed * tot / ln.len() as f64 * (1.0 + 1e-12)
    };
    let mval: Vec<f64> = (0..n).map(|u| if pool.contains(&u) { btab(u, gidx(qmax[u])) } else { 0.0 }).collect();
    let mut ord: Vec<usize> = pool.clone();
    ord.sort_by(|&a, &b| mval[b].partial_cmp(&mval[a]).unwrap());
    let mv: Vec<f64> = ord.iter().map(|&u| mval[u]).collect();
    // A structure's product ceiling is at most the guaranteed golds' factor times the mean of its members' M_u (each
    // member heads the same number of lineages): (3/12)·sum for four, sum/3 for three. Bands are in four-member sum
    // units: a three-member structure's sum s counts as e3(s) = 4s/3.
    let sep = |s: f64| fixed * 3.0 * s / 12.0 * (1.0 + 1e-12);
    let e3 = |s: f64| s * 4.0 / 3.0;
    let nreq = req.len();
    let choose = |m: usize, c: usize| (0..c).fold(1usize, |acc, i| acc * (m - i) / (i + 1));
    let total_foursomes: usize = { let m = k - nreq.min(k);
        (if with4 { choose(m, 4usize.saturating_sub(nreq)) } else { 0 }) + (if with3 && nreq <= 3 { choose(m, 3 - nreq) } else { 0 }) };
    let band = |lo: f64, hi: f64, out: &mut Vec<(f64, [usize; 4])>| {
        if with4 { for a in 0..k {
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
                        let ln = lins(4);
                        let qs = model.lineage_q_arr(&f, &|x, y| pb[f[x]][f[y]]);
                        let ub = fixed * (0..12).map(|l| btab(f[ln[l].0], gidx(qs[l]))).sum::<f64>() / 12.0 * (1.0 + 1e-12);
                        out.push((ub, f));
                    }
                }
            }
        } }
        if with3 && nreq <= 3 { for a in 0..k {
            if a + 2 >= k || e3(mv[a] + mv[a + 1] + mv[a + 2]) < lo { break; }
            for b in a + 1..k {
                if b + 1 >= k || e3(mv[a] + mv[b] + mv[b + 1]) < lo { break; }
                for c in b + 1..k {
                    let sm = e3(mv[a] + mv[b] + mv[c]);
                    if sm < lo { break; }
                    if sm >= hi { continue; }
                    let mut t = [ord[a], ord[b], ord[c]]; t.sort();
                    let f = [t[0], t[1], t[2], NONE];
                    if !req.iter().all(|r| f.contains(r)) { continue; }
                    let ln = lins(3);
                    let qs = model.lineage_q_arr(&t, &|x, y| pb[t[x]][t[y]]);
                    let ub = fixed * (0..3).map(|l| btab(t[ln[l].0], gidx(qs[l]))).sum::<f64>() / 3.0 * (1.0 + 1e-12);
                    out.push((ub, f));
                }
            }
        } }
    };
    let smax = (if with4 && k >= 4 { mv[0] + mv[1] + mv[2] + mv[3] } else { 0.0 }).max(if with3 && k >= 3 { e3(mv[0] + mv[1] + mv[2]) } else { 0.0 });
    // schedules found, per foursome (shared by the passes)
    let sched_c: std::cell::RefCell<crate::FxMap<[usize; 4], Res>> = Default::default();
    struct Pass { solved: Vec<([usize; 4], Res, Vec<f64>, Vec<std::rc::Rc<Best>>, f64)>, kth: f64, next_bound: f64, scanned: usize, generated: usize, sched_rejects: usize,
                  deck_best: crate::FxMap<(Vec<u32>, Vec<u8>, Vec<u32>, usize), (f64, usize, usize)>, lists: Vec<SList> }
    // One best-first pass over the foursomes with cap lv: ceilings are exact tables or product ceilings times lv, the
    // stop rule compares them with the k-th exact value. None when the budget runs out (the pass certifies nothing).
    let run_pass = |lv: f64| -> Option<Pass> {
        let mut hi_sum = f64::INFINITY;
        let lo_sum = smax * 0.85;
        let mut cand: Vec<(f64, [usize; 4])> = vec![];
        band(lo_sum, hi_sum, &mut cand);
        hi_sum = lo_sum;
        cand.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        let mut solved: Vec<([usize; 4], Res, Vec<f64>, Vec<std::rc::Rc<Best>>, f64)> = vec![];
        let mut vals: Vec<f64> = vec![];
        let mut next = 0; let mut scanned = 0; let mut generated = cand.len();
        let mut kth_val = f64::NEG_INFINITY;
        let mut sched_rejects = 0usize;
        // shared deck: each deck's best value and where (solved index, list index); the per-loop deck lists
        let mut deck_best: crate::FxMap<(Vec<u32>, Vec<u8>, Vec<u32>, usize), (f64, usize, usize)> = Default::default();
        let mut lists: Vec<SList> = vec![];
        loop {
            if aborted.get() { return None; }
            // Ceilings carry a (1 + 1e-12) guard against rounding, so a loop whose ceiling beats the k-th value by no
            // more than that only ties it and cannot change the result. Without this, a job whose targets are all
            // guaranteed (every loop has the same value) solved every foursome.
            let beats = |c: f64| c > kth_val * (1.0 + 4e-12);
            if !(next < cand.len() && beats(lv * cand[next].0)) {
                if hi_sum <= 0.0 || !beats(lv * sep(hi_sum)) { break; }
                let kth_lo = if kth_val > f64::NEG_INFINITY { kth_val / lv * 12.0 / (3.0 * fixed) * (1.0 - 1e-9) } else { 0.0 };
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
            let fm = members(&f); let nm = fm.len(); let ln = lins(nm); let nl = ln.len();
            let qpb = model.lineage_q_arr(fm, &|x, y| pb[fm[x]][fm[y]]);
            // cheapest first: the cell bounds as they stand, then the product ceiling with one deck per trainee
            if !beats((0..nl).map(|l| cell(f[ln[l].0], gidx(qpb[l]), lv).0).sum::<f64>() / nl as f64) { continue; }
            if !beats(lv * relaxed(f, &qpb)) { continue; }
            if shared && !beats(lv * relaxed_shared(f, &qpb)) { continue; }
            // exact ceiling at the pairwise-ceiling chances, valid for any schedule
            if !beats(exact_ceiling(f, &qpb, kth_val, lv)) { continue; }
            if aborted.get() { return None; }
            let r = sched_c.borrow().get(&f).cloned();
            let r = r.unwrap_or_else(|| {
                let r = match &fixed_s {
                    Some(fs) => Res { val: 0.0, s: std::array::from_fn(|i| if i < nm { fs[f[i]].unwrap() } else { [Choice::Rest; NT] }), order: f, evals: 0, secs: 0.0 },
                    None => optimize_fam(d, &f, "scf@0.5", sj.iters, sj.seed, 60.0),
                };
                sched_c.borrow_mut().insert(f, r.clone()); r });
            let ps: Vec<Vec<f64>> = (0..nm).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
            let qs = model.lineage_q(fm, &|x, y| if no_race { 0.0 } else { wval(&ps[x], &ps[y]) });
            let qa: [f64; 12] = std::array::from_fn(|i| qs.get(i).copied().unwrap_or(0.0));
            // rejected only with the schedule found: a better schedule could change this one's fate
            if (shared && !beats(lv * relaxed_shared(f, &qa))) || !beats(exact_ceiling(f, &qa, kth_val, lv)) { sched_rejects += 1; continue; }
            if shared {
                let sl = exact_shared(f, &qa[..nl], if kth_val.is_finite() { kth_val * nl as f64 } else { f64::NEG_INFINITY }, lv);
                if aborted.get() { return None; }
                if sl.is_empty() { continue; }
                let val = sl[0].0 / nl as f64;
                for (v, x) in sl.iter() {
                    let id = (x.ids.clone(), x.ends.clone(), x.vars.clone(), x.plain_ids.len());
                    let e = deck_best.entry(id).or_insert((f64::NEG_INFINITY, 0, 0));
                    if v / nl as f64 > e.0 { *e = (v / nl as f64, solved.len(), lists.len()); }
                }
                solved.push((f, r, qs, vec![], val));
                lists.push(sl);
                let mut vs: Vec<f64> = deck_best.values().map(|x| x.0).collect(); vs.sort_by(|a, b| b.partial_cmp(a).unwrap());
                kth_val = if vs.len() >= kloops { vs[kloops - 1] } else { f64::NEG_INFINITY };
                continue;
            }
            let mut tot = 0.0; let mut bests = vec![];
            for t in 0..nm {
                let qt: Vec<f64> = (0..nl).filter(|&l| ln[l].0 == t).map(|l| qa[l]).collect();
                let b = exact_best(f[t], &qt, lv); tot += b.sum; bests.push(b);
            }
            if aborted.get() { return None; }
            let val = tot / nl as f64;
            solved.push((f, r, qs, bests, val));
            vals.push(val);
            let mut vs = vals.clone(); vs.sort_by(|a, b| b.partial_cmp(a).unwrap());
            kth_val = if vs.len() >= kloops { vs[kloops - 1] } else { f64::NEG_INFINITY };
        }
        let next_bound = lv * (if next < cand.len() { cand[next].0 } else { 0.0 }).max(if hi_sum > 0.0 { sep(hi_sum) } else { 0.0 });
        Some(Pass { solved, kth: kth_val, next_bound, scanned, generated, sched_rejects, deck_best, lists })
    };
    // Try the caps highest first (EXACT_STEPS): the answer is certified under the first whose pass finishes within the
    // budget; a lower cap's pass reuses everything computed so far and gets a quarter of the budget more. When none
    // finishes, a pass with the cap at 1 (product odds: no deck searches) gives the answer.
    let steps: Vec<f64> = if !dj.exact { vec![] } else if let Ok(x) = std::env::var("EXACT_STEPS") { x.split(',').filter_map(|t| t.parse().ok()).collect() } else { EXACT_STEPS.to_vec() };
    let mut last: Option<Pass> = None; let mut cert = 1.0f64;
    let mut allow = budget;
    for &lv in &steps {
        budget_c.set(allow);
        match run_pass(lv) { Some(p) => { last = Some(p); cert = lv; break } None => { aborted.set(false); allow = work.get() + budget / 4; } }
    }
    let p = match last { Some(p) => p, None => { budget_c.set(usize::MAX); run_pass(1.0).expect("a pass without deck searches") } };
    let Pass { solved, kth: kth_val, next_bound, scanned, generated, sched_rejects, deck_best, lists } = p;
    if solved.is_empty() { return json!({"error": "No loop contains all the required umas."}); }
    if std::env::var("UPPER_CHECK").is_ok() { let v = UCHK.with(|c| c.get()); eprintln!("UPPER_CHECK: {} checked, max bound/exact {:.4}, {} decks not covered", v.0, v.1, v.2); }
    if std::env::var("DECK_DEBUG").is_ok() { let v = SPT.with(|c| c.get()); eprintln!("deck searches: setup {:.2}s tree {:.2}s configs {} count vectors {}", v.0, v.1, v.2, v.3); }
    if std::env::var("DECK_DEBUG").is_ok() { eprintln!("per-uma search {:.2}s cap {} scanned {} solved {} exact: {} deck searches, {} decks scored, {} bounded out, schedule rejects {} | band {:.2}s score {:.2}s seed-scan {:.2}s bound {:.2}s", tq.elapsed().as_secs_f64(), cert, scanned, solved.len(), n_band.get(), n_scored.get(), n_ufilt.get(), sched_rejects, t_band.get(), t_score.get(), t_seed.get(), t_ub.get()); }
    let mut order: Vec<usize> = (0..solved.len()).collect();
    // stable: ties keep solve order
    order.sort_by(|&a, &b| solved[b].4.partial_cmp(&solved[a].4).unwrap());
    let empty_deck = PDeck { names: vec![], ids: vec![], ends: vec![], vars: vec![], choices: vec![], plain_ids: vec![], pal_names: vec![], pals: 0, borrowed: None, dw: zero.clone(), dg: zero.clone() };
    // what is reported: per-uma, the best loops; shared, the best distinct decks, each with its best loop
    let picks_out: Vec<(usize, Vec<&PDeck>, Vec<(f64, f64, usize)>, f64)> = if shared {
        let mut db: Vec<(&(Vec<u32>, Vec<u8>, Vec<u32>, usize), &(f64, usize, usize))> = deck_best.iter().collect();
        db.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap().then(a.0.cmp(b.0)));
        db.iter().take(kloops).map(|&(id, &(v, si, li))| {
            let x = &lists[li].iter().find(|y| (y.1.ids.clone(), y.1.ends.clone(), y.1.vars.clone(), y.1.plain_ids.len()) == *id).unwrap().1;
            let (f, _, qs, _, _) = &solved[si];
            let (e, fl) = ents_of(x);
            let ln = lin_of(f); let nm = members(f).len();
            let mut per_l = vec![(0.0, 0.0, 0usize); ln.len()];
            for t in 0..nm {
                let ls: Vec<usize> = (0..ln.len()).filter(|&l| ln[l].0 == t).collect();
                let qt: Vec<f64> = ls.iter().map(|&l| qs[l]).collect();
                let j = joint(model, f[t], &qt, &e, fl, ntar, true);
                for (i, &l) in ls.iter().enumerate() { per_l[l] = j[i]; }
            }
            (si, vec![x; nm], per_l, v)
        }).collect()
    } else {
        order.iter().take(kloops).map(|&si| {
            let bests = &solved[si].3;
            let ln = lin_of(&solved[si].0); let nm = members(&solved[si].0).len();
            let mut per_l = vec![(0.0, 0.0, 0usize); ln.len()];
            for t in 0..nm { let ls: Vec<usize> = (0..ln.len()).filter(|&l| ln[l].0 == t).collect(); for (i, &l) in ls.iter().enumerate() { per_l[l] = bests[t].per[i]; } }
            (si, bests.iter().map(|b| b.deck.as_ref().unwrap_or(&empty_deck)).collect(), per_l, solved[si].4)
        }).collect()
    };
    let top: Vec<Value> = picks_out.iter().map(|(si, dk, per_l, pj)| {
        let (f, r, qs, _, _) = &solved[*si];
        let ln = lin_of(f); let f = members(f); let nm = f.len(); let nl = ln.len();
        let mut per = vec![(0.0, 0usize); nm]; let mut picks: Vec<Vec<usize>> = vec![vec![]; nm];
        for (l, &(t, ..)) in ln.iter().enumerate() { per[t].0 += per_l[l].0; per[t].1 += 1; picks[t].push(per_l[l].2); }
        let minus1 = per_l.iter().map(|x| x.1).sum::<f64>() / nl as f64;
        // the chance of exactly k targets, k = 0 ..= n (exact over each lineage's landed sets, as `p_all`), averaged over
        // the lineages: the uncertain targets' sets from the joint model, each guaranteed gold on its own (GEN_GOLD)
        let mut landed = vec![0.0f64; ntar + 1];
        for l in 0..nl {
            let t = ln[l].0; let (e, fl) = ents_of(dk[t]); let nu = unc.len();
            let mut ext = JointExt { su: vec![1.0; nu], sf: vec![1.0; ntar - nu], marg: vec![], sets: vec![0.0], per_outfit: vec![] };
            joint_fast(model, f[t], &[qs[l]], &e, fl, ntar, false, Some(&mut ext));
            let o = per_l[l].2;
            let mut g = vec![1.0f64];
            for _ in nu..ntar { let mut nn = vec![0.0; g.len() + 1]; for (i, x) in g.iter().enumerate() { nn[i] += x * (1.0 - GEN_GOLD); nn[i + 1] += x * GEN_GOLD; } g = nn; }
            for (mu, &x) in ext.sets[(o << nu)..((o + 1) << nu)].iter().enumerate() {
                if x == 0.0 { continue; }
                for (j, &y) in g.iter().enumerate() { landed[mu.count_ones() as usize + j] += x * y / nl as f64; }
            }
        }
        // the search's product value of this loop with these decks
        let rank_v = fixed * (0..nl).map(|l| model.trainee_fast(f[ln[l].0], qs[l], &dk[ln[l].0].dw, &dk[ln[l].0].dg)).sum::<f64>() / nl as f64;
        let outfits: Vec<Value> = (0..nm).map(|i| {
            let kk = *picks[i].iter().max_by_key(|x| picks[i].iter().filter(|y| y == x).count()).unwrap();
            let (st, cs) = &model.opts[f[i]][kk];
            let covers: Vec<Value> = st.iter().enumerate().filter(|(_, lv)| **lv > 0).map(|(j, lv)| json!({"group": unc[j], "level": lv})).collect();
            json!({"outfits": if covers.is_empty() { vec![] } else { cs.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>() }, "covers": covers})
        }).collect();
        let pss: Vec<Vec<f64>> = (0..nm).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let dks: Vec<(&[f64], &[f64])> = (0..nm).map(|t| (&dk[t].dw[..], &dk[t].dg[..])).collect();
        let mut detail = model.detail(f, &|a, b| if no_race { 0.0 } else { wval(&pss[a], &pss[b]) }, Some(&dks));
        if let Some(ls) = detail["lineages"].as_array_mut() { for (l, x) in ls.iter_mut().enumerate() { x["rank_value"] = x["p_all"].clone(); x["p_all"] = json!(per_l[l].0); } }
        let lp = json!({"members": member_json(d, f), "p_all": pj, "p_minus1": minus1, "landed": landed, "rank_value": rank_v, "outfits": outfits,
               "per_trainee": per.iter().map(|(s, c)| s / *c as f64).collect::<Vec<_>>(),
               "mean_proc": qs.iter().sum::<f64>() / qs.len() as f64, "schedule": chart_rows(d, f, &r.s[..nm]), "detail": detail});
        if shared { return json!({"loop": lp, "deck": dk[0].out(&unc)}); }
        // the deck most members share, for displays that show one deck
        let base = (0..nm).max_by_key(|&i| (dk.iter().filter(|x| x.names == dk[i].names).count(), nm - i)).unwrap();
        json!({"loop": lp, "deck": dk[base].out(&unc), "decks": dk.iter().map(|x| x.out(&unc)).collect::<Vec<_>>()})
    }).collect();
    let budget_out = dj.exact && cert < EXACT_RHO;
    let complete = next_bound <= kth_val * (1.0 + 4e-12);
    // what the answer rests on: `rho` the assumed cap on exact / product odds, `max_ratio` the largest seen on any deck
    // scored here; `complete` = the search ended by its stop rule (no unexamined loop or deck can win under the cap)
    let exact = json!({"rho": cert, "rho_target": EXACT_RHO, "max_ratio": max_ratio.get(), "complete": complete, "decks_scored": n_scored.get(),
                       "deck_searches": n_band.get(), "loops_solved": solved.len(), "schedule_rejects": sched_rejects, "cap_hit": cap_hit.get(), "budget_out": budget_out, "bound_filtered": n_ufilt.get()});
    let mut out = stats;
    for (key, val) in [("top", json!(top)), ("best", json!([])), ("per_uma", json!(!shared)), ("foursomes", json!(total_foursomes)),
                       ("foursomes_ranked", json!(generated)), ("foursomes_scanned", json!(scanned)), ("foursomes_solved", json!(solved.len())),
                       ("next_bound", json!(next_bound)), ("proven", json!(complete)), ("searches", json!(1 + n_band.get())), ("exact", exact)] {
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
        let fs: [&[usize]; 5] = [&[0, 5, 9, 20], &[3, 7, 11, 40], &[1, 2, 3, 4], &[0, 5, 9], &[3, 11, 40]];
        for (fi, &f) in fs.iter().enumerate() {
            let qa = m.lineage_q_arr(f, &|_, _| 3.0);
            let qv = m.lineage_q(f, &|_, _| 3.0);
            assert_eq!(qv.len(), if f.len() == 4 { 12 } else { 3 });
            assert!(qa.iter().zip(&qv).all(|(a, b)| (a - b).abs() < 1e-15));
            let dw: Vec<f64> = (0..nu).map(|k| ((k + fi) as f64 * 0.17) % 1.0).collect();
            let dg: Vec<f64> = (0..nu).map(|k| ((k * 3 + fi) as f64 * 0.11) % 1.0).collect();
            let a = m.metric_fast(f, &qa, &dw, &dg); let b = m.metric_d(f, &qv, &dw, &dg);
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

    /// The game's scenario bans reach the solver with the card data: Team Sirius out of Unity Cup and Grand Concert
    /// decks, allowed in URA Finale and Trackblazer.
    #[test]
    fn scenario_bans_exported() {
        let cr: CardsRaw = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web_cards.json")).unwrap()).unwrap();
        for sc in ["unity", "gc"] { assert!(cr.restrict.get(sc).map_or(false, |v| v.contains(&30081)), "{sc}: {:?}", cr.restrict); }
        for sc in ["ura", "trackblazer"] { assert!(!cr.restrict.get(sc).map_or(false, |v| v.contains(&30081))); }
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
    /// Measured outcome shares (`alt_w`) replace equal outcomes: 3:1 between two skills, link firing 80%.
    #[test]
    fn outcome_shares_weight_alts() {
        let c = halo();
        let dj = job_with(json!({"alts": [[30056, 3, [[[[20102, 1]], [[20159, 1]]]]]], "alt_w": [[30056, 3, [[0.75, 0.25]]]]}));
        let unc = [20102u32, 20159];
        let sb = Sources { dj: &dj, unc: &unc };
        let (mut mw, mut mg) = (vec![1.0; 2], vec![1.0; 2]);
        src_miss(&sb.src(&c, None, 0), &[1.0, 1.0, 0.8], &mut mw, &mut mg);
        assert!((mw[0] - (1.0 - 0.8 * 0.75)).abs() < 1e-12 && (mw[1] - (1.0 - 0.8 * 0.25)).abs() < 1e-12, "{mw:?}");
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
    /// The collapsed-state form equals the full enumeration (chains, coupled outcomes, a stop risk, fillers).
    #[test]
    fn joint_fast_matches_reference() {
        let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
        let sk: SkillsRaw = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web_skills.json")).unwrap()).unwrap();
        let sj: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
        let m = Model::new(&d, &sk, &sj);
        let g1 = |k: usize, lvl: u8, p: f64| -> Group { vec![(p, vec![(k, lvl)])] };
        let two = |a: usize, b: usize| -> Group { vec![(0.5, vec![(a, 2)]), (0.3, vec![(b, 1)])] };
        // chain cards with single-target and coupled outcomes, a coupled fixed event, a stop risk, fillers
        let a = Src { train: vec![(0, 0.3)], chains: vec![(1, g1(1, 1, 1.0)), (2, two(1, 2)), (3, g1(0, 2, 0.5))], ..Default::default() }.finish();
        let b = Src { fixed: vec![two(3, 0)], chains: vec![(2, g1(3, 2, 1.0))], ..Default::default() }.finish();
        let c = Src { train: vec![(2, 0.2)], chains: vec![(1, two(2, 3))], ..Default::default() }.finish();
        let e = Src { fixed: vec![g1(1, 2, 0.25)], ..Default::default() }.finish();
        let risk = stop_risk_code(&[(2, 1, 2)], 3);
        let ents = vec![(a, (3, 3, 0)), (b, (2, 2, 0)), (c, (3, 3, risk)), (e, (0, 0, 0))];
        for fill in [0usize, 2] {
            for u in [0usize, 5, 20] {
                let x = joint_fast(&m, u, &[0.1, 0.5, 0.9], &ents, fill, sj.targets.len(), true, None);
                let y = joint_trainee_ref(&m, u, &[0.1, 0.5, 0.9], &ents, fill, sj.targets.len());
                for (p, r) in x.iter().zip(&y) {
                    assert!((p.0 - r.0).abs() < 1e-13 && (p.1 - r.1).abs() < 1e-13 && p.2 == r.2, "fill {fill} uma {u}: {:?} vs {:?}", p, r);
                }
                let z = joint_fast(&m, u, &[0.1, 0.5, 0.9], &ents, fill, sj.targets.len(), false, None);
                for (p, r) in z.iter().zip(&y) { assert!((p.0 - r.0).abs() < 1e-13); }
            }
        }
    }

    /// Career planner: a per-target inheritance chance folded into the trainee's event hint chances, evaluated at
    /// q = 0, gives the same joint odds as the loop model at q; the spark scale multiplies P(all) by the product of
    /// the scales; with two targets the exact marginals satisfy P(A) + P(B) = P(all) + P(at least one).
    #[test]
    fn career_fold_and_scale() {
        let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
        let sk: SkillsRaw = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web_skills.json")).unwrap()).unwrap();
        let sj: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}],
            "trainee_hints": [[1001, 20166, 0.3]]})).unwrap();
        let m = Model::new(&d, &sk, &sj);
        let g1 = |k: usize, lvl: u8, p: f64| -> Group { vec![(p, vec![(k, lvl)])] };
        let two = |a: usize, b: usize| -> Group { vec![(0.5, vec![(a, 2)]), (0.3, vec![(b, 1)])] };
        let a = Src { train: vec![(0, 0.3)], chains: vec![(1, g1(1, 1, 1.0)), (2, two(1, 2)), (3, g1(0, 2, 0.5))], ..Default::default() }.finish();
        let b = Src { fixed: vec![two(3, 0)], chains: vec![(2, g1(3, 2, 1.0))], ..Default::default() }.finish();
        let c = Src { train: vec![(2, 0.2)], chains: vec![(1, two(2, 3))], ..Default::default() }.finish();
        let ents = vec![(a, (3, 3, 0)), (b, (2, 2, 0)), (c, (3, 3, 0))];
        let rel = |x: f64, y: f64| (x - y).abs() / y.abs().max(1e-300);
        for u in [0usize, 5, 20] {
            for q in [0.1, 0.6] {
                let x = joint_fast(&m, u, &[q], &ents, 1, sj.targets.len(), true, None)[0];
                let mut f = Model::new(&d, &sk, &sj);
                f.ews[u] = m.ews[u].iter().map(|o| o.iter().map(|e| 1.0 - (1.0 - e) * (1.0 - q)).collect()).collect();
                let y = joint_fast(&f, u, &[0.0], &ents, 1, sj.targets.len(), true, None)[0];
                assert!(rel(y.0, x.0) < 1e-12 && rel(y.1, x.1) < 1e-12 && x.2 == y.2, "fold uma {u} q {q}: {:?} vs {:?}", x, y);
                let su = vec![1.1f64.powi(-1), 1.0, 1.1f64.powi(-3), 1.1f64.powi(-6)];
                let mut ext = JointExt { su: su.clone(), sf: vec![1.1f64.powi(-2)], marg: vec![], sets: vec![], per_outfit: vec![] };
                let z = joint_fast(&f, u, &[0.0], &ents, 1, sj.targets.len(), true, Some(&mut ext))[0];
                let c = su.iter().product::<f64>() * 1.1f64.powi(-2);
                assert!(rel(z.0, c * y.0) < 1e-12 && z.2 == y.2, "scale uma {u}: {} vs {}", z.0, c * y.0);
                assert!(z.1 >= z.0 && z.1 <= 1.0);
            }
        }
        // two targets, coupled sources: marginals against P(all) and P(all but at most one)
        let sj2: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20166}]})).unwrap();
        let m2 = Model::new(&d, &sk, &sj2);
        let p = Src { train: vec![(0, 0.2)], chains: vec![(1, two(0, 1)), (2, g1(1, 1, 0.7))], ..Default::default() }.finish();
        let r = Src { fixed: vec![two(1, 0)], chains: vec![(1, g1(0, 2, 0.4))], ..Default::default() }.finish();
        let ents2 = vec![(p, (2, 2, 0)), (r, (2, 2, 0))];
        for u in [0usize, 5, 20] {
            let mut ext = JointExt { su: vec![0.9, 0.8], sf: vec![], marg: vec![], sets: vec![0.0], per_outfit: vec![] };
            let j = joint_fast(&m2, u, &[0.3], &ents2, 0, 2, true, Some(&mut ext))[0];
            let (m0, m1) = (ext.marg[j.2 * 2], ext.marg[j.2 * 2 + 1]);
            assert!(rel(m0 + m1, j.0 + j.1) < 1e-12, "uma {u}: {m0} + {m1} vs {} + {}", j.0, j.1);
            // the landed sets: they sum to 1, {both} is P(all), and each target's sets sum to its marginal
            let st = &ext.sets[(j.2 << 2)..(j.2 << 2) + 4];
            assert!((st.iter().sum::<f64>() - 1.0).abs() < 1e-12 && rel(st[3], j.0) < 1e-12 && rel(st[1] + st[3], m0) < 1e-12 && rel(st[2] + st[3], m1) < 1e-12, "sets {:?}", st);
        }
    }

    /// The negative-association bound is at least the exact value (chains, a card feeding two targets, a target fed by
    /// two cards, early ends, a stop risk, fillers), and close to it.
    #[test]
    fn upper_bound_covers_exact() {
        let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
        let sk: SkillsRaw = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web_skills.json")).unwrap()).unwrap();
        let sj: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
        let m = Model::new(&d, &sk, &sj);
        let g1 = |k: usize, lvl: u8, p: f64| -> Group { vec![(p, vec![(k, lvl)])] };
        let two = |a: usize, b: usize| -> Group { vec![(0.5, vec![(a, 2)]), (0.3, vec![(b, 1)])] };
        let a = Src { train: vec![(0, 0.3)], chains: vec![(1, g1(1, 1, 1.0)), (2, two(1, 2)), (3, g1(0, 2, 0.5))], ..Default::default() }.finish();
        let b = Src { fixed: vec![two(3, 0)], chains: vec![(2, g1(3, 2, 1.0))], ..Default::default() }.finish();
        let c = Src { train: vec![(2, 0.2)], chains: vec![(3, g1(2, 2, 0.6))], ..Default::default() }.finish();
        let e = Src { fixed: vec![g1(1, 2, 0.25)], ..Default::default() }.finish();
        let risk = stop_risk_code(&[(2, 1, 2)], 3);
        let decks: Vec<Vec<(Src, Kind)>> = vec![
            vec![(a.clone(), (3, 3, 0)), (b.clone(), (2, 2, 0)), (e.clone(), (0, 0, 0))],
            vec![(a.clone(), (3, 3, 0)), (c.clone(), (3, 3, 0)), (b.clone(), (2, 2, 0))],
            vec![(a.clone(), (3, 2, 0)), (c.clone(), (3, 3, risk)), (b.clone(), (2, 2, 0))],
            vec![(a.clone(), (3, 3, 0)), (c.clone(), (3, 1, 0)), (e.clone(), (0, 0, 0))],
        ];
        let mut covered = 0;
        for ents in &decks {
            for fill in [0usize, 1, 3] {
                for u in [0usize, 5, 20] {
                    let qs = [0.1, 0.5, 0.9];
                    let x = joint_fast(&m, u, &qs, ents, fill, sj.targets.len(), false, None);
                    let er: Vec<(&Src, Kind)> = ents.iter().map(|(s, k)| (s, *k)).collect();
                    if let Some(ub) = upper_na(&m, u, &qs, &er, fill) {
                        covered += 1;
                        for (i, y) in x.iter().enumerate() { assert!(ub[i] >= y.0 * (1.0 - 1e-12) && ub[i] <= y.0 * 1.5, "fill {fill} uma {u} q {}: bound {} exact {}", qs[i], ub[i], y.0); }
                    }
                }
            }
        }
        assert!(covered >= 30, "bound covered only {covered} cases");
    }

    /// Groups whose outcomes each hint one target pass the orthant check; an outcome hinting two targets together fails.
    #[test]
    fn group_orthant_check() {
        assert!(group_nuod(&vec![(0.5, vec![(0, 2)]), (0.3, vec![(1, 1)])]));
        assert!(!group_nuod(&vec![(0.5, vec![(0, 1), (1, 1)]), (0.3, vec![(1, 1)])]));
        assert!(group_nuod(&vec![(0.2, vec![(0, 1), (1, 1)]), (0.3, vec![(0, 1)]), (0.3, vec![(1, 1)])]));   // 0.2 <= 0.5 * 0.5
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
                let j = joint(&m, u, &[q], &[(a.clone(), (0, 0, 0)), (b.clone(), (0, 0, 0))], 0, sj.targets.len(), true);
                let p = m.fixed_factor() * m.trainee_fast(u, q, &dw, &dg);
                assert!((j[0].0 - p).abs() < 1e-14, "uma {u} q {q}: {} vs {p}", j[0].0);
            }
        }
    }
}
