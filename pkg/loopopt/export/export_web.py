"""Skill tables for the web app: each roster uma's outfits with the target-able skills on their own skill list
(need_rank <= 5, i.e. fully awakened), plus the catalog of white skills (and their gold upgrade) users can pick.
Writes the era's web_skills.json. usage: export_web.py global|jp"""

import json, os, re
from dataclasses import dataclass
from typing import NamedTuple
from era import c, names, BASE, OUT, outfit_released


@dataclass
class SkillNames:
    """A skill group's catalog names: its white and its first named gold."""

    white: str | None = None
    white_rank: tuple[int, int] = (2, 0)  # the white's place among the group's whites (lower is preferred)
    gold: str | None = None


class SkillGroup(NamedTuple):
    rarity: int
    group: int
    rate: int  # group_rate: a white's place in its group (○ 1, ◎ 2; × negative)


q = lambda s, *a: c.execute(s, a).fetchall()
base = json.load(open(BASE))
roster = base["roster"]
skill_names = names(47)
outfit_names = names(5)
# inherited unique skills (unique_skill_id_1 set) come only from a parent's unique spark, never a hint: not targets
skill_groups = {
    i: SkillGroup(rarity=r, group=g, rate=rate)
    for i, r, g, rate in q(
        "select id, rarity, group_id, group_rate from skill_data where unique_skill_id_1 = 0 order by id"
    )
}
# catalog: groups with a white (rarity 1) skill that has a name; gold = rarity 2 in the same group. A group's white is
# named by its lowest non-negative rate: the ○ where a group has ○ and ◎ (what card hints and sparks give: every
# training hint and white spark on such a group names the ○), else its lowest id (a group of one × keeps the ×)
groups = {}
for i, (r, g, rate) in skill_groups.items():
    if r in (1, 2) and i in skill_names:
        e = groups.setdefault(g, SkillNames())
        rank = (0, rate) if rate >= 0 else (1, 0)
        if r == 1 and (e.white is None or rank < e.white_rank):
            e.white, e.white_rank = skill_names[i], rank
        if r == 2 and e.gold is None:
            e.gold = skill_names[i]
catalog = sorted(
    [[g, names.white, names.gold] for g, names in groups.items() if names.white], key=lambda x: x[1].lower()
)
# every name an event reward can carry -> [group, 1 white / 2 gold]: the catalog's own names, plus a group's other
# whites at a non-negative rate (the ◎ beside a catalog ○: one group, one white target; a × is a different skill)
reward_names = {}
for g, white, gold in catalog:
    reward_names[white] = [g, 1]
    if gold:
        reward_names[gold] = [g, 2]
for i, (r, g, rate) in skill_groups.items():
    if r == 1 and rate >= 0 and g in groups and groups[g].white and i in skill_names:
        reward_names.setdefault(skill_names[i], [g, 1])
reward_names = dict(sorted(reward_names.items()))
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
        r, g, _ = skill_groups[skill_id]
        if g in groups and r in (1, 2):
            coverage[g] = max(coverage.get(g, 0), 2 if r == 2 else 1)
    label = (
        re.match(r"(\[.*?\])", outfit_names.get(card_id, "")).group(1)
        if re.match(r"\[.*?\]", outfit_names.get(card_id, ""))
        else str(card_id)
    )
    cards.setdefault(character_id, []).append([card_id, label, sorted(coverage.items())])
json.dump(
    {"cards": cards, "skills": catalog, "names": reward_names},
    open(os.path.join(OUT, "web_skills.json"), "w"),
    ensure_ascii=False,
)
print(len(catalog), "skills,", sum(len(v) for v in cards.values()), "outfits for", len(cards), "umas")
