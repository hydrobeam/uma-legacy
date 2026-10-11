"""Section 7 of docs/RESEARCH.md: chain events.

Reproduces the derived numbers (Binomial(21, 0.35), P(N >= 6), the exact allocation tables, the worked example) and
the fits on the run records (research/data/runs.jsonl, not included; Independent Training runs from uma-it).

Markers. A card's chain progress is read from "marker" skills of its link events (GameTora card events through
gametora.py, chain links from master.mdb single_mode_story_data.show_progress_1) credited to "<character> event" in a
run's hint list. A link is observable only if every choice and every outcome of its event carries a skill appearing
in no other event of the card. A credit "A or B event" naming the card counts as seen, unless B is another deck card
whose events can give the marker (then the link is unknown); a marker listed with "Inspiration" as its only source is
unknown. Decks with an unknown card or a non-pal R card are left out.

Data. Every analysis runs on every run record, selected only by pal count
where the analysis is by pal count:
  - the fit of N (0-5 pals): a distribution-free fit (EM over P(N = 0..15), P(N >= 16)) against Binomial(21, 0.35),
    Binomial(15, 0.48), Poisson(7.1) and the binomial's profile over n (p re-fitted at each n), each by likelihood ratio
    (Binomial(21, 0.35) with 14 fewer parameters, Poisson with 15, n and p free with 1 more than (21, 0.35)); the same
    fit per scenario (scenarios with 30+ runs) against one N for all;
  - N per pal count (1-4 pals): the binomial grid MLE on each pal count's runs;
  - the marker allowance: the share of links implied fired (a later link of the card shows) whose marker is missing,
    and the best p at n = 21 under allowances 0.01-0.045 and with none on the runs without such a contradiction;
  - final links by pal count against Binomial(21, 0.35) and Poisson(7.1);
  - final links by card against the model, the spread between cards with 60+ observations (run-clustered Wald chi2);
  - the allocation rule: slot rule against card rule, per-observation log-likelihood, and final links at 2 and 0 pals;
  - known misses: two-chain decks (two plain 3-link SSRs, 4 pals), early links (1 and 2) per pal count with the run as
    a cluster, chains never started with five or six chain cards;
  - the activation parameter a (a chain card active with probability a), fitted jointly with N ~ Binomial(21, p),
    exact over the chain states on every deck.
The model: N chain events per run; each advances one running non-pal card's chain by a link, the card picked with
weight = links left ("slots") or uniformly ("cards"); stop risk (card_events.json) ends a chain at link l with k/n.
Fits of N are exact over the chain states, with a 3% chance that a fired link's marker does not show (without it a run
whose later link shows while an earlier one does not has likelihood 0; allowance_claims measures the rate); the
allocation, final-link and early-link comparisons score each observation on its own
(P(card reaches link) on the run's deck). Intervals on rates over observations that come several to a run (final
links, the allowance, chains never started) are run-clustered (common.cluster_ci), the spread between cards a
run-clustered Wald test (common.cluster_chi2); a share with one observation per run (two-chain decks) takes Wilson."""
import collections, itertools, math, os, re, sys
from collections.abc import Callable, Iterable, Sequence
from typing import Any
import numpy as np
from scipy.optimize import minimize, minimize_scalar
from scipy.stats import chi2

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
from common import Run

Kind = tuple[int, int, tuple[tuple[float, ...], ...]]   # a chain card: (links, links, stop risk: (link, k, n) each)
Obs = tuple[int, int, bool]                             # an observation: (card index in the chain cards, link, seen)
Row = tuple[Run, list[Kind], list[int], list[Obs]]      # a run in a data set: (run, chain kinds, chain card ids, observations)
Paths = tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]
Pmf = Sequence[float] | np.ndarray

S: str = '8'
EPS: float = 0.03     # chance a fired link's marker hint does not show (the fits' allowance)
KMAX: int = 16      # distribution-free fit: P(N = 0..15) and P(N >= 16)
ALL: set[int] = {0, 1, 2, 3, 4, 5}    # pal counts of decks with a chain card
MISC: dict[Any, Any] = {}      # numbers the findings and the detail use that the document does not state


# ---------------------------------------------------------------- the model
def binom_pmf(n: int, p: float, upto: int = 61) -> list[float]:
    return [math.comb(n, e) * p ** e * (1 - p) ** (n - e) if e <= n else 0.0 for e in range(upto)]


def pois_pmf(lam: float, upto: int = 61) -> list[float]:
    return [math.exp(-lam) * lam ** e / math.factorial(e) for e in range(upto)]


def chain_model() -> tuple[int, float]:
    """the solver's event count, N ~ Binomial(CHAIN_N, CHAIN_P) (pkg/loopopt/src/deck/chains.rs)"""
    src = open(f'{common.ROOT}/pkg/loopopt/src/deck/chains.rs', encoding='utf-8').read()
    return int(re.search(r'CHAIN_N: u32 = (\d+);', src).group(1)), float(re.search(r'CHAIN_P: f64 = ([\d.]+);', src).group(1))


CN, CP = chain_model()
BIN: list[float] = binom_pmf(CN, CP)
POIS: list[float] = pois_pmf(7.1)


def canon(kinds: Sequence[Kind]) -> tuple[tuple[Kind, ...], dict[int, int]]:
    """the deck's chain kinds in a canonical order, and each card's position in it"""
    order = sorted(range(len(kinds)), key=lambda i: kinds[i])
    return tuple(kinds[i] for i in order), {ci: j for j, ci in enumerate(order)}


_paths: dict[tuple[tuple[Kind, ...], str], Paths] = {}


def paths(ck: tuple[Kind, ...], rule: str = 'slots') -> Paths:
    """Every (state, events taken, absorbed) the chain process passes through, with its probability, summed over
    stop-risk outcomes, as arrays (S: links fired per card, L, A, P). A state at level l is final with P(N = l), or with
    P(N >= l) when nothing can advance (absorbed). rule: 'slots' (weight = links left) or 'cards' (uniform)."""
    key = (ck, rule)
    if key in _paths: return _paths[key]
    n = len(ck); acc: collections.defaultdict[tuple[tuple[int, ...], int, bool], float] = collections.defaultdict(float)
    alts: list[list[tuple[float, float]]] = []
    for links, stop, risk in ck:
        v, go = [], 1.0
        for l, k, m in sorted(risk): v.append((l, go * k / m)); go *= 1 - k / m
        v.append((stop, go)); alts.append(v)
    for combo in itertools.product(*alts):
        w = math.prod(p for _, p in combo); stops = [s for s, _ in combo]
        if w == 0: continue
        layer: dict[tuple[int, ...], float] = {tuple([0] * n): 1.0}; lvl = 0
        while layer:
            nxt: collections.defaultdict[tuple[int, ...], float] = collections.defaultdict(float)
            for s, p in layer.items():
                left = [(ck[i][0] - s[i]) if s[i] < stops[i] else 0 for i in range(n)]
                ws = left if rule == 'slots' else [1 if x else 0 for x in left]
                tot = sum(ws); acc[(s, lvl, tot == 0)] += w * p
                for i in range(n):
                    if ws[i]: t = list(s); t[i] += 1; nxt[tuple(t)] += p * ws[i] / tot
            layer = nxt; lvl += 1
    ks = list(acc)
    out = (np.array([k[0] for k in ks], dtype=int).reshape(len(ks), n), np.array([k[1] for k in ks]),
           np.array([k[2] for k in ks]), np.array([acc[k] for k in ks]))
    _paths[key] = out
    return out


