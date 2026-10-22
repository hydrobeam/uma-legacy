//! Anchor bounds and the best partners after an outcome.
use super::fold::RiseInputs;
use super::step::{CandidateLift, OwnFullLanding, StepScratch};
use super::{
    Lo, ScoreBase, fm_masks, her_ends, her_pool, her_pool_done, her_trios, leaf_products, leaf_set, legal_pair,
    rise_leaves, v_after,
};
use crate::deck::career::sim::plan::cont::OwnTrio;
use crate::deck::career::sim::plan::{CandidateIndex, OutcomeIndex, ScoringContext};
use crate::deck::career::sim::{DeckRes, Env, Pool, Sim, Solved, holds_fix, now_ms, prof, rise_factor};
use crate::deck::career::{Career, Lineage};

/// Ceiling factors are flattened [outfit][uncertain target][star level].
struct CeilingFactors<'factors> {
    values: &'factors [f64],
    target_count: usize,
    outfit_count: usize,
    scale: f64,
}

/// Two alternating prefix-product buffers, reused by dense and sparse leaf expansion.
struct LeafProductWorkspace<'work> {
    current: &'work mut Vec<f64>,
    next: &'work mut Vec<f64>,
}

/// `outcome_hi`'s bound for one (candidate, partner), bd per leaf: the least of the ceiling rfc = rf x the best fast
/// outfit's product and, over the anchors (f, qa), f x `rise_leaves`(qa); the same f64s as `rfc.min(anc)` with anc
/// every anchor's value folded in at every leaf (a min may be taken in any order; a NaN rfc counts as +inf there, as
/// `f64::min` drops it). Found by an exact branch and bound over the leaves' digit prefixes (digit k at bits 2k..2k+1,
/// the products' order). An anchor's factors are all > 0 and f > 0 (checked; else it goes through `rise_leaves` whole),
/// so a leaf's product is at least its prefix's product times the least factor of each remaining digit; with slack
/// BB_SLACK for rounding (far more than the (1 + 2^-53)^(2 nu + 4) the chains can drift) that bounds f x the leaf's
/// value from below at every leaf under the prefix (correctly rounded products are monotone). Where the bound reaches
/// the most bd holds under the prefix (`prefix_maxima`), the anchor lowers no leaf there: the subtree is skipped.
struct AnchorBound {
    target_count: usize,
    outfit_count: usize,
    prefix_maxima: Vec<Vec<f64>>,
    prefix_products: Vec<f64>,
    suffix_minima: Vec<f64>,
    visited_nodes: u64,
    open_roots: u64,
    sparse_leaves: bool,
    marked_accumulators: Vec<f64>,
}

const BB_SLACK: f64 = 1.0 - 1e-12;

