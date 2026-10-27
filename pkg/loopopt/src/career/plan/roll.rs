//! The roll of the plan's pick and its kept state.
use crate::career::clock::{PROF, now_ms};
use crate::career::plan::caps::{StepCaps, cap_in, cap_out};
use crate::career::plan::cont::{Cont, MAX_PARTNERS, ScoreBase, ScoringKnobs};
use crate::career::plan::score::fm_masks;
use crate::career::plan::score::kernel::Scored;
use crate::career::plan::search::sim_for;
use crate::career::plan::step::{NO_SLOT, Route, WhileBuilding};
use crate::career::plan::{OutcomeIndex, PlanJob, TargetMask};
use crate::career::search::Found;
use crate::career::sim::Sim;
use crate::career::state::{KeptState, hash_text};
use crate::career::{Env, JobInputs, Lineage};
use crate::deck::space::SelectedDeck;
use serde_json::{Value, json};
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

/// a plan job's roll key: the whole job but its roll (`next.roll`), its check and oracle requests (`next.robust`,
/// `next.oracle`) and the page's flags that change no plan (`no_reports`, `compare`)
pub(crate) fn roll_key(job: &Value) -> u64 {
    hash_text(&crate::career::state::json_without(
        job,
        &["no_reports", "compare", "roll"],
        &["roll", "robust", "oracle", "pair_search"],
    ))
}

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
) -> Option<Value> {
    let stars = plan_job.roll.as_ref()?;
    let key = roll_key(input.job);
    let index = worker.rolls.iter().position(|roll| roll.key == key)?;
    let kept = worker.rolls.remove(index);
    // (the roll's own timers and counters, `prof`; what the job counted before goes back after)
    let started = now_ms();
    let prof_before = PROF.with(|prof| std::mem::take(&mut *prof.borrow_mut()));
    let answer = match sim_for(env, worker, input, plan_job) {
        Ok(sim) => roll_answer(&sim, &kept, stars),
        Err(error) => error,
    };
    keep_roll(worker, kept);
    let mut answer = answer;
    let roll_prof = PROF.with(|prof| std::mem::replace(&mut *prof.borrow_mut(), prof_before));
    if answer.is_object() {
        answer["ms"] = json!(now_ms() - started);
        answer["prof"] = json!(roll_prof);
    }
    Some(answer)
}

fn roll_answer(simulator: &Sim, kept: &RollKeep, stars: &[u8]) -> Value {
    let setup = &simulator.env.setup;
    if stars.len() != setup.target_count {
        return json!({"error": format!("roll: {} stars for {} targets", stars.len(), setup.target_count)});
    }
    let outcome = OutcomeIndex::from_stars(stars, &setup.uncertain_targets, &setup.fixed_targets);
    let score_base = kept.base.base();
    let y = kept.roll(&score_base, outcome);
    // (the scoring's routes only: a route the scoring has not, another lineage of a candidate's whose C her veteran
    // improves without lifting her odds, can save more, so the page confirms every roll with a Compare)
    json!({"y": (y * 100.0).round() / 100.0})
}

/// a route's C while a lifted side builds, stored: {"f": its step's bits}, {"s": a finisher slot} or null
fn building_out(while_building: WhileBuilding) -> Value {
    match while_building {
        WhileBuilding::Fixed(cost) => json!({"f": crate::career::state::hx(cost)}),
        WhileBuilding::Slot(slot) => json!({"s": slot}),
        WhileBuilding::Skip => Value::Null,
    }
}

fn building_in(stored: &Value) -> Option<WhileBuilding> {
    use crate::career::state::{fx, usz};
    if stored.is_null() {
        return Some(WhileBuilding::Skip);
    }
    match (stored.get("f"), stored.get("s")) {
        (Some(cost), None) => fx(cost).filter(|cost| *cost >= 0.0).map(WhileBuilding::Fixed),
        (None, Some(slot)) => usz(slot).map(WhileBuilding::Slot),
        _ => None,
    }
}

