//! The exact joint landing odds of one deck: `joint_prep` collapses its chain states and groups the coupled outcome
//! groups once per deck, and `evaluate_prepared` sums them per inheritance chance and outfit, in reusable buffers
//! (`JointScratch`). The unoptimized form `JOINT_CHECK` compares it with is in `reference`.
use crate::deck::chains::{ChainKind, ChainState, chain_states};
use crate::deck::joint::{ChainEntry, JointChances, JointExt, JointOptions, JointQuery, LandPre};
use crate::deck::sources::{
    ChainOutcome, EventOutcome, Group, SourceEntry, TargetReward, TargetSources, TrainingHint, group_gh,
};
use crate::skills::{GEN_GOLD, Model, OutfitOption};
/// One landed-target subset and its probability in the current outcome product.
#[derive(Clone, Copy)]
struct SubsetProbability {
    targets: usize,
    probability: f64,
}

/// The deck-dependent part of `joint_fast`, before inheritance chances or outfit state are applied.
/// Collapsed states retain first-seen order; each weight is summed in the original chain-state order.
pub(crate) struct JointPrep<'group> {
    /// Miss factors and coupled groups from sources outside the chains.
    base: ChainFactors,
    chains: Vec<ChainProgress>,
    /// One class index per relevant chain position, and that collapsed state's probability.
    states: Vec<CollapsedChainState>,
    groups: Vec<CoupledGroup<'group>>,
}
struct CollapsedChainState {
    /// Class index at each relevant chain position.
    classes: Vec<u8>,
    probability: f64,
}

