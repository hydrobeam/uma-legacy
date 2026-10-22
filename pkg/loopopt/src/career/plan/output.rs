//! The assembled plan response.
use crate::career::clock::{PROF, now_ms};
use crate::career::plan::anchors::{found_anchors, pick_her};
use crate::career::plan::combine::PlanScores;
use crate::career::plan::kept::{BESTS, Bests, CachedSearch, bests_key};
use crate::career::plan::roll::{KeptBase, RollKeep, keep_roll, roll_kept, roll_key};
use crate::career::plan::rows::{Pick, PlanRow, RowOrigin, has_looped_parents, near_floor, pick_of};
use crate::career::plan::score::KeptScoring;
use crate::career::plan::{PlanCandidates, PlanContext, PlanProfile, PlanSearches, unordered_parents};
use crate::career::search::Found;
use crate::career::sim::Sim;
use serde_json::{Value, json};
/// The plan's response: its options, pick and trio, with the pick's roll state and the plan's searches kept for later
/// jobs (`RollKeep`, `Bests`)
pub(crate) fn assemble_plan_output<'data>(
    mut sim: Sim<'_, 'data>,
    context: &PlanContext<'_, 'data>,
    searches: PlanSearches,
    mut runs: Vec<PlanRow>,
    scores: &PlanScores,
    profile: &PlanProfile,
    buf: &mut Vec<f64>,
) -> Value {
    let PlanContext { input, candidates } = *context;
    let sj = sim.env.skill_job;
    let legacies = input.legacies;
    let pool = input.pool;
    let data = context.data();
    let is_runnable = |f: &Found| input.is_runnable(f);
    // a looped or nb row that repeats an earlier row's career (a builder's exact search can land on the same 8/8 pair; an
    // nb row on her looped one): left out, the pick moved to the earlier one (the same career, the same score)
    let repeated: Vec<Option<usize>> = (0..runs.len()).map(|i| (0..i).find(|&j| runs[j].repeats(&runs[i]))).collect();
    let dup: Vec<bool> = repeated.iter().map(Option::is_some).collect();
    let undup: Vec<usize> = repeated.iter().enumerate().map(|(i, earlier)| earlier.unwrap_or(i)).collect();
    let can_roll =
        runs.iter().any(|r| is_runnable(&r.career)) && scores.any_scored && scores.fit.current_value.is_finite();
    let roll_state = can_roll.then(|| pick_roll_state(&mut sim, context, &mut runs, scores, &undup, buf));
    // Release the simulator's caches before publishing kept searches and reporting options.
    let env = {
        let finished = sim;
        finished.env
    };
    let incomplete = keep_plan_searches(context, searches);
    // nothing scored (no trio: fewer than two candidates of different umas): the best odds is the one option and the pick
    // (no row that can be the pick, `is_runnable`: the same, the pick's main search's)
    if !runs.iter().any(|r| is_runnable(&r.career)) {
        for f in &input.pick_solved().found {
            runs.push(PlanRow::proxy(f.trainee, f.clone(), vec![], RowOrigin::Fallback, None));
        }
    }
    // pick: the highest score; within NEAR of it (the model's noise), the better odds of her own 8/8
    let total = |i: usize| runs[i].total();
    let Pick { row: pick, top } = pick_of(&runs, is_runnable, &undup);
    let near = |i: usize| is_runnable(&runs[i].career) && total(i) >= near_floor(top);
    let mut options: Vec<Value> = vec![];
    for (i, row) in runs.iter().enumerate() {
        let (x, f) = (row.trainee, &row.career);
        // (a builder with a borrowed parent under "later" is no option)
        if dup.get(i).copied().unwrap_or(false) || !is_runnable(f) {
            continue;
        }
        let looped = has_looped_parents(sj, legacies, f);
        let mut report = env.report(pool, f, false);
        let option = report.as_object_mut().unwrap();
        option.insert("role".into(), json!(row_role(candidates, scores, x, looped)));
        option.insert(
            "best_odds".into(),
            json!(
                x == candidates.pick_career.trainee
                    && unordered_parents(f) == unordered_parents(&candidates.pick_career)
            ),
        );
        option.insert("chosen".into(), json!(i == pick));
        // both parents 8/8s: her 8/8 here is a fully looped parent
        option.insert("looped".into(), json!(looped));
        // false: a builder the plan could not turn on, her career from her deck class's winning decks (no career search)
        option.insert("exact".into(), json!(row.exact || row.origin == RowOrigin::Fallback));
        // her career search was not complete (its work cap `search_work`, a band overflow or a deck past its class's cap):
        // the best it found, not proven her best
        if row.exact && incomplete.of(row.origin, x) {
            option.insert("proven".into(), json!(false));
        }
        option.insert(
            "stage_cost".into(),
            json!(candidates.trainees.iter().position(|&c| c == x).map(|ui| scores.fit.step_costs[ui])),
        );
        option.insert("index".into(), json!(row.net_saving()));
        option.insert("saving".into(), json!(row.net_saving().map(|v| v + 1.0)));
        option.insert("lift".into(), json!(row.scoring.as_ref().map(|scoring| scoring.improvement_chance)));
        option.insert("near".into(), json!(near(i) && i != pick));
        option.insert("score".into(), json!(scores.any_scored.then(|| total(i))));
        options.push(report);
    }
    options.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .unwrap_or(f64::NEG_INFINITY)
            .partial_cmp(&a["score"].as_f64().unwrap_or(f64::NEG_INFINITY))
            .unwrap()
    });
    let choice = if runs.is_empty() { Value::Null } else { env.report(pool, &runs[pick].career, true) };
    let (plan_json, members_json) = trio_report(context, scores);
    // the career a roll of this plan is of (`next.roll`, `roll_kept`: its saving per outcome, computed on demand from the
    // roll state kept here): the pick, her uncertain targets (`uncv`) and the scenario's guaranteed ones (`fixed`)
    let outcomes = match runs.get(pick) {
        Some(row) if row.scoring.is_some() => {
            json!({"trainee": data.umas[row.trainee].id, "parents": [row.career.first_parent, row.career.second_parent],
        "uncv": env.uncertain_targets, "fixed": env.fixed_targets})
        }
        _ => Value::Null,
    };
    if let Some(roll) = roll_state.filter(|roll| runs.get(pick).is_some_and(|row| roll.is_of(row.trainee, &row.career)))
    {
        keep_roll(roll);
    }
    let fit = &scores.fit;
    let fit_json: Vec<Value> = (0..candidates.trainees.len())
        .map(|ui| {
            let continuation = &fit.continuations[ui];
            json!([data.umas[candidates.trainees[ui]].id, continuation.lam, continuation.phi, continuation.pinf])
        })
        .collect();
    let umas_unproven =
        data.umas.indices().filter(|&u| incomplete.base[u] || incomplete.looped[u] || incomplete.no_borrow[u]).count();
    let current_value = if fit.current_value.is_finite() { json!(fit.current_value) } else { Value::Null };
    let mut out = json!({"choice": choice, "rule": "index", "mode": "trio", "v0": current_value, "outcomes": outcomes,
           "members": members_json,
           "v0_plan": plan_json, "fit": fit_json, "loop_members": candidates.loop_member_info, "loop_note": candidates.loop_member_note, "hint": candidates.hint, "umas_scored": profile.scored_runnable_rows, "umas_searched": runs.iter().filter(|r| r.exact).count() - profile.certified_proxies, "umas_certified": profile.certified_proxies, "umas_unproven": umas_unproven, "work": scores.work,
           "sur": input.settings.sur, "sur_diff": profile.surrogate_differences, "loop_ms": profile.loop_ms, "search_ms": profile.search_ms, "combo_ms": scores.combo_ms, "index_ms": scores.index_ms, "total_ms": now_ms() - profile.started_ms});
    // (the option reports moved in, not copied by `json!`: the object's keys are ordered, so the same text)
    out["options"] = Value::Array(options);
    out["work"]["prof"] = json!(PROF.with(|p| std::mem::take(&mut *p.borrow_mut())));
    // a roll sent with the plan job (`next.roll`) when the worker kept no roll state for it: answered from the state just
    // kept (none when nothing was scored)
    if input.settings.roll.is_some() {
        out["roll"] = match roll_kept(env, input.resources, legacies, pool, input.fixed_parents, input.settings) {
            Some(mut roll) => {
                roll["planned"] = json!(true);
                roll
            }
            None => json!({"error": "the plan has no pick to roll"}),
        };
    }
    out
}

