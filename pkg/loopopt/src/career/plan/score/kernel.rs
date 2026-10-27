//! The scoring kernels: a prepared scoring (`Scored`) folded into its outcome rows and evaluated over every outcome
//! (`Scored::evaluate`, `evaluate_outcomes`, `sc_loop`), with the scratch they reuse between scorings. Building a scoring (her setups,
//! tables and collapsed targets) is `build.rs`'s; the step costs V after her reads are `step.rs`'s.
use crate::career::clock::{now_ms, prof};
use crate::career::plan::TargetMask;
use crate::career::plan::cont::{ScoreBase, Want};
use crate::career::plan::fold::{FoldContext, FoldCursor, FoldWorkspace, LeafLayout, Leaves, SlotTables, leaf_set};
use crate::career::plan::step::{AfterHer, CandidateLift, EstimateBuffers, Route, StepEstimate};
/// Cached leaf probabilities depend only on the star mix and uncertain-target order.
#[derive(Default)]
struct LeafCache {
    star_probabilities: Vec<f64>,
    uncertain_landed_masks: Vec<usize>,
    /// Three probability bit patterns followed by job-target indices; empty means partial leaves.
    key: Vec<u64>,
}

/// Reused outcome buffers. Every representative is written before a later leaf reads it.
#[derive(Default)]
struct OutcomeScratch {
    leaf_cache: LeafCache,
    workspace: OutcomeWorkspace,
}

#[derive(Default)]
struct OutcomeWorkspace {
    representative_savings: Vec<f64>,
    /// Offset into lift_records, or u32::MAX when the representative was not recorded.
    lift_offsets: Vec<u32>,
    lift_records: Vec<LiftRecord>,
    estimate: EstimateBuffers,
}

/// A representative's candidate odds; usize::MAX ends a record.
struct LiftRecord {
    candidate: usize,
    probability: f64,
}

pub(super) struct EvaluationCounters {
    pub(super) outcomes: usize,
    pub(super) changed_outcomes: usize,
    pub(super) cost_evaluations: usize,
    pub(super) cache_hits: usize,
}

pub(super) struct ScoringEvaluation {
    pub(super) expected_saving: f64,
    pub(super) improvement_chance: f64,
    pub(super) outcome_savings: Vec<f64>,
    pub(super) lift_stats: Vec<LiftStats>,
    /// With `Want::Joint`: per set of candidates her veteran lifts at once (bit c: candidate c), its chance, by set.
    pub(super) joint_lifts: Vec<(u64, f64)>,
    /// With `Want::Classes`: the saving split by outcome class, by class (`ClassSaving`).
    pub(super) classes: Vec<ClassSaving>,
    pub(super) counters: EvaluationCounters,
}

/// One outcome class of a scoring (`Want::Classes`): her full veteran or not, and the candidates her veteran lifts
/// through the lift slots V after her reads (bit c: candidate c); its chance, its chance-weighted saving, and per
/// lifted candidate the chance-weighted odds she is lifted to (the best of her lift slots at each outcome).
#[derive(Clone, Debug, Default)]
pub(crate) struct ClassSaving {
    pub(crate) full: bool,
    pub(crate) lifted: u64,
    pub(crate) chance: f64,
    pub(crate) saving_sum: f64,
    pub(crate) lifted_odds_sums: Vec<(usize, f64)>,
}

/// Reusable scoring storage. Fold writes outcome_values before the outcome loop reads it.
#[derive(Default)]
pub(super) struct FoldScratch {
    fold: FoldWorkspace,
    outcome_values: Vec<f64>,
    outcomes: OutcomeScratch,
}

thread_local! { pub(super) static FOLD_SCRATCH: std::cell::RefCell<FoldScratch> = std::cell::RefCell::new(FoldScratch::default()); }

