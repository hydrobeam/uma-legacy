"""Section 7 (training hints): reproduces docs/RESEARCH.md section 7 from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it) and master.mdb.

One observation per (run, non-pal deck card); its rows are the card's training-hint pool skills (master.mdb
single_mode_hint_gain), a row hit when the run's hint list names the card's character as a source of the skill. Hint
frequency (effect 18), hint levels per draw (1 + effect 17) and specialty priority (effect 19) are read at the card's
level: floored linear interpolation between milestone levels, plus the unique effect.

Every analysis runs on every record, on three row sets:
- the form study: every limit break, R cards included, the rate without the specialty term (h = base_s (1 + hf/100)):
  it compares forms of the hf rate (Bernoulli rows, P(hit) = 1 - (1 - 1/n)^h), the stat option, repeats and distinct
  counts, per-scenario bases, hf by card level and the per-draw increment, and through the Lv-5 cap (uniform draws over
  the skills still below Lv 5, h read as floor or ceiling; pkg/loopopt/src/deck/hints.rs) on its max-limit-break rows
  no spark hinted the cap, joint draws and pool size;
- the fit: max-limit-break card-runs, rows no spark hinted, R cards out, through the Lv-5 cap, with
  h = base_s (1 + hf/100) (1 + d_s spec/100); its later-runs test fits on the earlier half of the records (by seq) and
  scores the later half;
- tests of the current model: the adopted rate (the same h with hints.rs's constants) on the fit's card-runs, for the
  other covariates (a free level per scenario in both fits), the spark deficit (every row), drop-out and priority.
Every model choice is also tested held out: 5-fold cross-validation by common.fold(run), each fold scored by the fits
on the other four, per-run held-out log-likelihoods compared by common.compare_held_out (z > 1.96 = predicts unseen runs
better). Every other result is in-sample on the pooled runs. Intervals and z on observed / predicted over rows (the
spark deficit, priority, the specialty cells) are run-clustered (common.cluster_ci): one run gives many rows; the
specialty cells are tested jointly by a run-clustered Wald chi2 (common.cluster_chi2), their largest |z| only described.
"""
import collections, functools, math, re, unicodedata
from collections.abc import Callable, Sequence
from typing import Any
import numpy as np
from scipy.optimize import minimize, minimize_scalar
import common

SEC: str = '7'
MIL: list[int] = [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50]     # the milestone levels of support_card_effect_table

Rate = Callable[[np.ndarray], np.ndarray]     # parameters -> h per card-run
Mask = np.ndarray                             # a boolean mask over rows
Data = dict[str, Any]                         # build()'s arrays, card-runs and rows


def norm(s: str) -> str:
    return re.sub(r'\s+', ' ', unicodedata.normalize('NFKC', s)).strip()


# ---------------------------------------------------------------- game data
def _game() -> dict[str, Any]:
    if 'g' in common._cache: return common._cache['g']
    db = common.mdb(); q: Callable[[str], list[tuple]] = lambda s: db.execute(s).fetchall()
    lim = {r[0]: r[1:] for r in q('select rarity, limit_0, limit_1, limit_2, limit_3, limit_4 from support_card_limit')}
    eff = collections.defaultdict(dict)
    for row in q('select id, type, init,' + ','.join(f'limit_lv{l}' for l in range(5, 55, 5)) + ' from support_card_effect_table'):
        eff[row[0]][row[1]] = row[2:]
    info = {i: (rar, et) for i, rar, et in q('select id, rarity, effect_table_id from support_card_data')}
    uniq = collections.defaultdict(list)
    for cid, lv, t0, v0, t1, v1 in q('select id, lv, type_0, value_0, type_1, value_1 from support_card_unique_effect'):
        uniq[cid] += [(lv, t0, v0), (lv, t1, v1)]
    sname = dict(q('select [index], text from text_data where category=47'))
    pool = collections.defaultdict(list)
    for cid, gt, v in q('select support_card_id, hint_gain_type, hint_value_1 from single_mode_hint_gain'):
        if gt == 0: pool[cid].append(norm(sname[v]))
    g = common._cache['g'] = dict(lim=lim, eff=eff, info=info, uniq=uniq, pool=pool, cards=common.cards())
    return g


def effect(g: dict[str, Any], cid: int, typ: int, level: int, mode: str = 'lin') -> int:
    """support effect `typ` at `level`: floored linear interpolation between milestones (or the last milestone
    reached, mode 'step'), plus the unique effect once its level is reached"""
    vals = g['eff'][g['info'][cid][1]].get(typ); x = 0
    if vals:
        pts = [(m, v) for m, v in zip(MIL, vals) if v != -1]
        if pts and level >= pts[0][0]:
            if mode == 'step': x = [v for m, v in pts if m <= level][-1]
            else:
                x = pts[-1][1]
                for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
                    if level <= x1: x = y0 + (y1 - y0) * (level - x0) // (x1 - x0); break
    return x + sum(v for lv, t, v in g['uniq'].get(cid, []) if t == typ and level >= lv)


