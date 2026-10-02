"""Roster (released before the cutoff, has a trainee card) + aptitudes -> base.json. Run from this folder with master.mdb present.
Aptitudes come from each uma's base outfit (lowest card id) unless overridden with
OUTFIT="chara_id:card_id[,chara_id:card_id...]", e.g. OUTFIT=1008:100802 for [Fiery Aqua Vitae] Vodka."""
import sqlite3, datetime, json, os
c=sqlite3.connect('master.mdb'); q=lambda s,*a: c.execute(s,a).fetchall()
name={r[0]:r[1] for r in q("select [index],text from text_data where category=6")}
cut=int(datetime.datetime(2026,10,1).timestamp())
roster=[r[0] for r in q("select id from chara_data where start_date<? and id in (select chara_id from card_data where id<900000)",cut)]
OVR={int(a):int(b) for a,b in (x.split(':') for x in os.environ.get('OUTFIT','').split(',') if x)}
apt={}
for ch in roster:
    card=OVR.get(ch) or q("select id from card_data where chara_id=? and id<900000 order by id limit 1",ch)[0][0]
    assert q("select chara_id from card_data where id=?",card)[0][0]==ch, f"card {card} is not chara {ch}"
    apt[ch]=q("select proper_distance_short,proper_distance_mile,proper_distance_middle,proper_distance_long,proper_ground_turf,proper_ground_dirt from card_rarity_data where card_id=? order by rarity limit 1",card)[0]
json.dump({'roster':roster,'apt':apt,'name':{k:name[k] for k in roster}},open('base.json','w'))
print(len(roster),'umas', 'outfit overrides:',OVR or 'none')
