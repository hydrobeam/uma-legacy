//! The worker's saved searches: kept between jobs, and out to the page and back as one opaque string.

use crate::career::decks::DeckSearchCache;
use crate::career::plan::kept::Bests;
use crate::career::plan::roll::RollKeep;
use crate::career::pool::Pool;
use crate::career::reply::CareerReply;
use crate::career::search::{CareerSearchResult, Ref};
use crate::career::{CareerSetup, ClassFlags};
use crate::deck::space::SelectedDeck;
use crate::refusal::Refusal;
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Work retained between jobs. The worker owns it; career and planner operations borrow it explicitly. All but the
/// process-local memos leave with `state_out` and come back with `state_in`.
#[derive(Default, Serialize, Deserialize)]
pub(crate) struct KeptState {
    /// Best careers and proofs per plan, oldest first.
    pub(crate) bests: Vec<Bests>,
    /// Fully checked decks per (setup, trainees, pool) key (`CareerBuild::seen_fingerprint`) and deck class, oldest key
    /// first. Each Ref's chances and exact value depend on the setup and the trainees (their order sets the deck space's
    /// plain-pal order); they are kept per pool too, as `certified` could otherwise name another career of equal value
    /// after a job on another pool.
    pub(crate) seen: Vec<KeptSeen>,
    /// Main searches and their product proofs, oldest first, keyed by the job without `next` and `no_reports`.
    /// Reuse restores the product proofs a combined search/plan job would have left for its plan.
    pub(crate) searches: Vec<KeptSolve>,
    /// Class flags (`ClassFlags`, those with a flag set) by class content (`CareerSetup::class_key`: the setup and the
    /// class's deck problem, whatever the trainees, pool or class index): every job unions them in and keeps its own
    /// (`keep_flags`), so a breach or an assumption in any job holds for every later one with a class of that content,
    /// whichever search or plan it reuses. Never evicted (a few bytes a class), so no kept certificate outlives its
    /// flags; a kept plan also carries its own (`Bests::flags`).
    pub(crate) flags: std::collections::BTreeMap<u64, ClassFlags>,
    /// Roll scorings for recent plans, oldest first.
    pub(crate) rolls: Vec<RollKeep>,
    /// Recent plans' step caps (`plan::caps`), oldest first: what a later plan of a pool holding theirs reads.
    pub(crate) step_caps: Vec<crate::career::plan::caps::KeptCaps>,
    /// Recent plans' check states (`next.robust`), oldest first; not stored.
    #[serde(skip)]
    pub(crate) checks: Vec<crate::career::plan::CheckKeep>,
    /// Deck outcomes and corner tables per setup and trainee order; not stored.
    #[serde(skip)]
    pub(crate) decks: Vec<(u64, DeckSearchCache)>,
}

/// Replace all kept work, including roll states, when isolating a simulated index run.
pub(crate) const fn kept_swap(kept: &mut KeptState, replacement: KeptState) -> KeptState {
    std::mem::replace(kept, replacement)
}

/// The hash of a text (a legacy's identity, `Legacy::identity`).
pub(crate) fn hash_text(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// One key's kept fully checked decks (`KeptState::seen`).
#[derive(Serialize, Deserialize)]
pub(crate) struct KeptSeen {
    /// `CareerBuild::seen_fingerprint` of the jobs that made them.
    pub(crate) key: u64,
    /// Per deck class, its fully checked decks.
    pub(crate) refs: crate::uma::UmaTable<Vec<Ref>>,
}

/// One kept main search (`KeptState::searches`).
#[derive(Serialize, Deserialize)]
pub(crate) struct KeptSolve {
    /// The job's key (`CareerRun::search_key`).
    pub(crate) key: u64,
    pub(crate) solved: CareerSearchResult,
    /// What its product searches proved (`SearchState::ruled` as the search left it).
    pub(crate) ruled: Ruled,
    /// Per deck class, its flags as the search left them (`SearchState::class_flags`): unioned into a job that reuses
    /// the search.
    pub(crate) class_flags: crate::uma::UmaTable<ClassFlags>,
    /// The deck classes of its trainees, ascending: the classes its answer rests on.
    pub(crate) classes: Vec<crate::uma::UmaIndex>,
}

/// per deck class, the product searches' proofs (`SearchState::ruled`)
pub(crate) type Ruled = crate::uma::UmaTable<Vec<ProductProof>>;

/// What one product search proved (`best_deck` pass 1): every legal deck's product value at chances q (fixed factor
/// included, before the spark scale) is at most S.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ProductProof {
    /// q: the inheritance chances the search ran at, per job target.
    pub(crate) inheritance_chances: Vec<f64>,
    /// S: the best product value at q.
    pub(crate) product_bound: f64,
}

