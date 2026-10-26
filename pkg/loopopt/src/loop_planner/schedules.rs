//! Loop affinity scoring, coordinated schedule optimization and pairwise ceilings.
//! Single-uma best responses belong to `schedule`; chart formatting belongs to `chart`.
use crate::data::{Choice, Data, NT, Sched, Uma, dot};
use crate::schedule::MemoScope;
use crate::uma::{NONE, UmaIndex, UmaTable};

/// Race weight C of every pair and the trio coefficient of the base, with grandparent slots at half weight. Per lineage,
/// parent slots carry pair(T,Pi) + pair(P1,P2)/2 + 3*O(P1,P2)/2 each and a grandparent slot tri(T,Pi,G) + 3*O(Pi,G),
/// scaled by w; summed over the 12 self-copy-free lineages and normalized to 4 runs: base = 2*sum pair + 4w*sum trio,
/// C = (2+8w)/3 for every pair. w = 1/2 gives C = 2, trio coefficient 2. C is uniform over the pairs for any w, so w only
/// scales the objective: the schedules are those of plain total affinity.
pub(crate) const C_PAIR: f64 = 2.0;
const K_TRIO: f64 = 2.0;

/// The schedule problem of a loop of N members: N = 4 (the 12-lineage loop) or N = 3 (a three-uma rotation, whose
/// race weights are uniform too: see `skills::lins`). Its score is `base` + 3·C·Σ_{i<j} <P_i, P_j>, P_i member i's win
/// chances per race on her schedule.
pub(crate) struct LoopProblem<'data, const N: usize> {
    data: &'data Data,
    members: [UmaIndex; N],
    /// The score's schedule-free part: 2·Σ pair + K_TRIO·Σ trio over the members.
    pub(crate) base: f64,
}

impl<'data, const N: usize> LoopProblem<'data, N> {
    pub(crate) fn new(data: &'data Data, members: [UmaIndex; N]) -> Self {
        let mut pair_sum = 0.0;
        let mut trio_sum = 0.0;
        for i in 0..N {
            for j in i + 1..N {
                pair_sum += f64::from(data.pair_relations[members[i]][members[j]]);
                for k in j + 1..N {
                    trio_sum += f64::from(data.trio_relations[members[i]][members[j]][members[k]]);
                }
            }
        }
        Self { data, members, base: 2.0 * pair_sum + K_TRIO * trio_sum }
    }

