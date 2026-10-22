//! A roll of the plan's pick (`next.roll`, docs/SOLVER.md *Next career*): the careers her veteran saves at one outcome
//! and its bound, worked out from the plan's kept roll state, equal to the values the plan's full outcome table held at
//! that outcome before the table was dropped from the output (`fixtures/plan_roll.json`: `outcomes.y` and `y_hi` of the
//! val/plan jobs' outputs at 5834a65 with the goal races' streak shift (lib.rs `scored_position`) and the lineage
//! schedule's race weights from `FILL_EPS` (career/schedule.rs `schedule`) patched in, at the all-zero and all-max
//! outcomes and samples of y >= 0.5, of y < 0.5 <= y_hi and of y_hi < 0.5, as her stars per job target). Also: a roll
//! carried by another job (`roll`, as the page's Reroll sends it), a roll with no plan kept (the plan runs first), and
//! a roll in a new process after `state_out` / `state_in` (the stored roll state, no plan).
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// one process running `jobs` in turn from the era's data folder (one output line each), the state in from / out to the
/// given files
fn run(era: &str, jobs: &[&Path], state_in: Option<&Path>, state_out: Option<&Path>) -> Vec<Value> {
    let mut c = Command::new(env!("CARGO_BIN_EXE_loopopt"));
    c.current_dir(format!("{}/data/{era}", env!("CARGO_MANIFEST_DIR"))).arg("data.json").args(jobs);
    c.env_remove("LOOPOPT_STATE_IN").env_remove("LOOPOPT_STATE_OUT");
    if let Some(p) = state_in {
        c.env("LOOPOPT_STATE_IN", p);
    }
    if let Some(p) = state_out {
        c.env("LOOPOPT_STATE_OUT", p);
    }
    let output = c.output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}
fn job(name: &str) -> Value {
    let mut j: Value = serde_json::from_str(
        &fs::read_to_string(format!("{}/../../val/plan/{name}.json", env!("CARGO_MANIFEST_DIR"))).unwrap(),
    )
    .unwrap();
    j["no_reports"] = json!(true);
    j
}
fn with_roll(plan: &Value, stars: &Value) -> Value {
    let mut j = plan.clone();
    j["next"]["roll"] = stars.clone();
    j
}
fn tmp(tag: &str, i: usize) -> PathBuf {
    std::env::temp_dir().join(format!("uma-plan-roll-{}-{tag}-{i}.json", std::process::id()))
}
fn write(tag: &str, jobs: &[Value]) -> Vec<PathBuf> {
    jobs.iter()
        .enumerate()
        .map(|(i, j)| {
            let p = tmp(tag, i);
            fs::write(&p, j.to_string()).unwrap();
            p
        })
        .collect()
}
fn clean(ps: &[PathBuf]) {
    for p in ps {
        fs::remove_file(p).unwrap();
    }
}
/// a roll's answer against the table's (y, y_hi) at that outcome, to the bit
fn same(r: &Value, want: &[Value], what: &str) {
    let (y, hi) = (&want[1], &want[2]);
    assert!(
        r["y"].as_f64().map(f64::to_bits) == y.as_f64().map(f64::to_bits)
            && r["y_hi"].as_f64().map(f64::to_bits) == hi.as_f64().map(f64::to_bits)
            && r["y_hi"].is_null() == hi.is_null(),
        "{what}: stars {}: roll {r} vs table y {y} y_hi {hi}",
        want[0]
    );
}

#[test]
fn rolls_equal_the_full_tables() {
    let fx: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/plan_roll.json")).unwrap(),
    )
    .unwrap();
    for (name, e) in fx.as_object().unwrap() {
        let (era, rolls) = (e["era"].as_str().unwrap(), e["rolls"].as_array().unwrap());
        let plan = job(name);
        let mut jobs = vec![plan.clone()];
        for r in rolls {
            jobs.push(with_roll(&plan, &r[0]));
        }
        let ps = write(name, &jobs);
        let out = run(era, &ps.iter().map(std::path::PathBuf::as_path).collect::<Vec<_>>(), None, None);
        clean(&ps);
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
            same(ro, w.as_array().unwrap(), name);
        }
    }
}

#[test]
fn rolls_carried_cold_and_after_a_reload() {
    let fx: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/plan_roll.json")).unwrap(),
    )
    .unwrap();
    let rolls = fx["p16"]["rolls"].as_array().unwrap();
    let (w0, w1) = (rolls[0].as_array().unwrap(), rolls[rolls.len() - 1].as_array().unwrap());
    let plan = job("p16");
    // the page's Reroll: its search job (her veteran fixed as a parent) carries the roll of the plan on show
    let mut search = plan.clone();
    let o = search.as_object_mut().unwrap();
    o.remove("next");
    o.insert("compare".into(), json!(true));
    let carried = |w: &[Value]| {
        let mut j = search.clone();
        j["roll"] = json!({"job": plan, "stars": w[0]});
        j
    };
    let st = tmp("state", 0);
    // cold: no plan kept, so the carried roll's plan runs first (`planned`), then a roll of it is answered from its state
    let ps = write("cold", &[carried(w0), with_roll(&plan, &w1[0])]);
    let out = run("global", &ps.iter().map(std::path::PathBuf::as_path).collect::<Vec<_>>(), None, Some(&st));
    clean(&ps);
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
    let ps = write("reload", &[carried(w1), with_roll(&plan, &w0[0])]);
    let out = run("global", &ps.iter().map(std::path::PathBuf::as_path).collect::<Vec<_>>(), Some(&st), None);
    clean(&ps);
    fs::remove_file(&st).unwrap();
    assert!(
        out[0]["roll"]["planned"].is_null() && out[1]["next"]["roll"]["planned"].is_null(),
        "a roll planned again after the reload"
    );
    same(&out[0]["roll"], w1, "carried, after the reload");
    same(&out[1]["next"]["roll"], w0, "after the reload");
}
