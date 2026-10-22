"""Build a career-planner job from a veteran export (umadump's trained_chara_data.json), the way the career page
reads one: each veteran's pink sparks and white skill sparks (factor_id = factor group * 100 + stars, factor group -> skill group from
career.json), its single-race G1 wins (win saddle -> race index, career.json) and the same for its two parents
(succession_chara_array positions 10 and 20). A veteran counts as borrowed when owner_viewer_id is set: it is 0 on the
player's own veterans and the lender's viewer_id on the ones offered for borrowing (rental_chara_data.json).

usage: career_job.py TRAINED_CHARA_JSON BASE_JOB OUT_JOB [TABLES]
  BASE_JOB  a deck job (targets, cards, events files...); its mode becomes "career"
  TABLES    career.json (default pkg/loopopt/data/global/career.json)"""

import json
import sys
import os

REPO = os.path.dirname(os.path.abspath(__file__)) + "/../"
veterans, base_path, out_path = json.load(open(sys.argv[1])), sys.argv[2], sys.argv[3]
tables = json.load(open(sys.argv[4] if len(sys.argv) > 4 else REPO + "pkg/loopopt/data/global/career.json"))
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
    return sorted(sparks_by_group.items())


pink_groups = {int(k) for k in tables.get("pinks", {})}


def pinks(factors):
    return sorted(
        {
            factor["factor_id"] // 100: factor["factor_id"] % 100
            for factor in factors
            if factor["factor_id"] // 100 in pink_groups
        }.items()
    )


def wins(saddles):
    return sorted({saddle_races[saddle] for saddle in saddles if saddle in saddle_races})


def ancestor(veteran):
    return {
        "chara": veteran["card_id"] // 100,
        "sparks": sparks(veteran["factor_info_array"]),
        "pinks": pinks(veteran["factor_info_array"]),
        "wins": wins(veteran["win_saddle_id_array"]),
    }


legacies = []
for veteran in veterans:
    legacy = ancestor(veteran)
    legacy["borrowed"] = bool(veteran.get("owner_viewer_id"))
    legacy["parents"] = [
        ancestor(parent)
        for parent in sorted(veteran["succession_chara_array"], key=lambda parent: parent["position_id"])
        if parent["position_id"] in (10, 20)
    ]
    legacies.append(legacy)
job = json.load(open(base_path))
job["mode"] = "career"
job["legacies"] = legacies
json.dump(job, open(out_path, "w"))
print(len(legacies), "veterans,", sum(legacy["borrowed"] for legacy in legacies), "borrowed")
