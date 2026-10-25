"""Section 6, loop structures: the race weights (derived by expanding the six legacy affinities of every lineage of a
four-uma loop and a three-uma rotation into shared-win terms o(a, b)) and how often a trainee's own character appears
among her grandparents in the run records (research/data/runs.jsonl, not included; Independent Training runs from
uma-it), with how many lineages have the exact shape of a rotation or a loop lineage (by character).

Schedules for shared wins, not odds (derived, then computed on the validation results). The solver picks a structure's
race schedules to maximise sum over pairs of o(a, b) = sum_r P_a(r) P_b(r), not P-bar directly. P-bar is nondecreasing in
every o(a, b):
  o(a, b) enters a legacy's affinity A only with coefficient +3 (A_P = pair + pair + 3 o(P, P') + sum_G [tri + 3 o(P, G)],
  A_G = tri + 3 o(P, G));
  r = min(1, b (1 + A / 100)) is nondecreasing in A;
  q = 1 - prod over the six slots of (1 - r)^2 is nondecreasing in each r;
  P_lambda(all) = sum_c P(c) prod_k p_k(c), over the deck's outcomes c (independent of the schedule), where p_k(c) >= 0
  is nondecreasing in q (an uncertain target's white is GW (1 - (1 - hint_k(c)) (1 - ew_k) (1 - q))), so P_lambda is
  nondecreasing in q; the max over the trainee's outfits and the mean over the lineages keep that.
Hence, for a solved loop with its decks fixed, the optimal schedule for P-bar scores between P_found and the odds with
every pair at an upper bound on its overlap. The bound reported is PB(a, b), data.json's "pb" table, computed by the
solver's best-response search over feasible schedules (`loopopt pbtable`, docs/SOLVER.md `pair_ceilings`); P_PB is the
odds with every pair at PB, and P_floor (every o = 0) the loop without race affinity. PB is not independent of the
solver, so it is checked here against o_max(a, b) = sum_r Pmax_a(r) Pmax_b(r), an independent but loose ceiling: a
schedule attempts race r at a subset of the turns that offer it, each at clamp(bp - pen_k, 0, 100)% <= clamp(bp, 0,
100)% (pen_k >= 0), so P_a(r) <= Pmax_a(r) = 1 - prod over every turn offering r to a of (1 - clamp(bp, 0, 100)/100).
Computed on every deck result of the validation reference outputs (val/ref__*.out, val/refpu__*.out) with
val/joint_check.py's independent Python recomputation of the exact odds, the affinities rebuilt here from the reported
pairs' o and relation parts; o itself is recomputed from the reported schedule chart, o_max from data.json. PB itself is
checked against an independent MILP by the solver's tests (tests.rs pair_ceilings_match_milp, on frozen test data); the
count of its pairs is reported. Also per result: the loops the search rejected only after building their schedule
(exact.schedule_rejects), which no bound here covers; the winners' chart cells at aptitude basis <= 70 (section 4's known
miss); and the race model in odds: every listed loop's odds under CHART_VARIANTS (losses scaled by assumed sizes,
uniformly or at the lowest basis only; every win chance 1% lower), the winners' gain at the priced scale, whether any
listed runner-up then passes its winner, and the 1% change against the smallest runner-up gap."""

import collections, itertools, json, math, os, statistics, struct, sys
from dataclasses import dataclass
from collections.abc import Callable, Hashable, Iterable
from typing import Any, NamedTuple
import common
from common import Finding, Run, Value

SEC: str = "6"

# o(a, b): shared wins of each pair of a loop's members, by member index (a < b)
Overlap = dict[tuple[int, int], float]


