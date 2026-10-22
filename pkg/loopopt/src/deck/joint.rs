//! Joint landing odds of a deck: the entry points (`joint_b`, `joint_fast`, with `joint_checked`'s diagnostic checks) and
//! the records the exact sum (`exact`), its reference (`reference`) and the negative-association bound (`upper`) share,
//! with the per-target landing factor (`land_factor`, `LandPre`) and miss rows (`src_levels`) the state bounds also take.
use crate::checks::{CFCHK, SBCHK};
use crate::deck::DeckJob;
use crate::deck::chains::{ChainEvents, ChainKind};
use crate::deck::joint::exact::{joint_eval, joint_prep};
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
    /// The job's diagnostic switches (`DeckJob::checks`): `joint_b`'s checks and `upper_na`'s report.
    pub(crate) checks: &'chance crate::checks::Checks,
    /// Index in `Model::opts`.
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
            checks: deck_job.checks,
            trainee,
            inheritance_chances,
            event_chances: &model.ews[trainee],
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
    let r = evaluate_joint(model, query, entries, filler, options, None);
    if query.checks.sb_check {
        let er: Vec<SourceEntry<&TargetSources>> =
            entries.iter().map(|entry| SourceEntry { sources: entry.sources.borrow(), kind: entry.kind }).collect();
        if let Some(sv) = state_value(model, query, &er, filler) {
            SBCHK.with(|c| {
                let mut c = c.borrow_mut();
                for (v, x) in sv.iter().zip(&r) {
                    if x.all > 0.0 {
                        let rt = v / x.all;
                        c.decks += 1;
                        c.max_ratio = c.max_ratio.max(rt);
                        c.min_ratio = c.min_ratio.min(rt);
                        if rt < 1.0 - 1e-9 {
                            c.below += 1;
                        }
                    }
                }
            });
        }
    }
    // CF_CHECK: the cap-free bound's value at a whole deck (its leaf test) against the exact value
    if query.checks.cf_check > 0.0 {
        let er: Vec<SourceEntry<&TargetSources>> =
            entries.iter().map(|entry| SourceEntry { sources: entry.sources.borrow(), kind: entry.kind }).collect();
        if let Some(sv) = state_value_cf(model, query, &er, filler) {
            CFCHK.with(|c| {
                let mut c = c.borrow_mut();
                for (v, x) in sv.iter().zip(&r) {
                    c.scorings += 1;
                    if x.all > v * (1.0 + 1e-9) {
                        c.scoring_violations += 1;
                    }
                    if *v > 0.0 {
                        c.max_ratio = c.max_ratio.max(x.all / v);
                    }
                }
            });
        }
    }
    if query.checks.joint_check {
        let own: Vec<SourceEntry<TargetSources>> = entries
            .iter()
            .map(|entry| SourceEntry { sources: entry.sources.borrow().clone(), kind: entry.kind })
            .collect();
        let x = joint_trainee_ref(model, query, &own, filler, options.target_count);
        for (a, b) in r.iter().zip(&x) {
            let rel = |p: f64, q: f64| (p - q).abs() / q.abs().max(1e-300);
            assert!(
                rel(a.all, b.all) < 1e-11 && (!options.all_but_one || rel(a.all_but_one, b.all_but_one) < 1e-11),
                "joint mismatch: {a:?} vs {b:?}"
            );
        }
    }
    r
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
    joint_eval(
        model,
        query,
        &joint_prep(query.chain_events, model.uncertain.len(), entries, filler),
        options,
        extension,
    )
}

/// Per-target misses after conditioning on chain progress.
#[derive(Clone, Copy)]
pub(crate) struct MissFactors {
    pub(super) no_gold: f64,
    pub(super) no_hint: f64,
}

/// One entry's miss chances at each number of its chain links done: row d = links 1..=d fired, per target (P(no gold),
/// P(no hint)); training hints and non-chain groups in every row. `stop` rows after row 0. The same factors
/// `joint_fast` applies to an entry at chain progress d (`fixed_misses` and `chain_misses`).
pub(super) fn src_levels(source: &TargetSources, stop: u8, uncertain_count: usize) -> Vec<MissFactors> {
    let mut base = vec![MissFactors { no_gold: 1.0, no_hint: 1.0 }; uncertain_count];
    for &TrainingHint { target: k, probability: p } in &source.training_hints {
        base[k].no_hint *= 1.0 - p;
    }
    for &TargetMisses { target: k, no_gold: ng, no_hint: nh } in &source.fixed_misses {
        base[k].no_gold *= ng;
        base[k].no_hint *= nh;
    }
    let mut out = Vec::with_capacity((stop as usize + 1) * uncertain_count);
    out.extend_from_slice(&base);
    for d in 1..=stop {
        let mut row: Vec<MissFactors> = out[(d as usize - 1) * uncertain_count..d as usize * uncertain_count].to_vec();
        for TargetChainMisses { target: k, links } in &source.chain_misses {
            for &LinkMisses { link: l, no_gold: ng, no_hint: nh } in links {
                if l == d {
                    row[*k].no_gold *= ng;
                    row[*k].no_hint *= nh;
                }
            }
        }
        out.extend_from_slice(&row);
    }
    out
}
/// Target k's landing factor at its misses (no gold `ng`, no hint `nh`), the one definition `joint_eval`, `upper_na`
/// and the state bounds take (falls with ng and with nh). `skills::land_dw` writes the same model in one deck's
/// per-target odds; its floats are its own.
#[inline]
pub(super) fn land_factor(st: u8, ew: f64, q: f64, ng: f64, nh: f64) -> f64 {
    LandPre::new(st, ew, q).f(ng, nh)
}
/// `land_factor` of one (outfit, target) at one chance, its constants taken once: 1 - ew and 1 - q kept apart, so
/// nh * (1 - ew) * (1 - q) is multiplied in the same order (the same f64 for every ng, nh)
#[derive(Clone, Copy)]
pub(crate) struct LandPre {
    coverage: u8,
    no_event: f64,
    no_inheritance: f64,
}
impl LandPre {
    pub(super) fn new(st: u8, ew: f64, q: f64) -> Self {
        Self { coverage: st, no_event: 1.0 - ew, no_inheritance: 1.0 - q }
    }
    #[inline]
    pub(super) fn f(&self, ng: f64, nh: f64) -> f64 {
        use crate::skills::GEN_WHITE;
        let pg = 1.0 - ng;
        match self.coverage {
            2 => GEN_GOLD,
            1 => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg),
            _ => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg - nh * self.no_event * self.no_inheritance),
        }
    }
}
