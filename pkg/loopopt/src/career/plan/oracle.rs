//! The joint-lift oracle (`next.oracle: "joint"`, a diagnostic; docs/SOLVER.md "Next career"): the plan's cheapest
//! trios priced on a joint two-side lattice of their continuations' own paths, where one builder career can lift both
//! sides at once and is charged once, beside today's V, which adds the two sides' continuations as if every lift were a
//! career of its own. It reports and changes nothing.
use crate::career::clock::now_ms;
use crate::career::plan::combine::{FITTED_MODEL, PlanScores, fit_continuation};
use crate::career::plan::cont::{Cont, PathPoint, SideStep, TrioSlot, Want};
use crate::career::plan::robust::aside;
use crate::career::plan::rows::PlanRow;
use crate::career::plan::score::score_career;
use crate::career::plan::{Oracle, PlanContext, found_career};
use crate::career::sim::Sim;
use serde_json::{Value, json};

/// The trios the oracle prices, today's cheapest first.
const ORACLE_TRIOS: usize = 8;

/// One side of the lattice: per lifts in (0 to its cap) her odds then, and below the cap her own builder's lift chance
/// there (1 / the careers success n + 1 takes on her continuation's path, `Cont::building_path`)
pub(crate) struct SidePath {
    odds: Vec<f64>,
    rates: Vec<f64>,
}

impl SidePath {
    /// Her continuation's path from odds p.
    pub(crate) fn of(continuation: &Cont, p: f64) -> Self {
        let path = continuation.building_path(p);
        let rates = path.windows(2).map(|pair| 1.0 / (pair[1].built - pair[0].built)).collect();
        Self { odds: path.iter().map(|&PathPoint { odds, .. }| odds).collect(), rates }
    }
    const fn cap(&self) -> usize {
        self.rates.len()
    }
}

/// A builder career's chances, at the lattice's start, of lifting the first side and the second together, the first
/// alone, the second alone
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct JointLift {
    pub(crate) both: f64,
    pub(crate) first_only: f64,
    pub(crate) second_only: f64,
}

impl JointLift {
    const fn of_side(self, side: usize) -> f64 {
        if side == 0 { self.both + self.first_only } else { self.both + self.second_only }
    }
}

/// The joint two-side lattice (`value`). Assumed beyond Cont: a lift moves its side one step along her path whichever
/// builder made it; after n lifts a side's lift chance from any builder is its chance at the start thinned by
/// `min(1, rate_n / own_chance)`, the two thinnings independent (her own builder then lifts her at
/// `min(own_chance, rate_n)`: Cont's rate where her chance at the start is at least it, else slower, so a side whose
/// rescored chance fell below Cont's lam costs more than Cont's step);
/// a side's grind is a commitment (careers 1 / her odds, nothing lifted meanwhile); C's step is the trio's, unchanged
pub(crate) struct JointLattice<'lattice> {
    pub(crate) sides: [&'lattice SidePath; 2],
    pub(crate) builders: &'lattice [JointLift],
    /// Per side, the chance at the start that her own builder (the one her continuation's lam is from) lifts her.
    pub(crate) own_chance: [f64; 2],
}

