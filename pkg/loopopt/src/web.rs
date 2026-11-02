//! WebAssembly entry point, through wasm-bindgen's generated glue (`./build.sh wasm` writes it to
//! webapp/src/gen/loopopt.js, with its types in loopopt.d.ts). The page's worker makes one `Solver` from the era's data,
//! then sends it jobs; each answer is the reply's JSON text. In the native tests the same struct answers through
//! `open` and `reply` (`fixtures`, below, runs the flows' captured jobs through it, as the worker would):
//!   `new Solver(init)`: {"data":<data.json>,"skills":<web_skills.json>,"cards":<web_cards.json>,"events":<card_events.json>}
//!     (throws, saying why, on a request that does not parse)
//!   `solver.call(job)`: a `Request`: {"cmd":"deck","skills":SkillJob,"deck":DeckJob}  (needs "cards" and "events")
//!           {"cmd":"career","skills":...,"deck":...,"career":CareerJob}  (same)
//!           {"cmd":"state_out"}, {"cmd":"state_in","state":...}  the career planner's kept work out and back in
//!           (page reloads)
use crate::career::reply::CareerReply;
use crate::career::state::{KeptState, state_in, state_out};
use crate::data::{Data, DataRaw, load_raw};
use crate::deck::CardsRaw;
use crate::deck::sources::{CardEventCache, EventLists, event_lists};
use crate::refusal::Refusal;
use crate::request::Request;
use crate::skills::SkillsRaw;
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// The worker's solver: the era's data its jobs run on, the career planner's work kept between them, and the buffer
/// each reply is written into.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Solver {
    data: Data,
    skills: SkillsRaw,
    /// the card data the deck and career jobs need; none when the init request carried none that parses
    deck: Option<DeckData>,
    kept: KeptState,
    /// the last reply's JSON, its allocation reused by the next (`write_json`)
    out: String,
}

/// The card table, and the job lists its event cache adds to each job (`events` at init).
struct DeckData {
    cards: CardsRaw,
    events: EventLists,
}

/// The init request, parsed straight into the solver's input types (no intermediate `Value` tree for the large data
/// table: d97c26d measured init 42 -> 13 ms on Global). `cards` and `events` stay `Value`s: card data that does not parse
/// (a card table, or an event cache, `CardEventCache`) leaves the solver without card data, so an initial request can
/// still load the schedule data without it.
#[derive(Deserialize)]
struct InitReq {
    data: DataRaw,
    skills: SkillsRaw,
    #[serde(default)]
    cards: serde_json::Value,
    #[serde(default)]
    events: serde_json::Value,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Solver {
    /// The era's data, from the init request's JSON text, kept for the jobs.
    #[wasm_bindgen(constructor)]
    pub fn new(init: &str) -> Result<Self, JsError> {
        console_error_panic_hook::set_once();
        Self::open(init).map_err(|e| JsError::new(&e))
    }

    /// One job (a `Request`'s JSON text) run on the data and the kept work; its reply's JSON text.
    #[wasm_bindgen(unchecked_return_type = "string")]
    pub fn call(&mut self, job: &str) -> JsValue {
        JsValue::from_str(self.reply(job))
    }
}

impl Solver {
    /// The era's data, from the init request's JSON text, kept for the jobs; why the request does not parse otherwise.
    pub fn open(init: &str) -> Result<Self, String> {
        let request = serde_json::from_str::<InitReq>(init).map_err(|e| format!("bad init request: {e}"))?;
        // (no event cache: no event lists)
        let event_cache = Option::<CardEventCache>::deserialize(&request.events);
        let deck = match (CardsRaw::deserialize(&request.cards), event_cache) {
            (Ok(cards), Ok(event_cache)) => {
                Some(DeckData { cards, events: event_lists(&request.skills, event_cache.unwrap_or_default()) })
            }
            _ => None,
        };
        Ok(Self {
            data: load_raw(request.data),
            skills: request.skills,
            deck,
            kept: KeptState::default(),
            out: String::new(),
        })
    }

