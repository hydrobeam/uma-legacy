//! Exact shared-deck scans and their cached results, seeded from legal skyline decks.
//! Loop enumeration and certification across structures belong to `bands`.
use super::{ChanceKey, DeckSources, Multi, SharedQueryKey, lin_of};
use super::{LoopSearchState, Tally};
use crate::Instant;
use crate::data::Data;
use crate::deck::caps::src_class;
use crate::deck::enumerate::{BandBound, LineageChance, ScoredDeck, SearchClause, SearchMode, SearchOutcome};
use crate::deck::joint::upper::upper_na;
use crate::deck::joint::{JointQuery, joint_b};
use crate::deck::pdeck::{DeckId, PDeck};
use crate::deck::sources::{SourceEntry, TargetSources};
use crate::loop_planner::schedules::members;
use crate::uma::UmaIndex;
use std::rc::Rc;

/// A loop's shared-deck list: (exact sum over its lineages, deck), best first
pub(super) type SharedDecks = Rc<Vec<ScoredDeck<PDeck>>>;

/// A shared-deck cache result, valid below its cap and above its floor.
pub(super) struct SharedResult {
    certified_cap: f64,
    floor: f64,
    decks: SharedDecks,
}

/// One loop's shared-deck scoring: its members' ids, (trainee, chance) per lineage and each member's chances, the
/// floor, and the decks scored so far
struct SharedScan {
    members: [UmaIndex; 4],
    member_count: usize,
    member_ids: Vec<u32>,
    lineages: Vec<LineageChance>,
    chances_by_member: Vec<Vec<f64>>,
    floor: f64,
    loop_limit: usize,
    scored_decks: crate::FxSet<DeckId>,
    decks: Vec<ScoredDeck<PDeck>>,
}
impl SharedScan {
    fn new(data: &Data, structure: [UmaIndex; 4], lineage_chances: &[f64], floor: f64, loop_limit: usize) -> Self {
        let structure_lineages = lin_of(&structure);
        let member_slice = members(&structure);
        let member_count = member_slice.len();
        let member_ids: Vec<u32> = member_slice.iter().map(|&uma| data.umas[uma].id).collect();
        let lineages: Vec<LineageChance> = (0..structure_lineages.len())
            .map(|l| LineageChance { trainee: structure_lineages[l].trainee, inheritance: lineage_chances[l] })
            .collect();
        let chances_by_member: Vec<Vec<f64>> = (0..member_count)
            .map(|member| {
                (0..structure_lineages.len())
                    .filter(|&l| structure_lineages[l].trainee == member)
                    .map(|l| lineage_chances[l])
                    .collect()
            })
            .collect();
        Self {
            members: structure,
            member_count,
            member_ids,
            lineages,
            chances_by_member,
            floor,
            loop_limit,
            scored_decks: crate::FxSet::default(),
            decks: vec![],
        }
    }
    /// the threshold a deck must reach: the floor, or the loop's k-th best so far when that is higher
    fn need(&self) -> f64 {
        let mut values: Vec<f64> = self.decks.iter().map(|deck| deck.value).collect();
        values.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let kth_value = if values.len() >= self.loop_limit { values[self.loop_limit - 1] } else { f64::NEG_INFINITY };
        let threshold = self.floor.max(kth_value);
        if threshold.is_finite() { threshold } else { values.last().copied().unwrap_or(f64::NEG_INFINITY) }
    }
}

