"""Section 8 of docs/RESEARCH.md: chain events.

Reproduces the adopted binomial's moments and the fits on the run records (research/data/runs.jsonl, not included; Independent Training runs from uma-it).

Markers. A card's chain progress is read from "marker" skills of its link events (GameTora card events through
gametora.py, chain links from master.mdb single_mode_story_data.show_progress_1) credited to "<character> event" in a
run's hint list. A link is observable only if every choice and every outcome of its event carries a skill appearing
in no other event of the card. An observation is left out, seen or not, when another deck card's events could give a
marker skill, or when a marker skill was also hinted by inheritance (an "Inspiration" source: its level can hide the
event's hint, and inheritance does not depend on the chain). A marker shown though its link did not fire (the skill from
the scenario's or the trainee's events, which the records credit to the card) enters the likelihood as the marker's
background (marker_background). Decks with an unknown card or a non-pal R card are left out.

Data. Every analysis runs on every run record, selected only by pal count where the analysis is by pal count. Today's
model, Binomial(21, 0.35) with the card picked by links left, is the baseline the final model is tested against:
  - the fit of N (0-5 pals): a distribution-free fit (EM over P(N = 0..17), P(N >= 18)) against Binomial(21, 0.35),
    Poisson(7.1), Conway-Maxwell-Poisson and the binomial's profile over n (p re-fitted at each n), each by likelihood
    ratio (the thresholds used are in fit_findings); the same fit per scenario (scenarios with 30+ runs) against one N;
  - the pal-count budget (logit p = a + b x pals at n = CHAIN_N, pals excluding group cards): its fit, a scenario
    intercept and the own terms of Sasami Anshinzawa [This Might Sting!]'s decks and Inari One's careers on top of it;
  - held out (5-fold): the free distribution, the binomial by likelihood, the budget and the budget with the slot rule
    against today's model and against each other (held_out); the budget against today's model with every fitted
    nuisance measured on the training folds only (nested_held_out);
  - the stop share and no-list scale profiled on grids, the budget re-fitted at each value (profile_claims);
  - freed vs wasted chain ends (ends_claims): each rule's stop share and no-list scale from one coordinate search
    (end_search), checked against the joint grid; the in-sample difference by what can end a run's chains (end_group);
    held out with the constants searched on the training folds, and the mechanism alone with the budget fitted where no
    chain can end; a mixture's freed share; the direct deck-mate contrasts (decks whose other chain cards are plain
    3-link cards) against each rule's prediction; a re-fitting parametric bootstrap, BOOT_SIMS pools per rule, each
    scored by the whole procedure, for every one of these statistics; and whether prioritising an ending skill ends the
    chain (ending_choice_reading);
  - the marker allowance: the share of links implied fired (a later link shows on a clean marker) whose marker is
    missing, and the best p at n = 21 under allowances 0.002-0.02 and with none on the runs without a contradiction;
  - final links by pal count and by card (run-clustered Wald chi2 over cards with 60+ observations) against the final
    model and today's; the slot rule against the card rule at N fixed at Binomial(21, 0.35);
  - known misses: early links (1 and 2) per pal count with the run as a cluster, chains never started with five or six
    chain cards.
The final model: N ~ Binomial(CHAIN_N, p) per run, logit p = a + b x pals; each event advances one running chain card
by a link, every running card equally likely (RULE "cards"; "slots" weights by links left, "wasted" keeps an ended card
taking picks until its full length); each default-choice stop risk (card_events.json) ends its chain on STOP_SHARE of
its firings. A separate chain-ending choice (card_events.json 'end') ends it at its link with the chance IT takes that
choice on the run's priority list (steer_chance: the choice rule's three cases, reported in section 9); a run showing
no list takes NOLIST_SCALE times the mean of that chance over the list-showing runs with the card (chosen_end_share).
The three cases are read here (steer_readings: every card event whose choices the records show, plus Anshinzawa's
finale from section 9's hint-level reading) and reported in section 9.
Fits of N are exact over the chain states, with a chance EPS that a fired link's marker does not show (without it a run
whose later link shows while an earlier one does not has likelihood 0; allowance_claims measures the rate); the
allocation, final-link and early-link comparisons score each observation on its own (P(card reaches link) on the run's
deck). Intervals on rates over observations that come several to a run (final links, the allowance, chains never
started) are run-clustered (common.cluster_ci), the spread between cards a run-clustered Wald test
(common.cluster_chi2)."""

import collections, functools, itertools, math, os, re, sys
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from typing import Any, NamedTuple
import numpy as np
from scipy.optimize import brentq, minimize, minimize_scalar
from scipy.stats import binom, chi2

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
from common import BG_CLEAN, chosen_end, chosen_end_share, end_chance, game, marker_background, maxed_elsewhere
from common import steer_case
from common import Run


class Kind(NamedTuple):
    """a chain card"""

    links: int  # its chain's links
    last_link: int  # the link its chain ends at when no stop risk ends it earlier
    stop_risk: tuple[tuple[float, ...], ...]  # (link, k, n) each: the chain ends at that link with chance k / n


class Obs(NamedTuple):
    """an observation: whether a chain card's link was seen, and the chance its marker shows when the link did not fire
    (marker_background: the marker skill from a source the run's records cannot tell apart)"""

    card: int  # index in the run's chain cards
    link: int
    seen: bool
    background: float = 0.0


class ChainObservations(NamedTuple):
    """observations(): a run's chain cards and what its hints show of them"""

    kinds: list[Kind]
    chain: list[int]  # the chain cards' ids
    obs: list[Obs]


class Row(NamedTuple):
    """a run in a data set (dataset())"""

    run: Run
    kinds: list[Kind]
    chain: list[int]
    obs: list[Obs]


class Paths(NamedTuple):
    """paths(): every (state, events taken, absorbed) the chain process passes through, as parallel arrays by entry"""

    states: np.ndarray  # links fired per card (entries x cards)
    events: np.ndarray  # events taken to reach the state (its level)
    absorbed: np.ndarray  # nothing can advance from the state
    prob: np.ndarray  # the probability of passing through it, summed over stop-risk outcomes


Pmf = Sequence[float] | np.ndarray

SEC: str = "8"
EPS: float = (
    0.002  # chance a fired link's marker hint does not show (the fits' allowance; allowance_claims measures it)
)
# Global web_cards.json has at most three links per chain card, and a deck has six cards.
# Every path absorbs by sum(kind.last_link) <= 18; run_levels enforces that bound.
# Cutting here keeps all distinguishable counts, without duplicate post-absorption columns.
KMAX: int = 6 * 3  # distribution-free fit: P(N = 0..17) and P(N >= 18)
ALL: set[int] = {0, 1, 2, 3, 4, 5}  # pal counts of decks with a chain card
CHI1: float = 1.920729  # chi2(1) 95% / 2
ANSHINZAWA: int = 30080  # [This Might Sting!] Sasami Anshinzawa, a pal card: decks with her reach more chain events
INARI: int = 1034  # Inari One: her careers reach fewer
MISC: dict[Any, Any] = {}  # numbers the findings and the detail use that the document does not state
RULE: str = "cards"  # the model's allocation: each chain event goes to a running chain card, all equally likely
STOP_SHARE: float = 1 / 3  # every default-choice stop risk ends the chain on this share of its firings (profile_claims)
NOLIST_SCALE: float = 0.25  # a run showing no priority list: this times the listed runs' end chance (profile_claims)


# ---------------------------------------------------------------- the model
def binom_pmf(n: int, p: float, upto: int = 61) -> list[float]:
    return [math.comb(n, e) * p**e * (1 - p) ** (n - e) if e <= n else 0.0 for e in range(upto)]


def pois_pmf(lam: float, upto: int = 61) -> list[float]:
    return [math.exp(-lam) * lam**e / math.factorial(e) for e in range(upto)]


def chain_model() -> tuple[int, float]:
    """the solver's event count n (CHAIN_N, pkg/loopopt/src/deck/chains.rs) and today's p: the model before the
    pal-count budget, N ~ Binomial(21, 0.35) for every deck, the baseline the budget is tested against"""
    src = open(f"{common.ROOT}/pkg/loopopt/src/deck/chains.rs", encoding="utf-8").read()
    return int(re.search(r"CHAIN_N: u32 = (\d+);", src).group(1)), 0.35


CN, CP = chain_model()
BIN: list[float] = binom_pmf(CN, CP)
POIS: list[float] = pois_pmf(7.1)


def canon(kinds: Sequence[Kind]) -> tuple[tuple[Kind, ...], dict[int, int]]:
    """the deck's chain kinds in a canonical order, and each card's position in it"""
    order = sorted(range(len(kinds)), key=lambda i: kinds[i])
    return tuple(kinds[i] for i in order), {ci: j for j, ci in enumerate(order)}


_paths: dict[tuple[tuple[Kind, ...], str], Paths] = {}


def stop_outcomes(k: Kind) -> list[tuple[int, float]]:
    """the links a card's chain can end at, with their chances: each stop risk in link order, else its last link"""
    v, go = [], 1.0
    for link, k_, n in sorted(k.stop_risk):
        v.append((link, go * k_ / n))
        go *= 1 - k_ / n
    v.append((k.last_link, go))
    return v


def paths(ck: tuple[Kind, ...], rule: str = "slots") -> Paths:
    """Every (state, events taken, absorbed) the chain process passes through, with its probability, summed over
    stop-risk outcomes, as arrays (S: links fired per card, L, A, P). A state at level l is final with P(N = l), or with
    P(N >= l) when nothing can advance (absorbed). rule: 'slots' (weight = links left), 'cards' (uniform over the
    running cards) or 'wasted' (uniform over the cards short of their full length: an ended card keeps taking picks and
    shows none, the alternative to an end freeing its events)."""
    key = (ck, rule)
    if key in _paths:
        return _paths[key]
    n = len(ck)
    acc: collections.defaultdict[tuple[tuple[int, ...], int, bool], float] = collections.defaultdict(float)
    for combo in itertools.product(*[stop_outcomes(k) for k in ck]):
        w = math.prod(p for _, p in combo)
        stops = [s for s, _ in combo]
        if w == 0:
            continue
        layer: dict[tuple[int, ...], float] = {tuple([0] * n): 1.0}
        lvl = 0
        while layer:
            nxt: collections.defaultdict[tuple[int, ...], float] = collections.defaultdict(float)
            for s, p in layer.items():
                if rule == "wasted":
                    ws = [1 if s[i] < ck[i][0] else 0 for i in range(n)]
                else:
                    left = [(ck[i][0] - s[i]) if s[i] < stops[i] else 0 for i in range(n)]
                    ws = left if rule == "slots" else [1 if x else 0 for x in left]
                tot = sum(ws)
                seen = tuple(min(s[i], stops[i]) for i in range(n)) if rule == "wasted" else s
                acc[(seen, lvl, tot == 0)] += w * p
                for i in range(n):
                    if ws[i]:
                        t = list(s)
                        t[i] += 1
                        nxt[tuple(t)] += p * ws[i] / tot
            layer = nxt
            lvl += 1
    ks = list(acc)
    out = Paths(
        np.array([k[0] for k in ks], dtype=int).reshape(len(ks), n),
        np.array([k[1] for k in ks]),
        np.array([k[2] for k in ks]),
        np.array([acc[k] for k in ks]),
    )
    _paths[key] = out
    return out


def final_w(ck: tuple[Kind, ...], pmf: Pmf, rule: str = "slots") -> tuple[np.ndarray, np.ndarray]:
    """the path states and each state's probability of being final under the event-count pmf"""
    S, L, A, P = paths(ck, rule)
    pm = np.asarray(pmf, float)
    tail = survival(pm)
    ok = len(pm) > L
    Lc = np.minimum(L, len(pm) - 1)
    return S, np.where(ok, P * np.where(A, tail[Lc], pm[Lc]), 0.0)


def survival(pmf: Pmf) -> np.ndarray:
    """P(N >= l), at each pmf index l; clip negative roundoff after the cumulative sum reaches one."""
    return np.maximum(0.0, np.concatenate([[1.0], 1.0 - np.cumsum(pmf)[:-1]]))


_reach: dict[tuple[Any, ...], dict[tuple[int, int], float]] = {}


def reach(kinds: Sequence[Kind], pmf: Pmf = BIN, rule: str = "slots") -> dict[tuple[int, int], float]:
    """{(card index, link): P(the card reaches the link)} for a deck's chain kinds"""
    ck, pos = canon(tuple(kinds))
    key = (ck, rule, "B" if pmf is BIN else "P" if pmf is POIS else tuple(pmf))
    if key not in _reach:
        S, w = final_w(ck, pmf, rule)
        _reach[key] = {(j, k): float(w[S[:, j] >= k].sum()) for j in range(len(ck)) for k in range(1, ck[j][0] + 1)}
    r = _reach[key]
    return {(ci, k): r[(j, k)] for ci, j in pos.items() for k in range(1, kinds[ci][0] + 1)}


def run_levels(
    kinds: Sequence[Kind], obs: Iterable[Obs], eps: float = EPS, rule: str = "slots"
) -> tuple[np.ndarray, np.ndarray]:
    """the run's observations weighted over the chain process, by events taken: (A, B) arrays over levels 0..KMAX,
    A for states that end there with N = l, B for absorbed states (N >= l)"""
    if sum(k.last_link for k in kinds) > KMAX:
        raise ValueError(f"chain process exceeds the absorption bound {KMAX}")
    ck, pos = canon(tuple(kinds))
    S, L, A, P = paths(ck, rule)
    w = P.copy()
    for ci, lk, seen, bg in obs:
        fired = S[:, pos[ci]] >= lk
        # fired: the marker shows unless both the event's hint (eps) and the background miss; unfired: background only
        w = w * np.where(fired, 1 - eps * (1 - bg) if seen else eps * (1 - bg), bg if seen else 1 - bg)
    a = np.bincount(L[~A], w[~A], minlength=KMAX + 1)
    b = np.bincount(L[A], w[A], minlength=KMAX + 1)
    return a, b


def lump(v: Pmf) -> list[float]:
    """a pmf as the distribution-free fit's KMAX + 1 cells: P(N = 0..KMAX-1), P(N >= KMAX)"""
    v = list(v[:KMAX])
    v += [0.0] * (KMAX - len(v))
    return [*v, max(0.0, 1 - sum(v))]


def lk_matrix(xs: Iterable[tuple[list[Kind], list[Obs]]], eps: float = EPS, rule: str = RULE) -> np.ndarray:
    """per run, P(observations | N = k), k < KMAX, and N >= KMAX. Every path is absorbed by KMAX,
    so the last column is constant for all counts in the tail and needs no within-tail distribution."""
    out: list[list[float]] = []
    for kinds, obs in xs:
        a, b = run_levels(kinds, obs, eps, rule)
        row = [a[k] + b[: k + 1].sum() for k in range(KMAX)] + [b.sum()]
        out.append(row)
    return np.array(out)


