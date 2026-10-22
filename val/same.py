"""Whole-output identity (docs/PERF.md, *What "byte-identical" covers*): uv run val/same.py TAG [REF] [--plan | --sim]
Compares val/TAG__*.out with val/REF__*.out (default ref) and val/TAGpu__*.out with val/REFpu__*.out (with --plan:
val/plan/TAG__*.out with val/plan/REF__*.out, planval's outputs; with --sim: val/sim/TAG__*.out with
val/sim/REF__*.out, simval's outputs), every field but the timings (`timing_s`, `*_ms`; with --sim also the
simulator's `wall_s`, `init_s`, `plan_s`); prints each differing output with the paths that differ. Exit 1 on any
difference or when nothing was compared. Stricter than cmp2.py, which checks rounded P(all)."""

import glob
import json
import os
import sys

# the mode flags and the subdirectory each one compares in
MODE_DIRECTORIES = {"--plan": "plan", "--sim": "sim"}
VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__))
args = [argument for argument in sys.argv[1:] if argument not in MODE_DIRECTORIES]
mode = next((flag for flag in MODE_DIRECTORIES if flag in sys.argv), None)
if mode:
    VALIDATION_DIR = os.path.join(VALIDATION_DIR, MODE_DIRECTORIES[mode])
tag = args[0]
ref = args[1] if len(args) > 1 else "ref"
# wall clocks in seconds; the simulator's: per run, its setup, each plan of the index policy
TIMING_KEYS = {"timing_s"} | ({"wall_s", "init_s", "plan_s"} if mode == "--sim" else set())


# a key absent from one side: unequal to every value, null included (.get() alone would read it as None)
MISSING = object()


def is_timing(key):
    return key in TIMING_KEYS or key.endswith("_ms")


def load(path):
    text = open(path).read()
    return json.loads(text[text.index("{") :])


def paths(reference, candidate, at=""):
    """Paths whose values differ, excluding only timings."""
    if isinstance(reference, dict) and isinstance(candidate, dict):
        return [
            path
            for key in sorted(set(reference) | set(candidate))
            if not is_timing(key)
            for path in paths(reference.get(key, MISSING), candidate.get(key, MISSING), f"{at}.{key}")
        ]
    if isinstance(reference, list) and isinstance(candidate, list) and len(reference) == len(candidate):
        return [
            path
            for i, (reference_item, candidate_item) in enumerate(zip(reference, candidate))
            for path in paths(reference_item, candidate_item, f"{at}[{i}]")
        ]
    return [] if reference == candidate else [at or "."]


compared = mismatches = 0
for candidate_tag, reference_tag in ((tag, ref),) + (() if mode else ((tag + "pu", ref + "pu"),)):
    for candidate_path in sorted(glob.glob(os.path.join(VALIDATION_DIR, f"{candidate_tag}__*.out"))):
        reference_path = os.path.join(
            VALIDATION_DIR, f"{reference_tag}__" + candidate_path.split(f"{candidate_tag}__", 1)[1]
        )
        if not os.path.exists(reference_path):
            continue
        compared += 1
        differences = paths(load(reference_path), load(candidate_path))
        if differences:
            mismatches += 1
            print(
                f"{os.path.basename(candidate_path)}: {len(differences)} differ: "
                + ", ".join(differences[:6])
                + (" …" if len(differences) > 6 else "")
            )
print(f"{compared} compared, {mismatches} differ")
sys.exit(1 if mismatches or not compared else 0)
