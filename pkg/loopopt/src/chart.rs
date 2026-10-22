//! Race and member labels and structured schedule charts for solver results.
//! Win probabilities come from `data`; schedule optimization belongs to `loop_planner::schedules`.
use crate::data::{Choice, Data, NT, OptRaw, Sched};
use crate::uma::UmaIndex;
use serde_json::{Value, json};

/// Calendar label of turn `turn` (0-based): year, early (E) or late (L) half, month.
pub(crate) fn when(turn: usize) -> String {
    let year = if turn < 24 {
        "Jr"
    } else if turn < 48 {
        "Cl"
    } else {
        "Sr"
    };
    let turn_in_year = turn % 24;
    let month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][turn_in_year / 2];
    format!("{} {}-{}", year, if turn_in_year.is_multiple_of(2) { "E" } else { "L" }, month)
}
/// Short uma label for chart columns.
fn short_name(name: &str) -> String {
    let short_names: &[(&str, &str)] = &[
        ("T.M. Opera O", "Opera O"),
        ("El Condor Pasa", "Pasa"),
        ("Agnes Digital", "Digital"),
        ("Agnes Tachyon", "Tachyon"),
        ("Biwa Hayahide", "Biwa"),
        ("Super Creek", "Creek"),
        ("Gold Ship", "GoldShip"),
        ("Gold City", "GoldCity"),
        ("Symboli Rudolf", "Rudolf"),
        ("Narita Brian", "Brian"),
        ("Narita Taishin", "Taishin"),
        ("Narita Top Road", "TopRoad"),
        ("Sakura Bakushin O", "Bakushin"),
        ("Sakura Chiyono O", "Chiyono"),
        ("Special Week", "SpWeek"),
        ("Silence Suzuka", "Suzuka"),
        ("Tokai Teio", "Teio"),
        ("Grass Wonder", "Grass"),
        ("Hishi Amazon", "Amazon"),
        ("Hishi Akebono", "Akebono"),
        ("Mihono Bourbon", "Bourbon"),
        ("Yamanin Zephyr", "Zephyr"),
        ("Satono Diamond", "Diamond"),
        ("Kitasan Black", "Kitasan"),
        ("Haru Urara", "Urara"),
        ("Smart Falcon", "Falcon"),
        ("Curren Chan", "Curren"),
        ("King Halo", "Halo"),
        ("Nice Nature", "Nature"),
        ("Seiun Sky", "Seiun"),
        ("Taiki Shuttle", "Taiki"),
        ("Meisho Doto", "Doto"),
        ("Mayano Top Gun", "Mayano"),
        ("Air Groove", "Groove"),
        ("Air Shakur", "Shakur"),
        ("Matikanefukukitaru", "Fukukitr"),
        ("Matikanetannhauser", "Tannhaus"),
        ("Copano Rickey", "Copano"),
        ("Wonder Acute", "Acute"),
        ("Nakayama Festa", "Festa"),
        ("Yamanin Zephyr", "Zephyr"),
        ("Kawakami Princess", "Kawakami"),
        ("Fine Motion", "FineMotn"),
    ];
    if let Some((_, short)) = short_names.iter().find(|(full, _)| *full == name) {
        return short.to_string();
    }
    if let Some(rest) = name.strip_prefix("Mejiro ") {
        return rest.chars().take(8).collect();
    }
    name.split_whitespace().next().unwrap_or(name).chars().take(8).collect()
}
/// Race label for the chart (<= 22 chars).
pub(crate) fn chart_race_name(name: &str) -> String {
    let short_names: &[(&str, &str)] = &[
        ("Tokyo Yushun (Japanese Derby)", "Japanese Derby"),
        ("Asahi Hai Futurity Stakes", "Asahi Hai FS"),
        ("Mile Championship Nambu Hai", "MCS Nambu Hai"),
        ("Hanshin Juvenile Fillies", "Hanshin JF"),
        ("Zen-Nippon Junior Yushun", "Zen-Nippon Jr Yushun"),
    ];
    short_names
        .iter()
        .find(|(full, _)| *full == name)
        .map_or_else(|| name.chars().take(22).collect(), |(_, short)| short.to_string())
}

/// The option `uma` runs at `turn` on `schedule`, if she races then.
fn chosen<'data>(data: &'data Data, uma: UmaIndex, schedule: &Sched, turn: usize) -> Option<&'data OptRaw> {
    match schedule[turn] {
        Choice::Opt(option_index) => Some(&data.umas[uma].race_options[turn][option_index as usize]),
        Choice::Rest => None,
    }
}

/// Row label: the race, or "A/B" (alphabetical) when interchangeable alternatives exist at that turn.
fn row_label(data: &Data, members: &[UmaIndex], schedules: &[Sched], turn: usize, race: i32) -> String {
    let race_names = &data.race_names;
    let mut names = vec![chart_race_name(&race_names[race as usize])];
    for alternative in interchangeable(data, members, schedules, turn, race) {
        names.push(chart_race_name(&race_names[alternative as usize]));
    }
    names.sort();
    names.join("/")
}

