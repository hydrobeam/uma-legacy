"""What the Global exports count as released, and what is dated but has not started yet.

The exports keep every uma, outfit and support card that master.mdb dates up to AHEAD past the build: a master fetched
ahead of its update already holds the next week's, with their start times. The page hides each one
until its start time (webapp/src/data.js), so a banner turns on without a rebuild. Content the game has not dated
yet carries a 2050 placeholder and stays out.

Run as a script (step data, after the other exports): ../upcoming.json, the entries dated after the build, as
{umas, outfits, cards: [[id, start (unix s)], ...]}, for webapp/example.mjs, whose precomputed result must not use them.
"""
import json, sqlite3, time

AHEAD = 60 * 86400
NOW = int(time.time())
CUT = NOW + AHEAD


def outfit_starts(q):
    """outfit (trainee card) id -> its start: the first gacha it appeared in, else its uma's start date."""
    chara = dict(q('select id, start_date from chara_data'))
    first = dict(q('select a.card_id, min(d.start_date) from gacha_available a join gacha_data d on d.id = a.gacha_id group by a.card_id'))
    return {cid: first.get(cid, chara.get(ch, 0)) for cid, ch in q('select id, chara_id from card_data where id<900000')}


if __name__ == '__main__':
    c = sqlite3.connect('master.mdb'); q = lambda s, *a: c.execute(s, a).fetchall()
    roster = set(json.load(open('base.json'))['roster'])
    starts = outfit_starts(q)
    owner = dict(q('select id, chara_id from card_data where id<900000'))
    outfits = {o: t for o, t in starts.items() if owner[o] in roster and t <= CUT}
    later_o = {o: t for o, t in outfits.items() if t > NOW}
    umas = {}
    for o, t in outfits.items(): umas.setdefault(owner[o], []).append(t)
    later_u = {u: min(ts) for u, ts in umas.items() if min(ts) > NOW}
    cards = {cid for cid in (x['id'] for x in json.load(open('../web_cards.json'))['cards'])}
    later_c = {i: t for i, t in q('select id, start_date from support_card_data') if i in cards and t > NOW}
    out = {k: sorted(v.items()) for k, v in (('umas', later_u), ('outfits', later_o), ('cards', later_c))}
    json.dump(out, open('../upcoming.json', 'w'))
    print('upcoming (dated after the build):', {k: len(v) for k, v in out.items()})