/// the kept roll states for the page to store (`state_out`), every float as its bits
pub(crate) fn rolls_out(rolls: &[RollKeep]) -> Value {
    use crate::career::state::{fvec, hx, sel_out, ux};
    let slot_out = |slot: usize| if slot == NO_SLOT { Value::Null } else { json!(slot) };
    let base_out = |base: &KeptBase| {
        json!({"cands": base.candidates, "c1": fvec(&base.step_costs), "v0": hx(base.current_value), "pnow": fvec(&base.current_odds),
            "cont": base.continuations.iter().map(|continuation| json!([hx(continuation.lam), hx(continuation.phi), hx(continuation.pinf), !continuation.tables.is_empty(), hx(continuation.anchor_odds)])).collect::<Vec<_>>(),
            "caps": base.step_caps.iter().map(|caps| caps.curves.iter().map(cap_out).collect::<Vec<_>>()).collect::<Vec<_>>()})
    };
    let scored_out = |scored: &Scored| {
        json!({"n": [scored.uncertain_target_count, scored.fixed_target_count, scored.target_count, scored.slot_count], "goal": scored.completes_goal, "w": fvec(&scored.star_mix), "uncv": scored.uncertain_targets, "fmm": scored.fixed_landed_masks,
            "x0": scored.misses_without_her.iter().map(|value| fvec(value)).collect::<Vec<_>>(), "xm": scored.miss_by_stars.iter().map(|value| fvec(value)).collect::<Vec<_>>(),
            "pw": scored.target_spark_scales.iter().map(|value| fvec(&value.iter().flatten().copied().collect::<Vec<f64>>())).collect::<Vec<_>>(),
            "tsi": scored.setup_by_table, "t0": fvec(&scored.initial_tables), "col": scored.collapsed_targets, "sslot": scored.slot_by_setup, "p0": fvec(&scored.current_probabilities), "os": fvec(&scored.landed_set_chances),
            "sc": scored.slot_candidates, "lc": scored.lift_candidates,
            "rt": scored.routes.iter().map(|route| json!([slot_out(route.lifts[0]), slot_out(route.lifts[1]), route.full_only, hx(route.fixed_cost), slot_out(route.finisher_slot), building_out(route.while_building)])).collect::<Vec<_>>()})
    };
    Value::Array(
        rolls
            .iter()
            .map(|roll| {
                let lineage = roll.lineage;
                json!({
                    "k": ux(roll.key), "x": [lineage.trainee, lineage.first_parent, lineage.second_parent],
                    "sel": sel_out(&roll.deck),
                    "base": base_out(&roll.base),
                    "sd": scored_out(&roll.scored), "sdl": roll.later.iter().map(scored_out).collect::<Vec<_>>()})
            })
            .collect(),
    )
}

/// Kept roll states from the page's store (`state_in`); None refuses the store whole. (Absent in a state stored before
/// them: none.)
pub(crate) fn rolls_in(value: &Value) -> Option<Vec<RollKeep>> {
    use crate::career::state::{sel_in, uxr};
    let Some(stored) = value.as_array() else { return value.is_null().then(Vec::new) };
    stored
        .iter()
        .map(|entry| {
            let base = kept_base_in(&entry["base"])?;
            let mut scored = scored_in(&entry["sd"])?;
            // (a store that does not fit together is refused whole, as any other: the vectors' lengths and every index
            // within what it indexes, as `evaluate_career` builds them, before `derive` sizes its tables)
            if !scored.fits(&base) {
                return None;
            }
            scored.derive();
            // (the later batches, each fitting the base as the first; absent, as stored before them: none)
            let later = match entry.get("sdl") {
                None => vec![],
                Some(stored) => stored
                    .as_array()?
                    .iter()
                    .map(|stored_batch| {
                        let mut batch = scored_in(stored_batch)?;
                        batch.fits(&base).then(|| {
                            batch.derive();
                            batch
                        })
                    })
                    .collect::<Option<Vec<_>>>()?,
            };
            let lineage = usize_vec(&entry["x"])?;
            let &[trainee, first_parent, second_parent] = lineage.as_slice() else { return None };
            Some(RollKeep {
                key: uxr(&entry["k"])?,
                scored,
                later,
                base,
                lineage: Lineage { trainee: crate::uma::UmaIndex(trainee), first_parent, second_parent },
                deck: sel_in(&entry["sel"])?,
            })
        })
        .collect()
}

fn usize_vec(value: &Value) -> Option<Vec<usize>> {
    value.as_array()?.iter().map(crate::career::state::usz).collect()
}

