//! Reusable buffers for configuration and node bounds.
use crate::deck::enumerate::config::SearchConfig;
use crate::deck::enumerate::order::DeckPath;
use crate::deck::enumerate::traverse::{ParentRatioBounds, ParentTargetBounds, SearchControl};
use crate::deck::space::{TraversalEntryGroup, gm_g, gm_w};
/// The node bound's buffers and per-parent rows (`ConfigurationContext::hopeful`), and the dominance skip's, reused across
/// configurations
pub(crate) struct NodeScratch {
    pub(crate) white_chances: Vec<f64>,
    pub(crate) gold_chances: Vec<f64>,
    pub(crate) outfit_bounds: Vec<f64>,
    // (the expanded configuration's clause sums and B's odds)
    pub(crate) clause_values: Vec<f64>,
    pub(crate) partial_white: Vec<f64>,
    pub(crate) partial_gold: Vec<f64>,
    // the node bound's per-configuration constants, flat and reused (see `hopeful`)
    pub(crate) target_constants: Vec<f64>,
    pub(crate) ratio_constants: Vec<f64>,
    pub(crate) has_target_constants: Vec<bool>,
    pub(crate) has_ratio_constants: Vec<bool>,
    pub(crate) parent_targets: Vec<ParentTargetBounds>,
    pub(crate) parent_ratios: Vec<ParentRatioBounds>,
    // (a child's touched targets: their odds and the parent's factors they replace)
    pub(crate) touched_white: [f64; 32],
    pub(crate) touched_gold: [f64; 32],
    pub(crate) previous_factors: [f64; 32],
    // (and for B: one minus the white/gold misses, and one row's factors with the touched ones in place)
    pub(crate) ratio_white: [f64; 32],
    pub(crate) ratio_gold: [f64; 32],
    pub(crate) ratio_factors: [f64; 32],
    // (a parent's odds for its A factors, the first target_count slots written before each use)
    pub(crate) parent_white: [f64; 32],
    pub(crate) parent_gold: [f64; 32],
    // (`ConfigurationContext::required_after_dominance`'s per-target ppg * rlo: written before read in each call)
    pub(crate) dominance_products: [f64; 32],
    // (RB_CHECK: the node bounds on the current path, per depth its clause sums, the root's at depth 0)
    pub(crate) diagnostic_bounds: Vec<Vec<f64>>,
}

impl NodeScratch {
    /// Grow A's rows to `parent_row_count` and, with `cache_ratio_rows`, B's to `depth_count`.
    pub(crate) fn grow_parent_rows(
        &mut self,
        parent_row_count: usize,
        depth_count: usize,
        outfit_row_count: usize,
        target_count: usize,
        cache_ratio_rows: bool,
    ) {
        while self.parent_targets.len() < parent_row_count {
            self.parent_targets.push(ParentTargetBounds {
                parent_id: usize::MAX,
                factors: vec![0.0; outfit_row_count * target_count],
                prefix_products: vec![1.0; outfit_row_count * (target_count + 1)],
            });
        }
        if cache_ratio_rows {
            while self.parent_ratios.len() < depth_count {
                self.parent_ratios.push(ParentRatioBounds {
                    parent_id: usize::MAX,
                    white_chances: [0.0; 32],
                    gold_chances: [0.0; 32],
                    row_stamps: vec![usize::MAX; outfit_row_count],
                    factors: vec![0.0; outfit_row_count * target_count],
                    prefix_products: vec![1.0; outfit_row_count * (target_count + 1)],
                });
            }
        }
    }

    /// Clear the node bound's per-configuration constants for `row_count` rows (see `ConfigurationContext::hopeful`)
    pub(crate) fn reset_constants(&mut self, row_count: usize, target_count: usize, column_count: usize) {
        self.target_constants.clear();
        self.target_constants.resize(row_count * 2 * target_count, 0.0);
        self.ratio_constants.clear();
        self.ratio_constants.resize(row_count * column_count, 0.0);
        self.has_target_constants.clear();
        self.has_target_constants.resize(row_count, false);
        self.has_ratio_constants.clear();
        self.has_ratio_constants.resize(row_count, false);
    }
}

