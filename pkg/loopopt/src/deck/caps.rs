//! The assumed caps on exact over product odds, per deck structure class, and the exact search's budgets.
use super::TargetSources;

/// Assumed caps on a deck's exact odds over its product odds, by the deck's structure class (`cap_class`): decks and
/// loops are scored exactly wherever their product-odds ceiling times the cap could beat the best exact value. Chain
/// cards share one run's chain events, so the targets their links feed ("chain-varying") land or miss together through
/// the event count, and chain cards compete for those events, which can correlate even one target's feeders: every cap
/// is assumed, from measurement. At most one chain-varying target and no multi-target group 1.05 (no ratio above 1.0
/// seen); two 1.05 (largest seen 1.037, val/ref pal_Fk3); three, and four or more or a multi-target group, 1.25 (largest
/// seen: three 1.127, King Halo in val/plan boot_mob, which rules out a 1.10 cap for three; four or more 1.180, val/ref
/// pal_UF). Every deck scored reports its ratio by class (`exact.class_max_ratio`).
pub(crate) const EXACT_CAPS: [f64; 4] = [1.05, 1.05, 1.25, 1.25];
/// The highest cap (class 4+), the one every ceiling that does not know its deck takes.
pub(crate) const EXACT_RHO: f64 = EXACT_CAPS[3];
/// `CAPS_TEST=a,b,c,d` (checks only): these caps in place of `EXACT_CAPS` (lowered, they force breaches, so the
/// cap-free rerun can be checked against `cap_audit`)
pub(super) fn test_caps() -> Option<[f64; 4]> {
    static CAPS: std::sync::OnceLock<Option<[f64; 4]>> = std::sync::OnceLock::new();
    *CAPS.get_or_init(|| {
        // Preserve the diagnostic parser's permissive behavior: malformed fields are skipped.
        let values: Vec<f64> =
            std::env::var("CAPS_TEST").ok()?.split(',').filter_map(|field| field.parse().ok()).collect();
        (values.len() == 4).then(|| [values[0], values[1], values[2], values[3]])
    })
}
/// Every class cap under the job field `cap_audit` (`val/capaudit.py`: a deck the normal caps would miss shows up)
pub(super) const AUDIT_CAP: f64 = 2.0;
/// An entry's chain-varying targets (bit t: uncertain target t is fed by one of its chain links) and bit 63 when one
/// of its groups is not negatively associated (`TargetSources::uncertified_association`).
pub(super) fn cv_mask(source: &TargetSources) -> u64 {
    let chain_targets = source.chain_misses.iter().fold(0u64, |mask, target| mask | 1u64 << target.target);
    chain_targets | if source.uncertified_association { 1u64 << 63 } else { 0 }
}
/// The structure class of a deck from its entries' `cv_mask`s ORed (or of every deck a search node can still reach,
/// from the masks of its cards and of every candidate left): 0 at most one chain-varying target, 1 two, 2 three, 3 four
/// or more or a multi-target group.
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
pub(super) fn src_class<'src>(it: impl Iterator<Item = &'src TargetSources>) -> usize {
    cap_class(it.fold(0u64, |m, s| m | cv_mask(s)))
}
/// Caps tried in turn, highest first: the answer is certified under the first whose pass finishes within budget.
pub(super) const EXACT_STEPS: [f64; 3] = [EXACT_RHO, 1.12, 1.05];
/// Most decks one exact deck search may return; beyond it the answer is the best found and `exact.complete` is false.
pub(super) const EXACT_CAP: usize = 50_000;
/// Work budget of the exact per-uma search, in units of about 0.1 us natively (2 per partial deck tested, 25 per
/// bound, 200 per exact score, 10,000 per deck search): about 3 s for the pass at EXACT_RHO. A pass that runs out
/// certifies nothing; the next lower step gets a quarter more (see `run_multi`). Jobs where many decks lie within
/// the cap of each other (high inheritance chances, few open targets, borrowing) can exhaust it.
pub(super) const EXACT_BUDGET: usize = 30_000_000;
// exact scorings made on this thread (the planner reports its own as a load-free measure of its work)
thread_local! { pub(crate) static JCOUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
