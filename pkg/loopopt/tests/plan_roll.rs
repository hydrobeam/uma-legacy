//! A roll of the plan's pick (`next.roll`, docs/SOLVER.md *Next career*): the careers her veteran saves at one outcome,
//! worked out from the plan's kept roll state. This is a self-pin, not a check of the values: the full outcome table it
//! was first compared with is gone from the output, and `fixtures/plan_roll.json` (the val/plan jobs' rolls at the
//! all-zero and all-max outcomes and at samples of y >= 0.5 and y < 0.5, as her stars per job target, each with its y)
//! is re-taken from the solver's own rolls whenever results move on purpose: `BLESS=1 cargo test --release --test
//! plan_roll rolls_equal` writes each roll's new y in place and names every one that changed (docs/BUILD.md, *Tests*;
//! the other test reads the fixture, so it runs after). It detects
//! a roll that moves, and keeps the samples in their y bands. Also: a roll carried by another job (`roll`, as the
//! page's Reroll sends it), a roll with no plan kept (the plan runs first), and a roll in a new process after
//! `state_out` / `state_in` (the stored roll state, no plan).
mod common;
use common::{State, TempFile, loopopt, run_with, val_job};
use serde_json::{Value, json};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/plan_roll.json");

/// one process running `jobs` in turn from the era's data folder (one output each), with `state`'s files
fn run(era: &str, tag: &str, jobs: &[Value], state: State) -> Vec<Value> {
    run_with(loopopt(era, state), &format!("plan-roll-{tag}"), jobs).outputs
}
fn job(name: &str) -> Value {
    let mut j = val_job(&format!("plan/{name}.json"));
    j["job"]["career"]["no_reports"] = json!(true);
    j
}
fn with_roll(plan: &Value, stars: &Value) -> Value {
    let mut j = plan.clone();
    j["job"]["career"]["next"]["roll"] = stars.clone();
    j
}
fn fixture() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap()
}
/// a y's bits (None where it is not a number)
fn bits(y: &Value) -> Option<u64> {
    y.as_f64().map(f64::to_bits)
}
/// a roll's answer against the table's y at that outcome, to the bit
fn same(r: &Value, want: &[Value], what: &str) {
    let y = &want[1];
    assert!(bits(&r["y"]) == bits(y), "{what}: stars {}: roll {r} vs table y {y}", want[0]);
}
/// Whether this run re-takes the fixture (`BLESS=1`) instead of checking against it.
fn blessing() -> bool {
    std::env::var_os("BLESS").is_some_and(|bless| bless == "1")
}

#[test]
fn rolls_equal_their_pinned_values() {
    let mut fx = fixture();
    let bless = blessing();
    let mut changed = 0;
    for (name, e) in fx.as_object_mut().unwrap() {
        let era = e["era"].as_str().unwrap().to_owned();
        let rolls = e["rolls"].as_array_mut().unwrap();
        let plan = job(name);
        let jobs: Vec<Value> =
            std::iter::once(plan.clone()).chain(rolls.iter().map(|r| with_roll(&plan, &r[0]))).collect();
        let out = run(&era, name, &jobs, State::NONE);
        assert!(
            out[0]["next"]["outcomes"].is_object() && out[0]["next"]["outcomes"].get("y").is_none(),
            "{name}: the plan's outcomes"
        );
        for (r, w) in out[1..].iter().zip(rolls) {
            let ro = &r["next"]["roll"];
            assert!(
                ro["planned"].is_null() && r["next"].get("options").is_none(),
                "{name}: a roll of a kept plan planned again: {}",
                r.to_string().chars().take(200).collect::<String>()
            );
            if !bless {
                same(ro, w.as_array().unwrap(), name);
            } else if bits(&ro["y"]) != bits(&w[1]) {
                // (a sample whose y crossed 0.5 left its band: the fixture keeps one of each, so pick another there)
                let band = |y: &Value| y.as_f64().is_some_and(|y| y >= 0.5);
                let crossed = if band(&ro["y"]) == band(&w[1]) { "" } else { ", crossed y = 0.5" };
                eprintln!("{name}: stars {}: y {} -> {}{crossed}", w[0], w[1], ro["y"]);
                w[1] = ro["y"].clone();
                changed += 1;
            }
        }
    }
    if bless {
        std::fs::write(FIXTURE, serde_json::to_string(&fx).unwrap()).unwrap();
        eprintln!("BLESS: wrote {FIXTURE}, {changed} rolls changed");
    }
}

#[test]
fn rolls_carried_cold_and_after_a_reload() {
    let fx = fixture();
    let rolls = fx["p16"]["rolls"].as_array().unwrap();
    let (w0, w1) = (rolls[0].as_array().unwrap(), rolls[rolls.len() - 1].as_array().unwrap());
    let plan = job("p16");
    // the page's Reroll: its search job (her veteran fixed as a parent) carries the roll of the plan on show
    let mut search = plan.clone();
    let career = search["job"]["career"].as_object_mut().unwrap();
    career.remove("next");
    career.insert("compare".into(), json!(true));
    // (the carried plan job as a job's part: no command)
    let mut plan_part = plan["job"].clone();
    plan_part.as_object_mut().unwrap().remove("cmd");
    let carried = |w: &[Value]| {
        let mut j = search.clone();
        j["job"]["roll"] = json!({"job": plan_part, "stars": w[0]});
        j
    };
    let st = TempFile::new("plan-roll-state");
    // cold: no plan kept, so the carried roll's plan runs first (`planned`), then a roll of it is answered from its state
    let out =
        run("global", "cold", &[carried(w0), with_roll(&plan, &w1[0])], State { load: None, save: Some(st.path()) });
    assert!(
        out[0]["results"].is_array() && out[0]["roll"]["planned"] == json!(true),
        "the carried roll: {}",
        out[0]["roll"]
    );
    same(&out[0]["roll"], w0, "carried, cold");
    assert!(
        out[1]["next"]["roll"]["planned"].is_null(),
        "the second roll planned again: {}",
        out[1].to_string().chars().take(300).collect::<String>()
    );
    same(&out[1]["next"]["roll"], w1, "kept");
    // a new process with the stored state: answered from the stored roll state, no plan
    let out =
        run("global", "reload", &[carried(w1), with_roll(&plan, &w0[0])], State { load: Some(st.path()), save: None });
    assert!(
        out[0]["roll"]["planned"].is_null() && out[1]["next"]["roll"]["planned"].is_null(),
        "a roll planned again after the reload"
    );
    same(&out[0]["roll"], w1, "carried, after the reload");
    same(&out[1]["next"]["roll"], w0, "after the reload");
}
