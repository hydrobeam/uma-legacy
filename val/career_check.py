"""Independent check of the career reports (career mode: a job's `results`, and a plan's `next.options` and `choice`),
recomputed from the job and data.json alone, beside the solver's Rust:

- the six legacy slots: each slot's character, relation points and race affinity (docs/RESEARCH.md section 5: a parent
  P counts pair(T, P), pair(P, Q), and per own parent G the trio (T, P, G); her race part 3 per G1 she shares with Q
  and with each G; a grandparent G under P counts trio(T, P, G) and 3 per G1 shared with P; an unknown character
  counts no points), and their sum, `aff`;
- per target: the carriers (slots with a spark of it), the inheritance chance q = 1 - prod over carriers of
  (1 - min(1, 0.03 x stars x (1 + aff / 100)))^2 (grandparents at the full rate), and the spark scale
  1.1^(carriers - 6);
- per target, its exact chances jointly over the deck's chain states (val/joint_check.py's `deck_model`): the gold
  chance P(a gold hint) (1 at the outfit's level 2), and the landing chance p = scale x (0.4 x 1.1^6 - (0.4 - 0.2) x
  1.1^6 x P(no gold hint) - 0.2 x 1.1^6 x (1 - own event)(1 - q) x P(no hint)) at level 0, without the last term at
  level 1, 0.4 x 1.1^6 x scale at level 2 or for the scenario's golds. The report's p and `buy.gold` must equal them,
  `land_gold` = gold x 0.4 x 1.1^6 x scale, `land_white` = white x 0.2 x 1.1^6 x scale, and land_gold + land_white = p
  (the deck's own per-target hint chances, `deck_gold` and `deck_white`, are its sources at their marginal chances,
  shown only, and not checked);
- every chance a finite number in [0, 1];
- the landed sets where reported: they sum to 1, the all-landed one is P(all), each target's marginal is its p;
  P(all) is at most every p;
- P(all) jointly: the deck's sources and chain process restated by val/joint_check.py (`deck_model`), each target's
  landing assembled here from the recomputed q and scale, the outfit's level and her own-event chance as reported,
  then the expectation over the deck by inclusion-exclusion; within 1e-3 of the reported P(all), as joint_check holds
  the loop planner's (within 1e-12 where the report says 0).

The constants are typed in from the solver (src/skills.rs GEN_GOLD, GEN_WHITE and INSP_PER_STAR, src/skills.rs
GP_RATE 1), not read from its outputs; the own-event chances and outfit levels come from the report.

usage: uv run val/career_check.py [TAG]   (default TAG ref: val/TAG__ERA__NAME.out for val/jobs.json's career jobs,
  val/plan/TAG__NAME.out for val/plan/jobs.json); exits 1 on a disagreement"""

import itertools
import json
import math
import os
import sys
from typing import NamedTuple

import joint_check

VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(VALIDATION_DIR)
GEN_GOLD, GEN_WHITE = 0.4 * 1.771561, 0.2 * 1.771561
INSPIRATION_PER_STAR = 0.03
EXACT = 1e-9  # identities: equal up to rounding (relative)
ABSOLUTE = 1e-12  # ... and at zero
JOINT = 1e-3  # the joint recomputation, as val/joint_check.py holds the loop planner's


class Slot(NamedTuple):
    chara: int
    rel: float
    race: float
    legacy: dict  # the legacy or ancestor entry whose sparks the slot carries


class Relations:
    """data.json's relation groups: points shared by every set of characters that sit in one group together"""

    def __init__(self, data):
        self.ids = {uma["id"] for uma in data["umas"]}
        index_ids = [uma["id"] for uma in data["umas"]]
        self.groups = [(points, {index_ids[i] for i in members}) for points, members in data["relations"]]

    def points(self, *charas):
        if any(chara not in self.ids for chara in charas) or len(set(charas)) < len(charas):
            return 0.0
        return float(sum(points for points, members in self.groups if all(chara in members for chara in charas)))


def shared_wins(first, second):
    return len(set(first.get("wins", [])) & set(second.get("wins", [])))


