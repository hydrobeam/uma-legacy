//! Legal card-count configurations and their bounds.
use crate::deck::caps::cap_class;
use crate::deck::enumerate::SearchClause;
use crate::deck::enumerate::traverse::{DeckSearchContext, SearchControl};
use crate::deck::space::{EntryMisses, GroupMissKey, GroupSlots, GroupTable, gm_g, gm_w};
/// Legal groups and key counts used to enumerate configurations, in their original order.
pub(crate) struct ConfigurationTables<'tab> {
    pub(crate) counts: &'tab [Vec<usize>],
    pub(crate) by_kind: &'tab [Vec<usize>],
    pub(crate) pal_entries: &'tab [usize],
    pub(crate) kind_place_values: &'tab [u64],
    pub(crate) groups_all: &'tab [&'tab Vec<usize>],
    pub(crate) group_lock_bits: &'tab [u8],
    pub(crate) group_class_masks: &'tab [u64],
    pub(crate) filler_kind_index: usize,
    pub(crate) plain_pal_count: usize,
    pub(crate) all_lock_bits: u8,
    pub(crate) skip_bounded_keys: bool,
}

/// Configuration-build buffers stay owned by preparation; white/gold odds are reused by traversal.
pub(crate) struct ConfigurationScratch<'scratch> {
    pub(crate) white_chances: &'scratch mut [f64],
    pub(crate) gold_chances: &'scratch mut [f64],
    pub(crate) groups: &'scratch mut Vec<GroupSlots>,
    pub(crate) clause_bounds: &'scratch mut [f64],
    pub(crate) order_bounds: &'scratch mut [f64],
    pub(crate) ratio_products: &'scratch mut [f64],
    pub(crate) key_white: &'scratch mut [f64],
    pub(crate) key_gold: &'scratch mut [f64],
}

// every (key, real (2, 2) entries, kind-0 entries) with its groups and root bound; visited best bound first
pub(crate) struct SearchConfig {
    pub(crate) root: f64,
    pub(crate) code: u64,
    pub(crate) real_filler_count: usize,
    pub(crate) pal_entry_count: usize,
    pub(crate) filler_count: usize,
    pub(crate) plain_pal_count: usize,
    pub(crate) groups: Vec<GroupSlots>,
    pub(crate) group_misses: std::rc::Rc<GroupTable>,
    pub(crate) ratio_prefixes: std::rc::Rc<GroupTable>,
    // A tighter bound may prune without changing the root-bound visit order.
    pub(crate) prune_bound: f64,
    // Root clause values retained only for RB_CHECK.
    pub(crate) clause_bounds: Vec<f64>,
}

// A configuration passing the per-target bound, its root not yet computed: `upper_bound` bounds its root
// (the root sums fixed * max over outfits of min(A, B r) per (trainee, chance), the per-target value fixed *
// the sum of max A, the same A to the bit; terms >= 0, so the root is at most the value up to rounding);
// `ratio_key_index`: its key's slot in `key_ratios` (the key's ratio tables, built on first need)
pub(crate) struct PendingConfig {
    pub(crate) upper_bound: f64,
    pub(crate) ratio_key_index: usize,
    pub(crate) code: u64,
    pub(crate) real_filler_count: usize,
    pub(crate) pal_entry_count: usize,
    pub(crate) filler_count: usize,
    pub(crate) plain_pal_count: usize,
    pub(crate) needed_groups: u64,
    pub(crate) groups: Vec<GroupSlots>,
    pub(crate) group_misses: std::rc::Rc<GroupTable>,
}

/// Eager configurations retain their stable root order; pending configurations retain
/// insertion order until traversal builds its stable upper-bound ordering.
pub(crate) struct ConfigurationQueue {
    pub(crate) configurations: Vec<SearchConfig>,
    pub(crate) pending: Vec<PendingConfig>,
    pub(crate) key_ratios: Vec<Option<std::rc::Rc<GroupTable>>>,
}

/// The same bound operations serve eager configuration construction and lazy queue expansion.
pub(crate) struct ConfigurationBounds<Ratios, Odds, Root> {
    pub(crate) ratio_prefixes: Ratios,
    pub(crate) odds: Odds,
    pub(crate) root: Root,
}

/// Lower bounds on one target's misses over the remaining cards.
struct TargetMissBounds {
    white: f64,
    gold: f64,
}

