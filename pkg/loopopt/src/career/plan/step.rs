//! Step costs and the leaves used to estimate the next career.
use crate::career::plan::CandidateIndex;
use crate::career::plan::cont::{Cont, CostBoundCell, LB_CELLS, OwnTrio, ScoreBase, Trio, trio_v};
/// One candidate's improvement; slot indexes the estimate's per-row cost cache.
#[derive(Clone, Copy)]
pub(crate) struct CandidateLift {
    pub(crate) slot: usize,
    pub(crate) candidate: usize,
    pub(crate) probability: f64,
}

/// Mutable step costs for one outcome, restored to the base after evaluation.
pub(crate) struct StepScratch<'scratch> {
    pub(crate) costs: &'scratch mut [f64],
    pub(crate) changed_candidates: &'scratch mut Vec<usize>,
}

/// Credit for this career's own veteran when every target lands.
pub(crate) struct OwnFullLanding<'landing> {
    /// Index in ScoreBase's candidates, if this trainee is already a candidate.
    pub(crate) candidate_index: Option<usize>,
    pub(crate) completes_goal: bool,
    /// Additional trios available with this veteran as a free side.
    pub(crate) trios: &'landing [OwnTrio],
    pub(crate) full_mask: usize,
}

/// Direct-mapped probability cache entry; NaN bits mark an empty entry.
#[derive(Clone, Copy)]
pub(crate) struct CachedStepCost {
    probability_bits: u64,
    cost: f64,
}

/// How V after her (`v_after`) costs the steps of the candidates her veteran lifts: the estimate's (`StepEstimate`, `Cont::cost`,
/// for y) or the lower bound's (`Lo`, `Cont::cost_lo`, for y_hi). Monomorphised: each path keeps its own code
pub(crate) trait StepCost {
    /// Apply a candidate's lower step cost, or defer it for lazy evaluation.
    fn take(
        &mut self,
        score_base: &ScoreBase,
        costs: &mut [f64],
        changed_candidates: &mut Vec<usize>,
        lift: CandidateLift,
    );
    /// A full veteran makes her own candidate step free and cancels any pending cost.
    fn zeroed(&mut self, _candidate: usize) {}
    /// The own-veteran trios read all steps, so they require every deferred cost to be settled.
    fn value_after(
        &mut self,
        score_base: &ScoreBase,
        costs: &mut [f64],
        changed_candidates: &mut Vec<usize>,
        reads_all_steps: bool,
    ) -> f64;
}

/// The bound's steps: each lifted candidate at `Cont::cost_lo` (`saving_lo`), V after her by `v0_after`
pub(crate) struct Lo;

impl StepCost for Lo {
    #[inline]
    fn take(
        &mut self,
        score_base: &ScoreBase,
        costs: &mut [f64],
        changed_candidates: &mut Vec<usize>,
        lift: CandidateLift,
    ) {
        let step_cost = score_base.cost_lo(lift.candidate, lift.probability);
        if step_cost < costs[lift.candidate] {
            costs[lift.candidate] = step_cost;
            changed_candidates.push(lift.candidate);
        }
    }
    #[inline]
    fn value_after(&mut self, score_base: &ScoreBase, costs: &mut [f64], _: &mut Vec<usize>, _: bool) -> f64 {
        v0_after(score_base, costs)
    }
}

