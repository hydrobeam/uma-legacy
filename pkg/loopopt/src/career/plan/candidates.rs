//! Candidate setup and exact and surrogate careers.
use crate::career::Env;
use crate::career::clock::{now_ms, prof, progress};
use crate::career::plan::caps::{CapPool, held_caps, least_caps};
use crate::career::plan::cont::Want;
use crate::career::plan::kept::{
    CachedSearch, KeptPlans, best_of, bests_key, plan_scope, same_career, stamp_ok, stamped, take_kept_plan,
};
use crate::career::plan::rows::{PlanRow, RowOrigin};
use crate::career::plan::score::kernel::LiftStats;
use crate::career::plan::score::{CareerScore, score_career};
use crate::career::plan::search::{
    HintUma, IDX_CANDS, LoopRotation, PROXY_EFF, best_sur, capped, certified, hint_umas, is_full, rotation, sim_for,
    today_umas,
};
use crate::career::plan::{
    ParentPool, PlanCandidates, PlanContext, PlanInputs, PlanProfile, PlanRows, PlanSearches, PlanSetup, found_career,
    search_best_career,
};
use crate::career::pool::Pool;
use crate::career::search::{CareerSearchResult, Found};
use crate::career::sim::Sim;
use crate::deck::space::SelectedDeck;
use serde_json::{Value, json};
/// Choose the stage candidates, search their best careers, and build the simulator the later phases share.
pub(crate) fn setup_plan_candidates<'env, 'data>(
    env: &'env mut Env<'data>,
    kept: &'env mut crate::career::state::KeptState,
    input: &PlanInputs<'_, 'data>,
    started_ms: f64,
    pick_career: Found,
    buf: &mut Vec<f64>,
) -> Result<PlanSetup<'env, 'data>, Value> {
    let settings = input.settings;
    let legacies = input.legacies;
    let data = input.resources.data;
    // the work counters at the start (deck-search nodes, exact scorings): reported as this plan's own
    let (nodes_at_start, exact_scorings_at_start) = (env.setup.space.nodes.get(), env.state.exact_scorings);
    // Named loop members supplement the candidates; this setup does not run the loop search.
    let loop_rotation = match &settings.loop_members {
        Some(ids) => rotation(&env.setup, ids),
        None => LoopRotation { members: vec![], member_info: vec![], note: None },
    };
    let main_career = input.solved.found[0].clone();
    let StageCandidates { trainees: stage_trainees, hint, cut, side_only } =
        stage_candidates(env, input, &main_career, &loop_rotation.members);
    let loop_ms = now_ms() - started_ms;
    let allowed_trainees: Vec<crate::uma::UmaIndex> =
        input.trainees.iter().copied().filter(|&trainee| env.setup.deck_odds_ceilings[trainee].is_some()).collect();
    let searches_started_ms = now_ms();
    let env_budget = env.state.budget;
    // (the candidates' career searches below under the cap, as `capped` sets it: the Sim it takes is made after them)
    if settings.search_work > 0 {
        env.state.budget = settings.search_work;
        env.state.hard_work_cap = settings.search_work;
    }
    env.state.use_seen = true;
    let mut searches = input.new_searches(kept, &main_career, env_budget, env.setup.plan_space_key);
    // (the kept plans' class flags, by class content: what their careers and completeness rest on, `Bests::flags`; so
    // every reuse, `best_of` and the looped rows, and the output read them with this job's)
    for plan in [&searches.kept_same_pool, &searches.kept_previous_pool].into_iter().flatten() {
        env.setup.union_keyed(&mut env.state, plan.flags.iter().copied());
    }
    if settings.surrogate_mode != 1 {
        for &trainee in &stage_trainees {
            if searches.best_careers[trainee].is_unsearched() {
                let search = best_of(env, input.input_parents(), trainee, searches.kept_plans(legacies.len()));
                searches.record_best_career(trainee, search);
            }
        }
    }
    env.state.hard_work_cap = 0;
    env.state.budget = env_budget;
    let mut search_ms = now_ms() - searches_started_ms;
    // the caps the kept fits of pools this one holds keep for each stage candidate (none under a given fit: the job's
    // model; none for a side-only uma, who has no step)
    let cap_pool = CapPool::of(
        input.resources.job,
        input.fixed_parents,
        plan_scope(input.resources.job, env.setup.plan_space_key),
    );
    let held = if settings.fit.is_some() {
        vec![vec![]; stage_trainees.len()]
    } else {
        held_caps(&kept.step_caps, &cap_pool, &stage_trainees)
    };
    let mut sim = sim_for(env, kept, input.resources, settings)?;
    // the cache the planner's table values read (trios' third members, candidates' lifted odds): each search's exact
    // winners only, the main search's best careers and each uma's best one, so what it holds (and every number read from
    // it) depends on the job alone, not on what the pruning happened to check fully (earlier jobs' proofs, the order, a
    // kept plan).
    for found in &input.solved.found {
        sim.evaluator.cache.add(&sim.env.setup, &mut sim.env.state, &mut sim.env.memo, found.trainee, &found.deck);
    }
    for &trainee in &stage_trainees {
        if let Some(found) = searches.best_careers[trainee].found() {
            sim.evaluator.cache.add(&sim.env.setup, &mut sim.env.state, &mut sim.env.memo, trainee, &found.deck);
        }
    }
    // (`proxy_check`: the cache as it is before any builder's search, for the proxies at the end)
    let cache_before_builders = settings.proxy_check.then(|| sim.evaluator.cache.clone());
    // the surrogate's best careers (sur 1: in place of the searches; 2: beside them, differences reported)
    let mut surrogate_differences: Vec<Value> = vec![];
    if settings.surrogate_mode > 0 {
        let surrogate_started_ms = now_ms();
        capped(&mut sim, settings.search_work, env_budget, |sim| {
            for &trainee in &stage_trainees {
                if (settings.surrogate_mode == 2 || searches.best_careers[trainee].is_unsearched())
                    && let Some(difference) = plan_surrogate(sim, input, &mut searches.best_careers, trainee, buf)
                {
                    surrogate_differences.push(difference);
                }
            }
        });
        search_ms += now_ms() - surrogate_started_ms;
    }
    // the stage candidates, then the side-only umas (no career: no odds, no step)
    let stage_count = stage_trainees.len();
    let candidate_trainees: Vec<crate::uma::UmaIndex> = stage_trainees.into_iter().chain(side_only).collect();
    let careers: Vec<Option<Found>> = candidate_trainees
        .iter()
        .enumerate()
        .map(
            |(candidate, &trainee)| {
                if candidate < stage_count { searches.best_careers[trainee].found().cloned() } else { None }
            },
        )
        .collect();
    let candidate_count = candidate_trainees.len();
    let current_odds: Vec<f64> =
        careers.iter().map(|career| career.as_ref().map_or(0.0, |career| career.value)).collect();
    // each stage candidate's step cap: the least of her held caps at her odds now
    let mut step_caps = least_caps(&held, &current_odds[..stage_count]);
    step_caps.resize(candidate_count, None);
    // per candidate whether her odds now are her proven best career's: searched (not the surrogate's) and complete (the
    // main search's, for the main career's trainee)
    let certified: Vec<bool> = candidate_trainees
        .iter()
        .zip(&careers)
        .map(|(&trainee, career)| {
            career.is_some()
                && settings.surrogate_mode != 1
                && if trainee == main_career.trainee {
                    input.solved.complete
                } else {
                    !searches.base_incomplete[trainee]
                }
        })
        .collect();
    let skill_job = sim.env.setup.skill_job;
    // per candidate whether a full veteran of hers exists (a free side of the trios)
    let has_full_veteran: Vec<bool> = candidate_trainees
        .iter()
        .map(|&trainee| {
            legacies.iter().any(|legacy| {
                legacy.character_id == data.umas[trainee].id
                    && is_full(skill_job, &legacy.sparks)
                    && input.pool.alone_within_borrow_limit(legacy.borrowed)
            })
        })
        .collect();
    let index_started_ms = now_ms();
    // Builders through their deck class: a builder's career is first taken from her class's winning decks (the main
    // search's and the candidates' best careers of her class: legal for her, the same deck problem, valued at her own
    // chances through the surrogate, `best_sur`), no career search. Exact searches only where the plan can turn on her:
    // a lift efficiency within PROXY_EFF of a candidate's second best (Cont's two builders come from the exactly
    // searched careers), or a saving within PROXY_SAVE of the shown rows (the page's top OPT_SHOW) or the pick's near-tie
    // band. A proxy's odds cannot exceed the exact best (it is a career she can run); its efficiency can read high
    // or low. Calibration: docs/MODEL.md, "Next career"; `proxy_check` measures the current errors.
    // `exact_all` restores a search for every uma, and so does a job where proxies were measured too low for the band
    // (`exact_builders`).
    let proxies_enabled = settings.surrogate_mode == 0
        && !settings.exact_all
        && !settings.proxy_check
        && !exact_builders(sim.env.setup.target_count, legacies, input.pool.max_borrowed);
    let candidates = PlanCandidates {
        main_career,
        pick_career,
        loop_members: loop_rotation.members,
        loop_member_info: loop_rotation.member_info,
        loop_member_note: loop_rotation.note,
        hint,
        cut,
        trainees: candidate_trainees,
        stage_count,
        allowed_trainees,
        careers,
        current_odds,
        certified,
        has_full_veteran,
        // (a side-only uma's infinite: no lift of hers is gathered)
        placeholder_step_costs: (0..candidate_count)
            .map(|candidate| if candidate < stage_count { 1.0 } else { f64::INFINITY })
            .collect(),
        proxies_enabled,
        knobs: settings.knobs(),
        step_caps,
        cap_pool,
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

/// The stage-1 candidates (uma indices), the page's record of those the ceiling ranking added (`hint`) and of those it
/// left out (`cut`), and the side-only umas.
struct StageCandidates {
    trainees: Vec<crate::uma::UmaIndex>,
    hint: Vec<Value>,
    cut: Vec<Value>,
    /// Umas with a usable real full veteran that are not stage candidates (`side_only_umas`), ascending.
    side_only: Vec<crate::uma::UmaIndex>,
}

/// Stage-1 candidates (who A can be; whose odds a builder may lift): the best career's trainee, the named loop members,
/// the umas of the best career's lineage (its parents', then its grandparents', each generation by uma index), the
/// trainees of the main search's careers (best first), then the allowed umas by their odds on today's veterans
/// (`today_umas`: on the main search's careers' parent pairs), then the rest by their full-loop ceiling
/// (`hint_umas`: its lineage seeded with the first two of these); no deck search, equal odds by uma index; at most
/// IDX_CANDS.
/// None of it reads the veterans' order, only which veterans the best career and the main search's careers take (an
/// exact tie between two careers' odds aside, which the main search breaks by its own order). The ranked umas
/// past the last slot are `cut`. A given fit (`PlanJob::fit`) names them instead. Then the side-only umas.
/// Jobs whose builders are all searched exactly, as `exact_all` searches them: at `EXACT_BUILDERS_TARGETS` targets or
/// more, or with a borrowed veteran in the pool that the job lets a career use (`borrowed_parent` not "never": a pool
/// whose borrowed veterans can't be used plans as the pool without them). There proxies read further below the exact careers than the band
/// allows (measured 2026-10-27, `proxy_check`: efficiency 46% low on val/plan p16_t10, 10 targets of which 8 uncertain;
/// 6.4% low on p16_borrow, against the 6% band), and the exact searches cost little or save time (p16_t10 13.8 s exact
/// against 19.3 s with proxies, p16_borrow 1.3 against 1.2 s); elsewhere proxies stay (exact costs p10 3.37 against
/// 2.21 s, p16 1.38 against 0.83, gm16_nu8, 8 uncertain targets, 5.59 against 3.75)
fn exact_builders(target_count: usize, legacies: &[crate::career::Legacy], max_borrowed: u8) -> bool {
    target_count >= EXACT_BUILDERS_TARGETS || (max_borrowed > 0 && legacies.iter().any(|legacy| legacy.borrowed))
}

/// The fewest targets at which builders are searched exactly (`exact_builders`): the measured case is p16_t10's 10
/// targets (8 uncertain); gm16_nu8's 8 uncertain of 8 keeps its proxies within the band
const EXACT_BUILDERS_TARGETS: usize = 10;

fn stage_candidates(
    env: &mut Env,
    input: &PlanInputs,
    main_career: &Found,
    loop_members: &[crate::uma::UmaIndex],
) -> StageCandidates {
    let legacies = input.legacies;
    let settings = input.settings;
    let data = input.resources.data;
    let mut trainees: Vec<crate::uma::UmaIndex> = vec![main_career.trainee];
    let push = |uma: crate::uma::UmaIndex, candidates: &mut Vec<crate::uma::UmaIndex>| {
        if !candidates.contains(&uma) && input.trainees.contains(&uma) {
            candidates.push(uma);
        }
    };
    for &member in loop_members {
        push(member, &mut trainees);
    }
    // (the main career's parents' umas, then their own parents', each pair of generations by uma index: the parents'
    // positions in the legacies change no candidate, and `hint_umas` seeds its ceiling lineage with the first two
    // distinct candidates)
    let parents = [&legacies[main_career.first_parent], &legacies[main_career.second_parent]];
    let umas_by_index = |characters: &mut dyn Iterator<Item = u32>| {
        let mut umas: Vec<crate::uma::UmaIndex> =
            characters.filter_map(|character_id| env.setup.uma_index(character_id)).collect();
        umas.sort_unstable();
        umas
    };
    let parent_umas = umas_by_index(&mut parents.iter().map(|parent| parent.character_id));
    let grandparent_umas = umas_by_index(
        &mut parents.iter().flat_map(|parent| parent.parents.iter().map(|grandparent| grandparent.character_id)),
    );
    for uma in parent_umas.into_iter().chain(grandparent_umas) {
        push(uma, &mut trainees);
    }
    // (the main search's careers, best first: their trainees' odds now are known with no search; on p11_noagnes the
    // ceilings below sit within 2.5% of each other and alone cut V's B, Special Week, raising V 2.2%)
    for found in &input.solved.found {
        push(found.trainee, &mut trainees);
    }
    trainees.truncate(IDX_CANDS);
    let (mut hint, mut cut): (Vec<Value>, Vec<Value>) = (vec![], vec![]);
    if let Some(fit) = &settings.fit {
        // (each uma once, in the fit's order, at most IDX_CANDS: the planner's candidate limit, which the scoring's
        // memory budget assumes)
        trainees = vec![];
        for uma in fit.iter().filter_map(|entry| env.setup.uma_index(entry.character_id())) {
            if input.trainees.contains(&uma) && !trainees.contains(&uma) && trainees.len() < IDX_CANDS {
                trainees.push(uma);
            }
        }
    } else {
        // the remaining slots by the planner's own model, no deck search: the allowed umas ranked by their odds on
        // today's veterans (`today_umas`: on the main search's careers' parents), then those with no legal one
        // by their odds with a full loop lineage (`hint_umas`); each in uma order where the odds are equal
        let mut allowed = input.trainees.to_vec();
        allowed.sort_unstable();
        let free_slots = IDX_CANDS - trainees.len();
        let pairs: Vec<(usize, usize)> =
            input.solved.found.iter().map(|found| (found.first_parent, found.second_parent)).collect();
        let today = today_umas(&env.setup, &mut env.state, input.pool, &trainees, &allowed, &pairs);
        let today_ranked: Vec<crate::uma::UmaIndex> = today.iter().map(|uma| uma.trainee).collect();
        let rest: Vec<crate::uma::UmaIndex> =
            allowed.iter().copied().filter(|uma| !today_ranked.contains(uma)).collect();
        let full_loop = hint_umas(&env.setup, &mut env.state, input.pool, &trainees, &rest, usize::MAX);
        let ranked =
            today.into_iter().map(|uma| (uma, "today")).chain(full_loop.into_iter().map(|uma| (uma, "full_loop")));
        for (rank, (HintUma { trainee, ceiling }, basis)) in ranked.enumerate() {
            let record = json!({
                "id": data.umas[trainee].id, "name": data.umas[trainee].name, "ceiling": ceiling, "basis": basis
            });
            if rank < free_slots {
                trainees.push(trainee);
                hint.push(record);
            } else {
                cut.push(record);
            }
        }
    }
    let side_only = side_only_umas(env, input, &trainees);
    StageCandidates { trainees, hint, cut, side_only }
}

/// The umas, not among `stage`, with a real full veteran a career may take as a parent (borrowed ones where the
/// borrowing rule lets one alone be a parent), whatever the trainee filter, ascending: candidates whose only sides are
/// those veterans (free), with no career, no step, no continuation and no lift slots (`PlanCandidates::stage_count`)
fn side_only_umas(env: &Env, input: &PlanInputs, stage: &[crate::uma::UmaIndex]) -> Vec<crate::uma::UmaIndex> {
    let skill_job = env.setup.skill_job;
    let mut umas: Vec<crate::uma::UmaIndex> = input
        .legacies
        .iter()
        .filter(|legacy| is_full(skill_job, &legacy.sparks) && input.pool.alone_within_borrow_limit(legacy.borrowed))
        .filter_map(|legacy| env.setup.uma_index(legacy.character_id))
        .filter(|uma| !stage.contains(uma))
        .collect();
    umas.sort_unstable();
    umas.dedup();
    umas
}

impl PlanInputs<'_, '_> {
    /// The plan's searches before any is run: the main search's best career known, and the kept plans of this pool and of
    /// this pool less its last veteran taken out of `KeptState::bests`
    fn new_searches(
        &self,
        kept: &mut crate::career::state::KeptState,
        main_career: &Found,
        env_budget: usize,
        space_key: u64,
    ) -> PlanSearches {
        let uma_count = self.resources.data.umas.len();
        let legacy_count = self.legacies.len();
        let mut best_careers = crate::uma::UmaTable(vec![CachedSearch::Unsearched; uma_count]);
        best_careers[main_career.trainee] = CachedSearch::Known(Some(main_career.clone()));
        let kept_same_pool =
            take_kept_plan(kept, bests_key(self.resources.job, self.fixed_parents, legacy_count, space_key));
        let kept_previous_pool = if legacy_count > 1 {
            take_kept_plan(kept, bests_key(self.resources.job, self.fixed_parents, legacy_count - 1, space_key))
        } else {
            None
        };
        PlanSearches {
            best_careers,
            proof_refs: crate::uma::UmaTable(vec![crate::uma::UmaTable(vec![]); uma_count]),
            base_incomplete: crate::uma::UmaTable(vec![false; uma_count]),
            looped_incomplete: crate::uma::UmaTable(vec![false; uma_count]),
            no_borrow_incomplete: crate::uma::UmaTable(vec![false; uma_count]),
            looped_careers: crate::uma::UmaTable(vec![]),
            kept_same_pool,
            kept_previous_pool,
            env_budget,
            pair_careers: crate::FxMap::default(),
        }
    }
}

impl<'input> PlanInputs<'input, '_> {
    const fn input_parents(&self) -> ParentPool<'input> {
        ParentPool { pool: self.pool, fixed_parents: self.fixed_parents }
    }
}

