"""The era's roster (released by the cut: era.py; has a trainee card) and aptitudes -> base.json (codes 1..8 = G..S:
short, mile, middle, long, turf, dirt, then front runner, pace chaser, late surger, end closer).
usage: build_base.py global|jp [CHARA:CARD ...]
`apt`: each uma's aptitudes from her first outfit (the lowest released card id) unless a CHARA:CARD argument names
another, e.g. 1008:100802 for [Fiery Aqua Vitae] Vodka. `outfit_apt`: per uma, [card id, aptitudes] of each released
outfit whose own aptitudes differ from `apt` (the career planner grades a career by the outfit it runs; the loop
planner keeps `apt`)."""

import json, sys
from era import q, names, CUT, BASE, outfit_released

character_names = names(6)
outfits = lambda character_id: [
    x
    for (x,) in q("select id from card_data where chara_id=? and id<900000 order by id", character_id)
    if outfit_released(x)
]
roster = [r[0] for r in q("select id from chara_data where start_date<?", CUT) if outfits(r[0])]
outfit_overrides = {int(a): int(b) for a, b in (x.split(":") for x in sys.argv[2:] if ":" in x)}


def outfit_aptitudes(card_id):
    """an outfit's ten aptitude codes (its lowest rarity's row; no outfit's aptitudes vary by rarity, asserted)"""
    rows = q(
        "select distinct proper_distance_short,proper_distance_mile,proper_distance_middle,proper_distance_long,proper_ground_turf,proper_ground_dirt,proper_running_style_nige,proper_running_style_senko,proper_running_style_sashi,proper_running_style_oikomi from card_rarity_data where card_id=?",
        card_id,
    )
    assert len(rows) == 1, f"card {card_id}: aptitudes vary by rarity: {rows}"
    return list(rows[0])


aptitudes = {}
own_aptitudes = {}
for character_id in roster:
    card_id = outfit_overrides.get(character_id) or outfits(character_id)[0]
    assert q("select chara_id from card_data where id=?", card_id)[0][0] == character_id, (
        f"card {card_id} is not chara {character_id}"
    )
    aptitudes[character_id] = outfit_aptitudes(card_id)
    differing = [[card, outfit_aptitudes(card)] for card in outfits(character_id)]
    differing = [entry for entry in differing if entry[1] != aptitudes[character_id]]
    if differing:
        own_aptitudes[character_id] = differing
json.dump(
    {
        "roster": roster,
        "apt": aptitudes,
        "outfit_apt": own_aptitudes,
        "name": {k: character_names[k] for k in roster},
    },
    open(BASE, "w"),
)
print(
    len(roster),
    "umas",
    "outfit overrides:",
    outfit_overrides or "none",
    "outfits with their own aptitudes:",
    sorted(card for entries in own_aptitudes.values() for card, _ in entries),
)
