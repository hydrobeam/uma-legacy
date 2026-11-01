//! Career selection for greedy, loop, trace, exact and page-index policies, with rotation and index-run records.
//! Draws, path execution and exact audit comparisons live in the sibling modules.

use crate::career::evaluator::{Cur, DeckRequest, SurrogatePick};
use crate::career::plan;
use crate::career::plan::ScoringContext;
use crate::career::plan::report::{CheckReply, Plan, Recommendation};
use crate::career::pool::Pool;
use crate::career::reply::CareerReport;
use crate::career::sim::draw::{Keys, Outcomes, VeteranDraw};
use crate::career::sim::play::PathState;
use crate::career::sim::report::{ExecutedCareer, RotationAudit, SurrogateAudit, TraceReport, TraceRun};
use crate::career::sim::{ForcedCareer, Goal, IndexAction, PAGE_TOP, Sim};
use crate::career::{Career, Lineage, run_career_in};
use std::rc::Rc;

/// Loop rotation: the next member's position and each member's latest rotation veteran
#[derive(Clone, Default)]
pub(super) struct Rotation {
    /// Careers run so far; the next member is this modulo the member count.
    pub(super) position: usize,
    /// Per member, her rotation veteran's legacy index.
    pub(super) latest: Vec<Option<usize>>,
}

/// `trace`: a builder's best career and what it does for the trainee built for (A, best odds p).
#[derive(Clone, Copy)]
struct BuilderCareer {
    /// lam (1/p - 1/p1): the expected fall in A's careers needed, per career run.
    efficiency: f64,
    pick: SurrogatePick,
    /// lam: the chance her veteran raises A's best odds.
    lift_chance: f64,
    /// p1: A's mean odds when raised (p when never).
    lifted_odds: f64,
}

/// `index`, this run: what its plan jobs did.
#[derive(Default)]
pub(super) struct IndexRunCounts {
    /// Plan jobs run.
    pub(super) plans: usize,
    /// Each plan job's seconds, rounded to the millisecond.
    pub(super) plan_seconds: Vec<f64>,
    /// Per role of the chosen option (`next.options[].role`), how many picks had it.
    pub(super) roles: std::collections::BTreeMap<String, usize>,
    /// Picks that were the best-odds career (`best_odds`).
    pub(super) best_odds_picks: usize,
    /// `act_on` "pair_search" (`IndexAction::PairSearch`), per plan: what it ran, by the recommendation's `source`
    /// ("plan_pick", "pick_trainee_pair", "cross_trainee"; "no_check" where the plan kept no check, which the page
    /// then does not show), the career run (`kept::career_id`), and the check's seconds (`robust.check_ms`, inside the
    /// plan job's); the `career` jobs run for a recommendation other than the plan's pick, each one's seconds; and per
    /// plan the career that ran, as the report it was built from names it (`career`: the plan's `choice`, or the
    /// `career` job's result for a named career) with the landed-set chances it ran with (`outcome_sets`, `p_all`)
    pub(super) acted_on: std::collections::BTreeMap<String, usize>,
    pub(super) careers: Vec<String>,
    pub(super) check_seconds: Vec<f64>,
    pub(super) career_job_seconds: Vec<f64>,
    pub(super) executed: Vec<ExecutedCareer>,
}
impl IndexRunCounts {
    fn record_plan(&mut self, plan: Option<&Plan>, started: std::time::Instant) {
        self.plans += 1;
        self.plan_seconds.push((started.elapsed().as_secs_f64() * 1e3).round() / 1e3);
        if let Some(chosen) = plan.and_then(|plan| plan.options.iter().find(|option| option.chosen)) {
            *self.roles.entry(chosen.role.name().to_string()).or_default() += 1;
            if chosen.best_odds {
                self.best_odds_picks += 1;
            }
        }
    }
    /// `act_on` "pair_search": a plan's action (`IndexRunCounts::acted_on`).
    fn record_action(&mut self, source: &str, career: Option<&str>, check_ms: f64) {
        *self.acted_on.entry(source.to_string()).or_default() += 1;
        self.careers.push(career.unwrap_or("?").to_string());
        self.check_seconds.push(check_ms.round() / 1e3);
    }
    /// `act_on` "pair_search": the career a plan's action ran (`IndexRunCounts::executed`), `ran_from` the report its
    /// `Cur` was built from.
    fn record_executed(&mut self, ran_from: &CareerReport, career: &Cur) {
        self.executed.push(ExecutedCareer {
            career: ran_from.career.clone(),
            p_all: career.p_all,
            outcome_sets: career.outcome_sets.clone(),
        });
    }
}
thread_local! { pub(super) static INDEX_RUN_COUNTS: std::cell::RefCell<IndexRunCounts> = std::cell::RefCell::new(IndexRunCounts::default()); }

