"""Section 4: the race win table, tested against the races Independent Training runs actually ran.

The model (pkg/loopopt/src/lib.rs, data.json `pen`): an attempt at one of the 34 single-race G1s wins with
p = clamp(bp - PEN_j, 0, 100) / 100, bp = 110 - cost(distance grade) - cost(surface grade) from her base aptitudes
(data.json umas[].turns[t][].bp; cost S/A 0, B 10, C 20, D 30, E 40, F 60, G 90), PEN_j = data.json `pen` for the j-th
race in a row (any race, goals included; a turn without a race resets). A community table, assumed.

Reproduces from the run records (research/data/runs.jsonl, not included; Independent Training runs from uma-it):
- every attempt at one of the 34 by a trainee in data.json: the record's race name (after ALIAS), matched to the
  option data.json offers her at that turn index, t = (year - 1) * 24 + (month - 1) * 2 + (half - 1); attempts
  that do not match are counted and listed by detail();
- j from the run's own race turns (every race the career ran: debut, goals, G2/G3/OP);
- won = the record's win flag; predicted p from the table; obs / pred with its run-clustered 95% interval
  (common.cluster_ci: one run gives several attempts), overall, by bp, by j, by scenario, goal or chosen, and by
  predicted p;
- log-likelihoods on the same attempts: the table as stated (minus infinity when a certain prediction misses), a
  constant rate, the table with and without streak penalties each recalibrated as lo + (hi - lo) p (2 parameters),
  and free fits: a rate per (bp, j) cell and per bp (j capped at 6); these are in-sample, attempts treated as
  independent; the cells against the recalibrated pink table are also compared held out (5-fold by run, held_out_cells);
- aptitude raised by pink sparks: the planner's rule (docs/MODEL.md: at the start the six legacies' pink stars per
  aptitude raise the base grade by 1/2/3/4 at 1/4/7/10 stars, at most A; each April firing recorded in the run, years
  2 and 3 from turns 30 and 54 on, +1 grade, at most S) gives a pink-adjusted bp. Every calibration is reported on both
  the base bp (what the solver uses) and the pink-adjusted bp, as are the attempts the rule moves and those after an
  April firing of the race's distance or surface; the fits use the pink-adjusted bp;
- where the residual sits: goal and chosen races from the 4th in a row; detail() also scores a goal race one streak
  position earlier (j - 1), a lead those cells suggest, on each half of the runs (record order).
Findings: one per claim of section 4 (base under-predicts; pink-adjusted calibrated overall and per scenario; pink
beats base; the streak penalty; the table not exact in-sample, the cells not better held out; the excess in goal races from the 4th in a row). The other
cells, the certain-outcome counts and the fits behind them are printed by detail().

Conditional on selection: IT chose to run every non-goal attempt, presumably when she was likely to win, and the
runs are probably skewed to good careers; goal races are run regardless (scored separately). Her stats, mood and
skills are not modelled, by the table or here. A win is the record's flag (first place).
"""
import collections, math
import numpy as np
from scipy.optimize import minimize
from scipy.stats import chi2
import common
from common import Value

SEC = '4'
ALIAS = {'M.C. Nambu Hai': 'Mile Championship Nambu Hai'}     # the records' short name for the data.json race
MON = {m: i for i, m in enumerate('Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec'.split())}
COST = {8: 0, 7: 0, 6: 10, 5: 20, 4: 30, 3: 40, 2: 60, 1: 90}   # grade codes 1..8 = G..S
PINK_DIST, PINK_SURF = (31, 32, 33, 34), {1: 11, 2: 12}         # pink factor groups (factor id // 100)
APRIL = {'2': 30, '3': 54}                                       # inspiration years -> first turn index they count for
BPB = ((110, '110'), (100, '100'), (90, '90'), (80, '80'), (-99, '<=70'))
PB = ((1.0, '100%'), (0.9, '90-99%'), (0.8, '80-89%'), (0.6, '60-79%'), (0.0, '<60%'))


def tindex(year, turn):
    m, h = turn.split()
    return (year - 1) * 24 + MON[m] * 2 + int(h) - 1


def win_p(bp, j, pen):
    return min(100, max(0, bp - pen[min(j, 6) - 1])) / 100


