# Performance work on the solver

How to speed up `pkg/loopopt` without changing its results (the protocol, the codebase's pitfalls), and the planner's
speed record: where it stands, what is open, what was tried and rejected.

## Protocol

1. **Setup.** Build the starting binary twice and diff its outputs on every job before trusting it as a reference:
   anything that sums in `HashMap` order changes its last bits from run to run.
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
6. **Final.** Strip timers, rebuild, full validation, wasm timings on Node, `./build.sh page test`.

## Rules of thumb

- Run checks 12-14 at a time: wall time is set by the slowest job.
- Use callgrind on pal_U / pal_A2 only; on multi-second jobs it takes minutes each.
- Compare timings only at the same machine load. The i5-1340P used for the numbers here (4 performance + 8 efficiency
  cores) throttles: full parallel load roughly doubles the slowest job and inflates everything 1.5-2x.
- Profile before estimating where time goes:

  - A filter that rejects nearly everything can cost more than the work it saves: the beaten-deck bookkeeping
    (`Kept`) rejected 99% of candidates, so its cost was the scan itself (1.4G signature checks for 15.6M real
    dominance tests). Per-field bitmaps (64 decks per AND) made it the largest win: -20% on slow per-uma searches,
    -43% on shared three-result ones.
  - Computing the pair-ceiling table costs 10-21% of a typical run, hence the stored `pb` table.

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

## In the page

The wasm is compiled once and the `WebAssembly.Module` posted to each restarted worker (worker ready about 100-190 ms,
against 180-490 ms when each worker decoded a base64 copy). Wasm runs within 3% of native. There is no JS fallback:
wasm2js output was 8.8 MB and 3.6-6x slower than wasm, so the page requires WebAssembly and says so when a host
refuses it.

## Open in the deck search (estimates)

- Lazy k-best for shared `top_decks` > 1: pal_I shared k3 spends 68 of 75 s in the second enumeration pass. A k-best
  search would run the first pass three times, banning each deck found. Estimated 75 -> 15-20 s, several days, high
  risk of changing which tied deck is reported.
- Native threads: no browser gain; splitting the enumeration changes tie order.

## Planner

The career page's plan (`next` job, `sim::plan::index_rule`; `docs/SOLVER.md`, *Next career*). Harness:
`val/planval.py` runs the nine next-plan jobs in `val/plan/` (p16 = a 16-veteran export, p16_ss and p16_ura = p16 at
rank SS and on URA, p10, p11, gm16 = p16 on Grand Masters, gm16_nu8 = gm16 with no target guaranteed (8 uncertain
targets, as Grand Masters gives none), p16_88 = p16 with two of its veterans completed to 8/8 (looped rows),
boot_mob = the page's sample mob veterans) and compares each whole output strictly; the plan reports its own work
(`next.work`: deck-search nodes, exact scorings, and `prof`, its phase timers and counters: `hi_ms` (`outcome_hi`),
`score_ms`, `gen_ms` (the solves' candidate ceilings), `corner_ms`), deterministic but for the timers. `val/mono.py` checks that a veteran added never raises V. Rerun `next.proxy_check`
after any change to how builders' careers are built.

### Where it stands

- Whole job (search and plan), native, `val/planval.py` on 2026-10-10: p16 0.75 s, p16_ss 0.77 s, p16_ura 0.71 s,
  p10 1.69 s, p11 1.71 s, gm16 0.98 s, boot_mob 0.79 s.
- In the page (2026-10-09): p16's first plan 1.9 s cold, 1.4 s after a reload (kept work persisted).
- Reroll: about 30 ms of solver per roll. A saving roll's exact check, and Compare, are one planner run with her
  added (warm, native about 0.7-0.9 s). Chance a roll runs that plan: p16 37% (6.1% saving + 30.7% provisional), gm16
  37%, p10, p11 and boot_mob 100%.
- Where the time goes (callgrind, 2026-10-09): boot_mob 17.05G instructions, `best_sur` and the deck searches under
  it about 58%, the outcome loop (`score_career`'s own cost, the table folds, `Cont::plan`'s lookups) about 25%.
- Parallel helper workers are a last resort: they hide work rather than remove it, and a parallel plan is
  deterministic only with the winners-only cache and exact, unbudgeted searches.

### Open

- Consolidate `score_career`'s and `outcome_hi`'s outcome loops (one loop, estimate and bound; y byte-identical); the
  pass-2 replay (Tried) may fold in there.
- A skip-ahead in `Cont::plan` (x_n is convex in n analytically) needs a float-exact argument for its first stop.
- Not tried, estimates only: a node-marginal ratio bound at depth <= 2; the deck search's per-leaf `dw`/`dg`
  allocations and the band test's second evaluation of the same clause value (3-15%); reusing pass 1's setups in
  pass 2 (about 90 ms to gain, 31 MB to hold on p10); smoothing p_inf over the top 3 builders (the argmax makes p_inf
  jump when near-tied builders swap).
- Page: keep the worker's kept bests across a superseded job (a settings change during a plan discards them); reuse
  kept plans for pools that grow by several veterans or lose one (today: the same pool, or one more veteran at the
  end).

### Tried and not kept

| Item | Result | Why not |
|---|---|---|
| Seed each builder's floor with the previous builder's career | -7% / -4% / +5% | noise |
| Pass 2 against the bar the candidate must beat (not its own best) | -6% | worse with class sharing (fewer proofs to share) |
| Classmate's best deck as the product search's starting bar | nodes -4% (p16), -0.3% (p10) | bound looseness, not the incumbent, sets pass 1's tree |
| Trio-V memo keyed by the step-cost vector | exact, 68-92% hits; instructions p16 -1.2%, boot_mob +0.1% | `trio_v` costs about what hashing its key does |
| One pass instead of the deck search's two | pass-1 / pass-2 nodes: p16 1.11M / 2.62M, p10 8.21M / 7.04M, p11 10.17M / 3.74M, gm16 0.33M / 6.78M, boot_mob 1.40M / 4.18M | pass 1 is not a re-walk (it has no bar until it finds a deck); a merged pass would prune 25% looser during the weak-bar phase |
| Memoise `best_deck` calls with start decks | repeats: 0 on every job | nothing repeats |
| Fold Compare's "best career with her" search into its replan | the replan's searches bar each uma at her kept best | no work to share |
| Frozen-fit Compare (`next.fit`) | stopped one flip (refit +20 vs fixed -8.7 on V 1,083) | on a small pool Compare's plan differed from Add's (Agnes Digital at 1,530 vs Tamamo Cross at 2,009); `fit` stays in the solver, unused by the page |
| Each fresh 8/8 from her 3 best pairs | V 0.6-1.5% lower (p16 1,083 -> 1,067), fixes the extra-wins rise | +20-40% planner work (p16 3.39M -> 4.42M nodes, p10 9.90M -> 15.43M) and still not monotone |
| Configurations reused by the second pass | hits 472 of 1,163 searches | no CPU gain |
| Bounds restricted to the entries still pickable | nodes -9 to -21%, CPU +1 to +22% | the tables cost more than the nodes saved; caching them would take hundreds of MB of wasm heap |
| Each class's highest-ceiling trainees first | nodes p16 -0.6%, URA -1.7%, others 0% | under the 3% bar |
| Pass 2 replayed from pass 1's outcomes (`score_replay`) | exact; 2-7% on plans, about 10% on the warm Compare plan | 74 lines and a second scoring path; reconsider inside the outcome-loop consolidation |
| wasm SIMD (`+simd128`) | p16 -1.7%, boot_mob +3.8% (V8, within noise) | the hot loops are branchy with serial products; recheck if `outfit_fast` becomes branch-free |
| `outfit_fast` with u8-state `if` selects / f64-state compares | -0.4 to -1.4% / +0.6 to +0.8% | LLVM re-emits branches / no vectorisation (`OutfitPre`'s u64 masks do vectorise) |
| A/B pre-check at the root bound | its B side is `p0 * sp0`, no outfit call | nothing to save |
| mimalloc (native) | within noise on p16, p10, boot_mob, gm16, pal_A2 | allocation's RAM misses are at most about 7% of time |
| talc (wasm) | p16 -4.3%, boot_mob 0% | +7% download (wasm 1.84 -> 1.97 MB) for one pool |
| Incremental trio V (only trios reading a changed step cost) | exact; +218M instructions on p16 against `trio_v`'s 290M | nearly every trio reads the changed candidate |
| `trio_v` over trios sorted by C's step, stopping once c3 >= best | exact; +115M on p16 | C's step is small next to A's and B's, the stop rarely fires |
| Skip `hopeful` at the final slot for single-trainee clauses (reasoned, not measured) | `hopeful` costs less than the leaf's own outfit evaluation | every child it rejects would go to the leaf |
| Skipping builders exactly (estimate) | about 45-55 of 60 exact searches would remain | Cont needs the most efficient builder over every uma (efficiencies within a few per cent); a cheaper Cont is a model decision |
