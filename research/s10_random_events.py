"""Section 10: how often a support card's character's shared random event fires, from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it), and what leaving these events out of
the model does on real careers. Every run record is used; one sample.

Markers (GameTora training_events_shared, through gametora.py): a character's shared event with two or more choices
whose top choice gives a skill in every outcome and whose other choices give none; the skill is her marker (one per
character). A reading is one run with a card of hers in the deck (any rarity, group cards aside), someone else
training, and the marker not given by that card's own events; a hit is the marker credited to "<her name> event"
("A or B event" counts as a miss). K = random events on offer: every shared event of the deck's characters plus the
trainee's own.
The fits are maximum likelihood over readings: constant, a (1 - b)^(K - 4) form, power law, strict competition
1 - (1 - 1/K)^S, every event slot competing (chain links, random events and pals weighted in one pool), and a logistic
model in K. As stated in RESEARCH.md, the population includes R cards; `detail()` prints the SR/SSR-only fit beside it.
Findings: held-out comparisons of the forms (5-fold cross-validation by common.fold(run id), each run scored by the
forms fitted on the other folds, compared with common.compare_held_out), likelihood-ratio tests between nested forms and
on profiled parameters (uniform competition, chain-link and pal
weights), within-run independence (pairs of hits against Binomial(n, p) per run), the logistic slope, and the covariates
next to K (card effects at their limit break from master.mdb; rarity within a character); and, over every run that shows
a priority list, how often a bottom-choice skill of these events was prioritised and credited to the event (prioritised
or not), against the pooled top-choice marker rate. The pal weight's interval is its profile likelihood in the c / T form.

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
              without crediting: obs / pred with Wilson 95% intervals, and the Bernoulli log-likelihood difference;
              held out, the same rows by 5-fold cross-validation (the rate fitted on the other folds' readings), per-run
              log-likelihoods credited against left out
"""
import collections, math, os
import numpy as np
from scipy.optimize import minimize
import common

SEC = '10'
GEN_W, GEN_G = 0.2, 0.4          # section 12's W_k, G_k at n_k = 0


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


def offer(r, nshared, sc):
    """K: random events on offer in run r (every shared event of the deck's characters plus the trainee's own)"""
    chars = {sc[x]['char_id'] for x in (d[0] for d in r['deck']) if x in sc and sc[x].get('type') != 'group'}
    return sum(nshared[ch] for ch in chars - {r['chara']}) + nshared[r['chara']]


