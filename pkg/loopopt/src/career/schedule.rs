//! Aptitude sparks and career race schedules.
use crate::career::inherit::aff_factor;
use crate::career::pool::{Pool, RaceWin};
use crate::career::{CareerSetup, Lineage};
use crate::chart::{chart_race_name, when};
use crate::data::{Choice, NT, Uma, scored_position};
use crate::loop_planner::schedules::FILL_EPS;
use serde_json::{Value, json};
/// A pink spark's base chance to fire at one April inspiration, per star (0 to 3)
const PINK_RATE: [f64; 4] = [0.0, 0.01, 0.03, 0.05];

/// that chance from a slot of affinity factor `affinity_factor`
#[inline(always)]
pub(crate) fn pink_p(stars: u8, affinity_factor: f64) -> f64 {
    (PINK_RATE[stars as usize] * affinity_factor).min(1.0)
}

/// A legacy's pink spark over a career: her aptitude (data.json `apt` order), the chance it fires at one April
/// inspiration, the legacy slot, stars
#[derive(Clone)]
pub(crate) struct PinkFire {
    pub(crate) aptitude: usize,
    pub(crate) chance: f64,
    pub(crate) slot: usize,
    pub(crate) stars: u8,
}

/// The trainee's aptitude codes (base, after the career start's pinks) and her legacies' pink sparks
pub(crate) struct PinkCareer {
    pub(crate) base: [u8; 10],
    pub(crate) start: [u8; 10],
    pub(crate) fire: Vec<PinkFire>,
}

/// The April inspirations (0-based turns: Classic and Senior early April); a race on that turn already counts them
const INSP_TURNS: [usize; 2] = [30, 54];

/// aptitude index (data.json `apt` order) of a pink factor group: 31-34 short..long, 11 turf, 12 dirt, 21-24 front
/// runner..end closer
const fn pink_apt(group: u32) -> Option<usize> {
    match group {
        31..=34 => Some((group - 31) as usize),
        11 => Some(4),
        12 => Some(5),
        21..=24 => Some((group - 21) as usize + 6),
        _ => None,
    }
}

/// the pink factor group of aptitude k (inverse of `pink_apt`)
pub(crate) const fn apt_pink(k: usize) -> u32 {
    match k {
        0..=3 => 31 + k as u32,
        4 => 11,
        5 => 12,
        _ => 21 + (k as u32 - 6),
    }
}

/// distribution of the number of successes over independent `chances` (Poisson binomial)
fn pbinom(chances: &[f64]) -> Vec<f64> {
    let mut distribution = vec![1.0f64];
    for &p in chances {
        let mut next = vec![0.0f64; distribution.len() + 1];
        for (i, &mass) in distribution.iter().enumerate() {
            next[i] += mass * (1.0 - p);
            next[i + 1] += mass * p;
        }
        distribution = next;
    }
    distribution
}

/// the distribution of the sum of two independent counts with distributions `first` and `second`
fn conv(first: &[f64], second: &[f64]) -> Vec<f64> {
    let mut sum = vec![0.0f64; first.len() + second.len() - 1];
    for (i, &first_mass) in first.iter().enumerate() {
        for (j, &second_mass) in second.iter().enumerate() {
            sum[i + j] += first_mass * second_mass;
        }
    }
    sum
}

/// A career's race schedule (`CareerSetup::schedule`).
pub(crate) struct RaceSchedule {
    /// Every G1 she could enter, in turn order, as the report's rows.
    pub(crate) rows: Value,
    /// Her chance of at least one win, per race index.
    pub(crate) win_chances: Vec<f64>,
}

