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
use crate::career::plan::robust::{aside, aside_complete};
use crate::career::plan::rows::{PlanPick, PlanRow, pick_of};
use crate::career::plan::score::score_career;
use crate::career::plan::search::legal_pair;
use crate::career::plan::sweep::deck_odds;
use crate::career::plan::{PlanContext, found_career};
use crate::career::search::{Found, RankedPair};
use crate::career::sim::Sim;
use crate::refusal::Refusal;
use crate::uma::UmaIndex;
use serde::Serialize;
use ts_rs::TS;

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
) -> Option<PairAscent> {
    let input = context.input;
    if !input.settings.pair_ascent || !scores.any_scored || !scores.fit.current_value.is_finite() {
        return None;
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
    let ((ascents, legal), scoring_complete) = aside_complete(simulator, |simulator| {
        let ascents: Vec<AscentRun> = starts
            .iter()
            .map(|&start| {
                let (scorings_before, started) = (scorer.scorings, now_ms());
                let ended = ascend(simulator, context, &mut scorer, start, buf);
                let name = |lineage: Lineage| {
                    (data.umas[lineage.trainee].name.clone(), [lineage.first_parent, lineage.second_parent])
                };
                AscentRun {
                    start: name(start),
                    end: ended.as_ref().map(|ascent| name(ascent.lineage)),
                    score: ended.as_ref().map(|ascent| ascent.score),
                    rounds: ended.as_ref().map(|ascent| ascent.rounds),
                    scorings: scorer.scorings - scorings_before,
                    ms: now_ms() - started,
                }
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
    let per = |total: f64, count: usize| (count != 0).then(|| total / count as f64);
    Some(PairAscent {
        legal_pairs: LegalPairs {
            umas: legal.len(),
            total: legal.iter().sum::<usize>(),
            min: legal.first().copied(),
            median: legal.get(legal.len() / 2).copied(),
            max: legal.last().copied(),
        },
        ascents,
        scorings: scorer.scorings,
        ms_per_scoring: per(scorer.score_ms, scorer.scorings),
        ms_per_odds: per(scorer.odds_ms, scorer.odds_evaluations),
        ascent_ms: now_ms() - started_ms,
        scoring_complete,
    })
}

/// The pair ascent (`next.pair_ascent`): coordinate ascent over (trainee, parent pair) by score from each start.
#[derive(Serialize, TS)]
pub(crate) struct PairAscent {
    pub(crate) legal_pairs: LegalPairs,
    pub(crate) ascents: Vec<AscentRun>,
    pub(crate) scorings: usize,
    /// the cost per scoring and per P(all) evaluation over every ascent (null: none)
    pub(crate) ms_per_scoring: Option<f64>,
    pub(crate) ms_per_odds: Option<f64>,
    pub(crate) ascent_ms: f64,
    pub(crate) scoring_complete: bool,
}

/// The legal pairs per allowed uma with a cached deck: how many umas, the pairs in all, and the least, median and most
/// of an uma's (null: no uma).
#[derive(Serialize, TS)]
pub(crate) struct LegalPairs {
    pub(crate) umas: usize,
    pub(crate) total: usize,
    pub(crate) min: Option<usize>,
    pub(crate) median: Option<usize>,
    pub(crate) max: Option<usize>,
}

/// One ascent: its start and end (trainee by name, parents), the score and rounds at its end (null: it found no
/// career), the scorings it added and its time.
#[derive(Serialize, TS)]
pub(crate) struct AscentRun {
    pub(crate) start: (String, [usize; 2]),
    pub(crate) end: Option<(String, [usize; 2])>,
    pub(crate) score: Option<f64>,
    pub(crate) rounds: Option<usize>,
    pub(crate) scorings: usize,
    pub(crate) ms: f64,
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
) -> Option<Vec<PairDecomposition>> {
    let Some(requests) = &context.input.settings.pair_decompose else { return None };
    if !scores.fit.current_value.is_finite() {
        return None;
    }
    let data = context.data();
    let candidates = context.candidates;
    let base = scores.fit.score_base(candidates);
    let (mut chances, mut scratch) = (vec![], vec![]);
    let rows: Vec<PairDecomposition> = aside(simulator, |simulator| {
        requests
            .iter()
            .map(|&(chara, first_parent, second_parent)| {
                let refused = |why: &str| PairDecomposition::Refusal(Refusal::new(why));
                let Some(trainee) = simulator.env.setup.uma_index(chara) else { return refused("no such uma") };
                let lineage = Lineage { trainee, first_parent, second_parent };
                if !legal_pair(data, context.input.pool, lineage) {
                    return refused("not a legal pair");
                }
                let odds = deck_odds(simulator, context.input.pool, lineage, &mut chances, &mut scratch);
                let Some(deck) = (0..odds.len()).max_by(|&first, &second| odds[first].total_cmp(&odds[second])) else {
                    return refused("no cached deck");
                };
                let career = Found {
                    trainee,
                    first_parent,
                    second_parent,
                    value: odds[deck],
                    ceiling: odds[deck],
                    deck: simulator.evaluator.cache.decks[trainee][deck].deck.clone(),
                };
                let incomplete_before = crate::career::plan::robust::incompleteness_so_far();
                let dropped_before = crate::career::clock::PROF
                    .with(|prof| prof.borrow().get("sc_routes_dropped").copied().unwrap_or(0.0));
                let scored = score_career(
                    simulator,
                    &context.scoring(&base),
                    found_career(trainee, &career),
                    Want::Classes,
                    buf,
                );
                let dropped = crate::career::clock::PROF
                    .with(|prof| prof.borrow().get("sc_routes_dropped").copied().unwrap_or(0.0))
                    - dropped_before;
                // (the plain scoring beside it: the class split must sum to the same saving)
                let plain =
                    score_career(simulator, &context.scoring(&base), found_career(trainee, &career), Want::Score, buf);
                let mut classes = scored.classes;
                classes.sort_by(|first, second| second.saving_sum.total_cmp(&first.saving_sum));
                let classes = classes
                    .iter()
                    .map(|class| {
                        let lifted = class
                            .lifted_odds_sums
                            .iter()
                            .map(|&(candidate, odds_sum)| {
                                let lifted_to = odds_sum / class.chance;
                                LiftedCandidate {
                                    candidate: data.umas[candidates.trainees[candidate]].name.clone(),
                                    odds_now: candidates.current_odds[candidate],
                                    lifted_to,
                                    builds_there: base.step_plan(candidate, lifted_to).1,
                                }
                            })
                            .collect();
                        OutcomeClassReport {
                            full: class.full,
                            chance: class.chance,
                            contribution: class.saving_sum,
                            mean_saving: class.saving_sum / class.chance,
                            lifted,
                        }
                    })
                    .collect();
                PairDecomposition::Done(Decomposed {
                    trainee: data.umas[trainee].name.clone(),
                    parents: [first_parent, second_parent],
                    p_all: odds[deck],
                    saving: scored.expected_saving,
                    score: scored.expected_saving - 1.0,
                    routes_dropped: dropped,
                    score_plain: plain.expected_saving - 1.0,
                    scoring_complete: crate::career::plan::robust::incompleteness_so_far() == incomplete_before,
                    classes,
                })
            })
            .collect()
    });
    Some(rows)
}

/// One requested (chara, pair) of the decomposition (`next.pair_decompose`), or why it has none.
#[derive(Serialize, TS)]
#[serde(untagged)]
pub(crate) enum PairDecomposition {
    Done(Decomposed),
    Refusal(Refusal),
}

/// Her best cached deck on the pair, scored with its saving split by outcome class, beside the plain scoring.
#[derive(Serialize, TS)]
pub(crate) struct Decomposed {
    pub(crate) trainee: String,
    pub(crate) parents: [usize; 2],
    pub(crate) p_all: f64,
    pub(crate) saving: f64,
    pub(crate) score: f64,
    /// routes the scoring's memory budget dropped
    pub(crate) routes_dropped: f64,
    pub(crate) score_plain: f64,
    pub(crate) scoring_complete: bool,
    /// the outcome classes, by contribution, most first
    pub(crate) classes: Vec<OutcomeClassReport>,
}

/// An outcome class: whether her veteran is full there, its chance, its contribution to the saving (chance x mean
/// saving) and mean saving, and per lifted candidate her odds now, the mean odds she is lifted to and whether her step
/// still builds at them.
#[derive(Serialize, TS)]
pub(crate) struct OutcomeClassReport {
    pub(crate) full: bool,
    pub(crate) chance: f64,
    pub(crate) contribution: f64,
    pub(crate) mean_saving: f64,
    pub(crate) lifted: Vec<LiftedCandidate>,
}

#[derive(Serialize, TS)]
pub(crate) struct LiftedCandidate {
    pub(crate) candidate: String,
    pub(crate) odds_now: f64,
    pub(crate) lifted_to: f64,
    pub(crate) builds_there: bool,
}