def lineage_pinks(r):
    """pink group -> stars summed over the six legacies"""
    out = collections.Counter()
    for p in r.get('parents') or []:
        for a in [p] + list(p.get('gps') or []):
            for f, _ in (a or {}).get('factors') or []:
                if f < 10000 and f // 100 in PINK_DIST + tuple(PINK_SURF.values()): out[f // 100] += f % 100
    return out


def attempts(R):
    """(attempts, skipped Counter, unmatched G1 names Counter): one dict per attempt at the 34"""
    d = common.gjson('data.json')
    U = {u['id']: u for u in d['umas']}
    idx = {n: i for i, n in enumerate(d['race_names'])}
    pen = d['pen']
    out, skip, unk = [], collections.Counter(), collections.Counter()
    for i, r in enumerate(R):
        races = sorted(((tindex(x['year'], x['turn']), x) for x in r['races']), key=lambda t: t[0])
        ts = [t for t, _ in races]
        if len(set(ts)) != len(ts): skip['run with two races on one turn'] += 1; continue
        u = U.get(r['chara'])
        pk = lineage_pinks(r)
        fired = collections.defaultdict(list)   # pink group -> first turn index of each firing
        for y, t0 in APRIL.items():
            for f, n in (r.get('years') or {}).get(y, []):
                if f < 10000 and f // 100 in pk: fired[f // 100] += [t0] * n
        j = 0; prev = None; pw = None
        for t, x in races:
            cons = prev == t - 1; j = j + 1 if cons else 1; prev = t
            last, pw = (pw if cons else None), int(x['won'])   # the previous race's result, when it was the turn before
            name = ALIAS.get(x['race'], x['race'])
            if name not in idx:
                if x['grade'] == 'G1': unk[x['race']] += 1
                continue
            if u is None: skip['trainee not in data.json'] += 1; continue
            ri = idx[name]
            op = next((o for o in u['turns'][t] if o['race'] == ri), None) if t < len(u['turns']) else None
            if op is None: skip['race not offered to her at that turn'] += 1; continue
            cat, gr = d['race_cat'][ri], d['race_ground'][ri]
            gd, gs = PINK_DIST[cat], PINK_SURF[gr]
            adj = []; early = []
            for k, g in ((cat, gd), (4 if gr == 1 else 5, gs)):
                a = u['apt'][k]
                a = max(a, min(a + sum(pk[g] >= s for s in (1, 4, 7, 10)), 7))
                adj.append(min(8, a + sum(t0 <= t for t0 in fired[g])))
                early.append(min(8, a + sum(t0 - 6 <= t for t0 in fired[g])))   # the year's proc counted from its first turn
            bpa = max(-20, 110 - COST[adj[0]] - COST[adj[1]])
            out.append(dict(run=r['id'], bp=op['bp'], bpa=bpa, apt=tuple(adj), j=j, won=int(x['won']), p=win_p(op['bp'], j, pen),
                            p0=win_p(op['bp'], 1, pen), pa=win_p(bpa, j, pen), pa0=win_p(bpa, 1, pen),
                            scen=r['scenario'], goal=bool(x['required']), outfit=r['outfit'],
                            fired=any(t0 <= t for g in (gd, gs) for t0 in fired[g]), later=2 * i >= len(R),
                            last=last, t=t, pe=win_p(max(-20, 110 - COST[early[0]] - COST[early[1]]), j, pen), pg=win_p(bpa, j - 1 if x['required'] and j > 1 else j, pen)))
    return out, skip, unk


# ---------------------------------------------------------------- statistics
def calib(A, key):
    """(n, observed rate, mean predicted, obs / pred, low, high): the run-clustered 95% interval of obs / pred
    (common.cluster_ci; one run gives several attempts)"""
    n = len(A)
    if not n: return 0, *(float('nan'),) * 5
    k = sum(a['won'] for a in A); e = sum(a[key] for a in A) / n
    r, lo, hi, _ = common.cluster_ci([a['won'] for a in A], [a[key] for a in A], [a['run'] for a in A])
    return n, k / n, e, r, lo, hi


def ll(ps, y):
    """log-likelihood; -inf when a certain prediction misses"""
    with np.errstate(divide='ignore'):
        return float(np.sum(np.where(y == 1, np.log(ps), np.log(1 - ps))))