impl CareerSetup<'_> {
    /// The lineage trainee's aptitudes over the career with her parents (MODEL.md, aptitude sparks): at the start, per
    /// aptitude the six legacies' pink stars summed raise her base grade 1/2/3/4 grades at 1/4/7/10 stars, at most to
    /// A; at each April inspiration (Classic, Senior) every legacy's pink spark fires with base 1/3/5% by stars x (1 +
    /// that legacy's affinity / 100), each firing +1 grade, at most S. None when data.json has no aptitudes.
    pub(crate) fn pink_career(&self, pool: &Pool, lineage: Lineage) -> Option<PinkCareer> {
        let trainee = lineage.trainee;
        let aptitudes = &self.data.umas[trainee].aptitudes;
        if aptitudes.len() < 10 {
            return None;
        }
        let mut base = [0u8; 10];
        base.copy_from_slice(&aptitudes[..10]);
        let mut sum = [0u32; 10];
        let mut fire = vec![];
        for (slot_index, slot) in self.slots(pool, lineage).iter().enumerate() {
            for &(group, stars) in slot.pinks.iter() {
                let (Some(k), stars) = (pink_apt(group), stars.min(3)) else { continue };
                if stars == 0 {
                    continue;
                }
                sum[k] += u32::from(stars);
                fire.push(PinkFire {
                    aptitude: k,
                    chance: pink_p(stars, aff_factor(slot.relation_points, slot.race)),
                    slot: slot_index,
                    stars,
                });
            }
        }
        let mut start = base;
        for k in 0..10 {
            let grade_increase = match sum[k] {
                0 => 0,
                1..=3 => 1,
                4..=6 => 2,
                7..=9 => 3,
                _ => 4,
            };
            if base[k] < 7 {
                start[k] = (base[k] + grade_increase).min(7);
            }
        }
        Some(PinkCareer { base, start, fire })
    }
    /// Trainee t's race table with her pink sparks counted (`pink_career`): per G1 option the win chance per streak
    /// position, the expectation over her distance and surface grades at that turn (before Classic April: the start
    /// grades; then after one inspiration; from Senior April after both). None when nothing changes (no aptitude data,
    /// or her grades and the base data agree at every turn).
    /// Also per turn and option her expected aptitude score (110 - grade costs), as the exporter's bp.
    fn pink_uma(&self, pink_career: &PinkCareer, trainee: crate::uma::UmaIndex) -> Option<(Uma, Vec<Vec<f64>>)> {
        let data = self.data;
        if data.race_distance.len() < data.race_count || data.race_surface.len() < data.race_count {
            return None;
        }
        let uma = &data.umas[trainee];
        // per aptitude, the grade distribution after 0, 1 and 2 inspirations
        let grade_distributions: Vec<[Vec<(u8, f64)>; 3]> = (0..10)
            .map(|k| {
                let fire_chances: Vec<f64> =
                    pink_career.fire.iter().filter(|spark| spark.aptitude == k).map(|spark| spark.chance).collect();
                let one = pbinom(&fire_chances);
                let two = conv(&one, &one);
                let grade_distribution = |success_distribution: &[f64]| {
                    let mut distribution: Vec<(u8, f64)> = vec![];
                    for (i, &weight) in success_distribution.iter().enumerate() {
                        if weight <= 0.0 {
                            continue;
                        }
                        let grade = (pink_career.start[k] as usize + i).min(8) as u8;
                        if let Some(entry) = distribution.iter_mut().find(|entry| entry.0 == grade) {
                            entry.1 += weight;
                        } else {
                            distribution.push((grade, weight));
                        }
                    }
                    distribution
                };
                [vec![(pink_career.start[k], 1.0)], grade_distribution(&one), grade_distribution(&two)]
            })
            .collect();
        if pink_career.start == pink_career.base && pink_career.fire.iter().all(|spark| spark.aptitude >= 6) {
            return None;
        }
        let cost = |grade: u8| -> i32 { [90, 90, 60, 40, 30, 20, 10, 0, 0][grade as usize] };
        let mut adjusted_uma = uma.clone();
        let mut expected_basis: Vec<Vec<f64>> =
            uma.race_options.iter().map(|options| vec![0.0; options.len()]).collect();
        for turn in 0..NT {
            let inspirations = INSP_TURNS.iter().filter(|&&april_turn| turn >= april_turn).count();
            for (option_index, option) in uma.race_options[turn].iter().enumerate() {
                if option.race < 0 {
                    continue;
                }
                let race = option.race as usize;
                let (distance, surface) =
                    (data.race_distance[race] as usize, if data.race_surface[race] == 1 { 4 } else { 5 });
                let mut win_chances = [0.0f64; 7];
                for &(distance_grade, distance_weight) in &grade_distributions[distance][inspirations] {
                    for &(surface_grade, surface_weight) in &grade_distributions[surface][inspirations] {
                        let win_rate_basis = (110 - cost(distance_grade) - cost(surface_grade)).max(-20);
                        expected_basis[turn][option_index] +=
                            distance_weight * surface_weight * f64::from(win_rate_basis);
                        for k in 1..=6 {
                            win_chances[k] += distance_weight
                                * surface_weight
                                * data.attempt_chance(win_rate_basis, scored_position(option.goal, k));
                        }
                    }
                }
                adjusted_uma.choices[turn][option_index + 1].win_chances = win_chances;
            }
        }
        Some((adjusted_uma, expected_basis))
    }
    /// The chance of each pink spark of her veteran (MODEL.md): one, its aptitude uniform over those she ends at A or S
    /// (stars 20/70/10, not here): per aptitude (data.json `apt` order) P(it is the one); exact, the aptitudes'
    /// inspirations independent. The rest: no aptitude at A or S.
    pub(crate) fn pink_gen(pink_career: &PinkCareer) -> [f64; 10] {
        let q: Vec<f64> = (0..10)
            .map(|k| {
                let fire_chances: Vec<f64> =
                    pink_career.fire.iter().filter(|spark| spark.aptitude == k).map(|spark| spark.chance).collect();
                let one = pbinom(&fire_chances);
                let two = conv(&one, &one);
                two.iter()
                    .enumerate()
                    .filter(|(i, _)| pink_career.start[k] as usize + i >= 7)
                    .map(|entry| entry.1)
                    .sum::<f64>()
            })
            .collect();
        let mut spark_chances = [0.0f64; 10];
        for k in 0..10 {
            let others: Vec<f64> = (0..10).filter(|&j| j != k).map(|j| q[j]).collect();
            let others_distribution = pbinom(&others);
            spark_chances[k] =
                q[k] * others_distribution.iter().enumerate().map(|(i, mass)| mass / (i + 1) as f64).sum::<f64>();
        }
        spark_chances
    }
    /// The lineage trainee's race schedule with her parents: a G1 counts each parent's chance of having won it (a win
    /// she shares with a parent adds 3 affinity to each of them when she is a parent herself, and her parents' wins are
    /// the guide to what her later partners will have won), so she runs the races her parents won; every G1 also
    /// counts `FILL_EPS`, the loop planner's tie-break, so a G1 neither parent won enters only where it costs the
    /// parents' races nothing (and with no G1 among her parents' wins she runs the G1s that win the most). Rows: every G1 she could enter, in turn order; `run` false for those she skips, with what she does
    /// that turn. Also her win chance per race index.
    pub(crate) fn schedule(&self, pool: &Pool, lineage: Lineage) -> RaceSchedule {
        let Lineage { trainee, first_parent, second_parent } = lineage;
        let data = self.data;
        let adjusted_uma = self.pink_career(pool, lineage).and_then(|pink_career| self.pink_uma(&pink_career, trainee));
        let uma = adjusted_uma.as_ref().map_or(&data.umas[trainee], |adjusted| &adjusted.0);
        let mut race_weights = vec![FILL_EPS; data.race_count];
        for parent in [first_parent, second_parent] {
            for &RaceWin { race, chance: parent_chance } in &pool.veterans[parent].wins {
                if race < data.race_count {
                    race_weights[race] += parent_chance;
                }
            }
        }
        // her own race table (pinks counted): solved directly (the memo is keyed by uma id)
        let best_response = if adjusted_uma.is_some() {
            data.best_response_solve(uma, &race_weights, None)
        } else {
            data.best_response(uma, &race_weights, None)
        };
        let Some((schedule, _)) = best_response else {
            return RaceSchedule { rows: json!([]), win_chances: vec![0.0; data.race_count] };
        };
        let win_chances = data.win_chances(uma, &schedule);
        let won = |parent: usize, race: usize| {
            pool.veterans[parent].wins.iter().find(|win| win.race == race).map_or(0.0, |win| win.chance)
        };
        let mut rows = vec![];
        let mut k = 0usize;
        for turn in 0..NT {
            let chosen = match schedule[turn] {
                Choice::Rest => {
                    k = 0;
                    None
                }
                Choice::Opt(option_index) => {
                    k = (k + 1).min(6);
                    Some(option_index as usize)
                }
            };
            for (option_index, option) in uma.race_options[turn].iter().enumerate() {
                if option.race < 0 {
                    continue;
                }
                let race = option.race as usize;
                let run = chosen == Some(option_index);
                let instead = match chosen {
                    _ if run => Value::Null,
                    None => json!("rest"),
                    Some(other_option) => json!(uma.race_options[turn][other_option].name),
                };
                // the loop planner's chart row (one member: `cells`, `shared` = a parent won it, so it builds affinity),
                // plus `run`, `p`, `instead`, and `name` (the race's full name)
                let parent_wins = [won(first_parent, race) > 0.0, won(second_parent, race) > 0.0];
                // win chance and aptitude cost (distance + surface grade costs, as 110 - bp; expected) with her pinks
                let option_chances = uma.choices[turn][option_index + 1].win_chances;
                let aptitude_cost = adjusted_uma
                    .as_ref()
                    .map_or(f64::from(110 - option.win_rate_basis), |adjusted| 110.0 - adjusted.1[turn][option_index])
                    .round() as i64;
                let cell = if run {
                    json!({"p": (option_chances[k] * 100.0).round(), "goal": option.goal, "apt": aptitude_cost, "row": k, "pen": data.streak_penalty(option.goal, k)})
                } else {
                    Value::Null
                };
                rows.push(json!({"turn": turn, "when": when(turn), "year": turn / 24, "race": chart_race_name(&data.race_names[race]), "name": data.race_names[race],
                                 "run": run, "skipped": !run, "instead": instead, "p": if run { json!(option_chances[k]) } else { Value::Null }, "goal": option.goal,
                                 "parents": parent_wins, "shared": parent_wins[0] || parent_wins[1], "cells": [cell], "runners": u8::from(run), "alts": []}));
            }
        }
        RaceSchedule { rows: Value::Array(rows), win_chances }
    }
}
