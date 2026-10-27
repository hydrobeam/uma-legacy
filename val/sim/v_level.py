"""V's level against a feasible policy: the plan's V (`next.v0`) beside the measured cost of complete greedy runs under
goal "looped", the plan's own goal (docs/SOLVER.md, *Policy simulator*).

Per pool (val/plan): plan it with BIN for V, then RUNS greedy runs (margin 0) under goal "looped", seed 1,
`max_careers` 3 x V, `eps` 0.5, split into -j run ranges. Greedy is a policy the player could run, so its E is what
that policy costs under the model, and V - E_greedy > 0 means V reads high by at least that much against the value of
the best policy (an optimal V is at most every feasible policy's cost). The reverse sign says nothing about V by itself.

`eps` 0.5 stops a run once S < 0.5 and adds the tail S / p of the last career on two full veterans. For greedy that is
exact, not a cutoff: once two full veterans make a looped career greedy runs only looped careers, whose veterans
(forced failures) are never full veterans, so no later career can change its pick or its P(all), and repeating that
career is what greedy does. A run that never reaches two full veterans by `max_careers` is `censored`: its E is a lower
bound, and the script says so. Pools where greedy rarely finishes (no real full veteran, low odds) cost far more per run
and are mostly tail: use pools with a real full veteran (p16_borrow; a building pool for Cont is val/sim/README.md's).

With --index N, the plan's own policy (`index`) also runs on run indices 0 .. N-1, paired with greedy's runs of the
same index (common draws): its E is estimated as greedy's mean over all RUNS plus the paired mean of
D = E_index - E_greedy, which costs N index runs instead of the hundreds an unpaired mean needs. For index `eps` 0.5
is a stated stop, not exact: E is then the cost of "index until the stop, then repeat the last career on two full
veterans", itself a feasible policy (on p16_borrow run 0 it equalled the cost at eps 0.05 to 4e-14). An index run costs
about 50-160 s on p16_borrow.

What this does not test: Cont (on p16_borrow the one building side builds 1.8 of its 121.7 careers, and is an
Alternative side), and the pick between near options (a policy's whole cost, not a first step's ranking). It places V's
level on the pools it runs, and nothing else.

usage: uv run val/sim/v_level.py BIN OUT_DIR [--runs 1000] [--index 0] [-j 2] [POOL ...]
  POOL a val/plan job's name or a career job's path (era global, e.g. val/sim/pools/boot_build.json, the building pool
  for Cont: val/sim/README.md); pools default p16_borrow; writes OUT_DIR/POOL_plan.{json,out}, OUT_DIR/POOL_greedy_FIRST.{json,out} and
  OUT_DIR/POOL_index_R.{json,out}. Cost: about 0.04 s per greedy run on p16_borrow (1,000 runs: 37 s of CPU, 2026-10-26)."""

import json
import math
import os
import statistics
import sys
from concurrent.futures import ThreadPoolExecutor

from rollouts import PLAN_DIR, era_of, solve, t975

DEFAULT_POOLS = ["p16_borrow"]
SEED = 1
EPS = 0.5


def looped_job(job, v, simulate):
    return dict(job, simulate={"goal": "looped", "seed": SEED, "max_careers": math.ceil(3 * v), "eps": EPS, **simulate})


def run_ranges(runs, parts):
    """`first` and `runs` of `parts` contiguous run ranges covering 0 .. runs - 1"""
    size = math.ceil(runs / parts)
    return [(first, min(size, runs - first)) for first in range(0, runs, size)]


def pool_job(pool):
    """(its name, era, career job) for a val/plan job's name or a job file's path (era global)"""
    if pool.endswith(".json"):
        return os.path.splitext(os.path.basename(pool))[0], "global", json.load(open(pool))
    return pool, era_of(pool), json.load(open(os.path.join(PLAN_DIR, pool + ".json")))


def measure(binary, out_dir, pool, settings, executor):
    """(the pool's name, V, greedy's runs by run index, index's runs by run index); a run is the simulator's record"""
    name, era, job = pool_job(pool)
    plan = solve(binary, era, dict(job, next={**job.get("next", {})}), os.path.join(out_dir, f"{name}_plan"))["next"]
    v = plan["v0"]
    greedy_jobs = [
        (looped_job(job, v, {"policy": "greedy", "margin": 0, "first": first, "runs": runs}), f"{name}_greedy_{first}")
        for first, runs in run_ranges(settings["runs"], settings["parallelism"])
    ]
    index_jobs = [
        (looped_job(job, v, {"policy": "index", "first": r, "runs": 1}), f"{name}_index_{r}")
        for r in range(settings["index"])
    ]
    outputs = list(
        executor.map(
            lambda entry: solve(binary, era, entry[0], os.path.join(out_dir, entry[1])), greedy_jobs + index_jobs
        )
    )
    by_run = lambda outs: {run["run"]: run for out in outs for run in out["runs"]}
    return name, v, by_run(outputs[: len(greedy_jobs)]), by_run(outputs[len(greedy_jobs) :])