fn kept_base_in(stored: &Value) -> Option<KeptBase> {
    use crate::career::state::{fvec_r, fx};
    let continuations = stored["cont"]
        .as_array()?
        .iter()
        .map(|continuation| {
            let (lam, phi, pinf) = (fx(&continuation[0])?, fx(&continuation[1])?, fx(&continuation[2])?);
            let anchor_odds = fx(&continuation[4])?;
            // (a Cont without its table has no building: `Cont::default`'s lam 0; one that reads as building is refused)
            Some(if continuation[3].as_bool()? {
                Cont::new(lam, phi, pinf, anchor_odds)
            } else if lam > 0.0 && phi > 0.0 {
                return None;
            } else {
                Cont { lam, phi, pinf, anchor_odds, ..Cont::default() }
            })
        })
        .collect::<Option<Vec<Cont>>>()?;
    let current_odds = fvec_r(&stored["pnow"])?;
    // (each candidate's held caps, read at her odds now: a list of caps, or, as stored before, one cap or null; absent:
    // none)
    let step_caps = match stored["caps"].as_array() {
        Some(caps) => caps
            .iter()
            .enumerate()
            .map(|(candidate, stored_caps)| {
                let curves = match stored_caps.as_array() {
                    None if stored_caps.is_null() => vec![],
                    Some(list) if list.iter().all(Value::is_array) => {
                        list.iter().map(cap_in).collect::<Option<Vec<_>>>()?
                    }
                    _ => vec![cap_in(stored_caps)?],
                };
                Some(StepCaps::new(curves, *current_odds.get(candidate)?))
            })
            .collect::<Option<Vec<_>>>()?,
        None => vec![],
    };
    Some(KeptBase {
        candidates: usize_vec(&stored["cands"])?.into_iter().map(crate::uma::UmaIndex).collect(),
        step_costs: fvec_r(&stored["c1"])?,
        current_value: fx(&stored["v0"])?,
        current_odds,
        continuations,
        step_caps,
    })
}

/// A stored `Scored`, its derived tables left empty (`derive`)
fn scored_in(stored: &Value) -> Option<Scored> {
    use crate::career::state::{fvec_r, fx, pvec_r, usz};
    let sizes = usize_vec(&stored["n"])?;
    // (the plan's limits, `sim_for`: at most 12 targets, 10 of them uncertain, the rest guaranteed; checked before
    // any table is sized by them)
    let &[uncertain_target_count, fixed_target_count, target_count, slot_count] = sizes.as_slice() else { return None };
    if target_count > 12
        || uncertain_target_count > 10
        || fixed_target_count > 12
        || uncertain_target_count + fixed_target_count != target_count
    {
        return None;
    }
    let four = |value: &Value| -> Option<[f64; 4]> {
        let values = fvec_r(value)?;
        values.try_into().ok()
    };
    let star_mix: [f64; 3] = pvec_r(&stored["w"])?.try_into().ok()?;
    let target_spark_scales = stored["pw"]
        .as_array()?
        .iter()
        .map(|value| {
            let scales = fvec_r(value)?;
            (scales.len() == 2 * target_count).then(|| scales.chunks(2).map(|pair| [pair[0], pair[1]]).collect())
        })
        .collect::<Option<Vec<Vec<[f64; 2]>>>>()?;
    let slot_in = |value: &Value| if value.is_null() { Some(NO_SLOT) } else { usz(value) };
    let routes = stored["rt"]
        .as_array()?
        .iter()
        .map(|route| {
            Some(Route {
                lifts: [slot_in(&route[0])?, slot_in(&route[1])?],
                full_only: route[2].as_bool()?,
                fixed_cost: fx(&route[3])?,
                finisher_slot: slot_in(&route[4])?,
                while_building: building_in(route.get(5)?)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Scored {
        uncertain_target_count,
        fixed_target_count,
        target_count,
        slot_count,
        slot_candidates: usize_vec(&stored["sc"])?,
        lift_candidates: usize_vec(&stored["lc"])?,
        completes_goal: stored["goal"].as_bool()?,
        star_mix,
        uncertain_targets: usize_vec(&stored["uncv"])?.into(),
        fixed_landed_masks: usize_vec(&stored["fmm"])?
            .into_iter()
            .map(TargetMask::from_bits)
            .collect::<Vec<_>>()
            .into(),
        misses_without_her: stored["x0"].as_array()?.iter().map(fvec_r).collect::<Option<_>>()?,
        miss_by_stars: stored["xm"].as_array()?.iter().map(four).collect::<Option<_>>()?,
        target_spark_scales,
        setup_by_table: usize_vec(&stored["tsi"])?,
        initial_tables: fvec_r(&stored["t0"])?,
        collapsed_targets: stored["col"].as_array()?.iter().map(serde_json::Value::as_bool).collect::<Option<_>>()?,
        slot_by_setup: usize_vec(&stored["sslot"])?,
        current_probabilities: fvec_r(&stored["p0"])?,
        landed_set_chances: pvec_r(&stored["os"])?.into(),
        routes,
        // (a roll state's base prices every C at its common scale, 1)
        finisher_umas: vec![],
        present_factors: vec![],
        absent_factors: vec![],
        spark_scales: vec![],
        shadows: vec![],
        prior_least: vec![],
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
