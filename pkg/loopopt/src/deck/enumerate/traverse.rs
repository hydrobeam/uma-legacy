//! Configuration traversal and its borrowed-card and state bounds.
use crate::Instant;
use crate::checks::{SBN, SPT};
use crate::deck::chains::{ChainKind, ChainState, chain_states};
use crate::deck::enumerate::bounds::{OutfitRow, OutfitRun, SplitColumns};
use crate::deck::enumerate::config::{ConfigurationBounds, ConfigurationQueue, SearchConfig};
use crate::deck::enumerate::order::{ConfigOrder, DeckPath};
use crate::deck::enumerate::scratch::{ConfigScratch, NodeScratch};
use crate::deck::enumerate::walk::{ConfigurationContext, DeckConstraints, DeckCursor, WalkScratch};
use crate::deck::enumerate::{ScoredDeck, SearchClause, SearchOutcome, SearchStop};
use crate::deck::joint::{LandPre, MissFactors};
use crate::deck::space::{
    DeckSpace, DominanceKey, Dominator, EntryMisses, GroupSlots, GroupTable, SelectedDeck, TraversalEntryGroup,
};
use crate::deck::state_bound::{BoundOptions, CapFreeRatios, LandingBoundTables, StateBound, StateBoundConfig};
use crate::skills::DeckChances;
use std::collections::HashMap;
/// Immutable controls selected before traversal. Scalar copies do not change the search's buffers.
#[derive(Clone, Copy)]
pub(crate) struct SearchControl {
    pub(crate) best: bool,
    pub(crate) rho: f64,
    pub(crate) cap: usize,
    pub(crate) target_count: usize,
    pub(crate) size: usize,
    pub(crate) kind_count: usize,
    pub(crate) set_id: usize,
    pub(crate) fixed: f64,
    pub(crate) cap_free: bool,
    pub(crate) cap_free_node_limit: usize,
    pub(crate) cap_free_check_scale: f64,
    pub(crate) state_bound_mode: u8,
    pub(crate) state_bound_threshold: f64,
    pub(crate) bound_trainee: crate::uma::UmaIndex,
    pub(crate) bound_inheritance: f64,
    pub(crate) class_caps: [f64; 4],
    pub(crate) use_dominance: bool,
    pub(crate) lazy: bool,
    pub(crate) cap_free_outfit_count: usize,
    pub(crate) column_count: usize,
    pub(crate) bound_check_scale: f64,
    pub(crate) check_ratio_bounds: bool,
}

/// Read-only views into preparation's tables. The lazy-bound callbacks borrow those same
/// tables, so preparation retains their ownership until traversal returns.
#[derive(Clone, Copy)]
pub(crate) struct PreparedTables<'tab> {
    pub(crate) character_bits: &'tab [u128],
    pub(crate) groups_all: &'tab [&'tab Vec<usize>],
    pub(crate) state_rows: &'tab [Option<Vec<MissFactors>>],
    pub(crate) landing_factors: &'tab [LandPre],
    pub(crate) lock_bits: &'tab [u8],
    pub(crate) all_lock_bits: u8,
    pub(crate) outfit_rows: &'tab [OutfitRow],
    pub(crate) outfit_runs: &'tab [OutfitRun],
    pub(crate) outfits: &'tab [crate::skills::OutfitPre],
    pub(crate) zero_masks: &'tab [u32],
    pub(crate) nonzero_masks: &'tab [u32],
    pub(crate) alternative_columns: &'tab [SplitColumns],
}

/// Reuse preparation's odds buffers and retain diagnostic rows across configurations.
pub(crate) struct TraversalScratch {
    pub(crate) decks: Vec<ScoredDeck<SelectedDeck>>,
    pub(crate) best_value: f64,
    pub(crate) white_chances: Vec<f64>,
    pub(crate) gold_chances: Vec<f64>,
    pub(crate) target_bounds: Vec<f64>,
    pub(crate) diagnostic_bounds: Vec<Vec<f64>>,
}

/// A deck holds at most `allowed_borrow` (0 or 1) borrowed cards; the tables above take each group's best entries
/// whatever they are, so with borrowing they count several borrowed cards at once (every card the player lacks
/// is one), which leaves the bound loose. Per (key, group): the products of its r best owned entries, and of
/// r - 1 owned with its best borrowed one, for the miss chances (lowest first) and the ratios (highest first).
#[derive(Clone)]
pub(crate) struct MissProducts {
    pub(crate) white: Vec<f64>,
    pub(crate) gold: Vec<f64>,
}

pub(crate) struct BorrowBounds {
    pub(crate) owned_misses: Vec<MissProducts>,
    pub(crate) borrow_misses: Vec<MissProducts>,
    pub(crate) owned_ratios: Vec<Vec<f64>>,
    pub(crate) borrow_ratios: Vec<Vec<f64>>,
}