/// The estimate's steps (`sc_loop`): a step's cost through a per-row cache of the odds costed (`probability_costs`, slot j at j, the
/// candidates without a slot after the slots; `score_base.cost` is a pure function of them, direct-mapped by the odds' bits), V
/// after her lazily and through its shortcuts (no step moved: `unchanged_value`; one: `single_candidate_value`), with the counters `sc_eval`
/// reports.
///
/// V after her found lazily (with `lazy`): a lifted candidate on Cont's plan path (`Cont::lb_ok`) is first given her
/// odds' cell bound lo (`Cont::cost_lb`, direct-mapped by cell, LC per candidate): lo >= her step, she can't lower it
/// (cost >= lo); below, she is pending. Then the trios (by their base value, ascending; with no trio, each candidate's
/// costs + m23), each valued at lo for its pending candidates: one below the best so far has its pending candidates
/// costed (`cost`, as above) and its value taken. A trio passed over is at least that bound (lo <= the step she takes,
/// and rounded sums are monotone), so at least the minimum: every trio a pending candidate left uncosted reads is at
/// least the minimum, and with her step left at the base's (above the step she would take) still is, while every
/// trio at the minimum reads only costed steps. So the least trio value found is the minimum `v0_after` (or `single_candidate_value`)
/// takes over every trio, the same f64 (equal minima the same bits: `sc_outcomes`), and V after her. Her own 8/8's
/// trios (`Scored::own_trios`) read every step: then all are costed, and V after her is taken as without the lazy pass
#[derive(Default)]
pub(crate) struct StepEstimate<'base> {
    inputs: StepInputs<'base>,
    pub(crate) workspace: StepWorkspace,
}

/// Constructed once per scoring; costing an outcome never changes these tables.
#[derive(Default)]
struct StepInputs<'base> {
    lower_bound_continuations: Vec<Option<&'base Cont>>,
    trio_order: Vec<usize>,
    single_candidate_costs: &'base [One],
    unchanged_value: f64,
}

/// Probability caches and deferred candidate costs, reused for every outcome in one scoring.
#[derive(Default)]
pub(crate) struct StepWorkspace {
    pub(crate) probability_costs: Vec<CachedStepCost>,
    /// `Cont::cost_lb`'s memo, LB_CELLS entries per candidate.
    pub(crate) cell_costs: Vec<CostBoundCell>,
    pending_lifts: Vec<CandidateLift>,
    lower_bounds: Vec<f64>,
    pending_candidates: Vec<bool>,
    pub(crate) changed_outcomes: usize,
    pub(crate) cost_evaluations: usize,
    pub(crate) cache_hits: usize,
    pub(crate) single_candidate_shortcuts: usize,
}

const COST_CACHE_CAPACITY: usize = 256;

/// `StepEstimate`'s cache buffers between scorings (`FoldScratch`)
#[derive(Default)]
pub(crate) struct EstimateBuffers {
    pub(crate) probability_costs: Vec<CachedStepCost>,
    /// `Cont::cost_lb`'s memo, LB_CELLS entries per candidate.
    pub(crate) cell_costs: Vec<CostBoundCell>,
}

impl<'base> StepEstimate<'base> {
    /// for `rows` lift rows; `lazy` (`sc_outcomes`: every step, stage 2-3 cost and C's step a number >= 0, not -0.0)
    pub(crate) fn new(
        score_base: &'base ScoreBase<'_>,
        rows: usize,
        lazy: bool,
        single_candidate_costs: &'base [One],
        buffers: EstimateBuffers,
    ) -> Self {
        let n = score_base.candidates.len();
        let lower_bound_continuations: Vec<Option<&Cont>> =
            (0..n).map(|ui| score_base.continuations.get(ui).filter(|c| lazy && c.lb_ok())).collect();
        let trio_order: Vec<usize> = if lazy {
            let trio_values: Vec<f64> =
                score_base.trios.iter().map(|t| trio_v(score_base.step_costs, std::slice::from_ref(t))).collect();
            let mut t: Vec<usize> = (0..score_base.trios.len()).collect();
            t.sort_by(|&a, &b| trio_values[a].total_cmp(&trio_values[b]));
            t
        } else {
            vec![]
        };
        // (V after her when no candidate's step moves, most outcomes lifting no one: once)
        // (the caches' buffers reused, refilled: NaN's bits mark an empty probability-cache slot, u64::MAX an empty cell)
        let EstimateBuffers { mut probability_costs, mut cell_costs } = buffers;
        probability_costs.clear();
        probability_costs
            .resize(rows * COST_CACHE_CAPACITY, CachedStepCost { probability_bits: f64::NAN.to_bits(), cost: 0.0 });
        cell_costs.clear();
        if lazy {
            cell_costs.resize(n * LB_CELLS, CostBoundCell { key: u64::MAX, bound: 0.0 });
        }
        StepEstimate {
            inputs: StepInputs {
                lower_bound_continuations,
                trio_order,
                single_candidate_costs,
                unchanged_value: v0_after(score_base, score_base.step_costs),
            },
            workspace: StepWorkspace {
                probability_costs,
                cell_costs,
                pending_lifts: vec![],
                lower_bounds: vec![0.0f64; n],
                pending_candidates: vec![false; n],
                changed_outcomes: 0,
                cost_evaluations: 0,
                cache_hits: 0,
                single_candidate_shortcuts: 0,
            },
        }
    }
}

