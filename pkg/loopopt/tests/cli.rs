//! The command line's errors and its `constants` subcommand: a file it cannot read, a job file that is not one or a
//! missing argument exits 2 with a message (no panic), and `constants` prints the model constants as one JSON object.
mod common;
use common::State;
use std::process::Output;

/// `loopopt ARGS` run from the Global data folder.
fn loopopt(args: &[&str]) -> Output {
    common::loopopt("global", State::NONE).args(args).output().expect("the binary runs")
}

/// The command failed with exit 2, its stderr holding `said` and no panic.
fn refused(output: &Output, said: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "exit code; stderr: {stderr}");
    assert!(stderr.contains(said), "stderr says {said:?}: {stderr}");
    assert!(!stderr.contains("panicked"), "no panic: {stderr}");
}

#[test]
fn errors_name_the_file_and_exit_2() {
    refused(&loopopt(&["data.json", "no_such_job.json"]), "cannot read no_such_job.json");
    refused(&loopopt(&["no_such_data.json", "../../../../val/jobs/pal_U.json"]), "cannot read no_such_data.json");
    refused(&loopopt(&["pbtable", "no_such_data.json"]), "cannot read no_such_data.json");
    refused(&loopopt(&["data.json"]), "required arguments");
    let files = serde_json::json!({"skills": "web_skills.json", "cards": "web_cards.json"});
    let job = common::TempFile::job("cli-cmd", &serde_json::json!({"files": files, "job": {"cmd": "decks"}}));
    let output = loopopt(&["data.json", job.path().to_str().expect("a UTF-8 path")]);
    refused(&output, "unknown variant `decks`");
    let job = common::TempFile::job("cli-flat", &serde_json::json!({"mode": "deck", "targets": []}));
    let output = loopopt(&["data.json", job.path().to_str().expect("a UTF-8 path")]);
    refused(&output, "unknown field `mode`");
}

#[test]
fn constants_prints_the_model_constants() {
    let output = loopopt(&["constants"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let constants: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one JSON object");
    let keys: Vec<&str> = constants.as_object().expect("an object").keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "ceil_share",
            "chain_a",
            "chain_b",
            "chain_n",
            "chain_stop_share",
            "exact_builders_targets",
            "gen_gold",
            "gen_white",
            "insp_per_star",
            "lam_decay",
            "star_mix"
        ],
        "the keys scripts read"
    );
    for class in ["below_ss", "ss_and_up"] {
        let shares = constants["star_mix"][class].as_array().expect("three shares");
        let mix: f64 = shares.iter().filter_map(serde_json::Value::as_f64).sum();
        assert!(shares.len() == 3 && (mix - 1.0).abs() < 1e-12, "{class}: the star mix sums to 1: {mix}");
    }
}
