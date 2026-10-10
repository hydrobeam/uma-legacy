"""Section 7 of docs/RESEARCH.md: chain events.

Reproduces the derived numbers (Binomial(21, 0.35), P(N >= 6), the exact allocation tables, the worked example) and
the fits on the run records (research/data/runs.jsonl, not included; Independent Training runs from uma-it).

Method. A card's chain progress is read from "marker" skills of its link events (GameTora card events through
gametora.py, chain links from master.mdb single_mode_story_data.show_progress_1) credited to "<character> event" in a
run's hint list. Two measurements were used, in this order:
  v1 (first fit): markers on IT's default (first) choice of a link's event, unique to that link among the card's events
     and not given by any other deck card's events; a link is left out when another choice holds a skill that could be
     prioritised (no priority list shown, or the list names one). q = share of the default choice's outcomes that
     carry a marker. Ambiguous "A or B event" credits count as seen. R-card decks were kept (no slot).
  v2 (corrected): a link is observable only if every choice and every outcome of its event carries a skill appearing
     in no other event of the card; "A or B event" credits naming the card count as seen; a marker listed with
     "Inspiration" as its only source is unknown. Decks with a non-pal R card are dropped.
Each stated number is recomputed on the record prefix and measurement it came from (each claim's note names them):
the fit of N and its per-scenario split on v1, first 879 records; the refit on v1, first 2,425 records (Binomial grid
MLE per pal count); the final-link table, early-link shortfall and per-card list on v2, first 1,300 records; the
allocation rules and the activation parameter on v1, first 1,193 records; chains never started on v1, first 679.
The model: N chain events per run; each advances one running non-pal card's chain by a link, the card picked with
weight = links left ("slots") or uniformly ("cards"); stop risk (card_events.json) ends a chain at link l with k/n.
Fits of N are exact over the chain states, with a 3% chance that a fired link's marker does not show; the allocation
and activation comparisons score each observation on its own (P(card reaches link) on the run's deck)."""
import collections, itertools, math, os, sys
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

S = '7'
EPS = 0.03     # chance a fired link's marker hint does not show (the first fit's allowance)
KMAX = 16      # distribution-free fit: P(N = 0..15) and P(N >= 16)


# ---------------------------------------------------------------- the model
def binom_pmf(n, p, upto=61):
    return [math.comb(n, e) * p ** e * (1 - p) ** (n - e) if e <= n else 0.0 for e in range(upto)]


def pois_pmf(lam, upto=61):
    return [math.exp(-lam) * lam ** e / math.factorial(e) for e in range(upto)]


BIN = binom_pmf(21, 0.35)
POIS = pois_pmf(7.1)


def canon(kinds):
    """the deck's chain kinds in a canonical order, and each card's position in it"""
    order = sorted(range(len(kinds)), key=lambda i: kinds[i])
    return tuple(kinds[i] for i in order), {ci: j for j, ci in enumerate(order)}


_paths = {}


def paths(ck, rule='slots'):
    """Every (state, events taken, absorbed) the chain process passes through, with its probability, summed over
    stop-risk outcomes, as arrays (S: links fired per card, L, A, P). A state at level l is final with P(N = l), or with
    P(N >= l) when nothing can advance (absorbed). rule: 'slots' (weight = links left) or 'cards' (uniform)."""
    key = (ck, rule)
    if key in _paths: return _paths[key]
    n = len(ck); acc = collections.defaultdict(float)
    alts = []
    for links, stop, risk in ck:
        v, go = [], 1.0
        for l, k, m in sorted(risk): v.append((l, go * k / m)); go *= 1 - k / m
        v.append((stop, go)); alts.append(v)
    for combo in itertools.product(*alts):
        w = math.prod(p for _, p in combo); stops = [s for s, _ in combo]
        if w == 0: continue
        layer = {tuple([0] * n): 1.0}; lvl = 0
        while layer:
            nxt = collections.defaultdict(float)
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


def final_w(ck, pmf, rule='slots'):
    """each path entry's probability of being the final state under the event-count pmf"""
    S, L, A, P = paths(ck, rule); pm = np.asarray(pmf, float)
    tail = np.concatenate([[1.0], 1.0 - np.cumsum(pm)])
    ok = L < len(pm); Lc = np.minimum(L, len(pm) - 1)
    return S, np.where(ok, P * np.where(A, tail[Lc], pm[Lc]), 0.0)


_reach = {}


def reach(kinds, pmf=BIN, rule='slots'):
    """{(card index, link): P(the card reaches the link)} for a deck's chain kinds"""
    ck, pos = canon(tuple(kinds))
    key = (ck, rule, 'B' if pmf is BIN else 'P' if pmf is POIS else tuple(pmf))
    if key not in _reach:
        S, w = final_w(ck, pmf, rule)
        _reach[key] = {(j, k): float(w[S[:, j] >= k].sum()) for j in range(len(ck)) for k in range(1, ck[j][0] + 1)}
    r = _reach[key]
    return {(ci, k): r[(j, k)] for ci, j in pos.items() for k in range(1, kinds[ci][0] + 1)}


