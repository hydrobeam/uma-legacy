//! What a career job answers (`CareerReply`, told apart by its `kind`): the career search with its reports and
//! certificates, a plan's roll or check from the worker's kept state, the simulator's report, the kept state out and
//! back in, a job's answer with the roll it carried, or a refusal. The page's types are generated from these
//! (`crate::ts_gen`): a union the page narrows by `kind`. A reply owns what it holds: it outlives the job's search.
use crate::career::Pink;
use crate::career::plan::report::{CheckReply, PlanReply, RollReply};
use crate::career::search::{Assumes, Unproven};
use crate::chart::ScheduleRow;
use crate::codes::{Aptitude, Grades};
use crate::deck::pdeck::ResultDeck;
use crate::loop_planner::output::{OutfitLabel, TargetCover};
use crate::refusal::Refusal;
use crate::skills::LegacyKind;
use serde::Serialize;
use ts_rs::TS;

/// A career job's answer, by its `kind`.
#[derive(Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum CareerReply {
    Refusal(Refusal),
    /// the career search, with its plan (`next`) when asked
    Search(Box<CareerSearch>),
    /// a plan's roll answered from the worker's kept state, no search (`next.roll`)
    Roll {
        roll: RollReply,
    },
    /// a plan's check answered from the worker's kept state, no search (`next.robust`)
    Check {
        robust: CheckReply,
    },
    /// the simulator's runs (`simulate`; native only)
    Simulation(Box<crate::career::sim::report::SimReport>),
    /// the simulator's `trace` policy
    Trace(Box<crate::career::sim::report::TraceReport>),
    /// the kept work out (`state_out`)
    StateOut(crate::career::state::StateOut),
    /// what a stored state put back (`state_in`)
    StateIn(crate::career::state::StateCounts),
    /// a job's own answer with the roll of a kept plan it carried (`roll`, `run_carried_roll`)
    Carried(Box<CarriedRoll>),
    /// (tests: a job whose `Env` went to a check instead of the search; not in the page's types)
    #[cfg(test)]
    #[ts(skip)]
    Hooked {},
}

/// A job's answer with the roll it carried (`run_carried_roll`).
#[derive(Serialize, TS)]
pub(crate) struct CarriedRoll {
    /// the job's own answer (never a carried one)
    pub(crate) reply: CareerReply,
    pub(crate) roll: RollReply,
}

/// The career search's answer (career/run.rs `search_output`, or one career as it is, `given`): its careers, its counts,
/// and what the job's searches certified (`note_job_certificates`).
#[derive(Serialize, TS)]
pub(crate) struct CareerSearch {
    /// the best careers, best first (none for a plan job's `no_reports`)
    pub(crate) results: Vec<CareerReport>,
    /// careers it weighed
    pub(crate) candidates: usize,
    pub(crate) solved: usize,
    pub(crate) bounded: usize,
    pub(crate) scored: usize,
    pub(crate) skipped: usize,
    /// the ceiling of every career left unsolved (0 when none is left; null where one career was given)
    pub(crate) next_bound: Option<f64>,
    /// false: a search of the job was cut short, or certified at a lower deck cap only
    pub(crate) complete: bool,
    /// the best odds' order is certified
    pub(crate) proven_top: bool,
    /// the deck cap certified
    pub(crate) cap: f64,
    /// the assumed cap per deck class
    pub(crate) class_caps: [f64; 4],
    pub(crate) work: SearchWork,
    /// one career reported as it is (`career`): no search weighed it against any other
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) given: Option<bool>,
    /// why the main search is not complete
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) unproven: Option<Unproven>,
    /// the named assumptions the main search's proof rests on beyond the derived bounds and the measured class caps
    /// (docs/SOLVER.md, *What "proven" rests on*): still a proof
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) assumes: Option<Assumes>,
    /// a comparison's job
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) compare: Option<bool>,
    /// a plan job's plan
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) next: Option<PlanReply>,
    /// what every search of the job certified
    pub(crate) certificates: JobCertificates,
    /// the solver's time in seconds (0 in the browser build, which has no clock here)
    pub(crate) timing_s: f64,
}

