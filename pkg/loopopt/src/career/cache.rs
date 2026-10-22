//! The deck cache the simulator and the plan value careers through: per trainee her decks, each with its corner
//! tables. For a trainee and a fixed deck, P(all) per outfit is multilinear in the per-target inheritance chances
//! (derived; `corners`), so 2^nu exact evaluations per (trainee, deck) give it at any chances. Decks come from the
//! searches' fully checked decks (`Ref`, also kept for `rise`) and from deck searches where a bound says the cached
//! decks may fall short.
use crate::career::search::Ref;
use crate::career::{CareerSetup, DeckSearchMemo, SearchState};
use crate::deck::space::SelectedDeck;
use crate::uma::{UmaIndex, UmaTable};
/// A cached deck of one trainee and per outfit its corner table (P(all), every spark scale 1, at q_k in {0, 1} per
/// uncertain target; index bit k set = q_k = 0, as `corners` builds them from q0 = 0)
#[derive(Clone)]
pub(super) struct CachedDeck {
    pub(super) deck: SelectedDeck,
    /// Per outfit of the trainee, the deck's corner table.
    pub(super) tables: Vec<Vec<f64>>,
}
/// The deck cache of the simulator and the plan: per trainee her decks with corner tables and her fully checked decks (`Ref`, for `rise`)
#[derive(Clone)]
pub(super) struct Cache {
    /// Per trainee, her cached decks in the order they were added.
    pub(super) decks: UmaTable<Vec<CachedDeck>>,
    /// Per trainee, the searches' fully checked decks, repeats kept (each is a `rise` anchor at its own chances).
    pub(super) refs: UmaTable<Vec<Ref>>,
    /// How many decks (with their corner tables) have been added: a work counter.
    pub tables: usize,
}

impl Cache {
    /// Cached decks over all trainees.
    pub(super) fn deck_count(&self) -> usize {
        self.decks.iter().map(std::vec::Vec::len).sum()
    }

    pub(super) fn new(trainees: usize) -> Self {
        Self { decks: (0..trainees).map(|_| vec![]).collect(), refs: UmaTable(vec![vec![]; trainees]), tables: 0 }
    }

    /// Cache a trainee's deck and corner tables, unless that selection is already present (the tables are built in
    /// her folded buffer, `CareerSetup::corners_kept`, and kept in the memo).
    pub(super) fn add(
        &mut self,
        setup: &CareerSetup,
        state: &mut SearchState,
        memo: &mut DeckSearchMemo,
        trainee: UmaIndex,
        deck: &SelectedDeck,
    ) {
        if self.decks[trainee].iter().any(|cached| {
            cached.deck.entries == deck.entries
                && cached.deck.plain_pals == deck.plain_pals
                && cached.deck.filler_cards == deck.filler_cards
        }) {
            return;
        }
        let tables = setup.corners_kept(state, memo, trainee, deck);
        self.tables += 1;
        self.decks[trainee].push(CachedDeck { deck: deck.clone(), tables });
    }

    /// Absorb a search's fully checked decks. A repeated deck still contributes its proof at the
    /// search's inheritance chances; deduplicating decks must not discard these `rise` anchors.
    pub(super) fn absorb(
        &mut self,
        setup: &CareerSetup,
        state: &mut SearchState,
        memo: &mut DeckSearchMemo,
        refs: &UmaTable<Vec<Ref>>,
    ) {
        for (trainee, checked_decks) in refs.indexed() {
            for checked in checked_decks {
                self.add(setup, state, memo, trainee, &checked.deck);
                self.refs[trainee].push(checked.clone());
            }
        }
    }
}