def run_levels(kinds, obs, eps=EPS, rule='slots'):
    """the run's observations weighted over the chain process, by events taken: (A, B) arrays over levels 0..40,
    A for states that end there with N = l, B for absorbed states (N >= l)"""
    ck, pos = canon(tuple(kinds)); S, L, A, P = paths(ck, rule); w = P.copy()
    for ci, lk, seen, q in obs:
        fired = S[:, pos[ci]] >= lk
        w = w * np.where(fired, q * (1 - eps) if seen else 1 - q * (1 - eps), 0.0 if seen else 1.0)
    a = np.bincount(L[~A], w[~A], minlength=41)[:41]; b = np.bincount(L[A], w[A], minlength=41)[:41]
    return a, b


def lump(v):
    v = list(v[:KMAX]); v += [0.0] * (KMAX - len(v)); return v + [max(0.0, 1 - sum(v))]


def lk_matrix(xs):
    """per run, P(observations | N = k) for k = 0..KMAX-1 and N >= KMAX (the distribution-free fit's design)"""
    out = []
    for kinds, obs in xs:
        a, b = run_levels(kinds, obs)
        row = [a[k] + b[:k + 1].sum() for k in range(KMAX)] + [b.sum() + a[KMAX:].sum()]
        out.append(row)
    return np.array(out)


def em(M, iters=500):
    pi = np.full(KMAX + 1, 1 / (KMAX + 1))
    for _ in range(iters):
        w = pi * M; pi = (w / w.sum(1, keepdims=True)).mean(0)
    return pi


def ll(pi, M): return float(np.log(np.maximum(1e-300, M @ np.asarray(pi))).sum())


def mean_var(v):
    k = np.arange(len(v)); m = float((k * v).sum()); return m, float((k * k * v).sum()) - m * m


# ---------------------------------------------------------------- markers (GameTora card events)
_m = {}


def game():
    """skill names, chain link of each story, decoded SR/SSR card events and the per-card skill pools"""
    if _m: return _m
    gt = common.gametora(); db = common.mdb(); cards = common.cards()
    sk = dict(db.execute("select [index],text from text_data where category=47"))
    link_of = dict(db.execute("select story_id,show_progress_1 from single_mode_story_data where show_progress_1>0"))

    def names(hs):
        out = set()
        for kind, sid, _ in hs:
            for i in ([sid] if kind == 'hint' else sid):
                if i in sk: out.add(sk[i])
        return out
    evs = collections.defaultdict(list)     # card -> [(link or None, [choice -> [(outcome skills, ends chain)]])]
    for f in ('training_events_sr', 'training_events_ssr'):
        for cid, es in gt.load(f):
            for e in es:
                chs = [[(names(h), end) for h, end in gt.outcomes(ch[1])] for ch in gt.choices(e[1])]
                evs[cid].append((link_of.get(int(e[2])), chs))
    # every skill each card's events can give: SR/SSR events, pal / group events, card_events.json
    pool = collections.defaultdict(set)
    for cid, es in evs.items():
        for _, chs in es: pool[cid] |= {s for ch in chs for o, _ in ch for s in o}

    def walk(x, acc):
        if isinstance(x, list):
            if len(x) == 2 and isinstance(x[1], list) and x[1] and all(isinstance(k, int) for k in x[1]):
                acc |= names(gt.hints(x[1]))
            for y in x: walk(y, acc)
    for kindf in ('friend', 'group'):
        for row in gt.load(f'training_events_{kindf}'):
            acc = set(); walk(row[1:], acc)
            for cid, c in cards.items():
                if c['pal'] and ((kindf == 'friend' and c['chara'] == row[0]) or (kindf == 'group' and cid == row[0])): pool[cid] |= acc
    cev = common.gjson('card_events.json')['cards']
    for cid, e in cev.items():
        pool[int(cid)] |= set(e.get('always', [])) | {n for n, _ in e.get('disabled', [])} | {n for n, _ in e.get('chance', [])}
    # v1 markers: link -> (markers on the default choice, q, skills on other choices)
    v1 = {}
    for cid, es in evs.items():
        if cid not in cards: continue
        per = collections.defaultdict(list); nonchain = set(); cnt = collections.Counter()
        for lk, chs in es:
            sks = {s for ch in chs for o, _ in ch for s in o}
            if not lk: nonchain |= sks; continue
            per[lk].append(chs); cnt.update(sks)
        out = {}
        for lk, lst in per.items():
            if len(lst) != 1 or not lst[0]: continue
            first = lst[0][0]
            if any(end for _, end in first): continue          # the default choice can end the chain here
            cand = {s for o, _ in first for s in o if s not in nonchain and cnt[s] == 1}
            if not cand: continue
            out[lk] = (cand, sum(1 for o, _ in first if o & cand) / len(first), {s for ch in lst[0][1:] for o, _ in ch for s in o})
        v1[cid] = out
    # v2 markers: link -> skills unique to the link's single event, when every choice and outcome carries one
    v2 = {}
    for cid, es in evs.items():
        per = collections.defaultdict(list); cnt = collections.Counter()
        for lk, chs in es:
            cnt.update({s for ch in chs for o, _ in ch for s in o})
            if lk: per[lk].append(chs)
        out = {}
        for lk, lst in per.items():
            if len(lst) != 1 or not lst[0]: continue
            uniq = {s for ch in lst[0] for o, _ in ch for s in o if cnt[s] == 1}
            if uniq and all(o & uniq for ch in lst[0] for o, _ in ch): out[lk] = uniq
        v2[cid] = out
    _m.update(cards=cards, sk=sk, pool=pool, v1=v1, v2=v2, cev=cev, sknames=set(sk.values()))
    return _m


