//! The sequenced oracle (`next.oracle: "sequenced"`, a diagnostic; docs/SOLVER.md "Next career"): the plan's cheapest
//! trios with their sides made in turn. V adds A's and B's steps as if each were bred from today's pool; here the side
//! made first leaves her fresh full veteran in the pool (the concrete one a trio's side is: `TrioSideVeteran::legacy`),
//! the second is priced from her best career there (a search with that veteran as a parent, her step at its odds, or
//! her side as today where that is cheaper), and C is bred on the two; the cheaper order counts. It reports beside
//! today's V, and the best rows' scores with the trios at those values; it changes no pick or V. Its scorings and C's
//! values read the cached decks only (`aside`), but its career searches (`env.solve`) can keep proofs and set class
//! flags as any search does (as the lift audit's do), and the output reads the flags.
use crate::career::clock::now_ms;
use crate::career::plan::combine::PlanScores;
use crate::career::plan::cont::{ScoreBase, SideStep, Trio, TrioSlot, Want};
use crate::career::plan::robust::{aside, aside_complete};
use crate::career::plan::rows::PlanRow;
use crate::career::plan::score::score_career;
use crate::career::plan::search::{capped, legal_pair, val};
use crate::career::plan::{Oracle, PlanContext, found_career};
use crate::career::search::SearchRequest;
use crate::career::sim::Sim;
use crate::career::{Career, Legacy, Lineage};
use serde_json::{Value, json};

/// The trios sequenced, today's cheapest first.
const SEQUENCED_TRIOS: usize = 8;

/// The shown rows rescored against the sequenced trios: the top ones by score.
const RESCORED_ROWS: usize = 3;

/// One order of a trio's sides: the first as today, the second after her, C on the two.
struct Ordered {
    /// The second side's careers, and whether her best career with the first's veteran was cheaper than her side today.
    second_careers: f64,
    second_moved: bool,
    finisher_cost: f64,
    total: f64,
}

/// The sequenced oracle, or null without the flag, with a given fit or with nothing scored against a finite V. Per trio
/// of today's SEQUENCED_TRIOS cheapest: today's value and each order's (`Ordered`); `v0_sequenced` the least over them;
/// `best_trio_changes` where that least is not today's cheapest trio's. Then the RESCORED_ROWS best rows rescored
/// (`Want::Score`, the cached decks only: `aside`) against the trios at their sequenced values (each trio's change put in
/// its C term, so the routes' fixed sides and C while building are today's): `rows` with each one's score today and so,
/// and the gaps between them: the scores' sensitivity to putting the sequencing discount in C, not the sequenced rule
/// in V after her (which no route here applies), so no evidence about the pick.
pub(crate) fn sequenced_oracle(
    simulator: &mut Sim,
    context: &PlanContext,
    runs: &[PlanRow],
    scores: &PlanScores,
    buf: &mut Vec<f64>,
) -> Value {
    let started_ms = now_ms();
    let input = context.input;
    let fit = &scores.fit;
    if input.settings.oracle != Some(Oracle::Sequenced)
        || input.settings.fit.is_some()
        || !scores.any_scored
        || !fit.current_value.is_finite()
    {
        return Value::Null;
    }
    let data = context.data();
    let base = fit.score_base(context.candidates);
    let mut order: Vec<usize> = (0..fit.trios.len()).collect();
    order.sort_by(|&first, &second| fit.trio_value(&fit.trios[first]).total_cmp(&fit.trio_value(&fit.trios[second])));
    order.truncate(SEQUENCED_TRIOS);
    let mut sequenced_values: Vec<(usize, f64)> = vec![];
    // (the cached decks only, `aside`: the oracle adds nothing the plan's output or its check reads)
    let trios: Vec<Value> = aside(simulator, |simulator| {
        order
            .iter()
            .map(|&index| {
                let trio = &fit.trios[index];
                let today = fit.trio_value(trio);
                let [first, second] = trio.sides;
                let orders = [
                    sequence(simulator, context, &base, [first, second], buf),
                    sequence(simulator, context, &base, [second, first], buf),
                ];
                let value = orders.iter().flatten().map(|ordered| ordered.total).fold(today, f64::min);
                sequenced_values.push((index, value));
                let name = |uma: crate::uma::UmaIndex| json!(data.umas[uma].name);
                let names = [
                    name(context.candidates[first.candidate]),
                    name(context.candidates[second.candidate]),
                    name(trio.finisher),
                ];
                let order_json = |ordered: &Option<Ordered>| {
                    ordered.as_ref().map(|ordered| {
                        json!({"second": ordered.second_careers, "second_moved": ordered.second_moved,
                        "c": ordered.finisher_cost, "total": ordered.total})
                    })
                };
                json!({"trio": names, "sides": [base.side_cost(first), base.side_cost(second)], "c": trio.finisher_cost,
                "v": today, "first_a": order_json(&orders[0]), "first_b": order_json(&orders[1]), "v_sequenced": value})
            })
            .collect()
    });
    let best_today = order.first().copied();
    let best_sequenced = sequenced_values.iter().min_by(|first, second| first.1.total_cmp(&second.1)).copied();
    let rescored = rescore_top_rows(simulator, context, runs, scores, &sequenced_values, buf);
    json!({
        "v0": fit.current_value,
        "v0_sequenced": best_sequenced.map(|(_, value)| value),
        "best_trio_changes": best_sequenced.map(|(index, _)| index) != best_today,
        "trios": trios,
        "rows": rescored,
        "oracle_ms": now_ms() - started_ms,
    })
}

