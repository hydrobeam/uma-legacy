"""Shared helpers for the research scripts: paths, the run records, game data, intervals and the values table.

Each section script defines `claims()`, returning its Values and Findings, and prints its own detail when run alone.
Without the run records (research/data/runs.jsonl, not included), sections skip record-dependent calculations or
return findings as "needs data". Game-backed checks still require their game inputs."""

import collections, json, math, os, re, sqlite3, sys
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass
from types import ModuleType
from typing import TYPE_CHECKING, Any, NamedTuple, TypedDict

if TYPE_CHECKING:
    import numpy as np

HERE: str = os.path.dirname(os.path.abspath(__file__))
ROOT: str = os.path.dirname(HERE)
DATA: str = os.path.join(HERE, "data")
OUT: str = os.path.join(DATA, "out")  # intermediate outputs of the scripts (job files, solver results)
RUNS: str = os.path.join(DATA, "runs.jsonl")
GD: str = os.path.join(ROOT, "pkg", "loopopt", "data", "global")
MDB: str = os.path.join(ROOT, "game", "global", "master.mdb")
SOLVER: str = os.path.join(ROOT, "pkg", "loopopt", "target", "release", "loopopt")

SCEN: dict[int, str] = {1: "URA Finale", 2: "Unity Cup", 3: "Grand Concert", 4: "Trackblazer"}
DEFAULT: list[int] = [
    20159,
    20049,
    20166,
    20046,
    20161,
    21010,
    20165,
    20203,
]  # skill groups of the default eight targets
UNITY_GOLDS: set[int] = {20049, 20046}  # No Stopping Me!, It's On! (guaranteed in Unity Cup)


# ---------------------------------------------------------------- data
class Hint(TypedDict):
    """one skill hint a run's career showed: tier, skill name, hint level, the sources it came from"""

    tier: str
    name: str
    lv: int
    src: list[str]


class Race(TypedDict):
    """one race a run's career entered"""

    year: int
    turn: str
    race: str
    grade: str
    won: int
    required: bool


class Run(TypedDict):
    """one run record (a line of research/data/runs.jsonl)"""

    id: int
    scenario: int
    outfit: int
    chara: int
    deck: list[list[Any]]  # [card id, limit break, card name]
    contrib: list[dict[str, Any]]
    hints: list[Hint]
    owned: list[int]
    hint_groups: list[list[Any]]  # [group, [[skill id, name, rarity, ...], ...]]
    years: dict[str, list[list[int]]]
    priority: list[str] | None
    focus: str | None
    parents: list[dict[str, Any]]
    affinity: dict[str, list[int]]
    races: list[Race]
    score_max: int
    seq: int
    batch: int


_cache: dict[
    str, Any
] = {}  # loaded data by name, shared by every script in one process (the section scripts add theirs)


def have_runs() -> bool:
    return os.path.exists(RUNS)


def runs() -> list[Run] | None:
    """the run records, one dict per run, in record order (seq); None without them"""
    if not have_runs():
        return None
    if "runs" not in _cache:
        _cache["runs"] = sorted((json.loads(line) for line in open(RUNS, encoding="utf-8")), key=lambda r: r["seq"])
    return _cache["runs"]


def run_list() -> list[dict[str, Any]] | None:
    """the run list's rows (every completed run, with uma-it's rank-score estimates; more runs than the records),
    one dict per run; None without it"""
    path = os.path.join(DATA, "summary.jsonl")
    if not os.path.exists(path):
        return None
    if "list" not in _cache:
        _cache["list"] = [json.loads(line) for line in open(path, encoding="utf-8")]
    return _cache["list"]


def gjson(name: str) -> Any:
    """a tracked Global data file (pkg/loopopt/data/global/NAME)"""
    if name not in _cache:
        _cache[name] = json.load(open(os.path.join(GD, name), encoding="utf-8"))
    return _cache[name]


def cards() -> dict[int, dict[str, Any]]:
    """support card id -> its record in web_cards.json"""
    return {c["id"]: c for c in gjson("web_cards.json")["cards"]}


