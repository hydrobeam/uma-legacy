"""Section 13 (and the inheritance test of section 3, the certainty checks of section 9): the career model against real
careers. Each run record is rebuilt as a career-planner job (trainee in her outfit, scenario, the six cards locked at
their own limit breaks, each chosen chain end pinned by the run's priority list where it shows one (end_pins: a certain end becomes a stop of
1 in 1 at its link, an impossible one removes the end entries, a fractional one, and every run showing no list, is left
to the solver, which may then take the end as a player following its advice would), both
parents with their recorded parents and every ancestor's white and pink sparks, race wins laid out so every pair the
model reads shares the record's shared-G1 count), run through the native solver, and
compared with what the run shows, per (run, target) row, pooled. Intervals (95%): observed / predicted over rows by the
run-clustered ratio estimator (common.cluster_ci; a run's targets share its deck and lineage), the observed rate's
interval that ratio interval times the mean prediction; where every run gives one row, Wilson on the observed rate
(divided by the mean prediction for the ratio). Joint tests over buckets: the run-clustered Wald chi2
(common.cluster_chi2).

  uv run --group research research/s13_calibration.py [--wide]

Steps (card variants, jobs and solver in common.py; outputs under research/data/out/calib/; solver results cached by
common.fingerprint()):
  card variants   every card at limit breaks 0-3: hint frequency, increment and specialty priority read at that level cap
                  from master.mdb (effect values linear between milestone levels, floored; unique effect from its level)
  jobs            default eight targets (Unity Cup's two golds guaranteed there); --wide: every other skill a deck card
                  or an ancestor's spark could give, in jobs of at most 12 targets, on a seeded 30% sample of the runs
  solver          one process, 100 jobs per call, every job solved to the end (no time limit)
  analysis        the tests of RESEARCH.md section 13 and the conclusions drawn on them (Findings); the full tables go
                  to report8.txt / reportW.txt

Dropped runs: a deck that is not six known cards, an R card in the deck, a trainee outfit not in web_skills.json, the
same parent twice or the trainee as a parent, shared-G1 counts that do not fit the race list."""

import collections, math, os, sys
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from itertools import pairwise
from typing import Any, NamedTuple
import numpy as np
import common
from common import Value, Finding, Run, finding, wilson, DEFAULT
from common import CalRow, RunRows, Model, OUTD, card_variants, build_jobs, run_solver, rows_of

S3, S9, S13 = "3", "9", "13"  # the sections these values and findings belong to


RowFn = Callable[[CalRow], Any]


# ---------------------------------------------------------------- tables
class Stat(NamedTuple):
    """the counts and intervals of a set of rows (row_ci): the observed rate and observed / predicted, 95%"""

    rows: int
    predicted: float  # the mean prediction
    observed: float  # the observed rate
    hits: int
    low: float  # the observed rate's interval
    high: float
    ratio: float  # observed / predicted
    ratio_low: float
    ratio_high: float


class Bucket(NamedTuple):
    """one bucket of a reliability table: rows predicted in [low_edge, high_edge)"""

    low_edge: float
    high_edge: float
    rows: int
    observed: float
    predicted: float  # the mean prediction
    observed_low: float  # the observed rate's 95% interval (row_ci)
    observed_high: float


NAN: float = float("nan")
NO_BUCKET: Bucket = Bucket(NAN, NAN, 0, NAN, NAN, NAN, NAN)
ZERO_EDGE: float = 1e-12  # the high edge of the bucket predicted 0
EDGES: list[float] = [0, ZERO_EDGE, 0.2, 0.4, 0.6, 0.8, 0.95, 1 + 1e-9]


def row_ci(rs: list[CalRow], pred: Callable[[CalRow], float], obs: RowFn) -> Stat:
    """the Stat of rows rs: where a run gives several rows, the run-clustered ratio estimator (common.cluster_ci) and
    the rate's interval that ratio interval times the mean prediction (nothing predicted: the rate's own run-clustered
    interval); where every run gives one row, Wilson on the observed rate, divided by the mean prediction for the
    ratio"""
    n = len(rs)
    if n == 0:
        return Stat(0, NAN, NAN, 0, NAN, NAN, NAN, NAN, NAN)
    ps = [pred(r) for r in rs]
    hit = [1 if obs(r) else 0 for r in rs]
    mean_pred = sum(ps) / n
    k = sum(hit)
    if len({r["run"] for r in rs}) == n:
        lo, hi = wilson(k, n)
        return Stat(
            n,
            mean_pred,
            k / n,
            k,
            lo,
            hi,
            common.div(k / n, mean_pred),
            common.div(lo, mean_pred),
            common.div(hi, mean_pred),
        )
    if not mean_pred:  # nothing predicted (no ratio): the observed rate with its run-clustered interval
        _, lo, hi, _ = common.cluster_ci(hit, [1] * n, [r["run"] for r in rs])
        return Stat(n, mean_pred, k / n, k, lo, hi, NAN, NAN, NAN)
    ratio, rlo, rhi, _ = common.cluster_ci(hit, ps, [r["run"] for r in rs])
    return Stat(n, mean_pred, k / n, k, rlo * mean_pred, rhi * mean_pred, ratio, rlo, rhi)


class Report:
    """the analysis tables written to report8.txt / reportW.txt"""

    rows: list[CalRow]
    lines: list[str]

    def __init__(self, rows: list[CalRow]) -> None:
        self.rows, self.lines = rows, []

    def line(self, *a: object) -> None:
        self.lines.append(" ".join(str(x) for x in a))

    def stat(self, sel: RowFn, pred: Callable[[CalRow], float], obs: RowFn) -> Stat:
        """the rows sel picks: counts, the observed rate and obs / pred with their 95% intervals (row_ci)"""
        return row_ci([r for r in self.rows if sel(r)], pred, obs)

    def table(
        self,
        title: str,
        sel: RowFn,
        pred: Callable[[CalRow], float],
        obs: RowFn,
        key: RowFn | None = None,
        min_rows: int = 0,
    ) -> dict[Any, Stat]:
        """the stat of the rows sel picks per value of key, largest groups first, written as a table"""
        self.line(f"\n## {title}")
        groups: collections.defaultdict[Any, list[CalRow]] = collections.defaultdict(list)
        for r in self.rows:
            if sel(r):
                groups[key(r) if key else "all"].append(r)
        res: dict[Any, Stat] = {}
        for k in sorted(groups, key=lambda k: (-len(groups[k]), str(k))):
            if len(groups[k]) < min_rows:
                continue
            s = self.stat(lambda r, k=k: sel(r) and (key(r) if key else "all") == k, pred, obs)
            res[k] = s
            self.line(
                f"{str(k)[:60]:60s} {s.rows:6d} {s.predicted:7.3f} {s.observed:7.3f} [{s.low:6.3f}, {s.high:6.3f}] "
                f"{s.ratio:7.3f} [{s.ratio_low:5.2f}, {s.ratio_high:5.2f}]"
            )
        return res

    def reliability(
        self, title: str, sel: RowFn, pred: Callable[[CalRow], float], obs: RowFn, edges: Sequence[float]
    ) -> list[Bucket]:
        """the rows sel picks, per bucket of the prediction between successive edges, written as a table"""
        self.line(f"\n## Reliability: {title}")
        res: list[Bucket] = []
        for a, b in pairwise(edges):
            s = row_ci([r for r in self.rows if sel(r) and a <= pred(r) < b], pred, obs)
            res.append(Bucket(a, b, s.rows, s.observed, s.predicted, s.low, s.high))
            if s.rows:
                self.line(
                    f"[{a:.3g}, {b:.3g}) n {s.rows:6d} pred {s.predicted:.3f} obs {s.observed:.3f} "
                    f"[{s.low:.3f}, {s.high:.3f}]"
                )
        return res


BOOTSTRAP_DRAWS: int = 10_000  # enough that the 95% bounds move by under 0.01 between seeds


def ratio_draws(obs: Sequence[float], pred: Sequence[float], seed: int, draws: int = BOOTSTRAP_DRAWS) -> np.ndarray:
    """sum(obs) / sum(pred) over seeded resamples of whole runs (one entry per run), in chunks to bound memory"""
    o, p = np.asarray(obs, float), np.asarray(pred, float)
    rng = np.random.default_rng(seed)
    out: list[np.ndarray] = []
    for start in range(0, draws, 500):
        ix = rng.integers(0, len(o), (min(500, draws - start), len(o)))
        out.append(o[ix].sum(1) / np.maximum(1e-300, p[ix].sum(1)))
    return np.concatenate(out)


def percentile_interval(v: np.ndarray) -> tuple[float, float]:
    """the 95% percentile interval of bootstrap draws"""
    lo, hi = np.quantile(v, [0.025, 0.975])
    return float(lo), float(hi)


def effective_runs(weights: Sequence[float]) -> float:
    """(sum w)^2 / sum w^2: how many equally weighted runs carry the same information as these weighted ones (a
    ratio of sums is a weighted mean, weight each run's prediction)"""
    s, s2 = sum(weights), sum(w * w for w in weights)
    return s * s / s2 if s2 else float("nan")


# ---------------------------------------------------------------- row selections and predictions
def every_row(r: CalRow) -> bool:
    return True


def is_open(r: CalRow) -> bool:
    """neither her outfit nor the scenario gives the target: the deck, inspiration and events must"""
    return not r["outfit"] and not r["scen_gold"]


def in_deck_test(r: CalRow) -> bool:
    """an open target no inspiration touched; used to compare deck-attributed hints with deck odds"""
    return is_open(r) and not r["o_insp"] and not r["o_insp_src"]


def deck_odds(r: CalRow) -> float:
    """predicted P(a deck card hints the target, gold or white)"""
    return r["dg"] + (1 - r["dg"]) * r["dw"]


def deck_hinted(r: CalRow) -> bool:
    return r["o_deck"]


def inspiration_odds(r: CalRow) -> float:
    return r["q"]


def inspired(r: CalRow) -> bool:
    return r["o_insp"]


def acquisition_odds(r: CalRow) -> float:
    return r["acq"]


def acquired(r: CalRow) -> bool:
    return r["o_any"]


def acquired_modelled(r: CalRow) -> bool:
    """acquired through a source the model has: a deck card's hint or an inspiration"""
    return r["o_deck"] or r["o_insp"] or r["o_insp_src"]


def source_kind(r: CalRow) -> str:
    """which deck sources could hint the target"""
    return (
        "pool+event"
        if r["cards_pool"] and r["cards_ev"]
        else "training pool only"
        if r["cards_pool"]
        else "events only"
        if r["cards_ev"]
        else "no deck source"
    )


def one_deck_source(r: CalRow) -> bool:
    return len(set(r["cards_pool"]) | set(r["cards_ev"])) == 1


def deck_source(r: CalRow) -> int:
    """the one deck card that can hint the target (one_deck_source)"""
    return (set(r["cards_pool"]) | set(r["cards_ev"])).pop()


def where(*conditions: RowFn) -> RowFn:
    """the rows meeting every condition, tested in order"""
    return lambda r: all(c(r) for c in conditions)


def target_is(g: int) -> RowFn:
    return lambda r: r["g"] == g


def scenario_is(scenario: int) -> RowFn:
    return lambda r: r["scen"] == scenario


def kind_is(kind: str) -> RowFn:
    return lambda r: source_kind(r) == kind


def lowest_lb_is(lb: int) -> RowFn:
    return lambda r: r["pool_lb"] == lb


def prioritised(r: CalRow) -> bool | None:
    """the target counts as prioritised: on the run's priority list and not outranked on a rival choice of every event
    offering it (prio_outranked: between two prioritised skills IT takes the higher-ranked one); None when the run
    shows no list"""
    return None if r["prio"] is None else r["prio"] and not r["prio_outranked"]


def priority_is(prio: bool | None) -> RowFn:
    return lambda r: prioritised(r) is prio


def single_source_is(card: int | None) -> RowFn:
    return lambda r: one_deck_source(r) and deck_source(r) == card


def deck_test(rep: Report, *conditions: RowFn) -> Stat:
    """test 1 (deck hints, no inspiration seen) on the rows meeting the conditions"""
    return rep.stat(where(in_deck_test, *conditions), deck_odds, deck_hinted)


def inheritance_test(rep: Report, *conditions: RowFn) -> Stat:
    """test 2 (inspiration of the target) on the rows meeting the conditions"""
    return rep.stat(where(*conditions), inspiration_odds, inspired)


def acquisition_test(rep: Report, *conditions: RowFn) -> Stat:
    """test 3 (acquired from any source, open targets) on the rows meeting the conditions"""
    return rep.stat(where(is_open, *conditions), acquisition_odds, acquired)


class Tails(NamedTuple):
    """an observed count k against the exact distribution of the rows' sum (independent rows, each a Bernoulli of its
    prediction: Poisson-binomial)"""

    rows: int
    observed: int
    expected: float
    at_most: float  # P(X <= k)
    at_least: float  # P(X >= k)

    def consistent(self) -> bool:
        """neither tail below 2.5%: k inside the two-sided 95% range"""
        return self.at_most > 0.025 and self.at_least > 0.025

    def below(self) -> bool:
        return self.at_most < 0.025

    def text(self) -> str:
        return (
            f"{self.observed}/{self.rows} vs {self.expected:.1f} expected, P(X <= {self.observed}) {self.at_most:.3f}, "
            f"P(X >= {self.observed}) {self.at_least:.3f}"
        )


def exact_tails(rep: Report, sel: RowFn, pred: Callable[[CalRow], float], obs: RowFn) -> Tails:
    """the exact tails of the rows sel picks, where each run gives one row (a Wilson interval on the ratio is too
    narrow when the prediction is near 1)"""
    rs = [r for r in rep.rows if sel(r)]
    dist = [1.0]  # P(X = i) over the rows so far
    for r in rs:
        p = pred(r)
        dist = [a * (1 - p) + b * p for a, b in zip([*dist, 0.0], [0.0, *dist])]
    k = sum(1 for r in rs if obs(r))
    return Tails(len(rs), k, sum(pred(r) for r in rs), sum(dist[: k + 1]), sum(dist[k:]))


def covers(s: Stat) -> bool:
    return s.ratio_low <= 1 <= s.ratio_high


def covers_interval(v: RatioInterval) -> bool:
    return v.low <= 1 <= v.high


def below(s: Stat) -> bool:
    return s.ratio_high < 1


def ratio_text(s: Stat) -> str:
    return f"{s.ratio:.3f} [{s.ratio_low:.3f}, {s.ratio_high:.3f}], {s.rows:,} rows"


def bucket_label(b: Bucket) -> str:
    return "0" if b.high_edge <= ZERO_EDGE else f"{b.low_edge:.2f}-{min(b.high_edge, 1):.2f}"


# ---------------------------------------------------------------- values
# the value names of a test and the Stat field each reads
STAT: dict[str, str] = {
    "rows": "rows",
    "predicted": "predicted",
    "observed": "observed",
    "obs / pred": "ratio",
    "ratio 95% low": "ratio_low",
    "ratio 95% high": "ratio_high",
}
RATIO: tuple[str, ...] = ("obs / pred", "ratio 95% low", "ratio 95% high")
TABLE: tuple[str, ...] = ("rows", "predicted", *RATIO)  # the columns of the document's test table
# the single-source (card, target) pairs the document's table lists
SINGLE: list[tuple[str, int]] = [
    ("[This Might Sting!] Sasami Anshinzawa", 20203),
    ("[Planned Perfection] Riko Kashimoto", 20046),
    ("[Tonight, We Waltz] King Halo", 20159),
    ("[Cute + Cute = ?] Mayano Top Gun", 20049),
    ("[Let's Get This Party Lit!] Daitaku Helios", 20165),
    ("[Sunlit Angel ♪] Daitaku Helios", 20165),
    ("[Step! Smile! Wink!] Tokai Teio", 20049),
    ("[Dancing Light into the Night] Yukino Bijin", 20166),
]
# the three event sources of Playtime's Over! (the skill on the chain-ending choice 2)
PO_CARDS: tuple[str, ...] = (
    "[Fiery Discipline] Yaeno Muteki",
    "[Dancing Light into the Night] Yukino Bijin",
    "[Escorte Étoile] Sirius Symboli",
)
PLAYTIMES_OVER: int = 20166
RACING_SPIRIT_STAMINA: int = 21010
NO_DECK_SOURCE: set[int] = {RACING_SPIRIT_STAMINA}  # no deck card can hint it: no deck test per target
TAIL_HELD_HIGH: int = 20161
# the target the per-roll and by-carrier inheritance tests leave out, and report on their own: the one target of eight
# whose own interval excludes 1. The heterogeneity test (heterogeneity_chi2) reads whether the targets differ at all;
# where they do not, leaving it out is a selection, which the findings say
INHERITANCE_EXCEPTION: int = TAIL_HELD_HIGH
INHERITANCE_INTERVAL_QUOTED: set[int] = {TAIL_HELD_HIGH}  # the per-target intervals the document quotes
ACQUIRED_INTERVAL_QUOTED: set[int] = {20046, 20203}
LARGE_PAIR: int = 40  # rows a single-source pair needs for the document's table