impl StepWorkspace {
    /// her step at odds p: costed (through `probability_costs`), taken where it is lower
    #[inline(always)]
    fn cost(
        &mut self,
        score_base: &ScoreBase,
        costs: &mut [f64],
        changed_candidates: &mut Vec<usize>,
        lift: CandidateLift,
    ) {
        let probability_bits = lift.probability.to_bits();
        let k = lift.slot * COST_CACHE_CAPACITY + (probability_bits.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as usize;
        let step_cost = if self.probability_costs[k].probability_bits == probability_bits {
            self.cache_hits += 1;
            self.probability_costs[k].cost
        } else {
            self.cost_evaluations += 1;
            let step_cost = score_base.cost(lift.candidate, lift.probability);
            self.probability_costs[k] = CachedStepCost { probability_bits, cost: step_cost };
            step_cost
        };
        if step_cost < costs[lift.candidate] {
            costs[lift.candidate] = step_cost;
            changed_candidates.push(lift.candidate);
        }
    }
    /// a pending step a trio at the minimum may read, costed
    #[inline(always)]
    fn settle(&mut self, score_base: &ScoreBase, costs: &mut [f64], changed_candidates: &mut Vec<usize>, i: usize) {
        if self.pending_candidates[i] {
            self.pending_candidates[i] = false;
            let lift = *self.pending_lifts.iter().find(|lift| lift.candidate == i).unwrap();
            self.cost(score_base, costs, changed_candidates, lift);
        }
    }
}

impl StepWorkspace {
    /// Cost only candidates needed to establish the minimum, then clear this outcome's pending flags.
    /// Keeping this phase separate limits register pressure in the much more frequent outcome loop.
    #[inline(never)]
    fn settle_pending(
        &mut self,
        inputs: &StepInputs,
        score_base: &ScoreBase,
        costs: &mut [f64],
        changed_candidates: &mut Vec<usize>,
        reads_all_steps: bool,
    ) -> Option<f64> {
        let n = score_base.candidates.len();
        // (V after her where the lazy pass found it: the minimum)
        let mut lazy_value: Option<f64> = None;
        if !self.pending_lifts.is_empty() {
            if reads_all_steps {
                for k in 0..self.pending_lifts.len() {
                    let i = self.pending_lifts[k].candidate;
                    self.settle(score_base, costs, changed_candidates, i);
                }
            } else if score_base.trios.is_empty() {
                let mut best = f64::INFINITY;
                for a in 0..n {
                    if (if self.pending_candidates[a] { self.lower_bounds[a] } else { costs[a] })
                        + score_base.later_stage_costs[a]
                        >= best
                    {
                        continue;
                    }
                    self.settle(score_base, costs, changed_candidates, a);
                    let v = costs[a] + score_base.later_stage_costs[a];
                    if v < best {
                        best = v;
                    }
                }
                lazy_value = Some(best);
            } else {
                let mut best = f64::INFINITY;
                for &trio_index in &inputs.trio_order {
                    let Trio {
                        candidate_a: CandidateIndex(first_candidate),
                        candidate_b: CandidateIndex(second_candidate),
                        finisher_cost,
                        a_uses_existing: first_existing,
                        b_uses_existing: second_existing,
                    } = score_base.trios[trio_index];
                    let pending_step = |estimate: &Self, candidate: usize, existing: bool| {
                        if existing {
                            0.0
                        } else if estimate.pending_candidates[candidate] {
                            estimate.lower_bounds[candidate]
                        } else {
                            costs[candidate]
                        }
                    };
                    if pending_step(self, first_candidate, first_existing)
                        + pending_step(self, second_candidate, second_existing)
                        + finisher_cost
                        >= best
                    {
                        continue;
                    }
                    if !first_existing {
                        self.settle(score_base, costs, changed_candidates, first_candidate);
                    }
                    if !second_existing {
                        self.settle(score_base, costs, changed_candidates, second_candidate);
                    }
                    let v = (if first_existing { 0.0 } else { costs[first_candidate] })
                        + (if second_existing { 0.0 } else { costs[second_candidate] })
                        + finisher_cost;
                    if v < best {
                        best = v;
                    }
                }
                lazy_value = Some(best);
            }
            for k in 0..self.pending_lifts.len() {
                let i = self.pending_lifts[k].candidate;
                self.pending_candidates[i] = false;
            }
            self.pending_lifts.clear();
        }
        lazy_value
    }
}

impl StepCost for StepEstimate<'_> {
    #[inline(always)]
    fn take(
        &mut self,
        score_base: &ScoreBase,
        costs: &mut [f64],
        changed_candidates: &mut Vec<usize>,
        lift: CandidateLift,
    ) {
        let b = match self.inputs.lower_bound_continuations[lift.candidate] {
            Some(continuation) => continuation.cost_lb(
                lift.probability,
                (&mut self.workspace.cell_costs[lift.candidate * LB_CELLS..(lift.candidate + 1) * LB_CELLS])
                    .try_into()
                    .unwrap(),
            ),
            None => None,
        };
        match b {
            Some(b) if b >= costs[lift.candidate] => {}
            Some(b) if b < costs[lift.candidate] => {
                self.workspace.lower_bounds[lift.candidate] = b;
                self.workspace.pending_candidates[lift.candidate] = true;
                self.workspace.pending_lifts.push(lift);
            }
            _ => self.workspace.cost(score_base, costs, changed_candidates, lift),
        }
    }
    #[inline(always)]
    fn zeroed(&mut self, candidate: usize) {
        self.workspace.pending_candidates[candidate] = false;
    }
    #[inline(always)]
    fn value_after(
        &mut self,
        score_base: &ScoreBase,
        costs: &mut [f64],
        changed_candidates: &mut Vec<usize>,
        reads_all_steps: bool,
    ) -> f64 {
        let lazy_value = if self.workspace.pending_lifts.is_empty() {
            None
        } else {
            self.workspace.settle_pending(&self.inputs, score_base, costs, changed_candidates, reads_all_steps)
        };
        if !changed_candidates.is_empty() {
            self.workspace.changed_outcomes += 1;
        }
        if let Some(v) = lazy_value {
            v
        } else if changed_candidates.is_empty() {
            self.inputs.unchanged_value
        } else if changed_candidates.len() == 1
            && !self.inputs.single_candidate_costs.is_empty()
            && self.inputs.single_candidate_costs[changed_candidates[0]].valid
        {
            self.workspace.single_candidate_shortcuts += 1;
            single_candidate_value(
                &self.inputs.single_candidate_costs[changed_candidates[0]],
                costs[changed_candidates[0]],
            )
        } else {
            v0_after(score_base, costs)
        }
    }
}