def lineage_terms(T: Hashable, P1: Hashable, P2: Hashable, X: Hashable) -> collections.Counter[frozenset[Hashable]]:
    """coefficient of each o(a, b) (unordered pair) in the six affinities of lineage (T, P1, P2, X), in units of 3:
    A_P = ... + 3 o(P, P') + sum over G in par(P) of 3 o(P, G); A_G = ... + 3 o(P, G) (RESEARCH.md section 6)"""
    c: collections.Counter[frozenset[Hashable]] = collections.Counter()
    par = {P1: (P2, X), P2: (P1, X)}
    for P, Pp in ((P1, P2), (P2, P1)):
        c[frozenset((P, Pp))] += 1  # A_P: o(P, P')
        for G in par[P]:
            c[frozenset((P, G))] += 1  # A_P: o(P, G)
            c[frozenset((P, G))] += 1  # A_G: o(P, G)
    return c


def weights(lineages: Iterable[tuple[Hashable, ...]]) -> collections.Counter[frozenset[Hashable]]:
    """each o(a, b)'s coefficient summed over a structure's lineages"""
    tot: collections.Counter[frozenset[Hashable]] = collections.Counter()
    for lineage in lineages:
        tot.update(lineage_terms(*lineage))
    return tot


def shape(r: Run) -> str | None:
    """'rotation' when the lineage is (T, P1, P2, T) by character, 'loop' when (T, P1, P2, X) with X not T, else None"""
    T, ps = r["chara"], r["parents"]
    if len(ps) != 2 or any(len(p["gps"]) != 2 for p in ps):
        return None
    a, b = ps[0]["chara"], ps[1]["chara"]
    ga, gb = sorted(g["chara"] for g in ps[0]["gps"]), sorted(g["chara"] for g in ps[1]["gps"])
    X = [x for x in ga if x != b]
    if len(X) != 1 or ga != sorted([b, X[0]]) or gb != sorted([a, X[0]]):
        return None
    return "rotation" if X[0] == T else "loop"


def claims() -> list[Value | Finding]:
    out: list[Value | Finding] = []
    one = lineage_terms("T", "P1", "P2", "X")
    out += [
        Value(SEC, "one lineage: o(P1, P2) weight", one[frozenset(("P1", "P2"))]),
        Value(SEC, "one lineage: o(P1, X) weight", one[frozenset(("P1", "X"))]),
    ]
    m = "abcd"  # a four-uma loop's members
    loop = [
        (t, p1, p2, x)
        for t in m
        for p1, p2 in itertools.combinations([u for u in m if u != t], 2)
        for x in [u for u in m if u not in (t, p1, p2)]
    ]
    wl = weights(loop)
    out += [
        Value(SEC, "four-uma loop: lineages", len(loop)),
        Value(SEC, "loop: weight per pair", wl[frozenset("ab")] if len(set(wl.values())) == 1 else -1),
    ]
    r = "abc"  # a three-uma rotation's members
    rot = [(t, *[u for u in r if u != t], t) for t in r]
    wr = weights(rot)
    out += [
        Value(SEC, "three-uma rotation: lineages", len(rot)),
        Value(SEC, "rotation: weight per pair", wr[frozenset("ab")] if len(set(wr.values())) == 1 else -1),
    ]
    out.append(
        common.finding(
            SEC,
            "Race weights are uniform over pairs, so a structure's schedules maximise the plain sum of "
            "o(a, b): every one of the 6 loop pairs and the 3 rotation pairs gets one weight",
            len(wl) == math.comb(len(m), 2)
            and len(set(wl.values())) == 1
            and len(wr) == math.comb(len(r), 2)
            and len(set(wr.values())) == 1,
            f"loop weights {sorted(wl.values())}, rotation weights {sorted(wr.values())}",
        )
    )
    out += own_character_claims()
    out += schedule_claims()
    return out


