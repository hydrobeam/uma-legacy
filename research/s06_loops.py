"""Section 5, loop structures: the race weights (derived by expanding the six legacy affinities of every lineage of a
four-uma loop and a three-uma rotation into shared-win terms o(a, b)) and how often a trainee's own character appears
among her grandparents in the run records (research/data/runs.jsonl, not included; Independent Training runs from
uma-it), with how many lineages have the exact shape of a rotation or a loop lineage (by character).

Schedules for shared wins, not odds (derived, then computed on the validation results). The solver picks a structure's
race schedules to maximise sum over pairs of o(a, b) = sum_r P_a(r) P_b(r), not P-bar directly. P-bar is increasing in
every o(a, b):
  o(a, b) enters a legacy's affinity A only with coefficient +3 (A_P = pair + pair + 3 o(P, P') + sum_G [tri + 3 o(P, G)],
  A_G = tri + 3 o(P, G));
  r = min(1, b (1 + A / 100)) is nondecreasing in A;
  q = 1 - prod over the six slots of (1 - r)^2 is nondecreasing in each r;
  P_lambda(all) = sum_c P(c) prod_k p_k(c), over the deck's outcomes c (independent of the schedule), where p_k(c) >= 0
  is nondecreasing in q (an uncertain target's white is GW (1 - (1 - hint_k(c)) (1 - ew_k) (1 - q))), so P_lambda is
  nondecreasing in q; the max over the trainee's outfits and the mean over the lineages keep that.
A schedule attempts race r at a subset of the turns that offer it, each at clamp(bp - pen_k, 0, 100)% <= clamp(bp, 0,
100)% (pen_k >= 0), so P_a(r) <= Pmax_a(r) = 1 - prod over every turn offering r to a of (1 - clamp(bp, 0, 100)/100), and
o(a, b) <= o_max(a, b) = sum_r Pmax_a(r) Pmax_b(r) for any schedule. Hence, for a solved loop with its decks fixed, any
schedule, including one chosen for P-bar directly, scores between P_found and P_ceiling (every pair at o_max), and
P_floor (every o = 0) is the loop without race affinity. Computed on every deck result of the validation reference
outputs (val/ref__*.out, val/refpu__*.out) with val/joint_check.py's independent Python recomputation of the exact
odds, the affinities rebuilt here from the reported pairs' o and relation parts; o itself is recomputed from the
reported schedule chart and o_max from data.json. o_max is far from reachable (every offering of every G1, no streak
penalty, conflicting races on one turn all run), so a tighter ceiling is also used: PB(a, b), data.json's "pb" table,
computed by the solver's best-response search over feasible schedules (`loopopt pbtable`, docs/SOLVER.md
`pair_ceilings`; not independent of the solver, only checked here to sit between o and o_max). P_PB is the odds with
every pair at PB."""
import collections, itertools, json, math, os, statistics, struct, sys
import common
from common import Value

S = '6'


def lineage_terms(T, P1, P2, X):
    """coefficient of each o(a, b) (unordered pair) in the six affinities of lineage (T, P1, P2, X), in units of 3:
    A_P = ... + 3 o(P, P') + sum over G in par(P) of 3 o(P, G); A_G = ... + 3 o(P, G) (RESEARCH.md section 5)"""
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
    out += schedule_claims()
    return out


# ---------------------------------------------------------------- schedules: the bound on a different objective
VAL = os.path.join(common.ROOT, 'val')


def o_chart(lp):
    """o(a, b) from a result's schedule chart: P_a(r) = 1 - prod over a's attempts at race r (by name, either year) of
    (1 - win %), the cell's win % including the streak penalty"""
    n = len(lp['members']); miss = [collections.defaultdict(lambda: 1.0) for _ in range(n)]
    for row in lp['schedule']:
        for a, c in enumerate(row['cells']):
            if c is not None: miss[a][row['race']] *= 1 - c['p'] / 100
    P = [{r: 1 - m for r, m in mm.items()} for mm in miss]
    return {(a, b): sum(v * P[b].get(r, 0.0) for r, v in P[a].items()) for a, b in itertools.combinations(range(n), 2)}


