"""Section 10: how often a support card's character's shared random event fires, from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it), and what leaving these events out of
the model does on real careers. Every run record is used; one sample.

Markers (GameTora training_events_shared, through gametora.py): a character's shared event with two or more choices
whose top choice has a skill common to every outcome and whose other choices have no skill common to all their
outcomes; the top choice's skill is her marker (one per character). A reading is one run with a card of hers in the deck (any rarity, group cards aside), someone else
training, and the marker not given by that card's own events; a hit is the marker credited to "<her name> event"
("A or B event" counts as a miss). K = random events on offer: every shared event of the deck's characters plus the
trainee's own.
The fits are maximum likelihood over readings: constant, a (1 - b)^(K - 4) form, power law, strict competition
1 - (1 - 1/K)^S, every event slot competing (chain links, random events and pals weighted in one pool), and a logistic
model in K. As stated in RESEARCH.md, the population includes R cards; `detail()` prints the SR/SSR-only fit beside it.
Findings: held-out comparisons of the forms (5-fold cross-validation by common.fold(run id), each run scored by the
forms fitted on the other folds, compared with common.compare_held_out); tests between nested forms and on parameters
(the slope in K, uniform competition, chain-link and pal weights, the logistic slope, the covariates next to K: card
effects at their limit break from master.mdb, scenario, rarity within a character), run-clustered since a run gives
several readings: Huber-White sandwich Wald tests, a score test where the estimate sits on its bound (robust();
likelihood ratios shown beside them); within-run independence (pairs of hits against Binomial(n, p) per run); and, over
every run that shows a priority list, how often a bottom-choice skill of these events was prioritised and credited to
the event (prioritised or not; a run gives one candidate per such skill, so the rate's interval is run-clustered,
common.cluster_ci), against the pooled top-choice marker rate. The pal weight's intervals, in the c / T form: estimate
+- 1.96 run-clustered SE (the finding's test) and the profile likelihood.

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
  odds        each run's product of per-target odds, relative change from crediting: over the eight targets (mean,
              median, max over runs with a nonzero product, and over those with a credited row), and over the targets
              the model gives a positive chance without crediting (runs with a credited row among them)
  calibration on the rows, acquisition (hinted from any source or owned) against the predicted acquisition with and
              without crediting: obs / pred with run-clustered 95% intervals (common.cluster_ci; a run can give
              several rows), and the Bernoulli log-likelihood difference;
              held out, the same rows by 5-fold cross-validation (the rate fitted on the other folds' readings), per-run
              log-likelihoods credited against left out
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
    amb: int  # 1: credited only through an "A or B event" naming her
    cens: int  # 1: no credit, the marker already at Lv 5 from other sources (the event could not show)
    K: int  # random events on offer in the run (offer())
    pals: int  # pal cards in the deck
    links: int  # chain links of all non-pal cards in the deck
    char: int  # her character id
    lb: int  # the card's limit break
    scen: int  # scenario id


CalRow = dict[str, Any]  # one of section 13's calibration rows (s13_calibration.rows_of)
RunRows = dict[str, Any]  # one of section 13's per-run rows: its calibration rows under 'rows'
Fits = dict[str, Any]  # fits(): form -> (parameters, max LL); 'slots form' -> index into SLOT_FNS


class RandomRow(TypedDict):
    """random_rows(): a calibration row a random event could hint"""

    row: CalRow
    kind: str  # 'deck' (a deck card's character's event) or 'own' (the trainee's own)
    who: str | None  # the character's name
    rate: float | None  # the event's chance at the run's K ('own': filled in by careers())
    hit: bool  # the target credited to that event
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


def readings(R: list[Run]) -> list[Reading]:
    """every reading of the runs: a deck card of a character with a marker, someone else training, the marker not given
    by the card's own events"""
    nshared, marker, cardsk, sc = markers()
    name = dict(common.mdb().execute("select [index], text from text_data where category = 6"))
    wc = common.cards()
    rows: list[Reading] = []
    for r in R:
        dk = [d[0] for d in r["deck"]]
        tr = r["chara"]
        lbs = {d[0]: d[1] for d in r["deck"]}
        pals = sum(1 for x in dk if wc.get(x, {}).get("pal"))
        links = sum(wc.get(x, {}).get("links", 0) for x in dk if not wc.get(x, {}).get("pal"))
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
            hit = f"{nm} event" in srcs
            amb = not hit and any(nm in s and " or " in s for s in srcs)
            # censored: the marker already at hint Lv 5 from other sources, so the event's hint could not show
            cens = not hit and not amb and max([h["lv"] or 0 for h in r["hints"] if h["name"] == m] or [0]) >= 5
            rows.append(
                Reading(
                    run=r["id"],
                    card=cid,
                    rar=x["rarity"],
                    hit=int(hit),
                    amb=int(amb),
                    cens=int(cens),
                    K=K,
                    pals=pals,
                    links=links,
                    char=ch,
                    lb=lbs[cid],
                    scen=r["scenario"],
                )
            )
    return rows


def ll(y: np.ndarray, p: np.ndarray) -> float:
    """the Bernoulli log-likelihood of outcomes y at chances p (clipped to [1e-9, 1 - 1e-9])"""
    p = np.clip(p, 1e-9, 1 - 1e-9)
    return float(np.sum(y * np.log(p) + (1 - y) * np.log(1 - p)))