impl JointLattice<'_> {
    /// A side's thinning after n lifts (0 at her cap: no further lift).
    fn thinning(&self, side: usize, n: usize) -> f64 {
        let path = self.sides[side];
        if n >= path.cap() || !(self.own_chance[side] > 0.0) {
            return 0.0;
        }
        (path.rates[n] / self.own_chance[side]).min(1.0)
    }
    /// One side left (the other's full veteran in): per lifts n her expected careers to her full veteran, the least of
    /// grinding and each builder's careers (self-loop: (1 + q J(n + 1)) / q, q its lift chance there)
    fn one_side(&self, side: usize) -> Vec<f64> {
        let path = self.sides[side];
        let mut values = vec![0.0; path.cap() + 1];
        for n in (0..=path.cap()).rev() {
            let mut best = 1.0 / path.odds[n];
            let thinning = self.thinning(side, n);
            for builder in self.builders {
                let lift = builder.of_side(side) * thinning;
                if lift > 0.0 {
                    best = best.min((1.0 + lift * values[n + 1]) / lift);
                }
            }
            values[n] = best;
        }
        values
    }
    /// The expected careers to both sides' full veterans, solved exactly over the lattice (every state's successors
    /// have more lifts or a side done, so states are solved from the caps down, each by the self-loop formula
    /// `J = (1 + sum over moves P J') / P(move)` per builder, or a side's grind)
    pub(crate) fn value(&self) -> f64 {
        let (first_cap, second_cap) = (self.sides[0].cap(), self.sides[1].cap());
        let (first_left, second_left) = (self.one_side(0), self.one_side(1));
        let mut next_row = vec![0.0; second_cap + 1];
        let mut row = vec![0.0; second_cap + 1];
        for i in (0..=first_cap).rev() {
            let first_thinning = self.thinning(0, i);
            for j in (0..=second_cap).rev() {
                let second_thinning = self.thinning(1, j);
                let mut best =
                    (1.0 / self.sides[0].odds[i] + second_left[j]).min(1.0 / self.sides[1].odds[j] + first_left[i]);
                for builder in self.builders {
                    let both = builder.both * first_thinning * second_thinning;
                    let first_only = (builder.first_only + builder.both * (1.0 - second_thinning)) * first_thinning;
                    let second_only = (builder.second_only + builder.both * (1.0 - first_thinning)) * second_thinning;
                    let moves = both + first_only + second_only;
                    if !(moves > 0.0) {
                        continue;
                    }
                    // (a lift past a cap has thinning 0: its mass stays)
                    let after = |value: Option<f64>, chance: f64| {
                        if chance > 0.0 { chance * value.unwrap_or(0.0) } else { 0.0 }
                    };
                    let total = 1.0
                        + after(next_row.get(j).copied(), first_only)
                        + after(row.get(j + 1).copied(), second_only)
                        + after(next_row.get(j + 1).copied(), both);
                    best = best.min(total / moves);
                }
                row[j] = best;
            }
            std::mem::swap(&mut row, &mut next_row);
        }
        next_row[0]
    }
}

/// A trio side's odds on her continuation's path: her best career's now, or the other career's (`Alternative`).
fn side_odds(context: &PlanContext, scores: &PlanScores, slot: TrioSlot) -> Option<f64> {
    let side = &scores.fit.sides[slot.candidate.0][slot.side];
    match side.step {
        SideStep::Existing(_) => None,
        SideStep::Current => Some(context.candidates.current_odds[slot.candidate.0]),
        SideStep::Alternative(_) => Some(side.odds),
    }
}

