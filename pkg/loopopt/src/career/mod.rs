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
//! The next career the page picks (`next`) is `plan::index_rule`; the simulator (`simulate`) is `sim`.
//!
//! A career job enters at `run_career` (`run_career_in` with the worker's kept state, both here with the job's records
//! and `Env`): `setup::career_setup` builds the job's `Env`, `search::solve_kept` runs the main search, and `run` turns
//! it into the answer.
//! - `setup`: the job read three ways (`CareerJobs`) and its `Env`: deck space, deck-odds ceilings, deck classes.
//! - `run`: the main search's answer (`search_output`), the plan (`plan_next`), rolls and the job's certificates.
//! - `search`: career candidates, bounds and the best careers.
//! - `decks`: the exact career deck searches and their kept memos.
//! - `inherit`: legacy slots, affinity and inheritance chances.
//! - `pool`: veteran pools and legal parent pairs.
//! - `outcomes`: career reports, outcome sets and corner tables.
//! - `schedule`: aptitude sparks and career race schedules.
//! - `cache`: the deck cache the simulator and the plan value careers through.
//! - `evaluator`: the surrogate's valuation the simulator and the plan share.
//! - `plan/`: the page's next career (`plan::index_rule`).
//! - `sim/`: the policy simulator (`sim::play::simulate`).
//! - `state`: the worker's kept searches and their saved form.
//! - `clock`: host clocks, progress and phase counters.
//! - `tests`: career-job tests (kept-state round trips, inheritance copies, `json_without`).
use crate::Instant;
use crate::career::decks::{CfStats, DeckMemo, DeckSearchKey};
use crate::career::outcomes::CornerKey;
use crate::career::pool::Pool;
use crate::career::run::{
    CareerRun, answer_kept_roll, keep_seen, note_job_certificates, plan_next, run_carried_roll, search_output,
};
use crate::career::search::{Ref, solve_kept};
use crate::career::setup::{CareerBuild, CareerJobs, career_setup};
use crate::career::state::{KEPT, KeptState, ProductProof, state_in, state_out};
use crate::data::Data;
use crate::deck::space::{DeckSpace, SelectedDeck};
use crate::deck::{CardsRaw, d_true};
use crate::skills::{Model, SkillJob, SkillsRaw};
use crate::uma::{UmaIndex, UmaTable};
use serde::Deserialize;
use serde_json::{Value, json};
mod clock;
mod decks;
mod inherit;
mod outcomes;
mod pool;
mod schedule;
mod search;
pub(crate) mod state;

mod cache;
mod evaluator;

mod plan;
mod run;
mod setup;
mod sim;

/// A trainee and her parents in the supplied legacy pool; parent order is significant.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Lineage {
    /// Trainee's index in Data::umas.
    pub(crate) trainee: UmaIndex,
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
    pub(crate) deck: &'deck SelectedDeck,
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
    #[serde(rename = "chara")]
    pub character_id: u32,
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
    /// internal (the parent of a veteran before her result is known is such a veteran): the pinks it may carry
    /// (`Legacy::pink_chances`)
    #[serde(skip)]
    pub(crate) pink_chances: Vec<crate::career::schedule::PinkChance>,
}

impl Ancestor {
    /// A legacy as her child's record of a parent: her character, white sparks, G1 wins and pink sparks (or the pinks
    /// she may carry).
    fn of_legacy(legacy: &Legacy) -> Self {
        Self {
            character_id: legacy.character_id,
            sparks: legacy.sparks.clone(),
            wins: legacy.wins.clone(),
            pinks: legacy.pinks.clone(),
            pink_chances: legacy.pink_chances.clone(),
        }
    }
}

/// A legacy's recorded parents (the trainee's grandparents under her): at most two. More would count a target's
/// carriers n past six (a gold spark's chance, 0.4 x 1.1^n, passes 1 at n = 10).
fn recorded_parents<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<Ancestor>, D::Error> {
    let parents = Vec::<Ancestor>::deserialize(deserializer)?;
    if parents.len() > 2 {
        return Err(serde::de::Error::custom(format!(
            "a veteran has at most 2 parents on record, not {}",
            parents.len()
        )));
    }
    Ok(parents)
}

