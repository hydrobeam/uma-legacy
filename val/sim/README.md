# The policy simulator's experiments

The simulator (`simulate` on a career job: `docs/SOLVER.md`, *Policy simulator*) measures, under the solver's model,
the expected number of careers until the first veteran that lands every target, for a decision policy on a fixed
starting pool. It reproduces the exact model's trajectories bit for bit (14 whole trajectories of the greedy and loop
policies on three pools), at 20-150 times the speed. Policies: `greedy`, `greedy_exact`, `loop` (the baselines a new
policy are compared against), `trace` and `index` (the page's planner, `sim::plan::index_rule`, run each step as the
page's plan job); adding one: `docs/SOLVER.md`, *Policy simulator*.

Its one result in use: the shape of the career planner's continuation (`Cont`: `CEIL_SHARE` 0.6, `LAM_DECAY` 1.5 in
`src/career/plan/cont.rs`) was fitted to its `trace` policy, building for Agnes Digital with her most efficient
builders on 2-, 5-, 11- and 19-veteran pools (6 runs x 400 careers each). Results and the fit's error:
`docs/MODEL.md`, *Next career*. Reproduce one trace with a career job carrying
`"simulate": {"policy": "trace", "runs": 6, "max_careers": 400, "margin": 1e9, "loop_members": [1019, <five builders>]}`
(about 5 s a run natively).

The policy comparisons (greedy, the three-uma loop) run from the repository root (pools: `val/jobs/career_U5.json`
and `val/sim/pools/`):

    cargo build --release --manifest-path pkg/loopopt/Cargo.toml
    uv run val/sim/make_jobs.py /tmp/sim                         # job files and queue.txt
    (cd pkg/loopopt/data/global && for n in $(cat /tmp/sim/queue.txt); do ../../target/release/loopopt data.json /tmp/sim/$n.json > /tmp/sim/$n.out; done)
    uv run val/sim/report.py runs /tmp/sim/u5_*.out              # E per policy, paired differences
    uv run val/sim/report.py audit /tmp/sim/u5_*.out             # the surrogate's fidelity audits

The exact-model trajectories (`*_gex`, `*_lx`: a career search per decision) take the longest.

The page's planner against greedy, on the same draws (common random numbers: run r faces the same uniforms under both
policies, so differences are paired): a val/plan pool with a `simulate` block per policy, everything else equal, run
from the pool's data folder (val/plan/jobs.json names it):

    mkdir -p /tmp/sim; uv run python -c "import json; j=json.load(open('val/plan/p16.json')); [json.dump({**j, 'simulate': dict(policy=p, runs=2, max_careers=4)}, open(f'/tmp/sim/p16_{p}.json','w')) for p in ('greedy','index')]"
    (cd pkg/loopopt/data/global && for p in greedy index; do ../../target/release/loopopt data.json /tmp/sim/p16_$p.json > /tmp/sim/p16_$p.out; done)

Compare per run the survival at the cap, `tail` x `final_p` (E's tail repeats the last pick forever, which penalises
a plan that ends on a builder's career), and `E` once the runs reach S < `eps`. `index` adds per run the plans' count,
their seconds, the picks' roles and how many were the best-odds career. A plan costs about 0.2-1.8 s on pools of 10-20
veterans and more as the pool grows (*Policy simulator* in docs/SOLVER.md): size `runs` and `max_careers` from a
short run's `plan_s` first.
