//! Plan rows, their scoring and promotion.
use super::{
    Bests, CachedSearch, CareerScore, KeptScoring, OPT_SHOW, PROXY_SAVE, PlanCandidates, PlanCombination, PlanFit,
    PlanInputs, PlanProfile, PlanScores, PlanSearches, Trio, Want, best_sur, capped, certified, exact_no_borrow_career,
    exact_plan_career, is_full, lift_eff, memo_stat, plan_combinations, plan_continuations, plan_sides, same_career,
    score_career, set_plan_wins, stamp_ok, stamped, trio_v,
};
use crate::deck::career::sim::{Found, Sel, Sim, Value, json, now_ms, prof, progress};

/// Scores within this many careers per run of the best (or 5% of it) count as tied
const NEAR: f64 = 0.25;

/// the lowest score tied with `top` (`NEAR`)
pub(in crate::deck::career::sim::plan) fn near_floor(top: f64) -> f64 {
    top - NEAR.max(0.05 * top.abs())
}

/// Why a career was added to the planner's options. Rows retain their insertion order:
/// base careers, looped alternatives, then alternatives without borrowed parents.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::deck::career::sim::plan) enum RowOrigin {
    Base,
    Looped,
    NoBorrow,
    /// Main-search options used when no planner row can be picked. Not a planner search.
    Fallback,
}

/// A career and its planner state. Replacing the career preserves the previous scoring
/// until pass 2 either reuses it (same career and valid stamp) or scores it again.
pub(in crate::deck::career::sim::plan) struct PlanRow {
    pub(in crate::deck::career::sim::plan) trainee: usize,
    pub(in crate::deck::career::sim::plan) career: Found,
    pub(in crate::deck::career::sim::plan) lift_stats: Vec<(f64, f64)>,
    pub(in crate::deck::career::sim::plan) origin: RowOrigin,
    pub(in crate::deck::career::sim::plan) exact: bool,
    pub(in crate::deck::career::sim::plan) stats_stamp: Option<(usize, usize)>,
    pub(in crate::deck::career::sim::plan) scoring: Option<RowScoring>,
}

pub(in crate::deck::career::sim::plan) struct RowScoring {
    net_saving: f64,
    pub(in crate::deck::career::sim::plan) improvement_chance: f64,
    /// Can be discarded for a row that cannot win, or taken for the pick's roll state.
    /// The row remains scored in either case.
    pub(in crate::deck::career::sim::plan) kept: Option<KeptScoring>,
    /// Covers every trainee's decks: scoring can read any trio's final trainee.
    stamp: Option<(usize, usize)>,
}

impl PlanRow {
    pub(in crate::deck::career::sim::plan) const fn new(
        trainee: usize,
        career: Found,
        lift_stats: Vec<(f64, f64)>,
        origin: RowOrigin,
        exact: bool,
        stats_stamp: Option<(usize, usize)>,
    ) -> Self {
        Self { trainee, career, lift_stats, origin, exact, stats_stamp, scoring: None }
    }

    pub(in crate::deck::career::sim::plan) fn net_saving(&self) -> Option<f64> {
        self.scoring.as_ref().map(|scoring| scoring.net_saving)
    }

    pub(in crate::deck::career::sim::plan) fn total(&self) -> f64 {
        self.net_saving().unwrap_or(f64::NEG_INFINITY)
    }
}

/// The pick among the rows `runs` (a row without scoring has score -infinity): the highest score among the rows that can
/// be the pick (`vis`; the first such row before any is scored, by more than 1e-9), then within `near_floor` of that top
/// the best odds (a row that can be the pick), moved to the earlier row it repeats (`undup`). (The pick, top.) The one
/// rule for the plan's pick and the roll state kept for it
pub(in crate::deck::career::sim::plan) fn pick_of(
    runs: &[PlanRow],
    vis: impl Fn(&Found) -> bool,
    undup: &[usize],
) -> (usize, f64) {
    let total = |i: usize| runs[i].total();
    let mut pick = (0..runs.len()).find(|&i| vis(&runs[i].career)).unwrap_or(0);
    for i in 0..runs.len() {
        if total(i) > total(pick) + 1e-9 {
            pick = i;
        }
    }
    let top = if runs.is_empty() { 0.0 } else { total(pick) };
    for i in 0..runs.len() {
        if vis(&runs[i].career) && total(i) >= near_floor(top) && runs[i].career.val > runs[pick].career.val {
            pick = i;
        }
    }
    if let Some(&u) = undup.get(pick) {
        pick = u;
    }
    (pick, top)
}