/// One scoring's outcome evaluation as `Scored::evaluate` reads it (built by `evaluate_career`; a plan keeps its pick's for a
/// roll, `RollKeep`): per setup her miss factor's parts and her spark scales, with what the fold and the outcome loop
/// read of them (`derive`); the tables kept (`prepare_scoring`) and each one's setup; the collapsed targets; one slot
/// per setup and per slot the odds now; her landed sets' chances; the routes; the goal, the star mix. A setup is one
/// place her veteran may be: a parent of a stage candidate beside one partner (a partner slot, the candidate's odds
/// there), or in a route's C's lineage (a finisher slot, C's odds there: `score::routes`).
pub(crate) struct Scored {
    /// nu: the job's uncertain targets.
    pub(crate) uncertain_target_count: usize,
    /// nf: the scenario-guaranteed targets.
    pub(crate) fixed_target_count: usize,
    /// All job targets (nu + nf).
    pub(crate) target_count: usize,
    /// The slots, one per setup: the partner slots first, then the lift slots, then the finisher slots.
    pub(crate) slot_count: usize,
    /// The partner slots (each candidate's odds with her veteran as a parent beside one kept partner, the best of her
    /// cached decks: the lift statistics): per slot its candidate (candidate order, each candidate's consecutive).
    pub(crate) slot_candidates: Vec<usize>,
    /// The lift slots (one career each, a route's lifted step: `score::routes`): per slot its candidate.
    pub(crate) lift_candidates: Vec<usize>,
    /// Her parents are both full veterans: her full veteran is a fully looped parent (V after her 0).
    pub(crate) completes_goal: bool,
    /// The rank's star mix (1, 2, 3 stars).
    pub(crate) star_mix: [f64; 3],
    /// The uncertain targets as job-target indices, ascending (`CareerSetup::uncertain_targets`).
    pub(crate) uncertain_targets: Vec<usize>,
    /// Per mask of the guaranteed targets landed, its landed set over the job targets (`fm_masks`).
    pub(crate) fixed_landed_masks: Vec<TargetMask>,
    /// x0: per setup and uncertain target, the miss product without her (`HerSlot::misses_without_her`).
    pub(crate) misses_without_her: Vec<Vec<f64>>,
    /// xm: per setup, her miss factor per star count (`HerSlot::miss_by_stars`).
    pub(crate) miss_by_stars: Vec<[f64; 4]>,
    /// pw: per setup and job target, the spark scale without and with her spark (`HerSlot::target_spark_scales`).
    pub(crate) target_spark_scales: Vec<Vec<[f64; 2]>>,
    /// Per kept table, its setup (`prepare_scoring`).
    pub(crate) setup_by_table: Vec<usize>,
    /// The kept tables, level 0 of the fold: table j at j 2^nu (`prepare_scoring`).
    pub(crate) initial_tables: Vec<f64>,
    /// Per uncertain target, whether it is collapsed: folded at 0 and 1 star only (`prepare_scoring`).
    pub(crate) collapsed_targets: Vec<bool>,
    /// Per setup, its slot.
    pub(crate) slot_by_setup: Vec<usize>,
    /// Per slot, the odds below the setup's: a partner slot's candidate's odds now, a finisher slot's 0.
    pub(crate) current_probabilities: Vec<f64>,
    /// Per landed set over the job targets (bit i: target i), her chance of landing exactly it.
    pub(crate) landed_set_chances: Vec<f64>,
    /// The routes through her veteran (`score::routes`); none for a scoring of the stats alone.
    pub(crate) routes: Vec<Route>,
    /// Per finisher slot (a route's `finisher_slot`), its C's uma: the scale her odds are priced at
    /// (`OddsScales::of`). Empty (a stored roll state's): every C at the base's common scale.
    pub(crate) finisher_umas: Vec<crate::uma::UmaIndex>,
    /// Derived (`derive`): per setup, uncertain target and star level, x = x0 xm.
    pub(crate) present_factors: Vec<Vec<[f64; 4]>>,
    /// Derived (`derive`): 1 - x of `present_factors`.
    pub(crate) absent_factors: Vec<Vec<[f64; 4]>>,
    /// Derived (`derive`): per landed set m and setup, the spark scale (m-major: m * setup count + setup).
    pub(crate) spark_scales: Vec<f64>,
}

impl Scored {
    /// what the fold and the outcome loop read of the setups: per setup, target and star level x = x0 xm and 1 - x; per
    /// landed set m and setup the spark scale, the product over job targets of pw at her spark there (m-major:
    /// scf[m * ns + si])
    pub(crate) fn derive(&mut self) {
        let uncertain_target_count = self.uncertain_target_count;
        self.present_factors = self
            .misses_without_her
            .iter()
            .zip(&self.miss_by_stars)
            .map(|(x0, xm)| {
                (0..uncertain_target_count).map(|k| [0usize, 1, 2, 3].map(|her_stars| x0[k] * xm[her_stars])).collect()
            })
            .collect();
        self.absent_factors = self
            .present_factors
            .iter()
            .map(|setup_factors| setup_factors.iter().map(|star_factors| star_factors.map(|x| 1.0 - x)).collect())
            .collect();
        let (landed_set_count, target_count) = (1usize << self.target_count, self.target_count);
        self.spark_scales = Vec::with_capacity(landed_set_count * self.setup_count());
        for m in 0..landed_set_count {
            self.spark_scales.extend(self.target_spark_scales.iter().map(|pw| {
                (0..target_count).map(|k| pw[k][usize::from(TargetMask::from_bits(m).contains(k))]).product::<f64>()
            }));
        }
    }
    /// A setup's odds at one outcome (her stars `leaf`, her landed set `landed_mask`) before the slot's floor: its
    /// tables interpolated there directly (`interp_in`, the fold's order and arithmetic: its f64s at that leaf, up to
    /// the collapse's rounding) times its spark scale
    pub(crate) fn setup_odds_at(&self, setup: usize, leaf: usize, landed_mask: usize) -> f64 {
        let table_size = 1usize << self.uncertain_target_count;
        let mut buffer = vec![];
        let point: Vec<f64> =
            (0..self.uncertain_target_count).map(|k| self.present_factors[setup][k][(leaf >> (2 * k)) & 3]).collect();
        let most = (0..self.setup_by_table.len())
            .filter(|&table| self.setup_by_table[table] == setup)
            .map(|table| {
                crate::career::outcomes::interp_in(
                    &self.initial_tables[table * table_size..(table + 1) * table_size],
                    &point,
                    &mut buffer,
                )
            })
            .fold(0.0f64, f64::max);
        most * self.spark_scales[landed_mask * self.setup_count() + setup]
    }
    /// The finisher slots' odds at one outcome (`setup_odds_at`), raised by 1e-12 of themselves so no rounding can put
    /// one below the fold's: what `outcome_hi`'s bound takes for the routes' C
    pub(crate) fn finisher_odds_at(&self, leaf: usize, landed_mask: usize) -> Vec<f64> {
        (self.slot_candidates.len() + self.lift_candidates.len()..self.setup_count())
            .map(|setup| self.setup_odds_at(setup, leaf, landed_mask) * (1.0 + 1e-12))
            .collect()
    }
    pub(crate) const fn setup_count(&self) -> usize {
        self.misses_without_her.len()
    }
    /// the fold writes per leaf the setups' table values (fewer rows than per outcome and slot), else per outcome the
    /// slots' odds (`FoldContext::emit`)
    pub(crate) const fn emits_setup_products(&self) -> bool {
        (1usize << self.fixed_target_count) * self.slot_count > self.setup_count()
    }
    pub(crate) const fn full_mask(&self) -> TargetMask {
        TargetMask::from_bits((1usize << self.target_count) - 1)
    }
    /// a leaf's representative: every collapsed target's nonzero digit set to 1 (`evaluate_outcomes`)
    pub(crate) fn representative_of(&self, leaf: usize) -> usize {
        let collapsed_mask: usize =
            (0..self.uncertain_target_count).filter(|&k| self.collapsed_targets[k]).map(|k| 3usize << (2 * k)).sum();
        let lo_bits = collapsed_mask & (usize::MAX / 3);
        let collapsed_digits = leaf & collapsed_mask;
        (leaf & !collapsed_mask) | ((collapsed_digits | collapsed_digits >> 1) & lo_bits)
    }
}

