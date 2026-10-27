//! Policy simulator on a surrogate (`simulate` on a career job; docs/SOLVER.md "Policy simulator").
//! Shared simulator records and construction live here; the career valuation it owns (deck cache, `deck_for`, exact
//! numbers) is `career/evaluator.rs`; policies, draws, audits and path execution live in `sim/policies.rs`,
//! `sim/draw.rs`, `sim/audit.rs` and `sim/play.rs`.
//!
//! Under the model, the expected number of careers until the run's goal, for a decision policy on a fixed starting
//! pool. One process retains the loaded data and deck cache between careers; policies may still search for new decks
//! or careers. A career *finishes* when its success ends the run: under `goal` "first" (the default) every career,
//! the goal being the first veteran that lands every target; under `goal` "looped" a career whose parents are both
//! full veterans (a spark of every target, yours or borrowed), the goal being the page's plan's: a fully looped parent.
//! Per run (derived):
//!
//!   E = sum over careers t of S_t,   S_0 = 1,   S_{t+1} = S_t (1 - p_t) if career t finishes, else S_t
//!
//! with p_t the exact P(all) of career t. A finishing career's success is kept analytic: the path continues through
//! its failure only, the veteran drawn from its landed-set distribution with the all-landed set removed. A career that
//! does not finish (goal `looped` only) has its veteran drawn from every landed set, the all-landed one last; an
//! all-landed draw is a full veteran, and she joins the pool like any other. This is unbiased: the path is drawn from
//! the process with finishing careers forced to fail, and S_t is the chance the real process is still running at t
//! given the path's other draws. The path stops when S_t < `survival_cutoff` or at `max_careers`, where a tail is
//! added: S_t / p_t when career t finishes (career t repeated forever); under goal `looped`, when it does not, S_t / p
//! of the last finishing career the run ran (repeating that one forever stays possible: her parents stay in the pool);
//! when the run ran none (S_t = 1), the run is `censored`, its tail 0 and its E a lower bound. Under goal `looped` the
//! tail is a stated rule, not the policy's own continuation, so `max_careers` should be at least 3x the pool's V (the
//! plan's `v0`) for E to be mostly the policy's.
//!
//! The modules call each other on purpose: `run_career_in` (career/mod.rs) runs `simulate` for a job with `simulate`, and the
//! `exact` and `index` policies (`exact_cur`, `index_cur`) call `run_career_in` again on that job with `simulate` removed,
//! so the simulator tests the same path the page runs.
//!
//! The surrogate (policy `greedy`): the career with the highest P(all), where each candidate (trainee, parents) is
//! valued by its trainee's cached decks only (`Cache`: per deck and outfit the corner table, exact at any inheritance
//! chances, `CareerSetup::corners`). A candidate is first bounded by the search's ceiling and by `rise` from each fully
//! checked deck of hers (`Ref`, proven best at its chances) and by her product-search proofs (`ruled`); a candidate
//! whose bound rb clears the best so far B and exceeds her table value tv by more than the margin m gets an exact deck
//! search there (`best_deck` at bar B, the per-uma search), and the deck found joins the cache ("refresh"). Derived:
//! for the exact best career c* (value E*), either rb(c*) < B (so E* < B), or it was searched (tv = E* when the search
//! finished), or tv >= rb / (1 + m) >= E* / (1 + m); B only rises. So the career picked has an exact P(all), with the
//! cached deck it is run with, of at least E* / (1 + m) (m = 0: the exact greedy pick up to ties and searches that ran
//! out of work). A candidate is valued once, when its last parent arrives: a new veteran only creates candidates with
//! her as a parent, and each trainee's static ceiling (q = 1, every spark on six legacies) skips her outright once it
//! is below B.
//!
//! `margin: -1` searches every candidate no bound rules out (exact-backed: the career search's own proofs, nothing
//! trusted from the cache). `audit` records the surrogate against fresh exact searches (`audit_row`); `check` compares
//! it with the exact greedy pick every few veterans. The job's `fix` is ignored; its `trainees` are the trainees.
//!
//! Draws (`u01`): keyed by (seed, run, career index, stream, item), so a run index faces the same uniforms under every
//! policy (common random numbers: unbiased, and paired across policies, but not a monotone coupling: the landed set is
//! the inverse CDF along the (landed count, mask) order, not by set inclusion, and once two policies' careers differ
//! the same uniforms drive different careers). Landed set: inverse CDF over the failed sets (or every set, for a career
//! that does not finish) ordered fewest-landed first; stars per landed target from `star_dist`; G1 wins per race from
//! the schedule's chance of at least one win; her pink spark as the page's roll draws it (`draw.rs`). Pruning: a new
//! veteran is dropped when an owned veteran of the same character with the same two parents has at least her stars
//! on every target, every G1 she won, and the same pinks (pinks set her children's aptitudes). The dominator gives at
//! least the P(all) of every career the new one could parent, and is full when she is (a family fixes both parents);
//! it is not the same veteran for the path: her wins steer her children's schedules, so pruning is exact for P(all),
//! not for a whole run.
//!
//! Policies (`simulate`'s match on `policy`): `greedy` (surrogate; under goal `looped`, the best looped career, both
//! parents full veterans, once there is one, else the best career), `greedy_exact` (the career search in process per
//! decision: the best career by P(all); under goal `looped` the best among the full veterans' careers when there is
//! one), `loop` (a fixed rotation of `loop_members`: each career's parents are the
//! latest veterans of the previous two members in rotation order; bootstrap below), and `trace` (building for
//! `loop_members[0]` with the others as builders: the measurement `Cont`'s shape is fitted to), and `index` (the page's
//! plan: `index_rule` on the step's pool, run as the page's plan job in process, its `choice` the career run). A policy
//! picks the next career (`Cur`) from the state (`PathState`) in `play`; `trace` runs its own loop and returns its rows
//! (it has no goal: `goal` "looped" is refused there, as are greedy's `audit` and `check`, which measure the
//! unrestricted pick).
//!
//! `index` and the worker's kept state: a plan job reads and leaves the kept plans (`KeptState::bests`), proofs (`KeptState::seen`),
//! main searches (`KeptState::searches`), roll states (`KeptState::rolls`) and deck memos (`KeptState::decks`). Each run
//! starts with the entire record empty (`kept_swap`; the
//! state before the simulation is put back after it), so a run's plans read only what its own earlier steps left: a
//! run's careers are a function of the job and the run index, not of the runs before it or their order. Within a run
//! a step's pool is the last one plus her veteran at the end (or the same pool, when she is pruned), the case the kept
//! plans serve (`Bests`).
//!
//! Loop bootstrap: a member whose two parents (the latest rotation veterans of the previous two members) both exist
//! runs with them; otherwise her parents are the best pair for her by exact P(all) from the pool (the career search
//! with her as the only trainee) with every existing one of those rotation veterans fixed as a parent. A loop career
//! keeps her own schedule (the career model's); the loop planner coordinates schedules across members, this does not.
//! The loop adds every veteran without dominance pruning. Its rotation keeps the better of the previous veteran and
//! the new one for the next two loop careers (`rot_update`), keeping the previous veteran on a tie.
use crate::career::evaluator::{Cur, Evaluator};
/// the page's next career (`next` on a career job)
use crate::career::sim::play::PathState;
use crate::career::{Env, JobInputs, Legacy, Lineage};
use crate::data::Data;
use crate::deck::{CardsRaw, d_true};
use crate::skills::SkillsRaw;
use serde::Deserialize;
use serde_json::{Value, json};

