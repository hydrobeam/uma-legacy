"""Section 2: the star mix the loop model gives its legacies.

The mixes themselves (1/2/3-star shares 0.50/0.45/0.05 below rank score 17,500, 0.20/0.70/0.10 from 17,500) are
assumed inputs: the run records show only the legacies players chose, which are picked for their sparks, so the mix of
an unselected veteran cannot be read from them. What is computed here:
- the band each scenario's careers reach: the run list's rank-score estimate with every skill point spent
  (research/data/summary.jsonl, not included; Independent Training runs from uma-it), share at 17,500 or more per
  scenario, against the band the page picks for the scenario (webapp/src/data.js, `ss`). That estimate is an upper
  bound on the score a career ends with, so the shares are also given with the career's score taken SLACK points below
  it, and on the run list's floor (no skill point spent), with each scenario's median ceiling and the expected base of
  the run-weighted mixture of the two mixes;
- the star mix of the runs' legacies per scenario (the six ancestors' skill-white sparks in the run records, one count
  per spark per run): a selected reference, not the unselected mix, and it keys each legacy to the scenario of the
  career she is a legacy in, not the one she was bred in;
- that the solver's default mix (pkg/loopopt/src/skills.rs) is the page's below-17,500 mix (webapp/src/data.js RANKS);
- what replacing a legacy's stars by the expected base per roll b = 0.03 (pi_1 + 2 pi_2 + 3 pi_3) costs: per run of
  the records (research/data/runs.jsonl), with its six real affinities (common.six_affinities), q with b in each of the two
  rolls against the exact 1 - prod_l E_s[(1 - min(1, b(s)(1 + A_l / 100)))^2] (a legacy's stars are the same in both
  rolls), for one target carried by all six legacies, and the largest over every pattern of carriers (each nonempty
  subset of the six).
"""

import collections, math, os, re, statistics
from typing import Any, NamedTuple
import common
from common import Finding, Value

SEC: str = "2"
CUT: int = 17500  # the rank score from which the page uses the 17,500-and-up mix
SLACK: tuple[int, ...] = (250, 500, 1000)  # rank-score points the real score may sit below the ceiling
SLOTS: tuple[str, ...] = ("parent", "grandparent", "grandparent", "parent", "grandparent", "grandparent")  # G_OF order
BAND_LABEL: dict[str, str] = {"below": "below 17,500", "ss": "17,500 and up"}  # the page's mixes by RANKS key

# a star mix: the shares of 1, 2 and 3-star legacies
Mix = tuple[float, ...]


class StarMixes(NamedTuple):
    """the star mixes the page and the solver use"""

    page: dict[str, Mix]  # webapp/src/data.js RANKS: 'below' and 'ss'
    scenario_ss: dict[int, bool]  # scenario id -> the page puts its careers at 17,500 and up (data.js `ss`)
    solver_default: Mix  # pkg/loopopt/src/skills.rs dstar() (a `const fn`, its array on the next line)


def mixes() -> StarMixes:
    src = open(os.path.join(common.ROOT, "webapp", "src", "data.js")).read()
    ranks = dict(re.findall(r"(\w+): '([\d.]+,[\d.]+,[\d.]+)'", re.search(r"RANKS = \{(.*?)\}", src).group(1)))
    page = {k: tuple(float(x) for x in v.split(",")) for k, v in ranks.items()}
    scenario_ss = {int(i): b == "true" for i, b in re.findall(r"id: (\d+),[^}]*?ss: (true|false)", src)}
    rs = open(os.path.join(common.ROOT, "pkg", "loopopt", "src", "skills.rs")).read()
    solver = tuple(
        float(x) for x in re.search(r"(?:const )?fn dstar\(\) -> \[f64; 3\] \{\s*\[([^\]]*)\]", rs).group(1).split(",")
    )
    return StarMixes(page, scenario_ss, solver)


def expected_base(pi: Mix) -> float:
    """b = 0.03 (pi_1 + 2 pi_2 + 3 pi_3): the base per roll a legacy of mix pi has on average"""
    return 0.03 * (pi[0] + 2 * pi[1] + 3 * pi[2])


def claims() -> list[Value | Finding]:
    m = mixes()
    out: list[Value | Finding] = [
        common.finding(
            SEC,
            "The solver's default mix is the page's below-17,500 mix",
            m.solver_default == m.page["below"],
            f"solver {m.solver_default}, page {m.page['below']}",
        )
    ]
    out += [Value(SEC, f"expected base, {BAND_LABEL[k]}", expected_base(pi)) for k, pi in m.page.items()]
    out += scenario_bands(m.scenario_ss, m.page)
    out += legacy_star_mix(m.page, m.scenario_ss)
    out += averaging_cost(m.page)
    return out


