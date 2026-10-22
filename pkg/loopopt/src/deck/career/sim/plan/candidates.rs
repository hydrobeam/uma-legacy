//! Candidate setup and exact and surrogate careers.
use super::{
    BaseRows, CachedSearch, CareerScore, HINT_FLOOR, IDX_CANDS, PROXY_EFF, PlanCandidates, PlanInputs, PlanProfile,
    PlanRow, PlanSearches, PlanSetup, RowOrigin, Want, best_of, best_sur, bests_key, capped, certified, hint_umas,
    is_full, lift_eff, rotation, same_career, score_career, sim_for, stamp_ok, stamped, take_kept_plan,
};
use crate::deck::career::sim::plan::ScoringContext;
use crate::deck::career::sim::{Env, Found, Ref, Sel, Sim, Value, json, now_ms, prof, progress};
use crate::deck::career::{Career, Lineage};

pub(in crate::deck::career::sim::plan) fn setup_plan_candidates<'env, 'data>(
    env: &'env mut Env<'data>,
    input: &PlanInputs<'_, 'data>,
    t_all: f64,
    gp: Found,
    buf: &mut Vec<f64>,
) -> Result<PlanSetup<'env, 'data>, Value> {
    let PlanInputs { legacies: legs, pool, solved: sv, fixed_parents: fix, trainees, settings: pj, .. } = *input;
    let d = input.resources.data;
    let job = input.resources.job;
    // the work counters at the start (deck-search nodes, exact scorings): reported as this plan's own
    let (nodes0, joints0) = (env.space.nodes.get(), crate::deck::JCOUNT.with(std::cell::Cell::get));
    // the loop planner's members only when the job names them (`loop_members`, an override; the plan no longer runs the
    // loop planner's search itself: on the val/plan pools its members always entered the candidates by the routes below)
    let (members, loop_member_info, loop_member_note) = match &pj.loop_members {
        Some(ids) => rotation(env, d, ids),
        None => (vec![], vec![], None),
    };
    let g = sv.found[0].clone();
    // stage-1 candidates (who A can be; whose odds a builder may lift): the best career's trainee, the named loop members,
    // the umas of the best career's lineage, the umas you have veterans of, the search's other trainees; at most IDX_CANDS
    let mut cands: Vec<usize> = vec![g.t];
    let push = |u: usize, c: &mut Vec<usize>| {
        if !c.contains(&u) && trainees.contains(&u) {
            c.push(u);
        }
    };
    for &m in &members {
        push(m, &mut cands);
    }
    for x in [g.a, g.b] {
        for c in std::iter::once(legs[x].chara).chain(legs[x].parents.iter().map(|p| p.chara)) {
            if let Some(u) = env.uix(c) {
                push(u, &mut cands);
            }
        }
    }
    for l in legs.iter().rev() {
        if !l.borrowed
            && let Some(u) = env.uix(l.chara)
        {
            push(u, &mut cands);
        }
    }
    for f in &sv.found {
        push(f.t, &mut cands);
    }
    cands.truncate(IDX_CANDS);
    if let Some(f) = &pj.fit {
        cands = f.iter().filter_map(|&(c, ..)| env.uix(c)).filter(|u| trainees.contains(u)).collect();
    }
    // a pool that gives fewer than HINT_FLOOR candidates (a loop being started: veterans of umas that won't be in it, mob
    // parents) is filled from the planner's own model: the owned umas ranked by their odds with a full loop lineage
    // (`hint_umas`), no deck search
    let mut hint: Vec<Value> = vec![];
    if pj.loop_members.is_none() && pj.fit.is_none() && cands.len() < HINT_FLOOR {
        for (u, v) in hint_umas(env, d, pool, &cands, trainees, HINT_FLOOR - cands.len()) {
            cands.push(u);
            hint.push(json!({"id": d.umas[u].id, "name": d.umas[u].name, "ceiling": v}));
        }
    }
    let loop_ms = now_ms() - t_all;
    let all: Vec<usize> = trainees.iter().copied().filter(|&u| env.dmax[u].is_some()).collect();
    let t1 = now_ms();
    let original_budget = env.budget;
    // (the candidates' career searches below under the cap, as `capped` sets it: the Sim it takes is made after them)
    if pj.search_work > 0 {
        env.budget = pj.search_work;
        env.hard = pj.search_work;
    }
    env.use_seen = true;
    // best careers, searched on demand (stage-1 candidates now, builders when scored exactly)
    let mut best = vec![CachedSearch::Unsearched; d.umas.len()];
    best[g.t] = CachedSearch::Known(Some(g.clone()));
    // a kept plan of this pool less its last veteran (`Bests`), and this plan's own, kept at the end
    let nl = legs.len();
    // (taken out while this plan runs and put back after it: every reroll of a career reuses the same one)
    let kept_same_pool = take_kept_plan(bests_key(job, fix, nl));
    let kept_previous_pool = if nl > 1 { take_kept_plan(bests_key(job, fix, nl - 1)) } else { None };
    let mut proof_refs: Vec<Vec<Vec<Ref>>> = vec![vec![]; d.umas.len()];
    // per uma: her best career's search was not complete (`best_of`), and her looped one's
    let mut base_incomplete: Vec<bool> = vec![false; d.umas.len()];
    let looped_incomplete: Vec<bool> = vec![false; d.umas.len()];
    // (and her nb row's, under "later")
    let no_borrow_incomplete: Vec<bool> = vec![false; d.umas.len()];
    if pj.sur != 1 {
        for &u in &cands {
            if best[u].is_unsearched() {
                let (found, refs, complete) =
                    best_of(env, pool, fix, u, kept_same_pool.as_ref(), kept_previous_pool.as_ref(), nl);
                proof_refs[u] = refs;
                best[u] = CachedSearch::Known(found);
                base_incomplete[u] = !complete;
            }
        }
    }
    env.hard = 0;
    env.budget = original_budget;
    let mut search_ms = now_ms() - t1;
    let mut s = sim_for(env, input.resources, pj)?;
    // the cache the planner's table values read (trios' third members, candidates' lifted odds): each search's exact
    // winners only, the main search's best careers and each uma's best one, so what it holds (and every number read from
    // it) depends on the job alone, not on what the pruning happened to check fully (earlier jobs' proofs, the order, a
    // kept plan). Before, every fully checked deck went in: a candidate then had several decks, and her lifted odds
    // read the best of them; with one the planner's numbers moved once (val/plan: picks unchanged)
    for f in &sv.found {
        s.cache.add(s.env, f.t, &f.sel);
    }
    for &u in &cands {
        if let Some(f) = best[u].found() {
            s.cache.add(s.env, u, &f.sel);
        }
    }
    // (`proxy_check`: the cache as it is before any builder's search, for the proxies at the end)
    let cache0 = pj.proxy_check.then(|| s.cache.clone());
    // the surrogate's best careers (sur 1: in place of the searches; 2: beside them, differences reported)
    let mut sur_diff: Vec<Value> = vec![];
    if pj.sur > 0 {
        let ts = now_ms();
        capped(&mut s, pj.search_work, original_budget, |s| {
            for &u in &cands {
                if pj.sur == 2 || best[u].is_unsearched() {
                    plan_surrogate(s, input, &mut best, u, buf, &mut sur_diff);
                }
            }
        });
        search_ms += now_ms() - ts;
    }
    let fb: Vec<Option<Found>> = cands.iter().map(|&u| best[u].found().cloned()).collect();
    let n = cands.len();
    let pnow: Vec<f64> = fb.iter().map(|f| f.as_ref().map_or(0.0, |f| f.val)).collect();
    let sj = s.env.sj;
    // per candidate whether an 8/8 of hers exists (a free side of the trios)
    let have8: Vec<bool> = cands
        .iter()
        .map(|&u| legs.iter().any(|l| l.chara == d.umas[u].id && is_full(sj, &l.sparks) && pool.bor_one(l.borrowed)))
        .collect();
    let zeros = vec![0.0f64; n];
    let t3 = now_ms();
    // Builders through their deck class: a builder's career is first taken from her class's winning decks (the main
    // search's and the candidates' best careers of her class: legal for her, the same deck problem, valued at her own
    // chances through the surrogate, `best_sur`), no career search. Exact searches only where the plan can turn on her:
    // a lift efficiency within PROXY_EFF of a candidate's second best (Cont's two builders come from the exactly
    // searched careers), or a saving within PROXY_SAVE of the shown rows (the page's top OPT_SHOW) or the pick's near-tie
    // band. Measured on val/plan (`proxy_check`): the proxy's odds only ever read low (it is a
    // career she can run); its efficiency on contenders for a candidate's top two at most 2.8% low (p11; it can also
    // read high, which only searches more), its saving at most 2.7% low; exact on the other five jobs.
    // `exact_all` restores a search for every uma.
    let proxy_on = pj.sur == 0 && !pj.exact_all && !pj.proxy_check;
    let ones = vec![1.0f64; n];
    let candidates = PlanCandidates {
        main_career: g,
        pick_career: gp,
        members,
        loop_member_info,
        loop_member_note,
        hint,
        trainees: cands,
        allowed_trainees: all,
        careers: fb,
        pnow,
        have8,
        zeros,
        ones,
        proxy_on,
    };
    let searches = PlanSearches {
        best,
        proof_refs,
        base_incomplete,
        looped_incomplete,
        no_borrow_incomplete,
        looped_careers: vec![],
        kept_same_pool,
        kept_previous_pool,
        original_budget,
    };
    let profile = PlanProfile { t_all, t3, loop_ms, search_ms, nodes0, joints0, n_cert: 0, exact_n: 0, sur_diff };
    Ok(PlanSetup { sim: s, candidates, searches, profile, cache0 })
}