def skills() -> dict[int, tuple[str, str | None]]:
    """group -> (white name, gold name or None)"""
    return {group: (white, gold) for group, white, gold in gjson("web_skills.json")["skills"]}


def skill_names() -> dict[str, tuple[int, int]]:
    """skill name -> (group, 1 white / 2 gold)"""
    out: dict[str, tuple[int, int]] = {}
    for group, white, gold in gjson("web_skills.json")["skills"]:
        out[white] = (group, 1)
        if gold:
            out[gold] = (group, 2)
    return out


def factor_groups() -> dict[int, int]:
    """skill factor id // 100 -> skill group"""
    return {int(k): v for k, v in gjson("career.json")["factors"].items()}


def mdb() -> sqlite3.Connection | None:
    """master.mdb (game/global/master.mdb), or None"""
    if not os.path.exists(MDB):
        return None
    if "mdb" not in _cache:
        _cache["mdb"] = sqlite3.connect(f"file:{MDB}?mode=ro", uri=True)
    return _cache["mdb"]


def gametora() -> ModuleType:
    """the repo's one GameTora reader (pkg/loopopt/export/gametora.py)"""
    sys.path.insert(0, os.path.join(ROOT, "pkg", "loopopt", "export"))
    import gametora

    return gametora


def bare(card_name: str) -> str:
    """'[Title] Character' or 'SSR Character' -> 'Character' (as hint sources name a card)"""
    name = card_name.split("] ", 1)[1] if card_name.startswith("[") else card_name
    return name.split(" ", 1)[1] if name.split(" ", 1)[0] in ("SSR", "SR", "R") else name


# ---------------------------------------------------------------- statistics
class Interval(NamedTuple):
    """a two-sided interval (NaN bounds when it cannot be computed)"""

    low: float
    high: float


def wilson(k: float, n: float, z: float = 1.96) -> Interval:
    """the Wilson score interval of k successes in n trials (z = 1.96: 95%)"""
    if n == 0:
        return Interval(float("nan"), float("nan"))
    p = k / n
    denominator = 1 + z * z / n
    centre = p + z * z / (2 * n)
    half_width = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return Interval((centre - half_width) / denominator, (centre + half_width) / denominator)


def div(a: float, b: float) -> float:
    return a / b if b else float("nan")


def binom_pmf(n: int, p: float) -> list[float]:
    """P(X = 0..n) for X ~ Binomial(n, p)"""
    return [math.comb(n, e) * p**e * (1 - p) ** (n - e) for e in range(n + 1)]


# ---------------------------------------------------------------- intervals on rows grouped by run
class ClusterRatio(NamedTuple):
    """observed / predicted with its run-clustered interval (cluster_ci)"""

    ratio: float
    low: float
    high: float
    runs: int


def cluster_ci(obs: Iterable[float], exp: Iterable[float], run: Iterable[Any], z: float = 1.96) -> ClusterRatio:
    """observed / predicted over rows that come in groups (one run gives several rows, which need not be independent):
    (ratio, low, high, number of runs). R = sum obs / sum exp; its variance by the ratio estimator's linearisation over
    runs, Var(R) = C / (C - 1) * sum_runs (O_c - R E_c)^2 / E^2 (Cochran, Sampling Techniques, ch. 6: each run's
    residual against the model, so it holds whichever way rows of one run are correlated). With exp = 1 per row this is
    the observed rate with a run-clustered interval. When every run gives one row and exp = 1 per row, the Wilson
    interval is used instead (better for small counts). Bounds are clipped at 0."""
    obs, exp, run = list(obs), list(exp), list(run)
    E = sum(exp)
    O = sum(obs)
    n = len(obs)
    if not n or not E:
        return ClusterRatio(float("nan"), float("nan"), float("nan"), 0)
    R = O / E
    per_run: dict[Any, list[float]] = collections.defaultdict(lambda: [0.0, 0.0])  # run -> [O_c, E_c]
    for o, e, r in zip(obs, exp, run):
        sums = per_run[r]
        sums[0] += o
        sums[1] += e
    C = len(per_run)
    if n == C and all(e == 1 for e in exp):  # one row per run
        lo, hi = wilson(round(O), n, z)
        return ClusterRatio(R, lo, hi, C)
    if C < 2:
        return ClusterRatio(R, float("nan"), float("nan"), C)
    se = math.sqrt(C / (C - 1) * sum((o_c - R * e_c) ** 2 for o_c, e_c in per_run.values())) / E
    return ClusterRatio(R, max(0.0, R - z * se), R + z * se, C)


