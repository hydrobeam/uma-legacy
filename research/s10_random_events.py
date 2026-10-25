"""Section 10: how often a support card's character's shared random event fires, from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it), and what leaving these events out of
the model does on real careers. Every run record is used; one sample.

Markers (GameTora training_events_shared, through gametora.py): a character's shared event with two or more choices
whose top choice has a skill common to every outcome and whose other choices have no skill common to all their
outcomes; the top choice's skill is her marker (one per character). A reading is one run with an SR or SSR card of
hers in the deck (R cards are left out of the measured rate, a project rule; group cards aside), someone else
training, and the marker not given by that card's own events; a hit is the marker credited to "<her name> event" or to
"A or <her name> event", section 8's rule: a reading is unknown, and left out whatever its credits, when another deck
card's events (its own, section 8's skill pools, or its character's shared events) can give the marker. A reading
whose marker is already at Lv 5 from other sources counts as a miss in the fits (so the rate is that of a firing that
shows); the rate where it could show leaves those out. K = random events on offer: every shared event of the deck's
characters plus the
trainee's own.
The fits are maximum likelihood over readings: a constant rate and the K form a (1 - b)^(K - 4). K still counts an R
card's character, whose shared events are on offer whatever the card's rarity.
Findings: the K form against the constant held out (5-fold cross-validation by common.fold(run id), each run scored by
the forms fitted on the other folds, compared with common.compare_held_out); tests on parameters (the slope in K; the
covariates in a logistic model next to K: card effects at their limit break from master.mdb in one fit, tested jointly
and each with Holm's adjustment, then again next to character intercepts, scenario, SR against SSR within a
character),
run-clustered since a run gives several
readings: Huber-White sandwich Wald tests (robust(); likelihood ratios shown beside them); within-run independence
(pairs of hits against independent firing, two-sided, with one rate per run and with a rate per reading from a
logistic model of K, card effects, character and scenario).

Effect on real careers (section 13's calibration rows: each run rebuilt on its own deck, default targets, the solver's
per-target p, d^g, d^w, q, read through common's build_jobs / run_solver / rows_of, cached results reused):
  rows        free rows (not on the trainee's outfit, not a scenario gold) where a deck card's character's marker is the
              target's white, under the reading's rules (not the trainee, not a group card, not given by the card's
              own events); the trainee's own shared event where it is identifiable: her marker is the target's white
              and no other event of hers (story, date, outfit, conditional) nor of the run's scenario can hint it,
              so an "Events" credit can only be that event (GameTora training_events_char / _char_card / _scenario)
  contribution  rows whose target was credited to "<her name> event" (and to nothing else); for the trainee's own,
              to "Events"
  crediting   section 12: p = d^g G + (1 - d^g) W [1 - m], m = (1 - d^w)(1 - e)(1 - q); m is recovered from the row's p
              (G = 0.4 x 1.1^n, W = 0.2 x 1.1^n) and multiplied by (1 - r), r the fitted form at the run's K (the
              trainee's own at her measured rate); acquisition = d^g + (1 - d^g)(1 - m)
  odds        each run's product of per-target odds over the targets the model gives a positive chance without
              crediting, relative change from crediting (median and max over the runs with a credited row among them)
  control     the same targets' free rows on decks where no card's character has them as her marker (and the trainee
              is not that character): obs / pred left out, weighted to the treated rows' mix of targets, and the
              treated minus control difference with a run-clustered standard error (each row's linearised contribution
              to the two ratios, summed by run); stratified by target and predicted acquisition (the treated rows
              against the control's acquisition rate per stratum); both again on decks where no card's training hints
              (master.mdb single_mode_hint_gain) give the target; and the credited rows against the control
  calibration on the rows, acquisition (hinted from any source or owned) against the predicted acquisition with and
              without crediting: obs / pred with run-clustered 95% intervals (common.cluster_ci; a run can give
              several rows); held out, the same rows by 5-fold cross-validation (the rate fitted on the other folds'
              readings), per-run Bernoulli log-likelihoods credited against left out, over the rows with a positive
              prediction left out
"""

import collections, itertools, math
from collections.abc import Callable, Sequence
from typing import Any, NamedTuple, TypedDict
import numpy as np
from scipy.optimize import minimize
import common
from common import (
    CalRow,
    CalibTest,
    RunRows,
    bernoulli_ll,
    calib,
    calib_test,
    credit,
    event_groups,
    free,
    hint_srcs,
    marker_credit,
)
from common import Run

SEC: str = "10"


class Reading(TypedDict):
    """one reading (readings()): a deck card of another character in a run, and whether her marker was credited to her
    event"""

    run: int  # run id
    card: int  # the card's id
    rar: int  # its rarity (1 R, 2 SR, 3 SSR)
    hit: int  # 1: the marker credited to "<her name> event"
    amb: int  # 1: a hit credited only through an "A or B event" naming her (B's events cannot give the marker)
    cens: int  # 1: no credit, the marker already at Lv 5 from other sources (the event could not show)
    K: int  # random events on offer in the run (offer())
    char: int  # her character id
    lb: int  # the card's limit break
    scen: int  # scenario id


Fits = dict[str, Any]  # fits(): form -> (parameters, max LL)


class RandomRow(TypedDict):
    """random_rows(): a calibration row a random event could hint"""

    row: CalRow
    kind: str  # 'deck' (a deck card's character's event) or 'own' (the trainee's own)
    who: str | None  # the character's name
    rate: float | None  # the event's chance at the run's K ('own': filled in by careers())
    hit: bool  # the target credited to that event (marker_credit(): section 8's rule, unknown counts as not)
    only: bool  # ... and to nothing else, nor owned
    amb: bool  # credited only through an "A or B event" naming her


class SharedEventMarkers(NamedTuple):
    """markers(): what the readings are read from (GameTora, through gametora.py)"""

    nshared: collections.Counter[int]  # character -> count of her shared events
    marker: dict[int, str]  # character -> her marker skill
    cardsk: collections.defaultdict[int, set[str]]  # SR/SSR card -> skills decoded from its own events
    sc: dict[int, dict[str, Any]]  # GameTora support-cards by id


def outcome_skill_sets(rewards: Any, gt: Any, sk: dict[int, str]) -> list[set[str]]:
    """one choice's outcomes, each as the set of skills it can hint"""
    out: list[set[str]] = []
    for hs, _ in gt.outcomes(rewards):
        s = set()
        for kind, x, _ in hs:
            if kind == "hint" and x in sk:
                s.add(sk[x])
            elif kind == "random":
                s |= {sk[i] for i in x if i in sk}
        out.append(s)
    return out


def sure_skills(rewards: Any, gt: Any, sk: dict[int, str]) -> set[str]:
    """the skills every outcome of one choice hints"""
    o = outcome_skill_sets(rewards, gt, sk)
    return set.intersection(*o) if o else set()


def markers() -> SharedEventMarkers:
    """each character's marker: a skill common to every top-choice outcome, when no other choice has a skill common
    to all its outcomes"""
    gt = common.gametora()
    db = common.mdb()
    sk = dict(db.execute("select [index], text from text_data where category = 47"))
    nshared: collections.Counter[int] = collections.Counter()
    marker: dict[int, str] = {}
    for chara, es in gt.load("training_events_shared"):
        for e in es:
            chs = gt.choices(e[1])
            if not chs:
                continue
            nshared[chara] += 1
            if len(chs) >= 2:
                a, b = sure_skills(chs[0][1], gt, sk), set().union(*[sure_skills(ch[1], gt, sk) for ch in chs[1:]])
                if a and not b:
                    marker[chara] = sorted(a)[0]
    cardsk: collections.defaultdict[int, set[str]] = collections.defaultdict(set)
    for key in ("training_events_sr", "training_events_ssr"):
        for cid, es in gt.load(key):
            for e in es:
                for ch in gt.choices(e[1]):
                    cardsk[cid] |= set().union(*outcome_skill_sets(ch[1], gt, sk))
    sc = {x["support_id"]: x for x in gt.load("support-cards")}
    return SharedEventMarkers(nshared, marker, cardsk, sc)


