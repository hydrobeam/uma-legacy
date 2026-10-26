//! Race and schedule records, relation and ceiling tables, and the win-probability model.
//! Schedule search belongs to `schedule`; loop optimization belongs to `loop_planner::schedules`.
use crate::Fx;
use crate::uma::{UmaIndex, UmaPairTable, UmaTable};
use serde::{Deserialize, Serialize};

pub(crate) const NT: usize = 72;

/// One race option at a turn. `program_id` (the race program id) is read only by the tests, whose fixtures name options
/// by it.
#[derive(Deserialize, Clone)]
pub(crate) struct OptRaw {
    #[cfg_attr(not(test), allow(dead_code, reason = "read only by the tests, whose fixtures name options by it"))]
    #[serde(rename = "prog")]
    pub(crate) program_id: u32,
    pub(crate) race: i32,
    #[serde(rename = "bp")]
    pub(crate) win_rate_basis: i32,
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
    #[serde(rename = "apt")]
    aptitudes: Vec<u8>,
    /// (outfit card id, its aptitude codes) of each outfit whose own aptitudes differ from `apt` (the first outfit's)
    #[serde(default)]
    #[serde(rename = "outfit_apt")]
    outfit_aptitudes: Vec<(u32, Vec<u8>)>,
}
#[derive(Deserialize)]
pub(crate) struct DataRaw {
    race_names: Vec<String>,
    umas: Vec<UmaRaw>,
    /// relation groups: (points, roster indices); pair / tri are rebuilt from them (older data.json files carry the tables)
    #[serde(default)]
    relations: Option<Vec<(i32, Vec<UmaIndex>)>>,
    #[serde(default)]
    #[serde(rename = "pair")]
    pair_relations: Option<UmaTable<UmaTable<i32>>>,
    #[serde(default)]
    #[serde(rename = "tri")]
    trio_relations: Option<UmaTable<UmaTable<UmaTable<i32>>>>,
    #[serde(rename = "pen")]
    streak_penalties: Vec<i32>,
    #[serde(default)]
    #[serde(rename = "pb")]
    pair_ceiling_table: Option<PbRaw>,
    /// per race: distance class (0 short .. 3 long) and surface (1 turf, 2 dirt)
    #[serde(default)]
    #[serde(rename = "race_cat")]
    race_distance: Vec<u8>,
    #[serde(default)]
    #[serde(rename = "race_ground")]
    race_surface: Vec<u8>,
}
/// pair[a][b] / tri[a][b][c]: summed points of the relation groups holding every one of the (distinct) umas.
pub(crate) fn rel_tables(
    roster_len: usize,
    relations: &[(i32, Vec<UmaIndex>)],
) -> (UmaTable<UmaTable<i32>>, UmaTable<UmaTable<UmaTable<i32>>>) {
    let mut pair_relations = UmaTable(vec![UmaTable(vec![0i32; roster_len]); roster_len]);
    let mut trio_relations = UmaTable(vec![UmaTable(vec![UmaTable(vec![0i32; roster_len]); roster_len]); roster_len]);
    for (points, members) in relations {
        for &a in members {
            for &b in members {
                if a == b {
                    continue;
                }
                pair_relations[a][b] += points;
                for &c in members {
                    if c != a && c != b {
                        trio_relations[a][b][c] += points;
                    }
                }
            }
        }
    }
    (pair_relations, trio_relations)
}
/// Precomputed pairwise ceilings (`loopopt pbtable`, merged into data.json by `./build.sh pb`): `fp` is
/// `pb_fingerprint` of the data it was computed from; row a holds PB(a, b) for b > a as 16-hex-digit f64 bit patterns.
#[derive(Deserialize, Serialize)]
pub(crate) struct PbRaw {
    #[serde(rename = "fp")]
    pub(crate) fingerprint: String,
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
    pub(crate) race_options: Vec<Vec<OptRaw>>,
    /// Per turn: she must race one of these options (a single-turn goal).
    pub(crate) forced: Vec<Option<Vec<u8>>>,
    /// Per turn per option: the multi-turn goal bits it meets.
    pub(crate) goal_bits: Vec<Vec<u32>>,
    /// All multi-turn goal bits.
    pub(crate) all_goals: u32,
    /// Per race: her best-case win chance, every offer of it entered at streak position 1.
    pub(crate) best_case_probs: Vec<f64>,
    /// Per turn: choice code 0 = rest, code o + 1 = option o (precomputed for `dp_exact`).
    pub(crate) choices: Vec<[RaceChoice; 6]>,
    /// Joint pink-history tables for repeated races with uncertain aptitudes; empty for base aptitudes.
    pub(crate) repeated_races: Vec<RepeatedRace>,
    /// Per turn: the allowed choice codes when nothing is forced, rest first then options in index order, and their
    /// count.
    pub(crate) choice_codes: Vec<([u8; 6], usize)>,
    /// Aptitude codes (`UmaRaw`, her first outfit's: the grades `race_options`' bp are from); empty in older data.
    pub(crate) aptitudes: Vec<u8>,
    /// (outfit card id, aptitude codes) of each outfit whose own aptitudes differ from `aptitudes`.
    pub(crate) outfit_aptitudes: Vec<(u32, Vec<u8>)>,
}