/// main searches kept (`KeptState::searches`): the page's run, and one it goes back to (a setting changed and back, a Compare or
/// Reroll plan in between)
pub(crate) const SOLVE_KEEP: usize = 3;

/// Fully checked decks kept per trainee across jobs (the most recent): the bound loop over them stays cheap
pub(crate) const SEEN_KEEP: usize = 256;

// ---------- the worker's kept work across page loads ----------
// What a worker keeps between jobs (the kept plans, `KeptState::bests`; the proofs, `KeptState::seen`; its main
// searches, roll states and step caps) leaves as `state_out` and comes back as `state_in`, so the first plan after a page
// reload reuses the work of the last session. It leaves as one string the page stores and sends back unread: the JSON
// text of [STATE_VERSION, the kept types themselves (their serde derives)]. serde_json writes each f64 as its shortest
// text and reads it back to the same bits (`float_roundtrip`), and each key as its u64 integer, exact past 2^53 (the
// page's JavaScript never parses the string), so what comes back is what left, bit for bit. Every kept float is finite:
// a step that never ends is `StepCareers::Never`, not an infinite f64, and `encode` asserts it (JSON has no infinity,
// and serde_json writes one as null). The page keys what it stores by the solver's and data's fingerprints (engine.ts
// `stateKey`), so a release starts afresh (the keys inside, `career::keys` and `bests_key`, hash with std's
// DefaultHasher, fixed within a build). A string of another version, or one that does not parse to its end, is refused
// whole (the page then drops it and plans cold); nothing else in it is checked, as it is only ever this build's own
// `state_out`. What a store says of a setup (deck entries, chances per target, trainees and veterans) is checked where it
// meets one (`CareerSetup::seen_fits`, `solved_fits`).
// (14: JSON, the step that never ends typed, in place of postcard in base64; every earlier store is refused)
pub(crate) const STATE_VERSION: u64 = 14;

/// The kept work as `state_out` gives it: the JSON text of `version` and the kept state.
pub(super) fn encode(version: u64, kept: &KeptState) -> String {
    let text = serde_json::to_string(&(version, kept)).expect("the kept types serialize to JSON");
    // (serde_json writes a non-finite f64 as null, which no f64 field reads back: a kept float that is not finite
    // fails here, before a store that cannot come back as it left reaches the page)
    debug_assert!(
        serde_json::from_str::<(u64, KeptState)>(&text).is_ok(),
        "a kept float is not finite: the store would not come back as it left"
    );
    text
}

/// The kept state of an `encode`d string; the refusal's reason for one of another version or that does not parse to
/// its end.
fn decode(state: &str) -> Result<KeptState, &'static str> {
    const UNREADABLE: &str = "state: does not parse";
    // (the version first, the rest read past: a store of another version need not hold this build's kept types)
    let (version, IgnoredAny) = serde_json::from_str::<(u64, IgnoredAny)>(state).map_err(|_| UNREADABLE)?;
    if version != STATE_VERSION {
        return Err("state: another version");
    }
    serde_json::from_str::<(u64, KeptState)>(state).map(|(_, kept)| kept).map_err(|_| UNREADABLE)
}

pub(crate) fn state_out(kept: &KeptState) -> StateOut {
    StateOut { state: encode(STATE_VERSION, kept) }
}

/// replaces the kept plans, proofs, main searches, plans' roll states and step caps with the page's stored ones and unions its
/// class flags into the worker's (all or nothing: a store of another version, or that does not parse, is refused)
pub(crate) fn state_in(kept: &mut KeptState, state: &str) -> CareerReply {
    let mut stored = match decode(state) {
        Ok(stored) => stored,
        Err(reason) => return CareerReply::Refusal(Refusal::new(reason)),
    };
    // (a roll state's scorings without the tables `derive` builds from what is stored)
    for roll in &mut stored.rolls {
        roll.derive();
    }
    let counts = StateCounts {
        bests: stored.bests.len(),
        seen: stored.seen.len(),
        solve: stored.searches.len(),
        rolls: stored.rolls.len(),
        caps: stored.step_caps.len(),
    };
    // (the stored flags unioned into the worker's: a store never clears a flag)
    for (key, live) in std::mem::take(&mut kept.flags) {
        stored.flags.entry(key).or_default().union_with(&live);
    }
    // Deck memos and check states are process-local and never stored; retain them on import.
    stored.checks = std::mem::take(&mut kept.checks);
    stored.decks = std::mem::take(&mut kept.decks);
    *kept = stored;
    CareerReply::StateIn(counts)
}