def scenario_bands(scenario_ss: dict[int, bool], page: dict[str, Mix]) -> list[Value | Finding]:
    """per scenario, the share of the run list's careers reaching 17,500, against the band the page picks; the same
    with the score SLACK points below the ceiling and on the floor"""
    L = common.run_list()
    if L is None:
        return []
    out: list[Value | Finding] = []
    bad: list[str] = []
    slack_holds: list[str] = []  # each 17,500-and-up scenario's shares under slack, for the slack finding
    slack_ok = True  # each such scenario keeps a majority at the smallest slack and loses it at the largest
    floors: list[int] = []  # careers at 17,500 or more on the floor, per scenario
    for sid, name in common.SCEN.items():
        rows = [x for x in L if x["scenario_id"] == sid and x["score_ceiling"]]
        v = [x["score_ceiling"] for x in rows]
        if not v:
            continue
        k = sum(x >= CUT for x in v)
        lo, hi = common.wilson(k, len(v))
        share = k / len(v)
        under = {d: sum(x >= CUT + d for x in v) / len(v) for d in SLACK}
        floors.append(sum(x["score_floor"] >= CUT for x in rows))
        needed = statistics.median(map(realised_needed, rows))
        out += [
            Value(SEC, f"{name}: runs", len(v)),
            Value(SEC, f"{name}: share at 17,500 or more", share),
            Value(SEC, f"{name}: share 95% low", lo),
            Value(SEC, f"{name}: share 95% high", hi),
            *(Value(SEC, f"{name}: share at 17,500 or more, {d} below the ceiling", under[d]) for d in SLACK),
            Value(SEC, f"{name}: share at 17,500 or more on the floor", floors[-1] / len(v)),
            Value(SEC, f"{name}: median ceiling", statistics.median(v)),
            *(
                [Value(SEC, f"{name}: realised share of the skill-point gain for a majority", needed)]
                if math.isfinite(needed)
                else []
            ),
            Value(
                SEC,
                f"{name}: run-weighted expected base",
                share * expected_base(page["ss"]) + (1 - share) * expected_base(page["below"]),
            ),
        ]
        if scenario_ss[sid]:
            slack_ok &= under[SLACK[0]] > 0.5 > under[SLACK[-1]]
            slack_holds.append(
                f"{name} "
                + ", ".join(f"-{d}: {under[d]:.3f}" for d in SLACK)
                + f"; majority from {needed:.3f} realised"
            )
        if (lo > 0.5) != scenario_ss[sid] or (hi < 0.5) == scenario_ss[sid]:
            bad.append(f"{name} {share:.3f} [{lo:.3f}, {hi:.3f}], page SS {scenario_ss[sid]}")
    out.append(
        common.finding(
            SEC,
            "Each scenario's band is the one most of its careers reach: the share at 17,500 or more is "
            "above one half (95% interval) exactly for the scenarios the page puts at 17,500 and up",
            not bad,
            "; ".join(bad) or "every scenario agrees",
        )
    )
    out.append(
        common.finding(
            SEC,
            f"Descriptive, the offsets chosen after a first look at the shares: the 17,500-and-up default rests on "
            f"the ceiling, an upper bound; each scenario the page puts at 17,500 and up keeps a majority there with "
            f"its scores {SLACK[0]} below the ceiling and loses it at {SLACK[-1]} below; on the floor (no skill point "
            f"spent) no career of any scenario reaches 17,500",
            bool(slack_holds) and slack_ok and not any(floors),
            "; ".join(slack_holds) + f"; floor: {sum(floors)} careers at 17,500+",
        )
    )
    return out


def realised_needed(row: dict[str, Any]) -> float:
    """the share of the score from floor to ceiling (what spending the skill points adds, taken as linear) a career
    must realise to reach 17,500: 0 when the floor reaches it, infinite when the ceiling does not"""
    floor, ceiling = row["score_floor"], row["score_ceiling"]
    if floor >= CUT:
        return 0.0
    return (CUT - floor) / (ceiling - floor) if ceiling >= CUT else math.inf


