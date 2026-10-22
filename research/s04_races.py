"""Section 4: the race win table, tested against the races Independent Training runs actually ran.

The model (pkg/loopopt/src/lib.rs, data.json `pen`): an attempt at one of the 34 single-race G1s wins with
p = clamp(bp - PEN_j, 0, 100) / 100, bp = 110 - cost(distance grade) - cost(surface grade) from her base aptitudes
(data.json umas[].turns[t][].bp; cost S/A 0, B 10, C 20, D 30, E 40, F 60, G 90), PEN_j = data.json `pen` for the j-th
race in a row (any race, goals included; a turn without a race resets). A community table, assumed.

Reproduces from the run records (research/data/runs.jsonl, not included; Independent Training runs from uma-it):
- every attempt at one of the 34 by a trainee in data.json: the record's race name (after ALIAS), matched to the
  option data.json offers her at that turn index, t = (year - 1) * 24 + (month - 1) * 2 + (half - 1); attempts
  that do not match are counted and listed by detail();
- j from the run's own race turns (every race the career ran: debut, goals, G2/G3/OP);
- won = the record's win flag; predicted p from the table; obs / pred with its run-clustered 95% interval
  (common.cluster_ci: one run gives several attempts), overall, by bp, by j, by scenario, goal or chosen, and by
  predicted p;
- log-likelihoods on the same attempts: the table as stated (minus infinity when a certain prediction misses), a
  constant rate, the table with and without streak penalties each recalibrated as lo + (hi - lo) p (2 parameters),
  and free fits: a rate per (bp, j) cell and per bp (j capped at 6); these are in-sample, attempts treated as
  independent; the cells against the recalibrated pink table are also compared held out (5-fold by run, held_out_cells);
- aptitude raised by pink sparks: the planner's rule (docs/MODEL.md: at the start the six legacies' pink stars per
  aptitude raise the base grade by 1/2/3/4 at 1/4/7/10 stars, at most A; each April firing recorded in the run, years
  2 and 3 from turns 30 and 54 on, +1 grade, at most S) gives a pink-adjusted bp. Every calibration is reported on both
  the base bp (what the solver uses) and the pink-adjusted bp, as are the attempts the rule moves and those after an
  April firing of the race's distance or surface; the fits use the pink-adjusted bp;
- where the residual sits: goal and chosen races from the 4th in a row; detail() also scores a goal race one streak
  position earlier (j - 1), a lead those cells suggest, on each half of the runs (record order).
Findings: one per claim of section 4 (base under-predicts; pink-adjusted calibrated overall and per scenario; pink
beats base; the streak penalty; the table not exact in-sample, the cells not better held out; the excess in goal races from the 4th in a row). The other
cells, the certain-outcome counts and the fits behind them are printed by detail().

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
    p: float  # win chance: table on base aptitudes
    p0: float  # ... without the streak penalty
    pa: float  # pink table
    pa0: float  # pink table without the streak penalty
    scen: int  # scenario id
    goal: bool  # a career goal (run regardless), else chosen by IT
    outfit: int
    fired: bool  # after an April pink firing of the race's distance or surface
    later: bool  # in the later half of the runs (record order)
    last: int | None  # the previous race's result, when it was the turn before
    t: int  # turn index
    pe: float  # pink table, a mid-run pink counted from the year's first turn
    pg: float  # pink table, a goal race scored one streak position earlier


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
    """the table's chance at bp for the j-th race in a row"""
    return min(100, max(0, bp - pen[min(j, 6) - 1])) / 100


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
        prev: int | None = None
        prev_won: int | None = None
        for t, x in races:
            consecutive = prev == t - 1
            j = j + 1 if consecutive else 1
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
                    p=win_p(op["bp"], j, pen),
                    p0=win_p(op["bp"], 1, pen),
                    pa=win_p(bpa, j, pen),
                    pa0=win_p(bpa, 1, pen),
                    scen=r["scenario"],
                    goal=bool(x["required"]),
                    outfit=r["outfit"],
                    fired=any(t0 <= t for g in (gd, gs) for t0 in fired[g]),
                    later=2 * i >= len(R),
                    last=last,
                    t=t,
                    pe=win_p(bp_of(early[0], early[1]), j, pen),
                    pg=win_p(bpa, j - 1 if x["required"] and j > 1 else j, pen),
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