def kind(cid):
    """(links, links, stop risk) for a chain card, None for a pal, group or chainless card"""
    g = game(); c = g['cards'][cid]
    if c['pal'] or c['links'] == 0: return None
    risk = tuple(sorted(tuple(r) for r in g['cev'].get(str(cid), {}).get('stop_risk', []) if r[0] < c['links'] and r[0] <= 4))
    return (c['links'], c['links'], risk)


def pals(r):
    cards = game()['cards']
    return sum(1 for c, _, _ in r['deck'] if c in cards and cards[c]['pal'])


def obs_v1(r):
    """the first fit's observations of a run: (chain kinds, chain card ids, [(card index, link, seen, q)])"""
    g = game(); cards = g['cards']; deck = [c for c, _, _ in r['deck']]
    hints = {h['name']: set(h['src']) for h in r['hints']}
    pr = None if r['priority'] is None else {s for s in r['priority'] if s in g['sknames']}
    chain = [c for c in deck if c in cards and kind(c)]; obs = []
    for ci, cid in enumerate(chain):
        others = set().union(*(g['pool'][x] for x in deck if x != cid and x in cards))
        nm = cards[cid]['name'].split('] ', 1)[1]
        for lk, (mk0, q, other) in g['v1'].get(cid, {}).items():
            if other and (pr is None or (pr & other)): continue      # IT may have taken another choice
            mk = {s for s in mk0 if s not in others}
            if mk != mk0: continue                                    # another deck card could give a marker
            seen = any(any(x.endswith(' event') and nm in x for x in hints.get(s, ())) for s in mk)
            obs.append((ci, lk, seen, q))
    return [kind(c) for c in chain], chain, obs


def obs_v2(r):
    """the corrected observations of a run, or None for a deck with an unknown or non-pal R card"""
    g = game(); cards = g['cards']; deck = [c for c, _, _ in r['deck']]
    if any(c not in cards for c in deck): return None
    if any(cards[c]['rarity'] == 1 and not cards[c]['pal'] for c in deck): return None
    hints = {h['name']: h['src'] for h in r['hints']}
    name = {c: common.bare(n) for c, _, n in r['deck']}
    chain = [c for c in deck if kind(c)]; obs = []
    for ci, cid in enumerate(chain):
        nm = name[cid]
        for lk, mk in g['v2'].get(cid, {}).items():
            if lk > cards[cid]['links']: continue
            seen = any(any(x.endswith(' event') and nm in [y.strip() for y in x[:-6].split(' or ')] for x in hints.get(s, ())) for s in mk)
            if not seen and any(s in hints for s in mk) and all(hints[s] == ['Inspiration'] for s in mk if s in hints): continue
            obs.append((ci, lk, seen, 1.0))
    return [kind(c) for c in chain], chain, obs


# ---------------------------------------------------------------- data sets
_obs = {}


def dataset(version, upto, groups):
    """[(run, kinds, chain card ids, observations)] for the first `upto` records with a pal count in `groups`"""
    rs = common.runs(); fn = obs_v1 if version == 1 else obs_v2; out = []
    for r in rs:
        if r['seq'] >= upto or pals(r) not in groups: continue
        key = (version, r['id'])
        if key not in _obs: _obs[key] = fn(r)
        if _obs[key] is not None: out.append((r,) + _obs[key])
    return out


