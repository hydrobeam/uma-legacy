"""Section 6 (training hints): reproduces docs/RESEARCH.md section 6 from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it) and master.mdb.

One observation per (run, non-pal deck card); its rows are the card's training-hint pool skills (master.mdb
single_mode_hint_gain), a row hit when the run's hint list names the card's character as a source of the skill. Hint
frequency (effect 18) and hint levels per draw (1 + effect 17) are read at the card's level: floored linear
interpolation between milestone levels, plus the unique effect.

The form study uses the first 879 records, every limit break: Bernoulli rows, P(hit) = 1 - (1 - 1/n)^h. The adopted
bases are refit on the first 2,402 records, max-limit-break card-runs and rows no spark hinted, through the Lv-5 cap
(uniform draws over the skills still below Lv 5, h read as floor or ceiling; pkg/loopopt/src/deck/hints.rs). Every
result is in-sample on the pooled runs.
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
def build(nrec):
    """card-runs and pool-skill rows of the first `nrec` records"""
    key = ('obs', nrec)
    if key in common._cache: return common._cache[key]
    g = _game(); C = g['cards']; obs = []
    for r in common.runs()[:nrec]:
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
    """the 879-run fits: P(row hit) = 1 - (1 - 1/n)^h, h = hfun(theta) per card-run; Nelder-Mead then L-BFGS-B"""
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


# ---------------------------------------------------------------- claims
def C(name, stated, got, **kw):
    return common.Claim(SEC, name, stated, None if got is None else float(got), **kw)


def below(name, stated, got, bound, note=''):
    """an 'under X' claim: shown as the value, verdict by the inequality"""
    return C(name, stated, got, status='match' if got < bound else 'differs', note=note)


DATA_CLAIMS = [
    ('refit base Unity Cup', '2.708'), ('refit base Grand Concert', '2.026'), ('refit base URA Finale', '1.596'),
    ('refit base Trackblazer', '1.814'), ('refit rows Unity Cup', '8,754'), ('refit rows Grand Concert', '25,111'),
    ('refit rows URA Finale', '9,722'), ('refit rows Trackblazer', '3,349'), ('refit runs Unity Cup', '490'),
    ('refit runs Grand Concert', '1,251'), ('refit runs URA Finale', '514'), ('refit runs Trackblazer', '118'),
    ('adopted Unity Cup', '2.70'), ('adopted Grand Concert', '2.03'), ('adopted URA Finale', '1.60'),
    ('adopted Trackblazer', '1.81'), ('refit base without R cards Unity Cup', '2.667'), ('refit base without R cards Grand Concert', '2.026'),
    ('refit base without R cards URA Finale', '1.569'), ('refit base without R cards Trackblazer', '1.772'), ('refit runs adding a row', '2,373'), ('879: rows', '19,071'), ('879: rows without a spark', '17,079'),
    ('879: pooled base (Grand Masters)', '2.30'),
    ('LL 1+hf/25 over options', '−12,813.3'), ('LL 1+hf/25 over skills', '−12,461.0'), ('LL constant', '−12,048.0'),
    ('LL base(1+hf/100)', '−11,910.2'), ('LL per-scenario base(1+hf/100)', '−11,817.8'),
    ('LL free divisor', '−11,811.9'), ('free divisor c', '76'),
    ('cells var below stat-option floor', '16'), ('cells (card, hf)', '19'),
    ('repeat rate observed', '0.151'), ('repeat rate two-point count', '0.151'), ('repeat rate Poisson count', '0.199'),
    *[(f'distinct {k} observed', v) for k, v in zip((1, 2, 3, 4), ('0.093', '0.520', '0.307', '0.074'))],
    *[(f'distinct {k} model', v) for k, v in zip((1, 2, 3, 4), ('0.084', '0.476', '0.359', '0.079'))],
    ('per-scenario bases gain, nats', '92'), ('other covariates max gain', 'under 3.5'),
    ('covariate factor min', '0.91'), ('covariate factor max', '1.02'),
    ('cap gain 879, nats', '7.8'), ('cap gain 2,402, nats', '17.5'),
    *[(f'cap table {d} draws {w}', v) for d, vs in ((5, ('5,347', '0.968', '0.979')), (3, ('259', '0.863', '0.870')),
                                                    (2, ('7,335', '1.025', '1.024')), (1, ('94', '1.289', '0.985')))
      for w, v in zip(('rows', 'replacement', 'cap'), vs)],
    ('joint pairs', '38,927'), ('P(both) observed', '0.0872'), ('P(both) joint', '0.0862'),
    ('P(both) independent', '0.1135'), ('P(neither) observed', '0.4371'), ('P(neither) joint', '0.4354'),
    ('pool size obs/pred min', '0.88'), ('pool size obs/pred max', '1.14'),
    ('increment: multiples of iota', '5,144'), ('increment: single-source levels', '5,150'),
    ('hf max in the data', '60'), ('LL hf floored-linear', '−10,676.9'), ('LL hf step', '−10,677.4'), ('LL hf max-LB for all', '−10,683.8'),
    *[(f'spark {c} {w}', v) for c, vs in (('none', ('14,949', '0.345', '0.345', '1.00')),
                                          ('inspiration', ('1,696', '0.269', '0.347', '0.78')),
                                          ('inspiration + card event', ('139', '0.158', '0.369', '0.43')),
                                          ('card event', ('635', '0.334', '0.351', '0.95')),
                                          ('other card training', ('572', '0.372', '0.376', '0.99')))
      for w, v in zip(('rows', 'observed', 'predicted', 'ratio'), vs)],
    ('drop-out test observed', '0.375'), ('drop-out test no drop-out', '0.341'), ('drop-out test drop-out', '0.455'),
    ('specialty gain, nats', '18'), ('specialty factor per 10 points (log)', '0.0251'), ('specialty Unity base', '2.50'),
    ('specialty free divisor', '92'),
    *[(f'specialty {c} {s}', v) for s, vs in (('Unity', ('0.89', '0.86', '1.16')), ('GC', ('1.09', '0.96', '1.06')))
      for c, v in zip(('Marvelous Sunday', 'Matikanefukukitaru', 'Agnes Tachyon'), vs)],
    ('Unity hf-0 spec-0 over-rating', '13%'),
    ('priority runs', '101'), ('priority rows', '99'), ('priority observed', '0.222'), ('priority predicted', '0.298'),
]


def adopted():
    """the solver's training-hint bases by scenario (hint_base in pkg/loopopt/src/deck/hints.rs)"""
    src = open(f'{common.ROOT}/pkg/loopopt/src/deck/hints.rs', encoding='utf-8').read()
    body = re.search(r'fn hint_base[^{]*\{.*?match scenario \{(.*?)\}', src, re.S).group(1)
    v = {k: float(x) for k, x in re.findall(r'"(\w+)" => ([\d.]+)', body)}
    v['unity'] = float(re.search(r'_ => ([\d.]+)', body).group(1))
    return {1: v['ura'], 2: v['unity'], 3: v['gc'], 4: v['trackblazer']}


