# Planner speed: measurements and record

The career page's plan (`next` job, `sim::plan::index_rule`; `docs/SOLVER.md`, *Next career*). This file holds what was
measured about where its time goes, what was tried and kept or not, and what is open. The protocol is `docs/PERF.md`'s:
a deterministic reference, byte-identical outputs per item (or a deliberate, measured re-baseline), one item per commit
with its gain. The harness: `val/planval.py` runs the seven next-plan jobs in `val/plan/` (p16 = the owner's 16
veterans, p16 at rank SS and on URA, p10, p11, gm16 = p16 on Grand Masters, boot_mob = the page's sample mob veterans)
and compares each whole output strictly; the plan reports its own work (`next.work`: deck-search nodes, exact
scorings), deterministic and comparable at any machine load. `val/mono.py` checks that a veteran added never raises V.

## Where it stands (2026-10-09)

First plan on the owner's 16-veteran export, in the page: about 12 s (2026-10-08 morning) -> 2.4 s with the Cont proxy
-> 1.9 s cold and 1.4 s after a reload (kept work persisted; measured before the reroll table's own-8/8 credit, which
raised p16's exact builder searches 9 -> 28 until the proxy margins were recalibrated, 28 -> 10). Native plan
(`total_ms`, after the Cont memos): boot_mob 0.92 s, p16 0.93 s, p10 1.73 s, gm16 1.42 s. Sample mob veterans: native
25.2 s -> about 0.92 s. Reroll: about 30 ms of solver per roll; a saving roll's exact check and Compare are one planner
run with her added (warm, native about 0.7-0.9 s).

Merged (byte-identical on val/plan except the deliberate re-baselines: the winners-only cache, the Cont proxy, the
own-8/8 credit): deterministic cache, class-keyed proofs, planner arithmetic (Cont tables, fold buffers), report skip
(`no_reports`), `upper_na` relaxation and memoisation, the Cont proxy at 6% margins, certified proxies, derived stage
candidates (no loop-planner lookup), persistence (IndexedDB `state_in` / `state_out`), the state-conditioned bound at
configuration roots (`SBOUND`; `SB_CHECK` audits it), the reroll table's own-8/8 credit and `y_hi` bound, cap
tripwires, identical deck problems solved once (`Twin`, `deck_memo`), the Cont memos and `cost_lo`'s early exit.
Rerun `next.proxy_check` after any change to how builders' careers are built.

Owner decisions on record: the 1.25 cap stays as the top cap (real ratios 1.18 Global / 1.17 GM; the cap-free bound
is valid but 13-45x slower as a replacement, and a per-era cap would miss a deck above it silently); per-class caps
below it (2026-10-09, Done below); parallel helper workers are a
last resort (2026-10-08: meaningful gains first; parallel workers hide work rather than remove it; design in the
page-side agent's report: deterministic only with the winners-only cache and exact, unbudgeted searches).

## Open

- Reroll (2026-10-09, branch reroll-memos-on-caps, on main 6aed843 with the per-class caps). What a roll runs: the
  quick comparison (a career job, `fix` her) always; the plan with her added (the exact check) when the table's y >= 0.5
  or, after 800 ms, when y_hi >= 0.5. Chance a roll runs the plan: p16 37% (6.1% saving + 30.7% provisional), gm16 37%,
  p10 100% (52.6 + 47.4), p11 100%, boot_mob 100%. Callgrind per roll (before the caps, line-table build): p16 quick
  0.12G, saving plan 8.30G, provisional plan 6.61G; p10 quick 7.42G / 3.89G, plans 16.65G / 11.40G. In a p16 saving plan:
  deck searches 3.80G (16 proxy searches from `best_sur`, 2.47G), `score_career` 3.56G (190 calls, 18.7M each; its fold
  0.47G, Cont::plan 0.36G, trio_v about 0.31G), corner tables 0.27G. A provisional plan repeated every one of the base
  plan's proxy searches key for key. Kept: `best_deck`'s memo (replaying cert, band overflows, nodes, exact scorings and
  the per-class ratio and deck) and the corner tables, kept between jobs of one setup (`DECK_KEEP`). On main (release
  build, callgrind): p16 run's plan 9.10G -> 8.91G (-2.1%), saving-roll plan 8.38G -> 8.06G (-3.8%), provisional-roll
  plan 6.65G -> 4.33G (-35.0%); p10 (before the caps) saving -24.6%, provisional -36.8%; the quick job unchanged.
  Identical whole, work counters and per-class fields included: the Reroll sequences on p16, p10 and boot_mob (native;
  wasm before the caps), val/plan 7/7 against main; full val 0 mismatches; mono 0 rises.
- Consolidate `score_career`'s and `outcome_hi`'s outcome loops (one loop, estimate and bound; y byte-identical); the
  parked pass-2 replay (Done, below) may fold in there.
- Left on boot_mob after the memos (callgrind, 17.05G instructions): `best_sur` and the deck searches under it about
  58%, the outcome loop itself (score_career's self cost, the table folds, Cont::plan's lookups) about 25%; search_ms
  0.62 of 0.92 s. A skip-ahead in `Cont::plan` (x_n is convex in n analytically) would need a float-exact argument for
  its first stop.
- Not tried, estimates only: a node-marginal ratio bound at depth <= 2; the deck search's per-leaf `dw`/`dg`
  allocations and the band test's second evaluation of the same clause value (estimate 3-15%); reusing pass 1's setups
  in pass 2 (about 90 ms to gain, 31 MB to hold on p10); smoothing p_inf over the top 3 builders (the argmax makes
  p_inf jump when near-tied builders swap).
- Page: keep the worker's kept bests across a superseded job (a settings change during a plan discards them, as of
  2026-10-08); reuse kept plans for pools that grow by several veterans or lose one (today: the same pool, or one more
  veteran at the end).

## Baseline: where the time went (2026-10-08, before this work; native, p16)

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

The cap: rho = 1.25 (assumed, `EXACT_RHO`; on the planner jobs the largest exact/product ratio scored was 1.03-1.04, see *The cap*).
Builder time at rho 1.25 / 1.10 / 1.02: 4.4 / 2.1 / 1.1 s. At 1.02 one career changed (King Halo, 1 in 689 -> 691.7: a
deck with a ratio above 1.02 was skipped), so lowering the cap is an assumption change, not a speed-up.

## The cap (2026-10-08)

No data-free proven cap exists: a target whose only source is one card's last link makes exact/product = 1/P(link)
(about 6.7 with two such targets); real targets have floors (inheritance, white generation), but range-only and Hölder
caps still come out at 1.2-1.7. Measured: 1.18 on the Global deck job pal_UF (pal_I 1.165), 1.17 on Grand Masters; the
planner jobs' scored decks 1.03-1.04 (p16, p10). So 1.25 has about 6% of headroom on both eras. Guards (done): the
planner reports `next.work.max_ratio`, and `val/planval.py` / `val/cmp2.py` fail when a scored deck comes within 5% of
the cap; they cannot see pruned decks, hence the planned build-time audit (rerun near-best decks with the cap at 2.0).
The replacement that needs no cap: at a search node the chain-kind make-up is fixed, so the distribution of chain
progress states is known; given a state, targets are independent apart from exclusive outcomes (which only lower the
value), so the average over states of each target's best value given the state bounds every completion's exact value,
for any card data. Built (Done: state-conditioned bound): used as an extra prune at configuration roots; as a
replacement for the cap it is 10-45x slower.

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
| Configurations reused by the second pass | hits 472 of 1,163 searches, no CPU gain | no |
| Bounds restricted to the entries still pickable | nodes -9 to -21%, CPU +1 to +22%: the tables cost more than the nodes saved; paying would need them cached across calls (estimated hundreds of MB of wasm heap) | no |
| Solving each class's highest-ceiling trainees first (a pre-pass before the scoring loop) | nodes p16 -0.6%, p10/p11 0%, URA -1.7%, gm16 0% | no (under the 3% bar) |
| Pass 2 replayed from pass 1's recorded outcomes (`score_replay`) | exact, identical on all seven jobs, but 2-7% on plans and about 10% on the warm Compare plan for 74 lines and a second scoring path | parked on branch worktree-agent-a919b16f64bbaea00 (23d6968, 1ea4bc9); reconsider inside the outcome-loop consolidation |
| wasm SIMD (`+simd128`, 2026-10-09) | byte-identical outputs; node (V8), 3 interleaved fresh-process runs: p16 1,713 -> 1,683 ms avg (-1.7%), boot_mob 1,571 -> 1,630 ms (+3.8%), both within noise | no: the hot loops (node bound, outfit_fast) are branchy with serial products, nothing autovectorises; recheck once outfit_fast is branch-free |

| `outfit_fast` with u8-state `if` selects (2026-10-09) | LLVM re-emits branches; -0.4 to -1.4% | no (`OutfitPre`'s u64 masks vectorise) |
| `outfit_fast` with f64-state compares (2026-10-09) | not vectorised; +0.6 to +0.8% | no |
| A/B pre-check at the root bound (2026-10-09) | its B side is `p0 * sp0`, no outfit call | nothing to save |
| mimalloc as the native allocator (2026-10-09) | identical outputs; user CPU, 3 interleaved runs: p16 0.97 -> 1.00 s, p10 noise, boot_mob 1.25 -> 1.14 s (spread 0.98-1.28), gm16 1.83 -> 1.80 s, pal_A2 1.34 -> 1.39 s | no: within noise; cachegrind put allocation's RAM misses at most about 7% of time un-overlapped |
| talc as the wasm allocator (2026-10-09) | identical outputs; node, 3 interleaved runs: p16 1,386 -> 1,327 ms (-4.3%, every run faster), boot_mob 1,414 -> 1,413 ms; wasm 1.84 -> 1.97 MB | no: 4% on one pool for +7% download; retry if allocation still shows after the table deferral |

| Incremental trio V: only trios reading a changed step cost, min with V at the base (2026-10-09) | exact; +218M instructions on p16 against `trio_v`'s 290M: nearly every trio reads the changed candidate | no |
| `trio_v` over trios sorted by C's step, stop once c3 >= best (2026-10-09) | exact with positive-signed costs; +115M on p16: C's step is small next to A's and B's, the stop rarely fires | no |
| Skip `hopeful` at the final slot for single-trainee clauses (2026-10-09, reasoning, not measured) | after the per-parent factor reuse, `hopeful`'s A costs less than the leaf's own outfit evaluation, and the skip would send every child it rejects to the leaf | no |

## Findings from code (reasoning agents, 2026-10-08)

- A proven cap of 1 exists only for decks with no target-bearing chain card (negative dependence of one event's
  outcomes; derived), which is almost no deck. No general cap-free branch bound is valid: one chain card feeding two
  targets breaks a per-target mixture bound, and a later card can become a second feeder below a branch.
- Plan outputs can depend on search history: the planner's cache took every deck a search happened to check fully
  (`Cache::absorb`), and the trios' third members, candidates' lifted odds, Cont and V read the best of them. Pruning,
  the per-setup proofs kept between jobs (SEEN) and class sharing change which decks those are. Equal outputs in the
  experiments above were not guaranteed. Fixed: the cache holds exact winners only (Done, below).
- Exact skipping of builders is blocked by Cont (the most efficient builder over every uma, efficiencies within a few
  per cent): about 45-55 of 60 exact searches would remain (estimate). A cheaper Cont is a model decision.
- `rise(t, q', q)` and every proof (`Ref`, `ruled`) depend on the trainee only through her class: sharing is exact.

## Page JavaScript (separate from the planner's time)

The page's own work is milliseconds against seconds of solving. User-visible: the status spinner is rebuilt (and its
animation restarted) every second; every filter keystroke rebuilds the whole roster or card list (filter by `hidden`);
the card count is O(n^2) and goes stale after a toggle; `renderPool` redraws the closed veterans dialog on every event;
`fitSparks` forces a layout per step and stacks on resize. Transfer: nothing is minified (about 25 KB per page,
estimate); CSS and the shared data are inlined into each page and re-fetched after the 5-minute HTML cache. Code
health: `el`/`fmtIn`/backdrop-close duplicated across modules; career.js (about 1,000 lines) mixes job building, the
plan/compare/reroll state machines, views and import/export.

## Done
- Reroll's bound `y_hi` tightened (2026-10-09, owner-approved; still an upper bound on the replan's direct-lift
  saving; channel 2 stays out). Measured first on p16 (16,384 outcomes; the provisional band sampled by outcome
  probability, each partner's best career with her searched exactly): every sampled band outcome had an exact direct
  saving of 0, while y_hi read a median 36 careers. The slack, by source: the ceiling took `fastc`'s factor as
  A + B q' where `outfit_fast` has A - B (1 - q') (valid, B too high per target: 3.3 times the true ceiling at the
  pick's partners); `rise` ignores targets whose chance falls (her slot in place of a strong veteran), and the anchors
  (y, z) price every pair at p_now; `cost_lo` against `cost` and the rounding to 0.01 moved nothing measurable; partners
  beyond IDX_PARTNERS moved nothing (the band comes from the best two). (1) The ceiling's factor fixed: provisional
  (the chance a roll runs the full plan for a "no saving" check) p16 0.307 -> 0.156, gm16 0.311 -> 0.148, p16_ss
  0.401 -> 0.238, p10 0.474 and p11 0.028 unchanged; no added work. val/plan: only `y_hi` and `provisional` move.
- Per-class exact caps (2026-10-09, owner's call; outputs identical on the seven val/plan jobs, mono and the full val
  apart from cap fields and counters; the budget ladder now certifies higher caps on five val jobs). The cap on exact /
  product odds by the deck's chain-varying targets: 0-2 1.05 (assumed), 3+ or a multi-target group 1.25; a node
  takes the class of everything it can still reach. Callgrind gm16 19.458G -> 14.098G (-27.5%: nodes 5.71M -> 3.58M,
  exact scorings 919 -> 311), p16 9.873G -> 9.935G (+0.6%: its decks are mostly three-target, where the cap stays 1.25;
  nodes -0.7%); v_uc_k1 per-uma wall 1.79 -> 1.20 s (best of 3, interleaved, CPU clock-throttled at the time).
  Prototype at 1.10 for three targets (p16 1.39 -> 1.29 s) dropped: boot_mob scores a three-target King Halo deck at
  1.1265. `val/capaudit.py` (every cap at 2.0; 37 s): answers identical on p16, gm16, v_uc_k1 per-uma; no class 0-1
  deck is scored on any of them. With it, a career band over EXACT_CAP decks is rebuilt down the cap ladder instead of keeping
  the product-best deck unreported (the 27-card GM subset of gm16, order 2: no longer overflows at all).
- Exact batch (2026-10-09, merged at e674119; identical on all seven val/plan jobs after each item, work counters
  included; valq 0 mismatches; cargo test passes). Callgrind p16 / boot_mob 11.81G / 11.73G -> 9.87G / 10.26G
  (-16.4% / -12.5%); p16 wall at load 3: 1.42-1.47 -> 1.29 s.
  - `score_career`'s outcome loop: flat leaf-major folds and m-major spark scales with a per-setup candidate index;
    thresholds once per call; only candidates with setups tested; `up` and `c1` restored where changed instead of
    copied. p16 -3.8%.
  - Node bound: a child's untouched targets (miss exactly 1.0, per-entry masks cached with the key's miss tables) keep
    the parent's A factors bit for bit; factors once per parent and borrowing variant; a child recomputes only its
    touched targets (1.25 of 6 on p16, 8 children per parent); B's inputs only when B runs. `OutfitPre::eval`
    985M -> 144M; p16 -2.6%.
  - DFS miss rows in a per-configuration buffer, one row per depth, instead of push/truncate: p16 -4.5%.
  - `outcome_hi` leaf products level by level (`leaf_products`, the same left-to-right products): p16 -2.0%.
  - `Env::slots`: grandparents' race parts per legacy in `Pool.gpc`, parents' shared wins once, stars and pinks shared
    through `Rc`: p16 -2.4%.
  - Allocations: `ceiling` borrows `dmax`; `set_trainee` in place; `her_trios` push/pop on one pool;
    configuration-build scratches; `make_rpk` flat scratch; no one-borrowed tables without borrowing; reused
    per-configuration buffers. p16 -2.4%.
  - `upper_na` reads `Src::fm` instead of `group_gh` per call: gm16 -4.3%, p16 -0.1%.
- Deck search node bound, four exact items (2026-10-09, branch merged as e3c2303; same outputs and work counters on all
  seven val/plan jobs after each, valq and full val clean). Callgrind instructions p16 / boot_mob / pal_A2 (loop planner,
  per-uma): 16.57G / 17.05G / 22.19G before.
  - A/B pre-check in `hopeful`: the per-target values A first; if the clause sums with A alone fail, no B
    (`min(A, B r) <= A`); otherwise B only for outfits whose A is above the running max. -6.4 / -9.6 / -11.6%.
  - `OutfitPre`: the node and root bounds' outfit products through prepared rows (masks, 1 - ew), a bitwise select,
    a vectorised factor loop, the product in target order (unit test with non-finite inputs). -1.8 / -2.4 / -2.6%.
  - Configuration suffix tables built only when the DFS expands the configuration (flat, reused); the root bound's
    sp[0] computed directly in the same order; one-borrowed tables only when used. -3.0 / -2.0 / -2.1%.
  - Node-bound constants per (group, cards left, borrowing), tabulated lazily per configuration (the crate audit's
    prototype). -7.8 / -11.8 / -25.8%.
  - Totals 13.62G / 13.01G / 13.87G; wall (best of 3) p16 1.14 -> 0.92 s, boot_mob 1.05 -> 0.86 s, pal_A2 1.30 -> 0.88 s.
- `score_career`'s table fold (2026-10-09, b84526c): x and 1 - x per target and star level once, the last two levels
  unrolled; fold 2,124M -> 469M instructions, about -10% of p16; `trio_v` as a plain minimum (-0.9%).
- With both: native p16 0.79 s, p10 1.80 s, gm16 1.40 s, boot_mob 0.77 s (val/plan, one at a time).
- Deterministic cache, winners only (each search's best deck; before, the planner's cache took every deck a search
  happened to check fully). Moved results once: p16 candidates' stage costs +1-4%, p10's member lift 0.565 -> 0.526 and
  careers to loop 1044 -> 1058; URA and Grand Masters unchanged; picks and top five unchanged on all six jobs then. The
  loss is fidelity: a candidate's lifted odds are the best of her cached decks' tables at the new chances, and one deck
  reads lower than several. A variant (the main search without earlier jobs' proofs feeding the cache, plus every other
  search's winner) gave the same numbers on all five Global jobs. If the candidates' fidelity matters later: each
  candidate's top 3 careers' decks (deterministic, about 7 cheap searches).
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
  p16 -0.7%, gm16 0). val/plan identical on all seven, valq 0 mismatches, the five career jobs identical whole. What
  was left then was the per-outcome planner arithmetic (`score_career`'s outcome loop, 1.4 s against p16's 0.4 s at
  equal load): with candidates at p about 1.8e-5 every outcome of every builder lifts a candidate, so nearly every
  outcome pays `Cont::plan` and a trio V; the Cont memos (next entry) took most of it.

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
- Cont proxy margins recalibrated (owner-approved, 2026-10-09), PROXY_EFF and PROXY_SAVE 0.2 -> 0.06. `proxy_check` on
  the seven val/plan jobs with the own-8/8 credit: the proxy is the exact career for every builder on p16, p16 SS/URA,
  gm16 and boot_mob; worst contender efficiency 2.79% low (p11; p10 0.41%), worst saving 2.67% low (p11; p10 0.14%):
  0.06 is 2.15x / 2.25x (the first calibration: 0.2 = 1.8x an 11% that the merges since have removed). Picks, V, the
  top-8 rows, best-odds and chosen rows, Cont members and the reroll table identical on all seven; `exact_all` still the
  fully exact plan. Exact searches p16 28 -> 10, p10 22 -> 11, p11 38 -> 16, gm16 39 -> 10, boot_mob 21 (one per class);
  user CPU p16 1.73 -> 1.14 s, p10 3.70 -> 2.90 s, p11 3.92 -> 2.90 s, gm16 3.52 -> 1.86 s, boot_mob unchanged. Below
  the shown rows on p10 one builder keeps a proxy career 0.088 lower than her exact one (two rows below the eighth swap).

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
- Builders through their deck class (owner decision; margins 0.2 then, recalibrated to 0.06 in the entry above): proxy careers from the class's winning decks, exact searches only within PROXY_EFF / PROXY_SAVE = 0.2 of what the plan turns on (measured with `proxy_check`: proxy at most 11% low on contenders, 9% on savings near the shown rows, exact on five of six jobs). val/plan: picks, top-8 rows, V and shown scores unchanged; exact careers p16 9/68, p10 20/51, p11 38/51, gm16 10/83; nodes p16 39.8M -> 3.4M, p10 34.8M -> 16.5M, gm16 128.5M -> 13.2M; native time p16 10.1 -> 1.7 s, p10 15.8 -> 7.8 s, p11 13.3 -> 7.5 s, gm16 44.0 -> 5.5 s (machine shared).
- `upper_na` relaxed instead of giving up: a target with three or more feeding chain cards, or two feeders other than
  the one pair, is taken at its feeders' full progress (a valid constant bound; derivation in the code). Grand Masters
  plan (gm16): exact scorings 112,014 -> 17,350, time 44.0 -> 41.5 s (loaded machine; the search's 129M nodes
  dominate there); Global jobs exact -2% to -32%; all six plan outputs identical; `UPPER_CHECK=1` on the Grand
  Masters deck jobs: every scored deck covered, none below exact, bound/exact at most 1.29.
