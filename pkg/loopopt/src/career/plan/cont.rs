//! The continuation (`Cont`) and its memo: the fitted expected careers to finish after a candidate.
use crate::FxMap;
use crate::career::Legacy;
use crate::career::plan::CandidateIndex;
/// Partners kept per candidate where a career's veteran is a parent, unless the job names another count
/// (`PlanJob::partners`); at most MAX_PARTNERS
pub(crate) const IDX_PARTNERS: usize = 2;

pub(crate) const MAX_PARTNERS: usize = 8;

/// The job's scoring knobs (`PlanJob`)
#[derive(Clone, Copy, Debug)]
pub(crate) struct ScoringKnobs {
    /// Partners kept per candidate where the scored career's veteran is a parent (IDX_PARTNERS unless the job says).
    pub(crate) partners: usize,
}

impl Default for ScoringKnobs {
    fn default() -> Self {
        Self { partners: IDX_PARTNERS }
    }
}

/// What a career is scored against: per stage-1 candidate her step cost, V0, the stage-1 odds now, the fitted
/// continuations and the trios. Per-candidate slices follow `candidates`' order.
pub(crate) struct ScoreBase<'base> {
    /// The stage-1 candidates, as umas.
    pub(crate) candidates: &'base [crate::uma::UmaIndex],
    /// Each candidate's step in expected careers: a fresh full veteran from her best career now (c1).
    pub(crate) step_costs: &'base [f64],
    /// V0: the plan's expected careers now.
    pub(crate) current_value: f64,
    /// Each candidate's stage-1 odds now.
    pub(crate) current_odds: &'base [f64],
    /// Each candidate's fitted continuation; a candidate past its end has none (`cost` grinds at p).
    pub(crate) continuations: &'base [Cont],
    /// Per candidate, her step caps from kept fits (`caps::StepCaps`): her step at any odds is at most the least held
    /// curve's there. Empty, or none held: none (the robustness check's variants refit without them)
    pub(crate) step_caps: &'base [crate::career::plan::caps::StepCaps],
    /// The trios: V = min over them of A's side + B's side + C's step (`trio_value`). Empty: no V (infinite), and no
    /// career is scored against it
    pub(crate) trios: &'base [Trio],
    /// Per candidate, the trios' sides (`TrioSideVeteran`); with `finisher_umas`, what a scoring's routes
    /// (`score::routes`) pair her veteran's sides with. Empty: none
    pub(crate) trio_sides: &'base [Vec<TrioSideVeteran>],
    /// The umas a trio's C may be.
    pub(crate) finisher_umas: &'base [crate::uma::UmaIndex],
    /// The job's scoring knobs.
    pub(crate) knobs: ScoringKnobs,
    /// Every career's P(all) as this base prices it, a multiple of the odds the scoring reads (her lifted candidates'
    /// odds, C's, the scored career's own landed sets), per uma (`OddsScales`): 1 but in the robustness check's variants
    /// (`robust`: all-target ratio, every uma alike; differential, two umas at their own), whose continuations, steps,
    /// trios and V are fitted on odds scaled alike
    pub(crate) odds_scales: OddsScales,
}

/// Every career's P(all) as a multiple of the model's (`ScoreBase::odds_scales`): `common` for every uma but the (at
/// most two) `scaled` ones, at their own (an unused entry is `uma::NONE`)
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OddsScales {
    pub(crate) common: f64,
    pub(crate) scaled: [(crate::uma::UmaIndex, f64); 2],
}

impl OddsScales {
    /// The model's own odds.
    pub(crate) const ONE: Self = Self::uniform(1.0);
    /// Every uma's odds times `scale`.
    pub(crate) const fn uniform(scale: f64) -> Self {
        Self { common: scale, scaled: [(crate::uma::NONE, scale); 2] }
    }
    /// Uma `uma`'s multiple.
    pub(crate) fn of(self, uma: crate::uma::UmaIndex) -> f64 {
        self.scaled
            .iter()
            .find(|&&(scaled_uma, _)| scaled_uma == uma && uma != crate::uma::NONE)
            .map_or(self.common, |&(_, scale)| scale)
    }
}

/// What a trio side costs at the base: nothing (a real full veteran), the candidate's step, or a fixed step.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum SideStep {
    /// A real full veteran of hers already in the pool (her legacy index): free.
    Existing(usize),
    /// The fresh full veteran her best career now would leave: her step (`ScoreBase::step_costs`), the side a scored
    /// career's veteran lifts.
    Current,
    /// The fresh full veteran another of her top careers would leave (`FRESH_LINEAGES`): her step at that career's odds
    /// (`ScoreBase::step_plan` there, building or not: `plan_sides`), which a scored career's veteran lifts on that
    /// career's own lineage (her veteran beside one of its two parents, its deck: `score::routes`).
    Alternative(f64),
}

