//! The job's `career`: one career a report named (`plan::kept::career_id`), reported as it is, with no deck search.
//! The page's Use this sends it for a builder's other career (`PlanJob::builder_careers`), which a search on its
//! parents would replace with her best deck there.
use crate::career::inherit::{ExactOptions, Inheritance, inherit};
use crate::career::run::CareerRun;
use crate::career::search::Found;
use crate::career::{Env, Lineage};
use crate::deck::space::SelectedDeck;
use serde_json::{Value, json};

/// The career `id` names, parsed (`plan::kept::career_id`): her uma id, her parents (legacy indices, in order), her
/// deck's named entries by identity (`DeckSpace::entry_key`), its plain pals' card ids and its filler count.
struct CareerId<'id> {
    trainee_id: u32,
    parents: [usize; 2],
    entry_keys: Vec<&'id str>,
    plain_ids: Vec<u32>,
    filler_cards: usize,
}

/// The '.'-separated items of `text` (none when it is empty)
fn items(text: &str) -> Vec<&str> {
    if text.is_empty() { vec![] } else { text.split('.').collect() }
}

fn parse_id(id: &str) -> Option<CareerId<'_>> {
    let mut parts = id.split(':');
    let trainee_id = parts.next()?.parse().ok()?;
    let (first, second) = parts.next()?.split_once(',')?;
    let mut deck = parts.next()?.split('/');
    let entry_keys = items(deck.next()?);
    let plain_ids = items(deck.next()?).into_iter().map(|text| text.parse().ok()).collect::<Option<Vec<u32>>>()?;
    let filler_cards = deck.next()?.parse().ok()?;
    (parts.next().is_none() && deck.next().is_none()).then_some(CareerId {
        trainee_id,
        parents: [first.parse().ok()?, second.parse().ok()?],
        entry_keys,
        plain_ids,
        filler_cards,
    })
}

/// The job's answer for `career`: that career's report (its schedule included, as a search's first career's), its
/// hint chances worked out from its cards at her chain events (`DeckSpace::hint_chances_at`, the f64s a search may
/// have reached in another order, so its odds can differ from the report that named it in the last bits). It needs the
/// job's one trainee and both its fixed parents to be the id's, and the deck to be one of this job's for her: every
/// named entry in this job's deck space (by card, choice variant and early-end link), none of her own card, one per
/// character, every locked card held and at most the borrowed cards the job allows, the plain pals the first she may
/// take, the job's deck size. Its search fields say one career was scored, and none other weighed: `given`, and neither
/// `complete` nor `proven_top` (no search compared it with any other career or deck; the page says so, proofs.mjs
/// `searchCaveat`).
pub(super) fn given_career_output(env: &mut Env, run: &CareerRun, id: &str) -> Value {
    let error = |why: &str| json!({"error": format!("career: {why}")});
    let Some(career) = parse_id(id) else {
        return error("not a career id (uma:parent,parent:entries/plain pals/filler cards).");
    };
    let data = env.setup.data;
    let [first_parent, second_parent] = career.parents;
    let fixed = &run.job.fixed_parents;
    let (&[trainee], 2) = (run.trainees, fixed.len()) else {
        return error("takes one trainee and both parents fixed.");
    };
    if data.umas[trainee].id != career.trainee_id
        || !(fixed.contains(&first_parent) && fixed.contains(&second_parent) && first_parent != second_parent)
    {
        return error("its trainee and parents are not the job's.");
    }
    let pool = run.pick_pool.unwrap_or(run.pool);
    if !pool.pair_ok(data.umas[trainee].id, first_parent, second_parent) {
        return error("its parents are not a legal pair for her.");
    }
    let space = &env.setup.space;
    let trainee_ids = [data.umas[trainee].id];
    let not_a_deck = || error("its deck is not one of the job's decks for her.");
    let Some(mut entries) = career.entry_keys.iter().map(|key| space.entry_of_key(key)).collect::<Option<Vec<usize>>>()
    else {
        return not_a_deck();
    };
    entries.sort_unstable();
    let card = |entry: usize| space.cards[space.card_indices[entry]];
    let entries_ok = entries.windows(2).all(|pair| pair[0] < pair[1])
        && entries.iter().all(|&entry| !card(entry).blocks(trainee_ids[0], crate::checks::get()))
        && entries.iter().enumerate().all(|(i, &entry)| {
            entries[..i].iter().all(|&earlier| card(earlier).character_id != card(entry).character_id)
        })
        && space.locks_and_borrowing_hold(&entries);
    let plain_pals = career.plain_ids.len();
    let size = entries.len() + plain_pals + career.filler_cards;
    if !entries_ok || size != space.deck_job.deck_size || space.plain_ids(&trainee_ids, plain_pals) != career.plain_ids
    {
        return not_a_deck();
    }
    let mut deck = SelectedDeck {
        entries,
        plain_pals,
        filler_cards: career.filler_cards,
        white_hint_chances: vec![],
        gold_hint_chances: vec![],
    };
    let hints = space.hint_chances_at(&deck, space.deck_events(&deck, &trainee_ids));
    (deck.white_hint_chances, deck.gold_hint_chances) = (hints.white, hints.gold);
    let lineage = Lineage { trainee, first_parent, second_parent };
    let mut chances = vec![];
    let spark_scale = env.setup.lineage_into(pool, lineage, &mut chances);
    let Some(ceiling) = env.setup.ceiling(&mut env.state, trainee, &chances, spark_scale) else {
        return error("she has no legal deck.");
    };
    let Inheritance { chances: q, carriers } = inherit(&env.setup.slots(pool, lineage), env.setup.target_count);
    env.setup.set_trainee(&mut env.state, trainee, &q);
    let value = env
        .setup
        .exact(&env.state, trainee, &deck, &carriers, ExactOptions { all_but_one: false, landed_sets: false })
        .p_all;
    let found = Found { trainee, first_parent, second_parent, value, ceiling, deck };
    let report = env.setup.report(&mut env.state, pool, &found, true);
    let rho_top = env.setup.space.rho_top();
    json!({"results": [report], "candidates": 1, "solved": 1, "bounded": 0, "scored": 1, "skipped": 0, "next_bound": null,
           "complete": false, "proven_top": false, "cap": rho_top, "class_caps": env.setup.space.caps(),
           "work": {"nodes": 0, "exact": 0}, "given": true})
}