def fits(rows: Sequence[Reading]) -> Fits:
    """each form's maximum-likelihood fit over the readings: form -> (parameters, max LL)"""
    y = np.array([r["hit"] for r in rows], float)
    K = np.array([r["K"] for r in rows], float)
    Lk = np.array([r["links"] for r in rows], float)
    Pl = np.array([r["pals"] for r in rows], float)
    out: Fits = {"const": (y.mean(), ll(y, np.full_like(y, y.mean())))}
    o = minimize(lambda s: -ll(y, s[0] * (1 - s[1]) ** (K - 4)), [0.2, 0.05], bounds=[(0.01, 0.99), (-0.5, 0.5)])
    out["exp"] = (tuple(o.x), -o.fun)
    o = minimize(lambda s: -ll(y, s[0] * (K / 8) ** (-s[1])), [0.17, 0.5], bounds=[(0.01, 0.9), (-3, 3)])
    out["power"] = (tuple(o.x), -o.fun)
    o = minimize(lambda s: -ll(y, 1 - (1 - 1 / K) ** s[0]), [1.0], bounds=[(0.01, 50)])
    out["strict"] = (o.x[0], -o.fun)
    best: tuple[tuple[float, ...], float] | None = None
    for i, fn in enumerate(SLOT_FNS):
        o = fit_slot_form(fn, y, K, Lk, Pl)
        if best is None or -o.fun > best[1]:
            best = (tuple(o.x), -o.fun)
            out["slots form"] = i
    out["slots"] = best
    return out


def fit_slot_form(fn: Callable[[Any, Any], Any], y: np.ndarray, K: np.ndarray, Lk: np.ndarray, Pl: np.ndarray) -> Any:
    """one every-slot form fitted: parameters (s, pal weight, chain-link weight), T = link x links + K + pal x pals"""
    return minimize(
        lambda p: -ll(y, fn(p[0], p[2] * Lk + K + p[1] * Pl)), [3.0, 3.0, 1.0], bounds=[(0.01, 200), (0, 30), (0, 10)]
    )


# the every-slot forms: P(fires) from s and the slot total T, 1 - (1 - 1/T)^s (s draws, uniform) or s / T
SLOT_FNS: tuple[Callable[[Any, Any], Any], ...] = (lambda s, T: 1 - (1 - 1 / T) ** s, lambda s, T: s / T)


def form_p(f: Fits, rows: Sequence[Reading]) -> dict[str, np.ndarray]:
    """each form's P(fires) on `rows`, with the parameters of fits() output f"""
    K = np.array([r["K"] for r in rows], float)
    Lk = np.array([r["links"] for r in rows], float)
    Pl = np.array([r["pals"] for r in rows], float)
    (a, b), (c, pa), S, (sc, sw, sl) = f["exp"][0], f["power"][0], f["strict"][0], f["slots"][0]
    return {
        "const": np.full_like(K, f["const"][0]),
        "exp": a * (1 - b) ** (K - 4),
        "power": c * (K / 8) ** (-pa),
        "strict": 1 - (1 - 1 / K) ** S,
        "slots": SLOT_FNS[f["slots form"]](sc, sl * Lk + K + sw * Pl),
    }


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


def scen_fit(rows: Sequence[Reading], scen: list[int]) -> np.ndarray:
    """the K form with a per-scenario factor, a_s (1 - b)^(K - 4), s the run's scenario (indices into `scen`): (a_s..., b)"""
    y = np.array([r["hit"] for r in rows], float)
    K = np.array([r["K"] for r in rows], float)
    si = np.array([scen.index(r["scen"]) for r in rows])
    o = minimize(
        lambda q: -ll(y, q[:-1][si] * (1 - q[-1]) ** (K - 4)),
        [0.2] * len(scen) + [0.05],
        bounds=[(0.01, 0.99)] * len(scen) + [(-0.5, 0.5)],
    )
    return o.x


def held_out_scenario(rows: Sequence[Reading]) -> tuple[list[float], list[float]]:
    """rule 3 for scenario next to K: per-run held-out log-likelihood of the K form with a per-scenario factor
    (scen_fit) and without (fits()' a (1 - b)^(K - 4)), each fold scored by the forms fitted on the other four folds
    (common.fold(run id)). Returns (with, without), lists over runs sorted by id"""
    scen = sorted({r["scen"] for r in rows})
    w1: collections.defaultdict[int, float] = collections.defaultdict(float)
    w0: collections.defaultdict[int, float] = collections.defaultdict(float)
    for k in range(common.K_FOLDS):
        tr = [r for r in rows if common.fold(r["run"]) != k]
        te = [r for r in rows if common.fold(r["run"]) == k]
        q = scen_fit(tr, scen)
        a, b = fits(tr)["exp"][0]
        for r in te:
            y = r["hit"]
            for d, p in (
                (w1, q[scen.index(r["scen"])] * (1 - q[-1]) ** (r["K"] - 4)),
                (w0, a * (1 - b) ** (r["K"] - 4)),
            ):
                p = min(max(p, 1e-9), 1 - 1e-9)
                d[r["run"]] += math.log(p) if y else math.log(1 - p)
    runs = sorted(w0)
    return [w1[i] for i in runs], [w0[i] for i in runs]


def by_k(rows: Sequence[Reading]) -> collections.defaultdict[int, list[int]]:
    """K (14 and over together) -> [readings, hits]"""
    t: collections.defaultdict[int, list[int]] = collections.defaultdict(lambda: [0, 0])
    for r in rows:
        k = min(r["K"], 14)
        t[k][0] += 1
        t[k][1] += r["hit"]
    return t


def logistic_k(rows: Sequence[Reading]) -> np.ndarray:
    """logit P = b0 + b1 K"""
    y = np.array([r["hit"] for r in rows], float)
    X = np.column_stack([np.ones(len(rows)), [r["K"] for r in rows]])
    o = minimize(lambda b: float(np.sum(np.logaddexp(0, X @ b) - y * (X @ b))), np.zeros(2), method="BFGS")
    return o.x


class BottomChoice(NamedTuple):
    """bottom_prioritised(): over the runs showing a priority list"""

    shown: int  # runs showing a priority list
    prioritised: int  # (run, skill) candidates prioritised
    candidates: int  # (run, skill) candidates
    hits: collections.Counter[tuple[bool, bool]]  # (prioritised, credited to her event) -> candidates
    each: list[tuple[int, bool, bool]]  # per candidate: (run id, prioritised, credited)


