"""The career planner's next-plan jobs (val/plan/jobs.json): run them, then compare each output with the reference
strictly (the whole JSON, equal values; keys in SKIP and keys ending in _ms are ignored). For solver work that must
not move the planner's results; `val/runval.py` does not cover these jobs.

Then the proxy gate: each job again with `next.proxy_check`, which prices every builder row through its deck-class
proxy beside its exact career. The plan admits a builder for an exact search by its proxy efficiency (within
PROXY_EFF 6% of a candidate's second best) and promotes a proxy row by its proxy saving + 1 (within PROXY_SAVE 6%), so a
proxy that reads low can keep a better row out. The gate fails a job when a proxy reads more than PROXY_GATE (3%, half
the band) low: an efficiency (each candidate's, where the exact one is at least 1% of the row's best) or a saving + 1
(where the exact one is positive). Base rows only: a looped row (her career on two full veterans) is priced against
her unconstrained proxy, which is her base row's career; rows marked `looped` are dropped, or, from a solver that
does not mark them, every row but each trainee's best by exact odds. A high-side error (a proxy that reads high only
costs a search) is reported, not failed on.

usage: planval.py BIN TAG [REF] [-j N]
  writes val/plan/TAG__NAME.out, val/plan/TAG_proxy__NAME.out and val/plan/TAG_times.txt; compares with
  val/plan/REF__NAME.out (default REF: ref)
  -j N  jobs at a time (default 2; timings are only comparable at the same load)"""

import json
import os
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from typing import NamedTuple

PROXY_GATE = 0.03


class CapWarning(NamedTuple):
    class_index: int
    ratio: float
    cap: float


VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(VALIDATION_DIR)
PLAN_DIR = os.path.join(VALIDATION_DIR, "plan")
args = sys.argv[1:]
parallelism = 2
if "-j" in args:
    i = args.index("-j")
    parallelism = int(args[i + 1])
    del args[i : i + 2]
binary, tag = os.path.abspath(args[0]), args[1]
ref = args[2] if len(args) > 2 else "ref"
jobs = json.load(open(os.path.join(PLAN_DIR, "jobs.json")))
times = {}


def run(job_entry):
    era, name = job_entry
    started = time.time()
    result = subprocess.run(
        [binary, "data.json", os.path.join(PLAN_DIR, name + ".json")],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", era),
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"{name}: solver exited {result.returncode}\n{result.stderr}")
    open(os.path.join(PLAN_DIR, f"{tag}__{name}.out"), "w").write(result.stdout)
    times[name] = round(time.time() - started, 2)


with ThreadPoolExecutor(parallelism) as ex:
    list(ex.map(run, jobs))
open(os.path.join(PLAN_DIR, f"{tag}_times.txt"), "w").write(
    "".join(f"{name} {s}\n" for name, s in sorted(times.items()))
)
# This comparison skips work and bound diagnostics as well as timings; it does not enforce byte-identical outputs.
SKIP = {
    "timing_s",
    "work",
    "bounded",
    "solved",
    "scored",
    "skipped",
    "next_bound",
    "searches",
    "deck_searches",
    "decks_scored",
    "tables",
    "nodes",
}


def walk(reference, candidate, path, differences):
    if len(differences) >= 8:
        return
    if type(reference) != type(candidate):
        differences.append(f"{path}: type {type(reference).__name__} vs {type(candidate).__name__}")
        return
    if isinstance(reference, dict):
        for k in sorted(set(reference) | set(candidate)):
            if k in SKIP or k.endswith("_ms"):
                continue
            if k not in reference or k not in candidate:
                differences.append(f"{path}.{k}: only in {'new' if k in candidate else 'ref'}")
                continue
            walk(reference[k], candidate[k], f"{path}.{k}", differences)
    elif isinstance(reference, list):
        if len(reference) != len(candidate):
            differences.append(f"{path}: length {len(reference)} vs {len(candidate)}")
            return
        for i, (reference_item, candidate_item) in enumerate(zip(reference, candidate)):
            walk(reference_item, candidate_item, f"{path}[{i}]", differences)
    elif reference != candidate:
        differences.append(f"{path}: {str(reference)[:60]} vs {str(candidate)[:60]}")


# the deck structure classes of the exact caps (`cap_class`: chain-varying targets; a multi-target group counts as 4+)
CLS = ["0-1", "2", "3", "4+"]


def near_cap(ratio, cap):
    # a scored deck's exact / product r is near its class's assumed cap c: past three quarters of the cap's headroom
    # over 1 (1.1875 of 1.25; 1.0375 of 1.05).
    # The same rule as val/cmp2.py.
    return ratio > 1 + 0.75 * (cap - 1)


