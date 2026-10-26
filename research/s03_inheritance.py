"""Section 3: the inheritance roll's parts, tested on single-ancestor trials from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it).

A trial (built by common.trials) is one (ancestor, white skill spark, career year 2 or 3) where the spark's skill
is carried by exactly one of the six ancestors, so a year's inspiration of it can only come from her; it passes when the
factor shows among that year's inspirations. The model gives one roll per year, r = min(1, b(s) (1 + A / 100)),
b(s) = 3 / 6 / 9% by stars, A the ancestor's affinity (section 5, from master.mdb relation points and the record's
shared G1 wins). Ordinary ancestors only (no self-grandparent or grandparent equal to the parent), except one context
ratio on the parents whose own parent is the trainee's character (P_selfgp, tri(T, P, T) = 0 as section 5 fits).

Affinity and generation are confounded: grandparents' affinities stay below 100 and most parents' lie at 200 or
above, so a lower affinity slope and a flat parent shortfall predict nearly the same trials. Both one-parameter forms
are fitted (r scaled by a parent weight w_P; r = b (1 + s A / 100) with s free), compared with each other by
log-likelihood (same parameter count: rule 1 of How to read), each tested against the model in sample (rule 2: gain
over 1.92 nats, nominal LR p, and the run-clustered sandwich interval) and held out (rule 3: 5-fold cross-validation by
common.fold(run), common.compare_held_out, z > 1.96 establishes an improvement), and fitted together (whether either
adds a detectable gain to the other), on all trials and on parents alone (the one generation whose affinities span a
wide range). The slope is also fitted on grandparents alone, where generation does not vary.
Further tests: the grandparent weight (full base against halved), the slopes of A's two parts (each with the other at
1, then with the parent weight fitted), the base by stars (observed / predicted per star count, all trials and below
affinity 200) and per affinity band (each band's interval; the run-clustered Wald chi2 over the bands below 200; and,
there, one-degree-of-freedom tests of a common weight and a slope). Parents at affinity 200 or above read below the
model, a known miss stated with its size. A sum's interval is observed / predicted by the run-clustered ratio estimator
(common.cluster_ci: a run's trials share its lineage); fitted parameters carry run-clustered (sandwich) 95% intervals.
"""

import collections
import itertools
from collections.abc import Callable
from typing import NamedTuple
import numpy as np
from scipy.optimize import minimize, minimize_scalar
from scipy.stats import chi2
import common
from common import Finding, Value

SEC: str = "3"
CHI1: float = 1.920729  # chi2(1) 95% / 2: a likelihood gain beyond it is detectable (rule 2)
OPTIMISM: float = 1.0  # nats: the expected in-sample over held-out gain of one fitted parameter that changes nothing
STARS: dict[float, int] = {0.03: 1, 0.06: 2, 0.09: 3}
BANDS: list[tuple[int, int]] = [(0, 50), (50, 100), (100, 150), (150, 200), (200, 10**6)]
MISS_FROM: int = 200  # the affinity from which parents read below the model: the known miss, the top band's edge


class Roll(NamedTuple):
    """the roll's free parameters: r = min(1, w b (1 + (s_rel (A - race) + s_race race) / 100)), w by generation.
    The model has every field at 1."""

    parent_weight: float = 1.0
    grandparent_weight: float = 1.0
    relation_slope: float = 1.0  # on A's relation points
    race_slope: float = 1.0  # on A's 3 x shared G1 wins


MODEL: Roll = Roll()


def slope(s: float) -> Roll:
    """the roll with affinity slope s on all of A"""
    return Roll(relation_slope=s, race_slope=s)


class Trials(NamedTuple):
    """single-ancestor trials as parallel arrays, index i one trial (from common.Trial, tri(T, P, T) = 0)"""

    parent: np.ndarray  # True for a parent's trial, False for a grandparent's
    affinity: np.ndarray  # A
    race: np.ndarray  # A's shared-win part, 3 x shared G1 wins
    base: np.ndarray  # b by the spark's stars
    passed: np.ndarray  # 1.0 when the spark fired, else 0.0
    run: np.ndarray  # the run id

    def where(self, mask: np.ndarray) -> Trials:
        """the trials where mask holds"""
        return Trials(*(a[mask] for a in self))

    def chance(self, roll: Roll = MODEL) -> np.ndarray:
        """each trial's r under roll"""
        weight = np.where(self.parent, roll.parent_weight, roll.grandparent_weight)
        a = roll.relation_slope * (self.affinity - self.race) + roll.race_slope * self.race
        return np.minimum(1.0, weight * self.base * (1 + a / 100))

    def run_ll(self, roll: Roll = MODEL) -> dict[int, float]:
        """{run id: log-likelihood of that run's trials}, each chance clipped to [1e-12, 1 - 1e-12]"""
        p = np.clip(self.chance(roll), 1e-12, 1 - 1e-12)
        each = self.passed * np.log(p) + (1 - self.passed) * np.log(1 - p)
        runs, index = np.unique(self.run, return_inverse=True)
        return dict(zip(runs.tolist(), np.bincount(index, weights=each).tolist()))

    def ll(self, roll: Roll = MODEL) -> float:
        """log-likelihood of every trial"""
        return sum(self.run_ll(roll).values())