def recal(q, y):
    """the table recalibrated as lo + (hi - lo) q: (LL, lo, hi), fitted by maximum likelihood"""
    sig = lambda z: 1 / (1 + np.exp(-z))
    nll = lambda th: -ll(np.clip(sig(th[0]) + (sig(th[1]) - sig(th[0])) * q, 1e-12, 1 - 1e-12), y)
    res = minimize(nll, [-1.0, 4.0], method='Nelder-Mead', options=dict(xatol=1e-8, fatol=1e-9, maxiter=20000))
    return -res.fun, sig(res.x[0]), sig(res.x[1])


def saturated(A, key):
    """(LL, cells): every cell of `key` gets its own observed rate"""
    g = collections.defaultdict(lambda: [0, 0])
    for a in A: c = g[key(a)]; c[0] += a['won']; c[1] += 1
    xl = lambda x: x * math.log(x) if x else 0.0
    return sum(xl(k / n) * n + xl(1 - k / n) * n for k, n in g.values()), len(g)


def thr(k):
    """the likelihood-ratio threshold for k extra parameters: chi2(k) 95% / 2"""
    return chi2.ppf(0.95, k) / 2


def fits(A):
    """model -> (LL, parameters) on the same attempts; '_recal' -> {prediction: (lo, hi)}"""
    y = np.array([a['won'] for a in A]); k = int(y.sum()); n = len(y)
    out = {'constant': (k * math.log(k / n) + (n - k) * math.log(1 - k / n), 1), '_recal': {}}
    for key, lab in (('p', 'table'), ('p0', 'no penalty'), ('pa', 'pink table'), ('pa0', 'pink no penalty')):
        q = np.array([a[key] for a in A])
        out[lab] = (ll(q, y), 0)
        L, lo, hi = recal(q, y); out[lab + ', recalibrated'] = (L, 2); out['_recal'][lab] = (lo, hi)
    out['pink cells bp x j'] = saturated(A, lambda a: (a['bpa'], min(a['j'], 6)))
    out['pink cells bp'] = saturated(A, lambda a: a['bpa'])
    return out


def held_out_cells(A):
    """rule 3 for the cells: 5-fold cross-validation by common.fold(run). On four folds, a rate per (pink bp, j) cell,
    (wins + 1/2) / (attempts + 1) (Jeffreys; a raw 0 or 1 makes the held-out score that of the 1e-12 clip; a cell absent
    there takes the pink table's chance), and the pink table, each recalibrated as lo + (hi - lo) q on those folds; each
    run of the fifth scored by both. Returns common.compare_held_out(cells, table) and the run count"""
    cell = lambda a: (a['bpa'], min(a['j'], 6))
    y = np.array([a['won'] for a in A]); pa = np.array([a['pa'] for a in A])
    run = np.array([a['run'] for a in A]); fo = np.array([common.fold(r) for r in run])
    lc = np.zeros(len(A)); lt = np.zeros(len(A))
    for k in range(common.K_FOLDS):
        tr, te = fo != k, fo == k
        g = collections.defaultdict(lambda: [0, 0])
        for a in (a for a, t in zip(A, tr) if t): c = g[cell(a)]; c[0] += a['won']; c[1] += 1
        q = np.array([(g[cell(a)][0] + 0.5) / (g[cell(a)][1] + 1) if cell(a) in g else a['pa'] for a in A])
        for qq, dst in ((q, lc), (pa, lt)):
            _, lo, hi = recal(qq[tr], y[tr]); p_ = np.clip(lo + (hi - lo) * qq[te], 1e-12, 1 - 1e-12)   # as recal() fits
            dst[te] = np.where(y[te] == 1, np.log(p_), np.log(1 - p_))
    runs_ = sorted(set(run.tolist())); ix = {r: i for i, r in enumerate(runs_)}
    per_c = np.zeros(len(runs_)); per_t = np.zeros(len(runs_))
    np.add.at(per_c, [ix[r] for r in run], lc); np.add.at(per_t, [ix[r] for r in run], lt)
    return common.compare_held_out(per_c, per_t), len(runs_)


# ---------------------------------------------------------------- claims
def bucket(v, bs):
    return next(lab for lo, lab in bs if v >= lo)


def jlab(j):
    return str(j) if j < 6 else '6+'


def by(A, grp, key):
    """cell -> calib(cell's attempts, key)"""
    g = collections.defaultdict(list)
    for a in A: g[grp(a)].append(a)
    return {c: calib(S, key) for c, S in sorted(g.items())}


def cv(c):
    return f'obs/pred {c[3]:.3f} [{c[4]:.3f}, {c[5]:.3f}], n {c[0]:,}'


