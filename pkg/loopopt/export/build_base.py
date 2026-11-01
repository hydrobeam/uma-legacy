"""The era's roster (released by the cut: era.py; has a trainee card) and aptitudes -> base.json (codes 1..8 = G..S:
short, mile, middle, long, turf, dirt, then front runner, pace chaser, late surger, end closer).
usage: build_base.py global|jp [CHARA:CARD ...]
`apt`: each uma's aptitudes from her first outfit (the lowest released card id) unless a CHARA:CARD argument names
another, e.g. 1008:100802 for [Fiery Aqua Vitae] Vodka. `outfit_apt`: per uma, [card id, aptitudes] of each released
outfit whose own aptitudes differ from `apt` (the career planner grades a career by the outfit it runs; the loop
planner keeps `apt`)."""

import argparse, json

import era as eras


def outfit_choice(text):
    """CHARA:CARD -> (character id, outfit card id)"""
    character, card = text.split(":")
    return int(character), int(card)


def outfit_aptitudes(era, card_id):
    """an outfit's ten aptitude codes (its lowest rarity's row; no outfit's aptitudes vary by rarity, asserted)"""
    rows = era.q(
        "select distinct proper_distance_short,proper_distance_mile,proper_distance_middle,proper_distance_long,proper_ground_turf,proper_ground_dirt,proper_running_style_nige,proper_running_style_senko,proper_running_style_sashi,proper_running_style_oikomi from card_rarity_data where card_id=?",
        card_id,
    )
    assert len(rows) == 1, f"card {card_id}: aptitudes vary by rarity: {rows}"
    return list(rows[0])


def main():
    parser = argparse.ArgumentParser(description="The era's roster and aptitudes -> base.json.", allow_abbrev=False)
    eras.add_argument(parser)
    parser.add_argument(
        "outfits",
        nargs="*",
        type=outfit_choice,
        metavar="CHARA:CARD",
        help="take an uma's aptitudes from this outfit, e.g. 1008:100802 for [Fiery Aqua Vitae] Vodka",
    )
    arguments = parser.parse_args()
    era = eras.Era(arguments.era)
    character_names = era.names(6)
    outfits = lambda character_id: [
        x
        for (x,) in era.q("select id from card_data where chara_id=? and id<900000 order by id", character_id)
        if era.outfit_released(x)
    ]
    roster = [r[0] for r in era.q("select id from chara_data where start_date<?", era.cut) if outfits(r[0])]
    outfit_overrides = dict(arguments.outfits)
    aptitudes = {}
    own_aptitudes = {}
    for character_id in roster:
        card_id = outfit_overrides.get(character_id) or outfits(character_id)[0]
        assert era.q("select chara_id from card_data where id=?", card_id)[0][0] == character_id, (
            f"card {card_id} is not chara {character_id}"
        )
        aptitudes[character_id] = outfit_aptitudes(era, card_id)
        differing = [[card, outfit_aptitudes(era, card)] for card in outfits(character_id)]
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
        open(era.base, "w"),
    )
    print(
        len(roster),
        "umas",
        "outfit overrides:",
        outfit_overrides or "none",
        "outfits with their own aptitudes:",
        sorted(card for entries in own_aptitudes.values() for card, _ in entries),
    )


if __name__ == "__main__":
    main()
