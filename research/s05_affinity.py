"""Section 5: the self-grandparent fit, tri(T, P, T) = 0, and a check of the records' affinity tables.

Reproduces from the run records (research/data/runs.jsonl, not included; Independent Training runs from uma-it):
- the trials: one per (ancestor, skill white spark, career year 2 or 3) where that spark's skill is carried by exactly
  one of the six ancestors, so a year's inspiration of it can come only from her; a trial passes when that factor shows
  among the year's inspirations. Runs where the trainee equals a parent, or the parents are one character, are left out.
- each trial's predicted chance is one roll, min(1, b (1 + A / 100)), b = 3/6/9% by stars, A the ancestor's affinity
  (section 5) from relation points (master.mdb, a group counting distinct characters) plus 3 x the record's shared G1
  wins for each pair. H0 sets tri(T, P, T) = 0 for a self-grandparent; H1 sets it to pair(T, P); kappa scales between.
- the log-likelihood difference sums the self-grandparent trials and their parents' trials (both move with tri).
Also checks the records' relation-point column (base) against pair and tri computed from master.mdb.
Findings: each hypothesis's predicted pass sum against the observed one (z with a run-clustered sd: a run's trials
share its lineage, so the variance sums each run's squared residual, observed minus predicted passes), the kappa
grid maximum-likelihood estimate, the same H0/H1 test with the affinity slope s in b (1 + s A / 100) refitted on the
ordinary ancestors' trials (the fit's premise: the gap is not the ordinary trials' own misfit; section 3 keeps s = 1),
the ordinary ancestors' observed / predicted at s = 1 as context, and the records' +3 per shared win.

The planner's shared wins o(a, b) = sum_r P_a(r) P_b(r) use base aptitudes (section 4's table with the goal shift, on
data.json's base bp), which section 4 shows under-predict wins. Measured here on every career's attempts at the 34:
per (career, race), P(at least one win) on the base table against the record's win; over every pair of careers of
different characters at the same race, the summed products of their wins against the summed products of their base
chances (the planner's o for such a pair), with a delete-a-group jackknife interval over JACKKNIFE groups of runs by id,
over every row and over pairs whose two base chances lie in one band (BASE_BANDS).
Derived from it: dividing each record's shared-win part of A by that ratio gives the affinities the planner would
compute for the lineage; observed / predicted on the ordinary trials with those affinities (the pooled ratio applied to
every pair; IT's chosen schedules, not the planner's).
"""

import collections, itertools, math
from collections.abc import Callable
from typing import Any, NamedTuple
import numpy as np
import common
from common import GRANDPARENT_ROWS, PAIR_ROW, Trial, lineage, relations, trials
from common import Finding, Run, Value

SEC: str = "5"
CHI95: float = 1.920729
JACKKNIFE: int = 20  # groups of runs (by run id) for the shared-win ratio's interval
# bands of a career's base-table chance of winning a race, for where the shared-win shortfall lies
BASE_BANDS: dict[str, tuple[float, float]] = {
    "below 0.80": (0.0, 0.8),
    "0.80-0.95": (0.8, 0.95),
    "0.95 and up": (0.95, 2.0),
}
# section 4's race reading (s04_races.py; these belong in common.py): month order, the records' short race name
MONTHS: dict[str, int] = {
    m: i for i, m in enumerate(["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"])
}
ALIAS: dict[str, str] = {"M.C. Nambu Hai": "Mile Championship Nambu Hai"}


class PassSum(NamedTuple):
    """one ancestor category's passes (zsum())"""

    observed: int
    predicted: float
    z: float  # (observed - predicted) / its run-clustered sd


class PairRatio(NamedTuple):
    """observed over base-table products, summed over pairs of careers of different characters at the same race"""

    rows: int  # (career, race) rows the pairs are drawn from
    ratio: float
    low: float  # its jackknife 95% interval
    high: float


