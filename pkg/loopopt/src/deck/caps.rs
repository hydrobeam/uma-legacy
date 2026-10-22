//! The assumed caps on exact over product odds, per deck structure class, and the exact search's budgets.
use crate::deck::sources::TargetSources;
/// Assumed caps on a deck's exact odds over its product odds, by the deck's structure class (`cap_class`): decks and
/// loops are scored exactly wherever their product-odds ceiling times the cap could beat the best exact value. Chain
/// cards share one run's chain events, so the targets their links feed ("chain-varying") land or miss together through
/// the event count, and chain cards compete for those events, which can correlate even one target's feeders: every cap
/// is assumed, from measurement. The last class also includes any uncertified outcome group (`cv_mask`).
/// Measured maxima belong to the validation outputs' `exact.class_max_ratio`, not to these assumed ceilings.
pub(crate) const EXACT_CAPS: [f64; 4] = [1.05, 1.05, 1.25, 1.25];
/// The highest cap (class 4+), the one every ceiling that does not know its deck takes.
pub(crate) const EXACT_RHO: f64 = EXACT_CAPS[3];
/// Every class cap under the job field `cap_audit` (`val/capaudit.py`: a deck the normal caps would miss shows up)
pub(super) const AUDIT_CAP: f64 = 2.0;
impl TargetSources {
    /// An entry's chain-varying targets (bit t: uncertain target t is fed by one of its chain links) and bit 63 when one
    /// of its groups is not negatively associated (`TargetSources::uncertified_association`).
    pub(super) fn cv_mask(&self) -> u64 {
        let chain_targets = self.chain_misses.iter().fold(0u64, |mask, target| mask | (1u64 << target.target));
        chain_targets | if self.uncertified_association { 1u64 << 63 } else { 0 }
    }
}
/// The structure class of a deck from its entries' `cv_mask`s ORed (or of every deck a search node can still reach,
/// from the masks of its cards and of every candidate left): 0 at most one chain-varying target, 1 two, 2 three, 3 four
/// or more or an uncertified outcome group.
pub(crate) const fn cap_class(mask: u64) -> usize {
    if mask >> 63 != 0 {
        return 3;
    }
    match mask.count_ones() {
        0 | 1 => 0,
        2 => 1,
        3 => 2,
        _ => 3,
    }
}
/// The class of a deck given by its sources.
pub(super) fn src_class<'src>(sources: impl Iterator<Item = &'src TargetSources>) -> usize {
    cap_class(sources.fold(0u64, |mask, source| mask | source.cv_mask()))
}
/// Caps tried in turn, highest first: the answer is certified under the first whose pass finishes within budget.
pub(crate) const EXACT_STEPS: [f64; 3] = [EXACT_RHO, 1.12, 1.05];
/// Most decks one exact deck search may return; beyond it the answer is the best found and `exact.complete` is false.
pub(super) const EXACT_CAP: usize = 50_000;
/// Work budget of the exact per-uma search (2 per partial deck tested, 25 per bound, 200 per exact score,
/// 10,000 per deck search). A pass that runs out certifies nothing; the next lower step gets a quarter more
/// (see `run_multi`). Jobs where many decks lie within
/// the cap of each other (high inheritance chances, few open targets, borrowing) can exhaust it.
pub(crate) const EXACT_BUDGET: usize = 30_000_000;