def own_character_claims() -> list[Value | Finding]:
    """how often the trainee's own character sits among her grandparents in the run records"""
    R = common.runs()
    if R is None:
        return []
    gp = sum(any(g["chara"] == r["chara"] for p in r["parents"] for g in p["gps"]) for r in R)
    sh = collections.Counter(shape(r) for r in R)
    out: list[Value | Finding] = [
        Value(SEC, "runs", len(R)),
        Value(SEC, "a grandparent of her own character", gp),
        Value(SEC, "exact rotation lineages", sh["rotation"]),
    ]
    lo, hi = common.wilson(gp, len(R))
    out.append(
        common.finding(
            SEC,
            "The rotation (X = T) occurs in practice: runs with a grandparent of the trainee's own "
            "character, and runs whose lineage has the exact rotation shape, are both present",
            gp > 0 and sh["rotation"] > 0,
            f"own-character grandparent {gp:,}/{len(R):,} = {gp / len(R):.3f} [{lo:.3f}, {hi:.3f}]; "
            f"exact rotation lineages {sh['rotation']}, exact loop lineages {sh['loop']}",
        )
    )
    return out


# ---------------------------------------------------------------- schedules: the bound on a different objective
VAL: str = os.path.join(common.ROOT, "val")


# A chart cell's win % under a changed race model
ChartWin = Callable[[dict[str, Any]], float]


def is_low_basis(cell: dict[str, Any]) -> bool:
    """at aptitude basis bp = 110 - the cell's aptitude cost <= 70 (section 4's lowest bucket)"""
    return 110 - cell["apt"] <= 70


def loss_scaled(scale: float, only: Callable[[dict[str, Any]], bool] = lambda _: True) -> ChartWin:
    """a cell that can be won (win % > 0), and passes `only`, loses with scale x (100 - win %)"""
    return lambda c: 100 - (100 - c["p"]) * (scale if c["p"] > 0 and only(c) else 1.0)


# Sizes assumed, not fitted. Base grades under-predict IT's wins (section 4: losses about half as often overall, most
# of the gap at the lowest basis), so a loop whose members carry aptitude sparks loses less often than its chart says.
# Uniform: every cell's losses scaled alike; low basis: only cells at bp <= 70, which moves loops unevenly.
LOSS_SCALE_PRICED: float = 0.5  # the scale whose effect on the winners the document states
CHART_VARIANTS: dict[str, ChartWin] = {
    "uniform 0.5": loss_scaled(0.5),
    "uniform 0": loss_scaled(0.0),  # every race the chart gives a chance is won
    "low basis 0.5": loss_scaled(0.5, is_low_basis),
    "low basis 0": loss_scaled(0.0, is_low_basis),
    "wins 1% lower": lambda c: 0.99 * c["p"],  # section 4's 1% calibration allowance, against the odds
}
PRICED: str = "uniform 0.5"
ONE_PCT: str = "wins 1% lower"


def o_chart(lp: dict[str, Any], win: ChartWin = lambda c: c["p"]) -> Overlap:
    """o(a, b) from a result's schedule chart: P_a(r) = 1 - prod over a's attempts at race r (by name, either year) of
    (1 - win %), the cell's win % including the streak penalty, or as `win` changes it"""
    n = len(lp["members"])
    miss: list[dict[int, float]] = [collections.defaultdict(lambda: 1.0) for _ in range(n)]
    for row in lp["schedule"]:
        for a, c in enumerate(row["cells"]):
            if c is not None:
                miss[a][row["race"]] *= 1 - win(c) / 100
    P = [{r: 1 - m for r, m in mm.items()} for mm in miss]
    return {(a, b): sum(v * P[b].get(r, 0.0) for r, v in P[a].items()) for a, b in itertools.combinations(range(n), 2)}


def o_max(lp: dict[str, Any], data: dict[str, Any]) -> Overlap:
    """o_max(a, b) = sum_r Pmax_a(r) Pmax_b(r), Pmax over every turn offering race r, no streak penalty"""
    umas = {u["id"]: u for u in data["umas"]}
    pm: list[dict[int, float]] = []
    for m in lp["members"]:
        miss: dict[int, float] = collections.defaultdict(lambda: 1.0)
        for turn in umas[m["id"]]["turns"]:
            for op in turn:
                if op["race"] >= 0:
                    miss[op["race"]] *= 1 - min(100, max(0, op["bp"])) / 100
        pm.append({r: 1 - x for r, x in miss.items()})
    n = len(pm)
    return {
        (a, b): sum(v * pm[b].get(r, 0.0) for r, v in pm[a].items()) for a, b in itertools.combinations(range(n), 2)
    }


