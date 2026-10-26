use crate::data::load;
use crate::deck::chains::{
    CHAIN_A, CHAIN_B, CHAIN_N, ChainEvents, ChainKind, PickRule, chain_marginals, chain_marginals_dense,
    chain_marginals_ref, chain_states, event_pmf, expit, key_level, stop_risk_code,
};
use crate::deck::hints::{hint_base, hint_miss, hint_spec, miss_draws};
use crate::deck::joint::reference::joint_trainee_ref;
use crate::deck::joint::upper::upper_na;
use crate::deck::joint::{JointExt, JointQuery, LandPre, joint, joint_fast, land_factor};
use crate::deck::sources::{
    ChainOutcome, EventOutcome, Group, SourceEntry, Sources, TargetReward, TargetSources, TrainingHint, deck_entries,
    group_gh, group_nuod, group_nuod_lvl,
};
use crate::deck::space::{MAX_KINDS, kinds_all, top_asc, top_desc};
use crate::deck::state_bound::state_value_cf;
use crate::deck::*;
use crate::skills;
use crate::skills::{GEN_GOLD, Model, SkillJob, SkillsRaw};
use serde_json::{Value, json};
/// A three-link chain card played to its end.
const THREE_LINKS: ChainKind = ChainKind { links: 3, stop: 3, stop_risk: 0 };
/// A fixed chain-event law for the chain-process tests: Binomial(21, 0.35) picking by links left, the model before
/// the pal rate and the card rule.
const EVENTS_35: ChainEvents = ChainEvents { count: CHAIN_N, probability: 0.35, rule: PickRule::Slots };
/// The same under the card rule: the process tests run under both pick rules.
const EVENTS_35_CARD: ChainEvents = ChainEvents { rule: PickRule::Card, ..EVENTS_35 };

fn fixture_skills() -> SkillsRaw {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/web_skills.json"))
        .expect("read global skill fixture");
    serde_json::from_str(&text).expect("parse global skill fixture")
}

fn fixture_cards() -> CardsRaw {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/web_cards.json"))
        .expect("read global card fixture");
    serde_json::from_str(&text).expect("parse global card fixture")
}

