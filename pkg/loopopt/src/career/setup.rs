//! A career job read three ways (`CareerJobs`) and its `Env` built (`career_setup`): the model, the deck space, each
//! owned uma's deck-odds ceilings, the deck classes and what earlier jobs of the setup proved (`restore_seen`).
use crate::career::decks::{CfStats, DECK_KEEP_CAP, JOB_NO};
use crate::career::state::{KeptState, setup_fp};
use crate::career::{
    BorrowedParent, CAREER_BUDGET, CareerJob, CareerSetup, DeckSearchAudit, DeckSearchMemo, Env, JobInputs, SearchState,
};
use crate::data::Data;
use crate::deck::sources::deck_entries;
use crate::deck::space::{DeckSpace, deck_space};
use crate::deck::{Card, DeckJob};
use crate::skills::{Model, SkillJob};
use crate::uma::{UmaIndex, UmaTable};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
/// The three readings of one career job's JSON.
pub(super) struct CareerJobs {
    /// Targets, owned umas and the rank's star mix.
    pub(super) skills: SkillJob,
    /// The deck space: cards, deck size, locks, chain events.
    deck: DeckJob,
    /// Legacies, trainees, places and the planner's settings.
    pub(super) career: CareerJob,
}

impl CareerJobs {
    /// The job read three ways, in this order; the first error is the answer.
    pub(super) fn parse(job: &Value) -> Result<Self, Value> {
        let skills = SkillJob::deserialize(job).map_err(|e| json!({"error": e.to_string()}))?;
        let deck = DeckJob::deserialize(job).map_err(|e| json!({"error": e.to_string()}))?;
        let career = CareerJob::deserialize(job).map_err(|e| json!({"error": e.to_string()}))?;
        Ok(Self { skills, deck, career })
    }
}

/// A job's solver environment and what it was built for.
pub(super) struct CareerBuild<'data> {
    pub(super) env: Env<'data>,
    /// The job's trainees (uma indices), owned umas in roster order.
    pub(super) trainees: Vec<UmaIndex>,
    /// `setup_fp` of the job: what earlier jobs of this setup proved is kept under it (`KeptState::seen`).
    pub(super) setup_fingerprint: u64,
}