# Student t's 97.5% quantile at 1 .. 30 degrees of freedom (scipy.stats.t.ppf, 4 decimals); past 30 the series of
# rollouts.t975 is within 2e-6 of it
T975_SMALL = (12.7062, 4.3027, 3.1824, 2.7764, 2.5706, 2.4469, 2.3646, 2.3060, 2.2622, 2.2281, 2.2010, 2.1788, 2.1604,
              2.1448, 2.1314, 2.1199, 2.1098, 2.1009, 2.0930, 2.0860, 2.0796, 2.0739, 2.0687, 2.0639, 2.0595, 2.0555,
              2.0518, 2.0484, 2.0452, 2.0423)  # fmt: skip


def t_quantile(degrees):
    return T975_SMALL[degrees - 1] if degrees <= len(T975_SMALL) else t975(degrees)


def mean_interval(values):
    """(mean, its standard error, the 95% t half-width)"""
    mean = statistics.fmean(values)
    se = statistics.stdev(values) / math.sqrt(len(values)) if len(values) > 1 else math.nan
    return mean, se, t_quantile(len(values) - 1) * se if len(values) > 1 else math.nan


def report(pool, v, greedy, index):
    costs = [run["E"] for run in greedy.values()]
    censored = sum(run["censored"] for run in greedy.values())
    mean, se, half = mean_interval(costs)
    print(f"\n{pool}: V {v:,.2f}; greedy {len(costs)} runs: E {mean:,.1f} [{mean - half:,.1f}, {mean + half:,.1f}]")
    print(
        f"  V - greedy {v - mean:+,.1f} [{v - mean - half:+,.1f}, {v - mean + half:+,.1f}] careers "
        f"({v / mean - 1:+.1%} of greedy's E; "
        + (f"z {(v - mean) / se:+.1f})" if se > 0 else "every run the same: no spread, no z)")
        + (f"; {censored} runs censored: E a lower bound" if censored else "; no run censored")
    )
    if not index:
        return
    paired = sorted(r for r in index if r in greedy)
    differences = [index[r]["E"] - greedy[r]["E"] for r in paired]
    print(
        f"  index {len(paired)} runs, paired D = E_index - E_greedy: "
        + ", ".join(f"{d:+.1f}" for d in differences)
        + f"; {sum(index[r]['censored'] for r in paired)} censored"
    )
    if len(differences) < 2:
        return
    # Index's E = greedy's mean over all M runs + the paired mean of D over N. Split greedy's runs into the N paired and
    # the M - N unpaired: the estimate is (M - N) / M x the unpaired mean of g + the mean over paired runs of
    # w = E_index - (1 - N / M) g, two independent means whose estimated variances cannot be negative.
    d_mean = statistics.fmean(differences)
    m, n = len(costs), len(paired)
    unpaired = [greedy[r]["E"] for r in greedy if r not in index]
    weighted = [index[r]["E"] - (1 - n / m) * greedy[r]["E"] for r in paired]
    estimate = mean + d_mean
    summary = f"  D {d_mean:+.1f} (sd {statistics.stdev(differences):.1f}); index E {estimate:,.1f}"
    if len(unpaired) == 1:
        # (one unpaired run's mean has a variance this sample cannot estimate; none unpaired contributes none)
        print(f"{summary}; interval not estimable (exactly one unpaired greedy run); V - index {v - estimate:+,.1f}")
        return
    unpaired_variance = ((m - n) / m) ** 2 * statistics.variance(unpaired) / len(unpaired) if unpaired else 0.0
    index_half = t_quantile(n - 1) * math.sqrt(unpaired_variance + statistics.variance(weighted) / n)
    print(
        f"{summary} [{estimate - index_half:,.1f}, {estimate + index_half:,.1f}] (approximate: t on {n - 1} df, the "
        f"paired term's; the estimate is not exactly t-distributed); V - index {v - estimate:+,.1f}"
    )


def main(args):
    def option(flag, default):
        if flag not in args:
            return default
        i = args.index(flag)
        value = int(args[i + 1])
        del args[i : i + 2]
        return value

    settings = {"runs": option("--runs", 1000), "index": option("--index", 0), "parallelism": option("-j", 2)}
    binary, out_dir = os.path.abspath(args[0]), args[1]
    pools = args[2:] or DEFAULT_POOLS
    os.makedirs(out_dir, exist_ok=True)
    with ThreadPoolExecutor(settings["parallelism"]) as executor:
        for pool in pools:
            report(*measure(binary, out_dir, pool, settings, executor))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
