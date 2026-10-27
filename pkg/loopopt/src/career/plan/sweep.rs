//! The pair sweep (`next.pair_sweep`, a measurement): for the best-scoring trainees, their careers on many parent pairs
//! scored exactly, to see how far choosing a trainee's career by P(all) (her best `builder_careers` careers) misses the
//! career the score prefers. It reports and changes nothing.
use crate::career::Lineage;
use crate::career::clock::now_ms;
use crate::career::outcomes::interp_in;
use crate::career::plan::combine::PlanScores;
use crate::career::plan::cont::Want;
use crate::career::plan::robust::aside;
use crate::career::plan::rows::PlanRow;
use crate::career::plan::score::score_career;
use crate::career::plan::{PlanContext, found_career};
use crate::career::search::{Found, RankedPair};
use crate::career::sim::Sim;
use serde_json::{Value, json};

/// Trainees swept: the best-scoring ones (by their best runnable row).
const SWEEP_TRAINEES: usize = 4;

/// Pairs swept per trainee: every legal pair, or this many by their P(all) ceiling (`CareerSetup::ranked_pairs`).
const SWEEP_PAIRS: usize = 40;

/// The sweep, or null unless the job asks for it (`PlanJob::pair_sweep`) and rows were scored against a finite V. Per
/// trainee, per pair in ceiling order: its ceiling (the pair's P(all) bound), the P(all) of her best cached deck there
/// and that career's score (`Want::Score` against the plan's base, the cached decks only: `aside`), and every cached
/// deck's P(all) on the pair; `decks` scores every cached deck on her best-scoring pair.
pub(crate) fn pair_sweep(
    simulator: &mut Sim,
    context: &PlanContext,
    runs: &[PlanRow],
    scores: &PlanScores,
    buf: &mut Vec<f64>,
) -> Value {
    let input = context.input;
    if !input.settings.pair_sweep || !scores.any_scored || !scores.fit.current_value.is_finite() {
        return Value::Null;
    }
    let started_ms = now_ms();
    let data = context.data();
    let base = scores.fit.score_base(context.candidates);
    // the trainees, best row score first
    let mut ranked: Vec<(crate::uma::UmaIndex, f64)> = vec![];
    for row in runs.iter().filter(|row| input.is_runnable(&row.career)) {
        let Some(score) = row.net_saving().filter(|score| score.is_finite()) else { continue };
        match ranked.iter_mut().find(|(trainee, _)| *trainee == row.trainee) {
            Some(entry) => entry.1 = entry.1.max(score),
            None => ranked.push((row.trainee, score)),
        }
    }
    ranked.sort_by(|first, second| second.1.total_cmp(&first.1));
    ranked.truncate(SWEEP_TRAINEES);
    let parents = context.parents();
    let trainees: Vec<Value> = aside(simulator, |simulator| {
        ranked
            .iter()
            .map(|&(trainee, row_score)| {
                let pairs: Vec<RankedPair> = simulator
                    .env
                    .setup
                    .ranked_pairs(&mut simulator.env.state, parents.pool, parents.fixed_parents, trainee, |_, _| true)
                    .take(SWEEP_PAIRS)
                    .collect();
                let (mut chances, mut scratch) = (vec![], vec![]);
                let mut best_pair: Option<(f64, Lineage)> = None;
                let swept: Vec<Value> = pairs
                    .iter()
                    .map(|&RankedPair { ceiling, first_parent, second_parent }| {
                        let lineage = Lineage { trainee, first_parent, second_parent };
                        let deck_odds = deck_odds(simulator, parents.pool, lineage, &mut chances, &mut scratch);
                        let Some(best) = (0..deck_odds.len()).max_by(|&a, &b| deck_odds[a].total_cmp(&deck_odds[b]))
                        else {
                            return json!({"parents": [first_parent, second_parent], "ceiling": ceiling});
                        };
                        let career = Found {
                            trainee,
                            first_parent,
                            second_parent,
                            value: deck_odds[best],
                            ceiling,
                            deck: simulator.evaluator.cache.decks[trainee][best].deck.clone(),
                        };
                        let score = score_career(
                            simulator,
                            &context.scoring(&base),
                            found_career(trainee, &career),
                            Want::Score,
                            buf,
                        )
                        .expected_saving
                            - 1.0;
                        if best_pair.is_none_or(|(best_score, _)| score > best_score) {
                            best_pair = Some((score, lineage));
                        }
                        json!({"parents": [first_parent, second_parent], "ceiling": ceiling, "p_all": deck_odds[best],
                            "score": score, "deck_odds": deck_odds})
                    })
                    .collect();
                // every cached deck on her best-scoring pair, scored
                let decks: Vec<Value> = best_pair
                    .map(|(_, lineage)| {
                        let deck_odds = deck_odds(simulator, parents.pool, lineage, &mut chances, &mut scratch);
                        (0..deck_odds.len())
                            .map(|deck| {
                                let career = Found {
                                    trainee,
                                    first_parent: lineage.first_parent,
                                    second_parent: lineage.second_parent,
                                    value: deck_odds[deck],
                                    ceiling: deck_odds[deck],
                                    deck: simulator.evaluator.cache.decks[trainee][deck].deck.clone(),
                                };
                                let scored = score_career(
                                    simulator,
                                    &context.scoring(&base),
                                    found_career(trainee, &career),
                                    Want::Score,
                                    buf,
                                );
                                json!({"p_all": deck_odds[deck], "score": scored.expected_saving - 1.0})
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                json!({"trainee": data.umas[trainee].name, "row_score": row_score, "pairs": swept, "decks": decks})
            })
            .collect()
    });
    json!({"trainees": trainees, "sweep_ms": now_ms() - started_ms})
}

/// Her P(all) with each of her cached decks on `lineage` (their corner tables at its chances, as
/// `alternative_deck_careers` values them: no deck search)
pub(crate) fn deck_odds(
    simulator: &Sim,
    pool: &crate::career::pool::Pool,
    lineage: Lineage,
    chances: &mut Vec<f64>,
    scratch: &mut Vec<f64>,
) -> Vec<f64> {
    let setup = &simulator.env.setup;
    let spark_scale = setup.lineage_into(pool, lineage, chances);
    let misses: Vec<f64> = setup.uncertain_targets.iter().map(|&i| 1.0 - chances[i]).collect();
    simulator.evaluator.cache.decks[lineage.trainee]
        .iter()
        .map(|cached| {
            spark_scale * cached.tables.iter().map(|table| interp_in(table, &misses, scratch)).fold(0.0f64, f64::max)
        })
        .collect()
}
