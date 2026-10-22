//! The continuation (`Cont`) and its memo: the fitted expected careers to finish after a candidate.
use crate::FxMap;
use crate::career::Legacy;
use crate::career::plan::CandidateIndex;
/// What a career is scored against: per stage-1 candidate her step cost and stages 2-3, V0, the stage-1 odds now, the
/// fitted continuations and the trios. Per-candidate slices follow `candidates`' order.
pub(crate) struct ScoreBase<'base> {
    /// The stage-1 candidates, as umas.
    pub(crate) candidates: &'base [crate::uma::UmaIndex],
    /// Each candidate's stage-1 step in expected careers (c1; 0: her full veteran exists).
    pub(crate) step_costs: &'base [f64],
    /// Each candidate's stages 2-3 in expected careers (m23).
    pub(crate) later_stage_costs: &'base [f64],
    /// V0: the plan's expected careers now.
    pub(crate) current_value: f64,
    /// Each candidate's stage-1 odds now.
    pub(crate) current_odds: &'base [f64],
    /// Each candidate's fitted continuation; a candidate past its end has none (`cost` grinds at p).
    pub(crate) continuations: &'base [Cont],
    /// The trios (A, B as candidate indices, C's step, A and B from a real full veteran of hers): V = min over them of A's
    /// step + B's step + C's step, a side from a real full veteran costing 0, else c1; empty: min over a of c1[a] + m23[a]
    pub(crate) trios: &'base [Trio],
    /// Per candidate, the trios' sides: her real full veterans and the full veteran her best career now would leave; with
    /// `finisher_umas`, what `her_trios` pairs a new real full veteran with. Empty: none
    pub(crate) trio_sides: &'base [Vec<TrioSideVeteran>],
    /// The umas a trio's C may be.
    pub(crate) finisher_umas: &'base [crate::uma::UmaIndex],
}

/// A trio side's veteran as `her_trios` reads it: a real full veteran of the candidate's, or the fresh full veteran her best career now
/// would leave.
pub(crate) struct TrioSideVeteran {
    /// A real full veteran already in the pool (a side that costs no step).
    pub(crate) uses_existing: bool,
    pub(crate) legacy: Legacy,
}

/// Requested scoring outputs. Unrequested numeric results are placeholders, not measured zeroes:
///
/// - Stats: per-candidate lift statistics; expected saving and improvement chance are returned as 0.0.
/// - Score: expected saving and improvement chance; no per-candidate statistics.
/// - Keep: Score plus the prepared scoring needed to evaluate later rolls (`score_career`).
/// - Ys: Score plus a saving for each evaluated outcome (`Scored::evaluate`).
/// - Touch: nothing; the preparation's side effects only (her schedule, the partners' `val` searches, `prepare_scoring`'s
///   counters), its evaluation skipped, zeros returned (the looped rows, whose stats only `proxy_check` reads).
///
/// This selects outputs, not side effects: even Stats can search and populate the deck cache during preparation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Want {
    Stats,
    Score,
    Keep,
    Ys,
    Touch,
}

/// (A, B, C's step, A from a real full veteran, B from a real full veteran)
#[derive(Clone, Copy)]
pub(crate) struct Trio {
    /// First side's position in the candidate table.
    pub(crate) candidate_a: CandidateIndex,
    /// Second side's position in the candidate table.
    pub(crate) candidate_b: CandidateIndex,
    /// Expected careers for the finisher after both sides are full veterans.
    pub(crate) finisher_cost: f64,
    /// The first side already exists as a full veteran.
    pub(crate) a_uses_existing: bool,
    /// The second side already exists as a full veteran.
    pub(crate) b_uses_existing: bool,
}

/// One candidate paired with the new veteran as a full trio side.
#[derive(Clone, Copy)]
pub(crate) struct OwnTrio {
    /// Partner's position in the candidate table.
    pub(crate) candidate: CandidateIndex,
    /// The partner already exists as a full veteran.
    pub(crate) uses_existing: bool,
    /// Expected careers for the finisher after both sides are full veterans.
    pub(crate) finisher_cost: f64,
}

/// V over the trios at step costs `step_costs` (per candidate)
pub(crate) fn trio_v(step_costs: &[f64], combos: &[Trio]) -> f64 {
    // (a plain comparison: no value here is NaN, so the same minimum as f64::min)
    let mut best = f64::INFINITY;
    for trio in combos {
        let a_cost = if trio.a_uses_existing { 0.0 } else { step_costs[trio.candidate_a.0] };
        let b_cost = if trio.b_uses_existing { 0.0 } else { step_costs[trio.candidate_b.0] };
        let value = a_cost + b_cost + trio.finisher_cost;
        if value < best {
            best = value;
        }
    }
    best
}

