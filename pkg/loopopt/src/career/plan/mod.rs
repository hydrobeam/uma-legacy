//! The page's next career (`next` on a career job, `index_rule`; docs/SOLVER.md "Next career"): the career that brings
//! a fully looped parent closest per run, scored exactly over every result she can roll, with each candidate's
//! continuation (`Cont`) fitted to the pool. It values careers through the simulator's deck cache (`Sim`, `Cache`).
//!
//! `index_rule` (here, with the plan's records and job settings) runs the phases in order: `candidates` sets them up
//! and admits proxies, `rows` adds and scores the rows, `output` assembles the answer.
//! - `candidates`: candidate setup, exact and surrogate careers, the proxies admitted.
//! - `search`: candidate searches, rotation setup and proxy limits.
//! - `rows`: plan rows (looped, no-borrow), their scoring and promotion.
//! - `score/`: a career's score over her possible veterans (`score_career`).
//! - `step`: step costs and the leaves the next career is estimated with.
//! - `cont`: the continuation (`Cont`) and its memo.
//! - `combine`: continuation fits and candidate combinations.
//! - `fold`: folding spark outcomes and their rises.
//! - `anchors`: anchor bounds and the best partners after an outcome.
//! - `audit`: the lifted-odds audit (`audit_lifts`).
//! - `kept`: kept plans and cached career searches.
//! - `caps`: step caps from kept fits (a pool holding a kept pool's veterans can still follow its plan).
//! - `roll`: the roll of the plan's pick and its kept state (`roll_kept`).
//! - `oracle`: the joint-lift oracle (`joint_oracle`), a diagnostic: the cheapest trios on a joint two-side lattice.
//! - `ascent`: coordinate ascent over (trainee, pair) by score (`pair_ascent`), a measurement.
//! - `sweep`: the pair sweep (`pair_sweep`), a measurement: the best-scoring trainees' careers on many pairs scored.
//! - `sequenced`: the sequenced oracle (`sequenced_oracle`), a diagnostic: the cheapest trios with their sides made in
//!   turn (the second priced with the first's fresh full veteran in the pool, C bred on the two).
//! - `robust`: the robustness check on the pick, kept with the plan and run on it later (`check_kept`): the shown rows
//!   under model variants.
//! - `pair_bound`: the pair search's per-trainee bound (`PAIR_SEARCH_PAIRS`): which of her pairs it scores.
//! - `output`: the assembled plan response.
use crate::career::cache::Cache;
use crate::career::clock::{now_ms, progress};
use crate::career::plan::candidates::{score_and_admit_proxies, setup_plan_candidates};
use crate::career::plan::cont::{MemoReport, ScoreBase};
use crate::career::plan::kept::{BestCareer, Bests, CachedSearch, KeptPlans, best_of};
use crate::career::plan::output::assemble_plan_output;
use crate::career::plan::rows::{
    PlanRow, add_alternative_rows, add_looped_rows, add_no_borrow_rows, check_plan_proxies, score_and_promote_rows,
};
use crate::career::plan::search::{p_margin, p_swork};
use crate::career::pool::Pool;
use crate::career::search::{CareerSearchResult, Found, Ref};
use crate::career::sim::Sim;
use crate::career::{Career, Env, JobInputs, Legacy, Lineage};
use crate::data::Data;
use serde::Deserialize;
use serde_json::Value;
mod anchors;
mod ascent;
mod audit;
mod candidates;
pub(super) mod caps;
mod combine;
pub(super) mod cont;
mod fold;
pub(super) mod kept;
mod oracle;
mod output;
mod pair_bound;
mod robust;
mod sequenced;
mod sweep;
pub(crate) use robust::{CheckKeep, ScoringPool, check_before_plan};
pub(super) mod roll;
mod rows;
pub(super) mod score;
mod search;
mod step;

/// Position in the planner's fixed candidate order, distinct from Data::umas.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CandidateIndex(pub(crate) usize);