/// The one-borrowed-card tables of a configuration's groups and their suffixes (`owned_suffix`: no borrowed card left;
/// `borrow_suffix`: at most one, in one group); empty without borrowing (`has_borrowed_entries` false).
pub(crate) struct BorrowTables {
    pub(crate) has_borrowed_entries: bool,
    pub(crate) group_bounds: Vec<std::rc::Rc<BorrowBounds>>,
    pub(crate) owned_suffix: Vec<MissProducts>,
    pub(crate) borrow_suffix: Vec<MissProducts>,
    pub(crate) owned_ratio_suffix: Vec<Vec<f64>>,
    pub(crate) borrow_ratio_suffix: Vec<Vec<f64>>,
}

/// The node bound's per-parent rows (see `ConfigurationContext::hopeful`), per (depth, borrowing variant); parent ids never repeat
/// within a search
/// (`factors`: A's factors, row ix * target_count; `prefix_products`: their products from 1.0 over the first t, row ix * (target_count + 1))
pub(crate) struct ParentTargetBounds {
    pub(crate) parent_id: usize,
    pub(crate) factors: Vec<f64>,
    pub(crate) prefix_products: Vec<f64>,
}

/// The same for B, per depth (B has no borrowing variant): the parent's odds 1 - ppw, 1 - ppg, and per row, built
/// at the first child that evaluates its B (`row_stamps`: the parent id it was built for), its factors and their
/// products
pub(crate) struct ParentRatioBounds {
    pub(crate) parent_id: usize,
    pub(crate) white_chances: [f64; 32],
    pub(crate) gold_chances: [f64; 32],
    pub(crate) row_stamps: Vec<usize>,
    pub(crate) factors: Vec<f64>,
    pub(crate) prefix_products: Vec<f64>,
}

/// What one search's configuration build and every configuration of its traversal read: the search, its controls and
/// tables, and `clause_value`.
pub(crate) struct DeckSearchContext<'run, 'space, V> {
    pub(crate) space: &'run DeckSpace<'space>,
    pub(crate) chances: &'run DeckChances<'run>,
    pub(crate) clauses: &'run [SearchClause],
    pub(crate) controls: SearchControl,
    pub(crate) tables: PreparedTables<'run>,
    pub(crate) clause_value: V,
    pub(crate) outfit_row_count: usize,
    // the per-parent rows need every outfit row to read the target_count targets
    pub(crate) cache_parent_rows: bool,
    pub(crate) inheritance_chances: Vec<f64>,
}

/// One traversal: its inputs, the buffers and caches its configurations share, the decks kept and the bar
struct DeckSearchScratch {
    nodes: NodeScratch,
    configuration_scratch: ConfigScratch,
    borrow_cache: HashMap<BorrowKey, std::rc::Rc<BorrowBounds>>,
}

/// Borrowing bounds for a chain code, chain events and group within this search's fixed entry set.
#[derive(PartialEq, Eq, Hash)]
struct BorrowKey {
    code: u64,
    events: crate::deck::chains::EventsKey,
    group: usize,
}

pub(crate) struct SearchResults {
    pub(crate) decks: Vec<ScoredDeck<SelectedDeck>>,
    pub(crate) best_value: f64,
}

impl<'run, 'space, V: Fn(&SearchClause, &[f64], &[f64]) -> f64> DeckSearchContext<'run, 'space, V> {
    /// The search's context, built where its tables come into being (`search_with_bounds`) and read by the
    /// configuration build (`build_configurations`) and the traversal.
    pub(crate) fn new(
        space: &'run DeckSpace<'space>,
        chances: &'run DeckChances<'run>,
        clauses: &'run [SearchClause],
        controls: SearchControl,
        tables: PreparedTables<'run>,
        clause_value: V,
    ) -> Self {
        let outfit_row_count = tables.outfit_rows.len();
        let cache_parent_rows = tables.outfits.iter().all(|outfit| outfit.len() == controls.target_count);
        let inheritance_chances: Vec<f64> = tables.outfit_rows.iter().map(|row| row.inheritance).collect();
        Self {
            space,
            chances,
            clauses,
            controls,
            tables,
            clause_value,
            outfit_row_count,
            cache_parent_rows,
            inheritance_chances,
        }
    }