impl AnchorBound {
    const fn new() -> Self {
        Self {
            target_count: 0,
            outfit_count: 0,
            prefix_maxima: vec![],
            prefix_products: vec![],
            suffix_minima: vec![],
            visited_nodes: 0,
            open_roots: 0,
            sparse_leaves: false,
            marked_accumulators: vec![],
        }
    }
    /// bd per leaf (level nu of `prefix_maxima`)
    fn leaf_bounds(&self) -> &[f64] {
        &self.prefix_maxima[self.target_count]
    }
    /// a fresh (candidate, partner) with `no` outfits for the anchors and the ceiling's fast outfits gc ([outfit][target
    /// k][her stars], nfo of them): bd = rfc (per leaf rf x, from 0.0, the most of each outfit's product in
    /// `leaf_products`' order: the old ceiling's f64s) at the marked leaves (`Marks`), -inf at the others (so no anchor
    /// is folded in where nothing reads the bound: every test below compares with a most that holds marked leaves only),
    /// `prefix_maxima` built
    fn reset(
        &mut self,
        anchor_outfit_count: usize,
        ceiling: &CeilingFactors,
        marks: &Marks,
        products: &mut LeafProductWorkspace,
    ) {
        self.target_count = ceiling.target_count;
        self.outfit_count = anchor_outfit_count;
        self.prefix_products.clear();
        self.prefix_products.resize((ceiling.target_count + 1) * anchor_outfit_count, 1.0);
        self.suffix_minima.clear();
        self.suffix_minima.resize((ceiling.target_count + 1) * anchor_outfit_count, 1.0);
        self.prefix_maxima.resize(ceiling.target_count + 1, vec![]);
        let nan_inf = |b: f64| if b.is_nan() { f64::INFINITY } else { b };
        let mut leaf_bounds = std::mem::take(&mut self.prefix_maxima[ceiling.target_count]);
        if ceiling.target_count == 0 {
            leaf_bounds.clear();
            leaf_bounds.push(if marks.prefix_marks[0][0] {
                nan_inf(ceiling.scale * if ceiling.outfit_count > 0 { 1.0 } else { 0.0 })
            } else {
                f64::NEG_INFINITY
            });
            self.prefix_maxima[0] = leaf_bounds;
            return;
        }
        let prefix_count = 1usize << (2 * (ceiling.target_count - 1));
        if !marks.dense {
            // few marked leaves: only the marked prefixes are built (`reset_marked`). Every other entry of every level is
            // -inf, as the full build leaves it (an unmarked leaf's -inf, a prefix the most of its quarters): set here
            // once per AnchorBound (it serves one `Marks`), and nothing raises it after (`run` and the direct anchors only
            // lower a leaf; a prefix is again the most of its quarters)
            if !self.sparse_leaves || leaf_bounds.len() != prefix_count * 4 {
                leaf_bounds.clear();
                leaf_bounds.resize(prefix_count * 4, f64::NEG_INFINITY);
                for j in 0..ceiling.target_count {
                    let t = &mut self.prefix_maxima[j];
                    t.clear();
                    t.resize(1usize << (2 * j), f64::NEG_INFINITY);
                }
                self.sparse_leaves = true;
            }
            self.prefix_maxima[ceiling.target_count] = leaf_bounds;
            self.reset_marked(ceiling, marks, products);
            return;
        }
        self.sparse_leaves = false;
        leaf_bounds.clear();
        leaf_bounds.resize(prefix_count * 4, 0.0);
        let mut g: Vec<[f64; 4]> = Vec::with_capacity(ceiling.target_count);
        for o in 0..ceiling.outfit_count {
            g.clear();
            g.extend((0..ceiling.target_count).map(|k| {
                let b = (o * ceiling.target_count + k) * 4;
                [ceiling.values[b], ceiling.values[b + 1], ceiling.values[b + 2], ceiling.values[b + 3]]
            }));
            leaf_products(&g[..ceiling.target_count - 1], products.current, products.next);
            // the last digit's level straight into the running most
            for (stars, &last_factor) in g[ceiling.target_count - 1].iter().enumerate() {
                for (c, &prefix_product) in leaf_bounds[stars * prefix_count..(stars + 1) * prefix_count]
                    .iter_mut()
                    .zip(products.current.iter())
                {
                    let product = prefix_product * last_factor;
                    if product > *c {
                        *c = product;
                    }
                }
            }
        }
        for c in &mut leaf_bounds {
            *c = nan_inf(ceiling.scale * *c);
        }
        for &l in &marks.unmarked_leaves {
            leaf_bounds[l] = f64::NEG_INFINITY;
        }
        self.prefix_maxima[ceiling.target_count] = leaf_bounds;
        self.levels();
    }
    /// `reset`'s rebuild at the marked prefixes only (sparse marks, every other entry -inf already): per outfit the
    /// products level by level down `Marks::prefix_indices` (each prefix's its parent's times its last digit's factor, from 1.0:
    /// `leaf_products`' f64s at those leaves), the running most per marked leaf over the outfits in order from 0.0, rf x it
    /// (NaN as +inf) into bd, then each marked prefix's most of its four quarters as `levels` takes it
    #[inline(never)]
    fn reset_marked(&mut self, ceiling: &CeilingFactors, marks: &Marks, products: &mut LeafProductWorkspace) {
        let marked_leaf_count = marks.prefix_indices[ceiling.target_count].len();
        let mut accumulator = std::mem::take(&mut self.marked_accumulators);
        accumulator.clear();
        accumulator.resize(marked_leaf_count, 0.0);
        for o in 0..ceiling.outfit_count {
            let g = &ceiling.values[o * ceiling.target_count * 4..(o + 1) * ceiling.target_count * 4];
            products.current.clear();
            products.current.push(1.0);
            for j in 1..=ceiling.target_count {
                let target_factors = &g[(j - 1) * 4..j * 4];
                let digit_shift = 2 * (j - 1);
                if j == ceiling.target_count {
                    for ((a, &prefix), &parent) in
                        accumulator.iter_mut().zip(&marks.prefix_indices[j]).zip(&marks.parent_indices[j])
                    {
                        let v =
                            products.current[parent as usize] * target_factors[(prefix >> digit_shift) as usize & 3];
                        if v > *a {
                            *a = v;
                        }
                    }
                } else {
                    products.next.clear();
                    products.next.extend(marks.prefix_indices[j].iter().zip(&marks.parent_indices[j]).map(
                        |(&prefix, &parent)| {
                            products.current[parent as usize] * target_factors[(prefix >> digit_shift) as usize & 3]
                        },
                    ));
                    std::mem::swap(products.current, products.next);
                }
            }
        }
        {
            let leaf_bounds = &mut self.prefix_maxima[ceiling.target_count];
            for (&l, &a) in marks.prefix_indices[ceiling.target_count].iter().zip(&accumulator) {
                let b = ceiling.scale * a;
                leaf_bounds[l as usize] = if b.is_nan() { f64::INFINITY } else { b };
            }
        }
        self.marked_accumulators = accumulator;
        for j in (0..ceiling.target_count).rev() {
            let prefix_count = 1usize << (2 * j);
            let (lo, hi) = self.prefix_maxima.split_at_mut(j + 1);
            let (level, children) = (&mut lo[j], &hi[0]);
            for &p in &marks.prefix_indices[j] {
                let p = p as usize;
                level[p] = children[p]
                    .max(children[prefix_count + p])
                    .max(children[2 * prefix_count + p])
                    .max(children[3 * prefix_count + p]);
            }
        }
    }
    /// per prefix the most bd holds under it (level j: the 4^j prefixes of j digits; prefix p of j digits holds leaves
    /// p + 4^j r: the most of level j + 1's four quarters; no NaN here)
    fn levels(&mut self) {
        for j in (0..self.target_count).rev() {
            let prefix_count = 1usize << (2 * j);
            let mut level = std::mem::take(&mut self.prefix_maxima[j]);
            let children = &self.prefix_maxima[j + 1];
            level.clear();
            level.extend((0..prefix_count).map(|p| {
                children[p]
                    .max(children[prefix_count + p])
                    .max(children[2 * prefix_count + p])
                    .max(children[3 * prefix_count + p])
            }));
            self.prefix_maxima[j] = level;
        }
    }
    /// an anchor's suffix products of its least factors (`suffix_minima`), and whether its bound at the root is below th (the most bd
    /// holds, prefix_maxima[0][0], or a bar at least that: the prefix maxima only fall, so a no now is a no later too); f > 0, g its factors as `run`'s
    fn root_open(&mut self, f: f64, g: &[f64], th: f64) -> bool {
        let (target_count, outfit_count) = (self.target_count, self.outfit_count);
        if target_count == 0 {
            return true;
        }
        let mut lb = 0.0f64;
        for o in 0..outfit_count {
            for j in (0..target_count).rev() {
                let target_factors = &g[(o * target_count + j) * 4..(o * target_count + j) * 4 + 4];
                let mn = target_factors[0].min(target_factors[1]).min(target_factors[2]).min(target_factors[3]);
                self.suffix_minima[o * (target_count + 1) + j] =
                    self.suffix_minima[o * (target_count + 1) + j + 1] * mn;
            }
            let l = self.suffix_minima[o * (target_count + 1)];
            if l > lb {
                lb = l;
            }
        }
        f * BB_SLACK * lb < th
    }
    /// `root_open`'s answer for an anchor (f > 0, chances qa) from its 0-star factors alone (`rise_factors`' expressions;
    /// the same products in the same order; `suffix_minima` is not written, `run` writes it). Premises (the caller's): e, qa, x0 in
    /// [0, 1], xms falling and >= 0, nu > 0. Derived: qn = 1 - x0 xms[s] rises with s (rounding is monotone); a factor is 1
    /// where qn <= qa, else N(qn) / D, D = N(qa) = 1 - (1 - e)(1 - qa) in [0, 1] and N rising with qn: N >= D, so it is
    /// >= 1 (+inf where D = 0, as N > 0 there), never NaN, and rises with qn: each digit's least is its 0-star factor
    fn root_min(&self, f: f64, inputs: &RiseInputs, anchor_chances: &[f64], th: f64) -> bool {
        let target_count = self.target_count;
        let mut lb = 0.0f64;
        for (st, ew) in inputs.outfit_coverage.iter().zip(inputs.event_chances) {
            let mut l = 1.0f64;
            for k in (0..target_count).rev() {
                l *= rise_factor(
                    st[k],
                    ew[k],
                    1.0 - inputs.base_misses[k] * inputs.star_misses[0],
                    anchor_chances[inputs.uncertain_targets[k]],
                );
            }
            if l > lb {
                lb = l;
            }
        }
        f * BB_SLACK * lb < th
    }
    /// fold one anchor in: f > 0, g its factors [outfit][target k][her stars], all > 0
    fn run(&mut self, f: f64, g: &[f64]) {
        let (target_count, outfit_count) = (self.target_count, self.outfit_count);
        if target_count == 0 {
            let v = f * if outfit_count > 0 { 1.0 } else { 0.0 };
            if v < self.prefix_maxima[0][0] {
                self.prefix_maxima[0][0] = v;
            }
            return;
        }
        if !self.root_open(f, g, self.prefix_maxima[0][0]) {
            return;
        }
        for o in 0..outfit_count {
            self.prefix_products[o] = 1.0;
        }
        self.open_roots += 1;
        self.dfs(0, 0, f, f * BB_SLACK, g);
    }
    fn dfs(&mut self, j: usize, p: usize, f: f64, fs: f64, g: &[f64]) {
        let (target_count, outfit_count) = (self.target_count, self.outfit_count);
        self.visited_nodes += 1;
        for stars in 0..4usize {
            let prefix = p | stars << (2 * j);
            for o in 0..outfit_count {
                self.prefix_products[(j + 1) * outfit_count + o] =
                    self.prefix_products[j * outfit_count + o] * g[(o * target_count + j) * 4 + stars];
            }
            if j + 1 == target_count {
                let mut rv = 0.0f64;
                for o in 0..outfit_count {
                    let product = self.prefix_products[target_count * outfit_count + o];
                    if product > rv {
                        rv = product;
                    }
                }
                let v = f * rv;
                let b = &mut self.prefix_maxima[target_count][prefix];
                if v < *b {
                    *b = v;
                }
            } else {
                let mut lb = 0.0f64;
                for o in 0..outfit_count {
                    let l = self.prefix_products[(j + 1) * outfit_count + o]
                        * self.suffix_minima[o * (target_count + 1) + j + 1];
                    if l > lb {
                        lb = l;
                    }
                }
                if fs * lb >= self.prefix_maxima[j + 1][prefix] {
                    continue;
                }
                self.dfs(j + 1, prefix, f, fs, g);
            }
        }
        let children = &self.prefix_maxima[j + 1];
        let s = 1usize << (2 * j);
        self.prefix_maxima[j][p] = children[p].max(children[p + s]).max(children[p + 2 * s]).max(children[p + 3 * s]);
    }
}