/// Cumulative factors after a chain prefix, or from all sources outside the chains.
struct ChainFactors {
    no_gold: Vec<f64>,
    no_hint: Vec<f64>,
    coupled: Vec<usize>,
}
/// A chain position whose progress can change target factors. Progress before the next source-bearing link shares a class.
struct ChainProgress {
    position: usize,
    levels: Vec<ChainFactors>,
    class_of_progress: Vec<u8>,
}
/// A coupled group's ascending target indices and outcomes. The implicit no-reward outcome is last if above 1e-15.
struct CoupledGroup<'group> {
    targets: Vec<usize>,
    outcomes: Vec<EventOutcome<&'group [TargetReward]>>,
}
pub(crate) fn joint_prep<S: std::borrow::Borrow<TargetSources>>(
    chain_events: crate::deck::chains::ChainEvents,
    uncertain_count: usize,
    entries: &[SourceEntry<S>],
    filler: usize,
) -> JointPrep<'_> {
    fn add_group<'group>(
        group: &'group Group,
        groups: &mut Vec<CoupledGroup<'group>>,
        coupled: &mut Vec<usize>,
        no_gold: &mut [f64],
        no_hint: &mut [f64],
    ) {
        let marginals = group_gh(group);
        if marginals.len() >= 2 {
            let mut outcomes: Vec<EventOutcome<&[TargetReward]>> = group
                .iter()
                .map(|outcome| EventOutcome { probability: outcome.probability, rewards: &*outcome.rewards })
                .collect();
            let no_reward_probability = 1.0 - group.iter().map(|outcome| outcome.probability).sum::<f64>();
            if no_reward_probability > 1e-15 {
                outcomes.push(EventOutcome { probability: no_reward_probability, rewards: &[][..] });
            }
            groups.push(CoupledGroup { targets: marginals.iter().map(|entry| entry.0).collect(), outcomes });
            coupled.push(groups.len() - 1);
            return;
        }
        for (k, gold, any_hint) in marginals {
            no_gold[k] *= 1.0 - gold;
            no_hint[k] *= 1.0 - any_hint;
        }
    }
    let (mut base_no_gold, mut base_no_hint) = (vec![1.0f64; uncertain_count], vec![1.0f64; uncertain_count]);
    let mut base_coupled: Vec<usize> = vec![];
    let mut groups: Vec<CoupledGroup> = vec![];
    for SourceEntry { sources, .. } in entries {
        let sources: &TargetSources = sources.borrow();
        for &TrainingHint { target: k, probability: p } in &sources.training_hints {
            base_no_hint[k] *= 1.0 - p;
        }
        for group in &sources.fixed_outcomes {
            add_group(group, &mut groups, &mut base_coupled, &mut base_no_gold, &mut base_no_hint);
        }
    }
    let mut chain_entries: Vec<ChainEntry> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.kind != ChainKind::NONE)
        .map(|(i, entry)| ChainEntry { kind: entry.kind, source_index: Some(i) })
        .collect();
    for _ in 0..filler {
        chain_entries.push(ChainEntry { kind: ChainKind::FILLER, source_index: None });
    }
    chain_entries.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.source_index.cmp(&b.source_index)));
    let kinds: Vec<ChainKind> = chain_entries.iter().map(|entry| entry.kind).collect();
    let states = if kinds.is_empty() {
        std::rc::Rc::new(vec![ChainState { progress: vec![], probability: 1.0 }])
    } else {
        chain_states(chain_events, &kinds)
    };
    let mut chains: Vec<ChainProgress> = vec![];
    for (position, &ChainEntry { kind, source_index: entry }) in chain_entries.iter().enumerate() {
        let Some(i) = entry else { continue };
        let sources: &TargetSources = entries[i].sources.borrow();
        if sources.chain_outcomes.is_empty() {
            continue;
        }
        let (mut no_gold, mut no_hint, mut coupled) =
            (vec![1.0f64; uncertain_count], vec![1.0f64; uncertain_count], vec![]);
        let mut levels =
            vec![ChainFactors { no_gold: no_gold.clone(), no_hint: no_hint.clone(), coupled: coupled.clone() }];
        let mut class_of_progress = vec![0u8];
        for link in 1..=kind.stop {
            let mut changed = false;
            for ChainOutcome { link: group_link, outcomes: group } in &sources.chain_outcomes {
                if *group_link == link {
                    add_group(group, &mut groups, &mut coupled, &mut no_gold, &mut no_hint);
                    changed = true;
                }
            }
            if changed {
                levels.push(ChainFactors {
                    no_gold: no_gold.clone(),
                    no_hint: no_hint.clone(),
                    coupled: coupled.clone(),
                });
            }
            class_of_progress.push((levels.len() - 1) as u8);
        }
        if levels.len() > 1 {
            chains.push(ChainProgress { position, levels, class_of_progress });
        }
    }
    // Preserve first-seen order separately from the map: it controls the later floating-point sums.
    let mut collapsed: crate::FxMap<Vec<u8>, f64> = crate::FxMap::default();
    let mut order: Vec<Vec<u8>> = vec![];
    for ChainState { progress, probability } in states.iter() {
        let key: Vec<u8> =
            chains.iter().map(|chain| chain.class_of_progress[progress[chain.position] as usize]).collect();
        if let Some(weight) = collapsed.get_mut(&key) {
            *weight += probability;
        } else {
            collapsed.insert(key.clone(), *probability);
            order.push(key);
        }
    }
    let states = order
        .into_iter()
        .map(|key| {
            let probability = collapsed[&key];
            CollapsedChainState { classes: key, probability }
        })
        .collect();
    JointPrep {
        base: ChainFactors { no_gold: base_no_gold, no_hint: base_no_hint, coupled: base_coupled },
        chains,
        states,
        groups,
    }
}
impl JointPrep<'_> {
    /// Evaluate a prepared deck with fresh scratch. Disabling marginals leaves the extension's marginal buffer empty.
    pub(super) fn evaluate(
        &self,
        model: &Model,
        query: JointQuery,
        options: JointOptions,
        extension: Option<&mut JointExt>,
    ) -> Vec<JointChances> {
        evaluate_prepared(model, query, self, options, extension, &mut JointScratch::default())
    }
}

#[derive(Clone, Copy)]
pub(super) struct LandingProbability {
    pub(super) all: f64,
    pub(super) all_but_one: f64,
}

