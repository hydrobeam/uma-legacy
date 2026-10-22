//! Candidate searches, rotation setup and proxy limits.
use crate::career::evaluator::DeckRequest;
use crate::career::plan::{ParentPool, PlanJob};
use crate::career::pool::Pool;
use crate::career::search::{Bar, Found, RankedPair};
use crate::career::sim::{Sim, SimJob, SimulationOptions, SimulationRoster};
use crate::career::{Ancestor, Env, JobInputs, Legacy, Lineage};
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
    input: &JobInputs<'data>,
    settings: &PlanJob,
) -> Result<Sim<'env, 'data>, Value> {
    if env.target_count > 12 || env.uncertain_count > 10 {
        return Err(json!({"error": "The plan takes at most 12 targets, 10 of them not guaranteed by the scenario."}));
    }
    let cfg: SimJob = serde_json::from_value(json!({"margin": settings.margin}))
        .map_err(|e| json!({"error": format!("The plan's margin does not make a simulator setting: {e}")}))?;
    // as `simulate` builds it (the continuations are the player's later careers: every owned trainee)
    let trainees = env.owned.clone();
    Ok(Sim::new(
        env,
        input,
        cfg,
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
pub(crate) fn rotation(env: &Env, ids: &[u32]) -> LoopRotation {
    let member_info: Vec<Value> = ids
        .iter()
        .map(|&id| {
            let uma = env.data.umas.iter().find(|u| u.id == id);
            json!({"id": id, "name": uma.map_or_else(|| format!("Uma {id}"), |u| u.name.clone())})
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
        member_info.iter().map(|m| m["id"].as_u64().and_then(|id| env.uma_index(id as u32))).collect();
    if members.len() < 3
        || members.iter().any(|x| x.is_none_or(|t| !env.owned.contains(&t) || env.deck_odds_ceilings[t].is_none()))
    {
        return LoopRotation {
            members: vec![],
            member_info,
            note: Some("the loop's members must be at least three owned umas with a legal deck".into()),
        };
    }
    LoopRotation { members: members.into_iter().map(|x| x.unwrap()).collect(), member_info, note: None }
}

/// Candidates the index rule weighs at most, and partners kept per trainee where a candidate's veteran is a parent
pub(crate) const IDX_CANDS: usize = 8;

pub(crate) const IDX_PARTNERS: usize = 2;

/// legal parents a, b for trainee u (`Pool::pair_ok`)
pub(crate) fn legal_pair(data: &Data, pool: &Pool, lineage: Lineage) -> bool {
    pool.pair_ok(data.umas[lineage.trainee].id, lineage.first_parent, lineage.second_parent)
}

/// Whether builder x's career f (her deck class's proxy) is provably her best, without a career search: over every legal
/// pair one of the bounds `solve` prunes with (`Env::ceiling_out`; `refs_out` over the fully checked decks of her class,
/// exact for the class; `ruled_out` over its product proofs) is at most f's value (`Bar::AtMost`), so no deck and pair
/// beats it (a different deck or pair of equal value may exist; the value is the search's). With mob or affinity-free
/// parents every classmate has the same chances, so one classmate's search proves the rest of her class.
pub(crate) fn certified(env: &mut Env, parents: ParentPool, x: crate::uma::UmaIndex, f: &Found) -> bool {
    let ParentPool { pool, fixed_parents } = parents;
    let deck_class = env.deck_class[x];
    // (each test computed alone, so the order the pairs are taken in changes no number; a NaN bound proves nothing)
    let pairs = env.gen_cands(pool, fixed_parents, &[x], |_, _| true, |ub, _, _, a, b| (ub, a, b));
    let mut q = Vec::with_capacity(env.target_count);
    for (ub, a, b) in pairs {
        let bar = Bar::AtMost(f.value);
        if Env::ceiling_out(ub, bar) {
            continue;
        }
        let c = env.lineage_into(pool, Lineage { trainee: x, first_parent: a, second_parent: b }, &mut q);
        if env.refs_out(x, &q, c, env.seen[deck_class].iter(), bar)
            || env.ruled_out(x, &q, c, &env.ruled[deck_class], bar)
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
    let mut inheritance_chances = std::mem::take(&mut simulator.evaluator.qs);
    let spark_scale = simulator.env.lineage_into(pool, lineage, &mut inheritance_chances);
    let value = match simulator.env.ceiling(trainee, &inheritance_chances, spark_scale) {
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
            .map_or(0.0, |x| x.0),
    };
    simulator.evaluator.qs = inheritance_chances;
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
        simulator.env.ranked_pairs(pool, fixed_parents, trainee, |_, _| true)
    {
        let bar = best.map_or(0.0, |found| found.value);
        if ceiling <= bar {
            break;
        }
        let lineage = Lineage { trainee, first_parent, second_parent };
        let mut inheritance_chances = std::mem::take(&mut simulator.evaluator.qs);
        let spark_scale = simulator.env.lineage_into(pool, lineage, &mut inheritance_chances);
        let deck = simulator.evaluator.deck_for(
            simulator.env,
            DeckRequest { trainee, inheritance_chances: &inheritance_chances, spark_scale, ceiling, threshold: bar },
            score_buffer,
        );
        simulator.evaluator.qs = inheritance_chances;
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
        deck: simulator.evaluator.cache.decks[trainee][found.deck_index].sel.clone(),
    })
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
pub(crate) fn is_full(sj: &SkillJob, sparks: &[(u32, u8)]) -> bool {
    sj.targets.iter().all(|t| sparks.iter().any(|x| x.0 == t.group && x.1 > 0))
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
        simulator.env.budget = work;
        simulator.env.hard_work_cap = work;
    }
    let result = searches(simulator);
    simulator.env.hard_work_cap = 0;
    simulator.env.budget = env_budget;
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

/// The trainee's odds with a full loop lineage: parents hypothetical veterans of the two builders (every target, stars
/// at the rank's mix, G1 wins `wins` of each), each bred from the other and from her (grandparents every target at 2
/// stars)
pub(crate) fn loop_ceiling(
    simulator: &mut Sim,
    pool: &Pool,
    full_loop: FullLoop,
    wins: &dyn Fn(crate::uma::UmaIndex) -> Vec<usize>,
    buf: &mut Vec<f64>,
) -> f64 {
    let FullLoop { trainee, first_builder, second_builder } = full_loop;
    let d = simulator.d;
    let sj = simulator.env.skill_job;
    let star = simulator.env.skill_job.star_dist;
    let full: Vec<(u32, u8)> = sj.targets.iter().map(|t| (t.group, 2)).collect();
    let ancestor =
        |x: crate::uma::UmaIndex| Ancestor { chara: d.umas[x].id, sparks: full.clone(), wins: wins(x), pinks: vec![] };
    let veteran = |x: crate::uma::UmaIndex, p1: crate::uma::UmaIndex, p2: crate::uma::UmaIndex| Legacy {
        chara: d.umas[x].id,
        borrowed: false,
        sparks: full.clone(),
        wins: wins(x),
        pinks: vec![],
        parents: vec![ancestor(p1), ancestor(p2)],
        win_p: vec![],
        mix: Some(star),
    };
    // (`Env::pool` of legs and the two: the working copy of `pool` with them pushed, then popped)
    let mut p = simulator.evaluator.take_pool(pool);
    simulator.env.pool_push(&mut p, &veteran(first_builder, second_builder, trainee));
    simulator.env.pool_push(&mut p, &veteran(second_builder, first_builder, trainee));
    let n = p.veterans.len();
    let lineage = Lineage { trainee, first_parent: n - 2, second_parent: n - 1 };
    let v = if legal_pair(d, &p, lineage) { val(simulator, &p, lineage, buf) } else { 0.0 };
    p.pop_veteran();
    p.pop_veteran();
    simulator.evaluator.put_pool(pool, p);
    v
}

/// Below this many stage candidates the plan adds umas by `hint_umas`.
pub(crate) const HINT_FLOOR: usize = 4;

/// An uma `hint_umas` adds to the stage candidates.
pub(crate) struct HintUma {
    /// Index in `Data::umas`.
    pub(crate) trainee: crate::uma::UmaIndex,
    /// Her odds under the full loop lineage (`Env::ceiling`).
    pub(crate) ceiling: f64,
}

/// The owned umas (a legal deck, not among `candidates`) with the highest odds under a full loop lineage, at most
/// `count`, best first: her parents hypothetical veterans of the first two candidates' umas (every target, stars at the
/// rank's mix), each bred from the other and from her, as `loop_ceiling` builds p_inf's lineage (a missing candidate: an
/// ancestor of no uma, so no relation points); no G1 wins on the hypothetical ancestors (the race part of their
/// affinity is the same for every uma ranked, and the wins come from schedules the pool doesn't have yet). Valued by
/// `Env::ceiling` (the per-target best deck odds: no deck search), which ranks the umas the way a full loop would before
/// any building.
pub(crate) fn hint_umas(
    env: &mut Env,
    pool: &Pool,
    candidates: &[crate::uma::UmaIndex],
    trainees: &[crate::uma::UmaIndex],
    count: usize,
) -> Vec<HintUma> {
    let d = env.data;
    let sj = env.skill_job;
    let star = sj.star_dist;
    let full: Vec<(u32, u8)> = sj.targets.iter().map(|t| (t.group, 2)).collect();
    let mut sides: Vec<u32> = vec![];
    for &c in candidates {
        let id = d.umas[c].id;
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
    let ancestor = |chara: u32| Ancestor { chara, sparks: full.clone(), wins: vec![], pinks: vec![] };
    let veteran = |chara: u32, p1: u32, p2: u32| Legacy {
        chara,
        borrowed: false,
        sparks: full.clone(),
        wins: vec![],
        pinks: vec![],
        parents: vec![ancestor(p1), ancestor(p2)],
        win_p: vec![],
        mix: Some(star),
    };
    let mut out: Vec<HintUma> = vec![];
    // (`Env::pool` of the legacies and the two: one copy of `pool`, `Env::pool(legs)`, the two pushed and popped per uma)
    let mut p = pool.clone();
    for &u in trainees {
        let id = d.umas[u].id;
        if candidates.contains(&u) || env.deck_odds_ceilings[u].is_none() || id == first_side || id == second_side {
            continue;
        }
        env.pool_push(&mut p, &veteran(first_side, second_side, id));
        env.pool_push(&mut p, &veteran(second_side, first_side, id));
        let n = p.veterans.len();
        let lineage = Lineage { trainee: u, first_parent: n - 2, second_parent: n - 1 };
        if legal_pair(d, &p, lineage) {
            let (q, c) = env.lineage_q(&p, lineage);
            if let Some(ceiling) = env.ceiling(u, &q, c)
                && ceiling > 0.0
            {
                out.push(HintUma { trainee: u, ceiling });
            }
        }
        p.pop_veteran();
        p.pop_veteran();
    }
    // best first; equal odds in uma order (stable)
    out.sort_by(|x, y| y.ceiling.partial_cmp(&x.ceiling).unwrap());
    out.truncate(count);
    out
}
