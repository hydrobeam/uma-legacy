//! The career search does not read the veterans' order: on val/jobs/career_U2 (twelve veterans in four characters,
//! three of a kind each, so careers of exactly equal P(all) on different veterans tie: its fifth result, Special Week
//! on legacies 1 and 10 or on 1 and 9, P(all) the same f64, where ties broken by legacy index took 9 in this order and
//! 10 reversed), the legacies reversed give the same results, in order, each on the same
//! veterans (by their records), with the same deck and P(all) to the bit. Ties in the search's ranking break by the
//! veterans' content (`Pool::content_ranks`), not their places.
use serde_json::Value;
use std::{fs, process::Command};

/// The job's results: per result its trainee, its parents' records (sorted: a pair is unordered), its deck and its
/// P(all)'s bits
fn results(name: &str, job: &Value) -> Vec<(String, Vec<String>, String, u64)> {
    let path = std::env::temp_dir().join(format!("uma-career-order-{name}-{}.json", std::process::id()));
    fs::write(&path, job.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_loopopt"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .arg("data.json")
        .arg(&path)
        .env_remove("LOOPOPT_STATE_IN")
        .env_remove("LOOPOPT_STATE_OUT")
        .output()
        .unwrap();
    fs::remove_file(&path).unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let text = String::from_utf8_lossy(&output.stdout);
    let out: Value = serde_json::from_str(text.lines().rev().find(|line| !line.trim().is_empty()).unwrap()).unwrap();
    let legacies = job["legacies"].as_array().unwrap();
    out["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| {
            let career = result["career"].as_str().unwrap();
            let mut parts = career.splitn(3, ':');
            let (trainee, parents, deck) = (parts.next().unwrap(), parts.next().unwrap(), parts.next().unwrap());
            let mut records: Vec<String> =
                parents.split(',').map(|index| legacies[index.parse::<usize>().unwrap()].to_string()).collect();
            records.sort();
            (trainee.to_owned(), records, deck.to_owned(), result["p_all"].as_f64().unwrap().to_bits())
        })
        .collect()
}

#[test]
fn the_veterans_order_changes_no_career() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/jobs/career_U2.json");
    let in_order: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let mut reversed = in_order.clone();
    reversed["legacies"].as_array_mut().unwrap().reverse();
    let (first, second) = (results("in-order", &in_order), results("reversed", &reversed));
    assert!(first.len() >= 5, "{} results", first.len());
    assert_eq!(first, second);
}
