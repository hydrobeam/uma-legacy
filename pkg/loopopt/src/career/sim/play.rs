//! Simulation paths: initial pools, veteran admission and pruning, survival accumulation and run summaries.
//! Policies choose the careers; sibling draw and audit modules supply samples and exact comparisons.

use crate::career::cache::Cache;
use crate::career::evaluator::{Cur, SurrogatePick};
use crate::career::pool::Pool;
use crate::career::sim::audit::{AuditKind, RankedCareer};
use crate::career::sim::draw::{Keys, Outcomes, VeteranDraw};
use crate::career::sim::policies::{INDEX_RUN_COUNTS, IndexRunCounts, Rotation};
use crate::career::sim::{Goal, Sim, SimJob, SimulationOptions, SimulationRoster, family, leg_json};
use crate::career::state::{KeptState, kept_swap};
use crate::career::{Env, JobInputs, Legacy, Lineage};
use serde_json::{Value, json};
use std::rc::Rc;

/// A veteran's stars per job target, her distinct G1 wins and her pink sparks, sorted: what dominance pruning
/// compares.
#[derive(Clone)]
struct VeteranResult {
    stars: Vec<u8>,
    wins: Vec<usize>,
    pinks: Vec<(u32, u8)>,
}
/// One path's state: veterans, the solver's pool, owned veterans per family (dominance), the surrogate's best
#[derive(Clone)]
pub(super) struct PathState {
    pub(super) legacies: Vec<Legacy>,
    pub(super) pool: Pool,
    /// Per legacy, whether she is a full veteran (a spark of every target): a career on two of them finishes under
    /// goal `looped`.
    full: Vec<bool>,
    /// Per family (`family`), the results of the owned veterans in it.
    families: crate::FxMap<String, Vec<VeteranResult>>,
    pub(super) best: Option<SurrogatePick>,
    /// Goal `looped`: the surrogate's best looped career (both parents full veterans), the one greedy runs.
    pub(super) best_looped: Option<SurrogatePick>,
    /// The career the policy runs next, kept until the state changes.
    pub(super) current: Option<Rc<Cur>>,
    pub(super) rotation: Rotation,
    /// `trace`: each loop member's best career (by member index)
    pub(super) member_best: Vec<Option<SurrogatePick>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Base {
    Greedy,
    Loop,
    Trace,
    Index,
}

/// The t distribution's 97.5% quantile at `degrees_of_freedom`: z, the normal one, plus its series terms in 1/df
fn tq975(degrees_of_freedom: f64) -> f64 {
    let z = 1.959_964_f64;
    let (z3, z5, z7) = (z.powi(3), z.powi(5), z.powi(7));
    z + (z3 + z) / (4.0 * degrees_of_freedom)
        + (5.0 * z5 + 16.0 * z3 + 3.0 * z) / (96.0 * degrees_of_freedom * degrees_of_freedom)
        + (3.0 * z7 + 19.0 * z5 + 17.0 * z3 - 15.0 * z) / (384.0 * degrees_of_freedom.powi(3))
}
fn stats(values: &[f64]) -> Value {
    let count = values.len() as f64;
    if values.is_empty() {
        return Value::Null;
    }
    let mean = values.iter().sum::<f64>() / count;
    let standard_deviation = if values.len() > 1 {
        (values.iter().map(|value| (value - mean).powi(2)).sum::<f64>() / (count - 1.0)).sqrt()
    } else {
        0.0
    };
    let half_width = if values.len() > 1 { tq975(count - 1.0) * standard_deviation / count.sqrt() } else { 0.0 };
    json!({"n": values.len(), "mean": mean, "sd": standard_deviation, "ci95": [mean - half_width, mean + half_width], "min": values.iter().copied().fold(f64::INFINITY, f64::min), "max": values.iter().copied().fold(f64::NEG_INFINITY, f64::max)})
}

impl VeteranResult {
    /// At least her stars on every target and every G1 she won, with the same pinks: a veteran whose pinks differ is
    /// kept, since her pinks change her children's aptitudes and so their schedules and wins.
    fn dominates(&self, candidate: &Self) -> bool {
        self.stars.iter().zip(&candidate.stars).all(|(owned, new)| owned >= new)
            && candidate.wins.iter().all(|race| self.wins.binary_search(race).is_ok())
            && self.pinks == candidate.pinks
    }
}

impl PathState {
    /// Whether an owned veteran in this family has at least these stars and every G1 win.
    fn dominates(&self, legacy: &Legacy, candidate: &VeteranResult) -> bool {
        self.families.get(&family(legacy)).is_some_and(|owned| owned.iter().any(|veteran| veteran.dominates(candidate)))
    }