def o_pb(lp: dict[str, Any], data: dict[str, Any]) -> Overlap:
    """the solver's exact pairwise ceilings PB(a, b) (data.json "pb", written by `loopopt pbtable`: the best-response DP
    over feasible schedules, docs/SOLVER.md `pair_ceilings`); tighter than o_max but computed by the Rust solver"""
    idx = {u["id"]: i for i, u in enumerate(data["umas"])}
    rows = data["pb"]["rows"]

    def pb(a: int, b: int) -> float:
        a, b = sorted((a, b))
        k = b - a - 1
        return struct.unpack("<d", int(rows[a][16 * k : 16 * k + 16], 16).to_bytes(8, "little"))[0]

    ids = [idx[m["id"]] for m in lp["members"]]
    return {(a, b): pb(ids[a], ids[b]) for a, b in itertools.combinations(range(len(ids)), 2)}


def lineage_q(L: dict[str, Any], o: Overlap, det: dict[str, Any]) -> tuple[float, list[float]]:
    """q of lineage L when the pairs' shared wins are o: each slot's affinity = its relation part + 3 x its o terms
    (section 5), r = min(1, b (1 + A/100)), with b multiplied by gp_rate for a grandparent, q = 1 - prod (1 - r)^inspirations; also the
    slots' race parts, to check against the reported ones"""
    (p1, p2), x = L["p"], L["x"]
    par = {p1: (p2, x), p2: (p1, x)}
    oo: Callable[[int, int], float] = lambda a, b: o[(min(a, b), max(a, b))]
    races: list[float]
    miss, races = 1.0, []
    for sl in L["slots"]:
        if sl["kind"] == "parent":
            (P,) = sl["who"]
            Pp = p2 if p1 == P else p1  # P' the other parent
            race = 3 * (oo(P, Pp) + sum(oo(P, G) for G in par[P]))
            b = det["base"]
        else:
            P, G = sl["who"]
            assert G in par[P]
            race = 3 * oo(P, G)
            b = det["gp_rate"] * det["base"]
        miss *= 1 - min(1.0, b * (1 + (sl["rel"] + race) / 100))
        races.append(race)
    return 1 - miss ** det["inspirations"], races


@dataclass
class ScheduleRow:
    """one checked loop of schedule_bound()"""

    file: str  # the validation output, tag__era__job
    top: bool  # the result's winner (else a listed runner-up with other members)
    rep: float  # the odds the solver reported
    o_le_max: bool  # every o(a, b) <= o_max(a, b)
    o_le_pb: bool  # every o(a, b) <= PB(a, b) <= o_max(a, b)
    found: float = math.nan  # P_found: the odds recomputed by val/joint_check.py at the reported o
    floor: float = math.nan  # P_floor: every o = 0
    ceil: float = math.nan  # P_ceiling: every pair at o_max
    pb: float = math.nan  # P_PB: every pair at PB
    scaled: dict[str, float] | None = None  # CHART_VARIANTS -> the odds with the chart's win chances so changed


@dataclass
class MaxErrors:
    """the recomputation's largest deviations from the reported numbers over every loop"""

    p: float = 0.0  # |recomputed P / reported - 1|
    q: float = 0.0  # |recomputed q - reported q| per lineage
    race: float = 0.0  # |recomputed race part - reported| per slot
    o: float = 0.0  # |o from the schedule chart - reported o| per pair


class ChartCells(NamedTuple):
    """a winner's schedule chart, cell by cell"""

    attempts: int  # cells (race attempts)
    low_basis: int  # at aptitude basis bp = 110 - the cell's aptitude cost <= 70 (section 4's known miss)
    inconsistent: int  # cells whose win % is not clamp(bp - pen, 0, 100)


