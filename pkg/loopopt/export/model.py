import sqlite3, json, itertools, math, os
c=sqlite3.connect(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', 'game', 'global', 'master.mdb'))
q=lambda s,*a: c.execute(s,a).fetchall()
RN={r[0]:r[1] for r in q("select [index],text from text_data where category=32")}
B=json.load(open('base.json'))
ROSTER=B['roster']; NAME={int(k):v for k,v in B['name'].items()}
APT={int(k):v for k,v in B['apt'].items()}   # (short,mile,mid,long,turf,dirt) codes 1..8 = G..S
def off(code): # S,A ->0, B->1 ... G->6
    return max(0, 8-code-1)
def cat(dist): return 0 if dist<=1400 else 1 if dist<=1800 else 2 if dist<=2400 else 3
YB={1:[0],2:[24],3:[24,48],4:[48]}
def turn(yb,m,h): return yb+(m-1)*2+h
# program info
PROG={}
for pid,base,perm,m,h,rid,grade,dist,ground in q("""select p.id,p.base_program_id,p.race_permission,p.month,p.half,r.id,r.grade,cs.distance,cs.ground
 from single_mode_program p join race_instance ri on p.race_instance_id=ri.id join race r on ri.race_id=r.id join race_course_set cs on r.course_set=cs.id"""):
    PROG[pid]=dict(base=base,perm=perm,m=m,h=h,race=rid,grade=grade,dist=dist,ground=ground)
G1CAL={}  # turn -> list of (pid, race_id)
for pid,p in PROG.items():
    if p['grade']==100 and p['base']==0 and 1<=p['perm']<=4:
        for yb in YB[p['perm']]:
            G1CAL.setdefault(turn(yb,p['m'],p['h']),[]).append(pid)
# Win odds (community table, Oct 2026): base = 110 - cost(distance grade) - cost(surface grade),
# minus the streak penalty for the k-th race in a row; clamp 0..100.
# Grade codes 1..8 = G..S. Cost: S/A 0, B 10, C 20, D 30, E 40, F 60, G 90.
PEN={1:0,2:0,3:5,4:20,5:30,6:50}
GRADE_COST={8:0,7:0,6:10,5:20,4:30,3:40,2:60,1:90}
def basep(ch,pid,apt=None):
    a=apt or APT[ch]; p=PROG[pid]
    d=GRADE_COST[a[cat(p['dist'])]]; s=GRADE_COST[a[4] if p['ground']==1 else a[5]]
    return max(-20, 110-d-s)   # floor keeps the Rust lookup table in range; anything <= 0 is a 0% win
def pwin(bp,k): return max(0,min(100,bp-PEN[min(k,6)]))/100
def goals(ch, exclude_det2=True):
    rows=q("select sort_id,turn,condition_id,determine_race from single_mode_route_race where race_set_id=? and scenario_group_id=100 and condition_type=1 and turn<=72 order by sort_id,id",ch)
    g={}
    for sid,t,pid,det in rows:
        if exclude_det2 and det==2: continue
        g.setdefault(sid,[]).append((t,pid))
    return list(g.values())
def is_g1(pid): return PROG[pid]['grade']==100

JBC={20002:1025, 20003:1026}  # race_type-4 goals: JBC Sprint / JBC Classic base programs (assumed mapping)
def goals2(ch, exclude_det2=True):
    gs=[]
    for g in goals(ch, exclude_det2):
        gs.append([(t, JBC.get(p,p)) for t,p in g])
    return gs
# turns a career forbids racing (single_mode_race_restrict_turn): gain_id names the story event that sets it (1078 118 11 is
# story 501078118). Yamanin Zephyr turn 44 (Classic late October: Tenno Sho (Autumn), Shuka Sho, Kikuka Sho) and Sweep
# Tosho turn 36 (Classic late June: Takarazuka Kinen) are fixed story events that rest her (+20 energy), so she races
# nothing. Matikanetannhauser's turn 70 is the Japan Cup goal the event assigns her (route row determine_race 4), so only
# the goal race stays on a restricted turn.
RESTRICT={}
for _ch,_t in q("select chara_id, turn from single_mode_race_restrict_turn"): RESTRICT.setdefault(_ch,set()).add(_t)
def options(ch, exclude_det2=True):
    """turn -> list of programs runnable (one program per (turn,race)); goal groups remapped to those programs; only goal
    races at a turn the career forbids racing (RESTRICT)"""
    gs=goals2(ch, exclude_det2)
    opt={t:list(v) for t,v in G1CAL.items() if t not in RESTRICT.get(ch,())}
    canon={}
    for g in gs:
        for t,p in g:
            opt.setdefault(t,[])
            same=[x for x in opt[t] if PROG[x]['race']==PROG[p]['race']]
            if same and same[0] in G1CAL.get(t,[]) and p not in G1CAL.get(t,[]):
                opt[t]=[x for x in opt[t] if x!=same[0]]; opt[t].append(p); canon[(t,PROG[p]['race'])]=p
            elif same: canon.setdefault((t,PROG[p]['race']),same[0])
            else: opt[t].append(p); canon[(t,PROG[p]['race'])]=p
    gs2=[]
    for g in gs:
        gg=[]
        for t,p in g:
            q_=canon.get((t,PROG[p]['race']),p)
            if (t,q_) not in gg: gg.append((t,q_))
        gs2.append(gg)
    return opt, gs2
