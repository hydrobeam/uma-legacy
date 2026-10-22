//! The depth-first walk of decks in one configuration.
use super::RBCHK;
use super::{
    BorrowTables, DOM_GAP, DeckPath, DeckSearchContext, NodeScratch, ParentRatioBounds, PreparedTables, SearchConfig,
    SearchControl, SearchResults,
};
use crate::deck::{CFCHK, Kind, SBCHK, SBN, Sel, Src, StateBound, cap_class, gm_g, gm_w, joint_fast, sb_check};

/// Read-only tables for one configuration, borrowed separately from the node workspace.
pub(crate) struct ConfigurationContext<'walk, 'run, 'space, V> {
    pub(crate) search: &'walk DeckSearchContext<'run, 'space, V>,
    pub(crate) configuration: &'walk SearchConfig,
    pub(crate) filler: usize,
    pub(crate) plain: usize,
    /// Configuration groups in pick order: eligible entry indices and required slot count.
    pub(crate) groups: &'walk [(&'run [usize], usize)],
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
    pub(crate) dominators: &'walk [std::rc::Rc<Vec<Vec<(u32, u32)>>>],
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

/// Parent and child miss products, in target order.
struct MissRows<'row> {
    parent_white: &'row [f64],
    parent_gold: &'row [f64],
    child_white: &'row [f64],
    child_gold: &'row [f64],
}

/// Cache identity and the targets changed by this child.
#[derive(Clone, Copy)]
struct NodeIdentity {
    parent_depth: usize,
    parent_id: usize,
    touched_targets: u32,
}

