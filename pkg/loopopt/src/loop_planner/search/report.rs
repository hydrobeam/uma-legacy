//! Picks, the loop report (`LoopReport`) and diagnostic summaries from completed loop-search passes.
//! Search and certification belong to `bands`.
use super::Tally;
use super::bands::{DeckBest, Pass, SolvedLoop};
use super::{DeckSources, Multi, RatioExample, SharedDeckKey, lin_of};
use crate::chart::{chart_rows, loop_members};
use crate::checks::UCHK;
use crate::data::dot;
use crate::deck::joint::{JointChances, JointExt, JointQuery, joint_b, joint_fast};
use crate::deck::pdeck::PDeck;
use crate::loop_planner::output::{
    DeckStats, ExactSummary, LoopReport, LoopResult, MemberOutfits, OutfitLabel, RatioDeck, ResultLoop, TargetCover,
};
use crate::loop_planner::schedules::members;
use crate::skills::{GEN_GOLD, MemberLineage, OutfitOption};
use std::cell::Cell;

/// A reported loop: its solved index, each member's deck, per lineage (exact P(all), P(all but one), outfit), value
struct LoopPick<'pass> {
    solved_index: usize,
    decks: Vec<&'pass PDeck>,
    lineage_chances: Vec<JointChances>,
    value: f64,
}

/// `UPPER_CHECK`: the checks' counts on stderr.
pub(super) fn upper_check_summary() {
    if crate::checks::get().upper_check {
        let counts = UCHK.with(Cell::get);
        eprintln!(
            "UPPER_CHECK: {} checked, max bound/exact {:.4}, {} decks not covered",
            counts.checked, counts.max_ratio, counts.uncovered
        );
    }
}

