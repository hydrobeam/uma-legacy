"""Section 10: how often a support card's character's shared random event fires, from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it), and what leaving these events out of
the model does on real careers. Every run record is used; one sample.

Markers (GameTora training_events_shared, through gametora.py): a character's shared event with two or more choices
whose top choice has a skill common to every outcome and whose other choices have no skill common to all their
outcomes; the top choice's skill is her marker (one per character). A reading is one run with a card of hers in the deck (any rarity, group cards aside), someone else
training, and the marker not given by that card's own events; a hit is the marker credited to "<her name> event" or to
"A or <her name> event", section 8's rule: a reading is unknown, and left out, when every credit naming her also names
another deck card whose events (its own, section 8's skill pools, or its character's shared events) can give the
marker. K = random events on offer: every shared event of the deck's characters plus the
trainee's own.
The fits are maximum likelihood over readings: a constant rate and the K form a (1 - b)^(K - 4). As stated in
RESEARCH.md, the population includes R cards; `detail()` prints the SR/SSR-only fit beside it.
Findings: the K form against the constant held out (5-fold cross-validation by common.fold(run id), each run scored by
the forms fitted on the other folds, compared with common.compare_held_out); tests on parameters (the slope in K; the
covariates in a logistic model next to K: card effects at their limit break from master.mdb in one fit, tested jointly
and each with Holm's adjustment, scenario, rarity within a character), run-clustered since a run gives several
readings: Huber-White sandwich Wald tests (robust(); likelihood ratios shown beside them); within-run independence
(pairs of hits against Binomial(n, p) per run).

Effect on real careers (section 13's calibration rows: each run rebuilt on its own deck, default targets, the solver's
per-target p, d^g, d^w, q, read through s13_calibration's build_jobs / run_solver / rows_of, cached results reused):
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
  calibration on the rows, acquisition (hinted from any source or owned) against the predicted acquisition with and
              without crediting: obs / pred with run-clustered 95% intervals (common.cluster_ci; a run can give
              several rows); held out, the same rows by 5-fold cross-validation (the rate fitted on the other folds'
              readings), per-run Bernoulli log-likelihoods credited against left out, over the rows with a positive
              prediction left out
"""

import collections, itertools, math, os
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from typing import Any, NamedTuple, TypedDict
import numpy as np
from scipy.optimize import minimize
import common
from common import Run

SEC: str = "10"
GEN_W, GEN_G = 0.2, 0.4  # section 12's W_k, G_k at n_k = 0


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


CalRow = dict[str, Any]  # one of section 13's calibration rows (s13_calibration.rows_of)
RunRows = dict[str, Any]  # one of section 13's per-run rows: its calibration rows under 'rows'
Fits = dict[str, Any]  # fits(): form -> (parameters, max LL)


class RandomRow(TypedDict):
    """random_rows(): a calibration row a random event could hint"""

    row: CalRow
    kind: str  # 'deck' (a deck card's character's event) or 'own' (the trainee's own)
    who: str | None  # the character's name
    rate: float | None  # the event's chance at the run's K ('own': filled in by careers())
    hit: bool  # the target credited to that event (credited(): section 8's rule, unknown counts as not)
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
    unknown: int  # every credit naming her also names another deck card whose events can give the marker


def event_pools(sc: dict[int, dict[str, Any]]) -> dict[int, set[str]]:
    """support card -> every skill an event credited to its character can give: the card's own events (section 8's
    skill pools, s08_chains.game().pool) and its character's shared events (GameTora training_events_shared)"""
    import s08_chains

    gt = common.gametora()
    sk = dict(common.mdb().execute("select [index], text from text_data where category = 47"))
    shared: collections.defaultdict[int, set[str]] = collections.defaultdict(set)
    for chara, es in gt.load("training_events_shared"):
        for e in es:
            for ch in gt.choices(e[1]):
                shared[chara] |= set().union(*outcome_skill_sets(ch[1], gt, sk))
    own = s08_chains.game().pool
    return {cid: own.get(cid, set()) | shared.get(x["char_id"], set()) for cid, x in sc.items()}


