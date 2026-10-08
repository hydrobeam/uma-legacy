"""Tables for reading a veteran export (umadump's trained_chara_data.json) into the career planner. Run from an era's
export folder (master.mdb there) with that era's data.json; writes career.json next to it:
  factors: skill spark factor group -> skill group (succession_factor type 4, via succession_factor_effect.value_1 and
           skill_data.group_id). A spark's factor_id is factor group * 100 + stars.
  saddles: win saddle id -> race index in data.json's races, for saddles of one G1 race (single_mode_wins_saddle with
           race_instance_id_2 = 0 and race grade 100). Saddles of one race at another venue (Kikuka Sho at Hanshin,
           the JBC races per host track) share the saddle's group_id, so a saddle takes the race of its group (the
           group's saddle whose race is in data.json, relocated venues mapped as in samerace). Multi-race title
           saddles are left out: they count for nothing in affinity.
  pinks:   aptitude (pink) spark factor group -> name (succession_factor type 2; factor_id = group * 100 + stars).
  blues:   stat (blue) spark factor group 1..5 -> name (type 1). uniques: unique (green) spark factor group (an outfit's
           card id) -> its unique skill's name (type 3).
  mobs:    every named mob uma (mob_data, English names from the Global text_data category 59 by mob id): the career
           planner's sample veterans, drawn at random (no relation points with anyone; they only carry sparks).
usage: export_career.py DATA_JSON OUT_JSON"""
import sqlite3, json, sys, os
data_path, out_path = sys.argv[1], sys.argv[2]
c = sqlite3.connect('master.mdb'); q = lambda s, *a: c.execute(s, a).fetchall()
RELOC = {1025: 1012, 1027: 1006, 1028: 1005}   # samerace.py
races = json.load(open(data_path))['races']
ridx = {r: i for i, r in enumerate(races)}
factors = {}
for fg, sid in q("""select distinct s.factor_group_id, e.value_1 from succession_factor s
        join succession_factor_effect e on e.factor_group_id = s.factor_group_id where s.factor_type = 4"""):
    g = q('select group_id from skill_data where id = ?', sid)
    if g: factors.setdefault(fg, g[0][0])
rows = q("""select s.id, s.group_id, i.race_id from single_mode_wins_saddle s join race_instance i on i.id = s.race_instance_id_1
        join race r on r.id = i.race_id where s.race_instance_id_2 = 0 and r.grade = 100""")
grp = {}
for sid, g, rid in rows:
    rid = RELOC.get(rid, rid)
    if rid in ridx: assert grp.setdefault(g, ridx[rid]) == ridx[rid], (sid, g)
saddles = {sid: grp[g] for sid, g, _ in rows if g in grp}
unmapped = [sid for sid, g, _ in rows if g not in grp]
# names from the Global master.mdb next to this script (the JP one has Japanese text; the groups are the same)
gq = sqlite3.connect(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'master.mdb')).execute
pinks = {g: gq('select text from text_data where category = 147 and "index" = ?', (g * 100 + 1,)).fetchone()[0]
         for (g,) in q('select distinct factor_group_id from succession_factor where factor_type = 2 order by factor_group_id')}
# blue (stat) spark groups 1..5 and unique (green) spark groups (an outfit's card id) -> name, the same way
blues = {g: gq('select text from text_data where category = 147 and "index" = ?', (g * 100 + 1,)).fetchone()[0]
         for (g,) in q('select distinct factor_group_id from succession_factor where factor_type = 1 order by factor_group_id')}
uniques = {}
for (g,) in q('select distinct factor_group_id from succession_factor where factor_type = 3 order by factor_group_id'):
    r = gq('select text from text_data where category = 147 and "index" = ?', (g * 100 + 1,)).fetchone()
    if r: uniques[g] = r[0]
mobs = []
for (m,) in gq('select mob_id from mob_data order by mob_id'):
    r = gq('select text from text_data where category = 59 and "index" = ?', (m,)).fetchone()
    if r and r[0]: mobs.append([m, r[0]])
json.dump({'factors': factors, 'saddles': saddles, 'pinks': pinks, 'blues': blues, 'uniques': uniques, 'mobs': mobs}, open(out_path, 'w'), separators=(',', ':'))
print(len(factors), 'skill spark groups,', len(saddles), 'G1 saddles', f'({len(unmapped)} G1 saddles of races no trainee can run: {unmapped})' if unmapped else '')