impl<V: Fn(&SearchClause, &[f64], &[f64]) -> f64> DeckSearchContext<'_, '_, V> {
    /// Enumerate legal card-count configurations, preserving stable root-bound order and
    /// deferring ratio tables for the product search until traversal actually needs them.
    pub(crate) fn build_configurations(
        &self,
        tables: ConfigurationTables<'_>,
        scratch: ConfigurationScratch<'_>,
        best_value: f64,
        bounds: &ConfigurationBounds<
            impl Fn(&[f64], &[f64], &[u32], u64) -> GroupTable,
            impl Fn(&[GroupSlots], &GroupTable, &mut [f64], &mut [f64]),
            impl Fn(&[GroupSlots], &GroupTable, &[f64], &[f64], &mut [f64], &mut [f64], &mut [f64]),
        >,
    ) -> ConfigurationQueue {
        let Self { space, clauses, controls, clause_value, .. } = self;
        let SearchControl {
            best,
            rho,
            target_count,
            size,
            kind_count,
            set_id,
            fixed,
            cap_free,
            class_caps,
            lazy,
            check_ratio_bounds,
            ..
        } = *controls;
        let ConfigurationTables {
            counts,
            by_kind,
            pal_entries,
            kind_place_values,
            groups_all,
            group_lock_bits,
            group_class_masks,
            filler_kind_index,
            plain_pal_count,
            all_lock_bits,
            skip_bounded_keys,
        } = tables;
        let ConfigurationScratch {
            white_chances,
            gold_chances,
            groups: group_buffer,
            clause_bounds,
            order_bounds,
            ratio_products,
            key_white,
            key_gold,
        } = scratch;
        let key_slack = |clause: &SearchClause| 1e-11 * clause.lineages.len() as f64 * fixed.max(1.0);
        let mut configurations: Vec<SearchConfig> = vec![];
        let mut pending: Vec<PendingConfig> = vec![];
        let mut key_ratios: Vec<Option<std::rc::Rc<GroupTable>>> = vec![];
        for kind_counts in counts {
            let chain_slots: usize = kind_counts.iter().sum();
            if (0..kind_count).any(|i| i != filler_kind_index && kind_counts[i] > by_kind[i].len()) {
                continue;
            }
            let mut needs_ratio_key = true;
            let code: u64 = kind_counts
                .iter()
                .zip(kind_place_values)
                .map(|(&count, &place_value)| count as u64 * place_value)
                .sum();
            let key_misses = space.key_miss(code);
            let (white_misses, gold_misses) = (&key_misses.white, &key_misses.gold);
            // per kind group and target: products of the r lowest miss chances of distinct entries, r = 0..=size
            // (any r entries of the group miss at least that much); cached per trainee and key
            // (the groups a configuration of this key can use: its kinds and, with slots left, the pal entries)
            let needed_groups: u64 =
                (0..kind_count).filter(|&i| kind_counts[i] > 0).fold(0u64, |mask, i| mask | 1 << i)
                    | if chain_slots < size { 1 << kind_count } else { 0 };
            let group_misses = space.gm_cached(
                GroupMissKey { set_id, code },
                groups_all,
                needed_groups,
                EntryMisses { white: white_misses, gold: gold_misses, target_count },
            );
            // the key's bound: no variant (real (2, 2) and pal entries counted 0 ..= their most) can pass when the value
            // at odds above every variant's fails with room for rounding (`key_slack`)
            if skip_bounded_keys {
                let (real_filler_limit, pal_entry_limit) = (
                    kind_counts[filler_kind_index].min(by_kind[filler_kind_index].len()),
                    (size - chain_slots).min(pal_entries.len()),
                );
                let lowest_misses = |i: usize, count: usize, t: usize| -> TargetMissBounds {
                    let group_bound = group_misses.grp(i);
                    (0..=count).fold(TargetMissBounds { white: f64::INFINITY, gold: f64::INFINITY }, |misses, slots| {
                        TargetMissBounds {
                            white: misses.white.min(gm_w(group_bound, target_count, slots)[t]),
                            gold: misses.gold.min(gm_g(group_bound, target_count, slots)[t]),
                        }
                    })
                };
                for t in 0..target_count {
                    let (mut white_product, mut gold_product) = (1.0f64, 1.0f64);
                    if pal_entry_limit > 0 {
                        let misses = lowest_misses(kind_count, pal_entry_limit, t);
                        white_product *= misses.white;
                        gold_product *= misses.gold;
                    }
                    for i in (0..kind_count).rev() {
                        let slots = if i == filler_kind_index { real_filler_limit } else { kind_counts[i] };
                        if slots == 0 {
                            continue;
                        }
                        let misses = if i == filler_kind_index {
                            lowest_misses(i, slots, t)
                        } else {
                            let group_bound = group_misses.grp(i);
                            TargetMissBounds {
                                white: gm_w(group_bound, target_count, slots)[t],
                                gold: gm_g(group_bound, target_count, slots)[t],
                            }
                        };
                        white_product *= misses.white;
                        gold_product *= misses.gold;
                    }
                    key_white[t] = 1.0 - white_product;
                    key_gold[t] = 1.0 - gold_product;
                }
                // (a band's class caps are at most rho)
                let none = if best {
                    (clause_value(&clauses[0], key_white, key_gold) + key_slack(&clauses[0])) * (1.0 + 1e-9)
                        < best_value
                } else {
                    clauses.iter().all(|clause| {
                        rho * (clause_value(clause, key_white, key_gold) + key_slack(clause)) * (1.0 + 1e-9)
                            < clause.threshold
                    })
                };
                if none {
                    continue;
                }
            }
            // per kind group and (clause, chance, outfit): products of the r best single-card ratios, r = 0..=size;
            // built only for keys with a configuration the per-target bound lets through
            let mut ratio_prefixes: Option<std::rc::Rc<GroupTable>> = None;
            for real_filler_count in 0..=kind_counts[filler_kind_index].min(by_kind[filler_kind_index].len()) {
                let filler_count = kind_counts[filler_kind_index] - real_filler_count;
                let remaining_slots = size - chain_slots;
                for pal_entry_count in 0..=remaining_slots.min(pal_entries.len()) {
                    let plain_pals = remaining_slots - pal_entry_count;
                    if plain_pals > plain_pal_count {
                        continue;
                    }
                    if let Some(required_pals) = space.deck_job.pals
                        && (pal_entry_count + plain_pals < required_pals || plain_pals > required_pals)
                    {
                        continue;
                    }
                    let groups = &mut *group_buffer;
                    groups.clear();
                    for i in 0..kind_count {
                        let slots = if i == filler_kind_index { real_filler_count } else { kind_counts[i] };
                        if slots > 0 {
                            groups.push(GroupSlots { group: i, slots });
                        }
                    }
                    if pal_entry_count > 0 {
                        groups.push(GroupSlots { group: kind_count, slots: pal_entry_count });
                    }
                    let groups = &**group_buffer;
                    if all_lock_bits & !groups.iter().fold(0u8, |mask, group| mask | group_lock_bits[group.group]) != 0
                    {
                        continue;
                    }
                    // the per-target bound first, the ratio products only for configurations it lets through
                    (bounds.odds)(groups, &group_misses, white_chances, gold_chances);
                    // the configuration's class cap (every deck of it is in a class at most this one's)
                    let class_cap = if best {
                        rho
                    } else {
                        class_caps
                            [cap_class(groups.iter().fold(0u64, |mask, group| mask | group_class_masks[group.group]))]
                    };
                    let ideal_value = if best { clause_value(&clauses[0], white_chances, gold_chances) } else { 0.0 };
                    let ideal = if cap_free {
                        true
                    } else if best {
                        ideal_value * (1.0 + 1e-9) >= best_value
                    } else {
                        clauses.iter().any(|clause| {
                            class_cap * clause_value(clause, white_chances, gold_chances) * (1.0 + 1e-9)
                                >= clause.threshold
                        })
                    };
                    if !ideal {
                        continue;
                    }
                    if lazy {
                        if needs_ratio_key {
                            key_ratios.push(None);
                            needs_ratio_key = false;
                        }
                        pending.push(PendingConfig {
                            upper_bound: ideal_value * (1.0 + 1e-12),
                            ratio_key_index: key_ratios.len() - 1,
                            code,
                            real_filler_count,
                            pal_entry_count,
                            filler_count,
                            plain_pal_count: plain_pals,
                            needed_groups,
                            groups: groups.to_vec(),
                            group_misses: std::rc::Rc::clone(&group_misses),
                        });
                        continue;
                    }
                    if ratio_prefixes.is_none() {
                        ratio_prefixes = Some(std::rc::Rc::new((bounds.ratio_prefixes)(
                            white_misses,
                            gold_misses,
                            &key_misses.touched_targets,
                            needed_groups,
                        )));
                    }
                    let ratio_prefixes = ratio_prefixes.as_ref().unwrap();
                    let (clause_values, order_values) = (&mut *clause_bounds, &mut *order_bounds);
                    (bounds.root)(
                        groups,
                        ratio_prefixes,
                        white_chances,
                        gold_chances,
                        ratio_products,
                        clause_values,
                        order_values,
                    );
                    let root_of = |clause_values: &[f64]| {
                        if best {
                            clause_values[0]
                        } else {
                            (0..clauses.len())
                                .map(|clause_index| {
                                    if clauses[clause_index].threshold == f64::NEG_INFINITY {
                                        // An unrestricted catalogue must include zero-valued decks too.
                                        return f64::INFINITY;
                                    }
                                    rho * clause_values[clause_index] / clauses[clause_index].threshold.max(1e-300)
                                })
                                .fold(f64::NEG_INFINITY, f64::max)
                        }
                    };
                    let (root, prune_bound) = (root_of(order_values), root_of(clause_values));
                    // (the visiting order stays by `root`; a configuration whose root at its class cap fails is left out)
                    if !best
                        && !cap_free
                        && class_cap < rho
                        && (0..clauses.len()).all(|clause_index| {
                            class_cap * clause_values[clause_index] * (1.0 + 1e-9) < clauses[clause_index].threshold
                        })
                    {
                        continue;
                    }
                    configurations.push(SearchConfig {
                        root,
                        code,
                        real_filler_count,
                        pal_entry_count,
                        filler_count,
                        plain_pal_count: plain_pals,
                        groups: groups.to_vec(),
                        group_misses: std::rc::Rc::clone(&group_misses),
                        ratio_prefixes: std::rc::Rc::clone(ratio_prefixes),
                        prune_bound,
                        clause_bounds: if check_ratio_bounds { clause_values.to_vec() } else { vec![] },
                    });
                }
            }
        }
        // stable: equal bounds keep enumeration order
        configurations.sort_by(|a, b| b.root.partial_cmp(&a.root).unwrap());
        ConfigurationQueue { configurations, pending, key_ratios }
    }
}