def final_w(ck: tuple[Kind, ...], pmf: Pmf, rule: str = 'slots') -> tuple[np.ndarray, np.ndarray]:
    """each path entry's probability of being the final state under the event-count pmf"""
    S, L, A, P = paths(ck, rule); pm = np.asarray(pmf, float)
    tail = np.concatenate([[1.0], 1.0 - np.cumsum(pm)])
    ok = L < len(pm); Lc = np.minimum(L, len(pm) - 1)
    return S, np.where(ok, P * np.where(A, tail[Lc], pm[Lc]), 0.0)


_reach: dict[tuple[Any, ...], dict[tuple[int, int], float]] = {}


def reach(kinds: Sequence[Kind], pmf: Pmf = BIN, rule: str = 'slots') -> dict[tuple[int, int], float]:
    """{(card index, link): P(the card reaches the link)} for a deck's chain kinds"""
    ck, pos = canon(tuple(kinds))
    key = (ck, rule, 'B' if pmf is BIN else 'P' if pmf is POIS else tuple(pmf))
    if key not in _reach:
        S, w = final_w(ck, pmf, rule)
        _reach[key] = {(j, k): float(w[S[:, j] >= k].sum()) for j in range(len(ck)) for k in range(1, ck[j][0] + 1)}
    r = _reach[key]
    return {(ci, k): r[(j, k)] for ci, j in pos.items() for k in range(1, kinds[ci][0] + 1)}


def run_levels(kinds: Sequence[Kind], obs: Iterable[Obs], eps: float = EPS, rule: str = 'slots') -> tuple[np.ndarray, np.ndarray]:
    """the run's observations weighted over the chain process, by events taken: (A, B) arrays over levels 0..40,
    A for states that end there with N = l, B for absorbed states (N >= l)"""
    ck, pos = canon(tuple(kinds)); S, L, A, P = paths(ck, rule); w = P.copy()
    for ci, lk, seen in obs:
        fired = S[:, pos[ci]] >= lk
        w = w * np.where(fired, (1 - eps) if seen else eps, 0.0 if seen else 1.0)
    a = np.bincount(L[~A], w[~A], minlength=41)[:41]; b = np.bincount(L[A], w[A], minlength=41)[:41]
    return a, b


def lump(v: Pmf) -> list[float]:
    v = list(v[:KMAX]); v += [0.0] * (KMAX - len(v)); return v + [max(0.0, 1 - sum(v))]


def lk_matrix(xs: Iterable[tuple[list[Kind], list[Obs]]]) -> np.ndarray:
    """per run, P(observations | N = k) for k = 0..KMAX-1 and N >= KMAX (the distribution-free fit's design)"""
    out: list[list[float]] = []
    for kinds, obs in xs:
        a, b = run_levels(kinds, obs)
        row = [a[k] + b[:k + 1].sum() for k in range(KMAX)] + [b.sum() + a[KMAX:].sum()]
        out.append(row)
    return np.array(out)


def em(M: np.ndarray, iters: int = 500) -> np.ndarray:
    pi = np.full(KMAX + 1, 1 / (KMAX + 1))
    for _ in range(iters):
        w = pi * M; pi = (w / w.sum(1, keepdims=True)).mean(0)
    return pi


def ll(pi: Pmf, M: np.ndarray) -> float: return float(np.log(np.maximum(1e-300, M @ np.asarray(pi))).sum())


def mean_var(v: np.ndarray) -> tuple[float, float]:
    k = np.arange(len(v)); m = float((k * v).sum()); return m, float((k * k * v).sum()) - m * m


def obs_ll(xs: Iterable[Row], pmf: Pmf = BIN, rule: str = 'slots') -> float:
    """the per-observation log-likelihood of the runs' observations: each link scored on its own by P(the card reaches
    the link) on the run's deck"""
    t = 0.0
    for r, kinds, chain, obs in xs:
        mb = reach(kinds, pmf, rule)
        for ci, lk, seen in obs:
            pr = min(max(mb[(ci, lk)], 1e-9), 1 - 1e-9); t += math.log(pr if seen else 1 - pr)
    return t


# ---------------------------------------------------------------- markers (GameTora card events)
_m: dict[str, Any] = {}


def game() -> dict[str, Any]:
    """skill names, chain link of each story, decoded SR/SSR card events, the per-card skill pools and each card's
    observable links with their markers"""
    if _m: return _m
    gt = common.gametora(); db = common.mdb(); cards = common.cards()
    sk = dict(db.execute("select [index],text from text_data where category=47"))
    link_of = dict(db.execute("select story_id,show_progress_1 from single_mode_story_data where show_progress_1>0"))

    def names(hs: Iterable[tuple[str, Any, Any]]) -> set[str]:
        out: set[str] = set()
        for kind, sid, _ in hs:
            for i in ([sid] if kind == 'hint' else sid):
                if i in sk: out.add(sk[i])
        return out
    evs: collections.defaultdict[int, list[tuple[int | None, list[list[tuple[set[str], Any]]]]]] = collections.defaultdict(list)     # card -> [(link or None, [choice -> [(outcome skills, ends chain)]])]
    for f in ('training_events_sr', 'training_events_ssr'):
        for cid, es in gt.load(f):
            for e in es:
                chs = [[(names(h), end) for h, end in gt.outcomes(ch[1])] for ch in gt.choices(e[1])]
                evs[cid].append((link_of.get(int(e[2])), chs))
    # every skill each card's events can give: SR/SSR events, pal / group events, card_events.json
    pool: collections.defaultdict[int, set[str]] = collections.defaultdict(set)
    for cid, es in evs.items():
        for _, chs in es: pool[cid] |= {s for ch in chs for o, _ in ch for s in o}

    def walk(x: Any, acc: set[str]) -> None:
        if isinstance(x, list):
            if len(x) == 2 and isinstance(x[1], list) and x[1] and all(isinstance(k, int) for k in x[1]):
                acc |= names(gt.hints(x[1]))
            for y in x: walk(y, acc)
    for kindf in ('friend', 'group'):
        for row in gt.load(f'training_events_{kindf}'):
            acc: set[str] = set(); walk(row[1:], acc)
            for cid, c in cards.items():
                if c['pal'] and ((kindf == 'friend' and c['chara'] == row[0]) or (kindf == 'group' and cid == row[0])): pool[cid] |= acc
    cev = common.gjson('card_events.json')['cards']
    for cid, e in cev.items():
        pool[int(cid)] |= set(e.get('always', [])) | {n for n, _ in e.get('disabled', [])} | {n for n, _ in e.get('chance', [])}
    # markers: link -> skills unique to the link's single event, when every choice and outcome carries one
    markers: dict[int, dict[int, set[str]]] = {}
    for cid, es in evs.items():
        per: collections.defaultdict[int, list[list[list[tuple[set[str], Any]]]]] = collections.defaultdict(list); cnt: collections.Counter[str] = collections.Counter()
        for lk, chs in es:
            cnt.update({s for ch in chs for o, _ in ch for s in o})
            if lk: per[lk].append(chs)
        out: dict[int, set[str]] = {}
        for lk, lst in per.items():
            if len(lst) != 1 or not lst[0]: continue
            uniq = {s for ch in lst[0] for o, _ in ch for s in o if cnt[s] == 1}
            if uniq and all(o & uniq for ch in lst[0] for o, _ in ch): out[lk] = uniq
        markers[cid] = out
    _m.update(cards=cards, sk=sk, pool=pool, markers=markers, cev=cev)
    return _m


