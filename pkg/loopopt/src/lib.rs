//! # loopopt: Uma Musume legacy loop and deck optimizer
//!
//! The affinity core below optimizes race schedules for a loop's members by expected legacy affinity; the
//! skill-target model (`skills`), the deck search and the career planner (`career`) build on it. The CLI runs "deck"
//! and "career" jobs and `pbtable`; the page runs deck and career jobs through `web`.
//!
//! Objective, normalized to 4 runs:
//!     E = base + 3 * C * sum_{i<j} <P_i, P_j>
//! where P_i[r] = P(member i wins G1 race r at least once) = 1 - prod(1 - p) over her attempts at r,
//! and p = clamp(110 - cost(distance grade) - cost(surface grade) - PEN[scored position], 0, 100) / 100, the scored
//! position being the race's streak position, one less for a goal race (`data::scored_position`).
//! This scheduling objective retains the older half-grandparent normalization: C = 2,
//! base = 2*sum pair + 2*sum trio (`optimize::LoopProblem::new`). The skill model uses full grandparent rates.
//! Both weight every pair's shared wins uniformly, so the normalization preserves schedule rankings within a
//! structure; `skills::lins` derives the current model's weights for loops and rotations.
//!
//! Layers:
//! * `Uma::dp_exact` (in `schedule`): exact best response for one uma against fixed partners. Junior turns run in
//!   sequence; Classic turn t and Senior turn t+24 are decided jointly, so a race offered in both years
//!   (always 24 turns apart) is scored exactly. Checked against the MILP fixtures in `best_response_matches_milp`.
//! * `LoopProblem::ascent` (in `optimize`): best-response dynamics; stops when no member improves her weighted score by
//!   more than 1e-9.
//! * `LoopProblem::group_moves`: coalition deviations (2+ members forced onto the same race, or rest, at one turn),
//!   which escape the coordination equilibria that single-member moves can't leave.
//! * `optimize::optimize`: optimistic start + ascent + group moves + iterated local search (noisy best responses).
//! * `optimize::pair_ceilings`: exact pairwise ceilings PB(a, b), the bound the deck search screens structures with
//!   (stored in data.json by `loopopt pbtable`).
//!
//! Relocated-venue G1s are merged into their normal race by the exporter (export/export_rs.py).
//! See docs/SOLVER.md for the data format, job fields, validation and benchmarks.

mod uma;
use crate::cli::{
    CliBoundReport, Jobs, apply_cli_events, pb_table_json, print_cli_job, restore_cli_career_state,
    save_cli_career_state,
};
use crate::data::load;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
/// wasm32-unknown-unknown has no clock (Instant::now panics); timings read as zero there.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
struct Instant;
#[cfg(target_arch = "wasm32")]
impl Instant {
    const fn now() -> Self {
        Self
    }
    const fn elapsed(&self) -> std::time::Duration {
        std::time::Duration::ZERO
    }
}
/// Deterministic FxHash-style hasher for solver maps and the persisted `pb_fingerprint`.
/// Its rotate/xor/multiply algorithm and byte encoding are part of the stored table's compatibility contract;
/// replacing it with another hasher can invalidate every table (docs/PERF.md). It is not collision-resistant.
/// Numerical reductions must retain their explicit order rather than depend on hash-map iteration order.
#[derive(Default, Clone, Copy)]
pub(crate) struct Fx(u64);
impl Fx {
    #[inline(always)]
    const fn add(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}
impl std::hash::Hasher for Fx {
    #[inline]
    fn write(&mut self, b: &[u8]) {
        let mut c = b.chunks_exact(8);
        for w in &mut c {
            self.add(u64::from_le_bytes(w.try_into().unwrap()));
        }
        let r = c.remainder();
        if !r.is_empty() {
            let mut t = [0u8; 8];
            t[..r.len()].copy_from_slice(r);
            self.add(u64::from_le_bytes(t));
        }
    }
    #[inline]
    fn write_u8(&mut self, x: u8) {
        self.add(u64::from(x));
    }
    #[inline]
    fn write_u16(&mut self, x: u16) {
        self.add(u64::from(x));
    }
    #[inline]
    fn write_u32(&mut self, x: u32) {
        self.add(u64::from(x));
    }
    #[inline]
    fn write_u64(&mut self, x: u64) {
        self.add(x);
    }
    #[inline]
    fn write_usize(&mut self, x: usize) {
        self.add(x as u64);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}
pub(crate) type FxMap<K, V> = std::collections::HashMap<K, V, std::hash::BuildHasherDefault<Fx>>;
pub(crate) type FxSet<K> = std::collections::HashSet<K, std::hash::BuildHasherDefault<Fx>>;

mod career;
mod deck;
mod skills;
#[cfg(target_arch = "wasm32")]
mod web;

mod chart;
mod cli;
mod data;
mod optimize;
mod schedule;
#[cfg(test)]
mod tests;

pub fn cli_main() {
    let _report = CliBoundReport;
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "pbtable" {
        // ./build.sh pb merges the printed pairwise-ceiling table into data.json.
        println!("{}", pb_table_json(&std::fs::read_to_string(&args[2]).unwrap()));
        return;
    }

    // Validate the first job's mode before loading data or any auxiliary files.
    let job_text = std::fs::read_to_string(&args[2]).unwrap();
    let jobs: Jobs = serde_json::from_str(&job_text).unwrap();
    if jobs.mode != "deck" && jobs.mode != "career" {
        eprintln!("error: \"mode\" must be \"deck\" or \"career\"");
        std::process::exit(2);
    }
    let data = load(&args[1]);
    let mut job: serde_json::Value = serde_json::from_str(&job_text).unwrap();
    let skills: skills::SkillsRaw =
        serde_json::from_str(&std::fs::read_to_string(job["skills_file"].as_str().expect("skills_file")).unwrap())
            .unwrap();
    apply_cli_events(&mut job, &skills);
    let cards: crate::deck::CardsRaw =
        serde_json::from_str(&std::fs::read_to_string(job["cards_file"].as_str().expect("cards_file")).unwrap())
            .unwrap();

    if jobs.mode == "career" {
        restore_cli_career_state(&data, &skills, &cards);
    }

    // Several career jobs share the first job's loaded skills/cards and the worker's caches.
    // Keep the initial enrichment above, then reread and enrich every job (including the first).
    // This preserves file-loading order and which errors occur before state restoration.
    if jobs.mode == "career" && args.len() > 3 {
        for path in &args[2..] {
            let mut job: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            apply_cli_events(&mut job, &skills);
            print_cli_job("career", &data, &skills, &cards, &job);
        }
    } else {
        print_cli_job(&jobs.mode, &data, &skills, &cards, &job);
    }

    if jobs.mode == "career" {
        save_cli_career_state(&data, &skills, &cards);
    }
}
