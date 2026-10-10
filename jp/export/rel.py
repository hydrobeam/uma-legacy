import sqlite3,itertools, os
c=sqlite3.connect(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'game', 'jp', 'master.mdb'))
pt={r[0]:r[1] for r in c.execute("select relation_type,relation_point from succession_relation")}
mem={}
for t,ch in c.execute("select relation_type,chara_id from succession_relation_member"):
    mem.setdefault(ch,set()).add(t)
def pair(a,b): return 0 if a==b else sum(pt[t] for t in mem.get(a,set())&mem.get(b,set()))
def tri(a,b,c_): return 0 if len({a,b,c_})<3 else sum(pt[t] for t in mem.get(a,set())&mem.get(b,set())&mem.get(c_,set()))