def kind(cid: int) -> Kind | None:
    """(links, links, stop risk) for a chain card, None for a pal, group or chainless card"""
    g = game(); c = g['cards'][cid]
    if c['pal'] or c['links'] == 0: return None
    risk = tuple(sorted(tuple(r) for r in g['cev'].get(str(cid), {}).get('stop_risk', []) if r[0] < c['links'] and r[0] <= 4))
    return (c['links'], c['links'], risk)


def pals(r: Run) -> int:
    cards = game()['cards']
    return sum(1 for c, _, _ in r['deck'] if c in cards and cards[c]['pal'])


def observations(r: Run) -> tuple[list[Kind], list[int], list[Obs]] | None:
    """a run's chain observations: (chain kinds, chain card ids, [(card index, link, seen)]), or None for a deck with an
    unknown or non-pal R card"""
    g = game(); cards = g['cards']; deck = [c for c, _, _ in r['deck']]
    if any(c not in cards for c in deck): return None
    if any(cards[c]['rarity'] == 1 and not cards[c]['pal'] for c in deck): return None
    hints = {h['name']: h['src'] for h in r['hints']}
    name = {c: common.bare(n) for c, _, n in r['deck']}
    by_name = {name[c]: c for c in deck}
    chain = [c for c in deck if kind(c)]; obs: list[Obs] = []
    for ci, cid in enumerate(chain):
        nm = name[cid]
        for lk, mk in g['markers'].get(cid, {}).items():
            if lk > cards[cid]['links']: continue
            credits = [[y.strip() for y in x[:-6].split(' or ')] for s in mk for x in hints.get(s, ()) if x.endswith(' event')]
            credits = [c for c in credits if nm in c]
            # "A or B event": unknown when B is another deck card whose events can give the marker
            if credits and all(any(y != nm and y in by_name and mk & g['pool'][by_name[y]] for y in c) for c in credits): continue
            seen = bool(credits)
            if not seen and any(s in hints for s in mk) and all(hints[s] == ['Inspiration'] for s in mk if s in hints): continue
            obs.append((ci, lk, seen))
    return [kind(c) for c in chain], chain, obs


# ---------------------------------------------------------------- data sets
_obs: dict[int, tuple[list[Kind], list[int], list[Obs]] | None] = {}


def dataset(groups: set[int]) -> list[Row]:
    """[(run, kinds, chain card ids, observations)] for every run record with a pal count in `groups` (decks with an
    unknown or non-pal R card left out)"""
    out: list[Row] = []
    for r in common.runs():
        if pals(r) not in groups: continue
        if r['id'] not in _obs: _obs[r['id']] = observations(r)
        if _obs[r['id']] is not None: out.append((r,) + _obs[r['id']])
    return out


def dropped(groups: set[int]) -> int:
    """records with a pal count in `groups` left out for an unknown or non-pal R card"""
    dataset(groups)
    return sum(1 for r in common.runs() if pals(r) in groups and _obs[r['id']] is None)


def cmp_pmf(lam: float, nu: float, upto: int = 61) -> list[float]:
    lw = np.array([k * math.log(lam) - nu * math.lgamma(k + 1) for k in range(upto)]); w = np.exp(lw - lw.max()); return list(w / w.sum())


def cmp_ll(M: np.ndarray) -> tuple[float, float, float]:
    """the Conway-Maxwell-Poisson fit (P(N = k) proportional to lam^k / (k!)^nu: another two-parameter count family that
    can be under-dispersed), by Nelder-Mead from three starts: (loglik, lam, nu)"""
    def pmf(lam: float, nu: float) -> list[float]:
        lw = np.array([k * math.log(lam) - nu * math.lgamma(k + 1) for k in range(61)]); w = np.exp(lw - lw.max()); return list(w / w.sum())
    f: Callable[[np.ndarray], float] = lambda th: -ll(lump(pmf(math.exp(th[0]), math.exp(th[1]))), M)
    r = min((minimize(f, x0, method='Nelder-Mead') for x0 in ([2, 0], [3, 0.5], [4, 0.7])), key=lambda r: r.fun)
    return -r.fun, math.exp(r.x[0]), math.exp(r.x[1])


def binom_profile(M: np.ndarray, ns: Iterable[int] = range(8, 41)) -> list[tuple[int, float, float]]:
    """the profile likelihood of the binomial's n: at each n the best p (bounded scalar search), exact over the chain
    states through the run matrix M. Returns [(n, p, loglik)]; the data fix the mean n p well and n only within an
    interval, since n and p trade off along a ridge"""
    out: list[tuple[int, float, float]] = []
    for n in ns:
        r = minimize_scalar(lambda p: -ll(lump(binom_pmf(n, p)), M), bounds=(0.05, 0.95), method='bounded', options={'xatol': 1e-5})
        out.append((n, float(r.x), -float(r.fun)))
    return out


def held_out(xs: list[Row], M: np.ndarray) -> dict[str, tuple[float, float, float]]:
    """5-fold cross-validation (common.fold by run id): each fold's runs are scored by forms fitted on the other four, the
    free distribution, the binomial by likelihood (n 10..40, p re-fitted), the binomial by moments (n = mean^2 / (mean -
    variance) of the free fit, rounded; p = mean / n), the Conway-Maxwell-Poisson and the Poisson (mean fitted), against
    Binomial(21, 0.35) (nothing fitted). Per run held-out log-likelihoods; {form: (nats, SE, z)} from
    common.compare_held_out, positive when the form predicts unseen runs better than (21, 0.35)"""
    fo = np.array([common.fold(r['id']) for r, *_ in xs])
    per = {f: np.zeros(len(xs)) for f in ('free', 'binomial by likelihood', 'binomial by moments', 'Conway-Maxwell-Poisson', 'Poisson', 'base')}
    for k in range(common.K_FOLDS):
        tr, te = fo != k, fo == k
        T, E = M[tr], M[te]
        free = em(T); m, v = mean_var(free)
        bn, bp, _ = max(binom_profile(T, range(10, 41)), key=lambda x: x[2])
        nm = round(m * m / (m - v)); pm = m / nm
        _, lam, nu = cmp_ll(T)
        pmfs = {'free': list(free), 'binomial by likelihood': lump(binom_pmf(bn, bp)), 'binomial by moments': lump(binom_pmf(nm, pm)),
                'Conway-Maxwell-Poisson': lump(cmp_pmf(lam, nu)), 'Poisson': lump(pois_pmf(m)), 'base': lump(BIN)}
        for f, pmf in pmfs.items(): per[f][te] = np.log(np.maximum(1e-300, E @ np.asarray(pmf)))
    return {f: common.compare_held_out(per[f], per['base']) for f in per if f != 'base'}


