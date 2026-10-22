"""Compare rounded validation values, selected decks, and exact/product cap margins."""

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


class CareerSignature(NamedTuple):
    p_all: float
    p_minus1: float
    trainee_id: int
    parents: tuple
    card_ids: tuple


def deck_signatures(output):
    return [
        DeckSignature(
            p_all=round(top["loop"]["p_all"], 14),
            p_minus1=round(top["loop"]["p_minus1"], 12),
            cards_and_stops=tuple(sorted(zip(top["deck"]["card_ids"], top["deck"].get("end_chain_at", [0] * 9)))),
            members=tuple(
                member if isinstance(member, str) else member.get("name") for member in top["loop"]["members"]
            ),
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


tag = sys.argv[1]
mismatches = 0
compared = 0


def load(p):
    t = open(p).read()
    try:
        return json.loads(t[t.index("{") :])
    except Exception:
        return None


for reference_path in sorted(glob.glob(VALIDATION_DIR + sys.argv[2] + "__*.out")):
    name = reference_path.split(sys.argv[2] + "__")[1]
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
        ok = [s.p_all for s in reference_signatures] == [s.p_all for s in candidate_signatures]
        same = reference_signatures == candidate_signatures
        print(
            f"{name}: values {'OK' if ok else 'DIFF'} decks {'same' if same else 'differ(ties?)'} ref {[round(1 / s.p_all) for s in reference_signatures]} new {[round(1 / s.p_all) for s in candidate_signatures]} proven {reference.get('proven')}/{candidate.get('proven')}"
        )
        if not ok:
            mismatches += 1
    elif "results" in reference:
        reference_signatures, candidate_signatures = career_signatures(reference), career_signatures(candidate)
        ok = [s.p_all for s in reference_signatures] == [s.p_all for s in candidate_signatures]
        same = reference_signatures == candidate_signatures
        print(
            f"{name}: career values {'OK' if ok else 'DIFF'} picks {'same' if same else 'differ(ties?)'} proven {reference.get('proven_top')}/{candidate.get('proven_top')}"
        )
        if not ok:
            mismatches += 1
    else:
        print(name, "neither a deck nor a career output")
        mismatches += 1
print(f"{compared} compared, {mismatches} value mismatches")
sys.exit(1 if mismatches or not compared else 0)
