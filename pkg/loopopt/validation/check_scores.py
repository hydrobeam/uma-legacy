"""Score parity: run loopopt in 'score' mode on stored schedules and compare to the Python reference values."""
import json, subprocess
K=json.load(open('known_schedules.json')); worst=0
for scheme in ('scf','rot'):
    cs=[k for k in K if k['scheme']==scheme]
    json.dump({'scheme':scheme,'iters':0,'seed':1,'mode':'score','jobs':[{'members':k['members'],'init':[k['schedules']]} for k in cs]},open('_jobs.json','w'))
    res=json.loads(subprocess.run(['../target/release/loopopt','../data.json','_jobs.json'],capture_output=True,text=True,check=True).stdout)
    for k,r in zip(cs,res):
        worst=max(worst,abs(k['python_value']-r['value'])); assert all(r['feasible'])
print(len(K),'schedules, max |rust - python| =',worst)