def offer(r: Run, nshared: collections.Counter[int], sc: dict[int, dict[str, Any]]) -> int:
    """K: random events on offer in run r (every shared event of the deck's characters plus the trainee's own)"""
    chars = {sc[x]["char_id"] for x in (d[0] for d in r["deck"]) if x in sc and sc[x].get("type") != "group"}
    return sum(nshared[ch] for ch in chars - {r["chara"]}) + nshared[r["chara"]]


class ReadingSet(NamedTuple):
    """readings(): the readings, and how many were left out as unknown"""

    rows: list[Reading]
    r_cards: int  # readings of R cards, left out
    unknown: int  # another deck card's events can give the marker (marker_credit())


def event_pools(sc: dict[int, dict[str, Any]]) -> dict[int, set[str]]:
    """support card -> every skill an event credited to its character can give: the card's own events (section 8's
    skill pools, common.game().pool) and its character's shared events (GameTora training_events_shared)"""

    gt = common.gametora()
    sk = dict(common.mdb().execute("select [index], text from text_data where category = 47"))
    shared: collections.defaultdict[int, set[str]] = collections.defaultdict(set)
    for chara, es in gt.load("training_events_shared"):
        for e in es:
            for ch in gt.choices(e[1]):
                shared[chara] |= set().union(*outcome_skill_sets(ch[1], gt, sk))
    own = common.game().pool
    return {cid: own.get(cid, set()) | shared.get(x["char_id"], set()) for cid, x in sc.items()}


def deck_names(r: Run) -> dict[str, list[int]]:
    """run r's deck by bare card name (as hint sources name a card): name -> card ids"""
    out: collections.defaultdict[str, list[int]] = collections.defaultdict(list)
    for cid, _, nm in r["deck"]:
        out[common.bare(nm)].append(cid)
    return out


def readings(R: list[Run]) -> ReadingSet:
    """every reading of the runs: a deck card of a character with a marker, someone else training, the marker not given
    by the card's own events; a reading whose credit is unknown (marker_credit()) is left out and counted"""
    nshared, marker, cardsk, sc = markers()
    pool = event_pools(sc)
    name = dict(common.mdb().execute("select [index], text from text_data where category = 6"))
    rows: list[Reading] = []
    unknown = r_cards = 0
    for r in R:
        dk = [d[0] for d in r["deck"]]
        byname = deck_names(r)
        tr = r["chara"]
        lbs = {d[0]: d[1] for d in r["deck"]}
        K = offer(r, nshared, sc)
        for cid in dk:
            x = sc.get(cid)
            if not x or x.get("type") == "group":
                continue
            ch = x["char_id"]
            m = marker.get(ch)
            if not m or ch == tr or m in cardsk[cid]:
                continue
            if x["rarity"] < 2:
                r_cards += 1
                continue
            nm = name.get(ch)
            srcs = [s for h in r["hints"] if h["name"] == m for s in h["src"]]
            hit = marker_credit(srcs, nm, m, byname, pool)
            if hit is None:
                unknown += 1
                continue
            amb = hit and f"{nm} event" not in srcs
            # censored: the marker already at hint Lv 5 from other sources, so the event's hint could not show
            cens = not hit and max([h["lv"] or 0 for h in r["hints"] if h["name"] == m] or [0]) >= 5
            rows.append(
                Reading(
                    run=r["id"],
                    card=cid,
                    rar=x["rarity"],
                    hit=int(hit),
                    amb=int(amb),
                    cens=int(cens),
                    K=K,
                    char=ch,
                    lb=lbs[cid],
                    scen=r["scenario"],
                )
            )
    return ReadingSet(rows, r_cards, unknown)


def fits(rows: Sequence[Reading]) -> Fits:
    """the constant's and the K form's maximum-likelihood fits over the readings: form -> (parameters, max LL)"""
    y = np.array([r["hit"] for r in rows], float)
    K = np.array([r["K"] for r in rows], float)
    out: Fits = {"const": (y.mean(), bernoulli_ll(y, np.full_like(y, y.mean())))}
    o = minimize(
        lambda s: -bernoulli_ll(y, s[0] * (1 - s[1]) ** (K - 4)), [0.2, 0.05], bounds=[(0.01, 0.99), (-0.5, 0.5)]
    )
    out["exp"] = (tuple(o.x), -o.fun)
    return out


def form_p(f: Fits, rows: Sequence[Reading]) -> dict[str, np.ndarray]:
    """each form's P(fires) on `rows`, with the parameters of fits() output f"""
    K = np.array([r["K"] for r in rows], float)
    a, b = f["exp"][0]
    return {"const": np.full_like(K, f["const"][0]), "exp": a * (1 - b) ** (K - 4)}


def held_out(rows: Sequence[Reading]) -> dict[str, list[float]]:
    """rule 3: per-run held-out log-likelihood of each form. Runs fall in five folds by common.fold(run id), all of a
    run's readings in one; each fold's readings are scored by the forms fitted (fits()) on the other four folds. Returns
    form -> list over runs (sorted by id) of the run's summed Bernoulli log-likelihood"""
    runs = sorted({r["run"] for r in rows})
    out: collections.defaultdict[str, collections.defaultdict[int, float]] = collections.defaultdict(
        lambda: collections.defaultdict(float)
    )
    for k in range(common.K_FOLDS):
        test = [r for r in rows if common.fold(r["run"]) == k]
        P = form_p(fits([r for r in rows if common.fold(r["run"]) != k]), test)
        y = np.array([r["hit"] for r in test], float)
        for nm, p in P.items():
            p = np.clip(p, 1e-9, 1 - 1e-9)
            lr = y * np.log(p) + (1 - y) * np.log(1 - p)
            for r, v in zip(test, lr):
                out[nm][r["run"]] += float(v)
    return {nm: [d[i] for i in runs] for nm, d in out.items()}


def by_k(rows: Sequence[Reading]) -> collections.defaultdict[int, list[int]]:
    """K (14 and over together) -> [readings, hits]"""
    t: collections.defaultdict[int, list[int]] = collections.defaultdict(lambda: [0, 0])
    for r in rows:
        k = min(r["K"], 14)
        t[k][0] += 1
        t[k][1] += r["hit"]
    return t


def claims() -> list[common.Value | common.Finding]:
    """the readings' rate and fits, the tests on them, and the effect on real careers"""
    R = common.runs()
    if R is None:
        return []
    rows, nr, unk = readings(R)
    f = fits(rows)
    g: dict[str, Any] = {}  # value name -> value, in the order they are listed
    y = sum(r["hit"] for r in rows)
    n = len(rows)
    amb = sum(r["amb"] for r in rows)
    cz = sum(r["cens"] for r in rows)  # the marker at Lv 5 from other sources: the event could not show
    g.update(
        {
            "readings": n,
            "R-card readings left out": nr,
            "runs": len({r["run"] for r in rows}),
            "hits": y,
            "hits through a shared credit naming her": amb,
            "readings left out as unknown": unk,
            "readings where the marker was already at Lv 5": cz,
            "rate where it could show": y / (n - cz),
            "rate where it could show, unknown readings as hits (upper bound)": (y + unk) / (n + unk - cz),
        }
    )
    g["constant p"] = f["const"][0]
    a, b = f["exp"][0]
    g["fitted P at K = 4"] = a
    g["fitted P at K = 14"] = a * (1 - b) ** 10
    g["pairs both hitting: observed"], g["pairs both hitting: expected"], g["pairs both hitting: z"] = pairs_test(
        rows, a, b
    )
    _, g["pairs both hitting, rates by reading: expected"], g["pairs both hitting, rates by reading: z"] = pairs_test_p(
        rows, reading_rates(rows)
    )
    g.update(robust(rows, f))
    h = held_out(rows)
    g["held out: runs"] = len(h["exp"])
    lab = "fitted form against constant"
    g[f"held out: {lab}: nats"], g[f"held out: {lab}: SE"], g[f"held out: {lab}: z"] = common.compare_held_out(
        h["exp"], h["const"]
    )
    out = [common.Value(SEC, name, v) for name, v in g.items()] + findings(rows, f, g)
    return out + careers(R, (a, b), rows)