def test_values(name: str, s: Stat, fields: Iterable[str] = tuple(STAT), note: str = "") -> list[Value]:
    """the named values of one test (rows, predicted, observed, obs / pred, ratio 95% low / high), from its stat"""
    return [Value(S13, f"{name}: {f}", getattr(s, STAT[f]), note=note) for f in fields]


@dataclass
class Calibration:
    rows: list[CalRow]
    runs: list[RunRows]
    report: Report  # the tables go here (report8.txt)
    skill: dict[int, tuple[str, str | None]]  # group -> (white name, gold name)
    records: list[Run] | None  # the run records (their priority lists)
    card_id: dict[str, int]  # web_cards.json: card name -> id
    card_name: dict[int, str]  # id -> card name
    legs: dict[tuple[int, int], list[Leg]]  # (run, target) -> the legacies carrying its spark, as the row's plegs


class Leg(NamedTuple):
    """one legacy carrying a target's spark: its generation, base b (3% per star) and affinity A"""

    parent: bool
    base: float
    affinity: float


def leg_details(res: dict[str, Any]) -> dict[tuple[int, int], list[Leg]]:
    """(run, target) -> each legacy carrying the target's spark, in the order of the row's plegs (common.run_rows: the
    career's slots with a star of it), from the solver's slots. Should move to common.py beside run_rows."""
    out: dict[tuple[int, int], list[Leg]] = {}
    for fid, o in res.items():
        if not o.get("results"):
            continue
        career = o["results"][0]
        for k, t in enumerate(career["targets"]):
            out[int(fid.split("_")[0]), t["group"]] = [
                Leg(s["kind"] == "parent", 0.03 * s["stars"][k], s["aff"]) for s in career["slots"] if s["stars"][k] > 0
            ]
    return out


def calibration(R: list[Run] | None, M: Model, why: str | None) -> Calibration:
    """the default-target jobs solved and read into rows; without data (why) no rows"""
    bad: list[int]
    drop: dict[str, int]
    rows: list[CalRow]
    runrows: list[RunRows]
    badrows: dict[str, int]
    res: dict[str, Any] = {}
    if why:  # nothing computed: only the findings, as "needs data"
        bad, drop, rows, runrows, badrows = [], {}, [], [], {}
    else:
        os.makedirs(OUTD, exist_ok=True)
        bad = card_variants()
        jobdir = os.path.join(OUTD, "jobs8")
        man, files, drop = build_jobs(R, M, False, jobdir)
        res = run_solver(jobdir, files, os.path.join(OUTD, "res8.jsonl"))
        rows, runrows, badrows = rows_of(R, M, res, man)
    rep = Report(rows)
    rep.line(
        f"card variants: max-limit-break check {bad or 'all equal'}; dropped {dict(drop)}; problems {dict(badrows)}"
    )
    web_cards = common.gjson("web_cards.json")["cards"]
    return Calibration(
        rows,
        runrows,
        rep,
        common.skills(),
        R,
        card_id={c["name"]: c["id"] for c in web_cards},
        card_name={c["id"]: c["name"] for c in web_cards},
        legs=leg_details(res),
    )


def run_counts(cal: Calibration, out: list[Value]) -> None:
    rows = cal.rows
    by_scenario = collections.Counter(x["scen"] for x in cal.runs)
    out += [
        Value(S13, "runs used", len(cal.runs)),
        Value(S13, "URA Finale runs", by_scenario[1]),
        Value(S13, "Unity Cup runs", by_scenario[2]),
        Value(S13, "Grand Concert runs", by_scenario[3]),
        Value(S13, "Trackblazer runs", by_scenario[4]),
        Value(S13, "run-target rows", len(rows)),
        Value(S13, "rows on her outfit", sum(1 for r in rows if r["outfit"])),
        Value(S13, "scenario-gold rows", sum(1 for r in rows if r["scen_gold"])),
    ]


@dataclass
class DeckTests:
    """test 1, deck hints on rows no inspiration touched"""

    overall: Stat
    pool_only: Stat  # training pool only
    events_prioritised: Stat  # events only, on runs showing a priority list: the target on it
    events_not_prioritised: Stat  # ... the target not on it
    events_no_list: Stat  # events only, runs showing no list


def deck_tests(cal: Calibration, out: list[Value]) -> DeckTests:
    rep = cal.report
    overall = deck_test(rep)
    out += test_values("1. deck hint, no inspiration", overall, TABLE)
    pool_only = deck_test(rep, kind_is("training pool only"))
    out += test_values("1. training pool only", pool_only, TABLE)
    out += test_values("1. events only", deck_test(rep, kind_is("events only")), TABLE)

    events_prioritised = deck_test(rep, kind_is("events only"), priority_is(True))
    events_not_prioritised = deck_test(rep, kind_is("events only"), priority_is(False))
    no_list = deck_test(rep, priority_is(None), kind_is("events only"))
    outranked = deck_test(rep, kind_is("events only"), lambda r: r["prio_outranked"])
    out += test_values("1. events only, prioritised", events_prioritised, TABLE)
    out += test_values("1. events only, not prioritised", events_not_prioritised, TABLE)
    out += [
        Value(S9, "event-only hints, prioritised targets: obs / pred", events_prioritised.ratio),
        Value(S9, "event-only hints, non-prioritised targets: obs / pred", events_not_prioritised.ratio),
        Value(S9, "event-only hints, outranked on a rival choice: rows", outranked.rows),
    ]
    return DeckTests(overall, pool_only, events_prioritised, events_not_prioritised, no_list)


class CountTest(NamedTuple):
    """an observed count against its expectation, z run-clustered"""

    observed: int
    expected: float
    z: float


class Chi2(NamedTuple):
    statistic: float
    buckets: int
    p: float


@dataclass
class InheritanceTests:
    """test 2, inspiration of the target, and section 3's checks on it"""

    overall: Stat  # every row
    without_exception: Stat  # every row but INHERITANCE_EXCEPTION's
    by_carriers: dict[int, Stat]  # without INHERITANCE_EXCEPTION, as are the two per-roll counts
    spark_years: CountTest  # years with an inspiration of the target
    ancestor_hits: CountTest  # inspiration hits
    buckets: list[Bucket]  # reliability by q (every row)
    misses: list[Bucket]  # buckets with a positive prediction outside their own interval
    joint: Chi2  # the buckets jointly
    targets_differ: Chi2  # the eight targets' ratios against one common ratio (heterogeneity_chi2)
    dependence: Dependence  # within a run, across targets
    # every target, Tail Held High included (leaving out the one target whose interval excludes 1 is a selection)
    every_spark_years: CountTest
    every_hits: CountTest
    by_carriers_every: dict[int, Stat]


def spark_year_odds(r: CalRow) -> float:
    """P(a year fires an inspiration of the target): a spark-year fires per roll with y = 1 - prod(1 - r_l)"""
    return 1 - math.prod(1 - x for x in r["plegs"])


def clustered_z(resid: list[float], rows: list[CalRow]) -> float:
    """z of a count's residuals (one per row), run-clustered: a run's targets share its lineage and years"""
    if not resid:
        return float("nan")
    x2, _, _ = common.cluster_chi2([(0, d) for d in resid], [r["run"] for r in rows])
    return math.copysign(math.sqrt(x2), sum(resid))


def reliability_chi2(
    rows: list[CalRow], pred: Callable[[CalRow], float] = inspiration_odds, obs: RowFn = inspired
) -> Chi2:
    """jointly over the buckets with a positive prediction (and some prediction strictly between 0 and 1): the
    run-clustered Wald chi2 of the buckets' observed - expected (a run's targets fall in several buckets), one degree of
    freedom per bucket, nothing fitted"""
    resid: list[tuple[int, float]] = []
    runs: list[int] = []
    for j, (a, b) in enumerate(pairwise(EDGES)):
        qs = [r for r in rows if a <= pred(r) < b]
        if b <= ZERO_EDGE or not any(0 < pred(r) < 1 for r in qs):
            continue
        resid += [(j, obs(r) - pred(r)) for r in qs]
        runs += [r["run"] for r in qs]
    return cluster_chi2_safe(resid, runs) if resid else Chi2(0.0, 0, float("nan"))


def cluster_chi2_safe(resid: list[tuple[Any, float]], run: Sequence[Any]) -> Chi2:
    """common.cluster_chi2 without the groups whose residuals are 0 in every run (no variance: e.g. a prediction of 1
    always met), which make its covariance singular; should move into common.cluster_chi2"""
    spread: collections.defaultdict[Any, float] = collections.defaultdict(float)
    for g, d in resid:
        spread[g] += d * d
    keep = [(i, (g, d)) for i, (g, d) in enumerate(resid) if spread[g] > 0]
    if not keep:
        return Chi2(0.0, 0, NAN)
    return Chi2(*common.cluster_chi2([x for _, x in keep], [run[i] for i, _ in keep]))


def heterogeneity_chi2(rows: list[CalRow], pred: Callable[[CalRow], float], obs: RowFn, group: RowFn) -> Chi2:
    """do the groups' observed / predicted ratios differ? Under one common ratio R (fitted: sum obs / sum pred), each
    group's residual O_g - R E_g has mean 0. Run-clustered Wald chi2 on k - 1 of the k groups (the k residuals sum to
    0), each run's contribution linearised for R's estimation (u_g = e_g - E_g / E x the run's total residual);
    k - 1 degrees of freedom"""
    from scipy.stats import chi2

    groups = sorted({group(r) for r in rows})
    if len(groups) < 2:
        return Chi2(0.0, 0, NAN)
    index = {g: i for i, g in enumerate(groups)}
    E = sum(pred(r) for r in rows)
    R = sum(1 for r in rows if obs(r)) / E
    E_g = np.zeros(len(groups))
    for r in rows:
        E_g[index[group(r)]] += pred(r)
    per_run: dict[int, np.ndarray] = collections.defaultdict(lambda: np.zeros(len(groups)))
    for r in rows:
        per_run[r["run"]][index[group(r)]] += (1 if obs(r) else 0) - R * pred(r)
    e = np.array(list(per_run.values()))
    u = e - np.outer(e.sum(1), E_g / E)
    C = len(u)
    total = u.sum(0)[:-1]
    V = C / (C - 1) * (u[:, :-1].T @ u[:, :-1])
    x2 = float(total @ np.linalg.solve(V, total))
    return Chi2(x2, len(groups) - 1, float(chi2.sf(x2, len(groups) - 1)))


class RollForm(NamedTuple):
    """an alternative inheritance roll scored on the careers (the model unchanged): a legacy's per-roll chance
    r = min(1, w b (1 + s A / 100)), the weight w on parents only, the slope s on every legacy's affinity A. Each form
    frees one parameter, fitted by maximum likelihood on section 3's own selection of single-ancestor trials (ordinary
    parents and grandparents: common.trials' categories P and G, as s03_inheritance.py fits them), with its
    run-clustered 95% interval (common.robust_interval)"""

    name: str  # "parent factor" (w free) or "affinity slope" (s free)
    weight: float
    slope: float
    estimate: float  # the free parameter
    low: float
    high: float
    trials: int


SECTION_3_TRIALS: tuple[str, ...] = ("P", "G")  # the categories s03_inheritance.py fits its parent weight and slope on


def roll_forms(R: list[Run] | None) -> list[RollForm]:
    """the flat parent factor and the affinity slope, each refitted (should move to common.py, so section 3 and this
    section share one estimator)"""
    from scipy.optimize import minimize_scalar

    ts = [t for t in common.trials(R) if t.category in SECTION_3_TRIALS] if R else []
    if not ts:
        return [RollForm(name, NAN, NAN, NAN, NAN, NAN, 0) for name in ("parent factor", "affinity slope")]
    base = np.array([t.base for t in ts])
    affinity = np.array([t.affinity_h0 for t in ts], float)
    parent = np.array([t.category == "P" for t in ts])
    passed = np.array([t.passed for t in ts], float)
    _, run_ix = np.unique(np.array([t.run for t in ts]), return_inverse=True)

    def per_trial_ll(w: float, s: float) -> np.ndarray:
        p = np.clip(np.where(parent, w, 1.0) * base * (1 + s * affinity / 100), 1e-12, 1 - 1e-12)
        return passed * np.log(p) + (1 - passed) * np.log(1 - p)

    forms: list[RollForm] = []
    for name, roll in (("parent factor", lambda x: (x, 1.0)), ("affinity slope", lambda x: (1.0, x))):
        fit = minimize_scalar(
            lambda x, roll=roll: -per_trial_ll(*roll(x)).sum(),
            bounds=(0.5, 1.5),
            method="bounded",
            options={"xatol": 1e-7},
        )
        x = float(fit.x)
        lo, hi, _ = common.robust_interval(
            lambda t, roll=roll: dict(enumerate(np.bincount(run_ix, weights=per_trial_ll(*roll(t))))), x
        )
        forms.append(RollForm(name, *roll(x), x, lo, hi, len(ts)))
    return forms


def rolled_legs(legs: dict[tuple[int, int], list[Leg]], form: RollForm) -> Callable[[CalRow], list[float]]:
    """a row's per-roll inspiration chances under the form (the model's own plegs where its legacies are unknown)"""

    def chances(r: CalRow) -> list[float]:
        ls = legs.get((r["run"], r["g"]))
        if ls is None or len(ls) != len(r["plegs"]):
            return r["plegs"]
        return [min(1.0, (form.weight if x.parent else 1.0) * x.base * (1 + form.slope * x.affinity / 100)) for x in ls]

    return chances


def q_of_legs(legs: list[float]) -> float:
    """P(at least one inspiration of the target over the two rolls) from its legs' per-roll chances"""
    return 1 - (1 - (1 - math.prod(1 - x for x in legs))) ** 2


def not_the_exception(r: CalRow) -> bool:
    return r["g"] != INHERITANCE_EXCEPTION


