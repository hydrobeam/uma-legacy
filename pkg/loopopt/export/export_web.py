"""Skill tables for the web app: each roster uma's outfits with the target-able skills on their own skill list
(need_rank <= 5, i.e. fully awakened), plus the catalog of white skills (and their gold upgrade) users can pick.
Writes the era's web_skills.json. usage: export_web.py global|jp"""

import json, os, re
from dataclasses import dataclass
from typing import NamedTuple
from era import c, names, BASE, OUT, outfit_released


@dataclass
class SkillNames:
    """The first named white and gold skill in one skill group."""

    white: str | None = None
    gold: str | None = None


class SkillGroup(NamedTuple):
    rarity: int
    group: int


q = lambda s, *a: c.execute(s, a).fetchall()
base = json.load(open(BASE))
roster = base["roster"]
skill_names = names(47)
outfit_names = names(5)
# inherited unique skills (unique_skill_id_1 set) come only from a parent's unique spark, never a hint: not targets
skill_groups = {
    i: SkillGroup(rarity=r, group=g)
    for i, r, g in q("select id, rarity, group_id from skill_data where unique_skill_id_1 = 0")
}
# catalog: groups with a white (rarity 1) skill that has a name; gold = rarity 2 in the same group
groups = {}
for i, (r, g) in skill_groups.items():
    if r in (1, 2) and i in skill_names:
        e = groups.setdefault(g, SkillNames())
        if r == 1 and e.white is None:
            e.white = skill_names[i]
        if r == 2 and e.gold is None:
            e.gold = skill_names[i]
catalog = sorted(
    [[g, names.white, names.gold] for g, names in groups.items() if names.white], key=lambda x: x[1].lower()
)
cards = {}
for card_id, character_id, skill_set_id in q(
    "select id, chara_id, available_skill_set_id from card_data where id<900000 order by id"
):
    if character_id not in roster or not outfit_released(card_id):
        continue
    coverage = {}
    for skill_id, rank in q(
        "select skill_id, need_rank from available_skill_set where available_skill_set_id=?", skill_set_id
    ):
        if rank > 5 or skill_id not in skill_groups:
            continue
        r, g = skill_groups[skill_id]
        if g in groups and r in (1, 2):
            coverage[g] = max(coverage.get(g, 0), 2 if r == 2 else 1)
    label = (
        re.match(r"(\[.*?\])", outfit_names.get(card_id, "")).group(1)
        if re.match(r"\[.*?\]", outfit_names.get(card_id, ""))
        else str(card_id)
    )
    cards.setdefault(character_id, []).append([card_id, label, sorted(coverage.items())])
json.dump({"cards": cards, "skills": catalog}, open(os.path.join(OUT, "web_skills.json"), "w"), ensure_ascii=False)
print(len(catalog), "skills,", sum(len(v) for v in cards.values()), "outfits for", len(cards), "umas")
