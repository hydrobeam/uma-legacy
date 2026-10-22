//! Target sources of a deck entry: outcome groups, training hints and chain links per card, the event data merged into
//! a job, and the deck entries of a job.
use crate::deck::chains::{ChainKind, stop_risk_code};
use crate::deck::hints::{hint_base, hint_miss, hint_spec, scenario_name};
use crate::deck::space::{MAX_DECK, MAX_KINDS, MAX_UNCERTAIN, kinds_all};
use crate::deck::{Card, CardsRaw, DeckJob};
use crate::skills::SkillsRaw;
use serde_json::{Value, json};
/// Mutually exclusive outcomes of one event firing. Probabilities sum to at most 1; the remainder gives no hint.
/// Each outcome's rewards occur together, using hint levels 1 (white) and 2 (gold).
pub(super) type Group = Vec<EventOutcome>;

/// An event's probability and the target hints awarded together.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct EventOutcome<Rewards = Vec<TargetReward>> {
    pub(super) probability: f64,
    pub(super) rewards: Rewards,
}

/// One uncertain target's hint level in an outcome (1 white, 2 gold).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct TargetReward {
    pub(super) target: usize,
    pub(super) level: u8,
}

/// A card's independent training hint for one uncertain target.
#[derive(Clone, Copy, Debug)]
pub(super) struct TrainingHint {
    pub(super) target: usize,
    pub(super) probability: f64,
}

/// An outcome group that fires when the chain reaches a link.
#[derive(Clone, Debug)]
pub(super) struct ChainOutcome {
    pub(super) link: u8,
    pub(super) outcomes: Group,
}

/// Miss factors contributed by a fixed outcome group to one target.
#[derive(Clone, Copy)]
pub(super) struct TargetMisses {
    pub(super) target: usize,
    pub(super) no_gold: f64,
    pub(super) no_hint: f64,
}

/// Miss factors contributed when one chain link fires.
#[derive(Clone, Copy)]
pub(super) struct LinkMisses {
    pub(super) link: u8,
    pub(super) no_gold: f64,
    pub(super) no_hint: f64,
}

/// One target's miss factors, in chain-link order.
#[derive(Clone)]
pub(super) struct TargetChainMisses {
    pub(super) target: usize,
    pub(super) links: Vec<LinkMisses>,
}

/// A deck entry's target sources and its chain scheduling kind.
#[derive(Clone)]
pub(crate) struct SourceEntry<S> {
    pub(crate) sources: S,
    pub(crate) kind: ChainKind,
}
/// Per target a group touches: (k, P(gold for k), P(any hint for k)).
pub(super) fn group_gh(group: &Group) -> Vec<(usize, f64, f64)> {
    let mut marginals: Vec<(usize, f64, f64)> = vec![];
    for EventOutcome { probability, rewards } in group {
        // An outcome can list both white and gold for one target; only its best reward counts.
        let mut best_levels: Vec<TargetReward> = vec![];
        for &TargetReward { target, level } in rewards {
            match best_levels.iter_mut().find(|reward| reward.target == target) {
                Some(reward) => reward.level = reward.level.max(level),
                None => best_levels.push(TargetReward { target, level }),
            }
        }
        for TargetReward { target, level } in best_levels {
            let marginal = if let Some(i) = marginals.iter().position(|entry| entry.0 == target) {
                &mut marginals[i]
            } else {
                marginals.push((target, 0.0, 0.0));
                marginals.last_mut().unwrap()
            };
            if level >= 2 {
                marginal.1 += probability;
            }
            marginal.2 += probability;
        }
    }
    marginals.sort_by_key(|entry| entry.0);
    marginals
}

/// Highest hint level for one target in an outcome, using the event model's 1 = white / 2 = gold convention.
/// An absent target has level 0; any listed reward below gold is treated as white.
fn outcome_hint_level(rewards: &[TargetReward], target: usize) -> u8 {
    rewards
        .iter()
        .filter(|reward| reward.target == target)
        .map(|reward| if reward.level >= 2 { 2u8 } else { 1u8 })
        .max()
        .unwrap_or(0)
}

/// Whether a group's per-target hint levels are negatively associated, as the state-conditioned bound needs: every
/// outcome hints at most one target (one draw: a one-trial multinomial is negatively associated, and so are increasing
/// functions of disjoint sets of its outcomes), or it touches two targets and their levels are negatively quadrant
/// dependent, P(A >= a, B >= b) <= P(A >= a) P(B >= b) for a, b in {white, gold} (for two variables negative quadrant
/// dependence is negative association). Three or more targets with an outcome hinting several: not covered.
pub(super) fn group_assoc(group: &Group) -> bool {
    if group.iter().all(|outcome| outcome.rewards.iter().all(|reward| reward.target == outcome.rewards[0].target)) {
        return true;
    }
    let mut targets: Vec<usize> =
        group.iter().flat_map(|outcome| outcome.rewards.iter().map(|reward| reward.target)).collect();
    targets.sort_unstable();
    targets.dedup();
    if targets.len() != 2 {
        return false;
    }
    for a in 1..=2u8 {
        for b in 1..=2u8 {
            let first_chance: f64 = group
                .iter()
                .filter(|outcome| outcome_hint_level(&outcome.rewards, targets[0]) >= a)
                .map(|x| x.probability)
                .sum();
            let second_chance: f64 = group
                .iter()
                .filter(|outcome| outcome_hint_level(&outcome.rewards, targets[1]) >= b)
                .map(|x| x.probability)
                .sum();
            let joint_chance: f64 = group
                .iter()
                .filter(|outcome| {
                    outcome_hint_level(&outcome.rewards, targets[0]) >= a
                        && outcome_hint_level(&outcome.rewards, targets[1]) >= b
                })
                .map(|x| x.probability)
                .sum();
            if joint_chance > first_chance * second_chance * (1.0 + 1e-12) + 1e-15 {
                return false;
            }
        }
    }
    true
}
/// An event outcome projected onto a target subset.
struct HintMaskOutcome {
    probability: f64,
    targets: u32,
}