class SharedWins(NamedTuple):
    """the planner's base-aptitude shared wins against real careers' (shared_wins())"""

    rows: int  # (career, race) rows
    single: float  # observed / predicted P(at least one win) per (career, race)
    pairs: PairRatio  # every row
    by_base: dict[str, PairRatio]  # pairs whose two careers both have a base chance in the band (BASE_BANDS)


def race_turn(year: int, turn: str) -> int:
    """the turn index of a record's (year, 'Mon half'), as section 4"""
    month, half = turn.split()
    return (year - 1) * 24 + MONTHS[month] * 2 + int(half) - 1


def base_win(bp: int, position: int, goal: bool, pen: list[int]) -> float:
    """section 4's adopted table on base aptitudes: a goal race from the 2nd in a row pays the previous position's
    penalty, positions capped at 6"""
    s = min(position, 6) - 1 if goal and position >= 2 else position
    return min(100, max(0, bp - pen[min(s, 6) - 1])) / 100


def race_rows(R: list[Run]) -> np.ndarray:
    """one row per (career, race at the 34 she ran): [run id, character, race index, P(at least one win) on the base
    table, won at least once]; runs with two races on one turn, or a trainee not in data.json, are left out"""
    d = common.gjson("data.json")
    umas = {u["id"]: u for u in d["umas"]}
    index = {n: i for i, n in enumerate(d["race_names"])}
    rows: list[tuple[int, int, int, float, int]] = []
    for r in R:
        races = sorted(((race_turn(x["year"], x["turn"]), x) for x in r["races"]), key=lambda t: t[0])
        u = umas.get(r["chara"])
        if u is None or len({t for t, _ in races}) != len(races):
            continue
        miss: dict[int, float] = {}  # race index -> P(no win) over her attempts
        won: dict[int, int] = {}
        position, previous = 0, None
        for t, x in races:
            position = position + 1 if previous == t - 1 else 1  # any race counts toward the streak
            previous = t
            ri = index.get(ALIAS.get(x["race"], x["race"]))
            offered = u["turns"][t] if ri is not None and t < len(u["turns"]) else []
            option = next((o for o in offered if o["race"] == ri), None)
            if option is None:
                continue
            miss[ri] = miss.get(ri, 1.0) * (1 - base_win(option["bp"], position, x["required"], d["pen"]))
            won[ri] = won.get(ri, 0) | int(x["won"])
        rows += [(r["id"], r["chara"], ri, 1 - m, won[ri]) for ri, m in miss.items()]
    return np.array(rows, float)


def shared_wins(R: list[Run]) -> SharedWins:
    """per (career, race) and over pairs of careers of different characters at the same race, observed over the base
    table's prediction; the pairs' interval by a delete-a-group jackknife over JACKKNIFE groups of runs (by id)"""
    rows = race_rows(R)
    run, chara, race, p, won = rows.T
    _, by_race = np.unique(race, return_inverse=True)
    _, by_race_chara = np.unique(race * 10**7 + chara, return_inverse=True)

    def pair_sums(keep: np.ndarray) -> tuple[float, float]:
        """(sum of won_a won_b, sum of p_a p_b) over pairs at the same race of different characters, rows in keep"""

        def excess(group: np.ndarray) -> tuple[float, float]:  # sums over every pair within each group
            W = np.bincount(group, won * keep)
            S, SS = np.bincount(group, p * keep), np.bincount(group, p * p * keep)
            return float((W * W - W).sum()), float((S * S - SS).sum())

        (w_all, p_all), (w_same, p_same) = excess(by_race), excess(by_race_chara)
        return w_all - w_same, p_all - p_same

    group = run.astype(int) % JACKKNIFE

    def pair_ratio(keep: np.ndarray) -> PairRatio:
        """the ratio over the rows in keep (0 / 1), with its delete-a-group jackknife interval"""
        whole = float(np.divide(*pair_sums(keep)))
        leave_out = np.array([np.divide(*pair_sums(keep * (group != g))) for g in range(JACKKNIFE)])
        se = math.sqrt((JACKKNIFE - 1) / JACKKNIFE * float(((leave_out - leave_out.mean()) ** 2).sum()))
        return PairRatio(int(keep.sum()), whole, whole - 1.96 * se, whole + 1.96 * se)

    by_base = {k: pair_ratio(((p >= lo) & (p < hi)).astype(float)) for k, (lo, hi) in BASE_BANDS.items()}
    return SharedWins(len(p), float(won.sum() / p.sum()), pair_ratio(np.ones(len(p))), by_base)