def to_trials(ts: list[common.Trial]) -> Trials:
    return Trials(
        np.array([t.category.startswith("P") for t in ts]),
        np.array([t.affinity_h0 for t in ts], float),
        np.array([t.race_part for t in ts], float),
        np.array([t.base for t in ts]),
        np.array([t.passed for t in ts], float),
        np.array([t.run for t in ts]),
    )


class Ratio(NamedTuple):
    """observed / predicted passes of a set of trials under the model (ratio())"""

    trials: int
    observed: int
    predicted: float
    ratio: float
    low: float  # its run-clustered 95% interval
    high: float


class Fit(NamedTuple):
    """a one-parameter maximum-likelihood fit (fit1())"""

    estimate: float
    low: float  # run-clustered 95% interval
    high: float
    se: float  # run-clustered standard error
    gain: float  # in-sample log-likelihood over the model's, nats

    def wald_z(self) -> float:
        """(estimate - 1) / se: the run-clustered distance from the model's 1"""
        return (self.estimate - 1) / self.se

    def lr_p(self) -> float:
        """the nominal likelihood-ratio p against the model (chi2 on 1 degree of freedom)"""
        return float(chi2.sf(2 * self.gain, 1))


# a one-parameter form of the roll: the parameter -> the roll
Form = Callable[[float], Roll]


def fit1(ts: Trials, form: Form, lo: float = 0.0, hi: float = 3.0) -> Fit:
    """the maximum-likelihood parameter on [lo, hi] with its run-clustered 95% interval: a run gives many trials, so
    the interval is the sandwich one (common.robust_interval), not the likelihood interval of independent trials"""
    r = minimize_scalar(lambda x: -ts.ll(form(x)), bounds=(lo, hi), method="bounded", options={"xatol": 1e-5})
    x = float(r.x)
    low, high, se = common.robust_interval(lambda v: ts.run_ll(form(v)), x)
    return Fit(x, low, high, se, -float(r.fun) - ts.ll())


def fit2(ts: Trials, form: Callable[[float, float], Roll]) -> tuple[float, float, float]:
    """(first, second, in-sample gain over the model in nats): a two-parameter maximum-likelihood fit from (1, 1)"""
    r = minimize(lambda v: -ts.ll(form(v[0], v[1])), [1.0, 1.0], method="Nelder-Mead", options={"xatol": 1e-5})
    return float(r.x[0]), float(r.x[1]), -float(r.fun) - ts.ll()


def held_out(ts: Trials, form: Form) -> common.HeldOutComparison:
    """a refitted parameter against the model's 1, 5-fold held out by run: each fold's runs are scored by the parameter
    fitted on the other four folds and by the model; common.compare_held_out(refitted, model), so nats > 0 favour the
    refit"""
    fold = np.array([common.fold(int(r)) for r in ts.run])
    refit: dict[int, float] = {}
    model: dict[int, float] = {}
    for k in range(common.K_FOLDS):
        test = ts.where(fold == k)
        refit |= test.run_ll(form(fit1_estimate(ts.where(fold != k), form)))
        model |= test.run_ll()
    runs = sorted(model)
    return common.compare_held_out([refit[r] for r in runs], [model[r] for r in runs])


def fit1_estimate(ts: Trials, form: Form) -> float:
    """the maximum-likelihood parameter on [0, 3] (no interval)"""
    nll = lambda x: -ts.ll(form(x))
    return float(minimize_scalar(nll, bounds=(0.0, 3.0), method="bounded", options={"xatol": 1e-5}).x)


def fit_values(name: str, f: Fit) -> list[Value | Finding]:
    """a fit's estimate, interval, in-sample gain, nominal LR p and run-clustered z as values"""
    return [
        Value(SEC, f"{name} fitted", f.estimate),
        Value(SEC, f"{name} 95% low", f.low),
        Value(SEC, f"{name} 95% high", f.high),
        Value(SEC, f"{name} in sample over 1, nats", f.gain),
        Value(SEC, f"{name} LR p", f.lr_p()),
        Value(SEC, f"{name} run-clustered z", f.wald_z()),
    ]


def held_out_values(name: str, model: str, h: common.HeldOutComparison) -> list[Value | Finding]:
    """a held-out comparison's nats, standard error and z as values"""
    return [
        Value(SEC, f"{name} refitted over {model} held out, nats", h.nats),
        Value(SEC, f"{name} refitted over {model} held out, se", h.se),
        Value(SEC, f"{name} refitted over {model} held out, z", h.z),
    ]


def fit_text(f: Fit) -> str:
    return f"{f.estimate:.3f} [{f.low:.3f}, {f.high:.3f}], in sample {f.gain:+.2f} nats (LR p {f.lr_p():.1g})"


def held_text(h: common.HeldOutComparison) -> str:
    return f"{h.nats:+.2f} +- {h.se:.2f} nats, z {h.z:.2f}"