def binom_mle(xs):
    """the first fit's Binomial(n, p) grid MLE (n 6..24, p 0.20..0.99): (loglik, n, p)"""
    ab = [run_levels(k, o) for _, k, _, o in xs]
    A = np.array([a for a, _ in ab]); B = np.array([b for _, b in ab])
    grid = [(n, k / 200) for n in range(6, 25) for k in range(40, 200, 2)]
    PM = np.array([binom_pmf(n, p, 41) for n, p in grid]); TL = np.hstack([np.ones((len(grid), 1)), 1 - np.cumsum(PM, 1)[:, :40]])
    V = A @ PM.T + B @ TL.T
    lls = np.where((V <= 0).any(0), -1e18, np.log(np.maximum(V, 1e-300)).sum(0))
    return max((float(v), n, p) for v, (n, p) in zip(lls, grid))


class Approx(common.Claim):
    """a claim worded loosely ("about a third"): match when within `tol` of the worded value `target`"""
    def __init__(self, name, stated, got, target, tol, note=''):
        super().__init__(S, name, stated, got, status='match' if abs(got - target) <= tol else 'differs', note=note)

    def verdict(self): return self.status

    def shown(self): return f'{self.got * 100:.0f}%'


approx = Approx


# ---------------------------------------------------------------- claims
SHORT = [('Special Week [Take Hold of Our Dreams!]', 'Take Hold of Our Dreams!', '0.65', '0.82'),
         ('Yaeno Muteki [Fiery Discipline]', 'Fiery Discipline', '0.78', '0.90'),
         ('Zenno Rob Roy ["Rhodonite and the Dreamstone"]', 'Rhodonite and the Dreamstone', '0.63', '0.75'),
         ('Hishi Akebono [Who Wants the First Bite?]', 'Who Wants the First Bite?', '0.86', '0.93'),
         ('Smart Falcon [My Umadol Way! ☆]', 'My Umadol Way!', '0.73', '0.81'),
         ('Silence Suzuka [Beyond This Shining Moment]', 'Beyond This Shining Moment', '0.56', '0.77'),
         ('King Halo [Tonight, We Waltz]', 'Tonight, We Waltz', '0.91', '0.92')]
ALLOC = [('two 3-link SSRs', [(3, 3, ())] * 2, {'slots': [['1.00', '0.97', '0.87']], 'cards': [['0.99', '0.96', '0.87']]}),
         ('3-link SSR + 2-link SR', [(3, 3, ()), (2, 2, ())], {'slots': [['1.00', '0.99', '0.93'], ['0.99', '0.95']],
                                                              'cards': [['1.00', '0.98', '0.93'], ['0.99', '0.96']]}),
         ('three 3-link SSRs', [(3, 3, ())] * 3, {'slots': [['0.98', '0.84', '0.54']], 'cards': [['0.96', '0.81', '0.58']]}),
         ('four 3-link SSRs', [(3, 3, ())] * 4, {'slots': [['0.93', '0.65', '0.25']], 'cards': [['0.88', '0.62', '0.33']]})]
FINAL = {5: ('120', '0.98', '0.98', '0.96'), 4: ('285', '0.88', '0.86', '0.79'), 3: ('391', '0.58', '0.53', '0.48'),
         2: ('212', '0.29', '0.30', '0.29'), 1: ('168', '0.19', '0.16', '0.17'), 0: ('123', '0.14', '0.15', '0.15')}
EARLY = {4: ('0.02', '0.02', '0.9'), 3: ('0.07', '0.02', '3.3'), 2: ('0.06', '0.02', '2.5'), 1: ('0.05', '0.03', '1.8'),
         0: ('0.14', '0.03', '4.3')}


def derived():
    C = common.Claim; out = []
    m, v = 21 * 0.35, 21 * 0.35 * 0.65
    out += [C(S, 'Binomial(21, 0.35): mean', '7.35', m), C(S, 'Binomial(21, 0.35): variance', '4.8', v, note=f'{v:.4f}')]
    S2, w = final_w(((3, 3, ()), (3, 3, ())), BIN)
    both = float(w[(S2 == 3).all(1)].sum())
    out.append(C(S, 'two 3-link cards: P(N >= 6) = P(both chains complete)', '0.80', both, note=f'P(N >= 6) {sum(BIN[6:]):.4f}'))
    for deck, kinds, rows in ALLOC:
        for rule, cards_ in rows.items():
            r = reach(kinds, BIN, rule)
            for ci, vals in enumerate(cards_):
                who = '' if len(cards_) == 1 else (' SSR' if ci == 0 else ' SR')
                for k, st in enumerate(vals, 1):
                    out.append(C(S, f'{deck}{who}, {rule} rule: P(reach link {k})', st, r[(ci, k)]))
    for rule, st in (('slots', '5%'), ('cards', '12.5%')):
        S3, w = final_w(((3, 3, ()), (3, 3, ())), [0, 0, 0, 1.0], rule)
        out.append(C(S, f'two 3-link cards, exactly 3 events, {rule} rule: P(final state (3, 0))', st, float(w[(S3[:, 0] == 3) & (S3[:, 1] == 0)].sum())))
    out.append(C(S, 'distribution-free fit: parameters more than the binomial', '14', KMAX - 2, note=f'P(N = 0..{KMAX - 1}), P(N >= {KMAX}): {KMAX} free'))
    return out