#[derive(Deserialize, Clone, Default)]
pub(crate) struct Legacy {
    #[serde(rename = "chara")]
    pub character_id: u32,
    #[serde(default)]
    pub borrowed: bool,
    #[serde(default)]
    pub sparks: Vec<(u32, u8)>,
    #[serde(default)]
    pub wins: Vec<usize>,
    #[serde(default)]
    pub pinks: Vec<(u32, u8)>,
    /// its own parents (the new trainee's grandparents), at most two; empty when unknown
    #[serde(default, deserialize_with = "recorded_parents")]
    pub parents: Vec<Ancestor>,
    /// internal (a veteran before her result is known, `CareerSetup::veteran`): win chance per race index in place of
    /// `wins`, the star mix her sparks are drawn from in place of fixed stars, and the pinks she may carry in place of
    /// `pinks` (one at most: `CareerSetup::pink_chances`)
    #[serde(skip)]
    win_chances: Vec<f64>,
    #[serde(skip)]
    mix: Option<[f64; 3]>,
    #[serde(skip)]
    pink_chances: Vec<crate::career::schedule::PinkChance>,
}

const fn d_top() -> usize {
    5
}

/// The work budget of one career search (units as in the per-uma search: 2 per deck node tested, 200 per exact
/// scoring, 10,000 per deck search).
/// It does not stop the search: past it each deck search (`best_deck_run`) stops scoring its band, so every later
/// candidate reaching the band gets the exact values of its product deck and start decks only, and the search is not
/// `complete`; `solve` itself goes on to its
/// stop rule (only a `hard_work_cap`, `plan`'s extra searches, ends it early). `EXACT_BUDGET` in the environment
/// overrides it.
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
    #[serde(rename = "fix")]
    fixed_parents: Vec<usize>,
    /// one result per trainee (false: the best careers whoever trains, e.g. with the trainee fixed: the best parents)
    #[serde(default = "d_true")]
    per_trainee: bool,
    /// the builder-trace simulator (`sim`): runs instead of the search when present
    #[serde(default)]
    simulate: Option<Value>,
    /// the page's next career without simulation (`plan::index_rule`): the career saving the most careers to a fully looped veteran
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

/// A career job's solver environment, split by who changes what: the setup every search reads (`&`), the search's own
/// state (`&mut`), the deck searches' certificates and diagnostics, and their memo. Methods are `CareerSetup`'s and
/// take the parts they change. The career entry point publishes the memo explicitly after the job finishes.
struct Env<'data> {
    setup: CareerSetup<'data>,
    state: SearchState,
    audit: DeckSearchAudit,
    memo: DeckSearchMemo,
}

/// What every search of a job reads, built once (`career_setup`: the deck classes and ceilings included) and never
/// written after.
struct CareerSetup<'data> {
    data: &'data Data,
    /// The job's targets, owned umas and the rank's star mix.
    skill_job: &'data SkillJob,
    model: &'data Model<'data>,
    /// The uncertain targets (not guaranteed gold), as job-target indices.
    uncertain_targets: Vec<usize>,
    /// The other targets (guaranteed gold), as job-target indices.
    fixed_targets: Vec<usize>,
    /// How many targets the job has.
    target_count: usize,
    /// How many of them are uncertain.
    uncertain_count: usize,
    /// The skill group of each uncertain target.
    uncertain_groups: Vec<u32>,
    space: DeckSpace<'data>,
    /// Per uma (None when she does not train), the deck odds ceiling per uncertain target (`deck_odds_ceilings`).
    deck_odds_ceilings: UmaTable<Option<(Vec<f64>, Vec<f64>)>>,
    /// Per uma, per outfit option, its coverage of each uncertain target (2: the gold skill, 1: the white one, 0:
    /// neither; `Model::outfit_options` without the labels).
    outfit_coverage: UmaTable<Vec<Vec<u8>>>,
    owned: Vec<UmaIndex>,
    /// each trainee's deck class, as the index of its first trainee: trainees with the same outfit coverage and
    /// own-skill chances of the targets, the same relevant cards barred and as many plain pals face the same deck
    /// problem (her best deck and its value are one function of the inheritance chances; `rise` and
    /// `deck_odds_ceilings` agree), so a proof one of them makes (`Ref`, `ruled`) holds for all of them. Within a class
    /// only the slots' affinity (so q) differs.
    deck_class: UmaTable<UmaIndex>,
    /// borrowed veterans a pair of parents may hold in every pool this setup builds (`Pool::max_borrowed`): 1, 0 under
    /// `borrowed_parent` "never"
    max_borrowed: u8,
}

