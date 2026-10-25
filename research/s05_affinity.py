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
"""

import collections, math
from collections.abc import Callable
from typing import Any, NamedTuple
import common
from common import GRANDPARENT_ROWS, PAIR_ROW, Trial, lineage, relations, trials
from common import Finding, Run, Value

SEC: str = "5"
CHI95: float = 1.920729


class PassSum(NamedTuple):
    """one ancestor category's passes (zsum())"""

    observed: int
    predicted: float
    z: float  # (observed - predicted) / its run-clustered sd


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
    ts = trials(R)
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
