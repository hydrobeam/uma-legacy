//! Bounds for decks with a card whose own chain feeds two or more targets (a nested chain, `caps::NESTED_CHAIN`), a
//! shape the class caps (`EXACT_CAPS`, assumed) are known to miss: exact / product reaches about 1 / P(the card's lower
//! link fires) for two targets (4.00 on a constructed Seiun Sky [Paint the Sky Red] deck without pals), 1 / (ab) for
//! three at links reached with chances a, b, c.
//!
//! Derivation. One trainee at one inheritance chance (a career deck search); a deck D with one nested entry j whose
//! chain feeds the targets K (k >= 2 of them) and no other entry's chain feeds a target of K. Let x be the chain state
//! (each chain card's links done). Given x the deck's sources are independent and each group's levels negatively
//! upper-orthant dependent (`group_assoc` implies `group_nuod_lvl`), and a target's landing factor rises with its
//! level, so per outfit o
//!   P(all | x) <= prod_k F_ok(x),                                                                                (1)
//! F_ok(x) = P(k lands | x) = `LandPre::f` at the deck's miss products given x. For k in K, F_ok depends on x only
//! through d, j's links done: F_ok(d) = f_ok(u_k ng_jk(d), v_k nh_jk(d)), with j's misses after d links
//! (`miss_levels`) and the other entries' products u_k (no gold) and v_k (no hint), constants. When K is every
//! chain-varying target of D (no other chain card feeds a target), no other F_ok depends on x, so each is the product
//! value's factor p_ok, and averaging (1) over x, with H_o the fixed factor times the other targets' factors,
//!   exact(D) <= B(D) = max_o H_o E_d[prod_{k in K} F_ok(d)] = max_o P_o(D) R_o,   R_o = E_d[prod_K F_ok] / prod_K p_ok,
//! d's law the final progress of j in D's chain process (`chain_states`, at most four states), P_o the product value
//! at outfit o, whose factors take each card's misses at its own marginal chances. Proven, exact in the chain law and
//! loose only by (1) (`NestedTrainee::deck_bound`).
//!
//! Where D also has other chain cards feeding targets outside K, or a group not negatively associated (an uncertified
//! group: (1) fails), B(D) is the same expression with every other entry at its marginal misses: it leaves out the
//! dependence across j and the other cards (their shared chain events), or within the uncertified group. That is an
//! assumption, named (`caps::multi_feeder`, `ConfigurationQueue::multi`, `multi_feeder_cap`). A deck where another
//! chain card also feeds a target of K, or with two nested entries, has no B: the class cap holds for it, assumed (the
//! nested pair with another feeder in class 1, `two_feeder_cap`; the rest under `multi_feeder_cap`).
//!
//! Per configuration (chain kinds and events fixed, so d's law known) and trainee, R_o is bounded over every deck of it
//! of that shape whatever its other entries: E_d[prod_K F_ok] is affine in (u_k, v_k) for fixed values of the other
//! targets, p_ok affine and positive in it, so R_o is a ratio of affine functions of each target's point on the
//! triangle 0 <= v <= u <= 1 and takes its largest value at a corner of each. The configuration's cap is the largest R
//! over the nested entries its chain kinds allow, the outfits and the 3^k corner combinations (`NestedTrainee::
//! pair_caps`, memoised per search): near 1 wherever the card's links nearly always fire.
//!
//! Not covered by any bound, measured too costly: two cards each feeding one target (about 1.17 computed for decks
//! without pals) and a nested pair another chain card also feeds. The same corner caps over sets of two cards made
//! p10's plan 1.9 times, p16's 2.9 times and boot_mob's 15 times slower: these stay under the assumed class caps
//! (`two_feeder_cap`, `multi_feeder_cap`).
//!
//! Use (`enumerate`): in the career's capped band a node or configuration that can still reach a nested chain is tested
//! at the larger of its class's cap and the configuration's nested cap, and such a leaf deck passes when its own B
//! reaches the bar; the career's product search (`SearchMode::BestCovered`) ranks decks by max(P, B / rho), so
//! rho times its best bounds every deck's exact value (`ruled`, the seed, `Bounded`). Where the cap is at most the class
//! cap the search is the capped one, unchanged.
use crate::deck::caps::{NESTED_CHAIN, NESTED_TARGET_LIMIT};
use crate::deck::chains::{ChainEvents, ChainKind, EventsKey, chain_states};
use crate::deck::joint::{LandPre, MissFactors};
use crate::deck::sources::TargetSources;
use crate::deck::space::DeckSpace;
use crate::skills::DeckChances;

