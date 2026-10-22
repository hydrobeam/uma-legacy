"""Section 3: the inheritance roll's parts, tested on single-ancestor trials from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it).

A trial (built by s05_affinity.trials) is one (ancestor, white skill spark, career year 2 or 3) where the spark's skill
is carried by exactly one of the six ancestors, so a year's inspiration of it can only come from her; it passes when the
factor shows among that year's inspirations. The model gives one roll per year, r = min(1, b(s) (1 + A / 100)),
b(s) = 3 / 6 / 9% by stars, A the ancestor's affinity (section 5, from master.mdb relation points and the record's
shared G1 wins). Ordinary ancestors only (no self-grandparent or grandparent equal to the parent).
Fitted parameters carry run-clustered (sandwich) 95% intervals, since one run gives many trials.
Tests: the grandparent weight (r scaled by kappa_G: full base kappa_G = 1 against halved 0.5), the affinity slope
(r = b (1 + s A / 100), s free; the model has s = 1), and the base by stars (observed / predicted per star count and per
affinity band). A sum's interval is observed / predicted by the run-clustered ratio estimator (common.cluster_ci: a
run's trials share its lineage, so the variance comes from each run's residual against the model); the bands' joint
test is the run-clustered Wald chi2 (common.cluster_chi2), since one run's trials fall in several bands.
"""

import collections, math
from collections.abc import Callable
from typing import NamedTuple
from scipy.optimize import minimize_scalar
import common
from common import Finding, Value
import s05_affinity


class Trial(NamedTuple):
    """an ordinary ancestor's trial with its affinity (from s05_affinity.Trial, tri(T, P, T) = 0)"""

    category: str  # 'P' a parent, 'G' a grandparent
    affinity: int  # A
    base: float  # b by the spark's stars
    passed: bool
    run: int  # run id


class Ratio(NamedTuple):
    """observed / predicted passes of a set of trials under the model (ratio())"""

    trials: int
    observed: int
    predicted: float
    ratio: float
    low: float  # its run-clustered 95% interval
    high: float


class Fit(NamedTuple):
    """a one-parameter maximum-likelihood fit with its run-clustered 95% interval (fit1())"""

    estimate: float
    low: float
    high: float


# a fitted parameter's log-likelihood per run
RunLL = Callable[[float], dict[int, float]]

SEC: str = "3"
CHI1: float = 1.920729  # chi2(1) 95% / 2: a likelihood gain beyond it is detectable
STARS: dict[float, int] = {0.03: 1, 0.06: 2, 0.09: 3}
BANDS: list[tuple[int, int]] = [(0, 50), (50, 100), (100, 150), (150, 200), (200, 10**6)]


def clipped_ll(p: float, passed: bool) -> float:
    """one trial's log-likelihood, its chance clipped to [1e-12, 1 - 1e-12]"""
    p = min(max(p, 1e-12), 1 - 1e-12)
    return math.log(p if passed else 1 - p)


def ll(ts: list[Trial], parent_weight: float = 1.0, grandparent_weight: float = 1.0, slope: float = 1.0) -> float:
    """log-likelihood with r = min(1, weight b (1 + slope A / 100)), the weight by the ancestor's generation"""
    t = 0.0
    for x in ts:
        weight = grandparent_weight if x.category == "G" else parent_weight
        t += clipped_ll(min(1.0, weight * x.base * (1 + slope * x.affinity / 100)), x.passed)
    return t


def llr(
    ts: list[Trial], parent_weight: float = 1.0, grandparent_weight: float = 1.0, slope: float = 1.0
) -> dict[int, float]:
    """{run: log-likelihood of that run's trials} (as ll())"""
    out: dict[int, float] = collections.defaultdict(float)
    for x in ts:
        weight = grandparent_weight if x.category == "G" else parent_weight
        out[x.run] += clipped_ll(min(1.0, weight * x.base * (1 + slope * x.affinity / 100)), x.passed)
    return out


def parts_llr(full: list[s05_affinity.Trial], relation_slope: float, race_slope: float) -> dict[int, float]:
    """{run: log-likelihood} with the two parts of A apart: r = b (1 + (s_rel (A - race) + s_race race) / 100)"""
    out: dict[int, float] = collections.defaultdict(float)
    for t in full:
        a, race = t.affinity_h0, t.race_part
        p = min(1.0, t.base * (1 + (relation_slope * (a - race) + race_slope * race) / 100))
        out[t.run] += clipped_ll(p, t.passed)
    return out


