//! Outfit bounds and ratio prefixes.
use crate::deck::enumerate::{LineageChance, SearchClause};
use crate::deck::space::{DeckSpace, DominanceKey, Dominator, EntryMisses};
use crate::skills::DeckChances;
/// One outfit at one inheritance chance within a search clause.
#[derive(Clone, Copy)]
pub(crate) struct OutfitRow {
    pub(crate) clause: usize,
    pub(crate) trainee: crate::uma::UmaIndex,
    pub(crate) inheritance: f64,
    pub(crate) outfit: usize,
}

/// Consecutive outfit rows contributing one maximum to a clause.
#[derive(Clone, Copy)]
pub(crate) struct OutfitRun {
    pub(crate) clause: usize,
    pub(crate) start: usize,
    pub(crate) count: usize,
}

/// Half-open range of alternative ratio columns for one outfit row.
#[derive(Clone, Copy, Default)]
pub(crate) struct SplitColumns {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// Targets included in a ratio and their empty-deck product.
#[derive(Clone, Copy)]
pub(crate) struct RatioDivisor {
    pub(crate) targets: u32,
    pub(crate) value: f64,
}

/// Outfit rows and their zero/low-target split columns, in clause/chance/outfit order.
/// Bound calculations borrow the tables; traversal takes the `target_bounds` scratch buffer.
pub(crate) struct OutfitBounds {
    pub(crate) outfit_rows: Vec<OutfitRow>,
    pub(crate) outfit_runs: Vec<OutfitRun>,
    // Retain the preparation buffer until traversal ends, preserving its allocation lifetime.
    pub(crate) zero: Vec<f64>,
    pub(crate) target_bounds: Vec<f64>,
    pub(crate) outfits: Vec<crate::skills::OutfitPre>,
    pub(crate) empty_values: Vec<f64>,
    pub(crate) row_indices: Vec<usize>,
    pub(crate) zero_masks: Vec<u32>,
    pub(crate) nonzero_masks: Vec<u32>,
    pub(crate) divisors: Vec<f64>,
    pub(crate) alternative_columns: Vec<SplitColumns>,
    pub(crate) column_count: usize,
}

/// Empty-card factors, ratio-column masks/divisors, and ordered factor prefixes.
pub(crate) struct RatioPrefixes {
    pub(crate) empty_factors: Vec<f64>,
    pub(crate) ratio_divisors: Vec<Option<RatioDivisor>>,
    pub(crate) factor_prefixes: Vec<f64>,
}

pub(crate) fn prepare_outfit_bounds(
    chances: &DeckChances,
    trainees: &[crate::uma::UmaIndex],
    clauses: &[SearchClause],
    target_count: usize,
) -> OutfitBounds {
    // (clause, chance, outfit) triples; the product value of one outfit at one chance is log-submodular in the set of
    // cards (log of each target's factor gains less from a card the more the deck already has), so a deck's value
    // is at most a partial deck's value times the best ratios P({card}) / P({}) of the cards still to add
    // Each clause/trainee/chance contributes a run of its trainee's outfits to `outfit_runs`.
    let outfit_rows: Vec<OutfitRow> = clauses
        .iter()
        .enumerate()
        .flat_map(|(ci, cl)| {
            cl.lineages.iter().flat_map(move |&LineageChance { trainee: k, inheritance: q }| {
                (0..chances.model.n_outfits(trainees[k])).map(move |o| OutfitRow {
                    clause: ci,
                    trainee: trainees[k],
                    inheritance: q,
                    outfit: o,
                })
            })
        })
        .collect();
    let outfit_runs: Vec<OutfitRun> = {
        let mut v = vec![];
        let mut i = 0;
        while i < outfit_rows.len() {
            let no = chances.model.n_outfits(outfit_rows[i].trainee);
            v.push(OutfitRun { clause: outfit_rows[i].clause, start: i, count: no });
            i += no;
        }
        v
    };
    let zero = vec![0.0f64; target_count];
    // the node bound's per-target values, one per (clause, chance, outfit)
    let target_bounds = vec![0.0f64; outfit_rows.len()];
    // each (clause, chance, outfit)'s outfit, prepared once for the bounds below (`OutfitPre`: the same values as outfit_fast)
    let outfits: Vec<crate::skills::OutfitPre> =
        outfit_rows.iter().map(|&OutfitRow { trainee: u, outfit: o, .. }| chances.outfit_pre(u, o)).collect();
    let empty_values: Vec<f64> = outfit_rows
        .iter()
        .map(|&OutfitRow { trainee: u, inheritance: q, outfit: o, .. }| chances.outfit_fast(u, o, q, &zero, &zero))
        .collect();
    // The ratio bound split by target (ZSPLIT). An outfit's value is a product of per-target factors; split the
    // targets into Z and N: over the decks below a node, Z's factors are at most A's (each target's at its lowest
    // misses) and N's at most N's factors at the partial deck times the best ratios of N's factors alone (the same
    // per-target log-submodularity as the plain ratio bound, over N), so B = A_Z * P_N(partial) * r_N bounds the
    // value for any split, and a row's B may be the lowest of several. A row whose empty-deck value is 0 (a target
    // with no inheritance, event or outfit chance: its factor is 0 without a hint) has every plain ratio
    // P({card}) / P({}) infinite, which leaves only A: it is split by its zero targets. A row with low-base targets
    // (empty-deck factor at most LOW_SPLIT of the factor with a certain white hint: their ratios overstate what
    // several cards add) gets one more split, Z its zero and low targets, kept beside its own (neither is lower
    // everywhere). The ratio tables have one column per (row, split): the first outfit_row_count columns are the rows (zero mask 0:
    // the plain ratio bound, or for a zero row nothing to split on (target_count > 32, N's empty value 0 by underflow): its
    // ratios infinite, as before), then the low splits (`alternative_columns[row]`, a range). Per column: its row, Z's and N's
    // target masks, N's empty-deck value
    let outfit_row_count = outfit_rows.len();
    let (row_indices, zero_masks, nonzero_masks, divisors, alternative_columns) = {
        let (mut row_indices, mut zero_masks, mut nonzero_masks, mut divisors, mut alternative_columns) = (
            (0..outfit_row_count).collect::<Vec<_>>(),
            vec![0u32; outfit_row_count],
            vec![0u32; outfit_row_count],
            vec![1.0f64; outfit_row_count],
            vec![SplitColumns::default(); outfit_row_count],
        );
        if target_count <= 32 {
            for i in 0..outfit_row_count {
                if outfits[i].len() != target_count {
                    continue;
                }
                let q = outfit_rows[i].inheritance;
                let full = if target_count == 32 { u32::MAX } else { (1u32 << target_count) - 1 };
                let mut z = 0u32;
                for t in 0..target_count {
                    if !(outfits[i].factor_at(t, q, 0.0, 0.0) > 0.0) {
                        z |= 1 << t;
                    }
                }
                if z != 0 && !(empty_values[i] > 0.0) {
                    let pn = outfits[i].eval_in(q, &zero, &zero, full & !z);
                    if pn > 0.0 {
                        zero_masks[i] = z;
                        nonzero_masks[i] = full & !z;
                        divisors[i] = pn;
                    }
                }
                // (an alternative only where it adds targets to the previous split and leaves N something)
                let (start, mut prev) = (row_indices.len(), z);
                for &tau in &LOW_SPLIT {
                    let mut za = z;
                    for t in 0..target_count {
                        if outfits[i].factor_at(t, q, 0.0, 0.0) <= tau * outfits[i].factor_at(t, q, 1.0, 0.0) {
                            za |= 1 << t;
                        }
                    }
                    if za == prev || za == full {
                        continue;
                    }
                    let pn = outfits[i].eval_in(q, &zero, &zero, full & !za);
                    if pn > 0.0 {
                        row_indices.push(i);
                        zero_masks.push(za);
                        nonzero_masks.push(full & !za);
                        divisors.push(pn);
                        prev = za;
                    }
                }
                alternative_columns[i] = SplitColumns { start, end: row_indices.len() };
            }
        }
        (row_indices, zero_masks, nonzero_masks, divisors, alternative_columns)
    };
    let column_count = row_indices.len();
    OutfitBounds {
        outfit_rows,
        outfit_runs,
        zero,
        target_bounds,
        outfits,
        empty_values,
        row_indices,
        zero_masks,
        nonzero_masks,
        divisors,
        alternative_columns,
        column_count,
    }
}

/// Column i names an outfit row and a target split; every slice uses that column order.
pub(crate) struct RatioColumns<'table> {
    pub(crate) outfit_rows: &'table [OutfitRow],
    pub(crate) outfits: &'table [crate::skills::OutfitPre],
    pub(crate) empty_values: &'table [f64],
    pub(crate) row_indices: &'table [usize],
    pub(crate) zero_masks: &'table [u32],
    pub(crate) nonzero_masks: &'table [u32],
    pub(crate) divisors: &'table [f64],
    pub(crate) target_count: usize,
}

