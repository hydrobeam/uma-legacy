//! Race and schedule records, relation and ceiling tables, and the win-probability model.
//! Schedule search belongs to `schedule`; loop optimization belongs to `optimize`.
use crate::Fx;
use crate::uma::{UmaIndex, UmaPairTable, UmaTable};
use serde::{Deserialize, Serialize};

pub(crate) const NT: usize = 72;

/// One race option at a turn. `prog` (the race program id) is read only by the tests, whose fixtures name options by it.
#[derive(Deserialize, Clone)]
pub(crate) struct OptRaw {
    #[cfg_attr(not(test), allow(dead_code, reason = "read only by the tests, whose fixtures name options by it"))]
    pub(crate) prog: u32,
    pub(crate) race: i32,
    pub(crate) bp: i32,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) goal: bool,
}
#[derive(Deserialize)]
struct UmaRaw {
    id: u32,
    name: String,
    turns: Vec<Vec<OptRaw>>,
    goals: Vec<Vec<(usize, usize)>>,
    /// aptitude codes 1..8 = G..S: short, mile, middle, long, turf, dirt, front runner, pace chaser, late surger, end closer
    #[serde(default)]
    apt: Vec<u8>,
}
#[derive(Deserialize)]
pub(crate) struct DataRaw {
    race_names: Vec<String>,
    umas: Vec<UmaRaw>,
    /// relation groups: (points, roster indices); pair / tri are rebuilt from them (older data.json files carry the tables)
    #[serde(default)]
    relations: Option<Vec<(i32, Vec<UmaIndex>)>>,
    #[serde(default)]
    pair: Option<UmaTable<UmaTable<i32>>>,
    #[serde(default)]
    tri: Option<UmaTable<UmaTable<UmaTable<i32>>>>,
    pen: Vec<i32>,
    #[serde(default)]
    pb: Option<PbRaw>,
    /// per race: distance class (0 short .. 3 long) and surface (1 turf, 2 dirt)
    #[serde(default)]
    race_cat: Vec<u8>,
    #[serde(default)]
    race_ground: Vec<u8>,
}
/// pair[a][b] / tri[a][b][c]: summed points of the relation groups holding every one of the (distinct) umas.
pub(crate) fn rel_tables(
    n: usize,
    rels: &[(i32, Vec<UmaIndex>)],
) -> (UmaTable<UmaTable<i32>>, UmaTable<UmaTable<UmaTable<i32>>>) {
    let mut pair = UmaTable(vec![UmaTable(vec![0i32; n]); n]);
    let mut tri = UmaTable(vec![UmaTable(vec![UmaTable(vec![0i32; n]); n]); n]);
    for (points, members) in rels {
        for &a in members {
            for &b in members {
                if a == b {
                    continue;
                }
                pair[a][b] += points;
                for &c in members {
                    if c != a && c != b {
                        tri[a][b][c] += points;
                    }
                }
            }
        }
    }
    (pair, tri)
}
/// Precomputed pairwise ceilings (`loopopt pbtable`, merged into data.json by `./build.sh pb`): `fp` is
/// `pb_fingerprint` of the data it was computed from; row a holds PB(a, b) for b > a as 16-hex-digit f64 bit patterns.
#[derive(Deserialize, Serialize)]
pub(crate) struct PbRaw {
    pub(crate) fp: String,
    pub(crate) rows: Vec<String>,
}
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Choice {
    Rest,
    Opt(u8),
}

#[derive(Clone)]
pub(crate) struct Uma {
    pub(crate) id: u32,
    pub(crate) name: String,
    /// Per turn (0-based index = turn - 1): her race options.
    pub(crate) opts: Vec<Vec<OptRaw>>,
    /// Per turn: she must race one of these options (a single-turn goal).
    pub(crate) forced: Vec<Option<Vec<u8>>>,
    /// Per turn per option: the multi-turn goal bits it meets.
    pub(crate) bits: Vec<Vec<u32>>,
    /// All multi-turn goal bits.
    pub(crate) full: u32,
    /// Per race: her best-case win chance, every offer of it entered at streak position 1.
    pub(crate) best_case_probs: Vec<f64>,
    /// Per turn: choice code 0 = rest, code o + 1 = option o (precomputed for `dp_exact`).
    pub(crate) ent: Vec<[Ent; 6]>,
    /// Per turn: the allowed choice codes when nothing is forced, rest first then options in index order, and their
    /// count.
    pub(crate) choice_codes: Vec<([u8; 6], usize)>,
    /// Aptitude codes (`UmaRaw`); empty in older data.
    pub(crate) apt: Vec<u8>,
}