/// One candidate's lift statistics from a career's scoring (`score_career`'s stats): over her outcomes, the chance her
/// veteran lifts the candidate's odds and the lifted odds summed at each outcome's chance.
#[derive(Clone, Copy, Default)]
pub(crate) struct LiftStats {
    /// pl: P(her veteran lifts the candidate).
    pub(crate) lift_chance: f64,
    /// sp: the candidate's odds summed over the outcomes that lift her, each at its chance (mean lifted odds sp / pl).
    pub(crate) lifted_odds_sum: f64,
}

impl LiftStats {
    /// A builder's lift efficiency for the candidate at odds p: her veteran lifts the candidate with chance pl to odds
    /// sp / pl on average, so pl (1/p - 1/(sp/pl)) careers saved per career (Cont's builders, the proxies' admission
    /// and `proxy_check`; PROXY_EFF was calibrated on it)
    pub(crate) fn efficiency(self, p: f64) -> f64 {
        self.lift_chance * (1.0 / p - 1.0 / (self.lifted_odds_sum / self.lift_chance))
    }
}

impl Scored {
    /// Both evaluation phases borrow the same target layout and slot tables.
    fn fold_context<'score, L: Leaves>(&'score self, leaves: &'score L) -> FoldContext<'score, L> {
        FoldContext {
            leaf_layout: LeafLayout {
                uncertain_targets: &self.uncertain_targets,
                fixed_landed_masks: &self.fixed_landed_masks,
                fixed_target_count: self.fixed_target_count,
            },
            slot_tables: SlotTables {
                spark_scales: &self.spark_scales,
                slot_by_setup: &self.slot_by_setup,
                current_probabilities: &self.current_probabilities,
                setup_count: self.setup_count(),
                slot_count: self.slot_count,
            },
            initial_tables: &self.initial_tables,
            setup_by_table: &self.setup_by_table,
            present_factors: &self.present_factors,
            absent_factors: &self.absent_factors,
            collapsed_targets: &self.collapsed_targets,
            collapsed_star_levels: [0, 1],
            leaves,
            emit_setup_products: self.emits_setup_products(),
        }
    }

    pub(crate) fn after_her(&self) -> AfterHer<'_> {
        AfterHer {
            completes_goal: self.completes_goal,
            full_mask: self.full_mask().bits(),
            routes: &self.routes,
            lift_slot_count: self.lift_candidates.len(),
            finisher_umas: &self.finisher_umas,
        }
    }
}

impl Scored {
    /// A scoring's evaluation (`Scored`) at the leaves `leaves` produces: the fold of its tables into the outcome rows
    /// (with
    /// any setup slots: `FoldContext::fold`), then the outcome loop (`evaluate_outcomes`): what `evaluate_outcomes` returns, with whether an
    /// outcome
    /// that lands every target reads y < 0.5 (with y). The plan's scorings take `All`; a roll `OneLeaf` (`Scored::roll`), the same
    /// arithmetic in the same order at the leaves it produces
    #[inline(never)]
    pub(super) fn evaluate<L: Leaves>(
        &self,
        score_base: &ScoreBase,
        leaves: &L,
        want: Want,
        scratch: &mut FoldScratch,
    ) -> ScoringEvaluation {
        let (uncertain_target_count, fixed_target_count, setup_count, slot_count) =
            (self.uncertain_target_count, self.fixed_target_count, self.setup_count(), self.slot_count);
        let emit_setup_products = self.emits_setup_products();
        let fold_context = self.fold_context(leaves);
        let leaf_rows = leaves.nrows(1usize << (2 * uncertain_target_count));
        let outcome_value_count =
            if emit_setup_products { leaf_rows * setup_count } else { (leaf_rows << fixed_target_count) * slot_count };
        // (reused across calls: every row is written before it is read)
        if scratch.outcome_values.len() < outcome_value_count {
            scratch.outcome_values.resize(outcome_value_count, 0.0);
        }
        if slot_count > 0 {
            let table_count = self.setup_by_table.len();
            if scratch.fold.levels.len() < uncertain_target_count + 1 {
                scratch.fold.levels.resize(uncertain_target_count + 1, vec![]);
            }
            for k in 1..=uncertain_target_count {
                let level_size = table_count << (uncertain_target_count - k);
                if scratch.fold.levels[k].len() < level_size {
                    scratch.fold.levels[k].resize(level_size, 0.0);
                }
            }
            if scratch.fold.setup_factors.len() < 16 * setup_count {
                scratch.fold.setup_factors.resize(16 * setup_count, 0.0);
            }
            if scratch.fold.block_accumulators.len() < 16 * slot_count {
                scratch.fold.block_accumulators.resize(16 * slot_count, 0.0);
            }
            let fold_started = now_ms();
            fold_context.fold(
                FoldCursor { target: 0, leaf: 0, landed_mask: 0 },
                &mut scratch.fold,
                &mut scratch.outcome_values[..outcome_value_count],
            );
            prof("sc_fold_ms", now_ms() - fold_started);
        }
        let outcomes_started = now_ms();
        let evaluation = evaluate_outcomes(
            score_base,
            self,
            &fold_context,
            &scratch.outcome_values[..outcome_value_count],
            want,
            &mut scratch.outcomes,
        );
        prof("sc_loop_ms", now_ms() - outcomes_started);
        evaluation
    }
}