def em(M: np.ndarray, iters: int = 500) -> np.ndarray:
    """the distribution-free fit: P(N = 0..KMAX-1), P(N >= KMAX) by EM over the runs' likelihood rows M"""
    pi = np.full(KMAX + 1, 1 / (KMAX + 1))
    for _ in range(iters):
        w = pi * M
        pi = (w / w.sum(1, keepdims=True)).mean(0)
    return pi


def ll(pi: Pmf, M: np.ndarray) -> float:
    return float(np.log(np.maximum(1e-300, M @ np.asarray(pi))).sum())


def mean_var(v: np.ndarray) -> tuple[float, float]:
    """Moments of the indexed cells: for a lumped/free pmf these are of min(N, KMAX), not N.
    The tail's internal moments are unidentifiable. Both capped moments bound their uncapped counterparts
    from below, but their ratio does not determine whether N is under-dispersed."""
    k = np.arange(len(v))
    m = float((k * v).sum())
    return m, float((k * k * v).sum()) - m * m


def obs_ll(xs: Iterable[Row], pmf: Pmf = BIN, rule: str = "slots") -> float:
    """the per-observation log-likelihood of the runs' observations: each link scored on its own by P(the card reaches
    the link) on the run's deck"""
    t = 0.0
    for x in xs:
        mb = reach(x.kinds, pmf, rule)
        for ci, lk, seen, bg in x.obs:
            pr = min(max(seen_chance(mb[(ci, lk)], bg), 1e-9), 1 - 1e-9)
            t += math.log(pr if seen else 1 - pr)
    return t


def kind(cid: int) -> Kind | None:
    """(links, links, stop risk) for a chain card, None for a pal, group or chainless card; a default-choice stop risk
    (card_events.json) ends the chain on STOP_SHARE of its firings, one share for every such event"""
    g = game()
    c = g.cards[cid]
    if c["pal"] or c["links"] == 0:
        return None
    risk = tuple(
        sorted(
            (r[0], STOP_SHARE, 1)
            for r in g.card_events.get(str(cid), {}).get("stop_risk", [])
            if r[0] < c["links"] and r[0] <= 4
        )
    )
    return Kind(c["links"], c["links"], risk)


def kind_in_run(cid: int, r: Run) -> Kind | None:
    """the card's chain kind in this run: kind(cid), plus its chosen end as a stop at that link with end_chance (a run
    showing no list: NOLIST_SCALE times chosen_end_share)"""
    k = kind(cid)
    link_end = chosen_end(cid) if k else None
    if not k or not link_end:
        return k
    q = round(NOLIST_SCALE * chosen_end_share(cid), 6) if r["priority"] is None else round(end_chance(cid, r), 6)
    if q == 0:
        return k
    return Kind(k.links, k.last_link, tuple(sorted((*k.stop_risk, (link_end[0], q, 1)))))


def pals(r: Run) -> int:
    cards = game().cards
    return sum(1 for c, _, _ in r["deck"] if c in cards and cards[c]["pal"])


def seen_chance(reached: float, bg: float, eps: float = EPS) -> float:
    """the chance a link's marker shows given the chance the link fired: the likelihood's observation model"""
    return reached * (1 - eps * (1 - bg)) + (1 - reached) * bg


def observations(r: Run) -> ChainObservations | None:
    """a run's chain observations: (chain kinds, chain card ids, [(card index, link, seen, background)]), or None for a
    deck with an unknown or non-pal R card. A marker another deck card's events could also give, or one whose skill was
    also inherited, is unreadable and left out, seen or not."""
    g = game()
    cards = g.cards
    deck = [c for c, _, _ in r["deck"]]
    if any(c not in cards for c in deck):
        return None
    if any(cards[c]["rarity"] == 1 and not cards[c]["pal"] for c in deck):
        return None
    hints = {h["name"]: h["src"] for h in r["hints"]}
    name = {c: common.bare(n) for c, _, n in r["deck"]}
    chain = [c for c in deck if kind(c)]
    obs: list[Obs] = []
    for ci, cid in enumerate(chain):
        nm = name[cid]
        for lk, mk in g.markers.get(cid, {}).items():
            if lk > cards[cid]["links"] or any(y != cid and mk & g.pool[y] for y in deck):
                continue
            seen = any(
                nm in [y.strip() for y in x[:-6].split(" or ")]
                for s in mk
                for x in hints.get(s, ())
                if x.endswith(" event")
            )
            if any("Inspiration" in hints.get(s, ()) for s in mk):
                continue  # inherited: left out seen or not (inheritance is independent of the chain)
            obs.append(Obs(ci, lk, seen, marker_background(mk, r)))
    return ChainObservations([kind_in_run(c, r) for c in chain], chain, obs)


# ---------------------------------------------------------------- data sets
_obs: dict[int, ChainObservations | None] = {}


def dataset(groups: set[int]) -> list[Row]:
    """[(run, kinds, chain card ids, observations)] for every run record with a pal count in `groups` (decks with an
    unknown or non-pal R card left out)"""
    out: list[Row] = []
    for r in common.runs():
        if pals(r) not in groups:
            continue
        if r["id"] not in _obs:
            _obs[r["id"]] = observations(r)
        if _obs[r["id"]] is not None:
            out.append(Row(r, *_obs[r["id"]]))
    return out


def dropped(groups: set[int]) -> int:
    """records with a pal count in `groups` left out for an unknown or non-pal R card"""
    dataset(groups)
    return sum(1 for r in common.runs() if pals(r) in groups and _obs[r["id"]] is None)


def cmp_pmf(lam: float, nu: float, upto: int = 61) -> list[float]:
    """the Conway-Maxwell-Poisson pmf, P(N = k) proportional to lam^k / (k!)^nu, k < upto"""
    lw = np.array([k * math.log(lam) - nu * math.lgamma(k + 1) for k in range(upto)])
    w = np.exp(lw - lw.max())
    return list(w / w.sum())


def cmp_ll(M: np.ndarray) -> tuple[float, float, float]:
    """the Conway-Maxwell-Poisson fit (P(N = k) proportional to lam^k / (k!)^nu: another two-parameter count family that
    can be under-dispersed), by Nelder-Mead from three starts: (loglik, lam, nu)"""
    f: Callable[[np.ndarray], float] = lambda th: -ll(lump(cmp_pmf(math.exp(th[0]), math.exp(th[1]))), M)
    r = min((minimize(f, x0, method="Nelder-Mead") for x0 in ([2, 0], [3, 0.5], [4, 0.7])), key=lambda r: r.fun)
    return -r.fun, math.exp(r.x[0]), math.exp(r.x[1])


def best_binomial_p(M: np.ndarray, n: int) -> tuple[float, float]:
    """the best p at a given n (bounded scalar search), exact over the chain states: (p, loglik)"""
    r = minimize_scalar(
        lambda p: -ll(lump(binom_pmf(n, p)), M), bounds=(0.05, 0.95), method="bounded", options={"xatol": 1e-5}
    )
    return float(r.x), -float(r.fun)


def binom_profile(M: np.ndarray, ns: Iterable[int] = range(8, 41)) -> list[tuple[int, float, float]]:
    """the profile likelihood of the binomial's n: at each n the best p (bounded scalar search), exact over the chain
    states through the run matrix M. Returns [(n, p, loglik)]; the data fix the mean n p well and n only within an
    interval, since n and p trade off along a ridge"""
    return [(n, *best_binomial_p(M, n)) for n in ns]


def binomial_moments(mean: float, variance: float) -> tuple[int, float]:
    """Match the free fit's capped mean and closest capped variance over n = 10..40 (the held-out MLE grid).
    Matching uncapped binomial moments to these cells would silently put the whole tail at KMAX."""
    candidates = []
    for n in range(10, 41):
        if min(n, KMAX) < mean:
            continue
        p = brentq(lambda p, n=n: mean_var(np.asarray(lump(binom_pmf(n, p))))[0] - mean, 0.0, 1.0)
        _, fitted_variance = mean_var(np.asarray(lump(binom_pmf(n, p))))
        candidates.append((abs(fitted_variance - variance), n, p))
    _, n, p = min(candidates)
    return n, p


def budget_pals(r: Run) -> int:
    """the deck's pal cards that raise the chain-event rate: pal-flagged cards other than group cards (section 8)"""
    cards = game().cards
    return sum(1 for c, _, _ in r["deck"] if c in cards and cards[c]["pal"] and cards[c]["kind"] != "group")


def budget_rows(pal_counts: np.ndarray, a: float, b: float) -> np.ndarray:
    """per run, the pal-count budget's lumped pmf: Binomial(CHAIN_N, expit(a + b * pals)) over the free fit's cells"""
    p = np.clip(1 / (1 + np.exp(-(a + b * pal_counts))), 1e-9, 1 - 1e-9)
    body = binom.pmf(np.arange(KMAX)[None, :], CN, p[:, None])
    return np.hstack([body, np.maximum(0.0, 1 - body.sum(1, keepdims=True))])


def budget_fit(
    M: np.ndarray, pal_counts: np.ndarray, extra: np.ndarray | None = None, start: Sequence[float] | None = None
) -> tuple[np.ndarray, float]:
    """the pal-count budget fitted by likelihood: (a, b, c...) and the log-likelihood; `extra` (one indicator, or one
    column per indicator) adds a coefficient c each to the logit (a group's own effect on top of the pal count); `start`
    is the search's starting (a, b), by default one p = CP"""
    cols = [np.ones(len(M)), pal_counts] + ([extra] if extra is not None else [])
    X = np.column_stack(cols)

    def nll(beta: np.ndarray) -> float:
        p = np.clip(1 / (1 + np.exp(-(X @ beta))), 1e-9, 1 - 1e-9)
        body = binom.pmf(np.arange(KMAX)[None, :], CN, p[:, None])
        rows = np.hstack([body, np.maximum(0.0, 1 - body.sum(1, keepdims=True))])
        return -float(np.log(np.maximum(1e-300, (M * rows).sum(1))).sum())

    x0 = np.zeros(X.shape[1])
    x0[0] = math.log(CP / (1 - CP))
    if start is not None:
        x0[:2] = start
    r = minimize(nll, x0, method="Nelder-Mead", options={"xatol": 1e-5, "fatol": 1e-4, "maxiter": 4000})
    return r.x, -float(r.fun)


def held_out(xs: list[Row], M: np.ndarray, M_today: np.ndarray) -> dict[str, common.HeldOutComparison]:
    """5-fold cross-validation (common.fold by run id): each fold's runs are scored by forms fitted on the other four
    under the model's allocation (M, RULE): the free distribution, the binomial by likelihood (n 10..40, p re-fitted)
    and the pal-count budget; the budget also under the slot rule (M_today); against today's model, Binomial(21, 0.35) with the slot rule (N fixed: no budget fitted; the stop risks, end chances and
    backgrounds are the same fitted ones as every form's), and pairwise. Per-run held-out
    log-likelihoods; {comparison: (nats, SE, z)} from common.compare_held_out, positive when the first form predicts
    unseen runs better"""
    folds = np.array([common.fold(x.run["id"]) for x in xs])
    P = np.array([budget_pals(x.run) for x in xs], float)
    per = {
        f: np.zeros(len(xs))
        for f in (
            "free",
            "binomial by likelihood",
            "pal-count budget",
            "pal-count budget, slot rule",
            "base",
        )
    }
    for k in range(common.K_FOLDS):
        train, test = folds != k, folds == k
        T, E = M[train], M[test]
        bn, bp, _ = max(binom_profile(T, range(10, 41)), key=lambda x: x[2])
        pmfs = {"free": list(em(T)), "binomial by likelihood": lump(binom_pmf(bn, bp))}
        for f, pmf in pmfs.items():
            per[f][test] = np.log(np.maximum(1e-300, E @ np.asarray(pmf)))
        per["base"][test] = np.log(np.maximum(1e-300, M_today[test] @ np.asarray(lump(BIN))))
        (a, b), _ = budget_fit(T, P[train])
        per["pal-count budget"][test] = np.log(np.maximum(1e-300, (E * budget_rows(P[test], a, b)).sum(1)))
        (a, b), _ = budget_fit(M_today[train], P[train])
        per["pal-count budget, slot rule"][test] = np.log(
            np.maximum(1e-300, (M_today[test] * budget_rows(P[test], a, b)).sum(1))
        )
    out = {f: common.compare_held_out(per[f], per["base"]) for f in per if f != "base"}
    out["pal-count budget over free"] = common.compare_held_out(per["pal-count budget"], per["free"])
    out["pal-count budget over the same with the slot rule"] = common.compare_held_out(
        per["pal-count budget"], per["pal-count budget, slot rule"]
    )
    return out


def nested_held_out() -> dict[str, common.HeldOutComparison]:
    """5-fold held out with every fitted nuisance (marker backgrounds, steering shares, no-list end shares) measured on
    the training folds only (common.nuisance_runs), so nothing about a scored run enters its prediction except its own
    deck, list and trainee: the pal-count budget under RULE, fitted on the training runs, against today's model
    (Binomial(21, 0.35), slot rule). The stop share and no-list scale stay at the model's values (profiled on the whole
    pool; profile_claims)"""
    folds = {r["id"]: common.fold(r["id"]) for r in common.runs() or []}
    per: dict[str, dict[int, float]] = {f: {} for f in ("budget", "today")}
    for k in range(common.K_FOLDS):
        train_runs = [r for r in common.runs() or [] if folds[r["id"]] != k]
        with common.nuisance_runs(train_runs):
            _obs.clear()
            xs = dataset(ALL)
            train = np.array([folds[x.run["id"]] != k for x in xs])
            P = np.array([budget_pals(x.run) for x in xs], float)
            pairs = [(x.kinds, x.obs) for x in xs]
            M = lk_matrix(pairs, EPS, RULE)
            (a, b), _ = budget_fit(M[train], P[train])
            ll_test = np.log(np.maximum(1e-300, (M[~train] * budget_rows(P[~train], a, b)).sum(1)))
            per["budget"].update(zip((x.run["id"] for x, t in zip(xs, train) if not t), ll_test))
            M0 = lk_matrix(pairs, EPS, "slots")
            ll0 = np.log(np.maximum(1e-300, M0[~train] @ np.asarray(lump(BIN))))
            per["today"].update(zip((x.run["id"] for x, t in zip(xs, train) if not t), ll0))
    _obs.clear()
    ids = sorted(set(per["budget"]) & set(per["today"]))
    arr = {f: np.array([v[i] for i in ids]) for f, v in per.items()}
    return {
        "pal-count budget over today's model": common.compare_held_out(arr["budget"], arr["today"]),
    }