/// What a search changes as it runs: the folded chances it searches at, its work and budget, and what it proved for
/// later searches of the job.
struct SearchState {
    /// Reusable per-trainee buffers: own-event chances with the current lineage's inheritance folded in.
    folded_events: UmaTable<Vec<Vec<f64>>>,
    budget: usize,
    work: usize,
    /// Exact scorings the job's deck searches made (`best_deck`'s `joint_b` calls, a memo hit's as if it ran):
    /// reported as a load-free measure of the search's and the plan's work.
    exact_scorings: usize,
    /// every earlier search's fully checked decks per deck class (`deck_class`; one model, so each is a proven best at
    /// its chances for every trainee of the class): later searches bound with them (`rise`), but don't start from them,
    /// so ties resolve as in a lone search
    seen: UmaTable<Vec<Ref>>,
    /// whether `solve` bounds with `seen`: off for the search whose fully checked decks seed the simulator's cache
    /// (so the simulator's values do not depend on what earlier searches proved)
    use_seen: bool,
    /// per deck class (`deck_class`), what its product searches proved (`best_deck` pass 1). Bounds her other
    /// candidates (`solve`).
    ruled: UmaTable<Vec<ProductProof>>,
    /// what the last `best_deck` call's product search proved: Some(S) as in `ruled`, None when it proved nothing
    last_product_bound: Option<f64>,
    /// `solve` keeps only careers above this value (the plan's search with a new veteran fixed, against the trainee's kept
    /// best: careers below it are pruned as if the bar were already there); NEG_INFINITY for every other search
    pub(crate) floor: f64,
    /// a hard work cap for `solve` (0: none): past it the search stops taking candidates (`plan`'s extra searches)
    hard_work_cap: usize,
}

/// What the job's deck searches certified and the cap audit's diagnostics, reported with the job's answer
/// (`note_job_certificates`).
#[derive(Clone)]
struct DeckSearchAudit {
    /// per structure class (`cap_class`), the largest exact / product value of any deck `best_deck` scored (the class's
    /// assumed cap must exceed it; the planner reports it and `val/planval.py` fails when it comes near the cap)
    pub(crate) max_ratio: [f64; 4],
    /// exact scorings whose exact / product ratio passed their class's cap: the cap is an assumption (`EXACT_CAPS`), so
    /// a search that saw one may have pruned a deck that beats its answer; its deck search is rerun cap-free
    /// (`best_deck_run`), and the output lists the classes (`cap_exceeded`)
    pub(crate) cap_breach: usize,
    /// the cap-free searches (`best_deck_cf`): reruns after a breach, those that proved nothing, breaches left without
    /// a rerun (`CAP_RERUN=0`), their work (counted apart from `work`)
    pub(crate) cap_free_stats: CfStats,
    /// per class, the trainee and deck that set `max_ratio`
    pub(crate) max_deck: [Option<(UmaIndex, SelectedDeck)>; 4],
    /// the lowest cap any deck search was certified under (`best_deck_run`: a band over EXACT_CAP decks is retried
    /// down the cap ladder), and how many bands overflowed; a search below the top cap is not `full`
    pub(crate) certified_cap: f64,
    pub(crate) overflows: usize,
}

/// The deck searches' memo, kept between jobs of one setup (`KeptState::decks`).
struct DeckSearchMemo {
    /// `best_deck` calls without start decks, by what they read (`DeckSearchKey`): the outcome and what the call did besides
    decks: crate::FxMap<DeckSearchKey, DeckMemo>,
    /// `corners_kept`'s tables by what they read
    corners: crate::FxMap<CornerKey, std::rc::Rc<Vec<Vec<f64>>>>,
    /// the setup the memo is kept under, and this job's number
    fingerprint: u64,
    job_number: u64,
    /// State-bound configurations belong to this job's deck space, never to the retained cross-job memo.
    state_bounds: crate::deck::state_bound::StateBoundMemo,
}

/// a new data set (the worker's `init`): nothing kept from the old one
#[cfg(target_arch = "wasm32")]
pub(crate) fn forget_kept_decks() {
    KEPT.with(|cell| cell.borrow_mut().decks.clear());
}

pub(crate) fn run_career(data: &Data, skills: &SkillsRaw, cards: &CardsRaw, job: &Value) -> Value {
    KEPT.with(|kept| run_career_in(data, skills, cards, job, &mut kept.borrow_mut()))
}