def credited(
    srcs: Sequence[str], nm: str, m: str, deck: dict[str, list[int]], pool: dict[int, set[str]]
) -> bool | None:
    """section 8's rule for the marker m of the character named nm, from the sources its hint was credited to: True when
    a credit names her ("<nm> event", or "A or <nm> event"); None (unknown) when every credit naming her also names
    another deck card (deck: bare name -> card ids) whose events can give m (pool); False when no credit names her"""
    credits = [[y.strip() for y in s[:-6].split(" or ")] for s in srcs if s.endswith(" event")]
    credits = [c for c in credits if nm in c]
    if credits and all(any(y != nm and any(m in pool.get(c, ()) for c in deck.get(y, ())) for y in c) for c in credits):
        return None
    return bool(credits)


def deck_names(r: Run) -> dict[str, list[int]]:
    """run r's deck by bare card name (as hint sources name a card): name -> card ids"""
    out: collections.defaultdict[str, list[int]] = collections.defaultdict(list)
    for cid, _, nm in r["deck"]:
        out[common.bare(nm)].append(cid)
    return out


def readings(R: list[Run]) -> ReadingSet:
    """every reading of the runs: a deck card of a character with a marker, someone else training, the marker not given
    by the card's own events; a reading whose credit is unknown (credited()) is left out and counted"""
    nshared, marker, cardsk, sc = markers()
    pool = event_pools(sc)
    name = dict(common.mdb().execute("select [index], text from text_data where category = 6"))
    rows: list[Reading] = []
    unknown = 0
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
            nm = name.get(ch)
            srcs = [s for h in r["hints"] if h["name"] == m for s in h["src"]]
            hit = credited(srcs, nm, m, byname, pool)
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
    return ReadingSet(rows, unknown)


def ll(y: np.ndarray, p: np.ndarray) -> float:
    """the Bernoulli log-likelihood of outcomes y at chances p (clipped to [1e-9, 1 - 1e-9])"""
    p = np.clip(p, 1e-9, 1 - 1e-9)
    return float(np.sum(y * np.log(p) + (1 - y) * np.log(1 - p)))


def fits(rows: Sequence[Reading]) -> Fits:
    """the constant's and the K form's maximum-likelihood fits over the readings: form -> (parameters, max LL)"""
    y = np.array([r["hit"] for r in rows], float)
    K = np.array([r["K"] for r in rows], float)
    out: Fits = {"const": (y.mean(), ll(y, np.full_like(y, y.mean())))}
    o = minimize(lambda s: -ll(y, s[0] * (1 - s[1]) ** (K - 4)), [0.2, 0.05], bounds=[(0.01, 0.99), (-0.5, 0.5)])
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
    rows, unk = readings(R)
    f = fits(rows)
    g: dict[str, Any] = {}  # value name -> value, in the order they are listed
    y = sum(r["hit"] for r in rows)
    n = len(rows)
    amb = sum(r["amb"] for r in rows)
    cz = sum(r["cens"] for r in rows)  # the marker at Lv 5 from other sources: the event could not show
    g.update(
        {
            "readings": n,
            "runs": len({r["run"] for r in rows}),
            "clean hits": y - amb,
            "ambiguous credits, unknown (left out)": unk,
            "rate where it could show, ambiguous as hits": (y + unk) / (n + unk - cz),
        }
    )
    g["constant p"] = f["const"][0]
    a, b = f["exp"][0]
    g["fitted P at K = 4"] = a
    g["fitted P at K = 14"] = a * (1 - b) ** 10
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
    """the covariates next to K in a logistic model, rarity within a character"""
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
    # rarity within a character (characters with cards of more than one rarity)
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
    o, e, z = pairs_test(rows, *ab)
    out.append(
        F(
            "Within a run events fire independently: pairs of one run's readings that both hit are within 1.96 sd of "
            "the count independence predicts",
            abs(z) < 1.96,
            f"{o:.0f} pairs observed, {e:.1f} expected, z {z:+.2f}",
        )
    )
    return out


