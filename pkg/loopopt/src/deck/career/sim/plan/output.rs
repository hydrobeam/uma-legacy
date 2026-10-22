//! The assembled plan response.
use super::{
    BESTS, Bests, CachedSearch, KeptBase, KeptScoring, PlanCandidates, PlanInputs, PlanProfile, PlanRow, PlanScores,
    PlanSearches, RollKeep, RowOrigin, bests_key, found_anchors, is_full, keep_roll, near_floor, pick_her, pick_of,
    roll_kept, roll_key,
};
use crate::deck::career::sim::plan::ScoringContext;
use crate::deck::career::sim::{Found, PROF, Sim, Value, json, now_ms};
use crate::deck::career::{Career, Lineage};

pub(in crate::deck::career::sim::plan) fn assemble_plan_output<'data>(
    mut s: Sim<'_, 'data>,
    input: &PlanInputs<'_, 'data>,
    candidates: &PlanCandidates,
    searches: PlanSearches,
    mut runs: Vec<PlanRow>,
    scores: &PlanScores,
    profile: &PlanProfile,
    proxy_check: Value,
    buf: &mut Vec<f64>,
) -> Value {
    let sj = s.env.sj;
    let PlanInputs { legacies: legs, pool, solved: sv, fixed_parents: fix, settings: pj, .. } = *input;
    let d = input.resources.data;
    let job = input.resources.job;
    let psv = input.pick_solved();
    let n = candidates.trainees.len();
    let nl = legs.len();
    let is_runnable = |f: &Found| input.is_runnable(f);
    let looped = |f: &Found| is_full(sj, &legs[f.a].sparks) && is_full(sj, &legs[f.b].sparks);
    let step = |ui: usize, real: bool| if real { 0.0 } else { scores.fit.step_costs[ui] };
    // a looped or nb row that repeats an earlier row's career (a builder's exact search can land on the same 8/8 pair; an
    // nb row on her looped one): left out, the pick moved to the earlier one (the same career, the same score)
    let pair = |f: &Found| (f.a.min(f.b), f.a.max(f.b));
    let dup: Vec<bool> = (0..runs.len())
        .map(|i| (0..i).any(|j| runs[j].trainee == runs[i].trainee && pair(&runs[j].career) == pair(&runs[i].career)))
        .collect();
    let undup: Vec<usize> = (0..runs.len())
        .map(|i| {
            if dup[i] {
                (0..i)
                    .find(|&j| runs[j].trainee == runs[i].trainee && pair(&runs[j].career) == pair(&runs[i].career))
                    .unwrap()
            } else {
                i
            }
        })
        .collect();
    // the pick's roll state (`RollKeep`; the pick as below, with the same scores): its kept scoring and base, and what its
    // bound reads beyond them, as the plan ends (`pick_her`, `found_anchors`)
    let rk: Option<RollKeep> =
        (runs.iter().any(|r| is_runnable(&r.career)) && scores.any_scored && scores.fit.current_value.is_finite())
            .then(|| {
                let (pk, _) = pick_of(&runs, is_runnable, &undup);
                let base = scores.fit.score_base(candidates);
                let (x, f) = (runs[pk].trainee, runs[pk].career.clone());
                let KeptScoring { scored: sd, full_landing_below_threshold: full_lo } =
                    runs[pk].scoring.as_mut().unwrap().kept.take().expect("the pick's kept scoring was dropped");
                let her_hi = pick_her(
                    &mut s,
                    &ScoringContext { score_base: &base, legacies: legs, pool },
                    Career { lineage: Lineage { trainee: x, first_parent: f.a, second_parent: f.b }, deck: &f.sel },
                    full_lo,
                    buf,
                );
                RollKeep {
                    key: roll_key(job),
                    sd,
                    base: KeptBase::of(&base),
                    x,
                    a: f.a,
                    b: f.b,
                    sel: f.sel,
                    her_hi,
                    anc: found_anchors(s.env, pool, sv),
                    hi_anc: vec![],
                }
            });
    // Release the simulator's caches before publishing kept searches and reporting options.
    let env = {
        let finished = s;
        finished.env
    };
    {
        let key = bests_key(job, fix, nl);
        let mut kept_best = searches.best.clone();
        // The main search's best is per job, searched again (cheap) next time.
        kept_best[candidates.main_career.t] = CachedSearch::Unsearched;
        BESTS.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.retain(|plan| plan.key != key);
            cache.push(Bests {
                key,
                best: kept_best,
                refs: searches.proof_refs,
                looped: searches.looped_careers,
                unp: searches.base_incomplete.clone(),
                lunp: searches.looped_incomplete.clone(),
            });
            if let Some(previous) = searches.kept_previous_pool {
                cache.push(previous);
            }
            while cache.len() > 4 {
                cache.remove(0);
            }
        });
    }
    // nothing scored (no trio: fewer than two candidates of different umas): the best odds is the one option and the pick
    // (no row that can be the pick, `is_runnable`: the same, the pick's main search's)
    if !runs.iter().any(|r| is_runnable(&r.career)) {
        for f in &psv.found {
            runs.push(PlanRow::new(f.t, f.clone(), vec![], RowOrigin::Fallback, false, None));
        }
    }
    // pick: the highest score; within NEAR of it (the model's noise), the better odds of her own 8/8
    let total = |i: usize| runs[i].total();
    let (pick, top) = pick_of(&runs, is_runnable, &undup);
    let near = |i: usize| is_runnable(&runs[i].career) && total(i) >= near_floor(top);
    // per career: the trios' last step (C on A's and B's real 8/8s) only on two 8/8 parents
    let role = |u: usize, f: &Found| match scores.best_combo {
        Some(combination)
            if u == candidates[combination.candidate_a_index] || u == candidates[combination.candidate_b_index] =>
        {
            "plan"
        }
        Some(combination)
            if u == combination.finisher_trainee
                && combination.a_uses_existing
                && combination.b_uses_existing
                && looped(f) =>
        {
            "finish"
        }
        _ => {
            if candidates.members.contains(&u) {
                "loop"
            } else if candidates.trainees.contains(&u) {
                "candidate"
            } else {
                "builder"
            }
        }
    };
    let mut opts: Vec<Value> = vec![];
    for (i, row) in runs.iter().enumerate() {
        let (x, f) = (&row.trainee, &row.career);
        // (a builder with a borrowed parent under "later" is no option)
        if dup.get(i).copied().unwrap_or(false) || !is_runnable(f) {
            continue;
        }
        let mut rp = env.report(pool, f, false);
        let o = rp.as_object_mut().unwrap();
        o.insert("role".into(), json!(role(*x, f)));
        o.insert("best_odds".into(), json!(*x == candidates.pick_career.t && pair(f) == pair(&candidates.pick_career)));
        o.insert("chosen".into(), json!(i == pick));
        // both parents 8/8s: her 8/8 here is a fully looped parent
        o.insert("looped".into(), json!(looped(f)));
        // false: a builder the plan could not turn on, her career from her deck class's winning decks (no career search)
        o.insert("exact".into(), json!(row.exact || row.origin == RowOrigin::Fallback));
        // her career search was not complete (its work cap `search_work`, a band overflow or a deck past its class's cap):
        // the best it found, not proven her best
        if row.exact
            && match row.origin {
                RowOrigin::Base => searches.base_incomplete[*x],
                RowOrigin::Looped => searches.looped_incomplete[*x],
                RowOrigin::NoBorrow => searches.no_borrow_incomplete[*x],
                RowOrigin::Fallback => false,
            }
        {
            o.insert("proven".into(), json!(false));
        }
        o.insert(
            "stage_cost".into(),
            json!(candidates.trainees.iter().position(|&c| c == *x).map(|ui| scores.fit.step_costs[ui])),
        );
        o.insert("index".into(), json!(row.net_saving()));
        o.insert("saving".into(), json!(row.net_saving().map(|v| v + 1.0)));
        o.insert("lift".into(), json!(row.scoring.as_ref().map(|scoring| scoring.improvement_chance)));
        o.insert("near".into(), json!(near(i) && i != pick));
        o.insert("score".into(), json!(scores.any_scored.then(|| total(i))));
        opts.push(rp);
    }
    opts.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .unwrap_or(f64::NEG_INFINITY)
            .partial_cmp(&a["score"].as_f64().unwrap_or(f64::NEG_INFINITY))
            .unwrap()
    });
    let choice = if runs.is_empty() { Value::Null } else { env.report(pool, &runs[pick].career, true) };
    let uname = |u: usize| if u == usize::MAX { Value::Null } else { json!(d.umas[u].name) };
    // a member: `real` when the trio uses a real 8/8 of hers (her step free), else her fresh 8/8's step
    let member = |ui: usize, real: bool| {
        let continuation = &scores.fit.continuations[ui];
        let (_, builds, switch_chance) = continuation.plan(candidates.pnow[ui]);
        json!({
            "trainee": uname(candidates.trainees[ui]),
            "p": candidates.pnow[ui],
            "have8": candidates.have8[ui],
            "real": real,
            "p_inf": (continuation.lam > 0.0).then_some(continuation.pinf),
            "lam": continuation.lam,
            "phi": continuation.phi,
            "builds": builds,
            "p_switch": switch_chance,
            "careers": step(ui, real)
        })
    };
    let (plan_json, members_json) = match scores.best_combo {
        Some(combination) => {
            // A: the cheaper of the two steps first
            let a = (combination.candidate_a_index, combination.a_uses_existing);
            let b = (combination.candidate_b_index, combination.b_uses_existing);
            let ((first_index, first_uses_existing), (second_index, second_uses_existing)) =
                if step(a.0.0, a.1) <= step(b.0.0, b.1) { (a, b) } else { (b, a) };
            (
                json!({"a": uname(candidates[first_index]), "b": uname(candidates[second_index]), "c": uname(combination.finisher_trainee), "careers": [step(first_index.0, first_uses_existing), step(second_index.0, second_uses_existing), combination.finisher_cost]}),
                json!([member(first_index.0, first_uses_existing), member(second_index.0, second_uses_existing)]),
            )
        }
        None => (Value::Null, json!([])),
    };
    // the career a roll of this plan is of (`next.roll`, `roll_kept`: its saving per outcome, computed on demand from the
    // roll state kept here): the pick, her uncertain targets (`uncv`) and the scenario's guaranteed ones (`fixed`)
    let outcomes = match runs.get(pick) {
        Some(row) if row.scoring.is_some() => {
            json!({"trainee": d.umas[row.trainee].id, "parents": [row.career.a, row.career.b],
        "uncv": env.uncv, "fixed": env.fixed_t})
        }
        _ => Value::Null,
    };
    if let Some(r) = rk
        .filter(|r| runs.get(pick).is_some_and(|row| row.trainee == r.x && row.career.a == r.a && row.career.b == r.b))
    {
        keep_roll(r);
    }
    let mut out = json!({"choice": choice, "rule": "index", "mode": "trio", "v0": if scores.fit.current_value.is_finite() { json!(scores.fit.current_value) } else { Value::Null }, "outcomes": outcomes,
           "members": members_json,
           "v0_plan": plan_json, "fit": (0..n).map(|ui| json!([d.umas[candidates.trainees[ui]].id, scores.fit.continuations[ui].lam, scores.fit.continuations[ui].phi, scores.fit.continuations[ui].pinf])).collect::<Vec<_>>(), "loop_members": candidates.loop_member_info, "loop_note": candidates.loop_member_note, "hint": candidates.hint, "umas_scored": profile.exact_n, "umas_searched": runs.iter().filter(|r| r.exact).count() - profile.n_cert, "umas_certified": profile.n_cert, "umas_unproven": (0..d.umas.len()).filter(|&u| searches.base_incomplete[u] || searches.looped_incomplete[u] || searches.no_borrow_incomplete[u]).count(), "work": scores.work,
           "sur": pj.sur, "sur_diff": profile.sur_diff, "loop_ms": profile.loop_ms, "search_ms": profile.search_ms, "combo_ms": scores.combo_ms, "index_ms": scores.index_ms, "total_ms": now_ms() - profile.t_all});
    // (the option reports moved in, not copied by `json!`: the object's keys are ordered, so the same text)
    out["options"] = Value::Array(opts);
    if !proxy_check.is_null() {
        out["proxy_check"] = proxy_check;
    }
    out["work"]["prof"] = json!(PROF.with(|p| std::mem::take(&mut *p.borrow_mut())));
    // a roll sent with the plan job (`next.roll`) when the worker kept no roll state for it: answered from the state just
    // kept (none when nothing was scored)
    if pj.roll.is_some() {
        out["roll"] = match roll_kept(env, input.resources, legs, pool, fix, pj) {
            Some(mut r) => {
                r["planned"] = json!(true);
                r
            }
            None => json!({"error": "the plan has no pick to roll"}),
        };
    }
    out
}
