"""Export everything the Rust core needs to the era's data.json. Relocated venues = same race (samerace patch).
usage: export_rs.py global|jp [--out FILE]"""

import json
import os
import sys
from typing import NamedTuple

# Importing samerace applies the venue aliases before any program is exported.
from samerace import PROG
from model import G1CAL, ROSTER, NAME, APT, RN, PEN, options, is_g1, basep, cat
from rel import pt, mem
from era import OUT


class RaceAttributes(NamedTuple):
    """A race's distance class (0 short .. 3 long) and surface (1 turf, 2 dirt)."""

    distance_class: int
    ground: int


def collect_races():
    races = {PROG[program].race for programs in G1CAL.values() for program in programs}
    # Career objectives can be G1s outside the standard calendar programs.
    for character in ROSTER:
        runnable = options(character)
        races |= {
            PROG[program].race for programs in runnable.programs.values() for program in programs if is_g1(program)
        }
    return sorted(races)


def export_uma(character, *, races, race_indices):
    runnable = options(character)
    turns = []
    for turn in range(1, 73):
        choices = []
        for program in runnable.programs.get(turn, []):
            is_grade_one = is_g1(program)
            race = PROG[program].race if is_grade_one else None
            if is_grade_one:
                assert race in race_indices, (NAME[character], turn, program, race)
            choices.append(
                {
                    "prog": program,
                    "race": race_indices[race] if is_grade_one else -1,
                    "bp": basep(character, program) if is_grade_one else 0,
                    "name": RN.get(PROG[program].race, str(program)),
                    "goal": any((turn, program) in group for group in runnable.goals),
                }
            )
        turns.append(choices)
    groups = []
    for group in runnable.goals:
        groups.append(
            [[turn, [choice["prog"] for choice in turns[turn - 1]].index(program)] for turn, program in group]
        )
    # Each race appears at most once through Classic year, and once in Senior year.
    seen = {}
    for turn, choices in enumerate(turns, 1):
        for choice in choices:
            if choice["race"] >= 0:
                key = (choice["race"], turn <= 48)
                assert key not in seen, (NAME[character], RN[races[choice["race"]]])
                seen[key] = turn
    # Ten aptitude codes in base.json order; bp above uses the first six.
    return {"id": character, "name": NAME[character], "turns": turns, "goals": groups, "apt": list(APT[character])}


def export_relations():
    """[points, roster indices] for each relation type with at least two distinct umas.

    The solver rebuilds pair/trio tables from these groups (lib.rs rel_tables).
    """
    relations = []
    for relation_type in sorted(pt):
        members = [index for index, character in enumerate(ROSTER) if relation_type in mem.get(character, set())]
        if len(members) >= 2:
            relations.append([pt[relation_type], members])
    return relations


races = collect_races()
race_indices = {race: index for index, race in enumerate(races)}
umas = [export_uma(character, races=races, race_indices=race_indices) for character in ROSTER]
relations = export_relations()
race_attributes = {}
for program in PROG.values():
    race_attributes.setdefault(
        program.race, RaceAttributes(distance_class=cat(program.distance), ground=program.ground)
    )
output_path = sys.argv[sys.argv.index("--out") + 1] if "--out" in sys.argv else os.path.join(OUT, "data.json")
json.dump(
    {
        "races": races,
        "race_names": [RN[race] for race in races],
        "race_cat": [race_attributes[race].distance_class for race in races],
        "race_ground": [race_attributes[race].ground for race in races],
        "umas": umas,
        "relations": relations,
        "pen": [PEN[streak] for streak in range(1, 7)],
    },
    open(output_path, "w"),
)
print(
    "races",
    len(races),
    "umas",
    len(umas),
    "max options/turn",
    max(len(choices) for uma in umas for choices in uma["turns"]),
    "multi-turn goal groups",
    sum(1 for uma in umas for group in uma["goals"] if len({turn for turn, _ in group}) > 1),
    "same-turn multi-option groups",
    sum(1 for uma in umas for group in uma["goals"] if len(group) > 1 and len({turn for turn, _ in group}) == 1),
)
print(
    [(uma["name"], uma["goals"]) for uma in umas if any(len({turn for turn, _ in group}) > 1 for group in uma["goals"])]
)
