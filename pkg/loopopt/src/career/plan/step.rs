//! Step costs and V after her veteran, at one outcome.
use crate::career::plan::cont::ScoreBase;
/// One candidate's improvement; slot indexes the estimate's per-row cost cache.
#[derive(Clone, Copy)]
pub(crate) struct CandidateLift {
    pub(crate) slot: usize,
    pub(crate) candidate: usize,
    pub(crate) probability: f64,
}

/// No partner slot (`Route::lifts`).
pub(crate) const NO_SLOT: usize = usize::MAX;

/// The most routes one scoring keeps (`score::routes`), and so the most a stored roll state holds (`rolls_in`).
pub(crate) const MAX_ROUTES: usize = 1024;

/// A route through the scored career's veteran (`score::routes`): one trio taken with her in it, priced on its own
/// lineage at each outcome. Her veteran lifts a candidate by being her parent beside one partner (a partner slot), so
/// that candidate's fresh side is then bred from her and that partner, and C has her as a grandparent there; her own
/// full veteran (the full set) can be a side itself, C's parent. The route's C is one allowed uma, fixed, valued at
/// each outcome with her in her slots at that outcome's stars (a finisher slot of the scoring), where every lifted side
/// grinds at once at its lift's odds; where one still builds there (`BuildPlan::building_careers` > 0), its later
/// builders replace the parents that put her in C's lineage, so C is priced as V0 prices it (`while_building`). The
/// trios as they were stay in V0 beside every route.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Route {
    /// The partner slots whose lifts it takes (each lifted candidate's step at that slot's odds); NO_SLOT for none.
    pub(crate) lifts: [usize; 2],
    /// One side is her own full veteran: only where she lands every target.
    pub(crate) full_only: bool,
    /// Its sides' careers fixed at the base: the other side's step (0 for her own full veteran, or none).
    pub(crate) fixed_cost: f64,
    /// The finisher slot holding its C's odds; NO_SLOT for none: a route one of whose lift slots no outcome leaves her
    /// grinding at once after (`LiftRead::grinds`), so it takes its C while building wherever it is taken.
    pub(crate) finisher_slot: usize,
    /// C's step where a lifted side still builds at its lift's odds.
    pub(crate) while_building: WhileBuilding,
}

/// A route the memory budget left out (`score::routes`), checked at every outcome a scoring evaluates
/// (`Scored::shadows`): its lifts and sides as the route has them, and the least its C can cost there
/// (`finisher_floor`: 1 / the most its own C's odds can be, or its C while building, the least that route can take).
/// Where it is taken, its value is at least ((its steps + 0) + `fixed_cost`) + `finisher_floor`, the route's own sum
/// with its last term no larger (rounding is monotone), so where that is at least V after her its omission changed
/// nothing there: the certificate (`ScoringEvaluation::uncertified`)
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShadowRoute {
    pub(crate) lifts: [usize; 2],
    pub(crate) full_only: bool,
    pub(crate) fixed_cost: f64,
    pub(crate) finisher_floor: f64,
}

impl ShadowRoute {
    /// Whether it can be below V after her (`least`) at an outcome whose lifted slots' steps are `steps` (V after her's,
    /// `ScoreBase::least_after`) and where she landed every target or not (`full`)
    pub(crate) fn undercuts(&self, steps: &[Option<LiftStep>], full: bool, least: f64) -> bool {
        if self.full_only && !full {
            return false;
        }
        let step = |slot: usize| if slot == NO_SLOT { Some(0.0) } else { steps[slot].map(|lift| lift.step) };
        let [first, second] = self.lifts;
        let (Some(first_step), Some(second_step)) = (step(first), step(second)) else { return false };
        ((first_step + second_step) + self.fixed_cost) + self.finisher_floor < least
    }
}

/// A route's C where a lifted side still builds (`Route::while_building`): V0's convention, C on that side's lineage as
/// it is today.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum WhileBuilding {
    /// The trio's own C's step (`Trio::finisher_cost`): a trio's route.
    Fixed(f64),
    /// The finisher slot of the route with that side at the base: her own full veteran's route with a lifted side (her
    /// C's parent, the other side as today).
    Slot(usize),
    /// No such route kept (or no lifted side): the route is not taken while a lifted side builds.
    Skip,
}

