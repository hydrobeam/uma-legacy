//! Joint landing odds of a deck: the entry points (`joint_b`, `joint_fast`, with `joint_checked`'s diagnostic checks) and
//! the records the exact sum (`exact`), its reference (`reference`) and the negative-association bound (`upper`) share,
//! with the per-target landing factor (`land_factor`, `LandPre`) and miss rows (`TargetSources::miss_levels`) the state bounds also take.
use crate::checks::{CFCHK, SBCHK};
use crate::deck::DeckJob;
use crate::deck::chains::{ChainEvents, ChainKind};
use crate::deck::joint::exact::joint_prep;
use crate::deck::joint::reference::joint_trainee_ref;
use crate::deck::sources::{LinkMisses, SourceEntry, TargetChainMisses, TargetMisses, TargetSources, TrainingHint};
use crate::deck::state_bound::{state_value, state_value_cf};
use crate::skills::{GEN_GOLD, Model};
pub(crate) mod exact;
pub(super) mod reference;
pub(crate) mod upper;
/// The trainee and ordered inheritance chances to evaluate against one deck, under the job's chain events and checks.
#[derive(Clone, Copy)]
pub(crate) struct JointQuery<'chance> {
    /// The job's chain-event distribution (`DeckJob::chain_events`).
    pub(crate) chain_events: ChainEvents,
    /// The job's diagnostic switches (`checks::get`): `joint_b`'s checks and `upper_na`'s report.
    pub(crate) checks: &'chance crate::checks::Checks,
    /// Index in `Model::outfit_options`.
    pub(crate) trainee: crate::uma::UmaIndex,
    pub(crate) inheritance_chances: &'chance [f64],
    /// Per outfit and uncertain target: own events, or the career's folded event/inheritance chances.
    pub(crate) event_chances: &'chance [Vec<f64>],
}

impl<'chance> JointQuery<'chance> {
    /// A loop search keeps inheritance separate from the model's own-event chances.
    pub(crate) fn unfolded(
        model: &'chance Model,
        deck_job: &DeckJob,
        trainee: crate::uma::UmaIndex,
        inheritance_chances: &'chance [f64],
    ) -> Self {
        Self {
            chain_events: deck_job.chain_events(),
            checks: crate::checks::get(),
            trainee,
            inheritance_chances,
            event_chances: &model.event_chances[trainee],
        }
    }
}

/// Which exact landing probabilities the caller needs.
#[derive(Clone, Copy)]
struct JointOptions {
    /// Number of requested targets, including guaranteed golds.
    target_count: usize,
    /// Also compute the chance of landing at least all but one target.
    all_but_one: bool,
    /// Fill `JointExt::marginals` when an extension is supplied.
    marginals: bool,
}

/// Exact landing probabilities and the outfit maximizing the all-target chance.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct JointChances {
    pub(crate) all: f64,
    pub(crate) all_but_one: f64,
    /// Option index within the queried trainee's outfits.
    pub(crate) outfit: usize,
}

/// A chain's source entry, or a filler which still consumes chain events.
#[derive(Clone, Copy)]
pub(super) struct ChainEntry {
    pub(super) kind: ChainKind,
    pub(super) source_index: Option<usize>,
}

/// Exact chances that all targets land, and all but one, for `query.trainee` with one deck, using her explicit
/// `event_chances` and ordered `inheritance_chances`: P(all), P(all but one), and the outfit option per q, choosing the
/// higher P(all).
/// `entries`: the deck's sources and chain kinds; `filler`: cards without target hints, which still take chain
/// slots (as 2-link SRs). The per-target odds (`src_miss`) are the marginals of the same model; the metric the
/// search ranks by multiplies them as if targets were independent. They are not: chain cards share one run's
/// chain events (a run with many completes several chains, a short one starves them all), and one event's
/// outcomes can give one target or another. Here: the distribution of chain progress (`chain_states`); given it,
/// sources are independent, except outcome groups that touch two or more targets, whose outcomes are enumerated.
/// A card's training hints across several targets are one outcome group (`Sources::train_group`), so their
/// dependence is included in the same enumeration.
pub(super) fn joint(
    model: &Model,
    query: JointQuery,
    entries: &[SourceEntry<TargetSources>],
    filler: usize,
    requested_target_count: usize,
    all_but_one: bool,
) -> Vec<JointChances> {
    joint_b(model, query, entries, filler, requested_target_count, all_but_one)
}
/// `joint` with the sources owned or borrowed (it only reads them)
pub(crate) fn joint_b<S: std::borrow::Borrow<TargetSources>>(
    model: &Model,
    query: JointQuery,
    entries: &[SourceEntry<S>],
    filler: usize,
    requested_target_count: usize,
    all_but_one: bool,
) -> Vec<JointChances> {
    joint_checked(
        model,
        query,
        entries,
        filler,
        JointOptions { target_count: requested_target_count, all_but_one, marginals: true },
    )
}

