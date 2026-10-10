"""Roster (dated by the master, up to upcoming.AHEAD ahead; has a trainee card) + aptitudes -> base.json (codes 1..8 = G..S: short, mile,
middle, long, turf, dirt, then front runner, pace chaser, late surger, end closer). Run from this folder with master.mdb present.
Aptitudes come from each uma's base outfit (lowest card id) unless overridden with
OUTFIT="chara_id:card_id[,chara_id:card_id...]", e.g. OUTFIT=1008:100802 for [Fiery Aqua Vitae] Vodka."""
import sqlite3, datetime, json, os
c=sqlite3.connect(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', 'game', 'global', 'master.mdb')); q=lambda s,*a: c.execute(s,a).fetchall()
name={r[0]:r[1] for r in q("select [index],text from text_data where category=6")}
# dated by the master up to upcoming.AHEAD past the build: the page hides an uma until her start (upcoming.py)
from upcoming import CUT as cut
roster=[r[0] for r in q("select id from chara_data where start_date<? and id in (select chara_id from card_data where id<900000)",cut)]
OVR={int(a):int(b) for a,b in (x.split(':') for x in os.environ.get('OUTFIT','').split(',') if x)}
apt={}
for ch in roster:
    card=OVR.get(ch) or q("select id from card_data where chara_id=? and id<900000 order by id limit 1",ch)[0][0]
    assert q("select chara_id from card_data where id=?",card)[0][0]==ch, f"card {card} is not chara {ch}"
    apt[ch]=q("select proper_distance_short,proper_distance_mile,proper_distance_middle,proper_distance_long,proper_ground_turf,proper_ground_dirt,proper_running_style_nige,proper_running_style_senko,proper_running_style_sashi,proper_running_style_oikomi from card_rarity_data where card_id=? order by rarity limit 1",card)[0]
json.dump({'roster':roster,'apt':apt,'name':{k:name[k] for k in roster}},open('base.json','w'))
print(len(roster),'umas', 'outfit overrides:',OVR or 'none')