def compare_job(name):
    reference_path, candidate_path = (
        os.path.join(PLAN_DIR, f"{ref}__{name}.out"),
        os.path.join(PLAN_DIR, f"{tag}__{name}.out"),
    )
    if not os.path.exists(reference_path):
        print(f"{name}: no reference ({ref})")
        return 1
    try:
        reference, candidate = json.load(open(reference_path)), json.load(open(candidate_path))
    except Exception as e:
        print(f"{name}: PARSE FAIL {e}")
        return 1
    differences = []
    walk(reference, candidate, "", differences)
    reference_times = (
        dict(l.split() for l in open(os.path.join(PLAN_DIR, f"{ref}_times.txt")))
        if os.path.exists(os.path.join(PLAN_DIR, f"{ref}_times.txt"))
        else {}
    )
    # the plan's own work (deck-search nodes, exact scorings): deterministic, so comparable at any machine load
    work_of = lambda j: (j.get("next") or {}).get("work") or {}
    reference_work, candidate_work = work_of(reference), work_of(candidate)
    work = " ".join(
        f"{k} {reference_work.get(k, '?'):,} -> {candidate_work.get(k, '?'):,}"
        if isinstance(reference_work.get(k), int) and isinstance(candidate_work.get(k), int)
        else f"{k} {reference_work.get(k, '?')} -> {candidate_work.get(k, '?')}"
        for k in ("nodes", "exact")
    )
    # Ratios near an assumed cap warn that pruning may discard a better deck.
    max_ratios, caps = candidate_work.get("class_max_ratio"), candidate_work.get("class_caps")
    if not isinstance(max_ratios, list):
        max_ratios, caps = [candidate_work.get("max_ratio")], [candidate_work.get("cap")]
    is_number = lambda candidate: isinstance(candidate, (int, float))
    near = [
        CapWarning(class_index=k, ratio=ratio, cap=cap)
        for k, (ratio, cap) in enumerate(zip(max_ratios, caps))
        if is_number(ratio) and is_number(cap) and near_cap(ratio, cap)
    ]
    print(
        f"{name}: {'identical' if not differences else 'DIFFERS'}  {reference_times.get(name, '?')} s -> {times[name]} s  {work}"
        + (
            f"  max ratio/cap by class {' '.join(f'{ratio:.3f}/{cap}' for ratio, cap in zip(max_ratios, caps))}"
            if all(is_number(ratio) for ratio in max_ratios)
            else ""
        )
    )
    for k, ratio, cap in near:
        print(
            f"    CAP: a scored deck of class {CLS[k] if len(max_ratios) == 4 else 'any'} reached exact/product {ratio:.4f}, near its assumed cap {cap}"
        )
    for o in differences:
        print("   ", o)
    return bool(differences) or bool(near)


class ProxyErrors(NamedTuple):
    """A job's proxy errors over its base builder rows, relative (proxy / exact - 1)."""

    rows: int
    other_deck: int
    efficiency_low: float
    efficiency_high: float
    saving_low: float
    saving_high: float


def base_rows(rows):
    """The proxy rows of base careers: unmarked rows' best by exact odds per trainee (module notes)"""
    if any("looped" in row for row in rows):
        return [row for row in rows if not row["looped"]]
    best = {}
    for row in rows:
        if row["trainee"] not in best or row["exact_val"] > best[row["trainee"]]["exact_val"]:
            best[row["trainee"]] = row
    return list(best.values())


def proxy_errors(output):
    rows = [row for row in base_rows(output["next"]["proxy_check"]["rows"]) if not row["cand"]]
    efficiency, saving = [0.0], [0.0]
    for row in rows:
        row_best = max(row["eff"], default=0.0)
        efficiency += [
            proxy / exact - 1
            for exact, proxy in zip(row["eff"], row["proxy_eff"])
            if row_best > 0 and exact >= 0.01 * row_best
        ]
        if row["score"] is not None and row["proxy_score"] is not None and row["score"] + 1 > 0:
            saving.append((row["proxy_score"] + 1) / (row["score"] + 1) - 1)
    return ProxyErrors(
        rows=len(rows),
        other_deck=sum(not row["same_deck"] for row in rows),
        efficiency_low=min(efficiency),
        efficiency_high=max(efficiency),
        saving_low=min(saving),
        saving_high=max(saving),
    )


def proxy_gate(job_entry, job_dir):
    era, name = job_entry
    job = json.load(open(os.path.join(PLAN_DIR, name + ".json")))
    job["next"] = {**job.get("next", {}), "proxy_check": True}
    path = os.path.join(job_dir, name + ".json")
    json.dump(job, open(path, "w"))
    result = subprocess.run(
        [binary, "data.json", path],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", era),
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"{name} (proxy_check): solver exited {result.returncode}\n{result.stderr}")
    open(os.path.join(PLAN_DIR, f"{tag}_proxy__{name}.out"), "w").write(result.stdout)
    output = json.loads([line for line in result.stdout.splitlines() if line.strip()][-1])
    return proxy_errors(output) if "proxy_check" in (output.get("next") or {}) else None


failures = sum(compare_job(name) for _era, name in jobs)
print(f"{len(jobs)} plan jobs, {failures} differ")
with tempfile.TemporaryDirectory() as job_dir, ThreadPoolExecutor(parallelism) as ex:
    gates = list(ex.map(lambda job_entry: proxy_gate(job_entry, job_dir), jobs))
proxy_failures = 0
for (_era, name), errors in zip(jobs, gates):
    if errors is None:
        print(f"{name}: no proxy_check in the output")
        proxy_failures += 1
        continue
    low = errors.efficiency_low < -PROXY_GATE or errors.saving_low < -PROXY_GATE
    proxy_failures += low
    print(
        f"{name}: proxies {'LOW ' if low else ''}{errors.rows} rows ({errors.other_deck} on another deck): efficiency "
        f"{errors.efficiency_low:+.2%} .. {errors.efficiency_high:+.2%}, saving + 1 {errors.saving_low:+.2%} .. {errors.saving_high:+.2%}"
    )
print(f"{len(jobs)} proxy checks, {proxy_failures} read over {PROXY_GATE:.0%} low")
sys.exit(1 if failures or proxy_failures or not jobs else 0)
