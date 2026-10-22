//! Outfit bounds and ratio prefixes.
use crate::deck::space::EntryMisses;
use crate::deck::{DeckSpace, Model};

/// Outfit rows and their zero/low-target split columns, in clause/chance/outfit order.
/// Bound calculations borrow the tables; traversal takes the `av` scratch buffer.
pub(crate) struct OutfitBounds {
    pub(crate) oq: Vec<(usize, crate::UmaIndex, f64, usize)>,
    pub(crate) og: Vec<(usize, usize, usize)>,
    // Retain the preparation buffer until traversal ends, preserving its allocation lifetime.
    pub(crate) zero: Vec<f64>,
    pub(crate) av: Vec<f64>,
    pub(crate) orow: Vec<crate::skills::OutfitPre>,
    pub(crate) p0: Vec<f64>,
    pub(crate) cix: Vec<usize>,
    pub(crate) zm: Vec<u32>,
    pub(crate) nm: Vec<u32>,
    pub(crate) p0n: Vec<f64>,
    pub(crate) alt: Vec<(usize, usize)>,
    pub(crate) column_count: usize,
}

/// Empty-card factors, ratio-column masks/divisors, and ordered factor prefixes.
pub(crate) struct RatioPrefixes {
    pub(crate) f0: Vec<f64>,
    pub(crate) cmk: Vec<Option<(u32, f64)>>,
    pub(crate) cpre: Vec<f64>,
}

pub(crate) fn prepare_outfit_bounds(
    model: &Model,
    us: &[crate::UmaIndex],
    clauses: &[(Vec<(usize, f64)>, f64)],
    target_count: usize,
) -> OutfitBounds {
    // (clause, chance, outfit) triples; the product value of one outfit at one chance is submodular in the set of
    // cards (log of each target's factor gains less from a card the more the deck already has), so a deck's value
    // is at most a partial deck's value times the best ratios P({card}) / P({}) of the cards still to add
    // (clause, trainee, chance, outfit), each (clause, trainee, chance) a run of its trainee's outfits: og
    let oq: Vec<(usize, crate::UmaIndex, f64, usize)> = clauses
        .iter()
        .enumerate()
        .flat_map(|(ci, cl)| {
            cl.0.iter().flat_map(move |&(k, q)| (0..model.n_outfits(us[k])).map(move |o| (ci, us[k], q, o)))
        })
        .collect();
    let og: Vec<(usize, usize, usize)> = {
        let mut v = vec![];
        let mut i = 0;
        while i < oq.len() {
            let no = model.n_outfits(oq[i].1);
            v.push((oq[i].0, i, no));
            i += no;
        }
        v
    };
    let zero = vec![0.0f64; target_count];
    // the node bound's per-target values, one per (clause, chance, outfit)
    let av = vec![0.0f64; oq.len()];
    // each (clause, chance, outfit)'s outfit, prepared once for the bounds below (`OutfitPre`: the same values as outfit_fast)
    let orow: Vec<crate::skills::OutfitPre> = oq.iter().map(|&(_, u, _, o)| model.outfit_pre(u, o)).collect();
    let p0: Vec<f64> = oq.iter().map(|&(_, u, q, o)| model.outfit_fast(u, o, q, &zero, &zero)).collect();
    // The ratio bound split by target (ZSPLIT). An outfit's value is a product of per-target factors; split the
    // targets into Z and N: over the decks below a node, Z's factors are at most A's (each target's at its lowest
    // misses) and N's at most N's factors at the partial deck times the best ratios of N's factors alone (the same
    // per-target log-submodularity as the plain ratio bound, over N), so B = A_Z * P_N(partial) * r_N bounds the
    // value for any split, and a row's B may be the lowest of several. A row whose empty-deck value is 0 (a target
    // with no inheritance, event or outfit chance: its factor is 0 without a hint) has every plain ratio
    // P({card}) / P({}) infinite, which leaves only A: it is split by its zero targets. A row with low-base targets
    // (empty-deck factor at most LOW_SPLIT of the factor with a certain white hint: their ratios overstate what
    // several cards add) gets one more split, Z its zero and low targets, kept beside its own (neither is lower
    // everywhere). The ratio tables have one column per (row, split): columns 0..noq the rows themselves (zm = 0:
    // the plain ratio bound, or for a zero row nothing to split on (target_count > 32, N's empty value 0 by underflow): its
    // ratios infinite, as before), then the low splits (`alt[row]`, a range). Per column: its row, Z's and N's
    // target masks, N's empty-deck value
    let noq = oq.len();
    let (cix, zm, nm, p0n, alt): (Vec<usize>, Vec<u32>, Vec<u32>, Vec<f64>, Vec<(usize, usize)>) = {
        let (mut cix, mut zm, mut nm, mut p0n, mut alt) = (
            (0..noq).collect::<Vec<_>>(),
            vec![0u32; noq],
            vec![0u32; noq],
            vec![1.0f64; noq],
            vec![(0usize, 0usize); noq],
        );
        if target_count <= 32 {
            for i in 0..noq {
                if orow[i].len() != target_count {
                    continue;
                }
                let q = oq[i].2;
                let full = if target_count == 32 { u32::MAX } else { (1u32 << target_count) - 1 };
                let mut z = 0u32;
                for t in 0..target_count {
                    if !(orow[i].factor_at(t, q, 0.0, 0.0) > 0.0) {
                        z |= 1 << t;
                    }
                }
                if z != 0 && !(p0[i] > 0.0) {
                    let pn = orow[i].eval_in(q, &zero, &zero, full & !z);
                    if pn > 0.0 {
                        zm[i] = z;
                        nm[i] = full & !z;
                        p0n[i] = pn;
                    }
                }
                // (an alternative only where it adds targets to the previous split and leaves N something)
                let (start, mut prev) = (cix.len(), z);
                for &tau in &LOW_SPLIT {
                    let mut za = z;
                    for t in 0..target_count {
                        if orow[i].factor_at(t, q, 0.0, 0.0) <= tau * orow[i].factor_at(t, q, 1.0, 0.0) {
                            za |= 1 << t;
                        }
                    }
                    if za == prev || za == full {
                        continue;
                    }
                    let pn = orow[i].eval_in(q, &zero, &zero, full & !za);
                    if pn > 0.0 {
                        cix.push(i);
                        zm.push(za);
                        nm.push(full & !za);
                        p0n.push(pn);
                        prev = za;
                    }
                }
                alt[i] = (start, cix.len());
            }
        }
        (cix, zm, nm, p0n, alt)
    };
    let column_count = cix.len();
    OutfitBounds { oq, og, zero, av, orow, p0, cix, zm, nm, p0n, alt, column_count }
}

