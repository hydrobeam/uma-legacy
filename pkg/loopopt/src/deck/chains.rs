//! Chain events: the event count per run, the per-link marginals and the final-state distribution of a deck's chains,
//! and the per-key level tables.
use std::collections::HashMap;
/// Chain events per run ~ Binomial(CHAIN_N, p), p set by the deck's pals: logit p = CHAIN_A + CHAIN_B x pals, pals its
/// pal cards (group cards excluded) and plain pals (`ChainModel::PalRate`). Fitted on uma-it Independent Training runs
/// with readable chain markers, under the card rule and the stop share 1/3 (docs/RESEARCH.md section 8: the sample and
/// the evidence).
/// The current evidence does not make this count law exact for every deck or scenario.
pub(crate) const CHAIN_N: u32 = 21;
pub(crate) const CHAIN_A: f64 = -1.09469;
pub(crate) const CHAIN_B: f64 = 0.168_072;
/// The share of its firings on which a link where IT's default choice ends the chain on some outcomes (card_events.json
/// `stop_risk`, the job's `event_stop_risk`) ends it, as k of n: one share for every such link, in place of its own k
/// of n outcomes (docs/RESEARCH.md section 8's STOP_SHARE). Packed like a stop risk (`stop_risk_code`), so the chain odds take it as the exact fraction 1/3.
pub(crate) const CHAIN_STOP_SHARE: (u8, u8) = (1, 3);
/// How a chain event picks the card it advances among the cards whose chains still run (`pick_weight`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum PickRule {
    /// Each running card equally, however many links it has left: the model (docs/RESEARCH.md section 8).
    Card,
    /// Each running card in proportion to its links left: the model before the card rule (job `chain_rule: "slots"`).
    Slots,
}
/// A job's chain-event law: one distribution for every deck, or one per pal count. Every chance the solver computes
/// rises with p (given more events the chains only advance further, whatever the pick rule: a run with n + 1 events
/// is one with n and one more pick; and a link that fires loses no reward), so a bound
/// over decks whose pal counts are not yet known takes the largest p among them (`most_favourable`).
#[derive(Clone, Copy)]
pub(crate) enum ChainModel {
    /// The same distribution for every deck: the job's `chain_events`, or a pal rate without slope.
    Fixed(ChainEvents),
    /// Binomial(CHAIN_N, expit(intercept + slope x pals)), each event picking a card by `rule`.
    PalRate { intercept: f64, slope: f64, rule: PickRule },
}
impl ChainModel {
    /// A rate without slope is one distribution (`Fixed`), so its searches run as a fixed job's do.
    pub(crate) fn pal_rate(intercept: f64, slope: f64, rule: PickRule) -> Self {
        if slope == 0.0 {
            Self::Fixed(ChainEvents { count: CHAIN_N, probability: expit(intercept), rule })
        } else {
            Self::PalRate { intercept, slope, rule }
        }
    }
    /// Whether decks with different pal counts get different chain events.
    pub(crate) const fn pal_dependent(self) -> bool {
        matches!(self, Self::PalRate { .. })
    }
    /// The chain events of a deck with `pals` rate-raising pals (`Card::raises_chain_rate`, plain pals included).
    pub(crate) fn events(self, pals: usize) -> ChainEvents {
        match self {
            Self::Fixed(events) => events,
            Self::PalRate { intercept, slope, rule } => {
                ChainEvents { count: CHAIN_N, probability: expit(intercept + slope * pals as f64), rule }
            }
        }
    }
    /// The largest p over pal counts 0..=`most_pals` (p is monotone in the count): every chance at these events is at
    /// least the same chance at any of those counts' events.
    pub(crate) fn most_favourable(self, most_pals: usize) -> ChainEvents {
        let (none, most) = (self.events(0), self.events(most_pals));
        if most.probability >= none.probability { most } else { none }
    }
    /// The smallest p over pal counts 0..=`most_pals`: every chance at these events is at most the same chance at any of
    /// those counts' events.
    pub(crate) fn least_favourable(self, most_pals: usize) -> ChainEvents {
        let (none, most) = (self.events(0), self.events(most_pals));
        if most.probability <= none.probability { most } else { none }
    }
}
/// The logistic function 1 / (1 + e^-x).
pub(crate) fn expit(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
/// The weight a running entry (`done` of its `links` links fired, short of its stop) is picked with for the next chain
/// event: 1 under the card rule, its links left under the slots rule. The one pick rule of `chain_walk` and
/// `chain_marginals_ref`.
#[inline(always)]
fn pick_weight(links: u8, done: u8, rule: PickRule) -> f64 {
    match rule {
        PickRule::Card => 1.0,
        PickRule::Slots => f64::from(links - done),
    }
}
/// A deck entry's chain kind. A card played normally stops at its last link; a chosen early end stops sooner. While it
/// runs, an entry is picked with weight `pick_weight` (1, or its links left under the slots rule); once it reaches its
/// stop it is out.
/// Ordered and hashed as the tuple (links, stop, stop risk).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct ChainKind {
    /// The card's chain links.
    pub(super) links: u8,
    /// The link it stops after.
    pub(crate) stop: u8,
    /// Links where the chain ends on k of n of the link's firings, one byte per link 1..=4 ((k << 4) | n; 0 = none),
    /// only links before the stop (`stop_risk_code`). k / n is the effective fraction, not the event's outcome count: a
    /// card-event stop (`DeckJob::event_stop_risk`) at `DeckJob::chain_stop_share` (its own k of n only under the slots
    /// rule), a job's pinned stop (`DeckJob::stop_risk`) at its own k of n.
    pub(super) stop_risk: u32,
}