def legacy_star_mix(page: dict[str, Mix], scenario_ss: dict[int, bool]) -> list[Value | Finding]:
    """per scenario, the 1/2/3-star shares and the mean base of the runs' legacies' skill-white sparks (each spark
    once per run it appears in; the base's interval run-clustered): a selected reference"""
    R = common.runs()
    if R is None:
        return []
    _, factor_type = common.relations()
    below = expected_base(page["below"])
    out: list[Value | Finding] = []
    above: list[str] = []  # each scenario's mean base against the below-17,500 mix's
    own_default: list[
        tuple[str, bool]
    ] = []  # each 17,500-and-up scenario: its legacies' interval below that mix's base
    every_above = True
    for sid, name in common.SCEN.items():
        stars: list[int] = []
        run_of: list[int] = []
        for r in R:
            anc = common.lineage(r) if r["scenario"] == sid else None
            for a in anc or []:
                for f, _ in a.get("factors") or []:
                    if factor_type.get(f) == 4:
                        stars.append(f % 10)
                        run_of.append(r["id"])
        if not stars:
            continue
        base = common.cluster_ci([0.03 * s for s in stars], [1] * len(stars), run_of)
        out += [
            Value(SEC, f"{name}: legacies' white sparks", len(stars)),
            *(Value(SEC, f"{name}: legacies' {s} star share", stars.count(s) / len(stars)) for s in (1, 2, 3)),
            Value(SEC, f"{name}: legacies' mean base", base.ratio),
            Value(SEC, f"{name}: legacies' mean base 95% low", base.low),
            Value(SEC, f"{name}: legacies' mean base 95% high", base.high),
        ]
        every_above &= base.low > below
        above.append(f"{name} {base.ratio:.4f} [{base.low:.4f}, {base.high:.4f}] ({len(stars):,} sparks)")
        if scenario_ss[sid]:
            own_default.append((name, base.high < expected_base(page["ss"])))
    out.append(
        common.finding(
            SEC,
            "The runs' legacies, a selected set, carry a higher mean base than the below-17,500 mix in every scenario "
            "with runs (run-clustered 95% interval above it)",
            bool(above) and every_above,
            f"below-17,500 mix {below:.4f}; " + "; ".join(above),
        )
    )
    out.append(
        common.finding(
            SEC,
            "Each scenario the page puts at 17,500 and up has legacies whose mean base lies below the 17,500-and-up "
            "mix's that its default uses (run-clustered 95% interval below it), though they are selected for their "
            "sparks",
            bool(own_default) and all(ok for _, ok in own_default),
            f"17,500-and-up mix {expected_base(page['ss']):.4f}; " + ", ".join(n for n, _ in own_default),
        )
    )
    return out


def averaging_cost(page: dict[str, Mix]) -> list[Value | Finding]:
    """per lineage of the run records, q with the expected base in both rolls over the exact q over the stars"""
    R = common.runs()
    if R is None:
        return []
    S, _ = common.relations()
    ratios: dict[str, list[float]] = {k: [] for k in page}
    worst: dict[str, list[tuple[float, int]]] = {k: [] for k in page}  # per lineage: (largest ratio, carrier mask)
    for r in R:
        anc = common.lineage(r)
        aff = r.get("affinity") or {}
        if anc is None or not common.has_affinity_rows(aff):
            continue
        T = r["chara"]
        A = [a["chara"] for a in anc]
        if len({T, A[0], A[3]}) < 3:
            continue
        affinities = common.six_affinities(T, A, aff, S)
        for k, pi in page.items():
            b = expected_base(pi)
            loop = {i: (1 - min(1.0, b * (1 + a / 100))) ** 2 for i, a in affinities.items()}  # per slot: P(no fire)
            exact = {
                i: sum(w * (1 - min(1.0, 0.03 * (s + 1) * (1 + a / 100))) ** 2 for s, w in enumerate(pi))
                for i, a in affinities.items()
            }
            ratios[k].append((1 - math.prod(loop.values())) / (1 - math.prod(exact.values())))
            worst[k].append(max((pattern_ratio(loop, exact, mask), mask) for mask in range(1, 64)))
    out: list[Value | Finding] = [Value(SEC, "lineages", len(ratios["below"]))]
    for k, v in ratios.items():
        top, mask = max(worst[k])
        out += [
            Value(SEC, f"q overstatement, {BAND_LABEL[k]}: mean", statistics.mean(v) - 1),
            Value(SEC, f"q overstatement, {BAND_LABEL[k]}: max", max(v) - 1),
            Value(SEC, f"q overstatement, {BAND_LABEL[k]}: max over carrier patterns", top - 1),
            Value(
                SEC,
                f"q overstatement, {BAND_LABEL[k]}: mean of each lineage's worst pattern",
                statistics.mean(r for r, _ in worst[k]) - 1,
            ),
            Value(SEC, f"q overstatement, {BAND_LABEL[k]}: worst carrier pattern", carriers(mask)),
        ]
    out.append(
        common.finding(
            SEC,
            "A check of the computation, which convexity guarantees: replacing stars by their expected base never "
            "understates q; every lineage's ratio is at least 1",
            all(x >= 1 - 1e-12 for v in ratios.values() for x in v),
            ", ".join(f"{BAND_LABEL[k]} min {min(v):.6f}" for k, v in ratios.items()),
        )
    )
    return out


def in_mask(mask: int) -> list[int]:
    """the slots (0-5, G_OF order) whose bit is set in a carrier mask"""
    return [i for i in range(6) if (mask >> i) & 1]


def pattern_ratio(loop: dict[int, float], exact: dict[int, float], mask: int) -> float:
    """q with the expected base over the exact q when the slots in mask carry the target (loop, exact: per slot, the
    chance she fires in neither roll)"""
    return (1 - math.prod(loop[i] for i in in_mask(mask))) / (1 - math.prod(exact[i] for i in in_mask(mask)))


def carriers(mask: int) -> str:
    """a carrier pattern's label: how many parents and grandparents carry the target"""
    n = collections.Counter(SLOTS[i] for i in in_mask(mask))
    return ", ".join(f"{n[g]} {g}{'s' if n[g] != 1 else ''}" for g in ("parent", "grandparent") if n[g])


if __name__ == "__main__":
    common.main(claims)
