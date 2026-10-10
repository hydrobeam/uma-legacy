"""Independent check of the solver's exact landing odds (`joint_trainee` in deck.rs).

For every deck result of a validation run, recompute P(all targets land) from the job, the card data and the
reported decks and loops, jointly over the chain process (Binomial(21, 0.35) events, each advancing a random remaining
chain slot, weight = links left, stop-risk mixture), over each event's outcomes and over one card's training draws
(P(no hint for a set A of its targets) by forward simulation of the draws, each uniform over the skills below Lv 5, a
skill leaving after ceil(5 / hint levels per draw) draws; floor/ceil(h) draws, h = base(scenario) x (1 + hf/100)), and compare with the
reported p_all. Written separately from the Rust code: final chain states by plain forward iteration, and the joint
expectation by inclusion-exclusion over per-target "no gold" / "no hint" requirements.

usage: uv run val/joint_check.py TAG        (val/TAG__*.out and val/TAGpu__*.out)"""
import functools, itertools, json, math, os, sys
V = os.path.dirname(os.path.abspath(__file__)); S = os.path.dirname(V)
GW, GG = 0.2 * 1.771561, 0.4 * 1.771561


def draws_miss(k, n, t, cap):
    """P(none of k given skills drawn in t draws over n, each uniform over the skills drawn fewer than cap times), by
    recursion over the other skills' draw counts (sorted tuple; written independently of the solver's histogram DP)"""
    @functools.lru_cache(None)
    def go(st, t):
        if t == 0: return 1.0
        av = [i for i, c in enumerate(st) if c < cap]
        if not av and not k: return 1.0
        tot = 0.0
        for i in av:
            c = list(st); c[i] += 1; tot += go(tuple(sorted(c)), t - 1)
        return tot / (len(av) + k)
    return go(tuple([0] * (n - k)), t)