impl ChainKind {
    /// A pal's or a chainless card's entry: no chain.
    pub(crate) const NONE: Self = Self { links: 0, stop: 0, stop_risk: 0 };
    /// The fillers' chain: two links, played to the end.
    pub(crate) const FILLER: Self = Self { links: 2, stop: 2, stop_risk: 0 };
}
pub(super) fn stop_risk_code(risk: &[(u8, u8, u8)], stop: u8) -> u32 {
    risk.iter().filter(|risk_row| risk_row.0 >= 1 && risk_row.0 < stop && risk_row.0 <= 4 && risk_row.2 > 0).fold(
        0u32,
        |code, risk_row| {
            code | (((u32::from(risk_row.1) << 4) | u32::from(risk_row.2)) << (8 * (u32::from(risk_row.0) - 1)))
        },
    )
}

/// A deck's chain process: Binomial(count, probability) events per run, each picking a card by `pick_weight` under
/// `rule` (`ChainModel::events`). Passed by value to every chain computation; the caches below key
/// on it, so decks with different processes never share an entry.
#[derive(Clone, Copy)]
pub(crate) struct ChainEvents {
    pub(crate) count: u32,
    pub(crate) probability: f64,
    pub(crate) rule: PickRule,
}
/// Cache key of a chain process: n, the bits of p and the pick rule, kept separately to avoid XOR collisions.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct EventsKey {
    count: u32,
    probability_bits: u64,
    rule: PickRule,
}
/// Cache key of the chain caches (`chain_marginals`, `chain_states`, `key_level`): a deck's chain kinds and the
/// event distribution.
#[derive(Clone, PartialEq, Eq, Hash)]
struct ChainKey {
    kinds: Vec<ChainKind>,
    events: EventsKey,
}
impl ChainEvents {
    pub(crate) const fn key(self) -> EventsKey {
        EventsKey { count: self.count, probability_bits: self.probability.to_bits(), rule: self.rule }
    }
}
/// P(N = e) chain events per run for e = 0..=n, N ~ Binomial(`events`).
pub(super) fn event_pmf(events: ChainEvents, n: usize) -> Vec<f64> {
    let ChainEvents { count: trials, probability: chance, .. } = events;
    (0..=n)
        .map(|e| {
            if e as u32 <= trials {
                binom_coef(trials, e as u32) * chance.powi(e as i32) * (1.0 - chance).powi((trials - e as u32) as i32)
            } else {
                0.0
            }
        })
        .collect()
}
/// `event_pmf` over the whole count (n = `events.count`), the last (n, p) kept: a deck's calls repeat it
pub(super) fn event_pmf_job(events: ChainEvents) -> std::rc::Rc<Vec<f64>> {
    thread_local! { static EP: std::cell::RefCell<Option<(EventsKey, std::rc::Rc<Vec<f64>>)>> = const { std::cell::RefCell::new(None) }; }
    let key = events.key();
    if let Some(pmf) = EP.with(|cell| {
        cell.borrow().as_ref().filter(|cached| cached.0 == key).map(|cached| std::rc::Rc::clone(&cached.1))
    }) {
        return pmf;
    }
    let pmf = std::rc::Rc::new(event_pmf(events, key.count as usize));
    EP.with(|cell| *cell.borrow_mut() = Some((key, std::rc::Rc::clone(&pmf))));
    pmf
}
pub(super) fn binom_coef(n: u32, k: u32) -> f64 {
    (0..k).fold(1.0f64, |coefficient, i| coefficient * f64::from(n - i) / f64::from(i + 1))
}
/// A realised chain stop and its probability.
struct ChainStop {
    link: u8,
    probability: f64,
}

