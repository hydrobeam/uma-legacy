import samerace, json, subprocess, random, time
from opt import *
races=json.load(open('rs/data.json'))['races']; ridx={r:i for i,r in enumerate(races)}
rs_=random.Random(11); cases=[]
for ch in ROSTER:
    opt,gs=options(ch); goal_t={t for g in gs for t,p in g}
    for kind in ('full','sparse','forced'):
        W={r:rs_.uniform(0,80) for r in races if kind!='sparse' or rs_.random()<0.5}
        force=None
        if kind=='forced':
            cand=[(t,p) for t in range(1,73) for p in opt.get(t,[]) if is_g1(p) and t not in goal_t]
            t,p=rs_.choice(cand); force=(t,p) if rs_.random()<0.8 else (t,0)
        cases.append((ch,kind,W,force))
jobs={'scheme':'scf','iters':0,'seed':1,'mode':'br','jobs':[{'members':[c,c,c,c],'init':[[[[ridx[r],int(round(w*1e6))] for r,w in W.items()]]+([[list(f)]] if f else [])]} for c,k,W,f in cases]}
json.dump(jobs,open('rs/br_jobs2.json','w'))
out=json.loads(subprocess.run(['rs/target/release/loopopt','rs/data.json','rs/br_jobs2.json'],capture_output=True,text=True).stdout)
bad=0; worst=0; n=0
for (ch,kind,W,f),r in zip(cases,out):
    W={k:round(v*1e6)/1e6 for k,v in W.items()}
    force=None
    if f: force={f[0]:f[1]} if f[1] else None
    if f and not f[1]:
        # force rest: emulate in MILP by zero-ing all options at that turn via force of nothing -> skip MILP rest-force, compare to unforced upper bound only
        mv,_,_=best_response(ch,W); 
        if r['value'] is not None and r['value']>mv+1e-6: bad+=1; print('rest-forced exceeds unforced',NAME[ch])
        continue
    mv,ms,_=best_response(ch,W,force=force)
    n+=1; gap=mv-(r['value'] if r['value'] is not None else -1e9); worst=max(worst,abs(gap))
    if abs(gap)>1e-6: bad+=1; print('MISMATCH',NAME[ch],kind,f,round(mv,4),r['value'])
special=[NAME[c] for c in (1058,1064,1052,1005,1022,1056,1069)]
print(f'exact comparisons {n}, mismatches {bad}, max |gap| {worst:.2e}; covered special umas: {special}')
