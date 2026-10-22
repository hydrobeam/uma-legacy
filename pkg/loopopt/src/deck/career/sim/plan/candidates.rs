//! Candidate setup and exact and surrogate careers.
use super::{
    CachedSearch, CareerScore, HINT_FLOOR, IDX_CANDS, KeptPlans, PROXY_EFF, ParentPool, PlanCandidates, PlanContext,
    PlanInputs, PlanProfile, PlanRow, PlanRows, PlanSearches, PlanSetup, RowOrigin, Want, best_of, best_sur, bests_key,
    capped, certified, found_career, found_lineage, hint_umas, is_full, lift_eff, rotation, same_career, score_career,
    search_best_career, sim_for, stamp_ok, stamped, take_kept_plan, unordered_parents,
};
use crate::deck::career::sim::plan::search::{HintUma, LoopRotation};
use crate::deck::career::sim::{Env, Found, Pool, Sel, Sim, Solved, Value, json, now_ms, prof, progress};

/// Choose the stage candidates, search their best careers, and build the simulator the later phases share.
pub(crate) fn setup_plan_candidates<'env, 'data>(
    env: &'env mut Env<'data>,
    input: &PlanInputs<'_, 'data>,
    started_ms: f64,
    pick_career: Found,
    buf: &mut Vec<f64>,
) -> Result<PlanSetup<'env, 'data>, Value> {
    let settings = input.settings;
    let legacies = input.legacies;
    let data = input.resources.data;
    // the work counters at the start (deck-search nodes, exact scorings): reported as this plan's own
    let (nodes_at_start, exact_scorings_at_start) =
        (env.space.nodes.get(), crate::deck::JCOUNT.with(std::cell::Cell::get));
    // the loop planner's members only when the job names them (`loop_members`, an override; the plan no longer runs the
    // loop planner's search itself: on the val/plan pools its members always entered the candidates by the routes below)
    let loop_rotation = match &settings.loop_members {
        Some(ids) => rotation(env, ids),
        None => LoopRotation { members: vec![], member_info: vec![], note: None },
    };
    let main_career = input.solved.found[0].clone();
    let StageCandidates { trainees: candidate_trainees, hint } =
        stage_candidates(env, input, &main_career, &loop_rotation.members);
    let loop_ms = now_ms() - started_ms;
    let allowed_trainees: Vec<usize> = input.trainees.iter().copied().filter(|&u| env.dmax[u].is_some()).collect();
    let searches_started_ms = now_ms();
    let env_budget = env.budget;
    // (the candidates' career searches below under the cap, as `capped` sets it: the Sim it takes is made after them)
    if settings.search_work > 0 {
        env.budget = settings.search_work;
        env.hard = settings.search_work;
    }
    env.use_seen = true;
    let mut searches = new_searches(input, &main_career, env_budget);
    if settings.sur != 1 {
        for &u in &candidate_trainees {
            if searches.best_careers[u].is_unsearched() {
                let search = best_of(env, input_parents(input), u, searches.kept_plans(legacies.len()));
                searches.record_best_career(u, search);
            }
        }
    }
    env.hard = 0;
    env.budget = env_budget;
    let mut search_ms = now_ms() - searches_started_ms;
    let mut sim = sim_for(env, input.resources, settings)?;
    // the cache the planner's table values read (trios' third members, candidates' lifted odds): each search's exact
    // winners only, the main search's best careers and each uma's best one, so what it holds (and every number read from
    // it) depends on the job alone, not on what the pruning happened to check fully (earlier jobs' proofs, the order, a
    // kept plan). Before, every fully checked deck went in: a candidate then had several decks, and her lifted odds
    // read the best of them; with one the planner's numbers moved once (val/plan: picks unchanged)
    for found in &input.solved.found {
        sim.cache.add(sim.env, found.t, &found.sel);
    }
    for &u in &candidate_trainees {
        if let Some(found) = searches.best_careers[u].found() {
            sim.cache.add(sim.env, u, &found.sel);
        }
    }
    // (`proxy_check`: the cache as it is before any builder's search, for the proxies at the end)
    let cache_before_builders = settings.proxy_check.then(|| sim.cache.clone());
    // the surrogate's best careers (sur 1: in place of the searches; 2: beside them, differences reported)
    let mut surrogate_differences: Vec<Value> = vec![];
    if settings.sur > 0 {
        let surrogate_started_ms = now_ms();
        capped(&mut sim, settings.search_work, env_budget, |sim| {
            for &u in &candidate_trainees {
                if (settings.sur == 2 || searches.best_careers[u].is_unsearched())
                    && let Some(difference) = plan_surrogate(sim, input, &mut searches.best_careers, u, buf)
                {
                    surrogate_differences.push(difference);
                }
            }
        });
        search_ms += now_ms() - surrogate_started_ms;
    }
    let careers: Vec<Option<Found>> =
        candidate_trainees.iter().map(|&u| searches.best_careers[u].found().cloned()).collect();
    let candidate_count = candidate_trainees.len();
    let current_odds: Vec<f64> = careers.iter().map(|f| f.as_ref().map_or(0.0, |f| f.val)).collect();
    let sj = sim.env.sj;
    // per candidate whether an 8/8 of hers exists (a free side of the trios)
    let has_full_veteran: Vec<bool> = candidate_trainees
        .iter()
        .map(|&u| {
            legacies
                .iter()
                .any(|l| l.chara == data.umas[u].id && is_full(sj, &l.sparks) && input.pool.bor_one(l.borrowed))
        })
        .collect();
    let later_stage_costs = vec![0.0f64; candidate_count];
    let index_started_ms = now_ms();
    // Builders through their deck class: a builder's career is first taken from her class's winning decks (the main
    // search's and the candidates' best careers of her class: legal for her, the same deck problem, valued at her own
    // chances through the surrogate, `best_sur`), no career search. Exact searches only where the plan can turn on her:
    // a lift efficiency within PROXY_EFF of a candidate's second best (Cont's two builders come from the exactly
    // searched careers), or a saving within PROXY_SAVE of the shown rows (the page's top OPT_SHOW) or the pick's near-tie
    // band. Measured on val/plan (`proxy_check`): the proxy's odds only ever read low (it is a
    // career she can run); its efficiency on contenders for a candidate's top two at most 2.8% low (p11; it can also
    // read high, which only searches more), its saving at most 2.7% low; exact on the other five jobs.
    // `exact_all` restores a search for every uma.
    let proxies_enabled = settings.sur == 0 && !settings.exact_all && !settings.proxy_check;
    let candidates = PlanCandidates {
        main_career,
        pick_career,
        loop_members: loop_rotation.members,
        loop_member_info: loop_rotation.member_info,
        loop_member_note: loop_rotation.note,
        hint,
        trainees: candidate_trainees,
        allowed_trainees,
        careers,
        current_odds,
        has_full_veteran,
        later_stage_costs,
        placeholder_step_costs: vec![1.0f64; candidate_count],
        proxies_enabled,
    };
    let profile = PlanProfile {
        started_ms,
        index_started_ms,
        loop_ms,
        search_ms,
        nodes_at_start,
        exact_scorings_at_start,
        certified_proxies: 0,
        scored_runnable_rows: 0,
        surrogate_differences,
    };
    Ok(PlanSetup { sim, candidates, searches, profile, cache_before_builders })
}

