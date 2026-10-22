//! Aptitude sparks and career race schedules.
use crate::career::inherit::aff_factor;
use crate::career::pool::{Pool, RaceWin};
use crate::career::{Env, Lineage};
use crate::chart::{chart_race_name, when};
use crate::data::{Choice, NT, Uma, pk, probs, scored_position, streak_penalty};
use crate::schedule::best_response;
use serde_json::{Value, json};
/// A pink spark's base chance to fire at one April inspiration, per star (0 to 3)
const PINK_RATE: [f64; 4] = [0.0, 0.01, 0.03, 0.05];

/// that chance from a slot of affinity factor `fa`
#[inline(always)]
pub(crate) fn pink_p(st: u8, fa: f64) -> f64 {
    (PINK_RATE[st as usize] * fa).min(1.0)
}

/// A legacy's pink spark over a career: her aptitude (data.json `apt` order), the chance it fires at one April
/// inspiration, the legacy slot, stars
#[derive(Clone)]
pub(crate) struct PinkFire {
    pub(crate) apt: usize,
    pub(crate) p: f64,
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
const fn pink_apt(g: u32) -> Option<usize> {
    match g {
        31..=34 => Some((g - 31) as usize),
        11 => Some(4),
        12 => Some(5),
        21..=24 => Some((g - 21) as usize + 6),
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

/// distribution of the number of successes over independent chances ps (Poisson binomial)
fn pbinom(ps: &[f64]) -> Vec<f64> {
    let mut v = vec![1.0f64];
    for &p in ps {
        let mut w = vec![0.0f64; v.len() + 1];
        for (i, &x) in v.iter().enumerate() {
            w[i] += x * (1.0 - p);
            w[i + 1] += x * p;
        }
        v = w;
    }
    v
}

fn conv(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut w = vec![0.0f64; a.len() + b.len() - 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            w[i + j] += x * y;
        }
    }
    w
}

/// A career's race schedule (`Env::schedule`).
pub(crate) struct RaceSchedule {
    /// Every G1 she could enter, in turn order, as the report's rows.
    pub(crate) rows: Value,
    /// Her chance of at least one win, per race index.
    pub(crate) win_chances: Vec<f64>,
}

impl Env<'_> {
    /// The lineage trainee's aptitudes over the career with her parents (MODEL.md, aptitude sparks): at the start, per
    /// aptitude the six legacies' pink stars summed raise her base grade 1/2/3/4 grades at 1/4/7/10 stars, at most to
    /// A; at each April inspiration (Classic, Senior) every legacy's pink spark fires with base 1/3/5% by stars x (1 +
    /// that legacy's affinity / 100), each firing +1 grade, at most S. None when data.json has no aptitudes.
    pub(crate) fn pink_career(&self, p: &Pool, lineage: Lineage) -> Option<PinkCareer> {
        let t = lineage.trainee;
        let apt = &self.data.umas[t].apt;
        if apt.len() < 10 {
            return None;
        }
        let mut base = [0u8; 10];
        base.copy_from_slice(&apt[..10]);
        let mut sum = [0u32; 10];
        let mut fire = vec![];
        for (si, sl) in self.slots(p, lineage).iter().enumerate() {
            for &(g, st) in sl.pinks.iter() {
                let (Some(k), st) = (pink_apt(g), st.min(3)) else { continue };
                if st == 0 {
                    continue;
                }
                sum[k] += u32::from(st);
                fire.push(PinkFire { apt: k, p: pink_p(st, aff_factor(sl.rel, sl.race)), slot: si, stars: st });
            }
        }
        let mut start = base;
        for k in 0..10 {
            let up = match sum[k] {
                0 => 0,
                1..=3 => 1,
                4..=6 => 2,
                7..=9 => 3,
                _ => 4,
            };
            if base[k] < 7 {
                start[k] = (base[k] + up).min(7);
            }
        }
        Some(PinkCareer { base, start, fire })
    }
    /// Trainee t's race table with her pink sparks counted (`pink_career`): per G1 option the win chance per streak
    /// position, the expectation over her distance and surface grades at that turn (before Classic April: the start
    /// grades; then after one inspiration; from Senior April after both). None when nothing changes (no aptitude data,
    /// or her grades and the base data agree at every turn).
    /// Also per turn and option her expected aptitude score (110 - grade costs), as the exporter's bp.
    fn pink_uma(&self, pc: &PinkCareer, t: crate::uma::UmaIndex) -> Option<(Uma, Vec<Vec<f64>>)> {
        let d = self.data;
        if d.race_cat.len() < d.nr || d.race_ground.len() < d.nr {
            return None;
        }
        let u = &d.umas[t];
        // per aptitude, the grade distribution after 0, 1 and 2 inspirations
        let dist: Vec<[Vec<(u8, f64)>; 3]> = (0..10)
            .map(|k| {
                let ps: Vec<f64> = pc.fire.iter().filter(|f| f.apt == k).map(|f| f.p).collect();
                let one = pbinom(&ps);
                let two = conv(&one, &one);
                let g = |x: &[f64]| {
                    let mut m: Vec<(u8, f64)> = vec![];
                    for (i, &w) in x.iter().enumerate() {
                        if w <= 0.0 {
                            continue;
                        }
                        let gr = (pc.start[k] as usize + i).min(8) as u8;
                        if let Some(e) = m.iter_mut().find(|e| e.0 == gr) {
                            e.1 += w;
                        } else {
                            m.push((gr, w));
                        }
                    }
                    m
                };
                [vec![(pc.start[k], 1.0)], g(&one), g(&two)]
            })
            .collect();
        if pc.start == pc.base && pc.fire.iter().all(|f| f.apt >= 6) {
            return None;
        }
        let cost = |gr: u8| -> i32 { [90, 90, 60, 40, 30, 20, 10, 0, 0][gr as usize] };
        let mut v = u.clone();
        let mut ebp: Vec<Vec<f64>> = u.opts.iter().map(|l| vec![0.0; l.len()]).collect();
        for tt in 0..NT {
            let ph = INSP_TURNS.iter().filter(|&&x| tt >= x).count();
            for (o, op) in u.opts[tt].iter().enumerate() {
                if op.race < 0 {
                    continue;
                }
                let r = op.race as usize;
                let (kd, ks) = (d.race_cat[r] as usize, if d.race_ground[r] == 1 { 4 } else { 5 });
                let mut pr = [0.0f64; 7];
                for &(gd, wd) in &dist[kd][ph] {
                    for &(gs, ws) in &dist[ks][ph] {
                        let bp = (110 - cost(gd) - cost(gs)).max(-20);
                        ebp[tt][o] += wd * ws * f64::from(bp);
                        for k in 1..=6 {
                            pr[k] += wd * ws * pk(d, bp, scored_position(op.goal, k));
                        }
                    }
                }
                v.ent[tt][o + 1].p = pr;
            }
        }
        Some((v, ebp))
    }
    /// The chance of each pink spark of her veteran (MODEL.md): one, its aptitude uniform over those she ends at A or S
    /// (stars 20/70/10, not here): per aptitude (data.json `apt` order) P(it is the one); exact, the aptitudes'
    /// inspirations independent. The rest: no aptitude at A or S.
    pub(crate) fn pink_gen(pc: &PinkCareer) -> [f64; 10] {
        let q: Vec<f64> = (0..10)
            .map(|k| {
                let ps: Vec<f64> = pc.fire.iter().filter(|f| f.apt == k).map(|f| f.p).collect();
                let one = pbinom(&ps);
                let two = conv(&one, &one);
                two.iter().enumerate().filter(|(i, _)| pc.start[k] as usize + i >= 7).map(|x| x.1).sum::<f64>()
            })
            .collect();
        let mut out = [0.0f64; 10];
        for k in 0..10 {
            let others: Vec<f64> = (0..10).filter(|&j| j != k).map(|j| q[j]).collect();
            let n = pbinom(&others);
            out[k] = q[k] * n.iter().enumerate().map(|(i, w)| w / (i + 1) as f64).sum::<f64>();
        }
        out
    }
    /// The lineage trainee's race schedule with her parents: every G1 win counts one (a G1 on her record can be shared
    /// with a later partner), plus one per parent who won that race (a win she shares with a parent adds 3 affinity to
    /// each of them when she is a parent herself), so she runs the G1s that win the most and, where two clash, the
    /// shared one. Rows: every G1 she could enter, in turn order; `run` false for those she skips, with what she does
    /// that turn. Also her win chance per race index.
    pub(crate) fn schedule(&self, p: &Pool, lineage: Lineage) -> RaceSchedule {
        let Lineage { trainee: t, first_parent: a, second_parent: b } = lineage;
        let d = self.data;
        let pu = self.pink_career(p, lineage).and_then(|pc| self.pink_uma(&pc, t));
        let u = pu.as_ref().map_or(&d.umas[t], |x| &x.0);
        let mut wr = vec![1.0f64; d.nr];
        for x in [a, b] {
            for &RaceWin { race: r, chance: pr } in &p.veterans[x].wins {
                if r < d.nr {
                    wr[r] += pr;
                }
            }
        }
        // her own race table (pinks counted): solved directly (the memo is keyed by uma id)
        let br = if pu.is_some() {
            crate::schedule::best_response_solve(d, u, &wr, None)
        } else {
            best_response(d, u, &wr, None)
        };
        let Some((s, _)) = br else { return RaceSchedule { rows: json!([]), win_chances: vec![0.0; d.nr] } };
        let wp = probs(d, u, &s);
        let won = |x: usize, r: usize| p.veterans[x].wins.iter().find(|w| w.race == r).map_or(0.0, |w| w.chance);
        let mut rows = vec![];
        let mut k = 0usize;
        for tt in 0..NT {
            let chosen = match s[tt] {
                Choice::Rest => {
                    k = 0;
                    None
                }
                Choice::Opt(o) => {
                    k = (k + 1).min(6);
                    Some(o as usize)
                }
            };
            for (o, op) in u.opts[tt].iter().enumerate() {
                if op.race < 0 {
                    continue;
                }
                let r = op.race as usize;
                let run = chosen == Some(o);
                let instead = match chosen {
                    _ if run => Value::Null,
                    None => json!("rest"),
                    Some(c) => json!(u.opts[tt][c].name),
                };
                // the loop planner's chart row (one member: `cells`, `shared` = a parent won it, so it builds affinity),
                // plus `run`, `p`, `instead`, and `name` (the race's full name)
                let par = [won(a, r) > 0.0, won(b, r) > 0.0];
                // win chance and aptitude cost (distance + surface grade costs, as 110 - bp; expected) with her pinks
                let pw = u.ent[tt][o + 1].p;
                let apt = pu.as_ref().map_or(f64::from(110 - op.bp), |x| 110.0 - x.1[tt][o]).round() as i64;
                let cell = if run {
                    json!({"p": (pw[k] * 100.0).round(), "goal": op.goal, "apt": apt, "row": k, "pen": streak_penalty(d, op.goal, k)})
                } else {
                    Value::Null
                };
                rows.push(json!({"turn": tt, "when": when(tt), "year": tt / 24, "race": chart_race_name(&d.race_names[r]), "name": d.race_names[r],
                                 "run": run, "skipped": !run, "instead": instead, "p": if run { json!(pw[k]) } else { Value::Null }, "goal": op.goal,
                                 "parents": par, "shared": par[0] || par[1], "cells": [cell], "runners": u8::from(run), "alts": []}));
            }
        }
        RaceSchedule { rows: Value::Array(rows), win_chances: wp }
    }
}