/// `evaluate_career`'s outcome loop: every outcome `fold_context.leaves` produces (her stars per uncertain target,
/// the guaranteed golds present or not), its chance, V after her and y; (E[y], P(y > 1e-9), y per outcome (at its
/// leaf's row), per candidate
/// stats: what `want` asks for; whether an outcome landing every target reads y < 0.5; counters). Its own function,
/// entered once per scoring, so V8 compiles it optimised after the first few (`docs/PERF.md`, *In the page*). On a
/// collapsed target (`collapsed_targets`, `prepare_scoring`: folded at 0 and 1 star only) a leaf with 2 or 3 stars there reads
/// its representative's values (`representative_of`: those digits set to 1, so a smaller leaf, met first): y as
/// computed there (`representative_savings`) and each candidate's odds for the stats; every sum (E[y], the lift,
/// the stats) still takes
/// every outcome in the loop's order at its own chance, the same sums as without the collapse wherever the
/// representative's values are the leaf's own.
///
/// What is fixed per call is decided here, once: the loop (`sc_loop`) is specialised by whether it takes the products
/// itself (`emits_setup_products`), whether any target is collapsed and what is wanted (y or the stats); the partner
/// slots' candidates and thresholds (her odds now (1 + 1e-9)) are read off the slots
#[inline(never)]
fn evaluate_outcomes<L: Leaves>(
    score_base: &ScoreBase,
    scored: &Scored,
    fold_context: &FoldContext<L>,
    outcome_values: &[f64],
    want: Want,
    scratch: &mut OutcomeScratch,
) -> ScoringEvaluation {
    let leaves = fold_context.leaves;
    let (uncertain_target_count, slot_count) = (scored.uncertain_target_count, scored.slot_count);
    scored.prepare_leaf_cache(leaves, &mut scratch.leaf_cache);
    // A leaf's representative has each nonzero collapsed-target digit at 1.
    let collapse_mask: usize =
        (0..uncertain_target_count).filter(|&k| scored.collapsed_targets[k]).map(|k| 3usize << (2 * k)).sum();
    let low_digit_bits = collapse_mask & (usize::MAX / 3);
    // the partner slots' candidates (consecutive per candidate) and the odds above which each lifts hers
    let candidate_indices = &scored.slot_candidates;
    assert!(candidate_indices.windows(2).all(|pair| pair[0] <= pair[1]), "partner slots come in candidate order");
    let thresholds: Vec<f64> = (0..candidate_indices.len() + scored.lift_candidates.len())
        .map(|slot| scored.current_probabilities[slot] * (1.0 + 1e-9))
        .collect();
    // Setups are in slot order; slot j owns setup_offsets[j]..setup_offsets[j + 1].
    let mut setup_offsets = vec![0usize; slot_count + 1];
    for (setup, &slot) in scored.slot_by_setup.iter().enumerate() {
        assert!(setup == 0 || scored.slot_by_setup[setup - 1] <= slot, "setups come in slot order");
        setup_offsets[slot + 1] = setup + 1;
    }
    let context = OutcomeContext {
        score_base,
        fold: fold_context,
        candidates: CandidateSlots {
            values: outcome_values,
            setup_offsets: &setup_offsets,
            candidate_indices,
            lift_candidates: &scored.lift_candidates,
            thresholds: &thresholds,
        },
        after_her: scored.after_her(),
        probabilities: OutcomeProbabilities {
            outcome_probabilities: &scored.landed_set_chances,
            star_probabilities: &scratch.leaf_cache.star_probabilities,
            uncertain_landed_masks: &scratch.leaf_cache.uncertain_landed_masks,
            collapse_mask,
            low_digit_bits,
        },
        retain_outcome_savings: want == Want::Ys,
        joint_lifts: want == Want::Joint,
        class_split: want == Want::Classes,
    };
    context.evaluate(scored.emits_setup_products(), matches!(want, Want::Stats | Want::Joint), &mut scratch.workspace)
}