def slots_of(relations, legacies, trainee, parents):
    """The six slots in the report's order: parent a, her parents, parent b, hers"""
    slots = []
    for own, partner in ((parents[0], parents[1]), (parents[1], parents[0])):
        parent, other = legacies[own], legacies[partner]
        grandparents = parent.get("parents", [])
        rel = (
            relations.points(trainee, parent["chara"])
            + relations.points(parent["chara"], other["chara"])
            + sum(relations.points(trainee, parent["chara"], g.get("chara", 0)) for g in grandparents)
        )
        race = 3.0 * shared_wins(parent, other) + sum(3.0 * shared_wins(parent, g) for g in grandparents)
        slots.append(Slot(chara=parent["chara"], rel=rel, race=race, legacy=parent))
        for g in grandparents:
            slots.append(
                Slot(
                    chara=g.get("chara", 0),
                    rel=relations.points(trainee, parent["chara"], g.get("chara", 0)),
                    race=3.0 * shared_wins(parent, g),
                    legacy=g,
                )
            )
    return slots


def stars(legacy, group):
    return min(3, max([s for g, s in legacy.get("sparks", []) if g == group] or [0]))


class Inheritance(NamedTuple):
    q: list
    carriers: list
    scale: list


def inheritance(slots, groups):
    q, carriers, scale = [], [], []
    for group in groups:
        miss, carrying = 1.0, 0
        for slot in slots:
            star_count = stars(slot.legacy, group)
            if star_count:
                carrying += 1
                miss *= (1 - min(1.0, INSPIRATION_PER_STAR * star_count * (1 + (slot.rel + slot.race) / 100))) ** 2
        q.append(1 - miss)
        carriers.append(carrying)
        scale.append(1.1 ** (carrying - 6))
    return Inheritance(q=q, carriers=carriers, scale=scale)


def close(first, second, tolerance=EXACT):
    """equal to a relative tolerance, or to ABSOLUTE at zero"""
    return abs(first - second) <= max(tolerance * max(abs(first), abs(second)), ABSOLUTE)


def is_probability(value):
    return isinstance(value, (int, float)) and math.isfinite(value) and -ABSOLUTE <= value <= 1 + ABSOLUTE


class Findings:
    """Disagreements (failures) and the reported-only gaps of one output"""

    def __init__(self):
        self.failures = []
        self.joint_gap = 0.0  # |joint recomputation / reported P(all) - 1|
        self.careers = 0

    def fail(self, where, what):
        if len(self.failures) < 12:
            self.failures.append(f"{where}: {what}")


def check_career(report, job, relations, model, findings, where):
    findings.careers += 1
    legacies, targets = job["legacies"], job["targets"]
    groups = [t["group"] for t in targets]
    trainee = report["trainee"]["id"]
    slots = slots_of(relations, legacies, trainee, report["parents"])
    reported_slots = report.get("slots") or []
    if len(reported_slots) != len(slots):
        findings.fail(where, f"{len(reported_slots)} slots reported, {len(slots)} recomputed")
        return
    for k, (slot, reported) in enumerate(zip(slots, reported_slots)):
        if slot.chara != reported["chara"] or not all(
            close(value, reported[key]) for key, value in (("rel", slot.rel), ("race", slot.race))
        ):
            findings.fail(where, f"slot {k}: {slot.chara} rel {slot.rel} race {slot.race}, reported {reported}")
        if not close(slot.rel + slot.race, reported["aff"]):
            findings.fail(where, f"slot {k}: aff {slot.rel + slot.race}, reported {reported['aff']}")
    inherited = inheritance(slots, groups)
    p = [t["p"] for t in report["targets"]]
    reported_chances = [report["p_all"], *p, *(t["q"] for t in report["targets"]), *(report.get("outcome_sets") or [])]
    if not all(is_probability(value) for value in reported_chances):
        findings.fail(where, "a reported P(all), p, q or landed-set chance is not a finite number in [0, 1]")
        return
    exact_targets = target_chances(report, job, inherited, model)
    for k, (target, reported) in enumerate(zip(targets, report["targets"])):
        if reported["carriers"] != inherited.carriers[k]:
            findings.fail(
                where, f"target {target['group']}: carriers {inherited.carriers[k]}, reported {reported['carriers']}"
            )
        if not close(inherited.q[k], reported["q"]):
            findings.fail(where, f"target {target['group']}: q {inherited.q[k]}, reported {reported['q']}")
        check_buying(where, findings, target, reported, exact_targets[k], inherited.scale[k])
    p_all = report["p_all"]
    if p_all > min(p) * (1 + EXACT) + ABSOLUTE:
        findings.fail(where, f"P(all) {p_all} above a target's p {min(p)}")
    sets = report.get("outcome_sets")
    if isinstance(sets, list):
        if not close(sum(sets), 1.0) or not close(sets[-1], p_all):
            findings.fail(where, f"landed sets sum {sum(sets)}, all-landed {sets[-1]}, P(all) {p_all}")
        for k in range(len(targets)):
            marginal = sum(chance for mask, chance in enumerate(sets) if mask >> k & 1)
            if not close(marginal, p[k]):
                findings.fail(where, f"target {targets[k]['group']}: landed-set marginal {marginal}, p {p[k]}")
    joint = joint_p_all(report, job, inherited, model)
    # (relative where P(all) is positive, absolute at zero: a false zero is caught too)
    gap = abs(joint / p_all - 1) if p_all > 0 else abs(joint)
    findings.joint_gap = max(findings.joint_gap, gap)
    if gap > (JOINT if p_all > 0 else ABSOLUTE):
        findings.fail(where, f"P(all) jointly {joint}, reported {p_all}")


