//! The deck enumeration (`DeckSpace::search`): node bounds, the exact pass with its caps and certificate.
use crate::deck::joint::{LandPre, MissFactors};
use crate::deck::space::{DeckSpace, LegalTables, SelectedDeck};
use crate::skills::DeckChances;
pub(crate) mod bounds;
mod config;
mod order;
mod prepare;
mod scratch;
mod traverse;
mod walk;

/// The two enumeration passes have different stopping rules and inputs.
#[derive(Clone, Copy)]
pub(crate) enum SearchMode {
    /// Keep the first highest-value deck strictly above the initial floor.
    Best { seed: f64 },
    /// Collect decks that can reach a clause's threshold under its structure-class cap.
    Band { ratio_cap: f64, max_decks: usize },
}

/// Why enumeration stopped before checking all relevant decks. This is not an exact-scoring certificate.
pub(crate) enum SearchStop {
    CharacterLimit,
    UnsupportedCapFreeProof,
    CapFreeNodeLimit,
    OutputLimit,
}

/// One trainee position and inheritance chance contributing to a clause.
#[derive(Clone, Copy)]
pub(crate) struct LineageChance {
    pub(crate) trainee: usize,
    pub(crate) inheritance: f64,
}

/// Sum the best outfit value at each lineage, then compare with the threshold.
pub(crate) struct SearchClause {
    pub(crate) lineages: Vec<LineageChance>,
    pub(crate) threshold: f64,
}

/// A selected or detached deck with its search value.
#[derive(Clone)]
pub(crate) struct ScoredDeck<Deck> {
    pub(crate) value: f64,
    pub(crate) deck: Deck,
}

pub(crate) struct SearchOutcome {
    pub(crate) decks: Vec<ScoredDeck<SelectedDeck>>,
    /// None means enumeration finished under the requested bounds; exact scoring may still be needed.
    pub(crate) incomplete: Option<SearchStop>,
}

/// Search-wide legal entries and proof configuration. Own the cached tables so the traversal
/// can borrow their group/character slices without a self-referential context.
pub(crate) struct SearchSetup {
    best: bool,
    seed: f64,
    rho: f64,
    cap: usize,
    target_count: usize,
    size: usize,
    plain_pal_count: usize,
    kind_count: usize,
    filler_kind_index: usize,
    kind_place_values: Vec<u64>,
    set_id: usize,
    legal_tables: std::rc::Rc<LegalTables>,
    // Keep the cached-entry handle alive through traversal.
    legal_entries: std::rc::Rc<Vec<usize>>,
    fixed: f64,
    cap_free: bool,
    cap_free_node_limit: usize,
    cap_free_check_scale: f64,
    state_bound_mode: u8,
    state_rows: std::rc::Rc<Vec<Option<Vec<MissFactors>>>>,
    state_bound_threshold: f64,
    bound_trainee: crate::uma::UmaIndex,
    bound_inheritance: f64,
    landing_factors: Vec<LandPre>,
    class_caps: [f64; 4],
}

impl DeckSpace<'_> {
    /// Decks legal for `trainees` and their `trainee_ids`. Each clause names lineage chances by trainee position and
    /// a threshold; its value is the sum of the deck's product values, including the fixed factor, over those lineages.
    /// Best mode requires one clause and keeps its highest-value deck above `seed`
    /// (none if no deck beats it; ties: the first found). Band mode collects decks for which some clause has
    /// c * value >= threshold, c the deck's class cap (`cap_class`, `caps`) at most `ratio_cap`.
    /// Reaching `max_decks` ends the band immediately and reports `OutputLimit`, even if that was the last deck.
    pub(crate) fn search(
        &self,
        chances: &DeckChances,
        trainees: &[crate::uma::UmaIndex],
        trainee_ids: &[u32],
        clauses: &[SearchClause],
        mode: SearchMode,
    ) -> SearchOutcome {
        let setup = match self.configure_search(chances, trainees, trainee_ids, clauses, mode) {
            Ok(setup) => setup,
            Err(outcome) => return outcome,
        };
        self.search_with_bounds(chances, trainees, clauses, setup)
    }
}
