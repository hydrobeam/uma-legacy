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
Each number is computed on the record prefix and measurement it came from (each value's note names them):
the fit of N, its per-scenario split, the final-link table, the two-chain decks, chains never started, the early-link
shortfall and the per-card list on v2, first 1,300 records; the first fit on v1, first 879 records; the refit on v1,
first 2,425 records (Binomial grid MLE per pal count); the allocation rules and the activation parameter on v1, first
1,193 records.
The model: N chain events per run; each advances one running non-pal card's chain by a link, the card picked with
weight = links left ("slots") or uniformly ("cards"); stop risk (card_events.json) ends a chain at link l with k/n.
Fits of N are exact over the chain states, with a 3% chance that a fired link's marker does not show (without it a run
whose later link shows while an earlier one does not has likelihood 0; allowance_claims measures the rate); the allocation
and activation comparisons score each observation on its own (P(card reaches link) on the run's deck)."""
import collections, itertools, math, os, re, sys
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

S = '7'
EPS = 0.03     # chance a fired link's marker hint does not show (the first fit's allowance)
KMAX = 16      # distribution-free fit: P(N = 0..15) and P(N >= 16)
MISC = {}      # numbers the findings use that the document does not state


# ---------------------------------------------------------------- the model
def binom_pmf(n, p, upto=61):
    return [math.comb(n, e) * p ** e * (1 - p) ** (n - e) if e <= n else 0.0 for e in range(upto)]


def pois_pmf(lam, upto=61):
    return [math.exp(-lam) * lam ** e / math.factorial(e) for e in range(upto)]


def chain_model():
    """the solver's event count, N ~ Binomial(CHAIN_N, CHAIN_P) (pkg/loopopt/src/deck/chains.rs)"""
    src = open(f'{common.ROOT}/pkg/loopopt/src/deck/chains.rs', encoding='utf-8').read()
    return int(re.search(r'CHAIN_N: u32 = (\d+);', src).group(1)), float(re.search(r'CHAIN_P: f64 = ([\d.]+);', src).group(1))


CN, CP = chain_model()
BIN = binom_pmf(CN, CP)
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
    by_name = {name[c]: c for c in deck}
    chain = [c for c in deck if kind(c)]; obs = []
    for ci, cid in enumerate(chain):
        nm = name[cid]
        for lk, mk in g['v2'].get(cid, {}).items():
            if lk > cards[cid]['links']: continue
            credits = [[y.strip() for y in x[:-6].split(' or ')] for s in mk for x in hints.get(s, ()) if x.endswith(' event')]
            credits = [c for c in credits if nm in c]
            # "A or B event": unknown when B is another deck card whose events can give the marker
            if credits and all(any(y != nm and y in by_name and mk & g['pool'][by_name[y]] for y in c) for c in credits): continue
            seen = bool(credits)
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


# ---------------------------------------------------------------- values
SHORT = [('Special Week [Take Hold of Our Dreams!]', 'Take Hold of Our Dreams!'), ('Yaeno Muteki [Fiery Discipline]', 'Fiery Discipline'),
         ('Zenno Rob Roy ["Rhodonite and the Dreamstone"]', 'Rhodonite and the Dreamstone'),
         ('Hishi Akebono [Who Wants the First Bite?]', 'Who Wants the First Bite?'), ('Smart Falcon [My Umadol Way! ☆]', 'My Umadol Way!'),
         ('Silence Suzuka [Beyond This Shining Moment]', 'Beyond This Shining Moment'), ('King Halo [Tonight, We Waltz]', 'Tonight, We Waltz')]
# example decks: chain kinds, and the cards shown (one of identical cards)
ALLOC = [('two 3-link SSRs', [(3, 3, ())] * 2, ('',)), ('3-link SSR + 2-link SR', [(3, 3, ()), (2, 2, ())], (' SSR', ' SR')),
         ('three 3-link SSRs', [(3, 3, ())] * 3, ('',)), ('four 3-link SSRs', [(3, 3, ())] * 4, ('',))]


def V(name, got, **kw):
    return common.Value(S, name, got, **kw)


def derived():
    out = []
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


def fit_claims():
    """the fit of N (v2 markers, first 1,300 records, 0-5 pals) and its per-scenario split; the first fit (v1, first 879)"""
    xs = dataset(2, 1300, {0, 1, 2, 3, 4, 5}); note = 'v2, first 1,300 records'
    M = lk_matrix([(k, o) for _, k, _, o in xs]); pooled = em(M); llp = ll(pooled, M)
    mn, vr = mean_var(pooled)
    l21 = ll(lump(BIN), M); l15 = ll(lump(binom_pmf(15, 0.48)), M); lp = ll(lump(POIS), M)
    _, bn, bp = binom_mle(xs)
    out = [V('fit of N: runs', len(xs), note=note),
           V('distribution-free: mean', mn, note=note), V('distribution-free: variance', vr, note=note),
           V('Binomial(21, 0.35): nats behind distribution-free', llp - l21, note=f'LL {l21:.1f} vs {llp:.1f}'),
           V('Binomial(15, 0.48): nats behind distribution-free', llp - l15),
           V('Poisson(7.1): nats behind distribution-free', llp - lp),
           V('best binomial, n free: nats better than (21, 0.35)', ll(lump(binom_pmf(bn, bp)), M) - l21)]
    by = collections.defaultdict(list)
    for i, (r, *_rest) in enumerate(xs): by[r['scenario']].append(i)
    sep = {sc: em(M[ix]) for sc, ix in by.items() if len(ix) >= 30}
    lls = sum(ll(sep[sc], M[by[sc]]) for sc in sep) + sum(ll(pooled, M[ix]) for sc, ix in by.items() if sc not in sep)
    for sc in (2, 3, 1):
        out.append(V(f'per scenario: mean, {common.SCEN[sc]}', mean_var(sep[sc])[0] if sc in sep else None,
                     note=f'{note}; {len(by[sc])} runs'))
    out.append(V('per scenario: log-likelihood gain', lls - llp, note=note))
    out.append(V('per scenario: extra parameters', (len(sep) - 1) * KMAX, note=f'{len(sep)} scenarios with 30+ runs'))
    # the first fit: default-choice markers (v1), first 879 records, 1-4 pals; the binomial it chose
    x1 = dataset(1, 879, {1, 2, 3, 4}); _, n1, p1 = binom_mle(x1)
    out += [V('first fit: runs', len(x1), note='v1, first 879 records'), V('first fit: binomial n', n1), V('first fit: binomial p', p1)]
    return out, dict(pooled=pooled, M=M, xs=xs)


def refit_claims():
    """the 1,792-run refit: v1 markers, first 2,425 records, Binomial MLE per pal count"""
    out = []; res = {}
    for npal in (1, 2, 3, 4): res[npal] = (len(dataset(1, 2425, {npal})), binom_mle(dataset(1, 2425, {npal})))
    out.append(V('refit: runs (1-4 pals)', sum(n for n, _ in res.values()), note='v1, first 2,425 records'))
    for npal in (1, 2, 3, 4):
        n, (v, bn, bp) = res[npal]
        out.append(V(f'refit: mean, {npal} pals', bn * bp, note=f'{n} runs; Binomial MLE n {bn}, p {bp:.2f}'))
    vs = [bn * bp * (1 - bp) for _, (_, bn, bp) in res.values()]
    out += [V('refit: variance, lowest', min(vs)), V('refit: variance, highest', max(vs))]
    return out, res


def final_claims():
    """the final-link table, the early-link shortfall and the per-card list (v2 markers, first 1,300 records)"""
    out = []; note = 'v2, first 1,300 records'; tab = {}
    for npal in (5, 4, 3, 2, 1, 0):
        xs = dataset(2, 1300, {npal}); o = b = p = 0.0; n = 0
        for r, kinds, chain, obs in xs:
            mb, mp = reach(kinds, BIN), reach(kinds, POIS)
            for ci, lk, seen, q in obs:
                if q == 1 and lk == kinds[ci][0]: o += seen; b += mb[(ci, lk)]; p += mp[(ci, lk)]; n += 1
        tab[npal] = (len(xs), o / n, b / n, p / n, n)
        out += [V(f'final-link table, {npal} pals: runs', len(xs), note=note),
                V(f'final-link table, {npal} pals: observed', o / n, note=f'{n} card-links'),
                V(f'final-link table, {npal} pals: Binomial(21, 0.35)', b / n),
                V(f'final-link table, {npal} pals: Poisson(7.1)', p / n)]
    out.append(V('final-link table: runs in all', sum(t[0] for t in tab.values()), note='first 1,300 records, one deck with a non-pal R card dropped'))
    # binomial p refit on these runs (n = CHAIN_N; per-observation likelihood, all links)
    xs = dataset(2, 1300, {0, 1, 2, 3, 4, 5}); res = []
    for p_ in (0.33, 0.35, 0.37, 0.39, 0.41, 0.43):
        pm = binom_pmf(CN, p_); t = 0.0
        for r, kinds, chain, obs in xs:
            mb = reach(kinds, pm)
            for ci, lk, seen, q in obs:
                pr = min(max(mb[(ci, lk)], 1e-9), 1 - 1e-9); t += math.log(pr if seen else 1 - pr)
        res.append((t, p_))
    out.append(V('p refit on the 1,299 runs (n = 21)', max(res)[1], note=f'{note}; grid 0.33-0.43 step 0.02'))
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
        sd = math.sqrt(sum((x[0] - x[1]) ** 2 for x in per)); early[npal] = (O / n, E / n, n, len(per))
        out += [V(f'early-link shortfall, {npal} pals', (E - O) / n, note=f'{n} links in {len(per)} runs; obs {O / n:.2f} pred {E / n:.2f}'),
                V(f'early-link shortfall, {npal} pals: +-', sd / n), V(f'early-link shortfall, {npal} pals: z', (E - O) / sd)]
    # per card, early links, 0-4 pals
    pc = collections.defaultdict(lambda: [0, 0.0, 0]); cards = game()['cards']
    for r, kinds, chain, obs in dataset(2, 1300, {0, 1, 2, 3, 4}):
        mb = reach(kinds, BIN)
        for ci, lk, seen, q in obs:
            if lk == kinds[ci][0]: continue
            a = pc[chain[ci]]; a[0] += seen; a[1] += mb[(ci, lk)]; a[2] += 1
    for label, title in SHORT:
        cid = next(c for c in pc if title in cards[c]['name']); o, p, n = pc[cid]
        out += [V(f'early links {label}: observed', o / n, note=f'n {n}'), V(f'early links {label}: predicted', p / n)]
        if title == 'Tonight, We Waltz': out.append(V(f'early links {label}: n', n))
    return out, dict(final=tab, early=early, percard=pc)


def allowance_claims():
    """the marker allowance: links fire in order, so a card's later link seen means its earlier links fired; the share of
    those earlier links whose marker is missing measures how often a fired link goes unseen (v2, first 1,300 records)"""
    note = 'v2, first 1,300 records'; imp = un = 0
    for r, kinds, chain, obs in dataset(2, 1300, {0, 1, 2, 3, 4, 5}):
        per = collections.defaultdict(dict)
        for ci, lk, seen, q in obs: per[ci][lk] = seen
        for d in per.values():
            for lk, seen in d.items():
                if any(l2 > lk and s2 for l2, s2 in d.items()): imp += 1; un += not seen
    lo, hi = common.wilson(un, imp)
    return [V('allowance: earlier links implied fired', imp, note=note), V('allowance: marker not shown', un),
            V('allowance: share', un / imp), V('allowance: 95% low', lo), V('allowance: 95% high', hi)]


def final_by_card():
    """final-link reach by card (v2 markers, every research run) against the chain model on each run's deck: the pooled
    rate and the spread between cards with 60+ observations (sum of squared z against chi2 with one df per card)"""
    from scipy.stats import chi2
    per = collections.defaultdict(lambda: [0, 0.0, 0, 0.0])
    for r, kinds, chain, obs in dataset(2, 10 ** 9, {0, 1, 2, 3, 4, 5}):
        mb = reach(kinds)
        for ci, lk, seen, q in obs:
            if lk != kinds[ci][0]: continue
            a = per[chain[ci]]; p_ = mb[(ci, lk)]; a[0] += seen; a[1] += p_; a[2] += 1; a[3] += p_ * (1 - p_)
    k = sum(v[0] for v in per.values()); e = sum(v[1] for v in per.values()); n = sum(v[2] for v in per.values())
    zs = {c: (v[0] - v[1]) / math.sqrt(v[3]) for c, v in per.items() if v[2] >= 60}
    x2 = sum(z * z for z in zs.values()); names = game()['cards']
    return dict(obs=k / n, model=e / n, n=n, cards=len(zs), chi2=x2, p=chi2.sf(x2, len(zs)),
                low=sorted(zs.items(), key=lambda t: t[1])[:3], high=sorted(zs.items(), key=lambda t: -t[1])[:3],
                rate={c: (per[c][0] / per[c][2], per[c][1] / per[c][2]) for c in zs}, name=lambda c: names[c]['name'])


def allowance_sens():
    """the best p at n = CHAIN_N under several allowances, and with none on the runs showing no contradiction"""
    xs = dataset(2, 1300, {0, 1, 2, 3, 4, 5}); out = {}

    def best_p(xs, e):
        ab = [run_levels(k, o, eps=e) for _, k, _, o in xs]
        A = np.array([a for a, _ in ab]); B = np.array([b for _, b in ab]); res = []
        for p_ in [i / 1000 for i in range(300, 401, 5)]:
            pm = np.array(binom_pmf(CN, p_, 41)); v = A @ pm + B @ np.concatenate([[1.0], 1 - np.cumsum(pm)[:40]])
            if (v > 0).all(): res.append((float(np.log(v).sum()), p_))
        return max(res)[1]
    for e in (0.01, 0.02, 0.03, 0.045): out[e] = best_p(xs, e)

    def contra(o):
        per = collections.defaultdict(dict)
        for ci, lk, seen, q in o: per[ci][lk] = seen
        return any(not sn and any(l2 > lk and s2 for l2, s2 in d.items()) for d in per.values() for lk, sn in d.items())
    clean = [x for x in xs if not contra(x[3])]
    out[0.0] = best_p(clean, 0.0); out['clean runs'] = len(clean)
    return out


def alloc_claims():
    """slot rule against card rule (v1 markers, first 1,193 records, 0-5 pals; per-observation likelihood)"""
    note = 'v1, first 1,193 records'; tot = collections.Counter(); fin = {}
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
    out = [V('allocation: total LL, slot rule', tot['slots'], note=note), V('allocation: total LL, card rule', tot['cards'], note=note)]
    for npal in (2, 0):
        out += [V(f'allocation: final links, {npal} pals, observed', fin[(npal, 'slots')][0], note=f'{fin[(npal, "slots")][2]} card-links'),
                V(f'allocation: final links, {npal} pals, slot rule', fin[(npal, 'slots')][1]),
                V(f'allocation: final links, {npal} pals, card rule', fin[(npal, 'cards')][1])]
    return out, dict(total=tot, final=fin)


def miss_claims():
    out = []
    # two-chain decks: two plain 3-link SSRs with four pals, both final links observable (v2, first 1,300 records)
    clean = []
    for r, kinds, chain, obs in dataset(2, 1300, {4}):
        if len(kinds) != 2 or any(k != (3, 3, ()) for k in kinds): continue
        fz = {ci: seen for ci, lk, seen, q in obs if lk == 3}
        if len(fz) == 2: clean.append(sum(fz.values()) == 2)
    k = sum(clean); n = len(clean); lo, hi = common.wilson(k, n)
    out += [V('two-chain decks: both chains finish', k, note='v2, first 1,300 records'), V('two-chain decks: runs', n),
            V('two-chain decks: share', k / n), V('two-chain decks: 95% low', lo), V('two-chain decks: 95% high', hi),
            V('two-chain decks: predicted, P(N >= 6)', sum(BIN[6:]))]
    # chains never started: link 1 not reached, by chain cards in the deck (v2, first 1,300 records)
    rows = collections.defaultdict(lambda: [0, 0.0, 0])
    for r, kinds, chain, obs in dataset(2, 1300, {0, 1, 2, 3, 4, 5}):
        mb = reach(kinds, BIN)
        for ci, lk, seen, q in obs:
            if lk == 1: a = rows[len(kinds)]; a[0] += seen; a[1] += mb[(ci, 1)]; a[2] += 1
    for nc in (5, 6):
        o, p_, n = rows[nc]
        MISC[('never', nc)] = (n - o, n, 1 - p_ / n)
        out += [V(f'never started, {nc} chain cards: observed', 1 - o / n, note=f'link 1 not reached, {n} card-links'),
                V(f'never started, {nc} chain cards: model', 1 - p_ / n)]
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
    out.append(V('activation parameter a, three chain cards', best[1], note=f'v1, first 1,193 records, 3 pals ({len(xs)} runs); gain {best[0] - stats(1.0):+.1f} nats'))
    # the same on the corrected reading, fitted jointly with N ~ Binomial(CHAIN_N, p), exact over the chain process
    xs = dataset(2, 1300, {0, 1, 2, 3, 4, 5})

    def LL(p_, a):
        pm = np.array(binom_pmf(CN, p_, 41)); tl = np.concatenate([[1.0], 1 - np.cumsum(pm)[:40]]); t = 0.0
        for _, k, _, o in xs:
            kk = tuple((l, s_, tuple(sorted(rk + ((0, round(1 - a, 6), 1),)))) for l, s_, rk in k) if a < 1 else tuple(k)
            A, B = run_levels(kk, o); t += math.log(max(float(A @ pm + B @ tl), 1e-300))
        return t
    l1 = max(LL(p_, 1.0) for p_ in (0.33, 0.34, 0.35, 0.36, 0.37))
    lj, pj, aj = max((LL(p_, a), p_, a) for a in (0.97, 0.94, 0.90, 0.86) for p_ in (0.35, 0.37, 0.39, 0.41))
    out += [V('activation, corrected reading, joint with N: gain, nats', lj - l1, note=f'v2, first 1,300 records; p {pj}'),
            V('activation, corrected reading, joint with N: a', aj)]
    return out


def claims():
    out = derived()
    if not common.have_runs(): return out
    fc, f = fit_claims(); rc, res = refit_claims(); flc, fl = final_claims(); ac, al = alloc_claims()
    out += fc + allowance_claims() + rc + flc + ac + miss_claims()
    sens = allowance_sens()
    fb = final_by_card()
    out += [V('final links by card: pooled observed', fb['obs'], note=f"{fb['n']:,} card-links, every research run"),
            V('final links by card: pooled model', fb['model']), V('final links by card: cards', fb['cards']), V('final links by card: chi2', fb['chi2'])]
    MISC['final_by_card'] = fb
    out += [V('allowance 0: runs without a contradiction', sens['clean runs']),
            V('allowance sensitivity: lowest best p at n = 21', min(v for k, v in sens.items() if k != 'clean runs')),
            V('allowance sensitivity: highest best p at n = 21', max(v for k, v in sens.items() if k != 'clean runs'))]
    return out + findings(out, f, res, fl, al, sens)


def findings(out, f, res, fl, al, sens):
    """the section's conclusions, each tested on the numbers above"""
    from scipy.stats import chi2
    g = {c.name: c.got for c in out if isinstance(c, common.Value)}; F = lambda *a: common.finding(S, *a)
    gap = g['Binomial(21, 0.35): nats behind distribution-free']; df = g['distribution-free fit: parameters more than the binomial']; fs = []
    fs.append(F(f'N ~ Binomial({CN}, {CP}) explains chain progress as well as a free distribution of N: the {df} extra parameters '
                'are not significant (likelihood-ratio p > 0.05)', chi2.sf(2 * gap, df) > 0.05,
                f'{gap:.1f} nats behind distribution-free, LR p = {chi2.sf(2 * gap, df):.2f}'))
    fs.append(F('Poisson is rejected (variance = mean, too wide): it trails the binomial by more than 10 nats with as many parameters',
                g['Poisson(7.1): nats behind distribution-free'] - gap > 10,
                f'Poisson {g["Poisson(7.1): nats behind distribution-free"]:.1f} vs binomial {gap:.1f} nats behind'))
    fs.append(F(f'No binomial with n free does better than ({CN}, {CP}) (gain under 1.92 nats, the 1-parameter 5% bound)',
                g['best binomial, n free: nats better than (21, 0.35)'] < 1.92, f'{g["best binomial, n free: nats better than (21, 0.35)"]:.1f} nats'))
    pg = g['per scenario: log-likelihood gain']; dfs = g['per scenario: extra parameters']
    fs.append(F(f'One N for every scenario: per-scenario distributions gain nothing significant ({dfs} parameters)', chi2.sf(2 * pg, dfs) > 0.05,
                f'+{pg:.1f} nats, LR p = {chi2.sf(2 * pg, dfs):.2f}'))
    sh = g['allowance: share']; lo, hi = g['allowance: 95% low'], g['allowance: 95% high']
    fs.append(F(f'The {EPS} marker allowance is consistent with the measured rate of fired links whose marker is missing', lo <= EPS <= hi,
                f'{g["allowance: marker not shown"]:.0f} of {g["allowance: earlier links implied fired"]:.0f} = {sh:.3f} [{lo:.3f}, {hi:.3f}]'))
    ps = [v for k, v in sens.items() if k != 'clean runs']; lo_, hi_ = round(CP - 0.01, 2), round(CP + 0.01, 2)    # the adopted p +- 0.01
    fs.append(F(f'The fit of N does not depend on the allowance: the best p at n = {CN} stays within {lo_}-{hi_}', min(ps) >= lo_ and max(ps) <= hi_,
                ', '.join(f'eps {k}: {v:.3f}' for k, v in sens.items() if k != 'clean runs') + f' (eps 0.0 on the {sens["clean runs"]} runs without a contradiction)'))
    cover = {}
    for npal, (runs, o, b, p_, n) in fl['final'].items():
        l_, h_ = common.wilson(o * n, n); cover[npal] = (l_ <= b <= h_, o, b, l_, h_)
    fs.append(F('One N reproduces final-link completion at every pal count: the binomial prediction lies in each observed interval',
                all(c[0] for c in cover.values()), '; '.join(f'{k} pals {c[1]:.2f} [{c[3]:.2f}, {c[4]:.2f}] vs {c[2]:.2f}' for k, c in sorted(cover.items(), reverse=True))))
    means = {k: bn * bp for k, (n, (_, bn, bp)) in res.items()}
    fs.append(F('Pal events do not consume chain events: N fitted per pal count does not fall as pals rise', means[4] >= means[1],
                ', '.join(f'{k} pals {v:.2f}' for k, v in sorted(means.items()))))
    tl = al['total']; fin = al['final']
    fs.append(F('Slot and card allocation fit equally overall (within 2 nats)', abs(tl['slots'] - tl['cards']) < 2, f'slots {tl["slots"]:.1f}, cards {tl["cards"]:.1f}'))
    def inside(npal, rule):
        o, p_, n = fin[(npal, rule)]; l_, h_ = common.wilson(o * n, n); return l_ <= p_ <= h_, f'{npal} pals obs {o:.2f} [{l_:.2f}, {h_:.2f}], {rule} {p_:.2f}'
    ev = [inside(k, r) for k in (2, 0) for r in ('slots', 'cards')]
    fs.append(F('The slot rule is preferred: in crowded decks its final-link prediction lies in the observed interval and the card rule\'s does not',
                ev[0][0] and ev[2][0] and not ev[1][0] and not ev[3][0], '; '.join(e[1] for e in ev)))
    l_, h_ = g['two-chain decks: 95% low'], g['two-chain decks: 95% high']; pr = g['two-chain decks: predicted, P(N >= 6)']
    fs.append(F('Two-chain decks finish both chains as predicted (prediction within the observed interval)', l_ <= pr <= h_,
                f'{g["two-chain decks: share"]:.3f} [{l_:.3f}, {h_:.3f}] vs {pr:.3f}'))
    zs = {k: g[f'early-link shortfall, {k} pals: z'] for k in (4, 3, 2, 1, 0)}
    fs.append(F('Known miss: early links run short of the model in crowded decks (z > 2 at 3 and 0 pals)', zs[3] > 2 and zs[0] > 2,
                ', '.join(f'{k} pals z {v:.1f}' for k, v in zs.items())))
    nv = [(nc, *MISC[('never', nc)]) for nc in (5, 6)]; nv = [(nc, k / n, *common.wilson(k, n), m) for nc, k, n, m in nv]
    fs.append(F('Known miss: more chains never start than predicted with five or six chain cards (model below the observed interval)',
                all(lo > m for _, _, lo, _, m in nv), ', '.join(f'{nc} cards {o:.2f} [{lo:.2f}, {hi:.2f}] vs {m:.2f}' for nc, o, lo, hi, m in nv)))
    fb = MISC['final_by_card']
    fs.append(F('Final links fit when pooled, but cards differ beyond the model: pooled within 0.01, the spread between cards far '
                'beyond chance (p < 0.001)', abs(fb['obs'] - fb['model']) < 0.01 and fb['p'] < 0.001,
                f"pooled {fb['obs']:.3f} vs {fb['model']:.3f}; chi2 {fb['chi2']:.0f} on {fb['cards']} cards; lowest "
                + ', '.join(f"{fb['name'](c)} {fb['rate'][c][0]:.2f}/{fb['rate'][c][1]:.2f}" for c, _ in fb['low'])
                + '; highest ' + ', '.join(f"{fb['name'](c)} {fb['rate'][c][0]:.2f}/{fb['rate'][c][1]:.2f}" for c, _ in fb['high'])))
    ga = g['activation, corrected reading, joint with N: gain, nats']
    fs.append(F('A per-card activation parameter is rejected: on the corrected reading, fitted jointly with N, it gains under 1.92 nats',
                ga < 1.92, f'+{ga:.1f} nats at a = {g["activation, corrected reading, joint with N: a"]:.2f} (on the first reading, three chain cards '
                f'only: a = {g["activation parameter a, three chain cards"]:.2f})'))
    return fs


def detail():
    """the fits' exact figures, and the same checks on the corrected measurement"""
    if not common.have_runs(): return
    _, f = fit_claims(); M = f['M']; pooled = f['pooled']; llp = ll(pooled, M)
    print(f'\nfit of N (v2, first 1,300 records, {len(f["xs"])} runs): log-likelihood distribution-free {llp:.2f}, '
          + ', '.join(f'{nm} {ll(lump(v), M):.2f}' for nm, v in (('Binomial(21, 0.35)', BIN), ('Binomial(15, 0.48)', binom_pmf(15, 0.48)), ('Poisson(7.1)', POIS))))
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
          f'model P(N >= 6) {sum(BIN[6:]):.3f}, with the 3% unseen allowance on each final link {sum(BIN[6:]) * (1 - EPS) ** 2:.3f}')
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
