//! Coordinate ascent over (trainee, parent pair) by score (`next.pair_ascent`, a measurement): from a start, her best
//! pair over every legal pair, then the best trainee on that pair, until neither changes; each (trainee, pair) her best
//! cached deck there (by P(all)), scored exactly as a row is, the cached decks only (`aside`). It reports, beside the
//! pool's facts (legal pairs per trainee, the cost of one scoring and of the deck's P(all) on a pair), and changes
//! nothing.
use crate::FxMap;
use crate::career::Lineage;
use crate::career::clock::now_ms;
use crate::career::plan::combine::PlanScores;
use crate::career::plan::cont::{ScoreBase, Want};
use crate::career::plan::robust::aside;
use crate::career::plan::rows::{PlanPick, PlanRow, pick_of};
use crate::career::plan::score::score_career;
use crate::career::plan::search::legal_pair;
use crate::career::plan::sweep::deck_odds;
use crate::career::plan::{PlanContext, found_career};
use crate::career::search::{Found, RankedPair};
use crate::career::sim::Sim;
use crate::uma::UmaIndex;
use serde_json::{Value, json};

/// Starts after the plan's pick: the best-scoring trainees' best rows.
const ASCENT_STARTS: usize = 3;

/// The scorings an ascent shares (memoised per (trainee, ordered pair)), and their costs.
struct Scorer<'base> {
    base: ScoreBase<'base>,
    scores: FxMap<(UmaIndex, usize, usize), Option<f64>>,
    scorings: usize,
    score_ms: f64,
    odds_evaluations: usize,
    odds_ms: f64,
    chances: Vec<f64>,
    scratch: Vec<f64>,
}

impl Scorer<'_> {
    /// Her score on `lineage` with her best cached deck there (None: no cached deck), memoised
    fn score(
        &mut self,
        simulator: &mut Sim,
        context: &PlanContext,
        lineage: Lineage,
        buf: &mut Vec<f64>,
    ) -> Option<f64> {
        let key = (lineage.trainee, lineage.first_parent, lineage.second_parent);
        if let Some(&score) = self.scores.get(&key) {
            return score;
        }
        let started = now_ms();
        let odds = deck_odds(simulator, context.input.pool, lineage, &mut self.chances, &mut self.scratch);
        self.odds_ms += now_ms() - started;
        self.odds_evaluations += 1;
        let best = (0..odds.len()).max_by(|&first, &second| odds[first].total_cmp(&odds[second]));
        let score = best.map(|deck| {
            let career = Found {
                trainee: lineage.trainee,
                first_parent: lineage.first_parent,
                second_parent: lineage.second_parent,
                value: odds[deck],
                ceiling: odds[deck],
                deck: simulator.evaluator.cache.decks[lineage.trainee][deck].deck.clone(),
            };
            let started = now_ms();
            let scored = score_career(
                simulator,
                &context.scoring(&self.base),
                found_career(lineage.trainee, &career),
                Want::Score,
                buf,
            );
            self.score_ms += now_ms() - started;
            self.scorings += 1;
            scored.expected_saving - 1.0
        });
        self.scores.insert(key, score);
        score
    }
}

/// One ascent's end: the trainee, pair and score it stops at, and its rounds.
struct Ascent {
    lineage: Lineage,
    score: f64,
    rounds: usize,
}

/// Every legal pair of `trainee` (`ranked_pairs`, by ceiling)
fn legal_pairs(simulator: &mut Sim, context: &PlanContext, trainee: UmaIndex) -> Vec<(usize, usize)> {
    let parents = context.parents();
    simulator
        .env
        .setup
        .ranked_pairs(&mut simulator.env.state, parents.pool, parents.fixed_parents, trainee, |_, _| true)
        .map(|RankedPair { first_parent, second_parent, .. }| (first_parent, second_parent))
        .collect()
}

