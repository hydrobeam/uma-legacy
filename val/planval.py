"""The career planner's next-plan jobs (val/plan/jobs.json): run them, then compare each output with the reference
strictly (the whole JSON, equal values; only timings and search-work counters are ignored). For solver work that must
not move the planner's results; `val/runval.py` does not cover these jobs.

usage: planval.py BIN TAG [REF] [-j N]
  writes val/plan/TAG__NAME.out and val/plan/TAG_times.txt; compares with val/plan/REF__NAME.out (default REF: ref)
  -j N  jobs at a time (default 2: the jobs take 2-8 s each; timings are only comparable at the same load)"""
import json, os, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
V = os.path.dirname(os.path.abspath(__file__)); S = os.path.dirname(V); P = os.path.join(V, 'plan')
args = sys.argv[1:]
par = 2
if '-j' in args: i = args.index('-j'); par = int(args[i + 1]); del args[i:i + 2]
b, tag = os.path.abspath(args[0]), args[1]
ref = args[2] if len(args) > 2 else 'ref'
jobs = json.load(open(os.path.join(P, 'jobs.json')))
times = {}
def run(dj):
    era, n = dj; t0 = time.time()
    r = subprocess.run([b, 'data.json', os.path.join(P, n + '.json')], cwd=os.path.join(S, 'pkg', 'loopopt', 'data', era), capture_output=True, text=True)
    if r.returncode:
        raise RuntimeError(f'{n}: solver exited {r.returncode}\n{r.stderr}')
    open(os.path.join(P, f'{tag}__{n}.out'), 'w').write(r.stdout)
    times[n] = round(time.time() - t0, 2)
with ThreadPoolExecutor(par) as ex: list(ex.map(run, jobs))
open(os.path.join(P, f'{tag}_times.txt'), 'w').write(''.join(f'{n} {s}\n' for n, s in sorted(times.items())))
# search-work counters and timings: they move with any pruning change and say nothing about the answer
SKIP = {'timing_s', 'work', 'bounded', 'solved', 'scored', 'skipped', 'next_bound', 'searches', 'deck_searches', 'decks_scored', 'tables', 'nodes'}
def walk(a, b, p, out):
    if len(out) >= 8: return
    if type(a) != type(b): out.append(f'{p}: type {type(a).__name__} vs {type(b).__name__}'); return
    if isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k in SKIP or k.endswith('_ms'): continue
            if k not in a or k not in b: out.append(f'{p}.{k}: only in {"new" if k in b else "ref"}'); continue
            walk(a[k], b[k], f'{p}.{k}', out)
    elif isinstance(a, list):
        if len(a) != len(b): out.append(f'{p}: length {len(a)} vs {len(b)}'); return
        for i, (x, y) in enumerate(zip(a, b)): walk(x, y, f'{p}[{i}]', out)
    elif a != b: out.append(f'{p}: {str(a)[:60]} vs {str(b)[:60]}')
# the deck structure classes of the exact caps (`cap_class`: chain-varying targets; a multi-target group counts as 4+)
CLS = ['0-1', '2', '3', '4+']
def near_cap(r, c):
    # a scored deck's exact / product r is near its class's assumed cap c: past three quarters of the cap's headroom
    # over 1 (1.1875 of 1.25, about the old 1.25 / 1.05; 1.0375 of 1.05).
    # The same rule as val/cmp2.py.
    return r > 1 + 0.75 * (c - 1)
bad = 0
for d, n in jobs:
    rf, nf = os.path.join(P, f'{ref}__{n}.out'), os.path.join(P, f'{tag}__{n}.out')
    if not os.path.exists(rf): print(f'{n}: no reference ({ref})'); bad += 1; continue
    try: a, x = json.load(open(rf)), json.load(open(nf))
    except Exception as e: print(f'{n}: PARSE FAIL {e}'); bad += 1; continue
    out = []; walk(a, x, '', out)
    rt = dict(l.split() for l in open(os.path.join(P, f'{ref}_times.txt'))) if os.path.exists(os.path.join(P, f'{ref}_times.txt')) else {}
    # the plan's own work (deck-search nodes, exact scorings): deterministic, so comparable at any machine load
    wk = lambda j: (j.get('next') or {}).get('work') or {}
    wa, wx = wk(a), wk(x)
    work = ' '.join(f'{k} {wa.get(k, "?"):,} -> {wx.get(k, "?"):,}' if isinstance(wa.get(k), int) and isinstance(wx.get(k), int) else f'{k} {wa.get(k, "?")} -> {wx.get(k, "?")}' for k in ('nodes', 'exact'))
    # the assumed caps, per structure class: no scored deck's exact / product may come near its class's cap (a deck
    # above it would be missed unseen)
    mr, caps = wx.get('class_max_ratio'), wx.get('class_caps')
    if not isinstance(mr, list): mr, caps = [wx.get('max_ratio')], [wx.get('cap')]
    num = lambda x: isinstance(x, (int, float))
    near = [(k, r, c) for k, (r, c) in enumerate(zip(mr, caps)) if num(r) and num(c) and near_cap(r, c)]
    print(f'{n}: {"identical" if not out else "DIFFERS"}  {rt.get(n, "?")} s -> {times[n]} s  {work}' + (f'  max ratio/cap by class {" ".join(f"{r:.3f}/{c}" for r, c in zip(mr, caps))}' if all(num(r) for r in mr) else ''))
    for k, r, c in near: print(f'    CAP: a scored deck of class {CLS[k] if len(mr) == 4 else "any"} reached exact/product {r:.4f}, near its assumed cap {c}')
    for o in out: print('   ', o)
    bad += bool(out) or bool(near)
print(f'{len(jobs)} plan jobs, {bad} differ')
sys.exit(1 if bad or not jobs else 0)
