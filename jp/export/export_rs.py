"""Export everything the Rust core needs. Relocated venues = same race (samerace patch)."""
import samerace, json, itertools, os
from model import *
from rel import pt, mem
races={PROG[p]['race'] for v in G1CAL.values() for p in v}
# JP: some career objectives are G1s outside the standard calendar programs; include their races too
for ch in ROSTER:
    o,_=options(ch)
    races |= {PROG[p]['race'] for l in o.values() for p in l if is_g1(p)}
races=sorted(races)
ridx={r:i for i,r in enumerate(races)}
umas=[]
for ch in ROSTER:
    opt,gs=options(ch)
    turns=[]
    for t in range(1,73):
        lst=[]
        for p in opt.get(t,[]):
            g=is_g1(p); r=PROG[p]['race'] if g else None
            if g: assert r in ridx,(NAME[ch],t,p,r)
            lst.append({'prog':p,'race':ridx[r] if g else -1,'bp':basep(ch,p) if g else 0,'name':RN.get(PROG[p]['race'],str(p)),'goal':any((t,p) in [(tt,pp) for tt,pp in grp] for grp in gs)})
        turns.append(lst)
    groups=[]
    for g in gs:
        groups.append([[t,[o['prog'] for o in turns[t-1]].index(p)] for t,p in g])
    # sanity: a race appears at most once per year half
    seen={}
    for t,lst in enumerate(turns,1):
        for o in lst:
            if o['race']>=0:
                key=(o['race'],t<=48); assert key not in seen,(NAME[ch],RN[races[o['race']]]); seen[key]=t
    umas.append({'id':ch,'name':NAME[ch],'turns':turns,'goals':groups})
# relation groups instead of the expanded pair / trio tables: [points, [roster indices]] for every relation type with at
# least two roster members. pair(a,b) / tri(a,b,c) = sum of the points of the groups holding all of them (distinct umas);
# the solver rebuilds both tables at load (lib.rs rel_tables). Same numbers as rel.pair / rel.tri.
RELS=[[pt[t],m] for t in sorted(pt) for m in [[i for i,ch in enumerate(ROSTER) if t in mem.get(ch,set())]] if len(m)>=2]
json.dump({'races':races,'race_names':[RN[r] for r in races],'umas':umas,'relations':RELS,'pen':[PEN[k] for k in range(1,7)]},open(os.environ.get('OUT') or os.environ.get('JP_OUT', '..') + '/data.json','w'))
print('races',len(races),'umas',len(umas),'max options/turn',max(len(l) for u in umas for l in u['turns']),
      'multi-turn goal groups',sum(1 for u in umas for g in u['goals'] if len({t for t,_ in g})>1),
      'same-turn multi-option groups',sum(1 for u in umas for g in u['goals'] if len(g)>1 and len({t for t,_ in g})==1))
print([ (u['name'],u['goals']) for u in umas if any(len({t for t,_ in g})>1 for g in u['goals'])])