/// The stage-1 candidates (uma indices) and the page's record of those `hint_umas` added.
struct StageCandidates {
    trainees: Vec<usize>,
    hint: Vec<Value>,
}

/// Stage-1 candidates (who A can be; whose odds a builder may lift): the best career's trainee, the named loop members,
/// the umas of the best career's lineage, the umas you have veterans of, the search's other trainees; at most
/// IDX_CANDS. A given fit (`PlanJob::fit`) names them instead.
fn stage_candidates(env: &mut Env, input: &PlanInputs, main_career: &Found, loop_members: &[usize]) -> StageCandidates {
    let legacies = input.legacies;
    let settings = input.settings;
    let mut trainees: Vec<usize> = vec![main_career.t];
    let push = |u: usize, candidates: &mut Vec<usize>| {
        if !candidates.contains(&u) && input.trainees.contains(&u) {
            candidates.push(u);
        }
    };
    for &member in loop_members {
        push(member, &mut trainees);
    }
    for parent in [main_career.a, main_career.b] {
        let lineage_charas =
            std::iter::once(legacies[parent].chara).chain(legacies[parent].parents.iter().map(|p| p.chara));
        for chara in lineage_charas {
            if let Some(u) = env.uix(chara) {
                push(u, &mut trainees);
            }
        }
    }
    for legacy in legacies.iter().rev() {
        if !legacy.borrowed
            && let Some(u) = env.uix(legacy.chara)
        {
            push(u, &mut trainees);
        }
    }
    for found in &input.solved.found {
        push(found.t, &mut trainees);
    }
    trainees.truncate(IDX_CANDS);
    if let Some(fit) = &settings.fit {
        trainees =
            fit.iter().filter_map(|&(chara, ..)| env.uix(chara)).filter(|u| input.trainees.contains(u)).collect();
    }
    // a pool that gives fewer than HINT_FLOOR candidates (a loop being started: veterans of umas that won't be in it, mob
    // parents) is filled from the planner's own model: the owned umas ranked by their odds with a full loop lineage
    // (`hint_umas`), no deck search
    let mut hint: Vec<Value> = vec![];
    if settings.loop_members.is_none() && settings.fit.is_none() && trainees.len() < HINT_FLOOR {
        let data = input.resources.data;
        for HintUma { trainee, ceiling } in
            hint_umas(env, input.pool, &trainees, input.trainees, HINT_FLOOR - trainees.len())
        {
            trainees.push(trainee);
            hint.push(json!({"id": data.umas[trainee].id, "name": data.umas[trainee].name, "ceiling": ceiling}));
        }
    }
    StageCandidates { trainees, hint }
}

