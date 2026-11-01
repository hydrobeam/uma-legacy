//! The roll of the plan's pick and its kept state.
use crate::career::clock::{PROF, now_ms};
use crate::career::plan::caps::{StepCaps, StoredCap, cap_in, cap_out};
use crate::career::plan::cont::{Cont, MAX_PARTNERS, ScoreBase, ScoringKnobs};
use crate::career::plan::report::{RollAnswer, RollOutcome, RollReply};
use crate::career::plan::score::fm_masks;
use crate::career::plan::score::kernel::{BatchLeast, Scored};
use crate::career::plan::search::sim_for;
use crate::career::plan::step::{NO_SLOT, Route, WhileBuilding};
use crate::career::plan::{OutcomeIndex, PlanJob, TargetMask};
use crate::career::search::Found;
use crate::career::sim::Sim;
use crate::career::state::{KeptState, StoredDeck, chances, number, numbers, sel_in, sel_out};
use crate::career::{Env, JobInputs, Lineage};
use crate::deck::space::SelectedDeck;
use crate::hex::{Bits, Key, bits};
use crate::refusal::Refusal;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
// ---------- a roll of the plan's pick, on demand (`next.roll`) ----------
/// The base a kept scoring was scored against (`ScoreBase` without the trios, their sides and C's umas, which only the
/// scoring's routes read as it is built: V after her reads V0 and the scoring's routes). Per-candidate vectors follow
/// `candidates`' order.
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

/// A roll state as stored (`RollKeep`): its key, the pick's lineage (uma index, two legacy indices) and deck, its base
/// and its scorings (the first batch's, then the later ones'), every float as its bits.
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredRoll {
    pub(crate) key: Key,
    pub(crate) lineage: [usize; 3],
    pub(crate) deck: StoredDeck,
    pub(crate) base: StoredBase,
    pub(crate) scored: StoredScored,
    pub(crate) later: Vec<StoredScored>,
}

/// A roll state's base (`KeptBase`): the candidates (uma indices), their steps' costs, V, their odds now, their
/// continuations and held caps.
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredBase {
    pub(crate) candidates: Vec<usize>,
    pub(crate) step_costs: Vec<Bits>,
    pub(crate) current_value: Bits,
    pub(crate) current_odds: Vec<Bits>,
    /// per candidate: [lam, phi, pinf, whether it has its table (it builds), p0]
    pub(crate) continuations: Vec<(Bits, Bits, Bits, bool, Bits)>,
    /// per candidate, the caps she holds
    pub(crate) step_caps: Vec<Vec<StoredCap>>,
}

/// A stored `Scored` (score/kernel.rs), its derived tables left out (`derive` rebuilds them).
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredScored {
    pub(crate) uncertain_target_count: usize,
    pub(crate) fixed_target_count: usize,
    pub(crate) target_count: usize,
    pub(crate) slot_count: usize,
    pub(crate) completes_goal: bool,
    pub(crate) star_mix: [Bits; 3],
    pub(crate) uncertain_targets: Vec<usize>,
    pub(crate) fixed_landed_masks: Vec<usize>,
    pub(crate) misses_without_her: Vec<Vec<Bits>>,
    pub(crate) miss_by_stars: Vec<[Bits; 4]>,
    /// per setup, each target's pair of spark scales
    pub(crate) target_spark_scales: Vec<Vec<[Bits; 2]>>,
    pub(crate) setup_by_table: Vec<usize>,
    pub(crate) initial_tables: Vec<Bits>,
    pub(crate) collapsed_targets: Vec<bool>,
    pub(crate) slot_by_setup: Vec<usize>,
    pub(crate) current_probabilities: Vec<Bits>,
    pub(crate) landed_set_chances: Vec<Bits>,
    pub(crate) slot_candidates: Vec<usize>,
    pub(crate) lift_candidates: Vec<usize>,
    pub(crate) routes: Vec<StoredRoute>,
}

/// A route as stored: its lifted slots and finisher slot (null: none), whether it takes full veterans only, its fixed
/// cost, and its C while a lifted side builds (null: skipped).
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredRoute {
    pub(crate) lifts: [Option<usize>; 2],
    pub(crate) full_only: bool,
    pub(crate) fixed_cost: Bits,
    pub(crate) finisher_slot: Option<usize>,
    pub(crate) while_building: Option<StoredBuilding>,
}

/// A route's C while a lifted side builds: its step's fixed cost, or a finisher slot's.
#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub(crate) enum StoredBuilding {
    Fixed(Bits),
    Slot(usize),
}

const fn slot_out(slot: usize) -> Option<usize> {
    if slot == NO_SLOT { None } else { Some(slot) }
}