def o_max(lp, data):
    """o_max(a, b) = sum_r Pmax_a(r) Pmax_b(r), Pmax over every turn offering race r, no streak penalty"""
    umas = {u['id']: u for u in data['umas']}
    pm = []
    for m in lp['members']:
        miss = collections.defaultdict(lambda: 1.0)
        for turn in umas[m['id']]['turns']:
            for op in turn:
                if op['race'] >= 0: miss[op['race']] *= 1 - min(100, max(0, op['bp'])) / 100
        pm.append({r: 1 - x for r, x in miss.items()})
    n = len(pm)
    return {(a, b): sum(v * pm[b].get(r, 0.0) for r, v in pm[a].items()) for a, b in itertools.combinations(range(n), 2)}


def o_pb(lp, data):
    """the solver's exact pairwise ceilings PB(a, b) (data.json "pb", written by `loopopt pbtable`: the best-response DP
    over feasible schedules, docs/SOLVER.md `pair_ceilings`); tighter than o_max but computed by the Rust solver"""
    idx = {u['id']: i for i, u in enumerate(data['umas'])}; rows = data['pb']['rows']
    def pb(a, b):
        a, b = sorted((a, b)); k = b - a - 1
        return struct.unpack('<d', int(rows[a][16 * k:16 * k + 16], 16).to_bytes(8, 'little'))[0]
    ids = [idx[m['id']] for m in lp['members']]
    return {(a, b): pb(ids[a], ids[b]) for a, b in itertools.combinations(range(len(ids)), 2)}


def lineage_q(L, o, det):
    """q of lineage L when the pairs' shared wins are o: each slot's affinity = its relation part + 3 x its o terms
    (section 5), r = min(1, b (1 + A/100)) (x gp_rate for a grandparent), q = 1 - prod (1 - r)^inspirations; also the
    slots' race parts, to check against the reported ones"""
    (p1, p2), x = L['p'], L['x']
    par = {p1: (p2, x), p2: (p1, x)}
    oo = lambda a, b: o[(min(a, b), max(a, b))]
    miss, races = 1.0, []
    for sl in L['slots']:
        if sl['kind'] == 'parent':
            P, = sl['who']; Pp = p2 if P == p1 else p1
            race = 3 * (oo(P, Pp) + sum(oo(P, G) for G in par[P])); b = det['base']
        else:
            P, G = sl['who']; assert G in par[P]
            race = 3 * oo(P, G); b = det['gp_rate'] * det['base']
        miss *= 1 - min(1.0, b * (1 + (sl['rel'] + race) / 100)); races.append(race)
    return 1 - miss ** det['inspirations'], races