/// The plan's searches before any is run: the main search's best career known, and the kept plans of this pool and of
/// this pool less its last veteran taken out of `BESTS`
fn new_searches(input: &PlanInputs, main_career: &Found, env_budget: usize) -> PlanSearches {
    let uma_count = input.resources.data.umas.len();
    let legacy_count = input.legacies.len();
    let mut best_careers = vec![CachedSearch::Unsearched; uma_count];
    best_careers[main_career.t] = CachedSearch::Known(Some(main_career.clone()));
    let kept_same_pool = take_kept_plan(bests_key(input.resources.job, input.fixed_parents, legacy_count));
    let kept_previous_pool = if legacy_count > 1 {
        take_kept_plan(bests_key(input.resources.job, input.fixed_parents, legacy_count - 1))
    } else {
        None
    };
    PlanSearches {
        best_careers,
        proof_refs: vec![vec![]; uma_count],
        base_incomplete: vec![false; uma_count],
        looped_incomplete: vec![false; uma_count],
        no_borrow_incomplete: vec![false; uma_count],
        looped_careers: vec![],
        kept_same_pool,
        kept_previous_pool,
        env_budget,
    }
}

const fn input_parents<'input>(input: &PlanInputs<'input, '_>) -> ParentPool<'input> {
    ParentPool { pool: input.pool, fixed_parents: input.fixed_parents }
}

/// A winning deck of the main search or a candidate's best career, which a builder of the same deck class may use.
pub(crate) struct ClassWinner {
    trainee: usize,
    deck: Sel,
}

/// The main search's winners, then each candidate's best career (in candidate order)
pub(crate) fn class_winners(
    solved: &Solved,
    candidates: &PlanCandidates,
    best_careers: &[CachedSearch],
) -> Vec<ClassWinner> {
    solved
        .found
        .iter()
        .map(|found| ClassWinner { trainee: found.t, deck: found.sel.clone() })
        .chain(
            candidates.trainees.iter().filter_map(|&u| {
                best_careers[u].found().map(|found| ClassWinner { trainee: u, deck: found.sel.clone() })
            }),
        )
        .collect()
}