def ratio(ts: Trials) -> Ratio:
    """(trials, observed, predicted, obs / pred, low, high) under the model; the interval is run-clustered
    (common.cluster_ci)"""
    p = ts.chance()
    c = common.cluster_ci(ts.passed.tolist(), p.tolist(), ts.run.tolist())
    o = int(ts.passed.sum())
    return Ratio(len(p), o, float(p.sum()), common.div(o, float(p.sum())), c.low, c.high)


def ratio_values(name: str, c: Ratio) -> list[Value | Finding]:
    return [
        Value(SEC, f"{name} trials", c.trials),
        Value(SEC, f"{name} obs / pred", c.ratio),
        Value(SEC, f"{name} 95% low", c.low),
        Value(SEC, f"{name} 95% high", c.high),
    ]


def ratio_text(c: Ratio) -> str:
    return f"{c.ratio:.3f} [{c.low:.3f}, {c.high:.3f}] ({c.trials:,} trials)"


def claims() -> list[Value | Finding]:
    R = common.runs()
    if R is None:
        return []
    full = common.trials(R, race=True)
    ts = to_trials([t for t in full if t.category in ("P", "G")])
    self_gp_parents = to_trials([t for t in full if t.category == "P_selfgp"])
    form_out, forms = form_claims(ts)
    return (
        generation_claims(ts)
        + form_out
        + shortfall_claims(ts, self_gp_parents, forms)
        + affinity_part_claims(ts)
        + star_claims(ts)
        + band_claims(ts, forms)
        + pair_claims(R)
    )


def generation_claims(ts: Trials) -> list[Value | Finding]:
    """the trial counts by generation, how far affinity and generation overlap, and the grandparent weight (full base
    against halved)"""
    P, G = ts.where(ts.parent), ts.where(~ts.parent)
    below = ts.where(ts.affinity < MISS_FROM)
    g_max = float(G.affinity.max())
    p_high_share = float((P.affinity >= MISS_FROM).mean())
    g_share_below = float((~below.parent).mean())
    out: list[Value | Finding] = [
        Value(SEC, "trials, parents", len(P.run)),
        Value(SEC, "trials, grandparents", len(G.run)),
        Value(SEC, "grandparent trials: highest affinity", g_max),
        Value(SEC, "parent trials: lowest affinity", float(P.affinity.min())),
        Value(SEC, "parent trials: median affinity", float(np.median(P.affinity))),
        Value(SEC, "parent trials at affinity 200 and above, share", p_high_share),
        Value(SEC, "trials below affinity 200, parents", int(below.parent.sum())),
        Value(SEC, "trials below affinity 200, grandparents' share", g_share_below),
    ]
    out.append(
        common.finding(
            SEC,
            "Affinity and generation are confounded: every grandparent trial lies below affinity 100, over four fifths "
            "of parent trials lie at 200 or above, and over nine in ten trials below 200 are grandparents'",
            g_max < 100 and p_high_share > 0.8 and g_share_below > 0.9,
            f"grandparents' highest affinity {g_max:.0f}; parents' lowest {P.affinity.min():.0f}, median "
            f"{np.median(P.affinity):.0f}, at 200+ {p_high_share:.1%}; below 200, {int(below.parent.sum()):,} parent "
            f"trials, grandparents {g_share_below:.1%}",
        )
    )
    d_half = ts.ll() - ts.ll(Roll(grandparent_weight=0.5))
    kg = fit1(ts, lambda k: Roll(grandparent_weight=k), 0.2, 2.0)
    out += [
        Value(SEC, "grandparents full over halved, nats", d_half),
        Value(SEC, "grandparent weight fitted", kg.estimate),
        Value(SEC, "grandparent weight 95% low", kg.low),
        Value(SEC, "grandparent weight 95% high", kg.high),
    ]
    out.append(
        common.finding(
            SEC,
            "Grandparents take the full base: the fitted grandparent weight's 95% interval covers 1 and "
            "excludes 0.5, and the full base beats the halved one by more than 1.92 nats",
            kg.low <= 1 <= kg.high and not kg.low <= 0.5 <= kg.high and d_half > CHI1,
            f"weight {kg.estimate:.2f} [{kg.low:.2f}, {kg.high:.2f}] on {len(G.run):,} trials; full over halved "
            f"{d_half:.1f} nats",
        )
    )
    return out


class Forms(NamedTuple):
    """the two one-parameter forms of the parents' shortfall, fitted and held out (form_claims)"""

    slope: Fit
    weight: Fit  # the parent weight w_P
    slope_held: common.HeldOutComparison
    weight_held: common.HeldOutComparison


