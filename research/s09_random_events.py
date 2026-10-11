"""Section 9: how often a support card's character's shared random event fires, from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it).

Markers (GameTora training_events_shared, through gametora.py): a character's shared event with two or more choices
whose top choice gives a skill in every outcome and whose other choices give none; the skill is her marker (one per
character). A reading is one run with a card of hers in the deck (any rarity, group cards aside), someone else
training, and the marker not given by that card's own events; a hit is the marker credited to "<her name> event"
("A or B event" counts as a miss). K = random events on offer: every shared event of the deck's characters plus the
trainee's own. The first fit used the first 1,850 records, the larger sample the first 2,653, the priority-list check
the first 1,300.
The fits are maximum likelihood over readings: constant, a (1 - b)^(K - 4) form, power law, strict competition
1 - (1 - 1/K)^S, and every event slot competing (chain links, random events and pals weighted in one pool).
As stated in RESEARCH.md, the population includes R cards; `detail()` prints the SR/SSR-only fit beside it.
Findings: likelihood-ratio tests between the forms and on profiled parameters (uniform competition, chain-link and pal
weights), within-run independence (pairs of hits against Binomial(n, p) per run), and on the larger sample the slope and
the covariates next to K (card effects at their limit break from master.mdb; rarity within a character).
"""
import collections, math
import numpy as np
from scipy.optimize import minimize
import common

SEC = '9'
FIRST, LARGER, PRIO = 1850, 2653, 1300


def markers():
    gt = common.gametora(); db = common.mdb()
    sk = dict(db.execute('select [index], text from text_data where category = 47'))

    def outs(rewards):
        out = []
        for hs, _ in gt.outcomes(rewards):
            s = set()
            for kind, x, _ in hs:
                if kind == 'hint' and x in sk: s.add(sk[x])
                elif kind == 'random': s |= {sk[i] for i in x if i in sk}
            out.append(s)
        return out

    def sure(rewards):
        o = outs(rewards); return set.intersection(*o) if o else set()
    nshared = collections.Counter(); marker = {}
    for chara, es in gt.load('training_events_shared'):
        for e in es:
            chs = gt.choices(e[1])
            if not chs: continue
            nshared[chara] += 1
            if len(chs) >= 2:
                a, b = sure(chs[0][1]), set().union(*[sure(ch[1]) for ch in chs[1:]])
                if a and not b: marker[chara] = sorted(a)[0]
    cardsk = collections.defaultdict(set)    # every skill a card's own events can give
    for key in ('training_events_sr', 'training_events_ssr'):
        for cid, es in gt.load(key):
            for e in es:
                for ch in gt.choices(e[1]): cardsk[cid] |= set().union(*outs(ch[1]))
    sc = {x['support_id']: x for x in gt.load('support-cards')}
    return nshared, marker, cardsk, sc


def readings(R):
    nshared, marker, cardsk, sc = markers()
    name = dict(common.mdb().execute('select [index], text from text_data where category = 6'))
    wc = common.cards()
    rows = []
    for r in R:
        dk = [d[0] for d in r['deck']]; tr = r['chara']; lbs = {d[0]: d[1] for d in r['deck']}
        pals = sum(1 for x in dk if wc.get(x, {}).get('pal'))
        links = sum(wc.get(x, {}).get('links', 0) for x in dk if not wc.get(x, {}).get('pal'))
        chars = {sc[x]['char_id'] for x in dk if x in sc and sc[x].get('type') != 'group'}
        K = sum(nshared[ch] for ch in chars - {tr}) + nshared[tr]
        for cid in dk:
            x = sc.get(cid)
            if not x or x.get('type') == 'group': continue
            ch = x['char_id']; m = marker.get(ch)
            if not m or ch == tr or m in cardsk[cid]: continue
            nm = name.get(ch); srcs = [s for h in r['hints'] if h['name'] == m for s in h['src']]
            hit = f'{nm} event' in srcs
            amb = not hit and any(nm in s and ' or ' in s for s in srcs)
            rows.append(dict(run=r['id'], card=cid, rar=x['rarity'], hit=int(hit), amb=int(amb), K=K, pals=pals, links=links,
                             char=ch, lb=lbs[cid], scen=r['scenario']))
    return rows


