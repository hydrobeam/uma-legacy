"""Skill tables for the web app: each roster uma's outfits with the target-able skills on their own skill list
(need_rank <= 5, i.e. fully awakened), plus the catalog of white skills (and their gold upgrade) users can pick.
Writes the era's web_skills.json. usage: export_web.py global|jp"""
import sqlite3, json, os, re
from era import c, names, BASE, OUT, outfit_released
q = lambda s, *a: c.execute(s, a).fetchall()
B = json.load(open(BASE)); roster = B['roster']
sname = names(47)
cname = names(5)
# inherited unique skills (unique_skill_id_1 set) come only from a parent's unique spark, never a hint: not targets
sk = {i: (r, g) for i, r, g in q('select id, rarity, group_id from skill_data where unique_skill_id_1 = 0')}
# catalog: groups with a white (rarity 1) skill that has a name; gold = rarity 2 in the same group
groups = {}
for i, (r, g) in sk.items():
    if r in (1, 2) and i in sname:
        e = groups.setdefault(g, [None, None])
        if r == 1 and e[0] is None: e[0] = sname[i]
        if r == 2 and e[1] is None: e[1] = sname[i]
catalog = sorted([[g, w, gd] for g, (w, gd) in groups.items() if w], key=lambda x: x[1].lower())
cards = {}
for cid, ch, ass in q('select id, chara_id, available_skill_set_id from card_data where id<900000 order by id'):
    if ch not in roster or not outfit_released(cid): continue
    cov = {}
    for sid, rank in q('select skill_id, need_rank from available_skill_set where available_skill_set_id=?', ass):
        if rank > 5 or sid not in sk: continue
        r, g = sk[sid]
        if g in groups and r in (1, 2):
            cov[g] = max(cov.get(g, 0), 2 if r == 2 else 1)
    label = re.match(r'(\[.*?\])', cname.get(cid, '')).group(1) if re.match(r'\[.*?\]', cname.get(cid, '')) else str(cid)
    cards.setdefault(ch, []).append([cid, label, sorted(cov.items())])
json.dump({'cards': cards, 'skills': catalog}, open(os.path.join(OUT, 'web_skills.json'), 'w'), ensure_ascii=False)
print(len(catalog), 'skills,', sum(len(v) for v in cards.values()), 'outfits for', len(cards), 'umas')