/// Precomputed per-choice data for dp_exact: race (-1 = none), win chance per streak position k (same values as
/// `Data::option_chance`), goal bits.
#[derive(Clone, Copy)]
pub(crate) struct Ent {
    pub(crate) race: i32,
    pub(crate) rest: bool,
    pub(crate) bit: u32,
    pub(crate) p: [f64; 7],
}
const ENT_REST: Ent = Ent { race: -1, rest: true, bit: 0, p: [0.0; 7] };

pub(crate) struct Data {
    pub(crate) nr: usize,
    pub(crate) umas: UmaTable<Uma>,
    pub(crate) pair: UmaTable<UmaTable<i32>>,
    pub(crate) tri: UmaTable<UmaTable<UmaTable<i32>>>,
    /// Attempt win chance by [bp + 20][scored streak position 1..=6].
    win_table: Vec<[f64; 7]>,
    pub(crate) race_names: Vec<String>,
    pub(crate) pbt: Option<UmaPairTable<f64>>, // n x n pairwise ceilings from data.json's table, when its fingerprint matches
    /// Win-chance penalty for the k-th race in a row (index k - 1), for the schedule's explanations.
    streak_penalties: Vec<i32>,
    pub(crate) race_cat: Vec<u8>,
    pub(crate) race_ground: Vec<u8>,
}

/// Bump when best_response / pair_ceilings change in a way that could move a ceiling's bits: stored tables then
/// stop matching and are recomputed.
const PB_VERSION: u64 = 2;
/// Fingerprint of everything a pairwise ceiling depends on: every uma's id, options (race, bp) per turn and goals,
/// the streak penalties and the race count. Fx over fixed-width words, so native and wasm agree.
pub(crate) fn pb_fingerprint(raw: &DataRaw) -> String {
    use std::hash::Hasher;
    let mut hash = Fx::default();
    hash.write_u64(PB_VERSION);
    hash.write_u64(raw.race_names.len() as u64);
    for &penalty in &raw.pen {
        hash.write_u64(i64::from(penalty) as u64);
    }
    for uma in &raw.umas {
        hash.write_u64(u64::from(uma.id));
        hash.write_u64(uma.turns.len() as u64);
        for options in &uma.turns {
            hash.write_u64(options.len() as u64);
            for option in options {
                hash.write_u64(i64::from(option.race) as u64);
                hash.write_u64(i64::from(option.bp) as u64);
            }
        }
        hash.write_u64(uma.goals.len() as u64);
        for goal in &uma.goals {
            hash.write_u64(goal.len() as u64);
            for &(turn, option) in goal {
                hash.write_u64(turn as u64);
                hash.write_u64(option as u64);
            }
        }
    }
    format!("{:016x}", hash.finish())
}
fn pb_table_from(raw: &DataRaw) -> Option<UmaPairTable<f64>> {
    let stored = raw.pb.as_ref()?;
    let n = raw.umas.len();
    if stored.fp != pb_fingerprint(raw) || stored.rows.len() != n {
        return None;
    }
    let mut table = vec![0.0f64; n * n];
    for (a, row) in stored.rows.iter().enumerate() {
        if row.len() != 16 * (n - 1 - a) {
            return None;
        }
        for (offset, b) in (a + 1..n).enumerate() {
            let value = f64::from_bits(u64::from_str_radix(row.get(16 * offset..16 * offset + 16)?, 16).ok()?);
            table[a * n + b] = value;
            table[b * n + a] = value;
        }
    }
    Some(UmaPairTable { values: table, roster_len: n })
}

/// The streak position whose penalty a race at streak position `k` takes (`docs/MODEL.md`, win chance in IT): a goal
/// race (one the career's goals require) takes the previous position's, so it pays 0/0/0/5/20/30 where a chosen race
/// pays 0/0/5/20/30/50. It still counts toward the position of the races after it. The streak state stops at 6
/// (6th or later), so a goal race 7th or later in a row scores as the 6th.
pub(crate) const fn scored_position(goal: bool, k: usize) -> usize {
    if goal && k >= 2 { k - 1 } else { k }
}