mod audit;
mod draw;
pub(super) mod play;
mod policies;

fn d_policy() -> String {
    "greedy".into()
}
const fn d_runs() -> usize {
    16
}
const fn d_seed() -> u32 {
    1
}
const fn d_eps() -> f64 {
    1e-3
}
const fn d_maxc() -> usize {
    100_000
}
const fn d_frac() -> f64 {
    0.3
}
const fn d_asample() -> usize {
    3
}
const fn d_aevery() -> usize {
    50
}

#[derive(Deserialize, Clone)]
pub(super) struct SimJob {
    /// greedy | greedy_exact | loop | trace | index
    #[serde(default = "d_policy")]
    policy: String,
    #[serde(default = "d_runs")]
    runs: usize,
    #[serde(default)]
    first: usize,
    #[serde(default = "d_seed")]
    seed: u32,
    #[serde(default = "d_eps")]
    #[serde(rename = "eps")]
    survival_cutoff: f64,
    #[serde(default = "d_maxc")]
    max_careers: usize,
    /// refresh margin m (module notes); 0 = refresh whenever a bound exceeds the table value
    #[serde(default)]
    margin: f64,
    #[serde(default = "d_true")]
    prune: bool,
    /// keep the deck cache across runs (faster; with m > 0 a pick can depend on what earlier runs cached)
    #[serde(default = "d_true")]
    share_cache: bool,
    /// every `check` new veterans (and at the start), compare the surrogate with exact searches
    #[serde(default)]
    check: usize,
    /// lineages checked per check (the newest veteran's candidates, ceiling at least `check_frac` x B, best first)
    #[serde(default)]
    check_lineages: usize,
    #[serde(default = "d_frac")]
    check_frac: f64,
    /// loop: rotation members (chara ids) in order; trace: the trainee built for, then her builders
    #[serde(default)]
    loop_members: Vec<u32>,
    /// fidelity audit (`audit_row`): at every pick change of greedy (the exact argmax, the pick and `audit_sample`
    /// random candidates with the newest veteran), and every `audit_every` careers of the loop (its career, and its
    /// keep-or-replace decision under exact values)
    #[serde(default)]
    audit: bool,
    #[serde(default = "d_asample")]
    audit_sample: usize,
    #[serde(default = "d_aevery")]
    audit_every: usize,
    /// log progress to stderr every this many careers (0: off)
    #[serde(default)]
    progress: usize,
    /// index: the plan job's `next` (docs/SOLVER.md "Next career"); the page sends `{}`
    #[serde(default = "d_next")]
    next: Value,
    /// index: what each step runs (`IndexAction`)
    #[serde(default)]
    act_on: IndexAction,
    /// first (default) | looped: what a run ends at (module notes)
    #[serde(default)]
    goal: Goal,
    /// greedy, greedy_exact, index: every run's first career, the trainee (chara id) on the two parents (legacy
    /// indices) with her best deck, the policy taking over from the second (paired truncated rollouts,
    /// val/sim/rollouts.py)
    #[serde(default)]
    force: Option<ForcedCareer>,
    /// per run also the pool at the stop (`final_legacies`) and the survival there (`survival`)
    #[serde(default)]
    keep_pool: bool,
}
/// A run's first career, fixed by the job (`SimJob::force`).
#[derive(Deserialize, Clone, Copy)]
pub(super) struct ForcedCareer {
    pub(super) trainee: u32,
    pub(super) parents: [usize; 2],
}
fn d_next() -> Value {
    json!({})
}
/// What `index` runs each step (`SimJob::act_on`).
#[derive(Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum IndexAction {
    /// The plan's pick, `next.choice` (the table's first row).
    #[default]
    Choice,
    /// The page's final recommendation once its check is in: the plan job also asks `robust` "all" and `pair_search`,
    /// and the career `next.robust.pairs.recommendation` names runs with its own deck (`Sim::recommended_cur`)
    PairSearch,
}
/// What a run's path ends at (`SimJob::goal`; module notes).
#[derive(Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(super) enum Goal {
    /// The first veteran that lands every target.
    #[default]
    First,
    /// The first fully looped parent: a veteran that lands every target on two full-veteran parents (the page's plan's
    /// goal).
    Looped,
}
/// index: the page's `top` for its plan job (`ROLL_K` in webapp/src/career.js)
const PAGE_TOP: usize = 6;
/// Owned roster tables moved into a simulator.
pub(super) struct SimulationRoster {
    /// Trainees considered by later simulated careers, in search order.
    pub(super) trainees: Vec<crate::uma::UmaIndex>,
    /// Rotation members in the supplied order.
    pub(super) members: Vec<crate::uma::UmaIndex>,
    /// Rotation position for each Data::umas index, when tracing members.
    pub(super) member_by_trainee: crate::uma::UmaTable<Option<usize>>,
}