# ---------------------------------------------------------------- values
def V(name: str, got: float | int | str | None, **kw: str) -> common.Value:
    return common.Value(SEC, name, got, **kw)


def derived() -> list[common.Value | common.Finding]:
    """the numbers that need no run records: the binomial's moments and the free fit's parameter count"""
    out: list[common.Value | common.Finding] = []
    m, v = CN * CP, CN * CP * (1 - CP)
    out += [V("Binomial(21, 0.35): mean", m), V("Binomial(21, 0.35): variance", v, note=f"{v:.4f}")]
    out.append(
        V(
            "distribution-free fit: parameters more than the binomial",
            KMAX - 2,
            note=f"P(N = 0..{KMAX - 1}), P(N >= {KMAX}): {KMAX} free",
        )
    )
    out.append(V("fixed binomial: nominal LR df", KMAX))
    return out


def fit_claims() -> list[common.Value | common.Finding]:
    """the fit of N (every run, 0-5 pals) and its per-scenario split"""
    xs = dataset(ALL)
    note = f"every run, 0-5 pals; {dropped(ALL)} decks with a non-pal R card left out"
    M = lk_matrix([(x.kinds, x.obs) for x in xs])  # the model's allocation (RULE)
    M_today = lk_matrix([(x.kinds, x.obs) for x in xs], EPS, "slots")  # today's: links left
    pooled = em(M)
    llp = ll(pooled, M)
    mn, vr = mean_var(pooled)
    l21 = ll(lump(BIN), M)
    lp = ll(lump(POIS), M)
    prof = binom_profile(M)
    bn, bp, lb = max(prof, key=lambda x: x[2])
    ho = held_out(xs, M, M_today)
    nested = nested_held_out()
    l_cmp = cmp_ll(M)[0]
    out: list[common.Value | common.Finding] = [
        V("fit of N: runs", len(xs), note=note),
        V("distribution-free: mean", mn, note=f"E[min(N, {KMAX})]; uncapped moments are not identified"),
        V("distribution-free: variance", vr, note=f"Var(min(N, {KMAX}))"),
        V("Binomial(21, 0.35): nats behind distribution-free", llp - l21, note=f"LL {l21:.1f} vs {llp:.1f}"),
        V("Poisson(7.1): nats behind distribution-free", llp - lp),
        V("best binomial n free: n", bn),
        V("best binomial n free: p", bp, note="p re-fitted at each n"),
        V("best binomial n free: nats behind distribution-free", llp - lb),
        V("best binomial n free: LR p", float(chi2.sf(2 * (llp - lb), KMAX - 2))),
        V("binomial n: 95% profile interval low", min(n_ for n_, _, l_ in prof if lb - l_ <= CHI1)),
        V("binomial n: 95% profile interval high", max(n_ for n_, _, l_ in prof if lb - l_ <= CHI1)),
        V("binomial n = 21, p re-fitted: nats below the best n", lb - {n_: l_ for n_, _, l_ in prof}[21]),
        V("distribution-free: variance / mean", vr / mn, note=f"for min(N, {KMAX}), not N"),
        *[
            V(f"held out: {form}: {w}" if " over " in form else f"held out: {form} over today's model: {w}", x)
            for form, t_ in ho.items()
            for w, x in zip(("nats", "SE", "z"), t_)
        ],
        *[
            V(f"nested held out: {form}: {w}", x)
            for form, t_ in nested.items()
            for w, x in zip(("nats", "SE", "z"), t_)
        ],
        *budget_claims(xs, M),
        V(
            "pal-count budget: in-sample nats over today's model",
            budget_fit(M, np.array([budget_pals(x.run) for x in xs], float))[1] - ll(lump(BIN), M_today),
            note="Binomial(21, 0.35) with the slot rule",
        ),
        V("Conway-Maxwell-Poisson: LR p", float(chi2.sf(2 * (llp - l_cmp), KMAX - 2))),
        V("best binomial n free over Conway-Maxwell-Poisson: nats", lb - l_cmp, note="two parameters each"),
        *property_claims(xs, M),
    ]
    by: collections.defaultdict[int, list[int]] = collections.defaultdict(list)  # scenario -> row indices
    for i, x in enumerate(xs):
        by[x.run["scenario"]].append(i)
    sep = {sc: em(M[ix]) for sc, ix in by.items() if len(ix) >= 30}
    lls = sum(ll(sep[sc], M[by[sc]]) for sc in sep) + sum(ll(pooled, M[ix]) for sc, ix in by.items() if sc not in sep)
    out.append(V("per scenario: log-likelihood gain", lls - llp))
    out.append(V("per scenario: extra parameters", (len(sep) - 1) * KMAX, note=f"{len(sep)} scenarios with 30+ runs"))
    P = np.array([budget_pals(x.run) for x in xs], float)
    own = sorted(sep)[1:]  # one intercept per scenario with 30+ runs past the first
    l_sc = budget_fit(M, P, np.column_stack([[x.run["scenario"] == sc for x in xs] for sc in own]).astype(float))[1]
    out.append(V("per scenario on the pal-count budget: log-likelihood gain", l_sc - budget_fit(M, P)[1]))
    out.append(V("per scenario on the pal-count budget: extra parameters", len(own)))
    MISC["fit"] = {
        "llp": llp,
        "lls": {
            nm: ll(lump(v), M)
            for nm, v in (
                ("Binomial(21, 0.35)", BIN),
                ("Poisson(7.1)", POIS),
            )
        },
        "runs": len(xs),
        "by_pals": dict(sorted(collections.Counter(pals(x.run) for x in xs).items())),
    }
    return out


def budget_claims(xs: list[Row], M: np.ndarray) -> list[common.Value | common.Finding]:
    """the pal-count budget on every run: its coefficients, p and mean N per pal count, and its log-likelihood against
    Binomial(21, 0.35)"""
    P = np.array([budget_pals(x.run) for x in xs], float)
    (a, b), l_b = budget_fit(M, P)
    MISC["budget"] = (a, b)
    out: list[common.Value | common.Finding] = [
        V("pal-count budget: a", a),
        V("pal-count budget: b", b),
        V("pal-count budget: log-likelihood over Binomial(21, 0.35)", l_b - ll(lump(BIN), M)),
        V("pal-count budget: LR p against one p at n = 21", float(chi2.sf(2 * (l_b - best_binomial_p(M, CN)[1]), 1))),
    ]
    for k in range(6):
        pk = 1 / (1 + math.exp(-(a + b * k)))
        out += [V(f"pal-count budget: p at {k} pals", pk), V(f"pal-count budget: mean N at {k} pals", CN * pk)]
    return out


def property_claims(xs: list[Row], M: np.ndarray) -> list[common.Value | common.Finding]:
    """N on the decks with Sasami Anshinzawa [This Might Sting!] and on the Inari One careers against the other runs: each
    side's free-fit mean, and the gain of p at n = CHAIN_N re-fitted on each side (one parameter); then
    Binomial(CHAIN_N, 0.35) against a free fit on the runs with neither"""
    out: list[common.Value | common.Finding] = []
    _, l_one = best_binomial_p(M, CN)
    sides = {
        "Sasami Anshinzawa's card": np.array([any(c[0] == ANSHINZAWA for c in x.run["deck"]) for x in xs]),
        "Inari One": np.array([x.run["chara"] == INARI for x in xs]),
    }
    P = np.array([budget_pals(x.run) for x in xs], float)
    _, l_0 = budget_fit(M, P)
    for nm, g in sides.items():
        (p_in, l_in), (p_out, l_out) = best_binomial_p(M[g], CN), best_binomial_p(M[~g], CN)
        out += [
            V(f"{nm}: runs", int(g.sum())),
            V(f"{nm}: mean of N", mean_var(em(M[g]))[0], note=f"E[min(N, {KMAX})]"),
            V(f"{nm}: mean of N, the other runs", mean_var(em(M[~g]))[0], note=f"E[min(N, {KMAX})]"),
            V(f"{nm}: p at n = {CN}", p_in),
            V(f"{nm}: p at n = {CN}, the other runs", p_out),
            V(f"{nm}: p at n = {CN} on each side, LR p", float(chi2.sf(2 * (l_in + l_out - l_one), 1))),
        ]
        (_, _, c), l_c = budget_fit(M, P, g.astype(float))
        out += [
            V(f"{nm}: own term on the pal-count budget", c),
            V(f"{nm}: own term on the pal-count budget, LR p", float(chi2.sf(2 * max(0.0, l_c - l_0), 1))),
            V(f"{nm}: mean pal count", float(P[g].mean())),
            V(f"{nm}: mean pal count, the other runs", float(P[~g].mean())),
        ]
    g = ~sides["Sasami Anshinzawa's card"] & ~sides["Inari One"]
    gap = ll(em(M[g]), M[g]) - ll(lump(BIN), M[g])
    out.append(
        V(
            "Binomial(21, 0.35) LR p, runs with neither",
            float(chi2.sf(2 * gap, KMAX)),
            note=f"{KMAX} nominal df; {g.sum()} runs",
        )
    )
    return out


STOP_GRID: tuple[float, ...] = (0.2, 0.25, 0.3, 1 / 3, 0.4, 0.5)
NOLIST_GRID: tuple[float, ...] = (0.0, 0.1, 0.2, 0.25, 0.3, 0.4, 0.6, 1.0)


def profile_claims() -> list[common.Value | common.Finding]:
    """STOP_SHARE and NOLIST_SCALE profiled one at a time with the other at its value: the pal-count budget re-fitted at
    each grid value under RULE, and its log-likelihood below the grid's best (both enter every chain card's kind, so the
    observations are rebuilt at each value)"""
    global STOP_SHARE, NOLIST_SCALE
    chosen = (STOP_SHARE, NOLIST_SCALE)
    out: list[common.Value | common.Finding] = [V("stop share", STOP_SHARE), V("no-list scale", NOLIST_SCALE)]
    for nm, grid in (("stop share", STOP_GRID), ("no-list scale", NOLIST_GRID)):
        lls = {}
        for v in grid:
            STOP_SHARE, NOLIST_SCALE = (v, chosen[1]) if nm == "stop share" else (chosen[0], v)
            _obs.clear()
            xs = dataset(ALL)
            P = np.array([budget_pals(x.run) for x in xs], float)
            lls[v] = budget_fit(lk_matrix([(x.kinds, x.obs) for x in xs]), P)[1]
        best = max(lls.values())
        mine = chosen[0] if nm == "stop share" else chosen[1]
        out += [V(f"{nm} profile: {v:.3g}, nats below the grid's best", best - l_) for v, l_ in lls.items()]
        out.append(V(f"{nm} profile: the model's value, nats below the grid's best", best - lls[mine]))
        near = [v for v, l_ in lls.items() if best - l_ <= CHI1]
        out += [
            V(f"{nm} profile: lowest grid value within 1.92 nats", min(near)),
            V(f"{nm} profile: highest grid value within 1.92 nats", max(near)),
        ]
    STOP_SHARE, NOLIST_SCALE = chosen
    _obs.clear()
    return out


# ---------------------------------------------------------------- does an ended chain free its events?
# Freed (RULE "cards", the model): a chain that ends early leaves the running cards, so later events go to the others.
# Wasted ("wasted"): the ended card keeps taking its share of picks until its full length and shows none.
MODEL_ENDS: tuple[float, float] = (STOP_SHARE, NOLIST_SCALE)  # the model's values, where every search starts
END_RULES: tuple[str, ...] = ("cards", "wasted")
END_NAMES: dict[str, str] = {"cards": "freed", "wasted": "wasted"}
END_GROUPS: tuple[str, ...] = (
    "cannot end early",
    "default-choice stop only",
    "chosen end, list shown",
    "chosen end, no list shown",
)
BOOT_SIMS: int = 20  # synthetic pools per hypothesis (ends_bootstrap)
BOOT_JOBS: int = 6  # worker processes for the pools
BOOT_SEED: int = 20261024
MIX_GRID: tuple[float, ...] = tuple(i / 20 for i in range(21))  # end_mixture's freed shares
BG_SCALES: tuple[float, ...] = (0.5, 1.5)  # every marker background scaled by these, the comparison re-run


def rows_at(stop: float, nolist: float) -> list[Row]:
    """every run's chain observations with the stop share and no-list scale set to these values"""
    global STOP_SHARE, NOLIST_SCALE
    STOP_SHARE, NOLIST_SCALE = stop, nolist
    _obs.clear()
    return dataset(ALL)


_kinds_at: dict[tuple[float, float], list[list[Kind]]] = {}


def kinds_at(stop: float, nolist: float) -> list[list[Kind]]:
    """every run's chain kinds at this stop share and no-list scale, in dataset order (the observations themselves do
    not depend on them); the model's values are restored after"""
    if (stop, nolist) not in _kinds_at:
        _kinds_at[(stop, nolist)] = [x.kinds for x in rows_at(stop, nolist)]
        rows_at(*MODEL_ENDS)
    return _kinds_at[(stop, nolist)]


def run_ll(M: np.ndarray, P: np.ndarray, ab: Sequence[float]) -> np.ndarray:
    """per-run log-likelihoods of likelihood rows M under the pal-count budget at (a, b)"""
    return np.log(np.maximum(1e-300, (M * budget_rows(P, ab[0], ab[1])).sum(1)))


class EndFit(NamedTuple):
    """one rule's fitted chain-end constants and budget"""

    stop: float
    nolist: float
    ab: tuple[float, float]
    ll: float


FitAt = Callable[[float, float], tuple[tuple[float, float], float]]


def end_search(fit_at: FitAt) -> EndFit:
    """the coordinate search over STOP_GRID and NOLIST_GRID from the model's values: the best stop share at the current
    no-list scale, then the best no-list scale at that stop share, repeated until neither moves (at most four passes);
    fit_at gives ((a, b), log-likelihood) at a point"""
    stop, nolist = MODEL_ENDS
    for _ in range(4):
        s2 = max(STOP_GRID, key=lambda v: fit_at(v, nolist)[1])
        n2 = max(NOLIST_GRID, key=lambda v: fit_at(s2, v)[1])
        if (s2, n2) == (stop, nolist):
            break
        stop, nolist = s2, n2
    ab, l_ = fit_at(stop, nolist)
    return EndFit(stop, nolist, (float(ab[0]), float(ab[1])), l_)


MatrixAt = Callable[[float, float], np.ndarray]


def budget_search(M_at: MatrixAt, P: np.ndarray, sel: np.ndarray | slice = slice(None)) -> EndFit:
    """end_search with the budget re-fitted on the rows `sel` at every point (each fit starting from the last)"""
    cache: dict[tuple[float, float], tuple[tuple[float, float], float]] = {}
    last: list[Sequence[float]] = [(-1.1, 0.17)]

    def fit_at(st: float, nl: float) -> tuple[tuple[float, float], float]:
        if (st, nl) not in cache:
            ab, l_ = budget_fit(M_at(st, nl)[sel], P[sel], start=last[0])
            last[0] = ab
            cache[(st, nl)] = ((float(ab[0]), float(ab[1])), l_)
        return cache[(st, nl)]

    return end_search(fit_at)