def fit_claims():
    """the first fit (v1 markers, first 879 records, 1-4 pals) and its per-scenario split"""
    C = common.Claim; xs = dataset(1, 879, {1, 2, 3, 4}); note = 'v1, first 879 records'
    M = lk_matrix([(k, o) for _, k, _, o in xs]); pooled = em(M); llp = ll(pooled, M)
    mn, vr = mean_var(pooled)
    l21 = ll(lump(BIN), M); l15 = ll(lump(binom_pmf(15, 0.48)), M); lp = ll(lump(POIS), M)
    out = [C(S, 'fit of N: runs', '1,299', len(xs), note=f'the fit table comes from the first 879 records (v1); 1,299 is the final-link table (v2)'),
           C(S, 'distribution-free: mean', '7.30', mn, note=note), C(S, 'distribution-free: variance', '5.1', vr, note=note),
           C(S, 'Binomial(21, 0.35): nats behind distribution-free', '6.6', llp - l21, tol=0.1, note=f'{note}; LL {l21:.1f} vs {llp:.1f}'),
           C(S, 'Binomial(15, 0.48): nats worse than Binomial(21, 0.35)', '3.5', l21 - l15, tol=0.1, note=note),
           C(S, 'Poisson(7.1): nats worse', '27', llp - lp, note=f'{note}; vs distribution-free (vs Binomial(21, 0.35): {l21 - lp:.1f})')]
    by = collections.defaultdict(list)
    for i, (r, *_rest) in enumerate(xs): by[r['scenario']].append(i)
    sep = {sc: em(M[ix]) for sc, ix in by.items() if len(ix) >= 30}
    lls = sum(ll(sep[sc], M[by[sc]]) for sc in sep) + sum(ll(pooled, M[ix]) for sc, ix in by.items() if sc not in sep)
    for sc, st in ((2, '7.56'), (3, '7.16'), (1, '6.53')):
        out.append(C(S, f'per scenario: mean, {common.SCEN[sc]}', st, mean_var(sep[sc])[0] if sc in sep else None,
                     note=f'{note}; {len(by[sc])} runs'))
    out.append(C(S, 'per scenario: log-likelihood gain', '15.7', lls - llp, tol=0.1, note=note))
    out.append(C(S, 'per scenario: extra parameters', '32', (len(sep) - 1) * KMAX, note=f'{len(sep)} scenarios with 30+ runs'))
    return out, dict(pooled=pooled, M=M, xs=xs)


def refit_claims():
    """the 1,792-run refit: v1 markers, first 2,425 records, Binomial MLE per pal count"""
    C = common.Claim; out = []; res = {}
    for npal in (1, 2, 3, 4): res[npal] = (len(dataset(1, 2425, {npal})), binom_mle(dataset(1, 2425, {npal})))
    out.append(C(S, 'refit: runs (1-4 pals)', '1,792', sum(n for n, _ in res.values()), note='v1, first 2,425 records'))
    for npal, st in zip((1, 2, 3, 4), ('7.52', '6.84', '7.20', '7.92')):
        n, (v, bn, bp) = res[npal]
        out.append(C(S, f'refit: mean, {npal} pals', st, bn * bp, note=f'{n} runs; Binomial MLE n {bn}, p {bp:.2f}'))
    vs = [bn * bp * (1 - bp) for _, (_, bn, bp) in res.values()]
    out += [C(S, 'refit: variance, lowest', '4.0', min(vs)), C(S, 'refit: variance, highest', '5.3', max(vs))]
    return out, res


