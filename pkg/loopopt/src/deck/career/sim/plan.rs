//! The page's next career (`next` on a career job, `index_rule`; docs/SOLVER.md "Next career"): the career that brings
//! a fully looped parent closest per run, scored exactly over every result she can roll, with each candidate's
//! continuation (`Cont`) fitted to the pool. It values careers through the simulator's deck cache (`Sim`, `Cache`).
use crate::deck::career::sim::{
    Cache, CardsRaw, Data, Deserialize, Env, Found, Legacy, Pool, Ref, Sim, SkillsRaw, Solved, Value, now_ms, progress,
};
use anchors::{HiAnc, found_anchors, outcome_hi, pick_her};
use candidates::{
    exact_no_borrow_career, exact_plan_career, score_and_admit_proxies, set_plan_wins, setup_plan_candidates,
};
use combine::{PlanCombination, PlanFit, PlanScores, plan_combinations, plan_continuations, plan_sides};
use cont::{CEIL_SHARE, LB_CELLS, MemoReport, memo_stat, trio_v};
pub(in crate::deck::career::sim) use cont::{Cont, ScoreBase, Trio, Want};
use fold::{FOLD_SCRATCH, FoldCx, FoldScratch, fold_rec, leaf_products, leaf_set, rise_leaves};
use kept::{BESTS, CachedSearch, best_of, bests_key, h64, same_career, stamp_ok, stamped, take_kept_plan};
pub(in crate::deck::career) use kept::{Bests, bests_set, bests_take};
pub(in crate::deck::career) use kept::{bests_in, bests_out};
use output::assemble_plan_output;
pub(in crate::deck::career::sim) use roll::Scored;
use roll::{KeptBase, keep_roll, roll_key};
pub(in crate::deck::career) use roll::{RollKeep, roll_kept, rolls_in};
pub(in crate::deck::career) use roll::{rolls_out, rolls_set};
use rows::{
    PlanRow, RowOrigin, add_looped_rows, add_no_borrow_rows, check_plan_proxies, near_floor, pick_of,
    score_and_promote_rows,
};
pub(in crate::deck::career::sim) use score::{CareerScore, score_career, score_career_of};
use score::{KeptScoring, fm_masks, her_ends, her_pool, her_pool_done, her_trios, sc_roll, v0_after, with_her};
use search::{
    HINT_FLOOR, IDX_CANDS, IDX_PARTNERS, OPT_SHOW, PROXY_EFF, PROXY_SAVE, Setup, best_sur, capped, certified,
    hint_umas, is_full, legal_pair, lift_eff, loop_ceiling, p_margin, p_swork, rotation, sim_for, val,
};
pub(in crate::deck::career::sim) use step::{All, Leaves, OneLeaf};
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

#[derive(Deserialize, Clone)]
pub(in crate::deck::career) struct PlanJob {
    /// the surrogate's refresh margin (`sim` module notes)
    #[serde(default = "p_margin")]
    margin: f64,
    /// work budget (career-search units) of each candidate search beyond the job's own (also each loop bootstrap
    /// search), and of each deck search a continuation makes (a refresh); 0 = the job's budget
    #[serde(default = "p_swork")]
    search_work: usize,
    /// the rotation (chara ids, in order); absent: the loop planner's best loop for the job's settings; empty: no loop base
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
    /// `index_rule`: every uma's best career searched exactly (the rule before builders went through their deck class)
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

/// Inputs shared by the phases. Under `borrowed_parent: "later"`, `pick` is the pool
/// without borrowed parents and its main search. The candidates, builders, trios and
/// scoring partners still use `pool` and `sv`; only runnable rows use the pick's pool.
struct PlanInputs<'plan, 'data> {
    d: &'data Data,
    sk: &'data SkillsRaw,
    cr: &'data CardsRaw,
    job: &'data Value,
    legs: &'plan [Legacy],
    pool: &'plan Pool,
    sv: &'plan Solved,
    fix: &'plan [usize],
    trainees: &'plan [usize],
    pj: &'plan PlanJob,
    pick: Option<(&'plan Pool, &'plan Solved)>,
}

impl<'plan> PlanInputs<'plan, '_> {
    /// The pool runnable rows use: the pick's, else `pool`.
    fn pick_pool(&self) -> &'plan Pool {
        self.pick.map_or(self.pool, |p| p.0)
    }

    /// The main search runnable rows fall back to: the pick's, else `sv`.
    fn pick_solved(&self) -> &'plan Solved {
        self.pick.map_or(self.sv, |p| p.1)
    }

    /// A career that may run: its parents are legal on the pick's pool (the borrowing rule). Rows repeating another
    /// row's career are dropped later, at output.
    fn is_runnable(&self, f: &Found) -> bool {
        self.pick_pool().bor_ok(f.a, f.b)
    }
}

/// Candidate order and the starting odds stay fixed while later searches add decks.
struct PlanCandidates {
    /// Main searches with borrowing allowed, and with the pick's borrowing rule.
    main_career: Found,
    pick_career: Found,
    members: Vec<usize>,
    loop_member_info: Vec<Value>,
    loop_member_note: Option<String>,
    hint: Vec<Value>,
    trainees: Vec<usize>,
    allowed_trainees: Vec<usize>,
    /// Initial candidate careers: later builder searches do not replace these.
    careers: Vec<Option<Found>>,
    pnow: Vec<f64>,
    have8: Vec<bool>,
    zeros: Vec<f64>,
    ones: Vec<f64>,
    proxy_on: bool,
}

