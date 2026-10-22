//! The roll of the plan's pick and its kept state.
use crate::career::clock::{PROF, now_ms};
use crate::career::plan::anchors::{FoundAnchor, PartnerAnchors};
use crate::career::plan::cont::{Cont, OwnTrio, ScoreBase, Trio};
use crate::career::plan::score::fm_masks;
use crate::career::plan::score::kernel::{ActiveCandidate, Scored};
use crate::career::plan::search::{IDX_PARTNERS, sim_for};
use crate::career::plan::{CandidateIndex, OutcomeIndex, PlanJob, ScoringContext, TargetMask};
use crate::career::pool::Pool;
use crate::career::search::Found;
use crate::career::sim::Sim;
use crate::career::state::{KeptState, hash_text};
use crate::career::{Env, JobInputs, Legacy, Lineage};
use crate::deck::space::SelectedDeck;
use serde_json::{Value, json};
// ---------- a roll of the plan's pick, on demand (`next.roll`) ----------
/// The base a kept scoring was scored against (`ScoreBase` without the sides and C's umas, which only `her_trios` reads).
/// Per-candidate vectors follow `candidates`' order.
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
    trios: Vec<Trio>,
    /// Stages 2-3 per candidate (`ScoreBase::later_stage_costs`).
    later_stage_costs: Vec<f64>,
}

impl KeptBase {
    pub(crate) fn of(score_base: &ScoreBase) -> Self {
        Self {
            candidates: score_base.candidates.to_vec(),
            step_costs: score_base.step_costs.to_vec(),
            current_value: score_base.current_value,
            current_odds: score_base.current_odds.to_vec(),
            continuations: score_base.continuations.to_vec(),
            trios: score_base.trios.to_vec(),
            later_stage_costs: score_base.later_stage_costs.to_vec(),
        }
    }
    pub(super) fn base(&self) -> ScoreBase<'_> {
        ScoreBase {
            candidates: &self.candidates,
            step_costs: &self.step_costs,
            later_stage_costs: &self.later_stage_costs,
            current_value: self.current_value,
            current_odds: &self.current_odds,
            continuations: &self.continuations,
            trios: &self.trios,
            trio_sides: &[],
            finisher_umas: &[],
        }
    }
}

/// A plan's roll state (`next.roll`), kept per plan job (`roll_key`): the pick's last scoring (`Scored`) and the base it
/// was scored against, the pick's career, and what its bound reads beyond them (`outcome_hi`): her own full veteran's trio
/// sides at the plan's end (`pick_her`) and the main search's found-career anchors (`found_anchors`)
pub(crate) struct RollKeep {
    /// The plan job's roll key (`roll_key`).
    pub(crate) key: u64,
    pub(crate) scored: Scored,
    pub(crate) base: KeptBase,
    /// The pick's trainee and parents, and her deck.
    pub(crate) lineage: Lineage,
    pub(crate) deck: SelectedDeck,
    /// Her own full veteran's trio sides at the plan's end (`pick_her`).
    pub(crate) own_trios: Vec<OwnTrio>,
    /// The main search's found-career anchors (`found_anchors`).
    pub(crate) found_anchors: Vec<Option<FoundAnchor>>,
    /// The bound's anchors per (candidate, partner), retained for later rolls; not stored by `state_out`.
    pub(crate) partner_anchors: Vec<PartnerAnchors>,
}

impl RollKeep {
    /// The state is of this trainee's career on these parents, in this order.
    pub(crate) fn is_of(&self, trainee: crate::uma::UmaIndex, career: &Found) -> bool {
        self.lineage.trainee == trainee
            && self.lineage.first_parent == career.first_parent
            && self.lineage.second_parent == career.second_parent
    }
}

/// plans' roll states kept (the most recent last): the run's plan and the plan Compare makes with her added
const ROLL_KEEP: usize = 4;

/// a plan job's roll key: the whole job but its roll (`next.roll`) and the page's flags that change no plan
/// (`no_reports`, `compare`)
pub(crate) fn roll_key(job: &Value) -> u64 {
    hash_text(&crate::career::state::json_without(job, &["no_reports", "compare", "roll"], &["roll"]))
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
    json!({"y": (y * 100.0).round() / 100.0, "y_hi": hi.map(|bound| (bound.max(y) * 100.0).ceil() / 100.0)})
}