/// Prefix marks identify exactly the leaves whose bounds are read. Sparse traversal keeps
/// ascending prefix indices and each prefix's position in the previous level. Dense traversal
/// folds every leaf, then sets unmarked leaves to negative infinity.
struct Marks {
    marked_leaf_count: usize,
    prefix_marks: Vec<Vec<bool>>,
    dense: bool,
    unmarked_leaves: Vec<usize>,
    prefix_indices: Vec<Vec<u32>>,
    parent_indices: Vec<Vec<u32>>,
}

impl Marks {
    fn new(cells: &[usize], target_count: usize, fixed_target_count: usize, leaf_count: usize) -> Self {
        let mut marks = Self {
            prefix_marks: vec![vec![]; target_count + 1],
            dense: false,
            unmarked_leaves: vec![],
            prefix_indices: vec![vec![]; target_count + 1],
            parent_indices: vec![vec![]; target_count + 1],
            marked_leaf_count: 0,
        };
        marks.prefix_marks[target_count] = vec![false; leaf_count];
        for &i in cells {
            marks.prefix_marks[target_count][i >> fixed_target_count] = true;
        }
        for j in (0..target_count).rev() {
            let prefix_count = 1usize << (2 * j);
            let children = &marks.prefix_marks[j + 1];
            marks.prefix_marks[j] = (0..prefix_count)
                .map(|p| {
                    children[p]
                        || children[prefix_count + p]
                        || children[2 * prefix_count + p]
                        || children[3 * prefix_count + p]
                })
                .collect();
        }
        marks.marked_leaf_count = marks.prefix_marks[target_count].iter().filter(|&&b| b).count();
        marks.dense = 4 * marks.marked_leaf_count >= leaf_count;
        if marks.dense {
            marks.unmarked_leaves = (0..leaf_count).filter(|&l| !marks.prefix_marks[target_count][l]).collect();
        } else {
            for j in 0..=target_count {
                marks.prefix_indices[j] =
                    (0..marks.prefix_marks[j].len()).filter(|&p| marks.prefix_marks[j][p]).map(|p| p as u32).collect();
                if j > 0 {
                    // prefix p of j digits: parent p mod 4^(j-1), found in the ascending `prefix_indices[j - 1]`
                    let (parents, prefixes) = (&marks.prefix_indices[j - 1], &marks.prefix_indices[j]);
                    let parent_mask = (1u32 << (2 * (j - 1))) - 1;
                    marks.parent_indices[j] =
                        prefixes.iter().map(|&p| parents.binary_search(&(p & parent_mask)).unwrap() as u32).collect();
                }
            }
        }
        marks
    }
}

/// `rise_leaves`'s factors for chances qa, [outfit][target k][her stars] (the same expressions), into g
fn rise_factors(inputs: &RiseInputs, anchor_chances: &[f64], g: &mut Vec<f64>) {
    let nu = inputs.uncertain_targets.len();
    g.clear();
    for (st, ew) in inputs.outfit_coverage.iter().zip(inputs.event_chances) {
        for k in 0..nu {
            for sg in 0..4 {
                g.push(rise_factor(
                    st[k],
                    ew[k],
                    1.0 - inputs.base_misses[k] * inputs.star_misses[sg],
                    anchor_chances[inputs.uncertain_targets[k]],
                ));
            }
        }
    }
}

/// `outcome_hi` at one outcome (her stars `leaf`, landed set m): each candidate's odds `up` at the most any partner's
/// bound allows (into `up`), then `saving_lo`; `c1` scratch
fn hi_at(
    score_base: &ScoreBase,
    bounds: &[ExactPartnerBound],
    leaf: usize,
    landed_mask: usize,
    own: &OwnFullLanding,
    step_costs: &mut [f64],
    candidate_probabilities: &mut [f64],
) -> f64 {
    candidate_probabilities.copy_from_slice(score_base.pnow);
    for bound in bounds {
        let value = bound.spark_scales[landed_mask] * bound.leaf_bounds[leaf];
        if value > candidate_probabilities[bound.candidate] {
            candidate_probabilities[bound.candidate] = value;
        }
    }
    saving_lo(score_base, candidate_probabilities, landed_mask, own, step_costs)
}

/// V - V_lo at candidate odds `up`: each step that rose at `Cont::cost_lo`, her own 8/8 (`xi`, `her_trios`) on the full
/// set, the goal's V; `c1` scratch
fn saving_lo(
    score_base: &ScoreBase,
    candidate_probabilities: &[f64],
    landed_mask: usize,
    own: &OwnFullLanding,
    step_costs: &mut [f64],
) -> f64 {
    // (the candidates above their threshold, `score_career`'s pnow (1 + 1e-9), in candidate order)
    let lifted: Vec<CandidateLift> = (0..score_base.cands.len())
        .filter(|&candidate| candidate_probabilities[candidate] > score_base.pnow[candidate] * (1.0 + 1e-9))
        .map(|candidate| CandidateLift { slot: candidate, candidate, probability: candidate_probabilities[candidate] })
        .collect();
    step_costs.copy_from_slice(score_base.c1);
    let mut changed_candidates = vec![];
    let mut scratch = StepScratch { costs: step_costs, changed_candidates: &mut changed_candidates };
    v_after(score_base, &mut Lo, &mut scratch, &lifted, landed_mask, own)
}

/// The pick's bound's exact anchors (`outcome_hi`): her best deck searched with her at each of these stars on every
/// uncertain target, for at most HI_SEARCHES / 2 partners (with no limit p16 took 5 and left the same). Callgrind per
/// search: p16 55M, gm16 151M instructions (the val/plan jobs +6.0% and +11.1% in all). A leaf whose stars are all at
/// most 1 is rare at the rank's mix: adding 1 star moved p16's provisional 0.0275 -> 0.0272, gm16 0.0796 -> 0.0742 for
/// +8.8% and +16.5%
const HI_STARS: [usize; 2] = [2, 3];

/// Off by default (not worth the plan's +6-11%); 10 gives the numbers above.
const HI_SEARCHES: usize = 0;

/// ... and only for band outcomes whose partners' bounds are within this factor of p_now: a cost rule, not part of the
/// bound. It skips p10 and p11, whose band needs partners at 2.5 times p_now and more: there 15 searches (at 1, 2 and 3
/// stars) freed nothing (p11: 233 ms native spent), and only 63 (805 ms) moved p10 (0.474 -> 0.378); 1.3 and 1.5 chose
/// the same
const HI_RISE: f64 = 1.5;

/// `outcome_hi` for one (candidate ui, partner y): her slot's miss factors, the bound per leaf (`AnchorBound`), each marked
/// outcome's most of the partners' (`tu`: candidate ui's row of the table). Its own function, entered once per partner,
/// so V8 compiles it optimised after the first few (`docs/PERF.md`, *In the page*)
struct UpperBoundContext<'score, 'base> {
    scoring: &'score ScoringContext<'score, 'base>,
    fixed_parents: &'score [usize],
    uncertain_targets: &'score [usize],
    marked_outcomes: MarkedOutcomes<'score>,
    ceiling_scale: f64,
}