/// One order of a trio's sides (`Ordered`): None where the first side is a real full veteran already in the pool (the
/// order changes nothing) or no C is legal on the two.
fn sequence(
    simulator: &mut Sim,
    context: &PlanContext,
    base: &ScoreBase,
    [first, second]: [TrioSlot; 2],
    buf: &mut Vec<f64>,
) -> Option<Ordered> {
    let input = context.input;
    let data = context.data();
    let fit_sides = base.trio_sides;
    let first_side = &fit_sides[first.candidate.0][first.side];
    let second_side = &fit_sides[second.candidate.0][second.side];
    if matches!(first_side.step, SideStep::Existing(_)) {
        return None;
    }
    let env_budget = simulator.env.state.budget;
    let mut working_pool = simulator.evaluator.take_pool(input.pool);
    // her fresh full veteran in the pool (and among the legacies a new veteran's parents are read from)
    simulator.env.setup.pool_push(&mut working_pool, &first_side.legacy);
    let first_place = working_pool.veterans.len() - 1;
    let mut legacies: Vec<Legacy> = input.legacies.to_vec();
    legacies.push(first_side.legacy.clone());
    // the second side: as today, or her best career with the first's veteran as a parent where her step there is less
    let today_second = base.side_cost(second);
    let (mut second_careers, mut moved_veteran) = (today_second, None::<Legacy>);
    if !matches!(second_side.step, SideStep::Existing(_)) {
        let trainee = context.candidates[second.candidate];
        let searched = capped(simulator, input.settings.search_work, env_budget, |simulator| {
            simulator.env.solve(SearchRequest {
                pool: &working_pool,
                fixed_parents: &[first_place],
                trainees: &[trainee],
                per_trainee: false,
                top: 1,
            })
        });
        if let Some(found) = searched.found.into_iter().next() {
            let (step, _) = base.step_plan(second.candidate.0, found.value);
            if step < today_second {
                let lineage = Lineage { trainee, first_parent: found.first_parent, second_parent: found.second_parent };
                let career = Career { lineage, deck: &found.deck };
                let record = simulator.evaluator.career_wins(
                    &simulator.env.setup,
                    &mut simulator.env.state,
                    &working_pool,
                    career,
                );
                (second_careers, moved_veteran) =
                    (step, Some(simulator.env.setup.veteran(&legacies, lineage, &record)));
            }
        }
    }
    let second_moved = moved_veteran.is_some();
    let second_veteran = moved_veteran.as_ref().unwrap_or(&second_side.legacy);
    let second_place = match second_side.step {
        SideStep::Existing(legacy) => legacy,
        SideStep::Current | SideStep::Alternative(_) => {
            simulator.env.setup.pool_push(&mut working_pool, second_veteran);
            working_pool.veterans.len() - 1
        }
    };
    // C: the allowed uma, not of A's or B's, at her best odds on the two
    let (first_id, second_id) =
        (data.umas[context.candidates[first.candidate]].id, data.umas[context.candidates[second.candidate]].id);
    let mut best_odds = 0.0f64;
    for &finisher in &context.candidates.allowed_trainees {
        let id = data.umas[finisher].id;
        if id == first_id || id == second_id {
            continue;
        }
        let lineage = Lineage { trainee: finisher, first_parent: first_place, second_parent: second_place };
        if legal_pair(data, &working_pool, lineage) {
            best_odds = best_odds.max(val(simulator, &working_pool, lineage, buf));
        }
    }
    while working_pool.veterans.len() > input.pool.veterans.len() {
        working_pool.pop_veteran();
    }
    simulator.evaluator.put_pool(input.pool, working_pool);
    (best_odds > 0.0).then(|| {
        let finisher_cost = 1.0 / best_odds;
        Ordered {
            second_careers,
            second_moved,
            finisher_cost,
            total: base.side_cost(first) + second_careers + finisher_cost,
        }
    })
}