def schedule_bound(tags=('ref', 'refpu')):
    """per deck result of the validation reference outputs: the winner's (and each listed runner-up loop's) P_found,
    P_floor, P_ceiling, recomputed by val/joint_check.py at q rebuilt from o"""
    sys.path.insert(0, VAL)
    import joint_check
    rows, err = [], {'p': 0.0, 'q': 0.0, 'race': 0.0, 'o': 0.0}
    pens = set()
    for era, j in json.load(open(os.path.join(VAL, 'jobs.json'))):
        job = json.load(open(os.path.join(VAL, 'jobs', j + '.json')))
        if job.get('mode') != 'deck': continue
        dd = os.path.join(common.ROOT, 'pkg', 'loopopt', 'data', era)
        data = json.load(open(os.path.join(dd, 'data.json'))); pens.update(data['pen'])
        for tag in tags:
            f = os.path.join(VAL, f'{tag}__{era}__{j}.out')
            if not os.path.exists(f): continue
            out = json.load(open(f)); tops = out['top']
            # the winner, and each listed loop with other members (a listed same-loop entry differs only in its deck)
            keep = [t for i, t in enumerate(tops) if i == 0 or [m['id'] for m in t['loop']['members']]
                    != [m['id'] for m in tops[0]['loop']['members']]]
            qv = {}
            for t in keep:
                lp = t['loop']; det = lp['detail']
                o_rep = {(p['a'], p['b']): p['wins'] for p in det['pairs']}
                for k, v in o_chart(lp).items(): err['o'] = max(err['o'], abs(v - o_rep[k]))
                om = o_max(lp, data); opb = o_pb(lp, data)
                vs = []
                for o in (o_rep, {k: 0.0 for k in o_rep}, om, opb):
                    vs.append([lineage_q(L, o, det)[0] for L in det['lineages']])
                for L, q in zip(det['lineages'], vs[0]):
                    err['q'] = max(err['q'], abs(q - L['q']))
                    for sl, race in zip(L['slots'], lineage_q(L, o_rep, det)[1]): err['race'] = max(err['race'], abs(race - sl['race']))
                qv[id(t)] = vs
                rows.append(dict(file=f'{tag}__{era}__{j}', top=t is tops[0], members=len(lp['members']), rep=lp['p_all'],
                                 o_sum=sum(o_rep.values()), omax_sum=sum(om.values()),
                                 o_le_max=all(o_rep[k] <= om[k] + 1e-9 for k in o_rep),
                                 o_le_pb=all(o_rep[k] <= opb[k] + 1e-9 and opb[k] <= om[k] + 1e-9 for k in o_rep)))
            _, alt = joint_check.check(dict(out, top=keep), job, dd, alt_q=lambda t: qv[id(t)])
            for r, (pf, p0, pc, pb) in zip(rows[-len(keep):], alt):
                r.update(found=pf, floor=p0, ceil=pc, pb=pb); err['p'] = max(err['p'], abs(pf / r['rep'] - 1))
    return rows, err, pens


