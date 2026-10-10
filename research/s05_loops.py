"""Section 5, loop structures: the race weights (derived by expanding the six legacy affinities of every lineage of a
four-uma loop and a three-uma rotation into shared-win terms o(a, b)) and how often a trainee's own character appears
among her grandparents in the run records."""
import collections, itertools
import common
from common import Claim

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


def claims():
    out = []
    one = lineage_terms('T', 'P1', 'P2', 'X')
    out += [Claim(S, 'one lineage: o(P1, P2) weight', '6', one[frozenset(('P1', 'P2'))]),
            Claim(S, 'one lineage: o(P1, X) weight', '2', one[frozenset(('P1', 'X'))])]
    m = 'abcd'
    loop = [(t, p1, p2, x) for t in m for p1, p2 in itertools.combinations([u for u in m if u != t], 2)
            for x in [u for u in m if u not in (t, p1, p2)]]
    w = weights(loop)
    out.append(Claim(S, f'loop: {len(loop)} lineages, weight per pair', '20', w[frozenset('ab')] if len(set(w.values())) == 1 else -1))
    r = 'abc'
    rot = [(t, *[u for u in r if u != t], t) for t in r]
    w = weights(rot)
    out.append(Claim(S, f'rotation: {len(rot)} lineages, weight per pair', '10', w[frozenset('ab')] if len(set(w.values())) == 1 else -1))
    R = common.runs()
    if R is None:
        out += common.need_data(S, [('runs', '3,179'), ('a grandparent of her own character', '907'), ('a parent of her own character', '0')])
    else:
        out += [Claim(S, 'runs', '3,179', len(R)),
                Claim(S, 'a grandparent of her own character', '907', sum(any(g['chara'] == r['chara'] for p in r['parents'] for g in p['gps']) for r in R)),
                Claim(S, 'a parent of her own character', '0', sum(any(p['chara'] == r['chara'] for p in r['parents']) for r in R))]
    out += [Claim(S, 'schedules re-optimised for odds: max gain', '+0.12%', status='needs solver experiment'),
            Claim(S, 'same, on a winner', '+0.065%', status='needs solver experiment')]
    return out


if __name__ == '__main__': common.main(claims)
