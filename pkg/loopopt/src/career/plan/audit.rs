//! The lifted-odds audit (`PlanJob::audit_lifts`, a measurement): what the scorings read of a candidate's odds with a
//! career's veteran as a parent, against what an exact search finds.
use crate::career::plan::PlanContext;
use crate::career::plan::combine::PlanScores;
use crate::career::plan::cont::Want;
use crate::career::plan::fold::leaf_set;
use crate::career::plan::rows::{PlanPick, PlanRow, pick_of};
use crate::career::plan::score::kernel::Scored;
use crate::career::plan::score::{CareerScore, KeptScoring, score_career};
use crate::career::plan::search::{capped, legal_pair};
use crate::career::search::{Found, SearchRequest};
use crate::career::sim::Sim;
use crate::career::{Ancestor, Legacy, Lineage};
use serde_json::{Value, json};

/// Outcomes audited per row: her likeliest that land at least one target.
const AUDIT_OUTCOMES: usize = 5;

/// Builder rows audited beside the pick: the highest scored rows whose uma is no stage candidate.
const AUDIT_BUILDERS: usize = 3;

/// Per audited row (the pick, then the top AUDIT_BUILDERS builders by score) and each of its AUDIT_OUTCOMES likeliest
/// outcomes landing a target (her stars per uncertain target, 2 stars on a guaranteed target landed, her parents and
/// G1 win chances as her career's), per candidate her scoring can lift: her odds now (`pnow`), what the row's own
/// scoring read there (`table`: the partner slots' tables of the scoring the plan kept for the row, the best of them;
/// null where the plan kept none), the same of the row scored again on the plan's fit as it ends (`table_rescored`:
/// its preparation may have found more decks), the most her cached decks give with every legal partner
/// (`all_partners`, `Evaluator::tval`: no deck search, no refresh), and her best career with this veteran as a parent,
/// searched exactly (`exact`, under `search_work`; `complete` whether the search was). Every row is audited from one
/// frozen baseline, the plan's cache and proofs as its numbers were taken (put back before each row and after the
/// last: the scorings may add decks, the searches proofs): no row's audit reads another's, and no later number of the
/// job reads what it did (the deck searches' memo, exact, keeps its entries)
pub(crate) fn audit_lifts(
    simulator: &mut Sim,
    context: &PlanContext,
    rows: &[PlanRow],
    scores: &PlanScores,
    buf: &mut Vec<f64>,
) -> Value {
    let PlanContext { input, candidates } = *context;
    let data = context.data();
    let is_runnable = |career: &Found| input.is_runnable(career);
    let PlanPick { row: pick, .. } = pick_of(rows, is_runnable, &[]);
    let mut audited = vec![pick];
    let mut builders: Vec<usize> = (0..rows.len())
        .filter(|&i| i != pick && rows[i].scoring.is_some() && !candidates.trainees.contains(&rows[i].trainee))
        .collect();
    builders.sort_by(|&first, &second| rows[second].total().total_cmp(&rows[first].total()));
    audited.extend(builders.into_iter().take(AUDIT_BUILDERS));
    let env_budget = simulator.env.state.budget;
    let cache_before = simulator.evaluator.cache.clone();
    let (seen_before, ruled_before) = (simulator.env.state.seen.clone(), simulator.env.state.ruled.clone());
    let score_base = scores.fit.score_base(candidates);
    let mut report = vec![];
    for row_index in audited {
        simulator.evaluator.cache.clone_from(&cache_before);
        simulator.env.state.seen.clone_from(&seen_before);
        simulator.env.state.ruled.clone_from(&ruled_before);
        let row = &rows[row_index];
        let lineage = row.lineage();
        let original = row.scoring.as_ref().and_then(|scoring| scoring.kept.as_ref()).map(|kept| &kept.scored);
        let CareerScore { kept, .. } =
            score_career(simulator, &context.scoring(&score_base), row.scored_career(), Want::Keep, buf);
        let Some(KeptScoring { scored }) = kept else { continue };
        let current = simulator.evaluator.make_cur(
            &simulator.env.setup,
            &mut simulator.env.state,
            input.pool,
            row.scored_career(),
            false,
        );
        let setup = &simulator.env.setup;
        let (uncertain_targets, fixed_targets) = (setup.uncertain_targets.clone(), setup.fixed_targets.clone());
        let star_mix = setup.skill_job.star_dist;
        let target_groups: Vec<u32> = setup.skill_job.targets.iter().map(|target| target.group).collect();
        // (leaf, fixed targets landed, chance), the likeliest landing a target, ties in cell order
        let mut outcomes: Vec<(usize, usize, f64)> = vec![];
        for leaf in 0..1usize << (2 * uncertain_targets.len()) {
            let outcome = leaf_set(leaf, &uncertain_targets, star_mix);
            for fixed in 0..1usize << fixed_targets.len() {
                let landed = (0..fixed_targets.len())
                    .filter(|bit| fixed >> bit & 1 == 1)
                    .fold(outcome.landed_mask, |mask, bit| mask | 1 << fixed_targets[bit]);
                let chance = current.outcome_sets[landed] * outcome.star_probability;
                if landed != 0 && chance > 0.0 {
                    outcomes.push((leaf, fixed, chance));
                }
            }
        }
        outcomes.sort_by(|first, second| second.2.total_cmp(&first.2));
        outcomes.truncate(AUDIT_OUTCOMES);
        let parents = || {
            vec![
                Ancestor::of_legacy(&input.legacies[lineage.first_parent]),
                Ancestor::of_legacy(&input.legacies[lineage.second_parent]),
            ]
        };
        let mut outcome_report = vec![];
        for &(leaf, fixed, chance) in &outcomes {
            let landed = (0..fixed_targets.len())
                .filter(|bit| fixed >> bit & 1 == 1)
                .fold(leaf_set(leaf, &uncertain_targets, star_mix).landed_mask, |mask, bit| {
                    mask | 1 << fixed_targets[bit]
                });
            let mut stars = vec![0u8; target_groups.len()];
            for (digit, &target) in uncertain_targets.iter().enumerate() {
                stars[target] = (leaf >> (2 * digit) & 3) as u8;
            }
            for (bit, &target) in fixed_targets.iter().enumerate() {
                if fixed >> bit & 1 == 1 {
                    stars[target] = 2;
                }
            }
            let veteran = Legacy {
                character_id: data.umas[lineage.trainee].id,
                borrowed: false,
                sparks: (0..target_groups.len())
                    .filter(|&target| stars[target] > 0)
                    .map(|target| (target_groups[target], stars[target]))
                    .collect(),
                wins: vec![],
                pinks: vec![],
                parents: parents(),
                win_chances: current.record.win_chances.to_vec(),
                mix: None,
                pink_chances: vec![],
            };
            let mut lifts = vec![];
            for candidate in 0..candidates.trainees.len() {
                let candidate_trainee = candidates.trainees[candidate];
                if data.umas[candidate_trainee].id == data.umas[lineage.trainee].id
                    || !scores.fit.step_costs[candidate].is_finite()
                {
                    continue;
                }
                let at = ScoredOutcome { original, rescored: &scored, leaf, landed_mask: landed };
                lifts.push(audit_candidate(simulator, context, candidate, at, &veteran, env_budget));
            }
            outcome_report.push(json!({"stars": stars, "chance": chance, "lifts": lifts}));
        }
        report.push(json!({"trainee": data.umas[lineage.trainee].name, "origin": row.origin.name(), "parents": [lineage.first_parent, lineage.second_parent], "score": row.net_saving(), "outcomes": outcome_report}));
    }
    simulator.evaluator.cache = cache_before;
    (simulator.env.state.seen, simulator.env.state.ruled) = (seen_before, ruled_before);
    Value::Array(report)
}

