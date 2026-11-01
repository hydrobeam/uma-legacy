//! What the worker keeps a career job's work under: each key is the content (`crate::key::content_key`) of the parts
//! of the job that work reads, so a later job with the same parts finds it, whatever else it changes.
//! - `setup_key`: the setup (targets, roster, deck space, events), what one job's search proves that holds for the next
//!   (the deck memo, fully checked decks, class flags, a kept plan's scope).
//! - `search_key`, `borrowing_search_key`: a main search, by the whole job but the plan's own fields.
//! - `plan_key`: a plan's roll and check states, by the whole job but its follow-ups and the page's flags.
//!
//! Every field of a career job is placed in or out of each key below (`CareerJob` is destructured whole, so a new field
//! fails to compile until it is placed): a field a piece of work reads but its key leaves out would let a job reuse work
//! done for another.
use crate::career::plan::PlanJob;
use crate::career::sim::SimJob;
use crate::career::{BorrowedParent, CareerJob, Legacy};
use crate::deck::DeckJob;
use crate::key::content_key;
use crate::request::CareerRequest;
use crate::skills::SkillJob;
use serde::Serialize;

/// A key's content: the job's setup, and the career fields the key reads (`None`: out of it).
#[derive(Serialize)]
struct JobKey<'job> {
    skills: &'job SkillJob,
    deck: &'job DeckJob,
    simulate: &'job Option<SimJob>,
    given: &'job Option<String>,
    places: Option<Places<'job>>,
    compare: Option<bool>,
    borrowed_parent: Option<BorrowedParent>,
    next: Option<PlanJob>,
}

/// The pool and places a search runs on.
#[derive(Serialize)]
struct Places<'job> {
    legacies: &'job [Legacy],
    trainees: &'job [u32],
    top: usize,
    fixed_parents: &'job [usize],
    per_trainee: bool,
}

/// Which parts of the career job a key reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyKind {
    /// the setup: targets, roster, deck space and events, a simulation and a given career; not the pool, the places
    /// or the page's flags (`compare`, the plan's fields, the borrowing rule, a rule on the pairs as `fix` is)
    Setup,
    /// a main search: all but the plan's own fields (`next`, `no_reports`); with the borrowing rule or without
    Search { borrowing_rule: bool },
    /// a plan's roll and check states: all but `compare`, `no_reports` and the follow-ups asked of the plan
    /// (`PlanJob::plan_only`)
    Plan,
}

impl CareerRequest {
    fn key(&self, kind: KeyKind) -> u64 {
        let Self { skills, deck, career, roll: _ } = self;
        let CareerJob {
            legacies,
            trainees,
            top,
            fixed_parents,
            per_trainee,
            simulate,
            next,
            compare,
            no_reports: _,
            borrowed_parent,
            given,
        } = career;
        let places = (kind != KeyKind::Setup).then_some(Places {
            legacies,
            trainees,
            top: *top,
            fixed_parents,
            per_trainee: *per_trainee,
        });
        let borrowing_rule = matches!(kind, KeyKind::Search { borrowing_rule: true } | KeyKind::Plan);
        content_key(&JobKey {
            skills,
            deck,
            simulate,
            given,
            places,
            compare: matches!(kind, KeyKind::Search { .. }).then_some(*compare),
            borrowed_parent: borrowing_rule.then_some(*borrowed_parent),
            next: (kind == KeyKind::Plan).then(|| next.as_ref().map(PlanJob::plan_only)).flatten(),
        })
    }
    /// The key of what one job's search proves that holds for the next on the same setup (`KeptState`'s deck memo,
    /// fully checked decks and class flags; `plan_scope`).
    pub(crate) fn setup_key(&self) -> u64 {
        self.key(KeyKind::Setup)
    }
    /// The key a main search is kept under (`KeptState::searches`). The page sends a career job, then the same job with
    /// `next`: the plan job reuses that job's main search (a search is deterministic for its job, so the result is the
    /// one it would compute).
    pub(crate) fn search_key(&self) -> u64 {
        self.key(KeyKind::Search { borrowing_rule: true })
    }
    /// The key of the main search a plan's look-ahead borrows in under `borrowed_parent` "later": the same job's under
    /// "allow" (so a plan after an "allow" run takes it as it is).
    pub(crate) fn borrowing_search_key(&self) -> u64 {
        self.key(KeyKind::Search { borrowing_rule: false })
    }
    /// The key a plan's roll state and check state are kept under (`plan::roll`, `plan::robust`): every follow-up of
    /// one plan job finds them.
    pub(crate) fn plan_key(&self) -> u64 {
        self.key(KeyKind::Plan)
    }
    /// The borrowing rule unless "allow", for the keys of a kept plan (`plan::kept::plan_scope`): under "later" or
    /// "never" a plan keeps other looped careers, under "never" other best careers too.
    pub(crate) const fn restricted_borrowing(&self) -> Option<BorrowedParent> {
        match self.career.borrowed_parent {
            BorrowedParent::Allow => None,
            rule => Some(rule),
        }
    }
    /// The job's veterans and trainee filter, for the keys of a kept plan's caps (`plan::caps::CapPool`).
    pub(crate) fn veterans_and_trainees(&self) -> (&[Legacy], &[u32]) {
        (&self.career.legacies, &self.career.trainees)
    }
}
