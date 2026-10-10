# The policy simulator's experiments

The simulator (`simulate` on a career job: `docs/SOLVER.md`, *Policy simulator*) measures, under the solver's model,
the expected number of careers until the first veteran that lands every target, for a decision policy on a fixed
starting pool. It reproduces the exact model's trajectories bit for bit (14 whole trajectories of the greedy and loop
policies on three pools), at 20-150 times the speed. Policies: `greedy`, `greedy_exact`, `loop` (the baselines a new
policy is compared against) and `trace`; adding one: `docs/SOLVER.md`, *Policy simulator*.

Its one result in use: the shape of the career planner's continuation (`Cont`: `CEIL_SHARE` 0.6, `LAM_DECAY` 1.5 in
`src/deck/career/sim/plan.rs`) was fitted to its `trace` policy, building for Agnes Digital with her most efficient
builders on 2-, 5-, 11- and 19-veteran pools (6 runs x 400 careers each). Results and the fit's error:
`docs/RESEARCH.md` §11 and `docs/MODEL.md`, *Next career*. Reproduce one trace with a career job carrying
`"simulate": {"policy": "trace", "runs": 6, "max_careers": 400, "margin": 1e9, "loop_members": [1019, <five builders>]}`
(about 5 s a run natively).

The policy comparisons (greedy, the three-uma loop) run from the repository root (pools: `pkg/loopopt/jobs/career_U5.json`
and `val/sim/pools/`):

    cargo build --release --manifest-path pkg/loopopt/Cargo.toml
    uv run val/sim/make_jobs.py /tmp/sim                         # job files and queue.txt
    (cd pkg/loopopt && for n in $(cat /tmp/sim/queue.txt); do target/release/loopopt data.json /tmp/sim/$n.json > /tmp/sim/$n.out; done)
    uv run val/sim/report.py runs /tmp/sim/u5_*.out              # E per policy, paired differences
    uv run val/sim/report.py audit /tmp/sim/u5_*.out             # the surrogate's fidelity audits

The exact-model trajectories (`*_gex`, `*_lx`: a career search per decision) take the longest.