/// Expected careers saved at one outcome. Apply lifts in candidate order, then full-veteran
/// credit and the minimum continuation cost. Restore every changed step to the base before returning.
/// Both the estimate and upper bound use this sequence, with their own StepCost implementation.
#[inline(always)]
pub(crate) fn v_after<C: StepCost>(
    score_base: &ScoreBase,
    cost: &mut C,
    scratch: &mut StepScratch,
    lifted: &[CandidateLift],
    landed_mask: usize,
    own: &OwnFullLanding,
) -> f64 {
    // Split the mutable buffers before entering the estimator, preserving their disjoint borrows.
    let StepScratch { costs, changed_candidates } = scratch;
    // (the candidates whose step moved: if none, costs is the base's)
    changed_candidates.clear();
    for &lift in lifted {
        cost.take(score_base, costs, changed_candidates, lift);
    }
    if landed_mask == own.full_mask
        && let Some(candidate_index) = own.candidate_index
    {
        costs[candidate_index] = 0.0;
        changed_candidates.push(candidate_index);
        cost.zeroed(candidate_index);
    }
    let reads_all_steps = landed_mask == own.full_mask && !own.trios.is_empty();
    let mut value_after = cost.value_after(score_base, costs, changed_candidates, reads_all_steps);
    if reads_all_steps {
        value_after = with_her(value_after, own.trios, costs);
    }
    // costs back to the base's
    for &i in changed_candidates.iter() {
        costs[i] = score_base.step_costs[i];
    }
    if own.completes_goal && landed_mask == own.full_mask {
        score_base.current_value
    } else {
        (score_base.current_value - value_after).max(0.0)
    }
}

