//! The solver's diagnostic switches, read from the environment once (`Checks`), and the check counters they report.
//!
//! Every environment variable the solver reads is read here, once per process, into one immutable `Checks` record;
//! a job borrows it (`checks::get`) and passes it down. The counters stay thread-local: they total the checks of
//! every job the thread runs, the CLI prints them when it exits (`CliBoundReport`, also on a panic), and no search
//! reads them, so they change no result.
use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

/// The diagnostic switches. Unset, each leaves the solver's results as they are; set, most only add checks or
/// reports. `RULE_OFF`, `SBOUND`, `CAP_RERUN`, `CAPS_TEST`, `EXACT_BUDGET`, `EXACT_STEPS`, `CF_BUDGET` and the
/// `PLAN_MEMO_*` switches change what the searches do (experiments and tests).
pub(crate) struct Checks {
    /// `RULE_OFF` set: the character rule is off, so `Card::blocks` never blocks a card (brute-force checks only:
    /// val/permcheck.py, val/owncheck.py).
    pub(crate) rule_off: bool,
    /// `DECK_DEBUG` set: a deck job prints its dominance-test counts, enumeration timings, skyline size, dropped
    /// cards, class clusters and per-uma search tallies on stderr.
    pub(crate) deck_debug: bool,
    /// `DECK_DEBUG_PAIRS` set: a deck job prints, per kept card, its robust dominators among the kept cards.
    pub(crate) deck_debug_pairs: bool,
    /// `CAREER_WORK` set: a career job prints its main search's work, budget, solved and proven counts on stderr.
    pub(crate) career_work: bool,
    /// `JOB_RSS` set: the CLI prints the process's resident memory after each job on stderr (`VmRSS`, Linux only).
    pub(crate) job_rss: bool,
    /// `JOINT_CHECK` set: every exact scoring through `joint_b` is compared with the full enumeration
    /// (`joint_trainee_ref`); a mismatch panics.
    pub(crate) joint_check: bool,
    /// `SB_CHECK` set: every exact scoring's state-conditioned value is compared with the exact value, and in a band
    /// pass with the state bound every leaf's bound with its root's and depth-one bounds (`SBCHK`); mode 3 also
    /// evaluates the bound below the root to make that check. The CLI prints `SBCHK` and `SBN` on exit.
    pub(crate) sb_check: bool,
    /// `SBOUND` (3 unless set to a number): the state-conditioned bound in the career's capped band pass
    /// (`CareerSetup::best_deck_capped`): 3 at each configuration's root, beside the cap; 1 also at depth 1; 2 the band
    /// without the cap (the bound at every node, a leaf kept when its state-conditioned value reaches the bar); 0 off.
    pub(crate) sbound: u8,
    /// `SBOUND` set to anything: the CLI prints the state bound's counts (`SBN`) on exit.
    pub(crate) sbound_report: bool,
    /// `CF_CHECK=s` (0 unset, 1 when set to anything but a number): every capped career deck search also runs the
    /// cap-free one beside it (all it changes put back) and compares answers; inside a cap-free search every node
    /// passes at s times its bound (s >= 1: more leaves reached, the band the same) and every leaf reached is scored
    /// exactly against the bounds of the nodes above it; every exact scoring is compared with the cap-free bound at
    /// its whole deck (`CFCHK`). 0 is off.
    pub(crate) cf_check: f64,
    /// `CF_CHECK` set to anything: the CLI prints `CFCHK` on exit.
    pub(crate) cf_report: bool,
    /// `RB_CHECK=s` (0 unset, 1 when set to anything but a number): outside the cap-free search, nodes pass at s
    /// times their ratio bounds, so more leaves are reached, and each leaf's value is checked against the bounds of
    /// every node on its path (`RBCHK`); the results are unchanged. 0 is off.
    pub(crate) rb_check: f64,
    /// `RB_CHECK` set to anything: the CLI prints `RBCHK` on exit.
    pub(crate) rb_report: bool,
    /// `UPPER_CHECK` set: the loop search checks every exact scoring against its negative-association bound
    /// (`upper_na`; a bound below the exact value panics) and prints the counts (`UCHK`).
    pub(crate) upper_check: bool,
    /// `UPPER_WHY` set: `upper_na` prints the outcome group that keeps it from bounding a deck.
    pub(crate) upper_why: bool,
    /// `SIM_CHECK_UB` set: the simulator checks its fast ceiling against `CareerSetup::ceiling` at every candidate it
    /// considers; a difference panics.
    pub(crate) sim_check_ub: bool,
    /// `CAP_RERUN=0`: a career deck search whose exact scorings breach their class's cap is not rerun cap-free; the
    /// breach is counted (`CfStats::unhandled`) and the search is not complete.
    pub(crate) cap_rerun_off: bool,
    /// `CAPS_TEST=a,b,c,d` (checks only; fields that are not numbers are skipped, and anything but four numbers is
    /// ignored): these class caps in place of `EXACT_CAPS` (lowered, they force breaches, so the cap-free rerun can
    /// be checked against `cap_audit`).
    pub(crate) caps_test: Option<[f64; 4]>,
    /// `EXACT_BUDGET=n`: the work budget of the loop search's exact pass (else `EXACT_BUDGET`) and of a career
    /// search (else `CAREER_BUDGET`).
    pub(crate) exact_budget: Option<usize>,
    /// `EXACT_STEPS=a,b,...` (fields that are not numbers skipped): the caps the loop search's exact pass tries, in
    /// order, in place of `EXACT_STEPS`.
    pub(crate) exact_steps: Option<Vec<f64>>,
    /// `CF_BUDGET=n`: the work budget of one cap-free career deck search (else twice `EXACT_BUDGET`).
    pub(crate) cf_budget: Option<usize>,
    /// `PLAN_MEMO` set: the planner prints each plan's memo lookups, hits, gated-off memos, refits and clears.
    pub(crate) plan_memo: bool,
    /// `PLAN_MEMO_OFF` set: every planner memo is off from the start (the test that compares against no memo).
    pub(crate) plan_memo_off: bool,
    /// `PLAN_MEMO_CHECK` set: every planner memo hit is computed again and compared bit for bit; a difference panics.
    pub(crate) plan_memo_check: bool,
    /// `PLAN_MEMO_CAP=n`: the entries a planner memo keeps before it is cleared (else `MEMO_CAP`).
    pub(crate) plan_memo_cap: Option<usize>,
    /// `LOOPOPT_STATE_IN=path`: the CLI restores the career worker's kept work from this file before any job.
    pub(crate) state_in: Option<String>,
    /// `LOOPOPT_STATE_OUT=path`: the CLI saves the career worker's kept work to this file once, after all jobs.
    pub(crate) state_out: Option<String>,
}

