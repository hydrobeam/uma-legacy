"""Section 8 of docs/RESEARCH.md: chain events.

Reproduces the derived numbers (Binomial(21, 0.35), P(N >= 6), the exact allocation tables, the worked example) and
the fits on the run records (research/data/runs.jsonl, not included; Independent Training runs from uma-it).

Markers. A card's chain progress is read from "marker" skills of its link events (GameTora card events through
gametora.py, chain links from master.mdb single_mode_story_data.show_progress_1) credited to "<character> event" in a
run's hint list. A link is observable only if every choice and every outcome of its event carries a skill appearing
in no other event of the card. A link is unknown when every matching credit names another deck card
whose events can give the marker; otherwise a matching credit counts as seen; a marker listed with "Inspiration" as its only source is
unknown. Decks with an unknown card or a non-pal R card are left out.

Data. Every analysis runs on every run record, selected only by pal count
where the analysis is by pal count:
  - the fit of N (0-5 pals): a distribution-free fit (EM over P(N = 0..17), P(N >= 18)) against Binomial(21, 0.35),
    Binomial(15, 0.48), Poisson(7.1) and the binomial's profile over n (p re-fitted at each n), each by likelihood ratio
    (the thresholds used are in fit_findings); the same
    fit per scenario (scenarios with 30+ runs) against one N for all;
  - N per pal count (1-4 pals): the binomial grid MLE on each pal count's runs;
  - the marker allowance: the share of links implied fired (a later link of the card shows) whose marker is missing,
    and the best p at n = 21 under allowances 0.01-0.045 and with none on the runs without such a contradiction;
  - the decks with Sasami Anshinzawa [This Might Sting!] and the Inari One careers against the other runs: separate
    free fits, and p at n = 21 re-fitted on each side;
  - the candidate Binomial(18, 0.405) against Binomial(21, 0.35): in-sample, and the binomial fitted on the general
    collection (batches 0-17) scored on the other runs;
  - final links by pal count against Binomial(21, 0.35) and Poisson(7.1);
  - final links by card against the model, the spread between cards with 60+ observations (run-clustered Wald chi2);
  - the allocation rule: slot rule against card rule, per-observation log-likelihood, and final links at 2 and 0 pals;
  - known misses: two-chain decks (two plain 3-link SSRs, 4 pals), early links (1 and 2) per pal count with the run as
    a cluster, chains never started with five or six chain cards;
  - the activation parameter a (a chain card active with probability a), fitted jointly with N ~ Binomial(21, p),
    exact over the chain states on every deck.
The model: N chain events per run; each advances one running non-pal card's chain by a link, the card picked with
weight = links left ("slots") or uniformly ("cards"); stop risk (card_events.json) ends a chain at link l with k/n.
Fits of N are exact over the chain states, with a 2% chance that a fired link's marker does not show (without it a run
whose later link shows while an earlier one does not has likelihood 0; allowance_claims measures the rate); the
allocation, final-link and early-link comparisons score each observation on its own
(P(card reaches link) on the run's deck). Intervals on rates over observations that come several to a run (final
links, the allowance, chains never started) are run-clustered (common.cluster_ci), the spread between cards a
run-clustered Wald test (common.cluster_chi2); a share with one observation per run (two-chain decks) takes Wilson."""

import collections, itertools, math, os, re, sys
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from typing import Any, NamedTuple
import numpy as np
from scipy.optimize import brentq, minimize, minimize_scalar
from scipy.stats import chi2

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
from common import Run


class Kind(NamedTuple):
    """a chain card"""

    links: int  # its chain's links
    last_link: int  # the link its chain ends at when no stop risk ends it earlier
    stop_risk: tuple[tuple[float, ...], ...]  # (link, k, n) each: the chain ends at that link with chance k / n


class Obs(NamedTuple):
    """an observation: whether a chain card's link was seen"""

    card: int  # index in the run's chain cards
    link: int
    seen: bool


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


class BinomialFit(NamedTuple):
    """a Binomial(n, p) fit of N"""

    loglik: float
    n: int
    p: float


Pmf = Sequence[float] | np.ndarray

SEC: str = "8"
EPS: float = 0.02  # chance a fired link's marker hint does not show (the fits' allowance; allowance_claims measures it)
# Global web_cards.json has at most three links per chain card, and a deck has six cards.
# Every path absorbs by sum(kind.last_link) <= 18; run_levels enforces that bound.
# Cutting here keeps all distinguishable counts, without duplicate post-absorption columns.
KMAX: int = 6 * 3  # distribution-free fit: P(N = 0..17) and P(N >= 18)
ALL: set[int] = {0, 1, 2, 3, 4, 5}  # pal counts of decks with a chain card
CHI1: float = 1.920729  # chi2(1) 95% / 2
CANDIDATE: tuple[int, float] = (18, 0.405)  # the binomial proposed for N in place of (CHAIN_N, CHAIN_P)
ANSHINZAWA: int = 30080  # [This Might Sting!] Sasami Anshinzawa, a pal card: decks with her reach more chain events
INARI: int = 1034  # Inari One: her careers reach fewer
GENERAL_BATCHES: range = range(18)  # the general collection's batches; later ones were fetched for targeted tests
MISC: dict[Any, Any] = {}  # numbers the findings and the detail use that the document does not state


# ---------------------------------------------------------------- the model
def binom_pmf(n: int, p: float, upto: int = 61) -> list[float]:
    return [math.comb(n, e) * p**e * (1 - p) ** (n - e) if e <= n else 0.0 for e in range(upto)]


def pois_pmf(lam: float, upto: int = 61) -> list[float]:
    return [math.exp(-lam) * lam**e / math.factorial(e) for e in range(upto)]


def chain_model() -> tuple[int, float]:
    """the solver's event count, N ~ Binomial(CHAIN_N, CHAIN_P) (pkg/loopopt/src/deck/chains.rs)"""
    src = open(f"{common.ROOT}/pkg/loopopt/src/deck/chains.rs", encoding="utf-8").read()
    return int(re.search(r"CHAIN_N: u32 = (\d+);", src).group(1)), float(
        re.search(r"CHAIN_P: f64 = ([\d.]+);", src).group(1)
    )


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
    P(N >= l) when nothing can advance (absorbed). rule: 'slots' (weight = links left) or 'cards' (uniform)."""
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
                left = [(ck[i][0] - s[i]) if s[i] < stops[i] else 0 for i in range(n)]
                ws = left if rule == "slots" else [1 if x else 0 for x in left]
                tot = sum(ws)
                acc[(s, lvl, tot == 0)] += w * p
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
    for ci, lk, seen in obs:
        fired = S[:, pos[ci]] >= lk
        w = w * np.where(fired, (1 - eps) if seen else eps, 0.0 if seen else 1.0)
    a = np.bincount(L[~A], w[~A], minlength=KMAX + 1)
    b = np.bincount(L[A], w[A], minlength=KMAX + 1)
    return a, b


def lump(v: Pmf) -> list[float]:
    """a pmf as the distribution-free fit's KMAX + 1 cells: P(N = 0..KMAX-1), P(N >= KMAX)"""
    v = list(v[:KMAX])
    v += [0.0] * (KMAX - len(v))
    return [*v, max(0.0, 1 - sum(v))]