/// The deck search's allocation-free lineage chances and per-trainee odds equal the reference forms.
#[test]
fn fast_metric_matches_reference() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let skills = fixture_skills();
    let skill_job: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":21010},{"group":20165}]})).unwrap();
    let m = Model::new(&d, &skills, &skill_job);
    let nu = m.uncertain.len();
    let fs: [&[usize]; 5] = [&[0, 5, 9, 20], &[3, 7, 11, 40], &[1, 2, 3, 4], &[0, 5, 9], &[3, 11, 40]];
    for (fi, &f) in fs.iter().enumerate() {
        let f: Vec<crate::uma::UmaIndex> = f.iter().copied().map(crate::uma::UmaIndex).collect();
        let qa = m.lineage_q_arr(&f, &|_, _| 3.0);
        let qv = m.lineage_q(&f, &|_, _| 3.0);
        assert_eq!(qv.len(), if f.len() == 4 { 12 } else { 3 });
        assert!(qa.iter().zip(&qv).all(|(a, b)| a.to_bits() == b.to_bits()));
        let white_hint_chances: Vec<f64> = (0..nu).map(|k| ((k + fi) as f64 * 0.17) % 1.0).collect();
        let gold_hint_chances: Vec<f64> = (0..nu).map(|k| ((k * 3 + fi) as f64 * 0.11) % 1.0).collect();
        // the search's per-trainee term (best outfit, no allocation) against the detail view's (`trainee_d`): one
        // landing form (`land_dw`) multiplied in target order, so the same f64
        for (lineage, &q) in skills::lins(f.len()).iter().zip(&qv) {
            let t = f[lineage.trainee];
            let (a, b) = (
                m.trainee_fast(t, q, &white_hint_chances, &gold_hint_chances),
                m.trainee_d(t, q, &white_hint_chances, &gold_hint_chances).0,
            );
            assert_eq!(a.to_bits(), b.to_bits(), "{a} vs {b}");
        }
    }
}
/// dense chain_marginals equals the reference to float noise
#[test]
fn chain_marginals_dense_matches_ref() {
    let kinds: Vec<ChainKind> = vec![
        ChainKind::FILLER,
        THREE_LINKS,
        ChainKind { links: 3, stop: 2, stop_risk: 0 },
        ChainKind { links: 3, stop: 1, stop_risk: 0 },
        ChainKind { links: 4, stop: 4, stop_risk: 0 },
        ChainKind { links: 4, stop: 3, stop_risk: 0 },
        ChainKind { links: 5, stop: 5, stop_risk: 0 },
        ChainKind { links: 5, stop: 4, stop_risk: 0 },
        ChainKind { links: 5, stop: 2, stop_risk: 0 },
    ];
    let mut worst = 0.0f64;
    let mut multiset_count = 0;
    fn visit_multisets(
        kinds: &[ChainKind],
        start: usize,
        remaining: usize,
        current: &mut Vec<ChainKind>,
        visit: &mut dyn FnMut(&[ChainKind]),
    ) {
        if !current.is_empty() {
            visit(current);
        }
        if remaining == 0 {
            return;
        }
        for i in start..kinds.len() {
            current.push(kinds[i]);
            visit_multisets(kinds, i, remaining - 1, current, visit);
            current.pop();
        }
    }
    visit_multisets(&kinds, 0, 6, &mut vec![], &mut |multiset| {
        for events in [EVENTS_35, EVENTS_35_CARD] {
            let a = chain_marginals_ref(events, multiset);
            let b = chain_marginals_dense(events, multiset);
            multiset_count += 1;
            #[allow(clippy::iter_over_hash_type, reason = "a running max over every entry: the order changes nothing")]
            for (k, v) in &a {
                for (x, y) in v.iter().zip(&b[k]) {
                    worst = worst.max((x - y).abs());
                }
            }
        }
    });
    eprintln!("{multiset_count} multisets, max |dense - ref| = {worst:e}");
    assert!(worst < 1e-14);
}
/// The character rule: a card blocks its own character and, for a group card, every member.
#[test]
fn card_blocks_own_character_and_group_members() {
    let cards = fixture_cards();
    let sirius = cards.cards.iter().find(|c| c.id == 30081).expect("Team Sirius");
    let checks = crate::checks::get();
    // Special Week, Suzuka; not Vodka
    assert!(sirius.blocks(1001, checks) && sirius.blocks(1002, checks) && !sirius.blocks(1008, checks));
    let halo = cards.cards.iter().find(|c| c.name.contains("Tonight, We Waltz")).unwrap();
    assert!(halo.blocks(halo.character_id, checks) && !halo.blocks(1001, checks));
}
/// Retained chain-model regression values, plus the effect of freeing a slot by ending a chain early.
/// Current calibration: docs/RESEARCH.md section 8 (`8.fit_of_n_runs`).
#[test]
fn chain_model_calibration() {
    let m = chain_marginals(EVENTS_35, &[THREE_LINKS, THREE_LINKS]);
    assert!((m[&THREE_LINKS][2] - 0.866).abs() < 0.01, "{m:?}");
    assert!(m[&THREE_LINKS][0] > 0.99);
    let m9 = chain_marginals(EVENTS_35, &[THREE_LINKS, THREE_LINKS, THREE_LINKS]);
    assert!(m9[&THREE_LINKS][2] > 0.45 && m9[&THREE_LINKS][2] < 0.62, "{m9:?}");
    let one = chain_marginals(EVENTS_35, &[THREE_LINKS]);
    assert!(one[&THREE_LINKS][2] > 0.99);
    // ending a chain early frees its slot: the other card's links fire more often
    let early = chain_marginals(EVENTS_35, &[THREE_LINKS, ChainKind { links: 3, stop: 2, stop_risk: 0 }]);
    assert!(
        early[&THREE_LINKS][2] > m[&THREE_LINKS][2] && early[&ChainKind { links: 3, stop: 2, stop_risk: 0 }].len() == 2
    );
    let pm = event_pmf(EVENTS_35, 30);
    assert!(
        (pm.iter().sum::<f64>() - 1.0).abs() < 1e-12
            && (pm.iter().enumerate().map(|(e, p)| e as f64 * p).sum::<f64>() - 7.35).abs() < 1e-9
    );
}
/// A card's training draws over two targets in its pool: each target keeps its marginal from `hint_miss`, both
/// are hinted together less often than independent chances would say, and the outcomes are a distribution.
#[test]
fn training_draws_over_two_targets() {
    let cards = fixture_cards();
    let c = cards
        .cards
        .iter()
        .find(|c| c.skill_count > 0 && c.whites.contains(&20046) && c.whites.contains(&20161))
        .expect("a card with Ramp Up and Tail Held High");
    let deck_job = job_with(json!({}));
    let target_groups = [20046u32, 20161];
    let source_builder = Sources { deck_job: &deck_job, target_groups: &target_groups };
    let tr = source_builder.training_hints(c);
    assert_eq!(tr.len(), 2);
    let g = source_builder.train_group(c, &tr);
    assert!(g.iter().map(|x| x.probability).sum::<f64>() <= 1.0 + 1e-12);
    let gh = group_gh(&g);
    for (k, _, ph) in &gh {
        assert!((ph - tr[*k].probability).abs() < 1e-12, "{gh:?} vs {tr:?}");
    }
    let both = g.iter().filter(|x| x.rewards.len() == 2).map(|x| x.probability).sum::<f64>();
    assert!(both < tr[0].probability * tr[1].probability);
    let h = hint_base("unity")
        * (1.0 + c.hint_frequency / 100.0)
        * (1.0 + hint_spec("unity") * c.specialty_priority / 100.0);
    // the card's assembled sources carry the same odds (the group is not lost next to its other rewards)
    assert!((tr[0].probability - (1.0 - hint_miss(1, c.skill_count as usize, h, c.hint_levels))).abs() < 1e-12);
    let (mut white_misses, mut gold_misses) = (vec![1.0; 2], vec![1.0; 2]);
    source_builder.target_sources(c, None, 0).src_miss(&[1.0, 1.0, 1.0], &mut white_misses, &mut gold_misses);
    for k in 0..2 {
        assert!(
            (1.0 - white_misses[k] * gold_misses[k] - tr[k].probability).abs() < 1e-9 || gold_misses[k] < 1.0,
            "{white_misses:?} {gold_misses:?} {tr:?}"
        );
        assert!(white_misses[k] * gold_misses[k] < 1.0);
    }
}
/// The Lv-5 cap: a skill leaves a card's pool after ceil(5 / levels per draw) draws. Checked against brute-force
/// enumeration of every draw sequence, and on cases worked by hand (a +5 card draws without replacement).
#[test]
fn training_draws_leave_maxed_skills() {
    fn brute(k: usize, n: usize, draws_left: usize, cap: usize, counts: &mut Vec<usize>) -> f64 {
        if draws_left == 0 {
            return 1.0;
        }
        let avail: Vec<usize> = (0..n).filter(|&i| counts[i] < cap).collect();
        if avail.is_empty() {
            return 1.0;
        }
        let mut probability = 0.0;
        for &i in &avail {
            if i < k {
                continue;
            } // skills 0..k are the given ones: drawing one fails
            counts[i] += 1;
            probability += brute(k, n, draws_left - 1, cap, counts) / avail.len() as f64;
            counts[i] -= 1;
        }
        probability
    }
    for n in 2..=6 {
        for k in 1..=n {
            for t in 0..=6 {
                for cap in 1..=5 {
                    let b = brute(k, n, t, cap, &mut vec![0; n]);
                    assert!(
                        (miss_draws(k, n, t, cap) - b).abs() < 1e-12,
                        "k {k} n {n} t {t} cap {cap}: {} vs {b}",
                        miss_draws(k, n, t, cap)
                    );
                }
            }
        }
    }
    assert!((miss_draws(1, 5, 4, 1) - 0.2).abs() < 1e-12); // 4 of 5 skills, no repeats
    assert_eq!(miss_draws(2, 5, 4, 1), 0.0); // only 3 others to absorb 4 draws
    assert!((miss_draws(2, 5, 3, 1) - 0.1).abs() < 1e-12); // 1 / C(5, 3)
    assert!((miss_draws(1, 7, 2, 2) - (6.0f64 / 7.0).powi(2)).abs() < 1e-12); // 2 draws, cap 2: no skill leaves before the last
    // fractional h: floor / ceil mixture; levels per draw 1 -> cap 5, 3 or 4 -> 2, 5 -> 1
    assert!((hint_miss(1, 5, 4.25, 5) - (0.75 * 0.2 + 0.25 * 0.0)).abs() < 1e-12);
    assert!((hint_miss(1, 5, 2.0, 1) - 0.64).abs() < 1e-12);
}
/// A card-event stop risk ends its chain on the model's one share (1/3), whatever its own k of n, and on its own k of n
/// under the slots rule; a job's own stop keeps its k of n, so a pin of 1 of 1 stops the chain for certain, and the
/// card-event stop at a pinned link is left out.
#[test]
fn stop_share_and_job_pins() {
    let card = halo(); // King Halo [Tonight, We Waltz]: three links
    let target_groups = [20159u32];
    let kind_of = |job: Value| {
        let deck_job = job_with(job);
        Sources { deck_job: &deck_job, target_groups: &target_groups }.kind(&card, None)
    };
    let (event_stop, pin) = (json!([[30056, 2, 1, 2]]), json!([[30056, 1, 1, 1]]));
    assert_eq!(kind_of(json!({"event_stop_risk": event_stop})).stop_risk, stop_risk_code(&[(2, 1, 3)], 3));
    let slots = kind_of(json!({"event_stop_risk": event_stop, "chain_rule": "slots"}));
    assert_eq!(slots.stop_risk, stop_risk_code(&[(2, 1, 2)], 3));
    let pinned = kind_of(json!({"stop_risk": pin, "event_stop_risk": [[30056, 1, 1, 2], [30056, 2, 1, 2]]}));
    assert_eq!(pinned.stop_risk, stop_risk_code(&[(1, 1, 1), (2, 1, 3)], 3));
    let reach = &chain_marginals(EVENTS_35, &[pinned])[&pinned];
    assert!(reach[0] > 0.0 && reach[1] == 0.0 && reach[2] == 0.0, "{reach:?}");
}
/// Stop risk (IT's default choice ends the chain on some outcomes) is the exact mixture over where it stops.
#[test]
fn chain_stop_risk_is_a_mixture() {
    let r12 = stop_risk_code(&[(1, 1, 2), (2, 1, 2)], 3); // ends on 1 of 2 outcomes at links 1 and 2
    let r2 = stop_risk_code(&[(2, 1, 2)], 3);
    assert_eq!(stop_risk_code(&[(2, 1, 2), (3, 1, 2)], 2), 0); // links at or past the stop don't matter
    let full = chain_marginals(EVENTS_35, &[THREE_LINKS])[&THREE_LINKS].clone();
    let alone = chain_marginals(EVENTS_35, &[ChainKind { links: 3, stop: 3, stop_risk: r12 }])
        [&ChainKind { links: 3, stop: 3, stop_risk: r12 }]
        .clone();
    assert!(
        (alone[0] - full[0]).abs() < 1e-15
            && (alone[1] - 0.5 * full[1]).abs() < 1e-15
            && (alone[2] - 0.25 * full[2]).abs() < 1e-15,
        "{alone:?}"
    );
    // with a partner: 50% the deck is (full, full), 50% the risky card stops after link 2 and frees its slot
    let mix = chain_marginals(EVENTS_35, &[THREE_LINKS, ChainKind { links: 3, stop: 3, stop_risk: r2 }]);
    let a = chain_marginals(EVENTS_35, &[THREE_LINKS, THREE_LINKS]);
    let b = chain_marginals(EVENTS_35, &[THREE_LINKS, ChainKind { links: 3, stop: 2, stop_risk: 0 }]);
    for k in 0..3 {
        assert!((mix[&THREE_LINKS][k] - (0.5 * a[&THREE_LINKS][k] + 0.5 * b[&THREE_LINKS][k])).abs() < 1e-15);
    }
    assert!((mix[&ChainKind { links: 3, stop: 3, stop_risk: r2 }][2] - 0.5 * a[&THREE_LINKS][2]).abs() < 1e-15);
    assert!(mix[&THREE_LINKS][2] > a[&THREE_LINKS][2]);
}

