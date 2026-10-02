import numpy as np, scipy.sparse as sp
from scipy.optimize import milp, LinearConstraint, Bounds
from model import *
K=6
_cache={}
def setup(ch, apt=None, exclude_det2=True):
    key=(ch,apt,exclude_det2)
    if key in _cache: return _cache[key]
    opt,gs=options(ch,exclude_det2)
    var=[]  # (t,pid,k)
    for t in range(1,73):
        for pid in opt.get(t,[]):
            for k in range(1,K+1): var.append((t,pid,k))
    idx={v:i for i,v in enumerate(var)}
    # repeat races
    byrace={}
    for t in range(1,73):
        for pid in opt.get(t,[]):
            if is_g1(pid): byrace.setdefault(PROG[pid]['race'],[]).append((t,pid))
    rep={r:v for r,v in byrace.items() if len(v)>=2}
    assert all(len(v)==2 for v in rep.values()), rep
    _cache[key]=(opt,gs,var,idx,byrace,rep)
    return _cache[key]

def best_response(ch, W, apt=None, exclude_det2=True, time_limit=60, force=None):
    """maximize sum_r W[r]*P_ch(r) exactly. returns (value, sched)"""
    opt,gs,var,idx,byrace,rep=setup(ch,apt,exclude_det2)
    nY=len(var)
    zvars=[]  # (r,k1,k2)
    for r in rep:
        for k1 in range(1,K+1):
            for k2 in range(1,K+1): zvars.append((r,k1,k2))
    n=nY+len(zvars)
    cobj=np.zeros(n)
    for i,(t,pid,k) in enumerate(var):
        if is_g1(pid):
            cobj[i]=W.get(PROG[pid]['race'],0)*pwin(basep(ch,pid,apt),k)
    for j,(r,k1,k2) in enumerate(zvars):
        (t1,p1),(t2,p2)=rep[r]
        cobj[nY+j]=-W.get(r,0)*pwin(basep(ch,p1,apt),k1)*pwin(basep(ch,p2,apt),k2)
    rows=[];lo=[];hi=[]
    def add(coefs,l,h):
        rows.append(coefs); lo.append(l); hi.append(h)
    S=lambda t,k: [idx[(t,p,k)] for p in opt.get(t,[])]
    X=lambda t: [idx[(t,p,k)] for p in opt.get(t,[]) for k in range(1,K+1)]
    for t in range(1,73):
        if not opt.get(t): continue
        add({i:1 for i in X(t)},0,1)
        prevX=X(t-1) if opt.get(t-1) else []
        if prevX:
            d={i:1 for i in S(t,1)}; d.update({i:1 for i in prevX}); add(d,0,1)
            for k in range(2,K):
                d={i:1 for i in S(t,k)}; d.update({i:-1 for i in S(t-1,k-1)}); add(d,-np.inf,0)
            d={i:1 for i in S(t,K)}; d.update({i:-1 for i in S(t-1,K-1)+S(t-1,K)}); add(d,-np.inf,0)
        else:
            for k in range(2,K+1): add({i:1 for i in S(t,k)},0,0)
    for g in gs:
        d={}
        for t,p in g:
            for k in range(1,K+1): d[idx[(t,p,k)]]=1
        add(d,1,np.inf)
    for j,(r,k1,k2) in enumerate(zvars):
        (t1,p1),(t2,p2)=rep[r]
        add({nY+j:1, idx[(t1,p1,k1)]:-1, idx[(t2,p2,k2)]:-1},-1,np.inf)
    for t,p in (force or {}).items():
        if (t,p,1) in idx: add({idx[(t,p,k)]:1 for k in range(1,K+1)},1,1)
    A=sp.lil_matrix((len(rows),n))
    for i,d in enumerate(rows):
        for j,v in d.items(): A[i,j]=v
    integ=np.r_[np.ones(nY),np.zeros(len(zvars))]
    res=milp(-cobj,constraints=LinearConstraint(A.tocsr(),lo,hi),integrality=integ,bounds=Bounds(0,1),
             options=dict(time_limit=time_limit,mip_rel_gap=1e-9))
    assert res.status in (0,1), res.message
    x=res.x
    sched=[(t,pid) for i,(t,pid,k) in enumerate(var) if x[i]>0.5]
    P=Pvec(ch,sched,apt)
    val=sum(W.get(r,0)*P[r] for r in P)
    return val,sched,res

def PM(ch, apt=None):
    """best case P per race: 1-(1-base)^attempts, no streak penalty"""
    opt,gs,var,idx,byrace,rep=setup(ch,apt)
    out={}
    for r,v in byrace.items():
        b=min(1,max(0,basep(ch,v[0][1],apt))/100)
        out[r]=1-(1-b)**len(v)
    return out

def optimize_loop(o, apts=None, seeds=('opt',), iters=30, verbose=False, init=None):
    apts=apts or {}
    C=loop_C(o)
    def W_for(a,P):
        W={}
        for (x,y),w in C.items():
            if a in (x,y):
                b=y if x==a else x
                for r,pv in P[b].items(): W[r]=W.get(r,0)+3*w*pv
        return W
    best=None
    for seed in seeds:
        if init is not None: scheds=dict(init); P={ch:Pvec(ch,scheds[ch],apts.get(ch)) for ch in o}
        else:
            P={ch:PM(ch,apts.get(ch)) for ch in o}; scheds={}
            if seed!='opt':
                rs=np.random.RandomState(seed)
                P={ch:{r:v*rs.uniform(0.3,1) for r,v in P[ch].items()} for ch in o}
            for ch in o:
                _,s,_=best_response(ch,W_for(ch,P),apts.get(ch)); scheds[ch]=s
            P={ch:Pvec(ch,scheds[ch],apts.get(ch)) for ch in o}
        cur=score(o,scheds,'G',apts)[0]
        for it in range(iters):
            improved=False
            for ch in o:
                v,s,_=best_response(ch,W_for(ch,P),apts.get(ch))
                old=scheds[ch]; scheds[ch]=s; P[ch]=Pvec(ch,s,apts.get(ch))
                new=score(o,scheds,'G',apts)[0]
                if new>cur+1e-7: cur=new; improved=True
                elif new<cur-1e-7: scheds[ch]=old; P[ch]=Pvec(ch,old,apts.get(ch))
            if verbose: print(seed,it,round(cur,2))
            if not improved: break
        if best is None or cur>best[0]: best=(cur,{k:list(v) for k,v in scheds.items()})
    return best
