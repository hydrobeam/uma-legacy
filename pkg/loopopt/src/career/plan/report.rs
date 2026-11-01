//! What a plan job answers (`next`, career/plan/output.rs `assemble_plan_output` and `index_rule`), and its
//! follow-ups: a roll of its pick (`RollReply`) and its robustness check (`CheckReply`, plan/robust.rs).
use crate::career::reply::CareerReport;
use crate::career::search::Assumes;
use crate::deck::pdeck::Name;
use crate::refusal::Refusal;
use serde::Serialize;
use std::collections::BTreeMap;
use ts_rs::TS;

/// The plan (`next`), or why there is none.
#[derive(Serialize, TS)]
#[serde(untagged)]
pub(crate) enum PlanReply {
    Refusal(Refusal),
    Plan(Box<Plan>),
}

/// The page's next career: its options by score, its pick, and today's cheapest route to a fully looped parent.
#[derive(Serialize, TS)]
pub(crate) struct Plan {
    /// the plan's pick (null: no row)
    pub(crate) choice: Option<CareerReport>,
    pub(crate) rule: PlanRule,
    pub(crate) mode: PlanMode,
    /// expected careers to a fully looped parent with the veterans now (null: no route lands)
    pub(crate) v0: Option<f64>,
    /// the career a roll is of (`next.roll`; null where the pick was not scored)
    pub(crate) outcomes: Option<RollCareer>,
    /// the route's A and B, in its order (empty without a route)
    pub(crate) members: Vec<RouteMember>,
    /// the cheapest routes, cheapest first
    pub(crate) trios: Vec<RankedRoute>,
    /// today's cheapest route (null: none)
    pub(crate) v0_plan: Option<Route>,
    /// the stage candidates and their continuations the plan was fitted to: [chara, lam, phi, pinf, p0]
    pub(crate) fit: Vec<(u32, f64, f64, f64, f64)>,
    /// per candidate (chara id, as text) whose continuation was fitted to this pool, the two builders (chara ids) behind
    /// her full-loop ceiling
    pub(crate) pinf_builders: BTreeMap<String, [u32; 2]>,
    /// per candidate (chara id, as text) whose step is her kept cap's, its careers and her odds then
    pub(crate) step_caps: BTreeMap<String, StepCap>,
    pub(crate) loop_members: Vec<UmaName>,
    pub(crate) loop_note: Option<String>,
    pub(crate) hint: Vec<CandidateHint>,
    pub(crate) cut: Vec<CandidateHint>,
    pub(crate) side_only: Vec<UmaName>,
    pub(crate) umas_scored: usize,
    pub(crate) umas_searched: usize,
    pub(crate) umas_certified: usize,
    pub(crate) umas_unproven: usize,
    pub(crate) work: PlanWork,
    pub(crate) sur: u8,
    pub(crate) sur_diff: Vec<SurrogateDifference>,
    pub(crate) loop_ms: f64,
    pub(crate) search_ms: f64,
    pub(crate) combo_ms: f64,
    pub(crate) index_ms: f64,
    pub(crate) total_ms: f64,
    pub(crate) options: Vec<PlanOption>,
    /// routes the scorings' memory budget left out (each lowers its row's saving, never raises it)
    pub(crate) routes_dropped: f64,
    /// of those, the ones no check cleared (each could have lowered a row's V after her)
    pub(crate) routes_dropped_uncertified: f64,
    /// false where the memory budget left out something that could move a score: the savings are then lower bounds
    pub(crate) scoring_complete: bool,
    /// a roll sent with the plan job (`next.roll`), answered from the state the plan just kept
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) roll: Option<RollReply>,
    /// measurement (`next.oracle`): a diagnostic V beside the plan's
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) oracle: Option<OracleReport>,
    /// measurement (`next.pair_sweep`)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) pair_sweep: Option<crate::career::plan::sweep::PairSweep>,
    /// measurement (`next.pair_ascent`)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) pair_ascent: Option<crate::career::plan::ascent::PairAscent>,
    /// measurement (`next.pair_decompose`)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) pair_decompose: Option<Vec<crate::career::plan::ascent::PairDecomposition>>,
    /// the robustness check (`next.robust`), when the plan job asks it
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) robust: Option<CheckReply>,
    /// measurement (`next.audit_lifts`)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) lift_audit: Option<LiftAuditReply>,
    /// measurement (`next.proxy_check`)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) proxy_check: Option<crate::career::plan::rows::ProxyCheck>,
}

