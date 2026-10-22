//! Exact single-uma schedule search and its scratch buffers and scoped best-response memo.
//! Loop-wide optimization belongs to `optimize`; race data and win probabilities belong to `data`.
use crate::FxMap;
use crate::data::{Choice, Data, NT, RaceChoice, Sched, Uma, dot};

const NS: usize = 7; // streak state 0 (rested) ..= 6 (6+)
const HALF: usize = 48; // turns 1..=48 = Junior+Classic, 49..=72 = Senior

#[derive(Default)]
struct Scratch {
    // Phase 2: paired Classic/Senior turns. Values include a terminal layer; choices do not.
    paired_values: Vec<f64>,
    paired_choices: Vec<[u8; 2]>,
    // Phase 1: Junior turns, with phase 2's entry values as the terminal layer.
    junior_values: Vec<f64>,
    junior_choices: Vec<u8>,
}
thread_local! { static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::new(Scratch::default()); }

impl Uma {
    /// EXACT best response: maximize sum_r W[r] * P[r], with P[r] = 1 - prod(1 - p) over attempts.
    /// Junior turns 1..24 run sequentially; then Classic turn t and Senior turn t+24 are decided together,
    /// so a race offered in both years (always exactly 24 turns apart) is scored exactly.
    /// State: (classic streak, senior streak, guessed classic streak at end of turn 48 [only if turn 49 has options], goal mask).
    /// `force`: optional (turn, choice) the schedule must take at that turn.
    /// Implementation notes (performance): choices are codes (0 = rest, o+1 = option o) into per-uma precomputed tables;
    /// loops run choice-pair outer / state inner, but each state sees candidates in the same order (classic choice outer,
    /// senior choice inner, rest first then options by index) with strict '>' updates, so values and argmax match the
    /// straightforward state-outer formulation exactly.
    fn dp_exact(&self, weights: &[f64], force: Option<(usize, Choice)>) -> Option<(Sched, f64)> {
        SCRATCH.with(|scratch| dp_exact_in(self, weights, force, &mut scratch.borrow_mut()))
    }

    /// Allowed choice codes at turn t. A force replaces the list at its turn, but only with a choice the goals allow
    /// there.
    #[inline]
    fn choice_list(&self, turn: usize, force: Option<(usize, Choice)>) -> ([u8; 6], usize) {
        if let Some((forced_turn, forced_choice)) = force
            && forced_turn == turn
        {
            // a force can't override a single-turn goal: forcing rest, or an option outside the goal set, is infeasible
            let code = match forced_choice {
                Choice::Rest => self.forced[turn].is_none().then_some(0u8),
                Choice::Opt(option_index) => {
                    let offered = (option_index as usize) < self.race_options[turn].len();
                    let goals_allow = self.forced[turn].as_ref().is_none_or(|allowed| allowed.contains(&option_index));
                    (offered && goals_allow).then_some(option_index + 1)
                }
            };
            return match code {
                Some(code) => ([code, 0, 0, 0, 0, 0], 1),
                None => ([0; 6], 0),
            };
        }
        self.choice_codes[turn]
    }
}

#[inline]
const fn code_choice(choice_code: u8) -> Choice {
    if choice_code == 0 { Choice::Rest } else { Choice::Opt(choice_code - 1) }
}

