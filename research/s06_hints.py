"""Section 6 (training hints): reproduces docs/RESEARCH.md section 6 from the run records
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
  h = base_s (1 + hf/100) (1 + d_s spec/100); its held-out test fits on the earlier half of the records (by seq) and
  scores the later half;
- tests of the current model: the adopted rate (the same h with hints.rs's constants) on the fit's card-runs, for the
  other covariates (a free level per scenario in both fits), the spark deficit (every row), drop-out and priority.
Every other result is in-sample on the pooled runs.
"""
import collections, functools, math, re, unicodedata
import numpy as np
from scipy.optimize import minimize, minimize_scalar
import common

SEC = '6'
MIL = [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50]
norm = lambda s: re.sub(r'\s+', ' ', unicodedata.normalize('NFKC', s)).strip()


# ---------------------------------------------------------------- game data
def _game():
    if 'g' in common._cache: return common._cache['g']
    db = common.mdb(); q = lambda s: db.execute(s).fetchall()
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


def effect(g, cid, typ, level, mode='lin'):
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
def build():
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
    A = lambda k, t=float: np.array([x[k] for x in obs], t)
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
def miss(k, n, t, cap):
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


TMAX = 12
def miss_table(k, n, cap):
    """rows x (TMAX + 1): miss(k, n, t, cap) for t = 0..TMAX"""
    return np.array([[miss(int(a), int(b), t, int(c)) for t in range(TMAX + 1)] for a, b, c in zip(k, n, cap)])


def mix(tab, h):
    """h read as floor(h) or floor(h) + 1 draws, weighted by the fraction"""
    t0 = np.floor(h).astype(int); f = h - t0; r = np.arange(len(h))
    return (1 - f) * tab[r, t0] + f * tab[r, t0 + 1]


def bern_ll(p, y):
    p = np.clip(p, 1e-12, 1 - 1e-12)
    return np.where(y > 0, np.log(p), np.log(1 - p)).sum()


def fitB(D, hfun, x0, rows=None, nn=None):
    """the form study's fits: P(row hit) = 1 - (1 - 1/n)^h, h = hfun(theta) per card-run; Nelder-Mead then L-BFGS-B"""
    ci = D['ci']; nn = (D['nskill'] if nn is None else nn).astype(float)
    rows = np.ones(len(ci), bool) if rows is None else rows
    nr = nn[ci][rows]; y = D['hit'][rows]; cr = ci[rows]
    def nll(th):
        h = hfun(th)
        if np.any(h <= 0) or not np.all(np.isfinite(h)): return 1e12
        return -bern_ll(1 - (1 - 1 / nr) ** h[cr], y)
    r = minimize(nll, x0, method='Nelder-Mead', options=dict(xatol=1e-6, fatol=1e-7, maxiter=40000, maxfev=80000))
    r2 = minimize(nll, r.x, method='L-BFGS-B')
    b = r2 if r2.fun <= r.fun else r
    return b.x, -b.fun


def fit_cap(D, rows, cap=True, pooled=False):
    """per-scenario (or pooled) base for h = base (1 + hf/100), Bernoulli through the floor/ceiling mixture of
    miss(1, n, t, cap); cap=False draws with replacement. Returns bases, log-likelihood, per-row P(hit)"""
    ci = D['ci']; j = np.nonzero(rows)[0]; c = ci[j]
    capv = np.ceil(5 / D['hinc'][c]).astype(int) if cap else np.full(len(j), 99)
    tab = miss_table(np.ones(len(j), int), D['nskill'][c], capv)
    y = D['hit'][j]; g = np.zeros(len(j), int) if pooled else D['scen'][c]
    bases, ll, p = {}, 0.0, np.zeros(len(j))
    for s in sorted(set(g)):
        q = g == s; mult = 1 + D['hf'][c][q] / 100
        f = lambda b: -bern_ll(1 - mix(tab[q], b * mult), y[q])
        r = minimize_scalar(f, bounds=(0.5, 5), method='bounded', options=dict(xatol=1e-8))
        bases[int(s)] = r.x; ll -= r.fun; p[q] = 1 - mix(tab[q], r.x * mult)
    return bases, ll, p


