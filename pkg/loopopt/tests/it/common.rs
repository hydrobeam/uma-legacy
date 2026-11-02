//! The integration tests' harness: the solver binary run on jobs from an era's data folder, one process per run, the
//! jobs in turn as the page's worker runs them (its kept state carried between them). Every run clears the kept-state
//! variables (`LOOPOPT_STATE_IN`, `LOOPOPT_STATE_OUT`) of the shell running the tests and sets only the files it names.

use serde_json::Value;
use std::borrow::Borrow;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The kept state's files for a run: read in from `load` (`LOOPOPT_STATE_IN`), written out to `save`
/// (`LOOPOPT_STATE_OUT`).
#[derive(Clone, Copy, Default)]
pub struct State<'files> {
    pub load: Option<&'files Path>,
    pub save: Option<&'files Path>,
}

impl State<'_> {
    /// No kept state in or out.
    pub const NONE: Self = Self { load: None, save: None };
}

/// The solver run from `era`'s data folder (`data/global`, `data/jp`), with `state`'s files and no other kept state.
pub fn loopopt(era: &str, state: State) -> Command {
    in_era(Command::new(env!("CARGO_BIN_EXE_loopopt")), era, state)
}

fn in_era(mut command: Command, era: &str, state: State) -> Command {
    command.current_dir(format!("{}/data/{era}", env!("CARGO_MANIFEST_DIR")));
    command.env_remove("LOOPOPT_STATE_IN").env_remove("LOOPOPT_STATE_OUT");
    if let Some(path) = state.load {
        command.env("LOOPOPT_STATE_IN", path);
    }
    if let Some(path) = state.save {
        command.env("LOOPOPT_STATE_OUT", path);
    }
    command
}

/// A file in the temporary folder, removed when dropped (a test that panics leaves none behind). Its name holds the
/// process id and a number unique within the process, so tests running at once never share one.
pub struct TempFile(PathBuf);

impl TempFile {
    pub fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let number = NEXT.fetch_add(1, Ordering::Relaxed);
        Self(std::env::temp_dir().join(format!("uma-test-{tag}-{}-{number}.json", std::process::id())))
    }

    /// A new file holding `job`.
    pub fn job(tag: &str, job: &Value) -> Self {
        let file = Self::new(tag);
        std::fs::write(&file.0, job.to_string()).expect("a temporary job file");
        file
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        // (a state file a run was not asked to write may not exist)
        let _ = std::fs::remove_file(&self.0);
    }
}

/// One run's outputs, one per job in order, and what it wrote to stderr.
pub struct Ran {
    pub outputs: Vec<Value>,
    pub stderr: String,
}

/// `command` (from `loopopt`) running `jobs` in turn on data.json; panics unless it succeeds.
pub fn run_with<J: Borrow<Value>>(mut command: Command, tag: &str, jobs: &[J]) -> Ran {
    let files: Vec<TempFile> = jobs.iter().map(|job| TempFile::job(tag, job.borrow())).collect();
    let output = command.arg("data.json").args(files.iter().map(TempFile::path)).output().expect("the solver runs");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{tag}: {stderr}");
    let outputs = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("one JSON output per line"))
        .collect();
    Ran { outputs, stderr }
}

/// `jobs` run in turn in one process from the Global data, with no kept state: their outputs.
pub fn run_jobs<J: Borrow<Value>>(tag: &str, jobs: &[J]) -> Vec<Value> {
    run_with(loopopt("global", State::NONE), tag, jobs).outputs
}

/// `job` run alone from the Global data, with no kept state: its output.
pub fn run_job(tag: &str, job: &Value) -> Value {
    let outputs = run_jobs(tag, &[job]);
    assert_eq!(outputs.len(), 1, "{tag}: one output");
    outputs.into_iter().next().expect("one output")
}

/// The job file at `path` under the repository's val/ folder ("plan/p16.json", "jobs/pal_U.json"): `files` and the
/// `job` (`cmd`, `skills`, `deck`, `career`).
pub fn val_job(path: &str) -> Value {
    let full = format!("{}/../../val/{path}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&full).expect("a val job")).expect("a JSON job")
}

/// A plan job's roll: answered from the plan the worker keeps (`kind` "roll"), else the plan's own (`next.roll`).
pub fn roll_of(output: &Value) -> &Value {
    if output["kind"] == "roll" { &output["roll"] } else { &output["next"]["roll"] }
}

/// A plan job's check, likewise (`kind` "check", else `next.robust`).
pub fn check_of(output: &Value) -> &Value {
    if output["kind"] == "check" { &output["robust"] } else { &output["next"]["robust"] }
}

/// `value` without the keys `drop` names, at any depth.
fn without(value: &Value, drop: &impl Fn(&str) -> bool) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .filter(|(key, _)| !drop(key))
                .map(|(key, field)| (key.clone(), without(field, drop)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(|item| without(item, drop)).collect()),
        other => other.clone(),
    }
}

/// An output without its wall clocks (`timing_s`, `*_ms`), as val/same.py compares it.
pub fn without_clocks(value: &Value) -> Value {
    without(value, &|key| key == "timing_s" || key.ends_with("_ms"))
}

/// An output without its wall clocks and work counters (`work`, and a plan's counts of its own searches,
/// `umas_searched`, `umas_certified`: a plan of a pool whose searches it kept searches less).
pub fn without_work(value: &Value) -> Value {
    without(value, &|key| {
        ["timing_s", "work", "umas_searched", "umas_certified"].contains(&key) || key.ends_with("_ms")
    })
}