/// The oracle (`next.oracle`): per trio of today's ORACLE_TRIOS cheapest, A's and B's builders (the base rows each one's
/// continuation took lam and phi from, `fit_continuation`), their joint lift sets over every outcome (`Want::Joint`
/// against the placeholder base the lift statistics read, the cached decks only: `aside`), and V on the lattice
/// (`JointLattice`, + C's step): `v_joint` with each builder's joint lifts, `v_separate` with each builder lifting only
/// her own side (the reduction to today's V up to the lattice's thinning and the rescoring's lift chances), beside
/// today's `v`. A side's path is her continuation's state-local one (`Cont::building_path`), which the plan's cost
/// follows at and below her anchor only (above it the plan reads her lattice): a trio with a side above its anchor is
/// marked `above_anchor`, and the joint lifts' effect is `v_joint` against `v_separate` (the same paths), not against
/// `v` (`v0_separate` beside `v0_joint`). A trio with a real full veteran side has no lattice (its value today's). Null
/// without the flag, with a given fit or with nothing scored against a finite V
pub(crate) fn joint_oracle(
    simulator: &mut Sim,
    context: &PlanContext,
    runs: &[PlanRow],
    scores: &PlanScores,
    buf: &mut Vec<f64>,
) -> Value {
    let started_ms = now_ms();
    let PlanContext { input, candidates } = *context;
    let fit = &scores.fit;
    if input.settings.oracle != Some(Oracle::Joint)
        || input.settings.fit.is_some()
        || !scores.any_scored
        || !fit.current_value.is_finite()
    {
        return Value::Null;
    }
    let data = context.data();
    let mut order: Vec<usize> = (0..fit.trios.len()).collect();
    order.sort_by(|&first, &second| fit.trio_value(&fit.trios[first]).total_cmp(&fit.trio_value(&fit.trios[second])));
    order.truncate(ORACLE_TRIOS);
    // per candidate, the row her continuation's builder scored as (an exactly searched builder row)
    let builder_row = |candidate: usize| Some(fit_continuation(fit.evidence[candidate].as_ref()?, FITTED_MODEL).1);
    let stats_base = candidates.stats_base();
    let mut joint_sets: crate::FxMap<usize, Vec<(u64, f64)>> = crate::FxMap::default();
    let trios: Vec<Value> = aside(simulator, |simulator| {
        order
            .iter()
            .map(|&index| {
                let trio = &fit.trios[index];
                let today = fit.trio_value(trio);
                let [first, second] = trio.sides;
                let names = json!([
                    data.umas[candidates[first.candidate]].name,
                    data.umas[candidates[second.candidate]].name,
                    data.umas[trio.finisher].name
                ]);
                let (Some(first_odds), Some(second_odds)) =
                    (side_odds(context, scores, first), side_odds(context, scores, second))
                else {
                    return json!({"trio": names, "v": today, "v_joint": today, "lattice": false});
                };
                let rows = [builder_row(first.candidate.0), builder_row(second.candidate.0)];
                let mut lifts = [JointLift::default(); 2];
                for (side, row) in rows.iter().enumerate() {
                    let Some(row) = *row else { continue };
                    let sets = joint_sets.entry(row).or_insert_with(|| {
                        let career = found_career(runs[row].trainee, &runs[row].career);
                        score_career(simulator, &context.scoring(&stats_base), career, Want::Joint, buf).joint_lifts
                    });
                    let (first_bit, second_bit) = (1u64 << first.candidate.0, 1u64 << second.candidate.0);
                    for &(lifted, chance) in sets.iter() {
                        match (lifted & first_bit != 0, lifted & second_bit != 0) {
                            (true, true) => lifts[side].both += chance,
                            (true, false) => lifts[side].first_only += chance,
                            (false, true) => lifts[side].second_only += chance,
                            (false, false) => {}
                        }
                    }
                }
                let paths = [
                    SidePath::of(&fit.continuations[first.candidate.0], first_odds),
                    SidePath::of(&fit.continuations[second.candidate.0], second_odds),
                ];
                let own_chance = [lifts[0].of_side(0), lifts[1].of_side(1)];
                let same_builder = rows[0].is_some() && rows[0] == rows[1];
                let builders: Vec<JointLift> = if same_builder { vec![lifts[0]] } else { lifts.to_vec() };
                let separate = [
                    JointLift { first_only: own_chance[0], ..JointLift::default() },
                    JointLift { second_only: own_chance[1], ..JointLift::default() },
                ];
                let lattice = |builders: &[JointLift]| {
                    JointLattice { sides: [&paths[0], &paths[1]], builders, own_chance }.value() + trio.finisher_cost
                };
                let above_anchor = [(first, first_odds), (second, second_odds)].iter().any(|&(slot, odds)| {
                    let continuation = &fit.continuations[slot.candidate.0];
                    !continuation.lattice.is_empty() && odds > continuation.anchor_odds
                });
                json!({"trio": names, "v": today, "v_joint": lattice(&builders), "v_separate": lattice(&separate),
                    "lattice": true, "above_anchor": above_anchor, "same_builder": same_builder,
                    "lifts": lifts.iter().map(|lift| [lift.both, lift.first_only, lift.second_only]).collect::<Vec<_>>(),
                    "lam": [fit.continuations[first.candidate.0].lam, fit.continuations[second.candidate.0].lam]})
            })
            .collect()
    });
    let value_of = |trio: &Value| trio["v_joint"].as_f64().unwrap_or(f64::INFINITY);
    // (a trio without a lattice: today's value both ways)
    let separate_of =
        |trio: &Value| trio["v_separate"].as_f64().or_else(|| trio["v"].as_f64()).unwrap_or(f64::INFINITY);
    let best_joint =
        (0..trios.len()).min_by(|&first, &second| value_of(&trios[first]).total_cmp(&value_of(&trios[second])));
    json!({
        "v0": fit.current_value,
        "v0_joint": best_joint.map(|best| value_of(&trios[best])),
        "v0_separate": trios.iter().map(separate_of).fold(f64::INFINITY, f64::min),
        "above_anchor": trios.iter().any(|trio| trio["above_anchor"] == true),
        "best_trio_changes": best_joint.is_some_and(|best| best != 0),
        "trios": trios,
        "oracle_ms": now_ms() - started_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A side that needs one lift (its own builder's chance `rate`) to grind in one career, and 1e6 careers without.
    fn one_lift_side(rate: f64) -> SidePath {
        SidePath { odds: vec![1e-6, 1.0], rates: vec![rate] }
    }

    #[test]
    fn coincident_lifts_take_one_waiting_time_and_disjoint_ones_the_sum() {
        let (first, second) = (one_lift_side(0.5), one_lift_side(0.5));
        let sides = [&first, &second];
        let own_chance = [0.5, 0.5];
        // one builder lifting both at once with chance 1/2: 2 careers' wait, then the two grinds
        let together = [JointLift { both: 0.5, ..JointLift::default() }];
        let coincident = JointLattice { sides, builders: &together, own_chance }.value();
        assert!((coincident - 4.0).abs() < 1e-12, "{coincident}");
        // two builders, each lifting one side with chance 1/2: the two tasks' sum, 3 + 3
        let apart = [
            JointLift { first_only: 0.5, ..JointLift::default() },
            JointLift { second_only: 0.5, ..JointLift::default() },
        ];
        let disjoint = JointLattice { sides, builders: &apart, own_chance }.value();
        assert!((disjoint - 6.0).abs() < 1e-12, "{disjoint}");
        // one builder lifting one side or the other, each with chance 1/2: the collector's 3 careers, then the grinds
        let either = [JointLift { first_only: 0.5, second_only: 0.5, ..JointLift::default() }];
        let exclusive = JointLattice { sides, builders: &either, own_chance }.value();
        assert!((exclusive - 5.0).abs() < 1e-12, "{exclusive}");
    }

    #[test]
    fn disjoint_builders_reduce_to_the_two_continuations() {
        let first = Cont::new(0.3, 0.1, 0.002, 3e-4);
        let second = Cont::new(0.5, 0.5, 0.01, 0.001);
        let (first_path, second_path) = (SidePath::of(&first, 3e-4), SidePath::of(&second, 0.001));
        let builders = [
            JointLift { first_only: 0.3, ..JointLift::default() },
            JointLift { second_only: 0.5, ..JointLift::default() },
        ];
        let lattice = JointLattice { sides: [&first_path, &second_path], builders: &builders, own_chance: [0.3, 0.5] };
        let expected = first.cost(3e-4) + second.cost(0.001);
        let value = lattice.value();
        assert!((value - expected).abs() <= 1e-9 * expected, "{value} vs {expected}");
        // a builder lifting both at once instead can only cost less
        let shared = [JointLift { both: 0.3, second_only: 0.2, ..JointLift::default() }];
        let joint = JointLattice { builders: &shared, ..lattice }.value();
        assert!(joint < expected, "{joint} vs {expected}");
    }

    /// A side whose own builder's chance at the start is below her continuation's rate is lifted at that chance
    /// (`min(own_chance, rate_n)`), so the lattice prices her above Cont's step: the one-side optimum over her path at
    /// the clipped rates
    #[test]
    fn a_clipped_own_chance_prices_her_at_the_slower_rate() {
        let first = Cont::new(0.3, 0.1, 0.002, 3e-4);
        let second = Cont::new(0.5, 0.5, 0.01, 0.001);
        let (first_path, second_path) = (SidePath::of(&first, 3e-4), SidePath::of(&second, 0.001));
        let own = 0.15;
        let builders = [
            JointLift { first_only: own, ..JointLift::default() },
            JointLift { second_only: 0.5, ..JointLift::default() },
        ];
        let lattice = JointLattice { sides: [&first_path, &second_path], builders: &builders, own_chance: [own, 0.5] };
        // her path at the clipped rates, solved by its definition: the least of grinding now and one lift's wait first
        let mut clipped = vec![0.0; first_path.cap() + 1];
        for n in (0..=first_path.cap()).rev() {
            let grind = 1.0 / first_path.odds[n];
            clipped[n] = if n == first_path.cap() {
                grind
            } else {
                let rate = own.min(first_path.rates[n]);
                grind.min(1.0 / rate + clipped[n + 1])
            };
        }
        let expected = clipped[0] + second.cost(0.001);
        let value = lattice.value();
        assert!((value - expected).abs() <= 1e-9 * expected, "{value} vs {expected}");
        assert!(value > first.cost(3e-4) + second.cost(0.001), "clipped {value} not above Cont's");
    }
}