def recal(q: np.ndarray, y: np.ndarray) -> tuple[float, float, float]:
    """the table recalibrated as lo + (hi - lo) q: (LL, lo, hi), fitted by maximum likelihood"""
    sig: Callable[[Any], Any] = lambda z: 1 / (1 + np.exp(-z))
    nll: Callable[[np.ndarray], float] = lambda th: (
        -ll(np.clip(sig(th[0]) + (sig(th[1]) - sig(th[0])) * q, 1e-12, 1 - 1e-12), y)
    )
    res = minimize(nll, [-1.0, 4.0], method="Nelder-Mead", options={"xatol": 1e-8, "fatol": 1e-9, "maxiter": 20000})
    return -res.fun, sig(res.x[0]), sig(res.x[1])


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
    for key, lab in (("p", "table"), ("p0", "no penalty"), ("pa", "pink table"), ("pa0", "pink no penalty")):
        q = np.array([a[key] for a in A])
        out.log_likelihoods[lab] = (ll(q, y), 0)
        L, lo, hi = recal(q, y)
        out.log_likelihoods[lab + ", recalibrated"] = (L, 2)
        out.recalibration[lab] = (lo, hi)
    out.log_likelihoods["pink cells bp x j"] = saturated(A, lambda a: (a["bpa"], min(a["j"], 6)))
    out.log_likelihoods["pink cells bp"] = saturated(A, lambda a: a["bpa"])
    return out


def held_out_cells(A: list[Attempt]) -> tuple[common.HeldOutComparison, int]:
    """rule 3 for the cells: 5-fold cross-validation by common.fold(run). On four folds, a rate per (pink bp, j) cell,
    (wins + 1/2) / (attempts + 1) (Jeffreys; a raw 0 or 1 makes the held-out score that of the 1e-12 clip; a cell absent
    there takes the pink table's chance), and the pink table, each recalibrated as lo + (hi - lo) q on those folds; each
    run of the fifth scored by both. Returns common.compare_held_out(cells, table) and the run count"""
    cell: Callable[[Attempt], tuple[int, int]] = lambda a: (a["bpa"], min(a["j"], 6))
    y = np.array([a["won"] for a in A])
    pa = np.array([a["pa"] for a in A])
    run = np.array([a["run"] for a in A])
    folds = np.array([common.fold(r) for r in run])
    ll_cells = np.zeros(len(A))
    ll_table = np.zeros(len(A))  # each attempt's held-out log-likelihood
    for k in range(common.K_FOLDS):
        train, test = folds != k, folds == k
        g: dict[tuple[int, int], list[int]] = collections.defaultdict(lambda: [0, 0])  # cell -> [wins, attempts]
        for a in (a for a, t in zip(A, train) if t):
            c = g[cell(a)]
            c[0] += a["won"]
            c[1] += 1
        q = np.array([(g[cell(a)][0] + 0.5) / (g[cell(a)][1] + 1) if cell(a) in g else a["pa"] for a in A])
        for qq, dst in ((q, ll_cells), (pa, ll_table)):
            _, lo, hi = recal(qq[train], y[train])
            p_ = np.clip(lo + (hi - lo) * qq[test], 1e-12, 1 - 1e-12)  # as recal() fits
            dst[test] = np.where(y[test] == 1, np.log(p_), np.log(1 - p_))
    runs_ = sorted(set(run.tolist()))
    ix = {r: i for i, r in enumerate(runs_)}
    per_c = np.zeros(len(runs_))
    per_t = np.zeros(len(runs_))
    np.add.at(per_c, [ix[r] for r in run], ll_cells)
    np.add.at(per_t, [ix[r] for r in run], ll_table)
    return common.compare_held_out(per_c, per_t), len(runs_)


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


def after_result(A: list[Attempt]) -> dict[tuple[str, str], Calib]:
    """{(j bucket, 'win' | 'loss'): calib} on spark-raised aptitudes, for attempts in a streak (j >= 2) by the result of the
    race the turn before"""
    out: dict[tuple[str, str], Calib] = {}
    for jb in ("2", "3", "4+"):
        for res in ("win", "loss"):
            S = [
                a
                for a in A
                if a["j"] >= 2 and (str(a["j"]) == jb or (jb == "4+" and a["j"] >= 4)) and a["last"] == (res == "win")
            ]
            out[(jb, res)] = calib(S, "pa")
    return out


