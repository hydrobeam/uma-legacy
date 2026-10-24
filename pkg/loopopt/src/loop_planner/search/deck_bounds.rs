//! A complete small deck space lets each known deck compete against its own incumbent loop.
use super::bands::Pass;
use super::{Multi, SharedDeckKey, lin_of};
use crate::deck::chains::ChainEvents;
use crate::deck::enumerate::{SearchClause, SearchMode};
use crate::deck::joint::{JointQuery, joint_b};
use crate::deck::pdeck::PDeck;
use crate::deck::space::{DeckSpace, HintChances, SelectedDeck};
use crate::deck::state_bound::StateBoundMemo;
use crate::uma::{NONE, UmaIndex, UmaTable};

/// These limits bound the extra pruning work; incomplete enumerations fall back to the ordinary search.
const MAX_DECKS: usize = 65_536;
const MAX_STRUCTURES: usize = 50_000;

pub(super) struct DeckBound {
    pub(super) deck: PDeck,
    /// Retained to select legal plain pals when an undersupplied job needs every deck, including dominated seeds.
    pub(super) selection: SelectedDeck,
    key: SharedDeckKey,
    /// The chain events the bounds take: the most favourable any loop's copy of the deck can have
    /// (`DeckSpace::deck_events_most`). The catalogue's deck takes the first plain pals of all; a loop takes the first
    /// its members leave legal, which may raise the rate less, and every exact and product value rises with the rate.
    ceiling_events: ChainEvents,
    /// The hint chances at `ceiling_events` when they differ from the catalogue deck's own (None: `deck`'s).
    ceiling_odds: Option<HintChances>,
    /// Exact values at the upward-rounded chance grid; an empty row has not been evaluated yet.
    values: UmaTable<Vec<f64>>,
}
impl DeckBound {
    /// The hint chances (white, gold) the product prefilters take: at `ceiling_events`.
    fn ceiling_hint_chances(&self) -> (&[f64], &[f64]) {
        match &self.ceiling_odds {
            Some(HintChances { white, gold }) => (white, gold),
            None => (&self.deck.white_hint_chances, &self.deck.gold_hint_chances),
        }
    }
}

pub(super) struct ImprovingDecks {
    /// Membership only: the ordinary queue still determines solving order and ties.
    pub(super) structures: crate::FxSet<[UmaIndex; 4]>,
    decks: Vec<usize>,
}