/// V after her with her own 8/8 a trio side (`her_trios`), at the candidates' step costs c1 (a real partner side free)
fn with_her(v: f64, her: &[OwnTrio], c1: &[f64]) -> f64 {
    her.iter()
        .map(|&OwnTrio { candidate: CandidateIndex(ib), uses_existing: real, finisher_cost: c3 }| {
            (if real { 0.0 } else { c1[ib] }) + c3
        })
        .fold(v, f64::min)
}

/// V after her before her own 8/8 and the goal, at the candidates' step costs c1: the trios' least cost (`trio_v`; with no
/// trio, the least of each candidate's step plus her stages 2-3)
fn v0_after(bs: &ScoreBase, c1: &[f64]) -> f64 {
    if bs.trios.is_empty() {
        (0..bs.candidates.len()).map(|a| c1[a] + bs.later_stage_costs[a]).fold(f64::INFINITY, f64::min)
    } else {
        trio_v(c1, bs.trios)
    }
}

struct TrioRemainder {
    /// Rounded rest + finisher cost, used only for sorting and the lower bound.
    key: f64,
    rest: f64,
    finisher_cost: f64,
}

/// `trio_v` with one candidate's step changed (to x, from the base's c1), per candidate i: the minimum over the trios that
/// do not read her step (their values at the base's steps, trio_v's expression), and the trios that read it once, as
/// (key = rest + C's step, rest, C's step), key ascending: a trio's value (x + rest) + C's step, the same float as trio_v's
/// sum (addition commutes exactly). Not `valid` when a trio reads her step twice
pub(crate) struct One {
    unaffected_value: f64,
    reading_trios: Vec<TrioRemainder>,
    valid: bool,
}

pub(crate) fn one_of(combos: &[Trio], c1: &[f64], i: usize) -> One {
    let (mut unaffected_value, mut reading_trios, mut valid) = (f64::INFINITY, vec![], true);
    for &Trio {
        candidate_a: CandidateIndex(first_candidate),
        candidate_b: CandidateIndex(second_candidate),
        finisher_cost,
        a_uses_existing: first_existing,
        b_uses_existing: second_existing,
    } in combos
    {
        let (first_cost, second_cost) = (
            if first_existing { 0.0 } else { c1[first_candidate] },
            if second_existing { 0.0 } else { c1[second_candidate] },
        );
        match (first_candidate == i && !first_existing, second_candidate == i && !second_existing) {
            (false, false) => {
                let v = first_cost + second_cost + finisher_cost;
                if v < unaffected_value {
                    unaffected_value = v;
                }
            }
            (true, false) => {
                reading_trios.push(TrioRemainder {
                    key: second_cost + finisher_cost,
                    rest: second_cost,
                    finisher_cost,
                });
            }
            (false, true) => {
                reading_trios.push(TrioRemainder { key: first_cost + finisher_cost, rest: first_cost, finisher_cost });
            }
            (true, true) => valid = false,
        }
    }
    reading_trios.sort_by(|p, q| p.key.partial_cmp(&q.key).unwrap());
    One { unaffected_value, reading_trios, valid }
}