def lk_matrix(xs: Iterable[tuple[list[Kind], list[Obs]]], eps: float = EPS, rule: str = "slots") -> np.ndarray:
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
        for ci, lk, seen in x.obs:
            pr = min(max(mb[(ci, lk)], 1e-9), 1 - 1e-9)
            t += math.log(pr if seen else 1 - pr)
    return t


# ---------------------------------------------------------------- markers (GameTora card events)
# one card event: (its chain link or None, [choice -> [(outcome's skills, ends the chain)]])
CardEvent = tuple[int | None, list[list[tuple[set[str], Any]]]]


@dataclass
class GameData:
    """game(): what the markers are read from"""

    cards: dict[int, dict[str, Any]]  # web_cards.json by id
    skill_names: dict[int, str]  # skill id -> name (master.mdb text_data 47)
    pool: dict[int, set[str]]  # card -> every skill its events can give
    markers: dict[int, dict[int, set[str]]]  # card -> observable link -> its marker skills
    card_events: dict[str, Any]  # card_events.json 'cards', by card id as a string


_game: list[GameData] = []  # game()'s cache


def skill_set(hs: Iterable[tuple[str, Any, Any]], skill_names: dict[int, str]) -> set[str]:
    """the skill names of GameTora hints ('hint': one skill id, else a list of them)"""
    out: set[str] = set()
    for kind, sid, _ in hs:
        for i in [sid] if kind == "hint" else sid:
            if i in skill_names:
                out.add(skill_names[i])
    return out


def card_events(gt: Any, skill_names: dict[int, str], link_of: dict[int, int]) -> dict[int, list[CardEvent]]:
    """card -> its SR/SSR events, decoded"""
    evs: collections.defaultdict[int, list[CardEvent]] = collections.defaultdict(list)
    for f in ("training_events_sr", "training_events_ssr"):
        for cid, es in gt.load(f):
            for e in es:
                chs = [[(skill_set(h, skill_names), end) for h, end in gt.outcomes(ch[1])] for ch in gt.choices(e[1])]
                evs[cid].append((link_of.get(int(e[2])), chs))
    return evs


def hinted_skills(x: Any, acc: set[str], gt: Any, skill_names: dict[int, str]) -> None:
    """add to acc every skill hinted anywhere in a nested GameTora event row"""
    if isinstance(x, list):
        if len(x) == 2 and isinstance(x[1], list) and x[1] and all(isinstance(k, int) for k in x[1]):
            acc |= skill_set(gt.hints(x[1]), skill_names)
        for y in x:
            hinted_skills(y, acc, gt, skill_names)


def skill_pools(
    gt: Any,
    evs: dict[int, list[CardEvent]],
    cards: dict[int, dict[str, Any]],
    skill_names: dict[int, str],
    cev: dict[str, Any],
) -> dict[int, set[str]]:
    """every skill each card's events can give: SR/SSR events, pal / group events, card_events.json"""
    pool: collections.defaultdict[int, set[str]] = collections.defaultdict(set)
    for cid, es in evs.items():
        for _, chs in es:
            pool[cid] |= {s for ch in chs for o, _ in ch for s in o}
    for kindf in ("friend", "group"):
        for row in gt.load(f"training_events_{kindf}"):
            acc: set[str] = set()
            hinted_skills(row[1:], acc, gt, skill_names)
            for cid, c in cards.items():
                if c["pal"] and ((kindf == "friend" and c["chara"] == row[0]) or (kindf == "group" and cid == row[0])):
                    pool[cid] |= acc
    for cid, e in cev.items():
        pool[int(cid)] |= (
            set(e.get("always", [])) | {n for n, _ in e.get("disabled", [])} | {n for n, _ in e.get("chance", [])}
        )
    return pool


def link_markers(evs: dict[int, list[CardEvent]]) -> dict[int, dict[int, set[str]]]:
    """card -> link -> skills unique to the link's single event, when every choice and outcome carries one"""
    markers: dict[int, dict[int, set[str]]] = {}
    for cid, es in evs.items():
        per: collections.defaultdict[int, list[list[list[tuple[set[str], Any]]]]] = collections.defaultdict(list)
        cnt: collections.Counter[str] = collections.Counter()  # skill -> events of the card that can give it
        for lk, chs in es:
            cnt.update({s for ch in chs for o, _ in ch for s in o})
            if lk:
                per[lk].append(chs)
        out: dict[int, set[str]] = {}
        for lk, lst in per.items():
            if len(lst) != 1 or not lst[0]:
                continue
            uniq = {s for ch in lst[0] for o, _ in ch for s in o if cnt[s] == 1}
            if uniq and all(o & uniq for ch in lst[0] for o, _ in ch):
                out[lk] = uniq
        markers[cid] = out
    return markers


def game() -> GameData:
    """support cards, skill names, per-card skill pools, observable links with their markers, and exported card events
    (chain links from master.mdb single_mode_story_data)"""
    if _game:
        return _game[0]
    gt = common.gametora()
    db = common.mdb()
    cards = common.cards()
    sk = dict(db.execute("select [index],text from text_data where category=47"))
    link_of = dict(db.execute("select story_id,show_progress_1 from single_mode_story_data where show_progress_1>0"))
    evs = card_events(gt, sk, link_of)
    cev = common.gjson("card_events.json")["cards"]
    _game.append(
        GameData(
            cards=cards,
            skill_names=sk,
            pool=skill_pools(gt, evs, cards, sk, cev),
            markers=link_markers(evs),
            card_events=cev,
        )
    )
    return _game[0]


def kind(cid: int) -> Kind | None:
    """(links, links, stop risk) for a chain card, None for a pal, group or chainless card"""
    g = game()
    c = g.cards[cid]
    if c["pal"] or c["links"] == 0:
        return None
    risk = tuple(
        sorted(
            tuple(r) for r in g.card_events.get(str(cid), {}).get("stop_risk", []) if r[0] < c["links"] and r[0] <= 4
        )
    )
    return Kind(c["links"], c["links"], risk)


def pals(r: Run) -> int:
    cards = game().cards
    return sum(1 for c, _, _ in r["deck"] if c in cards and cards[c]["pal"])


