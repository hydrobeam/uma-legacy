//! Step caps from kept fits (docs/SOLVER.md "Next career", *Step caps*): a pool that holds every veteran of a pool
//! planned before can still follow that plan, so a candidate's step there costs at most what that plan's fit prices it
//! at, at her odds now: the kept fit's continuation (its curve, `CapCurve`) read at today's odds, so a refit never makes
//! a side dearer than the old fit says it is, and a lift since then still lowers the cap. A plan keeps its candidates'
//! curves (`KeptCaps`, in `KeptState::step_caps` and the page's store) and a later plan of the same setup reads them
//! while its pool holds that pool's veterans; a removed veteran voids them.
use crate::career::plan::cont::{BuildPlan, Cont};
use crate::career::state::{KeptState, STATE_MAX_N, fx, hash_text, hx, usz, ux, uxr};
use crate::uma::UmaIndex;
use serde_json::{Value, json};

/// Kept plans' caps held (the most recent last): a few numbers each.
const CAPS_KEEP: usize = 8;

/// A kept fit's continuation for one candidate: what `Cont::with_decay` takes, so the cap is that Cont's cost at any
/// odds (the same construction, so the same f64s as the kept plan priced her with).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CapCurve {
    lam: f64,
    phi: f64,
    pinf: f64,
    anchor_odds: f64,
    lam_decay: f64,
}

/// Curves' Conts held per thread before they are dropped and rebuilt (`CapCurve::with_cont`).
const CURVES_KEEP: usize = 256;

thread_local! {
    /// The curves' Conts by their parameters' bits (`CapCurve::with_cont`): a Cont's table and lattice cost hundreds of
    /// powers to build, and a step cap is read per lift in the scoring.
    static CURVES: std::cell::RefCell<crate::FxMap<[u64; 5], Cont>> = std::cell::RefCell::new(crate::FxMap::default());
}

impl CapCurve {
    /// A fitted continuation's.
    pub(crate) const fn of(continuation: &Cont) -> Self {
        let Cont { lam, phi, pinf, anchor_odds, lam_decay, .. } = *continuation;
        Self { lam, phi, pinf, anchor_odds, lam_decay }
    }
    /// `read` on the curve's Cont. A step cap is `Copy` (the plan's tables read it by value), so it holds the curve's
    /// parameters and its Cont is built once per thread, kept by those parameters' bits (a pure function of them)
    fn with_cont<T>(self, read: impl FnOnce(&Cont) -> T) -> T {
        let Self { lam, phi, pinf, anchor_odds, lam_decay } = self;
        let key = [lam, phi, pinf, anchor_odds, lam_decay].map(f64::to_bits);
        CURVES.with(|curves| {
            let mut curves = curves.borrow_mut();
            if curves.len() >= CURVES_KEEP && !curves.contains_key(&key) {
                curves.clear();
            }
            read(curves.entry(key).or_insert_with(|| Cont::with_decay(lam, phi, pinf, anchor_odds, lam_decay)))
        })
    }
    /// The curve's plan at odds p (`Cont::plan`).
    pub(crate) fn plan(self, p: f64) -> BuildPlan {
        self.with_cont(|continuation| continuation.plan(p))
    }
}

/// A candidate's kept cap: her kept fit's curve and her odds in the kept plan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct KeptCap {
    pub(crate) curve: CapCurve,
    pub(crate) odds: f64,
}

/// A candidate's step cap in a plan: her kept cap and its curve's plan at her odds now.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StepCap {
    pub(crate) kept: KeptCap,
    /// The curve's plan at her odds now (`CapCurve::plan`).
    pub(crate) now: BuildPlan,
}

impl StepCap {
    /// `kept` read at her odds now.
    pub(crate) fn at(kept: KeptCap, odds_now: f64) -> Self {
        Self { kept, now: kept.curve.plan(odds_now) }
    }
    /// Whether the cap is cheaper than `plan`, her plan at her odds now: following the kept plan beats the plan this
    /// fit makes
    pub(crate) fn binds(self, plan: &BuildPlan) -> bool {
        self.now.expected_careers < plan.expected_careers
    }
}

