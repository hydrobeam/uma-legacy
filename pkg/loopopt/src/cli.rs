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
    let fingerprint = pb_fingerprint(&raw);
    let mut data = load_str(json);
    data.pair_ceilings = None;
    let n = data.umas.len();
    let roster: Vec<UmaIndex> = data.umas.indices().collect();
    let ceilings = pair_ceilings(&data, &roster);
    let rows: Vec<String> = (0..n)
        .map(|a| (a + 1..n).map(|b| format!("{:016x}", ceilings[UmaIndex(a)][UmaIndex(b)].to_bits())).collect())
        .collect();
    serde_json::to_value(PbRaw { fingerprint, rows }).unwrap()
}

/// Emit the enabled bound diagnostics on normal return or panic unwind, including `pbtable`.
/// `process::exit` deliberately bypasses this guard, as it does all destructors.
pub(crate) struct CliBoundReport;

impl Drop for CliBoundReport {
    fn drop(&mut self) {
        let checks = crate::checks::get();
        if checks.sb_check {
            crate::checks::SBCHK.with(|counts| {
                let counts = counts.borrow();
                eprintln!(
                    "SBCHK decks {} max value/exact {:.6} min {:.9} below {} | leaves {} node violations {}",
                    counts.decks, counts.max_ratio, counts.min_ratio, counts.below, counts.leaves, counts.violations
                );
            });
        }
        if checks.sbound_report || checks.sb_check {
            crate::checks::SBN.with(|counts| {
                let counts = counts.borrow();
                eprintln!(
                    "SBN cfgs cut {} bound evals {} nodes cut {}",
                    counts.configurations_cut, counts.evaluations, counts.nodes_cut
                );
            });
        }
        if checks.rb_report {
            crate::checks::RBCHK.with(|counts| {
                let counts = counts.borrow();
                eprintln!(
                    "RBCHK leaves {} tests {} violations {} max value/bound {:.9}",
                    counts.leaves, counts.tests, counts.violations, counts.max_ratio
                );
            });
        }
        if checks.cf_report {
            crate::checks::CFCHK.with(|counts| {
                let counts = counts.borrow();
                eprintln!("CFCHK searches {} capped below cap-free {} cap-free below capped {} unproven {} | leaves {} violations {} | scorings {} violations {} max exact/bound {:.9}", counts.searches, counts.capped_below, counts.cap_free_below, counts.unproven, counts.leaves, counts.leaf_violations, counts.scorings, counts.scoring_violations, counts.max_ratio);
            });
        }
    }
}

/// Enrich the outer job first, then a carried roll's plan job, using the outer job's event file.
/// The carried job must match the enriched job that originally populated the worker's kept state.
pub(crate) fn apply_cli_events(job: &mut serde_json::Value, skills: &skills::SkillsRaw) {
    if let Some(path) = job["events_file"].as_str() {
        let event_cache: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        crate::deck::sources::apply_events_db(job, skills, &event_cache);
        if job["roll"]["job"].is_object() {
            crate::deck::sources::apply_events_db(&mut job["roll"]["job"], skills, &event_cache);
        }
    }
}

/// Restore the career worker's kept work before any jobs run, as the page does across reloads.
pub(crate) fn restore_cli_career_state(data: &Data, skills: &skills::SkillsRaw, cards: &crate::deck::CardsRaw) {
    if let Some(path) = &crate::checks::get().state_in {
        let state: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
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
    if let Some(path) = &crate::checks::get().state_out {
        let out = crate::career::run_career(data, skills, cards, &serde_json::json!({"cmd": "state_out"}));
        std::fs::write(path, out.to_string()).unwrap();
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