const fn building_out(while_building: WhileBuilding) -> Option<StoredBuilding> {
    match while_building {
        WhileBuilding::Fixed(cost) => Some(StoredBuilding::Fixed(Bits(cost))),
        WhileBuilding::Slot(slot) => Some(StoredBuilding::Slot(slot)),
        WhileBuilding::Skip => None,
    }
}

fn building_in(stored: Option<&StoredBuilding>) -> Option<WhileBuilding> {
    match stored {
        None => Some(WhileBuilding::Skip),
        Some(&StoredBuilding::Fixed(cost)) => Some(cost.0).filter(|cost| *cost >= 0.0).map(WhileBuilding::Fixed),
        Some(&StoredBuilding::Slot(slot)) => Some(WhileBuilding::Slot(slot)),
    }
}

fn base_out(base: &KeptBase) -> StoredBase {
    StoredBase {
        candidates: base.candidates.iter().map(|candidate| candidate.0).collect(),
        step_costs: bits(&base.step_costs),
        current_value: Bits(base.current_value),
        current_odds: bits(&base.current_odds),
        continuations: base
            .continuations
            .iter()
            .map(|continuation| {
                (
                    Bits(continuation.lam),
                    Bits(continuation.phi),
                    Bits(continuation.pinf),
                    !continuation.tables.is_empty(),
                    Bits(continuation.anchor_odds),
                )
            })
            .collect(),
        step_caps: base.step_caps.iter().map(|caps| caps.curves.iter().map(cap_out).collect()).collect(),
    }
}

fn scored_out(scored: &Scored) -> StoredScored {
    StoredScored {
        uncertain_target_count: scored.uncertain_target_count,
        fixed_target_count: scored.fixed_target_count,
        target_count: scored.target_count,
        slot_count: scored.slot_count,
        completes_goal: scored.completes_goal,
        star_mix: scored.star_mix.map(Bits),
        uncertain_targets: scored.uncertain_targets.to_vec(),
        fixed_landed_masks: scored.fixed_landed_masks.iter().map(|mask| mask.bits()).collect(),
        misses_without_her: scored.misses_without_her.iter().map(|misses| bits(misses)).collect(),
        miss_by_stars: scored.miss_by_stars.iter().map(|misses| misses.map(Bits)).collect(),
        target_spark_scales: scored
            .target_spark_scales
            .iter()
            .map(|scales| scales.iter().map(|pair| pair.map(Bits)).collect())
            .collect(),
        setup_by_table: scored.setup_by_table.clone(),
        initial_tables: bits(&scored.initial_tables),
        collapsed_targets: scored.collapsed_targets.clone(),
        slot_by_setup: scored.slot_by_setup.clone(),
        current_probabilities: bits(&scored.current_probabilities),
        landed_set_chances: bits(&scored.landed_set_chances),
        slot_candidates: scored.slot_candidates.clone(),
        lift_candidates: scored.lift_candidates.clone(),
        routes: scored
            .routes
            .iter()
            .map(|route| StoredRoute {
                lifts: route.lifts.map(slot_out),
                full_only: route.full_only,
                fixed_cost: Bits(route.fixed_cost),
                finisher_slot: slot_out(route.finisher_slot),
                while_building: building_out(route.while_building),
            })
            .collect(),
    }
}

/// the kept roll states for the page to store (`state_out`)
pub(crate) fn rolls_out(rolls: &[RollKeep]) -> Vec<StoredRoll> {
    rolls
        .iter()
        .map(|roll| StoredRoll {
            key: Key(roll.key),
            lineage: [roll.lineage.trainee.0, roll.lineage.first_parent, roll.lineage.second_parent],
            deck: sel_out(&roll.deck),
            base: base_out(&roll.base),
            scored: scored_out(&roll.scored),
            later: roll.later.iter().map(scored_out).collect(),
        })
        .collect()
}

/// Kept roll states from the page's store (`state_in`); None refuses the store whole.
pub(crate) fn rolls_in(stored: &[StoredRoll]) -> Option<Vec<RollKeep>> {
    stored
        .iter()
        .map(|entry| {
            let base = kept_base_in(&entry.base)?;
            // (a store that does not fit together is refused whole, as any other: the vectors' lengths and every index
            // within what it indexes, as `evaluate_career` builds them, before `derive` sizes its tables; each later
            // batch fitting the base as the first)
            let fitted = |stored: &StoredScored| {
                let mut scored = scored_in(stored)?;
                scored.fits(&base).then(|| {
                    scored.derive();
                    scored
                })
            };
            let scored = fitted(&entry.scored)?;
            let later = entry.later.iter().map(fitted).collect::<Option<Vec<_>>>()?;
            let [trainee, first_parent, second_parent] = entry.lineage;
            Some(RollKeep {
                key: entry.key.0,
                scored,
                later,
                base,
                lineage: Lineage { trainee: crate::uma::UmaIndex(trainee), first_parent, second_parent },
                deck: sel_in(&entry.deck)?,
            })
        })
        .collect()
}