def binom_mle(xs: list[Row]) -> tuple[float, int, float]:
    """the Binomial(n, p) grid MLE (n 6..40, p 0.200..0.995 step 0.005), exact over the chain states: (loglik, n, p)"""
    ab = [run_levels(k, o) for _, k, _, o in xs]
    A = np.array([a for a, _ in ab]); B = np.array([b for _, b in ab])
    grid = [(n, k / 200) for n in range(6, 41) for k in range(40, 200)]
    PM = np.array([binom_pmf(n, p, 41) for n, p in grid]); TL = np.hstack([np.ones((len(grid), 1)), 1 - np.cumsum(PM, 1)[:, :40]])
    V = A @ PM.T + B @ TL.T
    lls = np.where((V <= 0).any(0), -1e18, np.log(np.maximum(V, 1e-300)).sum(0))
    return max((float(v), n, p) for v, (n, p) in zip(lls, grid))


# ---------------------------------------------------------------- values
# example decks: chain kinds, and the cards shown (one of identical cards)
ALLOC: list[tuple[str, list[Kind], tuple[str, ...]]] = [('two 3-link SSRs', [(3, 3, ())] * 2, ('',)), ('3-link SSR + 2-link SR', [(3, 3, ()), (2, 2, ())], (' SSR', ' SR')),
         ('three 3-link SSRs', [(3, 3, ())] * 3, ('',)), ('four 3-link SSRs', [(3, 3, ())] * 4, ('',))]


def V(name: str, got: float | int | str | None, **kw: str) -> common.Value:
    return common.Value(S, name, got, **kw)


def derived() -> list[common.Value | common.Finding]:
    out: list[common.Value | common.Finding] = []
    m, v = CN * CP, CN * CP * (1 - CP)
    out += [V('Binomial(21, 0.35): mean', m), V('Binomial(21, 0.35): variance', v, note=f'{v:.4f}')]
    S2, w = final_w(((3, 3, ()), (3, 3, ())), BIN)
    both = float(w[(S2 == 3).all(1)].sum())
    out.append(V('two 3-link cards: P(N >= 6) = P(both chains complete)', both, note=f'P(N >= 6) {sum(BIN[6:]):.4f}'))
    for deck, kinds, whos in ALLOC:
        for rule in ('slots', 'cards'):
            r = reach(kinds, BIN, rule)
            for ci, who in enumerate(whos):
                for k in range(1, kinds[ci][0] + 1):
                    out.append(V(f'{deck}{who}, {rule} rule: P(reach link {k})', r[(ci, k)]))
    for rule in ('slots', 'cards'):
        S3, w = final_w(((3, 3, ()), (3, 3, ())), [0, 0, 0, 1.0], rule)
        out.append(V(f'two 3-link cards, exactly 3 events, {rule} rule: P(final state (3, 0))', float(w[(S3[:, 0] == 3) & (S3[:, 1] == 0)].sum())))
    out.append(V('distribution-free fit: parameters more than the binomial', KMAX - 2, note=f'P(N = 0..{KMAX - 1}), P(N >= {KMAX}): {KMAX} free'))
    return out


def fit_claims() -> list[common.Value | common.Finding]:
    """the fit of N (every run, 0-5 pals) and its per-scenario split"""
    xs = dataset(ALL); note = f'every run, 0-5 pals; {dropped(ALL)} decks with a non-pal R card left out'
    M = lk_matrix([(k, o) for _, k, _, o in xs]); pooled = em(M); llp = ll(pooled, M)
    mn, vr = mean_var(pooled)
    l21 = ll(lump(BIN), M); l15 = ll(lump(binom_pmf(15, 0.48)), M); lp = ll(lump(POIS), M)
    prof = binom_profile(M); bn, bp, lb = max(prof, key=lambda x: x[2])
    ho = held_out(xs, M)
    out: list[common.Value | common.Finding] = [V('fit of N: runs', len(xs), note=note),
           V('distribution-free: mean', mn), V('distribution-free: variance', vr),
           V('Binomial(21, 0.35): nats behind distribution-free', llp - l21, note=f'LL {l21:.1f} vs {llp:.1f}'),
           V('Binomial(15, 0.48): nats behind distribution-free', llp - l15),
           V('Poisson(7.1): nats behind distribution-free', llp - lp),
           V('best binomial (n free): nats better than (21, 0.35)', lb - l21),
           V('best binomial n free: n', bn), V('best binomial n free: p', bp, note='p re-fitted at each n'),
           V('best binomial n free: nats behind distribution-free', llp - lb),
           V('best binomial n free: LR p', float(chi2.sf(2 * (llp - lb), KMAX - 2))),
           V('binomial n: 95% profile interval low', min(n_ for n_, _, l_ in prof if lb - l_ <= 1.920729)),
           V('binomial n: 95% profile interval high', max(n_ for n_, _, l_ in prof if lb - l_ <= 1.920729)),
           V('binomial n = 21, p re-fitted: nats below the best n', lb - dict((n_, l_) for n_, _, l_ in prof)[21]),
           V('binomial mean n p at the best n', bn * bp),
           V('distribution-free: variance / mean', vr / mn),
           *[V(f'held out: {form} over Binomial(21, 0.35): {w}', x) for form, t_ in ho.items() for w, x in zip(('nats', 'SE', 'z'), t_)],
           V('Conway-Maxwell-Poisson: nats behind distribution-free', llp - cmp_ll(M)[0]),
           V('Conway-Maxwell-Poisson: LR p', float(chi2.sf(2 * (llp - cmp_ll(M)[0]), KMAX - 2)))]
    by: collections.defaultdict[int, list[int]] = collections.defaultdict(list)
    for i, (r, *_rest) in enumerate(xs): by[r['scenario']].append(i)
    sep = {sc: em(M[ix]) for sc, ix in by.items() if len(ix) >= 30}
    lls = sum(ll(sep[sc], M[by[sc]]) for sc in sep) + sum(ll(pooled, M[ix]) for sc, ix in by.items() if sc not in sep)
    for sc in sorted(set(by) | {1, 2, 3}, key=[2, 3, 1, 4].index):
        out.append(V(f'per scenario: mean, {common.SCEN[sc]}', mean_var(sep[sc])[0] if sc in sep else None, note=f'{len(by[sc])} runs'))
    out.append(V('per scenario: log-likelihood gain', lls - llp))
    out.append(V('per scenario: extra parameters', (len(sep) - 1) * KMAX, note=f'{len(sep)} scenarios with 30+ runs'))
    MISC['fit'] = dict(llp=llp, lls={nm: ll(lump(v), M) for nm, v in (('Binomial(21, 0.35)', BIN), ('Binomial(15, 0.48)', binom_pmf(15, 0.48)), ('Poisson(7.1)', POIS))},
                       runs=len(xs), by_pals=dict(sorted(collections.Counter(pals(r) for r, *_ in xs).items())))
    return out


