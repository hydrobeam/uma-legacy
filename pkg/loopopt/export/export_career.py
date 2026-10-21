"""Tables for reading a veteran export (umadump's trained_chara_data.json) into the career planner. Reads the era's
data.json (run after export_rs.py) and writes career.json next to it:
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
usage: export_career.py global|jp"""

import sqlite3, json, os
from era import q, OUT, GLOBAL_MDB

data_path, out_path = os.path.join(OUT, "data.json"), os.path.join(OUT, "career.json")
RELOC = {1025: 1012, 1027: 1006, 1028: 1005}  # samerace.py
races = json.load(open(data_path))["races"]
race_indices = {race_id: index for index, race_id in enumerate(races)}
factors = {}
for factor_group, skill_id in q("""select distinct s.factor_group_id, e.value_1 from succession_factor s
        join succession_factor_effect e on e.factor_group_id = s.factor_group_id where s.factor_type = 4"""):
    skill_group_rows = q("select group_id from skill_data where id = ?", skill_id)
    if skill_group_rows:
        factors.setdefault(factor_group, skill_group_rows[0][0])
rows = q("""select s.id, s.group_id, i.race_id from single_mode_wins_saddle s join race_instance i on i.id = s.race_instance_id_1
        join race r on r.id = i.race_id where s.race_instance_id_2 = 0 and r.grade = 100""")
saddle_groups = {}
for saddle_id, group_id, race_id in rows:
    race_id = RELOC.get(race_id, race_id)
    if race_id in race_indices:
        assert saddle_groups.setdefault(group_id, race_indices[race_id]) == race_indices[race_id], (saddle_id, group_id)
saddles = {saddle_id: saddle_groups[group_id] for saddle_id, group_id, _ in rows if group_id in saddle_groups}
unmapped = [saddle_id for saddle_id, group_id, _ in rows if group_id not in saddle_groups]
# names from the Global master.mdb (the JP one has Japanese text; the groups are the same)
global_query = sqlite3.connect(f"file:{GLOBAL_MDB}?mode=ro", uri=True).execute
pinks = {
    group_id: global_query(
        'select text from text_data where category = 147 and "index" = ?', (group_id * 100 + 1,)
    ).fetchone()[0]
    for (group_id,) in q(
        "select distinct factor_group_id from succession_factor where factor_type = 2 order by factor_group_id"
    )
}
# blue (stat) spark groups 1..5 and unique (green) spark groups (an outfit's card id) -> name, the same way
blues = {
    group_id: global_query(
        'select text from text_data where category = 147 and "index" = ?', (group_id * 100 + 1,)
    ).fetchone()[0]
    for (group_id,) in q(
        "select distinct factor_group_id from succession_factor where factor_type = 1 order by factor_group_id"
    )
}
uniques = {}
for (group_id,) in q(
    "select distinct factor_group_id from succession_factor where factor_type = 3 order by factor_group_id"
):
    row = global_query(
        'select text from text_data where category = 147 and "index" = ?', (group_id * 100 + 1,)
    ).fetchone()
    if row:
        uniques[group_id] = row[0]
mobs = []
for (mob_id,) in global_query("select mob_id from mob_data order by mob_id"):
    row = global_query('select text from text_data where category = 59 and "index" = ?', (mob_id,)).fetchone()
    if row and row[0]:
        mobs.append([mob_id, row[0]])
json.dump(
    {"factors": factors, "saddles": saddles, "pinks": pinks, "blues": blues, "uniques": uniques, "mobs": mobs},
    open(out_path, "w"),
    separators=(",", ":"),
)
print(
    len(factors),
    "skill spark groups,",
    len(saddles),
    "G1 saddles",
    f"({len(unmapped)} G1 saddles of races no trainee can run: {unmapped})" if unmapped else "",
)
