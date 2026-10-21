"""Whole-output identity (docs/PERF.md, *What "byte-identical" covers*): uv run val/same.py TAG [REF]
Compares val/TAG__*.out with val/REF__*.out (default ref) and val/TAGpu__*.out with val/REFpu__*.out, every field but
the timings (`timing_s`, `*_ms`); prints each differing output with the paths that differ. Exit 1 on any difference or
when nothing was compared. Stricter than cmp2.py, which checks rounded P(all)."""
import glob, json, os, sys

V = os.path.dirname(os.path.abspath(__file__))
tag = sys.argv[1]; ref = sys.argv[2] if len(sys.argv) > 2 else 'ref'

def load(p):
    t = open(p).read()
    return json.loads(t[t.index('{'):])

def paths(a, b, at=''):   # the paths where a and b differ, timings left out
    timing = lambda k: k == 'timing_s' or k.endswith('_ms')
    if isinstance(a, dict) and isinstance(b, dict):
        return [p for k in sorted(set(a) | set(b)) if not timing(k) for p in paths(a.get(k), b.get(k), f'{at}.{k}')]
    if isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
        return [p for i, (x, y) in enumerate(zip(a, b)) for p in paths(x, y, f'{at}[{i}]')]
    return [] if a == b else [at or '.']

n = bad = 0
for t, r in ((tag, ref), (tag + 'pu', ref + 'pu')):
    for nf in sorted(glob.glob(os.path.join(V, f'{t}__*.out'))):
        rf = os.path.join(V, f'{r}__' + nf.split(f'{t}__', 1)[1])
        if not os.path.exists(rf): continue
        n += 1; d = paths(load(rf), load(nf))
        if d: bad += 1; print(f'{os.path.basename(nf)}: {len(d)} differ: ' + ', '.join(d[:6]) + (' …' if len(d) > 6 else ''))
print(f'{n} compared, {bad} differ')
sys.exit(1 if bad or not n else 0)
