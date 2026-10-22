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
grid maximum-likelihood estimate, the same sums for ordinary ancestors (the fit's premise), and the records' +3 per shared win.
"""

import collections, math
from collections.abc import Callable
from typing import Any, NamedTuple
import common
from common import Finding, Run, Value

SEC: str = "5"
BASE: tuple[float, float, float] = (0.03, 0.06, 0.09)  # b by the spark's stars (1, 2, 3)
G_OF: dict[int, tuple[int, int]] = {0: (1, 2), 3: (4, 5)}  # ancestor slots: 0 P1, 1 G1a, 2 G1b, 3 P2, 4 G2a, 5 G2b
CHI95: float = 1.920729
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


class PassSum(NamedTuple):
    """one ancestor category's passes (zsum())"""

    observed: int
    predicted: float
    z: float  # (observed - predicted) / its run-clustered sd


def relations() -> Relations:
    db = common.mdb()
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


def roll(t: Trial, k: float) -> float:
    """the trial's chance with tri(T, P, T) = k pair(T, P): min(1, b (1 + A / 100))"""
    return min(1.0, t.base * (1 + (t.affinity_h0 + k * (t.affinity_h1 - t.affinity_h0)) / 100))


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
    # findings: the self-grandparent rule, the premise that the rest of the model fits these trials, the +3 rule
    z = {(k, h): zsum(ts, k, h) for k in ("G_self", "P_selfgp", "P", "G") for h in (0, 1)}
    out += [
        Value(SEC, "self-grandparent passes: z with 0", z["G_self", 0].z),
        Value(SEC, "self-grandparent passes: z with pair", z["G_self", 1].z),
    ]
    ev: Callable[[str, int], str] = lambda c, h: (
        f"{c}: {z[c, h].observed} obs, {z[c, h].predicted:.1f} pred, z {z[c, h].z:+.2f}"
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
    out.append(
        common.finding(
            SEC,
            "Premise of the fit (implicit in RESEARCH.md): the inspiration model fits the ordinary "
            "parents and grandparents of the same runs (each sum within 1.96 sd), so the self-grandparent "
            "gap is not a model-wide offset",
            all(abs(z[c, 0].z) < 1.96 for c in ("P", "G")),
            f"{ev('P', 0)}; {ev('G', 0)}",
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


def zsum(ts: list[Trial], cat: str, h: int) -> PassSum:
    """(observed passes, predicted sum, z) for one ancestor category under H0 (h = 0) or H1 (h = 1); the sum's variance is
    run-clustered (common.cluster_chi2 with one group): C / (C - 1) times the sum over runs of (a run's passes - its
    predicted passes)^2, since one run's trials share a lineage and need not be independent"""
    ps = [
        (min(1.0, t.base * (1 + (t.affinity_h1 if h else t.affinity_h0) / 100)), t.passed, t.run)
        for t in ts
        if t.category == cat
    ]
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
