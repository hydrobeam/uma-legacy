//! Candidate searches, rotation setup and proxy limits.
use super::PlanJob;
use crate::deck::career::sim::{
    Ancestor, Bar, CardsRaw, Data, Env, Found, Legacy, Pool, Sim, SimJob, SkillJob, SkillsRaw, Value, json,
};

pub(in crate::deck::career::sim::plan) const fn p_margin() -> f64 {
    1e9
}

pub(in crate::deck::career::sim::plan) const fn p_swork() -> usize {
    5_000_000
}

pub(in crate::deck::career::sim::plan) fn sim_for<'env, 'data>(
    env: &'env mut Env<'data>,
    d: &'data Data,
    sk: &'data SkillsRaw,
    cr: &'data CardsRaw,
    job: &'data Value,
    pj: &PlanJob,
) -> Result<Sim<'env, 'data>, Value> {
    let ntar = env.ntar;
    if ntar > 12 || env.nu > 10 {
        return Err(json!({"error": "The plan takes at most 12 targets, 10 of them not guaranteed by the scenario."}));
    }
    let cfg: SimJob = serde_json::from_value(json!({"margin": pj.margin}))
        .map_err(|e| json!({"error": format!("The plan's margin does not make a simulator setting: {e}")}))?;
    // as `simulate` builds it (the continuations are the player's later careers: every owned trainee)
    let trainees = env.owned.clone();
    Ok(Sim::new(env, d, sk, cr, job, cfg, trainees, vec![], vec![], false, true))
}

/// The rotation for the loop base `ids` (`PlanJob::loop_members`): (members as uma indices, their {id, name}, or why
/// there is none). Members must be owned umas with a legal deck, at least three of them.
pub(in crate::deck::career::sim::plan) fn rotation(
    env: &Env,
    d: &Data,
    ids: &[u32],
) -> (Vec<usize>, Vec<Value>, Option<String>) {
    let info: Vec<Value> = ids
        .iter()
        .map(|&id| {
            let u = d.umas.iter().find(|u| u.id == id);
            json!({"id": id, "name": u.map_or_else(|| format!("Uma {id}"), |u| u.name.clone())})
        })
        .collect();
    if info.is_empty() {
        return (vec![], vec![], Some("no loop base (loop_members empty)".into()));
    }
    let idx: Vec<Option<usize>> = info.iter().map(|m| m["id"].as_u64().and_then(|id| env.uix(id as u32))).collect();
    if idx.len() < 3 || idx.iter().any(|x| x.is_none_or(|t| !env.owned.contains(&t) || env.dmax[t].is_none())) {
        return (vec![], info, Some("the loop's members must be at least three owned umas with a legal deck".into()));
    }
    (idx.into_iter().map(|x| x.unwrap()).collect(), info, None)
}

/// Candidates the index rule weighs at most, and partners kept per trainee where a candidate's veteran is a parent
pub(in crate::deck::career::sim::plan) const IDX_CANDS: usize = 8;

pub(in crate::deck::career::sim::plan) const IDX_PARTNERS: usize = 2;

/// legal parents a, b for trainee u (`Pool::pair_ok`)
pub(in crate::deck::career::sim::plan) fn legal_pair(d: &Data, p: &Pool, u: usize, a: usize, b: usize) -> bool {
    p.pair_ok(d.umas[u].id, a, b)
}

/// Whether builder x's career f (her deck class's proxy) is provably her best, without a career search: over every legal
/// pair one of the bounds `solve` prunes with (`Env::ceiling_out`; `refs_out` over the fully checked decks of her class,
/// exact for the class; `ruled_out` over its product proofs) is at most f's value (`Bar::AtMost`), so no deck and pair
/// beats it (a different deck or pair of equal value may exist; the value is the search's). With mob or affinity-free
/// parents every classmate has the same chances, so one classmate's search proves the rest of her class.
pub(in crate::deck::career::sim::plan) fn certified(
    env: &mut Env,
    pool: &Pool,
    fix: &[usize],
    x: usize,
    f: &Found,
) -> bool {
    let ct = env.cls[x];
    // (each test computed alone, so the order the pairs are taken in changes no number; a NaN bound proves nothing)
    let pairs = env.gen_cands(pool, fix, &[x], |_, _| true, |ub, _, _, a, b| (ub, a, b));
    let mut q = Vec::with_capacity(env.ntar);
    for (ub, a, b) in pairs {
        let bar = Bar::AtMost(f.val);
        if Env::ceiling_out(ub, bar) {
            continue;
        }
        let c = env.lineage_into(pool, x, a, b, &mut q);
        if env.refs_out(x, &q, c, env.seen[ct].iter(), bar) || env.ruled_out(x, &q, c, &env.ruled[ct], bar) {
            continue;
        }
        return false;
    }
    true
}