class TargetChances(NamedTuple):
    """A target's exact chances over the deck's chain states (`target_chances`)."""

    gold: float  # P(bought as gold)
    p: float  # P(it lands)


def target_chances(report, job, inherited, model):
    """Per target, its exact gold chance and landing chance p, jointly over the deck (`deck_model`): with g the event
    "a gold hint" and h "any hint", landing is s (GG - (GG - GW) [no g] - GW (1 - own event)(1 - q) [no h]) at outfit
    level 0, s (GG - (GG - GW) [no g]) at level 1, s GG at level 2 or for the scenario's golds"""
    miss_probability = model.deck_miss_probability(report["deck"])
    uncertain = model.uncertain_targets
    chances = []
    for k, (target, reported) in enumerate(zip(job["targets"], report["targets"])):
        scale = inherited.scale[k]
        if target["group"] not in uncertain:
            chances.append(TargetChances(gold=1.0, p=GEN_GOLD * scale))
            continue
        level = max(reported["buy"]["outfit"], 1 if target.get("guaranteed", 0) == 1 else 0)
        position = uncertain.index(target["group"])
        requirement = lambda need, position=position: tuple(need if i == position else 0 for i in range(len(uncertain)))
        no_gold, no_hint = miss_probability(requirement(1)), miss_probability(requirement(2))
        if level == 2:
            chances.append(TargetChances(gold=1.0, p=GEN_GOLD * scale))
            continue
        landing = GEN_GOLD - (GEN_GOLD - GEN_WHITE) * no_gold
        if level == 0:
            landing -= GEN_WHITE * (1 - reported["buy"]["own_event"]) * (1 - inherited.q[k]) * no_hint
        chances.append(TargetChances(gold=1 - no_gold, p=landing * scale))
    return chances


def check_buying(where, findings, target, reported, exact, scale):
    """The report's buying and landing factors against the exact chances: p, the gold chance, its landing GG x scale,
    the white part's landing GW x scale x white, and land_gold + land_white = p"""
    buy, name = reported["buy"], target["group"]
    if not close(reported["p"], exact.p):
        findings.fail(where, f"target {name}: p {exact.p} jointly, reported {reported['p']}")
    if buy.get("scenario"):
        if not close(buy["land_gold"], exact.p):
            findings.fail(where, f"target {name}: scenario landing {exact.p}, reported {buy['land_gold']}")
        return
    values = [buy[key] for key in ("gold", "white", "land_gold", "land_white")]
    if not all(is_probability(value) for value in values):
        findings.fail(where, f"target {name}: a buying chance is not a finite number in [0, 1]: {buy}")
        return
    for key, value in (
        ("gold", exact.gold),
        ("land_gold", buy["gold"] * GEN_GOLD * scale),
        ("land_white", buy["white"] * GEN_WHITE * scale),
    ):
        if not close(value, buy[key]):
            findings.fail(where, f"target {name}: {key} {value}, reported {buy[key]}")
    if not close(buy["land_gold"] + buy["land_white"], reported["p"]):
        findings.fail(
            where, f"target {name}: land_gold + land_white {buy['land_gold'] + buy['land_white']}, p {reported['p']}"
        )