impl CareerSetup<'_> {
    // ---- what a stored state (`state_in`) says of a setup, checked where it meets this one ----
    /// a stored deck: entries of this job's deck space, within its deck size, a chance per uncertain target
    fn sel_fits(&self, deck: &SelectedDeck) -> bool {
        deck.entries.iter().all(|&j| j < self.space.card_indices.len())
            && deck.entries.len() + deck.plain_pals + deck.filler_cards <= self.space.deck_job.deck_size
            && deck.white_hint_chances.len() == self.uncertain_count
    }
    /// a stored fully checked deck: a chance per job target, and its deck
    fn ref_fits(&self, reference: &Ref) -> bool {
        reference.inheritance_chances.len() == self.target_count && self.sel_fits(&reference.deck)
    }
    /// stored proofs (`KeptState::seen`): per deck class (one per uma) the fully checked decks
    pub(crate) fn seen_fits(&self, seen: &crate::uma::UmaTable<Vec<Ref>>) -> bool {
        seen.len() == self.data.umas.len() && seen.iter().flatten().all(|reference| self.ref_fits(reference))
    }
    /// a stored main search (`KeptState::searches`) of `pool`: per uma its refs, product proofs and class flags,
    /// careers of its trainees and veterans
    pub(crate) fn solved_fits(
        &self,
        solved: &CareerSearchResult,
        ruled: &Ruled,
        class_flags: &crate::uma::UmaTable<ClassFlags>,
        pool: &Pool,
    ) -> bool {
        let uma_count = self.data.umas.len();
        ruled.len() == uma_count
            && class_flags.len() == uma_count
            && ruled.iter().flatten().all(|proof| proof.inheritance_chances.len() == self.target_count)
            && solved.refs.len() == uma_count
            && solved.refs.iter().flatten().all(|reference| self.ref_fits(reference))
            && solved.found.iter().all(|found_career| {
                found_career.trainee.0 < uma_count
                    && found_career.first_parent < pool.veterans.len()
                    && found_career.second_parent < pool.veterans.len()
                    && self.sel_fits(&found_career.deck)
            })
    }
}

/// `state_out`'s answer: the kept work to store.
#[derive(Serialize, TS)]
pub(crate) struct StateOut {
    /// the kept work as one opaque string (`encode`), which the page stores and sends back to `state_in` unread
    pub(crate) state: String,
}

/// What a store restored (`state_in`; a store it refuses is a refusal, which the page drops): the kept plans, proofs'
/// keys, main searches, roll states and step caps.
#[derive(Serialize, TS)]
pub(crate) struct StateCounts {
    pub(crate) bests: usize,
    pub(crate) seen: usize,
    pub(crate) solve: usize,
    pub(crate) rolls: usize,
    pub(crate) caps: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::plan::caps::{CapCurve, KeptCap, StepCaps};
    use crate::career::plan::cont::{Cont, OddsScales, ScoreBase, ScoringKnobs, StepCareers};
    use crate::career::plan::roll::KeptBase;
    use crate::career::plan::score::kernel::Scored;
    use crate::career::search::{Assumes, Unproven, UnprovenReason};
    use crate::career::{CertifiedCap, Lineage};
    use crate::uma::{UmaIndex, UmaTable};

    /// The refusal's reason for a store `state_in` refuses (None: it was taken).
    fn refusal(kept: &mut KeptState, state: &str) -> Option<String> {
        match state_in(kept, state) {
            CareerReply::Refusal(refusal) => Some(refusal.error),
            _ => None,
        }
    }

    /// A roll state whose base holds a finite step and one that never ends, and a step cap whose plan now never ends
    /// (her odds now 0).
    fn roll_with_a_step_that_never_ends(deck: SelectedDeck) -> RollKeep {
        let continuations = [Cont::new(0.5, 0.5, 0.01, 0.001), Cont::default()];
        let cap = KeptCap { curve: CapCurve::of(&continuations[0]), odds: 0.001 };
        let step_caps = [StepCaps::default(), StepCaps::new(vec![cap], 0.0)];
        let base = ScoreBase {
            candidates: &[UmaIndex(0), UmaIndex(1)],
            step_costs: &[StepCareers::finite(0.1 + 0.2), StepCareers::Never],
            current_value: 7.5,
            current_odds: &[0.001, 0.0],
            continuations: &continuations,
            step_caps: &step_caps,
            trios: &[],
            trio_sides: &[],
            finisher_umas: &[],
            knobs: ScoringKnobs::default(),
            odds_scales: OddsScales::ONE,
        };
        RollKeep {
            key: 5,
            scored: Scored::default(),
            later: vec![],
            base: KeptBase::of(&base),
            lineage: Lineage { trainee: UmaIndex(0), first_parent: 0, second_parent: 1 },
            deck,
        }
    }