/// trio_v at the base's steps with candidate i's at x (`One`): the minimum, the reading trios scanned by key until no
/// later one can be below it. Derived: every term is >= 0, so a trio's value is at least fl(x + key) (1 - u)^5 (u the
/// unit roundoff; key is rounded once, x + key once, the value's two sums once each) > fl(x + key) (1 - 1e-9), and keys
/// only rise; a trio at or above the minimum leaves it unchanged (min is order-free)
fn single_candidate_value(o: &One, x: f64) -> f64 {
    let mut best = o.unaffected_value;
    for remainder in &o.reading_trios {
        if (x + remainder.key) * (1.0 - 1e-9) >= best {
            break;
        }
        let v = (x + remainder.rest) + remainder.finisher_cost;
        if v < best {
            best = v;
        }
    }
    best
}

/// The leaves (her star patterns on the uncertain targets) a scoring's evaluation produces (`sc_eval`): `All`, the
/// plan's, every leaf in order (each test below is constant and compiles away), or `OneLeaf`, a roll's: one outcome's
/// leaf and its representative (`Scored::representative_of`). The fold goes down a prefix only where `wants` it, and
/// writes and reads each leaf's rows at `row`; the outcome loop takes the leaves in `list`'s order (ascending, as `All`
/// takes them). A leaf's values are computed down its own path alone (the fold's f64s at a leaf read no other leaf's),
/// so every value at a leaf produced is the one `All` computes there
pub(crate) trait Leaves {
    const ALL: bool;
    /// whether the fold goes down prefix p of k digits
    fn wants(&self, k: usize, p: usize) -> bool;
    /// the leaves the outcome loop takes, ascending (with `ALL`, unused: every leaf)
    fn list(&self) -> &[usize];
    /// a produced leaf's row (`ALL`: the leaf itself)
    fn row(&self, l: usize) -> usize;
    /// the rows produced, of nleaf leaves
    fn nrows(&self, nleaf: usize) -> usize;
}

pub(crate) struct All;

impl Leaves for All {
    const ALL: bool = true;
    #[inline(always)]
    fn wants(&self, _: usize, _: usize) -> bool {
        true
    }
    fn list(&self) -> &[usize] {
        &[]
    }
    #[inline(always)]
    fn row(&self, l: usize) -> usize {
        l
    }
    #[inline(always)]
    fn nrows(&self, nleaf: usize) -> usize {
        nleaf
    }
}

/// one outcome's leaf: the fold produces its representative only (row 0; a leaf that is not its own representative reads
/// the representative's y, row 1 its own)
pub(crate) struct OneLeaf {
    ls: [usize; 2],
    n: usize,
}

impl OneLeaf {
    pub(crate) const fn new(leaf: usize, rep: usize) -> Self {
        if rep == leaf { Self { ls: [leaf, leaf], n: 1 } } else { Self { ls: [rep, leaf], n: 2 } }
    }
}

impl Leaves for OneLeaf {
    const ALL: bool = false;
    fn wants(&self, k: usize, p: usize) -> bool {
        p == self.ls[0] & ((1usize << (2 * k)) - 1)
    }
    fn list(&self) -> &[usize] {
        &self.ls[..self.n]
    }
    fn row(&self, l: usize) -> usize {
        usize::from(l != self.ls[0])
    }
    fn nrows(&self, _: usize) -> usize {
        self.n
    }
}
