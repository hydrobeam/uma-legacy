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

The wasm is compiled once, with `WebAssembly.compileStreaming` (Chrome keeps compiled code across visits only for
streaming compiles; `compile` on the bytes where it is missing or refuses), and the `WebAssembly.Module` posted to
each restarted worker (worker ready about 100-190 ms, against 180-490 ms when each worker decoded a base64 copy;
2026-10-10). Optimised wasm ran within 3% of native when measured (2026-10-10); code V8 has not optimised yet runs
slower (below). There is no JS fallback:
wasm2js output was 8.8 MB and 3.6-6x slower than wasm, so the page requires WebAssembly and says so when a host
refuses it.

V8 (Chrome, Edge) runs a wasm function in baseline code (Liftoff) until it has run enough to be compiled optimised
(TurboFan), in the background; a call already running stays in the code it started in (no on-stack replacement for
wasm). A long loop in a function entered once therefore runs in baseline code throughout: the main search's candidate
generation took 2.0 s on export 2 against 1.2 s optimised (Node 22). So such a loop's body is its own `#[inline(never)]`
function, entered many times (`gen_cands`' per-trainee loop), and `wasm-opt` runs with `-ocimfs=0`, as by default it
inlines every function that has one caller, whatever its size. The planner's long loops are split the same way:
`outcome_hi`'s per-(candidate, partner) body (`hi_partner`) and `score_career`'s fold and outcome loop (`sc_fold`,
`sc_outcomes`); on export 2 (`.perf/careerbench.mjs`, Node, 2026-10-20) `hi_ms` 1.91 -> 1.28 s, `score_ms` 4.13 -> 3.99 s,
the plan 10.85 -> 10.1 s, native outputs byte-identical. The optimised code belongs to the compiled module: a
restarted worker given the same `WebAssembly.Module` starts with what the last one tiered up. Liftoff alone
(`--liftoff-only`) took export 2's plan 39 s against 22 s (at be6bb44, before the planner's loops were split), so slow
tier-up cost at most about 1.8x then.

## Open in the deck search (estimates)