def fixed_search(M_at: MatrixAt, rows: np.ndarray, ab: Sequence[float], sel: np.ndarray) -> EndFit:
    """end_search with the budget held at ab (its per-run rows `rows`), scored on the rows `sel`"""

    def fit_at(st: float, nl: float) -> tuple[tuple[float, float], float]:
        l_ = float(np.log(np.maximum(1e-300, (M_at(st, nl)[sel] * rows[sel]).sum(1))).sum())
        return (float(ab[0]), float(ab[1])), l_

    return end_search(fit_at)


def pool_matrix(
    cache: dict[tuple[str, float, float], np.ndarray], obs: list[list[Obs]], rule: str, st: float, nl: float
) -> np.ndarray:
    """the likelihood rows of these observations under `rule` at (st, nl), cached by (rule, st, nl)"""
    if (rule, st, nl) not in cache:
        cache[(rule, st, nl)] = lk_matrix(list(zip(kinds_at(st, nl), obs)), EPS, rule)
    return cache[(rule, st, nl)]


class EndStudy(NamedTuple):
    """end_procedure on one pool of observations"""

    fits: dict[str, EndFit]
    per_run: dict[str, np.ndarray]  # each rule's per-run log-likelihood at its own fit
    matrices: dict[tuple[str, float, float], np.ndarray]


def end_procedure(obs: list[list[Obs]], P: np.ndarray, matrices: dict | None = None) -> EndStudy:
    """the whole comparison on one pool of observations, the same for the real runs and every synthetic pool: for each
    rule the coordinate search (end_search), the budget re-fitted at every point it visits; matrices caches the
    likelihood rows by (rule, stop, nolist)"""
    M_at = {} if matrices is None else matrices
    fits = {rule: budget_search(functools.partial(pool_matrix, M_at, obs, rule), P) for rule in END_RULES}
    per = {rule: run_ll(M_at[(rule, f.stop, f.nolist)], P, f.ab) for rule, f in fits.items()}
    return EndStudy(fits, per, M_at)


def end_group(x: Row) -> str:
    """what can end one of the run's chains before its last link (x at the model's values): a chosen end (a separate
    chain-ending choice with a chance above zero; list shown or not), else a default-choice stop risk, else nothing"""
    if any(kind(cid) != k for cid, k in zip(x.chain, x.kinds)):
        return END_GROUPS[2] if x.run["priority"] is not None else END_GROUPS[3]
    return END_GROUPS[1] if any(k.stop_risk for k in x.kinds) else END_GROUPS[0]


# the direct comparison: decks whose other chain cards are all plain 3-link cards, the one other card being
#   P  a card whose chain-ending choice the run's shown list prioritises (end chance 0.5 or more)
#   S  a card with a default-choice stop risk and no chain-ending choice
#   C  a 2-link SR with neither;   D  none (every chain card a plain 3-link card)
DECKMATES: dict[str, str] = {
    "P": "prioritised chosen end",
    "S": "default-choice stop",
    "C": "2-link SR",
}


def deckmate_group(x: Row) -> tuple[str, int | None] | None:
    """(group, the odd card's index) of a run for the direct comparison (x at the model's values), None outside it"""
    plain = [k.links == 3 and not k.stop_risk for k in x.kinds]
    if all(plain):
        return "D", None
    odd = [i for i, p in enumerate(plain) if not p]
    if len(odd) != 1:
        return None
    f, r = odd[0], x.run
    cid, k, ce = x.chain[odd[0]], kind(x.chain[odd[0]]), chosen_end(x.chain[odd[0]])
    if k is None:
        return None
    if ce and r["priority"] is not None and ce[1] & set(r["priority"]) and end_chance(cid, r) >= 0.5:
        return "P", f
    if not ce and k.stop_risk:
        return "S", f
    if not ce and not k.stop_risk and k.links == 2:
        return "C", f
    return None


DeckmateLink = tuple[float, float, int, float, float]  # (seen, background, run id, freed reach, wasted reach)


def deckmate_cells(
    base: list[Row], obs: list[list[Obs]], reach_by: dict[str, list[dict[tuple[int, int], float]]] | None = None
) -> dict[tuple[int, int, int, int, bool], dict[str, list[DeckmateLink]]]:
    """every deck-mate link of the direct comparison's runs, by cell (budget pals, chain cards, link, scenario, Inari
    One's career or not: her careers reach fewer events and are unevenly spread over the groups) and group; reach_by gives each rule's per-run reach (0 when absent: a synthetic pool needs only what was seen)"""
    cells: collections.defaultdict[tuple[int, int, int, int, bool], collections.defaultdict[str, list[DeckmateLink]]]
    cells = collections.defaultdict(lambda: collections.defaultdict(list))
    for i, (x, o) in enumerate(zip(base, obs)):
        gf = deckmate_group(x)
        if gf is None:
            continue
        grp, f = gf
        key = (budget_pals(x.run), len(x.kinds), x.run["scenario"], x.run["chara"] == INARI)
        for ci, lk, seen, bg in o:
            if ci == f:
                continue
            rf = reach_by["cards"][i][(ci, lk)] if reach_by else 0.0
            rw = reach_by["wasted"][i][(ci, lk)] if reach_by else 0.0
            cells[(key[0], key[1], lk, key[2], key[3])][grp].append((float(seen), bg, x.run["id"], rf, rw))
    return cells


class Contrast(NamedTuple):
    """deckmate_contrast: a group's background-corrected deck-mate rate minus plain decks', within cells"""

    measured: float
    se: float  # run-clustered
    freed: float  # the same contrast of the freed model's reach
    wasted: float
    links: int


def deckmate_contrast(cells: dict, g: str, base: str = "D") -> Contrast | None:
    """the within-cell difference in background-corrected deck-mate rates, sum(seen - bg) / sum(1 - bg), between group g
    and `base`, weighted by g's links per cell (cells with 15 or more links in each); its standard error from each
    run's influence summed over every cell it feeds"""
    use = [row for row in cells.values() if len(row[g]) >= 15 and len(row[base]) >= 15]
    if not use:
        return None
    W = sum(len(row[g]) for row in use)
    num = mf = mw = 0.0
    infl: collections.defaultdict[int, float] = collections.defaultdict(float)
    for row in use:
        w = len(row[g]) / W
        for grp, sign in ((g, 1.0), (base, -1.0)):
            t = row[grp]
            den = sum(1 - u[1] for u in t)
            est = sum(u[0] - u[1] for u in t) / den
            num += sign * w * est
            mf += sign * w * sum(u[3] * (1 - u[1]) for u in t) / den
            mw += sign * w * sum(u[4] * (1 - u[1]) for u in t) / den
            for u in t:
                infl[u[2]] += sign * w * ((u[0] - u[1]) - est * (1 - u[1])) / den
    exact = [0.0 if abs(v) < 1e-12 else v for v in (num, mf, mw)]  # roundoff of a contrast that is zero by construction
    return Contrast(exact[0], math.sqrt(sum(v * v for v in infl.values())), exact[1], exact[2], W)


def end_subsets(base: list[Row], groups: np.ndarray) -> dict[str, np.ndarray]:
    """the runs each freed - wasted statistic sums over: every run, each end group, and the no-list group without
    Marvelous Sunday [Dazzling Day in the Snow]'s decks (she holds most of it)"""
    marv = np.array([MARVELOUS in x.chain for x in base])
    out = {"freed - wasted": np.ones(len(base), bool)}
    out |= {f"freed - wasted, {g}": groups == g for g in END_GROUPS}
    out[f"freed - wasted, {END_GROUPS[3]}, without {END_CARDS[MARVELOUS]}"] = (groups == END_GROUPS[3]) & ~marv
    return out


def end_statistics(
    study: EndStudy, subsets: dict[str, np.ndarray], base: list[Row], obs: list[list[Obs]]
) -> dict[str, float]:
    """the statistics the bootstrap compares: freed - wasted log-likelihood over each subset of runs (end_subsets) with
    its per-run standard error (the runs' differences treated as independent), and the measured deck-mate contrasts"""
    d = study.per_run["cards"] - study.per_run["wasted"]
    out: dict[str, float] = {}
    for nm, m in subsets.items():
        out[nm] = float(d[m].sum())
        out[f"{nm}, per-run SE"] = float(math.sqrt(m.sum()) * d[m].std(ddof=1))
    cells = deckmate_cells(base, obs)
    for g, nm in DECKMATES.items():
        c = deckmate_contrast(cells, g)
        out[f"deck-mates, {nm} - plain"] = c.measured if c else float("nan")
    return out


_BOOT: dict[str, Any] = {}  # what the bootstrap's worker processes read (set before they fork)


def boot_pool(job: tuple[str, int]) -> dict[str, float]:
    """one synthetic pool: observations drawn under `truth` at its real-data fit, then the real runs' whole procedure"""
    truth, i = job
    rng = np.random.default_rng((BOOT_SEED, END_RULES.index(truth), i))
    obs = simulate(_BOOT["rows"][truth], truth, _BOOT["fits"][truth].ab, rng)
    study = end_procedure(obs, _BOOT["P"])
    return end_statistics(study, _BOOT["subsets"], _BOOT["base"], obs)


def ends_bootstrap(
    base: list[Row], P: np.ndarray, subsets: dict[str, np.ndarray], fits: dict[str, EndFit]
) -> dict[str, list]:
    """BOOT_SIMS synthetic pools under each rule, each generated at that rule's real-data constants and budget and
    scored by the real runs' whole procedure (both rules' constants re-searched, budgets re-fitted): {truth: [each
    pool's statistics]}"""
    import concurrent.futures, multiprocessing

    for st, nl in itertools.product(STOP_GRID, NOLIST_GRID):
        kinds_at(st, nl)  # computed once here, inherited by the workers
    _BOOT.update(
        rows={r: [x._replace(kinds=k) for x, k in zip(base, kinds_at(f.stop, f.nolist))] for r, f in fits.items()},
        fits=fits,
        P=P,
        subsets=subsets,
        base=base,
    )
    jobs = [(truth, i) for truth in END_RULES for i in range(BOOT_SIMS)]
    ctx = multiprocessing.get_context("fork")
    with concurrent.futures.ProcessPoolExecutor(BOOT_JOBS, mp_context=ctx) as ex:
        done = list(ex.map(boot_pool, jobs))
    return {truth: [s for (t, _), s in zip(jobs, done) if t == truth] for truth in END_RULES}


def ends_held_out(
    base: list[Row], P: np.ndarray, groups: np.ndarray, matrix: Callable[[str, float, float], np.ndarray]
) -> tuple[dict[str, common.HeldOutComparison], dict[str, list[tuple[float, float]]]]:
    """5-fold held out, each rule's constants searched on the training folds (end_search) with its budget re-fitted at
    every point, then the test fold scored at that fit: freed over wasted in total and by end group; and the mechanism
    alone: the budget fitted on the training runs where no chain can end (the two rules coincide there), held fixed,
    each rule's constants searched on the training runs where one can, the test runs where one can scored. Also each
    rule's constants per fold"""
    folds = np.array([common.fold(x.run["id"]) for x in base])
    can = groups != END_GROUPS[0]
    per = {r: np.zeros(len(base)) for r in END_RULES}
    fixed = {r: np.zeros(len(base)) for r in END_RULES}
    chosen: dict[str, list[tuple[float, float]]] = {k: [] for r in END_RULES for k in (r, f"{r}, mechanism alone")}
    for k in range(common.K_FOLDS):
        train, test = folds != k, folds == k
        for rule in END_RULES:
            f = budget_search(functools.partial(matrix, rule), P, train)
            chosen[rule].append((f.stop, f.nolist))
            per[rule][test] = run_ll(matrix(rule, f.stop, f.nolist)[test], P[test], f.ab)
        sel = train & ~can
        ab0, _ = budget_fit(matrix("cards", *MODEL_ENDS)[sel], P[sel])
        rows0 = budget_rows(P, ab0[0], ab0[1])
        for rule in END_RULES:
            f = fixed_search(functools.partial(matrix, rule), rows0, ab0, train & can)
            chosen[f"{rule}, mechanism alone"].append((f.stop, f.nolist))
            t = test & can
            fixed[rule][t] = np.log(np.maximum(1e-300, (matrix(rule, f.stop, f.nolist)[t] * rows0[t]).sum(1)))
    out = {"freed over wasted": common.compare_held_out(per["cards"], per["wasted"])}
    out |= {
        f"freed over wasted, {g}": common.compare_held_out(per["cards"][groups == g], per["wasted"][groups == g])
        for g in END_GROUPS
    }
    out["mechanism alone: freed over wasted"] = common.compare_held_out(fixed["cards"][can], fixed["wasted"][can])
    return out, chosen


REFINE_STOP: tuple[float, ...] = (-1 / 30, -1 / 60, 0.0, 1 / 60, 1 / 30)  # refine's steps around a rule's best
REFINE_NOLIST: tuple[float, ...] = (-0.05, -0.025, 0.0, 0.025, 0.05)


def refine(M_at: MatrixAt, P: np.ndarray, f: EndFit) -> EndFit:
    """a rule's best on a finer grid around its coordinate-search best (the grids' coarse steps could favour the rule
    whose best sits on a grid point), the budget re-fitted at every point"""
    best = f
    for ds, dn in itertools.product(REFINE_STOP, REFINE_NOLIST):
        st = f.stop if ds == 0 else round(f.stop + ds, 6)
        nl = f.nolist if dn == 0 else round(f.nolist + dn, 6)
        if not 0 < st < 1 or nl < 0:
            continue
        ab, l_ = budget_fit(M_at(st, nl), P, start=f.ab)
        if l_ > best.ll:
            best = EndFit(st, nl, (float(ab[0]), float(ab[1])), l_)
    return best


_link_levels: list[dict[int, dict[int, dict[str, set[int]]]]] = []


def link_levels() -> dict[int, dict[int, dict[str, set[int]]]]:
    """card -> chain link -> skill -> the fixed hint levels that link's outcomes give it (GameTora's outcomes, links from
    master.mdb single_mode_story_data.show_progress_1; random levels left out)"""
    if not _link_levels:
        gt, db = common.gametora(), common.mdb()
        sk = dict(db.execute("select [index], text from text_data where category = 47"))
        title = dict(db.execute("select [index], text from text_data where category = 181"))
        cchar = dict(db.execute("select id, chara_id from support_card_data"))
        link_of = dict(
            db.execute("select story_id, show_progress_1 from single_mode_story_data where show_progress_1 > 0")
        )
        out: collections.defaultdict[int, collections.defaultdict[int, collections.defaultdict[str, set[int]]]]
        out = collections.defaultdict(lambda: collections.defaultdict(lambda: collections.defaultdict(set)))
        for cid, es in common.card_outcome_events(gt, title, cchar).items():
            for _, sid, chs in es:
                if link_of.get(sid):
                    for ch in chs:
                        for o in common.outcome_skills(ch, gt, sk):
                            for skill, v in o.items():
                                if isinstance(v, int):
                                    out[cid][link_of[sid]][skill].add(v)
        _link_levels.append(out)
    return _link_levels[0]


