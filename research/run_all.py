"""Run every section script: print each computed value and each tested conclusion, and write research/results.json.

  uv run --group research research/run_all.py [--quick] [--force] [--jobs=N]     (or ./build.sh research)

Each script's results are kept in research/data/out/claims/ with a fingerprint of its code (its source, the research
scripts it imports, common.py, val/joint_check.py) and of the data inputs' sizes and times (DATA_INPUTS, val's reference
outputs); a script whose fingerprint is unchanged is not run again. --force recomputes everything.

--quick skips the section 13 variant over every other skill (a solver run of several minutes the first time; its
results are reused while nothing it reads changes). The computed values go to research/results.json, from which
research/render.py renders docs/RESEARCH.md. Without the run records (research/data/runs.jsonl, not included)
the data claims read "needs data" and the derived ones still run."""

import collections, concurrent.futures, glob, hashlib, importlib, inspect, json, os, pickle, re, sys, time, traceback

for v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ.setdefault(v, "1")  # one core
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from typing import Any, NamedTuple
import common
from common import Value, Finding


# inputs every script may read; a change to any of them (size or time) recomputes every script
DATA_INPUTS: list[str] = [
    "research/data/runs.jsonl",
    "research/data/summary.jsonl",
    "game/global/master.mdb",
    "pkg/loopopt/data",
    "pkg/loopopt/src",
    "pkg/loopopt/export",
    "pkg/loopopt/target/release/loopopt",
    "cache/gametora",
    "val/jobs.json",
    "val/jobs",
    "webapp/src/data.js",
]
CACHE: str = os.path.join(common.OUT, "claims")
JOBS: int = 4  # scripts run at once, one core each (--jobs=N; 1 runs them one after another)


def data_stamp() -> list[tuple[str, int, int]]:
    """(path, size, modification time in ns) of every data input (content hashing 100 MB of inputs on every run would
    cost more than it saves); val's reference outputs included, which section 6 reads. Plain tuples: their repr is part
    of every fingerprint"""
    items: list[tuple[str, int, int]] = []
    paths = [os.path.join(common.ROOT, x) for x in DATA_INPUTS] + sorted(
        glob.glob(os.path.join(common.ROOT, "val", "ref*__*.out"))
    )
    for p in paths:
        if os.path.isdir(p):
            for d, _, fs in os.walk(p):
                for f in sorted(fs):
                    path = os.path.join(d, f)
                    st = os.stat(path)
                    items.append((os.path.relpath(path, common.ROOT), st.st_size, st.st_mtime_ns))
        elif os.path.exists(p):
            st = os.stat(p)
            items.append((os.path.relpath(p, common.ROOT), st.st_size, st.st_mtime_ns))
    return items


def code_of(name: str, seen: set[str] | None = None) -> str:
    """the source of a script and of every research script it imports, transitively (fingerprint() adds common.py and
    val/joint_check.py)"""
    seen = set() if seen is None else seen
    if name in seen:
        return ""
    seen.add(name)
    src = open(os.path.join(common.HERE, name + ".py")).read()
    deps = sorted(set(re.findall(r"^\s*import (s\d\d_\w+)", src, re.M)))
    return src + "".join(code_of(d, seen) for d in deps)


def fingerprint(name: str, kw: dict[str, Any], stamp: list[tuple[str, int, int]]) -> str:
    """a hash of everything a script's results depend on: its code, common.py, val/joint_check.py, its arguments and the
    data stamp"""
    h = hashlib.sha256()
    for part in (
        code_of(name),
        open(os.path.join(common.HERE, "common.py")).read(),
        open(os.path.join(common.ROOT, "val", "joint_check.py")).read(),
        repr(sorted(kw.items())),
        repr(stamp),
    ):
        h.update(part.encode())
    return h.hexdigest()


AFTER: dict[str, str] = {
    "s10_random_events": "s13_calibration",
    "s11_conditional": "s13_calibration",
}  # read s13's solver job files

Claims = list[Value | Finding]


class Pending(NamedTuple):
    """a script whose cached results are missing or stale"""

    kwargs: dict[str, Any]  # passed to its claims()
    fingerprint: str
    cache_file: str  # where its results are kept (a pickle of (fingerprint, claims))


def compute(name: str, kw: dict[str, Any]) -> tuple[Claims, float] | tuple[None, str]:
    """one script's claims (run in a worker process): (claims, seconds) or (None, traceback text)"""
    try:
        m = importlib.import_module(name)
        t0 = time.time()
        return m.claims(**kw), time.time() - t0
    except Exception:
        return None, traceback.format_exc()


def reuse_unchanged(
    names: list[str], quick: bool, force: bool, stamp: list[tuple[str, int, int]]
) -> tuple[dict[str, Claims], dict[str, Pending]]:
    """each script's cached claims where its fingerprint is unchanged (none with --force); the rest are pending"""
    got: dict[str, Claims] = {}
    pending: dict[str, Pending] = {}
    for name in names:
        m = importlib.import_module(name)
        kw = {"wide": not quick} if "wide" in inspect.signature(m.claims).parameters else {}
        fp = fingerprint(name, kw, stamp)
        cache_file = os.path.join(CACHE, name + ".pkl")
        hit = None
        if not force and os.path.exists(cache_file):
            with open(cache_file, "rb") as f:
                hit = pickle.load(f)
        if hit and hit[0] == fp:
            print(f"{name}: unchanged, results reused", file=sys.stderr)
            got[name] = hit[1]
        else:
            pending[name] = Pending(kw, fp, cache_file)
    return got, pending


