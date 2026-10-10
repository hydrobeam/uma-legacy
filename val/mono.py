"""Monotonicity tripwire for the career planner: a veteran added to a pool never raises V (every plan without her is
still there). For each val/plan job: plan the pool less its last veteran, then the full pool under that plan's fit
(`next.fit`, as the page's Compare prices it), and report V without -> V with and the route steps. A rise is the model's
error, not hers (the fixed fit rules out the continuation's refit; what remains is the route's fixed lineages: each fresh
8/8 bred from her uma's best pair now).

usage: mono.py BIN [JOB ...] [-j N]   (default: every job in val/plan/jobs.json; -j jobs at a time, default 2)
  writes nothing; exits 1 when a V rises by more than 1e-9 relative"""
import json, os, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor
V = os.path.dirname(os.path.abspath(__file__)); S = os.path.dirname(V); P = os.path.join(V, 'plan')
args = sys.argv[1:]
par = 2
if '-j' in args: i = args.index('-j'); par = int(args[i + 1]); del args[i:i + 2]
b = os.path.abspath(args[0]); want = set(args[1:])
jobs = [(d, n) for d, n in json.load(open(os.path.join(P, 'jobs.json'))) if not want or n in want]
tmp = tempfile.mkdtemp(prefix='mono_')

def plan(d, job, name):
    path = os.path.join(tmp, name + '.json'); json.dump(job, open(path, 'w'))
    r = subprocess.run([b, 'data.json', path], cwd=os.path.join(S, 'pkg', 'loopopt', 'data', d), capture_output=True, text=True)
    return json.loads(r.stdout)['next']

def one(dj):
    d, n = dj
    job = json.load(open(os.path.join(P, n + '.json')))
    if len(job.get('legacies', [])) < 3: return n, None
    less = dict(job, legacies=job['legacies'][:-1], next={**job.get('next', {})})
    a = plan(d, less, n + '_less')
    if a.get('v0') is None or not a.get('fit'): return n, ('no plan without her', a.get('error'))
    b2 = plan(d, dict(job, next={**job.get('next', {}), 'fit': a['fit']}), n + '_with')
    return n, (a, b2)

bad = 0
with ThreadPoolExecutor(par) as ex: res = list(ex.map(one, jobs))
for n, r in res:
    if r is None: print(f'{n}: under 3 veterans, skipped'); continue
    if isinstance(r[0], str): print(f'{n}: {r[0]} {r[1] or ""}'); continue
    a, w = r
    va, vw = a['v0'], w['v0']
    step = lambda P: ' / '.join(f'{P["v0_plan"][k]} {c:,.1f}' for k, c in zip('abc', P['v0_plan']['careers'])) if P.get('v0_plan') else '?'
    rise = vw is not None and vw > va * (1 + 1e-9)
    bad += rise
    print(f'{n}: V {va:,.2f} -> {vw:,.2f}  ({"RISES " if rise else ""}{vw - va:+,.2f})\n    without: {step(a)}\n    with:    {step(w)}')
print(f'{len(res)} pools, {bad} rise')
sys.exit(1 if bad else 0)
