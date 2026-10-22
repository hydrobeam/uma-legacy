//! The page's next career (`next` on a career job, `index_rule`; docs/SOLVER.md "Next career"): the career that brings
//! a fully looped parent closest per run, scored exactly over every result she can roll, with each candidate's
//! continuation (`Cont`) fitted to the pool. It values careers through the simulator's deck cache (`Sim`, `Cache`).
use crate::deck::career::sim::{
    Cache, Data, Deserialize, Env, Found, Legacy, Pool, Ref, Sim, Solved, Value, now_ms, progress,
};
use crate::deck::career::{Career, JobInputs, Lineage};
use anchors::{PartnerAnchors, found_anchors, outcome_hi, pick_her};
use candidates::{exact_no_borrow_career, score_and_admit_proxies, set_plan_wins, setup_plan_candidates};
use combine::{PlanCombination, PlanFit, PlanScores, plan_combinations, plan_continuations, plan_sides};
use cont::{CEIL_SHARE, CostBoundCell, LB_CELLS, MemoReport, TrioSideVeteran, count_refit, trio_v};
pub(crate) use cont::{Cont, ScoreBase, Trio, Want};
use fold::{FOLD_SCRATCH, FoldCx, FoldScratch, fold_rec, leaf_products, leaf_set, rise_leaves};
use kept::{
    BESTS, BestCareer, CacheStamp, CachedSearch, KeptPlans, best_of, bests_key, h64, same_career, stamp_ok, stamped,
    take_kept_plan,
};
pub(crate) use kept::{Bests, bests_set, bests_take};
pub(crate) use kept::{bests_in, bests_out};
use output::assemble_plan_output;
pub(crate) use roll::Scored;
use roll::{KeptBase, keep_roll, roll_key};
pub(crate) use roll::{RollKeep, roll_kept, rolls_in};
pub(crate) use roll::{rolls_out, rolls_set};
use rows::{
    PlanRow, RowOrigin, add_looped_rows, add_no_borrow_rows, check_plan_proxies, near_floor, pick_of,
    score_and_promote_rows,
};
pub(crate) use score::{CareerScore, LiftStats, score_career, score_career_of};
use score::{
    HerEnds, KeptScoring, fm_masks, her_ends, her_pool, her_pool_done, her_trios, sc_roll, v0_after, with_her,
};
use search::{
    FullLoop, HINT_FLOOR, IDX_CANDS, IDX_PARTNERS, OPT_SHOW, PROXY_EFF, PROXY_SAVE, Setup, best_sur, capped, certified,
    hint_umas, is_full, legal_pair, loop_ceiling, p_margin, p_swork, rotation, sim_for, val,
};
pub(crate) use step::{All, Leaves, OneLeaf};
use step::{Est, EstBufs, Lo, One, one_of, v_after};
mod anchors;
mod candidates;
mod combine;
mod cont;
mod fold;
mod kept;
mod output;
mod roll;
mod rows;
mod score;
mod search;
mod step;

/// Position in the planner's fixed candidate order, distinct from Data::umas.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CandidateIndex(pub(crate) usize);

impl std::ops::Index<CandidateIndex> for PlanCandidates {
    type Output = crate::UmaIndex;

    fn index(&self, index: CandidateIndex) -> &Self::Output {
        &self.trainees[index.0]
    }
}

/// A set of targets; bit positions follow the table's documented target order.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(crate) struct TargetMask(usize);

impl TargetMask {
    const fn from_bits(bits: usize) -> Self {
        Self(bits)
    }

    const fn bits(self) -> usize {
        self.0
    }

    const fn contains(self, target: usize) -> bool {
        self.0 & (1usize << target) != 0
    }

    const fn insert(&mut self, target: usize) {
        self.0 |= 1usize << target;
    }

    fn iter(self) -> impl Iterator<Item = usize> {
        let mut remaining = self.0;
        std::iter::from_fn(move || {
            if remaining == 0 {
                return None;
            }
            let target = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;
            Some(target)
        })
    }
}

/// Packed outcome cell: base-four uncertain stars followed by fixed-target landing bits.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct OutcomeIndex(usize);