/// A winning deck of the main search or a candidate's best career, which a builder of the same deck class may use.
pub(crate) struct ClassWinner {
    trainee: crate::uma::UmaIndex,
    deck: SelectedDeck,
}

/// The main search's winners, then each candidate's best career (in candidate order)
pub(crate) fn class_winners(
    solved: &CareerSearchResult,
    candidates: &PlanCandidates,
    best_careers: &crate::uma::UmaTable<CachedSearch>,
) -> Vec<ClassWinner> {
    solved
        .found
        .iter()
        .map(|found| ClassWinner { trainee: found.trainee, deck: found.deck.clone() })
        .chain(candidates.trainees.iter().filter_map(|&trainee| {
            best_careers[trainee].found().map(|found| ClassWinner { trainee, deck: found.deck.clone() })
        }))
        .collect()
}

/// Builder `trainee`'s proxy career: her deck class's winning decks added to her cached decks, then her best career
/// through the surrogate (`best_sur`, no refresh)
pub(crate) fn proxy_career(
    simulator: &mut Sim,
    winners: &[ClassWinner],
    parents: ParentPool,
    trainee: crate::uma::UmaIndex,
    buf: &mut Vec<f64>,
) -> Option<Found> {
    let deck_class = simulator.env.setup.deck_class[trainee];
    for winner in winners {
        if winner.trainee != trainee && simulator.env.setup.deck_class[winner.trainee] == deck_class {
            simulator.evaluator.cache.add(
                &simulator.env.setup,
                &mut simulator.env.state,
                &mut simulator.env.memo,
                trainee,
                &winner.deck,
            );
        }
    }
    best_sur(simulator, parents, trainee, 1e9, buf)
}