/// trainee u's P(all) with parents a, b (cached decks; a deck search where she has none)
pub(in crate::deck::career::sim::plan) fn val(
    s: &mut Sim,
    p: &Pool,
    u: usize,
    a: usize,
    b: usize,
    buf: &mut Vec<f64>,
) -> f64 {
    let mut q = std::mem::take(&mut s.qs);
    let c = s.env.lineage_into(p, u, a, b, &mut q);
    let v = match s.env.ceiling(u, &q, c) {
        None => 0.0,
        Some(ub) => s.deck_for(u, &q, c, ub, 0.0, buf).map_or(0.0, |x| x.0),
    };
    s.qs = q;
    v
}

/// Trainee u's best career through the surrogate (`deck_for`, sim module notes): every legal pair (all of `fix` among
/// the parents), highest ceiling first, valued by her cached decks, an exact deck search where the bound beats the table
/// by more than `margin`; stops when the next ceiling is below the best. margin 0: P(all) at least the exact best, up
/// to deck searches that run out of work.
pub(in crate::deck::career::sim::plan) fn best_sur(
    s: &mut Sim,
    pool: &Pool,
    fix: &[usize],
    u: usize,
    margin: f64,
    buf: &mut Vec<f64>,
) -> Option<Found> {
    let m0 = s.cfg.margin;
    s.cfg.margin = margin;
    // (the best so far as its pair, value, ceiling and cached deck index: her cached decks only grow, so the index stays
    // valid, and the deck is copied once, at the end)
    let mut best: Option<(usize, usize, f64, f64, usize)> = None;
    for (ub, a, b) in s.env.ranked_pairs(pool, fix, u, |_, _| true) {
        let bar = best.map_or(0.0, |f| f.2);
        if ub <= bar {
            break;
        }
        let mut q = std::mem::take(&mut s.qs);
        let c = s.env.lineage_into(pool, u, a, b, &mut q);
        let r = s.deck_for(u, &q, c, ub, bar, buf);
        s.qs = q;
        if let Some((v, di)) = r
            && v > bar
        {
            best = Some((a, b, v, ub, di));
        }
    }
    s.cfg.margin = m0;
    best.map(|(a, b, val, ub, di)| Found { t: u, a, b, val, ub, sel: s.cache.decks[u][di].sel.clone() })
}

/// an 8/8 veteran (a spark of every job target), yours or borrowed
pub(in crate::deck::career::sim::plan) fn is_full(sj: &SkillJob, sparks: &[(u32, u8)]) -> bool {
    sj.targets.iter().all(|t| sparks.iter().any(|x| x.0 == t.group && x.1 > 0))
}

/// One place a builder's new veteran may be a parent: the stage-1 candidate she may lift (index), per uncertain target
/// (env.uncv order) the miss product without her, per job target the spark scale without and with her spark, her miss
/// factor x per star count (0: absent), how many of the trainee's cached decks it reads (their corner tables; a trainee's
/// cached decks only grow, so the first `nd` are the ones there when the setup was made)
pub(in crate::deck::career::sim::plan) struct Setup {
    pub(in crate::deck::career::sim::plan) ui: usize,
    pub(in crate::deck::career::sim::plan) x0: Vec<f64>,
    pub(in crate::deck::career::sim::plan) pw: Vec<[f64; 2]>,
    pub(in crate::deck::career::sim::plan) xm: [f64; 4],
    pub(in crate::deck::career::sim::plan) nd: usize,
}

/// A builder's lift efficiency for a candidate at odds p: her veteran lifts the candidate with chance pl to odds sp / pl
/// on average (`score_career`'s stats), so pl (1/p - 1/(sp/pl)) careers saved per career (Cont's builders, the proxies'
/// admission and `proxy_check`; PROXY_EFF was calibrated on it)
pub(in crate::deck::career::sim::plan) fn lift_eff(pl: f64, sp: f64, p: f64) -> f64 {
    pl * (1.0 / p - 1.0 / (sp / pl))
}

/// `f` under the plan's career-search work cap `work` (`PlanJob::search_work`; 0: none), the env's own budget `budget0`
/// put back after (and no hard cap)
pub(in crate::deck::career::sim::plan) fn capped<'env, 'data, R>(
    s: &mut Sim<'env, 'data>,
    work: usize,
    budget0: usize,
    f: impl FnOnce(&mut Sim<'env, 'data>) -> R,
) -> R {
    if work > 0 {
        s.env.budget = work;
        s.env.hard = work;
    }
    let result = f(s);
    s.env.hard = 0;
    s.env.budget = budget0;
    result
}

/// Builders through their deck class (`index_rule`): a proxy is searched exactly when her lift efficiency is within this
/// share of a candidate's second best, or her saving within this share of the lowest saving the shown rows or the pick's
/// near-tie band take. Recalibrated 2026-10-09 (val/plan `proxy_check`, with the own-8/8 credit): the proxy is
/// the exact career for every builder on five of the seven jobs; it reads at most 2.79% low on a contender's efficiency
/// (exact at least 0.7 of a candidate's second best; p11) and 2.67% low on a builder's saving (p11): 0.06 is 2.15x and
/// 2.25x those. (The first measurement, on the planner as it was before the class winners and the merges since, had 11%
/// and 9%, with 0.2.) The errors move with the model: run `proxy_check` again after a change to what builders' careers
/// are made of
pub(in crate::deck::career::sim::plan) const PROXY_EFF: f64 = 0.06;