def form_claims(ts: Trials) -> tuple[list[Value | Finding], Forms]:
    """the affinity slope s and the parent weight w_P: each fitted, in sample and held out against 1, the two compared
    (same parameter count) and fitted together; affinity against none; the slope on grandparents alone"""
    d0 = ts.ll() - ts.ll(slope(0.0))
    weight_form: Form = lambda w: Roll(parent_weight=w)
    f = Forms(fit1(ts, slope), fit1(ts, weight_form), held_out(ts, slope), held_out(ts, weight_form))
    both_w, both_s, both_gain = fit2(ts, lambda w, s: Roll(parent_weight=w, relation_slope=s, race_slope=s))
    gp_slope = fit1(ts.where(~ts.parent), slope)
    P = ts.where(ts.parent)
    p_slope, p_weight = fit1(P, slope), fit1(P, weight_form)
    p_both_w, p_both_s, p_both_gain = fit2(P, lambda w, s: Roll(parent_weight=w, relation_slope=s, race_slope=s))
    out: list[Value | Finding] = [
        *fit_values("affinity slope", f.slope),
        Value(SEC, "affinity over none, nats", d0),
        *held_out_values("affinity slope", "slope 1", f.slope_held),
        *fit_values("parent weight", f.weight),
        *held_out_values("parent weight", "weight 1", f.weight_held),
        Value(SEC, "parent weight and slope together: parent weight", both_w),
        Value(SEC, "parent weight and slope together: slope", both_s),
        Value(SEC, "parent weight and slope together over the model, nats", both_gain),
        Value(SEC, "slope added to the parent weight, nats", both_gain - f.weight.gain),
        Value(SEC, "parent weight added to the slope, nats", both_gain - f.slope.gain),
        Value(SEC, "grandparents alone: affinity slope fitted", gp_slope.estimate),
        Value(SEC, "grandparents alone: affinity slope 95% low", gp_slope.low),
        Value(SEC, "grandparents alone: affinity slope 95% high", gp_slope.high),
        Value(SEC, "parents alone: affinity slope fitted", p_slope.estimate),
        Value(SEC, "parents alone: affinity slope 95% low", p_slope.low),
        Value(SEC, "parents alone: affinity slope 95% high", p_slope.high),
        Value(SEC, "parents alone: affinity slope in sample over 1, nats", p_slope.gain),
        Value(SEC, "parents alone: parent weight in sample over 1, nats", p_weight.gain),
        Value(SEC, "parents alone: together, parent weight", p_both_w),
        Value(SEC, "parents alone: together, slope", p_both_s),
        Value(SEC, "parents alone: together over the model, nats", p_both_gain),
        Value(SEC, "parents alone: slope added to the parent weight, nats", p_both_gain - p_weight.gain),
        Value(SEC, "parents alone: parent weight added to the slope, nats", p_both_gain - p_slope.gain),
    ]
    out.append(
        common.finding(
            SEC,
            "Affinity raises the roll, and a slope below 1 is detectable in sample but not established held out: the "
            "term beats no affinity by more than 1.92 nats; a refitted slope gains more than 1.92 nats over slope 1 "
            "with its run-clustered 95% interval below 1 (rule 2); its held-out improvement has z <= 1.96 (rule 3)",
            d0 > CHI1 and f.slope.gain > CHI1 and f.slope.high < 1 and f.slope_held.z <= 1.96,
            f"over none {d0:.1f} nats; slope {fit_text(f.slope)}, run-clustered z {f.slope.wald_z():.2f}; held out "
            f"{held_text(f.slope_held)}",
        )
    )
    out.append(
        common.finding(
            SEC,
            "A flat parent weight below 1 is detectable in sample but not established held out: it gains more than "
            "1.92 nats over weight 1 with its run-clustered 95% interval below 1 (rule 2); its held-out improvement "
            "has z <= 1.96 (rule 3)",
            f.weight.gain > CHI1 and f.weight.high < 1 and f.weight_held.z <= 1.96,
            f"parent weight {fit_text(f.weight)}, run-clustered z {f.weight.wald_z():.2f}; held out "
            f"{held_text(f.weight_held)}",
        )
    )
    out.append(
        common.finding(
            SEC,
            "The trials cannot separate a flat parent weight from an affinity slope: with one parameter each (rule 1, "
            "descriptive), the parent weight fits better over all trials, where the slope also moves the "
            "grandparents, and the slope fits better on parents alone, the one set with a wide range of affinity; "
            "fitted together, on all trials and on parents alone, neither adds a detectable gain to the other (each "
            "below 1.92 nats)",
            f.weight.gain > f.slope.gain
            and p_slope.gain > p_weight.gain
            and max(both_gain - f.weight.gain, both_gain - f.slope.gain) < CHI1
            and max(p_both_gain - p_weight.gain, p_both_gain - p_slope.gain) < CHI1,
            f"all trials: parent weight {f.weight.gain:+.2f} nats, slope {f.slope.gain:+.2f}; together (w_P "
            f"{both_w:.3f}, s {both_s:.3f}) {both_gain:+.2f}: the slope adds {both_gain - f.weight.gain:+.2f}, the "
            f"weight {both_gain - f.slope.gain:+.2f} | parents alone: weight {p_weight.gain:+.2f}, slope "
            f"{p_slope.gain:+.2f}; together (w_P {p_both_w:.3f}, s {p_both_s:.3f}) {p_both_gain:+.2f}: the slope adds "
            f"{p_both_gain - p_weight.gain:+.2f}, the weight {p_both_gain - p_slope.gain:+.2f}",
        )
    )
    near = lambda fit, h: abs(h.nats - (fit.gain - OPTIMISM)) < h.se
    expected_z = lambda fit, h: (fit.gain - OPTIMISM) / h.se  # the held-out z an effect of the in-sample size gives
    out += [
        Value(SEC, "affinity slope expected held-out z", expected_z(f.slope, f.slope_held)),
        Value(SEC, "parent weight expected held-out z", expected_z(f.weight, f.weight_held)),
    ]
    out.append(
        common.finding(
            SEC,
            "The held-out test lacks the power to establish effects of the in-sample size: each held-out gain lies "
            "within one held-out standard error of its in-sample gain less one nat (the expected optimism of one "
            "fitted parameter), and that expected gain over the held-out standard error is below 1.96",
            all(
                near(fit, h) and expected_z(fit, h) < 1.96
                for fit, h in ((f.slope, f.slope_held), (f.weight, f.weight_held))
            ),
            f"slope: in sample {f.slope.gain:+.2f}, expected z {expected_z(f.slope, f.slope_held):.2f}, held out "
            f"{held_text(f.slope_held)}; parent weight: in sample {f.weight.gain:+.2f}, expected z "
            f"{expected_z(f.weight, f.weight_held):.2f}, held out {held_text(f.weight_held)}",
        )
    )
    out.append(
        common.finding(
            SEC,
            "Among grandparents alone, where generation does not vary (affinity below 100), no slope away from 1 is "
            "detected: the slope's run-clustered 95% interval covers 1",
            gp_slope.low <= 1 <= gp_slope.high,
            f"grandparents {fit_text(gp_slope)}; parents alone {fit_text(p_slope)}",
        )
    )
    return out, f


