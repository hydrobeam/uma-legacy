//! Plan rows, their scoring and promotion.
use crate::career::cache::Cache;
use crate::career::clock::{now_ms, prof, progress};
use crate::career::plan::candidates::{
    MadeExact, class_winners, exact_no_borrow_career, make_row_exact, proxy_career, set_plan_wins,
};
use crate::career::plan::combine::{
    PlanCombination, PlanFit, PlanScores, plan_combinations, plan_continuations, plan_sides,
};
use crate::career::plan::cont::{Cont, Trio, Want, count_refit, trio_v};
use crate::career::plan::kept::{Bests, CacheStamp, CachedSearch, same_career, stamp_ok, stamped};
use crate::career::plan::score::kernel::LiftStats;
use crate::career::plan::score::{CareerScore, KeptScoring, score_career};
use crate::career::plan::search::{OPT_SHOW, PROXY_SAVE, best_sur, capped, certified, is_full};
use crate::career::plan::{
    ParentPool, PlanContext, PlanInputs, PlanProfile, PlanRows, PlanSearches, found_career, found_lineage,
};
use crate::career::search::{Found, RankedPair};
use crate::career::sim::Sim;
use crate::career::{Career, Legacy, Lineage};
use crate::skills::SkillJob;
use serde_json::{Value, json};
/// Scores within this many careers per run of the best (or 5% of it) count as tied
const NEAR: f64 = 0.25;

/// the lowest score tied with `top` (`NEAR`)
pub(crate) fn near_floor(top: f64) -> f64 {
    top - NEAR.max(0.05 * top.abs())
}

/// Why a career was added to the planner's options. Rows retain their insertion order:
/// base careers, looped alternatives, then alternatives without borrowed parents.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowOrigin {
    Base,
    Looped,
    NoBorrow,
    /// Main-search options used when no planner row can be picked. Not a planner search.
    Fallback,
}

/// A career and its planner state. Replacing the career preserves the previous scoring
/// until pass 2 either reuses it (same career and valid stamp) or scores it again.
pub(crate) struct PlanRow {
    /// The career's trainee (`Data::umas`).
    pub(crate) trainee: crate::uma::UmaIndex,
    pub(crate) career: Found,
    /// Per candidate (`PlanCandidates::trainees` order): P(her veteran lifts the candidate's odds) and the summed
    /// lifted odds (`CareerScore::lift_stats`); zeros where only Cont and `proxy_check` would read them.
    pub(crate) lift_stats: Vec<LiftStats>,
    pub(crate) origin: RowOrigin,
    /// The career was searched exactly (or certified); false: a proxy from her deck class's winning decks.
    pub(crate) exact: bool,
    /// The cache a proxy's lifts read (`stamped`), for reuse when her exact search finds the same career.
    pub(crate) stats_stamp: Option<CacheStamp>,
    pub(crate) scoring: Option<RowScoring>,
}

/// A row's pass-2 score against the fitted V.
pub(crate) struct RowScoring {
    /// E[V - V with her veteran] - 1, in careers; -infinity for a row that cannot be the pick.
    net_saving: f64,
    pub(crate) improvement_chance: f64,
    /// Can be discarded for a row that cannot win, or taken for the pick's roll state.
    /// The row remains scored in either case.
    pub(crate) kept: Option<KeptScoring>,
    /// Covers every trainee's decks: scoring can read any trio's final trainee.
    stamp: Option<CacheStamp>,
}

impl PlanRow {
    /// A row whose career was searched exactly.
    pub(crate) const fn exact(
        trainee: crate::uma::UmaIndex,
        career: Found,
        lift_stats: Vec<LiftStats>,
        origin: RowOrigin,
    ) -> Self {
        Self { trainee, career, lift_stats, origin, exact: true, stats_stamp: None, scoring: None }
    }

    /// A row whose career is a proxy (or, for `RowOrigin::Fallback`, the main search's), its lifts' stamp if any.
    pub(crate) const fn proxy(
        trainee: crate::uma::UmaIndex,
        career: Found,
        lift_stats: Vec<LiftStats>,
        origin: RowOrigin,
        stats_stamp: Option<CacheStamp>,
    ) -> Self {
        Self { trainee, career, lift_stats, origin, exact: false, stats_stamp, scoring: None }
    }