def inheritance_tests(cal: Calibration, out: list[Value]) -> InheritanceTests:
    rep, rows = cal.report, cal.rows
    rest = [r for r in rows if not_the_exception(r)]
    overall = inheritance_test(rep)
    without_exception = inheritance_test(rep, not_the_exception)
    out += test_values("2. inheritance", overall, TABLE)
    out += [
        Value(S3, "runs tested", len(cal.runs)),
        Value(S3, "run-target rows", overall.rows),
        Value(S3, "inheritance obs / pred", overall.ratio),
        Value(S3, "inheritance ratio 95% low", overall.ratio_low),
        Value(S3, "inheritance ratio 95% high", overall.ratio_high),
        Value(S3, "inheritance without Tail Held High: rows", without_exception.rows),
        Value(S3, "inheritance without Tail Held High: obs / pred", without_exception.ratio),
        Value(S3, "inheritance without Tail Held High: ratio 95% low", without_exception.ratio_low),
        Value(S3, "inheritance without Tail Held High: ratio 95% high", without_exception.ratio_high),
    ]
    by_carriers = rep.table(
        "2. inheritance by carriers, without Tail Held High",
        not_the_exception,
        inspiration_odds,
        inspired,
        lambda r: r["nc"],
    )
    ratios = [by_carriers[k].ratio for k in range(1, 7) if k in by_carriers] or [float("nan")]
    out += [
        Value(S3, "by carriers 1-6, without Tail Held High: lowest ratio", min(ratios)),
        Value(S3, "by carriers 1-6, without Tail Held High: highest ratio", max(ratios)),
    ]
    # per roll: a spark-year fires with spark_year_odds, a legacy hits with r_l; two rolls (years 2 and 3)
    hits = CountTest(
        sum(r["o_insp_hits"] for r in rest),
        sum(2 * sum(r["plegs"]) for r in rest),
        clustered_z([r["o_insp_hits"] - 2 * sum(r["plegs"]) for r in rest], rest),
    )
    years = CountTest(
        sum(r["o_insp_years"] for r in rest),
        sum(2 * spark_year_odds(r) for r in rest),
        clustered_z([r["o_insp_years"] - 2 * spark_year_odds(r) for r in rest], rest),
    )
    out += [
        Value(
            S3, "per roll, without Tail Held High: spark-years obs / exp", common.div(years.observed, years.expected)
        ),
        Value(S3, "per roll, without Tail Held High: spark-years, SDs below expectation", -years.z),
    ]
    buckets = rep.reliability("inheritance", every_row, inspiration_odds, inspired, EDGES)
    for b in buckets[:-1]:  # the document's table stops at 0.95
        lab = bucket_label(b)
        out += [
            Value(S3, f"reliability q {lab}: rows", b.rows),
            Value(S3, f"reliability q {lab}: observed", b.observed),
        ]
        if b.high_edge > ZERO_EDGE:
            out.append(Value(S3, f"reliability q {lab}: mean predicted", b.predicted))
    misses = [
        b
        for b in buckets
        if b.rows and b.high_edge > ZERO_EDGE and not b.observed_low <= b.predicted <= b.observed_high
    ]
    joint = reliability_chi2(rows)
    out.append(
        Value(S3, "reliability buckets outside their own 95% interval", ", ".join(map(bucket_label, misses)) or "none")
    )
    out += [
        Value(S3, "reliability chi2", joint.statistic),
        Value(S3, "reliability chi2 buckets", joint.buckets),
        Value(S3, "reliability chi2 p", joint.p),
    ]
    middle = next((b for b in buckets if b.low_edge == 0.4 and b.rows), NO_BUCKET)
    out += [
        Value(S3, "reliability q 0.40-0.60: observed 95% low", middle.observed_low),
        Value(S3, "reliability q 0.40-0.60: observed 95% high", middle.observed_high),
        Value(S3, "reliability q 0.40-0.60: mean predicted, three places", middle.predicted),
    ]
    targets_differ = heterogeneity_chi2(rows, inspiration_odds, inspired, lambda r: r["g"])
    dependence = within_run_dependence(cal.runs, every_row, inspiration_odds, inspired)
    every_years, every_hits = per_roll_counts(rows, lambda r: r["plegs"])
    by_carriers_every = rep.table(
        "2. inheritance by carriers, every target", every_row, inspiration_odds, inspired, lambda r: r["nc"]
    )
    every_ratios = [by_carriers_every[k].ratio for k in range(1, 7) if k in by_carriers_every] or [NAN]
    out += [
        Value(
            S13,
            "2. inheritance, every target: spark-years obs / exp",
            common.div(every_years.observed, every_years.expected),
        ),
        Value(S13, "2. inheritance, every target: spark-years z", every_years.z),
        Value(
            S13, "2. inheritance, every target: hits obs / exp", common.div(every_hits.observed, every_hits.expected)
        ),
        Value(S13, "2. inheritance, every target: hits z", every_hits.z),
        Value(S13, "2. inheritance, every target, by carriers 1-6: lowest ratio", min(every_ratios)),
        Value(S13, "2. inheritance, every target, by carriers 1-6: highest ratio", max(every_ratios)),
        Value(
            S13,
            "2. inheritance, every target, by carriers 1-6: intervals excluding 1",
            sum(1 for k in range(1, 7) if k in by_carriers_every and not covers(by_carriers_every[k])),
        ),
        Value(S13, "2. inheritance, within-run dependence: mean product", dependence.mean_product),
        Value(S13, "2. inheritance, within-run dependence: 95% half-width", dependence.half_width),
    ]
    out += [
        Value(S13, "2. inheritance, targets differ: chi2", targets_differ.statistic),
        Value(S13, "2. inheritance, targets differ: degrees of freedom", targets_differ.buckets),
        Value(S13, "2. inheritance, targets differ: p", targets_differ.p),
        Value(S13, "2. inheritance, within-run dependence: target pairs", dependence.pairs),
        Value(S13, "2. inheritance, within-run dependence: z", dependence.z),
    ]
    return InheritanceTests(
        overall,
        without_exception,
        by_carriers,
        years,
        hits,
        buckets,
        misses,
        joint,
        targets_differ,
        dependence,
        every_years,
        every_hits,
        by_carriers_every,
    )


class Dependence(NamedTuple):
    """do two targets of one run miss or beat their predictions together? The sum over runs of every pair of the run's
    targets' residual products, sum_{i<j} (o_i - p_i)(o_j - p_j); with the targets independent given the model inputs
    and each prediction right, its mean is 0. z: that sum over its run-clustered standard error"""

    runs: int
    pairs: int
    mean_product: float
    z: float
    half_width: float  # of the mean product's 95% interval: how large a residual covariance the test could miss


def within_run_dependence(runs: list[RunRows], sel: RowFn, pred: Callable[[CalRow], float], obs: RowFn) -> Dependence:
    total = square = 0.0
    pairs = used = 0
    for pr in runs:
        e = [(1 if obs(r) else 0) - pred(r) for r in pr["rows"] if r["g"] in DEFAULT and sel(r)]
        if len(e) < 2:
            continue
        s = (sum(e) ** 2 - sum(x * x for x in e)) / 2
        total += s
        square += s * s
        pairs += len(e) * (len(e) - 1) // 2
        used += 1
    return Dependence(
        used,
        pairs,
        common.div(total, pairs),
        common.div(total, math.sqrt(square)),
        1.96 * common.div(math.sqrt(square), pairs),
    )


def per_roll_counts(rows: list[CalRow], legs: Callable[[CalRow], list[float]]) -> tuple[CountTest, CountTest]:
    """years with an inspiration of the target and inspiration hits, observed against the legs' expectation over the two
    rolls (a year fires with 1 - prod(1 - r_l), a legacy hits with r_l), z run-clustered"""

    def year_odds(r: CalRow) -> float:
        return 1 - math.prod(1 - x for x in legs(r))

    years = CountTest(
        sum(r["o_insp_years"] for r in rows),
        sum(2 * year_odds(r) for r in rows),
        clustered_z([r["o_insp_years"] - 2 * year_odds(r) for r in rows], rows),
    )
    hits = CountTest(
        sum(r["o_insp_hits"] for r in rows),
        sum(2 * sum(legs(r)) for r in rows),
        clustered_z([r["o_insp_hits"] - 2 * sum(legs(r)) for r in rows], rows),
    )
    return years, hits


def several_carriers(r: CalRow) -> bool:
    """two or more legacies carry the target: rows that are not single-ancestor trials, the forms' fitting data"""
    return r["nc"] >= 2


@dataclass
class RollFormTests:
    """section 3's misses rescored under an alternative roll (RollForm; the model unchanged)"""

    form: RollForm
    q_rebuilt_error: float  # largest |q rebuilt from the row's legs at w = s = 1 - the solver's q|
    spark_years: CountTest  # every target
    ancestor_hits: CountTest
    spark_years_without_exception: CountTest
    overall: Stat
    without_exception: Stat
    exception: Stat
    several: Stat  # rows with two or more carriers
    several_model: Stat  # the same rows under the model
    middle: Bucket  # the 0.40-0.60 band, by the rescored q
    joint: Chi2  # the reliability buckets by the rescored q
    targets_differ: Chi2


def roll_form_tests(cal: Calibration, form: RollForm, out: list[Value]) -> RollFormTests:
    rows, rep = cal.rows, cal.report
    legs = rolled_legs(cal.legs, form)
    q_scaled = {id(r): q_of_legs(legs(r)) for r in rows}

    def q(r: CalRow) -> float:
        return q_scaled[id(r)]

    model = RollForm("model", 1.0, 1.0, 1.0, NAN, NAN, 0)
    rebuilt = max((abs(q_of_legs(rolled_legs(cal.legs, model)(r)) - r["q"]) for r in rows), default=NAN)
    years, hits = per_roll_counts(rows, legs)
    years_rest, _ = per_roll_counts([r for r in rows if not_the_exception(r)], legs)
    overall = rep.stat(every_row, q, inspired)
    without_exception = rep.stat(not_the_exception, q, inspired)
    exception = rep.stat(target_is(INHERITANCE_EXCEPTION), q, inspired)
    several = rep.stat(several_carriers, q, inspired)
    several_model = rep.stat(several_carriers, inspiration_odds, inspired)
    middle = row_ci([r for r in rows if 0.4 <= q(r) < 0.6], q, inspired)
    band = Bucket(0.4, 0.6, middle.rows, middle.observed, middle.predicted, middle.low, middle.high)
    joint = reliability_chi2(rows, q, inspired)
    differ = heterogeneity_chi2(rows, q, inspired, lambda r: r["g"])
    pre = form.name
    parameter = "weight" if form.name == "parent factor" else "slope"
    out += [
        Value(S13, f"{pre}: {parameter}", form.estimate, note="refitted on section 3's own selection of trials"),
        Value(S13, f"{pre}: 95% low", form.low),
        Value(S13, f"{pre}: 95% high", form.high),
        Value(S13, f"{pre}: trials", form.trials),
        Value(S13, f"{pre}: q rebuilt from the legs, largest error", rebuilt),
        Value(S13, f"{pre}: spark-years obs / exp", common.div(years_rest.observed, years_rest.expected)),
        Value(S13, f"{pre}: spark-years z", years_rest.z),
        Value(S13, f"{pre}: every target, spark-years obs / exp", common.div(years.observed, years.expected)),
        Value(S13, f"{pre}: every target, spark-years z", years.z),
        Value(S13, f"{pre}: every target, hits obs / exp", common.div(hits.observed, hits.expected)),
        Value(S13, f"{pre}: every target, hits z", hits.z),
        *test_values(f"{pre}: inheritance", overall, RATIO),
        *test_values(f"{pre}: inheritance without Tail Held High", without_exception, RATIO),
        *test_values(f"{pre}: Tail Held High", exception, RATIO),
        *test_values(f"{pre}: two or more carriers", several, ("rows", *RATIO)),
        Value(S13, f"{pre}: q 0.40-0.60 observed", band.observed),
        Value(S13, f"{pre}: q 0.40-0.60 mean predicted", band.predicted),
        Value(S13, f"{pre}: q 0.40-0.60 observed 95% low", band.observed_low),
        Value(S13, f"{pre}: q 0.40-0.60 observed 95% high", band.observed_high),
        Value(S13, f"{pre}: reliability chi2 p", joint.p),
        Value(S13, f"{pre}: targets differ p", differ.p),
    ]
    if form.name == "parent factor":  # the model on the same rows, once
        out += test_values("2. inheritance, two or more carriers", several_model, ("rows", *RATIO))
    return RollFormTests(
        form,
        rebuilt,
        years,
        hits,
        years_rest,
        overall,
        without_exception,
        exception,
        several,
        several_model,
        band,
        joint,
        differ,
    )


@dataclass
class AcquisitionTests:
    """tests 3 (acquired) and 4 (landing) on open targets"""

    any_source: Stat
    modelled_source: Stat  # acquired through a deck hint or an inspiration
    # the same two on the rows the model's priority assumption fits: the target prioritised, or the run shows no list
    any_source_assumed: Stat
    modelled_source_assumed: Stat
    prioritised: Stat
    not_prioritised: Stat
    unknown_priority: Stat
    unexplained: list[CalRow]  # acquired with no modelled source
    unexplained_events_only: int  # of them, credited to events only


def acquisition_tests(cal: Calibration, out: list[Value]) -> AcquisitionTests:
    rep = cal.report
    any_source = acquisition_test(rep)
    modelled_source = rep.stat(is_open, acquisition_odds, acquired_modelled)
    out += test_values("3. acquired, any source", any_source, TABLE)

    def priority_assumed(r: CalRow) -> bool:
        return prioritised(r) is not False

    any_source_assumed = acquisition_test(rep, priority_assumed)
    modelled_source_assumed = rep.stat(where(is_open, priority_assumed), acquisition_odds, acquired_modelled)
    out += test_values("3. acquired, any source, prioritised or no list shown", any_source_assumed, TABLE)
    out += test_values(
        "3. acquired, deck or inspiration only, prioritised or no list shown", modelled_source_assumed, ("obs / pred",)
    )
    shortfall = sum(acquisition_odds(r) - acquired(r) for r in cal.rows if is_open(r))
    shortfall_not_prioritised = sum(
        acquisition_odds(r) - acquired(r) for r in cal.rows if is_open(r) and prioritised(r) is False
    )
    out.append(
        Value(
            S13,
            "3. acquired: shortfall share on non-prioritised rows",
            common.div(shortfall_not_prioritised, shortfall),
        )
    )
    prioritised_rows = acquisition_test(rep, priority_is(True))
    out += test_values("3. acquired, prioritised", prioritised_rows, TABLE)
    not_prioritised = acquisition_test(rep, priority_is(False))
    out += test_values("3. acquired, not prioritised", not_prioritised, TABLE)
    unknown_priority = acquisition_test(rep, priority_is(None))
    out += test_values("3. acquired, unknown priority", unknown_priority, TABLE)
    open_rows = [r for r in cal.rows if is_open(r)]
    predicted, observed = sum(r["p_pred"] for r in open_rows), sum(r["l_obs"] for r in open_rows)
    out.append(Value(S13, "4. landing: ratio", common.div(observed, predicted)))
    unexplained = [r for r in open_rows if r["o_any"] and not acquired_modelled(r)]
    events_only = sum(1 for r in unexplained if r["o_events"] and not r["o_other"])
    out += [
        Value(S13, "acquisitions from unmodelled events", events_only),
        Value(S13, "acquisitions from unmodelled events, share of rows", common.div(events_only, len(open_rows))),
    ]
    return AcquisitionTests(
        any_source,
        modelled_source,
        any_source_assumed,
        modelled_source_assumed,
        prioritised_rows,
        not_prioritised,
        unknown_priority,
        unexplained,
        events_only,
    )


def per_target_tests(cal: Calibration, out: list[Value]) -> tuple[dict[int, Stat], dict[int, Stat]]:
    """per default target: the deck test where a deck card can hint the target, inheritance and acquired (the intervals
    the document quotes); returns the inheritance and the acquired tests"""
    rep = cal.report
    inheritance: dict[int, Stat] = {}
    acquired: dict[int, Stat] = {}
    for g in DEFAULT:
        nm = cal.skill[g][0]
        if g not in NO_DECK_SOURCE:
            s = deck_test(rep, target_is(g))
            out += [
                Value(S13, f"{nm}: deck hint", s.ratio),
                Value(S13, f"{nm}: deck hint 95% low", s.ratio_low),
                Value(S13, f"{nm}: deck hint 95% high", s.ratio_high),
            ]
        s = inheritance[g] = inheritance_test(rep, target_is(g))
        out.append(Value(S13, f"{nm}: inheritance", s.ratio))
        if g in INHERITANCE_INTERVAL_QUOTED:
            out += [
                Value(S13, f"{nm}: inheritance 95% low", s.ratio_low),
                Value(S13, f"{nm}: inheritance 95% high", s.ratio_high),
            ]
        s = acquired[g] = acquisition_test(rep, target_is(g))
        out.append(Value(S13, f"{nm}: acquired", s.ratio))
        if g in ACQUIRED_INTERVAL_QUOTED:
            out += [
                Value(S13, f"{nm}: acquired 95% low", s.ratio_low),
                Value(S13, f"{nm}: acquired 95% high", s.ratio_high),
            ]
    return inheritance, acquired


@dataclass
class PoolTests:
    """test 1 on targets only a training pool can hint"""

    by_scenario: dict[int, Stat]
    low_limit_breaks: list[Stat]  # by the pool's lowest limit break: 0, 1, 2