def shortfall_claims(ts: Trials, self_gp_parents: Trials, forms: Forms) -> list[Value | Finding]:
    """the known miss, parents at affinity 200 or above; all parents; parents whose own parent is the trainee's
    character"""
    P = ts.where(ts.parent)
    below = ratio(P.where(P.affinity < MISS_FROM))
    above = ratio(P.where(P.affinity >= MISS_FROM))
    parents = ratio(P)
    self_gp = ratio(self_gp_parents)
    out: list[Value | Finding] = [
        Value(SEC, "parents at affinity 200 and above obs / pred", above.ratio),
        Value(SEC, "parents at affinity 200 and above 95% low", above.low),
        Value(SEC, "parents at affinity 200 and above 95% high", above.high),
        *ratio_values("parents", parents),
        *ratio_values("parents with a self-grandparent", self_gp),
    ]
    out.append(
        common.finding(
            SEC,
            "Known miss: the model over-predicts parents at affinity 200 or above (observed / predicted, 95% interval "
            "below 1), and no refit is established held out (slope and parent weight, z <= 1.96 each)",
            above.high < 1 and forms.slope_held.z <= 1.96 and forms.weight_held.z <= 1.96,
            f"parents at 200+ {ratio_text(above)}, below 200 {ratio_text(below)}, all {ratio_text(parents)}; held "
            f"out: slope {held_text(forms.slope_held)}, parent weight {held_text(forms.weight_held)}",
        )
    )
    every = int((ts.affinity >= MISS_FROM).sum())
    out.append(
        common.finding(
            SEC,
            "Every trial at affinity 200 or above is a parent's, so the top band is the parents' known miss",
            above.trials == every,
            f"{above.trials:,} parents' trials of {every:,} at 200+",
        )
    )
    out.append(
        common.finding(
            SEC,
            "Parents whose own parent is the trainee's character, as in the planner's rotations, read below the "
            "model too (tri(T, P, T) = 0, section 5): their 95% interval lies below 1",
            self_gp.high < 1,
            f"{ratio_text(self_gp)}; ordinary parents {ratio_text(parents)}",
        )
    )
    return out