/// The looped rows (below) of `runs`' base rows, in their order.
pub(in crate::deck::career::sim::plan) fn add_looped_rows(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    runs: &[PlanRow],
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> Vec<PlanRow> {
    let mut added: Vec<PlanRow> = vec![];
    let base_s = candidates.stats_base();
    let PlanInputs { d, legs, pool, fix, pj, .. } = *input;
    let pick_pool = input.pick_pool();
    let nl = legs.len();
    let sj = s.env.sj;
    let is_runnable = |f: &Found| input.is_runnable(f);
    // Looped careers: per uma whose career above has parents that are not both 8/8s, her best career on two real 8/8
    // veterans (a legal pair, all of `fix` among them), a row of its own beside it. Only there is her own 8/8 a fully
    // looped parent (the goal), and the trios' last step is such a career: without these rows the career V itself
    // prices (C on A's and B's 8/8s) was never among the options, and a uma whose best-odds pair happened to be two 8/8s
    // took the goal's credit alone (inputs of 2026-10-17: Taiki Shuttle 0.454 over Inari One's looped career, 1.000).
    // Exact (the surrogate's under `sur` 1); pairs by ceiling, stopping below the best found. Not among Cont's builders
    // (Cont is fitted on each uma's best career) and not added to the cache (what the tables read stays the job's
    // winners), so V and every other row are as before.
    let looped = |f: &Found| is_full(sj, &legs[f.a].sparks) && is_full(sj, &legs[f.b].sparks);
    // Per uma her best looped career where searched (`Unsearched` or `Known`, including no legal pair), kept with
    // the plan's best careers: a kept plan of this very pool gives it as kept; one of this pool less its last veteran
    // gives it as kept unless she is an 8/8 (then only her pairs are new: the better of the kept one and those, a tie
    // keeping the kept one)
    searches.looped_careers = vec![CachedSearch::Unsearched; d.umas.len()];
    progress(3, 0, 0); // the looped careers
    if fix.len() < 2 {
        let full8: Vec<usize> = (0..nl).filter(|&i| is_full(sj, &legs[i].sparks)).collect();
        let is8: Vec<bool> = (0..nl).map(|i| full8.contains(&i)).collect();
        let her8 = nl > 0 && full8.contains(&(nl - 1));
        let ts = now_ms();
        let lnodes0 = s.env.space.nodes.get();
        capped(s, pj.search_work, searches.original_budget, |s| {
            for row in runs {
                if row.origin != RowOrigin::Base {
                    continue;
                }
                let x = row.trainee;
                if looped(&row.career) && is_runnable(&row.career) {
                    continue;
                }
                let kept = |b: &Option<Bests>| {
                    b.as_ref().and_then(|b| match b.looped.get(x) {
                        Some(CachedSearch::Known(found)) => Some(found.clone()),
                        _ => None,
                    })
                };
                let kept_incomplete =
                    |b: &Option<Bests>| b.as_ref().is_some_and(|b| b.lunp.get(x).copied().unwrap_or(false));
                let lf: Option<Found> = if let Some(v) = kept(&searches.kept_same_pool) {
                    searches.looped_incomplete[x] = kept_incomplete(&searches.kept_same_pool);
                    v
                } else {
                    let start = kept(&searches.kept_previous_pool);
                    let only_her = start.is_some();
                    if only_her {
                        searches.looped_incomplete[x] = kept_incomplete(&searches.kept_previous_pool);
                    }
                    if only_her && !her8 {
                        start.flatten()
                    } else {
                        // (the legal pairs of two 8/8s, only those with her when the kept plan had the rest; `pick_pool` and
                        // `pool` differ in `max_bor` alone, which the ceilings do not read)
                        let cs = s
                            .env
                            .ranked_pairs(pick_pool, fix, x, |a, b| is8[a] && is8[b] && (!only_her || b == nl - 1));
                        let mut lf: Option<Found> = start.flatten();
                        for (ub, a, b) in cs {
                            if ub <= lf.as_ref().map_or(0.0, |f| f.val) {
                                break;
                            }
                            // (the exact search's bar starts at the best looped career so far: only a career above it can
                            // replace it, as in `best_of`; a tie keeps the earlier pair either way)
                            let f = if pj.sur == 1 {
                                best_sur(s, pick_pool, &[a, b], x, pj.sur_margin, buf)
                            } else {
                                s.env.floor = lf.as_ref().map_or(f64::NEG_INFINITY, |f| f.val);
                                let sr = s.env.solve(pick_pool, &[a, b], &[x], false, 1);
                                if !sr.complete {
                                    searches.looped_incomplete[x] = true;
                                }
                                s.env.floor = f64::NEG_INFINITY;
                                sr.found.into_iter().next()
                            };
                            if let Some(f) = f
                                && f.val > lf.as_ref().map_or(0.0, |found| found.val)
                            {
                                lf = Some(f);
                            }
                        }
                        lf
                    }
                };
                searches.looped_careers[x] = CachedSearch::Known(lf.clone());
                if let Some(f) = lf {
                    // (its stats feed only `proxy_check`; without it the scoring runs for its state effects alone: its `val`
                    // searches, under this block's cap, add decks later scorings read)
                    let want = if pj.proxy_check { Want::Stats } else { Want::Touch };
                    let CareerScore { lift_stats: st, .. } =
                        score_career(s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, want, buf);
                    added.push(PlanRow::new(x, f, st, RowOrigin::Looped, true, None));
                }
            }
        });
        profile.search_ms += now_ms() - ts;
        prof("looped_ms", now_ms() - ts);
        prof("looped_nodes", (s.env.space.nodes.get() - lnodes0) as f64);
    }
    added
}

/// The no-borrow rows (below) of `runs`' base rows, in their order.
pub(in crate::deck::career::sim::plan) fn add_no_borrow_rows(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    runs: &[PlanRow],
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> Vec<PlanRow> {
    let mut added: Vec<PlanRow> = vec![];
    let PlanInputs { fix, pick, .. } = *input;
    let pick_pool = input.pick_pool();
    let n = candidates.trainees.len();
    let is_runnable = |career: &Found| input.is_runnable(career);
    // nb rows (`pick`): per base row whose career has a borrowed parent, her best career without one, a row of its own:
    // through her cached decks (`best_sur`, as a builder's proxy) while proxies are on, searched exactly once a shown
    // row or the pick's band can turn on it (pass 2); exact from the start when proxies are off. Like the looped rows
    // neither Cont's builders nor added to the cache; no lifts (zeros: only Cont and `proxy_check` read a row's lifts; the
    // looped rows' are zeros too unless `proxy_check`)
    if pick.is_some() {
        let ts = now_ms();
        for row in runs {
            if row.origin != RowOrigin::Base {
                continue;
            }
            if is_runnable(&row.career) {
                continue;
            }
            let trainee = row.trainee;
            let (career, is_exact) = if candidates.proxy_on {
                (best_sur(s, pick_pool, fix, trainee, 1e9, buf), false)
            } else {
                (exact_no_borrow_career(s, input, searches, trainee, buf), true)
            };
            if let Some(career) = career {
                added.push(PlanRow::new(trainee, career, vec![(0.0, 0.0); n], RowOrigin::NoBorrow, is_exact, None));
            }
        }
        profile.search_ms += now_ms() - ts;
    }
    added
}

pub(in crate::deck::career::sim::plan) fn score_and_promote_rows(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    searches: &mut PlanSearches,
    runs: &mut Vec<PlanRow>,
    wins_by_trainee: &mut [Vec<usize>],
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> PlanScores {
    let PlanInputs { d, legs, pool, fix, .. } = *input;
    let pick_pool = input.pick_pool();
    let n = candidates.trainees.len();
    let sj = s.env.sj;
    let is_runnable = |f: &Found| input.is_runnable(f);
    // each candidate's step with building: her most efficient builder now (rate, log-odds step), her full-loop ceiling with
    // her two most efficient builders (among the exactly searched careers)
    let continuations = plan_continuations(s, input, candidates, runs, wins_by_trainee, buf);
    // a fresh 8/8 of hers (her step); a real 8/8 of hers costs nothing (the trios' side choice)
    let step_costs: Vec<f64> = (0..n).map(|ui| continuations[ui].cost(candidates.pnow[ui])).collect();
    // the trios: A and B two candidates (unordered), C the best allowed uma with an 8/8 of A and one of B as parents. Each
    // side: a real 8/8 veteran of hers (free), or a fresh one, her step, the 8/8 her best career now would leave
    // (`Env::veteran`: its parents, every target at the rank's star mix, her G1 wins at their chances), whichever trio is
    // cheaper; C's 8/8 is then a fully looped parent
    let t_c = now_ms();
    let real8: Vec<Vec<usize>> = candidates
        .trainees
        .iter()
        .map(|&u| {
            (0..legs.len())
                .filter(|&i| {
                    legs[i].chara == d.umas[u].id && is_full(sj, &legs[i].sparks) && pool.bor_one(legs[i].borrowed)
                })
                .collect()
        })
        .collect();
    let mut combos = plan_combinations(s, input, candidates, &real8, &step_costs, buf);
    let sides = plan_sides(s, input, candidates, &real8, &step_costs);
    let mut combo_ms = now_ms() - t_c;
    let trios: Vec<Trio> = combos.iter().map(PlanCombination::as_trio).collect();
    let current_value = trio_v(&step_costs, &trios);
    let mut fit = PlanFit { continuations, step_costs, trios, sides, current_value };
    // pass 2: every career scored against V (exact over her outcomes): E[V - V with her veteran] - 1
    // (a row that is never the pick, a builder with a borrowed parent under "later", is not scored: no score, no kept scoring)
    progress(4, 0, runs.len()); // pass 2: every career against V
    if !runs.is_empty() && fit.current_value.is_finite() {
        for i in 0..runs.len() {
            let ((net_score, lift, kept), stamp) =
                stamped(s, None, |s| score_plan_row(s, input, candidates, &fit, &runs[i], buf));
            runs[i].scoring = Some(RowScoring { net_saving: net_score, improvement_chance: lift, kept, stamp });
            prune_plan_scorings(input, runs);
        }
        // the proxies a shown row or the pick's near-tie band can turn on: searched exactly (their
        // saving, score + 1, within PROXY_SAVE of the lowest saving those take), then scored again; when one of them joins a
        // candidate's two most efficient builders, Cont, the trios, V and every score are computed again
        #[allow(clippy::while_immutable_condition, reason = "proxy_on is fixed per call; the loop ends at its break")]
        while candidates.proxy_on {
            // (over the rows that can be shown or picked, `is_runnable`)
            let mut sv: Vec<f64> =
                runs.iter().filter(|r| is_runnable(&r.career)).filter_map(PlanRow::net_saving).collect();
            sv.sort_by(|x, y| y.partial_cmp(x).unwrap());
            if sv.is_empty() {
                break;
            }
            let (top, k8) = (sv[0], sv[(OPT_SHOW - 1).min(sv.len() - 1)]);
            let t = k8.min(near_floor(top)) + 1.0;
            let add: Vec<usize> = (0..runs.len())
                .filter(|&i| {
                    !runs[i].exact && is_runnable(&runs[i].career) && runs[i].total() + 1.0 >= t * (1.0 - PROXY_SAVE)
                })
                .collect();
            if add.is_empty() {
                break;
            }
            let ts = now_ms();
            // (the careers scored: a row whose search found the same one, or that was certified, may keep its score)
            let old: Vec<Found> = add.iter().map(|&i| runs[i].career.clone()).collect();
            for (k, &i) in add.iter().enumerate() {
                let x = runs[i].trainee;
                // an nb row: her best career without a borrowed parent, certified or searched on the pick's pool
                if runs[i].origin == RowOrigin::NoBorrow {
                    if certified(s.env, pick_pool, fix, x, &runs[i].career) {
                        profile.n_cert += 1;
                    } else if let Some(f) = exact_no_borrow_career(s, input, searches, x, buf) {
                        runs[i].career = f;
                    }
                    runs[i].exact = true;
                    continue;
                }
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
                    set_plan_wins(s, input, wins_by_trainee, x, &f);
                    runs[i].career = f;
                    runs[i].lift_stats = st;
                }
                runs[i].exact = true;
                runs[i].stats_stamp = None;
                // ("later") her best career has a borrowed parent: that row is a builder now, and the proxy's career,
                // without one, her nb row (scored below)
                if !is_runnable(&runs[i].career) {
                    runs.push(PlanRow::new(x, old[k].clone(), vec![(0.0, 0.0); n], RowOrigin::NoBorrow, false, None));
                }
            }
            profile.search_ms += now_ms() - ts;
            let next_continuations = plan_continuations(s, input, candidates, runs, wins_by_trainee, buf);
            let same_fit = next_continuations
                .iter()
                .zip(&fit.continuations)
                .all(|(a, b)| a.lam == b.lam && a.phi == b.phi && a.pinf == b.pinf);
            if same_fit {
                for (k, &i) in add.iter().enumerate() {
                    // the same career against the same base (Cont, so step_costs, V, the trios and sides, as when it was scored),
                    // nothing it reads changed: its score, lift and kept scoring stand (a dropped one stays dropped: its score
                    // is as low as it was, and lo only rises here)
                    if same_career(&old[k], &runs[i].career)
                        && stamp_ok(s, None, runs[i].scoring.as_ref().unwrap().stamp)
                    {
                        prof("reuse_sc", 1.0);
                        continue;
                    }
                    let ((net_score, lift, kept), stamp) =
                        stamped(s, None, |s| score_plan_row(s, input, candidates, &fit, &runs[i], buf));
                    runs[i].scoring = Some(RowScoring { net_saving: net_score, improvement_chance: lift, kept, stamp });
                }
                // (the nb rows added above)
                for i in 0..runs.len() {
                    if runs[i].scoring.is_some() {
                        continue;
                    }
                    let ((net_score, lift, kept), stamp) =
                        stamped(s, None, |s| score_plan_row(s, input, candidates, &fit, &runs[i], buf));
                    runs[i].scoring = Some(RowScoring { net_saving: net_score, improvement_chance: lift, kept, stamp });
                }
                prune_plan_scorings(input, runs);
            } else {
                memo_stat(6);
                fit.continuations = next_continuations;
                fit.step_costs = (0..n).map(|ui| fit.continuations[ui].cost(candidates.pnow[ui])).collect();
                let tc = now_ms();
                combos = plan_combinations(s, input, candidates, &real8, &fit.step_costs, buf);
                fit.sides = plan_sides(s, input, candidates, &real8, &fit.step_costs);
                combo_ms += now_ms() - tc;
                fit.trios = combos.iter().map(PlanCombination::as_trio).collect();
                fit.current_value = trio_v(&fit.step_costs, &fit.trios);
                if !fit.current_value.is_finite() {
                    for run in &mut *runs {
                        run.scoring = None;
                    }
                    break;
                }
                // (every row scored again: the old kept scorings go first, the new ones kept as above)
                for run in &mut *runs {
                    run.scoring = None;
                }
                for i in 0..runs.len() {
                    let ((net_score, lift, kept), stamp) =
                        stamped(s, None, |s| score_plan_row(s, input, candidates, &fit, &runs[i], buf));
                    runs[i].scoring = Some(RowScoring { net_saving: net_score, improvement_chance: lift, kept, stamp });
                    prune_plan_scorings(input, runs);
                }
            }
        }
        profile.exact_n = runs.iter().filter(|r| r.scoring.is_some() && is_runnable(&r.career)).count();
    }
    let best_combo = combos
        .iter()
        .min_by(|x, y| x.total_cost(&fit.step_costs).partial_cmp(&y.total_cost(&fit.step_costs)).unwrap())
        .copied();
    let index_ms = now_ms() - profile.t3;
    let work = json!({"nodes": s.env.space.nodes.get() - profile.nodes0, "exact": crate::deck::JCOUNT.with(std::cell::Cell::get) - profile.joints0, "max_ratio": s.env.max_ratio.iter().copied().fold(0.0f64, f64::max), "class_max_ratio": s.env.max_ratio, "class_max_deck": s.env.max_decks(), "cap": s.env.space.rho_top().min(s.env.cert), "class_caps": s.env.space.caps()});
    let any_scored = runs.iter().any(|r| r.scoring.is_some());
    PlanScores { fit, best_combo, any_scored, combo_ms, index_ms, work }
}

pub(in crate::deck::career::sim::plan) fn check_plan_proxies(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    best: &[CachedSearch],
    runs: &[PlanRow],
    scores: &PlanScores,
    cache0: Option<super::Cache>,
    buf: &mut Vec<f64>,
) -> Value {
    let PlanInputs { d, legs, pool, sv, fix, pj, .. } = *input;
    let n = candidates.trainees.len();
    // measurement (`proxy_check`): each builder's career from her deck class's winning decks (the main search's and the
    // candidates' best careers of her class, legal for her and valued at her own chances) through the surrogate, no career
    // search; its lifts per candidate (efficiency, as Cont takes them) and its score beside the exact ones. Runs after
    // everything else, on the cache as it was before the builders' searches, so the plan's own numbers are unchanged
    match (pj.proxy_check, cache0) {
        (true, Some(mut c0)) if !runs.is_empty() => {
            std::mem::swap(&mut s.cache, &mut c0);
            let wins: Vec<(usize, Sel)> = sv
                .found
                .iter()
                .map(|f| (f.t, f.sel.clone()))
                .chain(candidates.trainees.iter().filter_map(|&u| best[u].found().cloned().map(|f| (u, f.sel))))
                .collect();
            let base_s = candidates.stats_base();
            let base = scores.fit.score_base(candidates);
            let effs = |x: usize, st: &[(f64, f64)]| -> Vec<f64> {
                (0..n)
                    .map(|ui| {
                        let (pl, sp) = st[ui];
                        if d.umas[candidates.trainees[ui]].id == d.umas[x].id || pl <= 0.0 || candidates.pnow[ui] <= 0.0
                        {
                            0.0
                        } else {
                            lift_eff(pl, sp, candidates.pnow[ui])
                        }
                    })
                    .collect()
            };
            let mut rows = vec![];
            for r in runs {
                let x = r.trainee;
                let is_c = candidates.trainees.contains(&x);
                let ee = effs(x, &r.lift_stats);
                let (pf, pe, psc) = if is_c {
                    (Some(r.career.clone()), ee.clone(), r.net_saving())
                } else {
                    let cx = s.env.cls[x];
                    for (t, sel) in &wins {
                        if *t != x && s.env.cls[*t] == cx {
                            s.cache.add(s.env, x, sel);
                        }
                    }
                    match best_sur(s, pool, fix, x, 1e9, buf) {
                        None => (None, vec![0.0; n], None),
                        Some(f) => {
                            let CareerScore { lift_stats: st, .. } =
                                score_career(s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, Want::Stats, buf);
                            let pe = effs(x, &st);
                            let psc = scores.any_scored.then(|| {
                                score_career(s, d, &base, legs, pool, x, f.a, f.b, &f.sel, Want::Score, buf)
                                    .expected_saving
                                    - 1.0
                            });
                            (Some(f), pe, psc)
                        }
                    }
                };
                let same_pair = pf.as_ref().is_some_and(|f| {
                    (f.a.min(f.b), f.a.max(f.b)) == (r.career.a.min(r.career.b), r.career.a.max(r.career.b))
                });
                let same_deck = pf.as_ref().is_some_and(|f| {
                    f.sel.ents == r.career.sel.ents
                        && f.sel.plain == r.career.sel.plain
                        && f.sel.filler == r.career.sel.filler
                });
                rows.push(json!({"trainee": d.umas[x].name, "cand": is_c, "class": s.env.cls[x], "exact_val": r.career.val, "proxy_val": pf.as_ref().map(|f| f.val),
                "same_pair": same_pair, "same_deck": same_deck, "score": r.net_saving(), "proxy_score": psc, "eff": ee, "proxy_eff": pe}));
            }
            std::mem::swap(&mut s.cache, &mut c0);
            json!({"cands": candidates.trainees.iter().map(|&u| d.umas[u].name.clone()).collect::<Vec<_>>(), "rows": rows})
        }
        _ => Value::Null,
    }
}

fn score_plan_row(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    fit: &PlanFit,
    r: &PlanRow,
    buf: &mut Vec<f64>,
) -> (f64, f64, Option<KeptScoring>) {
    let PlanInputs { d, legs, pool, .. } = *input;
    let is_runnable = |f: &Found| input.is_runnable(f);

    if !is_runnable(&r.career) {
        return (f64::NEG_INFINITY, 0.0, None);
    }
    let base = fit.score_base(candidates);
    let CareerScore { expected_saving, improvement_chance, kept, .. } =
        score_career(s, d, &base, legs, pool, r.trainee, r.career.a, r.career.b, &r.career.sel, Want::Keep, buf);
    (expected_saving - 1.0, improvement_chance, kept)
}

// Each row's kept scoring (`Scored`: what a roll of the pick reads, its tables and outcome chances) is kept only
// while the row can still be the pick. The pick (below) is a row within NEAR.max(0.05 |top|) of top, top at least the
// highest score less 1e-9 (the argmax's tolerance), or the base row a looped pick repeats (`undup`). A row's score
// changes only when it is scored again, which keeps a fresh one, and an exactly searched row is scored again only
// when every row is (a refit). So lo, the highest score of an exact row among the rows scored since every row last
// was, is at most the final top; thr(t) = t - NEAR.max(0.05 |t|) rises with t; a row below thr(lo) (less a slack far
// above its rounding) is not near the final top unless it is scored again. Kept also: every row a later row repeats
// (same uma and pair: a looped or nb row the pick moves from; a row whose career changes is scored again). Only the
// pick's is read (`RollKeep`); lo is over the rows that can be the pick (`is_runnable`).
fn prune_plan_scorings(input: &PlanInputs, runs: &mut [PlanRow]) {
    let is_runnable = |f: &Found| input.is_runnable(f);

    let lo = runs
        .iter()
        .filter(|r| r.exact && is_runnable(&r.career))
        .filter_map(PlanRow::net_saving)
        .fold(f64::NEG_INFINITY, f64::max);
    if !lo.is_finite() {
        return;
    }
    let thr = near_floor(lo) - 1e-6 * (1.0 + lo.abs());
    let pr = |f: &Found| (f.a.min(f.b), f.a.max(f.b));
    for i in 0..runs.len() {
        let Some(scoring) = &runs[i].scoring else { continue };
        if scoring.kept.is_none() || !(scoring.net_saving < thr) {
            continue;
        }
        if (i + 1..runs.len()).any(|k| runs[k].trainee == runs[i].trainee && pr(&runs[k].career) == pr(&runs[i].career))
        {
            continue;
        }
        runs[i].scoring.as_mut().unwrap().kept = None;
    }
}
