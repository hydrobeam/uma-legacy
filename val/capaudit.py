"""Audit of the exact caps per deck structure class (`EXACT_CAPS`, docs/SOLVER.md): run jobs normally and with the job
field `cap_audit` (every class cap at 2.0, so the deck searches keep far more decks and score them exactly), then
compare. Prints, per class, the largest exact / product odds the audit run scored (and the deck), flags a class whose
largest ratio passes its normal cap, and fails when the answer moves (a deck the normal caps missed).

usage: capaudit.py BIN [JOB ...]
  JOB: a val/plan job name (p16, gm16, ...) or a val/jobs job name with ':pu' for per-uma (v_uc_k1:pu).
  Default: p16, gm16, v_uc_k1:pu, one job at a time. An audit that falls down the cap ladder
  checks only classes whose normal cap is below the reached cap.
  Exits 1 when an answer moves or a ratio passes its cap."""

import json
import os
import subprocess
import sys
import tempfile
import time
from typing import NamedTuple


class SolverRun(NamedTuple):
    output: dict
    seconds: float


class Certification(NamedTuple):
    reached: float | None
    target: float | None


VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(VALIDATION_DIR)
PLAN_DIR = os.path.join(VALIDATION_DIR, "plan")
CLS = ["0-1", "2", "3", "4+"]
args = sys.argv[1:]
binary = os.path.abspath(args[0])
want = args[1:] or ["p16", "gm16", "v_uc_k1:pu"]
plan_dirs = {name: era for era, name in json.load(open(os.path.join(PLAN_DIR, "jobs.json")))}
val_eras = {name: era for era, name in json.load(open(os.path.join(VALIDATION_DIR, "jobs.json")))}
temporary_dir = tempfile.mkdtemp(prefix="capaudit_")


def run(era, job, name, env=None):
    path = os.path.join(temporary_dir, name + ".json")
    json.dump(job, open(path, "w"))
    started = time.time()
    r = subprocess.run(
        [binary, "data.json", path],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", era),
        capture_output=True,
        text=True,
        env=env,
    )
    t = r.stdout
    return SolverRun(output=json.loads(t[t.index("{") :]), seconds=time.time() - started)


# what the answer is: everything but timings, search-work counters, the cap fields and the ceilings that scale with the
# top cap (`bound`: a career's ceiling; the plan's certification counts)
SKIP = {
    "bound",
    "umas_searched",
    "umas_certified",
    "umas_scored",
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
    "exact",
    "cap",
    "class_caps",
    "complete",
    "proven",
    "proven_top",
    "foursomes_ranked",
    "foursomes_scanned",
}


def walk(normal, audited, path, differences):
    if len(differences) >= 6:
        return
    if type(normal) != type(audited):
        differences.append(f"{path}: type")
        return
    if isinstance(normal, dict):
        for k in sorted(set(normal) | set(audited)):
            if k in SKIP or k.endswith("_ms"):
                continue
            if k not in normal or k not in audited:
                differences.append(f"{path}.{k}: only in one")
                continue
            walk(normal[k], audited[k], f"{path}.{k}", differences)
    elif isinstance(normal, list):
        if len(normal) != len(audited):
            differences.append(f"{path}: length {len(normal)} vs {len(audited)}")
            return
        for i, (x, y) in enumerate(zip(normal, audited)):
            walk(x, y, f"{path}[{i}]", differences)
    elif normal != audited:
        differences.append(f"{path}: {str(normal)[:50]} vs {str(audited)[:50]}")


problems = 0
started_all = time.time()
for requested in want:
    if requested in plan_dirs:
        era, job, name = plan_dirs[requested], json.load(open(os.path.join(PLAN_DIR, requested + ".json"))), requested
    else:
        name, _, mode = requested.partition(":")
        era = val_eras[name]
        job = json.load(open(os.path.join(VALIDATION_DIR, "jobs", name + ".json")))
        if mode == "pu":
            job["per_uma"] = True
            name += "_pu"
    normal, normal_seconds = run(era=era, job=job, name=name)
    # Validation jobs get a larger exact-search budget to reduce fallback down the cap ladder.
    env = None if requested in plan_dirs else dict(os.environ, EXACT_BUDGET=str(300_000_000))
    audited, audit_seconds = run(era=era, job=dict(job, cap_audit=True), name=name + "_audit", env=env)
    differences = []
    walk(normal, audited, "", differences)
    # the certificate and the per-class ratios: per-uma jobs in `exact`, plan jobs in `next.work`
    audit_certificate = audited.get("exact") or ((audited.get("next") or {}).get("work")) or {}
    normal_certificate = normal.get("exact") or ((normal.get("next") or {}).get("work")) or {}
    caps = normal_certificate.get("class_caps") or [None] * 4
    rho = (
        Certification(reached=audit_certificate.get("rho"), target=audit_certificate.get("rho_target"))
        if "rho" in audit_certificate
        else Certification(reached=min(audit_certificate.get("cap") or 0, audited.get("cap") or 0), target=2.0)
    )
    print(
        f"{name}: answer {'same' if not differences else 'MOVES'}  normal {normal_seconds:.1f} s, audit {audit_seconds:.1f} s (audit certified at {rho.reached} of {rho.target})"
    )
    for o in differences:
        print("    ", o)
    problems += bool(differences)
    if rho.reached is not None and rho.target is not None and rho.reached < rho.target:
        print(
            f"     (the audit run fell down the cap ladder to {rho.reached}: a class whose normal cap is at least that is not checked)"
        )
    if audited.get("complete") is False or (audited.get("exact") or {}).get("complete") is False:
        print("     (the audit run is not complete: it checks less than it claims)")
    for k in range(4):
        r = (audit_certificate.get("class_max_ratio") or [0] * 4)[k]
        deck = (audit_certificate.get("class_max_deck") or [None] * 4)[k]
        over = caps[k] is not None and r > caps[k] * (1 + 1e-9)
        problems += over
        who = f"  {deck['trainee']}: {', '.join(deck['cards'])}" if deck and (over or r > 1.0) else ""
        unchecked = rho.reached is not None and caps[k] is not None and rho.reached <= caps[k]
        print(
            f"    class {CLS[k]:>3}: max exact/product {r:.4f}, normal cap {caps[k]}{' (not checked)' if unchecked else ''}{'  OVER THE CAP' if over else ''}{who}"
        )
print(f"{len(want)} jobs in {time.time() - started_all:.0f} s, {problems} problems")
sys.exit(1 if problems else 0)
