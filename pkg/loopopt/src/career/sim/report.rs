//! The simulator's report (`simulate` on a career job: native only; val/simval.py and the research read it), and a
//! veteran as a job records her (`LegacyRecord`).
use serde::Serialize;
use std::collections::BTreeMap;
use ts_rs::TS;

/// The `trace` policy: per run, each career a builder of the finisher's (A's) loop ran.
#[derive(Serialize, TS)]
pub(crate) struct TraceReport {
    pub(crate) trace: Vec<TraceRun>,
    /// the finisher, A
    pub(crate) trainee: String,
    pub(crate) builders: Vec<String>,
}

/// One traced run: per career [career index, A's odds before it, the builder by name, her lift chance, the odds a lift
/// takes A to, A's odds after it].
#[derive(Serialize, TS)]
pub(crate) struct TraceRun {
    pub(crate) run: usize,
    pub(crate) rows: Vec<(usize, f64, String, f64, f64, f64)>,
}

/// Every run of a policy on the job's pool, and the expected careers over them.
#[derive(Serialize, TS)]
pub(crate) struct SimReport {
    /// the job's `simulate` settings, each as the job set it or at its default
    pub(crate) simulate: crate::career::sim::SimJob,
    pub(crate) policy: crate::career::sim::Policy,
    pub(crate) runs: Vec<SimRun>,
    /// the expected careers over the runs (null: no run)
    #[serde(rename = "E")]
    pub(crate) expected: Option<RunStats>,
    pub(crate) init_s: f64,
    pub(crate) decks: usize,
    pub(crate) deck_tables: usize,
    pub(crate) refreshes: usize,
    pub(crate) schedules: usize,
    /// goal `looped` (always "looped")
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) goal: Option<&'static str>,
    /// goal `looped`: runs cut off before their goal (their tails count 0, so E is a lower bound)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) censored: Option<usize>,
}

/// A sample's mean, standard deviation, 95% interval (Student's t), extremes and size.
#[derive(Serialize, TS)]
pub(crate) struct RunStats {
    pub(crate) n: usize,
    pub(crate) mean: f64,
    pub(crate) sd: f64,
    pub(crate) ci95: [f64; 2],
    pub(crate) min: f64,
    pub(crate) max: f64,
}

/// One run of the policy: its expected careers, careers run, the pick at each change, and its work.
#[derive(Serialize, TS)]
pub(crate) struct SimRun {
    pub(crate) run: usize,
    #[serde(rename = "E")]
    pub(crate) expected: f64,
    pub(crate) careers: usize,
    pub(crate) tail: f64,
    pub(crate) final_p: f64,
    /// the last pick's key (null: no pick)
    #[serde(rename = "final")]
    pub(crate) final_pick: Option<String>,
    pub(crate) pool: usize,
    pub(crate) pruned: usize,
    pub(crate) wall_s: f64,
    pub(crate) candidates: usize,
    pub(crate) ceiling_pass: usize,
    pub(crate) tables: usize,
    pub(crate) refreshes: usize,
    pub(crate) partial_searches: usize,
    /// each pick change
    pub(crate) segs: Vec<PickChange>,
    pub(crate) checks: Vec<SurrogateCheck>,
    pub(crate) audits: Vec<SurrogateAudit>,
    /// the index policy's plans
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) index: Option<IndexRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) finishing: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) full_veterans: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) censored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) survival: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) final_legacies: Option<Vec<LegacyRecord>>,
}

/// A pick change: career index, the pick's key, trainee and odds, the pool's size and the path's survival.
#[derive(Serialize, TS)]
pub(crate) struct PickChange {
    pub(crate) t: usize,
    pub(crate) pick: String,
    pub(crate) trainee: String,
    pub(crate) p: f64,
    pub(crate) pool: usize,
    pub(crate) s: f64,
}

/// The index policy's plans in a run: their count and times, the pick's role and whether it was the best odds; with
/// `act_on` "pair_search", what each plan's action ran.
#[derive(Serialize, TS)]
pub(crate) struct IndexRecord {
    pub(crate) plans: usize,
    pub(crate) plan_s: Vec<f64>,
    /// the pick's role, per plan, counted
    pub(crate) roles: BTreeMap<String, usize>,
    /// plans whose pick was the search's best odds
    pub(crate) best_odds: usize,
    /// `act_on` "pair_search": per recommendation `source`, the plans that acted on it
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) acted_on: Option<BTreeMap<String, usize>>,
    /// the careers they named
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) careers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) check_s: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) career_job_s: Option<Vec<f64>>,
    /// the careers that ran
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) executed: Option<Vec<ExecutedCareer>>,
}