/// Precomputed per-choice data for dp_exact: race (-1 = none), win chance per streak position k (same values as
/// `Data::option_chance`), goal bits.
#[derive(Clone, Copy)]
pub(crate) struct RaceChoice {
    pub(crate) race: i32,
    pub(crate) rest: bool,
    pub(crate) goal_bits: u32,
    pub(crate) win_chances: [f64; 7],
}
const REST_CHOICE: RaceChoice = RaceChoice { race: -1, rest: true, goal_bits: 0, win_chances: [0.0; 7] };

/// Two offers of one race, exactly 24 turns apart, sharing a career's aptitude history.
#[derive(Clone)]
pub(crate) struct RepeatedRace {
    pub(crate) race: usize,
    pub(crate) classic_turn: usize,
    pub(crate) classic_option: u8,
    pub(crate) senior_option: u8,
    /// E[pClassic * pSenior] by actual streak positions 1..=6 (goal shifts already applied).
    pub(crate) both_wins: [[f64; 7]; 7],
}

impl RepeatedRace {
    pub(crate) fn win_chance(&self, uma: &Uma, classic_streak: usize, senior_streak: usize) -> f64 {
        let classic = &uma.choices[self.classic_turn][self.classic_option as usize + 1];
        let senior = &uma.choices[self.classic_turn + 24][self.senior_option as usize + 1];
        classic.win_chances[classic_streak] + senior.win_chances[senior_streak]
            - self.both_wins[classic_streak][senior_streak]
    }
}

