//! Career planner: the best next career from the veterans a player actually has (their own, plus borrowed ones).
//!
//! The loop planner assumes a running loop: every legacy carries every target spark (stars from the rank mix) and
//! race overlaps come from planned schedules. Here each candidate career (trainee T, parents L1 and L2 from the
//! veteran list, at most one borrowed) uses what those veterans really carry:
//!   - affinity per legacy from relation points and the G1 wins on record: a parent P (other parent Q, P's own
//!     parents G) counts pair(T,P) + pair(P,Q) + 3 per G1 shared by P and Q + sum over G of [tri(T,P,G) + 3 per G1
//!     shared by P and G]; a grandparent G under P counts tri(T,P,G) + 3 per G1 shared by P and G (skills.rs, same
//!     fit); a veteran whose parents are unknown adds no grandparent slots;
//!   - per target k, the inheritance chance q_k = 1 - prod over the six legacies (1 - min(1, 0.03 s (1 + A/100)))^2,
//!     s = the stars of that legacy's spark of k (0 if it has none);
//!   - spark generation 0.2 / 0.4 x 1.1^n_k for a white / gold, n_k = how many of the six carry a spark of k.
//!
//! Everything else is the loop planner's model. q_k and the trainee's own event hint chance ew enter every formula
//! only as (1 - ew)(1 - q_k), so T's event hint chances are replaced by 1 - (1 - ew)(1 - q_k) and the deck search
//! runs at q = 0: its products, bounds and exact odds are then the career's, apart from the factor 1.1^(n_k - 6) per
//! target, which is the same for every deck and multiplies the result (`JointExt` applies it per target where the
//! number is not a plain product: P(all but one), the per-target chances).
//!
//! Search: every legal candidate gets a ceiling, EXACT_RHO x C x the product value at each target's best possible
//! deck odds (the deck_size lowest miss chances of any legal entries, links firing for sure); candidates are solved
//! best ceiling first (best product deck, then every deck whose product value times EXACT_RHO reaches the best exact
//! value, scored exactly, as in the per-uma search) until the next ceiling is below the top-k exact value.
//!
//! Schedule: every G1 win counts one, plus one per parent who won that race (`best_response` weights), under the same
//! goals, race clashes and streak penalties as the loop scheduler; the report lists every G1 she could enter.
//!
//! The next career the page picks (`next`) is `sim::plan::index_rule`; the simulator (`simulate`) is `sim`.
use crate::deck::{
    CHAIN_EVENTS, CHAIN_N, CHAIN_P, Card, CardsRaw, Data, DeckJob, DeckSpace, Deserialize, FxMap, GEN_GOLD, HashMap,
    Instant, Model, Sel, SkillJob, SkillsRaw, Value, d_true, deck_entries, deck_space, json, src_miss,
};
use clock::{PROF, now_ms, prof, progress};
pub(crate) use decks::CfStats;
use decks::{DECK_KEEP, JOB_NO};
use decks::{DECK_KEEP_CAP, DeckKey, DeckMemo, Kept};
pub(crate) use inherit::HerSlot;
use inherit::{
    RelOf, SlotSink, aff_factor, inherit, inherit_q, insp_miss, mix_term, part_sum, rise_factor, set_ews, spark_scale,
};
use outcomes::interp_in;
use pool::{Pool, Who, dominates, holds_fix, shared};
use schedule::{apt_pink, pink_p};
use search::{Bar, DeckRes, Found, Ref, SearchRequest, Solved, solve_kept};
use state::{LAST_SOLVE, SEEN};
use state::{
    Ruled, SEEN_KEEP, SOLVE_KEEP, STATE_MAX_N, found_in, found_out, fvec, fvec_r, fx, hx, len_in, pvec_r, sel_in,
    sel_out, setup_hash, state_in, state_out, usz, ux, uxr,
};
pub(crate) use state::{json_without, setup_fp};
mod clock;
mod decks;
mod inherit;
mod outcomes;
mod pool;
mod schedule;
mod search;
mod state;

mod cache;

mod sim;

/// A trainee and her parents in the supplied legacy pool; parent order is significant.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Lineage {
    /// Trainee's index in Data::umas.
    pub(crate) trainee: usize,
    /// First parent's index in the supplied pool or legacy slice.
    pub(crate) first_parent: usize,
    /// Second parent's index, preserving the caller's order.
    pub(crate) second_parent: usize,
}

impl Lineage {
    /// The same career with the lower legacy index as the first parent.
    pub(crate) fn ordered(self) -> Self {
        Self {
            trainee: self.trainee,
            first_parent: self.first_parent.min(self.second_parent),
            second_parent: self.first_parent.max(self.second_parent),
        }
    }
}

/// A borrowed deck and the lineage that will train with it.
#[derive(Clone, Copy)]
pub(crate) struct Career<'deck> {
    /// Trainee and ordered parents.
    pub(crate) lineage: Lineage,
    /// Selected support deck, borrowed without cloning.
    pub(crate) deck: &'deck Sel,
}

/// Immutable resources for one solver job.
pub(crate) struct JobInputs<'data> {
    /// Roster, affinities and race data.
    pub(crate) data: &'data Data,
    /// Skill and outfit source data.
    pub(crate) skills: &'data SkillsRaw,
    /// Support card source data.
    pub(crate) cards: &'data CardsRaw,
    /// Original job settings, including planner settings.
    pub(crate) job: &'data Value,
}

#[derive(Deserialize, Clone, Default)]
pub(crate) struct Ancestor {
    pub chara: u32,
    /// (skill group, stars 1..=3) of its white skill sparks
    #[serde(default)]
    pub sparks: Vec<(u32, u8)>,
    /// race indices (data.json `races`) of the single-race G1s it won
    #[serde(default)]
    pub wins: Vec<usize>,
    /// (aptitude factor group, stars) of its pink sparks: 11 turf, 12 dirt, 21-24 front runner .. end closer, 31-34
    /// sprint .. long; reported (inheritance chance per aptitude), not part of the odds
    #[serde(default)]
    pub pinks: Vec<(u32, u8)>,
}

impl Ancestor {
    /// A legacy as her child's record of a parent: her character, white sparks, G1 wins and pink sparks.
    fn of_legacy(legacy: &Legacy) -> Self {
        Self {
            chara: legacy.chara,
            sparks: legacy.sparks.clone(),
            wins: legacy.wins.clone(),
            pinks: legacy.pinks.clone(),
        }
    }
}

