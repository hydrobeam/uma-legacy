//! The assembled plan response.
use crate::career::ClassFlags;
use crate::career::clock::{PROF, now_ms};
use crate::career::plan::combine::{FITTED_MODEL, NEAR_TRIO_SHARE, PlanScores};
use crate::career::plan::cont::{SideStep, TrioSideVeteran, TrioSlot};
use crate::career::plan::kept::{Bests, CachedSearch, bests_key};
use crate::career::plan::roll::{KeptBase, RollKeep, keep_roll, roll_kept, roll_key};
use crate::career::plan::rows::{PlanPick, PlanRow, RowOrigin, has_looped_parents, pick_of};
use crate::career::plan::score::KeptScoring;
use crate::career::plan::{PlanCandidates, PlanContext, PlanProfile, PlanSearches};
use crate::career::search::Assumes;
use crate::career::search::Found;
use crate::career::sim::Sim;
use serde_json::{Value, json};
/// The plan's response: its options, pick and trio, with the pick's roll state and the plan's searches kept for later
/// jobs (`RollKeep`, `Bests`)
pub(crate) fn assemble_plan_output<'data>(
    sim: Sim<'_, 'data>,
    context: &PlanContext<'_, 'data>,
    searches: PlanSearches,
    mut runs: Vec<PlanRow>,
    scores: &PlanScores,
    profile: &PlanProfile,
) -> Value {
    let PlanContext { input, candidates } = *context;
    let skill_job = sim.env.setup.skill_job;
    let legacies = input.legacies;
    let pool = input.pool;
    let data = context.data();
    let is_runnable = |career: &Found| input.is_runnable(career);
    // a looped or nb row that repeats an earlier row's career (a builder's exact search can land on the same pair of full veterans; an
    // nb row on her looped one): left out, the pick moved to the earlier one (the same career, the same score)
    let repeated: Vec<Option<usize>> = (0..runs.len()).map(|i| (0..i).find(|&j| runs[j].repeats(&runs[i]))).collect();
    let duplicate: Vec<bool> = repeated.iter().map(Option::is_some).collect();
    let undup: Vec<usize> = repeated.iter().enumerate().map(|(i, earlier)| earlier.unwrap_or(i)).collect();
    let can_roll =
        runs.iter().any(|run| is_runnable(&run.career)) && scores.any_scored && scores.fit.current_value.is_finite();
    let roll_state = can_roll.then(|| pick_roll_state(context, &mut runs, scores, &undup));
    // Release the simulator's caches before publishing kept searches and reporting options.
    let (env, kept) = {
        let finished = sim;
        (finished.env, finished.kept)
    };
    let incomplete =
        context.keep_plan_searches(kept, searches, env.setup.keyed_flags(&env.state), env.setup.plan_space_key);
    // nothing scored (no trio: fewer than two candidates of different umas): the best odds is the one option and the pick
    // (no row that can be the pick, `is_runnable`: the same, the pick's main search's)
    if !runs.iter().any(|run| is_runnable(&run.career)) {
        for found in &input.pick_solved().found {
            runs.push(PlanRow::proxy(found.trainee, found.clone(), vec![], RowOrigin::Fallback, None));
        }
    }
    // pick: the highest score among the rows that can be it
    let total = |i: usize| runs[i].total();
    let PlanPick { row: pick, .. } = pick_of(&runs, is_runnable, &undup);
    // A row as the output reports it: exact (a career search behind it: hers, or for a fallback row the pick's main
    // search), and not proven where that search was not complete (a fallback row's: the pick's main search, its band
    // certificate included) or her deck class is flagged breached by any search so far, this plan's later ones
    // included (`ClassFlags`); the named assumptions it rests on: her class's, and a fallback row's main search's.
    let pick_solved = input.pick_solved();
    let pick_search = RowSearch {
        complete: pick_solved.complete && pick_solved.certified_cap >= env.setup.space.rho_top(),
        assumes: pick_solved.assumes,
    };
    let row_exact = |row: &PlanRow| row.exact || row.origin == RowOrigin::Fallback;
    let row_proofs: Vec<(bool, Assumes)> = runs
        .iter()
        .map(|row| {
            let search = if row.origin == RowOrigin::Fallback {
                pick_search
            } else {
                RowSearch { complete: !incomplete.of(row), assumes: Assumes::default() }
            };
            row_proof(row_exact(row), search, env.state.class_flags[env.setup.deck_class[row.trainee]])
        })
        .collect();
    let mut options: Vec<Value> = vec![];
    for (i, row) in runs.iter().enumerate() {
        let (trainee, career) = (row.trainee, &row.career);
        // (a builder with a borrowed parent under "later" is no option)
        if duplicate.get(i).copied().unwrap_or(false) || !is_runnable(career) {
            continue;
        }
        let looped = has_looped_parents(skill_job, legacies, career);
        let mut report = env.setup.report(&mut env.state, pool, career, false);
        let option = report.as_object_mut().unwrap();
        option.insert("role".into(), json!(row_role(candidates, scores, trainee, looped)));
        option.insert(
            "best_odds".into(),
            json!(
                trainee == candidates.pick_career.trainee
                    && career.unordered_parents() == candidates.pick_career.unordered_parents()
                    && crate::career::plan::kept::same_deck(&career.deck, &candidates.pick_career.deck)
            ),
        );
        option.insert("chosen".into(), json!(i == pick));
        // both parents full veterans: her full veteran here is a fully looped parent
        option.insert("looped".into(), json!(looped));
        // false: a builder the plan could not turn on, her career from her deck class's winning decks (no career search)
        option.insert("exact".into(), json!(row_exact(row)));
        // a builder's next best career (`builder_careers`), from her cached decks
        if row.origin == RowOrigin::Alternative {
            option.insert("alternative".into(), json!(true));
        }
        // her career search was not complete (its work cap `search_work`, a band overflow, or a deck past its class's
        // cap that the cap-free rerun did not settle) or her class is breached (`row_proof`): the best it found, not
        // proven her best; else proven under the named assumptions it rests on (`assumes`)
        let (proven, assumes) = row_proofs[i];
        if row_exact(row) && !proven {
            option.insert("proven".into(), json!(false));
            // (an alternative row never searched on its pair: her cached deck there, no search behind it to run out)
            if row.origin == RowOrigin::Alternative && row.pair_search.is_none() {
                option.insert("searched".into(), json!(false));
            }
        } else if row_exact(row) && assumes != Assumes::default() {
            option.insert("assumes".into(), json!(assumes.names()));
        }
        option.insert(
            "stage_cost".into(),
            json!(
                candidates
                    .trainees
                    .iter()
                    .position(|&candidate_trainee| candidate_trainee == trainee)
                    .map(|candidate| scores.fit.step_costs[candidate])
            ),
        );
        option.insert("index".into(), json!(row.net_saving()));
        option.insert("saving".into(), json!(row.net_saving().map(|net_saving| net_saving + 1.0)));
        option.insert("lift".into(), json!(row.scoring.as_ref().map(|scoring| scoring.improvement_chance)));
        // the side her veteran's routes lift most often where she lowers V, and how often she is a side herself
        option.insert(
            "lifts_side".into(),
            json!(row.scoring.as_ref().map(|scoring| lifts_side(candidates, &scoring.route_wins, data))),
        );
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
    let choice =
        if runs.is_empty() { Value::Null } else { env.setup.report(&mut env.state, pool, &runs[pick].career, true) };
    let (plan_json, members_json) = context.trio_report(scores, &runs, &env.setup.space);
    // the career a roll of this plan is of (`next.roll`, `roll_kept`: its saving per outcome, computed on demand from the
    // roll state kept here): the pick, her uncertain targets (`uncv`) and the scenario's guaranteed ones (`fixed`)
    let outcomes = match runs.get(pick) {
        Some(row) if row.scoring.is_some() => {
            json!({"trainee": data.umas[row.trainee].id, "parents": [row.career.first_parent, row.career.second_parent],
        "career": crate::career::plan::kept::career_id(&env.setup.space, data.umas[row.trainee].id, &row.career),
        "uncv": env.setup.uncertain_targets, "fixed": env.setup.fixed_targets})
        }
        _ => Value::Null,
    };
    if let Some(roll) = roll_state.filter(|roll| runs.get(pick).is_some_and(|row| roll.is_of(row.trainee, &row.career)))
    {
        keep_roll(kept, roll);
    }
    let fit = &scores.fit;
    // (the stage candidates': a side-only uma has no continuation, and a fit given back names stage candidates)
    let fit_json: Vec<Value> = (0..candidates.stage_count)
        .map(|candidate| {
            let continuation = &fit.continuations[candidate];
            json!([
                data.umas[candidates.trainees[candidate]].id,
                continuation.lam,
                continuation.phi,
                continuation.pinf,
                continuation.anchor_odds
            ])
        })
        .collect();
    let flagged = |uma: crate::uma::UmaIndex| {
        // (a search behind the row ran and is not proven: an alternative row never searched on its pair is not counted)
        runs.iter().zip(&row_proofs).any(|(row, &(proven, _))| {
            row.trainee == uma
                && row_exact(row)
                && !proven
                && !(row.origin == RowOrigin::Alternative && row.pair_search.is_none())
        })
    };
    let umas_unproven = data
        .umas
        .indices()
        .filter(|&uma| incomplete.base[uma] || incomplete.looped[uma] || incomplete.no_borrow[uma] || flagged(uma))
        .count();
    let current_value = if fit.current_value.is_finite() { json!(fit.current_value) } else { Value::Null };
    // per candidate (uma id) the two builders (uma ids) behind her full-loop ceiling, pinf's pair (`ContEvidence`), where
    // her Cont was fitted from this pool's evidence
    let pinf_builders: serde_json::Map<String, Value> = (0..candidates.trainees.len())
        .filter(|&candidate| !context.has_given_fit(candidate))
        .filter_map(|candidate| {
            let pair = fit.evidence.get(candidate)?.as_ref()?.loop_builders()?;
            Some((
                data.umas[candidates.trainees[candidate]].id.to_string(),
                json!([data.umas[pair[0]].id, data.umas[pair[1]].id]),
            ))
        })
        .collect();
    // per candidate (uma id) whose step is her kept cap's (`caps`: below her Cont's step now), its careers and her odds
    // then; a cap that does not bind is left out, so a plan of the same pool again (a reload) answers as before
    let step_caps: serde_json::Map<String, Value> = (0..candidates.trainees.len())
        .filter_map(|candidate| {
            let cap = candidates.step_caps[candidate].now?;
            let own_step = fit.continuations[candidate].cost(candidates.current_odds[candidate]);
            (cap.now.expected_careers < own_step).then(|| {
                (
                    data.umas[candidates.trainees[candidate]].id.to_string(),
                    json!({"careers": cap.now.expected_careers, "p": cap.kept.odds}),
                )
            })
        })
        .collect();
    // the side-only umas (`PlanCandidates::stage_count`): umas whose real full veterans are trio sides, not candidates
    let side_only: Vec<Value> = candidates.trainees[candidates.stage_count..]
        .iter()
        .map(|&uma| json!({"id": data.umas[uma].id, "name": data.umas[uma].name}))
        .collect();
    let mut out = json!({"choice": choice, "rule": "index", "mode": "trio", "v0": current_value, "outcomes": outcomes,
           "members": members_json, "trios": context.cheapest_trios_json(scores),
           "v0_plan": plan_json, "fit": fit_json, "pinf_builders": pinf_builders, "step_caps": step_caps, "loop_members": candidates.loop_member_info, "loop_note": candidates.loop_member_note, "hint": candidates.hint, "cut": candidates.cut, "side_only": side_only, "umas_scored": profile.scored_runnable_rows, "umas_searched": runs.iter().filter(|run| run.exact).count() - profile.certified_proxies, "umas_certified": profile.certified_proxies, "umas_unproven": umas_unproven, "work": scores.work,
           "sur": input.settings.surrogate_mode, "sur_diff": profile.surrogate_differences, "loop_ms": profile.loop_ms, "search_ms": profile.search_ms, "combo_ms": scores.combo_ms, "index_ms": scores.index_ms, "total_ms": now_ms() - profile.started_ms});
    // (the option reports moved in, not copied by `json!`: the object's keys are ordered, so the same text)
    out["options"] = Value::Array(options);
    let prof = PROF.with(|p| std::mem::take(&mut *p.borrow_mut()));
    // (routes the scorings' memory budget left out, `OUTCOME_BUDGET`: each lowers that row's saving, never raises it)
    out["routes_dropped"] = json!(prof.get("sc_routes_dropped").copied().unwrap_or(0.0));
    // (of the routes left out, those no check cleared, `ShadowRoute`: each could have lowered a row's V after her)
    let uncertified = prof.get("sc_routes_dropped_uncertified").copied().unwrap_or(0.0);
    out["routes_dropped_uncertified"] = json!(uncertified);
    // (false where the memory budget left out something that could move a score: a route no batch could hold, or a
    // partner setup `fit_partner_setups` dropped; each only lowers a row's saving, so the savings are then lower bounds,
    // not the model's. A route left out with its check cleared, `routes_dropped` less these, moves no score)
    let partners_dropped = prof.get("sc_partner_setups_dropped").copied().unwrap_or(0.0);
    out["scoring_complete"] = json!(uncertified == 0.0 && partners_dropped == 0.0);
    out["work"]["prof"] = json!(prof);
    // a roll sent with the plan job (`next.roll`) when the worker kept no roll state for it: answered from the state just
    // kept (none when nothing was scored)
    if input.settings.roll.is_some() {
        out["roll"] = match roll_kept(env, kept, input.resources, input.settings) {
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
/// scoring, taken out of its row, and base. Only where a row can be the pick and rows were scored against a finite V
fn pick_roll_state(context: &PlanContext, runs: &mut [PlanRow], scores: &PlanScores, undup: &[usize]) -> RollKeep {
    let input = context.input;
    let is_runnable = |career: &Found| input.is_runnable(career);
    let PlanPick { row: pick, .. } = pick_of(runs, is_runnable, undup);
    let base = scores.fit.score_base(context.candidates);
    let KeptScoring { scored, later } =
        runs[pick].scoring.as_mut().unwrap().kept.take().expect("the pick's kept scoring was dropped");
    let row = &runs[pick];
    RollKeep {
        key: roll_key(input.resources.job),
        scored,
        later,
        base: KeptBase::of(&base),
        lineage: row.lineage(),
        // (the roll state outlives the plan's rows)
        deck: row.career.deck.clone(),
    }
}

/// Per uma (`Data::umas`), whether a search behind her rows was incomplete, by the row's origin; an alternative row's
/// own (`PlanRow::pair_search`: not proven where none ran or it did not complete)
struct IncompleteSearches {
    base: crate::uma::UmaTable<bool>,
    looped: crate::uma::UmaTable<bool>,
    no_borrow: crate::uma::UmaTable<bool>,
}

impl IncompleteSearches {
    fn of(&self, row: &PlanRow) -> bool {
        let trainee = row.trainee;
        match row.origin {
            RowOrigin::Base => self.base[trainee],
            RowOrigin::Looped => self.looped[trainee],
            RowOrigin::NoBorrow => self.no_borrow[trainee],
            // (its cached deck on its pair is not proven her best there unless that pair was searched to the end)
            RowOrigin::Alternative => row.pair_search != Some(true),
            RowOrigin::Fallback => false,
        }
    }
}

impl PlanContext<'_, '_> {
    /// This plan's searches kept for the next plans (`KeptState::bests`, at most four), with the kept plan of the pool less its last
    /// veteran put back; what stays for the output is which searches were incomplete
    fn keep_plan_searches(
        &self,
        kept: &mut crate::career::state::KeptState,
        searches: PlanSearches,
        flags: Vec<(u64, crate::career::ClassFlags)>,
        space_key: u64,
    ) -> IncompleteSearches {
        let input = self.input;
        let key = bests_key(input.resources.job, input.fixed_parents, self.legacy_count(), space_key);
        let mut kept_best = searches.best_careers.clone();
        // The main search's best is per job, searched again (cheap) next time.
        kept_best[self.candidates.main_career.trainee] = CachedSearch::Unsearched;
        {
            let cache = &mut kept.bests;
            cache.retain(|plan| plan.key != key);
            cache.push(Bests {
                key,
                best_careers: kept_best,
                proof_refs: searches.proof_refs,
                looped_careers: searches.looped_careers,
                base_incomplete: searches.base_incomplete.clone(),
                looped_incomplete: searches.looped_incomplete.clone(),
                flags: flags.clone(),
            });
            // (the plan of the pool less its last veteran put back with this job's flags too: they only grow)
            if let Some(mut previous) = searches.kept_previous_pool {
                let mut merged: std::collections::BTreeMap<u64, crate::career::ClassFlags> =
                    std::collections::BTreeMap::new();
                for &(key, kept_flags) in previous.flags.iter().chain(&flags) {
                    let slot = merged.entry(key).or_default();
                    *slot = slot.union(kept_flags);
                }
                previous.flags = merged.into_iter().collect();
                cache.push(previous);
            }
            while cache.len() > 4 {
                cache.remove(0);
            }
        }
        IncompleteSearches {
            base: searches.base_incomplete,
            looped: searches.looped_incomplete,
            no_borrow: searches.no_borrow_incomplete,
        }
    }
}

/// A career's role in the plan: one of the best trio's A and B, its finisher (C on A's and B's real full veterans, so only on two
/// full veteran parents), else a loop member, a candidate or a builder
fn row_role(
    candidates: &PlanCandidates,
    scores: &PlanScores,
    trainee: crate::uma::UmaIndex,
    looped: bool,
) -> &'static str {
    let existing = |slot: TrioSlot| matches!(scores.fit.sides[slot.candidate.0][slot.side].step, SideStep::Existing(_));
    match scores.best_combo {
        Some(combination) if combination.trio.sides.iter().any(|slot| trainee == candidates[slot.candidate]) => "plan",
        Some(combination)
            if trainee == combination.finisher_trainee
                && combination.trio.sides.iter().all(|&slot| existing(slot))
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

impl PlanContext<'_, '_> {
    /// The cheapest trios (`PlanScores::cheapest_combos`), cheapest first, as the page names near-tied routes: A, B and
    /// C (A the cheaper side), their careers and the trio's value, and `near` where it is within NEAR_TRIO_SHARE of V's
    fn cheapest_trios_json(&self, scores: &PlanScores) -> Value {
        let data = self.data();
        let fit = &scores.fit;
        let step = |slot: TrioSlot| fit.sides[slot.candidate.0][slot.side].step.cost(fit.step_costs[slot.candidate.0]);
        let name = |uma: crate::uma::UmaIndex| json!(data.umas[uma].name);
        let trios: Vec<Value> = scores
            .cheapest_combos
            .iter()
            .map(|combination| {
                let [side_a, side_b] = combination.trio.sides;
                let (first, second) = if step(side_a) <= step(side_b) { (side_a, side_b) } else { (side_b, side_a) };
                let value = fit.trio_value(&combination.trio);
                let side_key = |slot: TrioSlot| side_key(&fit.sides[slot.candidate.0][slot.side]);
                json!({"a": name(self.candidates[first.candidate]), "b": name(self.candidates[second.candidate]),
                    "c": name(combination.finisher_trainee), "careers": [step(first), step(second), combination.trio.finisher_cost],
                    "sides": [side_key(first), side_key(second)],
                    "value": value, "near": value <= fit.current_value * (1.0 + NEAR_TRIO_SHARE)})
            })
            .collect();
        Value::Array(trios)
    }
    /// Whether the job's `next.fit` gave this candidate's Cont (`plan_continuations`)
    fn has_given_fit(&self, candidate: usize) -> bool {
        let character = self.data().umas[self.candidates.trainees[candidate]].id;
        self.input.settings.fit.as_ref().is_some_and(|fit| fit.iter().any(|entry| entry.character_id() == character))
    }
    /// The best trio as the page shows it (`v0_plan`: A, B, C and their careers, the cheaper of A's and B's steps first)
    /// and its two members' continuations (`members`)
    fn trio_report(
        &self,
        scores: &PlanScores,
        runs: &[PlanRow],
        space: &crate::deck::space::DeckSpace,
    ) -> (Value, Value) {
        let candidates = self.candidates;
        let data = self.data();
        let Some(combination) = scores.best_combo else { return (Value::Null, json!([])) };
        let uma_name = |uma: crate::uma::UmaIndex| {
            if uma == crate::uma::NONE { Value::Null } else { json!(data.umas[uma].name) }
        };
        let fit = &scores.fit;
        // a side's step: free from a real full veteran of hers, else her fresh full veteran's step (her best career's,
        // or another of her top careers' at its odds)
        let step = |slot: TrioSlot| fit.sides[slot.candidate.0][slot.side].step.cost(fit.step_costs[slot.candidate.0]);
        // a member: `real` when the trio uses a real full veteran of hers (her step free), else her fresh full veteran's
        // step; `p` and `parents` the career the fresh side is from (`alternative`: not her best career now), `p0` the
        // odds her continuation is anchored at
        let member = |slot: TrioSlot| {
            let candidate = slot.candidate.0;
            let side = &fit.sides[candidate][slot.side];
            let continuation = &fit.continuations[candidate];
            let alternative = matches!(side.step, SideStep::Alternative(_));
            let odds = if alternative { side.odds } else { candidates.current_odds[candidate] };
            // (her side as the fit prices it, `caps`: at her least held cap curve at the side's odds where that is
            // cheaper, for her best career's side the cap's plan now, `StepCaps::now`)
            let own_plan = continuation.plan(odds);
            let build_plan = candidates.step_caps[candidate].capped_at(own_plan, odds);
            let capped = build_plan.expected_careers < own_plan.expected_careers;
            // the builder row her lam and phi are from (`fit_continuation`): its career id; null where she grinds at
            // these odds, her Cont was given (`next.fit`) or her cap's plan (a kept fit's) is hers
            let builder = (build_plan.building_careers > 0.0 && !self.has_given_fit(candidate) && !capped)
                .then(|| fit.evidence.get(candidate)?.as_ref())
                .flatten()
                .map(|evidence| crate::career::plan::combine::fit_continuation(evidence, FITTED_MODEL).1)
                .and_then(|builder| runs.get(builder))
                .map(|row| crate::career::plan::kept::career_id(space, data.umas[row.trainee].id, &row.career));
            json!({
                "trainee": uma_name(candidates.trainees[candidate]),
                "p": odds,
                "parents": side.parents.map(|(first, second)| [first, second]),
                "alternative": alternative,
                "capped": capped,
                "proven": side.proven,
                "has_full_veteran": candidates.has_full_veteran[candidate],
                "real": matches!(side.step, SideStep::Existing(_)),
                "p_inf": (continuation.lam > 0.0).then_some(continuation.pinf),
                "lam": continuation.lam,
                "phi": continuation.phi,
                "p0": continuation.anchor_odds,
                "building_careers": build_plan.building_careers,
                "p_switch": build_plan.grind_odds,
                "builder": builder,
                "careers": step(slot)
            })
        };
        // A: the cheaper of the two steps first
        let [side_a, side_b] = combination.trio.sides;
        let (first, second) = if step(side_a) <= step(side_b) { (side_a, side_b) } else { (side_b, side_a) };
        (
            json!({"a": uma_name(candidates[first.candidate]), "b": uma_name(candidates[second.candidate]), "c": uma_name(combination.finisher_trainee), "careers": [step(first), step(second), combination.trio.finisher_cost],
                "sides": [side_key(&fit.sides[first.candidate.0][first.side]), side_key(&fit.sides[second.candidate.0][second.side])]}),
            json!([member(first), member(second)]),
        )
    }
}

/// A trio side's identity, for the page to tell whether two plans' routes are the same (`sameTrio`, with the sides'
/// umas): "real:i" her real full veteran, legacy i (a veteran added to the pool goes last, so i names the same one in
/// both plans); "current:i,j" the fresh full veteran of her best career now, on legacies i and j (a veteran that becomes
/// one of her parents is another lineage, so another route: its step's careers are not compared); "alt:i,j" the fresh
/// full veteran of another of her careers, on legacies i and j; i below j (a lineage's parents are a pair, unordered)
fn side_key(side: &TrioSideVeteran) -> String {
    let parents = side.parents.map(|(first, second)| (first.min(second), first.max(second)));
    match (side.step, parents) {
        (SideStep::Existing(legacy), _) => format!("real:{legacy}"),
        (SideStep::Current, Some((low, high))) => format!("current:{low},{high}"),
        (SideStep::Current, None) => "current".to_owned(),
        (SideStep::Alternative(_), Some((low, high))) => format!("alt:{low},{high}"),
        (SideStep::Alternative(_), None) => "alt".to_owned(),
    }
}

/// A scored row's `lifts_side`: over the outcomes where her veteran lowers V, the candidate (uma id) whose side the
/// route V after her takes lifts most often and that chance (null and 0 where no route taken lifts a side), and the
/// chance the route taken has her own full veteran as a side or is the goal itself (`kernel::RouteWins`)
fn lifts_side(
    candidates: &PlanCandidates,
    route_wins: &crate::career::plan::score::kernel::RouteWins,
    data: &crate::data::Data,
) -> Value {
    let most = route_wins.most_lifted();
    json!({
        "uma": most.map(|(candidate, _)| data.umas[candidates.trainees[candidate]].id),
        "chance": most.map_or(0.0, |(_, chance)| chance),
        "own": route_wins.own_side,
    })
}

/// The career search behind a plan row: whether it was complete, and the named assumptions it rests on.
#[derive(Clone, Copy)]
struct RowSearch {
    complete: bool,
    assumes: Assumes,
}

/// A row's proof as the output reports it: proven when exact (a career search behind it: hers, or a fallback row's,
/// the pick's main search), that search complete and her deck class not breached (`ClassFlags`, as the flags stand at
/// the end of the plan); the named assumptions it rests on, her class's and the search's.
const fn row_proof(exact: bool, search: RowSearch, flags: ClassFlags) -> (bool, Assumes) {
    (exact && search.complete && flags.proves(), flags.assumes.union(search.assumes))
}

#[cfg(test)]
mod tests {
    use super::{Assumes, ClassFlags, RowSearch, row_proof};
    /// A fallback row (the pick's main search's career, exported exact) is unproven when that search was not
    /// complete or her class is breached, and carries that search's assumptions and her class's.
    #[test]
    fn fallback_rows_take_the_pick_search_and_class_flags() {
        let multi = Assumes { two_feeder: false, multi_feeder: true };
        let two = Assumes { two_feeder: true, multi_feeder: false };
        let plain = ClassFlags::default();
        let breached = ClassFlags { breached: true, assumes: Assumes::default() };
        let assuming = ClassFlags { breached: false, assumes: two };
        let complete = RowSearch { complete: true, assumes: multi };
        let incomplete = RowSearch { complete: false, assumes: Assumes::default() };
        assert_eq!(row_proof(true, complete, plain), (true, multi));
        assert_eq!(row_proof(true, complete, assuming), (true, multi.union(two)));
        assert!(!row_proof(true, complete, breached).0, "a breached class's fallback row proven");
        assert!(!row_proof(true, incomplete, plain).0, "an incomplete pick search's fallback row proven");
        assert!(!row_proof(false, complete, plain).0, "a proxy row proven");
    }
}
