"""Shared helpers for the research scripts: paths, the run records, game data, intervals and the values table.

Each section script defines `claims()`, returning its Values and Findings, and prints its own detail when run alone.
Without the run records (research/data/runs.jsonl, not included), sections skip record-dependent calculations or
return findings as "needs data". Game-backed checks still require their game inputs."""

import collections, contextlib, hashlib, json, math, os, random, re, sqlite3, subprocess, sys, time
from collections.abc import Callable, Iterable, Iterator, Mapping, Sequence
from dataclasses import dataclass
from itertools import pairwise
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
    """skill name -> (group, 1 white / 2 gold): web_skills.json's `names` (the ○ beside a catalog ◎ included)"""
    return {name: (group, level) for name, (group, level) in gjson("web_skills.json")["names"].items()}


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


def compare_held_out(
    a: Sequence[float] | np.ndarray, b: Sequence[float] | np.ndarray, clusters: Sequence[Any] | np.ndarray | None = None
) -> HeldOutComparison:
    """paired comparison of two models' held-out log-likelihoods, one entry per run (each run scored by the model fitted
    on the other folds), or per row with `clusters` naming each row's unit: (sum of a - b in nats, its standard error, z).
    The standard error treats the units' summed differences as independent (each run, or each cluster: a card when the
    compared term is a card property, since rows of one card share its residual); it does not account for overlapping
    training folds."""
    import numpy as np

    d = np.asarray(a, float) - np.asarray(b, float)
    if clusters is not None:
        _, ix = np.unique(np.asarray(clusters), return_inverse=True)
        d_units = np.bincount(ix, weights=d)
    else:
        d_units = d
    se = float(np.sqrt(len(d_units)) * d_units.std(ddof=1)) if len(d_units) > 1 else float("nan")
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


# ---------------------------------------------------------------- affinity: relation points, lineages, the six affinities and single-ancestor trials (sections 2, 3, 5)
BASE: tuple[float, float, float] = (0.03, 0.06, 0.09)  # b by the spark's stars (1, 2, 3)


G_OF: dict[int, tuple[int, int]] = {0: (1, 2), 3: (4, 5)}  # ancestor slots: 0 P1, 1 G1a, 2 G1b, 3 P2, 4 G2a, 5 G2b


PAIR_ROW: str = "parent 1 × parent 2"  # the record's affinity row of the two parents


GRANDPARENT_ROWS: dict[int, str] = {
    1: "parent 1 × grandparent 1a",
    2: "parent 1 × grandparent 1b",
    4: "parent 2 × grandparent 2a",
    5: "parent 2 × grandparent 2b",
}  # by slot


# relation points of the groups holding every listed character
Relation = Callable[..., int]


class Relations(NamedTuple):
    """master.mdb's succession tables"""

    points: Relation  # relation points of the groups holding every listed character (set semantics)
    factor_type: dict[int, int]  # succession factor id -> its type (4: a skill white)


class Trial(NamedTuple):
    """a single-ancestor trial (trials()): one (ancestor, skill white spark, career year 2 or 3)"""

    category: str  # the ancestor: 'P' / 'G' an ordinary parent / grandparent; 'P_selfgp' a parent with a
    # grandparent of the trainee's character, 'P_gpp' one with a grandparent of her own;
    # 'G_self' a grandparent of the trainee's character, 'G_eqP' one equal to her parent
    affinity_h0: int  # the ancestor's affinity A with tri(T, P, T) = 0
    affinity_h1: int  # ... with tri(T, P, T) = pair(T, P)
    base: float  # b by the spark's stars
    passed: bool  # the factor showed among the year's inspirations
    run: int  # run id
    race_part: int | None  # the shared-win part of affinity_h0 (trials(race=True)), else None


def relations() -> Relations:
    db = mdb()
    points_of_group = dict(db.execute("select relation_type, relation_point from succession_relation"))
    groups_of: dict[int, set[int]] = collections.defaultdict(set)  # character -> the relation groups holding her
    for t, ch in db.execute("select relation_type, chara_id from succession_relation_member"):
        groups_of[ch].add(t)
    ftype = dict(db.execute("select factor_id, factor_type from succession_factor"))

    def points(*xs: int) -> int:  # relation points of the groups holding every listed character (set semantics)
        return sum(points_of_group[t] for t in set.intersection(*[groups_of[x] for x in xs]))

    return Relations(points, ftype)


def lineage(r: Run) -> list[dict[str, Any]] | None:
    """[P1, G1a, G1b, P2, G2a, G2b] as ancestor dictionaries, or None when a slot or its character is missing"""
    out: list[dict[str, Any]] = []
    for p in r.get("parents") or []:
        out.append(p)
        out.extend((p.get("gps") or [])[:2])
    if len(out) != 6 or any(a is None or not a.get("chara") for a in out):
        return None
    return out


def has_affinity_rows(aff: dict[str, list[int]]) -> bool:
    """the record's affinity table has the parents' row and every parent x grandparent row"""
    return all(label in aff for label in [*GRANDPARENT_ROWS.values(), PAIR_ROW])


def six_affinities(
    T: int, A: list[int], aff: dict[str, list[int]], S: Relation, h: int = 0, part: str = "all"
) -> dict[int, int]:
    """{slot: affinity} for the six ancestors (slots as G_OF) of trainee T, A the ancestors' characters by slot:
    relation points S from master.mdb plus 3 x the record's shared G1 wins per pair (aff: row label -> [base, shared,
    bonus, total]); when G equals T or P, h = 0 sets tri(T, P, G) to 0 and h = 1 to pair(T, P).
    part 'rel' or 'race' keeps only relation points or shared-win terms"""
    w = 0 if part == "rel" else 3
    shared_parents = w * aff[PAIR_ROW][1]
    shared_grandparent = {g: w * aff[label][1] for g, label in GRANDPARENT_ROWS.items()}
    if part == "race":
        S = lambda *xs: 0

    def tri(p: int, g: int) -> int:
        if A[g] in (T, A[p]):
            return S(T, A[p]) if h else 0
        return S(T, A[p], A[g])

    d: dict[int, int] = {}
    for p, o in ((0, 3), (3, 0)):
        d[p] = S(T, A[p]) + S(A[p], A[o]) + shared_parents + sum(tri(p, g) + shared_grandparent[g] for g in G_OF[p])
        for g in G_OF[p]:
            d[g] = tri(p, g) + shared_grandparent[g]
    return d


def category(slot: int, A: list[int], T: int) -> str:
    """the ancestor in `slot` as a Trial category (A the ancestors' characters by slot, T the trainee's)"""
    if slot in (0, 3):
        gs = G_OF[slot]
        return "P_selfgp" if any(A[g] == T for g in gs) else "P_gpp" if any(A[g] == A[slot] for g in gs) else "P"
    p = 0 if slot < 3 else 3
    return "G_self" if A[slot] == T else "G_eqP" if A[slot] == A[p] else "G"