def bottom_prioritised(R: list[Run]) -> BottomChoice:
    """runs showing a priority list, and how many prioritise a bottom-choice-only skill of a deck card's character;
    also each (run, skill) candidate as (run id, prioritised, credited to her event), several to a run"""
    gt = common.gametora()
    db = common.mdb()
    sk = dict(db.execute("select [index], text from text_data where category = 47"))
    bottom: collections.defaultdict[int, set[str]] = collections.defaultdict(
        set
    )  # character -> bottom-choice-only skills
    for chara, es in gt.load("training_events_shared"):
        for e in es:
            chs = gt.choices(e[1])
            if len(chs) < 2:
                continue
            a, b = sure_skills(chs[0][1], gt, sk), set().union(*[sure_skills(ch[1], gt, sk) for ch in chs[1:]])
            if b and not a:
                bottom[chara] |= b
    _, _, cardsk, sc = markers()
    name = dict(common.mdb().execute("select [index], text from text_data where category = 6"))
    out = BottomChoice(0, 0, 0, collections.Counter(), [])
    shown = n = cand = 0
    for r in R:
        p = r.get("priority")
        if p is None:
            continue
        shown += 1
        for cid in (d[0] for d in r["deck"]):
            x = sc.get(cid)
            if not x or x.get("type") == "group" or x["char_id"] == r["chara"]:
                continue
            ev = f"{name.get(x['char_id'])} event"
            for sk_ in bottom[x["char_id"]] - cardsk[cid]:
                n += sk_ in p
                cand += 1
                c = (sk_ in p, any(h["name"] == sk_ and ev in h["src"] for h in r["hints"]))
                out.hits[c] += 1
                out.each.append((r["id"], *c))
    return out._replace(shown=shown, prioritised=n, candidates=cand)


def claims() -> list[common.Value | common.Finding]:
    """the readings' rates and fits, the tests on them, and the effect on real careers"""
    R = common.runs()
    if R is None:
        return []
    rows = readings(R)
    f = fits(rows)
    g: dict[str, Any] = {}  # value name -> value, in the order they are listed
    y = sum(r["hit"] for r in rows)
    n = len(rows)
    amb = sum(r["amb"] for r in rows)
    g.update(
        {
            "readings": n,
            "runs": len({r["run"] for r in rows}),
            "clean hits": y,
            "ambiguous credits": amb,
            "rate with ambiguous, points higher": 100 * amb / n,
        }
    )
    cz = sum(r["cens"] for r in rows)
    g.update(
        {
            "readings where the event could not show (marker at Lv 5 from other sources)": cz,
            "rate where the event could show": y / (n - cz),
            "rate where it could show, ambiguous as hits": (y + amb) / (n - cz),
        }
    )
    g["constant p"], g["LL constant"] = f["const"]
    (a, b), g["LL fitted form"] = f["exp"]
    g["fitted form: P at K = 4"] = a
    g["fitted form: 1 - ratio per event"] = b
    (_, g["power law a"]), g["LL power law"] = f["power"]
    g["strict competition S"], g["LL strict competition"] = f["strict"]
    (
        (
            g["every slot competing: c"],
            g["every slot competing: pal weight"],
            g["every slot competing: chain-link weight"],
        ),
        g["LL every slot competing"],
    ) = f["slots"]
    t = by_k(rows)
    for k in (4, 6, 8, 10, 12, 14):
        g[f"hit rate K = {k}"] = t[k][1] / t[k][0]
    g["fitted P overall"] = float(np.mean([a * (1 - b) ** (r["K"] - 4) for r in rows]))
    g["fitted P at K = 4"] = a
    g["fitted P at K = 14"] = a * (1 - b) ** 10
    b0, b1 = logistic_k(rows)
    for rr, lab in ((3, "SSR"), (2, "SR")):
        xs = [r["hit"] for r in rows if r["rar"] == rr]
        g[f"pooled: {lab} marker rate"] = sum(xs) / len(xs)
    g["logistic: rate at K = 8"] = 1 / (1 + math.exp(-(b0 + 8 * b1)))
    g["logistic: rate at K = 14"] = 1 / (1 + math.exp(-(b0 + 14 * b1)))
    g["runs showing a priority list"], g["bottom-choice markers prioritised there"], cand, bh, each = (
        bottom_prioritised(R)
    )
    g["bottom-choice candidates there"] = cand
    g["bottom-choice, not prioritised: credited to her event"] = bh[(False, True)]
    g["bottom-choice, prioritised: credited to her event"] = bh[(True, True)]
    g["every slot competing: LL over constant"] = f["slots"][1] - f["const"][1]
    g["every slot competing: pal weight 95% low"], g["every slot competing: pal weight 95% high"] = pal_interval(rows)
    g["bottom-choice, prioritised: credited rate"] = common.div(bh[(True, True)], bh[(True, True)] + bh[(True, False)])
    g["bottom-choice, not prioritised: credited rate"] = common.div(
        bh[(False, True)], bh[(False, True)] + bh[(False, False)]
    )
    np_ = [(i, c) for i, pr, c in each if not pr]  # a run gives one candidate per bottom-choice skill: run-clustered
    _, g["bottom-choice, not prioritised: 95% low"], g["bottom-choice, not prioritised: 95% high"], _ = (
        common.cluster_ci([c for _, c in np_], [1] * len(np_), [i for i, _ in np_])
    )
    g["top-choice marker rate"] = y / n
    g.update(robust(rows, f))
    h = held_out(rows)
    g["held out: runs"] = len(h["exp"])
    (
        g["held out: scenario next to K: nats"],
        g["held out: scenario next to K: SE"],
        g["held out: scenario next to K: z"],
    ) = common.compare_held_out(*held_out_scenario(rows))
    for lab, x, z in HELD_OUT:
        g[f"held out: {lab}: nats"], g[f"held out: {lab}: SE"], g[f"held out: {lab}: z"] = common.compare_held_out(
            h[x], h[z]
        )
    out = [common.Value(SEC, name, v) for name, v in g.items()] + findings(rows, f, g, cand, bh)
    return out + careers(R, (a, b), rows)


