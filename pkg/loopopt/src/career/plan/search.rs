//! Candidate searches, rotation setup and proxy limits.
use crate::career::evaluator::DeckRequest;
use crate::career::plan::{ParentPool, PlanJob};
use crate::career::pool::Pool;
use crate::career::search::{Bar, Found, RankedPair};
use crate::career::sim::{Sim, SimJob, SimulationOptions, SimulationRoster};
use crate::career::{Ancestor, CareerSetup, Env, JobInputs, Legacy, Lineage};
use crate::data::Data;
use crate::skills::SkillJob;
use serde_json::{Value, json};
pub(crate) const fn p_margin() -> f64 {
    1e9
}

pub(crate) const fn p_swork() -> usize {
    5_000_000
}

pub(crate) fn sim_for<'env, 'data>(
    env: &'env mut Env<'data>,
    kept: &'env mut crate::career::state::KeptState,
    input: &JobInputs<'data>,
    settings: &PlanJob,
) -> Result<Sim<'env, 'data>, Value> {
    if env.setup.target_count > 12 || env.setup.uncertain_count > 10 {
        return Err(json!({"error": "The plan takes at most 12 targets, 10 of them not guaranteed by the scenario."}));
    }
    if !(1..=crate::career::plan::cont::MAX_PARTNERS).contains(&settings.partners)
        || !(1..=8).contains(&settings.builder_careers)
        || !(1..=64).contains(&settings.fresh_lineages)
    {
        return Err(
            json!({"error": "next.partners takes 1 to 8, next.builder_careers 1 to 8, next.fresh_lineages 1 to 64."}),
        );
    }
    // (the partner and lift slots' leaf rows within the scoring's memory, `OUTCOME_BUDGET`; with their corner tables
    // past it, the setups ranked last are left out, `score::build::fit_partner_setups`)
    if (2 * settings.partners * IDX_CANDS) << (2 * env.setup.uncertain_count)
        > crate::career::plan::score::build::OUTCOME_BUDGET
    {
        return Err(
            json!({"error": "next.partners: too many partners for the scoring's memory at this many uncertain targets."}),
        );
    }
    let sim_job: SimJob = serde_json::from_value(json!({"margin": settings.margin}))
        .map_err(|error| json!({"error": format!("The plan's margin does not make a simulator setting: {error}")}))?;
    // as `simulate` builds it (the continuations are the player's later careers: every owned trainee)
    let trainees = env.setup.owned.clone();
    Ok(Sim::new(
        env,
        kept,
        input,
        sim_job,
        SimulationRoster { trainees, members: vec![], member_by_trainee: crate::uma::UmaTable(vec![]) },
        SimulationOptions { record_audits: false, fill_missing_decks: true },
    ))
}

/// The rotation for a loop base (`PlanJob::loop_members`), or why there is none.
pub(crate) struct LoopRotation {
    /// Members as uma indices (`Data::umas`); empty when the base is refused.
    pub(crate) members: Vec<crate::uma::UmaIndex>,
    /// Each named member's {id, name}, for the page.
    pub(crate) member_info: Vec<Value>,
    /// Why there is no rotation.
    pub(crate) note: Option<String>,
}

/// The rotation for the loop base `ids` (chara ids). Members must be owned umas with a legal deck, at least three of
/// them.
pub(crate) fn rotation(setup: &CareerSetup, ids: &[u32]) -> LoopRotation {
    let member_info: Vec<Value> = ids
        .iter()
        .map(|&id| {
            let uma = setup.data.umas.iter().find(|roster_uma| roster_uma.id == id);
            json!({"id": id, "name": uma.map_or_else(|| format!("Uma {id}"), |roster_uma| roster_uma.name.clone())})
        })
        .collect();
    if member_info.is_empty() {
        return LoopRotation {
            members: vec![],
            member_info: vec![],
            note: Some("no loop base (loop_members empty)".into()),
        };
    }
    let members: Vec<Option<crate::uma::UmaIndex>> =
        member_info.iter().map(|info| info["id"].as_u64().and_then(|id| setup.uma_index(id as u32))).collect();
    if members.len() < 3
        || members.iter().any(|member| {
            member.is_none_or(|trainee| !setup.owned.contains(&trainee) || setup.deck_odds_ceilings[trainee].is_none())
        })
    {
        return LoopRotation {
            members: vec![],
            member_info,
            note: Some("the loop's members must be at least three owned umas with a legal deck".into()),
        };
    }
    LoopRotation { members: members.into_iter().map(|member| member.unwrap()).collect(), member_info, note: None }
}

