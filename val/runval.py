"""Run the validation jobs in parallel: every job in val/jobs.json with its own settings (val/TAG__*.out) and every deck
job again with per_uma on (val/TAGpu__*.out), plus val/TAG_times.txt and val/TAGpu_times.txt (seconds per job).

usage: runval.py BIN TAG [--quick] [-j N]
  --quick  leave out the jobs in val/slow.json. For iterating on a change; releases and merges run the full set.
  -j N     jobs at a time (default: nonzero $VAL_J, else max(1, CPU count - 2), or 2 if CPU count is unavailable; ./build.sh passes VAL_J through)."""

import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass


@dataclass(frozen=True)
class ValidationTask:
    """One solver invocation, run from the era's data directory with an absolute job path."""

    mode: str  # Empty for shared decks, 'pu' for per-uma decks.
    name: str  # Era-qualified name used in output filenames.
    era: str
    job_path: str


VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(VALIDATION_DIR)
args = sys.argv[1:]
quick = "--quick" in args
args = [a for a in args if a != "--quick"]
parallelism = int(os.environ.get("VAL_J", 0)) or max(1, (os.cpu_count() or 4) - 2)
if "-j" in args:
    i = args.index("-j")
    parallelism = int(args[i + 1])
    del args[i : i + 2]
binary, tag = os.path.abspath(args[0]), args[1]
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
    result = subprocess.run(
        [binary, "data.json", task.job_path],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", task.era),
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"{task.mode or 'shared'} {task.name}: solver exited {result.returncode}\n{result.stderr}")
    open(os.path.join(VALIDATION_DIR, f"{tag}{task.mode}__{task.name}.out"), "w").write(result.stdout)
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