impl SideStep {
    /// The side's careers when her step is `step`.
    pub(crate) const fn cost(self, step: f64) -> f64 {
        match self {
            Self::Existing(_) => 0.0,
            Self::Current => step,
            Self::Alternative(alternative_step) => alternative_step,
        }
    }
}

/// A trio side's veteran: a real full veteran of the candidate's, the fresh full veteran her best career now would leave,
/// or one another of her top careers would leave (`CareerSetup::veteran`: its parents, every target at the rank's star
/// mix, her G1 wins at their chances).
pub(crate) struct TrioSideVeteran {
    pub(crate) step: SideStep,
    pub(crate) legacy: Legacy,
    /// The career's trainee odds (P(all)) the side's step prices; a real full veteran's: 0.
    pub(crate) odds: f64,
    /// The career's parents (legacy indices); a real full veteran's: her own recorded parents are not indexed (None).
    pub(crate) parents: Option<(usize, usize)>,
    /// The career's deck (a fresh side's: the schedule and outfit of a lifted side read it); a real full veteran's: none.
    pub(crate) deck: Option<crate::deck::space::SelectedDeck>,
    /// Its odds are the career's proven best (a complete search: `PlanCandidates::certified`, `PairCareer::proven`); a real
    /// full veteran's: true (no odds read).
    pub(crate) proven: bool,
}

impl TrioSideVeteran {
    /// The same side at another step (a model variant's: `robust`).
    pub(crate) fn at_step(&self, step: SideStep) -> Self {
        Self {
            step,
            legacy: self.legacy.clone(),
            odds: self.odds,
            parents: self.parents,
            deck: self.deck.clone(),
            proven: self.proven,
        }
    }
}

/// Requested scoring outputs. Unrequested numeric results are placeholders, not measured zeroes:
///
/// - Stats: per-candidate lift statistics; expected saving and improvement chance are returned as 0.0.
/// - Score: expected saving and improvement chance; no per-candidate statistics.
/// - Keep: Score plus the prepared scoring needed to evaluate later rolls (`score_career`).
/// - Ys: Score plus a saving for each evaluated outcome (`Scored::evaluate`).
/// - Joint: Stats plus, per set of candidates lifted at once, its chance (`CareerScore::joint_lifts`; the joint-lift
///   oracle, `oracle`).
/// - Touch: nothing; the preparation's side effects only (her schedule, the partners' `val` searches, `prepare_scoring`'s
///   counters), its evaluation skipped, zeros returned (the looped rows, whose stats only `proxy_check` reads).
///
/// This selects outputs, not side effects: even Stats can search and populate the deck cache during preparation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Want {
    Stats,
    Joint,
    Score,
    Keep,
    Ys,
    Touch,
    /// Score plus its saving split by outcome class (`CareerScore::classes`: her full veteran or not, and the set of
    /// candidates her veteran lifts), a measurement; computed without collapsing targets (the same savings).
    Classes,
    /// Score, with V after her (and so every lifted slot's step, `StepWorkspace::lift_reads`) evaluated at every outcome
    /// whose landed set can happen, whatever her stars' own chance (a zero share of the rank's mix makes some leaves
    /// impossible that a scoring's collapsed targets still read, as their representative): the lift reads
    /// (`score::build::lift_reads`). E[y] and P(y > 1e-9) sum the possible outcomes only, as Score's.
    LiftReads,
}

/// One side of a trio: a candidate and which of her sides (`ScoreBase::trio_sides[candidate]`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TrioSlot {
    pub(crate) candidate: CandidateIndex,
    pub(crate) side: usize,
}

/// A, B and C's step: A's and B's sides, C (the allowed uma best on them) and the expected careers for her once both
/// are full veterans.
#[derive(Clone, Copy)]
pub(crate) struct Trio {
    pub(crate) sides: [TrioSlot; 2],
    pub(crate) finisher: crate::uma::UmaIndex,
    pub(crate) finisher_cost: f64,
}

impl ScoreBase<'_> {
    /// A side's careers at the base's steps.
    pub(crate) fn side_cost(&self, slot: TrioSlot) -> f64 {
        let candidate = slot.candidate.0;
        self.trio_sides[candidate][slot.side].step.cost(self.step_costs[candidate])
    }
    /// The side's step kind.
    pub(crate) fn side_step(&self, slot: TrioSlot) -> SideStep {
        self.trio_sides[slot.candidate.0][slot.side].step
    }
    /// Her Current side (the fresh full veteran of her best career now), where she has one.
    pub(crate) fn current_side(&self, candidate: usize) -> Option<TrioSlot> {
        let side = self.trio_sides.get(candidate)?.iter().position(|side| side.step == SideStep::Current)?;
        Some(TrioSlot { candidate: CandidateIndex(candidate), side })
    }
    /// The odds a fresh side's step is priced at, which a lift of it must pass: her odds now for her Current side, an
    /// Alternative side's own (its career's P(all)); a real full veteran's 0.
    pub(crate) fn side_odds(&self, slot: TrioSlot) -> f64 {
        let side = &self.trio_sides[slot.candidate.0][slot.side];
        match side.step {
            SideStep::Existing(_) => 0.0,
            SideStep::Current => self.current_odds[slot.candidate.0],
            SideStep::Alternative(_) => side.odds,
        }
    }
}

