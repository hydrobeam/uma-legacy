//! Career selection for greedy, loop, trace, exact and page-index policies, with rotation and index-run records.
//! Draws, path execution and exact audit comparisons live in the sibling modules.

use crate::career::evaluator::{Cur, DeckRequest, SurrogatePick};
use crate::career::plan;
use crate::career::plan::ScoringContext;
use crate::career::pool::Pool;
use crate::career::sim::draw::{Keys, Outcomes, VeteranDraw};
use crate::career::sim::play::PathState;
use crate::career::sim::{Goal, PAGE_TOP, Sim, leg_json};
use crate::career::{Career, Lineage, run_career_in};
use serde_json::{Value, json};
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
}
impl IndexRunCounts {
    fn record_plan(&mut self, report: &Value, started: std::time::Instant) {
        self.plans += 1;
        self.plan_seconds.push((started.elapsed().as_secs_f64() * 1e3).round() / 1e3);
        if let Some(chosen) = report["next"]["options"]
            .as_array()
            .and_then(|options| options.iter().find(|option| option["chosen"].as_bool() == Some(true)))
        {
            *self.roles.entry(chosen["role"].as_str().unwrap_or("?").to_string()).or_default() += 1;
            if chosen["best_odds"].as_bool() == Some(true) {
                self.best_odds_picks += 1;
            }
        }
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
    pub(super) fn trace(&mut self, initial_state: &PathState) -> Value {
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
                rows.push(json!([
                    career_index,
                    finisher_odds,
                    self.data.umas[builder.pick.lineage.trainee].name,
                    builder.lift_chance,
                    builder.lifted_odds,
                    after
                ]));
            }
            eprintln!(
                "trace run {}: {} careers, A from {:.3e} to {:.3e}, {:.1} s",
                run,
                rows.len(),
                initial_state.member_best[0].map_or(0.0, |pick| pick.value),
                state.member_best[0].map_or(0.0, |pick| pick.value),
                started.elapsed().as_secs_f64()
            );
            runs.push(json!({"run": run, "rows": rows}));
        }
        json!({"trace": runs, "trainee": self.data.umas[finisher].name, "builders": self.members[1..].iter().map(|&member| self.data.umas[member].name.clone()).collect::<Vec<_>>()})
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
            later_stage_costs: &[0.0],
            current_value: 0.0,
            current_odds: &current_odds,
            continuations: &[],
            trios: &[],
            trio_sides: &[],
            finisher_umas: &[],
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
            self.audits.push(json!({"t": state.rotation.position, "rotation": [previous_surrogate, new_surrogate, previous_exact, new_exact, (new_surrogate > previous_surrogate) == (new_exact > previous_exact)]}));
        }
        if new_surrogate > previous_surrogate {
            state.rotation.latest[member_index] = Some(veteran_index);
        }
    }
    /// Builds a runnable career from a selected internal solver result.
    /// The caller checks that `pick` is an object. Its required fields supply the
    /// exact probabilities; resolving the win chances also fills the schedule cache.
    fn cur_from_solver_pick(&mut self, pool: &Pool, pick: &Value) -> Cur {
        let trainee = self.env.setup.uma_index(pick["trainee"]["id"].as_u64().unwrap() as u32).unwrap();
        let (parent_a, parent_b) =
            (pick["parents"][0].as_u64().unwrap() as usize, pick["parents"][1].as_u64().unwrap() as usize);
        let outcome_sets: Vec<f64> =
            pick["outcome_sets"].as_array().unwrap().iter().map(|chance| chance.as_f64().unwrap()).collect();
        let lineage = Lineage { trainee, first_parent: parent_a, second_parent: parent_b };
        // (the outfit the pick's schedule ran: the report's `schedule_outfit`)
        let outfit = crate::career::schedule::ScheduleOutfits::Card(
            pick["trainee"]["schedule_outfit"].as_u64().unwrap_or(0) as u32,
        );
        let record = self.evaluator.wins(&self.env.setup, pool, lineage, outfit);
        Cur { lineage, p_all: pick["p_all"].as_f64().unwrap(), outcome_sets, record, rotation: false }
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
        let mut job = self.job.clone();
        let job_fields = job.as_object_mut()?;
        job_fields.remove("simulate");
        job_fields.insert(
            "legacies".into(),
            Value::Array(legacies.iter().map(|&legacy| leg_json(&state.legacies[legacy])).collect()),
        );
        // (the job's `fix` indexes its own legacies, not these: the simulator ignores it)
        job_fields.insert("fix".into(), json!([]));
        job_fields.insert("top".into(), json!(1));
        job_fields.insert("per_trainee".into(), json!(false));
        let report = run_career_in(self.data, self.skills, self.cards, &job, self.kept);
        let mut pick = report["results"][0].clone();
        if !pick.is_object() {
            return None;
        }
        // (the search's parents index `legacies`; the path's pool indexes every legacy)
        for parent in pick["parents"].as_array_mut()? {
            *parent = json!(legacies[parent.as_u64()? as usize]);
        }
        Some(self.cur_from_solver_pick(&state.pool, &pick))
    }
    /// `index`: the page's plan for this pool (the career job with the pool as legacies, no places, the page's `top`,
    /// `next`, `no_reports`: the plan job the page sends, docs/SOLVER.md "Next career"), in process; its `choice` runs.
    /// The job's `trainees` stay (the page sends none: every owned uma)
    pub(super) fn index_cur(&mut self, state: &PathState) -> Cur {
        let mut job = self.job.clone();
        let job_fields = job.as_object_mut().unwrap();
        job_fields.remove("simulate");
        job_fields.insert("legacies".into(), Value::Array(state.legacies.iter().map(leg_json).collect()));
        job_fields.insert("fix".into(), json!([]));
        job_fields.insert("top".into(), json!(PAGE_TOP));
        job_fields.insert("per_trainee".into(), json!(false));
        job_fields.insert("next".into(), self.settings.next.clone());
        job_fields.insert("no_reports".into(), json!(true));
        let started = std::time::Instant::now();
        let report = run_career_in(self.data, self.skills, self.cards, &job, self.kept);
        let pick = report["next"]["choice"].clone();
        assert!(
            pick.is_object(),
            "index: the plan gave no pick: {}",
            report.to_string().chars().take(300).collect::<String>()
        );
        INDEX_RUN_COUNTS.with(|counts| counts.borrow_mut().record_plan(&report, started));
        self.cur_from_solver_pick(&state.pool, &pick)
    }
}