#[derive(Deserialize, Clone, Default)]
pub(crate) struct Legacy {
    pub chara: u32,
    #[serde(default)]
    pub borrowed: bool,
    #[serde(default)]
    pub sparks: Vec<(u32, u8)>,
    #[serde(default)]
    pub wins: Vec<usize>,
    #[serde(default)]
    pub pinks: Vec<(u32, u8)>,
    /// its own parents (the new trainee's grandparents); empty when unknown
    #[serde(default)]
    pub parents: Vec<Ancestor>,
    /// internal (a veteran before her result is known, `Env::veteran`): win chance per race index in place of `wins`,
    /// and the star mix her sparks are drawn from in place of fixed stars
    #[serde(skip)]
    win_p: Vec<f64>,
    #[serde(skip)]
    mix: Option<[f64; 3]>,
}

const fn d_top() -> usize {
    5
}

/// The work budget of one career search (units as in the per-uma search: 2 per deck node tested, 200 per exact
/// scoring, 10,000 per deck search). Measured need for a proven result (2026-10-07, synthetic pools of 12-400
/// veterans, default 8 targets): 10M-85M, the most at 200 veterans (12 s native); the loop planner's 30M
/// (EXACT_BUDGET) left pools of 12, 100 and 200 veterans unproven with the same picks. 300M leaves 3.5x headroom.
/// It does not stop the search: past it each deck search (`best_deck_run`) stops scoring its band, so every later
/// candidate gets the exact values of its product deck and start decks only and the search is not `complete`; `solve` itself goes on to its
/// stop rule (only a `hard` cap, `plan`'s extra searches, ends it early). `EXACT_BUDGET` in the environment overrides it.
const CAREER_BUDGET: usize = 300_000_000;

#[derive(Deserialize)]
struct CareerJob {
    legacies: Vec<Legacy>,
    /// trainees to consider (chara ids); empty = every owned uma
    #[serde(default)]
    trainees: Vec<u32>,
    #[serde(default = "d_top")]
    top: usize,
    /// legacies (indices) that must be among the two parents
    #[serde(default)]
    fix: Vec<usize>,
    /// one result per trainee (false: the best careers whoever trains, e.g. with the trainee fixed: the best parents)
    #[serde(default = "d_true")]
    per_trainee: bool,
    /// the builder-trace simulator (`sim`): runs instead of the search when present
    #[serde(default)]
    simulate: Option<Value>,
    /// the page's next career without simulation (`sim::plan::index_rule`): the career saving the most careers to a fully looped veteran
    #[serde(default)]
    next: Option<Value>,
    /// the page's veteran-analysis comparison (echoed in the output, nothing else)
    #[serde(default)]
    compare: bool,
    /// the plan-only jobs' flag: the page reads only `next` from them, so the main search's career reports (and the first
    /// one's schedule search) are left out
    #[serde(default)]
    no_reports: bool,
    /// borrowed parents: "allow" (at most one per career, now and in the plan's look-ahead), "later" (none in the career
    /// picked: the search's results and the plan's rows; the plan's look-ahead still borrows), "never" (none anywhere)
    #[serde(default)]
    borrowed_parent: BorrowedParent,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
enum BorrowedParent {
    #[default]
    Allow,
    Later,
    Never,
}

struct Env<'data> {
    d: &'data Data,
    sj: &'data SkillJob,
    model: Model<'data>,
    ew0: Vec<Vec<Vec<f64>>>,
    uncv: Vec<usize>,
    fixed_t: Vec<usize>,
    ntar: usize,
    nu: usize,
    unc: Vec<u32>,
    space: DeckSpace<'data>,
    dmax: Vec<Option<(Vec<f64>, Vec<f64>)>>,
    sts0: Vec<Vec<Vec<u8>>>,
    owned: Vec<usize>,
    budget: usize,
    work: usize,
    /// every earlier search's fully checked decks per deck class (`cls`; one model, so each is a proven best at its
    /// chances for every trainee of the class): later searches bound with them (`rise`), but don't start from them, so
    /// ties resolve as in a lone search
    seen: Vec<Vec<Ref>>,
    /// each trainee's deck class, as the index of its first trainee: trainees with the same outfit coverage and own-skill
    /// chances of the targets, the same relevant cards barred and as many plain pals face the same deck problem (her
    /// best deck and its value are one function of the inheritance chances; `rise` and `dmax` agree), so a proof one of
    /// them makes (`Ref`, `ruled`) holds for all of them. Within a class only the slots' affinity (so q) differs.
    cls: Vec<usize>,
    /// whether `solve` bounds with `seen`: off for the search whose fully checked decks seed the simulator's cache
    /// (so the simulator's values do not depend on what earlier searches proved)
    use_seen: bool,
    /// per deck class (`cls`), what its product searches proved (`best_deck` pass 1): (chances q, S) with every legal deck's product
    /// value at q (fixed factor included, before the spark scale) at most S. Bounds her other candidates (`solve`).
    ruled: Vec<Vec<(Vec<f64>, f64)>>,
    /// what the last `best_deck` call's product search proved: Some(S) as in `ruled`, None when it proved nothing
    last_pv: Option<f64>,
    /// `solve` keeps only careers above this value (the plan's search with a new veteran fixed, against the trainee's kept
    /// best: careers below it are pruned as if the bar were already there); NEG_INFINITY for every other search
    pub(crate) floor: f64,
    /// per structure class (`cap_class`), the largest exact / product value of any deck `best_deck` scored (the class's
    /// assumed cap must exceed it; the planner reports it and `val/planval.py` fails when it comes near the cap)
    pub(crate) max_ratio: [f64; 4],
    /// exact scorings whose exact / product ratio passed their class's cap: the cap is an assumption (`EXACT_CAPS`), so
    /// a search that saw one may have pruned a deck that beats its answer; its deck search is rerun cap-free
    /// (`best_deck_run`), and the output lists the classes (`cap_exceeded`)
    pub(crate) cap_breach: usize,
    /// the cap-free searches (`best_deck_cf`): reruns after a breach, those that proved nothing, breaches left without
    /// a rerun (`CAP_RERUN=0`), their work (counted apart from `work`)
    pub(crate) cfs: CfStats,
    /// per class, the trainee and deck that set `max_ratio`
    pub(crate) max_deck: [Option<(usize, Sel)>; 4],
    /// the lowest cap any deck search was certified under (`best_deck_run`: a band over EXACT_CAP decks is retried
    /// down the cap ladder), and how many bands overflowed; a search below the top cap is not `full`
    pub(crate) cert: f64,
    pub(crate) overflows: usize,
    /// a hard work cap for `solve` (0: none): past it the search stops taking candidates (`plan`'s extra searches)
    hard: usize,
    /// `best_deck` calls without start decks, by what they read (`DeckKey`): the outcome and what the call did besides
    /// (kept between jobs of one setup, `DECK_KEEP`)
    deck_memo: crate::FxMap<DeckKey, DeckMemo>,
    /// `corners_kept`'s tables by (deck class, the deck's entries, filler), kept between jobs of one setup with `deck_memo`
    corner_memo: crate::FxMap<(usize, Vec<usize>, usize), std::rc::Rc<Vec<Vec<f64>>>>,
    /// the setup the memo is kept under, and this job's number
    memo_fp: u64,
    job: u64,
    /// borrowed veterans a pair of parents may hold in every pool this Env builds (`Pool::max_bor`): 1, 0 under
    /// `borrowed_parent` "never"
    max_bor: u8,
}

