//! The roll of the plan's pick and its kept state.
use crate::career::clock::{PROF, now_ms};
use crate::career::plan::caps::StepCaps;
use crate::career::plan::cont::{Cont, ScoreBase, ScoringKnobs};
use crate::career::plan::report::{RollAnswer, RollOutcome, RollReply};
use crate::career::plan::score::kernel::Scored;
use crate::career::plan::search::sim_for;
use crate::career::plan::{OutcomeIndex, PlanJob};
use crate::career::search::Found;
use crate::career::sim::Sim;
use crate::career::state::KeptState;
use crate::career::{Env, JobInputs, Lineage};
use crate::deck::space::SelectedDeck;
use crate::refusal::Refusal;
use serde::{Deserialize, Serialize};
// ---------- a roll of the plan's pick, on demand (`next.roll`) ----------
/// The base a kept scoring was scored against (`ScoreBase` without the trios, their sides and C's umas, which only the
/// scoring's routes read as it is built: V after her reads V0 and the scoring's routes). Per-candidate vectors follow
/// `candidates`' order.
#[derive(Serialize, Deserialize)]
pub(crate) struct KeptBase {
    /// The stage candidates (`ScoreBase::candidates`).
    candidates: Vec<crate::uma::UmaIndex>,
    /// Each candidate's step (`ScoreBase::step_costs`).
    step_costs: Vec<f64>,
    /// V (`ScoreBase::current_value`).
    current_value: f64,
    /// Each candidate's odds now (`ScoreBase::current_odds`).
    current_odds: Vec<f64>,
    continuations: Vec<Cont>,
    /// Each candidate's step caps (`ScoreBase::step_caps`; empty: none).
    step_caps: Vec<StepCaps>,
}

impl KeptBase {
    pub(crate) fn of(score_base: &ScoreBase) -> Self {
        Self {
            candidates: score_base.candidates.to_vec(),
            step_costs: score_base.step_costs.to_vec(),
            current_value: score_base.current_value,
            current_odds: score_base.current_odds.to_vec(),
            continuations: score_base.continuations.to_vec(),
            step_caps: score_base.step_caps.to_vec(),
        }
    }
    pub(super) fn base(&self) -> ScoreBase<'_> {
        ScoreBase {
            candidates: &self.candidates,
            step_costs: &self.step_costs,
            current_value: self.current_value,
            current_odds: &self.current_odds,
            continuations: &self.continuations,
            step_caps: &self.step_caps,
            trios: &[],
            trio_sides: &[],
            finisher_umas: &[],
            knobs: ScoringKnobs::default(),
            odds_scales: crate::career::plan::cont::OddsScales::ONE,
        }
    }
}

/// A plan's roll state (`next.roll`), kept per plan job (`roll_key`): the pick's last scoring (`Scored`) and the base it
/// was scored against, and the pick's career
#[derive(Serialize, Deserialize)]
pub(crate) struct RollKeep {
    /// The plan job's roll key (`roll_key`).
    pub(crate) key: u64,
    pub(crate) scored: Scored,
    /// The later batches' scorings (`KeptScoring::later`): V after her the least over all of them.
    pub(crate) later: Vec<Scored>,
    pub(crate) base: KeptBase,
    /// The pick's trainee and parents, and her deck.
    pub(crate) lineage: Lineage,
    pub(crate) deck: SelectedDeck,
}

impl RollKeep {
    /// Its scorings' derived tables built again (`Scored::derive`): a restored roll state's (`state_in`) holds what
    /// they are built from, not the tables.
    pub(crate) fn derive(&mut self) {
        self.scored.derive();
        for scored in &mut self.later {
            scored.derive();
        }
    }
    /// Every batch's scoring, the first's first (`KeptScoring::later`).
    pub(crate) fn batches(&self) -> impl Iterator<Item = &Scored> {
        std::iter::once(&self.scored).chain(&self.later)
    }
    /// A roll's y: the most any batch saves at the outcome (each batch's y is V0 less its least route, rounded
    /// monotonically, so the most of them is V0 less the least over every batch's routes: the plan's table value)
    pub(super) fn roll(&self, score_base: &ScoreBase, outcome: OutcomeIndex) -> f64 {
        self.batches().map(|scored| scored.roll(score_base, outcome)).fold(0.0f64, f64::max)
    }
    /// The state is of this trainee's career on these parents, in this order, with this deck.
    pub(crate) fn is_of(&self, trainee: crate::uma::UmaIndex, career: &Found) -> bool {
        self.lineage.trainee == trainee
            && self.lineage.first_parent == career.first_parent
            && self.lineage.second_parent == career.second_parent
            && crate::career::plan::kept::same_deck(&self.deck, &career.deck)
    }
}

/// plans' roll states kept (the most recent last): the run's plan and the plan Compare makes with her added
const ROLL_KEEP: usize = 4;

pub(crate) fn keep_roll(kept: &mut KeptState, roll: RollKeep) {
    kept.rolls.retain(|entry| entry.key != roll.key);
    kept.rolls.push(roll);
    while kept.rolls.len() > ROLL_KEEP {
        kept.rolls.remove(0);
    }
}

/// The plan's answer to a roll (`next.roll`: her stars per job target, 0 where it did not land) from its kept roll state,
/// with no search (None when the worker keeps none for this plan job: then the plan runs and answers it, `index_rule`).
/// y: the careers her veteran saves at that outcome, the pick's kept scoring evaluated at that one outcome (`Scored::evaluate`,
/// the plan's own arithmetic in its order: the f64 the plan's full table held), rounded as the table was, to 0.01
pub(crate) fn roll_kept<'data>(
    env: &mut Env<'data>,
    worker: &mut KeptState,
    input: &JobInputs<'data>,
    plan_job: &PlanJob,
) -> Option<RollReply> {
    let stars = plan_job.roll.as_ref()?;
    let key = input.job.plan_key();
    let index = worker.rolls.iter().position(|roll| roll.key == key)?;
    let kept = worker.rolls.remove(index);
    // (the roll's own timers and counters, `prof`; what the job counted before goes back after)
    let started = now_ms();
    let prof_before = PROF.with(|prof| std::mem::take(&mut *prof.borrow_mut()));
    let outcome = match sim_for(env, worker, input, plan_job) {
        Ok(sim) => roll_answer(&sim, &kept, stars),
        Err(refusal) => RollOutcome::Refusal(refusal),
    };
    keep_roll(worker, kept);
    let prof = PROF.with(|prof| std::mem::replace(&mut *prof.borrow_mut(), prof_before));
    Some(RollReply::Answer(RollAnswer { outcome, ms: now_ms() - started, prof, planned: None }))
}

fn roll_answer(simulator: &Sim, kept: &RollKeep, stars: &[u8]) -> RollOutcome {
    let setup = &simulator.env.setup;
    if stars.len() != setup.target_count {
        return RollOutcome::Refusal(Refusal::new(format!(
            "roll: {} stars for {} targets",
            stars.len(),
            setup.target_count
        )));
    }
    let outcome = OutcomeIndex::from_stars(stars, &setup.uncertain_targets, &setup.fixed_targets);
    let score_base = kept.base.base();
    let y = kept.roll(&score_base, outcome);
    // (the scoring's routes only: a route the scoring has not, another lineage of a candidate's whose C her veteran
    // improves without lifting her odds, can save more, so the page confirms every roll with a Compare)
    RollOutcome::Saving { y: (y * 100.0).round() / 100.0 }
}