def B4(D, th):
    return np.asarray(th[:4])[D['scen'] - 1]


# ---------------------------------------------------------------- values
def V(name, got, **kw):
    return common.Value(SEC, name, None if got is None else float(got), **kw)


def adopted():
    """the solver's training-hint bases and specialty weights by scenario (hint_base, hint_spec in
    pkg/loopopt/src/deck/hints.rs)"""
    src = open(f'{common.ROOT}/pkg/loopopt/src/deck/hints.rs', encoding='utf-8').read()

    def table(fn):
        body = re.search(r'fn ' + fn + r'[^{]*\{.*?match scenario \{(.*?)\}', src, re.S).group(1)
        v = {k: float(x) for k, x in re.findall(r'"(\w+)" => ([\d.]+)', body)}
        v['unity'] = float(re.search(r'_ => ([\d.]+)', body).group(1))
        return {1: v['ura'], 2: v['unity'], 3: v['gc'], 4: v['trackblazer'], 5: v['grandmasters']}
    return table('hint_base'), table('hint_spec')


def derived():
    """the h-reading example: n = 5, h = 2.74; smooth (1 - 1/n)^h against floor/ceiling draws"""
    n, h = 5, 2.74
    caps = [V(f'draws to Lv 5, +{i} card', math.ceil(5 / i), note='derived') for i in range(1, 6)]
    return caps + [V('miss (1-1/n)^h, n=5 h=2.74', (1 - 1 / n) ** h, note='derived'),
            V('miss floor/ceiling, n=5 h=2.74', mix(np.array([[(1 - 1 / n) ** t for t in range(TMAX + 1)]]), np.array([h]))[0], note='derived')]


def claims():
    if not common.have_runs() or common.mdb() is None:
        return derived()
    out = []
    def add(name, got, **kw): out.append(V(name, got, **kw))

    # ---- the solver's constants (read, not fitted)
    ADOPTED, ADOPTED_D = adopted()
    for s, nm in ((2, 'Unity Cup'), (3, 'Grand Concert'), (1, 'URA Finale'), (4, 'Trackblazer'), (5, 'Grand Masters')):
        add(f'adopted {nm}', ADOPTED[s], note='the solver\'s hint_base (pkg/loopopt/src/deck/hints.rs)')
        add(f'adopted d {nm}', ADOPTED_D[s], note='the solver\'s hint_spec')

    # ---- the form study: every record, every limit break, no specialty term
    D = build(); ci = D['ci']; hit = D['hit']; NI = D['noinsp']; hf = D['hf']; n = D['nskill'].astype(float)
    sc = D['scen']; max4 = D['lb'][ci] == 4
    add('form: rows', len(ci)); add('form: rows without a spark', NI.sum())
    add('form: runs', len({D['obs'][i]['run'] for i in ci}), note='runs that add a row')
    llB = lambda h, nn=n: bern_ll(1 - (1 - 1 / nn[ci]) ** h[ci], hit)
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
    q = lambda t: (1 - (1 - p) ** t, 1 - (1 - p) ** t - t * p * (1 - p) ** (t - 1))      # P(k >= 1), P(k >= 2)
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
    clip = lambda x: np.minimum(x, TMAX - 1e-6)
    pA = np.full(len(ci), np.nan); pA[jm] = 1 - mix(tabm, clip(hA[cm]))
    note = 'the adopted rate, Lv-5 cap, max limit break, R cards out'

    # covariates: the adopted rate times exp(X theta), a free level per scenario in both fits (so a covariate cannot
    # gain by absorbing an offset of the adopted bases), rows no spark hinted
    nim = NI[jm]; yk = hit[jm][nim]; tk = tabm[nim]; hk = hA[cm][nim]; sk = sc[cm][nim] - 1; ck = cm[nim]

    def fit_cov(X):
        k = X.shape[1]
        nll = lambda th: -bern_ll(1 - mix(tk, clip(hk * np.exp(th[:4][sk] + X @ th[4:]))), yk)
        r = minimize(nll, np.zeros(4 + k), method='Nelder-Mead', options=dict(xatol=1e-6, fatol=1e-7, maxiter=40000, maxfev=80000))
        r2 = minimize(nll, r.x, method='L-BFGS-B'); b = r2 if r2.fun <= r.fun else r
        return -b.fun, np.exp(b.x[4:])
    TY = ['stamina', 'power', 'guts', 'wit']                     # against speed
    covs = {'card type': np.stack([(D['ctype'] == t).astype(float) for t in TY], 1), 'SR': (D['rar'] == 2).astype(float)[:, None],
            'pals per deck': (D['npal'] - 2.5)[:, None].astype(float), 'initial bond': (D['bond0'] / 10)[:, None],
            'pool size': (n - 8)[:, None]}
    l0, _ = fit_cov(np.zeros((len(ck), 0))); facs = []
    add('covariate rows', nim.sum(), note=note)
    for nm, X in covs.items():
        ll_, f_ = fit_cov(X[ck]); facs += list(f_)
        add(f'covariate gain {nm}, nats', ll_ - l0, note=f'{X.shape[1]} parameter(s), factor(s) ' + ', '.join(f'{x:.3f}' for x in f_))
        if X.shape[1] == 1: add(f'covariate factor {nm}', f_[0])
    add('covariate factor min', min(facs)); add('covariate factor max', max(facs))

    # spark deficit by the other sources of the same skill (every row of those card-runs)
    cat = D['cat']
    for c, key in (('none', 'none'), ('inspiration', 'Inspiration'), ('inspiration + card event', 'Inspiration+card event'),
                   ('card event', 'card event'), ('other card training', 'other card')):
        q_ = M & (cat == key)
        add(f'spark {c} rows', q_.sum(), note=note); add(f'spark {c} observed', hit[q_].mean())
        add(f'spark {c} predicted', pA[q_].mean()); add(f'spark {c} ratio', hit[q_].mean() / pA[q_].mean())
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
    return derived() + out + findings(out)