def affinity_part_claims(ts: Trials) -> list[Value | Finding]:
    """the two parts of A apart: relation points and 3 x shared G1 wins (race), each with its own slope, fitted and
    held out against weight 1 with the other part fixed, then each fitted with the parent weight"""
    race_form: Form = lambda x: Roll(race_slope=x)
    relation_form: Form = lambda x: Roll(relation_slope=x)
    race, relation = fit1(ts, race_form), fit1(ts, relation_form)
    race_held, rel_held = held_out(ts, race_form), held_out(ts, relation_form)
    weight_gain = fit1(ts, lambda w: Roll(parent_weight=w)).gain
    _, rel_w, rel_w_gain = fit2(ts, lambda w, x: Roll(parent_weight=w, relation_slope=x))
    _, race_w, race_w_gain = fit2(ts, lambda w, x: Roll(parent_weight=w, race_slope=x))
    out: list[Value | Finding] = [
        Value(SEC, "race part slope fitted", race.estimate),
        Value(SEC, "race part slope 95% low", race.low),
        Value(SEC, "race part slope 95% high", race.high),
        Value(SEC, "relation part slope fitted", relation.estimate),
        Value(SEC, "relation part slope 95% low", relation.low),
        Value(SEC, "relation part slope 95% high", relation.high),
        *held_out_values("race part slope", "slope 1", race_held),
        *held_out_values("relation part slope", "slope 1", rel_held),
        Value(SEC, "relation part slope with the parent weight fitted", rel_w),
        Value(SEC, "relation part slope added to the parent weight, nats", rel_w_gain - weight_gain),
        Value(SEC, "race part slope with the parent weight fitted", race_w),
        Value(SEC, "race part slope added to the parent weight, nats", race_w_gain - weight_gain),
    ]
    out.append(
        common.finding(
            SEC,
            "Each part of A reads below its assumed weight in sample, and neither refit is established held out: with "
            "the other part fixed, each slope's run-clustered 95% interval lies below 1; neither refitted slope "
            "predicts unseen runs better than weight 1 (5-fold held out by run, z <= 1.96 each)",
            relation.high < 1 and race.high < 1 and rel_held.z <= 1.96 and race_held.z <= 1.96,
            f"relation {fit_text(relation)}, held out {held_text(rel_held)}; shared wins {fit_text(race)}, held out "
            f"{held_text(race_held)}",
        )
    )
    out.append(
        common.finding(
            SEC,
            "The parts' shortfalls are the parents' shortfall: with the parent weight fitted, neither part's slope "
            "adds a detectable gain (each below 1.92 nats)",
            rel_w_gain - weight_gain < CHI1 and race_w_gain - weight_gain < CHI1,
            f"with w_P: relation slope {rel_w:.3f} ({rel_w_gain - weight_gain:+.2f} nats), shared-win slope "
            f"{race_w:.3f} ({race_w_gain - weight_gain:+.2f} nats)",
        )
    )
    return out


def star_claims(ts: Trials) -> list[Value | Finding]:
    """obs / pred per star count (the base 3 / 6 / 9%), on every trial and below MISS_FROM: the base is tested where
    the affinity term holds, since the trials at MISS_FROM or above are the known miss of the affinity term"""
    out: list[Value | Finding] = []
    below: list[str] = []  # each star count below MISS_FROM
    below_cover = True  # every star count's interval below MISS_FROM covers 1
    outside: list[
        str
    ] = []  # star counts outside their interval on every trial, with their trials at MISS_FROM or above
    explained = True  # every such count's trials at MISS_FROM or above read below 1
    for b, st in STARS.items():
        S = ts.where(ts.base == b)
        c = ratio(S)
        lo = ratio(S.where(S.affinity < MISS_FROM))
        out += [
            Value(SEC, f"{st} star trials", c.trials),
            Value(SEC, f"{st} star obs / pred", c.ratio),
            Value(SEC, f"{st} star 95% low", c.low),
            Value(SEC, f"{st} star 95% high", c.high),
            Value(SEC, f"{st} star below {MISS_FROM} trials", lo.trials),
            Value(SEC, f"{st} star below {MISS_FROM} obs / pred", lo.ratio),
            Value(SEC, f"{st} star below {MISS_FROM} 95% low", lo.low),
            Value(SEC, f"{st} star below {MISS_FROM} 95% high", lo.high),
        ]
        below.append(f"{st} star {ratio_text(lo)}")
        below_cover &= lo.low <= 1 <= lo.high
        if c.trials and not c.low <= 1 <= c.high:
            hi = ratio(S.where(S.affinity >= MISS_FROM))
            explained &= hi.high < 1
            outside.append(
                f"{st} star {c.ratio:.3f} [{c.low:.3f}, {c.high:.3f}] on every trial; at {MISS_FROM}+ {ratio_text(hi)}"
            )
    out.append(Value(SEC, "star counts outside their own 95% interval on every trial", len(outside)))
    out.append(
        common.finding(
            SEC,
            f"The base by stars (3 / 6 / 9%) holds for each star count where the affinity term holds: below affinity "
            f"{MISS_FROM} every star count's interval covers 1; a star count whose interval on every trial excludes 1 "
            f"does so through its trials at {MISS_FROM} or above, the known miss (their interval entirely below 1)",
            below_cover and explained,
            f"below {MISS_FROM}: " + "; ".join(below) + " | every trial: " + ("; ".join(outside) or "all cover 1"),
        )
    )
    return out