def roll(t: Trial, k: float, slope: float = 1.0) -> float:
    """the trial's chance with tri(T, P, T) = k pair(T, P): min(1, b (1 + slope A / 100))"""
    return min(1.0, t.base * (1 + slope * (t.affinity_h0 + k * (t.affinity_h1 - t.affinity_h0)) / 100))


def ordinary_slope(ts: list[Trial]) -> float:
    """the maximum-likelihood affinity slope s in min(1, b (1 + s A / 100)) on the ordinary parents' and grandparents'
    trials (categories 'P' and 'G', where tri(T, P, T) plays no part)"""
    from scipy.optimize import minimize_scalar

    ordinary = [t for t in ts if t.category in ("P", "G")]

    def nll(s: float) -> float:
        total = 0.0
        for t in ordinary:
            p = min(max(roll(t, 0, s), 1e-12), 1 - 1e-12)
            total -= math.log(p if t.passed else 1 - p)
        return total

    return float(minimize_scalar(nll, bounds=(0.0, 3.0), method="bounded", options={"xatol": 1e-5}).x)


def ll(ts: list[Trial], k: float) -> float:
    """log-likelihood with tri(T, P, T) = k pair(T, P)"""
    s = 0.0
    for t in ts:
        p = roll(t, k)
        s += math.log(p if t.passed else 1 - p)
    return s


def table_check(R: list[Run]) -> collections.Counter[tuple[str, str]]:
    """the records' relation-point column (base) against master.mdb: trainee x parent and parent x parent rows against
    pair, parent x grandparent rows against pair(P, G) and tri(T, P, G); bonus = 3 x shared; total = base + bonus"""
    S, _ = relations()
    pair: Callable[[int, int], int] = lambda a, b: S(a, b) if a != b else 0
    counts: collections.Counter[tuple[str, str]] = collections.Counter()
    for r in R:
        anc = lineage(r)
        aff = r.get("affinity") or {}
        if anc is None or not aff:
            continue
        T = r["chara"]
        A = [a["chara"] for a in anc]
        for label, (x, y) in {
            "trainee × parent 1": (T, A[0]),
            "trainee × parent 2": (T, A[3]),
            PAIR_ROW: (A[0], A[3]),
        }.items():
            v = aff[label][0]
            m = pair(x, y)
            counts[("pair rows", "base = pair" if v == m else "base 0, pair > 0" if v == 0 else "other")] += 1
        for g, label in GRANDPARENT_ROWS.items():
            p = 0 if g < 3 else 3
            v = aff[label][0]
            t = S(T, A[p], A[g]) if len({T, A[p], A[g]}) == 3 else 0
            counts[
                (
                    "P x G rows",
                    "base = pair(P,G)"
                    if v == pair(A[p], A[g])
                    else "base = tri(T,P,G)"
                    if v == t
                    else "base 0"
                    if v == 0
                    else "other",
                )
            ] += 1
        for base, shared, bonus, total in aff.values():
            counts[
                (
                    "all rows",
                    "bonus = 3 x shared, total = base + bonus"
                    if bonus == 3 * shared and total == base + bonus
                    else "other",
                )
            ] += 1
    return counts