def ll(y, p):
    p = np.clip(p, 1e-9, 1 - 1e-9); return float(np.sum(y * np.log(p) + (1 - y) * np.log(1 - p)))


def fits(rows):
    y = np.array([r['hit'] for r in rows], float); K = np.array([r['K'] for r in rows], float)
    Lk = np.array([r['links'] for r in rows], float); Pl = np.array([r['pals'] for r in rows], float)
    out = {'const': (y.mean(), ll(y, np.full_like(y, y.mean())))}
    o = minimize(lambda s: -ll(y, s[0] * (1 - s[1]) ** (K - 4)), [0.2, 0.05], bounds=[(0.01, 0.99), (-0.5, 0.5)])
    out['exp'] = (tuple(o.x), -o.fun)
    o = minimize(lambda s: -ll(y, s[0] * (K / 8) ** (-s[1])), [0.17, 0.5], bounds=[(0.01, 0.9), (-3, 3)])
    out['power'] = (tuple(o.x), -o.fun)
    o = minimize(lambda s: -ll(y, 1 - (1 - 1 / K) ** s[0]), [1.0], bounds=[(0.01, 50)])
    out['strict'] = (o.x[0], -o.fun)
    best = None
    for fn in (lambda s, T: 1 - (1 - 1 / T) ** s, lambda s, T: s / T):
        o = minimize(lambda p: -ll(y, fn(p[0], p[2] * Lk + K + p[1] * Pl)), [3.0, 3.0, 1.0],
                     bounds=[(0.01, 200), (0, 30), (0, 10)])
        if best is None or -o.fun > best[1]: best = (tuple(o.x), -o.fun)
    out['slots'] = best
    return out


def by_k(rows):
    t = collections.defaultdict(lambda: [0, 0])
    for r in rows: k = min(r['K'], 14); t[k][0] += 1; t[k][1] += r['hit']
    return t


def logistic_k(rows):
    """logit P = b0 + b1 K"""
    y = np.array([r['hit'] for r in rows], float); X = np.column_stack([np.ones(len(rows)), [r['K'] for r in rows]])
    o = minimize(lambda b: float(np.sum(np.logaddexp(0, X @ b) - y * (X @ b))), np.zeros(2), method='BFGS')
    return o.x


def bottom_prioritised(R):
    """runs showing a priority list, and how many prioritise a bottom-choice-only skill of a deck card's character"""
    gt = common.gametora(); db = common.mdb()
    sk = dict(db.execute('select [index], text from text_data where category = 47'))

    def sure(rewards):
        o = []
        for hs, _ in gt.outcomes(rewards):
            s = set()
            for kind, x, _ in hs:
                if kind == 'hint' and x in sk: s.add(sk[x])
                elif kind == 'random': s |= {sk[i] for i in x if i in sk}
            o.append(s)
        return set.intersection(*o) if o else set()
    bottom = collections.defaultdict(set)
    for chara, es in gt.load('training_events_shared'):
        for e in es:
            chs = gt.choices(e[1])
            if len(chs) < 2: continue
            a, b = sure(chs[0][1]), set().union(*[sure(ch[1]) for ch in chs[1:]])
            if b and not a: bottom[chara] |= b
    _, _, cardsk, sc = markers()
    shown = n = cand = 0
    for r in R:
        p = r.get('priority')
        if p is None: continue
        shown += 1
        for cid in (d[0] for d in r['deck']):
            x = sc.get(cid)
            if not x or x.get('type') == 'group' or x['char_id'] == r['chara']: continue
            n += sum(s in p for s in bottom[x['char_id']] - cardsk[cid]); cand += len(bottom[x['char_id']] - cardsk[cid])
    return shown, n, cand