/// From `start` (its score known): her best pair, then the best trainee on it (allowed umas in uma order, with a
/// legal pair and a cached deck), until neither changes; a change only on a strictly higher score
fn ascend(
    simulator: &mut Sim,
    context: &PlanContext,
    scorer: &mut Scorer,
    start: Lineage,
    buf: &mut Vec<f64>,
) -> Option<Ascent> {
    let data = context.data();
    let mut current = start;
    let mut best = scorer.score(simulator, context, current, buf)?;
    let mut trainees = context.candidates.allowed_trainees.clone();
    trainees.sort_unstable();
    let mut rounds = 0;
    loop {
        rounds += 1;
        let before = current;
        // her best pair
        for (first_parent, second_parent) in legal_pairs(simulator, context, current.trainee) {
            let lineage = Lineage { trainee: current.trainee, first_parent, second_parent };
            if let Some(score) = scorer.score(simulator, context, lineage, buf)
                && score > best
            {
                (best, current) = (score, lineage);
            }
        }
        // the best trainee on it
        let pair = (current.first_parent, current.second_parent);
        for &trainee in &trainees {
            let lineage = Lineage { trainee, first_parent: pair.0, second_parent: pair.1 };
            if trainee == current.trainee
                || !legal_pair(data, context.input.pool, lineage)
                || simulator.evaluator.cache.decks[trainee].is_empty()
            {
                continue;
            }
            if let Some(score) = scorer.score(simulator, context, lineage, buf)
                && score > best
            {
                (best, current) = (score, lineage);
            }
        }
        if current == before {
            return Some(Ascent { lineage: current, score: best, rounds });
        }
    }
}

/// The ascent measurement, or null unless the job asks for it (`PlanJob::pair_ascent`) and rows were scored against a
/// finite V: the pool's facts (`legal_pairs`: per allowed uma with a cached deck, her legal pairs: count, min, median,
/// max), then per start (the plan's pick, then the ASCENT_STARTS best-scoring trainees' best rows) where it ends, its
/// rounds, the scorings it added and its ms, and the cost per scoring and per P(all) evaluation over all of them
pub(crate) fn pair_ascent(
    simulator: &mut Sim,
    context: &PlanContext,
    runs: &[PlanRow],
    scores: &PlanScores,
    buf: &mut Vec<f64>,
) -> Value {
    let input = context.input;
    if !input.settings.pair_ascent || !scores.any_scored || !scores.fit.current_value.is_finite() {
        return Value::Null;
    }
    let started_ms = now_ms();
    let data = context.data();
    // the starts: the plan's pick, then the best-scoring trainees' best rows
    let undup: Vec<usize> = (0..runs.len()).map(|i| (0..i).find(|&j| runs[j].repeats(&runs[i])).unwrap_or(i)).collect();
    let PlanPick { row: pick, .. } = pick_of(runs, |career| input.is_runnable(career), &undup);
    let mut starts: Vec<Lineage> = runs.get(pick).map(PlanRow::lineage).into_iter().collect();
    let mut by_trainee: Vec<(f64, Lineage)> = vec![];
    for row in runs.iter().filter(|row| input.is_runnable(&row.career)) {
        let Some(score) = row.net_saving().filter(|score| score.is_finite()) else { continue };
        match by_trainee.iter_mut().find(|(_, lineage)| lineage.trainee == row.trainee) {
            Some(entry) if score > entry.0 => *entry = (score, row.lineage()),
            Some(_) => {}
            None => by_trainee.push((score, row.lineage())),
        }
    }
    by_trainee.sort_by(|first, second| second.0.total_cmp(&first.0));
    starts.extend(by_trainee.iter().take(ASCENT_STARTS).map(|&(_, lineage)| lineage));
    let base = scores.fit.score_base(context.candidates);
    let mut scorer = Scorer {
        base,
        scores: FxMap::default(),
        scorings: 0,
        score_ms: 0.0,
        odds_evaluations: 0,
        odds_ms: 0.0,
        chances: vec![],
        scratch: vec![],
    };
    let (ascents, legal) = aside(simulator, |simulator| {
        let ascents: Vec<Value> = starts
            .iter()
            .map(|&start| {
                let (scorings_before, started) = (scorer.scorings, now_ms());
                let ended = ascend(simulator, context, &mut scorer, start, buf);
                let name = |lineage: Lineage| json!([data.umas[lineage.trainee].name, [lineage.first_parent, lineage.second_parent]]);
                json!({"start": name(start), "end": ended.as_ref().map(|ascent| name(ascent.lineage)),
                    "score": ended.as_ref().map(|ascent| ascent.score), "rounds": ended.as_ref().map(|ascent| ascent.rounds),
                    "scorings": scorer.scorings - scorings_before, "ms": now_ms() - started})
            })
            .collect();
        // the facts: legal pairs per allowed uma with a cached deck
        let mut counts: Vec<usize> = vec![];
        for &trainee in &context.candidates.allowed_trainees {
            if !simulator.evaluator.cache.decks[trainee].is_empty() {
                counts.push(legal_pairs(simulator, context, trainee).len());
            }
        }
        counts.sort_unstable();
        (ascents, counts)
    });
    let per = |total: f64, count: usize| if count == 0 { Value::Null } else { json!(total / count as f64) };
    json!({
        "legal_pairs": {"umas": legal.len(), "total": legal.iter().sum::<usize>(), "min": legal.first(),
            "median": legal.get(legal.len() / 2), "max": legal.last()},
        "ascents": ascents,
        "scorings": scorer.scorings,
        "ms_per_scoring": per(scorer.score_ms, scorer.scorings),
        "ms_per_odds": per(scorer.odds_ms, scorer.odds_evaluations),
        "ascent_ms": now_ms() - started_ms,
    })
}

