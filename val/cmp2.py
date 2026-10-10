import json,sys,glob,os
V=os.path.dirname(os.path.abspath(__file__))+'/'
# the deck structure classes of the exact caps (`cap_class`: chain-varying targets; a multi-target group counts as 4+)
CLS=['0-1','2','3','4+']
def near_cap(r, c):
    # a scored deck's exact / product r is near its class's assumed cap c: past three quarters of the cap's headroom
    # over 1 (1.1875 of 1.25, about the old 1.25 / 1.05; 1.0375 of 1.05).
    # The same rule as val/planval.py.
    return r > 1 + 0.75 * (c - 1)
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
    # the assumed caps per structure class: no scored deck's exact / product may come near its class's cap (a deck
    # above it is missed unseen)
    ex=x.get('exact') if isinstance(x, dict) else None
    if isinstance(ex, dict) and isinstance(ex.get('class_max_ratio'), list):
        for k,(rr,c) in enumerate(zip(ex['class_max_ratio'], ex['class_caps'])):
            if near_cap(rr, c): print(f"{name}: CAP class {CLS[k]} exact/product {rr:.4f} near its assumed cap {c}"); bad+=1
    elif isinstance(ex, dict) and isinstance(ex.get('max_ratio'), (int, float)):
        cap=ex.get('rho_target') or 1.25
        if near_cap(ex['max_ratio'], cap): print(f"{name}: CAP exact/product {ex['max_ratio']:.3f} near the assumed cap {cap}"); bad+=1
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
        print(name,'neither a deck nor a career output'); bad+=1
print(f'{n} compared, {bad} value mismatches')
