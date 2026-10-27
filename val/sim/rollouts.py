"""Paired truncated rollouts of the career planner's top rows: a Bellman-consistency check of V under greedy.

Per pool: plan it (`next`), take its top ROWS rows by score; for each row, force that career first (`simulate.force`,
the trainee on the row's parents with her best deck), then continue with one fixed cheap policy, greedy under goal
"looped" (docs/SOLVER.md *Policy simulator*), for CAREERS careers in all, on common draws: run r faces the same
uniforms under every row, so differences between rows are paired by run. Each run is valued at its end by the plan's
own V of the pool it ends with, weighted by the chance it is still running (`keep_pool`: the survival S and the final
legacies):

    T = sum over its careers of S_t  +  S_K x V(final pool)

(S_t falls only on careers on two full veterans, whose success is a fully looped parent; S_K = 1 while none has run.)
A row's score is its expected saving per run in the plan's units, so the paired difference T(row) - T(pick) estimates
score(pick) - score(row) if the plan's scoring agrees with its own V after CAREERS careers of the continuation. The
continuation is greedy, not the plan, so this tests the first step's ranking under one continuation policy (Bellman
consistency of the first step), not the plan's whole policy; and V at the end is the plan's own estimate, so a bias of V
common to all rows cancels in the differences while one that depends on the final pool does not. Where no run finishes
within CAREERS careers (S_K = 1: every run of the 2026-10-26 measurement on p10, p11, boot_mob and p16), T is exactly
CAREERS + V(final pool): both terms are V's, so the check cannot give V's level or the true cost of finishing
(val/sim/v_level.py measures V against complete greedy runs).

usage: uv run val/sim/rollouts.py BIN OUT_DIR [--runs 50] [--careers 50] [--rows 3] [-j 2] [POOL ...]
  pools default p10 p11 boot_mob p16 (val/plan); writes OUT_DIR/POOL_plan.{json,out}, OUT_DIR/POOL_rowI.{json,out}
  and OUT_DIR/POOL_rowI_runR_final.{json,out}; prints per pool each row's mean T, the paired T(row) - T(pick) with a
  95% t interval, and the score difference it is compared with. Cost: one plan per run and row (the final pool's), so
  rows x runs plans per pool: size it with --runs 1 first."""

import json
import math
import os
import statistics
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

SIM_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(SIM_DIR))
PLAN_DIR = os.path.join(REPO, "val", "plan")
DEFAULT_POOLS = ["p10", "p11", "boot_mob", "p16"]


def t975(degrees):
    """the t distribution's 97.5% quantile (series in 1/df, as sim/play.rs `tq975`)"""
    z = 1.959964
    return (
        z
        + (z**3 + z) / (4 * degrees)
        + (5 * z**5 + 16 * z**3 + 3 * z) / (96 * degrees**2)
        + (3 * z**7 + 19 * z**5 + 17 * z**3 - 15 * z) / (384 * degrees**3)
    )


def interval(values):
    mean = statistics.fmean(values)
    if len(values) < 2:
        return mean, math.nan, math.nan
    half = t975(len(values) - 1) * statistics.stdev(values) / math.sqrt(len(values))
    return mean, mean - half, mean + half


def solve(binary, era, job, path):
    json.dump(job, open(path + ".json", "w"))
    result = subprocess.run(
        [binary, "data.json", path + ".json"],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", era),
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"{path}: solver exited {result.returncode}\n{result.stderr}")
    open(path + ".out", "w").write(result.stdout)
    return json.loads([line for line in result.stdout.splitlines() if line.strip()][-1])


def era_of(pool):
    return next(era for era, name in json.load(open(os.path.join(PLAN_DIR, "jobs.json"))) if name == pool)


def rollouts(binary, out_dir, pool, settings, executor):
    era = era_of(pool)
    job = json.load(open(os.path.join(PLAN_DIR, pool + ".json")))
    plan = solve(binary, era, dict(job, next={**job.get("next", {})}), os.path.join(out_dir, f"{pool}_plan"))["next"]
    rows = sorted((o for o in plan["options"] if o.get("score") is not None), key=lambda o: -o["score"])
    rows = rows[: settings["rows"]]
    simulate = {
        "policy": "greedy",
        "goal": "looped",
        "seed": 1,
        "runs": settings["runs"],
        "max_careers": settings["careers"],
        "margin": 0,
        "eps": 0,
        "keep_pool": True,
    }
    row_jobs = [
        dict(job, simulate={**simulate, "force": {"trainee": row["trainee"]["id"], "parents": row["parents"]}})
        for row in rows
    ]
    paths = [os.path.join(out_dir, f"{pool}_row{i}") for i in range(len(rows))]
    simulations = list(executor.map(lambda pair: solve(binary, era, *pair), zip(row_jobs, paths)))
    # each run's end: the plan's V of its final pool, weighted by its survival
    finals = [
        (i, r, dict(job, legacies=run["final_legacies"], next={**job.get("next", {})}))
        for i, simulation in enumerate(simulations)
        for r, run in enumerate(simulation["runs"])
        if run["survival"] > 0
    ]
    values = executor.map(
        lambda final: solve(binary, era, final[2], os.path.join(out_dir, f"{pool}_row{final[0]}_run{final[1]}_final")),
        finals,
    )
    final_v = {(i, r): value["next"].get("v0") for (i, r, _), value in zip(finals, values)}
    totals = []
    for i, simulation in enumerate(simulations):
        row_totals = []
        for r, run in enumerate(simulation["runs"]):
            v = final_v.get((i, r), 0.0)
            if v is None:
                raise RuntimeError(f"{pool} row {i} run {r}: no V for the final pool")
            row_totals.append(run["E"] - run["tail"] + run["survival"] * v)
        totals.append(row_totals)
    report(pool, plan, rows, totals)


def report(pool, plan, rows, totals):
    print(f"== {pool}: V {plan['v0']:,.2f}, {len(totals[0])} paired runs")
    print("| row | career | score | V - score | mean T [95%] | T(row) - T(pick) [95%] | score(pick) - score(row) |")
    print("|---|---|---|---|---|---|---|")
    for i, (row, row_totals) in enumerate(zip(rows, totals)):
        mean, low, high = interval(row_totals)
        name = f"{row['trainee']['name']} on {row['parents']}"
        if i == 0:
            difference = "(the pick)"
        else:
            d, d_low, d_high = interval([a - b for a, b in zip(row_totals, totals[0])])
            difference = f"{d:+,.2f} [{d_low:+,.2f}, {d_high:+,.2f}]"
        print(
            f"| {i} | {name} | {row['score']:.3f} | {plan['v0'] - row['score']:,.2f} | "
            f"{mean:,.2f} [{low:,.2f}, {high:,.2f}] | {difference} | "
            f"{rows[0]['score'] - row['score']:+.3f} |"
        )


def main(args):
    def option(flag, default):
        if flag not in args:
            return default
        i = args.index(flag)
        value = int(args[i + 1])
        del args[i : i + 2]
        return value

    settings = {"runs": option("--runs", 50), "careers": option("--careers", 50), "rows": option("--rows", 3)}
    parallelism = option("-j", 2)
    binary, out_dir = os.path.abspath(args[0]), args[1]
    os.makedirs(out_dir, exist_ok=True)
    with ThreadPoolExecutor(parallelism) as executor:
        for pool in args[2:] or DEFAULT_POOLS:
            rollouts(binary, out_dir, pool, settings, executor)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