def impossible_links(base: list[Row]) -> set[tuple[int, int]]:
    """the (card, link) pairs some run reads as seen through a marker credited to the card's event below every level
    that link's outcomes give the skill (a credit the link cannot have made)"""
    lv, out = link_levels(), set()
    for x in base:
        hs = {h["name"]: h for h in x.run["hints"]}
        name = {c: common.bare(n) for c, _, n in x.run["deck"]}
        for ci, lk, seen, _ in x.obs:
            cid = x.chain[ci]
            if not seen:
                continue
            for skill in game().markers[cid][lk]:
                h = hs.get(skill)
                levels = lv[cid][lk].get(skill)
                credited = h and any(
                    name[cid] in [y.strip() for y in src[:-6].split(" or ")]
                    for src in h["src"]
                    if src.endswith(" event")
                )
                if credited and levels and h["lv"] < min(levels):
                    out.add((cid, lk))
    return out


def end_mixture(MF: np.ndarray, MW: np.ndarray, P: np.ndarray) -> dict[float, float]:
    """the log-likelihood of a mixture, each run's chains freed with chance lam and wasted otherwise (each rule's rows at
    its own fitted constants), the budget re-fitted at each lam of MIX_GRID"""
    out: dict[float, float] = {}
    start = np.array([-1.1, 0.17])
    for lam in MIX_GRID:

        def nll(beta: np.ndarray, lam: float = lam) -> float:
            rows = budget_rows(P, beta[0], beta[1])
            return -float(np.log(np.maximum(1e-300, lam * (MF * rows).sum(1) + (1 - lam) * (MW * rows).sum(1))).sum())

        r = minimize(nll, start, method="Nelder-Mead", options={"xatol": 1e-5, "fatol": 1e-4, "maxiter": 4000})
        start = r.x
        out[lam] = -float(r.fun)
    return out


class Reading(NamedTuple):
    """ending_choice_reading: the links after a chain-ending choice in one group"""

    links: int
    runs: int
    seen: int
    background: float  # links expected seen from their markers' background alone (the chain ended at the choice)
    model: float  # links expected seen under the model (seen_chance of its reach)
    never: float  # links expected seen if the chain never ended at the choice
    continuing: float  # the share of chains that went on past the choice: (seen - background) / (never - background)
    continuing_se: float  # Poisson: sqrt(seen) / (never - background)
    continuing_bg: dict[float, float]  # the same share with every background scaled by READING_BG


READING_BG: tuple[float, ...] = (0.8, 1.2)  # background scales for the ending-choice reading's continuing share
MARVELOUS: int = 30074  # Marvelous Sunday [Dazzling Day in the Snow]
END_CARDS: dict[int, str] = {
    30074: "Marvelous Sunday [Dazzling Day in the Snow]",
    30017: "Smart Falcon [My Umadol Way! ☆]",
}


def ending_choice_reading(base: list[Row], ab: Sequence[float]) -> dict[tuple[str, str], tuple]:
    """Does prioritising the skill on a chain-ending choice end the chain? On runs showing a list, the links after each
    card's chain-ending choice when its ending skill is prioritised alone (the one-sided case), when it is prioritised
    but already at hint Lv 5 from elsewhere (the maxed case: at Lv 5 at the end of the run, from another source, with no
    credit from the card's event), and when no skill on that
    choice is prioritised: links seen against the counts their markers' background alone gives, the model expects (card
    rule, budget at ab) and the chain would give if it never ended at the choice, and from these the share of chains
    that went on; every card with an ending skill pooled, and END_CARDS alone"""
    rows: collections.defaultdict[tuple[str, str], list[tuple[bool, float, float, float, int]]]
    rows = collections.defaultdict(list)
    for x in base:
        r = x.run
        if r["priority"] is None:
            continue
        pmf = binom_pmf(CN, 1 / (1 + math.exp(-(ab[0] + ab[1] * budget_pals(r)))))
        hs = {h["name"]: h for h in r["hints"]}
        for i, cid in enumerate(x.chain):
            ce = chosen_end(cid)
            if not ce or not ce[1]:
                continue
            link, ending, continuing = ce
            name = next(common.bare(n) for c, _, n in r["deck"] if c == cid)
            case = steer_case(r["priority"], ending, continuing, maxed_elsewhere(hs, ending, f"{name} event"))
            grp = {"one-sided": "prioritised", None: "not prioritised", "maxed": "maxed"}.get(case)
            if grp is None:
                continue
            pr = reach(x.kinds, pmf, RULE)
            no_end = list(x.kinds)
            no_end[i] = kind(cid) or no_end[i]  # the card without its chain-ending choice
            pn = reach(no_end, pmf, RULE)
            for ci, lk, seen, bg in x.obs:
                if ci == i and lk > link:
                    for who in ("every card", END_CARDS.get(cid)):
                        if who:
                            rows[(who, grp)].append((seen, bg, pr[(ci, lk)], pn[(ci, lk)], r["id"]))
    return {k: reading_of(v) for k, v in rows.items()}


def reading_of(v: list[tuple[bool, float, float, float, int]]) -> Reading:
    """one group's Reading from its (seen, background, model reach, reach without the end, run id) links"""
    seen = sum(u[0] for u in v)

    def share(f: float) -> float:
        bg = sum(min(1.0, u[1] * f) for u in v)
        never = sum(seen_chance(u[3], min(1.0, u[1] * f)) for u in v)
        return (seen - bg) / (never - bg)

    bg, never = sum(u[1] for u in v), sum(seen_chance(u[3], u[1]) for u in v)
    return Reading(
        len(v),
        len({u[4] for u in v}),
        seen,
        bg,
        sum(seen_chance(u[2], u[1]) for u in v),
        never,
        share(1.0),
        math.sqrt(seen) / (never - bg),
        {f: share(f) for f in READING_BG},
    )


def ends_claims() -> list[common.Value | common.Finding]:
    """Freed or wasted. Each rule's stop share and no-list scale come from the same coordinate search (end_search), on
    the real runs and on every synthetic pool; the joint grid checks that the search finds the grid's best on the real
    runs. Then: the in-sample difference, in total and by end group; held out with the constants searched on the training
    folds (ends_held_out), and the mechanism alone; a mixture's freed share (end_mixture); the direct deck-mate
    contrasts with each rule's prediction; and the re-fitting parametric bootstrap (ends_bootstrap), where each
    statistic of the real runs falls among the pools generated under each rule. Plus the ending-choice reading
    (ending_choice_reading), and the in-sample comparison re-run with every marker background scaled (BG_SCALES)"""
    base = rows_at(*MODEL_ENDS)
    P = np.array([budget_pals(x.run) for x in base], float)
    groups = np.array([end_group(x) for x in base])
    obs = [x.obs for x in base]
    real_M: dict[tuple[str, float, float], np.ndarray] = {}

    def matrix(rule: str, st: float, nl: float) -> np.ndarray:
        if (rule, st, nl) not in real_M:
            real_M[(rule, st, nl)] = lk_matrix(list(zip(kinds_at(st, nl), obs)), EPS, rule)
        return real_M[(rule, st, nl)]

    for st, nl in itertools.product(STOP_GRID, NOLIST_GRID):
        for rule in END_RULES:
            matrix(rule, st, nl)
    cannot = groups == END_GROUPS[0]
    assert np.allclose(real_M[("cards", *MODEL_ENDS)][cannot], real_M[("wasted", *MODEL_ENDS)][cannot])
    study = end_procedure(obs, P, real_M)
    joint = {
        rule: max(
            itertools.product(STOP_GRID, NOLIST_GRID),
            key=lambda sn, rule=rule: budget_fit(matrix(rule, *sn), P, start=study.fits[rule].ab)[1],
        )
        for rule in END_RULES
    }
    subsets = end_subsets(base, groups)
    stats = end_statistics(study, subsets, base, obs)
    refined = {rule: refine(functools.partial(matrix, rule), P, study.fits[rule]) for rule in END_RULES}
    ho, chosen = ends_held_out(base, P, groups, matrix)
    mix = end_mixture(
        matrix("cards", study.fits["cards"].stop, study.fits["cards"].nolist),
        matrix("wasted", study.fits["wasted"].stop, study.fits["wasted"].nolist),
        P,
    )
    reach_by = {
        rule: [
            reach(k, binom_pmf(CN, 1 / (1 + math.exp(-(f.ab[0] + f.ab[1] * p)))), rule)
            for k, p in zip(kinds_at(f.stop, f.nolist), P)
        ]
        for rule, f in study.fits.items()
    }
    cells = deckmate_cells(base, obs, reach_by)
    reading = ending_choice_reading(base, study.fits["cards"].ab)
    scaled = {
        f: end_procedure([[o._replace(background=min(1.0, o.background * f)) for o in ob] for ob in obs], P)
        for f in BG_SCALES
    }
    bad = impossible_links(base)
    kept = [[o for o in ob if (x.chain[o.card], o.link) not in bad] for x, ob in zip(base, obs)]
    without_bad = end_procedure(kept, P)
    boot = ends_bootstrap(base, P, subsets, study.fits)
    rows_at(*MODEL_ENDS)
    out: list[common.Value | common.Finding] = [
        V("chain ends: synthetic pools per hypothesis", BOOT_SIMS),
        V(
            "chain ends: pools per hypothesis plus one",
            BOOT_SIMS + 1,
            note="the real runs beat every pool by luck 1 in this",
        ),
    ]
    for rule, f in study.fits.items():
        nm = END_NAMES[rule]
        out += [
            V(f"chain ends, {nm}: stop share", f.stop),
            V(f"chain ends, {nm}: no-list scale", f.nolist),
            V(f"chain ends, {nm}: the search finds the joint grid's best", float(joint[rule] == (f.stop, f.nolist))),
            V(f"chain ends, {nm}: fold stop shares", ", ".join(sorted({f"{s_:.3g}" for s_, _ in chosen[rule]}))),
            V(f"chain ends, {nm}: fold no-list scales", ", ".join(sorted({f"{n_:.3g}" for _, n_ in chosen[rule]}))),
            V(
                f"chain ends, {nm}: mechanism-alone fold constants",
                ", ".join(sorted({f"{s_:.3g} / {n_:.3g}" for s_, n_ in chosen[f"{rule}, mechanism alone"]})),
            ),
            V(f"chain ends, {nm}: refined stop share", refined[rule].stop),
            V(f"chain ends, {nm}: refined no-list scale", refined[rule].nolist),
        ]
    out += [
        V(
            "chain ends: freed - wasted, constants refined on the finer grid",
            refined["cards"].ll - refined["wasted"].ll,
        ),
        V(
            f"chain ends: no-list runs with {END_CARDS[MARVELOUS]}",
            int(((groups == END_GROUPS[3]) & np.array([MARVELOUS in x.chain for x in base])).sum()),
        ),
    ]
    for g in END_GROUPS:
        out.append(V(f"chain ends: runs, {g}", int((groups == g).sum())))
    for f, st_ in scaled.items():
        d = float(st_.per_run["cards"].sum() - st_.per_run["wasted"].sum())
        out.append(V(f"chain ends: freed - wasted, every background times {f:g}", d, note="each rule's search re-run"))
    fw = without_bad.fits["cards"]
    out += [
        V(
            "impossible credits: links left out",
            len(bad),
            note="every run; a link some run reads through such a credit",
        ),
        V("impossible credits: readings left out", sum(len(ob) - len(k) for ob, k in zip(obs, kept))),
        V(
            "impossible credits: freed - wasted without those links",
            float(without_bad.per_run["cards"].sum() - without_bad.per_run["wasted"].sum()),
        ),
        V(
            "impossible credits: budget a without those links",
            fw.ab[0],
            note=f"freed, at {fw.stop:.3f} / {fw.nolist:.2f}",
        ),
        V("impossible credits: budget b without those links", fw.ab[1]),
        V("impossible credits: freed stop share without those links", fw.stop),
        V("impossible credits: freed no-list scale without those links", fw.nolist),
    ]
    for nm, t_ in ho.items():
        out += [V(f"chain ends held out: {nm}: {w}", x) for w, x in zip(("nats", "SE", "z"), t_)]
    best_lam = max(mix, key=mix.get)
    near = [lam for lam, l_ in mix.items() if mix[best_lam] - l_ <= CHI1]
    out += [
        V("chain ends, mixture: freed share", best_lam, note="each run's chains freed with this chance, else wasted"),
        V("chain ends, mixture: lowest freed share within 1.92 nats", min(near)),
        V("chain ends, mixture: highest freed share within 1.92 nats", max(near)),
        V("chain ends, mixture: nats of all wasted below the best", mix[best_lam] - mix[0.0]),
        V("chain ends, mixture: nats of all freed below the best", mix[best_lam] - mix[1.0]),
    ]
    for g, nm in DECKMATES.items():
        c = deckmate_contrast(cells, g)
        if c is None:
            continue  # no cell with enough links in both groups
        out += [
            V(f"deck-mates, {nm} - plain: measured", c.measured),
            V(f"deck-mates, {nm} - plain: SE", c.se),
            V(f"deck-mates, {nm} - plain: freed predicts", c.freed),
            V(f"deck-mates, {nm} - plain: wasted predicts", c.wasted),
            V(f"deck-mates, {nm} - plain: links", c.links),
            V(f"deck-mates, {nm} - plain: z from freed", (c.measured - c.freed) / c.se),
            V(f"deck-mates, {nm} - plain: z from wasted", (c.measured - c.wasted) / c.se),
            V(f"deck-mates, {nm} - plain: z from no gain", c.measured / c.se),
        ]
    for stat, real in stats.items():
        out.append(V(f"chain ends: {stat}: real runs", real))
        for truth in END_RULES:
            v = np.array([s[stat] for s in boot[truth]])
            nm = END_NAMES[truth]
            out += [
                V(f"chain ends: {stat}: pools if {nm}: mean", float(v.mean()), note=f"{BOOT_SIMS} pools"),
                V(f"chain ends: {stat}: pools if {nm}: SD", float(v.std(ddof=1))),
                V(f"chain ends: {stat}: pools if {nm} below the real runs", int((v < real).sum())),
            ]
    for (who, grp), rd in sorted(reading.items()):
        out += [
            V(f"ending choice, {who}, {grp}: later links read", rd.links),
            V(f"ending choice, {who}, {grp}: runs", rd.runs),
            V(f"ending choice, {who}, {grp}: later links seen", rd.seen),
            V(f"ending choice, {who}, {grp}: expected from background alone", rd.background),
            V(f"ending choice, {who}, {grp}: expected by the model", rd.model),
            V(f"ending choice, {who}, {grp}: expected if the chain never ended", rd.never),
            V(f"ending choice, {who}, {grp}: share of chains going on", rd.continuing),
            V(f"ending choice, {who}, {grp}: share of chains going on, SE", rd.continuing_se),
            *[
                V(f"ending choice, {who}, {grp}: share of chains going on, every background times {f:g}", v)
                for f, v in rd.continuing_bg.items()
            ],
        ]
    return out