# rule 3's comparisons of the forms: (label, first form, second form); nats, SE and z are first minus second
HELD_OUT: tuple[tuple[str, str, str], ...] = (
    ("fitted form against constant", "exp", "const"),
    ("every slot competing against fitted form", "slots", "exp"),
    ("power law against fitted form", "power", "exp"),
    ("strict competition against fitted form", "strict", "exp"),
)


# ---------------------------------------------------------------- findings: each conclusion as a likelihood or interval test
def slot_fit(
    rows: Sequence[Reading], link: float | None = None, pal: float | None = None
) -> tuple[tuple[float, float, float], float]:
    """the c / T fit, T = link x chain links + K + pal x pals, with either weight held fixed: ((c, pal, link), max LL)"""
    y = np.array([r["hit"] for r in rows], float)
    K = np.array([r["K"] for r in rows], float)
    Lk = np.array([r["links"] for r in rows], float)
    Pl = np.array([r["pals"] for r in rows], float)
    fx: Callable[[float | None, float], float] = lambda v, x: x if v is None else v
    o = minimize(
        lambda q: -ll(y, q[0] / (fx(link, q[2]) * Lk + K + fx(pal, q[1]) * Pl)),
        [3.0, 1.0, 0.5],
        bounds=[(0.01, 200), (0, 30), (0, 10)],
    )
    return (o.x[0], fx(pal, o.x[1]), fx(link, o.x[2])), -o.fun


def slot_ll(rows: Sequence[Reading], link: float | None = None, pal: float | None = None) -> float:
    """max LL of c / T, T = link x chain links + K + pal x pals, with either weight held fixed"""
    return slot_fit(rows, link, pal)[1]


def pal_interval(rows: Sequence[Reading]) -> tuple[float, float]:
    """the pal weight's 95% profile-likelihood interval in the c / T form (chain-link weight and c free): where the
    profile falls 1.92 nats below its maximum"""
    from scipy.optimize import brentq, minimize_scalar

    prof: Callable[[float], float] = lambda w: slot_ll(rows, pal=w)
    o = minimize_scalar(lambda w: -prof(w), bounds=(0, 10), method="bounded")
    top, w0 = -o.fun, o.x
    d: Callable[[float], float] = lambda w: top - prof(w) - 1.92
    lo = brentq(d, 0.0, w0) if d(0.0) > 0 else 0.0
    hi = brentq(d, w0, 10.0) if d(10.0) > 0 else float("nan")
    return lo, hi


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


def score_z(S: np.ndarray, H: np.ndarray, j: int) -> float:
    """run-clustered score test of parameter j at its null value (S, H at the fit restricted to it; the other parameters
    fitted): the efficient score's sum over runs over its run-clustered SD, a z (Boos 1992)"""
    o = [i for i in range(S.shape[1]) if i != j]
    s = S[:, j] - (S[:, o] @ np.linalg.solve(H[np.ix_(o, o)], H[o, j]) if o else 0)
    C = len(s)
    return float(s.sum() / np.sqrt(C / (C - 1) * (s**2).sum()))


class ReadingArrays(NamedTuple):
    """the readings as arrays, by reading"""

    y: np.ndarray  # hit
    K: np.ndarray
    Lk: np.ndarray  # chain links
    Pl: np.ndarray  # pals
    run: list[int]


def reading_arrays(rows: Sequence[Reading]) -> ReadingArrays:
    return ReadingArrays(
        np.array([r["hit"] for r in rows], float),
        np.array([r["K"] for r in rows], float),
        np.array([r["links"] for r in rows], float),
        np.array([r["pals"] for r in rows], float),
        [r["run"] for r in rows],
    )


def robust_z(b: np.ndarray, V: np.ndarray, naive: np.ndarray, j: int, null: float = 0.0) -> tuple[float, float]:
    """Wald z of parameter j against `null` with the run-clustered and the model covariance"""
    return (b[j] - null) / np.sqrt(V[j, j]), (b[j] - null) / np.sqrt(naive[j, j])


def robust(rows: Sequence[Reading], f: Fits) -> dict[str, float]:
    """each likelihood-ratio / profile test of the findings, redone run-clustered: numbers by name"""
    x = reading_arrays(rows)
    g = form_tests(x, f) | slot_tests(rows, x) | covariate_tests(rows, x)
    return {k: float(v) for k, v in g.items()}


def form_tests(x: ReadingArrays, f: Fits) -> dict[str, Any]:
    """the slope in K and uniform competition"""
    y, K, run = x.y, x.K, x.run
    g: dict[str, Any] = {}
    # slope in K: b of a (1 - b)^(K - 4), Wald against 0
    ab = np.array(f["exp"][0])
    V, Vn = sandwich(*run_scores(lambda t: t[0] * (1 - t[1]) ** (K - 4), y, run, ab, [0, 1]))
    g["slope in K: robust z"], g["slope in K: model z"] = robust_z(ab, V, Vn, 1)
    # uniform competition: a of c (K / 8)^(-a), Wald against 1
    ca = np.array(f["power"][0])
    V, Vn = sandwich(*run_scores(lambda t: t[0] * (K / 8) ** (-t[1]), y, run, ca, [0, 1]))
    g["uniform competition (power law a = 1): robust z"], g["uniform competition (power law a = 1): model z"] = (
        robust_z(ca, V, Vn, 1, 1.0)
    )
    return g