/// The game's scenario bans reach the solver with the card data: Team Sirius out of Unity Cup and Grand Concert
/// decks, allowed in URA Finale and Trackblazer.
#[test]
fn scenario_bans_exported() {
    let cards = fixture_cards();
    for sc in ["unity", "gc"] {
        assert!(cards.restrict.get(sc).is_some_and(|v| v.contains(&30081)), "{sc}: {:?}", cards.restrict);
    }
    for sc in ["ura", "trackblazer"] {
        assert!(!cards.restrict.get(sc).is_some_and(|v| v.contains(&30081)));
    }
}
fn job_with(job: Value) -> DeckJob {
    serde_json::from_value(job).unwrap()
}
/// The card rule: of two running cards with 1 and 3 links left, each takes the next event half the time (the slots
/// rule: 1 in 4 and 3 in 4), read from the final states after exactly one event.
#[test]
fn card_rule_picks_running_cards_equally() {
    let one_link = ChainKind { links: 1, stop: 1, stop_risk: 0 };
    let next = |rule: PickRule| {
        let states = chain_states(ChainEvents { count: 1, probability: 1.0, rule }, &[one_link, THREE_LINKS]);
        let chance = |a: u8, b: u8| {
            states.iter().filter(|state| state.progress == [a, b]).map(|state| state.probability).sum::<f64>()
        };
        (chance(1, 0), chance(0, 1))
    };
    assert_eq!(next(PickRule::Card), (0.5, 0.5));
    assert_eq!(next(PickRule::Slots), (0.25, 0.75));
}
/// The job's chain-event law: the pal rate by default, rising with pals; its bounds over pal counts at their ends; a
/// slope of 0 or `chain_events` one fixed law. Pal cards raise the rate, group cards do not.
#[test]
fn chain_model_by_pals() {
    let model = job_with(json!({})).chain_model();
    assert!(model.pal_dependent());
    let rate = |pals: usize| model.events(pals).probability;
    assert_eq!(rate(0), expit(CHAIN_A));
    assert!((1..=6).all(|pals| rate(pals) > rate(pals - 1)), "{CHAIN_B}");
    assert_eq!(model.most_favourable(4).probability, rate(4));
    assert_eq!(model.least_favourable(4).probability, rate(0));
    let flat = job_with(json!({"chain_budget": [0.0, 0.0]})).chain_model();
    assert!(!flat.pal_dependent() && flat.events(5).probability == 0.5);
    let fixed = job_with(json!({"chain_events": [3, 0.25]})).chain_model();
    assert!(!fixed.pal_dependent() && fixed.events(6).count == 3 && fixed.events(6).probability == 0.25);
    // the card rule under either law unless the job sets `chain_rule: "slots"`
    assert!(model.events(2).rule == PickRule::Card && fixed.events(2).rule == PickRule::Card);
    let slots = job_with(json!({"chain_events": [21, 0.35], "chain_rule": "slots"})).chain_model();
    assert!(slots.events(3).rule == PickRule::Slots && slots.events(3).probability == 0.35);
    let cards = fixture_cards();
    let card = |id: u32| cards.cards.iter().find(|card| card.id == id).unwrap();
    // [Tracen Reception] Tazuna Hayakawa (pal), [Esteemed and Adored] Heirs to the Throne (group), an R card
    assert!(card(30021).raises_chain_rate());
    assert!(card(30067).pal && !card(30067).raises_chain_rate());
    assert!(!card(10001).raises_chain_rate());
}
fn halo() -> Card {
    let cards = fixture_cards();
    cards.cards.into_iter().find(|c| c.id == 30056).unwrap()
}
/// An event offering targets on different choices gives one deck entry per choice, each with one choice's
/// rewards; with only one of them a target, the card keeps a single entry with that choice.
#[test]
fn choices_become_variants() {
    let c = halo();
    // King Halo [Tonight, We Waltz], link 1: Stop Right There! (choice 1) or Uma Stan (choice 2)
    let deck_job = job_with(
        json!({"alts": [[30056, 1, [[[[20102, 1]]], [[[20159, 1]]]]]], "events": [[30056, 20102, 1, 1], [30056, 20159, 1, 1]]}),
    );
    let target_groups = [20159u32, 20102];
    let source_builder = Sources { deck_job: &deck_job, target_groups: &target_groups };
    let vs = source_builder.variants(&c, None);
    assert_eq!(vs.len(), 2);
    for (i, &v) in vs.iter().enumerate() {
        let (mut white_misses, mut gold_misses) = (vec![1.0; 2], vec![1.0; 2]);
        source_builder.target_sources(&c, None, v).src_miss(&[1.0, 1.0, 1.0], &mut white_misses, &mut gold_misses);
        // with every link firing, exactly one of the two whites is certain
        let got: Vec<bool> = white_misses.iter().map(|&x| x < 1e-12).collect();
        assert_eq!(got.iter().filter(|&&x| x).count(), 1, "variant {i}: {white_misses:?}");
    }
    let one = [20159u32];
    let sb1 = Sources { deck_job: &deck_job, target_groups: &one };
    assert_eq!(sb1.variants(&c, None), vec![0]);
    let (mut white_misses, mut gold_misses) = (vec![1.0], vec![1.0]);
    sb1.target_sources(&c, None, 0).src_miss(&[1.0, 1.0, 1.0], &mut white_misses, &mut gold_misses);
    assert!(white_misses[0] < 1e-12 && gold_misses[0] == 1.0);
}
/// One event's outcomes exclude each other: a gold on two of four outcomes and its white on the other two give
/// a hint whenever the link fires (independent 50% chances would give 75%).
#[test]
fn outcomes_of_one_event_are_exclusive() {
    let c = halo();
    let deck_job = job_with(json!({"alts": [[30056, 3, [[[[20203, 2]], [[20203, 1]], [[20203, 2]], [[20203, 1]]]]]]}));
    let target_groups = [20203u32];
    let source_builder = Sources { deck_job: &deck_job, target_groups: &target_groups };
    let (mut white_misses, mut gold_misses) = (vec![1.0], vec![1.0]);
    source_builder.target_sources(&c, None, 0).src_miss(&[1.0, 1.0, 0.8], &mut white_misses, &mut gold_misses);
    assert!((gold_misses[0] - 0.6).abs() < 1e-12, "{gold_misses:?}"); // P(no gold) = 1 - 0.8 * 0.5
    assert!((gold_misses[0] * white_misses[0] - 0.2).abs() < 1e-12, "{white_misses:?}"); // P(no hint) = 1 - 0.8
}
/// Measured outcome shares (`choice_weights`) replace equal outcomes: 3:1 between two skills, link firing 80%.
#[test]
fn outcome_shares_weight_alts() {
    let c = halo();
    let deck_job =
        job_with(json!({"alts": [[30056, 3, [[[[20102, 1]], [[20159, 1]]]]]], "alt_w": [[30056, 3, [[0.75, 0.25]]]]}));
    let target_groups = [20102u32, 20159];
    let source_builder = Sources { deck_job: &deck_job, target_groups: &target_groups };
    let (mut white_misses, mut gold_misses) = (vec![1.0; 2], vec![1.0; 2]);
    source_builder.target_sources(&c, None, 0).src_miss(&[1.0, 1.0, 0.8], &mut white_misses, &mut gold_misses);
    assert!(
        (white_misses[0] - (1.0 - 0.8 * 0.75)).abs() < 1e-12 && (white_misses[1] - (1.0 - 0.8 * 0.25)).abs() < 1e-12,
        "{white_misses:?}"
    );
}
/// The final-state distribution has the same per-link marginals as `chain_marginals`, stop risk included.
#[test]
fn chain_states_match_marginals() {
    let r = stop_risk_code(&[(2, 1, 2)], 3);
    for ks in [
        vec![THREE_LINKS],
        vec![ChainKind::FILLER, THREE_LINKS, THREE_LINKS],
        vec![
            ChainKind { links: 3, stop: 2, stop_risk: 0 },
            ChainKind { links: 3, stop: 3, stop_risk: r },
            ChainKind { links: 4, stop: 4, stop_risk: 0 },
        ],
        vec![ChainKind::FILLER; 6],
    ] {
        let mut ks = ks;
        ks.sort_unstable();
        for events in [EVENTS_35, EVENTS_35_CARD] {
            let st = chain_states(events, &ks);
            assert!((st.iter().map(|x| x.probability).sum::<f64>() - 1.0).abs() < 1e-12);
            let m = chain_marginals(events, &ks);
            for (i, kd) in ks.iter().enumerate() {
                for l in 1..=kd.stop {
                    let p: f64 = st.iter().filter(|x| x.progress[i] >= l).map(|x| x.probability).sum();
                    assert!(
                        (p - m[kd][l as usize - 1]).abs() < 1e-12,
                        "{ks:?} entry {i} link {l}: {p} vs {}",
                        m[kd][l as usize - 1]
                    );
                }
            }
        }
    }
}
/// `key_level`'s per-count progress grids project the same process as `chain_states`: each final state's chance is
/// the grids' value at its grid index summed over the event count's distribution (a stop risk and early ends included)
#[test]
fn key_level_matches_states() {
    let r = stop_risk_code(&[(2, 1, 2)], 3);
    let pmf = event_pmf(EVENTS_35, CHAIN_N as usize);
    for ks in [
        vec![THREE_LINKS],
        vec![ChainKind::FILLER, THREE_LINKS, THREE_LINKS],
        vec![
            ChainKind { links: 3, stop: 2, stop_risk: 0 },
            ChainKind { links: 3, stop: 3, stop_risk: r },
            ChainKind { links: 4, stop: 4, stop_risk: 0 },
        ],
        vec![ChainKind::FILLER; 6],
    ] {
        let mut ks = ks;
        ks.sort_unstable();
        // (the two pick rules alternate over the cases)
        let events = if ks.len() % 2 == 0 { EVENTS_35 } else { EVENTS_35_CARD };
        let (st, kl) = (chain_states(events, &ks), key_level(events, &ks));
        let at =
            |gi: usize| (0..kl.level_probabilities.len()).map(|m| pmf[m] * kl.level_probabilities[m][gi]).sum::<f64>();
        let mut seen = 0.0f64;
        for crate::deck::chains::ChainState { progress: s, probability: p } in st.iter() {
            let gi: usize = s.iter().zip(&kl.grid_strides).map(|(&x, &g)| x as usize * g).sum();
            let q = at(gi);
            seen += q;
            assert!((q - p).abs() < 1e-12, "{ks:?} state {s:?}: key_level {q} vs chain_states {p}");
        }
        // (and no grid mass outside the final states)
        assert!((seen - (0..kl.level_probabilities[0].len()).map(at).sum::<f64>()).abs() < 1e-12, "{ks:?}");
    }
}
/// Jobs with the same chain kinds but different event distributions each get their own (the caches key on both).
#[test]
fn chain_event_parameters_are_independent() {
    let first = ChainEvents { count: 2, probability: 0.25, rule: PickRule::Slots };
    let same_count = ChainEvents { count: 2, probability: 0.75, rule: PickRule::Slots };
    let shorter = ChainEvents { count: 1, probability: 0.25, rule: PickRule::Slots };
    // a filler's second link fires when the run gets both events: P(N = 2)
    for (events, both) in [(first, 0.0625), (same_count, 0.5625), (shorter, 0.0), (first, 0.0625)] {
        let marginals = chain_marginals(events, &[ChainKind::FILLER]);
        assert!((marginals[&ChainKind::FILLER][1] - both).abs() < 1e-15, "{both}");
        let states = chain_states(events, &[ChainKind::FILLER]);
        let done: f64 = states.iter().filter(|state| state.progress[0] == 2).map(|state| state.probability).sum();
        assert!((done - both).abs() < 1e-15, "{both}");
    }
}
/// The collapsed-state form equals the full enumeration (chains, coupled outcomes, a stop risk, fillers).
#[test]
fn joint_fast_matches_reference() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let skills = fixture_skills();
    let skill_job: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
    let m = Model::new(&d, &skills, &skill_job);
    let g1 = |k: usize, level: u8, p: f64| -> Group {
        vec![EventOutcome { probability: p, rewards: vec![TargetReward { target: k, level }] }]
    };
    let two = |a: usize, b: usize| -> Group {
        vec![
            EventOutcome { probability: 0.5, rewards: vec![TargetReward { target: a, level: 2 }] },
            EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: b, level: 1 }] },
        ]
    };
    // chain cards with single-target and coupled outcomes, a coupled fixed event, a stop risk, fillers
    let a = TargetSources {
        training_hints: vec![TrainingHint { target: 0, probability: 0.3 }],
        chain_outcomes: vec![
            ChainOutcome { link: 1, outcomes: g1(1, 1, 1.0) },
            ChainOutcome { link: 2, outcomes: two(1, 2) },
            ChainOutcome { link: 3, outcomes: g1(0, 2, 0.5) },
        ],
        ..Default::default()
    }
    .finish();
    let b = TargetSources {
        fixed_outcomes: vec![two(3, 0)],
        chain_outcomes: vec![ChainOutcome { link: 2, outcomes: g1(3, 2, 1.0) }],
        ..Default::default()
    }
    .finish();
    let c = TargetSources {
        training_hints: vec![TrainingHint { target: 2, probability: 0.2 }],
        chain_outcomes: vec![ChainOutcome { link: 1, outcomes: two(2, 3) }],
        ..Default::default()
    }
    .finish();
    let e = TargetSources { fixed_outcomes: vec![g1(1, 2, 0.25)], ..Default::default() }.finish();
    let risk = stop_risk_code(&[(2, 1, 2)], 3);
    let ents: Vec<SourceEntry<TargetSources>> = vec![
        SourceEntry { sources: a, kind: THREE_LINKS },
        SourceEntry { sources: b, kind: ChainKind::FILLER },
        SourceEntry { sources: c, kind: ChainKind { links: 3, stop: 3, stop_risk: risk } },
        SourceEntry { sources: e, kind: ChainKind::NONE },
    ];
    for fill in [0usize, 2] {
        for u in [0, 5, 20].map(crate::uma::UmaIndex) {
            let x = joint_fast(
                &m,
                JointQuery::unfolded(&m, EVENTS_35, u, &[0.1, 0.5, 0.9]),
                &ents,
                fill,
                skill_job.targets.len(),
                true,
                None,
            );
            let y = joint_trainee_ref(
                &m,
                JointQuery::unfolded(&m, EVENTS_35, u, &[0.1, 0.5, 0.9]),
                &ents,
                fill,
                skill_job.targets.len(),
            );
            for (p, r) in x.iter().zip(&y) {
                assert!(
                    (p.all - r.all).abs() < 1e-13
                        && (p.all_but_one - r.all_but_one).abs() < 1e-13
                        && p.outfit == r.outfit,
                    "fill {fill} uma {u}: {p:?} vs {r:?}"
                );
            }
            let z = joint_fast(
                &m,
                JointQuery::unfolded(&m, EVENTS_35, u, &[0.1, 0.5, 0.9]),
                &ents,
                fill,
                skill_job.targets.len(),
                false,
                None,
            );
            for (p, r) in z.iter().zip(&y) {
                assert!((p.all - r.all).abs() < 1e-13);
            }
        }
    }
}

