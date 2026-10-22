//! Per-uma exact values, chance-grid ceilings and skyline clusters, including shared product ceilings.
//! These tables bound deck and loop searches; they do not enumerate loop structures or report results.
use super::{CardEntryKey, ChanceKey, DeckSources, ExactQueryKey, Multi, SelectionKey, lin_of};
use super::{LoopSearchState, Tally};
use crate::Instant;
use crate::checks::UCHK;
use crate::data::{Data, Sched, dot};
use crate::deck::caps::src_class;
use crate::deck::enumerate::{BandBound, LineageChance, ScoredDeck, SearchClause, SearchMode, SearchOutcome};
use crate::deck::joint::upper::upper_na;
use crate::deck::joint::{JointChances, JointQuery, joint, joint_b};
use crate::deck::pdeck::{DeckId, PDeck};
use crate::deck::sources::{SourceEntry, Sources, TargetSources};
use crate::deck::space::HintCoordinate;
use crate::loop_planner::dominance::{DeckCluster, DeckEval, k_filter, split};
use crate::loop_planner::schedules::{members, pair_ceilings};
use crate::skills::{Model, OutfitOption};
use crate::uma::{UmaIndex, UmaTable};
use std::borrow::Cow;
use std::rc::Rc;

/// Points on the chance grid of the ceiling tables
pub(super) const NG: usize = 64;
/// A grid cell's exact value and the cap under which it was certified.
#[derive(Clone, Copy)]
pub(super) struct ExactCeiling {
    pub(super) value: f64,
    pub(super) certified_cap: f64,
}

/// The chance grid the ceiling tables are kept on: NG points from the floor to the highest chance any uma reaches
pub(super) struct Grid {
    minimum_chance: f64,
    step: f64,
    chances: Vec<f64>,
}
impl Grid {
    pub(super) fn new(lowest: f64, highest: f64) -> Self {
        let step = if highest > lowest { (highest - lowest) / (NG - 1) as f64 } else { 0.0 };
        let chances: Vec<f64> = (0..NG).map(|i| if i == NG - 1 { highest } else { lowest + step * i as f64 }).collect();
        Self { minimum_chance: lowest, step, chances }
    }
    /// the first grid point at or above q (the last when the grid is one point)
    pub(super) fn idx(&self, q: f64) -> usize {
        if self.step == 0.0 {
            return NG - 1;
        }
        let mut i = (((q - self.minimum_chance) / self.step).ceil().max(0.0) as usize).min(NG - 1);
        while self.chances[i] < q && i < NG - 1 {
            i += 1;
        }
        i
    }
}

pub(super) struct Best {
    pub(super) sum: f64,
    pub(super) deck: Option<PDeck>,
    pub(super) lineage_chances: Vec<JointChances>,
}
// per (uma, chances): the best found, decks already scored or bounded, and the cap it is exact under
pub(super) struct ExactCacheEntry {
    best: Option<ScoredDeck<PDeck>>,
    /// Detached decks already scored exactly (including skyline seeds).
    scored_decks: crate::FxSet<DeckId>,
    /// Band selections already scored or rejected by the proven bound.
    checked_selections: crate::FxSet<SelectionKey>,
    /// Highest completed cap; -1 means unseeded, 1 means seeded only.
    certified_cap: f64,
    reported_best: Option<Rc<Best>>,
}

/// Each pair's race-win ceiling: the fixed schedules' values where the job fixes them (0 without races), else the
/// pair ceilings
pub(super) fn pair_table(
    data: &Data,
    pool: &[UmaIndex],
    fixed_s: Option<&UmaTable<Option<Sched>>>,
    no_race: bool,
) -> UmaTable<UmaTable<f64>> {
    let n = data.umas.len();
    match fixed_s {
        Some(fixed_schedules) => {
            let win_chances: UmaTable<Option<Vec<f64>>> = fixed_schedules
                .indexed()
                .map(|(i, schedule)| schedule.as_ref().map(|schedule| data.win_chances(&data.umas[i], schedule)))
                .collect();
            let mut table = UmaTable(vec![UmaTable(vec![0.0f64; n]); n]);
            if !no_race {
                for &a in pool {
                    for &b in pool {
                        if a != b {
                            table[a][b] = dot(win_chances[a].as_ref().unwrap(), win_chances[b].as_ref().unwrap());
                        }
                    }
                }
            }
            table
        }
        None => pair_ceilings(data, pool),
    }
}

