"""Summaries of policy-simulator outputs (`simulate` career jobs; docs/SOLVER.md "Policy simulator").

    uv run val/sim/report.py runs A.out [B.out ...]      per policy: E mean, sd, 95% CI, careers, final p, wall time;
                                                          paired differences between policies over common run indices
    uv run val/sim/report.py e1 A.out [B.out ...]        surrogate fidelity from the `check` records
    uv run val/sim/report.py audit A.out [B.out ...]     surrogate fidelity from the `audit` records

An output file is the solver's stdout for one job (one JSON line); several files of one policy (split runs) pool.
"""
import json, math, sys
from collections import defaultdict


def tq975(df):
    z = 1.959964
    return z + (z**3 + z) / (4 * df) + (5 * z**5 + 16 * z**3 + 3 * z) / (96 * df**2) + (3 * z**7 + 19 * z**5 + 17 * z**3 - 15 * z) / (384 * df**3)


def st(xs):
    n = len(xs); m = sum(xs) / n
    sd = math.sqrt(sum((x - m) ** 2 for x in xs) / (n - 1)) if n > 1 else 0.0
    h = tq975(n - 1) * sd / math.sqrt(n) if n > 1 else 0.0
    return n, m, sd, h


def load(paths):
    by = defaultdict(dict)
    for p in paths:
        for line in open(p):
            line = line.strip()
            if not line:
                continue
            o = json.loads(line)
            if 'runs' not in o:
                print(f'{p}: no runs ({str(o)[:200]})'); continue
            name = o['policy']
            sim = o['simulate']
            pool0 = o['runs'][0]['segs'][0]['pool'] if o['runs'] and o['runs'][0]['segs'] else '?'
            tag = f'[{pool0} vets] ' + name + ''.join(f' {k}={sim[k]}' for k in ('margin', 'samples', 'redecide', 'cont_eps', 'loop_members') if k in sim and k in o['simulate'])
            for r in o['runs']:
                by[tag][r['run']] = r
    return by


def runs(paths):
    by = load(paths)
    for tag, rs in by.items():
        E = [r['E'] for r in rs.values()]
        n, m, sd, h = st(E)
        c = st([r['careers'] for r in rs.values()])
        fp = st([r['final_p'] for r in rs.values()])
        w = st([r['wall_s'] for r in rs.values()])
        tail = st([r['tail'] for r in rs.values()])
        print(f'{tag}: n {n}  E {m:.1f}  sd {sd:.1f}  95% CI {m - h:.1f} to {m + h:.1f}  careers {c[1]:.0f}  final p {fp[1]:.3e} (1/{1 / fp[1]:.0f})  '
              f'tail {tail[1]:.2f}  wall {w[1]:.1f} s/run')
    tags = list(by)
    for i in range(len(tags)):
        for j in range(i + 1, len(tags)):
            if tags[i].split(']')[0] != tags[j].split(']')[0]:
                continue   # different pools
            a, b = by[tags[i]], by[tags[j]]
            common = sorted(set(a) & set(b))
            if len(common) < 2:
                continue
            d = [a[k]['E'] - b[k]['E'] for k in common]
            n, m, sd, h = st(d)
            lo = sum(x < 0 for x in d); hi = sum(x > 0 for x in d)
            print(f'  {tags[i]} - {tags[j]}: paired n {n}  {m:+.1f}  95% CI {m - h:+.1f} to {m + h:+.1f}  sd {sd:.1f}  (first lower in {lo}, higher in {hi})')


