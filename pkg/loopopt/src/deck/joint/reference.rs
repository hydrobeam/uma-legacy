//! The reference evaluation of a deck's joint landing odds (`joint_trainee_ref`): every chain state and every
//! combination of coupled outcomes, without the collapsing `exact` does. Setting `JOINT_CHECK` compares each exact
//! scoring with it; outside that check only tests call it.
use crate::deck::chains::{ChainKind, ChainState, chain_states};
use crate::deck::joint::exact::LandingProbability;
use crate::deck::joint::{ChainEntry, JointChances, JointOptions, JointQuery};
use crate::deck::sources::{
    ChainOutcome, EventOutcome, Group, SourceEntry, TargetReward, TargetSources, TrainingHint, group_gh,
};
use crate::skills::{GEN_GOLD, Model, OutfitOption};
/// Cumulative miss factors and coupled groups at one chain progress level.
struct ReferenceFactors<'group> {
    no_gold: Vec<f64>,
    no_hint: Vec<f64>,
    coupled: Vec<&'group Group>,
}

/// Reference form of `joint_b`: every chain state, every combination of coupled outcomes. Setting `JOINT_CHECK` runs
/// it in `joint_checked` and panics on a relative difference at or above 1e-11.
pub(in crate::deck) fn joint_trainee_ref(
    model: &Model,
    query: JointQuery,
    entries: &[SourceEntry<TargetSources>],
    filler: usize,
    requested_target_count: usize,
) -> Vec<JointChances> {
    reference_joint(
        model,
        query,
        entries,
        filler,
        JointOptions { target_count: requested_target_count, all_but_one: true, marginals: false },
    )
}