def covariate_findings(rows: Sequence[Reading], g: dict[str, Any]) -> list[common.Finding]:
    """card effects and scenario next to K, rarity within a character"""
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
            "Next to K, of the card effects only the initial friendship gauge is detectable, and it raises the rate: in one "
            "fit of friendship bonus, initial friendship gauge and specialty priority, the gauge's Holm-adjusted "
            "run-clustered Wald p is below 0.05 with its 95% interval above 0, and the other two's are above 0.05",
            g[f"{gauge}: Holm p"] < 0.05
            and g[f"{gauge}: 95% low"] > 0
            and all(g[f"{k}: Holm p"] > 0.05 for k in others),
            f"joint {g['card effects: robust Wald chi2']:.2f} (df 3, limit {chi2.ppf(0.95, 3):.2f}, p "
            f"{g['card effects: robust p']:.4f}; LR {lr(card_effects(rows)):.2f}); {each}",
        )
    )
    out.append(
        F(
            "Scenario is not detectable next to K at the 5% level: run-clustered Wald p >= 0.05 (df 3)",
            g["scenario: robust p"] >= 0.05,
            f"scenario {g['scenario: robust Wald chi2']:.2f} (df {len(sc)}, limit {chi2.ppf(0.95, len(sc)):.2f}, p "
            f"{g['scenario: robust p']:.3f}; LR {lr_sc:.2f})",
        )
    )
    rar = np.array([r["rar"] for r in rows], float)
    ch, w = multi_rarity(rows)
    D = np.array([[rows[i]["char"] == c for c in ch] for i in w], float)
    fe = logit_ll(yb[w], np.column_stack([D, Kb[w]]))
    fe_r = logit_ll(yb[w], np.column_stack([D, Kb[w], rar[w]]))
    out.append(
        F(
            "Rarity shows no effect within a character (character intercepts and K, characters with cards of more than "
            "one rarity): run-clustered Wald |z| below 1.96 (95%); pooled across characters the likelihood-ratio statistic "
            "is shown alongside",
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
            "The slope in K does not predict unseen runs better than a constant rate: the form a (1 - b)^(K - 4) against a "
            "constant has z <= 1.96",
            z <= 1.96,
            f"{g['held out: runs']:,} runs, 5 folds: {nats:+.2f} nats, SE {se:.2f}, z {z:+.2f} ({lab})",
        )
    ]


# ---------------------------------------------------------------- effect on real careers (section 13's rows)
def calib() -> tuple[list[CalRow], list[RunRows]]:
    """section 13's calibration rows and per-run rows (default targets, each run on its own deck), exactly as
    s13_calibration.claims builds them; solver-result reuse follows s13_calibration.fingerprint"""
    if "calib" not in common._cache:
        import s13_calibration as s13

        R = common.runs()
        M = s13.Model()
        s13.card_variants()
        jobdir = os.path.join(s13.OUTD, "jobs8")
        man, files, _ = s13.build_jobs(R, M, False, jobdir)
        res = s13.run_solver(jobdir, files, os.path.join(s13.OUTD, "res8.jsonl"))
        rows, runrows, _ = s13.rows_of(R, M, res, man)
        common._cache["calib"] = (rows, runrows)
    return common._cache["calib"]


def event_groups() -> dict[str, Any]:
    """skill groups each kind of event can hint (GameTora, through gametora.py): a trainee's story and date events
    (training_events_char), each outfit's own events (training_events_char_card), a character's shared events
    (training_events_shared), a trainee's current conditional events with their conditions, each scenario's events
    (training_events_scenario; its ids are the game's scenario ids)"""
    if "evgroups" in common._cache:
        return common._cache["evgroups"]
    gt = common.gametora()
    grp = dict(common.mdb().execute("select id, group_id from skill_data"))

    def of(chs: Any) -> set[int | None]:
        return {
            grp.get(x)
            for ch in gt.choices(chs)
            for kind, x0, _ in gt.hints(ch[1])
            for x in ([x0] if kind == "hint" else x0)
        }

    def of_list(lst: Any) -> set[int | None]:
        return set().union(
            set(), *[of(e[1]) for e in (lst if isinstance(lst, list) else []) if isinstance(e, list) and len(e) >= 2]
        )

    ordinary: dict[int, set[int | None]] = {}
    cond: collections.defaultdict[int, list[tuple[set[int | None], Any]]] = collections.defaultdict(list)
    for row in gt.load("training_events_char"):
        ordinary[row[0]] = of_list(row[2]) | of_list(row[3])
        for tag, ve in gt.conditional_events(row):
            if tag.startswith("current"):
                cond[row[0]].append((of(ve[1]), gt.extra(ve, 0)))
    outfit = {row[0]: of_list(row[1]) for row in gt.load("training_events_char_card")}
    shared = {ch: of_list(es) for ch, es in gt.load("training_events_shared")}
    scen = {
        row[0]: set().union(set(), *[of_list(part) for part in row[1:]]) for row in gt.load("training_events_scenario")
    }
    common._cache["evgroups"] = out = {
        "ordinary": ordinary,
        "cond": cond,
        "outfit": outfit,
        "shared": shared,
        "scen": scen,
    }
    return out