/// A search's own work: deck-search nodes and exact scorings (0 for a search kept from an earlier job).
#[derive(Clone, Serialize, TS)]
pub(crate) struct SearchWork {
    pub(crate) nodes: usize,
    pub(crate) exact: usize,
}

/// What every search of the job certified (`note_job_certificates`); each field only where it applies.
#[derive(Default, Serialize, TS)]
pub(crate) struct JobCertificates {
    /// deck bands too large to check in full (each then certified at a lower cap)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) band_overflows: Option<usize>,
    /// per deck class past its assumed cap: [class, ratio, cap]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) cap_exceeded: Option<Vec<(usize, f64, f64)>>,
    /// a `cap_free` job
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) cap_free: Option<bool>,
    /// the deck searches rerun without the cap after a breach
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) cap_rerun: Option<CapRerun>,
    /// capped deck searches of the job that scored a deck past its class's cap
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) cap_breaches: Option<usize>,
    /// trainees (chara ids) whose deck class breached its cap, here or before
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) cap_breached: Option<Vec<u32>>,
    /// the largest exact / product odds of a deck past its class's cap within its nested-chain bound
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) nested_max_ratio: Option<f64>,
    /// the named assumptions any search of the job rests on
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) job_assumes: Option<Assumes>,
    /// why any search of the job is not complete
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) job_unproven: Option<Unproven>,
}

/// The cap-free reruns of the job's deck searches.
#[derive(Clone, Serialize, TS)]
pub(crate) struct CapRerun {
    /// searches rerun without the cap
    pub(crate) searches: usize,
    /// of those, the ones that proved nothing
    pub(crate) unproven: usize,
    /// breaches left unanswered (`CAP_RERUN=0`)
    pub(crate) unanswered: usize,
    pub(crate) work: usize,
}

/// One career (career/outcomes.rs `report`): the trainee and her parents (by legacy index), her legacies, per target
/// odds, aptitudes, deck and schedule.
#[derive(Clone, Serialize, TS)]
pub(crate) struct CareerReport {
    pub(crate) trainee: CareerTrainee,
    /// the career's identity (`kept::career_id`: her uma, parents in order and deck)
    pub(crate) career: String,
    pub(crate) parents: [usize; 2],
    /// her six legacies
    pub(crate) slots: Vec<CareerSlot>,
    /// per target, in the job's order
    pub(crate) targets: Vec<CareerTarget>,
    /// the chance she lands every target
    pub(crate) p_all: f64,
    /// the chance she lands all but at most one
    pub(crate) p_minus1: f64,
    /// the targets she lands, expected
    pub(crate) expected: f64,
    /// the search's ceiling on her odds
    pub(crate) bound: f64,
    pub(crate) deck: ResultDeck,
    /// per pink the six legacies carry: the chance at least one of the two inheritance events passes it on
    pub(crate) pinks: Vec<PinkInheritance>,
    /// her aptitude grades and her veteran's pink spark (null: no pink career for her)
    pub(crate) aptitudes: Option<Aptitudes>,
    /// every G1 she could enter, in turn order (the search's best career and the plan's pick only)
    pub(crate) schedule: Option<Vec<CareerScheduleRow>>,
    /// [mask]: the chance of landing exactly the targets in mask (as `schedule`, up to 12 targets)
    pub(crate) outcome_sets: Option<Vec<f64>>,
    /// her G1 wins, expected (as `schedule`)
    pub(crate) g1_wins: Option<f64>,
}