/// An event outcome projected onto the best hint level of each touched target.
struct HintLevelOutcome {
    probability: f64,
    levels: Vec<u8>,
}

/// Certify the any-hint orthant inequality by checking every target subset S:
/// P(all of S hinted) <= product over S of P(k hinted). Exclusive single-target outcomes satisfy this;
/// outcomes hinting several targets together need the explicit check. Results are cached by the group's bits.
/// More than 20 targets is unproven.
pub(super) fn group_nuod(group: &Group) -> bool {
    thread_local! { static GN: std::cell::RefCell<crate::FxMap<Vec<(u64, Vec<TargetReward>)>, bool>> = std::cell::RefCell::new(crate::FxMap::default()); }
    let key: Vec<(u64, Vec<TargetReward>)> =
        group.iter().map(|outcome| (outcome.probability.to_bits(), outcome.rewards.clone())).collect();
    if let Some(r) = GN.with(|c| c.borrow().get(&key).copied()) {
        return r;
    }
    let mut targets: Vec<usize> =
        group.iter().flat_map(|outcome| outcome.rewards.iter().map(|reward| reward.target)).collect();
    targets.sort_unstable();
    targets.dedup();
    // Exhaustive checking is exponential. An unchecked group is not a proof of negative dependence;
    // decline the bound before constructing masks, which also avoids shifts beyond the mask width.
    if targets.len() > 20 {
        return false;
    }
    // per outcome, the targets it hints as a bit set
    let outcomes: Vec<HintMaskOutcome> = group
        .iter()
        .map(|outcome| HintMaskOutcome {
            probability: outcome.probability,
            targets: outcome.rewards.iter().fold(0u32, |mask, reward| {
                mask | (1 << targets.iter().position(|&target| target == reward.target).unwrap())
            }),
        })
        .collect();
    let marginal_chance: Vec<f64> = (0..targets.len())
        .map(|i| outcomes.iter().filter(|o| o.targets >> i & 1 == 1).map(|o| o.probability).sum())
        .collect();
    let certified = (1u32..1 << targets.len()).filter(|s| s.count_ones() >= 2).all(|s| {
        let joint: f64 = outcomes.iter().filter(|o| o.targets & s == s).map(|o| o.probability).sum();
        joint
            <= (0..targets.len()).filter(|i| s >> i & 1 == 1).map(|i| marginal_chance[i]).product::<f64>()
                * (1.0 + 1e-12)
                + 1e-15
    });
    GN.with(|c| c.borrow_mut().insert(key, certified));
    certified
}
/// Whether a group's per-target hint levels (Y_k: 0 none, 1 white, 2 gold, the best level its outcome gives k) are
/// negatively upper-orthant dependent at every level: for every set S of two or more of its targets and every s in
/// {1, 2}^S, P(Y_k >= s_k for all k in S) <= prod_S P(Y_k >= s_k). What `certified_cap_free` needs: a nonnegative
/// rising function of one Y_k is a nonnegative combination of 1, 1{Y_k >= 1} and 1{Y_k >= 2}, so the expected product
/// of such functions of distinct targets is at most the product of their expectations. Implied by `group_assoc` (two
/// targets: the same test; single-target outcomes: every joint term is 0). Over 6 targets: not checked (false).
pub(super) fn group_nuod_lvl(group: &Group) -> bool {
    let mut targets: Vec<usize> =
        group.iter().flat_map(|outcome| outcome.rewards.iter().map(|reward| reward.target)).collect();
    targets.sort_unstable();
    targets.dedup();
    if targets.len() < 2 {
        return true;
    }
    if targets.len() > 6 {
        return false;
    }
    let m = targets.len();
    // per outcome: its chance and each target's level
    let outcomes: Vec<HintLevelOutcome> = group
        .iter()
        .map(|outcome| HintLevelOutcome {
            probability: outcome.probability,
            levels: targets.iter().map(|&target| outcome_hint_level(&outcome.rewards, target)).collect(),
        })
        .collect();
    let marginal_chance =
        |i: usize, s: u8| -> f64 { outcomes.iter().filter(|o| o.levels[i] >= s).map(|o| o.probability).sum() };
    // every assignment of a level 0 (not in S), 1 or 2 to each target, base 3
    let mut a = vec![0u8; m];
    loop {
        let mut i = 0;
        while i < m {
            a[i] += 1;
            if a[i] < 3 {
                break;
            }
            a[i] = 0;
            i += 1;
        }
        if i == m {
            return true;
        }
        if a.iter().filter(|&&x| x > 0).count() < 2 {
            continue;
        }
        let joint: f64 = outcomes.iter().filter(|o| (0..m).all(|i| o.levels[i] >= a[i])).map(|o| o.probability).sum();
        let product_chance: f64 = (0..m).filter(|&i| a[i] > 0).map(|i| marginal_chance(i, a[i])).product();
        if joint > product_chance * (1.0 + 1e-12) + 1e-15 {
            return false;
        }
    }
}