impl Rotation {
    /// The next two members' careers that this veteran is a parent of (the other parent is their current
    /// rotation veteran; a term without one, or with her own character, is left out), in that order
    fn careers(
        &self,
        pool: &Pool,
        members: &[crate::uma::UmaIndex],
        member_index: usize,
        veteran_index: usize,
    ) -> Vec<Lineage> {
        let member_count = members.len();
        let mut careers = vec![];
        for offset in 1..=2 {
            let trainee = members[(member_index + offset) % member_count];
            let other = if offset == 1 {
                self.latest[(member_index + member_count - 1) % member_count]
            } else {
                self.latest[(member_index + 1) % member_count]
            };
            let Some(other_parent) = other else { continue };
            if other_parent == veteran_index
                || pool.veterans[other_parent].character_id == pool.veterans[veteran_index].character_id
            {
                continue;
            }
            careers.push(Lineage {
                trainee,
                first_parent: veteran_index.min(other_parent),
                second_parent: veteran_index.max(other_parent),
            });
        }
        careers
    }
}

impl Sim<'_, '_> {
    pub(super) fn greedy_cur(&mut self, state: &mut PathState) -> Rc<Cur> {
        if let Some(career) = &state.current {
            return Rc::clone(career);
        }
        let restricted = match self.settings.goal {
            Goal::First => None,
            Goal::Looped => state.best_looped,
        };
        let pick = restricted.or(state.best).expect("no legal career");
        let career = Rc::new(self.evaluator.cached_cur(&self.env.setup, &mut self.env.state, &state.pool, pick, false));
        state.current = Some(Rc::clone(&career));
        career
    }
    /// `trace`: building for loop_members[0] (A) with the others as builders. Each career: per builder her best career's
    /// exact chance of raising A's best odds (lam) and mean odds raised to (p1); the most efficient (lam (1/p - 1/p1)) runs,
    /// her veteran (drawn from every landed set) joins; recorded per career: A's odds before, the builder, lam, p1, and A's odds after
    pub(super) fn trace(&mut self, initial_state: &PathState) -> TraceReport {
        let finisher = self.members[0];
        let mut runs = vec![];
        for run in self.settings.first..self.settings.first + self.settings.runs {
            let mut state = initial_state.clone();
            let keys = Keys::Main { seed: self.settings.seed, run: run as u32 };
            let mut interpolation_scratch = vec![];
            let mut rows = vec![];
            let started = std::time::Instant::now();
            for career_index in 0..self.settings.max_careers {
                let Some(finisher_pick) = state.member_best[0] else { break };
                let finisher_odds = finisher_pick.value;
                let Some(builder) = self.trace_builder(&state, finisher_odds, &mut interpolation_scratch) else {
                    break;
                };
                let career =
                    self.evaluator.cached_cur(&self.env.setup, &mut self.env.state, &state.pool, builder.pick, false);
                // (every landed set: a builder's full veteran does not end building for A)
                let veteran = self.draw(&career, &state, VeteranDraw { keys, career_index, outcomes: Outcomes::Any });
                self.add_veteran(&mut state, veteran, true, &mut interpolation_scratch);
                let after = state.member_best[0].map_or(finisher_odds, |pick| pick.value);
                rows.push((
                    career_index,
                    finisher_odds,
                    self.data.umas[builder.pick.lineage.trainee].name.clone(),
                    builder.lift_chance,
                    builder.lifted_odds,
                    after,
                ));
            }
            eprintln!(
                "trace run {}: {} careers, A from {:.3e} to {:.3e}, {:.1} s",
                run,
                rows.len(),
                initial_state.member_best[0].map_or(0.0, |pick| pick.value),
                state.member_best[0].map_or(0.0, |pick| pick.value),
                started.elapsed().as_secs_f64()
            );
            runs.push(TraceRun { run, rows });
        }
        TraceReport {
            trace: runs,
            trainee: self.data.umas[finisher].name.clone(),
            builders: self.members[1..].iter().map(|&member| self.data.umas[member].name.clone()).collect(),
        }
    }
    fn trace_builder(
        &mut self,
        state: &PathState,
        finisher_odds: f64,
        interpolation_scratch: &mut Vec<f64>,
    ) -> Option<BuilderCareer> {
        // this career's scoring of the builders: A alone, at her best odds now, on this career's pool
        let finishers = [self.members[0]];
        let current_odds = [finisher_odds];
        let base = plan::cont::ScoreBase {
            candidates: &finishers,
            step_costs: &[1.0],
            current_value: 0.0,
            current_odds: &current_odds,
            continuations: &[],
            step_caps: &[],
            trios: &[],
            trio_sides: &[],
            finisher_umas: &[],
            knobs: plan::cont::ScoringKnobs::default(),
            odds_scales: crate::career::plan::cont::OddsScales::ONE,
        };
        let scoring = ScoringContext { score_base: &base, legacies: &state.legacies, pool: &state.pool };
        let mut best_builder: Option<BuilderCareer> = None;
        for member_index in 1..self.members.len() {
            let Some(pick) = state.member_best[member_index] else { continue };
            // (cloned: the scoring borrows the whole simulator and may add decks to her cache)
            let deck = self.evaluator.cache.decks[pick.lineage.trainee][pick.deck].deck.clone();
            let plan::score::CareerScore { lift_stats, .. } = plan::score::score_career_of(
                self,
                &scoring,
                Career { lineage: pick.lineage, deck: &deck },
                plan::cont::Want::Stats,
                interpolation_scratch,
            );
            let plan::score::kernel::LiftStats { lift_chance, lifted_odds_sum } = lift_stats[0];
            let (efficiency, lifted_odds) = if lift_chance > 0.0 {
                let lifted_odds = lifted_odds_sum / lift_chance;
                (lift_chance * (1.0 / finisher_odds - 1.0 / lifted_odds), lifted_odds)
            } else {
                (0.0, finisher_odds)
            };
            if best_builder.is_none_or(|builder| efficiency > builder.efficiency) {
                best_builder = Some(BuilderCareer { efficiency, pick, lift_chance, lifted_odds });
            }
        }
        best_builder
    }
    /// the loop's next career (module notes: rotation and bootstrap)
    pub(super) fn loop_cur(&mut self, state: &mut PathState) -> Rc<Cur> {
        if let Some(career) = &state.current {
            return Rc::clone(career);
        }
        let member_count = self.members.len();
        let position = state.rotation.position % member_count;
        let trainee = self.members[position];
        let parents: Vec<usize> = [1, 2]
            .iter()
            .filter_map(|&k| state.rotation.latest[(position + member_count - k) % member_count])
            .collect();
        let career = if parents.len() == 2 {
            let lineage = Lineage {
                trainee,
                first_parent: parents[0].min(parents[1]),
                second_parent: parents[0].max(parents[1]),
            };
            let (inheritance_chances, spark_scale) = self.env.setup.lineage_q(&state.pool, lineage);
            let ceiling = self
                .env
                .setup
                .ceiling(&mut self.env.state, trainee, &inheritance_chances, spark_scale)
                .unwrap_or(f64::INFINITY);
            let mut interpolation_scratch = vec![];
            let (value, deck) = self
                .evaluator
                .deck_for(
                    self.env,
                    DeckRequest {
                        trainee,
                        inheritance_chances: &inheritance_chances,
                        spark_scale,
                        ceiling,
                        threshold: 0.0,
                    },
                    &mut interpolation_scratch,
                )
                .expect("loop career without a deck");
            self.evaluator.cached_cur(
                &self.env.setup,
                &mut self.env.state,
                &state.pool,
                SurrogatePick { value, lineage, deck },
                true,
            )
        } else {
            self.env.state.use_seen = true;
            let search = self.env.solve(crate::career::search::SearchRequest {
                pool: &state.pool,
                fixed_parents: &parents,
                trainees: &[trainee],
                per_trainee: false,
                top: 1,
            });
            let found = search.found.first().expect("loop bootstrap: no legal career for a member");
            self.evaluator.cache.absorb(&self.env.setup, &mut self.env.state, &mut self.env.memo, &search.refs);
            self.evaluator.make_cur(
                &self.env.setup,
                &mut self.env.state,
                &state.pool,
                Career { lineage: found.lineage(), deck: &found.deck },
                true,
            )
        };
        let career = Rc::new(career);
        state.current = Some(Rc::clone(&career));
        career
    }
    /// The loop's rotation after a member's career (her veteran the pool's last): the member keeps the better of her
    /// rotation veteran and the new one, by the sum of the surrogate P(all) of the two loop careers she is a parent of
    /// (`Rotation::careers`, for both); the first one she has is kept as is. Then the next member trains.
    pub(super) fn rot_update(&mut self, state: &mut PathState) {
        let member_count = self.members.len();
        let member_index = state.rotation.position % member_count;
        let veteran_index = state.legacies.len() - 1;
        state.rotation.position += 1;
        state.current = None;
        let Some(previous_veteran) = state.rotation.latest[member_index] else {
            state.rotation.latest[member_index] = Some(veteran_index);
            return;
        };
        let mut interpolation_scratch = vec![];
        let mut surrogate = |simulator: &mut Self, parent_index: usize| -> f64 {
            let mut total = 0.0;
            for lineage in state.rotation.careers(&state.pool, &simulator.members, member_index, parent_index) {
                let trainee = lineage.trainee;
                let (inheritance_chances, spark_scale) = simulator.env.setup.lineage_q(&state.pool, lineage);
                let ceiling = simulator
                    .env
                    .setup
                    .ceiling(&mut simulator.env.state, trainee, &inheritance_chances, spark_scale)
                    .unwrap_or(f64::INFINITY);
                total += simulator
                    .evaluator
                    .deck_for(
                        simulator.env,
                        DeckRequest {
                            trainee,
                            inheritance_chances: &inheritance_chances,
                            spark_scale,
                            ceiling,
                            threshold: 0.0,
                        },
                        &mut interpolation_scratch,
                    )
                    .map_or(0.0, |(value, _)| value);
            }
            total
        };
        let (previous_surrogate, new_surrogate) = (surrogate(self, previous_veteran), surrogate(self, veteran_index));
        if self.settings.audit
            && self.record_audits
            && state.rotation.position.is_multiple_of(self.settings.audit_every)
        {
            // the keep-or-replace decision under exact values (a full deck search per term)
            let exact = |simulator: &mut Self, parent_index: usize| -> f64 {
                let mut total = 0.0;
                for lineage in state.rotation.careers(&state.pool, &simulator.members, member_index, parent_index) {
                    let (inheritance_chances, spark_scale) = simulator.env.setup.lineage_q(&state.pool, lineage);
                    total += simulator.exact_best(lineage.trainee, &inheritance_chances, spark_scale).value;
                }
                total
            };
            let (previous_exact, new_exact) = (exact(self, previous_veteran), exact(self, veteran_index));
            self.audits.push(SurrogateAudit::Rotation(RotationAudit {
                t: state.rotation.position,
                rotation: (
                    previous_surrogate,
                    new_surrogate,
                    previous_exact,
                    new_exact,
                    (new_surrogate > previous_surrogate) == (new_exact > previous_exact),
                ),
            }));
        }
        if new_surrogate > previous_surrogate {
            state.rotation.latest[member_index] = Some(veteran_index);
        }
    }
    /// Builds a runnable career from a selected internal solver result: its trainee, its parents (`parents`: the
    /// report's, or as the caller maps them into the pool) and its exact probabilities (a report with its schedule,
    /// so its `outcome_sets`); resolving the win chances also fills the schedule cache.
    fn cur_from_solver_pick(&mut self, pool: &Pool, pick: &CareerReport, parents: [usize; 2]) -> Cur {
        let trainee = self.env.setup.uma_index(pick.trainee.id).unwrap();
        let outcome_sets = pick.outcome_sets.clone().expect("a pick's report has its landed sets");
        let lineage = Lineage { trainee, first_parent: parents[0], second_parent: parents[1] };
        // (the outfit the pick's schedule ran: the report's `schedule_outfit`)
        let outfit = crate::career::schedule::ScheduleOutfits::Card(pick.trainee.schedule_outfit);
        let record = self.evaluator.wins(&self.env.setup, pool, lineage, outfit);
        Cur { lineage, p_all: pick.p_all, outcome_sets, record, rotation: false }
    }
    /// the in-process career search's pick (greedy_exact: the best career by P(all); under goal `looped` the best
    /// among the full veterans' careers when they make one)
    pub(super) fn exact_cur(&mut self, state: &PathState) -> Cur {
        if self.settings.goal == Goal::Looped
            && let Some(career) = self.exact_cur_among(state, &state.full_veterans())
        {
            return career;
        }
        let every_legacy: Vec<usize> = (0..state.legacies.len()).collect();
        self.exact_cur_among(state, &every_legacy).expect("solver gave no pick")
    }
    /// The career search's best career with both parents among `legacies` (indices into the path's legacies), in
    /// process; None when they make no legal career.
    fn exact_cur_among(&mut self, state: &PathState, legacies: &[usize]) -> Option<Cur> {
        if legacies.len() < 2 {
            return None;
        }
        let job = self.own_job(legacies.iter().map(|&legacy| &state.legacies[legacy]), |_| {});
        let report = run_career_in(self.data, self.skills, self.cards, &job, self.kept);
        let pick = report.search()?.results.first()?;
        // (the search's parents index `legacies`; the path's pool indexes every legacy)
        let parents = pick.parents.map(|parent| legacies[parent]);
        Some(self.cur_from_solver_pick(&state.pool, pick, parents))
    }
    /// `force`: the forced first career, the trainee on the two parents with her best deck (the career search with
    /// both parents fixed, in process)
    pub(super) fn forced_cur(&mut self, state: &PathState, forced: ForcedCareer) -> Cur {
        let job = self.own_job(state.legacies.iter(), |career| {
            career.trainees = vec![forced.trainee];
            career.fixed_parents = forced.parents.to_vec();
        });
        let report = run_career_in(self.data, self.skills, self.cards, &job, self.kept);
        let pick = report.search().and_then(|search| search.results.first());
        let Some(pick) = pick.filter(|pick| pick.trainee.id == forced.trainee) else {
            panic!("force: no career of {} on {:?}: {}", forced.trainee, forced.parents, report.excerpt(300));
        };
        self.cur_from_solver_pick(&state.pool, pick, pick.parents)
    }
    /// `index`: the page's plan for this pool (the career job with the pool as legacies, no places, the page's `top`,
    /// `next`, `no_reports`: the plan job the page sends, docs/SOLVER.md "Next career"), in process; its `choice` runs.
    /// The job's `trainees` stay (the page sends none: every owned uma)
    pub(super) fn index_cur(&mut self, state: &PathState) -> Cur {
        let job = self.own_job(state.legacies.iter(), |career| {
            career.top = PAGE_TOP;
            let mut next = self.settings.next.clone();
            if self.settings.act_on == IndexAction::PairSearch {
                // (the page's check, every variant then the pair search, answered in the plan's own job: `check_kept`
                // runs it on the plan's kept state after the plan)
                next.ask_check_and_pair_search();
            }
            career.next = Some(next);
            career.no_reports = true;
        });
        let started = std::time::Instant::now();
        let report = run_career_in(self.data, self.skills, self.cards, &job, self.kept);
        let plan = report.plan();
        let Some(pick) = plan.and_then(|plan| plan.choice.as_ref()) else {
            panic!("index: the plan gave no pick: {}", report.excerpt(300));
        };
        INDEX_RUN_COUNTS.with(|counts| counts.borrow_mut().record_plan(plan, started));
        match self.settings.act_on {
            IndexAction::Choice => self.cur_from_solver_pick(&state.pool, pick, pick.parents),
            IndexAction::PairSearch => self.recommended_cur(state, plan, pick),
        }
    }
    /// `act_on` "pair_search": the career the plan's check recommends (`next.robust.pairs.recommendation`, robust.rs
    /// `CheckWork::verdict`, what the page's `pairSearchNotes` shows): the plan's pick (`pick`, its `next.choice`) where
    /// it names that, or where the plan kept no check (no pair search: the page shows none); else the career it names,
    /// run with its own deck (`named_cur`)
    fn recommended_cur(&mut self, state: &PathState, plan: Option<&Plan>, pick: &CareerReport) -> Cur {
        let plan_pick = plan.and_then(|plan| plan.outcomes.as_ref()).map(|outcomes| outcomes.career.as_str());
        let check = match plan.and_then(|plan| plan.robust.as_ref()) {
            Some(CheckReply::Report(check)) => Some(check),
            _ => None,
        };
        let recommendation = check.and_then(|check| check.pairs.as_ref()).map(|pairs| &pairs.recommendation);
        let source = recommendation.map_or("no_check", |recommendation| recommendation.source);
        let career = recommendation.map_or(plan_pick, |recommendation| Some(recommendation.career.as_str()));
        let check_ms = check.map_or(0.0, |check| check.check_ms);
        INDEX_RUN_COUNTS.with(|counts| counts.borrow_mut().record_action(source, career, check_ms));
        match (source, recommendation) {
            ("no_check" | "plan_pick", _) | (_, None) => {
                assert!(
                    career == plan_pick && plan_pick.is_some(),
                    "index: the check's plan pick {career:?} is not the plan's {plan_pick:?}"
                );
                let cur = self.cur_from_solver_pick(&state.pool, pick, pick.parents);
                INDEX_RUN_COUNTS.with(|counts| counts.borrow_mut().record_executed(pick, &cur));
                cur
            }
            (_, Some(recommendation)) => self.named_cur(state, recommendation),
        }
    }
    /// The career `recommendation` names (its `career` id, `trainee_id` and `parents`), run as the page runs a named
    /// career: a `career` job (docs/SOLVER.md) with her the one trainee and both parents fixed, its deck as named, no
    /// deck search; its report gives the career's exact P(all) and landed-set chances
    fn named_cur(&mut self, state: &PathState, recommendation: &Recommendation) -> Cur {
        let job = self.own_job(state.legacies.iter(), |career| {
            career.next = None;
            career.trainees = vec![recommendation.trainee_id];
            career.fixed_parents = recommendation.parents.to_vec();
            career.given = Some(recommendation.career.clone());
        });
        let started = std::time::Instant::now();
        let report = run_career_in(self.data, self.skills, self.cards, &job, self.kept);
        let named = report.search().and_then(|search| search.results.first());
        let Some(named) = named.filter(|named| named.trainee.id == recommendation.trainee_id) else {
            panic!("index: the recommended career {} did not run: {}", recommendation.career, report.excerpt(300));
        };
        let seconds = (started.elapsed().as_secs_f64() * 1e3).round() / 1e3;
        let cur = self.cur_from_solver_pick(&state.pool, named, named.parents);
        INDEX_RUN_COUNTS.with(|counts| {
            let mut counts = counts.borrow_mut();
            counts.career_job_seconds.push(seconds);
            counts.record_executed(named, &cur);
        });
        cur
    }
}