/// The trainee of a career and her outfit.
#[derive(Clone, Serialize, TS)]
pub(crate) struct CareerTrainee {
    pub(crate) id: u32,
    pub(crate) name: String,
    /// her outfits that cover a target as this career's
    pub(crate) outfits: Vec<OutfitLabel>,
    pub(crate) covers: Vec<TargetCover>,
    /// the outfit (card id) her schedule runs
    pub(crate) schedule_outfit: u32,
}

/// One of a career's six legacies.
#[derive(Clone, Serialize, TS)]
pub(crate) struct CareerSlot {
    pub(crate) kind: LegacyKind,
    /// the parent's legacy index
    pub(crate) legacy: usize,
    /// a grandparent: which of the parent's two (null: a parent)
    pub(crate) gp: Option<usize>,
    pub(crate) chara: u32,
    pub(crate) name: String,
    /// relation points
    pub(crate) rel: f64,
    /// shared G1 wins, as points
    pub(crate) race: f64,
    /// affinity: rel + race
    pub(crate) aff: f64,
    /// her spark's stars per target, in the job's order
    pub(crate) stars: std::rc::Rc<Vec<u8>>,
    /// her pink spark (null: a veteran the planner imagines, whose pink is not known)
    pub(crate) pink: Option<Pink>,
}

/// One target of a career.
#[derive(Clone, Serialize, TS)]
pub(crate) struct CareerTarget {
    pub(crate) group: u32,
    /// the chance inheritance gives it
    pub(crate) q: f64,
    /// legacies with its spark
    pub(crate) carriers: i32,
    /// the chance she lands it
    pub(crate) p: f64,
    /// a gold the scenario hands out
    pub(crate) gold: bool,
    pub(crate) buy: TargetBuy,
}

/// How she buys a target and lands it as a spark, by `kind`: the chance she buys it as gold and as white, and lands
/// each.
#[derive(Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum TargetBuy {
    /// an uncertain target, with its sources
    Sourced(SourcedBuy),
    /// a gold the scenario hands out
    Scenario(ScenarioBuy),
}

/// An uncertain target's buying and landing, with its sources.
#[derive(Clone, Serialize, TS)]
pub(crate) struct SourcedBuy {
    pub(crate) gold: f64,
    pub(crate) white: f64,
    pub(crate) land_gold: f64,
    pub(crate) land_white: f64,
    /// the deck hints its gold
    pub(crate) deck_gold: f64,
    /// the deck hints its white, given it hints no gold
    pub(crate) deck_white: f64,
    /// her own story events give it
    pub(crate) own_event: f64,
    /// inspiration gives it
    pub(crate) inherit: f64,
    /// her outfit's own skill: 0 none, 1 the white, 2 the gold
    pub(crate) outfit: u8,
}

/// A scenario gold: bought for sure, landing at the gold rate.
#[derive(Clone, Serialize, TS)]
pub(crate) struct ScenarioBuy {
    pub(crate) gold: f64,
    pub(crate) white: f64,
    pub(crate) land_gold: f64,
    pub(crate) land_white: f64,
}

/// A pink (aptitude) spark the legacies carry.
#[derive(Clone, Serialize, TS)]
pub(crate) struct PinkInheritance {
    pub(crate) aptitude: Aptitude,
    /// the chance at least one of the two inheritance events passes it on
    pub(crate) q: f64,
    /// legacies carrying it
    pub(crate) carriers: usize,
}

/// Her aptitude grades and her veteran's pink spark.
#[derive(Clone, Serialize, TS)]
pub(crate) struct Aptitudes {
    /// her grades before any pink spark fires
    pub(crate) base: Grades,
    /// the chance of each pink on her veteran, an entry per aptitude in the solver's order (`Aptitude::ORDER`: the
    /// page's roll draws from them in it)
    pub(crate) pink_gen: Vec<PinkTypeChance>,
    /// where her grades start
    pub(crate) start: AptitudeStart,
}

