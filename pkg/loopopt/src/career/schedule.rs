//! Aptitude sparks and career race schedules.
use crate::career::inherit::aff_factor;
use crate::career::pool::{Pool, RaceWin};
use crate::career::{CareerSetup, Lineage};
use crate::chart::{chart_race_name, when};
use crate::data::{Choice, Data, NT, RepeatedRace, Uma, scored_position};
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

/// One aptitude's persistent grades after the Classic and Senior inspirations.
struct GradeHistory {
    classic: u8,
    senior: u8,
    mass: f64,
}

struct AptitudeGrades {
    /// Before either inspiration, after Classic, after Senior.
    marginals: [Vec<(u8, f64)>; 3],
    histories: Vec<GradeHistory>,
}

struct PinkUma {
    uma: Uma,
    /// Per turn and option, the expected win-rate basis for the report.
    expected_basis: Vec<Vec<f64>>,
}

/// Trainee t's race table with her pink sparks counted (`pink_career`): per G1 option the win chance per streak
/// position, the expectation over her distance and surface grades at that turn (before Classic April: the start
/// grades; then after one inspiration; from Senior April after both). None when nothing changes (no aptitude data,
/// or her grades and the base data agree at every turn).
/// Also per turn and option her expected aptitude score (110 - grade costs), as the exporter's bp.
fn pink_uma(data: &Data, pink_career: &PinkCareer, trainee: crate::uma::UmaIndex) -> Option<PinkUma> {
    if data.race_distance.len() < data.race_count || data.race_surface.len() < data.race_count {
        return None;
    }
    if pink_career.start == pink_career.base && pink_career.fire.iter().all(|spark| spark.aptitude >= 6) {
        return None;
    }
    let uma = &data.umas[trainee];
    // per aptitude, the grade distribution after 0, 1 and 2 inspirations
    let grade_distributions: Vec<AptitudeGrades> = (0..10)
        .map(|k| {
            let fire_chances: Vec<f64> =
                pink_career.fire.iter().filter(|spark| spark.aptitude == k).map(|spark| spark.chance).collect();
            let one = pbinom(&fire_chances);
            let two = conv(&one, &one);
            let grade_distribution = |start: u8, success_distribution: &[f64]| {
                let mut distribution: Vec<(u8, f64)> = vec![];
                for (i, &weight) in success_distribution.iter().enumerate() {
                    if weight <= 0.0 {
                        continue;
                    }
                    let grade = (start as usize + i).min(8) as u8;
                    if let Some(entry) = distribution.iter_mut().find(|entry| entry.0 == grade) {
                        entry.1 += weight;
                    } else {
                        distribution.push((grade, weight));
                    }
                }
                distribution
            };
            let start = pink_career.start[k];
            let classic_grades = grade_distribution(start, &one);
            let mut histories = vec![];
            for &(classic, classic_mass) in &classic_grades {
                for (senior, senior_mass) in grade_distribution(classic, &one) {
                    histories.push(GradeHistory { classic, senior, mass: classic_mass * senior_mass });
                }
            }
            AptitudeGrades {
                marginals: [vec![(start, 1.0)], classic_grades, grade_distribution(start, &two)],
                histories,
            }
        })
        .collect();
    let cost = |grade: u8| -> i32 { [90, 90, 60, 40, 30, 20, 10, 0, 0][grade as usize] };
    // The adjusted table is an owned variant; the base uma remains available to other lineages.
    let mut adjusted_uma = uma.clone();
    let mut expected_basis: Vec<Vec<f64>> = uma.race_options.iter().map(|options| vec![0.0; options.len()]).collect();
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
            for &(distance_grade, distance_weight) in &grade_distributions[distance].marginals[inspirations] {
                for &(surface_grade, surface_weight) in &grade_distributions[surface].marginals[inspirations] {
                    let win_rate_basis = (110 - cost(distance_grade) - cost(surface_grade)).max(-20);
                    expected_basis[turn][option_index] += distance_weight * surface_weight * f64::from(win_rate_basis);
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
    // Repeated races share the Classic inspiration. Before Classic April the first attempt is
    // deterministic; certain grades also keep the original independent-attempt arithmetic.
    for classic_turn in INSP_TURNS[0]..48 {
        for (classic_option, classic) in uma.race_options[classic_turn].iter().enumerate() {
            if classic.race < 0 {
                continue;
            }
            let Some((senior_option, senior)) =
                uma.race_options[classic_turn + 24].iter().enumerate().find(|(_, option)| option.race == classic.race)
            else {
                continue;
            };
            let race = classic.race as usize;
            let distance = &grade_distributions[data.race_distance[race] as usize];
            let surface = &grade_distributions[if data.race_surface[race] == 1 { 4 } else { 5 }];
            if distance.histories.len() == 1 && surface.histories.len() == 1 {
                continue;
            }
            let mut repeated = RepeatedRace {
                race,
                classic_turn,
                classic_option: classic_option as u8,
                senior_option: senior_option as u8,
                both_wins: [[0.0; 7]; 7],
            };
            for distance in &distance.histories {
                for surface in &surface.histories {
                    let mass = distance.mass * surface.mass;
                    let classic_basis = (110 - cost(distance.classic) - cost(surface.classic)).max(-20);
                    let senior_basis = (110 - cost(distance.senior) - cost(surface.senior)).max(-20);
                    for classic_streak in 1..=6 {
                        let classic_chance =
                            data.attempt_chance(classic_basis, scored_position(classic.goal, classic_streak));
                        for senior_streak in 1..=6 {
                            let senior_chance =
                                data.attempt_chance(senior_basis, scored_position(senior.goal, senior_streak));
                            repeated.both_wins[classic_streak][senior_streak] += mass * classic_chance * senior_chance;
                        }
                    }
                }
            }
            adjusted_uma.repeated_races.push(repeated);
        }
    }
    Some(PinkUma { uma: adjusted_uma, expected_basis })
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
        let adjusted_uma =
            self.pink_career(pool, lineage).and_then(|pink_career| pink_uma(data, &pink_career, trainee));
        let uma = adjusted_uma.as_ref().map_or(&data.umas[trainee], |adjusted| &adjusted.uma);
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
                    .map_or(f64::from(110 - option.win_rate_basis), |adjusted| {
                        110.0 - adjusted.expected_basis[turn][option_index]
                    })
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

#[cfg(test)]
mod tests {
    use super::{PinkCareer, PinkFire, pink_uma};
    use crate::data::{Choice, Data, NT, Sched, load_str};
    use crate::uma::UmaIndex;
    use serde_json::json;

    const COST: [i32; 9] = [90, 90, 60, 40, 30, 20, 10, 0, 0];
    const PENALTY: [i32; 6] = [0, 0, 5, 20, 30, 50];

    /// Forced lead-in races set each attempt's streak; a second Senior option competes with the repeat.
    fn repeat_data(classic_turn: usize, streaks: [usize; 2], goals: [bool; 2], aptitudes: [u8; 10]) -> Data {
        let mut turns = vec![vec![]; NT];
        let mut forced = vec![];
        let basis = 110 - COST[aptitudes[0] as usize] - COST[aptitudes[4] as usize];
        for (year, turn) in [classic_turn, classic_turn + 24].into_iter().enumerate() {
            for lead_in in turn + 1 - streaks[year]..turn {
                turns[lead_in].push(json!({"prog": 0, "race": -1, "bp": 0}));
                forced.push(json!([[lead_in + 1, 0]]));
            }
            turns[turn].push(json!({"prog": 1, "race": 0, "bp": basis, "goal": goals[year]}));
        }
        forced.push(json!([[classic_turn + 1, 0]]));
        turns[classic_turn + 24].push(json!({"prog": 2, "race": 1, "bp": 110}));
        load_str(
            &json!({
                "race_names": ["Repeat", "Alternative"], "race_cat": [0, 1], "race_ground": [1, 2],
                "relations": [], "pen": PENALTY,
                "umas": [{"id": 1, "name": "Synthetic", "apt": aptitudes, "turns": turns, "goals": forced}]
            })
            .to_string(),
        )
    }

    fn repeat_schedule(classic_turn: usize, streaks: [usize; 2]) -> Sched {
        let mut schedule = [Choice::Rest; NT];
        for (year, turn) in [classic_turn, classic_turn + 24].into_iter().enumerate() {
            schedule[turn + 1 - streaks[year]..=turn].fill(Choice::Opt(0));
        }
        schedule
    }

    /// Independent oracle: enumerate every individual spark's two Bernoulli firings, apply them in
    /// turn order, then average the conditional miss product. No production distribution/table helpers.
    fn enumerated_win(pink: &PinkCareer, classic_turn: usize, streaks: [usize; 2], goals: [bool; 2]) -> f64 {
        let mut total = 0.0;
        for firings in 0..(1usize << (2 * pink.fire.len())) {
            let mut grades = pink.start;
            let mut mass = 1.0;
            let mut miss = 1.0;
            let mut inspiration = 0;
            for (year, turn) in [classic_turn, classic_turn + 24].into_iter().enumerate() {
                while inspiration < 2 && turn >= [30, 54][inspiration] {
                    for (index, spark) in pink.fire.iter().enumerate() {
                        let fires = firings & (1 << (inspiration * pink.fire.len() + index)) != 0;
                        if fires {
                            mass *= spark.chance;
                            grades[spark.aptitude] = (grades[spark.aptitude] + 1).min(8);
                        } else {
                            mass *= 1.0 - spark.chance;
                        }
                    }
                    inspiration += 1;
                }
                let position = if goals[year] { streaks[year].saturating_sub(1).max(1) } else { streaks[year] };
                let basis = 110 - COST[grades[0] as usize] - COST[grades[4] as usize];
                let chance = f64::from((basis - PENALTY[position - 1]).clamp(0, 100)) / 100.0;
                miss *= 1.0 - chance;
            }
            // Integrate any inspiration after the final attempt as well, so each bit pattern has its mass.
            for later in inspiration..2 {
                for (index, spark) in pink.fire.iter().enumerate() {
                    mass *= if firings & (1 << (later * pink.fire.len() + index)) != 0 {
                        spark.chance
                    } else {
                        1.0 - spark.chance
                    };
                }
            }
            total += mass * (1.0 - miss);
        }
        total
    }

    #[test]
    fn pink_repeats_match_independent_enumeration() {
        for (classic_turn, distance, surface) in [(35, 4, 6), (29, 4, 6), (35, 7, 7), (35, 8, 8)] {
            let mut start = [7; 10];
            start[0] = distance;
            start[4] = surface;
            let pink = PinkCareer {
                base: start,
                start,
                fire: [(0, 0.17), (0, 0.29), (4, 0.23)]
                    .into_iter()
                    .enumerate()
                    .map(|(slot, (aptitude, chance))| PinkFire { aptitude, chance, slot, stars: 3 })
                    .collect(),
            };
            for classic_streak in 1..=6 {
                for senior_streak in 1..=6 {
                    let streaks = [classic_streak, senior_streak];
                    let goals = [true, false];
                    let data = repeat_data(classic_turn, streaks, goals, start);
                    let adjusted = pink_uma(&data, &pink, UmaIndex(0)).unwrap();
                    let schedule = repeat_schedule(classic_turn, streaks);
                    let expected = enumerated_win(&pink, classic_turn, streaks, goals);
                    let actual = data.win_chances(&adjusted.uma, &schedule)[0];
                    assert!(
                        (actual - expected).abs() < 1e-12,
                        "turn {classic_turn}, streaks {streaks:?}: {actual} != {expected}"
                    );
                    let (_, value) = data.best_response_solve(&adjusted.uma, &[1.0, 0.0], None).unwrap();
                    assert!((value - expected).abs() < 1e-12, "DP value {value} != {expected}");
                    // Entering just one attempt must still use its marginal chance.
                    let mut single = schedule;
                    single[classic_turn + 24] = Choice::Rest;
                    let marginal = adjusted.uma.choices[classic_turn][1].win_chances[classic_streak];
                    assert_eq!(
                        data.win_chances(&adjusted.uma, &single)[0].to_bits(),
                        (1.0 - (1.0 - marginal)).to_bits()
                    );
                }
            }
        }
    }

    #[test]
    fn pink_correlation_changes_the_best_schedule() {
        let mut start = [7; 10];
        start[0] = 4;
        let pink =
            PinkCareer { base: start, start, fire: vec![PinkFire { aptitude: 0, chance: 0.2, slot: 0, stars: 3 }] };
        let data = repeat_data(35, [1, 1], [false, false], start);
        let adjusted = pink_uma(&data, &pink, UmaIndex(0)).unwrap();
        let exact = enumerated_win(&pink, 35, [1, 1], [false, false]);
        assert!((exact - 0.9696).abs() < 1e-12, "analytic control: {exact}");
        // Independent marginals give .9712, above the alternative's .82 + .1504.
        // The exact repeat is below it, so the DP must choose the alternative.
        let (schedule, value) = data.best_response_solve(&adjusted.uma, &[1.0, 0.1504], None).unwrap();
        assert_eq!(schedule[59], Choice::Opt(1));
        assert!((value - 0.9704).abs() < 1e-12, "alternative value: {value}");
    }

    #[test]
    fn no_pink_repeats_keep_the_original_bits() {
        let mut start = [7; 10];
        start[0] = 4;
        let pink = PinkCareer { base: start, start, fire: vec![] };
        for classic_streak in 1..=6 {
            for senior_streak in 1..=6 {
                let streaks = [classic_streak, senior_streak];
                let data = repeat_data(35, streaks, [false, true], start);
                assert!(pink_uma(&data, &pink, UmaIndex(0)).is_none(), "no pinks must use the base table");
                let uma = &data.umas[UmaIndex(0)];
                assert!(uma.repeated_races.is_empty(), "certain aptitudes need no joint table");
                let classic = uma.choices[35][1].win_chances[classic_streak];
                let senior = uma.choices[59][1].win_chances[senior_streak];
                let mut original_miss = 1.0;
                original_miss *= 1.0 - classic;
                original_miss *= 1.0 - senior;
                let expected = 1.0 - original_miss;
                let schedule = repeat_schedule(35, streaks);
                assert_eq!(data.win_chances(uma, &schedule)[0].to_bits(), expected.to_bits());
                let (chosen, value) = data.best_response_solve(uma, &[1.0, 0.0], None).unwrap();
                assert_eq!(chosen, schedule);
                assert_eq!(value.to_bits(), expected.to_bits());
                assert_eq!((1.0 - (1.0 - classic) * (1.0 - senior)).to_bits(), expected.to_bits());
            }
        }
    }
}