impl std::ops::Index<CandidateIndex> for PlanCandidates {
    type Output = crate::uma::UmaIndex;

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
    #[serde(rename = "sur")]
    surrogate_mode: u8,
    #[serde(default)]
    sur_margin: f64,
    /// `index_rule` measurement: each builder's career also from her deck class's winning decks alone (no career search),
    /// its lifts and score beside the exact ones (`proxy_check` in the output); changes nothing else
    #[serde(default)]
    proxy_check: bool,
    /// `index_rule`: search every allowed uma's best career instead of using builder proxies.
    #[serde(default)]
    exact_all: bool,
    /// `index_rule`: the stage candidates (chara ids, in order) and their continuations taken as given, not fitted to
    /// this pool: a comparison prices a pool with one more veteran under the fit of the plan it compares with (the
    /// plan's `fit`), so the two values differ only by what she changes, not by a refit (`FitEntry`)
    #[serde(default)]
    fit: Option<Vec<FitEntry>>,
    /// `index_rule` knob: partners kept per candidate where a scored career's veteran is a parent (IDX_PARTNERS, at most
    /// MAX_PARTNERS)
    #[serde(default = "default_partners")]
    partners: usize,
    /// `index_rule` knob: careers per builder scored as rows (her best and her next `builder_careers - 1` through her
    /// cached decks, `RowOrigin::Alternative`); 1: her best alone
    #[serde(default = "default_builder_careers")]
    builder_careers: usize,
    /// `index_rule` knob: fresh lineages a trio side may take per candidate (`FRESH_LINEAGES` unless the job says; 1 to
    /// 64): her best career now and her next best careers by cached decks
    #[serde(default = "default_fresh_lineages")]
    fresh_lineages: usize,
    /// `index_rule` measurement: the lifted odds the pick's and the top 3 builders' scorings read (their cached decks,
    /// their best partners) beside every partner's and an exact career search with her veteran fixed, at each one's 5
    /// likeliest outcomes that land a target (`lift_audit` in the output); changes nothing else
    #[serde(default)]
    audit_lifts: bool,
    /// `index_rule` test knob: PROXY_EFF in place of 0.06 (-1 leaves pass 1's builders to their proxies, so the pass-2
    /// searches refit Cont: tests/plan_memo.rs runs that path)
    #[serde(default)]
    proxy_eff: Option<f64>,
    /// a roll of the plan's pick (her stars per job target, 0 where it did not land): answered from the plan's kept roll
    /// state (`roll_kept`), the plan run first when the worker keeps none
    #[serde(default)]
    roll: Option<Vec<u8>>,
    /// the robustness check on the pick (`robust` in the output): its next variant or every one left, answered from the
    /// plan's kept check state (`robust::check_kept`), the plan run first when the worker keeps none; absent: none (the
    /// plan keeps the state). It changes nothing else
    #[serde(default)]
    robust: Option<CheckRequest>,
    /// with `robust`, after its variants: the within-trainee pair search (`robust::CheckWork::pair_search`, `pairs` in
    /// the check's report): for the pick's trainee and each the differential scenarios pick, her every runnable pair
    /// scored; it changes no pick
    #[serde(default)]
    pair_search: bool,
    /// `index_rule` measurement: "joint", the joint-lift oracle beside V (`oracle` in the output,
    /// `oracle::joint_oracle`), or "sequenced", the sequenced oracle (`sequenced::sequenced_oracle`, whose searches can
    /// keep proofs and set class flags); changes no pick or V
    #[serde(default)]
    oracle: Option<Oracle>,
    /// `index_rule` measurement: the best-scoring trainees' careers on their best pairs by ceiling, each scored
    /// (`pair_sweep` in the output, `sweep::pair_sweep`); changes nothing else
    #[serde(default)]
    pair_sweep: bool,
    /// `index_rule` measurement: coordinate ascent over (trainee, parent pair) by score (`pair_ascent` in the output,
    /// `ascent::pair_ascent`); changes nothing else
    #[serde(default)]
    pair_ascent: bool,
    /// `index_rule` measurement: per (chara id, first parent, second parent), her best cached deck there scored with its
    /// saving split by outcome class (`pair_decompose` in the output, `ascent::pair_decompose`); changes nothing else
    #[serde(default)]
    pair_decompose: Option<Vec<(u32, usize, usize)>>,
}

