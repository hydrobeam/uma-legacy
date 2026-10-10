"""Export everything the Rust core needs to the era's data.json. Relocated venues = same race (samerace patch).
usage: export_rs.py global|jp [--out FILE]"""
import samerace, json, itertools, os, sys
from model import *
from rel import pt, mem
from era import OUT
races={PROG[p]['race'] for v in G1CAL.values() for p in v}
for ch in ROSTER:   # career objectives can be G1s outside the standard calendar programs; include their races too
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
    # her ten aptitude codes (base.json order), for the career page's pink sparks (bp above is from the first six)
    umas.append({'id':ch,'name':NAME[ch],'turns':turns,'goals':groups,'apt':list(APT[ch])})
# relation groups instead of the expanded pair / trio tables: [points, [roster indices]] for every relation type with at
# least two roster members. pair(a,b) / tri(a,b,c) = sum of the points of the groups holding all of them (distinct umas);
# the solver rebuilds both tables at load (lib.rs rel_tables). Same numbers as rel.pair / rel.tri.
RELS=[[pt[t],m] for t in sorted(pt) for m in [[i for i,ch in enumerate(ROSTER) if t in mem.get(ch,set())]] if len(m)>=2]
# per race: distance class (0 short .. 3 long, as basep) and surface (1 turf, 2 dirt), for aptitude-dependent win chances
RACE_AT={}
for pid,p in PROG.items(): RACE_AT.setdefault(p['race'],(cat(p['dist']),p['ground']))
json.dump({'races':races,'race_names':[RN[r] for r in races],'race_cat':[RACE_AT[r][0] for r in races],'race_ground':[RACE_AT[r][1] for r in races],'umas':umas,'relations':RELS,'pen':[PEN[k] for k in range(1,7)]},open(sys.argv[sys.argv.index('--out')+1] if '--out' in sys.argv else os.path.join(OUT,'data.json'),'w'))
print('races',len(races),'umas',len(umas),'max options/turn',max(len(l) for u in umas for l in u['turns']),
      'multi-turn goal groups',sum(1 for u in umas for g in u['goals'] if len({t for t,_ in g})>1),
      'same-turn multi-option groups',sum(1 for u in umas for g in u['goals'] if len(g)>1 and len({t for t,_ in g})==1))
print([ (u['name'],u['goals']) for u in umas if any(len({t for t,_ in g})>1 for g in u['goals'])])