/// a new data set (the worker's `init`): nothing kept from the old one
#[cfg(target_arch = "wasm32")]
pub(crate) fn forget_kept_decks() {
    DECK_KEEP.with(|c| c.borrow_mut().clear());
}

pub(crate) fn run_career(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    // the worker's kept work, out to the page and back in after a reload (`state_out` / `state_in`)
    match v["cmd"].as_str() {
        Some("state_out") => return state_out(),
        Some("state_in") => return state_in(&v["state"]),
        _ => {}
    }
    if let Some(roll) = v.get("roll").filter(|roll| !roll.is_null()) {
        return run_carried_roll(d, sk, cr, v, roll);
    }
    let jobs = match CareerJobs::parse(v) {
        Ok(jobs) => jobs,
        Err(error) => return error,
    };
    CHAIN_EVENTS.with(|c| c.set(jobs.deck.chain_events.unwrap_or((CHAIN_N, CHAIN_P))));
    let t_all = Instant::now();
    let input = JobInputs { data: d, skills: sk, cards: cr, job: v };
    let CareerSetup { mut env, trainees, setup_fingerprint } = match career_setup(&input, &jobs) {
        Ok(setup) => setup,
        Err(error) => return error,
    };
    let cj = &jobs.career;
    let legs = &cj.legacies;
    let rho_top = env.space.rho_top();
    restore_seen(&mut env, setup_fingerprint);
    if let Some(sm) = &cj.simulate {
        return sim::simulate(&mut env, &input, legs, &trainees, sm);
    }
    let pool = env.pool(legs);
    // (tests: the job's Env and pool handed to a check instead of the search)
    #[cfg(test)]
    if let Some(h) = tests::ENV_HOOK.with(|c| c.borrow_mut().take()) {
        h(&mut env, &pool);
        return json!({});
    }
    let mut run = CareerRun { input: &input, job: cj, trainees: &trainees, pool: &pool, pick_pool: None };
    if let Some(roll) = answer_kept_roll(&mut env, &run) {
        return json!({"next": {"roll": roll}});
    }
    // `borrowed_parent` "later": the career picked (the search's results, the plan's rows) comes from the pool without
    // borrowed parents; the plan's look-ahead keeps `pool` (under "never" `pool` itself allows none)
    let own_pool = (cj.borrowed_parent == BorrowedParent::Later).then(|| Pool { max_bor: 0, ..pool.clone() });
    run.pick_pool = own_pool.as_ref();
    env.use_seen = true;
    // the main search's own work (deck-search nodes, exact scorings; deterministic, comparable at any load): 0 when the
    // search was kept from the job before
    let (sn0, sj0) = (env.space.nodes.get(), crate::deck::JCOUNT.with(std::cell::Cell::get));
    let sv = solve_kept(&mut env, run.search_key(&["next", "no_reports"]), &run.pick_search_request());
    let search_work =
        json!({"nodes": env.space.nodes.get() - sn0, "exact": crate::deck::JCOUNT.with(std::cell::Cell::get) - sj0});
    env.use_seen = true;
    if std::env::var("CAREER_WORK").is_ok() {
        eprintln!("career search: work {} of {}, solved {}, proven {}", sv.work, env.budget, sv.solved, sv.proven);
    }
    if sv.cands == 0 {
        keep_seen(&env, setup_fingerprint);
        return json!({"error": if cj.borrowed_parent == BorrowedParent::Allow { "No legal career: a trainee needs two veterans of other characters (not her own, not the same as each other), at most one of them borrowed." }
        else { "No legal career: a trainee needs two of your own veterans of other characters (not her own, not the same as each other): borrowed parents are off." } });
    }
    let mut out = search_output(&mut env, &run, &sv, search_work, rho_top);
    if let Some(x) = &cj.next
        && !sv.found.is_empty()
    {
        out["next"] = plan_next(&mut env, &run, &sv, x);
    }
    keep_seen(&env, setup_fingerprint);
    note_job_certificates(&env, &mut out, rho_top);
    out["timing_s"] = json!(t_all.elapsed().as_secs_f64());
    out
}

/// A roll of a kept plan carried by another job (`roll`: {job: the plan job, stars}; the page's Reroll now sends the
/// plan job with `next.roll` on its own first, so its saving shows before the search): the job runs without it, then
/// the plan job with `next.roll` (its kept roll state, or the plan run again when the worker has none), its answer as
/// `roll`
fn run_carried_roll(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value, roll: &Value) -> Value {
    let mut search_job = v.clone();
    if let Some(object) = search_job.as_object_mut() {
        object.remove("roll");
    }
    let mut out = run_career(d, sk, cr, &search_job);
    let mut plan_job = roll["job"].clone();
    out["roll"] = match plan_job.get_mut("next").and_then(|next| next.as_object_mut()) {
        Some(next) => {
            next.insert("roll".into(), roll["stars"].clone());
            let plan_out = run_career(d, sk, cr, &plan_job);
            if plan_out["next"]["roll"].is_null() {
                json!({"error": plan_out["error"].as_str().or_else(|| plan_out["next"]["error"].as_str()).unwrap_or("the plan has no roll")})
            } else {
                plan_out["next"]["roll"].clone()
            }
        }
        None => json!({"error": "roll: the job is not a plan job (no `next`)"}),
    };
    out
}