# ---------------------------------------------------------------- observations
def build() -> Data:
    """card-runs and pool-skill rows of every record"""
    key = 'obs'
    if key in common._cache: return common._cache[key]
    g = _game(); C = g['cards']; obs = []
    for r in common.runs():
        hints = [(norm(h['name']), h['lv'], tuple(h['src'])) for h in r['hints']]
        npal = sum(1 for c, _, _ in r['deck'] if C.get(c, {}).get('pal'))
        for cid, lb, nm in r['deck']:
            if cid not in C or C[cid]['pal']: continue
            ch = common.bare(nm); rar = g['info'][cid][0]; level = g['lim'][rar][lb]
            tr = {}; src_of = {}
            for sk, lv, src in hints:
                src_of[sk] = src
                if ch in src: tr[sk] = (lv, src)
            obs.append(dict(run=r['id'], seq=r['seq'], scen=r['scenario'], cid=cid, name=nm, ch=ch, rar=rar, lb=lb,
                            hf=effect(g, cid, 18, level), hf_step=effect(g, cid, 18, level, 'step'), hf_max=C[cid]['hf'],
                            hlv=effect(g, cid, 17, level), spec=effect(g, cid, 19, level), bond0=effect(g, cid, 14, level),
                            hinc=C[cid]['hinc'], nskill=C[cid]['nskill'], opts=C[cid]['opts'], pool=g['pool'][cid],
                            tr=tr, src_of=src_of, prio=r.get('priority'), npal=npal, kind=C[cid]['kind']))
    # rows
    ci, skn, kk, kind, cat = [], [], [], [], []
    for i, x in enumerate(obs):
        inc = 1 + x['hlv']
        for s in x['pool']:
            ci.append(i); skn.append(s)
            others = [t for t in x['src_of'].get(s, ()) if t != x['ch']]
            kinds = {'Inspiration' if t == 'Inspiration' else 'Events' if t == 'Events' else 'Pre-career' if t == 'Pre-career'
                     else 'card event' if t.endswith(' event') else 'other card' for t in others}
            cat.append('+'.join(sorted(kinds)) if kinds else 'none')
            if s not in x['tr']: kk.append(0); kind.append(0); continue
            lv, src = x['tr'][s]
            if src == (x['ch'],) and lv < 5 and lv % inc == 0: kk.append(lv // inc); kind.append(0)
            elif src == (x['ch'],) and lv == 5: kk.append(math.ceil(5 / inc)); kind.append(1)
            else: kk.append(1); kind.append(1)
    D = dict(obs=obs, ci=np.array(ci, int), skn=np.array(skn), kk=np.array(kk), kind=np.array(kind), cat=np.array(cat))
    D['hit'] = (D['kk'] >= 1).astype(float)
    D['noinsp'] = np.array(['Inspiration' not in c for c in cat])
    A: Callable[..., np.ndarray] = lambda k, t=float: np.array([x[k] for x in obs], t)
    for k in ('hf', 'hf_step', 'hf_max', 'spec', 'bond0'): D[k] = A(k)
    for k in ('scen', 'lb', 'rar', 'npal', 'nskill', 'opts', 'hinc'): D[k] = A(k, int)
    D['inc'] = 1 + A('hlv', int)
    D['N'] = len(obs)
    common._cache[key] = D
    D['ctype'] = np.array([x['kind'] for x in obs])
    D['seq'] = A('seq', int)
    return D


# ---------------------------------------------------------------- the draw model
@functools.lru_cache(None)
def miss(k: int, n: int, t: int, cap: int) -> float:
    """P(none of k given pool skills drawn) in t draws, each uniform over the n skills still below Lv 5; a skill
    leaves after `cap` draws (pkg/loopopt/src/deck/hints.rs miss_draws)"""
    if k == 0: return 1.0
    if k >= n: return 1.0 if t == 0 else 0.0
    if t <= cap: return (1 - k / n) ** t
    st = {tuple([n - k] + [0] * cap): 1.0}           # how many of the other skills have taken 0..cap draws
    for _ in range(t):
        nx = collections.defaultdict(float)
        for s, p in st.items():
            avail = n - s[cap]
            for j in range(cap):
                if s[j]:
                    u = list(s); u[j] -= 1; u[j + 1] += 1; nx[tuple(u)] += p * s[j] / avail
        st = nx
    return sum(st.values())


TMAX: int = 12            # the most draws a table covers


def miss_table(k: np.ndarray, n: np.ndarray, cap: np.ndarray) -> np.ndarray:
    """rows x (TMAX + 1): miss(k, n, t, cap) for t = 0..TMAX (computed once per distinct (k, n, cap))"""
    u, inv = np.unique(np.stack([np.asarray(k), np.asarray(n), np.asarray(cap)], 1).astype(int), axis=0, return_inverse=True)
    t = np.array([[miss(int(a), int(b), t, int(c)) for t in range(TMAX + 1)] for a, b, c in u]).reshape(len(u), TMAX + 1)
    return t[inv.ravel()]


def mix(tab: np.ndarray, h: np.ndarray) -> np.ndarray:
    """h read as floor(h) or floor(h) + 1 draws, weighted by the fraction"""
    t0 = np.floor(h).astype(int); f = h - t0; r = np.arange(len(h))
    return (1 - f) * tab[r, t0] + f * tab[r, t0 + 1]


def row_ll(p: np.ndarray, y: np.ndarray) -> np.ndarray:
    """each row's Bernoulli log-likelihood"""
    p = np.clip(p, 1e-12, 1 - 1e-12)
    return np.where(y > 0, np.log(p), np.log(1 - p))


def bern_ll(p: np.ndarray, y: np.ndarray) -> float:
    return row_ll(p, y).sum()


# ---------------------------------------------------------------- classes: rows that a fit predicts alike
# A fit's prediction for a row reads only its card-run's covariates (scenario, hf, specialty, pool size, ...), so rows
# with equal covariates share P(hit): a fit sums hits * log p + misses * log(1 - p) over these classes instead of over
# rows (the same log-likelihood up to rounding, many times fewer terms).
def alike(*cols: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """rows alike in every column: (class of each row, a row of each class)"""
    _, first, inv = np.unique(np.stack([np.asarray(c, float) for c in cols], 1), axis=0, return_index=True, return_inverse=True)
    return inv.ravel(), first


def tally(cls: np.ndarray, y: np.ndarray, rows: Mask, ncls: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """the classes of rows `rows` (a mask): (classes present, hits in each, rows in each)"""
    c = cls[rows]; n = np.bincount(c, minlength=ncls); w = np.nonzero(n)[0]
    return w, np.bincount(c, weights=y[rows], minlength=ncls)[w], n[w].astype(float)


def class_ll(p: np.ndarray, hits: np.ndarray, n: np.ndarray) -> float:
    """Bernoulli log-likelihood of classes with `hits` hits of `n` rows, each row P(hit) = p (clipped as row_ll)"""
    p = np.clip(p, 1e-12, 1 - 1e-12)
    return float(hits @ np.log(p) + (n - hits) @ np.log(1 - p))


def on_rows(w: np.ndarray, v: np.ndarray, cls: np.ndarray, ncls: int) -> np.ndarray:
    """class values v (of classes w) on rows of classes `cls`"""
    full = np.zeros(ncls); full[w] = v
    return full[cls]


# ---------------------------------------------------------------- held out (5-fold cross-validation by run)
CV_FOLDS: list[int] | None = None           # every fold; a list of folds times a subset


def cv(run_rows: np.ndarray, fold_fn: Callable[[Mask, Mask], dict[str, np.ndarray]]) -> dict[str, np.ndarray]:
    """5-fold cross-validation by common.fold(run), a run's rows all in one fold: fold_fn(train, test) fits every model on
    the rows in `train` (boolean masks over the rows) and returns {model: log-likelihood of each row in `test`, in row
    order}. Returns {model: held-out log-likelihood per run}, runs in sorted order"""
    runs = np.unique(run_rows); fo = np.array([common.fold(int(r)) for r in run_rows]); ll = {}
    for f in (range(common.K_FOLDS) if CV_FOLDS is None else CV_FOLDS):
        te = fo == f
        for m, v in fold_fn(~te, te).items(): ll.setdefault(m, np.zeros(len(run_rows)))[te] = v
    ix = np.searchsorted(runs, run_rows)
    return {m: np.bincount(ix, weights=v, minlength=len(runs)) for m, v in ll.items()}


def compared(name: str, a: np.ndarray, b: np.ndarray) -> list[common.Value]:
    """the Values of a held-out comparison, a (the more complex model) minus b, summed over runs: nats, SE, z"""
    s, se, z = common.compare_held_out(a, b)
    return [V(f'held out: {name}: nats', s), V(f'held out: {name}: SE', se), V(f'held out: {name}: z', z)]


def form_classes(D: Data) -> tuple[np.ndarray, np.ndarray]:
    """card-runs alike for every rate of the form study (scenario, hf in its three readings, pool size): (class of each
    card-run, a card-run of each class)"""
    if 'form_classes' not in D: D['form_classes'] = alike(D['scen'], D['hf'], D['hf_step'], D['hf_max'], D['nskill'])
    return D['form_classes']


def fitB(D: Data, hfun: Rate, x0: Sequence[float], rows: Mask | None = None) -> tuple[np.ndarray, float]:
    """the form study's fits: P(row hit) = 1 - (1 - 1/n)^h, h = hfun(theta) per card-run (reading only the scenario and
    hf); Nelder-Mead then L-BFGS-B"""
    ci = D['ci']; cc, rep = form_classes(D)
    rows = np.ones(len(ci), bool) if rows is None else rows
    w, hits, nr = tally(cc[ci], D['hit'], rows, len(rep)); cr = rep[w]; keep = 1 - 1 / D['nskill'][cr].astype(float)
    def nll(th: np.ndarray) -> float:
        h = hfun(th)
        if np.any(h <= 0) or not np.all(np.isfinite(h)): return 1e12
        return -class_ll(1 - keep ** h[cr], hits, nr)
    r = minimize(nll, x0, method='Nelder-Mead', options=dict(xatol=1e-6, fatol=1e-7, maxiter=40000, maxfev=80000))
    r2 = minimize(nll, r.x, method='L-BFGS-B')
    b = r2 if r2.fun <= r.fun else r
    return b.x, -b.fun


def fit_cap(D: Data, rows: Mask, cap: bool = True, pooled: bool = False) -> tuple[dict[int, float], float, np.ndarray]:
    """per-scenario (or pooled) base for h = base (1 + hf/100), Bernoulli through the floor/ceiling mixture of
    miss(1, n, t, cap); cap=False draws with replacement. Returns bases, log-likelihood, per-row P(hit)"""
    ci = D['ci']; j = np.nonzero(rows)[0]; c = ci[j]
    cc, rep = alike(D['scen'], D['hf'], D['nskill'], D['hinc']); rc = cc[c]; nc = len(rep)
    capv = np.ceil(5 / D['hinc'][rep]).astype(int) if cap else np.full(nc, 99)
    tab = miss_table(np.ones(nc, int), D['nskill'][rep], capv)          # per class
    y = D['hit'][j]; g = np.zeros(len(j), int) if pooled else D['scen'][c]
    bases, ll, p = {}, 0.0, np.zeros(len(j))
    for s in sorted(set(g)):
        q = g == s; w, hits, nr = tally(rc, y, q, nc); tq = tab[w]; mult = 1 + D['hf'][rep[w]] / 100
        f = lambda b: -class_ll(1 - mix(tq, b * mult), hits, nr)
        r = minimize_scalar(f, bounds=(0.5, 5), method='bounded', options=dict(xatol=1e-8))
        bases[int(s)] = r.x; ll -= r.fun; p[q] = on_rows(w, 1 - mix(tq, r.x * mult), rc[q], nc)
    return bases, ll, p


def B4(D: Data, th: Sequence[float] | np.ndarray) -> np.ndarray:
    return np.asarray(th[:4])[D['scen'] - 1]


# ---------------------------------------------------------------- values
def V(name: str, got: float | np.number | None, **kw: str) -> common.Value:
    return common.Value(SEC, name, None if got is None else float(got), **kw)


def adopted() -> tuple[dict[int, float], dict[int, float]]:
    """the solver's training-hint bases and specialty weights by scenario (hint_base, hint_spec in
    pkg/loopopt/src/deck/hints.rs)"""
    src = open(f'{common.ROOT}/pkg/loopopt/src/deck/hints.rs', encoding='utf-8').read()

    def table(fn: str) -> dict[int, float]:
        body = re.search(r'fn ' + fn + r'[^{]*\{.*?match scenario \{(.*?)\}', src, re.S).group(1)
        v = {k: float(x) for k, x in re.findall(r'"(\w+)" => ([\d.]+)', body)}
        v['unity'] = float(re.search(r'_ => ([\d.]+)', body).group(1))
        return {1: v['ura'], 2: v['unity'], 3: v['gc'], 4: v['trackblazer'], 5: v['grandmasters']}
    return table('hint_base'), table('hint_spec')


def derived() -> list[common.Value]:
    """the h-reading example: n = 5, h = 2.74; smooth (1 - 1/n)^h against floor/ceiling draws"""
    n, h = 5, 2.74
    caps = [V(f'draws to Lv 5, +{i} card', math.ceil(5 / i), note='derived') for i in range(1, 6)]
    return caps + [V('miss (1-1/n)^h, n=5 h=2.74', (1 - 1 / n) ** h, note='derived'),
            V('miss floor/ceiling, n=5 h=2.74', mix(np.array([[(1 - 1 / n) ** t for t in range(TMAX + 1)]]), np.array([h]))[0], note='derived')]


def claims() -> list[common.Value | common.Finding]:
    if not common.have_runs() or common.mdb() is None:
        return derived()
    out: list[common.Value] = []
    def add(name: str, got: float | np.number | None, **kw: str) -> None: out.append(V(name, got, **kw))

    # ---- the solver's constants (read, not fitted)
    ADOPTED, ADOPTED_D = adopted()
    for s, nm in ((2, 'Unity Cup'), (3, 'Grand Concert'), (1, 'URA Finale'), (4, 'Trackblazer'), (5, 'Grand Masters')):
        add(f'adopted {nm}', ADOPTED[s], note='the solver\'s hint_base (pkg/loopopt/src/deck/hints.rs)')
        add(f'adopted d {nm}', ADOPTED_D[s], note='the solver\'s hint_spec')

    # ---- a worked example: the median card in the runs' decks (SR/SSR, not pal or group), at maximum limit break
    wc = common.cards(); deckc = [wc[c] for r in common.runs() for c, _, _ in r['deck'] if c in wc and wc[c]['rarity'] > 1 and not wc[c]['pal'] and wc[c].get('kind') != 'group']
    mhf = float(np.median([c['hf'] for c in deckc])); msp = float(np.median([c['spec'] for c in deckc]))
    add('example: median hint frequency', mhf, note=f'{len(deckc):,} deck cards in the runs'); add('example: median specialty priority', msp)
    for s_, nm in ((2, 'Unity Cup'), (3, 'Grand Concert'), (1, 'URA Finale'), (4, 'Trackblazer')):
        add(f'example: hints per career, median card, {nm}', ADOPTED[s_] * (1 + mhf / 100) * (1 + ADOPTED_D[s_] * msp / 100))
        add(f'example: hints per career, no effects, {nm}', ADOPTED[s_])

    # ---- the form study: every record, every limit break, no specialty term
    D = build(); ci = D['ci']; hit = D['hit']; NI = D['noinsp']; hf = D['hf']; n = D['nskill'].astype(float)
    sc = D['scen']; max4 = D['lb'][ci] == 4
    add('form: rows', len(ci)); add('form: rows without a spark', NI.sum())
    add('form: runs', len({D['obs'][i]['run'] for i in ci}), note='runs that add a row')
    llB: Callable[..., float] = lambda h, nn=n: bern_ll(1 - (1 - 1 / nn[ci]) ** h[ci], hit)
    add('LL 1+hf/25 over options', llB(1 + hf / 25, D['opts'].astype(float)))
    add('LL 1+hf/25 over skills', llB(1 + hf / 25))
    add('LL constant', fitB(D, lambda th: th[0] + 0 * hf, [2.])[1])
    ll1 = fitB(D, lambda th: th[0] * (1 + hf / 100), [2.])[1]; add('LL base(1+hf/100)', ll1)
    th4, ll4 = fitB(D, lambda th: B4(D, th) * (1 + hf / 100), [1.5, 2.6, 2., 1.7]); add('LL per-scenario base(1+hf/100)', ll4)
    thc, llc = fitB(D, lambda th: B4(D, th) * (1 + hf / th[4]), list(th4) + [100.])
    add('LL free divisor', llc); add('free divisor c', thc[4])

    # skills only: per (card, hf) with >= 15 card-runs whose every row is exact, var(T) against the floor a drawn
    # stat option implies (E = mean T (o / n) draws over o = n + 1 options give var >= E (1/o)(1 - 1/o))
    tot = np.bincount(ci, weights=D['kk'], minlength=D['N']); cens = np.bincount(ci, weights=D['kind'], minlength=D['N'])
    cid = np.array([x['cid'] for x in D['obs']]); o = D['opts'].astype(float); cells = below_n = 0
    for c in sorted(set(cid)):
        for v in sorted(set(hf[cid == c])):
            m = (cid == c) & (hf == v) & (cens == 0)
            if m.sum() < 15: continue
            T = tot[m]; oo = o[m][0]; cells += oo > 1
            if oo <= 1: continue    # an empty pool (SSR Haru Urara) can't be tested
            below_n += T.var() < T.mean() / (oo - 1) * (1 - 1 / oo)
    add('cells var below stat-option floor', below_n)
    add('cells (card, hf)', cells, note='testable cells (an empty pool is left out)')

    # repeats and distinct counts (bases: per-scenario, rows no spark hinted, every limit break)
    bs, ll_ni = fitB(D, lambda th: B4(D, th) * (1 + hf / 100), [1.5, 2.7, 2., 1.8], rows=NI)
    h = B4(D, bs) * (1 + hf / 100)
    inc, kk, kind = D['inc'][ci], D['kk'], D['kind']
    k1 = (kind == 0) & (kk == 1); k2 = ((kind == 0) & (kk >= 2)) | ((kind == 1) & (kk >= 2))
    use = (k1 | k2) & (inc <= 4); p = 1 / n[ci]; hh = h[ci]; f0 = np.floor(hh); fr = hh - f0
    q: Callable[[np.ndarray], tuple[np.ndarray, np.ndarray]] = lambda t: (1 - (1 - p) ** t, 1 - (1 - p) ** t - t * p * (1 - p) ** (t - 1))      # P(k >= 1), P(k >= 2)
    (a1, a2), (b1, b2) = q(f0), q(f0 + 1)
    r_two = ((1 - fr) * a2 + fr * b2) / ((1 - fr) * a1 + fr * b1)
    mu = hh / n[ci]; r_poi = 1 - mu * np.exp(-mu) / (1 - np.exp(-mu))
    add('repeat rate observed', k2[use].mean()); add('repeat rate two-point count', r_two[use].mean())
    add('repeat rate Poisson count', r_poi[use].mean())
    has = n > 0; dist = np.bincount(ci, weights=hit, minlength=D['N']).astype(int)
    model = np.zeros(TMAX + 1)
    for i in np.nonzero(has)[0]:          # distinct skills after t uniform draws over n: C(n,d) d! S(t,d) / n^t
        t0 = int(h[i]); w = h[i] - t0
        for t, wt in ((t0, 1 - w), (t0 + 1, w)):
            model[:t + 1] += wt * distinct_pmf(int(n[i]), t)
    model /= has.sum()
    for k in (1, 2, 3, 4):
        add(f'distinct {k} observed', np.mean(dist[has] == k))
        add(f'distinct {k} model', model[k], note='exact')

    # per-scenario bases against one base (form study)
    add('per-scenario bases gain, nats', ll4 - ll1)

    # the form study's rate forms held out: each fold's fits on the other four folds' rows, every row scored
    one: Rate = lambda th: th[0] * (1 + hf / 100); per: Rate = lambda th: B4(D, th) * (1 + hf / 100)
    free: Rate = lambda th: B4(D, th) * (1 + hf / th[4]); const: Rate = lambda th: th[0] + 0 * hf
    sc_h: Callable[..., np.ndarray] = lambda h, te, nn=n: row_ll(1 - (1 - 1 / nn[ci][te]) ** h[ci][te], hit[te])

    def fold_form(tr: Mask, te: Mask) -> dict[str, np.ndarray]:
        t4 = fitB(D, per, [1.5, 2.6, 2., 1.7], rows=tr)[0]
        return {'one base': sc_h(one(fitB(D, one, [2.], rows=tr)[0]), te), 'per-scenario': sc_h(per(t4), te),
                'free divisor': sc_h(free(fitB(D, free, list(t4) + [100.], rows=tr)[0]), te),
                'constant': sc_h(const(fitB(D, const, [2.], rows=tr)[0]), te),
                '1+hf/25 over options': sc_h(1 + hf / 25, te, D['opts'].astype(float)), '1+hf/25 over skills': sc_h(1 + hf / 25, te)}
    hof = cv(np.array([x['run'] for x in D['obs']])[ci], fold_form)
    add('held out: form study runs', len(hof['per-scenario']))
    out.extend(compared('free divisor vs 1/100', hof['free divisor'], hof['per-scenario']))
    out.extend(compared('per-scenario bases vs one base', hof['per-scenario'], hof['one base']))
    for m in ('constant', '1+hf/25 over options', '1+hf/25 over skills'):
        out.extend(compared(f'per-scenario bases vs {m}', hof['per-scenario'], hof[m]))

    # the Lv-5 cap against drawing with replacement (max limit break, rows no spark hinted)
    rows = max4 & NI
    _, llc_, pc = fit_cap(D, rows); _, llr_, pr_ = fit_cap(D, rows, cap=False)
    add('cap gain, nats', llc_ - llr_)
    j = np.nonzero(rows)[0]; capd = np.ceil(5 / D['hinc'][ci[j]]).astype(int)
    for d in (5, 3, 2, 1):
        q_ = capd == d
        add(f'cap table {d} draws rows', q_.sum())
        add(f'cap table {d} draws replacement', hit[j][q_].sum() / pr_[q_].sum())
        add(f'cap table {d} draws cap', hit[j][q_].sum() / pc[q_].sum())

    # joint draws: pairs of no-spark pool skills on one max-limit-break card-run (URA, Unity, Grand Concert)
    m = rows & (sc[ci] != 4); by = collections.defaultdict(list)
    for r_ in np.nonzero(m)[0]: by[ci[r_]].append(r_)
    P_ = np.array([(i, a, b) for i, js in by.items() for x, a in enumerate(js) for b in js[x + 1:]])
    I, a, b = P_.T; nI = n[I]; hI = h[I]
    m1, m2 = (1 - 1 / nI) ** hI, (1 - 2 / nI) ** hI
    add('joint pairs', len(P_)); add('P(both) observed', np.mean(hit[a] * hit[b]))
    add('P(both) joint', np.mean(1 - 2 * m1 + m2)); add('P(both) independent', np.mean((1 - m1) ** 2))
    add('P(neither) observed', np.mean((1 - hit[a]) * (1 - hit[b]))); add('P(neither) joint', np.mean(m2))

    # pool size: obs / pred by n (cap fit above), Unity Cup and Grand Concert, n with >= 150 rows
    rat = []
    for s in (2, 3):
        for v in sorted(set(n[ci[j]])):
            q_ = (n[ci[j]] == v) & (sc[ci[j]] == s)
            if q_.sum() >= 150: rat.append(hit[j][q_].sum() / pc[q_].sum())
    note = 'Lv-5 cap, max limit break, no spark, per (scenario, n)'
    add('pool size obs/pred min', min(rat), note=note); add('pool size obs/pred max', max(rat), note=note)

    # per-draw increment: levels from this card's training alone
    ok = tot_ = 0
    for x in D['obs']:
        for lv, src in x['tr'].values():
            if src == (x['ch'],): tot_ += 1; ok += (lv % (1 + x['hlv']) == 0) or lv == 5
    add('increment: multiples of iota', ok); add('increment: single-source levels', tot_)

    # hint frequency by card level (rows no spark hinted)
    add('hf max in the data', hf[n > 0].max(), note=f'min {hf[n > 0].min():.0f}')
    add('LL hf floored-linear', ll_ni)
    for nm, H in (('LL hf step', D['hf_step']), ('LL hf max-LB for all', D['hf_max'])):
        add(nm, fitB(D, lambda th, H=H: B4(D, th) * (1 + H / 100), list(bs), rows=NI)[1])

    # ---- tests of the current model: the adopted rate (hints.rs constants, specialty term in) through the Lv-5 cap, on
    # max-limit-break card-runs with R cards out (the fit's card-runs; spark-hinted rows only where a test reads them)
    M = max4 & (D['rar'][ci] != 1); jm = np.nonzero(M)[0]; cm = ci[jm]
    tabm = miss_table(np.ones(len(jm), int), D['nskill'][cm], np.ceil(5 / D['hinc'][cm]).astype(int))
    bA = np.array([ADOPTED[s] for s in (1, 2, 3, 4)]); dA = np.array([ADOPTED_D[s] for s in (1, 2, 3, 4)])
    hA = bA[sc - 1] * (1 + hf / 100) * (1 + dA[sc - 1] * D['spec'] / 100)           # per card-run
    clip: Callable[[np.ndarray], np.ndarray] = lambda x: np.minimum(x, TMAX - 1e-6)
    pA = np.full(len(ci), np.nan); pA[jm] = 1 - mix(tabm, clip(hA[cm]))
    note = 'the adopted rate, Lv-5 cap, max limit break, R cards out'

    # covariates: the adopted rate times exp(X theta), a free level per scenario in both fits (so a covariate cannot
    # gain by absorbing an offset of the adopted bases), rows no spark hinted
    nim = NI[jm]; yk = hit[jm][nim]; tk = tabm[nim]; hk = hA[cm][nim]; sk = sc[cm][nim] - 1; ck = cm[nim]

    p_cov: Callable[[np.ndarray, np.ndarray, Mask], np.ndarray] = lambda th, X, q: 1 - mix(tk[q], clip(hk[q] * np.exp(th[:4][sk[q]] + X[q] @ th[4:])))
    TY = ['stamina', 'power', 'guts', 'wit']                     # against speed
    covs = {'card type': np.stack([(D['ctype'] == t).astype(float) for t in TY], 1), 'SR': (D['rar'] == 2).astype(float)[:, None],
            'pals per deck': (D['npal'] - 2.5)[:, None].astype(float), 'initial bond': (D['bond0'] / 10)[:, None],
            'pool size': (n - 8)[:, None]}
    kc, krep = alike(sk, hk, D['nskill'][ck], D['hinc'][ck], *np.concatenate([X[ck] for X in covs.values()], 1).T)

    def fit_cov(X: np.ndarray, q: Mask | None = None, theta: bool = False) -> np.ndarray | tuple[float, np.ndarray]:
        """X: the covariates of each row (columns of covs[...][ck])"""
        k = X.shape[1]; q = np.ones(len(yk), bool) if q is None else q
        w, hits, nr = tally(kc, yk, q, len(krep)); r_ = krep[w]; tw_, hw_, sw_, Xw = tk[r_], hk[r_], sk[r_], X[r_]
        nll = lambda th: -class_ll(1 - mix(tw_, clip(hw_ * np.exp(th[:4][sw_] + Xw @ th[4:]))), hits, nr)
        r = minimize(nll, np.zeros(4 + k), method='Nelder-Mead', options=dict(xatol=1e-6, fatol=1e-7, maxiter=40000, maxfev=80000))
        r2 = minimize(nll, r.x, method='L-BFGS-B'); b = r2 if r2.fun <= r.fun else r
        return b.x if theta else (-b.fun, np.exp(b.x[4:]))
    l0, _ = fit_cov(np.zeros((len(ck), 0))); facs = []
    add('covariate rows', nim.sum(), note=note)
    for nm, X in covs.items():
        ll_, f_ = fit_cov(X[ck]); facs += list(f_)
        add(f'covariate gain {nm}, nats', ll_ - l0, note=f'{X.shape[1]} parameter(s), factor(s) ' + ', '.join(f'{x:.3f}' for x in f_))
        if X.shape[1] == 1: add(f'covariate factor {nm}', f_[0])
    add('covariate factor min', min(facs)); add('covariate factor max', max(facs))
    # the detectable covariates held out: the adopted rate with a free level per scenario, with and without each
    held = {'none': np.zeros((len(ck), 0)), 'SR': covs['SR'][ck], 'pals per deck': covs['pals per deck'][ck]}
    hoc = cv(np.array([x['run'] for x in D['obs']])[ck],
             lambda tr, te: {m: row_ll(p_cov(fit_cov(X, tr, theta=True), X, te), yk[te]) for m, X in held.items()})
    add('held out: covariate runs', len(hoc['none']))
    for nm in ('SR', 'pals per deck'):
        out.extend(compared(f'covariate {nm} vs none', hoc[nm], hoc['none']))

    # spark deficit by the other sources of the same skill (every row of those card-runs); the observed rate's 95% interval
    # is the predicted rate times the run-clustered interval of obs / pred (common.cluster_ci: a run gives several rows)
    cat = D['cat']; rrun = np.array([x['run'] for x in D['obs']])[ci]

    def obs_ci(q_: Mask, ratio: bool = False) -> tuple[float, ...]:
        r_, lo_, hi_, _ = common.cluster_ci(hit[q_], pA[q_], rrun[q_]); e_ = pA[q_].mean()
        return (r_, lo_, hi_) if ratio else (lo_ * e_, hi_ * e_)
    for c, key in (('none', 'none'), ('inspiration', 'Inspiration'), ('inspiration + card event', 'Inspiration+card event'),
                   ('card event', 'card event'), ('other card training', 'other card')):
        q_ = M & (cat == key)
        add(f'spark {c} rows', q_.sum(), note=note); add(f'spark {c} observed', hit[q_].mean())
        add(f'spark {c} predicted', pA[q_].mean()); add(f'spark {c} ratio', hit[q_].mean() / pA[q_].mean())
        lo_, hi_ = obs_ci(q_); add(f'spark {c} observed 95% low', lo_); add(f'spark {c} observed 95% high', hi_)
        if c == 'none':
            r_, lo_, hi_ = obs_ci(q_, True); add('spark none obs/pred', r_)
            add('spark none obs/pred 95% low', lo_); add('spark none obs/pred 95% high', hi_)
    # drop-out: card-runs with exactly two pool skills spark-hinted, their other (no-spark) rows
    ins = np.bincount(ci, weights=(~NI).astype(float), minlength=D['N'])[ci]
    jj = np.nonzero(NI & M & (ins == 2))[0]; c_ = ci[jj]
    capj = np.ceil(5 / D['hinc'][c_]).astype(int); hj = clip(hA[c_])
    keep = 1 - mix(miss_table(np.ones(len(jj), int), D['nskill'][c_], capj), hj)
    drop = 1 - mix(miss_table(np.ones(len(jj), int), D['nskill'][c_] - 2, capj), hj)
    add('drop-out test rows', len(jj), note=note); add('drop-out test observed', hit[jj].mean())
    add('drop-out test no drop-out', keep.mean()); add('drop-out test drop-out', drop.mean())

    out += specialty(D)

    # prioritised skills
    prio = [None if x['prio'] is None else {norm(s) for s in x['prio']} for x in D['obs']]
    isp = np.array([prio[ci[r_]] is not None and D['skn'][r_] in prio[ci[r_]] for r_ in range(len(ci))]) & NI & M
    add('priority runs', len({x['run'] for x in D['obs'] if x['prio'] is not None}))
    add('priority rows', isp.sum()); add('priority observed', hit[isp].mean())
    add('priority predicted', pA[isp].mean(), note=note)
    lo_, hi_ = obs_ci(isp); add('priority observed 95% low', lo_); add('priority observed 95% high', hi_)
    return derived() + out + findings(out)


def findings(out: list[common.Value]) -> list[common.Finding]:
    """the section's conclusions, each tested on the numbers above"""
    from scipy.stats import binom, chi2
    g: dict[str, Any] = {c.name: c.got for c in out}; F: Callable[..., common.Finding] = lambda *a: common.finding(SEC, *a)
    fs: list[common.Finding] = []
    lps = g['LL per-scenario base(1+hf/100)']
    H: Callable[[str], tuple[float, float, float]] = lambda c: (g[f'held out: {c}: nats'], g[f'held out: {c}: SE'], g[f'held out: {c}: z'])
    hs: Callable[[str], str] = lambda c: '{:+.1f} +- {:.1f} nats, z = {:.2f}'.format(*H(c))
    rf = ('constant', '1+hf/25 over options', '1+hf/25 over skills')
    fs.append(F('h = beta_s (1 + hf/100) predicts unseen runs better than the other rate forms tried without a free slope (a constant, '
                '1 + hf/25 over options or over skills; 5-fold held out, each z > 1.96)', all(H(f'per-scenario bases vs {m}')[2] > 1.96 for m in rf),
                '; '.join(f'against {m} ' + hs(f'per-scenario bases vs {m}') for m in rf) + f' on {g["held out: form study runs"]:,.0f} runs'))
    gc = g['LL free divisor'] - lps
    fs.append(F('Known miss (form study, without the specialty term): the slope 1/100 is assumed; a free divisor is detectable (LR gain above 1.92 nats, the 1-parameter 5% '
                'bound) and predicts unseen runs better than 1/100 (5-fold held out, z > 1.96)', gc > 1.92 and H('free divisor vs 1/100')[2] > 1.96,
                f'c = {g["free divisor c"]:.0f}; in-sample +{gc:.1f} nats; held out ' + hs('free divisor vs 1/100')
                + f' on {g["held out: form study runs"]:,.0f} runs'))
    k, n = g['cells var below stat-option floor'], g['cells (card, hf)']
    fs.append(F('The stat option is never drawn: the draw-count variance is below the floor a drawn stat option implies in most (card, hf) '
                'cells (sign test p < 0.05)', binom.sf(k - 1, n, 0.5) < 0.05, f'{k:.0f} of {n:.0f} cells, p = {binom.sf(k - 1, n, 0.5):.4f}'))
    o, t2, po = g['repeat rate observed'], g['repeat rate two-point count'], g['repeat rate Poisson count']
    fs.append(F('Draws repeat skills as uniform draws with a near-fixed count do, not as a Poisson count', abs(o - t2) < abs(o - po),
                f'repeat rate observed {o:.3f}, two-point {t2:.3f}, Poisson {po:.3f}'))
    gb = g['per-scenario bases gain, nats']
    fs.append(F('Bases differ by scenario (per-scenario bases against one base, LR p < 0.001 on 3 parameters)', chi2.sf(2 * gb, 3) < 0.001,
                f'+{gb:.0f} nats, LR p = {chi2.sf(2 * gb, 3):.1e}'))
    fs.append(F('Per-scenario bases predict unseen runs better than one base (form study, 5-fold held out, z > 1.96)',
                H('per-scenario bases vs one base')[2] > 1.96, hs('per-scenario bases vs one base')))
    cv = {nm: (g[f'covariate gain {nm}, nats'], chi2.sf(2 * g[f'covariate gain {nm}, nats'], k_))
          for nm, k_ in (('card type', 4), ('SR', 1), ('pals per deck', 1), ('initial bond', 1), ('pool size', 1))}
    miss_c, none_c = ('SR', 'pals per deck'), ('card type', 'initial bond', 'pool size')
    fs.append(F('Known miss (not modelled): rarity (SR against SSR) and pals per deck are detectable on top of the adopted rate '
                '(each LR p < 0.05)', all(cv[nm][1] < 0.05 for nm in miss_c),
                '; '.join(f'{nm} +{cv[nm][0]:.1f} nats, p = {cv[nm][1]:.1e}' for nm in miss_c)))
    fs.append(F('Known miss (not modelled): rarity (SR against SSR) and pals per deck each predict unseen runs better than the adopted '
                'rate without them (5-fold held out, each z > 1.96)', all(H(f'covariate {nm} vs none')[2] > 1.96 for nm in miss_c),
                '; '.join(f'{nm} ' + hs(f'covariate {nm} vs none') for nm in miss_c) + f' on {g["held out: covariate runs"]:,.0f} runs'))
    fs.append(F('Card type, initial bond and pool size add nothing significant to the adopted rate (each LR p > 0.05)',
                all(cv[nm][1] > 0.05 for nm in none_c), '; '.join(f'{nm} +{cv[nm][0]:.1f} nats, p = {cv[nm][1]:.3f}' for nm in none_c)))
    fs.append(F('The Lv-5 cap (a skill leaves the pool at Lv 5) beats drawing with replacement, with no parameter',
                g['cap gain, nats'] > 1.92, f'+{g["cap gain, nats"]:.1f} nats on {g["form: runs"]:,.0f} runs'))
    b, j, i = g['P(both) observed'], g['P(both) joint'], g['P(both) independent']
    fs.append(F('Two targets in one card\'s pool land jointly as the draws imply, not independently', abs(b - j) < abs(b - i),
                f'P(both) observed {b:.4f}, joint {j:.4f}, independent {i:.4f} ({g["joint pairs"]:,.0f} pairs)'))
    m, t = g['increment: multiples of iota'], g['increment: single-source levels']
    fs.append(F('Each draw adds the card\'s hint levels (at least 99.5% of single-source levels are multiples of it, capped at 5)',
                m / t >= 0.995, f'{m:,.0f} of {t:,.0f}'))
    fl, stp, mx = g['LL hf floored-linear'], g['LL hf step'], g['LL hf max-LB for all']
    fs.append(F('hf at a card\'s level is the floored linear interpolation between milestone levels (it fits at least as well as a step '
                'or the max-limit-break value)', fl >= stp and fl >= mx, f'floored-linear {fl:.1f}, step {stp:.1f}, max-LB {mx:.1f}'))
    sp = {}
    for c in ('none', 'inspiration', 'card event', 'other card training'):
        ob, pr, lo, hi = (g[f'spark {c} {k}'] for k in ('observed', 'predicted', 'observed 95% low', 'observed 95% high'))
        sp[c] = (lo <= pr <= hi, ob, pr, lo, hi)
    v = sp['none']
    fs.append(F('With no other source of the skill the adopted rate slightly under-predicts (the run-clustered interval lies entirely '
                'above the prediction)', v[3] > v[2],
                f'{v[1]:.3f} [{v[3]:.3f}, {v[4]:.3f}] vs {v[2]:.3f}; obs/pred {g["spark none obs/pred"]:.3f} '
                f'[{g["spark none obs/pred 95% low"]:.3f}, {g["spark none obs/pred 95% high"]:.3f}]'))
    fs.append(F('A card\'s own event or another card\'s training leaves the adopted rate calibrated (prediction inside the interval)',
                sp['card event'][0] and sp['other card training'][0],
                '; '.join(f'{c} {v[1]:.3f} [{v[3]:.3f}, {v[4]:.3f}] vs {v[2]:.3f}' for c, v in sp.items() if c in ('card event', 'other card training'))))
    v = sp['inspiration']
    fs.append(F('Known miss (not modelled): a skill also hinted by inspiration is hinted by training less than predicted', v[4] < v[2],
                f'{v[1]:.3f} [{v[3]:.3f}, {v[4]:.3f}] vs {v[2]:.3f}'))
    o, nd, dd = g['drop-out test observed'], g['drop-out test no drop-out'], g['drop-out test drop-out']
    fs.append(F('Spark-hinted skills do not drop out of the draws (observed nearer no drop-out than full drop-out)', abs(o - nd) < abs(o - dd),
                f'observed {o:.3f}, no drop-out {nd:.3f}, drop-out {dd:.3f}'))
    gs = g['specialty gain per scenario, nats']
    fs.append(F('Specialty priority raises training hints: (1 + d_s sp/100) is detectable (4 parameters, LR p < 0.001)',
                chi2.sf(2 * gs, 4) < 0.001, f'+{gs:.1f} nats, LR p = {chi2.sf(2 * gs, 4):.1e}'))
    fs.append(F('The specialty term (a d per scenario) predicts unseen runs better than no term (5-fold held out, z > 1.96)',
                H('specialty term vs none')[2] > 1.96, hs('specialty term vs none') + f' on {g["held out: specialty runs"]:,.0f} runs'))
    fs.append(F('A d per scenario predicts unseen runs better than one pooled d (5-fold held out, z > 1.96)',
                H('per-scenario d vs one d')[2] > 1.96, hs('per-scenario d vs one d') + '; one d against none ' + hs('one d vs none')))
    bf = [(b_, g[f'specialty band {b_} before'], g[f'specialty band {b_} after']) for b_ in ('0', '1-29', '30-59', '60+')]
    fs.append(F('With it the residual by specialty is flat: every band within 0.98-1.02',
                all(0.98 <= a_ <= 1.02 for _, _, a_ in bf), '; '.join(f'{b_} {x:.3f} -> {y:.3f}' for b_, x, y in bf)))
    x2, dfc, pc = g['specialty cells: joint chi2'], g['specialty cells: df'], g['specialty cells: p']
    fs.append(F('After the specialty term the (scenario, band) cells with 200+ rows are jointly consistent with the model (run-clustered '
                'Wald chi2, p > 0.05)', pc > 0.05, f'chi2 {x2:.1f} on {dfc:.0f} cells, p = {pc:.3f}; largest single |z| {g["specialty cells max |z| after"]:.2f} (descriptive)'))
    fs.append(F('The specialty effect is not an artefact of one card, one rarity or one period: one pooled d stays positive and within '
                'a factor 2 everywhere', 0 < g['specialty d robustness min'] and g['specialty d robustness max'] < 2 * g['specialty d robustness min'],
                f'd {g["specialty d robustness min"]:.2f}-{g["specialty d robustness max"]:.2f}'))
    gd = g['specialty model: free divisor gain, nats']
    fs.append(F('On the adopted model (a base and d per scenario, the specialty fit\'s rows) a free divisor is detectable in-sample (LR '
                'gain above 1.92 nats, the 1-parameter 5% bound) but does not predict unseen runs better than 1/100 (5-fold held out, '
                'z <= 1.96): 1/100 stays', gd > 1.92 and H('specialty model, free divisor vs 1/100')[2] <= 1.96,
                f"c = {g['specialty model: free divisor c']:.0f}; in-sample +{gd:.2f} nats, LR p = {chi2.sf(2 * gd, 1):.3f}; held out "
                + hs('specialty model, free divisor vs 1/100') + f" on {g['held out: specialty runs']:,.0f} runs"))
    fs.append(F('Later runs: fitted on the earlier half of the records, the specialty term predicts the later half better than no term '
                '(z > 1.96; the gain smaller than in-sample)', H('later runs: specialty term vs none')[2] > 1.96
                and g['specialty held-out gain, nats'] < g['specialty gain per scenario, nats'],
                'held out ' + hs('later runs: specialty term vs none') + f" on {g['specialty held-out rows']:,.0f} rows; in-sample +{g['specialty gain per scenario, nats']:.1f}"))
    fs.append(F('Not shown to be causal: within the same card across limit breaks the estimate agrees in size with the across-card '
                'one but is not significant (gain under 1.92 nats)', g['specialty same-card: gain, nats'] < 1.92 and g['specialty same-card: d'] > 0,
                f"same-card d {g['specialty same-card: d']:.2f} on {g['specialty same-card: cards']:.0f} cards, +{g['specialty same-card: gain, nats']:.1f} nats; "
                f"across cards d {g['specialty d URA Finale']:.2f} URA Finale, {g['specialty d Unity Cup']:.2f} Unity Cup"))
    rows, ob, pr = g['priority rows'], g['priority observed'], g['priority predicted']; lo, hi = g['priority observed 95% low'], g['priority observed 95% high']
    fs.append(F('No evidence that prioritising a skill raises its training hints (the observed rate is not above the prediction)',
                ob <= pr or lo <= pr, f'{ob:.3f} [{lo:.3f}, {hi:.3f}] vs {pr:.3f} ({rows:.0f} rows); a weak test'))
    return fs


def specialty(A: Data) -> list[common.Value]:
    """specialty priority (effect 19, at the card's level) on the fit's setup over every record: max-limit-break
    card-runs, rows no spark hinted, Lv-5 cap, R cards out. h = base_s (1 + hf/100) (1 + d spec/100)."""
    ci = A['ci']; rows = (A['lb'][ci] == 4) & A['noinsp'] & (A['rar'][ci] != 1)
    j = np.nonzero(rows)[0]; c = ci[j]
    tab = miss_table(np.ones(len(j), int), A['nskill'][c], np.ceil(5 / A['hinc'][c]).astype(int))
    y = A['hit'][j]; hfm = 1 + A['hf'][c] / 100; sp = A['spec'][c]; sc = A['scen'][c]
    cid = np.array([x['cid'] for x in A['obs']])[c]; seq = A['seq'][c]; rar = A['rar'][c]
    out: list[common.Value] = []; add: Callable[..., None] = lambda name, got, **kw: out.append(V(name, got, **kw))

    # every multiplier below is a function of hf and specialty: rows alike in those, the pool size and the increment
    # (the draw table) are predicted alike
    rc, rep = alike(A['hf'][c], sp, A['nskill'][c], A['hinc'][c]); tabc = tab[rep]
    tallies: dict[bytes, tuple[np.ndarray, np.ndarray, np.ndarray]] = {}

    def fit(q: Mask, m: np.ndarray, rows: bool = False) -> tuple[float, float, np.ndarray | None]:
        """one scenario's base for multiplier m: (LL, base, P(hit) per row of q if `rows`, else None)"""
        key = q.tobytes()
        if key not in tallies: tallies[key] = tally(rc, y, q, len(rep))
        w, hits, nr = tallies[key]; tq = tabc[w]; mq = m[rep[w]]
        f = lambda b: -class_ll(1 - mix(tq, np.minimum(b * mq, TMAX - 1e-6)), hits, nr)
        b = minimize_scalar(f, bounds=(0.3, 6), method='bounded', options=dict(xatol=1e-7)).x
        return -f(b), b, on_rows(w, 1 - mix(tq, np.minimum(b * mq, TMAX - 1e-6)), rc[q], len(rep)) if rows else None

    def total(m: np.ndarray, q: Mask | None = None) -> float:
        q = np.ones(len(y), bool) if q is None else q
        return sum(fit(q & (sc == k), m)[0] for k in (1, 2, 3, 4) if (q & (sc == k)).any())

    def pooled_d(q: Mask | None = None) -> tuple[float, float]:
        r = minimize_scalar(lambda d: -total(hfm * (1 + d * sp / 100), q), bounds=(-0.5, 1.5), method='bounded', options=dict(xatol=1e-4))
        return r.x, -r.fun - total(hfm, q)
    add('specialty rows', len(j)); add('specialty card-runs', len(set(c)))
    run = np.array([A['obs'][i]['run'] for i in c])
    for k, nm in ((1, 'URA Finale'), (2, 'Unity Cup'), (3, 'Grand Concert'), (4, 'Trackblazer')):
        add(f'specialty rows {nm}', (sc == k).sum()); add(f'specialty runs {nm}', len(set(run[sc == k])))
    d_s, p0, p1, bases, l1s = {}, np.zeros(len(y)), np.zeros(len(y)), {}, []
    gain_s = 0.0
    for k in (1, 2, 3, 4):
        q = sc == k
        r = minimize_scalar(lambda d: -fit(q, hfm * (1 + d * sp / 100))[0], bounds=(-0.5, 1.5), method='bounded', options=dict(xatol=1e-5))
        l1, bases[k], p1[q] = fit(q, hfm * (1 + r.x * sp / 100), True); l0, _, p0[q] = fit(q, hfm, True)
        d_s[k] = r.x; gain_s += l1 - l0; l1s.append(l1)
    add('specialty gain per scenario, nats', gain_s)
    for k, nm in ((1, 'URA Finale'), (2, 'Unity Cup'), (3, 'Grand Concert'), (4, 'Trackblazer')):
        add(f'specialty d {nm}', d_s[k]); add(f'specialty base {nm}', bases[k])
    dp, gp = pooled_d(); add('specialty gain one d, nats', gp); add('specialty d pooled', dp)
    allq = np.ones(len(y), bool)
    r1 = minimize_scalar(lambda d: -fit(allq, hfm * (1 + d * sp / 100))[0], bounds=(-0.5, 1.5), method='bounded', options=dict(xatol=1e-5))
    add('one base and one d: base', fit(allq, hfm * (1 + r1.x * sp / 100))[1]); add('one base and one d: d', r1.x)
    ge = sum(-minimize_scalar(lambda g: -fit(sc == k, hfm * np.exp(g * sp / 10))[0], bounds=(-0.1, 0.1), method='bounded').fun
             - fit(sc == k, hfm)[0] for k in (1, 2, 3, 4)); add('specialty gain exp form, nats', ge)
    bands = ((0, 1, '0'), (1, 30, '1-29'), (30, 60, '30-59'), (60, 1000, '60+'))
    band = {}
    for a_, b_, nm in bands:
        q = (sp >= a_) & (sp < b_); add(f'specialty band {nm} rows', q.sum())
        add(f'specialty band {nm} before', y[q].sum() / p0[q].sum()); add(f'specialty band {nm} after', y[q].sum() / p1[q].sum())
    zcell = []; cres, crun = [], []
    for k in (1, 2, 3, 4):
        for a_, b_, nm in bands:
            q = (sc == k) & (sp >= a_) & (sp < b_)
            if q.sum() >= 200:     # z of obs / pred against 1, its standard error run-clustered (common.cluster_ci)
                r_, _, hi_, _ = common.cluster_ci(y[q], p1[q], run[q]); zcell.append((r_ - 1) / ((hi_ - r_) / 1.96))
                cres += [((k, nm), d_) for d_ in y[q] - p1[q]]; crun += list(run[q])
    add('specialty cells max |z| after', max(abs(z) for z in zcell), note=f'{len(zcell)} (scenario, band) cells with 200+ rows, run-clustered; descriptive')
    # the cells jointly: one Wald test, the covariance run-clustered (one run's rows fall in several cells)
    x2c, dfc, pc = common.cluster_chi2(cres, crun)
    add('specialty cells: joint chi2', x2c, note='run-clustered Wald, (scenario, band) cells with 200+ rows, after the term')
    add('specialty cells: df', dfc); add('specialty cells: p', pc)
    u = (sc == 2) & (A['hf'][c] == 0) & (sp == 0)
    add('Unity hf-0 spec-0 before', y[u].sum() / p0[u].sum()); add('Unity hf-0 spec-0 after', y[u].sum() / p1[u].sum())
    # robustness of one pooled d: halves of the records, rarity, without each of the eight most-used cards
    runs_ = common.runs(); cut = runs_[len(runs_) // 2]['seq']          # the later half of the records starts here
    rob = {'earlier half': seq < cut, 'later half': seq >= cut, 'SR': rar == 2, 'SSR': rar == 3}
    top = [k for k, _ in collections.Counter(cid).most_common(8)]
    rob.update({f'without card {k}': cid != k for k in top})
    ds = {nm: pooled_d(q)[0] for nm, q in rob.items()}
    add('specialty d robustness min', min(ds.values()), note='; '.join(f'{k} {v:.2f}' for k, v in ds.items() if not k.startswith('without')))
    add('specialty d robustness max', max(ds.values()))
    # held out: bases and d fitted on the earlier half of the records, scored on the later half, against the same
    # without d
    def d_of(q: Mask, xatol: float = 1e-5, base: np.ndarray = hfm) -> float:
        return minimize_scalar(lambda d: -fit(q, base * (1 + d * sp / 100))[0], bounds=(-0.5, 1.5), method='bounded', options=dict(xatol=xatol)).x

    # the adopted model with a free hf divisor: beta_s (1 + hf/c) (1 + d_s sp/100), one c, a base and d per scenario;
    # fitted as the slope 1/c in [0, 0.05] (c >= 20), each slope's bases and d's profiled out
    hfr = A['hf'][c]

    def ll_ds(q: Mask, base: np.ndarray) -> float:
        """LL with a base and d per scenario on top of multiplier `base`"""
        return sum(fit(q & (sc == k), base * (1 + d_of(q & (sc == k), base=base) * sp / 100))[0] for k in (1, 2, 3, 4) if (q & (sc == k)).any())

    def slope_of(q: Mask) -> float:
        return minimize_scalar(lambda s_: -ll_ds(q, 1 + s_ * hfr), bounds=(0, 0.05), method='bounded', options=dict(xatol=1e-6)).x
    allq = np.ones(len(y), bool); s_free = slope_of(allq)
    add('specialty model: free divisor c', 1 / s_free)
    # ll_ds(allq, hfm) is the per-scenario fit above (the same d fits, the same rows)
    add('specialty model: free divisor gain, nats', ll_ds(allq, 1 + s_free * hfr) - sum(l1s))

    def scored(qa: Mask, qb: Mask, m: np.ndarray) -> np.ndarray:
        """rows qb's log-likelihoods under the base fitted on rows qa for multiplier m"""
        return row_ll(1 - mix(tab[qb], np.minimum(fit(qa, m)[1] * m[qb], TMAX - 1e-6)), y[qb])
    tr, te = seq < cut, seq >= cut; ho = {spec: np.zeros(len(y)) for spec in (False, True)}
    for k in (1, 2, 3, 4):
        qa, qb = tr & (sc == k), te & (sc == k)
        if not qb.any(): continue
        ho[False][qb] = scored(qa, qb, hfm); ho[True][qb] = scored(qa, qb, hfm * (1 + d_of(qa, 1e-4) * sp / 100))
    ix = np.searchsorted(np.unique(run[te]), run[te])
    add('specialty held-out gain, nats', ho[True][te].sum() - ho[False][te].sum()); add('specialty held-out rows', te.sum())
    add('specialty held-out fit rows', tr.sum()); add('specialty held-out first later record', cut, note='seq; records before it fit')
    out.extend(compared('later runs: specialty term vs none', *(np.bincount(ix, weights=ho[s][te]) for s in (True, False))))

    # 5-fold: no term, one pooled d and a d per scenario (each with a base per scenario), fitted on four folds
    def fold_spec(tr: Mask, te: Mask) -> dict[str, np.ndarray]:
        r = {m: np.zeros(len(y)) for m in ('none', 'one d', 'per-scenario d', 'free divisor')}; dp = pooled_d(tr)[0]
        hfc = 1 + slope_of(tr) * hfr
        for k in (1, 2, 3, 4):
            qa, qb = tr & (sc == k), te & (sc == k)
            if not qb.any(): continue
            r['none'][qb] = scored(qa, qb, hfm); r['one d'][qb] = scored(qa, qb, hfm * (1 + dp * sp / 100))
            r['per-scenario d'][qb] = scored(qa, qb, hfm * (1 + d_of(qa) * sp / 100))
            r['free divisor'][qb] = scored(qa, qb, hfc * (1 + d_of(qa, base=hfc) * sp / 100))
        return {m: v[te] for m, v in r.items()}
    hos = cv(run, fold_spec)
    add('held out: specialty runs', len(hos['none']))
    out.extend(compared('specialty term vs none', hos['per-scenario d'], hos['none']))
    out.extend(compared('per-scenario d vs one d', hos['per-scenario d'], hos['one d']))
    out.extend(compared('one d vs none', hos['one d'], hos['none']))
    out.extend(compared('specialty model, free divisor vs 1/100', hos['free divisor'], hos['per-scenario d']))
    # the same card at different limit breaks: a base per (card, scenario), d pooled, every limit break
    cidall = np.array([x['cid'] for x in A['obs']])
    lv = collections.defaultdict(set)
    for i, x in enumerate(A['obs']):
        if x['rar'] != 1: lv[x['cid']].add(A['spec'][i])
    vary = [k for k, v in lv.items() if len(v) > 1]
    jr = np.nonzero(A['noinsp'] & (A['rar'][ci] != 1) & np.isin(cidall[ci], vary))[0]; cw = ci[jr]
    tw = miss_table(np.ones(len(jr), int), A['nskill'][cw], np.ceil(5 / A['hinc'][cw]).astype(int)); yw = A['hit'][jr]
    hw = 1 + A['hf'][cw] / 100; sw = A['spec'][cw]; gw = np.array([f'{a}-{b}' for a, b in zip(cidall[cw], A['scen'][cw])])
    gu, gcode = np.unique(gw, return_inverse=True)                      # (card, scenario) groups, in sorted order
    wc, wrep = alike(gcode, A['hf'][cw], sw, A['nskill'][cw], A['hinc'][cw]); nwc = len(wrep)
    wh = np.bincount(wc, weights=yw, minlength=nwc); wn = np.bincount(wc, minlength=nwc).astype(float); twc = tw[wrep]
    gq = [np.nonzero(gcode[wrep] == u)[0] for u in range(len(gu))]     # each group's classes

    def lw(m: np.ndarray) -> float:
        mc = m[wrep]
        return sum(-minimize_scalar(lambda b: -class_ll(1 - mix(twc[q], np.minimum(b * mc[q], TMAX - 1e-6)), wh[q], wn[q]), bounds=(0.05, 12),
                                    method='bounded', options=dict(xatol=1e-6)).fun for q in gq)
    l0 = lw(hw); rw = minimize_scalar(lambda d: -lw(hw * (1 + d * sw / 100)), bounds=(-0.5, 0.7), method='bounded', options=dict(xatol=1e-3))
    add('specialty same-card: cards', len(vary)); add('specialty same-card: d', rw.x); add('specialty same-card: gain, nats', -rw.fun - l0)
    SPEC.update(d=d_s, gain=gain_s, gain1=gp, bases=bases, zmax=max(abs(z) for z in zcell), ds=ds)
    return out


SPEC: dict[str, Any] = {}     # specialty()'s fitted d, gains, bases, largest cell |z| and robustness d's


@functools.lru_cache(None)
def distinct_pmf(n: int, t: int) -> np.ndarray:
    """P(d distinct of n skills after t uniform draws with replacement), d = 0..t"""
    S = [[0] * (t + 1) for _ in range(t + 1)]; S[0][0] = 1          # Stirling numbers of the second kind
    for i in range(1, t + 1):
        for k in range(1, i + 1): S[i][k] = k * S[i - 1][k] + S[i - 1][k - 1]
    return np.array([math.comb(n, d) * math.factorial(d) * S[t][d] / n ** t for d in range(t + 1)])


if __name__ == '__main__':
    common.main(claims)
