"""Section 4: the race win table, tested against the races Independent Training runs actually ran.

The model (pkg/loopopt/src/lib.rs, data.json `pen`): an attempt at one of the 34 single-race G1s wins with
p = clamp(bp - PEN_s, 0, 100) / 100, bp = max(-20, 110 - cost(distance grade) - cost(surface grade)) from her base aptitudes
(data.json umas[].turns[t][].bp; cost S/A 0, B 10, C 20, D 30, E 40, F 60, G 90), PEN = data.json `pen` (0, 0, 5, 20,
30, 50; the community table) and s the race's scored position (scored_j): its position j in the run of consecutive
races (any race, goals included; a turn without a race resets; capped at 6, as the solver's streak state), one less
for a goal race from the 2nd in a row (lib.rs scored_position). The community table scores every race at j; the goal
shift is the adopted model, tested here against it and against goal races not counting at all (exclusion).

Reproduces from the run records (research/data/runs.jsonl, not included; Independent Training runs from uma-it):
- every attempt at one of the 34 by a trainee in data.json: the record's race name (after ALIAS), matched to the
  option data.json offers her at that turn index, t = (year - 1) * 24 + (month - 1) * 2 + (half - 1); attempts
  that do not match are counted and listed by detail();
- j from the run's own race turns (every race the career ran: debut, goals, G2/G3/OP);
- won = the record's win flag; predicted p from the table; obs / pred with its run-clustered 95% interval
  (common.cluster_ci: one run gives several attempts), overall, by bp, by scenario, and goal or chosen by j;
- log-likelihoods on the same attempts: the table as stated (minus infinity when a certain prediction misses), a
  constant rate, the table with and without streak penalties each recalibrated as lo + (hi - lo) p (2 parameters),
  and free fits: a rate per (bp, j, goal or chosen) cell and per bp (j capped at 6); these are in-sample, attempts
  treated as independent; the cells against the recalibrated pink table are also compared held out (5-fold by run,
  held_out_cells);
- aptitude raised by pink sparks: the planner's rule (docs/MODEL.md: at the start the six legacies' pink stars per
  aptitude raise the base grade by 1/2/3/4 at 1/4/7/10 stars, at most A; each April firing recorded in the run, years
  2 and 3 from turns 30 and 54 on, +1 grade, at most S) gives a pink-adjusted bp. The overall and detail calibrations are reported on both
  the base bp (what the solver uses) and the pink-adjusted bp, as are the attempts the rule moves and those after an
  April firing of the race's distance or surface; the fits use the pink-adjusted bp;
- the goal shift: the adopted table against the community table (every race at j) and against exclusion (goal races
  neither shifted nor counted: every race takes PEN at the count of chosen races before it in the run, plus one if it is
  chosen), each recalibrated, 5-fold held out by run (held_out_tables); goal and chosen races by streak position, and
  goal races from the 4th in a row under the community table; detail() also compares the forms in-sample on each half
  of the runs (record order).
Findings: one per claim of section 4 (base under-predicts; pink-adjusted, goal races calibrated at every position and
chosen races from the 3rd on, chosen races at the 1st and 2nd slightly over-predicted (a known miss), every scenario
calibrated outside those; pink beats base; the streak penalty; the goal shift beats the community table and exclusion
held out; the cells not better held out; the previous race's result, as obs / pred after a loss over after a win,
run-clustered, at each streak position). The other cells, the certain-outcome counts and the fits behind them are
printed by detail().

Conditional on selection: IT chose to run every non-goal attempt, presumably when she was likely to win; goal races are
run regardless (scored separately). Her stats, mood and skills are not modelled, by the table or here. A win is the record's flag (first place).
"""

import collections, math
from collections.abc import Callable, Hashable
from typing import Any, NamedTuple, TypedDict
import numpy as np
from scipy.optimize import minimize
from scipy.stats import chi2
import common
from common import Finding, Run, Value

SEC: str = "4"
ALIAS: dict[str, str] = {
    "M.C. Nambu Hai": "Mile Championship Nambu Hai"
}  # the records' short name for the data.json race
MON: dict[str, int] = {
    m: i for i, m in enumerate(["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"])
}
COST: dict[int, int] = {8: 0, 7: 0, 6: 10, 5: 20, 4: 30, 3: 40, 2: 60, 1: 90}  # grade codes 1..8 = G..S
GRADE: dict[int, str] = {
    1: "G",
    2: "F",
    3: "E",
    4: "D",
    5: "C",
    6: "B",
    7: "A",
    8: "A",
}  # aptitude codes; S costs what A does
COST_L: dict[str, int] = {"A": 0, "B": 10, "C": 20, "D": 30, "E": 40, "F": 60, "G": 90}  # COST by grade letter
PINK_DIST: tuple[int, ...] = (31, 32, 33, 34)  # pink factor groups (factor id // 100) by distance category
PINK_SURF: dict[int, int] = {1: 11, 2: 12}  # ... by ground (1 turf, 2 dirt)
APRIL: dict[str, int] = {"2": 30, "3": 54}  # inspiration years -> first turn index they count for
BPB: tuple[tuple[int, str], ...] = ((110, "110"), (100, "100"), (90, "90"), (80, "80"), (-99, "<=70"))  # bp buckets
PB: tuple[tuple[float, str], ...] = (
    (1.0, "100%"),
    (0.9, "90-99%"),
    (0.8, "80-89%"),
    (0.6, "60-79%"),
    (0.0, "<60%"),
)  # by p
IN_SAMPLE: str = "in-sample, attempts treated as independent"