/// Builder `trainee`'s proxy career: her deck class's winning decks added to her cached decks, then her best career
/// through the surrogate (`best_sur`, no refresh)
pub(crate) fn proxy_career(
    sim: &mut Sim,
    winners: &[ClassWinner],
    parents: ParentPool,
    trainee: usize,
    buf: &mut Vec<f64>,
) -> Option<Found> {
    let deck_class = sim.env.cls[trainee];
    for winner in winners {
        if winner.trainee != trainee && sim.env.cls[winner.trainee] == deck_class {
            sim.cache.add(sim.env, trainee, &winner.deck);
        }
    }
    best_sur(sim, parents, trainee, 1e9, buf)
}

/// Pass 1: every allowed uma's best career; per candidate, the chance her veteran raises that candidate's odds and the
/// mean odds raised to (exact over her outcomes). Then the near-top builders' proxies searched exactly, and each row's
/// trainee's G1 wins.
pub(crate) fn score_and_admit_proxies(
    sim: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> PlanRows {
    let mut rows = pass_one_rows(sim, context, searches, profile, buf);
    if context.candidates.proxies_enabled {
        admit_near_top_proxies(sim, context, searches, profile, &mut rows, buf);
    }
    let mut wins_by_trainee: Vec<Vec<usize>> = vec![vec![]; context.data().umas.len()];
    for row in &rows {
        set_plan_wins(sim, context.input.pool, &mut wins_by_trainee, row.trainee, &row.career);
    }
    PlanRows { rows, wins_by_trainee }
}

/// Every allowed uma's base row, the main career's trainee first: a proxy while proxies are on and she is unsearched,
/// else her searched best career. (Placeholder step costs: 1, so every candidate's lifts are gathered; an 8/8 of hers
/// may be a poorer parent than a fresh one.)
fn pass_one_rows(
    sim: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> Vec<PlanRow> {
    let PlanContext { input, candidates } = *context;
    let settings = input.settings;
    let winners = class_winners(input.solved, candidates, &searches.best_careers);
    let stats_base = candidates.stats_base();
    let mut rows: Vec<PlanRow> = vec![];
    let mut order: Vec<usize> = vec![candidates.main_career.t];
    for &trainee in &candidates.allowed_trainees {
        if trainee != candidates.main_career.t {
            order.push(trainee);
        }
    }
    for &x in &order {
        progress(2, order.iter().position(|&y| y == x).unwrap_or(0), order.len()); // pass 1: uma k of n
        if candidates.proxies_enabled && searches.best_careers[x].is_unsearched() {
            let proxy_started_ms = now_ms();
            let proxy = proxy_career(sim, &winners, context.parents(), x, buf);
            profile.search_ms += now_ms() - proxy_started_ms;
            let Some(career) = proxy else { continue };
            let (CareerScore { lift_stats, .. }, stats_stamp) = stamped(sim, Some(&candidates.trainees), |sim| {
                score_career(sim, &context.scoring(&stats_base), found_career(x, &career), Want::Stats, buf)
            });
            rows.push(PlanRow::proxy(x, career, lift_stats, RowOrigin::Base, stats_stamp));
            continue;
        }
        if searches.best_careers[x].is_unsearched() || (settings.sur == 2 && !candidates.trainees.contains(&x)) {
            let search_started_ms = now_ms();
            capped(sim, settings.search_work, searches.env_budget, |sim| {
                if settings.sur != 1 && searches.best_careers[x].is_unsearched() {
                    search_best_career(sim, context, searches, x);
                }
                if settings.sur > 0
                    && let Some(difference) = plan_surrogate(sim, input, &mut searches.best_careers, x, buf)
                {
                    profile.surrogate_differences.push(difference);
                }
            });
            profile.search_ms += now_ms() - search_started_ms;
        }
        let Some(career) = searches.best_careers[x].found().cloned() else { continue };
        let CareerScore { lift_stats, .. } =
            score_career(sim, &context.scoring(&stats_base), found_career(x, &career), Want::Stats, buf);
        rows.push(PlanRow::exact(x, career, lift_stats, RowOrigin::Base));
    }
    rows
}

/// A row's efficiency for one candidate (`lift_eff`).
#[derive(Clone, Copy)]
struct RowEfficiency {
    efficiency: f64,
    /// Position in the plan's rows.
    row: usize,
}

/// The near-top builders searched exactly: per candidate every proxy whose efficiency reaches (1 - PROXY_EFF) of the
/// candidate's second best (exact and proxy careers together), until none is left
fn admit_near_top_proxies(
    sim: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    rows: &mut [PlanRow],
    buf: &mut Vec<f64>,
) {
    loop {
        let admitted = near_top_proxies(context, rows);
        if admitted.is_empty() {
            break;
        }
        let search_started_ms = now_ms();
        for row in admitted {
            make_row_exact(sim, context, searches, profile, &mut rows[row], buf);
        }
        profile.search_ms += now_ms() - search_started_ms;
    }
}

/// The proxy rows within PROXY_EFF of a candidate's second most efficient row, in row order
fn near_top_proxies(context: &PlanContext, rows: &[PlanRow]) -> Vec<usize> {
    let PlanContext { input, candidates } = *context;
    let data = context.data();
    let mut admitted: Vec<usize> = vec![];
    for ui in 0..candidates.trainees.len() {
        if candidates.current_odds[ui] <= 0.0 {
            continue;
        }
        let (u, p) = (candidates.trainees[ui], candidates.current_odds[ui]);
        let mut efficiencies: Vec<RowEfficiency> = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| data.umas[r.trainee].id != data.umas[u].id && r.lift_stats[ui].0 > 0.0)
            .map(|(row, r)| {
                let (lift_chance, lifted_odds_sum) = r.lift_stats[ui];
                RowEfficiency { efficiency: lift_eff(lift_chance, lifted_odds_sum, p), row }
            })
            .collect();
        efficiencies.sort_by(|x, y| y.efficiency.partial_cmp(&x.efficiency).unwrap());
        let Some(best) = efficiencies.first() else { continue };
        let second = efficiencies.get(1).map_or(best.efficiency, |x| x.efficiency);
        for candidate_row in &efficiencies {
            if candidate_row.efficiency < second * (1.0 - input.settings.proxy_eff.unwrap_or(PROXY_EFF)) {
                break;
            }
            if !rows[candidate_row.row].exact && !admitted.contains(&candidate_row.row) {
                admitted.push(candidate_row.row);
            }
        }
    }
    admitted.sort_unstable();
    admitted
}

/// How a proxy row was made exact.
pub(crate) enum MadeExact {
    /// Her class's proofs certify the proxy: it is her best career's value, no search.
    Certified,
    /// Her best career searched; `replaced`: the row's career is now that one.
    Searched { replaced: bool },
}

/// A proxy row made exact: certified by her class's proofs, else her best career searched (`exact_plan_career`)
pub(crate) fn make_row_exact(
    sim: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    row: &mut PlanRow,
    buf: &mut Vec<f64>,
) -> MadeExact {
    let trainee = row.trainee;
    // a proxy the class's proofs certify needs no search (it is her best career's value)
    if certified(sim.env, context.parents(), trainee, &row.career) {
        searches.best_careers[trainee] = CachedSearch::Known(Some(row.career.clone()));
        row.exact = true;
        profile.certified_proxies += 1;
        return MadeExact::Certified;
    }
    let exact = exact_plan_career(sim, context, searches, row, buf);
    let replaced = exact.is_some();
    if let Some(ExactCareer { career, lift_stats }) = exact {
        row.career = career;
        row.lift_stats = lift_stats;
    }
    row.exact = true;
    row.stats_stamp = None;
    MadeExact::Searched { replaced }
}

/// Where `sur` 2's surrogate and search disagree for one uma, as the output reports it; with `sur` 1 the surrogate's
/// career replaces her best career (None returned)
fn plan_surrogate(
    sim: &mut Sim,
    input: &PlanInputs,
    best_careers: &mut [CachedSearch],
    u: usize,
    buf: &mut Vec<f64>,
) -> Option<Value> {
    let settings = input.settings;
    let surrogate = best_sur(sim, input_parents(input), u, settings.sur_margin, buf);
    if settings.sur != 2 {
        best_careers[u] = CachedSearch::Known(surrogate);
        return None;
    }
    let searched = best_careers[u].found().cloned();
    let (searched_value, surrogate_value) =
        (searched.as_ref().map_or(0.0, |f| f.val), surrogate.as_ref().map_or(0.0, |f| f.val));
    let same = searched.as_ref().map(unordered_parents) == surrogate.as_ref().map(unordered_parents);
    if !same || (searched_value - surrogate_value).abs() > 1e-9 * searched_value.max(surrogate_value) {
        let data = input.resources.data;
        return Some(
            json!({"trainee": data.umas[u].name, "search": [searched.as_ref().map(|f| [f.a, f.b]), searched_value], "sur": [surrogate.as_ref().map(|f| [f.a, f.b]), surrogate_value]}),
        );
    }
    None
}

/// An exact career and its lifts per candidate.
pub(crate) struct ExactCareer {
    pub(crate) career: Found,
    pub(crate) lift_stats: Vec<(f64, f64)>,
}

/// The proxy row's trainee's exact best career (searched now if not yet) and its lifts. The proxy's lifts are taken as
/// they are when the search found the same career and nothing they read has changed (their stamp, `stamped`, over the
/// candidates' decks: this base reads no other's)
fn exact_plan_career(
    sim: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    proxy: &PlanRow,
    buf: &mut Vec<f64>,
) -> Option<ExactCareer> {
    let x = proxy.trainee;
    let settings = context.settings();
    if searches.best_careers[x].is_unsearched() {
        capped(sim, settings.search_work, searches.env_budget, |sim| {
            search_best_career(sim, context, searches, x);
        });
    }
    let career = searches.best_careers[x].found().cloned()?;
    if same_career(&proxy.career, &career) && stamp_ok(sim, Some(&context.candidates.trainees), proxy.stats_stamp) {
        prof("reuse_st", 1.0);
        return Some(ExactCareer { career, lift_stats: proxy.lift_stats.clone() });
    }
    let stats_base = context.candidates.stats_base();
    let CareerScore { lift_stats, .. } =
        score_career(sim, &context.scoring(&stats_base), found_career(x, &career), Want::Stats, buf);
    Some(ExactCareer { career, lift_stats })
}

/// Record the G1 races trainee's career wins with a chance above 0.5 (`PlanRows::wins_by_trainee`)
pub(crate) fn set_plan_wins(
    sim: &mut Sim,
    pool: &Pool,
    wins_by_trainee: &mut [Vec<usize>],
    trainee: usize,
    career: &Found,
) {
    let win_chances = sim.wins(pool, found_lineage(trainee, career));
    wins_by_trainee[trainee] =
        win_chances.iter().enumerate().filter(|(_, chance)| **chance > 0.5).map(|(race, _)| race).collect();
}

/// The trainee's best career without a borrowed parent, on the pick's pool: the surrogate's under `sur` 1, else
/// searched (not reusing kept plans)
pub(crate) fn exact_no_borrow_career(
    sim: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    trainee: usize,
    buf: &mut Vec<f64>,
) -> Option<Found> {
    let settings = context.settings();
    let no_kept_plans = KeptPlans { same_pool: None, previous_pool: None, legacy_count: context.legacy_count() };
    capped(sim, settings.search_work, searches.env_budget, |sim| {
        if settings.sur == 1 {
            best_sur(sim, context.pick_parents(), trainee, settings.sur_margin, buf)
        } else {
            let search = best_of(sim.env, context.pick_parents(), trainee, no_kept_plans);
            if !search.complete {
                searches.no_borrow_incomplete[trainee] = true;
            }
            search.found
        }
    })
}