pub(in crate::deck::career::sim::plan) fn score_and_admit_proxies(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> BaseRows {
    let PlanInputs { legacies: legs, pool, solved: sv, fixed_parents: fix, settings: pj, .. } = *input;
    let d = input.resources.data;
    let n = candidates.trainees.len();
    let nl = legs.len();
    let mut runs: Vec<PlanRow> = vec![];
    let mut wins_by_trainee: Vec<Vec<usize>> = vec![vec![]; d.umas.len()];
    let wins: Vec<(usize, Sel)> = sv
        .found
        .iter()
        .map(|f| (f.t, f.sel.clone()))
        .chain(candidates.trainees.iter().filter_map(|&u| searches.best[u].found().cloned().map(|f| (u, f.sel))))
        .collect();
    // pass 1: every allowed uma's best career; per candidate, the chance her veteran raises that candidate's odds and the
    // mean odds raised to (exact over her outcomes)
    // (placeholder step costs: 1, so every candidate's lifts are gathered; an 8/8 of hers may be a poorer parent than a
    // fresh one)
    let base_s = candidates.stats_base();
    {
        let mut order: Vec<usize> = vec![candidates.main_career.t];
        for &x in &candidates.allowed_trainees {
            if x != candidates.main_career.t {
                order.push(x);
            }
        }
        for &x in &order {
            progress(2, order.iter().position(|&y| y == x).unwrap_or(0), order.len()); // pass 1: uma k of n
            if candidates.proxy_on && searches.best[x].is_unsearched() {
                let ts = now_ms();
                let cx = s.env.cls[x];
                for (t, sel) in &wins {
                    if *t != x && s.env.cls[*t] == cx {
                        s.cache.add(s.env, x, sel);
                    }
                }
                let pf = best_sur(s, pool, fix, x, 1e9, buf);
                profile.search_ms += now_ms() - ts;
                let Some(f) = pf else { continue };
                let (CareerScore { lift_stats: st, .. }, sv) = stamped(s, Some(&candidates.trainees), |s| {
                    score_career(
                        s,
                        &ScoringContext { score_base: &base_s, legacies: legs, pool },
                        Career { lineage: Lineage { trainee: x, first_parent: f.a, second_parent: f.b }, deck: &f.sel },
                        Want::Stats,
                        buf,
                    )
                });
                runs.push(PlanRow::new(x, f, st, RowOrigin::Base, false, sv));
                continue;
            }
            if searches.best[x].is_unsearched() || (pj.sur == 2 && !candidates.trainees.contains(&x)) {
                let ts = now_ms();
                capped(s, pj.search_work, searches.original_budget, |s| {
                    if pj.sur != 1 && searches.best[x].is_unsearched() {
                        let (f, r, ok) = best_of(
                            s.env,
                            pool,
                            fix,
                            x,
                            searches.kept_same_pool.as_ref(),
                            searches.kept_previous_pool.as_ref(),
                            nl,
                        );
                        if let Some(f) = &f {
                            s.cache.add(s.env, x, &f.sel);
                        }
                        searches.proof_refs[x] = r;
                        searches.best[x] = CachedSearch::Known(f);
                        searches.base_incomplete[x] = !ok;
                    }
                    if pj.sur > 0 {
                        plan_surrogate(s, input, &mut searches.best, x, buf, &mut profile.sur_diff);
                    }
                });
                profile.search_ms += now_ms() - ts;
            }
            let Some(f) = searches.best[x].found().cloned() else { continue };
            let CareerScore { lift_stats: st, .. } = score_career(
                s,
                &ScoringContext { score_base: &base_s, legacies: legs, pool },
                Career { lineage: Lineage { trainee: x, first_parent: f.a, second_parent: f.b }, deck: &f.sel },
                Want::Stats,
                buf,
            );
            runs.push(PlanRow::new(x, f, st, RowOrigin::Base, true, None));
        }
        // the near-top builders searched exactly: per candidate every proxy whose efficiency reaches (1 - PROXY_EFF) of the
        // candidate's second best (exact and proxy careers together), until none is left
        #[allow(clippy::while_immutable_condition, reason = "proxy_on is fixed per call; the loop ends at its break")]
        while candidates.proxy_on {
            let mut add: Vec<usize> = vec![];
            for ui in 0..n {
                if candidates.pnow[ui] <= 0.0 {
                    continue;
                }
                let (u, p) = (candidates.trainees[ui], candidates.pnow[ui]);
                let mut e: Vec<(f64, usize)> = runs
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| d.umas[r.trainee].id != d.umas[u].id && r.lift_stats[ui].0 > 0.0)
                    .map(|(i, r)| {
                        let (pl, sp) = r.lift_stats[ui];
                        (lift_eff(pl, sp, p), i)
                    })
                    .collect();
                e.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
                let Some(&(e1, _)) = e.first() else { continue };
                let second = e.get(1).map_or(e1, |x| x.0);
                for &(v, i) in &e {
                    if v < second * (1.0 - pj.proxy_eff.unwrap_or(PROXY_EFF)) {
                        break;
                    }
                    if !runs[i].exact && !add.contains(&i) {
                        add.push(i);
                    }
                }
            }
            if add.is_empty() {
                break;
            }
            add.sort_unstable();
            let ts = now_ms();
            for i in add {
                let x = runs[i].trainee;
                // a proxy the class's proofs certify needs no search (it is her best career's value)
                if certified(s.env, pool, fix, x, &runs[i].career) {
                    searches.best[x] = CachedSearch::Known(Some(runs[i].career.clone()));
                    runs[i].exact = true;
                    profile.n_cert += 1;
                    continue;
                }
                let r = {
                    let ri = &runs[i];
                    exact_plan_career(
                        s,
                        input,
                        candidates,
                        searches,
                        x,
                        buf,
                        Some((&ri.career, ri.lift_stats.as_slice(), runs[i].stats_stamp)),
                    )
                };
                if let Some((f, st)) = r {
                    runs[i].career = f;
                    runs[i].lift_stats = st;
                }
                runs[i].exact = true;
                runs[i].stats_stamp = None;
            }
            profile.search_ms += now_ms() - ts;
        }
        for r in &runs {
            set_plan_wins(s, input, &mut wins_by_trainee, r.trainee, &r.career);
        }
    }
    BaseRows { runs, wins_by_trainee }
}