def claims() -> list[Value | Finding]:
    R = common.runs()
    if R is None:
        return []
    ts = trials(R, race=True)
    self_gps = [t for t in ts if t.category == "G_self"]
    with_parents = [t for t in ts if t.category in ("G_self", "P_selfgp")]  # both move with tri(T, P, T)

    def predicted(k: float) -> float:  # the self-grandparents' predicted passes with tri(T, P, T) = k pair(T, P)
        return sum(roll(t, k) for t in self_gps)

    grid = [i / 1000 for i in range(0, 1001)]
    L = [ll(with_parents, k) for k in grid]
    i = max(range(len(grid)), key=L.__getitem__)
    out: list[Value | Finding] = [
        Value(SEC, "trials attributable to one ancestor", len(ts)),
        Value(SEC, "self-grandparent passes observed", sum(t.passed for t in self_gps)),
        Value(SEC, "predicted, tri(T,P,T) = 0", predicted(0)),
        Value(SEC, "predicted, tri(T,P,T) = pair(T,P)", predicted(1)),
        Value(SEC, "LL(0) - LL(pair), nats", ll(with_parents, 0) - ll(with_parents, 1)),
        Value(SEC, "kappa fitted", grid[i]),
    ]
    # findings: the self-grandparent rule, the same rule with the slope refitted on the ordinary trials, the +3 rule
    z = {(k, h): zsum(ts, k, h) for k in ("G_self", "P_selfgp", "P", "G") for h in (0, 1)}
    s = ordinary_slope(ts)
    zs = {(k, h): zsum(ts, k, h, s) for k in ("G_self", "P_selfgp") for h in (0, 1)}
    out += [
        Value(SEC, "self-grandparent passes: z with 0", z["G_self", 0].z),
        Value(SEC, "self-grandparent passes: z with pair", z["G_self", 1].z),
        Value(SEC, "affinity slope refitted on the ordinary trials", s),
    ]
    ev: Callable[..., str] = lambda c, h, zz=z: (
        f"{c}: {zz[c, h].observed} obs, {zz[c, h].predicted:.1f} pred, z {zz[c, h].z:+.2f}"
    )
    out.append(
        common.finding(
            SEC,
            "tri(T, P, T) = 0 predicts the self-grandparent passes and tri(T, P, T) = pair(T, P) "
            "does not: observed within 1.96 sd of the H0 sum, beyond 1.96 sd of the H1 sum",
            abs(z["G_self", 0].z) < 1.96 and abs(z["G_self", 1].z) > 1.96,
            f"H0 {ev('G_self', 0)}; H1 {ev('G_self', 1)}; their parents: H0 {ev('P_selfgp', 0)}, "
            f"H1 {ev('P_selfgp', 1)}",
        )
    )
    ratio: Callable[[str], str] = lambda c: f"{c} {z[c, 0].observed / z[c, 0].predicted:.3f} ({ev(c, 0)})"
    out.append(
        common.finding(
            SEC,
            "Premise of the fit: the self-grandparent gap is not the ordinary "
            "trials' own misfit: with the affinity slope refitted on the ordinary parents and grandparents "
            "of the same runs, the self-grandparent passes stay within 1.96 sd of the H0 sum and beyond "
            "1.96 sd of the H1 sum",
            abs(zs["G_self", 0].z) < 1.96 and abs(zs["G_self", 1].z) > 1.96,
            f"slope {s:.3f}: H0 {ev('G_self', 0, zs)}; H1 {ev('G_self', 1, zs)}; their parents: "
            f"H0 {ev('P_selfgp', 0, zs)}, H1 {ev('P_selfgp', 1, zs)}. Context, ordinary obs / pred at slope 1: "
            f"{ratio('P')}; {ratio('G')}",
        )
    )
    out += planned_shared_win_claims(R, ts)
    tc = table_check(R)
    rows = sum(v for k, v in tc.items() if k[0] == "all rows")
    bad = tc[("all rows", "other")]
    out.append(
        common.finding(
            SEC,
            "The records' affinity tables apply the assumed +3 per shared G1 win (bonus = 3 x shared, "
            "total = base + bonus) on every row: a further implementation agreeing, not the game's rule",
            rows > 0 and bad == 0,
            f"{rows - bad:,} of {rows:,} rows",
        )
    )
    return out


