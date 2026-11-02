"""Build a career-planner job from a veteran export (umadump's trained_chara_data.json), the way the career page
reads one: each veteran's pink sparks and white skill sparks (factor_id = factor group * 100 + stars, factor group -> skill group from
career.json), its single-race G1 wins (win saddle -> race index, career.json) and the same for its two parents
(succession_chara_array positions 10 and 20). A veteran counts as borrowed when owner_viewer_id is set: it is 0 on the
player's own veterans and the lender's viewer_id on the ones offered for borrowing (rental_chara_data.json).

usage: career_job.py TRAINED_CHARA_JSON BASE_JOB OUT_JOB [TABLES]
  BASE_JOB  a deck job file (files, targets, cards...); it becomes a career job on these veterans
  TABLES    career.json (default pkg/loopopt/data/global/career.json)"""

import argparse
import json
import os

REPO = os.path.dirname(os.path.abspath(__file__)) + "/../"
parser = argparse.ArgumentParser(
    description="Build a career-planner job from a veteran export (umadump's trained_chara_data.json).",
    allow_abbrev=False,
)
parser.add_argument("veterans", metavar="TRAINED_CHARA_JSON")
parser.add_argument(
    "base_job", metavar="BASE_JOB", help='a deck job (targets, cards, events files...); its mode becomes "career"'
)
parser.add_argument("out_job", metavar="OUT_JOB")
parser.add_argument(
    "tables",
    nargs="?",
    default=REPO + "pkg/loopopt/data/global/career.json",
    metavar="TABLES",
    help="career.json (default pkg/loopopt/data/global/career.json)",
)
arguments = parser.parse_args()
veterans, base_path, out_path = json.load(open(arguments.veterans)), arguments.base_job, arguments.out_job
tables = json.load(open(arguments.tables))
factor_groups, saddle_races = (
    {int(k): veteran for k, veteran in tables["factors"].items()},
    {int(k): veteran for k, veteran in tables["saddles"].items()},
)


def sparks(factors):
    sparks_by_group = {}
    for factor in factors:
        group = factor_groups.get(factor["factor_id"] // 100)
        if group is not None:
            sparks_by_group[group] = max(sparks_by_group.get(group, 0), factor["factor_id"] % 100)
    return [{"group": group, "stars": stars} for group, stars in sorted(sparks_by_group.items())]


pink_groups = {int(k) for k in tables.get("pinks", {})}


def pink(factors):
    """her one pink spark (every trained uma has one)"""
    found = [
        {"aptitude": factor["factor_id"] // 100, "stars": factor["factor_id"] % 100}
        for factor in factors
        if factor["factor_id"] // 100 in pink_groups
    ]
    if len(found) != 1:
        raise SystemExit(f"a veteran has one pink spark, not {len(found)}: {found}")
    return found[0]


def wins(saddles):
    return sorted({saddle_races[saddle] for saddle in saddles if saddle in saddle_races})


def ancestor(veteran):
    return {
        "chara": veteran["card_id"] // 100,
        "sparks": sparks(veteran["factor_info_array"]),
        "pink": pink(veteran["factor_info_array"]),
        "wins": wins(veteran["win_saddle_id_array"]),
    }


legacies = []
for veteran in veterans:
    legacy = ancestor(veteran)
    legacy["borrowed"] = bool(veteran.get("owner_viewer_id"))
    # her parents at positions 10 and 20, each null where the export has none
    by_position = {parent["position_id"]: parent for parent in veteran["succession_chara_array"]}
    legacy["parents"] = [ancestor(by_position[p]) if p in by_position else None for p in (10, 20)]
    legacies.append(legacy)
job = json.load(open(base_path))
job["job"] = {**job["job"], "cmd": "career", "career": {"legacies": legacies}}
json.dump(job, open(out_path, "w"))
print(len(legacies), "veterans,", sum(legacy["borrowed"] for legacy in legacies), "borrowed")
