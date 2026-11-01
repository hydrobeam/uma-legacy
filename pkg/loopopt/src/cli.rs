//! The native command line (`loopopt`, clap's derive API): runs deck and career job files, prints a data.json's
//! pairwise-ceiling table (`pbtable`) or the model constants scripts read (`constants`). A file it cannot read or parse
//! is an error naming the file, and the process exits 2. Solver algorithms and the wasm interface live in their own
//! modules; the wasm build leaves this one (and clap) out.
use crate::career::reply::CareerReply;
use crate::career::state::KeptState;
use crate::data::{Data, DataRaw, PbRaw, load_raw, pb_fingerprint};
use crate::deck::CardsRaw;
use crate::loop_planner::schedules::pair_ceilings;
use crate::request::{CareerRequest, DeckRequest, JobFile, Request};
use crate::uma::UmaIndex;
use crate::{Instant, skills};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The legacy loop and deck planner's solver: `loopopt DATA JOB...` runs job files on an era's data.json.
#[derive(Parser)]
#[command(name = "loopopt", args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// the era's data.json
    #[arg(required = true)]
    data: Option<PathBuf>,
    /// job files: one deck job, or one or more career jobs (one output line each; they share the first job's skills,
    /// cards and kept work)
    #[arg(required = true)]
    jobs: Vec<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    /// print a data.json's exact pairwise-ceiling table, computed fresh (its "pb" field: ./build.sh pb merges it)
    Pbtable {
        /// the era's data.json
        data: PathBuf,
    },
    /// print the model constants scripts read, as JSON (constants.rs `SolverConstants`)
    Constants,
}

/// Why the command line could not run.
struct CliError(String);

/// A file's text.
fn read(path: &Path) -> Result<String, CliError> {
    std::fs::read_to_string(path).map_err(|e| CliError(format!("cannot read {}: {e}", path.display())))
}

/// A file's JSON, parsed as `T`.
fn parse<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, CliError> {
    serde_json::from_str(&read(path)?).map_err(|e| CliError(format!("{}: {e}", path.display())))
}

/// A value as one line of JSON (a reply or table of the solver's own types, which always serialise).
fn json_line(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("the solver's replies serialise")
}

/// The command line: runs it, or says why it could not and exits 2.
pub(crate) fn main() -> ExitCode {
    let _report = CliBoundReport;
    let cli = Cli::parse();
    let outcome = match cli.command {
        Some(Command::Pbtable { data }) => pb_table(&data).map(|table| println!("{}", json_line(&table))),
        Some(Command::Constants) => {
            println!("{}", json_line(&crate::constants::SolverConstants::now()));
            Ok(())
        }
        None => run_jobs(cli.data.as_deref().expect("clap requires DATA"), &cli.jobs),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError(why)) => {
            eprintln!("error: {why}");
            ExitCode::from(2)
        }
    }
}

/// What every job of a run reads.
struct CliInputs<'data> {
    data: &'data Data,
    skills: &'data skills::SkillsRaw,
    cards: &'data CardsRaw,
}

/// The job files on `data_path`'s data: one deck job, or career jobs sharing the first one's files and the kept work.
fn run_jobs(data_path: &Path, job_paths: &[PathBuf]) -> Result<(), CliError> {
    let [first_path, ..] = job_paths else { return Err(CliError("no job file".to_owned())) };
    // the first job, read (and checked) before the data or any file it names
    let first: JobFile = parse(first_path)?;
    let data = load_raw(parse::<DataRaw>(data_path)?);
    let skills: skills::SkillsRaw = parse(&first.files.skills)?;
    let cards: CardsRaw = parse(&first.files.cards)?;
    let inputs = CliInputs { data: &data, skills: &skills, cards: &cards };
    match first.job {
        Request::Deck(mut job) => {
            if let Some(path) = &first.files.events {
                cli_events(path, &skills)?.add_to(&mut job.deck);
            }
            print_deck_job(&inputs, &job);
        }
        Request::Career(_) => {
            // the career planner's work kept between the run's jobs (and across runs: LOOPOPT_STATE_IN / _OUT)
            let mut kept = KeptState::default();
            restore_cli_career_state(&mut kept)?;
            // several career jobs share the first job's skills and cards and the worker's caches; each job is read with
            // its own events
            for path in job_paths {
                let JobFile { files, job } = parse(path)?;
                let Request::Career(mut job) = job else {
                    return Err(CliError(format!("{}: a career run's jobs are career jobs", path.display())));
                };
                if let Some(events) = &files.events {
                    add_cli_events(&mut job, &cli_events(events, &skills)?);
                }
                print_career_job(&inputs, &job, &mut kept);
            }
            save_cli_career_state(&kept)?;
        }
        Request::StateOut | Request::StateIn { .. } => {
            return Err(CliError(format!(
                "{}: a job file runs a deck or career job (the kept work is LOOPOPT_STATE_IN / _OUT)",
                first_path.display()
            )));
        }
    }
    Ok(())
}

