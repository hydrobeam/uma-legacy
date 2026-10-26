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

## Paired truncated rollouts (a check that can fail on a pick)

`rollouts.py` forces each of a pool's top rows as the first career (`simulate.force`), continues with greedy under goal
`looped` for 50 careers in all on common draws, and values each run's end by the plan's own V of its final pool
(`simulate.keep_pool`), weighted by the run's survival. The paired difference T(row) - T(pick) is compared with
score(pick) - score(row). It tests the first step's ranking under one continuation, not the plan's whole policy. One
command reruns it (`uv run val/sim/rollouts.py BIN OUT_DIR --runs N [POOL ...]`); a run costs about 3 plans per pool.

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