def schedule_claims():
    out = []
    rows, err, pens = schedule_bound()
    W = [r for r in rows if r['top']]; RU = [r for r in rows if not r['top']]
    gain = [100 * (r['ceil'] / r['found'] - 1) for r in W]
    worth = [100 * (r['found'] / r['floor'] - 1) for r in W]
    left = [100 * (r['ceil'] - r['found']) / (r['found'] - r['floor']) for r in W]
    over = [100 * r['o_sum'] / r['omax_sum'] for r in W]
    gpb = [100 * (r['pb'] / r['found'] - 1) for r in W]
    out += [Value(S, 'schedules: result files', len(W)),
            Value(S, 'schedules: four-uma winners', sum(r['members'] == 4 for r in W)),
            Value(S, 'schedules: recomputed odds max relative error', err['p']),
            Value(S, 'schedules: q max error', err['q']),
            Value(S, 'schedules: o from the chart max error', err['o']),
            Value(S, 'schedules: ceiling gain max %', max(gain)),
            Value(S, 'schedules: ceiling gain median %', statistics.median(gain)),
            Value(S, 'schedules: ceiling gain min %', min(gain)),
            Value(S, 'schedules: shared-win value min %', min(worth)),
            Value(S, 'schedules: shared-win value median %', statistics.median(worth)),
            Value(S, 'schedules: shared-win value max %', max(worth)),
            Value(S, 'schedules: ceiling as % of the shared-win value, max', max(left)),
            Value(S, 'schedules: overlap found as % of o_max, min', min(over)),
            Value(S, 'schedules: overlap found as % of o_max, median', statistics.median(over)),
            Value(S, 'schedules: runner-up loops checked', len(RU)),
            Value(S, 'schedules: PB ceiling gain max %', max(gpb)),
            Value(S, 'schedules: PB ceiling gain median %', statistics.median(gpb)),
            Value(S, 'schedules: PB ceiling gain min %', min(gpb))]
    out.append(common.finding(S, 'The independent recomputation reproduces every validated loop\'s reported odds and q from '
                              'the reported o and relation parts, and o from the schedule chart',
                              err['p'] < 1e-9 and err['q'] < 1e-12 and err['race'] < 1e-9 and err['o'] < 1e-9,
                              f'max |P/reported - 1| {err["p"]:.1e}, |dq| {err["q"]:.1e}, |d race part| {err["race"]:.1e}, '
                              f'|d o| {err["o"]:.1e} over {len(rows)} loops'))
    out.append(common.finding(S, 'The solver\'s pair ceilings sit between: o(a, b) <= PB(a, b) <= o_max(a, b) on every pair of '
                              'every loop', all(r['o_le_pb'] for r in rows), f'{sum(r["o_le_pb"] for r in rows)}/{len(rows)} loops'))
    out.append(common.finding(S, 'The bound applies: streak penalties are >= 0, every reported o(a, b) is at most o_max(a, b), '
                              'and on every loop P_floor <= P_found <= P_ceiling (the odds rise with o, as derived)',
                              min(pens) >= 0 and all(r['o_le_max'] and r['floor'] <= r['found'] <= r['ceil'] for r in rows),
                              f'penalties {sorted(pens)}; {sum(r["o_le_max"] for r in rows)}/{len(rows)} loops with o <= o_max; '
                              f'{sum(r["floor"] <= r["found"] <= r["ceil"] for r in rows)}/{len(rows)} ordered'))
    out.append(common.finding(S, 'On every loop P_found <= P_PB <= P_ceiling (the odds rise with o, and o <= PB <= o_max)',
                              all(r['found'] <= r['pb'] <= r['ceil'] for r in rows),
                              f'{sum(r["found"] <= r["pb"] <= r["ceil"] for r in rows)}/{len(rows)} ordered; PB ceiling gain on '
                              f'the winners {min(gpb):.2f}-{max(gpb):.2f}% (median {statistics.median(gpb):.2f}%)'))
    out.append(common.finding(S, 'No schedule, including one chosen for P-bar directly, can raise a validated winner\'s odds by '
                              'as much as the shared wins it already has are worth (ceiling gain < shared-win value)',
                              all(r['ceil'] - r['found'] < r['found'] - r['floor'] for r in W),
                              f'ceiling gain {min(gain):.2f}-{max(gain):.2f}% (median {statistics.median(gain):.2f}%); shared wins '
                              f'worth {min(worth):.1f}-{max(worth):.1f}% (median {statistics.median(worth):.1f}%); '
                              f'ceiling at most {max(left):.1f}% of that'))
    if RU:
        pas = [r for r in RU if r['ceil'] >= next(w for w in W if w['file'] == r['file'])['found']]
        ppb = [r for r in RU if r['pb'] >= next(w for w in W if w['file'] == r['file'])['found']]
        out.append(Value(S, 'schedules: runner-ups whose ceiling reaches the winner', len(pas)))
        out.append(Value(S, 'schedules: runner-ups whose PB ceiling reaches the winner', len(ppb)))
        out.append(common.finding(S, 'The o_max bound alone cannot settle the ranking: some listed runner-up loop\'s o_max '
                                  'ceiling reaches its winner\'s odds found', bool(pas),
                                  f'{len(pas)} of {len(RU)} reach; ' +
                                  '; '.join(f'{r["file"]}: runner-up ceiling {r["ceil"]:.6f} vs winner '
                                            f'{next(w for w in W if w["file"] == r["file"])["found"]:.6f}' for r in RU)))
        out.append(common.finding(S, 'With the solver\'s pair ceilings, no schedule could reorder a listed top: no listed '
                                  'runner-up loop\'s PB ceiling reaches its winner\'s odds found', not ppb,
                                  f'{len(ppb)} of {len(RU)} reach; ' +
                                  '; '.join(f'{r["file"]}: runner-up PB ceiling {r["pb"]:.6f} vs winner '
                                            f'{next(w for w in W if w["file"] == r["file"])["found"]:.6f}' for r in RU)))
    return out


if __name__ == '__main__': common.main(claims)