    /// One job (a `Request`'s JSON text) run on the data and the kept work; its reply's JSON text.
    pub fn reply(&mut self, job: &str) -> &str {
        self.answer(job);
        &self.out
    }

    /// `reply`'s work: the job's reply written into `out` (a reply may borrow the data, so it is written where the data
    /// is borrowed).
    fn answer(&mut self, job: &str) {
        let Self { data, skills, deck, kept, out } = self;
        let request = match serde_json::from_str::<Request>(job) {
            Ok(request) => request,
            Err(e) => return write_json(out, &Refusal::new(format!("bad request: {e}"))),
        };
        match request {
            Request::Deck(mut job) => {
                let Some(DeckData { cards, events }) = deck else {
                    return write_json(out, &Refusal::new("no card data"));
                };
                events.add_to(&mut job.deck);
                write_json(out, &crate::loop_planner::job::run_deck(data, skills, cards, &job));
            }
            // (a carried roll's plan job takes the events too: its kept roll state is found by the plan job as it ran)
            Request::Career(mut job) => {
                let Some(DeckData { cards, events }) = deck else {
                    return write_json(out, &Refusal::new("no card data"));
                };
                events.add_to(&mut job.deck);
                if let Some(roll) = &mut job.roll {
                    events.add_to(&mut roll.job.deck);
                }
                write_json(out, &crate::career::run_career_in(data, skills, cards, &job, kept));
            }
            Request::StateOut => write_json(out, &CareerReply::StateOut(state_out(kept))),
            Request::StateIn { state } => write_json(out, &state_in(kept, &state)),
        }
    }
}

/// `value`'s JSON written into `out`, in place of what it held. serde_json writes to an `io::Write`, which `String` is
/// not, so the buffer goes in as its bytes (a `Vec<u8>`, which is one) and comes back as text, checked as UTF-8 once.
/// Reusing it keeps one allocation across calls: a reply neither allocates afresh nor copies itself each time the
/// buffer grows. The field stays a `String` so its type says it holds UTF-8.
fn write_json(out: &mut String, value: &impl Serialize) {
    let mut bytes = std::mem::take(out).into_bytes();
    bytes.clear();
    serde_json::to_writer(&mut bytes, value).expect("the solver's replies serialise");
    *out = String::from_utf8(bytes).expect("serde_json writes UTF-8");
}

#[cfg(test)]
mod fixtures {
    // The browser flows' canned solver replies (webapp/test/fixtures/solver/, docs/BUILD.md, *Tests*): for each
    // scenario, one job of each kind its flows send (`<scenario>.jobs.json`: a deck search, a career search and plan,
    // a robustness-check step, a comparison and its plan, a roll, the kept work out and in), run in that order through
    // one worker's `Solver`, so a check, a comparison and a roll find the plan kept, and a `state_in` restores what the
    // `state_out` before it gave, as the page stores it; a deck job runs twice, as the worker runs it (src/worker.ts:
    // its fast pass, then the exact one). The replies are an insta binary snapshot beside the jobs
    // (`<scenario>.snap.json`, one kind a line), which the flows' stand-in for the worker answers each job of that kind
    // with. A reply's wall clocks (`timing_s`, `ms`, `*_ms`) are written as 0, so it is the same on every run and every
    // machine. `INSTA_UPDATE=always` rewrites them (./build.sh fixtures); one that changed otherwise fails.
    use super::Solver;
    use serde_json::{Map, Value};
    use std::path::{Path, PathBuf};

    /// The fixtures' folder.
    const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../webapp/test/fixtures/solver");

    /// The solver's data for the page's era ("global", "gm"), as the page build writes it (vite/pagedata.mjs `era`).
    fn era_data(era: &str) -> String {
        let dir = match era {
            "global" => "global",
            "gm" => "jp",
            other => panic!("an era of the page: {other}"),
        };
        let read = |name: &str| {
            let path = format!("{}/data/{dir}/{name}", env!("CARGO_MANIFEST_DIR"));
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
        };
        format!(
            r#"{{"data":{},"skills":{},"cards":{},"events":{}}}"#,
            read("data.json"),
            read("web_skills.json"),
            read("web_cards.json"),
            read("card_events.json")
        )
    }