/// Target sources of one deck entry (a card, a card whose chain the player ends early, or a card taking a given
/// choice where an event offers targets on different choices): training hints (target, chance), outcome groups that
/// do not depend on the chain, and chain outcome groups with their link, sorted by link. `fixed_misses` and
/// `chain_misses` are the same
/// per target as miss factors (1 - P(gold), 1 - P(any hint)), the chain ones per link in link order.
#[derive(Clone, Default)]
pub(crate) struct TargetSources {
    pub(super) training_hints: Vec<TrainingHint>,
    pub(super) fixed_outcomes: Vec<Group>,
    pub(super) chain_outcomes: Vec<ChainOutcome>,
    pub(super) fixed_misses: Vec<TargetMisses>,
    pub(super) chain_misses: Vec<TargetChainMisses>,
    /// a group of it touching two or more targets has not been proved negatively upper-orthant dependent:
    /// `upper_na` does not cover a deck with it
    pub(super) uncertified_orthant: bool,
    /// a group's hint levels are not certified negatively associated by `group_assoc`; the ordinary
    /// state-conditioned bound does not cover it. Multiple targets alone do not imply this flag:
    /// `group_assoc` also certifies some two-target groups with joint rewards.
    pub(super) uncertified_association: bool,
    /// every group of it passes `group_nuod_lvl`: the state-conditioned bound holds for it without any cap (the
    /// cap-free deck search, `CareerSetup::best_deck_cf`, covers only decks of such entries)
    pub(super) certified_cap_free: bool,
}
impl TargetSources {
    pub(super) fn finish(mut self) -> Self {
        self.chain_outcomes.sort_by_key(|x| x.link);
        self.fixed_misses = self
            .fixed_outcomes
            .iter()
            .flat_map(group_gh)
            .map(|(target, gold_chance, hint_chance)| TargetMisses {
                target,
                no_gold: 1.0 - gold_chance,
                no_hint: 1.0 - hint_chance,
            })
            .collect();
        let mut chain_misses: Vec<TargetChainMisses> = vec![];
        for ChainOutcome { link, outcomes: group } in &self.chain_outcomes {
            for (target, gold_chance, hint_chance) in group_gh(group) {
                let links = if let Some(i) = chain_misses.iter().position(|x| x.target == target) {
                    &mut chain_misses[i].links
                } else {
                    chain_misses.push(TargetChainMisses { target, links: vec![] });
                    &mut chain_misses.last_mut().unwrap().links
                };
                match links.last_mut() {
                    Some(x) if x.link == *link => {
                        x.no_gold *= 1.0 - gold_chance;
                        x.no_hint *= 1.0 - hint_chance;
                    }
                    _ => links.push(LinkMisses { link: *link, no_gold: 1.0 - gold_chance, no_hint: 1.0 - hint_chance }),
                }
            }
        }
        chain_misses.sort_by_key(|x| x.target);
        self.chain_misses = chain_misses;
        self.uncertified_orthant = self
            .fixed_outcomes
            .iter()
            .chain(self.chain_outcomes.iter().map(|x| &x.outcomes))
            .any(|group| group_gh(group).len() >= 2 && !group_nuod(group));
        self.uncertified_association = self
            .fixed_outcomes
            .iter()
            .chain(self.chain_outcomes.iter().map(|x| &x.outcomes))
            .any(|group| !group_assoc(group));
        self.certified_cap_free =
            self.fixed_outcomes.iter().chain(self.chain_outcomes.iter().map(|x| &x.outcomes)).all(group_nuod_lvl);
        self
    }
    pub(super) const fn is_empty(&self) -> bool {
        self.training_hints.is_empty() && self.fixed_outcomes.is_empty() && self.chain_outcomes.is_empty()
    }

