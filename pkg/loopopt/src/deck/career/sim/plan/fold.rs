//! Folding spark outcomes and their rises.
use super::Leaves;
use crate::deck::career::sim::HerSlot;
use crate::deck::career::sim::plan::TargetMask;
use crate::deck::career::sim::rise_factor;

/// Reusable scoring storage. Fold writes outcome_values before the outcome loop reads it.
#[derive(Default)]
pub(crate) struct FoldScratch {
    pub(crate) fold: FoldWorkspace,
    pub(crate) outcome_values: Vec<f64>,
    pub(crate) outcomes: super::score::OutcomeScratch,
}

/// Mutable fold levels and the last two levels' sixteen-leaf blocks.
#[derive(Default)]
pub(crate) struct FoldWorkspace {
    /// Level k holds table j at j * 2^(uncertain_target_count - k).
    pub(crate) levels: Vec<Vec<f64>>,
    /// Setup-major blocks, indexed setup * 16 + star block.
    pub(crate) setup_factors: Vec<f64>,
    /// Slot-major blocks, indexed slot * 16 + star block.
    pub(crate) block_accumulators: Vec<f64>,
}

thread_local! { pub(crate) static FOLD_SCRATCH: std::cell::RefCell<FoldScratch> = std::cell::RefCell::new(FoldScratch::default()); }

/// The target bits represented by a leaf's star digits and an outcome's fixed mask.
pub(crate) struct LeafLayout<'layout> {
    /// Job-target indices in leaf-digit order.
    pub(crate) uncertain_targets: &'layout [usize],
    /// Job-target masks indexed by the compact fixed-target mask.
    pub(crate) fixed_landed_masks: &'layout [TargetMask],
    pub(crate) fixed_target_count: usize,
}

/// Tables shared by folding and outcome scoring. Setup and slot indices are distinct.
pub(crate) struct SlotTables<'tables> {
    /// Landed-mask-major scales, one entry per setup.
    pub(crate) spark_scales: &'tables [f64],
    /// Candidate slot for each setup; setup order is slot order.
    pub(crate) slot_by_setup: &'tables [usize],
    /// Base odds in slot order.
    pub(crate) current_probabilities: &'tables [f64],
    pub(crate) setup_count: usize,
    pub(crate) slot_count: usize,
}

/// Immutable interpolation inputs; the mutable levels live in FoldWorkspace.
pub(crate) struct FoldContext<'fold, L: Leaves> {
    pub(crate) leaf_layout: LeafLayout<'fold>,
    pub(crate) slot_tables: SlotTables<'fold>,
    pub(crate) initial_tables: &'fold [f64],
    pub(crate) setup_by_table: &'fold [usize],
    /// Per setup, uncertain target and star level: x and 1 - x.
    pub(crate) present_factors: &'fold [Vec<[f64; 4]>],
    pub(crate) absent_factors: &'fold [Vec<[f64; 4]>],
    pub(crate) collapsed_targets: &'fold [bool],
    pub(crate) collapsed_star_levels: [usize; 2],
    pub(crate) leaves: &'fold L,
    /// Emit setup products for sc_row, or complete candidate-slot odds.
    pub(crate) emit_setup_products: bool,
}

// Compatibility name for the planner module's existing import.
pub(crate) use FoldContext as FoldCx;

/// Position in the fold; leaf digits and landed target bits use different layouts.
#[derive(Clone, Copy)]
pub(crate) struct FoldCursor {
    pub(crate) target: usize,
    pub(crate) leaf: usize,
    pub(crate) landed_mask: usize,
}

impl<L: Leaves> FoldContext<'_, L> {
    /// her star levels folded on target k
    const fn star_levels(&self, k: usize) -> &[usize] {
        if self.collapsed_targets[k] { &self.collapsed_star_levels } else { &[0, 1, 2, 3] }
    }
    /// level k's tables (level 0 the kept ones)
    fn level<'tab>(&'tab self, lv: &'tab [Vec<f64>], k: usize) -> &'tab [f64] {
        if k == 0 { self.initial_tables } else { &lv[k] }
    }
}