def band_claims(ts: Trials, forms: Forms) -> list[Value | Finding]:
    """obs / pred per affinity band, the bands' joint run-clustered Wald test, and below MISS_FROM one-degree-of-freedom
    tests with a direction: a common weight on every trial's roll and an affinity slope"""
    out: list[Value | Finding] = []
    miss: list[str] = []
    resid: list[tuple[tuple[int, float], int]] = []  # ((band, observed - predicted), run)
    for j, (a, b) in enumerate(BANDS):
        sub = ts.where((ts.affinity >= a) & (ts.affinity < b))
        c = ratio(sub)
        resid += [((j, float(d)), int(run)) for d, run in zip(sub.passed - sub.chance(), sub.run)]
        lab = f"{a}+" if b > 10**5 else f"{a}-{b - 1}"
        out += [
            Value(SEC, f"affinity {lab} trials", c.trials),
            Value(SEC, f"affinity {lab} obs / pred", c.ratio),
            Value(SEC, f"affinity {lab} 95% low", c.low),
            Value(SEC, f"affinity {lab} 95% high", c.high),
        ]
        if c.trials >= 200 and not c.low <= 1 <= c.high:
            miss.append(f"{lab} {c.ratio:.3f} [{c.low:.2f}, {c.high:.2f}]")
    chi2_all, k, chi2_p = common.cluster_chi2([x for x, _ in resid], [run for _, run in resid])
    top = len(BANDS) - 1  # the band from MISS_FROM up: the known miss
    assert BANDS[top][0] == MISS_FROM
    rest = [(x, run) for x, run in resid if x[0] < top]
    chi2_rest, k_rest, p_rest = common.cluster_chi2([x for x, _ in rest], [run for _, run in rest])
    below = ts.where(ts.affinity < MISS_FROM)
    level = fit1(below, lambda w: Roll(parent_weight=w, grandparent_weight=w), 0.2, 2.0)
    tilt = fit1(below, slope)
    parents_below = ratio(below.where(below.parent))
    outside = ", ".join(m.split(" ")[0] for m in miss) or "none"
    out += [
        Value(SEC, "affinity bands chi2", chi2_all),
        Value(SEC, "affinity bands chi2 p", chi2_p),
        Value(SEC, "affinity bands below 200 chi2", chi2_rest),
        Value(SEC, "affinity bands below 200 chi2 p", p_rest),
        Value(SEC, "affinity bands outside their own 95% interval", outside),
        Value(SEC, "below affinity 200: common weight fitted", level.estimate),
        Value(SEC, "below affinity 200: common weight 95% low", level.low),
        Value(SEC, "below affinity 200: common weight 95% high", level.high),
        Value(SEC, "below affinity 200: affinity slope fitted", tilt.estimate),
        Value(SEC, "below affinity 200: affinity slope 95% low", tilt.low),
        Value(SEC, "below affinity 200: affinity slope 95% high", tilt.high),
        *ratio_values("parents below affinity 200", parents_below),
    ]
    out.append(
        common.finding(
            SEC,
            "Below affinity 200, where nine in ten trials are grandparents', the tests mostly re-test the grandparents "
            "and cannot tell the formula from the parents' shortfall: a fitted common weight's and a fitted slope's "
            "run-clustered 95% intervals each cover 1 (one degree of freedom each), the slope's also covers the "
            "slope fitted on all trials, and the parents below 200 read an interval covering both 1 and the fitted "
            "parent weight; the four bands' chi2 and the five bands' chi2 are reported",
            level.low <= 1 <= level.high
            and tilt.low <= 1 <= tilt.high
            and tilt.low <= forms.slope.estimate <= tilt.high
            and parents_below.low <= min(1.0, forms.weight.estimate)
            and parents_below.high >= 1,
            f"below 200: weight {fit_text(level)}, slope {fit_text(tilt)} (all trials {forms.slope.estimate:.3f}), "
            f"parents {ratio_text(parents_below)} (parent weight {forms.weight.estimate:.3f}); chi2 {chi2_rest:.1f} "
            f"on {k_rest} bands, p {p_rest:.3f}; all five: chi2 {chi2_all:.1f} on {k} bands, p {chi2_p:.3f}",
        )
    )
    out.append(
        common.finding(
            SEC,
            "Every affinity band outside its own 95% interval lies at 200 or above, the known miss (the bands outside "
            "are named from the computed list)",
            all(m.split(" ")[0] == f"{MISS_FROM}+" for m in miss),
            "; ".join(miss) or "none",
        )
    )
    return out


PAIR_BOOTSTRAP: int = 2000  # run-clustered bootstrap draws for the pair ratios
PAIR_KINDS: tuple[str, ...] = ("one ancestor", "two ancestors")
GENERATIONS: tuple[str, ...] = ("P", "G")
# one run's sums, as one row: per kind and generation pair (GG, GP, PP) both shown and the model's chance of both,
# then per generation each single-carrier spark-year's shown count and chance (the marginals, counted once each)
PAIR_CLASSES: tuple[tuple[str, str], ...] = (("G", "G"), ("G", "P"), ("P", "P"))  # each sorted
PAIR_COLUMNS: int = 2 * len(PAIR_KINDS) * len(PAIR_CLASSES) + 2 * len(GENERATIONS)


def pair_column(kind: str, generations: tuple[str, str]) -> int:
    """the column of a kind and generation pair's 'both shown'; its model chance is the next column"""
    return 2 * (PAIR_KINDS.index(kind) * len(PAIR_CLASSES) + PAIR_CLASSES.index(tuple(sorted(generations))))


def marginal_column(generation: str) -> int:
    """the column of a generation's shown count; its model chance is the next column"""
    return 2 * len(PAIR_KINDS) * len(PAIR_CLASSES) + 2 * GENERATIONS.index(generation)


