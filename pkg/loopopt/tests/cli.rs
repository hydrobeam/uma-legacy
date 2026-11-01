//! The command line's errors and its `constants` subcommand: a file it cannot read, a job without a known mode or a
//! missing argument exits 2 with a message (no panic), and `constants` prints the model constants as one JSON object.
use std::process::{Command, Output};

/// `loopopt ARGS` run from the Global data folder.
fn loopopt(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_loopopt"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .args(args)
        .output()
        .expect("the binary runs")
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
    let job = std::env::temp_dir().join(format!("uma-cli-mode-{}.json", std::process::id()));
    std::fs::write(&job, r#"{"mode": "decks"}"#).expect("a temporary file");
    let output = loopopt(&["data.json", job.to_str().expect("a UTF-8 path")]);
    std::fs::remove_file(&job).expect("the temporary file goes");
    refused(&output, r#""mode" must be "deck" or "career""#);
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
            "default_star_mix",
            "exact_builders_targets",
            "lam_decay"
        ],
        "the keys scripts read"
    );
    let mix: f64 = constants["default_star_mix"]
        .as_array()
        .expect("three shares")
        .iter()
        .filter_map(serde_json::Value::as_f64)
        .sum();
    assert!((mix - 1.0).abs() < 1e-12, "the star mix sums to 1: {mix}");
}
