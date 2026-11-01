"""The race model the solver's data.json is exported from: an era's G1 programs, each roster uma's runnable programs
and goals, and the win table. `RaceModel(era)` reads them; nothing here runs at import."""

import json
from dataclasses import dataclass
from typing import NamedTuple


def off(code):  # S,A ->0, B->1 ... G->6
    return max(0, 8 - code - 1)


def cat(distance):
    return 0 if distance <= 1400 else 1 if distance <= 1800 else 2 if distance <= 2400 else 3


YB = {1: [0], 2: [24], 3: [24, 48], 4: [48]}


def turn(year_base, month, half):
    return year_base + (month - 1) * 2 + half


@dataclass
class RaceProgram:
    """A scheduled race from master.mdb; distance is in metres, month/half are one-based."""

    base_program: int
    permission: int  # race_permission, indexing YB's eligible years
    month: int
    half: int
    race: int
    grade: int
    distance: int
    ground: int  # 1 turf, 2 dirt


class GoalRace(NamedTuple):
    """One permitted program for a career goal, at a one-based turn."""

    turn: int
    program: int


class RaceOptions(NamedTuple):
    """Runnable programs by turn and goal groups remapped onto those programs."""

    programs: dict[int, list[int]]
    goals: list[list[GoalRace]]


# Win odds (community table, Oct 2026): base = 110 - cost(distance grade) - cost(surface grade),
# minus the streak penalty for the k-th race in a row; clamp 0..100.
# Grade codes 1..8 = G..S. Cost: S/A 0, B 10, C 20, D 30, E 40, F 60, G 90.
PEN = {1: 0, 2: 0, 3: 5, 4: 20, 5: 30, 6: 50}
GRADE_COST = {8: 0, 7: 0, 6: 10, 5: 20, 4: 30, 3: 40, 2: 60, 1: 90}


def pwin(base_chance, k):
    return max(0, min(100, base_chance - PEN[min(k, 6)])) / 100


JBC = {20002: 1025, 20003: 1026}  # race_type-4 goals: JBC Sprint / JBC Classic base programs (assumed mapping)
# Relocated venues run the same race: these race ids are read as the race they stand in for (alias_venues)
VENUE_ALIASES = {1025: 1012, 1027: 1006, 1028: 1005, 1029: 1016}