def pair_sums(R: list[common.Run]) -> np.ndarray:
    """per run, the pair sums (PAIR_COLUMNS): pairs of white skill sparks, each carried by exactly one of the six
    ordinary ancestors (the trials' rule), in the same run and career year, so one inspiration rolls both"""
    S, ftype = common.relations()
    rows = []
    for r in R:
        anc = common.lineage(r)
        if anc is None:
            continue
        T = r["chara"]
        A = [a["chara"] for a in anc]
        aff = r.get("affinity") or {}
        yrs = r.get("years") or {}
        if len({T, A[0], A[3]}) < 3 or not common.has_affinity_rows(aff) or "2" not in yrs or "3" not in yrs:
            continue
        affinity = common.six_affinities(T, A, aff, S, h=0)
        sparks = [{f // 100: f for f, _ in (a.get("factors") or [])} for a in anc]
        carriers = collections.Counter(n for s in sparks for n in s)
        single = [
            (slot, common.category(slot, A, T), f, min(1.0, common.BASE[f % 10 - 1] * (1 + affinity[slot] / 100)))
            for slot in range(6)
            if common.category(slot, A, T) in GENERATIONS
            for n, f in sparks[slot].items()
            if ftype.get(f) == 4 and carriers[n] == 1
        ]
        row = np.zeros(PAIR_COLUMNS)
        for y in ("2", "3"):
            shown = {f for f, _ in yrs[y]}
            for _, generation, f, chance in single:
                row[marginal_column(generation)] += f in shown
                row[marginal_column(generation) + 1] += chance
            for (slot_a, gen_a, fa, ra), (slot_b, gen_b, fb, rb) in itertools.combinations(single, 2):
                column = pair_column("one ancestor" if slot_a == slot_b else "two ancestors", (gen_a, gen_b))
                row[column] += fa in shown and fb in shown
                row[column + 1] += ra * rb
        if row[: 2 * len(PAIR_KINDS) * len(PAIR_CLASSES)].any():
            rows.append(row)
    return np.array(rows)


def pair_ratios(rows: np.ndarray) -> dict[str, float]:
    """per kind, both shown over the model's chance of both with each spark's chance scaled by her generation's
    observed / predicted (so the parents' known shortfall, or any calibration gap between generations, does not read
    as dependence)"""
    t = rows.sum(axis=0)
    scale = {g: t[marginal_column(g)] / t[marginal_column(g) + 1] for g in GENERATIONS}
    out = {}
    for kind in PAIR_KINDS:
        both = sum(t[pair_column(kind, c)] for c in PAIR_CLASSES)
        chance = sum(t[pair_column(kind, c) + 1] * scale[c[0]] * scale[c[1]] for c in PAIR_CLASSES)
        out[kind] = float(both / chance)
    return out


def pair_claims(R: list[common.Run]) -> list[Value | Finding]:
    """whether a per-ancestor trigger shows: an ancestor active in an inspiration with chance g, her sparks then firing
    with chance r / g, leaves each marginal at r and raises a pair on her to r_i r_j / g, so the one-ancestor ratio
    reads 1 / g while the two-ancestor pairs, a built-in control, stay at 1 (derived)"""
    rows = pair_sums(R)
    rng = np.random.default_rng(3)
    draws = [pair_ratios(rows[rng.integers(0, len(rows), len(rows))]) for _ in range(PAIR_BOOTSTRAP)]
    estimate = pair_ratios(rows)
    out: list[Value | Finding] = [Value(SEC, "spark pairs runs", len(rows))]
    texts, covers, bounds = [], True, {}
    for kind in PAIR_KINDS:
        low, high = (float(np.quantile([d[kind] for d in draws], q)) for q in (0.025, 0.975))
        both = sum(rows[:, pair_column(kind, c)].sum() for c in PAIR_CLASSES)
        out += [
            Value(SEC, f"spark pairs on {kind} both shown", float(both)),
            Value(SEC, f"spark pairs on {kind} obs / pred", estimate[kind]),
            Value(SEC, f"spark pairs on {kind} 95% low", low),
            Value(SEC, f"spark pairs on {kind} 95% high", high),
        ]
        texts.append(f"{kind} {estimate[kind]:.3f} [{low:.3f}, {high:.3f}] ({both:,.0f} both shown)")
        covers &= low <= 1 <= high
        bounds[kind] = high
    gate = 1 / bounds["one ancestor"]  # the weakest trigger the one-ancestor interval excludes, under its model
    out.append(Value(SEC, "spark pairs per-ancestor trigger excluded below", gate))
    out.append(
        common.finding(
            SEC,
            "No per-ancestor trigger is detected within one inspiration: pairs of single-carrier white sparks show "
            "together as often as the product of their generation-calibrated chances predicts, on one ancestor and on "
            "two (each interval covers 1)",
            covers,
            "; ".join(texts) + f"; a trigger with g below {gate:.3f} is excluded",
        )
    )
    return out


if __name__ == "__main__":
    common.main(claims)