impl OutcomeIndex {
    fn from_stars(stars: &[u8], uncovered_targets: &[usize], fixed_targets: &[usize]) -> Self {
        let leaf = uncovered_targets
            .iter()
            .enumerate()
            .fold(0usize, |leaf, (digit, &target)| leaf | (usize::from(stars[target].min(3)) << (2 * digit)));
        let mut fixed = TargetMask::default();
        for (bit, &target) in fixed_targets.iter().enumerate() {
            if stars[target] > 0 {
                fixed.insert(bit);
            }
        }
        Self((leaf << fixed_targets.len()) | fixed.bits())
    }

    const fn leaf(self, fixed_target_count: usize) -> usize {
        self.0 >> fixed_target_count
    }

    const fn fixed_mask(self, fixed_target_count: usize) -> TargetMask {
        TargetMask::from_bits(self.0 & ((1usize << fixed_target_count) - 1))
    }
}

/// Immutable continuation and pool inputs shared by career scorings.
pub(crate) struct ScoringContext<'score, 'base> {
    /// Continuations and trio costs in candidate order.
    pub(crate) score_base: &'score ScoreBase<'base>,
    /// Recorded lineages, in parent-index order.
    pub(crate) legacies: &'score [Legacy],
    /// Pool in which the career's parents are indexed.
    pub(crate) pool: &'score Pool,
}

/// Whether the search pool was built directly from the recorded legacies.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PoolProvenance {
    RecordedLegacies,
    Extended,
}

#[derive(Deserialize, Clone)]
pub(crate) struct PlanJob {
    /// the surrogate's refresh margin (`sim` module notes)
    #[serde(default = "p_margin")]
    margin: f64,
    /// work budget (career-search units) of each candidate search beyond the job's own (also each loop bootstrap
    /// search), and of each deck search a continuation makes (a refresh); 0 = the job's budget
    #[serde(default = "p_swork")]
    search_work: usize,
    /// Named rotation (chara ids, in order) added to the stage candidates; absent or empty: no named rotation.
    #[serde(default)]
    loop_members: Option<Vec<u32>>,
    /// `index_rule`: each uma's best career through the surrogate (`best_sur`) instead of the career search (1), or
    /// both, reporting where they differ (2); the surrogate's refresh margin there
    #[serde(default)]
    sur: u8,
    #[serde(default)]
    sur_margin: f64,
    /// `index_rule` measurement: each builder's career also from her deck class's winning decks alone (no career search),
    /// its lifts and score beside the exact ones (`proxy_check` in the output); changes nothing else
    #[serde(default)]
    proxy_check: bool,
    /// `index_rule`: search every allowed uma's best career instead of using builder proxies.
    #[serde(default)]
    exact_all: bool,
    /// `index_rule`: the stage candidates (chara ids, in order) and their continuations (lam, phi, pinf) taken as given,
    /// not fitted to this pool: a comparison prices a pool with one more veteran under the fit of the plan it compares
    /// with (the plan's `fit`), so the two values differ only by what she changes, not by a refit
    #[serde(default)]
    fit: Option<Vec<(u32, f64, f64, f64)>>,
    /// `index_rule` test knob: PROXY_EFF in place of 0.06 (-1 leaves pass 1's builders to their proxies, so the pass-2
    /// searches refit Cont: tests/plan_memo.rs runs that path)
    #[serde(default)]
    proxy_eff: Option<f64>,
    /// a roll of the plan's pick (her stars per job target, 0 where it did not land): answered from the plan's kept roll
    /// state (`roll_kept`), the plan run first when the worker keeps none
    #[serde(default)]
    roll: Option<Vec<u8>>,
}

/// Inputs shared by the phases. Under `borrowed_parent: "later"`, `pick_search` is the pool
/// without borrowed parents and its main search. The candidates, builders, trios and
/// scoring partners still use `pool` and `solved`; only runnable rows use the pick's pool.
pub(crate) struct PlanInputs<'plan, 'data> {
    /// Immutable data and settings for this job.
    pub(crate) resources: &'plan JobInputs<'data>,
    /// Recorded veterans in parent-index order.
    pub(crate) legacies: &'plan [Legacy],
    /// Search pool, including any borrowed veterans.
    pub(crate) pool: &'plan Pool,
    /// Main career search, including its proofs.
    pub(crate) solved: &'plan Solved,
    /// Parent indices required by the job.
    pub(crate) fixed_parents: &'plan [usize],
    /// Allowed trainees in search order.
    pub(crate) trainees: &'plan [crate::UmaIndex],
    /// Settings for the next-career planner.
    pub(crate) settings: &'plan PlanJob,
    /// Runnable pool and search when borrowed parents are deferred.
    pub(crate) pick_search: Option<PickSearch<'plan>>,
}

