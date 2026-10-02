"""Support cards for deck search -> ../web_cards.json. Run from this folder with master.mdb present.
Per released card: chara, rarity, pal (friend type), chain length (story events with progress markers),
hint frequency at max limit break (effect 18), number of distinct training-hint options (skills + stat option),
and the skill groups (white level) in its training-hint pool. Event skill rewards are NOT in master.mdb tables;
they are supplied per job (`events`)."""
import sqlite3, json, time
c = sqlite3.connect('master.mdb'); q = lambda s, *a: c.execute(s, a).fetchall()
cn = dict(q("select [index],text from text_data where category=75"))
grp = dict(q("select id, group_id from skill_data"))
now = int(time.time()); out = []
# characters who cannot train with a card: its own character plus, for group cards, every member
members = {}
for sid, ch in q("select support_card_id, chara_id from support_card_group"): members.setdefault(sid, set()).add(ch)
KIND = {101: 'speed', 105: 'stamina', 102: 'power', 103: 'guts', 106: 'wit'}
for sid, chara, rar, typ, eff, sd, cmd in q("select id,chara_id,rarity,support_card_type,effect_table_id,start_date,command_id from support_card_data"):
    if sd > now: continue   # group cards (type 3) act as pals: no chain slots, skills from their dates
    rows = q("select hint_group,hint_gain_type,hint_value_1 from single_mode_hint_gain where support_card_id=?", sid)
    opts = len({g for g, _, _ in rows})
    whites = sorted({grp[v] for g, t, v in rows if t == 0 and v in grp})
    hf = q("select max(init,limit_lv5,limit_lv10,limit_lv15,limit_lv20,limit_lv25,limit_lv30,limit_lv35,limit_lv40,limit_lv45,limit_lv50) from support_card_effect_table where id=? and type=18", eff)
    links = q("select max(show_progress_2) from single_mode_story_data where support_card_id=?", sid)[0][0] or 0
    out.append({"id": sid, "name": cn[sid], "chara": chara, "rarity": rar, "pal": typ in (2, 3), "links": links,
                "hf": max(0, hf[0][0]) if hf else 0, "opts": opts, "whites": whites, "charas": sorted(members.get(sid, set()) | {chara}), "kind": 'pal' if typ == 2 else 'group' if typ == 3 else KIND.get(cmd, 'other')})
json.dump({"cards": out}, open('../web_cards.json', 'w'), ensure_ascii=False, separators=(',', ':'))
print(len(out), 'cards;', sum(x['pal'] for x in out), 'pals')
