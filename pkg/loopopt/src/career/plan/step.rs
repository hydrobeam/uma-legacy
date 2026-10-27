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
    /// The finisher slot holding its C's odds.
    pub(crate) finisher_slot: usize,
    /// C's step where a lifted side still builds at its lift's odds.
    pub(crate) while_building: WhileBuilding,
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
/// the lift's odds (None: not known, the bound's: V after her then takes the cheaper C of the two).
#[derive(Clone, Copy)]
pub(crate) struct LiftStep {
    pub(crate) step: f64,
    pub(crate) builds: Option<bool>,
}

/// How V after her (`v_after`) costs the steps of the candidates her veteran lifts: the estimate's (`StepEstimate`,
/// `Cont::cost`, for y) or the lower bound's (`Lo`, `ScoreBase::step_floor`, for y_hi). Monomorphised: each path keeps
/// its own code
pub(crate) trait StepCost {
    /// Her step at the lift's odds (the estimate's), or at most every step she could have there (the bound's).
    fn step(&mut self, score_base: &ScoreBase, lift: CandidateLift) -> LiftStep;
    /// Count an outcome whose lifts V after her read.
    fn changed(&mut self) {}
}

/// The bound's steps: each lifted candidate's at `ScoreBase::step_floor` at the lift's odds, at most her step at any
/// odds from hers now to them (`saving_lo`)
pub(crate) struct Lo;

impl StepCost for Lo {
    #[inline]
    fn step(&mut self, score_base: &ScoreBase, lift: CandidateLift) -> LiftStep {
        LiftStep { step: score_base.step_floor(lift.candidate, lift.probability), builds: None }
    }
}

/// The estimate's steps (`sc_loop`): a step's cost through a per-row cache of the odds costed (`probability_costs`, lift
/// slot j at j; `score_base.cost` is a pure function of them, direct-mapped by the odds' bits), with the counters
/// `Scored::evaluate` reports
#[derive(Default)]
pub(crate) struct StepEstimate {
    pub(crate) workspace: StepWorkspace,
}

/// Probability caches, reused for every outcome in one scoring.
#[derive(Default)]
pub(crate) struct StepWorkspace {
    pub(crate) probability_costs: Vec<CachedStepCost>,
    pub(crate) changed_outcomes: usize,
    pub(crate) cost_evaluations: usize,
    pub(crate) cache_hits: usize,
}

const COST_CACHE_CAPACITY: usize = 256;

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
        Self { workspace: StepWorkspace { probability_costs, changed_outcomes: 0, cost_evaluations: 0, cache_hits: 0 } }
    }
}

impl StepCost for StepEstimate {
    #[inline(always)]
    fn step(&mut self, score_base: &ScoreBase, lift: CandidateLift) -> LiftStep {
        let workspace = &mut self.workspace;
        let probability_bits = lift.probability.to_bits();
        let k = lift.slot * COST_CACHE_CAPACITY + (probability_bits.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as usize;
        let cached = &mut workspace.probability_costs[k];
        if cached.probability_bits == probability_bits {
            workspace.cache_hits += 1;
            LiftStep { step: cached.cost, builds: Some(cached.builds) }
        } else {
            workspace.cost_evaluations += 1;
            let (cost, builds) = score_base.step_plan(lift.candidate, lift.probability);
            *cached = CachedStepCost { probability_bits, cost, builds };
            LiftStep { step: cost, builds: Some(builds) }
        }
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
    /// at once, else `Route::while_building`'s (the bound, not knowing, takes the cheaper of the two); the goal's V when
    /// her parents are full veterans. `lifted`: one per lift slot whose odds rose; `finisher_odds` the finisher slots'
    /// odds at this outcome; `steps` scratch. Both the estimate and the bound use this sequence, with their own StepCost
    #[inline(always)]
    pub(crate) fn v_after<C: StepCost>(
        &self,
        cost: &mut C,
        steps: &mut Vec<Option<LiftStep>>,
        lifted: &[CandidateLift],
        finisher_odds: &[f64],
        landed_mask: usize,
        after: &AfterHer,
    ) -> f64 {
        let full = landed_mask == after.full_mask;
        if after.completes_goal && full {
            return self.current_value;
        }
        if lifted.is_empty() && !full {
            return 0.0;
        }
        steps.clear();
        steps.resize(after.lift_slot_count, None);
        for &lift in lifted {
            steps[lift.slot] = Some(cost.step(self, lift));
        }
        if !lifted.is_empty() {
            cost.changed();
        }
        let mut best = self.current_value;
        let finisher_cost = |slot: usize| {
            let odds = finisher_odds[slot];
            let scale = after.finisher_umas.get(slot).map_or(self.odds_scales.common, |&uma| self.odds_scales.of(uma));
            (odds > 0.0).then(|| 1.0 / (odds * scale))
        };
        for route in after.routes {
            if route.full_only && !full {
                continue;
            }
            let step = |slot: usize| {
                if slot == NO_SLOT { Some(LiftStep { step: 0.0, builds: Some(false) }) } else { steps[slot] }
            };
            let [first, second] = route.lifts;
            let (Some(first_step), Some(second_step)) = (step(first), step(second)) else { continue };
            let building_cost = || match route.while_building {
                WhileBuilding::Fixed(cost) => Some(cost),
                WhileBuilding::Slot(slot) => finisher_cost(slot),
                WhileBuilding::Skip => None,
            };
            let finisher = match (first_step.builds, second_step.builds) {
                (Some(false), Some(false)) => finisher_cost(route.finisher_slot),
                (Some(true), _) | (_, Some(true)) => building_cost(),
                // (the bound: either C, so the cheaper)
                _ => match (finisher_cost(route.finisher_slot), building_cost()) {
                    (Some(own), Some(building)) => Some(own.min(building)),
                    (own, building) => own.or(building),
                },
            };
            let Some(finisher) = finisher else { continue };
            let value = ((first_step.step + second_step.step) + route.fixed_cost) + finisher;
            if value < best {
                best = value;
            }
        }
        (self.current_value - best).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::plan::cont::Cont;

    /// One candidate on a synthetic Cont, her step in one route whose other side costs `side` and whose C costs `finisher`
    /// (the same with her lifted side): a builder whose veteran lifts her, with the fitted chance, to the odds the fitted
    /// step closes scores lam (V - V after her) - 1 = 0 where building pays, by the estimate, and at least that by the
    /// bound
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
            certified: &[true],
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
        let bound = base.v_after(&mut Lo, &mut vec![], &lift, &finisher_odds, 0, &after);
        assert!(bound >= saving, "y_hi {bound} below y {saving}");
    }
}