/// The lift audit (`next.audit_lifts`), or the browser build's refusal of it (the native loopopt runs it).
#[derive(Serialize, TS)]
#[serde(untagged)]
pub(crate) enum LiftAuditReply {
    Rows(Vec<crate::career::plan::audit::LiftAudit>),
    Refusal(Refusal),
}

/// The oracle asked for (`next.oracle`), or the browser build's refusal of it.
#[derive(Serialize, TS)]
#[serde(untagged)]
pub(crate) enum OracleReport {
    Joint(Box<crate::career::plan::oracle::JointOracle>),
    Sequenced(Box<crate::career::plan::sequenced::SequencedOracle>),
    Refusal(Refusal),
}

/// What the plan's rule is (always the index rule).
#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PlanRule {
    Index,
}

/// What the plan's routes are (always trios: A and B building, C finishing).
#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PlanMode {
    Trio,
}

/// The career a roll is of: the pick, her uncertain targets and the scenario's guaranteed ones (job-target indices).
#[derive(Clone, Serialize, TS)]
pub(crate) struct RollCareer {
    pub(crate) trainee: u32,
    pub(crate) parents: [usize; 2],
    pub(crate) career: String,
    pub(crate) uncv: Vec<usize>,
    pub(crate) fixed: Vec<usize>,
}

/// A route to a fully looped parent: A, B (the cheaper step first) and C by name, their steps' expected careers, and
/// A's and B's sides ("real:i" her veteran i, "current:i,j" her best career now on veterans i and j, "alt:i,j"
/// another career of hers).
#[derive(Clone, Serialize, TS)]
pub(crate) struct Route {
    pub(crate) a: Option<String>,
    pub(crate) b: Option<String>,
    pub(crate) c: Option<String>,
    /// each step's expected careers (null: infinite, a step that cannot land)
    #[ts(type = "[number | null, number | null, number | null]")]
    pub(crate) careers: [f64; 3],
    pub(crate) sides: [String; 2],
}

/// One of the cheapest routes, with its value and whether it is about as cheap as V's.
#[derive(Clone, Serialize, TS)]
pub(crate) struct RankedRoute {
    #[serde(flatten)]
    pub(crate) route: Route,
    pub(crate) value: f64,
    /// within NEAR_TRIO_SHARE of V's
    pub(crate) near: bool,
}

/// A parent on the plan's route (A or B): her step to an all-targets veteran.
#[derive(Clone, Serialize, TS)]
pub(crate) struct RouteMember {
    /// her name (null: no uma)
    pub(crate) trainee: Option<String>,
    /// the chance of landing every target today of the career her fresh side is from
    pub(crate) p: f64,
    /// that career's parents, by legacy index (null for a real full veteran)
    pub(crate) parents: Option<[usize; 2]>,
    /// that career is another of her top careers, not her best today
    pub(crate) alternative: bool,
    /// her step is her kept cap's (`caps`), cheaper than her continuation's
    pub(crate) capped: bool,
    /// a search with both its parents fixed proved that career's odds
    pub(crate) proven: bool,
    /// she already has a full veteran (a poorer parent than a fresh one)
    pub(crate) has_full_veteran: bool,
    /// the route uses her existing full veteran: her step costs no careers
    pub(crate) real: bool,
    /// her effective ceiling, the odds her lifts approach (null when lam is 0: no building)
    pub(crate) p_inf: Option<f64>,
    /// the chance a building career lifts her odds, before it falls with the remaining gap
    pub(crate) lam: f64,
    /// the share of the remaining log-odds gap to p_inf that each lift closes
    pub(crate) phi: f64,
    /// the odds her continuation is anchored at
    pub(crate) p0: f64,
    /// the expected careers she spends building lifts before rerunning her best career; 0 when she does not build
    pub(crate) building_careers: f64,
    /// her odds once building stops
    pub(crate) p_switch: f64,
    /// the career id of the row her lifts are fitted from (null where she grinds, her continuation was given, or her
    /// step is her cap's)
    pub(crate) builder: Option<String>,
    /// her step's expected careers (0 when `real`; null: infinite)
    #[ts(type = "number | null")]
    pub(crate) careers: f64,
}

/// A candidate whose step is her kept cap's: its careers and her odds then.
#[derive(Clone, Serialize, TS)]
pub(crate) struct StepCap {
    pub(crate) careers: f64,
    pub(crate) p: f64,
}

/// An uma by chara id and name.
#[derive(Clone, Serialize, TS)]
pub(crate) struct UmaName {
    pub(crate) id: u32,
    pub(crate) name: String,
}

