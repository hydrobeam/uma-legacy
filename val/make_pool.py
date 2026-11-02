"""Write a synthetic large veteran pool as a val/plan job: the planner at the scale of a real roster (export 5, the
maintainer's case, had 360 veterans, 75 of them borrowed), which the hand-built val/plan pools (6-17 veterans) never
reach. It is a scale test: its veterans are recombined from real records, not drawn from a model of real rosters.

Templates: the distinct veterans of the hand-built Global pools (val/plan's, at most HAND_BUILT veterans, so no
pool this script wrote, and val/jobs' career jobs). Each veteran takes
- a character drawn from the Global roster, and two parent characters drawn apart from hers and from each other (an
  unknown parent, null, stays unknown);
- one template's sparks and her parents' records (sparks, pink, wins: inherited together, so kept together). FULL
  veterans (default 1) take a full template, every base target sparked; the others a template short of at least one
  target, a borrowed one's from the strong ones (at least STRONG_SPARKS sparked; assumed: lent veterans are strong
  ones). The hand-built pools hold many full records: drawn freely they gave 101 full veterans of 360, and with 8
  V's trio still stood on two of them (V about 78, the finish alone), so the plan had nothing to build; the real
  roster above had V's trio on one real side and one still building;
- another template's G1 wins and a third's pink.
Seeded: the same arguments and templates write the same job.

usage: uv run val/make_pool.py [--size N] [--borrowed K] [--full F] [--seed S] [--base JOB] [--plain] OUT
  defaults: 360 veterans, 75 borrowed, 1 full, seed 1, base val/plan/p16.json (its targets and settings); the job asks for the
  robustness check and the pair search (`next`: robust "all", pair_search), as the page's check job does, unless
  --plain. OUT: val/plan/p360_check.json for the val/plan job."""

import argparse
import json
import os
import random
from typing import NamedTuple

VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__))
PLAN_DIR = os.path.join(VALIDATION_DIR, "plan")
REPO = os.path.dirname(VALIDATION_DIR)
HAND_BUILT = 20  # veterans: a template pool has at most this many (the hand-built ones have 6-17)
STRONG_SPARKS = 6  # of the base job's targets: a borrowed veteran's sparks come from a template with this many


def template_jobs():
    """the Global job files whose veterans serve as templates: val/plan's pools, then val/jobs' career jobs"""
    plan = [
        os.path.join(PLAN_DIR, name + ".json")
        for era, name in json.load(open(os.path.join(PLAN_DIR, "jobs.json")))
        if era == "global"
    ]
    career = [
        os.path.join(VALIDATION_DIR, "jobs", name + ".json")
        for era, name in json.load(open(os.path.join(VALIDATION_DIR, "jobs.json")))
        if era == "global" and name.startswith("career_")
    ]
    return plan + career


class Templates(NamedTuple):
    """The distinct template records, in job order, by how many of the base job's targets they spark."""

    full: list  # every target
    partial: list  # not every target
    strong: list  # not every target, at least STRONG_SPARKS
    every: list  # all of them: the G1 wins' and pinks' sources


def templates(base_targets):
    """the `Templates` from the hand-built pools only"""
    seen, veterans = set(), []
    for path in template_jobs():
        pool = json.load(open(path))["job"]["career"]["legacies"]
        if len(pool) > HAND_BUILT:
            continue
        for veteran in pool:
            record = {key: veteran[key] for key in ("sparks", "pink", "wins", "parents")}
            key = json.dumps(record, sort_keys=True)
            if key not in seen:
                seen.add(key)
                veterans.append(record)
    sparked = lambda record: sum(spark["group"] in base_targets for spark in record["sparks"])
    partial = [v for v in veterans if sparked(v) < len(base_targets)]
    return Templates(
        full=[v for v in veterans if sparked(v) == len(base_targets)],
        partial=partial,
        strong=[v for v in partial if sparked(v) >= STRONG_SPARKS],
        every=veterans,
    )


def veteran(sparks_from, wins_from, pinks_from, roster, rng, borrowed):
    """a veteran of a fresh character: `sparks_from`'s sparks and parents' records (her parents under fresh characters
    apart from hers; an unknown one, null, stays unknown), `wins_from`'s wins, `pinks_from`'s pink"""
    chara = rng.choice(roster)
    others = rng.sample([c for c in roster if c != chara], 2)
    parents = [
        None if parent is None else {**parent, "chara": other}
        for parent, other in zip(sparks_from["parents"], others, strict=True)
    ]
    return {
        "chara": chara,
        "sparks": sparks_from["sparks"],
        "pink": pinks_from["pink"],
        "wins": wins_from["wins"],
        "borrowed": borrowed,
        "parents": parents,
    }


def main():
    parser = argparse.ArgumentParser(description="Write a synthetic large veteran pool as a val/plan job.")
    parser.add_argument("out", metavar="OUT")
    parser.add_argument("--size", type=int, default=360)
    parser.add_argument("--borrowed", type=int, default=75)
    parser.add_argument("--full", type=int, default=1, help="veterans with every base target sparked")
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--base", default=os.path.join(PLAN_DIR, "p16.json"))
    parser.add_argument("--plain", action="store_true", help="no robustness check or pair search in `next`")
    arguments = parser.parse_args()
    base = json.load(open(arguments.base))
    base_targets = {target["group"] for target in base["job"]["skills"]["targets"]}
    records = templates(base_targets)
    roster = json.load(open(os.path.join(REPO, "pkg", "loopopt", "data", "global", "base.json")))["roster"]
    rng = random.Random(arguments.seed)
    borrowed = set(rng.sample(range(arguments.size), arguments.borrowed))
    full = set(rng.sample(range(arguments.size), arguments.full))
    sparks_source = lambda i: records.full if i in full else records.strong if i in borrowed else records.partial
    legacies = [
        veteran(
            sparks_from=rng.choice(sparks_source(i)),
            wins_from=rng.choice(records.every),
            pinks_from=rng.choice(records.every),
            roster=roster,
            rng=rng,
            borrowed=i in borrowed,
        )
        for i in range(arguments.size)
    ]
    plan = {} if arguments.plain else {"robust": "all", "pair_search": True}
    career = {**base["job"]["career"], "legacies": legacies, "next": plan}
    json.dump({**base, "job": {**base["job"], "career": career}}, open(arguments.out, "w"))
    print(
        f"{arguments.out}: {len(legacies)} veterans ({len(borrowed)} borrowed, {len(full)} full) from "
        f"{len(records.every)} templates ({len(records.full)} full, {len(records.strong)} strong of the rest), "
        f"{len(roster)} characters"
    )


if __name__ == "__main__":
    main()