impl ChainKind {
    /// Where this chain stops: risky links in order, then the planned stop with the remaining probability.
    /// This is the single decoder of the packed risk bytes.
    fn stop_alternatives(&self) -> Vec<ChainStop> {
        let mut alternatives = vec![];
        let mut surviving_probability = 1.0f64;
        for link in 1..self.stop.min(5) {
            let risk_byte = (self.stop_risk >> (8 * (u32::from(link) - 1))) & 0xff;
            if risk_byte == 0 {
                continue;
            }
            let stop_probability = f64::from(risk_byte >> 4) / f64::from(risk_byte & 15);
            alternatives.push(ChainStop { link, probability: surviving_probability * stop_probability });
            surviving_probability *= 1.0 - stop_probability;
        }
        alternatives.push(ChainStop { link: self.stop, probability: surviving_probability });
        alternatives
    }
}
/// The stop-risk mixture: every realisation of where the entries stop (one `ChainKind::stop_alternatives` choice
/// each, the first entry varying fastest), with probability w (the product in entry order), `visit(w, stops)` where
/// w > 0.
fn for_each_realisation(kinds: &[ChainKind], mut visit: impl FnMut(f64, &[u8])) {
    let alternatives: Vec<Vec<ChainStop>> = kinds.iter().map(ChainKind::stop_alternatives).collect();
    let n = kinds.len();
    let mut choices = vec![0usize; n];
    let mut stops = vec![0u8; n];
    loop {
        let probability: f64 = (0..n).map(|i| alternatives[i][choices[i]].probability).product();
        if probability > 0.0 {
            for i in 0..n {
                stops[i] = alternatives[i][choices[i]].link;
            }
            visit(probability, &stops);
        }
        // Advance the mixed-radix choice vector, first entry fastest. With no entries, visit once.
        let mut i = 0;
        while i < n {
            choices[i] += 1;
            if choices[i] < alternatives[i].len() {
                break;
            }
            choices[i] = 0;
            i += 1;
        }
        if i == n {
            break;
        }
    }
}
/// The chain process for entries of `links` links stopping at `stops` (no risk left). States are in mixed-radix
/// index order: entry i's progress is the digit with stride prod_{j<i} (stops[j] + 1). Every transition raises the
/// index, so this order is topological. `visit(progress, events, grid_index, probability)` receives reachable states
/// up to `max_events`; `grid_strides` maps their progress to the caller's grid. Below `max_events`, an entry takes
/// the next event with its `pick_weight` under `rule` over the sum among entries not stopped.
/// The one transition rule of `chain_marginals_dense`, `chain_states` and `key_level`.
#[inline(always)]
fn chain_walk(
    links: &[u8],
    stops: &[u8],
    max_events: usize,
    grid_strides: &[usize],
    rule: PickRule,
    mut visit: impl FnMut(&[u8], usize, usize, f64),
) {
    let n = links.len();
    let mut strides = vec![0usize; n];
    let mut state_count = 1usize;
    for i in 0..n {
        strides[i] = state_count;
        state_count *= stops[i] as usize + 1;
    }
    let mut probabilities = vec![0.0f64; state_count];
    probabilities[0] = 1.0;
    // Advance progress, its sum and its caller-grid index together, as an odometer.
    let mut progress = vec![0u8; n];
    let (mut events, mut grid_index) = (0usize, 0usize);
    for state_index in 0..state_count {
        let probability = probabilities[state_index];
        if probability != 0.0 && events <= max_events {
            visit(&progress, events, grid_index, probability);
            if events < max_events {
                let mut total_weight = 0.0f64;
                for i in 0..n {
                    if progress[i] < stops[i] {
                        total_weight += pick_weight(links[i], progress[i], rule);
                    }
                }
                if total_weight > 0.0 {
                    for i in 0..n {
                        if progress[i] < stops[i] {
                            probabilities[state_index + strides[i]] +=
                                probability * pick_weight(links[i], progress[i], rule) / total_weight;
                        }
                    }
                }
            }
        }
        for i in 0..n {
            if progress[i] < stops[i] {
                progress[i] += 1;
                events += 1;
                grid_index += grid_strides[i];
                break;
            }
            events -= progress[i] as usize;
            grid_index -= progress[i] as usize * grid_strides[i];
            progress[i] = 0;
        }
    }
}
/// P(link k of an entry fires), k = 1..=stop, for every chain kind present, given the deck's chain kinds.
/// Dense, deterministic, memoised version of `chain_marginals_ref`. States are mixed-radix indices over the
/// entries' progress; every transition raises the index, so one pass in index order is a topological order.
/// Entries are sorted first so the floating-point summation order is canonical (the reference sums its states in
/// another order, so the two agree to float noise).
pub(crate) fn chain_marginals(events: ChainEvents, kinds: &[ChainKind]) -> HashMap<ChainKind, Vec<f64>> {
    thread_local! { static CM: std::cell::RefCell<crate::FxMap<ChainKey, HashMap<ChainKind, Vec<f64>>>> = std::cell::RefCell::new(crate::FxMap::default()); }
    let mut sorted_kinds: Vec<ChainKind> = kinds.to_vec();
    sorted_kinds.sort_unstable();
    let key = ChainKey { kinds: sorted_kinds, events: events.key() };
    if let Some(marginals) = CM.with(|cell| cell.borrow().get(&key).cloned()) {
        return marginals;
    }
    let marginals = if key.kinds.iter().any(|k| k.stop_risk != 0) {
        chain_marginals_risk(events, &key.kinds)
    } else {
        chain_marginals_dense(events, &key.kinds)
    };
    CM.with(|cell| cell.borrow_mut().insert(key, marginals.clone()));
    marginals
}
/// Kinds with a stop risk: the chance that IT's default choice ends a chain at a link is independent of how many
/// chain events the run gets, so the run is a mixture over where each risky entry would stop. Each realisation is a
/// plain kind (links, realised stop): the entry competes for events with its full chain until it stops, then leaves
/// the running cards, exactly as a chosen early end does. Exact.
pub(super) fn chain_marginals_risk(events: ChainEvents, kinds: &[ChainKind]) -> HashMap<ChainKind, Vec<f64>> {
    let n = kinds.len();
    let mut sums: Vec<Vec<f64>> = kinds.iter().map(|k| vec![0.0; k.stop as usize]).collect();
    for_each_realisation(kinds, |w, stops| {
        let real: Vec<ChainKind> =
            (0..n).map(|i| ChainKind { links: kinds[i].links, stop: stops[i], stop_risk: 0 }).collect();
        let marginals = chain_marginals(events, &real);
        for i in 0..n {
            if let Some(link_marginals) = marginals.get(&real[i]) {
                for (k, marginal) in link_marginals.iter().enumerate() {
                    sums[i][k] += w * marginal;
                }
            }
        }
    });
    let mut out: HashMap<ChainKind, Vec<f64>> = HashMap::new();
    for i in 0..n {
        out.entry(kinds[i]).or_insert_with(|| sums[i].clone());
    }
    out
}
pub(super) fn chain_marginals_dense(events: ChainEvents, kinds: &[ChainKind]) -> HashMap<ChainKind, Vec<f64>> {
    let n = kinds.len();
    let mut out: HashMap<ChainKind, Vec<f64>> = HashMap::new();
    if n == 0 {
        return out;
    }
    let max_events = 60usize;
    let event_weights = event_pmf(events, max_events);
    let total: usize = kinds.iter().map(|kind| kind.stop as usize).sum();
    let last = max_events.min(total);
    let width = kinds.iter().map(|k| k.stop as usize).max().unwrap_or(0).max(1);
    // done[e][i][k] = P(after e events entry i has >= k+1 links done)
    let mut done = vec![0.0f64; (last + 1) * n * width];
    let (links, stops): (Vec<u8>, Vec<u8>) = kinds.iter().map(|k| (k.links, k.stop)).unzip();
    chain_walk(&links, &stops, last, &vec![0; n], events.rule, |progress, event_count, _, probability| {
        if event_count >= 1 {
            let base = event_count * n * width;
            for i in 0..n {
                for k in 0..progress[i] as usize {
                    done[base + i * width + k] += probability;
                }
            }
        }
    });
    let mut fired = vec![vec![0.0f64; width]; n];
    for e in 1..=last {
        let event_weight =
            event_weights[e] + if e == last { 1.0 - event_weights[..=e].iter().sum::<f64>() } else { 0.0 };
        let base = e * n * width;
        for i in 0..n {
            for k in 0..width {
                fired[i][k] += event_weight * done[base + i * width + k];
            }
        }
    }
    for i in 0..n {
        let kind = kinds[i];
        out.entry(kind).or_insert_with(|| (0..kind.stop as usize).map(|k| fired[i][k]).collect());
    }
    out
}