/// Her step's plan at her odds now, `plan` (her continuation's there) or the cap's where that is cheaper.
pub(crate) fn capped(plan: BuildPlan, cap: Option<StepCap>) -> BuildPlan {
    match cap {
        Some(cap) if cap.binds(&plan) => cap.now,
        _ => plan,
    }
}

/// A candidate's step caps (`held_caps`: the curve of every kept plan whose pool this one holds, in the order kept) and
/// the one that binds at her odds now (`now`, `least_caps`' choice: the cap reported, and kept for later plans,
/// `plan_caps`). Her step at any odds p is the least of her continuation's plan and every held curve's plan at p
/// (`capped_at`): one envelope for the candidate, read at whatever odds a side of hers is priced at (her best career's,
/// an Alternative's own, a lifted one's), so the plan's steps stay a function of candidate and odds. At her odds now it
/// is `now`'s (the same first least curve), so her best career's step is what one cap gave.
#[derive(Clone, Debug, Default)]
pub(crate) struct StepCaps {
    pub(crate) curves: Vec<KeptCap>,
    pub(crate) now: Option<StepCap>,
}

impl StepCaps {
    /// Her held caps, the least at her odds now `odds_now` chosen as `least_caps` chooses it.
    pub(crate) fn new(curves: Vec<KeptCap>, odds_now: f64) -> Self {
        let now = least_caps(std::slice::from_ref(&curves), &[odds_now])[0];
        Self { curves, now }
    }
    /// The least held curve's plan at odds p: the first of the least (None: none held).
    pub(crate) fn least_at(&self, p: f64) -> Option<BuildPlan> {
        self.curves
            .iter()
            .map(|cap| cap.curve.plan(p))
            .reduce(|least, plan| if plan.expected_careers < least.expected_careers { plan } else { least })
    }
    /// Her step's plan at odds p: `plan` (her continuation's there), or the least held curve's there where cheaper.
    pub(crate) fn capped_at(&self, plan: BuildPlan, p: f64) -> BuildPlan {
        match self.least_at(p) {
            Some(cap_plan) if cap_plan.expected_careers < plan.expected_careers => cap_plan,
            _ => plan,
        }
    }
}

/// Where a plan's caps hold: its scope (the setup, deck space and borrowing rule, `kept::plan_scope`; the fixed parents'
/// veterans; and the trainee filter: the careers a kept plan's policy runs must stay open) and its pool's veterans, each
/// veteran by her record's text (`hash_text`), ascending (a multiset: two identical records are two veterans). The fixed
/// parents are named by their veterans, not their positions: the pool's order is not part of the key, so a position
/// would name another veteran once the legacies are reordered
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CapPool {
    scope: u64,
    veterans: Vec<u64>,
}

impl CapPool {
    /// A job's: `fixed_parents` its fixed parents' positions in its legacies, `plan_scope` its `kept::plan_scope`.
    pub(crate) fn of(job: &Value, fixed_parents: &[usize], plan_scope: u64) -> Self {
        let trainees = job.get("trainees").map_or_else(String::new, Value::to_string);
        let legacies = job.get("legacies").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
        let veteran = |legacy: &Value| hash_text(&legacy.to_string());
        let mut veterans: Vec<u64> = legacies.iter().map(veteran).collect();
        veterans.sort_unstable();
        let mut fixed: Vec<u64> =
            fixed_parents.iter().filter_map(|&parent| legacies.get(parent)).map(veteran).collect();
        fixed.sort_unstable();
        Self {
            scope: plan_scope ^ hash_text(&trainees).rotate_left(5) ^ hash_text(&format!("{fixed:?}")).rotate_left(1),
            veterans,
        }
    }
    /// Whether this pool holds every veteran of `kept` (each as often), in the same scope.
    fn holds(&self, kept: &Self) -> bool {
        if self.scope != kept.scope {
            return false;
        }
        // (both ascending: each of kept's veterans matched to one of ours in turn)
        let mut ours = self.veterans.iter().peekable();
        kept.veterans.iter().all(|veteran| {
            while ours.next_if(|&ours_veteran| ours_veteran < veteran).is_some() {}
            ours.next_if_eq(&veteran).is_some()
        })
    }
}