/// `JointPrep::evaluate`'s working buffers, kept by a caller that evaluates many times (`CareerSetup::corners`): each call
/// sets every one to the contents a fresh one starts with (the same lengths and values; the clusters' and the landed sets'
/// inner buffers are cleared where first used, as fresh ones are), so only their capacity carries over
#[derive(Default)]
pub(crate) struct JointScratch {
    landed_sets: Vec<Vec<f64>>,
    totals: Vec<LandingProbability>,
    landing_factors: Vec<[f64; 3]>,
    no_gold: Vec<f64>,
    no_hint: Vec<f64>,
    coupled_groups: Vec<usize>,
    cluster_of_target: Vec<usize>,
    hint_levels: Vec<u8>,
    target_parents: Vec<usize>,
    cluster_targets: Vec<Vec<usize>>,
    cluster_groups: Vec<Vec<usize>>,
    in_cluster: Vec<bool>,
    all_land: Vec<f64>,
    one_misses: Vec<f64>,
    cluster_all_land: Vec<f64>,
    cluster_one_misses: Vec<f64>,
    outcome_indices: Vec<usize>,
    cluster_distribution: Vec<f64>,
    cluster_masks: Vec<usize>,
    subsets: Vec<SubsetProbability>,
    next_subsets: Vec<SubsetProbability>,
    next_distribution: Vec<f64>,
}
impl JointScratch {
    /// Clear values while retaining capacities, including each cluster's inner buffers.
    fn reset(&mut self, target_count: usize, query_outfit_count: usize, want_sets: bool) {
        self.landed_sets.resize(if want_sets { query_outfit_count } else { 0 }, vec![]);
        self.totals.clear();
        self.totals.resize(query_outfit_count, LandingProbability { all: 0.0, all_but_one: 0.0 });
        // Per (chance, outfit, target): no coupled hint, coupled white hint, coupled gold hint.
        self.landing_factors.clear();
        self.landing_factors.resize(query_outfit_count * target_count, [0.0f64; 3]);
        self.no_gold.clear();
        self.no_gold.resize(target_count, 1.0);
        self.no_hint.clear();
        self.no_hint.resize(target_count, 1.0);
        self.coupled_groups.clear();
        self.cluster_of_target.clear();
        self.cluster_of_target.resize(target_count, usize::MAX);
        self.hint_levels.clear();
        self.hint_levels.resize(target_count, 0);
        // Each chain state writes these buffers before reading them. Cluster distributions are dense:
        // entry j holds cluster_masks[j], and each outcome combination writes subsets in the same order.
        self.target_parents.clear();
        self.in_cluster.clear();
        self.in_cluster.resize(target_count, false);
        for buffer in
            [&mut self.all_land, &mut self.one_misses, &mut self.cluster_all_land, &mut self.cluster_one_misses]
        {
            buffer.clear();
            buffer.resize(query_outfit_count, 0.0);
        }
        self.outcome_indices.clear();
        self.cluster_distribution.clear();
        self.cluster_masks.clear();
        self.subsets.clear();
        self.next_subsets.clear();
        self.next_distribution.clear();
    }

    /// Union targets connected by coupled outcome groups, retaining first-seen cluster order.
    fn build_clusters(&mut self, groups: &[CoupledGroup], target_count: usize) -> usize {
        self.target_parents.clear();
        self.target_parents.extend(0..target_count);
        const fn root(parents: &mut [usize], mut target: usize) -> usize {
            while parents[target] != target {
                parents[target] = parents[parents[target]];
                target = parents[target];
            }
            target
        }
        for &group in &self.coupled_groups {
            let targets = &groups[group].targets;
            for adjacent_targets in targets.windows(2) {
                let (first_root, second_root) = (
                    root(&mut self.target_parents, adjacent_targets[0]),
                    root(&mut self.target_parents, adjacent_targets[1]),
                );
                if first_root != second_root {
                    self.target_parents[first_root.max(second_root)] = first_root.min(second_root);
                }
            }
        }
        self.cluster_of_target.fill(usize::MAX);
        let mut cluster_count = 0usize;
        for &group in &self.coupled_groups {
            let group_root = root(&mut self.target_parents, groups[group].targets[0]);
            let cluster_index = match self.cluster_of_target[group_root] {
                usize::MAX => {
                    if cluster_count == self.cluster_targets.len() {
                        self.cluster_targets.push(vec![]);
                        self.cluster_groups.push(vec![]);
                    }
                    self.cluster_targets[cluster_count].clear();
                    self.cluster_groups[cluster_count].clear();
                    self.cluster_of_target[group_root] = cluster_count;
                    cluster_count += 1;
                    cluster_count - 1
                }
                c => c,
            };
            self.cluster_groups[cluster_index].push(group);
        }
        for k in 0..target_count {
            if self.cluster_of_target[root(&mut self.target_parents, k)] != usize::MAX {
                let cluster_index = self.cluster_of_target[root(&mut self.target_parents, k)];
                self.cluster_targets[cluster_index].push(k);
            }
        }
        for k in 0..target_count {
            self.in_cluster[k] = self.cluster_of_target[root(&mut self.target_parents, k)] != usize::MAX;
        }
        cluster_count
    }
}