/// Candidates the index rule weighs at most
pub(crate) const IDX_CANDS: usize = 8;

/// legal parents a, b for trainee u (`Pool::pair_ok`)
pub(crate) fn legal_pair(data: &Data, pool: &Pool, lineage: Lineage) -> bool {
    pool.pair_ok(data.umas[lineage.trainee].id, lineage.first_parent, lineage.second_parent)
}

/// Whether `builder`'s `career` (her deck class's proxy) is provably her best, without a career search: over every
/// legal pair one of the bounds `solve` prunes with (`CareerSetup::ceiling_out`; `refs_out` over the fully checked
/// decks of her class, exact for the class; `ruled_out` over its product proofs) is at most its value (`Bar::AtMost`),
/// so no deck and pair beats it (a different deck or pair of equal value may exist; the value is the search's). With
/// mob or affinity-free parents every classmate has the same chances, so one classmate's search proves the rest of her
/// class.
pub(crate) fn certified(
    setup: &crate::career::CareerSetup,
    state: &mut crate::career::SearchState,
    parents: ParentPool,
    builder: crate::uma::UmaIndex,
    career: &Found,
) -> bool {
    let ParentPool { pool, fixed_parents } = parents;
    let deck_class = setup.deck_class[builder];
    // (a breached class certifies nothing: `ClassFlags::proves`)
    if !state.class_flags[deck_class].proves() {
        return false;
    }
    // (each test computed alone, so the order the pairs are taken in changes no number; a NaN bound proves nothing)
    let pairs = setup.gen_cands(
        state,
        pool,
        fixed_parents,
        &[builder],
        |_, _| true,
        |upper_bound, _, _, first_parent, second_parent| (upper_bound, first_parent, second_parent),
    );
    let mut q = Vec::with_capacity(setup.target_count);
    for (upper_bound, first_parent, second_parent) in pairs {
        let bar = Bar::AtMost(career.value);
        if CareerSetup::ceiling_out(upper_bound, bar) {
            continue;
        }
        let spark_scale = setup.lineage_into(pool, Lineage { trainee: builder, first_parent, second_parent }, &mut q);
        if setup.refs_out(builder, &q, spark_scale, state.seen[deck_class].iter(), bar)
            || setup.ruled_out(builder, &q, spark_scale, &state.ruled[deck_class], bar)
        {
            continue;
        }
        return false;
    }
    true
}

/// trainee u's P(all) with parents a, b (cached decks; a deck search where she has none)
pub(crate) fn val(simulator: &mut Sim, pool: &Pool, lineage: Lineage, score_buffer: &mut Vec<f64>) -> f64 {
    let trainee = lineage.trainee;
    let mut inheritance_chances = std::mem::take(&mut simulator.evaluator.inheritance_buffer);
    let spark_scale = simulator.env.setup.lineage_into(pool, lineage, &mut inheritance_chances);
    let value = match simulator.env.setup.ceiling(&mut simulator.env.state, trainee, &inheritance_chances, spark_scale)
    {
        None => 0.0,
        Some(ceiling) => simulator
            .evaluator
            .deck_for(
                simulator.env,
                DeckRequest {
                    trainee,
                    inheritance_chances: &inheritance_chances,
                    spark_scale,
                    ceiling,
                    threshold: 0.0,
                },
                score_buffer,
            )
            .map_or(0.0, |result| result.0),
    };
    simulator.evaluator.inheritance_buffer = inheritance_chances;
    value
}