/// Compact outcome-table indexing used for every candidate and partner.
struct MarkedOutcomes<'marks> {
    marks: &'marks Marks,
    cell_indices: &'marks [usize],
    landed_masks: &'marks [u32],
    target_count: usize,
    outcome_count: usize,
    fixed_target_count: usize,
    cell_count: usize,
    marked_cell_count: usize,
}

/// Reused across partners; each call writes every entry it reads.
struct UpperBoundScratch {
    anchor_bound: AnchorBound,
    rise_factors: Vec<f64>,
    ceiling_factors: Vec<f64>,
    leaf_products: Vec<f64>,
    next_leaf_products: Vec<f64>,
    inheritance_chances: Vec<f64>,
    relation_points: Vec<f64>,
}

struct CandidateBoundInputs<'partner> {
    extended_pool: &'partner Pool,
    candidate_index: usize,
    outfit_coverage: &'partner [Vec<u8>],
    event_chances: &'partner [Vec<f64>],
    event_chances_valid: bool,
    anchor_indices: &'partner [usize],
    anchors: &'partner [Option<FoundAnchor>],
}

/// A partner changes within the candidate phase; the candidate's tables stay borrowed.
struct PartnerBoundInputs<'partner, 'inputs> {
    candidate: &'partner CandidateBoundInputs<'inputs>,
    lineage: Lineage,
}

impl<'inputs> CandidateBoundInputs<'inputs> {
    const fn partner(&self, score_base: &ScoreBase, first_parent: usize) -> PartnerBoundInputs<'_, 'inputs> {
        PartnerBoundInputs {
            candidate: self,
            lineage: Lineage {
                trainee: score_base.cands[self.candidate_index],
                first_parent,
                second_parent: self.extended_pool.me.len() - 1,
            },
        }
    }
}

/// Parallel anchor buffers preserve evaluation order. Anchor i owns factors[i * stride..(i + 1) * stride].
#[derive(Default)]
struct AnchorBatch {
    factors: Vec<f64>,
    values: Vec<f64>,
    direct: bool,
    retained: Vec<(f64, Vec<f64>)>,
}

struct AnchorPriority {
    bound_at_two_stars: f64,
    anchor_index: usize,
}

struct ExactPartnerBound {
    candidate: usize,
    leaf_bounds: Vec<f64>,
    spark_scales: Vec<f64>,
}

struct ExactAnchorChances {
    lineage: Lineage,
    initial_chances: Vec<f64>,
    base_misses: Vec<f64>,
    star_misses: [f64; 4],
}

impl ExactAnchorChances {
    /// Borrow outfit and target tables only after the mutable deck search finishes.
    fn rise_inputs<'rise>(&'rise self, environment: &'rise Env) -> RiseInputs<'rise> {
        RiseInputs {
            outfit_coverage: &environment.sts0[self.lineage.trainee],
            event_chances: &environment.ew0[self.lineage.trainee],
            uncertain_targets: &environment.uncv,
            base_misses: &self.base_misses,
            star_misses: &self.star_misses,
        }
    }
}

struct PartnerBoundOutput<'output> {
    candidate_bounds: &'output mut [f64],
    exact_bounds: &'output mut Vec<ExactPartnerBound>,
    exact_chances: &'output mut Vec<ExactAnchorChances>,
    retained_anchors: &'output mut Option<Vec<(f64, Vec<f64>)>>,
}

/// One (candidate, partner)'s anchors as `hi_partner` keeps them for the next call of the same pick (a roll's: none until
/// its first roll): (candidate, partner), and per anchor (f, chances) that can lower some leaf, in their order
pub(in crate::deck::career::sim::plan) struct PartnerAnchors {
    /// Candidate whose odds these anchors bound.
    pub(in crate::deck::career::sim::plan) candidate: CandidateIndex,
    /// Partner's index in the extended legacy pool.
    pub(in crate::deck::career::sim::plan) partner: usize,
    /// Retained (factor, inheritance chances) anchors, in their original order.
    pub(in crate::deck::career::sim::plan) anchors: Option<Vec<(f64, Vec<f64>)>>,
}

/// A fully checked career used as an exact anchor for its deck class.
pub(in crate::deck::career::sim::plan) struct FoundAnchor {
    /// Deck class shared by trainees with the same legal deck search.
    pub(in crate::deck::career::sim::plan) deck_class: usize,
    /// Inheritance chances at the exact check, in job target order.
    pub(in crate::deck::career::sim::plan) inheritance_chances: Vec<f64>,
    /// Best deck value at those chances and unit spark scale.
    pub(in crate::deck::career::sim::plan) value: f64,
}

/// Extra immutable inputs to the bound on a career's outcomes.
pub(in crate::deck::career::sim::plan) struct OutcomeBounds<'bounds> {
    /// Parent indices required by the job.
    pub(in crate::deck::career::sim::plan) fixed_parents: &'bounds [usize],
    /// Fully checked main-search careers, indexed in found-career order.
    pub(in crate::deck::career::sim::plan) found_anchors: &'bounds [Option<FoundAnchor>],
    /// Trio sides contributed by this career's own full veteran.
    pub(in crate::deck::career::sim::plan) own_trios: &'bounds [OwnTrio],
}

/// A cell's probability and its landed job-target bits are distinct from its cell index.
struct OutcomeChance {
    probability: f64,
    landed_mask: usize,
}

/// One outcome and its estimated saving, before bounding that same outcome.
#[derive(Clone, Copy)]
pub(in crate::deck::career::sim::plan) struct OutcomeEstimate {
    /// Cell built from uncertain stars and the fixed-target landing mask.
    pub(in crate::deck::career::sim::plan) index: OutcomeIndex,
    /// Expected careers saved at this outcome.
    pub(in crate::deck::career::sim::plan) saving: f64,
}