    /// What `state_out` gives, `state_in` puts back bit for bit: any finite f64 (-0.0, a subnormal, 0.1 + 0.2, the
    /// largest), a step that never ends (`StepCareers::Never`, a step and a step cap's plan), a key past 2^53, the reasons
    /// and assumptions as sets, a certificate of none; the same string out again. A store of another version, not JSON,
    /// cut short or with text past its end is refused, nothing installed.
    #[test]
    fn a_store_comes_back_as_it_left_and_another_version_is_refused() {
        let deck = SelectedDeck {
            entries: vec![2],
            plain_pals: 1,
            filler_cards: 2,
            white_hint_chances: vec![-0.0],
            gold_hint_chances: vec![f64::MIN_POSITIVE / 2.0],
        };
        let reference = Ref { inheritance_chances: vec![0.1 + 0.2, 1.0], value: 0.3, deck: deck.clone() };
        let solved = CareerSearchResult {
            found: vec![],
            refs: UmaTable(vec![vec![], vec![reference.clone()]]),
            work: 7,
            candidates: 9,
            solved: 1,
            bounded: 2,
            scored: 3,
            skipped: 4,
            next_bound: 0.0,
            complete: false,
            unproven: Unproven::from([UnprovenReason::WorkCap, UnprovenReason::CapBreach]),
            assumes: Assumes::new(),
            proven: false,
            certified_cap: CertifiedCap::default(),
        };
        let proof = ProductProof { inheritance_chances: vec![0.5], product_bound: f64::MAX };
        let kept = KeptState {
            seen: vec![KeptSeen { key: u64::MAX - 1, refs: UmaTable(vec![vec![reference]]) }],
            searches: vec![KeptSolve {
                key: 1 << 60,
                solved,
                ruled: UmaTable(vec![vec![proof], vec![]]),
                class_flags: UmaTable(vec![ClassFlags::default(); 2]),
                classes: vec![UmaIndex(1)],
            }],
            rolls: vec![roll_with_a_step_that_never_ends(deck)],
            ..KeptState::default()
        };
        let stored = state_out(&kept).state;
        let mut restored = KeptState::default();
        assert_eq!(refusal(&mut restored, &stored), None);
        assert_eq!(state_out(&restored).state, stored, "the same store out again");
        let bits = |values: &[f64]| values.iter().map(|value| value.to_bits()).collect::<Vec<_>>();
        let reference = &restored.seen[0].refs[UmaIndex(0)][0];
        assert_eq!(restored.seen[0].key, u64::MAX - 1);
        assert_eq!(bits(&reference.inheritance_chances), bits(&[0.1 + 0.2, 1.0]));
        assert_eq!(bits(&reference.deck.white_hint_chances), bits(&[-0.0]));
        assert_eq!(bits(&reference.deck.gold_hint_chances), bits(&[f64::MIN_POSITIVE / 2.0]));
        let search = &restored.searches[0];
        assert_eq!(search.key, 1 << 60);
        assert_eq!(search.ruled[UmaIndex(0)][0].product_bound.to_bits(), f64::MAX.to_bits());
        assert_eq!(search.solved.unproven, Unproven::from([UnprovenReason::WorkCap, UnprovenReason::CapBreach]));
        assert_eq!(search.solved.certified_cap, CertifiedCap::default());
        let base = restored.rolls[0].base.base();
        assert!(
            matches!(base.step_costs, [StepCareers::Finite(step), StepCareers::Never] if step.to_bits() == (0.1f64 + 0.2).to_bits()),
            "the steps came back as {:?}",
            base.step_costs
        );
        let cap_now = base.step_caps[1].now.expect("her cap");
        assert_eq!(cap_now.now.expected_careers, StepCareers::Never, "the cap's plan at no odds");

        let other_version = encode(STATE_VERSION - 1, &KeptState::default());
        assert_eq!(refusal(&mut restored, &other_version).as_deref(), Some("state: another version"));
        let mut cut_short = stored.clone();
        cut_short.pop();
        let unreadable = ["not JSON!".to_owned(), cut_short, format!("{stored} 0")];
        for state in &unreadable {
            assert_eq!(refusal(&mut restored, state).as_deref(), Some("state: does not parse"));
        }
        assert_eq!(state_out(&restored).state, stored, "a refused store installed something");
    }
}
