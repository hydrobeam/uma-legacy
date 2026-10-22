//! Reusable buffers for configuration and node bounds.
use super::{DeckPath, ParentRatioBounds, ParentTargetBounds, SearchConfig};
use crate::deck::{gm_g, gm_w};

/// The node bound's buffers and per-parent rows (`ConfigurationContext::hopeful`), and the dominance skip's, reused across
/// configurations
pub(in crate::deck::enumerate) struct NodeScratch {
    pub(in crate::deck::enumerate) white_chances: Vec<f64>,
    pub(in crate::deck::enumerate) gold_chances: Vec<f64>,
    pub(in crate::deck::enumerate) outfit_bounds: Vec<f64>,
    // (the expanded configuration's clause sums and B's odds)
    pub(in crate::deck::enumerate) clause_values: Vec<f64>,
    pub(in crate::deck::enumerate) partial_white: Vec<f64>,
    pub(in crate::deck::enumerate) partial_gold: Vec<f64>,
    // the node bound's per-configuration constants, flat and reused (see `hopeful`)
    pub(in crate::deck::enumerate) target_constants: Vec<f64>,
    pub(in crate::deck::enumerate) ratio_constants: Vec<f64>,
    pub(in crate::deck::enumerate) has_target_constants: Vec<bool>,
    pub(in crate::deck::enumerate) has_ratio_constants: Vec<bool>,
    pub(in crate::deck::enumerate) parent_targets: Vec<ParentTargetBounds>,
    pub(in crate::deck::enumerate) parent_ratios: Vec<ParentRatioBounds>,
    // (a child's touched targets: their odds and the parent's factors they replace)
    pub(in crate::deck::enumerate) touched_white: [f64; 32],
    pub(in crate::deck::enumerate) touched_gold: [f64; 32],
    pub(in crate::deck::enumerate) previous_factors: [f64; 32],
    // (and for B: their odds 1 - pw, 1 - pg, and one row's factors with the touched ones in place)
    pub(in crate::deck::enumerate) ratio_white: [f64; 32],
    pub(in crate::deck::enumerate) ratio_gold: [f64; 32],
    pub(in crate::deck::enumerate) ratio_factors: [f64; 32],
    // (a parent's odds for its A factors, the first target_count slots written before each use)
    pub(in crate::deck::enumerate) parent_white: [f64; 32],
    pub(in crate::deck::enumerate) parent_gold: [f64; 32],
    // (`ConfigurationContext::dom`'s per-target ppg * rlo: written before read in each call)
    pub(in crate::deck::enumerate) dominance_products: [f64; 32],
    // (RB_CHECK: the node bounds on the current path, per depth its clause sums, the root's at depth 0)
    pub(in crate::deck::enumerate) diagnostic_bounds: Vec<Vec<f64>>,
}

impl NodeScratch {
    /// Grow the per-parent rows to `nhc` (A) and, with `a_rows`, `nrows` (B)
    pub(in crate::deck::enumerate) fn grow_parent_rows(
        &mut self,
        nhc: usize,
        nrows: usize,
        noq: usize,
        target_count: usize,
        a_rows: bool,
    ) {
        while self.parent_targets.len() < nhc {
            self.parent_targets.push(ParentTargetBounds {
                parent_id: usize::MAX,
                factors: vec![0.0; noq * target_count],
                prefix_products: vec![1.0; noq * (target_count + 1)],
            });
        }
        if a_rows {
            while self.parent_ratios.len() < nrows {
                self.parent_ratios.push(ParentRatioBounds {
                    parent_id: usize::MAX,
                    white_chances: [0.0; 32],
                    gold_chances: [0.0; 32],
                    row_stamps: vec![usize::MAX; noq],
                    factors: vec![0.0; noq * target_count],
                    prefix_products: vec![1.0; noq * (target_count + 1)],
                });
            }
        }
    }

    /// Clear the node bound's per-configuration constants for `nrow` rows (see `ConfigurationContext::hopeful`)
    pub(in crate::deck::enumerate) fn reset_constants(
        &mut self,
        nrow: usize,
        target_count: usize,
        column_count: usize,
    ) {
        self.target_constants.clear();
        self.target_constants.resize(nrow * 2 * target_count, 0.0);
        self.ratio_constants.clear();
        self.ratio_constants.resize(nrow * column_count, 0.0);
        self.has_target_constants.clear();
        self.has_target_constants.resize(nrow, false);
        self.has_ratio_constants.clear();
        self.has_ratio_constants.resize(nrow, false);
    }
}