// A uma's value depends only on the targets some outfit of hers lacks: (g, h) = (P(gold), P(any hint)) where
// an outfit lacks the skill, g alone where her outfits have at most the white. Decks are thinned per class of
// umas with the same coordinates: a deck another beats on all of them never wins for those umas.
fn ucoords(model: &Model, uma: UmaIndex, target_count: usize) -> Vec<HintCoordinate> {
    let mut coordinates = vec![];
    for k in 0..target_count {
        if model.outfit_options[uma].iter().any(|OutfitOption { coverage, .. }| coverage[k] == 0) {
            coordinates.push(HintCoordinate { target: k, gold: true });
            coordinates.push(HintCoordinate { target: k, gold: false });
        } else if model.outfit_options[uma].iter().any(|OutfitOption { coverage, .. }| coverage[k] == 1) {
            coordinates.push(HintCoordinate { target: k, gold: true });
        }
    }
    coordinates
}
/// The decks of `list` no other beats on the coordinates `coordinates`, by index, ascending
fn thin(list: &[PDeck], coordinates: &[HintCoordinate]) -> Vec<usize> {
    if list.is_empty() {
        return vec![];
    }
    if coordinates.is_empty() {
        return vec![0];
    }
    let tagged: Vec<DeckEval> = list
        .iter()
        .enumerate()
        .map(|(i, deck)| {
            let dominance_values: Vec<f64> = coordinates
                .iter()
                .map(|&HintCoordinate { target: k, gold }| {
                    if gold {
                        deck.gold_hint_chances[k]
                    } else {
                        1.0 - (1.0 - deck.white_hint_chances[k]) * (1.0 - deck.gold_hint_chances[k])
                    }
                })
                .collect();
            let coordinate_sum = dominance_values.iter().sum();
            DeckEval {
                pals: 0,
                borrowed: None,
                cards: vec![i],
                ends: vec![],
                variants: vec![],
                plain: 0,
                filler: 0,
                white_hint_chances: vec![],
                gold_hint_chances: vec![],
                dominance_values,
                coordinate_sum,
            }
        })
        .collect();
    let mut kept: Vec<usize> =
        k_filter(tagged, 1, coordinates.len()).into_list().into_iter().map(|eval| eval.cards[0]).collect();
    kept.sort_unstable();
    kept
}
/// The pool's classes of umas with the same coordinates: each class's skyline decks, and each uma's class
pub(super) fn classes(
    model: &Model,
    pool: &[UmaIndex],
    skyline: &[PDeck],
    target_count: usize,
    n: usize,
) -> (Vec<Vec<usize>>, UmaTable<usize>) {
    // the full coordinate set needs no thinning: the skyline is already thin on it
    let full: Vec<HintCoordinate> = (0..target_count)
        .flat_map(|k| [HintCoordinate { target: k, gold: true }, HintCoordinate { target: k, gold: false }])
        .collect();
    let mut class_of_coordinates: crate::FxMap<Vec<HintCoordinate>, usize> = crate::FxMap::default();
    let mut class_sky: Vec<Vec<usize>> = vec![];
    let mut class_by_uma = UmaTable(vec![usize::MAX; n]);
    for &uma in pool {
        let coordinates = ucoords(model, uma, target_count);
        let class_index = if let Some(&class) = class_of_coordinates.get(&coordinates) {
            class
        } else {
            class_sky.push(if coordinates == full {
                (0..skyline.len()).collect()
            } else {
                thin(skyline, &coordinates)
            });
            class_of_coordinates.insert(coordinates, class_sky.len() - 1);
            class_sky.len() - 1
        };
        class_by_uma[uma] = class_index;
    }
    (class_sky, class_by_uma)
}
/// Clusters of the whole skyline (one cluster of every deck without uncertain targets); none with per-uma decks
pub(super) fn full_clusters(skyline: &[PDeck], target_count: usize, per_uma: bool) -> Vec<DeckCluster> {
    if per_uma {
        vec![]
    } else {
        let mut members: Vec<usize> = (0..skyline.len()).collect();
        let mut out = vec![];
        if target_count > 0 && !members.is_empty() {
            split(skyline, &mut members, target_count, &mut out);
        } else {
            out.push(DeckCluster { max_white_chances: vec![], max_gold_chances: vec![], members });
        }
        out
    }
}