def after_result(A):
    """{(j bucket, 'win' | 'loss'): calib} on spark-raised aptitudes, for attempts in a streak (j >= 2) by the result of the
    race the turn before"""
    out = {}
    for jb in ('2', '3', '4+'):
        for res in ('win', 'loss'):
            S = [a for a in A if a['j'] >= 2 and (str(a['j']) == jb or (jb == '4+' and a['j'] >= 4)) and a['last'] == (res == 'win')]
            out[(jb, res)] = calib(S, 'pa')
    return out


GRADE = {1: 'G', 2: 'F', 3: 'E', 4: 'D', 5: 'C', 6: 'B', 7: 'A', 8: 'A'}     # aptitude codes; S costs what A does


def grid(A, pen, minn=30, mincol=200):
    """the community table's layout from the runs: columns the (distance, surface) grade pair as a sorted pair of letters
    (spark-raised grades, S as A), rows the race's position in a run of consecutive races (1 to 6+); each cell the
    observed win rate, with the table's chance in parentheses; pairs with `mincol` or more attempts in all, cells under
    `minn` attempts show a dot. Returns the
    markdown table and the attempts table"""
    cell = collections.defaultdict(lambda: [0, 0])
    for a in A:
        pair = ''.join(sorted((GRADE[a['apt'][0]], GRADE[a['apt'][1]])))
        c = cell[(pair, min(a['j'], 6))]; c[0] += a['won']; c[1] += 1
    pairs = {}
    for (pair, j), (w, n) in cell.items(): pairs[pair] = pairs.get(pair, 0) + n
    cols = sorted((p for p, n in pairs.items() if n >= mincol), key=lambda p: (-(110 - COST_L[p[0]] - COST_L[p[1]]), p))
    rows = ['1 race', '2 races', '3 races', '4 races', '5 races', '6 or more']
    head = '| Races in a row | ' + ' | '.join(cols) + ' |\n|---|' + '---|' * len(cols) + '\n'
    body = cnt = ''
    for j in range(1, 7):
        line, cline = f'| {rows[j - 1]} |', f'| {rows[j - 1]} |'
        for p in cols:
            w, n = cell.get((p, j), (0, 0)); pred = min(100, max(0, 110 - COST_L[p[0]] - COST_L[p[1]] - pen[j - 1]))
            line += f' {100 * w / n:.0f}% ({pred}%) |' if n >= minn else f' · ({pred}%) |'
            cline += f' {n:,} |'
        body += line + '\n'; cnt += cline + '\n'
    return head + body.rstrip('\n'), head + cnt.rstrip('\n')


COST_L = {'A': 0, 'B': 10, 'C': 20, 'D': 30, 'E': 40, 'F': 60, 'G': 90}


