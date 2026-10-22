//! The deck cache the simulator and the plan value careers through: per trainee her decks, each with its corner
//! tables. For a trainee and a fixed deck, P(all) per outfit is multilinear in the per-target inheritance chances
//! (derived; `corners`), so 2^nu exact evaluations per (trainee, deck) give it at any chances. Decks come from the
//! searches' fully checked decks (`Ref`, also kept for `rise`) and from deck searches where a bound says the cached
//! decks may fall short.
use super::{Env, Ref, SelectedDeck};
use crate::{UmaIndex, UmaTable};

/// A cached deck of one trainee and per outfit its corner table (P(all), every spark scale 1, at q_k in {0, 1} per
/// uncertain target; index bit k set = q_k = 0, as `corners` builds them from q0 = 0)
#[derive(Clone)]
pub(super) struct CDeck {
    pub(super) sel: SelectedDeck,
    pub(super) tab: Vec<Vec<f64>>,
}
/// The deck cache of the simulator and the plan: per trainee her decks with corner tables and her fully checked decks (`Ref`, for `rise`)
#[derive(Clone)]
pub(super) struct Cache {
    pub(super) decks: UmaTable<Vec<CDeck>>,
    pub(super) refs: UmaTable<Vec<Ref>>,
    pub tables: usize,
}

impl Cache {
    pub(super) fn n_decks(&self) -> usize {
        self.decks.iter().map(std::vec::Vec::len).sum()
    }

    pub(super) fn new(trainees: usize) -> Self {
        Self { decks: (0..trainees).map(|_| vec![]).collect(), refs: UmaTable(vec![vec![]; trainees]), tables: 0 }
    }

    /// Cache a trainee's deck and corner tables, unless that selection is already present.
    pub(super) fn add(&mut self, env: &mut Env, trainee: UmaIndex, sel: &SelectedDeck) {
        if self.decks[trainee].iter().any(|deck| {
            deck.sel.entries == sel.entries
                && deck.sel.plain_pals == sel.plain_pals
                && deck.sel.filler_cards == sel.filler_cards
        }) {
            return;
        }
        let tab = env.corners_kept(trainee, sel);
        self.tables += 1;
        self.decks[trainee].push(CDeck { sel: sel.clone(), tab });
    }

    /// Absorb a search's fully checked decks. A repeated deck still contributes its proof at the
    /// search's inheritance chances; deduplicating decks must not discard these `rise` anchors.
    pub(super) fn absorb(&mut self, env: &mut Env, refs: &UmaTable<Vec<Ref>>) {
        for (trainee, checked_decks) in refs.indexed() {
            for checked in checked_decks {
                self.add(env, trainee, &checked.deck);
                self.refs[trainee].push(checked.clone());
            }
        }
    }
}
