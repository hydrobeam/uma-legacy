//! The deck enumeration (`DeckSpace::search`): node bounds, the exact pass with its caps and certificate.
//!
//! The entry is `DeckSpace::search` (here, with the records a search takes and returns): `prepare`'s `configure_search`
//! sets the pass up and `search_with_bounds` runs it.
//! - `prepare`: search legality and the shared bounds the enumeration uses; the two calls above.
//! - `config`: the legal card-count configurations and their bounds.
//! - `order`: the order queued configurations are visited in.
//! - `traverse`: the traversal of configurations, with their borrowed-card and state bounds.
//! - `walk`: the depth-first walk of the decks of one configuration.
//! - `bounds`: outfit bounds and ratio prefixes.
//! - `scratch`: the reusable buffers of configuration and node bounds.
use crate::deck::joint::LandPre;
use crate::deck::space::{DeckSpace, LegalTables, SelectedDeck, StateRows};
use crate::skills::DeckChances;
pub(crate) mod bounds;
mod config;
mod order;
mod prepare;
mod scratch;
mod traverse;
mod walk;

/// Enumeration passes have different stopping rules and inputs.
#[derive(Clone, Copy)]
pub(crate) enum SearchMode {
    /// Enumerate the unreduced deck space, up to `max_decks`: empty trainee slices and one empty clause with a -inf
    /// threshold. Keeping raw entries preserves every possible loop's deck identities.
    All { max_decks: usize },
    /// Keep the first highest-value deck strictly above the initial floor.
    Best { seed: f64 },
    /// `Best` for the career's capped deck search (one trainee at one chance): a deck's value is max(P, B / rho), P
    /// its product value and B the nested-chain bound of a deck with a nested chain, any class, where its
    /// configuration's cap passes rho (`deck::nested`: proven for a nested card alone feeding every chain-varying
    /// target, assumed beside other feeders, `multi_feeder_cap`), else P; so rho times the best bounds every deck's
    /// exact value under the class caps and those assumptions (`ruled`, the seed, `Bounded`).
    BestCovered { seed: f64, rho: f64 },
    /// Collect decks that can reach a clause's threshold under its structure-class cap, pruned also by `bound`.
    Band { ratio_cap: f64, max_decks: usize, bound: BandBound },
}

/// What a band search prunes by beside its class caps. The state bounds need one clause of one lineage (one trainee
/// at one chance: the career's searches); any other band runs as `Capped`.
#[derive(Clone, Copy)]
pub(crate) enum BandBound {
    /// The class caps alone.
    Capped,
    /// The state-conditioned bound beside the caps, in `mode` (`Checks::sbound`, `SBOUND`: 3 at each
    /// configuration's root, 1 also at depth 1, 2 at every node without the caps, 0 off); the career's band, so where
    /// a deck with a nested chain may be reached its proven bound joins the caps (`deck::nested`).
    StateBound { mode: u8 },
    /// The cap-free band (`CareerSetup::best_deck_cf`): two proven bounds at every node and leaf are its only prunes,
    /// no cap; it stops (`CapFreeNodeLimit`) once the space's node count (`DeckSpace::nodes`) passes `node_limit`.
    CapFree { node_limit: usize },
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
    /// The band was built without the class caps (state-bound mode 2, or cap-free): every deck in it may reach the
    /// threshold, none was cut by a cap. False on a search that stopped before its setup.
    pub(crate) uncapped: bool,
    /// It pruned decks with a nested chain beside other chain feeders or an uncertified group
    /// (`ConfigurationQueue::multi`): its proof rests on the nested bound taken for them (`multi_feeder_cap`).
    pub(crate) multi: bool,
    /// It pruned decks of a shape left under the assumed class cap (`ConfigurationQueue::assumed`): its proof rests on
    /// that cap.
    pub(crate) assumed: bool,
}

/// Search-wide legal entries and proof configuration. Own the cached tables so the traversal
/// can borrow their group/character slices without a self-referential context.
pub(crate) struct SearchSetup {
    best: bool,
    seed: f64,
    rho: f64,
    /// the nested-chain bounds (`deck::nested`) are on: the career's band (`BandBound::StateBound`) or product search
    /// (`BestCovered`, its rho in `cover_rho`, else 0), one trainee at one chance, not cap-free
    nested_bounds: bool,
    cover_rho: f64,
    cap: usize,
    target_count: usize,
    size: usize,
    plain_pal_count: usize,
    /// per plain-pal count, how many of those raise the chain rate (`DeckSpace::plain_rate_pals`)
    plain_rate_pals: Vec<usize>,
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
    state_rows: std::rc::Rc<StateRows>,
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
        state_bounds: &mut crate::deck::state_bound::StateBoundMemo,
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
        self.search_with_bounds(state_bounds, chances, trainees, clauses, setup)
    }
}
