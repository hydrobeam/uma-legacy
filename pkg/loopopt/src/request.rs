//! What the page sends the solver's worker (`call`, web.rs): a job by its command; and what `init` answers. The solver reads each job as its
//! parts (`SkillJob`, `DeckJob`, `CareerJob`) from the request after adding the event lists it keeps (`sources`);
//! `Command` is what `handle` dispatches on, and `Request` is the whole job, for the page's types.
use crate::refusal::Refusal;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The command a request names (`cmd`); its other fields are the job's. An unknown command does not parse.
#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
#[cfg_attr(not(any(target_arch = "wasm32", test)), expect(dead_code, reason = "the wasm's dispatch (web.rs) reads it"))]
pub(crate) enum Command {
    Deck,
    Career,
    StateOut,
    StateIn,
}

/// A job by its command (the page's `SolverJob`). For the page's types: `handle` reads the command (`Command`), then
/// each command its parts from the request. Not every field a job may carry is here: a career job's `roll` (a roll of a
/// kept plan carried by another job, which the page does not send) and the event lists the solver adds (`sources`).
#[derive(Deserialize, TS)]
#[serde(tag = "cmd", rename_all = "snake_case")]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the page's types of a job (`ts_gen`), checked against the jobs the solver runs (tests)"
    )
)]
pub(crate) enum Request {
    /// a loop and deck search
    Deck(DeckRequest),
    /// a career search, with the plan (`next`) when asked
    Career(CareerRequest),
    /// the career planner's kept work out (the page stores it)
    StateOut,
    /// that work back in after a reload
    StateIn {
        /// what `state_out` gave, as the page stored it
        #[ts(type = "unknown")]
        state: serde_json::Value,
    },
}

/// A deck job: the targets and roster (`SkillJob`) and the deck search's settings (`DeckJob`).
#[derive(Deserialize, TS)]
#[cfg_attr(not(test), expect(dead_code, reason = "the page's types of a job (`ts_gen`)"))]
pub(crate) struct DeckRequest {
    #[serde(flatten)]
    pub(crate) skills: crate::skills::SkillJob,
    #[serde(flatten)]
    pub(crate) deck: crate::deck::DeckJob,
}

/// A career job: a deck job's parts and the career search's (`CareerJob`: the veterans, places, plan or simulation).
#[derive(Deserialize, TS)]
#[cfg_attr(not(test), expect(dead_code, reason = "the page's types of a job (`ts_gen`)"))]
pub(crate) struct CareerRequest {
    #[serde(flatten)]
    pub(crate) skills: crate::skills::SkillJob,
    #[serde(flatten)]
    pub(crate) deck: crate::deck::DeckJob,
    #[serde(flatten)]
    pub(crate) career: crate::career::CareerJob,
}

/// `init`'s answer: the data loaded, or why not.
#[derive(Serialize, TS)]
#[serde(untagged)]
#[cfg_attr(not(target_arch = "wasm32"), expect(dead_code, reason = "the wasm's `init` (web.rs) answers it"))]
pub(crate) enum InitReply {
    Loaded(Loaded),
    Refusal(Refusal),
}

/// The era's data loaded: its uma count.
#[derive(Serialize, TS)]
#[cfg_attr(not(any(target_arch = "wasm32", test)), expect(dead_code, reason = "the wasm's `init` (web.rs) answers it"))]
pub(crate) struct Loaded {
    /// always true
    pub(crate) ok: bool,
    pub(crate) umas: usize,
}

#[cfg(test)]
mod tests {
    use super::{Command, Request};
    use std::fs;

    /// Every job the validation runs (val/jobs: deck and career jobs; val/plan: plan jobs), sent as the page sends a
    /// job (`cmd` for its file's `mode`), parses as a `Request` of its command: the page's type of a job is what the
    /// solver reads. An unknown command parses as none.
    #[test]
    fn the_validation_jobs_parse_as_requests() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../val");
        let mut parsed = (0, 0);
        for folder in ["jobs", "plan"] {
            for entry in fs::read_dir(format!("{root}/{folder}")).unwrap().flatten() {
                let path = entry.path();
                if path.extension().is_none_or(|extension| extension != "json") {
                    continue;
                }
                let mut job: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
                let Some(mode) = job.get("mode").and_then(serde_json::Value::as_str).map(str::to_owned) else {
                    continue;
                };
                job["cmd"] = serde_json::json!(mode);
                match serde_json::from_value::<Request>(job) {
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
        assert!(serde_json::from_value::<Command>(serde_json::json!({"cmd": "dock"})).is_err(), "an unknown command");
        assert!(matches!(serde_json::from_value(serde_json::json!({"cmd": "state_out"})), Ok(Command::StateOut)));
        let state_in = serde_json::from_value(serde_json::json!({"cmd": "state_in", "state": {"v": 1}}));
        assert!(matches!(state_in, Ok(Request::StateIn { state }) if state["v"] == 1), "a stored state, as sent");
    }
}
