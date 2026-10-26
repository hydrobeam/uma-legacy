"""Test `LAM_DECAY` (src/career/plan/cont.rs) on val/mono.py's plans, with no plan of its own.

The continuation anchored at a candidate's odds p0 now prices her lift chance at later odds p as
lam(p) = lam0 g(p)^LAM_DECAY, g(p) = min(1, ln(pinf / p) / ln(pinf / p0)) (cont.rs). A refit at the pool with a rolled
veteran added measures her lift chance afresh at her new odds p' (the refit's fit entry: [chara, lam, phi, pinf, p0]
with p0 = p'). Over every candidate in both a pool's plan and a rolled veteran's refit whose odds rose (p' > p0, so
g < 1), the decay the refits imply is the slope of ln(lam_refit / lam0) on ln g through the origin; LAM_DECAY is
consistent with them if that slope's interval covers it. Also reported: lam_refit / lam(p') per candidate.

A refit's lam is the cost-minimising builder's at the new pool, which also changes the builders (her veteran can be
a builder's parent), so this tests the anchored decay as the plan applies it, not a law of building.

usage: uv run val/sim/lam_regress.py MONO_DIR    (the folder val/mono.py prints at its end)"""

import glob
import json
import math
import os
import re
import statistics
import sys

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def lam_decay():
    src = open(os.path.join(REPO, "pkg", "loopopt", "src", "career", "plan", "cont.rs")).read()
    return float(re.search(r"const LAM_DECAY: f64 = ([\d.]+);", src).group(1))


def fit_of(path):
    lines = [line for line in open(path).read().splitlines() if line.strip()]
    plan = json.loads(lines[-1]).get("next") or {}
    return {entry[0]: entry for entry in plan.get("fit") or [] if len(entry) >= 5}


def implied_decay(points):
    """the slope of ln(lam_refit / lam0) on ln g through the origin, with a 95% normal interval"""
    xs, ys = [p[3] for p in points], [p[4] for p in points]
    slope = sum(x * y for x, y in zip(xs, ys)) / sum(x * x for x in xs)
    residuals = [y - slope * x for x, y in zip(xs, ys)]
    standard_error = math.sqrt(sum(r * r for r in residuals) / (len(xs) - 1) / sum(x * x for x in xs))
    return slope, slope - 1.96 * standard_error, slope + 1.96 * standard_error


def main(mono_dir):
    decay = lam_decay()
    points = []  # (pool, roll, chara, ln g, ln(lam_refit / lam0), lam_refit / lam(p'), lam0)
    for base_path in sorted(glob.glob(os.path.join(mono_dir, "*_base.out"))):
        pool = os.path.basename(base_path)[: -len("_base.out")]
        base = fit_of(base_path)
        for refit_path in sorted(glob.glob(os.path.join(mono_dir, f"{pool}_r*_refit.out"))):
            roll = os.path.basename(refit_path)[len(pool) + 1 : -len("_refit.out")]
            for chara, (_, lam_refit, _phi, _pinf, p_new) in fit_of(refit_path).items():
                if chara not in base:
                    continue
                _, lam0, _, pinf, p0 = base[chara]
                if not (p0 < p_new < pinf) or lam0 <= 0 or lam_refit <= 0:
                    continue
                g = min(1.0, math.log(pinf / p_new) / math.log(pinf / p0))
                anchored = lam0 * g**decay
                points.append((pool, roll, chara, math.log(g), math.log(lam_refit / lam0), lam_refit / anchored, lam0))
    if len(points) < 2:
        print(f"{len(points)} candidates whose odds rose: nothing to fit")
        return 1
    slope, low, high = implied_decay(points)
    ratios = sorted(p[5] for p in points)
    print(f"{len(points)} candidates whose odds rose with a rolled veteran, over {len({p[0] for p in points})} pools")
    print(f"implied decay {slope:.2f} (95% {low:.2f} .. {high:.2f}) against LAM_DECAY {decay}")
    # (robustness: lam can sit at its cap of 1, where it cannot fall; and one pool can carry the fit)
    unsaturated = [p for p in points if p[6] < 0.999]
    if len(unsaturated) > 1:
        slope, low, high = implied_decay(unsaturated)
        print(f"without lam0 at 1: {slope:.2f} (95% {low:.2f} .. {high:.2f}), {len(unsaturated)} candidates")
    for pool in sorted({p[0] for p in points}):
        rest = [p for p in points if p[0] != pool]
        if len(rest) > 1:
            slope, low, high = implied_decay(rest)
            print(f"without {pool}: {slope:.2f} (95% {low:.2f} .. {high:.2f})")
    print(
        f"lam_refit / lam(p'): median {statistics.median(ratios):.3f}, range {ratios[0]:.3f} .. {ratios[-1]:.3f}, "
        f"above 1 in {sum(r > 1 for r in ratios)}"
    )
    print("| pool | roll | chara | g | lam_refit / lam0 | lam_refit / lam(p') |")
    print("|---|---|---|---|---|---|")
    for pool, roll, chara, log_g, log_ratio, ratio, _ in points:
        print(f"| {pool} | {roll} | {chara} | {math.exp(log_g):.3f} | {math.exp(log_ratio):.3f} | {ratio:.3f} |")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