/// the kept roll states for the page to store (`state_out`), every float as its bits
pub(crate) fn rolls_out(rolls: &[RollKeep]) -> Value {
    use crate::career::state::{fvec, hx, sel_out, ux};
    let trios_out = |trios: &[OwnTrio]| {
        trios
            .iter()
            .map(|trio| json!([trio.candidate.0, trio.uses_existing, hx(trio.finisher_cost)]))
            .collect::<Vec<_>>()
    };
    let base_out = |base: &KeptBase| {
        json!({"cands": base.candidates, "c1": fvec(&base.step_costs), "v0": hx(base.current_value), "pnow": fvec(&base.current_odds), "m23": fvec(&base.later_stage_costs),
            "cont": base.continuations.iter().map(|continuation| json!([hx(continuation.lam), hx(continuation.phi), hx(continuation.pinf), !continuation.tables.is_empty()])).collect::<Vec<_>>(),
            "combos": base.trios.iter().map(|trio| json!([trio.candidate_a.0, trio.candidate_b.0, hx(trio.finisher_cost), trio.a_uses_existing, trio.b_uses_existing])).collect::<Vec<_>>()})
    };
    let scored_out = |scored: &Scored| {
        json!({"n": [scored.uncertain_target_count, scored.fixed_target_count, scored.target_count, scored.slot_count], "xi": scored.own_candidate, "goal": scored.completes_goal, "w": fvec(&scored.star_mix), "uncv": scored.uncertain_targets, "fmm": scored.fixed_landed_masks,
            "x0": scored.misses_without_her.iter().map(|value| fvec(value)).collect::<Vec<_>>(), "xm": scored.miss_by_stars.iter().map(|value| fvec(value)).collect::<Vec<_>>(),
            "pw": scored.target_spark_scales.iter().map(|value| fvec(&value.iter().flatten().copied().collect::<Vec<f64>>())).collect::<Vec<_>>(),
            "tsi": scored.setup_by_table, "t0": fvec(&scored.initial_tables), "col": scored.collapsed_targets, "sslot": scored.slot_by_setup, "p0": fvec(&scored.current_probabilities), "os": fvec(&scored.landed_set_chances),
            "act": scored.active_candidates.iter().map(|active| json!([active.candidate.0, active.slot, hx(active.threshold), hx(active.current_chance)])).collect::<Vec<_>>(),
            "her": trios_out(&scored.own_trios)})
    };
    Value::Array(
        rolls.iter()
            .map(|roll| {
                let lineage = roll.lineage;
                json!({
                    "k": ux(roll.key), "x": [lineage.trainee, lineage.first_parent, lineage.second_parent],
                    "sel": sel_out(&roll.deck), "her": trios_out(&roll.own_trios),
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
            // (the anchors' chances: one per job target; her trio sides at the plan's end: candidate indices)
            let own_trios = own_trios_in(&entry["her"])?;
            if found_anchors
                .iter()
                .flatten()
                .any(|stored_anchor| stored_anchor.inheritance_chances.len() != scored.target_count)
                || own_trios.iter().any(|trio| trio.candidate.0 >= base.candidates.len())
            {
                return None;
            }
            Some(RollKeep {
                key: uxr(&entry["k"])?,
                scored,
                base,
                lineage: Lineage { trainee: crate::uma::UmaIndex(trainee), first_parent, second_parent },
                deck: sel_in(&entry["sel"])?,
                own_trios,
                found_anchors,
                partner_anchors: vec![],
            })
        })
        .collect()
}

fn usize_vec(value: &Value) -> Option<Vec<usize>> {
    value.as_array()?.iter().map(crate::career::state::usz).collect()
}

fn own_trios_in(value: &Value) -> Option<Vec<OwnTrio>> {
    use crate::career::state::{fx, usz};
    value
        .as_array()?
        .iter()
        .map(|stored| {
            Some(OwnTrio {
                candidate: CandidateIndex(usz(&stored[0])?),
                uses_existing: stored[1].as_bool()?,
                finisher_cost: fx(&stored[2])?,
            })
        })
        .collect()
}

fn kept_base_in(stored: &Value) -> Option<KeptBase> {
    use crate::career::state::{fvec_r, fx, usz};
    let continuations = stored["cont"]
        .as_array()?
        .iter()
        .map(|continuation| {
            let (lam, phi, pinf) = (fx(&continuation[0])?, fx(&continuation[1])?, fx(&continuation[2])?);
            // (a Cont without its table has no building: `Cont::default`'s lam 0; one that reads as building is refused)
            Some(if continuation[3].as_bool()? {
                Cont::new(lam, phi, pinf)
            } else if lam > 0.0 && phi > 0.0 {
                return None;
            } else {
                Cont { lam, phi, pinf, ..Cont::default() }
            })
        })
        .collect::<Option<Vec<Cont>>>()?;
    let trios = stored["combos"]
        .as_array()?
        .iter()
        .map(|trio| {
            Some(Trio {
                candidate_a: CandidateIndex(usz(&trio[0])?),
                candidate_b: CandidateIndex(usz(&trio[1])?),
                finisher_cost: fx(&trio[2])?,
                a_uses_existing: trio[3].as_bool()?,
                b_uses_existing: trio[4].as_bool()?,
            })
        })
        .collect::<Option<Vec<Trio>>>()?;
    Some(KeptBase {
        candidates: usize_vec(&stored["cands"])?.into_iter().map(crate::uma::UmaIndex).collect(),
        step_costs: fvec_r(&stored["c1"])?,
        current_value: fx(&stored["v0"])?,
        current_odds: fvec_r(&stored["pnow"])?,
        continuations,
        trios,
        later_stage_costs: fvec_r(&stored["m23"])?,
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
    let active_candidates = stored["act"]
        .as_array()?
        .iter()
        .map(|active| {
            Some(ActiveCandidate {
                candidate: CandidateIndex(usz(&active[0])?),
                slot: if active[1].is_null() { None } else { Some(usz(&active[1])?) },
                threshold: fx(&active[2])?,
                current_chance: fx(&active[3])?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Scored {
        uncertain_target_count,
        fixed_target_count,
        target_count,
        slot_count,
        own_candidate: if stored["xi"].is_null() { None } else { Some(usz(&stored["xi"])?) },
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
        active_candidates,
        own_trios: own_trios_in(&stored["her"])?,
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
            && base.later_stage_costs.len() == candidate_count
            && base.continuations.len() <= candidate_count
            && base
                .trios
                .iter()
                .all(|trio| trio.candidate_a.0 < candidate_count && trio.candidate_b.0 < candidate_count);
        // her setups (at most IDX_PARTNERS per candidate), the candidates the loop lifts (those with a slot in slot
        // order, one per slot), her own candidate index and trio sides: candidate indices
        let fits_cands = setup_count <= IDX_PARTNERS * candidate_count
            && self.active_candidates.iter().filter_map(|active| active.slot).eq(0..self.slot_count)
            && self.active_candidates.iter().all(|active| active.candidate.0 < candidate_count)
            && self.own_candidate.is_none_or(|own| own < candidate_count)
            && self.own_trios.iter().all(|trio| trio.candidate.0 < candidate_count);
        fits_base
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
