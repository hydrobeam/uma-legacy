"""Compare a validation tag with a reference when every value is expected to scale: same loops and decks, and the
new/reference ratios of p_all and p_minus1 for the first result. usage: cmp_ratio.py TAG REF"""

import json
import sys
import glob
import os
from dataclasses import dataclass


@dataclass(frozen=True)
class RatioComparison:
    """Reciprocal odds (expected attempts), with new/reference probability ratios."""

    name: str
    same_choices: bool
    all_reference: float
    all_candidate: float
    all_ratio: float
    minus1_reference: float
    minus1_candidate: float
    minus1_ratio: float
    proven: object


VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__)) + "/"
tag, ref = sys.argv[1], sys.argv[2]


def load(p):
    t = open(p).read()
    try:
        return json.loads(t[t.index("{") :])
    except Exception:
        return None


rows = []
for reference_path in sorted(glob.glob(VALIDATION_DIR + ref + "__*.out")):
    name = reference_path.split(ref + "__")[1]
    candidate_path = VALIDATION_DIR + f"{tag}__{name}"
    if not os.path.exists(candidate_path):
        continue
    reference, candidate = load(reference_path), load(candidate_path)
    if reference is None or candidate is None:
        print(name, "PARSE FAIL")
        continue
    if "top" in reference:
        reference_results = reference["top"]
        candidate_results = candidate["top"]
        members = lambda l: tuple(m["name"] if isinstance(m, dict) else m for m in l["members"])
        same = [members(a["loop"]) for a in reference_results] == [members(b["loop"]) for b in candidate_results] and [
            sorted(a["deck"]["card_ids"]) for a in reference_results
        ] == [sorted(b["deck"]["card_ids"]) for b in candidate_results]
        if "decks" in reference["top"][0]:
            same = same and [[sorted(d["card_ids"]) for d in t["decks"]] for t in reference["top"]] == [
                [sorted(d["card_ids"]) for d in t["decks"]] for t in candidate["top"]
            ]
        all_ratio = candidate_results[0]["loop"]["p_all"] / reference_results[0]["loop"]["p_all"]
        minus1_ratio = candidate_results[0]["loop"]["p_minus1"] / reference_results[0]["loop"]["p_minus1"]
        rows.append(
            RatioComparison(
                name=name.replace(".out", ""),
                same_choices=same,
                all_reference=1 / reference_results[0]["loop"]["p_all"],
                all_candidate=1 / candidate_results[0]["loop"]["p_all"],
                all_ratio=all_ratio,
                minus1_reference=1 / reference_results[0]["loop"]["p_minus1"],
                minus1_candidate=1 / candidate_results[0]["loop"]["p_minus1"],
                minus1_ratio=minus1_ratio,
                proven=candidate.get("proven"),
            )
        )
    else:
        print(name, "not a deck output: skipped")
print(f"{'setup':34s} same  all: ref -> new (ratio)        n-1: ref -> new (ratio)   proven")
for row in rows:
    print(
        f"{row.name:34s} {'yes' if row.same_choices else 'NO ':4s} 1 in {row.all_reference:6.0f} -> {row.all_candidate:6.0f} (x{row.all_ratio:.4f})   "
        + (
            f"1 in {row.minus1_reference:5.1f} -> {row.minus1_candidate:5.1f} (x{row.minus1_ratio:.4f})"
            if row.minus1_reference
            else " " * 33
        )
        + f"  {row.proven}"
    )
print(
    f"{len(rows)} compared; {sum(not row.same_choices for row in rows)} with different loops or decks; "
    f"p_all ratio range {min(row.all_ratio for row in rows):.5f}..{max(row.all_ratio for row in rows):.5f}"
)