def slot_tests(rows: Sequence[Reading], x: ReadingArrays) -> dict[str, Any]:
    """the c / T form's weights: chain-link weight 1 (a score test) and the pal weight's interval"""
    y, K, Lk, Pl, run = x
    g: dict[str, Any] = {}
    # c / T: t = (c, pal weight, chain-link weight)
    ct: Callable[[np.ndarray], np.ndarray] = lambda t: t[0] / (t[2] * Lk + K + t[1] * Pl)
    # Score test at chain-link weight 1, fitting c and the pal weight under the null.
    th1, _ = slot_fit(rows, link=1.0)
    S, H = run_scores(ct, y, run, th1, [0, 1, 2])
    g["chain-link weight 1: robust score z"] = score_z(S, H, 2)
    g["chain-link weight 1: model score z"] = float(
        S[:, 2].sum() - H[2, :2] @ np.linalg.solve(H[:2, :2], S[:, :2].sum(0))
    ) / np.sqrt(-(H[2, 2] - H[2, :2] @ np.linalg.solve(H[:2, :2], H[:2, 2])))
    # Pal-weight interval: hold the chain-link weight fixed if its fit is at the lower bound.
    th, _ = slot_fit(rows)
    free = [0, 1] + ([2] if th[2] > 1e-6 else [])
    V, Vn = sandwich(*run_scores(ct, y, run, th, free))
    g["every slot competing: pal weight robust SE"] = se = float(np.sqrt(V[1, 1]))
    g["every slot competing: pal weight robust 95% low"], g["every slot competing: pal weight robust 95% high"] = (
        th[1] - 1.96 * se,
        th[1] + 1.96 * se,
    )
    g["every slot competing: pal weight model SE"] = float(np.sqrt(Vn[1, 1]))
    return g


def covariate_tests(rows: Sequence[Reading], x: ReadingArrays) -> dict[str, Any]:
    """the logistic slope in K, the covariates next to K, rarity within a character"""
    y, K, run = x.y, x.K, x.run
    g: dict[str, Any] = {}
    one = np.ones(len(rows))
    X = np.column_stack([one, K])
    b = logit_fit(y, X)
    V, Vn = sandwich(*logit_scores(y, X, run, b))
    g["logistic slope in K: robust z"], g["logistic slope in K: model z"] = robust_z(b, V, Vn, 1)
    # covariates next to K
    cache: dict[tuple[int, int, int], float] = {}
    cov: dict[str, list[np.ndarray]] = {
        nm: [
            np.array([cache.setdefault((r["card"], r["lb"], t), card_effect(r["card"], r["lb"], t)) for r in rows]) / 10
        ]
        for t, nm in COVARIATES
    }
    cov["scenario"] = [np.array([r["scen"] == v for r in rows], float) for v in sorted({r["scen"] for r in rows})[1:]]
    for nm, cols in cov.items():
        X = np.column_stack([one, K, *cols])
        b = logit_fit(y, X)
        V, _ = sandwich(*logit_scores(y, X, run, b))
        g[f"{nm}: robust Wald chi2"], _, g[f"{nm}: robust p"] = wald(b, V, list(range(2, X.shape[1])))
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


def findings(
    rows: Sequence[Reading], f: Fits, g: dict[str, Any], cand: int, bh: collections.Counter[tuple[bool, bool]]
) -> list[common.Finding]:
    """the readings' conclusions, each as a likelihood or interval test (g: value name -> value)"""
    return form_findings(rows, f, g) + covariate_findings(rows, g) + [bottom_choice_finding(g, cand, bh)]


def rz(g: dict[str, Any], nm: str) -> str:
    """a test's run-clustered z beside the one with independent readings"""
    return f"run-clustered z {g[nm + ': robust z']:+.2f} (independent readings {g[nm + ': model z']:+.2f})"