/// The pool the career picked comes from when it differs from the plan's, and its main search
/// (`borrowed_parent` "later": the pool without borrowed parents).
#[derive(Clone, Copy)]
pub(crate) struct PickSearch<'plan> {
    /// The pool without borrowed parents.
    pub(crate) pool: &'plan Pool,
    /// The main career search on that pool.
    pub(crate) solved: &'plan Solved,
}

impl<'plan> PlanInputs<'plan, '_> {
    /// The pool runnable rows use: the pick's, else `pool`.
    fn pick_pool(&self) -> &'plan Pool {
        self.pick_search.map_or(self.pool, |pick| pick.pool)
    }

    /// The main search runnable rows fall back to: the pick's, else `solved`.
    fn pick_solved(&self) -> &'plan Solved {
        self.pick_search.map_or(self.solved, |pick| pick.solved)
    }

    /// A career that may run: its parents are legal on the pick's pool (the borrowing rule). Rows repeating another
    /// row's career are dropped later, at output.
    fn is_runnable(&self, f: &Found) -> bool {
        self.pick_pool().bor_ok(f.first_parent, f.second_parent)
    }
}

/// The veterans a career's parents come from, and the ones every career must include.
#[derive(Clone, Copy)]
struct ParentPool<'pool> {
    pool: &'pool Pool,
    /// Parent indices every career must include (`PlanInputs::fixed_parents`).
    fixed_parents: &'pool [usize],
}

/// Candidate order and the starting odds stay fixed while later searches add decks. Per-candidate vectors follow
/// `trainees`' order.
struct PlanCandidates {
    /// The main search's best career (borrowing allowed).
    main_career: Found,
    /// The best career of the search runnable rows fall back to (`PlanInputs::pick_solved`).
    pick_career: Found,
    /// The named loop's members (`PlanJob::loop_members`), as uma indices, with their {id, name} and why the rotation
    /// was refused.
    loop_members: Vec<crate::UmaIndex>,
    loop_member_info: Vec<Value>,
    loop_member_note: Option<String>,
    /// The candidates `hint_umas` added, as the page shows them.
    hint: Vec<Value>,
    /// The stage candidates, as uma indices (`Data::umas`).
    trainees: Vec<crate::UmaIndex>,
    /// The umas a trio's finisher may be: the allowed trainees with a legal deck.
    allowed_trainees: Vec<crate::UmaIndex>,
    /// Initial candidate careers: later builder searches do not replace these.
    careers: Vec<Option<Found>>,
    /// Each candidate's odds now: her initial career's P(all), 0 without one.
    current_odds: Vec<f64>,
    /// Whether an 8/8 of hers exists (a free side of the trios).
    has_full_veteran: Vec<bool>,
    /// Zeros: the stages-2-3 costs (`ScoreBase::later_stage_costs`), which the trios replace.
    later_stage_costs: Vec<f64>,
    /// Ones: pass 1's placeholder step costs, so that every candidate's lifts are gathered.
    placeholder_step_costs: Vec<f64>,
    /// Builders' careers come from their deck class's winning decks first (`sur` 0, neither `exact_all` nor
    /// `proxy_check`).
    proxies_enabled: bool,
}

impl PlanCandidates {
    /// Placeholder step costs gather every candidate's lifts, before the continuation is fitted.
    fn stats_base(&self) -> ScoreBase<'_> {
        ScoreBase {
            candidates: &self.trainees,
            step_costs: &self.placeholder_step_costs,
            later_stage_costs: &self.later_stage_costs,
            current_value: 0.0,
            current_odds: &self.current_odds,
            continuations: &[],
            trios: &[],
            trio_sides: &[],
            finisher_umas: &[],
        }
    }
}

/// What every phase after setup reads and none changes, built once in `index_rule`: the job's inputs (data, legacies,
/// pools, fixed parents, settings) and the fixed candidates. The score base is not here: a refit replaces it, so a
/// scoring takes the current one (`scoring`).
#[derive(Clone, Copy)]
struct PlanContext<'context, 'data> {
    input: &'context PlanInputs<'context, 'data>,
    candidates: &'context PlanCandidates,
}

