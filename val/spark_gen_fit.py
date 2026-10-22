"""Measure spark generation (the chance a skill learned in a career becomes a white spark on the veteran) from a real
umadump trained_chara_data.json, by the version bought and by how many of the career's six ancestors carried that
spark. Fits "x m per carrier" against "+a points per carrier".

usage: spark_gen_fit.py TRAINED_CHARA_DATA_JSON [MASTER_MDB] [--scenario N]
MASTER_MDB defaults to game/global/master.mdb only without --scenario; supply it explicitly with --scenario.
Ancestors: positions 10/20 are the veteran's parents. Its grandparents are positions 11/12/21/22 when the dump has
them; otherwise a parent is matched to exactly one input veteran with the same card, sparks and win saddles, and that
veteran's parents are used. Skill-observations whose six ancestors are not all known are reported but not fitted.
"""

import json
import math
import sqlite3
import sys
import collections
import os
from dataclasses import dataclass
from typing import NamedTuple


class SparkSkill(NamedTuple):
    factor_group: int
    tier: str


class VeteranFingerprint(NamedTuple):
    card_id: int
    factors: tuple[int, ...]
    win_saddles: tuple[int, ...]


class SparkObservation(NamedTuple):
    """One learned skill whose six ancestors are known."""

    tier: str
    parent_carriers: int
    grandparent_carriers: int
    sparked: bool


@dataclass
class OutcomeCounts:
    sparked: int = 0
    observations: int = 0


@dataclass(frozen=True)
class FittedCurve:
    log_likelihood: float
    step: float  # Multiplier or additive probability per carrier, according to fit kind.
    bases: dict[str, float]


args = [a for a in sys.argv[1:] if not a.startswith("--")]
scenario = int(sys.argv[sys.argv.index("--scenario") + 1]) if "--scenario" in sys.argv else None
veterans = json.load(open(args[0]))
db = sqlite3.connect(
    args[1] if len(args) > 1 else os.path.dirname(os.path.abspath(__file__)) + "/../game/global/master.mdb"
)
query = lambda s, *a: db.execute(s, a).fetchall()

# skill -> white spark factor group, and the tier it was bought at
skill_groups = dict(query("select id, group_id from skill_data"))
skill_rarities = dict(query("select id, rarity from skill_data"))
skill_names = dict(query("select [index], text from text_data where category = 47"))
white_factor_groups = {}  # skill group -> factor group of its white spark
for factor_group, skill_id in query(
    "select distinct f.factor_group_id, e.value_1 from succession_factor f join succession_factor_effect e"
    " on e.factor_group_id = f.factor_group_id where f.factor_type = 4 and e.target_type = 41"
):
    if skill_id in skill_groups:
        white_factor_groups.setdefault(skill_groups[skill_id], factor_group)


def spark_of(skill_id):
    g = skill_groups.get(skill_id)
    if g is None or g not in white_factor_groups or skill_rarities.get(skill_id, 9) > 2:
        return None
    tier = "gold" if skill_rarities[skill_id] == 2 else "dbl" if "◎" in skill_names.get(skill_id, "") else "white"
    return SparkSkill(factor_group=white_factor_groups[g], tier=tier)