def trials(R: list[Run], race: bool = False) -> list[Trial]:
    """every single-ancestor trial of the runs: a skill white spark carried by exactly one of the six ancestors, in
    career years 2 and 3; runs where the trainee equals a parent or the parents are one character are left out"""
    S, ftype = relations()
    out: list[Trial] = []
    for r in R:
        anc = lineage(r)
        if anc is None:
            continue
        T = r["chara"]
        A = [a["chara"] for a in anc]
        if len({T, A[0], A[3]}) < 3:
            continue
        aff = r.get("affinity") or {}
        if not has_affinity_rows(aff):
            continue
        yrs = r.get("years") or {}
        if "2" not in yrs or "3" not in yrs:
            continue
        inspired = {y: {f for f, _ in yrs[y]} for y in ("2", "3")}  # year -> factors shown among its inspirations
        aff0, aff1 = six_affinities(T, A, aff, S, h=0), six_affinities(T, A, aff, S, h=1)
        race_parts = six_affinities(T, A, aff, S, h=0, part="race") if race else None
        sparks = [
            {f // 100: f for f, _ in (a.get("factors") or [])} for a in anc
        ]  # per slot: factor group -> factor id
        carriers = collections.Counter(n for s in sparks for n in s)
        for i in range(6):
            cat = category(i, A, T)
            for n, f in sparks[i].items():
                if ftype.get(f) != 4 or carriers[n] != 1:
                    continue
                for y in ("2", "3"):
                    out.append(
                        Trial(
                            cat,
                            aff0[i],
                            aff1[i],
                            BASE[f % 10 - 1],
                            f in inspired[y],
                            r["id"],
                            race_parts[i] if race_parts is not None else None,
                        )
                    )
    return out


# ---------------------------------------------------------------- card events, chain-event markers and their background, chain ends, the choice rule (sections 8-10, 13)
# ---------------------------------------------------------------- markers (GameTora card events)
# one card event: (its chain link or None, [choice -> [(outcome's skills, ends the chain)]])
CardEvent = tuple[int | None, list[list[tuple[set[str], Any]]]]


@dataclass
class GameData:
    """game(): what the markers are read from"""

    cards: dict[int, dict[str, Any]]  # web_cards.json by id
    skill_names: dict[int, str]  # skill id -> name (master.mdb text_data 47)
    pool: dict[int, set[str]]  # card -> every skill its events can give
    markers: dict[int, dict[int, set[str]]]  # card -> observable link -> its marker skills
    card_events: dict[str, Any]  # card_events.json 'cards', by card id as a string


_game: list[GameData] = []  # game()'s cache


def skill_set(hs: Iterable[tuple[str, Any, Any]], skill_names: dict[int, str]) -> set[str]:
    """the skill names of GameTora hints ('hint': one skill id, else a list of them)"""
    out: set[str] = set()
    for kind, sid, _ in hs:
        for i in [sid] if kind == "hint" else sid:
            if i in skill_names:
                out.add(skill_names[i])
    return out


def card_events(gt: Any, skill_names: dict[int, str], link_of: dict[int, int]) -> dict[int, list[CardEvent]]:
    """card -> its SR/SSR events, decoded"""
    evs: collections.defaultdict[int, list[CardEvent]] = collections.defaultdict(list)
    for f in ("training_events_sr", "training_events_ssr"):
        for cid, es in gt.load(f):
            for e in es:
                chs = [[(skill_set(h, skill_names), end) for h, end in gt.outcomes(ch[1])] for ch in gt.choices(e[1])]
                evs[cid].append((link_of.get(int(e[2])), chs))
    return evs


def hinted_skills(x: Any, acc: set[str], gt: Any, skill_names: dict[int, str]) -> None:
    """add to acc every skill hinted anywhere in a nested GameTora event row"""
    if isinstance(x, list):
        if len(x) == 2 and isinstance(x[1], list) and x[1] and all(isinstance(k, int) for k in x[1]):
            acc |= skill_set(gt.hints(x[1]), skill_names)
        for y in x:
            hinted_skills(y, acc, gt, skill_names)


def skill_pools(
    gt: Any,
    evs: dict[int, list[CardEvent]],
    cards: dict[int, dict[str, Any]],
    skill_names: dict[int, str],
    cev: dict[str, Any],
) -> dict[int, set[str]]:
    """every skill each card's events can give: SR/SSR events, pal / group events, card_events.json"""
    pool: collections.defaultdict[int, set[str]] = collections.defaultdict(set)
    for cid, es in evs.items():
        for _, chs in es:
            pool[cid] |= {s for ch in chs for o, _ in ch for s in o}
    for kindf in ("friend", "group"):
        for row in gt.load(f"training_events_{kindf}"):
            acc: set[str] = set()
            hinted_skills(row[1:], acc, gt, skill_names)
            for cid, c in cards.items():
                if c["pal"] and ((kindf == "friend" and c["chara"] == row[0]) or (kindf == "group" and cid == row[0])):
                    pool[cid] |= acc
    for cid, e in cev.items():
        pool[int(cid)] |= (
            set(e.get("always", [])) | {n for n, _ in e.get("disabled", [])} | {n for n, _ in e.get("chance", [])}
        )
    return pool


def link_markers(evs: dict[int, list[CardEvent]]) -> dict[int, dict[int, set[str]]]:
    """card -> link -> skills unique to the link's single event, when every choice and outcome carries one"""
    markers: dict[int, dict[int, set[str]]] = {}
    for cid, es in evs.items():
        per: collections.defaultdict[int, list[list[list[tuple[set[str], Any]]]]] = collections.defaultdict(list)
        cnt: collections.Counter[str] = collections.Counter()  # skill -> events of the card that can give it
        for lk, chs in es:
            cnt.update({s for ch in chs for o, _ in ch for s in o})
            if lk:
                per[lk].append(chs)
        out: dict[int, set[str]] = {}
        for lk, lst in per.items():
            if len(lst) != 1 or not lst[0]:
                continue
            uniq = {s for ch in lst[0] for o, _ in ch for s in o if cnt[s] == 1}
            if uniq and all(o & uniq for ch in lst[0] for o, _ in ch):
                out[lk] = uniq
        markers[cid] = out
    return markers


def game() -> GameData:
    """support cards, skill names, per-card skill pools, observable links with their markers, and exported card events
    (chain links from master.mdb single_mode_story_data)"""
    if _game:
        return _game[0]
    gt = gametora()
    db = mdb()
    card_table = cards()
    sk = dict(db.execute("select [index],text from text_data where category=47"))
    link_of = dict(db.execute("select story_id,show_progress_1 from single_mode_story_data where show_progress_1>0"))
    evs = card_events(gt, sk, link_of)
    cev = gjson("card_events.json")["cards"]
    _game.append(
        GameData(
            cards=card_table,
            skill_names=sk,
            pool=skill_pools(gt, evs, card_table, sk, cev),
            markers=link_markers(evs),
            card_events=cev,
        )
    )
    return _game[0]


def reward_names(rewards: Iterable[Any]) -> set[str]:
    """skill names of card_events.json rewards (a name, or [name, chance])"""
    return {x if isinstance(x, str) else x[0] for x in rewards}


def chosen_end(cid: int) -> tuple[int, set[str], set[str]] | None:
    """(link, the ending choice's skills, the continuing choice's skills there) for a card with a separate choice that
    ends its chain before its last link; None otherwise"""
    g = game()
    ev = g.card_events.get(str(cid), {})
    ends = [(e[0], reward_names(e[1])) for e in ev.get("end", []) if 0 < e[0] < g.cards[cid]["links"]]
    if not ends:
        return None
    link, ending = min(ends, key=lambda e: e[0])
    continuing = {s for e in ev.get("chain", []) if e[0] == link for s in reward_names(e[1])}
    return link, ending, continuing


# ---------------------------------------------------------------- the choice rule's three cases
# Where a run shows its priority list and a prioritised skill sits on a choice other than the top one ("steered"):
#   one-sided  only the steered choice holds a prioritised skill, and none of its skills was at hint Lv 5 from another
#              source when the event fired: IT takes it;
#   maxed      a steered skill was at hint Lv 5 from other sources before the event (maxed_elsewhere(): at Lv 5 at the
#              end, from another source, and never credited to the card's event, whose credit would show it below Lv 5
#              then);
#   both       the top choice holds a prioritised skill too.
STEER_CASES: tuple[str, ...] = ("one-sided", "maxed", "both")


class SteerReading(NamedTuple):
    event: str
    case: str
    taken: bool  # IT took the steered choice


def steer_case(priority: Sequence[str], steered: set[str], default: set[str], steered_maxed: bool) -> str | None:
    """the case of an event on a shown priority list; None when no steered skill is prioritised (IT keeps the top)"""
    if not steered & set(priority):
        return None
    if default & set(priority):
        return "both"
    return "maxed" if steered_maxed else "one-sided"


def names_source(src: str, source: str) -> bool:
    """a hint credit names `source` ("<card> event"), alone or in a shared credit ("A or <card> event")"""
    return src.endswith(" event") and source[:-6] in [y.strip() for y in src[:-6].split(" or ")]


def maxed_elsewhere(hs: dict[str, Any], skills: set[str], source: str) -> bool:
    """a skill of `skills` was at hint Lv 5 from other sources before the card's event (`source`, "<card> event")
    fired: it ends the run at Lv 5, some hint of it came from another source, and none is credited to that event, alone
    or shared (a credit from it shows the skill was below Lv 5 when it fired, since a hint at Lv 5 is not credited)"""
    return any(
        s in hs
        and hs[s]["lv"] >= 5
        and any(x != source for x in hs[s]["src"])
        and not any(names_source(x, source) for x in hs[s]["src"])
        for s in skills
    )


def choice_events() -> dict[int, list[tuple[str, list[list[set[str]]]]]]:
    """card -> (event label, [choice -> [outcome -> skills]]) for every SR/SSR, pal and group event (per-card variants
    of pal and group events as the card sees them)"""
    if not _choice_events:
        g = game()
        gt = gametora()
        out: collections.defaultdict[int, list[tuple[str, list[list[set[str]]]]]] = collections.defaultdict(list)
        for f in ("training_events_sr", "training_events_ssr"):
            for cid, es in gt.load(f):
                for e in es:
                    chs = [[skill_set(h, g.skill_names) for h, _ in gt.outcomes(ch[1])] for ch in gt.choices(e[1])]
                    out[cid].append((f"{g.cards.get(cid, {}).get('name', cid)} event {e[2]}", chs))
        for kindf in ("friend", "group"):
            for row in gt.load(f"training_events_{kindf}"):
                holders = [
                    cid
                    for cid, c in g.cards.items()
                    if c["pal"]
                    and ((kindf == "friend" and c["chara"] == row[0]) or (kindf == "group" and cid == row[0]))
                ]
                variants: dict[str, dict[Any, Any]] = collections.defaultdict(dict)
                for ev in gt.events(row, kindf):
                    variants[str(ev.event[2])][ev.card_id] = ev.choices
                for sid, by_card in variants.items():
                    for cid in holders:
                        raw = by_card.get(cid, by_card.get(None))
                        chs = [[skill_set(h, g.skill_names) for h, _ in gt.outcomes(ch[1])] for ch in gt.choices(raw)]
                        out[cid].append((f"{g.cards[cid]['name']} event {sid}", chs))
        _choice_events.append(dict(out))
    return _choice_events[0]


_choice_events: list[dict[int, list[tuple[str, list[list[set[str]]]]]]] = []


def readable_choice(chs: list[list[set[str]]], j: int, unique: set[str]) -> set[str] | None:
    """choice j's skills when every one of its outcomes gives one of them and all are in `unique`; else None"""
    skills = {s for o in chs[j] for s in o}
    if not chs[j] or not skills or not skills <= unique or not all(o for o in chs[j]):
        return None
    return skills


def steer_readings(R: list[Run] | None = None) -> list[SteerReading]:
    """every (run, event) where the run shows a priority list, exactly one choice besides the top holds a prioritised
    skill, and the records show which of the two IT took: every outcome of both gives a skill only that choice of this
    event gives among the card's events, that no other deck card's events can give, with background under BG_CLEAN;
    IT's choice is the one whose skill is credited to the card ("<card> event"), runs crediting both or neither left
    out, and runs whose every steered skill is already at hint Lv 5 from another source left out (the steered choice
    could not show). A reading in the maxed case is left out too: maxed_elsewhere() requires no credit from the card's
    event, so a reading that took the steered choice can never be maxed and the case's share would be 0 by
    construction. Plus Anshinzawa's finale from section 9's hint-level reading."""
    g = game()
    evs = choice_events()
    out: list[SteerReading] = []
    for r in R if R is not None else runs() or []:
        p = r["priority"]
        if p is None:
            continue
        deck = [c for c, _, _ in r["deck"]]
        if any(c not in g.cards for c in deck):
            continue
        hs = {h["name"]: h for h in r["hints"]}
        name = {c: bare(n) for c, _, n in r["deck"]}
        for cid in deck:
            elsewhere = set().union(*(g.pool.get(y, set()) for y in deck if y != cid))
            cnt = collections.Counter(s for _, chs in evs.get(cid, []) for ch in chs for o in ch for s in o)
            for label, chs in evs.get(cid, []):
                if len(chs) < 2:
                    continue
                steered = [j for j in range(1, len(chs)) if {s for o in chs[j] for s in o} & set(p)]
                if len(steered) != 1:
                    continue
                unique = {
                    s
                    for ch in chs
                    for o in ch
                    for s in o
                    if cnt[s] == 1 and s not in elsewhere and marker_background({s}, r) < BG_CLEAN
                }
                top, st = readable_choice(chs, 0, unique), readable_choice(chs, steered[0], unique)
                if top is None or st is None or top & st:
                    continue
                src = f"{name[cid]} event"
                if all(s in hs and hs[s]["lv"] >= 5 and any(x != src for x in hs[s]["src"]) for s in st):
                    continue
                took_st = any(s in hs and src in hs[s]["src"] for s in st)
                took_top = any(s in hs and src in hs[s]["src"] for s in top)
                if took_st == took_top:
                    continue
                case = steer_case(p, st, top, maxed_elsewhere(hs, st, src))
                if case and case != "maxed":
                    out.append(SteerReading(label, case, took_st))
    return out + finale_steer_readings(R if R is not None else runs() or [])


_steer: dict[str, tuple[int, int]] = {}
_nuisance_pool: list[list[Run]] = []  # nuisance_runs(): the runs the steering shares, end shares and backgrounds use


def nuisance_pool() -> list[Run]:
    """the runs the fitted nuisances (steer_shares, chosen_end_share, background_counts) are measured on: every run,
    or the set nuisance_runs() holds"""
    return _nuisance_pool[-1] if _nuisance_pool else runs() or []


@contextlib.contextmanager
def nuisance_runs(R: list[Run]) -> Iterator[None]:
    """measure the nuisances on R only while inside (held-out scoring fits them on the training runs); their caches are
    cleared on entry and exit"""
    _nuisance_pool.append(R)
    for cache in (_steer, _end_share, _bg):
        cache.clear()
    try:
        yield
    finally:
        _nuisance_pool.pop()
        for cache in (_steer, _end_share, _bg):
            cache.clear()


def steer_shares() -> dict[str, tuple[int, int]]:
    """per case, (runs taking the steered choice, runs), pooled over steer_readings on the nuisance pool"""
    if not _steer:
        xs = steer_readings(nuisance_pool())
        for case in STEER_CASES:
            _steer[case] = (sum(x.taken for x in xs if x.case == case), sum(1 for x in xs if x.case == case))
    return _steer


def steer_chance(case: str | None) -> float:
    """the chance IT takes the steered choice in this case (0 when nothing on it is prioritised)"""
    if case is None:
        return 0.0
    k, n = steer_shares()[case]
    return k / n if n else 0.0


def end_chance(cid: int, r: Run) -> float:
    """the chance IT takes the card's chain-ending choice in a run showing a priority list (section 9's three cases; the
    ending skill counts as maxed only when maxed_elsewhere() says it was at Lv 5 before the card's event fired)"""
    link_end = chosen_end(cid)
    if not link_end or r["priority"] is None:
        return 0.0
    _, ending, continuing = link_end
    name = next(bare(n) for c, _, n in r["deck"] if c == cid)
    hs = {h["name"]: h for h in r["hints"]}
    maxed = maxed_elsewhere(hs, ending, f"{name} event")
    return steer_chance(steer_case(r["priority"], ending, continuing, maxed))


_end_share: dict[int, float] = {}


def chosen_end_share(cid: int) -> float:
    """the mean end_chance over the nuisance pool's runs with the card that show a priority list"""
    if cid not in _end_share:
        xs = [
            end_chance(cid, r)
            for r in nuisance_pool()
            if r["priority"] is not None and any(c == cid for c, _, _ in r["deck"])
        ]
        _end_share[cid] = round(sum(xs) / len(xs), 6) if xs else 0.0
    return _end_share[cid]


BG_SHRINK: int = 20  # runs: a trainee's own background rate is pulled toward the scenario's with this weight


BG_CLEAN: float = (
    0.01  # background below which a seen later link proves its earlier links fired (allowance, contradicts)
)


_bg: list[tuple[dict[tuple[str, int], list[int]], dict[tuple[str, int, int], list[int]]]] = []


def background_counts() -> tuple[dict[tuple[str, int], list[int]], dict[tuple[str, int, int], list[int]]]:
    """per (marker skill, scenario) and (marker skill, scenario, trainee): [runs crediting the skill to plain "Events",
    runs whose deck holds no card whose events can give it]"""
    if not _bg:
        g = game()
        skills = {sk for m in g.markers.values() for mk in m.values() for sk in mk}
        by_scen: collections.defaultdict[tuple[str, int], list[int]] = collections.defaultdict(lambda: [0, 0])
        by_tr: collections.defaultdict[tuple[str, int, int], list[int]] = collections.defaultdict(lambda: [0, 0])
        for r in nuisance_pool():
            deck = [c for c, _, _ in r["deck"]]
            if any(c not in g.cards for c in deck):
                continue
            givable = set().union(*(g.pool.get(c, set()) for c in deck))
            events = {h["name"] for h in r["hints"] if "Events" in h["src"]}
            for sk in skills - givable:
                for key, table in (((sk, r["scenario"]), by_scen), ((sk, r["scenario"], r["chara"]), by_tr)):
                    table[key][1] += 1
                    table[key][0] += sk in events
        _bg.append((dict(by_scen), dict(by_tr)))
    return _bg[0]


def marker_background(mk: set[str], r: Run) -> float:
    """the chance a link's marker shows in this run although the link did not fire: some marker skill from a source
    the records credit to the card anyway (scenario and trainee events), measured where no deck card can give it, per
    scenario, a trainee's own rate pulled toward it (BG_SHRINK)"""
    by_scen, by_tr = background_counts()
    q = 1.0
    for sk in mk:
        k, n = by_scen.get((sk, r["scenario"]), [0, 0])
        b = k / n if n else 0.0
        kt, nt = by_tr.get((sk, r["scenario"], r["chara"]), [0, 0])
        q *= 1 - (kt + BG_SHRINK * b) / (nt + BG_SHRINK)
    return 1 - q


# ---------------------------------------------------------------- Anshinzawa's finale reading and event outcomes with hint levels (sections 9, 13)
# one outcome of a choice: skill name -> hint level, or '?' when the level is random
Outcome = dict[str, int | str]


# one card event: (title, story id, [each choice's rewards])
OutcomeEvent = tuple[str, int, list[Any]]


ANSHIN: int = 30080  # Sasami Anshinzawa [This Might Sting!]


FESTA: int = 30108  # Nakayama Festa [Trusting the Dice]


HER_A: str = "Sasami Anshinzawa event"


C1_LV: int = 3  # her finale's Nothing Ventured at this level or above: choice 1


def deck_ids(r: Run) -> list[int]:
    return [d[0] for d in r["deck"]]


def hints(r: Run) -> dict[str, Hint]:
    return {h["name"]: h for h in r["hints"]}


def credited(hs: dict[str, Hint], skill: str, source: str) -> bool:
    return skill in hs and source in hs[skill]["src"]


def outcome_skills(rewards: Any, gt: Any, sk: dict[int, str]) -> list[Outcome]:
    """one choice -> [ {skill name: level or '?'} ] per outcome"""
    out: list[Outcome] = []
    for hs, _ in gt.outcomes(rewards):
        o: Outcome = {}
        for kind, x, lv in hs:
            if kind == "hint" and x in sk:
                level = str(lv).lstrip("+") if lv else "1"
                o[sk[x]] = int(level) if level.isdigit() else "?"
            elif kind == "random":
                for i in x:
                    if i in sk:
                        o[sk[i]] = "?"
        out.append(o)
    return out


def card_outcome_events(gt: Any, title: dict[int, str], cchar: dict[int, int]) -> dict[int, list[OutcomeEvent]]:
    """card -> its SR/SSR events and, for a pal, its character's friend events (a per-card variant where it has one)"""
    evs: dict[int, list[OutcomeEvent]] = collections.defaultdict(list)
    for key in ("training_events_sr", "training_events_ssr"):
        for cid, es in gt.load(key):
            for e in es:
                evs[cid].append((title.get(int(e[2]), str(e[2])), int(e[2]), [ch[1] for ch in gt.choices(e[1])]))
    by_chara: dict[int, list[int]] = collections.defaultdict(list)
    for cid, ch in cchar.items():
        by_chara[ch].append(cid)
    for row in gt.load("training_events_friend"):
        base: dict[int, Any] = {}
        var: dict[int, dict[Any, Any]] = collections.defaultdict(dict)  # by id(event)
        for e, only, chs in gt.events(row, "friend"):
            if only is None:
                base[id(e)] = e
            else:
                var[id(e)][only] = chs
        for k, e in base.items():
            for cid in by_chara[row[0]]:
                chs = var[k].get(cid, e[1])
                evs[cid].append((title.get(int(e[2]), str(e[2])), int(e[2]), [ch[1] for ch in gt.choices(chs)]))
    return evs


# ---------------------------------------------------------------- Sasami Anshinzawa's finale
@dataclass
class AnshinRun:
    """one run with Anshinzawa's card (anshin_runs())"""

    festa: bool  # Nakayama Festa's card in the deck too (joint credits)
    shown: bool  # the run shows a priority list
    nv_pri: bool  # Nothing Ventured prioritised
    rb_pri: bool  # Risky Business prioritised
    NV: tuple[int, list[str]] | None  # Nothing Ventured's (hint level, sources), if hinted
    RB: tuple[int, list[str]] | None  # Risky Business's
    c: str = ""  # the finale's reading (classify())


def anshin_runs(R: list[Run]) -> list[AnshinRun]:
    out: list[AnshinRun] = []
    for r in R:
        d = deck_ids(r)
        if ANSHIN not in d:
            continue
        hs = hints(r)
        pri = r.get("priority")
        g = {s: (hs[s]["lv"], hs[s]["src"]) for s in ("Nothing Ventured", "Risky Business") if s in hs}
        out.append(
            AnshinRun(
                festa=FESTA in d,
                shown=pri is not None,
                nv_pri="Nothing Ventured" in (pri or []),
                rb_pri="Risky Business" in (pri or []),
                NV=g.get("Nothing Ventured"),
                RB=g.get("Risky Business"),
            )
        )
    for o in out:
        o.c = classify(o)
    return out


def classify(o: AnshinRun) -> str:
    """the finale's reading: 'c1_win' / 'c2_win' (Nothing Ventured at or above C1_LV / below); without Nothing Ventured,
    by her Risky Business when it is credited to her alone (each of her +1s is one event: the date-start event's
    bottom choice, which IT takes when Risky Business is prioritised, and the finale's choice-2 loss): 'c2_lose' at Lv 2
    (both her +1s) or at Lv 1 on a shown list without Risky Business, 'c1_blank' at Lv 1 with Risky Business prioritised
    (the date-start's +1 alone: the finale's choice 1 gave nothing); 'c2_lose?' (doubtful) at Lv 1 with no list shown,
    or when the credit is shared ("A or B event") or another source adds to her level, which then cannot be read;
    'none_rb5' (nothing, Risky Business at Lv 5 from other sources), 'none'"""
    her: Callable[[tuple[int, list[str]] | None], Any] = lambda x: x and any("Anshinzawa" in s for s in x[1])
    nv, rb = o.NV, o.RB
    if nv:
        return "c1_win" if nv[0] >= C1_LV else "c2_win"
    if her(rb):
        if rb[1] != [HER_A]:
            return "c2_lose?"
        if rb[0] == 2:
            return "c2_lose"
        if not o.shown:  # whether the date-start's +1 came is unknown
            return "c2_lose?"
        date = 1 if o.rb_pri else 0  # the date-start event's +1
        if rb[0] == date + 1:
            return "c2_lose"
        return "c1_blank" if rb[0] == date else "c2_lose?"
    if rb and rb[0] >= 5:
        return "none_rb5"
    return "none"


# ---------------------------------------------------------------- the choice rule's three cases (computed in section 8)
def finale_steer_readings(R: list[Run]) -> list[SteerReading]:
    """Anshinzawa's finale on the three cases: both its choices can give Nothing Ventured, so the generic scan
    (steer_readings) cannot read it; its hint levels can (classify()). Maxed: Risky Business at Lv 5 at the end and
    none of it credited to her (her date-start event, which comes first, would have been credited had it been below
    Lv 5 then), so the finale fired with it at Lv 5; a run crediting her Risky Business was below Lv 5 at the date-start
    and is read as one-sided. A choice-2 win shows by Nothing Ventured whatever Risky Business's level, so the reading
    does not depend on the outcome."""
    out: list[SteerReading] = []
    for o in anshin_runs(R):
        if o.festa or not o.shown or not o.rb_pri or o.c not in ("c1_win", "c1_blank", "c2_win", "c2_lose"):
            continue
        maxed = bool(o.RB) and o.RB[0] >= 5 and not any("Anshinzawa" in x for x in o.RB[1])
        case = "both" if o.nv_pri else "maxed" if maxed else "one-sided"
        out.append(SteerReading("Anshinzawa's finale (hint levels)", case, o.c not in ("c1_win", "c1_blank")))
    return out


# ---------------------------------------------------------------- the career model against real careers: jobs, the solver, calibration rows (sections 10, 11, 13)
OUTD: str = os.path.join(OUT, "calib")


def write_atomic(path: str, text: str) -> None:
    """write a file under OUTD so a reader never sees it half-written: sections 10, 11 and 13 build and read the same
    calibration files from parallel processes (run_all.py --jobs), so each write goes to a private temporary file that
    then replaces the target"""
    tmp = f"{path}.{os.getpid()}.tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        f.write(text)
    os.replace(tmp, path)


SCENARIO_KEY: dict[int, str] = {1: "ura", 2: "unity", 3: "gc", 4: "trackblazer"}  # the solver's scenario names
# (character, skill group, 1 white / 2 gold): Yamanin Zephyr's Slipstream, Tokai Teio's Head-On (as the planner page
# sends them)
EXTRA: list[list[int]] = [[1078, 20165, 1], [1003, 20190, 1]]
PINK: set[int] = {11, 12, 21, 22, 23, 24, 31, 32, 33, 34}  # pink spark factor ids // 100 (track, distance, style)
# the most targets one --wide job carries. The exact scoring of a job's deck grows exponentially with its targets: at
# 24 some jobs ran for over half an hour (a 2 s limit dropped them, a different number on every run); at 12 the
# slowest of those take a tenth of a second. q does not depend on the split; deck odds do (targets share a deck's
# choices and hints), so the split is part of what the wide sample measures
WIDE_JOB_TARGETS: int = 12
SOLVER_BATCH: int = 100  # jobs per solver call (default targets)
# master.mdb support effect types
EFFECT_HINT_LEVELS, EFFECT_HINT_FREQUENCY, EFFECT_SPECIALTY = 17, 18, 19
MILESTONES: list[int] = [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50]  # the levels of support_card_effect_table's columns
# the generic spark chance of a learned white / gold at six carriers (docs/MODEL.md, spark generation: 20% / 40%,
# x1.1 per carrier); a row rescales it by 1.1 ** (carriers - 6)
SPARK_WHITE_6, SPARK_GOLD_6 = 0.2 * 1.1**6, 0.4 * 1.1**6


class CalRow(TypedDict):
    """one (run, target) row of the analysis (rows_of); s10_random_events and s11_conditional read these keys"""

    run: int  # run id
    scen: int  # scenario (SCEN)
    g: int  # the target's skill group
    scen_gold: bool  # the scenario guarantees the target's gold (Unity Cup's two)
    outfit: int  # her outfit gives the skill: 0 no, 1 its white, 2 its gold
    zero: bool  # nothing can give the target: not sent to the solver, every prediction 0
    dg: float  # predicted P(a deck card hints the gold)
    dw: float  # predicted P(a deck card hints the white)
    q: float  # predicted P(at least one inspiration of the target in year 2 or 3)
    nc: int  # carriers: legacies holding the target's spark
    p: float  # the solver's P(the target lands: acquired and sparked)
    acq: float  # predicted P(acquired, gold or white); 1 for a scenario gold
    gold: float  # predicted P(the gold acquired); 1 for a scenario gold
    plegs: list[float]  # per legacy carrying the spark: its inspiration chance per roll
    o_any: bool  # observed: hinted from any source, or owned
    o_gold: bool  # observed: a gold hint
    o_deck: bool  # observed: a hint credited to a deck card
    o_deck_gold: bool  # observed: a gold hint credited to a deck card
    o_insp: bool  # observed: an inspiration of the target fired in year 2 or 3
    o_insp_years: int  # observed: years (of 2 and 3) with an inspiration of the target
    o_insp_hits: int  # observed: inspiration hits of the target over both years
    o_insp_src: bool  # observed: a hint credited to Inspiration
    o_events: bool  # observed: a hint credited to Events
    o_other: bool  # observed: a hint credited to a source neither Inspiration, Events nor a deck card
    prio: bool | None  # the target is on the run's priority list; None when the run shows no list
    prio_outranked: bool  # on the list, but outranked on a rival choice of every event offering it (Model.outranked)
    pool_lb: int | None  # the lowest limit break among deck cards whose training pool holds it; None: no card
    cards_pool: list[int]  # deck cards whose training pool holds the target
    cards_ev: list[int]  # deck cards whose events can hint the target
    p_pred: float  # predicted landing: p, or the gold's generic spark chance for a scenario gold
    l_obs: float  # observed-equivalent landing: spark chance of the hinted or outfit-owned skill (gold, white or none)


class RunRows(TypedDict):
    """one run's rows (rows_of); s10_random_events and s11_conditional read these keys"""

    id: int
    scen: int
    p_all: float  # the solver's P(every target lands); 0 when a target is unreachable
    rows: list[CalRow]
    wide: bool  # from a --wide job (targets beyond the default eight)


class ManEntry(NamedTuple):
    """a manifest entry (build_jobs): a run and its targets nothing can give (written to manifest.json as a list)"""

    run_id: int
    unreachable: list[int]


SolverOut = dict[str, dict[str, Any]]  # a solver output per job file id


# ---------------------------------------------------------------- card variants per limit break
def variant(cid: int, lb: int | None) -> int:
    """the card id of a card at limit break lb (lb 4 or none: the card itself)"""
    return cid if lb is None or lb >= 4 else (lb + 1) * 1_000_000 + cid


class HintEffects(NamedTuple):
    """a card's training-hint effects at one level: web_cards.json's hf, hinc and spec"""

    frequency: int  # hint frequency (effect 18)
    levels: int  # hint levels per draw (1 + effect 17)
    specialty: int  # specialty priority (effect 19)


def card_variants() -> list[int]:
    """write cards_lb.json and events_lb.json (every card also at limit breaks 0-3) under OUTD; returns the cards whose
    limit-break-4 effects read from master.mdb differ from web_cards.json"""
    db = mdb()

    def query(sql: str, *args: Any) -> list[Any]:
        return db.execute(sql, args).fetchall()

    level_cap: dict[int, list[int]] = {
        r: lv for r, *lv in query("select rarity, limit_0, limit_1, limit_2, limit_3, limit_4 from support_card_limit")
    }

    def effect(effect_table: int, typ: int, level: int) -> float:
        """the effect's value at level, linear between milestone levels (-1: no value at that milestone)"""
        row = query(
            "select init,limit_lv5,limit_lv10,limit_lv15,limit_lv20,limit_lv25,limit_lv30,limit_lv35,limit_lv40,"
            "limit_lv45,limit_lv50 from support_card_effect_table where id=? and type=?",
            effect_table,
            typ,
        )
        if not row:
            return 0.0
        pts = [(lv, v) for lv, v in zip(MILESTONES, row[0]) if v != -1]
        if not pts or level < pts[0][0]:
            return 0.0
        for (l0, v0), (l1, v1) in pairwise(pts):
            if l0 <= level <= l1:
                return v0 + (v1 - v0) * (level - l0) / (l1 - l0)
        return float(pts[-1][1])

    def unique(sid: int, typ: int, level: int) -> int:
        return sum(
            v
            for lv, t0, v0, t1, v1 in query(
                "select lv, type_0, value_0, type_1, value_1 from support_card_unique_effect where id=?", sid
            )
            for t, v in ((t0, v0), (t1, v1))
            if t == typ and level >= lv
        )

    def at_lb(sid: int, lb: int) -> HintEffects:
        effect_table, rarity = query("select effect_table_id, rarity from support_card_data where id=?", sid)[0]
        lv = level_cap[rarity][lb]
        return HintEffects(
            frequency=max(0, math.floor(effect(effect_table, EFFECT_HINT_FREQUENCY, lv)))
            + unique(sid, EFFECT_HINT_FREQUENCY, lv),
            levels=1
            + max(0, math.floor(effect(effect_table, EFFECT_HINT_LEVELS, lv)))
            + unique(sid, EFFECT_HINT_LEVELS, lv),
            specialty=max(0, math.floor(effect(effect_table, EFFECT_SPECIALTY, lv)))
            + unique(sid, EFFECT_SPECIALTY, lv),
        )

    web_cards = gjson("web_cards.json")
    card_events = gjson("card_events.json")
    out: list[dict[str, Any]] = list(web_cards["cards"])
    bad: list[int] = []
    for card in web_cards["cards"]:
        if at_lb(card["id"], 4) != (card["hf"], card["hinc"], card.get("spec", 0)):
            bad.append(card["id"])
        for lb in range(4):
            fx = at_lb(card["id"], lb)
            out.append(dict(card, id=variant(card["id"], lb), hf=fx.frequency, hinc=fx.levels, spec=fx.specialty))
    restrict = {sc: ids + [variant(i, lb) for i in ids for lb in range(4)] for sc, ids in web_cards["restrict"].items()}
    events = dict(card_events["cards"])
    for k, v in card_events["cards"].items():
        for lb in range(4):
            events[str(variant(int(k), lb))] = v
    os.makedirs(OUTD, exist_ok=True)
    write_atomic(
        os.path.join(OUTD, "cards_lb.json"), json.dumps({"cards": out, "restrict": restrict}, ensure_ascii=False)
    )
    write_atomic(
        os.path.join(OUTD, "events_lb.json"),
        json.dumps({"about": card_events["about"], "cards": events}, ensure_ascii=False),
    )
    return bad


# ---------------------------------------------------------------- jobs
class RaceWins(NamedTuple):
    """the race indices each ancestor is laid out to have won (Model.wins)"""

    parent1: list[int]
    grandparent1a: list[int]
    grandparent1b: list[int]
    parent2: list[int]
    grandparent2a: list[int]
    grandparent2b: list[int]


class Model:
    """the game data the jobs and the analysis read"""

    cards: dict[int, dict[str, Any]]  # web_cards.json by card id
    web_skills: dict[str, Any]  # web_skills.json
    skill_groups: set[int]  # every skill group web_skills.json lists (a job can target)
    factor_group: dict[int, int]  # white spark factor id // 100 -> skill group
    race_count: int  # races in data.json
    card_events: dict[str, dict[str, Any]]  # card_events.json's cards, by card id as a string
    skill_level: dict[str, tuple[int, int]]  # skill name -> (group, 1 white / 2 gold)
    name_group: dict[str, int]  # skill name -> group
    _choices: dict[int, list[list[set[int]]]] | None  # event_choices(), read once

    def __init__(self) -> None:
        self._choices = None
        self.cards = cards()
        self.web_skills = gjson("web_skills.json")
        self.skill_groups = {g for g, w, gd in self.web_skills["skills"]}
        self.factor_group = factor_groups()
        self.race_count = len(gjson("data.json")["races"])
        self.card_events = gjson("card_events.json")["cards"]
        self.skill_level = skill_names()
        self.name_group = {n: g for n, (g, _) in self.skill_level.items()}

    def sparks(self, factors: Iterable[Sequence[int]]) -> list[list[int]]:
        """an ancestor's white sparks: [skill group, stars], the most stars per group"""
        out: dict[int, int] = {}
        for f, _ in factors:
            if 2000000 <= f < 3000000 and f // 100 in self.factor_group:
                g = self.factor_group[f // 100]
                out[g] = max(out.get(g, 0), f % 100)
        return sorted([g, s] for g, s in out.items())

    @staticmethod
    def pinks(factors: Iterable[Sequence[int]]) -> list[list[int]]:
        """an ancestor's pink sparks: [factor id // 100, stars]"""
        out: dict[int, int] = {}
        for f, _ in factors:
            if f < 10000 and f // 100 in PINK:
                out[f // 100] = max(out.get(f // 100, 0), f % 100)
        return sorted([g, s] for g, s in out.items())

    def ev_groups(self, c: int) -> set[int]:
        """skill groups a card's events can hint (every reward the solver reads)"""
        e = self.card_events.get(str(c), {})
        names: set[str] = set()
        for key in ("chain", "end"):
            for link in e.get(key, []):
                for n in link[1]:
                    names.add(n[0] if isinstance(n, list) else n)
        for alt in e.get("alts", []):
            for choice in alt[1]:
                for outcome in choice:
                    names.update(outcome)
        names.update(e.get("always", []))
        names.update(x[0] for x in e.get("chance", []))
        return {self.name_group[n] for n in names if n in self.name_group}

    def event_choices(self, c: int) -> list[list[set[int]]]:
        """a card's events (GameTora, as section 9 reads them): per event, per choice, the skill groups any of its
        outcomes hints"""
        if self._choices is None:
            gt, db = gametora(), mdb()
            sk = dict(db.execute("select [index], text from text_data where category = 47"))
            title = dict(db.execute("select [index], text from text_data where category = 181"))
            cchar = dict(db.execute("select id, chara_id from support_card_data"))
            self._choices = {
                cid: [
                    [
                        {self.name_group[n] for o in outcome_skills(ch, gt, sk) for n in o if n in self.name_group}
                        for ch in chs
                    ]
                    for _, _, chs in es
                ]
                for cid, es in card_outcome_events(gt, title, cchar).items()
            }
        return self._choices.get(c, [])

    def outranked(self, g: int, rank: dict[int, int], cards: Iterable[int]) -> bool:
        """the target g is prioritised (rank: group -> its place on the run's priority list, 0 first), yet every event
        of the cards that puts it on a choice has a higher-ranked prioritised skill on a choice without it: between two
        prioritised skills IT takes the higher-ranked one, so no event gives g (an event with g on its only choice, or on
        every choice, is never outranked). False when no event offers g"""
        offers = [chs for c in cards for chs in self.event_choices(c) if any(g in ch for ch in chs)]
        return bool(offers) and all(
            any(h in rank and rank[h] < rank[g] for ch in chs if g not in ch for h in ch) for chs in offers
        )

    def deck_groups(self, cards: Iterable[int]) -> set[int]:
        """skill groups a deck can hint: its cards' training pools and events"""
        gs: set[int] = set()
        for c in cards:
            gs.update(self.cards[c]["whites"])
            gs |= self.ev_groups(c)
        return gs

    def spark_groups(self, r: Run) -> set[int]:
        """skill groups any of the run's six ancestors carries a white spark of"""
        return {g for p in r["parents"] for a in [p] + p["gps"] for g, s in self.sparks(a["factors"]) if s > 0}

    def wins(self, affinity: dict[str, list[int]]) -> RaceWins | None:
        """race wins laid out so every pair the model reads shares the record's shared-G1 count: parent 1 wins races
        0..m1-1 (m1 the most she shares with anyone), her parents its first a1 / b1; parent 2 shares the first s12
        of them and wins new races after m1 for what her parents share beyond s12. None when that needs more
        races than the race list has."""

        def shared(pair: str) -> int:
            return affinity[pair][1]

        s12, a1, b1, a2, b2 = (
            shared("parent 1 × parent 2"),
            shared("parent 1 × grandparent 1a"),
            shared("parent 1 × grandparent 1b"),
            shared("parent 2 × grandparent 2a"),
            shared("parent 2 × grandparent 2b"),
        )
        m1 = max(s12, a1, b1)
        p1 = list(range(m1))
        p2 = list(range(s12)) + list(range(m1, m1 + max(0, max(a2, b2) - s12)))
        if m1 + max(0, max(a2, b2) - s12) > self.race_count:
            return None
        return RaceWins(p1, p1[:a1], p1[:b1], p2, p2[:a2], p2[:b2])


def drop_reason(r: Run, M: Model, outfits: list[int]) -> str | None:
    """why a run cannot be rebuilt as a job (outfits: her character's outfits in web_skills.json), or None"""
    chara, deck, parents = r["chara"], r["deck"], r["parents"]
    if len(deck) != 6:
        return "deck not 6 cards"
    if any(c not in M.cards for c, _, _ in deck):
        return "card not in web_cards"
    if any(M.cards[c]["rarity"] == 1 for c, _, _ in deck):
        return "R card in deck"
    if r["outfit"] not in outfits:
        return "outfit not in web_skills"
    if len(parents) != 2:
        return "parents not recorded"
    if parents[0]["chara"] == parents[1]["chara"] or chara in (parents[0]["chara"], parents[1]["chara"]):
        return "same parent twice"
    return None


def legacy_tree(r: Run, M: Model, wins: RaceWins) -> list[dict[str, Any]]:
    """the job's two legacies, each with its two recorded parents, sparks and laid-out race wins"""

    def ancestor(a: dict[str, Any], won: list[int]) -> dict[str, Any]:
        return {"chara": a["chara"], "sparks": M.sparks(a["factors"]), "pinks": M.pinks(a["factors"]), "wins": won}

    parents = r["parents"]
    legs: list[dict[str, Any]] = []
    for parent, won, grandparent_wins in (
        (parents[0], wins.parent1, (wins.grandparent1a, wins.grandparent1b)),
        (parents[1], wins.parent2, (wins.grandparent2a, wins.grandparent2b)),
    ):
        leg = ancestor(parent, won)
        leg["borrowed"] = False
        leg["parents"] = [ancestor(g, gw) for g, gw in zip(parent["gps"], grandparent_wins)]
        legs.append(leg)
    return legs


def job_targets(r: Run, M: Model, wide: bool, scen: str) -> tuple[list[dict[str, Any]], list[int]]:
    """the job's targets and the candidate targets nothing can give. A target nothing can give has P = 0 in every
    mechanism; the career search drops careers with P(all) = 0, so such targets are not sent and are scored 0 by the
    analysis (listed in the manifest)"""
    chara, outfit = r["chara"], r["outfit"]
    deck_ids = [c for c, _, _ in r["deck"]]
    if wide:
        candidates = sorted(
            g for g in set(DEFAULT) | M.deck_groups(deck_ids) | M.spark_groups(r) if g in M.skill_groups
        )
    else:
        candidates = list(DEFAULT)
    outfit_gives = {g for o in M.web_skills["cards"][str(chara)] if o[0] == outfit for g, _ in o[2]} | {
        g for u, g, _ in EXTRA if u == chara
    }
    reachable = M.deck_groups(deck_ids) | M.spark_groups(r) | outfit_gives | (UNITY_GOLDS if scen == "unity" else set())
    unreachable = [g for g in candidates if g not in reachable]
    targets = [
        {"group": g, **({"guaranteed": 2} if scen == "unity" and g in UNITY_GOLDS else {})}
        for g in candidates
        if g in reachable
    ]
    return targets, unreachable


_events_off: dict[frozenset[int], str] = {}


def events_without_ends(off: frozenset[int]) -> str:
    """events_lb.json without the chain-ending choices of the cards in `off` (limit-break variant ids): written once per
    process, so it always follows the current events_lb.json"""
    if off not in _events_off:
        ev = json.load(open(os.path.join(OUTD, "events_lb.json")))
        for cid in off:
            ev["cards"][str(cid)] = {k: v for k, v in ev["cards"][str(cid)].items() if k != "end"}
        path = os.path.join(OUTD, f"events_lb_noend_{'_'.join(map(str, sorted(off)))}.json")
        write_atomic(path, json.dumps(ev, ensure_ascii=False))
        _events_off[off] = path
    return _events_off[off]


def end_pins(r: Run) -> tuple[list[list[int]], frozenset[int], int]:
    """the run's chosen chain ends pinned by its priority list (section 9's three cases, end_chance): a
    certain stop [card, link, 1, 1] where IT takes the ending choice, the cards whose end entries are removed where it
    does not; a chance strictly between (the maxed and both-prioritised cases) leaves the card unpinned, counted in the
    third value. Nothing for a run showing no list (the solver then picks, as every target is assumed prioritised)."""
    if r["priority"] is None:
        return [], frozenset(), 0
    stops: list[list[int]] = []
    off: set[int] = set()
    open_ends = 0
    for c, lb, _ in r["deck"]:
        link_end = chosen_end(c)
        if not link_end:
            continue
        q = end_chance(c, r)
        if q >= 1:
            stops.append([variant(c, lb), link_end[0], 1, 1])
        elif q <= 0:
            off.add(variant(c, lb))
        else:
            open_ends += 1
    return stops, frozenset(off), open_ends


def build_jobs(
    R: list[Run], M: Model, wide: bool, jobdir: str
) -> tuple[list[ManEntry], list[str], collections.Counter[str]]:
    """one career-planner job file per usable run (--wide: per run and chunk of targets) and manifest.json; returns the
    manifest entries, the job file ids and the dropped runs by reason"""
    os.makedirs(jobdir, exist_ok=True)
    drop: collections.Counter[str] = collections.Counter()
    man: list[ManEntry] = []
    files: list[str] = []
    unpinned = 0  # runs keeping a chain end the solver chooses (end_pins' maxed or both-prioritised case)
    for r in R:
        chara, outfit = r["chara"], r["outfit"]
        outfits = [o[0] for o in M.web_skills["cards"].get(str(chara), [])]
        reason = drop_reason(r, M, outfits)
        if reason:
            drop[reason] += 1
            continue
        wins = M.wins(r["affinity"])
        if wins is None:
            drop["shared G1 counts exceed the race list"] += 1
            continue
        legs = legacy_tree(r, M, wins)
        scen = SCENARIO_KEY[r["scenario"]]
        targets, unreachable = job_targets(r, M, wide, scen)
        ids = [variant(c, lb) for c, lb, _ in r["deck"]]
        job = {
            "mode": "career",
            "iters": 30,
            "seed": 3,
            "skills_file": os.path.join(GD, "web_skills.json"),
            "cards_file": os.path.join(OUTD, "cards_lb.json"),
            "events_file": os.path.join(OUTD, "events_lb.json"),
            "scenario": scen,
            "targets": targets,
            "extra": EXTRA,
            "exclude_outfits": [o for o in outfits if o != outfit],
            "owned": [chara],
            "trainees": [chara],
            "owned_cards": ids,
            "lock_cards": ids,
            "borrow": False,
            "legacies": legs,
            "fix": [0, 1],
            "top": 1,
            "per_trainee": True,
        }
        stops, off, open_ends = end_pins(r)
        unpinned += open_ends > 0
        if stops:
            job["stop_risk"] = stops
        if off:
            job["events_file"] = events_without_ends(off)
        if not wide:
            write_atomic(os.path.join(jobdir, f"{r['id']}.json"), json.dumps(job))
            files.append(str(r["id"]))
        else:
            rest = [t for t in targets if t["group"] not in DEFAULT]
            for k in range(0, len(rest), WIDE_JOB_TARGETS):
                chunk = f"{r['id']}_{k // WIDE_JOB_TARGETS}"
                write_atomic(
                    os.path.join(jobdir, f"{chunk}.json"), json.dumps(dict(job, targets=rest[k : k + WIDE_JOB_TARGETS]))
                )
                files.append(chunk)
        man.append(ManEntry(r["id"], unreachable))
    write_atomic(
        os.path.join(jobdir, "manifest.json"),
        json.dumps({"runs": man, "files": files, "dropped": drop, "unpinned_ends": unpinned}),
    )
    return man, files, drop


def fingerprint(jobdir: str, files: Iterable[str]) -> str:
    """a hash of the jobs and of every file the solver reads for them: the binary, data.json, web_skills.json (each
    job's skills_file), cards_lb.json (its cards_file), events_lb.json and its end-free variants"""
    h = hashlib.sha256()
    for f in sorted(files):
        h.update(open(os.path.join(jobdir, f + ".json"), "rb").read())
    for p in (
        SOLVER,
        os.path.join(GD, "data.json"),
        os.path.join(GD, "web_skills.json"),
        os.path.join(OUTD, "cards_lb.json"),
        os.path.join(OUTD, "events_lb.json"),
        *sorted(_events_off.values()),
    ):
        h.update(open(p, "rb").read())
    return h.hexdigest()


def solver_lines(stdout: str) -> list[str]:
    return [line for line in stdout.splitlines() if line.startswith("{")]


def solve_in_batches(jobdir: str, files: list[str]) -> SolverOut:
    """SOLVER_BATCH jobs per solver call"""
    res: SolverOut = {}
    for i in range(0, len(files), SOLVER_BATCH):
        part = files[i : i + SOLVER_BATCH]
        p = subprocess.run(
            [SOLVER, "data.json"] + [os.path.join(jobdir, x + ".json") for x in part],
            cwd=GD,
            capture_output=True,
            text=True,
        )
        lines = solver_lines(p.stdout)
        if p.returncode != 0 or len(lines) != len(part):
            sys.exit(f"solver failed on chunk starting {part[0]}: rc {p.returncode}\n{p.stderr[-2000:]}")
        for x, line in zip(part, lines):
            res[x] = json.loads(line)
    return res


def run_solver(
    jobdir: str,
    files: list[str],
    resf: str,
    share: float | None = None,
    man: list[ManEntry] | None = None,
) -> SolverOut:
    """one process at a time, every job solved to the end (no time limit: the same jobs give the same outputs on any
    machine); returns {file id: output}; cached in resf using fingerprint(). share: the seeded fraction of the
    manifest's runs whose jobs run (sampled from sorted run ids)"""
    if share is not None:
        keep = set(random.Random(1).sample(sorted(m.run_id for m in man), round(share * len(man))))
        files = [x for x in files if int(x.split("_")[0]) in keep]
    fp = fingerprint(jobdir, files)
    if os.path.exists(resf):
        with open(resf) as f:
            head = json.loads(f.readline())
            if head.get("fingerprint") == fp:
                cached = {x["id"]: x["out"] for x in map(json.loads, f)}
                # results written under the old 2 s limit mark the jobs over it {"timeout": true}: solved again
                if not any(o.get("timeout") for o in cached.values()):
                    return cached
    if not os.path.exists(SOLVER):
        sys.exit("native solver missing: cd pkg/loopopt && cargo build --release")
    t0 = time.time()
    print(f"solver: {len(files)} jobs, one process", file=sys.stderr, flush=True)
    res = solve_in_batches(jobdir, files)
    print(f"solver: done in {time.time() - t0:.0f} s", file=sys.stderr, flush=True)
    write_atomic(
        resf,
        json.dumps({"fingerprint": fp}) + "\n" + "".join(json.dumps({"id": x, "out": res[x]}) + "\n" for x in files),
    )
    return res


# ---------------------------------------------------------------- calibration rows
class HintSeen(NamedTuple):
    """one hint record of a target"""

    tier: int  # 1 white, 2 gold
    sources: list[str]  # the record's credited sources ("<card> event", a card's character, "Inspiration", ...)
    level: int  # the hint level shown


@dataclass
class Inspirations:
    """a skill group's inspirations over years 2 and 3"""

    years: int = 0  # years with at least one
    hits: int = 0  # every hit, summed over both years


def hints_by_group(
    r: Run, M: Model, other_names: dict[str, tuple[int, int]], bad: collections.Counter[str]
) -> collections.defaultdict[int, list[HintSeen]]:
    """the run's hint records by skill group; a name in neither web_skills.json nor other_names counts in bad"""
    hints: collections.defaultdict[int, list[HintSeen]] = collections.defaultdict(list)
    for h in r["hints"]:
        if h["name"] in M.skill_level:
            g, tier = M.skill_level[h["name"]]
        elif h["name"] in other_names:
            g, tier = other_names[h["name"]]
        else:
            bad["hint name unmapped"] += 1
            continue
        hints[g].append(HintSeen(tier, h["src"], h["lv"]))
    return hints


def inspirations_of(r: Run, M: Model) -> collections.defaultdict[int, Inspirations]:
    """the run's white-spark inspirations in years 2 and 3, by skill group"""
    insp: collections.defaultdict[int, Inspirations] = collections.defaultdict(Inspirations)
    for y in ("2", "3"):
        seen: set[int] = set()
        for f, hit in r["years"].get(y, []):
            if 2000000 <= f < 3000000 and f // 100 in M.factor_group:
                g = M.factor_group[f // 100]
                insp[g].hits += hit
                if g not in seen:
                    insp[g].years += 1
                    seen.add(g)
    return insp


def credited_cards(sources: Iterable[str], card_of: dict[str, int]) -> set[tuple[int, bool]]:
    """the deck cards a hint's sources credit (card_of: a deck card's character -> card): (card, by its event)"""
    cs: set[tuple[int, bool]] = set()
    for t in sources:
        for n in t[:-6].split(" or ") if t.endswith(" event") else [t]:
            if n in card_of:
                cs.add((card_of[n], t.endswith(" event")))
    return cs


def run_rows(
    r: Run, career: dict[str, Any], unreachable: list[int], wide: bool, M: Model, bad: collections.Counter[str]
) -> RunRows:
    """one run's rows: a row per target of the solver's best career (career), then (default targets) one per target
    nothing can give"""
    card_of = {bare(nm): c for c, lb, nm in r["deck"]}
    # skill names outside web_skills.json, from the run's own hint groups: name -> (group, 1 white / 2 gold)
    other_names = {v[1]: (gid, 2 if (v[2] or 1) >= 2 else 1) for gid, vs in r.get("hint_groups", []) for v in vs}
    hints = hints_by_group(r, M, other_names, bad)
    owned = {s // 10 for s in r.get("owned", [])}
    insp = inspirations_of(r, M)
    rank: dict[int, int] = {}  # the priority list: group -> its first place (0 first)
    for i, n in enumerate(r["priority"] or []):
        group = M.skill_level.get(n, other_names.get(n, (None,)))[0]
        if group is not None:
            rank.setdefault(group, i)
    solved = {t["group"]: t for t in career["targets"]}
    order = [t["group"] for t in career["targets"]]
    slots = career["slots"]
    run: RunRows = {
        "id": r["id"],
        "scen": r["scenario"],
        "p_all": career["p_all"] * (0 if unreachable else 1),
        "rows": [],
        "wide": wide,
    }
    for g in order + ([] if wide else unreachable):
        t = solved.get(g)
        if t is None:
            # the solver's acquisition odds: gold, white, from the deck, from her outfit (0 none, 1 white, 2 gold)
            buy: dict[str, Any] = {"gold": 0.0, "white": 0.0, "deck_gold": 0.0, "deck_white": 0.0, "outfit": 0}
            q, nc, p, scen_g, plegs = 0.0, 0, 0.0, False, []
        else:
            buy = t["buy"]
            q, nc, p, scen_g = t["q"], t["carriers"], t["p"], bool(buy.get("scenario"))
            k = order.index(g)
            # per legacy carrying the spark: 3% per star, raised by its affinity, per roll
            plegs = [min(1.0, 0.03 * s["stars"][k] * (1 + s["aff"] / 100)) for s in slots if s["stars"][k] > 0]
        hs = hints.get(g, [])
        deck_sources = set().union(*[credited_cards(h.sources, card_of) for h in hs]) if hs else set()
        prio = None if r["priority"] is None else g in rank
        row = CalRow(
            run=r["id"],
            scen=r["scenario"],
            g=g,
            scen_gold=scen_g,
            outfit=buy.get("outfit", 0),
            zero=t is None,
            dg=buy.get("deck_gold", 0.0),
            dw=buy.get("deck_white", 0.0),
            q=q,
            nc=nc,
            p=p,
            acq=1.0 if scen_g else buy["gold"] + buy["white"],
            gold=1.0 if scen_g else buy["gold"],
            plegs=plegs,
            o_any=bool(hs) or g in owned,
            o_gold=any(h.tier == 2 for h in hs),
            o_deck=bool(deck_sources),
            o_deck_gold=any(h.tier == 2 and credited_cards(h.sources, card_of) for h in hs),
            o_insp=insp[g].years > 0,
            o_insp_years=insp[g].years,
            o_insp_hits=insp[g].hits,
            o_insp_src=any("Inspiration" in h.sources for h in hs),
            o_events=any("Events" in h.sources for h in hs),
            o_other=any(
                any(x not in ("Inspiration", "Events") and not credited_cards([x], card_of) for x in h.sources)
                for h in hs
            ),
            prio=prio,
            prio_outranked=bool(prio) and M.outranked(g, rank, [c for c, _, _ in r["deck"] if g in M.ev_groups(c)]),
            pool_lb=min(
                [lb for c, lb, _ in r["deck"] if g in M.cards[c]["whites"]] or [None],
                key=lambda v: 9 if v is None else v,
            ),
            cards_pool=[c for c, _, _ in r["deck"] if g in M.cards[c]["whites"]],
            cards_ev=[c for c, _, _ in r["deck"] if g in M.ev_groups(c)],
        )
        carriers = 1.1 ** (nc - 6)
        row["p_pred"] = SPARK_GOLD_6 * carriers if scen_g else p
        row["l_obs"] = (
            SPARK_GOLD_6 * carriers
            if (row["o_gold"] or row["outfit"] == 2)
            else SPARK_WHITE_6 * carriers
            if (row["o_any"] or row["outfit"] == 1)
            else 0.0
        )
        run["rows"].append(row)
    return run


def rows_of(
    R: list[Run], M: Model, res: SolverOut, man: list[ManEntry]
) -> tuple[list[CalRow], list[RunRows], collections.Counter[str]]:
    """the (run, target) rows of every solved job, the rows by run, and the problems met (by kind)"""
    by_id = {r["id"]: r for r in R}
    unreachable = {m.run_id: m.unreachable for m in man}
    rows: list[CalRow] = []
    runrows: list[RunRows] = []
    bad: collections.Counter[str] = collections.Counter()
    for fid, o in res.items():
        wide = "_" in fid
        r = by_id[int(fid.split("_")[0])]
        if "results" not in o or not o["results"]:
            bad["no result"] += 1
            continue
        run = run_rows(r, o["results"][0], unreachable[r["id"]], wide, M, bad)
        rows.extend(run["rows"])
        runrows.append(run)
    return rows, runrows, bad


# ---------------------------------------------------------------- random-event calibration rows (sections 10, 11)
def marker_credit(
    srcs: Sequence[str], nm: str, m: str, deck: dict[str, list[int]], pool: dict[int, set[str]]
) -> bool | None:
    """section 8's rule for the marker m of the character named nm, from the sources its hint was credited to: None
    (unknown) when another deck card (deck: bare name -> card ids) has events that can give m (pool), seen or not, since
    the record cannot tell the two sources apart and dropping only the seen readings would bias the rate down; else True
    when a credit names her ("<nm> event", or "A or <nm> event"), False when none does"""
    if any(y != nm and any(m in pool.get(c, ()) for c in ids) for y, ids in deck.items()):
        return None
    return any(nm in [y.strip() for y in s[:-6].split(" or ")] for s in srcs if s.endswith(" event"))


def bernoulli_ll(y: np.ndarray, p: np.ndarray) -> float:
    """the Bernoulli log-likelihood of outcomes y at chances p (clipped to [1e-9, 1 - 1e-9])"""
    import numpy as np

    p = np.clip(p, 1e-9, 1 - 1e-9)
    return float(np.sum(y * np.log(p) + (1 - y) * np.log(1 - p)))


# ---------------------------------------------------------------- effect on real careers: section 13's rows, random events credited or left out (sections 10, 11)
def calib() -> tuple[list[CalRow], list[RunRows]]:
    """section 13's calibration rows and per-run rows (default targets, each run on its own deck), exactly as
    s13_calibration.claims builds them; solver-result reuse follows fingerprint"""
    if "calib" not in _cache:
        R = runs()
        M = Model()
        card_variants()
        jobdir = os.path.join(OUTD, "jobs8")
        man, files, _ = build_jobs(R, M, False, jobdir)
        res = run_solver(jobdir, files, os.path.join(OUTD, "res8.jsonl"))
        rows, runrows, _ = rows_of(R, M, res, man)
        _cache["calib"] = (rows, runrows)
    return _cache["calib"]


def event_groups() -> dict[str, Any]:
    """skill groups each kind of event can hint (GameTora, through gametora.py): a trainee's story and date events
    (training_events_char), each outfit's own events (training_events_char_card), a character's shared events
    (training_events_shared), a trainee's current conditional events with their conditions, each scenario's events
    (training_events_scenario; its ids are the game's scenario ids)"""
    if "evgroups" in _cache:
        return _cache["evgroups"]
    gt = gametora()
    grp = dict(mdb().execute("select id, group_id from skill_data"))

    def of(chs: Any) -> set[int | None]:
        return {
            grp.get(x)
            for ch in gt.choices(chs)
            for kind, x0, _ in gt.hints(ch[1])
            for x in ([x0] if kind == "hint" else x0)
        }

    def of_list(lst: Any) -> set[int | None]:
        return set().union(
            set(), *[of(e[1]) for e in (lst if isinstance(lst, list) else []) if isinstance(e, list) and len(e) >= 2]
        )

    ordinary: dict[int, set[int | None]] = {}
    cond: collections.defaultdict[int, list[tuple[set[int | None], Any]]] = collections.defaultdict(list)
    for row in gt.load("training_events_char"):
        ordinary[row[0]] = of_list(row[2]) | of_list(row[3])
        for tag, ve in gt.conditional_events(row):
            if tag.startswith("current"):
                cond[row[0]].append((of(ve[1]), gt.extra(ve, 0)))
    outfit = {row[0]: of_list(row[1]) for row in gt.load("training_events_char_card")}
    shared = {ch: of_list(es) for ch, es in gt.load("training_events_shared")}
    scen = {
        row[0]: set().union(set(), *[of_list(part) for part in row[1:]]) for row in gt.load("training_events_scenario")
    }
    _cache["evgroups"] = out = {
        "ordinary": ordinary,
        "cond": cond,
        "outfit": outfit,
        "shared": shared,
        "scen": scen,
    }
    return out


def hint_srcs(r: Run) -> collections.defaultdict[int, list[Hint]]:
    """skill group -> the run's hint records of it (white or gold), names mapped as section 13 maps them"""
    names = skill_names()
    pn = {v[1]: gid for gid, vs in r.get("hint_groups", []) for v in vs}
    out: collections.defaultdict[int, list[Hint]] = collections.defaultdict(list)
    for h in r["hints"]:
        g = names[h["name"]][0] if h["name"] in names else pn.get(h["name"])
        if g is not None:
            out[g].append(h)
    return out


def free(row: CalRow) -> bool:
    """the row can move: not on the trainee's outfit, not a scenario gold"""
    return not row["outfit"] and not row["scen_gold"]


def credit(row: CalRow, r: float) -> tuple[float, float]:
    """Hybrid (landing, acquisition) with the reconstructed miss multiplied by (1 - r).

    The row's p is an exact landing marginal, but dg is the report's approximate gold chance. Even at r = 0,
    recovered acquisition is p / (GEN_W * 1.1**nc) - dg, not exact acquisition. Both comparison sides use this
    reconstruction. A row on the trainee's outfit or a scenario gold does not move.
    """
    if not free(row):
        return row["p"], row["acq"]
    s = 1.1 ** row["nc"]
    dg = row["dg"]
    if dg >= 1:
        return row["p"], 1.0
    m = 1 - (row["p"] - dg * GEN_G * s) / ((1 - dg) * GEN_W * s)
    mm = m * (1 - r)
    return dg * GEN_G * s + (1 - dg) * GEN_W * s * (1 - mm), dg + (1 - dg) * (1 - mm)


class CalibSide(NamedTuple):
    """calib_test() one way (left out or credited)"""

    pred: float  # mean predicted acquisition
    ratio: float  # obs / pred
    rlo: float  # its run-clustered 95% interval
    rhi: float
    ll: float  # the Bernoulli log-likelihood over the rows with a positive prediction left out


@dataclass
class CalibTest:
    """calib_test(): acquisition observed against predicted on a set of calibration rows"""

    n: int  # rows
    k: int  # acquired
    obs: float
    lo: float  # the observed rate's run-clustered 95% interval
    hi: float
    left_out: CalibSide
    credited: CalibSide
    n_pos: int  # rows with a positive prediction left out
    n_zero: int  # the rest

    def ratio_text(self, side: CalibSide) -> str:
        """obs / pred [95%] as the findings print it"""
        return f"{side.ratio:.3f} [{side.rlo:.2f}, {side.rhi:.2f}]"

    def ll_gain(self) -> float:
        return self.credited.ll - self.left_out.ll


def calib_test(rs: Sequence[CalRow], rate: Callable[[CalRow], float]) -> CalibTest:
    """acquisition observed (hinted from any source, or owned) against predicted, without and with crediting each row at
    rate(row): rows, observed (its 95% interval run-clustered), predicted, obs / pred and its 95% interval each way
    (cluster_ci: a run can give several rows, and the predictions differ by row); the Bernoulli log-likelihood each way
    over the rows whose prediction without crediting is positive (a row predicted at 0 and acquired has no finite
    likelihood; those rows are counted apart)"""
    n = len(rs)
    k = sum(1 for x in rs if x["o_any"])
    ob = [1 if x["o_any"] else 0 for x in rs]
    run = [x["run"] for x in rs]
    _, lo, hi, _ = cluster_ci(ob, [1] * n, run)
    p0 = [credit(x, 0.0)[1] for x in rs]
    p1 = [credit(x, rate(x))[1] for x in rs]
    pos = [i for i, q in enumerate(p0) if q > 1e-12]
    sides: list[CalibSide] = []
    for pr in (p0, p1):
        ep = div(sum(pr), n)
        ratio, rlo, rhi, _ = cluster_ci(ob, pr, run)
        sides.append(
            CalibSide(
                ep,
                ratio,
                rlo,
                rhi,
                sum(math.log(pr[i]) if rs[i]["o_any"] else math.log(1 - pr[i]) for i in pos if 0 < pr[i] < 1)
                + sum(0.0 if (pr[i] >= 1) == bool(rs[i]["o_any"]) else -math.inf for i in pos if not 0 < pr[i] < 1),
            )
        )
    return CalibTest(
        n=n,
        k=k,
        obs=div(k, n),
        lo=lo,
        hi=hi,
        left_out=sides[0],
        credited=sides[1],
        n_pos=len(pos),
        n_zero=n - len(pos),
    )


def calib_values(sec: str, pre: str, t: CalibTest) -> list[Value | Finding]:
    """a calibration's obs / pred left out and the log-likelihood gain from crediting, as Values of section `sec`"""
    V: Callable[[str, Any], Value] = lambda nm, v: Value(sec, nm, v)
    return [
        V(f"{pre}: obs / pred, left out", t.left_out.ratio),
        V(f"{pre}: ratio 95% low, left out", t.left_out.rlo),
        V(f"{pre}: ratio 95% high, left out", t.left_out.rhi),
        V(f"{pre}: LL credited minus left out, positive rows", t.ll_gain()),
    ]


# ---------------------------------------------------------------- generation constants (sections 10, 12)
GEN_W, GEN_G = 0.2, 0.4  # section 12's W_k, G_k at n_k = 0