class ScheduleBound(NamedTuple):
    rows: list[ScheduleRow]
    errors: MaxErrors
    penalties: set[int]  # every streak penalty in the eras' data.json
    rejects: dict[str, int]  # result file -> loops the search rejected only after building their schedule
    charts: dict[str, ChartCells]  # result file -> its winner's chart


def chart_cells(lp: dict[str, Any]) -> ChartCells:
    """count a loop's chart cells and those at the lowest aptitude basis"""
    cells = [c for row in lp["schedule"] for c in row["cells"] if c is not None]
    return ChartCells(
        len(cells),
        sum(110 - c["apt"] <= 70 for c in cells),
        sum(c["p"] != min(100, max(0, 110 - c["apt"] - c["pen"])) for c in cells),
    )


def check_loop(
    t: dict[str, Any], data: dict[str, Any], name: str, is_top: bool, err: MaxErrors
) -> tuple[ScheduleRow, list[list[float]]]:
    """one listed loop: its row, and its per-lineage q at the reported o, at o = 0, at o_max, at PB and at the chart's
    o under each of CHART_VARIANTS (in that order, as joint_check reads them); the recomputation's deviations go into
    err"""
    lp = t["loop"]
    det = lp["detail"]
    o_rep = {(p["a"], p["b"]): p["wins"] for p in det["pairs"]}
    for k, v in o_chart(lp).items():
        err.o = max(err.o, abs(v - o_rep[k]))
    om = o_max(lp, data)
    opb = o_pb(lp, data)
    vs: list[list[float]] = []
    for o in (o_rep, dict.fromkeys(o_rep, 0.0), om, opb, *(o_chart(lp, w) for w in CHART_VARIANTS.values())):
        vs.append([lineage_q(L, o, det)[0] for L in det["lineages"]])
    for L, q in zip(det["lineages"], vs[0]):
        err.q = max(err.q, abs(q - L["q"]))
        for sl, race in zip(L["slots"], lineage_q(L, o_rep, det)[1]):
            err.race = max(err.race, abs(race - sl["race"]))
    row = ScheduleRow(
        file=name,
        top=is_top,
        rep=lp["p_all"],
        o_le_max=all(o_rep[k] <= om[k] + 1e-9 for k in o_rep),
        o_le_pb=all(o_rep[k] <= opb[k] + 1e-9 and opb[k] <= om[k] + 1e-9 for k in o_rep),
    )
    return row, vs


def check_result_file(
    out: dict[str, Any], job: dict[str, Any], data_dir: str, data: dict[str, Any], name: str, err: MaxErrors
) -> list[ScheduleRow]:
    """the rows of one validation output: the winner and each listed loop with other members (a listed same-loop entry
    differs only in its deck), with P_found, P_floor, P_ceiling and P_PB from val/joint_check.py"""
    import joint_check

    tops = out["top"]
    keep = [
        t
        for i, t in enumerate(tops)
        if i == 0 or [m["id"] for m in t["loop"]["members"]] != [m["id"] for m in tops[0]["loop"]["members"]]
    ]
    rows: list[ScheduleRow] = []
    q_vectors: dict[int, list[list[float]]] = {}  # id(listed loop) -> its q vectors
    for t in keep:
        row, q_vectors[id(t)] = check_loop(t, data, name, t is tops[0], err)
        rows.append(row)
    _, alt = joint_check.check(dict(out, top=keep), job, data_dir, alt_q=lambda t: q_vectors[id(t)])
    for r, (pf, p0, pc, pb, *scaled) in zip(rows, alt):
        r.found, r.floor, r.ceil, r.pb = pf, p0, pc, pb
        r.scaled = dict(zip(CHART_VARIANTS, scaled))
        err.p = max(err.p, abs(pf / r.rep - 1))
    return rows


