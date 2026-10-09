"""Audit of the exact caps per deck structure class (`EXACT_CAPS`, docs/SOLVER.md): run jobs normally and with the job
field `cap_audit` (every class cap at 2.0, so the deck searches keep far more decks and score them exactly), then
compare. Prints, per class, the largest exact / product odds the audit run scored (and the deck), flags a class whose
largest ratio passes its normal cap, and fails when the answer moves (a deck the normal caps missed).

usage: capaudit.py BIN [JOB ...]
  JOB: a val/plan job name (p16, gm16, ...) or a val job as DIR/FILE with ':pu' for per-uma (jp/jobs/v_uc_k1.json:pu).
  Default: p16, gm16, jp/jobs/v_uc_k1.json:pu (about 35 s natively, one job at a time; the audit runs take most of
  it). v_uc_k1's per-uma audit run lists over 500,000 decks at 2.0 and falls to 1.12: there it checks the 1.05 classes
  (0-1 and 2) only.
  Exits 1 when an answer moves or a ratio passes its cap."""
import json, os, subprocess, sys, tempfile, time
V = os.path.dirname(os.path.abspath(__file__)); S = os.path.dirname(V); P = os.path.join(V, 'plan')
CLS = ['0-1', '2', '3', '4+']
args = sys.argv[1:]
b = os.path.abspath(args[0]); want = args[1:] or ['p16', 'gm16', 'jp/jobs/v_uc_k1.json:pu']
plan_dirs = dict((n, d) for d, n in json.load(open(os.path.join(P, 'jobs.json'))))
tmp = tempfile.mkdtemp(prefix='capaudit_')

def run(d, job, name, env=None):
    path = os.path.join(tmp, name + '.json'); json.dump(job, open(path, 'w'))
    t0 = time.time()
    r = subprocess.run([b, 'data.json', path], cwd=os.path.join(S, d), capture_output=True, text=True, env=env)
    t = r.stdout
    return json.loads(t[t.index('{'):]), time.time() - t0

# what the answer is: everything but timings, search-work counters, the cap fields and the ceilings that scale with the
# top cap (`bound`: a career's ceiling; the reroll table's `y_hi` and `provisional`; the plan's certification counts)
SKIP = {'bound', 'y_hi', 'provisional', 'umas_searched', 'umas_certified', 'umas_scored', 'timing_s', 'work', 'bounded', 'solved', 'scored', 'skipped', 'next_bound', 'searches', 'deck_searches', 'decks_scored',
        'tables', 'nodes', 'exact', 'cap', 'class_caps', 'complete', 'proven', 'proven_top', 'foursomes_ranked', 'foursomes_scanned'}
def walk(a, c, p, out):
    if len(out) >= 6: return
    if type(a) != type(c): out.append(f'{p}: type'); return
    if isinstance(a, dict):
        for k in sorted(set(a) | set(c)):
            if k in SKIP or k.endswith('_ms'): continue
            if k not in a or k not in c: out.append(f'{p}.{k}: only in one'); continue
            walk(a[k], c[k], f'{p}.{k}', out)
    elif isinstance(a, list):
        if len(a) != len(c): out.append(f'{p}: length {len(a)} vs {len(c)}'); return
        for i, (x, y) in enumerate(zip(a, c)): walk(x, y, f'{p}[{i}]', out)
    elif a != c: out.append(f'{p}: {str(a)[:50]} vs {str(c)[:50]}')

bad = 0; t_all = time.time()
for w in want:
    if w in plan_dirs: d, job, name = plan_dirs[w], json.load(open(os.path.join(P, w + '.json'))), w
    else:
        f, _, mode = w.partition(':'); d, jf = f.split('/', 1)
        job = json.load(open(os.path.join(S, d, jf))); name = os.path.basename(jf).replace('.json', '')
        if mode == 'pu': job['per_uma'] = True; name += '_pu'
    a, ta = run(d, job, name)
    # (a per-uma audit run gets ten times the work budget, as `cap_audit` gives its bands ten times the room: at caps of
    # 2.0 the defaults run out and the pass falls down the ladder; the career planner's own budget is ample)
    env = None if w in plan_dirs else dict(os.environ, EXACT_BUDGET=str(300_000_000))
    c, tc = run(d, dict(job, cap_audit=True), name + '_audit', env)
    out = []; walk(a, c, '', out)
    # the certificate and the per-class ratios: per-uma jobs in `exact`, plan jobs in `next.work`
    cx = c.get('exact') or ((c.get('next') or {}).get('work')) or {}
    ax = a.get('exact') or ((a.get('next') or {}).get('work')) or {}
    caps = ax.get('class_caps') or [None] * 4
    rho = (cx.get('rho'), cx.get('rho_target')) if 'rho' in cx else (min(cx.get('cap') or 0, c.get('cap') or 0), 2.0)
    print(f'{name}: answer {"same" if not out else "MOVES"}  normal {ta:.1f} s, audit {tc:.1f} s (audit certified at {rho[0]} of {rho[1]})')
    for o in out: print('    ', o)
    bad += bool(out)
    if rho[0] is not None and rho[1] is not None and rho[0] < rho[1]: print(f'     (the audit run fell down the cap ladder to {rho[0]}: a class whose normal cap is at least that is not checked)')
    if c.get('complete') is False or (c.get('exact') or {}).get('complete') is False: print('     (the audit run is not complete: it checks less than it claims)')
    for k in range(4):
        r = (cx.get('class_max_ratio') or [0] * 4)[k]; dk = (cx.get('class_max_deck') or [None] * 4)[k]
        over = caps[k] is not None and r > caps[k] * (1 + 1e-9)
        bad += over
        who = f"  {dk['trainee']}: {', '.join(dk['cards'])}" if dk and (over or r > 1.0) else ''
        unchecked = rho[0] is not None and caps[k] is not None and rho[0] <= caps[k]
        print(f'    class {CLS[k]:>3}: max exact/product {r:.4f}, normal cap {caps[k]}{" (not checked)" if unchecked else ""}{"  OVER THE CAP" if over else ""}{who}')
print(f'{len(want)} jobs in {time.time() - t_all:.0f} s, {bad} problems')
sys.exit(1 if bad else 0)