/// An audited row's scorings at one of her outcomes: the plan's own (if kept) and the audit's again.
#[derive(Clone, Copy)]
struct ScoredOutcome<'scored> {
    original: Option<&'scored Scored>,
    rescored: &'scored Scored,
    leaf: usize,
    landed_mask: usize,
}

/// One candidate's odds with the veteran `veteran` (one outcome of the row's career) as a parent (`audit_lifts`)
fn audit_candidate(
    simulator: &mut Sim,
    context: &PlanContext,
    candidate: usize,
    at: ScoredOutcome,
    veteran: &Legacy,
    env_budget: usize,
) -> Value {
    let PlanContext { input, candidates } = *context;
    let data = context.data();
    let trainee = candidates.trainees[candidate];
    let pnow = candidates.current_odds[candidate];
    // what a scoring reads: the best of her partner slots there (each floored at her odds now, as the fold's slots)
    let read = |scored: &Scored| {
        (0..scored.slot_candidates.len())
            .filter(|&slot| scored.slot_candidates[slot] == candidate)
            .map(|slot| scored.setup_odds_at(slot, at.leaf, at.landed_mask))
            .fold(pnow, f64::max)
    };
    let (table, table_rescored) = (at.original.map(read), read(at.rescored));
    let mut pool = simulator.evaluator.take_pool(input.pool);
    simulator.env.setup.pool_push(&mut pool, veteran);
    let her = pool.veterans.len() - 1;
    let (mut chances, mut scratch) = (vec![], vec![]);
    let mut all_partners = 0.0f64;
    for partner in 0..her {
        let lineage = Lineage { trainee, first_parent: partner, second_parent: her };
        if legal_pair(data, &pool, lineage) {
            let spark_scale = simulator.env.setup.lineage_into(&pool, lineage, &mut chances);
            let (value, _) =
                simulator.evaluator.tval(&simulator.env.setup, trainee, &chances, spark_scale, &mut scratch);
            all_partners = all_partners.max(value);
        }
    }
    let searched = capped(simulator, input.settings.search_work, env_budget, |simulator| {
        simulator.env.solve(SearchRequest {
            pool: &pool,
            fixed_parents: &[her],
            trainees: &[trainee],
            per_trainee: false,
            top: 1,
        })
    });
    pool.pop_veteran();
    simulator.evaluator.put_pool(input.pool, pool);
    let exact = searched.found.first().map_or(0.0, |found| found.value);
    json!({"candidate": data.umas[trainee].name, "pnow": pnow, "table": table, "table_rescored": table_rescored, "all_partners": all_partners.max(pnow), "exact": exact.max(pnow), "complete": searched.complete})
}