def e1(paths):
    by = load(paths)
    for tag, rs in by.items():
        am = [c['argmax'] for r in rs.values() for c in r['checks']]
        diff = [a for a in am if a['ex'] != a['sur']]
        rel = [(a['ex_p'] - a['sur_p']) / a['ex_p'] for a in am if a['ex_p']]
        print(f'{tag}: {len(am)} argmax checks over {len(rs)} runs (pools {min(a["pool"] for a in am)}-{max(a["pool"] for a in am)}): '
              f'surrogate career differs in {len(diff)}, max relative P(all) shortfall {max(rel):.2e}, unproven exact {sum(not a["proven"] for a in am)}')
        for a in diff[:5]:
            print(f'    differs: pool {a["pool"]} sur {a["sur"]} {a["sur_p"]:.6e} ex {a["ex"]} {a["ex_p"]:.6e}')
        L = [x + [cc['argmax']['sur_val'] or 0] for r in rs.values() for cc in r['checks'] for x in cc['lineages']]
        # x = [ub, rb, tv, exact, rel err, trusted, tvd, full, pick, B]
        has = [x for x in L if x[2] > 0]
        rel_ = sorted(x[4] for x in has)
        q = lambda v, p: v[min(len(v) - 1, int(p * (len(v) - 1)))] if v else float('nan')
        print(f'  lineages {len(L)}: trainee without a cached deck {len(L) - len(has)}; with one: relative error (exact - table)/exact '
              f'median {q(rel_, .5):.1e}, 99% {q(rel_, .99):.1e}, max {q(rel_, 1):.1e}; over 1e-9: {sum(e > 1e-9 for e in rel_)}')
        cont = [x for x in has if x[3] >= 0.5 * x[9]]
        print(f'  contenders (exact >= B/2) {len(cont)}: over 1e-9 {sum(x[4] > 1e-9 for x in cont)}, max {max((x[4] for x in cont), default=0):.1e}; '
              f'landed-set total variation max {max((x[6] for x in has), default=0):.1e}; bound rb / exact median {q(sorted(x[1] / x[3] for x in L), .5):.3f}, '
              f'ceiling / exact median {q(sorted(x[0] / x[3] for x in L), .5):.3f}; picks checked {sum(x[8] for x in L)}')
        bad = sorted([x for x in has if x[4] > 1e-9], key=lambda x: -x[4])[:5]
        for x in bad:
            print(f'    table short: exact {x[3]:.4e} table {x[2]:.4e} ({x[4]:.2%}), B {x[9]:.4e}, trusted {x[5]}, tvd {x[6]:.3f}')


KIND = {0: 'greedy pick', 1: 'random candidate', 2: 'rollout candidate', 3: 'loop career'}


def audit(paths):
    """the `audit` records: per kind of row, the table value against a fresh exact deck search"""
    by = load(paths)
    for tag, rs in by.items():
        A = [a for r in rs.values() for a in r.get('audits', [])]
        if not A:
            continue
        rows = [x for a in A for x in a.get('rows', [])]
        am = [a for a in A if 'differs' in a]
        rot = [a['rotation'] for a in A if 'rotation' in a]
        print(f'{tag}: {len(rs)} runs, {len(A)} audit points, {len(rows)} rows')
        if am:
            print(f'  greedy argmax vs exact career search: {len(am)} pick changes, differs in {sum(a["differs"] for a in am)}'
                  f' (max relative shortfall {max((a["ex_p"] - a["sur_p"]) / a["ex_p"] for a in am if a["ex_p"]):.1e}); pools {min(a["pool"] for a in am)}-{max(a["pool"] for a in am)}')
            for a in [a for a in am if a['differs']][:5]:
                print(f'    differs at t {a["t"]} pool {a["pool"]}: sur {a["sur"]} {a["sur_p"]:.6e} exact {a["ex"]} {a["ex_p"]:.6e}')
        for k in sorted({x[0] for x in rows}):
            R = [x for x in rows if x[0] == k]
            nodeck = [x for x in R if x[1] == 0]
            has = [x for x in R if x[1] > 0]
            e = sorted(x[3] for x in has)
            q = lambda v, p: v[min(len(v) - 1, int(p * (len(v) - 1)))] if v else float('nan')
            print(f'  {KIND.get(k, k)}: {len(R)} rows, no cached deck {len(nodeck)}; table vs exact relative error median {q(e, .5):.1e}, 99% {q(e, .99):.1e}, '
                  f'max {q(e, 1):.1e}, over 1e-9 {sum(v > 1e-9 for v in e)}; landed-set TV max {max((x[4] for x in has), default=0):.1e}; unfinished searches {sum(not x[5] for x in R)}')
        if rot:
            print(f'  loop keep-or-replace: {len(rot)} audited, same decision under exact values {sum(r[4] for r in rot)}; max relative score error '
                  f'{max(max(abs(r[0] - r[2]) / r[2] if r[2] else 0, abs(r[1] - r[3]) / r[3] if r[3] else 0) for r in rot):.1e}')


if __name__ == '__main__':
    if sys.argv[1] == 'e1':
        e1(sys.argv[2:])
    elif sys.argv[1] == 'audit':
        audit(sys.argv[2:])
    else:
        runs(sys.argv[2:] if sys.argv[1] == 'runs' else sys.argv[1:])