fn plan_surrogate(
    s: &mut Sim,
    input: &PlanInputs,
    best: &mut [CachedSearch],
    u: usize,
    buf: &mut Vec<f64>,
    sur_diff: &mut Vec<Value>,
) {
    let PlanInputs { pool, fixed_parents: fix, settings: pj, .. } = *input;
    let d = input.resources.data;

    let f = best_sur(s, pool, fix, u, pj.sur_margin, buf);
    if pj.sur == 2 {
        let e = best[u].found().cloned();
        let (ve, vs) = (e.as_ref().map_or(0.0, |f| f.val), f.as_ref().map_or(0.0, |f| f.val));
        let same = e.as_ref().map(|f| (f.a.min(f.b), f.a.max(f.b))) == f.as_ref().map(|f| (f.a.min(f.b), f.a.max(f.b)));
        if !same || (ve - vs).abs() > 1e-9 * ve.max(vs) {
            sur_diff.push(json!({"trainee": d.umas[u].name, "search": [e.as_ref().map(|f| [f.a, f.b]), ve], "sur": [f.as_ref().map(|f| [f.a, f.b]), vs]}));
        }
    } else {
        best[u] = CachedSearch::Known(f);
    }
}

// uma x's exact best career (searched now if not yet) and its lifts; `reuse`: her proxy career, its lifts and their
// stamp (`stamped`, over the candidates' decks: this base reads no other's), taken as they are when the search found
// the same career and nothing they read has changed
pub(in crate::deck::career::sim::plan) fn exact_plan_career(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    searches: &mut PlanSearches,
    x: usize,
    buf: &mut Vec<f64>,
    reuse: Option<(&Found, &[(f64, f64)], Option<(usize, usize)>)>,
) -> Option<(Found, Vec<(f64, f64)>)> {
    let PlanInputs { legacies: legs, pool, fixed_parents: fix, settings: pj, .. } = *input;
    let nl = input.legacies.len();
    let base_s = candidates.stats_base();

    if searches.best[x].is_unsearched() {
        capped(s, pj.search_work, searches.original_budget, |s| {
            let (found, refs, complete) = best_of(
                s.env,
                pool,
                fix,
                x,
                searches.kept_same_pool.as_ref(),
                searches.kept_previous_pool.as_ref(),
                nl,
            );
            if let Some(found) = &found {
                s.cache.add(s.env, x, &found.sel);
            }
            searches.proof_refs[x] = refs;
            searches.best[x] = CachedSearch::Known(found);
            searches.base_incomplete[x] = !complete;
        });
    }
    let f = searches.best[x].found().cloned()?;
    if let Some((proxy, proxy_stats, stamp)) = reuse
        && same_career(proxy, &f)
        && stamp_ok(s, Some(&candidates.trainees), stamp)
    {
        prof("reuse_st", 1.0);
        return Some((f, proxy_stats.to_vec()));
    }
    let CareerScore { lift_stats: st, .. } = score_career(
        s,
        &ScoringContext { score_base: &base_s, legacies: legs, pool },
        Career { lineage: Lineage { trainee: x, first_parent: f.a, second_parent: f.b }, deck: &f.sel },
        Want::Stats,
        buf,
    );
    Some((f, st))
}