/// Her starting grades, by `kind`: known (one outcome), or one per outcome of a veteran among her legacies whose result
/// is not known yet.
#[derive(Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AptitudeStart {
    Known(PinkOutcomeReport),
    /// each possible start, with its chance
    Outcomes {
        outcomes: Vec<WeightedPinkOutcome>,
    },
}

/// A start's grades and the legacy pinks that can fire at an April inspiration.
#[derive(Clone, Serialize, TS)]
pub(crate) struct PinkOutcomeReport {
    pub(crate) start: Grades,
    pub(crate) fire: Vec<PinkFireReport>,
}

/// One possible start and its chance.
#[derive(Clone, Serialize, TS)]
pub(crate) struct WeightedPinkOutcome {
    pub(crate) p: f64,
    pub(crate) outcome: PinkOutcomeReport,
}

/// A legacy pink that can fire: its aptitude, its chance, the legacy slot and its stars.
#[derive(Clone, Serialize, TS)]
pub(crate) struct PinkFireReport {
    pub(crate) aptitude: Aptitude,
    pub(crate) p: f64,
    pub(crate) slot: usize,
    pub(crate) stars: u8,
}

/// A pink type's chance on her veteran.
#[derive(Clone, Serialize, TS)]
pub(crate) struct PinkTypeChance {
    pub(crate) aptitude: Aptitude,
    pub(crate) p: f64,
}

/// A G1 of a career's schedule: the loop schedule's row (one cell, hers; `shared`: a parent won it), with the race's
/// turn and what she does.
#[derive(Clone, Serialize, TS)]
pub(crate) struct CareerScheduleRow {
    /// the loop schedule's row
    pub(crate) chart: ScheduleRow,
    /// 0..71
    pub(crate) turn: usize,
    /// the race's full name (`race`: as the chart shows it)
    pub(crate) name: String,
    /// she runs it
    pub(crate) run: bool,
    /// she skips it
    pub(crate) skipped: bool,
    /// what she does that turn instead (a race's name, or "rest"; null when she runs it)
    pub(crate) instead: Option<String>,
    /// her win chance, a fraction (null when she skips it)
    pub(crate) p: Option<f64>,
    /// a goal race of her scenario
    pub(crate) goal: bool,
    /// which of her parents won it
    pub(crate) parents: [bool; 2],
}

impl CareerReply {
    /// The roll a plan job answered (`run_carried_roll`): its `next.roll`, else why there is none (the job's refusal,
    /// its plan's, or that the plan has no roll).
    pub(crate) fn into_roll(self) -> RollReply {
        let no_roll = || RollReply::Refusal(Refusal::new("the plan has no roll"));
        match self {
            Self::Roll { roll } => roll,
            Self::Search(search) => match search.next {
                Some(PlanReply::Plan(plan)) => plan.roll.unwrap_or_else(no_roll),
                Some(PlanReply::Refusal(refusal)) => RollReply::Refusal(refusal),
                None => no_roll(),
            },
            Self::Refusal(refusal) => RollReply::Refusal(refusal),
            Self::Carried(carried) => carried.reply.into_roll(),
            _ => no_roll(),
        }
    }
    /// The career search's answer, if this is one (a job that carried a roll: its own answer's).
    pub(crate) fn search(&self) -> Option<&CareerSearch> {
        match self {
            Self::Search(search) => Some(search),
            Self::Carried(carried) => carried.reply.search(),
            _ => None,
        }
    }
    /// The search's plan (`next`), if it made one.
    pub(crate) fn plan(&self) -> Option<&crate::career::plan::report::Plan> {
        match self.search()?.next.as_ref()? {
            PlanReply::Plan(plan) => Some(plan),
            PlanReply::Refusal(_) => None,
        }
    }
    /// The answer as one line of JSON, at most `limit` characters (for a message).
    pub(crate) fn excerpt(&self, limit: usize) -> String {
        serde_json::to_string(self).unwrap_or_default().chars().take(limit).collect()
    }
}