/// Independent simulator behaviours selected by its caller.
#[derive(Clone, Copy)]
pub(super) struct SimulationOptions {
    /// Record audit output for the main simulation.
    pub(super) record_audits: bool,
    /// Search for decks absent from the simulator cache.
    pub(super) fill_missing_decks: bool,
}

/// The simulator: the career setup it searches in (`env`, the one mutable owner), the evaluator it values careers with
/// (`career::evaluator`, owned here and borrowed beside `env`), and the path's fixed inputs.
pub(crate) struct Sim<'env, 'data> {
    pub(super) env: &'env mut Env<'data>,
    /// Retained work shared with nested career jobs; index runs replace and restore it explicitly.
    pub(super) kept: &'env mut crate::career::state::KeptState,
    pub(super) data: &'data Data,
    skills: &'data SkillsRaw,
    cards: &'data CardsRaw,
    job: &'data Value,
    pub(super) settings: SimJob,
    pub(super) evaluator: Evaluator,
    trainees: Vec<crate::uma::UmaIndex>,
    /// Per uma, her static ceiling (q = 1 on every target; 0 when she is not owned)
    static_ceilings: crate::uma::UmaTable<f64>,
    /// The failed landed sets (every set but the full one), fewest landed first, ties by mask: the draw's order
    failed_sets: Vec<usize>,
    /// The rank's 1/2/3-star shares of a landed target's spark
    star_mix: [f64; 3],
    members: Vec<crate::uma::UmaIndex>,
    /// trace: trainee → loop member index (empty under the other policies)
    member_by_trainee: crate::uma::UmaTable<Option<usize>>,
    /// the simulator's own path (false when the plan uses this machinery): audits are taken only there
    record_audits: bool,
    audits: Vec<Value>,
}
fn canon(character_id: u32, sparks: &[(u32, u8)], pinks: &[(u32, u8)], wins: &[usize]) -> String {
    let (mut sparks, mut pinks, mut wins) = (sparks.to_vec(), pinks.to_vec(), wins.to_vec());
    sparks.sort_unstable();
    pinks.sort_unstable();
    wins.sort_unstable();
    format!("{character_id}|{sparks:?}|{pinks:?}|{wins:?}")
}
/// a veteran's family: the character and her two parents as recorded
fn family(legacy: &Legacy) -> String {
    let mut parents: Vec<String> = legacy
        .parents
        .iter()
        .map(|parent| canon(parent.character_id, &parent.sparks, &parent.pinks, &parent.wins))
        .collect();
    parents.sort();
    format!("{}#{}", legacy.character_id, parents.join("#"))
}
fn leg_json(legacy: &Legacy) -> Value {
    json!({"chara": legacy.character_id, "borrowed": legacy.borrowed, "sparks": legacy.sparks, "wins": legacy.wins, "pinks": legacy.pinks,
           "parents": legacy.parents.iter().map(|parent| json!({"chara": parent.character_id, "sparks": parent.sparks, "wins": parent.wins, "pinks": parent.pinks})).collect::<Vec<_>>()})
}