def planned_shared_win_claims(R: list[Run], ts: list[Trial]) -> list[Value | Finding]:
    """the planner's base-aptitude shared wins against real careers', and what dividing the records' shared-win part
    of affinity by that ratio does to observed / predicted on the ordinary parents' and grandparents' trials"""
    sw = shared_wins(R)
    shrink = 1 - 1 / sw.pairs.ratio  # the share of each shared-win count the planner's base aptitudes leave out
    out: list[Value | Finding] = [
        Value(SEC, "career-race rows at the 34", sw.rows),
        Value(SEC, "wins per career and race, observed / base table", sw.single),
        Value(SEC, "shared wins over pairs, observed / base table", sw.pairs.ratio),
        Value(SEC, "shared wins over pairs, 95% low", sw.pairs.low),
        Value(SEC, "shared wins over pairs, 95% high", sw.pairs.high),
    ]
    for band, c in sw.by_base.items():
        out += [
            Value(SEC, f"shared wins over pairs, base chance {band}: rows", c.rows),
            Value(SEC, f"shared wins over pairs, base chance {band}: observed / base table", c.ratio),
            Value(SEC, f"shared wins over pairs, base chance {band}: 95% low", c.low),
            Value(SEC, f"shared wins over pairs, base chance {band}: 95% high", c.high),
        ]
    top = list(BASE_BANDS)[-1]
    top_shrink = 1 - 1 / sw.by_base[top].ratio  # the same at the ratio of pairs both at the top band's chances
    planned: dict[str, common.ClusterRatio] = {}  # at the pooled ratio
    planned_top: dict[str, common.ClusterRatio] = {}  # at the top band's ratio
    for cat, label in (("P", "parents"), ("G", "grandparents")):
        sel = [t for t in ts if t.category == cat]
        real = sum(min(1.0, t.base * (1 + t.affinity_h0 / 100)) for t in sel)
        chance = [min(1.0, t.base * (1 + (t.affinity_h0 - shrink * t.race_part) / 100)) for t in sel]
        planned[label] = common.cluster_ci([t.passed for t in sel], chance, [t.run for t in sel])
        chance_top = [min(1.0, t.base * (1 + (t.affinity_h0 - top_shrink * t.race_part) / 100)) for t in sel]
        planned_top[label] = common.cluster_ci([t.passed for t in sel], chance_top, [t.run for t in sel])
        at_top = f"with the planner's shared wins at the {top} ratio"
        out += [
            Value(SEC, f"{label}: obs / pred {at_top}", planned_top[label].ratio),
            Value(SEC, f"{label}: obs / pred {at_top}, 95% low", planned_top[label].low),
            Value(SEC, f"{label}: obs / pred {at_top}, 95% high", planned_top[label].high),
            Value(
                SEC,
                f"{label}: shared-win share of affinity",
                sum(t.race_part for t in sel) / sum(t.affinity_h0 for t in sel),
            ),
            Value(SEC, f"{label}: predicted passes, planner's shared wins over the records'", sum(chance) / real),
            Value(SEC, f"{label}: obs / pred with the planner's shared wins", planned[label].ratio),
            Value(SEC, f"{label}: obs / pred with the planner's shared wins, 95% low", planned[label].low),
            Value(SEC, f"{label}: obs / pred with the planner's shared wins, 95% high", planned[label].high),
        ]
    out.append(
        common.finding(
            SEC,
            "On base aptitudes the planner's shared wins fall short of real careers': over pairs of careers of "
            "different characters at the same race, both winning is more frequent than the base table's products "
            "predict (jackknife 95% interval above 1)",
            sw.pairs.low > 1,
            f"pairs {sw.pairs.ratio:.4f} [{sw.pairs.low:.4f}, {sw.pairs.high:.4f}]; one career {sw.single:.4f}, over "
            f"{sw.rows:,} career-race rows",
        )
    )
    bands = list(sw.by_base.values())  # BASE_BANDS order, lowest chance first
    out.append(
        common.finding(
            SEC,
            "The shortfall grows as the base chance falls: in every band of the two careers' base chances the pair "
            "ratio's jackknife 95% interval lies above 1, and each band's interval lies wholly above the next higher "
            "band's",
            all(c.low > 1 for c in bands) and all(a.low > b.high for a, b in itertools.pairwise(bands)),
            "; ".join(
                f"{k}: {c.ratio:.3f} [{c.low:.3f}, {c.high:.3f}] ({c.rows:,} rows)" for k, c in sw.by_base.items()
            ),
        )
    )
    out.append(
        common.finding(
            SEC,
            f"The planner's net bias on parents' rolls depends on where its races fall: with each record's shared "
            f"wins cut by the pooled pair ratio, observed / predicted over the ordinary parents' and the "
            f"grandparents' trials each has its run-clustered 95% interval above 1; cut by the ratio of pairs both "
            f"at base chances {top}, the parents' interval lies below 1",
            all(c.low > 1 for c in planned.values()) and planned_top["parents"].high < 1,
            "pooled: "
            + "; ".join(f"{k} {c.ratio:.3f} [{c.low:.3f}, {c.high:.3f}]" for k, c in planned.items())
            + f" | at {top}: "
            + "; ".join(f"{k} {c.ratio:.3f} [{c.low:.3f}, {c.high:.3f}]" for k, c in planned_top.items()),
        )
    )
    return out