/// The three readings of one career job's JSON.
struct CareerJobs {
    /// Targets, owned umas and the rank's star mix.
    skills: SkillJob,
    /// The deck space: cards, deck size, locks, chain events.
    deck: DeckJob,
    /// Legacies, trainees, places and the planner's settings.
    career: CareerJob,
}

impl CareerJobs {
    /// The job read three ways, in this order; the first error is the answer.
    fn parse(v: &Value) -> Result<Self, Value> {
        let skills = SkillJob::deserialize(v).map_err(|e| json!({"error": e.to_string()}))?;
        let deck = DeckJob::deserialize(v).map_err(|e| json!({"error": e.to_string()}))?;
        let career = CareerJob::deserialize(v).map_err(|e| json!({"error": e.to_string()}))?;
        Ok(Self { skills, deck, career })
    }
}

/// A job's solver environment and what it was built for.
struct CareerSetup<'data> {
    env: Env<'data>,
    /// The job's trainees (uma indices), owned umas in roster order.
    trainees: Vec<usize>,
    /// `setup_fp` of the job: what earlier jobs of this setup proved is kept under it (`SEEN`).
    setup_fingerprint: u64,
}

/// The job's Env: the model, the deck space, each owned uma's deck-odds ceiling, the kept deck memo of this setup and
/// the deck classes. Errors (no trainee, fewer than two legacies, a deck space the search refuses) come before
/// anything kept is touched.
fn career_setup<'data>(input: &JobInputs<'data>, jobs: &'data CareerJobs) -> Result<CareerSetup<'data>, Value> {
    let (d, sk, cr, v) = (input.data, input.skills, input.cards, input.job);
    let (sj, dj, cj) = (&jobs.skills, &jobs.deck, &jobs.career);
    let model = Model::new(d, sk, sj);
    let ew0 = model.ews.clone();
    let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
    let nu = unc.len();
    let ntar = sj.targets.len();
    let uncv = model.uncertain.clone();
    let fixed_t: Vec<usize> = (0..ntar).filter(|i| !uncv.contains(i)).collect();
    let n = d.umas.len();
    let card_of: HashMap<u32, &Card> = cr.cards.iter().map(|c| (c.id, c)).collect();
    let lock_blocked =
        |u: usize| dj.lock_cards.iter().any(|id| card_of.get(id).is_some_and(|c| c.blocks(d.umas[u].id)));
    let id_ok = |id: u32| (sj.owned.is_empty() || sj.owned.contains(&id)) && !sj.exclude.contains(&id);
    // every uma that can train (the plan's next career takes any of them); the job's trainees are among these
    let owned: Vec<usize> =
        (0..n).filter(|&i| id_ok(d.umas[i].id) && !model.no_outfit[i] && !lock_blocked(i)).collect();
    let trainees: Vec<usize> =
        owned.iter().copied().filter(|&i| cj.trainees.is_empty() || cj.trainees.contains(&d.umas[i].id)).collect();
    if trainees.is_empty() {
        return Err(json!({"error": "No trainee to search: pick at least one owned uma."}));
    }
    if cj.legacies.len() < 2 {
        return Err(json!({"error": "Add at least two veterans to choose parents from."}));
    }
    let pool_ids: Vec<u32> = trainees.iter().map(|&i| d.umas[i].id).collect();
    let ent = deck_entries(cr, dj, &unc, &pool_ids)?;
    let space = deck_space(cr, dj, sk, &unc, &ent);
    let dmax = deck_odds_ceilings(d, &space, &owned, nu, dj.deck_size);
    let sts0: Vec<Vec<Vec<u8>>> = model.opts.iter().map(|m| m.iter().map(|x| x.0.clone()).collect()).collect();
    let budget = std::env::var("EXACT_BUDGET").ok().and_then(|s| s.parse().ok()).unwrap_or(CAREER_BUDGET);
    // the kept `best_deck` memo of this setup (`DECK_KEEP`), and this job's number
    let setup_fingerprint = setup_fp(v);
    let memo_fp = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        setup_fingerprint.hash(&mut h);
        pool_ids.hash(&mut h);
        h.finish()
    };
    let job = JOB_NO.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    let kept = DECK_KEEP
        .with(|c| {
            let mut c = c.borrow_mut();
            c.iter().position(|e| e.0 == memo_fp).map(|i| c.remove(i).1)
        })
        .unwrap_or_default();
    let deck_memo = if kept.decks.len() < DECK_KEEP_CAP { kept.decks } else { HashMap::default() };
    let corner_memo = if kept.corners.len() < DECK_KEEP_CAP { kept.corners } else { HashMap::default() };
    let mut env = Env {
        d,
        sj,
        model,
        ew0,
        uncv,
        fixed_t,
        ntar,
        nu,
        unc,
        space,
        dmax,
        sts0,
        owned,
        budget,
        work: 0,
        seen: vec![vec![]; n],
        use_seen: true,
        ruled: vec![vec![]; n],
        last_pv: None,
        floor: f64::NEG_INFINITY,
        hard: 0,
        max_ratio: [0.0; 4],
        cap_breach: 0,
        cfs: CfStats::default(),
        max_deck: Default::default(),
        cert: f64::INFINITY,
        overflows: 0,
        deck_memo,
        corner_memo,
        memo_fp,
        job,
        cls: (0..n).collect(),
        max_bor: u8::from(cj.borrowed_parent != BorrowedParent::Never),
    };
    assign_deck_classes(&mut env);
    Ok(CareerSetup { env, trainees, setup_fingerprint })
}