/// `JointPrep::evaluate` with reused buffers from `scratch`.
pub(crate) fn joint_eval_in(
    model: &Model,
    query: JointQuery,
    prepared: &JointPrep,
    requested_target_count: usize,
    all_but_one: bool,
    marginals: bool,
    extension: Option<&mut JointExt>,
    scratch: &mut JointScratch,
) -> Vec<JointChances> {
    evaluate_prepared(
        model,
        query,
        prepared,
        JointOptions { target_count: requested_target_count, all_but_one, marginals },
        extension,
        scratch,
    )
}

fn evaluate_prepared(
    model: &Model,
    query: JointQuery,
    prepared: &JointPrep,
    options: JointOptions,
    mut extension: Option<&mut JointExt>,
    workspace: &mut JointScratch,
) -> Vec<JointChances> {
    // Phase 1: reset per-query outputs and retained scratch; unit scales leave factors bit-identical.
    let target_count = model.uncertain.len();
    let mut scratch = std::mem::take(workspace);
    let JointPrep { base, chains, states, groups } = prepared;
    let outfits = &model.outfit_options[query.trainee];
    let outfit_count = outfits.len();
    let chance_count = query.inheritance_chances.len();
    let fixed_target_count = model.n_fixed_gold as usize;
    let fixed_factor = match &extension {
        Some(extra) => extra.scales.guaranteed.iter().fold(1.0, |product, scale| product * GEN_GOLD * scale),
        None => model.fixed_factor(),
    };
    let want_landed_sets = extension.as_ref().is_some_and(|extra| !extra.sets.is_empty()) && target_count <= 16;
    if let Some(extra) = extension.as_deref_mut() {
        if options.marginals {
            extra.marginals = vec![0.0; chance_count * outfit_count * target_count];
        }
        if want_landed_sets {
            extra.sets = vec![0.0; (chance_count * outfit_count) << target_count];
        }
    }
    // (a scale of exactly 1.0 for every target leaves each factor as it is, x * 1.0 = x: not multiplied)
    let has_unit_scales =
        extension.as_ref().is_none_or(|extra| extra.scales.uncertain.iter().all(|&scale| scale == 1.0));
    let query_outfit_count = chance_count * outfit_count;
    scratch.reset(target_count, query_outfit_count, want_landed_sets);
    let mut outcomes: Vec<&[EventOutcome<&[TargetReward]>]> = vec![];
    // Phase 2: condition on each collapsed chain state in first-seen order. Given that state,
    // only targets connected by a coupled outcome group can remain dependent.
    for state in states {
        let state_probability = state.probability;
        scratch.no_gold.copy_from_slice(&base.no_gold);
        scratch.no_hint.copy_from_slice(&base.no_hint);
        scratch.coupled_groups.clear();
        scratch.coupled_groups.extend(base.coupled.iter().copied());
        for (j, chain) in chains.iter().enumerate() {
            let factors = &chain.levels[state.classes[j] as usize];
            for k in 0..target_count {
                scratch.no_gold[k] *= factors.no_gold[k];
                scratch.no_hint[k] *= factors.no_hint[k];
            }
            scratch.coupled_groups.extend(factors.coupled.iter().copied());
        }
        // Phase 3: tabulate landing factors for no coupled hint, a white hint, and a gold hint.
        for (chance_index, &q) in query.inheritance_chances.iter().enumerate() {
            for (outfit, OutfitOption { coverage, .. }) in outfits.iter().enumerate() {
                let event_chances = &query.event_chances[outfit];
                let base = (chance_index * outfit_count + outfit) * target_count;
                for k in 0..target_count {
                    let landing_factor = LandPre::new(coverage[k], event_chances[k], q);
                    scratch.landing_factors[base + k] = [
                        landing_factor.f(scratch.no_gold[k], scratch.no_hint[k]),
                        landing_factor.f(scratch.no_gold[k], 0.0),
                        landing_factor.f(0.0, 0.0),
                    ];
                    if !has_unit_scales && let Some(extra) = &extension {
                        for factor in &mut scratch.landing_factors[base + k] {
                            *factor *= extra.scales.uncertain[k];
                        }
                    }
                }
            }
        }
        // Phase 4: combine independent targets; each update preserves P(all) and P(exactly one miss).
        let cluster_count = scratch.build_clusters(groups, target_count);
        // per (q, outfit): (P(all land), P(exactly one misses)) of each independent unit, combined
        scratch.all_land.fill(1.0);
        scratch.one_misses.fill(0.0);
        let combine_independent =
            |all_land: &mut [f64], one_misses: &mut [f64], i: usize, unit_all: f64, unit_one: f64| {
                one_misses[i] = one_misses[i] * unit_all + all_land[i] * unit_one;
                all_land[i] *= unit_all;
            };
        for i in 0..query_outfit_count {
            for k in 0..target_count {
                if !scratch.in_cluster[k] {
                    let unit_factor = scratch.landing_factors[i * target_count + k][0];
                    combine_independent(
                        &mut scratch.all_land,
                        &mut scratch.one_misses,
                        i,
                        unit_factor,
                        1.0 - unit_factor,
                    );
                    if options.marginals
                        && let Some(extra) = extension.as_deref_mut()
                    {
                        extra.marginals[i * target_count + k] += state_probability * unit_factor;
                    }
                }
            }
        }
        // the landed sets: independent targets first, clusters below (their targets join through `scratch.cluster_distribution`)
        if want_landed_sets {
            for i in 0..query_outfit_count {
                let distribution = &mut scratch.landed_sets[i];
                distribution.clear();
                distribution.resize(1 << target_count, 0.0);
                distribution[0] = 1.0;
                for k in 0..target_count {
                    if scratch.in_cluster[k] {
                        continue;
                    }
                    let (landing_chance, target_bit) = (scratch.landing_factors[i * target_count + k][0], 1usize << k);
                    for mask in 0..(1usize << target_count) {
                        if mask & target_bit == 0 && distribution[mask] != 0.0 {
                            let probability = distribution[mask];
                            distribution[mask] = probability * (1.0 - landing_chance);
                            distribution[mask | target_bit] += probability * landing_chance;
                        }
                    }
                }
            }
        }
        // Phase 5: enumerate each connected cluster's outcomes, then convolve its independent
        // contribution into the state totals and, when requested, the landed-set distribution.
        for (targets, group_indices) in
            scratch.cluster_targets[..cluster_count].iter().zip(&scratch.cluster_groups[..cluster_count])
        {
            outcomes.clear();
            outcomes.extend(group_indices.iter().map(|&group| &*groups[group].outcomes));
            scratch.cluster_all_land.fill(0.0);
            scratch.cluster_one_misses.fill(0.0);
            // per (chance, outfit): distribution over the subsets of this cluster's targets (as full masks)
            if want_landed_sets {
                let subset_count = 1usize << targets.len();
                scratch.cluster_distribution.clear();
                scratch.cluster_distribution.resize(query_outfit_count * subset_count, 0.0);
                scratch.cluster_masks.clear();
                scratch.cluster_masks.resize(subset_count, 0);
            }
            scratch.outcome_indices.clear();
            scratch.outcome_indices.resize(outcomes.len(), 0);
            loop {
                let outcome_probability: f64 =
                    (0..outcomes.len()).map(|i| outcomes[i][scratch.outcome_indices[i]].probability).product();
                for &k in targets {
                    scratch.hint_levels[k] = 0;
                }
                for i in 0..outcomes.len() {
                    for &TargetReward { target: k, level } in outcomes[i][scratch.outcome_indices[i]].rewards {
                        scratch.hint_levels[k] = scratch.hint_levels[k].max(if level >= 2 { 2 } else { 1 });
                    }
                }
                for i in 0..query_outfit_count {
                    let (mut all_land, mut one_misses) = (1.0f64, 0.0f64);
                    for &k in targets {
                        let landing_chance =
                            scratch.landing_factors[i * target_count + k][scratch.hint_levels[k] as usize];
                        one_misses = one_misses * landing_chance + all_land * (1.0 - landing_chance);
                        all_land *= landing_chance;
                    }
                    scratch.cluster_all_land[i] += outcome_probability * all_land;
                    scratch.cluster_one_misses[i] += outcome_probability * one_misses;
                    if options.marginals
                        && let Some(extra) = extension.as_deref_mut()
                    {
                        for &k in targets {
                            extra.marginals[i * target_count + k] += state_probability
                                * outcome_probability
                                * scratch.landing_factors[i * target_count + k][scratch.hint_levels[k] as usize];
                        }
                    }
                    if want_landed_sets {
                        scratch.subsets.clear();
                        scratch.subsets.push(SubsetProbability { targets: 0, probability: outcome_probability });
                        for &k in targets {
                            let landing_chance =
                                scratch.landing_factors[i * target_count + k][scratch.hint_levels[k] as usize];
                            scratch.next_subsets.clear();
                            for &SubsetProbability { targets: mask, probability } in &scratch.subsets {
                                scratch.next_subsets.push(SubsetProbability {
                                    targets: mask,
                                    probability: probability * (1.0 - landing_chance),
                                });
                                scratch.next_subsets.push(SubsetProbability {
                                    targets: mask | (1 << k),
                                    probability: probability * landing_chance,
                                });
                            }
                            std::mem::swap(&mut scratch.subsets, &mut scratch.next_subsets);
                        }
                        let cluster_row = &mut scratch.cluster_distribution
                            [i * scratch.subsets.len()..(i + 1) * scratch.subsets.len()];
                        for (j, &SubsetProbability { targets: mask, probability }) in scratch.subsets.iter().enumerate()
                        {
                            cluster_row[j] += probability;
                            scratch.cluster_masks[j] = mask;
                        }
                    }
                }
                let mut i = 0;
                while i < outcomes.len() {
                    scratch.outcome_indices[i] += 1;
                    if scratch.outcome_indices[i] < outcomes[i].len() {
                        break;
                    }
                    scratch.outcome_indices[i] = 0;
                    i += 1;
                }
                if i == outcomes.len() {
                    break;
                }
            }
            for i in 0..query_outfit_count {
                combine_independent(
                    &mut scratch.all_land,
                    &mut scratch.one_misses,
                    i,
                    scratch.cluster_all_land[i],
                    scratch.cluster_one_misses[i],
                );
            }
            // (each entry m0 | m of a row is written once, m0 holding no target of this cluster: the order is free)
            if want_landed_sets {
                let subset_count = scratch.cluster_masks.len();
                for i in 0..query_outfit_count {
                    let distribution = &mut scratch.landed_sets[i];
                    scratch.next_distribution.clear();
                    scratch.next_distribution.resize(1 << target_count, 0.0);
                    for (outside_mask, &outside_probability) in distribution.iter().enumerate() {
                        if outside_probability == 0.0 {
                            continue;
                        }
                        for (&cluster_mask, &cluster_probability) in scratch
                            .cluster_masks
                            .iter()
                            .zip(&scratch.cluster_distribution[i * subset_count..(i + 1) * subset_count])
                        {
                            scratch.next_distribution[outside_mask | cluster_mask] +=
                                outside_probability * cluster_probability;
                        }
                    }
                    std::mem::swap(distribution, &mut scratch.next_distribution);
                }
            }
        }
        // Phase 6: include fixed golds and accumulate this state in the original summation order.
        for i in 0..query_outfit_count {
            let (mut all_land, mut one_misses) = (scratch.all_land[i], scratch.one_misses[i]);
            if options.all_but_one {
                for j in 0..fixed_target_count {
                    let gold_chance = match &extension {
                        Some(extra) => GEN_GOLD * extra.scales.guaranteed[j],
                        None => GEN_GOLD,
                    };
                    one_misses = one_misses * gold_chance + all_land * (1.0 - gold_chance);
                    all_land *= gold_chance;
                }
            } else {
                all_land *= fixed_factor;
            }
            // P(at least options.target_count - 1 land) over target_count + fixed_target_count targets
            let all_but_one = if options.target_count <= target_count + fixed_target_count {
                if options.target_count == 0 { 1.0 } else { all_land + one_misses }
            } else {
                0.0
            };
            scratch.totals[i].all += state_probability * all_land;
            scratch.totals[i].all_but_one += state_probability * if options.all_but_one { all_but_one } else { 0.0 };
            if want_landed_sets && let Some(extra) = extension.as_deref_mut() {
                for (landed_mask, &probability) in scratch.landed_sets[i].iter().enumerate() {
                    extra.sets[(i << target_count) + landed_mask] += state_probability * probability;
                }
            }
        }
    }
    // Phase 7: publish per-outfit totals and select the first outfit maximizing P(all).
    if let Some(extra) = extension {
        extra.per_outfit.clear();
        extra.per_outfit.extend(scratch.totals.iter().map(|total| total.all));
    }
    let chances = (0..chance_count)
        .map(|chance_index| {
            let mut best_outfit = 0;
            for outfit in 1..outfit_count {
                if scratch.totals[chance_index * outfit_count + outfit].all
                    > scratch.totals[chance_index * outfit_count + best_outfit].all
                {
                    best_outfit = outfit;
                }
            }
            JointChances {
                all: scratch.totals[chance_index * outfit_count + best_outfit].all,
                all_but_one: scratch.totals[chance_index * outfit_count + best_outfit].all_but_one,
                outfit: best_outfit,
            }
        })
        .collect();
    *workspace = scratch;
    chances
}