/// An exact memo of a pure function of its key, owned (through an `Rc`) by the one object whose fixed parameters the
/// function reads; a hit returns the value a miss computed for the same key bits, so outputs are the same by
/// construction. On a miss after MEMO_PROBE lookups, fewer than one hit in MEMO_MIN_HIT disables it.
/// Before an insertion at capacity it is cleared to limit memory; `PLAN_MEMO_CAP` overrides MEMO_CAP.
pub(crate) struct Memo<K, V> {
    map: FxMap<K, V>,
    lookups: u64,
    hits: u64,
    /// Gated off (or off from the start, `PLAN_MEMO_OFF`): every call computes.
    disabled: bool,
    /// Which memo this is, for its counters in `MEMO_STATS`.
    kind: MemoKind,
    /// Entries past which the map is cleared.
    capacity: usize,
    /// `PLAN_MEMO_CHECK`: every hit computed again and compared bit for bit.
    check_hits: bool,
}

/// The memo kinds, by their counters' place in `MEMO_STATS` (lookups, hits and gated-off memos at 3 x kind).
#[derive(Clone, Copy)]
enum MemoKind {
    /// `Cont::plan`'s results.
    Plan = 0,
    /// `Cont::cost_lo`'s results.
    CostLowerBound = 1,
}

/// `MEMO_STATS`' places after the memo kinds': Cont refits and the cap's clears.
const REFITS_STAT: usize = 6;

const CLEARS_STAT: usize = 7;

const MEMO_PROBE: u64 = 4096;

const MEMO_MIN_HIT: u64 = 8;

/// Default entry cap per continuation memo.
const MEMO_CAP: usize = 1 << 14;

/// Bit equality of memoised values for `PLAN_MEMO_CHECK`.
pub(crate) trait SameBits {
    fn same_bits(&self, other: &Self) -> bool;
}