def hint_srcs(r: Run) -> collections.defaultdict[int, list[common.Hint]]:
    """skill group -> the run's hint records of it (white or gold), names mapped as section 13 maps them"""
    names = common.skill_names()
    pn = {v[1]: gid for gid, vs in r.get("hint_groups", []) for v in vs}
    out: collections.defaultdict[int, list[common.Hint]] = collections.defaultdict(list)
    for h in r["hints"]:
        g = names[h["name"]][0] if h["name"] in names else pn.get(h["name"])
        if g is not None:
            out[g].append(h)
    return out


def free(row: CalRow) -> bool:
    """the row can move: not on the trainee's outfit, not a scenario gold"""
    return not row["outfit"] and not row["scen_gold"]


def credit(row: CalRow, r: float) -> tuple[float, float]:
    """(p, acquisition) of a calibration row with m multiplied by (1 - r) (section 12; m recovered from the row's p);
    a row on the trainee's outfit or a scenario gold does not move"""
    if not free(row):
        return row["p"], row["acq"]
    s = 1.1 ** row["nc"]
    dg = row["dg"]
    if dg >= 1:
        return row["p"], 1.0
    m = 1 - (row["p"] - dg * GEN_G * s) / ((1 - dg) * GEN_W * s)
    mm = m * (1 - r)
    return dg * GEN_G * s + (1 - dg) * GEN_W * s * (1 - mm), dg + (1 - dg) * (1 - mm)


def odds_change(runrows: Sequence[RunRows], rate: Callable[[CalRow], float]) -> list[float]:
    """relative change of each run's product of per-target odds when each row is credited at rate(row) (0: not), over
    the targets the model gives a positive chance without crediting: one per run with a credited row among them"""
    out: list[float] = []
    for pr in runrows:
        pos = [x for x in pr["rows"] if x["p"] > 0]
        if any(rate(x) > 0 and free(x) for x in pos):
            out.append(math.prod(credit(x, rate(x))[0] for x in pos) / math.prod(x["p"] for x in pos) - 1)
    return out


class CalibSide(NamedTuple):
    """calib_test() one way (left out or credited)"""

    pred: float  # mean predicted acquisition
    ratio: float  # obs / pred
    rlo: float  # its run-clustered 95% interval
    rhi: float
    ll: float  # the Bernoulli log-likelihood over the rows with a positive prediction left out


@dataclass
class CalibTest:
    """calib_test(): acquisition observed against predicted on a set of calibration rows"""

    n: int  # rows
    k: int  # acquired
    obs: float
    lo: float  # the observed rate's run-clustered 95% interval
    hi: float
    left_out: CalibSide
    credited: CalibSide
    n_pos: int  # rows with a positive prediction left out
    n_zero: int  # the rest

    def ratio_text(self, side: CalibSide) -> str:
        """obs / pred [95%] as the findings print it"""
        return f"{side.ratio:.3f} [{side.rlo:.2f}, {side.rhi:.2f}]"

    def ll_gain(self) -> float:
        return self.credited.ll - self.left_out.ll


def calib_test(rs: Sequence[CalRow], rate: Callable[[CalRow], float]) -> CalibTest:
    """acquisition observed (hinted from any source, or owned) against predicted, without and with crediting each row at
    rate(row): rows, observed (its 95% interval run-clustered), predicted, obs / pred and its 95% interval each way
    (common.cluster_ci: a run can give several rows, and the predictions differ by row); the Bernoulli log-likelihood each way
    over the rows whose prediction without crediting is positive (a row predicted at 0 and acquired has no finite
    likelihood; those rows are counted apart)"""
    n = len(rs)
    k = sum(1 for x in rs if x["o_any"])
    ob = [1 if x["o_any"] else 0 for x in rs]
    run = [x["run"] for x in rs]
    _, lo, hi, _ = common.cluster_ci(ob, [1] * n, run)
    p0 = [credit(x, 0.0)[1] for x in rs]
    p1 = [credit(x, rate(x))[1] for x in rs]
    pos = [i for i, q in enumerate(p0) if q > 1e-12]
    sides: list[CalibSide] = []
    for pr in (p0, p1):
        ep = common.div(sum(pr), n)
        ratio, rlo, rhi, _ = common.cluster_ci(ob, pr, run)
        sides.append(
            CalibSide(
                ep,
                ratio,
                rlo,
                rhi,
                sum(math.log(pr[i]) if rs[i]["o_any"] else math.log(1 - pr[i]) for i in pos if 0 < pr[i] < 1)
                + sum(0.0 if (pr[i] >= 1) == bool(rs[i]["o_any"]) else -math.inf for i in pos if not 0 < pr[i] < 1),
            )
        )
    return CalibTest(
        n=n,
        k=k,
        obs=common.div(k, n),
        lo=lo,
        hi=hi,
        left_out=sides[0],
        credited=sides[1],
        n_pos=len(pos),
        n_zero=n - len(pos),
    )


