"""Run every deck validation job with per_uma on: val/TAG__*.out. usage: run_pu.py BIN TAG"""
import json, subprocess, sys, time, os
S = os.path.dirname(os.path.abspath(__file__)) + '/../'
b, tag = sys.argv[1], sys.argv[2]
for d, j in json.load(open(S + 'val/jobs.json')):
    job = json.load(open(S + d + '/' + j))
    if job.get('mode') != 'deck': continue
    job['per_uma'] = True
    tmp = S + f'_pu_tmp_{tag}.json'; json.dump(job, open(tmp, 'w'))
    name = (d.replace('/', '_') + '__' + j.split('/')[-1]).replace('.json', '')
    t = time.time(); out = subprocess.run([b, 'data.json', tmp], cwd=S + d, capture_output=True, text=True).stdout
    open(S + f'val/{tag}__{name}.out', 'w').write(out); print(name, round(time.time() - t, 1), flush=True)