/// Trainee u's best career through the surrogate (`deck_for`, sim module notes): every legal pair (all of `fix` among
/// the parents), highest ceiling first, valued by her cached decks, an exact deck search where the bound beats the table
/// by more than `margin`; stops when the next ceiling is below the best. margin 0: P(all) at least the exact best, up
/// to deck searches that run out of work.
pub(crate) fn best_sur(
    simulator: &mut Sim,
    parents: ParentPool,
    trainee: crate::uma::UmaIndex,
    margin: f64,
    score_buffer: &mut Vec<f64>,
) -> Option<Found> {
    let ParentPool { pool, fixed_parents } = parents;
    let margin_before = simulator.evaluator.margin;
    simulator.evaluator.margin = margin;
    // (her cached decks only grow, so the best's deck index stays valid, and the deck is copied once, at the end)
    let mut best: Option<SurrogateBest> = None;
    for RankedPair { ceiling, first_parent, second_parent } in
        simulator.env.setup.ranked_pairs(&mut simulator.env.state, pool, fixed_parents, trainee, |_, _| true)
    {
        let bar = best.map_or(0.0, |found| found.value);
        if ceiling <= bar {
            break;
        }
        let lineage = Lineage { trainee, first_parent, second_parent };
        let mut inheritance_chances = std::mem::take(&mut simulator.evaluator.inheritance_buffer);
        let spark_scale = simulator.env.setup.lineage_into(pool, lineage, &mut inheritance_chances);
        let deck = simulator.evaluator.deck_for(
            simulator.env,
            DeckRequest { trainee, inheritance_chances: &inheritance_chances, spark_scale, ceiling, threshold: bar },
            score_buffer,
        );
        simulator.evaluator.inheritance_buffer = inheritance_chances;
        if let Some((value, deck_index)) = deck
            && value > bar
        {
            best = Some(SurrogateBest { lineage, value, ceiling, deck_index });
        }
    }
    simulator.evaluator.margin = margin_before;
    best.map(|found| Found {
        trainee,
        first_parent: found.lineage.first_parent,
        second_parent: found.lineage.second_parent,
        value: found.value,
        ceiling: found.ceiling,
        deck: simulator.evaluator.cache.decks[trainee][found.deck_index].deck.clone(),
    })
}

/// Trainee u's next `count` best careers through her cached decks (as `best_sur` at margin 1e9: no refresh), on pairs
/// other than `exclude` (unordered), best first, ties in pair order: every legal pair by ceiling, valued until the next
/// ceiling is at most the `count`-th best so far. Each is a career she can run (a real deck on a legal pair) at the odds
/// stated, at most her best career's on that pair
pub(crate) fn alternative_careers(
    simulator: &mut Sim,
    parents: ParentPool,
    trainee: crate::uma::UmaIndex,
    exclude: (usize, usize),
    count: usize,
    score_buffer: &mut Vec<f64>,
) -> Vec<Found> {
    let ParentPool { pool, fixed_parents } = parents;
    if count == 0 {
        return vec![];
    }
    let margin_before = simulator.evaluator.margin;
    simulator.evaluator.margin = 1e9;
    let mut kept: Vec<SurrogateBest> = vec![];
    for RankedPair { ceiling, first_parent, second_parent } in
        simulator.env.setup.ranked_pairs(&mut simulator.env.state, pool, fixed_parents, trainee, |_, _| true)
    {
        if (first_parent.min(second_parent), first_parent.max(second_parent)) == exclude {
            continue;
        }
        let bar = if kept.len() == count { kept[count - 1].value } else { 0.0 };
        if ceiling <= bar {
            break;
        }
        let lineage = Lineage { trainee, first_parent, second_parent };
        let mut inheritance_chances = std::mem::take(&mut simulator.evaluator.inheritance_buffer);
        let spark_scale = simulator.env.setup.lineage_into(pool, lineage, &mut inheritance_chances);
        let deck = simulator.evaluator.deck_for(
            simulator.env,
            DeckRequest { trainee, inheritance_chances: &inheritance_chances, spark_scale, ceiling, threshold: bar },
            score_buffer,
        );
        simulator.evaluator.inheritance_buffer = inheritance_chances;
        if let Some((value, deck_index)) = deck
            && value > bar
        {
            let place = kept.iter().position(|career| value > career.value).unwrap_or(kept.len());
            kept.insert(place, SurrogateBest { lineage, value, ceiling, deck_index });
            kept.truncate(count);
        }
    }
    simulator.evaluator.margin = margin_before;
    kept.into_iter()
        .map(|career| Found {
            trainee,
            first_parent: career.lineage.first_parent,
            second_parent: career.lineage.second_parent,
            value: career.value,
            ceiling: career.ceiling,
            deck: simulator.evaluator.cache.decks[trainee][career.deck_index].deck.clone(),
        })
        .collect()
}