def pool_tests(cal: Calibration, out: list[Value]) -> PoolTests:
    rep = cal.report
    pool_only = kind_is("training pool only")
    by_scenario: dict[int, Stat] = {}
    for scn in (3, 2, 1, 4):
        by_scenario[scn] = deck_test(rep, pool_only, scenario_is(scn))
        out += test_values(f"1. training pool only, {common.SCEN[scn]}", by_scenario[scn], RATIO)
    low = [deck_test(rep, pool_only, lowest_lb_is(lb)) for lb in (0, 1, 2)]
    for lb, st in zip((0, 1, 2), low):  # outside section 7's fit, which reads max-limit-break cards only
        out += test_values(f"1. training pool only, lowest limit break {lb}", st, TABLE)
    return PoolTests(by_scenario, low)


class UnityGolds(NamedTuple):
    runs: int  # Unity Cup runs
    with_both: int  # of them, both guaranteed golds hinted


@dataclass
class GoldTests:
    """Unity Cup's guaranteed golds, and the acquisition of every target's gold on open rows the deck can give it"""

    unity: UnityGolds
    by_gold: dict[int, Stat]  # target group -> its gold's test
    pooled: Stat  # every gold together
    joint: Chi2  # every gold calibrated: run-clustered Wald chi2 of each gold's observed - predicted, nothing fitted
    uncertain: Stat  # the rows whose gold is not predicted certain (See Ya Later!'s certain rows left out)
    uncertain_by_priority: dict[str, Stat]  # ... per priority status (PRIORITY_STATUS)


def gold_row(r: CalRow) -> bool:
    """an open target whose gold the deck can give (it is not the scenario's: Unity Cup's two are is_open's
    scenario golds)"""
    return is_open(r) and r["gold"] > 0


def gold_odds(r: CalRow) -> float:
    return r["gold"]


def gold_hinted(r: CalRow) -> bool:
    return r["o_gold"]


def gold_tests(cal: Calibration, out: list[Value]) -> GoldTests:
    unity = [x for x in cal.runs if x["scen"] == 2]
    golds = UnityGolds(len(unity), sum(1 for x in unity if all(r["o_gold"] for r in x["rows"] if r["scen_gold"])))
    out.append(Value(S13, "Unity Cup's guaranteed golds: runs with both", golds.with_both))
    by_gold: dict[int, Stat] = {}
    for g in DEFAULT:
        nm = cal.skill[g][1]
        if nm is None or (cal.rows and not any(gold_row(r) and r["g"] == g for r in cal.rows)):  # no deck gives it
            continue
        s = by_gold[g] = cal.report.stat(where(gold_row, target_is(g)), gold_odds, gold_hinted)
        out += [
            Value(S13, f"gold acquisition {nm}", s.ratio),
            Value(S13, f"gold acquisition {nm}: rows", s.rows),
            Value(S13, f"gold acquisition {nm}: 95% low", s.ratio_low),
            Value(S13, f"gold acquisition {nm}: 95% high", s.ratio_high),
        ]
    pooled = cal.report.stat(gold_row, gold_odds, gold_hinted)
    # the golds the joint test can read: some row with a prediction strictly between 0 and 1 (a gold predicted certain
    # everywhere has no variance and is left out)
    uncertain = {r["g"] for r in cal.rows if gold_row(r) and r["gold"] < 1}
    rows = [r for r in cal.rows if gold_row(r) and r["g"] in uncertain]
    joint = (
        Chi2(
            *cluster_chi2_safe(
                [(r["g"], (1 if r["o_gold"] else 0) - r["gold"]) for r in rows], [r["run"] for r in rows]
            )
        )
        if rows
        else Chi2(0.0, 0, NAN)
    )
    out += [
        *test_values("gold acquisition, every gold", pooled, TABLE),
        Value(S13, "gold acquisition, every gold calibrated: chi2", joint.statistic),
        Value(S13, "gold acquisition, every gold calibrated: golds", joint.buckets),
        Value(S13, "gold acquisition, every gold calibrated: p", joint.p),
    ]

    def uncertain_gold(r: CalRow) -> bool:
        return gold_row(r) and r["gold"] < 1

    uncertain = cal.report.stat(uncertain_gold, gold_odds, gold_hinted)
    by_priority = {
        status: cal.report.stat(where(uncertain_gold, priority_is(prio)), gold_odds, gold_hinted)
        for status, prio in PRIORITY_STATUS
    }
    out += test_values("gold acquisition, uncertain golds", uncertain, TABLE)
    for status, st in by_priority.items():
        out += test_values(f"gold acquisition, uncertain golds, {status}", st, TABLE)
    return GoldTests(golds, by_gold, pooled, joint, uncertain, by_priority)


@dataclass
class DeckReliability:
    """deck shortfalls by predicted chance, and acquisition reliability under the report approximation"""

    acquisition: list[Bucket]
    deck_tested: list[Bucket]  # deck buckets with rows and a positive prediction
    shortfall: float  # over every deck-test row
    shortfall_events: float  # rows with an event source
    shortfall_top: float  # rows predicted 0.6 and above
    top_rows: int
    top_rows_with_event: int
    by_priority: dict[str, list[Bucket]]  # acquisition reliability per priority status (PRIORITY_STATUS)
    by_priority_joint: dict[str, Chi2]  # ... its positive buckets jointly (reliability_chi2)
    # the same counting only modelled sources (a deck card's hint or an inspiration): the model's own claim
    modelled_by_priority: dict[str, list[Bucket]]
    modelled_by_priority_joint: dict[str, Chi2]
    likely: dict[str, Stat]  # rows predicted 0.6 or more, any source, per priority status
    likely_modelled: dict[str, Stat]  # ... modelled sources only
    unlikely_modelled: dict[str, Stat]  # rows predicted above 0 and under 0.6, modelled sources only
    mixture_share: float  # the prioritised share that mixes prioritised and not-prioritised rows into the unknown ratio


PRIORITY_STATUS: tuple[tuple[str, bool | None], ...] = (
    ("unknown priority", None),
    ("prioritised", True),
    ("not prioritised", False),
)


def deck_shortfall(rows: list[CalRow], sel: RowFn) -> float:
    return sum(deck_odds(r) - r["o_deck"] for r in rows if in_deck_test(r) and sel(r))


def reliability_tests(cal: Calibration, out: list[Value]) -> DeckReliability:
    rep, rows = cal.report, cal.rows
    deck = rep.reliability("deck hint", in_deck_test, deck_odds, deck_hinted, EDGES)
    # Acquisition uses the report's product approximation on open targets, including its zero bucket.
    acquisition = rep.reliability("acquired", is_open, acquisition_odds, acquired, EDGES)
    for bucket in acquisition:
        label = bucket_label(bucket)
        out += [
            Value(S13, f"reliability acquired {label}: rows", bucket.rows),
            Value(S13, f"reliability acquired {label}: observed", bucket.observed),
        ]
        if bucket.high_edge > ZERO_EDGE:
            out.append(Value(S13, f"reliability acquired {label}: mean predicted", bucket.predicted))

    # the deck's shortfall in its buckets predicted 0.6 and above, and how many of their rows have an event source
    def top(r: CalRow) -> bool:
        return deck_odds(r) >= 0.6

    top_rows = [r for r in rows if in_deck_test(r) and top(r)]
    by_priority: dict[str, list[Bucket]] = {}
    by_priority_joint: dict[str, Chi2] = {}
    for status, prio in PRIORITY_STATUS:
        sel = where(is_open, priority_is(prio))
        by_priority[status] = rep.reliability(f"acquired, {status}", sel, acquisition_odds, acquired, EDGES)
        joint = by_priority_joint[status] = reliability_chi2([r for r in rows if sel(r)], acquisition_odds, acquired)
        for bucket in by_priority[status]:
            if bucket.high_edge <= ZERO_EDGE:
                continue
            label = bucket_label(bucket)
            out += [
                Value(S13, f"reliability acquired, {status}, {label}: rows", bucket.rows),
                Value(S13, f"reliability acquired, {status}, {label}: observed", bucket.observed),
                Value(S13, f"reliability acquired, {status}, {label}: mean predicted", bucket.predicted),
            ]
        out += [
            Value(S13, f"reliability acquired, {status}: chi2", joint.statistic),
            Value(S13, f"reliability acquired, {status}: buckets", joint.buckets),
            Value(S13, f"reliability acquired, {status}: p", joint.p),
        ]
    modelled_by_priority: dict[str, list[Bucket]] = {}
    modelled_by_priority_joint: dict[str, Chi2] = {}
    likely: dict[str, Stat] = {}
    likely_modelled: dict[str, Stat] = {}
    unlikely_modelled: dict[str, Stat] = {}
    for status, prio in PRIORITY_STATUS:
        sel = where(is_open, priority_is(prio))
        buckets = modelled_by_priority[status] = rep.reliability(
            f"acquired from a modelled source, {status}", sel, acquisition_odds, acquired_modelled, EDGES
        )
        joint = modelled_by_priority_joint[status] = reliability_chi2(
            [r for r in rows if sel(r)], acquisition_odds, acquired_modelled
        )
        for bucket in buckets:
            if bucket.high_edge > ZERO_EDGE:
                out.append(
                    Value(S13, f"reliability modelled, {status}, {bucket_label(bucket)}: observed", bucket.observed)
                )
        likely[status] = rep.stat(where(sel, lambda r: acquisition_odds(r) >= 0.6), acquisition_odds, acquired)
        likely_modelled[status] = rep.stat(
            where(sel, lambda r: acquisition_odds(r) >= 0.6), acquisition_odds, acquired_modelled
        )
        unlikely_modelled[status] = rep.stat(
            where(sel, lambda r: 0 < acquisition_odds(r) < 0.6), acquisition_odds, acquired_modelled
        )
        out += [
            Value(S13, f"reliability modelled, {status}: chi2", joint.statistic),
            Value(S13, f"reliability modelled, {status}: p", joint.p),
            *test_values(f"acquired, {status}, predicted 0.6 and above", likely[status], RATIO),
            *test_values(
                f"acquired from a modelled source, {status}, predicted 0.6 and above", likely_modelled[status], RATIO
            ),
            *test_values(
                f"acquired from a modelled source, {status}, predicted under 0.6", unlikely_modelled[status], RATIO
            ),
        ]
    # inference, not a test: the share w of prioritised rows with w x prioritised + (1 - w) x not prioritised equal to
    # the unknown-priority ratio, on rows predicted 0.6 and above
    mixture = common.div(
        likely["unknown priority"].ratio - likely["not prioritised"].ratio,
        likely["prioritised"].ratio - likely["not prioritised"].ratio,
    )
    out.append(Value(S13, "unknown priority, predicted 0.6 and above: prioritised share of a mixture", mixture))
    # residual dependence between a run's targets on the deck and acquisition tests: reported, not tested, since the
    # model itself makes these targets dependent (shared chain events, one event's exclusive outcomes)
    for name, sel, pred, obs in (
        ("deck hint", in_deck_test, deck_odds, deck_hinted),
        ("acquired", is_open, acquisition_odds, acquired),
    ):
        dep = within_run_dependence(cal.runs, sel, pred, obs)
        rep.line(f"\n## Within-run residual dependence, {name}: {dep.pairs} pairs over {dep.runs} runs, z {dep.z:+.2f}")
        out.append(Value(S13, f"{name}, within-run dependence: z", dep.z))
    rel = DeckReliability(
        acquisition,
        [b for b in deck if b.rows and b.high_edge > ZERO_EDGE],
        shortfall=deck_shortfall(rows, every_row),
        shortfall_events=deck_shortfall(rows, lambda r: source_kind(r) in ("events only", "pool+event")),
        shortfall_top=deck_shortfall(rows, top),
        top_rows=len(top_rows),
        top_rows_with_event=sum(1 for r in top_rows if r["cards_ev"]),
        by_priority=by_priority,
        by_priority_joint=by_priority_joint,
        modelled_by_priority=modelled_by_priority,
        modelled_by_priority_joint=modelled_by_priority_joint,
        likely=likely,
        likely_modelled=likely_modelled,
        unlikely_modelled=unlikely_modelled,
        mixture_share=mixture,
    )
    out += [
        Value(S13, "deck shortfall in buckets predicted 0.6 and above", common.div(rel.shortfall_top, rel.shortfall)),
        Value(S13, "their rows with an event source", common.div(rel.top_rows_with_event, rel.top_rows)),
    ]
    return rel


@dataclass
class RunOdds:
    """one run's default targets: the open ones acquired, and landing every target"""

    all_acquired_odds: float  # the product of the open targets' acquisition odds
    all_acquired: bool
    p_all: float  # the solver's P(every target lands)
    landing_observed: float  # the product of the observed-equivalent landings
    priority_assumed: bool  # no open target shown un-prioritised (each prioritised, or the run shows no list)
    list_shown: bool  # the run shows a priority list
    # the same product under the career report's approximation (per target: gold spark chance x P(gold) + white spark
    # chance x P(white only)), counted by tier, and counted tier-blind (every acquisition scored as a white, on both sides)
    tiered_predicted: float
    blind_observed: float
    blind_predicted: float


def run_odds(pr: RunRows, list_shown: bool) -> RunOdds:
    targets = [r for r in pr["rows"] if r["g"] in DEFAULT]
    opened = [r for r in targets if is_open(r)]

    def carriers(r: CalRow) -> float:
        return 1.1 ** (r["nc"] - 6)

    def tiered(r: CalRow) -> float:
        g, w = common.SPARK_GOLD_6 * carriers(r), common.SPARK_WHITE_6 * carriers(r)
        if r["scen_gold"] or r["outfit"] == 2:
            return g
        if r["outfit"] == 1:  # her outfit gives the white: the gold or else the white
            return g * r["gold"] + w * (1 - r["gold"])
        return g * r["gold"] + w * (r["acq"] - r["gold"])

    def certain(r: CalRow) -> bool:
        return bool(r["outfit"]) or r["scen_gold"]

    return RunOdds(
        math.prod(r["acq"] for r in opened),
        all(r["o_any"] for r in opened),
        pr["p_all"],
        math.prod(r["l_obs"] for r in targets),
        all(prioritised(r) is not False for r in opened),
        list_shown,
        math.prod(tiered(r) for r in targets),
        math.prod(common.SPARK_WHITE_6 * carriers(r) * (r["o_any"] or certain(r)) for r in targets),
        math.prod(common.SPARK_WHITE_6 * carriers(r) * (1.0 if certain(r) else r["acq"]) for r in targets),
    )


class RatioInterval(NamedTuple):
    ratio: float
    low: float
    high: float


def ratio_interval(
    xs: Sequence[RunOdds], obs: Callable[[RunOdds], float], pred: Callable[[RunOdds], float]
) -> RatioInterval:
    """sum obs / sum pred over runs, its 95% interval bootstrapping whole runs (seeded)"""
    if not xs:
        return RatioInterval(NAN, NAN, NAN)
    o, p = [obs(x) for x in xs], [pred(x) for x in xs]
    return RatioInterval(common.div(sum(o), sum(p)), *percentile_interval(ratio_draws(o, p, seed=1)))


class AcquiredPerRun(NamedTuple):
    """runs acquiring every open target over a set of runs (Wilson 95%), against the model's mean product of odds"""

    runs: int
    all_acquired_rate: float
    all_acquired_low: float
    all_acquired_high: float
    all_acquired_odds_mean: float  # the mean product of the model's odds

    def all_acquired_covered(self) -> bool:
        return self.all_acquired_low <= self.all_acquired_odds_mean <= self.all_acquired_high

    def text(self) -> str:
        return (
            f"all open: {self.all_acquired_rate:.4f} [{self.all_acquired_low:.4f}, {self.all_acquired_high:.4f}] "
            f"vs {self.all_acquired_odds_mean:.4f}; {self.runs:,} runs"
        )


def acquired_per_run(runs: list[RunOdds]) -> AcquiredPerRun:
    n = len(runs) or float("nan")
    n_all = sum(x.all_acquired for x in runs)
    lo, hi = wilson(n_all, n)
    return AcquiredPerRun(len(runs), n_all / n, lo, hi, sum(x.all_acquired_odds for x in runs) / n)