    /// Multiply an entry's miss chances into the output slices; link_chances[l - 1] = P(chain link l fires).
    /// Gold misses get P(no gold hint) and white misses get
    /// P(no white hint | no gold hint) = P(no hint) / P(no gold hint), both exact for the entry: its sources are
    /// independent of each other, while a skill's gold and white from one event (e.g. on different outcomes) are not.
    /// Links fire in order, so a skill offered at several links comes from the earliest one that fires:
    /// P(miss) = 1 - sum_l P(link l fires) * (P(not from links < l) - P(not from links <= l)).
    /// Both misses fall with every link chance, so the lowest possible link odds bound them from above.
    pub(crate) fn src_miss(&self, link_chances: &[f64], white_misses: &mut [f64], gold_misses: &mut [f64]) {
        let mut entry_no_gold = [1.0f64; 32];
        let mut entry_no_hint = [1.0f64; 32];
        for &TrainingHint { target, probability } in &self.training_hints {
            entry_no_hint[target] *= 1.0 - probability;
        }
        for &TargetMisses { target, no_gold, no_hint } in &self.fixed_misses {
            entry_no_gold[target] *= no_gold;
            entry_no_hint[target] *= no_hint;
        }
        for TargetChainMisses { target, links } in &self.chain_misses {
            let (mut prior_no_gold, mut prior_no_hint, mut chain_no_gold, mut chain_no_hint) =
                (1.0f64, 1.0f64, 1.0f64, 1.0f64);
            for &LinkMisses { link, no_gold, no_hint } in links {
                let link_chance = link_chances.get(link as usize - 1).copied().unwrap_or(0.0);
                let (next_no_gold, next_no_hint) = (prior_no_gold * no_gold, prior_no_hint * no_hint);
                chain_no_gold -= link_chance * (prior_no_gold - next_no_gold);
                chain_no_hint -= link_chance * (prior_no_hint - next_no_hint);
                prior_no_gold = next_no_gold;
                prior_no_hint = next_no_hint;
            }
            entry_no_gold[*target] *= chain_no_gold;
            entry_no_hint[*target] *= chain_no_hint;
        }
        for target in 0..white_misses.len() {
            gold_misses[target] *= entry_no_gold[target];
            white_misses[target] *=
                if entry_no_gold[target] > 0.0 { entry_no_hint[target] / entry_no_gold[target] } else { 1.0 };
        }
    }
}

/// Merge a card-event cache (card_events.json) into a deck job: skill names -> (group, level) via the skills table.
pub(crate) fn apply_events_db(v: &mut Value, sk: &SkillsRaw, db: &Value) {
    apply_event_lists(v, &event_lists(sk, db));
}

/// The job lists a card-event cache adds, in `EVENT_KEYS` order: the same for every job (the page's worker builds
/// them once, at init).
pub(crate) struct EventLists([Vec<Value>; 8]);
const EVENT_KEYS: [&str; 8] = ["events", "events_p", "odds_events", "end_links", "ends", "stop_risk", "alts", "alt_w"];
/// Each list after the job's own entries of it, in their order.
pub(crate) fn apply_event_lists(v: &mut Value, l: &EventLists) {
    for (k, add) in EVENT_KEYS.iter().zip(&l.0) {
        let mut a: Vec<Value> = match v[*k].take() {
            Value::Array(a) => a,
            _ => vec![],
        };
        a.extend(add.iter().cloned());
        v[*k] = Value::Array(a);
    }
}
/// Cached rewards are either a bare skill name (certain) or [name, chance]. Keep the cache reader's defaults
/// here so chain and early-end rewards interpret the same representation in the same way.
fn event_reward(value: &Value) -> (&str, f64) {
    if let Some(reward) = value.as_array() {
        (reward[0].as_str().unwrap_or(""), reward[1].as_f64().unwrap_or(0.0))
    } else {
        (value.as_str().unwrap_or(""), 1.0)
    }
}
pub(crate) fn event_lists(sk: &SkillsRaw, db: &Value) -> EventLists {
    let lookup_skill = |name: &str| {
        sk.skills.iter().find_map(|(group, white, gold)| {
            if white == name {
                Some((*group, 1u8))
            } else if gold.as_deref() == Some(name) {
                Some((*group, 2u8))
            } else {
                None
            }
        })
    };
    let [
        mut events,
        mut probabilistic_events,
        mut odds_events,
        mut end_links,
        mut end_rewards,
        mut stop_risks,
        mut alternatives,
        mut alternative_weights,
    ]: [Vec<Value>; 8] = Default::default();
    if let Some(cards) = db["cards"].as_object() {
        for (id, card) in cards {
            let Ok(id) = id.parse::<u32>() else { continue };
            for link in card["chain"].as_array().into_iter().flatten() {
                let link_number = link[0].as_u64().unwrap_or(0);
                for reward in link[1].as_array().into_iter().flatten() {
                    let (name, probability) = event_reward(reward);
                    if let Some((group, level)) = lookup_skill(name) {
                        if probability >= 1.0 {
                            events.push(json!([id, group, level, link_number]));
                        } else {
                            probabilistic_events.push(json!([id, group, level, link_number, probability]));
                        }
                    }
                }
            }
            for link in card["end"].as_array().into_iter().flatten() {
                let link_number = link[0].as_u64().unwrap_or(0);
                end_links.push(json!([id, link_number]));
                for reward in link[1].as_array().into_iter().flatten() {
                    let (name, probability) = event_reward(reward);
                    if let Some((group, level)) = lookup_skill(name) {
                        end_rewards.push(json!([id, link_number, group, level, probability]));
                    }
                }
            }
            for risk in card["stop_risk"].as_array().into_iter().flatten() {
                stop_risks.push(json!([id, risk[0], risk[1], risk[2]]));
            }
            for event in card["alts"].as_array().into_iter().flatten() {
                let choices: Vec<Vec<Vec<(u32, u8)>>> = event[1]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|choice| {
                        choice
                            .as_array()
                            .into_iter()
                            .flatten()
                            .map(|outcome| {
                                outcome
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .filter_map(|name| lookup_skill(name.as_str().unwrap_or("")))
                                    .collect()
                            })
                            .collect()
                    })
                    .collect();
                alternatives.push(json!([id, event[0], choices]));
            }
            for weights in card["alt_w"].as_array().into_iter().flatten() {
                alternative_weights.push(json!([id, weights[0], weights[1]]));
            }
            for name in card["always"].as_array().into_iter().flatten() {
                if let Some((group, level)) = lookup_skill(name.as_str().unwrap_or("")) {
                    events.push(json!([id, group, level, 0]));
                }
            }
            for event in card["chance"].as_array().into_iter().flatten() {
                if let Some((group, level)) = lookup_skill(event[0].as_str().unwrap_or("")) {
                    odds_events.push(json!([id, group, level, event[1].as_f64().unwrap_or(0.0)]));
                }
            }
        }
    }
    EventLists([
        events,
        probabilistic_events,
        odds_events,
        end_links,
        end_rewards,
        stop_risks,
        alternatives,
        alternative_weights,
    ])
}