/// Per entry of a deck space, what the nested bounds read: the targets its chain feeds, its chain kind and character,
/// its misses after each number of links done; per target its chain feeders; and the configuration caps found so far.
pub(crate) struct NestedChains {
    target_count: usize,
    /// Per entry, the targets its chain links feed (bit t), as `cv_mask` counts them.
    chain_targets: Vec<u32>,
    kinds: Vec<ChainKind>,
    /// Per entry with chain targets, `TargetSources::miss_levels` (row d: d links done); empty for the others.
    levels: Vec<Vec<MissFactors>>,
    /// Per target, the entries whose chains feed it.
    feeders: Vec<Vec<usize>>,
    /// Per (chain key, events), its chain law.
    laws: std::cell::RefCell<crate::FxMap<(u64, EventsKey), std::rc::Rc<ChainLaw>>>,
}

/// A configuration's cap for a trainee (`NestedTrainee::pair_caps`): on R over its decks with a nested card whose
/// targets no other card's chain feeds (`nested`, module notes); 0 where it can hold no nested card.
#[derive(Clone, Copy, Default)]
pub(crate) struct PairCaps {
    pub(crate) nested: f64,
}

impl NestedChains {
    /// The entries' tables, from their sources and chain kinds.
    pub(super) fn new(sources: &[TargetSources], kinds: &[ChainKind], target_count: usize) -> Self {
        let chain_targets: Vec<u32> = sources
            .iter()
            .map(|entry_sources| (entry_sources.cv_mask() & !NESTED_CHAIN & u64::from(u32::MAX)) as u32)
            .collect();
        let levels = sources
            .iter()
            .zip(kinds)
            .zip(&chain_targets)
            .map(
                |((entry_sources, kind), &targets)| {
                    if targets != 0 { entry_sources.miss_levels(kind.stop, target_count) } else { vec![] }
                },
            )
            .collect();
        let feeders = (0..target_count)
            .map(|t| (0..chain_targets.len()).filter(|&j| chain_targets[j] >> t & 1 == 1).collect())
            .collect();
        Self {
            target_count,
            chain_targets,
            kinds: kinds.to_vec(),
            levels,
            feeders,
            laws: std::cell::RefCell::default(),
        }
    }
    /// The chain law of chain key `code` (its kinds `kinds`) at `events`, memoised
    fn law(&self, code: u64, events: ChainEvents, kinds: Vec<ChainKind>) -> std::rc::Rc<ChainLaw> {
        let key = (code, events.key());
        if let Some(law) = self.laws.borrow().get(&key) {
            return std::rc::Rc::clone(law);
        }
        let law = std::rc::Rc::new(ChainLaw::new(events, kinds));
        self.laws.borrow_mut().insert(key, std::rc::Rc::clone(&law));
        law
    }
}

/// The targets of a mask (bit t), ascending.
fn targets_of(mask: u32) -> Vec<usize> {
    (0..32).filter(|&t| mask >> t & 1 == 1).collect()
}

/// The final progress of a deck's chain positions (sorted chain kinds, `chain_states`): per position its law.
pub(crate) struct ChainLaw {
    kinds: Vec<ChainKind>,
    single: Vec<Vec<f64>>,
}

impl ChainLaw {
    /// The law of chain kinds `kinds` (sorted) at `events`.
    pub(super) fn new(events: ChainEvents, mut kinds: Vec<ChainKind>) -> Self {
        kinds.sort_unstable();
        let states = chain_states(events, &kinds);
        let mut single: Vec<Vec<f64>> = kinds.iter().map(|kind| vec![0.0; usize::from(kind.stop) + 1]).collect();
        for state in states.iter() {
            for (position, law) in single.iter_mut().enumerate() {
                law[usize::from(state.progress[position])] += state.probability;
            }
        }
        Self { kinds, single }
    }
    /// The law of the links done of an entry of chain kind `kind` (any position of the kind: they are exchangeable);
    /// None when the key lacks it.
    fn of_kind(&self, kind: ChainKind) -> Option<&[f64]> {
        let position = self.kinds.iter().position(|&listed| listed == kind)?;
        Some(&self.single[position])
    }
}

/// One nested entry in one chain law: the targets K its chain feeds, the law of its links done, its marginal no-gold
/// and no-hint products per target of K (the product value's), and the other entries' products per target of K.
struct NestedState<'state> {
    chains: &'state NestedChains,
    entry: usize,
    targets: &'state [usize],
    law: &'state [f64],
    marginals: &'state [(f64, f64)],
    others: &'state [(f64, f64)],
}