impl<'context, 'data> PlanContext<'context, 'data> {
    const fn data(&self) -> &'data Data {
        self.input.resources.data
    }

    const fn settings(&self) -> &'context PlanJob {
        self.input.settings
    }

    const fn legacy_count(&self) -> usize {
        self.input.legacies.len()
    }

    /// The search pool with the job's fixed parents.
    const fn parents(&self) -> ParentPool<'context> {
        ParentPool { pool: self.input.pool, fixed_parents: self.input.fixed_parents }
    }

    /// The pool runnable rows use (`PlanInputs::pick_pool`) with the job's fixed parents.
    fn pick_parents(&self) -> ParentPool<'context> {
        ParentPool { pool: self.input.pick_pool(), fixed_parents: self.input.fixed_parents }
    }

    /// A career scoring against `score_base` on the search pool.
    const fn scoring<'score>(&'score self, score_base: &'score ScoreBase<'score>) -> ScoringContext<'score, 'score> {
        ScoringContext { score_base, legacies: self.input.legacies, pool: self.input.pool }
    }
}

/// A found career's trainee and parents. `trainee` is the uma the row or search is for.
const fn found_lineage(trainee: crate::UmaIndex, found: &Found) -> Lineage {
    Lineage { trainee, first_parent: found.first_parent, second_parent: found.second_parent }
}

/// A found career as `score_career` takes it.
const fn found_career(trainee: crate::UmaIndex, found: &Found) -> Career<'_> {
    Career { lineage: found_lineage(trainee, found), deck: &found.deck }
}

/// A found career's parents in either order.
fn unordered_parents(found: &Found) -> (usize, usize) {
    (found.first_parent.min(found.second_parent), found.first_parent.max(found.second_parent))
}

/// Searches and their completeness, including the kept plans used to extend this pool. Per-uma vectors are indexed by
/// `Data::umas`.
struct PlanSearches {
    /// Each uma's best career, searched on demand (stage-1 candidates in setup, builders when scored exactly).
    best_careers: crate::UmaTable<CachedSearch>,
    /// The decks each uma's best-career search checked fully, kept with the plan.
    proof_refs: crate::UmaTable<crate::UmaTable<Vec<Ref>>>,
    /// Per uma, her searches were incomplete: her best, looped and no-borrow careers'.
    base_incomplete: crate::UmaTable<bool>,
    looped_incomplete: crate::UmaTable<bool>,
    no_borrow_incomplete: crate::UmaTable<bool>,
    /// Each uma's best career on two real 8/8 parents, where searched (`add_looped_rows`).
    looped_careers: crate::UmaTable<CachedSearch>,
    /// A kept plan of this very pool, and one of this pool less its last veteran (taken out of `BESTS` while this
    /// plan runs and put back after it: every reroll of a career reuses the same one).
    kept_same_pool: Option<Bests>,
    kept_previous_pool: Option<Bests>,
    /// The env's own career-search budget, put back after each capped search (`capped`).
    env_budget: usize,
}

impl PlanSearches {
    const fn kept_plans(&self, legacy_count: usize) -> KeptPlans<'_> {
        KeptPlans {
            same_pool: self.kept_same_pool.as_ref(),
            previous_pool: self.kept_previous_pool.as_ref(),
            legacy_count,
        }
    }

    /// Record `trainee`'s best-career search.
    fn record_best_career(&mut self, trainee: crate::UmaIndex, search: BestCareer) {
        self.proof_refs[trainee] = search.proof_refs;
        self.best_careers[trainee] = CachedSearch::Known(search.found);
        self.base_incomplete[trainee] = !search.complete;
    }
}

/// `trainee`'s best career searched now, its deck added to the cache the planner's tables read, and recorded
fn search_best_career(sim: &mut Sim, context: &PlanContext, searches: &mut PlanSearches, trainee: crate::UmaIndex) {
    let search = best_of(sim.env, context.parents(), trainee, searches.kept_plans(context.legacy_count()));
    if let Some(found) = &search.found {
        sim.cache.add(sim.env, trainee, &found.deck);
    }
    searches.record_best_career(trainee, search);
}