/// No candidate (`RouteWin::lifted`); every candidate index is below it (`evaluate_outcomes` asserts it).
pub(crate) const NO_CANDIDATE: u16 = u16::MAX;

/// What V after her took at one outcome (`ScoreBase::least_after`): the candidates whose lifted sides its least route
/// takes (`NO_CANDIDATE` in a place it does not fill), and whether her own full veteran is a side of that route or is
/// the goal itself (her parents both full veterans). `NONE` where V after her is V0, no route below it. (Kept per
/// outcome beside V after her, `kernel::BatchLeast`: u16 candidates hold it in 6 bytes.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RouteWin {
    pub(crate) lifted: [u16; 2],
    pub(crate) own_side: bool,
}

impl RouteWin {
    /// No route taken.
    pub(crate) const NONE: Self = Self { lifted: [NO_CANDIDATE; 2], own_side: false };
}

/// V after her at one outcome (`ScoreBase::least_after`) and the route it took there (`RouteWin`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Least {
    pub(crate) value: f64,
    pub(crate) win: RouteWin,
}

/// What V after her reads beyond the lifts at one outcome.
pub(crate) struct AfterHer<'after> {
    /// Her parents are both full veterans: her own full veteran is the goal (V after her 0).
    pub(crate) completes_goal: bool,
    pub(crate) full_mask: usize,
    pub(crate) routes: &'after [Route],
    /// The lift slots (a route's lifts index them).
    pub(crate) lift_slot_count: usize,
    /// Per finisher slot, its C's uma (`Scored::finisher_umas`; empty: every C at the common scale).
    pub(crate) finisher_umas: &'after [crate::uma::UmaIndex],
}

/// Direct-mapped probability cache entry; NaN bits mark an empty entry.
#[derive(Clone, Copy)]
pub(crate) struct CachedStepCost {
    probability_bits: u64,
    cost: f64,
    builds: bool,
}

/// A lifted candidate's step at one outcome, as V after her takes it (a lift slot's), and whether she still builds at
/// the lift's odds.
#[derive(Clone, Copy)]
pub(crate) struct LiftStep {
    pub(crate) step: f64,
    pub(crate) builds: bool,
}

/// How V after her (`v_after`) costs the steps of the candidates her veteran lifts: the estimate's (`StepEstimate`,
/// `Cont::cost`), or a test's stand-in (`score::routes`' tests). Monomorphised
pub(crate) trait StepCost {
    /// Her step at the lift's odds.
    fn step(&mut self, score_base: &ScoreBase, lift: CandidateLift) -> LiftStep;
    /// Count an outcome whose lifts V after her read.
    fn changed(&mut self) {}
}

/// The estimate's steps (`sc_loop`): a step's cost through a per-row cache of the odds costed (`probability_costs`, lift
/// slot j at j; `score_base.cost` is a pure function of them, direct-mapped by the odds' bits), with the counters
/// `Scored::evaluate` reports
#[derive(Default)]
pub(crate) struct StepEstimate {
    pub(crate) workspace: StepWorkspace,
}

/// Probability caches, reused for every outcome in one scoring, and per lift slot what V after her read of it.
#[derive(Default)]
pub(crate) struct StepWorkspace {
    pub(crate) probability_costs: Vec<CachedStepCost>,
    /// Per lift slot, what any outcome's V after her read of it (`LiftRead`).
    pub(crate) lift_reads: Vec<LiftRead>,
    pub(crate) changed_outcomes: usize,
    pub(crate) cost_evaluations: usize,
    pub(crate) cache_hits: usize,
}

const COST_CACHE_CAPACITY: usize = 256;