impl Scored {
    fn prepare_leaf_cache<L: Leaves>(&self, leaves: &L, cache: &mut LeafCache) {
        let (uncertain_target_count, w) = (self.uncertain_target_count, self.star_mix);
        // per leaf (at its row) her landed set on the uncertain targets and her stars' chance: the product over k of w[her
        // stars on target k - 1] where she has any, in k order from 1.0 (every leaf: the leaves with digit k > 0 extend those
        // below 4^k; else per leaf, `leaf_set`, the same f64s)
        // (they depend on w and `uncertain_targets` alone: built for every leaf, they are kept until those change, `cache.key`)
        if L::ALL {
            let same = cache.key.len() == 3 + uncertain_target_count
                && cache.key[..3].iter().zip(&w).all(|(&k, weight)| k == weight.to_bits())
                && cache.key[3..].iter().zip(&self.uncertain_targets).all(|(&k, &i)| k == i as u64);
            if !same {
                cache.star_probabilities.clear();
                cache.uncertain_landed_masks.clear();
                cache.star_probabilities.push(1.0);
                cache.uncertain_landed_masks.push(0usize);
                for k in 0..uncertain_target_count {
                    let lo = 1usize << (2 * k);
                    for stars in 1..4usize {
                        for i in 0..lo {
                            cache.star_probabilities.push(cache.star_probabilities[i] * w[stars - 1]);
                            cache
                                .uncertain_landed_masks
                                .push(cache.uncertain_landed_masks[i] | (1 << self.uncertain_targets[k]));
                        }
                    }
                }
                cache.key.clear();
                cache.key.extend(w.iter().map(|weight| weight.to_bits()));
                cache.key.extend(self.uncertain_targets.iter().map(|&i| i as u64));
            }
        } else {
            cache.key.clear();
            cache.star_probabilities.clear();
            cache.uncertain_landed_masks.clear();
            for &leaf in leaves.list() {
                let outcome = leaf_set(leaf, &self.uncertain_targets, w);
                cache.star_probabilities.push(outcome.star_probability);
                cache.uncertain_landed_masks.push(outcome.landed_mask);
            }
        }
    }
}

/// Outcome-only rows and candidate lookup, alongside the shared SlotTables view.
struct CandidateSlots<'slots> {
    values: &'slots [f64],
    /// Slot j owns setups offsets[j]..offsets[j + 1].
    setup_offsets: &'slots [usize],
    /// Per partner slot, its candidate; per lift slot, its; per partner then lift slot, her threshold.
    candidate_indices: &'slots [usize],
    lift_candidates: &'slots [usize],
    thresholds: &'slots [f64],
}

struct OutcomeProbabilities<'out> {
    /// Probability indexed by the job-target landed mask, before the star mix.
    outcome_probabilities: &'out [f64],
    /// Both leaf arrays use Leaves::row, including partial-leaf scorings.
    star_probabilities: &'out [f64],
    uncertain_landed_masks: &'out [usize],
    collapse_mask: usize,
    low_digit_bits: usize,
}

/// Immutable outcome inputs, separate from the reusable caches and accumulators.
struct OutcomeContext<'out, 'base, L: Leaves> {
    score_base: &'out ScoreBase<'base>,
    fold: &'out FoldContext<'out, L>,
    candidates: CandidateSlots<'out>,
    probabilities: OutcomeProbabilities<'out>,
    after_her: AfterHer<'out>,
    retain_outcome_savings: bool,
    /// The stats also gather the joint lift sets (`ScoringEvaluation::joint_lifts`).
    joint_lifts: bool,
    /// The savings also split by outcome class (`ScoringEvaluation::classes`).
    class_split: bool,
}

impl<L: Leaves> OutcomeContext<'_, '_, L> {
    /// Each slot's odds after her at row r (outcome m, fm): with PL the best of her odds now and her setups' spark scale times
    /// table value, per slot over its setups in setup order (the same strict maxima `FoldContext::emit` takes), else the folded row
    #[inline(always)]
    fn row<'row, const PL: bool>(
        &'row self,
        leaf_row: usize,
        landed_mask: usize,
        fixed_mask: usize,
        probability_buffer: &'row mut [f64],
    ) -> &'row [f64] {
        if PL {
            let (folded_row, scale_row) = (
                &self.candidates.values[leaf_row * self.fold.slot_tables.setup_count..]
                    [..self.fold.slot_tables.setup_count],
                &self.fold.slot_tables.spark_scales[landed_mask * self.fold.slot_tables.setup_count..]
                    [..self.fold.slot_tables.setup_count],
            );
            for ((probability, &current_probability), setup_range) in probability_buffer
                .iter_mut()
                .zip(self.fold.slot_tables.current_probabilities)
                .zip(self.candidates.setup_offsets.windows(2))
            {
                let mut best = current_probability;
                for (&scratch, &factor) in
                    scale_row[setup_range[0]..setup_range[1]].iter().zip(&folded_row[setup_range[0]..setup_range[1]])
                {
                    let value = scratch * factor;
                    if value > best {
                        best = value;
                    }
                }
                *probability = best;
            }
            probability_buffer
        } else {
            &self.candidates.values[((leaf_row << self.fold.leaf_layout.fixed_target_count) | fixed_mask)
                * self.fold.slot_tables.slot_count..][..self.fold.slot_tables.slot_count]
        }
    }
}