/// The pick's roll state (`RollKeep`; the pick as `assemble_plan_output` takes it, with the same scores): its kept
/// scoring, taken out of its row, and base, and what its bound reads beyond them, as the plan ends (`pick_her`,
/// `found_anchors`). Only where a row can be the pick and rows were scored against a finite V
fn pick_roll_state(
    sim: &mut Sim,
    context: &PlanContext,
    runs: &mut [PlanRow],
    scores: &PlanScores,
    undup: &[usize],
    buf: &mut Vec<f64>,
) -> RollKeep {
    let input = context.input;
    let is_runnable = |f: &Found| input.is_runnable(f);
    let Pick { row: pick, .. } = pick_of(runs, is_runnable, undup);
    let base = scores.fit.score_base(context.candidates);
    let KeptScoring { scored, full_landing_below_threshold } =
        runs[pick].scoring.as_mut().unwrap().kept.take().expect("the pick's kept scoring was dropped");
    let row = &runs[pick];
    let own_trios = pick_her(sim, &context.scoring(&base), row.scored_career(), full_landing_below_threshold, buf);
    RollKeep {
        key: roll_key(input.resources.job),
        scored,
        base: KeptBase::of(&base),
        lineage: row.lineage(),
        // (the roll state outlives the plan's rows)
        deck: row.career.deck.clone(),
        own_trios,
        found_anchors: found_anchors(sim.env, input.pool, input.solved),
        partner_anchors: vec![],
    }
}