def grid(A: list[Attempt], pen: list[int], minn: int = 30, mincol: int = 200) -> tuple[str, str]:
    """the community table's layout from the runs: columns the (distance, surface) grade pair as a sorted pair of letters
    (spark-raised grades, S as A), rows the race's position in a run of consecutive races (1 to 6+); each cell the
    observed win rate, with the table's chance in parentheses; pairs with `mincol` or more attempts in all, cells under
    `minn` attempts show a dot. Returns the markdown table and the attempts table"""
    cell: dict[tuple[str, int], list[int]] = collections.defaultdict(lambda: [0, 0])  # (pair, j) -> [wins, attempts]
    for a in A:
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
    body = cnt = ""
    for j in range(1, 7):
        line, cline = f"| {rows[j - 1]} |", f"| {rows[j - 1]} |"
        for p in cols:
            w, n = cell.get((p, j), (0, 0))
            pred = min(100, max(0, 110 - COST_L[p[0]] - COST_L[p[1]] - pen[j - 1]))
            line += f" {100 * w / n:.0f}% ({pred}%) |" if n >= minn else f" · ({pred}%) |"
            cline += f" {n:,} |"
        body += line + "\n"
        cnt += cline + "\n"
    return head + body.rstrip("\n"), head + cnt.rstrip("\n")


def calib_values(lab: str, c: Calib) -> list[Value | Finding]:
    """a cell's attempts and obs / pred with its interval, as Values named `lab` ..."""
    return [
        Value(SEC, f"{lab} attempts", c.n),
        Value(SEC, f"{lab} obs/pred", c.ratio),
        Value(SEC, f"{lab} obs/pred 95% low", c.low),
        Value(SEC, f"{lab} obs/pred 95% high", c.high),
    ]


class Calibrations(NamedTuple):
    """calibration_claims(): the calibrations the findings test"""

    base: Calib  # every attempt, base aptitudes
    pink: Calib  # every attempt, pink-adjusted
    cells: dict[str, dict[Any, Calib]]  # pink-adjusted, by 'bp', 'j' and 'scenario'
    from_fourth: dict[str, Calib]  # pink-adjusted, from the 4th race in a row: 'goal' and 'chosen'


class LikelihoodComparison(NamedTuple):
    """likelihood_claims(): the fits compared (in-sample, attempts treated as independent) and held out"""

    pink_over_base: float  # recalibrated pink table - recalibrated base table, nats
    penalty: float  # recalibrated pink table - recalibrated pink no penalty, nats
    shape: float  # pink cells bp x j - recalibrated pink table, nats
    cells: int  # pink (bp, j) cells
    held_out: common.HeldOutComparison  # cells against the table, 5-fold by run
    held_out_runs: int


def claims() -> list[Value | Finding]:
    R = common.runs()
    if R is None:
        return []
    A, skip, _ = attempts(R)
    cal_values, cal = calibration_claims(A, skip)
    lik_values, lik = likelihood_claims(A)
    g_tab, g_n = grid(A, common.gjson("data.json")["pen"])
    return (
        cal_values
        + lik_values
        + calibration_findings(cal, lik)
        + [Value(SEC, "race grid by aptitude and streak", g_tab), Value(SEC, "race grid attempts", g_n)]
        + pink_timing_claims(A)
        + streak_result_claims(A)
    )