def observations(r: Run) -> ChainObservations | None:
    """a run's chain observations: (chain kinds, chain card ids, [(card index, link, seen)]), or None for a deck with an
    unknown or non-pal R card"""
    g = game()
    cards = g.cards
    deck = [c for c, _, _ in r["deck"]]
    if any(c not in cards for c in deck):
        return None
    if any(cards[c]["rarity"] == 1 and not cards[c]["pal"] for c in deck):
        return None
    hints = {h["name"]: h["src"] for h in r["hints"]}
    name = {c: common.bare(n) for c, _, n in r["deck"]}
    by_name = {name[c]: c for c in deck}
    chain = [c for c in deck if kind(c)]
    obs: list[Obs] = []
    for ci, cid in enumerate(chain):
        nm = name[cid]
        for lk, mk in g.markers.get(cid, {}).items():
            if lk > cards[cid]["links"]:
                continue
            credits = [
                [y.strip() for y in x[:-6].split(" or ")] for s in mk for x in hints.get(s, ()) if x.endswith(" event")
            ]
            credits = [c for c in credits if nm in c]
            # Unknown only when every matching credit has another possible deck-card source.
            if credits and all(any(y != nm and y in by_name and mk & g.pool[by_name[y]] for y in c) for c in credits):
                continue
            seen = bool(credits)
            if not seen and any(s in hints for s in mk) and all(hints[s] == ["Inspiration"] for s in mk if s in hints):
                continue
            obs.append(Obs(ci, lk, seen))
    return ChainObservations([kind(c) for c in chain], chain, obs)


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


def held_out(xs: list[Row], M: np.ndarray) -> dict[str, common.HeldOutComparison]:
    """5-fold cross-validation (common.fold by run id): each fold's runs are scored by forms fitted on the other four, the
    free distribution, the binomial by likelihood (n 10..40, p re-fitted), the binomial by capped moments
    (binomial_moments), the Conway-Maxwell-Poisson and the Poisson (capped mean matched), against
    Binomial(21, 0.35) (nothing fitted). Per run held-out log-likelihoods; {form: (nats, SE, z)} from
    common.compare_held_out, positive when the form predicts unseen runs better than (21, 0.35)"""
    folds = np.array([common.fold(x.run["id"]) for x in xs])
    per = {
        f: np.zeros(len(xs))
        for f in ("free", "binomial by likelihood", "binomial by moments", "Conway-Maxwell-Poisson", "Poisson", "base")
    }
    for k in range(common.K_FOLDS):
        train, test = folds != k, folds == k
        T, E = M[train], M[test]
        free = em(T)
        m, v = mean_var(free)
        bn, bp, _ = max(binom_profile(T, range(10, 41)), key=lambda x: x[2])
        nm, pm = binomial_moments(m, v)
        poisson_mean = brentq(lambda lam, m=m: mean_var(np.asarray(lump(pois_pmf(lam))))[0] - m, 0.0, 100.0)
        _, lam, nu = cmp_ll(T)
        pmfs = {
            "free": list(free),
            "binomial by likelihood": lump(binom_pmf(bn, bp)),
            "binomial by moments": lump(binom_pmf(nm, pm)),
            "Conway-Maxwell-Poisson": lump(cmp_pmf(lam, nu)),
            "Poisson": lump(pois_pmf(poisson_mean)),
            "base": lump(BIN),
        }
        for f, pmf in pmfs.items():
            per[f][test] = np.log(np.maximum(1e-300, E @ np.asarray(pmf)))
    return {f: common.compare_held_out(per[f], per["base"]) for f in per if f != "base"}


def binom_mle(xs: list[Row]) -> BinomialFit:
    """the Binomial(n, p) grid MLE (n 6..40, p 0.200..0.995 step 0.005), exact over the chain states: (loglik, n, p)"""
    ab = [run_levels(x.kinds, x.obs) for x in xs]
    A = np.array([a for a, _ in ab])
    B = np.array([b for _, b in ab])
    grid = [(n, k / 200) for n in range(6, 41) for k in range(40, 200)]
    PM = np.array([binom_pmf(n, p, KMAX + 1) for n, p in grid])
    TL = np.array([survival(pm) for pm in PM])
    V = A @ PM.T + B @ TL.T
    lls = np.where((V <= 0).any(0), -1e18, np.log(np.maximum(V, 1e-300)).sum(0))
    return BinomialFit(*max((float(v), n, p) for v, (n, p) in zip(lls, grid)))


# ---------------------------------------------------------------- values
# example decks: chain kinds, and the cards shown (one of identical cards)
SSR3: Kind = Kind(3, 3, ())  # a plain 3-link SSR
ALLOC: list[tuple[str, list[Kind], tuple[str, ...]]] = [
    ("two 3-link SSRs", [SSR3] * 2, ("",)),
    ("3-link SSR + 2-link SR", [SSR3, Kind(2, 2, ())], (" SSR", " SR")),
    ("three 3-link SSRs", [SSR3] * 3, ("",)),
    ("four 3-link SSRs", [SSR3] * 4, ("",)),
]


def V(name: str, got: float | int | str | None, **kw: str) -> common.Value:
    return common.Value(SEC, name, got, **kw)


def derived() -> list[common.Value | common.Finding]:
    """the numbers that need no run records: the binomial's moments, the worked example, the allocation tables"""
    out: list[common.Value | common.Finding] = []
    m, v = CN * CP, CN * CP * (1 - CP)
    out += [V("Binomial(21, 0.35): mean", m), V("Binomial(21, 0.35): variance", v, note=f"{v:.4f}")]
    S2, w = final_w((SSR3, SSR3), BIN)
    both = float(w[(S2 == 3).all(1)].sum())
    out.append(V("two 3-link cards: P(N >= 6) = P(both chains complete)", both, note=f"P(N >= 6) {sum(BIN[6:]):.4f}"))
    for deck, kinds, whos in ALLOC:
        for rule in ("slots", "cards"):
            r = reach(kinds, BIN, rule)
            for ci, who in enumerate(whos):
                for k in range(1, kinds[ci][0] + 1):
                    out.append(V(f"{deck}{who}, {rule} rule: P(reach link {k})", r[(ci, k)]))
    for rule in ("slots", "cards"):
        S3, w = final_w((SSR3, SSR3), [0, 0, 0, 1.0], rule)
        out.append(
            V(
                f"two 3-link cards, exactly 3 events, {rule} rule: P(final state (3, 0))",
                float(w[(S3[:, 0] == 3) & (S3[:, 1] == 0)].sum()),
            )
        )
    out.append(
        V(
            "distribution-free fit: parameters more than the binomial",
            KMAX - 2,
            note=f"P(N = 0..{KMAX - 1}), P(N >= {KMAX}): {KMAX} free",
        )
    )
    return out


