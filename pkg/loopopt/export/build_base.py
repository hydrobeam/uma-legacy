"""The era's roster (released by the cut: era.py; has a trainee card) and aptitudes -> base.json (codes 1..8 = G..S:
short, mile, middle, long, turf, dirt, then front runner, pace chaser, late surger, end closer).
usage: build_base.py global|jp [CHARA:CARD ...]
Aptitudes come from each uma's first outfit (the lowest released card id) unless a CHARA:CARD
argument names another, e.g. 1008:100802 for [Fiery Aqua Vitae] Vodka."""
import json, sys
from era import q, names, CUT, BASE, outfit_released
name = names(6)
outfits = lambda ch: [x for (x,) in q("select id from card_data where chara_id=? and id<900000 order by id", ch) if outfit_released(x)]
roster = [r[0] for r in q("select id from chara_data where start_date<?", CUT) if outfits(r[0])]
OVR = {int(a): int(b) for a, b in (x.split(':') for x in sys.argv[2:] if ':' in x)}
apt = {}
for ch in roster:
    card = OVR.get(ch) or outfits(ch)[0]
    assert q("select chara_id from card_data where id=?", card)[0][0] == ch, f"card {card} is not chara {ch}"
    apt[ch] = q("select proper_distance_short,proper_distance_mile,proper_distance_middle,proper_distance_long,proper_ground_turf,proper_ground_dirt,proper_running_style_nige,proper_running_style_senko,proper_running_style_sashi,proper_running_style_oikomi from card_rarity_data where card_id=? order by rarity limit 1", card)[0]
json.dump({'roster': roster, 'apt': apt, 'name': {k: name[k] for k in roster}}, open(BASE, 'w'))
print(len(roster), 'umas', 'outfit overrides:', OVR or 'none')