#[inline(never)]
#[allow(clippy::absurd_extreme_comparisons, reason = "HI_SEARCHES is a tuning constant, now 0 (anchors off)")]
fn hi_partner(
    simulator: &Sim,
    context: &UpperBoundContext,
    partner: &PartnerBoundInputs,
    scratch: &mut UpperBoundScratch,
    output: &mut PartnerBoundOutput,
) {
    let candidate = partner.candidate;
    // her slot's miss factors and spark scales (`Env::her_slot`, as `score_career`'s setups)
    let Some(slot) = simulator.env.her_slot(candidate.extended_pool, partner.lineage) else {
        return;
    };
    let rise =
        RiseInputs::for_slot(candidate.outfit_coverage, candidate.event_chances, context.uncertain_targets, &slot);
    // the best outfit's product per leaf: per outfit and target the factor A - B (1 - q') at each of her four star
    // levels, 1 - q' = x0 xm (`outfit_fast` at her chances folded in, as `Sim::fast_ub` takes it)
    let ceiling_started = now_ms();
    scratch.ceiling_factors.clear();
    for o in &simulator.fastc[partner.lineage.trainee] {
        for k in 0..context.uncertain_targets.len() {
            let (intercept, miss_coefficient) = o[k];
            for sg in 0..4 {
                scratch.ceiling_factors.push(intercept - miss_coefficient * (slot.x0[k] * slot.xms[sg]));
            }
        }
    }
    // the anchored rise bound per leaf: the least over today's pairs (y, z) keeping the places of
    // (pnow / c_yz) rise(q'(leaf), q_yz), q'_k(leaf) = 1 - x0[k] xm[her stars], and over the main search's proven
    // careers of her deck class (E_best(q_f) = val_f there: val_f rise(q', q_f)); at most the ceiling too (under
    // EXACT_RHO): bd = min(rf ceil, anc) per leaf, by `AnchorBound` (anchors in any order: a min)
    let ceiling = CeilingFactors {
        values: &scratch.ceiling_factors,
        target_count: context.uncertain_targets.len(),
        outfit_count: simulator.fastc[partner.lineage.trainee].len(),
        scale: context.ceiling_scale,
    };
    let mut products =
        LeafProductWorkspace { current: &mut scratch.leaf_products, next: &mut scratch.next_leaf_products };
    scratch.anchor_bound.reset(candidate.outfit_coverage.len(), &ceiling, context.marked_outcomes.marks, &mut products);
    let anchors_started = now_ms();
    // the anchors whose bound at the root is open (f, factors), and whether any went in whole
    let mut batch = AnchorBatch::default();
    // (`AnchorBound::root_min`'s premises but the anchor's chances)
    let monotone_factors = candidate.event_chances_valid
        && !context.uncertain_targets.is_empty()
        && slot.xms[0] >= slot.xms[1]
        && slot.xms[1] >= slot.xms[2]
        && slot.xms[2] >= slot.xms[3]
        && slot.xms[3] >= 0.0
        && slot.x0.iter().all(|v| (0.0..=1.0).contains(v));
    // The most the ceiling holds at any leaf (`AnchorBound::reset`'s bd at the leaf of each digit's largest factor: per outfit the
    // product in the leaves' order from 1.0, the most over outfits from 0.0, rf x it; with every factor >= 0 rounding is
    // monotone, so no leaf's is above it), +inf where a factor is negative or NaN. An anchor whose root test fails against
    // it fails at every set of marked leaves (the test's own bar, the most bd holds at the marked leaves, is at most it):
    // the anchors kept for later calls (`retained_anchors`, a roll's: the same pair, pool and odds now at every outcome) are the others,
    // in their order, so a later call folds in the same anchors as computing them all would, minus ones that lower nothing
    let ceiling_maximum = {
        let (no, mut acc, mut ok) = (simulator.fastc[partner.lineage.trainee].len(), 0.0f64, true);
        for o in 0..no {
            let mut p = 1.0f64;
            for k in 0..context.uncertain_targets.len() {
                let g = &scratch.ceiling_factors[(o * context.uncertain_targets.len() + k) * 4..][..4];
                ok &= g.iter().all(|v| *v >= 0.0);
                p *= g[0].max(g[1]).max(g[2]).max(g[3]);
            }
            if p > acc {
                acc = p;
            }
        }
        let b = context.ceiling_scale * acc;
        if ok && !b.is_nan() { b } else { f64::INFINITY }
    };
    let building_cache = output.retained_anchors.is_none();
    let mut add = |anchor_bound: &mut AnchorBound, f: f64, anchor_chances: &[f64], batch: &mut AnchorBatch| {
        if f > 0.0
            && monotone_factors
            && context.uncertain_targets.iter().all(|&i| (0.0..=1.0).contains(&anchor_chances[i]))
        {
            // every factor >= 1, each digit's least its 0-star one: the root test on those alone, the rest only
            // for an anchor it keeps (as the test below would: all factors > 0, `root_open` the same answer)
            if building_cache {
                if !anchor_bound.root_min(f, &rise, anchor_chances, ceiling_maximum) {
                    return;
                }
                batch.retained.push((f, anchor_chances.to_vec()));
            }
            if !anchor_bound.root_min(f, &rise, anchor_chances, anchor_bound.prefix_maxima[0][0]) {
                return;
            }
            rise_factors(&rise, anchor_chances, &mut scratch.rise_factors);
            debug_assert!(
                scratch.rise_factors.iter().all(|&v| v > 0.0)
                    && anchor_bound.root_open(f, &scratch.rise_factors, anchor_bound.prefix_maxima[0][0]),
                "a candidate kept for a rise has positive factors and an open root"
            );
            batch.values.push(f);
            batch.factors.extend_from_slice(&scratch.rise_factors);
            return;
        }
        rise_factors(&rise, anchor_chances, &mut scratch.rise_factors);
        if f > 0.0 && scratch.rise_factors.iter().all(|&v| v > 0.0) {
            if building_cache {
                if !anchor_bound.root_open(f, &scratch.rise_factors, ceiling_maximum) {
                    return;
                }
                batch.retained.push((f, anchor_chances.to_vec()));
            }
            if anchor_bound.root_open(f, &scratch.rise_factors, anchor_bound.prefix_maxima[0][0]) {
                batch.values.push(f);
                batch.factors.extend_from_slice(&scratch.rise_factors);
            }
        } else {
            // outside the bound's premises: every leaf, as before
            if building_cache {
                batch.retained.push((f, anchor_chances.to_vec()));
            }
            batch.direct = true;
            let leaf_rises =
                rise_leaves(&rise, anchor_chances, &mut scratch.leaf_products, &mut scratch.next_leaf_products);
            for (av, rv) in anchor_bound.prefix_maxima[anchor_bound.target_count].iter_mut().zip(&leaf_rises) {
                let v = f * rv;
                if v < *av {
                    *av = v;
                }
            }
        }
    };
    if let Some(ks) = output.retained_anchors.as_ref() {
        for (f, anchor_chances) in ks {
            add(&mut scratch.anchor_bound, *f, anchor_chances, &mut batch);
        }
    } else {
        if context.scoring.score_base.pnow[candidate.candidate_index] > 0.0 {
            let mut legal_anchors = 0usize;
            for lineage in (0..context.scoring.legacies.len())
                .map(|second_parent| Lineage { second_parent, ..partner.lineage })
                .filter(|&lineage| {
                    legal_pair(simulator.d, context.scoring.pool, lineage)
                        && holds_fix(context.fixed_parents, lineage.first_parent, lineage.second_parent)
                })
            {
                let anchor_scale = simulator.env.lineage_qc(
                    context.scoring.pool,
                    lineage,
                    context.uncertain_targets,
                    &mut scratch.inheritance_chances,
                    &mut scratch.relation_points,
                );
                if anchor_scale <= 0.0 {
                    continue;
                }
                legal_anchors += 1;
                add(
                    &mut scratch.anchor_bound,
                    context.scoring.score_base.pnow[candidate.candidate_index] / anchor_scale,
                    &scratch.inheritance_chances,
                    &mut batch,
                );
            }
            if legal_anchors > 0 {
                prof("hi_z", legal_anchors as f64);
            }
        }
        for &i in candidate.anchor_indices {
            let FoundAnchor { inheritance_chances: anchor_chances, value: anchor_value, .. } =
                candidate.anchors[i].as_ref().unwrap();
            add(&mut scratch.anchor_bound, *anchor_value, anchor_chances, &mut batch);
        }
        prof("hi_kept", batch.retained.len() as f64);
        *output.retained_anchors = Some(batch.retained);
    }
    if batch.direct {
        scratch.anchor_bound.levels();
    }
    // the likeliest lowest first (f x the best outfit's product at 2 stars on every target): only speed
    let factor_stride = candidate.outfit_coverage.len() * context.uncertain_targets.len() * 4;
    let mut anchor_order: Vec<AnchorPriority> = batch
        .values
        .iter()
        .enumerate()
        .map(|(i, &f)| {
            let gi = &batch.factors[i * factor_stride..(i + 1) * factor_stride];
            AnchorPriority {
                bound_at_two_stars: f
                    * (0..candidate.outfit_coverage.len())
                        .map(|o| {
                            (0..context.uncertain_targets.len())
                                .map(|k| gi[(o * context.uncertain_targets.len() + k) * 4 + 2])
                                .product::<f64>()
                        })
                        .fold(0.0f64, f64::max),
                anchor_index: i,
            }
        })
        .collect();
    anchor_order.sort_by(|p, q| {
        p.bound_at_two_stars.total_cmp(&q.bound_at_two_stars).then(p.anchor_index.cmp(&q.anchor_index))
    });
    let fold_started = now_ms();
    let nodes_before = scratch.anchor_bound.visited_nodes;
    for priority in &anchor_order {
        let i = priority.anchor_index;
        scratch.anchor_bound.run(batch.values[i], &batch.factors[i * factor_stride..(i + 1) * factor_stride]);
    }
    prof("hi_nodes", (scratch.anchor_bound.visited_nodes - nodes_before) as f64);
    prof("hi_open", batch.values.len() as f64);
    prof("hi_roots", scratch.anchor_bound.open_roots as f64);
    scratch.anchor_bound.open_roots = 0;
    let emit_started = now_ms();
    // the spark scale per landed set m: every m's when the marked cells are as many (or the exact anchors keep them), else each
    // marked cell's own (the same product)
    let scale_for_mask =
        |m: usize| -> f64 { (0..context.marked_outcomes.target_count).map(|k| slot.pw[k][(m >> k) & 1]).product() };
    let spark_scales: Vec<f64> =
        if HI_SEARCHES > 0 || context.marked_outcomes.marked_cell_count >= context.marked_outcomes.outcome_count {
            (0..context.marked_outcomes.outcome_count).map(&scale_for_mask).collect()
        } else {
            vec![]
        };
    // each outcome's most of the partners' sc[m] bd[leaf] (from pnow; in partner order per candidate, as `hi_at`)
    let leaf_bounds = scratch.anchor_bound.leaf_bounds();
    if context.marked_outcomes.marked_cell_count == context.marked_outcomes.cell_count
        && context.marked_outcomes.fixed_target_count == 0
    {
        for ((t, &m), &b) in
            output.candidate_bounds.iter_mut().zip(context.marked_outcomes.landed_masks).zip(leaf_bounds)
        {
            let v = spark_scales[m as usize] * b;
            if v > *t {
                *t = v;
            }
        }
    } else {
        for ((t, &i), &m) in output
            .candidate_bounds
            .iter_mut()
            .zip(context.marked_outcomes.cell_indices)
            .zip(context.marked_outcomes.landed_masks)
        {
            let v = (if spark_scales.is_empty() { scale_for_mask(m as usize) } else { spark_scales[m as usize] })
                * leaf_bounds[i >> context.marked_outcomes.fixed_target_count];
            if v > *t {
                *t = v;
            }
        }
    }
    prof("hi_ceil_ms", anchors_started - ceiling_started);
    prof("hi_anc_ms", fold_started - anchors_started);
    prof("hi_bb_ms", emit_started - fold_started);
    prof("hi_fold_ms", now_ms() - emit_started);
    prof("hi_pairs", 1.0);
    // (both read only by the exact anchors, HI_SEARCHES > 0)
    if HI_SEARCHES > 0 {
        output.exact_bounds.push(ExactPartnerBound {
            candidate: candidate.candidate_index,
            leaf_bounds: scratch.anchor_bound.leaf_bounds().to_vec(),
            spark_scales,
        });
        output.exact_chances.push(ExactAnchorChances {
            lineage: partner.lineage,
            initial_chances: slot.q0,
            base_misses: slot.x0,
            star_misses: slot.xms,
        });
    }
}