/// A trio's careers at steps `step_costs`: A's side + B's side + C's step
pub(crate) fn trio_value(step_costs: &[f64], sides: &[Vec<TrioSideVeteran>], trio: &Trio) -> f64 {
    let side = |slot: TrioSlot| sides[slot.candidate.0][slot.side].step.cost(step_costs[slot.candidate.0]);
    side(trio.sides[0]) + side(trio.sides[1]) + trio.finisher_cost
}

/// V over the trios at steps `step_costs`: the least `trio_value` (infinite with no trio)
pub(crate) fn trio_v(step_costs: &[f64], sides: &[Vec<TrioSideVeteran>], trios: &[Trio]) -> f64 {
    // (a plain comparison: no value here is NaN, so the same minimum as f64::min)
    let mut best = f64::INFINITY;
    for trio in trios {
        let value = trio_value(step_costs, sides, trio);
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
                "PLAN_MEMO plan lookups {} hits {} ({:.1}%) off {} | refits {} clears {}",
                stats[0],
                stats[1],
                hit_percent(stats[0], stats[1]),
                stats[2],
                stats[REFITS_STAT],
                stats[CLEARS_STAT]
            );
        }
    }
}

/// A step's continuation (form assumed; the rate and step fitted to the pool now, the shape to traces): builder careers
/// lift her odds, each lift closing `phi` of the remaining gap in log odds to `pinf`, her effective ceiling; the lift
/// chance falls with the remaining gap, lam(p) = lam g(p)^LAM_DECAY, g(p) = ln(pinf / p) / ln(pinf / p0) the share of
/// the gap left of the one at `anchor_odds` p0 (the odds the fit was made at; clamped to [0, 1], so 1 at and below p0).
/// After j lifts from p0, g = (1 - phi)^j: lam_j = lam ((1 - phi)^j)^LAM_DECAY. The cost is a function of the gap left,
/// so a lifted candidate's step continues her decay where the lift left it (it does not start again at lam). Above p0
/// the cost is read off the lattice of p0's path, p_j = pinf (p0 / pinf)^((1 - phi)^j) (`lattice`: the cost to go from
/// each p_j, the least over the rest of the same build, which falls with j), interpolated linearly in the fractional
/// index t (g = (1 - phi)^t), and never above 1 / p (grinding at once), so it falls as the odds rise and is the table's
/// own at p0 (a design choice: a lift between lattice points is priced between their costs; with phi = 1 every odds below
/// pinf is at index 0, so there the least of K_0 and 1 / p). Below p0 each
/// success's rate is the state-local one, capped at lam (`Building`). pinf = CEIL_SHARE x her full-loop odds. Shape calibration and its limits: docs/MODEL.md, "Next career"; trace setup:
/// val/sim/README.md. lam = 0: no building.
#[derive(Clone)]
pub(crate) struct Cont {
    pub(crate) lam: f64,
    pub(crate) phi: f64,
    pub(crate) pinf: f64,
    /// p0: her odds when lam was fitted (the anchor of g); 0 or less: none (g = 1 at every p).
    pub(crate) anchor_odds: f64,
    /// The lift chance's decay exponent: LAM_DECAY but in the robustness check's shape variants (`ContShape`).
    pub(crate) lam_decay: f64,
    /// per n (0..=CONT_N), at p0: `plan` reads them instead of evaluating the powers per call (the same expressions, so
    /// the same values) and scales the building by `building_scale(p)`. Empty in a default Cont.
    pub(crate) tables: std::rc::Rc<Vec<ContStep>>,
    /// per lattice point p_j of p0's path (j = 0..=CONT_N), her plan from there: the cost to go (non-increasing in j;
    /// at j = 0 `plan(p0)`'s own) and the careers it builds. Empty with no anchor in (0, pinf) or no building.
    pub(crate) lattice: std::rc::Rc<Vec<LatticePoint>>,
    /// `plan`'s results by the bits of p. Exact: it reads p and lam, phi, pinf, p0 and tables alone,
    /// which are kept unchanged after construction (a refit makes new Conts). A clone shares the memos and carries the
    /// same parameters; a new or default Cont gets its own
    pub(crate) memo: std::rc::Rc<ContMemo>,
}