impl<'env, 'data> Sim<'env, 'data> {
    /// The simulator's machinery for a job (`simulate`'s, and the plan's through `plan::search::sim_for`): the landed sets in
    /// order of size, each owned trainee's ceiling at q = 1 (`static_ceilings`), the rank's star mix, the evaluator
    /// (`Evaluator::new`)
    pub(super) fn new(
        env: &'env mut Env<'data>,
        kept: &'env mut crate::career::state::KeptState,
        input: &JobInputs<'data>,
        settings: SimJob,
        roster: SimulationRoster,
        options: SimulationOptions,
    ) -> Self {
        let Env { ref setup, ref mut state, .. } = *env;
        let target_count = setup.target_count;
        let full = (1usize << target_count) - 1;
        let mut failed_sets: Vec<usize> = (0..full).collect();
        failed_sets.sort_by_key(|&mask| (mask.count_ones(), mask));
        let one = vec![1.0f64; target_count];
        let static_ceilings: crate::uma::UmaTable<f64> = setup
            .data
            .umas
            .indices()
            .map(|trainee| {
                if setup.owned.contains(&trainee) {
                    setup.ceiling(state, trainee, &one, 1.0).unwrap_or(0.0)
                } else {
                    0.0
                }
            })
            .collect();
        let star_mix = setup.skill_job.star_dist;
        let evaluator = Evaluator::new(setup, settings.margin, options.fill_missing_decks);
        Sim {
            env,
            kept,
            data: input.data,
            skills: input.skills,
            cards: input.cards,
            job: input.job,
            settings,
            evaluator,
            trainees: roster.trainees,
            static_ceilings,
            failed_sets,
            star_mix,
            members: roster.members,
            member_by_trainee: roster.member_by_trainee,
            record_audits: options.record_audits,
            audits: vec![],
        }
    }
    fn key(&self, career: &Cur) -> String {
        let Lineage { trainee, first_parent, second_parent } = career.lineage;
        format!("{}:{},{}", self.data.umas[trainee].id, first_parent, second_parent)
    }
    /// the value a career of `trainee` must beat to matter: the best overall, or under `trace` a member's own best
    fn bar(&self, path: &PathState, trainee: crate::uma::UmaIndex) -> f64 {
        match self.member_by_trainee.get(trainee).copied().flatten() {
            Some(member) => path.member_best[member].map_or(0.0, |pick| pick.value),
            None => path.best.map_or(0.0, |pick| pick.value),
        }
    }
    /// the value `lineage` must beat to matter: `bar`, or under goal `looped` for a looped career (both parents full
    /// veterans) the best looped career's, which greedy runs once there is one
    fn lineage_bar(&self, path: &PathState, lineage: Lineage) -> f64 {
        let bar = self.bar(path, lineage.trainee);
        if self.settings.goal == Goal::Looped && path.is_looped(lineage) { bar.min(path.looped_bar()) } else { bar }
    }
    /// value a candidate lineage against its bar (`lineage_bar`, `Evaluator::consider`), kept when it beats the best
    /// so far (or, under `trace`, her member's best; under goal `looped`, a looped career the best looped one)
    fn consider(&mut self, path: &mut PathState, lineage: Lineage, buf: &mut Vec<f64>) {
        let bar = self.bar(path, lineage.trainee);
        let Some(pick) = self.evaluator.consider(self.env, &path.pool, lineage, self.lineage_bar(path, lineage), buf)
        else {
            return;
        };
        if let Some(member) = self.member_by_trainee.get(lineage.trainee).copied().flatten()
            && pick.value > bar
        {
            path.member_best[member] = Some(pick);
            path.current = None;
        }
        if pick.value > path.best.map_or(0.0, |best_pick| best_pick.value) {
            path.best = Some(pick);
            path.current = None;
        }
        if self.settings.goal == Goal::Looped && path.is_looped(lineage) && pick.value > path.looped_bar() {
            path.best_looped = Some(pick);
            path.current = None;
        }
    }
    fn legal(&self, path: &PathState, lineage: Lineage) -> bool {
        path.pool.pair_ok(self.data.umas[lineage.trainee].id, lineage.first_parent, lineage.second_parent)
    }
}