def zsum(ts: list[Trial], cat: str, h: int, slope: float = 1.0) -> PassSum:
    """(observed passes, predicted sum, z) for one ancestor category under H0 (h = 0) or H1 (h = 1), affinity scaled by
    `slope` as in roll(); the sum's variance is
    run-clustered (common.cluster_chi2 with one group): C / (C - 1) times the sum over runs of (a run's passes - its
    predicted passes)^2, since one run's trials share a lineage and need not be independent"""
    ps = [(roll(t, h, slope), t.passed, t.run) for t in ts if t.category == cat]
    o = sum(f for _, f, _ in ps)
    e = sum(p for p, _, _ in ps)
    x2, _, _ = common.cluster_chi2([(0, f - p) for p, f, _ in ps], [run for _, _, run in ps])
    return PassSum(o, e, math.copysign(math.sqrt(x2), o - e))


def detail() -> None:
    R = common.runs()
    ts = trials(R)
    agg: dict[str, list[Any]] = collections.defaultdict(lambda: [0, 0, 0.0, 0.0])  # [trials, passed, pred H0, pred H1]
    for t in ts:
        x = agg[t.category]
        x[0] += 1
        x[1] += t.passed
        x[2] += min(1, t.base * (1 + t.affinity_h0 / 100))
        x[3] += min(1, t.base * (1 + t.affinity_h1 / 100))
    print(f"{'ancestor':10s} {'trials':>7s} {'passed':>7s} {'pred H0':>8s} {'pred H1':>8s}")
    for k in ("P", "P_selfgp", "P_gpp", "G", "G_self", "G_eqP"):
        x = agg[k]
        print(f"{k:10s} {x[0]:7d} {x[1]:7d} {x[2]:8.1f} {x[3]:8.1f}")
    with_parents = [t for t in ts if t.category in ("G_self", "P_selfgp")]
    print(
        "LL by kappa:",
        ", ".join(f"{k}: {ll(with_parents, k) - ll(with_parents, 0):+.2f}" for k in (0, 0.25, 0.35, 0.5, 1)),
    )
    print("\nrecords' affinity tables against master.mdb relation points:")
    for k, v in sorted(table_check(R).items()):
        print(f"  {k[0]:12s} {k[1]:16s} {v}")


if __name__ == "__main__":
    common.main(claims)
    if common.have_runs():
        print()
        detail()