/// The decomposition measurement (`PlanJob::pair_decompose`), or null without it: per requested (chara, pair), her best
/// cached deck there scored with its saving split by outcome class (`Want::Classes`, the cached decks only: `aside`):
/// per class its chance, its contribution to the saving (chance x mean saving), and per lifted candidate her odds now,
/// the mean odds she is lifted to there and whether her step still builds at them (so the route's C is the trio's,
/// `WhileBuilding`); the routes the scoring's memory budget dropped
pub(crate) fn pair_decompose(
    simulator: &mut Sim,
    context: &PlanContext,
    scores: &PlanScores,
    buf: &mut Vec<f64>,
) -> Value {
    let Some(requests) = &context.input.settings.pair_decompose else { return Value::Null };
    if !scores.fit.current_value.is_finite() {
        return Value::Null;
    }
    let data = context.data();
    let candidates = context.candidates;
    let base = scores.fit.score_base(candidates);
    let (mut chances, mut scratch) = (vec![], vec![]);
    let rows: Vec<Value> = aside(simulator, |simulator| {
        requests
            .iter()
            .map(|&(chara, first_parent, second_parent)| {
                let Some(trainee) = simulator.env.setup.uma_index(chara) else { return json!({"error": "no such uma"}) };
                let lineage = Lineage { trainee, first_parent, second_parent };
                if !legal_pair(data, context.input.pool, lineage) {
                    return json!({"error": "not a legal pair"});
                }
                let odds = deck_odds(simulator, context.input.pool, lineage, &mut chances, &mut scratch);
                let Some(deck) = (0..odds.len()).max_by(|&first, &second| odds[first].total_cmp(&odds[second])) else {
                    return json!({"error": "no cached deck"});
                };
                let career = Found {
                    trainee,
                    first_parent,
                    second_parent,
                    value: odds[deck],
                    ceiling: odds[deck],
                    deck: simulator.evaluator.cache.decks[trainee][deck].deck.clone(),
                };
                let dropped_before = crate::career::clock::PROF
                    .with(|prof| prof.borrow().get("sc_routes_dropped").copied().unwrap_or(0.0));
                let scored =
                    score_career(simulator, &context.scoring(&base), found_career(trainee, &career), Want::Classes, buf);
                let dropped = crate::career::clock::PROF
                    .with(|prof| prof.borrow().get("sc_routes_dropped").copied().unwrap_or(0.0))
                    - dropped_before;
                // (the plain scoring beside it: the class split must sum to the same saving)
                let plain =
                    score_career(simulator, &context.scoring(&base), found_career(trainee, &career), Want::Score, buf);
                let mut classes = scored.classes;
                classes.sort_by(|first, second| second.saving_sum.total_cmp(&first.saving_sum));
                let classes: Vec<Value> = classes
                    .iter()
                    .map(|class| {
                        let lifted: Vec<Value> = class
                            .lifted_odds_sums
                            .iter()
                            .map(|&(candidate, odds_sum)| {
                                let lifted_to = odds_sum / class.chance;
                                json!({"candidate": data.umas[candidates.trainees[candidate]].name,
                                    "odds_now": candidates.current_odds[candidate], "lifted_to": lifted_to,
                                    "builds_there": base.step_plan(candidate, lifted_to).1})
                            })
                            .collect();
                        json!({"full": class.full, "chance": class.chance, "contribution": class.saving_sum,
                            "mean_saving": class.saving_sum / class.chance, "lifted": lifted})
                    })
                    .collect();
                json!({"trainee": data.umas[trainee].name, "parents": [first_parent, second_parent], "p_all": odds[deck],
                    "saving": scored.expected_saving, "score": scored.expected_saving - 1.0, "routes_dropped": dropped,
                    "score_plain": plain.expected_saving - 1.0,
                    "classes": classes})
            })
            .collect()
    });
    Value::Array(rows)
}