impl Multi<'_> {
    /// Every loop that could improve a distinct result. For deck D the threshold is max(k-th, incumbent(D)).
    /// Each member heads the same number of lineages, so the mean of her values at her maximum possible chance
    /// bounds the loop. Ignoring card legality and required members here only enlarges the candidate set.
    pub(super) fn improving_structures(&self, bounds: &[DeckBound], pass: &Pass) -> Option<ImprovingDecks> {
        let mut candidates = crate::FxSet::default();
        let mut decks = vec![];
        for (index, bound) in bounds.iter().enumerate() {
            let threshold = pass.deck_best.get(&bound.key).map_or(pass.kth_best, |best| best.value.max(pass.kth_best));
            if !threshold.is_finite() {
                return None;
            }
            let (white_hint_chances, gold_hint_chances) = bound.ceiling_hint_chances();
            let mut product_values: Vec<f64> = self
                .pool
                .iter()
                .map(|&uma| {
                    self.fixed
                        * self.maximum_exact_ratio
                        * self.inputs.model.trainee_fast(
                            uma,
                            self.maximum_chances[uma],
                            white_hint_chances,
                            gold_hint_chances,
                        )
                })
                .collect();
            product_values.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let possible = [3, 4].into_iter().any(|size| {
                ((size == 3 && self.allow_three) || (size == 4 && self.allow_four))
                    && product_values.len() >= size
                    && product_values[..size].iter().sum::<f64>() * (1.0 + 1e-9) > threshold * size as f64
            });
            if !possible {
                continue;
            }
            let sources = self.ents_of(&bound.deck);
            let mut ranked: Vec<(f64, UmaIndex)> = self
                .pool
                .iter()
                .map(|&uma| {
                    let value = joint_b(
                        self.inputs.model,
                        JointQuery::unfolded(
                            self.inputs.model,
                            bound.ceiling_events,
                            uma,
                            &[self.maximum_chances[uma]],
                        ),
                        &sources.entries,
                        sources.filler_count,
                        self.requested_target_count,
                        false,
                    )[0]
                    .all;
                    (value, uma)
                })
                .collect();
            ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let possible = [3, 4].into_iter().any(|size| {
                ((size == 3 && self.allow_three) || (size == 4 && self.allow_four))
                    && ranked.len() >= size
                    && ranked[..size].iter().map(|entry| entry.0).sum::<f64>() * (1.0 + 1e-9) >= threshold * size as f64
            });
            if !possible {
                continue;
            }
            decks.push(index);
            for size in [3, 4] {
                if (size == 3 && !self.allow_three) || (size == 4 && !self.allow_four) {
                    continue;
                }
                let ceiling = StructureCeiling { ranked: &ranked, size, threshold: threshold * size as f64 };
                ceiling.collect(&mut candidates, &mut Vec::new(), 0, 0.0);
                if candidates.len() >= MAX_STRUCTURES {
                    return None;
                }
            }
        }
        candidates.retain(|structure| self.required_umas.iter().all(|required| structure.contains(required)));
        Some(ImprovingDecks { structures: candidates, decks })
    }
    /// Enumerate without trainee-specific merging or legality: a superset of every loop's deck identities.
    /// A partial catalogue cannot certify a cutoff or reduce the requested result count.
    pub(super) fn deck_bounds(&self, state_bounds: &mut StateBoundMemo) -> Option<Vec<DeckBound>> {
        if !self.shared || self.loop_limit == 1 || self.inputs.deck_job.pals.is_none() {
            return None;
        }
        let outcome = self.inputs.deck_space.search(
            state_bounds,
            &self.inputs.model.deck_chances(),
            &[],
            &[],
            &[SearchClause { lineages: vec![], threshold: f64::NEG_INFINITY }],
            SearchMode::All { max_decks: MAX_DECKS },
        );
        if outcome.incomplete.is_some() {
            return None;
        }
        let mut seen = crate::FxSet::default();
        Some(
            outcome
                .decks
                .into_iter()
                .filter_map(|selected| {
                    let space = self.inputs.deck_space;
                    let deck = space.pdeck(&selected.deck, &[]);
                    if !self.deck_has_loop(&deck) || !seen.insert(deck.id()) {
                        return None;
                    }
                    let key = SharedDeckKey::of(&deck);
                    let ceiling_events = space.deck_events_most(&selected.deck);
                    let ceiling_odds = (ceiling_events.key() != space.deck_events(&selected.deck, &[]).key())
                        .then(|| space.hint_chances_at(&selected.deck, ceiling_events));
                    Some(DeckBound {
                        deck,
                        selection: selected.deck,
                        key,
                        ceiling_events,
                        ceiling_odds,
                        values: self.inputs.data.umas.indices().map(|_| vec![]).collect(),
                    })
                })
                .collect(),
        )
    }

    /// Exclude identities that cannot be used by any allowed structure before capping the requested count.
    /// Plain pals are interchangeable, so their legality is checked as a count after selecting the members.
    fn deck_has_loop(&self, deck: &PDeck) -> bool {
        let legal = |uma| {
            let id = self.inputs.data.umas[uma].id;
            !deck.ids.iter().any(|card| self.card_of[card].blocks(id, crate::checks::get()))
        };
        if self.required_umas.iter().any(|&uma| !legal(uma)) {
            return false;
        }
        let size = if self.allow_three && self.required_umas.len() <= 3 { 3 } else { 4 };
        let mut required: Vec<u32> = self.required_umas.iter().map(|&uma| self.inputs.data.umas[uma].id).collect();
        let candidates: Vec<u32> = self
            .pool
            .iter()
            .filter(|&&uma| legal(uma) && !self.required_umas.contains(&uma))
            .map(|&uma| self.inputs.data.umas[uma].id)
            .collect();
        legal_members(
            self.inputs.deck_space,
            &candidates,
            &mut required,
            size - self.required_umas.len(),
            deck.plain_ids.len(),
        )
    }

    /// Tighten the same deck-specific bounds at this structure's pair ceilings. Exact odds are monotone in the
    /// inheritance chance; rounding each chance upward to the grid preserves an upper bound for every schedule.
    pub(super) fn shared_can_improve(
        &self,
        bounds: &mut [DeckBound],
        structure: [UmaIndex; 4],
        chances: &[f64; 12],
        pass: &Pass,
    ) -> bool {
        let lineages = lin_of(&structure);
        let indices: &[usize] = match &pass.improving {
            Some(improving) => &improving.decks,
            None => return true,
        };
        for &index in indices {
            let bound = &mut bounds[index];
            let threshold = pass.deck_best.get(&bound.key).map_or(pass.kth_best, |best| best.value.max(pass.kth_best));
            if !threshold.is_finite() {
                return true;
            }
            let (white_hint_chances, gold_hint_chances) = bound.ceiling_hint_chances();
            let product = lineages
                .iter()
                .enumerate()
                .map(|(l, lineage)| {
                    self.inputs.model.trainee_fast(
                        structure[lineage.trainee],
                        chances[l],
                        white_hint_chances,
                        gold_hint_chances,
                    )
                })
                .sum::<f64>()
                * self.fixed
                / lineages.len() as f64;
            if self.maximum_exact_ratio * product * (1.0 + 1e-9) < threshold {
                continue;
            }
            let mut ceiling = 0.0;
            for (l, lineage) in lineages.iter().enumerate() {
                let uma = structure[lineage.trainee];
                if bound.values[uma].is_empty() {
                    let sources = self.ents_of(&bound.deck);
                    bound.values[uma] = joint_b(
                        self.inputs.model,
                        JointQuery::unfolded(self.inputs.model, bound.ceiling_events, uma, &self.grid.chances),
                        &sources.entries,
                        sources.filler_count,
                        self.requested_target_count,
                        false,
                    )
                    .into_iter()
                    .map(|value| value.all)
                    .collect();
                }
                ceiling += bound.values[uma][self.grid.idx(chances[l])];
            }
            if ceiling / lineages.len() as f64 * (1.0 + 1e-9) >= threshold {
                return true;
            }
        }
        false
    }
}