/// One row of a Cont's table, for n successes.
#[derive(Clone, Copy)]
pub(crate) struct ContStep {
    /// The careers built through success n + 1 (the sum `plan` takes, in its order).
    pub(crate) careers_built: f64,
    /// 1 / lam_n at p0: success n + 1's waiting time (the term `careers_built` adds).
    pub(crate) step: f64,
    /// (1 - phi)^n: the share of the log-odds gap to pinf left after n successes.
    pub(crate) gap_left: f64,
}

/// A lattice point's plan (`Cont::lattice`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct LatticePoint {
    /// Expected careers to her full veteran from p_j.
    pub(crate) cost: f64,
    /// The careers of it spent building (0 where she grinds at p_j).
    pub(crate) building: f64,
}

/// What `Cont::plan` chooses at odds p: build n successes, then grind.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BuildPlan {
    /// Expected careers to her full veteran.
    pub(crate) expected_careers: f64,
    /// Expected careers spent building her n successes (each success takes 1 / lam(p_j) careers): the table's
    /// `careers_built` through success n times `building_scale(p)`, 0 with no building. `expected_careers` is this plus
    /// 1 / `grind_odds`.
    pub(crate) building_careers: f64,
    /// Her odds once building stops.
    pub(crate) grind_odds: f64,
}

pub(crate) struct ContMemo {
    plan: std::cell::RefCell<Memo<u64, BuildPlan>>,
}

fn cont_memo() -> std::rc::Rc<ContMemo> {
    std::rc::Rc::new(ContMemo { plan: std::cell::RefCell::new(Memo::new(MemoKind::Plan)) })
}

impl Default for Cont {
    fn default() -> Self {
        Self {
            lam: 0.0,
            phi: 0.0,
            pinf: 0.0,
            anchor_odds: 0.0,
            lam_decay: LAM_DECAY,
            tables: std::rc::Rc::default(),
            lattice: std::rc::Rc::default(),
            memo: cont_memo(),
        }
    }
}

/// `plan`'s cap on successes before grinding
const CONT_N: usize = 500;

/// Effective ceiling as a share of the full-loop odds; the following exponent controls the lift chance's decline.
/// Calibration: docs/MODEL.md, "Next career" (`CEIL_SHARE`, `LAM_DECAY`).
pub(crate) const CEIL_SHARE: f64 = 0.6;

const LAM_DECAY: f64 = 1.5;

/// The continuation's assumed shape: pinf's share of her full-loop odds and the lift chance's decay exponent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ContShape {
    pub(crate) ceil_share: f64,
    pub(crate) lam_decay: f64,
}

/// The shape the plan prices with (`CEIL_SHARE`, `LAM_DECAY`).
pub(crate) const FITTED_SHAPE: ContShape = ContShape { ceil_share: CEIL_SHARE, lam_decay: LAM_DECAY };