/// `evaluate_outcomes`' loop, specialised: PL (the products taken here, `OutcomeContext::row`), COL (a target collapsed), Y (y and its sums
/// computed; else E[y] and the lift 0.0, no step costed), S (the stats; else none), L (the leaves taken, in order; rows,
/// y and the representatives' records at each leaf's row). With S a representative outcome that
/// other leaves read records the candidates it lifts and their odds (`lift_records` from `lift_offsets[outcome]`, ended by usize::MAX;
/// u32::MAX: not recorded, past the cap), which the leaves reading it add to their stats (each candidate's sums still in
/// outcome order: the same f64s); unrecorded, they take its row again. Separate mutable buffer
/// borrows preserve nonaliasing information across this hot function boundary.
#[inline(never)]
fn sc_loop<const PL: bool, const COL: bool, const Y: bool, const S: bool, L: Leaves>(
    context: &OutcomeContext<L>,
    representative_savings: &mut Vec<f64>,
    lift_offsets: &mut Vec<u32>,
    lift_records: &mut Vec<LiftRecord>,
    estimate_buffers: &mut EstimateBuffers,
) -> ScoringEvaluation {
    let score_base = context.score_base;
    // Keep immutable hot slices independent of writes to the reusable outcome buffers.
    let candidate_indices = context.candidates.candidate_indices;
    let thresholds = context.candidates.thresholds;
    let outcome_probabilities = context.probabilities.outcome_probabilities;
    let fixed_landed_masks = context.fold.leaf_layout.fixed_landed_masks;
    let candidate_count = score_base.candidates.len();
    let (fixed_target_count, slot_count) =
        (context.fold.leaf_layout.fixed_target_count, context.fold.slot_tables.slot_count);
    let partner_slot_count = candidate_indices.len();
    let lift_candidates = context.candidates.lift_candidates;
    let lifting_slots = partner_slot_count + lift_candidates.len();
    let (leaf_count, leaves) =
        ((1usize << (2 * context.fold.leaf_layout.uncertain_targets.len())), context.fold.leaves);
    let leaf_rows = leaves.nrows(leaf_count);
    // every outcome: uncertain targets' star pattern, guaranteed golds present or not
    let (mut expected_saving, mut improvement_chance) = (0.0f64, 0.0f64);
    // y per outcome (careers saved), at (row << nf) | fm (0 where the outcome can't happen); only when the caller reads it
    let mut outcome_savings: Vec<f64> =
        if context.retain_outcome_savings { vec![0.0; leaf_rows << fixed_target_count] } else { vec![] };
    // per stage-1 candidate: the chance her veteran raises that candidate's odds, and the odds raised to (summed, weighted)
    let mut lift_stats = if S { vec![LiftStats::default(); candidate_count] } else { vec![] };
    // (per set of candidates lifted at once, its chance: `Want::Joint` only)
    let mut joint_lifts: crate::FxMap<u64, f64> = crate::FxMap::default();
    assert!(!context.joint_lifts || candidate_count <= 64, "a joint lift set is one bit per candidate");
    let mut lift_steps = vec![];
    let mut classes: Vec<ClassSaving> = vec![];
    // (`Want::Classes`: per representative outcome the lifts it read, for the outcomes that reuse its saving)
    let mut representative_lifts: crate::FxMap<usize, Vec<CandidateLift>> = crate::FxMap::default();
    let mut outcome_count = 0usize;
    let mut probability_buffer = vec![0.0f64; slot_count];
    // V after her (`v_after` at the estimate's step costs, `StepEstimate`), from the candidates each outcome lifts (row, candidate,
    // odds: `lifted`)
    let mut estimate = if Y {
        StepEstimate::new(lift_candidates.len(), std::mem::take(estimate_buffers))
    } else {
        StepEstimate::default()
    };
    let mut lifted: Vec<CandidateLift> = vec![];
    let (collapse_mask, low_digit_bits) = (context.probabilities.collapse_mask, context.probabilities.low_digit_bits);
    let rep_of = |leaf: usize| {
        let collapsed_digits = leaf & collapse_mask;
        (leaf & !collapse_mask) | ((collapsed_digits | collapsed_digits >> 1) & low_digit_bits)
    };
    // Bound the representative records to about twice the savings buffer's memory.
    let record_capacity = (leaf_count << fixed_target_count).clamp(1 << 16, u32::MAX as usize - 1);
    // Buffers only grow. A reused representative precedes this leaf and has the same landed set,
    // so its positive outcome probability made this call write both its saving and record offset.
    if COL {
        if Y && representative_savings.len() < leaf_rows << fixed_target_count {
            representative_savings.resize(leaf_rows << fixed_target_count, 0.0);
        }
        if S {
            if lift_offsets.len() < leaf_rows << fixed_target_count {
                lift_offsets.resize(leaf_rows << fixed_target_count, u32::MAX);
            }
            lift_records.clear();
            lift_records.push(LiftRecord { candidate: usize::MAX, probability: 0.0 });
        }
    }
    let visited_leaves = if L::ALL { leaf_count } else { leaves.list().len() };
    for visit in 0..visited_leaves {
        let leaf = if L::ALL { visit } else { leaves.list()[visit] };
        let leaf_row = leaves.row(leaf);
        let (uncertain_mask, star_probability) = (
            context.probabilities.uncertain_landed_masks[leaf_row],
            context.probabilities.star_probabilities[leaf_row],
        );
        let representative = if COL { rep_of(leaf) } else { leaf };
        // (a representative that other leaves read: y computed wherever her landed set can happen, even at chance 0 itself)
        let is_representative = COL && representative == leaf && leaf & collapse_mask != 0;
        for (fixed_mask, &fixed_landed_mask) in fixed_landed_masks.iter().enumerate() {
            let landed_mask = uncertain_mask | fixed_landed_mask.bits();
            let outcome_probability = outcome_probabilities[landed_mask] * star_probability;
            if outcome_probability <= 0.0 && !(is_representative && outcome_probabilities[landed_mask] > 0.0) {
                continue;
            }
            if COL && representative != leaf {
                // the representative's y, the stats at the representative's odds and this outcome's chance
                outcome_count += 1;
                let representative_row = leaves.row(representative);
                let representative_outcome = (representative_row << fixed_target_count) | fixed_mask;
                if S {
                    let record_start = lift_offsets[representative_outcome];
                    let mut lifted_set = 0u64;
                    if record_start == u32::MAX {
                        let probabilities =
                            context.row::<PL>(representative_row, landed_mask, fixed_mask, &mut probability_buffer);
                        candidate_maxima(
                            probabilities,
                            candidate_indices,
                            thresholds,
                            |candidate, lifted_probability| {
                                lift_stats[candidate].lift_chance += outcome_probability;
                                lift_stats[candidate].lifted_odds_sum += outcome_probability * lifted_probability;
                                if context.joint_lifts {
                                    lifted_set |= 1 << candidate;
                                }
                            },
                        );
                    } else {
                        for &LiftRecord { candidate, probability: lifted_probability } in lift_records
                            [record_start as usize..]
                            .iter()
                            .take_while(|record| record.candidate != usize::MAX)
                        {
                            lift_stats[candidate].lift_chance += outcome_probability;
                            lift_stats[candidate].lifted_odds_sum += outcome_probability * lifted_probability;
                            if context.joint_lifts {
                                lifted_set |= 1 << candidate;
                            }
                        }
                    }
                    if context.joint_lifts && lifted_set != 0 {
                        *joint_lifts.entry(lifted_set).or_default() += outcome_probability;
                    }
                }
                if Y {
                    let saving = representative_savings[representative_outcome];
                    expected_saving += outcome_probability * saving;
                    if saving > 1e-9 {
                        improvement_chance += outcome_probability;
                    }
                    if context.retain_outcome_savings {
                        outcome_savings[(leaf_row << fixed_target_count) | fixed_mask] = saving;
                    }
                    if context.class_split {
                        // (the same landed set as its representative, so the same full flag)
                        let lifts = representative_lifts.get(&representative_outcome).map_or(&[][..], Vec::as_slice);
                        let full = landed_mask == context.after_her.full_mask;
                        add_to_class(&mut classes, full, lifts, outcome_probability, saving);
                    }
                }
                continue;
            }
            // each slot's odds after her: a partner slot's the best of its setup's and her odds now, a finisher slot's C's
            // (`FoldContext::emit`; with pl, here)
            let probabilities = context.row::<PL>(leaf_row, landed_mask, fixed_mask, &mut probability_buffer);
            let possible = outcome_probability > 0.0;
            if possible {
                outcome_count += 1;
            }
            // (a representative records what it lifts, within the cap)
            let record_lifts = S && is_representative && lift_records.len() + partner_slot_count < record_capacity;
            let record_start = lift_records.len();
            // the candidates it lifts (each at the best of her partner slots): their stats and records here
            if S {
                let mut lifted_set = 0u64;
                candidate_maxima(probabilities, candidate_indices, thresholds, |candidate, lifted_probability| {
                    if possible {
                        lift_stats[candidate].lift_chance += outcome_probability;
                        lift_stats[candidate].lifted_odds_sum += outcome_probability * lifted_probability;
                    }
                    if record_lifts {
                        lift_records.push(LiftRecord { candidate, probability: lifted_probability });
                    }
                    if context.joint_lifts {
                        lifted_set |= 1 << candidate;
                    }
                });
                if context.joint_lifts && possible && lifted_set != 0 {
                    *joint_lifts.entry(lifted_set).or_default() += outcome_probability;
                }
            }
            // the lift slots it lifts, for V after her (in slot order)
            lifted.clear();
            if Y {
                for (slot, ((&lifted_probability, &threshold), &candidate)) in probabilities
                    [partner_slot_count..lifting_slots]
                    .iter()
                    .zip(&thresholds[partner_slot_count..])
                    .zip(lift_candidates)
                    .enumerate()
                {
                    if lifted_probability > threshold {
                        lifted.push(CandidateLift { slot, candidate, probability: lifted_probability });
                    }
                }
            }
            if S && is_representative {
                let outcome_index = (leaf_row << fixed_target_count) | fixed_mask;
                lift_offsets[outcome_index] = if !record_lifts {
                    u32::MAX
                } else if lift_records.len() == record_start {
                    0
                } else {
                    lift_records.push(LiftRecord { candidate: usize::MAX, probability: 0.0 });
                    record_start as u32
                };
            }
            if !Y {
                continue;
            }
            let finisher_odds = &probabilities[lifting_slots..];
            let saving = score_base.v_after(
                &mut estimate,
                &mut lift_steps,
                &lifted,
                finisher_odds,
                landed_mask,
                &context.after_her,
            );
            if is_representative {
                representative_savings[(leaf_row << fixed_target_count) | fixed_mask] = saving;
                if context.class_split {
                    representative_lifts.insert((leaf_row << fixed_target_count) | fixed_mask, lifted.clone());
                }
            }
            if !possible {
                continue;
            }
            expected_saving += outcome_probability * saving;
            if saving > 1e-9 {
                improvement_chance += outcome_probability;
            }
            if context.retain_outcome_savings {
                outcome_savings[(leaf_row << fixed_target_count) | fixed_mask] = saving;
            }
            if context.class_split {
                add_to_class(
                    &mut classes,
                    landed_mask == context.after_her.full_mask,
                    &lifted,
                    outcome_probability,
                    saving,
                );
            }
        }
    }
    if Y {
        *estimate_buffers =
            EstimateBuffers { probability_costs: std::mem::take(&mut estimate.workspace.probability_costs) };
    }
    let mut joint_lifts: Vec<(u64, f64)> = joint_lifts.into_iter().collect();
    joint_lifts.sort_by_key(|&(lifted_set, _)| lifted_set);
    ScoringEvaluation {
        expected_saving,
        improvement_chance,
        outcome_savings,
        lift_stats,
        joint_lifts,
        classes,
        counters: EvaluationCounters {
            outcomes: outcome_count,
            changed_outcomes: estimate.workspace.changed_outcomes,
            cost_evaluations: estimate.workspace.cost_evaluations,
            cache_hits: estimate.workspace.cache_hits,
        },
    }
}

