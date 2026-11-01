"""Write a synthetic large veteran pool as a val/plan job: the planner at the scale of a real roster (export 5, the
maintainer's case, had 360 veterans, 75 of them borrowed), which the hand-built val/plan pools (6-17 veterans) never
reach. It is a scale test: its veterans are recombined from real records, not drawn from a model of real rosters.

Templates: the distinct veterans of the hand-built Global pools (val/plan's, at most HAND_BUILT veterans, so no
pool this script wrote, and val/jobs' career jobs). Each veteran takes
- a character drawn from the Global roster, and two parent characters drawn apart from hers and from each other (an
  unknown parent, character 0, stays unknown);
- one template's sparks and her parents' records (sparks, pinks, wins: inherited together, so kept together), from the
  strong templates (at least STRONG_SPARKS of the base job's targets sparked) for a borrowed veteran (assumed:
  lent veterans are strong ones), else from every template;
- another template's G1 wins and a third's pinks.
Seeded: the same arguments and templates write the same job.

usage: uv run val/make_pool.py [--size N] [--borrowed K] [--seed S] [--base JOB] [--plain] OUT
  defaults: 360 veterans, 75 borrowed, seed 1, base val/plan/p16.json (its targets and settings); the job asks for the
  robustness check and the pair search (`next`: robust "all", pair_search), as the page's check job does, unless
  --plain. OUT: val/plan/p360_check.json for the val/plan job."""

import argparse
import json
import os
import random

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


def templates(base_targets):
    """(every distinct template veteran, the strong ones), in job order, from the hand-built pools only"""
    seen, veterans = set(), []
    for path in template_jobs():
        pool = json.load(open(path))["legacies"]
        if len(pool) > HAND_BUILT:
            continue
        for veteran in pool:
            record = {key: veteran[key] for key in ("sparks", "pinks", "wins", "parents")}
            key = json.dumps(record, sort_keys=True)
            if key not in seen:
                seen.add(key)
                veterans.append(record)
    strong = [v for v in veterans if sum(group in base_targets for group, _stars in v["sparks"]) >= STRONG_SPARKS]
    return veterans, strong


def veteran(sparks_from, wins_from, pinks_from, roster, rng, borrowed):
    """a veteran of a fresh character: `sparks_from`'s sparks and parents' records (her parents under fresh characters
    apart from hers), `wins_from`'s wins, `pinks_from`'s pinks"""
    chara = rng.choice(roster)
    others = rng.sample(
        [c for c in roster if c != chara], len(sparks_from["parents"])
    )  # (a record may list fewer than two)
    parents = [
        {**parent, "chara": parent["chara"] and other}
        for parent, other in zip(sparks_from["parents"], others, strict=True)
    ]
    return {
        "chara": chara,
        "sparks": sparks_from["sparks"],
        "pinks": pinks_from["pinks"],
        "wins": wins_from["wins"],
        "borrowed": borrowed,
        "parents": parents,
    }


def main():
    parser = argparse.ArgumentParser(description="Write a synthetic large veteran pool as a val/plan job.")
    parser.add_argument("out", metavar="OUT")
    parser.add_argument("--size", type=int, default=360)
    parser.add_argument("--borrowed", type=int, default=75)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--base", default=os.path.join(PLAN_DIR, "p16.json"))
    parser.add_argument("--plain", action="store_true", help="no robustness check or pair search in `next`")
    arguments = parser.parse_args()
    base = json.load(open(arguments.base))
    base_targets = {target["group"] for target in base["targets"]}
    veterans, strong = templates(base_targets)
    roster = json.load(open(os.path.join(REPO, "pkg", "loopopt", "data", "global", "base.json")))["roster"]
    rng = random.Random(arguments.seed)
    borrowed = set(rng.sample(range(arguments.size), arguments.borrowed))
    legacies = [
        veteran(
            sparks_from=rng.choice(strong if i in borrowed else veterans),
            wins_from=rng.choice(veterans),
            pinks_from=rng.choice(veterans),
            roster=roster,
            rng=rng,
            borrowed=i in borrowed,
        )
        for i in range(arguments.size)
    ]
    plan = {} if arguments.plain else {"robust": "all", "pair_search": True}
    json.dump({**base, "legacies": legacies, "next": plan}, open(arguments.out, "w"))
    print(
        f"{arguments.out}: {len(legacies)} veterans ({len(borrowed)} borrowed) from {len(veterans)} templates "
        f"({len(strong)} strong), {len(roster)} characters"
    )


if __name__ == "__main__":
    main()
