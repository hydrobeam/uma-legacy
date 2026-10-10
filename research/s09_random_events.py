"""Section 9: how often a support card's character's shared random event fires, from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it).

Markers (GameTora training_events_shared, through gametora.py): a character's shared event with two or more choices
whose top choice gives a skill in every outcome and whose other choices give none; the skill is her marker (one per
character). A reading is one run with a card of hers in the deck (any rarity, group cards aside), someone else
training, and the marker not given by that card's own events; a hit is the marker credited to "<her name> event"
("A or B event" counts as a miss). K = random events on offer: every shared event of the deck's characters plus the
trainee's own. The first fit used the first 1,850 records, the larger sample the first 2,653, the priority-list check
the first 1,300.
The fits are maximum likelihood over readings: constant, 0.21 (1 - b)^(K - 4) form, power law, strict competition
1 - (1 - 1/K)^S, and every event slot competing (chain links, random events and pals weighted in one pool).
As stated in RESEARCH.md, the population includes R cards; `detail()` prints the SR/SSR-only fit beside it.
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
        dk = [d[0] for d in r['deck']]; tr = r['chara']
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
            rows.append(dict(run=r['id'], card=cid, rar=x['rarity'], hit=int(hit), amb=int(amb), K=K, pals=pals, links=links))
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


ITEMS = [('readings', '2,871'), ('runs', '1,461'), ('clean hits', '426'), ('ambiguous credits', '58'),
         ('rate with ambiguous, points higher', '2'),
         ('constant p', '0.148'), ('LL constant', '−1205.5'),
         ('fitted form: P at K = 4', '0.21'), ('fitted form: 1 - ratio per event', '0.047'), ('LL fitted form', '−1199.9'),
         ('power law a', '0.39'), ('LL power law', '−1200.7'),
         ('strict competition S', '1.57'), ('LL strict competition', '−1213.4'),
         ('every slot competing: c', '1.85'), ('every slot competing: pal weight', '0.94'),
         ('every slot competing: chain-link weight', '0'), ('LL every slot competing', '−1200.1'),
         ('hit rate K = 4', '0.17'), ('hit rate K = 6', '0.20'), ('hit rate K = 8', '0.19'), ('hit rate K = 10', '0.17'),
         ('hit rate K = 12', '0.14'), ('hit rate K = 14', '0.13'),
         ('fitted P overall', '0.15'), ('fitted P at K = 4', '0.21'), ('fitted P at K = 14', '0.13'),
         ('larger sample readings', '4,748'), ('larger sample runs', '2,264'),
         ('larger sample: rate at K = 8', '15.7%'), ('larger sample: rate at K = 14', '12.5%'),
         ('runs showing a priority list (first 1,300 records)', '175'),
         ('bottom-choice markers prioritised there', '0')]


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
    shown = n = 0
    for r in R:
        p = r.get('priority')
        if p is None: continue
        shown += 1
        for cid in (d[0] for d in r['deck']):
            x = sc.get(cid)
            if not x or x.get('type') == 'group' or x['char_id'] == r['chara']: continue
            n += sum(s in p for s in bottom[x['char_id']] - cardsk[cid])
    return shown, n


def claims():
    R = common.runs()
    if R is None: return common.need_data(SEC, ITEMS)
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
    g['larger sample: rate at K = 8'] = 1 / (1 + math.exp(-(b0 + 8 * b1)))
    g['larger sample: rate at K = 14'] = 1 / (1 + math.exp(-(b0 + 14 * b1)))
    g['runs showing a priority list (first 1,300 records)'], g['bottom-choice markers prioritised there'] = \
        bottom_prioritised([r for r in R if r['seq'] < PRIO])
    out = []
    for name, s in ITEMS:
        tol = 0.1 if name.startswith('LL') else 0
        note = 'about' if 'points' in name else ''
        out.append(common.Claim(SEC, name, s, g[name], tol=tol, note=note))
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