impl Cont {
    pub(crate) fn new(lam: f64, phi: f64, pinf: f64, anchor_odds: f64) -> Self {
        Self::with_decay(lam, phi, pinf, anchor_odds, LAM_DECAY)
    }
    /// `new` with the lift chance decaying at `lam_decay` (a shape variant's, `ContShape`).
    pub(crate) fn with_decay(lam: f64, phi: f64, pinf: f64, anchor_odds: f64, lam_decay: f64) -> Self {
        let r = 1.0 - phi;
        let mut tables = Vec::with_capacity(CONT_N + 1);
        let mut built = 0.0f64;
        for n in 0..=CONT_N as i32 {
            let step = 1.0 / (lam * r.powi(n).powf(lam_decay));
            let careers_built = built + step;
            tables.push(ContStep { careers_built, step, gap_left: r.powi(n) });
            built = careers_built;
        }
        let mut cont = Self {
            lam,
            phi,
            pinf,
            anchor_odds,
            lam_decay,
            tables: std::rc::Rc::new(tables),
            lattice: std::rc::Rc::default(),
            memo: cont_memo(),
        };
        if lam > 0.0 && phi > 0.0 && anchor_odds > 0.0 && anchor_odds < pinf {
            cont.lattice = std::rc::Rc::new(cont.lattice_points());
        }
        cont
    }
    /// The lattice of p0's path (`lattice`): backwards from p_N (N = CONT_N, where she grinds), the cost to go from p_j
    /// the least of grinding there and success j + 1 (the table's 1 / lam_j, the rate at p_j) plus the cost to go from
    /// p_{j+1}; each at most the next one's plus a positive term, and 1 / p_j at least 1 / p_{j+1}, so it falls with j.
    /// At j = 0 `plan_n(p0)`'s own value (the table's first non-improving n: the same minimum, its f64s those V0 is
    /// priced with), each later cost at most the one before it (a rounding apart, the min changes nothing)
    fn lattice_points(&self) -> Vec<LatticePoint> {
        let lr = (self.anchor_odds / self.pinf).ln();
        let odds = |j: usize| self.pinf * (lr * self.tables[j].gap_left).exp();
        let mut points = vec![LatticePoint { cost: 0.0, building: 0.0 }; CONT_N + 1];
        points[CONT_N] = LatticePoint { cost: 1.0 / odds(CONT_N), building: 0.0 };
        for j in (0..CONT_N).rev() {
            let (grind, next) = (1.0 / odds(j), points[j + 1]);
            let build = self.tables[j].step + next.cost;
            points[j] = if grind <= build {
                LatticePoint { cost: grind, building: 0.0 }
            } else {
                LatticePoint { cost: build, building: self.tables[j].step + next.building }
            };
        }
        let at_anchor = self.plan_n(self.anchor_odds);
        points[0] = LatticePoint { cost: at_anchor.expected_careers, building: at_anchor.building_careers };
        for j in 1..=CONT_N {
            if points[j].cost > points[j - 1].cost {
                points[j] = points[j - 1];
            }
        }
        points
    }
    /// `plan` above p0 (below pinf): the least of grinding at p (1 / p, no building, her odds p) and the lattice's plans
    /// at the fractional index t of p (ln(pinf / p) = (1 - phi)^t ln(pinf / p0); 0 with phi = 1), interpolated linearly
    /// between lattice points j = floor(t) and j + 1 and held between their costs; both fall as p rises, so their least
    /// does, and it is never above grinding at once (between two points that both grind, the interpolation alone was;
    /// with phi = 1, or past the lattice's last point, it stayed at a point's cost below pinf). The careers built
    /// interpolated alike, the grind's odds what makes `expected_careers` their sum
    fn lattice_plan(&self, p: f64) -> BuildPlan {
        let lattice = &self.lattice;
        let g = (p / self.pinf).ln() / (self.anchor_odds / self.pinf).ln();
        let r = 1.0 - self.phi;
        let t = if r > 0.0 { (g.ln() / r.ln()).max(0.0) } else { 0.0 };
        let j = t.floor();
        let (cost, building) = if j >= CONT_N as f64 {
            (lattice[CONT_N].cost, lattice[CONT_N].building)
        } else {
            let (low, high) = (lattice[j as usize], lattice[j as usize + 1]);
            let frac = t - j;
            let between =
                |first: f64, second: f64| (first + frac * (second - first)).clamp(first.min(second), first.max(second));
            (between(low.cost, high.cost), between(low.building, high.building).min(between(low.cost, high.cost)))
        };
        if 1.0 / p <= cost {
            return BuildPlan { expected_careers: 1.0 / p, building_careers: 0.0, grind_odds: p };
        }
        let grind = cost - building;
        BuildPlan {
            expected_careers: cost,
            building_careers: building,
            grind_odds: if grind > 0.0 { 1.0 / grind } else { self.pinf },
        }
    }
    /// s(p) = (ln(pinf / p) / ln(pinf / p0))^-LAM_DECAY: what success j from p costs as a multiple of the table's 1 / lam_j
    /// (lam(x) = lam g(x)^1.5 at every odds x on her path, and the path from p is p0's shifted along the gap). Exactly 1 at
    /// p0 and with no anchor (V0 and every step at the fit's odds are the table's own sums), above 1 between p0 and pinf,
    /// below 1 under p0 (where each success's rate is capped at lam: `Building`). An anchor outside (0, pinf) (a fit
    /// given from another pool) is none
    fn building_scale(&self, p: f64) -> f64 {
        if !(self.anchor_odds > 0.0 && self.anchor_odds < self.pinf) || p == self.anchor_odds {
            return 1.0;
        }
        ((p / self.pinf).ln() / (self.anchor_odds / self.pinf).ln()).powf(-self.lam_decay)
    }
    /// The best plan from odds p: build n successes, then grind at p_n; the best n. At pinf and above (phi = 1 lifts
    /// straight there) she grinds: no building is left
    pub(crate) fn plan(&self, p: f64) -> BuildPlan {
        if p <= 0.0 {
            return BuildPlan { expected_careers: f64::INFINITY, building_careers: 0.0, grind_odds: p };
        }
        if self.lam <= 0.0 || self.phi <= 0.0 || self.pinf <= p {
            return BuildPlan { expected_careers: 1.0 / p, building_careers: 0.0, grind_odds: p };
        }
        if p > self.anchor_odds && !self.lattice.is_empty() {
            return self.memo.plan.borrow_mut().get_or(|| p.to_bits(), || self.lattice_plan(p));
        }
        self.memo.plan.borrow_mut().get_or(|| p.to_bits(), || self.plan_n(p))
    }
    /// `plan` past its early returns, at or below p0 (or with no anchor): the first n that does not improve ends the minimum (each success's waiting time,
    /// `Building::next`, rises with n and the grind's saving falls, so the differences rise). Past them lam > 0, which
    /// only `Cont::new` makes (with its table; `rolls_in` refuses a stored Cont with lam and phi > 0 but no table)
    fn plan_n(&self, p: f64) -> BuildPlan {
        debug_assert!(self.tables.len() > CONT_N, "plan_n needs the continuation's table");
        let lr = (p / self.pinf).ln();
        let t = &self.tables;
        let mut building = Building::new(self, p);
        let (mut n, mut best, mut building_careers) = (0usize, 1.0 / p, 0.0f64);
        while n < CONT_N {
            let built = building.next(t, n);
            let x = built + 1.0 / (self.pinf * (lr * t[n + 1].gap_left).exp());
            if x >= best {
                break;
            }
            (best, building_careers) = (x, built);
            n += 1;
        }
        BuildPlan { expected_careers: best, building_careers, grind_odds: self.pinf * (lr * t[n].gap_left).exp() }
    }
    pub(crate) fn cost(&self, p: f64) -> f64 {
        self.plan(p).expected_careers
    }
    /// The path `plan_n` scans from odds p: per n successes (0 to CONT_N) the careers built through them and her odds
    /// then (the same expressions, so at or below p0 `cost(p)` is the least of built + 1 / odds over its first points;
    /// above p0 the path's building is the state-local rate's, which the lattice's cost does not follow); one point,
    /// (0, p), where she only grinds. The joint-lift oracle's sides (`oracle`)
    pub(crate) fn building_path(&self, p: f64) -> Vec<PathPoint> {
        let mut path = vec![PathPoint { built: 0.0, odds: p }];
        if p <= 0.0 || self.lam <= 0.0 || self.phi <= 0.0 || self.pinf <= p {
            return path;
        }
        let lr = (p / self.pinf).ln();
        let mut building = Building::new(self, p);
        for n in 0..CONT_N {
            let built = building.next(&self.tables, n);
            path.push(PathPoint { built, odds: self.pinf * (lr * self.tables[n + 1].gap_left).exp() });
        }
        path
    }
}