/// The RESCORED_ROWS best runnable rows by score (`Want::Score` on the cached decks only, `aside`) against the plan's
/// base with each sequenced trio at its sequenced value (`sequenced_values`: (trio index, value); the change put in the
/// trio's C term, `Trio::finisher_cost`), beside their scores today and the gaps between consecutive ones
fn rescore_top_rows(
    simulator: &mut Sim,
    context: &PlanContext,
    runs: &[PlanRow],
    scores: &PlanScores,
    sequenced_values: &[(usize, f64)],
    buf: &mut Vec<f64>,
) -> Value {
    let fit = &scores.fit;
    let mut trios: Vec<Trio> = fit.trios.clone();
    for &(index, value) in sequenced_values {
        let trio = &mut trios[index];
        trio.finisher_cost += value - fit.trio_value(&fit.trios[index]);
    }
    let current_value = trios.iter().map(|trio| fit.trio_value(trio)).fold(f64::INFINITY, f64::min);
    let base = ScoreBase { trios: &trios, current_value, ..fit.score_base(context.candidates) };
    let mut top: Vec<usize> = (0..runs.len())
        .filter(|&i| context.input.is_runnable(&runs[i].career) && runs[i].net_saving().is_some_and(f64::is_finite))
        .collect();
    top.sort_by(|&first, &second| runs[second].total().total_cmp(&runs[first].total()));
    top.truncate(RESCORED_ROWS);
    let (rescored, scoring_complete): (Vec<f64>, bool) = aside_complete(simulator, |simulator| {
        top.iter()
            .map(|&i| {
                let career = found_career(runs[i].trainee, &runs[i].career);
                score_career(simulator, &context.scoring(&base), career, Want::Score, buf).expected_saving - 1.0
            })
            .collect()
    });
    let today: Vec<f64> = top.iter().map(|&i| runs[i].total()).collect();
    let gaps = |values: &[f64]| values.windows(2).map(|pair| pair[0] - pair[1]).collect::<Vec<f64>>();
    json!({"v0": current_value, "trainees": top.iter().map(|&i| context.data().umas[runs[i].trainee].name.clone()).collect::<Vec<_>>(),
        "score": today, "score_sequenced": rescored, "gaps": gaps(&today), "gaps_sequenced": gaps(&rescored),
        "scoring_complete": scoring_complete})
}
