"""What the validation scripts share: running the native solver on job files, reading its outputs (one JSON line per
job), the model constants it prints, a recursive comparison of two outputs, and the rule for a deck's exact / product
ratio near its class's cap."""

import functools
import json
import os
import subprocess

VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(VALIDATION_DIR)
PLAN_DIR = os.path.join(VALIDATION_DIR, "plan")
BINARY = os.path.join(REPO, "pkg", "loopopt", "target", "release", "loopopt")  # ./build.sh native


@functools.cache
def constants(binary=BINARY):
    """the model constants the solver itself uses, as `loopopt constants` prints them: chain_n, chain_a, chain_b,
    chain_stop_share ([k, n]), exact_builders_targets, ceil_share, lam_decay, default_star_mix"""
    if not os.path.exists(binary):
        raise FileNotFoundError(f"{binary} is missing: ./build.sh native builds it")
    result = subprocess.run([binary, "constants"], capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(f"constants: solver exited {result.returncode}\n{result.stderr}")
    return json.loads(result.stdout)


def era_dir(era):
    """the era's data folder (pkg/loopopt/data/ERA), where the solver runs: jobs name their data files relative to it"""
    return os.path.join(REPO, "pkg", "loopopt", "data", era)


def run(binary, era, *job_paths, name, env=None):
    """The solver's stdout on these job files (one output line per job), run from the era's data folder. A nonzero
    exit raises, naming `name` and carrying the solver's stderr, whatever the stdout holds."""
    result = subprocess.run(
        [binary, "data.json", *job_paths], cwd=era_dir(era), capture_output=True, text=True, env=env
    )
    if result.returncode:
        raise RuntimeError(f"{name}: solver exited {result.returncode}\n{result.stderr}")
    return result.stdout


def solve(binary, era, job, path, env=None):
    """Writes the job to PATH.json, runs it, keeps the solver's stdout in PATH.out; returns the output"""
    json.dump(job, open(path + ".json", "w"))
    stdout = run(binary, era, path + ".json", name=path, env=env)
    open(path + ".out", "w").write(stdout)
    return last_output(stdout)


def outputs(text):
    """every output in a solver's stdout (or a kept .out file), in job order"""
    return [json.loads(line) for line in text.splitlines() if line.strip()]


def last_output(text):
    """the last output in a solver's stdout; IndexError when there is none, ValueError when it is not JSON"""
    return json.loads([line for line in text.splitlines() if line.strip()][-1])


def read_output(path):
    """the last output in a kept .out file"""
    return last_output(open(path).read())


def read_output_or_none(path):
    """read_output, or None where the file is missing, empty or not JSON"""
    try:
        return read_output(path)
    except OSError, IndexError, ValueError:
        return None


def differences(reference, candidate, skip, limit):
    """Up to `limit` paths where two outputs differ, each with what differs: a type, a key in one only, a list's length,
    a value. Keys in `skip` and keys ending in _ms are not compared; ints and floats are different types."""
    found = []

    def walk(reference, candidate, path):
        if len(found) >= limit:
            return
        if type(reference) is not type(candidate):
            found.append(f"{path}: type {type(reference).__name__} vs {type(candidate).__name__}")
            return
        if isinstance(reference, dict):
            for key in sorted(set(reference) | set(candidate)):
                if key in skip or key.endswith("_ms"):
                    continue
                if key not in reference or key not in candidate:
                    found.append(f"{path}.{key}: only in {'new' if key in candidate else 'ref'}")
                    continue
                walk(reference[key], candidate[key], f"{path}.{key}")
        elif isinstance(reference, list):
            if len(reference) != len(candidate):
                found.append(f"{path}: length {len(reference)} vs {len(candidate)}")
                return
            for i, (reference_item, candidate_item) in enumerate(zip(reference, candidate)):
                walk(reference_item, candidate_item, f"{path}[{i}]")
        elif reference != candidate:
            found.append(f"{path}: {str(reference)[:60]} vs {str(candidate)[:60]}")

    walk(reference, candidate, "")
    return found


# the deck structure classes of the exact caps (`cap_class`: chain-varying targets; a multi-target group counts as 4+)
CAP_CLASSES = ["0-1", "2", "3", "4+"]


def near_cap(ratio, cap):
    """a scored deck's exact / product r is near its class's assumed cap c: past three quarters of the cap's headroom
    over 1 (1.1875 of 1.25; 1.0375 of 1.05). Near the cap, pruning may discard a better deck."""
    return ratio > 1 + 0.75 * (cap - 1)