/// The job's Env: the model, the deck space, each owned uma's deck-odds ceiling, the deck classes, the kept deck memo
/// of this setup and what earlier jobs of the setup proved (`KeptState::seen`). Errors (no trainee, fewer than two legacies, a deck space the search refuses) come before
/// anything kept is touched.
pub(super) fn career_setup<'data>(
    input: &JobInputs<'data>,
    jobs: &'data CareerJobs,
    model: &'data Model<'data>,
    worker: &mut KeptState,
) -> Result<CareerBuild<'data>, Value> {
    let (data, skills, cards, job_json) = (input.data, input.skills, input.cards, input.job);
    let (skill_job, deck_job, career_job) = (&jobs.skills, &jobs.deck, &jobs.career);
    let folded_events = UmaTable(
        model
            .event_chances
            .iter()
            .map(|outfits| outfits.iter().map(|chances| vec![0.0; chances.len()]).collect())
            .collect(),
    );
    let uncertain_groups: Vec<u32> = model.uncertain.iter().map(|&i| skill_job.targets[i].group).collect();
    let uncertain_count = uncertain_groups.len();
    let target_count = skill_job.targets.len();
    let uncertain_targets = model.uncertain.clone();
    let fixed_targets: Vec<usize> = (0..target_count).filter(|i| !uncertain_targets.contains(i)).collect();
    let uma_count = data.umas.len();
    let card_of: HashMap<u32, &Card> = cards.cards.iter().map(|card| (card.id, card)).collect();
    let lock_blocked = |uma: UmaIndex| {
        deck_job
            .lock_cards
            .iter()
            .any(|id| card_of.get(id).is_some_and(|card| card.blocks(data.umas[uma].id, crate::checks::get())))
    };
    let id_ok =
        |id: u32| (skill_job.owned.is_empty() || skill_job.owned.contains(&id)) && !skill_job.exclude.contains(&id);
    // every uma that can train (the plan's next career takes any of them); the job's trainees are among these
    let owned: Vec<UmaIndex> =
        data.umas.indices().filter(|&i| id_ok(data.umas[i].id) && !model.no_outfit[i] && !lock_blocked(i)).collect();
    let trainees: Vec<UmaIndex> = owned
        .iter()
        .copied()
        .filter(|&i| career_job.trainees.is_empty() || career_job.trainees.contains(&data.umas[i].id))
        .collect();
    if trainees.is_empty() {
        return Err(json!({"error": "No trainee to search: pick at least one owned uma."}));
    }
    if career_job.legacies.len() < 2 {
        return Err(json!({"error": "Add at least two veterans to choose parents from."}));
    }
    let pool_ids: Vec<u32> = trainees.iter().map(|&i| data.umas[i].id).collect();
    let entries = deck_entries(cards, deck_job, &uncertain_groups, &pool_ids)?;
    let space = deck_space(cards, deck_job, skills, &uncertain_groups, &entries);
    let deck_odds_ceilings = deck_odds_ceilings(data, &space, &owned, uncertain_count, deck_job.deck_size);
    let outfit_coverage: UmaTable<Vec<Vec<u8>>> = model
        .outfit_options
        .iter()
        .map(|options| options.iter().map(|option| option.coverage.clone()).collect())
        .collect();
    let budget = crate::checks::get().exact_budget.unwrap_or(CAREER_BUDGET);
    // the kept `best_deck` memo of this setup (`KeptState::decks`), and this job's number
    let setup_fingerprint = setup_fp(job_json);
    let memo_fingerprint = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        setup_fingerprint.hash(&mut hasher);
        pool_ids.hash(&mut hasher);
        hasher.finish()
    };
    let job_number = JOB_NO.with(|counter| {
        counter.set(counter.get() + 1);
        counter.get()
    });
    let kept = worker
        .decks
        .iter()
        .position(|entry| entry.0 == memo_fingerprint)
        .map(|i| worker.decks.remove(i).1)
        .unwrap_or_default();
    let memo = DeckSearchMemo {
        decks: if kept.decks.len() < DECK_KEEP_CAP { kept.decks } else { HashMap::default() },
        corners: if kept.corners.len() < DECK_KEEP_CAP { kept.corners } else { HashMap::default() },
        fingerprint: memo_fingerprint,
        job_number,
        state_bounds: crate::deck::state_bound::StateBoundMemo::default(),
    };
    let deck_class = deck_classes(data, model, &space, &outfit_coverage, &deck_odds_ceilings);
    let setup = CareerSetup {
        data,
        skill_job,
        model,
        uncertain_targets,
        fixed_targets,
        target_count,
        uncertain_count,
        uncertain_groups,
        space,
        deck_odds_ceilings,
        outfit_coverage,
        owned,
        deck_class,
        max_borrowed: u8::from(career_job.borrowed_parent != BorrowedParent::Never),
    };
    let mut state = SearchState {
        folded_events,
        budget,
        work: 0,
        exact_scorings: 0,
        seen: UmaTable(vec![vec![]; uma_count]),
        use_seen: true,
        ruled: UmaTable(vec![vec![]; uma_count]),
        last_product_bound: None,
        floor: f64::NEG_INFINITY,
        hard_work_cap: 0,
    };
    restore_seen(worker, &setup, &mut state, setup_fingerprint);
    let audit = DeckSearchAudit {
        max_ratio: [0.0; 4],
        cap_breach: 0,
        cap_free_stats: CfStats::default(),
        max_deck: Default::default(),
        certified_cap: f64::INFINITY,
        overflows: 0,
    };
    Ok(CareerBuild { env: Env { setup, state, audit, memo }, trainees, setup_fingerprint })
}

