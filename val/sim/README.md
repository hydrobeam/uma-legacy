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
pools, a 2-veteran one cut from pool4), building for each pool's two stage candidates rather than one uma. It scores
each pool under the shape fitted without it (log-odds loss and cost error against the pool's achievable cost, split in
half since the in-sample minimum over noisy means reads low), reports the plan's own fitted continuation apart from
the one fitted to the traces, and prices the unbuilt variant that takes lam as the larger of now and one lift later.
A builder's veteran in a trace is drawn from every landed set (her full veteran does not end the building), so traces
taken before 2026-10-26 drew differently. It runs at integration, after
every change that moves Cont or V:

    cargo build --release --manifest-path pkg/loopopt/Cargo.toml
    uv run val/sim/fit_cont.py pkg/loopopt/target/release/loopopt /tmp/fit        # about 10 minutes on two cores
    uv run val/sim/fit_cont.py pkg/loopopt/target/release/loopopt /tmp/fit0 --margin 0 p10 p11   # the cached decks' share

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
about 10% of V, since Cont's shape alone moves V by up to about 30% on the building pools (p10, p11, boot_mob). How many runs that takes is unknown:
review estimates of sd(D) without keeping the finishing careers' success analytic were 1-1.4 V, about 1,500 runs per
pool; keeping it analytic lowers that by an unmeasured amount. Run the 4-run pilot first and size the full run from its
sd(D). A plan costs about 0.6-5 s natively on the val/plan pools (about 5 s at 10 targets), and its cost at the hundreds of veterans a long run builds has
not been measured: time one plan on a pool grown by a pilot before sizing. The pick between near options cannot be
tested this way at any affordable size (score gaps of 0.06-0.2% of V on val/plan); only whole policies can.

`index` adds per run the plans' count, their seconds, the picks' roles and how many were the best-odds career; under
goal `looped` every run adds `finishing` (careers run on two full veterans), `full_veterans` and `censored`.