def pal_count_claims() -> tuple[list[common.Value | common.Finding], dict[int, tuple[int, tuple[float, int, float]]]]:
    """N per pal count: the binomial grid MLE on each pal count's runs (every run, 1-4 pals)"""
    out: list[common.Value | common.Finding] = []; res: dict[int, tuple[int, tuple[float, int, float]]] = {}
    for npal in (1, 2, 3, 4):
        xs = dataset({npal}); res[npal] = (len(xs), binom_mle(xs))
    out.append(V('per pal count: runs (1-4 pals)', sum(n for n, _ in res.values()), note='every run'))
    for npal in (1, 2, 3, 4):
        n, (v, bn, bp) = res[npal]
        out.append(V(f'per pal count: mean, {npal} pals', bn * bp, note=f'{n} runs; Binomial MLE n {bn}, p {bp:.2f}'))
    vs = [bn * bp * (1 - bp) for _, (_, bn, bp) in res.values()]
    out += [V('per pal count: variance, lowest', min(vs)), V('per pal count: variance, highest', max(vs))]
    return out, res


def obs_ci(rows: list[tuple[float, float, int]]) -> tuple[float, float]:
    """the observed rate's 95% interval over (observed, predicted, run id) rows that come several to a run: the mean
    prediction times the run-clustered interval of observed / predicted (common.cluster_ci); Wilson when every run gives
    one row (5 pals: one chain card)"""
    o, e, r = zip(*rows)
    if len(set(r)) == len(r): return common.wilson(round(sum(o)), len(o))
    _, lo, hi, _ = common.cluster_ci(o, e, r); m = sum(e) / len(e)
    return lo * m, hi * m


def final_claims() -> tuple[list[common.Value | common.Finding], dict[str, dict[int, tuple[Any, ...]]]]:
    """the final-link table (each observed rate with its run-clustered 95% interval against Binomial(21, 0.35)) and the
    early-link shortfall (every run)"""
    out: list[common.Value | common.Finding] = []; tab: dict[int, tuple[Any, ...]] = {}
    for npal in (5, 4, 3, 2, 1, 0):
        xs = dataset({npal}); o = b = p = 0.0; n = 0; rows: list[tuple[float, float, int]] = []
        for r, kinds, chain, obs in xs:
            mb, mp = reach(kinds, BIN), reach(kinds, POIS)
            for ci, lk, seen in obs:
                if lk == kinds[ci][0]: o += seen; b += mb[(ci, lk)]; p += mp[(ci, lk)]; n += 1; rows.append((seen, mb[(ci, lk)], r['id']))
        lo, hi = obs_ci(rows)
        tab[npal] = (len(xs), o / n, b / n, p / n, n, lo, hi)
        out += [V(f'final-link table, {npal} pals: runs', len(xs)),
                V(f'final-link table, {npal} pals: observed', o / n, note=f'{n} card-links'),
                V(f'final-link table, {npal} pals: observed 95% low', lo), V(f'final-link table, {npal} pals: observed 95% high', hi),
                V(f'final-link table, {npal} pals: Binomial(21, 0.35)', b / n),
                V(f'final-link table, {npal} pals: Poisson(7.1)', p / n)]
    out.append(V('final-link table: runs in all', sum(t[0] for t in tab.values()), note=f'every run; {dropped(ALL)} decks with a non-pal R card left out'))
    # early links (1 and 2), per run as a cluster
    early: dict[int, tuple[Any, ...]] = {}
    for npal in (4, 3, 2, 1, 0):
        per: list[tuple[float, float, int]] = []
        for r, kinds, chain, obs in dataset({npal}):
            mb = reach(kinds, BIN); o = e = 0.0; k = 0
            for ci, lk, seen in obs:
                if lk == kinds[ci][0]: continue
                o += seen; e += mb[(ci, lk)]; k += 1
            if k: per.append((o, e, k))
        n = sum(x[2] for x in per); O = sum(x[0] for x in per); E = sum(x[1] for x in per)
        sd = math.sqrt(sum((x[0] - x[1]) ** 2 for x in per)); early[npal] = (O / n, E / n, n, len(per))
        out += [V(f'early-link shortfall, {npal} pals', (E - O) / n, note=f'{n} links in {len(per)} runs; obs {O / n:.2f} pred {E / n:.2f}'),
                V(f'early-link shortfall, {npal} pals: +-', sd / n), V(f'early-link shortfall, {npal} pals: z', (E - O) / sd)]
    return out, dict(final=tab, early=early)


def allowance_claims() -> list[common.Value | common.Finding]:
    """the marker allowance: links fire in order, so a card's later link seen means its earlier links fired; the share of
    those earlier links whose marker is missing measures how often a fired link goes unseen (every run; its 95% interval
    run-clustered)"""
    imp = un = 0; rows: list[tuple[float, float, int]] = []
    for r, kinds, chain, obs in dataset(ALL):
        per: collections.defaultdict[int, dict[int, bool]] = collections.defaultdict(dict)
        for ci, lk, seen in obs: per[ci][lk] = seen
        for d in per.values():
            for lk, seen in d.items():
                if any(l2 > lk and s2 for l2, s2 in d.items()): imp += 1; un += not seen; rows.append((int(not seen), 1, r['id']))
    lo, hi = obs_ci(rows)          # one run gives several such links: the run-clustered interval of the rate
    return [V('allowance: earlier links implied fired', imp, note='every run'), V('allowance: marker not shown', un),
            V('allowance: share', un / imp), V('allowance: 95% low', lo), V('allowance: 95% high', hi)]