- Pruning without the assumed caps (in progress): the class caps (`EXACT_CAPS`) are assumed, and a 10-target export
  scored a deck past one (1.297 against 1.25). Such a search is reported not complete (`cap_exceeded`) but not redone;
  the work under way reruns it under a proven bound (the per-event-count mixture bound, `upper_na`'s form) in place of
  the cap.
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

The large pools are two exports (`.perf/p0/`, not tracked): export 2 (e2_full), 316 veterans on Grand Masters with
every card and uma (8 uncertain targets: the scenario guarantees none), with 24- and 48-veteran cuts (e2_v24, e2_v48);
and export 1 (e1_full), 315 veterans on Unity Cup (6 uncertain targets). Times are native on the i5-1340P unless
marked, each from the commit named; nothing after that commit has re-timed it.

- val/plan, whole job (search and plan), `val/planval.py` at its default two jobs at a time, cb93545
  (`val/plan/ref_times.txt`): p16 0.85 s, p16_ss 0.95 s, p16_ura 0.80 s, p10 2.38 s, p11 2.24 s, gm16 0.92 s,
  gm16_nu8 2.66 s, p16_88 1.82 s, boot_mob 0.84 s. Compare only at the same load (*Rules of thumb*).
- Export 2: 125 s and 1.26 GB at 95f4de8 (about 110 s of it `outcome_hi`, untimed until then: (candidate x partner
  x anchor) x 4^nu leaves and a per-partner leaf store); 18.0 s and 279 MB with `outcome_hi`'s branch and bound
  (eb1c5dc); 9.96 s with `score_career`'s one-walk fold (4ad4491); peak 285 -> 221 MB with per-outcome tables kept
  only for rows that can still be the pick (8c657b7); 8.6 -> 6.1 s with the star-inert collapse (d7e0cc0). In wasm
  (Node, `.perf/careerbench.mjs`) the plan took 10.1 s at fa20726, before the collapse.
- Export 1: 13.9 s at 95f4de8, 9.0 s at eb1c5dc, 5.8 s at 4ad4491. Later commits cut its work (looped-row nodes
  21.7M -> 7.5M, 8c68adc; product-search nodes -25%, 2228d93; `score_ms` 0.40 -> 0.33 s, d7e0cc0); the whole job is
  not re-timed since 4ad4491.
- 8c657b7 against cb93545 (results unchanged, `y_hi` null where y >= 0.5): peak RSS e2_v24 230 -> 188 MB, e2_v48
  229 -> 189, e1_full 94 -> 84, e2_full 285 -> 222; scorings 234 -> 166 on e2_v24/v48 (68 proxy scores reused),
  332 -> 330 on e2_full; gm16_nu8 34.30G -> 32.58G instructions (-5.0%: `score_career` -1.56G, the anchors' branch and
  bound -0.9G).
- The star-inert collapse (d7e0cc0): an uncertain target none of whose setups' table values can move with her 1-3 stars
  (`sc_fold`) is folded at 0 and 1 star only, and the outcome loop reads 1 star's y and odds at 2 and 3 stars while
  every sum still takes every outcome at its own chance. On the `.perf/p0` and val/plan jobs every scoring collapses 2
  of 6 or 3 of 8 targets, all by flat tables; `score_ms` e2_v24 1.27 -> 0.46 s, e2_full 2.81 -> 1.26 s, e1_full
  0.40 -> 0.33 s, e2_v24_t10 23.2 -> 9.5 s (3 of 10). Not byte-identical (a flat table's fold rounds differently per
  star level): decisions identical on all twelve jobs; largest relative change 1.2e-13 (a score near 0.06), savings
  1.1e-15, every `y` and lift identical.
- 10 uncertain targets (e2_v24_t10: e2_v24 plus two targets, one, Straight Descent, with no carrier in the pool): 7.6
  min and 2.44 GB at 95f4de8 (the per-outcome tables 8 MB a row over 323 rows), about 54 s and 490 MB at 8c657b7.
  There y < 0.5 holds at 1% of the outcomes (10,823 of 1,048,576; about 90% on the 8-target jobs, where restricting
  `outcome_hi` to them skips little), `outcome_hi` took 0.6 s, and 73 of its 75 per-uma searches stopped at
  `search_work` (`umas_unproven`). The zero- and low-base bound split (e8fced4) has since cut the deck-search nodes of
  such targets (a 10-target job 7.6M -> 0.34M, gm16_nu8 5.7M -> 2.3M) and the collapse its `score_ms`; the combined
  code is not timed at 10 targets.
- In the page (wasm, 2026-10-09, before all of the above): p16's first plan 1.9 s cold, 1.4 s after a reload (kept
  work persisted). Since 4689795 the stored work includes the main search (`LAST_SOLVE`), so a reload's search job
  searches nothing (native, after a reload: p16 0.09 s -> 0, p10 0.35 s -> 0, gm16_nu8 0.61 s -> 0; stored state
  31-149 KB).
- Reroll (2026-10-10): about 30 ms of solver per roll. A saving roll's exact check, and Compare, are one planner run
  with her added (warm, native about 0.7-0.9 s then). Chance a roll runs that plan: p16 37% (6.1% saving + 30.7%
  provisional), gm16 37%, p10, p11 and boot_mob 100% (`provisional` is unchanged by 8c657b7).
- Small pools (callgrind, 2026-10-09, before the 2026-10-20 work): boot_mob 17.05G instructions, `best_sur` and the
  deck searches under it about 58%, the outcome loop (`score_career`'s own cost, the table folds, `Cont::plan`'s
  lookups) about 25%.

### Open

- `outcome_hi`: y_hi is computed only where y < 0.5; stopping each cell's bound once it reaches 0.5 would keep every
  page decision but change those cells' rounded y_hi (planval would differ).
- More than 8 uncertain targets: the planner's tables grow as 2^nu (corner tables), 4^nu (outcome tables) and 2^T
  (`outcome_sets`, up to 12 targets). A design for 10-20 targets, not built: a factored exact form of the corner tables
  per chain state, outcomes as count-indexed brackets refined only where the pick is undecided, decks by local search
  with the exact search as verifier, and a second tier whose decisions match the exact one's within a stated
  tolerance instead of byte for byte.
- The page's planner against simpler policies: the simulator's `index` policy (468c861) runs it on the same draws as
  `greedy`; no comparison of useful size has been run (about 0.2-0.5 s per plan step on 10-16-veteran pools, minutes
  per run of a few hundred careers).