/// Clusters for each class retain the skyline's ordering before the median splits.
pub(super) fn class_clusters(
    skyline: &[PDeck],
    class_decks: &[Vec<usize>],
    target_count: usize,
) -> Vec<Vec<DeckCluster>> {
    class_decks
        .iter()
        .map(|decks| {
            let mut indices = decks.clone();
            let mut clusters = vec![];
            if target_count > 0 && !indices.is_empty() {
                split(skyline, &mut indices, target_count, &mut clusters);
            }
            clusters
        })
        .collect()
}

impl<'data> Multi<'data> {
    fn uview(&self, uma: UmaIndex) -> &Vec<usize> {
        &self.class_decks[self.class_by_trainee[uma]]
    }

    // ---- product-odds ceilings ----

    /// max(init, max over u's class of value_of(deck)), with bound(cluster maxima) >= value_of(member) for every member
    /// (the clusters' bounds in a reused buffer, popped from a heap in the stable sort's order: `ByBound`)
    fn bb_max(
        &self,
        scratch: &mut Vec<ByBound>,
        uma: UmaIndex,
        init: f64,
        value_of: &dyn Fn(&[f64], &[f64]) -> f64,
    ) -> f64 {
        let clusters = &self.class_clusters[self.class_by_trainee[uma]];
        let skyline = &self.inputs.skyline;
        if clusters.is_empty() {
            return self
                .uview(uma)
                .iter()
                .map(|&j| value_of(&skyline[j].white_hint_chances, &skyline[j].gold_hint_chances))
                .fold(init, f64::max);
        }
        let mut bounds = std::mem::take(scratch);
        bounds.clear();
        bounds.extend(clusters.iter().enumerate().map(|(i, cluster)| ByBound {
            bound: value_of(&cluster.max_white_chances, &cluster.max_gold_chances),
            cluster: i,
        }));
        let mut heap = std::collections::BinaryHeap::from(bounds);
        let mut best = init;
        while let Some(ByBound { bound, cluster: i }) = heap.pop() {
            if bound * (1.0 + 1e-9) < best {
                break;
            }
            for &j in &clusters[i].members {
                best = best.max(value_of(&skyline[j].white_hint_chances, &skyline[j].gold_hint_chances));
            }
        }
        *scratch = heap.into_vec();
        best
    }

    /// B[u][i], memoised in `relaxed_ceiling_cache` at first need.
    pub(super) fn btab(&self, state: &mut LoopSearchState, uma: UmaIndex, i: usize) -> f64 {
        let value = state.relaxed_ceiling_cache[uma][i];
        if !value.is_nan() {
            return value;
        }
        let (model, skyline) = (self.inputs.model, &self.inputs.skyline);
        let q = self.grid.chances[i];
        let init = model.trainee_fast(uma, q, &self.no_hint_chances, &self.no_hint_chances);
        let value = if (0.0..=1.0).contains(&q) {
            self.bb_max(&mut state.cluster_bound_scratch, uma, init, &|white_hint_chances, gold_hint_chances| {
                model.trainee_fast(uma, q, white_hint_chances, gold_hint_chances)
            })
        } else {
            self.uview(uma)
                .iter()
                .map(|&j| model.trainee_fast(uma, q, &skyline[j].white_hint_chances, &skyline[j].gold_hint_chances))
                .fold(init, f64::max)
        };
        state.relaxed_ceiling_cache[uma][i] = value;
        value
    }