def schedule_bound(tags: tuple[str, ...] = ("ref", "refpu")) -> ScheduleBound:
    """per deck result of the validation reference outputs: the winner's (and each listed runner-up loop's) P_found,
    P_floor, P_ceiling, recomputed by val/joint_check.py at q rebuilt from o"""
    sys.path.insert(0, VAL)
    bound = ScheduleBound([], MaxErrors(), set(), {}, {})
    for era, j in json.load(open(os.path.join(VAL, "jobs.json"))):
        job = json.load(open(os.path.join(VAL, "jobs", j + ".json")))
        if job.get("mode") != "deck":
            continue
        dd = os.path.join(common.ROOT, "pkg", "loopopt", "data", era)
        data = json.load(open(os.path.join(dd, "data.json")))
        bound.penalties.update(data["pen"])
        for tag in tags:
            f = os.path.join(VAL, f"{tag}__{era}__{j}.out")
            if not os.path.exists(f):
                continue
            out = json.load(open(f))
            name = f"{tag}__{era}__{j}"
            bound.rows.extend(check_result_file(out, job, dd, data, name, bound.errors))
            bound.rejects[name] = out["exact"]["schedule_rejects"]
            bound.charts[name] = chart_cells(out["top"][0]["loop"])
    return bound


# the solver's pair ceilings against an independent MILP (pkg/loopopt/src/tests.rs pair_ceilings_match_milp, on the
# frozen test data tests/fixtures/data_v1.json; val/milp_fixtures.py writes it)
PB_MILP: str = os.path.join(common.ROOT, "pkg", "loopopt", "tests", "fixtures", "pb_milp.json")