pub(crate) struct Data {
    pub(crate) race_count: usize,
    pub(crate) umas: UmaTable<Uma>,
    pub(crate) pair_relations: UmaTable<UmaTable<i32>>,
    pub(crate) trio_relations: UmaTable<UmaTable<UmaTable<i32>>>,
    /// Attempt win chance by [bp + 20][scored streak position 1..=6].
    win_table: Vec<[f64; 7]>,
    pub(crate) race_names: Vec<String>,
    /// n x n pairwise ceilings from data.json's table, when its fingerprint matches
    pub(crate) pair_ceilings: Option<UmaPairTable<f64>>,
    /// Win-chance penalty for the k-th race in a row (index k - 1), for the schedule's explanations.
    streak_penalties: Vec<i32>,
    pub(crate) race_distance: Vec<u8>,
    pub(crate) race_surface: Vec<u8>,
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
    for &penalty in &raw.streak_penalties {
        hash.write_u64(i64::from(penalty) as u64);
    }
    for uma in &raw.umas {
        hash.write_u64(u64::from(uma.id));
        hash.write_u64(uma.turns.len() as u64);
        for options in &uma.turns {
            hash.write_u64(options.len() as u64);
            for option in options {
                hash.write_u64(i64::from(option.race) as u64);
                hash.write_u64(i64::from(option.win_rate_basis) as u64);
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
    let stored = raw.pair_ceiling_table.as_ref()?;
    let roster_len = raw.umas.len();
    if stored.fingerprint != pb_fingerprint(raw) || stored.rows.len() != roster_len {
        return None;
    }
    let mut table = vec![0.0f64; roster_len * roster_len];
    for (a, row) in stored.rows.iter().enumerate() {
        if row.len() != 16 * (roster_len - 1 - a) {
            return None;
        }
        for (offset, b) in (a + 1..roster_len).enumerate() {
            let value = f64::from_bits(u64::from_str_radix(row.get(16 * offset..16 * offset + 16)?, 16).ok()?);
            table[a * roster_len + b] = value;
            table[b * roster_len + a] = value;
        }
    }
    Some(UmaPairTable { values: table, roster_len })
}

/// The streak position whose penalty a race at streak position `k` takes (`docs/MODEL.md`, win chance in IT): a goal
/// race (one the career's goals require) takes the previous position's, so it pays 0/0/0/5/20/30 where a chosen race
/// pays 0/0/5/20/30/50. It still counts toward the position of the races after it. The streak state stops at 6
/// (6th or later), so a goal race 7th or later in a row scores as the 6th.
pub(crate) const fn scored_position(goal: bool, k: usize) -> usize {
    if goal && k >= 2 { k - 1 } else { k }
}

impl Data {
    /// Win chance of an attempt with win-rate basis `win_rate_basis` (as low as -20) at scored streak position `k`
    /// (1..=6).
    pub(crate) fn attempt_chance(&self, win_rate_basis: i32, k: usize) -> f64 {
        self.win_table[(win_rate_basis + 20) as usize][k]
    }

    /// Win chance of one option at streak position `k` (1..=6), its goal shift included.
    pub(crate) fn option_chance(&self, option: &OptRaw, k: usize) -> f64 {
        self.attempt_chance(option.win_rate_basis, scored_position(option.goal, k))
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

/// Every multi-turn goal in the data lies within one year (Junior, Classic or Senior) and no option belongs to two of
/// them; asserted so that a data change breaking either is seen (the goal-bit DP itself needs neither: it requires
/// only that every goal's bit is set by the end).
fn check_goal_groups(umas: &[UmaRaw]) {
    for uma in umas {
        let mut groups_of_option: std::collections::BTreeMap<(usize, usize), usize> = std::collections::BTreeMap::new();
        for goal in &uma.goals {
            let years: std::collections::BTreeSet<usize> = goal.iter().map(|&(turn, _)| (turn - 1) / 24).collect();
            let turns: std::collections::BTreeSet<usize> = goal.iter().map(|&(turn, _)| turn).collect();
            if turns.len() < 2 {
                continue;
            }
            assert!(years.len() == 1, "{}: multi-turn goal {goal:?} spans more than one year", uma.name);
            for &option in goal {
                *groups_of_option.entry(option).or_default() += 1;
            }
        }
        if let Some((option, _)) = groups_of_option.iter().find(|&(_, &groups)| groups > 1) {
            panic!("{}: option {option:?} belongs to more than one multi-turn goal", uma.name);
        }
    }
}

pub(crate) fn load_raw(raw: DataRaw) -> Data {
    let pair_ceilings = pb_table_from(&raw);
    let roster_len = raw.umas.len();
    check_repeated_races(&raw.umas);
    check_goal_groups(&raw.umas);
    let race_count = raw.race_names.len();
    let mut win_table = vec![[0.0f64; 7]; 140];
    for (i, row) in win_table.iter_mut().enumerate() {
        let win_rate_basis = i as i32 - 20;
        for k in 1..=6 {
            row[k] = f64::from((win_rate_basis - raw.streak_penalties[k - 1]).clamp(0, 100)) / 100.0;
        }
    }
    let mut umas = vec![];
    for uma in raw.umas {
        umas.push(Uma::from_raw(uma, &win_table, race_count));
    }
    let (pair_relations, trio_relations) = match (raw.pair_relations, raw.trio_relations, &raw.relations) {
        (Some(pair_relations), Some(trio_relations), _) => (pair_relations, trio_relations),
        (_, _, Some(relations)) => rel_tables(roster_len, relations),
        _ => panic!("data.json has neither relations nor pair/tri tables"),
    };
    Data {
        race_count,
        umas: UmaTable(umas),
        pair_relations,
        trio_relations,
        win_table,
        race_names: raw.race_names,
        pair_ceilings,
        streak_penalties: raw.streak_penalties,
        race_distance: raw.race_distance,
        race_surface: raw.race_surface,
    }
}
pub(crate) type Sched = [Choice; NT];

impl Uma {
    /// Her tables from her data.json entry: a single-turn goal forces its options at that turn, a multi-turn goal is a
    /// goal bit set on each of its options, and every turn's choices are precomputed for `dp_exact`.
    fn from_raw(raw: UmaRaw, win_table: &[[f64; 7]], race_count: usize) -> Self {
        let mut forced: Vec<Option<Vec<u8>>> = vec![None; NT];
        let mut goal_bits = raw.turns.iter().map(|options| vec![0u32; options.len()]).collect::<Vec<_>>();
        let mut multi_turn_goals = 0;
        for goal in &raw.goals {
            let goal_turns: std::collections::BTreeSet<usize> = goal.iter().map(|&(turn, _)| turn).collect();
            if goal_turns.len() == 1 {
                let turn = *goal_turns.iter().next().unwrap() - 1;
                let allowed: Vec<u8> = goal.iter().map(|&(_, option)| option as u8).collect();
                // two single-turn goals at one turn: only options in both
                forced[turn] = Some(match &forced[turn] {
                    None => allowed,
                    Some(old) => old.iter().copied().filter(|option_index| allowed.contains(option_index)).collect(),
                });
            } else {
                for &(turn, option) in goal {
                    goal_bits[turn - 1][option] |= 1 << multi_turn_goals;
                }
                multi_turn_goals += 1;
            }
        }
        // best case per race: every offer of it entered at streak position 1
        let mut miss = vec![1.0f64; race_count];
        for options in &raw.turns {
            for option in options {
                if option.race >= 0 {
                    miss[option.race as usize] *= 1.0 - win_table[(option.win_rate_basis + 20) as usize][1];
                }
            }
        }
        let mut choices = vec![[REST_CHOICE; 6]; NT];
        let mut choice_codes = vec![([0u8; 6], 0usize); NT];
        for turn in 0..NT {
            assert!(raw.turns[turn].len() <= 5, "more than 5 options at one turn");
            for (option_index, option) in raw.turns[turn].iter().enumerate() {
                let chances = &win_table[(option.win_rate_basis + 20) as usize];
                let mut win_chances = [0.0f64; 7];
                for (k, chance) in win_chances.iter_mut().enumerate().skip(1) {
                    *chance = chances[scored_position(option.goal, k)];
                }
                choices[turn][option_index + 1] = RaceChoice {
                    race: option.race,
                    rest: false,
                    goal_bits: goal_bits[turn][option_index],
                    win_chances,
                };
            }
            let (codes, count) = &mut choice_codes[turn];
            if forced[turn].is_none() {
                codes[*count] = 0;
                *count += 1;
            }
            for option_index in 0..raw.turns[turn].len() {
                if forced[turn].as_ref().is_none_or(|allowed| allowed.contains(&(option_index as u8))) {
                    codes[*count] = option_index as u8 + 1;
                    *count += 1;
                }
            }
        }
        Self {
            id: raw.id,
            name: raw.name,
            race_options: raw.turns,
            forced,
            goal_bits,
            all_goals: (1u32 << multi_turn_goals) - 1,
            best_case_probs: miss.iter().map(|miss_chance| 1.0 - miss_chance).collect(),
            choices,
            repeated_races: vec![],
            choice_codes,
            aptitudes: raw.aptitudes,
            outfit_aptitudes: raw.outfit_aptitudes,
        }
    }

    /// The aptitude codes of her outfit `card` (her first outfit's, `aptitudes`, unless it has its own).
    pub(crate) fn aptitudes_of(&self, card: u32) -> &[u8] {
        self.outfit_aptitudes.iter().find(|(own_card, _)| *own_card == card).map_or(&self.aptitudes, |(_, own)| own)
    }

    /// Whether `schedule` is legal for her: every option exists, every single-turn goal is raced and every multi-turn
    /// goal is met.
    pub(crate) fn feasible(&self, schedule: &Sched) -> bool {
        let mut goals_met = 0u32;
        for turn in 0..NT {
            match schedule[turn] {
                Choice::Rest => {
                    if self.forced[turn].is_some() {
                        return false;
                    }
                }
                Choice::Opt(option_index) => {
                    if (option_index as usize) >= self.race_options[turn].len() {
                        return false;
                    }
                    if let Some(allowed) = &self.forced[turn]
                        && !allowed.contains(&option_index)
                    {
                        return false;
                    }
                    goals_met |= self.goal_bits[turn][option_index as usize];
                }
            }
        }
        goals_met == self.all_goals
    }
}

/// The dot product a · b: a member's weighted score (weights · win chances), or two members' expected shared wins.
pub(crate) fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

impl Data {
    /// Per race, the chance she wins it at least once on `schedule`, averaging over shared pink histories.
    pub(crate) fn win_chances(&self, uma: &Uma, schedule: &Sched) -> Vec<f64> {
        let mut chances = vec![];
        self.win_chances_into(uma, schedule, &mut chances);
        chances
    }

    /// `win_chances` into a reused buffer. Certain aptitudes retain the original miss-product arithmetic.
    pub(crate) fn win_chances_into(&self, uma: &Uma, schedule: &Sched, chances: &mut Vec<f64>) {
        // per race, the chance she misses every attempt, then its complement
        chances.clear();
        chances.resize(self.race_count, 1.0);
        let mut k = 0usize;
        for turn in 0..NT {
            match schedule[turn] {
                Choice::Rest => k = 0,
                Choice::Opt(option_index) => {
                    k = (k + 1).min(6);
                    // choices[turn][option_index + 1].win_chances[k] == option_chance(option, k) bit for bit (copied
                    // from the same table at load)
                    let choice = &uma.choices[turn][option_index as usize + 1];
                    if choice.race >= 0 {
                        chances[choice.race as usize] *= 1.0 - choice.win_chances[k];
                    }
                }
            }
        }
        for chance in chances.iter_mut() {
            *chance = 1.0 - *chance;
        }
        if uma.repeated_races.is_empty() {
            return;
        }
        let mut streaks = [0usize; NT];
        let mut streak = 0;
        for (turn, choice) in schedule.iter().enumerate() {
            streak = if *choice == Choice::Rest { 0 } else { (streak + 1).min(6) };
            streaks[turn] = streak;
        }
        for repeated in &uma.repeated_races {
            let classic_turn = repeated.classic_turn;
            let senior_turn = classic_turn + 24;
            if schedule[classic_turn] == Choice::Opt(repeated.classic_option)
                && schedule[senior_turn] == Choice::Opt(repeated.senior_option)
            {
                chances[repeated.race] = repeated.win_chance(uma, streaks[classic_turn], streaks[senior_turn]);
            }
        }
    }
}
