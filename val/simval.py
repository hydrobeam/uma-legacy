"""The policy simulator's check jobs (val/sim/jobs.json: one or more per policy, a few seconds each): run them, then
compare each output with its reference through `val/same.py TAG REF --sim` (the whole JSON, equal values; only the
wall clocks are ignored). For solver work that must not move the simulator's trajectories; `val/runval.py` and
`val/planval.py` do not run the simulator.

usage: simval.py BIN TAG [REF] [-j N]
  writes val/sim/TAG__NAME.out; compares with val/sim/REF__NAME.out (default REF: ref)
  -j N  jobs at a time (default 2)

Each job is a pool file (a career job) with the job's `simulate` block put in, run from the era's data folder (the
pool's file paths are relative to it)."""

import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from typing import NamedTuple

import solver
from solver import REPO, VALIDATION_DIR

SIM_DIR = os.path.join(VALIDATION_DIR, "sim")


class SimulatorJob(NamedTuple):
    name: str
    era: str
    pool: str  # the career job, relative to the repository root
    simulate: dict  # the `simulate` block (docs/SOLVER.md, *Policy simulator*)


def run_job(binary, tag, job_dir, job):
    """Runs one job; returns its wall seconds."""
    career_job = json.load(open(os.path.join(REPO, job.pool)))
    career_job["job"]["career"]["simulate"] = job.simulate
    job_path = os.path.join(job_dir, job.name + ".json")
    json.dump(career_job, open(job_path, "w"))
    started = time.time()
    stdout = solver.run(binary, job.era, job_path, name=job.name)
    open(os.path.join(SIM_DIR, f"{tag}__{job.name}.out"), "w").write(stdout)
    return round(time.time() - started, 2)


def main():
    parser = argparse.ArgumentParser(description="Run the policy simulator's check jobs.", allow_abbrev=False)
    parser.add_argument("binary", metavar="BIN")
    parser.add_argument("tag", help="writes val/sim/TAG__NAME.out")
    parser.add_argument("ref", nargs="?", default="ref", help="compares with val/sim/REF__NAME.out (default ref)")
    parser.add_argument("-j", type=int, default=2, dest="parallelism", metavar="N", help="jobs at a time")
    arguments = parser.parse_intermixed_args()
    parallelism = arguments.parallelism
    binary, tag = os.path.abspath(arguments.binary), arguments.tag
    ref = arguments.ref
    jobs = [SimulatorJob(**entry) for entry in json.load(open(os.path.join(SIM_DIR, "jobs.json")))]
    with tempfile.TemporaryDirectory() as job_dir, ThreadPoolExecutor(parallelism) as executor:
        seconds = list(executor.map(lambda job: run_job(binary, tag, job_dir, job), jobs))
    print("  ".join(f"{job.name} {job_seconds} s" for job, job_seconds in zip(jobs, seconds)), flush=True)
    same = os.path.join(VALIDATION_DIR, "same.py")
    return subprocess.run([sys.executable, same, tag, ref, "--sim"]).returncode


if __name__ == "__main__":
    sys.exit(main())