class RobustInterval(NamedTuple):
    """a fitted parameter's run-clustered interval and standard error (robust_interval)"""

    low: float
    high: float
    se: float


def robust_interval(
    run_ll: Callable[[float], Mapping[Any, float]], theta: float, h: float = 1e-3, z: float = 1.96
) -> RobustInterval:
    """run-clustered (Huber-White sandwich) interval for a one-parameter maximum-likelihood fit whose observations
    come several to a run: run_ll(t) -> {run: that run's log-likelihood at parameter t}. Each run's score is the
    central difference of its log-likelihood at the fit, the information the second difference of the total;
    se = sqrt(C / (C - 1) sum_runs score^2) / |second difference|. Returns (low, high, se)."""
    below, at, above = run_ll(theta - h), run_ll(theta), run_ll(theta + h)
    runs = list(at)
    scores = [(above[r] - below[r]) / (2 * h) for r in runs]
    H = (sum(above.values()) - 2 * sum(at.values()) + sum(below.values())) / h**2
    C = len(runs)
    se = math.sqrt(C / (C - 1) * sum(x * x for x in scores)) / abs(H) if H and C > 1 else float("nan")
    return RobustInterval(theta - z * se, theta + z * se, se)


class WaldTest(NamedTuple):
    """a Wald chi2 test: the statistic, its degrees of freedom and p"""

    chi2: float
    df: int
    p: float


def cluster_chi2(resid: list[tuple[Any, float]], run: Iterable[Any]) -> WaldTest:
    """joint test that several groups' residuals (observed - predicted, one vector entry per group, e.g. reliability
    buckets) are all zero, when one run's rows fall in several groups: Wald chi2 = r' V^-1 r with V the run-clustered
    covariance of the group sums (sum over runs of each run's residual vector, outer product, times C / (C - 1)).
    `resid` is a list of (group index, residual) per row. Returns (chi2, degrees of freedom, p)."""
    import numpy as np
    from scipy.stats import chi2

    groups = sorted({g for g, _ in resid})
    index = {g: i for i, g in enumerate(groups)}
    k = len(groups)
    per_run: dict[Any, np.ndarray] = collections.defaultdict(lambda: np.zeros(k))  # run -> its residual sum per group
    for (g, d), r in zip(resid, run):
        per_run[r][index[g]] += d
    M = np.array(list(per_run.values()))
    C = len(M)
    total = M.sum(0)
    V = C / (C - 1) * (M.T @ M)
    x2 = float(total @ np.linalg.solve(V, total))
    return WaldTest(x2, k, float(chi2.sf(x2, k)))


# ---------------------------------------------------------------- held-out comparison
K_FOLDS: int = 5


def fold(run_id: int, k: int = K_FOLDS) -> int:
    """the cross-validation group of a run: its id mod k (fixed, unrelated to record order or who ran it)"""
    return run_id % k


class HeldOutComparison(NamedTuple):
    """compare_held_out: the summed per-run difference in nats, its standard error and z"""

    nats: float
    se: float
    z: float


def compare_held_out(a: Sequence[float] | np.ndarray, b: Sequence[float] | np.ndarray) -> HeldOutComparison:
    """paired comparison of two models' held-out log-likelihoods, one entry per run (each run scored by the model fitted
    on the other folds): (sum of a - b in nats, its standard error, z). The normal approximation assumes independent
    per-run differences; this standard error does not account for overlapping training folds."""
    import numpy as np

    d = np.asarray(a, float) - np.asarray(b, float)
    se = float(np.sqrt(len(d)) * d.std(ddof=1)) if len(d) > 1 else float("nan")
    return HeldOutComparison(float(d.sum()), se, float(d.sum() / se) if se else float("nan"))