fn reference_joint(
    model: &Model,
    query: JointQuery,
    entries: &[SourceEntry<TargetSources>],
    filler: usize,
    options: JointOptions,
) -> Vec<JointChances> {
    use crate::skills::GEN_WHITE;
    let uncertain_count = model.uncertain.len();
    // per-target miss factors (no gold, no hint) of the groups that touch one target; the others are "coupled"
    fn add_group<'group>(
        group: &'group Group,
        coupled: &mut Vec<&'group Group>,
        no_gold: &mut [f64],
        no_hint: &mut [f64],
    ) {
        let marginals = group_gh(group);
        if marginals.len() >= 2 {
            coupled.push(group);
            return;
        }
        for (k, gold_chance, hint_chance) in marginals {
            no_gold[k] *= 1.0 - gold_chance;
            no_hint[k] *= 1.0 - hint_chance;
        }
    }
    // sources that do not depend on the chain
    let (mut base_no_gold, mut base_no_hint) = (vec![1.0f64; uncertain_count], vec![1.0f64; uncertain_count]);
    let mut base_coupled: Vec<&Group> = vec![];
    for SourceEntry { sources, .. } in entries {
        for &TrainingHint { target: k, probability: p } in &sources.training_hints {
            base_no_hint[k] *= 1.0 - p;
        }
        for group in &sources.fixed_outcomes {
            add_group(group, &mut base_coupled, &mut base_no_gold, &mut base_no_hint);
        }
    }
    // chain entries sorted by kind (with fillers) for the memoised state distribution; per entry and links done,
    // the cumulative factors and coupled groups of the links fired so far
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
        chain_states(query.chain_events, &kinds)
    };
    let chain_levels: Vec<Option<Vec<ReferenceFactors>>> = chain_entries
        .iter()
        .map(|&ChainEntry { kind, source_index }| {
            source_index.map(|i| {
                let sources = &entries[i].sources;
                let (mut no_gold, mut no_hint, mut coupled) =
                    (vec![1.0f64; uncertain_count], vec![1.0f64; uncertain_count], vec![]);
                let mut levels = vec![ReferenceFactors {
                    no_gold: no_gold.clone(),
                    no_hint: no_hint.clone(),
                    coupled: coupled.clone(),
                }];
                for link in 1..=kind.stop {
                    for ChainOutcome { link: group_link, outcomes: group } in &sources.chain_outcomes {
                        if *group_link == link {
                            add_group(group, &mut coupled, &mut no_gold, &mut no_hint);
                        }
                    }
                    levels.push(ReferenceFactors {
                        no_gold: no_gold.clone(),
                        no_hint: no_hint.clone(),
                        coupled: coupled.clone(),
                    });
                }
                levels
            })
        })
        .collect();
    let outfits = &model.outfit_options[query.trainee];
    let outfit_count = outfits.len();
    let chance_count = query.inheritance_chances.len();
    let fixed_target_count = model.n_fixed_gold as usize;
    let fixed = model.fixed_factor();
    let mut totals = vec![LandingProbability { all: 0.0, all_but_one: 0.0 }; chance_count * outfit_count];
    let (mut no_gold, mut no_hint) = (vec![1.0f64; uncertain_count], vec![1.0f64; uncertain_count]);
    let mut coupled: Vec<&Group> = vec![];
    let mut outcomes: Vec<Vec<EventOutcome<&[TargetReward]>>> = vec![];
    let mut outcome_indices: Vec<usize> = vec![];
    let mut landed_count_distribution = [0.0f64; 40];
    for ChainState { progress, probability: state_probability } in states.iter() {
        no_gold.copy_from_slice(&base_no_gold);
        no_hint.copy_from_slice(&base_no_hint);
        coupled.clear();
        coupled.extend(base_coupled.iter().copied());
        for (position, levels) in chain_levels.iter().enumerate() {
            let Some(levels) = levels else { continue };
            let factors = &levels[progress[position] as usize];
            for k in 0..uncertain_count {
                no_gold[k] *= factors.no_gold[k];
                no_hint[k] *= factors.no_hint[k];
            }
            coupled.extend(factors.coupled.iter().copied());
        }
        // outcomes of the coupled groups: each group's outcomes plus "nothing" (the rest of its weight)
        outcomes.clear();
        for group in &coupled {
            let mut group_outcomes: Vec<EventOutcome<&[TargetReward]>> = group
                .iter()
                .map(|outcome| EventOutcome { probability: outcome.probability, rewards: &*outcome.rewards })
                .collect();
            let no_reward_probability = 1.0 - group.iter().map(|outcome| outcome.probability).sum::<f64>();
            if no_reward_probability > 1e-15 {
                group_outcomes.push(EventOutcome { probability: no_reward_probability, rewards: &[][..] });
            }
            outcomes.push(group_outcomes);
        }
        outcome_indices.clear();
        outcome_indices.resize(outcomes.len(), 0);
        loop {
            let outcome_probability: f64 =
                (0..outcomes.len()).map(|i| outcomes[i][outcome_indices[i]].probability).product::<f64>()
                    * state_probability;
            let mut coupled_gold = [false; 32];
            let mut coupled_hint = [false; 32];
            for i in 0..outcomes.len() {
                for &TargetReward { target: k, level } in outcomes[i][outcome_indices[i]].rewards {
                    coupled_hint[k] = true;
                    if level >= 2 {
                        coupled_gold[k] = true;
                    }
                }
            }
            for (chance_index, &q) in query.inheritance_chances.iter().enumerate() {
                for (outfit, OutfitOption { coverage, .. }) in outfits.iter().enumerate() {
                    let event_chances = &query.event_chances[outfit];
                    // P(all land) and the distribution of how many of the uncertain targets land (fixed golds after)
                    let mut all = fixed;
                    landed_count_distribution[..=uncertain_count + fixed_target_count]
                        .iter_mut()
                        .for_each(|probability| *probability = 0.0);
                    landed_count_distribution[0] = 1.0;
                    for k in 0..uncertain_count {
                        let gold_chance = if coupled_gold[k] { 1.0 } else { 1.0 - no_gold[k] };
                        let miss_hint = if coupled_hint[k] { 0.0 } else { no_hint[k] };
                        let landing_chance = match coverage[k] {
                            2 => GEN_GOLD,
                            1 => GEN_GOLD * gold_chance + GEN_WHITE * (1.0 - gold_chance),
                            _ => {
                                GEN_GOLD * gold_chance
                                    + GEN_WHITE * (1.0 - gold_chance - miss_hint * (1.0 - event_chances[k]) * (1.0 - q))
                            }
                        };
                        all *= landing_chance;
                        for j in (0..=k).rev() {
                            landed_count_distribution[j + 1] += landed_count_distribution[j] * landing_chance;
                            landed_count_distribution[j] *= 1.0 - landing_chance;
                        }
                    }
                    for _ in 0..fixed_target_count {
                        for j in (0..=uncertain_count + fixed_target_count).rev() {
                            if j + 1 < landed_count_distribution.len() {
                                landed_count_distribution[j + 1] += landed_count_distribution[j] * GEN_GOLD;
                            }
                            landed_count_distribution[j] *= 1.0 - GEN_GOLD;
                        }
                    }
                    let all_but_one: f64 = landed_count_distribution[options
                        .target_count
                        .saturating_sub(1)
                        .min(uncertain_count + fixed_target_count)
                        ..=uncertain_count + fixed_target_count]
                        .iter()
                        .sum();
                    let total = &mut totals[chance_index * outfit_count + outfit];
                    total.all += outcome_probability * all;
                    total.all_but_one += outcome_probability * all_but_one;
                }
            }
            let mut i = 0;
            while i < outcomes.len() {
                outcome_indices[i] += 1;
                if outcome_indices[i] < outcomes[i].len() {
                    break;
                }
                outcome_indices[i] = 0;
                i += 1;
            }
            if i == outcomes.len() {
                break;
            }
        }
    }
    (0..chance_count)
        .map(|chance_index| {
            let mut best_outfit = 0;
            for outfit in 1..outfit_count {
                if totals[chance_index * outfit_count + outfit].all
                    > totals[chance_index * outfit_count + best_outfit].all
                {
                    best_outfit = outfit;
                }
            }
            JointChances {
                all: totals[chance_index * outfit_count + best_outfit].all,
                all_but_one: totals[chance_index * outfit_count + best_outfit].all_but_one,
                outfit: best_outfit,
            }
        })
        .collect()
}