impl Multi<'_> {
    // ---- the answer ----

    /// The output: the reported loops, the search's counts and what the answer rests on, over the deck search's stats
    pub(super) fn report(&self, tally: &Tally, pass: Pass, certified_cap: f64, stats: DeckStats) -> LoopReport {
        let empty_deck = PDeck {
            names: vec![],
            ids: vec![],
            ends: vec![],
            variants: vec![],
            choices: vec![],
            plain_ids: vec![],
            pal_names: vec![],
            pals: 0,
            borrowed: None,
            white_hint_chances: self.no_hint_chances.clone(),
            gold_hint_chances: self.no_hint_chances.clone(),
        };
        // what is reported: per-uma, the best loops; shared, the best distinct decks, each with its best loop
        let picks = if self.shared { self.shared_picks(&pass) } else { self.per_uma_picks(&pass, &empty_deck) };
        let top = picks.iter().map(|pick| self.loop_result(&pass, pick)).collect();
        let budget_out = self.inputs.deck_job.exact && certified_cap < self.maximum_exact_ratio;
        // complete: nothing left to solve (an exhausted queue leaves no ceiling, even with fewer than k values found),
        // or nothing left could beat the k-th best
        let complete = pass.next_bound <= 0.0 || pass.next_bound <= pass.kth_best * (1.0 + 4e-12);
        LoopReport {
            stats,
            top,
            per_uma: !self.shared,
            foursomes: self.total_foursomes(),
            foursomes_ranked: pass.generated,
            foursomes_scanned: pass.scanned,
            foursomes_solved: pass.solved.len(),
            next_bound: pass.next_bound,
            proven: complete,
            searches: 1 + tally.band_decks,
            exact: self.exact_summary(tally, &pass, certified_cap, complete, budget_out),
        }
    }

    /// The structures the job's sizes and required umas allow
    fn total_foursomes(&self) -> usize {
        let (pool_size, required_count) = (self.pool.len(), self.required_umas.len());
        let choose = |n: usize, k: usize| (0..k).fold(1usize, |acc, i| acc * (n - i) / (i + 1));
        let free_count = pool_size - required_count.min(pool_size);
        (if self.allow_four { choose(free_count, 4usize.saturating_sub(required_count)) } else { 0 })
            + (if self.allow_three && required_count <= 3 { choose(free_count, 3 - required_count) } else { 0 })
    }

    /// shared: the best `top_decks` distinct decks (ties by identity), each with the loop it scored best in
    fn shared_picks<'pass>(&self, pass: &'pass Pass) -> Vec<LoopPick<'pass>> {
        let mut ranked: Vec<(&SharedDeckKey, &DeckBest)> = pass.deck_best.iter().collect();
        ranked.sort_by(|a, b| b.1.value.total_cmp(&a.1.value).then(a.0.cmp(b.0)));
        ranked
            .iter()
            .take(self.loop_limit)
            .map(|&(id, &DeckBest { value, solved_index, list_index })| {
                let deck = &pass.lists[list_index]
                    .iter()
                    .find(|listed| {
                        listed.deck.ids == id.card_ids
                            && listed.deck.ends == id.end_links
                            && listed.deck.variants == id.variants
                            && listed.deck.plain_ids.len() == id.plain_pals
                    })
                    .unwrap()
                    .deck;
                let SolvedLoop { members, inheritance_chances, .. } = &pass.solved[solved_index];
                let DeckSources { entries, filler_count, chain_events } = self.ents_of(deck);
                let lineages = lin_of(members);
                let member_count = crate::loop_planner::schedules::members(members).len();
                let mut per_lineage = vec![JointChances::default(); lineages.len()];
                for member in 0..member_count {
                    let member_lineages: Vec<usize> =
                        (0..lineages.len()).filter(|&l| lineages[l].trainee == member).collect();
                    let member_chances: Vec<f64> = member_lineages.iter().map(|&l| inheritance_chances[l]).collect();
                    let results = joint_b(
                        self.inputs.model,
                        JointQuery::unfolded(self.inputs.model, chain_events, members[member], &member_chances),
                        &entries,
                        filler_count,
                        self.requested_target_count,
                        true,
                    );
                    for (i, &l) in member_lineages.iter().enumerate() {
                        per_lineage[l] = results[i];
                    }
                }
                LoopPick { solved_index, decks: vec![deck; member_count], lineage_chances: per_lineage, value }
            })
            .collect()
    }

    /// per-uma: the best `top_decks` loops (ties in solve order), each member with her best deck
    fn per_uma_picks<'pass>(&self, pass: &'pass Pass, empty_deck: &'pass PDeck) -> Vec<LoopPick<'pass>> {
        let solved = &pass.solved;
        let mut order: Vec<usize> = (0..solved.len()).collect();
        // stable: ties keep solve order
        order.sort_by(|&a, &b| solved[b].value.total_cmp(&solved[a].value));
        order
            .iter()
            .take(self.loop_limit)
            .map(|&solved_index| {
                let bests = &solved[solved_index].bests;
                let lineages = lin_of(&solved[solved_index].members);
                let member_count = members(&solved[solved_index].members).len();
                let mut per_lineage = vec![JointChances::default(); lineages.len()];
                for member in 0..member_count {
                    let member_lineages: Vec<usize> =
                        (0..lineages.len()).filter(|&l| lineages[l].trainee == member).collect();
                    for (i, &l) in member_lineages.iter().enumerate() {
                        per_lineage[l] = bests[member].lineage_chances[i];
                    }
                }
                let decks = bests.iter().map(|best| best.deck.as_ref().unwrap_or(empty_deck)).collect();
                LoopPick { solved_index, decks, lineage_chances: per_lineage, value: solved[solved_index].value }
            })
            .collect()
    }

    /// A reported loop: its members, odds, outfits, schedule and detail, with its deck (shared) or decks (per-uma)
    fn loop_result(&self, pass: &Pass, pick: &LoopPick) -> LoopResult {
        let (data, model, uncertain_groups, requested_target_count) =
            (self.inputs.data, self.inputs.model, &self.target_groups, self.requested_target_count);
        let (decks, per_lineage, loop_value) = (&pick.decks, &pick.lineage_chances, pick.value);
        let SolvedLoop { members, schedule, inheritance_chances, .. } = &pass.solved[pick.solved_index];
        let lineages = lin_of(members);
        let member_slice = crate::loop_planner::schedules::members(members);
        let member_count = member_slice.len();
        let lineage_count = lineages.len();
        let mut per_trainee = vec![(0.0, 0usize); member_count];
        let mut picks: Vec<Vec<usize>> = vec![vec![]; member_count];
        for (l, &MemberLineage { trainee: member, .. }) in lineages.iter().enumerate() {
            per_trainee[member].0 += per_lineage[l].all;
            per_trainee[member].1 += 1;
            picks[member].push(per_lineage[l].outfit);
        }
        let minus1 = per_lineage.iter().map(|chances| chances.all_but_one).sum::<f64>() / lineage_count as f64;
        // the chance of exactly k targets, k = 0 ..= n (exact over each lineage's landed sets, as `p_all`), averaged over
        // the lineages: the uncertain targets' sets from the joint model, each guaranteed gold on its own (GEN_GOLD)
        // (none above the exact engine's landed-set limit: the page then shows no per-count table)
        let with_landed = uncertain_groups.len() <= crate::deck::joint::exact::MAX_LANDED_SET_TARGETS;
        let mut landed = vec![0.0f64; requested_target_count + 1];
        // (each member's deck's sources once, read by each of her lineages)
        let deck_sources: Vec<_> = (0..member_count).map(|member| self.ents_of(decks[member])).collect();
        for l in (0..lineage_count).filter(|_| with_landed) {
            let member = lineages[l].trainee;
            let (entries, filler_count) = (&deck_sources[member].entries, deck_sources[member].filler_count);
            let target_count = uncertain_groups.len();
            let mut joint_extras = JointExt {
                scales: crate::deck::joint::SparkScales {
                    uncertain: vec![1.0; target_count],
                    guaranteed: vec![1.0; requested_target_count - target_count],
                },
                marginals: vec![],
                sets: vec![0.0],
                per_outfit: vec![],
            };
            joint_fast(
                model,
                JointQuery::unfolded(
                    model,
                    deck_sources[member].chain_events,
                    member_slice[member],
                    &[inheritance_chances[l]],
                ),
                entries,
                filler_count,
                requested_target_count,
                false,
                Some(&mut joint_extras),
            );
            let outfit = per_lineage[l].outfit;
            let mut guaranteed_landed = vec![1.0f64];
            for _ in target_count..requested_target_count {
                let mut next = vec![0.0; guaranteed_landed.len() + 1];
                for (i, probability) in guaranteed_landed.iter().enumerate() {
                    next[i] += probability * (1.0 - GEN_GOLD);
                    next[i + 1] += probability * GEN_GOLD;
                }
                guaranteed_landed = next;
            }
            for (landed_mask, &set_chance) in
                joint_extras.sets[(outfit << target_count)..((outfit + 1) << target_count)].iter().enumerate()
            {
                if set_chance == 0.0 {
                    continue;
                }
                for (j, &probability) in guaranteed_landed.iter().enumerate() {
                    landed[landed_mask.count_ones() as usize + j] += set_chance * probability / lineage_count as f64;
                }
            }
        }
        // the search's product value of this loop with these decks
        let rank_value = self.fixed
            * (0..lineage_count)
                .map(|l| {
                    model.trainee_fast(
                        member_slice[lineages[l].trainee],
                        inheritance_chances[l],
                        &decks[lineages[l].trainee].white_hint_chances,
                        &decks[lineages[l].trainee].gold_hint_chances,
                    )
                })
                .sum::<f64>()
            / lineage_count as f64;
        let outfits = (0..member_count)
            .map(|i| {
                let outfit_index =
                    *picks[i].iter().max_by_key(|pick| picks[i].iter().filter(|other| other == pick).count()).unwrap();
                let OutfitOption { coverage, outfits: outfit_list } =
                    &model.outfit_options[member_slice[i]][outfit_index];
                let covers: Vec<TargetCover> = coverage
                    .iter()
                    .enumerate()
                    .filter(|(_, level)| **level > 0)
                    .map(|(j, &level)| TargetCover { group: uncertain_groups[j], level })
                    .collect();
                let outfits = if covers.is_empty() {
                    vec![]
                } else {
                    outfit_list.iter().map(|(id, label)| OutfitLabel { id: *id, label: label.clone() }).collect()
                };
                MemberOutfits { outfits, covers }
            })
            .collect();
        let win_chances: Vec<Vec<f64>> =
            (0..member_count).map(|i| data.win_chances(&data.umas[member_slice[i]], &schedule.schedules[i])).collect();
        let deck_odds: Vec<(&[f64], &[f64])> = (0..member_count)
            .map(|member| (&*decks[member].white_hint_chances, &*decks[member].gold_hint_chances))
            .collect();
        let exact_p_all: Vec<f64> = per_lineage.iter().map(|chances| chances.all).collect();
        let detail = model.detail(
            member_slice,
            &|a, b| if self.no_race { 0.0 } else { dot(&win_chances[a], &win_chances[b]) },
            Some(&deck_odds),
            &exact_p_all,
        );
        let result_loop = ResultLoop {
            members: loop_members(data, member_slice),
            p_all: loop_value,
            p_minus1: minus1,
            landed: with_landed.then_some(landed),
            rank_value,
            outfits,
            per_trainee: per_trainee.iter().map(|(sum, count)| sum / *count as f64).collect(),
            mean_proc: inheritance_chances.iter().sum::<f64>() / inheritance_chances.len() as f64,
            schedule: chart_rows(data, member_slice, &schedule.schedules[..member_count]),
            detail,
        };
        if self.shared {
            return LoopResult { r#loop: result_loop, deck: decks[0].out(uncertain_groups), decks: None };
        }
        // the deck most members share, for displays that show one deck
        let base = (0..member_count)
            .max_by_key(|&i| (decks.iter().filter(|deck| deck.names == decks[i].names).count(), member_count - i))
            .unwrap();
        LoopResult {
            r#loop: result_loop,
            deck: decks[base].out(uncertain_groups),
            decks: Some(decks.iter().map(|deck| deck.out(uncertain_groups)).collect()),
        }
    }

    /// What the answer rests on: `rho` the highest cap certified (`rho_target` the one sought), `class_caps` the
    /// assumed cap per structure class (each certified at most at rho), `class_max_ratio` the largest exact / product
    /// odds seen per class on any deck scored here (`max_ratio` of them all); `complete` = the search ended by its stop
    /// rule (no unexamined loop or deck can win under the certified caps)
    fn exact_summary(
        &self,
        tally: &Tally,
        pass: &Pass,
        certified_cap: f64,
        complete: bool,
        budget_out: bool,
    ) -> ExactSummary {
        let class_max_ratios = tally.max_ratio;
        // (the names are shared: an Rc each)
        let class_max_deck = tally.max_deck.each_ref().map(|example| {
            example
                .as_ref()
                .map(|RatioExample { trainee_id, cards }| RatioDeck { trainee: *trainee_id, cards: cards.clone() })
        });
        ExactSummary {
            rho: certified_cap,
            rho_target: self.maximum_exact_ratio,
            class_caps: self.exact_caps,
            max_ratio: class_max_ratios.iter().copied().fold(0.0f64, f64::max),
            class_max_ratio: class_max_ratios,
            class_max_deck,
            complete,
            decks_scored: tally.scored_decks,
            deck_searches: tally.band_decks,
            loops_solved: pass.solved.len(),
            schedule_rejects: pass.schedule_rejects,
            cap_hit: tally.cap_hit,
            budget_out,
            bound_filtered: tally.bound_rejections,
        }
    }
}
