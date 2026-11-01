"""Compare rounded validation values, selected decks or careers, and exact/product cap margins.

Each output's results are compared rank by rank. A rank whose P(all) differs (rounded to 14 decimals) fails. A rank
that names another deck or career fails too, unless it is a confirmed tie: the reference's and the new result at that
rank have P(all) and P(all but one) equal to the bit, the two values the search ranks and reports by. A confirmed tie
is printed as such and passes here; `val/same.py`, which `./build.sh val` runs next, still fails on it, so the
commit that moves a tie re-takes the reference."""

import argparse
import json
import sys
import glob
import os
from typing import NamedTuple


class DeckSignature(NamedTuple):
    """Rounded odds and choices that determine a reported loop's identity."""

    p_all: float
    p_minus1: float
    cards_and_stops: tuple
    members: tuple
    exact_odds: tuple  # (P(all), P(all but one)) as reported, unrounded: a tie is these equal to the bit


class CareerSignature(NamedTuple):
    p_all: float
    p_minus1: float
    trainee_id: int
    parents: tuple
    card_ids: tuple
    exact_odds: tuple  # (P(all), P(all but one)) as reported, unrounded


def exact_odds(result):
    """P(all) and P(all but one) by their bits (float.hex: -0.0 and 0.0 differ, as their bits do)"""
    return (float(result["p_all"]).hex(), float(result["p_minus1"]).hex())


class RankChanges(NamedTuple):
    """How the results moved, rank by rank (the ranks listed from 0)"""

    values_equal: bool  # the rounded P(all) list, every rank, is the reference's
    ties: list  # another result at odds equal to the bit: a confirmed tie
    moved: list  # another result at other odds
    revalued: list  # the same result at other rounded odds


def identity(signature):
    """what names a result: every signature field but its odds"""
    return signature._replace(p_all=None, p_minus1=None, exact_odds=None)


def compare_ranks(reference_signatures, candidate_signatures):
    """the `RankChanges` from the reference's results to the candidate's"""
    changes = RankChanges(
        values_equal=[s.p_all for s in reference_signatures] == [s.p_all for s in candidate_signatures],
        ties=[],
        moved=[],
        revalued=[],
    )
    for rank, (reference, candidate) in enumerate(zip(reference_signatures, candidate_signatures)):
        if reference == candidate:
            continue
        if identity(reference) == identity(candidate):
            changes.revalued.append(rank)
        elif reference.exact_odds == candidate.exact_odds:
            changes.ties.append(rank)
        else:
            changes.moved.append(rank)
    return changes


def rank_note(changes, what):
    """the printed verdict on the ranks whose result moved"""
    notes = [
        f"{text} at ranks {ranks}"
        for text, ranks in (
            (f"{what} differ, confirmed ties (P(all) and P(all but one) equal to the bit)", changes.ties),
            (f"{what} DIFFER, not a tie (other odds)", changes.moved),
            (f"same {what}, ODDS DIFFER", changes.revalued),
        )
        if ranks
    ]
    return "; ".join(notes) or f"{what} same"


def fails(changes):
    """a value or pick change that is not a confirmed tie"""
    return not changes.values_equal or bool(changes.moved or changes.revalued)


def deck_signatures(output):
    return [
        DeckSignature(
            p_all=round(top["loop"]["p_all"], 14),
            p_minus1=round(top["loop"]["p_minus1"], 12),
            cards_and_stops=tuple(sorted(zip(top["deck"]["card_ids"], top["deck"].get("end_chain_at", [0] * 9)))),
            members=tuple(
                member if isinstance(member, str) else member.get("name") for member in top["loop"]["members"]
            ),
            exact_odds=exact_odds(top["loop"]),
        )
        for top in output["top"]
    ]


def career_signatures(output):
    return [
        CareerSignature(
            p_all=round(career["p_all"], 14),
            p_minus1=round(career["p_minus1"], 12),
            trainee_id=career["trainee"]["id"],
            parents=tuple(career["parents"]),
            card_ids=tuple(sorted(career["deck"]["card_ids"])),
            exact_odds=exact_odds(career),
        )
        for career in output["results"]
    ]


VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__)) + "/"
# the deck structure classes of the exact caps (`cap_class`: chain-varying targets; a multi-target group counts as 4+)
CAP_CLASSES = ["0-1", "2", "3", "4+"]


def near_cap(ratio, cap):
    # a scored deck's exact / product r is near its class's assumed cap c: past three quarters of the cap's headroom
    # over 1 (1.1875 of 1.25; 1.0375 of 1.05).
    # The same rule as val/planval.py.
    return ratio > 1 + 0.75 * (cap - 1)


parser = argparse.ArgumentParser(
    description="Compare rounded validation values, selected decks or careers, and exact/product cap margins.",
    allow_abbrev=False,
)
parser.add_argument("tag", help="the outputs val/TAG__*.out")
parser.add_argument("ref", help="the references val/REF__*.out")
arguments = parser.parse_args()
tag, ref = arguments.tag, arguments.ref
mismatches = 0
compared = 0


def load(p):
    t = open(p).read()
    try:
        return json.loads(t[t.index("{") :])
    except Exception:
        return None


# an output without a reference (a new job) compares with nothing: it fails until its reference is taken
for candidate_path in sorted(glob.glob(VALIDATION_DIR + tag + "__*.out")):
    name = candidate_path.split(tag + "__")[1]
    if not os.path.exists(VALIDATION_DIR + f"{ref}__{name}"):
        print(f"{name}: NO REFERENCE ({ref}__{name}): take it once the output is checked")
        mismatches += 1
for reference_path in sorted(glob.glob(VALIDATION_DIR + ref + "__*.out")):
    name = reference_path.split(ref + "__")[1]
    candidate_path = VALIDATION_DIR + f"{tag}__{name}"
    if not os.path.exists(candidate_path):
        continue
    reference, candidate = load(reference_path), load(candidate_path)
    compared += 1
    if reference is None or candidate is None:
        print(name, "PARSE FAIL", reference is None, candidate is None)
        mismatches += 1
        continue
    # Ratios near an assumed cap warn that pruning may discard a better deck.
    certificate = candidate.get("exact") if isinstance(candidate, dict) else None
    if isinstance(certificate, dict) and isinstance(certificate.get("class_max_ratio"), list):
        for k, (ratio, cap) in enumerate(zip(certificate["class_max_ratio"], certificate["class_caps"])):
            if near_cap(ratio, cap):
                print(f"{name}: CAP class {CAP_CLASSES[k]} exact/product {ratio:.4f} near its assumed cap {cap}")
                mismatches += 1
    elif isinstance(certificate, dict) and isinstance(certificate.get("max_ratio"), (int, float)):
        cap = certificate.get("rho_target") or 1.25
        if near_cap(certificate["max_ratio"], cap):
            print(f"{name}: CAP exact/product {certificate['max_ratio']:.3f} near the assumed cap {cap}")
            mismatches += 1
    if "top" in reference:
        reference_signatures, candidate_signatures = deck_signatures(reference), deck_signatures(candidate)
        changes = compare_ranks(reference_signatures, candidate_signatures)
        print(
            f"{name}: values {'OK' if changes.values_equal else 'DIFF'} {rank_note(changes, 'decks')} ref {[round(1 / s.p_all) for s in reference_signatures]} new {[round(1 / s.p_all) for s in candidate_signatures]} proven {reference.get('proven')}/{candidate.get('proven')}"
        )
        if fails(changes):
            mismatches += 1
    elif "results" in reference:
        reference_signatures, candidate_signatures = career_signatures(reference), career_signatures(candidate)
        changes = compare_ranks(reference_signatures, candidate_signatures)
        print(
            f"{name}: career values {'OK' if changes.values_equal else 'DIFF'} {rank_note(changes, 'picks')} proven {reference.get('proven_top')}/{candidate.get('proven_top')}"
        )
        if fails(changes):
            mismatches += 1
    else:
        print(name, "neither a deck nor a career output")
        mismatches += 1
print(f"{compared} compared, {mismatches} mismatches (values, picks not a confirmed tie, cap margins, parse failures)")
sys.exit(1 if mismatches or not compared else 0)