/// A point of a continuation's path (`Cont::building_path`): the careers built through n successes, her odds then.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PathPoint {
    pub(crate) built: f64,
    pub(crate) odds: f64,
}

/// `Cont::with_decay(lam, phi, pinf, p, lam_decay).cost(p)` without the table or a memo (the same expressions in the same
/// order, so the same f64): what a continuation anchored at her odds now costs her, for choosing among builders
/// (`fit_continuation`)
pub(crate) fn anchored_cost(lam: f64, phi: f64, pinf: f64, p: f64, lam_decay: f64) -> f64 {
    if p <= 0.0 {
        return f64::INFINITY;
    }
    if lam <= 0.0 || phi <= 0.0 || pinf <= p {
        return 1.0 / p;
    }
    let (r, lr) = (1.0 - phi, (p / pinf).ln());
    let (mut built, mut best) = (0.0f64, 1.0 / p);
    for n in 0..CONT_N as i32 {
        built += 1.0 / (lam * r.powi(n).powf(lam_decay));
        let x = built + 1.0 / (pinf * (lr * r.powi(n + 1)).exp());
        if x >= best {
            break;
        }
        best = x;
    }
    best
}

/// The careers spent building through each success from one odds (W_n, `Cont::building_scale` s there): s x the table's
/// sum where s >= 1 (from p0 up every rate is lam g^1.5 <= lam, the table's terms scaled alike; at p0, the table's own
/// f64s; `plan` reads it above p0 only with no lattice, `building_path` always), else the running sum of max(1/lam, s / lam_j) (below p0 a success's rate is lam g^1.5 capped at lam: the
/// state-local rate at each odds on the path, so the cap holds until the gap closes to p0's and the decay resumes there)
struct Building {
    scale: f64,
    /// 1 / lam, the least waiting time (below p0).
    floor: f64,
    /// The running sum (below p0).
    sum: f64,
}

impl Building {
    fn new(cont: &Cont, p: f64) -> Self {
        Self { scale: cont.building_scale(p), floor: 1.0 / cont.lam, sum: 0.0 }
    }
    /// The careers built through success n + 1, called for n = 0, 1, ... in turn
    #[inline]
    fn next(&mut self, table: &[ContStep], n: usize) -> f64 {
        if self.scale >= 1.0 {
            self.scale * table[n].careers_built
        } else {
            self.sum += self.floor.max(self.scale * table[n].step);
            self.sum
        }
    }
}