impl NestedState<'_> {
    /// R = E_d[prod_K F_k(d)] / prod_K p_k of the module notes at the landing factors `landing` (per target of K)
    fn ratio(&self, landing: &[&LandPre]) -> f64 {
        let target_count = self.chains.target_count;
        let mut expected = 0.0f64;
        for (done, &chance) in self.law.iter().enumerate() {
            if chance == 0.0 {
                continue;
            }
            expected += self.targets.iter().enumerate().fold(chance, |term, (i, &t)| {
                let misses = self.chains.levels[self.entry][done * target_count + t];
                let (no_gold, no_hint) = self.others[i];
                term * landing[i].f(no_gold * misses.no_gold, no_hint * misses.no_hint)
            });
        }
        let mut product = 1.0f64;
        for (i, &(no_gold, no_hint)) in self.others.iter().enumerate() {
            let (marginal_gold, marginal_hint) = self.marginals[i];
            let factor = landing[i].f(no_gold * marginal_gold, no_hint * marginal_hint);
            if factor <= 0.0 {
                return 1.0;
            }
            product *= factor;
        }
        expected / product
    }
}

impl NestedChains {
    /// `NestedTrainee::pair_caps` at chain law `law`, `marginal` each entry's (no gold, no hint) per target at its links'
    /// marginal chances (the product value's), for `trainee`
    pub(super) fn caps_with(
        &self,
        law: &ChainLaw,
        marginal: &dyn Fn(usize, usize) -> (f64, f64),
        trainee: &NestedTrainee,
    ) -> PairCaps {
        let target_count = self.target_count;
        let mut caps = PairCaps::default();
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)];
        // (a card past `NESTED_TARGET_LIMIT` targets gets no cap: its decks are class 3, their caps assumed,
        // `caps::multi_feeder`; never a maximum over some of the corners)
        let capped = |j: usize| (2..=NESTED_TARGET_LIMIT).contains(&self.chain_targets[j].count_ones());
        for j in (0..self.chain_targets.len()).filter(|&j| capped(j)) {
            let Some(entry_law) = law.of_kind(self.kinds[j]) else { continue };
            let targets = targets_of(self.chain_targets[j]);
            let marginals: Vec<(f64, f64)> = targets.iter().map(|&t| marginal(j, t)).collect();
            let mut others = vec![(1.0, 1.0); targets.len()];
            let mut landing: Vec<&LandPre> = Vec::with_capacity(targets.len());
            // (every corner combination: corner c_i of target i, the combination's base-3 digits)
            for combination in 0..3usize.pow(targets.len() as u32) {
                let mut digits = combination;
                for other in &mut others {
                    *other = corners[digits % 3];
                    digits /= 3;
                }
                let state = NestedState {
                    chains: self,
                    entry: j,
                    targets: &targets,
                    law: entry_law,
                    marginals: &marginals,
                    others: &others,
                };
                for outfit in 0..trainee.outfit_count {
                    landing.clear();
                    landing.extend(targets.iter().map(|&t| &trainee.landing[outfit * target_count + t]));
                    caps.nested = caps.nested.max(state.ratio(&landing)).max(1.0);
                }
            }
        }
        caps
    }
}

/// A deck as the nested bound reads it: its entries and its hint odds per target (white given no gold, gold).
#[derive(Clone, Copy)]
pub(crate) struct NestedDeck<'deck> {
    pub(crate) entries: &'deck [usize],
    pub(crate) white: &'deck [f64],
    pub(crate) gold: &'deck [f64],
}

/// The trainee side of the nested bound: her landing factor per outfit and target at the search's chance, and the fixed
/// gold factor.
pub(crate) struct NestedTrainee {
    target_count: usize,
    outfit_count: usize,
    /// Row o * target_count + t.
    landing: Vec<LandPre>,
    fixed: f64,
    /// Per (chain key, events), its pair caps for her.
    caps: std::cell::RefCell<crate::FxMap<(u64, EventsKey), PairCaps>>,
}