class ListDifference(NamedTuple):
    """the P(all eight) ratio on runs showing no list minus that on runs showing one: two-sample bootstrap (each set
    resampled on its own, seeded), 95% percentile interval and the share of draws at or above 0"""

    difference: float
    low: float
    high: float
    share_at_or_above_0: float

    def covers_0(self) -> bool:
        return self.low <= 0 <= self.high


@dataclass
class PerRunTests:
    runs: list[RunOdds]  # in run-id order (the bootstrap draws)
    acquired: AcquiredPerRun  # every run
    acquired_priority_assumed: AcquiredPerRun  # the runs whose open targets the priority assumption fits
    landing: RatioInterval  # P(all eight) observed-equivalent / predicted over every run, bootstrap 95%
    landing_by_list: dict[str, RatioInterval]  # the same on runs showing a priority list and on runs showing none
    effective: dict[str, float]  # the effective number of runs behind each ratio (weights: the solver's P(all))
    list_difference: ListDifference
    # the same set of runs: P(all eight) under the report's approximation counted by tier, and counted tier-blind
    tiered: dict[str, RatioInterval]
    blind: dict[str, RatioInterval]
    exact_p_all: float  # the solver's exact P(all), summed over runs
    product_p_all: float  # the product of its per-target p, summed over runs
    extras: PAllExtras


class Interval3(NamedTuple):
    estimate: float
    low: float
    high: float

    def covers_0(self) -> bool:
        return self.low <= 0 <= self.high


@dataclass
class PAllExtras:
    """how fragile the P(all eight) ratio is, and where its shortfall comes from"""

    bca: dict[str, tuple[float, float]]  # BCa 95% interval per run set
    drop_top: dict[str, dict[int, float]]  # the ratio with the k runs of highest predicted P(all) left out
    tier_difference: dict[str, Interval3]  # tier-blind minus by-tier ratio, paired bootstrap (the same resamples)
    recalibrated: dict[str, RatioInterval]  # tier-blind with each open row's chance scaled by its cell's ratio
    recalibrated_modelled: RatioInterval  # the same counting only modelled sources, every run


DROP_TOP: tuple[int, ...] = (10, 25, 50)


def bca_interval(obs: Sequence[float], pred: Sequence[float], draws: np.ndarray) -> tuple[float, float]:
    """the bias-corrected and accelerated 95% interval of sum(obs) / sum(pred) from its bootstrap draws (Efron 1987):
    bias from the share of draws below the estimate, acceleration from the jackknife (leave one run out)"""
    from scipy.stats import norm

    o, p = np.asarray(obs, float), np.asarray(pred, float)
    theta = o.sum() / p.sum()
    below = float((draws < theta).mean())
    if not 0 < below < 1:
        return percentile_interval(draws)
    z0 = norm.ppf(below)
    jack = (o.sum() - o) / np.maximum(1e-300, p.sum() - p)
    d = jack.mean() - jack
    a = float((d**3).sum() / (6 * ((d**2).sum()) ** 1.5)) if (d**2).sum() else 0.0
    bounds = []
    for z in (norm.ppf(0.025), norm.ppf(0.975)):
        bounds.append(float(norm.cdf(z0 + (z0 + z) / (1 - a * (z0 + z)))))
    lo, hi = np.quantile(draws, bounds)
    return float(lo), float(hi)


def paired_difference(
    obs1: Sequence[float], pred1: Sequence[float], obs2: Sequence[float], pred2: Sequence[float], seed: int
) -> Interval3:
    """ratio 1 minus ratio 2 over the same runs, both ratios on the same seeded resamples of whole runs"""
    o1, p1, o2, p2 = (np.asarray(x, float) for x in (obs1, pred1, obs2, pred2))
    rng = np.random.default_rng(seed)
    out: list[np.ndarray] = []
    for start in range(0, BOOTSTRAP_DRAWS, 500):
        ix = rng.integers(0, len(o1), (min(500, BOOTSTRAP_DRAWS - start), len(o1)))
        out.append(
            o1[ix].sum(1) / np.maximum(1e-300, p1[ix].sum(1)) - o2[ix].sum(1) / np.maximum(1e-300, p2[ix].sum(1))
        )
    d = np.concatenate(out)
    return Interval3(float(o1.sum() / p1.sum() - o2.sum() / p2.sum()), *percentile_interval(d))


def band_of(p: float) -> int:
    """the reliability band (index into pairwise(EDGES)) of a prediction"""
    return next(j for j, (a, b) in enumerate(pairwise(EDGES)) if a <= p < b)


def cell_ratios(rows: list[CalRow], obs: RowFn) -> dict[tuple[bool | None, int], float]:
    """observed / predicted acquisition per (priority status, band) over open rows: the recalibration's cells (in-sample)"""
    sums: collections.defaultdict[tuple[bool | None, int], list[float]] = collections.defaultdict(lambda: [0.0, 0.0])
    for r in rows:
        if is_open(r):
            cell = sums[prioritised(r), band_of(r["acq"])]
            cell[0] += 1 if obs(r) else 0
            cell[1] += r["acq"]
    return {k: o / e for k, (o, e) in sums.items() if e > 0}


def recalibrated_blind(pr: RunRows, ratios: dict[tuple[bool | None, int], float], obs: RowFn) -> tuple[float, float]:
    """one run's tier-blind P(all eight), observed and predicted, each open row's chance scaled by its cell's ratio"""
    targets = [r for r in pr["rows"] if r["g"] in DEFAULT]
    o = p = 1.0
    for r in targets:
        w = common.SPARK_WHITE_6 * 1.1 ** (r["nc"] - 6)
        certain = bool(r["outfit"]) or r["scen_gold"]
        o *= w * (1.0 if certain or obs(r) else 0.0)
        p *= w * (1.0 if certain else min(1.0, r["acq"] * ratios.get((prioritised(r), band_of(r["acq"])), 1.0)))
    return o, p


def p_all_extras(cal: Calibration, ordered: list[RunRows], runs: list[RunOdds], out: list[Value]) -> PAllExtras:
    by_set = {
        label: [i for i, x in enumerate(runs) if want is None or x.list_shown is want] for label, want in RUN_SETS
    }
    any_ratios, modelled_ratios = cell_ratios(cal.rows, acquired), cell_ratios(cal.rows, acquired_modelled)
    recal = [recalibrated_blind(pr, any_ratios, acquired) for pr in ordered]
    recal_m = [recalibrated_blind(pr, modelled_ratios, acquired_modelled) for pr in ordered]
    bca: dict[str, tuple[float, float]] = {}
    drop_top: dict[str, dict[int, float]] = {}
    tier_difference: dict[str, Interval3] = {}
    recalibrated: dict[str, RatioInterval] = {}
    for label, ix in by_set.items():
        sub = [runs[i] for i in ix]
        if not sub:
            bca[label], drop_top[label] = (NAN, NAN), {}
            tier_difference[label], recalibrated[label] = Interval3(NAN, NAN, NAN), RatioInterval(NAN, NAN, NAN)
            continue
        o, p = [x.landing_observed for x in sub], [x.p_all for x in sub]
        bca[label] = bca_interval(o, p, ratio_draws(o, p, seed=1))
        top = sorted(range(len(sub)), key=lambda i: -p[i])
        drop_top[label] = {}
        for k in DROP_TOP:
            keep = set(top[k:])
            drop_top[label][k] = common.div(sum(o[i] for i in keep), sum(p[i] for i in keep))
        tier_difference[label] = paired_difference(
            [x.blind_observed for x in sub],
            [x.blind_predicted for x in sub],
            o,
            [x.tiered_predicted for x in sub],
            seed=4,
        )
        ro, rp = [recal[i][0] for i in ix], [recal[i][1] for i in ix]
        recalibrated[label] = RatioInterval(
            common.div(sum(ro), sum(rp)), *percentile_interval(ratio_draws(ro, rp, seed=5))
        )
        out += [
            Value(S13, f"P(all eight) ratio, {label}: BCa 95% low", bca[label][0]),
            Value(S13, f"P(all eight) ratio, {label}: BCa 95% high", bca[label][1]),
            *[Value(S13, f"P(all eight) ratio, {label}: without the top {k}", v) for k, v in drop_top[label].items()],
            Value(S13, f"P(all eight), tier-blind minus by tier, {label}", tier_difference[label].estimate),
            Value(S13, f"P(all eight), tier-blind minus by tier, {label}: 95% low", tier_difference[label].low),
            Value(S13, f"P(all eight), tier-blind minus by tier, {label}: 95% high", tier_difference[label].high),
            Value(S13, f"P(all eight), tier-blind recalibrated, {label}", recalibrated[label].ratio),
            Value(S13, f"P(all eight), tier-blind recalibrated, {label}: 95% low", recalibrated[label].low),
            Value(S13, f"P(all eight), tier-blind recalibrated, {label}: 95% high", recalibrated[label].high),
        ]
    mo, mp = [x[0] for x in recal_m], [x[1] for x in recal_m]
    recal_modelled = (
        RatioInterval(common.div(sum(mo), sum(mp)), *percentile_interval(ratio_draws(mo, mp, seed=6)))
        if mo
        else RatioInterval(NAN, NAN, NAN)
    )
    out += [
        Value(S13, "P(all eight), tier-blind recalibrated, modelled sources only, all", recal_modelled.ratio),
        Value(S13, "P(all eight), tier-blind recalibrated, modelled sources only, all: 95% low", recal_modelled.low),
        Value(S13, "P(all eight), tier-blind recalibrated, modelled sources only, all: 95% high", recal_modelled.high),
        Value(S13, "P(all eight), recalibration cells", len(any_ratios)),
    ]
    return PAllExtras(bca, drop_top, tier_difference, recalibrated, recal_modelled)


RUN_SETS: tuple[tuple[str, bool | None], ...] = (("all", None), ("list shown", True), ("no list shown", False))


def per_run_tests(cal: Calibration, out: list[Value]) -> PerRunTests:
    shown = {r["id"]: r["priority"] is not None for r in cal.records or []}
    ordered = sorted(cal.runs, key=lambda x: x["id"])  # bootstrap draws in run-id order
    runs = [run_odds(pr, shown.get(pr["id"], False)) for pr in ordered]
    acquired = acquired_per_run(runs)
    assumed = acquired_per_run([x for x in runs if x.priority_assumed])
    out += [
        Value(S13, "per run: all open acquired, product of odds", acquired.all_acquired_odds_mean),
        Value(S13, "per run: all open acquired, observed", acquired.all_acquired_rate),
        Value(S13, "per run: all open acquired, 95% low", acquired.all_acquired_low),
        Value(S13, "per run: all open acquired, 95% high", acquired.all_acquired_high),
        Value(S13, "runs acquiring every open target", sum(x.all_acquired for x in runs)),
        Value(S13, "per run, priority assumption fits: runs", assumed.runs),
        Value(
            S13, "per run, priority assumption fits: all open acquired, product of odds", assumed.all_acquired_odds_mean
        ),
        Value(S13, "per run, priority assumption fits: all open acquired, observed", assumed.all_acquired_rate),
        Value(S13, "per run, priority assumption fits: all open acquired, 95% low", assumed.all_acquired_low),
        Value(S13, "per run, priority assumption fits: all open acquired, 95% high", assumed.all_acquired_high),
    ]

    def landing_obs(x: RunOdds) -> float:
        return x.landing_observed

    def landing_pred(x: RunOdds) -> float:
        return x.p_all

    by_set: dict[str, list[RunOdds]] = {
        label: [x for x in runs if want is None or x.list_shown is want] for label, want in RUN_SETS
    }
    landing_by_list: dict[str, RatioInterval] = {}
    effective: dict[str, float] = {}
    tiered: dict[str, RatioInterval] = {}
    blind: dict[str, RatioInterval] = {}
    for label, sub in by_set.items():
        landing_by_list[label] = ratio_interval(sub, landing_obs, landing_pred)
        effective[label] = effective_runs([x.p_all for x in sub])
        tiered[label] = ratio_interval(sub, landing_obs, lambda x: x.tiered_predicted)
        blind[label] = ratio_interval(sub, lambda x: x.blind_observed, lambda x: x.blind_predicted)
        v = landing_by_list[label]
        out += [
            Value(S13, f"P(all eight) ratio, {label}", v.ratio, note=f"{len(sub)} runs"),
            Value(S13, f"P(all eight) ratio, {label}: 95% low", v.low),  # bootstrap bounds: Monte Carlo (seeded)
            Value(S13, f"P(all eight) ratio, {label}: 95% high", v.high),
            Value(S13, f"P(all eight) ratio, {label}: runs", len(sub)),
            Value(S13, f"P(all eight) ratio, {label}: effective runs", effective[label]),
            Value(S13, f"P(all eight), report approximation by tier, {label}", tiered[label].ratio),
            Value(S13, f"P(all eight), tier-blind, {label}", blind[label].ratio),
            Value(S13, f"P(all eight), tier-blind, {label}: 95% low", blind[label].low),
            Value(S13, f"P(all eight), tier-blind, {label}: 95% high", blind[label].high),
        ]
    with_list, without = by_set["list shown"], by_set["no list shown"]
    if with_list and without:
        d = ratio_draws([x.landing_observed for x in without], [x.p_all for x in without], seed=2) - ratio_draws(
            [x.landing_observed for x in with_list], [x.p_all for x in with_list], seed=3
        )
        diff = ListDifference(
            landing_by_list["no list shown"].ratio - landing_by_list["list shown"].ratio,
            *percentile_interval(d),
            float((d >= 0).mean()),
        )
    else:
        diff = ListDifference(NAN, NAN, NAN, NAN)
    out += [
        Value(S13, "P(all eight) ratio, no list minus list", diff.difference),
        Value(S13, "P(all eight) ratio, no list minus list: 95% low", diff.low),
        Value(S13, "P(all eight) ratio, no list minus list: 95% high", diff.high),
        Value(S13, "P(all eight) ratio, no list minus list: share of draws at or above 0", diff.share_at_or_above_0),
    ]
    out.append(
        Value(
            S13,
            "per run: all open acquired, observed / product",
            common.div(acquired.all_acquired_rate, acquired.all_acquired_odds_mean),
        )
    )
    extras = p_all_extras(cal, ordered, runs, out)
    # the solver's exact P(all) against the product of its per-target p, summed over runs
    exact = sum(x["p_all"] for x in cal.runs)
    product = sum(math.prod(r["p_pred"] for r in x["rows"]) for x in cal.runs)
    return PerRunTests(
        runs,
        acquired,
        assumed,
        landing_by_list["all"],
        landing_by_list,
        effective,
        diff,
        tiered,
        blind,
        exact,
        product,
        extras,
    )


class PairStat(NamedTuple):
    """the deck test of one (card, target) pair where the card is the target's one deck source"""

    card: int
    target: int
    stat: Stat


@dataclass
class SingleSourceTests:
    """the deck test per (card, target) pair where the card is the target's one deck source"""

    listed: set[tuple[int | None, int]]  # the pairs the document's table lists
    of_listed: dict[tuple[int | None, int], Stat]
    pairs: list[tuple[int, int]]  # every pair in the rows
    off: list[PairStat]  # pairs whose ratio interval excludes 1
    off_large: set[tuple[int, int]]  # of them, with LARGE_PAIR rows or more
    off_unlisted: list[PairStat]  # large, not in the table
    off_small: list[PairStat]  # under LARGE_PAIR rows
    playtime_small: list[PairStat]  # small, Playtime's Over!, below its prediction
    playtime_hits: int
    playtime_rows: int
    playtime_lowest_predicted: float