/// The plan's timers and work counters, and the surrogate's differences (`sur` 2): recorded by the phases, reported
/// with the output.
struct PlanProfile {
    /// The plan's start, and the index's (pass 1's).
    started_ms: f64,
    index_started_ms: f64,
    /// Choosing the candidates, and the career searches.
    loop_ms: f64,
    search_ms: f64,
    /// The work counters at the start (deck-search nodes, exact scorings): the plan reports its own.
    nodes_at_start: usize,
    exact_scorings_at_start: usize,
    /// Proxies certified by their class's proofs, without a search.
    certified_proxies: usize,
    /// Rows scored that can be the pick.
    scored_runnable_rows: usize,
    /// Where the surrogate's best career differs from the search's (`sur` 2).
    surrogate_differences: Vec<Value>,
}

/// The plan's rows, in their order (`RowOrigin`), and each trainee's G1 wins, read when fitting a continuation.
struct PlanRows {
    rows: Vec<PlanRow>,
    /// indexed by uma (her index in `d.umas`), filled for each row's trainee: the G1 races her latest row's career
    /// wins with a chance above 0.5 (`set_plan_wins`), the continuation fit's shared-win input; empty for other umas
    wins_by_trainee: crate::UmaTable<Vec<usize>>,
}

/// The page's next career, exact where it can be and without simulation: the career that brings a FULLY LOOPED veteran
/// (an 8/8 whose parents are 8/8s) the closest. V = the cheapest trio: A and B two stage candidates, each step "build
/// while it pays, then run her" (`Cont`, fitted to the pool now; free when an 8/8 of hers exists), C the best allowed
/// uma with A's and B's 8/8s as parents (1 / her full-loop odds). A career is scored E[V - V with her veteran] - 1, exact
/// over her outcomes: her veteran as a parent raising any candidate's odds lowers that step, her own 8/8 ends her own
/// step; a roll that changes nothing leaves every score as it was (repeat). Pick: the highest;
/// within NEAR, the better own odds. The goal is always the next fully looped parent, any uma: one you have already is a
/// real 8/8 (free) with 8/8 parents, a strong side of the trios.
pub(crate) fn index_rule<'data>(env: &mut Env<'data>, input: &PlanInputs<'_, 'data>) -> Value {
    let started_ms = now_ms();
    let pick_career = input.pick_solved().found[0].clone();
    progress(1, 0, 0);
    // First in, dropped last, after every memo belonging to this plan.
    let _memo_report = MemoReport;
    // (the phases' scratch for deck values)
    let mut deck_values = vec![];
    let PlanSetup { mut sim, candidates, mut searches, mut profile, cache_before_builders } =
        match setup_plan_candidates(env, input, started_ms, pick_career, &mut deck_values) {
            Ok(prepared) => prepared,
            Err(error) => return error,
        };
    let context = PlanContext { input, candidates: &candidates };
    let mut rows = score_and_admit_proxies(&mut sim, &context, &mut searches, &mut profile, &mut deck_values);
    let looped = add_looped_rows(&mut sim, &context, &rows.rows, &mut searches, &mut profile, &mut deck_values);
    rows.rows.extend(looped);
    let no_borrow = add_no_borrow_rows(&mut sim, &context, &rows.rows, &mut searches, &mut profile, &mut deck_values);
    rows.rows.extend(no_borrow);
    let scores = score_and_promote_rows(&mut sim, &context, &mut searches, &mut rows, &mut profile, &mut deck_values);
    let rows = rows.rows;
    // (`proxy_check` alone keeps the cache as it was before the builders' searches)
    let proxy_check = match cache_before_builders {
        Some(cache) if !rows.is_empty() => {
            check_plan_proxies(&mut sim, &context, &searches, &rows, &scores, cache, &mut deck_values)
        }
        _ => Value::Null,
    };
    let mut output = assemble_plan_output(sim, &context, searches, rows, &scores, &profile, &mut deck_values);
    if !proxy_check.is_null() {
        output["proxy_check"] = proxy_check;
    }
    output
}

/// What setup hands the phases: the simulator, the fixed candidates, the search history, the profile, and (with
/// `proxy_check`) the cache as it was before any builder's search, for the proxy check only
struct PlanSetup<'env, 'data> {
    sim: Sim<'env, 'data>,
    candidates: PlanCandidates,
    searches: PlanSearches,
    profile: PlanProfile,
    cache_before_builders: Option<Cache>,
}