def compute_pending(pending: dict[str, Pending], jobs: int, got: dict[str, Claims]) -> None:
    """run the pending scripts in parallel, one core each, into `got`, caching each result that has no error; a script
    that reads another's files (AFTER) waits for it"""
    waves = [[n for n in pending if AFTER.get(n) not in pending], [n for n in pending if AFTER.get(n) in pending]]
    with concurrent.futures.ProcessPoolExecutor(max_workers=max(1, jobs)) as ex:
        for wave in waves:
            futures = {n: ex.submit(compute, n, pending[n].kwargs) for n in wave}
            for n in wave:
                res, info = futures[n].result()
                if res is None:
                    print(info, file=sys.stderr)
                    got[n] = [Value(n[1:3].lstrip("0"), f"{n} failed", status="error")]
                    continue
                print(f"{n}: computed in {info:.0f} s", file=sys.stderr)
                got[n] = res
                if common.have_runs() and not any(isinstance(c, Value) and c.status == "error" for c in res):
                    with open(pending[n].cache_file, "wb") as f:
                        pickle.dump((pending[n].fingerprint, res), f)


def main() -> None:
    quick = "--quick" in sys.argv
    force = "--force" in sys.argv
    jobs = int(next((a.split("=")[1] for a in sys.argv if a.startswith("--jobs=")), JOBS))
    stamp = data_stamp()
    os.makedirs(CACHE, exist_ok=True)
    if not common.have_runs():
        common.say_missing()
    names = [os.path.basename(p)[:-3] for p in sorted(glob.glob(os.path.join(common.HERE, "s[0-9][0-9]_*.py")))]
    got, pending = reuse_unchanged(names, quick, force, stamp)
    compute_pending(pending, jobs, got)
    allc = [c for n in names for c in got[n]]
    common.table(allc)
    write_results(allc, fresh=common.have_runs() and not quick)
    vals = [c for c in allc if isinstance(c, Value)]
    fs = [f for f in allc if isinstance(f, Finding)]
    n = collections.Counter("computed" if not c.missing() else (c.status or "not computed") for c in vals)
    print("\nvalues: " + ", ".join(f"{v}: {k}" for k, v in n.most_common()))
    n = collections.Counter(f.verdict() for f in fs)
    print("findings: " + ", ".join(f"{v}: {k}" for k, v in n.most_common()))
    unused(vals)


def write_results(items: list[Value | Finding], fresh: bool) -> None:
    """research/results.json: every computed value by key, and the findings with their verdicts. A full run with the run
    records rebuilds it; otherwise (a clone, or --quick) what this run computes replaces the committed entries and the
    rest are kept, so the whole document still renders."""
    path = os.path.join(common.HERE, "results.json")
    old = json.load(open(path)) if os.path.exists(path) and not fresh else {"values": {}, "findings": []}
    values: dict[str, Any] = dict(old["values"])
    seen: dict[str, str] = {}
    for c in items:
        if not isinstance(c, Value) or c.missing():
            continue
        k = common.key_of(c.section, c.name)
        if k in seen and seen[k] != c.name:
            sys.exit(f"two values share the key {k}: {seen[k]!r} and {c.name!r}")
        seen[k] = c.name
        values[k] = c.got if isinstance(c.got, str) else float(c.got)
    by_statement = {(f["section"], f["statement"]): f for f in old["findings"]}
    for f in items:  # a finding evaluated now replaces the committed one; one that needs data only fills a gap
        if isinstance(f, Finding) and (f.holds is not None or (f.section, f.statement) not in by_statement):
            by_statement[(f.section, f.statement)] = {
                "section": f.section,
                "statement": f.statement,
                "verdict": f.verdict(),
                "evidence": f.evidence,
            }
    findings = list(by_statement.values())
    for sec in {f["section"] for f in findings}:
        mine = [f for f in findings if f["section"] == sec]
        values[f"{sec}.findings_total"] = len(mine)
        values[f"{sec}.findings_holding"] = sum(f["verdict"] == "holds" for f in mine)
    json.dump(
        {"values": dict(sorted(values.items())), "findings": findings}, open(path, "w"), indent=1, ensure_ascii=False
    )
    print(f"wrote research/results.json: {len(values)} values, {len(findings)} findings")


def unused(vals: list[Value]) -> None:
    """computed values the document template never uses: either missing from the document or not needed"""
    t = open(os.path.join(common.ROOT, "docs", "RESEARCH.template.md"), encoding="utf-8").read()
    used = set(re.findall(r"\{\{([0-9]+\.[a-z0-9_]+)\|", t))
    un = sorted({common.key_of(c.section, c.name) for c in vals if not c.missing()} - used)
    print(
        f"computed values the document does not use: {len(un)}"
        + (": " + ", ".join(un[:40]) + (" ..." if len(un) > 40 else "") if un else "")
    )


if __name__ == "__main__":
    main()