def calib_values(sec: str, pre: str, t: CalibTest) -> list[common.Value | common.Finding]:
    """a calibration's obs / pred left out and the log-likelihood gain from crediting, as Values of section `sec`"""
    V: Callable[[str, Any], common.Value] = lambda nm, v: common.Value(sec, nm, v)
    return [
        V(f"{pre}: obs / pred, left out", t.left_out.ratio),
        V(f"{pre}: ratio 95% low, left out", t.left_out.rlo),
        V(f"{pre}: ratio 95% high, left out", t.left_out.rhi),
        V(f"{pre}: LL credited minus left out, positive rows", t.ll_gain()),
    ]


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
            nm = name.get(x["char_id"])
            ev = f"{nm} event"
            out.append(
                RandomRow(
                    row=row,
                    kind="deck",
                    who=nm,
                    rate=a * (1 - b) ** (K - 4),
                    hit=credited(srcs, nm, white[g], deck_names(r), pool) is True,
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
    rr, _ = random_rows(R, ab)
    _, runrows = calib()
    dk = [x for x in rr if x["kind"] == "deck"]
    own = [x for x in rr if x["kind"] == "own"]
    r_own = common.div(sum(x["hit"] for x in own), len(own))
    for x in own:
        x["rate"] = r_own
    rate = {id(x["row"]): x["rate"] for x in rr}
    t = calib_test([x["row"] for x in dk], lambda row: rate[id(row)])
    zero = [x for x in dk if credit(x["row"], 0.0)[1] <= 1e-12]
    c1, c0, nho = careers_held_out(dk, R, rows)
    ho = common.compare_held_out(c1, c0)
    odds = odds_change(runrows, lambda row: rate.get(id(row), 0.0))
    median, mx = (float(np.median(odds)), float(np.max(odds))) if odds else (float("nan"),) * 2
    lab = "odds change, targets with a positive chance, runs with a credited row"
    pre = "calibration"
    out: list[common.Value | common.Finding] = [
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
    ]
    return out + career_findings(t, ho, len(c1), nho, zero)


def career_findings(
    t: CalibTest, ho: common.HeldOutComparison, runs: int, nho: int, zero: list[RandomRow]
) -> list[common.Value | common.Finding]:
    """the careers' conclusions: t credited at the fitted rate"""
    return [
        F(
            "Deck cards' random events, left out: acquisition on the rows they could hint is under-predicted: obs / pred "
            "has a 95% interval entirely above 1",
            lambda: t.left_out.rlo > 1,
            lambda: (
                f"{t.n} rows, {t.k} acquired; left out {t.ratio_text(t.left_out)}; credited {t.ratio_text(t.credited)}"
            ),
        ),
        F(
            "Deck cards' random events, credited at the fitted rate: acquisition obs / pred has a 95% interval covering 1",
            lambda: t.credited.rlo <= 1 <= t.credited.rhi,
            lambda: f"credited {t.ratio_text(t.credited)}",
        ),
        F(
            "Crediting deck cards' random events at the fitted rate is not shown to predict unseen runs' acquisition better "
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
    sr = [r for r in rows if r["rar"] >= 2]
    f = fits(sr)
    (a, b), L = f["exp"]
    print(
        f"   SR/SSR only: {len(sr)} readings, {len({r['run'] for r in sr})} runs, {sum(r['hit'] for r in sr)} hits; "
        f"constant {f['const'][0]:.3f} LL {f['const'][1]:.1f}; {a:.3f} (1 - {b:.3f})^(K-4) LL {L:.1f}"
    )
    careers_detail(R)


if __name__ == "__main__":
    common.main(claims)
    if common.have_runs():
        print()
        detail()
