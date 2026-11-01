"""Export everything the Rust core needs to the era's data.json. Relocated venues = same race (model.VENUE_ALIASES).
usage: export_rs.py global|jp [--out FILE]"""

import argparse
import json
import os
from typing import NamedTuple

import era as eras
from model import PEN, RaceModel, cat
from rel import relation_points


class RaceAttributes(NamedTuple):
    """A race's distance class (0 short .. 3 long) and surface (1 turf, 2 dirt)."""

    distance_class: int
    ground: int


def collect_races(model):
    races = {model.programs[program].race for programs in model.g1_calendar.values() for program in programs}
    # Career objectives can be G1s outside the standard calendar programs.
    for character in model.roster:
        runnable = model.options(character)
        races |= {
            model.programs[program].race
            for programs in runnable.programs.values()
            for program in programs
            if model.is_g1(program)
        }
    return sorted(races)


def export_uma(model, character, *, races, race_indices):
    runnable = model.options(character)
    turns = []
    for turn in range(1, 73):
        choices = []
        for program in runnable.programs.get(turn, []):
            is_grade_one = model.is_g1(program)
            race = model.programs[program].race if is_grade_one else None
            if is_grade_one:
                assert race in race_indices, (model.name[character], turn, program, race)
            choices.append(
                {
                    "prog": program,
                    "race": race_indices[race] if is_grade_one else -1,
                    "bp": model.basep(character, program) if is_grade_one else 0,
                    "name": model.race_names.get(model.programs[program].race, str(program)),
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
                assert key not in seen, (model.name[character], model.race_names[races[choice["race"]]])
                seen[key] = turn
    # Ten aptitude codes in base.json order; bp above uses the first six. `outfit_apt`: [card id, aptitudes] of each
    # outfit whose own differ (the career planner's grades for a career in that outfit; bp stays the first outfit's)
    exported = {
        "id": character,
        "name": model.name[character],
        "turns": turns,
        "goals": groups,
        "apt": list(model.apt[character]),
    }
    if character in model.outfit_apt:
        exported["outfit_apt"] = model.outfit_apt[character]
    return exported


def export_relations(model):
    """[points, roster indices] for each relation type with at least two distinct umas.

    The solver rebuilds pair/trio tables from these groups (lib.rs rel_tables).
    """
    pt, mem = relation_points(model.era)
    relations = []
    for relation_type in sorted(pt):
        members = [index for index, character in enumerate(model.roster) if relation_type in mem.get(character, set())]
        if len(members) >= 2:
            relations.append([pt[relation_type], members])
    return relations


def main():
    parser = argparse.ArgumentParser(
        description="Export everything the Rust core needs to the era's data.json.", allow_abbrev=False
    )
    eras.add_argument(parser)
    parser.add_argument("--out", metavar="FILE", help="write here instead of the era's data.json")
    arguments = parser.parse_args()
    model = RaceModel(eras.Era(arguments.era))
    model.alias_venues()  # before any program is exported
    races = collect_races(model)
    race_indices = {race: index for index, race in enumerate(races)}
    umas = [export_uma(model, character, races=races, race_indices=race_indices) for character in model.roster]
    relations = export_relations(model)
    race_attributes = {}
    for program in model.programs.values():
        race_attributes.setdefault(
            program.race, RaceAttributes(distance_class=cat(program.distance), ground=program.ground)
        )
    output_path = os.path.join(model.era.out, "data.json") if arguments.out is None else arguments.out
    json.dump(
        {
            "races": races,
            "race_names": [model.race_names[race] for race in races],
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
        [
            (uma["name"], uma["goals"])
            for uma in umas
            if any(len({turn for turn, _ in group}) > 1 for group in uma["goals"])
        ]
    )


if __name__ == "__main__":
    main()