/// A diagnostic V beside the plan's (`PlanJob::oracle`).
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Oracle {
    /// The cheapest trios on a joint two-side lattice of their continuations (`oracle::JointLattice`).
    Joint,
    /// The cheapest trios with their sides made in turn (`sequenced::sequenced_oracle`).
    Sequenced,
}

/// What a check job asks of the plan's kept check state (`PlanJob::robust`).
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CheckRequest {
    /// The next variant, from a kept state only (the page: one short job at a time, so a newer job waits at most one
    /// variant).
    Next,
    /// Every variant left (the plan run first where the worker keeps no state).
    All,
}

const fn default_partners() -> usize {
    crate::career::plan::cont::IDX_PARTNERS
}

const fn default_fresh_lineages() -> usize {
    crate::career::plan::combine::FRESH_LINEAGES
}

/// Careers per builder scored as rows unless the job says (`PlanJob::builder_careers`): her best only. The next ones
/// (3: her next two) changed the pick on p10, p10_pink and gm16_nu8 (V_score_second), but the pair search's final
/// recommendation agrees at 1 and 3 on 11 of the 13 val/plan pools, while 3 doubles the plan's time and its extra
/// careers enter Cont's builder evidence, lowering V by up to 7.3% (SOLVER.md, `builder_careers`)
const fn default_builder_careers() -> usize {
    1
}

impl PlanJob {
    /// The scoring knobs this job sets.
    const fn knobs(&self) -> crate::career::plan::cont::ScoringKnobs {
        crate::career::plan::cont::ScoringKnobs { partners: self.partners }
    }
}

/// A given continuation (`PlanJob::fit`, a plan's own `fit` output): `[chara, lam, phi, pinf, p0]`, p0 the odds it was
/// fitted at (`Cont::anchor_odds`); a 4-element entry (written before p0 was) is anchored at the candidate's odds now
#[derive(Deserialize, Clone, Copy)]
#[serde(untagged)]
pub(crate) enum FitEntry {
    Anchored(u32, f64, f64, f64, f64),
    Unanchored(u32, f64, f64, f64),
}

impl FitEntry {
    pub(crate) const fn character_id(&self) -> u32 {
        match *self {
            Self::Anchored(character_id, ..) | Self::Unanchored(character_id, ..) => character_id,
        }
    }
    /// (lam, phi, pinf)
    pub(crate) const fn shape(&self) -> (f64, f64, f64) {
        match *self {
            Self::Anchored(_, lam, phi, pinf, _) | Self::Unanchored(_, lam, phi, pinf) => (lam, phi, pinf),
        }
    }
    pub(crate) const fn anchor_odds(&self) -> Option<f64> {
        match *self {
            Self::Anchored(.., anchor_odds) => Some(anchor_odds),
            Self::Unanchored(..) => None,
        }
    }
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
    pub(crate) solved: &'plan CareerSearchResult,
    /// Parent indices required by the job.
    pub(crate) fixed_parents: &'plan [usize],
    /// Allowed trainees in search order.
    pub(crate) trainees: &'plan [crate::uma::UmaIndex],
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
    pub(crate) solved: &'plan CareerSearchResult,
}