def fit1(run_ll: RunLL, lo: float, hi: float) -> Fit:
    """(maximum-likelihood estimate on [lo, hi], run-clustered 95% low, high): a run gives many trials, so the
    interval is the sandwich one (common.robust_interval), not the likelihood interval of independent trials"""
    r = minimize_scalar(lambda x: -sum(run_ll(x).values()), bounds=(lo, hi), method="bounded", options={"xatol": 1e-5})
    a, b, _ = common.robust_interval(run_ll, float(r.x))
    return Fit(float(r.x), a, b)


def pred(a: float, b: float) -> float:
    """the model's chance at affinity a and base b"""
    return min(1.0, b * (1 + a / 100))


def ratio(ts: list[Trial]) -> Ratio:
    """(trials, observed, predicted, obs / pred, low, high) under the model; the interval is run-clustered
    (common.cluster_ci)"""
    o = sum(t.passed for t in ts)
    e = sum(pred(t.affinity, t.base) for t in ts)
    _, lo, hi, _ = common.cluster_ci(
        [t.passed for t in ts], [pred(t.affinity, t.base) for t in ts], [t.run for t in ts]
    )
    return Ratio(len(ts), o, e, common.div(o, e), lo, hi)


def claims() -> list[Value | Finding]:
    R = common.runs()
    if R is None:
        return []
    full = [t for t in s05_affinity.trials(R, race=True) if t.category in ("P", "G")]
    ts = [Trial(t.category, t.affinity_h0, t.base, t.passed, t.run) for t in full]
    return generation_claims(ts) + slope_claims(ts) + affinity_part_claims(full) + star_claims(ts) + band_claims(ts)


def generation_claims(ts: list[Trial]) -> list[Value | Finding]:
    """obs / pred for parents and grandparents, and the grandparent weight (full base against halved)"""
    out: list[Value | Finding] = []
    P = [t for t in ts if t.category == "P"]
    G = [t for t in ts if t.category == "G"]
    for lab, sub in (("parents", P), ("grandparents", G)):
        c = ratio(sub)
        out += [
            Value(SEC, f"trials, {lab}", c.trials),
            Value(SEC, f"{lab}: passes observed", c.observed),
            Value(SEC, f"{lab}: predicted, full base", c.predicted),
            Value(SEC, f"{lab}: obs / pred", c.ratio),
            Value(SEC, f"{lab}: obs / pred 95% low", c.low),
            Value(SEC, f"{lab}: obs / pred 95% high", c.high),
        ]
    halved = sum(min(1.0, 0.5 * t.base * (1 + t.affinity / 100)) for t in G)
    d_half = ll(ts) - ll(ts, grandparent_weight=0.5)
    kg, kg_lo, kg_hi = fit1(lambda k: llr(ts, grandparent_weight=k), 0.2, 2.0)
    out += [
        Value(SEC, "grandparents: predicted, halved base", halved),
        Value(SEC, "grandparents full over halved, nats", d_half),
        Value(SEC, "grandparent weight fitted", kg),
        Value(SEC, "grandparent weight 95% low", kg_lo),
        Value(SEC, "grandparent weight 95% high", kg_hi),
    ]
    out.append(
        common.finding(
            SEC,
            "Grandparents take the full base: the fitted grandparent weight's 95% interval covers 1 and "
            "excludes 0.5, and the full base beats the halved one by more than 1.92 nats",
            kg_lo <= 1 <= kg_hi and not kg_lo <= 0.5 <= kg_hi and d_half > CHI1,
            f"weight {kg:.2f} [{kg_lo:.2f}, {kg_hi:.2f}] on {len(G):,} trials; full over halved {d_half:.1f} nats",
        )
    )
    return out


def slope_claims(ts: list[Trial]) -> list[Value | Finding]:
    """the affinity slope s in r = b (1 + s A / 100), and A against no affinity"""
    s_hat, s_lo, s_hi = fit1(lambda s: llr(ts, slope=s), 0.0, 3.0)
    d0 = ll(ts) - ll(ts, slope=0)
    out: list[Value | Finding] = [
        Value(SEC, "affinity slope fitted", s_hat),
        Value(SEC, "affinity slope 95% low", s_lo),
        Value(SEC, "affinity slope 95% high", s_hi),
        Value(SEC, "affinity over none, nats", d0),
    ]
    out.append(
        common.finding(
            SEC,
            "Affinity scales the roll as (1 + A/100): the fitted slope's 95% interval covers 1, and "
            "the term beats no affinity by more than 1.92 nats",
            s_lo <= 1 <= s_hi and d0 > CHI1,
            f"slope {s_hat:.2f} [{s_lo:.2f}, {s_hi:.2f}]; over none {d0:.1f} nats",
        )
    )
    return out