/// Trainee u's next `count` best careers as distinct (parent pair, cached deck) careers other than `base` (her best:
/// its pair and deck), best first, ties in pair then deck order: every legal pair by ceiling, each of her cached decks
/// valued there (its corner tables at the pair's chances, as `Evaluator::tval` values them: no deck search, no
/// refresh), until the next ceiling is at most the `count`-th best so far. So a lower-odds deck on her best career's own
/// parents counts as well as another pair (`PlanJob::builder_careers`)
pub(crate) fn alternative_deck_careers(
    simulator: &mut Sim,
    parents: ParentPool,
    base: &Found,
    count: usize,
) -> Vec<Found> {
    let ParentPool { pool, fixed_parents } = parents;
    let trainee = base.trainee;
    let mut kept: Vec<Found> = vec![];
    let (mut chances, mut scratch) = (vec![], vec![]);
    let base_pair = base.unordered_parents();
    for RankedPair { ceiling, first_parent, second_parent } in
        simulator.env.setup.ranked_pairs(&mut simulator.env.state, pool, fixed_parents, trainee, |_, _| true)
    {
        let bar = if kept.len() == count { kept[count - 1].value } else { 0.0 };
        if count == 0 || ceiling <= bar {
            break;
        }
        let lineage = Lineage { trainee, first_parent, second_parent };
        let spark_scale = simulator.env.setup.lineage_into(pool, lineage, &mut chances);
        let on_base_pair = (first_parent.min(second_parent), first_parent.max(second_parent)) == base_pair;
        let misses: Vec<f64> = simulator.env.setup.uncertain_targets.iter().map(|&i| 1.0 - chances[i]).collect();
        for cached in &simulator.evaluator.cache.decks[trainee] {
            if on_base_pair && crate::career::plan::kept::same_deck(&cached.deck, &base.deck) {
                continue;
            }
            let value = spark_scale
                * cached
                    .tables
                    .iter()
                    .map(|table| crate::career::outcomes::interp_in(table, &misses, &mut scratch))
                    .fold(0.0f64, f64::max);
            let bar = if kept.len() == count { kept[count - 1].value } else { 0.0 };
            if value > bar {
                let place = kept.iter().position(|career| value > career.value).unwrap_or(kept.len());
                kept.insert(
                    place,
                    Found { trainee, first_parent, second_parent, value, ceiling, deck: cached.deck.clone() },
                );
                kept.truncate(count);
            }
        }
    }
    kept
}

/// `best_sur`'s best career so far.
#[derive(Clone, Copy)]
struct SurrogateBest {
    lineage: Lineage,
    /// P(all) through her cached decks.
    value: f64,
    /// The pair's ceiling.
    ceiling: f64,
    /// Position of the deck in her cached decks.
    deck_index: usize,
}

/// a full veteran (a spark of every job target), yours or borrowed
pub(crate) fn is_full(skill_job: &SkillJob, sparks: &[(u32, u8)]) -> bool {
    skill_job.targets.iter().all(|target| sparks.iter().any(|spark| spark.0 == target.group && spark.1 > 0))
}

/// `searches` under the plan's career-search work cap `work` (`PlanJob::search_work`; 0: none), the env's own budget
/// `env_budget` put back after (and no hard cap)
pub(crate) fn capped<'env, 'data, R>(
    simulator: &mut Sim<'env, 'data>,
    work: usize,
    env_budget: usize,
    searches: impl FnOnce(&mut Sim<'env, 'data>) -> R,
) -> R {
    if work > 0 {
        simulator.env.state.budget = work;
        simulator.env.state.hard_work_cap = work;
    }
    let result = searches(simulator);
    simulator.env.state.hard_work_cap = 0;
    simulator.env.state.budget = env_budget;
    result
}

/// Builders through their deck class (`index_rule`): a proxy is searched exactly when her lift efficiency is within this
/// share of a candidate's second best, or her saving within this share of the lowest saving the shown rows or the pick's
/// near-tie band take. Calibration: docs/MODEL.md, "Next career". Run `proxy_check` again after changing how
/// builders' careers are chosen or scored.
pub(crate) const PROXY_EFF: f64 = 0.06;

pub(crate) const PROXY_SAVE: f64 = 0.06;

/// the rows the page shows (career.js `OPT_SHOW`): their careers are searched exactly
pub(crate) const OPT_SHOW: usize = 8;

/// A full loop lineage for `loop_ceiling`, as uma indices (`Data::umas`): the trainee, and the two umas whose
/// hypothetical veterans are her parents.
#[derive(Clone, Copy)]
pub(crate) struct FullLoop {
    pub(crate) trainee: crate::uma::UmaIndex,
    pub(crate) first_builder: crate::uma::UmaIndex,
    pub(crate) second_builder: crate::uma::UmaIndex,
}

/// A hypothetical full veteran of a character: every job target at the rank's star mix, G1 wins at `win_chances` (as
/// `CareerSetup::veteran` makes the fresh ones), no pinks; as a legacy with `parents`, or as an ancestor
struct HypotheticalVeteran<'veteran> {
    character_id: u32,
    full: &'veteran [(u32, u8)],
    star_mix: [f64; 3],
    win_chances: Vec<f64>,
}