/// P(link k of an entry fires), k = 1..=stop, for every chain kind present, given the deck's chain kinds
/// (reference implementation, for the tests).
#[cfg(test)]
pub(super) fn chain_marginals_ref(events: ChainEvents, kinds: &[ChainKind]) -> HashMap<ChainKind, Vec<f64>> {
    let n = kinds.len();
    let mut out: HashMap<ChainKind, Vec<f64>> = HashMap::new();
    if n == 0 {
        return out;
    }
    let max_events = 60usize;
    let event_weights = event_pmf(events, max_events);
    let total: usize = kinds.iter().map(|kind| kind.stop as usize).sum();
    let width = kinds.iter().map(|k| k.stop as usize).max().unwrap_or(0).max(1);
    let mut fired = vec![vec![0.0f64; width]; n]; // P(entry i has >= k+1 links done), accumulated over E
    let mut states: std::collections::BTreeMap<Vec<u8>, f64> = std::collections::BTreeMap::new();
    states.insert(vec![0; n], 1.0);
    for e in 1..=max_events.min(total) {
        let mut next_states: std::collections::BTreeMap<Vec<u8>, f64> = std::collections::BTreeMap::new();
        for (state, p) in &states {
            let weights: Vec<f64> =
                (0..n)
                    .map(|i| {
                        if state[i] < kinds[i].stop { pick_weight(kinds[i].links, state[i], events.rule) } else { 0.0 }
                    })
                    .collect();
            let weight_sum: f64 = weights.iter().sum();
            if weight_sum == 0.0 {
                *next_states.entry(state.clone()).or_insert(0.0) += p;
                continue;
            }
            for i in 0..n {
                if weights[i] > 0.0 {
                    let mut next = state.clone();
                    next[i] += 1;
                    *next_states.entry(next).or_insert(0.0) += p * weights[i] / weight_sum;
                }
            }
        }
        states = next_states;
        let mut prev_done = vec![vec![0.0f64; width]; n];
        for (state, p) in &states {
            for i in 0..n {
                for k in 0..state[i] as usize {
                    prev_done[i][k] += p;
                }
            }
        }
        // P(link done) = sum_e P(E = e) * P(done after e events); accumulate with P(E = e)
        let event_weight = event_weights[e]
            + if e == max_events.min(total) { 1.0 - event_weights[..=e].iter().sum::<f64>() } else { 0.0 };
        for i in 0..n {
            for k in 0..width {
                fired[i][k] += event_weight * prev_done[i][k];
            }
        }
    }
    for i in 0..n {
        let kind = kinds[i];
        out.entry(kind).or_insert_with(|| (0..kind.stop as usize).map(|k| fired[i][k]).collect());
    }
    out
}