def claims():
    R = common.runs()
    if R is None: return []
    rows = readings([r for r in R if r['seq'] < FIRST])
    f = fits(rows); g = {}
    y = sum(r['hit'] for r in rows); n = len(rows); amb = sum(r['amb'] for r in rows)
    g.update({'readings': n, 'runs': len({r['run'] for r in rows}), 'clean hits': y, 'ambiguous credits': amb,
              'rate with ambiguous, points higher': 100 * amb / n})
    g['constant p'], g['LL constant'] = f['const']
    (a, b), g['LL fitted form'] = f['exp']; g['fitted form: P at K = 4'] = a; g['fitted form: 1 - ratio per event'] = b
    (_, g['power law a']), g['LL power law'] = f['power']
    g['strict competition S'], g['LL strict competition'] = f['strict']
    (g['every slot competing: c'], g['every slot competing: pal weight'], g['every slot competing: chain-link weight']), \
        g['LL every slot competing'] = f['slots']
    t = by_k(rows)
    for k in (4, 6, 8, 10, 12, 14): g[f'hit rate K = {k}'] = t[k][1] / t[k][0]
    g['fitted P overall'] = float(np.mean([a * (1 - b) ** (r['K'] - 4) for r in rows]))
    g['fitted P at K = 4'] = a; g['fitted P at K = 14'] = a * (1 - b) ** 10
    big = readings([r for r in R if r['seq'] < LARGER])
    g['larger sample readings'] = len(big); g['larger sample runs'] = len({r['run'] for r in big})
    b0, b1 = logistic_k(big)
    for rr, lab in ((3, 'SSR'), (2, 'SR')):
        xs = [r['hit'] for r in big if r['rar'] == rr]; g[f'larger sample, pooled: {lab} marker rate'] = sum(xs) / len(xs)
    g['larger sample: rate at K = 8'] = 1 / (1 + math.exp(-(b0 + 8 * b1)))
    g['larger sample: rate at K = 14'] = 1 / (1 + math.exp(-(b0 + 14 * b1)))
    g['runs showing a priority list (first 1,300 records)'], g['bottom-choice markers prioritised there'], cand = \
        bottom_prioritised([r for r in R if r['seq'] < PRIO])
    g['every slot competing: LL over constant'] = f['slots'][1] - f['const'][1]
    return [common.Value(SEC, name, v) for name, v in g.items()] + findings(rows, f, big, g, cand)


# ---------------------------------------------------------------- findings: each conclusion as a likelihood or interval test
def slot_ll(rows, link=None, pal=None):
    """max LL of c / T, T = link x chain links + K + pal x pals, with either weight held fixed"""
    y = np.array([r['hit'] for r in rows], float); K = np.array([r['K'] for r in rows], float)
    Lk = np.array([r['links'] for r in rows], float); Pl = np.array([r['pals'] for r in rows], float)
    fx = lambda v, x: x if v is None else v
    o = minimize(lambda q: -ll(y, q[0] / (fx(link, q[2]) * Lk + K + fx(pal, q[1]) * Pl)), [3.0, 1.0, 0.5],
                 bounds=[(0.01, 200), (0, 30), (0, 10)])
    return -o.fun


def logit_ll(y, X):
    """max log-likelihood of a logistic regression (X holds its own intercept columns)"""
    f = lambda b: float(np.sum(np.logaddexp(0, X @ b) - y * (X @ b)))
    gr = lambda b: X.T @ (1 / (1 + np.exp(-(X @ b))) - y)
    return -minimize(f, np.zeros(X.shape[1]), jac=gr, method='BFGS').fun


def card_effect(cid, lb, typ):
    """a support card's effect of one type at its limit break's level cap (master.mdb: linear between milestone levels,
    plus unique effects reached)"""
    db = common.mdb()
    eff, rar = db.execute('select effect_table_id, rarity from support_card_data where id = ?', (cid,)).fetchone()
    lv = db.execute(f'select limit_{lb} from support_card_limit where rarity = ?', (rar,)).fetchone()[0]
    row = db.execute('select init, limit_lv5, limit_lv10, limit_lv15, limit_lv20, limit_lv25, limit_lv30, limit_lv35, limit_lv40, '
                     'limit_lv45, limit_lv50 from support_card_effect_table where id = ? and type = ?', (eff, typ)).fetchone()
    v = 0.0
    pts = [(l, x) for l, x in zip([1] + list(range(5, 55, 5)), row or []) if x != -1]
    if pts and lv >= pts[0][0]:
        v = float(pts[-1][1])
        for (l0, v0), (l1, v1) in zip(pts, pts[1:]):
            if l0 <= lv <= l1: v = v0 + (v1 - v0) * (lv - l0) / (l1 - l0); break
    for ulv, t0, v0, t1, v1 in db.execute('select lv, type_0, value_0, type_1, value_1 from support_card_unique_effect where id = ?', (cid,)):
        v += sum(x for t, x in ((t0, v0), (t1, v1)) if t == typ and lv >= ulv)
    return v