impl Data {
    /// Win chance of an attempt with win-rate basis `bp` (as low as -20) at scored streak position `k` (1..=6).
    pub(crate) fn attempt_chance(&self, bp: i32, k: usize) -> f64 {
        self.win_table[(bp + 20) as usize][k]
    }

    /// Win chance of one option at streak position `k` (1..=6), its goal shift included.
    pub(crate) fn option_chance(&self, option: &OptRaw, k: usize) -> f64 {
        self.attempt_chance(option.bp, scored_position(option.goal, k))
    }

    /// The streak penalty (percentage points) an option takes at streak position `k`, for the schedule's explanations
    /// (0 at k = 0).
    pub(crate) fn streak_penalty(&self, goal: bool, k: usize) -> i32 {
        self.streak_penalties.get(scored_position(goal, k).max(1) - 1).copied().unwrap_or(0)
    }
}

pub(crate) fn load(path: &str) -> Data {
    load_str(&std::fs::read_to_string(path).unwrap())
}
/// Win chance of an attempt = clamp(bp - pen[k], 0, 100)%, where the exporter sets bp = 110 - grade costs
/// (distance + surface; A 0, B 10, C 20, D 30, E 40, F 60, G 90) and pen = streak penalties from data.json.
pub(crate) fn load_str(json: &str) -> Data {
    load_raw(serde_json::from_str(json).unwrap())
}

/// `dp_exact` pairs Classic and Senior turns. A repeated race must occur exactly 24 turns apart,
/// never in Junior year and never more than twice. This checks that specific DP precondition;
/// it is not a general validator for all dimensions and indices in the input.
fn check_repeated_races(umas: &[UmaRaw]) {
    for uma in umas {
        let mut turns_by_race: std::collections::BTreeMap<i32, Vec<usize>> = std::collections::BTreeMap::default();
        for (turn, options) in uma.turns.iter().enumerate() {
            for option in options {
                if option.race >= 0 {
                    turns_by_race.entry(option.race).or_default().push(turn);
                }
            }
        }
        for (race, turns) in turns_by_race {
            assert!(turns.len() <= 2, "{}: race {race} offered {} times", uma.name, turns.len());
            if turns.len() == 2 {
                assert!(
                    turns[1] == turns[0] + 24 && turns[0] >= 24,
                    "{}: race {race} offered at turns {turns:?}, not a Classic/Senior pair",
                    uma.name
                );
            }
        }
    }
}

pub(crate) fn load_raw(raw: DataRaw) -> Data {
    let pbt = pb_table_from(&raw);
    let roster_len = raw.umas.len();
    check_repeated_races(&raw.umas);
    let nr = raw.race_names.len();
    let mut win_table = vec![[0.0f64; 7]; 140];
    for (i, row) in win_table.iter_mut().enumerate() {
        let bp = i as i32 - 20;
        for k in 1..=6 {
            row[k] = f64::from((bp - raw.pen[k - 1]).clamp(0, 100)) / 100.0;
        }
    }
    let mut umas = vec![];
    for uma in raw.umas {
        umas.push(Uma::from_raw(uma, &win_table, nr));
    }
    let (pair, tri) = match (raw.pair, raw.tri, &raw.relations) {
        (Some(pair), Some(tri), _) => (pair, tri),
        (_, _, Some(relations)) => rel_tables(roster_len, relations),
        _ => panic!("data.json has neither relations nor pair/tri tables"),
    };
    Data {
        nr,
        umas: UmaTable(umas),
        pair,
        tri,
        win_table,
        race_names: raw.race_names,
        pbt,
        streak_penalties: raw.pen,
        race_cat: raw.race_cat,
        race_ground: raw.race_ground,
    }
}
pub(crate) type Sched = [Choice; NT];