pub(in crate::deck::career::sim::plan) fn set_plan_wins(
    s: &mut Sim,
    input: &PlanInputs,
    wins_by_trainee: &mut [Vec<usize>],
    x: usize,
    f: &Found,
) {
    let PlanInputs { pool, .. } = *input;

    let w = s.wins(pool, Lineage { trainee: x, first_parent: f.a, second_parent: f.b });
    wins_by_trainee[x] = w.iter().enumerate().filter(|x| *x.1 > 0.5).map(|x| x.0).collect();
}

pub(in crate::deck::career::sim::plan) fn exact_no_borrow_career(
    s: &mut Sim,
    input: &PlanInputs,
    searches: &mut PlanSearches,
    x: usize,
    buf: &mut Vec<f64>,
) -> Option<Found> {
    let PlanInputs { fixed_parents: fix, settings: pj, .. } = *input;
    let pick_pool = input.pick_pool();
    let nl = input.legacies.len();

    capped(s, pj.search_work, searches.original_budget, |s| {
        if pj.sur == 1 {
            best_sur(s, pick_pool, fix, x, pj.sur_margin, buf)
        } else {
            let (f, _, ok) = best_of(s.env, pick_pool, fix, x, None, None, nl);
            if !ok {
                searches.no_borrow_incomplete[x] = true;
            }
            f
        }
    })
}
