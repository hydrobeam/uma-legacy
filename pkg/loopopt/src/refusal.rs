//! A refusal: the answer to a job the solver did not do, and why. Every command's reply is its report or a refusal.
use serde::Serialize;
use ts_rs::TS;

/// Why a job was not done (a bad request, a job outside the search's limits, nothing to search). Only a refusal
/// carries `error`, so `'error' in reply` tells it from a report.
#[derive(Clone, Debug, Serialize, TS)]
pub(crate) struct Refusal {
    pub(crate) error: String,
}

impl Refusal {
    pub(crate) fn new(error: impl Into<String>) -> Self {
        Self { error: error.into() }
    }
}