/// Per uma (None when she does not train), the best possible deck odds per uncertain target, a ceiling for every
/// legal deck: (dw, dg) with dg the chance of a gold hint and (1 - dw)(1 - dg) the chance of no hint, from her
/// `deck_size` most likely hinting legal entries.
fn deck_odds_ceilings(
    d: &Data,
    space: &DeckSpace,
    owned: &[usize],
    nu: usize,
    deck_size: usize,
) -> Vec<Option<(Vec<f64>, Vec<f64>)>> {
    // per relevant entry: (miss of a gold hint, miss of any hint) per uncertain target
    let strong: Vec<(Vec<f64>, Vec<f64>)> = (0..space.rel.len())
        .map(|j| {
            let (mut mw, mut mg) = (vec![1.0f64; nu], vec![1.0f64; nu]);
            src_miss(&space.srcs[j], &vec![1.0; space.kind[j].1 as usize], &mut mw, &mut mg);
            let mh: Vec<f64> = (0..nu).map(|k| mg[k] * mw[k]).collect();
            (mg, mh)
        })
        .collect();
    (0..d.umas.len())
        .map(|u| {
            if !owned.contains(&u) {
                return None;
            }
            let uid = d.umas[u].id;
            let legal: Vec<usize> = (0..space.rel.len()).filter(|&j| !space.cards[space.rel[j]].blocks(uid)).collect();
            let lowest = |f: &dyn Fn(usize) -> f64| {
                let mut m: Vec<f64> = legal.iter().map(|&j| f(j)).collect();
                m.sort_by(|a, b| a.partial_cmp(b).unwrap());
                m.iter().take(deck_size).product::<f64>()
            };
            let (mut dw, mut dg) = (vec![0.0f64; nu], vec![0.0f64; nu]);
            for k in 0..nu {
                let (g, h) = (1.0 - lowest(&|j| strong[j].0[k]), 1.0 - lowest(&|j| strong[j].1[k]));
                // (g, h) = (P(gold hint), P(any hint)); the product formula takes dw with (1 - dw)(1 - dg) = 1 - h
                dg[k] = g;
                dw[k] = if g < 1.0 { (1.0 - (1.0 - h) / (1.0 - g)).max(0.0) } else { 1.0 };
            }
            Some((dw, dg))
        })
        .collect()
}

/// Deck classes (`Env::cls`): the key holds everything of a trainee the deck search reads (outfit coverage, own-skill
/// chances to the bit, `dmax`, the relevant cards she may not take, how many plain pals she may); each trainee's class
/// is the first trainee with her key.
fn assign_deck_classes(env: &mut Env) {
    let mut first: crate::FxMap<Vec<u64>, usize> = crate::FxMap::default();
    for u in 0..env.d.umas.len() {
        let uid = env.d.umas[u].id;
        let mut k: Vec<u64> = vec![];
        for o in &env.sts0[u] {
            k.push(o.len() as u64);
            k.extend(o.iter().map(|&x| u64::from(x)));
        }
        k.push(u64::MAX);
        for o in &env.ew0[u] {
            k.push(o.len() as u64);
            k.extend(o.iter().map(|x| x.to_bits()));
        }
        k.push(u64::MAX);
        match &env.dmax[u] {
            Some((w, g)) => {
                k.push(1);
                k.extend(w.iter().chain(g).map(|x| x.to_bits()));
            }
            None => k.push(0),
        }
        k.push(u64::MAX);
        k.extend((0..env.space.rel.len()).filter(|&j| env.space.cards[env.space.rel[j]].blocks(uid)).map(|j| j as u64));
        k.push(u64::MAX);
        k.push(env.space.legal_plain(&[uid]).len() as u64);
        env.cls[u] = *first.entry(k).or_insert(u);
    }
}

/// What earlier jobs on this setup proved (the page's worker keeps it between jobs, `SEEN`), when it fits this Env.
fn restore_seen(env: &mut Env, setup_fingerprint: u64) {
    if let Some(s) = SEEN.with(|c| c.borrow().iter().find(|e| e.0 == setup_fingerprint).map(|e| e.1.clone()))
        && env.seen_fits(&s)
    {
        env.seen = s;
    }
}

/// Keep this job's fully checked decks for the next job on its setup (the last `SEEN_KEEP` per class, four setups).
fn keep_seen(env: &Env, setup_fingerprint: u64) {
    SEEN.with(|c| {
        let mut c = c.borrow_mut();
        c.retain(|e| e.0 != setup_fingerprint);
        c.push((
            setup_fingerprint,
            env.seen.iter().map(|rs| rs[rs.len().saturating_sub(SEEN_KEEP)..].to_vec()).collect(),
        ));
        if c.len() > 4 {
            c.remove(0);
        }
    });
}

/// A roll of a plan this worker keeps (`next.roll`): answered from its roll state, no search (`sim::plan::roll_kept`);
/// None when the job carries no roll or the worker keeps no state for it (the job then runs, its plan answering it).
fn answer_kept_roll<'data>(env: &mut Env<'data>, run: &CareerRun<'_, 'data>) -> Option<Value> {
    let cj = run.job;
    let plan_job = cj
        .next
        .as_ref()
        .filter(|x| x.get("roll").is_some_and(|r| !r.is_null()))
        .and_then(|x| serde_json::from_value::<sim::plan::PlanJob>(x.clone()).ok())?;
    sim::plan::roll_kept(env, run.input, &cj.legacies, run.pool, &cj.fix, &plan_job)
}

/// What the job's main search and its plan read.
struct CareerRun<'run, 'data> {
    input: &'run JobInputs<'data>,
    job: &'run CareerJob,
    /// The job's trainees (uma indices).
    trainees: &'run [usize],
    /// The job's pool of legacies, borrowing as `borrowed_parent` allows in the plan's look-ahead.
    pool: &'run Pool,
    /// `borrowed_parent` "later": `pool` without borrowed parents, where the career picked comes from.
    pick_pool: Option<&'run Pool>,
}

impl CareerRun<'_, '_> {
    /// The main search over `pool` with the job's places, trainees and result count.
    fn search_request(&self) -> SearchRequest<'_> {
        SearchRequest {
            pool: self.pool,
            fixed_parents: &self.job.fix,
            trainees: self.trainees,
            per_trainee: self.job.per_trainee,
            top: self.job.top,
        }
    }
    /// The main search of the career picked: over `pick_pool` when there is one.
    fn pick_search_request(&self) -> SearchRequest<'_> {
        SearchRequest { pool: self.pick_pool.unwrap_or(self.pool), ..self.search_request() }
    }
    /// The key a main search is kept under (`LAST_SOLVE`): the hash of the job's text without the keys `drop`
    /// (`json_without`: no clone). The page sends a career job, then the same job with `next`: the plan job reuses
    /// that job's main search (a search is deterministic for its job, so the result is the one it would compute).
    fn search_key(&self, drop: &[&str]) -> u64 {
        setup_hash(&json_without(self.input.job, drop, &[]))
    }
}