impl<V: Fn(&(Vec<(usize, f64)>, f64), &[f64], &[f64]) -> f64> ConfigurationContext<'_, '_, '_, V> {
    /// Walk completions in group/entry order. A true result means the output limit stopped the search.
    /// The context's hot slices are copied here so mutable node writes cannot force their repeated reload.
    pub(crate) fn rec(
        &self,
        scratch: &mut WalkScratch<'_>,
        results: &mut SearchResults,
        path: &mut DeckPath,
        cursor: DeckCursor,
        constraints: &DeckConstraints,
    ) -> bool {
        let (space, target_count, groups) = (self.search.space, self.search.controls.target_count, self.groups);
        let PreparedTables { ebit, lbit, all_lk: all_locked, .. } = self.search.tables;
        let (avail, white_misses, gold_misses, touched_targets) =
            (self.available_locks, self.white_misses, self.gold_misses, self.touched_targets);
        if cursor.remaining == 0 {
            if cursor.group + 1 >= groups.len() {
                if constraints.required_characters & !constraints.characters != 0 {
                    return false;
                }
                let row_offset = path.sel.len() * target_count;
                return self.leaf(
                    scratch,
                    results,
                    &path.sel,
                    &path.pw[row_offset..row_offset + target_count],
                    &path.pg[row_offset..row_offset + target_count],
                    constraints.class_mask,
                );
            }
            return self.rec(
                scratch,
                results,
                path,
                DeckCursor { group: cursor.group + 1, start: 0, remaining: groups[cursor.group + 1].1 },
                constraints,
            );
        }
        if all_locked & !constraints.locked_cards & !avail[cursor.group][cursor.start] != 0 {
            return false;
        }
        let list = groups[cursor.group].0;
        // the deck so far is row sel.len(); a child's row is the next one (overwritten per child)
        let row_offset = path.sel.len() * target_count;
        // this parent's id and depth (the per-parent rows of `hopeful`)
        path.parent_id += 1;
        let (parent_id, parent_depth) = (path.parent_id, path.sel.len());
        for p in cursor.start..list.len() {
            if list.len() - p < cursor.remaining {
                break;
            }
            let j = list[p];
            if constraints.characters & ebit[j] != 0
                || (space.ebor[j] && constraints.borrowed_count >= space.allowed_borrow)
            {
                continue;
            }
            let Some(required_characters) =
                self.dom(scratch, cursor, constraints, p, &path.pg[row_offset..row_offset + target_count], &path.sel)
            else {
                continue;
            };
            space.nodes.set(space.nodes.get() + 1);
            // the child's row (depth + 1): the parent's times the card's misses
            {
                let (parent, child) = path.pw.split_at_mut(row_offset + target_count);
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
                let (parent, child) = path.pg.split_at_mut(row_offset + target_count);
                let (entry_misses, parent, child) = (
                    &gold_misses[j * target_count..(j + 1) * target_count],
                    &parent[row_offset..],
                    &mut child[..target_count],
                );
                for t in 0..target_count {
                    child[t] = parent[t] * entry_misses[t];
                }
            }
            let child_class = constraints.class_mask | space.cvm[j];
            let child_constraints = DeckConstraints {
                borrowed_count: constraints.borrowed_count + usize::from(space.ebor[j]),
                class_mask: child_class,
                ..*constraints
            };
            if !self.hopeful(
                scratch,
                results,
                DeckCursor { remaining: cursor.remaining - 1, ..cursor },
                &child_constraints,
                p,
                MissRows {
                    child_white: &path.pw[row_offset + target_count..row_offset + 2 * target_count],
                    child_gold: &path.pg[row_offset + target_count..row_offset + 2 * target_count],
                    parent_white: &path.pw[row_offset..row_offset + target_count],
                    parent_gold: &path.pg[row_offset..row_offset + target_count],
                },
                NodeIdentity { parent_depth, parent_id, touched_targets: touched_targets[j] },
            ) {
                continue;
            }
            path.sel.push(j);
            if !self.sbx(scratch, &path.sel) {
                path.sel.pop();
                continue;
            }
            let stop = self.rec(
                scratch,
                results,
                path,
                DeckCursor { group: cursor.group, start: p + 1, remaining: cursor.remaining - 1 },
                &DeckConstraints {
                    characters: constraints.characters | ebit[j],
                    locked_cards: constraints.locked_cards | lbit[j],
                    required_characters,
                    ..child_constraints
                },
            );
            path.sel.pop();
            if stop {
                return true;
            }
        }
        false
    }

    /// Skip a candidate dominated by an earlier eligible entry, preserving the first deck on ties.
    /// A surviving completion must contain a dominator's character whenever that character can still follow.
    /// Gold-miss products are initialized lazily per touched target, tracked by the per-call bit mask.
    fn dom(
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
        let (groups, doff, doms, dsuf) = (self.groups, self.dominance_offsets, self.dominators, self.dominance_suffix);
        let (ebit, touched_targets, gold_misses) = (self.search.tables.ebit, self.touched_targets, self.gold_misses);
        let l = groups[cursor.group].0;
        let bj = l[position];
        let f = doff[cursor.group] + position;
        let mut nd = constraints.required_characters;
        let dl = &doms[cursor.group][position];
        if !dl.is_empty() {
            // (the deck's picks in group cursor.group so far, all at positions below `cursor.start`)
            let sg = &selection[selection.len() - (groups[cursor.group].1 - cursor.remaining)..];
            let (pr, mut prm) = (&mut scratch.nodes.dominance_products, 0u32);
            for &(a, sm) in dl {
                let a = a as usize;
                let aj = l[a];
                if constraints.characters & ebit[aj] != 0 {
                    continue;
                }
                if a < cursor.start && sg.iter().any(|&j| j > aj && touched_targets[j] & touched_targets[bj] != 0) {
                    continue;
                }
                if sm != 0 {
                    let mut ok = true;
                    let mut m = sm;
                    while m != 0 {
                        let t = m.trailing_zeros() as usize;
                        m &= m - 1;
                        if prm >> t & 1 == 0 {
                            let rlo = gm_g(self.group_misses[cursor.group], target_count, cursor.remaining - 1)[t]
                                * self.suffix_gold[(cursor.group + 1) * target_count + t];
                            pr[t] = parent_gold[t] * rlo;
                            prm |= 1 << t;
                        }
                        if !(pr[t] * (gold_misses[bj * target_count + t] - gold_misses[aj * target_count + t])
                            >= DOM_GAP)
                        {
                            ok = false;
                            break;
                        }
                    }
                    if !ok {
                        continue;
                    }
                }
                if ebit[aj] != ebit[bj] && ebit[aj] & dsuf[f + 1] != 0 {
                    nd |= ebit[aj];
                    continue;
                }
                return None;
            }
        }
        // a needed character that no later pick can bring
        if nd & !(constraints.characters | ebit[bj]) & !dsuf[f + 1] != 0 {
            return None;
        }
        Some(nd)
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
        misses: MissRows<'_>,
        node: NodeIdentity,
    ) -> bool {
        // (the cap-free band prunes by the state-conditioned bound alone, `sbx`)
        if self.search.controls.cap_free {
            return true;
        }
        // (each table is read where it is first needed: a read through `search` or `bt` written before the node's stores
        // stays there, as the compiler cannot tell those stores leave it unchanged, and the early return then pays it)
        let search = self.search;
        let target_count = search.controls.target_count;
        let nodes = &mut *scratch.nodes;
        // the cap of every deck the child can still reach: its cards' classes and every entry cursor.remaining
        let rho = if search.controls.best {
            search.controls.rho
        } else {
            let (csuf, goff) = (self.class_suffix, self.group_offsets);
            search.controls.class_caps[cap_class(
                constraints.class_mask
                    | if cursor.remaining > 0 {
                        csuf[goff[cursor.group] + position + 1]
                    } else {
                        csuf[goff[cursor.group + 1]]
                    },
            )]
        };
        // borrowed cards still allowed (only with the one-borrowed tables)
        let can = constraints.borrowed_count < 1;
        let mut have_a = false;
        let has_borrowed_entries = self.borrow_bounds.has_borrowed_entries;
        let row = if cursor.group >= self.group_count {
            usize::MAX
        } else {
            (self.remaining_offsets[cursor.group] + cursor.remaining) * self.borrow_state_count
                + usize::from(has_borrowed_entries && can)
        };
        let (pw, pg) = (&misses.child_white[..target_count], &misses.child_gold[..target_count]);
        if row == usize::MAX {
            for t in 0..target_count {
                nodes.white_chances[t] = 1.0 - pw[t] * 1.0;
                nodes.gold_chances[t] = 1.0 - pg[t] * 1.0;
                nodes.partial_white[t] = 1.0 - pw[t];
                nodes.partial_gold[t] = 1.0 - pg[t];
            }
        } else {
            if !nodes.has_target_constants[row] {
                nodes.has_target_constants[row] = true;
                let (group_misses, sufw, sufg) = (self.group_misses, self.suffix_white, self.suffix_gold);
                let BorrowTables { group_bounds, owned_suffix, borrow_suffix, .. } = self.borrow_bounds;
                for t in 0..target_count {
                    let (a, b) = if !has_borrowed_entries {
                        (
                            gm_w(group_misses[cursor.group], target_count, cursor.remaining)[t]
                                * sufw[(cursor.group + 1) * target_count + t],
                            gm_g(group_misses[cursor.group], target_count, cursor.remaining)[t]
                                * sufg[(cursor.group + 1) * target_count + t],
                        )
                    } else if can {
                        let gb = &group_bounds[cursor.group];
                        (
                            (gb.one[cursor.remaining].white[t] * owned_suffix[cursor.group + 1].white[t])
                                .min(gb.own[cursor.remaining].white[t] * borrow_suffix[cursor.group + 1].white[t]),
                            (gb.one[cursor.remaining].gold[t] * owned_suffix[cursor.group + 1].gold[t])
                                .min(gb.own[cursor.remaining].gold[t] * borrow_suffix[cursor.group + 1].gold[t]),
                        )
                    } else {
                        (
                            group_bounds[cursor.group].own[cursor.remaining].white[t]
                                * owned_suffix[cursor.group + 1].white[t],
                            group_bounds[cursor.group].own[cursor.remaining].gold[t]
                                * owned_suffix[cursor.group + 1].gold[t],
                        )
                    };
                    nodes.target_constants[row * 2 * target_count + t] = a;
                    nodes.target_constants[row * 2 * target_count + target_count + t] = b;
                }
            }
            let (ka, kb) =
                nodes.target_constants[row * 2 * target_count..(row + 1) * 2 * target_count].split_at(target_count);
            if search.cache_parent_rows {
                let (outfit_row_count, orow, inheritance_chances) =
                    (search.outfit_row_count, search.tables.orow, &*search.inheritance_chances);
                // A child is its parent plus one card: for a target the card does not touch (its miss
                // chances exactly 1.0) the child's pw[t] = parent's pw[t] * 1.0 is the parent's, bit for
                // bit, and so are nodes.white_chances[t], nodes.gold_chances[t] and every outfit's A factor of t (the same expressions on
                // the same f64s; ka, kb depend only on the row, the same for all children of one parent
                // and borrowing variant). So the factors are computed once per parent (here, at its first
                // child), a child recomputes only its touched targets', and A is the product in target
                // order (`OutfitPre::eval`'s). The factors before the first touched target t0 are the
                // parent's, and so is their product from 1.0 (`prefix_products`, kept per parent): a child's product
                // starts there and multiplies on from t0, the same multiplications in the same order
                let c = &mut nodes.parent_targets[node.parent_depth * 2 + usize::from(has_borrowed_entries && can)];
                if c.parent_id != node.parent_id {
                    c.parent_id = node.parent_id;
                    for t in 0..target_count {
                        nodes.parent_white[t] = 1.0 - misses.parent_white[t] * ka[t];
                        nodes.parent_gold[t] = 1.0 - misses.parent_gold[t] * kb[t];
                    }
                    for ix in 0..outfit_row_count {
                        orow[ix].factors(
                            inheritance_chances[ix],
                            &nodes.parent_white[..target_count],
                            &nodes.parent_gold[..target_count],
                            &mut c.factors[ix * target_count..(ix + 1) * target_count],
                        );
                    }
                    for (row, pr) in
                        c.factors.chunks_exact(target_count).zip(c.prefix_products.chunks_exact_mut(target_count + 1))
                    {
                        for t in 0..target_count {
                            pr[t + 1] = pr[t] * row[t];
                        }
                    }
                }
                let mut m = node.touched_targets;
                while m != 0 {
                    let t = m.trailing_zeros() as usize;
                    m &= m - 1;
                    nodes.touched_white[t] = 1.0 - pw[t] * ka[t];
                    nodes.touched_gold[t] = 1.0 - pg[t] * kb[t];
                }
                let t0 = if node.touched_targets == 0 {
                    target_count
                } else {
                    node.touched_targets.trailing_zeros() as usize
                };
                for (ix, (row, pr)) in c
                    .factors
                    .chunks_exact_mut(target_count)
                    .zip(c.prefix_products.chunks_exact(target_count + 1))
                    .enumerate()
                {
                    let (o, q) = (&orow[ix], inheritance_chances[ix]);
                    let mut m = node.touched_targets;
                    while m != 0 {
                        let t = m.trailing_zeros() as usize;
                        m &= m - 1;
                        nodes.previous_factors[t] = row[t];
                        row[t] = o.factor_at(t, q, nodes.touched_white[t], nodes.touched_gold[t]);
                    }
                    let mut position = pr[t0];
                    for &x in &row[t0..] {
                        position *= x;
                    }
                    let mut m = node.touched_targets;
                    while m != 0 {
                        let t = m.trailing_zeros() as usize;
                        m &= m - 1;
                        row[t] = nodes.previous_factors[t];
                    }
                    nodes.outfit_bounds[ix] = position;
                }
                // B's partial-deck chances are filled below, only when B is evaluated.
                have_a = true;
            } else {
                for t in 0..target_count {
                    nodes.white_chances[t] = 1.0 - pw[t] * ka[t];
                    nodes.gold_chances[t] = 1.0 - pg[t] * kb[t];
                    nodes.partial_white[t] = 1.0 - pw[t];
                    nodes.partial_gold[t] = 1.0 - pg[t];
                }
            }
        }
        // (bound_check_scale: 1, or RB_CHECK's s; x * 1.0 is x)
        let (best, best_value, clauses, bound_check_scale) =
            (search.controls.best, results.best_value, search.clauses, search.controls.bound_check_scale);
        let pass = |cvx: &[f64]| {
            if best {
                cvx[0] * bound_check_scale * (1.0 + 1e-9) >= best_value
            } else {
                (0..clauses.len()).any(|ci| rho * cvx[ci] * bound_check_scale * (1.0 + 1e-9) >= clauses[ci].1)
            }
        };
        // the per-target values A first (each outfit's term is min(A, B r) <= A, so the clause sums with
        // A alone are upper bounds of the real ones: when they fail, so does the real test; A is finite,
        // its inputs being probabilities, and a NaN falls through to the real test)
        nodes.clause_values.fill(0.0);
        let PreparedTables { oq, og, orow, .. } = search.tables;
        let fixed = search.controls.fixed;
        let mut nan = false;
        for &(ci, i, no) in og {
            let q = oq[i].2;
            let mut m = 0.0f64;
            for o in 0..no {
                let a = if have_a {
                    nodes.outfit_bounds[i + o]
                } else {
                    orow[i + o].eval(q, &nodes.white_chances, &nodes.gold_chances)
                };
                nodes.outfit_bounds[i + o] = a;
                nan |= a.is_nan();
                m = m.max(a);
            }
            nodes.clause_values[ci] += fixed * m;
        }
        if !nan && !pass(&nodes.clause_values) {
            return false;
        }
        let column_count = search.controls.column_count;
        if row != usize::MAX && !nodes.has_ratio_constants[row] {
            nodes.has_ratio_constants[row] = true;
            let (size, group_ratios, spf) = (search.controls.size, self.group_ratios, self.suffix_ratios);
            let BorrowTables { group_bounds, owned_ratio_suffix, borrow_ratio_suffix, .. } = self.borrow_bounds;
            for ix in 0..column_count {
                nodes.ratio_constants[row * column_count + ix] = if !has_borrowed_entries {
                    group_ratios[cursor.group][ix * (size + 1) + cursor.remaining]
                        * spf[(cursor.group + 1) * column_count + ix]
                } else if can {
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
        // B's factors are at the child's odds 1 - pw, 1 - pg. With the per-parent rows (have_a) they are
        // the parent's at its targets the card does not touch (pw[t] is the parent's, bit for bit, as
        // for A): per row, the parent's factors (`ParentRatioBounds`, built at the first child that evaluates the row)
        // with the touched targets' recomputed, and B the product over its targets in target
        // order from 1.0 (`eval`'s and `eval_in`'s), the full one from the parent's prefix at t0.
        // Otherwise use the partial-deck chances filled above.
        let t0 = if node.touched_targets == 0 { target_count } else { node.touched_targets.trailing_zeros() as usize };
        if have_a {
            let h = &mut nodes.parent_ratios[node.parent_depth];
            if h.parent_id != node.parent_id {
                h.parent_id = node.parent_id;
                for t in 0..target_count {
                    h.white_chances[t] = 1.0 - misses.parent_white[t];
                    h.gold_chances[t] = 1.0 - misses.parent_gold[t];
                }
            }
            let mut m = node.touched_targets;
            while m != 0 {
                let t = m.trailing_zeros() as usize;
                m &= m - 1;
                nodes.ratio_white[t] = 1.0 - pw[t];
                nodes.ratio_gold[t] = 1.0 - pg[t];
            }
        }
        // (have_a: row ix's B factors into `nodes.ratio_factors`, the parent's row with the touched ones recomputed;
        // with `full`, their product, else 0)
        let inheritance_chances = &*search.inheritance_chances;
        let mut bfill = |ix: usize, full: bool, brow: &mut [f64; 32]| -> f64 {
            let (o, q) = (&orow[ix], inheritance_chances[ix]);
            let ParentRatioBounds { row_stamps, white_chances: hw, gold_chances: hg, factors, prefix_products, .. } =
                &mut nodes.parent_ratios[node.parent_depth];
            let (fr, pr) = (
                &mut factors[ix * target_count..(ix + 1) * target_count],
                &mut prefix_products[ix * (target_count + 1)..(ix + 1) * (target_count + 1)],
            );
            if row_stamps[ix] != node.parent_id {
                row_stamps[ix] = node.parent_id;
                o.factors(q, &hw[..target_count], &hg[..target_count], fr);
                for t in 0..target_count {
                    pr[t + 1] = pr[t] * fr[t];
                }
            }
            brow[..target_count].copy_from_slice(fr);
            let mut m = node.touched_targets;
            while m != 0 {
                let t = m.trailing_zeros() as usize;
                m &= m - 1;
                brow[t] = o.factor_at(t, q, nodes.ratio_white[t], nodes.ratio_gold[t]);
            }
            if !full {
                return 0.0;
            }
            let mut position = pr[t0];
            for &x in &brow[t0..target_count] {
                position *= x;
            }
            position
        };
        // Product over the selected targets, in the same order as OutfitPre::eval_in.
        let bin = |brow: &[f64; 32], z: u32| -> f64 {
            let (mut z, mut position) = (z, 1.0f64);
            while z != 0 {
                let t = z.trailing_zeros() as usize;
                z &= z - 1;
                position *= brow[t];
            }
            position
        };
        // the real bound, the same expression in the same order; an outfit whose A is at most the running
        // maximum leaves it unchanged (min(A, B r) <= A <= m) and is skipped
        nodes.clause_values.fill(0.0);
        {
            let PreparedTables { zm, nm, alt, .. } = search.tables;
            // (ZSPLIT; a row without a split, zm 0 and no low split, takes min(A, B r)) A's product over a
            // split's Z: from the per-parent row with the child's touched targets recomputed (A's own
            // factors), or at the node's odds
            let hrow =
                if have_a { node.parent_depth * 2 + usize::from(has_borrowed_entries && can) } else { usize::MAX };
            let a_z = |ix: usize, z: u32| -> f64 {
                let (o, q) = (&orow[ix], inheritance_chances[ix]);
                if hrow == usize::MAX {
                    return o.eval_in(q, &nodes.white_chances, &nodes.gold_chances, z);
                }
                let factors = &nodes.parent_targets[hrow].factors[ix * target_count..(ix + 1) * target_count];
                let (mut z, mut position) = (z, 1.0f64);
                while z != 0 {
                    let t = z.trailing_zeros() as usize;
                    z &= z - 1;
                    position *= if node.touched_targets >> t & 1 != 0 {
                        o.factor_at(t, q, nodes.touched_white[t], nodes.touched_gold[t])
                    } else {
                        factors[t]
                    };
                }
                position
            };
            for &(ci, i, no) in og {
                let q = oq[i].2;
                let mut m = 0.0f64;
                for o in 0..no {
                    let ix = i + o;
                    let a = nodes.outfit_bounds[ix];
                    if a <= m {
                        continue;
                    }
                    let r = if row == usize::MAX { 1.0 } else { nodes.ratio_constants[row * column_count + ix] };
                    // A's factors of the zero targets times N's ratio bound; a low split's, the lower
                    let mut b = if have_a {
                        let full = bfill(ix, zm[ix] == 0, &mut nodes.ratio_factors);
                        if zm[ix] != 0 { a_z(ix, zm[ix]) * (bin(&nodes.ratio_factors, nm[ix]) * r) } else { full * r }
                    } else if zm[ix] != 0 {
                        a_z(ix, zm[ix]) * (orow[ix].eval_in(q, &nodes.partial_white, &nodes.partial_gold, nm[ix]) * r)
                    } else {
                        orow[ix].eval(q, &nodes.partial_white, &nodes.partial_gold) * r
                    };
                    for c in alt[ix].0..alt[ix].1 {
                        let r = if row == usize::MAX { 1.0 } else { nodes.ratio_constants[row * column_count + c] };
                        let bn = if have_a {
                            bin(&nodes.ratio_factors, nm[c])
                        } else {
                            orow[ix].eval_in(q, &nodes.partial_white, &nodes.partial_gold, nm[c])
                        };
                        b = b.min(a_z(ix, zm[c]) * (bn * r));
                    }
                    m = m.max(a.min(b));
                }
                nodes.clause_values[ci] += fixed * m;
            }
        }
        let ok = pass(&nodes.clause_values);
        if search.controls.check_ratio_bounds && ok {
            let s = &mut nodes.diagnostic_bounds;
            if s.len() < node.parent_depth + 2 {
                s.resize(node.parent_depth + 2, vec![]);
            }
            s[node.parent_depth + 1].clear();
            s[node.parent_depth + 1].extend_from_slice(&nodes.clause_values);
        }
        ok
    }

    /// The state-conditioned bound at depth <= 1 (mode 1) or at every depth, the leaf included (mode 2); with
    /// SB_CHECK, every leaf's value against its root's and depth-1 node's bounds. False: the node is pruned
    fn sbx(&self, scratch: &mut WalkScratch<'_>, sel: &[usize]) -> bool {
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
            let (space, model, total_slots) = (search.space, search.model, self.total_slots);
            // the cap-free band: the bound at every node and leaf, nothing else prunes; past the node
            // limit every child fails and the search ends with CapFreeNodeLimit
            if scratch.limit_reached {
                return false;
            }
            if space.nodes.get() > cap_free_node_limit {
                scratch.limit_reached = true;
                return false;
            }
            let d = sel.len();
            let leafd = d == total_slots;
            let s = if leafd { 1.0 } else { cap_free_check_scale.max(1.0) };
            // the cheap bound first, the state-conditioned one only where it passes
            let ubc = scratch.state_bound.cheap(sel);
            // (CF_CHECK: both always, so the check below tests each: an exact value above either is above their minimum)
            let ub = if cap_free_check_scale == 0.0 && ubc * s * (1.0 + 1e-9) < state_bound_threshold {
                ubc
            } else {
                ubc.min(
                    scratch
                        .state_bound
                        .full(sel, if cap_free_check_scale == 0.0 { state_bound_threshold / s } else { 0.0 }),
                )
            };
            SBN.with(|c| c.borrow_mut().1 += 1);
            if cap_free_check_scale > 0.0 {
                // CF_CHECK: a leaf's exact value against its own bound and every node's above it
                let path = &mut scratch.bound_path;
                path.truncate(d);
                path.push(ub);
                if leafd {
                    let ents: Vec<(&Src, Kind)> = sel.iter().map(|&j| (&space.srcs[j], space.kind[j])).collect();
                    let x = joint_fast(
                        model,
                        bound_trainee,
                        &[bound_inheritance],
                        &ents,
                        self.filler,
                        target_count + model.n_fixed_gold as usize,
                        false,
                        None,
                    )[0]
                    .0;
                    CFCHK.with(|c| {
                        let mut c = c.borrow_mut();
                        c.4 += 1;
                        for &b in path.iter() {
                            if x > b * (1.0 + 1e-9) {
                                c.5 += 1;
                            }
                            if b > 0.0 {
                                c.8 = c.8.max(x / b);
                            }
                        }
                    });
                }
            }
            let pass = ub * s * (1.0 + 1e-9) >= state_bound_threshold;
            if !pass {
                SBN.with(|c| c.borrow_mut().2 += 1);
            }
            return pass;
        }
        let state_bound_mode = search.controls.state_bound_mode;
        if state_bound_mode == 0 || (state_bound_mode == 3 && !sb_check()) {
            return true;
        }
        let d = sel.len();
        let chk = sb_check();
        let leafd = d == self.total_slots;
        if (state_bound_mode == 1 || state_bound_mode == 3) && d > 1 && !(chk && leafd) {
            return true;
        }
        if state_bound_mode == 3 && d == 1 && !chk {
            return true;
        }
        let ub = scratch.state_bound.bound(sel);
        SBN.with(|c| c.borrow_mut().1 += 1);
        if d == 1 {
            scratch.depth_one_bound = ub;
        }
        if chk && leafd {
            let (sb_root, sb_d1) = (self.root_bound, scratch.depth_one_bound);
            SBCHK.with(|c| {
                let mut c = c.borrow_mut();
                c.4 += 1;
                if ub > sb_root * (1.0 + 1e-9) || (d > 1 && ub > sb_d1 * (1.0 + 1e-9)) {
                    c.5 += 1;
                }
            });
        }
        if (state_bound_mode == 1 && d > 1) || state_bound_mode == 3 {
            return true;
        }
        let pass = ub * (1.0 + 1e-9) >= search.controls.state_bound_threshold;
        if !pass {
            SBN.with(|c| c.borrow_mut().2 += 1);
        }
        pass
    }

    /// A complete deck `sel` (its miss products `pw`, `pg`, class mask `cmk`): kept when it beats the bar (product
    /// search) or passes a clause at its class cap (band). True: the output is full, the search stops
    pub(crate) fn leaf(
        &self,
        scratch: &mut WalkScratch<'_>,
        results: &mut SearchResults,
        sel: &[usize],
        pw: &[f64],
        pg: &[f64],
        cmk: u64,
    ) -> bool {
        let SearchControl { best, cap, target_count, cap_free, class_caps, check_ratio_bounds, .. } =
            self.search.controls;
        let (space, clauses, val) = (self.search.space, self.search.clauses, &self.search.clause_value);
        let (plain, filler) = (self.plain, self.filler);
        let pals = sel.iter().filter(|&&j| space.cards[space.rel[j]].pal).count() + plain;
        if let Some(p) = space.dj.pals
            && p != pals
        {
            return false;
        }
        if !space.locked.iter().all(|l| sel.iter().any(|&j| space.rel[j] == *l)) {
            return false;
        }
        for t in 0..target_count {
            scratch.white_chances[t] = 1.0 - pw[t];
            scratch.gold_chances[t] = 1.0 - pg[t];
        }
        let (dw, dg) = (&scratch.white_chances[..target_count], &scratch.gold_chances[..target_count]);
        // RB_CHECK: each clause's value against the bounds of the root and of every node above the deck
        if check_ratio_bounds {
            let (s, cf) = (&scratch.nodes.diagnostic_bounds, self.configuration);
            RBCHK.with(|c| {
                let mut c = c.borrow_mut();
                c.0 += 1;
                for (ci, cl) in clauses.iter().enumerate() {
                    if best && ci > 0 {
                        break;
                    }
                    let x = val(cl, dw, dg);
                    for d in 0..=sel.len() {
                        let ub = if d == 0 { cf.clause_bounds[ci] } else { s[d][ci] };
                        c.1 += 1;
                        if x > ub * (1.0 + 1e-9) {
                            c.2 += 1;
                        }
                        if ub > 0.0 {
                            c.3 = c.3.max(x / ub);
                        }
                    }
                }
            });
        }
        let mk = |v: f64| {
            let mut e = sel.to_vec();
            e.sort_unstable();
            (v, Sel { ents: e, plain, filler, dw: dw.to_vec(), dg: dg.to_vec() })
        };
        if best {
            let v = val(&clauses[0], dw, dg);
            if v > results.best_value {
                results.best_value = v;
                results.decks.clear();
                results.decks.push(mk(v));
            }
            false
        } else {
            // each clause's value once: the best of them, and whether any passes its bar at the deck's class cap
            let (mut v, mut pass) = (f64::NEG_INFINITY, false);
            let rho = class_caps[cap_class(cmk)];
            for cl in clauses {
                let x = val(cl, dw, dg);
                v = v.max(x);
                pass = pass || cap_free || rho * x * (1.0 + 1e-9) >= cl.1;
            }
            if pass {
                results.decks.push(mk(v));
                if results.decks.len() >= cap {
                    return true;
                }
            }
            false
        }
    }
}