/// G1s that can replace shared race `race` at `turn` with no change to the score: every member running it there
/// is offered the alternative at that turn with the same win-rate basis (so the same odds at the same streak
/// position), it is nobody's goal, and neither race appears anywhere else in any member's schedule (so all pair
/// overlaps are preserved when all runners switch together).
pub(crate) fn interchangeable(
    data: &Data,
    members: &[UmaIndex],
    schedules: &[Sched],
    turn: usize,
    race: i32,
) -> Vec<i32> {
    let member_count = members.len();
    let appears = |race_index: i32, skip_turn: Option<usize>| {
        (0..member_count).any(|i| {
            (0..NT).any(|other| {
                Some(other) != skip_turn
                    && chosen(data, members[i], &schedules[i], other).is_some_and(|option| option.race == race_index)
            })
        })
    };
    if appears(race, Some(turn)) {
        return vec![];
    }
    let runners: Vec<usize> = (0..member_count)
        .filter(|&i| chosen(data, members[i], &schedules[i], turn).is_some_and(|option| option.race == race))
        .collect();
    if runners.iter().any(|&i| chosen(data, members[i], &schedules[i], turn).is_some_and(|option| option.goal)) {
        return vec![];
    }
    let mut alternatives: Vec<i32> = vec![];
    for &i in &runners {
        for option in &data.umas[members[i]].race_options[turn] {
            if option.race >= 0 && option.race != race && !alternatives.contains(&option.race) {
                alternatives.push(option.race);
            }
        }
    }
    alternatives.retain(|&alternative| {
        !appears(alternative, None)
            && runners.iter().all(|&i| {
                let Some(current) = chosen(data, members[i], &schedules[i], turn) else {
                    unreachable!("runners all run race {race} at turn {turn}")
                };
                data.umas[members[i]].race_options[turn].iter().any(|option| {
                    option.race == alternative && option.win_rate_basis == current.win_rate_basis && !option.goal
                })
            })
    });
    alternatives
}

/// Structured schedule chart: shared G1s (affinity) and optional G1s (one runner, race sparks only).
pub(crate) fn chart_rows(data: &Data, members: &[UmaIndex], schedules: &[Sched]) -> Value {
    let member_count = members.len();
    let streaks: Vec<Vec<usize>> = (0..member_count)
        .map(|i| {
            let mut streak = 0;
            (0..NT)
                .map(|turn| {
                    match schedules[i][turn] {
                        Choice::Rest => streak = 0,
                        Choice::Opt(_) => streak = (streak + 1).min(6),
                    };
                    streak
                })
                .collect()
        })
        .collect();
    let mut runners: Vec<Vec<bool>> = vec![vec![false; member_count]; data.race_count];
    for i in 0..member_count {
        for turn in 0..NT {
            if let Some(chosen) = chosen(data, members[i], &schedules[i], turn)
                && chosen.race >= 0
            {
                runners[chosen.race as usize][i] = true;
            }
        }
    }
    let mut rows: Vec<(usize, i32)> = vec![];
    for turn in 0..NT {
        for i in 0..member_count {
            if let Some(chosen) = chosen(data, members[i], &schedules[i], turn)
                && chosen.race >= 0
                && !rows.contains(&(turn, chosen.race))
            {
                rows.push((turn, chosen.race));
            }
        }
    }
    rows.sort_unstable();
    let rows_json: Vec<Value> = rows
        .iter()
        .map(|&(turn, race)| {
            let mut runner_count = 0;
            let cells: Vec<Value> = (0..member_count)
                .map(|i| match chosen(data, members[i], &schedules[i], turn) {
                    Some(chosen) if chosen.race == race => {
                        runner_count += 1;
                        let streak = streaks[i][turn];
                        // Win chance accounts for aptitude grades (distance + surface, out of 110) and consecutive
                        // races.
                        json!({"p": (data.option_chance(chosen, streak) * 100.0).round(), "goal": chosen.goal,
                           "apt": 110 - chosen.win_rate_basis, "row": streak,
                           "pen": data.streak_penalty(chosen.goal, streak)})
                    }
                    _ => Value::Null,
                })
                .collect();
            let shared = runners[race as usize].iter().filter(|runs_race| **runs_race).count() >= 2;
            let alternatives: Vec<String> = interchangeable(data, members, schedules, turn, race)
                .iter()
                .map(|&alternative| chart_race_name(&data.race_names[alternative as usize]))
                .collect();
            json!({"race": row_label(data, members, schedules, turn, race), "when": when(turn), "year": turn / 24,
               "shared": shared, "cells": cells, "runners": runner_count, "alts": alternatives})
        })
        .collect();
    Value::Array(rows_json)
}

pub(crate) fn member_json(data: &Data, members: &[UmaIndex]) -> Vec<Value> {
    members
        .iter()
        .map(|&member| {
            let uma = &data.umas[member];
            json!({"id": uma.id, "name": uma.name, "short": short_name(&uma.name)})
        })
        .collect()
}