/// Career planner: a per-target inheritance chance folded into the trainee's event hint chances, evaluated at
/// q = 0, gives the same joint odds as the loop model at q; the spark scale multiplies P(all) by the product of
/// the scales; with two targets the exact marginals satisfy P(A) + P(B) = P(all) + P(at least one).
#[test]
fn career_fold_and_scale() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let skills = fixture_skills();
    let skill_job: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}],
        "trainee_hints": [[1001, 20166, 0.3]]})).unwrap();
    let m = Model::new(&d, &skills, &skill_job);
    let g1 = |k: usize, level: u8, p: f64| -> Group {
        vec![EventOutcome { probability: p, rewards: vec![TargetReward { target: k, level }] }]
    };
    let two = |a: usize, b: usize| -> Group {
        vec![
            EventOutcome { probability: 0.5, rewards: vec![TargetReward { target: a, level: 2 }] },
            EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: b, level: 1 }] },
        ]
    };
    let a = TargetSources {
        training_hints: vec![TrainingHint { target: 0, probability: 0.3 }],
        chain_outcomes: vec![
            ChainOutcome { link: 1, outcomes: g1(1, 1, 1.0) },
            ChainOutcome { link: 2, outcomes: two(1, 2) },
            ChainOutcome { link: 3, outcomes: g1(0, 2, 0.5) },
        ],
        ..Default::default()
    }
    .finish();
    let b = TargetSources {
        fixed_outcomes: vec![two(3, 0)],
        chain_outcomes: vec![ChainOutcome { link: 2, outcomes: g1(3, 2, 1.0) }],
        ..Default::default()
    }
    .finish();
    let c = TargetSources {
        training_hints: vec![TrainingHint { target: 2, probability: 0.2 }],
        chain_outcomes: vec![ChainOutcome { link: 1, outcomes: two(2, 3) }],
        ..Default::default()
    }
    .finish();
    let ents: Vec<SourceEntry<TargetSources>> = vec![
        SourceEntry { sources: a, kind: THREE_LINKS },
        SourceEntry { sources: b, kind: ChainKind::FILLER },
        SourceEntry { sources: c, kind: THREE_LINKS },
    ];
    let rel = |x: f64, y: f64| (x - y).abs() / y.abs().max(1e-300);
    for u in [0, 5, 20].map(crate::uma::UmaIndex) {
        for q in [0.1, 0.6] {
            let x = joint_fast(
                &m,
                JointQuery::unfolded(&m, EVENTS_35, u, &[q]),
                &ents,
                1,
                skill_job.targets.len(),
                true,
                None,
            )[0];
            let folded: Vec<Vec<f64>> =
                m.event_chances[u].iter().map(|o| o.iter().map(|e| 1.0 - (1.0 - e) * (1.0 - q)).collect()).collect();
            let y = joint_fast(
                &m,
                JointQuery {
                    chain_events: EVENTS_35,
                    checks: crate::checks::get(),
                    trainee: u,
                    inheritance_chances: &[0.0],
                    event_chances: &folded,
                },
                &ents,
                1,
                skill_job.targets.len(),
                true,
                None,
            )[0];
            assert!(
                rel(y.all, x.all) < 1e-12 && rel(y.all_but_one, x.all_but_one) < 1e-12 && x.outfit == y.outfit,
                "fold uma {u} q {q}: {x:?} vs {y:?}"
            );
            let su = vec![1.1f64.powi(-1), 1.0, 1.1f64.powi(-3), 1.1f64.powi(-6)];
            let mut ext = JointExt {
                scales: crate::deck::joint::SparkScales { uncertain: su.clone(), guaranteed: vec![1.1f64.powi(-2)] },
                marginals: vec![],
                sets: vec![],
                per_outfit: vec![],
            };
            let z = joint_fast(
                &m,
                JointQuery {
                    chain_events: EVENTS_35,
                    checks: crate::checks::get(),
                    trainee: u,
                    inheritance_chances: &[0.0],
                    event_chances: &folded,
                },
                &ents,
                1,
                skill_job.targets.len(),
                true,
                Some(&mut ext),
            )[0];
            let c = su.iter().product::<f64>() * 1.1f64.powi(-2);
            assert!(rel(z.all, c * y.all) < 1e-12 && z.outfit == y.outfit, "scale uma {u}: {} vs {}", z.all, c * y.all);
            assert!(z.all_but_one >= z.all && z.all_but_one <= 1.0);
        }
    }
    // two targets, coupled sources: marginals against P(all) and P(all but at most one)
    let sj2: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20166}]})).unwrap();
    let m2 = Model::new(&d, &skills, &sj2);
    let p = TargetSources {
        training_hints: vec![TrainingHint { target: 0, probability: 0.2 }],
        chain_outcomes: vec![
            ChainOutcome { link: 1, outcomes: two(0, 1) },
            ChainOutcome { link: 2, outcomes: g1(1, 1, 0.7) },
        ],
        ..Default::default()
    }
    .finish();
    let r = TargetSources {
        fixed_outcomes: vec![two(1, 0)],
        chain_outcomes: vec![ChainOutcome { link: 1, outcomes: g1(0, 2, 0.4) }],
        ..Default::default()
    }
    .finish();
    let ents2: Vec<SourceEntry<TargetSources>> =
        vec![SourceEntry { sources: p, kind: ChainKind::FILLER }, SourceEntry { sources: r, kind: ChainKind::FILLER }];
    for u in [0, 5, 20].map(crate::uma::UmaIndex) {
        let mut ext = JointExt {
            scales: crate::deck::joint::SparkScales { uncertain: vec![0.9, 0.8], guaranteed: vec![] },
            marginals: vec![],
            sets: vec![0.0],
            per_outfit: vec![],
        };
        let j = joint_fast(&m2, JointQuery::unfolded(&m2, EVENTS_35, u, &[0.3]), &ents2, 0, 2, true, Some(&mut ext))[0];
        let (m0, m1) = (ext.marginals[j.outfit * 2], ext.marginals[j.outfit * 2 + 1]);
        assert!(rel(m0 + m1, j.all + j.all_but_one) < 1e-12, "uma {u}: {m0} + {m1} vs {} + {}", j.all, j.all_but_one);
        // the landed sets: they sum to 1, {both} is P(all), and each target's sets sum to its marginal
        let st = &ext.sets[(j.outfit << 2)..(j.outfit << 2) + 4];
        assert!(
            (st.iter().sum::<f64>() - 1.0).abs() < 1e-12
                && rel(st[3], j.all) < 1e-12
                && rel(st[1] + st[3], m0) < 1e-12
                && rel(st[2] + st[3], m1) < 1e-12,
            "sets {st:?}"
        );
    }
}