/// A plan's caps kept for later plans: per candidate uma (`Data::umas`) her kept cap.
pub(crate) struct KeptCaps {
    pool: CapPool,
    caps: Vec<(UmaIndex, KeptCap)>,
}

/// Per trainee, the caps kept for her by the kept plans whose pool `pool` holds, in the order kept (`least_caps` reads
/// them at her odds now, once a plan knows them)
pub(crate) fn held_caps(kept: &[KeptCaps], pool: &CapPool, trainees: &[UmaIndex]) -> Vec<Vec<KeptCap>> {
    trainees
        .iter()
        .map(|&trainee| {
            kept.iter()
                .filter(|entry| pool.holds(&entry.pool))
                .flat_map(|entry| entry.caps.iter().filter(|&&(uma, _)| uma == trainee).map(|&(_, cap)| cap))
                .collect()
        })
        .collect()
}

/// Per trainee (`held_caps`' order, her odds now `odds_now`), the least of her held caps at those odds (None: none
/// held; equal caps: the earlier kept)
pub(crate) fn least_caps(held: &[Vec<KeptCap>], odds_now: &[f64]) -> Vec<Option<StepCap>> {
    held.iter()
        .zip(odds_now)
        .map(|(caps, &odds)| {
            caps.iter()
                .map(|&cap| StepCap::at(cap, odds))
                .reduce(|least, cap| if cap.binds(&least.now) { cap } else { least })
        })
        .collect()
}

/// A plan's caps to keep: per candidate (`trainees`, her odds now `current_odds`) with odds and a finite step, her
/// continuation's curve, or her own cap's where that binds at her odds now (`StepCaps::now`; so a cap only falls along
/// a chain of added veterans, at the odds each plan had), with her odds now
pub(crate) fn plan_caps(
    trainees: &[UmaIndex],
    current_odds: &[f64],
    continuations: &[Cont],
    caps: &[StepCaps],
) -> Vec<(UmaIndex, KeptCap)> {
    (0..trainees.len())
        .filter_map(|candidate| {
            let odds = current_odds[candidate];
            let continuation = continuations.get(candidate)?;
            let own_plan = continuation.plan(odds);
            let curve = match caps.get(candidate).and_then(|caps| caps.now) {
                Some(cap) if cap.binds(&own_plan) => cap.kept.curve,
                _ => CapCurve::of(continuation),
            };
            (odds > 0.0 && own_plan.expected_careers.is_finite())
                .then_some((trainees[candidate], KeptCap { curve, odds }))
        })
        .collect()
}

/// Keep a plan's caps (replacing any kept for the same pool), at most CAPS_KEEP.
pub(crate) fn keep_caps(kept: &mut KeptState, pool: CapPool, caps: Vec<(UmaIndex, KeptCap)>) {
    kept.step_caps.retain(|entry| entry.pool != pool);
    kept.step_caps.push(KeptCaps { pool, caps });
    while kept.step_caps.len() > CAPS_KEEP {
        kept.step_caps.remove(0);
    }
}

/// A cap as stored: [lam, phi, pinf, p0, lam decay, odds], every float as its bits.
pub(crate) fn cap_out(cap: &KeptCap) -> Value {
    let CapCurve { lam, phi, pinf, anchor_odds, lam_decay } = cap.curve;
    json!([hx(lam), hx(phi), hx(pinf), hx(anchor_odds), hx(lam_decay), hx(cap.odds)])
}