impl SameBits for f64 {
    fn same_bits(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl SameBits for BuildPlan {
    fn same_bits(&self, other: &Self) -> bool {
        self.expected_careers.same_bits(&other.expected_careers)
            && self.building_careers.same_bits(&other.building_careers)
            && self.grind_odds.same_bits(&other.grind_odds)
    }
}

impl<K: std::hash::Hash + Eq, V: Copy + SameBits + std::fmt::Debug> Memo<K, V> {
    // (`PLAN_MEMO_OFF`: every memo off from the start, for the test that compares against no memo)
    // (`PLAN_MEMO_CAP`: another cap, for the test: entries kept long enough to be read after a refit)
    fn new(kind: MemoKind) -> Self {
        let checks = crate::checks::get();
        Self {
            map: FxMap::default(),
            lookups: 0,
            hits: 0,
            disabled: checks.plan_memo_off,
            kind,
            capacity: checks.plan_memo_cap.unwrap_or(MEMO_CAP),
            check_hits: checks.plan_memo_check,
        }
    }
    #[inline]
    fn get_or(&mut self, key: impl FnOnce() -> K, compute: impl FnOnce() -> V) -> V {
        if self.disabled {
            return compute();
        }
        self.lookups += 1;
        let key = key();
        if let Some(&value) = self.map.get(&key) {
            self.hits += 1;
            // (`PLAN_MEMO_CHECK`: every hit computed again and compared bit for bit; tests/plan_memo.rs)
            if self.check_hits {
                let computed = compute();
                assert!(
                    value.same_bits(&computed),
                    "memo kind {}: hit {value:?}, computed {computed:?}",
                    self.kind as usize
                );
            }
            return value;
        }
        let value = compute();
        if self.lookups >= MEMO_PROBE && self.hits * MEMO_MIN_HIT < self.lookups {
            self.disabled = true;
            self.map = FxMap::default();
            return value;
        }
        if self.map.len() >= self.capacity {
            self.map.clear();
            memo_stat(CLEARS_STAT);
        }
        self.map.insert(key, value);
        value
    }
}

// (`PLAN_MEMO`: per plan, each memo kind's lookups, hits and memos gated off, the Cont refits and the cap's clears, on
// stderr)
thread_local! { static MEMO_STATS: std::cell::Cell<[u64; 8]> = const { std::cell::Cell::new([0; 8]) }; }

impl<K, V> Drop for Memo<K, V> {
    fn drop(&mut self) {
        let first = 3 * self.kind as usize;
        MEMO_STATS.with(|stats_cell| {
            let mut stats = stats_cell.get();
            stats[first] += self.lookups;
            stats[first + 1] += self.hits;
            stats[first + 2] += u64::from(self.disabled);
            stats_cell.set(stats);
        });
    }
}

fn memo_stat(place: usize) {
    MEMO_STATS.with(|stats_cell| {
        let mut stats = stats_cell.get();
        stats[place] += 1;
        stats_cell.set(stats);
    });
}

/// Count a Cont refit for `PLAN_MEMO`.
pub(crate) fn count_refit() {
    memo_stat(REFITS_STAT);
}

pub(crate) struct MemoReport;

impl Drop for MemoReport {
    fn drop(&mut self) {
        let stats = MEMO_STATS.with(|stats_cell| stats_cell.replace([0; 8]));
        if crate::checks::get().plan_memo {
            let hit_percent =
                |lookups: u64, hits: u64| if lookups == 0 { 0.0 } else { 100.0 * hits as f64 / lookups as f64 };
            eprintln!(
                "PLAN_MEMO plan lookups {} hits {} ({:.1}%) off {} | cost_lo lookups {} hits {} ({:.1}%) off {} | refits {} clears {}",
                stats[0],
                stats[1],
                hit_percent(stats[0], stats[1]),
                stats[2],
                stats[3],
                stats[4],
                hit_percent(stats[3], stats[4]),
                stats[5],
                stats[REFITS_STAT],
                stats[CLEARS_STAT]
            );
        }
    }
}

/// A step's continuation (form assumed; the rate and step fitted to the pool now, the shape to traces): builder careers
/// lift her odds, each lift closing `phi` of the remaining gap in log odds to `pinf`, her effective ceiling; the lift
/// chance falls with the remaining gap: lam_j = lam ((1 - phi)^j)^LAM_DECAY. pinf = CEIL_SHARE x her full-loop odds.
/// Shape calibration and its limits: docs/MODEL.md, "Next career"; trace setup: val/sim/README.md.
/// lam = 0: no building.
#[derive(Clone)]
pub(crate) struct Cont {
    pub(crate) lam: f64,
    pub(crate) phi: f64,
    pub(crate) pinf: f64,
    /// per n (0..=CONT_N): `plan` reads them instead of evaluating the powers per call (the same expressions, so the
    /// same values). Empty in a default Cont.
    pub(crate) tables: std::rc::Rc<Vec<ContStep>>,
    /// `plan`'s and `cost_lo`'s results by the bits of p. Exact: both read p and lam, phi, pinf and tab alone, which are
    /// kept unchanged after construction (a refit makes new Conts). A clone shares the memos and
    /// carries the same parameters; a new or default Cont gets its own
    pub(crate) memo: std::rc::Rc<ContMemo>,
}

/// One row of a Cont's table, for n successes.
#[derive(Clone, Copy)]
pub(crate) struct ContStep {
    /// The careers built through success n + 1 (the sum `plan` takes, in its order).
    pub(crate) careers_built: f64,
    /// (1 - phi)^n: the share of the log-odds gap to pinf left after n successes.
    pub(crate) gap_left: f64,
}

/// What `Cont::plan` chooses at odds p: build n successes, then grind.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BuildPlan {
    /// Expected careers to her full veteran.
    pub(crate) expected_careers: f64,
    /// Expected careers spent building her n successes (each success takes 1 / lam_j careers): the table's
    /// `careers_built` through success n, 0 with no building. `expected_careers` is this plus 1 / `grind_odds`.
    pub(crate) building_careers: f64,
    /// Her odds once building stops.
    pub(crate) grind_odds: f64,
}

pub(crate) struct ContMemo {
    plan: std::cell::RefCell<Memo<u64, BuildPlan>>,
    cost_lower_bound: std::cell::RefCell<Memo<u64, f64>>,
}

fn cont_memo() -> std::rc::Rc<ContMemo> {
    std::rc::Rc::new(ContMemo {
        plan: std::cell::RefCell::new(Memo::new(MemoKind::Plan)),
        cost_lower_bound: std::cell::RefCell::new(Memo::new(MemoKind::CostLowerBound)),
    })
}

impl Default for Cont {
    fn default() -> Self {
        Self { lam: 0.0, phi: 0.0, pinf: 0.0, tables: std::rc::Rc::default(), memo: cont_memo() }
    }
}

/// `plan`'s cap on successes before grinding
const CONT_N: usize = 500;

/// Effective ceiling as a share of the full-loop odds; the following exponent controls the lift chance's decline.
/// Calibration: docs/MODEL.md, "Next career" (`CEIL_SHARE`, `LAM_DECAY`).
pub(crate) const CEIL_SHARE: f64 = 0.6;

const LAM_DECAY: f64 = 1.5;

impl Cont {
    pub(crate) fn new(lam: f64, phi: f64, pinf: f64) -> Self {
        let r = 1.0 - phi;
        let mut tables = Vec::with_capacity(CONT_N + 1);
        let mut built = 0.0f64;
        for n in 0..=CONT_N as i32 {
            let careers_built = built + 1.0 / (lam * r.powi(n).powf(LAM_DECAY));
            tables.push(ContStep { careers_built, gap_left: r.powi(n) });
            built = careers_built;
        }
        Self { lam, phi, pinf, tables: std::rc::Rc::new(tables), memo: cont_memo() }
    }
    /// The best plan from odds p: build n successes (sum of 1 / lam_j careers), then grind at p_n; the best n
    pub(crate) fn plan(&self, p: f64) -> BuildPlan {
        if p <= 0.0 {
            return BuildPlan { expected_careers: f64::INFINITY, building_careers: 0.0, grind_odds: p };
        }
        if self.lam <= 0.0 || self.phi <= 0.0 || self.pinf <= p {
            return BuildPlan { expected_careers: 1.0 / p, building_careers: 0.0, grind_odds: p };
        }
        self.memo.plan.borrow_mut().get_or(|| p.to_bits(), || self.plan_n(p))
    }
    /// `plan` past its early returns (the first stop of the building loop). Past them lam > 0, which only `Cont::new`
    /// makes (with its table; `rolls_in` refuses a stored Cont with lam and phi > 0 but no table)
    fn plan_n(&self, p: f64) -> BuildPlan {
        debug_assert!(self.tables.len() > CONT_N, "plan_n needs the continuation's table");
        let lr = (p / self.pinf).ln();
        let t = &self.tables;
        let (mut n, mut best) = (0usize, 1.0 / p);
        while n < CONT_N {
            let x = t[n].careers_built + 1.0 / (self.pinf * (lr * t[n + 1].gap_left).exp());
            if x >= best {
                break;
            }
            best = x;
            n += 1;
        }
        // (row n - 1 holds the careers built through success n, the sum `best` took)
        let building_careers = if n == 0 { 0.0 } else { t[n - 1].careers_built };
        BuildPlan { expected_careers: best, building_careers, grind_odds: self.pinf * (lr * t[n].gap_left).exp() }
    }
    pub(crate) fn cost(&self, p: f64) -> f64 {
        self.plan(p).expected_careers
    }
    /// a lower bound on `cost` at every odds up to p: the minimum over every n (not the first local one `plan` stops at).
    /// Each n's cost B_n + 1 / p_n falls as p rises (p_n = pinf exp(ln(p / pinf) r^n) rises with p), so this minimum falls
    /// with p, and at any p' <= p it is at most the minimum at p', which is at most `cost(p')`
    fn cost_lo(&self, p: f64) -> f64 {
        if p <= 0.0 {
            return f64::INFINITY;
        }
        if self.lam <= 0.0 || self.phi <= 0.0 || self.pinf <= p {
            return 1.0 / p;
        }
        self.memo.cost_lower_bound.borrow_mut().get_or(|| p.to_bits(), || self.cost_lo_n(p))
    }
    /// `cost_lo` past its early returns (every n: CONT_N exp; `outcome_hi` asks for it per outcome and candidate; the
    /// table as in `plan_n`)
    fn cost_lo_n(&self, p: f64) -> f64 {
        debug_assert!(self.tables.len() > CONT_N, "cost_lo_n needs the continuation's table");
        let lr = (p / self.pinf).ln();
        let mut best = 1.0 / p;
        // every n's cost is its building (rising with n: each term 1/lam_j > 0) plus a positive grind, so once the
        // building alone reaches the best found no later n can be lower: the scan stops there, the same minimum
        let t = &self.tables;
        for n in 0..CONT_N {
            if t[n].careers_built >= best {
                break;
            }
            best = best.min(t[n].careers_built + 1.0 / (self.pinf * (lr * t[n + 1].gap_left).exp()));
        }
        best
    }
    /// Whether `cost_lb` holds: `plan`'s path past its early returns at every p in (1e-250 pinf, pinf) (lam > 0, 0 < phi <
    /// 1, pinf finite, the table built: `careers_built` nondecreasing, `gap_left` = (1 - phi)^n >= 0)
    pub(crate) fn lb_ok(&self) -> bool {
        self.lam > 0.0
            && self.lam.is_finite()
            && self.phi > 0.0
            && self.phi < 1.0
            && self.pinf >= 1e-50
            && self.pinf.is_finite()
            && self.tables.len() > CONT_N
    }
    /// With `lb_ok`, a lower bound on `cost(p)` (None: none derived), the same for every p in p's cell (its bits >>
    /// LB_SHIFT, 128 cells per binade), memoised in `cell` (direct-mapped by cell): cost_lo(g) (1 - 1e-9), g the cell's top
    /// (bits (key + 1) << LB_SHIFT, above every p in it), where pinf 1e-250 < p and g < pinf. Derived: computed cost(p) >=
    /// computed cost_lo(p) bit for bit (the same x_n expressions; cost_lo's minimum is over every n until the building
    /// alone B_n (t[n].careers_built) reaches its best, so it is at most each x_n, as each is at least its B_n, and at
    /// most 1/p, where `plan` returns one of them). In exact arithmetic on the stored t, X_n(p) = B_n + 1 / (pinf
    /// exp(ln(p / pinf) r_(n+1))), r_n = t[n].gap_left, and 1/p fall as p rises, so L(p) = min(1/p, min_n<CONT_N X_n(p)) does; each computed x_n is within a
    /// factor 1 +- eps of X_n, eps <= (2 |ln(p / pinf)| + 6) u < 2e-13 here (ln and exp within 1 ulp, as glibc's and the
    /// musl-derived wasm libm are; |ln(p / pinf)| < 576), and L(g) >= cost_lo(g) / (1 + eps) (the n past cost_lo's stop
    /// are each at least B_n >= its best). So for p < g: cost(p) >= cost_lo(p) >= (1 - eps) L(p) >= (1 - eps) L(g) >=
    /// cost_lo(g) (1 - eps) / (1 + eps) > the bound, rounded once more
    pub(crate) fn cost_lb(&self, p: f64, cell: &mut [CostBoundCell; LB_CELLS]) -> Option<f64> {
        if !(p > self.pinf * 1e-250) {
            return None;
        }
        let key = p.to_bits() >> LB_SHIFT;
        let g = f64::from_bits((key + 1) << LB_SHIFT);
        if !(g < self.pinf) {
            return None;
        }
        let e = &mut cell[key as usize & (LB_CELLS - 1)];
        if e.key != key {
            *e = CostBoundCell { key, bound: self.cost_lo(g) * (1.0 - 1e-9) };
        }
        Some(e.bound)
    }
}

/// One memo entry of `Cont::cost_lb`, direct-mapped by cell.
#[derive(Clone, Copy)]
pub(crate) struct CostBoundCell {
    /// The cell held: p's bits >> LB_SHIFT; u64::MAX marks an empty entry.
    pub(crate) key: u64,
    /// The lower bound on `cost(p)` for every p in the cell.
    pub(crate) bound: f64,
}

/// `Cont::cost_lb`'s cells: 2^(52 - LB_SHIFT) per binade of p; LB_CELLS of them memoised per candidate
const LB_SHIFT: u32 = 45;

pub(crate) const LB_CELLS: usize = 256;

impl ScoreBase<'_> {
    // (no Cont: a default one's, lam = 0: grinding at p)
    pub(crate) fn cost(&self, candidate: usize, p: f64) -> f64 {
        match self.continuations.get(candidate) {
            Some(continuation) => continuation.cost(p),
            None => {
                if p <= 0.0 {
                    f64::INFINITY
                } else {
                    1.0 / p
                }
            }
        }
    }
    pub(crate) fn cost_lo(&self, candidate: usize, p: f64) -> f64 {
        match self.continuations.get(candidate) {
            Some(continuation) => continuation.cost_lo(p),
            None => {
                if p <= 0.0 {
                    f64::INFINITY
                } else {
                    1.0 / p
                }
            }
        }
    }
}