    /// The best `count` skyline decks by `value_of` among those passing `ok` (cluster maxima bound their members, best
    /// bound first; `value_of` must rise with every odds coordinate)
    // (as `bb_max`: the clusters in the stable sort's order, equal bounds by index, from a heap over a reused buffer)
    pub(super) fn sky_top(
        &self,
        scratch: &mut Vec<ByBound>,
        count: usize,
        value_of: &dyn Fn(&[f64], &[f64]) -> f64,
        ok: &dyn Fn(&PDeck) -> bool,
    ) -> Vec<(f64, usize)> {
        let (skyline, clusters, target_count) = (&self.inputs.skyline, &self.skyline_clusters, self.target_count);
        let mut bounds = std::mem::take(scratch);
        bounds.clear();
        bounds.extend(clusters.iter().enumerate().map(|(i, cluster)| ByBound {
            bound: if target_count > 0 {
                value_of(&cluster.max_white_chances, &cluster.max_gold_chances)
            } else {
                f64::INFINITY
            },
            cluster: i,
        }));
        let mut heap = std::collections::BinaryHeap::from(bounds);
        let mut top: Vec<(f64, usize)> = vec![];
        while let Some(ByBound { bound, cluster: cluster_index }) = heap.pop() {
            if top.len() >= count && bound * (1.0 + 1e-9) < top[count - 1].0 {
                break;
            }
            for &j in &clusters[cluster_index].members {
                if !ok(&skyline[j]) {
                    continue;
                }
                let value = value_of(&skyline[j].white_hint_chances, &skyline[j].gold_hint_chances);
                if top.len() < count || value > top[count - 1].0 {
                    let position = top.iter().position(|entry| value > entry.0).unwrap_or(top.len());
                    top.insert(position, (value, j));
                    top.truncate(count);
                }
            }
        }
        *scratch = heap.into_vec();
        top
    }

    /// Shared-deck ceiling: the best single skyline deck (legal or not) over the structure's lineages, product odds.
    pub(super) fn relaxed_shared(
        &self,
        state: &mut LoopSearchState,
        structure: [UmaIndex; 4],
        lineage_chances: &[f64; 12],
    ) -> f64 {
        let model = self.inputs.model;
        let lineages = lin_of(&structure);
        let value_of = |white_hint_chances: &[f64], gold_hint_chances: &[f64]| {
            (0..lineages.len())
                .map(|l| {
                    model.trainee_fast(
                        structure[lineages[l].trainee],
                        lineage_chances[l],
                        white_hint_chances,
                        gold_hint_chances,
                    )
                })
                .sum::<f64>()
        };
        self.sky_top(&mut state.skyline_bound_scratch, 1, &value_of, &|_| true).first().map_or(0.0, |best| best.0)
            * self.fixed
            / lineages.len() as f64
            * (1.0 + 1e-12)
    }

    /// ceiling with each uma's best skyline deck (legal or not), the same deck across her lineages
    pub(super) fn relaxed(
        &self,
        state: &mut LoopSearchState,
        structure: [UmaIndex; 4],
        lineage_chances: &[f64; 12],
    ) -> f64 {
        let (model, skyline) = (self.inputs.model, &self.inputs.skyline);
        let mut total = 0.0;
        let lineages = lin_of(&structure);
        for member in 0..members(&structure).len() {
            let member_chances: Vec<f64> =
                (0..lineages.len()).filter(|&l| lineages[l].trainee == member).map(|l| lineage_chances[l]).collect();
            let init = member_chances
                .iter()
                .map(|&q| model.trainee_fast(structure[member], q, &self.no_hint_chances, &self.no_hint_chances))
                .sum::<f64>();
            let value_of = |white_hint_chances: &[f64], gold_hint_chances: &[f64]| {
                member_chances
                    .iter()
                    .map(|&q| model.trainee_fast(structure[member], q, white_hint_chances, gold_hint_chances))
                    .sum::<f64>()
            };
            total += if member_chances.iter().all(|q| (0.0..=1.0).contains(q)) {
                self.bb_max(&mut state.cluster_bound_scratch, structure[member], init, &value_of)
            } else {
                self.uview(structure[member])
                    .iter()
                    .map(|&j| value_of(&skyline[j].white_hint_chances, &skyline[j].gold_hint_chances))
                    .fold(init, f64::max)
            };
        }
        self.fixed * total / lineages.len() as f64 * (1.0 + 1e-12)
    }
    // ---- exact per-uma values ----
    // Loops are ranked by exact odds. A trainee's exact value at her lineages' inheritance chances is the best exact
    // P(all) over every legal deck (`DeckSpace`, not only the skyline). It is seeded with her best legal skyline decks
    // by product odds; other decks are bounded or scored under their structure-class caps, limited by the pass's cap.
    // Exactness assumes each deck's exact/product ratio stays within that cap and the pass finishes within budget.

