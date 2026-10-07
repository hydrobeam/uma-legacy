// Decision policies for val/policy_sim.mjs. A policy is one async function
//
//     (job, ctx) => { pick, best, diag }
//
// `job` is the current pool as a career-mode job (the fixture's settings plus every veteran so far; do not mutate it),
// `ctx.solve(job)` runs the native solver on a job object and resolves to its parsed output, `ctx.opt` holds the CLI
// options (`k` = the planner's rollout width). It returns
//   pick: the career to run next, a solver result with `trainee`, `parents`, `p_all`, `outcome_sets` and `schedule`
//         (the solver attaches the last two to `results[0]` and to `rollout.choice` only);
//   best: the pool's best career by P(all) (`results[0]`), used by the stopping rule;
//   diag: anything worth keeping in the per-run record (optional).
// A new policy (another planner, another binary, several solves per step) is one more entry here.
//
// `repeat` is special: its expected careers are exactly 1/p* with p* today's best P(all), so it is computed once
// (`exact: true`), never simulated.

const base = (job) => ({ ...job, lookahead: false, policy: undefined });

export const POLICIES = {
  // The page's planner: the job's one-step rollout over the K best careers (`rollout.choice`, lowest Q).
  planner: async (job, { solve, opt }) => {
    const out = await solve({ ...base(job), rollout: opt.k ?? job.rollout ?? 6 });
    const r = out.rollout;
    if (!r || !r.choice) throw new Error('planner: no rollout.choice in the solver output');
    const c = r.candidates?.[r.pick] ?? {};
    return {
      pick: r.choice, best: out.results[0],
      diag: { q: c.q, improve: c.improve, gain: c.gain, f_fail: c.f_fail, p_star: r.p_star, p_bar: r.p_bar,
              lower_bound: r.lower_bound, differs: r.differs, proven: !!(r.candidates_proven && r.shortlist_proven && r.p_star_proven) },
    };
  },
  // Greedy: the best P(all) now, re-searched after every career.
  greedy: async (job, { solve }) => {
    const out = await solve({ ...base(job), rollout: 0, top: 1, per_trainee: false });
    return { pick: out.results[0], best: out.results[0], diag: { proven: !!out.proven_top } };
  },
  // Repeat today's best forever: E = 1/p*, exact.
  repeat: Object.assign(async (job, { solve }) => {
    const out = await solve({ ...base(job), rollout: 0, top: 1, per_trainee: false });
    return { pick: out.results[0], best: out.results[0] };
  }, { exact: true }),
};