def simulate(xs: list[Row], rule: str, ab: tuple[float, float], rng: np.random.Generator) -> list[list[Obs]]:
    """each run's markers drawn under `rule`: a final chain state from its kinds and the budget at ab, then each
    observed link seen with 1 - EPS (1 - background) if fired, else with its background"""
    a, b = ab
    out = []
    for x in xs:
        pmf = binom_pmf(CN, 1 / (1 + math.exp(-(a + b * budget_pals(x.run)))))
        pth, surv = paths(tuple(x.kinds), rule), survival(pmf)
        level = np.minimum(pth.events, len(pmf) - 1)
        w = np.where(pth.absorbed, surv[level], np.asarray(pmf)[level]) * pth.prob
        st = pth.states[rng.choice(len(w), p=w / w.sum())]
        out.append(
            [
                Obs(ci, lk, bool(rng.random() < ((1 - EPS * (1 - bg)) if st[ci] >= lk else bg)), bg)
                for ci, lk, _, bg in x.obs
            ]
        )
    return out


def obs_ci(rows: list[tuple[float, float, int]]) -> tuple[float, float]:
    """the observed rate's 95% interval over (observed, predicted, run id) rows that come several to a run: the mean
    prediction times the run-clustered interval of observed / predicted (common.cluster_ci); Wilson when every run gives
    one row (5 pals: one chain card)"""
    o, e, r = zip(*rows)
    if len(set(r)) == len(r):
        return common.wilson(round(sum(o)), len(o))
    _, lo, hi, _ = common.cluster_ci(o, e, r)
    m = sum(e) / len(e)
    return lo * m, hi * m


class FinalLinks(NamedTuple):
    """final links reached at one pal count"""

    runs: int
    observed: float
    binomial: float  # predicted, Binomial(21, 0.35)
    links: int  # card-links observed
    low: float  # the observed rate's run-clustered 95% interval
    high: float
    budget: float = float("nan")  # predicted, the pal-count budget


class EarlyLinks(NamedTuple):
    """early links (1 and 2) reached at one pal count"""

    observed: float
    predicted: float
    links: int
    runs: int


class LinkTables(NamedTuple):
    """final_claims(): by pal count"""

    final: dict[int, FinalLinks]
    early: dict[int, EarlyLinks]


def budget_pmf(r: Run) -> Pmf:
    """the run's chain-event pmf under the pal-count budget fitted on every run (fit_claims sets MISC["budget"])"""
    a, b = MISC["budget"]
    return binom_pmf(CN, round(1 / (1 + math.exp(-(a + b * budget_pals(r)))), 6))


def final_claims() -> tuple[list[common.Value | common.Finding], LinkTables]:
    """the final-link table (each observed rate with its run-clustered 95% interval against Binomial(21, 0.35) and the
    pal-count budget) and the early-link shortfall under both (every run)"""
    out: list[common.Value | common.Finding] = []
    tab: dict[int, FinalLinks] = {}
    for npal in (5, 4, 3, 2, 1, 0):
        xs = dataset({npal})
        o = b = bb = 0.0
        n = 0
        rows: list[tuple[float, float, int]] = []
        hers = [0.0, 0.0, 0]  # decks with Sasami Anshinzawa's card: observed, predicted, card-links
        for r, kinds, _, obs in xs:
            mb = reach(kinds, BIN)
            mu = reach(kinds, budget_pmf(r), RULE)
            her = any(c[0] == ANSHINZAWA for c in r["deck"])
            for ci, lk, seen, bg in obs:
                if lk == kinds[ci][0]:
                    o += seen
                    b += seen_chance(mb[(ci, lk)], bg)
                    bb += seen_chance(mu[(ci, lk)], bg)
                    n += 1
                    rows.append((seen, seen_chance(mb[(ci, lk)], bg), r["id"]))
                    if her:
                        hers = [hers[0] + seen, hers[1] + seen_chance(mb[(ci, lk)], bg), hers[2] + 1]
        lo, hi = obs_ci(rows)
        tab[npal] = FinalLinks(len(xs), o / n, b / n, n, lo, hi, bb / n)
        out += [
            V(f"final-link table, {npal} pals: runs", len(xs)),
            V(f"final-link table, {npal} pals: observed", o / n, note=f"{n} card-links"),
            V(f"final-link table, {npal} pals: observed 95% low", lo),
            V(f"final-link table, {npal} pals: observed 95% high", hi),
            V(f"final-link table, {npal} pals: Binomial(21, 0.35)", b / n),
            V(f"final-link table, {npal} pals: pal-count budget", bb / n),
        ]
        if hers[2]:
            out += [
                V(f"final-link table, {npal} pals, Sasami Anshinzawa's card: card-links", hers[2]),
                V(f"final-link table, {npal} pals, Sasami Anshinzawa's card: observed", hers[0] / hers[2]),
                V(f"final-link table, {npal} pals, Sasami Anshinzawa's card: Binomial(21, 0.35)", hers[1] / hers[2]),
            ]
    # early links (1 and 2), per run as a cluster
    early: dict[int, EarlyLinks] = {}
    for npal in (4, 3, 2, 1, 0):
        per: list[tuple[float, float, int]] = []  # per run: (observed, predicted, links)
        per_b: list[tuple[float, float, int]] = []  # the same under the pal-count budget
        for r, kinds, _, obs in dataset({npal}):
            mb = reach(kinds, BIN)
            mu = reach(kinds, budget_pmf(r), RULE)
            o = e = eb = 0.0
            k = 0
            for ci, lk, seen, bg in obs:
                if lk == kinds[ci][0]:
                    continue
                o += seen
                e += seen_chance(mb[(ci, lk)], bg)
                eb += seen_chance(mu[(ci, lk)], bg)
                k += 1
            if k:
                per.append((o, e, k))
                per_b.append((o, eb, k))
        n = sum(x[2] for x in per)
        O = sum(x[0] for x in per)
        E = sum(x[1] for x in per)
        sd = math.sqrt(sum((x[0] - x[1]) ** 2 for x in per))
        early[npal] = EarlyLinks(O / n, E / n, n, len(per))
        out += [
            V(
                f"early-link shortfall, {npal} pals",
                (E - O) / n,
                note=f"{n} links in {len(per)} runs; obs {O / n:.2f} pred {E / n:.2f}",
            ),
            V(f"early-link shortfall, {npal} pals: +-", sd / n),
            V(f"early-link shortfall, {npal} pals: z", (E - O) / sd),
        ]
        Eb = sum(x[1] for x in per_b)
        sdb = math.sqrt(sum((x[0] - x[1]) ** 2 for x in per_b))
        out += [
            V(f"early-link shortfall under the budget, {npal} pals", (Eb - O) / n),
            V(f"early-link shortfall under the budget, {npal} pals: z", (Eb - O) / sdb),
        ]
    outside = lambda pred: sum(not t.low <= pred(t) <= t.high for t in tab.values())
    out += [
        V("final-link table: pal counts with today's model outside the interval", outside(lambda t: t.binomial)),
        V("final-link table: pal counts with the final model outside the interval", outside(lambda t: t.budget)),
    ]
    return out, LinkTables(tab, early)


def allowance_claims() -> list[common.Value | common.Finding]:
    """the marker allowance: links fire in order, so a card's later link seen with a clean marker (background under
    BG_CLEAN) means its earlier links fired; the share of those earlier links whose marker is missing measures how
    often a fired link goes unseen (every run; its 95% interval run-clustered; with none missing, the exact upper limit
    counting each run as one trial, since the clustered interval collapses at zero)"""
    imp = un = 0
    rows: list[tuple[float, float, int]] = []
    for x in dataset(ALL):
        # card -> link -> (seen, background); a later link proves the earlier fired only where its marker is clean
        per: collections.defaultdict[int, dict[int, tuple[bool, float]]] = collections.defaultdict(dict)
        for ci, lk, seen, bg in x.obs:
            per[ci][lk] = (seen, bg)
        for d in per.values():
            for lk, (seen, _) in d.items():
                if any(l2 > lk and s2 and b2 < BG_CLEAN for l2, (s2, b2) in d.items()):
                    imp += 1
                    un += not seen
                    rows.append((int(not seen), 1, x.run["id"]))
    if un:
        lo, hi = obs_ci(rows)  # one run gives several such links: the run-clustered interval of the rate
    else:
        lo, hi = 0.0, 1 - 0.025 ** (1 / len({rr for _, _, rr in rows}))
    return [
        V("allowance: the fits' value", EPS),
        V("allowance: earlier links implied fired", imp, note="every run"),
        V("allowance: marker not shown", un),
        V("allowance: share", un / imp),
        V("allowance: 95% low", lo),
        V("allowance: 95% high", hi),
    ]


@dataclass
class FinalByCard:
    """final_by_card()"""

    obs: float  # pooled observed rate
    model: float  # pooled predicted rate
    n: int  # card-links
    cards: int  # cards with 60+ observations
    chi2: float  # the spread between them, run-clustered Wald
    p: float
    low: list[tuple[int, float]]  # the three lowest (card, z)
    high: list[tuple[int, float]]  # the three highest
    rate: dict[int, tuple[float, float]]  # card -> (observed, predicted)
    name: Callable[[int], str]  # card id -> name


def final_by_card(budget: bool = False) -> FinalByCard:
    """final-link reach by card (every run) against the chain model on each run's deck (Binomial(21, 0.35), or the
    pal-count budget with budget=True): the pooled rate and the spread
    between cards with 60+ observations (a joint Wald test that every card's observed - predicted is zero, one df per
    card, its covariance run-clustered by common.cluster_chi2: the chain cards of one run share its N). Each card's z
    (binomial variance; one observation per run) only ranks the cards"""
    # card -> [observed, predicted, observations, binomial variance]
    per: collections.defaultdict[int, list[float]] = collections.defaultdict(lambda: [0, 0.0, 0, 0.0])
    res: list[tuple[int, float, int]] = []
    for r, kinds, chain, obs in dataset(ALL):
        mb = reach(kinds, budget_pmf(r), RULE) if budget else reach(kinds)
        for ci, lk, seen, bg in obs:
            if lk != kinds[ci][0]:
                continue
            a = per[chain[ci]]
            p_ = seen_chance(mb[(ci, lk)], bg)
            a[0] += seen
            a[1] += p_
            a[2] += 1
            a[3] += p_ * (1 - p_)
            res.append((chain[ci], seen - p_, r["id"]))
    k = sum(v[0] for v in per.values())
    e = sum(v[1] for v in per.values())
    n = sum(v[2] for v in per.values())
    zs = {c: (v[0] - v[1]) / math.sqrt(v[3]) for c, v in per.items() if v[2] >= 60}
    kept = [(c, d, r) for c, d, r in res if c in zs]
    x2, _, pv = common.cluster_chi2([(c, d) for c, d, _ in kept], [r for _, _, r in kept])
    names = game().cards
    return FinalByCard(
        obs=k / n,
        model=e / n,
        n=n,
        cards=len(zs),
        chi2=x2,
        p=pv,
        low=sorted(zs.items(), key=lambda t: t[1])[:3],
        high=sorted(zs.items(), key=lambda t: -t[1])[:3],
        rate={c: (per[c][0] / per[c][2], per[c][1] / per[c][2]) for c in zs},
        name=lambda c: names[c]["name"],
    )


class AllowanceSensitivity(NamedTuple):
    """allowance_sens()"""

    best_p: dict[float, float]  # allowance -> the best p at n = CHAIN_N (0.0: on the runs without a contradiction)
    clean_runs: int  # runs whose observations show no contradiction


def best_p_at_allowance(xs: list[Row], eps: float) -> float:
    """the best p at n = CHAIN_N on a grid 0.300..0.400, with a fired link's marker missing at chance eps"""
    ab = [run_levels(x.kinds, x.obs, eps=eps) for x in xs]
    A = np.array([a for a, _ in ab])
    B = np.array([b for _, b in ab])
    res: list[tuple[float, float]] = []
    for p_ in [i / 1000 for i in range(300, 401, 5)]:
        pm = np.array(binom_pmf(CN, p_, KMAX + 1))
        v = A @ pm + B @ survival(pm)
        if (v > 0).all():
            res.append((float(np.log(v).sum()), p_))
    return max(res)[1]


def contradicts(obs: list[Obs]) -> bool:
    """some card's link unseen while a later link of it is seen with a clean marker (impossible without the allowance)"""
    per: collections.defaultdict[int, dict[int, tuple[bool, float]]] = collections.defaultdict(dict)
    for ci, lk, seen, bg in obs:
        per[ci][lk] = (seen, bg)
    return any(
        not sn and any(l2 > lk and s2 and b2 < BG_CLEAN for l2, (s2, b2) in d.items())
        for d in per.values()
        for lk, (sn, _) in d.items()
    )


def allowance_sens() -> AllowanceSensitivity:
    """the best p at n = CHAIN_N under several allowances, and with none on the runs showing no contradiction"""
    xs = dataset(ALL)
    best: dict[float, float] = {}
    for e in (0.002, 0.005, 0.01, 0.02):
        best[e] = best_p_at_allowance(xs, e)
    clean = [x for x in xs if not contradicts(x.obs)]
    best[0.0] = best_p_at_allowance(clean, 0.0)
    return AllowanceSensitivity(best, len(clean))


class RuleFinal(NamedTuple):
    """final links at one pal count against one allocation rule"""

    observed: float
    predicted: float
    links: int
    low: float  # the observed rate's run-clustered 95% interval
    high: float


class Allocation(NamedTuple):
    """alloc_claims()"""

    total: collections.Counter[str]  # rule -> total per-observation log-likelihood
    final: dict[tuple[int, str], RuleFinal]  # (pal count, rule) -> final links


