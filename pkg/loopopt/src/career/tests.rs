//! Career-job tests: `json_without`, inheritance copies against their lineage, and the kept state's round trip.
use super::*;
use crate::career::inherit::{aff_factor, inherit, insp_miss2, spark_scale};
use crate::career::state::{STATE_VERSION, json_without};
type Hook = Box<dyn for<'env> FnOnce(&mut Env<'env>, &Pool)>;
thread_local! { pub(super) static ENV_HOOK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) }; }
/// val/plan's p16 plan job with its events, and the Global data it runs on
fn p16() -> (Data, SkillsRaw, CardsRaw, Value) {
    let r = concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/");
    let rd = |f: &str| std::fs::read_to_string(format!("{r}{f}")).unwrap();
    let data = crate::data::load_str(&rd("data.json"));
    let skills: SkillsRaw = serde_json::from_str(&rd("web_skills.json")).unwrap();
    let cards: CardsRaw = serde_json::from_str(&rd("web_cards.json")).unwrap();
    let mut v: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap(),
    )
    .unwrap();
    let db: Value = serde_json::from_str(&rd("card_events.json")).unwrap();
    crate::deck::sources::apply_events_db(&mut v, &skills, &db);
    (data, skills, cards, v)
}
/// `run_career` on val/plan's p16 pool (G1 wins added, so the race parts are not 0), with `hook` given its Env and
/// pool
fn with_env(hook: Hook) {
    let (data, skills, cards, mut job) = p16();
    job.as_object_mut().unwrap().remove("next");
    for (i, legacy) in job["legacies"].as_array_mut().unwrap().iter_mut().enumerate() {
        legacy["wins"] = json!([(i % 5) as u64, 7 + (i % 3) as u64, 20]);
        for (j, grandparent) in legacy["parents"].as_array_mut().unwrap().iter_mut().enumerate() {
            grandparent["wins"] = json!([(i + j) % 5]);
        }
    }
    ENV_HOOK.with(|cell| *cell.borrow_mut() = Some(hook));
    run_career(&data, &skills, &cards, &job);
    assert!(ENV_HOOK.with(|cell| cell.borrow().is_none()), "the hook did not run");
}
/// `json_without` writes the text of a clone with those keys removed, byte for byte (the keys `setup_fp`, the plan's
/// `roll_key` and the main search's key hash: stored in the page's state)
#[test]
fn json_without_matches_clone() {
    let p16: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json")).unwrap(),
    )
    .unwrap();
    let mut rolled = p16.clone();
    rolled["roll"] = json!({"stars": [1, 2]});
    rolled["next"]["roll"] = json!([0, 3]);
    rolled["compare"] = json!(true);
    let cases = [
        p16,
        rolled,
        json!({}),
        json!({"a": 1}),
        json!({"next": {"roll": 1}}),
        json!([1, {"b": 2.5e-300}]),
        json!(-0.0),
        json!({"x": {"y": {}}, "z": []}),
    ];
    let drops: [(&[&str], &[&str]); 4] = [
        (
            &[
                "legacies",
                "trainees",
                "fix",
                "top",
                "per_trainee",
                "cmd",
                "mode",
                "compare",
                "next",
                "no_reports",
                "borrowed_parent",
            ],
            &[],
        ),
        (&["next", "no_reports"], &[]),
        (&["no_reports", "compare", "roll"], &["roll"]),
        (&["a", "x"], &["roll"]),
    ];
    for v in &cases {
        for (dropped_keys, next_dropped) in drops {
            let mut k = v.clone();
            if let Some(o) = k.as_object_mut() {
                for x in dropped_keys {
                    o.remove(*x);
                }
                if let Some(n) = o.get_mut("next").and_then(|n| n.as_object_mut()) {
                    for x in next_dropped {
                        n.remove(*x);
                    }
                }
            }
            assert_eq!(json_without(v, dropped_keys, next_dropped), k.to_string());
        }
    }
}
/// The copies of the inheritance chance agree to the bit with `lineage` (slots, `inherit`, `scale`): `lineage_qc`,
/// `gen_cands`' ceilings (against `ceiling` at `lineage`'s chances, as `solve` takes them) and `her_slot` (against
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
        let ks: Vec<usize> = (0..setup.target_count).collect();
        let trainees = &setup.owned;
        let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<u64>>();
        let (mut n, mut tg, mut tg_t) = (0usize, vec![], crate::uma::NONE);
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
                if t != tg_t {
                    tg.clear();
                    tg_t = t;
                }
                let mut qq = vec![0.0f64; setup.target_count];
                let cq = setup.lineage_qc(&p, lineage, &ks, &mut qq, &mut tg);
                assert_eq!((bits(&qq), cq.to_bits()), (bits(&q), c.to_bits()), "lineage_qc t {t} pair {a} {b}");
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
                    assert_eq!(bits(&hs.chances_without_her), bits(&q0));
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
/// The worker's kept work leaves (`state_out`) and comes back (`state_in`) bit for bit after a plan, and a roll after
/// the round trip answers as before it. A malformed store (a count past `STATE_MAX_N`, an index past its list, a NaN,
/// a chance outside [0, 1], a deck past six cards, a truncated vector, a roll state that does not fit together) is
/// refused whole with an error, nothing installed; one whose decks or careers do not fit the setup it meets
/// (`CareerSetup::seen_fits`, `solved_fits`) is not used there.
#[test]
fn state_round_trips_and_refuses_malformed() {
    let (data, skills, cards, v) = p16();
    let run = |job: &Value| run_career(&data, &skills, &cards, job);
    let state_out = || run(&json!({"cmd": "state_out"}));
    let state_in = |st: &Value| run(&json!({"cmd": "state_in", "state": st}));
    let plan = run(&v);
    assert!(plan.get("error").is_none(), "{}", plan["error"]);
    let out = state_out();
    let st = out["state"].clone();
    KEPT.with(|worker| {
        let mut worker = worker.borrow_mut();
        let deck_setups = worker.decks.len();
        assert!(deck_setups > 0, "the completed job did not publish deck memos");
        let saved = state::kept_swap(&mut worker, KeptState::default());
        assert!(worker.bests.is_empty() && worker.seen.is_empty() && worker.searches.is_empty());
        assert!(worker.rolls.is_empty() && worker.decks.is_empty(), "simulation must isolate rolls and decks");
        state::kept_swap(&mut worker, saved);
        assert_eq!(state::state_out(&worker), out, "simulation must restore every serialized collection");
        assert_eq!(worker.decks.len(), deck_setups);
    });
    for k in ["bests", "seen", "solve", "rolls"] {
        assert!(st[k].as_array().is_some_and(|a| !a.is_empty()), "{k}: nothing kept");
    }
    let mut rolled = v.clone();
    rolled["next"]["roll"] = json!(vec![1u8; v["targets"].as_array().unwrap().len()]);
    let roll = || {
        let mut r = run(&rolled);
        assert!(r["next"]["roll"].get("error").is_none() && r["next"]["roll"]["y"].is_number(), "{r}");
        for k in ["ms", "prof"] {
            r["next"]["roll"].as_object_mut().unwrap().remove(k);
        }
        r
    };
    let roll0 = roll();
    let empty = json!({"v": STATE_VERSION, "bests": [], "seen": [], "solve": [], "rolls": []});
    assert_eq!(state_in(&empty)["ok"], true);
    assert_eq!(state_in(&st)["ok"], true);
    assert_eq!(state_out(), out, "the state did not round-trip");
    assert_eq!(roll(), roll0, "a roll after the round trip");

    // ---- malformed stores: refused whole, the kept work as it was ----
    let kept = state_out();
    let hex = |x: f64| Value::String(format!("{:016x}", x.to_bits()));
    let refused = |what: &str, f: &dyn Fn(&mut Value)| {
        let mut bad = st.clone();
        f(&mut bad);
        let r = state_in(&bad);
        assert!(r["error"].is_string(), "{what}: {r}");
        assert_eq!(state_out(), kept, "{what}: something was installed");
    };
    let big = json!(1u64 << 40);
    refused("seen: a class count past STATE_MAX_N", &|s| s["seen"][0][1] = big.clone());
    refused("seen: a class index past its count", &|s| {
        let n = s["seen"][0][1].clone();
        s["seen"][0][2][0][0] = n;
    });
    refused("seen: a NaN value", &|s| s["seen"][0][2][0][1][0][1] = hex(f64::NAN));
    refused("seen: a chance above 1", &|s| s["seen"][0][2][0][1][0][0][0] = hex(1.5));
    refused("seen: a deck hint chance below 0", &|s| s["seen"][0][2][0][1][0][2][3][0] = hex(-0.25));
    refused("seen: seven cards", &|s| s["seen"][0][2][0][1][0][2][0] = json!([0, 1, 2, 3, 4, 5, 6]));
    refused("seen: a deck truncated", &|s| {
        s["seen"][0][2][0][1][0][2].as_array_mut().unwrap().pop();
    });
    refused("seen: white and gold chances of different lengths", &|s| {
        s["seen"][0][2][0][1][0][2][4].as_array_mut().unwrap().pop();
    });
    refused("bests: a uma count past STATE_MAX_N", &|s| s["bests"][0]["n"] = big.clone());
    refused("solve: a list count past STATE_MAX_N", &|s| s["solve"][0][1]["refs"][0] = big.clone());
    refused("solve: a NaN bound", &|s| s["solve"][0][1]["next_bound"] = hex(f64::NAN));
    refused("rolls: 64 targets", &|s| s["rolls"][0]["sd"]["n"][2] = json!(64));
    refused("rolls: 11 uncertain targets", &|s| s["rolls"][0]["sd"]["n"][0] = json!(11));
    refused("rolls: a guaranteed count that wraps the sum", &|s| s["rolls"][0]["sd"]["n"][1] = json!(u64::MAX));
    refused("rolls: uncertain and guaranteed not the targets", &|s| {
        let nf = s["rolls"][0]["sd"]["n"][1].as_u64().unwrap();
        s["rolls"][0]["sd"]["n"][1] = json!(nf + 1);
    });
    refused("rolls: an uncertain target past the targets", &|s| {
        let ntar = s["rolls"][0]["sd"]["n"][2].clone();
        let u = s["rolls"][0]["sd"]["uncv"].as_array_mut().unwrap();
        *u.last_mut().expect("an uncertain target") = ntar;
    });
    refused("rolls: guaranteed masks that are not the targets'", &|s| s["rolls"][0]["sd"]["fmm"][0] = json!(12345));
    refused("rolls: landed-set chances truncated", &|s| {
        s["rolls"][0]["sd"]["os"].as_array_mut().unwrap().pop();
    });
    refused("rolls: a NaN star share", &|s| s["rolls"][0]["sd"]["w"][0] = hex(f64::NAN));
    refused("rolls: a candidate's step costs not one per candidate", &|s| {
        s["rolls"][0]["base"]["c1"].as_array_mut().unwrap().push(hex(1.0));
    });
    refused("rolls: her trio side past the candidates", &|s| {
        s["rolls"][0]["her"].as_array_mut().unwrap().push(json!([1_000_000, false, hex(1.0)]));
    });
    refused("rolls: a lifted candidate past the candidates", &|s| {
        let act = s["rolls"][0]["sd"]["act"].as_array_mut().unwrap();
        act.push(json!([1_000_000, null, hex(0.5), hex(0.5)]));
    });
    refused("rolls: a slot out of order", &|s| {
        let act = s["rolls"][0]["sd"]["act"].as_array_mut().unwrap();
        act.push(json!([0, 1_000_000, hex(0.5), hex(0.5)]));
    });
    refused("rolls: a trio past the candidates", &|s| {
        s["rolls"][0]["base"]["combos"].as_array_mut().unwrap().push(json!([1_000_000, 0, hex(1.0), false, false]));
    });
    refused("rolls: an anchor's chances truncated", &|s| {
        let anc = s["rolls"][0]["anc"].as_array_mut().unwrap();
        let a = anc.iter_mut().find(|a| !a.is_null()).expect("an anchor");
        a[1].as_array_mut().unwrap().pop();
    });

    // ---- stores that parse but do not fit the setup they meet: not used there ----
    let seen_used = |st: &Value| {
        assert_eq!(state_in(st)["ok"], true);
        let used = std::rc::Rc::new(std::cell::Cell::new(false));
        let u = std::rc::Rc::clone(&used);
        ENV_HOOK.with(|c| {
            *c.borrow_mut() =
                Some(Box::new(move |env: &mut Env, _: &Pool| u.set(env.state.seen.iter().any(|r| !r.is_empty()))));
        });
        let mut job = v.clone();
        job.as_object_mut().unwrap().remove("next");
        run(&job);
        used.get()
    };
    assert!(seen_used(&st), "the stored proofs were not used");
    let mut bad = st.clone();
    bad["seen"][0][2][0][1][0][2][0][0] = json!(1_000_000);
    assert!(!seen_used(&bad), "a deck entry past the deck space was used");
    let mut bad = st.clone();
    bad["seen"][0][2][0][1][0][0].as_array_mut().unwrap().pop();
    assert!(!seen_used(&bad), "chances for fewer targets were used");
    // a kept main search whose career names a trainee past the umas: searched again, the same plan
    let mut bad = st.clone();
    bad["solve"].as_array_mut().unwrap().iter_mut().for_each(|e| {
        if let Some(f) = e[1]["found"].as_array_mut().and_then(|f| f.first_mut()) {
            f[0] = json!(1_000_000);
        }
    });
    assert_eq!(state_in(&bad)["ok"], true);
    let again = run(&v);
    assert!(again.get("error").is_none(), "{}", again["error"]);
    assert_eq!((&again["next"]["choice"], &again["results"]), (&plan["next"]["choice"], &plan["results"]));
}