def findings(out):
    """the section's conclusions, each tested on the numbers above"""
    from scipy.stats import binom, chi2
    g = {c.name: c.got for c in out}; F = lambda *a: common.finding(SEC, *a); fs = []
    ll = {k: g[k] for k in ('LL 1+hf/25 over options', 'LL 1+hf/25 over skills', 'LL constant', 'LL base(1+hf/100)')}
    lps = g['LL per-scenario base(1+hf/100)']
    fs.append(F('h = beta_s (1 + hf/100) beats the other rate forms tried (1 + hf/25 over options or skills, a constant, one base)',
                all(lps > v for v in ll.values()), ', '.join(f'{k[3:]} {v - lps:+.1f}' for k, v in ll.items()) + ' nats against it'))
    gc = g['LL free divisor'] - lps
    fs.append(F('Known miss: the slope 1/100 is assumed; a free divisor fits better (gain above 1.92 nats, the 1-parameter 5% bound)',
                gc > 1.92, f'c = {g["free divisor c"]:.0f}, +{gc:.1f} nats on {g["form: runs"]:,.0f} runs'))
    k, n = g['cells var below stat-option floor'], g['cells (card, hf)']
    fs.append(F('The stat option is never drawn: the draw-count variance is below the floor a drawn stat option implies in most (card, hf) '
                'cells (sign test p < 0.05)', binom.sf(k - 1, n, 0.5) < 0.05, f'{k:.0f} of {n:.0f} cells, p = {binom.sf(k - 1, n, 0.5):.4f}'))
    o, t2, po = g['repeat rate observed'], g['repeat rate two-point count'], g['repeat rate Poisson count']
    fs.append(F('Draws repeat skills as uniform draws with a near-fixed count do, not as a Poisson count', abs(o - t2) < abs(o - po),
                f'repeat rate observed {o:.3f}, two-point {t2:.3f}, Poisson {po:.3f}'))
    gb = g['per-scenario bases gain, nats']
    fs.append(F('Bases differ by scenario (per-scenario bases against one base, LR p < 0.001 on 3 parameters)', chi2.sf(2 * gb, 3) < 0.001,
                f'+{gb:.0f} nats, LR p = {chi2.sf(2 * gb, 3):.1e}'))
    cv = {nm: (g[f'covariate gain {nm}, nats'], chi2.sf(2 * g[f'covariate gain {nm}, nats'], k_))
          for nm, k_ in (('card type', 4), ('SR', 1), ('pals per deck', 1), ('initial bond', 1), ('pool size', 1))}
    miss_c, none_c = ('SR', 'pals per deck'), ('card type', 'initial bond', 'pool size')
    fs.append(F('Known miss (not modelled): rarity (SR against SSR) and pals per deck change the rate beyond the adopted rate '
                '(each LR p < 0.05)', all(cv[nm][1] < 0.05 for nm in miss_c),
                '; '.join(f'{nm} +{cv[nm][0]:.1f} nats, p = {cv[nm][1]:.1e}' for nm in miss_c)))
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
        rows, ob, pr = g[f'spark {c} rows'], g[f'spark {c} observed'], g[f'spark {c} predicted']; lo, hi = common.wilson(ob * rows, rows)
        sp[c] = (lo <= pr <= hi, ob, pr, lo, hi)
    fs.append(F('With no other source of the skill the adopted rate is calibrated, and a card\'s own event or another card\'s training '
                'leaves it calibrated too (prediction inside the observed interval)', sp['none'][0] and sp['card event'][0] and sp['other card training'][0],
                '; '.join(f'{c} {v[1]:.3f} [{v[3]:.3f}, {v[4]:.3f}] vs {v[2]:.3f}' for c, v in sp.items() if c != 'inspiration')))
    v = sp['inspiration']
    fs.append(F('Known miss (not modelled): a skill also hinted by inspiration is hinted by training less than predicted', v[4] < v[2],
                f'{v[1]:.3f} [{v[3]:.3f}, {v[4]:.3f}] vs {v[2]:.3f}'))
    o, nd, dd = g['drop-out test observed'], g['drop-out test no drop-out'], g['drop-out test drop-out']
    fs.append(F('Spark-hinted skills do not drop out of the draws (observed nearer no drop-out than full drop-out)', abs(o - nd) < abs(o - dd),
                f'observed {o:.3f}, no drop-out {nd:.3f}, drop-out {dd:.3f}'))
    gs = g['specialty gain per scenario, nats']
    fs.append(F('Specialty priority raises training hints: (1 + d_s sp/100) gains significantly (4 parameters, LR p < 0.001)',
                chi2.sf(2 * gs, 4) < 0.001, f'+{gs:.1f} nats, LR p = {chi2.sf(2 * gs, 4):.1e}'))
    bf = [(b_, g[f'specialty band {b_} before'], g[f'specialty band {b_} after']) for b_ in ('0', '1-29', '30-59', '60+')]
    fs.append(F('With it the residual by specialty is flat: every band within 0.98-1.02 and no (scenario, band) cell beyond |z| = 2',
                all(0.98 <= a_ <= 1.02 for _, _, a_ in bf) and g['specialty cells max |z| after'] < 2,
                '; '.join(f'{b_} {x:.3f} -> {y:.3f}' for b_, x, y in bf) + f'; max |z| {g["specialty cells max |z| after"]:.2f}'))
    fs.append(F('The specialty effect is not an artefact of one card, one rarity or one period: one pooled d stays positive and within '
                'a factor 2 everywhere', 0 < g['specialty d robustness min'] and g['specialty d robustness max'] < 2 * g['specialty d robustness min'],
                f'd {g["specialty d robustness min"]:.2f}-{g["specialty d robustness max"]:.2f}'))
    fs.append(F('Specialty priority predicts runs it was not fitted to: fitted on the earlier runs, it raises the later runs\' '
                'likelihood (held-out gain above 0; smaller than in-sample)', 0 < g['specialty held-out gain, nats'] < g['specialty gain per scenario, nats'],
                f"held out +{g['specialty held-out gain, nats']:.1f} nats on {g['specialty held-out rows']:,.0f} rows; in-sample +{g['specialty gain per scenario, nats']:.1f}"))
    fs.append(F('Not shown to be causal: within the same card across limit breaks the estimate agrees in size with the across-card '
                'one but is not significant (gain under 1.92 nats)', g['specialty same-card: gain, nats'] < 1.92 and g['specialty same-card: d'] > 0,
                f"same-card d {g['specialty same-card: d']:.2f} on {g['specialty same-card: cards']:.0f} cards, +{g['specialty same-card: gain, nats']:.1f} nats; "
                f"across cards d {g['specialty d URA Finale']:.2f} URA Finale, {g['specialty d Unity Cup']:.2f} Unity Cup"))
    rows, ob, pr = g['priority rows'], g['priority observed'], g['priority predicted']; lo, hi = common.wilson(ob * rows, rows)
    fs.append(F('No evidence that prioritising a skill raises its training hints (the observed rate is not above the prediction)',
                ob <= pr or lo <= pr, f'{ob:.3f} [{lo:.3f}, {hi:.3f}] vs {pr:.3f} ({rows:.0f} rows); a weak test'))
    return fs


