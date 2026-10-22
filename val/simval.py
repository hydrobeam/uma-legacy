"""The policy simulator's check jobs (val/sim/jobs.json: one or more per policy, a few seconds each): run them, then
compare each output with its reference through `val/same.py TAG REF --sim` (the whole JSON, equal values; only the
wall clocks are ignored). For solver work that must not move the simulator's trajectories; `val/runval.py` and
`val/planval.py` do not run the simulator.

usage: simval.py BIN TAG [REF] [-j N]
  writes val/sim/TAG__NAME.out; compares with val/sim/REF__NAME.out (default REF: ref)
  -j N  jobs at a time (default 2)

Each job is a pool file (a career job) with the job's `simulate` block put in, run from the era's data folder (the
pool's file paths are relative to it)."""

import json
import os
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from typing import NamedTuple

VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(VALIDATION_DIR)
SIM_DIR = os.path.join(VALIDATION_DIR, "sim")


class SimulatorJob(NamedTuple):
    name: str
    era: str
    pool: str  # the career job, relative to the repository root
    simulate: dict  # the `simulate` block (docs/SOLVER.md, *Policy simulator*)


def run_job(binary, tag, job_dir, job):
    """Runs one job; returns its wall seconds."""
    career_job = json.load(open(os.path.join(REPO, job.pool)))
    career_job["simulate"] = job.simulate
    job_path = os.path.join(job_dir, job.name + ".json")
    json.dump(career_job, open(job_path, "w"))
    started = time.time()
    result = subprocess.run(
        [binary, "data.json", job_path],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", job.era),
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"{job.name}: solver exited {result.returncode}\n{result.stderr}")
    open(os.path.join(SIM_DIR, f"{tag}__{job.name}.out"), "w").write(result.stdout)
    return round(time.time() - started, 2)


def main(args):
    parallelism = 2
    if "-j" in args:
        i = args.index("-j")
        parallelism = int(args[i + 1])
        del args[i : i + 2]
    binary, tag = os.path.abspath(args[0]), args[1]
    ref = args[2] if len(args) > 2 else "ref"
    jobs = [SimulatorJob(**entry) for entry in json.load(open(os.path.join(SIM_DIR, "jobs.json")))]
    with tempfile.TemporaryDirectory() as job_dir, ThreadPoolExecutor(parallelism) as executor:
        seconds = list(executor.map(lambda job: run_job(binary, tag, job_dir, job), jobs))
    print("  ".join(f"{job.name} {job_seconds} s" for job, job_seconds in zip(jobs, seconds)), flush=True)
    same = os.path.join(VALIDATION_DIR, "same.py")
    return subprocess.run([sys.executable, same, tag, ref, "--sim"]).returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