def pairs_test(rows, a, b):
    """within-run independence: pairs of readings of one run that both hit, against Binomial(n, p) per run with the
    fitted p (one K per run); the variance is exact per run"""
    by = collections.defaultdict(list)
    for r in rows: by[r['run']].append(r)
    o = e = v = 0.0
    for rs in by.values():
        n = len(rs); p = a * (1 - b) ** (rs[0]['K'] - 4); h = sum(r['hit'] for r in rs)
        pm = common.binom_pmf(n, p); c2 = [k * (k - 1) / 2 for k in range(n + 1)]
        m = sum(q * c for q, c in zip(pm, c2)); o += h * (h - 1) / 2; e += m; v += sum(q * c * c for q, c in zip(pm, c2)) - m * m
    return o, e, (o - e) / math.sqrt(v)


def findings(rows, f, big, g, cand):
    F = lambda st, test, ev: common.finding(SEC, st, test, ev)
    from scipy.stats import chi2
    L0, (ab, Le), (pw, Lp), (_, Ls), Lslot = f['const'][1], f['exp'], f['power'], f['strict'], f['slots'][1]
    out = [F('Events on offer matter: the adopted form a (1 - b)^(K - 4) beats a constant by more than 1.92 nats '
             '(one extra parameter, 95%)', Le - L0 > 1.92,
             f'a {ab[0]:.2f}, b {ab[1]:.3f}: LL {Le:.1f} against {L0:.1f}: {Le - L0:+.1f} nats')]
    y = np.array([r['hit'] for r in rows], float); K = np.array([r['K'] for r in rows], float)
    L1 = -minimize(lambda q: -ll(y, q[0] * (K / 8) ** -1.0), [0.17], bounds=[(0.01, 0.9)]).fun
    out.append(F('Competition is far weaker than uniform: the power law\'s a = 1 (uniform competition) is rejected against '
                 'the fitted a by more than 1.92 nats', Lp - L1 > 1.92, f'a = {pw[1]:.2f}: LL {Lp:.1f}; a = 1: LL {L1:.1f}'))
    out.append(F('Strict competition 1 - (1 - 1/K)^S is rejected: it fits worse than a constant with the same number of '
                 'parameters', Ls < L0, f'LL {Ls:.1f} against constant {L0:.1f}'))
    out.append(F('The every-slot form fits about as well as the adopted one: within 1.92 nats of it', abs(Lslot - Le) < 1.92,
                 f'slots LL {Lslot:.1f} ({Lslot - L0:+.1f} over constant, two extra parameters); adopted {Le:.1f}'))
    Lk1 = slot_ll(rows, link=1.0)
    out.append(F('Chain links play no part: their fitted weight is 0, and weight 1 (a link competing like a random event) '
                 'is rejected by more than 1.92 nats', g['every slot competing: chain-link weight'] < 0.01 and Lslot - Lk1 > 1.92,
                 f'weight {g["every slot competing: chain-link weight"]:.2f}: LL {Lslot:.1f}; weight 1: LL {Lk1:.1f}'))
    Lp0, Lp1 = slot_ll(rows, pal=0.0), slot_ll(rows, pal=1.0)
    out.append(F('A pal counts about as one more random event: pal weight 1 inside the 95% likelihood interval (within 1.92 '
                 'nats), weight 0 outside it', Lslot - Lp1 < 1.92 and Lslot - Lp0 > 1.92,
                 f'weight {g["every slot competing: pal weight"]:.2f}: LL {Lslot:.1f}; weight 1: {Lp1:.1f}; weight 0: {Lp0:.1f}'))
    o, e, z = pairs_test(rows, *ab)
    out.append(F('Within a run events fire independently: pairs of one run\'s readings that both hit are within 1.96 sd of '
                 'the count independence predicts', abs(z) < 1.96, f'{o:.0f} pairs observed, {e:.1f} expected, z {z:+.2f}'))
    yb = np.array([r['hit'] for r in big], float); one = np.ones(len(big)); Kb = np.array([r['K'] for r in big], float)
    base = logit_ll(yb, np.column_stack([one, Kb])); Lc = logit_ll(yb, one[:, None])
    out.append(F('On the larger sample the slope holds: K in the logistic model gains more than 1.92 nats, and the rate falls',
                 base - Lc > 1.92 and g['larger sample: rate at K = 14'] < g['larger sample: rate at K = 8'],
                 f'{base - Lc:+.1f} nats; {g["larger sample: rate at K = 8"]:.3f} at 8, {g["larger sample: rate at K = 14"]:.3f} at 14'))
    # covariates next to K, larger sample: likelihood-ratio tests at 95%
    lr = lambda cols: 2 * (logit_ll(yb, np.column_stack([one, Kb] + cols)) - base)
    eff = {}
    for t, nm in ((1, 'friendship bonus'), (14, 'initial friendship gauge'), (19, 'specialty priority')):
        cache = {}
        x = np.array([cache.setdefault((r['card'], r['lb']), card_effect(r['card'], r['lb'], t)) for r in big]) / 10
        eff[nm] = (lr([x]), 1)
    sc = sorted({r['scen'] for r in big})[1:]
    eff['scenario'] = (lr([np.array([r['scen'] == v for r in big], float) for v in sc]), len(sc))
    out.append(F('Friendship (bonus, initial gauge), specialty priority and scenario show no effect next to K: each '
                 'likelihood-ratio statistic below its chi2 95% point', all(s < chi2.ppf(0.95, d) for s, d in eff.values()),
                 '; '.join(f'{k} {s:.2f} (df {d}, limit {chi2.ppf(0.95, d):.2f})' for k, (s, d) in eff.items())))
    rar = np.array([r['rar'] for r in big], float)
    multi = collections.defaultdict(set)
    for r in big: multi[r['char']].add(r['rar'])
    ch = sorted(c for c, v in multi.items() if len(v) > 1)
    w = [i for i, r in enumerate(big) if r['char'] in ch]
    D = np.array([[big[i]['char'] == c for c in ch] for i in w], float)
    fe = logit_ll(yb[w], np.column_stack([D, Kb[w]])); fe_r = logit_ll(yb[w], np.column_stack([D, Kb[w], rar[w]]))
    out.append(F('Rarity shows no effect within a character (character intercepts and K, characters with cards of more than '
                 'one rarity): likelihood-ratio statistic below 3.84; pooled across characters it is shown alongside',
                 2 * (fe_r - fe) < 3.84, f'within {len(ch)} characters ({len(w):,} readings): {2 * (fe_r - fe):.2f}; pooled: {lr([rar]):.2f}'))
    out.append(F('Bottom-choice skills of these events need a priority, and none was prioritised: 0 of the bottom-choice '
                 'skills on offer in the runs showing a priority list', g['bottom-choice markers prioritised there'] == 0,
                 f'{g["bottom-choice markers prioritised there"]} of {cand:,} (run, skill) candidates in '
                 f'{g["runs showing a priority list (first 1,300 records)"]} runs'))
    return out