impl HypotheticalVeteran<'_> {
    fn ancestor(&self) -> Ancestor {
        Ancestor {
            character_id: self.character_id,
            sparks: self.full.to_vec(),
            wins: vec![],
            pinks: vec![],
            win_chances: self.win_chances.clone(),
            mix: Some(self.star_mix),
            pink_chances: vec![],
        }
    }
    fn legacy(&self, parents: [Ancestor; 2]) -> Legacy {
        Legacy {
            character_id: self.character_id,
            borrowed: false,
            sparks: self.full.to_vec(),
            wins: vec![],
            pinks: vec![],
            parents: parents.to_vec(),
            win_chances: self.win_chances.clone(),
            mix: Some(self.star_mix),
            pink_chances: vec![],
        }
    }
}

/// The trainee's odds with a full loop lineage: parents hypothetical full veterans of the two builders, each bred from
/// the other and from her, every one of them (parents and grandparents) as a fresh veteran is (`HypotheticalVeteran`:
/// the rank's star mix, the G1 wins of the uma's row's career at their chances, `wins`)
pub(crate) fn loop_ceiling(
    simulator: &mut Sim,
    pool: &Pool,
    full_loop: FullLoop,
    wins: &dyn Fn(crate::uma::UmaIndex) -> Vec<f64>,
    buf: &mut Vec<f64>,
) -> f64 {
    let FullLoop { trainee, first_builder, second_builder } = full_loop;
    let data = simulator.data;
    let skill_job = simulator.env.setup.skill_job;
    let star_mix = simulator.env.setup.skill_job.star_dist;
    let full: Vec<(u32, u8)> = skill_job.targets.iter().map(|target| (target.group, 2)).collect();
    let hypothetical = |uma: crate::uma::UmaIndex| HypotheticalVeteran {
        character_id: data.umas[uma].id,
        full: &full,
        star_mix,
        win_chances: wins(uma),
    };
    let veteran =
        |uma: crate::uma::UmaIndex, first_parent: crate::uma::UmaIndex, second_parent: crate::uma::UmaIndex| {
            hypothetical(uma).legacy([hypothetical(first_parent).ancestor(), hypothetical(second_parent).ancestor()])
        };
    // (`CareerSetup::pool` of the legacies and the two: the working copy of `pool` with them pushed, then popped)
    let mut working_pool = simulator.evaluator.take_pool(pool);
    simulator.env.setup.pool_push(&mut working_pool, &veteran(first_builder, second_builder, trainee));
    simulator.env.setup.pool_push(&mut working_pool, &veteran(second_builder, first_builder, trainee));
    let n = working_pool.veterans.len();
    let lineage = Lineage { trainee, first_parent: n - 2, second_parent: n - 1 };
    let value =
        if legal_pair(data, &working_pool, lineage) { val(simulator, &working_pool, lineage, buf) } else { 0.0 };
    working_pool.pop_veteran();
    working_pool.pop_veteran();
    simulator.evaluator.put_pool(pool, working_pool);
    value
}

/// An uma `hint_umas` ranks for the stage candidates' remaining slots.
pub(crate) struct HintUma {
    /// Index in `Data::umas`.
    pub(crate) trainee: crate::uma::UmaIndex,
    /// Her ranking odds (`CareerSetup::ceiling_uniform`): under the full loop lineage (`hint_umas`) or on today's
    /// veterans (`today_umas`).
    pub(crate) ceiling: f64,
}