/// A stage candidate the plan considered (`hint`: taken; `cut`: past the slots), with the odds it was ranked by and
/// where they come from ("searched", "today", "full_loop").
#[derive(Clone, Serialize, TS)]
pub(crate) struct CandidateHint {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) ceiling: f64,
    pub(crate) basis: &'static str,
}

/// The plan's own work: deck-search nodes and exact scorings, the deck caps' certificates, and its counters.
#[derive(Clone, Serialize, TS)]
pub(crate) struct PlanWork {
    pub(crate) nodes: usize,
    pub(crate) exact: usize,
    pub(crate) max_ratio: f64,
    pub(crate) class_max_ratio: [f64; 4],
    pub(crate) class_max_deck: [Option<ClassMaxDeck>; 4],
    pub(crate) cap: f64,
    pub(crate) class_caps: [f64; 4],
    /// the plan's timers and work counters by name
    pub(crate) prof: BTreeMap<&'static str, f64>,
}

/// The deck that reached a class's largest exact / product odds: its trainee and cards by name.
#[derive(Clone, Serialize, TS)]
pub(crate) struct ClassMaxDeck {
    pub(crate) trainee: String,
    pub(crate) cards: Vec<Name>,
}

/// Where the surrogate's best career for a trainee differs from the search's (`sur` 2): each one's parents and odds.
#[derive(Clone, Serialize, TS)]
pub(crate) struct SurrogateDifference {
    pub(crate) trainee: String,
    pub(crate) search: (Option<[usize; 2]>, f64),
    pub(crate) sur: (Option<[usize; 2]>, f64),
}

/// One career of the plan's table: a career report with its place in the plan.
#[derive(Clone, Serialize, TS)]
pub(crate) struct PlanOption {
    #[serde(flatten)]
    pub(crate) report: CareerReport,
    /// her place in today's estimate
    pub(crate) role: OptionRole,
    /// the search's best career
    pub(crate) best_odds: bool,
    /// the plan's pick
    pub(crate) chosen: bool,
    /// both parents are full veterans
    pub(crate) looped: bool,
    /// scored from her own career search (false: a builder the plan could not turn on, from her class's decks)
    pub(crate) exact: bool,
    /// a builder's next best career, from her cached decks
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) alternative: Option<bool>,
    /// false: her career search was cut short, or her deck class breached its assumed cap
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) proven: Option<bool>,
    /// false: an alternative row never searched on its pair
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) searched: Option<bool>,
    /// the named assumptions her proven career rests on
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) assumes: Option<Assumes>,
    /// her stage step's careers (null: not a stage candidate)
    pub(crate) stage_cost: Option<f64>,
    /// the careers she saves toward a fully looped parent, net (null: not scored)
    pub(crate) index: Option<f64>,
    pub(crate) saving: Option<f64>,
    /// the chance her result lowers the careers to a fully looped parent
    pub(crate) lift: Option<f64>,
    /// over her results that lower it, the side lifted most often (null where the row was not scored)
    pub(crate) lifts_side: Option<LiftsSide>,
    /// the table's order (null: nothing scored)
    pub(crate) score: Option<f64>,
}

/// A row's place in the plan.
#[derive(Clone, Copy, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OptionRole {
    /// one of the best trio's A and B
    Plan,
    /// its finisher, on A's and B's real full veterans
    Finish,
    Loop,
    Candidate,
    Builder,
}

/// Over a row's outcomes that lower V: the stage candidate (uma id) whose side the route taken lifts most often and
/// that chance (null and 0 where none), and the chance the route has her own full veteran as a side.
#[derive(Clone, Serialize, TS)]
pub(crate) struct LiftsSide {
    pub(crate) uma: Option<u32>,
    pub(crate) chance: f64,
    pub(crate) own: f64,
}

/// A roll of the plan's pick (`next.roll`), or why there is none.
#[derive(Serialize, TS)]
#[serde(untagged)]
pub(crate) enum RollReply {
    Answer(RollAnswer),
    Refusal(Refusal),
}

/// A roll's answer from the plan's kept roll state, with its time and counters.
#[derive(Serialize, TS)]
pub(crate) struct RollAnswer {
    #[serde(flatten)]
    pub(crate) outcome: RollOutcome,
    pub(crate) ms: f64,
    /// the roll's own timers and counters
    pub(crate) prof: BTreeMap<&'static str, f64>,
    /// answered from the state the plan job just kept (`next.roll` sent with the plan)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) planned: Option<bool>,
}