def claims():
    R = common.runs()
    if R is None: return []
    A, skip, unk = attempts(R)
    base, pink = calib(A, 'p'), calib(A, 'pa')
    out = [Value(SEC, 'race attempts at the 34', len(A)), Value(SEC, 'race attempts won', sum(a['won'] for a in A)),
           Value(SEC, 'race attempts skipped', sum(skip.values())),
           Value(SEC, 'race goal attempts', sum(a['goal'] for a in A)),
           Value(SEC, 'race win rate observed', base[1]),
           Value(SEC, 'race win rate predicted, base', base[2]), Value(SEC, 'race win rate predicted, pink', pink[2])]
    for lab, c in (('base', base), ('pink', pink)):
        out += [Value(SEC, f'race obs/pred, {lab}', c[3]), Value(SEC, f'race obs/pred, {lab}, 95% low', c[4]),
                Value(SEC, f'race obs/pred, {lab}, 95% high', c[5])]
    b70 = by(A, lambda a: bucket(a['bp'], BPB), 'p')['<=70']
    out += [Value(SEC, 'race base bp <=70 attempts', b70[0]), Value(SEC, 'race base bp <=70 obs/pred', b70[3])]
    cells = {'bp': by(A, lambda a: bucket(a['bpa'], BPB), 'pa'), 'j': by(A, lambda a: jlab(a['j']), 'pa'),
             'scenario': by(A, lambda a: common.SCEN[a['scen']], 'pa')}
    for lab, cs in cells.items():
        for c, v in cs.items():
            out += [Value(SEC, f'race pink {lab} {c} attempts', v[0]), Value(SEC, f'race pink {lab} {c} obs/pred', v[3]),
                    Value(SEC, f'race pink {lab} {c} obs/pred 95% low', v[4]),
                    Value(SEC, f'race pink {lab} {c} obs/pred 95% high', v[5])]
    gj = {lab: calib([a for a in A if a['goal'] == (lab == 'goal') and a['j'] >= 4], 'pa') for lab in ('goal', 'chosen')}
    for lab, v in gj.items():
        out += [Value(SEC, f'race pink {lab} j>=4 attempts', v[0]), Value(SEC, f'race pink {lab} j>=4 obs/pred', v[3]),
                Value(SEC, f'race pink {lab} j>=4 obs/pred 95% low', v[4]),
                Value(SEC, f'race pink {lab} j>=4 obs/pred 95% high', v[5])]
    F = fits(A)
    L = {m: v[0] for m, v in F.items() if not m.startswith('_')}
    ncell = F['pink cells bp x j'][1]
    d = {'pink': L['pink table, recalibrated'] - L['table, recalibrated'],
         'pen': L['pink table, recalibrated'] - L['pink no penalty, recalibrated'],
         'shape': L['pink cells bp x j'] - L['pink table, recalibrated']}
    ins = 'in-sample, attempts treated as independent'
    out += [Value(SEC, 'race LL recalibrated pink table - recalibrated base table, nats', d['pink'], note=ins),
            Value(SEC, 'race LL recalibrated pink table - recalibrated pink no penalty, nats', d['pen'], note=ins),
            Value(SEC, 'race pink cells bp x j', ncell), Value(SEC, 'race LL pink cells bp x j: extra parameters', ncell - 2),
            Value(SEC, 'race LL pink cells bp x j - recalibrated pink table, nats', d['shape'], note=ins),
            Value(SEC, 'race LL threshold, pink cells bp x j', thr(ncell - 2), note=ins)]
    (ho_n, ho_se, ho_z), ho_runs = held_out_cells(A)
    out += [Value(SEC, 'held out: cells vs table: nats', ho_n, note=f'5-fold by run, {ho_runs:,} runs'),
            Value(SEC, 'held out: cells vs table: SE', ho_se), Value(SEC, 'held out: cells vs table: z', ho_z),
            Value(SEC, 'held out: cells vs table: runs', ho_runs)]

    # ---- findings: one per claim the section makes
    f = lambda *a: out.append(common.finding(SEC, *a))
    f('On base aptitudes (the solver\'s bp) the table under-predicts IT\'s wins: the obs/pred 95% interval lies '
      'entirely above 1', base[4] > 1, cv(base))
    f('On pink-adjusted aptitudes the table is calibrated overall: the obs/pred 95% interval covers 1',
      pink[4] <= 1 <= pink[5], cv(pink))
    f(f'Pink-adjusted aptitudes fit better than base ones: recalibrated pink table - recalibrated base table > '
      f'{thr(1):.2f} nats (same 2 parameters)', d['pink'] > thr(1), f'{d["pink"]:+,.1f} nats')
    f(f'The streak penalty is supported: the recalibrated pink table fits better with PEN than without (same 2 '
      f'parameters) by more than {thr(1):.2f} nats', d['pen'] > thr(1), f'{d["pen"]:+,.1f} nats')
    f(f'In-sample (attempts treated as independent), the table is not exact: a free rate per (pink bp, j) cell beats the recalibrated pink table by more than '
      f'chi2({ncell - 2}) 95% / 2 = {thr(ncell - 2):.2f} nats', d['shape'] > thr(ncell - 2),
      f'{d["shape"]:+,.1f} nats ({ncell} cells)')
    f('The per-cell rates do not predict unseen runs better than the recalibrated pink table (5-fold held out by run, '
      'z <= 1.96)', ho_z <= 1.96, f'{ho_n:+.1f} +- {ho_se:.1f} nats, z = {ho_z:.2f} on {ho_runs:,} runs')
    f('The excess sits in goal races from the 4th in a row: their obs/pred 95% interval lies entirely above 1, while '
      'chosen races from the 4th in a row cover 1', gj['goal'][4] > 1 and gj['chosen'][4] <= 1 <= gj['chosen'][5],
      f'goal {cv(gj["goal"])}; chosen {cv(gj["chosen"])}')
    lo70 = cells['bp']['<=70']
    f('Low chances win a little less than predicted: pink-adjusted bp <= 70 has an obs/pred 95% interval entirely '
      'below 1', lo70[5] < 1, cv(lo70))
    sc = cells['scenario']
    f('Pink-adjusted, the table is calibrated in every scenario: each obs/pred 95% interval covers 1',
      bool(sc) and all(v[4] <= 1 <= v[5] for v in sc.values()), '; '.join(f'{c}: {cv(v)}' for c, v in sc.items()))
    g_tab, g_n = grid(A, common.gjson('data.json')['pen'])
    out += [Value(SEC, 'race grid by aptitude and streak', g_tab), Value(SEC, 'race grid attempts', g_n)]
    # when a mid-run pink takes effect: the records say which year's inspiration fired it; the rule puts it at that year's
    # April (turns 30, 54). Senior January-March attempts where counting it from the year's first turn would change the grade;
    # the observed rate's 95% interval is run-clustered (common.cluster_ci with one expected per attempt)
    S_ = [a for a in A if 48 <= a['t'] < 54 and a['pe'] != a['pa']]
    tw = sum(a['won'] for a in S_)
    tlo, thi = common.cluster_ci([a['won'] for a in S_], [1] * len(S_), [a['run'] for a in S_])[1:3] if S_ else (float('nan'),) * 2
    e_apr, e_start = (sum(a[k] for a in S_) / len(S_) if S_ else float('nan') for k in ('pa', 'pe'))
    out += [Value(SEC, 'proc timing: attempts', len(S_)), Value(SEC, 'proc timing: observed', tw / len(S_) if S_ else None),
            Value(SEC, 'proc timing: observed 95% low', tlo), Value(SEC, 'proc timing: observed 95% high', thi),
            Value(SEC, 'proc timing: predicted, from April', e_apr), Value(SEC, 'proc timing: predicted, from the year start', e_start)]
    f('A pink fired at an inspiration takes effect from that April, not from the start of the year: on Senior January-March '
      'attempts where the two timings differ, the April prediction lies inside the observed 95% interval and the year-start '
      'one outside', bool(S_) and tlo <= e_apr <= thi and not tlo <= e_start <= thi,
      f'{tw}/{len(S_)} won [{tlo:.3f}, {thi:.3f}]; April {e_apr:.3f}, year start {e_start:.3f}')
    ar = after_result(A)
    for (jb, res), c in ar.items():
        out += [Value(SEC, f'race in a streak, j {jb}, after a {res}: attempts', c[0]), Value(SEC, f'race in a streak, j {jb}, after a {res}: obs/pred', c[3]),
                Value(SEC, f'race in a streak, j {jb}, after a {res}: obs/pred 95% low', c[4]), Value(SEC, f'race in a streak, j {jb}, after a {res}: obs/pred 95% high', c[5])]
    f('Within a streak the previous race\'s result does not change the next chance beyond the table: at the 2nd and 3rd race '
      'in a row, attempts after a win and after a loss each have an obs/pred 95% interval covering 1',
      all(ar[(jb, r_)][4] <= 1 <= ar[(jb, r_)][5] for jb in ('2', '3') for r_ in ('win', 'loss')),
      '; '.join(f'j {jb} after a {r_}: {cv(ar[(jb, r_)])}' for jb in ('2', '3', '4+') for r_ in ('win', 'loss')))
    f('From the 4th race in a row, attempts after a win and after a loss both win more than the table predicts (each 95% '
      'interval entirely above 1)', all(ar[('4+', r_)][4] > 1 for r_ in ('win', 'loss')),
      '; '.join(f'after a {r_}: {cv(ar[("4+", r_)])}' for r_ in ('win', 'loss')))
    return out