    /// Traverse the prepared configurations in their visiting order (`ConfigOrder`): each one's root tests, then its
    /// tables and the depth-first walk of its decks (`visit`), until one ends the search.
    pub(crate) fn traverse_configurations(
        &self,
        state_bounds: &mut crate::deck::state_bound::StateBoundMemo,
        queue: ConfigurationQueue,
        scratch: TraversalScratch,
        bounds: &ConfigurationBounds<
            impl Fn(&[f64], &[f64], &[u32], u64) -> GroupTable,
            impl Fn(&[GroupSlots], &GroupTable, &mut [f64], &mut [f64]),
            impl Fn(&[GroupSlots], &GroupTable, &[f64], &[f64], &mut [f64], &mut [f64], &mut [f64]),
        >,
        ratio_at: impl Fn(usize, &[f64], &[f64]) -> f64,
        cap_free_ratios: &CapFreeRatios<'_>,
    ) -> SearchOutcome {
        let traversal_start = Instant::now();
        let (mut scratch, mut results) = Self::workspace(scratch);
        let uncapped = self.controls.state_bound_mode == 2;
        let mut order = ConfigOrder::new(queue, self.controls);
        while let Some(configuration) = order.next(self.space, results.best_value, bounds) {
            if self.past_bar(&configuration, &results) {
                break;
            }
            if let Some(stop) =
                self.visit(state_bounds, &mut scratch, &mut results, &configuration, &ratio_at, cap_free_ratios)
            {
                return SearchOutcome { decks: results.decks, incomplete: Some(stop), uncapped };
            }
        }
        SPT.with(|cell| {
            let mut stats = cell.get();
            stats.tree_seconds += traversal_start.elapsed().as_secs_f64();
            cell.set(stats);
        });
        SearchOutcome { decks: results.decks, incomplete: None, uncapped }
    }

    /// The traversal's buffers (preparation's odds buffers reused) and its results so far (no deck, preparation's bar).
    fn workspace(scratch: TraversalScratch) -> (DeckSearchScratch, SearchResults) {
        let TraversalScratch { decks, best_value, white_chances, gold_chances, target_bounds, diagnostic_bounds } =
            scratch;
        let zeros = [0.0f64; 32];
        let workspace = DeckSearchScratch {
            nodes: NodeScratch {
                white_chances,
                gold_chances,
                outfit_bounds: target_bounds,
                clause_values: vec![],
                partial_white: vec![],
                partial_gold: vec![],
                target_constants: vec![],
                ratio_constants: vec![],
                has_target_constants: vec![],
                has_ratio_constants: vec![],
                parent_targets: vec![],
                parent_ratios: vec![],
                touched_white: zeros,
                touched_gold: zeros,
                previous_factors: zeros,
                ratio_white: zeros,
                ratio_gold: zeros,
                ratio_factors: zeros,
                parent_white: zeros,
                parent_gold: zeros,
                dominance_products: zeros,
                diagnostic_bounds,
            },
            configuration_scratch: ConfigScratch {
                suffix_white: vec![],
                suffix_gold: vec![],
                suffix_ratios: vec![],
                class_suffix: vec![],
                group_offsets: vec![],
                available_locks: vec![],
                path: DeckPath { entries: vec![], white_misses: vec![], gold_misses: vec![], parent_id: 0 },
            },
            borrow_cache: HashMap::new(),
        };
        (workspace, SearchResults { decks, best_value })
    }

    /// No configuration from this `configuration` on can reach the bar (they come in root order).
    fn past_bar(&self, configuration: &SearchConfig, results: &SearchResults) -> bool {
        let SearchControl { best, cap_free, .. } = self.controls;
        (best && configuration.root * (1.0 + 1e-9) < results.best_value)
            || (!best && !cap_free && configuration.root * (1.0 + 1e-9) < 1.0)
    }