/// What V after her read of one lift slot over a scoring's outcomes (`StepWorkspace::lift_reads`): the least step it
/// costed (infinite: no outcome lifted it), what a route through the slot costs at least, and whether any outcome left
/// her grinding at once after it (where none did, a route lifting it takes its C while building at every outcome:
/// `score::routes`' lower bounds and dominance).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LiftRead {
    pub(crate) least_step: f64,
    pub(crate) grinds: bool,
}

impl LiftRead {
    /// A slot no outcome has read.
    pub(crate) const UNREAD: Self = Self { least_step: f64::INFINITY, grinds: false };
}

/// `StepEstimate`'s cache buffer between scorings (`FoldScratch`)
#[derive(Default)]
pub(crate) struct EstimateBuffers {
    pub(crate) probability_costs: Vec<CachedStepCost>,
}

impl StepEstimate {
    /// for `rows` lift slots
    pub(crate) fn new(rows: usize, buffers: EstimateBuffers) -> Self {
        // (the cache's buffer reused, refilled: NaN's bits mark an empty slot)
        let EstimateBuffers { mut probability_costs } = buffers;
        probability_costs.clear();
        probability_costs.resize(
            rows * COST_CACHE_CAPACITY,
            CachedStepCost { probability_bits: f64::NAN.to_bits(), cost: 0.0, builds: false },
        );
        let workspace = StepWorkspace {
            probability_costs,
            lift_reads: vec![LiftRead::UNREAD; rows],
            changed_outcomes: 0,
            cost_evaluations: 0,
            cache_hits: 0,
        };
        Self { workspace }
    }
}