def final_by_card() -> dict[str, Any]:
    """final-link reach by card (every run) against the chain model on each run's deck: the pooled rate and the spread
    between cards with 60+ observations (a joint Wald test that every card's observed - predicted is zero, one df per
    card, its covariance run-clustered by common.cluster_chi2: the chain cards of one run share its N). Each card's z
    (binomial variance; one observation per run) only ranks the cards"""
    per: collections.defaultdict[int, list[float]] = collections.defaultdict(lambda: [0, 0.0, 0, 0.0]); res: list[tuple[int, float, int]] = []
    for r, kinds, chain, obs in dataset(ALL):
        mb = reach(kinds)
        for ci, lk, seen in obs:
            if lk != kinds[ci][0]: continue
            a = per[chain[ci]]; p_ = mb[(ci, lk)]; a[0] += seen; a[1] += p_; a[2] += 1; a[3] += p_ * (1 - p_)
            res.append((chain[ci], seen - p_, r['id']))
    k = sum(v[0] for v in per.values()); e = sum(v[1] for v in per.values()); n = sum(v[2] for v in per.values())
    zs = {c: (v[0] - v[1]) / math.sqrt(v[3]) for c, v in per.items() if v[2] >= 60}
    kept = [(c, d, r) for c, d, r in res if c in zs]
    x2, _, pv = common.cluster_chi2([(c, d) for c, d, _ in kept], [r for _, _, r in kept]); names = game()['cards']
    return dict(obs=k / n, model=e / n, n=n, cards=len(zs), chi2=x2, p=pv,
                low=sorted(zs.items(), key=lambda t: t[1])[:3], high=sorted(zs.items(), key=lambda t: -t[1])[:3],
                rate={c: (per[c][0] / per[c][2], per[c][1] / per[c][2]) for c in zs}, name=lambda c: names[c]['name'])


def allowance_sens() -> dict[float | str, float]:
    """the best p at n = CHAIN_N under several allowances, and with none on the runs showing no contradiction"""
    xs = dataset(ALL); out: dict[float | str, float] = {}

    def best_p(xs: list[Row], e: float) -> float:
        ab = [run_levels(k, o, eps=e) for _, k, _, o in xs]
        A = np.array([a for a, _ in ab]); B = np.array([b for _, b in ab]); res: list[tuple[float, float]] = []
        for p_ in [i / 1000 for i in range(300, 401, 5)]:
            pm = np.array(binom_pmf(CN, p_, 41)); v = A @ pm + B @ np.concatenate([[1.0], 1 - np.cumsum(pm)[:40]])
            if (v > 0).all(): res.append((float(np.log(v).sum()), p_))
        return max(res)[1]
    for e in (0.01, 0.02, 0.03, 0.045): out[e] = best_p(xs, e)

    def contra(o: list[Obs]) -> bool:
        per: collections.defaultdict[int, dict[int, bool]] = collections.defaultdict(dict)
        for ci, lk, seen in o: per[ci][lk] = seen
        return any(not sn and any(l2 > lk and s2 for l2, s2 in d.items()) for d in per.values() for lk, sn in d.items())
    clean = [x for x in xs if not contra(x[3])]
    out[0.0] = best_p(clean, 0.0); out['clean runs'] = len(clean)
    return out


def alloc_claims() -> tuple[list[common.Value | common.Finding], dict[str, Any]]:
    """slot rule against card rule (every run, 0-5 pals; per-observation likelihood); final links observed with the
    run-clustered 95% interval against each rule"""
    tot = collections.Counter(); fin: dict[tuple[int, str], tuple[float, ...]] = {}; runs = 0
    for npal in (5, 4, 3, 2, 1, 0):
        xs = dataset({npal}); runs += len(xs)
        for rule in ('slots', 'cards'):
            tot[rule] += obs_ll(xs, BIN, rule); o = p = 0.0; n = 0; rows: list[tuple[float, float, int]] = []
            for r, kinds, chain, obs in xs:
                mb = reach(kinds, BIN, rule)
                for ci, lk, seen in obs:
                    if lk == kinds[ci][0]: o += seen; p += mb[(ci, lk)]; n += 1; rows.append((seen, mb[(ci, lk)], r['id']))
            fin[(npal, rule)] = (o / n, p / n, n, *obs_ci(rows))     # the observed rate's interval against this rule
    out: list[common.Value | common.Finding] = [V('allocation: runs', runs, note='every run, 0-5 pals'),
           V('allocation: total LL, slot rule', tot['slots']), V('allocation: total LL, card rule', tot['cards'])]
    for npal in (2, 0):
        out += [V(f'allocation: final links, {npal} pals, observed', fin[(npal, 'slots')][0], note=f'{fin[(npal, "slots")][2]} card-links'),
                V(f'allocation: final links, {npal} pals, slot rule', fin[(npal, 'slots')][1]),
                V(f'allocation: final links, {npal} pals, card rule', fin[(npal, 'cards')][1])]
    out.append(V('allocation: slot over card, nats', tot['slots'] - tot['cards']))
    return out, dict(total=tot, final=fin)


def with_activation(kinds: Sequence[Kind], a: float) -> tuple[Kind, ...]:
    """the chain kinds with each card active with probability a (a stop at link 0 with chance 1 - a)"""
    return tuple((l, s_, tuple(sorted(rk + ((0, round(1 - a, 6), 1),)))) for l, s_, rk in kinds) if a < 1 else tuple(kinds)


def miss_claims() -> list[common.Value | common.Finding]:
    out: list[common.Value | common.Finding] = []
    # two-chain decks: two plain 3-link SSRs with four pals, both final links observable
    clean: list[bool] = []
    for r, kinds, chain, obs in dataset({4}):
        if len(kinds) != 2 or any(k != (3, 3, ()) for k in kinds): continue
        fz = {ci: seen for ci, lk, seen in obs if lk == 3}
        if len(fz) == 2: clean.append(sum(fz.values()) == 2)
    k = sum(clean); n = len(clean); lo, hi = common.wilson(k, n)
    out += [V('two-chain decks: both chains finish', k, note='every run'), V('two-chain decks: runs', n),
            V('two-chain decks: share', k / n), V('two-chain decks: 95% low', lo), V('two-chain decks: 95% high', hi),
            V('two-chain decks: predicted, P(N >= 6)', sum(BIN[6:]))]
    MISC['two-chain'] = (k, n, lo, hi)
    # chains never started: link 1 not reached, by chain cards in the deck
    rows: collections.defaultdict[int, list[float]] = collections.defaultdict(lambda: [0, 0.0, 0])
    nrows: collections.defaultdict[int, list[tuple[float, float, int]]] = collections.defaultdict(list)
    for r, kinds, chain, obs in dataset(ALL):
        mb = reach(kinds, BIN)
        for ci, lk, seen in obs:
            if lk == 1:
                a = rows[len(kinds)]; a[0] += seen; a[1] += mb[(ci, 1)]; a[2] += 1
                nrows[len(kinds)].append((1 - seen, 1 - mb[(ci, 1)], r['id']))
    MISC['never rows'] = dict(rows)
    for nc in (5, 6):       # several chain cards per run: the observed rate's interval is run-clustered
        o, p_, n = rows[nc]; lo, hi = obs_ci(nrows[nc])
        MISC[('never', nc)] = (n - o, n, 1 - p_ / n, lo, hi)
        out += [V(f'never started, {nc} chain cards: observed', 1 - o / n, note=f'link 1 not reached, {n} card-links'),
                V(f'never started, {nc} chain cards: observed 95% low', lo), V(f'never started, {nc} chain cards: observed 95% high', hi),
                V(f'never started, {nc} chain cards: model', 1 - p_ / n)]
    # activation parameter, jointly with N ~ Binomial(CHAIN_N, p), exact over the chain process, every deck
    xs = dataset(ALL)

    def LL(p_: float, a: float) -> float:
        pm = np.array(binom_pmf(CN, p_, 41)); tl = np.concatenate([[1.0], 1 - np.cumsum(pm)[:40]]); t = 0.0
        for _, k, _, o in xs:
            A, B = run_levels(with_activation(k, a), o); t += math.log(max(float(A @ pm + B @ tl), 1e-300))
        return t
    l1 = max(LL(p_, 1.0) for p_ in (0.33, 0.34, 0.35, 0.36, 0.37))
    lj, pj, aj = max((LL(p_, a), p_, a) for a in (0.97, 0.94, 0.90, 0.86) for p_ in (0.35, 0.37, 0.39, 0.41))
    out += [V('activation, corrected reading, joint with N: gain, nats', lj - l1, note=f'every run; p {pj}'),
            V('activation, corrected reading, joint with N: a', aj)]
    return out


