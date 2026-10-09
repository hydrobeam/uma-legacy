# Planner speed: measurements and plan

The career page's first plan (`next` job, `sim::plan::index_rule`) takes about 12 s in wasm on the owner's 16-veteran
pool; this file holds what was measured about where that time goes, what was tried, and the plan of attack. The
protocol is `docs/PERF.md`'s: a deterministic reference, byte-identical outputs per item (or a deliberate, measured
re-baseline), one item per commit with its gain.

## Status (2026-10-09, early morning)

First plan on the owner's 16-veteran export, in the page: about 12 s (2026-10-08 morning) -> 2.4 s with the Cont proxy
-> 1.9 s cold and 1.4 s after a reload (kept work persisted); measured before the reroll-table merge, whose own-8/8
credit raised p16's exact builder searches 9 -> 28 (native plan about 1.4 -> 1.8 s). Reroll: about 30 ms of solver per
roll; a table zero is now proven (y_hi < 0.5) or checked in the background; a saving roll's exact check and Compare are
one planner run with her added (warm, native about 0.7-0.9 s). Sample mob veterans: native 25.2 -> about 8-9 s
(certified proxies) -> about 7 s on main 619c0ed -> about 2.1 s (2026-10-09, identical deck problems searched once:
nodes 67.3M -> 5.4M; now about 1.4x p16 at the same load, was 5.7x) -> about 0.92 s (Cont memos, below: total_ms
1.59 -> 0.92 s; p16 1.04 -> 0.93 s, p10 2.11 -> 1.73 s, gm16 1.55 -> 1.42 s). Grand Masters: native 44 -> about 2.3-4 s.

Merged on main (all byte-identical on val/plan except the deliberate re-baselines: the winners-only cache, the Cont
proxy, the own-8/8 credit): deterministic cache, class-keyed proofs, planner arithmetic (Cont tables, fold buffers),
report skip, upper_na relaxation and memoisation, Cont proxy (PROXY_EFF/PROXY_SAVE 0.2), certified proxies, derived
stage candidates (no loop-planner lookup), persistence (IndexedDB state_in/state_out), the state-conditioned bound at
configuration roots (SBOUND; SB_CHECK audits it), reroll table own-8/8 credit and y_hi bound, cap tripwires.

Recalibrated (2026-10-09): PROXY_EFF/PROXY_SAVE 0.2 -> 0.06 (about 2.2x the largest proxy error measured on current
code, 2.8% on p11; proxy == exact for every builder on five of seven jobs); shown rows, pick, V, Cont identical on all
seven; exact searches p16 28 -> 10, gm16 39 -> 10, p11 38 -> 16; plan CPU p16 1.73 -> 1.14 s, gm16 3.52 -> 1.86 s.
Rerun next.proxy_check after any change to how builders' careers are built.

Open:
- Consolidate score_career's and outcome_hi's outcome loops (one loop, estimate and bound; y byte-identical); the parked
  pass-2 replay may fold in there.
- Page: Compare shows the table's instant figure while its replan runs (as Reroll does).
- The browser suite has not run since v40; several suite checks were edited to match new behaviour without running.
  Run it once before any release. Nothing since v40 is deployed.
- Owner decisions on record: the 1.25 cap stays (real ratios 1.18 Global / 1.17 GM; the cap-free bound is valid but
  13-45x slower as a replacement); parallel helper workers are a last resort.

## Where the time goes (measured 2026-10-08, native, p16 = the owner's 16 veterans)

| Step | Native | Note |
|---|---|---|
| Builder best careers (60 umas) | 4.1-4.5 s | about 80% of the plan; wasm about 2x native (9.2 s) |
| Stage candidates' best careers (7) | 0.37 s | |
| Index (passes 1 and 2 without searches) | about 0.35 s | |
| Two deep | 0.29 s | |
| Loop lookup | 0.16 s | |

Inside the builder searches (p16): 60 solves make 607 deck searches (`best_deck`), 424 of them complete (about 7 per
builder: the parent pairs whose ceiling clears the bar). Product search (pass 1) 1.5 s, 10.7M nodes; band enumeration
(pass 2, every deck with product x rho >= best exact) 2.5 s, 26.1M nodes, 41k decks, of which `upper_na` leaves 1.8k
for exact scoring (0.15 s). On the 10-veteran pools the shape differs: 2 complete searches per builder, but each costs
80k product nodes (p16: 17k).

