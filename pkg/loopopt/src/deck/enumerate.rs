//! The deck enumeration (`DeckSpace::search`): node bounds, the exact pass with its caps and certificate.
use crate::deck::{DeckSpace, LandPre, LegalTabs, Model, Sel};
pub(crate) use bounds::RBCHK;
use bounds::{DOM_GAP, OutfitBounds, RatioPrefixes, prepare_outfit_bounds, prepare_ratio_prefixes, rb_check};
use config::{ConfigurationQueue, ConfigurationScratch, ConfigurationTables, PendingConfig, SearchConfig};
use order::{ConfigOrder, DeckPath};
use scratch::{ConfigScratch, NodeScratch};
use traverse::{
    BorrowTables, ParentRatioBounds, ParentTargetBounds, SearchControl, TraversalInputs, TraversalScratch,
    TraversalTables,
};
use walk::Walk;
mod bounds;
mod config;
mod order;
mod prepare;
mod scratch;
mod traverse;
mod walk;

/// The two enumeration passes have different stopping rules and inputs.
#[derive(Clone, Copy)]
pub(super) enum SearchMode {
    /// Keep the first highest-value deck strictly above the initial floor.
    Best { seed: f64 },
    /// Collect decks that can reach a clause's threshold under its structure-class cap.
    Band { ratio_cap: f64, max_decks: usize },
}

/// Why enumeration stopped before checking all relevant decks. This is not an exact-scoring certificate.
pub(super) enum SearchStop {
    CharacterLimit,
    UnsupportedCapFreeProof,
    CapFreeNodeLimit,
    OutputLimit,
}

pub(super) struct SearchOutcome {
    pub(super) decks: Vec<(f64, Sel)>,
    /// None means enumeration finished under the requested bounds; exact scoring may still be needed.
    pub(super) incomplete: Option<SearchStop>,
}

/// Search-wide legal entries and proof configuration. Own the cached tables so the traversal
/// can borrow their group/character slices without a self-referential context.
struct SearchSetup {
    best: bool,
    seed: f64,
    rho: f64,
    cap: usize,
    nu: usize,
    size: usize,
    nplain: usize,
    nk: usize,
    ki22: usize,
    kpos: Vec<u64>,
    set_id: usize,
    lt: std::rc::Rc<LegalTabs>,
    // Keep the cached-entry handle alive through traversal, as before extraction.
    legal_entries: std::rc::Rc<Vec<usize>>,
    fixed: f64,
    cfm: bool,
    cf_lim: usize,
    cfs: f64,
    sbm: u8,
    sb_rows: std::rc::Rc<Vec<Option<Vec<(f64, f64)>>>>,
    sb_bar: f64,
    sb_u: usize,
    sb_q: f64,
    lp: Vec<LandPre>,
    ccap: [f64; 4],
}

impl DeckSpace<'_> {
    /// Decks legal for trainees `us` (uma indices; `ids` their ids). `clauses`: ((trainee position in `us`,
    /// inheritance chance) pairs, threshold); a deck's clause value is the sum of its product values (fixed factor
    /// included) over those pairs. Best mode requires one clause and keeps its highest-value deck above `seed`
    /// (none if no deck beats it; ties: the first found). Band mode collects decks for which some clause has
    /// c * value >= threshold, c the deck's class cap (`cap_class`, `caps`) at most `ratio_cap`.
    /// Reaching `max_decks` ends the band immediately and reports `OutputLimit`, even if that was the last deck.
    pub(super) fn search(
        &self,
        model: &Model,
        us: &[usize],
        ids: &[u32],
        clauses: &[(Vec<(usize, f64)>, f64)],
        mode: SearchMode,
    ) -> SearchOutcome {
        let setup = match self.configure_search(model, us, ids, clauses, mode) {
            Ok(setup) => setup,
            Err(outcome) => return outcome,
        };
        self.search_with_bounds(model, us, clauses, setup)
    }
}
