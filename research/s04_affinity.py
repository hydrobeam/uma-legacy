"""Section 4: the self-grandparent fit, tri(T, P, T) = 0, and a check of the records' affinity tables.

Reproduces from the run records (research/data/runs.jsonl, not included; Independent Training runs from uma-it):
- the trials: one per (ancestor, skill white spark, career year 2 or 3) where that spark's skill is carried by exactly
  one of the six ancestors, so a year's inspiration of it can come only from her; a trial passes when that factor shows
  among the year's inspirations. Runs where the trainee equals a parent, or the parents are one character, are left out.
- each trial's predicted chance is one roll, min(1, b (1 + A / 100)), b = 3/6/9% by stars, A the ancestor's affinity
  (section 4) from relation points (master.mdb, a group counting distinct characters) plus 3 x the record's shared G1
  wins for each pair. H0 sets tri(T, P, T) = 0 for a self-grandparent; H1 sets it to pair(T, P); kappa scales between.
- the log-likelihood difference sums the self-grandparent trials and their parents' trials (both move with tri).
Also checks the records' relation-point column (base) against pair and tri computed from master.mdb.
"""
import collections, math
import common

SEC = '4'
BASE = (0.03, 0.06, 0.09)
G_OF = {0: (1, 2), 3: (4, 5)}           # ancestor slots: 0 P1, 1 G1a, 2 G1b, 3 P2, 4 G2a, 5 G2b
GLBL = {1: 'parent 1 × grandparent 1a', 2: 'parent 1 × grandparent 1b', 4: 'parent 2 × grandparent 2a',
        5: 'parent 2 × grandparent 2b'}


def relations():
    db = common.mdb()
    pt = dict(db.execute('select relation_type, relation_point from succession_relation'))
    mem = collections.defaultdict(set)
    for t, ch in db.execute('select relation_type, chara_id from succession_relation_member'): mem[ch].add(t)
    ftype = dict(db.execute('select factor_id, factor_type from succession_factor'))

    def S(*xs):   # relation points of the groups holding every listed character (set semantics)
        return sum(pt[t] for t in set.intersection(*[mem[x] for x in xs]))
    return S, ftype


def lineage(r):
    """[P1, G1a, G1b, P2, G2a, G2b] as (chara, factors) or None when a slot is missing"""
    out = []
    for p in r.get('parents') or []:
        out.append(p)
        out.extend((p.get('gps') or [])[:2])
    if len(out) != 6 or any(a is None or not a.get('chara') for a in out): return None
    return out