impl<'plan> PlanInputs<'plan, '_> {
    /// The pool runnable rows use: the pick's, else `pool`.
    fn pick_pool(&self) -> &'plan Pool {
        self.pick_search.map_or(self.pool, |pick| pick.pool)
    }

    /// The main search runnable rows fall back to: the pick's, else `solved`.
    fn pick_solved(&self) -> &'plan CareerSearchResult {
        self.pick_search.map_or(self.solved, |pick| pick.solved)
    }

    /// A career that may run: its parents are legal on the pick's pool (the borrowing rule). Rows repeating another
    /// row's career are dropped later, at output.
    fn is_runnable(&self, career: &Found) -> bool {
        self.pick_pool().within_borrow_limit(career.first_parent, career.second_parent)
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
    loop_members: Vec<crate::uma::UmaIndex>,
    loop_member_info: Vec<Value>,
    loop_member_note: Option<String>,
    /// The candidates the full-loop ceiling ranking added (`hint_umas`), and the ranked umas it left out, as the page
    /// shows them.
    hint: Vec<Value>,
    cut: Vec<Value>,
    /// The candidates, as uma indices (`Data::umas`): the stage candidates, then the side-only umas.
    trainees: Vec<crate::uma::UmaIndex>,
    /// The first `stage_count` candidates are the stage candidates; the rest are side-only (a usable real full veteran
    /// of hers, whatever the trainee filter): her real full veterans are her only sides, with no career, odds, step,
    /// continuation or lift slots, outside IDX_CANDS.
    stage_count: usize,
    /// The umas a trio's finisher may be: the allowed trainees with a legal deck.
    allowed_trainees: Vec<crate::uma::UmaIndex>,
    /// Initial candidate careers: later builder searches do not replace these.
    careers: Vec<Option<Found>>,
    /// Each candidate's odds now: her initial career's P(all), 0 without one.
    current_odds: Vec<f64>,
    /// Whether each candidate's odds now are her proven best career's (`ScoreBase::certified`).
    certified: Vec<bool>,
    /// Whether a full veteran of hers exists (a free side of the trios).
    has_full_veteran: Vec<bool>,
    /// Ones: pass 1's placeholder step costs, so that every candidate's lifts are gathered.
    placeholder_step_costs: Vec<f64>,
    /// Builders' careers come from their deck class's winning decks first (`sur` 0, neither `exact_all` nor
    /// `proxy_check`).
    proxies_enabled: bool,
    /// The job's scoring knobs.
    knobs: crate::career::plan::cont::ScoringKnobs,
    /// Each candidate's step cap from the worker's kept fits (`caps::step_caps`; none under a given fit).
    step_caps: Vec<Option<caps::StepCap>>,
    /// Where this plan's own caps hold (`caps::CapPool`), kept with them after it.
    cap_pool: caps::CapPool,
}

