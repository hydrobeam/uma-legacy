//! The depth-first walk of decks in one configuration.
use crate::checks::{CFCHK, RBCHK, SBCHK, SBN};
use crate::deck::caps::cap_class;
use crate::deck::enumerate::bounds::DOM_GAP;
use crate::deck::enumerate::config::SearchConfig;
use crate::deck::enumerate::order::DeckPath;
use crate::deck::enumerate::scratch::NodeScratch;
use crate::deck::enumerate::traverse::{
    BorrowTables, DeckSearchContext, ParentRatioBounds, PreparedTables, SearchControl, SearchResults,
};
use crate::deck::joint::{JointQuery, joint_fast};
use crate::deck::sources::{SourceEntry, TargetSources};
use crate::deck::space::{Dominator, SelectedDeck, TraversalEntryGroup, gm_g, gm_w};
use crate::deck::state_bound::StateBound;
/// Read-only tables for one configuration, borrowed separately from the node workspace.
pub(crate) struct ConfigurationContext<'walk, 'run, 'space, V> {
    pub(crate) search: &'walk DeckSearchContext<'run, 'space, V>,
    pub(crate) configuration: &'walk SearchConfig,
    pub(crate) filler_count: usize,
    pub(crate) plain_pal_count: usize,
    /// Configuration groups in pick order: eligible entry indices and required slot count.
    pub(crate) groups: &'walk [TraversalEntryGroup<'run>],
    /// Per group, white/gold miss-product rows indexed by gm_w/gm_g.
    pub(crate) group_misses: &'walk [&'walk [f64]],
    /// Per group, column-major ratio prefixes with deck_size + 1 entries per column.
    pub(crate) group_ratios: &'walk [&'walk [f64]],
    /// Entry-major probabilities, one target_count-wide row per entry.
    pub(crate) white_misses: &'walk [f64],
    /// Same entry/target indexing as white_misses.
    pub(crate) gold_misses: &'walk [f64],
    /// Per-entry target masks; untouched targets have miss probability exactly one.
    pub(crate) touched_targets: &'walk [u32],
    /// Target-major blocks for each remaining group suffix.
    pub(crate) suffix_white: &'walk [f64],
    /// Gold misses in the same suffix layout as suffix_white.
    pub(crate) suffix_gold: &'walk [f64],
    /// Per group suffix, one product per ratio column.
    pub(crate) suffix_ratios: &'walk [f64],
    /// Per group and candidate position, earlier dominator positions and their gold-gap masks.
    pub(crate) dominators: &'walk [std::rc::Rc<Vec<Vec<Dominator>>>],
    /// Flat entry offsets for each group in dominance_suffix.
    pub(crate) dominance_offsets: &'walk [usize],
    /// Characters available from each flat entry position onward.
    pub(crate) dominance_suffix: &'walk [u128],
    pub(crate) borrow_bounds: &'walk BorrowTables,
    /// OR of class masks from each group/position onward.
    pub(crate) class_suffix: &'walk [u64],
    /// Flat group starts in class_suffix.
    pub(crate) group_offsets: &'walk [usize],
    /// Per group/start, lock bits still available in that suffix.
    pub(crate) available_locks: &'walk [Vec<u8>],
    /// Group starts for rows indexed by remaining slots and borrowing state.
    pub(crate) remaining_offsets: &'walk [usize],
    /// One without borrowing, two when owned/one-borrow bounds are distinguished.
    pub(crate) borrow_state_count: usize,
    pub(crate) group_count: usize,
    pub(crate) total_slots: usize,
    /// State-conditioned bound retained for diagnostics.
    pub(crate) root_bound: f64,
}

/// Reused node buffers, state-bound caches and diagnostics for the current walk.
pub(crate) struct WalkScratch<'walk> {
    pub(crate) nodes: &'walk mut NodeScratch,
    pub(crate) state_bound: StateBound<'walk>,
    pub(crate) depth_one_bound: f64,
    pub(crate) bound_path: Vec<f64>,
    pub(crate) limit_reached: bool,
    pub(crate) white_chances: [f64; 32],
    pub(crate) gold_chances: [f64; 32],
}

/// Position and legality constraints of a partial deck. Masks use the prepared entry indexing.
#[derive(Clone, Copy)]
pub(crate) struct DeckCursor {
    /// Current configuration group and first entry position still eligible in it.
    pub(crate) group: usize,
    pub(crate) start: usize,
    /// Unfilled slots in this group.
    pub(crate) remaining: usize,
}

/// Legality masks change only when a card is chosen. Borrow these across group transitions.
#[derive(Clone, Copy)]
pub(crate) struct DeckConstraints {
    /// Characters already selected; no character may occupy two slots.
    pub(crate) characters: u128,
    pub(crate) borrowed_count: usize,
    /// Locked cards already represented in the path.
    pub(crate) locked_cards: u8,
    pub(crate) class_mask: u64,
    /// Dominance pruning requires these characters in any surviving completion.
    pub(crate) required_characters: u128,
}

/// Cache identity and the targets changed by this child.
#[derive(Clone, Copy)]
struct NodeIdentity {
    parent_depth: usize,
    parent_id: usize,
    touched_targets: u32,
}