/// The pick's upper bound at the outcomes wanted (a roll's `y_hi`, `roll_answer`): the most V can fall through her
/// veteran's DIRECT lifts. The outcomes, base and own-8/8 credit (`her_trios`) are `score_career`'s; but each stage
/// candidate's odds with her as a parent, with EVERY legal partner y (not the table value of her IDX_PARTNERS best), are
/// bounded from the candidate's exact best now (pnow): every pair (y, z) of today's pool that keeps the plan's places is
/// a career of hers worth at most pnow, so (`Env::rise`: E(q') <= E(q) x rise(q', q) for every deck)
///     value(y, her) = c' E_best(q') <= c' (pnow / c_yz) rise(q', q_yz),
/// the least over z (the veteran most like her: rise 1 when one already carries what she does), and over the main
/// search's found careers f of her deck class that it fully checked (a `Ref` at q_f: every deck of every classmate is
/// worth at most val_f there, `Env::seen`, under the caps every search assumes; the found careers are the search's
/// answer, so the anchors depend on the job alone)
///     value(y, her) <= c' val_f rise(q', q_f);
/// and, for the partners behind the band's likeliest outcomes (y < 0.5 <= the bound so far; `HI_SEARCHES`, `HI_RISE`),
/// over her best deck searched at her s = 2, 3 stars on every uncertain target (q^s, `best_deck` fully checked: E_best
/// there, under the caps every search assumes)
///     value(y, her) <= c' E_best(q^s) rise(q', q^s),
/// rise 1 where her stars are at most s (the band's outcomes are bounded again after); also at most the ceiling
/// (`Env::ceiling` through `Sim::fastc`, under EXACT_RHO: per outfit and target A - B (1 - q'), `outfit_fast`'s factor at
/// q'; before 2026-10-09 the bound took A + B q', B more per target, valid and on p16 3.3 times as high at the pick's
/// partners). A candidate's step costs the lowest cost at those odds
/// (`Cont::cost_lo`). Derived, per outcome:
/// (1) For each partner, the bound >= the best career's exact value >= the table value `score_career` reads. A cached
///     deck's corner table is multilinear in the targets' miss chances, as P(all) of a fixed deck is (each chance enters
///     one target's factor linearly), so its interpolation is that deck's exact value; a real deck is at most the best.
///     Her best partners are among every partner. So up_hi >= up for every candidate.
/// (2) cost_lo(up_hi) <= cost(up) (`Cont::cost_lo`), a step cost never rises from the base, and V over the trios (and
///     `with_her`) does not fall when a step cost rises; so V_lo <= V_table and y_hi >= y.
/// (3) Each anchor A (a pair (y, z), a found career, a search at q^s) has a value E_A with E_best(q_A) <= E_A (a career
///     of today's pool is worth at most pnow; a fully checked deck search is the best under the caps). Every deck D legal
///     for the candidate has E_D(q') <= E_D(q_A) rise(q', q_A) (`Env::rise`) <= E_A rise(q', q_A), so her best career
///     with partner y is at most c' E_A rise(q', q_A) for every A, and at most its ceiling. In the exact replan with her
///     in the pool, with today's Cont and trios, her step then costs at least cost_lo(up_hi): y_hi bounds the direct-lift
///     saving. Not bounded (channel 2, out of scope by design): what the replan's refits do (Cont fitted
///     again, other careers' best careers with her as a parent, the stage candidates chosen again beyond her own 8/8).
pub(in crate::deck::career::sim::plan) fn outcome_hi(
    simulator: &mut Sim,
    context: &ScoringContext,
    career: Career,
    bounds: &OutcomeBounds,
    outcome: OutcomeEstimate,
    anchor_cache: &mut Vec<PartnerAnchors>,
) -> f64 {
    let data = simulator.d;
    let trainee = career.lineage.trainee;
    let want = &[outcome];

    let candidate_count = context.score_base.cands.len();
    let target_count = simulator.env.ntar;
    let uncertain_target_count = simulator.env.uncv.len();
    let star_mix = simulator.env.sj.star_dist;
    let (own_candidate, completes_goal) = her_ends(simulator, context.score_base, context.legacies, career.lineage);
    let current = simulator.make_cur(context.pool, career, false);
    let extended_pool = her_pool(simulator, context.legacies, context.pool, true, career.lineage, &current);
    let uncertain_targets = &simulator.env.uncv;
    let new_veteran = extended_pool.me.len() - 1;
    // per candidate and EVERY legal partner: her slot's miss factors and the spark scale, as `score_career`'s setups
    let leaf_count = 1usize << (2 * uncertain_target_count);
    let outcome_count = 1usize << target_count;
    // (candidate, bound at scale 1 per leaf, spark scale per m): kept only for the exact anchors (`HI_SEARCHES` > 0); else
    // each outcome reads `tab`, the same maxima folded in as each partner is done
    let mut exact_bounds: Vec<ExactPartnerBound> = vec![];
    let mut exact_chances: Vec<ExactAnchorChances> = vec![];
    let ceiling_scale = simulator.env.space.rho_top() * simulator.fixed;
    let (leaf_products_buffer, next_leaf_products): (Vec<f64>, Vec<f64>) = (vec![], vec![]);
    // an outcome (her stars `leaf`, the fixed targets landed fm: cell (leaf << nf) | fm): its chance and landed set
    let fixed_target_count = simulator.env.fixed_t.len();
    let cell_count = leaf_count << fixed_target_count;
    // (`score_career`'s cells: the landed set of her stars' leaf and the fixed targets' fm, its chance os[m] x her stars')
    let fixed_landed_masks = fm_masks(&simulator.env.fixed_t);
    let cell = |i: usize| {
        let outcome = leaf_set(i >> fixed_target_count, uncertain_targets, star_mix);
        let m = outcome.landed_mask | fixed_landed_masks[i & ((1usize << fixed_target_count) - 1)].bits();
        OutcomeChance { probability: current.os[m] * outcome.star_probability, landed_mask: m }
    };
    // the outcomes whose bound is read (`want`, ascending, each with its y < 0.5: the page reads y_hi only there) that
    // can happen; elsewhere the bound is not computed (no cell's value depends on another's: each is the most over
    // partners of a per-leaf least, computed alone)
    let requested_chances: Vec<OutcomeChance> = want.iter().map(|estimate| cell(estimate.index.0)).collect();
    let marked_cells: Vec<usize> = want
        .iter()
        .zip(&requested_chances)
        .filter(|(_, chance)| chance.probability > 0.0)
        .map(|(estimate, _)| estimate.index.0)
        .collect();
    let marks = Marks::new(&marked_cells, uncertain_target_count, fixed_target_count, leaf_count);
    prof("hi_cells", marked_cells.len() as f64);
    prof("hi_mleaves", marks.marked_leaf_count as f64);
    if marked_cells.is_empty() {
        return 0.0;
    }
    // the cells the table holds: the marked ones, or with most leaves marked every possible one, folded as before
    // (vectorised where every cell is possible; an unmarked leaf's bound is -inf, so its cells keep p_now, never read)
    let marked_cells: Vec<usize> =
        if marks.dense { (0..cell_count).filter(|&i| cell(i).probability > 0.0).collect() } else { marked_cells };
    let landed_masks: Vec<usize> = marked_cells.iter().map(|&i| cell(i).landed_mask).collect();
    let compact_masks: Vec<u32> = landed_masks.iter().map(|&m| m as u32).collect();
    let marked_cell_count = marked_cells.len();
    // per candidate and marked outcome her odds' bound so far: pnow, then the most of every partner's (`hi_at` without the exact-anchor tables)
    let mut candidate_bounds: Vec<f64> = (0..candidate_count)
        .flat_map(|candidate| std::iter::repeat_n(context.score_base.pnow[candidate], marked_cell_count))
        .collect();
    // an anchor's chances (`Env::lineage_qc`: only the uncertain targets' entries, the ones the factors read)
    let mut scratch = UpperBoundScratch {
        anchor_bound: AnchorBound::new(),
        rise_factors: vec![],
        ceiling_factors: vec![],
        leaf_products: leaf_products_buffer,
        next_leaf_products,
        inheritance_chances: vec![0.0f64; target_count],
        relation_points: vec![],
    };
    let upper_bound = UpperBoundContext {
        scoring: context,
        fixed_parents: bounds.fixed_parents,
        uncertain_targets,
        marked_outcomes: MarkedOutcomes {
            marks: &marks,
            cell_indices: &marked_cells,
            landed_masks: &compact_masks,
            target_count,
            outcome_count,
            fixed_target_count,
            cell_count,
            marked_cell_count,
        },
        ceiling_scale,
    };
    // (each (candidate, partner)'s anchors as a call keeps them for the next, `acache`: in this loop's order)
    let mut partner_count = 0usize;
    for candidate in 0..candidate_count {
        let candidate_trainee = context.score_base.cands[candidate];
        if data.umas[candidate_trainee].id == data.umas[trainee].id
            || context.score_base.c1[candidate] == 0.0
            || simulator.fastc[candidate_trainee].is_empty()
        {
            continue;
        }
        // her outfits' skill coverage and own chances (as `Env::rise` reads them)
        let (outfit_coverage, event_chances) =
            (&simulator.env.sts0[candidate_trainee], &simulator.env.ew0[candidate_trainee]);
        scratch.relation_points.clear(); // (`Env::lineage_qc`'s table: another trainee)
        // the main search's found careers of her deck class that it fully checked (`found_anchors`)
        let deck_class = simulator.env.cls[candidate_trainee];
        let anchor_indices: Vec<usize> = (0..bounds.found_anchors.len())
            .filter(|&i| bounds.found_anchors[i].as_ref().is_some_and(|f| f.deck_class == deck_class))
            .collect();
        // `AnchorBound::root_min`'s premises on her own chances
        let event_chances_valid = event_chances.iter().all(|ew| ew.iter().all(|e| (0.0..=1.0).contains(e)));
        let candidate_inputs = CandidateBoundInputs {
            extended_pool: &extended_pool,
            candidate_index: candidate,
            outfit_coverage,
            event_chances,
            event_chances_valid,
            anchor_indices: &anchor_indices,
            anchors: bounds.found_anchors,
        };
        for partner in 0..new_veteran {
            let partner_inputs = candidate_inputs.partner(context.score_base, partner);
            if !legal_pair(data, &extended_pool, partner_inputs.lineage) {
                continue;
            }
            if anchor_cache.len() <= partner_count {
                anchor_cache.push(PartnerAnchors { candidate: CandidateIndex(candidate), partner, anchors: None });
            }
            if anchor_cache[partner_count].candidate != CandidateIndex(candidate)
                || anchor_cache[partner_count].partner != partner
            {
                anchor_cache[partner_count] =
                    PartnerAnchors { candidate: CandidateIndex(candidate), partner, anchors: None };
            }
            let mut partner_output = PartnerBoundOutput {
                candidate_bounds: &mut candidate_bounds
                    [candidate * marked_cell_count..(candidate + 1) * marked_cell_count],
                exact_bounds: &mut exact_bounds,
                exact_chances: &mut exact_chances,
                retained_anchors: &mut anchor_cache[partner_count].anchors,
            };
            hi_partner(simulator, &upper_bound, &partner_inputs, &mut scratch, &mut partner_output);
            partner_count += 1;
        }
    }
    her_pool_done(simulator, context.pool, true, extended_pool);
    let UpperBoundScratch { leaf_products: mut leaf_products_buffer, mut next_leaf_products, .. } = scratch;
    let own = OwnFullLanding {
        candidate_index: own_candidate,
        completes_goal,
        trios: bounds.own_trios,
        full_mask: outcome_count - 1,
    };
    // per wanted cell its bound (0 where it can't happen), from its place among the cells held (`mc`, ascending)
    let mut savings = [0.0];
    let at: Vec<Option<usize>> = want
        .iter()
        .zip(&requested_chances)
        .map(
            |(estimate, chance)| {
                if chance.probability > 0.0 { marked_cells.binary_search(&estimate.index.0).ok() } else { None }
            },
        )
        .collect();
    let mut step_costs = context.score_base.c1.to_vec();
    let mut candidate_probabilities = vec![0.0f64; candidate_count];
    let savings_started = now_ms();
    for (k, &j) in at.iter().enumerate() {
        let Some(j) = j else { continue };
        for candidate in 0..candidate_count {
            candidate_probabilities[candidate] = candidate_bounds[candidate * marked_cell_count + j];
        }
        savings[k] = saving_lo(context.score_base, &candidate_probabilities, landed_masks[j], &own, &mut step_costs);
    }
    drop(candidate_bounds);
    prof("hi_ys_ms", now_ms() - savings_started);
    // exact anchors (`HI_SEARCHES`) for the partners that hold the most likely outcomes of the band (y < 0.5 <= y_hi) in
    // it: per band outcome, most likely first, the partners that must all fall for it to leave the band (every partner
    // above p_now of each candidate whose odds move V; none further than HI_RISE above p_now), taken whole while they fit
    let mut band: Vec<usize> =
        (0..want.len()).filter(|&k| at[k].is_some() && savings[k].max(want[k].saving) >= 0.5).collect();
    band.sort_by(|&i, &j| {
        requested_chances[j]
            .probability
            .partial_cmp(&requested_chances[i].probability)
            .unwrap()
            .then(want[i].index.0.cmp(&want[j].index.0))
    });
    let search_limit = HI_SEARCHES / HI_STARS.len();
    let mut chosen: Vec<usize> = vec![];
    for &k in &band {
        if chosen.len() >= search_limit {
            break;
        }
        let (leaf, m) = (want[k].index.0 >> fixed_target_count, requested_chances[k].landed_mask);
        let h = hi_at(context.score_base, &exact_bounds, leaf, m, &own, &mut step_costs, &mut candidate_probabilities);
        let (mut need, mut far): (Vec<usize>, bool) = (vec![], false);
        for candidate in 0..candidate_count {
            if candidate_probabilities[candidate] <= context.score_base.pnow[candidate] * (1.0 + 1e-9) {
                continue;
            }
            let keep = candidate_probabilities[candidate];
            candidate_probabilities[candidate] = context.score_base.pnow[candidate];
            let h2 = saving_lo(context.score_base, &candidate_probabilities, m, &own, &mut step_costs);
            candidate_probabilities[candidate] = keep;
            if h2 >= h - 1e-9 {
                continue;
            }
            for (partner_index, bound) in exact_bounds.iter().enumerate() {
                if bound.candidate != candidate {
                    continue;
                }
                let v = bound.spark_scales[m] * bound.leaf_bounds[leaf];
                if v > context.score_base.pnow[candidate] * (1.0 + 1e-9) {
                    need.push(partner_index);
                    if v > HI_RISE * context.score_base.pnow[candidate] {
                        far = true;
                    }
                }
            }
        }
        if far || need.is_empty() {
            continue;
        }
        need.retain(|partner_index| !chosen.contains(partner_index));
        if chosen.len() + need.len() <= search_limit {
            chosen.extend(need);
        }
    }
    // each one: her best deck searched at her s stars on every uncertain target (q^s, `HI_STARS`), proven (`best_deck`
    // full, under the caps): value(y, her) <= c' E_best(q^s) rise(q', q^s) for every outcome (rise 1 where her stars <= s)
    for &partner_index in &chosen {
        let anchor = &exact_chances[partner_index];
        for &st in &HI_STARS {
            let mut q = anchor.initial_chances.clone();
            for k in 0..uncertain_target_count {
                q[simulator.env.uncv[k]] = 1.0 - anchor.base_misses[k] * anchor.star_misses[st];
            }
            simulator.env.set_trainee(anchor.lineage.trainee, &q);
            simulator.env.work = 0;
            let mut nsc = 0;
            if let DeckRes::Done(v, _, true) = simulator.env.best_deck(anchor.lineage.trainee, 1.0, 0.0, &[], &mut nsc)
            {
                let rise = anchor.rise_inputs(simulator.env);
                let rz = rise_leaves(&rise, &q, &mut leaf_products_buffer, &mut next_leaf_products);
                for (bv, r) in exact_bounds[partner_index].leaf_bounds.iter_mut().zip(&rz) {
                    let v2 = v * r;
                    if v2 < *bv {
                        *bv = v2;
                    }
                }
            }
        }
    }
    if !chosen.is_empty() {
        for &k in &band {
            savings[k] = hi_at(
                context.score_base,
                &exact_bounds,
                want[k].index.0 >> fixed_target_count,
                requested_chances[k].landed_mask,
                &own,
                &mut step_costs,
                &mut candidate_probabilities,
            );
        }
    }
    savings[0]
}