pub(in crate::deck::career::sim::plan) const PROXY_SAVE: f64 = 0.06;

/// the rows the page shows (career.js `OPT_SHOW`): their careers are searched exactly
pub(in crate::deck::career::sim::plan) const OPT_SHOW: usize = 8;

/// Trainee u's odds with a full loop lineage: parents hypothetical veterans of b1 and b2 (every target, stars at the
/// rank's mix, G1 wins `w1` / `w2`), each bred from the other and from u (grandparents every target at 2 stars)
pub(in crate::deck::career::sim::plan) fn loop_ceiling(
    s: &mut Sim,
    d: &Data,
    pool: &Pool,
    u: usize,
    b1: usize,
    b2: usize,
    wins: &dyn Fn(usize) -> Vec<usize>,
    buf: &mut Vec<f64>,
) -> f64 {
    let sj = s.env.sj;
    let star = s.env.sj.star_dist;
    let full: Vec<(u32, u8)> = sj.targets.iter().map(|t| (t.group, 2)).collect();
    let anc = |x: usize| Ancestor { chara: d.umas[x].id, sparks: full.clone(), wins: wins(x), pinks: vec![] };
    let mk = |x: usize, p1: usize, p2: usize| Legacy {
        chara: d.umas[x].id,
        borrowed: false,
        sparks: full.clone(),
        wins: wins(x),
        pinks: vec![],
        parents: vec![anc(p1), anc(p2)],
        win_p: vec![],
        mix: Some(star),
    };
    // (`Env::pool` of legs and the two: the working copy of `pool` with them pushed, then popped)
    let mut p = s.take_pool(pool);
    s.env.pool_push(&mut p, &mk(b1, b2, u));
    s.env.pool_push(&mut p, &mk(b2, b1, u));
    let n = p.me.len();
    let v = if legal_pair(d, &p, u, n - 2, n - 1) { val(s, &p, u, n - 2, n - 1, buf) } else { 0.0 };
    Env::pool_pop(&mut p);
    Env::pool_pop(&mut p);
    s.put_pool(pool, p);
    v
}

/// Below this many stage candidates the plan adds umas by `hint_umas` (the loop planner's search added up to three; a
/// pool with veterans of its loop's umas gives four or more by the other routes: 6-8 on the val/plan pools)
pub(in crate::deck::career::sim::plan) const HINT_FLOOR: usize = 4;

/// The owned umas (a legal deck, not among `cands`) with the highest odds under a full loop lineage, at most `k`, best
/// first: her parents hypothetical veterans of the first two candidates' umas (every target, stars at the rank's mix),
/// each bred from the other and from her, as `loop_ceiling` builds p_inf's lineage (a missing candidate: an ancestor of
/// no uma, so no relation points); no G1 wins on the hypothetical ancestors (the race part of their affinity is the
/// same for every uma ranked, and the wins come from schedules the pool doesn't have yet). Valued by `Env::ceiling` (the
/// per-target best deck odds: no deck search), which ranks the umas the way a full loop would before any building.
pub(in crate::deck::career::sim::plan) fn hint_umas(
    env: &mut Env,
    d: &Data,
    pool: &Pool,
    cands: &[usize],
    trainees: &[usize],
    k: usize,
) -> Vec<(usize, f64)> {
    let sj = env.sj;
    let star = sj.star_dist;
    let full: Vec<(u32, u8)> = sj.targets.iter().map(|t| (t.group, 2)).collect();
    let mut sides: Vec<u32> = vec![];
    for &c in cands {
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
    let (ca, cb) = (sides[0], sides[1]);
    let anc = |chara: u32| Ancestor { chara, sparks: full.clone(), wins: vec![], pinks: vec![] };
    let mk = |chara: u32, p1: u32, p2: u32| Legacy {
        chara,
        borrowed: false,
        sparks: full.clone(),
        wins: vec![],
        pinks: vec![],
        parents: vec![anc(p1), anc(p2)],
        win_p: vec![],
        mix: Some(star),
    };
    let mut out: Vec<(usize, f64)> = vec![];
    // (`Env::pool` of the legacies and the two: one copy of `pool`, `Env::pool(legs)`, the two pushed and popped per uma)
    let mut p = pool.clone();
    for &u in trainees {
        let id = d.umas[u].id;
        if cands.contains(&u) || env.dmax[u].is_none() || id == ca || id == cb {
            continue;
        }
        env.pool_push(&mut p, &mk(ca, cb, id));
        env.pool_push(&mut p, &mk(cb, ca, id));
        let n = p.me.len();
        if legal_pair(d, &p, u, n - 2, n - 1) {
            let (q, c) = env.lineage_q(&p, u, n - 2, n - 1);
            if let Some(v) = env.ceiling(u, &q, c)
                && v > 0.0
            {
                out.push((u, v));
            }
        }
        Env::pool_pop(&mut p);
        Env::pool_pop(&mut p);
    }
    // best first; equal odds in uma order (stable)
    out.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap());
    out.truncate(k);
    out
}
