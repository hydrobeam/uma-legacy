"""Run every section script: print each computed value and each tested conclusion, and write research/results.json.

  uv run --group research research/run_all.py [--quick]     (or ./build.sh research)

--quick skips the section 12 variant over every other skill (a solver run of several minutes the first time; its
results are reused while nothing it reads changes). The computed values go to research/results.json, from which
research/render.py renders docs/RESEARCH.md. Without the run records (research/data/runs.jsonl, not included)
the data claims read "needs data" and the derived ones still run."""
import collections, glob, importlib, inspect, json, os, re, sys, traceback
for v in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'): os.environ.setdefault(v, '1')   # one core
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
from common import Value, Finding


def main():
    quick = '--quick' in sys.argv
    if not common.have_runs(): common.say_missing()
    allc = []
    for path in sorted(glob.glob(os.path.join(common.HERE, 's[0-9][0-9]_*.py'))):
        name = os.path.basename(path)[:-3]
        try:
            m = importlib.import_module(name)
            kw = {'wide': not quick} if 'wide' in inspect.signature(m.claims).parameters else {}
            allc += m.claims(**kw)
        except Exception as e:
            traceback.print_exc()
            allc.append(Value(name[1:3].lstrip('0'), f'{name} failed: {type(e).__name__}', status='error'))
    common.table(allc)
    write_results(allc, fresh=common.have_runs() and not quick)
    vals = [c for c in allc if isinstance(c, Value)]; fs = [f for f in allc if isinstance(f, Finding)]
    n = collections.Counter('computed' if not c.missing() else (c.status or 'not computed') for c in vals)
    print('\nvalues: ' + ', '.join(f'{v}: {k}' for k, v in n.most_common()))
    n = collections.Counter(f.verdict() for f in fs)
    print('findings: ' + ', '.join(f'{v}: {k}' for k, v in n.most_common()))
    unused(vals)


def write_results(items, fresh):
    """research/results.json: every computed value by key, and the findings with their verdicts. A full run with the run
    records rebuilds it; otherwise (a clone, or --quick) what this run computes replaces the committed entries and the
    rest are kept, so the whole document still renders."""
    path = os.path.join(common.HERE, 'results.json')
    old = json.load(open(path)) if os.path.exists(path) and not fresh else {'values': {}, 'findings': []}
    values = dict(old['values']); seen = {}
    for c in items:
        if not isinstance(c, Value) or c.missing(): continue
        k = common.key_of(c.section, c.name)
        if k in seen and seen[k] != c.name: sys.exit(f'two values share the key {k}: {seen[k]!r} and {c.name!r}')
        seen[k] = c.name; values[k] = float(c.got)
    findings = {(f['section'], f['statement']): f for f in old['findings']}
    for f in items:                                       # a finding evaluated now replaces the committed one
        if isinstance(f, Finding) and f.holds is not None:
            findings[(f.section, f.statement)] = dict(section=f.section, statement=f.statement, verdict=f.verdict(), evidence=f.evidence)
        elif isinstance(f, Finding) and (f.section, f.statement) not in findings:
            findings[(f.section, f.statement)] = dict(section=f.section, statement=f.statement, verdict=f.verdict(), evidence=f.evidence)
    findings = list(findings.values())
    for sec in {f['section'] for f in findings}:
        mine = [f for f in findings if f['section'] == sec]
        values[f'{sec}.findings_total'] = len(mine); values[f'{sec}.findings_holding'] = sum(f['verdict'] == 'holds' for f in mine)
    json.dump({'values': dict(sorted(values.items())), 'findings': findings}, open(path, 'w'), indent=1, ensure_ascii=False)
    print(f'wrote research/results.json: {len(values)} values, {len(findings)} findings')


def unused(vals):
    """computed values the document template never uses: either missing from the document or not needed"""
    t = open(os.path.join(common.ROOT, 'docs', 'RESEARCH.template.md'), encoding='utf-8').read()
    used = set(re.findall(r'\{\{([0-9]+\.[a-z0-9_]+)\|', t))
    un = sorted({common.key_of(c.section, c.name) for c in vals if not c.missing()} - used)
    print(f'computed values the document does not use: {len(un)}' + (': ' + ', '.join(un[:40]) + (' ...' if len(un) > 40 else '') if un else ''))

if __name__ == '__main__': main()