/// The main search's answer: the reports of its careers (the race schedule only for the best one, the one the page
/// shows; each takes a schedule search) and its counters.
fn search_output(env: &mut Env, run: &CareerRun, sv: &Solved, search_work: Value, rho_top: f64) -> Value {
    let cj = run.job;
    let results: Vec<Value> = if cj.no_reports {
        vec![]
    } else {
        sv.found.iter().enumerate().map(|(i, f)| env.report(run.pool, f, i == 0)).collect()
    };
    let mut out = json!({"results": results, "candidates": sv.cands, "solved": sv.solved, "bounded": sv.bounded, "scored": sv.scored, "skipped": sv.skipped,
           "next_bound": sv.next_bound, "complete": sv.complete, "proven_top": sv.proven, "cap": rho_top.min(sv.cert), "class_caps": env.space.caps(), "work": search_work});
    // a deck band over EXACT_CAP decks (in the main search or any later one of this job) was certified at a lower cap
    // only: `cap` says which, `complete` is false
    if sv.cert < rho_top {
        out["complete"] = json!(false);
    }
    if cj.compare {
        out["compare"] = json!(true);
    }
    out
}

/// The job's `next`: the page's next career (`sim::plan::index_rule`) on the main search `sv`.
fn plan_next<'data>(env: &mut Env<'data>, run: &CareerRun<'_, 'data>, sv: &Solved, next: &Value) -> Value {
    let cj = run.job;
    let settings = match serde_json::from_value::<sim::plan::PlanJob>(next.clone()) {
        Ok(settings) => settings,
        Err(e) => return json!({"error": format!("next: {e}")}),
    };
    let borrowing;
    let (solved, pick) = match run.pick_pool {
        None => (sv, None),
        // "later": the look-ahead's main search is the one that borrows: the same job's as "allow", kept under its
        // key (so a plan after an "allow" run takes it as it is)
        Some(pick_pool) => {
            borrowing =
                solve_kept(env, run.search_key(&["next", "no_reports", "borrowed_parent"]), &run.search_request());
            (&borrowing, Some((pick_pool, sv)))
        }
    };
    sim::plan::index_rule(
        env,
        &sim::plan::PlanInputs {
            resources: run.input,
            legacies: &cj.legacies,
            pool: run.pool,
            solved,
            fixed_parents: &cj.fix,
            trainees: run.trainees,
            settings: &settings,
            pick,
        },
    )
}