# ---------------------------------------------------------------- values and findings
def key_of(section: str, name: str) -> str:
    """a value's key in results.json and in the document template: section.slug_of_the_name"""
    for symbol, word in (
        ("+-", " pm "),
        ("±", " pm "),
        ("%", " pct "),
        (">=", " ge "),
        ("<", " lt "),
        (">", " gt "),
        ("/", " per "),
    ):
        name = name.replace(symbol, word)
    return f"{section}.{re.sub(r'[^a-z0-9]+', '_', name.lower()).strip('_')}"


@dataclass
class Value:
    """One number or rendered table a section computes: its name (the key in results.json and in the document template is
    key_of(section, name)) and the computed value; None when it could not be computed (`status` says why, e.g.
    'needs data' without the run records)."""

    section: str
    name: str
    got: float | int | str | None = None
    status: str = ""
    note: str = ""

    def missing(self) -> bool:
        """no number: None, or a NaN"""
        return self.got is None or (isinstance(self.got, float) and math.isnan(self.got))

    def shown(self) -> str:
        """the value as the table prints it (a table value by its line count)"""
        if isinstance(self.got, str):
            return f"(table, {self.got.count(chr(10)) + 1} lines)"
        return "-" if self.missing() else f"{self.got:,.6g}"


@dataclass
class Finding:
    """One conclusion RESEARCH.md draws, tested on the reproduced numbers: `holds` is the test's outcome (None when the
    numbers could not be computed), `evidence` the numbers it rests on, as printed."""

    section: str
    statement: str
    holds: bool | None
    evidence: str = ""

    def verdict(self) -> str:
        """'needs data', 'holds' or 'FAILS'"""
        return "needs data" if self.holds is None else "holds" if self.holds else "FAILS"


def finding(
    section: str, statement: str, test: bool | Callable[[], Any], evidence: str | Callable[[], str] = ""
) -> Finding:
    """a Finding from a test that may need missing numbers: `test` is a bool or a function returning one; TypeError, KeyError,
    ValueError or ZeroDivisionError gives holds = None"""
    try:
        holds = bool(test() if callable(test) else test)  # a missing number surfaces as one of these errors
    except TypeError, KeyError, ValueError, ZeroDivisionError:
        holds = None
    if callable(evidence):
        try:
            evidence = evidence()
        except TypeError, KeyError, ValueError, ZeroDivisionError:
            evidence = ""
    return Finding(section, statement, holds, evidence)


def say_missing() -> None:
    print("run records not found (research/data/runs.jsonl, not included): data claims skipped")


def findings_table(fs: list[Finding], out: Callable[[str], Any] = print) -> None:
    if not fs:
        return
    out("\nFindings (each conclusion, tested on the reproduced numbers):")
    for f in fs:
        out(f"{f.section:7s}  {f.verdict():10s}  {f.statement}")
        if f.evidence:
            out(f"{'':19s}{f.evidence}")


def table(items: Sequence[Value | Finding], out: Callable[[str], Any] = print) -> None:
    """print every value (name, computed, status or note), then the findings"""
    vals = [c for c in items if isinstance(c, Value)]
    name_width = max([*(len(c.name) for c in vals), 10])
    out(f"{'section':7s}  {'value':{name_width}s}  {'computed':>14s}")
    for c in vals:
        remark = c.status or c.note
        out(f"{c.section:7s}  {c.name:{name_width}s}  {c.shown():>14s}" + (f"  ({remark})" if remark else ""))
    findings_table([f for f in items if isinstance(f, Finding)], out)


def main(claims_fn: Callable[[], list[Value | Finding]]) -> None:
    if not have_runs():
        say_missing()
    table(claims_fn())