def form_findings(rows: Sequence[Reading], f: Fits, g: dict[str, Any]) -> list[common.Finding]:
    """the form in K, competition, chain links, pals and within-run independence"""
    L0, (ab, Le), (pw, Lp), (_, Ls), Lslot = f["const"][1], f["exp"], f["power"], f["strict"], f["slots"][1]
    out = [
        F(
            "The slope in K is detectable: the adopted form a (1 - b)^(K - 4) against a constant (b = 0, nested), "
            "run-clustered Wald |z| above 1.96 (95%)",
            abs(g["slope in K: robust z"]) > 1.96,
            f"a {ab[0]:.2f}, b {ab[1]:.3f}: LL {Le:.1f} against {L0:.1f}: LR {2 * (Le - L0):.2f}; {rz(g, 'slope in K')}",
        )
    ]
    out += held_out_findings(g)
    y = np.array([r["hit"] for r in rows], float)
    K = np.array([r["K"] for r in rows], float)
    L1 = -minimize(lambda q: -ll(y, q[0] * (K / 8) ** -1.0), [0.17], bounds=[(0.01, 0.9)]).fun
    uc = "uniform competition (power law a = 1)"
    out.append(
        F(
            "Competition is far weaker than uniform: in the power law, a = 1 (uniform competition) is rejected against "
            "the fitted a, run-clustered Wald |z| above 1.96 (95%)",
            abs(g[f"{uc}: robust z"]) > 1.96,
            f"a = {pw[1]:.2f}: LL {Lp:.1f}; a = 1: LL {L1:.1f}; LR {2 * (Lp - L1):.2f}; {rz(g, uc)}",
        )
    )
    out.append(
        F(
            "Strict competition 1 - (1 - 1/K)^S has a lower in-sample log-likelihood than a constant, which has the "
            "same number of parameters (one)",
            Ls < L0,
            f"LL {Ls:.1f} against constant {L0:.1f}",
        )
    )
    Lk1 = slot_ll(rows, link=1.0)
    out.append(
        F(
            "Chain links play no part: their fitted weight is 0, and weight 1 (a link competing like a random event) "
            "is rejected, run-clustered score |z| above 1.96 (95%; the fitted weight sits on its bound 0, so the test is "
            "made at weight 1)",
            g["every slot competing: chain-link weight"] < 0.01
            and abs(g["chain-link weight 1: robust score z"]) > 1.96,
            f"weight {g['every slot competing: chain-link weight']:.2f}: LL {Lslot:.1f}; weight 1: LL {Lk1:.1f}; "
            f"LR {2 * (Lslot - Lk1):.2f}; run-clustered score z {g['chain-link weight 1: robust score z']:+.2f} "
            f"(independent readings {g['chain-link weight 1: model score z']:+.2f})",
        )
    )
    plo, phi = g["every slot competing: pal weight 95% low"], g["every slot competing: pal weight 95% high"]
    rlo, rhi = (
        g["every slot competing: pal weight robust 95% low"],
        g["every slot competing: pal weight robust 95% high"],
    )
    out.append(
        F(
            "Pals compete with random events: pal weight 0 is rejected (outside the run-clustered 95% interval, "
            "estimate +- 1.96 robust SE)",
            rlo > 0,
            f"weight {g['every slot competing: pal weight']:.2f}, run-clustered [{rlo:.2f}, {rhi:.2f}] (SE "
            f"{g['every slot competing: pal weight robust SE']:.3f}; independent readings "
            f"{g['every slot competing: pal weight model SE']:.3f}); profile likelihood [{plo:.2f}, {phi:.2f}]; "
            f"LL at 0 {slot_ll(rows, pal=0.0):.1f}, at 1 {slot_ll(rows, pal=1.0):.1f}, maximum {slot_ll(rows):.1f}",
        )
    )
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
    """the logistic slope, card effects and scenario next to K, rarity within a character"""
    from scipy.stats import chi2

    out: list[common.Finding] = []
    yb = np.array([r["hit"] for r in rows], float)
    one = np.ones(len(rows))
    Kb = np.array([r["K"] for r in rows], float)
    base = logit_ll(yb, np.column_stack([one, Kb]))
    Lc = logit_ll(yb, one[:, None])
    out.append(
        F(
            "In logistic form too the slope is detectable: K's run-clustered Wald |z| above 1.96 (95%), and the rate falls",
            abs(g["logistic slope in K: robust z"]) > 1.96
            and g["logistic: rate at K = 14"] < g["logistic: rate at K = 8"],
            f"LR {2 * (base - Lc):.2f}; {rz(g, 'logistic slope in K')}; {g['logistic: rate at K = 8']:.3f} at 8, "
            f"{g['logistic: rate at K = 14']:.3f} at 14",
        )
    )
    # covariates next to K: likelihood-ratio tests at 95%
    lr: Callable[[list[np.ndarray]], float] = lambda cols: 2 * (logit_ll(yb, np.column_stack([one, Kb, *cols])) - base)
    eff: dict[str, tuple[float, int]] = {}  # covariate -> (LR, df)
    for t, nm in COVARIATES:
        cache: dict[tuple[int, int], float] = {}
        x = np.array([cache.setdefault((r["card"], r["lb"]), card_effect(r["card"], r["lb"], t)) for r in rows]) / 10
        eff[nm] = (lr([x]), 1)
    sc = sorted({r["scen"] for r in rows})[1:]
    eff["scenario"] = (lr([np.array([r["scen"] == v for r in rows], float) for v in sc]), len(sc))
    ev: Callable[[Sequence[str]], str] = lambda ks: "; ".join(
        f"{k} {g[k + ': robust Wald chi2']:.2f} (df {eff[k][1]}, limit {chi2.ppf(0.95, eff[k][1]):.2f}, "
        f"p {g[k + ': robust p']:.3f}; LR {eff[k][0]:.2f})"
        for k in ks
    )
    fr = tuple(nm for _, nm in COVARIATES)
    out.append(
        F(
            "Friendship (bonus, initial gauge) and specialty priority show no effect next to K: each run-clustered Wald "
            "p > 0.05",
            all(g[f"{k}: robust p"] > 0.05 for k in fr),
            ev(fr),
        )
    )
    out.append(
        F(
            "Scenario is detectable next to K at the 5% level: run-clustered Wald p < 0.05 (df 3)",
            g["scenario: robust p"] < 0.05,
            ev(("scenario",)),
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


def bottom_choice_finding(g: dict[str, Any], cand: int, bh: collections.Counter[tuple[bool, bool]]) -> common.Finding:
    """bottom-choice skills against top-choice markers"""
    return F(
        "Bottom-choice skills are credited to the character's event less often than top-choice markers: the "
        "non-prioritised bottom-choice rate's 95% interval lies entirely below the pooled top-choice marker rate",
        g["bottom-choice, not prioritised: 95% high"] < g["top-choice marker rate"],
        f"not prioritised {bh[(False, True)]}/{bh[(False, True)] + bh[(False, False)]} = "
        f"{g['bottom-choice, not prioritised: credited rate']:.3f} [{g['bottom-choice, not prioritised: 95% low']:.3f}, "
        f"{g['bottom-choice, not prioritised: 95% high']:.3f}]; prioritised {bh[(True, True)]}/{bh[(True, True)] + bh[(True, False)]}; "
        f"top-choice {g['top-choice marker rate']:.3f}; {g['bottom-choice markers prioritised there']} of {cand:,} (run, skill) "
        f"candidates prioritised in {g['runs showing a priority list']} runs",
    )


def held_out_findings(g: dict[str, Any]) -> list[common.Finding]:
    """rule 3 on the forms, criteria fixed before the numbers: the adopted form predicts unseen runs better than a
    constant (z > 1.96); the every-slot form predicts them about as well as the adopted one (|z| <= 1.96); the power law
    and strict competition do not predict them better than the adopted form (z <= 1.96, worded by what the data give);
    a per-scenario factor on the K form predicts them better than without (z > 1.96) or not, worded by what the data give"""
    v: Callable[[str], tuple[float, float, float]] = lambda lab: (
        g[f"held out: {lab}: nats"],
        g[f"held out: {lab}: SE"],
        g[f"held out: {lab}: z"],
    )
    ev: Callable[[str], str] = lambda lab: (
        f"{g['held out: runs']:,} runs, 5 folds: {v(lab)[0]:+.2f} nats, SE {v(lab)[1]:.2f}, z {v(lab)[2]:+.2f} ({lab})"
    )
    out = [
        F(
            "The slope in K does not predict unseen runs better than a constant rate: the form a (1 - b)^(K - 4) against a "
            "constant has z <= 1.96",
            v("fitted form against constant")[2] <= 1.96,
            ev("fitted form against constant"),
        ),
        F(
            "The every-slot form does not predict unseen runs better than the adopted form, nor worse (|z| <= 1.96)",
            abs(v("every slot competing against fitted form")[2]) <= 1.96,
            ev("every slot competing against fitted form"),
        ),
    ]
    out.append(
        F(
            "The power law does not predict unseen runs better than the K form (z <= 1.96)",
            v("power law against fitted form")[2] <= 1.96,
            ev("power law against fitted form"),
        )
    )
    out.append(
        F(
            "Strict competition predicts unseen runs worse than the K form (z < -1.96)",
            v("strict competition against fitted form")[2] < -1.96,
            ev("strict competition against fitted form"),
        )
    )
    out.append(
        F(
            "A per-scenario rate (the K form with a factor per scenario) does not predict unseen runs better than the K "
            "form alone (z <= 1.96)",
            v("scenario next to K")[2] <= 1.96,
            ev("scenario next to K"),
        )
    )
    return out


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


class OddsChange(NamedTuple):
    """odds_change(): relative changes of each run's product of per-target odds"""

    change: list[float]  # over the eight targets, runs with a nonzero product
    touched: list[float]  # ... those of them with a credited row
    lifted: int  # runs whose product crediting lifts from 0
    positive: list[float]  # over the targets with a positive chance without crediting, runs with a credited row


def odds_change(runrows: Sequence[RunRows], rate: Callable[[CalRow], float]) -> OddsChange:
    """relative change of each run's product of per-target odds when each row is credited at rate(row) (0: not): over
    the eight targets (runs with a nonzero product; those of them with a credited row; runs whose product crediting
    lifts from 0), and over the targets the model gives a positive chance without crediting (runs with a credited
    row among them)"""
    ch: list[float] = []
    tch: list[float] = []
    lifted = 0
    pos: list[float] = []
    for pr in runrows:
        new = [credit(x, rate(x))[0] for x in pr["rows"]]
        p0 = math.prod(x["p"] for x in pr["rows"])
        p1 = math.prod(new)
        touched = [rate(x) > 0 and free(x) for x in pr["rows"]]
        if any(t and x["p"] > 0 for t, x in zip(touched, pr["rows"])):
            pos.append(
                math.prod(q for q, x in zip(new, pr["rows"]) if x["p"] > 0)
                / math.prod(x["p"] for x in pr["rows"] if x["p"] > 0)
                - 1
            )
        if p0 <= 0:
            lifted += p1 > 0
            continue
        ch.append(p1 / p0 - 1)
        if any(touched):
            tch.append(p1 / p0 - 1)
    return OddsChange(ch, tch, lifted, pos)


def summary(xs: Sequence[float]) -> tuple[float, ...]:
    """(mean, median, max), NaN for none"""
    return (float(np.mean(xs)), float(np.median(xs)), float(np.max(xs))) if xs else (float("nan"),) * 3


def odds_values(sec: str, odds: OddsChange) -> list[common.Value | common.Finding]:
    """the odds changes' runs, mean, median and max, as Values of section `sec`"""
    out: list[common.Value | common.Finding] = []
    for lab, xs in (
        ("", odds.change),
        (", runs with a credited row", odds.touched),
        (", targets with a positive chance, runs with a credited row", odds.positive),
    ):
        m, md, mx = summary(xs)
        out += [
            common.Value(sec, f"odds change{lab}: runs", len(xs)),
            common.Value(sec, f"odds change{lab}: mean", m),
            common.Value(sec, f"odds change{lab}: median", md),
            common.Value(sec, f"odds change{lab}: max", mx),
        ]
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
    """a calibration's rows, observed rate, and obs / pred left out and credited, as Values of section `sec`"""
    V: Callable[[str, Any], common.Value] = lambda nm, v: common.Value(sec, nm, v)
    return [
        V(f"{pre}: rows", t.n),
        V(f"{pre}: observed", t.obs),
        V(f"{pre}: obs / pred, left out", t.left_out.ratio),
        V(f"{pre}: ratio 95% low, left out", t.left_out.rlo),
        V(f"{pre}: ratio 95% high, left out", t.left_out.rhi),
        V(f"{pre}: obs / pred, credited", t.credited.ratio),
        V(f"{pre}: ratio 95% low, credited", t.credited.rlo),
        V(f"{pre}: ratio 95% high, credited", t.credited.rhi),
        V(f"{pre}: LL credited minus left out, positive rows", t.ll_gain()),
    ]


def random_rows(R: list[Run], ab: tuple[float, float]) -> tuple[list[RandomRow], collections.Counter[str]]:
    """the calibration rows a random event could hint, as RandomRow dictionaries, and counts of skipped rows; kind
    'deck' uses the fitted rate at the run's K, 'own' has its rate filled in later"""
    a, b = ab
    nshared, marker, cardsk, sc = markers()
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
                    hit=ev in srcs,
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
    rr, skipped = random_rows(R, ab)
    _, runrows = calib()
    dk = [x for x in rr if x["kind"] == "deck"]
    own = [x for x in rr if x["kind"] == "own"]
    ko = sum(x["hit"] for x in own)
    r_own = common.div(ko, len(own))
    _, lo_o, hi_o, _ = common.cluster_ci([x["hit"] for x in own], [1] * len(own), [x["row"]["run"] for x in own])
    for x in own:
        x["rate"] = r_own
    rate = {id(x["row"]): x["rate"] for x in rr}
    t = calib_test([x["row"] for x in dk], lambda row: rate[id(row)])
    zero = [x for x in dk if credit(x["row"], 0.0)[1] <= 1e-12]
    th = calib_test([x["row"] for x in dk], held_out_rate(R, dk))
    c1, c0, nho = careers_held_out(dk, R, rows)
    ho = common.compare_held_out(c1, c0)
    odds = odds_change(runrows, lambda row: rate.get(id(row), 0.0))
    out: list[common.Value | common.Finding] = [
        V("careers: rows a deck card's random event could hint", len(dk)),
        V("careers: runs with such a row", len({x["row"]["run"] for x in dk})),
        V("careers: acquired through that event", sum(x["hit"] for x in dk)),
        V("careers: acquired through that event only", sum(x["only"] for x in dk)),
        V("careers: ambiguous credits there", sum(x["amb"] for x in dk)),
        V("careers: mean fitted rate there", float(np.mean([x["rate"] for x in dk])) if dk else float("nan")),
        V("careers: rows the model gives no chance", len(zero)),
        V("careers: of them acquired", sum(x["row"]["o_any"] for x in zero)),
        V("careers: of them acquired through that event", sum(x["hit"] for x in zero)),
        V("careers: rows of the trainee's own event, identifiable", len(own)),
        V("careers: trainee's own, rows not identifiable", skipped["trainee's own, not identifiable"]),
        V("careers: trainee's own, acquired through her events", ko),
        V("careers: trainee's own, rate", r_own),
        V("careers: trainee's own, rate 95% low", lo_o),
        V("careers: trainee's own, rate 95% high", hi_o),
    ]
    out += odds_values(SEC, odds)
    out += [
        V(
            "odds change: runs with a credited row and a zero product",
            len({x["row"]["run"] for x in dk}) - len(odds.touched),
        ),
        V("odds change: runs lifted from zero", odds.lifted),
    ]
    pre = "calibration"
    out += [
        V(f"{pre}: rows", t.n),
        V(f"{pre}: acquired", t.k),
        V(f"{pre}: observed", t.obs),
        V(f"{pre}: predicted, left out", t.left_out.pred),
        V(f"{pre}: obs / pred, left out", t.left_out.ratio),
        V(f"{pre}: ratio 95% low, left out", t.left_out.rlo),
        V(f"{pre}: ratio 95% high, left out", t.left_out.rhi),
        V(f"{pre}: predicted, credited", t.credited.pred),
        V(f"{pre}: obs / pred, credited", t.credited.ratio),
        V(f"{pre}: ratio 95% low, credited", t.credited.rlo),
        V(f"{pre}: ratio 95% high, credited", t.credited.rhi),
        V(f"{pre}: rows with a positive prediction", t.n_pos),
        V(f"{pre}: LL credited minus left out, positive rows", t.ll_gain()),
        V(f"{pre}, rate held out: obs / pred, credited", th.credited.ratio),
        V(f"{pre}, rate held out: ratio 95% low, credited", th.credited.rlo),
        V(f"{pre}, rate held out: ratio 95% high, credited", th.credited.rhi),
        V(f"{pre}, rate held out: LL credited minus left out, positive rows", th.ll_gain()),
        V("held out: credited against left out: runs", len(c1)),
        V("held out: credited against left out: rows", nho),
        V("held out: credited against left out: nats", ho.nats),
        V("held out: credited against left out: SE", ho.se),
        V("held out: credited against left out: z", ho.z),
    ]
    return out + career_findings(t, th, ho, len(c1), nho, zero)


def held_out_rate(R: list[Run], dk: Sequence[RandomRow]) -> Callable[[CalRow], float]:
    """each deck row's rate from the form refitted without the readings of these rows' cards in these runs"""
    used = {(x["row"]["run"], x["who"]) for x in dk}
    name = dict(common.mdb().execute("select [index], text from text_data where category = 6"))
    ha, hb = fits([r for r in readings(R) if (r["run"], name.get(r["char"])) not in used])["exp"][0]
    nshared, _, _, sc = markers()
    byid = {r["id"]: r for r in R}
    hrate = {id(x["row"]): ha * (1 - hb) ** (offer(byid[x["row"]["run"]], nshared, sc) - 4) for x in dk}
    return lambda row: hrate[id(row)]


def career_findings(
    t: CalibTest, th: CalibTest, ho: common.HeldOutComparison, runs: int, nho: int, zero: list[RandomRow]
) -> list[common.Value | common.Finding]:
    """the careers' conclusions: t credited at the fitted rate, th at the rate fitted without these rows"""
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
            lambda: (
                f"credited {t.ratio_text(t.credited)}; with the rate fitted without these rows {th.ratio_text(th.credited)}"
            ),
        ),
        F(
            "Crediting deck cards' random events at the fitted rate raises the log-likelihood of the rows the model gives a "
            "positive chance without them by more than 1.92 nats, with the rate fitted on every reading and with it fitted "
            "without these rows' readings (a check on the size of the gain; the claim that crediting predicts better is "
            "the 5-fold test)",
            lambda: t.ll_gain() > 1.92 and th.ll_gain() > 1.92,
            lambda: (
                f"{t.n_pos} rows: LL {t.left_out.ll:.1f} left out, {t.credited.ll:.1f} credited ({t.ll_gain():+.1f} nats); "
                f"rate held out {th.ll_gain():+.1f} nats"
            ),
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
    a = fits(readings(R))["exp"][0]
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
    rows = readings(R)
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
        f"constant {f['const'][0]:.3f} LL {f['const'][1]:.1f}; {a:.3f} (1 - {b:.3f})^(K-4) LL {L:.1f}; "
        f"strict S {f['strict'][0]:.2f} LL {f['strict'][1]:.1f}; power a {f['power'][0][1]:.2f} LL {f['power'][1]:.1f}"
    )
    careers_detail(R)


if __name__ == "__main__":
    common.main(claims)
    if common.have_runs():
        print()
        detail()