def schedule_claims() -> list[Value | Finding]:
    rows, err, pens, rejects, charts = schedule_bound()
    W = [r for r in rows if r.top]
    RU = [r for r in rows if not r.top]  # winners, runner-ups
    worth = [100 * (r.found / r.floor - 1) for r in W]
    gpb = [100 * (r.pb / r.found - 1) for r in W]
    gmax = [100 * (r.ceil / r.found - 1) for r in W]  # the loose independent ceiling's gain
    rejecting = {f: n for f, n in rejects.items() if n}
    cells = ChartCells(*(sum(c[i] for c in charts.values()) for i in range(len(ChartCells._fields))))
    out: list[Value | Finding] = [
        Value(SEC, "schedules: result files", len(W)),
        Value(SEC, "schedules: recomputed odds max relative error", err.p),
        Value(SEC, "schedules: shared-win value min %", min(worth)),
        Value(SEC, "schedules: shared-win value max %", max(worth)),
        Value(SEC, "schedules: runner-up loops checked", len(RU)),
        Value(SEC, "schedules: PB ceiling gain max %", max(gpb)),
        Value(SEC, "schedules: PB ceiling gain median %", statistics.median(gpb)),
        Value(SEC, "schedules: PB ceiling gain min %", min(gpb)),
        Value(SEC, "schedules: o_max ceiling gain min %", min(gmax)),
        Value(SEC, "schedules: o_max ceiling gain max %", max(gmax)),
        Value(SEC, "schedules: PB pairs checked against the MILP", len(json.load(open(PB_MILP)))),
        Value(SEC, "schedules: rejected after scheduling", sum(rejects.values())),
        Value(SEC, "schedules: result files with such rejects", len(rejecting)),
        Value(SEC, "schedules: such rejects max per result", max(rejects.values())),
        Value(SEC, "schedules: winners' chart attempts", cells.attempts),
        Value(SEC, "schedules: winners' chart attempts at bp <=70", cells.low_basis),
        Value(SEC, "schedules: winners' chart share at bp <=70", cells.low_basis / cells.attempts),
        common.finding(
            SEC,
            "The search also rejects loops after building their schedule (exact.schedule_rejects, the reported pass "
            "only): each failed a bound against the k-th best value found so far (the last listed place at that "
            "point), not against the final listing or its winner, so how many a better schedule could have kept is unknown; no bound here "
            "excludes them (counted per result)",
            sum(rejects.values()) > 0,
            f"{sum(rejects.values())} in {len(rejecting)} of {len(rejects)} results: "
            + ", ".join(f"{f} {n}" for f, n in sorted(rejecting.items(), key=lambda x: -x[1])),
        ),
        common.finding(
            SEC,
            "Every winner's schedule chart states each win chance as clamp(110 - aptitude cost - streak penalty, 0, 100)",
            cells.inconsistent == 0,
            f"{cells.inconsistent} of {cells.attempts} cells off; {cells.low_basis} at bp <= 70",
        ),
    ]
    out += loss_scale_claims(W, RU)
    out.append(
        common.finding(
            SEC,
            "The independent recomputation reproduces every validated loop's reported odds and q from "
            "the reported o and relation parts, and o from the schedule chart",
            err.p < 1e-9 and err.q < 1e-12 and err.race < 1e-9 and err.o < 1e-9,
            f"max |P/reported - 1| {err.p:.1e}, |dq| {err.q:.1e}, |d race part| {err.race:.1e}, "
            f"|d o| {err.o:.1e} over {len(rows)} loops",
        )
    )
    out.append(
        common.finding(
            SEC,
            "The solver's pair ceilings sit between: o(a, b) <= PB(a, b) <= o_max(a, b) on every pair of every loop",
            all(r.o_le_pb for r in rows),
            f"{sum(r.o_le_pb for r in rows)}/{len(rows)} loops",
        )
    )
    out.append(
        common.finding(
            SEC,
            "The bound applies: streak penalties are >= 0, every reported o(a, b) is at most o_max(a, b), "
            "and on every loop P_floor <= P_found <= P_ceiling (the odds rise with o, as derived)",
            min(pens) >= 0 and all(r.o_le_max and r.floor <= r.found <= r.ceil for r in rows),
            f"penalties {sorted(pens)}; {sum(r.o_le_max for r in rows)}/{len(rows)} loops with o <= o_max; "
            f"{sum(r.floor <= r.found <= r.ceil for r in rows)}/{len(rows)} ordered",
        )
    )
    out.append(
        common.finding(
            SEC,
            "On every loop P_found <= P_PB <= P_ceiling (the odds rise with o, and o <= PB <= o_max)",
            all(r.found <= r.pb <= r.ceil for r in rows),
            f"{sum(r.found <= r.pb <= r.ceil for r in rows)}/{len(rows)} ordered; PB ceiling gain on "
            f"the winners {min(gpb):.2f}-{max(gpb):.2f}% (median {statistics.median(gpb):.2f}%)",
        )
    )
    if RU:
        out += runner_up_claims(W, RU)
    return out