/// The process's checks, read from the environment on the first call.
pub(crate) fn get() -> &'static Checks {
    static CHECKS: OnceLock<Checks> = OnceLock::new();
    CHECKS.get_or_init(Checks::from_env)
}

impl Checks {
    fn from_env() -> Self {
        fn var(name: &str) -> Option<String> {
            std::env::var(name).ok()
        }
        fn set(name: &str) -> bool {
            std::env::var(name).is_ok()
        }
        fn number<T: std::str::FromStr>(name: &str) -> Option<T> {
            var(name).and_then(|v| v.parse().ok())
        }
        /// A scale switch: 0 unset, 1 when set to anything but a number.
        fn scale(name: &str) -> f64 {
            var(name).map_or(0.0, |v| v.parse().unwrap_or(1.0))
        }
        /// A list switch: its comma-separated fields that are numbers.
        fn numbers(name: &str) -> Option<Vec<f64>> {
            var(name).map(|v| v.split(',').filter_map(|field| field.parse().ok()).collect())
        }
        Self {
            rule_off: set("RULE_OFF"),
            deck_debug: set("DECK_DEBUG"),
            deck_debug_pairs: set("DECK_DEBUG_PAIRS"),
            career_work: set("CAREER_WORK"),
            job_rss: set("JOB_RSS"),
            joint_check: set("JOINT_CHECK"),
            sb_check: set("SB_CHECK"),
            sbound: number("SBOUND").unwrap_or(3),
            sbound_report: set("SBOUND"),
            cf_check: scale("CF_CHECK"),
            cf_report: set("CF_CHECK"),
            rb_check: scale("RB_CHECK"),
            rb_report: set("RB_CHECK"),
            upper_check: set("UPPER_CHECK"),
            upper_why: set("UPPER_WHY"),
            sim_check_ub: set("SIM_CHECK_UB"),
            cap_rerun_off: var("CAP_RERUN").is_some_and(|v| v == "0"),
            caps_test: numbers("CAPS_TEST")
                .and_then(|caps| (caps.len() == 4).then(|| [caps[0], caps[1], caps[2], caps[3]])),
            exact_budget: number("EXACT_BUDGET"),
            exact_steps: numbers("EXACT_STEPS"),
            cf_budget: number("CF_BUDGET"),
            plan_memo: set("PLAN_MEMO"),
            plan_memo_off: set("PLAN_MEMO_OFF"),
            plan_memo_check: set("PLAN_MEMO_CHECK"),
            plan_memo_cap: number("PLAN_MEMO_CAP"),
            state_in: var("LOOPOPT_STATE_IN"),
            state_out: var("LOOPOPT_STATE_OUT"),
        }
    }
}