/// The negative-association bound is at least the exact value (chains, a card feeding two targets, a target fed by
/// two cards, early ends, a stop risk, fillers), and close to it.
#[test]
fn upper_bound_covers_exact() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let skills = fixture_skills();
    let skill_job: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
    let m = Model::new(&d, &skills, &skill_job);
    let g1 = |k: usize, level: u8, p: f64| -> Group {
        vec![EventOutcome { probability: p, rewards: vec![TargetReward { target: k, level }] }]
    };
    let two = |a: usize, b: usize| -> Group {
        vec![
            EventOutcome { probability: 0.5, rewards: vec![TargetReward { target: a, level: 2 }] },
            EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: b, level: 1 }] },
        ]
    };
    let a = TargetSources {
        training_hints: vec![TrainingHint { target: 0, probability: 0.3 }],
        chain_outcomes: vec![
            ChainOutcome { link: 1, outcomes: g1(1, 1, 1.0) },
            ChainOutcome { link: 2, outcomes: two(1, 2) },
            ChainOutcome { link: 3, outcomes: g1(0, 2, 0.5) },
        ],
        ..Default::default()
    }
    .finish();
    let b = TargetSources {
        fixed_outcomes: vec![two(3, 0)],
        chain_outcomes: vec![ChainOutcome { link: 2, outcomes: g1(3, 2, 1.0) }],
        ..Default::default()
    }
    .finish();
    let c = TargetSources {
        training_hints: vec![TrainingHint { target: 2, probability: 0.2 }],
        chain_outcomes: vec![ChainOutcome { link: 3, outcomes: g1(2, 2, 0.6) }],
        ..Default::default()
    }
    .finish();
    let e = TargetSources { fixed_outcomes: vec![g1(1, 2, 0.25)], ..Default::default() }.finish();
    let risk = stop_risk_code(&[(2, 1, 2)], 3);
    let decks: Vec<Vec<SourceEntry<TargetSources>>> = vec![
        vec![
            SourceEntry { sources: a.clone(), kind: THREE_LINKS },
            SourceEntry { sources: b.clone(), kind: ChainKind::FILLER },
            SourceEntry { sources: e.clone(), kind: ChainKind::NONE },
        ],
        vec![
            SourceEntry { sources: a.clone(), kind: THREE_LINKS },
            SourceEntry { sources: c.clone(), kind: THREE_LINKS },
            SourceEntry { sources: b.clone(), kind: ChainKind::FILLER },
        ],
        vec![
            SourceEntry { sources: a.clone(), kind: ChainKind { links: 3, stop: 2, stop_risk: 0 } },
            SourceEntry { sources: c.clone(), kind: ChainKind { links: 3, stop: 3, stop_risk: risk } },
            SourceEntry { sources: b, kind: ChainKind::FILLER },
        ],
        vec![
            SourceEntry { sources: a, kind: THREE_LINKS },
            SourceEntry { sources: c, kind: ChainKind { links: 3, stop: 1, stop_risk: 0 } },
            SourceEntry { sources: e, kind: ChainKind::NONE },
        ],
    ];
    let mut covered = 0;
    for ents in &decks {
        for fill in [0usize, 1, 3] {
            for u in [0, 5, 20].map(crate::uma::UmaIndex) {
                let qs = [0.1, 0.5, 0.9];
                let x = joint_fast(
                    &m,
                    JointQuery::unfolded(&m, EVENTS_35, u, &qs),
                    ents,
                    fill,
                    skill_job.targets.len(),
                    false,
                    None,
                );
                let er: Vec<SourceEntry<&TargetSources>> =
                    ents.iter().map(|entry| SourceEntry { sources: &entry.sources, kind: entry.kind }).collect();
                if let Some(ub) = upper_na(&m, JointQuery::unfolded(&m, EVENTS_35, u, &qs), &er, fill) {
                    covered += 1;
                    for (i, y) in x.iter().enumerate() {
                        assert!(
                            ub[i] >= y.all * (1.0 - 1e-12) && ub[i] <= y.all * 1.5,
                            "fill {fill} uma {u} q {}: bound {} exact {}",
                            qs[i],
                            ub[i],
                            y.all
                        );
                    }
                }
            }
        }
    }
    assert!(covered >= 30, "bound covered only {covered} cases");
}

/// Groups whose outcomes each hint one target pass the orthant check; an outcome hinting two targets together fails.
#[test]
fn group_orthant_check() {
    assert!(group_nuod(&vec![
        EventOutcome { probability: 0.5, rewards: vec![TargetReward { target: 0, level: 2 }] },
        EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: 1, level: 1 }] }
    ]));
    assert!(!group_nuod(&vec![
        EventOutcome {
            probability: 0.5,
            rewards: vec![TargetReward { target: 0, level: 1 }, TargetReward { target: 1, level: 1 }]
        },
        EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: 1, level: 1 }] }
    ]));
    assert!(group_nuod(&vec![
        EventOutcome {
            probability: 0.2,
            rewards: vec![TargetReward { target: 0, level: 1 }, TargetReward { target: 1, level: 1 }]
        },
        EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: 0, level: 1 }] },
        EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: 1, level: 1 }] }
    ])); // 0.2 <= 0.5 * 0.5
}

