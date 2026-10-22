//! The roll of the plan's pick and its kept state.
use crate::career::clock::{PROF, now_ms};
use crate::career::plan::anchors::{FoundAnchor, OutcomeBounds, OutcomeEstimate, PartnerAnchors, outcome_hi};
use crate::career::plan::cont::{Cont, OwnTrio, ScoreBase, Trio};
use crate::career::plan::score::fm_masks;
use crate::career::plan::score::kernel::{ActiveCandidate, Scored};
use crate::career::plan::search::{IDX_PARTNERS, sim_for};
use crate::career::plan::{CandidateIndex, OutcomeIndex, PlanJob, ScoringContext, TargetMask};
use crate::career::pool::Pool;
use crate::career::search::Found;
use crate::career::sim::Sim;
use crate::career::state::setup_hash;
use crate::career::{Career, Env, JobInputs, Legacy, Lineage};
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
    fn base(&self) -> ScoreBase<'_> {
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
/// was scored against, the pick's career, and what its bound reads beyond them (`outcome_hi`): her own 8/8's trio sides as
/// the plan's end has them (`pick_her`) and the main search's found-career anchors (`found_anchors`)
pub(crate) struct RollKeep {
    /// The plan job's roll key (`roll_key`).
    pub(crate) key: u64,
    pub(crate) scored: Scored,
    pub(crate) base: KeptBase,
    /// The pick's trainee and parents, and her deck.
    pub(crate) lineage: Lineage,
    pub(crate) deck: SelectedDeck,
    /// Her own 8/8's trio sides at the plan's end (`pick_her`).
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

thread_local! { static ROLLS: std::cell::RefCell<Vec<RollKeep>> = const { std::cell::RefCell::new(Vec::new()) }; }

/// plans' roll states kept (the most recent last): the run's plan and the plan Compare makes with her added
const ROLL_KEEP: usize = 4;

/// a plan job's roll key: the whole job but its roll (`next.roll`) and the page's flags that change no plan
/// (`no_reports`, `compare`)
pub(crate) fn roll_key(job: &Value) -> u64 {
    setup_hash(&crate::career::state::json_without(job, &["no_reports", "compare", "roll"], &["roll"]))
}

pub(crate) fn keep_roll(roll: RollKeep) {
    ROLLS.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain(|entry| entry.key != roll.key);
        cache.push(roll);
        while cache.len() > ROLL_KEEP {
            cache.remove(0);
        }
    });
}

/// The plan's answer to a roll (`next.roll`: her stars per job target, 0 where it did not land) from its kept roll state,
/// with no search (None when the worker keeps none for this plan job: then the plan runs and answers it, `index_rule`).
/// y: the careers her veteran saves at that outcome, the pick's kept scoring evaluated at that one outcome (`Scored::evaluate`,
/// the plan's own arithmetic in its order: the f64 the plan's full table held); where y < 0.5 the bound y_hi
/// (`outcome_hi` at that one cell). Rounded as the table was: y to 0.01, y_hi up to 0.01 (null where y >= 0.5)
pub(crate) fn roll_kept<'data>(
    env: &mut Env<'data>,
    input: &JobInputs<'data>,
    legs: &[Legacy],
    pool: &Pool,
    fix: &[usize],
    pj: &PlanJob,
) -> Option<Value> {
    let stars = pj.roll.as_ref()?;
    let key = roll_key(input.job);
    let mut kept = ROLLS.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|roll| roll.key == key)?;
        Some(cache.remove(index))
    })?;
    // (the roll's own timers and counters, `prof`; what the job counted before goes back after)
    let started = now_ms();
    let prof_before = PROF.with(|prof| std::mem::take(&mut *prof.borrow_mut()));
    let answer = match sim_for(env, input, pj) {
        Ok(mut sim) => roll_answer(&mut sim, &RollPool { legacies: legs, pool, fixed_parents: fix }, &mut kept, stars),
        Err(error) => error,
    };
    keep_roll(kept);
    let mut answer = answer;
    let roll_prof = PROF.with(|prof| std::mem::replace(&mut *prof.borrow_mut(), prof_before));
    if answer.is_object() {
        answer["ms"] = json!(now_ms() - started);
        answer["prof"] = json!(roll_prof);
    }
    Some(answer)
}

/// The pool a roll's bound reads.
struct RollPool<'roll> {
    /// Recorded veterans in parent-index order.
    legacies: &'roll [Legacy],
    pool: &'roll Pool,
    /// Parent indices required by the job.
    fixed_parents: &'roll [usize],
}