def fit_claims() -> list[common.Value | common.Finding]:
    """the fit of N (every run, 0-5 pals) and its per-scenario split"""
    xs = dataset(ALL)
    note = f"every run, 0-5 pals; {dropped(ALL)} decks with a non-pal R card left out"
    M = lk_matrix([(x.kinds, x.obs) for x in xs])
    pooled = em(M)
    llp = ll(pooled, M)
    mn, vr = mean_var(pooled)
    l21 = ll(lump(BIN), M)
    l15 = ll(lump(binom_pmf(15, 0.48)), M)
    lp = ll(lump(POIS), M)
    prof = binom_profile(M)
    bn, bp, lb = max(prof, key=lambda x: x[2])
    ho = held_out(xs, M)
    l_cmp = cmp_ll(M)[0]
    out: list[common.Value | common.Finding] = [
        V("fit of N: runs", len(xs), note=note),
        V("distribution-free: mean", mn, note=f"E[min(N, {KMAX})]; uncapped moments are not identified"),
        V("distribution-free: variance", vr, note=f"Var(min(N, {KMAX}))"),
        V("Binomial(21, 0.35): nats behind distribution-free", llp - l21, note=f"LL {l21:.1f} vs {llp:.1f}"),
        V("Binomial(15, 0.48): nats behind distribution-free", llp - l15),
        V("Poisson(7.1): nats behind distribution-free", llp - lp),
        V("best binomial (n free): nats better than (21, 0.35)", lb - l21),
        V("best binomial n free: n", bn),
        V("best binomial n free: p", bp, note="p re-fitted at each n"),
        V("best binomial n free: nats behind distribution-free", llp - lb),
        V("best binomial n free: LR p", float(chi2.sf(2 * (llp - lb), KMAX - 2))),
        V("binomial n: 95% profile interval low", min(n_ for n_, _, l_ in prof if lb - l_ <= CHI1)),
        V("binomial n: 95% profile interval high", max(n_ for n_, _, l_ in prof if lb - l_ <= CHI1)),
        V("binomial n = 21, p re-fitted: nats below the best n", lb - {n_: l_ for n_, _, l_ in prof}[21]),
        V("binomial mean n p at the best n", bn * bp),
        V("distribution-free: variance / mean", vr / mn, note=f"for min(N, {KMAX}), not N"),
        *[
            V(f"held out: {form} over Binomial(21, 0.35): {w}", x)
            for form, t_ in ho.items()
            for w, x in zip(("nats", "SE", "z"), t_)
        ],
        V("Conway-Maxwell-Poisson: nats behind distribution-free", llp - l_cmp),
        V("Conway-Maxwell-Poisson: LR p", float(chi2.sf(2 * (llp - l_cmp), KMAX - 2))),
        V("best binomial n free over Conway-Maxwell-Poisson: nats", lb - l_cmp, note="two parameters each"),
        *candidate_claims(xs, M),
        *property_claims(xs, M),
    ]
    by: collections.defaultdict[int, list[int]] = collections.defaultdict(list)  # scenario -> row indices
    for i, x in enumerate(xs):
        by[x.run["scenario"]].append(i)
    sep = {sc: em(M[ix]) for sc, ix in by.items() if len(ix) >= 30}
    lls = sum(ll(sep[sc], M[by[sc]]) for sc in sep) + sum(ll(pooled, M[ix]) for sc, ix in by.items() if sc not in sep)
    for sc in sorted(set(by) | {1, 2, 3}, key=[2, 3, 1, 4].index):
        out.append(
            V(
                f"per scenario: mean, {common.SCEN[sc]}",
                mean_var(sep[sc])[0] if sc in sep else None,
                note=f"{len(by[sc])} runs; E[min(N, {KMAX})]",
            )
        )
    out.append(V("per scenario: log-likelihood gain", lls - llp))
    out.append(V("per scenario: extra parameters", (len(sep) - 1) * KMAX, note=f"{len(sep)} scenarios with 30+ runs"))
    MISC["fit"] = {
        "llp": llp,
        "lls": {
            nm: ll(lump(v), M)
            for nm, v in (
                ("Binomial(21, 0.35)", BIN),
                ("Binomial(15, 0.48)", binom_pmf(15, 0.48)),
                ("Poisson(7.1)", POIS),
            )
        },
        "runs": len(xs),
        "by_pals": dict(sorted(collections.Counter(pals(x.run) for x in xs).items())),
    }
    return out


def property_claims(xs: list[Row], M: np.ndarray) -> list[common.Value | common.Finding]:
    """N on the decks with Sasami Anshinzawa [This Might Sting!] and on the Inari One careers against the other runs: each
    side's free-fit mean, the gain of separate free fits (KMAX parameters), and of p at n = CHAIN_N re-fitted on each
    side (one parameter); then Binomial(CHAIN_N, CHAIN_P) on the runs with neither, and on the general collection"""
    out: list[common.Value | common.Finding] = []
    llp = ll(em(M), M)
    _, l_one = best_binomial_p(M, CN)
    sides = {
        "Sasami Anshinzawa's card": np.array([any(c[0] == ANSHINZAWA for c in x.run["deck"]) for x in xs]),
        "Inari One": np.array([x.run["chara"] == INARI for x in xs]),
    }
    for nm, g in sides.items():
        (p_in, l_in), (p_out, l_out) = best_binomial_p(M[g], CN), best_binomial_p(M[~g], CN)
        free_gain = ll(em(M[g]), M[g]) + ll(em(M[~g]), M[~g]) - llp
        out += [
            V(f"{nm}: runs", int(g.sum())),
            V(f"{nm}: mean of N", mean_var(em(M[g]))[0], note=f"E[min(N, {KMAX})]"),
            V(f"{nm}: mean of N, the other runs", mean_var(em(M[~g]))[0], note=f"E[min(N, {KMAX})]"),
            V(f"{nm}: separate free fits, gain", free_gain, note=f"{KMAX} parameters"),
            V(f"{nm}: separate free fits, LR p", float(chi2.sf(2 * free_gain, KMAX))),
            V(f"{nm}: p at n = {CN}", p_in),
            V(f"{nm}: p at n = {CN}, the other runs", p_out),
            V(f"{nm}: p at n = {CN} on each side, LR p", float(chi2.sf(2 * (l_in + l_out - l_one), 1))),
        ]
    for nm, g in (
        ("runs with neither", ~sides["Sasami Anshinzawa's card"] & ~sides["Inari One"]),
        ("general collection", np.array([x.run["batch"] in GENERAL_BATCHES for x in xs])),
    ):
        gap = ll(em(M[g]), M[g]) - ll(lump(BIN), M[g])
        out.append(V(f"Binomial(21, 0.35) LR p, {nm}", float(chi2.sf(2 * gap, KMAX - 2)), note=f"{g.sum()} runs"))
    return out