/// Exceeding the exhaustive-check budget must not certify a positively correlated group, or shift past u32.
#[test]
fn group_orthant_check_declines_unchecked_groups() {
    for n in [20, 21, 32, 33] {
        // Any pair has joint probability 0.5, above the product of its marginals (0.25).
        let g = vec![EventOutcome {
            probability: 0.5,
            rewards: (0..n).map(|k| TargetReward { target: k, level: 1 }).collect(),
        }];
        assert!(!group_nuod(&g), "{n} targets");
        for src in [
            TargetSources { fixed_outcomes: vec![g.clone()], ..Default::default() },
            TargetSources { chain_outcomes: vec![ChainOutcome { link: 1, outcomes: g }], ..Default::default() },
        ] {
            assert!(src.finish().uncertified_orthant, "{n} targets");
        }
    }
}

/// Inline event alternatives can reach the unchecked-group case without exponential training-hint setup.
#[test]
fn unchecked_inline_event_disables_orthant_bound() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let skills = fixture_skills();
    let target_groups: Vec<u32> = skills.skills.iter().take(21).map(|s| s.0).collect();
    assert_eq!(target_groups.len(), 21);
    let targets: Vec<Value> = target_groups.iter().map(|g| json!({"group": g})).collect();
    let skill_job: SkillJob = serde_json::from_value(json!({"targets": targets})).unwrap();
    let model = Model::new(&d, &skills, &skill_job);
    let mut card = halo();
    card.skill_count = 0;
    card.whites.clear();
    let rewards: Vec<(u32, u8)> = target_groups.iter().map(|&g| (g, 1)).collect();
    let deck_job = job_with(json!({"alts": [[card.id, 1, [[rewards, []]]]]}));
    let source_builder = Sources { deck_job: &deck_job, target_groups: &target_groups };
    let src = source_builder.target_sources(&card, None, 0);
    assert_eq!(src.chain_outcomes.len(), 1);
    assert_eq!(src.chain_outcomes[0].outcomes[0].probability, 0.5);
    assert_eq!(src.chain_outcomes[0].outcomes[0].rewards.len(), 21);
    assert!(src.uncertified_orthant);
    assert!(
        upper_na(
            &model,
            JointQuery::unfolded(&model, EVENTS_35, crate::uma::UmaIndex(0), &[0.0]),
            &[SourceEntry { sources: &src, kind: source_builder.kind(&card, None) }],
            0
        )
        .is_none()
    );
}

/// The level check behind the cap-free bound (`group_nuod_lvl`): single-target outcomes pass, an outcome hinting two
/// targets together fails; gold on one target with white on another in one outcome fails where the any-hint check
/// (`group_nuod`) passes (P(gold 0, 1) = 0.2 > 0.2 x 0.5); and for decks of covered groups the state value is at
/// least the exact value.
#[test]
fn group_level_orthant_check() {
    assert!(group_nuod_lvl(&vec![
        EventOutcome { probability: 0.5, rewards: vec![TargetReward { target: 0, level: 2 }] },
        EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: 1, level: 1 }] }
    ]));
    assert!(!group_nuod_lvl(&vec![
        EventOutcome {
            probability: 0.5,
            rewards: vec![TargetReward { target: 0, level: 1 }, TargetReward { target: 1, level: 1 }]
        },
        EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: 1, level: 1 }] }
    ]));
    let g: Group = vec![
        EventOutcome {
            probability: 0.2,
            rewards: vec![TargetReward { target: 0, level: 2 }, TargetReward { target: 1, level: 1 }],
        },
        EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: 0, level: 1 }] },
        EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: 1, level: 1 }] },
    ];
    assert!(group_nuod(&g) && !group_nuod_lvl(&g));
    // such a group must not reach `upper_na`: its marginals understate the gold-and-white outcome (a coverage-1
    // trainee on target 0: exact W[0.5 W + 0.2 (G - W)] against the marginals' W[0.5 W + 0.1 (G - W)])
    assert!(TargetSources { fixed_outcomes: vec![g], ..Default::default() }.finish().uncertified_orthant);
    // three targets, white only, joint terms below the products (a card's training draws)
    assert!(group_nuod_lvl(&vec![
        EventOutcome { probability: 0.2, rewards: vec![TargetReward { target: 0, level: 1 }] },
        EventOutcome { probability: 0.2, rewards: vec![TargetReward { target: 1, level: 1 }] },
        EventOutcome { probability: 0.2, rewards: vec![TargetReward { target: 2, level: 1 }] },
        EventOutcome {
            probability: 0.03,
            rewards: vec![TargetReward { target: 0, level: 1 }, TargetReward { target: 1, level: 1 }]
        },
        EventOutcome {
            probability: 0.03,
            rewards: vec![TargetReward { target: 1, level: 1 }, TargetReward { target: 2, level: 1 }]
        },
        EventOutcome {
            probability: 0.03,
            rewards: vec![TargetReward { target: 0, level: 1 }, TargetReward { target: 2, level: 1 }]
        }
    ]));
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let skills = fixture_skills();
    let skill_job: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
    let m = Model::new(&d, &skills, &skill_job);
    let g1 = |k: usize, level: u8, p: f64| -> Group {
        vec![EventOutcome { probability: p, rewards: vec![TargetReward { target: k, level }] }]
    };
    let two = |a: usize, b: usize| -> Group {
        vec![
            EventOutcome { probability: 0.5, rewards: vec![TargetReward { target: a, level: 2 }] },
            EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: b, level: 1 }] },
        ]
    };
    // (an outcome hinting two targets at once, at most independent at every level: covered, though not `group_assoc`)
    let three: Group = vec![
        EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: 0, level: 1 }] },
        EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: 2, level: 1 }] },
        EventOutcome { probability: 0.3, rewards: vec![TargetReward { target: 3, level: 1 }] },
        EventOutcome {
            probability: 0.02,
            rewards: vec![TargetReward { target: 0, level: 1 }, TargetReward { target: 2, level: 1 }],
        },
    ];
    let a = TargetSources {
        training_hints: vec![TrainingHint { target: 0, probability: 0.3 }],
        chain_outcomes: vec![
            ChainOutcome { link: 1, outcomes: g1(1, 1, 1.0) },
            ChainOutcome { link: 2, outcomes: two(1, 2) },
            ChainOutcome { link: 3, outcomes: g1(0, 2, 0.5) },
        ],
        ..Default::default()
    }
    .finish();
    let b = TargetSources {
        fixed_outcomes: vec![two(3, 0), three],
        chain_outcomes: vec![ChainOutcome { link: 2, outcomes: g1(3, 2, 1.0) }],
        ..Default::default()
    }
    .finish();
    let c = TargetSources {
        training_hints: vec![TrainingHint { target: 2, probability: 0.2 }],
        chain_outcomes: vec![ChainOutcome { link: 3, outcomes: g1(2, 2, 0.6) }],
        ..Default::default()
    }
    .finish();
    assert!(a.certified_cap_free && b.certified_cap_free && c.certified_cap_free && b.uncertified_association);
    let decks: [Vec<SourceEntry<TargetSources>>; 2] = [
        vec![
            SourceEntry { sources: a.clone(), kind: THREE_LINKS },
            SourceEntry { sources: b.clone(), kind: ChainKind::FILLER },
        ],
        vec![
            SourceEntry { sources: a, kind: THREE_LINKS },
            SourceEntry { sources: c, kind: THREE_LINKS },
            SourceEntry { sources: b, kind: ChainKind::FILLER },
        ],
    ];
    for ents in decks {
        for fill in [0usize, 2] {
            for u in [0, 5, 20].map(crate::uma::UmaIndex) {
                let qs = [0.1, 0.5, 0.9];
                let x = joint_fast(
                    &m,
                    JointQuery::unfolded(&m, EVENTS_35, u, &qs),
                    &ents,
                    fill,
                    skill_job.targets.len(),
                    false,
                    None,
                );
                let er: Vec<SourceEntry<&TargetSources>> =
                    ents.iter().map(|entry| SourceEntry { sources: &entry.sources, kind: entry.kind }).collect();
                let sv = state_value_cf(&m, JointQuery::unfolded(&m, EVENTS_35, u, &qs), &er, fill).unwrap();
                for (i, y) in x.iter().enumerate() {
                    assert!(
                        sv[i] >= y.all * (1.0 - 1e-12),
                        "fill {fill} uma {u}: state value {} exact {}",
                        sv[i],
                        y.all
                    );
                }
            }
        }
    }
}

