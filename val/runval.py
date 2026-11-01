"""Run the validation jobs in parallel: every job in val/jobs.json with its own settings (val/TAG__*.out) and every deck
job again with per_uma on (val/TAGpu__*.out), plus val/TAG_times.txt and val/TAGpu_times.txt (seconds per job).

usage: runval.py BIN TAG [--quick] [-j N]
  --quick  leave out the jobs in val/slow.json. For iterating on a change; releases and merges run the full set.
  -j N     jobs at a time (default: nonzero $VAL_J, else max(1, CPU count - 2), or 2 if CPU count is unavailable; ./build.sh passes VAL_J through)."""

import argparse
import json
import os
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass

import solver


@dataclass(frozen=True)
class ValidationTask:
    """One solver invocation, run from the era's data directory with an absolute job path."""

    mode: str  # Empty for shared decks, 'pu' for per-uma decks.
    name: str  # Era-qualified name used in output filenames.
    era: str
    job_path: str


VALIDATION_DIR = solver.VALIDATION_DIR
parser = argparse.ArgumentParser(description="Run the validation jobs in parallel.", allow_abbrev=False)
parser.add_argument("binary", metavar="BIN")
parser.add_argument("tag", help="writes val/TAG__*.out and val/TAGpu__*.out")
parser.add_argument("--quick", action="store_true", help="leave out the jobs in val/slow.json")
parser.add_argument(
    "-j",
    type=int,
    default=int(os.environ.get("VAL_J", 0)) or max(1, (os.cpu_count() or 4) - 2),
    dest="parallelism",
    metavar="N",
    help="jobs at a time (default: nonzero $VAL_J, else the CPU count less two, at least one)",
)
arguments = parser.parse_intermixed_args()
quick, parallelism = arguments.quick, arguments.parallelism
binary, tag = os.path.abspath(arguments.binary), arguments.tag
slow = {tuple(x) for x in json.load(open(os.path.join(VALIDATION_DIR, "slow.json")))} if quick else set()


def prepare_tasks():
    tasks = []
    for era, job_name in json.load(open(os.path.join(VALIDATION_DIR, "jobs.json"))):
        name = era + "__" + job_name
        job_path = os.path.join(VALIDATION_DIR, "jobs", job_name + ".json")
        job = json.load(open(job_path))
        if ("", era, job_name) not in slow and ("sh", era, job_name) not in slow:
            tasks.append(ValidationTask(mode="", name=name, era=era, job_path=job_path))
        if job.get("mode") == "deck" and ("pu", era, job_name) not in slow:
            job["per_uma"] = True
            temporary_job = os.path.join(VALIDATION_DIR, f"_pu_tmp_{tag}__{name}.json")
            json.dump(job, open(temporary_job, "w"))
            tasks.append(ValidationTask(mode="pu", name=name, era=era, job_path=temporary_job))
    return tasks


tasks = prepare_tasks()
times = {"": {}, "pu": {}}


def run(task):
    started = time.time()
    stdout = solver.run(binary, task.era, task.job_path, name=f"{task.mode or 'shared'} {task.name}")
    open(os.path.join(VALIDATION_DIR, f"{tag}{task.mode}__{task.name}.out"), "w").write(stdout)
    times[task.mode][task.name] = round(time.time() - started, 1)
    if task.mode == "pu":
        os.remove(task.job_path)


started = time.time()
# longest first (reference times), so the slow jobs don't start last
reference_times = {
    m: dict(l.split() for l in open(os.path.join(VALIDATION_DIR, f"ref{m}_times.txt")))
    if os.path.exists(os.path.join(VALIDATION_DIR, f"ref{m}_times.txt"))
    else {}
    for m in ("", "pu")
}
with ThreadPoolExecutor(parallelism) as executor:
    list(executor.map(run, sorted(tasks, key=lambda task: -float(reference_times[task.mode].get(task.name, 0)))))
for mode in ("", "pu"):
    open(os.path.join(VALIDATION_DIR, f"{tag}{mode}_times.txt"), "w").write(
        "".join(f"{n} {s}\n" for n, s in sorted(times[mode].items()))
    )
print(
    f"{len(tasks)} jobs{' (quick: ' + str(len(slow)) + ' slow left out)' if quick else ''} in {time.time() - started:.0f} s, {parallelism} at a time"
)
