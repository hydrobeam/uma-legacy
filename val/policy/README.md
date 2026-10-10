# Policy simulator in JavaScript (`val/policy_sim.mjs`)

For a decision policy (which career to run next) on a fixed starting pool, the expected number of careers until the
first veteran that lands every target, by repeated calls of the native solver. The maths (the analytic tail, pruning
dominated veterans, common random numbers) is the solver's own simulator's: `docs/SOLVER.md`, *Policy simulator*,
which reproduces this tool's draws bit for bit.

    node val/policy_sim.mjs --pool pkg/loopopt/jobs/career_U5.json --policy greedy,repeat \
         --runs 48 --steps 20 --settle 2 --seed 1 -j 4 --out /tmp/u5
    node val/policy_sim.mjs report /tmp/u5 --steps 10        # the same runs cut at another horizon

Run it from the repository root. A policy is one function in `policies.mjs`: `(job, {solve, opt}) => {pick, best,
diag}`; `greedy` takes the best P(all) now, `repeat` today's best forever, `planner` the solver's one-step rollout.
Finished runs append to `DIR/<policy>.jsonl`, so an interrupted batch resumes.

Pools (`pools/`): `pool4.json` and `pool14.json`, a player's 4- and 14-veteran pools (Unity Cup, owned roster and
cards, borrowing on).
