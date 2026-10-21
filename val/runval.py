"""Run the validation jobs in parallel: every job in val/jobs.json with its own settings (val/TAG__*.out) and every deck
job again with per_uma on (val/TAGpu__*.out), plus val/TAG_times.txt and val/TAGpu_times.txt (seconds per job).

usage: runval.py BIN TAG [--quick] [-j N]
  --quick  leave out the jobs in val/slow.json (the shared-deck three-result runs that take minutes; every code path
           they use is also exercised by the smaller three-result jobs, at small scale). For iterating on a change;
           releases and merges run the full set.
  -j N     jobs at a time (default: $VAL_J, else CPU count - 2; ./build.sh passes VAL_J through)."""
import json, os, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
V = os.path.dirname(os.path.abspath(__file__)); S = os.path.dirname(V)
args = sys.argv[1:]
quick = '--quick' in args; args = [a for a in args if a != '--quick']
par = int(os.environ.get('VAL_J', 0)) or max(1, (os.cpu_count() or 4) - 2)
if '-j' in args: i = args.index('-j'); par = int(args[i + 1]); del args[i:i + 2]
b, tag = os.path.abspath(args[0]), args[1]
slow = {tuple(x) for x in json.load(open(os.path.join(V, 'slow.json')))} if quick else set()
tasks = []   # (mode, name, era, job file); the solver runs in the era's data folder, as the job files expect
for era, j in json.load(open(os.path.join(V, 'jobs.json'))):
    name = era + '__' + j
    jf = os.path.join(V, 'jobs', j + '.json')
    job = json.load(open(jf))
    if ('', era, j) not in slow and ('sh', era, j) not in slow: tasks.append(('', name, era, jf))
    if job.get('mode') == 'deck' and ('pu', era, j) not in slow:
        job['per_uma'] = True
        tmp = os.path.join(V, f'_pu_tmp_{tag}__{name}.json'); json.dump(job, open(tmp, 'w'))
        tasks.append(('pu', name, era, tmp))
times = {'': {}, 'pu': {}}
def run(t):
    mode, name, era, jf = t; t0 = time.time()
    result = subprocess.run([b, 'data.json', jf], cwd=os.path.join(S, 'pkg', 'loopopt', 'data', era), capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(f'{mode or "shared"} {name}: solver exited {result.returncode}\n{result.stderr}')
    open(os.path.join(V, f'{tag}{mode}__{name}.out'), 'w').write(result.stdout)
    times[mode][name] = round(time.time() - t0, 1)
    if mode == 'pu': os.remove(jf)
t0 = time.time()
# longest first (reference times), so the slow jobs don't start last
ref = {m: dict(l.split() for l in open(os.path.join(V, f'ref{m}_times.txt'))) if os.path.exists(os.path.join(V, f'ref{m}_times.txt')) else {} for m in ('', 'pu')}
with ThreadPoolExecutor(par) as ex: list(ex.map(run, sorted(tasks, key=lambda t: -float(ref[t[0]].get(t[1], 0)))))
for mode in ('', 'pu'):
    open(os.path.join(V, f'{tag}{mode}_times.txt'), 'w').write(''.join(f'{n} {s}\n' for n, s in sorted(times[mode].items())))
print(f'{len(tasks)} jobs{" (quick: " + str(len(slow)) + " slow left out)" if quick else ""} in {time.time() - t0:.0f} s, {par} at a time')
