"""Support cards for deck search -> the era's web_cards.json. usage: export_cards.py global|jp
Per released card: chara, rarity, pal (friend type), chain length (story events with progress markers),
hint frequency at max limit break (effect 18, plus the unique effect's), number of distinct training-hint options (skills + stat option) and of skill options alone (nskill),
hint levels per training-hint draw at max limit break (hinc: 1 + effect 17, plus the unique effect's),
and the skill groups (white level) in its training-hint pool. Event skill rewards are NOT in master.mdb tables;
they are supplied per job (`events`)."""

import json, os
from typing import NamedTuple
from era import q, names, OUT, released

card_names = names(75)
skill_groups = dict(q("select id, group_id from skill_data"))
out = []
# characters who cannot train with a card: its own character plus, for group cards, every member
members = {}
for card_id, character_id in q("select support_card_id, chara_id from support_card_group"):
    members.setdefault(card_id, set()).add(character_id)
KIND = {101: "speed", 105: "stamina", 102: "power", 103: "guts", 106: "wit"}


class SupportCard(NamedTuple):
    """Source card identity, release time (Unix seconds), and effect-table lookup IDs."""

    id: int
    character: int
    rarity: int
    type: int
    effect_table: int
    start_date: int
    command: int


def max_effect(effect_table, *, effect_type):
    return q(
        "select max(init,limit_lv5,limit_lv10,limit_lv15,limit_lv20,limit_lv25,limit_lv30,limit_lv35,limit_lv40,limit_lv45,limit_lv50) from support_card_effect_table where id=? and type=?",
        effect_table,
        effect_type,
    )


def unique_effect(card_id, *, effect_type):
    return sum(
        value
        for type_0, value_0, type_1, value_1 in q(
            "select type_0, value_0, type_1, value_1 from support_card_unique_effect where id=?", card_id
        )
        for kind, value in ((type_0, value_0), (type_1, value_1))
        if kind == effect_type
    )


def export_card(card):
    rows = q(
        "select hint_group,hint_gain_type,hint_value_1 from single_mode_hint_gain where support_card_id=?", card.id
    )
    hint_options = len({g for g, _, _ in rows})
    skill_options = len({g for g, t, _ in rows if t == 0})  # skill options (the stat option is never drawn in IT)
    whites = sorted({skill_groups[v] for g, t, v in rows if t == 0 and v in skill_groups})
    hint_frequency = max_effect(card.effect_table, effect_type=18)
    # unique effect (active at max limit break): +hint frequency on some cards (type 18 in either slot)
    unique_hint_frequency = unique_effect(card.id, effect_type=18)
    # hint levels per training-hint draw at max limit break: 1 + effect 17 (plus the unique effect's). A skill at Lv 5
    # leaves the card's pool, so a card draws one skill at most ceil(5 / hinc) times.
    hint_level = max_effect(card.effect_table, effect_type=17)
    unique_hint_level = unique_effect(card.id, effect_type=17)
    # specialty priority at max limit break (effect 19, plus the unique effect's): more training hints (hints.rs hint_spec)
    specialty = max_effect(card.effect_table, effect_type=19)
    unique_specialty = unique_effect(card.id, effect_type=19)
    links = q("select max(show_progress_2) from single_mode_story_data where support_card_id=?", card.id)[0][0] or 0
    return {
        "id": card.id,
        "name": card_names[card.id],
        "chara": card.character,
        "rarity": card.rarity,
        "pal": card.type in (2, 3),
        "links": links,
        "hf": (max(0, hint_frequency[0][0]) if hint_frequency else 0) + unique_hint_frequency,
        "spec": (max(0, specialty[0][0]) if specialty and specialty[0][0] is not None else 0) + unique_specialty,
        "opts": hint_options,
        "nskill": skill_options,
        "hinc": 1
        + (max(0, hint_level[0][0]) if hint_level and hint_level[0][0] is not None else 0)
        + unique_hint_level,
        "whites": whites,
        "charas": sorted(members.get(card.id, set()) | {card.character}),
        "kind": "pal" if card.type == 2 else "group" if card.type == 3 else KIND.get(card.command, "other"),
    }


for row in q(
    "select id,chara_id,rarity,support_card_type,effect_table_id,start_date,command_id from support_card_data"
):
    card = SupportCard(*row)
    if released(card.start_date):
        out.append(export_card(card))
# cards the game bans from a scenario's deck (single_mode_restrict_support: Team Sirius in Unity Cup and Grand
# Concert), keyed by the solver's scenario names (single_mode_scenario ids 1-5; later JP scenarios are out of scope)
SCENARIOS = {1: "ura", 2: "unity", 3: "gc", 4: "trackblazer", 5: "grandmasters"}
have = {c["id"] for c in out}
restrict = {}
for scenario, card_id in q(
    "select scenario_id, support_card_id from single_mode_restrict_support order by scenario_id, support_card_id"
):
    if scenario in SCENARIOS and card_id in have:
        restrict.setdefault(SCENARIOS[scenario], []).append(card_id)
json.dump(
    {"cards": out, "restrict": restrict},
    open(os.path.join(OUT, "web_cards.json"), "w"),
    ensure_ascii=False,
    separators=(",", ":"),
)
print(len(out), "cards;", sum(x["pal"] for x in out), "pals; restricted", restrict)