def detail():
    """the numbers behind the findings that the section does not state"""
    R = common.runs()
    A, skip, unk = attempts(R)
    print(f'aliases applied: {ALIAS}')
    print(f'skipped attempts: {dict(skip) or "none"}; G1 names unmatched after aliases: {dict(unk) or "none"}')
    db = common.mdb()
    if db is not None:
        U = {u['id']: u['apt'][:6] for u in common.gjson('data.json')['umas']}
        q = ('select proper_distance_short, proper_distance_mile, proper_distance_middle, proper_distance_long, '
             'proper_ground_turf, proper_ground_dirt from card_rarity_data where card_id = ? order by rarity limit 1')
        diff = {o for o in {a['outfit'] for a in A} if (row := db.execute(q, (o,)).fetchone()) and list(row) != U[o // 100]}
        print(f'attempts by outfits whose aptitudes differ from data.json (first outfit): '
              f'{sum(a["outfit"] in diff for a in A):,} ({len(diff)} outfits); scored with data.json\'s aptitudes')
    for key, lab in (('p', 'base'), ('pa', 'pink-adjusted')):
        cert = [a for a in A if a[key] == 1]; zero = [a for a in A if a[key] == 0]
        print(f'{lab}: {sum(1 - a["won"] for a in cert):,} losses in {len(cert):,} attempts at 100%; '
              f'{sum(a["won"] for a in zero):,} wins in {len(zero):,} at 0%')
    print('\nobs/pred [95%] by cell, base | pink-adjusted:')
    for lab, grp in (('bp', lambda k: (lambda a: bucket(a['bp' if k == 'p' else 'bpa'], BPB))),
                     ('j', lambda k: (lambda a: jlab(a['j']))), ('predicted', lambda k: (lambda a: bucket(a[k], PB))),
                     ('kind', lambda k: (lambda a: 'goal' if a['goal'] else 'chosen')),
                     ('scenario', lambda k: (lambda a: common.SCEN[a['scen']]))):
        cb, cp = by(A, grp('p'), 'p'), by(A, grp('pa'), 'pa')
        for c in sorted(set(cb) | set(cp)):
            s = lambda v: f'{v[3]:.3f} [{v[4]:.3f}, {v[5]:.3f}] n {v[0]:<6,}' if v else '-'
            print(f'  {lab:9s} {c:14s} {s(cb.get(c)):34s} | {s(cp.get(c))}')
    for lab, S in (('bp moved by the pink rule', [a for a in A if a['bpa'] != a['bp']]),
                   ('after an April pink firing of the race\'s aptitudes', [a for a in A if a['fired']])):
        print(f'  {lab}: base {cv(calib(S, "p"))}; pink-adjusted {cv(calib(S, "pa"))}')
    for key, bpk, lab in (('p', 'bp', 'base aptitudes'), ('pa', 'bpa', 'pink-adjusted')):
        print(f'\n{lab}: wins/attempts, mean predicted')
        print(f'{"bp \\ j":>7s}' + ''.join(f'{j:>18s}' for j in ('1', '2', '3', '4', '5', '6+')))
        for _, b in BPB:
            row = []
            for j in range(1, 7):
                S = [a for a in A if bucket(a[bpk], BPB) == b and min(a['j'], 6) == j]
                row.append(f'{sum(a["won"] for a in S):>6d}/{len(S):<6d}{(sum(a[key] for a in S) / len(S) if S else 0):5.2f}')
            print(f'{b:>7s}' + ''.join(f'{x:>18s}' for x in row))
    F = fits(A)
    print('\nlog-likelihoods (nats, parameters):')
    for m, v in F.items():
        if not m.startswith('_'): print(f'  {m:32s} {v[0]:12.2f}  {v[1]}')
    print('recalibration lo, hi:', {k: (round(float(a), 4), round(float(b), 5)) for k, (a, b) in F['_recal'].items()})
    # a hypothesis the goal cells suggest (not a claim): the goal race scored one streak position earlier (j - 1)
    print('\ngoal race scored at j - 1 (a lead from the goal cells, not a claim):')
    for lab, S in (('all', A), ('earlier half', [a for a in A if not a['later']]), ('later half', [a for a in A if a['later']])):
        y = np.array([a['won'] for a in S])
        dl = recal(np.array([a['pg'] for a in S]), y)[0] - recal(np.array([a['pa'] for a in S]), y)[0]
        print(f'  {lab}: LL(shifted, recalibrated) - LL(pink table, recalibrated) {dl:+.1f} nats')
    Lc, ncg = saturated(A, lambda a: (a['bpa'], min(a['j'], 6), a['goal']))
    g = Lc - recal(np.array([a['pg'] for a in A]), np.array([a['won'] for a in A]))[0]
    print(f'  rate per (pink bp, j, goal) cell over the shifted table: {g:+.1f} nats, {ncg} cells '
          f'(chi2({ncg - 2}) 95% / 2 = {thr(ncg - 2):.2f}); losses where the shift gives 100%: '
          f'{sum(1 - a["won"] for a in A if a["pg"] == 1)}')


if __name__ == '__main__':
    common.main(claims)
    if common.have_runs(): print(); detail()
