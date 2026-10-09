# Planner speed: measurements and plan

The career page's first plan (`next` job, `sim::plan::index_rule`) takes about 12 s in wasm on the owner's 16-veteran
pool; this file holds what was measured about where that time goes, what was tried, and the plan of attack. The
protocol is `docs/PERF.md`'s: a deterministic reference, byte-identical outputs per item (or a deliberate, measured
re-baseline), one item per commit with its gain.

## Status (2026-10-08, evening)

The career page's first plan on the owner's 16-veteran export: about 12 s in the morning, 4.2-4.4 s after class proofs,
the planner arithmetic and the report skip, 2.4 s with the Cont proxy (9 of 68 umas searched exactly on p16). Earlier:
4.2-4.4 s after the merged
items below ("Planned in", Chromium, two runs; the main search adds 0.3 s). Native val/plan: p16 10.1 -> 2.9 s, p10
15.8 -> 8.2 s, p11 13.3 -> 6.8 s, Grand Masters 44.0 -> 13.8 s (the before runs were on a busier machine; nodes p16
39.8M -> 15.9M, gm16 128.5M -> 57.3M are load-free). All byte-identical after the one deliberate cache re-baseline.

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

- Class-keyed proofs (`seen`, `ruled` per deck class, `Env::cls`): val/plan identical on all six jobs. Nodes (exact
  scorings) p16 39.8M -> 15.9M (2,069 -> 735), p10 34.8M -> 31.4M, p11 21.6M -> 19.5M, gm16 128.5M -> 57.0M (112k ->
  52.8k); native time p16 10.1 -> 5.4 s, p10 15.8 -> 14.8 s, p11 13.3 -> 12.3 s, gm16 44.0 -> 22.8 s (machine shared).
- Planner scoring: the table folds reuse one scratch buffer per level and read cached tables in place (p10 folds 278-311
  -> 150 ms per plan); the outcome loop's `Cont::plan` (10.5M calls on p10, powi/powf/exp per iteration) reads a table of
  its powers and build-cost sums built once per candidate, each setup's spark scale is computed once per landed set, and
  V after her is reused when no candidate's step moves (p10 two deep 1.2 -> 0.36 s, about 1.7 s per plan; p16 about
  0.35 s). Byte-identical. Not done: reusing pass 1's setups in pass 2 (about 90 ms to gain, 31 MB to hold on p10).
- Deck search memory traffic: the flat miss stack (no 32-wide copies per node) and per-leaf scratch: identical, no
  measurable gain (the copies were not a cost); per-call setup hoisted: configuration building is about a quarter of
  deck-search time (p16), the hoists within noise.
- Builders through their deck class (owner decision): proxy careers from the class's winning decks, exact searches only within PROXY_EFF / PROXY_SAVE = 0.2 of what the plan turns on (measured with `proxy_check`: proxy at most 11% low on contenders, 9% on savings near the shown rows, exact on five of six jobs). val/plan: picks, top-8 rows, V and shown scores unchanged; exact careers p16 9/68, p10 20/51, p11 38/51, gm16 10/83; nodes p16 39.8M -> 3.4M, p10 34.8M -> 16.5M, gm16 128.5M -> 13.2M; native time p16 10.1 -> 1.7 s, p10 15.8 -> 7.8 s, p11 13.3 -> 7.5 s, gm16 44.0 -> 5.5 s (machine shared).
- Tried, not kept: solving each class's highest-ceiling trainees first (a pre-pass before the scoring loop): nodes
  p16 -0.6%, p10/p11 0%, URA -1.7%, gm16 0% (under the 3% bar).
- `upper_na` relaxed instead of giving up: a target with three or more feeding chain cards, or two feeders other than
  the one pair, is taken at its feeders' full progress (a valid constant bound; derivation in the code). Grand Masters
  plan (gm16): exact scorings 112,014 -> 17,350, time 44.0 -> 41.5 s (loaded machine; the search's 129M nodes
  dominate there); Global jobs exact -2% to -32%; all six plan outputs identical; `UPPER_CHECK=1` on the Grand
  Masters deck jobs: every scored deck covered, none below exact, bound/exact at most 1.29.