/// One final state of the chain process, in sorted chain-kind order.
pub(super) struct ChainState {
    /// Completed links at each chain position.
    pub(super) progress: Vec<u8>,
    pub(super) probability: f64,
}

/// Final-state distribution of the chain process behind `chain_marginals`.
/// Memoised per sorted kind list; `kinds` must be sorted. Exact: a state at level L (L events taken) is final
/// with P(N = L) unless every entry is at its stop, then with P(N >= L); stop-risk entries are the mixture over
/// where they stop, as in `chain_marginals_risk`.
pub(super) fn chain_states(events: ChainEvents, kinds: &[ChainKind]) -> std::rc::Rc<Vec<ChainState>> {
    thread_local! { static CS: std::cell::RefCell<crate::FxMap<ChainKey, std::rc::Rc<Vec<ChainState>>>> = std::cell::RefCell::new(crate::FxMap::default()); }
    let key = ChainKey { kinds: kinds.to_vec(), events: events.key() };
    if let Some(states) = CS.with(|cell| cell.borrow().get(&key).cloned()) {
        return states;
    }
    let n = kinds.len();
    let links: Vec<u8> = kinds.iter().map(|k| k.links).collect();
    let mut state_chances: crate::FxMap<Vec<u8>, f64> = crate::FxMap::default();
    for_each_realisation(kinds, |w, stops| {
        let total: usize = stops.iter().map(|&stop| stop as usize).sum();
        let event_weights = event_pmf(events, total);
        // (a state below the last level has an entry not stopped: final with P(N = L); at it, with P(N >= L))
        chain_walk(&links, stops, total, &vec![0; n], events.rule, |progress, event_count, _, probability| {
            let final_chance = if event_count < total {
                event_weights[event_count]
            } else {
                (1.0 - event_weights[..event_count].iter().sum::<f64>()).max(0.0)
            };
            *state_chances.entry(progress.to_vec()).or_insert(0.0) += w * probability * final_chance;
        });
    });
    let mut sorted: Vec<ChainState> =
        state_chances.into_iter().map(|(progress, probability)| ChainState { progress, probability }).collect();
    sorted.sort_by(|a, b| a.progress.cmp(&b.progress));
    let states = std::rc::Rc::new(sorted);
    CS.with(|cell| cell.borrow_mut().insert(key, std::rc::Rc::clone(&states)));
    states
}