def calibration_claims(A: list[Attempt], skip: collections.Counter[str]) -> tuple[list[Value | Finding], Calibrations]:
    """the attempts, and obs / pred overall (base and pink-adjusted) and by cell"""
    base, pink = calib(A, "p"), calib(A, "pa")
    out: list[Value | Finding] = [
        Value(SEC, "race attempts at the 34", len(A)),
        Value(SEC, "race attempts won", sum(a["won"] for a in A)),
        Value(SEC, "race attempts skipped", sum(skip.values())),
        Value(SEC, "race goal attempts", sum(a["goal"] for a in A)),
        Value(SEC, "race win rate observed", base.observed_rate),
        Value(SEC, "race win rate predicted, base", base.mean_predicted),
        Value(SEC, "race win rate predicted, pink", pink.mean_predicted),
    ]
    for lab, c in (("base", base), ("pink", pink)):
        out += [
            Value(SEC, f"race obs/pred, {lab}", c.ratio),
            Value(SEC, f"race obs/pred, {lab}, 95% low", c.low),
            Value(SEC, f"race obs/pred, {lab}, 95% high", c.high),
        ]
    b70 = by(A, lambda a: bucket(a["bp"], BPB), "p")["<=70"]
    out += [Value(SEC, "race base bp <=70 attempts", b70.n), Value(SEC, "race base bp <=70 obs/pred", b70.ratio)]
    cells = {
        "bp": by(A, lambda a: bucket(a["bpa"], BPB), "pa"),
        "j": by(A, lambda a: jlab(a["j"]), "pa"),
        "scenario": by(A, lambda a: common.SCEN[a["scen"]], "pa"),
    }
    for lab, cs in cells.items():
        for c, v in cs.items():
            out += calib_values(f"race pink {lab} {c}", v)
    from_fourth = {
        lab: calib([a for a in A if a["goal"] == (lab == "goal") and a["j"] >= 4], "pa") for lab in ("goal", "chosen")
    }
    for lab, v in from_fourth.items():
        out += calib_values(f"race pink {lab} j>=4", v)
    return out, Calibrations(base, pink, cells, from_fourth)


def likelihood_claims(A: list[Attempt]) -> tuple[list[Value | Finding], LikelihoodComparison]:
    """the in-sample fits compared, and the cells against the table held out"""
    F = fits(A).log_likelihoods
    L = {m: v[0] for m, v in F.items()}
    ncell = F["pink cells bp x j"][1]
    d_pink = L["pink table, recalibrated"] - L["table, recalibrated"]
    d_pen = L["pink table, recalibrated"] - L["pink no penalty, recalibrated"]
    d_shape = L["pink cells bp x j"] - L["pink table, recalibrated"]
    out: list[Value | Finding] = [
        Value(SEC, "race LL recalibrated pink table - recalibrated base table, nats", d_pink, note=IN_SAMPLE),
        Value(SEC, "race LL recalibrated pink table - recalibrated pink no penalty, nats", d_pen, note=IN_SAMPLE),
        Value(SEC, "race pink cells bp x j", ncell),
        Value(SEC, "race LL pink cells bp x j: extra parameters", ncell - 2),
        Value(SEC, "race LL pink cells bp x j - recalibrated pink table, nats", d_shape, note=IN_SAMPLE),
        Value(SEC, "race LL threshold, pink cells bp x j", thr(ncell - 2), note=IN_SAMPLE),
    ]
    ho, ho_runs = held_out_cells(A)
    out += [
        Value(SEC, "held out: cells vs table: nats", ho.nats, note=f"5-fold by run, {ho_runs:,} runs"),
        Value(SEC, "held out: cells vs table: SE", ho.se),
        Value(SEC, "held out: cells vs table: z", ho.z),
        Value(SEC, "held out: cells vs table: runs", ho_runs),
    ]
    return out, LikelihoodComparison(d_pink, d_pen, d_shape, ncell, ho, ho_runs)