/// A stored cap (`cap_out`): every field finite, lam, pinf and p0 not negative, phi in [0, 1], the decay positive and
/// her odds in (0, 1]
pub(crate) fn cap_in(value: &Value) -> Option<KeptCap> {
    let fields = value.as_array().filter(|fields| fields.len() == 6)?;
    let field = |index: usize| fx(&fields[index]).filter(|value| value.is_finite());
    let curve =
        CapCurve { lam: field(0)?, phi: field(1)?, pinf: field(2)?, anchor_odds: field(3)?, lam_decay: field(4)? };
    let odds = field(5)?;
    let valid = curve.lam >= 0.0
        && (0.0..=1.0).contains(&curve.phi)
        && curve.pinf >= 0.0
        && curve.anchor_odds >= 0.0
        && curve.lam_decay > 0.0
        && odds > 0.0
        && odds <= 1.0;
    valid.then_some(KeptCap { curve, odds })
}

/// The kept caps for the page to store (`state_out`).
pub(crate) fn caps_out(kept: &[KeptCaps]) -> Value {
    Value::Array(
        kept.iter()
            .map(|entry| {
                json!({"s": ux(entry.pool.scope), "vets": entry.pool.veterans.iter().map(|&veteran| ux(veteran)).collect::<Vec<_>>(),
                    "caps": entry.caps.iter().map(|(uma, cap)| json!([uma.0, cap_out(cap)])).collect::<Vec<_>>()})
            })
            .collect(),
    )
}

