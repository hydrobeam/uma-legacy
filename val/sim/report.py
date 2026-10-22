"""Summaries of policy-simulator outputs (`simulate` career jobs; docs/SOLVER.md "Policy simulator").

    uv run val/sim/report.py runs A.out [B.out ...]      per policy: E mean, sd, 95% CI, careers, final p, wall time;
                                                          paired differences between policies over common run indices
    uv run val/sim/report.py e1 A.out [B.out ...]        surrogate fidelity from the `check` records
    uv run val/sim/report.py audit A.out [B.out ...]     surrogate fidelity from the `audit` records

An output file is the solver's stdout for one job (one JSON line); several files of one policy (split runs) pool.
"""

import json
import math
import sys
from collections import defaultdict
from typing import NamedTuple


class SampleSummary(NamedTuple):
    count: int
    mean: float
    standard_deviation: float
    interval_half_width: float  # Two-sided 95% confidence interval for the mean.


class LineageCheck(NamedTuple):
    """The simulator's lineage check row, followed by the chosen surrogate value."""

    ceiling: float
    bound: float
    table_value: float
    exact: float
    relative_error: float
    trusted: bool
    total_variation: float
    full: bool
    pick: bool
    chosen_value: float


class AuditRow(NamedTuple):
    """The simulator's positional audit row, decoded at the input boundary."""

    kind: int
    table_value: float
    exact: float
    relative_error: float
    total_variation: float
    complete: bool


class RotationAudit(NamedTuple):
    keep: float
    replace: float
    exact_keep: float
    exact_replace: float
    same_decision: bool


def student_t_975(df):
    z = 1.959964
    return (
        z
        + (z**3 + z) / (4 * df)
        + (5 * z**5 + 16 * z**3 + 3 * z) / (96 * df**2)
        + (3 * z**7 + 19 * z**5 + 17 * z**3 - 15 * z) / (384 * df**3)
    )


def summarize(values):
    count = len(values)
    mean = sum(values) / count
    standard_deviation = math.sqrt(sum((x - mean) ** 2 for x in values) / (count - 1)) if count > 1 else 0.0
    half_width = student_t_975(count - 1) * standard_deviation / math.sqrt(count) if count > 1 else 0.0
    return SampleSummary(count=count, mean=mean, standard_deviation=standard_deviation, interval_half_width=half_width)


def load(paths):
    by_policy = defaultdict(dict)
    for p in paths:
        for line in open(p):
            line = line.strip()
            if not line:
                continue
            o = json.loads(line)
            if "runs" not in o:
                print(f"{p}: no runs ({str(o)[:200]})")
                continue
            name = o["policy"]
            sim = o["simulate"]
            pool0 = o["runs"][0]["segs"][0]["pool"] if o["runs"] and o["runs"][0]["segs"] else "?"
            tag = (
                f"[{pool0} vets] "
                + name
                + "".join(f" {k}={sim[k]}" for k in ("margin", "loop_members") if k in sim and k in o["simulate"])
            )
            for r in o["runs"]:
                by_policy[tag][r["run"]] = r
    return by_policy


def runs(paths):
    by_policy = load(paths)
    for tag, policy_runs in by_policy.items():
        expected_careers = [r["E"] for r in policy_runs.values()]
        count, mean, standard_deviation, half_width = summarize(expected_careers)
        c = summarize([r["careers"] for r in policy_runs.values()])
        final_probability = summarize([r["final_p"] for r in policy_runs.values()])
        w = summarize([r["wall_s"] for r in policy_runs.values()])
        tail_value = summarize([r["tail"] for r in policy_runs.values()])
        print(
            f"{tag}: n {count}  E {mean:.1f}  sd {standard_deviation:.1f}  95% CI {mean - half_width:.1f} to {mean + half_width:.1f}  careers {c.mean:.0f}  final p {final_probability.mean:.3e} (1/{1 / final_probability.mean:.0f})  "
            f"tail {tail_value.mean:.2f}  wall {w.mean:.1f} s/run"
        )
    tags = list(by_policy)
    for i in range(len(tags)):
        for j in range(i + 1, len(tags)):
            if tags[i].split("]")[0] != tags[j].split("]")[0]:
                continue  # different pools
            a, b = by_policy[tags[i]], by_policy[tags[j]]
            common = sorted(set(a) & set(b))
            if len(common) < 2:
                continue
            d = [a[k]["E"] - b[k]["E"] for k in common]
            count, mean, standard_deviation, half_width = summarize(d)
            lo = sum(x < 0 for x in d)
            hi = sum(x > 0 for x in d)
            print(
                f"  {tags[i]} - {tags[j]}: paired n {count}  {mean:+.1f}  95% CI {mean - half_width:+.1f} to {mean + half_width:+.1f}  sd {standard_deviation:.1f}  (first lower in {lo}, higher in {hi})"
            )


