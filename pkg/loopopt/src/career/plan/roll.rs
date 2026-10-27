//! The roll of the plan's pick and its kept state.
use crate::career::clock::{PROF, now_ms};
use crate::career::plan::anchors::{FoundAnchor, PartnerAnchors};
use crate::career::plan::caps::{StepCap, cap_in, cap_out};
use crate::career::plan::cont::{Cont, MAX_PARTNERS, ScoreBase, ScoringKnobs};
use crate::career::plan::score::fm_masks;
use crate::career::plan::score::kernel::Scored;
use crate::career::plan::search::sim_for;
use crate::career::plan::step::{NO_SLOT, Route, WhileBuilding};
use crate::career::plan::{OutcomeIndex, PlanJob, ScoringContext, TargetMask};
use crate::career::pool::Pool;
use crate::career::search::Found;
use crate::career::sim::Sim;
use crate::career::state::{KeptState, hash_text};
use crate::career::{Env, JobInputs, Legacy, Lineage};
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
    /// Whether each is her proven best career's (`ScoreBase::certified`).
    certified: Vec<bool>,
    continuations: Vec<Cont>,
    /// Each candidate's step cap (`ScoreBase::step_caps`; empty: none).
    step_caps: Vec<Option<StepCap>>,
}

impl KeptBase {
    pub(crate) fn of(score_base: &ScoreBase) -> Self {
        Self {
            candidates: score_base.candidates.to_vec(),
            step_costs: score_base.step_costs.to_vec(),
            current_value: score_base.current_value,
            current_odds: score_base.current_odds.to_vec(),
            certified: score_base.certified.to_vec(),
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
            certified: &self.certified,
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
/// was scored against, the pick's career, and what its bound reads beyond them (`outcome_hi`): the main search's
/// found-career anchors (`found_anchors`)
pub(crate) struct RollKeep {
    /// The plan job's roll key (`roll_key`).
    pub(crate) key: u64,
    pub(crate) scored: Scored,
    pub(crate) base: KeptBase,
    /// The pick's trainee and parents, and her deck.
    pub(crate) lineage: Lineage,
    pub(crate) deck: SelectedDeck,
    /// The main search's found-career anchors (`found_anchors`).
    pub(crate) found_anchors: Vec<Option<FoundAnchor>>,
    /// The bound's anchors per (candidate, partner), retained for later rolls; not stored by `state_out`.
    pub(crate) partner_anchors: Vec<PartnerAnchors>,
}

impl RollKeep {
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
/// the plan's own arithmetic in its order: the f64 the plan's full table held); where y < 0.5 the bound y_hi
/// (`outcome_hi` at that one cell). Rounded as the table was: y to 0.01, y_hi up to 0.01 (null where y >= 0.5)
pub(crate) fn roll_kept<'data>(
    env: &mut Env<'data>,
    worker: &mut KeptState,
    input: &JobInputs<'data>,
    legacies: &[Legacy],
    pool: &Pool,
    fixed_parents: &[usize],
    plan_job: &PlanJob,
) -> Option<Value> {
    let stars = plan_job.roll.as_ref()?;
    let key = roll_key(input.job);
    let index = worker.rolls.iter().position(|roll| roll.key == key)?;
    let mut kept = worker.rolls.remove(index);
    // (the roll's own timers and counters, `prof`; what the job counted before goes back after)
    let started = now_ms();
    let prof_before = PROF.with(|prof| std::mem::take(&mut *prof.borrow_mut()));
    let answer = match sim_for(env, worker, input, plan_job) {
        Ok(mut sim) => roll_answer(&mut sim, &RollPool { legacies, pool, fixed_parents }, &mut kept, stars),
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

/// The pool a roll's bound reads.
pub(super) struct RollPool<'roll> {
    /// Recorded veterans in parent-index order.
    legacies: &'roll [Legacy],
    pool: &'roll Pool,
    /// Parent indices required by the job.
    pub(super) fixed_parents: &'roll [usize],
}

impl RollPool<'_> {
    /// A scoring against `score_base` on this pool.
    pub(super) const fn scoring<'score>(
        &'score self,
        score_base: &'score ScoreBase<'score>,
    ) -> ScoringContext<'score, 'score> {
        ScoringContext { score_base, legacies: self.legacies, pool: self.pool }
    }
}

fn roll_answer(simulator: &mut Sim, roll_pool: &RollPool, kept: &mut RollKeep, stars: &[u8]) -> Value {
    let setup = &simulator.env.setup;
    if stars.len() != setup.target_count {
        return json!({"error": format!("roll: {} stars for {} targets", stars.len(), setup.target_count)});
    }
    let outcome = OutcomeIndex::from_stars(stars, &setup.uncertain_targets, &setup.fixed_targets);
    let score_base = kept.base.base();
    let y = kept.scored.roll(&score_base, outcome);
    let mut hi = None;
    if y < 0.5 {
        hi = Some(kept.outcome_hi(simulator, roll_pool, outcome, y));
    }
    // (y_hi >= y is derived, `outcome_hi` (1)-(3): checked here, within rounding, where the max below would hide a bound
    // that drifted under the estimate; the max stays for the page)
    debug_assert!(hi.is_none_or(|bound| bound >= y - 1e-9 * y.abs().max(1.0)), "roll: y_hi {hi:?} below y {y}");
    // (y_hi bounds the scoring's routes only: a route the scoring has not, another lineage of a candidate's whose C her
    // veteran improves without lifting her odds, can save careers below it, so the page confirms every roll)
    json!({"y": (y * 100.0).round() / 100.0, "y_hi": hi.map(|bound| (bound.max(y) * 100.0).ceil() / 100.0)})
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
        json!({"cands": base.candidates, "c1": fvec(&base.step_costs), "v0": hx(base.current_value), "pnow": fvec(&base.current_odds), "cert": base.certified,
            "cont": base.continuations.iter().map(|continuation| json!([hx(continuation.lam), hx(continuation.phi), hx(continuation.pinf), !continuation.tables.is_empty(), hx(continuation.anchor_odds)])).collect::<Vec<_>>(),
            "caps": base.step_caps.iter().map(|cap| cap.as_ref().map(|cap| cap_out(&cap.kept))).collect::<Vec<_>>()})
    };
    let scored_out = |scored: &Scored| {
        json!({"n": [scored.uncertain_target_count, scored.fixed_target_count, scored.target_count, scored.slot_count], "goal": scored.completes_goal, "w": fvec(&scored.star_mix), "uncv": scored.uncertain_targets, "fmm": scored.fixed_landed_masks,
            "x0": scored.misses_without_her.iter().map(|value| fvec(value)).collect::<Vec<_>>(), "xm": scored.miss_by_stars.iter().map(|value| fvec(value)).collect::<Vec<_>>(),
            "pw": scored.target_spark_scales.iter().map(|value| fvec(&value.iter().flatten().copied().collect::<Vec<f64>>())).collect::<Vec<_>>(),
            "tsi": scored.setup_by_table, "t0": fvec(&scored.initial_tables), "col": scored.collapsed_targets, "sslot": scored.slot_by_setup, "p0": fvec(&scored.current_probabilities), "os": fvec(&scored.landed_set_chances),
            "sc": scored.slot_candidates, "lc": scored.lift_candidates,
            "rt": scored.routes.iter().map(|route| json!([slot_out(route.lifts[0]), slot_out(route.lifts[1]), route.full_only, hx(route.fixed_cost), route.finisher_slot, building_out(route.while_building)])).collect::<Vec<_>>()})
    };
    Value::Array(
        rolls.iter()
            .map(|roll| {
                let lineage = roll.lineage;
                json!({
                    "k": ux(roll.key), "x": [lineage.trainee, lineage.first_parent, lineage.second_parent],
                    "sel": sel_out(&roll.deck),
                    "anc": roll.found_anchors.iter().map(|found_anchor| found_anchor.as_ref().map(|anchor| json!([anchor.deck_class, fvec(&anchor.inheritance_chances), hx(anchor.value)]))).collect::<Vec<_>>(),
                    "base": base_out(&roll.base),
                    "sd": scored_out(&roll.scored)})
            })
            .collect(),
    )
}

/// Kept roll states from the page's store (`state_in`); None refuses the store whole. (Absent in a state stored before
/// them: none.)
pub(crate) fn rolls_in(value: &Value) -> Option<Vec<RollKeep>> {
    use crate::career::state::{fx, sel_in, usz, uxr};
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
            let lineage = usize_vec(&entry["x"])?;
            let &[trainee, first_parent, second_parent] = lineage.as_slice() else { return None };
            let found_anchors = entry["anc"]
                .as_array()?
                .iter()
                .map(|stored_anchor| {
                    if stored_anchor.is_null() {
                        Some(None)
                    } else {
                        Some(Some(FoundAnchor {
                            deck_class: crate::uma::UmaIndex(usz(&stored_anchor[0])?),
                            inheritance_chances: crate::career::state::pvec_r(&stored_anchor[1])?,
                            value: fx(&stored_anchor[2])?,
                        }))
                    }
                })
                .collect::<Option<Vec<_>>>()?;
            // (the anchors' chances: one per job target)
            if found_anchors
                .iter()
                .flatten()
                .any(|stored_anchor| stored_anchor.inheritance_chances.len() != scored.target_count)
            {
                return None;
            }
            Some(RollKeep {
                key: uxr(&entry["k"])?,
                scored,
                base,
                lineage: Lineage { trainee: crate::uma::UmaIndex(trainee), first_parent, second_parent },
                deck: sel_in(&entry["sel"])?,
                found_anchors,
                partner_anchors: vec![],
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
    // (each candidate's cap or null, read at her odds now; absent: none)
    let step_caps = match stored["caps"].as_array() {
        Some(caps) => caps
            .iter()
            .enumerate()
            .map(|(candidate, cap)| {
                if cap.is_null() {
                    Some(None)
                } else {
                    Some(Some(StepCap::at(cap_in(cap)?, *current_odds.get(candidate)?)))
                }
            })
            .collect::<Option<Vec<_>>>()?,
        None => vec![],
    };
    Some(KeptBase {
        candidates: usize_vec(&stored["cands"])?.into_iter().map(crate::uma::UmaIndex).collect(),
        step_costs: fvec_r(&stored["c1"])?,
        current_value: fx(&stored["v0"])?,
        current_odds,
        certified: stored["cert"].as_array()?.iter().map(Value::as_bool).collect::<Option<_>>()?,
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
                finisher_slot: usz(&route[4])?,
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
        uncertain_targets: usize_vec(&stored["uncv"])?,
        fixed_landed_masks: usize_vec(&stored["fmm"])?.into_iter().map(TargetMask::from_bits).collect(),
        misses_without_her: stored["x0"].as_array()?.iter().map(fvec_r).collect::<Option<_>>()?,
        miss_by_stars: stored["xm"].as_array()?.iter().map(four).collect::<Option<_>>()?,
        target_spark_scales,
        setup_by_table: usize_vec(&stored["tsi"])?,
        initial_tables: fvec_r(&stored["t0"])?,
        collapsed_targets: stored["col"].as_array()?.iter().map(serde_json::Value::as_bool).collect::<Option<_>>()?,
        slot_by_setup: usize_vec(&stored["sslot"])?,
        current_probabilities: fvec_r(&stored["p0"])?,
        landed_set_chances: pvec_r(&stored["os"])?,
        routes,
        // (a roll state's base prices every C at its common scale, 1)
        finisher_umas: vec![],
        present_factors: vec![],
        absent_factors: vec![],
        spark_scales: vec![],
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
            && base.certified.len() == candidate_count
            && base.continuations.len() <= candidate_count
            && base.step_caps.len() <= candidate_count;
        // her setups, one slot each (`slot_count` and `slot_by_setup` the identity): the partner slots and the lift
        // slots (each at most MAX_PARTNERS per candidate, candidate indices; the partner slots in candidate order), then
        // one finisher slot per route (at most MAX_ROUTES routes); each route's lifts lift slots, its finisher and its C
        // while building (`Slot`) finisher slots
        let (partner_slot_count, lift_slot_count) = (self.slot_candidates.len(), self.lift_candidates.len());
        let fits_cands = partner_slot_count <= MAX_PARTNERS * candidate_count
            && lift_slot_count <= MAX_PARTNERS * candidate_count
            && self.slot_candidates.iter().chain(&self.lift_candidates).all(|&candidate| candidate < candidate_count)
            && self.slot_candidates.windows(2).all(|pair| pair[0] <= pair[1])
            && self.routes.len() <= crate::career::plan::step::MAX_ROUTES
            && setup_count == partner_slot_count + lift_slot_count + self.routes.len()
            && self.slot_count == setup_count
            && self.slot_by_setup.iter().enumerate().all(|(setup, &slot)| slot == setup)
            && self.routes.iter().all(|route| {
                route.lifts.iter().all(|&slot| slot == NO_SLOT || slot < lift_slot_count)
                    && route.finisher_slot < self.routes.len()
                    && match route.while_building {
                        WhileBuilding::Slot(slot) => slot < self.routes.len(),
                        _ => true,
                    }
            });
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
            && self.fixed_landed_masks == fm_masks(&fixed)
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