def joint_p_all(report, job, inherited, model):
    """E over the deck of the product of each target's landing chance (inclusion-exclusion over its no-gold / no-hint
    requirements, joint_check's `lineage` with per-target q and spark scale)"""
    groups = [t["group"] for t in job["targets"]]
    miss_probability = model.deck_miss_probability(report["deck"])  # (memoised per deck: `memoised`)
    coefficients, fixed = [], 1.0
    for k, (target, reported) in enumerate(zip(job["targets"], report["targets"])):
        scale = inherited.scale[k]
        if target["group"] not in model.uncertain_targets:
            fixed *= GEN_GOLD * scale
            continue
        level = max(reported["buy"]["outfit"], 1 if target.get("guaranteed", 0) == 1 else 0)
        own_event = reported["buy"]["own_event"]
        gold, white = GEN_GOLD * scale, GEN_WHITE * scale
        if level == 2:
            coefficients.append((gold, 0.0, 0.0))
        elif level == 1:
            coefficients.append((gold, gold - white, 0.0))
        else:
            coefficients.append((gold, gold - white, white * (1 - own_event) * (1 - inherited.q[k])))
    assert len(coefficients) == len(model.uncertain_targets) == len([g for g in groups if g in model.uncertain_targets])
    total = 0.0
    for requirement in itertools.product(range(3), repeat=len(coefficients)):
        term = 1.0
        for coefficient, need in zip(coefficients, requirement):
            term *= coefficient[0] if need == 0 else -coefficient[need]
            if term == 0:
                break
        if term:
            total += term * miss_probability(requirement)
    return total * fixed


def memoised(model):
    """`model` with its per-deck miss probabilities kept: a plan reports one deck for many careers"""
    decks = {}

    def deck_miss_probability(deck):
        key = json.dumps(deck, sort_keys=True)
        if key not in decks:
            decks[key] = model.deck_miss_probability(deck)
        return decks[key]

    return model._replace(deck_miss_probability=deck_miss_probability)


def careers_of(output):
    """(where, report) for every career report in a career job's output"""
    for i, report in enumerate(output.get("results") or []):
        yield f"results[{i}]", report
    plan = output.get("next") or {}
    if plan.get("choice"):
        yield "next.choice", plan["choice"]
    for i, report in enumerate(plan.get("options") or []):
        yield f"next.options[{i}]", report


def check_output(name, job_path, output_path, era):
    data_dir = os.path.join(REPO, "pkg", "loopopt", "data", era)
    job = json.load(open(job_path))
    output = json.loads([line for line in open(output_path).read().splitlines() if line.strip()][-1])
    relations = Relations(json.load(open(os.path.join(data_dir, "data.json"))))
    model = memoised(joint_check.deck_model(job, data_dir))
    findings = Findings()
    for where, report in careers_of(output):
        if report.get("slots") and report.get("targets") and report.get("deck"):
            check_career(report, job, relations, model, findings, where)
    print(
        f"{name}: {findings.careers} careers, {'OK' if not findings.failures else 'DIFFERS'}; joint P(all) within "
        f"{findings.joint_gap:.1e}"
    )
    for failure in findings.failures:
        print("   ", failure)
    return bool(findings.failures) or findings.careers == 0


def main(tag):
    pairs = []
    for era, name in json.load(open(os.path.join(VALIDATION_DIR, "jobs.json"))):
        job_path = os.path.join(VALIDATION_DIR, "jobs", name + ".json")
        if json.load(open(job_path)).get("mode") == "career":
            pairs.append((f"{era}__{name}", job_path, os.path.join(VALIDATION_DIR, f"{tag}__{era}__{name}.out"), era))
    plan_dir = os.path.join(VALIDATION_DIR, "plan")
    for era, name in json.load(open(os.path.join(plan_dir, "jobs.json"))):
        pairs.append(
            (f"plan {name}", os.path.join(plan_dir, name + ".json"), os.path.join(plan_dir, f"{tag}__{name}.out"), era)
        )
    failures = 0
    for name, job_path, output_path, era in pairs:
        if not os.path.exists(output_path):
            print(f"{name}: no output {os.path.relpath(output_path, REPO)}")
            failures += 1
            continue
        failures += check_output(name, job_path, output_path, era)
    print(f"{len(pairs)} outputs, {failures} differ or are missing")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "ref"))