def alloc_claims() -> tuple[list[common.Value | common.Finding], Allocation]:
    """slot rule against card rule (every run, 0-5 pals; per-observation likelihood); final links observed with the
    run-clustered 95% interval against each rule"""
    tot: collections.Counter[str] = collections.Counter()
    fin: dict[tuple[int, str], RuleFinal] = {}
    for npal in (5, 4, 3, 2, 1, 0):
        xs = dataset({npal})
        for rule in ("slots", "cards"):
            tot[rule] += obs_ll(xs, BIN, rule)
            o = p = 0.0
            n = 0
            rows: list[tuple[float, float, int]] = []
            for r, kinds, _, obs in xs:
                mb = reach(kinds, BIN, rule)
                for ci, lk, seen, bg in obs:
                    if lk == kinds[ci][0]:
                        o += seen
                        p += seen_chance(mb[(ci, lk)], bg)
                        n += 1
                        rows.append((seen, seen_chance(mb[(ci, lk)], bg), r["id"]))
            fin[(npal, rule)] = RuleFinal(
                o / n, p / n, n, *obs_ci(rows)
            )  # the observed rate's interval against this rule
    out: list[common.Value | common.Finding] = [
        V(f"allocation: final links, {npal} pals, card rule", fin[(npal, "cards")].predicted) for npal in (2, 0)
    ]
    out.append(V("allocation: slot over card, nats", tot["slots"] - tot["cards"], note="every run, 0-5 pals"))
    return out, Allocation(tot, fin)


def with_activation(kinds: Sequence[Kind], a: float) -> tuple[Kind, ...]:
    """the chain kinds with each card active with probability a (a stop at link 0 with chance 1 - a)"""
    return (
        tuple(Kind(k.links, k.last_link, tuple(sorted((*k.stop_risk, (0, round(1 - a, 6), 1))))) for k in kinds)
        if a < 1
        else tuple(kinds)
    )


class NeverStarted(NamedTuple):
    """chains never started (link 1 not reached) with a given number of chain cards"""

    not_started: float
    links: int
    model: float  # the model's share not started
    low: float  # the observed share's run-clustered 95% interval
    high: float
    final: float  # the final model's share not started


def never_started_claims() -> list[common.Value | common.Finding]:
    """chains never started: link 1 not reached, by chain cards in the deck"""
    out: list[common.Value | common.Finding] = []
    # chain cards -> [link 1 seen, predicted reach, observations]
    rows: collections.defaultdict[int, list[float]] = collections.defaultdict(lambda: [0, 0.0, 0, 0.0])
    nrows: collections.defaultdict[int, list[tuple[float, float, int]]] = collections.defaultdict(list)
    for r, kinds, _, obs in dataset(ALL):
        mb, mf = reach(kinds, BIN), reach(kinds, budget_pmf(r), RULE)
        for ci, lk, seen, bg in obs:
            if lk == 1:
                a = rows[len(kinds)]
                a[0] += seen
                a[1] += seen_chance(mb[(ci, 1)], bg)
                a[2] += 1
                a[3] += seen_chance(mf[(ci, 1)], bg)
                nrows[len(kinds)].append((1 - seen, 1 - seen_chance(mb[(ci, 1)], bg), r["id"]))
    MISC["never rows"] = dict(rows)
    for nc in (5, 6):  # several chain cards per run: the observed rate's interval is run-clustered
        o, p_, n, pf = rows[nc]
        lo, hi = obs_ci(nrows[nc])
        MISC[("never", nc)] = NeverStarted(n - o, n, 1 - p_ / n, lo, hi, 1 - pf / n)
        out += [
            V(f"never started, {nc} chain cards: observed", 1 - o / n, note=f"link 1 not reached, {n} card-links"),
            V(f"never started, {nc} chain cards: observed 95% low", lo),
            V(f"never started, {nc} chain cards: observed 95% high", hi),
            V(f"never started, {nc} chain cards: model", 1 - p_ / n),
            V(f"never started, {nc} chain cards: final model", 1 - pf / n),
        ]
    return out


def claims() -> list[common.Value | common.Finding]:
    out = derived()
    if not common.have_runs():
        return out
    fc = fit_claims()
    flc, fl = final_claims()
    ac, al = alloc_claims()
    out += fc + allowance_claims() + flc + ac + never_started_claims() + profile_claims() + ends_claims()
    sens = allowance_sens()
    gap = next(
        v.got
        for v in out
        if isinstance(v, common.Value) and v.name == "Binomial(21, 0.35): nats behind distribution-free"
    )
    out.append(
        V(
            "binomial(21, 0.35) LR p",
            float(chi2.sf(2 * gap, KMAX)),
            note=f"{KMAX} nominal df; reference needs calibration",
        )
    )
    fb = final_by_card()
    out += [
        V("final links by card: pooled observed", fb.obs, note=f"{fb.n:,} card-links, every run"),
        V("final links by card: pooled model", fb.model),
        V("final links by card: cards", fb.cards),
        V("final links by card: chi2", fb.chi2),
    ]
    fbb = final_by_card(budget=True)
    out += [
        V("final links by card, pal-count budget: pooled model", fbb.model),
        V("final links by card, pal-count budget: chi2", fbb.chi2),
        V("final links by card, pal-count budget: Wald p", fbb.p),
        V("final links by card, pal-count budget: largest |z|", max(abs(z) for _, z in fbb.low + fbb.high)),
    ]
    MISC["final_by_card"] = fb
    MISC["final_by_card_budget"] = fbb
    MISC["alloc"] = al
    out += [
        V("allowance sensitivity: lowest best p at n = 21", min(sens.best_p.values())),
        V("allowance sensitivity: highest best p at n = 21", max(sens.best_p.values())),
    ]
    g: dict[str, Any] = {c.name: c.got for c in out if isinstance(c, common.Value)}
    return out + fit_findings(g) + allowance_findings(g, sens) + link_findings(g, fl, al) + miss_findings(g)


# ---------------------------------------------------------------- findings: the section's conclusions, each tested on
# the numbers above (g: value name -> value)
def F(*a: Any) -> common.Finding:
    return common.finding(SEC, *a)


def fit_findings(g: dict[str, Any]) -> list[common.Finding]:
    """the form of N"""
    fs: list[common.Finding] = []
    gap = g["Binomial(21, 0.35): nats behind distribution-free"]
    df = g["fixed binomial: nominal LR df"]
    lrp = g["binomial(21, 0.35) LR p"]
    fs.append(
        F(
            f"Known miss: Binomial({CN}, {CP}) has nominal LR p < 0.05 against a free distribution of N "
            f"({df:.0f} df; the reference distribution needs calibration)",
            gap > 0 and df == KMAX and lrp < 0.05,
            f"{gap:.1f} nats behind distribution-free, LR p = {lrp:.2g}; best binomial (n free) "
            f"{g['best binomial n free: nats behind distribution-free']:.1f} nats behind, p "
            f"{g['best binomial n free: LR p']:.3f}; Binomial({CN}, {CP}) on the runs with neither Sasami Anshinzawa's card "
            f"nor Inari One p {g['Binomial(21, 0.35) LR p, runs with neither']:.3f}",
        )
    )
    a_, i_ = "Sasami Anshinzawa's card", "Inari One"
    fs.append(
        F(
            "N rises with the deck's pal cards (group cards excluded): the pal-count budget, logit p = a + b x pals at "
            f"n = {CN}, has b > 0 with likelihood-ratio p < 0.05 against one p",
            g["pal-count budget: b"] > 0 and g["pal-count budget: LR p against one p at n = 21"] < 0.05,
            f"a {g['pal-count budget: a']:+.3f}, b {g['pal-count budget: b']:+.3f} (LR p "
            f"{g['pal-count budget: LR p against one p at n = 21']:.2g}); p from {g['pal-count budget: p at 0 pals']:.3f} "
            f"at 0 pals to {g['pal-count budget: p at 5 pals']:.3f} at 5, mean N {g['pal-count budget: mean N at 0 pals']:.2f} "
            f"to {g['pal-count budget: mean N at 5 pals']:.2f}",
        )
    )
    fs.append(
        F(
            "On top of the pal-count budget, no own term for decks with Sasami Anshinzawa [This Might Sting!] is detectable "
            "(LR p > 0.05): their extra chain events are accounted for by their pal count",
            g[f"{a_}: own term on the pal-count budget, LR p"] > 0.05,
            f"own term {g[f'{a_}: own term on the pal-count budget']:+.3f}, LR p "
            f"{g[f'{a_}: own term on the pal-count budget, LR p']:.2f}; without the budget, p at n = {CN} "
            f"{g[f'{a_}: p at n = {CN}']:.3f} vs {g[f'{a_}: p at n = {CN}, the other runs']:.3f} (LR p "
            f"{g[f'{a_}: p at n = {CN} on each side, LR p']:.2g}); mean pal count {g[f'{a_}: mean pal count']:.2f} vs "
            f"{g[f'{a_}: mean pal count, the other runs']:.2f}",
        )
    )
    fs.append(
        F(
            "Known miss: Inari One's careers reach fewer chain events than the pal-count budget predicts (her own term "
            "below zero, LR p < 0.05)",
            g[f"{i_}: own term on the pal-count budget"] < 0
            and g[f"{i_}: own term on the pal-count budget, LR p"] < 0.05,
            f"own term {g[f'{i_}: own term on the pal-count budget']:+.3f}, LR p "
            f"{g[f'{i_}: own term on the pal-count budget, LR p']:.2g} on {g[f'{i_}: runs']:.0f} runs",
        )
    )
    pg_ = g["Poisson(7.1): nats behind distribution-free"]
    dfp = KMAX  # the free fit's KMAX + 1 cells have KMAX free parameters; the fixed Poisson has none
    fs.append(
        F(
            f"Poisson(7.1) is rejected against a free distribution of N (likelihood ratio on the free fit's {dfp} parameters, a fixed law against them; p < 0.05)",
            chi2.sf(2 * pg_, dfp) < 0.05,
            f"{pg_:.1f} nats behind distribution-free, LR p = {chi2.sf(2 * pg_, dfp):.2g}",
        )
    )
    vm, cp_, bp_, bc = (
        g["distribution-free: variance / mean"],
        g["Conway-Maxwell-Poisson: LR p"],
        g["best binomial n free: LR p"],
        g["best binomial n free over Conway-Maxwell-Poisson: nats"],
    )
    fs.append(
        F(
            f"The capped count min(N, {KMAX}) has variance / mean below 1 (uncapped dispersion is not identified), and "
            "of the two under-dispersed two-parameter families the binomial fits better: the best binomial's log-likelihood "
            "is above the Conway-Maxwell-Poisson fit's (same parameter count, compared directly)",
            vm < 1 and bc > 0,
            f"variance / mean {vm:.2f}; best binomial {bc:+.1f} nats over CMP; against distribution-free, LR p binomial "
            f"{bp_:.3f}, CMP {cp_:.3f}",
        )
    )
    hz = {k.split(": ")[1]: v for k, v in g.items() if k.startswith("held out:") and k.endswith(": z")}
    best = (
        "pal-count budget over today's model",
        "pal-count budget over free",
        "pal-count budget over the same with the slot rule",
    )
    fs += end_findings(g)
    nz = {k.split(": ")[1]: v for k, v in g.items() if k.startswith("nested held out:") and k.endswith(": z")}
    fs.append(
        F(
            "With every fitted nuisance (marker backgrounds, steering and no-list end shares) measured on the training "
            "folds only, the pal-count budget still predicts unseen runs better than today's model (5-fold, z > 1.96)",
            nz["pal-count budget over today's model"] > 1.96,
            "; ".join(f"{f} z {z:+.2f}" for f, z in nz.items()),
        )
    )
    fs.append(
        F(
            "The pal-count budget under the card rule predicts unseen runs better than today's model (Binomial(21, 0.35) "
            "with the slot rule), than a free distribution of N, and than the same budget with the slot rule: 5-fold "
            "cross-validation, per-run held-out log-likelihoods compared pairwise, each z > 1.96",
            all(hz[f] > 1.96 for f in best),
            "; ".join(f"{f} z {z:+.2f}" for f, z in hz.items()),
        )
    )
    lo_, hi_, b21 = (
        g["binomial n: 95% profile interval low"],
        g["binomial n: 95% profile interval high"],
        g["binomial n = 21, p re-fitted: nats below the best n"],
    )
    fs.append(
        F(
            "In-sample, n = 21 lies just outside the 95% profile-likelihood interval for the binomial's n (p re-fitted at each n; "
            "more than 1.92 nats below the best n)",
            b21 > CHI1 and not lo_ <= 21 <= hi_,
            f"n in [{lo_:.0f}, {hi_:.0f}], best n {g['best binomial n free: n']:.0f}; n = 21 is {b21:.2f} nats below",
        )
    )
    pg = g["per scenario on the pal-count budget: log-likelihood gain"]
    dfs = g["per scenario on the pal-count budget: extra parameters"]
    pf, dff = g["per scenario: log-likelihood gain"], g["per scenario: extra parameters"]
    fs.append(
        F(
            "No scenario intercept is detectable on the pal-count budget (likelihood ratio on "
            f"{dfs:.0f} parameters, p > 0.05)",
            chi2.sf(2 * pg, dfs) > 0.05,
            f"+{pg:.1f} nats, LR p = {chi2.sf(2 * pg, dfs):.2f}; a free distribution per scenario: +{pf:.1f} nats on "
            f"{dff:.0f}, LR p = {chi2.sf(2 * pf, dff):.2g} (the scenarios' pal mixes differ)",
        )
    )
    fs.append(
        F(
            f"The stop share {STOP_SHARE:.3g} and the no-list scale {NOLIST_SCALE:.3g} each lie within 1.92 nats of "
            "their grid profile's best (the budget re-fitted at each value)",
            g["stop share profile: the model's value, nats below the grid's best"] <= CHI1
            and g["no-list scale profile: the model's value, nats below the grid's best"] <= CHI1,
            "; ".join(
                f"{nm} {g[f'{nm} profile: the model' + chr(39) + 's value, nats below the grid' + chr(39) + 's best']:.2f} "
                f"nats below, grid values within 1.92 nats {g[f'{nm} profile: lowest grid value within 1.92 nats']:.3g}"
                f"-{g[f'{nm} profile: highest grid value within 1.92 nats']:.3g}"
                for nm in ("stop share", "no-list scale")
            ),
        )
    )
    return fs