impl RatioColumns<'_> {
    pub(crate) fn prepare_prefixes(&self) -> RatioPrefixes {
        let outfit_row_count = self.outfit_rows.len();
        let column_count = self.row_indices.len();
        // A column's ratio of one card (`ratio_at`, or without the split the plain ratio: the same form, every zero mask 0) is
        // the product of its row's factors over a target mask (the column's nonzero mask for a split column, else all the row's targets, as
        // `outfit_fast` multiplies them), in target order from 1.0, over a divisor (the column's divisor or the row's empty value), or infinite (no
        // split and the row's empty value not > 0: `ratio_divisors` None). At a target the card does not touch (miss chances
        // exactly 1.0, outside `KeyMisses::touched_targets`) its odds are 1 - 1 = 0, giving its factor in `empty_factors`.
        // Per column, `factor_prefixes` multiplies these factors over the mask's targets below each t in the same order; a card's ratio starts
        // from the prefix at its first touched target: the same f64s
        let empty_factors: Vec<f64> = (0..outfit_row_count)
            .flat_map(|i| {
                let (o, q) = (&self.outfits[i], self.outfit_rows[i].inheritance);
                (0..self.target_count).map(move |t| if t < o.len() { o.factor_at(t, q, 0.0, 0.0) } else { 1.0 })
            })
            .collect();
        let ratio_divisors: Vec<Option<RatioDivisor>> = (0..column_count)
            .map(|c| {
                let i = self.row_indices[c];
                let n = self.outfits[i].len();
                if self.zero_masks[c] != 0 {
                    Some(RatioDivisor { targets: self.nonzero_masks[c], value: self.divisors[c] })
                } else if self.empty_values[i] > 0.0 {
                    Some(RatioDivisor {
                        targets: if n >= 32 { u32::MAX } else { (1u32 << n) - 1 },
                        value: self.empty_values[i],
                    })
                } else {
                    None
                }
            })
            .collect();
        let factor_prefixes: Vec<f64> = (0..column_count)
            .flat_map(|c| {
                let i = self.row_indices[c];
                let m = ratio_divisors[c].map_or(0, |x| x.targets);
                let mut v = vec![1.0f64; self.target_count + 1];
                for t in 0..self.target_count {
                    v[t + 1] = if m >> t & 1 != 0 { v[t] * empty_factors[i * self.target_count + t] } else { v[t] };
                }
                v
            })
            .collect();
        RatioPrefixes { empty_factors, ratio_divisors, factor_prefixes }
    }
}