def candidate_claims(xs: list[Row], M: np.ndarray) -> list[common.Value | common.Finding]:
    """the candidate Binomial(18, 0.405) against Binomial(CHAIN_N, CHAIN_P): in-sample on every run; the binomial fitted on
    the general collection (n by profile, p re-fitted) scored on the other runs, per run paired (common.compare_held_out).
    The 5-fold comparison with refit is held_out()'s binomial by likelihood"""
    cn, cp = CANDIDATE
    base = np.log(np.maximum(1e-300, M @ np.asarray(lump(BIN))))
    cand = np.log(np.maximum(1e-300, M @ np.asarray(lump(binom_pmf(cn, cp)))))
    gen = np.array([x.run["batch"] in GENERAL_BATCHES for x in xs])
    gn, gp, _ = max(binom_profile(M[gen]), key=lambda x: x[2])
    fit = np.log(np.maximum(1e-300, M[~gen] @ np.asarray(lump(binom_pmf(gn, gp)))))
    out = common.compare_held_out(fit, base[~gen])
    return [
        V(f"candidate Binomial({cn}, {cp}): mean", cn * cp),
        V(f"candidate Binomial({cn}, {cp}): variance", cn * cp * (1 - cp)),
        V(f"candidate Binomial({cn}, {cp}) over Binomial(21, 0.35), in-sample: nats", float((cand - base).sum())),
        V("fitted on the general collection: runs", int(gen.sum())),
        V("fitted on the general collection: n", gn),
        V("fitted on the general collection: p", gp),
        V("fitted on the general collection, scored on the other runs: runs", int((~gen).sum())),
        *[
            V(f"fitted on the general collection over Binomial(21, 0.35), scored on the other runs: {w}", x)
            for w, x in zip(("nats", "SE", "z"), out)
        ],
    ]


# pal count -> (runs, the Binomial MLE on them)
PalCountFits = dict[int, tuple[int, BinomialFit]]


def pal_count_claims() -> tuple[list[common.Value | common.Finding], PalCountFits]:
    """N per pal count: the binomial grid MLE on each pal count's runs (every run, 1-4 pals)"""
    out: list[common.Value | common.Finding] = []
    res: PalCountFits = {}
    for npal in (1, 2, 3, 4):
        xs = dataset({npal})
        res[npal] = (len(xs), binom_mle(xs))
    out.append(V("per pal count: runs (1-4 pals)", sum(n for n, _ in res.values()), note="every run"))
    for npal in (1, 2, 3, 4):
        n, (_, bn, bp) = res[npal]
        out.append(V(f"per pal count: mean, {npal} pals", bn * bp, note=f"{n} runs; Binomial MLE n {bn}, p {bp:.2f}"))
    vs = [bn * bp * (1 - bp) for _, (_, bn, bp) in res.values()]
    out += [V("per pal count: variance, lowest", min(vs)), V("per pal count: variance, highest", max(vs))]
    return out, res


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
    poisson: float  # predicted, Poisson(7.1)
    links: int  # card-links observed
    low: float  # the observed rate's run-clustered 95% interval
    high: float


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


def final_claims() -> tuple[list[common.Value | common.Finding], LinkTables]:
    """the final-link table (each observed rate with its run-clustered 95% interval against Binomial(21, 0.35)) and the
    early-link shortfall (every run)"""
    out: list[common.Value | common.Finding] = []
    tab: dict[int, FinalLinks] = {}
    for npal in (5, 4, 3, 2, 1, 0):
        xs = dataset({npal})
        o = b = p = 0.0
        n = 0
        rows: list[tuple[float, float, int]] = []
        hers = [0.0, 0.0, 0]  # decks with Sasami Anshinzawa's card: observed, predicted, card-links
        for r, kinds, _, obs in xs:
            mb, mp = reach(kinds, BIN), reach(kinds, POIS)
            her = any(c[0] == ANSHINZAWA for c in r["deck"])
            for ci, lk, seen in obs:
                if lk == kinds[ci][0]:
                    o += seen
                    b += mb[(ci, lk)]
                    p += mp[(ci, lk)]
                    n += 1
                    rows.append((seen, mb[(ci, lk)], r["id"]))
                    if her:
                        hers = [hers[0] + seen, hers[1] + mb[(ci, lk)], hers[2] + 1]
        lo, hi = obs_ci(rows)
        tab[npal] = FinalLinks(len(xs), o / n, b / n, p / n, n, lo, hi)
        out += [
            V(f"final-link table, {npal} pals: runs", len(xs)),
            V(f"final-link table, {npal} pals: observed", o / n, note=f"{n} card-links"),
            V(f"final-link table, {npal} pals: observed 95% low", lo),
            V(f"final-link table, {npal} pals: observed 95% high", hi),
            V(f"final-link table, {npal} pals: Binomial(21, 0.35)", b / n),
            V(f"final-link table, {npal} pals: Poisson(7.1)", p / n),
        ]
        if hers[2]:
            out += [
                V(f"final-link table, {npal} pals, Sasami Anshinzawa's card: card-links", hers[2]),
                V(f"final-link table, {npal} pals, Sasami Anshinzawa's card: observed", hers[0] / hers[2]),
                V(f"final-link table, {npal} pals, Sasami Anshinzawa's card: Binomial(21, 0.35)", hers[1] / hers[2]),
            ]
    out.append(
        V(
            "final-link table: runs in all",
            sum(t.runs for t in tab.values()),
            note=f"every run; {dropped(ALL)} decks with a non-pal R card left out",
        )
    )
    # early links (1 and 2), per run as a cluster
    early: dict[int, EarlyLinks] = {}
    for npal in (4, 3, 2, 1, 0):
        per: list[tuple[float, float, int]] = []  # per run: (observed, predicted, links)
        for _, kinds, _, obs in dataset({npal}):
            mb = reach(kinds, BIN)
            o = e = 0.0
            k = 0
            for ci, lk, seen in obs:
                if lk == kinds[ci][0]:
                    continue
                o += seen
                e += mb[(ci, lk)]
                k += 1
            if k:
                per.append((o, e, k))
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
    return out, LinkTables(tab, early)