def affinity_part_claims(full: list[s05_affinity.Trial]) -> list[Value | Finding]:
    """the two parts of A apart: relation points and 3 x shared G1 wins (race), each with its own slope"""
    sc_hat, sc_lo, sc_hi = fit1(lambda x: parts_llr(full, relation_slope=1.0, race_slope=x), 0.0, 3.0)
    sr_hat, sr_lo, sr_hi = fit1(lambda x: parts_llr(full, relation_slope=x, race_slope=1.0), 0.0, 3.0)
    out: list[Value | Finding] = [
        Value(SEC, "race part slope fitted", sc_hat),
        Value(SEC, "race part slope 95% low", sc_lo),
        Value(SEC, "race part slope 95% high", sc_hi),
        Value(SEC, "relation part slope fitted", sr_hat),
        Value(SEC, "relation part slope 95% low", sr_lo),
        Value(SEC, "relation part slope 95% high", sr_hi),
        Value(SEC, "trials with shared wins", sum(1 for t in full if t.race_part > 0)),
    ]
    out.append(
        common.finding(
            SEC,
            "Each part of A carries its assumed weight: with the other part fixed, the slope of the "
            "relation points and of 3 x shared G1 wins each has a 95% interval covering 1",
            sr_lo <= 1 <= sr_hi and sc_lo <= 1 <= sc_hi,
            f"relation {sr_hat:.2f} [{sr_lo:.2f}, {sr_hi:.2f}]; shared wins {sc_hat:.2f} [{sc_lo:.2f}, {sc_hi:.2f}]",
        )
    )
    return out


def star_claims(ts: list[Trial]) -> list[Value | Finding]:
    """obs / pred per star count (the base 3 / 6 / 9%)"""
    out: list[Value | Finding] = []
    miss: list[str] = []
    for b, st in STARS.items():
        c = ratio([t for t in ts if t.base == b])
        out += [
            Value(SEC, f"{st} star trials", c.trials),
            Value(SEC, f"{st} star obs / pred", c.ratio),
            Value(SEC, f"{st} star 95% low", c.low),
            Value(SEC, f"{st} star 95% high", c.high),
        ]
        if c.trials and not c.low <= 1 <= c.high:
            miss.append(f"{st} star {c.ratio:.3f} [{c.low:.2f}, {c.high:.2f}]")
    out.append(
        common.finding(
            SEC,
            "The base by stars (3 / 6 / 9%) holds for each star count: every interval covers 1",
            not miss,
            "; ".join(miss) or "all cover 1",
        )
    )
    return out


def band_claims(ts: list[Trial]) -> list[Value | Finding]:
    """obs / pred per affinity band, and the bands' joint run-clustered Wald test"""
    out: list[Value | Finding] = []
    miss: list[str] = []
    resid: list[tuple[tuple[int, float], int]] = []  # ((band, observed - predicted), run)
    for j, (a, b) in enumerate(BANDS):
        sub = [t for t in ts if a <= t.affinity < b]
        c = ratio(sub)
        resid += [((j, t.passed - pred(t.affinity, t.base)), t.run) for t in sub]
        lab = f"{a}+" if b > 10**5 else f"{a}-{b - 1}"
        out += [
            Value(SEC, f"affinity {lab} trials", c.trials),
            Value(SEC, f"affinity {lab} obs / pred", c.ratio),
            Value(SEC, f"affinity {lab} 95% low", c.low),
            Value(SEC, f"affinity {lab} 95% high", c.high),
        ]
        if c.trials >= 200 and not c.low <= 1 <= c.high:
            miss.append(f"{lab} {c.ratio:.3f} [{c.low:.2f}, {c.high:.2f}]")
    chi2, k, chi2_p = common.cluster_chi2([x for x, _ in resid], [run for _, run in resid])
    out += [Value(SEC, "affinity bands chi2", chi2), Value(SEC, "affinity bands chi2 p", chi2_p)]
    out.append(
        common.finding(
            SEC,
            "By affinity band the model holds jointly: chi2 over the five bands (5 degrees of freedom, "
            "no parameter fitted) has p above 0.05",
            chi2_p > 0.05,
            f"chi2 {chi2:.1f} on {k} bands, p {chi2_p:.3f}; outside their own 95% interval: "
            + ("; ".join(miss) or "none"),
        )
    )
    out.append(
        common.finding(
            SEC,
            "The bands outside their own 95% interval (as the text names them) are 150-199 and 200+",
            [m.split(" ")[0] for m in miss] == ["150-199", "200+"],
            "; ".join(miss) or "none",
        )
    )
    return out


if __name__ == "__main__":
    common.main(claims)