impl PlanCandidates {
    /// Placeholder step costs gather every candidate's lifts, before the continuation is fitted.
    fn stats_base(&self) -> ScoreBase<'_> {
        ScoreBase {
            candidates: &self.trainees,
            step_costs: &self.placeholder_step_costs,
            current_value: 0.0,
            current_odds: &self.current_odds,
            certified: &self.certified,
            continuations: &[],
            step_caps: &[],
            trios: &[],
            trio_sides: &[],
            finisher_umas: &[],
            knobs: self.knobs,
            odds_scales: crate::career::plan::cont::OddsScales::ONE,
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
const fn found_lineage(trainee: crate::uma::UmaIndex, found: &Found) -> Lineage {
    Lineage { trainee, first_parent: found.first_parent, second_parent: found.second_parent }
}

/// A found career as `score_career` takes it.
const fn found_career(trainee: crate::uma::UmaIndex, found: &Found) -> Career<'_> {
    Career { lineage: found_lineage(trainee, found), deck: &found.deck }
}

impl Found {
    /// A found career's parents in either order.
    fn unordered_parents(&self) -> (usize, usize) {
        (self.first_parent.min(self.second_parent), self.first_parent.max(self.second_parent))
    }
}

/// Searches and their completeness, including the kept plans used to extend this pool. Per-uma vectors are indexed by
/// `Data::umas`.
struct PlanSearches {
    /// Each uma's best career, searched on demand (stage-1 candidates in setup, builders when scored exactly).
    best_careers: crate::uma::UmaTable<CachedSearch>,
    /// The decks each uma's best-career search checked fully, kept with the plan.
    proof_refs: crate::uma::UmaTable<crate::uma::UmaTable<Vec<Ref>>>,
    /// Per uma, her searches were incomplete: her best, looped and no-borrow careers'.
    base_incomplete: crate::uma::UmaTable<bool>,
    looped_incomplete: crate::uma::UmaTable<bool>,
    no_borrow_incomplete: crate::uma::UmaTable<bool>,
    /// Each uma's best career on two real full veteran parents, where searched (`add_looped_rows`).
    looped_careers: crate::uma::UmaTable<CachedSearch>,
    /// A kept plan of this very pool, and one of this pool less its last veteran (taken out of `KeptState::bests` while this
    /// plan runs and put back after it: every reroll of a career reuses the same one).
    kept_same_pool: Option<Bests>,
    kept_previous_pool: Option<Bests>,
    /// The env's own career-search budget, put back after each capped search (`capped`).
    env_budget: usize,
    /// Per (trainee, parent pair, ascending), her best career there searched exactly (`exact_pair_career`).
    pair_careers: crate::career::plan::combine::PairMemo,
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
    fn record_best_career(&mut self, trainee: crate::uma::UmaIndex, search: BestCareer) {
        self.proof_refs[trainee] = search.proof_refs;
        self.best_careers[trainee] = CachedSearch::Known(search.found);
        self.base_incomplete[trainee] = !search.complete;
    }
}

/// `trainee`'s best career searched now, its deck added to the cache the planner's tables read, and recorded
fn search_best_career(
    simulator: &mut Sim,
    context: &PlanContext,
    searches: &mut PlanSearches,
    trainee: crate::uma::UmaIndex,
) {
    let search = best_of(simulator.env, context.parents(), trainee, searches.kept_plans(context.legacy_count()));
    if let Some(found) = &search.found {
        simulator.evaluator.cache.add(
            &simulator.env.setup,
            &mut simulator.env.state,
            &mut simulator.env.memo,
            trainee,
            &found.deck,
        );
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
    /// indexed by uma (her index in `d.umas`), filled for each row's trainee: her latest row's career's G1 win chances
    /// (`set_plan_wins`), the wins of her hypothetical veterans in the continuation fit (`loop_ceiling`); empty for
    /// other umas
    wins_by_trainee: crate::uma::UmaTable<Vec<f64>>,
}

/// The page's next career, exact where it can be and without simulation: the career that brings a FULLY LOOPED veteran
/// (a full veteran whose parents are full veterans) the closest. V = the cheapest trio: A and B two stage candidates, each step "build
/// while it pays, then run her" (`Cont`, fitted to the pool now; free when a full veteran of hers exists), C the best allowed
/// uma with A's and B's full veterans as parents (1 / her full-loop odds). A career is scored E[V - V with her veteran] - 1, exact
/// over her outcomes: her veteran as a parent raising any candidate's odds lowers that step, her own full veteran ends her own
/// step; a roll that changes nothing leaves every score as it was (repeat). Pick: the highest
/// (a strict argmax; the rows within NEAR are labelled near). The goal is always the next fully looped parent, any uma: one you have already is a
/// real full veteran (free) with full veteran parents, a strong side of the trios.
pub(crate) fn index_rule<'data>(
    env: &mut Env<'data>,
    kept: &mut crate::career::state::KeptState,
    input: &PlanInputs<'_, 'data>,
) -> Value {
    let started_ms = now_ms();
    let pick_career = input.pick_solved().found[0].clone();
    progress(1, 0, 0);
    // First in, dropped last, after every memo belonging to this plan.
    let _memo_report = MemoReport;
    // (the phases' scratch for deck values)
    let mut deck_values = vec![];
    let PlanSetup { mut sim, candidates, mut searches, mut profile, cache_before_builders } =
        match setup_plan_candidates(env, kept, input, started_ms, pick_career, &mut deck_values) {
            Ok(prepared) => prepared,
            Err(error) => return error,
        };
    let context = PlanContext { input, candidates: &candidates };
    let mut rows = score_and_admit_proxies(&mut sim, &context, &mut searches, &mut profile, &mut deck_values);
    let looped = add_looped_rows(&mut sim, &context, &rows.rows, &mut searches, &mut profile, &mut deck_values);
    rows.rows.extend(looped);
    let no_borrow = add_no_borrow_rows(&mut sim, &context, &rows.rows, &mut searches, &mut profile, &mut deck_values);
    rows.rows.extend(no_borrow);
    let alternatives = add_alternative_rows(&mut sim, &context, &rows.rows, &mut deck_values);
    rows.rows.extend(alternatives);
    let scores = score_and_promote_rows(&mut sim, &context, &mut searches, &mut rows, &mut profile, &mut deck_values);
    let rows = rows.rows;
    // (`proxy_check` alone keeps the cache as it was before the builders' searches)
    let proxy_check = match cache_before_builders {
        Some(cache) if !rows.is_empty() => {
            check_plan_proxies(&mut sim, &context, &searches, &rows, &scores, cache, &mut deck_values)
        }
        _ => Value::Null,
    };
    let lift_audit = if input.settings.audit_lifts && scores.any_scored {
        audit::audit_lifts(&mut sim, &context, &rows, &scores, &mut deck_values)
    } else {
        Value::Null
    };
    let pair_sweep = sweep::pair_sweep(&mut sim, &context, &rows, &scores, &mut deck_values);
    let pair_ascent = ascent::pair_ascent(&mut sim, &context, &rows, &scores, &mut deck_values);
    let pair_decompose = ascent::pair_decompose(&mut sim, &context, &scores, &mut deck_values);
    let oracle = match input.settings.oracle {
        Some(Oracle::Sequenced) => sequenced::sequenced_oracle(&mut sim, &context, &rows, &scores, &mut deck_values),
        _ => oracle::joint_oracle(&mut sim, &context, &rows, &scores, &mut deck_values),
    };
    // (what the check reads, kept after the output: the shown rows, and the deck cache, which the output does not read)
    let check_draft = robust::check_draft(&context, &rows, &scores.fit, scores.any_scored);
    let check_cache = check_draft
        .is_some()
        .then(|| std::mem::replace(&mut sim.evaluator.cache, Cache::new(context.data().umas.len())));
    let check_ids = check_draft.as_ref().map(|draft| draft.row_ids(context.data(), &sim.env.setup.space));
    let mut output = assemble_plan_output(sim, &context, searches, rows, &scores, &profile);
    // (its steps kept as caps for later plans of a pool holding this one's veterans; none under a given fit)
    if input.settings.fit.is_none() {
        let fit = &scores.fit;
        let plan_caps =
            caps::plan_caps(&candidates.trainees, &candidates.current_odds, &fit.continuations, &candidates.step_caps);
        caps::keep_caps(kept, candidates.cap_pool.clone(), plan_caps);
    }
    if !oracle.is_null() {
        output["oracle"] = oracle;
    }
    if !pair_sweep.is_null() {
        output["pair_sweep"] = pair_sweep;
    }
    if !pair_ascent.is_null() {
        output["pair_ascent"] = pair_ascent;
    }
    if !pair_decompose.is_null() {
        output["pair_decompose"] = pair_decompose;
    }
    if let (Some(draft), Some(cache), Some(row_ids)) = (check_draft, check_cache, check_ids) {
        let key = robust::check_key(input.resources.job);
        let source = robust::CheckSource { candidates, fit: scores.fit, cache };
        robust::keep_check(kept, row_ids, key, draft, source);
        let pool = robust::ScoringPool {
            legacies: input.legacies,
            pool: input.pool,
            pick_pool: input.pick_pool(),
            fixed_parents: input.fixed_parents,
        };
        if let Some(check) = robust::check_kept(env, kept, input.resources, pool, input.settings) {
            output["robust"] = check;
        }
    }
    if !lift_audit.is_null() {
        output["lift_audit"] = lift_audit;
    }
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