/// Kept caps from the page's store (`state_in`); None refuses the store whole. (Absent: none.)
pub(crate) fn caps_in(value: &Value) -> Option<Vec<KeptCaps>> {
    let Some(stored) = value.as_array() else { return value.is_null().then(Vec::new) };
    stored
        .iter()
        .map(|entry| {
            let stored_veterans = entry["vets"].as_array().filter(|list| list.len() <= STATE_MAX_N)?;
            let mut veterans: Vec<u64> = stored_veterans.iter().map(uxr).collect::<Option<_>>()?;
            veterans.sort_unstable();
            let caps = entry["caps"]
                .as_array()?
                .iter()
                .map(|stored_cap| {
                    let uma = usz(stored_cap.get(0)?).filter(|&uma| uma < STATE_MAX_N)?;
                    Some((UmaIndex(uma), cap_in(stored_cap.get(1)?)?))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(KeptCaps { pool: CapPool { scope: uxr(&entry["s"])?, veterans }, caps })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Her odds in the kept plan and now (a veteran added since lifted her).
    const KEPT_ODDS: f64 = 0.001;
    const ODDS_NOW: f64 = 0.002;

    /// A kept cap whose curve builds at rate `lam` (Astra's example's shape otherwise: phi 0.5, pinf 0.01, p0 0.001).
    fn kept_cap(lam: f64) -> KeptCap {
        KeptCap { curve: CapCurve::of(&Cont::with_decay(lam, 0.5, 0.01, KEPT_ODDS, 1.5)), odds: KEPT_ODDS }
    }

    /// The cap's careers at her odds now, read through the kept plans of `pool`.
    fn careers(kept: &KeptState, pool: &CapPool) -> Option<f64> {
        least_caps(&held_caps(&kept.step_caps, pool, &[UmaIndex(4)]), &[ODDS_NOW])[0]
            .map(|cap| cap.now.expected_careers)
    }

    fn pool(job: Value) -> CapPool {
        CapPool::of(&job, &[], 7)
    }

    /// A pool holds a kept one when it has each of its veterans (as often, any order) in the same scope; a removed
    /// veteran, another trainee filter or another scope voids the kept caps; the least applicable cap is taken.
    #[test]
    fn caps_apply_only_while_the_pool_holds_the_kept_one() {
        let (first, second, third) = (json!({"chara": 1}), json!({"chara": 2}), json!({"chara": 3}));
        let kept_pool = pool(json!({"legacies": [first, second], "trainees": []}));
        let mut kept = KeptState::default();
        let (slow, fast) = (kept_cap(0.01), kept_cap(0.05));
        let (slow_careers, fast_careers) =
            (slow.curve.plan(ODDS_NOW).expected_careers, fast.curve.plan(ODDS_NOW).expected_careers);
        assert!(fast_careers < slow_careers);
        keep_caps(&mut kept, kept_pool.clone(), vec![(UmaIndex(4), slow)]);
        let added = pool(json!({"legacies": [third, second, first], "trainees": []}));
        let removed = pool(json!({"legacies": [first, third], "trainees": []}));
        let filtered = pool(json!({"legacies": [first, second, third], "trainees": [5]}));
        let twice = pool(json!({"legacies": [first, first, third], "trainees": []}));
        assert_eq!(careers(&kept, &added), Some(slow_careers));
        assert!(
            held_caps(&kept.step_caps, &added, &[UmaIndex(5)])[0].is_empty(),
            "a candidate the kept plan did not have"
        );
        assert!(careers(&kept, &removed).is_none(), "a removed veteran keeps the cap");
        assert!(careers(&kept, &filtered).is_none(), "another trainee filter keeps the cap");
        assert!(careers(&kept, &twice).is_none(), "a repeated veteran stands for a missing one");
        let other_scope = CapPool::of(&json!({"legacies": [first, second, third], "trainees": []}), &[], 8);
        assert!(careers(&kept, &other_scope).is_none(), "another scope");
        keep_caps(&mut kept, added, vec![(UmaIndex(4), fast)]);
        let more = pool(json!({"legacies": [first, second, third, json!({"chara": 9})], "trainees": []}));
        assert_eq!(careers(&kept, &more), Some(fast_careers), "the least cap");
        assert_eq!(careers(&kept, &kept_pool), Some(slow_careers), "the larger pool's cap");
        // stored and read back, bit for bit
        let stored = caps_out(&kept.step_caps);
        let restored = caps_in(&stored).unwrap();
        assert_eq!(caps_out(&restored), stored);
        let bad_phi = json!([{"s": ux(1), "vets": [], "caps": [[4, [hx(0.1), hx(1.5), hx(0.01), hx(0.001), hx(1.5), hx(0.001)]]]}]);
        assert!(caps_in(&bad_phi).is_none());
        assert!(caps_in(&Value::Null).unwrap().is_empty());
    }

    /// A fixed parent is her veteran, not her position (Astra's p10 case, 2026-10-26: legacies 1 and 6 swapped with
    /// fix [6], then the order restored keeping fix [6], imported caps fitted with another mandatory parent): the same
    /// veteran fixed at another position keeps the caps, another veteran fixed at the same position voids them
    #[test]
    fn caps_follow_the_fixed_veterans_not_their_positions() {
        let veterans = [json!({"chara": 1}), json!({"chara": 2}), json!({"chara": 3})];
        let (first, second, third) = (&veterans[0], &veterans[1], &veterans[2]);
        let swapped = json!({"legacies": [first, third, second], "trainees": []});
        let in_order = json!({"legacies": [first, second, third], "trainees": []});
        let mut kept = KeptState::default();
        keep_caps(&mut kept, CapPool::of(&swapped, &[2], 7), vec![(UmaIndex(4), kept_cap(0.01))]);
        let read = |job: &Value, fixed: &[usize]| careers(&kept, &CapPool::of(job, fixed, 7));
        assert!(read(&in_order, &[2]).is_none(), "position 2 is another veteran now");
        assert!(read(&in_order, &[1]).is_some(), "the same veteran");
        assert!(read(&in_order, &[]).is_none(), "no fixed parent is another policy");
    }

    /// The cap is the kept fit's curve at today's odds (V_cont_second.md: a refit must not make a side dearer than the
    /// old fit says it is), not its cost at her kept odds: a lift since then lowers it, as the old fit would have
    #[test]
    fn a_cap_is_the_old_curve_at_todays_odds() {
        let kept = kept_cap(0.05);
        let continuation = Cont::with_decay(0.05, 0.5, 0.01, KEPT_ODDS, 1.5);
        let cap = StepCap::at(kept, ODDS_NOW);
        assert_eq!(cap.now.expected_careers.to_bits(), continuation.cost(ODDS_NOW).to_bits());
        assert!(cap.now.expected_careers < continuation.cost(KEPT_ODDS), "the kept odds' cost would plateau");
        // a dearer refit is capped at the old curve, at every odds; a cheaper one stands
        let held = StepCaps::new(vec![kept], ODDS_NOW);
        let dear = BuildPlan { expected_careers: 1e6, building_careers: 0.0, grind_odds: 1e-6 };
        assert_eq!(capped(dear, Some(cap)).expected_careers, cap.now.expected_careers);
        let at_three = held.capped_at(dear, 0.003).expected_careers;
        assert_eq!(at_three.to_bits(), continuation.cost(0.003).to_bits());
        let cheap = BuildPlan { expected_careers: 10.0, building_careers: 0.0, grind_odds: 0.1 };
        assert_eq!(capped(cheap, Some(cap)).expected_careers, 10.0);
        assert_eq!(held.capped_at(cheap, 0.003).expected_careers, 10.0);
        assert_eq!(capped(dear, None).expected_careers, 1e6);
        // a plan's own caps: her curve, or her cap's where it binds at her odds now
        let own = Cont::with_decay(0.01, 0.5, 0.01, ODDS_NOW, 1.5);
        let kept_now = plan_caps(&[UmaIndex(4)], &[ODDS_NOW], std::slice::from_ref(&own), std::slice::from_ref(&held));
        assert_eq!(kept_now[0].1, KeptCap { curve: kept.curve, odds: ODDS_NOW });
        let uncapped = plan_caps(&[UmaIndex(4)], &[ODDS_NOW], std::slice::from_ref(&own), &[StepCaps::default()]);
        assert_eq!(uncapped[0].1.curve, CapCurve::of(&own));
    }

    /// Astra's counterexample (astra-iv.out, 3): an earlier kept cap A that only grinds and a later one B (lam 0.1,
    /// phi 1, pinf 0.01, anchor 0.001). At her odds now, 0.02 (past pinf), both cost 50, so the one cap chosen there is
    /// the earlier, A; an Alternative side of hers still at 0.001 costs 1000 on A and about 110 on B. Her held caps price
    /// it at the least, B's, as they do every lift and floor; the one cap chosen at her odds now priced it at 1000
    #[test]
    fn every_held_cap_prices_a_side_at_its_own_odds() {
        let grinding = KeptCap { curve: CapCurve::of(&Cont::with_decay(0.0, 0.0, 0.0, 0.001, 1.5)), odds: 0.001 };
        let building = KeptCap { curve: CapCurve::of(&Cont::with_decay(0.1, 1.0, 0.01, 0.001, 1.5)), odds: 0.001 };
        let (now_odds, alternative_odds) = (0.02, 0.001);
        assert_eq!(grinding.curve.plan(now_odds).expected_careers, 50.0);
        assert_eq!(building.curve.plan(now_odds).expected_careers, 50.0);
        let held = StepCaps::new(vec![grinding, building], now_odds);
        assert_eq!(held.now.unwrap().kept, grinding, "tied at her odds now: the earlier");
        let grind_at_once = BuildPlan { expected_careers: 1000.0, building_careers: 0.0, grind_odds: alternative_odds };
        let one_cap = least_caps(&[vec![grinding, building]], &[now_odds])[0].unwrap();
        assert_eq!(one_cap.kept.curve.plan(alternative_odds).expected_careers, 1000.0, "the one cap's price");
        let priced = held.capped_at(grind_at_once, alternative_odds).expected_careers;
        assert!((priced - 110.0).abs() < 0.5, "the held caps' price {priced}");
    }
}