/// What every search of the job certified, into its answer: band overflows (`cap`, `complete`), decks past their
/// class's assumed cap, and the cap-free searches.
fn note_job_certificates(env: &Env, out: &mut Value, rho_top: f64) {
    if env.overflows > 0 {
        out["complete"] = json!(false);
        out["cap"] = json!(rho_top.min(env.cert));
        out["band_overflows"] = json!(env.overflows);
    }
    // a scored deck past its class's assumed cap (in any search of the job): per class [class, ratio, cap]
    if env.cap_breach > 0 {
        let caps = env.space.caps();
        out["cap_exceeded"] = json!(
            (0..4)
                .filter(|&k| env.max_ratio[k] > caps[k] * (1.0 + 1e-9))
                .map(|k| json!([k, env.max_ratio[k], caps[k]]))
                .collect::<Vec<_>>()
        );
    }
    // the deck searches that saw a breach and were answered by the cap-free search (`best_deck_cf`; every search of the
    // job, the plan's included), how many of those (or of a `cap_free` job's searches) proved nothing (then `complete`
    // is false), breaches left unanswered (`CAP_RERUN=0`), and the cap-free work
    if env.space.dj.cap_free {
        out["cap_free"] = json!(true);
    }
    if env.cfs != CfStats::default() {
        out["cap_rerun"] = json!({"searches": env.cfs.reruns, "unproven": env.cfs.open, "unanswered": env.cfs.unhandled, "work": env.cfs.work});
        if env.cfs.open > 0 || env.cfs.unhandled > 0 {
            out["complete"] = json!(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::inherit::insp_miss2;
    use super::state::STATE_VERSION;
    use super::*;
    type Hook = Box<dyn for<'env> FnOnce(&mut Env<'env>, &Pool)>;
    thread_local! { pub(super) static ENV_HOOK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) }; }
    /// val/plan's p16 plan job with its events, and the Global data it runs on
    fn p16() -> (Data, SkillsRaw, CardsRaw, Value) {
        let r = concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/");
        let rd = |f: &str| std::fs::read_to_string(format!("{r}{f}")).unwrap();
        let d = crate::load_str(&rd("data.json"));
        let sk: SkillsRaw = serde_json::from_str(&rd("web_skills.json")).unwrap();
        let cr: CardsRaw = serde_json::from_str(&rd("web_cards.json")).unwrap();
        let mut v: Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap(),
        )
        .unwrap();
        let db: Value = serde_json::from_str(&rd("card_events.json")).unwrap();
        crate::deck::apply_events_db(&mut v, &sk, &db);
        (d, sk, cr, v)
    }
    /// `run_career` on val/plan's p16 pool (G1 wins added, so the race parts are not 0), with `h` given its Env and pool
    fn with_env(h: Hook) {
        let (d, sk, cr, mut v) = p16();
        v.as_object_mut().unwrap().remove("next");
        for (i, l) in v["legacies"].as_array_mut().unwrap().iter_mut().enumerate() {
            l["wins"] = json!([(i % 5) as u64, 7 + (i % 3) as u64, 20]);
            for (j, g) in l["parents"].as_array_mut().unwrap().iter_mut().enumerate() {
                g["wins"] = json!([(i + j) % 5]);
            }
        }
        ENV_HOOK.with(|c| *c.borrow_mut() = Some(h));
        run_career(&d, &sk, &cr, &v);
        assert!(ENV_HOOK.with(|c| c.borrow().is_none()), "the hook did not run");
    }
    /// `json_without` writes the text of a clone with those keys removed, byte for byte (the keys `setup_fp`, the plan's
    /// `roll_key` and the main search's key hash: stored in the page's state)
    #[test]
    fn json_without_matches_clone() {
        let p16: Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap(),
        )
        .unwrap();
        let mut rolled = p16.clone();
        rolled["roll"] = json!({"stars": [1, 2]});
        rolled["next"]["roll"] = json!([0, 3]);
        rolled["compare"] = json!(true);
        let cases = [
            p16,
            rolled,
            json!({}),
            json!({"a": 1}),
            json!({"next": {"roll": 1}}),
            json!([1, {"b": 2.5e-300}]),
            json!(-0.0),
            json!({"x": {"y": {}}, "z": []}),
        ];
        let drops: [(&[&str], &[&str]); 4] = [
            (
                &[
                    "legacies",
                    "trainees",
                    "fix",
                    "top",
                    "per_trainee",
                    "cmd",
                    "mode",
                    "compare",
                    "next",
                    "no_reports",
                    "borrowed_parent",
                ],
                &[],
            ),
            (&["next", "no_reports"], &[]),
            (&["no_reports", "compare", "roll"], &["roll"]),
            (&["a", "x"], &["roll"]),
        ];
        for v in &cases {
            for (d, nd) in drops {
                let mut k = v.clone();
                if let Some(o) = k.as_object_mut() {
                    for x in d {
                        o.remove(*x);
                    }
                    if let Some(n) = o.get_mut("next").and_then(|n| n.as_object_mut()) {
                        for x in nd {
                            n.remove(*x);
                        }
                    }
                }
                assert_eq!(json_without(v, d, nd), k.to_string());
            }
        }
    }
    /// The copies of the inheritance chance agree to the bit with `lineage` (slots, `inherit`, `scale`): `lineage_qc`,
    /// `gen_cands`' ceilings (against `ceiling` at `lineage`'s chances, as `solve` takes them) and `her_slot` (against
    /// `inherit` on the slots with hers emptied), on a pool with random-star slots (parents and grandparents) beside
    /// fixed ones
    #[test]
    fn inheritance_copies_match_lineage() {
        with_env(Box::new(|env: &mut Env, pool: &Pool| {
            let mut p = pool.clone();
            for x in 0..p.me.len() {
                if x % 3 == 0 {
                    p.me[x].mix = Some([0.5, 0.4, 0.1]);
                }
                if x % 4 == 1
                    && let Some(g) = p.gp[x].first_mut()
                {
                    g.stars = std::rc::Rc::new(p.me[x].stars.iter().map(|&s| s.max(1)).collect());
                    g.mix = Some([0.2, 0.3, 0.5]);
                }
            }
            let ks: Vec<usize> = (0..env.ntar).collect();
            let trainees: Vec<usize> = env.owned.clone();
            let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<u64>>();
            let (mut n, mut tg, mut tg_t) = (0usize, vec![], usize::MAX);
            for fix in [vec![], vec![2usize]] {
                let cands = env.gen_cands(&p, &fix, &trainees, |_, _| true, |ub, _, t, a, b| (ub, t, a, b));
                assert!(!cands.is_empty());
                for (ub, t, a, b) in cands {
                    let lineage = Lineage { trainee: t, first_parent: a, second_parent: b };
                    let inherit::LineageChances { chances: q, spark_scale: c, .. } = env.lineage(&p, lineage);
                    assert_eq!(
                        ub.to_bits(),
                        env.ceiling(t, &q, c).unwrap().to_bits(),
                        "gen_cands ceiling t {t} pair {a} {b}"
                    );
                    if t != tg_t {
                        tg.clear();
                        tg_t = t;
                    }
                    let mut qq = vec![0.0f64; env.ntar];
                    let cq = env.lineage_qc(&p, lineage, &ks, &mut qq, &mut tg);
                    assert_eq!((bits(&qq), cq.to_bits()), (bits(&q), c.to_bits()), "lineage_qc t {t} pair {a} {b}");
                    for (y, iv) in [(a, b), (b, a)] {
                        let her_lineage = Lineage { trainee: t, first_parent: y, second_parent: iv };
                        let sl = env.slots(&p, her_lineage);
                        let pos = sl.iter().position(|z| z.parent && z.legacy == iv).unwrap();
                        let mut sl0 = sl.clone();
                        sl0[pos].stars = std::rc::Rc::new(vec![0; env.ntar]);
                        sl0[pos].mix = None;
                        let inherit::Inheritance { chances: q0, carriers: nc0 } = inherit(&sl0, env.ntar);
                        let hs = env.her_slot(&p, her_lineage).unwrap();
                        let fa = aff_factor(sl[pos].rel, sl[pos].race);
                        assert_eq!(bits(&hs.q0), bits(&q0));
                        assert_eq!(
                            bits(&hs.xms),
                            bits(&[1.0, insp_miss2(1.0, fa), insp_miss2(2.0, fa), insp_miss2(3.0, fa)])
                        );
                        assert!(hs.pw.iter().zip(&nc0).all(|(w, &m)| w[0].to_bits() == spark_scale(m).to_bits()
                            && w[1].to_bits() == spark_scale(m + 1).to_bits()));
                    }
                    n += 1;
                }
            }
            assert!(n > 1000, "{n} candidates");
        }));
    }
    /// The worker's kept work leaves (`state_out`) and comes back (`state_in`) bit for bit after a plan, and a roll after
    /// the round trip answers as before it. A malformed store (a count past `STATE_MAX_N`, an index past its list, a NaN,
    /// a chance outside [0, 1], a deck past six cards, a truncated vector, a roll state that does not fit together) is
    /// refused whole with an error, nothing installed; one whose decks or careers do not fit the setup it meets
    /// (`Env::seen_fits`, `solved_fits`) is not used there.
    #[test]
    fn state_round_trips_and_refuses_malformed() {
        let (d, sk, cr, v) = p16();
        let run = |job: &Value| run_career(&d, &sk, &cr, job);
        let state_out = || run(&json!({"cmd": "state_out"}));
        let state_in = |st: &Value| run(&json!({"cmd": "state_in", "state": st}));
        let plan = run(&v);
        assert!(plan.get("error").is_none(), "{}", plan["error"]);
        let out = state_out();
        let st = out["state"].clone();
        for k in ["bests", "seen", "solve", "rolls"] {
            assert!(st[k].as_array().is_some_and(|a| !a.is_empty()), "{k}: nothing kept");
        }
        let mut rolled = v.clone();
        rolled["next"]["roll"] = json!(vec![1u8; v["targets"].as_array().unwrap().len()]);
        let roll = || {
            let mut r = run(&rolled);
            assert!(r["next"]["roll"].get("error").is_none() && r["next"]["roll"]["y"].is_number(), "{r}");
            for k in ["ms", "prof"] {
                r["next"]["roll"].as_object_mut().unwrap().remove(k);
            }
            r
        };
        let roll0 = roll();
        let empty = json!({"v": STATE_VERSION, "bests": [], "seen": [], "solve": [], "rolls": []});
        assert_eq!(state_in(&empty)["ok"], true);
        assert_eq!(state_in(&st)["ok"], true);
        assert_eq!(state_out(), out, "the state did not round-trip");
        assert_eq!(roll(), roll0, "a roll after the round trip");

        // ---- malformed stores: refused whole, the kept work as it was ----
        let kept = state_out();
        let hex = |x: f64| Value::String(format!("{:016x}", x.to_bits()));
        let refused = |what: &str, f: &dyn Fn(&mut Value)| {
            let mut bad = st.clone();
            f(&mut bad);
            let r = state_in(&bad);
            assert!(r["error"].is_string(), "{what}: {r}");
            assert_eq!(state_out(), kept, "{what}: something was installed");
        };
        let big = json!(1u64 << 40);
        refused("seen: a class count past STATE_MAX_N", &|s| s["seen"][0][1] = big.clone());
        refused("seen: a class index past its count", &|s| {
            let n = s["seen"][0][1].clone();
            s["seen"][0][2][0][0] = n;
        });
        refused("seen: a NaN value", &|s| s["seen"][0][2][0][1][0][1] = hex(f64::NAN));
        refused("seen: a chance above 1", &|s| s["seen"][0][2][0][1][0][0][0] = hex(1.5));
        refused("seen: a deck hint chance below 0", &|s| s["seen"][0][2][0][1][0][2][3][0] = hex(-0.25));
        refused("seen: seven cards", &|s| s["seen"][0][2][0][1][0][2][0] = json!([0, 1, 2, 3, 4, 5, 6]));
        refused("seen: a deck truncated", &|s| {
            s["seen"][0][2][0][1][0][2].as_array_mut().unwrap().pop();
        });
        refused("seen: white and gold chances of different lengths", &|s| {
            s["seen"][0][2][0][1][0][2][4].as_array_mut().unwrap().pop();
        });
        refused("bests: a uma count past STATE_MAX_N", &|s| s["bests"][0]["n"] = big.clone());
        refused("solve: a list count past STATE_MAX_N", &|s| s["solve"][0][1]["refs"][0] = big.clone());
        refused("solve: a NaN bound", &|s| s["solve"][0][1]["next_bound"] = hex(f64::NAN));
        refused("rolls: 64 targets", &|s| s["rolls"][0]["sd"]["n"][2] = json!(64));
        refused("rolls: 11 uncertain targets", &|s| s["rolls"][0]["sd"]["n"][0] = json!(11));
        refused("rolls: a guaranteed count that wraps the sum", &|s| s["rolls"][0]["sd"]["n"][1] = json!(u64::MAX));
        refused("rolls: uncertain and guaranteed not the targets", &|s| {
            let nf = s["rolls"][0]["sd"]["n"][1].as_u64().unwrap();
            s["rolls"][0]["sd"]["n"][1] = json!(nf + 1);
        });
        refused("rolls: an uncertain target past the targets", &|s| {
            let ntar = s["rolls"][0]["sd"]["n"][2].clone();
            let u = s["rolls"][0]["sd"]["uncv"].as_array_mut().unwrap();
            *u.last_mut().expect("an uncertain target") = ntar;
        });
        refused("rolls: guaranteed masks that are not the targets'", &|s| s["rolls"][0]["sd"]["fmm"][0] = json!(12345));
        refused("rolls: landed-set chances truncated", &|s| {
            s["rolls"][0]["sd"]["os"].as_array_mut().unwrap().pop();
        });
        refused("rolls: a NaN star share", &|s| s["rolls"][0]["sd"]["w"][0] = hex(f64::NAN));
        refused("rolls: a candidate's step costs not one per candidate", &|s| {
            s["rolls"][0]["base"]["c1"].as_array_mut().unwrap().push(hex(1.0));
        });
        refused("rolls: her trio side past the candidates", &|s| {
            s["rolls"][0]["her"].as_array_mut().unwrap().push(json!([1_000_000, false, hex(1.0)]));
        });
        refused("rolls: a lifted candidate past the candidates", &|s| {
            let act = s["rolls"][0]["sd"]["act"].as_array_mut().unwrap();
            act.push(json!([1_000_000, null, hex(0.5), hex(0.5)]));
        });
        refused("rolls: a slot out of order", &|s| {
            let act = s["rolls"][0]["sd"]["act"].as_array_mut().unwrap();
            act.push(json!([0, 1_000_000, hex(0.5), hex(0.5)]));
        });
        refused("rolls: a trio past the candidates", &|s| {
            s["rolls"][0]["base"]["combos"].as_array_mut().unwrap().push(json!([1_000_000, 0, hex(1.0), false, false]));
        });
        refused("rolls: an anchor's chances truncated", &|s| {
            let anc = s["rolls"][0]["anc"].as_array_mut().unwrap();
            let a = anc.iter_mut().find(|a| !a.is_null()).expect("an anchor");
            a[1].as_array_mut().unwrap().pop();
        });

        // ---- stores that parse but do not fit the setup they meet: not used there ----
        let seen_used = |st: &Value| {
            assert_eq!(state_in(st)["ok"], true);
            let used = std::rc::Rc::new(std::cell::Cell::new(false));
            let u = std::rc::Rc::clone(&used);
            ENV_HOOK.with(|c| {
                *c.borrow_mut() =
                    Some(Box::new(move |env: &mut Env, _: &Pool| u.set(env.seen.iter().any(|r| !r.is_empty()))));
            });
            let mut job = v.clone();
            job.as_object_mut().unwrap().remove("next");
            run(&job);
            used.get()
        };
        assert!(seen_used(&st), "the stored proofs were not used");
        let mut bad = st.clone();
        bad["seen"][0][2][0][1][0][2][0][0] = json!(1_000_000);
        assert!(!seen_used(&bad), "a deck entry past the deck space was used");
        let mut bad = st.clone();
        bad["seen"][0][2][0][1][0][0].as_array_mut().unwrap().pop();
        assert!(!seen_used(&bad), "chances for fewer targets were used");
        // a kept main search whose career names a trainee past the umas: searched again, the same plan
        let mut bad = st.clone();
        bad["solve"].as_array_mut().unwrap().iter_mut().for_each(|e| {
            if let Some(f) = e[1]["found"].as_array_mut().and_then(|f| f.first_mut()) {
                f[0] = json!(1_000_000);
            }
        });
        assert_eq!(state_in(&bad)["ok"], true);
        let again = run(&v);
        assert!(again.get("error").is_none(), "{}", again["error"]);
        assert_eq!((&again["next"]["choice"], &again["results"]), (&plan["next"]["choice"], &plan["results"]));
    }
}
