import json,sys,glob,os
V=os.path.dirname(os.path.abspath(__file__))+'/'
tag=sys.argv[1]; bad=0; n=0
def load(p):
    t=open(p).read()
    try: return json.loads(t[t.index('{'):])
    except Exception: return None
for rf in sorted(glob.glob(V+sys.argv[2]+'__*.out')):
    name=rf.split(sys.argv[2]+'__')[1]; nf=V+f'{tag}__{name}'
    if not os.path.exists(nf): continue
    r,x=load(rf),load(nf); n+=1
    if r is None or x is None: print(name,'PARSE FAIL', r is None, x is None); bad+=1; continue
    if 'top' in r:
        sig=lambda j:[(round(t['loop']['p_all'],14),round(t['loop']['p_minus1'],12),tuple(sorted(zip(t['deck']['card_ids'],t['deck'].get('end_chain_at',[0]*9)))),tuple(m if isinstance(m,str) else m.get('name') for m in t['loop']['members'])) for t in j['top']]
        a,b=sig(r),sig(x); ok=[s[0] for s in a]==[s[0] for s in b]; same=a==b
        print(f"{name}: values {'OK' if ok else 'DIFF'} decks {'same' if same else 'differ(ties?)'} ref {[round(1/s[0]) for s in a]} new {[round(1/s[0]) for s in b]} proven {r.get('proven')}/{x.get('proven')}")
        if not ok: bad+=1
    elif 'results' in r:
        sig=lambda j:[(round(c['p_all'],14),round(c['p_minus1'],12),c['trainee']['id'],tuple(c['parents']),tuple(sorted(c['deck']['card_ids']))) for c in j['results']]
        a,b=sig(r),sig(x); ok=[s[0] for s in a]==[s[0] for s in b]; same=a==b
        print(f"{name}: career values {'OK' if ok else 'DIFF'} picks {'same' if same else 'differ(ties?)'} proven {r.get('proven_top')}/{x.get('proven_top')}")
        if not ok: bad+=1
    else:
        sig=lambda j:[(round(l['p_all'],14),tuple(m if isinstance(m,str) else m.get('name') for m in l['members'])) for l in j.get('loops',[])]
        ok=sig(r)==sig(x); print(name,'skills', 'OK' if ok else 'DIFF'); bad+= not ok
print(f'{n} compared, {bad} value mismatches')