def e1(paths):
    by_policy = load(paths)
    for tag, policy_runs in by_policy.items():
        argmax_checks = [c["argmax"] for r in policy_runs.values() for c in r["checks"]]
        diff = [a for a in argmax_checks if a["ex"] != a["sur"]]
        rel = [(a["ex_p"] - a["sur_p"]) / a["ex_p"] for a in argmax_checks if a["ex_p"]]
        print(
            f"{tag}: {len(argmax_checks)} argmax checks over {len(policy_runs)} runs (pools {min(a['pool'] for a in argmax_checks)}-{max(a['pool'] for a in argmax_checks)}): "
            f"surrogate career differs in {len(diff)}, max relative P(all) shortfall {max(rel):.2e}, unproven exact {sum(not a['proven'] for a in argmax_checks)}"
        )
        for a in diff[:5]:
            print(f"    differs: pool {a['pool']} sur {a['sur']} {a['sur_p']:.6e} ex {a['ex']} {a['ex_p']:.6e}")
        lineage_checks = [
            LineageCheck(*x, cc["argmax"]["sur_val"] or 0)
            for r in policy_runs.values()
            for cc in r["checks"]
            for x in cc["lineages"]
        ]
        with_deck = [x for x in lineage_checks if x.table_value > 0]
        relative_errors = sorted(x.relative_error for x in with_deck)
        quantile = lambda v, p: v[min(len(v) - 1, int(p * (len(v) - 1)))] if v else float("nan")
        print(
            f"  lineages {len(lineage_checks)}: trainee without a cached deck {len(lineage_checks) - len(with_deck)}; with one: relative error (exact - table)/exact "
            f"median {quantile(relative_errors, 0.5):.1e}, 99% {quantile(relative_errors, 0.99):.1e}, max {quantile(relative_errors, 1):.1e}; over 1e-9: {sum(e > 1e-9 for e in relative_errors)}"
        )
        contenders = [x for x in with_deck if x.exact >= 0.5 * x.chosen_value]
        print(
            f"  contenders (exact >= B/2) {len(contenders)}: over 1e-9 {sum(x.relative_error > 1e-9 for x in contenders)}, max {max((x.relative_error for x in contenders), default=0):.1e}; "
            f"landed-set total variation max {max((x.total_variation for x in with_deck), default=0):.1e}; bound rb / exact median {quantile(sorted(x.bound / x.exact for x in lineage_checks), 0.5):.3f}, "
            f"ceiling / exact median {quantile(sorted(x.ceiling / x.exact for x in lineage_checks), 0.5):.3f}; picks checked {sum(x.pick for x in lineage_checks)}"
        )
        bad = sorted([x for x in with_deck if x.relative_error > 1e-9], key=lambda x: -x.relative_error)[:5]
        for x in bad:
            print(
                f"    table short: exact {x.exact:.4e} table {x.table_value:.4e} ({x.relative_error:.2%}), B {x.chosen_value:.4e}, trusted {x.trusted}, tvd {x.total_variation:.3f}"
            )


KIND = {0: "greedy pick", 1: "random candidate", 3: "loop career"}


def audit(paths):
    """the `audit` records: per kind of row, the table value against a fresh exact deck search"""
    by_policy = load(paths)
    for tag, policy_runs in by_policy.items():
        audits = [a for r in policy_runs.values() for a in r.get("audits", [])]
        if not audits:
            continue
        rows = [AuditRow(*x) for a in audits for x in a.get("rows", [])]
        argmax_checks = [a for a in audits if "differs" in a]
        rotations = [RotationAudit(*a["rotation"]) for a in audits if "rotation" in a]
        print(f"{tag}: {len(policy_runs)} runs, {len(audits)} audit points, {len(rows)} rows")
        if argmax_checks:
            print(
                f"  greedy argmax vs exact career search: {len(argmax_checks)} pick changes, differs in {sum(a['differs'] for a in argmax_checks)}"
                f" (max relative shortfall {max((a['ex_p'] - a['sur_p']) / a['ex_p'] for a in argmax_checks if a['ex_p']):.1e}); pools {min(a['pool'] for a in argmax_checks)}-{max(a['pool'] for a in argmax_checks)}"
            )
            uncached_candidates = [
                (AuditRow(*x), a["sur_p"])
                for a in argmax_checks
                for x in map(AuditRow._make, a["rows"])
                if x.table_value == 0
            ]
            print(
                f"  candidates the surrogate holds no deck for (valued by bounds only): {len(uncached_candidates)}, exact P(all) / pick: max "
                f"{max((x.exact / p for x, p in uncached_candidates), default=0):.3f}, at or above the pick {sum(x.exact >= p for x, p in uncached_candidates)}"
            )
            for a in [a for a in argmax_checks if a["differs"]][:5]:
                print(
                    f"    differs at t {a['t']} pool {a['pool']}: sur {a['sur']} {a['sur_p']:.6e} exact {a['ex']} {a['ex_p']:.6e}"
                )
        for k in sorted({x.kind for x in rows}):
            kind_rows = [x for x in rows if x.kind == k]
            without_deck = [x for x in kind_rows if x.table_value == 0]
            with_deck = [x for x in kind_rows if x.table_value > 0]
            e = sorted(x.relative_error for x in with_deck)
            quantile = lambda v, p: v[min(len(v) - 1, int(p * (len(v) - 1)))] if v else float("nan")
            print(
                f"  {KIND.get(k, k)}: {len(kind_rows)} rows, no cached deck {len(without_deck)}; table vs exact relative error median {quantile(e, 0.5):.1e}, 99% {quantile(e, 0.99):.1e}, "
                f"max {quantile(e, 1):.1e}, over 1e-9 {sum(v > 1e-9 for v in e)}; landed-set TV max {max((x.total_variation for x in with_deck), default=0):.1e}; unfinished searches {sum(not x.complete for x in kind_rows)}"
            )
        if rotations:
            print(
                f"  loop keep-or-replace: {len(rotations)} audited, same decision under exact values {sum(r.same_decision for r in rotations)}; max relative score error "
                f"{max(max(abs(r.keep - r.exact_keep) / r.exact_keep if r.exact_keep else 0, abs(r.replace - r.exact_replace) / r.exact_replace if r.exact_replace else 0) for r in rotations):.1e}"
            )


if __name__ == "__main__":
    if sys.argv[1] == "e1":
        e1(sys.argv[2:])
    elif sys.argv[1] == "audit":
        audit(sys.argv[2:])
    else:
        runs(sys.argv[2:] if sys.argv[1] == "runs" else sys.argv[1:])
