//! Deck mode with three-uma rotations next to four-uma loops (`sizes`), on the shipped Global data.
use serde_json::{Value, json};
use std::{fs, process::Command};

fn run(job: &Value) -> Value {
    run_command(job, Command::new(env!("CARGO_BIN_EXE_loopopt")))
}

fn run_bounded(job: &Value) -> Value {
    // Kill the child on regression instead of leaving the test suite in an hours-long schedule search.
    let mut command = Command::new("timeout");
    command.args(["10s", env!("CARGO_BIN_EXE_loopopt")]);
    run_command(job, command)
}

fn run_command(job: &Value, mut command: Command) -> Value {
    let tag = job.to_string().bytes().fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b)));
    let path = std::env::temp_dir().join(format!("uma-rotations-{}-{tag}.json", std::process::id()));
    fs::write(&path, job.to_string()).unwrap();
    let output = command
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .args(["data.json", path.to_str().unwrap()])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}

/// The runner-up is much weaker than the best deck. A bound against only the runner-up used to schedule thousands
/// of loops that merely rediscovered the best deck; both multi-result jobs exceeded the timeout.
#[test]
fn fixed_pals_distinct_decks_finish() {
    let mut j = job(false, json!([3, 4]), json!([]));
    j["pals"] = json!(5);
    let single = run_bounded(&j);
    for count in [2, 3] {
        j["top_decks"] = json!(count);
        let result = run_bounded(&j);
        assert_eq!(result["top"].as_array().unwrap().len(), count);
        assert_eq!(result["top"][0], single["top"][0]);
        assert_eq!(result["proven"], json!(true));
        assert!(result["foursomes_solved"].as_u64().unwrap() < 10);
        let decks: Vec<String> = result["top"].as_array().unwrap().iter().map(|row| row["deck"].to_string()).collect();
        for (i, deck) in decks.iter().enumerate() {
            assert!(!decks[..i].contains(deck));
        }
    }
}

/// The complete deck space contains one identity: requesting three must return that deck with its best loop.
#[test]
fn fixed_pals_fewer_decks_than_requested() {
    let mut j = job(false, json!([3, 4]), json!([]));
    j["deck_size"] = json!(1);
    j["pals"] = json!(1);
    j["owned_cards"] = json!([30052]);
    j["lock_cards"] = json!([30052]);
    let single = run_bounded(&j);
    j["top_decks"] = json!(3);
    let result = run_bounded(&j);
    assert_eq!(result["top"], single["top"]);
    assert_eq!(result["top"].as_array().unwrap().len(), 1);
    assert_eq!(result["proven"], json!(true));

    // Two versions of Light Hello: the skyline omits the weaker deck, which must still be returned.
    j["owned_cards"] = json!([10083, 30052]);
    j["lock_cards"] = json!([]);
    for count in [2, 3] {
        j["top_decks"] = json!(count);
        let result = run_bounded(&j);
        assert_eq!(result["top"].as_array().unwrap().len(), 2);
        assert_eq!(result["top"][0], single["top"][0]);
        assert_eq!(result["proven"], json!(true));
    }
}
fn job(per_uma: bool, sizes: Value, require: Value) -> Value {
    let mut j: Value = serde_json::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/jobs/pal_U.json")).unwrap(),
    )
    .unwrap();
    j["per_uma"] = json!(per_uma);
    j["sizes"] = sizes;
    j["require"] = require;
    j
}
fn ids(l: &Value) -> Vec<u64> {
    l["members"].as_array().unwrap().iter().map(|m| m["id"].as_u64().unwrap()).collect()
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * a.abs().max(b.abs())
}

/// Per-uma decks: a rotation result has three members, three decks, three lineages (X = the trainee), and its odds are
/// the mean of its three careers'. The combined search's best is the better of the two single-size searches'.
#[test]
fn per_uma_rotation_result() {
    let both = run(&job(true, json!([3, 4]), json!([])));
    let three = run(&job(true, json!([3]), json!([])));
    let four = run(&job(true, json!([4]), json!([])));
    let p = |r: &Value| r["top"][0]["loop"]["p_all"].as_f64().unwrap();
    assert_eq!(p(&both), p(&three).max(p(&four)));
    for r in [&both, &three, &four] {
        assert_eq!(r["proven"], json!(true));
    }
    let l = &three["top"][0]["loop"];
    assert_eq!(ids(l).len(), 3);
    assert_eq!(three["top"][0]["decks"].as_array().unwrap().len(), 3);
    let lins = l["detail"]["lineages"].as_array().unwrap();
    assert_eq!(lins.len(), 3);
    for (i, x) in lins.iter().enumerate() {
        assert_eq!(x["t"], json!(i));
        assert_eq!(x["x"], json!(i));
    }
    let pt: Vec<f64> = l["per_trainee"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
    let pl: Vec<f64> = lins.iter().map(|x| x["p_all"].as_f64().unwrap()).collect();
    assert!(pt.iter().zip(&pl).all(|(a, b)| close(*a, *b)));
    assert!(close(pl.iter().sum::<f64>() / 3.0, p(&three)));
    assert_eq!(l["schedule"][0]["cells"].as_array().unwrap().len(), 3);
    assert_eq!(l["detail"]["pairs"].as_array().unwrap().len(), 3);
    // four required: loops only, the same answer as a four-only search with that requirement
    let req = json!(ids(&four["top"][0]["loop"]));
    let r4 = run(&job(true, json!([3, 4]), req));
    assert_eq!(ids(&r4["top"][0]["loop"]), ids(&four["top"][0]["loop"]));
    assert_eq!(p(&r4), p(&four));
}

/// One shared deck: with the rotation required, the deck is legal for its three members and the odds are the mean
/// of the three careers'.
#[test]
fn shared_rotation_result() {
    let r = run(&job(false, json!([3]), json!([1019, 1021, 1034])));
    assert_eq!(r["foursomes"], json!(1));
    let t = &r["top"][0];
    assert_eq!(ids(&t["loop"]), vec![1019, 1021, 1034]);
    let pl: Vec<f64> =
        t["loop"]["detail"]["lineages"].as_array().unwrap().iter().map(|x| x["p_all"].as_f64().unwrap()).collect();
    assert_eq!(pl.len(), 3);
    assert!(close(pl.iter().sum::<f64>() / 3.0, t["loop"]["p_all"].as_f64().unwrap()));
    assert!(t["decks"].is_null());
    assert!(t["deck"]["card_ids"].is_array());
}