def derived():
    """the h-reading example: n = 5, h = 2.74; smooth (1 - 1/n)^h against floor/ceiling draws"""
    n, h = 5, 2.74
    caps = [C(f'draws to Lv 5, +{i} card', v, math.ceil(5 / i), note='derived') for i, v in zip(range(1, 6), '53221')]
    return caps + [C('miss (1-1/n)^h, n=5 h=2.74', '0.5426', (1 - 1 / n) ** h, note='derived'),
            C('miss floor/ceiling, n=5 h=2.74', '0.5453', mix(np.array([[(1 - 1 / n) ** t for t in range(TMAX + 1)]]), np.array([h]))[0], note='derived')]


def claims():
    if not common.have_runs() or common.mdb() is None:
        return derived() + common.need_data(SEC, DATA_CLAIMS)
    out = []; st = dict(DATA_CLAIMS)
    def add(name, got, **kw): out.append(C(name, st[name], got, **kw))

    # ---- the adopted fit: first 2,402 records, max-limit-break card-runs, rows no spark hinted, Lv-5 cap
    ADOPTED = adopted()
    A = build(len(common.runs())); ca = A['ci']
    fitset = (A['seq'][ca] < 2402) & (A['lb'][ca] == 4) & A['noinsp']
    bases, ll_cap, _ = fit_cap(A, fitset)
    _, ll_rep, _ = fit_cap(A, fitset, cap=False)
    noR = fit_cap(A, fitset & (A['rar'][ca] != 1))[0]      # the refit included R cards; shown for reference
    for s, nm in ((2, 'Unity Cup'), (3, 'Grand Concert'), (1, 'URA Finale'), (4, 'Trackblazer')):
        q = fitset & (A['scen'][ca] == s)
        add(f'refit base {nm}', bases[s], note='R card-runs included, as fitted')
        add(f'refit base without R cards {nm}', noR[s])
        add(f'refit rows {nm}', q.sum())
        add(f'adopted {nm}', ADOPTED[s], note='the solver\'s hint_base (pkg/loopopt/src/deck/hints.rs)')
        add(f'refit runs {nm}', len({A['obs'][i]['run'] for i in ca[q]}))
    add('refit runs adding a row', len({A['obs'][i]['run'] for i in ca[fitset]}))

    # ---- the 879-record set (every limit break)
    D = build(879); ci = D['ci']; hit = D['hit']; NI = D['noinsp']; hf = D['hf']; n = D['nskill'].astype(float)
    sc = D['scen']; max4 = D['lb'][ci] == 4
    add('879: rows', len(ci)); add('879: rows without a spark', NI.sum())
    add('879: pooled base (Grand Masters)', fit_cap(D, max4 & NI, pooled=True)[0][0],
        note='Lv-5 cap, max limit break, no spark (the adopted method, one base)')
    llB = lambda h, nn=n: bern_ll(1 - (1 - 1 / nn[ci]) ** h[ci], hit)
    add('LL 1+hf/25 over options', llB(1 + hf / 25, D['opts'].astype(float)), tol=0.1)
    add('LL 1+hf/25 over skills', llB(1 + hf / 25), tol=0.1)
    add('LL constant', fitB(D, lambda th: th[0] + 0 * hf, [2.])[1], tol=0.1)
    ll1 = fitB(D, lambda th: th[0] * (1 + hf / 100), [2.])[1]; add('LL base(1+hf/100)', ll1, tol=0.1)
    th4, ll4 = fitB(D, lambda th: B4(D, th) * (1 + hf / 100), [1.5, 2.6, 2., 1.7]); add('LL per-scenario base(1+hf/100)', ll4, tol=0.1)
    thc, llc = fitB(D, lambda th: B4(D, th) * (1 + hf / th[4]), list(th4) + [100.])
    add('LL free divisor', llc, tol=0.1); add('free divisor c', thc[4])

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

    # covariates on all rows of the 879 set
    add('per-scenario bases gain, nats', ll4 - ll1)
    base = lambda th: B4(D, th) * (1 + hf / 100)
    TY = ['stamina', 'power', 'guts', 'wit']
    covs = {'card type': np.stack([(D['ctype'] == t).astype(float) for t in TY], 1),
            'SR': (D['rar'] == 2).astype(float)[:, None], 'LB < 4': (D['lb'] < 4).astype(float)[:, None],
            'pals per deck': (D['npal'] - 2.5)[:, None].astype(float), 'initial bond': (D['bond0'] / 10)[:, None],
            'pool size': (n - 8)[:, None]}
    gains, facs = {}, []
    for nm, X in covs.items():
        k = X.shape[1]
        th, ll = fitB(D, lambda th, X=X, k=k: base(th[:4]) * np.exp(X @ th[4:4 + k]), list(th4) + [0.] * k)
        gains[nm] = ll - ll4; facs += list(np.exp(th[4:]))
    out.append(below('other covariates max gain', st['other covariates max gain'], max(gains.values()), 3.5,
                     note=', '.join(f'{k} {v:.1f}' for k, v in gains.items())))
    add('covariate factor min', min(facs)); add('covariate factor max', max(facs))

    # the Lv-5 cap against drawing with replacement (max limit break, rows no spark hinted)
    rows = max4 & NI
    _, l879c, pc = fit_cap(D, rows); _, l879r, pr_ = fit_cap(D, rows, cap=False)
    add('cap gain 879, nats', l879c - l879r); add('cap gain 2,402, nats', ll_cap - ll_rep)
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
    add('LL hf floored-linear', ll_ni, tol=0.1)
    for nm, H in (('LL hf step', D['hf_step']), ('LL hf max-LB for all', D['hf_max'])):
        add(nm, fitB(D, lambda th, H=H: B4(D, th) * (1 + H / 100), list(bs), rows=NI)[1], tol=0.1)

    # spark deficit by the other sources of the same skill (every row; model fitted on rows no spark hinted)
    prow = (1 - (1 - 1 / np.maximum(n, 1)) ** h)[ci]; cat = D['cat']
    for c, key in (('none', 'none'), ('inspiration', 'Inspiration'), ('inspiration + card event', 'Inspiration+card event'),
                   ('card event', 'card event'), ('other card training', 'other card')):
        q_ = cat == key
        add(f'spark {c} rows', q_.sum()); add(f'spark {c} observed', hit[q_].mean())
        add(f'spark {c} predicted', prow[q_].mean()); add(f'spark {c} ratio', hit[q_].mean() / prow[q_].mean())
    # drop-out: max-limit-break card-runs with exactly two pool skills spark-hinted, their other (no-spark) rows
    ins = np.bincount(ci, weights=(~NI).astype(float), minlength=D['N'])[ci]
    q_ = NI & max4 & (ins == 2); jj = np.nonzero(q_)[0]; c_ = ci[jj]
    capj = np.ceil(5 / D['hinc'][c_]).astype(int); hj = B4(D, [bases[s] for s in (1, 2, 3, 4)])[c_] * (1 + hf[c_] / 100)
    keep = 1 - mix(miss_table(np.ones(len(jj), int), D['nskill'][c_], capj), hj)
    drop = 1 - mix(miss_table(np.ones(len(jj), int), D['nskill'][c_] - 2, capj), hj)
    note = 'adopted bases, Lv-5 cap'
    add('drop-out test observed', hit[jj].mean(), note=note)
    add('drop-out test no drop-out', keep.mean(), note=note); add('drop-out test drop-out', drop.mean(), note=note)

    # not adopted: specialty priority (effect 19)
    spec = D['spec']
    add('specialty gain, nats', fitB(D, lambda th: base(th) * np.exp(th[4] * spec / 10), list(th4) + [.02])[1] - ll4,
        note='all rows, as the covariate table')
    ths, _ = fitB(D, lambda th: base(th) * np.exp(th[4] * spec / 10), list(bs) + [.02], rows=NI)
    add('specialty factor per 10 points (log)', ths[4], note='rows no spark'); add('specialty Unity base', ths[1], note='rows no spark')
    thd, _ = fitB(D, lambda th: B4(D, th) * (1 + hf / th[4]) * np.exp(th[5] * spec / 10), list(ths[:4]) + [100., ths[4]], rows=NI)
    add('specialty free divisor', thd[4])
    for c, cardid in (('Marvelous Sunday', 30074), ('Matikanefukukitaru', 30078), ('Agnes Tachyon', 30101)):
        for s, sn in ((2, 'Unity'), (3, 'GC')):
            q_ = NI & (cid[ci] == cardid) & (sc[ci] == s)
            add(f'specialty {c} {sn}', hit[q_].sum() / prow[q_].sum())
    q_ = NI & (sc[ci] == 2) & (hf[ci] == 0) & (spec[ci] == 0)
    add('Unity hf-0 spec-0 over-rating', 1 - hit[q_].sum() / prow[q_].sum(), note='about; 1 - obs/pred')

    # prioritised skills
    prio = [None if x['prio'] is None else {norm(s) for s in x['prio']} for x in D['obs']]
    isp = np.array([prio[ci[r_]] is not None and D['skn'][r_] in prio[ci[r_]] for r_ in range(len(ci))]) & NI
    add('priority runs', len({x['run'] for x in D['obs'] if x['prio'] is not None}))
    add('priority rows', isp.sum()); add('priority observed', hit[isp].mean())
    add('priority predicted', prow[isp].mean())
    order = [nm for nm, _ in DATA_CLAIMS]
    return derived() + sorted(out, key=lambda c: order.index(c.name))


@functools.lru_cache(None)
def distinct_pmf(n, t):
    """P(d distinct of n skills after t uniform draws with replacement), d = 0..t"""
    S = [[0] * (t + 1) for _ in range(t + 1)]; S[0][0] = 1          # Stirling numbers of the second kind
    for i in range(1, t + 1):
        for k in range(1, i + 1): S[i][k] = k * S[i - 1][k] + S[i - 1][k - 1]
    return np.array([math.comb(n, d) * math.factorial(d) * S[t][d] / n ** t for d in range(t + 1)])


if __name__ == '__main__':
    common.main(claims)