/// Column i names an outfit row and a target split; every slice uses that column order.
pub(crate) struct RatioColumns<'table> {
    pub(crate) outfit_rows: &'table [(usize, crate::UmaIndex, f64, usize)],
    pub(crate) outfits: &'table [crate::skills::OutfitPre],
    pub(crate) empty_values: &'table [f64],
    pub(crate) row_indices: &'table [usize],
    pub(crate) zero_masks: &'table [u32],
    pub(crate) nonzero_masks: &'table [u32],
    pub(crate) divisors: &'table [f64],
    pub(crate) target_count: usize,
}

pub(crate) fn prepare_ratio_prefixes(columns: &RatioColumns<'_>) -> RatioPrefixes {
    let noq = columns.outfit_rows.len();
    let column_count = columns.row_indices.len();
    // A column's ratio of one card (`ratio_at`, or without the split the plain ratio: the same form, every columns.zero_masks 0) is
    // the product of its row's factors over a target mask (columns.nonzero_masks[c] for a split column, else all the row's targets, as
    // `outfit_fast` multiplies them), in target order from 1.0, over a divisor (columns.divisors[c], columns.empty_values[i]), or infinite (no
    // split and columns.empty_values[i] not > 0: `cmk` None). At a target the card does not touch (miss chances exactly 1.0, the key's
    // mask `km.2`) its odds are 1 - 1 = 0, the factor `f0` at (0, 0): so per column the products of `f0` over the
    // mask's targets below each t (`cpre`, the same multiplications in the same order), and a card's ratio starts
    // from the prefix at its first touched target: the same f64s
    let f0: Vec<f64> = (0..noq)
        .flat_map(|i| {
            let (o, q) = (&columns.outfits[i], columns.outfit_rows[i].2);
            (0..columns.target_count).map(move |t| if t < o.len() { o.factor_at(t, q, 0.0, 0.0) } else { 1.0 })
        })
        .collect();
    let cmk: Vec<Option<(u32, f64)>> = (0..column_count)
        .map(|c| {
            let i = columns.row_indices[c];
            let n = columns.outfits[i].len();
            if columns.zero_masks[c] != 0 {
                Some((columns.nonzero_masks[c], columns.divisors[c]))
            } else if columns.empty_values[i] > 0.0 {
                Some((if n >= 32 { u32::MAX } else { (1u32 << n) - 1 }, columns.empty_values[i]))
            } else {
                None
            }
        })
        .collect();
    let cpre: Vec<f64> = (0..column_count)
        .flat_map(|c| {
            let i = columns.row_indices[c];
            let m = cmk[c].map_or(0, |x| x.0);
            let mut v = vec![1.0f64; columns.target_count + 1];
            for t in 0..columns.target_count {
                v[t + 1] = if m >> t & 1 != 0 { v[t] * f0[i * columns.target_count + t] } else { v[t] };
            }
            v
        })
        .collect();
    RatioPrefixes { f0, cmk, cpre }
}