/// Per uma (None when she does not train), the best possible deck odds per uncertain target, a ceiling for every
/// legal deck: (dw, dg) with dg the chance of a gold hint and (1 - dw)(1 - dg) the chance of no hint, from her
/// `deck_size` most likely hinting legal entries.
fn deck_odds_ceilings(
    data: &Data,
    space: &DeckSpace,
    owned: &[UmaIndex],
    uncertain_count: usize,
    deck_size: usize,
) -> UmaTable<Option<(Vec<f64>, Vec<f64>)>> {
    // per relevant entry: (miss of a gold hint, miss of any hint) per uncertain target
    let strong: Vec<(Vec<f64>, Vec<f64>)> = (0..space.card_indices.len())
        .map(|j| {
            let (mut white_misses, mut gold_misses) = (vec![1.0f64; uncertain_count], vec![1.0f64; uncertain_count]);
            space.sources[j].src_miss(&vec![1.0; space.kind[j].stop as usize], &mut white_misses, &mut gold_misses);
            let hint_misses: Vec<f64> = (0..uncertain_count).map(|k| gold_misses[k] * white_misses[k]).collect();
            (gold_misses, hint_misses)
        })
        .collect();
    data.umas
        .indices()
        .map(|uma| {
            if !owned.contains(&uma) {
                return None;
            }
            let uma_id = data.umas[uma].id;
            let legal: Vec<usize> = (0..space.card_indices.len())
                .filter(|&j| !space.cards[space.card_indices[j]].blocks(uma_id, crate::checks::get()))
                .collect();
            let lowest = |miss_of: &dyn Fn(usize) -> f64| {
                let mut misses: Vec<f64> = legal.iter().map(|&j| miss_of(j)).collect();
                misses.sort_by(|a, b| a.partial_cmp(b).unwrap());
                misses.iter().take(deck_size).product::<f64>()
            };
            let (mut white_hint_chances, mut gold_hint_chances) =
                (vec![0.0f64; uncertain_count], vec![0.0f64; uncertain_count]);
            for k in 0..uncertain_count {
                let (g, h) = (1.0 - lowest(&|j| strong[j].0[k]), 1.0 - lowest(&|j| strong[j].1[k]));
                // (g, h) = (P(gold hint), P(any hint)); the product formula takes dw with (1 - dw)(1 - dg) = 1 - h
                gold_hint_chances[k] = g;
                white_hint_chances[k] = if g < 1.0 { (1.0 - (1.0 - h) / (1.0 - g)).max(0.0) } else { 1.0 };
            }
            Some((white_hint_chances, gold_hint_chances))
        })
        .collect()
}

/// Deck classes (`CareerSetup::deck_class`): the key holds everything of a trainee the deck search reads (outfit
/// coverage, own-skill chances to the bit, `deck_odds_ceilings`, the relevant cards she may not take, how many plain
/// pals she may); each trainee's class is the first trainee with her key.
fn deck_classes(
    data: &Data,
    model: &Model,
    space: &DeckSpace,
    outfit_coverage: &UmaTable<Vec<Vec<u8>>>,
    deck_odds_ceilings: &UmaTable<Option<(Vec<f64>, Vec<f64>)>>,
) -> UmaTable<UmaIndex> {
    let mut first: crate::FxMap<Vec<u64>, UmaIndex> = crate::FxMap::default();
    let mut deck_class: UmaTable<UmaIndex> = data.umas.indices().collect();
    for uma in data.umas.indices() {
        let uma_id = data.umas[uma].id;
        let mut key: Vec<u64> = vec![];
        for outfit in &outfit_coverage[uma] {
            key.push(outfit.len() as u64);
            key.extend(outfit.iter().map(|&value| u64::from(value)));
        }
        key.push(u64::MAX);
        for outfit in &model.event_chances[uma] {
            key.push(outfit.len() as u64);
            key.extend(outfit.iter().map(|value| value.to_bits()));
        }
        key.push(u64::MAX);
        match &deck_odds_ceilings[uma] {
            Some((white, gold)) => {
                key.push(1);
                key.extend(white.iter().chain(gold).map(|value| value.to_bits()));
            }
            None => key.push(0),
        }
        key.push(u64::MAX);
        key.extend(
            (0..space.card_indices.len())
                .filter(|&j| space.cards[space.card_indices[j]].blocks(uma_id, crate::checks::get()))
                .map(|j| j as u64),
        );
        key.push(u64::MAX);
        key.push(space.legal_plain(&[uma_id]).len() as u64);
        deck_class[uma] = *first.entry(key).or_insert(uma);
    }
    deck_class
}

/// What earlier jobs on this setup proved (the page's worker keeps it between jobs, `KeptState::seen`), when it fits this setup.
fn restore_seen(kept: &KeptState, setup: &CareerSetup, state: &mut SearchState, setup_fingerprint: u64) {
    if let Some((_, seen)) = kept.seen.iter().find(|entry| entry.0 == setup_fingerprint)
        && setup.seen_fits(seen)
    {
        // The worker retains its proofs while this job extends its own copy.
        state.seen = seen.clone();
    }
}