    /// Visit one configuration: its split root bound, its tables, its state-conditioned root bound, then the walk of
    /// its decks. Some: the search stops there
    fn visit(
        &self,
        state_bounds: &mut crate::deck::state_bound::StateBoundMemo,
        scratch: &mut DeckSearchScratch,
        results: &mut SearchResults,
        configuration: &SearchConfig,
        ratio_at: &impl Fn(usize, &[f64], &[f64]) -> f64,
        cap_free_ratios: &CapFreeRatios<'_>,
    ) -> Option<SearchStop> {
        let search = self;
        let DeckSearchScratch { nodes, configuration_scratch, borrow_cache } = scratch;
        let controls = search.controls;
        let tables = search.tables;
        let space = search.space;
        // (the split's root bound: skip, the order kept)
        if (controls.best && configuration.prune_bound * controls.bound_check_scale * (1.0 + 1e-9) < results.best_value)
            || (!controls.best
                && !controls.cap_free
                && configuration.prune_bound * controls.bound_check_scale * (1.0 + 1e-9) < 1.0)
        {
            return None;
        }
        // (every miss, bound and state distribution below is at the configuration's own chain events, those of each of
        // its decks: `SearchConfig::events`)
        let key_misses = space.key_miss(configuration.code, configuration.events);
        let (white_misses, gold_misses, touched_targets) =
            (&key_misses.white, &key_misses.gold, (&*key_misses.touched_targets));
        let groups: Vec<TraversalEntryGroup<'_>> = configuration
            .groups
            .iter()
            .map(|group| TraversalEntryGroup { entries: tables.groups_all[group.group], slots: group.slots })
            .collect();
        // Per configured group: its blocks from the key's miss and ratio tables.
        // (blocks: group_misses[g]'s row r by `gm_w`, `gm_g`; group_ratios[g]'s column ix at ix (controls.size + 1))
        let group_misses: Vec<&[f64]> =
            configuration.groups.iter().map(|group| configuration.group_misses.grp(group.group)).collect();
        let group_ratios: Vec<&[f64]> =
            configuration.groups.iter().map(|group| configuration.ratio_prefixes.grp(group.group)).collect();
        configuration_scratch.fill_suffixes(configuration, &controls, &group_misses, &group_ratios);
        let misses = EntryMisses { white: white_misses, gold: gold_misses, target_count: controls.target_count };
        let dominance = search.dominance(configuration, &groups, misses);
        let borrow_bounds = search.borrow_tables(borrow_cache, configuration, &groups, misses, ratio_at);
        nodes.clause_values.clear();
        nodes.clause_values.resize(search.clauses.len(), 0.0);
        nodes.partial_white.clear();
        nodes.partial_white.resize(controls.target_count, 0.0);
        nodes.partial_gold.clear();
        nodes.partial_gold.resize(controls.target_count, 0.0);
        let group_count = groups.len();
        if !controls.best {
            configuration_scratch.fill_class_suffixes(&groups, &space.class_masks);
        }
        // A root bound below the bar rules out every deck in the configuration.
        let state_configuration = search.state_configuration(state_bounds, configuration, &groups);
        let mut state_bound = StateBound::new(
            state_configuration,
            &groups,
            &configuration.groups,
            search.chances.model,
            LandingBoundTables {
                factors: tables.landing_factors,
                entry_rows: tables.state_rows,
                target_count: controls.target_count,
                outfit_count: controls.cap_free_outfit_count,
            },
            cap_free_ratios,
            BoundOptions {
                chain_events: configuration.events,
                cap_free: controls.cap_free,
                trainee: controls.bound_trainee,
                fixed_factor: controls.fixed,
            },
        );
        let sb_root =
            state_bound.root(controls.cap_free_check_scale, controls.state_bound_threshold, controls.state_bound_mode);
        if controls.cap_free {
            // (the cap-free band: past its node limit it stops; CF_CHECK passes the root at controls.cap_free_check_scale times its bound)
            if space.nodes.get() > controls.cap_free_node_limit {
                return Some(SearchStop::CapFreeNodeLimit);
            }
            if sb_root * controls.cap_free_check_scale.max(1.0) * (1.0 + 1e-9) < controls.state_bound_threshold {
                SBN.with(|counters| counters.borrow_mut().configurations_cut += 1);
                return None;
            }
        } else if controls.state_bound_mode > 0 && sb_root * (1.0 + 1e-9) < controls.state_bound_threshold {
            SBN.with(|counters| counters.borrow_mut().configurations_cut += 1);
            return None;
        }
        // (CF_CHECK: the bounds of the nodes on the current path, per depth, the root's at 0)
        let cf_path = if controls.cap_free_check_scale > 0.0 { vec![sb_root] } else { vec![] };
        let row_count = groups.iter().map(|group| group.slots).sum::<usize>() + 1;
        let path_rows = &mut configuration_scratch.path;
        path_rows.entries.clear();
        path_rows.white_misses.clear();
        path_rows.white_misses.resize(row_count * controls.target_count, 1.0);
        path_rows.gold_misses.clear();
        path_rows.gold_misses.resize(row_count * controls.target_count, 1.0);
        nodes.grow_parent_rows(
            2 * (groups.iter().map(|group| group.slots).sum::<usize>() + 1),
            row_count,
            search.outfit_row_count,
            controls.target_count,
            search.cache_parent_rows,
        );
        // per (group g, cards left, borrowing allowed): the per-target suffix factors (a, b) and the per-outfit
        // ratio products r depend on nothing else, so they are constants of the configuration: tabulated on
        // first use of their row (the same expressions, so the same f64s as computing them per node); row
        // (remaining_offsets[g] + left) * borrow_state_count + can, a row of 2 controls.target_count factors (a then b)
        // and one of outfit_row_count ratios
        let borrow_state_count = if borrow_bounds.has_borrowed_entries { 2 } else { 1 };
        let mut remaining_offsets = vec![0usize; group_count + 1];
        for g in 0..group_count {
            remaining_offsets[g + 1] = remaining_offsets[g] + groups[g].slots + 1;
        }
        nodes.reset_constants(
            remaining_offsets[group_count] * borrow_state_count,
            controls.target_count,
            controls.column_count,
        );
        // (a branch that can no longer place every locked card is dropped)
        if !configuration_scratch.fill_available_locks(&groups, tables.lock_bits, tables.all_lock_bits) {
            return None;
        }
        let ConfigScratch {
            suffix_white,
            suffix_gold,
            suffix_ratios,
            class_suffix,
            group_offsets,
            available_locks,
            path,
        } = configuration_scratch;
        let walk = ConfigurationContext {
            search,
            configuration,
            filler_count: configuration.filler_count,
            plain_pal_count: configuration.plain_pal_count,
            groups: &groups,
            group_misses: &group_misses,
            group_ratios: &group_ratios,
            white_misses,
            gold_misses,
            touched_targets,
            suffix_white,
            suffix_gold,
            suffix_ratios,
            dominators: &dominance.tables,
            dominance_offsets: &dominance.offsets,
            dominance_suffix: &dominance.suffix_characters,
            borrow_bounds: &borrow_bounds,
            class_suffix,
            group_offsets,
            available_locks,
            remaining_offsets: &remaining_offsets,
            borrow_state_count,
            group_count,
            total_slots: groups.iter().map(|group| group.slots).sum(),
            root_bound: sb_root,
        };
        let mut walk_scratch = WalkScratch {
            nodes,
            state_bound,
            depth_one_bound: f64::INFINITY,
            bound_path: cf_path,
            limit_reached: false,
            white_chances: [0.0f64; 32],
            gold_chances: [0.0f64; 32],
        };
        let stop = if groups.is_empty() {
            walk.leaf(
                &mut walk_scratch,
                results,
                &[],
                &[1.0; 32][..controls.target_count],
                &[1.0; 32][..controls.target_count],
                0,
            )
        } else {
            walk.walk(
                &mut walk_scratch,
                results,
                path,
                DeckCursor { group: 0, start: 0, remaining: groups[0].slots },
                &DeckConstraints {
                    characters: 0,
                    borrowed_count: 0,
                    locked_cards: 0,
                    class_mask: 0,
                    required_characters: 0,
                },
            )
        };
        if stop {
            return Some(SearchStop::OutputLimit);
        }
        if walk_scratch.limit_reached {
            return Some(SearchStop::CapFreeNodeLimit);
        }
        None
    }
}