def final_claims():
    """the final-link table, the early-link shortfall and the per-card list (v2 markers, first 1,300 records)"""
    C = common.Claim; out = []; note = 'v2, first 1,300 records'; tab = {}
    for npal in (5, 4, 3, 2, 1, 0):
        xs = dataset(2, 1300, {npal}); o = b = p = 0.0; n = 0
        for r, kinds, chain, obs in xs:
            mb, mp = reach(kinds, BIN), reach(kinds, POIS)
            for ci, lk, seen, q in obs:
                if q == 1 and lk == kinds[ci][0]: o += seen; b += mb[(ci, lk)]; p += mp[(ci, lk)]; n += 1
        st = FINAL[npal]; tab[npal] = (len(xs), o / n, b / n, p / n, n)
        out += [C(S, f'final-link table, {npal} pals: runs', st[0], len(xs), note=note),
                C(S, f'final-link table, {npal} pals: observed', st[1], o / n, note=f'{n} card-links'),
                C(S, f'final-link table, {npal} pals: Binomial(21, 0.35)', st[2], b / n),
                C(S, f'final-link table, {npal} pals: Poisson(7.1)', st[3], p / n)]
    out.append(C(S, 'final-link table: runs in all', '1,299', sum(t[0] for t in tab.values()), note='first 1,300 records, one deck with a non-pal R card dropped'))
    # binomial p refit on these runs (n = 21; per-observation likelihood, all links)
    xs = dataset(2, 1300, {0, 1, 2, 3, 4, 5}); res = []
    for p_ in (0.33, 0.35, 0.37, 0.39, 0.41, 0.43):
        pm = binom_pmf(21, p_); t = 0.0
        for r, kinds, chain, obs in xs:
            mb = reach(kinds, pm)
            for ci, lk, seen, q in obs:
                pr = min(max(mb[(ci, lk)], 1e-9), 1 - 1e-9); t += math.log(pr if seen else 1 - pr)
        res.append((t, p_))
    out.append(C(S, 'p refit on the 1,299 runs (n = 21)', '0.35', max(res)[1], note=f'{note}; grid 0.33-0.43 step 0.02'))
    # early links (1 and 2), per run as a cluster
    early = {}
    for npal in (4, 3, 2, 1, 0):
        per = []
        for r, kinds, chain, obs in dataset(2, 1300, {npal}):
            mb = reach(kinds, BIN); o = e = 0.0; k = 0
            for ci, lk, seen, q in obs:
                if q != 1 or lk == kinds[ci][0]: continue
                o += seen; e += mb[(ci, lk)]; k += 1
            if k: per.append((o, e, k))
        n = sum(x[2] for x in per); O = sum(x[0] for x in per); E = sum(x[1] for x in per)
        sd = math.sqrt(sum((x[0] - x[1]) ** 2 for x in per)); st = EARLY[npal]; early[npal] = (O / n, E / n, n, len(per))
        out += [C(S, f'early-link shortfall, {npal} pals', st[0], (E - O) / n, note=f'{n} links in {len(per)} runs; obs {O / n:.2f} pred {E / n:.2f}'),
                C(S, f'early-link shortfall, {npal} pals: +-', st[1], sd / n), C(S, f'early-link shortfall, {npal} pals: z', st[2], (E - O) / sd)]
    # per card, early links, 0-4 pals
    pc = collections.defaultdict(lambda: [0, 0.0, 0]); cards = game()['cards']
    for r, kinds, chain, obs in dataset(2, 1300, {0, 1, 2, 3, 4}):
        mb = reach(kinds, BIN)
        for ci, lk, seen, q in obs:
            if lk == kinds[ci][0]: continue
            a = pc[chain[ci]]; a[0] += seen; a[1] += mb[(ci, lk)]; a[2] += 1
    for label, title, so, sp in SHORT:
        cid = next(c for c in pc if title in cards[c]['name']); o, p, n = pc[cid]
        out += [C(S, f'early links {label}: observed', so, o / n, note=f'n {n}'), C(S, f'early links {label}: predicted', sp, p / n)]
        if title == 'Tonight, We Waltz': out.append(C(S, f'early links {label}: n', '125', n))
    return out, dict(final=tab, early=early, percard=pc)


def alloc_claims():
    """slot rule against card rule (v1 markers, first 1,193 records, 0-5 pals; per-observation likelihood)"""
    C = common.Claim; note = 'v1, first 1,193 records'; tot = collections.Counter(); fin = {}
    for npal in (5, 4, 3, 2, 1, 0):
        xs = dataset(1, 1193, {npal})
        for rule in ('slots', 'cards'):
            l_ = 0.0; o = p = 0.0; n = 0
            for r, kinds, chain, obs in xs:
                mb = reach(kinds, BIN, rule)
                for ci, lk, seen, q in obs:
                    if q != 1: continue
                    pr = min(max(mb[(ci, lk)], 1e-9), 1 - 1e-9); l_ += math.log(pr if seen else 1 - pr)
                    if lk == kinds[ci][0]: o += seen; p += pr; n += 1
            tot[rule] += l_; fin[(npal, rule)] = (o / n, p / n, n)
    out = [C(S, 'allocation: total LL, slot rule', '−2,397.2', tot['slots'], tol=0.1, note=note),
           C(S, 'allocation: total LL, card rule', '−2,396.7', tot['cards'], tol=0.1, note=note)]
    for npal, so, ss, sc in ((2, '0.27', '0.29', '0.36'), (0, '0.13', '0.15', '0.22')):
        out += [C(S, f'allocation: final links, {npal} pals, observed', so, fin[(npal, 'slots')][0], note=f'{fin[(npal, "slots")][2]} card-links'),
                C(S, f'allocation: final links, {npal} pals, slot rule', ss, fin[(npal, 'slots')][1]),
                C(S, f'allocation: final links, {npal} pals, card rule', sc, fin[(npal, 'cards')][1])]
    return out, dict(total=tot, final=fin)