# ---------------------------------------------------------------- findings: each conclusion as a likelihood or interval test
def logit_ll(y: np.ndarray, X: np.ndarray) -> float:
    """max log-likelihood of a logistic regression (X holds its own intercept columns)"""
    f = lambda b: float(np.sum(np.logaddexp(0, X @ b) - y * (X @ b)))
    gr = lambda b: X.T @ (1 / (1 + np.exp(-(X @ b))) - y)
    return -minimize(f, np.zeros(X.shape[1]), jac=gr, method="BFGS").fun


# ---------------------------------------------------------------- run-clustered (sandwich) tests
# A run gives several readings (one per eligible deck card), so the likelihood-ratio and profile-likelihood tests,
# which treat readings as independent, can overstate the evidence. Each test is redone with the run as the unit:
# per-run scores s_r (the run's derivative of its log-likelihood) and H (the second derivative of the total), the
# robust covariance H^-1 B H^-1 with B = C / (C - 1) sum_r s_r s_r' (Huber 1967, White 1982; Liang and Zeger 1986).
def run_scores(
    p_fn: Callable[[np.ndarray], np.ndarray],
    y: np.ndarray,
    run: Sequence[int],
    th: Sequence[float] | np.ndarray,
    free: Sequence[int],
) -> tuple[np.ndarray, np.ndarray]:
    """per-run scores (C x k) and the Hessian (k x k) of the total Bernoulli log-likelihood of P = p_fn(theta) in the
    parameters `free` (indices into theta; the rest held fixed), by central differences at theta"""
    th = np.asarray(th, float)
    free = list(free)
    ix = {r: i for i, r in enumerate(sorted(set(run)))}
    ri = np.array([ix[r] for r in run])

    def rowscore(t: np.ndarray) -> np.ndarray:
        p = np.clip(p_fn(t), 1e-9, 1 - 1e-9)
        w = y / p - (1 - y) / (1 - p)
        J = np.empty((len(y), len(free)))
        for j, q in enumerate(free):
            h = 1e-6 * max(1.0, abs(t[q]))
            a, b = t.copy(), t.copy()
            a[q] += h
            b[q] -= h
            J[:, j] = (p_fn(a) - p_fn(b)) / (2 * h)
        return w[:, None] * J

    S = np.zeros((len(ix), len(free)))
    np.add.at(S, ri, rowscore(th))
    H = np.empty((len(free), len(free)))
    for j, q in enumerate(free):
        h = 1e-4 * max(1.0, abs(th[q]))
        a, b = th.copy(), th.copy()
        a[q] += h
        b[q] -= h
        H[:, j] = (rowscore(a).sum(0) - rowscore(b).sum(0)) / (2 * h)
    return S, (H + H.T) / 2


def logit_fit(y: np.ndarray, X: np.ndarray) -> np.ndarray:
    """a logistic regression's coefficients (as logit_ll fits it)"""
    f = lambda b: float(np.sum(np.logaddexp(0, X @ b) - y * (X @ b)))
    gr = lambda b: X.T @ (1 / (1 + np.exp(-(X @ b))) - y)
    return minimize(f, np.zeros(X.shape[1]), jac=gr, method="BFGS").x


