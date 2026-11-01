//! WebAssembly entry point, through wasm-bindgen's generated glue (`./build.sh wasm` writes it to
//! webapp/src/gen/loopopt.js, with its types in loopopt.d.ts). The page's worker makes one `Solver` from the era's data,
//! then sends it jobs; each answer is the reply's JSON text:
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
use wasm_bindgen::prelude::*;

/// The worker's solver: the era's data its jobs run on, the career planner's work kept between them, and the buffer
/// each reply is written into.
#[wasm_bindgen]
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

#[wasm_bindgen]
impl Solver {
    /// The era's data, from the init request's JSON text, kept for the jobs.
    #[wasm_bindgen(constructor)]
    pub fn new(init: &str) -> Result<Self, JsError> {
        console_error_panic_hook::set_once();
        let request =
            serde_json::from_str::<InitReq>(init).map_err(|e| JsError::new(&format!("bad init request: {e}")))?;
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
    #[wasm_bindgen(unchecked_return_type = "string")]
    pub fn call(&mut self, job: &str) -> JsValue {
        self.answer(job);
        JsValue::from_str(&self.out)
    }
}

impl Solver {
    /// `call`'s work: the job's reply written into `out` (a reply may borrow the data, so it is written where the data
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