/// The owned umas (a legal deck, not among `candidates`) with the highest odds under a full loop lineage, at most
/// `count`, best first: her parents hypothetical veterans of the first two candidates' umas, each bred from the other
/// and from her, every one of them at the rank's star mix, as `loop_ceiling` builds p_inf's lineage (a missing
/// candidate: an ancestor of no uma, so no relation points); no G1 wins on them (no schedule of theirs is known before
/// the candidates are searched: the race part of the affinity is left out for every uma ranked). Valued by
/// `CareerSetup::ceiling` (the per-target best deck odds: no deck search), which ranks the umas the way a full loop
/// would before any building.
pub(crate) fn hint_umas(
    setup: &crate::career::CareerSetup,
    state: &mut crate::career::SearchState,
    pool: &Pool,
    candidates: &[crate::uma::UmaIndex],
    trainees: &[crate::uma::UmaIndex],
    count: usize,
) -> Vec<HintUma> {
    let data = setup.data;
    let skill_job = setup.skill_job;
    let star = skill_job.star_dist;
    let full: Vec<(u32, u8)> = skill_job.targets.iter().map(|target| (target.group, 2)).collect();
    let mut sides: Vec<u32> = vec![];
    for &candidate in candidates {
        let id = data.umas[candidate].id;
        if !sides.contains(&id) {
            sides.push(id);
        }
        if sides.len() == 2 {
            break;
        }
    }
    while sides.len() < 2 {
        sides.push(0);
    }
    let (first_side, second_side) = (sides[0], sides[1]);
    let hypothetical =
        |character_id: u32| HypotheticalVeteran { character_id, full: &full, star_mix: star, win_chances: vec![] };
    let veteran = |character_id: u32, first_parent: u32, second_parent: u32| {
        hypothetical(character_id)
            .legacy([hypothetical(first_parent).ancestor(), hypothetical(second_parent).ancestor()])
    };
    let mut out: Vec<HintUma> = vec![];
    // (`CareerSetup::pool` of the legacies and the two: one copy of `pool`, `CareerSetup::pool(legacies)`, the two pushed
    // and popped per uma)
    let mut working_pool = pool.clone();
    for &trainee in trainees {
        let id = data.umas[trainee].id;
        if candidates.contains(&trainee)
            || setup.deck_odds_ceilings[trainee].is_none()
            || id == first_side
            || id == second_side
        {
            continue;
        }
        setup.pool_push(&mut working_pool, &veteran(first_side, second_side, id));
        setup.pool_push(&mut working_pool, &veteran(second_side, first_side, id));
        let n = working_pool.veterans.len();
        let lineage = Lineage { trainee, first_parent: n - 2, second_parent: n - 1 };
        if legal_pair(data, &working_pool, lineage) {
            let (q, spark_scale) = setup.lineage_q(&working_pool, lineage);
            if let Some(ceiling) = setup.ceiling_uniform(state, trainee, &q, spark_scale)
                && ceiling > 0.0
            {
                out.push(HintUma { trainee, ceiling });
            }
        }
        working_pool.pop_veteran();
        working_pool.pop_veteran();
    }
    // best first; equal odds in uma order (stable)
    out.sort_by(|first, second| second.ceiling.partial_cmp(&first.ceiling).unwrap());
    out.truncate(count);
    out
}

/// The owned umas (a legal deck, not among `candidates`) by their odds on today's veterans, best first: each one's
/// `CareerSetup::ceiling_uniform` on the best of `pairs` (the main search's careers' parents) she may take
/// (`legal_pair`), no deck search; umas with none of them legal are left out. The full-loop ceiling (`hint_umas`) sits
/// within ~1% across umas on the exported rosters (export 4: 0.004956 to 0.004903 over the five slots and the first
/// cuts), so it cut Gold Ship, a trio side on today's veterans, and raised V 3.6-4.3% on both rosters without borrows;
/// these pairs restore her there. Used where the builders are proxies (no borrows, under 10 targets); elsewhere every
/// uma is searched exactly first and the slots rank by the careers found (`setup_plan_candidates`): there these pairs
/// are the main trainee's (Agnes Digital's, on borrowed full veterans) and left Gold Ship cut, and each uma's best
/// ceiling over every pair ranked worse still (2026-10-27, .review/planner/harness/candfix)
pub(crate) fn today_umas(
    setup: &crate::career::CareerSetup,
    state: &mut crate::career::SearchState,
    pool: &Pool,
    candidates: &[crate::uma::UmaIndex],
    trainees: &[crate::uma::UmaIndex],
    pairs: &[(usize, usize)],
) -> Vec<HintUma> {
    let mut out: Vec<HintUma> = vec![];
    for &trainee in trainees {
        if candidates.contains(&trainee) || setup.deck_odds_ceilings[trainee].is_none() {
            continue;
        }
        let best = pairs
            .iter()
            .map(|&(first_parent, second_parent)| Lineage { trainee, first_parent, second_parent })
            .filter(|&lineage| legal_pair(setup.data, pool, lineage))
            .filter_map(|lineage| {
                let (q, spark_scale) = setup.lineage_q(pool, lineage);
                setup.ceiling_uniform(state, trainee, &q, spark_scale)
            })
            .fold(0.0_f64, f64::max);
        if best > 0.0 {
            out.push(HintUma { trainee, ceiling: best });
        }
    }
    // best first; equal odds in uma order (stable)
    out.sort_by(|first, second| second.ceiling.total_cmp(&first.ceiling));
    out
}