impl DeckSpace<'_> {
    /// The product search's dominance table for `key`: per position p in `entries`, the earlier
    /// positions a whose entry A could take B's place in any deck (borrowed only if B is, B's locked card if B
    /// is one, B's pal status when the job fixes the pals) with every miss chance in [0, 1] and at most B's, white and
    /// gold, for every target; with the targets where A's gold miss is the lower (bit t).
    ///
    /// Why the product search (`best`) may skip a child B of a node (`required_after_dominance` in `search`): let A be such an entry, free at
    /// the node (no pick of its character) and taking B's place in the pick order: a sibling (position in [start, p),
    /// right after the node's picks of group g) or an earlier one no later pick of g touches B's targets with (their
    /// factors are exactly 1.0 there). Any deck D through B and D' = D - B + A then multiply their miss chances in the
    /// same order but for that one factor, so with rounding monotone every miss product of D' is at most D's: every
    /// white odd at least D's, every gold odd equal to D's (equal gold misses) or above it by more than any rounding
    /// (`DOM_GAP`). The value is monotone in the white odds operation by operation, and a gold odd's factor rises by at
    /// least (GEN_GOLD - GEN_WHITE) times its gap, far above its rounding; so val(D') >= val(D) as computed. D' is legal
    /// when no pick of D after B is of A's character (nor of B's: A shares it or is free) and comes before D in the
    /// enumeration (the same configuration, lexicographically lower). The search returns the first deck of the highest
    /// value in enumeration order (strictly higher replaces it; a branch is pruned only below the best so far), which is
    /// never such a D: the decks through B are skipped when nothing of A's character can follow B, else those without a
    /// pick of A's character (`required_characters`). Only which decks are visited (the rest in the same order) and
    /// how the bar rises change: the deck found, its value and odds do not.
    pub(super) fn dominance_table(
        &self,
        key: DominanceKey,
        entries: &[usize],
        lock_bits: &[u8],
        misses: EntryMisses<'_>,
    ) -> std::rc::Rc<Vec<Vec<Dominator>>> {
        if let Some(r) = self.dominance_cache.borrow().get(&key) {
            return std::rc::Rc::clone(r);
        }
        let target_count = self.target_groups.len();
        let in01 = |x: f64| (0.0..=1.0).contains(&x);
        let ok = |j: usize| {
            (0..target_count)
                .all(|t| in01(misses.white[j * target_count + t]) && in01(misses.gold[j * target_count + t]))
        };
        let pal = |j: usize| self.cards[self.card_indices[j]].pal;
        let r = std::rc::Rc::new(
            (0..entries.len())
                .map(|p| {
                    let b = entries[p];
                    if !ok(b) {
                        return vec![];
                    }
                    (0..p)
                        .filter_map(|a| {
                            let aj = entries[a];
                            if self.borrowed[aj] && !self.borrowed[b] {
                                return None;
                            }
                            if lock_bits[b] != 0 && lock_bits[aj] != lock_bits[b] {
                                return None;
                            }
                            if self.deck_job.pals.is_some() && pal(aj) != pal(b) {
                                return None;
                            }
                            if !ok(aj)
                                || !(0..target_count).all(|t| {
                                    misses.white[aj * target_count + t] <= misses.white[b * target_count + t]
                                        && misses.gold[aj * target_count + t] <= misses.gold[b * target_count + t]
                                })
                            {
                                return None;
                            }
                            Some(Dominator {
                                position: a as u32,
                                gold_gap_targets: (0..target_count)
                                    .filter(|&t| misses.gold[aj * target_count + t] < misses.gold[b * target_count + t])
                                    .fold(0u32, |m, t| m | 1 << t),
                            })
                        })
                        .collect()
                })
                .collect::<Vec<_>>(),
        );
        self.dominance_cache.borrow_mut().insert(key, std::rc::Rc::clone(&r));
        r
    }
}

/// The least exact gap (parent's gold-miss product times the picks left's lowest times the misses' difference) for
/// which a lower gold miss is sure to give a gold odd and factor above the other deck's after rounding (both are
/// within about 2^-50 of exact)
pub(crate) const DOM_GAP: f64 = 1.0 / (1u64 << 28) as f64;

/// The low-base split's threshold (ZSPLIT): a target whose empty-deck factor is at most this share of its factor with a
/// certain white hint.
const LOW_SPLIT: [f64; 1] = [0.5];
