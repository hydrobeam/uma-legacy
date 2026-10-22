//! The continuation (`Cont`) and its memo: the fitted expected careers to finish after a candidate.
use crate::deck::career::sim::plan::CandidateIndex;
use crate::deck::career::sim::{FxMap, Legacy};

/// What a career is scored against: stage-1 costs per candidate (index into `cands`; 0 = her 8/8 exists), stages 2-3 per
/// A, V0, and the stage-1 odds now
pub(in crate::deck::career::sim) struct ScoreBase<'base> {
    pub(in crate::deck::career::sim) cands: &'base [usize],
    pub(in crate::deck::career::sim) c1: &'base [f64],
    pub(in crate::deck::career::sim) m23: &'base [f64],
    pub(in crate::deck::career::sim) v0: f64,
    pub(in crate::deck::career::sim) pnow: &'base [f64],
    pub(in crate::deck::career::sim) cont: &'base [Cont],
    /// the trios (A, B as candidate indices, C's step, A and B from a real 8/8 of hers): V = min over them of A's step +
    /// B's step + C's step, a side from a real 8/8 costing 0, else c1; empty: min over a of c1[a] + m23[a]
    pub(in crate::deck::career::sim) combos: &'base [Trio],
    /// per candidate, the trios' sides: (a real 8/8 veteran of hers, the legacy): her real 8/8s and the 8/8 her best career
    /// now would leave; with `all` (the umas a trio's C may be), what `her_trios` pairs a new real 8/8 with. Empty: none
    pub(in crate::deck::career::sim) sides: &'base [Vec<(bool, Legacy)>],
    pub(in crate::deck::career::sim) all: &'base [usize],
}

/// Requested scoring outputs. Unrequested numeric results are placeholders, not measured zeroes:
///
/// - Stats: per-candidate lift statistics; expected saving and improvement chance are returned as 0.0.
/// - Score: expected saving and improvement chance; no per-candidate statistics.
/// - Keep: Score plus the prepared scoring needed to evaluate later rolls (`score_career`).
/// - Ys: Score plus a saving for each evaluated outcome (`sc_eval`).
/// - Touch: nothing; the preparation's side effects only (her schedule, the partners' `val` searches, `sc_prep`'s
///   counters), its evaluation skipped, zeros returned (the looped rows, whose stats only `proxy_check` reads).
///
/// This selects outputs, not side effects: even Stats can search and populate the deck cache during preparation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::deck::career::sim) enum Want {
    Stats,
    Score,
    Keep,
    Ys,
    Touch,
}

/// (A, B, C's step, A from a real 8/8, B from a real 8/8)
#[derive(Clone, Copy)]
pub(in crate::deck::career::sim) struct Trio {
    /// First side's position in the candidate table.
    pub(in crate::deck::career::sim) candidate_a: CandidateIndex,
    /// Second side's position in the candidate table.
    pub(in crate::deck::career::sim) candidate_b: CandidateIndex,
    /// Expected careers for the finisher after both sides are full veterans.
    pub(in crate::deck::career::sim) finisher_cost: f64,
    /// The first side already exists as a full veteran.
    pub(in crate::deck::career::sim) a_uses_existing: bool,
    /// The second side already exists as a full veteran.
    pub(in crate::deck::career::sim) b_uses_existing: bool,
}

/// One candidate paired with the new veteran as a full trio side.
#[derive(Clone, Copy)]
pub(in crate::deck::career::sim) struct OwnTrio {
    /// Partner's position in the candidate table.
    pub(in crate::deck::career::sim) candidate: CandidateIndex,
    /// The partner already exists as a full veteran.
    pub(in crate::deck::career::sim) uses_existing: bool,
    /// Expected careers for the finisher after both sides are full veterans.
    pub(in crate::deck::career::sim) finisher_cost: f64,
}

