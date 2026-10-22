//! Race and member labels and structured schedule charts for solver results.
//! Win probabilities come from `data`; schedule optimization belongs to `optimize`.
use crate::data::{Choice, Data, NT, Sched, option_chance, streak_penalty};
use crate::uma::UmaIndex;
use serde_json::{Value, json};

pub(crate) fn when(t: usize) -> String {
    let year = if t < 24 {
        "Jr"
    } else if t < 48 {
        "Cl"
    } else {
        "Sr"
    };
    let turn_in_year = t % 24;
    let month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][turn_in_year / 2];
    format!("{} {}-{}", year, if turn_in_year.is_multiple_of(2) { "E" } else { "L" }, month)
}
/// Short uma label for chart columns.
fn short_name(name: &str) -> String {
    let m: &[(&str, &str)] = &[
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
    if let Some((_, a)) = m.iter().find(|(n, _)| *n == name) {
        return a.to_string();
    }
    if let Some(rest) = name.strip_prefix("Mejiro ") {
        return rest.chars().take(8).collect();
    }
    name.split_whitespace().next().unwrap_or(name).chars().take(8).collect()
}
/// Row label: the race, or "A/B" (alphabetical) when interchangeable alternatives exist at that turn.
fn row_label(d: &Data, m: &[UmaIndex], s: &[Sched], t: usize, race: i32) -> String {
    let mut names = vec![chart_race_name(&d.race_names[race as usize])];
    for a in interchangeable(d, m, s, t, race) {
        names.push(chart_race_name(&d.race_names[a as usize]));
    }
    names.sort();
    names.join("/")
}
/// Race label for the chart (<= 22 chars).
pub(crate) fn chart_race_name(name: &str) -> String {
    let m: &[(&str, &str)] = &[
        ("Tokyo Yushun (Japanese Derby)", "Japanese Derby"),
        ("Asahi Hai Futurity Stakes", "Asahi Hai FS"),
        ("Mile Championship Nambu Hai", "MCS Nambu Hai"),
        ("Hanshin Juvenile Fillies", "Hanshin JF"),
        ("Zen-Nippon Junior Yushun", "Zen-Nippon Jr Yushun"),
    ];
    m.iter().find(|(n, _)| *n == name).map_or_else(|| name.chars().take(22).collect(), |(_, a)| a.to_string())
}

/// G1s that can replace shared race `race` at turn `t` with no change to the score: every member running it there
/// is offered the alternative at that turn with the same win-rate basis (so the same odds at the same streak position),
/// it is nobody's goal, and neither race appears anywhere else in any member's schedule (so all pair overlaps are
/// preserved when all runners switch together).
pub(crate) fn interchangeable(d: &Data, m: &[UmaIndex], s: &[Sched], t: usize, race: i32) -> Vec<i32> {
    let n = m.len();
    let appears = |r: i32, skip_t: Option<usize>| {
        (0..n).any(|i| {
            (0..NT).any(|tt| {
                Some(tt) != skip_t && matches!(s[i][tt], Choice::Opt(o) if d.umas[m[i]].opts[tt][o as usize].race == r)
            })
        })
    };
    if appears(race, Some(t)) {
        return vec![];
    }
    let runners: Vec<usize> = (0..n)
        .filter(|&i| matches!(s[i][t], Choice::Opt(o) if d.umas[m[i]].opts[t][o as usize].race == race))
        .collect();
    if runners.iter().any(|&i| match s[i][t] {
        Choice::Opt(o) => d.umas[m[i]].opts[t][o as usize].goal,
        Choice::Rest => false,
    }) {
        return vec![];
    }
    let mut alts: Vec<i32> = vec![];
    for i in &runners {
        for op in &d.umas[m[*i]].opts[t] {
            if op.race >= 0 && op.race != race && !alts.contains(&op.race) {
                alts.push(op.race);
            }
        }
    }
    alts.retain(|&a| {
        !appears(a, None)
            && runners.iter().all(|&i| {
                let cur = match s[i][t] {
                    Choice::Opt(o) => &d.umas[m[i]].opts[t][o as usize],
                    Choice::Rest => unreachable!("runners all run race {race} at turn {t}"),
                };
                d.umas[m[i]].opts[t].iter().any(|op| op.race == a && op.bp == cur.bp && !op.goal)
            })
    });
    alts
}

/// Structured schedule chart: shared G1s (affinity) and optional G1s (one runner, race sparks only).
pub(crate) fn chart_rows(d: &Data, members: &[UmaIndex], schedules: &[Sched]) -> Value {
    let n = members.len();
    let streaks: Vec<Vec<usize>> = (0..n)
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
    let mut runners: Vec<Vec<bool>> = vec![vec![false; n]; d.nr];
    for i in 0..n {
        for turn in 0..NT {
            if let Choice::Opt(option) = schedules[i][turn] {
                let chosen = &d.umas[members[i]].opts[turn][option as usize];
                if chosen.race >= 0 {
                    runners[chosen.race as usize][i] = true;
                }
            }
        }
    }
    let mut rows: Vec<(usize, i32)> = vec![];
    for turn in 0..NT {
        for i in 0..n {
            if let Choice::Opt(option) = schedules[i][turn] {
                let chosen = &d.umas[members[i]].opts[turn][option as usize];
                if chosen.race >= 0 && !rows.contains(&(turn, chosen.race)) {
                    rows.push((turn, chosen.race));
                }
            }
        }
    }
    rows.sort_unstable();
    let out: Vec<Value> = rows
        .iter()
        .map(|&(turn, race)| {
            let mut runner_count = 0;
            let cells: Vec<Value> = (0..n)
                .map(|i| match schedules[i][turn] {
                    Choice::Opt(option) => {
                        let chosen = &d.umas[members[i]].opts[turn][option as usize];
                        if chosen.race == race {
                            runner_count += 1;
                            let streak = streaks[i][turn];
                            // Win chance accounts for aptitude grades (distance + surface, out of 110) and consecutive races.
                            json!({"p": (option_chance(d, chosen, streak) * 100.0).round(), "goal": chosen.goal,
                           "apt": 110 - chosen.bp, "row": streak,
                           "pen": streak_penalty(d, chosen.goal, streak)})
                        } else {
                            Value::Null
                        }
                    }
                    Choice::Rest => Value::Null,
                })
                .collect();
            let shared = runners[race as usize].iter().filter(|x| **x).count() >= 2;
            let alts: Vec<String> = interchangeable(d, members, schedules, turn, race)
                .iter()
                .map(|&alternative| chart_race_name(&d.race_names[alternative as usize]))
                .collect();
            json!({"race": row_label(d, members, schedules, turn, race), "when": when(turn), "year": turn / 24,
               "shared": shared, "cells": cells, "runners": runner_count, "alts": alts})
        })
        .collect();
    Value::Array(out)
}

pub(crate) fn member_json(d: &Data, m: &[UmaIndex]) -> Vec<Value> {
    m.iter()
        .map(|&x| json!({"id": d.umas[x].id, "name": d.umas[x].name, "short": short_name(&d.umas[x].name)}))
        .collect()
}
