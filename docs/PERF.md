# Performance work on the solver

How to speed up `pkg/loopopt` without changing its results, learned from the 2026-10-03 run (branch
`perf-plan-proto`, merged: every search at 42-55% of its previous time, outputs unchanged on all validation jobs).

## Protocol

1. **Setup.** Work in a worktree. Build the starting binary twice and diff its outputs on every job before trusting
   it as a reference: anything that sums in `HashMap` order changes its last bits from run to run (the old
   `chain_marginals` did, on 9 of 46 job files).
2. **Reference.** A deterministic build, committed as WIP. Every later item must be byte-identical to the previous
   accepted state (ignoring `timing_s`), which is stricter than `val/cmp2.py`.
3. **Measure.**
   - Phase timers or callgrind on the fast jobs (pal_U, pal_A2 per-uma) to pick targets; total instruction counts
     are a noise-free A/B signal. Line-level `callgrind_annotate` shows whether a phase's cost is the work or the scan
     around it.
   - Wall time for slow jobs: the two binaries side by side, best of 3, at the same machine load.
4. **Verify each item.** `cargo test --release`; `./build.sh valq TAG` (every job but the slow ones in `val/slow.json`); the full `./build.sh val TAG`
   when the change touches the shared three-result search (`top_decks` > 1: the second enumeration pass,
   `k_filter` with kmax > 1, frontier caps, branch reuse), and once at the end.
5. **Commit.** One item per commit, the measured gain in the message. Revert anything under about 3% unless it is
   trivially small.
6. **Final.** Strip timers, rebuild, full validation, wasm timings on Node, `./build.sh page test` on main.

## What wasted time

- Treating a non-deterministic binary as the exact reference.
- Running checks 4 at a time on an idle machine. Wall time is set by the slowest job; run 12-14 at a time.
- Callgrind on multi-second jobs (minutes each). Use it on pal_U / pal_A2 only.
- Timing while another session loads the machine. This laptop (i5-1340P: 4 performance + 8 efficiency cores)
  throttles; full parallel load roughly doubles the slowest job and inflates everything 1.5-2x. Compare only runs at
  the same load.
- Plan-mode interruptions mid-item. Commit early.

## Estimates that were badly off

- The beaten-deck bookkeeping (`Kept`): planned as small, measured as the largest win (-20% on slow per-uma searches,
  -43% on shared three-result ones). The old filter rejected 99% of candidates, so the cost was the scan itself
  (1.4G signature checks for 15.6M real dominance tests); per-field bitmaps test 64 decks per AND.
- The precomputed pair-ceiling table: planned at tens of ms, measured at 10-21% of a typical run.
- Dropping native timers in `probs` / `score`: planned as a gain, measured as none.
- Lazy class thinning: planned as a gain, infeasible without changing output bits.
- No identity check failed during the whole run, on any subset.

## Pitfalls in this codebase

- `Kept` visit order matters: the first beating deck found feeds the recent-winner ring and `lastrow`. Prefilters may
  skip candidates but must visit the survivors in index order.
- Float order: a maximum may be computed in any order, a sum may not.
- Pruning bounds need a rounding slack (1e-9 relative). A trainee's value rises with every deck odds coordinate only
  while white < gold and 0 <= q <= 1.
- serde_json does not round-trip floats exactly without its `float_roundtrip` feature: store bit patterns (hex) for
  anything that must be exact (the `pb` table does).
- `data.json` is tracked and written by Python (`json.dump` defaults). The `pb` table carries a fingerprint over the
  race data; a stale table is ignored (slower, still correct). Rerun `./build.sh pb` after `data` or a solver change.
- The hasher is a hand-written FxHash (`lib.rs`), deliberately not a crate: the `pb` fingerprint uses it, and a
  crate update that changes the algorithm would silently stale every stored table. Randomly seeded hashers (std
  SipHash, ahash) bring back run-to-run nondeterminism.
- The deck enumeration depends on the uma pool only through the plain-pal display order; that is what makes the
  shared-mode branch cache valid. Recheck it if pool-dependent code enters `run_deck_relaxed` before the frontier is
  built. `run_deck` branches change `exclude`, `require` and `exclude_cards`.

Done in the page afterwards: the wasm is compiled once and the `WebAssembly.Module` posted to each restarted worker
(worker ready about 100-190 ms against 180-490 ms with base64). The wasm2js fallback was dropped (2026-10-03): it was
8.8 MB of the then single-file page and 3.6-6x slower than wasm, which runs within 3% of native; the page now requires
WebAssembly and says so when a host refuses it.

## Not done (estimates)

- Lazy k-best for shared `top_decks` > 1: pal_I shared k3 spends 68 of 75 s in the second enumeration pass. A k-best
  search would run the first pass three times, banning each deck found. Estimated 75 -> 15-20 s, several days, high
  risk of changing which tied deck is reported.
- Native threads: no browser gain; splitting the enumeration changes tie order.