/// V over the trios at step costs c1 (per candidate)
pub(in crate::deck::career::sim::plan) fn trio_v(c1: &[f64], combos: &[Trio]) -> f64 {
    // (a plain comparison: no value here is NaN, so the same minimum as f64::min)
    let mut best = f64::INFINITY;
    for trio in combos {
        let a_cost = if trio.a_uses_existing { 0.0 } else { c1[trio.candidate_a.0] };
        let b_cost = if trio.b_uses_existing { 0.0 } else { c1[trio.candidate_b.0] };
        let v = a_cost + b_cost + trio.finisher_cost;
        if v < best {
            best = v;
        }
    }
    best
}

/// An exact memo of a pure function of its key, owned (through an `Rc`) by the one object whose fixed parameters the
/// function reads; a hit returns the value a miss computed for the same key bits, so outputs are the same by
/// construction. The gate only decides whether to look up: after MEMO_PROBE lookups with fewer than one hit in
/// MEMO_MIN_HIT it stops (the hash and insert would cost more than the misses save). Past MEMO_CAP entries it is cleared
/// (the wasm page's memory).
pub(in crate::deck::career::sim) struct Memo<K, V> {
    map: FxMap<K, V>,
    lookups: u64,
    hits: u64,
    /// Gated off (or off from the start, `PLAN_MEMO_OFF`): every call computes.
    disabled: bool,
    /// Which memo this is, for its counters in `MEMO_STATS`.
    kind: MemoKind,
    /// Entries past which the map is cleared.
    capacity: usize,
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

/// (measured on val/plan: a Cont memo without a cap would hit 0.2-1.5% more often; 1 << 14 entries hold about 0.6 MB)
const MEMO_CAP: usize = 1 << 14;

/// bit equality of memoised values (the audit's test)
pub(in crate::deck::career::sim) trait SameBits {
    fn same_bits(&self, o: &Self) -> bool;
}

impl SameBits for f64 {
    fn same_bits(&self, o: &Self) -> bool {
        self.to_bits() == o.to_bits()
    }
}

impl SameBits for BuildPlan {
    fn same_bits(&self, o: &Self) -> bool {
        self.expected_careers.same_bits(&o.expected_careers)
            && self.builds == o.builds
            && self.grind_odds.same_bits(&o.grind_odds)
    }
}

impl<K: std::hash::Hash + Eq, V: Copy + SameBits + std::fmt::Debug> Memo<K, V> {
    // (`PLAN_MEMO_OFF`: every memo off from the start, for the test that compares against no memo)
    // (`PLAN_MEMO_CAP`: another cap, for the test: entries kept long enough to be read after a refit)
    fn new(kind: MemoKind) -> Self {
        let settings = memo_settings();
        Self { map: FxMap::default(), lookups: 0, hits: 0, disabled: settings.off, kind, capacity: settings.capacity }
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
            if memo_settings().check_hits {
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
        MEMO_STATS.with(|c| {
            let mut stats = c.get();
            stats[first] += self.lookups;
            stats[first + 1] += self.hits;
            stats[first + 2] += u64::from(self.disabled);
            c.set(stats);
        });
    }
}

/// The memos' test settings, read from the environment once.
#[derive(Clone, Copy)]
struct MemoSettings {
    /// `PLAN_MEMO_OFF`
    off: bool,
    /// `PLAN_MEMO_CHECK`
    check_hits: bool,
    /// `PLAN_MEMO_CAP`, else MEMO_CAP
    capacity: usize,
}

fn memo_settings() -> MemoSettings {
    static SETTINGS: std::sync::OnceLock<MemoSettings> = std::sync::OnceLock::new();
    *SETTINGS.get_or_init(|| MemoSettings {
        off: std::env::var("PLAN_MEMO_OFF").is_ok(),
        check_hits: std::env::var("PLAN_MEMO_CHECK").is_ok(),
        capacity: std::env::var("PLAN_MEMO_CAP").ok().and_then(|v| v.parse().ok()).unwrap_or(MEMO_CAP),
    })
}

fn memo_stat(place: usize) {
    MEMO_STATS.with(|c| {
        let mut stats = c.get();
        stats[place] += 1;
        c.set(stats);
    });
}

/// Count a Cont refit for `PLAN_MEMO`.
pub(in crate::deck::career::sim::plan) fn count_refit() {
    memo_stat(REFITS_STAT);
}

pub(in crate::deck::career::sim::plan) struct MemoReport;

impl Drop for MemoReport {
    fn drop(&mut self) {
        let stats = MEMO_STATS.with(|c| c.replace([0; 8]));
        if std::env::var("PLAN_MEMO").is_ok() {
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
/// Both constants fitted (val/sim/README.md E5) to traces of building for Agnes Digital on 2-, 5-, 11- and
/// 19-veteran pools (the best builder each career, 6 runs x 400 careers): within 10-20% on 11 and 19, building
/// underestimated on 5 and overestimated early on 2; leave-one-pool-out fits ranged 0.6-1.0 and 1-3. lam = 0: none.
#[derive(Clone)]
pub(in crate::deck::career::sim) struct Cont {
    pub(in crate::deck::career::sim::plan) lam: f64,
    pub(in crate::deck::career::sim::plan) phi: f64,
    pub(in crate::deck::career::sim::plan) pinf: f64,
    /// per n (0..=CONT_N): `plan` reads them instead of evaluating the powers per call (the same expressions, so the
    /// same values). Empty in a default Cont.
    pub(in crate::deck::career::sim::plan) tab: std::rc::Rc<Vec<ContStep>>,
    /// `plan`'s and `cost_lo`'s results by the bits of p. Exact: both read p and lam, phi, pinf and tab alone, which are
    /// set once in `new` and never written after (private fields; a refit makes new Conts). A clone shares the memos and
    /// carries the same parameters; a new or default Cont gets its own
    pub(in crate::deck::career::sim::plan) memo: std::rc::Rc<ContMemo>,
}

/// One row of a Cont's table, for n successes.
#[derive(Clone, Copy)]
pub(in crate::deck::career::sim) struct ContStep {
    /// The careers built through the n-th success (the sum `plan` takes, in its order).
    pub(in crate::deck::career::sim::plan) careers_built: f64,
    /// (1 - phi)^n: the share of the log-odds gap to pinf left after n successes.
    pub(in crate::deck::career::sim::plan) gap_left: f64,
}

/// What `Cont::plan` chooses at odds p: build `builds` successes, then grind.
#[derive(Clone, Copy, Debug)]
pub(in crate::deck::career::sim::plan) struct BuildPlan {
    /// Expected careers to her 8/8.
    pub(in crate::deck::career::sim::plan) expected_careers: f64,
    /// Successes built before grinding.
    pub(in crate::deck::career::sim::plan) builds: i32,
    /// Her odds once building stops.
    pub(in crate::deck::career::sim::plan) grind_odds: f64,
}

pub(in crate::deck::career::sim) struct ContMemo {
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
        Self { lam: 0.0, phi: 0.0, pinf: 0.0, tab: std::rc::Rc::default(), memo: cont_memo() }
    }
}

/// `plan`'s cap on successes before grinding
const CONT_N: usize = 500;

/// Effective ceiling as a share of the full-loop odds, and the lift chance's decline exponent (val/sim/README.md E5)
pub(in crate::deck::career::sim::plan) const CEIL_SHARE: f64 = 0.6;

const LAM_DECAY: f64 = 1.5;

impl Cont {
    pub(in crate::deck::career::sim::plan) fn new(lam: f64, phi: f64, pinf: f64) -> Self {
        let r = 1.0 - phi;
        let mut tab = Vec::with_capacity(CONT_N + 1);
        let mut built = 0.0f64;
        for n in 0..=CONT_N as i32 {
            let careers_built = built + 1.0 / (lam * r.powi(n).powf(LAM_DECAY));
            tab.push(ContStep { careers_built, gap_left: r.powi(n) });
            built = careers_built;
        }
        Self { lam, phi, pinf, tab: std::rc::Rc::new(tab), memo: cont_memo() }
    }
    /// The best plan from odds p: build n successes (sum of 1 / lam_j careers), then grind at p_n; the best n
    pub(in crate::deck::career::sim::plan) fn plan(&self, p: f64) -> BuildPlan {
        if p <= 0.0 {
            return BuildPlan { expected_careers: f64::INFINITY, builds: 0, grind_odds: p };
        }
        if self.lam <= 0.0 || self.phi <= 0.0 || self.pinf <= p {
            return BuildPlan { expected_careers: 1.0 / p, builds: 0, grind_odds: p };
        }
        self.memo.plan.borrow_mut().get_or(|| p.to_bits(), || self.plan_n(p))
    }
    /// `plan` past its early returns (the first stop of the building loop). Past them lam > 0, which only `Cont::new`
    /// makes (with its table; `rolls_in` refuses a stored Cont with lam and phi > 0 but no table)
    fn plan_n(&self, p: f64) -> BuildPlan {
        debug_assert!(self.tab.len() > CONT_N, "plan_n needs the continuation's table");
        let lr = (p / self.pinf).ln();
        let t = &self.tab;
        let (mut n, mut best) = (0usize, 1.0 / p);
        while n < CONT_N {
            let x = t[n].careers_built + 1.0 / (self.pinf * (lr * t[n + 1].gap_left).exp());
            if x >= best {
                break;
            }
            best = x;
            n += 1;
        }
        BuildPlan { expected_careers: best, builds: n as i32, grind_odds: self.pinf * (lr * t[n].gap_left).exp() }
    }
    pub(in crate::deck::career::sim::plan) fn cost(&self, p: f64) -> f64 {
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
        debug_assert!(self.tab.len() > CONT_N, "cost_lo_n needs the continuation's table");
        let lr = (p / self.pinf).ln();
        let mut best = 1.0 / p;
        // every n's cost is its building (rising with n: each term 1/lam_j > 0) plus a positive grind, so once the
        // building alone reaches the best found no later n can be lower: the scan stops there, the same minimum
        let t = &self.tab;
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
    pub(in crate::deck::career::sim::plan) fn lb_ok(&self) -> bool {
        self.lam > 0.0
            && self.lam.is_finite()
            && self.phi > 0.0
            && self.phi < 1.0
            && self.pinf >= 1e-50
            && self.pinf.is_finite()
            && self.tab.len() > CONT_N
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
    pub(in crate::deck::career::sim::plan) fn cost_lb(&self, p: f64, cell: &mut [(u64, f64); LB_CELLS]) -> Option<f64> {
        if !(p > self.pinf * 1e-250) {
            return None;
        }
        let key = p.to_bits() >> LB_SHIFT;
        let g = f64::from_bits((key + 1) << LB_SHIFT);
        if !(g < self.pinf) {
            return None;
        }
        let e = &mut cell[key as usize & (LB_CELLS - 1)];
        if e.0 != key {
            *e = (key, self.cost_lo(g) * (1.0 - 1e-9));
        }
        Some(e.1)
    }
}

/// `Cont::cost_lb`'s cells: 2^(52 - LB_SHIFT) per binade of p; LB_CELLS of them memoised per candidate
const LB_SHIFT: u32 = 45;

pub(in crate::deck::career::sim::plan) const LB_CELLS: usize = 256;

impl ScoreBase<'_> {
    // (no Cont: a default one's, lam = 0: grinding at p)
    pub(in crate::deck::career::sim::plan) fn cost(&self, ui: usize, p: f64) -> f64 {
        match self.cont.get(ui) {
            Some(c) => c.cost(p),
            None => {
                if p <= 0.0 {
                    f64::INFINITY
                } else {
                    1.0 / p
                }
            }
        }
    }
    pub(in crate::deck::career::sim::plan) fn cost_lo(&self, ui: usize, p: f64) -> f64 {
        match self.cont.get(ui) {
            Some(c) => c.cost_lo(p),
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