def loss_scale_claims(W: list[ScheduleRow], RU: list[ScheduleRow]) -> list[Value | Finding]:
    """the race model's base-grade simplification in odds: each winner's odds with its chart's losses scaled
    (CHART_VARIANTS, uniform and at the lowest basis only), and whether a listed runner-up (changed alike) then passes
    its winner; and what section 4's 1% allowance moves: each winner's odds with every chart win chance 1% lower,
    against the smallest gain a listed runner-up needs to tie its winner"""
    change: Callable[[str], list[float]] = lambda v: [100 * (r.scaled[v] / r.found - 1) for r in W]
    gain, one_pct = change(PRICED), [abs(x) for x in change(ONE_PCT)]
    winner = {w.file: w for w in W}
    passes = {v: [r.file for r in RU if r.scaled[v] > winner[r.file].scaled[v]] for v in CHART_VARIANTS}
    gap = [100 * (winner[r.file].found / r.found - 1) for r in RU]  # the gain each listed runner-up needs to tie
    return [
        Value(SEC, "schedules: loss scale priced", LOSS_SCALE_PRICED, note="assumed"),
        Value(SEC, "schedules: winners' gain at the loss scale min %", min(gain)),
        Value(SEC, "schedules: winners' gain at the loss scale max %", max(gain)),
        Value(SEC, "schedules: winners' gain at the loss scale median %", statistics.median(gain)),
        Value(SEC, "schedules: loss scalings tried", len(CHART_VARIANTS) - 1),
        Value(SEC, "schedules: winners' odds change with wins 1% lower max %", max(one_pct)),
        Value(SEC, "schedules: winners' odds change with wins 1% lower median %", statistics.median(one_pct)),
        Value(SEC, "schedules: smallest runner-up gap to its winner %", min(gap)),
        common.finding(
            SEC,
            f"Check (it cannot fail: the odds rise with o by derivation): with each chart cell's loss chance scaled by "
            f"{LOSS_SCALE_PRICED} (assumed), every winner's odds rise",
            all(g > 0 for g in gain),
            f"gain {min(gain):.2f}-{max(gain):.2f}%, median {statistics.median(gain):.2f}%",
        ),
        common.finding(
            SEC,
            "Lowering the listed loops' scheduled losses, uniformly or at the lowest aptitude basis only, at the "
            "assumed scales tried, reorders no listed top: no listed runner-up passes its winner (unlisted loops not "
            "rechecked)",
            not any(passes[v] for v in CHART_VARIANTS if v != ONE_PCT),
            "; ".join(f"{v}: {', '.join(passes[v]) or 'none'}" for v in CHART_VARIANTS if v != ONE_PCT),
        ),
        common.finding(
            SEC,
            "Section 4's 1% allowance is not small in odds: every scheduled win chance 1% lower moves some winner's "
            "odds by more than the smallest gain a listed runner-up needs to tie its winner; applied alike to every "
            "listed loop, it reorders no listed top",
            max(one_pct) > min(gap) and not passes[ONE_PCT],
            f"winners' change up to {max(one_pct):.2f}% (median {statistics.median(one_pct):.2f}%); smallest runner-up "
            f"gap {min(gap):.2f}%; runner-ups passing their winner: {', '.join(passes[ONE_PCT]) or 'none'}",
        ),
    ]


def runner_up_claims(W: list[ScheduleRow], RU: list[ScheduleRow]) -> list[Value | Finding]:
    """whether a listed runner-up loop's ceiling at the solver's PB reaches its winner's odds found"""
    winner_found = {w.file: w.found for w in W}
    ppb = [r for r in RU if r.pb >= winner_found[r.file]]
    over = [100 * (r.pb / winner_found[r.file] - 1) for r in ppb]  # how far each such ceiling passes its winner, %
    need = [100 * (winner_found[r.file] / r.found - 1) for r in ppb]  # the gain over its own odds found that would tie
    line: Callable[[ScheduleRow], str] = lambda r: (
        f"{r.file}: runner-up found {r.found:.6f}, PB ceiling {r.pb:.6f} vs winner {winner_found[r.file]:.6f}"
    )
    return [
        Value(SEC, "schedules: runner-ups whose PB ceiling reaches the winner", len(ppb)),
        Value(SEC, "schedules: result files with such a runner-up", len({r.file for r in ppb})),
        Value(SEC, "schedules: runner-up PB ceiling over its winner max %", max(over, default=0.0)),
        Value(SEC, "schedules: such a runner-up's gain needed to tie min %", min(need, default=math.nan)),
        Value(SEC, "schedules: such a runner-up's gain needed to tie max %", max(need, default=math.nan)),
        common.finding(
            SEC,
            "Known miss: the solver's pair ceilings do not certify every listed top: on some results a listed "
            "runner-up loop's PB ceiling reaches its winner's odds found, so a schedule the search did not find could "
            "reorder that top (each named, with its odds found)",
            bool(ppb),
            f"{len(ppb)} of {len(RU)} reach: "
            + ("; ".join(line(r) for r in ppb) or "none")
            + " | every runner-up: "
            + "; ".join(line(r) for r in RU),
        ),
    ]


if __name__ == "__main__":
    common.main(claims)