def specialty(A):
    """specialty priority (effect 19, at the card's level) on the fit's setup over every record: max-limit-break
    card-runs, rows no spark hinted, Lv-5 cap, R cards out. h = base_s (1 + hf/100) (1 + d spec/100)."""
    ci = A['ci']; rows = (A['lb'][ci] == 4) & A['noinsp'] & (A['rar'][ci] != 1)
    j = np.nonzero(rows)[0]; c = ci[j]
    tab = miss_table(np.ones(len(j), int), A['nskill'][c], np.ceil(5 / A['hinc'][c]).astype(int))
    y = A['hit'][j]; hfm = 1 + A['hf'][c] / 100; sp = A['spec'][c]; sc = A['scen'][c]
    cid = np.array([x['cid'] for x in A['obs']])[c]; seq = A['seq'][c]; rar = A['rar'][c]
    out = []; add = lambda name, got, **kw: out.append(V(name, got, **kw))

    def fit(q, m):
        """one scenario's base for multiplier m: (LL, base, P(hit) per row)"""
        f = lambda b: -bern_ll(1 - mix(tab[q], np.minimum(b * m[q], TMAX - 1e-6)), y[q])
        b = minimize_scalar(f, bounds=(0.3, 6), method='bounded', options=dict(xatol=1e-7)).x
        return -f(b), b, 1 - mix(tab[q], np.minimum(b * m[q], TMAX - 1e-6))

    def total(m, q=None):
        q = np.ones(len(y), bool) if q is None else q
        return sum(fit(q & (sc == k), m)[0] for k in (1, 2, 3, 4) if (q & (sc == k)).any())

    def pooled_d(q=None):
        r = minimize_scalar(lambda d: -total(hfm * (1 + d * sp / 100), q), bounds=(-0.5, 1.5), method='bounded', options=dict(xatol=1e-4))
        return r.x, -r.fun - total(hfm, q)
    add('specialty rows', len(j)); add('specialty card-runs', len(set(c)))
    run = np.array([A['obs'][i]['run'] for i in c])
    for k, nm in ((1, 'URA Finale'), (2, 'Unity Cup'), (3, 'Grand Concert'), (4, 'Trackblazer')):
        add(f'specialty rows {nm}', (sc == k).sum()); add(f'specialty runs {nm}', len(set(run[sc == k])))
    d_s, p0, p1, bases = {}, np.zeros(len(y)), np.zeros(len(y)), {}
    gain_s = 0.0
    for k in (1, 2, 3, 4):
        q = sc == k
        r = minimize_scalar(lambda d: -fit(q, hfm * (1 + d * sp / 100))[0], bounds=(-0.5, 1.5), method='bounded', options=dict(xatol=1e-5))
        l1, bases[k], p1[q] = fit(q, hfm * (1 + r.x * sp / 100)); l0, _, p0[q] = fit(q, hfm)
        d_s[k] = r.x; gain_s += l1 - l0
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
    zcell = []
    for k in (1, 2, 3, 4):
        for a_, b_, nm in bands:
            q = (sc == k) & (sp >= a_) & (sp < b_)
            if q.sum() >= 200: zcell.append((y[q].sum() - p1[q].sum()) / np.sqrt((p1[q] * (1 - p1[q])).sum()))
    add('specialty cells max |z| after', max(abs(z) for z in zcell), note=f'{len(zcell)} (scenario, band) cells with 200+ rows')
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
    tr, te = seq < cut, seq >= cut; ho = {}
    for spec in (False, True):
        ll = 0.0
        for k in (1, 2, 3, 4):
            qa, qb = tr & (sc == k), te & (sc == k)
            if not qb.any(): continue
            m = hfm
            if spec:
                dd = minimize_scalar(lambda d: -fit(qa, hfm * (1 + d * sp / 100))[0], bounds=(-0.5, 1.5), method='bounded', options=dict(xatol=1e-4)).x
                m = hfm * (1 + dd * sp / 100)
            b = fit(qa, m)[1]; ll += bern_ll(1 - mix(tab[qb], np.minimum(b * m[qb], TMAX - 1e-6)), y[qb])
        ho[spec] = ll
    add('specialty held-out gain, nats', ho[True] - ho[False]); add('specialty held-out rows', te.sum())
    add('specialty held-out fit rows', tr.sum()); add('specialty held-out first later record', cut, note='seq; records before it fit')
    # the same card at different limit breaks: a base per (card, scenario), d pooled, every limit break
    cidall = np.array([x['cid'] for x in A['obs']])
    lv = collections.defaultdict(set)
    for i, x in enumerate(A['obs']):
        if x['rar'] != 1: lv[x['cid']].add(A['spec'][i])
    vary = [k for k, v in lv.items() if len(v) > 1]
    jr = np.nonzero(A['noinsp'] & (A['rar'][ci] != 1) & np.isin(cidall[ci], vary))[0]; cw = ci[jr]
    tw = miss_table(np.ones(len(jr), int), A['nskill'][cw], np.ceil(5 / A['hinc'][cw]).astype(int)); yw = A['hit'][jr]
    hw = 1 + A['hf'][cw] / 100; sw = A['spec'][cw]; gw = np.array([f'{a}-{b}' for a, b in zip(cidall[cw], A['scen'][cw])])
    gq = [gw == u for u in sorted(set(gw))]

    def lw(m):
        return sum(-minimize_scalar(lambda b: -bern_ll(1 - mix(tw[q], np.minimum(b * m[q], TMAX - 1e-6)), yw[q]), bounds=(0.05, 12),
                                    method='bounded', options=dict(xatol=1e-6)).fun for q in gq)
    l0 = lw(hw); rw = minimize_scalar(lambda d: -lw(hw * (1 + d * sw / 100)), bounds=(-0.5, 0.7), method='bounded', options=dict(xatol=1e-3))
    add('specialty same-card: cards', len(vary)); add('specialty same-card: d', rw.x); add('specialty same-card: gain, nats', -rw.fun - l0)
    SPEC.update(d=d_s, gain=gain_s, gain1=gp, bases=bases, zmax=max(abs(z) for z in zcell), ds=ds)
    return out


SPEC = {}


@functools.lru_cache(None)
def distinct_pmf(n, t):
    """P(d distinct of n skills after t uniform draws with replacement), d = 0..t"""
    S = [[0] * (t + 1) for _ in range(t + 1)]; S[0][0] = 1          # Stirling numbers of the second kind
    for i in range(1, t + 1):
        for k in range(1, i + 1): S[i][k] = k * S[i - 1][k] + S[i - 1][k - 1]
    return np.array([math.comb(n, d) * math.factorial(d) * S[t][d] / n ** t for d in range(t + 1)])


if __name__ == '__main__':
    common.main(claims)