/// What a roll's outcome is worth, or why it could not be scored.
#[derive(Serialize, TS)]
#[serde(untagged)]
pub(crate) enum RollOutcome {
    /// the careers her veteran saves at that outcome, to 0.01
    Saving {
        y: f64,
    },
    Refusal(Refusal),
}

/// The plan's robustness check so far (`next.robust`), or why there is none.
#[derive(Serialize, TS)]
#[serde(untagged)]
pub(crate) enum CheckReply {
    Report(Box<CheckReport>),
    /// the worker keeps no plan to check (`unavailable`: always true)
    Unavailable {
        error: String,
        unavailable: bool,
    },
    Refusal(Refusal),
}

/// The check's report: the plan's pick and rows (career ids), each variant done, and the pair search once it ran.
#[derive(Serialize, TS)]
pub(crate) struct CheckReport {
    /// the plan's pick
    pub(crate) plan: String,
    pub(crate) rows: Vec<String>,
    pub(crate) variants: Vec<VariantReport>,
    /// the variants left
    pub(crate) pending: usize,
    /// every variant done scores the pick highest
    pub(crate) stable: bool,
    /// each variant's strict argmax, in the order first picked
    pub(crate) contenders: Vec<String>,
    /// the variants disagree: the pick varies under them (a sensitivity, not a tie)
    pub(crate) pick_varies: bool,
    pub(crate) check_ms: f64,
    /// the within-trainee pair search (null before it ran)
    pub(crate) pairs: Option<PairSearch>,
    /// every scoring of the check so far was complete
    pub(crate) scoring_complete: bool,
    /// why the pair search asked for is not there
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) pairs_error: Option<&'static str>,
}

/// A check variant's scores (`kind` "plan" for the plan's own; else the variant's), its pick (a row's career id) and
/// the routes its scoring's memory budget dropped (null: the plan's own entry, whose `next` reports them).
#[derive(Clone, Serialize, TS)]
pub(crate) struct VariantReport {
    pub(crate) kind: &'static str,
    pub(crate) ceil_share: f64,
    pub(crate) lam_decay: f64,
    pub(crate) odds_scale: f64,
    /// the scaled umas (uma indices) and their scales; none for a uniform scale
    pub(crate) scaled: Vec<(usize, f64)>,
    #[ts(type = "number | null")]
    pub(crate) v0: f64,
    pub(crate) pick: String,
    pub(crate) score: f64,
    pub(crate) margin: f64,
    pub(crate) scores: Vec<f64>,
    pub(crate) routes_dropped: Option<f64>,
    pub(crate) routes_dropped_uncertified: Option<f64>,
    pub(crate) partner_setups_dropped: Option<f64>,
}

/// The within-trainee pair search (`next.pair_search`): one report per trainee it ran for, the shared-pair finalists,
/// the best (trainee, pair) by score and the career it recommends.
#[derive(Clone, Serialize, TS)]
pub(crate) struct PairSearch {
    pub(crate) trainees: Vec<TraineePairsReport>,
    pub(crate) shared: Vec<SharedPair>,
    pub(crate) shared_coverage: SharedCoverage,
    /// the best (trainee, pair) by score over both phases (null: none)
    pub(crate) best: Option<BestPair>,
    pub(crate) recommendation: Recommendation,
    /// trainees the work limit left out of phase 1
    pub(crate) skipped_for_work: Vec<String>,
    /// the pairs scored per trainee at most
    pub(crate) pair_bound: usize,
    pub(crate) pairs_ms: f64,
    pub(crate) scoring_complete: bool,
}

/// How a finalist's career was chosen and searched.
#[derive(Clone, Serialize, TS)]
pub(crate) struct FinalistReport {
    pub(crate) career: String,
    pub(crate) score: f64,
    /// how its exact search ended
    pub(crate) search: &'static str,
    pub(crate) deck_choice: DeckChoice,
}

/// A finalist's deck: its P(all)-best cached deck's score, its best-scoring cached deck's (and whether that is another
/// deck), the exact search's career's, and which was kept ("searched" or "cached").
#[derive(Clone, Copy, Serialize, TS)]
pub(crate) struct DeckChoice {
    pub(crate) p_all_deck_score: f64,
    pub(crate) best_cached_score: f64,
    pub(crate) score_deck_differs: bool,
    pub(crate) searched_score: f64,
    pub(crate) kept: &'static str,
}

/// The pair search's deterministic work: the outcomes its scorings evaluated and the deck searches' nodes.
#[derive(Clone, Copy, Serialize, TS)]
pub(crate) struct PairSearchWork {
    pub(crate) outcomes: f64,
    pub(crate) nodes: f64,
}