fn joint_checked<S: std::borrow::Borrow<TargetSources>>(
    model: &Model,
    query: JointQuery,
    entries: &[SourceEntry<S>],
    filler: usize,
    options: JointOptions,
) -> Vec<JointChances> {
    let chances = evaluate_joint(model, query, entries, filler, options, None);
    if query.checks.sb_check {
        let borrowed_entries: Vec<SourceEntry<&TargetSources>> =
            entries.iter().map(|entry| SourceEntry { sources: entry.sources.borrow(), kind: entry.kind }).collect();
        if let Some(state_bounds) = state_value(model, query, &borrowed_entries, filler) {
            SBCHK.with(|cell| {
                let mut counts = cell.borrow_mut();
                for (bound, exact) in state_bounds.iter().zip(&chances) {
                    if exact.all > 0.0 {
                        let bound_ratio = bound / exact.all;
                        counts.decks += 1;
                        counts.max_ratio = counts.max_ratio.max(bound_ratio);
                        counts.min_ratio = counts.min_ratio.min(bound_ratio);
                        if bound_ratio < 1.0 - 1e-9 {
                            counts.below += 1;
                        }
                    }
                }
            });
        }
    }
    // CF_CHECK: the cap-free bound's value at a whole deck (its leaf test) against the exact value
    if query.checks.cf_check > 0.0 {
        let borrowed_entries: Vec<SourceEntry<&TargetSources>> =
            entries.iter().map(|entry| SourceEntry { sources: entry.sources.borrow(), kind: entry.kind }).collect();
        if let Some(state_bounds) = state_value_cf(model, query, &borrowed_entries, filler) {
            CFCHK.with(|cell| {
                let mut counts = cell.borrow_mut();
                for (bound, exact) in state_bounds.iter().zip(&chances) {
                    counts.scorings += 1;
                    if exact.all > bound * (1.0 + 1e-9) {
                        counts.scoring_violations += 1;
                    }
                    if *bound > 0.0 {
                        counts.max_ratio = counts.max_ratio.max(exact.all / bound);
                    }
                }
            });
        }
    }
    if query.checks.joint_check {
        let owned_entries: Vec<SourceEntry<TargetSources>> = entries
            .iter()
            .map(|entry| SourceEntry { sources: entry.sources.borrow().clone(), kind: entry.kind })
            .collect();
        let reference = joint_trainee_ref(model, query, &owned_entries, filler, options.target_count);
        for (a, b) in chances.iter().zip(&reference) {
            let relative_error = |p: f64, q: f64| (p - q).abs() / q.abs().max(1e-300);
            assert!(
                relative_error(a.all, b.all) < 1e-11
                    && (!options.all_but_one || relative_error(a.all_but_one, b.all_but_one) < 1e-11),
                "joint mismatch: {a:?} vs {b:?}"
            );
        }
    }
    chances
}

/// Per-target spark-generation scales, with each partition in job order.
pub(crate) struct SparkScales {
    pub(crate) uncertain: Vec<f64>,
    pub(crate) guaranteed: Vec<f64>,
}

pub(crate) struct JointExt {
    pub scales: SparkScales,
    pub marginals: Vec<f64>,
    pub sets: Vec<f64>,
    pub per_outfit: Vec<f64>,
}
/// `joint_b` without the diagnostic checks; `all_but_one` also computes P(at least all but one) (else 0).
/// Faster form of the same sum: chain states are collapsed to the progress that changes some target's factors (a
/// filler's progress, and links that add nothing, drop out), and outcome groups touching two or more targets are
/// enumerated per cluster of targets they connect (clusters are independent given the chain state).
/// `extension` (the career planner): each target's landing factor times its spark-generation scale (`scales.uncertain` or
/// `scales.guaranteed`, in job order), and per (chance, outfit, uncertain target) the exact chance it
/// lands, summed into `marginals`; with `sets` non-empty on entry, per (chance, outfit) the chance of each set of uncertain
/// targets landing (bit k = uncertain target k), 2^nu entries each. None leaves every number as it is.
pub(crate) fn joint_fast<S: std::borrow::Borrow<TargetSources>>(
    model: &Model,
    query: JointQuery,
    entries: &[SourceEntry<S>],
    filler: usize,
    requested_target_count: usize,
    all_but_one: bool,
    extension: Option<&mut JointExt>,
) -> Vec<JointChances> {
    evaluate_joint(
        model,
        query,
        entries,
        filler,
        JointOptions { target_count: requested_target_count, all_but_one, marginals: true },
        extension,
    )
}