def allowance_claims() -> list[common.Value | common.Finding]:
    """the marker allowance: links fire in order, so a card's later link seen means its earlier links fired; the share of
    those earlier links whose marker is missing measures how often a fired link goes unseen (every run; its 95% interval
    run-clustered)"""
    imp = un = 0
    rows: list[tuple[float, float, int]] = []
    for x in dataset(ALL):
        per: collections.defaultdict[int, dict[int, bool]] = collections.defaultdict(dict)  # card -> link -> seen
        for ci, lk, seen in x.obs:
            per[ci][lk] = seen
        for d in per.values():
            for lk, seen in d.items():
                if any(l2 > lk and s2 for l2, s2 in d.items()):
                    imp += 1
                    un += not seen
                    rows.append((int(not seen), 1, x.run["id"]))
    lo, hi = obs_ci(rows)  # one run gives several such links: the run-clustered interval of the rate
    return [
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


def final_by_card() -> FinalByCard:
    """final-link reach by card (every run) against the chain model on each run's deck: the pooled rate and the spread
    between cards with 60+ observations (a joint Wald test that every card's observed - predicted is zero, one df per
    card, its covariance run-clustered by common.cluster_chi2: the chain cards of one run share its N). Each card's z
    (binomial variance; one observation per run) only ranks the cards"""
    # card -> [observed, predicted, observations, binomial variance]
    per: collections.defaultdict[int, list[float]] = collections.defaultdict(lambda: [0, 0.0, 0, 0.0])
    res: list[tuple[int, float, int]] = []
    for r, kinds, chain, obs in dataset(ALL):
        mb = reach(kinds)
        for ci, lk, seen in obs:
            if lk != kinds[ci][0]:
                continue
            a = per[chain[ci]]
            p_ = mb[(ci, lk)]
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
    """some card's link unseen while a later link of it is seen (impossible without the allowance)"""
    per: collections.defaultdict[int, dict[int, bool]] = collections.defaultdict(dict)
    for ci, lk, seen in obs:
        per[ci][lk] = seen
    return any(not sn and any(l2 > lk and s2 for l2, s2 in d.items()) for d in per.values() for lk, sn in d.items())


def allowance_sens() -> AllowanceSensitivity:
    """the best p at n = CHAIN_N under several allowances, and with none on the runs showing no contradiction"""
    xs = dataset(ALL)
    best: dict[float, float] = {}
    for e in (0.01, 0.02, 0.03, 0.045):
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
    runs = 0
    for npal in (5, 4, 3, 2, 1, 0):
        xs = dataset({npal})
        runs += len(xs)
        for rule in ("slots", "cards"):
            tot[rule] += obs_ll(xs, BIN, rule)
            o = p = 0.0
            n = 0
            rows: list[tuple[float, float, int]] = []
            for r, kinds, _, obs in xs:
                mb = reach(kinds, BIN, rule)
                for ci, lk, seen in obs:
                    if lk == kinds[ci][0]:
                        o += seen
                        p += mb[(ci, lk)]
                        n += 1
                        rows.append((seen, mb[(ci, lk)], r["id"]))
            fin[(npal, rule)] = RuleFinal(
                o / n, p / n, n, *obs_ci(rows)
            )  # the observed rate's interval against this rule
    out: list[common.Value | common.Finding] = [
        V("allocation: runs", runs, note="every run, 0-5 pals"),
        V("allocation: total LL, slot rule", tot["slots"]),
        V("allocation: total LL, card rule", tot["cards"]),
    ]
    for npal in (2, 0):
        out += [
            V(
                f"allocation: final links, {npal} pals, observed",
                fin[(npal, "slots")].observed,
                note=f"{fin[(npal, 'slots')].links} card-links",
            ),
            V(f"allocation: final links, {npal} pals, slot rule", fin[(npal, "slots")].predicted),
            V(f"allocation: final links, {npal} pals, card rule", fin[(npal, "cards")].predicted),
        ]
    out.append(V("allocation: slot over card, nats", tot["slots"] - tot["cards"]))
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


def miss_claims() -> list[common.Value | common.Finding]:
    """the known misses: two-chain decks, chains never started, and the activation parameter"""
    return two_chain_claims() + never_started_claims() + activation_claims()


def two_chain_claims() -> list[common.Value | common.Finding]:
    """two-chain decks: two plain 3-link SSRs with four pals, both final links observable"""
    clean: list[bool] = []
    for _, kinds, _, obs in dataset({4}):
        if len(kinds) != 2 or any(k != SSR3 for k in kinds):
            continue
        fz = {ci: seen for ci, lk, seen in obs if lk == 3}
        if len(fz) == 2:
            clean.append(sum(fz.values()) == 2)
    k = sum(clean)
    n = len(clean)
    lo, hi = common.wilson(k, n)
    MISC["two-chain"] = (k, n, lo, hi)
    return [
        V("two-chain decks: both chains finish", k, note="every run"),
        V("two-chain decks: runs", n),
        V("two-chain decks: share", k / n),
        V("two-chain decks: 95% low", lo),
        V("two-chain decks: 95% high", hi),
        V("two-chain decks: predicted, P(N >= 6)", sum(BIN[6:])),
    ]


def never_started_claims() -> list[common.Value | common.Finding]:
    """chains never started: link 1 not reached, by chain cards in the deck"""
    out: list[common.Value | common.Finding] = []
    # chain cards -> [link 1 seen, predicted reach, observations]
    rows: collections.defaultdict[int, list[float]] = collections.defaultdict(lambda: [0, 0.0, 0])
    nrows: collections.defaultdict[int, list[tuple[float, float, int]]] = collections.defaultdict(list)
    for r, kinds, _, obs in dataset(ALL):
        mb = reach(kinds, BIN)
        for ci, lk, seen in obs:
            if lk == 1:
                a = rows[len(kinds)]
                a[0] += seen
                a[1] += mb[(ci, 1)]
                a[2] += 1
                nrows[len(kinds)].append((1 - seen, 1 - mb[(ci, 1)], r["id"]))
    MISC["never rows"] = dict(rows)
    for nc in (5, 6):  # several chain cards per run: the observed rate's interval is run-clustered
        o, p_, n = rows[nc]
        lo, hi = obs_ci(nrows[nc])
        MISC[("never", nc)] = NeverStarted(n - o, n, 1 - p_ / n, lo, hi)
        out += [
            V(f"never started, {nc} chain cards: observed", 1 - o / n, note=f"link 1 not reached, {n} card-links"),
            V(f"never started, {nc} chain cards: observed 95% low", lo),
            V(f"never started, {nc} chain cards: observed 95% high", hi),
            V(f"never started, {nc} chain cards: model", 1 - p_ / n),
        ]
    return out


def activation_claims() -> list[common.Value | common.Finding]:
    """the activation parameter, jointly with N ~ Binomial(CHAIN_N, p), exact over the chain process, every deck"""
    xs = dataset(ALL)

    def LL(p_: float, a: float) -> float:
        pm = np.array(binom_pmf(CN, p_, KMAX + 1))
        tl = survival(pm)
        t = 0.0
        for x in xs:
            A, B = run_levels(with_activation(x.kinds, a), x.obs)
            t += math.log(max(float(A @ pm + B @ tl), 1e-300))
        return t

    l1 = max(LL(p_, 1.0) for p_ in (0.33, 0.34, 0.35, 0.36, 0.37))
    lj, pj, aj = max((LL(p_, a), p_, a) for a in (0.97, 0.94, 0.90, 0.86) for p_ in (0.35, 0.37, 0.39, 0.41))
    return [
        V("activation, corrected reading, joint with N: gain, nats", lj - l1, note=f"every run; p {pj}"),
        V("activation, corrected reading, joint with N: a", aj),
    ]


def claims() -> list[common.Value | common.Finding]:
    out = derived()
    if not common.have_runs():
        return out
    fc = fit_claims()
    pc, res = pal_count_claims()
    flc, fl = final_claims()
    ac, al = alloc_claims()
    out += fc + allowance_claims() + pc + flc + ac + miss_claims()
    sens = allowance_sens()
    gap = next(
        v.got
        for v in out
        if isinstance(v, common.Value) and v.name == "Binomial(21, 0.35): nats behind distribution-free"
    )
    out.append(V("binomial(21, 0.35) LR p", float(chi2.sf(2 * gap, KMAX - 2)), note=f"{KMAX - 2} df"))
    fb = final_by_card()
    out += [
        V("final links by card: pooled observed", fb.obs, note=f"{fb.n:,} card-links, every run"),
        V("final links by card: pooled model", fb.model),
        V("final links by card: cards", fb.cards),
        V("final links by card: chi2", fb.chi2),
    ]
    MISC["final_by_card"] = fb
    MISC["pal counts"] = res
    MISC["alloc"] = al
    out += [
        V("allowance 0: runs without a contradiction", sens.clean_runs),
        V("allowance sensitivity: lowest best p at n = 21", min(sens.best_p.values())),
        V("allowance sensitivity: highest best p at n = 21", max(sens.best_p.values())),
    ]
    g: dict[str, Any] = {c.name: c.got for c in out if isinstance(c, common.Value)}
    return out + fit_findings(g) + allowance_findings(g, sens) + link_findings(g, res, fl, al) + miss_findings(g)


# ---------------------------------------------------------------- findings: the section's conclusions, each tested on
# the numbers above (g: value name -> value)
def F(*a: Any) -> common.Finding:
    return common.finding(SEC, *a)


def fit_findings(g: dict[str, Any]) -> list[common.Finding]:
    """the form of N"""
    fs: list[common.Finding] = []
    gap = g["Binomial(21, 0.35): nats behind distribution-free"]
    df = g["distribution-free fit: parameters more than the binomial"]
    lrp = g["binomial(21, 0.35) LR p"]
    fs.append(
        F(
            f"Known miss: Binomial({CN}, {CP}) is rejected against a free distribution of N (likelihood ratio on its {df:.0f} "
            "fewer parameters, p < 0.05)",
            lrp < 0.05,
            f"{gap:.1f} nats behind distribution-free, LR p = {lrp:.2g}; best binomial (n free) "
            f"{g['best binomial n free: nats behind distribution-free']:.1f} nats behind, p "
            f"{g['best binomial n free: LR p']:.3f}; Binomial({CN}, {CP}) on the runs with neither Sasami Anshinzawa's card "
            f"nor Inari One p {g['Binomial(21, 0.35) LR p, runs with neither']:.3f}, on the general collection p "
            f"{g['Binomial(21, 0.35) LR p, general collection']:.3f}",
        )
    )
    a_, i_ = "Sasami Anshinzawa's card", "Inari One"
    fs.append(
        F(
            "N depends on the deck and the trainee where the model has it fixed: decks with Sasami Anshinzawa "
            "[This Might Sting!] reach more chain events than the other runs and Inari One's careers fewer (p at n = 21 "
            "re-fitted on each side, likelihood ratio on one parameter, p < 0.05)",
            g[f"{a_}: p at n = {CN}"] > g[f"{a_}: p at n = {CN}, the other runs"]
            and g[f"{a_}: p at n = {CN} on each side, LR p"] < 0.05
            and g[f"{i_}: p at n = {CN}"] < g[f"{i_}: p at n = {CN}, the other runs"]
            and g[f"{i_}: p at n = {CN} on each side, LR p"] < 0.05,
            "; ".join(
                f"{nm}: mean of N {g[f'{nm}: mean of N']:.2f} vs {g[f'{nm}: mean of N, the other runs']:.2f}, p "
                f"{g[f'{nm}: p at n = {CN}']:.3f} vs {g[f'{nm}: p at n = {CN}, the other runs']:.3f} "
                f"(LR p {g[f'{nm}: p at n = {CN} on each side, LR p']:.2g})"
                for nm in (a_, i_)
            ),
        )
    )
    pg_ = g["Poisson(7.1): nats behind distribution-free"]
    dfp = KMAX - 1
    fs.append(
        F(
            f"Poisson(7.1) is rejected against a free distribution of N (likelihood ratio on its {dfp} fewer parameters, p < 0.05)",
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
    hz = {
        k.split(": ")[1].replace(" over Binomial(21, 0.35)", ""): v
        for k, v in g.items()
        if k.startswith("held out:") and k.endswith(": z")
    }
    fs.append(
        F(
            "No alternative form predicts unseen runs better than Binomial(21, 0.35): 5-fold cross-validation, per-run held-out "
            "log-likelihoods compared pairwise, every z at most 1.96 (the free distribution, the binomial fitted by likelihood or "
            "by moments, Conway-Maxwell-Poisson, Poisson)",
            max(hz.values()) <= 1.96,
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
    pg = g["per scenario: log-likelihood gain"]
    dfs = g["per scenario: extra parameters"]
    fs.append(
        F(
            f"One N for every scenario: per-scenario distributions gain nothing significant (likelihood ratio on {dfs:.0f} parameters, p > 0.05)",
            chi2.sf(2 * pg, dfs) > 0.05,
            f"+{pg:.1f} nats, LR p = {chi2.sf(2 * pg, dfs):.2f}",
        )
    )
    return fs


def allowance_findings(g: dict[str, Any], sens: AllowanceSensitivity) -> list[common.Finding]:
    """the marker allowance: measured, and what it does to the fit"""
    sh = g["allowance: share"]
    lo, hi = g["allowance: 95% low"], g["allowance: 95% high"]
    ps = list(sens.best_p.values())
    lo_, hi_ = round(CP - 0.01, 2), round(CP + 0.01, 2)  # the adopted p +- 0.01
    return [
        F(
            f"The {EPS} marker allowance lies inside the 95% interval of the measured rate of fired links whose marker is missing",
            lo <= EPS <= hi,
            f"{g['allowance: marker not shown']:.0f} of {g['allowance: earlier links implied fired']:.0f} = {sh:.3f} [{lo:.3f}, {hi:.3f}]",
        ),
        F(
            f"The fit of N does not depend on the allowance: over allowances 0-0.045 the best p at n = {CN} stays within {lo_}-{hi_}",
            min(ps) >= lo_ and max(ps) <= hi_,
            ", ".join(f"eps {k}: {v:.3f}" for k, v in sens.best_p.items())
            + f" (eps 0.0 on the {sens.clean_runs} runs without a contradiction)",
        ),
    ]


def placement(prediction: float, low: float, high: float) -> str:
    """where a prediction lies against an observed interval"""
    return "inside" if low <= prediction <= high else "above" if prediction > high else "below"


def link_findings(g: dict[str, Any], res: PalCountFits, fl: LinkTables, al: Allocation) -> list[common.Finding]:
    """final links by pal count, pal events, and the allocation rule"""
    fs: list[common.Finding] = []
    pos = {npal: placement(t.binomial, t.low, t.high) for npal, t in fl.final.items()}
    fs.append(
        F(
            "Final links: the Binomial(21, 0.35) prediction lies inside the observed 95% interval at 3, 1 and 0 pals, above it "
            "at 5 and 2 pals and below it at 4 pals",
            pos == {5: "above", 4: "below", 3: "inside", 2: "above", 1: "inside", 0: "inside"},
            "; ".join(
                f"{k} pals {fl.final[k].observed:.3f} [{fl.final[k].low:.3f}, {fl.final[k].high:.3f}] vs {fl.final[k].binomial:.3f} ({c})"
                for k, c in sorted(pos.items(), reverse=True)
            )
            + "; decks with Sasami Anshinzawa's card: "
            + ", ".join(
                f"{k} pals {g[h + 'observed']:.3f} vs {g[h + 'Binomial(21, 0.35)']:.3f} ({g[h + 'card-links']:.0f} links)"
                for k, h in ((k, f"final-link table, {k} pals, Sasami Anshinzawa's card: ") for k in (5, 4, 3, 2))
                if h + "observed" in g
            ),
        )
    )
    means = {k: fit.n * fit.p for k, (_, fit) in res.items()}
    fs.append(
        F(
            "Pal events do not consume chain events: the mean of N fitted at 4 pals is at least that at 1 pal",
            means[4] >= means[1],
            ", ".join(f"{k} pals {v:.2f}" for k, v in sorted(means.items())),
        )
    )
    tl = al.total
    fs.append(
        F(
            "The slot rule beats the card rule on total log-likelihood by more than 1.92 nats",
            g["allocation: slot over card, nats"] > 1.92,
            f"slots {tl['slots']:.1f}, cards {tl['cards']:.1f}: {tl['slots'] - tl['cards']:+.1f} nats",
        )
    )

    def where(npal: int, rule: str) -> tuple[str, float, str]:
        x = al.final[(npal, rule)]
        return (
            placement(x.predicted, x.low, x.high),
            x.predicted,
            f"{npal} pals obs {x.observed:.3f} [{x.low:.3f}, {x.high:.3f}], {rule} {x.predicted:.3f}",
        )

    w = {(k, r): where(k, r) for k in (0, 2) for r in ("slots", "cards")}
    fs.append(
        F(
            "Final links in crowded decks: at 0 pals the slot rule lies inside the observed interval and the card rule outside; at 2 pals "
            "both lie above it, the card rule further",
            w[(0, "slots")][0] == "inside"
            and w[(0, "cards")][0] != "inside"
            and w[(2, "slots")][0] == "above"
            and w[(2, "cards")][0] == "above"
            and w[(2, "cards")][1] > w[(2, "slots")][1],
            "; ".join(v[2] for v in w.values()),
        )
    )
    return fs


def miss_findings(g: dict[str, Any]) -> list[common.Finding]:
    """the known misses and the activation parameter"""
    fs: list[common.Finding] = []
    l_, h_ = g["two-chain decks: 95% low"], g["two-chain decks: 95% high"]
    pr = g["two-chain decks: predicted, P(N >= 6)"]
    fs.append(
        F(
            "Two-chain decks finish both chains as predicted (prediction within the observed interval)",
            l_ <= pr <= h_,
            f"{g['two-chain decks: share']:.3f} [{l_:.3f}, {h_:.3f}] vs {pr:.3f}",
        )
    )
    zs = {k: g[f"early-link shortfall, {k} pals: z"] for k in (4, 3, 2, 1, 0)}
    fs.append(
        F(
            "Known miss: early links run short of the model (z > 2) at exactly 3, 2, 1 and 0 pals",
            {k for k, v in zs.items() if v > 2} == {3, 2, 1, 0},
            ", ".join(f"{k} pals z {v:.1f}" for k, v in zs.items()),
        )
    )
    nv = {nc: MISC[("never", nc)] for nc in (5, 6)}
    fs.append(
        F(
            "Known miss: more chains never start than predicted with five or six chain cards (model below the observed interval)",
            all(x.low > x.model for x in nv.values()),
            ", ".join(
                f"{nc} cards {x.not_started / x.links:.2f} [{x.low:.2f}, {x.high:.2f}] vs {x.model:.2f}"
                for nc, x in nv.items()
            ),
        )
    )
    fb: FinalByCard = MISC["final_by_card"]
    fs.append(
        F(
            "Final links fit when pooled, but cards differ beyond the model: pooled within 0.01, the spread between cards far "
            "beyond chance (p < 0.001)",
            abs(fb.obs - fb.model) < 0.01 and fb.p < 0.001,
            f"pooled {fb.obs:.3f} vs {fb.model:.3f}; chi2 {fb.chi2:.0f} on {fb.cards} cards, p = {fb.p:.1g}; lowest "
            + ", ".join(f"{fb.name(c)} {fb.rate[c][0]:.2f}/{fb.rate[c][1]:.2f}" for c, _ in fb.low)
            + "; highest "
            + ", ".join(f"{fb.name(c)} {fb.rate[c][0]:.2f}/{fb.rate[c][1]:.2f}" for c, _ in fb.high),
        )
    )
    ga = g["activation, corrected reading, joint with N: gain, nats"]
    fs.append(
        F(
            "A per-card activation parameter is rejected: fitted jointly with N, it gains under 1.92 nats",
            ga < 1.92,
            f"+{ga:.1f} nats at a = {g['activation, corrected reading, joint with N: a']:.2f}",
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
    k, n, lo, hi = MISC["two-chain"]
    print(
        f"two plain 3-link SSRs, 4 pals: both finished {k}/{n} = {k / n:.3f} ({lo:.3f}-{hi:.3f}); "
        f"model P(N >= 6) {sum(BIN[6:]):.3f}, with the {EPS:.0%} unseen allowance on each final link {sum(BIN[6:]) * (1 - EPS) ** 2:.3f}"
    )
    print(
        "link 1 not reached, observed / Binomial(21, 0.35), by chain cards: "
        + ", ".join(
            f"{c}: {1 - o / n:.2f} / {1 - p / n:.2f} (n {n})" for c, (o, p, n) in sorted(MISC["never rows"].items())
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
    print(
        "N per pal count: "
        + ", ".join(
            f"{npal} pals {n} runs Binomial({bn}, {bp:.2f}) mean {bn * bp:.2f} variance {bn * bp * (1 - bp):.2f}"
            for npal, (n, (_, bn, bp)) in MISC["pal counts"].items()
        )
    )


if __name__ == "__main__":
    common.main(claims)
    detail()
