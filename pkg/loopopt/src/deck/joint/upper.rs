//! The negative-association upper bound on a deck's joint P(all) (`upper_na`), over the per-chain target tables it
//! builds. The landing factor and miss rows it shares with the exact sum are in `joint`; the state-conditioned bound
//! is in `state_bound`.
use crate::deck::chains::{ChainKind, event_pmf_job, key_level};
use crate::deck::joint::{JointQuery, MissFactors, land_factor};
use crate::deck::sources::{
    LinkMisses, SourceEntry, TargetChainMisses, TargetMisses, TargetSources, TrainingHint, group_gh, group_nuod,
};
use crate::skills::Model;
struct BoundTarget {
    target: usize,
    /// Cumulative misses after each number of completed chain links, starting at zero.
    by_progress: Vec<MissFactors>,
}

struct BoundChain {
    kind: ChainKind,
    targets: Vec<BoundTarget>,
}

/// The one pair of chain positions whose shared targets are integrated together.
#[derive(Clone, Copy, PartialEq, Eq)]
struct ChainPair {
    first: usize,
    second: usize,
}

/// Upper bound on exact P(all), per inheritance chance, including the fixed-gold factor.
/// Each chain card is grouped with the targets it feeds. Given N = n, the checked upper-orthant inequality lets
/// the expectation of the product be bounded by the product of expectations over those groups. One pair of cards
/// may form a joint group (`KeyLevel::pair_ok`); targets that would connect more cards are bounded at full progress.
/// The proof writes each rising group factor as a positive combination of indicators {X_i >= t}; the checked
/// inequality bounds each term, giving U = sum_n P(N = n) product_groups E[group factor | n], times the factors
/// of targets outside those groups. Event outcomes and training hints are independent of the chain allocation.
/// Event groups must pass `group_nuod`, since their per-target marginals replace the full outcome distribution.
/// Returns None when either dependence check is unproven; callers must then use a different bound or score exactly.
pub(crate) fn upper_na(
    model: &Model,
    query: JointQuery,
    entries: &[SourceEntry<&TargetSources>],
    filler: usize,
) -> Option<Vec<f64>> {
    let target_count = model.uncertain.len();
    // every group touching two or more targets must be negatively upper-orthant dependent, or the per-target
    // marginals below could understate a group whose outcomes hint targets together
    // (checked once per source, when it is built: `TargetSources::uncertified_orthant`)
    for SourceEntry { sources, .. } in entries {
        if sources.uncertified_orthant {
            if query.checks.upper_why
                && let Some(group) = sources
                    .fixed_outcomes
                    .iter()
                    .chain(sources.chain_outcomes.iter().map(|outcome| &outcome.outcomes))
                    .find(|candidate| group_gh(candidate).len() >= 2 && !group_nuod(candidate))
            {
                eprintln!("why: group {group:?}");
            }
            return None;
        }
    }
    let (mut base_no_gold, mut base_no_hint) = (vec![1.0f64; target_count], vec![1.0f64; target_count]);
    for SourceEntry { sources, .. } in entries {
        for &TrainingHint { target: k, probability: p } in &sources.training_hints {
            base_no_hint[k] *= 1.0 - p;
        }
        // (`fixed_misses`: the fixed groups' target and miss factors in group order, as group_gh gives them: the same
        // factors in the same order, computed once per source)
        for &TargetMisses { target: k, no_gold: miss_gold, no_hint: miss_hint } in &sources.fixed_misses {
            base_no_gold[k] *= miss_gold;
            base_no_hint[k] *= miss_hint;
        }
    }
    // chain cards in key order (fillers included): (kind, per target: cumulative (miss gold, miss any) after l links)
    let mut chains: Vec<BoundChain> = vec![];
    for SourceEntry { sources, kind } in entries {
        if *kind == ChainKind::NONE {
            continue;
        }
        let stop = kind.stop as usize;
        let targets: Vec<BoundTarget> = sources
            .chain_misses
            .iter()
            .map(|TargetChainMisses { target: k, links }| {
                let mut cumulative = vec![MissFactors { no_gold: 1.0, no_hint: 1.0 }; stop + 1];
                for l in 1..=stop {
                    cumulative[l] = cumulative[l - 1];
                    for &LinkMisses { link, no_gold: miss_gold, no_hint: miss_hint } in links {
                        if link as usize == l {
                            cumulative[l].no_gold *= miss_gold;
                            cumulative[l].no_hint *= miss_hint;
                        }
                    }
                }
                BoundTarget { target: *k, by_progress: cumulative }
            })
            .collect();
        chains.push(BoundChain { kind: *kind, targets });
    }
    for _ in 0..filler {
        chains.push(BoundChain { kind: ChainKind::FILLER, targets: vec![] });
    }
    chains.sort_by_key(|chain| chain.kind);
    let kinds: Vec<ChainKind> = chains.iter().map(|chain| chain.kind).collect();
    // groups: cards joined by a target their chains both feed; at most one group of two cards, none larger. A target
    // that would join a larger group (three or more feeders, or two feeders other than the one pair) is "maxed": its
    // factor is taken at every feeder's full progress, a constant, and it leaves its feeders' target lists. Valid
    // (derived): per target phi = G(1 - ng) + W(ng - nh a) with a = (1 - e)(1 - q) in [0, 1] and G > W, so phi >= 0
    // (nh <= ng) and phi falls with ng and with nh; the miss products only fall as links progress (each link's factor
    // is at most 1), so f_k(L) <= f_k(full progress) on every chain state L, and E[prod_k f_k] <= E[prod_kept f_k] x
    // prod_maxed f_k^max; the kept targets then meet the structure the bound below needs.
    let mut feeders: Vec<Vec<usize>> = vec![vec![]; target_count];
    for (chain_index, chain) in chains.iter().enumerate() {
        for target in &chain.targets {
            feeders[target.target].push(chain_index);
        }
    }
    let mut pair: Option<ChainPair> = None;
    let mut maxed: Vec<Option<MissFactors>> = vec![None; target_count];
    for k in 0..target_count {
        let use_full_progress = match feeders[k].len() {
            0 | 1 => false,
            2 => {
                let candidate_pair = ChainPair { first: feeders[k][0], second: feeders[k][1] };
                if pair.is_some_and(|previous| previous != candidate_pair) {
                    true
                } else {
                    pair = Some(candidate_pair);
                    false
                }
            }
            _ => true,
        };
        if use_full_progress {
            let (mut miss_gold, mut miss_hint) = (base_no_gold[k], base_no_hint[k]);
            for &chain_index in &feeders[k] {
                #[allow(clippy::unwrap_in_result, reason = "feeders[k] lists only chains with target k (built above)")]
                let cumulative = &chains[chain_index].targets.iter().find(|row| row.target == k).unwrap().by_progress;
                let last = cumulative[cumulative.len() - 1];
                miss_gold *= last.no_gold;
                miss_hint *= last.no_hint;
            }
            maxed[k] = Some(MissFactors { no_gold: miss_gold, no_hint: miss_hint });
        }
    }
    if maxed.iter().any(std::option::Option::is_some) {
        for chain in &mut chains {
            chain.targets.retain(|row| maxed[row.target].is_none());
        }
        for k in 0..target_count {
            if maxed[k].is_some() {
                feeders[k].clear();
            }
        }
    }
    let has_feeder: Vec<bool> = (0..target_count).map(|k| !feeders[k].is_empty()).collect();
    let chain_levels = if kinds.is_empty() { None } else { Some(key_level(query.chain_events, &kinds)) };
    if let Some(chain_level) = &chain_levels {
        let ok = match pair {
            None => chain_level.nuod,
            Some(ChainPair { first, second }) => chain_level.pair_ok(first, second),
        };
        if !ok {
            return None;
        }
    }
    let event_probabilities = event_pmf_job(query.chain_events);
    let outfits = &model.outfit_options[query.trainee];
    let fixed = model.fixed_factor();
    let phi = |outfit: usize, q: f64, k: usize, no_gold: f64, no_hint: f64| -> f64 {
        land_factor(outfits[outfit].coverage[k], query.event_chances[outfit][k], q, no_gold, no_hint)
    };
    let pair_marginals: Option<std::rc::Rc<crate::deck::chains::PairMarginals>> = match (pair, &chain_levels) {
        (Some(ChainPair { first, second }), Some(chain_level)) => Some(chain_level.pair_margs(first, second)),
        _ => None,
    };
    Some(
        query
            .inheritance_chances
            .iter()
            .map(|&q| {
                let mut best = 0.0f64;
                for outfit in 0..outfits.len() {
                    let mut outside_product = 1.0;
                    for k in 0..target_count {
                        if let Some(MissFactors { no_gold: miss_gold, no_hint: miss_hint }) = maxed[k] {
                            outside_product *= phi(outfit, q, k, miss_gold, miss_hint);
                        } else if !has_feeder[k] {
                            outside_product *= phi(outfit, q, k, base_no_gold[k], base_no_hint[k]);
                        }
                    }
                    // per single card and links done: the product over the targets only it feeds
                    let single = |chain_index: usize, l: usize| -> f64 {
                        chains[chain_index]
                            .targets
                            .iter()
                            .filter(|row| feeders[row.target].len() == 1)
                            .map(|row| {
                                phi(
                                    outfit,
                                    q,
                                    row.target,
                                    base_no_gold[row.target] * row.by_progress[l].no_gold,
                                    base_no_hint[row.target] * row.by_progress[l].no_hint,
                                )
                            })
                            .product()
                    };
                    let single_products: Vec<Vec<f64>> = (0..chains.len())
                        .map(|chain_index| {
                            (0..=chains[chain_index].kind.stop as usize).map(|l| single(chain_index, l)).collect()
                        })
                        .collect();
                    // the pair's group: both cards' single targets and the shared ones, over (first_links,
                    // second_links)
                    let pair_products: Option<Vec<Vec<f64>>> = pair.map(|ChainPair { first, second }| {
                        (0..=chains[first].kind.stop as usize)
                            .map(|first_links| {
                                (0..=chains[second].kind.stop as usize)
                                    .map(|second_links| {
                                        let mut group_product =
                                            single_products[first][first_links] * single_products[second][second_links];
                                        for target in &chains[first].targets {
                                            let k = target.target;
                                            let first_misses = &target.by_progress;
                                            if feeders[k].len() == 2 {
                                                let second_misses = &chains[second]
                                                    .targets
                                                    .iter()
                                                    .find(|row| row.target == k)
                                                    .unwrap()
                                                    .by_progress;
                                                group_product *= phi(
                                                    outfit,
                                                    q,
                                                    k,
                                                    base_no_gold[k]
                                                        * first_misses[first_links].no_gold
                                                        * second_misses[second_links].no_gold,
                                                    base_no_hint[k]
                                                        * first_misses[first_links].no_hint
                                                        * second_misses[second_links].no_hint,
                                                );
                                            }
                                        }
                                        group_product
                                    })
                                    .collect()
                            })
                            .collect()
                    });
                    let mut total = 0.0;
                    for (m, &event_probability) in event_probabilities.iter().enumerate() {
                        if event_probability == 0.0 {
                            continue;
                        }
                        let mut product = outside_product;
                        if let Some(chain_level) = &chain_levels {
                            for chain_index in 0..chains.len() {
                                if chains[chain_index].targets.is_empty()
                                    || pair.is_some_and(|ChainPair { first, second }| {
                                        chain_index == first || chain_index == second
                                    })
                                {
                                    continue;
                                }
                                product *= chain_level.marginals[chain_index][m]
                                    .iter()
                                    .zip(&single_products[chain_index])
                                    .map(|(probability, value)| probability * value)
                                    .sum::<f64>();
                            }
                            if let (Some(pair_values), Some(pair_rows)) = (&pair_products, &pair_marginals) {
                                product *= pair_rows[m]
                                    .iter()
                                    .zip(pair_values)
                                    .map(|(row, values)| {
                                        row.iter()
                                            .zip(values)
                                            .map(|(probability, value)| probability * value)
                                            .sum::<f64>()
                                    })
                                    .sum::<f64>();
                            }
                        }
                        total += event_probability * product;
                    }
                    best = best.max(total);
                }
                fixed * best * (1.0 + 1e-9)
            })
            .collect(),
    )
}