impl StepCost for StepEstimate {
    #[inline(always)]
    fn step(&mut self, score_base: &ScoreBase, lift: CandidateLift) -> LiftStep {
        let workspace = &mut self.workspace;
        let probability_bits = lift.probability.to_bits();
        let k = lift.slot * COST_CACHE_CAPACITY + (probability_bits.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as usize;
        let cached = &mut workspace.probability_costs[k];
        let lift_step = if cached.probability_bits == probability_bits {
            workspace.cache_hits += 1;
            LiftStep { step: cached.cost, builds: cached.builds }
        } else {
            workspace.cost_evaluations += 1;
            let (cost, builds) = score_base.step_plan(lift.candidate, lift.probability);
            *cached = CachedStepCost { probability_bits, cost, builds };
            LiftStep { step: cost, builds }
        };
        let read = &mut workspace.lift_reads[lift.slot];
        if lift_step.step < read.least_step {
            read.least_step = lift_step.step;
        }
        read.grinds |= !lift_step.builds;
        lift_step
    }
    fn changed(&mut self) {
        self.workspace.changed_outcomes += 1;
    }
}

impl ScoreBase<'_> {
    /// Expected careers saved at one outcome: V0 less V after her, the least of V0 (every trio as it was) and every
    /// route (`Route`) whose lifts happened at this outcome (and, for one with her own full veteran as a side, where she
    /// landed every target): its lifted steps (each at its lift slot's odds) plus its fixed sides plus C's step,
    /// 1 / its finisher slot's odds here (as the base prices her uma's odds, `ScoreBase::odds_scales`) where every lifted side grinds
    /// at once, else `Route::while_building`'s; the goal's V when her parents are full veterans. `lifted`: one per lift
    /// slot whose odds rose; `finisher_odds` the finisher slots' odds at this outcome; `steps` scratch
    /// (tests: the plan's scorings read `least_after` in their outcome loop)
    #[cfg(test)]
    pub(crate) fn v_after<C: StepCost>(
        &self,
        cost: &mut C,
        steps: &mut Vec<Option<LiftStep>>,
        lifted: &[CandidateLift],
        finisher_odds: &[f64],
        landed_mask: usize,
        after: &AfterHer,
    ) -> f64 {
        (self.current_value - self.least_after(cost, steps, lifted, finisher_odds, landed_mask, after).value).max(0.0)
    }
    /// V after her at one outcome (`v_after`'s saving is V0 less it): the least of V0 and the routes taken there, 0 where
    /// her own full veteran is the goal, with the route it took (`RouteWin`: the first of the least routes, in route
    /// order; the value is the same whichever is named). Where it costs any step (some slot lifted, or she landed every
    /// target), `steps` holds each lifted slot's step afterwards (`Scored::shadows` read them)
    #[inline(always)]
    pub(crate) fn least_after<C: StepCost>(
        &self,
        cost: &mut C,
        steps: &mut Vec<Option<LiftStep>>,
        lifted: &[CandidateLift],
        finisher_odds: &[f64],
        landed_mask: usize,
        after: &AfterHer,
    ) -> Least {
        let full = landed_mask == after.full_mask;
        if after.completes_goal && full {
            return Least { value: 0.0, win: RouteWin { own_side: true, ..RouteWin::NONE } };
        }
        if lifted.is_empty() && !full {
            return Least { value: self.current_value, win: RouteWin::NONE };
        }
        steps.clear();
        steps.resize(after.lift_slot_count, None);
        for &lift in lifted {
            steps[lift.slot] = Some(cost.step(self, lift));
        }
        if !lifted.is_empty() {
            cost.changed();
        }
        let (mut best, mut best_route) = (self.current_value, None);
        let finisher_cost = |slot: usize| {
            let odds = *finisher_odds.get(slot)?;
            let scale = after.finisher_umas.get(slot).map_or(self.odds_scales.common, |&uma| self.odds_scales.of(uma));
            (odds > 0.0).then(|| 1.0 / (odds * scale))
        };
        for (route_index, route) in after.routes.iter().enumerate() {
            if route.full_only && !full {
                continue;
            }
            let step = |slot: usize| {
                if slot == NO_SLOT { Some(LiftStep { step: 0.0, builds: false }) } else { steps[slot] }
            };
            let [first, second] = route.lifts;
            let (Some(first_step), Some(second_step)) = (step(first), step(second)) else { continue };
            let building_cost = || match route.while_building {
                WhileBuilding::Fixed(cost) => Some(cost),
                WhileBuilding::Slot(slot) => finisher_cost(slot),
                WhileBuilding::Skip => None,
            };
            let finisher = if first_step.builds || second_step.builds {
                building_cost()
            } else {
                finisher_cost(route.finisher_slot)
            };
            let Some(finisher) = finisher else { continue };
            let value = ((first_step.step + second_step.step) + route.fixed_cost) + finisher;
            if value < best {
                (best, best_route) = (value, Some(route_index));
            }
        }
        let win = best_route.map_or(RouteWin::NONE, |route_index| {
            let route = &after.routes[route_index];
            // (a lifted slot's candidate: the lift that set its step)
            let candidate_of = |slot: usize| {
                lifted.iter().find(|lift| lift.slot == slot).map_or(NO_CANDIDATE, |lift| lift.candidate as u16)
            };
            RouteWin {
                lifted: route.lifts.map(|slot| if slot == NO_SLOT { NO_CANDIDATE } else { candidate_of(slot) }),
                own_side: route.full_only,
            }
        });
        Least { value: best, win }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::plan::cont::Cont;

    /// One candidate on a synthetic Cont, her step in one route whose other side costs `side` and whose C costs `finisher`
    /// (the same with her lifted side): a builder whose veteran lifts her, with the fitted chance, to the odds the fitted
    /// step closes scores lam (V - V after her) - 1 = 0 where building pays
    #[test]
    fn the_plans_own_next_builder_scores_zero() {
        let (lam, phi, pinf, p0, side, finisher) = (0.3, 0.1, 0.002, 3e-4, 400.0, 500.0);
        let cont = Cont::new(lam, phi, pinf, p0);
        let step = cont.cost(p0);
        let lifted_odds = pinf * ((p0 / pinf).ln() * (1.0 - phi)).exp();
        let base = ScoreBase {
            candidates: &[crate::uma::UmaIndex(0)],
            step_costs: &[step],
            current_value: (step + side) + finisher,
            current_odds: &[p0],
            continuations: std::slice::from_ref(&cont),
            step_caps: &[],
            trios: &[],
            trio_sides: &[],
            finisher_umas: &[],
            knobs: crate::career::plan::cont::ScoringKnobs::default(),
            odds_scales: crate::career::plan::cont::OddsScales::ONE,
        };
        let routes = [Route {
            lifts: [0, NO_SLOT],
            full_only: false,
            fixed_cost: side,
            finisher_slot: 0,
            while_building: WhileBuilding::Fixed(finisher),
        }];
        let after =
            AfterHer { completes_goal: false, full_mask: 1, routes: &routes, lift_slot_count: 1, finisher_umas: &[] };
        let lift = [CandidateLift { slot: 0, candidate: 0, probability: lifted_odds }];
        let finisher_odds = [1.0 / finisher];
        let mut estimate = StepEstimate::new(1, EstimateBuffers::default());
        let saving = base.v_after(&mut estimate, &mut vec![], &lift, &finisher_odds, 0, &after);
        assert!((lam * saving - 1.0).abs() < 1e-9, "score {}", lam * saving - 1.0);
    }

    /// Every lifted step at a fixed cost, never building.
    struct FixedStep(f64);
    impl StepCost for FixedStep {
        fn step(&mut self, _: &ScoreBase, _: CandidateLift) -> LiftStep {
            LiftStep { step: self.0, builds: false }
        }
    }

    /// V after her names the route it takes (`RouteWin`): a lifted side's route names that slot's candidate, her own
    /// full veteran's route her own side, the goal her own side, and no route taken none
    #[test]
    fn least_after_names_the_route_it_takes() {
        let cont = Cont::new(0.3, 0.1, 0.002, 3e-4);
        let base = ScoreBase {
            candidates: &[crate::uma::UmaIndex(0)],
            step_costs: &[10.0],
            current_value: 100.0,
            current_odds: &[3e-4],
            continuations: std::slice::from_ref(&cont),
            step_caps: &[],
            trios: &[],
            trio_sides: &[],
            finisher_umas: &[],
            knobs: crate::career::plan::cont::ScoringKnobs::default(),
            odds_scales: crate::career::plan::cont::OddsScales::ONE,
        };
        // (a lifted side beside a fixed one, 20 + 30 + C 40; her own full veteran beside a fixed side, 0 + 20 + C 40)
        let routes = [
            Route {
                lifts: [0, NO_SLOT],
                full_only: false,
                fixed_cost: 30.0,
                finisher_slot: 0,
                while_building: WhileBuilding::Skip,
            },
            Route {
                lifts: [NO_SLOT, NO_SLOT],
                full_only: true,
                fixed_cost: 20.0,
                finisher_slot: 1,
                while_building: WhileBuilding::Skip,
            },
        ];
        let after =
            AfterHer { completes_goal: false, full_mask: 1, routes: &routes, lift_slot_count: 1, finisher_umas: &[] };
        let lift = [CandidateLift { slot: 0, candidate: 3, probability: 0.01 }];
        let finisher_odds = [1.0 / 40.0, 1.0 / 40.0];
        let least = |lifted: &[CandidateLift], landed_mask: usize, after: &AfterHer| {
            base.least_after(&mut FixedStep(20.0), &mut vec![], lifted, &finisher_odds, landed_mask, after)
        };
        let lifted_route = least(&lift, 0, &after);
        assert!((lifted_route.value - 90.0).abs() < 1e-9, "the lifted route: {}", lifted_route.value);
        assert_eq!(lifted_route.win, RouteWin { lifted: [3, NO_CANDIDATE], own_side: false }, "names candidate 3");
        let own_route = least(&lift, 1, &after);
        assert!((own_route.value - 60.0).abs() < 1e-9, "her own full veteran's route: {}", own_route.value);
        assert_eq!(own_route.win, RouteWin { own_side: true, ..RouteWin::NONE }, "names her own side");
        assert_eq!(least(&[], 0, &after).win, RouteWin::NONE, "no lift, not full: V0, no route");
        let goal = AfterHer { completes_goal: true, ..after };
        let at_goal = least(&lift, 1, &goal);
        assert!(at_goal.value == 0.0 && at_goal.win.own_side, "her full veteran is the goal");
    }
}