/// Existence only: stop at the first member set with enough legal plain pals.
fn legal_members(
    space: &DeckSpace,
    candidates: &[u32],
    members: &mut Vec<u32>,
    remaining: usize,
    plain_pals: usize,
) -> bool {
    if candidates.len() < remaining || space.legal_plain(members).len() < plain_pals {
        return false;
    }
    if remaining == 0 {
        return true;
    }
    for i in 0..=candidates.len() - remaining {
        members.push(candidates[i]);
        let possible = legal_members(space, &candidates[i + 1..], members, remaining - 1, plain_pals);
        members.pop();
        if possible {
            return true;
        }
    }
    false
}

/// A deck's separable ceiling: the selected members' sum must reach `threshold`.
struct StructureCeiling<'bounds> {
    ranked: &'bounds [(f64, UmaIndex)],
    size: usize,
    threshold: f64,
}
impl StructureCeiling<'_> {
    fn collect(
        &self,
        candidates: &mut crate::FxSet<[UmaIndex; 4]>,
        selected: &mut Vec<UmaIndex>,
        start: usize,
        sum: f64,
    ) {
        let needed = self.size - selected.len();
        if needed == 0 {
            let mut structure = [NONE; 4];
            structure[..self.size].copy_from_slice(selected);
            structure[..self.size].sort_unstable();
            candidates.insert(structure);
            return;
        }
        for next in start..self.ranked.len().saturating_sub(needed - 1) {
            if (sum + self.ranked[next..next + needed].iter().map(|entry| entry.0).sum::<f64>()) * (1.0 + 1e-9)
                < self.threshold
                || candidates.len() >= MAX_STRUCTURES
            {
                break;
            }
            selected.push(self.ranked[next].1);
            self.collect(candidates, selected, next + 1, sum + self.ranked[next].0);
            selected.pop();
        }
    }
}