/// Pass 1: every allowed uma's best career; per candidate, the chance her veteran raises that candidate's odds and the
/// mean odds raised to (exact over her outcomes). Then the near-top builders' proxies searched exactly, and each row's
/// trainee's G1 wins.
pub(crate) fn score_and_admit_proxies(
    simulator: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> PlanRows {
    let mut rows = pass_one_rows(simulator, context, searches, profile, buf);
    if context.candidates.proxies_enabled {
        admit_near_top_proxies(simulator, context, searches, profile, &mut rows, buf);
    }
    let mut wins_by_trainee = crate::uma::UmaTable(vec![vec![]; context.data().umas.len()]);
    for row in &rows {
        set_plan_wins(simulator, context.input.pool, &mut wins_by_trainee, row.trainee, &row.career);
    }
    PlanRows { rows, wins_by_trainee }
}

/// Every allowed uma's base row, the main career's trainee first: a proxy while proxies are on and she is unsearched,
/// else her searched best career. (Placeholder step costs: 1, so every candidate's lifts are gathered; a full veteran of hers
/// may be a poorer parent than a fresh one.)
fn pass_one_rows(
    simulator: &mut Sim,
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
    let mut order: Vec<crate::uma::UmaIndex> = vec![candidates.main_career.trainee];
    for &trainee in &candidates.allowed_trainees {
        if trainee != candidates.main_career.trainee {
            order.push(trainee);
        }
    }
    for &trainee in &order {
        progress(2, order.iter().position(|&other| other == trainee).unwrap_or(0), order.len()); // pass 1: uma k of n
        if candidates.proxies_enabled && searches.best_careers[trainee].is_unsearched() {
            let proxy_started_ms = now_ms();
            let proxy = proxy_career(simulator, &winners, context.parents(), trainee, buf);
            profile.search_ms += now_ms() - proxy_started_ms;
            let Some(career) = proxy else { continue };
            let (CareerScore { lift_stats, .. }, stats_stamp) =
                stamped(simulator, Some(&candidates.trainees), |simulator| {
                    score_career(
                        simulator,
                        &context.scoring(&stats_base),
                        found_career(trainee, &career),
                        Want::Stats,
                        buf,
                    )
                });
            rows.push(PlanRow::proxy(trainee, career, lift_stats, RowOrigin::Base, stats_stamp));
            continue;
        }
        if searches.best_careers[trainee].is_unsearched()
            || (settings.surrogate_mode == 2 && !candidates.trainees.contains(&trainee))
        {
            let search_started_ms = now_ms();
            capped(simulator, settings.search_work, searches.env_budget, |simulator| {
                if settings.surrogate_mode != 1 && searches.best_careers[trainee].is_unsearched() {
                    search_best_career(simulator, context, searches, trainee);
                }
                if settings.surrogate_mode > 0
                    && let Some(difference) = plan_surrogate(simulator, input, &mut searches.best_careers, trainee, buf)
                {
                    profile.surrogate_differences.push(difference);
                }
            });
            profile.search_ms += now_ms() - search_started_ms;
        }
        let Some(career) = searches.best_careers[trainee].found().cloned() else { continue };
        let CareerScore { lift_stats, .. } =
            score_career(simulator, &context.scoring(&stats_base), found_career(trainee, &career), Want::Stats, buf);
        rows.push(PlanRow::exact(trainee, career, lift_stats, RowOrigin::Base));
    }
    rows
}

/// A row's efficiency for one candidate (`LiftStats::efficiency`).
#[derive(Clone, Copy)]
struct RowEfficiency {
    efficiency: f64,
    /// Position in the plan's rows.
    row: usize,
}

/// The near-top builders searched exactly: per candidate every proxy whose efficiency reaches (1 - PROXY_EFF) of the
/// candidate's second best (exact and proxy careers together), until none is left
fn admit_near_top_proxies(
    simulator: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    rows: &mut [PlanRow],
    buf: &mut Vec<f64>,
) {
    loop {
        let admitted = context.near_top_proxies(rows);
        if admitted.is_empty() {
            break;
        }
        let search_started_ms = now_ms();
        for row in admitted {
            make_row_exact(simulator, context, searches, profile, &mut rows[row], buf);
        }
        profile.search_ms += now_ms() - search_started_ms;
    }
}

impl PlanContext<'_, '_> {
    /// The proxy rows within PROXY_EFF of a candidate's second most efficient row, in row order
    fn near_top_proxies(&self, rows: &[PlanRow]) -> Vec<usize> {
        let PlanContext { input, candidates } = *self;
        let data = self.data();
        let mut admitted: Vec<usize> = vec![];
        for candidate in 0..candidates.trainees.len() {
            if candidates.current_odds[candidate] <= 0.0 {
                continue;
            }
            let (trainee, odds) = (candidates.trainees[candidate], candidates.current_odds[candidate]);
            let mut efficiencies: Vec<RowEfficiency> = rows
                .iter()
                .enumerate()
                .filter(|(_, plan_row)| {
                    data.umas[plan_row.trainee].id != data.umas[trainee].id
                        && plan_row.lift_stats[candidate].lift_chance > 0.0
                })
                .map(|(row, plan_row)| RowEfficiency {
                    efficiency: plan_row.lift_stats[candidate].efficiency(odds),
                    row,
                })
                .collect();
            efficiencies.sort_by(|first, second| second.efficiency.partial_cmp(&first.efficiency).unwrap());
            let Some(best) = efficiencies.first() else { continue };
            let second = efficiencies.get(1).map_or(best.efficiency, |runner_up| runner_up.efficiency);
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
    simulator: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    row: &mut PlanRow,
    buf: &mut Vec<f64>,
) -> MadeExact {
    let trainee = row.trainee;
    // a proxy the class's proofs certify needs no search (it is her best career's value)
    if certified(&simulator.env.setup, &mut simulator.env.state, context.parents(), trainee, &row.career) {
        searches.best_careers[trainee] = CachedSearch::Known(Some(row.career.clone()));
        row.exact = true;
        profile.certified_proxies += 1;
        return MadeExact::Certified;
    }
    let exact = exact_plan_career(simulator, context, searches, row, buf);
    let replaced = exact.is_some();
    if let Some(ExactPlanCareer { career, lift_stats }) = exact {
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
    simulator: &mut Sim,
    input: &PlanInputs,
    best_careers: &mut crate::uma::UmaTable<CachedSearch>,
    trainee: crate::uma::UmaIndex,
    buf: &mut Vec<f64>,
) -> Option<Value> {
    let settings = input.settings;
    let surrogate = best_sur(simulator, input.input_parents(), trainee, settings.sur_margin, buf);
    if settings.surrogate_mode != 2 {
        best_careers[trainee] = CachedSearch::Known(surrogate);
        return None;
    }
    let searched = best_careers[trainee].found().cloned();
    let (searched_value, surrogate_value) =
        (searched.as_ref().map_or(0.0, |career| career.value), surrogate.as_ref().map_or(0.0, |career| career.value));
    let same = searched.as_ref().map(Found::unordered_parents) == surrogate.as_ref().map(Found::unordered_parents);
    if !same || (searched_value - surrogate_value).abs() > 1e-9 * searched_value.max(surrogate_value) {
        let data = input.resources.data;
        return Some(
            json!({"trainee": data.umas[trainee].name, "search": [searched.as_ref().map(|career| [career.first_parent, career.second_parent]), searched_value], "sur": [surrogate.as_ref().map(|career| [career.first_parent, career.second_parent]), surrogate_value]}),
        );
    }
    None
}

/// An exact career and its lifts per candidate.
pub(crate) struct ExactPlanCareer {
    pub(crate) career: Found,
    pub(crate) lift_stats: Vec<LiftStats>,
}

/// The proxy row's trainee's exact best career (searched now if not yet) and its lifts. The proxy's lifts are taken as
/// they are when the search found the same career and nothing they read has changed (their stamp, `stamped`, over the
/// candidates' decks: this base reads no other's)
fn exact_plan_career(
    simulator: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    proxy: &PlanRow,
    buf: &mut Vec<f64>,
) -> Option<ExactPlanCareer> {
    let trainee = proxy.trainee;
    let settings = context.settings();
    if searches.best_careers[trainee].is_unsearched() {
        capped(simulator, settings.search_work, searches.env_budget, |simulator| {
            search_best_career(simulator, context, searches, trainee);
        });
    }
    let career = searches.best_careers[trainee].found().cloned()?;
    if same_career(&proxy.career, &career) && stamp_ok(simulator, Some(&context.candidates.trainees), proxy.stats_stamp)
    {
        prof("reuse_st", 1.0);
        return Some(ExactPlanCareer { career, lift_stats: proxy.lift_stats.clone() });
    }
    let stats_base = context.candidates.stats_base();
    let CareerScore { lift_stats, .. } =
        score_career(simulator, &context.scoring(&stats_base), found_career(trainee, &career), Want::Stats, buf);
    Some(ExactPlanCareer { career, lift_stats })
}

/// Record the G1 win chances of trainee's career (`PlanRows::wins_by_trainee`)
pub(crate) fn set_plan_wins(
    simulator: &mut Sim,
    pool: &Pool,
    wins_by_trainee: &mut crate::uma::UmaTable<Vec<f64>>,
    trainee: crate::uma::UmaIndex,
    career: &Found,
) {
    let record = simulator.evaluator.career_wins(
        &simulator.env.setup,
        &mut simulator.env.state,
        pool,
        crate::career::plan::found_career(trainee, career),
    );
    wins_by_trainee[trainee] = record.win_chances.to_vec();
}

/// The trainee's best career without a borrowed parent, on the pick's pool: the surrogate's under `sur` 1, else
/// searched (not reusing kept plans)
pub(crate) fn exact_no_borrow_career(
    simulator: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    trainee: crate::uma::UmaIndex,
    buf: &mut Vec<f64>,
) -> Option<Found> {
    let settings = context.settings();
    let no_kept_plans = KeptPlans { same_pool: None, previous_pool: None, legacy_count: context.legacy_count() };
    capped(simulator, settings.search_work, searches.env_budget, |simulator| {
        if settings.surrogate_mode == 1 {
            best_sur(simulator, context.pick_parents(), trainee, settings.sur_margin, buf)
        } else {
            let search = best_of(simulator.env, context.pick_parents(), trainee, no_kept_plans);
            if !search.complete {
                searches.no_borrow_incomplete[trainee] = true;
            }
            search.found
        }
    })
}