/// The expanded configuration's tables, flat and reused across configurations
pub(in crate::deck::enumerate) struct ConfigScratch {
    // the suffix tables: per target (g * target_count + t) the products of the groups' lowest misses from group g on (white,
    // gold), per (clause, chance, outfit) (g * noq + i) those of the best ratios
    pub(in crate::deck::enumerate) sufw: Vec<f64>,
    pub(in crate::deck::enumerate) sufg: Vec<f64>,
    pub(in crate::deck::enumerate) spf: Vec<f64>,
    // (the class masks: csuf[goff[g] + p] = the `cv_mask`s ORed of group g's entries from position p on and of every
    // later group's)
    pub(in crate::deck::enumerate) csuf: Vec<u64>,
    pub(in crate::deck::enumerate) goff: Vec<usize>,
    // locked cards as bits; avail[g][p]: locked cards with an entry at position >= p of group g or in a later group
    pub(in crate::deck::enumerate) avail: Vec<Vec<u8>>,
    pub(in crate::deck::enumerate) path: DeckPath,
}

/// Configuration-local group blocks: miss rows use (remaining, colour, target), ratio columns use (column, remaining).
pub(in crate::deck::enumerate) struct GroupBounds<'table> {
    pub(in crate::deck::enumerate) misses: &'table [&'table [f64]],
    pub(in crate::deck::enumerate) ratios: &'table [&'table [f64]],
    pub(in crate::deck::enumerate) target_count: usize,
    pub(in crate::deck::enumerate) column_count: usize,
    pub(in crate::deck::enumerate) deck_size: usize,
}

impl ConfigScratch {
    /// The suffix tables of `cf` (its groups' key tables `gmin`, `rp`): the same products in the same order as the
    /// per-configuration tables they replace
    pub(in crate::deck::enumerate) fn fill_suffixes(&mut self, cf: &SearchConfig, tables: &GroupBounds<'_>) {
        let Self { sufw, sufg, spf, .. } = self;
        let ng = cf.gs.len();
        sufw.clear();
        sufw.resize((ng + 1) * tables.target_count, 1.0);
        sufg.clear();
        sufg.resize((ng + 1) * tables.target_count, 1.0);
        spf.clear();
        spf.resize((ng + 1) * tables.column_count, 1.0);
        for g in (0..ng).rev() {
            let (gw, gg) = (
                gm_w(tables.misses[g], tables.target_count, cf.gs[g].1),
                gm_g(tables.misses[g], tables.target_count, cf.gs[g].1),
            );
            for t in 0..tables.target_count {
                sufw[g * tables.target_count + t] = sufw[(g + 1) * tables.target_count + t] * gw[t];
                sufg[g * tables.target_count + t] = sufg[(g + 1) * tables.target_count + t] * gg[t];
            }
        }
        for g in (0..ng).rev() {
            let rpg = tables.ratios[g];
            for i in 0..tables.column_count {
                spf[g * tables.column_count + i] =
                    spf[(g + 1) * tables.column_count + i] * rpg[i * (tables.deck_size + 1) + cf.gs[g].1];
            }
        }
    }

    /// The class masks of groups `gs` (entries' `cv_mask`s `cvm`)
    pub(in crate::deck::enumerate) fn fill_class_suffixes(&mut self, gs: &[(&[usize], usize)], cvm: &[u64]) {
        let Self { csuf, goff, .. } = self;
        let ng = gs.len();
        goff.clear();
        goff.push(0);
        for g in 0..ng {
            goff.push(goff[g] + gs[g].0.len());
        }
        csuf.clear();
        csuf.resize(goff[ng] + 1, 0);
        for g in (0..ng).rev() {
            for (p, &j) in gs[g].0.iter().enumerate().rev() {
                csuf[goff[g] + p] = csuf[goff[g] + p + 1] | cvm[j];
            }
        }
    }

    /// The locked-card table of groups `gs`; false when they cannot place every locked card (`all_locked`)
    pub(in crate::deck::enumerate) fn fill_avail(
        &mut self,
        gs: &[(&[usize], usize)],
        lbit: &[u8],
        all_locked: u8,
    ) -> bool {
        let avail = &mut self.avail;
        let ng = gs.len();
        avail.truncate(ng + 1);
        avail.resize_with(ng + 1, Vec::new);
        avail[ng].clear();
        avail[ng].push(0);
        for g in (0..ng).rev() {
            let l = gs[g].0;
            let next = avail[g + 1][0];
            let v = &mut avail[g];
            v.clear();
            v.resize(l.len() + 1, 0u8);
            v[l.len()] = next;
            for p in (0..l.len()).rev() {
                v[p] = v[p + 1] | lbit[l[p]];
            }
        }
        all_locked & !avail[0][0] == 0
    }
}