/// One leaf's rows (each fm; her landed set mu on the uncertain targets) at its row: per candidate slot the best of her
/// odds now and each setup's spark scale at m times its table value f (the products and the strict maxima the outcome
/// loop took, in any setup order: a maximum)
fn fold_emit<L: Leaves>(context: &FoldContext<L>, leaf: usize, mu: usize, f: &[f64], u: &mut [f64]) {
    let leaf_row = context.leaves.row(leaf);
    if context.emit_setup_products {
        u[leaf_row * context.slot_tables.setup_count..(leaf_row + 1) * context.slot_tables.setup_count]
            .copy_from_slice(f);
        return;
    }
    for (fm, &mf) in context.leaf_layout.fixed_landed_masks.iter().enumerate() {
        let mf = mf.bits();
        let m = mu | mf;
        let row = &mut u
            [((leaf_row << context.leaf_layout.fixed_target_count) | fm) * context.slot_tables.slot_count..]
            [..context.slot_tables.slot_count];
        row.copy_from_slice(context.slot_tables.current_probabilities);
        let scale_row = &context.slot_tables.spark_scales
            [m * context.slot_tables.setup_count..(m + 1) * context.slot_tables.setup_count];
        for ((&scale, &factor), &slot) in scale_row.iter().zip(f).zip(context.slot_tables.slot_by_setup) {
            let value = scale * factor;
            if value > row[slot] {
                row[slot] = value;
            }
        }
    }
}

/// `fold_emit` for the 16 leaves of the last two levels (f[si * 16 + b], b = digit k | digit k+1 << 2), through a slot-major
/// block block_accumulators[slot * 16 + b]; `blocks`: the blocks folded (`fold_rec`)
fn fold_emit16<L: Leaves>(
    context: &FoldContext<L>,
    cursor: FoldCursor,
    f: &[f64],
    blocks: &[usize],
    block_accumulators: &mut [f64],
    u: &mut [f64],
) {
    let (setup_count, slot_count) = (context.slot_tables.setup_count, context.slot_tables.slot_count);
    if context.emit_setup_products {
        for &b in blocks {
            let leaf_row = context
                .leaves
                .row(cursor.leaf | ((b & 3) << (2 * cursor.target)) | ((b >> 2) << (2 * (cursor.target + 1))));
            for (si, r) in u[leaf_row * setup_count..(leaf_row + 1) * setup_count].iter_mut().enumerate() {
                *r = f[si * 16 + b];
            }
        }
        return;
    }
    let (first_target_bit, second_target_bit) = (
        1usize << context.leaf_layout.uncertain_targets[cursor.target],
        1usize << context.leaf_layout.uncertain_targets[cursor.target + 1],
    );
    for (fm, &mf) in context.leaf_layout.fixed_landed_masks.iter().enumerate() {
        let mf = mf.bits();
        // per leaf of the block, where its spark scales start (m-major)
        let scale_offsets: [usize; 16] = std::array::from_fn(|b| {
            (cursor.landed_mask
                | mf
                | if b & 3 > 0 { first_target_bit } else { 0 }
                | if b >> 2 > 0 { second_target_bit } else { 0 })
                * setup_count
        });
        for (slot, a) in block_accumulators.chunks_exact_mut(16).enumerate() {
            a.fill(context.slot_tables.current_probabilities[slot]);
        }
        for (si, &slot) in context.slot_tables.slot_by_setup.iter().enumerate() {
            let (setup_factors, a) = (&f[si * 16..si * 16 + 16], &mut block_accumulators[slot * 16..slot * 16 + 16]);
            for &b in blocks {
                let value = context.slot_tables.spark_scales[scale_offsets[b] + si] * setup_factors[b];
                if value > a[b] {
                    a[b] = value;
                }
            }
        }
        for &b in blocks {
            let leaf_row = context
                .leaves
                .row(cursor.leaf | ((b & 3) << (2 * cursor.target)) | ((b >> 2) << (2 * (cursor.target + 1))));
            let row = &mut u[((leaf_row << context.leaf_layout.fixed_target_count) | fm) * slot_count..][..slot_count];
            for (slot, r) in row.iter_mut().enumerate() {
                *r = block_accumulators[slot * 16 + b];
            }
        }
    }
}