def miss_claims():
    C = common.Claim; out = []
    # two-chain decks: both final links seen, clean two-SSR runs (v1, first 879 records)
    clean = []
    for r, kinds, chain, obs in dataset(1, 879, {1, 2, 3, 4}):
        if len(kinds) != 2 or any(k != (3, 3, ()) for k in kinds): continue
        f = {ci: seen for ci, lk, seen, q in obs if lk == 3 and q == 1}
        if len(f) == 2: clean.append(sum(f.values()))
    S2, w = final_w(((3, 3, ()), (3, 3, ())), lump(BIN))
    fin = (S2 == 3).sum(1); pred = float((w * (1 - EPS) ** fin * (fin == 2)).sum())
    k = sum(1 for c in clean if c == 2)
    out += [C(S, 'two-chain decks: both chains finish, observed', '82%', k / len(clean), note=f'{k}/{len(clean)} clean two-SSR runs (v1, first 879 records); Wilson {common.wilson(k, len(clean))[0]:.2f}-{common.wilson(k, len(clean))[1]:.2f}'),
            C(S, 'two-chain decks: both chains finish, predicted', '75%', pred, note='Binomial(21, 0.35), 3% unseen hint allowance')]
    # chains that never start in crowded decks: link-1 markers (v1, first 679 records, the distribution-free fit there)
    xs = dataset(1, 679, {1, 2, 3, 4}); pooled = em(lk_matrix([(k_, o) for _, k_, _, o in xs])); rows = collections.defaultdict(lambda: [0, 0.0, 0.0, 0])
    for r, kinds, chain, obs in xs:
        mf, mb = reach(kinds, list(pooled)), reach(kinds, BIN)
        for ci, lk, seen, q in obs:
            if lk == 1 and q == 1:
                a = rows[len(kinds)]; a[0] += seen; a[1] += (1 - EPS) * mf[(ci, 1)]; a[2] += (1 - EPS) * mb[(ci, 1)]; a[3] += 1
    o, pf, pb, n = rows[5]; o45 = [sum(rows[c][i] for c in (4, 5)) for i in range(4)]
    out += [approx('5 chain cards: chains never started (link-1 marker unseen), observed', 'about a third', 1 - o / n, 1 / 3, 0.05,
                   note=f'{n - o:.0f}/{n} (v1, first 679 records); 4-5 chain cards: {1 - o45[0] / o45[3]:.2f} of {o45[3]}'),
            approx('5 chain cards: chains never started, model', 'about 16%', 1 - pf / n, 0.16, 0.015,
                   note=f'distribution-free fit on those records, 3% unseen allowance; Binomial(21, 0.35): {1 - pb / n:.2f}; 4-5 chain cards: {1 - o45[1] / o45[3]:.2f}')]
    # activation parameter at 3 chain cards (v1, first 1,193 records)
    xs = dataset(1, 1193, {3})

    def stats(a):
        l_ = 0.0
        for r, kinds, chain, obs in xs:
            kk = tuple((l, s_, tuple(sorted(rk + ((0, 1 - a, 1),)))) for l, s_, rk in kinds) if a < 1 else tuple(kinds)
            mb = reach(kk, BIN)
            for ci, lk, seen, q in obs:
                if q != 1: continue
                pr = min(max(mb[(ci, lk)], 1e-9), 1 - 1e-9); l_ += math.log(pr if seen else 1 - pr)
        return l_
    best = max((stats(a), a) for a in [i / 100 for i in range(60, 101, 2)])
    out.append(C(S, 'activation parameter a, three chain cards', '0.82', best[1], note=f'v1, first 1,193 records, 3 pals ({len(xs)} runs); gain {best[0] - stats(1.0):+.1f} nats'))
    return out


def claims():
    out = derived()
    if not common.have_runs():
        return out + common.need_data(S, [('distribution-free: mean', '7.30'), ('distribution-free: variance', '5.1'),
                                          ('Binomial(21, 0.35): nats behind distribution-free', '6.6'),
                                          ('Binomial(15, 0.48): nats worse than Binomial(21, 0.35)', '3.5'), ('Poisson(7.1): nats worse', '27'),
                                          ('per scenario: means', '7.56 / 7.16 / 6.53'), ('per scenario: log-likelihood gain', '15.7'),
                                          ('refit: runs', '1,792'), ('refit: means, 1 / 2 / 3 / 4 pals', '7.52 / 6.84 / 7.20 / 7.92'),
                                          ('final-link table: runs', '1,299'), ('allocation: total LL, slot / card rule', '−2,397.2 / −2,396.7'),
                                          ('two-chain decks: both finish, observed / predicted', '82% / 75%'),
                                          ('early-link shortfall and z by pals', 'see RESEARCH.md'), ('per-card early links', 'see RESEARCH.md'),
                                          ('chains never started (4-5 chain cards)', 'about a third'), ('activation parameter a', '0.82')])
    out += fit_claims()[0] + refit_claims()[0] + final_claims()[0] + alloc_claims()[0] + miss_claims()
    return out