    fn member(&self, i: usize) -> &'data Uma {
        &self.data.umas[self.members[i]]
    }

    /// Every member's win chances per race on her schedule.
    fn member_win_chances(&self, schedules: &[Sched; N]) -> Vec<Vec<f64>> {
        (0..N).map(|i| self.data.win_chances(self.member(i), &schedules[i])).collect()
    }

    pub(crate) fn score(&self, schedules: &[Sched; N]) -> f64 {
        self.score_from_chances(&self.member_win_chances(schedules))
    }

    /// Score from precomputed win chances (chances[i] must equal member i's on her schedule; win_chances is
    /// deterministic, so the result is bit-identical to `score`).
    fn score_from_chances(&self, chances: &[Vec<f64>]) -> f64 {
        let mut total = self.base;
        for i in 0..N {
            for j in i + 1..N {
                total += 3.0 * C_PAIR * dot(&chances[i], &chances[j]);
            }
        }
        total
    }

    /// Member i's race weights against her partners: 3·C·Σ_{j≠i} P_j[r].
    pub(crate) fn weights(&self, chances: &[Vec<f64>], i: usize) -> Vec<f64> {
        let mut weights = vec![];
        self.weights_into(chances, i, &mut weights);
        weights
    }

    fn weights_into(&self, chances: &[Vec<f64>], i: usize, weights: &mut Vec<f64>) {
        let race_count = self.data.race_count;
        weights.clear();
        weights.resize(race_count, 0.0);
        for j in 0..chances.len() {
            if j != i {
                for race in 0..race_count {
                    weights[race] += 3.0 * C_PAIR * chances[j][race];
                }
            }
        }
    }

    /// Best-response dynamics: each member re-solves against the others until none improves by more than 1e-9.
    fn ascent(&self, schedules: &mut [Sched; N]) -> f64 {
        let mut chances = self.member_win_chances(schedules);
        self.ascent_from_chances(schedules, &mut chances)
    }

    /// `ascent` with the members' win chances supplied (chances[i] must equal member i's on schedules[i]); they are
    /// kept equal.
    fn ascent_from_chances(&self, schedules: &mut [Sched; N], chances: &mut [Vec<f64>]) -> f64 {
        let race_count = self.data.race_count;
        let mut weights: Vec<f64> = Vec::with_capacity(race_count);
        // stale[i]: some partner's schedule changed since member i was last solved (or she was never solved here).
        // If not stale, her weights are bit-identical to her last solve, so the deterministic best response is the same
        // schedule: either it was rejected then (and would be again) or it was accepted and now equals schedules[i]
        // exactly (its value equals the current one, not a strict improvement). Skipping her therefore cannot change the
        // trajectory.
        let mut stale = [true; N];
        loop {
            let mut improved = false;
            for i in 0..N {
                if !stale[i] {
                    continue;
                }
                stale[i] = false;
                let uma = self.member(i);
                self.weights_into(chances, i, &mut weights);
                let current_value = dot(&weights, &chances[i]);
                if let Some((response, value)) = self.data.best_response(uma, &weights, None)
                    && value > current_value + 1e-9
                {
                    schedules[i] = response;
                    self.data.win_chances_into(uma, &response, &mut chances[i]);
                    improved = true;
                    for j in 0..N {
                        if j != i {
                            stale[j] = true;
                        }
                    }
                }
            }
            if !improved {
                break;
            }
        }
        self.score_from_chances(chances) // chances[i] == member i's on schedules[i] is maintained above
    }

    /// The races any member is offered at turn t, after rest (None): the targets of a group move there.
    fn turn_targets(&self, turn: usize) -> Vec<Option<i32>> {
        let mut targets: Vec<Option<i32>> = vec![None];
        for i in 0..N {
            for option in &self.member(i).race_options[turn] {
                if option.race >= 0 && !targets.contains(&Some(option.race)) {
                    targets.push(Some(option.race));
                }
            }
        }
        targets
    }

    /// Member i's choice that hits `target` (a race, or rest for None) at turn t; None when her options or goals
    /// don't allow it.
    fn target_choice(&self, i: usize, turn: usize, target: Option<i32>) -> Option<Choice> {
        let uma = self.member(i);
        match target {
            None => uma.forced[turn].is_none().then_some(Choice::Rest),
            Some(race) => {
                let option_index = uma.race_options[turn].iter().position(|option| option.race == race)? as u8;
                let goals_allow = uma.forced[turn].as_ref().is_none_or(|allowed| allowed.contains(&option_index));
                goals_allow.then_some(Choice::Opt(option_index))
            }
        }
    }

    /// Coordinated moves: force a subset of >=2 members onto the same race (or rest) at one turn, then re-ascend.
    fn group_moves(&self, schedules: &mut [Sched; N], margin: f64) -> f64 {
        let race_count = self.data.race_count;
        let mut value = self.score(schedules);
        // win chances on the current *schedules, refreshed whenever they change (win_chances is deterministic, so this
        // equals recomputing per mask)
        let mut current_chances = self.member_win_chances(schedules);
        let mut chances: Vec<Vec<f64>> = current_chances.clone();
        let mut weights: Vec<f64> = Vec::with_capacity(race_count);
        loop {
            let mut improved = false;
            for turn in 0..NT {
                for target in self.turn_targets(turn) {
                    // choice for each member to hit the target at t (None = can't)
                    let mut choices: [Option<Choice>; N] = [None; N];
                    let mut on_target = [false; N];
                    for i in 0..N {
                        let choice = self.target_choice(i, turn, target);
                        choices[i] = choice;
                        on_target[i] = choice.is_some_and(|choice| schedules[i][turn] == choice);
                    }
                    let movable: Vec<usize> = (0..N).filter(|&i| choices[i].is_some() && !on_target[i]).collect();
                    let on_target_count: usize = on_target.iter().filter(|x| **x).count();
                    if movable.is_empty() {
                        continue;
                    }
                    if target.is_none() && on_target_count == 0 && movable.len() < 2 {
                        continue;
                    }
                    // subsets of movable with (subset + already-on) >= 2 members and subset >= 2 (single-member switches
                    // are covered by best responses); if nobody is on target, require all movable
                    for mask in 1u32..(1 << movable.len()) {
                        let subset_size = mask.count_ones() as usize;
                        if subset_size + on_target_count < 2 || subset_size == 1 {
                            continue;
                        }
                        if on_target_count == 0 && target.is_some() && subset_size != movable.len() {
                            continue;
                        }
                        let mut trial = *schedules;
                        let mut feasible = true;
                        chances.clone_from(&current_chances);
                        for (bit, &i) in movable.iter().enumerate() {
                            if mask & (1 << bit) == 0 {
                                continue;
                            }
                            let uma = self.member(i);
                            self.weights_into(&chances, i, &mut weights);
                            if let Some((response, _)) =
                                self.data.best_response(uma, &weights, Some((turn, choices[i].unwrap())))
                            {
                                trial[i] = response;
                                self.data.win_chances_into(uma, &response, &mut chances[i]);
                            } else {
                                feasible = false;
                                break;
                            }
                        }
                        // chances[i] == member i's on trial[i] here
                        if !feasible || self.score_from_chances(&chances) < value - margin {
                            continue;
                        }
                        let trial_value = self.ascent_from_chances(&mut trial, &mut chances); // keeps chances on trial
                        if trial_value > value + 1e-7 {
                            value = trial_value;
                            *schedules = trial;
                            improved = true;
                            current_chances.clone_from(&chances);
                        }
                    }
                }
            }
            if !improved {
                break;
            }
        }
        value
    }

    /// Every member's best response against partners at their best-case win chances.
    fn optimistic_start(&self) -> [Sched; N] {
        let chances: Vec<Vec<f64>> = (0..N).map(|i| self.member(i).best_case_probs.clone()).collect();
        let mut schedules = [[Choice::Rest; NT]; N];
        for i in 0..N {
            let weights = self.weights(&chances, i);
            schedules[i] = self.data.best_response(self.member(i), &weights, None).expect("infeasible goals").0;
        }
        schedules
    }

    /// Optimistic start, ascent and group moves, then `iterations` rounds of iterated local search (1-2 members
    /// perturbed against noisy weights, re-ascent, group moves), then the optional G1s that cost no affinity.
    fn solve(&self, iterations: usize, seed: u64, margin: f64) -> [Sched; N] {
        let mut rng = Rng(seed.wrapping_mul(2_654_435_761).wrapping_add(1) | 1);
        let mut schedules = self.optimistic_start();
        let mut value = self.ascent(&mut schedules);
        value = value.max(self.group_moves(&mut schedules, margin));
        for _ in 0..iterations {
            let mut candidate_schedules = schedules;
            let mut candidate_chances = self.member_win_chances(&candidate_schedules);
            let moves = 1 + (rng.next() % 2) as usize;
            for _ in 0..moves {
                let i = (rng.next() % N as u64) as usize;
                let mut noisy_weights = self.weights(&candidate_chances, i);
                for weight in &mut noisy_weights {
                    *weight *= 0.2 + 1.6 * rng.unif();
                }
                if let Some((response, _)) = self.data.best_response(self.member(i), &noisy_weights, None) {
                    candidate_schedules[i] = response;
                    candidate_chances[i] = self.data.win_chances(self.member(i), &response);
                }
            }
            self.ascent_from_chances(&mut candidate_schedules, &mut candidate_chances);
            let candidate_value = self.group_moves(&mut candidate_schedules, margin);
            if candidate_value > value + 1e-7 {
                value = candidate_value;
                schedules = candidate_schedules;
            }
        }
        // add optional G1s that cost no affinity (race sparks)
        self.fill_optional(&mut schedules, value);
        schedules
    }

    /// Optional G1s that cost no affinity: the tie-break pass, then the costless entries. Returns the score.
    fn fill_optional(&self, schedules: &mut [Sched; N], value: f64) -> f64 {
        let value = self.tie_break_pass(schedules, value);
        self.enter_costless_g1s(schedules, value)
    }

    /// Tie-break pass: each member re-solves against her partners with a tiny bonus (FILL_EPS per expected G1 win) on
    /// every G1, so races that cost no affinity get added (they can still pass down race sparks). Accepted only if the
    /// affinity score does not drop by more than 1e-6; races nobody else runs leave every partner's weights unchanged.
    fn tie_break_pass(&self, schedules: &mut [Sched; N], value: f64) -> f64 {
        let mut current = self.score(schedules);
        debug_assert!(
            (current - value).abs() < 1e-6,
            "fill_optional: the caller's score {value} is not the schedule's {current}"
        );
        for i in 0..N {
            let chances = self.member_win_chances(schedules);
            let weights: Vec<f64> = self.weights(&chances, i).iter().map(|weight| weight + FILL_EPS).collect();
            if let Some((response, _)) = self.data.best_response(self.member(i), &weights, None) {
                let mut trial = *schedules;
                trial[i] = response;
                let trial_value = self.score(&trial);
                if trial_value >= current - 1e-6 {
                    *schedules = trial;
                    current = trial_value;
                }
            }
        }
        current
    }

    /// Enter every remaining G1 that costs nothing, even at 0% (the table's 0% may really be a few percent): on each
    /// rest turn, try the best-odds G1 offered; keep it only if the loop's score does not drop at all AND none of that
    /// uma's other attempts (goal races included) loses win chance from the longer streak.
    fn enter_costless_g1s(&self, schedules: &mut [Sched; N], value: f64) -> f64 {
        let mut current = value;
        // per turn: the win chance of her attempt there (-1 where she makes none)
        let attempt_chances = |uma: &Uma, schedule: &Sched| -> Vec<f64> {
            let mut k = 0usize;
            (0..NT)
                .map(|turn| match schedule[turn] {
                    Choice::Rest => {
                        k = 0;
                        -1.0
                    }
                    Choice::Opt(option_index) => {
                        k = (k + 1).min(6);
                        let option = &uma.race_options[turn][option_index as usize];
                        if option.race >= 0 { self.data.option_chance(option, k) } else { -1.0 }
                    }
                })
                .collect()
        };
        loop {
            let mut changed = false;
            for i in 0..N {
                let uma = self.member(i);
                for turn in 0..NT {
                    if !matches!(schedules[i][turn], Choice::Rest) || uma.forced[turn].is_some() {
                        continue;
                    }
                    let mut race_options: Vec<usize> = (0..uma.race_options[turn].len())
                        .filter(|&option_index| uma.race_options[turn][option_index].race >= 0)
                        .collect();
                    race_options.sort_by_key(|&option_index| {
                        std::cmp::Reverse(uma.race_options[turn][option_index].win_rate_basis)
                    });
                    for option_index in race_options {
                        let mut trial = *schedules;
                        trial[i][turn] = Choice::Opt(option_index as u8);
                        if !uma.feasible(&trial[i]) {
                            continue;
                        }
                        let (before, after) = (attempt_chances(uma, &schedules[i]), attempt_chances(uma, &trial[i]));
                        if (0..NT).any(|other| other != turn && before[other] >= 0.0 && after[other] < before[other]) {
                            continue;
                        }
                        let trial_value = self.score(&trial);
                        if trial_value >= current - 1e-12 {
                            *schedules = trial;
                            current = current.max(trial_value);
                            changed = true;
                            break;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        current
    }
}

struct Rng(u64);
impl Rng {
    const fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unif(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Bonus per expected G1 win in the tie-break pass (`LoopProblem::tie_break_pass`).
pub(crate) const FILL_EPS: f64 = 1e-7;

/// Schedules found for a loop, in member order; a three-member structure fills s[0..3] and leaves s[3] all rest.
#[derive(Clone)]
pub(crate) struct Res {
    pub(crate) schedules: [Sched; 4],
}
/// The members of a structure stored as [UmaIndex; 4] (three members: the fourth is `NONE`).
pub(crate) fn members(structure: &[UmaIndex; 4]) -> &[UmaIndex] {
    if structure[3] == NONE { &structure[..3] } else { &structure[..] }
}

/// The loop's schedules (`LoopProblem::solve`), with the best-response memo active for the search.
pub(crate) fn optimize<const N: usize>(
    data: &Data,
    members: [UmaIndex; N],
    iterations: usize,
    seed: u64,
    margin: f64,
) -> Res {
    let _memo = MemoScope::new();
    let schedules = LoopProblem::new(data, members).solve(iterations, seed, margin);
    let mut result_schedules = [[Choice::Rest; NT]; 4];
    result_schedules[..N].copy_from_slice(&schedules);
    Res { schedules: result_schedules }
}
/// `optimize` for a structure of three or four members (`members` of a [UmaIndex; 4] with `NONE` last for three).
pub(crate) fn optimize_fam(data: &Data, structure: &[UmaIndex; 4], iterations: usize, seed: u64, margin: f64) -> Res {
    if structure[3] == NONE {
        optimize(data, [structure[0], structure[1], structure[2]], iterations, seed, margin)
    } else {
        optimize(data, *structure, iterations, seed, margin)
    }
}

/// Exact pairwise ceilings PB(a,b) = min(BR_a(W = PM_b), BR_b(W = PM_a)) for all pairs in `pool`, PM a uma's
/// best-case win chances. Valid because P_b <= PM_b elementwise and P_a >= 0, so <P_a,P_b> <= <P_a,PM_b> <= BR_a(PM_b).
pub(crate) fn pair_ceilings(data: &Data, pool: &[UmaIndex]) -> UmaTable<UmaTable<f64>> {
    let roster_len = data.umas.len();
    let mut ceilings = UmaTable(vec![UmaTable(vec![0.0f64; roster_len]); roster_len]);
    if let Some(stored) = &data.pair_ceilings {
        for (position, &a) in pool.iter().enumerate() {
            for &b in &pool[position + 1..] {
                ceilings[a][b] = stored[(a, b)];
                ceilings[b][a] = stored[(b, a)];
            }
        }
        return ceilings;
    }
    let best_against_best_case = |a: UmaIndex, b: UmaIndex| {
        data.best_response(&data.umas[a], &data.umas[b].best_case_probs, None).map_or(0.0, |response| response.1)
    };
    for (position, &a) in pool.iter().enumerate() {
        for &b in &pool[position + 1..] {
            let ceiling = best_against_best_case(a, b).min(best_against_best_case(b, a));
            ceilings[a][b] = ceiling;
            ceilings[b][a] = ceiling;
        }
    }
    ceilings
}