The cap: rho = 1.25 (assumed, `EXACT_RHO`; largest exact/product ratio seen 1.17 on Grand Masters, 1.04 on Global).
Builder time at rho 1.25 / 1.10 / 1.02: 4.4 / 2.1 / 1.1 s. At 1.02 one career changed (King Halo, 1 in 689 -> 691.7: a
deck with a ratio above 1.02 was skipped), so lowering the cap is an assumption change, not a speed-up.

## The cap (reasoning agent, 2026-10-08)

No data-free proven cap exists: a target whose only source is one card's last link makes exact/product = 1/P(link)
(about 6.7 with two such targets); real targets have floors (inheritance, white generation), but range-only and Hölder
caps still come out at 1.2-1.7. Measured: 1.18 on the Global deck job pal_UF (pal_I 1.165), 1.17 on Grand Masters; the
planner jobs' scored decks 1.03-1.04 (p16, p10). So 1.25 has about 6% of headroom on both eras. Guards (done): the
planner reports `next.work.max_ratio`, and `val/planval.py` / `val/cmp2.py` fail when a scored deck comes within 5% of
the cap; they cannot see pruned decks, hence the planned build-time audit (rerun near-best decks with the cap at 2.0).
The replacement that needs no cap: at a search node the chain-kind make-up is fixed, so the distribution of chain
progress states is known; given a state, targets are independent apart from exclusive outcomes (which only lower the
value), so the average over states of each target's best value given the state bounds every completion's exact value,
for any card data. About 20-50x a node's current bound cost (estimate); prototype at configuration roots and depth <= 1,
measured in nodes.

## Deck classes