def single_source_tests(cal: Calibration, out: list[Value]) -> SingleSourceTests:
    rep = cal.report
    listed: set[tuple[int | None, int]] = set()
    of_listed: dict[tuple[int | None, int], Stat] = {}
    for card, g in SINGLE:
        cid = cal.card_id.get(card)
        listed.add((cid, g))
        s = of_listed[cid, g] = deck_test(rep, single_source_is(cid), target_is(g))
        nm = card + " > " + cal.skill[g][0]  # the full card name: two Helios cards share a target
        out += [
            Value(S13, f"{nm}: rows", s.rows),
            Value(S13, f"{nm}: predicted", s.predicted),
            Value(S13, f"{nm}: ratio", s.ratio),
            Value(S13, f"{nm}: ratio 95% low", s.ratio_low),
            Value(S13, f"{nm}: ratio 95% high", s.ratio_high),
        ]
    # every single-source (card, target) pair whose ratio interval excludes 1
    pairs = sorted({(deck_source(r), r["g"]) for r in cal.rows if in_deck_test(r) and one_deck_source(r)})
    off: list[PairStat] = []
    for cid, g in pairs:
        s = deck_test(rep, single_source_is(cid), target_is(g))
        # a tolerance: k = n at a prediction of 1 is no miss
        if s.predicted and not s.ratio_low - 1e-9 <= 1 <= s.ratio_high + 1e-9:
            off.append(PairStat(cid, g, s))
    off_large = {(x.card, x.target) for x in off if x.stat.rows >= LARGE_PAIR}
    off_unlisted = [x for x in off if x.stat.rows >= LARGE_PAIR and (x.card, x.target) not in listed]
    small = [x for x in off if x.stat.rows < LARGE_PAIR]
    playtime = [x for x in small if x.target == PLAYTIMES_OVER and x.stat.ratio < 1]  # below its prediction
    hits, rows = sum(x.stat.hits for x in playtime), sum(x.stat.rows for x in playtime)
    lowest = min([x.stat.predicted for x in playtime] or [float("nan")])
    out += [
        Value(S13, "pairs under 40 rows excluding 1", len(small)),
        Value(S13, "pairs under 40 rows excluding 1: Playtime's Over! event sources", len(playtime)),
        Value(S13, "pairs under 40 rows excluding 1: Playtime's Over! event sources, runs", rows),
        Value(S13, "pairs under 40 rows excluding 1: Playtime's Over! event sources, hinted", hits),
        Value(S13, "pairs under 40 rows excluding 1: Playtime's Over! lowest predicted", lowest),
        Value(
            S13,
            "pairs under 40 rows excluding 1: Playtime's Over! highest predicted",
            max([x.stat.predicted for x in playtime] or [float("nan")]),
        ),
    ]
    return SingleSourceTests(
        listed, of_listed, pairs, off, off_large, off_unlisted, small, playtime, hits, rows, lowest
    )


@dataclass
class PriorityTests:
    """the likely reasons the single-source table gives, read on the runs that show a priority list (priority by skill
    name); each a list over: the skill on the list, on a list without it[, no list shown]"""

    sasami: list[Stat]  # [This Might Sting!] Sasami Anshinzawa > Risky Business
    king_halo: list[Tails]  # [Tonight, We Waltz] King Halo > Uma Stan (exact tails: predictions near 1)
    riko: list[Stat]  # [Planned Perfection] Riko Kashimoto > Ramp Up, by Lone Wolf
    playtime: list[Stat]  # the three Playtime's Over! event sources pooled, by the target's priority
    playtime_cards: set[int | None]


def priority_tests(cal: Calibration, out: list[Value]) -> PriorityTests:
    rep = cal.report
    priority_list: dict[int, list[str] | None] = {r["id"]: r["priority"] for r in (cal.records or [])}

    def on_list(skill: str, listed: bool | None) -> RowFn:
        """rows whose run lists skill (True), shows a list without it (False) or shows no list (None)"""

        def test(r: CalRow) -> bool:
            plist = priority_list.get(r["run"])
            return (None if plist is None else skill in plist) is listed

        return test

    def by_priority(card: str, g: int, skill: str, listed: bool | None) -> Stat:
        return deck_test(rep, single_source_is(cal.card_id.get(card)), target_is(g), on_list(skill, listed))

    sasami = [by_priority("[This Might Sting!] Sasami Anshinzawa", 20203, "Risky Business", v) for v in (True, False)]
    king_halo = [
        exact_tails(
            rep,
            where(
                in_deck_test,
                single_source_is(cal.card_id.get("[Tonight, We Waltz] King Halo")),
                target_is(20159),
                on_list("Uma Stan", v),
            ),
            deck_odds,
            deck_hinted,
        )
        for v in (True, False)
    ]
    for label, x in zip(("prioritised", "on a list, not prioritised"), king_halo):
        out += [
            Value(S13, f"King Halo > Uma Stan, {label}: rows", x.rows),
            Value(S13, f"King Halo > Uma Stan, {label}: hinted", x.observed),
            Value(S13, f"King Halo > Uma Stan, {label}: expected", x.expected),
        ]
    out.append(Value(S13, "King Halo > Uma Stan, prioritised: P(at most the hinted)", king_halo[0].at_most))
    riko = [by_priority("[Planned Perfection] Riko Kashimoto", 20046, "Lone Wolf", v) for v in (True, False, None)]
    playtime_cards = {cal.card_id.get(c) for c in PO_CARDS}

    def from_playtime_card(r: CalRow) -> bool:
        return one_deck_source(r) and deck_source(r) in playtime_cards

    po = [deck_test(rep, from_playtime_card, target_is(PLAYTIMES_OVER), priority_is(v)) for v in (True, False, None)]
    out.append(Value(S13, "Playtime's Over! event sources: runs showing a priority list", po[0].rows + po[1].rows))
    out += [
        Value(S13, "Playtime's Over! event sources, prioritised: rows", po[0].rows),
        Value(S13, "Playtime's Over! event sources, prioritised: hinted", po[0].hits),
        Value(S13, "Playtime's Over! event sources, prioritised: ratio 95% low", po[0].ratio_low),
        Value(S13, "Playtime's Over! event sources, prioritised: ratio 95% high", po[0].ratio_high),
    ]
    return PriorityTests(sasami, king_halo, riko, po, playtime_cards)


def certainty_checks(cal: Calibration, out: list[Value]) -> dict[str, Stat]:
    """section 9's certainty checks: every run carrying the card credited, where it is the skill's one deck source;
    by the card's holder"""
    cert: dict[str, Stat] = {}
    for card, g in (("[From the Ground Up] Light Hello", 20166), ("[Team Sirius] Team Sirius", 20203)):
        holder = card.split("] ")[1]
        cid = cal.card_id.get(card) or next(
            (c["id"] for c in common.gjson("web_cards.json")["cards"] if holder in c["name"]), None
        )
        s = cert[holder] = deck_test(cal.report, single_source_is(cid), target_is(g))
        out += [
            Value(
                S9,
                f"{holder} credited: runs carrying her",
                s.rows,
                note="her the one deck source, no inspiration of it",
            ),
            Value(S9, f"{holder} credited: credited", s.hits),
        ]
    return cert


@dataclass
class Tests:
    deck: DeckTests
    inheritance: InheritanceTests
    acquisition: AcquisitionTests
    target_inheritance: dict[int, Stat]
    target_acquired: dict[int, Stat]
    pool: PoolTests
    golds: GoldTests
    reliability: DeckReliability
    per_run: PerRunTests
    single: SingleSourceTests
    priority: PriorityTests
    certainty: dict[str, Stat]
    forms: list[RollFormTests]  # the parent factor, then the affinity slope


def computed_values(cal: Calibration, forms: list[RollForm], out: list[Value]) -> Tests:
    """every value of the section, in the document's order, appended to out"""
    run_counts(cal, out)
    deck = deck_tests(cal, out)
    inheritance = inheritance_tests(cal, out)
    form_tests = [roll_form_tests(cal, form, out) for form in forms]
    acquisition = acquisition_tests(cal, out)
    target_inheritance, target_acquired = per_target_tests(cal, out)
    pool = pool_tests(cal, out)
    golds = gold_tests(cal, out)
    reliability = reliability_tests(cal, out)
    per_run = per_run_tests(cal, out)
    single = single_source_tests(cal, out)
    priority = priority_tests(cal, out)
    certainty = certainty_checks(cal, out)
    return Tests(
        deck,
        inheritance,
        acquisition,
        target_inheritance,
        target_acquired,
        pool,
        golds,
        reliability,
        per_run,
        single,
        priority,
        certainty,
        form_tests,
    )