/// The expanded configuration's tables, flat and reused across configurations
pub(crate) struct ConfigScratch {
    // the suffix tables: per target (g * target_count + t) the products of the groups' lowest misses from group g on (white,
    // gold), per (clause, chance, outfit) (g * noq + i) those of the best ratios
    pub(crate) suffix_white: Vec<f64>,
    pub(crate) suffix_gold: Vec<f64>,
    pub(crate) suffix_ratios: Vec<f64>,
    // (the class masks: class_suffix[group_offsets[g] + p] = the `cv_mask`s ORed from position p of group g and every
    // later group's)
    pub(crate) class_suffix: Vec<u64>,
    pub(crate) group_offsets: Vec<usize>,
    // Locked cards as bits; available_locks[g][p] covers positions >= p of group g and every later group.
    pub(crate) available_locks: Vec<Vec<u8>>,
    pub(crate) path: DeckPath,
}

impl ConfigScratch {
    /// The suffix tables of `configuration`, from its group miss and ratio tables: the same products in the same order as the
    /// per-configuration tables they replace
    pub(crate) fn fill_suffixes(
        &mut self,
        configuration: &SearchConfig,
        controls: &SearchControl,
        group_misses: &[&[f64]],
        group_ratios: &[&[f64]],
    ) {
        let Self { suffix_white, suffix_gold, suffix_ratios, .. } = self;
        let group_count = configuration.groups.len();
        suffix_white.clear();
        suffix_white.resize((group_count + 1) * controls.target_count, 1.0);
        suffix_gold.clear();
        suffix_gold.resize((group_count + 1) * controls.target_count, 1.0);
        suffix_ratios.clear();
        suffix_ratios.resize((group_count + 1) * controls.column_count, 1.0);
        for g in (0..group_count).rev() {
            let (gw, gg) = (
                gm_w(group_misses[g], controls.target_count, configuration.groups[g].slots),
                gm_g(group_misses[g], controls.target_count, configuration.groups[g].slots),
            );
            for t in 0..controls.target_count {
                suffix_white[g * controls.target_count + t] = suffix_white[(g + 1) * controls.target_count + t] * gw[t];
                suffix_gold[g * controls.target_count + t] = suffix_gold[(g + 1) * controls.target_count + t] * gg[t];
            }
        }
        for g in (0..group_count).rev() {
            let group_ratios = group_ratios[g];
            for i in 0..controls.column_count {
                suffix_ratios[g * controls.column_count + i] = suffix_ratios[(g + 1) * controls.column_count + i]
                    * group_ratios[i * (controls.size + 1) + configuration.groups[g].slots];
            }
        }
    }

    /// The suffix class masks of `groups`, from the entries' `class_masks`.
    pub(crate) fn fill_class_suffixes(&mut self, groups: &[TraversalEntryGroup<'_>], class_masks: &[u64]) {
        let Self { class_suffix, group_offsets, .. } = self;
        let group_count = groups.len();
        group_offsets.clear();
        group_offsets.push(0);
        for g in 0..group_count {
            group_offsets.push(group_offsets[g] + groups[g].entries.len());
        }
        class_suffix.clear();
        class_suffix.resize(group_offsets[group_count] + 1, 0);
        for g in (0..group_count).rev() {
            for (p, &j) in groups[g].entries.iter().enumerate().rev() {
                class_suffix[group_offsets[g] + p] = class_suffix[group_offsets[g] + p + 1] | class_masks[j];
            }
        }
    }

    /// The locked-card table of `groups`; false when they cannot place every locked card (`all_locked`).
    pub(crate) fn fill_available_locks(
        &mut self,
        groups: &[TraversalEntryGroup<'_>],
        lock_bits: &[u8],
        all_locked: u8,
    ) -> bool {
        let available_locks = &mut self.available_locks;
        let group_count = groups.len();
        available_locks.truncate(group_count + 1);
        available_locks.resize_with(group_count + 1, Vec::new);
        available_locks[group_count].clear();
        available_locks[group_count].push(0);
        for g in (0..group_count).rev() {
            let l = groups[g].entries;
            let next = available_locks[g + 1][0];
            let v = &mut available_locks[g];
            v.clear();
            v.resize(l.len() + 1, 0u8);
            v[l.len()] = next;
            for p in (0..l.len()).rev() {
                v[p] = v[p + 1] | lock_bits[l[p]];
            }
        }
        all_locked & !available_locks[0][0] == 0
    }
}