impl Multi<'_> {
    // ---- shared deck ----
    // With one deck for the whole structure, its value is the best exact sum over its lineages among decks legal
    // for every member. Per structure: every such deck whose exact sum is at least `floor` (the k-th best distinct
    // deck so far, times the lineage count) or among its own best `top_decks`, under cap `cap`; sorted, best first.
    // Ceilings
    // stay the per-uma ones (each trainee's best deck bounds her share of any shared deck).
    pub(super) fn exact_shared(
        &self,
        state: &mut LoopSearchState,
        structure: [UmaIndex; 4],
        lineage_chances: &[f64],
        floor: f64,
        cap: f64,
    ) -> SharedDecks {
        let key = SharedQueryKey { members: structure, chances: ChanceKey::new(lineage_chances) };
        if let Some(SharedResult { certified_cap, floor: cached_floor, decks }) = state.shared_cache.get(&key)
            && *certified_cap >= cap
            && *cached_floor <= floor
        {
            return Rc::clone(decks);
        }
        let mut scan = SharedScan::new(self.inputs.data, structure, lineage_chances, floor, self.loop_limit);
        self.seed_shared(state, &mut scan);
        let complete = if cap > 1.0 && !scan.decks.is_empty() {
            self.band_shared(&mut state.state_bounds, &mut state.tally, &mut scan, cap)
        } else {
            true
        };
        let threshold = scan.need();
        let mut list = scan.decks;
        list.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());
        list.retain(|deck| deck.value >= threshold);
        let decks: SharedDecks = Rc::new(list);
        if complete {
            state.shared_cache.insert(key, SharedResult { certified_cap: cap, floor, decks: Rc::clone(&decks) });
        }
        decks
    }

    /// Scores `deck` exactly over the loop's lineages (once per loop) and lists it
    fn score_shared(&self, tally: &mut Tally, scan: &mut SharedScan, deck: &PDeck) {
        if !scan.scored_decks.insert(deck.id()) {
            return;
        }
        let (model, structure) = (self.inputs.model, scan.members);
        let started = Instant::now();
        let DeckSources { entries, filler_count } = self.ents_of(deck);
        let deck_class = src_class(entries.iter().map(|entry| &*entry.sources));
        let mut value_sum = 0.0;
        for member in 0..scan.member_count {
            let results = joint_b(
                model,
                JointQuery::unfolded(model, self.inputs.deck_job, structure[member], &scan.chances_by_member[member]),
                &entries,
                filler_count,
                self.requested_target_count,
                false,
            );
            for (i, &q) in scan.chances_by_member[member].iter().enumerate() {
                let product_value = self.fixed
                    * model.trainee_fast(structure[member], q, &deck.white_hint_chances, &deck.gold_hint_chances);
                if product_value > 0.0 {
                    tally.ratio_at(
                        deck_class,
                        results[i].all / product_value,
                        self.inputs.data.umas[structure[member]].id,
                        deck,
                    );
                }
                value_sum += results[i].all;
            }
        }
        tally.score_seconds += started.elapsed().as_secs_f64();
        tally.scored_decks += 1;
        tally.spend(800);
        scan.decks.push(ScoredDeck { value: value_sum, deck: deck.clone() });
    }

    /// seed: the loop's best `top_decks` + 2 skyline decks legal for every member by product value (or its best legal
    /// deck when none is on the skyline)
    fn seed_shared(&self, state: &mut LoopSearchState, scan: &mut SharedScan) {
        let (model, space, fixed, structure) = (self.inputs.model, self.inputs.deck_space, self.fixed, scan.members);
        let started = Instant::now();
        let pairs = &scan.lineages;
        let value_of = |white_hint_chances: &[f64], gold_hint_chances: &[f64]| {
            fixed
                * pairs
                    .iter()
                    .map(|&LineageChance { trainee, inheritance: q }| {
                        model.trainee_fast(structure[trainee], q, white_hint_chances, gold_hint_chances)
                    })
                    .sum::<f64>()
        };
        let ids = &scan.member_ids;
        let seeds = self.sky_top(&mut state.skyline_bound_scratch, self.loop_limit + 2, &value_of, &|deck| {
            ids.iter().all(|&id| !deck.blocks(id, &self.card_of, crate::checks::get()))
        });
        state.tally.seed_seconds += started.elapsed().as_secs_f64();
        for &(_, j) in &seeds {
            self.score_shared(&mut state.tally, scan, &self.inputs.skyline[j]);
        }
        if scan.decks.is_empty() {
            let SearchOutcome { decks, .. } = space.search(
                &mut state.state_bounds,
                &model.deck_chances(),
                members(&structure),
                &scan.member_ids,
                &[SearchClause { lineages: scan.lineages.clone(), threshold: 0.0 }],
                SearchMode::Best { seed: f64::NEG_INFINITY },
            );
            if let Some(ScoredDeck { value: _, deck: best }) = decks.first() {
                let deck = space.pdeck(best, &scan.member_ids);
                self.score_shared(&mut state.tally, scan, &deck);
            }
        }
    }

    /// Every legal deck that might reach the loop's threshold under cap `cap`, scored (after the proven bound); false
    /// (and the pass aborted) when a deck search was cut short or the budget ran out
    fn band_shared(
        &self,
        state_bounds: &mut crate::deck::state_bound::StateBoundMemo,
        tally: &mut Tally,
        scan: &mut SharedScan,
        cap: f64,
    ) -> bool {
        let (model, space, structure) = (self.inputs.model, self.inputs.deck_space, scan.members);
        let mut complete = true;
        tally.band_decks += 1;
        let started = Instant::now();
        let nodes_before = space.nodes.get();
        let SearchOutcome { decks: mut candidates, incomplete, .. } = space.search(
            state_bounds,
            &model.deck_chances(),
            members(&structure),
            &scan.member_ids,
            &[SearchClause { lineages: scan.lineages.clone(), threshold: scan.need() }],
            SearchMode::Band { ratio_cap: cap, max_decks: space.band_cap(), bound: BandBound::Capped },
        );
        tally.spend(2 * (space.nodes.get() - nodes_before) + 5 * candidates.len() + 10_000);
        tally.band_seconds += started.elapsed().as_secs_f64();
        if incomplete.is_some() {
            tally.cap_hit = true;
            complete = false;
        }
        candidates.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());
        let mut seen: crate::FxSet<(Vec<usize>, usize, usize)> = crate::FxSet::default();
        if complete {
            for ScoredDeck { value, deck: selected } in &candidates {
                let threshold = scan.need();
                if cap * value * (1.0 + 1e-9) < threshold {
                    break;
                }
                if tally.work > tally.budget {
                    complete = false;
                    break;
                }
                if space.sel_cap(selected, cap) * value * (1.0 + 1e-9) < threshold {
                    continue;
                }
                if !seen.insert((selected.entries.clone(), selected.plain_pals, selected.filler_cards)) {
                    continue;
                }
                let entries: Vec<SourceEntry<&TargetSources>> = selected
                    .entries
                    .iter()
                    .map(|&j| SourceEntry { sources: &space.sources[j], kind: space.kind[j] })
                    .collect();
                let bound_started = Instant::now();
                let upper_bound: Option<f64> = (0..scan.member_count)
                    .map(|member| {
                        upper_na(
                            model,
                            JointQuery::unfolded(
                                model,
                                self.inputs.deck_job,
                                structure[member],
                                &scan.chances_by_member[member],
                            ),
                            &entries,
                            selected.filler_cards,
                        )
                        .map(|bounds| bounds.iter().sum::<f64>())
                    })
                    .sum();
                tally.bound_seconds += bound_started.elapsed().as_secs_f64();
                if let Some(upper_bound) = upper_bound {
                    tally.spend(100);
                    if upper_bound * (1.0 + 1e-9) < threshold {
                        tally.bound_rejections += 1;
                        continue;
                    }
                }
                let deck = space.pdeck(selected, &scan.member_ids);
                self.score_shared(tally, scan, &deck);
            }
        }
        if !complete {
            tally.aborted = true;
        }
        complete
    }
}