def end_findings(g: dict[str, Any]) -> list[common.Finding]:
    """freed or wasted (ends_claims): the search, the bootstrap placements, held out, the mixture, the direct deck-mate
    contrasts and the ending-choice reading"""
    fs: list[common.Finding] = []
    joint = [g[f"chain ends, {nm}: the search finds the joint grid's best"] for nm in END_NAMES.values()]
    fs.append(
        F(
            "On the real runs, each rule's coordinate search over the stop share and no-list scale reaches the best point "
            "of the full grid (the procedure the synthetic pools repeat finds each rule's best constants)",
            all(j == 1 for j in joint),
            ", ".join(
                f"{nm}: {g[f'chain ends, {nm}: stop share']:.3f} / {g[f'chain ends, {nm}: no-list scale']:.2f}"
                for nm in END_NAMES.values()
            ),
        )
    )

    def place(stat: str) -> tuple[int, int, str]:
        bf = g[f"chain ends: {stat}: pools if freed below the real runs"]
        bw = g[f"chain ends: {stat}: pools if wasted below the real runs"]
        return (
            bf,
            bw,
            f"{stat}: real {g[f'chain ends: {stat}: real runs']:+.3g}; freed pools "
            f"{g[f'chain ends: {stat}: pools if freed: mean']:+.3g} ± {g[f'chain ends: {stat}: pools if freed: SD']:.2g} "
            f"({bf} below), wasted pools {g[f'chain ends: {stat}: pools if wasted: mean']:+.3g} ± "
            f"{g[f'chain ends: {stat}: pools if wasted: SD']:.2g} ({bw} below)",
        )

    lls = ["freed - wasted"] + [f"freed - wasted, {grp}" for grp in END_GROUPS]
    pl = [place(st) for st in lls]
    fs.append(
        F(
            "Conditional on the rest of the model, ended chains free their events: on the freed - wasted log-likelihood "
            "the real runs score above every synthetic pool generated with ends wasted and inside the range of the pools "
            "generated with ends freed, in total and within each group of runs by what can end a chain (re-fitting "
            "parametric bootstrap: both rules' constants re-searched and budgets re-fitted in every pool)",
            all(bw == BOOT_SIMS and 0 < bf < BOOT_SIMS for bf, bw, _ in pl),
            "; ".join(t for _, _, t in pl),
        )
    )
    nm_ = f"freed - wasted, {END_GROUPS[3]}, without {END_CARDS[MARVELOUS]}"
    bf, bw, t_nm = place(nm_)
    fs.append(
        F(
            f"On runs showing no list, the evidence comes from {END_CARDS[MARVELOUS]}'s decks: without them the real runs' "
            "freed - wasted lies inside the range of both the freed and the wasted pools",
            0 < bf < BOOT_SIMS and 0 < bw < BOOT_SIMS,
            t_nm,
        )
    )
    hz = {nm: g[f"chain ends held out: {nm}: z"] for nm in ("freed over wasted", "mechanism alone: freed over wasted")}
    fs.append(
        F(
            "Held out, with each rule's stop share and no-list scale searched on the training folds, freed predicts unseen "
            "runs better than wasted (5-fold, z > 1.96), and so does the mechanism alone: the budget fitted on the "
            "training runs where no chain can end and held fixed, the runs where one can scored",
            all(z > 1.96 for z in hz.values()),
            "; ".join(f"{nm} z {z:+.2f}" for nm, z in hz.items()),
        )
    )
    fs.append(
        F(
            "Freed stays ahead of wasted with every link left out that some run reads through a credit below the levels "
            "its outcomes give (each rule's constants and budget re-fitted)",
            g["impossible credits: freed - wasted without those links"] > 0,
            f"{g['impossible credits: freed - wasted without those links']:+.1f} nats without "
            f"{g['impossible credits: links left out']:.0f} links ({g['impossible credits: readings left out']:,.0f} "
            f"readings); budget a {g['impossible credits: budget a without those links']:+.3f}, b "
            f"{g['impossible credits: budget b without those links']:+.3f}",
        )
    )
    sc = {f: g[f"chain ends: freed - wasted, every background times {f:g}"] for f in BG_SCALES}
    fs.append(
        F(
            "Freed stays ahead of wasted in-sample with every marker background halved or raised by half (each rule's "
            "constants and budget re-fitted)",
            all(v > 0 for v in sc.values()),
            ", ".join(f"times {f:g}: {v:+.1f} nats" for f, v in sc.items()),
        )
    )
    lo, hi = (
        g["chain ends, mixture: lowest freed share within 1.92 nats"],
        g["chain ends, mixture: highest freed share within 1.92 nats"],
    )
    fs.append(
        F(
            "A mixture whose runs' chains are freed with chance lam and wasted otherwise puts lam's 1.92-nat range "
            "away from 0 (every run wasted)",
            lo > 0,
            f"lam {g['chain ends, mixture: freed share']:.2f}, range {lo:.2f}-{hi:.2f}; all wasted "
            f"{g['chain ends, mixture: nats of all wasted below the best']:.1f} nats below the best, all freed "
            f"{g['chain ends, mixture: nats of all freed below the best']:.1f}",
        )
    )
    c = "deck-mates, 2-link SR - plain"
    fs.append(
        F(
            "The direct deck-mate comparison sees freed events where both rules agree they exist: the deck-mates of a "
            "2-link SR gain, against plain 3-link decks, what both rules predict (|z| < 2 from each) and more than no "
            "gain (z > 2: a finished chain's events go to the other cards, not lost with it)",
            abs(g[f"{c}: z from freed"]) < 2 and abs(g[f"{c}: z from wasted"]) < 2 and g[f"{c}: z from no gain"] > 2,
            f"measured {g[f'{c}: measured']:+.3f} ± {g[f'{c}: SE']:.3f}; freed {g[f'{c}: freed predicts']:+.3f}, wasted "
            f"{g[f'{c}: wasted predicts']:+.3f}; z from no gain {g[f'{c}: z from no gain']:+.2f}",
        )
    )
    early = ("deck-mates, default-choice stop - plain", "deck-mates, prioritised chosen end - plain")
    fs.append(
        F(
            "Directly, the deck-mates of a card whose chain ends early (a default-choice stop, or a chain-ending choice "
            "the list prioritises) gain between the two rules' predictions, within 2 SE of each: the direct comparison "
            "does not decide between them",
            all(abs(g[f"{e}: z from freed"]) < 2 and abs(g[f"{e}: z from wasted"]) < 2 for e in early),
            "; ".join(
                f"{e}: {g[f'{e}: measured']:+.3f} ± {g[f'{e}: SE']:.3f}, freed {g[f'{e}: freed predicts']:+.3f}, "
                f"wasted {g[f'{e}: wasted predicts']:+.3f}"
                for e in early
            ),
        )
    )
    fs.append(
        F(
            "With each rule's constants refined on a finer grid around its best, freed stays ahead of wasted in-sample",
            g["chain ends: freed - wasted, constants refined on the finer grid"] > 0,
            f"{g['chain ends: freed - wasted, constants refined on the finer grid']:+.1f} nats; freed "
            f"{g['chain ends, freed: refined stop share']:.3f} / {g['chain ends, freed: refined no-list scale']:.3f}, "
            f"wasted {g['chain ends, wasted: refined stop share']:.3f} / {g['chain ends, wasted: refined no-list scale']:.3f}",
        )
    )
    k = "ending choice, every card"
    seen_p, bg_p = g[f"{k}, prioritised: later links seen"], g[f"{k}, prioritised: expected from background alone"]
    seen_n, mod_n = g[f"{k}, not prioritised: later links seen"], g[f"{k}, not prioritised: expected by the model"]
    fs.append(
        F(
            "Prioritising the skill on a chain-ending choice ends the chain: with it prioritised alone, the links after "
            "the choice show about as often as their markers' background alone gives (Poisson z < 2); with nothing on "
            "that choice prioritised, they show as often as the model predicts (|z| < 2)",
            (seen_p - bg_p) / math.sqrt(bg_p) < 2 and abs(seen_n - mod_n) / math.sqrt(mod_n) < 2,
            f"prioritised: {seen_p:.0f} seen of {g[f'{k}, prioritised: later links read']:.0f}, background alone "
            f"{bg_p:.1f}, if never ended {g[f'{k}, prioritised: expected if the chain never ended']:.1f}, chains going on "
            f"{g[f'{k}, prioritised: share of chains going on']:.3f} ± "
            f"{g[f'{k}, prioritised: share of chains going on, SE']:.3f}; not prioritised: {seen_n:.0f} of {g[f'{k}, not prioritised: later links read']:.0f}, model "
            f"{mod_n:.1f}, background alone {g[f'{k}, not prioritised: expected from background alone']:.1f}",
        )
    )
    return fs


def allowance_findings(g: dict[str, Any], sens: AllowanceSensitivity) -> list[common.Finding]:
    """the marker allowance: measured, and what it does to the fit"""
    sh = g["allowance: share"]
    lo, hi = g["allowance: 95% low"], g["allowance: 95% high"]
    ps = list(sens.best_p.values())
    return [
        F(
            f"The {EPS} marker allowance lies inside the 95% interval of the measured rate of fired links whose marker is missing",
            lo <= EPS <= hi,
            f"{g['allowance: marker not shown']:.0f} of {g['allowance: earlier links implied fired']:.0f} = {sh:.3f} [{lo:.3f}, {hi:.3f}]",
        ),
        F(
            f"At fixed n = {CN}, the best p moves by at most 0.01 over the tested marker allowances; zero allowance "
            "uses only runs without a contradiction",
            bool(ps) and max(ps) - min(ps) <= 0.01,
            ", ".join(f"eps {k}: {v:.3f}" for k, v in sens.best_p.items())
            + f" (eps 0.0 on the {sens.clean_runs} runs without a contradiction)",
        ),
    ]


def placement(prediction: float, low: float, high: float) -> str:
    """where a prediction lies against an observed interval"""
    return "inside" if low <= prediction <= high else "above" if prediction > high else "below"


def link_findings(g: dict[str, Any], fl: LinkTables, al: Allocation) -> list[common.Finding]:
    """final links by pal count and the allocation rule"""
    fs: list[common.Finding] = []
    pos = {npal: placement(t.budget, t.low, t.high) for npal, t in fl.final.items()}
    today = {npal: placement(t.binomial, t.low, t.high) for npal, t in fl.final.items()}
    fs.append(
        F(
            "Final-link markers by pal count: the final model's prediction lies inside the observed 95% interval at every "
            "pal count; today's model lies outside it at five of the six",
            all(v == "inside" for v in pos.values()) and sum(v != "inside" for v in today.values()) == 5,
            "; ".join(
                f"{k} pals {t.observed:.3f} [{t.low:.3f}, {t.high:.3f}]: final {t.budget:.3f} ({pos[k]}), today "
                f"{t.binomial:.3f} ({today[k]})"
                for k, t in sorted(fl.final.items(), reverse=True)
            ),
        )
    )
    tl = al.total
    fs.append(
        F(
            "With N fixed at Binomial(21, 0.35) (allowance and backgrounds as in the fits), the slot rule exceeds the card rule by more than "
            "1.92 nats in composite link score (with the pal-count budget the card rule predicts better: fit_findings)",
            g["allocation: slot over card, nats"] > 1.92,
            f"slots {tl['slots']:.1f}, cards {tl['cards']:.1f}: {tl['slots'] - tl['cards']:+.1f} nats",
        )
    )

    return fs


def miss_findings(g: dict[str, Any]) -> list[common.Finding]:
    """the known misses"""
    fs: list[common.Finding] = []
    zs = {k: g[f"early-link shortfall, {k} pals: z"] for k in (4, 3, 2, 1, 0)}
    zb = {k: g[f"early-link shortfall under the budget, {k} pals: z"] for k in (4, 3, 2, 1, 0)}
    fs.append(
        F(
            "Known miss: early-link markers (links 1 and 2) against the final model run short at 3 pals (z > 2) and above "
            "it at 1 pal (z < -2), within |z| 2 elsewhere; today's model runs short (z > 2) at 3, 2, 1 and 0 pals",
            {k: v > 0 for k, v in zb.items() if abs(v) > 2} == {3: True, 1: False}
            and {k for k, v in zs.items() if v > 2} == {3, 2, 1, 0},
            "final "
            + ", ".join(f"{k} pals z {v:+.1f}" for k, v in zb.items())
            + "; today "
            + ", ".join(f"{k} pals z {v:+.1f}" for k, v in zs.items()),
        )
    )
    nv = {nc: MISC[("never", nc)] for nc in (5, 6)}
    fs.append(
        F(
            "Chains never started (link 1 not reached) with five or six chain cards: today's model predicts too few, below "
            "the observed 95% interval at both; the final model lies inside it at six and just above it at five",
            all(x.low > x.model for x in nv.values())
            and nv[6].low <= nv[6].final <= nv[6].high
            and nv[5].high < nv[5].final < nv[5].high + 0.01,
            ", ".join(
                f"{nc} cards {x.not_started / x.links:.2f} [{x.low:.2f}, {x.high:.2f}] vs {x.model:.2f} (final model "
                f"{x.final:.2f})"
                for nc, x in nv.items()
            ),
        )
    )
    fb: FinalByCard = MISC["final_by_card_budget"]
    zmax = max(abs(z) for _, z in fb.low + fb.high)
    fs.append(
        F(
            "Final-link markers by card against the final model: pooled within 0.01, no card beyond |z| = 3, and the "
            "run-clustered spread across cards at nominal Wald p > 0.01",
            abs(fb.obs - fb.model) < 0.01 and zmax <= 3 and fb.p > 0.01,
            f"pooled {fb.obs:.3f} vs {fb.model:.3f}; chi2 {fb.chi2:.0f} on {fb.cards} cards, p = {fb.p:.1g}; lowest "
            + ", ".join(f"{fb.name(c)} {fb.rate[c][0]:.2f}/{fb.rate[c][1]:.2f}" for c, _ in fb.low)
            + "; highest "
            + ", ".join(f"{fb.name(c)} {fb.rate[c][0]:.2f}/{fb.rate[c][1]:.2f}" for c, _ in fb.high),
        )
    )
    return fs


def detail() -> None:
    """the fits' exact figures and the breakdowns behind the known misses (from the run of claims())"""
    if not common.have_runs() or "fit" not in MISC:
        return
    f = MISC["fit"]
    print(
        f"\nfit of N ({f['runs']} runs; by pals {f['by_pals']}): log-likelihood distribution-free {f['llp']:.2f}, "
        + ", ".join(f"{nm} {v:.2f}" for nm, v in f["lls"].items())
    )
    print(
        "link 1 not reached, observed / Binomial(21, 0.35) / final model, by chain cards: "
        + ", ".join(
            f"{c}: {1 - o / n:.2f} / {1 - p / n:.2f} / {1 - pf / n:.2f} (n {n})"
            for c, (o, p, n, pf) in sorted(MISC["never rows"].items())
        )
    )
    fin: dict[tuple[int, str], RuleFinal] = MISC["alloc"].final
    print(
        "allocation, final links observed / slot rule / card rule: "
        + ", ".join(
            f"{npal} pals {fin[(npal, 'slots')].observed:.2f} / {fin[(npal, 'slots')].predicted:.2f} / {fin[(npal, 'cards')].predicted:.2f}"
            for npal in (5, 4, 3, 2, 1, 0)
        )
    )


if __name__ == "__main__":
    common.main(claims)
    detail()