def hint_miss(k, n, h, hinc):
    cap = -(-5 // max(1, hinc)); t0 = math.floor(h); f = h - t0
    return (1 - f) * draws_miss(k, n, t0, cap) + (f * draws_miss(k, n, t0 + 1, cap) if f else 0.0)


def check(out, job, d):
    rd = lambda f: json.load(open(os.path.join(d, f)))
    look = {}
    for g, w, gd in rd(job['skills_file'])['skills']:
        look[w] = (g, 1)
        if gd: look[gd] = (g, 2)
    cards = {c['id']: c for c in rd(job['cards_file'])['cards']}
    db = rd(job.get('events_file', 'card_events.json'))['cards']
    # the solver's model, restated: chain events ~ Binomial(n, p); training draws h = base(scenario) x (1 + hf/100)
    # over the card's skill options (nskill)
    bn, bp = job.get('chain_events') or (21, 0.35)
    base = {'gc': 2.08, 'ura': 1.60, 'trackblazer': 2.30, 'grandmasters': 2.30}.get(job.get('scenario', 'unity'), 2.70)
    pmf = lambda e: math.comb(bn, e) * bp ** e * (1 - bp) ** (bn - e) if 0 <= e <= bn else 0.0
    unc = [t['group'] for t in job['targets'] if t.get('guaranteed', 0) != 2]
    nfix = len(job['targets']) - len(unc); K = len(unc)
    nm = lambda x: (x, 1.0) if isinstance(x, str) else (x[0], x[1])
    tk = lambda n: (unc.index(look[n][0]), look[n][1]) if n in look and look[n][0] in unc else None

    def choice_groups(cid):
        """per alts link: the choices with targets, as outcome groups [(p, [(k, lvl)])], dominated ones dropped"""
        res = {}
        for lk, chs in db.get(str(cid), {}).get('alts', []):
            gs = []
            for ch in chs:
                g = [(1 / len(ch), sorted({x for x in map(tk, o) if x})) for o in ch]
                gs.append([x for x in g if x[1]])
            def prof(g):
                p = [[0.0, 0.0] for _ in range(K)]
                for w, o in g:
                    for k in {k for k, _ in o}:
                        p[k][1] += w
                        if any(kk == k and l == 2 for kk, l in o): p[k][0] += w
                return p
            P = [prof(g) for g in gs]
            ge = lambda a, b: all(x[0] >= y[0] and x[1] >= y[1] for x, y in zip(a, b))
            keep = [g for i, g in enumerate(gs) if g and not any(j != i and gs[j] and ge(P[j], P[i]) and (P[j] != P[i] or (gs[j] == g and j < i)) for j in range(len(gs)))]
            res[lk] = keep
        return res

    def card_src(cid, end, label):
        c = cards[cid]; e = db.get(str(cid), {})
        tr = [unc.index(g) for g in c['whites'] if g in unc] if c['nskill'] else []
        fixed = []
        for n in e.get('always', []):
            if tk(n): fixed.append([(1.0, [tk(n)])])
        for n, p in e.get('chance', []):
            if tk(n): fixed.append([(p, [tk(n)])])
        for c_, g, lvl, p in job.get('odds_events', []):      # inline per-run events of the job
            if c_ == cid and g in unc: fixed.append([(p, [(unc.index(g), lvl)])])
        chain = {}; kind = None
        if not c['pal'] and c['links'] > 0:
            stop = end or c['links']; alts = choice_groups(cid)
            picks = dict(x.split(': ', 1) for x in label.split('; ')) if label else {}
            for lk, names in e.get('chain', []):
                if (end and lk >= stop) or lk in alts: continue
                for n, p in map(nm, names):
                    if tk(n): chain.setdefault(lk, []).append([(p, [tk(n)])])
            for lk, keep in alts.items():
                if end and lk >= stop or not keep: continue
                if len(keep) == 1: g = keep[0]
                else:
                    want = set(picks[f'link {lk}'].split(', '))
                    g = next(g for g in keep if {n for _, o in g for k, l in o for n in [nm_of(k, l)]} == want)
                chain.setdefault(lk, []).append(g)
            if end:
                for lk, names in e.get('end', []):
                    if lk != end: continue
                    for n, p in map(nm, names):
                        if tk(n): chain.setdefault(lk, []).append([(p, [tk(n)])])
            kind = (c['links'], stop, tuple(tuple(r) for r in e.get('stop_risk', []) if r[0] < stop and r[0] <= 4))
        return dict(train=(tr, base * (1 + c['hf'] / 100), c['nskill'], c.get('hinc', 1)), fixed=fixed, chain=chain, kind=kind)

    skills_db = rd(job['skills_file'])
    names = {g: (w, gd) for g, w, gd in skills_db['skills']}
    nm_of = lambda k, l: names[unc[k]][1] if l == 2 and names[unc[k]][1] else names[unc[k]][0]

    def grp_ok(groups, req):
        """P(no outcome of these independent groups breaks req); req[k]: 0 any, 1 no gold, 2 no hint"""
        pr = 1.0
        for g in groups:
            bad = sum(p for p, o in g if any(req[k] == 2 or (req[k] == 1 and l == 2) for k, l in o))
            pr *= 1 - bad
        return pr

    def chain_dist(kinds):
        n = len(kinds); res = {}
        alts = []
        for links, stop, risk in kinds:
            v = []; go = 1.0
            for l, k, nn in sorted(risk): q = k / nn; v.append((l, go * q)); go *= 1 - q
            v.append((stop, go)); alts.append(v)
        for combo in itertools.product(*alts):
            w = math.prod(p for _, p in combo); stops = [s for s, _ in combo]
            if w == 0: continue
            cur = {tuple([0] * n): 1.0}; e = 0
            while True:
                pe = pmf(e)
                nxt = {}
                for s, p in cur.items():
                    ws = [(kinds[i][0] - s[i]) if s[i] < stops[i] else 0 for i in range(n)]
                    if not sum(ws): res[s] = res.get(s, 0) + w * p * (1 - sum(pmf(x) for x in range(e))); continue
                    res[s] = res.get(s, 0) + w * p * pe
                    for i in range(n):
                        if ws[i]: t = list(s); t[i] += 1; t = tuple(t); nxt[t] = nxt.get(t, 0) + p * ws[i] / sum(ws)
                if not nxt: break
                cur = nxt; e += 1
        return res

    def deck_eval(dk, deck_size=6):
        ch = dk.get('choices') or [''] * len(dk['card_ids'])
        srcs = [card_src(cid, e, lb) for cid, e, lb in zip(dk['card_ids'], dk['end_chain_at'], ch)]
        filler = deck_size - len(dk['card_ids']) - len(dk.get('plain_ids', []))
        chs = [s for s in srcs if s['kind']]
        dist = chain_dist([s['kind'] for s in chs] + [(2, 2, ())] * filler)
        memo = {}
        def pmiss(req):
            if req in memo: return memo[req]
            pr = 1.0
            for s in srcs:
                tr, h, o, hinc = s['train']
                n = sum(1 for k in tr if req[k] == 2)
                if n: pr *= hint_miss(n, o, h, hinc)
                pr *= grp_ok(s['fixed'], req)
            tot = 0.0
            for st, p in dist.items():
                x = p
                for i, s in enumerate(chs):
                    for lk, groups in s['chain'].items():
                        if st[i] >= lk: x *= grp_ok(groups, req)
                tot += x
            memo[req] = pr * tot
            return memo[req]
        return pmiss

    def lineage(pmiss, lv, q, ew):
        coef = [(GG, 0, 0) if v == 2 else (GG, GG - GW, 0) if v == 1 else (GG, GG - GW, GW * (1 - ew[k]) * (1 - q)) for k, v in enumerate(lv)]
        tot = 0.0
        for req in itertools.product(range(3), repeat=K):
            c = 1.0
            for k, r in enumerate(req):
                c *= coef[k][0] if r == 0 else -coef[k][r]
                if c == 0: break
            if c: tot += c * pmiss(req)
        return tot * GG ** nfix

    worst = 0.0
    for top in out.get('top', []):
        lp = top['loop']; decks = top.get('decks') or [top['deck']] * 4
        pm = [deck_eval(dk) for dk in decks]
        # each lineage at the trainee's best outfit, as the solver does: per outfit the targets it carries (and the job's
        # `extra` / white guarantees), and the trainee events that hint a target's white (`trainee_hints`)
        def options(t):
            uid = lp['members'][t]['id']
            outs = [o for o in skills_db['cards'].get(str(uid), []) if o[0] not in job.get('exclude_outfits', [])] or [[0, 'any', []]]
            res = []
            for o in outs:
                lv = []
                for i, g in enumerate(unc):
                    l = max([x[1] for x in o[2] if x[0] == g] + [x[2] for x in job.get('extra', []) if x[0] == uid and x[1] == g] + [0])
                    if job['targets'][[t_['group'] for t_ in job['targets']].index(g)].get('guaranteed', 0) == 1: l = max(l, 1)
                    lv.append(l)
                ids = {uid, o[0]}
                ew = [1 - math.prod(1 - h[2] for h in job.get('trainee_hints', []) if h[0] in ids and h[1] == g) for g in unc]
                res.append((lv, ew))
            return res
        mine = sum(max(lineage(pm[L['t']], lv, L['q'], ew) for lv, ew in options(L['t'])) for L in lp['detail']['lineages']) / 12
        worst = max(worst, abs(mine / lp['p_all'] - 1))
    return worst


tag = sys.argv[1]; bad = 0
for era, j in json.load(open(os.path.join(V, 'jobs.json'))):
    job = json.load(open(os.path.join(V, 'jobs', j + '.json')))
    if job.get('mode') != 'deck': continue
    name = era + '__' + j
    for m in ('', 'pu'):
        f = os.path.join(V, f'{tag}{m}__{name}.out')
        if not os.path.exists(f): continue
        w = check(json.load(open(f)), job, os.path.join(S, 'pkg', 'loopopt', 'data', era))
        flag = w > 1e-3; bad += flag
        print(f"{m or 'sh'} {name}: max |python / solver - 1| = {w:.1e}{'  <<' if flag else ''}", flush=True)
print('all within 1e-3' if not bad else f'{bad} outside 1e-3')