/// Dominance is cached per legal entry set, chain key and group.
#[derive(Clone, Copy)]
pub(crate) struct DominanceRequest<'group> {
    pub(crate) set_id: usize,
    pub(crate) code: u64,
    pub(crate) group: usize,
    pub(crate) entries: &'group [usize],
    pub(crate) lock_bits: &'group [u8],
}

impl DeckSpace<'_> {
    /// The product search's dominance table of group `gi` (entries `l`) under key `code`: per position p, the earlier
    /// positions a whose entry A could take B = l[p]'s place in any deck (borrowed only if B is, B's locked card if B
    /// is one, B's pal status when the job fixes the pals) with every miss chance in [0, 1] and at most B's, white and
    /// gold, for every target; with the targets where A's gold miss is the lower (bit t).
    ///
    /// Why the product search (`best`) may skip a child B of a node (`dom` in `search`): let A be such an entry, free at
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
    /// pick of A's character (`need`). Only which decks are visited (the rest in the same order) and how the bar rises
    /// change: the deck found, its value and odds do not.
    pub(crate) fn dom_table(
        &self,
        request: DominanceRequest<'_>,
        misses: EntryMisses<'_>,
    ) -> std::rc::Rc<Vec<Vec<(u32, u32)>>> {
        if let Some(r) = self.domc.borrow().get(&(request.set_id, request.code, request.group)) {
            return std::rc::Rc::clone(r);
        }
        let target_count = self.unc.len();
        let in01 = |x: f64| (0.0..=1.0).contains(&x);
        let ok = |j: usize| {
            (0..target_count)
                .all(|t| in01(misses.white[j * target_count + t]) && in01(misses.gold[j * target_count + t]))
        };
        let pal = |j: usize| self.cards[self.rel[j]].pal;
        let r = std::rc::Rc::new(
            (0..request.entries.len())
                .map(|p| {
                    let b = request.entries[p];
                    if !ok(b) {
                        return vec![];
                    }
                    (0..p)
                        .filter_map(|a| {
                            let aj = request.entries[a];
                            if self.ebor[aj] && !self.ebor[b] {
                                return None;
                            }
                            if request.lock_bits[b] != 0 && request.lock_bits[aj] != request.lock_bits[b] {
                                return None;
                            }
                            if self.dj.pals.is_some() && pal(aj) != pal(b) {
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
                            Some((
                                a as u32,
                                (0..target_count)
                                    .filter(|&t| misses.gold[aj * target_count + t] < misses.gold[b * target_count + t])
                                    .fold(0u32, |m, t| m | 1 << t),
                            ))
                        })
                        .collect()
                })
                .collect::<Vec<_>>(),
        );
        self.domc.borrow_mut().insert((request.set_id, request.code, request.group), std::rc::Rc::clone(&r));
        r
    }
}

/// The least exact gap (parent's gold-miss product times the picks left's lowest times the misses' difference) for
/// which a lower gold miss is sure to give a gold odd and factor above the other deck's after rounding (both are
/// within about 2^-50 of exact)
pub(crate) const DOM_GAP: f64 = 1.0 / (1u64 << 28) as f64;

thread_local! {
    /// RB_CHECK: (leaves checked, (leaf, clause, ancestor) tests, violations: a value above an ancestor's bound by more
    /// than 1e-9, the highest value / bound)
    pub(crate) static RBCHK: std::cell::RefCell<(usize, usize, usize, f64)> = const { std::cell::RefCell::new((0, 0, 0, 0.0)) };
}

pub(crate) fn rb_check() -> f64 {
    static C: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *C.get_or_init(|| std::env::var("RB_CHECK").ok().map_or(0.0, |v| v.parse().unwrap_or(1.0)))
}

/// The low-base split's threshold (ZSPLIT): a target whose empty-deck factor is at most this share of its factor with a
/// certain white hint. Measured on tiny 8-target jobs (instructions): 0.25 and 0.5 the same, 0.75 alone or as a second
/// split (0.5 and 0.75) fewer nodes but more instructions
const LOW_SPLIT: [f64; 1] = [0.5];