/// Phase 1: one trainee's best pair against her shown career.
#[derive(Clone, Serialize, TS)]
pub(crate) struct TraineePairsReport {
    pub(crate) trainee: String,
    /// why she was searched: "pick", "scenario" or "top"
    pub(crate) reason: &'static str,
    /// the career she is weighed against
    pub(crate) shown: String,
    pub(crate) shown_score: f64,
    pub(crate) best_parents: [usize; 2],
    pub(crate) career: String,
    pub(crate) score: f64,
    pub(crate) search: &'static str,
    pub(crate) proven: bool,
    pub(crate) deck_choice: DeckChoice,
    pub(crate) margin_over_shown: f64,
    /// her best beats her shown career on another pair
    pub(crate) better_parents: bool,
    /// over the best other pair evaluated (null: none)
    pub(crate) margin_over_other_pairs: Option<f64>,
    pub(crate) best_other_pair: Option<String>,
    /// the margin over her shown career under each building-curve corner
    pub(crate) corner_margins: Vec<f64>,
    /// at her own odds x0.76 and x1.32
    pub(crate) scale_margins: Vec<f64>,
    /// the other finalists
    pub(crate) alternatives: Vec<FinalistReport>,
    pub(crate) pairs_scored: usize,
    pub(crate) pairs_runnable: usize,
    pub(crate) pairs_truncated: bool,
    pub(crate) trainee_ms: f64,
    pub(crate) work: PairSearchWork,
}

/// Phase 2: a best pair of phase 1 tried for a trainee without her own search.
#[derive(Clone, Serialize, TS)]
pub(crate) struct SharedPair {
    #[serde(flatten)]
    pub(crate) finalist: FinalistReport,
    pub(crate) trainee: String,
    /// always "shared_pair"
    pub(crate) reason: &'static str,
    pub(crate) best_parents: [usize; 2],
    pub(crate) pairs_tried: usize,
    pub(crate) phase_ms: f64,
    pub(crate) work: PairSearchWork,
}

/// Phase 2's reach: the trainees tried, the careers scored, the pairs they could run, the trainees the bound cut.
#[derive(Clone, Serialize, TS)]
pub(crate) struct SharedCoverage {
    pub(crate) trainees: usize,
    pub(crate) careers: usize,
    pub(crate) pairs_runnable: usize,
    pub(crate) trainees_truncated: usize,
}

/// The best (trainee, pair) by score, its margin over the pick's trainee's shown career, and where it is not the
/// pick, its margins over the pick.
#[derive(Clone, Serialize, TS)]
pub(crate) struct BestPair {
    pub(crate) trainee: String,
    pub(crate) career: String,
    pub(crate) best_parents: [usize; 2],
    pub(crate) score: f64,
    pub(crate) reason: &'static str,
    pub(crate) pick_trainee_shown: Option<String>,
    pub(crate) margin_over_pick_trainee_shown: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) versus_pick: Option<VersusPick>,
}

/// A career's margins over the plan's pick.
#[derive(Clone, Serialize, TS)]
pub(crate) struct VersusPick {
    pub(crate) career: String,
    pub(crate) margins: ScenarioMargins,
    /// the scenarios whose margin's sign differs from the nominal one ("pick_down", "pick_up")
    pub(crate) varies_under: Vec<&'static str>,
}

/// Margins under the nominal model and the two per-uma scenarios.
#[derive(Clone, Copy, Serialize, TS)]
pub(crate) struct ScenarioMargins {
    pub(crate) nominal: f64,
    pub(crate) pick_down: f64,
    pub(crate) pick_up: f64,
}

/// The career the page recommends: the higher-scoring of the plan's pick and the best pair, as fitted.
#[derive(Clone, Serialize, TS)]
pub(crate) struct Recommendation {
    pub(crate) career: String,
    pub(crate) trainee: String,
    pub(crate) trainee_id: u32,
    pub(crate) parents: [usize; 2],
    /// "cross_trainee", "pick_trainee_pair" or "plan_pick"
    pub(crate) source: &'static str,
    /// its pair-search score (null: the plan's pick)
    pub(crate) score: Option<f64>,
    pub(crate) report: CareerReport,
}

impl OptionRole {
    /// The role's name, as the output writes it.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Finish => "finish",
            Self::Loop => "loop",
            Self::Candidate => "candidate",
            Self::Builder => "builder",
        }
    }
}