impl RollPool<'_> {
    /// A scoring against `score_base` on this pool.
    const fn scoring<'score>(&'score self, score_base: &'score ScoreBase<'score>) -> ScoringContext<'score, 'score> {
        ScoringContext { score_base, legacies: self.legacies, pool: self.pool }
    }
}

fn roll_answer(simulator: &mut Sim, roll_pool: &RollPool, kept: &mut RollKeep, stars: &[u8]) -> Value {
    let (uncv, fixed_t) = (simulator.env.uncertain_targets.clone(), simulator.env.fixed_targets.clone());
    if stars.len() != simulator.env.target_count {
        return json!({"error": format!("roll: {} stars for {} targets", stars.len(), simulator.env.target_count)});
    }
    let outcome = OutcomeIndex::from_stars(stars, &uncv, &fixed_t);
    let mut partner_anchors = std::mem::take(&mut kept.partner_anchors);
    let score_base = kept.base.base();
    let y = kept.scored.roll(&score_base, outcome);
    let mut hi = None;
    if y < 0.5 {
        let context = roll_pool.scoring(&score_base);
        let career = Career { lineage: kept.lineage, deck: &kept.deck };
        let bounds = OutcomeBounds {
            fixed_parents: roll_pool.fixed_parents,
            found_anchors: &kept.found_anchors,
            own_trios: &kept.own_trios,
        };
        let estimate = OutcomeEstimate { index: outcome, saving: y };
        hi = Some(outcome_hi(simulator, &context, career, &bounds, estimate, &mut partner_anchors));
    }
    kept.partner_anchors = partner_anchors;
    // (y_hi >= y is derived, `outcome_hi` (1)-(3): checked here, within rounding, where the max below would hide a bound
    // that drifted under the estimate; the max stays for the page)
    debug_assert!(hi.is_none_or(|h| h >= y - 1e-9 * y.abs().max(1.0)), "roll: y_hi {hi:?} below y {y}");
    json!({"y": (y * 100.0).round() / 100.0, "y_hi": hi.map(|h| (h.max(y) * 100.0).ceil() / 100.0)})
}

/// the kept roll states for the page to store (`state_out`), every float as its bits
pub(crate) fn rolls_out() -> Value {
    use crate::career::state::{fvec, hx, sel_out, ux};
    let trios_out = |trios: &[OwnTrio]| {
        trios
            .iter()
            .map(|trio| json!([trio.candidate.0, trio.uses_existing, hx(trio.finisher_cost)]))
            .collect::<Vec<_>>()
    };
    let base_out = |base: &KeptBase| {
        json!({"cands": base.candidates, "c1": fvec(&base.step_costs), "v0": hx(base.current_value), "pnow": fvec(&base.current_odds), "m23": fvec(&base.later_stage_costs),
            "cont": base.continuations.iter().map(|c| json!([hx(c.lam), hx(c.phi), hx(c.pinf), !c.tab.is_empty()])).collect::<Vec<_>>(),
            "combos": base.trios.iter().map(|trio| json!([trio.candidate_a.0, trio.candidate_b.0, hx(trio.finisher_cost), trio.a_uses_existing, trio.b_uses_existing])).collect::<Vec<_>>()})
    };
    let scored_out = |sd: &Scored| {
        json!({"n": [sd.uncertain_target_count, sd.fixed_target_count, sd.target_count, sd.slot_count], "xi": sd.own_candidate, "goal": sd.completes_goal, "w": fvec(&sd.star_mix), "uncv": sd.uncertain_targets, "fmm": sd.fixed_landed_masks,
            "x0": sd.misses_without_her.iter().map(|v| fvec(v)).collect::<Vec<_>>(), "xm": sd.miss_by_stars.iter().map(|v| fvec(v)).collect::<Vec<_>>(),
            "pw": sd.target_spark_scales.iter().map(|v| fvec(&v.iter().flatten().copied().collect::<Vec<f64>>())).collect::<Vec<_>>(),
            "tsi": sd.setup_by_table, "t0": fvec(&sd.initial_tables), "col": sd.collapsed_targets, "sslot": sd.slot_by_setup, "p0": fvec(&sd.current_probabilities), "os": fvec(&sd.landed_set_chances),
            "act": sd.active_candidates.iter().map(|active| json!([active.candidate.0, active.slot, hx(active.threshold), hx(active.current_chance)])).collect::<Vec<_>>(),
            "her": trios_out(&sd.own_trios)})
    };
    ROLLS.with(|c| {
        Value::Array(
            c.borrow()
                .iter()
                .map(|roll| {
                    let lineage = roll.lineage;
                    json!({
                        "k": ux(roll.key), "x": [lineage.trainee, lineage.first_parent, lineage.second_parent],
                        "sel": sel_out(&roll.deck), "her": trios_out(&roll.own_trios),
                        "anc": roll.found_anchors.iter().map(|a| a.as_ref().map(|anchor| json!([anchor.deck_class, fvec(&anchor.inheritance_chances), hx(anchor.value)]))).collect::<Vec<_>>(),
                        "base": base_out(&roll.base),
                        "sd": scored_out(&roll.scored)})
                })
                .collect(),
        )
    })
}

