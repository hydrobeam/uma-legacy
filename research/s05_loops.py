"""Section 5, loop structures: the race weights (derived by expanding the six legacy affinities of every lineage of a
four-uma loop and a three-uma rotation into shared-win terms o(a, b)) and how often a trainee's own character appears
among her grandparents in the run records (research/data/runs.jsonl, not included; Independent Training runs from
uma-it), with how many lineages have the exact shape of a rotation or a loop lineage (by character)."""
import collections, itertools, math
import common
from common import Value

S = '5'


def lineage_terms(T, P1, P2, X):
    """coefficient of each o(a, b) (unordered pair) in the six affinities of lineage (T, P1, P2, X), in units of 3:
    A_P = ... + 3 o(P, P') + sum over G in par(P) of 3 o(P, G); A_G = ... + 3 o(P, G) (RESEARCH.md section 4)"""
    c = collections.Counter()
    par = {P1: (P2, X), P2: (P1, X)}
    for P, Pp in ((P1, P2), (P2, P1)):
        c[frozenset((P, Pp))] += 1                       # A_P: o(P, P')
        for G in par[P]:
            c[frozenset((P, G))] += 1                    # A_P: o(P, G)
            c[frozenset((P, G))] += 1                    # A_G: o(P, G)
    return c


def weights(lineages):
    tot = collections.Counter()
    for l in lineages: tot.update(lineage_terms(*l))
    return tot


def shape(r):
    """'rotation' when the lineage is (T, P1, P2, T) by character, 'loop' when (T, P1, P2, X) with X not T, else None"""
    T, ps = r['chara'], r['parents']
    if len(ps) != 2 or any(len(p['gps']) != 2 for p in ps): return None
    a, b = ps[0]['chara'], ps[1]['chara']
    ga, gb = sorted(g['chara'] for g in ps[0]['gps']), sorted(g['chara'] for g in ps[1]['gps'])
    X = [x for x in ga if x != b]
    if len(X) != 1 or ga != sorted([b, X[0]]) or gb != sorted([a, X[0]]): return None
    return 'rotation' if X[0] == T else 'loop'


def claims():
    out = []
    one = lineage_terms('T', 'P1', 'P2', 'X')
    out += [Value(S, 'one lineage: o(P1, P2) weight', one[frozenset(('P1', 'P2'))]),
            Value(S, 'one lineage: o(P1, X) weight', one[frozenset(('P1', 'X'))])]
    m = 'abcd'
    loop = [(t, p1, p2, x) for t in m for p1, p2 in itertools.combinations([u for u in m if u != t], 2)
            for x in [u for u in m if u not in (t, p1, p2)]]
    wl = weights(loop)
    out += [Value(S, 'four-uma loop: lineages', len(loop)),
            Value(S, 'loop: weight per pair', wl[frozenset('ab')] if len(set(wl.values())) == 1 else -1)]
    r = 'abc'
    rot = [(t, *[u for u in r if u != t], t) for t in r]
    wr = weights(rot)
    out += [Value(S, 'three-uma rotation: lineages', len(rot)),
            Value(S, 'rotation: weight per pair', wr[frozenset('ab')] if len(set(wr.values())) == 1 else -1)]
    out.append(common.finding(S, 'Race weights are uniform over pairs, so a structure\'s schedules maximise the plain sum of '
                              'o(a, b): every one of the 6 loop pairs and the 3 rotation pairs gets one weight',
                              len(wl) == math.comb(len(m), 2) and len(set(wl.values())) == 1
                              and len(wr) == math.comb(len(r), 2) and len(set(wr.values())) == 1,
                              f'loop weights {sorted(wl.values())}, rotation weights {sorted(wr.values())}'))
    R = common.runs()
    if R is not None:
        gp = sum(any(g['chara'] == r['chara'] for p in r['parents'] for g in p['gps']) for r in R)
        par = sum(any(p['chara'] == r['chara'] for p in r['parents']) for r in R)
        sh = collections.Counter(shape(r) for r in R)
        out += [Value(S, 'runs', len(R)), Value(S, 'a grandparent of her own character', gp),
                Value(S, 'a parent of her own character', par)]
        lo, hi = common.wilson(gp, len(R)); plo, phi = common.wilson(par, len(R))
        out.append(common.finding(S, 'The rotation (X = T) occurs in practice: runs with a grandparent of the trainee\'s own '
                                  'character, and runs whose lineage has the exact rotation shape, are both present',
                                  gp > 0 and sh['rotation'] > 0,
                                  f'own-character grandparent {gp:,}/{len(R):,} = {gp / len(R):.3f} [{lo:.3f}, {hi:.3f}]; '
                                  f'exact rotation lineages {sh["rotation"]}, exact loop lineages {sh["loop"]}'))
        out.append(common.finding(S, 'X = T sits only at grandparent level: no run has a parent of the trainee\'s own '
                                  'character', par == 0, f'{par}/{len(R):,}, 95% upper {phi:.4f}'))
    return out


if __name__ == '__main__': common.main(claims)
