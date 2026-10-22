//! The loop planner: the best four-uma loops and three-uma rotations, with their support decks, for a deck job. It is
//! built on the deck-search engine in `deck`, which the career planner (`career`) shares.
//!
//! A deck job enters at `job::run_deck`, which builds the deck frontier and hands it to `search::run_multi`.
//! - `job`: the deck job's setup and its nondominated deck frontier (`run_deck`).
//! - `search`: the loop search over structures and decks: setup (`Multi`), ceiling tables, scans, bands and reports.
//! - `schedules`: the loop's coordinated race schedules (`LoopProblem`, `optimize`) and the pairwise ceilings
//!   (`pair_ceilings`, the table `loopopt pbtable` stores); the career scheduler takes its `FILL_EPS`.
pub(crate) mod job;
pub(crate) mod schedules;
mod search;
