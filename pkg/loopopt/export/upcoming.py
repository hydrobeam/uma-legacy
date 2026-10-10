"""What the Global export holds that is dated but has not started yet. usage: upcoming.py global

The exports keep every uma, outfit and support card that master.mdb dates up to era.AHEAD past the build: a master taken
ahead of its update already holds the next week's, with their start times. The page hides each one
until its start time (webapp/src/data.js), so a banner turns on without a rebuild. Content the game has not dated
yet carries a 2050 placeholder and stays out.

Run as a script (step data, after the other exports): ../upcoming.json, the entries dated after the build, as
{umas, outfits, cards: [[id, start (unix s)], ...]}, for webapp/example.mjs, whose precomputed result must not use them.
"""
import json, os
from era import q, CUT, NOW, OUT, BASE, outfit_starts


if __name__ == '__main__':
    roster = set(json.load(open(BASE))['roster'])
    starts = outfit_starts()
    owner = dict(q('select id, chara_id from card_data where id<900000'))
    outfits = {o: t for o, t in starts.items() if owner[o] in roster and t <= CUT}
    later_o = {o: t for o, t in outfits.items() if t > NOW}
    umas = {}
    for o, t in outfits.items(): umas.setdefault(owner[o], []).append(t)
    later_u = {u: min(ts) for u, ts in umas.items() if min(ts) > NOW}
    cards = {cid for cid in (x['id'] for x in json.load(open(os.path.join(OUT, 'web_cards.json')))['cards'])}
    later_c = {i: t for i, t in q('select id, start_date from support_card_data') if i in cards and t > NOW}
    out = {k: sorted(v.items()) for k, v in (('umas', later_u), ('outfits', later_o), ('cards', later_c))}
    json.dump(out, open(os.path.join(OUT, 'upcoming.json'), 'w'))
    print('upcoming (dated after the build):', {k: len(v) for k, v in out.items()})