def claims() -> list[common.Value | common.Finding]:
    out = derived()
    if not common.have_runs(): return out
    fc = fit_claims(); pc, res = pal_count_claims(); flc, fl = final_claims(); ac, al = alloc_claims()
    out += fc + allowance_claims() + pc + flc + ac + miss_claims()
    sens = allowance_sens()
    from scipy.stats import chi2
    gap = next(v.got for v in out if isinstance(v, common.Value) and v.name == 'Binomial(21, 0.35): nats behind distribution-free')
    out.append(V('binomial(21, 0.35) LR p', float(chi2.sf(2 * gap, KMAX - 2)), note=f'{KMAX - 2} df'))
    fb = final_by_card()
    out += [V('final links by card: pooled observed', fb['obs'], note=f"{fb['n']:,} card-links, every run"),
            V('final links by card: pooled model', fb['model']), V('final links by card: cards', fb['cards']), V('final links by card: chi2', fb['chi2'])]
    MISC['final_by_card'] = fb; MISC['pal counts'] = res; MISC['alloc'] = al
    out += [V('allowance 0: runs without a contradiction', sens['clean runs']),
            V('allowance sensitivity: lowest best p at n = 21', min(v for k, v in sens.items() if k != 'clean runs')),
            V('allowance sensitivity: highest best p at n = 21', max(v for k, v in sens.items() if k != 'clean runs'))]
    return out + findings(out, res, fl, al, sens)