factor_groups = lambda factors: {f["factor_id"] // 100 for f in factors}

# owned veterans by fingerprint, to recover grandparents
fingerprint = lambda e: VeteranFingerprint(
    e["card_id"],
    tuple(sorted(f["factor_id"] for f in e["factor_info_array"])),
    tuple(sorted(e["win_saddle_id_array"])),
)
owned_by_fingerprint = collections.defaultdict(list)
for v in veterans:
    owned_by_fingerprint[fingerprint(v)].append(v)

observations = []
partial = 0
missing_parents = 0
for v in veterans:
    if scenario is not None and v.get("scenario_id") != scenario:
        continue
    anc = {e["position_id"]: e for e in v["succession_chara_array"]}
    if 10 not in anc or 20 not in anc:
        missing_parents += 1
        continue
    for p, (a, b) in ((10, (11, 12)), (20, (21, 22))):
        if a in anc and b in anc:
            continue
        m = owned_by_fingerprint.get(fingerprint(anc[p]), [])
        if len(m) == 1:
            ga = {e["position_id"]: e for e in m[0]["succession_chara_array"]}
            if 10 in ga and 20 in ga:
                anc[a], anc[b] = ga[10], ga[20]
    full = all(k in anc for k in (10, 11, 12, 20, 21, 22))
    mine = factor_groups(v["factor_info_array"])
    seen = set()
    for s in v["skill_array"]:
        r = spark_of(s["skill_id"])
        if not r or r.factor_group in seen:
            continue
        seen.add(r.factor_group)
        factor_group, tier = r
        if not full:
            partial += 1
            continue
        parent_carriers = sum(factor_group in factor_groups(anc[k]["factor_info_array"]) for k in (10, 20))
        grandparent_carriers = sum(factor_group in factor_groups(anc[k]["factor_info_array"]) for k in (11, 12, 21, 22))
        observations.append(
            SparkObservation(
                tier=tier,
                parent_carriers=parent_carriers,
                grandparent_carriers=grandparent_carriers,
                sparked=factor_group in mine,
            )
        )
print(
    f"veterans: {len(veterans)}; skill-observations with all six ancestors known: {len(observations)}; without: {partial}; "
    f"veterans with no parents stored: {missing_parents}"
)
if not observations:
    sys.exit()


def wilson(k, n, z=1.96):
    if n == 0:
        return (0, 0)
    p = k / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return ((c - h) / d, (c + h) / d)


counts = collections.defaultdict(OutcomeCounts)
for tier, parent_carriers, grandparent_carriers, hit in observations:
    counts[(tier, parent_carriers + grandparent_carriers)].sparked += hit
    counts[(tier, parent_carriers + grandparent_carriers)].observations += 1
print("\ncarriers  tier    n      sparked  rate    95% CI          x1.1^n model  +2/carrier model")
base = {"white": 0.20, "dbl": 0.25, "gold": 0.40}
for tier in ("white", "dbl", "gold"):
    for n in range(7):
        cell = counts[(tier, n)]
        k, N = cell.sparked, cell.observations
        if N == 0:
            continue
        lo, hi = wilson(k, N)
        print(
            f"{n:8d}  {tier:6s} {N:6d}  {k:7d}  {k / N:6.3f}  [{lo:.3f}, {hi:.3f}]  {base[tier] * 1.1**n:12.3f}  {base[tier] + 0.02 * n:16.3f}"
        )


# maximum likelihood on the (tier, carriers) table: p = b_tier * m^n  vs  p = b_tier + a*n (grid search)
def binomial_log_likelihood(k, N, p):
    p = min(max(p, 1e-9), 1 - 1e-9)
    return k * math.log(p) + (N - k) * math.log(1 - p)


tiers = sorted({o.tier for o in observations})


def log_likelihood(f):
    return sum(
        binomial_log_likelihood(k=cell.sparked, N=cell.observations, p=f(t, n))
        for (t, n), cell in counts.items()
        if cell.observations
    )


def fit(kind):
    best = None
    steps = [x / 1000 for x in range(0, 301, 2)] if kind == "add" else [1 + x / 1000 for x in range(0, 301, 2)]
    for step in steps:
        g = (lambda b, n, step=step: b * step**n) if kind == "mul" else (lambda b, n, step=step: b + step * n)
        bases = {
            t: max(
                (b / 1000 for b in range(50, 801, 2)),
                key=lambda b: sum(
                    binomial_log_likelihood(k=counts[(t, n)].sparked, N=counts[(t, n)].observations, p=g(b, n))
                    for n in range(7)
                    if counts[(t, n)].observations
                ),
            )
            for t in tiers
        }
        v = log_likelihood(lambda t, n, g=g, bases=bases: g(bases[t], n))
        if best is None or v > best.log_likelihood:
            best = FittedCurve(log_likelihood=v, step=step, bases=bases)
    return best


for kind, label in (("mul", "x m per carrier"), ("add", "+a per carrier")):
    curve = fit(kind)
    print(
        f"\n{label}: step {curve.step:.3f}, bases "
        + ", ".join(f"{t} {b:.3f}" for t, b in curve.bases.items())
        + f", log-likelihood {curve.log_likelihood:.1f}"
    )
fixed = log_likelihood(lambda t, n: base[t] * 1.1**n)
fixed2 = log_likelihood(lambda t, n: base[t] + 0.02 * n)
print(
    f"\nlog-likelihood of the published curves: 20/25/40% x 1.1^n {fixed:.1f}; 20/25/40% + 2 points per carrier {fixed2:.1f}"
)
print(
    "(a log-likelihood gap above ~2 per extra parameter is a real preference; below that the data can't tell them apart)"
)

# do parents and grandparents count the same? white tier, by (parents carrying, grandparents carrying)
print("\nwhite tier by parents carrying / grandparents carrying (rate, n):")
parent_counts = collections.defaultdict(OutcomeCounts)
for tier, a, g, hit in observations:
    if tier == "white":
        parent_counts[(a, g)].sparked += hit
        parent_counts[(a, g)].observations += 1
for a in range(3):
    print(
        f"  parents {a:d}: "
        + "  ".join(
            f"gp{g} {parent_counts[(a, g)].sparked / parent_counts[(a, g)].observations:.3f} ({parent_counts[(a, g)].observations})"
            for g in range(5)
            if parent_counts[(a, g)].observations
        )
    )

# star mix of skill sparks by the veteran's rank (the inheritance base rate is 3/6/9% for 1/2/3 stars)
print("\nwhite skill spark stars by veteran rank code (share of 1/2/3 stars, n; expected base rate per spark):")
# Each rank histogram is indexed by star level minus one.
rank_counts = collections.defaultdict(lambda: [0, 0, 0])
for v in veterans:
    if scenario is not None and v.get("scenario_id") != scenario:
        continue
    for f in v["factor_info_array"]:
        if 2000000 <= f["factor_id"] < 3000000 and f["factor_id"] % 100 in (1, 2, 3):
            rank_counts[v["rank"]][f["factor_id"] % 100 - 1] += 1
for r in sorted(rank_counts):
    c = rank_counts[r]
    n = sum(c)
    print(
        f"  rank {r:3d}: {c[0] / n:.2f} / {c[1] / n:.2f} / {c[2] / n:.2f}  (n {n}; base {(3 * c[0] + 6 * c[1] + 9 * c[2]) / n:.2f}%)"
    )