impl ScoreBase<'_> {
    /// Her step at odds p (as the base prices odds, `odds_scale`): her Cont's cost there, or her least held cap curve's
    /// there where cheaper (`caps::StepCaps::capped_at`), and whether she still builds at those odds
    /// (`BuildPlan::building_careers` > 0); with no Cont (a default one's, lam = 0) she grinds at p
    pub(crate) fn step_plan(&self, candidate: usize, p: f64) -> (f64, bool) {
        let p = p * self.odds_scales.of(self.candidates[candidate]);
        match self.continuations.get(candidate) {
            Some(continuation) => {
                let own = continuation.plan(p);
                let plan = self.step_caps.get(candidate).map_or(own, |caps| caps.capped_at(own, p));
                (plan.expected_careers, plan.building_careers > 0.0)
            }
            None => (if p <= 0.0 { f64::INFINITY } else { 1.0 / p }, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Astra's example (`.review/planner`, 2026-10-26): p0 0.001, pinf 0.01, lam 0.5, phi 0.5
    const P0: f64 = 0.001;
    const PINF: f64 = 0.01;
    const LAM: f64 = 0.5;

    /// The state-local rate at odds x: lam min(1, ln(pinf / x) / ln(pinf / p0))^LAM_DECAY
    fn rate(x: f64) -> f64 {
        LAM * ((PINF / x).ln() / (PINF / P0).ln()).min(1.0).powf(LAM_DECAY)
    }

    /// The cost by its definition, every n scanned: min_n sum_{j<n} 1 / rate(p_j) + 1 / p_n, p_j = pinf (p / pinf)^(r^j)
    fn scanned(phi: f64, p: f64) -> f64 {
        let r = 1.0 - phi;
        let odds = |n: i32| PINF * ((p / PINF).ln() * r.powi(n)).exp();
        let (mut built, mut best) = (0.0f64, 1.0 / p);
        for n in 0..CONT_N as i32 {
            built += 1.0 / rate(odds(n));
            best = best.min(built + 1.0 / odds(n + 1));
        }
        best
    }

    fn close(first: f64, second: f64, relative: f64) -> bool {
        (first - second).abs() <= relative * first.abs().max(second.abs())
    }

    #[test]
    fn at_the_anchor_the_table_alone() {
        let cont = Cont::new(LAM, 0.5, PINF, P0);
        // the cost before the anchor existed: the table's sums, unscaled, the first non-improving n
        let lr = (P0 / PINF).ln();
        let (mut n, mut best) = (0usize, 1.0 / P0);
        while n < CONT_N {
            let x = cont.tables[n].careers_built + 1.0 / (PINF * (lr * cont.tables[n + 1].gap_left).exp());
            if x >= best {
                break;
            }
            best = x;
            n += 1;
        }
        assert_eq!(cont.cost(P0).to_bits(), best.to_bits());
        assert_eq!(anchored_cost(LAM, 0.5, PINF, P0, LAM_DECAY).to_bits(), best.to_bits());
    }

    #[test]
    fn at_and_below_the_anchor_the_cost_is_its_definition() {
        for phi in [0.5, 0.2, 0.09] {
            let cont = Cont::new(LAM, phi, PINF, P0);
            for k in 1..200 {
                let p = 1e-5 * 1.03f64.powi(k);
                if p > P0 {
                    break;
                }
                assert!(
                    close(cont.cost(p), scanned(phi, p), 1e-9),
                    "phi {phi} p {p}: {} vs {}",
                    cont.cost(p),
                    scanned(phi, p)
                );
            }
        }
        let cont = Cont::new(LAM, 0.5, PINF, P0);
        assert!(close(cont.cost(0.001), 157.008_997, 1e-8), "{}", cont.cost(0.001));
        // phi = 1: one success reaches pinf; at pinf nothing is left to build
        let lifted_once = Cont::new(LAM, 1.0, PINF, P0);
        assert!(close(lifted_once.cost(0.001), 102.0, 1e-12));
        assert_eq!(lifted_once.cost(PINF), 1.0 / PINF);
    }

    /// Above p0 the cost falls as the odds rise (the lattice's costs to go, interpolated in the fractional index), meets
    /// the lattice at its points, and at p0 is the table's own f64 (V0 unchanged): on a grid of shapes and on the
    /// fits val/plan's references hold
    #[test]
    fn above_the_anchor_the_cost_falls_and_is_the_tables_at_p0() {
        let mut fits: Vec<(f64, f64, f64, f64, f64)> = vec![];
        for lam in [0.02, 0.3, 1.0] {
            for phi in [0.05, 0.2, 0.368, 0.7, 1.0] {
                for ratio in [2.0, 10.0, 100.0] {
                    for decay in [0.6, 1.3, LAM_DECAY] {
                        fits.push((lam, phi, 1e-2, 1e-2 / ratio, decay));
                    }
                }
            }
        }
        let refs = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan")).unwrap();
        for entry in refs.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !(name.starts_with("ref__") && name.ends_with(".out")) {
                continue;
            }
            let out: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(entry.path()).unwrap()).unwrap();
            for fit in out["next"]["fit"].as_array().into_iter().flatten() {
                let at = |i: usize| fit[i].as_f64().unwrap();
                fits.push((at(1).min(1.0), at(2), at(3), at(4), LAM_DECAY));
            }
        }
        assert!(fits.len() > 135, "the references' fits were read");
        for (lam, phi, pinf, p0, decay) in fits {
            if !(lam > 0.0 && phi > 0.0 && p0 > 0.0 && p0 < pinf) {
                continue;
            }
            let cont = Cont::with_decay(lam, phi, pinf, p0, decay);
            assert_eq!(cont.cost(p0).to_bits(), anchored_cost(lam, phi, pinf, p0, decay).to_bits());
            let mut last = cont.cost(p0);
            for k in 1..=2000 {
                let p = p0 * (pinf / p0).powf(f64::from(k) / 2000.0);
                if p >= pinf {
                    break;
                }
                let cost = cont.cost(p);
                assert!(cost <= last, "lam {lam} phi {phi} pinf {pinf} p0 {p0}: cost({p}) {cost} > {last}");
                last = cost;
            }
            // never above grinding at once, at any odds
            for k in 0..=400 {
                let p = p0 * 0.25 * (4.0 * pinf / p0).powf(f64::from(k) / 400.0);
                assert!(cont.cost(p) <= 1.0 / p, "lam {lam} phi {phi}: cost({p}) {} > 1 / p", cont.cost(p));
            }
            if phi < 1.0 {
                for j in 1..6 {
                    let p_j = pinf * ((p0 / pinf).ln() * (1.0 - phi).powi(j)).exp();
                    let point = cont.lattice[j as usize].cost;
                    let want = point.min(1.0 / p_j);
                    assert!(close(cont.cost(p_j), want, 1e-9), "phi {phi}: cost(p_{j}) {} vs {want}", cont.cost(p_j));
                }
            }
        }
    }

    /// Astra's cases (`.review/planner/astra-review-planfinal.out`): between two lattice points that both grind, at
    /// phi = 1 below pinf, and past a small phi's last lattice point, the cost is at most grinding at once and runs on
    /// to 1 / pinf without a jump
    #[test]
    fn grinding_at_once_bounds_the_cost_above_the_anchor() {
        let cont = Cont::new(0.5, 0.5, 0.01, 0.001);
        // (1 / q = 122.5712: the interpolation alone was 124.415)
        let q = 0.008_158_525;
        assert!(cont.cost(q) <= 1.0 / q && cont.cost(q) < 122.5712, "{}", cont.cost(q));
        let once = Cont::new(0.5, 1.0, 0.01, 0.001);
        assert!(once.cost(0.0099) <= 101.011, "{}", once.cost(0.0099));
        assert!(close(once.cost(0.0099), 1.0 / 0.0099, 1e-12));
        assert_eq!(once.plan(0.0099).building_careers, 0.0);
        assert_eq!(once.plan(0.0099).grind_odds, 0.0099);
        // a small phi: the lattice's 500 points end short of pinf; just below pinf she grinds, at about 1 / pinf
        let slow = Cont::new(0.5, 0.001, 0.01, 0.001);
        let near_pinf = 0.01 * (1.0 - 1e-9);
        assert!(slow.cost(near_pinf) <= 1.0 / near_pinf && close(slow.cost(near_pinf), 100.0, 1e-6));
        let mut last = slow.cost(0.001);
        for k in 1..=1000 {
            let p = 0.001 * 10f64.powf(f64::from(k) / 1000.0) * (1.0 - 1e-12);
            assert!(slow.cost(p) <= last, "{p}");
            last = slow.cost(p);
        }
    }

    #[test]
    fn a_builder_lifting_by_the_fitted_step_scores_zero() {
        // a builder that lifts her with the rate at her odds to the odds one step closes, and changes nothing else:
        // score = rate (C(p) - C(p1)) - 1, zero where building pays (C(p) = 1 / rate + C(p1))
        let phi = 0.09;
        let cont = Cont::new(LAM, phi, PINF, P0);
        // (at and below p0, and at p0's lattice points above it: between them the lattice is interpolated)
        let p1 = PINF * ((P0 / PINF).ln() * (1.0 - phi)).exp();
        for p in [P0, 0.5 * P0, p1] {
            let lifted = PINF * ((p / PINF).ln() * (1.0 - phi)).exp();
            assert!(cont.plan(p).building_careers > 0.0, "building pays at {p}");
            let score = rate(p) * (cont.cost(p) - cont.cost(lifted)) - 1.0;
            assert!(score.abs() < 1e-6, "p {p}: score {score}");
        }
        // the old continuation, its decay started again at the lifted odds, credits the same builder with careers
        let restarted = Cont::new(LAM, phi, PINF, PINF * ((P0 / PINF).ln() * (1.0 - phi)).exp());
        let lifted = PINF * ((P0 / PINF).ln() * (1.0 - phi)).exp();
        let restart_score = LAM * (cont.cost(P0) - restarted.cost(lifted)) - 1.0;
        assert!(restart_score > 0.5, "the restart's score {restart_score}");
    }
}