class Attempt(TypedDict):
    """one attempt at a race at the 34 (built by attempts())"""

    run: int
    bp: int  # the table's bp on base aptitudes
    bpa: int  # on spark-raised (pink) aptitudes
    apt: tuple[int, ...]  # (distance, surface) aptitude codes, spark-raised
    j: int  # the race's position in a run of consecutive races
    won: int
    p: float  # win chance: the adopted table (goal shift) on base aptitudes
    p0: float  # ... without the streak penalty
    pa: float  # the adopted table on spark-raised (pink) aptitudes
    pa0: float  # pink table without the streak penalty
    pc: float  # pink, the community table: every race at its own position j
    px: float  # pink, exclusion: goal races neither shifted nor counted
    scen: int  # scenario id
    goal: bool  # a career goal (run regardless), else chosen by IT
    outfit: int
    fired: bool  # after an April pink firing of the race's distance or surface
    later: bool  # in the later half of the runs (record order)
    last: int | None  # the previous race's result, when it was the turn before
    t: int  # turn index
    pe: float  # pink table, a mid-run pink counted from the year's first turn


class AttemptSet(NamedTuple):
    """attempts(): every attempt at the 34, and what was left out"""

    attempts: list[Attempt]
    skipped: collections.Counter[str]  # attempts or runs left out, by reason
    unmatched: collections.Counter[str]  # G1 names not in data.json after ALIAS


class Calib(NamedTuple):
    """calib(): observed against predicted on a set of attempts"""

    n: int
    observed_rate: float
    mean_predicted: float
    ratio: float  # obs / pred
    low: float  # its run-clustered 95% interval
    high: float


def tindex(year: int, turn: str) -> int:
    """the turn index of a record's (year, 'Mon half')"""
    m, h = turn.split()
    return (year - 1) * 24 + MON[m] * 2 + int(h) - 1


def win_p(bp: int, j: int, pen: list[int]) -> float:
    """the table's chance at bp for a race scored at position j (6 and later take the 6th's penalty)"""
    return min(100, max(0, bp - pen[min(j, 6) - 1])) / 100


def scored_j(j: int, goal: bool) -> int:
    """the position whose penalty a race at position j takes in the adopted model (lib.rs scored_position): a goal race
    from the 2nd in a row the previous one, j capped at 6 first as the solver's streak state is"""
    return min(j, 6) - 1 if goal and j >= 2 else j


def bp_of(distance_grade: int, surface_grade: int) -> int:
    """the table's bp from two aptitude codes"""
    return max(-20, 110 - COST[distance_grade] - COST[surface_grade])


