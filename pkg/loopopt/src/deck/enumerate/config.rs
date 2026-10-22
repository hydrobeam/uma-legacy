//! Legal card-count configurations and their bounds.
use super::{SearchClause, SearchControl};
use crate::deck::space::{EntryMisses, GroupSlots, GroupTableRequest};
use crate::deck::{DeckSpace, GroupTable, cap_class, gm_g, gm_w};

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

impl DeckSpace<'_> {
    /// Enumerate legal card-count configurations, preserving stable root-bound order and
    /// deferring ratio tables for the product search until traversal actually needs them.
    pub(crate) fn build_configurations(
        &self,
        clauses: &[SearchClause],
        controls: SearchControl,
        tables: ConfigurationTables<'_>,
        scratch: ConfigurationScratch<'_>,
        best_value: f64,
        clause_value: impl Fn(&SearchClause, &[f64], &[f64]) -> f64,
        bounds: &ConfigurationBounds<
            impl Fn(&[f64], &[f64], &[u32], u64) -> GroupTable,
            impl Fn(&[GroupSlots], &GroupTable, &mut [f64], &mut [f64]),
            impl Fn(&[GroupSlots], &GroupTable, &[f64], &[f64], &mut [f64], &mut [f64], &mut [f64]),
        >,
    ) -> ConfigurationQueue {
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
        } = controls;
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
        let key_slack = |cl: &SearchClause| 1e-11 * cl.lineages.len() as f64 * fixed.max(1.0);
        let mut configurations: Vec<SearchConfig> = vec![];
        let mut pending: Vec<PendingConfig> = vec![];
        let mut key_ratios: Vec<Option<std::rc::Rc<GroupTable>>> = vec![];
        for cnt in counts {
            let chain_slots: usize = cnt.iter().sum();
            if (0..kind_count).any(|i| i != filler_kind_index && cnt[i] > by_kind[i].len()) {
                continue;
            }
            let mut pkey_new = true;
            let code: u64 = cnt.iter().zip(kind_place_values).map(|(&c, &p)| c as u64 * p).sum();
            let key_misses = self.key_miss(code);
            let (mw, mg) = (&key_misses.white, &key_misses.gold);
            // per kind group and target: products of the r lowest miss chances of distinct entries, r = 0..=size
            // (any r entries of the group miss at least that much); cached per trainee and key
            // (the groups a configuration of this key can use: its kinds and, with slots left, the pal entries)
            let needed_groups: u64 = (0..kind_count).filter(|&i| cnt[i] > 0).fold(0u64, |m, i| m | 1 << i)
                | if chain_slots < size { 1 << kind_count } else { 0 };
            let group_misses = self.gm_cached(
                GroupTableRequest { set_id, code, groups: groups_all, needed_groups, deck_size: size },
                EntryMisses { white: mw, gold: mg, target_count },
            );
            // the key's bound: no variant (real (2, 2) and pal entries counted 0 ..= their most) can pass when the value
            // at odds above every variant's fails with room for rounding (`key_slack`)
            if skip_bounded_keys {
                let (r22, npm) = (
                    cnt[filler_kind_index].min(by_kind[filler_kind_index].len()),
                    (size - chain_slots).min(pal_entries.len()),
                );
                let lo = |i: usize, c: usize, t: usize| -> (f64, f64) {
                    let gb = group_misses.grp(i);
                    (0..=c).fold((f64::INFINITY, f64::INFINITY), |m, x| {
                        (m.0.min(gm_w(gb, target_count, x)[t]), m.1.min(gm_g(gb, target_count, x)[t]))
                    })
                };
                for t in 0..target_count {
                    let (mut a, mut b) = (1.0f64, 1.0f64);
                    if npm > 0 {
                        let x = lo(kind_count, npm, t);
                        a *= x.0;
                        b *= x.1;
                    }
                    for i in (0..kind_count).rev() {
                        let c = if i == filler_kind_index { r22 } else { cnt[i] };
                        if c == 0 {
                            continue;
                        }
                        let x = if i == filler_kind_index {
                            lo(i, c, t)
                        } else {
                            let gb = group_misses.grp(i);
                            (gm_w(gb, target_count, c)[t], gm_g(gb, target_count, c)[t])
                        };
                        a *= x.0;
                        b *= x.1;
                    }
                    key_white[t] = 1.0 - a;
                    key_gold[t] = 1.0 - b;
                }
                // (a band's class caps are at most rho)
                let none = if best {
                    (clause_value(&clauses[0], key_white, key_gold) + key_slack(&clauses[0])) * (1.0 + 1e-9)
                        < best_value
                } else {
                    clauses.iter().all(|cl| {
                        rho * (clause_value(cl, key_white, key_gold) + key_slack(cl)) * (1.0 + 1e-9) < cl.threshold
                    })
                };
                if none {
                    continue;
                }
            }
            // per kind group and (clause, chance, outfit): products of the r best single-card ratios, r = 0..=size;
            // built only for keys with a configuration the per-target bound lets through
            let mut ratio_prefixes: Option<std::rc::Rc<GroupTable>> = None;
            for real22 in 0..=cnt[filler_kind_index].min(by_kind[filler_kind_index].len()) {
                let filler = cnt[filler_kind_index] - real22;
                let rem = size - chain_slots;
                for np in 0..=rem.min(pal_entries.len()) {
                    let plain = rem - np;
                    if plain > plain_pal_count {
                        continue;
                    }
                    if let Some(p) = self.deck_job.pals
                        && (np + plain < p || plain > p)
                    {
                        continue;
                    }
                    let groups = &mut *group_buffer;
                    groups.clear();
                    for i in 0..kind_count {
                        let c = if i == filler_kind_index { real22 } else { cnt[i] };
                        if c > 0 {
                            groups.push(GroupSlots { group: i, slots: c });
                        }
                    }
                    if np > 0 {
                        groups.push(GroupSlots { group: kind_count, slots: np });
                    }
                    let groups = &**group_buffer;
                    if all_lock_bits & !groups.iter().fold(0u8, |m, group| m | group_lock_bits[group.group]) != 0 {
                        continue;
                    }
                    // the per-target bound first, the ratio products only for configurations it lets through
                    (bounds.odds)(groups, &group_misses, white_chances, gold_chances);
                    // the configuration's class cap (every deck of it is in a class at most this one's)
                    let rc = if best {
                        rho
                    } else {
                        class_caps[cap_class(groups.iter().fold(0u64, |m, group| m | group_class_masks[group.group]))]
                    };
                    let iv = if best { clause_value(&clauses[0], white_chances, gold_chances) } else { 0.0 };
                    let ideal = if cap_free {
                        true
                    } else if best {
                        iv * (1.0 + 1e-9) >= best_value
                    } else {
                        clauses
                            .iter()
                            .any(|cl| rc * clause_value(cl, white_chances, gold_chances) * (1.0 + 1e-9) >= cl.threshold)
                    };
                    if !ideal {
                        continue;
                    }
                    if lazy {
                        if pkey_new {
                            key_ratios.push(None);
                            pkey_new = false;
                        }
                        pending.push(PendingConfig {
                            upper_bound: iv * (1.0 + 1e-12),
                            ratio_key_index: key_ratios.len() - 1,
                            code,
                            real_filler_count: real22,
                            pal_entry_count: np,
                            filler_count: filler,
                            plain_pal_count: plain,
                            needed_groups,
                            groups: groups.to_vec(),
                            group_misses: std::rc::Rc::clone(&group_misses),
                        });
                        continue;
                    }
                    if ratio_prefixes.is_none() {
                        ratio_prefixes = Some(std::rc::Rc::new((bounds.ratio_prefixes)(
                            mw,
                            mg,
                            &key_misses.touched_targets,
                            needed_groups,
                        )));
                    }
                    let ratio_prefixes = ratio_prefixes.as_ref().unwrap();
                    let (cv, cvo) = (&mut *clause_bounds, &mut *order_bounds);
                    (bounds.root)(groups, ratio_prefixes, white_chances, gold_chances, ratio_products, cv, cvo);
                    let rt = |cv: &[f64]| {
                        if best {
                            cv[0]
                        } else {
                            (0..clauses.len())
                                .map(|ci| rho * cv[ci] / clauses[ci].threshold.max(1e-300))
                                .fold(f64::NEG_INFINITY, f64::max)
                        }
                    };
                    let (root, prune_bound) = (rt(cvo), rt(cv));
                    // (the visiting order stays by `root`; a configuration whose root at its class cap fails is left out)
                    if !best
                        && !cap_free
                        && rc < rho
                        && (0..clauses.len()).all(|ci| rc * cv[ci] * (1.0 + 1e-9) < clauses[ci].threshold)
                    {
                        continue;
                    }
                    configurations.push(SearchConfig {
                        root,
                        code,
                        real_filler_count: real22,
                        pal_entry_count: np,
                        filler_count: filler,
                        plain_pal_count: plain,
                        groups: groups.to_vec(),
                        group_misses: std::rc::Rc::clone(&group_misses),
                        ratio_prefixes: std::rc::Rc::clone(ratio_prefixes),
                        prune_bound,
                        clause_bounds: if check_ratio_bounds { cv.to_vec() } else { vec![] },
                    });
                }
            }
        }
        // stable: equal bounds keep enumeration order
        configurations.sort_by(|a, b| b.root.partial_cmp(&a.root).unwrap());
        ConfigurationQueue { configurations, pending, key_ratios }
    }
}