fn kept_base_in(stored: &StoredBase) -> Option<KeptBase> {
    let continuations = stored
        .continuations
        .iter()
        .map(|&(lam, phi, pinf, has_table, anchor_odds)| {
            let (lam, phi, pinf, anchor_odds) = (number(lam)?, number(phi)?, number(pinf)?, number(anchor_odds)?);
            // (a Cont without its table has no building: `Cont::default`'s lam 0; one that reads as building is refused)
            Some(if has_table {
                Cont::new(lam, phi, pinf, anchor_odds)
            } else if lam > 0.0 && phi > 0.0 {
                return None;
            } else {
                Cont { lam, phi, pinf, anchor_odds, ..Cont::default() }
            })
        })
        .collect::<Option<Vec<Cont>>>()?;
    let current_odds = numbers(&stored.current_odds)?;
    // (each candidate's held caps, read at her odds now)
    let step_caps = stored
        .step_caps
        .iter()
        .enumerate()
        .map(|(candidate, caps)| {
            let curves = caps.iter().map(cap_in).collect::<Option<Vec<_>>>()?;
            Some(StepCaps::new(curves, *current_odds.get(candidate)?))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(KeptBase {
        candidates: stored.candidates.iter().map(|&candidate| crate::uma::UmaIndex(candidate)).collect(),
        step_costs: numbers(&stored.step_costs)?,
        current_value: number(stored.current_value)?,
        current_odds,
        continuations,
        step_caps,
    })
}

/// A stored `Scored`, its derived tables left empty (`derive`)
fn scored_in(stored: &StoredScored) -> Option<Scored> {
    let StoredScored { uncertain_target_count, fixed_target_count, target_count, slot_count, .. } = *stored;
    // (the plan's limits, `sim_for`: at most 12 targets, 10 of them uncertain, the rest guaranteed; checked before
    // any table is sized by them)
    if target_count > 12
        || uncertain_target_count > 10
        || fixed_target_count > 12
        || uncertain_target_count + fixed_target_count != target_count
    {
        return None;
    }
    let four = |values: &[Bits; 4]| -> Option<[f64; 4]> { numbers(values)?.try_into().ok() };
    let star_mix: [f64; 3] = chances(&stored.star_mix)?.try_into().ok()?;
    let target_spark_scales = stored
        .target_spark_scales
        .iter()
        .map(|scales| {
            (scales.len() == target_count)
                .then(|| scales.iter().map(|&[first, second]| Some([number(first)?, number(second)?])).collect())
                .flatten()
        })
        .collect::<Option<Vec<Vec<[f64; 2]>>>>()?;
    let routes = stored
        .routes
        .iter()
        .map(|route| {
            Some(Route {
                lifts: route.lifts.map(|slot| slot.unwrap_or(NO_SLOT)),
                full_only: route.full_only,
                fixed_cost: number(route.fixed_cost)?,
                finisher_slot: route.finisher_slot.unwrap_or(NO_SLOT),
                while_building: building_in(route.while_building.as_ref())?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Scored {
        uncertain_target_count,
        fixed_target_count,
        target_count,
        slot_count,
        slot_candidates: stored.slot_candidates.clone(),
        lift_candidates: stored.lift_candidates.clone(),
        completes_goal: stored.completes_goal,
        star_mix,
        uncertain_targets: stored.uncertain_targets.clone().into(),
        fixed_landed_masks: stored.fixed_landed_masks.iter().map(|&mask| TargetMask::from_bits(mask)).collect(),
        misses_without_her: stored.misses_without_her.iter().map(|misses| numbers(misses)).collect::<Option<_>>()?,
        miss_by_stars: stored.miss_by_stars.iter().map(four).collect::<Option<_>>()?,
        target_spark_scales,
        setup_by_table: stored.setup_by_table.clone(),
        initial_tables: numbers(&stored.initial_tables)?,
        collapsed_targets: stored.collapsed_targets.clone(),
        slot_by_setup: stored.slot_by_setup.clone(),
        current_probabilities: numbers(&stored.current_probabilities)?,
        landed_set_chances: chances(&stored.landed_set_chances)?.into(),
        routes,
        // (a roll state's base prices every C at its common scale, 1)
        finisher_umas: vec![],
        present_factors: vec![],
        absent_factors: vec![],
        spark_scales: vec![],
        shadows: vec![],
        prior_least: BatchLeast::default(),
        retain_least: false,
    })
}

impl Scored {
    /// A stored scoring and its base fit together: the vectors' lengths, and every index within what it indexes
    fn fits(&self, base: &KeptBase) -> bool {
        let (setup_count, candidate_count) = (self.misses_without_her.len(), base.candidates.len());
        let fixed: Vec<usize> = (0..self.target_count).filter(|i| !self.uncertain_targets.contains(i)).collect();
        let fits_base = candidate_count <= crate::career::state::STATE_MAX_N
            && base.step_costs.len() == candidate_count
            && base.current_odds.len() == candidate_count
            && base.continuations.len() <= candidate_count
            && base.step_caps.len() <= candidate_count;
        // her setups, one slot each (`slot_count` and `slot_by_setup` the identity): the partner slots (at most
        // MAX_PARTNERS per candidate, in candidate order) and the lift slots (her Current side's, one per partner slot,
        // and two per Alternative side of hers, at most MAX_FRESH_LINEAGES - 1; candidate indices), then one finisher
        // slot per route (at most MAX_ROUTES routes); each route's lifts lift slots, its finisher and its C while
        // building (`Slot`) finisher slots
        let (partner_slot_count, lift_slot_count) = (self.slot_candidates.len(), self.lift_candidates.len());
        let finisher_slot_count = setup_count.saturating_sub(partner_slot_count + lift_slot_count);
        let lift_slots_per_candidate = MAX_PARTNERS + 2 * (crate::career::plan::combine::MAX_FRESH_LINEAGES - 1);
        let fits_cands = partner_slot_count <= MAX_PARTNERS * candidate_count
            && lift_slot_count <= lift_slots_per_candidate * candidate_count
            && self.slot_candidates.iter().chain(&self.lift_candidates).all(|&candidate| candidate < candidate_count)
            && self.slot_candidates.windows(2).all(|pair| pair[0] <= pair[1])
            && self.routes.len() <= crate::career::plan::step::MAX_ROUTES
            && setup_count >= partner_slot_count + lift_slot_count
            && self.slot_count == setup_count
            && self.slot_by_setup.iter().enumerate().all(|(setup, &slot)| slot == setup)
            && self.routes.iter().all(|route| {
                // (a route that reads no C of its own has no finisher slot, NO_SLOT; the others one each, in order)
                route.lifts.iter().all(|&slot| slot == NO_SLOT || slot < lift_slot_count)
                    && (route.finisher_slot == NO_SLOT || route.finisher_slot < finisher_slot_count)
                    && match route.while_building {
                        WhileBuilding::Slot(slot) => slot < finisher_slot_count,
                        _ => true,
                    }
            })
            && self.routes.iter().filter(|route| route.finisher_slot != NO_SLOT).count() == finisher_slot_count;
        // (what `derive` allocates, checked: per landed set and setup a spark scale, per setup and uncertain target two
        // star rows; at most what a plan's scoring holds, `OUTCOME_BUDGET`)
        let derived = (1usize << self.target_count.min(12))
            .checked_mul(setup_count)
            .and_then(|scales| scales.checked_add(setup_count.checked_mul(8 * self.uncertain_target_count)?));
        let fits_size = derived.is_some_and(|entries| entries <= crate::career::plan::score::build::OUTCOME_BUDGET);
        fits_base
            && fits_size
            && fits_cands
            && self.uncertain_targets.windows(2).all(|window| window[0] < window[1])
            && !self.uncertain_targets.last().is_some_and(|&i| i >= self.target_count)
            && *self.fixed_landed_masks == *fm_masks(&fixed)
            && !self.slot_by_setup.windows(2).any(|window| window[0] > window[1])
            && self.uncertain_targets.len() == self.uncertain_target_count
            && self.collapsed_targets.len() == self.uncertain_target_count
            && self.miss_by_stars.len() == setup_count
            && self.target_spark_scales.len() == setup_count
            && self.slot_by_setup.len() == setup_count
            && self.landed_set_chances.len() == 1usize << self.target_count
            && self.fixed_landed_masks.len() == 1usize << self.fixed_target_count
            && self.initial_tables.len() == self.setup_by_table.len() << self.uncertain_target_count
            && !self.misses_without_her.iter().any(|value| value.len() != self.uncertain_target_count)
            && !self.setup_by_table.iter().any(|&setup| setup >= setup_count)
            && !self.slot_by_setup.iter().any(|&slot| slot >= self.slot_count)
            && self.current_probabilities.len() == self.slot_count
    }
}