impl NestedTrainee {
    /// Trainee `trainee` at inheritance chance `inheritance` (the deck search's lineage), at `chances`' event chances.
    pub(crate) fn new(chances: &DeckChances, trainee: crate::uma::UmaIndex, inheritance: f64) -> Self {
        let target_count = chances.model.uncertain.len();
        let outfit_count = chances.model.outfit_options[trainee].len();
        let landing = (0..outfit_count)
            .flat_map(|outfit| {
                (0..target_count).map(move |t| {
                    LandPre::new(
                        chances.model.outfit_options[trainee][outfit].coverage[t],
                        chances.event_chances[trainee][outfit][t],
                        inheritance,
                    )
                })
            })
            .collect();
        Self {
            target_count,
            outfit_count,
            landing,
            fixed: chances.model.fixed_factor(),
            caps: std::cell::RefCell::default(),
        }
    }
    /// The pair caps of chain key `code` at `events` for her (module notes): over every entry of the space (a superset of
    /// any search's legal ones); memoised for the search.
    pub(crate) fn pair_caps(&self, space: &DeckSpace, code: u64, events: ChainEvents) -> PairCaps {
        if let Some(&caps) = self.caps.borrow().get(&(code, events.key())) {
            return caps;
        }
        let chains = &space.nested_chains;
        let kinds = space.kinds_of_code(code);
        let present = |j: usize| kinds.contains(&chains.kinds[j]);
        let target_count = chains.target_count;
        let any_pair = (0..target_count).filter(|&t| chains.feeders[t].iter().any(|&j| present(j))).count() >= 2;
        let caps = if any_pair {
            let law = chains.law(code, events, kinds);
            let key_misses = space.key_miss(code, events);
            let marginal = |j: usize, t: usize| {
                let index = j * target_count + t;
                (key_misses.gold[index], key_misses.gold[index] * key_misses.white[index])
            };
            chains.caps_with(&law, &marginal, self)
        } else {
            PairCaps::default()
        };
        self.caps.borrow_mut().insert((code, events.key()), caps);
        caps
    }
    /// The deck's value at one outfit at its odds `white` (given no gold) and `gold`: fixed factor times the product of
    /// its landing factors (`land_dw`'s form)
    fn outfit_value(&self, outfit: usize, white: &[f64], gold: &[f64]) -> f64 {
        let target_count = self.target_count;
        (0..target_count).fold(self.fixed, |value, t| {
            let no_gold = 1.0 - gold[t];
            value * self.landing[outfit * target_count + t].f(no_gold, no_gold * (1.0 - white[t]))
        })
    }
    /// B(D) of the module notes for a deck with one nested entry whose targets no other entry's chain feeds (an upper
    /// bound on its exact value, proven when no other chain card feeds a target and every group is negatively
    /// associated, else under `multi_feeder_cap`): `deck`'s entries and odds, its chain key (`DeckSpace::deck_code`)
    /// and events. NEG_INFINITY for a deck of any other shape (no bound here).
    pub(crate) fn deck_bound(&self, space: &DeckSpace, deck: NestedDeck, code: u64, events: ChainEvents) -> f64 {
        let chains = &space.nested_chains;
        if !deck.entries.iter().any(|&j| chains.chain_targets[j].count_ones() >= 2) {
            return f64::NEG_INFINITY;
        }
        let law = chains.law(code, events, space.kinds_of_code(code));
        let key_misses = space.key_miss(code, events);
        let target_count = self.target_count;
        let marginal = |j: usize, t: usize| {
            let index = j * target_count + t;
            (key_misses.gold[index], key_misses.gold[index] * key_misses.white[index])
        };
        self.bound_with(chains, &law, deck, &marginal)
    }
    /// `deck_bound` at chain law `law`, `marginal` each entry's (no gold, no hint) per target at its links' marginal
    /// chances (the product value's)
    pub(super) fn bound_with(
        &self,
        chains: &NestedChains,
        law: &ChainLaw,
        deck: NestedDeck,
        marginal: &dyn Fn(usize, usize) -> (f64, f64),
    ) -> f64 {
        let NestedDeck { entries, white, gold } = deck;
        let mut nested = entries.iter().copied().filter(|&j| chains.chain_targets[j].count_ones() >= 2);
        let (Some(j), None) = (nested.next(), nested.next()) else { return f64::NEG_INFINITY };
        let feeds = chains.chain_targets[j];
        // (no other entry's chain feeds a target of K: the shape the bound covers)
        if entries.iter().any(|&other| other != j && chains.chain_targets[other] & feeds != 0) {
            return f64::NEG_INFINITY;
        }
        let Some(entry_law) = law.of_kind(chains.kinds[j]) else { return f64::NEG_INFINITY };
        let targets = targets_of(feeds);
        let marginals: Vec<(f64, f64)> = targets.iter().map(|&t| marginal(j, t)).collect();
        let others: Vec<(f64, f64)> = targets
            .iter()
            .map(|&t| {
                entries.iter().filter(|&&other| other != j).fold((1.0f64, 1.0f64), |(no_gold, no_hint), &other| {
                    let (entry_gold, entry_hint) = marginal(other, t);
                    (no_gold * entry_gold, no_hint * entry_hint)
                })
            })
            .collect();
        let state =
            NestedState { chains, entry: j, targets: &targets, law: entry_law, marginals: &marginals, others: &others };
        let target_count = self.target_count;
        let mut landing: Vec<&LandPre> = Vec::with_capacity(targets.len());
        (0..self.outfit_count)
            .map(|outfit| {
                landing.clear();
                landing.extend(targets.iter().map(|&t| &self.landing[outfit * target_count + t]));
                self.outfit_value(outfit, white, gold) * state.ratio(&landing)
            })
            .fold(f64::NEG_INFINITY, f64::max)
    }
}