impl<V: Fn(&crate::deck::enumerate::SearchClause, &[f64], &[f64]) -> f64> ConfigurationContext<'_, '_, '_, V> {
    /// Walk completions in group/entry order. A true result means the output limit stopped the search.
    /// The context's hot slices are copied here so mutable node writes cannot force their repeated reload.
    pub(crate) fn walk(
        &self,
        scratch: &mut WalkScratch<'_>,
        results: &mut SearchResults,
        path: &mut DeckPath,
        cursor: DeckCursor,
        constraints: &DeckConstraints,
    ) -> bool {
        let (space, target_count, groups) = (self.search.space, self.search.controls.target_count, self.groups);
        let PreparedTables { character_bits, lock_bits, all_lock_bits: all_locked, .. } = self.search.tables;
        let (available_locks, white_misses, gold_misses, touched_targets) =
            (self.available_locks, self.white_misses, self.gold_misses, self.touched_targets);
        if cursor.remaining == 0 {
            if cursor.group + 1 >= groups.len() {
                if constraints.required_characters & !constraints.characters != 0 {
                    return false;
                }
                let row_offset = path.entries.len() * target_count;
                return self.leaf(
                    scratch,
                    results,
                    &path.entries,
                    &path.white_misses[row_offset..row_offset + target_count],
                    &path.gold_misses[row_offset..row_offset + target_count],
                    constraints.class_mask,
                );
            }
            return self.walk(
                scratch,
                results,
                path,
                DeckCursor { group: cursor.group + 1, start: 0, remaining: groups[cursor.group + 1].slots },
                constraints,
            );
        }
        if all_locked & !constraints.locked_cards & !available_locks[cursor.group][cursor.start] != 0 {
            return false;
        }
        let list = groups[cursor.group].entries;
        // The deck so far is row path.entries.len(); a child's row is the next one (overwritten per child).
        let row_offset = path.entries.len() * target_count;
        // this parent's id and depth (the per-parent rows of `hopeful`)
        path.parent_id += 1;
        let (parent_id, parent_depth) = (path.parent_id, path.entries.len());
        for p in cursor.start..list.len() {
            if list.len() - p < cursor.remaining {
                break;
            }
            let j = list[p];
            if constraints.characters & character_bits[j] != 0
                || (space.borrowed[j] && constraints.borrowed_count >= space.allowed_borrow)
            {
                continue;
            }
            let Some(required_characters) = self.required_after_dominance(
                scratch,
                cursor,
                constraints,
                p,
                &path.gold_misses[row_offset..row_offset + target_count],
                &path.entries,
            ) else {
                continue;
            };
            space.nodes.set(space.nodes.get() + 1);
            // the child's row (depth + 1): the parent's times the card's misses
            {
                let (parent, child) = path.white_misses.split_at_mut(row_offset + target_count);
                let (entry_misses, parent, child) = (
                    &white_misses[j * target_count..(j + 1) * target_count],
                    &parent[row_offset..],
                    &mut child[..target_count],
                );
                for t in 0..target_count {
                    child[t] = parent[t] * entry_misses[t];
                }
            }
            {
                let (parent, child) = path.gold_misses.split_at_mut(row_offset + target_count);
                let (entry_misses, parent, child) = (
                    &gold_misses[j * target_count..(j + 1) * target_count],
                    &parent[row_offset..],
                    &mut child[..target_count],
                );
                for t in 0..target_count {
                    child[t] = parent[t] * entry_misses[t];
                }
            }
            let child_class = constraints.class_mask | space.class_masks[j];
            let child_constraints = DeckConstraints {
                borrowed_count: constraints.borrowed_count + usize::from(space.borrowed[j]),
                class_mask: child_class,
                ..*constraints
            };
            if !self.hopeful(
                scratch,
                results,
                DeckCursor { remaining: cursor.remaining - 1, ..cursor },
                &child_constraints,
                p,
                path,
                row_offset,
                NodeIdentity { parent_depth, parent_id, touched_targets: touched_targets[j] },
            ) {
                continue;
            }
            path.entries.push(j);
            if !self.passes_state_bound(scratch, &path.entries) {
                path.entries.pop();
                continue;
            }
            let stop = self.walk(
                scratch,
                results,
                path,
                DeckCursor { group: cursor.group, start: p + 1, remaining: cursor.remaining - 1 },
                &DeckConstraints {
                    characters: constraints.characters | character_bits[j],
                    locked_cards: constraints.locked_cards | lock_bits[j],
                    required_characters,
                    ..child_constraints
                },
            );
            path.entries.pop();
            if stop {
                return true;
            }
        }
        false
    }

    /// Skip a candidate dominated by an earlier eligible entry, preserving the first deck on ties.
    /// A surviving completion must contain a dominator's character whenever that character can still follow.
    /// Gold-miss products are initialized lazily per touched target, tracked by the per-call bit mask.
    fn required_after_dominance(
        &self,
        scratch: &mut WalkScratch<'_>,
        cursor: DeckCursor,
        constraints: &DeckConstraints,
        position: usize,
        parent_gold: &[f64],
        selection: &[usize],
    ) -> Option<u128> {
        let SearchControl { use_dominance, target_count, .. } = self.search.controls;
        if !use_dominance {
            return Some(constraints.required_characters);
        }
        let (groups, dominance_offsets, dominators, dominance_suffix) =
            (self.groups, self.dominance_offsets, self.dominators, self.dominance_suffix);
        let (character_bits, touched_targets, gold_misses) =
            (self.search.tables.character_bits, self.touched_targets, self.gold_misses);
        let entries = groups[cursor.group].entries;
        let candidate_entry = entries[position];
        let flat_position = dominance_offsets[cursor.group] + position;
        let mut required_characters = constraints.required_characters;
        let earlier_dominators = &dominators[cursor.group][position];
        if !earlier_dominators.is_empty() {
            // (the deck's picks in group cursor.group so far, all at positions below `cursor.start`)
            let group_picks = &selection[selection.len() - (groups[cursor.group].slots - cursor.remaining)..];
            let (gold_products, mut computed_targets) = (&mut scratch.nodes.dominance_products, 0u32);
            for &Dominator { position: a, gold_gap_targets: gap_targets } in earlier_dominators {
                let a = a as usize;
                let dominator_entry = entries[a];
                if constraints.characters & character_bits[dominator_entry] != 0 {
                    continue;
                }
                if a < cursor.start
                    && group_picks
                        .iter()
                        .any(|&j| j > dominator_entry && touched_targets[j] & touched_targets[candidate_entry] != 0)
                {
                    continue;
                }
                if gap_targets != 0 {
                    let mut has_gold_gap = true;
                    let mut remaining_targets = gap_targets;
                    while remaining_targets != 0 {
                        let t = remaining_targets.trailing_zeros() as usize;
                        remaining_targets &= remaining_targets - 1;
                        if computed_targets >> t & 1 == 0 {
                            let remaining_gold_miss =
                                gm_g(self.group_misses[cursor.group], target_count, cursor.remaining - 1)[t]
                                    * self.suffix_gold[(cursor.group + 1) * target_count + t];
                            gold_products[t] = parent_gold[t] * remaining_gold_miss;
                            computed_targets |= 1 << t;
                        }
                        if !(gold_products[t]
                            * (gold_misses[candidate_entry * target_count + t]
                                - gold_misses[dominator_entry * target_count + t])
                            >= DOM_GAP)
                        {
                            has_gold_gap = false;
                            break;
                        }
                    }
                    if !has_gold_gap {
                        continue;
                    }
                }
                if character_bits[dominator_entry] != character_bits[candidate_entry]
                    && character_bits[dominator_entry] & dominance_suffix[flat_position + 1] != 0
                {
                    required_characters |= character_bits[dominator_entry];
                    continue;
                }
                return None;
            }
        }
        // a needed character that no later pick can bring
        if required_characters
            & !(constraints.characters | character_bits[candidate_entry])
            & !dominance_suffix[flat_position + 1]
            != 0
        {
            return None;
        }
        Some(required_characters)
    }

    /// Test the child's target bound first, then its ratio bound. Parent rows are cached by node identity;
    /// only the selected card's touched targets differ from those rows.
    fn hopeful(
        &self,
        scratch: &mut WalkScratch<'_>,
        results: &mut SearchResults,
        cursor: DeckCursor,
        constraints: &DeckConstraints,
        position: usize,
        path: &DeckPath,
        row_offset: usize,
        node: NodeIdentity,
    ) -> bool {
        // (the cap-free band prunes by the state-conditioned bound alone, `passes_state_bound`)
        if self.search.controls.cap_free {
            return true;
        }
        // Phase 1: choose the completion class cap and remaining-slot row; cap-free search bypasses both.
        // Read the bound tables after the cap-free early return, where this bound is first needed.
        let search = self.search;
        let target_count = search.controls.target_count;
        let nodes = &mut *scratch.nodes;
        // The child and every entry still eligible contribute to the highest class any completion can reach.
        let rho = if search.controls.best {
            search.controls.rho
        } else {
            let (class_suffix, group_offsets) = (self.class_suffix, self.group_offsets);
            search.controls.class_caps[cap_class(
                constraints.class_mask
                    | if cursor.remaining > 0 {
                        class_suffix[group_offsets[cursor.group] + position + 1]
                    } else {
                        class_suffix[group_offsets[cursor.group + 1]]
                    },
            )]
        };
        // borrowed cards still allowed (only with the one-borrowed tables)
        let can_borrow = constraints.borrowed_count < 1;
        let mut cached_target_bounds = false;
        let has_borrowed_entries = self.borrow_bounds.has_borrowed_entries;
        let row = if cursor.group >= self.group_count {
            usize::MAX
        } else {
            (self.remaining_offsets[cursor.group] + cursor.remaining) * self.borrow_state_count
                + usize::from(has_borrowed_entries && can_borrow)
        };
        let parent_white = &path.white_misses[row_offset..row_offset + target_count];
        let parent_gold = &path.gold_misses[row_offset..row_offset + target_count];
        let (white_misses, gold_misses) = (
            &path.white_misses[row_offset + target_count..row_offset + 2 * target_count],
            &path.gold_misses[row_offset + target_count..row_offset + 2 * target_count],
        );
        // Phase 2: fill child target bounds from remaining miss products. Parent factors are reusable
        // only for untouched targets and the same borrowing variant; products retain target order.
        if row == usize::MAX {
            for t in 0..target_count {
                nodes.white_chances[t] = 1.0 - white_misses[t] * 1.0;
                nodes.gold_chances[t] = 1.0 - gold_misses[t] * 1.0;
                nodes.partial_white[t] = 1.0 - white_misses[t];
                nodes.partial_gold[t] = 1.0 - gold_misses[t];
            }
        } else {
            if !nodes.has_target_constants[row] {
                nodes.has_target_constants[row] = true;
                let (group_misses, suffix_white, suffix_gold) =
                    (self.group_misses, self.suffix_white, self.suffix_gold);
                let BorrowTables { group_bounds, owned_suffix, borrow_suffix, .. } = self.borrow_bounds;
                for t in 0..target_count {
                    let (a, b) = if !has_borrowed_entries {
                        (
                            gm_w(group_misses[cursor.group], target_count, cursor.remaining)[t]
                                * suffix_white[(cursor.group + 1) * target_count + t],
                            gm_g(group_misses[cursor.group], target_count, cursor.remaining)[t]
                                * suffix_gold[(cursor.group + 1) * target_count + t],
                        )
                    } else if can_borrow {
                        let group_bound = &group_bounds[cursor.group];
                        (
                            (group_bound.borrow_misses[cursor.remaining].white[t]
                                * owned_suffix[cursor.group + 1].white[t])
                                .min(
                                    group_bound.owned_misses[cursor.remaining].white[t]
                                        * borrow_suffix[cursor.group + 1].white[t],
                                ),
                            (group_bound.borrow_misses[cursor.remaining].gold[t]
                                * owned_suffix[cursor.group + 1].gold[t])
                                .min(
                                    group_bound.owned_misses[cursor.remaining].gold[t]
                                        * borrow_suffix[cursor.group + 1].gold[t],
                                ),
                        )
                    } else {
                        (
                            group_bounds[cursor.group].owned_misses[cursor.remaining].white[t]
                                * owned_suffix[cursor.group + 1].white[t],
                            group_bounds[cursor.group].owned_misses[cursor.remaining].gold[t]
                                * owned_suffix[cursor.group + 1].gold[t],
                        )
                    };
                    nodes.target_constants[row * 2 * target_count + t] = a;
                    nodes.target_constants[row * 2 * target_count + target_count + t] = b;
                }
            }
            let (remaining_white, remaining_gold) =
                nodes.target_constants[row * 2 * target_count..(row + 1) * 2 * target_count].split_at(target_count);
            if search.cache_parent_rows {
                let (outfit_row_count, outfit_factors, inheritance_chances) =
                    (search.outfit_row_count, search.tables.outfits, &*search.inheritance_chances);
                // A child is its parent plus one card: for a target the card does not touch (its miss
                // chances exactly 1.0) the child's white miss is the parent's times 1.0: equal bit for
                // bit, and so are nodes.white_chances[t], nodes.gold_chances[t] and every outfit's A factor of t (the same expressions on
                // the same f64s; remaining miss products depend only on the row, the same for all children of one parent
                // and borrowing variant). So the factors are computed once per parent (here, at its first
                // child), a child recomputes only its touched targets', and A is the product in target
                // order (`OutfitPre::eval`'s). The factors before the first touched target are the
                // parent's, and so is their product from 1.0 (`prefix_products`, kept per parent): a child's product
                // starts there and multiplies on from that target, the same multiplications in the same order
                let parent_targets =
                    &mut nodes.parent_targets[node.parent_depth * 2 + usize::from(has_borrowed_entries && can_borrow)];
                if parent_targets.parent_id != node.parent_id {
                    parent_targets.parent_id = node.parent_id;
                    for t in 0..target_count {
                        nodes.parent_white[t] = 1.0 - parent_white[t] * remaining_white[t];
                        nodes.parent_gold[t] = 1.0 - parent_gold[t] * remaining_gold[t];
                    }
                    for ix in 0..outfit_row_count {
                        outfit_factors[ix].factors(
                            inheritance_chances[ix],
                            &nodes.parent_white[..target_count],
                            &nodes.parent_gold[..target_count],
                            &mut parent_targets.factors[ix * target_count..(ix + 1) * target_count],
                        );
                    }
                    for (row, prefix_products) in parent_targets
                        .factors
                        .chunks_exact(target_count)
                        .zip(parent_targets.prefix_products.chunks_exact_mut(target_count + 1))
                    {
                        for t in 0..target_count {
                            prefix_products[t + 1] = prefix_products[t] * row[t];
                        }
                    }
                }
                let mut remaining_targets = node.touched_targets;
                while remaining_targets != 0 {
                    let t = remaining_targets.trailing_zeros() as usize;
                    remaining_targets &= remaining_targets - 1;
                    nodes.touched_white[t] = 1.0 - white_misses[t] * remaining_white[t];
                    nodes.touched_gold[t] = 1.0 - gold_misses[t] * remaining_gold[t];
                }
                let first_touched = if node.touched_targets == 0 {
                    target_count
                } else {
                    node.touched_targets.trailing_zeros() as usize
                };
                for (ix, (row, prefix_products)) in parent_targets
                    .factors
                    .chunks_exact_mut(target_count)
                    .zip(parent_targets.prefix_products.chunks_exact(target_count + 1))
                    .enumerate()
                {
                    let (outfit, q) = (&outfit_factors[ix], inheritance_chances[ix]);
                    let mut remaining_targets = node.touched_targets;
                    while remaining_targets != 0 {
                        let t = remaining_targets.trailing_zeros() as usize;
                        remaining_targets &= remaining_targets - 1;
                        nodes.previous_factors[t] = row[t];
                        row[t] = outfit.factor_at(t, q, nodes.touched_white[t], nodes.touched_gold[t]);
                    }
                    let mut product = prefix_products[first_touched];
                    for &factor in &row[first_touched..] {
                        product *= factor;
                    }
                    let mut remaining_targets = node.touched_targets;
                    while remaining_targets != 0 {
                        let t = remaining_targets.trailing_zeros() as usize;
                        remaining_targets &= remaining_targets - 1;
                        row[t] = nodes.previous_factors[t];
                    }
                    nodes.outfit_bounds[ix] = product;
                }
                // B's partial-deck chances are filled below, only when B is evaluated.
                cached_target_bounds = true;
            } else {
                for t in 0..target_count {
                    nodes.white_chances[t] = 1.0 - white_misses[t] * remaining_white[t];
                    nodes.gold_chances[t] = 1.0 - gold_misses[t] * remaining_gold[t];
                    nodes.partial_white[t] = 1.0 - white_misses[t];
                    nodes.partial_gold[t] = 1.0 - gold_misses[t];
                }
            }
        }
        // (bound_check_scale: 1, or RB_CHECK's s; x * 1.0 is x)
        let (best, best_value, clauses, bound_check_scale) =
            (search.controls.best, results.best_value, search.clauses, search.controls.bound_check_scale);
        let pass = |clause_values: &[f64]| {
            if best {
                clause_values[0] * bound_check_scale * (1.0 + 1e-9) >= best_value
            } else {
                (0..clauses.len()).any(|clause_index| {
                    rho * clause_values[clause_index] * bound_check_scale * (1.0 + 1e-9)
                        >= clauses[clause_index].threshold
                })
            }
        };
        // Phase 3: reject on the target-only bound before paying for ratio factors.
        // the per-target values A first (each outfit's term is min(A, B r) <= A, so the clause sums with
        // A alone are upper bounds of the real ones: when they fail, so does the real test; A is finite,
        // its inputs being probabilities, and a NaN falls through to the real test)
        nodes.clause_values.fill(0.0);
        let PreparedTables { outfit_rows, outfit_runs, outfits, .. } = search.tables;
        let fixed = search.controls.fixed;
        let mut has_nan = false;
        for &crate::deck::enumerate::bounds::OutfitRun { clause: clause_index, start: i, count: outfit_count } in
            outfit_runs
        {
            let q = outfit_rows[i].inheritance;
            let mut outfit_best = 0.0f64;
            for offset in 0..outfit_count {
                let a = if cached_target_bounds {
                    nodes.outfit_bounds[i + offset]
                } else {
                    outfits[i + offset].eval(q, &nodes.white_chances, &nodes.gold_chances)
                };
                nodes.outfit_bounds[i + offset] = a;
                has_nan |= a.is_nan();
                outfit_best = outfit_best.max(a);
            }
            nodes.clause_values[clause_index] += fixed * outfit_best;
        }
        if !has_nan && !pass(&nodes.clause_values) {
            return false;
        }
        // Phase 4: prepare remaining-card ratio ceilings and reuse the parent's untouched factors.
        let column_count = search.controls.column_count;
        if row != usize::MAX && !nodes.has_ratio_constants[row] {
            nodes.has_ratio_constants[row] = true;
            let (size, group_ratios, suffix_ratios) = (search.controls.size, self.group_ratios, self.suffix_ratios);
            let BorrowTables { group_bounds, owned_ratio_suffix, borrow_ratio_suffix, .. } = self.borrow_bounds;
            for ix in 0..column_count {
                nodes.ratio_constants[row * column_count + ix] = if !has_borrowed_entries {
                    group_ratios[cursor.group][ix * (size + 1) + cursor.remaining]
                        * suffix_ratios[(cursor.group + 1) * column_count + ix]
                } else if can_borrow {
                    (group_bounds[cursor.group].borrow_ratios[ix][cursor.remaining]
                        * owned_ratio_suffix[cursor.group + 1][ix])
                        .max(
                            group_bounds[cursor.group].owned_ratios[ix][cursor.remaining]
                                * borrow_ratio_suffix[cursor.group + 1][ix],
                        )
                } else {
                    group_bounds[cursor.group].owned_ratios[ix][cursor.remaining]
                        * owned_ratio_suffix[cursor.group + 1][ix]
                };
            }
        }
        // B's factors are at one minus the child's misses. With the per-parent rows (cached target bounds) they are
        // the parent's at targets the card does not touch (the white miss is the parent's, bit for bit, as
        // for A): per row, the parent's factors (`ParentRatioBounds`, built at the first child that evaluates the row)
        // with the touched targets' recomputed, and B the product over its targets in target
        // order from 1.0 (`eval`'s and `eval_in`'s), the full one from the parent's prefix at the first touched target.
        // Otherwise use the partial-deck chances filled above.
        let first_touched =
            if node.touched_targets == 0 { target_count } else { node.touched_targets.trailing_zeros() as usize };
        if cached_target_bounds {
            let parent_ratios = &mut nodes.parent_ratios[node.parent_depth];
            if parent_ratios.parent_id != node.parent_id {
                parent_ratios.parent_id = node.parent_id;
                for t in 0..target_count {
                    parent_ratios.white_chances[t] = 1.0 - parent_white[t];
                    parent_ratios.gold_chances[t] = 1.0 - parent_gold[t];
                }
            }
            let mut remaining_targets = node.touched_targets;
            while remaining_targets != 0 {
                let t = remaining_targets.trailing_zeros() as usize;
                remaining_targets &= remaining_targets - 1;
                nodes.ratio_white[t] = 1.0 - white_misses[t];
                nodes.ratio_gold[t] = 1.0 - gold_misses[t];
            }
        }
        // (with cached target bounds: row ix's B factors into `nodes.ratio_factors`, the parent's row with the touched ones recomputed;
        // with `full`, their product, else 0)
        let inheritance_chances = &*search.inheritance_chances;
        let mut fill_ratio_factors = |ix: usize, full: bool, ratio_factors: &mut [f64; 32]| -> f64 {
            let (outfit, q) = (&outfits[ix], inheritance_chances[ix]);
            let ParentRatioBounds {
                row_stamps,
                white_chances: parent_white_chances,
                gold_chances: parent_gold_chances,
                factors,
                prefix_products,
                ..
            } = &mut nodes.parent_ratios[node.parent_depth];
            let (parent_factors, prefix_products) = (
                &mut factors[ix * target_count..(ix + 1) * target_count],
                &mut prefix_products[ix * (target_count + 1)..(ix + 1) * (target_count + 1)],
            );
            if row_stamps[ix] != node.parent_id {
                row_stamps[ix] = node.parent_id;
                outfit.factors(
                    q,
                    &parent_white_chances[..target_count],
                    &parent_gold_chances[..target_count],
                    parent_factors,
                );
                for t in 0..target_count {
                    prefix_products[t + 1] = prefix_products[t] * parent_factors[t];
                }
            }
            ratio_factors[..target_count].copy_from_slice(parent_factors);
            let mut remaining_targets = node.touched_targets;
            while remaining_targets != 0 {
                let t = remaining_targets.trailing_zeros() as usize;
                remaining_targets &= remaining_targets - 1;
                ratio_factors[t] = outfit.factor_at(t, q, nodes.ratio_white[t], nodes.ratio_gold[t]);
            }
            if !full {
                return 0.0;
            }
            let mut product = prefix_products[first_touched];
            for &factor in &ratio_factors[first_touched..target_count] {
                product *= factor;
            }
            product
        };
        // Product over the selected targets, in the same order as OutfitPre::eval_in.
        let masked_product = |ratio_factors: &[f64; 32], target_mask: u32| -> f64 {
            let (mut target_mask, mut product) = (target_mask, 1.0f64);
            while target_mask != 0 {
                let t = target_mask.trailing_zeros() as usize;
                target_mask &= target_mask - 1;
                product *= ratio_factors[t];
            }
            product
        };
        // Phase 5: combine target and ratio bounds per split, then test the clause thresholds.
        // the real bound, the same expression in the same order; an outfit whose A is at most the running
        // maximum leaves it unchanged (min(A, B r) <= A <= m) and is skipped
        nodes.clause_values.fill(0.0);
        {
            let PreparedTables { zero_masks, nonzero_masks, alternative_columns, .. } = search.tables;
            // (ZSPLIT; a row without a split, zero mask 0 and no low split, takes min(A, B r)) A's product over a
            // split's Z: from the per-parent row with the child's touched targets recomputed (A's own
            // factors), or at the node's odds
            let parent_row = if cached_target_bounds {
                node.parent_depth * 2 + usize::from(has_borrowed_entries && can_borrow)
            } else {
                usize::MAX
            };
            let a_z = |ix: usize, target_mask: u32| -> f64 {
                let (outfit, q) = (&outfits[ix], inheritance_chances[ix]);
                if parent_row == usize::MAX {
                    return outfit.eval_in(q, &nodes.white_chances, &nodes.gold_chances, target_mask);
                }
                let factors = &nodes.parent_targets[parent_row].factors[ix * target_count..(ix + 1) * target_count];
                let (mut target_mask, mut product) = (target_mask, 1.0f64);
                while target_mask != 0 {
                    let t = target_mask.trailing_zeros() as usize;
                    target_mask &= target_mask - 1;
                    product *= if node.touched_targets >> t & 1 != 0 {
                        outfit.factor_at(t, q, nodes.touched_white[t], nodes.touched_gold[t])
                    } else {
                        factors[t]
                    };
                }
                product
            };
            for &crate::deck::enumerate::bounds::OutfitRun { clause: clause_index, start: i, count: outfit_count } in
                outfit_runs
            {
                let q = outfit_rows[i].inheritance;
                let mut outfit_best = 0.0f64;
                for offset in 0..outfit_count {
                    let ix = i + offset;
                    let a = nodes.outfit_bounds[ix];
                    if a <= outfit_best {
                        continue;
                    }
                    let r = if row == usize::MAX { 1.0 } else { nodes.ratio_constants[row * column_count + ix] };
                    // A's factors of the zero targets times N's ratio bound; a low split's, the lower
                    let mut b = if cached_target_bounds {
                        let full = fill_ratio_factors(ix, zero_masks[ix] == 0, &mut nodes.ratio_factors);
                        if zero_masks[ix] != 0 {
                            a_z(ix, zero_masks[ix]) * (masked_product(&nodes.ratio_factors, nonzero_masks[ix]) * r)
                        } else {
                            full * r
                        }
                    } else if zero_masks[ix] != 0 {
                        a_z(ix, zero_masks[ix])
                            * (outfits[ix].eval_in(q, &nodes.partial_white, &nodes.partial_gold, nonzero_masks[ix]) * r)
                    } else {
                        outfits[ix].eval(q, &nodes.partial_white, &nodes.partial_gold) * r
                    };
                    for column in alternative_columns[ix].start..alternative_columns[ix].end {
                        let r =
                            if row == usize::MAX { 1.0 } else { nodes.ratio_constants[row * column_count + column] };
                        let nonzero_product = if cached_target_bounds {
                            masked_product(&nodes.ratio_factors, nonzero_masks[column])
                        } else {
                            outfits[ix].eval_in(q, &nodes.partial_white, &nodes.partial_gold, nonzero_masks[column])
                        };
                        b = b.min(a_z(ix, zero_masks[column]) * (nonzero_product * r));
                    }
                    outfit_best = outfit_best.max(a.min(b));
                }
                nodes.clause_values[clause_index] += fixed * outfit_best;
            }
        }
        let passes_bound = pass(&nodes.clause_values);
        // Phase 6: retain passing bounds for the leaf diagnostic, without changing pruning.
        if search.controls.check_ratio_bounds && passes_bound {
            let diagnostic_bounds = &mut nodes.diagnostic_bounds;
            if diagnostic_bounds.len() < node.parent_depth + 2 {
                diagnostic_bounds.resize(node.parent_depth + 2, vec![]);
            }
            diagnostic_bounds[node.parent_depth + 1].clear();
            diagnostic_bounds[node.parent_depth + 1].extend_from_slice(&nodes.clause_values);
        }
        passes_bound
    }

    /// The state-conditioned bound at depth <= 1 (mode 1) or at every depth, the leaf included (mode 2); with
    /// SB_CHECK, every leaf's value against its root's and depth-1 node's bounds. False: the node is pruned
    fn passes_state_bound(&self, scratch: &mut WalkScratch<'_>, selection: &[usize]) -> bool {
        let search = self.search;
        if search.controls.cap_free {
            let SearchControl {
                target_count,
                cap_free_node_limit,
                cap_free_check_scale,
                state_bound_threshold,
                bound_trainee,
                bound_inheritance,
                ..
            } = search.controls;
            let (space, chances, total_slots) = (search.space, search.chances, self.total_slots);
            // the cap-free band: the bound at every node and leaf, nothing else prunes; past the node
            // limit every child fails and the search ends with CapFreeNodeLimit
            if scratch.limit_reached {
                return false;
            }
            if space.nodes.get() > cap_free_node_limit {
                scratch.limit_reached = true;
                return false;
            }
            let depth = selection.len();
            let is_leaf = depth == total_slots;
            let check_scale = if is_leaf { 1.0 } else { cap_free_check_scale.max(1.0) };
            // the cheap bound first, the state-conditioned one only where it passes
            let cheap_bound = scratch.state_bound.cheap(selection);
            // (CF_CHECK: both always, so the check below tests each: an exact value above either is above their minimum)
            let upper_bound =
                if cap_free_check_scale == 0.0 && cheap_bound * check_scale * (1.0 + 1e-9) < state_bound_threshold {
                    cheap_bound
                } else {
                    cheap_bound.min(scratch.state_bound.full(
                        selection,
                        if cap_free_check_scale == 0.0 { state_bound_threshold / check_scale } else { 0.0 },
                    ))
                };
            SBN.with(|cell| cell.borrow_mut().evaluations += 1);
            if cap_free_check_scale > 0.0 {
                // CF_CHECK: a leaf's exact value against its own bound and every node's above it
                let path = &mut scratch.bound_path;
                path.truncate(depth);
                path.push(upper_bound);
                if is_leaf {
                    let entries: Vec<SourceEntry<&TargetSources>> = selection
                        .iter()
                        .map(|&j| SourceEntry { sources: &space.sources[j], kind: space.kind[j] })
                        .collect();
                    let leaf_value = joint_fast(
                        chances.model,
                        JointQuery {
                            chain_events: space.deck_job.chain_events(),
                            checks: crate::checks::get(),
                            trainee: bound_trainee,
                            inheritance_chances: &[bound_inheritance],
                            event_chances: &chances.event_chances[bound_trainee],
                        },
                        &entries,
                        self.filler_count,
                        target_count + chances.model.n_fixed_gold as usize,
                        false,
                        None,
                    )[0]
                    .all;
                    CFCHK.with(|cell| {
                        let mut counters = cell.borrow_mut();
                        counters.leaves += 1;
                        for &bound in path.iter() {
                            if leaf_value > bound * (1.0 + 1e-9) {
                                counters.leaf_violations += 1;
                            }
                            if bound > 0.0 {
                                counters.max_ratio = counters.max_ratio.max(leaf_value / bound);
                            }
                        }
                    });
                }
            }
            let pass = upper_bound * check_scale * (1.0 + 1e-9) >= state_bound_threshold;
            if !pass {
                SBN.with(|cell| cell.borrow_mut().nodes_cut += 1);
            }
            return pass;
        }
        let state_bound_mode = search.controls.state_bound_mode;
        if state_bound_mode == 0 {
            return true;
        }
        let check_bounds = crate::checks::get().sb_check;
        if state_bound_mode == 3 && !check_bounds {
            return true;
        }
        let depth = selection.len();
        let is_leaf = depth == self.total_slots;
        if (state_bound_mode == 1 || state_bound_mode == 3) && depth > 1 && !(check_bounds && is_leaf) {
            return true;
        }
        if state_bound_mode == 3 && depth == 1 && !check_bounds {
            return true;
        }
        let upper_bound = scratch.state_bound.bound(selection);
        SBN.with(|cell| cell.borrow_mut().evaluations += 1);
        if depth == 1 {
            scratch.depth_one_bound = upper_bound;
        }
        if check_bounds && is_leaf {
            let (root_bound, depth_one_bound) = (self.root_bound, scratch.depth_one_bound);
            SBCHK.with(|cell| {
                let mut counters = cell.borrow_mut();
                counters.leaves += 1;
                if upper_bound > root_bound * (1.0 + 1e-9)
                    || (depth > 1 && upper_bound > depth_one_bound * (1.0 + 1e-9))
                {
                    counters.violations += 1;
                }
            });
        }
        if (state_bound_mode == 1 && depth > 1) || state_bound_mode == 3 {
            return true;
        }
        let pass = upper_bound * (1.0 + 1e-9) >= search.controls.state_bound_threshold;
        if !pass {
            SBN.with(|cell| cell.borrow_mut().nodes_cut += 1);
        }
        pass
    }

    /// A complete deck `selection` (its `white_misses`, `gold_misses`, `class_mask`): kept when it beats the bar (product
    /// search) or passes a clause at its class cap (band). True: the output is full, the search stops
    pub(crate) fn leaf(
        &self,
        scratch: &mut WalkScratch<'_>,
        results: &mut SearchResults,
        selection: &[usize],
        white_misses: &[f64],
        gold_misses: &[f64],
        class_mask: u64,
    ) -> bool {
        let SearchControl { best, cap, target_count, cap_free, class_caps, check_ratio_bounds, .. } =
            self.search.controls;
        let (space, clauses, clause_value) = (self.search.space, self.search.clauses, &self.search.clause_value);
        let (plain_pals, filler_cards) = (self.plain_pal_count, self.filler_count);
        let pal_count = selection.iter().filter(|&&j| space.cards[space.card_indices[j]].pal).count() + plain_pals;
        if let Some(required_pals) = space.deck_job.pals
            && required_pals != pal_count
        {
            return false;
        }
        if !space.locked.iter().all(|locked| selection.iter().any(|&j| space.card_indices[j] == *locked)) {
            return false;
        }
        for t in 0..target_count {
            scratch.white_chances[t] = 1.0 - white_misses[t];
            scratch.gold_chances[t] = 1.0 - gold_misses[t];
        }
        let (white_chances, gold_chances) =
            (&scratch.white_chances[..target_count], &scratch.gold_chances[..target_count]);
        // RB_CHECK: each clause's value against the bounds of the root and of every node above the deck
        if check_ratio_bounds {
            let (path_bounds, configuration) = (&scratch.nodes.diagnostic_bounds, self.configuration);
            RBCHK.with(|cell| {
                let mut counters = cell.borrow_mut();
                counters.leaves += 1;
                for (clause_index, clause) in clauses.iter().enumerate() {
                    if best && clause_index > 0 {
                        break;
                    }
                    let value = clause_value(clause, white_chances, gold_chances);
                    for depth in 0..=selection.len() {
                        let bound = if depth == 0 {
                            configuration.clause_bounds[clause_index]
                        } else {
                            path_bounds[depth][clause_index]
                        };
                        counters.tests += 1;
                        if value > bound * (1.0 + 1e-9) {
                            counters.violations += 1;
                        }
                        if bound > 0.0 {
                            counters.max_ratio = counters.max_ratio.max(value / bound);
                        }
                    }
                }
            });
        }
        let scored_deck = |value: f64| -> crate::deck::enumerate::ScoredDeck<SelectedDeck> {
            let mut entries = selection.to_vec();
            entries.sort_unstable();
            crate::deck::enumerate::ScoredDeck {
                value,
                deck: SelectedDeck {
                    entries,
                    plain_pals,
                    filler_cards,
                    white_hint_chances: white_chances.to_vec(),
                    gold_hint_chances: gold_chances.to_vec(),
                },
            }
        };
        if best {
            let value = clause_value(&clauses[0], white_chances, gold_chances);
            if value > results.best_value {
                results.best_value = value;
                results.decks.clear();
                results.decks.push(scored_deck(value));
            }
            false
        } else {
            // each clause's value once: the best of them, and whether any passes its bar at the deck's class cap
            let (mut best_clause_value, mut pass) = (f64::NEG_INFINITY, false);
            let rho = class_caps[cap_class(class_mask)];
            for clause in clauses {
                let value = clause_value(clause, white_chances, gold_chances);
                best_clause_value = best_clause_value.max(value);
                pass = pass || cap_free || rho * value * (1.0 + 1e-9) >= clause.threshold;
            }
            if pass {
                results.decks.push(scored_deck(best_clause_value));
                if results.decks.len() >= cap {
                    return true;
                }
            }
            false
        }
    }
}