    pub(crate) fn net_saving(&self) -> Option<f64> {
        self.scoring.as_ref().map(|scoring| scoring.net_saving)
    }

    pub(crate) fn total(&self) -> f64 {
        self.net_saving().unwrap_or(f64::NEG_INFINITY)
    }

    /// The row's trainee and her career's parents. Derived, not stored: exact searches replace the career in place.
    pub(crate) const fn lineage(&self) -> Lineage {
        found_lineage(self.trainee, &self.career)
    }

    /// The row's career as `score_career` takes it.
    pub(crate) const fn scored_career(&self) -> Career<'_> {
        found_career(self.trainee, &self.career)
    }

    /// The same trainee on the same parents (in either order): a row repeating another.
    pub(crate) fn repeats(&self, other: &Self) -> bool {
        self.trainee == other.trainee && self.career.unordered_parents() == other.career.unordered_parents()
    }
}

/// Both parents of `career` are full veterans: her full veteran from it is a fully looped parent
pub(crate) fn has_looped_parents(sj: &SkillJob, legacies: &[Legacy], career: &Found) -> bool {
    is_full(sj, &legacies[career.first_parent].sparks) && is_full(sj, &legacies[career.second_parent].sparks)
}

/// The plan's pick among its rows, and the top score it was taken within `near_floor` of.
#[derive(Clone, Copy)]
pub(crate) struct PlanPick {
    pub(crate) row: usize,
    pub(crate) top: f64,
}

/// The pick among the rows `runs` (a row without scoring has score -infinity): the highest score among the rows that can
/// be the pick (`vis`; the first such row before any is scored, by more than 1e-9), then within `near_floor` of that top
/// the best odds (a row that can be the pick), moved to the earlier row it repeats (`undup`). The one rule for the
/// plan's pick and the roll state kept for it
pub(crate) fn pick_of(runs: &[PlanRow], vis: impl Fn(&Found) -> bool, undup: &[usize]) -> PlanPick {
    let total = |i: usize| runs[i].total();
    let mut pick = (0..runs.len()).find(|&i| vis(&runs[i].career)).unwrap_or(0);
    for i in 0..runs.len() {
        if total(i) > total(pick) + 1e-9 {
            pick = i;
        }
    }
    let top = if runs.is_empty() { 0.0 } else { total(pick) };
    for i in 0..runs.len() {
        if vis(&runs[i].career) && total(i) >= near_floor(top) && runs[i].career.value > runs[pick].career.value {
            pick = i;
        }
    }
    if let Some(&u) = undup.get(pick) {
        pick = u;
    }
    PlanPick { row: pick, top }
}