A uma enters the deck search only through her outfits' coverage of the targets (`model.opts`, `ew0`), the relevant
cards she cannot use (her own, her groups'), the plain pals she may take and `dmax` (which follows from those). Umas
equal on all four have the same deck problem: the best deck is a function of the inheritance chances q alone, times
the spark scale c. Within a class and for one parent pair only the slots' affinity differs (q moves; c does not).
p16: 47 of 68 umas in one class (21 classes); p10/p11: 38 of 51 (13 classes).

## Tried

| Item | Result | Kept |
|---|---|---|
| Seed each builder's floor with the previous builder's career | -7% / -4% / +5% (noise) | no |
| Pass 2 against the bar the candidate must beat (not its own best) | -6%; worse with class sharing (fewer proofs to share) | no |
| Class-shared proofs (`seen`, `ruled` keyed by class) | p16 builders 4.07 -> 1.70 s (2.4x); p10 -12%, p11 -15%; identical careers, V, scores | yes (item 2) |
| Classmate's best deck as the product search's starting bar | nodes -4% (p16), -0.3% (p10) | no: bound looseness, not the incumbent, sets pass 1's tree |
| Trio-V memo keyed by the step-cost vector (2026-10-09) | exact, 68-92% hits, but instructions p16 -1.2%, boot_mob +0.1%: `trio_v` costs about what hashing its key does | no (below the 3% bar; patch was in the memo agent's scratch) |
| `Cont::cost_lo` early exit (2026-10-09): stop once the building alone reaches the best (later n cost more) | identical on val/plan; p16 16.92G -> 16.56G instructions (-2.1%) on top of the memo | yes (c9d0bfe) |
| Merge the deck search's two passes into one (2026-10-09, measured before building) | per plan, pass-1 / pass-2 nodes: p16 1.11M / 2.62M, p10 8.21M / 7.04M, p11 10.17M / 3.74M, gm16 0.33M / 6.78M, boot_mob 1.40M / 4.18M; band decks reached by the scoring loop: all but 0-310 of those collected | no: pass 1 is not a re-walk (it is larger than pass 2 on p10/p11: it has no bar until it finds a deck, pass 2 starts with one), the fixed band bar wastes about nothing, and a merged pass would prune 25% looser during the weak-bar phase |
| Memoise `best_deck` calls with start decks (2026-10-09, counted before keeping) | calls with starts / repeats: p16 54/0, p10 28/0, p11 56/0, gm16 42/0, boot_mob 0/0 (its proxy searches have no starts and the existing memo already hits 64 of 87) | no: nothing repeats |
| Fold Compare's "best career with her" search into its replan (2026-10-09, code reading) | the replan's searches with her fixed bar at each uma's kept best (`env.floor`), so they never find a career with her below it; the panel needs the unconditional best | no: no work to share but one job's overhead |
| Frozen-fit Compare (`next.fit`, 2026-10-08/09) | stopped the Inari flip (refit +20 vs fixed -8.7 on V 1,083), but on a small pool Compare's plan differed from Add's (Gold Ship: Agnes Digital at 1,530 vs Tamamo Cross at 2,009) | reverted in b2ecc73; `fit` stays in the solver (output and input), unused by the page |
| Lineage alternatives: each fresh 8/8 from her 3 best pairs (2026-10-09) | V 0.6-1.5% lower (owner pool 1,083 -> 1,067; p10 -0.6%), fixes the extra-wins rise; +20-40% planner work (p16 3.39M -> 4.42M nodes, p10 9.90M -> 15.43M); still not monotone (a pair pushed past the top 3) | parked on branch worktree-agent-acf153943fcd00fff (82a3721, 58959aa), `next.alt_k` |

## Findings from code (reasoning agents, 2026-10-08)

- A proven cap of 1 exists only for decks with no target-bearing chain card (negative dependence of one event's
  outcomes; derived), which is almost no deck. No general cap-free branch bound is valid: one chain card feeding two
  targets breaks a per-target mixture bound, and a later card can become a second feeder below a branch.
- Plan outputs can depend on search history: the planner's cache took every deck a search happened to check fully
  (`Cache::absorb`), and the trios' third members, candidates' lifted odds, Cont and V read the best of them. Pruning,
  the per-setup proofs kept between jobs (SEEN) and class sharing change which decks those are. Equal outputs in the
  experiments above were not guaranteed. Fix: the cache holds exact winners only (item 1).
- Exact skipping of builders is blocked by Cont (the most efficient builder over every uma, efficiencies within a few
  per cent): about 45-55 of 60 exact searches would remain (estimate). A cheaper Cont is a model decision.
- `rise(t, q', q)` and every proof (`Ref`, `ruled`) depend on the trainee only through her class: sharing is exact.

## Plan (in order)

0. Harness: `val/planval.py` runs six next-plan jobs (`val/plan/`: p16, p10, p11, p16 at rank SS, p16 on URA, p16 on
   Grand Masters) and compares the whole output strictly; the plan reports its own work (`next.work`: deck-search
   nodes, exact scorings), deterministic and comparable at any machine load.
1. Deterministic cache. Prerequisite for every pruning change and for parallel workers; moves results once.
   Winners only (each search's best deck) measured against the old cache: p16 candidates' stage costs +1-4%, p10's
   member lift 0.565 -> 0.526 and careers to loop 1044 -> 1058; URA and Grand Masters unchanged. The loss is fidelity: a
   candidate's lifted odds are the best of her cached decks' tables at the new chances, and one deck reads lower than
   several. Variant: the main search runs without earlier jobs' proofs (its fully checked decks then depend on the job
   alone) and feeds the cache, plus every other search's winner: the same numbers as winners only on all five Global
   jobs, so winners only it is (done: picks and top five unchanged on all six jobs, V unchanged but p10 +0.5%, stage
   costs up to 4.5%). If the candidates' fidelity matters later: each candidate's top 3 careers' decks (deterministic,
   about 7 cheap searches).
2. Class-keyed proofs (`seen`, `ruled`; `gm_cached`, `reduced_cached` by legal-set content). Measured 2.4x on p16
   builders. Solve classmates in slot-wise affinity order (dominant first: their proofs transfer with rise = 1).
3. Deck search, exact and byte-identical: build the configuration skeleton once per `best_deck` (both passes run at the
   same q) and hoist per-call work (counts, legal-set tables, lock bits); suffix-restricted bound tables (a branch can
   only take entries after its position; estimate 10-40% fewer nodes); node-marginal ratio bound at depth <= 2 (measure).
3b. Deck search memory traffic (byte-safe): the DFS copies four `[f64; 32]` per node (`rec`, about 1 KB, 6 values
   live; 37M nodes per plan on p16): a flat stack of stride `nu` (estimate 10-25% of search time); per-leaf `dw`/`dg`
   allocations and the band test's second evaluation of the same clause value (estimate 3-15%); `gm_cached` keyed by
   content and stored flat (classmates rebuild identical tables today: tens of MB of wasm heap high-water, estimate).
3c. `upper_na`'s per-call constants (measured on main after the merges, native): Grand Masters 1.70M calls, 6.56 s of
   a 16.6 s plan (39%), 543k on the pair path whose `pair_marg` rebuild alone is 1.77 s; p16 22k calls, 0.18 s of 3.7 s
   (5%). Memoise the pair marginals per key and pair, the event pmf, each group's NUOD flag (static per card source),
   the key level lookup; byte-identical (data-structure survey: no container change elsewhere is worth 1%).
4. Merge the two passes (an existence check, then one band DFS): exact under the cap; counters re-baseline; estimate
   -5 to -20% after item 3.
5. Planner arithmetic: reuse `score_career`'s setups and folds between passes 1 and 2; the fold recursion allocates a
   fresh `Vec` per branch (about 5,460 per table fold, nu = 6): one scratch buffer per level (estimate 30-60% of the
   index's non-search time); borrow cached tables instead of cloning them.
6. Page: keep worker state across a superseded job (today a settings change during a plan discards every kept best);
   persist kept bests in IndexedDB (float bits, keyed by build id, setup and legacies); generalise reuse to pools that
   grow by several veterans or lose one; skip the plan job's unused `results`.
7. Last resort, not planned (owner, 2026-10-08: meaningful gains first; parallel workers hide work rather than remove
   it): helper workers for the builder searches (design in the page-side agent's report: deterministic only with the
   winners-only cache and exact, unbudgeted searches).
8. Owner decisions (change results):
   - Cont from near-top builders: rank every builder by a table-value proxy (her class's winning decks at her own q),
     exact searches only for builders within about 10% of a candidate's top two, the displayed rows, the pick's
     near-tie band and the two-deep top 3. Estimate: 60 -> 10-20 exact builder searches (65-80% off builder time). The
     rule's fit (val/sim/README.md E5) is good to 10-20%, while the top builders' efficiencies differ by a few per cent,
     so the exact argmax buys precision the model lacks; optionally smooth p_inf over the top 3 builders (the argmax
     makes p_inf jump when near-tied builders swap). Validate on val/plan and the simulator's planner audit.
   - A per-era cap (e.g. 1.06 Global, 1.25 Grand Masters): a deck above the cap is missed silently.

## Page JavaScript (separate from the planner's time)

The page's own work is milliseconds against seconds of solving. User-visible: the status spinner is rebuilt (and its
animation restarted) every second; every filter keystroke rebuilds the whole roster or card list (filter by `hidden`);
the card count is O(n^2) and goes stale after a toggle; `renderPool` redraws the closed veterans dialog on every event;
`fitSparks` forces a layout per step and stacks on resize. Transfer: nothing is minified (about 25 KB per page,
estimate); CSS and the shared data are inlined into each page and re-fetched after the 5-minute HTML cache. Code
health: `el`/`fmtIn`/backdrop-close duplicated across modules; career.js (966 lines) mixes job building, the
plan/compare/reroll state machines, views and import/export.

## Done
- Sample mob veterans (val/plan boot_mob, the page's "Try sample veterans"), 2026-10-09. Cause: a sample pool is six mob
  veterans with one 1-star spark each and mob parents; no mob is in `data.umas`, so no relation or race points anywhere and
  every draw is the same problem (verified: two fresh draws give the main search's and the plan's work counters to the
  unit). For every trainee all 15 parent pairs then have bit-identical chances, and every trainee of one deck class has
  the same chances too. Nothing collapsed that symmetry: `dominates` needs the same character, and every bound prunes
  only strictly below the bar, so a twin (a tie) was searched in full. Measured on main: each `best_of` solved up to 15
  identical pairs (one full search each, each re-scoring every earlier pair's deck as a start: 120 exact scorings), the
  hard work cap cutting some at 6-11; pass 1 searched each of the 47 umas of the big class in full (about 450k nodes
  each: a class with no winning deck gets no proxy deck), and the proxy loop's 17 exact searches repeated the class
  searches. Plan 67.3M nodes, 1,316 exact scorings, 6.0 of 7.0 s in searches; the search job itself was 0.12 s.
  Fixes (exact; derivations in the code): `Twin` in `solve` (an earlier candidate's outcome for a bit-identical
  problem: 67.3M -> 31.6M nodes, 1,316 -> 158 scorings) and `deck_memo` in `best_deck` (calls without start decks
  memoised by what they read, the work budget's tests reproduced: 31.6M -> 5.4M, 158 -> 35; p10 -5.8%, p11 -10.2% nodes,
  p16 -0.7%, gm16 0). val/plan identical on all seven, valq 0 mismatches, the five career jobs identical whole. Left:
  the per-outcome planner arithmetic (`score_career`'s outcome loop, 1.4 s against p16's 0.4 s at equal load, from
  instrumented timers): with candidates at p about 1.8e-5 every outcome of every builder lifts a candidate, so nearly
  every outcome pays `Cont::plan` (about 16 exp per call at builds 16) and a trio V; p16's outcomes mostly lift no one.
  A skip-ahead in `Cont::plan` (x_n is convex in n analytically) would need a float-exact argument for its first stop.
  Done next (entry below): the Cont memos.
- Cont memos (2026-10-09, exact by construction): `Cont::plan` and `Cont::cost_lo` memoised per Cont by the bits of p
  (both read p and the Cont's parameters, set once in `new`; clones share the memo and the parameters, a refit makes new
  Conts with empty memos). Lookups gated off below 1 hit in 8 after 4,096 (never triggered on val/plan), cleared past
  16k entries per memo (about 0.6 MB; no cap would add 0.2-1.5% hits). Hit rates: plan 89-99% on all seven jobs
  (boot_mob 95.9%, p16 92.2%, p10 89.1%, gm16 98.9%; 3-8M lookups per plan, so even p16's outcomes call it millions of
  times); cost_lo (outcome_hi, 500 exp per call, 49k calls on boot_mob) 99.5% boot_mob, 77-80% p16/gm16, 48-56% p10/p11.
  Instructions (callgrind): boot_mob 26.74G -> 18.45G (plan) -> 17.05G (cost_lo), p16 18.73G -> 18.33G -> 16.93G.
  Native total_ms, 3 interleaved runs each at one load: boot_mob 1589-1591 -> 1002-1012 -> 919-920 (two_ms 286 -> 82),
  p16 1037-1047 -> 1014-1038 -> 930-936, p10 2107-2111 -> 1726-1740 -> 1724-1729 (two_ms 343 -> 151), gm16 1548-1556 ->
  1499-1504 -> 1416-1422; work counters unchanged. val/plan identical on all seven, also with `PLAN_MEMO_CHECK` (every
  hit recomputed and compared bit for bit); mono output identical; valq 0 mismatches. `PLAN_MEMO=1` prints each plan's
  lookups, hits, refits and clears. tests/plan_memo.rs: none of the seven jobs refits Cont, so the test runs p10 with
  `next.proxy_eff` -1 (pass 1's builders left to their proxies; the pass-2 searches refit one candidate's step 1122.8 ->
  1128.4) under the audit with the cap lifted (`PLAN_MEMO_CAP`), and against `PLAN_MEMO_OFF`. A deliberately stale memo
  (the old Conts' carried across the refit) fails it; at the default cap it would pass, since the pre-refit entries are
  cleared before the rescoring reads them, and the plan's output does not show it either (that candidate is in no
  cheapest trio).
  Tried, not kept: V over the trios memoised per trio set by the c1 bits (one memo per set, a refit's new set a new
  memo). Hits 68-92%, but `trio_v` over at most 4 x 28 trios costs about what hashing nine words does: instructions
  p16 -1.2%, boot_mob +0.1%, wall within noise (boot_mob two_ms 80 -> 98). The combos only change at a refit when a
  candidate's step turns finite or infinite (`combos_of` reads c1 only for that), so a stale set would rarely have shown.
  Left on boot_mob after both (callgrind, 17.05G): `best_sur` and the deck searches under it about 58%, the outcome loop
  itself (score_career's self cost, the table folds `rec`, Cont::plan's lookups) about 25%; search_ms 0.62 of 0.92 s.
- Cont proxy margins recalibrated (owner-approved, 2026-10-09), PROXY_EFF and PROXY_SAVE 0.2 -> 0.06. `proxy_check` on
  the seven val/plan jobs with the own-8/8 credit: the proxy is the exact career for every builder on p16, p16 SS/URA,
  gm16 and boot_mob; worst contender efficiency 2.79% low (p11; p10 0.41%), worst saving 2.67% low (p11; p10 0.14%):
  0.06 is 2.15x / 2.25x (the first calibration: 0.2 = 1.8x an 11% that the merges since have removed). Picks, V, the
  top-8 rows, best-odds and chosen rows, Cont members and the reroll table identical on all seven; `exact_all` still the
  fully exact plan. Exact searches p16 28 -> 10, p10 22 -> 11, p11 38 -> 16, gm16 39 -> 10, boot_mob 21 (one per class);
  user CPU p16 1.73 -> 1.14 s, p10 3.70 -> 2.90 s, p11 3.92 -> 2.90 s, gm16 3.52 -> 1.86 s, boot_mob unchanged. Below
  the shown rows on p10 one builder keeps a proxy career 0.088 lower than her exact one (two rows below the eighth swap).

- Parked (not merged): pass 2 replayed from pass 1's recorded outcomes (`score_replay`, branch
  worktree-agent-a919b16f64bbaea00, commits 23d6968/1ea4bc9): exact, identical on all seven jobs, but 2-7% on plans and
  about 10% on the warm Compare plan for 74 lines and a second scoring path; reconsider inside the consolidation of
  score_career's and outcome_hi's outcome loops. Dropping unshown rows is not possible safely: pass 1 must cover every
  uma (the proxy's admission reads every builder's efficiency) and no cheap valid score bound exists (Cont's cost is a
  minimum over n, not convex).
- State-conditioned bound (prototype of the cap-free bound): at a configuration the chain-kind make-up is fixed, so its
  progress states x are known; given x a deck's exact value is at most the product over targets of their factors at
  their misses (groups whose outcomes each hint one target, or two-target groups that are negatively quadrant
  dependent, are negatively associated), equal without coupled groups; per open slot each target's lowest miss over
  the group's entries at that slot's level bounds every completion (`state_value`, the `sb_ub` closure in `search`).
  Checked (SB_CHECK): value >= exact on every exactly scored deck (pal_A 895, pal_UF 4,529, v_ng_k1 636, p16, gm16; at
  most 0.2% above), node bound >= every leaf below it (gm16 295k leaves, p10 43k): no violation. Used in the career
  band pass at configuration roots (default, `SBOUND=3`): identical outputs on val/plan; Grand Masters nodes 13.2M ->
  5.7M, exact scorings 2,508 -> 1,779, user CPU -24 to -28%; Global within noise. Also at depth 1 (`SBOUND=1`): Global
  +7 to +16% CPU. Without the cap in the band pass (`SBOUND=2`): identical outputs but more nodes (p16 3.4M -> 5.3M)
  and about 10-45x the time: alone it is looser than rho x the product bound inside the tree (a slot may reuse the
  group's best entry; no per-card ratio term) and about 100x a node's cost. Phase 1 and the candidate ceilings still
  assume the cap.

- The worker's kept work survives a page reload (`state_out` / `state_in`, IndexedDB on the career page): kept plans
  (each uma's exact best career per pool; a plan of the same pool now takes them without a search) and kept proofs,
  floats as hex bits. Byte-identical to a cold plan (p16, p10, gm16: in one process and restored into a new one). State
  17.6 KB (p16), 28 KB (p10), 19 KB (gm16); the proofs add about 0.08 s of the gain. Native plan restored: p16 1.69 ->
  1.28 s, p10 7.34 -> 4.97 s, gm16 4.88 -> 3.54 s (the rest of p10's is the loop lookup, which the page keeps itself);
  in the page, the first plan after a reload 2.6 -> 1.3-1.4 s (p16 export, loop members also kept).

- Class-keyed proofs (`seen`, `ruled` per deck class, `Env::cls`): val/plan identical on all six jobs. Nodes (exact
  scorings) p16 39.8M -> 15.9M (2,069 -> 735), p10 34.8M -> 31.4M, p11 21.6M -> 19.5M, gm16 128.5M -> 57.0M (112k ->
  52.8k); native time p16 10.1 -> 5.4 s, p10 15.8 -> 14.8 s, p11 13.3 -> 12.3 s, gm16 44.0 -> 22.8 s (machine shared).
- Planner scoring: the table folds reuse one scratch buffer per level and read cached tables in place (p10 folds 278-311
  -> 150 ms per plan); the outcome loop's `Cont::plan` (10.5M calls on p10, powi/powf/exp per iteration) reads a table of
  its powers and build-cost sums built once per candidate, each setup's spark scale is computed once per landed set, and
  V after her is reused when no candidate's step moves (p10 two deep 1.2 -> 0.36 s, about 1.7 s per plan; p16 about
  0.35 s). Byte-identical. Not done: reusing pass 1's setups in pass 2 (about 90 ms to gain, 31 MB to hold on p10).
- Deck search, byte-identical: per-call setup hoisted, the flat DFS stack, classmates sharing one set id (tables built
  once per class): no measurable CPU gain (the per-node copies the memory review counted were not a cost).
- `upper_na` memoised (pair marginals per key level and pair, the event distribution, each source's group check at
  build time, the last key level): Grand Masters bound 2.82 -> 1.08 s, plan 7.46 -> 5.90 s CPU (-21%); p16 unchanged.
- The plan's loop lookup (the loop planner's own search for the setup, 0.27 s native on p16, 3.6 s on p10) dropped:
  with `loop_members: []` every val/plan job gave the same plan (its members always entered the candidates by the other
  routes); a pool giving fewer than 4 candidates is filled from the planner's own model instead (`hint`: odds under a
  full loop lineage by `Env::ceiling`, no search). Sample mob veterans: same pick, trio and V as with the loop lookup
  (hint Inari One, Tamamo Cross, Nakayama Festa against the loop's Tamamo Cross, Inari One). The page's loop-members
  store (945b557) retired.
- Proxy certification (the page's "Try sample veterans" plan, val/plan boot_mob): with mob veterans every candidate's odds
  are tiny (p about 1.8e-5), a builder's efficiency is about pl/p, and every builder's lift chance is nearly the same,
  so all 64 builders sat within 3.2% of each candidate's second best and the 20% band searched them all (25-30 s native)
  although the proxy was exact for every one (same pair, deck, odds). An admitted proxy is now first certified by her
  class's proofs (ceiling, fully checked decks through rise, product proofs); the first of each class still needs a
  search (17 classes there: only the cap's product proofs bound her, 18-25% above). boot_mob: 68 -> 21 searches, 25.2
  -> 12.0 s, every row identical to exact_all; the six other jobs identical but for the new counter (p11: one builder
  certified, 38 -> 37 searches).
- Tried, reverted: configurations reused by the second pass (hits 472 of 1,163 searches, no CPU gain); bounds restricted
  to the entries still pickable (nodes -9 to -21%, CPU +1 to +22%: the tables cost more than the nodes saved; paying
  would need them cached across calls, estimated hundreds of MB of wasm heap).
- Builders through their deck class (owner decision): proxy careers from the class's winning decks, exact searches only within PROXY_EFF / PROXY_SAVE = 0.2 of what the plan turns on (measured with `proxy_check`: proxy at most 11% low on contenders, 9% on savings near the shown rows, exact on five of six jobs). val/plan: picks, top-8 rows, V and shown scores unchanged; exact careers p16 9/68, p10 20/51, p11 38/51, gm16 10/83; nodes p16 39.8M -> 3.4M, p10 34.8M -> 16.5M, gm16 128.5M -> 13.2M; native time p16 10.1 -> 1.7 s, p10 15.8 -> 7.8 s, p11 13.3 -> 7.5 s, gm16 44.0 -> 5.5 s (machine shared).
- Tried, not kept: solving each class's highest-ceiling trainees first (a pre-pass before the scoring loop): nodes
  p16 -0.6%, p10/p11 0%, URA -1.7%, gm16 0% (under the 3% bar).
- `upper_na` relaxed instead of giving up: a target with three or more feeding chain cards, or two feeders other than
  the one pair, is taken at its feeders' full progress (a valid constant bound; derivation in the code). Grand Masters
  plan (gm16): exact scorings 112,014 -> 17,350, time 44.0 -> 41.5 s (loaded machine; the search's 129M nodes
  dominate there); Global jobs exact -2% to -32%; all six plan outputs identical; `UPPER_CHECK=1` on the Grand
  Masters deck jobs: every scored deck covered, none below exact, bound/exact at most 1.29.
