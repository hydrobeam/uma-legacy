"""Monotonicity tripwire for the career planner: a veteran added to a pool never raises V (every plan without her is
still there). For each val/plan job: plan the pool less its last veteran, then the full pool under that plan's fit
(`next.fit`, as the page's Compare prices it), and report V without -> V with and the route steps. A rise is the model's
error, not hers (the fixed fit rules out the continuation's refit; what remains is the route's fixed lineages: each fresh
all-target veteran bred from her uma's best pair now).

usage: mono.py BIN [JOB ...] [-j N]   (default: every job in val/plan/jobs.json; -j jobs at a time, default 2)
  writes temporary job files; exits 1 when a V rises by more than 1e-9 relative"""

import json
import os
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass


@dataclass(frozen=True)
class PoolComparison:
    """A pool's two plans under the same fit, or the reason they cannot be compared."""

    name: str
    without: dict | None = None
    with_veteran: dict | None = None
    unavailable: bool = False
    error: str | None = None


def route_steps(plan):
    if not plan.get("v0_plan"):
        return "?"
    return " / ".join(
        f"{plan['v0_plan'][role]} {careers:,.1f}" for role, careers in zip("abc", plan["v0_plan"]["careers"])
    )


VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(VALIDATION_DIR)
PLAN_DIR = os.path.join(VALIDATION_DIR, "plan")
args = sys.argv[1:]
parallelism = 2
if "-j" in args:
    i = args.index("-j")
    parallelism = int(args[i + 1])
    del args[i : i + 2]
binary = os.path.abspath(args[0])
want = set(args[1:])
jobs = [(era, name) for era, name in json.load(open(os.path.join(PLAN_DIR, "jobs.json"))) if not want or name in want]
temporary_dir = tempfile.mkdtemp(prefix="mono_")


def plan(era, job, name):
    path = os.path.join(temporary_dir, name + ".json")
    json.dump(job, open(path, "w"))
    result = subprocess.run(
        [binary, "data.json", path],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", era),
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)["next"]


def one(job_entry):
    era, name = job_entry
    job = json.load(open(os.path.join(PLAN_DIR, name + ".json")))
    if len(job.get("legacies", [])) < 3:
        return PoolComparison(name=name)
    less = dict(job, legacies=job["legacies"][:-1], next={**job.get("next", {})})
    without = plan(era=era, job=less, name=name + "_less")
    if without.get("v0") is None or not without.get("fit"):
        return PoolComparison(name=name, unavailable=True, error=without.get("error"))
    with_veteran = plan(
        era=era, job=dict(job, next={**job.get("next", {}), "fit": without["fit"]}), name=name + "_with"
    )
    return PoolComparison(name=name, without=without, with_veteran=with_veteran)


rises = 0
with ThreadPoolExecutor(parallelism) as ex:
    comparisons = list(ex.map(one, jobs))
for comparison in comparisons:
    name = comparison.name
    if comparison.without is None and not comparison.unavailable:
        print(f"{name}: under 3 veterans, skipped")
        continue
    if comparison.unavailable:
        print(f"{name}: no plan without her {comparison.error or ''}")
        continue
    without, with_veteran = comparison.without, comparison.with_veteran
    value_without, value_with = without["v0"], with_veteran["v0"]
    rise = value_with is not None and value_with > value_without * (1 + 1e-9)
    rises += rise
    print(
        f"{name}: V {value_without:,.2f} -> {value_with:,.2f}  ({'RISES ' if rise else ''}{value_with - value_without:+,.2f})\n    without: {route_steps(without)}\n    with:    {route_steps(with_veteran)}"
    )
print(f"{len(comparisons)} pools, {rises} rise")
sys.exit(1 if rises else 0)