/// One outcome added to its class (`ClassSaving`, `Want::Classes`)
fn add_to_class(classes: &mut Vec<ClassSaving>, full: bool, lifted: &[CandidateLift], chance: f64, saving: f64) {
    let lifted_set = lifted.iter().fold(0u64, |set, lift| set | (1 << lift.candidate));
    let index =
        classes.iter().position(|class| class.full == full && class.lifted == lifted_set).unwrap_or_else(|| {
            classes.push(ClassSaving { full, lifted: lifted_set, ..ClassSaving::default() });
            classes.len() - 1
        });
    let class = &mut classes[index];
    class.chance += chance;
    class.saving_sum += chance * saving;
    // (a candidate with several lift slots, one per kept partner: her best of them at this outcome, counted once)
    for (index, lift) in lifted.iter().enumerate() {
        if lifted[..index].iter().any(|earlier| earlier.candidate == lift.candidate) {
            continue;
        }
        let best = lifted
            .iter()
            .filter(|other| other.candidate == lift.candidate)
            .map(|other| other.probability)
            .fold(0.0f64, f64::max);
        match class.lifted_odds_sums.iter_mut().find(|(candidate, _)| *candidate == lift.candidate) {
            Some(entry) => entry.1 += chance * best,
            None => class.lifted_odds_sums.push((lift.candidate, chance * best)),
        }
    }
}