/// The pick's own 8/8 as a trio side (`her_trios`) for `outcome_hi`, taken as the plan ends (the deck cache then:
/// `her_trios` reads C's cached decks), where an outcome that lands every target reads y < 0.5 (`full_lo`: only there
/// does `saving_lo` read it; else none)
pub(in crate::deck::career::sim::plan) fn pick_her(
    s: &mut Sim,
    context: &ScoringContext,
    career: Career,
    full_lo: bool,
    buf: &mut Vec<f64>,
) -> Vec<OwnTrio> {
    let (xi, goal) = her_ends(s, context.score_base, context.legacies, career.lineage);
    if !full_lo || xi.is_some() || goal || context.score_base.sides.is_empty() {
        return vec![];
    }
    let c = s.make_cur(context.pool, career, false);
    let full_m = (1usize << s.env.ntar) - 1;
    if c.os[full_m] <= 0.0 {
        return vec![];
    }
    let mut pv = her_pool(s, context.legacies, context.pool, true, career.lineage, &c);
    let iv = pv.me.len() - 1;
    let r = her_trios(s, context.score_base, &mut pv, iv, career.lineage.trainee, buf);
    her_pool_done(s, context.pool, true, pv);
    r
}

/// `outcome_hi`'s found-career anchors: per found career of the main search `msv` (its order), (her deck class, q_f,
/// val_f) when the search fully checked it (a Ref at the career's own chances: val_f is the best value of any deck at q_f
/// for every trainee of the class, `Env::seen`, scale 1), else none
pub(in crate::deck::career::sim::plan) fn found_anchors(
    env: &Env,
    pool: &Pool,
    msv: &Solved,
) -> Vec<Option<FoundAnchor>> {
    msv.found
        .iter()
        .map(|f| {
            let lineage = Lineage { trainee: f.t, first_parent: f.a, second_parent: f.b };
            let (fq, _) = env.lineage_q(pool, lineage);
            msv.refs.get(f.t).and_then(|rs| rs.iter().find(|r| r.q == fq)).map(|r| FoundAnchor {
                deck_class: env.cls[f.t],
                inheritance_chances: r.q.clone(),
                value: r.val,
            })
        })
        .collect()
}