/// With no chain cards and no event touching two targets, every source is independent: the exact value equals
/// the product of per-target odds the search ranks by.
#[test]
fn joint_equals_product_without_correlation() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let skills = fixture_skills();
    let skill_job: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
    let m = Model::new(&d, &skills, &skill_job);
    let nu = m.uncertain.len();
    let g1 = |k: usize, level: u8, p: f64| -> Group {
        vec![EventOutcome { probability: p, rewards: vec![TargetReward { target: k, level }] }]
    };
    let a = TargetSources {
        training_hints: vec![
            TrainingHint { target: 0, probability: 0.3 },
            TrainingHint { target: 2, probability: 0.2 },
        ],
        fixed_outcomes: vec![g1(1, 2, 0.5), g1(3, 2, 0.5), g1(3, 1, 1.0)],
        ..Default::default()
    }
    .finish();
    let b = TargetSources {
        training_hints: vec![TrainingHint { target: 1, probability: 0.4 }],
        fixed_outcomes: vec![g1(0, 2, 0.25)],
        ..Default::default()
    }
    .finish();
    let (mut white_misses, mut gold_misses) = (vec![1.0; nu], vec![1.0; nu]);
    for x in [&a, &b] {
        x.src_miss(&[], &mut white_misses, &mut gold_misses);
    }
    let white_hint_chances: Vec<f64> = white_misses.iter().map(|x| 1.0 - x).collect();
    let gold_hint_chances: Vec<f64> = gold_misses.iter().map(|x| 1.0 - x).collect();
    for u in [0, 5, 20].map(crate::uma::UmaIndex) {
        for q in [0.3, 0.75] {
            let j = joint(
                &m,
                JointQuery::unfolded(&m, EVENTS_35, u, &[q]),
                &[
                    SourceEntry { sources: a.clone(), kind: ChainKind::NONE },
                    SourceEntry { sources: b.clone(), kind: ChainKind::NONE },
                ],
                0,
                skill_job.targets.len(),
                true,
            );
            let p = m.fixed_factor() * m.trainee_fast(u, q, &white_hint_chances, &gold_hint_chances);
            assert!((j[0].all - p).abs() < 1e-14, "uma {u} q {q}: {} vs {p}", j[0].all);
        }
    }
}
/// `top_asc` and `top_desc` keep the stable sorts' first k values bit for bit, ties (0 and -0 among them) in input order.
#[test]
fn top_k_is_stable_sort_prefix() {
    let pool = [0.0f64, -0.0, 0.25, 0.5, 1.0, 0.5, f64::INFINITY, 1e-300, 0.25];
    let mut s = 12345u64;
    for _ in 0..4000 {
        s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let n = (s >> 60) as usize;
        let k = ((s >> 56) & 7) as usize;
        let v: Vec<f64> = (0..n).map(|i| pool[((s >> (i * 4 % 48)) % pool.len() as u64) as usize]).collect();
        let (mut a, mut d) = (vec![], vec![]);
        for &x in &v {
            top_asc(&mut a, k, x);
            top_desc(&mut d, k, x);
        }
        let mut sa = v.clone();
        sa.sort_by(|x, y| x.partial_cmp(y).unwrap());
        let mut sd = v.clone();
        sd.sort_by(|x, y| y.partial_cmp(x).unwrap());
        let bits = |x: &[f64]| x.iter().map(|y| y.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&a), bits(&sa[..k.min(n)]), "{v:?} k {k}");
        assert_eq!(bits(&d), bits(&sd[..k.min(n)]), "{v:?} k {k}");
    }
}
/// The ng/nh landing factor's forms agree to the bit: `land_factor`, `LandPre::f` (its constants taken once) and the
/// expression `JointPrep::evaluate` and `upper_na` wrote out before they called it, on random and non-finite inputs
#[test]
fn land_factor_forms_match() {
    use crate::skills::GEN_WHITE;
    let reference = |st: u8, ew: f64, q: f64, ng: f64, nh: f64| {
        let pg = 1.0 - ng;
        match st {
            2 => GEN_GOLD,
            1 => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg),
            _ => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg - nh * (1.0 - ew) * (1.0 - q)),
        }
    };
    let mut s = 0x2545_f491_4f6c_dd1d_u64;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    let odd = [f64::NAN, f64::INFINITY, -0.0, 0.0, 1.0, 1e-300];
    for it in 0..20000usize {
        let st = (rnd() * 3.0) as u8;
        let mut f = |i: usize| if it % 5 == 0 && rnd() < 0.3 { odd[(it + i) % odd.len()] } else { rnd() };
        let (ew, q, ng, nh) = (f(0), f(1), f(2), f(3));
        for (g, h) in [(ng, nh), (ng, 0.0), (0.0, 0.0)] {
            let r = reference(st, ew, q, g, h).to_bits();
            assert_eq!(land_factor(st, ew, q, g, h).to_bits(), r, "{st} {ew} {q} {g} {h}");
            assert_eq!(LandPre::new(st, ew, q).f(g, h).to_bits(), r, "{st} {ew} {q} {g} {h}");
        }
    }
}

/// The packed search's limits (`MAX_DECK`, `MAX_UNCERTAIN`, `MAX_KINDS`, locks within the deck) refuse a job in
/// `deck_entries` with an error, before any table is built from it; the page's six-card job passes.
#[test]
fn deck_limits_refuse_with_an_error() {
    let cards = fixture_cards();
    let banned = cards.restrict.get("unity").cloned().unwrap_or_default();
    let chain: Vec<&Card> =
        cards.cards.iter().filter(|c| !c.pal && c.links >= 2 && c.rarity >= 2 && !banned.contains(&c.id)).collect();
    assert!(chain.len() >= 20, "{} chain cards", chain.len());
    let target_groups = [20159u32];
    let err = |deck_job: &DeckJob, target_groups: &[u32]| match deck_entries(&cards, deck_job, target_groups, &[]) {
        Ok(_) => None,
        Err(e) => Some(e["error"].as_str().expect("an error message").to_string()),
    };
    assert_eq!(err(&job_with(json!({})), &target_groups), None);
    for size in [0, 7, 8, usize::MAX] {
        assert!(err(&job_with(json!({ "deck_size": size })), &target_groups).is_some(), "deck_size {size}");
    }
    let many: Vec<u32> = (0..33).map(|i| 20000 + i).collect();
    assert!(err(&job_with(json!({})), &many).is_some(), "33 uncertain targets");
    assert_eq!(err(&job_with(json!({})), &many[..32]), None);
    // locks: as many as the deck holds, not one more (duplicates and unknown ids count nothing, as before)
    let ids: Vec<u32> = chain.iter().map(|c| c.id).collect();
    assert_eq!(err(&job_with(json!({"lock_cards": ids[..6], "deck_size": 6})), &target_groups), None);
    assert_eq!(err(&job_with(json!({"lock_cards": [ids[0], ids[0], 1, 2, 3, 4, 5, 6, 7]})), &target_groups), None);
    assert!(err(&job_with(json!({"lock_cards": ids[..7]})), &target_groups).is_some(), "7 locks");
    assert!(
        err(&job_with(json!({"lock_cards": ids[..3], "deck_size": 2})), &target_groups).is_some(),
        "3 locks, deck of 2"
    );
    // chain kinds: each card its own stop-risk code, so at least its own kind; a job passes exactly when its entries'
    // kinds with the fillers' (2, 2) are at most MAX_KINDS
    let (mut passed, mut refused) = (0, 0);
    for n in 1..=20 {
        let cs = &chain[..n];
        let deck_job = job_with(json!({
            "events": cs.iter().map(|c| json!([c.id, target_groups[0], 1, 1])).collect::<Vec<_>>(),
            "stop_risk": cs.iter().enumerate().map(|(i, c)| json!([c.id, 1, i / 15, 1 + i % 15])).collect::<Vec<_>>(),
        }));
        match deck_entries(&cards, &deck_job, &target_groups, &[]) {
            Ok(e) => {
                assert!(kinds_all(&e.kind).len() <= MAX_KINDS, "{n} cards");
                passed += 1;
            }
            Err(e) => {
                assert!(e["error"].as_str().unwrap().contains("kinds"), "{n} cards: {e}");
                refused += 1;
            }
        }
    }
    assert!(passed > 0 && refused > 0, "{passed} passed, {refused} refused");
}