impl PlanCandidates {
    /// Placeholder step costs gather every candidate's lifts, before the continuation is fitted.
    fn stats_base(&self) -> ScoreBase<'_> {
        ScoreBase {
            cands: &self.trainees,
            c1: &self.ones,
            m23: &self.zeros,
            v0: 0.0,
            pnow: &self.pnow,
            cont: &[],
            combos: &[],
            sides: &[],
            all: &[],
        }
    }
}

/// Searches and their completeness, including the kept plans used to extend this pool.
struct PlanSearches {
    best: Vec<CachedSearch>,
    proof_refs: Vec<Vec<Vec<Ref>>>,
    /// Per-trainee incomplete searches: base, looped and no-borrow careers.
    base_incomplete: Vec<bool>,
    looped_incomplete: Vec<bool>,
    no_borrow_incomplete: Vec<bool>,
    looped_careers: Vec<CachedSearch>,
    kept_same_pool: Option<Bests>,
    kept_previous_pool: Option<Bests>,
    original_budget: usize,
}

/// The plan's timers and work counters, and the surrogate's differences (`sur` 2): recorded by the phases, reported
/// with the output.
struct PlanProfile {
    /// The plan's start, and pass 1's.
    t_all: f64,
    t3: f64,
    /// Choosing the candidates, and the career searches.
    loop_ms: f64,
    search_ms: f64,
    /// The work counters at the start (deck-search nodes, exact scorings): the plan reports its own.
    nodes0: usize,
    joints0: usize,
    /// Admitted proxies certified by their class's proofs, without a search.
    n_cert: usize,
    exact_n: usize,
    sur_diff: Vec<Value>,
}

/// Pass 1's rows (`RowOrigin::Base`), and each one's G1 wins, read when fitting a continuation.
struct BaseRows {
    runs: Vec<PlanRow>,
    /// indexed by uma (her index in `d.umas`), filled for each row's trainee: the G1 races her latest row's career
    /// wins with a chance above 0.5 (`set_plan_wins`), the continuation fit's shared-win input; empty for other umas
    wins_by_trainee: Vec<Vec<usize>>,
}

/// The page's next career, exact where it can be and without simulation: the career that brings a FULLY LOOPED veteran
/// (an 8/8 whose parents are 8/8s) the closest. V = the cheapest trio: A and B two stage candidates, each step "build
/// while it pays, then run her" (`Cont`, fitted to the pool now; free when an 8/8 of hers exists), C the best allowed
/// uma with A's and B's 8/8s as parents (1 / her full-loop odds). A career is scored E[V - V with her veteran] - 1, exact
/// over her outcomes: her veteran as a parent raising any candidate's odds lowers that step, her own 8/8 ends her own
/// step; a roll that changes nothing leaves every score as it was (repeat). Pick: the highest;
/// within NEAR, the better own odds. The goal is always the next fully looped parent, any uma: one you have already is a
/// real 8/8 (free) with 8/8 parents, a strong side of the trios.
pub(in crate::deck::career) fn index_rule<'data>(
    env: &mut Env<'data>,
    d: &'data Data,
    sk: &'data SkillsRaw,
    cr: &'data CardsRaw,
    job: &'data Value,
    legs: &[Legacy],
    pool: &Pool,
    sv: &Solved,
    fix: &[usize],
    trainees: &[usize],
    pj: &PlanJob,
    pick: Option<(&Pool, &Solved)>,
) -> Value {
    let t_all = now_ms();
    let input = PlanInputs { d, sk, cr, job, legs, pool, sv, fix, trainees, pj, pick };
    let gp = input.pick_solved().found[0].clone();
    progress(1, 0, 0);
    // First in, dropped last, after every memo belonging to this plan.
    let _memo_report = MemoReport;
    // (the phases' scratch for deck values)
    let mut buf = vec![];
    let PlanSetup { sim: mut s, candidates, mut searches, mut profile, cache0 } =
        match setup_plan_candidates(env, &input, t_all, gp, &mut buf) {
            Ok(prepared) => prepared,
            Err(error) => return error,
        };
    let BaseRows { mut runs, mut wins_by_trainee } =
        score_and_admit_proxies(&mut s, &input, &candidates, &mut searches, &mut profile, &mut buf);
    let looped = add_looped_rows(&mut s, &input, &candidates, &runs, &mut searches, &mut profile, &mut buf);
    runs.extend(looped);
    let no_borrow = add_no_borrow_rows(&mut s, &input, &candidates, &runs, &mut searches, &mut profile, &mut buf);
    runs.extend(no_borrow);
    let scores = score_and_promote_rows(
        &mut s,
        &input,
        &candidates,
        &mut searches,
        &mut runs,
        &mut wins_by_trainee,
        &mut profile,
        &mut buf,
    );
    let proxy_check = check_plan_proxies(&mut s, &input, &candidates, &searches.best, &runs, &scores, cache0, &mut buf);
    assemble_plan_output(s, &input, &candidates, searches, runs, &scores, &profile, proxy_check, &mut buf)
}

/// What setup hands the phases: the simulator, the fixed candidates, the search history, the profile, and (with
/// `proxy_check`) the cache as it was before any builder's search, for the proxy check only
struct PlanSetup<'env, 'data> {
    sim: Sim<'env, 'data>,
    candidates: PlanCandidates,
    searches: PlanSearches,
    profile: PlanProfile,
    cache0: Option<super::Cache>,
}
