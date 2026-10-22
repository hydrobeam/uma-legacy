"""What the Global export holds that is dated but has not started yet. usage: upcoming.py global

The exports keep every uma, outfit and support card that master.mdb dates up to era.AHEAD past the build: a master taken
ahead of its update already holds the next week's, with their start times. The page hides each one
until its start time (webapp/src/data.js), so a banner turns on without a rebuild. Content the game has not dated
yet carries a 2050 placeholder and stays out.

Run as a script (step data, after the other exports): pkg/loopopt/data/global/upcoming.json, the entries dated after the build, as
{umas, outfits, cards: [[id, start (unix s)], ...]}, for webapp/example.mjs, whose precomputed result must not use them.
"""

import json, os
from era import q, CUT, NOW, OUT, BASE, outfit_starts


if __name__ == "__main__":
    roster = set(json.load(open(BASE))["roster"])
    starts = outfit_starts()
    outfit_owners = dict(q("select id, chara_id from card_data where id<900000"))
    outfits = {
        outfit_id: start_time
        for outfit_id, start_time in starts.items()
        if outfit_owners[outfit_id] in roster and start_time <= CUT
    }
    upcoming_outfits = {outfit_id: start_time for outfit_id, start_time in outfits.items() if start_time > NOW}
    umas = {}
    for outfit_id, start_time in outfits.items():
        umas.setdefault(outfit_owners[outfit_id], []).append(start_time)
    upcoming_umas = {
        character_id: min(start_times) for character_id, start_times in umas.items() if min(start_times) > NOW
    }
    cards = {card["id"] for card in json.load(open(os.path.join(OUT, "web_cards.json")))["cards"]}
    upcoming_cards = {
        i: start_time
        for i, start_time in q("select id, start_date from support_card_data")
        if i in cards and start_time > NOW
    }
    out = {
        k: sorted(v.items())
        for k, v in (("umas", upcoming_umas), ("outfits", upcoming_outfits), ("cards", upcoming_cards))
    }
    json.dump(out, open(os.path.join(OUT, "upcoming.json"), "w"))
    print("upcoming (dated after the build):", {k: len(v) for k, v in out.items()})