/// Fold each table down the star digits requested by Leaves, in target order.
/// Interpolation stays t[2i] * absent + t[2i+1] * present. The last two levels remain
/// unrolled into sixteen leaves per setup; only strict maxima combine different tables.
pub(crate) fn fold_rec<L: Leaves>(
    context: &FoldContext<L>,
    cursor: FoldCursor,
    scratch: &mut FoldWorkspace,
    u: &mut [f64],
) {
    let (target_count, setup_count) = (context.leaf_layout.uncertain_targets.len(), context.slot_tables.setup_count);
    if cursor.target == target_count {
        // (target_count < 2: one value per table)
        let f = &mut scratch.setup_factors[..setup_count];
        f.fill(0.0);
        let t = context.level(&scratch.levels, cursor.target);
        for (j, &si) in context.setup_by_table.iter().enumerate() {
            let value = t[j];
            if value > f[si] {
                f[si] = value;
            }
        }
        fold_emit(context, cursor.leaf, cursor.landed_mask, f, u);
        return;
    }
    if target_count - cursor.target == 2 {
        // the last two levels: 4 values per table, 16 leaves per setup (f[si * 16 + (first stars | (second stars << 2))]), the
        // blocks folded every one but a collapsed target's other star levels and those not wanted
        let mut blocks = [0usize; 16];
        let mut block_count = 0;
        for &next_stars in context.star_levels(cursor.target + 1) {
            for &stars in context.star_levels(cursor.target) {
                if !L::ALL
                    && !context.leaves.wants(
                        target_count,
                        cursor.leaf | (stars << (2 * cursor.target)) | (next_stars << (2 * (cursor.target + 1))),
                    )
                {
                    continue;
                }
                blocks[block_count] = stars | (next_stars << 2);
                block_count += 1;
            }
        }
        let f = &mut scratch.setup_factors[..16 * setup_count];
        f.fill(0.0);
        let t = context.level(&scratch.levels, cursor.target);
        for (j, &si) in context.setup_by_table.iter().enumerate() {
            let (absent_both, present_first, present_second, present_both) =
                (t[4 * j], t[4 * j + 1], t[4 * j + 2], t[4 * j + 3]);
            let (first_present, first_absent, second_present, second_absent) = (
                &context.present_factors[si][cursor.target],
                &context.absent_factors[si][cursor.target],
                &context.present_factors[si][cursor.target + 1],
                &context.absent_factors[si][cursor.target + 1],
            );
            let second_absent_values: [f64; 4] =
                std::array::from_fn(|stars| absent_both * first_absent[stars] + present_first * first_present[stars]);
            let second_present_values: [f64; 4] =
                std::array::from_fn(|stars| present_second * first_absent[stars] + present_both * first_present[stars]);
            let o = &mut f[si * 16..si * 16 + 16];
            for &b in &blocks[..block_count] {
                let (stars, next_stars) = (b & 3, b >> 2);
                let value = second_absent_values[stars] * second_absent[next_stars]
                    + second_present_values[stars] * second_present[next_stars];
                let best = &mut o[b];
                if value > *best {
                    *best = value;
                }
            }
        }
        fold_emit16(
            context,
            cursor,
            f,
            &blocks[..block_count],
            &mut scratch.block_accumulators[..16 * context.slot_tables.slot_count],
            u,
        );
        return;
    }
    let half_size = 1usize << (target_count - cursor.target - 1);
    for &stars in context.star_levels(cursor.target) {
        let next_leaf = cursor.leaf | (stars << (2 * cursor.target));
        if !L::ALL && !context.leaves.wants(cursor.target + 1, next_leaf) {
            continue;
        }
        {
            let (lo, hi) = scratch.levels.split_at_mut(cursor.target + 1);
            let (t, f) = (if cursor.target == 0 { context.initial_tables } else { &*lo[cursor.target] }, &mut hi[0]);
            for (j, &si) in context.setup_by_table.iter().enumerate() {
                let (x, c) = (
                    context.present_factors[si][cursor.target][stars],
                    context.absent_factors[si][cursor.target][stars],
                );
                let (source_table, folded_table) =
                    (&t[2 * half_size * j..2 * half_size * (j + 1)], &mut f[half_size * j..half_size * (j + 1)]);
                for i in 0..half_size {
                    folded_table[i] = source_table[2 * i] * c + source_table[2 * i + 1] * x;
                }
            }
        }
        fold_rec(
            context,
            FoldCursor {
                target: cursor.target + 1,
                leaf: next_leaf,
                landed_mask: if stars > 0 {
                    cursor.landed_mask | (1 << context.leaf_layout.uncertain_targets[cursor.target])
                } else {
                    cursor.landed_mask
                },
            },
            scratch,
            u,
        );
    }
}