def lineage_pinks(r: Run) -> collections.Counter[int]:
    """pink group -> stars summed over the six legacies"""
    out: collections.Counter[int] = collections.Counter()
    for p in r.get("parents") or []:
        for a in [p, *list(p.get("gps") or [])]:
            for f, _ in (a or {}).get("factors") or []:
                if f < 10000 and f // 100 in PINK_DIST + tuple(PINK_SURF.values()):
                    out[f // 100] += f % 100
    return out


def pink_firings(r: Run, pinks: collections.Counter[int]) -> dict[int, list[int]]:
    """pink group -> the first turn index of each of its firings at a year's inspiration (one entry per firing)"""
    fired: dict[int, list[int]] = collections.defaultdict(list)
    for y, t0 in APRIL.items():
        for f, n in (r.get("years") or {}).get(y, []):
            if f < 10000 and f // 100 in pinks:
                fired[f // 100] += [t0] * n
    return fired


def raised_grade(grade: int, pink_stars: int, firings: list[int], t: int, lead: int = 0) -> int:
    """an aptitude code at turn t under the planner's pink rule: the legacies' stars raise the base grade by 1/2/3/4 at
    1/4/7/10 stars (at most A), each firing from its turn on (or `lead` turns earlier) by 1 more (at most S)"""
    grade = max(grade, min(grade + sum(pink_stars >= s for s in (1, 4, 7, 10)), 7))
    return min(8, grade + sum(t0 - lead <= t for t0 in firings))


def attempts(R: list[Run]) -> AttemptSet:
    """(attempts, skipped Counter, unmatched G1 names Counter): one dict per attempt at the 34"""
    d = common.gjson("data.json")
    U = {u["id"]: u for u in d["umas"]}
    idx = {n: i for i, n in enumerate(d["race_names"])}
    pen = d["pen"]
    found = AttemptSet([], collections.Counter(), collections.Counter())
    out, skip, unk = found
    for i, r in enumerate(R):
        races = sorted(((tindex(x["year"], x["turn"]), x) for x in r["races"]), key=lambda t: t[0])
        turns = [t for t, _ in races]
        if len(set(turns)) != len(turns):
            skip["run with two races on one turn"] += 1
            continue
        u = U.get(r["chara"])
        pk = lineage_pinks(r)
        fired = pink_firings(r, pk)
        j = 0
        chosen_before = 0  # chosen races before this one in the current run of consecutive races (exclusion's count)
        prev: int | None = None
        prev_won: int | None = None
        for t, x in races:
            consecutive = prev == t - 1
            j = j + 1 if consecutive else 1
            chosen_before = chosen_before if consecutive else 0
            jx = chosen_before + (0 if x["required"] else 1)  # exclusion's position: goal races not counted
            chosen_before += 0 if x["required"] else 1
            prev = t
            last, prev_won = (prev_won if consecutive else None), int(x["won"])
            name = ALIAS.get(x["race"], x["race"])
            if name not in idx:
                if x["grade"] == "G1":
                    unk[x["race"]] += 1
                continue
            if u is None:
                skip["trainee not in data.json"] += 1
                continue
            ri = idx[name]
            op = next((o for o in u["turns"][t] if o["race"] == ri), None) if t < len(u["turns"]) else None
            if op is None:
                skip["race not offered to her at that turn"] += 1
                continue
            cat, gr = d["race_cat"][ri], d["race_ground"][ri]
            gd, gs = PINK_DIST[cat], PINK_SURF[gr]
            adj: list[int] = []
            early: list[int] = []
            for k, g in ((cat, gd), (4 if gr == 1 else 5, gs)):
                adj.append(raised_grade(u["apt"][k], pk[g], fired[g], t))
                early.append(
                    raised_grade(u["apt"][k], pk[g], fired[g], t, lead=6)
                )  # the year's proc from its first turn
            bpa = bp_of(adj[0], adj[1])
            out.append(
                Attempt(
                    run=r["id"],
                    bp=op["bp"],
                    bpa=bpa,
                    apt=tuple(adj),
                    j=j,
                    won=int(x["won"]),
                    p=win_p(op["bp"], scored_j(j, x["required"]), pen),
                    p0=win_p(op["bp"], 1, pen),
                    pa=win_p(bpa, scored_j(j, x["required"]), pen),
                    pa0=win_p(bpa, 1, pen),
                    pc=win_p(bpa, j, pen),
                    px=win_p(bpa, max(jx, 1), pen),
                    scen=r["scenario"],
                    goal=bool(x["required"]),
                    outfit=r["outfit"],
                    fired=any(t0 <= t for g in (gd, gs) for t0 in fired[g]),
                    later=2 * i >= len(R),
                    last=last,
                    t=t,
                    pe=win_p(bp_of(early[0], early[1]), scored_j(j, x["required"]), pen),
                )
            )
    return found


# ---------------------------------------------------------------- statistics
def calib(A: list[Attempt], key: str) -> Calib:
    """(n, observed rate, mean predicted, obs / pred, low, high): the run-clustered 95% interval of obs / pred
    (common.cluster_ci; one run gives several attempts)"""
    n = len(A)
    if not n:
        return Calib(0, *(float("nan"),) * 5)
    k = sum(a["won"] for a in A)
    e = sum(a[key] for a in A) / n
    r, lo, hi, _ = common.cluster_ci([a["won"] for a in A], [a[key] for a in A], [a["run"] for a in A])
    return Calib(n, k / n, e, r, lo, hi)


def ll(ps: np.ndarray, y: np.ndarray) -> float:
    """log-likelihood; -inf when a certain prediction misses"""
    with np.errstate(divide="ignore"):
        return float(np.sum(np.where(y == 1, np.log(ps), np.log(1 - ps))))


CLIP: float = 1e-12  # predictions are kept in [CLIP, 1 - CLIP]; a certain miss scores log(CLIP)


def recal(q: np.ndarray, y: np.ndarray) -> tuple[float, float, float]:
    """the table recalibrated as lo + (hi - lo) q: (LL, lo, hi), fitted by maximum likelihood with lo and hi each in
    [CLIP, 1 - CLIP]. The log-likelihood is concave in (lo, hi) (each prediction is linear in them), so a bounded
    quasi-Newton fit (L-BFGS-B, exact gradient) reaches the maximum, at the bound when the data put it there (a ceiling
    of 1 when no attempt at q = 1 loses); two starts, one fixed and one from the edge cells' smoothed rates (the
    attempts at the lowest and highest q, (wins + 1/2) / (attempts + 1)), keep the better. A logistic scale on lo and
    hi cannot reach 1 and stalls near it."""
    q, y = np.asarray(q, float), np.asarray(y, float)

    def nll(th: np.ndarray) -> tuple[float, np.ndarray]:
        p = np.clip(th[0] + (th[1] - th[0]) * q, CLIP, 1 - CLIP)
        d = y / p - (1 - y) / (1 - p)  # d LL / d p per attempt
        return -float(np.sum(y * np.log(p) + (1 - y) * np.log(1 - p))), -np.array([np.sum(d * (1 - q)), np.sum(d * q)])

    edge = [((y[m].sum() + 0.5) / (m.sum() + 1)) for m in (q == q.min(), q == q.max())]
    best = None
    for start in ([0.05, 0.95], edge):
        res = minimize(
            nll,
            np.clip(start, CLIP, 1 - CLIP),
            jac=True,
            method="L-BFGS-B",
            bounds=[(CLIP, 1 - CLIP)] * 2,
            options={"ftol": 1e-15, "gtol": 1e-10, "maxiter": 20000},
        )
        if best is None or res.fun < best.fun:
            best = res
    return -float(best.fun), float(best.x[0]), float(best.x[1])


def saturated(A: list[Attempt], key: Callable[[Attempt], Hashable]) -> tuple[float, int]:
    """(LL, cells): every cell of `key` gets its own observed rate"""
    g: dict[Hashable, list[int]] = collections.defaultdict(lambda: [0, 0])  # cell -> [wins, attempts]
    for a in A:
        c = g[key(a)]
        c[0] += a["won"]
        c[1] += 1
    xl: Callable[[float], float] = lambda x: x * math.log(x) if x else 0.0
    return sum(xl(k / n) * n + xl(1 - k / n) * n for k, n in g.values()), len(g)


def thr(k: int) -> float:
    """the likelihood-ratio threshold for k extra parameters: chi2(k) 95% / 2"""
    return chi2.ppf(0.95, k) / 2


class ModelFits(NamedTuple):
    """fits(): each model's in-sample fit on the same attempts"""

    log_likelihoods: dict[str, tuple[float, int]]  # model -> (LL, parameters)
    recalibration: dict[str, tuple[float, float]]  # prediction -> its recalibration's (lo, hi)


def fits(A: list[Attempt]) -> ModelFits:
    """model -> (LL, parameters) on the same attempts, and each prediction's recalibration (lo, hi)"""
    y = np.array([a["won"] for a in A])
    k = int(y.sum())
    n = len(y)
    out = ModelFits({"constant": (k * math.log(k / n) + (n - k) * math.log(1 - k / n), 1)}, {})
    for key, lab in (
        ("p", "table"),
        ("p0", "no penalty"),
        ("pa", "pink table"),
        ("pa0", "pink no penalty"),
        ("pc", "pink community table"),
        ("px", "pink exclusion"),
    ):
        q = np.array([a[key] for a in A])
        out.log_likelihoods[lab] = (ll(q, y), 0)
        L, lo, hi = recal(q, y)
        out.log_likelihoods[lab + ", recalibrated"] = (L, 2)
        out.recalibration[lab] = (lo, hi)
    out.log_likelihoods["pink cells bp x j x goal"] = saturated(A, cell_of)
    out.log_likelihoods["pink cells bp"] = saturated(A, lambda a: a["bpa"])
    return out


def cell_of(a: Attempt) -> tuple[int, int, bool]:
    """an attempt's free-rate cell: (pink bp, j capped at 6, goal), the covariates the adopted table reads"""
    return a["bpa"], min(a["j"], 6), a["goal"]


def per_run_held_out(A: list[Attempt], predict: Callable[[np.ndarray], np.ndarray]) -> np.ndarray:
    """each run's held-out log-likelihood, 5-fold by common.fold(run), runs in id order: on each fold's training
    attempts, `predict(train mask)` gives a chance per attempt (fitted on the training attempts only), recalibrated there
    as lo + (hi - lo) q; the fold's own runs are scored by it"""
    y = np.array([a["won"] for a in A])
    run = np.array([a["run"] for a in A])
    folds = np.array([common.fold(r) for r in run])
    ll_attempt = np.zeros(len(A))  # each attempt's held-out log-likelihood
    for k in range(common.K_FOLDS):
        train, test = folds != k, folds == k
        q = predict(train)
        _, lo, hi = recal(q[train], y[train])
        p_ = np.clip(lo + (hi - lo) * q[test], CLIP, 1 - CLIP)  # as recal() fits
        ll_attempt[test] = np.where(y[test] == 1, np.log(p_), np.log(1 - p_))
    runs_ = sorted(set(run.tolist()))
    ix = {r: i for i, r in enumerate(runs_)}
    per_run = np.zeros(len(runs_))
    np.add.at(per_run, [ix[r] for r in run], ll_attempt)
    return per_run


def table_prediction(A: list[Attempt], key: str) -> Callable[[np.ndarray], np.ndarray]:
    """a fixed table's chances (attempt field `key`), the same whatever the training folds"""
    q = np.array([a[key] for a in A])
    return lambda _train: q


def held_out_cells(A: list[Attempt]) -> tuple[common.HeldOutComparison, int]:
    """rule 3 for the cells: on four folds, a rate per (pink bp, j, goal) cell, (wins + 1/2) / (attempts + 1) (Jeffreys;
    a raw 0 or 1 makes the held-out score that of the 1e-12 clip; a cell absent there takes the adopted table's chance),
    and the adopted table, each recalibrated on those folds; each run of the fifth scored by both (per_run_held_out).
    Returns common.compare_held_out(cells, table) and the run count"""

    def cells(train: np.ndarray) -> np.ndarray:
        g: dict[tuple[int, int, bool], list[int]] = collections.defaultdict(lambda: [0, 0])  # cell -> [wins, attempts]
        for a in (a for a, t in zip(A, train) if t):
            c = g[cell_of(a)]
            c[0] += a["won"]
            c[1] += 1
        return np.array([(g[cell_of(a)][0] + 0.5) / (g[cell_of(a)][1] + 1) if cell_of(a) in g else a["pa"] for a in A])

    per_table = per_run_held_out(A, table_prediction(A, "pa"))
    return common.compare_held_out(per_run_held_out(A, cells), per_table), len(per_table)


def held_out_tables(A: list[Attempt], key: str, other: str) -> common.HeldOutComparison:
    """two tables (attempt fields), each recalibrated on four folds and scored on the fifth: compare_held_out(key, other)"""
    return common.compare_held_out(
        per_run_held_out(A, table_prediction(A, key)), per_run_held_out(A, table_prediction(A, other))
    )


# ---------------------------------------------------------------- claims
def bucket(v: float, bs: tuple[tuple[float, str], ...]) -> str:
    """the label of the first bucket whose lower bound v reaches"""
    return next(lab for lo, lab in bs if v >= lo)


def jlab(j: int) -> str:
    return str(j) if j < 6 else "6+"


def by(A: list[Attempt], grp: Callable[[Attempt], Any], key: str) -> dict[Any, Calib]:
    """cell -> calib(cell's attempts, key)"""
    g: dict[Any, list[Attempt]] = collections.defaultdict(list)
    for a in A:
        g[grp(a)].append(a)
    return {c: calib(S, key) for c, S in sorted(g.items())}


def cv(c: Calib) -> str:
    """a calibration as the findings print it"""
    return f"obs/pred {c.ratio:.3f} [{c.low:.3f}, {c.high:.3f}], n {c.n:,}"


def grid(A: list[Attempt], pen: list[int], minn: int = 30, mincol: int = 200) -> str:
    """the community table's layout from the chosen races in the runs (a goal race takes the row above's chance): columns
    the (distance, surface) grade pair as a sorted pair of letters (spark-raised grades, S as A), rows the race's position
    in a run of consecutive races (1 to 6+); each cell the observed win rate, with the table's chance in parentheses;
    pairs with `mincol` or more attempts in all, cells under `minn` attempts show a dot. Returns the markdown table"""
    cell: dict[tuple[str, int], list[int]] = collections.defaultdict(lambda: [0, 0])  # (pair, j) -> [wins, attempts]
    for a in (a for a in A if not a["goal"]):
        pair = "".join(sorted((GRADE[a["apt"][0]], GRADE[a["apt"][1]])))
        c = cell[(pair, min(a["j"], 6))]
        c[0] += a["won"]
        c[1] += 1
    pairs: dict[str, int] = {}
    for (pair, _), (_, n) in cell.items():
        pairs[pair] = pairs.get(pair, 0) + n
    cols = sorted((p for p, n in pairs.items() if n >= mincol), key=lambda p: (-(110 - COST_L[p[0]] - COST_L[p[1]]), p))
    rows = ["1 race", "2 races", "3 races", "4 races", "5 races", "6 or more"]
    head = "| Races in a row | " + " | ".join(cols) + " |\n|---|" + "---|" * len(cols) + "\n"
    body = ""
    for j in range(1, 7):
        line = f"| {rows[j - 1]} |"
        for p in cols:
            w, n = cell.get((p, j), (0, 0))
            pred = min(100, max(0, 110 - COST_L[p[0]] - COST_L[p[1]] - pen[j - 1]))
            line += f" {100 * w / n:.0f}% ({pred}%) |" if n >= minn else f" · ({pred}%) |"
        body += line + "\n"
    return head + body.rstrip("\n")


def ratio_values(lab: str, c: Calib) -> list[Value | Finding]:
    """a cell's obs / pred with its interval, as Values named `lab` ..."""
    return [
        Value(SEC, f"{lab} obs/pred", c.ratio),
        Value(SEC, f"{lab} obs/pred 95% low", c.low),
        Value(SEC, f"{lab} obs/pred 95% high", c.high),
    ]


def calib_values(lab: str, c: Calib) -> list[Value | Finding]:
    """a cell's attempts and obs / pred with its interval, as Values named `lab` ..."""
    return [Value(SEC, f"{lab} attempts", c.n), *ratio_values(lab, c)]


class Calibrations(NamedTuple):
    """calibration_claims(): the calibrations the findings test"""

    base: Calib  # every attempt, base aptitudes
    pink: Calib  # every attempt, pink-adjusted
    cells: dict[str, dict[Any, Calib]]  # pink-adjusted, by 'bp', 'goal j', 'chosen j' and 'scenario'
    from_fourth: dict[str, Calib]  # pink-adjusted, from the 4th race in a row: 'goal' and 'chosen'
    community_goal_from_fourth: Calib  # goal races from the 4th in a row under the community table


class LikelihoodComparison(NamedTuple):
    """likelihood_claims(): the fits compared (in-sample, attempts treated as independent) and held out"""

    pink_over_base: float  # recalibrated pink table - recalibrated base table, nats
    penalty: float  # recalibrated pink table - recalibrated pink no penalty, nats
    held_out: common.HeldOutComparison  # cells against the table, 5-fold by run
    held_out_runs: int
    shift_over_community: common.HeldOutComparison  # the adopted table against the community table, held out
    shift_over_exclusion: common.HeldOutComparison  # ... against exclusion


def claims() -> list[Value | Finding]:
    R = common.runs()
    if R is None:
        return []
    A = attempts(R).attempts
    cal_values, cal = calibration_claims(A)
    lik_values, lik = likelihood_claims(A)
    return (
        cal_values
        + lik_values
        + calibration_findings(cal, lik)
        + [Value(SEC, "race grid by aptitude and streak", grid(A, common.gjson("data.json")["pen"]))]
        + pink_timing_claims(A)
        + streak_result_claims(A)
    )


def calibration_claims(A: list[Attempt]) -> tuple[list[Value | Finding], Calibrations]:
    """the attempts, and obs / pred overall (base and pink-adjusted) and by cell"""
    base, pink = calib(A, "p"), calib(A, "pa")
    out: list[Value | Finding] = [
        Value(SEC, "race attempts at the 34", len(A)),
        Value(SEC, "race goal attempts", sum(a["goal"] for a in A)),
    ]
    for lab, c in (("base", base), ("pink", pink)):
        out += [
            Value(SEC, f"race obs/pred, {lab}", c.ratio),
            Value(SEC, f"race obs/pred, {lab}, 95% low", c.low),
            Value(SEC, f"race obs/pred, {lab}, 95% high", c.high),
        ]
    cells = {
        "bp": by(A, lambda a: bucket(a["bpa"], BPB), "pa"),
        "goal j": by([a for a in A if a["goal"]], lambda a: jlab(a["j"]), "pa"),
        "chosen j": by([a for a in A if not a["goal"]], lambda a: jlab(a["j"]), "pa"),
        "scenario": by(A, lambda a: common.SCEN[a["scen"]], "pa"),
        "scenario, goal or 3rd on": by(
            [a for a in A if a["goal"] or a["j"] >= 3], lambda a: common.SCEN[a["scen"]], "pa"
        ),  # every attempt but chosen races at the 1st and 2nd race in a row
    }
    out += calib_values("race pink bp <=70", cells["bp"]["<=70"])
    for lab in ("goal j", "chosen j"):
        for c, v in cells[lab].items():
            out += calib_values(f"race pink {lab} {c}", v)
    fourth = {lab: [a for a in A if a["goal"] == (lab == "goal") and a["j"] >= 4] for lab in ("goal", "chosen")}
    from_fourth = {lab: calib(S, "pa") for lab, S in fourth.items()}
    community = calib(fourth["goal"], "pc")
    out += ratio_values("race community table goal j>=4", community)
    seventh = calib([a for a in A if a["goal"] and a["j"] >= 7], "pa")
    out += calib_values("race pink goal j>=7", seventh)
    out.append(
        common.finding(
            SEC,
            "Known miss (the streak state stops at 6): goal races 7th or later in a row, scored as the 6th (PEN 30 where "
            "the shift gives 50), win less than predicted: their obs/pred 95% interval lies entirely below 1",
            seventh.high < 1,
            cv(seventh),
        )
    )
    return out, Calibrations(base, pink, cells, from_fourth, community)


def likelihood_claims(A: list[Attempt]) -> tuple[list[Value | Finding], LikelihoodComparison]:
    """the in-sample fits compared, and the cells and the goal-race forms against the table held out"""
    F = fits(A).log_likelihoods
    L = {m: v[0] for m, v in F.items()}
    ncell = F["pink cells bp x j x goal"][1]
    d_pink = L["pink table, recalibrated"] - L["table, recalibrated"]
    d_pen = L["pink table, recalibrated"] - L["pink no penalty, recalibrated"]
    out: list[Value | Finding] = [
        Value(SEC, "race LL recalibrated pink table - recalibrated base table, nats", d_pink, note=IN_SAMPLE),
        Value(SEC, "race LL recalibrated pink table - recalibrated pink no penalty, nats", d_pen, note=IN_SAMPLE),
        Value(SEC, "race pink cells bp x j x goal", ncell),
    ]
    ho, ho_runs = held_out_cells(A)
    comparisons = {
        "cells vs table": ho,
        "goal shift vs community table": held_out_tables(A, "pa", "pc"),
        "goal shift vs exclusion": held_out_tables(A, "pa", "px"),
    }
    for lab, c in comparisons.items():
        out += [
            Value(SEC, f"held out: {lab}: nats", c.nats, note=f"5-fold by run, {ho_runs:,} runs"),
            Value(SEC, f"held out: {lab}: SE", c.se),
            Value(SEC, f"held out: {lab}: z", c.z),
        ]
    out.append(Value(SEC, "held out: runs", ho_runs))
    return out, LikelihoodComparison(
        d_pink,
        d_pen,
        ho,
        ho_runs,
        comparisons["goal shift vs community table"],
        comparisons["goal shift vs exclusion"],
    )


def calibration_findings(cal: Calibrations, lik: LikelihoodComparison) -> list[Value | Finding]:
    """one finding per claim the section makes about the table's calibration and fit"""
    F: Callable[..., Finding] = lambda *a: common.finding(SEC, *a)
    base, pink, gj, ho = cal.base, cal.pink, cal.from_fourth, lik.held_out
    lo70 = cal.cells["bp"]["<=70"]
    sc = cal.cells["scenario"]
    goal_j, chosen_j = cal.cells["goal j"], cal.cells["chosen j"]
    covers: Callable[[Calib], bool] = lambda c: c.low <= 1 <= c.high
    early = {j: chosen_j.get(j) for j in ("1", "2")}  # chosen races at the 1st and 2nd race in a row
    later = {j: v for j, v in chosen_j.items() if j not in early}
    rest_sc = cal.cells["scenario, goal or 3rd on"]
    held: Callable[[common.HeldOutComparison], str] = lambda c: f"{c.nats:+,.1f} +- {c.se:.1f} nats, z = {c.z:.2f}"
    return [
        F(
            "On base aptitudes (the solver's bp) the table under-predicts IT's wins: the obs/pred 95% interval lies "
            "entirely above 1",
            base.low > 1,
            cv(base),
        ),
        F(
            "On pink-adjusted aptitudes the adopted table is calibrated on goal races at every streak position and on "
            "chosen races from the 3rd race in a row on: each obs/pred 95% interval covers 1",
            bool(goal_j) and bool(later) and all(covers(v) for v in [*goal_j.values(), *later.values()]),
            "; ".join(f"goal j {c}: {cv(v)}" for c, v in goal_j.items())
            + "; "
            + "; ".join(f"chosen j {c}: {cv(v)}" for c, v in later.items()),
        ),
        F(
            "Known miss: chosen races at the 1st and 2nd race in a row are slightly over-predicted (each obs/pred 95% "
            "interval entirely below 1), which puts the overall obs/pred below 1",
            lambda: all(v.high < 1 for v in early.values()) and pink.high < 1,
            lambda: "; ".join(f"chosen j {c}: {cv(v)}" for c, v in early.items()) + f"; overall {cv(pink)}",
        ),
        F(
            f"Pink-adjusted aptitudes fit better than base ones: recalibrated pink table - recalibrated base table > "
            f"{thr(1):.2f} nats (same 2 parameters)",
            lik.pink_over_base > thr(1),
            f"{lik.pink_over_base:+,.1f} nats",
        ),
        F(
            f"The streak penalty is supported: the recalibrated pink table fits better with PEN than without (same 2 "
            f"parameters) by more than {thr(1):.2f} nats",
            lik.penalty > thr(1),
            f"{lik.penalty:+,.1f} nats",
        ),
        F(
            "A goal race takes the previous streak position's penalty: the adopted table predicts unseen runs better than "
            "the community table, which scores it at its own position (5-fold held out by run, each recalibrated, z > 1.96)",
            lik.shift_over_community.z > 1.96,
            held(lik.shift_over_community) + f" on {lik.held_out_runs:,} runs",
        ),
        F(
            "A goal race still counts toward the position of the races after it: the adopted table predicts unseen runs "
            "better than exclusion, where goal races are not counted (5-fold held out by run, z > 1.96)",
            lik.shift_over_exclusion.z > 1.96,
            held(lik.shift_over_exclusion),
        ),
        F(
            "Under the community table goal races from the 4th race in a row win more than predicted (obs/pred 95% "
            "interval entirely above 1); with the shift they are calibrated (it covers 1), as chosen races there are",
            cal.community_goal_from_fourth.low > 1 and all(covers(v) for v in gj.values()),
            f"community table {cv(cal.community_goal_from_fourth)}; adopted: goal {cv(gj['goal'])}; chosen "
            f"{cv(gj['chosen'])}",
        ),
        F(
            "The per-cell rates do not predict unseen runs better than the recalibrated pink table (5-fold held out by run, "
            "z <= 1.96)",
            ho.z <= 1.96,
            held(ho) + f" on {lik.held_out_runs:,} runs",
        ),
        F(
            "Low chances win a little less than predicted: pink-adjusted bp <= 70 has an obs/pred 95% interval entirely "
            "below 1",
            lo70.high < 1,
            cv(lo70),
        ),
        F(
            "Pink-adjusted, outside chosen races at the 1st and 2nd race in a row the table is calibrated in every "
            "scenario (each obs/pred 95% interval covers 1)",
            bool(rest_sc) and all(covers(v) for v in rest_sc.values()),
            "; ".join(f"{c}: {cv(v)}" for c, v in rest_sc.items())
            + "; all attempts, outside their interval: "
            + (", ".join(f"{c} {cv(v)}" for c, v in sc.items() if not covers(v)) or "none"),
        ),
    ]


def pink_timing_claims(A: list[Attempt]) -> list[Value | Finding]:
    """when a mid-run pink takes effect: the records say which year's inspiration fired it; the rule puts it at that year's
    April (turns 30, 54). Senior January-March attempts where counting it from the year's first turn would change the
    grade; the observed rate's 95% interval is run-clustered (common.cluster_ci with one expected per attempt)"""
    S_ = [a for a in A if 48 <= a["t"] < 54 and a["pe"] != a["pa"]]
    tw = sum(a["won"] for a in S_)
    tlo, thi = (
        common.cluster_ci([a["won"] for a in S_], [1] * len(S_), [a["run"] for a in S_])[1:3]
        if S_
        else (float("nan"),) * 2
    )
    e_apr, e_start = (sum(a[k] for a in S_) / len(S_) if S_ else float("nan") for k in ("pa", "pe"))
    return [
        Value(SEC, "proc timing: attempts", len(S_)),
        Value(SEC, "proc timing: observed", tw / len(S_) if S_ else None),
        Value(SEC, "proc timing: observed 95% low", tlo),
        Value(SEC, "proc timing: observed 95% high", thi),
        Value(SEC, "proc timing: predicted, from April", e_apr),
        Value(SEC, "proc timing: predicted, from the year start", e_start),
        common.finding(
            SEC,
            "A pink fired at an inspiration takes effect from that April, not from the start of the year: on Senior January-March "
            "attempts where the two timings differ, the April prediction lies inside the observed 95% interval and the year-start "
            "one outside",
            bool(S_) and tlo <= e_apr <= thi and not tlo <= e_start <= thi,
            f"{tw}/{len(S_)} won [{tlo:.3f}, {thi:.3f}]; April {e_apr:.3f}, year start {e_start:.3f}",
        ),
    ]


class Contrast(NamedTuple):
    """contrast(): one set's obs / pred over another's, on the same table"""

    ratio: float  # (obs / pred of the first set) / (obs / pred of the second)
    low: float  # its run-clustered 95% interval
    high: float
    z: float  # log ratio / its standard error


def contrast(A: list[Attempt], B: list[Attempt], key: str) -> Contrast:
    """obs / pred on A over obs / pred on B, with a run-clustered 95% interval on the log scale: each ratio R = O / E
    linearised per run as (O_c - R E_c) / O (common.cluster_ci's estimator, on log R), the two sets' terms differenced
    per run so that a run in both sets counts once, Var = C / (C - 1) * sum_runs u_c^2 over the C runs in either set"""
    O = [sum(a["won"] for a in S) for S in (A, B)]
    E = [sum(a[key] for a in S) for S in (A, B)]
    if not all(O) or not all(E):
        return Contrast(*(float("nan"),) * 4)
    R = [o / e for o, e in zip(O, E)]
    u: dict[int, float] = collections.defaultdict(float)  # run -> its term of log(ratio)
    for sign, S, o, r in ((1, A, O[0], R[0]), (-1, B, O[1], R[1])):
        for a in S:
            u[a["run"]] += sign * (a["won"] - r * a[key]) / o
    C = len(u)
    se = math.sqrt(C / (C - 1) * sum(v * v for v in u.values()))
    lr = math.log(R[0] / R[1])
    return Contrast(R[0] / R[1], math.exp(lr - 1.96 * se), math.exp(lr + 1.96 * se), lr / se)


def show_contrast(c: Contrast) -> str:
    return f"{c.ratio:.3f} [{c.low:.3f}, {c.high:.3f}], z {c.z:+.2f}"


def streak_result_claims(A: list[Attempt]) -> list[Value | Finding]:
    """within a streak, obs / pred after a loss against after a win (the race the turn before)"""
    out: list[Value | Finding] = []
    con: dict[str, Contrast] = {}
    for jb in ("2", "3", "4+"):
        S = {
            res: [a for a in A if (str(a["j"]) == jb or (jb == "4+" and a["j"] >= 4)) and a["last"] == (res == "win")]
            for res in ("win", "loss")
        }
        con[jb] = contrast(S["loss"], S["win"], "pa")
    c = con["4+"]
    lab = "race in a streak, j 4+, after a loss / after a win: obs/pred"
    out += [Value(SEC, lab, c.ratio), Value(SEC, f"{lab} 95% low", c.low), Value(SEC, f"{lab} 95% high", c.high)]
    out.append(
        common.finding(
            SEC,
            "Within a streak the previous race's result does not change the next chance: at the 2nd, 3rd and 4th or later "
            "race in a row, obs/pred after a loss over obs/pred after a win (run-clustered) has a 95% interval covering 1",
            all(con[jb].low <= 1 <= con[jb].high for jb in ("2", "3", "4+")),
            "; ".join(f"j {jb}: {show_contrast(con[jb])}" for jb in ("2", "3", "4+")),
        )
    )
    return out


# ---------------------------------------------------------------- detail
def detail() -> None:
    """the numbers behind the findings that the section does not state"""
    R = common.runs()
    A, skip, unk = attempts(R)
    print(f"aliases applied: {ALIAS}")
    print(f"skipped attempts: {dict(skip) or 'none'}; G1 names unmatched after aliases: {dict(unk) or 'none'}")
    detail_outfits(A)
    for key, lab in (("p", "base"), ("pa", "pink-adjusted")):
        cert = [a for a in A if a[key] == 1]
        zero = [a for a in A if a[key] == 0]
        print(
            f"{lab}: {sum(1 - a['won'] for a in cert):,} losses in {len(cert):,} attempts at 100%; "
            f"{sum(a['won'] for a in zero):,} wins in {len(zero):,} at 0%"
        )
    detail_cells(A)
    detail_grids(A)
    F = fits(A)
    print("\nlog-likelihoods (nats, parameters):")
    for m, v in F.log_likelihoods.items():
        print(f"  {m:32s} {v[0]:12.2f}  {v[1]}")
    print(
        "recalibration lo, hi:", {k: (round(float(a), 4), round(float(b), 5)) for k, (a, b) in F.recalibration.items()}
    )
    detail_goal_shift(A)


def detail_outfits(A: list[Attempt]) -> None:
    """attempts by outfits whose master.mdb aptitudes differ from data.json's"""
    db = common.mdb()
    if db is None:
        return
    U = {u["id"]: u["apt"][:6] for u in common.gjson("data.json")["umas"]}
    q = (
        "select proper_distance_short, proper_distance_mile, proper_distance_middle, proper_distance_long, "
        "proper_ground_turf, proper_ground_dirt from card_rarity_data where card_id = ? order by rarity limit 1"
    )
    diff = {o for o in {a["outfit"] for a in A} if (row := db.execute(q, (o,)).fetchone()) and list(row) != U[o // 100]}
    print(
        f"attempts by outfits whose aptitudes differ from data.json (first outfit): "
        f"{sum(a['outfit'] in diff for a in A):,} ({len(diff)} outfits); scored with data.json's aptitudes"
    )


def show_calib(v: Calib | None) -> str:
    return f"{v.ratio:.3f} [{v.low:.3f}, {v.high:.3f}] n {v.n:<6,}" if v else "-"


def detail_cells(A: list[Attempt]) -> None:
    """obs / pred by cell, base against pink-adjusted"""
    print("\nobs/pred [95%] by cell, base | pink-adjusted:")
    for lab, grp in (
        ("bp", lambda k: lambda a: bucket(a["bp" if k == "p" else "bpa"], BPB)),
        ("j", lambda k: lambda a: jlab(a["j"])),
        ("predicted", lambda k: lambda a: bucket(a[k], PB)),
        ("kind", lambda k: lambda a: "goal" if a["goal"] else "chosen"),
        ("scenario", lambda k: lambda a: common.SCEN[a["scen"]]),
    ):
        cb, cp = by(A, grp("p"), "p"), by(A, grp("pa"), "pa")
        for c in sorted(set(cb) | set(cp)):
            print(f"  {lab:9s} {c:14s} {show_calib(cb.get(c)):34s} | {show_calib(cp.get(c))}")
    for lab, S in (
        ("bp moved by the pink rule", [a for a in A if a["bpa"] != a["bp"]]),
        ("after an April pink firing of the race's aptitudes", [a for a in A if a["fired"]]),
    ):
        print(f"  {lab}: base {cv(calib(S, 'p'))}; pink-adjusted {cv(calib(S, 'pa'))}")


def detail_grids(A: list[Attempt]) -> None:
    """wins / attempts and the mean prediction by (bp bucket, j), base and pink-adjusted"""
    for key, bpk, lab in (("p", "bp", "base aptitudes"), ("pa", "bpa", "pink-adjusted")):
        print(f"\n{lab}: wins/attempts, mean predicted")
        print(f"{'bp \\ j':>7s}" + "".join(f"{j:>18s}" for j in ("1", "2", "3", "4", "5", "6+")))
        for _, b in BPB:
            row: list[str] = []
            for j in range(1, 7):
                S = [a for a in A if bucket(a[bpk], BPB) == b and min(a["j"], 6) == j]
                row.append(
                    f"{sum(a['won'] for a in S):>6d}/{len(S):<6d}{(sum(a[key] for a in S) / len(S) if S else 0):5.2f}"
                )
            print(f"{b:>7s}" + "".join(f"{x:>18s}" for x in row))


def detail_goal_shift(A: list[Attempt]) -> None:
    """the adopted table against the community table and exclusion, in-sample on each half of the runs (record order)"""
    print("\ngoal shift, in-sample, recalibrated LL differences:")
    for lab, S in (
        ("all", A),
        ("earlier half", [a for a in A if not a["later"]]),
        ("later half", [a for a in A if a["later"]]),
    ):
        y = np.array([a["won"] for a in S])
        shifted = recal(np.array([a["pa"] for a in S]), y)[0]
        others = {k: shifted - recal(np.array([a[k] for a in S]), y)[0] for k in ("pc", "px")}
        print(f"  {lab}: over the community table {others['pc']:+.1f} nats, over exclusion {others['px']:+.1f} nats")
    for key, lab in (("pa", "adopted"), ("pc", "community")):
        print(
            f"  goal races from the 7th in a row, {lab}: {cv(calib([a for a in A if a['goal'] and a['j'] >= 7], key))}"
        )


if __name__ == "__main__":
    common.main(claims)
    if common.have_runs():
        print()
        detail()