/// The looped rows (below) of `runs`' base rows, in their order.
pub(crate) fn add_looped_rows(
    simulator: &mut Sim,
    context: &PlanContext,
    runs: &[PlanRow],
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> Vec<PlanRow> {
    let mut added: Vec<PlanRow> = vec![];
    let PlanContext { input, candidates } = *context;
    let stats_base = candidates.stats_base();
    let legacies = input.legacies;
    let legacy_count = legacies.len();
    let sj = simulator.env.skill_job;
    // Looped careers: per uma whose career above has parents that are not both full veterans, her best career on two real
    // full veterans (a legal pair, all of `fix` among them), a row of its own beside it. Only there is her full veteran a fully
    // looped parent (the goal), so these rows make the trios' runnable last steps available as options.
    // Exact (the surrogate's under `sur` 1); pairs by ceiling, stopping below the best found. Not among Cont's builders
    // (Cont is fitted on each uma's best career) and not added to the cache (what the tables read stays the job's
    // winners). Their scoring preparation can still search for decks that later scorings read.
    // Per uma her best looped career where searched (`Unsearched` or `Known`, including no legal pair), kept with
    // the plan's best careers: a kept plan of this very pool gives it as kept; one of this pool less its last veteran
    // gives it as kept unless she is a full veteran (then only her pairs are new: the better of the kept one and those, a tie
    // keeping the kept one)
    searches.looped_careers = crate::uma::UmaTable(vec![CachedSearch::Unsearched; context.data().umas.len()]);
    progress(3, 0, 0); // the looped careers
    if input.fixed_parents.len() >= 2 {
        return added;
    }
    let full_veteran_indices: Vec<usize> = (0..legacy_count).filter(|&i| is_full(sj, &legacies[i].sparks)).collect();
    let full_veterans: Vec<bool> = (0..legacy_count).map(|i| full_veteran_indices.contains(&i)).collect();
    let looped_started_ms = now_ms();
    let nodes_before = simulator.env.space.nodes.get();
    capped(simulator, input.settings.search_work, searches.env_budget, |simulator| {
        for row in runs {
            if row.origin != RowOrigin::Base {
                continue;
            }
            let x = row.trainee;
            if has_looped_parents(sj, legacies, &row.career) && input.is_runnable(&row.career) {
                continue;
            }
            let looped = looped_career(simulator, context, searches, x, &full_veterans, buf);
            searches.looped_careers[x] = CachedSearch::Known(looped.clone());
            if let Some(career) = looped {
                // (its stats feed only `proxy_check`; without it the scoring runs for its state effects alone: its `val`
                // searches, under this block's cap, add decks later scorings read)
                let want = if input.settings.proxy_check { Want::Stats } else { Want::Touch };
                let CareerScore { lift_stats, .. } =
                    score_career(simulator, &context.scoring(&stats_base), found_career(x, &career), want, buf);
                added.push(PlanRow::exact(x, career, lift_stats, RowOrigin::Looped));
            }
        }
    });
    profile.search_ms += now_ms() - looped_started_ms;
    prof("looped_ms", now_ms() - looped_started_ms);
    prof("looped_nodes", (simulator.env.space.nodes.get() - nodes_before) as f64);
    added
}

/// A trainee's looped career as a kept plan holds it.
struct KeptLooped {
    career: Option<Found>,
    incomplete: bool,
}

fn kept_looped(kept: Option<&Bests>, trainee: crate::uma::UmaIndex) -> Option<KeptLooped> {
    let kept = kept?;
    match kept.looped_careers.get(trainee) {
        Some(CachedSearch::Known(found)) => Some(KeptLooped {
            career: found.clone(),
            incomplete: kept.looped_incomplete.get(trainee).copied().unwrap_or(false),
        }),
        _ => None,
    }
}

/// `trainee`'s best career on two real full veterans as parents: as kept, else the legal pairs of full veterans searched
/// (only those with the newest veteran when a kept plan of the pool less her had the rest), by ceiling, the exact
/// search's bar at the best so far
fn looped_career(
    simulator: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    trainee: crate::uma::UmaIndex,
    is_full: &[bool],
    buf: &mut Vec<f64>,
) -> Option<Found> {
    let settings = context.settings();
    if let Some(kept) = kept_looped(searches.kept_same_pool.as_ref(), trainee) {
        searches.looped_incomplete[trainee] = kept.incomplete;
        return kept.career;
    }
    let start = kept_looped(searches.kept_previous_pool.as_ref(), trainee);
    let only_newest = start.is_some();
    if let Some(kept) = &start {
        searches.looped_incomplete[trainee] = kept.incomplete;
    }
    let start = start.and_then(|kept| kept.career);
    if only_newest && !is_full.last().copied().unwrap_or(false) {
        return start;
    }
    // (`pick_pool` and `pool` differ in `max_borrowed` alone, which the ceilings do not read)
    let ParentPool { pool: pick_pool, fixed_parents } = context.pick_parents();
    let newest = context.legacy_count().wrapping_sub(1);
    let pairs = simulator.env.ranked_pairs(pick_pool, fixed_parents, trainee, |a, b| {
        is_full[a] && is_full[b] && (!only_newest || b == newest)
    });
    let mut best = start;
    for RankedPair { ceiling, first_parent: a, second_parent: b } in pairs {
        if ceiling <= best.as_ref().map_or(0.0, |f| f.value) {
            break;
        }
        let pair = [a, b];
        // (the exact search's bar starts at the best looped career so far: only a career above it can
        // replace it, as in `best_of`; a tie keeps the earlier pair either way)
        let found = if settings.sur == 1 {
            best_sur(simulator, ParentPool { pool: pick_pool, fixed_parents: &pair }, trainee, settings.sur_margin, buf)
        } else {
            simulator.env.floor = best.as_ref().map_or(f64::NEG_INFINITY, |f| f.value);
            let searched = simulator.env.solve(pick_pool, &pair, &[trainee], false, 1);
            if !searched.complete {
                searches.looped_incomplete[trainee] = true;
            }
            simulator.env.floor = f64::NEG_INFINITY;
            searched.found.into_iter().next()
        };
        if let Some(found) = found
            && found.value > best.as_ref().map_or(0.0, |f| f.value)
        {
            best = Some(found);
        }
    }
    best
}

/// The no-borrow rows (below) of `runs`' base rows, in their order.
pub(crate) fn add_no_borrow_rows(
    simulator: &mut Sim,
    context: &PlanContext,
    runs: &[PlanRow],
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> Vec<PlanRow> {
    let mut added: Vec<PlanRow> = vec![];
    let PlanContext { input, candidates } = *context;
    let candidate_count = candidates.trainees.len();
    // nb rows (`pick_search`): per base row whose career has a borrowed parent, her best career without one, a row of
    // its own: through her cached decks (`best_sur`, as a builder's proxy) while proxies are on, searched exactly once
    // a shown row or the pick's band can turn on it (pass 2); exact from the start when proxies are off. Like the
    // looped rows neither Cont's builders nor added to the cache; no lifts (zeros: only Cont and `proxy_check` read a
    // row's lifts; the looped rows' are zeros too unless `proxy_check`)
    if input.pick_search.is_none() {
        return added;
    }
    let started_ms = now_ms();
    for row in runs {
        if row.origin != RowOrigin::Base || input.is_runnable(&row.career) {
            continue;
        }
        let trainee = row.trainee;
        let career = if candidates.proxies_enabled {
            best_sur(simulator, context.pick_parents(), trainee, 1e9, buf)
        } else {
            exact_no_borrow_career(simulator, context, searches, trainee, buf)
        };
        let Some(career) = career else { continue };
        let no_lifts = vec![LiftStats::default(); candidate_count];
        added.push(if candidates.proxies_enabled {
            PlanRow::proxy(trainee, career, no_lifts, RowOrigin::NoBorrow, None)
        } else {
            PlanRow::exact(trainee, career, no_lifts, RowOrigin::NoBorrow)
        });
    }
    profile.search_ms += now_ms() - started_ms;
    added
}

/// The fit's trios with their finishers, and the time spent on them.
struct FittedTrios {
    fit: PlanFit,
    combinations: Vec<PlanCombination>,
    combination_ms: f64,
}

/// The trios under `continuations`: A and B two candidates (unordered), C the best allowed uma with a full veteran of A
/// and one of B as parents. Each side: a real full veteran of hers (free), or a fresh full veteran from her best career
/// now, costing her step (`Env::veteran`: its parents, every target at the rank's star mix, her G1 wins at their chances),
/// whichever trio is cheaper; C's full veteran is then a fully looped parent
fn fit_trios(
    simulator: &mut Sim,
    context: &PlanContext,
    continuations: Vec<Cont>,
    full_veterans: &[Vec<usize>],
    buf: &mut Vec<f64>,
) -> FittedTrios {
    let candidates = context.candidates;
    // a fresh full veteran of hers (her step); a real full veteran of hers costs nothing (the trios' side choice)
    let step_costs: Vec<f64> =
        (0..candidates.trainees.len()).map(|ui| continuations[ui].cost(candidates.current_odds[ui])).collect();
    let started_ms = now_ms();
    let combinations = plan_combinations(simulator, context, full_veterans, &step_costs, buf);
    let sides = plan_sides(simulator, context, full_veterans, &step_costs);
    let combination_ms = now_ms() - started_ms;
    let trios: Vec<Trio> = combinations.iter().map(PlanCombination::as_trio).collect();
    let current_value = trio_v(&step_costs, &trios);
    FittedTrios {
        fit: PlanFit { continuations, step_costs, trios, sides, current_value },
        combinations,
        combination_ms,
    }
}

/// Pass 2: the continuations fitted, then every row scored against V, the proxies that can turn the shown rows or the
/// pick searched exactly, and the fit and scores redone while that changes Cont
pub(crate) fn score_and_promote_rows(
    simulator: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    plan_rows: &mut PlanRows,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> PlanScores {
    let PlanContext { input, candidates } = *context;
    let legacies = input.legacies;
    let data = context.data();
    let sj = simulator.env.skill_job;
    // each candidate's step with building: her most efficient builder now (rate, log-odds step), her full-loop ceiling with
    // her two most efficient builders (among the exactly searched careers)
    let continuations = plan_continuations(simulator, context, &plan_rows.rows, &plan_rows.wins_by_trainee, buf);
    // per candidate, her real full veterans this pool may use (legacy indices)
    let full_veterans: Vec<Vec<usize>> = candidates
        .trainees
        .iter()
        .map(|&u| {
            (0..legacies.len())
                .filter(|&i| {
                    legacies[i].chara == data.umas[u].id
                        && is_full(sj, &legacies[i].sparks)
                        && input.pool.alone_within_borrow_limit(legacies[i].borrowed)
                })
                .collect()
        })
        .collect();
    let FittedTrios { mut fit, mut combinations, combination_ms: mut combo_ms } =
        fit_trios(simulator, context, continuations, &full_veterans, buf);
    // pass 2: every career scored against V (exact over her outcomes): E[V - V with her veteran] - 1
    // (a row that is never the pick, a builder with a borrowed parent under "later", is not scored: no score, no kept scoring)
    progress(4, 0, plan_rows.rows.len());
    if !plan_rows.rows.is_empty() && fit.current_value.is_finite() {
        score_every_row(simulator, context, &fit, &mut plan_rows.rows, buf);
        // the proxies a shown row or the pick's near-tie band can turn on: searched exactly (their
        // saving, score + 1, within PROXY_SAVE of the lowest saving those take), then scored again; when one of them joins a
        // candidate's two most efficient builders, Cont, the trios, V and every score are computed again
        if candidates.proxies_enabled {
            loop {
                let promoted = input.promotable_proxies(&plan_rows.rows);
                if promoted.is_empty() {
                    break;
                }
                let started_ms = now_ms();
                // (the careers scored: a row whose search found the same one, or that was certified, may keep its score)
                let old_careers: Vec<Found> = promoted.iter().map(|&i| plan_rows.rows[i].career.clone()).collect();
                for (&i, old_career) in promoted.iter().zip(&old_careers) {
                    let row = &mut plan_rows.rows[i];
                    let x = row.trainee;
                    if row.origin == RowOrigin::NoBorrow {
                        make_no_borrow_row_exact(simulator, context, searches, profile, row, buf);
                        continue;
                    }
                    let MadeExact::Searched { replaced } =
                        make_row_exact(simulator, context, searches, profile, row, buf)
                    else {
                        continue;
                    };
                    if replaced {
                        set_plan_wins(simulator, input.pool, &mut plan_rows.wins_by_trainee, x, &row.career);
                    }
                    // ("later") her best career has a borrowed parent: that row is a builder now, and the proxy's career,
                    // without one, her nb row (scored below)
                    if !input.is_runnable(&row.career) {
                        let no_lifts = vec![LiftStats::default(); candidates.trainees.len()];
                        plan_rows.rows.push(PlanRow::proxy(x, old_career.clone(), no_lifts, RowOrigin::NoBorrow, None));
                    }
                }
                profile.search_ms += now_ms() - started_ms;
                let next_continuations =
                    plan_continuations(simulator, context, &plan_rows.rows, &plan_rows.wins_by_trainee, buf);
                let same_fit = next_continuations
                    .iter()
                    .zip(&fit.continuations)
                    .all(|(a, b)| a.lam == b.lam && a.phi == b.phi && a.pinf == b.pinf);
                let runs = &mut plan_rows.rows;
                if same_fit {
                    for (&i, old_career) in promoted.iter().zip(&old_careers) {
                        // the same career against the same base (Cont, so step_costs, V, the trios and sides, as when it was scored),
                        // nothing it reads changed: its score, lift and kept scoring stand (a dropped one stays dropped: its score
                        // is as low as it was, and lo only rises here)
                        if same_career(old_career, &runs[i].career)
                            && stamp_ok(simulator, None, runs[i].scoring.as_ref().unwrap().stamp)
                        {
                            prof("reuse_sc", 1.0);
                            continue;
                        }
                        runs[i].scoring = Some(score_row(simulator, context, &fit, &runs[i], buf));
                    }
                    // (the nb rows added above)
                    for i in 0..runs.len() {
                        if runs[i].scoring.is_some() {
                            continue;
                        }
                        runs[i].scoring = Some(score_row(simulator, context, &fit, &runs[i], buf));
                    }
                    input.prune_plan_scorings(runs);
                } else {
                    count_refit();
                    let refit = fit_trios(simulator, context, next_continuations, &full_veterans, buf);
                    fit = refit.fit;
                    combinations = refit.combinations;
                    combo_ms += refit.combination_ms;
                    // (every row scored again, or none: the old kept scorings go first, the new ones kept as above)
                    for run in &mut *runs {
                        run.scoring = None;
                    }
                    if !fit.current_value.is_finite() {
                        break;
                    }
                    score_every_row(simulator, context, &fit, runs, buf);
                }
            }
        }
        profile.scored_runnable_rows =
            plan_rows.rows.iter().filter(|r| r.scoring.is_some() && input.is_runnable(&r.career)).count();
    }
    let runs = &plan_rows.rows;
    let best_combo = combinations
        .iter()
        .min_by(|x, y| x.total_cost(&fit.step_costs).partial_cmp(&y.total_cost(&fit.step_costs)).unwrap())
        .copied();
    let index_ms = now_ms() - profile.index_started_ms;
    let env = &simulator.env;
    let work = json!({"nodes": env.space.nodes.get() - profile.nodes_at_start, "exact": env.exact_scorings - profile.exact_scorings_at_start, "max_ratio": env.max_ratio.iter().copied().fold(0.0f64, f64::max), "class_max_ratio": env.max_ratio, "class_max_deck": env.max_decks(), "cap": env.space.rho_top().min(env.cert), "class_caps": env.space.caps()});
    let any_scored = runs.iter().any(|r| r.scoring.is_some());
    PlanScores { fit, best_combo, any_scored, combo_ms, index_ms, work }
}

/// Every row scored against the fit in order, the kept scorings that can no longer be the pick dropped after each
fn score_every_row(
    simulator: &mut Sim,
    context: &PlanContext,
    fit: &PlanFit,
    runs: &mut [PlanRow],
    buf: &mut Vec<f64>,
) {
    for i in 0..runs.len() {
        runs[i].scoring = Some(score_row(simulator, context, fit, &runs[i], buf));
        context.input.prune_plan_scorings(runs);
    }
}

impl PlanInputs<'_, '_> {
    /// The proxy rows a shown row or the pick's near-tie band can turn on (over the rows that can be shown or picked,
    /// `is_runnable`): their saving within PROXY_SAVE of the lowest saving those take, in row order
    fn promotable_proxies(&self, runs: &[PlanRow]) -> Vec<usize> {
        let mut savings: Vec<f64> =
            runs.iter().filter(|r| self.is_runnable(&r.career)).filter_map(PlanRow::net_saving).collect();
        savings.sort_by(|x, y| y.partial_cmp(x).unwrap());
        if savings.is_empty() {
            return vec![];
        }
        let (top, last_shown) = (savings[0], savings[(OPT_SHOW - 1).min(savings.len() - 1)]);
        let lowest_saving = last_shown.min(near_floor(top)) + 1.0;
        (0..runs.len())
            .filter(|&i| {
                !runs[i].exact
                    && self.is_runnable(&runs[i].career)
                    && runs[i].total() + 1.0 >= lowest_saving * (1.0 - PROXY_SAVE)
            })
            .collect()
    }
}

/// A promoted nb row made exact: her best career without a borrowed parent, certified or searched on the pick's pool
fn make_no_borrow_row_exact(
    simulator: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    row: &mut PlanRow,
    buf: &mut Vec<f64>,
) {
    if certified(simulator.env, context.pick_parents(), row.trainee, &row.career) {
        profile.certified_proxies += 1;
    } else if let Some(career) = exact_no_borrow_career(simulator, context, searches, row.trainee, buf) {
        row.career = career;
    }
    row.exact = true;
}

/// The proxy check (`proxy_check`, a measurement): each builder's career from her deck class's winning decks (the main
/// search's and the candidates' best careers of her class, legal for her and valued at her own chances) through the
/// surrogate, no career search; its lifts per candidate (efficiency, as Cont takes them) and its score beside the exact
/// ones. Runs after everything else, on `cache_before_builders` (the cache as it was before the builders' searches,
/// swapped in and back), so the plan's own numbers are unchanged
pub(crate) fn check_plan_proxies(
    simulator: &mut Sim,
    context: &PlanContext,
    searches: &PlanSearches,
    runs: &[PlanRow],
    scores: &PlanScores,
    mut cache_before_builders: Cache,
    buf: &mut Vec<f64>,
) -> Value {
    let PlanContext { input, candidates } = *context;
    let data = context.data();
    let candidate_count = candidates.trainees.len();
    std::mem::swap(&mut simulator.evaluator.cache, &mut cache_before_builders);
    let winners = class_winners(input.solved, candidates, &searches.best_careers);
    let stats_base = candidates.stats_base();
    let score_base = scores.fit.score_base(candidates);
    let efficiencies = |x: crate::uma::UmaIndex, lift_stats: &[LiftStats]| -> Vec<f64> {
        (0..candidate_count)
            .map(|ui| {
                let lift = lift_stats[ui];
                let odds = candidates.current_odds[ui];
                if data.umas[candidates.trainees[ui]].id == data.umas[x].id || lift.lift_chance <= 0.0 || odds <= 0.0 {
                    0.0
                } else {
                    lift.efficiency(odds)
                }
            })
            .collect()
    };
    let mut rows = vec![];
    for r in runs {
        let x = r.trainee;
        let is_candidate = candidates.trainees.contains(&x);
        let exact_efficiencies = efficiencies(x, &r.lift_stats);
        let (proxy, proxy_efficiencies, proxy_score) = if is_candidate {
            (Some(r.career.clone()), exact_efficiencies.clone(), r.net_saving())
        } else {
            match proxy_career(simulator, &winners, context.parents(), x, buf) {
                None => (None, vec![0.0; candidate_count], None),
                Some(career) => {
                    let CareerScore { lift_stats, .. } = score_career(
                        simulator,
                        &context.scoring(&stats_base),
                        found_career(x, &career),
                        Want::Stats,
                        buf,
                    );
                    let proxy_efficiencies = efficiencies(x, &lift_stats);
                    #[expect(
                        clippy::if_then_some_else_none,
                        reason = "Keep the multiline scoring call in an explicit conditional (Code clarity: Calls)."
                    )]
                    let proxy_score = if scores.any_scored {
                        let score = score_career(
                            simulator,
                            &context.scoring(&score_base),
                            found_career(x, &career),
                            Want::Score,
                            buf,
                        );
                        Some(score.expected_saving - 1.0)
                    } else {
                        None
                    };
                    (Some(career), proxy_efficiencies, proxy_score)
                }
            }
        };
        let same_pair = proxy.as_ref().is_some_and(|f| f.unordered_parents() == r.career.unordered_parents());
        let same_deck = proxy.as_ref().is_some_and(|f| {
            f.deck.entries == r.career.deck.entries
                && f.deck.plain_pals == r.career.deck.plain_pals
                && f.deck.filler_cards == r.career.deck.filler_cards
        });
        rows.push(json!({"trainee": data.umas[x].name, "cand": is_candidate, "class": simulator.env.deck_class[x], "exact_val": r.career.value, "proxy_val": proxy.as_ref().map(|f| f.value),
        "same_pair": same_pair, "same_deck": same_deck, "score": r.net_saving(), "proxy_score": proxy_score, "eff": exact_efficiencies, "proxy_eff": proxy_efficiencies}));
    }
    std::mem::swap(&mut simulator.evaluator.cache, &mut cache_before_builders);
    json!({"cands": candidates.trainees.iter().map(|&u| data.umas[u].name.clone()).collect::<Vec<_>>(), "rows": rows})
}

/// A row scored against the fit, with the stamp of the cache it read (`stamped`)
fn score_row(
    simulator: &mut Sim,
    context: &PlanContext,
    fit: &PlanFit,
    row: &PlanRow,
    buf: &mut Vec<f64>,
) -> RowScoring {
    let (scoring, stamp) = stamped(simulator, None, |simulator| score_plan_row(simulator, context, fit, row, buf));
    RowScoring { stamp, ..scoring }
}

/// The row's saving against the fit (`Want::Keep`), unstamped; a row that cannot be the pick is not scored
fn score_plan_row(
    simulator: &mut Sim,
    context: &PlanContext,
    fit: &PlanFit,
    row: &PlanRow,
    buf: &mut Vec<f64>,
) -> RowScoring {
    if !context.input.is_runnable(&row.career) {
        return RowScoring { net_saving: f64::NEG_INFINITY, improvement_chance: 0.0, kept: None, stamp: None };
    }
    let score_base = fit.score_base(context.candidates);
    let CareerScore { expected_saving, improvement_chance, kept, .. } =
        score_career(simulator, &context.scoring(&score_base), row.scored_career(), Want::Keep, buf);
    RowScoring { net_saving: expected_saving - 1.0, improvement_chance, kept, stamp: None }
}

impl PlanInputs<'_, '_> {
    // Each row's kept scoring (`Scored`: what a roll of the pick reads, its tables and outcome chances) is kept only
    // while the row can still be the pick. The pick (below) is a row within NEAR.max(0.05 |top|) of top, top at least the
    // highest score less 1e-9 (the argmax's tolerance), or the base row a looped pick repeats (`undup`). A row's score
    // changes only when it is scored again, which keeps a fresh one, and an exactly searched row is scored again only
    // when every row is (a refit). The highest exact saving among the rows scored since every row last
    // was scored is at most the final top; thr(t) = t - NEAR.max(0.05 |t|) rises with t; a row below thr(best_exact_saving) (less a slack far
    // above its rounding) is not near the final top unless it is scored again. Kept also: every row a later row repeats
    // (same uma and pair: a looped or nb row the pick moves from; a row whose career changes is scored again). Only the
    // pick's is read (`RollKeep`); the highest exact saving is over runnable rows (`is_runnable`).
    fn prune_plan_scorings(&self, runs: &mut [PlanRow]) {
        let best_exact_saving = runs
            .iter()
            .filter(|r| r.exact && self.is_runnable(&r.career))
            .filter_map(PlanRow::net_saving)
            .fold(f64::NEG_INFINITY, f64::max);
        if !best_exact_saving.is_finite() {
            return;
        }
        let retention_floor = near_floor(best_exact_saving) - 1e-6 * (1.0 + best_exact_saving.abs());
        for i in 0..runs.len() {
            let Some(scoring) = &runs[i].scoring else { continue };
            if scoring.kept.is_none() || !(scoring.net_saving < retention_floor) {
                continue;
            }
            if (i + 1..runs.len()).any(|k| runs[k].repeats(&runs[i])) {
                continue;
            }
            runs[i].scoring.as_mut().unwrap().kept = None;
        }
    }
}