/// A career an action ran: its id (as the report it was built from names it), P(all) and landed-set chances.
#[derive(Serialize, TS)]
pub(crate) struct ExecutedCareer {
    pub(crate) career: String,
    pub(crate) p_all: f64,
    pub(crate) outcome_sets: Vec<f64>,
}

/// A surrogate audit, by `kind`: at a greedy pick change, the exact career search's pick against the surrogate's with
/// rows for sampled careers; or, on a loop, one row every `audit_every` careers.
#[derive(Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum SurrogateAudit {
    Greedy(GreedyAudit),
    Loop(LoopAudit),
    Rotation(RotationAudit),
}

/// A rotation's keep-or-replace decision at its position: the surrogate's values of the previous veteran and the new
/// one, the exact values of both, and whether the two agree on the decision.
#[derive(Serialize, TS)]
pub(crate) struct RotationAudit {
    pub(crate) t: usize,
    pub(crate) rotation: (f64, f64, f64, f64, bool),
}

/// An audit row: [kind, table value, exact value, (exact - table) / exact, landed-set distance, search finished].
pub(crate) type AuditRow = (u8, f64, f64, f64, f64, bool);

/// The greedy audit at a pick change: the exact career search's pick against the surrogate's, with audit rows.
#[derive(Serialize, TS)]
pub(crate) struct GreedyAudit {
    pub(crate) t: usize,
    pub(crate) pool: usize,
    pub(crate) sur: String,
    pub(crate) sur_p: f64,
    pub(crate) ex: Option<String>,
    pub(crate) ex_p: Option<f64>,
    pub(crate) differs: bool,
    pub(crate) proven: bool,
    pub(crate) rows: Vec<AuditRow>,
}

/// A loop's audit row, every `audit_every` careers.
#[derive(Serialize, TS)]
pub(crate) struct LoopAudit {
    pub(crate) t: usize,
    pub(crate) pool: usize,
    pub(crate) rows: [AuditRow; 1],
}

/// The surrogate against exact searches on a pool: the argmax career, and the lineages checked.
#[derive(Serialize, TS)]
pub(crate) struct SurrogateCheck {
    pub(crate) argmax: ArgmaxCheck,
    pub(crate) lineages: Vec<LineageCheck>,
}

/// A lineage checked: [ceiling, reference bound, table value, exact value, (exact - table) / exact (null where the
/// exact value is 0), bound within the margin, landed-set distance, search finished, the pick].
pub(crate) type LineageCheck = (f64, f64, f64, f64, f64, bool, f64, bool, bool);

/// The surrogate's argmax career on the pool against the exact search's.
#[derive(Serialize, TS)]
pub(crate) struct ArgmaxCheck {
    pub(crate) pool: usize,
    pub(crate) sur: String,
    pub(crate) sur_p: f64,
    pub(crate) sur_val: Option<f64>,
    pub(crate) ex: Option<String>,
    pub(crate) ex_p: Option<f64>,
    pub(crate) proven: bool,
}

/// A veteran as a job records her (`legacies`): what the simulator's pool holds at a run's end, and what it sends its
/// in-process jobs (a copy: the pool changes as the path goes on).
#[derive(Serialize, TS)]
pub(crate) struct LegacyRecord {
    pub(crate) chara: u32,
    pub(crate) borrowed: bool,
    pub(crate) sparks: Vec<(u32, u8)>,
    pub(crate) wins: Vec<usize>,
    pub(crate) pinks: Vec<(u32, u8)>,
    pub(crate) parents: Vec<AncestorRecord>,
}

/// A recorded parent of a veteran.
#[derive(Serialize, TS)]
pub(crate) struct AncestorRecord {
    pub(crate) chara: u32,
    pub(crate) sparks: Vec<(u32, u8)>,
    pub(crate) wins: Vec<usize>,
    pub(crate) pinks: Vec<(u32, u8)>,
}
