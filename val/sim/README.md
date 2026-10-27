# The policy simulator's experiments

The simulator (`simulate` on a career job: `docs/SOLVER.md`, *Policy simulator*) measures, under the solver's model,
the expected number of careers E to a goal, for a decision policy on a fixed starting pool. What E counts depends on
`goal`:

- `first` (the default): careers until the first veteran that lands every target. This is greedy's goal, and the one
  every reference in this folder before goal `looped` measured.
- `looped`: careers until the first fully looped parent (a veteran that lands every target on two full-veteran
  parents), the goal of the page's plan (`index`). Careers on two full veterans keep their success analytic; the
  others' all-target veterans are drawn and join the pool. A run stopped by the cap is priced by repeating the last
  career on two full veterans it ran (`censored` when it ran none), so set `max_careers` to at least 3x the pool's V.

Policies: `greedy`, `greedy_exact`, `loop` (the baselines a new policy is compared against), `trace` and `index` (the
page's planner, `career::plan::index_rule`, run each step as the page's plan job); adding one: `docs/SOLVER.md`, *Policy
simulator*. How closely the surrogate (`greedy` at a margin) follows the exact model (`greedy_exact`, `loop` at margin
-1) was measured on 2026-10-07 under an older model; it is to be re-measured at integration (a pool14 `greedy` against
`greedy_exact` over 200 careers, about 5 minutes), and no figure for it is stated until then.

## The continuation's shape (`Cont`)

`CEIL_SHARE` 0.6 and `LAM_DECAY` 1.5 (`src/career/plan/cont.rs`) were fitted on 2026-10-08 to the `trace` policy's
building for Agnes Digital on four player pools of 2, 5, 11 and 19 veterans, under the model of that day. Those pools
are not in the repository. `fit_cont.py` refits the shape on the pools in `trace_pools.json` (val/plan and val/sim
pools, a 2-veteran one cut from pool4), building for each pool's two stage candidates rather than one uma. It fits the
shape to the traces' log odds and, apart, to the step's cost (what the plan reads), and scores each pool under the
shape fitted without it (log-odds loss and cost error against the pool's achievable cost, split in half since the
in-sample minimum over noisy means reads low), reports the plan's own fitted continuation apart from
the one fitted to the traces, and prices the unbuilt variant that takes lam as the larger of now and one lift later.
A builder's veteran in a trace is drawn from every landed set (her full veteran does not end the building), so traces
taken before 2026-10-26 drew differently. It runs after every change that moves Cont or V.

    cargo build --release --manifest-path pkg/loopopt/Cargo.toml
    uv run val/sim/fit_cont.py pkg/loopopt/target/release/loopopt /tmp/fit        # about 10 minutes on two cores
    uv run val/sim/fit_cont.py pkg/loopopt/target/release/loopopt /tmp/fit0 --margin 0 p10 p11   # the cached decks' share