def findings(cal: Calibration, t: Tests) -> list[Finding]:
    """each conclusion, tested on the numbers above"""

    def tn(g: int) -> str:
        return cal.skill[g][0]

    def bucket_text(b: Bucket) -> str:
        return f"{bucket_label(b)}: {b.observed:.3f} [{b.observed_low:.3f}, {b.observed_high:.3f}] vs {b.predicted:.3f}"

    inh, deck, acq, rel, per_run, single, prio = (
        t.inheritance,
        t.deck,
        t.acquisition,
        t.reliability,
        t.per_run,
        t.single,
        t.priority,
    )
    acquired_below = [g for g in DEFAULT if g in t.target_acquired and t.target_acquired[g].ratio_high < 1]
    out_fs = [
        Finding(
            S13,
            "Known miss: per target, acquisition from any source reads below its prediction for "
            + " and ".join(tn(g) for g in sorted(ACQUIRED_INTERVAL_QUOTED))
            + " (each 95% interval entirely below 1)",
            set(ACQUIRED_INTERVAL_QUOTED) <= set(acquired_below),
            "targets below 1: "
            + (", ".join(f"{tn(g)} {ratio_text(t.target_acquired[g])}" for g in acquired_below) or "none"),
        )
    ]
    byc = inh.by_carriers
    t_off = [g for g in DEFAULT if g in t.target_inheritance and not covers(t.target_inheritance[g])]
    exception = t.target_inheritance.get(INHERITANCE_EXCEPTION)
    acq_all = per_run.acquired
    half_width = (acq_all.all_acquired_high - acq_all.all_acquired_low) / 2 / acq_all.all_acquired_rate

    def exception_text() -> str:
        return (
            f"without {tn(INHERITANCE_EXCEPTION)} {ratio_text(inh.without_exception)}; every row {ratio_text(inh.overall)}; "
            f"{tn(INHERITANCE_EXCEPTION)} {ratio_text(exception) if exception else 'no rows'}"
        )

    def choice_rule_consequence() -> bool:
        """non-prioritised (or outranked) event-only hints below 1 and below the prioritised interval"""
        p, n = deck.events_prioritised, deck.events_not_prioritised
        return below(n) and n.ratio_high < p.ratio_low

    def playtime_priority() -> bool:
        if prio.playtime[1].rows < 5:
            raise KeyError("too few rows on a list")  # common.finding: "needs data"
        return (
            (prio.playtime[0].rows < 5 or covers(prio.playtime[0]))
            and prio.playtime[1].hits == 0
            and below(prio.playtime[2])
        )

    po = prio.playtime
    return [
        *out_fs,
        # section 3
        finding(
            S3,
            "without Tail Held High, the other seven targets' observed / predicted inspiration has a 95% interval covering "
            "1; the targets do not differ detectably (heterogeneity p above 0.05), so leaving Tail Held High out is a "
            "selection of the one target whose own interval excludes 1, not evidence that it differs",
            lambda: covers(inh.without_exception) and inh.targets_differ.p >= 0.05,
            lambda: (
                exception_text()
                + "; per target, intervals excluding 1: "
                + (", ".join(f"{tn(g)} {ratio_text(t.target_inheritance[g])}" for g in t_off) or "none")
            ),
        ),
        finding(
            S3,
            "Tail Held High's own inheritance interval lies entirely below 1, the one target of eight whose interval does",
            lambda: exception is not None and below(exception) and t_off == [INHERITANCE_EXCEPTION],
            exception_text,
        ),
        finding(
            S3,
            "by carriers 1-6, with Tail Held High left out (a selection: the one target whose own interval excludes 1; "
            "the every-target version follows), every interval covers 1",
            lambda: all(k in byc and covers(byc[k]) for k in range(1, 7)),
            lambda: "; ".join(
                f"{k}: {byc[k].ratio:.3f} [{byc[k].ratio_low:.3f}, {byc[k].ratio_high:.3f}]"
                for k in range(1, 7)
                if k in byc
            ),
        ),
        finding(
            S3,
            "known miss: per roll, with Tail Held High left out (a selection), spark-years and ancestor hits each read below the model (z < -1.96, "
            "z run-clustered) by under 2%; the cause is not established at this level (parents at affinity 200 or more, a known "
            "miss of section 3, are a candidate, untested here)",
            lambda: (
                inh.spark_years.z < -1.96
                and inh.ancestor_hits.z < -1.96
                and inh.spark_years.observed / inh.spark_years.expected > 0.98
                and inh.ancestor_hits.observed / inh.ancestor_hits.expected > 0.98
            ),
            lambda: (
                f"spark-years {inh.spark_years.observed:,} vs {inh.spark_years.expected:,.1f} expected (z {inh.spark_years.z:+.2f}); "
                f"hits {inh.ancestor_hits.observed:,} vs {inh.ancestor_hits.expected:,.1f} (z {inh.ancestor_hits.z:+.2f})"
            ),
        ),
        finding(
            S3,
            "reliability: calibrated jointly over the buckets with a positive prediction (chi2 over the buckets, one degree of "
            "freedom each, nothing fitted, p above 0.05); the buckets outside their own 95% interval are named from the computed list",
            lambda: inh.joint.p > 0.05,
            lambda: (
                (
                    "misses: " + "; ".join(f"{bucket_text(b)} ({b.rows:,} rows)" for b in inh.misses)
                    if inh.misses
                    else "no bucket misses"
                )
                + f" (of {sum(1 for b in inh.buckets if b.rows and b.high_edge > ZERO_EDGE)} buckets tested at 95% each; "
                f"q = 0 bucket observed {inh.buckets[0].observed:.4f} over {inh.buckets[0].rows:,} rows); "
                f"chi2 {inh.joint.statistic:.1f} on {inh.joint.buckets} buckets, p {inh.joint.p:.3f}"
            ),
        ),
        finding(
            S13,
            "acquired: known miss, prioritised targets read slightly above the model's rate (run-clustered interval entirely "
            "above 1, by under 5%), non-prioritised below it (interval entirely below 1); a target counts as prioritised "
            "when it is on the run's list and no higher-ranked prioritised skill sits on a rival choice of every event "
            "offering it",
            lambda: acq.prioritised.ratio_low > 1 and acq.prioritised.ratio < 1.05 and below(acq.not_prioritised),
            lambda: (
                f"prioritised {acq.prioritised.ratio:.3f} [{acq.prioritised.ratio_low:.3f}, {acq.prioritised.ratio_high:.3f}]; "
                f"not prioritised {acq.not_prioritised.ratio:.3f} [{acq.not_prioritised.ratio_low:.3f}, {acq.not_prioritised.ratio_high:.3f}]"
            ),
        ),
        # section 9
        finding(
            S9,
            "the choice rule (IT takes an event's top choice unless a prioritised skill sits on another; between two prioritised "
            "skills on rival choices the higher-ranked one is assumed to win, untested) shows in its consequences: event-only hints of non-prioritised "
            "or outranked targets fall clearly below those of prioritised ones (interval entirely below 1 and below the "
            "prioritised interval)",
            choice_rule_consequence,
            lambda: (
                f"prioritised {ratio_text(deck.events_prioritised)}; not prioritised {ratio_text(deck.events_not_prioritised)}"
            ),
        ),
        finding(
            S9,
            "event-only hints of prioritised targets read at the model's rate (interval covers 1; with each run's priority "
            "list read for chain ends, the earlier shortfall is gone)",
            lambda: covers(deck.events_prioritised),
            lambda: f"prioritised {ratio_text(deck.events_prioritised)}",
        ),
        *[
            finding(
                S9,
                f"{nm}'s card is credited with her finale skill in every sampled run carrying her (an observed n of n "
                "rate, not a game rule)",
                lambda s=s: s.rows > 0 and s.hits == s.rows,
                lambda s=s: f"{s.hits} of {s.rows} (Wilson 95% low {s.low:.4f})",
            )
            for nm, s in t.certainty.items()
        ],
        finding(
            S3,
            "inheritance over all eight targets reads slightly below q: observed / predicted inspiration has a 95% "
            "interval entirely below 1",
            lambda: below(inh.overall),
            lambda: ratio_text(inh.overall),
        ),
        finding(
            S3,
            "the eight targets' inheritance ratios do not differ detectably: against one common ratio, the run-clustered "
            "Wald chi2 over the targets (7 degrees of freedom) has p of 0.05 or more",
            lambda: inh.targets_differ.p >= 0.05,
            lambda: (
                f"chi2 {inh.targets_differ.statistic:.2f} on {inh.targets_differ.buckets} degrees of freedom, p "
                f"{inh.targets_differ.p:.3f}; per target: "
                + ", ".join(f"{tn(g)} {ratio_text(s)}" for g, s in t.target_inheritance.items())
            ),
        ),
        finding(
            S3,
            "known miss: per roll, over every target (Tail Held High included), spark-years and ancestor hits each read "
            "below the model (z < -1.96, run-clustered)",
            lambda: inh.every_spark_years.z < -1.96 and inh.every_hits.z < -1.96,
            lambda: (
                f"spark-years {inh.every_spark_years.observed:,} vs {inh.every_spark_years.expected:,.1f} "
                f"(z {inh.every_spark_years.z:+.2f}); hits {inh.every_hits.observed:,} vs "
                f"{inh.every_hits.expected:,.1f} (z {inh.every_hits.z:+.2f})"
            ),
        ),
        finding(
            S3,
            "by carriers 1-6, over every target, every point estimate lies below 1 (the pointwise intervals are listed; "
            "how many exclude 1 is reported, not tested)",
            lambda: all(k in inh.by_carriers_every and inh.by_carriers_every[k].ratio < 1 for k in range(1, 7)),
            lambda: "; ".join(
                f"{k}: {ratio_text(inh.by_carriers_every[k])}" for k in range(1, 7) if k in inh.by_carriers_every
            ),
        ),
        finding(
            S13,
            "no dependence between the inspirations of one run's targets is detectable beyond the model, which takes them "
            "independent given the lineage (pairwise residual products, run-clustered |z| < 1.96)",
            lambda: abs(inh.dependence.z) < 1.96,
            lambda: (
                f"{inh.dependence.pairs:,} target pairs over {inh.dependence.runs:,} runs, mean product "
                f"{inh.dependence.mean_product:+.5f} (95% half-width {inh.dependence.half_width:.5f}), z "
                f"{inh.dependence.z:+.2f}"
            ),
        ),
        *[x for pf in t.forms for x in roll_form_findings(pf)],
        # section 13
        finding(
            S13,
            "the acquisition product approximation assigns zero probability to some observed acquisitions",
            lambda: any(b.rows > 0 and b.observed > 0 for b in rel.acquisition if b.high_edge <= ZERO_EDGE),
            lambda: "; ".join(bucket_text(b) for b in rel.acquisition if b.high_edge <= ZERO_EDGE),
        ),
        finding(
            S13,
            "the acquisition product approximation has no detected pooled bias on unknown-priority rows (95% ratio interval "
            "covers 1); known-prioritised rows lie slightly above 1 and known-unprioritised rows below 1",
            lambda: covers(acq.unknown_priority) and acq.prioritised.ratio_low > 1 and below(acq.not_prioritised),
            lambda: (
                f"prioritised {ratio_text(acq.prioritised)}; unknown priority {ratio_text(acq.unknown_priority)}; "
                f"not prioritised {ratio_text(acq.not_prioritised)}; "
                f"every row {ratio_text(acq.any_source)}"
            ),
        ),
        finding(
            S13,
            "inheritance reads slightly below q as a shortfall common to the targets: over every target the interval lies "
            "entirely below 1, the targets do not differ detectably (heterogeneity p above 0.05), and Tail Held High is "
            "the one target of eight whose own interval excludes 1",
            lambda: below(inh.overall) and inh.targets_differ.p >= 0.05 and t_off == [INHERITANCE_EXCEPTION],
            lambda: (
                exception_text()
                + "; per-target exceptions: "
                + (", ".join(f"{tn(g)} {ratio_text(t.target_inheritance[g])}" for g in t_off) or "none")
            ),
        ),
        finding(
            S13,
            "deck hints read optimistic: the deck test's observed / predicted interval lies entirely below 1",
            lambda: below(deck.overall),
            lambda: ratio_text(deck.overall),
        ),
        finding(
            S13,
            "event-source rows carry at least 80% of the positive deck shortfall (predicted minus observed hints); "
            "this does not identify its cause",
            lambda: rel.shortfall > 0 and rel.shortfall_events / rel.shortfall >= 0.8,
            lambda: (
                f"event-source shortfall {rel.shortfall_events:.1f} of {rel.shortfall:.1f} hints ({common.div(rel.shortfall_events, rel.shortfall):.0%}); "
                f"training pool only {ratio_text(deck.pool_only)}; prioritised {ratio_text(deck.events_prioritised)}; "
                f"not prioritised {ratio_text(deck.events_not_prioritised)}; no priority list shown {ratio_text(deck.events_no_list)}"
            ),
        ),
        finding(
            S13,
            "unmodelled scenario and trainee events supply what the deck loses: on the rows the priority assumption fits "
            "(target prioritised or no list shown), acquired from any source covers 1 and acquired from the deck or an "
            "inspiration lies below 1",
            lambda: covers(acq.any_source_assumed) and below(acq.modelled_source_assumed),
            lambda: (
                f"any source {ratio_text(acq.any_source_assumed)}; deck or inspiration {ratio_text(acq.modelled_source_assumed)}; "
                f"every row: any source {ratio_text(acq.any_source)}, deck or inspiration {ratio_text(acq.modelled_source)}; "
                f"rows acquired with no modelled source {len(acq.unexplained)}, {acq.unexplained_events_only} of them credited to events only"
            ),
        ),
        finding(
            S13,
            "Racing Spirit: Stamina has no deck source: no deck card in any run can hint it",
            lambda: (
                any(r["g"] == RACING_SPIRIT_STAMINA for r in cal.rows)
                and not any(r["g"] == RACING_SPIRIT_STAMINA and (r["cards_pool"] or r["cards_ev"]) for r in cal.rows)
            ),
            lambda: (
                f"{sum(1 for r in cal.rows if r['g'] == RACING_SPIRIT_STAMINA and (r['cards_pool'] or r['cards_ev']))} of "
                f"{sum(1 for r in cal.rows if r['g'] == RACING_SPIRIT_STAMINA):,} rows with a deck source"
            ),
        ),
        finding(
            S13,
            "training-pool hints (beta): Grand Concert, Unity Cup and URA Finale each have an interval covering 1",
            lambda: all(covers(t.pool.by_scenario[k]) for k in (3, 2, 1)),
            lambda: "; ".join(f"{common.SCEN[k]} {ratio_text(t.pool.by_scenario[k])}" for k in (3, 2, 1)),
        ),
        finding(
            S13,
            "only Trackblazer's interval excludes 1, its beta over-predicting: its interval lies entirely below 1, the other "
            "three cover 1",
            lambda: below(t.pool.by_scenario[4]) and all(covers(t.pool.by_scenario[k]) for k in (3, 2, 1)),
            lambda: f"Trackblazer {ratio_text(t.pool.by_scenario[4])}",
        ),
        finding(
            S13,
            "training pool by lowest limit break: LB0, LB1 and LB2 each have an interval covering 1",
            lambda: all(covers(s) for s in t.pool.low_limit_breaks),
            lambda: "; ".join(f"LB{lb} {ratio_text(s)}" for lb, s in zip((0, 1, 2), t.pool.low_limit_breaks)),
        ),
        finding(
            S13,
            "Unity Cup's two scenario golds show hinted in every sampled Unity Cup run (an observed n of n rate)",
            lambda: t.golds.unity.runs > 0 and t.golds.unity.with_both == t.golds.unity.runs,
            lambda: f"{t.golds.unity.with_both} of {t.golds.unity.runs} runs",
        ),
        finding(
            S13,
            "the deck's over-prediction sits in its top buckets, the event rewards: buckets predicted 0.6 and above carry over half the "
            "shortfall, each lies below its mean prediction, and over half their rows have an event source",
            lambda: (
                rel.shortfall_top / rel.shortfall > 0.5
                and rel.top_rows_with_event / rel.top_rows > 0.5
                and all(b.observed_high < b.predicted for b in rel.deck_tested if b.low_edge >= 0.6)
            ),
            lambda: (
                f"top-bucket shortfall {rel.shortfall_top:.1f} of {rel.shortfall:.1f} ({rel.shortfall_top / rel.shortfall:.0%}); "
                f"event-source rows {rel.top_rows_with_event:,} of {rel.top_rows:,} ({rel.top_rows_with_event / rel.top_rows:.0%}); "
                "buckets lying below their mean prediction: "
                + "; ".join(
                    f"{bucket_label(b)} {b.observed:.3f} [{b.observed_low:.3f}, {b.observed_high:.3f}] vs {b.predicted:.3f}"
                    for b in rel.deck_tested
                    if b.observed_high < b.predicted
                )
            ),
        ),
        finding(
            S13,
            "per run, acquiring every open target shows no detected miss over every run or over the runs the priority "
            "assumption fits (every open target prioritised, or no list shown): each all-open-acquired interval covers "
            "the mean product of the model's odds (its ratio, below 1, is reported)",
            lambda: (
                per_run.acquired.all_acquired_covered() and per_run.acquired_priority_assumed.all_acquired_covered()
            ),
            lambda: (
                f"every run: {per_run.acquired.text()}, ratio "
                f"{common.div(per_run.acquired.all_acquired_rate, per_run.acquired.all_acquired_odds_mean):.3f}; "
                f"priority assumption fits: {per_run.acquired_priority_assumed.text()}"
            ),
        ),
        finding(
            S13,
            "known miss: landing P(all eight) reads below the model, the pooled bootstrap interval, percentile and BCa, "
            "entirely below 1 (the model is optimistic about landing every target)",
            lambda: per_run.landing.high < 1 and per_run.extras.bca["all"][1] < 1,
            lambda: (
                f"{per_run.landing.ratio:.3f} [{per_run.landing.low:.3f}, {per_run.landing.high:.3f}], BCa "
                f"[{per_run.extras.bca['all'][0]:.3f}, {per_run.extras.bca['all'][1]:.3f}], "
                f"{per_run.effective['all']:.0f} effective runs; without the top runs by predicted P(all): "
                + ", ".join(f"{k} {v:.3f}" for k, v in per_run.extras.drop_top["all"].items())
            ),
        ),
        finding(
            S13,
            "P(all eight) on runs showing no priority list and on runs showing one: each point estimate is below 1; how "
            "far each interval is from 1 is reported, not tested, since a few runs carry most of the predicted mass "
            "and leaving out the top 10, 25 or 50 of them moves the ratio",
            lambda: (
                per_run.landing_by_list["no list shown"].ratio < 1 and per_run.landing_by_list["list shown"].ratio < 1
            ),
            lambda: "; ".join(
                f"{k} {v.ratio:.3f} [{v.low:.3f}, {v.high:.3f}], BCa [{per_run.extras.bca[k][0]:.3f}, "
                f"{per_run.extras.bca[k][1]:.3f}], {per_run.effective[k]:.0f} effective runs, without the top "
                + ", ".join(f"{n} {r:.3f}" for n, r in per_run.extras.drop_top[k].items())
                for k, v in per_run.landing_by_list.items()
            ),
        ),
        finding(
            S13,
            "P(all eight): no difference between runs showing a priority list and runs showing none is detectable: the "
            "two-sample bootstrap interval of the no-list ratio minus the list ratio covers 0",
            per_run.list_difference.covers_0,
            lambda: (
                f"difference {per_run.list_difference.difference:+.3f} [{per_run.list_difference.low:+.3f}, "
                f"{per_run.list_difference.high:+.3f}], share of draws at or above 0 "
                f"{per_run.list_difference.share_at_or_above_0:.3f}"
            ),
        ),
        finding(
            S13,
            "the gold tier carries part of the P(all eight) shortfall only on runs showing no list: the tier-blind minus "
            "by-tier ratio (every acquisition scored as a white on both sides, against the same approximation by tier; "
            "paired bootstrap on the same resamples) lies entirely above 0 there, and covers 0 over every run and on runs "
            "showing a list",
            lambda: (
                per_run.extras.tier_difference["no list shown"].low > 0
                and per_run.extras.tier_difference["all"].covers_0()
                and per_run.extras.tier_difference["list shown"].covers_0()
            ),
            lambda: "; ".join(
                f"{k}: by tier {per_run.tiered[k].ratio:.3f}, tier-blind {per_run.blind[k].ratio:.3f}, difference "
                f"{per_run.extras.tier_difference[k].estimate:+.3f} [{per_run.extras.tier_difference[k].low:+.3f}, "
                f"{per_run.extras.tier_difference[k].high:+.3f}]"
                for k, _ in RUN_SETS
            ),
        ),
        finding(
            S13,
            "in-sample: with each open row's acquisition chance scaled by the observed / predicted ratio of its cell "
            "(priority status and band of prediction), the tier-blind P(all eight) ratio's interval covers 1 on every run "
            "set: the acquisition part of the shortfall is the per-target miss at high predictions compounded over the "
            "targets, not a failure of the joint",
            lambda: all(covers_interval(per_run.extras.recalibrated[k]) for k, _ in RUN_SETS),
            lambda: (
                "; ".join(
                    f"{k}: tier-blind {per_run.blind[k].ratio:.3f}, recalibrated {per_run.extras.recalibrated[k].ratio:.3f} "
                    f"[{per_run.extras.recalibrated[k].low:.3f}, {per_run.extras.recalibrated[k].high:.3f}]"
                    for k, _ in RUN_SETS
                )
                + f"; modelled sources only, every run {per_run.extras.recalibrated_modelled.ratio:.3f} "
                f"[{per_run.extras.recalibrated_modelled.low:.3f}, {per_run.extras.recalibrated_modelled.high:.3f}]"
            ),
        ),
        finding(
            S13,
            "known miss: gold acquisition, pooled over the golds the deck can give and the model does not predict "
            "certain, reads below the model (run-clustered interval entirely below 1)",
            lambda: below(t.golds.uncertain),
            lambda: (
                f"uncertain golds {ratio_text(t.golds.uncertain)}; every gold {ratio_text(t.golds.pooled)}; jointly chi2 {t.golds.joint.statistic:.1f} on "
                f"{t.golds.joint.buckets} golds, p {t.golds.joint.p:.4f}; per gold: "
                + "; ".join(f"{cal.skill[g][1]} {ratio_text(st)}" for g, st in t.golds.by_gold.items())
            ),
        ),
        finding(
            S13,
            "the gold miss sits where the target is not shown prioritised: on those golds, prioritised rows have an "
            "interval covering 1, while unknown-priority and not-prioritised rows each lie entirely below 1",
            lambda: (
                covers(t.golds.uncertain_by_priority["prioritised"])
                and below(t.golds.uncertain_by_priority["unknown priority"])
                and below(t.golds.uncertain_by_priority["not prioritised"])
            ),
            lambda: "; ".join(f"{k} {ratio_text(v)}" for k, v in t.golds.uncertain_by_priority.items()),
        ),
        finding(
            S13,
            "the golds jointly depart from the model: the run-clustered chi2 of every gold's observed - predicted, nothing "
            "fitted, has p below 0.05",
            lambda: t.golds.joint.p < 0.05,
            lambda: f"chi2 {t.golds.joint.statistic:.1f} on {t.golds.joint.buckets} golds, p {t.golds.joint.p:.4f}",
        ),
        finding(
            S13,
            "known miss: on unknown-priority rows the model is optimistic where it rates targets likely: rows predicted "
            "0.6 and above read below it from any source (interval entirely below 1), and counting only the sources the "
            "model has (a deck card's hint or an inspiration) the bands jointly depart from it (run-clustered chi2 over "
            "the bands with a positive prediction, nothing fitted, p below 0.05)",
            lambda: (
                below(rel.likely["unknown priority"]) and rel.modelled_by_priority_joint["unknown priority"].p < 0.05
            ),
            lambda: (
                f"0.6 and above: any source {ratio_text(rel.likely['unknown priority'])}, modelled only "
                f"{ratio_text(rel.likely_modelled['unknown priority'])}; modelled-only bands: "
                + modelled_text(rel, "unknown priority")
                + f"; any-source bands: {reliability_text(rel, 'unknown priority')}"
            ),
        ),
        finding(
            S13,
            "on unknown-priority rows predicted under 0.6, counting only modelled sources, no departure is detectable "
            "(interval covers 1): the excess the any-source bands show there comes from sources the model leaves out",
            lambda: covers(rel.unlikely_modelled["unknown priority"]),
            lambda: f"modelled only {ratio_text(rel.unlikely_modelled['unknown priority'])}",
        ),
        finding(
            S13,
            "on prioritised rows no departure by band is detectable (the same chi2, p above 0.05)",
            lambda: rel.by_priority_joint["prioritised"].p > 0.05,
            lambda: reliability_text(rel, "prioritised"),
        ),
        finding(
            S13,
            "too few runs acquired every open target to separate the exact joint from the product: the solver's exact P(all) differs "
            "from the product of its per-target p by less than the relative half-width of the all-acquired interval",
            lambda: abs(per_run.exact_p_all / per_run.product_p_all - 1) < half_width,
            lambda: (
                f"exact / product {per_run.exact_p_all / per_run.product_p_all:.3f} (sums over runs {per_run.exact_p_all:.4f} / "
                f"{per_run.product_p_all:.4f}); all acquired {sum(x.all_acquired for x in per_run.runs)} runs, relative half-width {half_width:.0%}"
            ),
        ),
        finding(
            S13,
            "the single-source table holds exactly the pairs of 40+ rows whose ratio interval excludes 1",
            lambda: bool(single.pairs) and not single.off_unlisted and single.listed <= single.off_large,
            lambda: (
                f"{len(single.off)} of {len(single.pairs)} pairs exclude 1; not listed: "
                + (
                    "; ".join(
                        f"{cal.card_name[x.card]} > {tn(x.target)} {x.stat.hits}/{x.stat.rows} vs pred {x.stat.predicted:.3f} = {x.stat.ratio:.2f} "
                        f"[{x.stat.ratio_low:.2f}, {x.stat.ratio_high:.2f}]"
                        for x in sorted(single.off_unlisted, key=lambda x: -x.stat.rows)
                    )
                    or "none"
                )
                + "; listed without excluding 1: "
                + (
                    ", ".join(
                        f"{cal.card_name[c]} > {tn(g)} [{single.of_listed[c, g].ratio_low:.2f}, {single.of_listed[c, g].ratio_high:.2f}]"
                        for c, g in sorted(single.listed - single.off_large)
                    )
                    or "none"
                )
            ),
        ),
        finding(
            S13,
            "below 40 rows more pairs exclude 1; Playtime's Over!'s chain-ending event sources among them read below their "
            "prediction: their pooled hint rate has a 95% interval entirely under their lowest prediction",
            lambda: (
                bool(single.off_small)
                and bool(single.playtime_small)
                and wilson(single.playtime_hits, single.playtime_rows)[1] < single.playtime_lowest_predicted
            ),
            lambda: (
                f"{len(single.off_small)} pairs; Playtime's Over! sources hinted {single.playtime_hits}/{single.playtime_rows} "
                f"[{wilson(single.playtime_hits, single.playtime_rows)[0]:.3f}, {wilson(single.playtime_hits, single.playtime_rows)[1]:.3f}] "
                f"vs lowest prediction {single.playtime_lowest_predicted:.3f}; "
                + "; ".join(
                    f"{cal.card_name[x.card]} > {tn(x.target)} {x.stat.hits}/{x.stat.rows} vs pred {x.stat.predicted:.3f} "
                    f"[{x.stat.ratio_low:.2f}, {x.stat.ratio_high:.2f}]"
                    for x in single.off_small
                )
            ),
        ),
        finding(
            S13,
            "the Playtime's Over! miss is the priority assumption (the skill sits on a chain-ending choice 2, taken only when "
            "prioritised): with each run's list read, prioritised rows cover 1, rows on a list without it are predicted none "
            "and show none, and rows from runs showing no list lie below 1 (there the model assumes the end can be taken; "
            '"needs data" when fewer than 5 rows are on a list)',
            playtime_priority,
            lambda: (
                f"prioritised {po[0].hits}/{po[0].rows} "
                + (ratio_text(po[0]) if po[0].rows else "")
                + f"; on a list, not prioritised "
                f"{po[1].hits}/{po[1].rows} {ratio_text(po[1])}; no list shown {po[2].hits}/{po[2].rows} {ratio_text(po[2])}"
            ),
        ),
        finding(
            S13,
            "Sasami Anshinzawa [This Might Sting!] > Risky Business: the model assumes Risky Business prioritised: with it on the "
            "priority list her hint lands at the model's rate (interval covers 1), without it below (interval below 1)",
            lambda: covers(prio.sasami[0]) and below(prio.sasami[1]),
            lambda: (
                f"prioritised {ratio_text(prio.sasami[0])}; on a list, not prioritised {ratio_text(prio.sasami[1])}"
            ),
        ),
        finding(
            S13,
            "King Halo [Tonight, We Waltz] > Uma Stan sits on a choice that needs a priority: prioritised, the hints are "
            "consistent with the model (exact two-sided 95%: neither tail of the rows' Poisson-binomial below 2.5%); on a "
            "list without it, below (lower tail under 2.5%)",
            lambda: prio.king_halo[0].consistent() and prio.king_halo[1].below(),
            lambda: f"prioritised {prio.king_halo[0].text()}; on a list, not prioritised {prio.king_halo[1].text()}",
        ),
        finding(
            S13,
            "Riko Kashimoto [Planned Perfection] > Ramp Up: her Lone Wolf choice when prioritised accounts for part: rows with Lone "
            "Wolf prioritised lie below 1, and rows from runs showing no priority list still lie below 1",
            lambda: below(prio.riko[0]) and below(prio.riko[2]),
            lambda: (
                f"Lone Wolf prioritised {ratio_text(prio.riko[0])}; on a list, not prioritised {ratio_text(prio.riko[1])}; "
                f"no list shown {ratio_text(prio.riko[2])}"
            ),
        ),
    ]