/// `SB_CHECK`: per exact scoring, the state-conditioned value against the exact value; and the node bounds against
/// the leaves below them.
pub(crate) struct StateBoundChecks {
    /// Exact scorings compared.
    pub(crate) decks: usize,
    /// The highest and lowest state-conditioned value / exact value.
    pub(crate) max_ratio: f64,
    pub(crate) min_ratio: f64,
    /// Scorings whose state-conditioned value is below the exact value (by more than 1e-9).
    pub(crate) below: usize,
    /// Leaves checked against their root's and depth-one bounds, and those above either (by more than 1e-9).
    pub(crate) leaves: usize,
    pub(crate) violations: usize,
}

/// The state bound's work (`SBOUND`): configurations cut at their root, bound evaluations, nodes cut.
pub(crate) struct StateBoundCounts {
    pub(crate) configurations_cut: usize,
    pub(crate) evaluations: usize,
    pub(crate) nodes_cut: usize,
}

/// `CF_CHECK`: the cap-free searches run beside capped ones, and the cap-free bounds against exact values.
pub(crate) struct CapFreeChecks {
    /// Cap-free searches run beside a capped one.
    pub(crate) searches: usize,
    /// Capped answers below the cap-free one (a deck the caps pruned).
    pub(crate) capped_below: usize,
    /// Cap-free answers below the capped one (a bug).
    pub(crate) cap_free_below: usize,
    /// Cap-free searches that proved nothing.
    pub(crate) unproven: usize,
    /// Leaves checked against their ancestors' bounds, and violations (an exact value above a bound).
    pub(crate) leaves: usize,
    pub(crate) leaf_violations: usize,
    /// Exact scorings checked against the cap-free bound at their whole deck, and violations.
    pub(crate) scorings: usize,
    pub(crate) scoring_violations: usize,
    /// The largest exact value / bound.
    pub(crate) max_ratio: f64,
}

/// `RB_CHECK`: leaf values against the ratio bounds of the nodes on their path.
pub(crate) struct RatioBoundChecks {
    pub(crate) leaves: usize,
    /// (leaf, clause, ancestor) tests.
    pub(crate) tests: usize,
    /// Values above an ancestor's bound by more than 1e-9.
    pub(crate) violations: usize,
    /// The highest value / bound.
    pub(crate) max_ratio: f64,
}

/// `UPPER_CHECK`: exact scorings checked against `upper_na`, the highest bound / exact value, decks it does not bound.
#[derive(Clone, Copy)]
pub(crate) struct UpperChecks {
    pub(crate) checked: usize,
    pub(crate) max_ratio: f64,
    pub(crate) uncovered: usize,
}

/// `DECK_DEBUG`: the deck searches' setup and tree seconds, configurations and count vectors.
#[derive(Clone, Copy)]
pub(crate) struct SearchProfile {
    pub(crate) setup_seconds: f64,
    pub(crate) tree_seconds: f64,
    pub(crate) configurations: usize,
    pub(crate) count_vectors: usize,
}

thread_local! {
    pub(crate) static SBCHK: RefCell<StateBoundChecks> = const {
        RefCell::new(StateBoundChecks {
            decks: 0,
            max_ratio: 0.0,
            min_ratio: f64::INFINITY,
            below: 0,
            leaves: 0,
            violations: 0,
        })
    };
    pub(crate) static SBN: RefCell<StateBoundCounts> =
        const { RefCell::new(StateBoundCounts { configurations_cut: 0, evaluations: 0, nodes_cut: 0 }) };
    pub(crate) static CFCHK: RefCell<CapFreeChecks> = const {
        RefCell::new(CapFreeChecks {
            searches: 0,
            capped_below: 0,
            cap_free_below: 0,
            unproven: 0,
            leaves: 0,
            leaf_violations: 0,
            scorings: 0,
            scoring_violations: 0,
            max_ratio: 0.0,
        })
    };
    pub(crate) static RBCHK: RefCell<RatioBoundChecks> =
        const { RefCell::new(RatioBoundChecks { leaves: 0, tests: 0, violations: 0, max_ratio: 0.0 }) };
    pub(crate) static UCHK: Cell<UpperChecks> =
        const { Cell::new(UpperChecks { checked: 0, max_ratio: 0.0, uncovered: 0 }) };
    pub(crate) static SPT: Cell<SearchProfile> = const {
        Cell::new(SearchProfile { setup_seconds: 0.0, tree_seconds: 0.0, configurations: 0, count_vectors: 0 })
    };
}