    /// Lowest incumbent value across the overall pick and the trace members, and under goal `looped` the best looped
    /// career once two full veterans can make one.
    fn candidate_floor(&self, goal: Goal) -> f64 {
        let floor = self.member_best.iter().fold(self.best.map_or(0.0, |pick| pick.value), |minimum, pick| {
            minimum.min(pick.map_or(0.0, |pick| pick.value))
        });
        if goal == Goal::Looped && self.full_count() >= 2 { floor.min(self.looped_bar()) } else { floor }
    }

    /// Whether both parents of `lineage` are full veterans: her success is a fully looped parent.
    pub(super) fn is_looped(&self, lineage: Lineage) -> bool {
        self.full[lineage.first_parent] && self.full[lineage.second_parent]
    }

    /// The best looped career's value so far (0 when there is none).
    pub(super) fn looped_bar(&self) -> f64 {
        self.best_looped.map_or(0.0, |pick| pick.value)
    }

    /// How many full veterans the pool holds.
    fn full_count(&self) -> usize {
        self.full.iter().filter(|&&full| full).count()
    }

    /// The legacy indices of the full veterans.
    pub(super) fn full_veterans(&self) -> Vec<usize> {
        (0..self.full.len()).filter(|&legacy| self.full[legacy]).collect()
    }
}

impl Sim<'_, '_> {
    fn veteran_result(&self, legacy: &Legacy) -> VeteranResult {
        let stars = self
            .env
            .setup
            .skill_job
            .targets
            .iter()
            .map(|target| {
                legacy.sparks.iter().filter(|spark| spark.0 == target.group).map(|spark| spark.1).max().unwrap_or(0)
            })
            .collect();
        let mut wins = legacy.wins.clone();
        wins.sort_unstable();
        wins.dedup();
        let mut pinks = legacy.pinks.clone();
        pinks.sort_unstable();
        VeteranResult { stars, wins, pinks }
    }
    /// a veteran joins the pool; `evaluate_candidates`: the surrogate values every new candidate (she is one of its parents)
    pub(super) fn add_veteran(
        &mut self,
        state: &mut PathState,
        legacy: Legacy,
        evaluate_candidates: bool,
        interpolation_scratch: &mut Vec<f64>,
    ) {
        // (her entry as `CareerSetup::pool` builds it, win chances and star mix kept)
        self.env.setup.pool_push(&mut state.pool, &legacy);
        let result = self.veteran_result(&legacy);
        let full = result.stars.iter().all(|&stars| stars > 0);
        if !legacy.borrowed {
            state.families.entry(family(&legacy)).or_default().push(result);
        }
        state.legacies.push(legacy);
        state.full.push(full);
        if !evaluate_candidates {
            return;
        }
        let veteran_index = state.legacies.len() - 1;
        // (goal `looped`: a full veteran's careers with another full veteran are looped, valued against the best looped
        // career's bar)
        let looped_bar =
            (self.settings.goal == Goal::Looped && full && state.full_count() >= 2).then(|| state.looped_bar());
        let trainees = self.trainees.clone();
        for &trainee in &trainees {
            let trainee_bar = looped_bar.map_or(self.bar(state, trainee), |bar| bar.min(self.bar(state, trainee)));
            if self.static_ceilings[trainee] * (1.0 + 1e-9) < trainee_bar {
                continue;
            }
            for other_parent in 0..veteran_index {
                let lineage = Lineage { trainee, first_parent: other_parent, second_parent: veteran_index };
                if !self.legal(state, lineage) {
                    continue;
                }
                // the allocation-free ceiling first (`fast_ub`, the same bound to rounding)
                if self.evaluator.fast_ub(&self.env.setup, &state.pool, lineage) * (1.0 + 1e-6)
                    < self.lineage_bar(state, lineage)
                {
                    self.evaluator.counters.candidates += 1;
                    continue;
                }
                self.consider(state, lineage, interpolation_scratch);
            }
        }
    }
    /// the initial state: the job's pool, every candidate valued (best ceiling first)
    fn initial(&mut self, legacies: &[Legacy], base: Base) -> PathState {
        let mut state = PathState {
            legacies: vec![],
            pool: self.env.setup.pool(&[]),
            full: vec![],
            families: std::collections::HashMap::default(),
            best: None,
            best_looped: None,
            current: None,
            rotation: Rotation { position: 0, latest: vec![None; self.members.len()] },
            member_best: vec![None; self.members.len()],
        };
        let mut interpolation_scratch = vec![];
        for legacy in legacies {
            self.add_veteran(&mut state, legacy.clone(), false, &mut interpolation_scratch);
        }
        if !matches!(base, Base::Loop | Base::Index) {
            self.seed_surrogate(&mut state, &mut interpolation_scratch);
        }
        state
    }
    fn seed_surrogate(&mut self, state: &mut PathState, interpolation_scratch: &mut Vec<f64>) {
        // seed the cache with the career search's fully checked decks
        self.env.state.use_seen = false;
        let trainees = self.trainees.clone();
        // (on the path's pool, `CareerSetup::pool(legacies)`: `add_veteran` builds each entry as it does)
        let search = self.env.solve(crate::career::search::SearchRequest {
            pool: &state.pool,
            fixed_parents: &[],
            trainees: &trainees,
            per_trainee: false,
            top: 1,
        });
        self.env.state.use_seen = true;
        self.evaluator.cache.absorb(&self.env.setup, &mut self.env.state, &mut self.env.memo, &search.refs);
        // (every legal career's ceiling, `gen_cands`: `ceiling` at `lineage`'s chances, to the bit; in (trainee,
        // pair) order, then best first)
        let mut candidates =
            self.env.setup.gen_cands(&mut self.env.state, &state.pool, &[], &trainees, |_, _| true, RankedCareer::new);
        candidates.sort_by(|first, second| second.ceiling.partial_cmp(&first.ceiling).unwrap());
        for candidate in candidates {
            let floor = state.candidate_floor(self.settings.goal);
            if candidate.ceiling * (1.0 + 1e-9) < floor {
                break;
            }
            if candidate.ceiling * (1.0 + 1e-9) < self.lineage_bar(state, candidate.lineage) {
                continue;
            }
            self.consider(state, candidate.lineage, interpolation_scratch);
        }
    }
    fn policy_career(
        &mut self,
        state: &mut PathState,
        base: Base,
        exact: bool,
        kept_decision: &mut Option<Rc<Cur>>,
    ) -> Rc<Cur> {
        if exact {
            if let Some(career) = kept_decision.as_ref() {
                Rc::clone(career)
            } else {
                let career = Rc::new(self.exact_cur(state));
                *kept_decision = Some(Rc::clone(&career));
                career
            }
        } else {
            match base {
                Base::Greedy => self.greedy_cur(state),
                Base::Loop => self.loop_cur(state),
                Base::Trace => unreachable!("trace runs its own loop"),
                Base::Index => {
                    if let Some(career) = kept_decision.as_ref() {
                        Rc::clone(career)
                    } else {
                        let career = Rc::new(self.index_cur(state));
                        *kept_decision = Some(Rc::clone(&career));
                        career
                    }
                }
            }
        }
    }
    /// Whether `career`'s success ends the run: every career under goal `first`, a looped one under `looped`.
    fn finishes(&self, state: &PathState, career: &Cur) -> bool {
        match self.settings.goal {
            Goal::First => true,
            Goal::Looped => state.is_looped(career.lineage),
        }
    }
    /// Follow `options.base` from the initial state (survival 1, career 0) until the survival S < `survival_cutoff` or
    /// `max_careers`, recording pick changes, checks and audits into `trace`.
    fn play(&mut self, state: &mut PathState, options: PlayOptions, trace: &mut PathTrace) -> PathResult {
        let PlayOptions { base, keys, survival_cutoff, run } = options;
        let (mut survival, mut career_index) = (1.0f64, 0usize);
        let mut expected_careers = 0.0f64;
        let mut pruned = 0usize;
        // (goal `looped`: the finishing careers run so far, and the last one's P(all), for the tail)
        let (mut finishing_careers, mut last_finishing_p_all) = (0usize, None::<f64>);
        let mut interpolation_scratch = vec![];
        let mut last_key = String::new();
        let exact = self.settings.policy == "greedy_exact";
        // Exact and index policies keep their decision until the pool changes.
        let mut kept_decision: Option<Rc<Cur>> = None;
        let mut since_check = 0usize;
        if self.settings.check > 0 && base == Base::Greedy {
            trace.checks.push(self.check(state, true));
        }
        loop {
            let career = match self.settings.force {
                Some(forced) if career_index == 0 => Rc::new(self.forced_cur(state, forced)),
                _ => self.policy_career(state, base, exact, &mut kept_decision),
            };
            let pick_key = self.key(&career);
            if pick_key != last_key {
                trace.pick_changes.push(json!({"t": career_index, "pick": pick_key, "trainee": self.data.umas[career.lineage.trainee].name, "p": career.p_all, "pool": state.legacies.len(), "s": survival}));
                last_key = pick_key;
                if self.settings.audit && base == Base::Greedy && !exact {
                    let audit = self.audit_greedy(state, keys, career_index);
                    self.audits.push(audit);
                }
            }
            if self.settings.audit && base == Base::Loop && career_index.is_multiple_of(self.settings.audit_every) {
                let audit_row = self.audit_row(&state.pool, career.lineage, AuditKind::Loop);
                self.audits.push(json!({"t": career_index, "pool": state.legacies.len(), "rows": [audit_row]}));
            }
            if self.settings.progress > 0 && career_index.is_multiple_of(self.settings.progress) && career_index > 0 {
                eprintln!(
                    "  {} run {}: {} careers, pool {}, S {:.4}, p {:.3e}",
                    self.settings.policy,
                    run,
                    career_index,
                    state.legacies.len(),
                    survival,
                    career.p_all
                );
            }
            let finishes = self.finishes(state, &career);
            if survival < survival_cutoff || career_index >= self.settings.max_careers {
                // (the tail repeats this career forever, or under goal `looped` the last finishing one: module notes)
                let tail_p_all = if finishes { Some(career.p_all) } else { last_finishing_p_all };
                let tail = tail_p_all.map_or(0.0, |p_all| survival / p_all);
                return PathResult {
                    expected_careers: expected_careers + tail,
                    careers: career_index,
                    tail,
                    final_p_all: career.p_all,
                    pruned,
                    survival,
                    looped: LoopedRun { finishing_careers, censored: tail_p_all.is_none() },
                };
            }
            expected_careers += survival;
            let outcomes = if finishes {
                survival *= 1.0 - career.p_all;
                finishing_careers += 1;
                last_finishing_p_all = Some(career.p_all);
                Outcomes::Failed
            } else {
                Outcomes::Any
            };
            let veteran = self.draw(&career, state, VeteranDraw { keys, career_index, outcomes });
            career_index += 1;
            match base {
                Base::Loop => {
                    self.add_veteran(state, veteran, false, &mut interpolation_scratch);
                    if career.rotation {
                        self.rot_update(state);
                    }
                    state.current = None;
                    kept_decision = None;
                }
                Base::Trace => unreachable!("trace runs its own loop"),
                // (index: a pruned veteran leaves the pool as it was, so the plan, and its pick, are kept)
                Base::Greedy | Base::Index => {
                    if self.settings.prune && state.dominates(&veteran, &self.veteran_result(&veteran)) {
                        pruned += 1;
                        continue;
                    }
                    self.add_veteran(state, veteran, !exact && base == Base::Greedy, &mut interpolation_scratch);
                    kept_decision = None;
                    since_check += 1;
                    if base == Base::Greedy && self.settings.check > 0 && since_check >= self.settings.check {
                        since_check = 0;
                        trace.checks.push(self.check(state, false));
                    }
                }
            }
        }
    }
}

/// How a run's path is followed (`Sim::play`).
#[derive(Clone, Copy)]
struct PlayOptions {
    base: Base,
    keys: Keys,
    /// Stop once the survival S falls below this.
    survival_cutoff: f64,
    /// The run index (progress lines).
    run: u32,
}

/// What a path records as it goes (`Sim::play`).
#[derive(Default)]
struct PathTrace {
    /// Each change of the career picked: career index, pick, trainee, P(all), pool size, survival.
    pick_changes: Vec<Value>,
    /// The surrogate's checks against exact searches (`Sim::check`).
    checks: Vec<Value>,
}

/// One run's result (`Sim::play`).
struct PathResult {
    /// E: the sum of the survivals S_t, the tail S / p included (module notes).
    expected_careers: f64,
    /// Careers run before the path stopped.
    careers: usize,
    /// The tail S / p added at the stop.
    tail: f64,
    /// P(all) of the career the path stopped at.
    final_p_all: f64,
    /// New veterans dropped as dominated.
    pruned: usize,
    /// The survival S at the stop (before the tail).
    survival: f64,
    /// What goal `looped` reports beside.
    looped: LoopedRun,
}

/// One run's goal-`looped` record (`Sim::play`).
#[derive(Clone, Copy)]
struct LoopedRun {
    /// Careers run on two full-veteran parents (each one's success would have ended the run).
    finishing_careers: usize,
    /// The run ran no finishing career, so it has no tail: its E is a lower bound.
    censored: bool,
}

pub(in crate::career) fn simulate<'data>(
    env: &mut Env<'data>,
    kept: &mut KeptState,
    input: &JobInputs<'data>,
    legacies: &[Legacy],
    trainees: &[crate::uma::UmaIndex],
    configuration: &Value,
) -> Value {
    let config: SimJob = match serde_json::from_value(configuration.clone()) {
        Ok(config) => config,
        Err(error) => return json!({"error": format!("simulate: {error}")}),
    };
    let target_count = env.setup.target_count;
    if target_count > 12 || env.setup.uncertain_count > 10 {
        return json!({"error": "simulate takes at most 12 targets, 10 of them not guaranteed"});
    }
    let base = match config.policy.as_str() {
        "greedy" | "greedy_exact" => Base::Greedy,
        "loop" => Base::Loop,
        "trace" => Base::Trace,
        "index" => Base::Index,
        policy => return json!({"error": format!("simulate: unknown policy {policy}")}),
    };
    if config.force.is_some() && matches!(base, Base::Loop | Base::Trace) {
        return json!({"error": "simulate: force is for greedy, greedy_exact and index (a loop or trace runs its own careers)"});
    }
    if config.goal == Goal::Looped && base == Base::Trace {
        return json!({"error": "simulate: trace has no goal (goal looped is for the path policies)"});
    }
    if config.goal == Goal::Looped && (config.audit || config.check > 0) && base == Base::Greedy {
        return json!({"error": "simulate: audit and check measure greedy's unrestricted pick: run them with goal first"});
    }
    let members: Vec<crate::uma::UmaIndex> =
        config.loop_members.iter().filter_map(|&id| env.setup.uma_index(id)).collect();
    if matches!(base, Base::Loop | Base::Trace)
        && (members.len() < 3
            || members.len() != config.loop_members.len()
            || members.iter().any(|member| !env.setup.owned.contains(member)))
    {
        return json!({"error": "loop: loop_members must be at least three owned umas"});
    }
    let uma_count = input.data.umas.len();
    let mut member_by_trainee = crate::uma::UmaTable(vec![None; if base == Base::Trace { uma_count } else { 0 }]);
    if base == Base::Trace {
        for (member_index, &member) in members.iter().enumerate() {
            member_by_trainee[member] = Some(member_index);
        }
    }
    let mut simulator = Sim::new(
        env,
        kept,
        input,
        config.clone(),
        SimulationRoster { trainees: trainees.to_vec(), members, member_by_trainee },
        SimulationOptions { record_audits: true, fill_missing_decks: false },
    );
    let initialization_started = std::time::Instant::now();
    let initial_state = simulator.initial(legacies, base);
    if base == Base::Trace {
        return simulator.trace(&initial_state);
    }
    let initialization_seconds = initialization_started.elapsed().as_secs_f64();
    let mut runs = vec![];
    let mut run_expectations = vec![];
    let mut censored_runs = 0usize;
    let survival_cutoff = config.survival_cutoff;
    // index: the kept state before the simulation, put back after it (module notes)
    let kept_before_simulation = (base == Base::Index).then(|| kept_swap(simulator.kept, KeptState::default()));
    for run in config.first..config.first + config.runs {
        if !config.share_cache && run > config.first {
            simulator.evaluator.cache = Cache::new(uma_count);
        }
        if base == Base::Index {
            kept_swap(simulator.kept, KeptState::default());
            INDEX_RUN_COUNTS.with(|index_counts| *index_counts.borrow_mut() = IndexRunCounts::default());
        }
        let started = std::time::Instant::now();
        let counts_before = simulator.evaluator.counters;
        let mut state = initial_state.clone();
        let mut trace = PathTrace::default();
        let keys = Keys::Main { seed: config.seed, run: run as u32 };
        let PathResult { expected_careers, careers, tail, final_p_all, pruned, survival, looped } =
            simulator.play(&mut state, PlayOptions { base, keys, survival_cutoff, run: run as u32 }, &mut trace);
        let counts_after = simulator.evaluator.counters;
        run_expectations.push(expected_careers);
        let PathTrace { pick_changes, checks } = trace;
        let final_pick = pick_changes.last().map_or(Value::Null, |change| change["pick"].clone());
        let wall = started.elapsed().as_secs_f64();
        eprintln!(
            "{} run {}: E {:.1}, {} careers, tail {:.2}, final p {:.3e}, pool {}, {:.1} s",
            config.policy,
            run,
            expected_careers,
            careers,
            tail,
            final_p_all,
            state.legacies.len(),
            wall
        );
        runs.push(json!({"run": run, "E": expected_careers, "careers": careers, "tail": tail, "final_p": final_p_all, "final": final_pick, "pool": state.legacies.len(), "pruned": pruned,
            "wall_s": wall, "candidates": counts_after.candidates - counts_before.candidates, "ceiling_pass": counts_after.ceiling_passes - counts_before.ceiling_passes, "tables": counts_after.tables - counts_before.tables,
            "refreshes": counts_after.searches - counts_before.searches, "partial_searches": counts_after.partial_searches - counts_before.partial_searches, "segs": pick_changes, "checks": checks, "audits": std::mem::take(&mut simulator.audits)}));
        if base == Base::Index {
            let index_counts = INDEX_RUN_COUNTS.with(|index_counts| std::mem::take(&mut *index_counts.borrow_mut()));
            runs.last_mut().unwrap()["index"] = json!({"plans": index_counts.plans, "plan_s": index_counts.plan_seconds, "roles": index_counts.roles, "best_odds": index_counts.best_odds_picks});
        }
        if config.goal == Goal::Looped {
            censored_runs += usize::from(looped.censored);
            let run_record = runs.last_mut().unwrap();
            run_record["finishing"] = json!(looped.finishing_careers);
            run_record["full_veterans"] = json!(state.full_count());
            run_record["censored"] = json!(looped.censored);
        }
        if config.keep_pool {
            let run_record = runs.last_mut().unwrap();
            run_record["survival"] = json!(survival);
            run_record["final_legacies"] = json!(state.legacies.iter().map(leg_json).collect::<Vec<_>>());
        }
    }
    if let Some(kept) = kept_before_simulation {
        kept_swap(simulator.kept, kept);
    }
    let mut output = json!({"simulate": configuration, "policy": config.policy, "runs": runs, "E": stats(&run_expectations), "init_s": initialization_seconds, "decks": simulator.evaluator.cache.deck_count(), "deck_tables": simulator.evaluator.cache.tables,
           "refreshes": simulator.evaluator.counters.searches, "schedules": simulator.evaluator.win_chances.schedules()});
    if config.goal == Goal::Looped {
        // (E over the censored runs is a lower bound: their tails are 0)
        output["goal"] = json!("looped");
        output["censored"] = json!(censored_runs);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::VeteranResult;

    #[test]
    /// Dominance pruning keeps a veteran whose pinks differ from an owned one with the same stars and wins (the pinks
    /// change her children's aptitudes), and drops one whose pinks are the same.
    fn sim_pruning_keeps_different_pinks() {
        let veteran = |pinks: Vec<(u32, u8)>| VeteranResult { stars: vec![2, 1, 3], wins: vec![4, 9], pinks };
        let owned = veteran(vec![(32, 2)]);
        assert!(owned.dominates(&veteran(vec![(32, 2)])));
        assert!(!owned.dominates(&veteran(vec![(11, 2)])));
        assert!(!owned.dominates(&veteran(vec![(32, 3)])));
        assert!(!owned.dominates(&veteran(vec![])));
        assert!(!veteran(vec![]).dominates(&veteran(vec![(32, 1)])));
    }
}