/// Kept roll states from the page's store (`state_in`); None refuses the store whole. (Absent in a state stored before
/// them: none.)
pub(crate) fn rolls_in(v: &Value) -> Option<Vec<RollKeep>> {
    use crate::career::state::{fx, sel_in, usz, uxr};
    let Some(stored) = v.as_array() else { return v.is_null().then(Vec::new) };
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
                .map(|a| {
                    if a.is_null() {
                        Some(None)
                    } else {
                        Some(Some(FoundAnchor {
                            deck_class: crate::uma::UmaIndex(usz(&a[0])?),
                            inheritance_chances: crate::career::state::pvec_r(&a[1])?,
                            value: fx(&a[2])?,
                        }))
                    }
                })
                .collect::<Option<Vec<_>>>()?;
            // (the anchors' chances: one per job target; her trio sides at the plan's end: candidate indices)
            let own_trios = own_trios_in(&entry["her"])?;
            if found_anchors.iter().flatten().any(|a| a.inheritance_chances.len() != scored.target_count)
                || own_trios.iter().any(|h| h.candidate.0 >= base.candidates.len())
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

fn usize_vec(v: &Value) -> Option<Vec<usize>> {
    v.as_array()?.iter().map(crate::career::state::usz).collect()
}

fn own_trios_in(v: &Value) -> Option<Vec<OwnTrio>> {
    use crate::career::state::{fx, usz};
    v.as_array()?
        .iter()
        .map(|t| {
            Some(OwnTrio {
                candidate: CandidateIndex(usz(&t[0])?),
                uses_existing: t[1].as_bool()?,
                finisher_cost: fx(&t[2])?,
            })
        })
        .collect()
}

fn kept_base_in(b: &Value) -> Option<KeptBase> {
    use crate::career::state::{fvec_r, fx, usz};
    let continuations = b["cont"]
        .as_array()?
        .iter()
        .map(|c| {
            let (lam, phi, pinf) = (fx(&c[0])?, fx(&c[1])?, fx(&c[2])?);
            // (a Cont without its table has no building: `Cont::default`'s lam 0; one that reads as building is refused)
            Some(if c[3].as_bool()? {
                Cont::new(lam, phi, pinf)
            } else if lam > 0.0 && phi > 0.0 {
                return None;
            } else {
                Cont { lam, phi, pinf, ..Cont::default() }
            })
        })
        .collect::<Option<Vec<Cont>>>()?;
    let trios = b["combos"]
        .as_array()?
        .iter()
        .map(|t| {
            Some(Trio {
                candidate_a: CandidateIndex(usz(&t[0])?),
                candidate_b: CandidateIndex(usz(&t[1])?),
                finisher_cost: fx(&t[2])?,
                a_uses_existing: t[3].as_bool()?,
                b_uses_existing: t[4].as_bool()?,
            })
        })
        .collect::<Option<Vec<Trio>>>()?;
    Some(KeptBase {
        candidates: usize_vec(&b["cands"])?.into_iter().map(crate::uma::UmaIndex).collect(),
        step_costs: fvec_r(&b["c1"])?,
        current_value: fx(&b["v0"])?,
        current_odds: fvec_r(&b["pnow"])?,
        continuations,
        trios,
        later_stage_costs: fvec_r(&b["m23"])?,
    })
}

/// A stored `Scored`, its derived tables left empty (`derive`)
fn scored_in(s: &Value) -> Option<Scored> {
    use crate::career::state::{fvec_r, fx, pvec_r, usz};
    let sizes = usize_vec(&s["n"])?;
    // (the plan's limits, `sim_for`: at most 12 targets, 10 of them uncertain, the rest guaranteed; checked before
    // any table is sized by them)
    let &[nu, nf, ntar, na] = sizes.as_slice() else { return None };
    if ntar > 12 || nu > 10 || nf > 12 || nu + nf != ntar {
        return None;
    }
    let four = |v: &Value| -> Option<[f64; 4]> {
        let x = fvec_r(v)?;
        x.try_into().ok()
    };
    let w: [f64; 3] = pvec_r(&s["w"])?.try_into().ok()?;
    let pw = s["pw"]
        .as_array()?
        .iter()
        .map(|v| {
            let f = fvec_r(v)?;
            (f.len() == 2 * ntar).then(|| f.chunks(2).map(|c| [c[0], c[1]]).collect())
        })
        .collect::<Option<Vec<Vec<[f64; 2]>>>>()?;
    let act = s["act"]
        .as_array()?
        .iter()
        .map(|t| {
            Some(ActiveCandidate {
                candidate: CandidateIndex(usz(&t[0])?),
                slot: if t[1].is_null() { None } else { Some(usz(&t[1])?) },
                threshold: fx(&t[2])?,
                current_chance: fx(&t[3])?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Scored {
        uncertain_target_count: nu,
        fixed_target_count: nf,
        target_count: ntar,
        slot_count: na,
        own_candidate: if s["xi"].is_null() { None } else { Some(usz(&s["xi"])?) },
        completes_goal: s["goal"].as_bool()?,
        star_mix: w,
        uncertain_targets: usize_vec(&s["uncv"])?,
        fixed_landed_masks: usize_vec(&s["fmm"])?.into_iter().map(TargetMask::from_bits).collect(),
        misses_without_her: s["x0"].as_array()?.iter().map(fvec_r).collect::<Option<_>>()?,
        miss_by_stars: s["xm"].as_array()?.iter().map(four).collect::<Option<_>>()?,
        target_spark_scales: pw,
        setup_by_table: usize_vec(&s["tsi"])?,
        initial_tables: fvec_r(&s["t0"])?,
        collapsed_targets: s["col"].as_array()?.iter().map(serde_json::Value::as_bool).collect::<Option<_>>()?,
        slot_by_setup: usize_vec(&s["sslot"])?,
        current_probabilities: fvec_r(&s["p0"])?,
        landed_set_chances: pvec_r(&s["os"])?,
        active_candidates: act,
        own_trios: own_trios_in(&s["her"])?,
        present_factors: vec![],
        absent_factors: vec![],
        spark_scales: vec![],
    })
}

impl Scored {
    /// A stored scoring and its base fit together: the vectors' lengths, and every index within what it indexes
    fn fits(&self, base: &KeptBase) -> bool {
        let (ns, nc) = (self.misses_without_her.len(), base.candidates.len());
        let fixed: Vec<usize> = (0..self.target_count).filter(|i| !self.uncertain_targets.contains(i)).collect();
        let fits_base = nc <= crate::career::state::STATE_MAX_N
            && base.step_costs.len() == nc
            && base.current_odds.len() == nc
            && base.later_stage_costs.len() == nc
            && base.continuations.len() <= nc
            && base.trios.iter().all(|t| t.candidate_a.0 < nc && t.candidate_b.0 < nc);
        // her setups (at most IDX_PARTNERS per candidate), the candidates the loop lifts (those with a slot in slot
        // order, one per slot), her own candidate index and trio sides: candidate indices
        let fits_cands = ns <= IDX_PARTNERS * nc
            && self.active_candidates.iter().filter_map(|a| a.slot).eq(0..self.slot_count)
            && self.active_candidates.iter().all(|a| a.candidate.0 < nc)
            && self.own_candidate.is_none_or(|x| x < nc)
            && self.own_trios.iter().all(|h| h.candidate.0 < nc);
        fits_base
            && fits_cands
            && self.uncertain_targets.windows(2).all(|p| p[0] < p[1])
            && !self.uncertain_targets.last().is_some_and(|&i| i >= self.target_count)
            && self.fixed_landed_masks == fm_masks(&fixed)
            && !self.slot_by_setup.windows(2).any(|p| p[0] > p[1])
            && self.uncertain_targets.len() == self.uncertain_target_count
            && self.collapsed_targets.len() == self.uncertain_target_count
            && self.miss_by_stars.len() == ns
            && self.target_spark_scales.len() == ns
            && self.slot_by_setup.len() == ns
            && self.landed_set_chances.len() == 1usize << self.target_count
            && self.fixed_landed_masks.len() == 1usize << self.fixed_target_count
            && self.initial_tables.len() == self.setup_by_table.len() << self.uncertain_target_count
            && !self.misses_without_her.iter().any(|v| v.len() != self.uncertain_target_count)
            && !self.setup_by_table.iter().any(|&si| si >= ns)
            && !self.slot_by_setup.iter().any(|&sl| sl >= self.slot_count)
            && self.current_probabilities.len() == self.slot_count
    }
}

pub(crate) fn rolls_set(r: Vec<RollKeep>) {
    ROLLS.with(|c| *c.borrow_mut() = r);
}
