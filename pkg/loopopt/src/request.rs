//! What the solver is asked: a job by its command (`Request`, what the page's worker sends `call`, web.rs), each job
//! its parts as nested records (`SkillJob`, `DeckJob`, `CareerJob`); the native command line's job files (`JobFile`:
//! the job and the data files it names). Every job record refuses a field it does not know.
use crate::career::CareerJob;
use crate::career::state::StoredState;
use crate::deck::DeckJob;
use crate::skills::SkillJob;
use serde::Deserialize;
use ts_rs::TS;

/// A job by its command (`cmd`).
#[derive(Deserialize, TS)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub(crate) enum Request {
    /// a loop and deck search
    Deck(Box<DeckRequest>),
    /// a career search, with the plan (`next`) or the simulator (`simulate`) when asked
    Career(Box<CareerRequest>),
    /// the career planner's kept work out (the page stores it)
    StateOut,
    /// that work back in after a reload
    StateIn {
        /// what `state_out` gave, as the page stored it
        #[cfg_attr(
            not(any(target_arch = "wasm32", test)),
            expect(dead_code, reason = "the wasm's dispatch (web.rs) reads it")
        )]
        state: Box<StoredState>,
    },
}

/// A deck job: the targets and roster (`SkillJob`) and the deck search's settings (`DeckJob`).
#[derive(Deserialize, Clone, TS)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeckRequest {
    pub(crate) skills: SkillJob,
    pub(crate) deck: DeckJob,
}

/// A career job: a deck job's parts and the career search's (`CareerJob`: the veterans, places, plan or simulation),
/// and a roll of a kept plan it carries.
#[derive(Deserialize, Clone, TS)]
#[serde(deny_unknown_fields)]
pub(crate) struct CareerRequest {
    pub(crate) skills: SkillJob,
    pub(crate) deck: DeckJob,
    pub(crate) career: CareerJob,
    /// a roll of a kept plan, answered beside this job's own answer (`career::run::run_carried_roll`); the page sends
    /// the plan job with `next.roll` instead
    #[serde(default)]
    #[ts(optional)]
    pub(crate) roll: Option<Box<CarriedRollJob>>,
}

/// A roll carried by another job: the plan job and her stars per job target (0 where it did not land).
#[derive(Deserialize, Clone, TS)]
#[serde(deny_unknown_fields)]
pub(crate) struct CarriedRollJob {
    pub(crate) job: CareerRequest,
    pub(crate) stars: Vec<u8>,
}

/// A job file of the native command line: the job, and the data files it reads beside the era's data.json (paths
/// relative to the era's data folder, where the solver runs).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JobFile {
    pub(crate) files: JobFiles,
    pub(crate) job: Request,
}

/// The data files a job file names: the skills and card tables (the first job's serve every job of a run) and the card
/// events added to its job (each job's own).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JobFiles {
    pub(crate) skills: std::path::PathBuf,
    pub(crate) cards: std::path::PathBuf,
    #[serde(default)]
    pub(crate) events: Option<std::path::PathBuf>,
}

/// Jobs for the crate's tests, read from the validation's job files.
#[cfg(test)]
pub(crate) mod testing {
    use super::CareerRequest;
    use serde_json::Value;

    /// The career job of val's job file `path` (under `val/`), adjusted as JSON by `adjust` before it is read, with the
    /// card events of its file (`files.events`, in `era`'s data folder) added as the command line adds them.
    pub(crate) fn val_career(
        path: &str,
        era: &str,
        skills: &crate::skills::SkillsRaw,
        adjust: impl FnOnce(&mut Value),
    ) -> CareerRequest {
        let read = |path: String| std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let file: Value =
            serde_json::from_str(&read(format!("{}/../../val/{path}", env!("CARGO_MANIFEST_DIR")))).unwrap();
        let mut job = file["job"].clone();
        job.as_object_mut().unwrap().remove("cmd");
        adjust(&mut job);
        let mut job: CareerRequest = serde_json::from_value(job).unwrap_or_else(|e| panic!("{path}: {e}"));
        if let Some(events) = file["files"]["events"].as_str() {
            let cache = serde_json::from_str(&read(format!("{}/data/{era}/{events}", env!("CARGO_MANIFEST_DIR"))));
            crate::deck::sources::event_lists(skills, cache.unwrap()).add_to(&mut job.deck);
        }
        job
    }
}

#[cfg(test)]
mod tests {
    use super::{JobFile, Request};
    use std::fs;

    /// Every job file the validation runs (val/jobs: deck and career jobs; val/plan: plan jobs) parses as a deck or
    /// career job: the job types are what the files and the page send. An unknown command or field parses as none.
    #[test]
    fn the_validation_jobs_parse_as_requests() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../val");
        let mut parsed = (0, 0);
        for folder in ["jobs", "plan"] {
            for entry in fs::read_dir(format!("{root}/{folder}")).unwrap().flatten() {
                let path = entry.path();
                let text = fs::read_to_string(&path).unwrap();
                // (val/plan's job lists are lists, not jobs)
                if path.extension().is_none_or(|extension| extension != "json") || text.starts_with('[') {
                    continue;
                }
                match serde_json::from_str::<JobFile>(&text).map(|file| file.job) {
                    Ok(Request::Deck(deck)) => {
                        assert!(!deck.skills.targets.is_empty() && deck.deck.deck_size > 0, "{}", path.display());
                        parsed.0 += 1;
                    }
                    Ok(Request::Career(career)) => {
                        assert!(!career.skills.targets.is_empty() && career.deck.deck_size > 0, "{}", path.display());
                        assert!(!career.career.legacies.is_empty(), "{}", path.display());
                        parsed.1 += 1;
                    }
                    Ok(_) => panic!("{}: not a deck or career job", path.display()),
                    Err(error) => panic!("{}: {error}", path.display()),
                }
            }
        }
        assert!(parsed.0 >= 20 && parsed.1 >= 15, "deck and career jobs parsed: {parsed:?}");
        let request = |text: &str| serde_json::from_str::<Request>(text);
        assert!(request(r#"{"cmd": "dock"}"#).is_err(), "an unknown command");
        assert!(matches!(request(r#"{"cmd": "state_out"}"#), Ok(Request::StateOut)));
        let deck = r#"{"cmd": "deck", "skills": {"targets": [{"group": 20159}]}, "deck": {}"#;
        assert!(request(&format!("{deck}}}")).is_ok(), "a deck job");
        assert!(request(&format!("{deck}, \"star_dist\": [0.5, 0.45, 0.05]}}")).is_err(), "an unknown field");
    }
}