def readings(R):
    nshared, marker, cardsk, sc = markers()
    name = dict(common.mdb().execute('select [index], text from text_data where category = 6'))
    wc = common.cards()
    rows = []
    for r in R:
        dk = [d[0] for d in r['deck']]; tr = r['chara']; lbs = {d[0]: d[1] for d in r['deck']}
        pals = sum(1 for x in dk if wc.get(x, {}).get('pal'))
        links = sum(wc.get(x, {}).get('links', 0) for x in dk if not wc.get(x, {}).get('pal'))
        K = offer(r, nshared, sc)
        for cid in dk:
            x = sc.get(cid)
            if not x or x.get('type') == 'group': continue
            ch = x['char_id']; m = marker.get(ch)
            if not m or ch == tr or m in cardsk[cid]: continue
            nm = name.get(ch); srcs = [s for h in r['hints'] if h['name'] == m for s in h['src']]
            hit = f'{nm} event' in srcs
            amb = not hit and any(nm in s and ' or ' in s for s in srcs)
            # censored: the marker already at hint Lv 5 from other sources, so the event's hint could not show
            cens = not hit and not amb and max([h['lv'] or 0 for h in r['hints'] if h['name'] == m] or [0]) >= 5
            rows.append(dict(run=r['id'], card=cid, rar=x['rarity'], hit=int(hit), amb=int(amb), cens=int(cens), K=K, pals=pals, links=links,
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
    for i, fn in enumerate(SLOT_FNS):
        o = minimize(lambda p: -ll(y, fn(p[0], p[2] * Lk + K + p[1] * Pl)), [3.0, 3.0, 1.0],
                     bounds=[(0.01, 200), (0, 30), (0, 10)])
        if best is None or -o.fun > best[1]: best = (tuple(o.x), -o.fun); out['slots form'] = i
    out['slots'] = best
    return out


SLOT_FNS = (lambda s, T: 1 - (1 - 1 / T) ** s, lambda s, T: s / T)


def form_p(f, rows):
    """each form's P(fires) on `rows`, with the parameters of fits() output f"""
    K = np.array([r['K'] for r in rows], float)
    Lk = np.array([r['links'] for r in rows], float); Pl = np.array([r['pals'] for r in rows], float)
    (a, b), (c, pa), S, (sc, sw, sl) = f['exp'][0], f['power'][0], f['strict'][0], f['slots'][0]
    return {'const': np.full_like(K, f['const'][0]), 'exp': a * (1 - b) ** (K - 4), 'power': c * (K / 8) ** (-pa),
            'strict': 1 - (1 - 1 / K) ** S, 'slots': SLOT_FNS[f['slots form']](sc, sl * Lk + K + sw * Pl)}


def held_out(rows):
    """rule 3: per-run held-out log-likelihood of each form. Runs fall in five folds by common.fold(run id), all of a
    run's readings in one; each fold's readings are scored by the forms fitted (fits()) on the other four folds. Returns
    form -> list over runs (sorted by id) of the run's summed Bernoulli log-likelihood"""
    runs = sorted({r['run'] for r in rows}); out = collections.defaultdict(lambda: collections.defaultdict(float))
    for k in range(common.K_FOLDS):
        test = [r for r in rows if common.fold(r['run']) == k]
        P = form_p(fits([r for r in rows if common.fold(r['run']) != k]), test)
        y = np.array([r['hit'] for r in test], float)
        for nm, p in P.items():
            p = np.clip(p, 1e-9, 1 - 1e-9); lr = y * np.log(p) + (1 - y) * np.log(1 - p)
            for r, v in zip(test, lr): out[nm][r['run']] += float(v)
    return {nm: [d[i] for i in runs] for nm, d in out.items()}


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
    name = dict(common.mdb().execute('select [index], text from text_data where category = 6'))
    shown = n = cand = 0; hits = collections.Counter()      # (prioritised, credited to her event) -> candidates
    for r in R:
        p = r.get('priority')
        if p is None: continue
        shown += 1
        for cid in (d[0] for d in r['deck']):
            x = sc.get(cid)
            if not x or x.get('type') == 'group' or x['char_id'] == r['chara']: continue
            ev = f'{name.get(x["char_id"])} event'
            for sk_ in bottom[x['char_id']] - cardsk[cid]:
                n += sk_ in p; cand += 1
                hits[(sk_ in p, any(h['name'] == sk_ and ev in h['src'] for h in r['hints']))] += 1
    return shown, n, cand, hits


def claims():
    R = common.runs()
    if R is None: return []
    rows = readings(R)
    f = fits(rows); g = {}
    y = sum(r['hit'] for r in rows); n = len(rows); amb = sum(r['amb'] for r in rows)
    g.update({'readings': n, 'runs': len({r['run'] for r in rows}), 'clean hits': y, 'ambiguous credits': amb,
              'rate with ambiguous, points higher': 100 * amb / n})
    cz = sum(r['cens'] for r in rows)
    g.update({'readings where the event could not show (marker at Lv 5 from other sources)': cz,
              'rate where the event could show': y / (n - cz), 'rate where it could show, ambiguous as hits': (y + amb) / (n - cz)})
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
    b0, b1 = logistic_k(rows)
    for rr, lab in ((3, 'SSR'), (2, 'SR')):
        xs = [r['hit'] for r in rows if r['rar'] == rr]; g[f'pooled: {lab} marker rate'] = sum(xs) / len(xs)
    g['logistic: rate at K = 8'] = 1 / (1 + math.exp(-(b0 + 8 * b1)))
    g['logistic: rate at K = 14'] = 1 / (1 + math.exp(-(b0 + 14 * b1)))
    g['runs showing a priority list'], g['bottom-choice markers prioritised there'], cand, bh = bottom_prioritised(R)
    g['bottom-choice candidates there'] = cand
    g['bottom-choice, not prioritised: credited to her event'] = bh[(False, True)]
    g['bottom-choice, prioritised: credited to her event'] = bh[(True, True)]
    g['every slot competing: LL over constant'] = f['slots'][1] - f['const'][1]
    g['every slot competing: pal weight 95% low'], g['every slot competing: pal weight 95% high'] = pal_interval(rows)
    g['bottom-choice, prioritised: credited rate'] = common.div(bh[(True, True)], bh[(True, True)] + bh[(True, False)])
    g['bottom-choice, not prioritised: credited rate'] = nb = common.div(bh[(False, True)], bh[(False, True)] + bh[(False, False)])
    g['bottom-choice, not prioritised: 95% low'], g['bottom-choice, not prioritised: 95% high'] = \
        common.wilson(bh[(False, True)], bh[(False, True)] + bh[(False, False)])
    g['top-choice marker rate'] = y / n
    h = held_out(rows); g['held out: runs'] = len(h['exp'])
    for lab, x, z in HELD_OUT:
        g[f'held out: {lab}: nats'], g[f'held out: {lab}: SE'], g[f'held out: {lab}: z'] = common.compare_held_out(h[x], h[z])
    out = [common.Value(SEC, name, v) for name, v in g.items()] + findings(rows, f, g, cand, bh)
    return out + careers(R, (a, b), rows)


# rule 3's comparisons of the forms: (label, first form, second form); nats, SE and z are first minus second
HELD_OUT = (('fitted form against constant', 'exp', 'const'), ('every slot competing against fitted form', 'slots', 'exp'),
            ('power law against fitted form', 'power', 'exp'), ('strict competition against fitted form', 'strict', 'exp'))


# ---------------------------------------------------------------- findings: each conclusion as a likelihood or interval test
def slot_ll(rows, link=None, pal=None):
    """max LL of c / T, T = link x chain links + K + pal x pals, with either weight held fixed"""
    y = np.array([r['hit'] for r in rows], float); K = np.array([r['K'] for r in rows], float)
    Lk = np.array([r['links'] for r in rows], float); Pl = np.array([r['pals'] for r in rows], float)
    fx = lambda v, x: x if v is None else v
    o = minimize(lambda q: -ll(y, q[0] / (fx(link, q[2]) * Lk + K + fx(pal, q[1]) * Pl)), [3.0, 1.0, 0.5],
                 bounds=[(0.01, 200), (0, 30), (0, 10)])
    return -o.fun


def pal_interval(rows):
    """the pal weight's 95% profile-likelihood interval in the c / T form (chain-link weight and c free): where the
    profile falls 1.92 nats below its maximum"""
    from scipy.optimize import brentq, minimize_scalar
    prof = lambda w: slot_ll(rows, pal=w)
    o = minimize_scalar(lambda w: -prof(w), bounds=(0, 10), method='bounded'); top, w0 = -o.fun, o.x
    d = lambda w: top - prof(w) - 1.92
    lo = brentq(d, 0.0, w0) if d(0.0) > 0 else 0.0
    hi = brentq(d, w0, 10.0) if d(10.0) > 0 else float('nan')
    return lo, hi


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


def findings(rows, f, g, cand, bh):
    F = lambda st, test, ev: common.finding(SEC, st, test, ev)
    from scipy.stats import chi2
    L0, (ab, Le), (pw, Lp), (_, Ls), Lslot = f['const'][1], f['exp'], f['power'], f['strict'], f['slots'][1]
    out = [F('The slope in K is detectable: the adopted form a (1 - b)^(K - 4) against a constant (b = 0, nested), '
             'LR = 2 x (LL difference) above 3.84 (chi2, df 1, 95%)', 2 * (Le - L0) > 3.84,
             f'a {ab[0]:.2f}, b {ab[1]:.3f}: LL {Le:.1f} against {L0:.1f}: LR {2 * (Le - L0):.2f}')]
    out += held_out_findings(g)
    y = np.array([r['hit'] for r in rows], float); K = np.array([r['K'] for r in rows], float)
    L1 = -minimize(lambda q: -ll(y, q[0] * (K / 8) ** -1.0), [0.17], bounds=[(0.01, 0.9)]).fun
    out.append(F('Competition is far weaker than uniform: in the power law, a = 1 (uniform competition) is rejected against '
                 'the fitted a, LR above 3.84 (chi2, df 1)', 2 * (Lp - L1) > 3.84,
                 f'a = {pw[1]:.2f}: LL {Lp:.1f}; a = 1: LL {L1:.1f}; LR {2 * (Lp - L1):.2f}'))
    out.append(F('Strict competition 1 - (1 - 1/K)^S has a lower in-sample log-likelihood than a constant, which has the '
                 'same number of parameters (one)', Ls < L0, f'LL {Ls:.1f} against constant {L0:.1f}'))
    Lk1 = slot_ll(rows, link=1.0)
    out.append(F('Chain links play no part: their fitted weight is 0, and weight 1 (a link competing like a random event) '
                 'is rejected, LR above 3.84 (chi2, df 1)', g['every slot competing: chain-link weight'] < 0.01 and 2 * (Lslot - Lk1) > 3.84,
                 f'weight {g["every slot competing: chain-link weight"]:.2f}: LL {Lslot:.1f}; weight 1: LL {Lk1:.1f}; '
                 f'LR {2 * (Lslot - Lk1):.2f}'))
    plo, phi = g['every slot competing: pal weight 95% low'], g['every slot competing: pal weight 95% high']
    out.append(F('Pals compete with random events: pal weight 0 is rejected (outside the 95% profile-likelihood interval, '
                 '1.92 nats)', plo > 0,
                 f'weight {g["every slot competing: pal weight"]:.2f} [{plo:.2f}, {phi:.2f}]; LL at 0 {slot_ll(rows, pal=0.0):.1f}, '
                 f'at 1 {slot_ll(rows, pal=1.0):.1f}, maximum {slot_ll(rows):.1f}'))
    o, e, z = pairs_test(rows, *ab)
    out.append(F('Within a run events fire independently: pairs of one run\'s readings that both hit are within 1.96 sd of '
                 'the count independence predicts', abs(z) < 1.96, f'{o:.0f} pairs observed, {e:.1f} expected, z {z:+.2f}'))
    yb = np.array([r['hit'] for r in rows], float); one = np.ones(len(rows)); Kb = np.array([r['K'] for r in rows], float)
    base = logit_ll(yb, np.column_stack([one, Kb])); Lc = logit_ll(yb, one[:, None])
    out.append(F('In logistic form too the slope is detectable: K\'s LR above 3.84 (chi2, df 1), and the rate falls',
                 2 * (base - Lc) > 3.84 and g['logistic: rate at K = 14'] < g['logistic: rate at K = 8'],
                 f'LR {2 * (base - Lc):.2f}; {g["logistic: rate at K = 8"]:.3f} at 8, {g["logistic: rate at K = 14"]:.3f} at 14'))
    # covariates next to K: likelihood-ratio tests at 95%
    lr = lambda cols: 2 * (logit_ll(yb, np.column_stack([one, Kb] + cols)) - base)
    eff = {}
    for t, nm in ((1, 'friendship bonus'), (14, 'initial friendship gauge'), (19, 'specialty priority')):
        cache = {}
        x = np.array([cache.setdefault((r['card'], r['lb']), card_effect(r['card'], r['lb'], t)) for r in rows]) / 10
        eff[nm] = (lr([x]), 1)
    sc = sorted({r['scen'] for r in rows})[1:]
    eff['scenario'] = (lr([np.array([r['scen'] == v for r in rows], float) for v in sc]), len(sc))
    out.append(F('Friendship (bonus, initial gauge), specialty priority and scenario show no effect next to K: each '
                 'likelihood-ratio statistic below its chi2 95% point', all(s < chi2.ppf(0.95, d) for s, d in eff.values()),
                 '; '.join(f'{k} {s:.2f} (df {d}, limit {chi2.ppf(0.95, d):.2f})' for k, (s, d) in eff.items())))
    rar = np.array([r['rar'] for r in rows], float)
    multi = collections.defaultdict(set)
    for r in rows: multi[r['char']].add(r['rar'])
    ch = sorted(c for c, v in multi.items() if len(v) > 1)
    w = [i for i, r in enumerate(rows) if r['char'] in ch]
    D = np.array([[rows[i]['char'] == c for c in ch] for i in w], float)
    fe = logit_ll(yb[w], np.column_stack([D, Kb[w]])); fe_r = logit_ll(yb[w], np.column_stack([D, Kb[w], rar[w]]))
    out.append(F('Rarity shows no effect within a character (character intercepts and K, characters with cards of more than '
                 'one rarity): likelihood-ratio statistic below 3.84; pooled across characters it is shown alongside',
                 2 * (fe_r - fe) < 3.84, f'within {len(ch)} characters ({len(w):,} readings): {2 * (fe_r - fe):.2f}; pooled: {lr([rar]):.2f}'))
    out.append(F('Bottom-choice skills are credited to the character\'s event less often than top-choice markers: the '
                 'non-prioritised bottom-choice rate\'s 95% interval lies entirely below the pooled top-choice marker rate',
                 g['bottom-choice, not prioritised: 95% high'] < g['top-choice marker rate'],
                 f'not prioritised {bh[(False, True)]}/{bh[(False, True)] + bh[(False, False)]} = '
                 f'{g["bottom-choice, not prioritised: credited rate"]:.3f} [{g["bottom-choice, not prioritised: 95% low"]:.3f}, '
                 f'{g["bottom-choice, not prioritised: 95% high"]:.3f}]; prioritised {bh[(True, True)]}/{bh[(True, True)] + bh[(True, False)]}; '
                 f'top-choice {g["top-choice marker rate"]:.3f}; {g["bottom-choice markers prioritised there"]} of {cand:,} (run, skill) '
                 f'candidates prioritised in {g["runs showing a priority list"]} runs'))
    return out


def held_out_findings(g):
    """rule 3 on the forms, criteria fixed before the numbers: the adopted form predicts unseen runs better than a
    constant (z > 1.96); the every-slot form predicts them about as well as the adopted one (|z| <= 1.96); the power law
    and strict competition do not predict them better than the adopted form (z <= 1.96, worded by what the data give)"""
    F = lambda st, test, ev: common.finding(SEC, st, test, ev)
    v = lambda lab: (g[f'held out: {lab}: nats'], g[f'held out: {lab}: SE'], g[f'held out: {lab}: z'])
    ev = lambda lab: f'{g["held out: runs"]:,} runs, 5 folds: {v(lab)[0]:+.2f} nats, SE {v(lab)[1]:.2f}, z {v(lab)[2]:+.2f} ({lab})'
    out = [F('The slope in K does not predict unseen runs better than a constant rate: the form a (1 - b)^(K - 4) against a '
             'constant has z <= 1.96', v('fitted form against constant')[2] <= 1.96, ev('fitted form against constant')),
           F('The every-slot form does not predict unseen runs better than the adopted form, nor worse (|z| <= 1.96)',
             abs(v('every slot competing against fitted form')[2]) <= 1.96, ev('every slot competing against fitted form'))]
    out.append(F('The power law does not predict unseen runs better than the K form (z <= 1.96)',
                 v('power law against fitted form')[2] <= 1.96, ev('power law against fitted form')))
    out.append(F('Strict competition predicts unseen runs worse than the K form (z < -1.96)',
                 v('strict competition against fitted form')[2] < -1.96, ev('strict competition against fitted form')))
    return out


# ---------------------------------------------------------------- effect on real careers (section 13's rows)
def calib():
    """section 13's calibration rows and per-run rows (default targets, each run on its own deck), exactly as
    s13_calibration.claims builds them; the solver's results are reused from its cache while nothing they read changes"""
    if 'calib' not in common._cache:
        import s13_calibration as s13
        R = common.runs(); M = s13.Model()
        s13.card_variants()
        jobdir = os.path.join(s13.OUTD, 'jobs8')
        man, files, _ = s13.build_jobs(R, M, False, jobdir)
        res = s13.run_solver(jobdir, files, os.path.join(s13.OUTD, 'res8.jsonl'))
        rows, runrows, _ = s13.rows_of(R, M, res, man)
        common._cache['calib'] = (rows, runrows)
    return common._cache['calib']


def event_groups():
    """skill groups each kind of event can hint (GameTora, through gametora.py): a trainee's story and date events
    (training_events_char), each outfit's own events (training_events_char_card), a character's shared events
    (training_events_shared), a trainee's current conditional events with their conditions, each scenario's events
    (training_events_scenario; its ids are the game's scenario ids)"""
    if 'evgroups' in common._cache: return common._cache['evgroups']
    gt = common.gametora(); grp = dict(common.mdb().execute('select id, group_id from skill_data'))

    def of(chs):
        return {grp.get(x) for ch in gt.choices(chs) for kind, x0, _ in gt.hints(ch[1]) for x in ([x0] if kind == 'hint' else x0)}

    def of_list(lst):
        return set().union(set(), *[of(e[1]) for e in (lst if isinstance(lst, list) else []) if isinstance(e, list) and len(e) >= 2])
    ordinary, cond = {}, collections.defaultdict(list)
    for row in gt.load('training_events_char'):
        ordinary[row[0]] = of_list(row[2]) | of_list(row[3])
        for tag, ve in gt.conditional_events(row):
            if tag.startswith('current'): cond[row[0]].append((of(ve[1]), gt.extra(ve, 0)))
    outfit = {row[0]: of_list(row[1]) for row in gt.load('training_events_char_card')}
    shared = {ch: of_list(es) for ch, es in gt.load('training_events_shared')}
    scen = {row[0]: set().union(set(), *[of_list(part) for part in row[1:]]) for row in gt.load('training_events_scenario')}
    common._cache['evgroups'] = out = dict(ordinary=ordinary, cond=cond, outfit=outfit, shared=shared, scen=scen)
    return out


def hint_srcs(r):
    """skill group -> the run's hint records of it (white or gold), names mapped as section 13 maps them"""
    names = common.skill_names(); pn = {v[1]: gid for gid, vs in r.get('hint_groups', []) for v in vs}
    out = collections.defaultdict(list)
    for h in r['hints']:
        g = names[h['name']][0] if h['name'] in names else pn.get(h['name'])
        if g is not None: out[g].append(h)
    return out


def free(row): return not row['outfit'] and not row['scen_gold']


def credit(row, r):
    """(p, acquisition) of a calibration row with m multiplied by (1 - r) (section 12; m recovered from the row's p);
    a row on the trainee's outfit or a scenario gold does not move"""
    if not free(row): return row['p'], row['acq']
    s = 1.1 ** row['nc']; dg = row['dg']
    if dg >= 1: return row['p'], 1.0
    m = 1 - (row['p'] - dg * GEN_G * s) / ((1 - dg) * GEN_W * s); mm = m * (1 - r)
    return dg * GEN_G * s + (1 - dg) * GEN_W * s * (1 - mm), dg + (1 - dg) * (1 - mm)


def odds_change(runrows, rate):
    """relative change of each run's product of per-target odds when each row is credited at rate(row) (0: not): over
    the eight targets (runs with a nonzero product; those of them with a credited row; runs whose product crediting
    lifts from 0), and over the targets the model gives a positive chance without crediting (runs with a credited
    row among them)"""
    ch, tch, lifted, pos = [], [], 0, []
    for pr in runrows:
        new = [credit(x, rate(x))[0] for x in pr['rows']]
        p0 = math.prod(x['p'] for x in pr['rows']); p1 = math.prod(new)
        touched = [rate(x) > 0 and free(x) for x in pr['rows']]
        if any(t and x['p'] > 0 for t, x in zip(touched, pr['rows'])):
            pos.append(math.prod(q for q, x in zip(new, pr['rows']) if x['p'] > 0) / math.prod(x['p'] for x in pr['rows'] if x['p'] > 0) - 1)
        if p0 <= 0: lifted += p1 > 0; continue
        ch.append(p1 / p0 - 1)
        if any(touched): tch.append(p1 / p0 - 1)
    return ch, tch, lifted, pos


def summary(xs):
    return (float(np.mean(xs)), float(np.median(xs)), float(np.max(xs))) if xs else (float('nan'),) * 3


def calib_test(rs, rate):
    """acquisition observed (hinted from any source, or owned) against predicted, without and with crediting each row at
    rate(row): rows, observed, predicted, obs / pred and its 95% interval each way; the Bernoulli log-likelihood each way
    over the rows whose prediction without crediting is positive (a row predicted at 0 and acquired has no finite
    likelihood; those rows are counted apart)"""
    n = len(rs); k = sum(1 for x in rs if x['o_any']); lo, hi = common.wilson(k, n)
    out = dict(n=n, k=k, obs=common.div(k, n), lo=lo, hi=hi)
    p0 = [credit(x, 0.0)[1] for x in rs]; p1 = [credit(x, rate(x))[1] for x in rs]
    pos = [i for i, q in enumerate(p0) if q > 1e-12]
    for tag, pr in (('0', p0), ('1', p1)):
        ep = common.div(sum(pr), n)
        out.update({f'pred{tag}': ep, f'ratio{tag}': common.div(common.div(k, n), ep),
                    f'rlo{tag}': common.div(lo, ep), f'rhi{tag}': common.div(hi, ep),
                    f'll{tag}': sum(math.log(pr[i]) if rs[i]['o_any'] else math.log(1 - pr[i]) for i in pos if 0 < pr[i] < 1)
                    + sum(0.0 if (pr[i] >= 1) == bool(rs[i]['o_any']) else -math.inf for i in pos if not 0 < pr[i] < 1)})
    out['n_pos'] = len(pos); out['n_zero'] = n - len(pos)
    return out


def random_rows(R, ab):
    """the calibration rows a random event could hint: (row, kind, rate, hit, only, ambiguous); kind 'deck' (a deck
    card's character's event, at the fitted form at the run's K) or 'own' (the trainee's own, rate filled in later)"""
    a, b = ab
    nshared, marker, cardsk, sc = markers(); E = event_groups()
    name = dict(common.mdb().execute('select [index], text from text_data where category = 6'))
    gn = common.skills(); white = {g: gn[g][0] for g in common.DEFAULT}
    rows, _ = calib(); byid = {r['id']: r for r in R}
    out, skipped = [], collections.Counter()
    for row in rows:
        if not free(row): continue
        r = byid[row['run']]; g = row['g']; hs = hint_srcs(r)[g]; srcs = [x for h in hs for x in h['src']]
        K = offer(r, nshared, sc)
        for cid in (d[0] for d in r['deck']):
            x = sc.get(cid)
            if not x or x.get('type') == 'group' or marker.get(x['char_id']) != white[g]: continue
            if x['char_id'] == r['chara']: skipped['own character'] += 1; continue
            if white[g] in cardsk[cid]: skipped['card\'s own events give it'] += 1; continue
            nm = name.get(x['char_id']); ev = f'{nm} event'
            out.append(dict(row=row, kind='deck', who=nm, rate=a * (1 - b) ** (K - 4), hit=ev in srcs,
                            only=ev in srcs and set(srcs) == {ev} and g not in {s // 10 for s in r.get('owned', [])},
                            amb=ev not in srcs and any(nm in t and ' or ' in t for t in srcs)))
        u = r['chara']
        if marker.get(u) == white[g]:
            other = E['ordinary'].get(u, set()) | E['outfit'].get(r['outfit'], set()) | E['scen'].get(r['scenario'], set()) \
                | set().union(set(), *[gs for gs, _ in E['cond'].get(u, [])])
            if g in other: skipped['trainee\'s own, not identifiable'] += 1; continue
            out.append(dict(row=row, kind='own', who=name.get(u), rate=None, hit='Events' in srcs,
                            only=set(srcs) == {'Events'} and g not in {s // 10 for s in r.get('owned', [])}, amb=False))
    return out, skipped


def bern_ll(pr, acquired):
    """one row's Bernoulli log-likelihood (as calib_test scores it)"""
    if 0 < pr < 1: return math.log(pr) if acquired else math.log(1 - pr)
    return 0.0 if (pr >= 1) == bool(acquired) else -math.inf


def careers_held_out(dk, R, rows):
    """rule 3 on the calibration rows: per-run held-out log-likelihood of the acquisition rows a deck card's random
    event could hint, credited against left out. Runs fall in five folds by common.fold(run id); each fold's rows are
    credited at the adopted form fitted on the other four folds' readings. Scored on the rows the model gives a positive
    chance without crediting (as calib_test). Returns (credited, left out) per run, and the number of rows"""
    nshared, _, _, sc = markers(); byid = {r['id']: r for r in R}
    l1, l0 = collections.defaultdict(float), collections.defaultdict(float); n = 0
    for k in range(common.K_FOLDS):
        a, b = fits([r for r in rows if common.fold(r['run']) != k])['exp'][0]
        for x in dk:
            row = x['row']
            if common.fold(row['run']) != k: continue
            p0 = credit(row, 0.0)[1]
            if p0 <= 1e-12: continue
            p1 = credit(row, a * (1 - b) ** (offer(byid[row['run']], nshared, sc) - 4))[1]
            l0[row['run']] += bern_ll(p0, row['o_any']); l1[row['run']] += bern_ll(p1, row['o_any']); n += 1
    runs = sorted(l0)
    return [l1[i] for i in runs], [l0[i] for i in runs], n


def careers(R, ab, rows):
    F = lambda st, test, ev: common.finding(SEC, st, test, ev)
    V = lambda nm, v: common.Value(SEC, nm, v)
    rr, skipped = random_rows(R, ab); _, runrows = calib()
    dk = [x for x in rr if x['kind'] == 'deck']; own = [x for x in rr if x['kind'] == 'own']
    ko = sum(x['hit'] for x in own); lo_o, hi_o = common.wilson(ko, len(own)); r_own = common.div(ko, len(own))
    for x in own: x['rate'] = r_own
    rate = {id(x['row']): x['rate'] for x in rr}
    t = calib_test([x['row'] for x in dk], lambda row: rate[id(row)])
    zero = [x for x in dk if credit(x['row'], 0.0)[1] <= 1e-12]
    # held out: the form refitted without the readings of these rows' cards in these runs
    used = {(x['row']['run'], x['who']) for x in dk}
    name = dict(common.mdb().execute('select [index], text from text_data where category = 6'))
    ha, hb = fits([r for r in readings(R) if (r['run'], name.get(r['char'])) not in used])['exp'][0]
    nshared, _, _, sc = markers(); byid = {r['id']: r for r in R}
    hrate = {id(x['row']): ha * (1 - hb) ** (offer(byid[x['row']['run']], nshared, sc) - 4) for x in dk}
    th = calib_test([x['row'] for x in dk], lambda row: hrate[id(row)])
    c1, c0, nho = careers_held_out(dk, R, rows); hd, hse, hz = common.compare_held_out(c1, c0)
    ch, tch, lifted, pos = odds_change(runrows, lambda row: rate.get(id(row), 0.0))
    out = [V('careers: rows a deck card\'s random event could hint', len(dk)),
           V('careers: runs with such a row', len({x['row']['run'] for x in dk})),
           V('careers: acquired through that event', sum(x['hit'] for x in dk)),
           V('careers: acquired through that event only', sum(x['only'] for x in dk)),
           V('careers: ambiguous credits there', sum(x['amb'] for x in dk)),
           V('careers: mean fitted rate there', float(np.mean([x['rate'] for x in dk])) if dk else float('nan')),
           V('careers: rows the model gives no chance', len(zero)),
           V('careers: of them acquired', sum(x['row']['o_any'] for x in zero)),
           V('careers: of them acquired through that event', sum(x['hit'] for x in zero)),
           V('careers: rows of the trainee\'s own event, identifiable', len(own)),
           V('careers: trainee\'s own, rows not identifiable', skipped['trainee\'s own, not identifiable']),
           V('careers: trainee\'s own, acquired through her events', ko),
           V('careers: trainee\'s own, rate', r_own), V('careers: trainee\'s own, rate 95% low', lo_o),
           V('careers: trainee\'s own, rate 95% high', hi_o)]
    for lab, xs in (('', ch), (', runs with a credited row', tch), (', targets with a positive chance, runs with a credited row', pos)):
        m, md, mx = summary(xs)
        out += [V(f'odds change{lab}: runs', len(xs)), V(f'odds change{lab}: mean', m), V(f'odds change{lab}: median', md),
                V(f'odds change{lab}: max', mx)]
    out += [V('odds change: runs with a credited row and a zero product', len({x['row']['run'] for x in dk}) - len(tch)),
            V('odds change: runs lifted from zero', lifted)]
    pre = 'calibration'
    out += [V(f'{pre}: rows', t['n']), V(f'{pre}: acquired', t['k']), V(f'{pre}: observed', t['obs']),
            V(f'{pre}: predicted, left out', t['pred0']), V(f'{pre}: obs / pred, left out', t['ratio0']),
            V(f'{pre}: ratio 95% low, left out', t['rlo0']), V(f'{pre}: ratio 95% high, left out', t['rhi0']),
            V(f'{pre}: predicted, credited', t['pred1']), V(f'{pre}: obs / pred, credited', t['ratio1']),
            V(f'{pre}: ratio 95% low, credited', t['rlo1']), V(f'{pre}: ratio 95% high, credited', t['rhi1']),
            V(f'{pre}: rows with a positive prediction', t['n_pos']),
            V(f'{pre}: LL credited minus left out, positive rows', t['ll1'] - t['ll0']),
            V(f'{pre}, rate held out: obs / pred, credited', th['ratio1']),
            V(f'{pre}, rate held out: ratio 95% low, credited', th['rlo1']), V(f'{pre}, rate held out: ratio 95% high, credited', th['rhi1']),
            V(f'{pre}, rate held out: LL credited minus left out, positive rows', th['ll1'] - th['ll0']),
            V('held out: credited against left out: runs', len(c1)), V('held out: credited against left out: rows', nho),
            V('held out: credited against left out: nats', hd), V('held out: credited against left out: SE', hse),
            V('held out: credited against left out: z', hz)]
    rt = lambda t, k: f'{t["ratio" + k]:.3f} [{t["rlo" + k]:.2f}, {t["rhi" + k]:.2f}]'
    covers = lambda t, k: t[f'rlo{k}'] <= 1 <= t[f'rhi{k}']
    out += [F('Deck cards\' random events, left out: acquisition on the rows they could hint is under-predicted: obs / pred '
              'has a 95% interval entirely above 1', lambda: t['rlo0'] > 1, lambda: f'{t["n"]} rows, {t["k"]} acquired; left out {rt(t, "0")}; credited {rt(t, "1")}'),
            F('Deck cards\' random events, credited at the fitted rate: acquisition obs / pred has a 95% interval covering 1',
              lambda: covers(t, '1'), lambda: f'credited {rt(t, "1")}; with the rate fitted without these rows {rt(th, "1")}'),
            F('Crediting deck cards\' random events at the fitted rate raises the log-likelihood of the rows the model gives a '
              'positive chance without them by more than 1.92 nats, with the rate fitted on every reading and with it fitted '
              'without these rows\' readings (a check on the size of the gain; the claim that crediting predicts better is '
              'the 5-fold test)', lambda: t['ll1'] - t['ll0'] > 1.92 and th['ll1'] - th['ll0'] > 1.92,
              lambda: f'{t["n_pos"]} rows: LL {t["ll0"]:.1f} left out, {t["ll1"]:.1f} credited ({t["ll1"] - t["ll0"]:+.1f} nats); '
                      f'rate held out {th["ll1"] - th["ll0"]:+.1f} nats'),
            F('Crediting deck cards\' random events at the fitted rate is not shown to predict unseen runs\' acquisition better '
              'than leaving them out: the held-out gain is positive but z <= 1.96', lambda: 0 < hd and hz <= 1.96,
              lambda: f'{len(c1)} runs, {nho} rows with a positive chance left out; rate fitted on the other four folds\' readings: '
                      f'{hd:+.2f} nats, SE {hse:.2f}, z {hz:+.2f} (credited against left out)'),
            F('Left out, the model gives some of these targets no chance at all, and the event hinted the target on at least one of them',
              lambda: len(zero) > 0 and sum(x['hit'] for x in zero) > 0,
              lambda: f'{len(zero)} rows at 0; {sum(x["row"]["o_any"] for x in zero)} acquired, {sum(x["hit"] for x in zero)} through the event')]
    return out


def careers_detail(R):
    a = fits(readings(R))['exp'][0]
    rr, skipped = random_rows(R, a)
    print('\ncareers: rows by who', collections.Counter((x['kind'], x['who'], x['row']['g'], x['hit']) for x in rr), dict(skipped))
    rows, _ = calib()
    dev = max(abs(credit(x, 0.0)[1] - x['acq']) for x in rows)
    print(f'careers: largest |acquisition from m recovered from p - the solver\'s acquisition| over {len(rows):,} rows: {dev:.2e}')


def detail():
    R = common.runs()
    rows = readings(R)
    print('all records: rate by K', ', '.join(f'{k}: {h}/{n}' for k, (n, h) in sorted(by_k(rows).items())))
    t = collections.defaultdict(lambda: [0, 0])
    for r in rows: t[r['rar']][0] += 1; t[r['rar']][1] += r['hit']
    print('   by rarity (1 R, 2 SR, 3 SSR):', ', '.join(f'{k}: {h}/{n} = {h / n:.3f}' for k, (n, h) in sorted(t.items())))
    sr = [r for r in rows if r['rar'] >= 2]; f = fits(sr); (a, b), L = f['exp']
    print(f'   SR/SSR only: {len(sr)} readings, {len({r["run"] for r in sr})} runs, {sum(r["hit"] for r in sr)} hits; '
          f'constant {f["const"][0]:.3f} LL {f["const"][1]:.1f}; {a:.3f} (1 - {b:.3f})^(K-4) LL {L:.1f}; '
          f'strict S {f["strict"][0]:.2f} LL {f["strict"][1]:.1f}; power a {f["power"][0][1]:.2f} LL {f["power"][1]:.1f}')
    careers_detail(R)


if __name__ == '__main__':
    common.main(claims)
    if common.have_runs(): print(); detail()