fn evaluate_joint<S: std::borrow::Borrow<TargetSources>>(
    model: &Model,
    query: JointQuery,
    entries: &[SourceEntry<S>],
    filler: usize,
    options: JointOptions,
    extension: Option<&mut JointExt>,
) -> Vec<JointChances> {
    joint_prep(query.chain_events, model.uncertain.len(), entries, filler).evaluate(model, query, options, extension)
}

/// Per-target misses after conditioning on chain progress.
#[derive(Clone, Copy)]
pub(crate) struct MissFactors {
    pub(super) no_gold: f64,
    pub(super) no_hint: f64,
}

impl TargetSources {
    /// One entry's miss chances at each number of its chain links done: row d = links 1..=d fired, per target (P(no gold),
    /// P(no hint)); training hints and non-chain groups in every row. `stop` rows after row 0. The same factors
    /// `joint_fast` applies to an entry at chain progress d (`fixed_misses` and `chain_misses`).
    pub(crate) fn miss_levels(&self, stop: u8, uncertain_count: usize) -> Vec<MissFactors> {
        let mut base = vec![MissFactors { no_gold: 1.0, no_hint: 1.0 }; uncertain_count];
        for &TrainingHint { target: k, probability: p } in &self.training_hints {
            base[k].no_hint *= 1.0 - p;
        }
        for &TargetMisses { target: k, no_gold, no_hint } in &self.fixed_misses {
            base[k].no_gold *= no_gold;
            base[k].no_hint *= no_hint;
        }
        let mut levels = Vec::with_capacity((stop as usize + 1) * uncertain_count);
        levels.extend_from_slice(&base);
        for level in 1..=stop {
            let mut row: Vec<MissFactors> =
                levels[(level as usize - 1) * uncertain_count..level as usize * uncertain_count].to_vec();
            for TargetChainMisses { target: k, links } in &self.chain_misses {
                for &LinkMisses { link, no_gold, no_hint } in links {
                    if link == level {
                        row[*k].no_gold *= no_gold;
                        row[*k].no_hint *= no_hint;
                    }
                }
            }
            levels.extend_from_slice(&row);
        }
        levels
    }
}

/// Target k's landing factor at its misses (`no_gold`, `no_hint`), the one definition `JointPrep::evaluate`, `upper_na`
/// and the state bounds take (falls with both miss chances). `skills::land_dw` writes the same model in one deck's
/// per-target odds; its floats are its own.
#[inline]
pub(super) fn land_factor(coverage: u8, event_chance: f64, inheritance_chance: f64, no_gold: f64, no_hint: f64) -> f64 {
    LandPre::new(coverage, event_chance, inheritance_chance).f(no_gold, no_hint)
}
/// `land_factor` of one (outfit, target) at one chance, its constants taken once: 1 - event_chance and 1 - inheritance_chance kept apart, so
/// no_hint * (1 - event_chance) * (1 - inheritance_chance) is multiplied in the same order (the same f64 for every pair of miss chances)
#[derive(Clone, Copy)]
pub(crate) struct LandPre {
    coverage: u8,
    no_event: f64,
    no_inheritance: f64,
}
impl LandPre {
    pub(super) fn new(coverage: u8, event_chance: f64, inheritance_chance: f64) -> Self {
        Self { coverage, no_event: 1.0 - event_chance, no_inheritance: 1.0 - inheritance_chance }
    }
    #[inline]
    pub(super) fn f(&self, no_gold: f64, no_hint: f64) -> f64 {
        use crate::skills::GEN_WHITE;
        let gold_chance = 1.0 - no_gold;
        match self.coverage {
            2 => GEN_GOLD,
            1 => GEN_GOLD * gold_chance + GEN_WHITE * (1.0 - gold_chance),
            _ => {
                GEN_GOLD * gold_chance + GEN_WHITE * (1.0 - gold_chance - no_hint * self.no_event * self.no_inheritance)
            }
        }
    }
}