    /// A deck's sources: the deck space's entry for (card, end link, variant) where it has one of that card, borrowed;
    /// else built. With the filler count.
    pub(super) fn ents_of(&self, deck: &PDeck) -> DeckSources<'data> {
        let space = self.inputs.deck_space;
        let source_builder = Sources { deck_job: self.inputs.deck_job, target_groups: &self.target_groups };
        let entries: Vec<SourceEntry<Cow<TargetSources>>> = deck
            .ids
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                self.card_of.get(id).map(|card| {
                    let end_link = deck.ends.get(i).copied().filter(|&end| end > 0);
                    let variant = deck.variants.get(i).copied().unwrap_or(0);
                    match self.entry_by_card.get(&CardEntryKey {
                        card_id: *id,
                        end_link: end_link.unwrap_or(0),
                        variant,
                    }) {
                        Some(&j) if std::ptr::eq(space.cards[space.card_indices[j]], *card) => {
                            SourceEntry { sources: Cow::Borrowed(&space.sources[j]), kind: space.kind[j] }
                        }
                        _ => SourceEntry {
                            sources: Cow::Owned(source_builder.target_sources(card, end_link, variant)),
                            kind: source_builder.kind(card, end_link),
                        },
                    }
                })
            })
            .collect();
        DeckSources {
            entries,
            filler_count: self.inputs.deck_job.deck_size.saturating_sub(deck.ids.len() + deck.plain_ids.len()),
        }
    }

    /// Exact value of trainee u at chances `chances` under cap `cap`: the best exact P(all) over her best legal skyline
    /// decks and every legal deck whose product value times `cap` reaches it (each first through the proven bound).
    pub(super) fn exact_best(&self, state: &mut LoopSearchState, uma: UmaIndex, chances: &[f64], cap: f64) -> Rc<Best> {
        let key = ExactQueryKey { trainee: uma, chances: ChanceKey::new(chances) };
        if let Some(entry) = state.exact_cache.get(&key)
            && entry.certified_cap >= cap
            && let Some(result) = &entry.reported_best
        {
            return Rc::clone(result);
        }
        let mut entry = state.exact_cache.remove(&key).unwrap_or(ExactCacheEntry {
            best: None,
            scored_decks: crate::FxSet::default(),
            checked_selections: crate::FxSet::default(),
            certified_cap: -1.0,
            reported_best: None,
        });
        if entry.certified_cap < 0.0 {
            self.seed_exact(&mut state.state_bounds, &mut state.tally, uma, chances, &mut entry);
            entry.certified_cap = 1.0;
        }
        if cap > entry.certified_cap {
            self.raise_exact(&mut state.state_bounds, &mut state.tally, uma, chances, cap, &mut entry);
        }
        let best = Rc::new(self.best_of(uma, chances, &entry));
        entry.reported_best = Some(Rc::clone(&best));
        state.exact_cache.insert(key, entry);
        best
    }

    /// Scores deck `deck` exactly for trainee u at chances `chances` (once per entry) and keeps it if it is the best
    fn score_exact(
        &self,
        tally: &mut Tally,
        uma: UmaIndex,
        chances: &[f64],
        deck: &PDeck,
        entry: &mut ExactCacheEntry,
    ) {
        if !entry.scored_decks.insert(deck.id()) {
            return;
        }
        let model = self.inputs.model;
        let uma_id = self.inputs.data.umas[uma].id;
        let started = Instant::now();
        let DeckSources { entries, filler_count } = self.ents_of(deck);
        let results = joint_b(
            model,
            JointQuery::unfolded(model, self.inputs.deck_job, uma, chances),
            &entries,
            filler_count,
            self.requested_target_count,
            false,
        );
        tally.score_seconds += started.elapsed().as_secs_f64();
        if crate::checks::get().upper_check {
            let borrowed_entries: Vec<SourceEntry<&TargetSources>> =
                entries.iter().map(|entry| SourceEntry { sources: &*entry.sources, kind: entry.kind }).collect();
            if let Some(upper_bounds) = upper_na(
                model,
                JointQuery::unfolded(model, self.inputs.deck_job, uma, chances),
                &borrowed_entries,
                filler_count,
            ) {
                for (i, result) in results.iter().enumerate() {
                    assert!(
                        upper_bounds[i] >= result.all * (1.0 - 1e-12),
                        "upper bound below exact: {} < {} for {:?}",
                        upper_bounds[i],
                        result.all,
                        deck.names
                    );
                    UCHK.with(|cell| {
                        let mut counts = cell.get();
                        counts.checked += 1;
                        counts.max_ratio = counts.max_ratio.max(upper_bounds[i] / result.all);
                        cell.set(counts);
                    });
                }
            } else {
                UCHK.with(|cell| {
                    let mut counts = cell.get();
                    counts.uncovered += 1;
                    cell.set(counts);
                });
            }
        }
        tally.scored_decks += 1;
        tally.spend(200);
        let deck_class = src_class(entries.iter().map(|entry| &*entry.sources));
        for (i, &q) in chances.iter().enumerate() {
            let product_value =
                self.fixed * model.trainee_fast(uma, q, &deck.white_hint_chances, &deck.gold_hint_chances);
            if product_value > 0.0 {
                tally.ratio_at(deck_class, results[i].all / product_value, uma_id, deck);
            }
        }
        let value_sum: f64 = results.iter().map(|result| result.all).sum();
        if entry.best.as_ref().is_none_or(|best| value_sum > best.value) {
            entry.best = Some(ScoredDeck { value: value_sum, deck: deck.clone() });
        }
    }

    /// seed: her best legal skyline decks by product value (or the best legal deck when none is on the skyline)
    fn seed_exact(
        &self,
        state_bounds: &mut crate::deck::state_bound::StateBoundMemo,
        tally: &mut Tally,
        uma: UmaIndex,
        chances: &[f64],
        entry: &mut ExactCacheEntry,
    ) {
        let (model, space) = (self.inputs.model, self.inputs.deck_space);
        let uma_id = self.inputs.data.umas[uma].id;
        let started = Instant::now();
        // the first 3 of a stable sort by value, highest first: each deck goes after every kept one at least as high
        let mut seeds: Vec<(f64, &PDeck)> = Vec::with_capacity(4);
        // (the value first, the blocks lookups only for a deck that would enter: one that can't is passed over blocked
        // or not, so the seeds are the same)
        for deck in self.uview(uma).iter().map(|&j| &self.inputs.skyline[j]) {
            let value = self.fixed
                * chances
                    .iter()
                    .map(|&q| model.trainee_fast(uma, q, &deck.white_hint_chances, &deck.gold_hint_chances))
                    .sum::<f64>();
            let position = seeds
                .iter()
                .position(|seed| seed.0.partial_cmp(&value).unwrap() == std::cmp::Ordering::Less)
                .unwrap_or(seeds.len());
            if position < 3 && !deck.blocks(uma_id, &self.card_of, crate::checks::get()) {
                seeds.insert(position, (value, deck));
                seeds.truncate(3);
            }
        }
        tally.seed_seconds += started.elapsed().as_secs_f64();
        for (_, deck) in seeds.iter().take(3) {
            self.score_exact(tally, uma, chances, deck, entry);
        }
        if entry.best.is_none() {
            let SearchOutcome { decks, .. } = space.search(
                state_bounds,
                &model.deck_chances(),
                &[uma],
                &[uma_id],
                &[SearchClause {
                    lineages: chances.iter().map(|&q| LineageChance { trainee: 0, inheritance: q }).collect(),
                    threshold: 0.0,
                }],
                SearchMode::Best { seed: f64::NEG_INFINITY },
            );
            if let Some(ScoredDeck { value: _, deck: best }) = decks.first() {
                let deck = space.pdeck(best, &[uma_id]);
                self.score_exact(tally, uma, chances, &deck, entry);
            }
        }
    }

    /// Raises an entry's certified cap to `cap`: every legal deck that might beat its best under the cap is scored
    /// (after the proven bound); a deck search cut short or the budget running out certifies nothing and aborts the
    /// pass
    fn raise_exact(
        &self,
        state_bounds: &mut crate::deck::state_bound::StateBoundMemo,
        tally: &mut Tally,
        uma: UmaIndex,
        chances: &[f64],
        cap: f64,
        entry: &mut ExactCacheEntry,
    ) {
        let (model, space) = (self.inputs.model, self.inputs.deck_space);
        let uma_id = self.inputs.data.umas[uma].id;
        let mut complete = true;
        let Some(threshold) = entry.best.as_ref().map(|b| b.value) else {
            entry.certified_cap = cap;
            return;
        };
        tally.band_decks += 1;
        let started = Instant::now();
        let nodes_before = space.nodes.get();
        let SearchOutcome { decks: mut candidates, incomplete, .. } = space.search(
            state_bounds,
            &model.deck_chances(),
            &[uma],
            &[uma_id],
            &[SearchClause {
                lineages: chances.iter().map(|&q| LineageChance { trainee: 0, inheritance: q }).collect(),
                threshold,
            }],
            SearchMode::Band { ratio_cap: cap, max_decks: space.band_cap(), bound: BandBound::Capped },
        );
        tally.spend(2 * (space.nodes.get() - nodes_before) + 5 * candidates.len() + 10_000);
        tally.band_seconds += started.elapsed().as_secs_f64();
        if incomplete.is_some() {
            tally.cap_hit = true;
            complete = false;
        }
        candidates.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());
        if complete {
            for ScoredDeck { value, deck: selected } in &candidates {
                if cap * value * (1.0 + 1e-9) < entry.best.as_ref().unwrap().value {
                    break;
                }
                if tally.work > tally.budget {
                    complete = false;
                    break;
                }
                // below the best at its own class's cap (the best rose since the band was built)
                if space.sel_cap(selected, cap) * value * (1.0 + 1e-9) < entry.best.as_ref().unwrap().value {
                    continue;
                }
                if !entry.checked_selections.insert(SelectionKey {
                    entries: selected.entries.clone(),
                    plain_pals: selected.plain_pals,
                    filler_cards: selected.filler_cards,
                }) {
                    continue;
                }
                // the proven bound first: a deck it puts below the best exact value cannot beat it
                let entries: Vec<SourceEntry<&TargetSources>> = selected
                    .entries
                    .iter()
                    .map(|&j| SourceEntry { sources: &space.sources[j], kind: space.kind[j] })
                    .collect();
                let bound_started = Instant::now();
                let upper_bounds = upper_na(
                    model,
                    JointQuery::unfolded(model, self.inputs.deck_job, uma, chances),
                    &entries,
                    selected.filler_cards,
                );
                tally.bound_seconds += bound_started.elapsed().as_secs_f64();
                if let Some(bounds) = upper_bounds {
                    tally.spend(25);
                    if bounds.iter().sum::<f64>() * (1.0 + 1e-9) < entry.best.as_ref().unwrap().value {
                        tally.bound_rejections += 1;
                        continue;
                    }
                }
                let deck = space.pdeck(selected, &[uma_id]);
                self.score_exact(tally, uma, chances, &deck, entry);
            }
        }
        if complete {
            entry.certified_cap = cap;
        } else {
            tally.aborted = true;
        }
    }

    /// The entry's best with its per-lineage values (the deckless value when no deck is legal)
    fn best_of(&self, uma: UmaIndex, chances: &[f64], entry: &ExactCacheEntry) -> Best {
        if let Some(ScoredDeck { value: sum, deck }) = &entry.best {
            let DeckSources { entries, filler_count } = self.ents_of(deck);
            let per_lineage = joint_b(
                self.inputs.model,
                JointQuery::unfolded(self.inputs.model, self.inputs.deck_job, uma, chances),
                &entries,
                filler_count,
                self.requested_target_count,
                true,
            );
            Best { sum: *sum, deck: Some(deck.clone()), lineage_chances: per_lineage }
        } else {
            let per_lineage = joint(
                self.inputs.model,
                JointQuery::unfolded(self.inputs.model, self.inputs.deck_job, uma, chances),
                &[],
                0,
                self.requested_target_count,
                true,
            );
            Best { sum: per_lineage.iter().map(|result| result.all).sum(), deck: None, lineage_chances: per_lineage }
        }
    }

    /// X[u][i] under cap `cap`, memoised in `exact_ceiling_cache` when certified under `cap` or more.
    fn xtab(&self, state: &mut LoopSearchState, uma: UmaIndex, i: usize, cap: f64) -> f64 {
        let ExactCeiling { value, certified_cap } = state.exact_ceiling_cache[uma][i];
        if !value.is_nan() && certified_cap >= cap {
            return value;
        }
        let value = self.exact_best(state, uma, &[self.grid.chances[i]], cap).sum;
        // a value from a deck search the budget cut short is not exact under `cap`: not kept
        if !state.tally.aborted {
            state.exact_ceiling_cache[uma][i] = ExactCeiling { value, certified_cap: cap };
        }
        value
    }

    /// Ceiling of a cell (uma, grid point) under cap `cap`: the exact value if computed under `cap` or more, else the
    /// lowest such value at a higher grid point, else the product-odds bound times `cap`; (bound, exact?)
    pub(super) fn cell(&self, state: &mut LoopSearchState, uma: UmaIndex, i: usize, cap: f64) -> (f64, bool) {
        let mut bound = cap * self.fixed * self.btab(state, uma, i) * (1.0 + 1e-12);
        let exact_ceilings = &state.exact_ceiling_cache;
        let row = &exact_ceilings[uma];
        for j in i..NG {
            if !row[j].value.is_nan() && row[j].certified_cap >= cap {
                bound = bound.min(row[j].value * (1.0 + 1e-12));
                return (bound, j == i);
            }
        }
        (bound, false)
    }

    /// Exact ceiling of a foursome at chances `lineage_chances`, refined lazily: while it beats `kth_best`, the loosest
    /// inexact term is tightened (first computed at the uma's highest grid point, then halfway down towards its own)
    /// until the ceiling stops beating `kth_best` or every term is exact. Each step is one exact deck search for one
    /// uma.
    pub(super) fn exact_ceiling(
        &self,
        state: &mut LoopSearchState,
        structure: [UmaIndex; 4],
        lineage_chances: &[f64; 12],
        kth_best: f64,
        cap: f64,
    ) -> f64 {
        loop {
            let mut total = 0.0;
            let mut worst: Option<(f64, UmaIndex, usize)> = None;
            let lineages = lin_of(&structure);
            for l in 0..lineages.len() {
                let (uma, i) = (structure[lineages[l].trainee], self.grid.idx(lineage_chances[l]));
                let (bound, exact) = self.cell(state, uma, i, cap);
                total += bound;
                if !exact && worst.is_none_or(|current| bound > current.0) {
                    worst = Some((bound, uma, i));
                }
            }
            let ceiling = total / lineages.len() as f64;
            let Some((_, uma, i)) = worst else { return ceiling };
            if ceiling <= kth_best * (1.0 + 4e-12) || state.tally.aborted {
                return ceiling;
            }
            let j = {
                let exact_ceilings = &state.exact_ceiling_cache;
                let row = &exact_ceilings[uma];
                (i..NG).find(|&j| !row[j].value.is_nan() && row[j].certified_cap >= cap)
            };
            let grid_point = match j {
                None => i.max(self.grid.idx(self.maximum_chances[uma])),
                Some(j) => i + (j - i) / 2,
            };
            self.xtab(state, uma, grid_point, cap);
        }
    }
}
/// (bound, cluster) ordered as the stable sort by bound, highest first, visits them: a heap's pops give the higher bound
/// first and equal bounds (`partial_cmp`: 0 and -0 tie) in index order, the same sequence (a NaN bound panics in
/// either)
pub(super) struct ByBound {
    bound: f64,
    cluster: usize,
}
impl PartialEq for ByBound {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}
impl Eq for ByBound {}
impl PartialOrd for ByBound {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ByBound {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bound
            .partial_cmp(&other.bound)
            .unwrap()
            // BinaryHeap pops the greatest item: reverse indices so earlier clusters win ties.
            .then(other.cluster.cmp(&self.cluster))
    }
}
