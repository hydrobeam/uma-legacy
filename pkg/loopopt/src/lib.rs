//! # loopopt: Uma Musume legacy loop and deck optimizer
//!
//! The affinity core below optimizes race schedules for a loop's members by expected legacy affinity; the
//! skill-target model (`skills`), the deck search and the career planner (`career`) build on it. The CLI runs "deck"
//! and "career" jobs, `pbtable` and `constants`; the page runs deck and career jobs through `web`.
//!
//! Objective, normalized to 4 runs:
//!     E = base + 3 * C * sum_{i<j} <P_i, P_j>
//! where P_i[r] = P(member i wins G1 race r at least once) = 1 - prod(1 - p) over her attempts at r,
//! and p = clamp(110 - cost(distance grade) - cost(surface grade) - PEN[scored position], 0, 100) / 100, the scored
//! position being the race's streak position, one less for a goal race (`data::scored_position`).
//! This scheduling objective retains the older half-grandparent normalization: C = 2,
//! base = 2*sum pair + 2*sum trio (`loop_planner::schedules::LoopProblem::new`). The skill model uses full grandparent
//! rates.
//! Both weight every pair's shared wins uniformly, so the normalization preserves schedule rankings within a
//! structure; `skills::lins` derives the current model's weights for loops and rotations.
//!
//! Layers:
//! * `Uma::dp_exact` (in `schedule`): exact best response for one uma against fixed partners. Junior turns run in
//!   sequence; Classic turn t and Senior turn t+24 are decided jointly, so a race offered in both years
//!   (always 24 turns apart) is scored exactly. Checked against the MILP fixtures in `best_response_matches_milp`.
//! * `LoopProblem::ascent` (in `loop_planner::schedules`): best-response dynamics; stops when no member improves her
//!   weighted score by more than 1e-9.
//! * `LoopProblem::group_moves`: coalition deviations (2+ members forced onto the same race, or rest, at one turn),
//!   which escape the coordination equilibria that single-member moves can't leave.
//! * `loop_planner::schedules::optimize`: optimistic start + ascent + group moves + iterated local search (noisy best
//!   responses).
//! * `loop_planner::schedules::pair_ceilings`: exact pairwise ceilings PB(a, b), the bound the deck search screens
//!   structures with (stored in data.json by `loopopt pbtable`).
//!
//! Relocated-venue G1s are merged into their normal race by the exporter (export/export_rs.py).
//! See docs/SOLVER.md for the data format, job fields, validation and benchmarks.

mod uma;
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
    const fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}
impl std::hash::Hasher for Fx {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for w in &mut chunks {
            self.add(u64::from_le_bytes(w.try_into().unwrap()));
        }
        let remainder = chunks.remainder();
        if !remainder.is_empty() {
            let mut tail = [0u8; 8];
            tail[..remainder.len()].copy_from_slice(remainder);
            self.add(u64::from_le_bytes(tail));
        }
    }
    #[inline]
    fn write_u8(&mut self, word: u8) {
        self.add(u64::from(word));
    }
    #[inline]
    fn write_u16(&mut self, word: u16) {
        self.add(u64::from(word));
    }
    #[inline]
    fn write_u32(&mut self, word: u32) {
        self.add(u64::from(word));
    }
    #[inline]
    fn write_u64(&mut self, word: u64) {
        self.add(word);
    }
    #[inline]
    fn write_usize(&mut self, word: usize) {
        self.add(word as u64);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}
pub(crate) type FxMap<K, V> = std::collections::HashMap<K, V, std::hash::BuildHasherDefault<Fx>>;
pub(crate) type FxSet<K> = std::collections::HashSet<K, std::hash::BuildHasherDefault<Fx>>;

/// A memo's value at `key`: the one kept, else `compute(&key)`'s, kept. The memo is not borrowed while `compute` runs,
/// so a computation may read the same memo (`entry().or_insert_with` would hold the borrow across it).
pub(crate) fn memoised<K: Eq + std::hash::Hash, V: Clone>(
    memo: &std::cell::RefCell<FxMap<K, V>>,
    key: K,
    compute: impl FnOnce(&K) -> V,
) -> V {
    if let Some(value) = memo.borrow().get(&key) {
        return value.clone();
    }
    let value = compute(&key);
    memo.borrow_mut().insert(key, value.clone());
    value
}

mod career;
mod deck;
mod skills;
#[cfg(target_arch = "wasm32")]
mod web;

mod chart;
mod checks;
#[cfg(not(target_arch = "wasm32"))]
mod cli;
mod codes;
#[cfg(not(target_arch = "wasm32"))]
mod constants;
mod data;
mod hex;
mod key;
mod loop_planner;
mod refusal;
mod request;
mod rng;
mod schedule;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ts_gen;

/// The native command line (`cli.rs`): `loopopt DATA JOB...`, `loopopt pbtable DATA`, `loopopt constants`.
#[cfg(not(target_arch = "wasm32"))]
pub fn cli_main() -> std::process::ExitCode {
    cli::main()
}