class RaceModel:
    """An era's race programs (`programs`: program id -> RaceProgram), its G1 calendar (`g1_calendar`: turn -> program
    ids), the roster with names and aptitudes from base.json, race names, and the turns each uma may not race"""

    def __init__(self, era):
        self.era = era
        self.race_names = era.names(32)
        base = json.load(open(era.base))
        self.roster = base["roster"]
        self.name = {int(k): v for k, v in base["name"].items()}
        # distance, surface, then running-style grades; base.json order, codes 1..8 = G..S
        self.apt = {int(k): v for k, v in base["apt"].items()}
        # per uma, [card id, aptitudes] of her outfits whose own aptitudes differ from apt's (build_base.py)
        self.outfit_apt = {int(k): v for k, v in base.get("outfit_apt", {}).items()}
        self.programs = {}
        for (
            program_id,
            base_program,
            permission,
            month,
            half,
            race_id,
            grade,
            distance,
            ground,
        ) in era.q("""select p.id,p.base_program_id,p.race_permission,p.month,p.half,r.id,r.grade,cs.distance,cs.ground
 from single_mode_program p join race_instance ri on p.race_instance_id=ri.id join race r on ri.race_id=r.id join race_course_set cs on r.course_set=cs.id"""):
            self.programs[program_id] = RaceProgram(
                base_program=base_program,
                permission=permission,
                month=month,
                half=half,
                race=race_id,
                grade=grade,
                distance=distance,
                ground=ground,
            )
        self.g1_calendar = {}  # turn -> program IDs
        for program_id, program in self.programs.items():
            if program.grade == 100 and program.base_program == 0 and 1 <= program.permission <= 4:
                for year_base in YB[program.permission]:
                    self.g1_calendar.setdefault(turn(year_base, program.month, program.half), []).append(program_id)
        # turns a career forbids racing (single_mode_race_restrict_turn): gain_id names the story event that sets it
        # (1078 118 11 is story 501078118). Yamanin Zephyr turn 44 (Classic late October: Tenno Sho (Autumn), Shuka
        # Sho, Kikuka Sho) and Sweep Tosho turn 36 (Classic late June: Takarazuka Kinen) are fixed story events that
        # rest her (+20 energy), so she races nothing. Matikanetannhauser's turn 70 is the Japan Cup goal the event
        # assigns her (route row determine_race 4), so only the goal race stays on a restricted turn.
        self.restrict = {}
        for character_id, turn_number in era.q("select chara_id, turn from single_mode_race_restrict_turn"):
            self.restrict.setdefault(character_id, set()).add(turn_number)

    def alias_venues(self):
        """reads each relocated venue's race id as the race it stands in for (VENUE_ALIASES), in every program"""
        for program in self.programs.values():
            program.race = VENUE_ALIASES.get(program.race, program.race)

    def basep(self, character_id, program_id, apt=None):
        aptitudes = apt or self.apt[character_id]
        program = self.programs[program_id]
        distance_cost = GRADE_COST[aptitudes[cat(program.distance)]]
        surface_cost = GRADE_COST[aptitudes[4] if program.ground == 1 else aptitudes[5]]
        return max(
            -20, 110 - distance_cost - surface_cost
        )  # floor keeps the Rust lookup table in range; anything <= 0 is a 0% win

    def goals(self, character_id, exclude_det2=True):
        """her race goals (condition_type 1; fan-count and placement goals, condition_type 3 and 2, are not exported),
        one list of permitted programs per goal. A goal over several turns is determine_race 3 (asserted); the solver
        flags every one of its races a goal race (docs/MODEL.md: the run records do not fit one reading of these
        groups)"""
        rows = self.era.q(
            "select sort_id,turn,condition_id,determine_race from single_mode_route_race where race_set_id=? and scenario_group_id=100 and condition_type=1 and turn<=72 order by sort_id,id",
            character_id,
        )
        group: dict[int, list[GoalRace]] = {}
        determinations = {}
        for sort_id, turn_number, program_id, determination in rows:
            if exclude_det2 and determination == 2:
                continue
            group.setdefault(sort_id, []).append(GoalRace(turn=turn_number, program=program_id))
            determinations.setdefault(sort_id, set()).add(determination)
        for sort_id, races in group.items():
            # (with the determine_race 2 alternates kept, a goal's alternate may sit on another turn)
            if exclude_det2 and len({race.turn for race in races}) > 1:
                assert determinations[sort_id] == {3}, (self.name[character_id], sort_id, determinations[sort_id])
        return list(group.values())

    def is_g1(self, program_id):
        return self.programs[program_id].grade == 100

    def goals2(self, character_id, exclude_det2=True):
        goal_groups = []
        for group in self.goals(character_id, exclude_det2=exclude_det2):
            goal_groups.append(
                [GoalRace(turn=turn_number, program=JBC.get(program, program)) for turn_number, program in group]
            )
        return goal_groups

    def options(self, character_id, exclude_det2=True):
        """turn -> list of programs runnable (one program per (turn,race)); goal groups remapped to those programs; only goal
        races at a turn the career forbids racing (restrict)"""
        goal_groups = self.goals2(character_id, exclude_det2=exclude_det2)
        programs_by_turn = {
            turn_number: list(v)
            for turn_number, v in self.g1_calendar.items()
            if turn_number not in self.restrict.get(character_id, ())
        }
        canonical_programs = {}
        for group in goal_groups:
            for turn_number, program in group:
                programs_by_turn.setdefault(turn_number, [])
                same = [
                    x for x in programs_by_turn[turn_number] if self.programs[x].race == self.programs[program].race
                ]
                if (
                    same
                    and same[0] in self.g1_calendar.get(turn_number, [])
                    and program not in self.g1_calendar.get(turn_number, [])
                ):
                    programs_by_turn[turn_number] = [x for x in programs_by_turn[turn_number] if x != same[0]]
                    programs_by_turn[turn_number].append(program)
                    canonical_programs[(turn_number, self.programs[program].race)] = program
                elif same:
                    canonical_programs.setdefault((turn_number, self.programs[program].race), same[0])
                else:
                    programs_by_turn[turn_number].append(program)
                    canonical_programs[(turn_number, self.programs[program].race)] = program
        remapped_goals = []
        for group in goal_groups:
            remapped_group = []
            for turn_number, program in group:
                canonical_program = canonical_programs.get((turn_number, self.programs[program].race), program)
                if (turn_number, canonical_program) not in remapped_group:
                    remapped_group.append(GoalRace(turn=turn_number, program=canonical_program))
            remapped_goals.append(remapped_group)
        return RaceOptions(programs=programs_by_turn, goals=remapped_goals)
