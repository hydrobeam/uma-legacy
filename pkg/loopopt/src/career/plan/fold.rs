//! Folding spark outcomes and their rises.
use crate::career::inherit::{HerSlot, rise_factor};
use crate::career::plan::TargetMask;
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
    /// Emit setup products for OutcomeContext::row, or complete candidate-slot odds.
    pub(crate) emit_setup_products: bool,
}

/// Position in the fold; leaf digits and landed target bits use different layouts.
#[derive(Clone, Copy)]
pub(crate) struct FoldCursor {
    pub(crate) target: usize,
    pub(crate) leaf: usize,
    pub(crate) landed_mask: usize,
}

impl<L: Leaves> FoldContext<'_, L> {
    /// Star levels folded for this uncertain target.
    const fn star_levels(&self, target: usize) -> &[usize] {
        if self.collapsed_targets[target] { &self.collapsed_star_levels } else { &[0, 1, 2, 3] }
    }
    /// Tables at this target depth; depth zero uses the kept tables.
    fn level<'tab>(&'tab self, levels: &'tab [Vec<f64>], target: usize) -> &'tab [f64] {
        if target == 0 { self.initial_tables } else { &levels[target] }
    }
    /// Emit each fixed-target mask at this leaf. Each candidate slot takes the strict maximum of its
    /// current odds and every setup's spark scale times its folded factor, in setup order.
    fn emit(&self, leaf: usize, uncertain_mask: usize, factors: &[f64], outcomes: &mut [f64]) {
        let leaf_row = self.leaves.row(leaf);
        if self.emit_setup_products {
            outcomes[leaf_row * self.slot_tables.setup_count..(leaf_row + 1) * self.slot_tables.setup_count]
                .copy_from_slice(factors);
            return;
        }
        for (fixed_mask_index, &fixed_mask) in self.leaf_layout.fixed_landed_masks.iter().enumerate() {
            let fixed_mask = fixed_mask.bits();
            let landed_mask = uncertain_mask | fixed_mask;
            let row = &mut outcomes[((leaf_row << self.leaf_layout.fixed_target_count) | fixed_mask_index)
                * self.slot_tables.slot_count..][..self.slot_tables.slot_count];
            row.copy_from_slice(self.slot_tables.current_probabilities);
            let scale_row = &self.slot_tables.spark_scales
                [landed_mask * self.slot_tables.setup_count..(landed_mask + 1) * self.slot_tables.setup_count];
            for ((&scale, &factor), &slot) in scale_row.iter().zip(factors).zip(self.slot_tables.slot_by_setup) {
                let value = scale * factor;
                if value > row[slot] {
                    row[slot] = value;
                }
            }
        }
    }
    /// Emit the last two levels' sixteen-leaf blocks through slot-major accumulators.
    /// Factors are indexed by setup * 16 + block; a block packs the first star digit in its low two bits.
    /// `blocks` lists the star pairs requested by the fold.
    fn emit16(
        &self,
        cursor: FoldCursor,
        factors: &[f64],
        blocks: &[usize],
        block_accumulators: &mut [f64],
        outcomes: &mut [f64],
    ) {
        let (setup_count, slot_count) = (self.slot_tables.setup_count, self.slot_tables.slot_count);
        if self.emit_setup_products {
            for &block in blocks {
                let leaf_row = self.leaves.row(
                    cursor.leaf | ((block & 3) << (2 * cursor.target)) | ((block >> 2) << (2 * (cursor.target + 1))),
                );
                for (setup, output) in
                    outcomes[leaf_row * setup_count..(leaf_row + 1) * setup_count].iter_mut().enumerate()
                {
                    *output = factors[setup * 16 + block];
                }
            }
            return;
        }
        let (first_target_bit, second_target_bit) = (
            1usize << self.leaf_layout.uncertain_targets[cursor.target],
            1usize << self.leaf_layout.uncertain_targets[cursor.target + 1],
        );
        for (fixed_mask_index, &fixed_mask) in self.leaf_layout.fixed_landed_masks.iter().enumerate() {
            let fixed_mask = fixed_mask.bits();
            // Each leaf's scale row starts at its landed mask times the setup count.
            let scale_offsets: [usize; 16] = std::array::from_fn(|block| {
                (cursor.landed_mask
                    | fixed_mask
                    | if block & 3 > 0 { first_target_bit } else { 0 }
                    | if block >> 2 > 0 { second_target_bit } else { 0 })
                    * setup_count
            });
            for (slot, accumulators) in block_accumulators.chunks_exact_mut(16).enumerate() {
                accumulators.fill(self.slot_tables.current_probabilities[slot]);
            }
            for (setup, &slot) in self.slot_tables.slot_by_setup.iter().enumerate() {
                let (setup_factors, accumulators) =
                    (&factors[setup * 16..setup * 16 + 16], &mut block_accumulators[slot * 16..slot * 16 + 16]);
                for &block in blocks {
                    let value = self.slot_tables.spark_scales[scale_offsets[block] + setup] * setup_factors[block];
                    if value > accumulators[block] {
                        accumulators[block] = value;
                    }
                }
            }
            for &block in blocks {
                let leaf_row = self.leaves.row(
                    cursor.leaf | ((block & 3) << (2 * cursor.target)) | ((block >> 2) << (2 * (cursor.target + 1))),
                );
                let row = &mut outcomes
                    [((leaf_row << self.leaf_layout.fixed_target_count) | fixed_mask_index) * slot_count..]
                    [..slot_count];
                for (slot, output) in row.iter_mut().enumerate() {
                    *output = block_accumulators[slot * 16 + block];
                }
            }
        }
    }
    /// Fold each table down the star digits requested by Leaves, in target order.
    /// Interpolation stays source[2i] * absent + source[2i+1] * present. The last two levels remain
    /// unrolled into sixteen leaves per setup; only strict maxima combine different tables.
    pub(crate) fn fold(&self, cursor: FoldCursor, scratch: &mut FoldWorkspace, outcomes: &mut [f64]) {
        let (target_count, setup_count) = (self.leaf_layout.uncertain_targets.len(), self.slot_tables.setup_count);
        if cursor.target == target_count {
            // (target_count < 2: one value per table)
            let factors = &mut scratch.setup_factors[..setup_count];
            factors.fill(0.0);
            let tables = self.level(&scratch.levels, cursor.target);
            for (j, &setup) in self.setup_by_table.iter().enumerate() {
                let value = tables[j];
                if value > factors[setup] {
                    factors[setup] = value;
                }
            }
            self.emit(cursor.leaf, cursor.landed_mask, factors, outcomes);
            return;
        }
        if target_count - cursor.target == 2 {
            // The last two levels have four values per table and sixteen leaves per setup.
            // Fold only requested blocks, omitting the other star levels of collapsed targets.
            let mut blocks = [0usize; 16];
            let mut block_count = 0;
            for &next_stars in self.star_levels(cursor.target + 1) {
                for &stars in self.star_levels(cursor.target) {
                    if !L::ALL
                        && !self.leaves.wants(
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
            let factors = &mut scratch.setup_factors[..16 * setup_count];
            factors.fill(0.0);
            let tables = self.level(&scratch.levels, cursor.target);
            for (j, &setup) in self.setup_by_table.iter().enumerate() {
                let (absent_both, present_first, present_second, present_both) =
                    (tables[4 * j], tables[4 * j + 1], tables[4 * j + 2], tables[4 * j + 3]);
                let (first_present, first_absent, second_present, second_absent) = (
                    &self.present_factors[setup][cursor.target],
                    &self.absent_factors[setup][cursor.target],
                    &self.present_factors[setup][cursor.target + 1],
                    &self.absent_factors[setup][cursor.target + 1],
                );
                let second_absent_values: [f64; 4] = std::array::from_fn(|stars| {
                    absent_both * first_absent[stars] + present_first * first_present[stars]
                });
                let second_present_values: [f64; 4] = std::array::from_fn(|stars| {
                    present_second * first_absent[stars] + present_both * first_present[stars]
                });
                let setup_block = &mut factors[setup * 16..setup * 16 + 16];
                for &block in &blocks[..block_count] {
                    let (stars, next_stars) = (block & 3, block >> 2);
                    let value = second_absent_values[stars] * second_absent[next_stars]
                        + second_present_values[stars] * second_present[next_stars];
                    let best = &mut setup_block[block];
                    if value > *best {
                        *best = value;
                    }
                }
            }
            self.emit16(
                cursor,
                factors,
                &blocks[..block_count],
                &mut scratch.block_accumulators[..16 * self.slot_tables.slot_count],
                outcomes,
            );
            return;
        }
        let half_size = 1usize << (target_count - cursor.target - 1);
        for &stars in self.star_levels(cursor.target) {
            let next_leaf = cursor.leaf | (stars << (2 * cursor.target));
            if !L::ALL && !self.leaves.wants(cursor.target + 1, next_leaf) {
                continue;
            }
            {
                let (lo, hi) = scratch.levels.split_at_mut(cursor.target + 1);
                let (tables, factors) =
                    (if cursor.target == 0 { self.initial_tables } else { &*lo[cursor.target] }, &mut hi[0]);
                for (j, &setup) in self.setup_by_table.iter().enumerate() {
                    let (present, absent) = (
                        self.present_factors[setup][cursor.target][stars],
                        self.absent_factors[setup][cursor.target][stars],
                    );
                    let (source_table, folded_table) = (
                        &tables[2 * half_size * j..2 * half_size * (j + 1)],
                        &mut factors[half_size * j..half_size * (j + 1)],
                    );
                    for i in 0..half_size {
                        folded_table[i] = source_table[2 * i] * absent + source_table[2 * i + 1] * present;
                    }
                }
            }
            self.fold(
                FoldCursor {
                    target: cursor.target + 1,
                    leaf: next_leaf,
                    landed_mask: if stars > 0 {
                        cursor.landed_mask | (1 << self.leaf_layout.uncertain_targets[cursor.target])
                    } else {
                        cursor.landed_mask
                    },
                },
                scratch,
                outcomes,
            );
        }
    }
}

/// Leaf `leaf`'s landed set on the uncertain targets (digit k: her stars on target uncv[k], 0 absent) and her stars'
/// chance: the product over k of w[stars - 1] where she has any, in k order from 1.0 (the f64 `evaluate_outcomes`' per-leaf
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

impl RiseInputs<'_> {
    /// Per leaf (as `leaf_products`) the best outfit's `CareerSetup::rise` of trainee (sts, ews: her outfits' coverage
    /// and own chances) from chances qa (job-target order) to q'(leaf), q'_k = 1 - x0[k] xms[her stars on uncertain
    /// target k]: per outfit and target the factor at each of her four star levels (1 where it can't rise), their
    /// product per leaf
    pub(crate) fn leaves(
        &self,
        anchor_chances: &[f64],
        leaf_products_buffer: &mut Vec<f64>,
        next_leaf_products: &mut Vec<f64>,
    ) -> Vec<f64> {
        let target_count = self.uncertain_targets.len();
        let mut rises = vec![0.0f64; 1usize << (2 * target_count)];
        for (coverage, events) in self.outfit_coverage.iter().zip(self.event_chances) {
            let factors: Vec<[f64; 4]> = (0..target_count)
                .map(|k| {
                    let i = self.uncertain_targets[k];
                    [0, 1, 2, 3].map(|sg| {
                        rise_factor(
                            coverage[k],
                            events[k],
                            1.0 - self.base_misses[k] * self.star_misses[sg],
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
}

/// The leaves (her star patterns on the uncertain targets) a scoring's evaluation produces (`Scored::evaluate`): `All`, the
/// plan's, every leaf in order (each test below is constant and compiles away), or `OneLeaf`, a roll's: one outcome's
/// leaf and its representative (`Scored::representative_of`). The fold goes down a prefix only where `wants` it, and
/// writes and reads each leaf's rows at `row`; the outcome loop takes the leaves in `list`'s order (ascending, as `All`
/// takes them). A leaf's values are computed down its own path alone (the fold's f64s at a leaf read no other leaf's),
/// so every value at a leaf produced is the one `All` computes there
pub(crate) trait Leaves {
    const ALL: bool;
    /// Whether the fold visits this prefix of the given digit count.
    fn wants(&self, digit_count: usize, prefix: usize) -> bool;
    /// the leaves the outcome loop takes, ascending (with `ALL`, unused: every leaf)
    fn list(&self) -> &[usize];
    /// a produced leaf's row (`ALL`: the leaf itself)
    fn row(&self, leaf: usize) -> usize;
    /// Number of rows produced from this many leaves.
    fn nrows(&self, leaf_count: usize) -> usize;
}

pub(crate) struct All;

impl Leaves for All {
    const ALL: bool = true;
    #[inline(always)]
    fn wants(&self, _: usize, _: usize) -> bool {
        true
    }
    fn list(&self) -> &[usize] {
        &[]
    }
    #[inline(always)]
    fn row(&self, leaf: usize) -> usize {
        leaf
    }
    #[inline(always)]
    fn nrows(&self, leaf_count: usize) -> usize {
        leaf_count
    }
}

/// one outcome's leaf: the fold produces its representative only (row 0; a leaf that is not its own representative reads
/// the representative's y, row 1 its own)
pub(crate) struct OneLeaf {
    leaves: [usize; 2],
    count: usize,
}

impl OneLeaf {
    pub(crate) const fn new(leaf: usize, representative: usize) -> Self {
        if representative == leaf {
            Self { leaves: [leaf, leaf], count: 1 }
        } else {
            Self { leaves: [representative, leaf], count: 2 }
        }
    }
}

impl Leaves for OneLeaf {
    const ALL: bool = false;
    fn wants(&self, digit_count: usize, prefix: usize) -> bool {
        prefix == self.leaves[0] & ((1usize << (2 * digit_count)) - 1)
    }
    fn list(&self) -> &[usize] {
        &self.leaves[..self.count]
    }
    fn row(&self, leaf: usize) -> usize {
        usize::from(leaf != self.leaves[0])
    }
    fn nrows(&self, _: usize) -> usize {
        self.count
    }
}