/// A data.json's full pairwise-ceiling table, as stored in its "pb" field (computed fresh, ignoring any table).
fn pb_table(path: &Path) -> Result<PbRaw, CliError> {
    let raw: DataRaw = parse(path)?;
    let fingerprint = pb_fingerprint(&raw);
    let mut data = load_raw(raw);
    data.pair_ceilings = None;
    let n = data.umas.len();
    let roster: Vec<UmaIndex> = data.umas.indices().collect();
    let ceilings = pair_ceilings(&data, &roster);
    let rows: Vec<String> =
        (0..n).map(|a| (a + 1..n).map(|b| crate::hex::f64_hex(ceilings[UmaIndex(a)][UmaIndex(b)])).collect()).collect();
    Ok(PbRaw { fingerprint, rows })
}

/// Emits the enabled bound diagnostics on normal return or panic unwind, `pbtable` included.
struct CliBoundReport;

impl Drop for CliBoundReport {
    fn drop(&mut self) {
        let checks = crate::checks::get();
        if checks.sb_check {
            crate::checks::SBCHK.with(|counts| {
                let counts = counts.borrow();
                eprintln!(
                    "SBCHK decks {} max value/exact {:.6} min {:.9} below {} | leaves {} node violations {}",
                    counts.decks, counts.max_ratio, counts.min_ratio, counts.below, counts.leaves, counts.violations
                );
            });
        }
        if checks.sbound_report || checks.sb_check {
            crate::checks::SBN.with(|counts| {
                let counts = counts.borrow();
                eprintln!(
                    "SBN cfgs cut {} bound evals {} nodes cut {}",
                    counts.configurations_cut, counts.evaluations, counts.nodes_cut
                );
            });
        }
        if checks.rb_report {
            crate::checks::RBCHK.with(|counts| {
                let counts = counts.borrow();
                eprintln!(
                    "RBCHK leaves {} tests {} violations {} max value/bound {:.9}",
                    counts.leaves, counts.tests, counts.violations, counts.max_ratio
                );
            });
        }
        if checks.cf_report {
            crate::checks::CFCHK.with(|counts| {
                let counts = counts.borrow();
                eprintln!("CFCHK searches {} capped below cap-free {} cap-free below capped {} unproven {} | leaves {} violations {} | scorings {} violations {} max exact/bound {:.9}", counts.searches, counts.capped_below, counts.cap_free_below, counts.unproven, counts.leaves, counts.leaf_violations, counts.scorings, counts.scoring_violations, counts.max_ratio);
            });
        }
    }
}

/// A job's card-event file (`files.events`), as the lists it adds to a job.
fn cli_events(path: &Path, skills: &skills::SkillsRaw) -> Result<crate::deck::sources::EventLists, CliError> {
    Ok(crate::deck::sources::event_lists(skills, parse(path)?))
}

/// A career job's card events, added to it and to a carried roll's plan job (the carried job must match the enriched
/// job that filled the worker's kept state).
fn add_cli_events(job: &mut CareerRequest, events: &crate::deck::sources::EventLists) {
    events.add_to(&mut job.deck);
    if let Some(roll) = &mut job.roll {
        events.add_to(&mut roll.job.deck);
    }
}

/// A `state_out` reply as the command line saves it: the worker reads (and checks) its `state`.
#[derive(Deserialize)]
struct SavedState {
    state: crate::career::state::StoredState,
}

/// Restores the career worker's kept work (`STATE_IN`) before any job runs, as the page does across reloads.
fn restore_cli_career_state(kept: &mut KeptState) -> Result<(), CliError> {
    if let Some(path) = &crate::checks::get().state_in {
        let saved: SavedState = parse(Path::new(path))?;
        eprintln!("state_in: {}", json_line(&crate::career::state::state_in(kept, &saved.state)));
    }
    Ok(())
}

/// Saves the career worker's kept work (`STATE_OUT`) once, after every job's output is printed.
fn save_cli_career_state(kept: &KeptState) -> Result<(), CliError> {
    if let Some(path) = &crate::checks::get().state_out {
        std::fs::write(path, json_line(&CareerReply::StateOut(crate::career::state::state_out(kept))))
            .map_err(|e| CliError(format!("cannot write {path}: {e}")))?;
    }
    Ok(())
}

/// Runs a deck job and prints its reply (`print_timed`).
fn print_deck_job(inputs: &CliInputs, job: &DeckRequest) {
    let started = Instant::now();
    let reply = crate::loop_planner::job::run_deck(inputs.data, inputs.skills, inputs.cards, job);
    print_timed(&reply, started);
}

/// Runs a career job on the run's kept work and prints its reply (`print_timed`).
fn print_career_job(inputs: &CliInputs, job: &CareerRequest, kept: &mut KeptState) {
    let started = Instant::now();
    let reply = crate::career::run_career_in(inputs.data, inputs.skills, inputs.cards, job, kept);
    print_timed(&reply, started);
}

/// Prints a reply, timed from `started` (the solver's dispatch only: file loading, event enrichment and output
/// serialisation stay outside): one JSON object, its keys sorted, with the solver's time beside its fields, replacing
/// a career reply's own `timing_s` (which counts from after the job was read).
fn print_timed(reply: &impl serde::Serialize, started: Instant) {
    let seconds = started.elapsed().as_secs_f64();
    let mut out = serde_json::to_value(reply).expect("the solver's replies serialise");
    out["timing_s"] = serde_json::Value::from(seconds);
    println!("{out}");
}