/// Per chain-kind key (sorted kinds, fillers as (2, 2)): for each position, P(links done = l | N = n), n = 0..=max_events,
/// and whether the progress vector given every n is negatively upper-orthant dependent:
///   P(X_i >= t_i for all i) <= prod_i P(X_i >= t_i)   for every threshold vector t,
/// checked exactly (to 1e-12) over the key's state distribution. Early ends and stop risk are included (stop risk as
/// the mixture over where each risky card stops, which does not depend on N).
pub(super) struct KeyLevel {
    pub(super) marginals: Vec<Vec<Vec<f64>>>,
    pub(super) nuod: bool,
    pub(super) level_probabilities: Vec<Vec<f64>>,
    pub(super) dimensions: Vec<usize>,
    pub(super) grid_strides: Vec<usize>,
    pub(super) pairs: std::cell::RefCell<crate::FxMap<(usize, usize), bool>>,
    pub(super) pair_marginals: std::cell::RefCell<crate::FxMap<(usize, usize), std::rc::Rc<PairMarginals>>>,
}
/// A pair of entries' joint progress per event count: [m][a][b] = P(X_i = a, X_j = b | N = m) (`KeyLevel::pair_marg`).
pub(super) type PairMarginals = Vec<Vec<Vec<f64>>>;
impl KeyLevel {
    /// P(X_i = a, X_j = b | N = m)
    pub(super) fn pair_marg(&self, i: usize, j: usize, m: usize) -> Vec<Vec<f64>> {
        let mut marginal = vec![vec![0.0f64; self.dimensions[j]]; self.dimensions[i]];
        for (grid_index, &p) in self.level_probabilities[m].iter().enumerate() {
            if p != 0.0 {
                marginal[(grid_index / self.grid_strides[i]) % self.dimensions[i]]
                    [(grid_index / self.grid_strides[j]) % self.dimensions[j]] += p;
            }
        }
        marginal
    }
    /// `pair_marg` at every event count m = 0..=n, kept per pair (a function of the key alone)
    pub(super) fn pair_margs(&self, i: usize, j: usize) -> std::rc::Rc<PairMarginals> {
        if let Some(by_count) = self.pair_marginals.borrow().get(&(i, j)) {
            return std::rc::Rc::clone(by_count);
        }
        let by_count =
            std::rc::Rc::new((0..self.level_probabilities.len()).map(|m| self.pair_marg(i, j, m)).collect::<Vec<_>>());
        self.pair_marginals.borrow_mut().insert((i, j), std::rc::Rc::clone(&by_count));
        by_count
    }
    /// The orthant check with positions i and j as one block: for every up-set A of the (i, j) grid and every
    /// threshold vector t on the other positions, P((X_i, X_j) in A, X_k >= t_k) <= P((X_i, X_j) in A) prod P(X_k >= t_k).
    /// Any rising function of (X_i, X_j) is a positive combination of up-set indicators, so this is what the bound
    /// needs when one group holds both cards.
    pub(super) fn pair_ok(&self, i: usize, j: usize) -> bool {
        if let Some(&cached) = self.pairs.borrow().get(&(i, j)) {
            return cached;
        }
        let (first_dimension, second_dimension) = (self.dimensions[i], self.dimensions[j]);
        // up-sets of the di x dj grid: for each row a, the first column c_a in the set, non-increasing in a
        let mut up_sets: Vec<Vec<usize>> = vec![];
        fn upsets(a: usize, first_dimension: usize, hi: usize, current: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
            if a == first_dimension {
                out.push(current.clone());
                return;
            }
            for c in 0..=hi {
                current.push(c);
                upsets(a + 1, first_dimension, c, current, out);
                current.pop();
            }
        }
        upsets(0, first_dimension, second_dimension, &mut vec![], &mut up_sets);
        let n = self.dimensions.len();
        let others: Vec<usize> = (0..n).filter(|&k| k != i && k != j).collect();
        let mut ok = true;
        'outer: for m in 0..self.level_probabilities.len() {
            let distribution = &self.level_probabilities[m];
            for up_set in &up_sets {
                // indicator of the up-set (column c >= up_set[row]) over the grid, times the distribution
                let mut mass: Vec<f64> = distribution
                    .iter()
                    .enumerate()
                    .map(|(grid_index, &p)| {
                        let a = (grid_index / self.grid_strides[i]) % first_dimension;
                        let c = (grid_index / self.grid_strides[j]) % second_dimension;
                        if c >= up_set[a] { p } else { 0.0 }
                    })
                    .collect();
                let up_set_chance: f64 = mass.iter().sum();
                for &k in &others {
                    for grid_index in (0..mass.len()).rev() {
                        let c = (grid_index / self.grid_strides[k]) % self.dimensions[k];
                        if c + 1 < self.dimensions[k] {
                            mass[grid_index] += mass[grid_index + self.grid_strides[k]];
                        }
                    }
                }
                for grid_index in 0..mass.len() {
                    if !(grid_index / self.grid_strides[i]).is_multiple_of(first_dimension)
                        || !(grid_index / self.grid_strides[j]).is_multiple_of(second_dimension)
                    {
                        continue;
                    }
                    let prod: f64 = others
                        .iter()
                        .map(|&k| {
                            self.marginals[k][m][(grid_index / self.grid_strides[k]) % self.dimensions[k]..]
                                .iter()
                                .sum::<f64>()
                        })
                        .product();
                    if mass[grid_index] > up_set_chance * prod * (1.0 + 1e-12) + 1e-15 {
                        ok = false;
                        break 'outer;
                    }
                }
            }
        }
        self.pairs.borrow_mut().insert((i, j), ok);
        ok
    }
}
pub(super) fn key_level(events: ChainEvents, kinds: &[ChainKind]) -> std::rc::Rc<KeyLevel> {
    thread_local! { static KL: std::cell::RefCell<crate::FxMap<ChainKey, std::rc::Rc<KeyLevel>>> = std::cell::RefCell::new(crate::FxMap::default()); }
    thread_local! { static KL_LAST: std::cell::RefCell<Option<(ChainKey, std::rc::Rc<KeyLevel>)>> = const { std::cell::RefCell::new(None) }; }
    let events_key = events.key();
    if let Some(level) = KL_LAST.with(|cell| {
        cell.borrow()
            .as_ref()
            .filter(|last| last.0.kinds == kinds && last.0.events == events_key)
            .map(|last| std::rc::Rc::clone(&last.1))
    }) {
        return level;
    }
    let key = ChainKey { kinds: kinds.to_vec(), events: events_key };
    if let Some(level) = KL.with(|cell| cell.borrow().get(&key).cloned()) {
        KL_LAST.with(|cell| *cell.borrow_mut() = Some((key, std::rc::Rc::clone(&level))));
        return level;
    }
    let trials = events.count;
    let max_events = trials as usize;
    let n = kinds.len();
    let dimensions: Vec<usize> = kinds.iter().map(|k| k.stop as usize + 1).collect();
    let mut grid_strides = vec![1usize; n];
    let mut grid_size = 1usize;
    for i in 0..n {
        grid_strides[i] = grid_size;
        grid_size *= dimensions[i];
    }
    // per level, the distribution over the grid of progress vectors
    let mut level_probabilities = vec![vec![0.0f64; grid_size]; max_events + 1];
    let links: Vec<u8> = kinds.iter().map(|k| k.links).collect();
    for_each_realisation(kinds, |w, stops| {
        let total: usize = stops.iter().map(|&stop| stop as usize).sum();
        chain_walk(&links, stops, total, &grid_strides, events.rule, |_, event_count, grid_index, probability| {
            if event_count <= max_events {
                level_probabilities[event_count][grid_index] += w * probability;
            }
            if event_count == total {
                for m in total + 1..=max_events {
                    level_probabilities[m][grid_index] += w * probability;
                }
            }
        });
    });
    let mut marginals = vec![vec![vec![0.0f64; 0]; max_events + 1]; n];
    let mut nuod = true;
    // each grid index's digits, digits[g * n + i] = (g / grid_strides[i]) % dimensions[i] (the same sums below, in the
    // same order)
    let mut digits = vec![0u8; grid_size * n];
    {
        let mut odometer = vec![0u8; n];
        for grid_index in 0..grid_size {
            digits[grid_index * n..(grid_index + 1) * n].copy_from_slice(&odometer);
            for i in 0..n {
                if (odometer[i] as usize) + 1 < dimensions[i] {
                    odometer[i] += 1;
                    break;
                }
                odometer[i] = 0;
            }
        }
    }
    for m in 0..=max_events {
        let distribution = &level_probabilities[m];
        // (grid index g = b + c * stride + lo: by blocks b of stride * dimension, then digit c, then lo, the indices in
        // increasing order, so each sum takes its terms in the same order)
        for i in 0..n {
            let (stride, dimension) = (grid_strides[i], dimensions[i]);
            let mut marginal = vec![0.0f64; dimension];
            for block in distribution.chunks_exact(stride * dimension) {
                for (c, row) in block.chunks_exact(stride).enumerate() {
                    for &mass in row {
                        marginal[c] += mass;
                    }
                }
            }
            marginals[i][m] = marginal;
        }
        if !nuod {
            continue;
        }
        // upper-orthant sums by suffix sums along every axis (digit c from the top down: each entry adds the one a
        // digit above it, already summed; the entries of one digit are independent)
        let mut upper = distribution.clone();
        for i in 0..n {
            let (stride, dimension) = (grid_strides[i], dimensions[i]);
            for block in upper.chunks_exact_mut(stride * dimension) {
                for c in (0..dimension - 1).rev() {
                    let (lo, hi) = block[c * stride..(c + 2) * stride].split_at_mut(stride);
                    for (x, &y) in lo.iter_mut().zip(hi.iter()) {
                        *x += y;
                    }
                }
            }
        }
        let tail: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let mut tail_row = marginals[i][m].clone();
                for c in (0..dimensions[i] - 1).rev() {
                    tail_row[c] += tail_row[c + 1];
                }
                tail_row
            })
            .collect();
        for grid_index in 0..grid_size {
            let grid_digits = &digits[grid_index * n..(grid_index + 1) * n];
            let prod: f64 = (0..n).map(|i| tail[i][grid_digits[i] as usize]).product();
            if upper[grid_index] > prod * (1.0 + 1e-12) + 1e-15 {
                nuod = false;
                break;
            }
        }
    }
    let level = std::rc::Rc::new(KeyLevel {
        marginals,
        nuod,
        level_probabilities,
        dimensions,
        grid_strides,
        pairs: std::cell::RefCell::default(),
        pair_marginals: std::cell::RefCell::default(),
    });
    KL_LAST.with(|cell| *cell.borrow_mut() = Some((key.clone(), std::rc::Rc::clone(&level))));
    KL.with(|cell| cell.borrow_mut().insert(key, std::rc::Rc::clone(&level)));
    level
}