/// A target reward supplied with a fixed chance.
#[derive(Clone, Copy)]
struct ProbabilisticReward {
    target: usize,
    level: u8,
    probability: f64,
}

/// A target reward supplied at a chain link (zero denotes a fixed event).
struct ChainReward {
    target: usize,
    level: u8,
    link: u8,
    probability: f64,
}

/// A link with several choices worth keeping, in link order.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct ChoiceLink {
    link: u8,
    choices: usize,
}

/// Builds a deck entry's target sources and chain kind from the deck job; shared by the deck search and the exact
/// evaluation of the decks it reports.
pub(super) struct Sources<'data> {
    pub(super) dj: &'data DeckJob,
    pub(super) unc: &'data [u32],
}
pub(super) const fn in_range(link: u8, stop: Option<u8>) -> bool {
    match stop {
        Some(st) => link < st,
        None => true,
    }
}

impl Sources<'_> {
    fn chain_rewards(&self, id: u32) -> Vec<ChainReward> {
        let unc = self.unc;
        let mut rewards: Vec<ChainReward> = self
            .dj
            .events
            .iter()
            .filter(|e| e.0 == id)
            .filter_map(|e| {
                unc.iter().position(|&g| g == e.1).map(|target| ChainReward {
                    target,
                    level: e.2,
                    link: e.3,
                    probability: 1.0,
                })
            })
            .collect();
        rewards.extend(self.dj.events_p.iter().filter(|e| e.0 == id).filter_map(|e| {
            unc.iter().position(|&g| g == e.1).map(|target| ChainReward {
                target,
                level: e.2,
                link: e.3,
                probability: e.4,
            })
        }));
        rewards
    }
    fn fixed_rewards(&self, id: u32) -> Vec<ProbabilisticReward> {
        self.dj
            .odds_events
            .iter()
            .filter(|e| e.0 == id)
            .filter_map(|e| {
                self.unc.iter().position(|&g| g == e.1).map(|target| ProbabilisticReward {
                    target,
                    level: e.2,
                    probability: e.3,
                })
            })
            .collect()
    }
    /// Training-hint draws per run: base(scenario) x (1 + hint frequency / 100) x (1 + d(scenario) x specialty / 100),
    /// each uniform over the card's skill options still below Lv 5. Per target in the pool: P(hinted) = 1 - hint_miss(1).
    pub(super) fn draws(&self, c: &Card) -> f64 {
        let s = &self.dj.scenario;
        hint_base(s) * (1.0 + c.hf / 100.0) * (1.0 + hint_spec(s) * c.spec / 100.0)
    }
    pub(super) fn training_hints(&self, c: &Card) -> Vec<TrainingHint> {
        if c.nskill == 0 {
            return vec![];
        }
        let p = 1.0 - hint_miss(1, c.nskill as usize, self.draws(c), c.hinc);
        self.unc
            .iter()
            .enumerate()
            .filter(|(_, g)| c.whites.contains(g))
            .map(|(k, _)| TrainingHint { target: k, probability: p })
            .collect()
    }
    /// A card's training hints over two or more of its targets as one outcome group: one outcome per non-empty set S
    /// of targets hinted, P(exactly S) = sum over U in S of (-1)^(|S|-|U|) hint_miss(m - |U|) (m targets in the
    /// pool), so the draws that hint one target cannot also hint another. Each target's own chance is unchanged.
    pub(super) fn train_group(&self, c: &Card, tr: &[TrainingHint]) -> Group {
        let (m, n, h) = (tr.len(), c.nskill as usize, self.draws(c));
        // Only the number of unhinted targets matters. Reuse these probabilities without changing the
        // inclusion-exclusion summation order (and therefore its floating-point rounding).
        let miss: Vec<f64> = (0..=m).map(|unhit| hint_miss(unhit, n, h, c.hinc)).collect();
        let mut g: Group = vec![];
        for sm in 1u32..(1 << m) {
            let mut w = 0.0f64;
            let ssz = sm.count_ones() as i32;
            let mut u = sm;
            loop {
                // every subset U of S (including the empty set)
                let usz = u.count_ones() as i32;
                w += if (ssz - usz) % 2 == 0 { 1.0 } else { -1.0 } * miss[m - usz as usize];
                if u == 0 {
                    break;
                }
                u = (u - 1) & sm;
            }
            g.push(EventOutcome {
                probability: w.max(0.0),
                rewards: (0..m)
                    .filter(|i| sm >> i & 1 == 1)
                    .map(|i| TargetReward { target: tr[i].target, level: 1u8 })
                    .collect(),
            });
        }
        g
    }
    /// early-end options: links where a card has a choice that ends its chain, and the target rewards of that choice
    pub(super) fn end_links(&self, id: u32) -> Vec<u8> {
        let mut v: Vec<u8> = self.dj.end_links.iter().filter(|e| e.0 == id).map(|e| e.1).collect();
        v.sort_unstable();
        v.dedup();
        v
    }
    fn end_rewards(&self, id: u32, link: u8) -> Vec<ProbabilisticReward> {
        self.dj
            .ends
            .iter()
            .filter(|e| e.0 == id && e.1 == link)
            .filter_map(|e| {
                self.unc.iter().position(|&g| g == e.2).map(|target| ProbabilisticReward {
                    target,
                    level: e.3,
                    probability: e.4,
                })
            })
            .collect()
    }
    /// The choices worth taking at an event listed in `alts` (None: not listed), as outcome groups over the targets:
    /// those with a target that no other choice beats on every target (P(gold) and P(any hint)); of two with the same
    /// odds, both are kept unless their outcomes are the same (then the first).
    pub(super) fn choices(&self, id: u32, link: u8) -> Option<Vec<Group>> {
        let event = self.dj.alts.iter().find(|event| event.0 == id && event.1 == link)?;
        let weights =
            self.dj.alt_w.iter().find(|weights| weights.0 == id && weights.1 == link).map(|weights| &weights.2);
        let nu = self.unc.len();
        let groups: Vec<Group> = event
            .2
            .iter()
            .enumerate()
            .map(|(choice_index, choice)| {
                let outcome_count = choice.len().max(1) as f64;
                choice
                    .iter()
                    .enumerate()
                    .map(|(outcome_index, outcome)| {
                        let mut rewards: Vec<TargetReward> = outcome
                            .iter()
                            .filter_map(|&(group, level)| {
                                self.unc
                                    .iter()
                                    .position(|&target| target == group)
                                    .map(|k| TargetReward { target: k, level })
                            })
                            .collect();
                        rewards.sort_unstable();
                        rewards.dedup();
                        let probability = weights
                            .and_then(|weights| weights.get(choice_index))
                            .and_then(|choice| choice.get(outcome_index))
                            .copied()
                            .unwrap_or(1.0 / outcome_count);
                        EventOutcome { probability, rewards }
                    })
                    .filter(|outcome| !outcome.rewards.is_empty())
                    .collect()
            })
            .collect();
        let profiles: Vec<Vec<(f64, f64)>> = groups
            .iter()
            .map(|group| {
                let mut profile = vec![(0.0, 0.0); nu];
                for (k, gold, any_hint) in group_gh(group) {
                    profile[k] = (gold, any_hint);
                }
                profile
            })
            .collect();
        let beats = |challenger: usize, candidate: usize| {
            profiles[challenger].iter().zip(&profiles[candidate]).all(|(a, b)| a.0 >= b.0 && a.1 >= b.1)
                && (profiles[challenger] != profiles[candidate]
                    || (groups[challenger] == groups[candidate] && challenger < candidate))
        };
        Some(
            (0..groups.len())
                .filter(|&candidate| {
                    !groups[candidate].is_empty()
                        && !(0..groups.len()).any(|challenger| {
                            challenger != candidate && !groups[challenger].is_empty() && beats(challenger, candidate)
                        })
                })
                .map(|candidate| groups[candidate].clone())
                .collect(),
        )
    }
    /// links (before the entry's stop) where two or more choices are worth taking, with how many
    fn choice_links(&self, c: &Card, stop: Option<u8>) -> Vec<ChoiceLink> {
        if c.pal {
            return vec![];
        }
        let mut v: Vec<ChoiceLink> = self
            .dj
            .alts
            .iter()
            .filter(|a| a.0 == c.id && in_range(a.1, stop))
            .filter_map(|a| {
                let n = self.choices(c.id, a.1).map_or(0, |x| x.len());
                (n >= 2).then_some(ChoiceLink { link: a.1, choices: n })
            })
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }
    /// The entry's choice variants: the choice taken at each `choice_links` link, 4 bits per link in link order.
    pub(super) fn variants(&self, c: &Card, stop: Option<u8>) -> Vec<u32> {
        let mut out = vec![0u32];
        for (pos, choice_link) in self.choice_links(c, stop).iter().enumerate() {
            out = out
                .iter()
                .flat_map(|&v| (0..choice_link.choices.min(16)).map(move |i| v | ((i as u32) << (4 * pos))))
                .collect();
        }
        out
    }
    pub(super) fn has_choices(&self, id: u32) -> bool {
        self.dj.alts.iter().any(|a| a.0 == id && self.choices(id, a.1).is_some_and(|x| !x.is_empty()))
    }
    /// Sources of a card played normally (stop = None) or ending its chain at link `stop` (links before it as usual,
    /// then the ending choice's rewards at that link), taking choice variant `variant`.
    pub(super) fn target_sources(&self, card: &Card, stop: Option<u8>, variant: u32) -> TargetSources {
        let one = |target: usize, level: u8, probability: f64| -> Group {
            vec![EventOutcome { probability, rewards: vec![TargetReward { target, level }] }]
        };
        let mut sources = TargetSources {
            fixed_outcomes: self
                .fixed_rewards(card.id)
                .into_iter()
                .map(|reward| one(reward.target, reward.level, reward.probability))
                .collect(),
            ..TargetSources::default()
        };
        let training_hints = self.training_hints(card);
        if training_hints.len() >= 2 {
            sources.fixed_outcomes.push(self.train_group(card, &training_hints));
        } else {
            sources.training_hints = training_hints;
        }
        let alt_links: Vec<u8> = self.dj.alts.iter().filter(|a| a.0 == card.id).map(|a| a.1).collect();
        for ChainReward { target, level, link, probability } in self.chain_rewards(card.id) {
            if link == 0 {
                sources.fixed_outcomes.push(one(target, level, probability));
                continue;
            }
            if card.pal || alt_links.contains(&link) || !in_range(link, stop) {
                continue;
            } // pals have no chain slots
            sources.chain_outcomes.push(ChainOutcome { link, outcomes: one(target, level, probability) });
        }
        if !card.pal {
            let choice_links = self.choice_links(card, stop);
            let mut links = alt_links;
            links.sort_unstable();
            links.dedup();
            for link in links {
                if !in_range(link, stop) {
                    continue;
                }
                let choices = self.choices(card.id, link).unwrap_or_default();
                let choice_index = choice_links
                    .iter()
                    .position(|choice_link| choice_link.link == link)
                    .map_or(0, |pos| ((variant >> (4 * pos)) & 15) as usize);
                if let Some(outcomes) = choices.get(choice_index) {
                    sources.chain_outcomes.push(ChainOutcome { link, outcomes: outcomes.clone() });
                }
            }
        }
        if let Some(stop_link) = stop {
            for ProbabilisticReward { target, level, probability } in self.end_rewards(card.id, stop_link) {
                sources
                    .chain_outcomes
                    .push(ChainOutcome { link: stop_link, outcomes: one(target, level, probability) });
            }
        }
        sources.finish()
    }
    pub(super) fn kind(&self, c: &Card, stop: Option<u8>) -> ChainKind {
        if c.pal || c.links == 0 {
            return ChainKind::NONE;
        }
        let risk: Vec<(u8, u8, u8)> =
            self.dj.stop_risk.iter().filter(|r| r.0 == c.id).map(|r| (r.1, r.2, r.3)).collect();
        let st = stop.unwrap_or(c.links);
        ChainKind { links: c.links, stop: st, stop_risk: stop_risk_code(&risk, st) }
    }
    /// "link 1: Uma Stan" for each link where the variant picks between choices ("" when there is none)
    pub(super) fn label(&self, c: &Card, stop: Option<u8>, var: u32, sk: &SkillsRaw) -> String {
        let name = |k: usize, lvl: u8| {
            sk.skills
                .iter()
                .find(|x| x.0 == self.unc[k])
                .map(|x| if lvl >= 2 { x.2.clone().unwrap_or_else(|| x.1.clone()) } else { x.1.clone() })
                .unwrap_or_default()
        };
        self.choice_links(c, stop)
            .iter()
            .enumerate()
            .map(|(pos, choice_link)| {
                let l = choice_link.link;
                let i = ((var >> (4 * pos)) & 15) as usize;
                let g = self.choices(c.id, l).unwrap_or_default().get(i).cloned().unwrap_or_default();
                let mut ns: Vec<String> = vec![];
                for EventOutcome { rewards: out, .. } in &g {
                    for &TargetReward { target: k, level: lvl } in out {
                        let n = name(k, lvl);
                        if !ns.contains(&n) {
                            ns.push(n);
                        }
                    }
                }
                format!("link {}: {}", l, ns.join(", "))
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Deck entries of a job: per entry the card (`rel`, an index into `cards`, the cards a deck may hold), chain kind,
/// early-end link (0 = played normally), choice variant and sources; the plain pals (owned pal characters with no
/// relevant card, best-rarity card) and the locked cards. `pool_ids`: the umas that may train (plain pals that block
/// none of them come first).
pub(crate) struct Entries<'data> {
    pub(super) cards: Vec<&'data Card>,
    pub(super) rel: Vec<usize>,
    pub(super) kind: Vec<ChainKind>,
    pub(super) endk: Vec<u8>,
    pub(super) vark: Vec<u32>,
    pub(super) srcs: Vec<TargetSources>,
    pub(super) plain_ids: Vec<u32>,
    pub(super) plain_names: Vec<String>,
    pub(super) plain_chars: Vec<u32>,
    pub(super) locked: Vec<usize>,
}
pub(crate) fn deck_entries<'data>(
    cr: &'data CardsRaw,
    dj: &DeckJob,
    unc: &[u32],
    pool_ids: &[u32],
) -> Result<Entries<'data>, Value> {
    // the packed search's limits (`MAX_DECK`, `MAX_UNCERTAIN`; `MAX_KINDS` and the locks below), refused before any table
    if !(1..=MAX_DECK).contains(&dj.deck_size) {
        return Err(json!({"error": format!("deck_size: a deck holds 1 to {MAX_DECK} cards, not {}.", dj.deck_size)}));
    }
    if unc.len() > MAX_UNCERTAIN {
        return Err(
            json!({"error": format!("At most {MAX_UNCERTAIN} targets can be left to the deck, not {}.", unc.len())}),
        );
    }
    let is_owned = |id: u32| dj.owned_cards.is_empty() || dj.owned_cards.contains(&id) || dj.lock_cards.contains(&id);
    // a card the scenario bans can't be in the deck, owned, borrowed or locked
    let banned: &[u32] = cr.restrict.get(&dj.scenario).map_or(&[], |v| v.as_slice());
    if let Some(c) =
        dj.lock_cards.iter().find(|id| banned.contains(id)).and_then(|id| cr.cards.iter().find(|c| c.id == *id))
    {
        return Err(
            json!({"error": format!("{} can't be used in {}: the game bans it from that scenario's deck.", c.name, scenario_name(&dj.scenario))}),
        );
    }
    let owned_c = |id: u32| (is_owned(id) || dj.borrow) && !dj.exclude_cards.contains(&id) && !banned.contains(&id);
    // ---- card sources ----
    let srcb = Sources { dj, unc };
    let cards: Vec<&Card> = cr.cards.iter().filter(|c| owned_c(c.id)).collect();
    // deck entries: card index (`rel`), chain kind, early-end link (0 = played normally), choice variant and sources
    let mut rel: Vec<usize> = vec![];
    let mut kind: Vec<ChainKind> = vec![];
    let mut endk: Vec<u8> = vec![];
    let mut vark: Vec<u32> = vec![];
    let mut srcs: Vec<TargetSources> = vec![];
    let mut plain_names: Vec<String> = vec![];
    let mut plain_ids: Vec<u32> = vec![];
    let mut plain_chars: Vec<u32> = vec![];
    for (i, c) in cards.iter().enumerate() {
        let useful = !srcb.chain_rewards(c.id).is_empty()
            || !srcb.fixed_rewards(c.id).is_empty()
            || !srcb.training_hints(c).is_empty()
            || srcb.has_choices(c.id);
        let locked = dj.lock_cards.contains(&c.id);
        if c.pal || c.links == 0 {
            // pals, and cards without chain events (R cards): like a pal, an R card takes no chain slot
            if useful || locked {
                rel.push(i);
                kind.push(ChainKind::NONE);
                endk.push(0);
                vark.push(0);
                srcs.push(srcb.target_sources(c, None, 0));
            }
        } else if c.rarity >= dj.min_rarity {
            // one entry per choice variant (an event offering targets on different choices: the player prioritises
            // one); the same card played to end its chain at link l is its own entry too. One character, so a deck
            // holds at most one of a card's entries.
            if useful || locked {
                for v in srcb.variants(c, None) {
                    rel.push(i);
                    kind.push(srcb.kind(c, None));
                    endk.push(0);
                    vark.push(v);
                    srcs.push(srcb.target_sources(c, None, v));
                }
            }
            for l in srcb.end_links(c.id) {
                if l == 0 || l > c.links {
                    continue;
                }
                for v in srcb.variants(c, Some(l)) {
                    let x = srcb.target_sources(c, Some(l), v);
                    if !x.is_empty() || locked {
                        rel.push(i);
                        kind.push(srcb.kind(c, Some(l)));
                        endk.push(l);
                        vark.push(v);
                        srcs.push(x);
                    }
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
        if rel.iter().any(|&r| cards[r].chara == c.chara) || plain_chars.contains(&c.chara) {
            continue;
        }
        plain_chars.push(c.chara);
        let best =
            cards.iter().filter(|x| x.pal && x.chara == c.chara && is_owned(x.id)).max_by_key(|x| x.rarity).unwrap();
        plain.push((pool_ids.iter().filter(|&&u| best.blocks(u, dj.checks)).count(), best.id, best.name.clone()));
    }
    plain.sort_by_key(|x| x.0); // stable: ties keep card order
    for (_, id, name) in plain {
        plain_ids.push(id);
        plain_names.push(name);
    }
    if dj.borrow {
        let mut seen: Vec<u32> = vec![];
        for c in &cards {
            if !c.pal
                || is_owned(c.id)
                || plain_chars.contains(&c.chara)
                || seen.contains(&c.chara)
                || rel.iter().any(|&r| cards[r].chara == c.chara)
            {
                continue;
            }
            let best = cards
                .iter()
                .enumerate()
                .filter(|(_, x)| x.pal && x.chara == c.chara)
                .max_by_key(|(_, x)| x.rarity)
                .unwrap()
                .0;
            seen.push(c.chara);
            rel.push(best);
            kind.push(ChainKind::NONE);
            endk.push(0);
            vark.push(0);
            srcs.push(srcb.target_sources(cards[best], None, 0));
        }
    }
    let mut locked: Vec<usize> = rel.iter().copied().filter(|&r| dj.lock_cards.contains(&cards[r].id)).collect();
    locked.sort_unstable();
    locked.dedup();
    if locked.len() > dj.deck_size {
        return Err(
            json!({"error": format!("{} locked cards don't fit in a deck of {}.", locked.len(), dj.deck_size)}),
        );
    }
    let nk = kinds_all(&kind).len();
    if nk > MAX_KINDS {
        return Err(
            json!({"error": format!("The cards' chains come in {nk} kinds (lengths, early ends, stop risks); the search takes at most {MAX_KINDS}.")}),
        );
    }
    Ok(Entries { cards, rel, kind, endk, vark, srcs, plain_ids, plain_names, plain_chars, locked })
}