Run on 2026-10-26 on the planner as changed that day (9 pools, 2 trainees each, 6 runs of 400 careers): fitted to the
odds path the shape runs to the grid's edge (`CEIL_SHARE` 1.2, `LAM_DECAY` 3.0) and prices the cost worse held out
(mean |cost error| 18%); fitted to the cost, 0.55 / 1.0 (leave-one-pool-out 0.5-0.65 / 0.6-1.3), held-out mean
|cost error| 10.8% against 10.5% for the current 0.6 / 1.5, which were kept. Per trace the current shape's cost error is
-13% to +34% (too pessimistic on most pools where building pays, p10's Agnes Digital +34.5%; exact on p16 and gm16).
The lam = max(now, one lift later) variant changed no trace's cost.

`fit_cont.py` also reports, not fitted on, the value of a lift: the plan's Cont's K(p0) - K(p') at m = 1, 2, 4 of its
first lifts beside the traces' segment estimator (each run's first arrival at those odds, the open-loop cost from there
against that from the start; no pooling by odds, no bandwidth), and their paired difference with a bootstrap 90%
interval over runs (each draw recomputes the segment and Cont's dK at its own arrival odds; the count of failed
draws, under 3 arriving runs or a horizon under 50, is printed, and the interval is conditional on the rest). On the
cont review's 2026-10-26 traces (`--fit-only`, 14 building traces on 7 pools): all 6 runs arrive in every cell of the
building traces that has a segment, so no draw failed there (the one short cell is p16 Agnes Digital's at m = 4, 5 of
6 arriving, 1 of 300 draws failed); the difference's
interval lies below 0 on 2, 10 and 6 of them at m = 1, 2, 4, contains 0 on 12, 4 and 5, lies above on 0, 0 and 2
(pool14's two at m = 4). p11 Taiki Shuttle has no m = 4 segment: 1 of 6 runs arrives, with a horizon of 12. The four grinding traces of p16 and gm16 differ by
at most 3 careers. At one lift, the scale a single builder career moves, six runs do not resolve the slope: Cont's
dK reads low at two and four lifts, not resolved at one.

`lam_regress.py` tests `LAM_DECAY` on val/mono.py's plans (no plan of its own): where a rolled veteran raised a
candidate's odds from p0 to p', the refit's lift chance against the anchored one, lam0 g(p')^LAM_DECAY. On 2026-10-26
(the integration's mono jobs, re-run with the binary of that morning: 50 candidates on 8 pools) the implied decay was
0.64 [0.42, 0.87], but 2.37 [1.64, 3.10] without p16_borrow, whose third roll moved its candidates furthest (g down to
0.12); every other pool left out keeps it near 0.6. LAM_DECAY 1.5 is outside both, on opposite sides: the refits' lift
chances are not a function of the remaining gap alone, so the anchored form does not describe how the plan's own lam
moves (refit / anchored lam per candidate: median 1.05, range 0.31-8.3).

## Policy comparisons

The greedy and loop policies (pools: `val/jobs/career_U5.json` and `val/sim/pools/`), run from the repository root:

    cargo build --release --manifest-path pkg/loopopt/Cargo.toml
    uv run val/sim/make_jobs.py /tmp/sim                         # job files and queue.txt
    (cd pkg/loopopt/data/global && for n in $(cat /tmp/sim/queue.txt); do ../../target/release/loopopt data.json /tmp/sim/$n.json > /tmp/sim/$n.out; done)
    uv run val/sim/report.py runs /tmp/sim/u5_*.out              # E per policy, paired differences
    uv run val/sim/report.py audit /tmp/sim/u5_*.out             # the surrogate's fidelity audits

The exact-model trajectories (`*_gex`, `*_lx`: a career search per decision) take the longest.

## The page's planner on its own goal

`index` against `greedy` (restricted to looped careers once two full veterans make one) and the plan's own trio run as
`loop`, all under goal `looped`, on the same draws (run r faces the same uniforms under each policy, so differences are
paired by run; the coupling is not monotone, so the pairing lowers the variance by an unknown amount):

    uv run val/sim/make_jobs.py /tmp/lp --looped --runs 4 p16 p11   # queue_looped.txt; max_careers 3x the reference V
    (cd pkg/loopopt/data/global && for n in $(cat /tmp/lp/queue_looped.txt); do ../../target/release/loopopt data.json /tmp/lp/$n.json > /tmp/lp/$n.out; done)

The statistic is the paired D = E_index - E_baseline per run, as D / V with a t interval. A difference matters at
about 10% of V, since Cont's shape alone moved V by up to about 30% on the building pools (p10, p11, boot_mob) over a grid of
`CEIL_SHARE` 0.4-1.0 and `LAM_DECAY` 1-3, on the solver before 2026-10-26. How many runs that takes is unknown:
review estimates of sd(D) without keeping the finishing careers' success analytic were 1-1.4 V, about 1,500 runs per
pool; keeping it analytic lowers that by an unmeasured amount. Run the 4-run pilot first and size the full run from its
sd(D). A plan costs about 0.6-5 s natively on the val/plan pools of 8 targets and 12.5-14.4 s on p16_t10 (10 targets; planval at integration, 2026-10-27), and its cost at the hundreds of veterans a long run builds has
not been measured: time one plan on a pool grown by a pilot before sizing. The pick between near options cannot be
tested this way at any affordable size (score gaps of 0.06-0.2% of V on val/plan before the 2026-10-26 plan changes);
only whole policies can, or a first step against one continuation (below).

`index` adds per run the plans' count, their seconds, the picks' roles and how many were the best-odds career; under
goal `looped` every run adds `finishing` (careers run on two full veterans), `full_veterans` and `censored`.

`simulate.act_on: "pair_search"` makes `index` run the page's final recommendation instead of the plan's pick: each
plan job also runs the page's whole check and pair search, and the career `pairs.recommendation` names runs with its
own deck (docs/SOLVER.md, *Policy simulator*; `tests/sim_pair_search.rs` checks the first career against the page's
own job sequence on three pools). Its cost grows with the pool, since the pair search scores every runnable pair of
the pick's and the tied trainees: one p16_borrow plan job with the check costs 4.6 s at the pool's 17 legacies (1.5 s
without), 11.3 s at 57 and 27.0 s at 116 (1.7 s and 1.9 s without; pools grown by a greedy run, 2026-10-26), against a
mean 0.48 s per plan inside a default index run there (kept plans make the plan cheaper; the check is not kept between
steps).

## Paired truncated rollouts (a Bellman-consistency check of V under greedy)

`rollouts.py` forces each of a pool's top rows as the first career (`simulate.force`), continues with greedy under goal
`looped` for 50 careers in all on common draws, and values each run's end by the plan's own V of its final pool
(`simulate.keep_pool`), weighted by the run's survival. The paired difference T(row) - T(pick) is compared with
score(pick) - score(row). One command reruns it (`uv run val/sim/rollouts.py BIN OUT_DIR --runs N [POOL ...]`); a run
costs about 3 plans per pool.

What it can and cannot see: in the measurement below no run of any row on any of the four pools finished within its 50
careers (every run's survival is 1 and none ran a career on two full veterans), so every run's T is exactly
50 + V(final pool). The draws of the 50 careers are the simulator's, so the check can fail: it tests whether V after
50 greedy careers agrees with V and the scores at the start (Bellman consistency of V under one continuation), but both
terms it compares are V's. It cannot give V's level or the true cost of finishing, and it tests the first step's
ranking only as V itself values the pools that follow, not against what playing on costs. V's level against a feasible
policy is `v_level.py`'s (below).

First measurement (2026-10-26, a binary built from the main checkout's source before the plan fixes of the final round:
the plan's scoring as it stood then; about 13 minutes of CPU):

| pool | runs | row (score) | T(row) - T(pick) [95%] | score(pick) - score(row) |
|---|---|---|---|---|
| p10 | 10 | Gold City, any outfit (4.50) | +6.7 [-11.4, +24.7] | +0.82 |
| p10 | 10 | Maruzensky, any outfit (4.35) | +7.4 [-10.3, +25.1] | +0.97 |
| p11 | 20 | Yamanin Zephyr [Fluttertail Spirit] (4.08) | +7.4 [-17.2, +32.1] | +0.08 |
| p11 | 20 | Maruzensky, any outfit (4.00) | +5.2 [-6.7, +17.1] | +0.15 |
| boot_mob | 50 | Agnes Digital [Full-Color Fangirling] (547.1) | -179.1 [-362.4, +4.2] | +11.9 |
| boot_mob | 50 | Yukino Bijin, any outfit (543.6) | +27.8 [-11.1, +66.7] | +15.4 |
| p16 | 20 | Inari One [Edomurasaki] (1.24) | 0.0 exactly | +0.74 |
| p16 | 20 | Tamamo Cross [Fast as Lightning] (0.66) | 0.0 exactly | +1.32 |

The picks: Agnes Digital [Full-Color Fangirling] on p10 and p16, Gold City (any outfit) on p11, Oguri Cap (any outfit)
on boot_mob. No interval excludes the score difference, so nothing here contradicts the ranking; nothing confirms it
either. At this size a paired difference resolves about 10-20 careers on p10 and p11, where the score gaps are under
one career; on p16 the rows' savings come almost entirely from the all-landed outcome (P(all) about 0.004), which no run
drew, so every run's T is the same. boot_mob, the one pool where score gaps are large, puts Agnes Digital's row 179
careers ahead of the pick, an interval that just reaches 0: the place to spend more runs.

The pick's mean T sits below V - score on every pool, outside its 95% interval on three (p10 2,457 [2,257, 2,657]
against 2,732; p11 1,870 [1,812, 1,928] against 1,956; boot_mob 1,470 [1,322, 1,618] against 2,550; p16 648 [586, 710]
against 693): 50 greedy careers and then the plan's own V cost less than the plan's V says at the start. So V is not
a fixed point of its own continuation; unless it reads low at the pools the runs end with, it reads high at the start
by about those gaps.

## V's level against complete greedy runs

`v_level.py` plans a pool for V, then runs greedy to the end under goal `looped` (seed 1, `max_careers` 3 x V, `eps`
0.5, exact for greedy: once two full veterans make a looped career its pick is frozen, so the tail S / p is what greedy
then does) and reports V - E_greedy with its 95% interval. Greedy is a feasible policy, so V above it means V reads high
by at least the gap. `--index N` adds N runs of the plan's own policy paired with greedy's runs of the same index
(greedy's mean plus the paired mean of D = E_index - E_greedy; its interval is approximate, a t on the paired runs'
degrees of freedom). It tests neither Cont nor the pick: run it on pools
whose greedy runs finish (a real full veteran), where it places V's level and nothing else.

    uv run val/sim/v_level.py pkg/loopopt/target/release/loopopt /tmp/vl --runs 1000 p16_borrow   # 37 s of CPU
    uv run val/sim/v_level.py pkg/loopopt/target/release/loopopt /tmp/vl --runs 1000 --index 5 p16_borrow   # + 5 index runs, 50-160 s each

Run on 2026-10-26 (a release binary whose plan reproduces val/plan's reference V to the bit): p16_borrow V 212.52, greedy 198.1 [192.2, 204.0] over 1,000 runs (none
censored), V - greedy +14.4 [+8.5, +20.3] careers: V reads 7.3% high against a policy the player can run (z 4.8). Runs
0-199 reproduce the review's 200-run measurement (199.33) to the last digits. p16_borrow's one building side (Agnes
Digital) builds 1.8 of her 121.7 careers and is an Alternative side, so this says nothing about Cont.

`pools/boot_build.json` is the pool for testing Cont this way: boot_mob's six mob veterans plus p16_88's El Condor Pasa
full veteran (one-star sparks but 21010 at three, on partial parents), targets less 20203. Planned 2026-10-26: V 334.29
= El Condor Pasa's real full veteran (0) + Inari One's side 145.34 + Agnes Digital as C 188.95, Inari One's side
`Current` (not alternative), building 13.9 of her 145.3 careers against 1,103 grinding at once (Cont carries 87% of her
side), `routes_dropped` 0; the pick is her first builder (Agnes Digital on mob veteran 0 and the El Condor Pasa full
veteran, 6), scoring +0.27. Pilot (200 greedy runs, 3 index runs, about 4 minutes of CPU, an index run 41-81 s): greedy 310.7 [291.0,
330.4], V 7.6% above it (z 2.4); paired D = E_index - E_greedy -79.8, -5.4, -0.7 (sd 44, against greedy's 141: the
pairing cuts the variance about 10x here, not p16_borrow's 280x, since index builds and greedy does not). Descriptive
only: greedy's careers before its first looped career average 147.3 +/- 19.6 and its cost after it 163.4 +/- 2.9 (the
analytic finishing term), beside Inari One's side 145.3 and C's 189.0. That is consistent with sequencing effects, and
it calibrates neither Cont nor C: greedy's stopping event, second full veteran and lineage differ from V's trio. Full test, sized from the pilot's sd(D) (itself known only to within about [23, 279] at 2 df): 1,000 greedy runs
(about 3 minutes of CPU, +/-8.8 careers) and 10 index runs (about 9 minutes of CPU) place the plan's own cost within
about +/-10% of V; +/-5% needs about 40 index runs (about 40 minutes). Not yet run.