/// Per uma (`Data::umas`), whether a search behind her rows was incomplete, by the row's origin
struct IncompleteSearches {
    base: crate::uma::UmaTable<bool>,
    looped: crate::uma::UmaTable<bool>,
    no_borrow: crate::uma::UmaTable<bool>,
}

impl IncompleteSearches {
    fn of(&self, origin: RowOrigin, trainee: crate::uma::UmaIndex) -> bool {
        match origin {
            RowOrigin::Base => self.base[trainee],
            RowOrigin::Looped => self.looped[trainee],
            RowOrigin::NoBorrow => self.no_borrow[trainee],
            RowOrigin::Fallback => false,
        }
    }
}

/// This plan's searches kept for the next plans (`BESTS`, at most four), with the kept plan of the pool less its last
/// veteran put back; what stays for the output is which searches were incomplete
fn keep_plan_searches(context: &PlanContext, searches: PlanSearches) -> IncompleteSearches {
    let input = context.input;
    let key = bests_key(input.resources.job, input.fixed_parents, context.legacy_count());
    let mut kept_best = searches.best_careers.clone();
    // The main search's best is per job, searched again (cheap) next time.
    kept_best[context.candidates.main_career.trainee] = CachedSearch::Unsearched;
    BESTS.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain(|plan| plan.key != key);
        cache.push(Bests {
            key,
            best_careers: kept_best,
            proof_refs: searches.proof_refs,
            looped_careers: searches.looped_careers,
            base_incomplete: searches.base_incomplete.clone(),
            looped_incomplete: searches.looped_incomplete.clone(),
        });
        if let Some(previous) = searches.kept_previous_pool {
            cache.push(previous);
        }
        while cache.len() > 4 {
            cache.remove(0);
        }
    });
    IncompleteSearches {
        base: searches.base_incomplete,
        looped: searches.looped_incomplete,
        no_borrow: searches.no_borrow_incomplete,
    }
}

/// A career's role in the plan: one of the best trio's A and B, its finisher (C on A's and B's real 8/8s, so only on two
/// 8/8 parents), else a loop member, a candidate or a builder
fn row_role(
    candidates: &PlanCandidates,
    scores: &PlanScores,
    trainee: crate::uma::UmaIndex,
    looped: bool,
) -> &'static str {
    match scores.best_combo {
        Some(combination)
            if trainee == candidates[combination.candidate_a_index]
                || trainee == candidates[combination.candidate_b_index] =>
        {
            "plan"
        }
        Some(combination)
            if trainee == combination.finisher_trainee
                && combination.a_uses_existing
                && combination.b_uses_existing
                && looped =>
        {
            "finish"
        }
        _ => {
            if candidates.loop_members.contains(&trainee) {
                "loop"
            } else if candidates.trainees.contains(&trainee) {
                "candidate"
            } else {
                "builder"
            }
        }
    }
}

/// The best trio as the page shows it (`v0_plan`: A, B, C and their careers, the cheaper of A's and B's steps first)
/// and its two members' continuations (`members`)
fn trio_report(context: &PlanContext, scores: &PlanScores) -> (Value, Value) {
    let candidates = context.candidates;
    let data = context.data();
    let Some(combination) = scores.best_combo else { return (Value::Null, json!([])) };
    let uma_name = |u: crate::uma::UmaIndex| if u == crate::NONE { Value::Null } else { json!(data.umas[u].name) };
    // a side's step: free from a real 8/8 of hers, else her fresh 8/8's step
    let step = |ui: usize, uses_existing: bool| if uses_existing { 0.0 } else { scores.fit.step_costs[ui] };
    // a member: `real` when the trio uses a real 8/8 of hers (her step free), else her fresh 8/8's step
    let member = |ui: usize, uses_existing: bool| {
        let continuation = &scores.fit.continuations[ui];
        let build_plan = continuation.plan(candidates.current_odds[ui]);
        json!({
            "trainee": uma_name(candidates.trainees[ui]),
            "p": candidates.current_odds[ui],
            "have8": candidates.has_full_veteran[ui],
            "real": uses_existing,
            "p_inf": (continuation.lam > 0.0).then_some(continuation.pinf),
            "lam": continuation.lam,
            "phi": continuation.phi,
            "builds": build_plan.builds,
            "p_switch": build_plan.grind_odds,
            "careers": step(ui, uses_existing)
        })
    };
    // A: the cheaper of the two steps first
    let a = (combination.candidate_a_index, combination.a_uses_existing);
    let b = (combination.candidate_b_index, combination.b_uses_existing);
    let ((first_index, first_uses_existing), (second_index, second_uses_existing)) =
        if step(a.0.0, a.1) <= step(b.0.0, b.1) { (a, b) } else { (b, a) };
    (
        json!({"a": uma_name(candidates[first_index]), "b": uma_name(candidates[second_index]), "c": uma_name(combination.finisher_trainee), "careers": [step(first_index.0, first_uses_existing), step(second_index.0, second_uses_existing), combination.finisher_cost]}),
        json!([member(first_index.0, first_uses_existing), member(second_index.0, second_uses_existing)]),
    )
}