def trials(R):
    S, ftype = relations()
    out = []   # (category, a0, a1, base, passed)
    for r in R:
        anc = lineage(r)
        if anc is None: continue
        T = r['chara']; A = [a['chara'] for a in anc]
        if len({T, A[0], A[3]}) < 3: continue
        aff = r.get('affinity') or {}
        if any(l not in aff for l in list(GLBL.values()) + ['parent 1 × parent 2']): continue
        shP = 3 * aff['parent 1 × parent 2'][1]
        shG = {g: 3 * aff[l][1] for g, l in GLBL.items()}
        yrs = r.get('years') or {}
        if '2' not in yrs or '3' not in yrs: continue
        got = {y: {f for f, _ in yrs[y]} for y in ('2', '3')}

        def tri(p, g, h):   # h = 0: tri(T, P, T) = 0; h = 1: tri(T, P, T) = pair(T, P)
            if A[g] in (T, A[p]): return S(T, A[p]) if h else 0
            return S(T, A[p], A[g])
        aff0, aff1 = {}, {}
        for p, o in ((0, 3), (3, 0)):
            for h, d in ((0, aff0), (1, aff1)):
                d[p] = S(T, A[p]) + S(A[p], A[o]) + shP + sum(tri(p, g, h) + shG[g] for g in G_OF[p])
                for g in G_OF[p]: d[g] = tri(p, g, h) + shG[g]
        sp = [{f // 100: f for f, _ in (a.get('factors') or [])} for a in anc]
        carriers = collections.Counter(n for s in sp for n in s)
        for i in range(6):
            if i in (0, 3):
                gs = G_OF[i]
                cat = 'P_selfgp' if any(A[g] == T for g in gs) else 'P_gpp' if any(A[g] == A[i] for g in gs) else 'P'
            else:
                p = 0 if i < 3 else 3
                cat = 'G_self' if A[i] == T else 'G_eqP' if A[i] == A[p] else 'G'
            for n, f in sp[i].items():
                if ftype.get(f) != 4 or carriers[n] != 1: continue
                for y in ('2', '3'):
                    out.append((cat, aff0[i], aff1[i], BASE[f % 10 - 1], f in got[y]))
    return out


def ll(ts, k):
    """log-likelihood with tri(T, P, T) = k pair(T, P)"""
    s = 0.0
    for _, a0, a1, b, f in ts:
        p = min(1.0, b * (1 + (a0 + k * (a1 - a0)) / 100))
        s += math.log(p if f else 1 - p)
    return s


def table_check(R):
    """the records' relation-point column (base) against master.mdb: trainee x parent and parent x parent rows against
    pair, parent x grandparent rows against pair(P, G) and tri(T, P, G); bonus = 3 x shared; total = base + bonus"""
    S, _ = relations()
    pair = lambda a, b: S(a, b) if a != b else 0
    c = collections.Counter()
    for r in R:
        anc = lineage(r); aff = r.get('affinity') or {}
        if anc is None or not aff: continue
        T = r['chara']; A = [a['chara'] for a in anc]
        for l, (x, y) in {'trainee × parent 1': (T, A[0]), 'trainee × parent 2': (T, A[3]),
                          'parent 1 × parent 2': (A[0], A[3])}.items():
            v = aff[l][0]; m = pair(x, y)
            c[('pair rows', 'base = pair' if v == m else 'base 0, pair > 0' if v == 0 else 'other')] += 1
        for g, l in GLBL.items():
            p = 0 if g < 3 else 3; v = aff[l][0]
            t = S(T, A[p], A[g]) if len({T, A[p], A[g]}) == 3 else 0
            c[('P x G rows', 'base = pair(P,G)' if v == pair(A[p], A[g]) else 'base = tri(T,P,G)' if v == t
               else 'base 0' if v == 0 else 'other')] += 1
        for v in aff.values():
            c[('all rows', 'bonus = 3 x shared, total = base + bonus' if v[2] == 3 * v[1] and v[3] == v[0] + v[2]
               else 'other')] += 1
    return c


def claims():
    R = common.runs()
    items = [('trials attributable to one ancestor', '101,536'), ('self-grandparent passes observed', '523'),
             ('predicted, tri(T,P,T) = 0', '521.8'), ('predicted, tri(T,P,T) = pair(T,P)', '605.4'),
             ('LL(0) - LL(pair), nats', '13'), ('kappa fitted', '0'), ('kappa 95% upper limit', '0.35')]
    if R is None: return common.need_data(SEC, items)
    ts = trials(R)
    gs = [t for t in ts if t[0] == 'G_self']
    sub = [t for t in ts if t[0] in ('G_self', 'P_selfgp')]
    pred = lambda k: sum(min(1.0, b * (1 + (a0 + k * (a1 - a0)) / 100)) for _, a0, a1, b, _ in gs)
    grid = [i / 1000 for i in range(0, 1001)]
    L = [ll(sub, k) for k in grid]
    i = max(range(len(grid)), key=L.__getitem__)
    up = next((k for k, v in zip(grid[i:], L[i:]) if L[i] - v > 1.920729), None)   # chi2(1) 95% / 2
    got = [len(ts), sum(f for *_, f in gs), pred(0), pred(1), ll(sub, 0) - ll(sub, 1), grid[i], up]
    out = []
    for (n, s), g in zip(items, got):
        out.append(common.Claim(SEC, n, s, g, tol=0.5 if 'nats' in n else 0.05 if 'upper' in n else 0,
                                note='"about 13"' if 'nats' in n else '"about 0.35"' if 'upper' in n else ''))
    return out


def detail():
    R = common.runs()
    ts = trials(R)
    agg = collections.defaultdict(lambda: [0, 0, 0.0, 0.0])
    for cat, a0, a1, b, f in ts:
        x = agg[cat]; x[0] += 1; x[1] += f
        x[2] += min(1, b * (1 + a0 / 100)); x[3] += min(1, b * (1 + a1 / 100))
    print(f'{"ancestor":10s} {"trials":>7s} {"passed":>7s} {"pred H0":>8s} {"pred H1":>8s}')
    for k in ('P', 'P_selfgp', 'P_gpp', 'G', 'G_self', 'G_eqP'):
        x = agg[k]; print(f'{k:10s} {x[0]:7d} {x[1]:7d} {x[2]:8.1f} {x[3]:8.1f}')
    sub = [t for t in ts if t[0] in ('G_self', 'P_selfgp')]
    print('LL by kappa:', ', '.join(f'{k}: {ll(sub, k) - ll(sub, 0):+.2f}' for k in (0, 0.25, 0.35, 0.5, 1)))
    print('\nrecords\' affinity tables against master.mdb relation points:')
    for k, v in sorted(table_check(R).items()): print(f'  {k[0]:12s} {k[1]:16s} {v}')


if __name__ == '__main__':
    common.main(claims)
    if common.have_runs(): print(); detail()