fn dp_exact_in(
    uma: &Uma,
    weights: &[f64],
    force: Option<(usize, Choice)>,
    scratch: &mut Scratch,
) -> Option<(Sched, f64)> {
    let goal_state_count = (uma.all_goals + 1) as usize;
    let boundary_streak_count = if uma.race_options[HALF].is_empty() { 1 } else { NS };
    let unreachable_value = f64::NEG_INFINITY;
    let goal_block_size = boundary_streak_count * goal_state_count; // block of (s0, m) states
    let pair_stride = NS * NS * goal_block_size; // stride of one pair index j
    let all_goals = uma.all_goals as usize;
    // streak position after choosing entry e with incoming streak s (also the next streak)
    #[inline(always)]
    const fn next_streak(choice: &RaceChoice, streak: usize) -> usize {
        if choice.rest {
            0
        } else if streak >= 5 {
            6
        } else {
            streak + 1
        }
    }
    #[inline(always)]
    const fn choice_chance(choice: &RaceChoice, k: usize) -> f64 {
        if choice.race >= 0 { choice.win_chances[k] } else { 0.0 }
    }
    // ---- phase 2: pairs j = 0..24 (classic turn 24+j with senior turn 48+j)
    let paired_values = &mut scratch.paired_values;
    let paired_choices = &mut scratch.paired_choices;
    paired_values.clear();
    paired_values.resize(25 * pair_stride, unreachable_value);
    paired_choices.clear();
    paired_choices.resize(24 * pair_stride, [0, 0]);
    for classic_streak in 0..NS {
        for senior_streak in 0..NS {
            for boundary_streak in 0..boundary_streak_count {
                if boundary_streak_count > 1 && classic_streak != boundary_streak {
                    continue;
                }
                paired_values[24 * pair_stride
                    + (classic_streak * NS + senior_streak) * goal_block_size
                    + boundary_streak * goal_state_count
                    + all_goals] = 0.0;
            }
        }
    }
    for j in (0..24).rev() {
        let (classic_turn, senior_turn) = (24 + j, 48 + j);
        let (classic_codes, classic_code_count) = uma.choice_list(classic_turn, force);
        let (senior_codes, senior_code_count) = uma.choice_list(senior_turn, force);
        let (earlier_layers, later_layers) = paired_values.split_at_mut((j + 1) * pair_stride);
        let current_values = &mut earlier_layers[j * pair_stride..];
        let next_values = &later_layers[..pair_stride];
        let best_choices = &mut paired_choices[j * pair_stride..(j + 1) * pair_stride];
        for &classic_code in &classic_codes[..classic_code_count] {
            let classic_choice = &uma.choices[classic_turn][classic_code as usize];
            // per classic streak sc: next streak kc and (separate-race) reward term 0.0 + w*p
            let mut classic_next_streaks = [0usize; NS];
            let mut classic_rewards = [0.0f64; NS];
            let mut classic_chances = [0.0f64; NS];
            for classic_streak in 0..NS {
                let k = next_streak(classic_choice, classic_streak);
                classic_next_streaks[classic_streak] = k;
                classic_chances[classic_streak] = choice_chance(classic_choice, k);
                classic_rewards[classic_streak] = if classic_choice.race >= 0 {
                    0.0 + weights[classic_choice.race as usize] * classic_chances[classic_streak]
                } else {
                    0.0
                };
            }
            for &senior_code in &senior_codes[..senior_code_count] {
                let senior_choice = &uma.choices[senior_turn][senior_code as usize];
                let chosen_goal_bits = (classic_choice.goal_bits | senior_choice.goal_bits) as usize;
                let mut senior_next_streaks = [0usize; NS];
                let mut senior_rewards = [0.0f64; NS];
                let mut senior_chances = [0.0f64; NS];
                for senior_streak in 0..NS {
                    let k = next_streak(senior_choice, senior_streak);
                    senior_next_streaks[senior_streak] = k;
                    senior_chances[senior_streak] = choice_chance(senior_choice, k);
                    senior_rewards[senior_streak] = if senior_choice.race >= 0 {
                        weights[senior_choice.race as usize] * senior_chances[senior_streak]
                    } else {
                        0.0
                    };
                }
                let same = classic_choice.race >= 0 && classic_choice.race == senior_choice.race;
                let shared_race_weight = if same { weights[classic_choice.race as usize] } else { 0.0 };
                // reward for (sc, ss); bit-identical to the original formula:
                //   same race: w * (1 - (1-pc)(1-ps)); otherwise (0.0 + w_c*pc) + w_s*ps, with absent terms dropped
                let reward = |classic_streak: usize, senior_streak: usize| -> f64 {
                    if same {
                        shared_race_weight
                            * (1.0 - (1.0 - classic_chances[classic_streak]) * (1.0 - senior_chances[senior_streak]))
                    } else {
                        classic_rewards[classic_streak] + senior_rewards[senior_streak]
                    }
                };
                if goal_block_size == 1 {
                    for classic_streak in 0..NS {
                        let next_row = &next_values
                            [classic_next_streaks[classic_streak] * NS..classic_next_streaks[classic_streak] * NS + NS];
                        let current_row = &mut current_values[classic_streak * NS..classic_streak * NS + NS];
                        let choice_row = &mut best_choices[classic_streak * NS..classic_streak * NS + NS];
                        for senior_streak in 0..NS {
                            // a next_row value of -inf gives candidate_value == -inf, which never passes the strict '>' (same as skipping it)
                            let candidate_value =
                                reward(classic_streak, senior_streak) + next_row[senior_next_streaks[senior_streak]];
                            if candidate_value > current_row[senior_streak] {
                                current_row[senior_streak] = candidate_value;
                                choice_row[senior_streak] = [classic_code, senior_code];
                            }
                        }
                    }
                } else {
                    for classic_streak in 0..NS {
                        for senior_streak in 0..NS {
                            let pair_reward = reward(classic_streak, senior_streak);
                            let next_block = (classic_next_streaks[classic_streak] * NS
                                + senior_next_streaks[senior_streak])
                                * goal_block_size;
                            let current_block = (classic_streak * NS + senior_streak) * goal_block_size;
                            for boundary_streak in 0..boundary_streak_count {
                                for goals_met in 0..goal_state_count {
                                    let candidate_value = pair_reward
                                        + next_values[next_block
                                            + boundary_streak * goal_state_count
                                            + (goals_met | chosen_goal_bits)];
                                    let state_index = current_block + boundary_streak * goal_state_count + goals_met;
                                    if candidate_value > current_values[state_index] {
                                        current_values[state_index] = candidate_value;
                                        best_choices[state_index] = [classic_code, senior_code];
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // entry value into phase 2 from junior end state (s, m): choose s0 freely; senior streak starts at s0 (or 0)
    let entry = |streak: usize, goals_met: usize| -> (f64, usize) {
        let mut best_entry = (unreachable_value, 0);
        for boundary_streak in 0..boundary_streak_count {
            let senior_streak = if boundary_streak_count > 1 { boundary_streak } else { 0 };
            let candidate_value = paired_values
                [(streak * NS + senior_streak) * goal_block_size + boundary_streak * goal_state_count + goals_met];
            if candidate_value > best_entry.0 {
                best_entry = (candidate_value, boundary_streak);
            }
        }
        best_entry
    };
    // ---- phase 1: junior turns 0..24
    let junior_values = &mut scratch.junior_values;
    let junior_choices = &mut scratch.junior_choices;
    let junior_stride = NS * goal_state_count;
    junior_values.clear();
    junior_values.resize(25 * junior_stride, unreachable_value);
    junior_choices.clear();
    junior_choices.resize(24 * junior_stride, 0);
    for streak in 0..NS {
        for goals_met in 0..goal_state_count {
            junior_values[24 * junior_stride + streak * goal_state_count + goals_met] = entry(streak, goals_met).0;
        }
    }
    for turn in (0..24).rev() {
        let (classic_codes, classic_code_count) = uma.choice_list(turn, force);
        let (earlier_layers, later_layers) = junior_values.split_at_mut((turn + 1) * junior_stride);
        let current_values = &mut earlier_layers[turn * junior_stride..];
        let next_values = &later_layers[..junior_stride];
        let best_choices = &mut junior_choices[turn * junior_stride..(turn + 1) * junior_stride];
        for &choice_code in &classic_codes[..classic_code_count] {
            let choice = &uma.choices[turn][choice_code as usize];
            let chosen_goal_bits = choice.goal_bits as usize;
            for streak in 0..NS {
                let k = next_streak(choice, streak);
                let reward = if choice.race >= 0 { weights[choice.race as usize] * choice.win_chances[k] } else { 0.0 };
                for goals_met in 0..goal_state_count {
                    let candidate_value = next_values[k * goal_state_count + (goals_met | chosen_goal_bits)] + reward;
                    let state_index = streak * goal_state_count + goals_met;
                    if candidate_value > current_values[state_index] {
                        current_values[state_index] = candidate_value;
                        best_choices[state_index] = choice_code;
                    }
                }
            }
        }
    }
    let total = junior_values[0];
    if total == unreachable_value {
        return None;
    }
    let mut schedule = [Choice::Rest; NT];
    let (mut streak, mut goals_met) = (0usize, 0usize);
    for turn in 0..24 {
        let choice_code = junior_choices[turn * junior_stride + streak * goal_state_count + goals_met];
        let choice = &uma.choices[turn][choice_code as usize];
        schedule[turn] = code_choice(choice_code);
        streak = next_streak(choice, streak);
        goals_met |= choice.goal_bits as usize;
    }
    let (_, boundary_streak) = entry(streak, goals_met);
    let (mut classic_streak, mut senior_streak) = (streak, if boundary_streak_count > 1 { boundary_streak } else { 0 });
    for j in 0..24 {
        let [classic_code, senior_code] = paired_choices[j * pair_stride
            + (classic_streak * NS + senior_streak) * goal_block_size
            + boundary_streak * goal_state_count
            + goals_met];
        let (classic_choice, senior_choice) =
            (&uma.choices[24 + j][classic_code as usize], &uma.choices[48 + j][senior_code as usize]);
        schedule[24 + j] = code_choice(classic_code);
        schedule[48 + j] = code_choice(senior_code);
        classic_streak = next_streak(classic_choice, classic_streak);
        senior_streak = next_streak(senior_choice, senior_streak);
        goals_met |= (classic_choice.goal_bits | senior_choice.goal_bits) as usize;
    }
    Some((schedule, total))
}

/// Best-response memo, active only inside `optimize` (one per thread, cleared per call). dp_exact + the evaluation below
/// are a pure, deterministic function of (uma, force, exact weight bits), so a hit returns exactly what a fresh solve
/// would.
/// Key: [uma id << 16 | force code, weight bits...], built in `key` (reused) so a hit allocates nothing.
#[derive(Default)]
struct BrMemo {
    map: FxMap<Box<[u64]>, Option<(Sched, f64)>>,
    key: Vec<u64>,
}
thread_local! { static BR_MEMO: std::cell::RefCell<Option<BrMemo>> = const { std::cell::RefCell::new(None) }; }

impl Data {
    /// Best response (exact): her schedule maximising `weights` · her win chances, `force` taken if given. Wraps
    /// `dp_exact` and cross-checks its value against direct evaluation; memoised inside a `MemoScope`.
    pub(crate) fn best_response(
        &self,
        uma: &Uma,
        weights: &[f64],
        force: Option<(usize, Choice)>,
    ) -> Option<(Sched, f64)> {
        BR_MEMO.with(|cell| {
            let mut borrowed = cell.borrow_mut();
            let Some(memo) = borrowed.as_mut() else {
                drop(borrowed);
                return self.best_response_solve(uma, weights, force);
            };
            // One byte each for turn and option; 255 denotes rest. All-ones denotes no force.
            let force_code: u16 = match force {
                None => u16::MAX,
                Some((turn, Choice::Rest)) => (turn * 256 + 255) as u16,
                Some((turn, Choice::Opt(option))) => (turn * 256 + option as usize) as u16,
            };
            memo.key.clear();
            memo.key.push(u64::from(uma.id) << 16 | u64::from(force_code));
            memo.key.extend(weights.iter().map(|weight| weight.to_bits()));
            if let Some(result) = memo.map.get(&*memo.key) {
                return *result;
            }
            let result = self.best_response_solve(uma, weights, force);
            let key: Box<[u64]> = memo.key.as_slice().into();
            memo.map.insert(key, result);
            result
        })
    }

    /// `best_response` without the memo (for a uma whose tables differ from her roster entry's under the same id).
    pub(crate) fn best_response_solve(
        &self,
        uma: &Uma,
        weights: &[f64],
        force: Option<(usize, Choice)>,
    ) -> Option<(Sched, f64)> {
        let (schedule, dp_value) = uma.dp_exact(weights, force)?;
        let value = dot(weights, &self.win_chances(uma, &schedule));
        assert!((value - dp_value).abs() < 1e-6, "dp_exact value {dp_value} != evaluated {value} for {}", uma.name);
        assert!(uma.feasible(&schedule), "best response produced an infeasible schedule for {}", uma.name);
        Some((schedule, value))
    }
}

/// Enables the best-response memo for the current thread until dropped (restores the previous state).
pub(crate) struct MemoScope(Option<BrMemo>);
impl MemoScope {
    pub(crate) fn new() -> Self {
        Self(BR_MEMO.with(|memo| memo.borrow_mut().replace(BrMemo::default())))
    }
}
impl Drop for MemoScope {
    fn drop(&mut self) {
        let previous = self.0.take();
        BR_MEMO.with(|memo| *memo.borrow_mut() = previous);
    }
}