/// Per candidate with partner slots (`candidate_indices`, consecutive per candidate), the best of her slots' odds in
/// `probabilities` where it is above her threshold: `lifted(candidate, odds)`, in candidate order (the candidate's odds as
/// one slot over all her setups took them: the strict maximum, order-free)
#[inline(always)]
fn candidate_maxima(
    probabilities: &[f64],
    candidate_indices: &[usize],
    thresholds: &[f64],
    mut lifted: impl FnMut(usize, f64),
) {
    let mut slot = 0;
    while slot < candidate_indices.len() {
        let (candidate, threshold) = (candidate_indices[slot], thresholds[slot]);
        let mut best = probabilities[slot];
        slot += 1;
        while slot < candidate_indices.len() && candidate_indices[slot] == candidate {
            if probabilities[slot] > best {
                best = probabilities[slot];
            }
            slot += 1;
        }
        if best > threshold {
            lifted(candidate, best);
        }
    }
}

impl<L: Leaves> OutcomeContext<'_, '_, L> {
    /// Select the outcome kernel once per scoring; its mutable buffers remain separate.
    fn evaluate(
        &self,
        emit_setup_products: bool,
        stats_only: bool,
        scratch: &mut OutcomeWorkspace,
    ) -> ScoringEvaluation {
        let OutcomeWorkspace { representative_savings, lift_offsets, lift_records, estimate } = scratch;
        // (Y: y and its sums, S: the stats)
        match (emit_setup_products, self.probabilities.collapse_mask != 0, stats_only) {
            (false, false, false) => sc_loop::<false, false, true, false, L>(
                self,
                representative_savings,
                lift_offsets,
                lift_records,
                estimate,
            ),
            (false, true, false) => sc_loop::<false, true, true, false, L>(
                self,
                representative_savings,
                lift_offsets,
                lift_records,
                estimate,
            ),
            (true, false, false) => sc_loop::<true, false, true, false, L>(
                self,
                representative_savings,
                lift_offsets,
                lift_records,
                estimate,
            ),
            (true, true, false) => sc_loop::<true, true, true, false, L>(
                self,
                representative_savings,
                lift_offsets,
                lift_records,
                estimate,
            ),
            (false, false, true) => sc_loop::<false, false, false, true, L>(
                self,
                representative_savings,
                lift_offsets,
                lift_records,
                estimate,
            ),
            (false, true, true) => sc_loop::<false, true, false, true, L>(
                self,
                representative_savings,
                lift_offsets,
                lift_records,
                estimate,
            ),
            (true, false, true) => sc_loop::<true, false, false, true, L>(
                self,
                representative_savings,
                lift_offsets,
                lift_records,
                estimate,
            ),
            (true, true, true) => sc_loop::<true, true, false, true, L>(
                self,
                representative_savings,
                lift_offsets,
                lift_records,
                estimate,
            ),
        }
    }
}
