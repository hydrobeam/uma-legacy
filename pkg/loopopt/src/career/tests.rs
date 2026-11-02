//! Career-job tests: the jobs' keys, inheritance copies against their lineage, and the kept state's round trip.
use super::*;
use crate::career::inherit::{aff_factor, inherit, insp_miss2, spark_scale};
use crate::career::state::STATE_VERSION;
use crate::request::Request;
use serde_json::{Value, json};
pub(super) type Hook = Box<dyn for<'env> FnOnce(&mut Env<'env>, &Pool)>;
thread_local! { pub(super) static ENV_HOOK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) }; }
/// The Global era's data file `name`.
fn global(name: &str) -> String {
    std::fs::read_to_string(format!("{}/data/global/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}
/// The page's worker as the tests drive it: the Global data, and the kept work its jobs share (in a cell, so a test's
/// helpers can each run jobs on it, as the page's calls share the worker's)
struct Worker {
    data: Data,
    skills: SkillsRaw,
    cards: CardsRaw,
    kept: std::cell::RefCell<KeptState>,
}
impl Worker {
    /// A job as the page's worker takes it (`Request`, as JSON: web.rs `Solver::call`) on its kept work: a career job
    /// with the Global card events added, the kept work out or in; its reply as JSON (a refusal where it does not parse)
    fn call(&self, job: &Value) -> Value {
        let kept = &mut self.kept.borrow_mut();
        let reply = match serde_json::from_value::<Request>(job.clone()) {
            Ok(Request::Career(mut job)) => {
                let events = crate::deck::sources::event_lists(
                    &self.skills,
                    serde_json::from_str(&global("card_events.json")).unwrap(),
                );
                events.add_to(&mut job.deck);
                run_career_in(&self.data, &self.skills, &self.cards, &job, kept)
            }
            Ok(Request::StateOut) => CareerReply::StateOut(state::state_out(kept)),
            Ok(Request::StateIn { state }) => state::state_in(kept, &state),
            Ok(Request::Deck(_)) => panic!("a deck job"),
            Err(e) => return json!({"error": format!("bad request: {e}")}),
        };
        serde_json::to_value(reply).unwrap()
    }
}
/// val/plan's p16 plan job (a `Request`, as the page sends it), and a worker on the Global data it runs on, with
/// nothing kept
fn p16() -> (Worker, Value) {
    let data = crate::data::load_str(&global("data.json"));
    let skills: SkillsRaw = serde_json::from_str(&global("web_skills.json")).unwrap();
    let cards: CardsRaw = serde_json::from_str(&global("web_cards.json")).unwrap();
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap(),
    )
    .unwrap();
    (Worker { data, skills, cards, kept: std::cell::RefCell::default() }, file["job"].clone())
}
/// `run_career_in` on val/plan's p16 pool (G1 wins added, so the race parts are not 0), with `hook` given its Env and
/// pool
pub(super) fn with_env(hook: Hook) {
    let (worker, mut job) = p16();
    job["career"].as_object_mut().unwrap().remove("next");
    for (i, legacy) in job["career"]["legacies"].as_array_mut().unwrap().iter_mut().enumerate() {
        legacy["wins"] = json!([(i % 5) as u64, 7 + (i % 3) as u64, 20]);
        for (j, grandparent) in legacy["parents"].as_array_mut().unwrap().iter_mut().enumerate() {
            if !grandparent.is_null() {
                grandparent["wins"] = json!([(i + j) % 5]);
            }
        }
    }
    ENV_HOOK.with(|cell| *cell.borrow_mut() = Some(hook));
    worker.call(&job);
    assert!(ENV_HOOK.with(|cell| cell.borrow().is_none()), "the hook did not run");
}
/// Each key the worker keeps work under (`career::keys`) reads its parts of the job and no others: a change to a part
/// it reads gives another key, a change to any other part the same key; a field sent at its default is the field left
/// out.
#[test]
fn job_keys_read_their_parts() {
    let (_, v) = p16();
    let keys = |job: &Value| {
        let Ok(Request::Career(job)) = serde_json::from_value::<Request>(job.clone()) else { panic!("{job}") };
        [job.setup_key(), job.search_key(), job.borrowing_search_key(), job.plan_key()]
    };
    let base = keys(&v);
    let two_veterans = json!(v["career"]["legacies"].as_array().unwrap()[..2]);
    // (part, value, read by: setup, search, borrowing search, plan)
    let cases: [(&[&str], Value, [bool; 4]); 12] = [
        (&["skills", "targets"], json!([{"group": 20159}]), [true; 4]),
        (&["skills", "rank_class"], json!("ss_and_up"), [true; 4]),
        (&["deck", "scenario"], json!("ura"), [true; 4]),
        (&["career", "legacies"], two_veterans, [false, true, true, true]),
        (&["career", "top"], json!(2), [false, true, true, true]),
        (&["career", "compare"], json!(true), [false, true, true, false]),
        (&["career", "no_reports"], json!(true), [false; 4]),
        (&["career", "borrowed_parent"], json!("never"), [false, true, false, true]),
        (&["career", "given"], json!("1001:0,1"), [true; 4]),
        (&["career", "next", "roll"], json!([1, 1]), [false; 4]),
        (&["career", "next", "robust"], json!("all"), [false; 4]),
        (&["career", "next", "margin"], json!(0.5), [false, false, false, true]),
    ];
    for (path, value, read) in cases {
        let mut job = v.clone();
        let (last, parents) = path.split_last().unwrap();
        parents.iter().fold(&mut job, |at, key| &mut at[*key])[*last] = value;
        let changed = keys(&job);
        assert_eq!(std::array::from_fn(|i| changed[i] != base[i]), read, "{path:?}");
    }
    let mut defaults = v;
    defaults["career"]["borrowed_parent"] = json!("allow");
    defaults["skills"]["rank_class"] = json!("below_ss");
    defaults["career"]["next"]["sur"] = json!("search");
    assert_eq!(keys(&defaults), base, "fields at their defaults");
}
/// The copies of the inheritance chance agree to the bit with `lineage` (slots, `inherit`, `scale`): `gen_cands`'
/// ceilings (against `ceiling` at `lineage`'s chances, as `solve` takes them) and `her_slot` (against
/// `inherit` on the slots with hers emptied), on a pool with random-star slots (parents and grandparents) beside
/// fixed ones
#[test]
fn inheritance_copies_match_lineage() {
    with_env(Box::new(|env: &mut Env, pool: &Pool| {
        let mut p = pool.clone();
        for x in 0..p.veterans.len() {
            if x % 3 == 0 {
                p.veterans[x].star_mix = Some([0.5, 0.4, 0.1]);
            }
            if x % 4 == 1
                && let Some(g) = p.parents[x].first_mut()
            {
                g.stars = std::rc::Rc::new(p.veterans[x].stars.iter().map(|&s| s.max(1)).collect());
                g.star_mix = Some([0.2, 0.3, 0.5]);
            }
        }
        let Env { ref setup, ref mut state, .. } = *env;
        let trainees = &setup.owned;
        let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<u64>>();
        let mut n = 0usize;
        for fix in [vec![], vec![2usize]] {
            let candidates = setup.gen_cands(state, &p, &fix, trainees, |_, _| true, |ub, _, t, a, b| (ub, t, a, b));
            assert!(!candidates.is_empty());
            for (ub, t, a, b) in candidates {
                let lineage = Lineage { trainee: t, first_parent: a, second_parent: b };
                let inherit::LineageChances { chances: q, spark_scale: c, .. } = setup.lineage(&p, lineage);
                assert_eq!(
                    ub.to_bits(),
                    setup.ceiling(state, t, &q, c).unwrap().to_bits(),
                    "gen_cands ceiling t {t} pair {a} {b}"
                );
                for (y, iv) in [(a, b), (b, a)] {
                    let her_lineage = Lineage { trainee: t, first_parent: y, second_parent: iv };
                    let sl = setup.slots(&p, her_lineage);
                    let pos = sl.iter().position(|z| z.parent && z.legacy == iv).unwrap();
                    let mut sl0 = sl.clone();
                    sl0[pos].stars = std::rc::Rc::new(vec![0; setup.target_count]);
                    sl0[pos].mix = None;
                    let inherit::Inheritance { chances: q0, carriers: nc0 } = inherit(&sl0, setup.target_count);
                    let hs = setup.her_slot(&p, her_lineage).unwrap();
                    let fa = aff_factor(sl[pos].relation_points, sl[pos].race);
                    let misses: Vec<f64> = setup.uncertain_targets.iter().map(|&i| 1.0 - q0[i]).collect();
                    assert_eq!(bits(&hs.misses_without_her), bits(&misses));
                    assert_eq!(
                        bits(&hs.miss_by_stars),
                        bits(&[1.0, insp_miss2(1.0, fa), insp_miss2(2.0, fa), insp_miss2(3.0, fa)])
                    );
                    assert!(
                        hs.target_spark_scales
                            .iter()
                            .zip(&nc0)
                            .all(|(w, &m)| w[0].to_bits() == spark_scale(m).to_bits()
                                && w[1].to_bits() == spark_scale(m + 1).to_bits())
                    );
                }
                n += 1;
            }
        }
        assert!(n > 1000, "{n} candidates");
    }));
}
/// The worker's kept work leaves (`state_out`) and comes back (`state_in`) bit for bit after a plan (the same string out
/// again), and a roll after the round trip answers as before it. A store of another version, one that does not decode
/// to its end (not base64, cut short, a byte past its end) or one of the earlier encoding (JSON records) is refused
/// whole with an error, nothing installed; one whose decks or careers do not fit the setup it meets
/// (`CareerSetup::seen_fits`, `solved_fits`) is not used there.
#[test]
fn state_round_trips_and_refuses_another_version() {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;
    let (worker, v) = p16();
    let run = |job: &Value| worker.call(job);
    let state_out = || run(&json!({"cmd": "state_out"}));
    let state_in = |st: &Value| run(&json!({"cmd": "state_in", "state": st}));
    let plan = run(&v);
    assert!(plan.get("error").is_none(), "{}", plan["error"]);
    let out = state_out();
    let st = out["state"].clone();
    assert!(st.is_string(), "the state is not one string: {st}");
    {
        let mut kept = worker.kept.borrow_mut();
        assert!(!kept.bests.is_empty() && !kept.seen.is_empty() && !kept.searches.is_empty(), "nothing kept");
        assert!(!kept.flags.is_empty() && !kept.rolls.is_empty(), "no flags or roll state kept");
        assert!(kept.searches.iter().any(|search| search.ruled.iter().any(|proofs| !proofs.is_empty())), "no proof");
        let deck_setups = kept.decks.len();
        assert!(deck_setups > 0, "the completed job did not publish deck memos");
        let saved = state::kept_swap(&mut kept, KeptState::default());
        assert!(kept.bests.is_empty() && kept.seen.is_empty() && kept.searches.is_empty());
        assert!(kept.rolls.is_empty() && kept.decks.is_empty(), "simulation must isolate rolls and decks");
        state::kept_swap(&mut kept, saved);
        assert_eq!(json!(state::state_out(&kept).state), st, "simulation must restore every serialized collection");
        assert_eq!(kept.decks.len(), deck_setups);
    }
    let mut rolled = v.clone();
    rolled["career"]["next"]["roll"] = json!(vec![1u8; v["skills"]["targets"].as_array().unwrap().len()]);
    // (a roll of the plan the worker keeps: answered from it, `kind` "roll"; without one, the plan's own)
    let roll = || {
        let r = run(&rolled);
        let mut roll = if r["kind"] == "roll" { r["roll"].clone() } else { r["next"]["roll"].clone() };
        assert!(roll["outcome"]["y"].is_number(), "{r}");
        for k in ["ms", "prof"] {
            roll.as_object_mut().unwrap().remove(k);
        }
        roll
    };
    let roll0 = roll();
    let empty = json!(state::state_out(&KeptState::default()).state);
    assert_eq!(state_in(&empty)["kind"], "state_in");
    assert_eq!(state_in(&st)["kind"], "state_in");
    assert_eq!(state_out(), out, "the state did not round-trip");
    assert_eq!(roll(), roll0, "a roll after the round trip");

    // ---- refused whole, the kept work as it was ----
    let kept = state_out();
    let refused = |what: &str, bad: &Value, error: &str| {
        let r = state_in(bad);
        assert!(r["error"].as_str().is_some_and(|reason| reason.starts_with(error)), "{what}: {r}");
        assert_eq!(state_out(), kept, "{what}: something was installed");
    };
    let other_version = json!(state::encode(STATE_VERSION - 1, &worker.kept.borrow()));
    refused("another version", &other_version, "state: another version");
    refused("not base64", &json!("not base64!"), "state: does not parse");
    let bytes = BASE64.decode(st.as_str().unwrap()).unwrap();
    refused("cut short", &json!(BASE64.encode(&bytes[..bytes.len() - 1])), "state: does not parse");
    let longer = [bytes.as_slice(), &[0]].concat();
    refused("a byte past its end", &json!(BASE64.encode(longer)), "state: does not parse");
    let old = json!({"v": 12, "bests": [], "seen": [], "solve": [], "flags": [], "rolls": [], "caps": []});
    refused("the earlier encoding", &old, "bad request");

    // ---- stores that decode but do not fit the setup they meet: not used there ----
    // (the store as it left, with `edit` made to what it holds)
    let edited = |edit: &dyn Fn(&mut KeptState)| {
        assert_eq!(state_in(&st)["kind"], "state_in");
        edit(&mut worker.kept.borrow_mut());
        state_out()["state"].clone()
    };
    fn first_ref(kept: &mut KeptState) -> &mut Ref {
        kept.seen[0].refs.0.iter_mut().flatten().next().expect("a fully checked deck kept")
    }
    let seen_used = |st: &Value| {
        assert_eq!(state_in(st)["kind"], "state_in");
        let used = std::rc::Rc::new(std::cell::Cell::new(false));
        let u = std::rc::Rc::clone(&used);
        ENV_HOOK.with(|c| {
            *c.borrow_mut() =
                Some(Box::new(move |env: &mut Env, _: &Pool| u.set(env.state.seen.iter().any(|r| !r.is_empty()))));
        });
        let mut job = v.clone();
        job["career"].as_object_mut().unwrap().remove("next");
        run(&job);
        used.get()
    };
    assert!(seen_used(&st), "the stored proofs were not used");
    let bad = edited(&|kept| first_ref(kept).deck.entries[0] = 1_000_000);
    assert!(!seen_used(&bad), "a deck entry past the deck space was used");
    let bad = edited(&|kept| {
        first_ref(kept).inheritance_chances.pop();
    });
    assert!(!seen_used(&bad), "chances for fewer targets were used");
    // a kept main search whose career names a trainee past the umas: searched again, the same plan
    let bad = edited(&|kept| {
        for search in &mut kept.searches {
            if let Some(found) = search.solved.found.first_mut() {
                found.trainee = UmaIndex(1_000_000);
            }
        }
    });
    assert_eq!(state_in(&bad)["kind"], "state_in");
    let again = run(&v);
    assert!(again.get("error").is_none(), "{}", again["error"]);
    assert_eq!((&again["next"]["choice"], &again["results"]), (&plan["next"]["choice"], &plan["results"]));

    // ---- class flags: a store unions its flags into the worker's and never clears one; a kept main search reused
    // after its class was flagged is unproven ----
    let flagged = edited(&|kept| {
        for flags in kept.flags.values_mut() {
            flags.breached = true;
        }
    });
    assert_eq!(state_in(&flagged)["kind"], "state_in");
    assert_eq!(state_in(&st)["kind"], "state_in");
    assert!(worker.kept.borrow().flags.values().all(|flags| flags.breached), "a store cleared a flag");
    let mut search_job = v.clone();
    search_job["career"].as_object_mut().unwrap().remove("next");
    let reused = run(&search_job);
    assert_eq!((&reused["complete"], &reused["proven_top"]), (&json!(false), &json!(false)), "{reused}");
    assert!(reused["unproven"].as_array().is_some_and(|reasons| reasons.contains(&json!("cap_breach"))), "{reused}");
}

/// A kept plan carries the class flags it rested on (`Bests::flags`): stored with a breach in its snapshot and none in
/// the worker's flags, and the worker's flag table churned past any eviction limit (100 other classes), the same plan
/// job reusing the kept plan and main search reports the breach (the main search not proven, rows not proven).
#[test]
fn a_kept_plan_keeps_its_flags_through_flag_churn() {
    use crate::career::search::{Assumes, Assumption};
    let (worker, v) = p16();
    let run = |job: &Value| worker.call(job);
    let plan = run(&v);
    assert!(plan.get("error").is_none(), "{}", plan["error"]);
    assert_eq!(plan["proven_top"], json!(true), "the first plan is not proven: {}", plan["unproven"]);
    {
        let mut kept = worker.kept.borrow_mut();
        assert!(!kept.bests[0].flags.is_empty(), "the kept plan carries no flags");
        for kept_plan in &mut kept.bests {
            for (_, flags) in &mut kept_plan.flags {
                flags.breached = true;
            }
        }
        let assuming = ClassFlags { breached: false, assumes: Assumes::from([Assumption::TwoFeederCap]) };
        kept.flags = (0..100u64).map(|i| (0xdead_0000 + i, assuming.clone())).collect();
    }
    let st = run(&json!({"cmd": "state_out"}))["state"].clone();
    state::kept_swap(&mut worker.kept.borrow_mut(), KeptState::default());
    assert_eq!(run(&json!({"cmd": "state_in", "state": st}))["kind"], "state_in");
    let again = run(&v);
    assert_eq!((&again["complete"], &again["proven_top"]), (&json!(false), &json!(false)), "{}", again["unproven"]);
    assert!(again["unproven"].as_array().is_some_and(|reasons| reasons.contains(&json!("cap_breach"))));
    let rows = again["next"]["options"].as_array().unwrap();
    assert!(rows.iter().any(|row| row["proven"] == json!(false)), "no row unproven");
    assert!(again["next"]["umas_unproven"].as_u64().is_some_and(|count| count > 0));
}

/// The reviewers' case for kept plans across trainee filters: on the nested_nopal pool with cards 30067 and 30080 as
/// plain pals (no relevant card of theirs), trainee 1004's plain-pal rate prefix under [1003, 1004] differs from the one
/// under [1004], so her class content key differs, and so does the plan key's deck space (`plan_space_key`): a
/// [1003, 1004] plan is never reused by a [1004] one. The same filter twice gives the same keys.
#[test]
fn a_trainee_filter_that_changes_a_class_changes_the_plan_key() {
    let (worker, _) = p16();
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/nested_nopal.json")).unwrap(),
    )
    .unwrap();
    let mut job = file["job"].clone();
    job["deck"]["exclude_cards"].as_array_mut().unwrap().retain(|card| card != 30067 && card != 30080);
    let her = UmaIndex(worker.data.umas.iter().position(|uma| uma.id == 1004).unwrap());
    let keys = |trainees: Value| {
        let mut filtered = job.clone();
        filtered["career"]["trainees"] = trainees;
        let seen = std::rc::Rc::new(std::cell::Cell::new((0u64, 0u64, 0usize)));
        let sink = std::rc::Rc::clone(&seen);
        ENV_HOOK.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(move |env: &mut Env, _: &Pool| {
                let plain_pals = env.setup.space.plain_pal_ids().filter(|&id| id == 30067 || id == 30080).count();
                sink.set((env.setup.class_key[her], env.setup.plan_space_key, plain_pals));
            }));
        });
        worker.call(&filtered);
        seen.get()
    };
    let (wide, narrow) = (keys(json!([1003, 1004])), keys(json!([1004])));
    assert_eq!((wide.2, narrow.2), (2, 2), "30067 and 30080 are not both plain pals");
    assert_ne!(wide.0, narrow.0, "her class content is the same under both filters");
    assert_ne!(wide.1, narrow.1, "the plan key's deck space is the same under both filters");
    assert_eq!(keys(json!([1004])), narrow, "the same filter gave other keys");
}