def logit_scores(y: np.ndarray, X: np.ndarray, run: Sequence[int], b: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """a logistic regression's per-run scores and Hessian at b (exact)"""
    p = 1 / (1 + np.exp(-(X @ b)))
    ix = {r: i for i, r in enumerate(sorted(set(run)))}
    ri = np.array([ix[r] for r in run])
    S = np.zeros((len(ix), X.shape[1]))
    np.add.at(S, ri, (y - p)[:, None] * X)
    return S, -(X.T * (p * (1 - p))) @ X


def sandwich(S: np.ndarray, H: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """(robust covariance H^-1 B H^-1, B = C / (C - 1) S'S; the model's covariance -H^-1)"""
    C = len(S)
    Hi = np.linalg.inv(H)
    return Hi @ (C / (C - 1) * S.T @ S) @ Hi, -Hi


def wald(b: np.ndarray, V: np.ndarray, idx: list[int], null: Sequence[float] | None = None) -> tuple[float, int, float]:
    """Wald chi2 of b[idx] = null (0 by default) under covariance V: (chi2, df, p)"""
    from scipy.stats import chi2

    d = np.asarray(b, float)[idx] - (0 if null is None else np.asarray(null, float))
    Vs = np.asarray(V)[np.ix_(idx, idx)]
    x2 = float(d @ np.linalg.solve(Vs, d))
    return x2, len(idx), float(chi2.sf(x2, len(idx)))


class ReadingArrays(NamedTuple):
    """the readings as arrays, by reading"""

    y: np.ndarray  # hit
    K: np.ndarray
    run: list[int]


def reading_arrays(rows: Sequence[Reading]) -> ReadingArrays:
    return ReadingArrays(
        np.array([r["hit"] for r in rows], float),
        np.array([r["K"] for r in rows], float),
        [r["run"] for r in rows],
    )


def robust_z(b: np.ndarray, V: np.ndarray, naive: np.ndarray, j: int, null: float = 0.0) -> tuple[float, float]:
    """Wald z of parameter j against `null` with the run-clustered and the model covariance"""
    return (b[j] - null) / np.sqrt(V[j, j]), (b[j] - null) / np.sqrt(naive[j, j])


def robust(rows: Sequence[Reading], f: Fits) -> dict[str, float]:
    """each likelihood-ratio / profile test of the findings, redone run-clustered: numbers by name"""
    x = reading_arrays(rows)
    g = slope_test(x, f) | covariate_tests(rows, x)
    return {k: float(v) for k, v in g.items()}


def slope_test(x: ReadingArrays, f: Fits) -> dict[str, Any]:
    """the slope in K: b of a (1 - b)^(K - 4), Wald against 0"""
    ab = np.array(f["exp"][0])
    V, Vn = sandwich(*run_scores(lambda t: t[0] * (1 - t[1]) ** (x.K - 4), x.y, x.run, ab, [0, 1]))
    out: dict[str, Any] = {}
    out["slope in K: robust z"], out["slope in K: model z"] = robust_z(ab, V, Vn, 1)
    return out


def covariate_tests(rows: Sequence[Reading], x: ReadingArrays) -> dict[str, Any]:
    """the covariates next to K in a logistic model, SR against SSR within a character"""
    y, K, run = x.y, x.K, x.run
    g: dict[str, Any] = {}
    one = np.ones(len(rows))
    # the card effects next to K, in one fit: a joint test (df 3), and each coefficient with Holm's adjustment
    X = np.column_stack([one, K, *card_effects(rows)])
    b = logit_fit(y, X)
    V, _ = sandwich(*logit_scores(y, X, run, b))
    g["card effects: robust Wald chi2"], _, g["card effects: robust p"] = wald(b, V, [2, 3, 4])
    ps = {nm: wald(b, V, [j])[2] for j, (_, nm) in enumerate(COVARIATES, 2)}
    holm = holm_adjust(ps)
    for j, (_, nm) in enumerate(COVARIATES, 2):
        se = float(np.sqrt(V[j, j]))
        g[f"{nm}: logit per 10 points"] = b[j]
        g[f"{nm}: 95% low"], g[f"{nm}: 95% high"] = b[j] - 1.96 * se, b[j] + 1.96 * se
        g[f"{nm}: robust p"], g[f"{nm}: Holm p"] = ps[nm], holm[nm]
    # scenario next to K
    X = np.column_stack([one, K, *scenario_columns(rows)])
    b = logit_fit(y, X)
    V, _ = sandwich(*logit_scores(y, X, run, b))
    g["scenario: robust Wald chi2"], _, g["scenario: robust p"] = wald(b, V, list(range(2, X.shape[1])))
    for j, (sid, _) in enumerate(scenario_levels(rows), 2):
        g[f"scenario {sid}: logit against URA Finale"] = b[j]
    # scenario next to K and the card effects (the gauge is a known miss of the rate): does it survive them?
    X = np.column_stack([one, K, *card_effects(rows), *scenario_columns(rows)])
    b = logit_fit(y, X)
    V, _ = sandwich(*logit_scores(y, X, run, b))
    g["scenario next to the card effects: robust Wald chi2"], _, g["scenario next to the card effects: robust p"] = (
        wald(b, V, list(range(5, X.shape[1])))
    )
    # the card effects within a character: character intercepts next to K and the card effects (the gauge's effect
    # across characters could be a difference between characters)
    chars = sorted({r["char"] for r in rows})
    D = [np.array([r["char"] == c for r in rows], float) for c in chars[1:]]
    X = np.column_stack([one, K, *card_effects(rows), *D])
    b = logit_fit(y, X)
    V, Vn = sandwich(*logit_scores(y, X, run, b))
    g["characters next to K and the card effects: robust Wald chi2"], g["characters: df"], g["characters: robust p"] = (
        wald(b, V, list(range(5, X.shape[1])))
    )
    g["within a character: slope in K: robust z"], g["within a character: slope in K: model z"] = robust_z(b, V, Vn, 1)
    ps = {nm: wald(b, V, [j])[2] for j, (_, nm) in enumerate(COVARIATES, 2)}
    holm = holm_adjust(ps)
    for j, (_, nm) in enumerate(COVARIATES, 2):
        se = float(np.sqrt(V[j, j]))
        g[f"within a character: {nm}: logit per 10 points"] = b[j]
        g[f"within a character: {nm}: 95% low"] = b[j] - 1.96 * se
        g[f"within a character: {nm}: 95% high"] = b[j] + 1.96 * se
        g[f"within a character: {nm}: Holm p"] = holm[nm]
    # SR against SSR within a character (characters with cards of both rarities; R cards are not read)
    ch, w = multi_rarity(rows)
    X = np.column_stack(
        [np.array([[rows[i]["char"] == c for c in ch] for i in w], float), K[w], [rows[i]["rar"] for i in w]]
    )
    b = logit_fit(y[w], X)
    V, Vn = sandwich(*logit_scores(y[w], X, [run[i] for i in w], b))
    g["rarity within a character: robust z"], g["rarity within a character: model z"] = robust_z(
        b, V, Vn, X.shape[1] - 1
    )
    return g


# card effects tested next to K: (master.mdb effect type, name)
COVARIATES: tuple[tuple[int, str], ...] = (
    (1, "friendship bonus"),
    (14, "initial friendship gauge"),
    (19, "specialty priority"),
)


def card_effects(rows: Sequence[Reading]) -> list[np.ndarray]:
    """each reading's card effects (COVARIATES, in that order) at its limit break, per 10 points"""
    cache: dict[tuple[int, int, int], float] = {}
    return [
        np.array([cache.setdefault((r["card"], r["lb"], t), card_effect(r["card"], r["lb"], t)) for r in rows]) / 10
        for t, _ in COVARIATES
    ]


def scenario_columns(rows: Sequence[Reading]) -> list[np.ndarray]:
    """an indicator per scenario but the first (by id)"""
    return [np.array([r["scen"] == v for r in rows], float) for v in sorted({r["scen"] for r in rows})[1:]]


def scenario_levels(rows: Sequence[Reading]) -> list[tuple[str, int]]:
    """(name, id) of each scenario scenario_columns() gives a column, in its order"""
    return [(common.SCEN.get(v, str(v)), v) for v in sorted({r["scen"] for r in rows})[1:]]


def holm_adjust(ps: dict[str, float]) -> dict[str, float]:
    """Holm's step-down adjustment of a family of p-values (Holm 1979): name -> adjusted p"""
    out: dict[str, float] = {}
    run = 0.0
    for i, (nm, p) in enumerate(sorted(ps.items(), key=lambda t: t[1])):
        run = max(run, min(1.0, (len(ps) - i) * p))
        out[nm] = run
    return out


def multi_rarity(rows: Sequence[Reading]) -> tuple[list[int], list[int]]:
    """the characters with cards of more than one rarity among the readings, and the indices of their readings"""
    multi: collections.defaultdict[int, set[int]] = collections.defaultdict(set)
    for r in rows:
        multi[r["char"]].add(r["rar"])
    ch = sorted(c for c, v in multi.items() if len(v) > 1)
    return ch, [i for i, r in enumerate(rows) if r["char"] in ch]


def card_effect(cid: int, lb: int, typ: int) -> float:
    """a support card's effect of one type at its limit break's level cap (master.mdb: linear between milestone levels,
    plus unique effects reached)"""
    db = common.mdb()
    eff, rar = db.execute("select effect_table_id, rarity from support_card_data where id = ?", (cid,)).fetchone()
    lv = db.execute(f"select limit_{lb} from support_card_limit where rarity = ?", (rar,)).fetchone()[0]
    row = db.execute(
        "select init, limit_lv5, limit_lv10, limit_lv15, limit_lv20, limit_lv25, limit_lv30, limit_lv35, limit_lv40, "
        "limit_lv45, limit_lv50 from support_card_effect_table where id = ? and type = ?",
        (eff, typ),
    ).fetchone()
    v = 0.0
    pts = [(level, x) for level, x in zip([1, *range(5, 55, 5)], row or []) if x != -1]  # (level, value) milestones
    if pts and lv >= pts[0][0]:
        v = float(pts[-1][1])
        for (l0, v0), (l1, v1) in itertools.pairwise(pts):
            if l0 <= lv <= l1:
                v = v0 + (v1 - v0) * (lv - l0) / (l1 - l0)
                break
    for ulv, t0, v0, t1, v1 in db.execute(
        "select lv, type_0, value_0, type_1, value_1 from support_card_unique_effect where id = ?", (cid,)
    ):
        v += sum(x for t, x in ((t0, v0), (t1, v1)) if t == typ and lv >= ulv)
    return v


def reading_rates(rows: Sequence[Reading]) -> np.ndarray:
    """each reading's fitted chance from a logistic model of K, the card effects, character intercepts and scenario"""
    one = np.ones(len(rows))
    K = np.array([r["K"] for r in rows], float)
    chars = sorted({r["char"] for r in rows})
    D = [np.array([r["char"] == c for r in rows], float) for c in chars[1:]]
    X = np.column_stack([one, K, *card_effects(rows), *D, *scenario_columns(rows)])
    y = np.array([r["hit"] for r in rows], float)
    return 1 / (1 + np.exp(-(X @ logit_fit(y, X))))


def pairs_test_p(rows: Sequence[Reading], p: np.ndarray) -> tuple[float, float, float]:
    """within-run independence with a chance per reading: pairs of one run's readings that both hit against the count
    independent firing at those chances gives (each run's hit count Poisson binomial; mean and variance exact)"""
    by: collections.defaultdict[int, list[int]] = collections.defaultdict(list)
    for i, r in enumerate(rows):
        by[r["run"]].append(i)
    o = e = v = 0.0
    for ix in by.values():
        dist = [1.0]
        for q in p[ix]:
            dist = [u * (1 - q) + w * q for u, w in zip([*dist, 0.0], [0.0, *dist])]
        c2 = [k * (k - 1) / 2 for k in range(len(dist))]
        m = sum(q * c for q, c in zip(dist, c2))
        h = sum(rows[i]["hit"] for i in ix)
        o += h * (h - 1) / 2
        e += m
        v += sum(q * c * c for q, c in zip(dist, c2)) - m * m
    return o, e, (o - e) / math.sqrt(v)


def pairs_test(rows: Sequence[Reading], a: float, b: float) -> tuple[float, float, float]:
    """within-run independence: pairs of readings of one run that both hit, against Binomial(n, p) per run with the
    fitted p (one K per run); the variance is exact per run"""
    by: collections.defaultdict[int, list[Reading]] = collections.defaultdict(list)
    for r in rows:
        by[r["run"]].append(r)
    o = e = v = 0.0
    for rs in by.values():
        n = len(rs)
        p = a * (1 - b) ** (rs[0]["K"] - 4)
        h = sum(r["hit"] for r in rs)
        pm = common.binom_pmf(n, p)
        c2 = [k * (k - 1) / 2 for k in range(n + 1)]
        m = sum(q * c for q, c in zip(pm, c2))
        o += h * (h - 1) / 2
        e += m
        v += sum(q * c * c for q, c in zip(pm, c2)) - m * m
    return o, e, (o - e) / math.sqrt(v)


def F(st: str, test: Any, ev: Any) -> common.Finding:
    return common.finding(SEC, st, test, ev)


def findings(rows: Sequence[Reading], f: Fits, g: dict[str, Any]) -> list[common.Finding]:
    """the readings' conclusions, each as a likelihood or interval test (g: value name -> value)"""
    return form_findings(rows, f, g) + covariate_findings(rows, g)


def rz(g: dict[str, Any], nm: str) -> str:
    """a test's run-clustered z beside the one with independent readings"""
    return f"run-clustered z {g[nm + ': robust z']:+.2f} (independent readings {g[nm + ': model z']:+.2f})"


def form_findings(rows: Sequence[Reading], f: Fits, g: dict[str, Any]) -> list[common.Finding]:
    """the slope in K and within-run independence"""
    L0, (ab, Le) = f["const"][1], f["exp"]
    out = [
        F(
            "The slope in K is detectable: the K form a (1 - b)^(K - 4) against a constant (b = 0, nested), "
            "run-clustered Wald |z| above 1.96 (95%)",
            abs(g["slope in K: robust z"]) > 1.96,
            f"a {ab[0]:.2f}, b {ab[1]:.3f}: LL {Le:.1f} against {L0:.1f}: LR {2 * (Le - L0):.2f}; {rz(g, 'slope in K')}",
        )
    ]
    out += held_out_findings(g)
    o, e, z = g["pairs both hitting: observed"], g["pairs both hitting: expected"], g["pairs both hitting: z"]
    ea, za = g["pairs both hitting, rates by reading: expected"], g["pairs both hitting, rates by reading: z"]
    out.append(
        F(
            "Within a run, no departure from independent firing is detectable in either direction: pairs of one run's "
            "readings that both hit are within 1.96 sd of the count independence predicts, with one rate per run and "
            "with a rate per reading (K, card effects, character, scenario)",
            abs(z) < 1.96 and abs(za) < 1.96,
            f"{o:.0f} pairs observed; one rate per run {e:.1f} expected, z {z:+.2f}; rate per reading {ea:.1f}, z "
            f"{za:+.2f}",
        )
    )
    return out


def covariate_findings(rows: Sequence[Reading], g: dict[str, Any]) -> list[common.Finding]:
    """card effects and scenario next to K, SR against SSR within a character"""
    from scipy.stats import chi2

    out: list[common.Finding] = []
    yb = np.array([r["hit"] for r in rows], float)
    one = np.ones(len(rows))
    Kb = np.array([r["K"] for r in rows], float)
    base = logit_ll(yb, np.column_stack([one, Kb]))
    # covariates next to K: likelihood ratios shown beside the run-clustered tests
    lr: Callable[[list[np.ndarray]], float] = lambda cols: 2 * (logit_ll(yb, np.column_stack([one, Kb, *cols])) - base)
    sc = scenario_columns(rows)
    lr_sc = lr(sc)
    fr = tuple(nm for _, nm in COVARIATES)
    gauge, others = "initial friendship gauge", [k for k in fr if k != "initial friendship gauge"]
    each = "; ".join(
        f"{k} {g[k + ': logit per 10 points']:+.3f} [{g[k + ': 95% low']:+.3f}, {g[k + ': 95% high']:+.3f}] logit per 10 "
        f"points, p {g[k + ': robust p']:.4f}, Holm {g[k + ': Holm p']:.4f}"
        for k in fr
    )
    out.append(
        F(
            "Across characters, next to K, of the card effects only the initial friendship gauge is detectable, and it "
            "goes with a higher rate: in one fit of friendship bonus, initial friendship gauge and specialty priority, "
            "the gauge's Holm-adjusted run-clustered Wald p is below 0.05 with its 95% interval above 0, and the other "
            "two's are above 0.05",
            g[f"{gauge}: Holm p"] < 0.05
            and g[f"{gauge}: 95% low"] > 0
            and all(g[f"{k}: Holm p"] > 0.05 for k in others),
            f"joint {g['card effects: robust Wald chi2']:.2f} (df 3, limit {chi2.ppf(0.95, 3):.2f}, p "
            f"{g['card effects: robust p']:.4f}; LR {lr(card_effects(rows)):.2f}); {each}",
        )
    )
    lv = "; ".join(f"{sid} {g[f'scenario {sid}: logit against URA Finale']:+.3f}" for sid, _ in scenario_levels(rows))
    out.append(
        F(
            "Known miss of the measured rate: scenario is detectable next to K at the 5% level (run-clustered Wald p < 0.05, "
            "df 3), and stays so next to K and the card effects",
            g["scenario: robust p"] < 0.05 and g["scenario next to the card effects: robust p"] < 0.05,
            f"scenario {g['scenario: robust Wald chi2']:.2f} (df {len(sc)}, limit {chi2.ppf(0.95, len(sc)):.2f}, p "
            f"{g['scenario: robust p']:.3f}; LR {lr_sc:.2f}); next to the card effects "
            f"{g['scenario next to the card effects: robust Wald chi2']:.2f}, p "
            f"{g['scenario next to the card effects: robust p']:.3f}; logit against URA Finale: {lv}",
        )
    )
    within = "; ".join(
        f"{k} {g[f'within a character: {k}: logit per 10 points']:+.3f} [{g[f'within a character: {k}: 95% low']:+.3f}, "
        f"{g[f'within a character: {k}: 95% high']:+.3f}], Holm {g[f'within a character: {k}: Holm p']:.3f}"
        for k in fr
    )
    out.append(
        F(
            "Known miss of the measured rate: it differs between characters (character intercepts next to K and the "
            "card effects, run-clustered Wald p < 0.05; this can be how well each character's marker shows as much as "
            "how often her event fires), and within a character no card effect is detectable (each Holm-adjusted p "
            "above 0.05). That test is underpowered: the gauge's 95% interval within a character still contains its "
            "effect across characters, so the two cannot be told apart",
            g["characters: robust p"] < 0.05
            and all(g[f"within a character: {k}: Holm p"] > 0.05 for k in fr)
            and g["within a character: initial friendship gauge: 95% low"]
            <= g["initial friendship gauge: logit per 10 points"]
            <= g["within a character: initial friendship gauge: 95% high"],
            f"characters {g['characters next to K and the card effects: robust Wald chi2']:.1f} (df "
            f"{g['characters: df']:.0f}, p {g['characters: robust p']:.1e}); {within}; slope in K within a character: "
            f"{rz(g, 'within a character: slope in K')}",
        )
    )
    rar = np.array([r["rar"] for r in rows], float)
    ch, w = multi_rarity(rows)
    D = np.array([[rows[i]["char"] == c for c in ch] for i in w], float)
    fe = logit_ll(yb[w], np.column_stack([D, Kb[w]]))
    fe_r = logit_ll(yb[w], np.column_stack([D, Kb[w], rar[w]]))
    out.append(
        F(
            "SR against SSR is not detectable within a character (character intercepts and K, characters with an SR "
            "and an SSR card; R cards are left out of the rate): run-clustered Wald |z| below 1.96 (95%); pooled across "
            "characters the likelihood-ratio statistic is shown alongside",
            abs(g["rarity within a character: robust z"]) < 1.96,
            f"within {len(ch)} characters ({len(w):,} readings): {rz(g, 'rarity within a character')}, LR {2 * (fe_r - fe):.2f}; "
            f"pooled LR {lr([rar]):.2f}",
        )
    )
    return out


def held_out_findings(g: dict[str, Any]) -> list[common.Finding]:
    """rule 3 on the forms, criterion fixed before the numbers: the K form predicts unseen runs better than a constant
    (z > 1.96) or not, worded by what the data give"""
    lab = "fitted form against constant"
    nats, se, z = g[f"held out: {lab}: nats"], g[f"held out: {lab}: SE"], g[f"held out: {lab}: z"]
    return [
        F(
            "On SR and SSR cards the K form is not shown to predict unseen runs better than a constant rate, though "
            "its slope is detectable in sample: the form a (1 - b)^(K - 4) against a constant has a positive held-out "
            "gain with z <= 1.96",
            nats > 0 and z <= 1.96,
            f"{g['held out: runs']:,} runs, 5 folds: {nats:+.2f} nats, SE {se:.2f}, z {z:+.2f} ({lab})",
        )
    ]


def odds_change(runrows: Sequence[RunRows], rate: Callable[[CalRow], float]) -> list[float]:
    """relative change of each run's product of per-target odds when each row is credited at rate(row) (0: not), over
    the targets the model gives a positive chance without crediting: one per run with a credited row among them"""
    out: list[float] = []
    for pr in runrows:
        pos = [x for x in pr["rows"] if x["p"] > 0]
        if any(rate(x) > 0 and free(x) for x in pos):
            out.append(math.prod(credit(x, rate(x))[0] for x in pos) / math.prod(x["p"] for x in pos) - 1)
    return out


def random_rows(R: list[Run], ab: tuple[float, float]) -> tuple[list[RandomRow], collections.Counter[str]]:
    """the calibration rows a random event could hint, as RandomRow dictionaries, and counts of skipped rows; kind
    'deck' uses the fitted rate at the run's K, 'own' has its rate filled in later"""
    a, b = ab
    nshared, marker, cardsk, sc = markers()
    pool = event_pools(sc)
    E = event_groups()
    name = dict(common.mdb().execute("select [index], text from text_data where category = 6"))
    gn = common.skills()
    white = {g: gn[g][0] for g in common.DEFAULT}
    rows, _ = calib()
    byid = {r["id"]: r for r in R}
    out: list[RandomRow] = []
    skipped: collections.Counter[str] = collections.Counter()
    for row in rows:
        if not free(row):
            continue
        r = byid[row["run"]]
        g = row["g"]
        hs = hint_srcs(r)[g]
        srcs = [x for h in hs for x in h["src"]]
        K = offer(r, nshared, sc)
        for cid in (d[0] for d in r["deck"]):
            x = sc.get(cid)
            if not x or x.get("type") == "group" or marker.get(x["char_id"]) != white[g]:
                continue
            if x["char_id"] == r["chara"]:
                skipped["own character"] += 1
                continue
            if white[g] in cardsk[cid]:
                skipped["card's own events give it"] += 1
                continue
            if x["rarity"] < 2:
                skipped["R card"] += 1
                continue
            nm = name.get(x["char_id"])
            ev = f"{nm} event"
            out.append(
                RandomRow(
                    row=row,
                    kind="deck",
                    who=nm,
                    rate=a * (1 - b) ** (K - 4),
                    hit=marker_credit(srcs, nm, white[g], deck_names(r), pool) is True,
                    only=ev in srcs and set(srcs) == {ev} and g not in {s // 10 for s in r.get("owned", [])},
                    amb=ev not in srcs and any(nm in t and " or " in t for t in srcs),
                )
            )
        u = r["chara"]
        if marker.get(u) == white[g]:
            other = (
                E["ordinary"].get(u, set())
                | E["outfit"].get(r["outfit"], set())
                | E["scen"].get(r["scenario"], set())
                | set().union(set(), *[gs for gs, _ in E["cond"].get(u, [])])
            )
            if g in other:
                skipped["trainee's own, not identifiable"] += 1
                continue
            out.append(
                RandomRow(
                    row=row,
                    kind="own",
                    who=name.get(u),
                    rate=None,
                    hit="Events" in srcs,
                    only=set(srcs) == {"Events"} and g not in {s // 10 for s in r.get("owned", [])},
                    amb=False,
                )
            )
    return out, skipped


def control_rows(R: list[Run], treated: Sequence[RandomRow]) -> list[CalRow]:
    """the control for the left-out comparison: free calibration rows of the same targets (a default target that is a
    character's marker) where no deck card's character has that marker and the trainee is not that character, so no
    shared random event of this kind can hint them"""
    _, marker, _, sc = markers()
    gn = common.skills()
    white = {g: gn[g][0] for g in common.DEFAULT}
    targets = {g for g, w in white.items() if w in set(marker.values())}
    seen = {id(x["row"]) for x in treated}
    byid = {r["id"]: r for r in R}
    out: list[CalRow] = []
    rows, _ = calib()
    for row in rows:
        if row["g"] not in targets or not free(row) or id(row) in seen:
            continue
        r = byid[row["run"]]
        w = white[row["g"]]
        holders = [
            c for c, _, _ in r["deck"] if c in sc and sc[c].get("type") != "group" and marker.get(sc[c]["char_id"]) == w
        ]
        if not holders and marker.get(r["chara"]) != w:
            out.append(row)
    return out


class ControlTest(NamedTuple):
    """control_test(): the treated rows' obs / pred against the control's, both left out"""

    treated: float  # obs / pred on the treated rows
    control: float  # on the control rows, weighted to the treated rows' mix of targets
    diff: float  # treated - control
    se: float  # run-clustered (linearised ratios, summed by run)
    z: float


def control_test(
    T: Sequence[CalRow], C: Sequence[CalRow], pred_t: Callable[[CalRow], float] | None = None
) -> ControlTest:
    """obs / pred on the treated rows T (left out, or pred_t) against the control rows C left out, the control's ratio
    per target weighted by the treated rows' predicted acquisitions; the difference's standard error from each row's
    linearised contribution to the two ratios, summed by run (a run can give rows to both)"""
    p0 = lambda x: credit(x, 0.0)[1]
    pt = pred_t or p0
    ET = sum(pt(x) for x in T)
    RT = sum(bool(x["o_any"]) for x in T) / ET
    w = {g: sum(pt(x) for x in T if x["g"] == g) / ET for g in {x["g"] for x in T}}
    EC = {g: sum(p0(x) for x in C if x["g"] == g) for g in w}
    RC = {g: sum(bool(x["o_any"]) for x in C if x["g"] == g) / EC[g] for g in w}
    rc = sum(w[g] * RC[g] for g in w)
    infl: collections.defaultdict[int, float] = collections.defaultdict(float)
    for x in T:
        infl[x["run"]] += (bool(x["o_any"]) - RT * pt(x)) / ET
    for x in C:
        if x["g"] in w:
            infl[x["run"]] -= w[x["g"]] * (bool(x["o_any"]) - RC[x["g"]] * p0(x)) / EC[x["g"]]
    n = len(infl)
    se = math.sqrt(n / (n - 1) * sum(v * v for v in infl.values()))
    return ControlTest(RT, rc, RT - rc, se, (RT - rc) / se)


PRED_BINS: tuple[float, ...] = (0.2, 0.4, 0.6, 0.8)  # cut points of predicted acquisition for the stratified control


def stratified_control(T: Sequence[CalRow], C: Sequence[CalRow]) -> tuple[float, float, float]:
    """the treated rows' acquisitions against what the control's acquisition rate gives them, stratum by stratum
    (target, and predicted acquisition left out cut at PRED_BINS): (ratio, run-clustered SE, z of ratio - 1). Each
    row's linearised contribution summed by run"""
    import bisect

    st: Callable[[CalRow], tuple[int, int]] = lambda x: (x["g"], bisect.bisect(PRED_BINS, credit(x, 0.0)[1]))
    nt = collections.Counter(st(x) for x in T)
    nc = collections.Counter(st(x) for x in C)
    oc = collections.Counter(st(x) for x in C if x["o_any"])
    rc = {k: oc[k] / nc[k] for k in nc}
    T = [x for x in T if nc[st(x)]]  # a stratum with no control rows cannot be compared
    E = sum(rc[st(x)] for x in T)
    R = sum(bool(x["o_any"]) for x in T) / E
    infl: collections.defaultdict[int, float] = collections.defaultdict(float)
    for x in T:
        infl[x["run"]] += (bool(x["o_any"]) - R * rc[st(x)]) / E
    for x in C:
        k = st(x)
        if nt[k]:
            infl[x["run"]] -= R * nt[k] / nc[k] * (bool(x["o_any"]) - rc[k]) / E
    n = len(infl)
    se = math.sqrt(n / (n - 1) * sum(v * v for v in infl.values()))
    return R, se, (R - 1) / se


def without_hint_source(rows: Sequence[CalRow], R: list[Run]) -> list[CalRow]:
    """the rows whose deck has no card whose training hints (master.mdb single_mode_hint_gain) include the target's
    white skill"""
    db = common.mdb()
    gn = common.skills()
    sid = {v: k for k, v in db.execute("select [index], text from text_data where category = 47")}
    pool: collections.defaultdict[int, set[int]] = collections.defaultdict(set)
    for c, v in db.execute("select support_card_id, hint_value_1 from single_mode_hint_gain where hint_gain_type = 0"):
        pool[c].add(v)
    byid = {r["id"]: r for r in R}
    return [x for x in rows if not any(sid.get(gn[x["g"]][0]) in pool[c] for c, _, _ in byid[x["run"]]["deck"])]


def later_choice_targets() -> list[tuple[str, int, int]]:
    """default targets a character's shared event gives on a choice below the top one, which IT reaches only when the
    skill is prioritised: (skill, character, choice index) (GameTora, through gametora.py)"""
    gt = common.gametora()
    sk = dict(common.mdb().execute("select [index], text from text_data where category = 47"))
    gn = common.skills()
    white = {gn[g][0] for g in common.DEFAULT}
    out: set[tuple[str, int, int]] = set()
    for chara, es in gt.load("training_events_shared"):
        for e in es:
            for j, ch in enumerate(gt.choices(e[1])):
                if j:
                    out |= {(x, chara, j) for x in set().union(set(), *outcome_skill_sets(ch[1], gt, sk)) & white}
    return sorted(out)


def later_choice_runs(R: list[Run], skill: str, chara: int) -> tuple[int, int]:
    """runs with a card of the character (not a group card), someone else training, the skill prioritised on a shown
    list: (the skill credited to her event by section 8's rule, those runs whose credit is not unknown)"""
    _, _, _, sc = markers()
    pool = event_pools(sc)
    nm = dict(common.mdb().execute("select [index], text from text_data where category = 6")).get(chara)
    k = n = 0
    for r in R:
        p = r.get("priority")
        if p is None or skill not in p or r["chara"] == chara:
            continue
        if not any(c in sc and sc[c]["char_id"] == chara and sc[c].get("type") != "group" for c, _, _ in r["deck"]):
            continue
        hit = marker_credit(
            [x for h in r["hints"] if h["name"] == skill for x in h["src"]], nm, skill, deck_names(r), pool
        )
        if hit is None:
            continue
        n += 1
        k += hit
    return k, n


def bern_ll(pr: float, acquired: Any) -> float:
    """one row's Bernoulli log-likelihood (as calib_test scores it)"""
    if 0 < pr < 1:
        return math.log(pr) if acquired else math.log(1 - pr)
    return 0.0 if (pr >= 1) == bool(acquired) else -math.inf


def careers_held_out(
    dk: Sequence[RandomRow], R: list[Run], rows: Sequence[Reading]
) -> tuple[list[float], list[float], int]:
    """rule 3 on the calibration rows: per-run held-out log-likelihood of the acquisition rows a deck card's random
    event could hint, credited against left out. Runs fall in five folds by common.fold(run id); each fold's rows are
    credited at the adopted form fitted on the other four folds' readings. Scored on the rows the model gives a positive
    chance without crediting (as calib_test). Returns (credited, left out) per run, and the number of rows"""
    nshared, _, _, sc = markers()
    byid = {r["id"]: r for r in R}
    l1: collections.defaultdict[int, float] = collections.defaultdict(float)
    l0: collections.defaultdict[int, float] = collections.defaultdict(float)
    n = 0
    for k in range(common.K_FOLDS):
        a, b = fits([r for r in rows if common.fold(r["run"]) != k])["exp"][0]
        for x in dk:
            row = x["row"]
            if common.fold(row["run"]) != k:
                continue
            p0 = credit(row, 0.0)[1]
            if p0 <= 1e-12:
                continue
            p1 = credit(row, a * (1 - b) ** (offer(byid[row["run"]], nshared, sc) - 4))[1]
            l0[row["run"]] += bern_ll(p0, row["o_any"])
            l1[row["run"]] += bern_ll(p1, row["o_any"])
            n += 1
    runs = sorted(l0)
    return [l1[i] for i in runs], [l0[i] for i in runs], n


def careers(R: list[Run], ab: tuple[float, float], rows: Sequence[Reading]) -> list[common.Value | common.Finding]:
    """what leaving random events out does on real careers (section 13's rows), and what crediting them does"""
    V: Callable[[str, Any], common.Value] = lambda nm, v: common.Value(SEC, nm, v)
    rr, skipped = random_rows(R, ab)
    _, runrows = calib()
    dk = [x for x in rr if x["kind"] == "deck"]
    own = [x for x in rr if x["kind"] == "own"]
    r_own = common.div(sum(x["hit"] for x in own), len(own))
    for x in own:
        x["rate"] = r_own
    rate = {id(x["row"]): x["rate"] for x in rr}
    t = calib_test([x["row"] for x in dk], lambda row: rate[id(row)])
    ctrl = control_rows(R, rr)
    tc = calib_test(ctrl, lambda row: 0.0)
    T = [x["row"] for x in dk]
    ct = control_test(T, ctrl)
    cc = control_test(T, ctrl, lambda row: credit(row, rate[id(row)])[1])
    cs = stratified_control(T, ctrl)
    Th, Ch = without_hint_source(T, R), without_hint_source(ctrl, R)
    ch = control_test(Th, Ch)
    chs = stratified_control(Th, Ch)
    later = later_choice_targets()
    lr = [(skill, chara, later_choice_runs(R, skill, chara)) for skill, chara, _ in later]
    zero = [x for x in dk if credit(x["row"], 0.0)[1] <= 1e-12]
    c1, c0, nho = careers_held_out(dk, R, rows)
    ho = common.compare_held_out(c1, c0)
    odds = odds_change(runrows, lambda row: rate.get(id(row), 0.0))
    median, mx = (float(np.median(odds)), float(np.max(odds))) if odds else (float("nan"),) * 2
    lab = "odds change, targets with a positive chance, runs with a credited row"
    pre = "calibration"
    out: list[common.Value | common.Finding] = [
        V("careers: rows whose card is an R card, left out", skipped["R card"]),
        V("careers: acquired through that event", sum(x["hit"] for x in dk)),
        V("careers: acquired through that event only", sum(x["only"] for x in dk)),
        V("careers: mean fitted rate there", float(np.mean([x["rate"] for x in dk])) if dk else float("nan")),
        V("careers: rows the model gives no chance", len(zero)),
        V("careers: of them acquired through that event", sum(x["hit"] for x in zero)),
        V(f"{lab}: runs", len(odds)),
        V(f"{lab}: median", median),
        V(f"{lab}: max", mx),
        V(f"{pre}: rows", t.n),
        V(f"{pre}: obs / pred, left out", t.left_out.ratio),
        V(f"{pre}: ratio 95% low, left out", t.left_out.rlo),
        V(f"{pre}: ratio 95% high, left out", t.left_out.rhi),
        V(f"{pre}: obs / pred, credited", t.credited.ratio),
        V(f"{pre}: ratio 95% low, credited", t.credited.rlo),
        V(f"{pre}: ratio 95% high, credited", t.credited.rhi),
        V(f"{pre}: rows with a positive prediction", t.n_pos),
        V("held out: credited against left out: nats", ho.nats),
        V("held out: credited against left out: SE", ho.se),
        V("held out: credited against left out: z", ho.z),
        V("control: rows", tc.n),
        V("control: obs / pred, left out", tc.left_out.ratio),
        V("control: ratio 95% low", tc.left_out.rlo),
        V("control: ratio 95% high", tc.left_out.rhi),
        V("control: weighted to the treated targets", ct.control),
        V("control: difference", ct.diff),
        V("control: difference SE", ct.se),
        V("control: difference z", ct.z),
        V("control: credited minus control", cc.diff),
        V("control: credited minus control SE", cc.se),
        V("control: credited minus control z", cc.z),
        V("control, stratified: treated against the control's rate", cs[0]),
        V("control, stratified: SE", cs[1]),
        V("control, stratified: z", cs[2]),
        V("control, no training-hint source: treated rows", len(Th)),
        V("control, no training-hint source: control rows", len(Ch)),
        V("control, no training-hint source: treated", ch.treated),
        V("control, no training-hint source: control", ch.control),
        V("control, no training-hint source: difference", ch.diff),
        V("control, no training-hint source: difference z", ch.z),
        V("control, no training-hint source, stratified: treated against the control's rate", chs[0]),
        V("control, no training-hint source, stratified: z", chs[2]),
    ]
    name = dict(common.mdb().execute("select [index], text from text_data where category = 6"))
    for skill, chara, (k, n) in lr:
        out += [
            V(f"later choice, {name.get(chara)}, {skill}: runs prioritising it", n),
            V(f"later choice, {name.get(chara)}, {skill}: credited to her", k),
        ]
    return out + career_findings(t, ho, len(c1), nho, zero) + control_findings(ct, tc, lr, cc, cs, ch, chs)


def control_findings(
    ct: ControlTest,
    tc: CalibTest,
    lr: list[tuple[str, int, tuple[int, int]]],
    cc: ControlTest,
    cs: tuple[float, float, float],
    ch: ControlTest,
    chs: tuple[float, float, float],
) -> list[common.Value | common.Finding]:
    """the left-out shortfall against its control; the default targets the events touch"""
    _, marker, _, _ = markers()
    gn = common.skills()
    white = {gn[g][0] for g in common.DEFAULT}
    name = dict(common.mdb().execute("select [index], text from text_data where category = 6"))
    tops = sorted((name.get(c), m) for c, m in marker.items() if m in white)
    return [
        F(
            "Left out, the rows a deck card's random event could hint read above the same targets on decks with no "
            "card that has them as its marker, by more than 1.96 run-clustered standard errors: weighted to the same "
            "mix of targets, stratified by target and predicted acquisition, and on decks where no card's training "
            "hints give the target",
            lambda: ct.z > 1.96 and cs[2] > 1.96 and ch.z > 1.96,
            lambda: (
                f"treated {ct.treated:.3f}; control {tc.n} rows {tc.ratio_text(tc.left_out)}, weighted to the treated "
                f"targets {ct.control:.3f}; difference {ct.diff:+.3f}, SE {ct.se:.3f}, z {ct.z:+.2f}; stratified "
                f"{cs[0]:.3f} of the control's rate, z {cs[2]:+.2f}; no training-hint source: treated {ch.treated:.3f}, "
                f"control {ch.control:.3f}, z {ch.z:+.2f}, stratified {chs[0]:.3f}, z {chs[2]:+.2f}"
            ),
        ),
        F(
            "Credited at the fitted rate, the same rows read as the control: their difference is within 1.96 "
            "run-clustered standard errors",
            lambda: abs(cc.z) < 1.96,
            lambda: (
                f"credited {cc.treated:.3f} against control {cc.control:.3f}: {cc.diff:+.3f}, SE {cc.se:.3f}, z {cc.z:+.2f}"
            ),
        ),
        F(
            "Two default targets are a character's top-choice marker (Vodka's Nimble Navigator, Nice Nature's Ramp "
            "Up); one more sits on a later choice of a shared event, reached only when prioritised, and no run "
            "prioritising it with that character's card credits it to her",
            lambda: len(tops) == 2 and len(lr) == 1 and lr[0][2][0] == 0,
            lambda: (
                f"top-choice markers {tops}; later choices "
                + "; ".join(f"{name.get(c)} {s}: {k} of {n} runs prioritising it" for s, c, (k, n) in lr)
            ),
        ),
    ]


def career_findings(
    t: CalibTest, ho: common.HeldOutComparison, runs: int, nho: int, zero: list[RandomRow]
) -> list[common.Value | common.Finding]:
    """the careers' conclusions: t credited at the fitted rate"""
    return [
        F(
            "Deck cards' random events, left out: hybrid reconstructed acquisition on the rows they could hint is "
            "under-predicted (a known miss): obs / pred has a 95% interval entirely above 1",
            lambda: t.left_out.rlo > 1,
            lambda: (
                f"{t.n} rows, {t.k} acquired; left out {t.ratio_text(t.left_out)}; credited {t.ratio_text(t.credited)}"
            ),
        ),
        F(
            "Deck cards' random events, credited at the fitted rate: hybrid reconstructed acquisition obs / pred has a 95% interval covering 1",
            lambda: t.credited.rlo <= 1 <= t.credited.rhi,
            lambda: f"credited {t.ratio_text(t.credited)}",
        ),
        F(
            "For hybrid reconstructed acquisition on positive-baseline rows, crediting random events is not shown to predict better "
            "than leaving them out: the held-out gain is positive but z <= 1.96",
            lambda: ho.nats > 0 and ho.z <= 1.96,
            lambda: (
                f"{runs} runs, {nho} rows with a positive chance left out; rate fitted on the other four folds' readings: "
                f"{ho.nats:+.2f} nats, SE {ho.se:.2f}, z {ho.z:+.2f} (credited against left out)"
            ),
        ),
        F(
            "Left out, the model gives some of these targets no chance at all, and the event hinted the target on at least one of them",
            lambda: len(zero) > 0 and sum(x["hit"] for x in zero) > 0,
            lambda: (
                f"{len(zero)} rows at 0; {sum(x['row']['o_any'] for x in zero)} acquired, {sum(x['hit'] for x in zero)} through the event"
            ),
        ),
    ]


def careers_detail(R: list[Run]) -> None:
    a = fits(readings(R).rows)["exp"][0]
    rr, skipped = random_rows(R, a)
    print(
        "\ncareers: rows by who",
        collections.Counter((x["kind"], x["who"], x["row"]["g"], x["hit"]) for x in rr),
        dict(skipped),
    )
    rows, _ = calib()
    dev = max(abs(credit(x, 0.0)[1] - x["acq"]) for x in rows)
    print(
        f"careers: largest |acquisition from m recovered from p - the solver's acquisition| over {len(rows):,} rows: {dev:.2e}"
    )


def detail() -> None:
    R = common.runs()
    rows = readings(R).rows
    print("all records: rate by K", ", ".join(f"{k}: {h}/{n}" for k, (n, h) in sorted(by_k(rows).items())))
    t: collections.defaultdict[int, list[int]] = collections.defaultdict(lambda: [0, 0])
    for r in rows:
        t[r["rar"]][0] += 1
        t[r["rar"]][1] += r["hit"]
    print(
        "   by rarity (1 R, 2 SR, 3 SSR):", ", ".join(f"{k}: {h}/{n} = {h / n:.3f}" for k, (n, h) in sorted(t.items()))
    )
    careers_detail(R)


if __name__ == "__main__":
    common.main(claims)
    if common.have_runs():
        print()
        detail()