def detail():
    R = common.runs()
    for lab, cut in (('first sample', FIRST), ('larger sample', LARGER)):
        rows = readings([r for r in R if r['seq'] < cut])
        print(f'{lab} ({cut} records): rate by K', ', '.join(f'{k}: {h}/{n}' for k, (n, h) in sorted(by_k(rows).items())))
        t = collections.defaultdict(lambda: [0, 0])
        for r in rows: t[r['rar']][0] += 1; t[r['rar']][1] += r['hit']
        print('   by rarity (1 R, 2 SR, 3 SSR):', ', '.join(f'{k}: {h}/{n} = {h / n:.3f}' for k, (n, h) in sorted(t.items())))
        sr = [r for r in rows if r['rar'] >= 2]; f = fits(sr); (a, b), L = f['exp']
        print(f'   SR/SSR only: {len(sr)} readings, {len({r["run"] for r in sr})} runs, {sum(r["hit"] for r in sr)} hits; '
              f'constant {f["const"][0]:.3f} LL {f["const"][1]:.1f}; {a:.3f} (1 - {b:.3f})^(K-4) LL {L:.1f}; '
              f'strict S {f["strict"][0]:.2f} LL {f["strict"][1]:.1f}; power a {f["power"][0][1]:.2f} LL {f["power"][1]:.1f}')


if __name__ == '__main__':
    common.main(claims)
    if common.have_runs(): print(); detail()