def detail():
    """the fits' exact figures, and the same checks on the corrected measurement"""
    if not common.have_runs(): return
    _, f = fit_claims(); M = f['M']; pooled = f['pooled']; llp = ll(pooled, M)
    print(f'\nfirst fit (v1, first 879 records, {len(f["xs"])} runs): log-likelihood distribution-free {llp:.2f}, '
          + ', '.join(f'{nm} {ll(lump(v), M):.2f}' for nm, v in (('Binomial(21, 0.35)', BIN), ('Binomial(15, 0.48)', binom_pmf(15, 0.48)), ('Poisson(7.1)', POIS))))
    xs = dataset(2, 1300, {0, 1, 2, 3, 4, 5}); M2 = lk_matrix([(k, o) for _, k, _, o in xs]); p2 = em(M2); l2 = ll(p2, M2)
    print(f'the same fit on the corrected measurement (v2, first 1,300 records, {len(xs)} runs): distribution-free mean {mean_var(p2)[0]:.2f}, '
          f'variance {mean_var(p2)[1]:.2f}; behind it: ' + ', '.join(f'{nm} {l2 - ll(lump(v), M2):.1f}' for nm, v in (('Binomial(21, 0.35)', BIN), ('Binomial(15, 0.48)', binom_pmf(15, 0.48)), ('Poisson(7.1)', POIS))))
    for v, upto in ((1, 879), (1, 1193), (1, 2425), (2, 1300)):
        ds = dataset(v, upto, {0, 1, 2, 3, 4, 5}); cards = game()['cards']
        nr = sum(1 for r, *_ in ds if any(cards.get(c, {}).get('rarity') == 1 and not cards[c]['pal'] for c, _, _ in r['deck']))
        print(f'v{v}, first {upto:,} records: {len(ds)} runs, {nr} with a non-pal R card; by pals ' + str(dict(sorted(collections.Counter(pals(r) for r, *_ in ds).items()))))
    # two-chain decks and chains never started, on the corrected measurement
    clean = []
    for r, kinds, chain, obs in dataset(2, 1300, {4}):
        if len(kinds) != 2 or any(k != (3, 3, ()) for k in kinds): continue
        fz = {ci: seen for ci, lk, seen, q in obs if lk == 3}
        if len(fz) == 2: clean.append(sum(fz.values()) == 2)
    k = sum(clean); lo, hi = common.wilson(k, len(clean))
    print(f'two plain 3-link SSRs, 4 pals (v2, first 1,300 records): both finished {k}/{len(clean)} = {k / len(clean):.3f} ({lo:.3f}-{hi:.3f}); '
          f'model P(N >= 6) {sum(BIN[6:]):.3f}, with the 3% unseen allowance on each final link {sum(BIN[6:]) * 0.97 ** 2:.3f}')
    rows = collections.defaultdict(lambda: [0, 0.0, 0])
    for r, kinds, chain, obs in dataset(2, 1300, {0, 1, 2, 3, 4, 5}):
        mb = reach(kinds, BIN)
        for ci, lk, seen, q in obs:
            if lk == 1: a = rows[len(kinds)]; a[0] += seen; a[1] += mb[(ci, 1)]; a[2] += 1
    print('link 1 not reached, observed / Binomial(21, 0.35), by chain cards (v2, first 1,300 records): '
          + ', '.join(f'{c}: {1 - o / n:.2f} / {1 - p / n:.2f} (n {n})' for c, (o, p, n) in sorted(rows.items())))
    _, a = alloc_claims()
    print('allocation (v1, first 1,193 records), final links observed / slot rule / card rule: '
          + ', '.join(f'{npal} pals {a["final"][(npal, "slots")][0]:.2f} / {a["final"][(npal, "slots")][1]:.2f} / {a["final"][(npal, "cards")][1]:.2f}' for npal in (5, 4, 3, 2, 1, 0)))
    _, res = refit_claims()
    print('refit (v1, first 2,425 records): ' + ', '.join(f'{npal} pals {n} runs Binomial({bn}, {bp:.2f}) mean {bn * bp:.2f} variance {bn * bp * (1 - bp):.2f}' for npal, (n, (_, bn, bp)) in res.items()))


if __name__ == '__main__':
    common.main(claims)
    detail()