/// Run one job against the caller's retained work; simulator jobs reuse this borrow without entering worker TLS.
pub(crate) fn run_career_in(
    data: &Data,
    skills: &SkillsRaw,
    cards: &CardsRaw,
    job: &Value,
    kept: &mut KeptState,
) -> Value {
    // the worker's kept work, out to the page and back in after a reload (`state_out` / `state_in`)
    match job["cmd"].as_str() {
        Some("state_out") => return state_out(kept),
        Some("state_in") => return state_in(kept, &job["state"]),
        _ => {}
    }
    if let Some(roll) = job.get("roll").filter(|roll| !roll.is_null()) {
        return run_carried_roll(data, skills, cards, job, roll, kept);
    }
    let jobs = match CareerJobs::parse(job) {
        Ok(jobs) => jobs,
        Err(error) => return error,
    };
    let t_all = Instant::now();
    let input = JobInputs { data, skills, cards, job };
    let model = Model::new(data, skills, &jobs.skills);
    let CareerBuild { mut env, trainees, setup_fingerprint } = match career_setup(&input, &jobs, &model, kept) {
        Ok(setup) => setup,
        Err(error) => return error,
    };
    let out = (|| {
        let career_job = &jobs.career;
        let legacies = &career_job.legacies;
        let rho_top = env.setup.space.rho_top();
        if let Some(simulation) = &career_job.simulate {
            return crate::career::sim::play::simulate(&mut env, kept, &input, legacies, &trainees, simulation);
        }
        let pool = env.setup.pool(legacies);
        // (tests: the job's Env and pool handed to a check instead of the search)
        #[cfg(test)]
        if let Some(hook) = tests::ENV_HOOK.with(|cell| cell.borrow_mut().take()) {
            hook(&mut env, &pool);
            return json!({});
        }
        let mut run = CareerRun { input: &input, job: career_job, trainees: &trainees, pool: &pool, pick_pool: None };
        if let Some(roll) = answer_kept_roll(&mut env, kept, &run) {
            return json!({"next": {"roll": roll}});
        }
        // `borrowed_parent` "later": the career picked (the search's results, the plan's rows) comes from the pool without
        // borrowed parents; the plan's look-ahead keeps `pool` (under "never" `pool` itself allows none)
        let own_pool =
            (career_job.borrowed_parent == BorrowedParent::Later).then(|| Pool { max_borrowed: 0, ..pool.clone() });
        run.pick_pool = own_pool.as_ref();
        env.state.use_seen = true;
        // the main search's own work (deck-search nodes, exact scorings; deterministic, comparable at any load): 0 when the
        // search was kept from the job before
        let (nodes_before, exact_scorings_before) = (env.setup.space.nodes.get(), env.state.exact_scorings);
        let search = solve_kept(&mut env, kept, run.search_key(&["next", "no_reports"]), &run.pick_search_request());
        let search_work = json!({"nodes": env.setup.space.nodes.get() - nodes_before, "exact": env.state.exact_scorings - exact_scorings_before});
        env.state.use_seen = true;
        if crate::checks::get().career_work {
            eprintln!(
                "career search: work {} of {}, solved {}, proven {}",
                search.work, env.state.budget, search.solved, search.proven
            );
        }
        if search.candidates == 0 {
            keep_seen(kept, &env.state, setup_fingerprint);
            return json!({"error": if career_job.borrowed_parent == BorrowedParent::Allow { "No legal career: a trainee needs two veterans of other characters (not her own, not the same as each other), at most one of them borrowed." }
        else { "No legal career: a trainee needs two of your own veterans of other characters (not her own, not the same as each other): borrowed parents are off." } });
        }
        let mut out = search_output(&mut env, &run, &search, search_work, rho_top);
        if let Some(next_job) = &career_job.next
            && !search.found.is_empty()
        {
            out["next"] = plan_next(&mut env, kept, &run, &search, next_job);
        }
        keep_seen(kept, &env.state, setup_fingerprint);
        note_job_certificates(&env.setup, &env.audit, &mut out, rho_top);
        out["timing_s"] = json!(t_all.elapsed().as_secs_f64());
        out
    })();
    // Publish reusable tables after every completed path, including early answers and simulator jobs.
    env.memo.keep(kept);
    out
}

#[cfg(test)]
mod tests;
