//! CLI job loading, event enrichment, kept-state restoration and result dispatch.
//! Solver algorithms and the wasm interface live in their own modules.
use crate::data::{Data, DataRaw, PbRaw, load_str, pb_fingerprint};
use crate::optimize::pair_ceilings;
use crate::uma::UmaIndex;
use crate::{Instant, skills};
use serde::Deserialize;

/// A job file's mode ("deck" or "career"); the rest of the file is the job itself.
#[derive(Deserialize)]
pub(crate) struct Jobs {
    #[serde(default)]
    pub(crate) mode: String,
}

/// The full pairwise-ceiling table of a data.json, as stored in its "pb" field (computed fresh, ignoring any table).
pub(crate) fn pb_table_json(json: &str) -> serde_json::Value {
    let raw: DataRaw = serde_json::from_str(json).unwrap();
    let fp = pb_fingerprint(&raw);
    let mut d = load_str(json);
    d.pbt = None;
    let n = d.umas.len();
    let all: Vec<UmaIndex> = d.umas.indices().collect();
    let pb = pair_ceilings(&d, &all);
    let rows: Vec<String> = (0..n)
        .map(|a| (a + 1..n).map(|b| format!("{:016x}", pb[UmaIndex(a)][UmaIndex(b)].to_bits())).collect())
        .collect();
    serde_json::to_value(PbRaw { fp, rows }).unwrap()
}

/// Emit the enabled bound diagnostics on normal return or panic unwind, including `pbtable`.
/// `process::exit` deliberately bypasses this guard, as it does all destructors.
pub(crate) struct CliBoundReport;

impl Drop for CliBoundReport {
    fn drop(&mut self) {
        if std::env::var("SB_CHECK").is_ok() {
            crate::deck::joint::SBCHK.with(|c| {
                let c = c.borrow();
                eprintln!(
                    "SBCHK decks {} max value/exact {:.6} min {:.9} below {} | leaves {} node violations {}",
                    c.0, c.1, c.2, c.3, c.4, c.5
                );
            });
        }
        if std::env::var("SBOUND").is_ok() || std::env::var("SB_CHECK").is_ok() {
            crate::deck::joint::SBN.with(|c| {
                let c = c.borrow();
                eprintln!("SBN cfgs cut {} bound evals {} nodes cut {}", c.0, c.1, c.2);
            });
        }
        if std::env::var("RB_CHECK").is_ok() {
            crate::deck::enumerate::bounds::RBCHK.with(|c| {
                let c = c.borrow();
                eprintln!("RBCHK leaves {} tests {} violations {} max value/bound {:.9}", c.0, c.1, c.2, c.3);
            });
        }
        if std::env::var("CF_CHECK").is_ok() {
            crate::deck::joint::CFCHK.with(|c| {
                let c = c.borrow();
                eprintln!("CFCHK searches {} capped below cap-free {} cap-free below capped {} unproven {} | leaves {} violations {} | scorings {} violations {} max exact/bound {:.9}", c.0, c.1, c.2, c.3, c.4, c.5, c.6, c.7, c.8);
            });
        }
    }
}

/// Enrich the outer job first, then a carried roll's plan job, using the outer job's event file.
/// The carried job must match the enriched job that originally populated the worker's kept state.
pub(crate) fn apply_cli_events(job: &mut serde_json::Value, skills: &skills::SkillsRaw) {
    if let Some(path) = job["events_file"].as_str() {
        let db: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        crate::deck::sources::apply_events_db(job, skills, &db);
        if job["roll"]["job"].is_object() {
            crate::deck::sources::apply_events_db(&mut job["roll"]["job"], skills, &db);
        }
    }
}

/// Restore the career worker's kept work before any jobs run, as the page does across reloads.
pub(crate) fn restore_cli_career_state(data: &Data, skills: &skills::SkillsRaw, cards: &crate::deck::CardsRaw) {
    if let Ok(path) = std::env::var("LOOPOPT_STATE_IN") {
        let state: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let out = crate::career::run_career(
            data,
            skills,
            cards,
            &serde_json::json!({"cmd": "state_in", "state": state["state"]}),
        );
        eprintln!("state_in: {out}");
    }
}

/// Save the career worker's kept work once, after all job outputs have been printed.
pub(crate) fn save_cli_career_state(data: &Data, skills: &skills::SkillsRaw, cards: &crate::deck::CardsRaw) {
    if let Ok(path) = std::env::var("LOOPOPT_STATE_OUT") {
        let out = crate::career::run_career(data, skills, cards, &serde_json::json!({"cmd": "state_out"}));
        std::fs::write(&path, out.to_string()).unwrap();
    }
}

/// Time solver dispatch only: file loading, event enrichment and output serialization stay outside.
/// `mode` has already been validated by `cli_main`; a batch always uses its first job's career mode.
pub(crate) fn print_cli_job(
    mode: &str,
    data: &Data,
    skills: &skills::SkillsRaw,
    cards: &crate::deck::CardsRaw,
    job: &serde_json::Value,
) {
    let started = Instant::now();
    let mut out = if mode == "career" {
        crate::career::run_career(data, skills, cards, job)
    } else {
        crate::deck::frontier::run_deck(data, skills, cards, job)
    };
    out["timing_s"] = serde_json::json!(started.elapsed().as_secs_f64());
    println!("{out}");
}