/// Decks with a nested chain (`deck::nested`): one card's chain feeding two targets, the second at a later link, with
/// nothing else giving the later one (the shape of a Seiun Sky [Paint the Sky Red] deck without pals, 4.00 in the
/// career search), the same with a training source, and one card's chain feeding three targets at links 1, 2 and 3
/// (exact / product 1 / (ab) for links reached with chances a, b, c): each one's exact value is at most its bound B,
/// B / P at most the configuration's nested cap (`caps_with`), and the nested decks and the three-target one pass the
/// class-1 cap (1.05; the three-target ratio is printed). A nested pair beside a chain card feeding a third target
/// gets B too, an assumption (`multi_feeder_cap`; exact / B and exact / max(1.25 P, B) are printed). The shapes no bound covers get none:
/// a second chain feeder of a nested target, two cards each feeding one target, three feeders, one chain-varying
/// target.
#[test]
fn pair_bound_covers_exact() {
    use crate::deck::nested::{ChainLaw, NestedChains, NestedDeck, NestedTrainee};
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let skills = fixture_skills();
    let skill_job: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
    let m = Model::new(&d, &skills, &skill_job);
    let target_count = m.uncertain.len();
    let g1 = |k: usize, level: u8| -> Group {
        vec![EventOutcome { probability: 1.0, rewards: vec![TargetReward { target: k, level }] }]
    };
    let chain = |links: &[(u8, usize, u8)]| -> TargetSources {
        TargetSources {
            chain_outcomes: links
                .iter()
                .map(|&(link, k, level)| ChainOutcome { link, outcomes: g1(k, level) })
                .collect(),
            ..Default::default()
        }
        .finish()
    };
    let nested = chain(&[(2, 0, 2), (3, 2, 1)]);
    let blank = TargetSources::default().finish();
    let trainer =
        TargetSources { training_hints: vec![TrainingHint { target: 2, probability: 0.3 }], ..Default::default() }
            .finish();
    let first_late = chain(&[(3, 0, 1)]);
    let first_early = chain(&[(2, 0, 2)]);
    let second_late = chain(&[(3, 2, 1)]);
    let three = chain(&[(1, 0, 1), (2, 2, 1), (3, 3, 1)]);
    let third_feeder = chain(&[(2, 3, 1)]);
    // (what the bound is for a shape: proven, an assumption, or none)
    #[derive(PartialEq, Eq)]
    enum Bound {
        Proven,
        Assumed,
        None,
    }
    let decks: Vec<(&str, Vec<TargetSources>, Bound)> = vec![
        (
            "nested alone",
            vec![nested.clone(), blank.clone(), blank.clone(), blank.clone(), blank.clone()],
            Bound::Proven,
        ),
        ("nested, a training source", vec![nested.clone(), trainer, blank.clone(), blank.clone()], Bound::Proven),
        ("three targets on one chain", vec![three, blank.clone(), blank.clone(), blank.clone()], Bound::Proven),
        ("nested, a third target's feeder", vec![nested.clone(), third_feeder, blank.clone()], Bound::Assumed),
        (
            "nested, a second chain feeder",
            vec![nested.clone(), first_late.clone(), blank.clone(), blank.clone()],
            Bound::None,
        ),
        (
            "two cards, one target each",
            vec![first_early.clone(), second_late.clone(), blank.clone(), blank.clone()],
            Bound::None,
        ),
        ("three feeders", vec![nested, first_late, second_late, blank.clone()], Bound::None),
        ("one chain-varying target", vec![first_early, blank.clone(), blank], Bound::None),
    ];
    let (mut nested_ratio, mut three_ratio) = (0.0f64, 0.0f64);
    for (name, sources, shape) in &decks {
        let kinds = vec![THREE_LINKS; sources.len()];
        let marginals = chain_marginals(EVENTS_35_CARD, &kinds);
        let entry_misses: Vec<(Vec<f64>, Vec<f64>)> = sources
            .iter()
            .map(|entry| {
                let (mut white, mut gold) = (vec![1.0; target_count], vec![1.0; target_count]);
                entry.src_miss(&marginals[&THREE_LINKS], &mut white, &mut gold);
                (white, gold)
            })
            .collect();
        // the deck's odds per target: one minus the product of its entries' misses (white given no gold, gold)
        let (white, gold): (Vec<f64>, Vec<f64>) = (0..target_count)
            .map(|t| {
                let white_miss: f64 = entry_misses.iter().map(|(entry_white, _)| entry_white[t]).product();
                let gold_miss: f64 = entry_misses.iter().map(|(_, entry_gold)| entry_gold[t]).product();
                (1.0 - white_miss, 1.0 - gold_miss)
            })
            .unzip();
        let chains = NestedChains::new(sources, &kinds, target_count);
        let law = ChainLaw::new(EVENTS_35_CARD, kinds.clone());
        let marginal = |j: usize, t: usize| (entry_misses[j].1[t], entry_misses[j].1[t] * entry_misses[j].0[t]);
        let entries: Vec<usize> = (0..sources.len()).collect();
        let owned: Vec<SourceEntry<TargetSources>> =
            sources.iter().map(|entry| SourceEntry { sources: entry.clone(), kind: THREE_LINKS }).collect();
        let deck_chances = crate::skills::DeckChances { model: &m, event_chances: &m.event_chances };
        for u in [0, 5, 20].map(crate::uma::UmaIndex) {
            for q in [0.0, 0.3] {
                let trainee = NestedTrainee::new(&deck_chances, u, q);
                let caps = chains.caps_with(&law, &marginal, &trainee);
                let product = m.fixed_factor() * m.trainee_fast(u, q, &white, &gold);
                let exact = joint(
                    &m,
                    JointQuery::unfolded(&m, EVENTS_35_CARD, u, &[q]),
                    &owned,
                    0,
                    skill_job.targets.len(),
                    false,
                )[0]
                .all;
                let deck = NestedDeck { entries: &entries, white: &white, gold: &gold };
                let bound = trainee.bound_with(&chains, &law, deck, &marginal);
                if *shape == Bound::None {
                    assert_eq!(bound, f64::NEG_INFINITY, "{name}: a bound for a shape it does not cover");
                    continue;
                }
                assert!(bound.is_finite(), "{name}: no bound for a shape it covers");
                if *shape == Bound::Assumed {
                    // (the assumption: exact <= max(class cap x P, B), the class-2 cap 1.25 here)
                    println!(
                        "{name} uma {u} q {q}: exact / B {}, / max(1.25 P, B) {}",
                        exact / bound,
                        exact / bound.max(1.25 * product)
                    );
                    continue;
                }
                assert!(exact <= bound * (1.0 + 1e-12), "{name} uma {u} q {q}: exact {exact} above its bound {bound}");
                assert!(
                    bound <= caps.nested * product * (1.0 + 1e-12),
                    "{name} uma {u} q {q}: B / P {} above the nested cap {}",
                    bound / product,
                    caps.nested
                );
                if name.starts_with("nested") {
                    nested_ratio = nested_ratio.max(exact / product);
                } else if name.starts_with("three") {
                    three_ratio = three_ratio.max(exact / product);
                }
            }
        }
    }
    assert!(nested_ratio > 1.05, "the nested decks' exact / product {nested_ratio} stays within the class-1 cap");
    println!("three targets on one chain: exact / product at most {three_ratio}");
    assert!(three_ratio > 1.05, "the three-target deck's exact / product {three_ratio} stays within the class-1 cap");
}

/// The nested shapes the nested-chain bound covers only under an assumption are named (`caps::multi_feeder`,
/// `multi_feeder_cap`): a nested pair beside a chain card feeding a third target, beside a chainless entry with an
/// uncertified group (class 3), a three-target chain beside another feeder. Not named: a nested chain whose card
/// alone feeds every chain-varying target (proven in any class, a card feeding three targets included), decks without
/// a nested chain, and a class-1 nested pair with another feeder (`two_feeder_cap`'s).
#[test]
fn multi_feeder_names_the_nested_shapes_left_assumed() {
    use crate::deck::caps::{cap_class, multi_feeder};
    let reward = |target: usize| TargetReward { target, level: 1 };
    let chain = |links: &[(u8, usize)]| -> TargetSources {
        TargetSources {
            chain_outcomes: links
                .iter()
                .map(|&(link, k)| ChainOutcome {
                    link,
                    outcomes: vec![EventOutcome { probability: 1.0, rewards: vec![reward(k)] }],
                })
                .collect(),
            ..Default::default()
        }
        .finish()
    };
    let uncertified = TargetSources {
        fixed_outcomes: vec![vec![EventOutcome { probability: 0.5, rewards: vec![reward(1), reward(3), reward(4)] }]],
        ..Default::default()
    }
    .finish();
    assert!(uncertified.uncertified_association);
    let pair = chain(&[(2, 0), (3, 2)]);
    let three = chain(&[(1, 0), (2, 2), (3, 3)]);
    let (first, second, third) = (chain(&[(2, 0)]), chain(&[(3, 2)]), chain(&[(2, 3)]));
    let cases: [(&str, Vec<&TargetSources>, bool); 8] = [
        ("a nested pair alone", vec![&pair], false),
        ("three targets on one chain", vec![&three], false),
        ("a nested pair and a third chain-varying target", vec![&pair, &third], true),
        ("a nested pair and a chainless uncertified group", vec![&pair, &uncertified], true),
        ("three targets on one chain and another feeder", vec![&three, &first], true),
        ("a nested pair and a feeder of one of its targets", vec![&pair, &first], false),
        ("two cards, one target each", vec![&first, &second], false),
        ("three chain-varying targets, no nested chain", vec![&first, &second, &third], false),
    ];
    for (name, deck, named) in cases {
        let mask = deck.iter().fold(0u64, |mask, entry| mask | entry.cv_mask());
        let feeding_slots = deck.iter().filter(|entry| entry.cv_mask() & u64::from(u32::MAX) != 0).count();
        assert_eq!(multi_feeder(mask, feeding_slots), named, "{name}: class {}", cap_class(mask));
    }
}
