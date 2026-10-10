"""Run every section script and print one table: claim, stated, reproduced, verdict.

  uv run --group research research/run_all.py [--quick]     (or ./build.sh research)

--quick skips the section 12 variant over every other skill (a solver run of several minutes the first time; its
results are reused while nothing it reads changes). Without the run records (research/data/runs.jsonl, not included)
the data claims read "needs data" and the derived ones still run."""
import collections, glob, importlib, inspect, os, sys, traceback
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
from common import Claim


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
            allc.append(Claim(name[1:3].lstrip('0'), f'{name} failed: {type(e).__name__}', '-', status='error'))
    common.table(allc)
    n = collections.Counter(c.verdict() for c in allc)
    print('\n' + ', '.join(f'{v}: {k}' for k, v in sorted(n.items(), key=lambda x: -x[1])))


if __name__ == '__main__': main()