    /// `value` with every wall clock (`timing_s`, `ms`, a key ending in `_ms`) at 0.
    fn clocks_zeroed(value: Value) -> Value {
        match value {
            Value::Object(fields) => Value::Object(
                fields
                    .into_iter()
                    .map(|(key, field)| {
                        let clock = (key == "timing_s" || key == "ms" || key.ends_with("_ms")) && field.is_number();
                        (key, if clock { Value::from(0) } else { clocks_zeroed(field) })
                    })
                    .collect(),
            ),
            Value::Array(items) => Value::Array(items.into_iter().map(clocks_zeroed).collect()),
            other => other,
        }
    }

    /// One job's answer, as the worker posts it: a deck job's fast pass (`interim`), then its `result`.
    fn answer(solver: &mut Solver, job: &Value) -> Value {
        let reply = |solver: &mut Solver, job: &Value| -> Value {
            let text = solver.reply(&job.to_string());
            clocks_zeroed(serde_json::from_str(text).expect("a reply is JSON"))
        };
        let mut answer = Map::new();
        if job["cmd"] == "deck" && job["deck"]["exact"] != false {
            let mut quick = job.clone();
            quick["deck"]["exact"] = Value::Bool(false);
            let interim = reply(solver, &quick);
            if interim["kind"] == "refusal" {
                answer.insert("result".to_owned(), interim.clone());
                answer.insert("interim".to_owned(), interim);
                return Value::Object(answer);
            }
            answer.insert("interim".to_owned(), interim);
        }
        answer.insert("result".to_owned(), reply(solver, job));
        Value::Object(answer)
    }

    /// The scenarios with jobs (`<scenario>.jobs.json`), by name, sorted.
    fn scenarios() -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(DIR)
            .expect("webapp/test/fixtures/solver reads")
            .map(|entry| entry.expect("an entry").file_name().into_string().expect("a UTF-8 name"))
            .filter_map(|name| name.strip_suffix(".jobs.json").map(str::to_owned))
            .collect();
        names.sort();
        names
    }

    /// Each scenario's canned replies, one per job kind: the solver's answers now, as its snapshot holds them.
    #[test]
    fn replies() {
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_path(PathBuf::from(DIR));
        settings.set_prepend_module_to_snapshot(false);
        settings.set_omit_expression(true);
        settings.set_description("the scenario's canned replies: ./build.sh fixtures writes them from its jobs");
        let scenarios = scenarios();
        assert!(!scenarios.is_empty(), "scenarios with jobs in {DIR}");
        settings.bind(|| {
            for scenario in &scenarios {
                let path = Path::new(DIR).join(format!("{scenario}.jobs.json"));
                let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                let fixture: Value = serde_json::from_str(&text).expect("a jobs file is JSON");
                let mut solver =
                    Solver::open(&era_data(fixture["era"].as_str().expect("the era"))).expect("the era's data loads");
                // (the kept work the last `state_out` gave: what the page stores and a `state_in` sends back)
                let mut stored = Value::Null;
                let mut lines = Vec::new();
                for entry in fixture["jobs"].as_array().expect("the jobs, by kind") {
                    let kind = entry["kind"].as_str().expect("a job's kind");
                    let mut job = entry["job"].clone();
                    if job["cmd"] == "state_in" {
                        job["state"] = stored.clone();
                    }
                    let reply = answer(&mut solver, &job);
                    assert_ne!(reply["result"]["kind"], "refusal", "{scenario} {kind}: {}", reply["result"]);
                    if reply["result"]["kind"] == "state_out" {
                        stored = reply["result"]["state"].clone();
                    }
                    lines.push(format!("{}:{reply}", Value::from(kind)));
                }
                let bytes = format!("{{\n{}\n}}\n", lines.join(",\n")).into_bytes();
                insta::assert_binary_snapshot!(&format!("{scenario}.json"), bytes);
            }
        });
    }
}