- `score_career`'s outcome loop: its per-candidate branches (about 70 ns per outcome, estimated before 4ad4491 reworked
  the loop). `multi.rs`'s `ents_of` still clones.
- Cooperative cancel (a superseded plan keeps its worker and caches): a SharedArrayBuffer flag polled at the progress
  points and in `best_deck`, with no partial work stored; it needs COOP/COEP headers on the site (a deploy decision),
  terminate-and-restart kept as the fallback.
- Consolidate `score_career`'s and `outcome_hi`'s outcome loops (one loop, estimate and bound; y byte-identical); the
  pass-2 replay (Tried) may fold in there.
- Not tried, estimates only: a node-marginal ratio bound at depth <= 2; the deck search's per-leaf `dw`/`dg`
  allocations and the band test's second evaluation of the same clause value (3-15%); reusing pass 1's setups in
  pass 2 (about 90 ms to gain, 31 MB to hold on p10); smoothing p_inf over the top 3 builders (the argmax makes p_inf
  jump when near-tied builders swap).
- Page: keep the worker's kept bests across a superseded job (a settings change during a plan discards them; an edit
  that leaves the job as it is no longer stops the plan); reuse kept plans for pools that grow by several veterans or
  lose one (today: the same pool, or one more veteran at the end).
- Parallel helper workers are a last resort: they hide work rather than remove it, and a parallel plan is
  deterministic only with the winners-only cache and exact, unbudgeted searches.

### Tried and not kept

| Item | Result | Why not |
|---|---|---|
| A skip-ahead or another memo for `Cont::plan` | memo variants within noise | `Cont::plan` is 1-2% of a plan; on the large exports its scan stops at n <= 1 (at most 19 on val/plan) |
| Dominance in the deck search's band pass | not invisible | removing decks changes the band's size, the cap ladder and which decks are scored exactly |
| A Pareto filter on `outcome_hi`'s anchors (about 30% survive) | not needed | the branch and bound's root test drops about 98% of anchors at about 0.3 us each |
| A fused depth-first ceiling pass in `outcome_hi` | slower | the vectorised `leaf_products` is memory-bound but faster |
| Partner pruning by ceiling in `score_career`'s setups | not exact | `val` can run a fill-mode deck search that adds to the cache; setups cost 48-80 ms |
| Skipping cost lookups for candidates whose step can't move (`cost_lo` bound) | exact; lookups -62% | about 6% of the outcome loop |
| Caching the main search's ceilings for the per-uma solves | about 0.3% | about 13 MB on 315 veterans, 60+ MB on 400, with a pool key |
| An index on `ruled` | scans about 1 ms per job | nothing to gain |
| Carrying `ruled` across pools (per setup, like SEEN) | changes outputs | `certified` takes a proxy career a foreign proof certifies, possibly another pair or deck of equal value |
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
| Collapsed targets at their summed star chance (one outcome for 1-3 stars, weight w1 + w2 + w3) | `score_ms` e2_v24_t10 23.2 -> 6.6 s; decisions identical | the regrouped sums move the fit (lam, phi) by up to 3.9e-12 and savings by 5.3e-12 relative on e2_v24_t10, past the 1e-12 bar; kept: every outcome summed in order (9.5 s) |
| A/B pre-check at the root bound | its B side is `p0 * sp0`, no outfit call | nothing to save |
| mimalloc (native) | within noise on p16, p10, boot_mob, gm16, pal_A2 | allocation's RAM misses are at most about 7% of time |
| talc (wasm) | p16 -4.3%, boot_mob 0% | +7% download (wasm 1.84 -> 1.97 MB) for one pool |
| Incremental trio V (only trios reading a changed step cost) | exact; +218M instructions on p16 against `trio_v`'s 290M | nearly every trio reads the changed candidate |
| `trio_v` over trios sorted by C's step, stopping once c3 >= best | exact; +115M on p16 | C's step is small next to A's and B's, the stop rarely fires |
| Skip `hopeful` at the final slot for single-trainee clauses (reasoned, not measured) | `hopeful` costs less than the leaf's own outfit evaluation | every child it rejects would go to the leaf |
| Skipping builders exactly (estimate) | about 45-55 of 60 exact searches would remain | Cont needs the most efficient builder over every uma (efficiencies within a few per cent); a cheaper Cont is a model decision |