def calibration_findings(cal: Calibrations, lik: LikelihoodComparison) -> list[Value | Finding]:
    """one finding per claim the section makes about the table's calibration and fit"""
    F: Callable[..., Finding] = lambda *a: common.finding(SEC, *a)
    base, pink, gj, ho = cal.base, cal.pink, cal.from_fourth, lik.held_out
    lo70 = cal.cells["bp"]["<=70"]
    sc = cal.cells["scenario"]
    return [
        F(
            "On base aptitudes (the solver's bp) the table under-predicts IT's wins: the obs/pred 95% interval lies "
            "entirely above 1",
            base.low > 1,
            cv(base),
        ),
        F(
            "On pink-adjusted aptitudes the table is calibrated overall: the obs/pred 95% interval covers 1",
            pink.low <= 1 <= pink.high,
            cv(pink),
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
            f"In-sample (attempts treated as independent), the table is not exact: a free rate per (pink bp, j) cell beats the recalibrated pink table by more than "
            f"chi2({lik.cells - 2}) 95% / 2 = {thr(lik.cells - 2):.2f} nats",
            lik.shape > thr(lik.cells - 2),
            f"{lik.shape:+,.1f} nats ({lik.cells} cells)",
        ),
        F(
            "The per-cell rates do not predict unseen runs better than the recalibrated pink table (5-fold held out by run, "
            "z <= 1.96)",
            ho.z <= 1.96,
            f"{ho.nats:+.1f} +- {ho.se:.1f} nats, z = {ho.z:.2f} on {lik.held_out_runs:,} runs",
        ),
        F(
            "The excess sits in goal races from the 4th in a row: their obs/pred 95% interval lies entirely above 1, while "
            "chosen races from the 4th in a row cover 1",
            gj["goal"].low > 1 and gj["chosen"].low <= 1 <= gj["chosen"].high,
            f"goal {cv(gj['goal'])}; chosen {cv(gj['chosen'])}",
        ),
        F(
            "Low chances win a little less than predicted: pink-adjusted bp <= 70 has an obs/pred 95% interval entirely "
            "below 1",
            lo70.high < 1,
            cv(lo70),
        ),
        F(
            "Pink-adjusted, the table is calibrated in every scenario: each obs/pred 95% interval covers 1",
            bool(sc) and all(v.low <= 1 <= v.high for v in sc.values()),
            "; ".join(f"{c}: {cv(v)}" for c, v in sc.items()),
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


def streak_result_claims(A: list[Attempt]) -> list[Value | Finding]:
    """within a streak, obs / pred by the previous race's result"""
    out: list[Value | Finding] = []
    ar = after_result(A)
    for (jb, res), c in ar.items():
        out += calib_values(f"race in a streak, j {jb}, after a {res}:", c)
    out.append(
        common.finding(
            SEC,
            "Within a streak the previous race's result does not change the next chance beyond the table: at the 2nd and 3rd race "
            "in a row, attempts after a win and after a loss each have an obs/pred 95% interval covering 1",
            all(ar[(jb, r_)].low <= 1 <= ar[(jb, r_)].high for jb in ("2", "3") for r_ in ("win", "loss")),
            "; ".join(f"j {jb} after a {r_}: {cv(ar[(jb, r_)])}" for jb in ("2", "3", "4+") for r_ in ("win", "loss")),
        )
    )
    out.append(
        common.finding(
            SEC,
            "From the 4th race in a row, attempts after a win and after a loss both win more than the table predicts (each 95% "
            "interval entirely above 1)",
            all(ar[("4+", r_)].low > 1 for r_ in ("win", "loss")),
            "; ".join(f"after a {r_}: {cv(ar[('4+', r_)])}" for r_ in ("win", "loss")),
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
    """a hypothesis the goal cells suggest (not a claim): the goal race scored one streak position earlier (j - 1)"""
    print("\ngoal race scored at j - 1 (a lead from the goal cells, not a claim):")
    for lab, S in (
        ("all", A),
        ("earlier half", [a for a in A if not a["later"]]),
        ("later half", [a for a in A if a["later"]]),
    ):
        y = np.array([a["won"] for a in S])
        dl = recal(np.array([a["pg"] for a in S]), y)[0] - recal(np.array([a["pa"] for a in S]), y)[0]
        print(f"  {lab}: LL(shifted, recalibrated) - LL(pink table, recalibrated) {dl:+.1f} nats")
    Lc, ncg = saturated(A, lambda a: (a["bpa"], min(a["j"], 6), a["goal"]))
    g = Lc - recal(np.array([a["pg"] for a in A]), np.array([a["won"] for a in A]))[0]
    print(
        f"  rate per (pink bp, j, goal) cell over the shifted table: {g:+.1f} nats, {ncg} cells "
        f"(chi2({ncg - 2}) 95% / 2 = {thr(ncg - 2):.2f}); losses where the shift gives 100%: "
        f"{sum(1 - a['won'] for a in A if a['pg'] == 1)}"
    )


if __name__ == "__main__":
    common.main(claims)
    if common.have_runs():
        print()
        detail()