def findings(out: list[common.Value | common.Finding], res: dict[int, tuple[int, tuple[float, int, float]]],
             fl: dict[str, dict[int, tuple[Any, ...]]], al: dict[str, Any], sens: dict[float | str, float]) -> list[common.Finding]:
    """the section's conclusions, each tested on the numbers above"""
    from scipy.stats import chi2
    g: dict[str, Any] = {c.name: c.got for c in out if isinstance(c, common.Value)}
    F: Callable[..., common.Finding] = lambda *a: common.finding(S, *a)
    gap = g['Binomial(21, 0.35): nats behind distribution-free']; df = g['distribution-free fit: parameters more than the binomial']; fs: list[common.Finding] = []
    lrp = g['binomial(21, 0.35) LR p']
    fs.append(F(f'Binomial({CN}, {CP}) is not rejected against a free distribution of N (likelihood ratio on its {df:.0f} fewer '
                'parameters, p > 0.05)', lrp > 0.05, f'{gap:.1f} nats behind distribution-free, LR p = {lrp:.3f}'))
    pg_ = g['Poisson(7.1): nats behind distribution-free']; dfp = KMAX - 1
    fs.append(F(f'Poisson(7.1) is rejected against a free distribution of N (likelihood ratio on its {dfp} fewer parameters, p < 0.05)',
                chi2.sf(2 * pg_, dfp) < 0.05, f'{pg_:.1f} nats behind distribution-free, LR p = {chi2.sf(2 * pg_, dfp):.2g}'))
    vm, cp_, bp_ = g['distribution-free: variance / mean'], g['Conway-Maxwell-Poisson: LR p'], g['best binomial n free: LR p']
    fs.append(F('N is under-dispersed (the free fit\'s variance / mean below 1, so no Poisson or over-dispersed family fits), and '
                'the data do not single out the binomial among under-dispersed two-parameter families: the best binomial and the '
                'Conway-Maxwell-Poisson fit are each not rejected against a free distribution (p > 0.05)',
                vm < 1 and bp_ > 0.05 and cp_ > 0.05, f'variance / mean {vm:.2f}; best binomial p {bp_:.3f}; CMP p {cp_:.3f}'))
    hz = {k.split(': ')[1].replace(' over Binomial(21, 0.35)', ''): v for k, v in g.items() if k.startswith('held out:') and k.endswith(': z')}
    fs.append(F('No alternative form predicts unseen runs better than Binomial(21, 0.35): 5-fold cross-validation, per-run held-out '
                'log-likelihoods compared pairwise, every z at most 1.96 (the free distribution, the binomial fitted by likelihood or '
                'by moments, Conway-Maxwell-Poisson, Poisson)', max(hz.values()) <= 1.96,
                '; '.join(f'{f} z {z:+.2f}' for f, z in hz.items())))
    lo_, hi_, b21 = g['binomial n: 95% profile interval low'], g['binomial n: 95% profile interval high'], g['binomial n = 21, p re-fitted: nats below the best n']
    fs.append(F(f'In-sample, n = 21 lies just outside the 95% profile-likelihood interval for the binomial\'s n (p re-fitted at each n; '
                'more than 1.92 nats below the best n)', b21 > 1.920729 and not lo_ <= 21 <= hi_,
                f'n in [{lo_:.0f}, {hi_:.0f}], best n {g["best binomial n free: n"]:.0f}; n = 21 is {b21:.2f} nats below'))
    pg = g['per scenario: log-likelihood gain']; dfs = g['per scenario: extra parameters']
    fs.append(F(f'One N for every scenario: per-scenario distributions gain nothing significant (likelihood ratio on {dfs:.0f} parameters, p > 0.05)',
                chi2.sf(2 * pg, dfs) > 0.05, f'+{pg:.1f} nats, LR p = {chi2.sf(2 * pg, dfs):.2f}'))
    sh = g['allowance: share']; lo, hi = g['allowance: 95% low'], g['allowance: 95% high']
    fs.append(F(f'The {EPS} marker allowance lies inside the 95% interval of the measured rate of fired links whose marker is missing', lo <= EPS <= hi,
                f'{g["allowance: marker not shown"]:.0f} of {g["allowance: earlier links implied fired"]:.0f} = {sh:.3f} [{lo:.3f}, {hi:.3f}]'))
    ps = [v for k, v in sens.items() if k != 'clean runs']; lo_, hi_ = round(CP - 0.01, 2), round(CP + 0.01, 2)    # the adopted p +- 0.01
    fs.append(F(f'The fit of N does not depend on the allowance: over allowances 0-0.045 the best p at n = {CN} stays within {lo_}-{hi_}',
                min(ps) >= lo_ and max(ps) <= hi_,
                ', '.join(f'eps {k}: {v:.3f}' for k, v in sens.items() if k != 'clean runs') + f' (eps 0.0 on the {sens["clean runs"]} runs without a contradiction)'))
    pos: dict[int, tuple[Any, ...]] = {}
    for npal, (runs, o, b, p_, n, l_, h_) in fl['final'].items():
        pos[npal] = ('inside' if l_ <= b <= h_ else 'above' if b > h_ else 'below', o, b, l_, h_)
    fs.append(F('Final links: the Binomial(21, 0.35) prediction lies inside the observed 95% interval at 5, 4, 3, 1 and 0 pals and above it at 2 pals',
                {k: v[0] for k, v in pos.items()} == {5: 'inside', 4: 'inside', 3: 'inside', 2: 'above', 1: 'inside', 0: 'inside'},
                '; '.join(f'{k} pals {c[1]:.3f} [{c[3]:.3f}, {c[4]:.3f}] vs {c[2]:.3f} ({c[0]})' for k, c in sorted(pos.items(), reverse=True))))
    means = {k: bn * bp for k, (n, (_, bn, bp)) in res.items()}
    fs.append(F('Pal events do not consume chain events: the mean of N fitted at 4 pals is at least that at 1 pal', means[4] >= means[1],
                ', '.join(f'{k} pals {v:.2f}' for k, v in sorted(means.items()))))
    tl = al['total']; fin = al['final']
    fs.append(F('The slot rule beats the card rule on total log-likelihood by more than 1.92 nats', g['allocation: slot over card, nats'] > 1.92,
                f'slots {tl["slots"]:.1f}, cards {tl["cards"]:.1f}: {tl["slots"] - tl["cards"]:+.1f} nats'))

    def where(npal: int, rule: str) -> tuple[str, float, str]:
        o, p_, n, l_, h_ = fin[(npal, rule)]
        return ('inside' if l_ <= p_ <= h_ else 'above' if p_ > h_ else 'below'), p_, f'{npal} pals obs {o:.3f} [{l_:.3f}, {h_:.3f}], {rule} {p_:.3f}'
    w = {(k, r): where(k, r) for k in (0, 2) for r in ('slots', 'cards')}
    fs.append(F('Final links in crowded decks: at 0 pals the slot rule lies inside the observed interval and the card rule outside; at 2 pals '
                'both lie above it, the card rule further',
                w[(0, 'slots')][0] == 'inside' and w[(0, 'cards')][0] != 'inside' and w[(2, 'slots')][0] == 'above'
                and w[(2, 'cards')][0] == 'above' and w[(2, 'cards')][1] > w[(2, 'slots')][1], '; '.join(v[2] for v in w.values())))
    l_, h_ = g['two-chain decks: 95% low'], g['two-chain decks: 95% high']; pr = g['two-chain decks: predicted, P(N >= 6)']
    fs.append(F('Two-chain decks finish both chains as predicted (prediction within the observed interval)', l_ <= pr <= h_,
                f'{g["two-chain decks: share"]:.3f} [{l_:.3f}, {h_:.3f}] vs {pr:.3f}'))
    zs = {k: g[f'early-link shortfall, {k} pals: z'] for k in (4, 3, 2, 1, 0)}
    fs.append(F('Known miss: early links run short of the model (z > 2) at exactly 3, 2, 1 and 0 pals', {k for k, v in zs.items() if v > 2} == {3, 2, 1, 0},
                ', '.join(f'{k} pals z {v:.1f}' for k, v in zs.items())))
    nv = [(nc, *MISC[('never', nc)]) for nc in (5, 6)]; nv = [(nc, k / n, lo, hi, m) for nc, k, n, m, lo, hi in nv]
    fs.append(F('Known miss: more chains never start than predicted with five or six chain cards (model below the observed interval)',
                all(lo > m for _, _, lo, _, m in nv), ', '.join(f'{nc} cards {o:.2f} [{lo:.2f}, {hi:.2f}] vs {m:.2f}' for nc, o, lo, hi, m in nv)))
    fb = MISC['final_by_card']
    fs.append(F('Final links fit when pooled, but cards differ beyond the model: pooled within 0.01, the spread between cards far '
                'beyond chance (p < 0.001)', abs(fb['obs'] - fb['model']) < 0.01 and fb['p'] < 0.001,
                f"pooled {fb['obs']:.3f} vs {fb['model']:.3f}; chi2 {fb['chi2']:.0f} on {fb['cards']} cards, p = {fb['p']:.1g}; lowest "
                + ', '.join(f"{fb['name'](c)} {fb['rate'][c][0]:.2f}/{fb['rate'][c][1]:.2f}" for c, _ in fb['low'])
                + '; highest ' + ', '.join(f"{fb['name'](c)} {fb['rate'][c][0]:.2f}/{fb['rate'][c][1]:.2f}" for c, _ in fb['high'])))
    ga = g['activation, corrected reading, joint with N: gain, nats']
    fs.append(F('A per-card activation parameter is rejected: fitted jointly with N, it gains under 1.92 nats',
                ga < 1.92, f'+{ga:.1f} nats at a = {g["activation, corrected reading, joint with N: a"]:.2f}'))
    return fs


def detail() -> None:
    """the fits' exact figures and the breakdowns behind the known misses (from the run of claims())"""
    if not common.have_runs() or 'fit' not in MISC: return
    f = MISC['fit']
    print(f'\nfit of N ({f["runs"]} runs; by pals {f["by_pals"]}): log-likelihood distribution-free {f["llp"]:.2f}, '
          + ', '.join(f'{nm} {v:.2f}' for nm, v in f['lls'].items()))
    k, n, lo, hi = MISC['two-chain']
    print(f'two plain 3-link SSRs, 4 pals: both finished {k}/{n} = {k / n:.3f} ({lo:.3f}-{hi:.3f}); '
          f'model P(N >= 6) {sum(BIN[6:]):.3f}, with the 3% unseen allowance on each final link {sum(BIN[6:]) * (1 - EPS) ** 2:.3f}')
    print('link 1 not reached, observed / Binomial(21, 0.35), by chain cards: '
          + ', '.join(f'{c}: {1 - o / n:.2f} / {1 - p / n:.2f} (n {n})' for c, (o, p, n) in sorted(MISC['never rows'].items())))
    a = MISC['alloc']
    print('allocation, final links observed / slot rule / card rule: '
          + ', '.join(f'{npal} pals {a["final"][(npal, "slots")][0]:.2f} / {a["final"][(npal, "slots")][1]:.2f} / {a["final"][(npal, "cards")][1]:.2f}' for npal in (5, 4, 3, 2, 1, 0)))
    print('N per pal count: ' + ', '.join(f'{npal} pals {n} runs Binomial({bn}, {bp:.2f}) mean {bn * bp:.2f} variance {bn * bp * (1 - bp):.2f}'
                                         for npal, (n, (_, bn, bp)) in MISC['pal counts'].items()))


if __name__ == '__main__':
    common.main(claims)
    detail()
