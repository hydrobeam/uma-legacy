//! Race and member labels and structured schedule charts for solver results.
//! Win probabilities come from `data`; schedule optimization belongs to `optimize`.
use crate::data::{Choice, Data, NT, OptRaw, Sched};
use crate::uma::UmaIndex;
use serde_json::{Value, json};

/// Calendar label of turn t (0-based): year, early (E) or late (L) half, month.
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

/// A loop's schedules, read for the chart: `schedules[i]` is the schedule of `members[i]`.
pub(crate) struct ScheduleChart<'chart> {
    pub(crate) data: &'chart Data,
    pub(crate) members: &'chart [UmaIndex],
    pub(crate) schedules: &'chart [Sched],
}

impl ScheduleChart<'_> {
    /// The option member i runs at turn t, if she races then.
    fn chosen(&self, i: usize, t: usize) -> Option<&OptRaw> {
        match self.schedules[i][t] {
            Choice::Opt(o) => Some(&self.data.umas[self.members[i]].opts[t][o as usize]),
            Choice::Rest => None,
        }
    }

    /// Row label: the race, or "A/B" (alphabetical) when interchangeable alternatives exist at that turn.
    fn row_label(&self, t: usize, race: i32) -> String {
        let race_names = &self.data.race_names;
        let mut names = vec![chart_race_name(&race_names[race as usize])];
        for alternative in self.interchangeable(t, race) {
            names.push(chart_race_name(&race_names[alternative as usize]));
        }
        names.sort();
        names.join("/")
    }

    /// G1s that can replace shared race `race` at turn `t` with no change to the score: every member running it there
    /// is offered the alternative at that turn with the same win-rate basis (so the same odds at the same streak
    /// position), it is nobody's goal, and neither race appears anywhere else in any member's schedule (so all pair
    /// overlaps are preserved when all runners switch together).
    pub(crate) fn interchangeable(&self, t: usize, race: i32) -> Vec<i32> {
        let n = self.members.len();
        let appears = |r: i32, skip_turn: Option<usize>| {
            (0..n).any(|i| {
                (0..NT).any(|other| Some(other) != skip_turn && self.chosen(i, other).is_some_and(|op| op.race == r))
            })
        };
        if appears(race, Some(t)) {
            return vec![];
        }
        let runners: Vec<usize> = (0..n).filter(|&i| self.chosen(i, t).is_some_and(|op| op.race == race)).collect();
        if runners.iter().any(|&i| self.chosen(i, t).is_some_and(|op| op.goal)) {
            return vec![];
        }
        let mut alternatives: Vec<i32> = vec![];
        for &i in &runners {
            for op in &self.data.umas[self.members[i]].opts[t] {
                if op.race >= 0 && op.race != race && !alternatives.contains(&op.race) {
                    alternatives.push(op.race);
                }
            }
        }
        alternatives.retain(|&alternative| {
            !appears(alternative, None)
                && runners.iter().all(|&i| {
                    let Some(current) = self.chosen(i, t) else {
                        unreachable!("runners all run race {race} at turn {t}")
                    };
                    self.data.umas[self.members[i]].opts[t]
                        .iter()
                        .any(|op| op.race == alternative && op.bp == current.bp && !op.goal)
                })
        });
        alternatives
    }

    /// Structured schedule chart: shared G1s (affinity) and optional G1s (one runner, race sparks only).
    fn rows(&self) -> Value {
        let d = self.data;
        let n = self.members.len();
        let streaks: Vec<Vec<usize>> = (0..n)
            .map(|i| {
                let mut streak = 0;
                (0..NT)
                    .map(|turn| {
                        match self.schedules[i][turn] {
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
                if let Some(chosen) = self.chosen(i, turn)
                    && chosen.race >= 0
                {
                    runners[chosen.race as usize][i] = true;
                }
            }
        }
        let mut rows: Vec<(usize, i32)> = vec![];
        for turn in 0..NT {
            for i in 0..n {
                if let Some(chosen) = self.chosen(i, turn)
                    && chosen.race >= 0
                    && !rows.contains(&(turn, chosen.race))
                {
                    rows.push((turn, chosen.race));
                }
            }
        }
        rows.sort_unstable();
        let out: Vec<Value> = rows
            .iter()
            .map(|&(turn, race)| {
                let mut runner_count = 0;
                let cells: Vec<Value> = (0..n)
                    .map(|i| match self.chosen(i, turn) {
                        Some(chosen) if chosen.race == race => {
                            runner_count += 1;
                            let streak = streaks[i][turn];
                            // Win chance accounts for aptitude grades (distance + surface, out of 110) and consecutive races.
                            json!({"p": (d.option_chance(chosen, streak) * 100.0).round(), "goal": chosen.goal,
                               "apt": 110 - chosen.bp, "row": streak,
                               "pen": d.streak_penalty(chosen.goal, streak)})
                        }
                        _ => Value::Null,
                    })
                    .collect();
                let shared = runners[race as usize].iter().filter(|x| **x).count() >= 2;
                let alternatives: Vec<String> = self
                    .interchangeable(turn, race)
                    .iter()
                    .map(|&alternative| chart_race_name(&d.race_names[alternative as usize]))
                    .collect();
                json!({"race": self.row_label(turn, race), "when": when(turn), "year": turn / 24,
                   "shared": shared, "cells": cells, "runners": runner_count, "alts": alternatives})
            })
            .collect();
        Value::Array(out)
    }
}

/// Structured schedule chart of `members` on `schedules` (`ScheduleChart::rows`).
pub(crate) fn chart_rows(d: &Data, members: &[UmaIndex], schedules: &[Sched]) -> Value {
    ScheduleChart { data: d, members, schedules }.rows()
}

pub(crate) fn member_json(d: &Data, members: &[UmaIndex]) -> Vec<Value> {
    members
        .iter()
        .map(|&member| {
            let uma = &d.umas[member];
            json!({"id": uma.id, "name": uma.name, "short": short_name(&uma.name)})
        })
        .collect()
}