/// Per-group dominance candidates and the flattened character suffix used to reject impossible completions.
struct ConfigurationDominance {
    tables: Vec<std::rc::Rc<Vec<Vec<Dominator>>>>,
    offsets: Vec<usize>,
    suffix_characters: Vec<u128>,
}

/// Chain-distribution position and its configuration group; fillers have no group.
struct ConfigurationChain {
    kind: ChainKind,
    group: usize,
}

impl<V> DeckSearchContext<'_, '_, V> {
    /// The dominance tables of `configuration`'s groups (`dominance_table`), and per flat index f (the groups' entries in turn) the
    /// characters of the entries from f on (`dominance_suffix`, group g's from `dominance_offsets[g]`); empty without the dominance skip
    fn dominance(
        &self,
        configuration: &SearchConfig,
        groups: &[TraversalEntryGroup<'_>],
        misses: EntryMisses<'_>,
    ) -> ConfigurationDominance {
        let SearchControl { use_dominance, set_id, .. } = self.controls;
        let PreparedTables { character_bits, groups_all, lock_bits, .. } = self.tables;
        let dominators: Vec<std::rc::Rc<Vec<Vec<Dominator>>>> = if use_dominance {
            configuration
                .groups
                .iter()
                .map(|&GroupSlots { group: i, .. }| {
                    self.space.dominance_table(
                        DominanceKey { set_id, code: configuration.code, events: configuration.events.key(), group: i },
                        groups_all[i],
                        lock_bits,
                        misses,
                    )
                })
                .collect()
        } else {
            vec![]
        };
        let (mut dominance_offsets, mut dominance_suffix): (Vec<usize>, Vec<u128>) = (vec![0], vec![]);
        if use_dominance {
            for group in groups {
                for &j in group.entries {
                    dominance_suffix.push(character_bits[j]);
                }
                dominance_offsets.push(dominance_suffix.len());
            }
            dominance_suffix.push(0);
            for f in (0..dominance_suffix.len() - 1).rev() {
                dominance_suffix[f] |= dominance_suffix[f + 1];
            }
        }
        ConfigurationDominance { tables: dominators, offsets: dominance_offsets, suffix_characters: dominance_suffix }
    }

    /// Group gi's `BorrowBounds` under the key's miss chances `white_misses`, `gold_misses` (`ratio_at`: a card's ratio per column)
    fn borrow_bounds(
        &self,
        group_index: usize,
        misses: EntryMisses<'_>,
        ratio_at: &impl Fn(usize, &[f64], &[f64]) -> f64,
    ) -> BorrowBounds {
        let SearchControl { target_count, size, column_count, .. } = self.controls;
        let entries = self.tables.groups_all[group_index];
        let (owned_entries, borrowed_entries): (Vec<usize>, Vec<usize>) =
            entries.iter().partition(|&&j| !self.space.borrowed[j]);
        let mut owned_misses =
            vec![MissProducts { white: vec![1.0; target_count], gold: vec![1.0; target_count] }; size + 1];
        let mut best_borrowed_misses = MissProducts { white: vec![1.0; target_count], gold: vec![1.0; target_count] };
        for t in 0..target_count {
            let mut white_sorted: Vec<f64> =
                owned_entries.iter().map(|&j| misses.white[j * target_count + t]).collect();
            white_sorted.sort_by(|left, right| left.partial_cmp(right).unwrap());
            let mut gold_sorted: Vec<f64> = owned_entries.iter().map(|&j| misses.gold[j * target_count + t]).collect();
            gold_sorted.sort_by(|left, right| left.partial_cmp(right).unwrap());
            for r in 1..=size {
                owned_misses[r].white[t] =
                    owned_misses[r - 1].white[t] * white_sorted.get(r - 1).copied().unwrap_or(1.0);
                owned_misses[r].gold[t] = owned_misses[r - 1].gold[t] * gold_sorted.get(r - 1).copied().unwrap_or(1.0);
            }
            for &j in &borrowed_entries {
                best_borrowed_misses.white[t] = best_borrowed_misses.white[t].min(misses.white[j * target_count + t]);
                best_borrowed_misses.gold[t] = best_borrowed_misses.gold[t].min(misses.gold[j * target_count + t]);
            }
        }
        let mut one_borrow_misses = owned_misses.clone();
        for r in 1..=size {
            for t in 0..target_count {
                one_borrow_misses[r].white[t] =
                    owned_misses[r].white[t].min(owned_misses[r - 1].white[t] * best_borrowed_misses.white[t]);
                one_borrow_misses[r].gold[t] =
                    owned_misses[r].gold[t].min(owned_misses[r - 1].gold[t] * best_borrowed_misses.gold[t]);
            }
        }
        let (mut white_chances, mut gold_chances) = (vec![0.0f64; target_count], vec![0.0f64; target_count]);
        // (`ratio_at`: without a split column, every zm 0 and cix the identity, the plain ratio)
        let mut ratio = |j: usize| -> Vec<f64> {
            for t in 0..target_count {
                white_chances[t] = 1.0 - misses.white[j * target_count + t];
                gold_chances[t] = 1.0 - misses.gold[j * target_count + t];
            }
            (0..column_count).map(|i| ratio_at(i, &white_chances, &gold_chances)).collect()
        };
        let owned_card_ratios: Vec<Vec<f64>> = owned_entries.iter().map(|&j| ratio(j)).collect();
        let borrowed_card_ratios: Vec<Vec<f64>> = borrowed_entries.iter().map(|&j| ratio(j)).collect();
        let (mut owned_ratios, mut borrow_ratios) = (vec![], vec![]);
        for i in 0..column_count {
            let mut sorted_ratios: Vec<f64> = owned_card_ratios.iter().map(|card_ratios| card_ratios[i]).collect();
            sorted_ratios.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let best_borrowed_ratio =
                borrowed_card_ratios.iter().map(|card_ratios| card_ratios[i]).fold(f64::NEG_INFINITY, f64::max);
            let mut prefixes = vec![1.0f64; size + 1];
            for k in 1..=size {
                prefixes[k] = prefixes[k - 1] * sorted_ratios.get(k - 1).copied().unwrap_or(1.0);
            }
            let mut borrow_ratio_suffix = prefixes.clone();
            if best_borrowed_ratio > f64::NEG_INFINITY {
                for k in 1..=size {
                    borrow_ratio_suffix[k] = prefixes[k].max(prefixes[k - 1] * best_borrowed_ratio);
                }
            }
            owned_ratios.push(prefixes);
            borrow_ratios.push(borrow_ratio_suffix);
        }
        BorrowBounds { owned_misses, borrow_misses: one_borrow_misses, owned_ratios, borrow_ratios }
    }

    /// `configuration`'s one-borrowed-card tables (each group's `BorrowBounds` cached by `BorrowKey` in `borrow_cache`).
    fn borrow_tables(
        &self,
        borrow_cache: &mut HashMap<BorrowKey, std::rc::Rc<BorrowBounds>>,
        configuration: &SearchConfig,
        groups: &[TraversalEntryGroup<'_>],
        misses: EntryMisses<'_>,
        ratio_at: &impl Fn(usize, &[f64], &[f64]) -> f64,
    ) -> BorrowTables {
        let SearchControl { target_count, column_count, .. } = self.controls;
        let space = self.space;
        let has_borrowed_entries =
            space.allowed_borrow == 1 && groups.iter().any(|group| group.entries.iter().any(|&j| space.borrowed[j]));
        let group_bounds: Vec<std::rc::Rc<BorrowBounds>> = if has_borrowed_entries {
            configuration
                .groups
                .iter()
                .map(|&GroupSlots { group: i, .. }| {
                    std::rc::Rc::clone(
                        borrow_cache
                            .entry(BorrowKey { code: configuration.code, events: configuration.events.key(), group: i })
                            .or_insert_with(|| std::rc::Rc::new(self.borrow_bounds(i, misses, ratio_at))),
                    )
                })
                .collect()
        } else {
            vec![]
        };
        let group_count = configuration.groups.len();
        // (allocated only when used)
        let suffix_count = if has_borrowed_entries { group_count + 1 } else { 0 };
        // (vec![x; 0] would still build x: none without borrowing)
        let (mut owned_suffix, mut borrow_suffix) = if suffix_count > 0 {
            (
                vec![MissProducts { white: vec![1.0; target_count], gold: vec![1.0; target_count] }; suffix_count],
                vec![MissProducts { white: vec![1.0; target_count], gold: vec![1.0; target_count] }; suffix_count],
            )
        } else {
            (vec![], vec![])
        };
        let (mut owned_ratio_suffix, mut borrow_ratio_suffix) = if suffix_count > 0 {
            (vec![vec![1.0f64; column_count]; suffix_count], vec![vec![1.0f64; column_count]; suffix_count])
        } else {
            (vec![], vec![])
        };
        if has_borrowed_entries {
            for g in (0..group_count).rev() {
                let (group_bound, slot_count) = (&group_bounds[g], configuration.groups[g].slots);
                for t in 0..target_count {
                    owned_suffix[g].white[t] =
                        group_bound.owned_misses[slot_count].white[t] * owned_suffix[g + 1].white[t];
                    owned_suffix[g].gold[t] =
                        group_bound.owned_misses[slot_count].gold[t] * owned_suffix[g + 1].gold[t];
                    borrow_suffix[g].white[t] = (group_bound.borrow_misses[slot_count].white[t]
                        * owned_suffix[g + 1].white[t])
                        .min(group_bound.owned_misses[slot_count].white[t] * borrow_suffix[g + 1].white[t]);
                    borrow_suffix[g].gold[t] = (group_bound.borrow_misses[slot_count].gold[t]
                        * owned_suffix[g + 1].gold[t])
                        .min(group_bound.owned_misses[slot_count].gold[t] * borrow_suffix[g + 1].gold[t]);
                }
                for i in 0..column_count {
                    owned_ratio_suffix[g][i] = group_bound.owned_ratios[i][slot_count] * owned_ratio_suffix[g + 1][i];
                    borrow_ratio_suffix[g][i] = (group_bound.borrow_ratios[i][slot_count]
                        * owned_ratio_suffix[g + 1][i])
                        .max(group_bound.owned_ratios[i][slot_count] * borrow_ratio_suffix[g + 1][i]);
                }
            }
        }
        BorrowTables {
            has_borrowed_entries,
            group_bounds,
            owned_suffix,
            borrow_suffix,
            owned_ratio_suffix,
            borrow_ratio_suffix,
        }
    }

    /// The state-conditioned tables of `configuration` and `groups`, cached by `StateBoundKey`:
    /// chain positions (sorted kinds, the key's state distribution projected on the positions whose group has chain
    /// targets), per group its slots' positions and, per chain level, each target's lowest misses over the group's
    /// entries (a slot may take any of them, the same one twice: a relaxation). None without the bound (`state_bound_mode` 0)
    fn state_configuration(
        &self,
        state_bounds: &mut crate::deck::state_bound::StateBoundMemo,
        configuration: &SearchConfig,
        groups: &[TraversalEntryGroup<'_>],
    ) -> Option<std::rc::Rc<StateBoundConfig>> {
        let SearchControl { target_count, kind_count, set_id, state_bound_mode, .. } = self.controls;
        let (space, state_rows, filler, group_count) =
            (self.space, self.tables.state_rows, configuration.filler_count, groups.len());
        let state_key = crate::deck::state_bound::StateBoundKey {
            set_id,
            code: configuration.code,
            real_filler_count: configuration.real_filler_count,
            pal_entry_count: configuration.pal_entry_count,
            other_entry_count: configuration.other_entry_count,
            events: configuration.events.key(),
        };
        if state_bound_mode == 0 {
            None
        } else if let Some(cached) = state_bounds.configurations.get(&state_key).cloned() {
            Some(cached)
        } else {
            let mut chains: Vec<ConfigurationChain> = vec![]; // usize::MAX marks a filler group.
            for (g, &GroupSlots { group: i, slots: slot_count }) in configuration.groups.iter().enumerate() {
                if i < kind_count {
                    for _ in 0..slot_count {
                        chains.push(ConfigurationChain { kind: space.kinds_all[i], group: g });
                    }
                }
            }
            for _ in 0..filler {
                chains.push(ConfigurationChain { kind: ChainKind::FILLER, group: usize::MAX });
            }
            chains.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.group.cmp(&b.group)));
            let chain_kinds: Vec<ChainKind> = chains.iter().map(|chain| chain.kind).collect();
            let full = if chain_kinds.is_empty() {
                std::rc::Rc::new(vec![ChainState { progress: vec![], probability: 1.0 }])
            } else {
                chain_states(configuration.events, &chain_kinds)
            };
            let has_chain: Vec<bool> = groups
                .iter()
                .map(|group| group.entries.iter().any(|&j| !space.sources[j].chain_misses.is_empty()))
                .collect();
            let projected_positions: Vec<usize> =
                (0..chains.len()).filter(|&p| chains[p].group != usize::MAX && has_chain[chains[p].group]).collect();
            let mut slots: Vec<Vec<usize>> = vec![vec![]; group_count];
            for (projected_index, &p) in projected_positions.iter().enumerate() {
                slots[chains[p].group].push(projected_index);
            }
            for g in 0..group_count {
                while slots[g].len() < groups[g].slots {
                    slots[g].push(usize::MAX);
                }
            }
            // (with at most 16 positions and levels below 16, the projected key packed 4 bits per position, the first
            // highest: the same sums, each in the order of `full`, sorted the same way)
            let states: Vec<ChainState> = if projected_positions.len() <= 16 && chain_kinds.iter().all(|k| k.stop < 16)
            {
                let mut projected_probabilities: crate::FxMap<u64, f64> = crate::FxMap::default();
                for ChainState { progress, probability } in full.iter() {
                    let key =
                        projected_positions.iter().fold(0u64, |packed, &p| (packed << 4) | u64::from(progress[p]));
                    *projected_probabilities.entry(key).or_insert(0.0) += probability;
                }
                let mut sorted: Vec<(u64, f64)> = projected_probabilities.into_iter().collect();
                sorted.sort_unstable_by_key(|a| a.0);
                let position_count = projected_positions.len();
                sorted
                    .into_iter()
                    .map(|(packed, probability)| ChainState {
                        progress: (0..position_count)
                            .map(|i| (packed >> (4 * (position_count - 1 - i)) & 15) as u8)
                            .collect(),
                        probability,
                    })
                    .collect()
            } else {
                let mut projected_probabilities: crate::FxMap<Vec<u8>, f64> = crate::FxMap::default();
                for ChainState { progress, probability } in full.iter() {
                    let key: Vec<u8> = projected_positions.iter().map(|&p| progress[p]).collect();
                    *projected_probabilities.entry(key).or_insert(0.0) += probability;
                }
                let mut states: Vec<ChainState> = projected_probabilities
                    .into_iter()
                    .map(|(progress, probability)| ChainState { progress, probability })
                    .collect();
                states.sort_by(|a, b| a.progress.cmp(&b.progress));
                states
            };
            let stops: Vec<usize> = configuration
                .groups
                .iter()
                .map(|&GroupSlots { group: i, .. }| if i < kind_count { space.kinds_all[i].stop as usize } else { 0 })
                .collect();
            let group_misses: Vec<Vec<MissFactors>> = (0..group_count)
                .map(|g| {
                    let mut lowest = vec![MissFactors { no_gold: 1.0, no_hint: 1.0 }; (stops[g] + 1) * target_count];
                    for &j in groups[g].entries {
                        if let Some(rows) = &state_rows[j] {
                            for (lowest_miss, row_miss) in lowest.iter_mut().zip(rows.iter()) {
                                lowest_miss.no_gold = lowest_miss.no_gold.min(row_miss.no_gold);
                                lowest_miss.no_hint = lowest_miss.no_hint.min(row_miss.no_hint);
                            }
                        }
                    }
                    lowest
                })
                .collect();
            let configuration_bounds = std::rc::Rc::new(StateBoundConfig {
                states,
                slots,
                group_misses,
                kinds: chain_kinds,
                projected_positions,
                order: std::cell::OnceCell::default(),
            });
            state_bounds.configurations.insert(state_key, std::rc::Rc::clone(&configuration_bounds));
            Some(configuration_bounds)
        }
    }
}