/// Leaf `leaf`'s landed set on the uncertain targets (digit k: her stars on target uncv[k], 0 absent) and her stars'
/// chance: the product over k of w[stars - 1] where she has any, in k order from 1.0 (the f64 `sc_outcomes`' per-leaf
/// table holds: each of its entries is a lower leaf's times its top digit's w, the same product in the same order)
pub(crate) fn leaf_set(leaf: usize, uncertain_targets: &[usize], star_mix: [f64; 3]) -> LeafOutcome {
    let mut landed_mask = 0usize;
    let mut star_probability = 1.0f64;
    for (k, &target) in uncertain_targets.iter().enumerate() {
        let stars = (leaf >> (2 * k)) & 3;
        if stars > 0 {
            landed_mask |= 1 << target;
            star_probability *= star_mix[stars - 1];
        }
    }
    LeafOutcome { landed_mask, star_probability }
}

/// A star pattern's landed job-target bits and probability conditional on those landings.
pub(crate) struct LeafOutcome {
    pub(crate) landed_mask: usize,
    pub(crate) star_probability: f64,
}

/// Per leaf (base-4 digits, digit k at bits 2k..2k+1) the product of g[k][digit k] over k, into `out`: level by level,
/// each level's products the previous level's (at the leaf's lower digits) times g[k][digit k], from 1.0. The same
/// operations in the same order as `let mut pr = 1.0; for k in 0..nu { pr *= g[k][digit k]; }` per leaf, so the same
/// f64s; each prefix is computed once instead of once per leaf (4/3 4^nu products for nu 4^nu)
pub(crate) fn leaf_products(g: &[[f64; 4]], out: &mut Vec<f64>, tmp: &mut Vec<f64>) {
    out.clear();
    out.push(1.0);
    for (k, gk) in g.iter().enumerate() {
        let low = (1usize << (2 * k)) - 1;
        tmp.clear();
        for &gd in gk {
            tmp.extend(out[..=low].iter().map(|&o| o * gd));
        }
        std::mem::swap(out, tmp);
    }
}

/// Outfit and inheritance inputs to the rise bound. Outfit rows use uncertain-target order.
pub(crate) struct RiseInputs<'rise> {
    pub(crate) outfit_coverage: &'rise [Vec<u8>],
    pub(crate) event_chances: &'rise [Vec<f64>],
    pub(crate) uncertain_targets: &'rise [usize],
    /// Narrow view of HerSlot's miss factors; no owned vectors are copied.
    pub(crate) base_misses: &'rise [f64],
    pub(crate) star_misses: &'rise [f64; 4],
}

impl<'rise> RiseInputs<'rise> {
    pub(crate) fn for_slot(
        outfit_coverage: &'rise [Vec<u8>],
        event_chances: &'rise [Vec<f64>],
        uncertain_targets: &'rise [usize],
        slot: &'rise HerSlot,
    ) -> Self {
        Self {
            outfit_coverage,
            event_chances,
            uncertain_targets,
            base_misses: &slot.misses_without_her,
            star_misses: &slot.miss_by_stars,
        }
    }
}

/// Per leaf (as `leaf_products`) the best outfit's `Env::rise` of trainee (sts, ews: her outfits' coverage and own
/// chances) from chances qa (job-target order) to q'(leaf), q'_k = 1 - x0[k] xms[her stars on uncertain target k]:
/// per outfit and target the factor at each of her four star levels (1 where it can't rise), their product per leaf
pub(crate) fn rise_leaves(
    inputs: &RiseInputs,
    anchor_chances: &[f64],
    leaf_products_buffer: &mut Vec<f64>,
    next_leaf_products: &mut Vec<f64>,
) -> Vec<f64> {
    let target_count = inputs.uncertain_targets.len();
    let mut rises = vec![0.0f64; 1usize << (2 * target_count)];
    for (coverage, events) in inputs.outfit_coverage.iter().zip(inputs.event_chances) {
        let factors: Vec<[f64; 4]> = (0..target_count)
            .map(|k| {
                let i = inputs.uncertain_targets[k];
                [0, 1, 2, 3].map(|sg| {
                    rise_factor(
                        coverage[k],
                        events[k],
                        1.0 - inputs.base_misses[k] * inputs.star_misses[sg],
                        anchor_chances[i],
                    )
                })
            })
            .collect();
        leaf_products(&factors, leaf_products_buffer, next_leaf_products);
        for (rise, &product) in rises.iter_mut().zip(leaf_products_buffer.iter()) {
            if product > *rise {
                *rise = product;
            }
        }
    }
    rises
}