impl Uma {
    /// Her tables from her data.json entry: a single-turn goal forces its options at that turn, a multi-turn goal is a
    /// goal bit set on each of its options, and every turn's choices are precomputed for `dp_exact`.
    fn from_raw(raw: UmaRaw, win_table: &[[f64; 7]], race_count: usize) -> Self {
        let mut forced: Vec<Option<Vec<u8>>> = vec![None; NT];
        let mut bits = raw.turns.iter().map(|options| vec![0u32; options.len()]).collect::<Vec<_>>();
        let mut multi_turn_goals = 0;
        for goal in &raw.goals {
            let goal_turns: std::collections::BTreeSet<usize> = goal.iter().map(|&(turn, _)| turn).collect();
            if goal_turns.len() == 1 {
                let t = *goal_turns.iter().next().unwrap() - 1;
                let allowed: Vec<u8> = goal.iter().map(|&(_, option)| option as u8).collect();
                // two single-turn goals at one turn: only options in both
                forced[t] = Some(match &forced[t] {
                    None => allowed,
                    Some(old) => old.iter().copied().filter(|o| allowed.contains(o)).collect(),
                });
            } else {
                for &(turn, option) in goal {
                    bits[turn - 1][option] |= 1 << multi_turn_goals;
                }
                multi_turn_goals += 1;
            }
        }
        // best case per race: every offer of it entered at streak position 1
        let mut miss = vec![1.0f64; race_count];
        for options in &raw.turns {
            for option in options {
                if option.race >= 0 {
                    miss[option.race as usize] *= 1.0 - win_table[(option.bp + 20) as usize][1];
                }
            }
        }
        let mut ent = vec![[ENT_REST; 6]; NT];
        let mut choice_codes = vec![([0u8; 6], 0usize); NT];
        for t in 0..NT {
            assert!(raw.turns[t].len() <= 5, "more than 5 options at one turn");
            for (o, option) in raw.turns[t].iter().enumerate() {
                let chances = &win_table[(option.bp + 20) as usize];
                let mut p = [0.0f64; 7];
                for (k, chance) in p.iter_mut().enumerate().skip(1) {
                    *chance = chances[scored_position(option.goal, k)];
                }
                ent[t][o + 1] = Ent { race: option.race, rest: false, bit: bits[t][o], p };
            }
            let (codes, count) = &mut choice_codes[t];
            if forced[t].is_none() {
                codes[*count] = 0;
                *count += 1;
            }
            for o in 0..raw.turns[t].len() {
                if forced[t].as_ref().is_none_or(|allowed| allowed.contains(&(o as u8))) {
                    codes[*count] = o as u8 + 1;
                    *count += 1;
                }
            }
        }
        Self {
            id: raw.id,
            name: raw.name,
            opts: raw.turns,
            forced,
            bits,
            full: (1u32 << multi_turn_goals) - 1,
            best_case_probs: miss.iter().map(|m| 1.0 - m).collect(),
            ent,
            choice_codes,
            apt: raw.apt,
        }
    }

    /// Per race, the chance she wins it at least once on `schedule`: 1 - prod(1 - p) over her attempts at it.
    pub(crate) fn win_chances(&self, race_count: usize, schedule: &Sched) -> Vec<f64> {
        let mut chances = vec![];
        self.win_chances_into(race_count, schedule, &mut chances);
        chances
    }

    /// `win_chances` into a reused buffer (same arithmetic, so bit-identical).
    pub(crate) fn win_chances_into(&self, race_count: usize, schedule: &Sched, chances: &mut Vec<f64>) {
        // per race, the chance she misses every attempt, then its complement
        chances.clear();
        chances.resize(race_count, 1.0);
        let mut k = 0usize;
        for t in 0..NT {
            match schedule[t] {
                Choice::Rest => k = 0,
                Choice::Opt(o) => {
                    k = (k + 1).min(6);
                    // ent[t][o+1].p[k] == option_chance(op, k) bit for bit (copied from the same table at load)
                    let e = &self.ent[t][o as usize + 1];
                    if e.race >= 0 {
                        chances[e.race as usize] *= 1.0 - e.p[k];
                    }
                }
            }
        }
        for chance in chances.iter_mut() {
            *chance = 1.0 - *chance;
        }
    }

    /// Whether `schedule` is legal for her: every option exists, every single-turn goal is raced and every multi-turn
    /// goal is met.
    pub(crate) fn feasible(&self, schedule: &Sched) -> bool {
        let mut goals_met = 0u32;
        for t in 0..NT {
            match schedule[t] {
                Choice::Rest => {
                    if self.forced[t].is_some() {
                        return false;
                    }
                }
                Choice::Opt(o) => {
                    if (o as usize) >= self.opts[t].len() {
                        return false;
                    }
                    if let Some(allowed) = &self.forced[t]
                        && !allowed.contains(&o)
                    {
                        return false;
                    }
                    goals_met |= self.bits[t][o as usize];
                }
            }
        }
        goals_met == self.full
    }
}

/// The dot product a · b: a member's weighted score (weights · win chances), or two members' expected shared wins.
pub(crate) fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
