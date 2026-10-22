//! Picks, JSON results and diagnostic summaries from completed loop-search passes.
//! Search and certification belong to `bands`.
use super::Tally;
use super::bands::{DeckBest, Pass, SolvedLoop};
use super::{DeckSources, Multi, RatioExample, SharedDeckKey, lin_of};
use crate::chart::{chart_rows, member_json};
use crate::checks::{SPT, UCHK};
use crate::data::dot;
use crate::deck::dominance::PDeck;
use crate::deck::joint::{JointChances, JointExt, JointQuery, joint_b, joint_fast};
use crate::loop_planner::schedules::members;
use crate::skills::{GEN_GOLD, MemberLineage, OutfitOption};
use serde_json::{Value, json};
use std::cell::Cell;

/// A reported loop: its solved index, each member's deck, per lineage (exact P(all), P(all but one), outfit), value
struct LoopPick<'pass> {
    solved_index: usize,
    decks: Vec<&'pass PDeck>,
    lineage_chances: Vec<JointChances>,
    value: f64,
}

impl Multi<'_> {
    // ---- the answer ----

    pub(super) fn debug_summary(&self, tally: &Tally, pass: &Pass, certified_cap: f64) {
        let checks = crate::checks::get();
        if checks.upper_check {
            let counts = UCHK.with(Cell::get);
            eprintln!(
                "UPPER_CHECK: {} checked, max bound/exact {:.4}, {} decks not covered",
                counts.checked, counts.max_ratio, counts.uncovered
            );
        }
        if checks.deck_debug {
            let counts = SPT.with(Cell::get);
            eprintln!(
                "deck searches: setup {:.2}s tree {:.2}s configs {} count vectors {}",
                counts.setup_seconds, counts.tree_seconds, counts.configurations, counts.count_vectors
            );
        }
        if checks.deck_debug {
            eprintln!(
                "per-uma search {:.2}s cap {} scanned {} solved {} exact: {} deck searches, {} decks scored, {} bounded out, schedule rejects {} | band {:.2}s score {:.2}s seed-scan {:.2}s bound {:.2}s",
                self.started.elapsed().as_secs_f64(),
                certified_cap,
                pass.scanned,
                pass.solved.len(),
                tally.band_decks,
                tally.scored_decks,
                tally.bound_rejections,
                pass.schedule_rejects,
                tally.band_seconds,
                tally.score_seconds,
                tally.seed_seconds,
                tally.bound_seconds
            );
        }
    }

    /// The output: the reported loops, the search's counts and what the answer rests on, over the deck search's stats
    pub(super) fn report(&self, tally: &Tally, pass: Pass, certified_cap: f64, stats: Value) -> Value {
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
        let top: Vec<Value> = picks.iter().map(|pick| self.loop_json(&pass, pick)).collect();
        let budget_out = self.inputs.deck_job.exact && certified_cap < self.maximum_exact_ratio;
        let complete = pass.next_bound <= pass.kth_best * (1.0 + 4e-12);
        let exact = self.exact_json(tally, &pass, certified_cap, complete, budget_out);
        let band_searches = tally.band_decks;
        let mut out = stats;
        for (key, value) in [
            ("top", json!(top)),
            ("best", json!([])),
            ("per_uma", json!(!self.shared)),
            ("foursomes", json!(self.total_foursomes())),
            ("foursomes_ranked", json!(pass.generated)),
            ("foursomes_scanned", json!(pass.scanned)),
            ("foursomes_solved", json!(pass.solved.len())),
            ("next_bound", json!(pass.next_bound)),
            ("proven", json!(complete)),
            ("searches", json!(1 + band_searches)),
            ("exact", exact),
        ] {
            out[key] = value;
        }
        out
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
        ranked.sort_by(|a, b| b.1.value.partial_cmp(&a.1.value).unwrap().then(a.0.cmp(b.0)));
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
                let DeckSources { entries, filler_count } = self.ents_of(deck);
                let lineages = lin_of(members);
                let member_count = crate::loop_planner::schedules::members(members).len();
                let mut per_lineage = vec![JointChances::default(); lineages.len()];
                for member in 0..member_count {
                    let member_lineages: Vec<usize> =
                        (0..lineages.len()).filter(|&l| lineages[l].trainee == member).collect();
                    let member_chances: Vec<f64> = member_lineages.iter().map(|&l| inheritance_chances[l]).collect();
                    let results = joint_b(
                        self.inputs.model,
                        JointQuery::unfolded(self.inputs.model, self.inputs.deck_job, members[member], &member_chances),
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
        order.sort_by(|&a, &b| solved[b].value.partial_cmp(&solved[a].value).unwrap());
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
    fn loop_json(&self, pass: &Pass, pick: &LoopPick) -> Value {
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
        let mut landed = vec![0.0f64; requested_target_count + 1];
        // (each member's deck's sources once, read by each of her lineages)
        let deck_sources: Vec<_> = (0..member_count).map(|member| self.ents_of(decks[member])).collect();
        for l in 0..lineage_count {
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
                JointQuery::unfolded(model, self.inputs.deck_job, member_slice[member], &[inheritance_chances[l]]),
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
        let outfits: Vec<Value> = (0..member_count)
            .map(|i| {
                let outfit_index =
                    *picks[i].iter().max_by_key(|pick| picks[i].iter().filter(|other| other == pick).count()).unwrap();
                let OutfitOption { coverage, outfits: outfit_list } =
                    &model.outfit_options[member_slice[i]][outfit_index];
                let covers: Vec<Value> = coverage
                    .iter()
                    .enumerate()
                    .filter(|(_, level)| **level > 0)
                    .map(|(j, level)| json!({"group": uncertain_groups[j], "level": level}))
                    .collect();
                let outfits = if covers.is_empty() {
                    vec![]
                } else {
                    outfit_list.iter().map(|outfit| json!({"id": outfit.0, "label": outfit.1})).collect::<Vec<_>>()
                };
                json!({"outfits": outfits, "covers": covers})
            })
            .collect();
        let win_chances: Vec<Vec<f64>> =
            (0..member_count).map(|i| data.win_chances(&data.umas[member_slice[i]], &schedule.schedules[i])).collect();
        let deck_odds: Vec<(&[f64], &[f64])> = (0..member_count)
            .map(|member| (&*decks[member].white_hint_chances, &*decks[member].gold_hint_chances))
            .collect();
        let mut detail = model.detail(
            member_slice,
            &|a, b| if self.no_race { 0.0 } else { dot(&win_chances[a], &win_chances[b]) },
            Some(&deck_odds),
        );
        if let Some(rows) = detail["lineages"].as_array_mut() {
            for (l, row) in rows.iter_mut().enumerate() {
                row["rank_value"] = row["p_all"].clone();
                row["p_all"] = json!(per_lineage[l].all);
            }
        }
        let loop_summary = json!({"members": member_json(data, member_slice), "p_all": loop_value, "p_minus1": minus1,
               "landed": landed, "rank_value": rank_value, "outfits": outfits,
               "per_trainee": per_trainee.iter().map(|(sum, count)| sum / *count as f64).collect::<Vec<_>>(),
               "mean_proc": inheritance_chances.iter().sum::<f64>() / inheritance_chances.len() as f64,
               "schedule": chart_rows(data, member_slice, &schedule.schedules[..member_count]),
               "detail": detail});
        if self.shared {
            return json!({"loop": loop_summary, "deck": decks[0].out(uncertain_groups)});
        }
        // the deck most members share, for displays that show one deck
        let base = (0..member_count)
            .max_by_key(|&i| (decks.iter().filter(|deck| deck.names == decks[i].names).count(), member_count - i))
            .unwrap();
        json!({"loop": loop_summary, "deck": decks[base].out(uncertain_groups),
               "decks": decks.iter().map(|deck| deck.out(uncertain_groups)).collect::<Vec<_>>()})
    }

    /// What the answer rests on: `rho` the highest cap certified (`rho_target` the one sought), `class_caps` the
    /// assumed cap per structure class (each certified at most at rho), `class_max_ratio` the largest exact / product
    /// odds seen per class on any deck scored here (`max_ratio` of them all); `complete` = the search ended by its stop
    /// rule (no unexamined loop or deck can win under the certified caps)
    fn exact_json(&self, tally: &Tally, pass: &Pass, certified_cap: f64, complete: bool, budget_out: bool) -> Value {
        let class_max_ratios = tally.max_ratio;
        let max_deck: Vec<Option<Value>> = tally
            .max_deck
            .iter()
            .map(|example| {
                example
                    .as_ref()
                    .map(|RatioExample { trainee_id, cards }| json!({"trainee": trainee_id, "cards": cards}))
            })
            .collect();
        json!({"rho": certified_cap, "rho_target": self.maximum_exact_ratio, "class_caps": self.exact_caps,
               "max_ratio": class_max_ratios.iter().copied().fold(0.0f64, f64::max),
               "class_max_ratio": class_max_ratios,
               "class_max_deck": max_deck, "complete": complete, "decks_scored": tally.scored_decks,
               "deck_searches": tally.band_decks, "loops_solved": pass.solved.len(),
               "schedule_rejects": pass.schedule_rejects,
               "cap_hit": tally.cap_hit, "budget_out": budget_out, "bound_filtered": tally.bound_rejections})
    }
}