def modelled_text(rel: DeckReliability, status: str) -> str:
    j = rel.modelled_by_priority_joint[status]
    return f"chi2 {j.statistic:.1f} on {j.buckets} bands, p {j.p:.2g}; " + "; ".join(
        f"{bucket_label(b)}: {b.observed:.3f} vs {b.predicted:.3f}"
        for b in rel.modelled_by_priority[status]
        if b.rows and b.high_edge > ZERO_EDGE
    )


def reliability_text(rel: DeckReliability, status: str) -> str:
    j = rel.by_priority_joint[status]
    return f"chi2 {j.statistic:.1f} on {j.buckets} bands, p {j.p:.4f}; " + "; ".join(
        f"{bucket_label(b)}: {b.observed:.3f} [{b.observed_low:.3f}, {b.observed_high:.3f}] vs {b.predicted:.3f}"
        for b in rel.by_priority[status]
        if b.rows and b.high_edge > ZERO_EDGE
    )


def roll_form_findings(pf: RollFormTests) -> list[Finding]:
    """section 3's misses under an alternative roll (a check: the model is unchanged). The form is fitted on
    single-ancestor trials of these same careers, so the rows with one carrier overlap its fit: in-sample, not held
    out; the rows with two or more carriers are the part the fit did not read directly"""
    form = pf.form
    what = (
        "each parent's per-roll chance scaled by a flat parent factor"
        if form.name == "parent factor"
        else "every legacy's affinity scaled by a slope"
    )
    pre = (
        f"{form.name} (a check, the model unchanged, in-sample): with {what}, refitted on section 3's own selection of "
        "single-ancestor trials,"
    )

    def z_text(c: CountTest) -> str:
        return f"{c.observed:,} vs {c.expected:,.1f} (z {c.z:+.2f})"

    return [
        finding(
            S3,
            f"{pre} the per-roll counts over every target, spark-years and ancestor hits, each read within the model "
            "(|z| < 1.96, run-clustered)",
            lambda: abs(pf.spark_years.z) < 1.96 and abs(pf.ancestor_hits.z) < 1.96,
            lambda: (
                f"{form.name} {form.estimate:.4f} [{form.low:.4f}, {form.high:.4f}] on {form.trials:,} trials; "
                f"spark-years {z_text(pf.spark_years)}; hits {z_text(pf.ancestor_hits)}; without Tail Held High "
                f"spark-years {z_text(pf.spark_years_without_exception)}; q rebuilt from the legs within "
                f"{pf.q_rebuilt_error:.1e}"
            ),
        ),
        finding(
            S3,
            f"{pre} inheritance over all eight targets has an interval covering 1, and so has Tail Held High's",
            lambda: covers(pf.overall) and covers(pf.exception),
            lambda: f"every target {ratio_text(pf.overall)}; Tail Held High {ratio_text(pf.exception)}",
        ),
        finding(
            S3,
            f"{pre} the target rows with two or more carriers, outside the fitting trials, have an interval covering 1",
            lambda: covers(pf.several),
            lambda: f"scaled {ratio_text(pf.several)}; the model {ratio_text(pf.several_model)}",
        ),
        finding(
            S3,
            f"{pre} the q band 0.40-0.60 covers its mean prediction",
            lambda: pf.middle.observed_low <= pf.middle.predicted <= pf.middle.observed_high,
            lambda: (
                f"{pf.middle.observed:.4f} [{pf.middle.observed_low:.4f}, {pf.middle.observed_high:.4f}] vs "
                f"{pf.middle.predicted:.4f} on {pf.middle.rows:,} rows; reliability chi2 p {pf.joint.p:.3f}; targets "
                f"differ p {pf.targets_differ.p:.3f}"
            ),
        ),
    ]


def claims(wide: bool | None = None) -> list[Value | Finding]:
    R = common.runs()
    why = "needs data" if R is None else "needs master.mdb" if common.mdb() is None else None
    M = Model()
    cal = calibration(R, M, why)
    values: list[Value] = []
    forms = roll_forms(None if why else R)
    tests = computed_values(cal, forms, values)
    fs = findings(cal, tests)
    if why:  # nothing computed: only the findings, as "needs data"
        for f in fs:
            f.holds, f.evidence = None, why
        return fs + wide_findings(None, None, why=why)
    out: list[Value | Finding] = [*values, *fs]
    open(os.path.join(OUTD, "report8.txt"), "w").write("\n".join(cal.report.lines) + "\n")
    if wide is None:
        wide = "--wide" in sys.argv
    out += wide_claims(R, M, forms) if wide else wide_findings(None, None, why="not run (--wide)")
    return out


def wide_claims(R: list[Run], M: Model, forms: list[RollForm]) -> list[Value | Finding]:
    """inheritance and training-pool hints on every other skill a deck card or an ancestor's spark could give"""
    jobdir = os.path.join(OUTD, "jobsW")
    man, files, _ = build_jobs(R, M, True, jobdir)
    res = run_solver(jobdir, files, os.path.join(OUTD, "resW.jsonl"), share=0.3, man=man)
    rows, _, _ = rows_of(R, M, res, man)
    rows = [r for r in rows if r["g"] not in DEFAULT]
    rep = Report(rows)
    s_i = inheritance_test(rep)
    s_p = deck_test(rep, lambda r: r["cards_pool"] and not r["cards_ev"])
    legs = leg_details(res)
    # under each alternative roll: every non-target row, and the rows with two or more carriers (outside its fit)
    form_stats: list[tuple[RollForm, Stat, Stat]] = []
    for form in forms:
        chances = rolled_legs(legs, form)
        q_form = {id(r): q_of_legs(chances(r)) for r in rows}
        form_stats.append(
            (
                form,
                rep.stat(every_row, lambda r, q_form=q_form: q_form[id(r)], inspired),
                rep.stat(several_carriers, lambda r, q_form=q_form: q_form[id(r)], inspired),
            )
        )
    several_model = rep.stat(several_carriers, inspiration_odds, inspired)
    # the inheritance shortfall (predicted minus observed inspirations) by skill: the largest one skill's share
    by_skill: collections.Counter[int] = collections.Counter()
    for r in rows:
        by_skill[r["g"]] += r["q"] - r["o_insp"]
    top_group, top = by_skill.most_common(1)[0] if by_skill else (0, float("nan"))
    top_share = common.div(top, sum(by_skill.values()))
    open(os.path.join(OUTD, "reportW.txt"), "w").write(f"runs {len({r['run'] for r in rows})}, jobs {len(res)}\n")
    sampled = len({m.run_id for m in man if any(f.startswith(f"{m.run_id}_") for f in res)})
    return [
        Value(S13, "other skills: runs sampled", sampled),
        Value(S13, "other skills: inheritance rows", s_i.rows),
        Value(S13, "other skills: inheritance", s_i.ratio),
        Value(S13, "other skills: inheritance 95% low", s_i.ratio_low),
        Value(S13, "other skills: inheritance 95% high", s_i.ratio_high),
        Value(S13, "other skills: training-pool rows", s_p.rows),
        Value(S13, "other skills: training-pool hints", s_p.ratio),
        Value(S13, "other skills: training-pool 95% low", s_p.ratio_low),
        Value(S13, "other skills: training-pool 95% high", s_p.ratio_high),
        Value(S3, "non-target skills: inheritance rows", s_i.rows),
        Value(S3, "non-target skills: inheritance", s_i.ratio),
        Value(S3, "non-target skills: inheritance 95% low", s_i.ratio_low),
        Value(S3, "non-target skills: inheritance 95% high", s_i.ratio_high),
        Value(S13, "other skills: largest one skill's share of the inheritance shortfall", top_share),
        *test_values("other skills: inheritance, two or more carriers", several_model, ("rows", *RATIO)),
        *[
            v
            for form, every, several in form_stats
            for v in (
                *test_values(f"{form.name}: other skills' inheritance", every, RATIO),
                *test_values(
                    f"{form.name}: other skills' inheritance, two or more carriers", several, ("rows", *RATIO)
                ),
            )
        ],
        *wide_findings(
            s_i,
            s_p,
            top_share,
            common.skills()[top_group][0] if top_group else "none",
            forms=form_stats,
            several_model=several_model,
        ),
    ]


def wide_findings(
    s_i: Stat | None,
    s_p: Stat | None,
    top_share: float = NAN,
    top_skill: str = "",
    why: str | None = None,
    forms: list[tuple[RollForm, Stat, Stat]] | None = None,
    several_model: Stat | None = None,
) -> list[Finding]:
    """the conclusions drawn on the other skills (section 3's non-target rows, section 13's "Other skills"); not tested
    (why) when they were not computed"""
    statements = [
        (
            S3,
            "known miss: inspiration odds on non-target skills read short of the model (interval entirely below 1), the "
            "shortfall spread over many skills (no one skill carries a quarter of it)",
            lambda: below(s_i) and top_share < 0.25,
            lambda: f"{ratio_text(s_i)}; largest one skill's share of the shortfall {top_skill} {top_share:.0%}",
        ),
        (
            S13,
            "other skills, known miss: beyond the eight targets inheritance reads short of the model (interval entirely "
            "below 1), spread over many skills (no one skill carries a quarter of the shortfall)",
            lambda: below(s_i) and top_share < 0.25,
            lambda: f"{ratio_text(s_i)}; largest one skill's share of the shortfall {top_skill} {top_share:.0%}",
        ),
        (
            S13,
            "other skills: training-pool hints under-predicted: interval entirely above 1",
            lambda: s_p.ratio_low > 1,
            lambda: ratio_text(s_p),
        ),
        *[
            (
                S3,
                f"{name} (a check, the model unchanged, in-sample): rescored with it, the non-target skills' rows with two "
                "or more carriers, which are not its fitting trials, have an inheritance interval covering 1",
                lambda i=i: covers(forms[i][2]),
                lambda i=i: (
                    f"two or more carriers: scaled {ratio_text(forms[i][2])}, the model {ratio_text(several_model)}; "
                    f"every row: scaled {ratio_text(forms[i][1])}, the model {ratio_text(s_i)}"
                ),
            )
            for i, name in enumerate(("parent factor", "affinity slope"))
        ],
    ]
    if why:
        return [Finding(section, statement, None, why) for section, statement, _, _ in statements]
    return [finding(section, statement, test, evidence) for section, statement, test, evidence in statements]


if __name__ == "__main__":
    common.main(claims)
