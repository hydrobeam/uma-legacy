"""Refit the shape of the career planner's continuation (`Cont`, src/career/plan/cont.rs: `CEIL_SHARE`, `LAM_DECAY`)
to builder traces measured with the policy simulator's `trace` policy, and evaluate it on pools left out of the fit.

Per trace pool (val/sim/trace_pools.json: a career job, optionally its first `legacies` veterans only):
1. plan the pool (`next`), and take the cheapest trio's two stage candidates (`v0_plan` a and b, each one that has no
   full veteran of her own) as the trainees built for, A; her builders are the plan's five best-scoring builder rows'
   umas other than A;
2. `trace` for A with those builders (`runs` x `careers`, at `margin`): each career the most efficient builder runs and
   her veteran joins; a row records A's best odds before, the builder, her lift chance lam and A's mean lifted odds p1,
   and A's odds after;
3. from the traces: A's odds after n careers (geometric mean over runs; a run that stops early keeps its last odds),
   and the achievable cost min over k of (k + mean over runs of 1 / p_k): build k careers, then grind. That minimum
   over noisy means reads low in sample, so it is also given split in half (k chosen on the even runs, priced on the
   odd ones, and the reverse, averaged).

Cont at a shape (s, d), restated from cont.rs at A's start: p0 her odds, lam and p1 the first trace row's (the most
efficient builder's), C her full-loop odds (the plan's fit entry: C = pinf / CEIL_SHARE where pinf is above p1; where
pinf is p1 the ceiling did not bind and C is unknown, so that trace constrains d alone). pinf = max(s C, p1),
phi = ln(p1 / p0) / ln(pinf / p0), success j closes phi of the log-odds gap and takes 1 / (lam ((1 - phi)^j)^d) careers;
its odds after n careers are those after the successes completed by n, its cost cont.rs's `plan`: the first local
minimum over n of the careers built + 1 / p_n. The fit: least squares on log odds at the checkpoints (10, 25, 50, 100,
200, 400 careers, those within `careers`) over every trace, on a grid of s and d. Held out: each pool's traces are
scored under the shape fitted without that pool (its log-odds loss, and its cost error against its split-half
achievable cost), with the totals over pools; this, not the in-sample fit, is the evidence for a shape.

Two continuations are reported per trace and kept apart. "The plan's Cont" is the plan's own fit entry for A,
[chara, lam, phi, pinf, p0] (p0 the trace's start where the entry has four elements), priced as cont.rs prices it at
p0: the continuation the page uses, its lam and p1 from whichever builder the plan takes them from. "Cont from the
trace" takes lam and p1 from the trace's own first row (its most efficient builder) and the shape (s, d): the form
being fitted. Also priced: that form under the variant that takes lam as the larger of her lift chance now and one lift
later (the mean over runs of the first row after her odds first rose), an unbuilt form docs/MODEL.md names.

The value of a lift (reported, not fitted on): what a score differences is K(p0) - K(p'), so per trace the plan's Cont's
dK at m = 1, 2, 4 lifts (p' at q = p0 (p1 / p0)^m, p1 the plan's first lift) is printed beside the segment estimator's:
each run's first arrival at odds >= q, then the forward open-loop cost min over k of k + mean 1 / p(t_a + k) from the
arrivals against the same from the start, at the same horizon (careers - the latest arrival, at least 50); with 3 or
more arriving runs. Cont's dK is the mean over the arriving runs of K(p0) - K(p_a), K cont.rs's Cont (`PlanCont`).
The test is their paired difference: 300 bootstrap draws of the runs (seed 1), each recomputing the segment and Cont's
dK at that draw's own arrival odds; its 90% interval, and the count of draws that failed (under 3 arriving runs or a
horizon under 50), on which the interval is conditional. Each cell gives how many runs arrive; a cell with no segment
says why (how many arrive, the horizon). The estimator pools nothing by odds and has no
bandwidth; it is open loop, so it reads a lift's value low where the traces' hazard falls with careers elapsed.

usage: uv run val/sim/fit_cont.py BIN OUT_DIR [--runs 6] [--careers 400] [--margin 1e9] [-j 2] [--fit-only] [POOL ...]
  writes OUT_DIR/NAME_plan.{json,out} and OUT_DIR/NAME_A.{json,out} (A the trainee's id), then the fit; --fit-only
  reads the outputs already there. Cost: a trace run is about 5 s per 400 careers on these pools; 9 pools x 2
  trainees x 6 runs is about 10 minutes on two cores. A `--margin 0` run sizes how much of the plateau comes from the
  traces' cached decks (margin 1e9 searches a trainee's deck only when she has none)."""

import json
import math
import os
import random
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from typing import NamedTuple

SIM_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(SIM_DIR))
CHECKPOINTS = (10, 25, 50, 100, 200, 400)
BUILDERS = 5
CONT_N = 500  # cont.rs: successes before grinding, at most
SHARES = [round(0.3 + 0.05 * i, 2) for i in range(19)]  # 0.3 .. 1.2
DECAYS = [round(0.5 + 0.1 * i, 1) for i in range(46)]  # 0.5 .. 5.0


def cont_constant(name):
    """`CEIL_SHARE` or `LAM_DECAY` as cont.rs states it"""
    src = open(os.path.join(REPO, "pkg", "loopopt", "src", "career", "plan", "cont.rs")).read()
    return float(re.search(rf"const {name}: f64 = ([\d.]+);", src).group(1))


CEIL_SHARE, LAM_DECAY = cont_constant("CEIL_SHARE"), cont_constant("LAM_DECAY")


class Trace(NamedTuple):
    """One pool's traces for one trainee built for, and what Cont is fitted from."""

    pool: str
    trainee: str
    p0: float
    lam: float
    p1: float
    ceiling: float | None  # her full-loop odds C, None where the plan's pinf was p1
    lam_after_lift: float | None  # lam at the first row after her odds first rose, mean over runs
    odds: list  # per run, A's odds after n careers (n = 0..careers)
    plan_cont: tuple | None  # the plan's fit entry for her as (lam, phi, pinf, p0), None without one


def last_line(path):
    return json.loads([line for line in open(path).read().splitlines() if line.strip()][-1])


def run_solver(binary, era, job, path):
    json.dump(job, open(path + ".json", "w"))
    result = subprocess.run(
        [binary, "data.json", path + ".json"],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", era),
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"{path}: solver exited {result.returncode}\n{result.stderr}")
    open(path + ".out", "w").write(result.stdout)
    return last_line(path + ".out")


def pool_job(entry):
    job = json.load(open(os.path.join(REPO, entry["pool"])))
    if "legacies" in entry:
        job["legacies"] = job["legacies"][: entry["legacies"]]
    return job


def trainees_built_for(plan):
    """(A's id, her name, her builders' ids) for each stage candidate of the cheapest trio without a full veteran"""
    ids = {}
    for row in plan["options"] + [plan["choice"]]:
        ids[row["trainee"]["name"]] = row["trainee"]["id"]
    members = {member["trainee"]: member for member in plan.get("members", [])}
    out = []
    for role in ("a", "b"):
        name = plan["v0_plan"][role]
        if members.get(name, {}).get("real") or name not in ids:
            continue
        builders = []
        for row in sorted(plan["options"], key=lambda row: -row["score"]):
            builder = row["trainee"]["id"]
            if row["role"] == "builder" and builder != ids[name] and builder not in builders:
                builders.append(builder)
        out.append((ids[name], name, builders[:BUILDERS]))
    return out


def fit_entry(plan, trainee_id):
    """The plan's fit entry for her: [chara, lam, phi, pinf] (a fifth element, where present, is p0)"""
    return next((entry for entry in plan.get("fit") or [] if entry[0] == trainee_id), None)


def measure(binary, out_dir, entry, settings):
    """Plan the pool, then trace each trainee built for (or read both from out_dir with --fit-only)"""
    name, era = entry["name"], entry["era"]
    job = pool_job(entry)
    plan_path = os.path.join(out_dir, f"{name}_plan")
    if settings["fit_only"]:
        plan = last_line(plan_path + ".out").get("next") or {}
    else:
        plan = run_solver(binary, era, dict(job, next={**job.get("next", {})}), plan_path).get("next") or {}
    if not plan.get("v0_plan") or not plan.get("choice"):
        print(f"{name}: no plan with a trio ({plan.get('error', 'no next')}), skipped")
        return []
    traces = []
    for trainee_id, trainee_name, builders in trainees_built_for(plan):
        trace_path = os.path.join(out_dir, f"{name}_{trainee_id}")
        simulate = {
            "policy": "trace",
            "runs": settings["runs"],
            "max_careers": settings["careers"],
            "margin": settings["margin"],
            "loop_members": [trainee_id, *builders],
        }
        if settings["fit_only"]:
            output = last_line(trace_path + ".out")
        else:
            output = run_solver(binary, era, dict(job, simulate=simulate), trace_path)
        traces.append(trace_of(name, trainee_name, fit_entry(plan, trainee_id), output, settings["careers"]))
    return [trace for trace in traces if trace]


def trace_of(pool, trainee, fit, output, careers):
    runs = [run["rows"] for run in output.get("trace", []) if run["rows"]]
    if not runs:
        return None
    first = runs[0][0]
    p0, lam, p1 = first[1], first[3], first[4]
    return Trace(
        pool=pool,
        trainee=trainee,
        p0=p0,
        lam=lam,
        p1=p1,
        ceiling=full_loop_odds(fit, p0),
        lam_after_lift=lam_after_first_lift(runs, p0),
        odds=odds_paths(runs, careers),
        plan_cont=(fit[1], fit[2], fit[3], fit[4] if len(fit) > 4 else p0) if fit else None,
    )


def full_loop_odds(fit, p0):
    """C from the plan's fit entry: pinf / CEIL_SHARE where pinf is above the plan's own p1 (p1 = p (pinf / p)^phi at
    its odds p: the fifth element where the solver gives it, else the trace's start), None where pinf is p1"""
    if not fit:
        return None
    _chara, _lam, phi, pinf = fit[:4]
    p = fit[4] if len(fit) > 4 else p0
    plan_p1 = p * (pinf / p) ** phi
    return pinf / CEIL_SHARE if pinf > plan_p1 * (1 + 1e-9) else None


def lam_after_first_lift(runs, p0):
    """the mean over runs of lam at the first row after her odds first rose (None where none rose in time)"""
    after_lift = []
    for rows in runs:
        risen = next((i for i, row in enumerate(rows) if row[5] > p0 * (1 + 1e-12)), None)
        if risen is not None and risen + 1 < len(rows):
            after_lift.append(rows[risen + 1][3])
    return sum(after_lift) / len(after_lift) if after_lift else None


def odds_paths(runs, careers):
    """per run, her odds after n = 0..careers careers (a run that stopped keeps its last odds)"""
    odds = []
    for rows in runs:
        path = [rows[0][1]] + [row[5] for row in rows]
        odds.append(path + [path[-1]] * (careers + 1 - len(path)))
    return odds


# ---- Cont, restated from cont.rs ----
class Shape(NamedTuple):
    share: float
    decay: float


def cont_of(trace, shape, lam=None):
    """(lam, phi, pinf) at a shape, from the trace's start"""
    lam = trace.lam if lam is None else lam
    pinf = max(shape.share * trace.ceiling, trace.p1) if trace.ceiling else trace.p1
    phi = 0.0
    if pinf > trace.p0 and trace.p1 > trace.p0:
        phi = min(1.0, max(0.0, math.log(trace.p1 / trace.p0) / math.log(pinf / trace.p0)))
    return lam, phi, pinf


def success_times(lam, phi, decay, horizon):
    """cumulative careers to each success, while under `horizon`"""
    times, built, r = [], 0.0, 1 - phi
    for j in range(CONT_N):
        rate = lam * (r**j) ** decay
        if rate <= 0:  # (phi = 1: the gap closed at success j, cont.rs's 1 / 0 = infinity)
            break
        built += 1 / rate
        if built > horizon:
            break
        times.append(built)
    return times


def odds_after(trace, shape, careers):
    lam, phi, pinf = cont_of(trace, shape)
    if lam <= 0 or phi <= 0:
        return trace.p0
    successes = len(success_times(lam, phi, shape.decay, careers))
    return pinf * math.exp(math.log(trace.p0 / pinf) * (1 - phi) ** successes)


def cont_cost(trace, shape, lam=None):
    """Cont from the trace at a shape, priced at p0 (`plan_cost`)"""
    lam, phi, pinf = cont_of(trace, shape, lam)
    return plan_cost(lam, phi, pinf, trace.p0, shape.decay)


def plan_cost(lam, phi, pinf, p, decay):
    """cont.rs `Cont::plan` at odds p: the first local minimum over n of careers built + 1 / p_n"""
    if lam <= 0 or phi <= 0 or pinf <= p:
        return 1 / p
    r, log_ratio = 1 - phi, math.log(p / pinf)
    best, built = 1 / p, 0.0
    for n in range(CONT_N):
        rate = lam * (r**n) ** decay
        if rate <= 0:  # (as success_times)
            break
        built += 1 / rate
        cost = built + 1 / (pinf * math.exp(log_ratio * r ** (n + 1)))
        if cost >= best:
            break
        best = cost
    return best


def reciprocal(x):
    """1 / x, infinity at 0 (Rust's f64 division, where a lift chance underflows or phi = 1 closes the gap)"""
    return math.inf if x == 0 else 1 / x


class PlanCont:
    """cont.rs's `Cont` (`with_decay` at LAM_DECAY) restated, for the plan's fit entry (lam, phi, pinf, p0 = its anchor
    odds): `cost(p)` is `Cont::cost`, the lattice of p0's path above p0 (`lattice_points`, `lattice_plan`) and `plan_n`
    at or below it, with `building_scale` and `Building`'s rate capped at lam below p0. The same expressions in the
    same order; not bit-exact (Python's ** for Rust's powi)."""

    def __init__(self, lam, phi, pinf, p0):
        self.lam, self.phi, self.pinf, self.p0 = lam, phi, pinf, p0
        r = 1 - phi
        self.gap_left = [r**n for n in range(CONT_N + 1)]
        self.step = [reciprocal(lam * gap**LAM_DECAY) for gap in self.gap_left]
        self.careers_built, built = [], 0.0
        for step in self.step:
            built += step
            self.careers_built.append(built)
        self.lattice = self.lattice_points() if lam > 0 and phi > 0 and 0 < p0 < pinf else None

    def lattice_points(self):
        odds = lambda j: self.pinf * math.exp(math.log(self.p0 / self.pinf) * self.gap_left[j])
        points = [0.0] * CONT_N + [1 / odds(CONT_N)]
        for j in range(CONT_N - 1, -1, -1):
            points[j] = min(1 / odds(j), self.step[j] + points[j + 1])
        points[0] = self.plan_n(self.p0)
        for j in range(1, CONT_N + 1):
            points[j] = min(points[j], points[j - 1])
        return points

    def building_scale(self, p):
        if not 0 < self.p0 < self.pinf or p == self.p0:
            return 1.0
        return (math.log(p / self.pinf) / math.log(self.p0 / self.pinf)) ** -LAM_DECAY

    def plan_n(self, p):
        """the first n that does not improve ends the minimum of careers built + 1 / p_n; below p0 each success's
        waiting time is the scaled table step, at least 1 / lam (`Building`)"""
        log_ratio, scale = math.log(p / self.pinf), self.building_scale(p)
        best, running = 1 / p, 0.0
        for n in range(CONT_N):
            if scale >= 1:
                built = scale * self.careers_built[n]
            else:
                running += max(1 / self.lam, scale * self.step[n])
                built = running
            cost = built + 1 / (self.pinf * math.exp(log_ratio * self.gap_left[n + 1]))
            if cost >= best:
                break
            best = cost
        return best

    def lattice_plan(self, p):
        g = math.log(p / self.pinf) / math.log(self.p0 / self.pinf)
        r = 1 - self.phi
        t = max(0.0, math.log(g) / math.log(r)) if r > 0 else 0.0
        j = math.floor(t)
        if j >= CONT_N:
            cost = self.lattice[CONT_N]
        else:
            low, high = self.lattice[j], self.lattice[j + 1]
            cost = min(max(low + (t - j) * (high - low), min(low, high)), max(low, high))
        return min(cost, 1 / p)

    def lift_value(self, arrival_odds):
        """the mean over these odds of K(p0) - K(p)"""
        return sum(self.cost(self.p0) - self.cost(p) for p in arrival_odds) / len(arrival_odds)

    def cost(self, p):
        if p <= 0:
            return math.inf
        if self.lam <= 0 or self.phi <= 0 or self.pinf <= p:
            return 1 / p
        if p > self.p0 and self.lattice:
            return self.lattice_plan(p)
        return self.plan_n(p)


# ---- the traces' own numbers ----
def mean_log_odds(trace, careers):
    return sum(math.log(run[careers]) for run in trace.odds) / len(trace.odds)


def achievable(trace, runs=None):
    """min over k of k + mean over `runs` (default all) of 1 / p_k, and the k it takes"""
    runs = trace.odds if runs is None else runs
    costs = [k + sum(1 / run[k] for run in runs) / len(runs) for k in range(len(runs[0]))]
    k = min(range(len(costs)), key=costs.__getitem__)
    return costs[k], k


def achievable_split(trace):
    """k chosen on half the runs, priced on the other half, both ways, averaged (None under 2 runs)"""
    even, odd = trace.odds[::2], trace.odds[1::2]
    if not odd:
        return None
    price = lambda runs, k: k + sum(1 / run[k] for run in runs) / len(runs)
    return (price(odd, achievable(trace, even)[1]) + price(even, achievable(trace, odd)[1])) / 2


LIFT_MULTIPLES = (1, 2, 4)
SEGMENT_MIN_HORIZON = 50
BOOTSTRAP_DRAWS = 300
ROUNDING = 1e-9  # careers: a difference this small is 0 (a grinding trace's Cont and segment agree to ~1e-13)


class Segment(NamedTuple):
    """The segment estimator's value of reaching odds q: the open-loop cost from the start less that from the arrivals"""

    saving: float
    arrival_odds: list  # per arriving run, her odds on arrival


def arrivals_at(paths, q):
    """(career of first arrival at odds >= q, the path) for each run that arrives"""
    arrivals = []
    for path in paths:
        t = next((i for i, p in enumerate(path) if p >= q * (1 - 1e-12)), None)
        if t is not None:
            arrivals.append((t, path))
    return arrivals


def segment(paths, q, careers):
    """Segment at q over these runs' odds paths (module notes), None under 3 arriving runs or a horizon under 50"""
    arrivals = arrivals_at(paths, q)
    if len(arrivals) < 3:
        return None
    horizon = careers - max(t for t, _ in arrivals)
    if horizon < SEGMENT_MIN_HORIZON:
        return None
    from_start = min(k + sum(1 / path[k] for path in paths) / len(paths) for k in range(horizon + 1))
    from_arrival = min(k + sum(1 / path[t + k] for t, path in arrivals) / len(arrivals) for k in range(horizon + 1))
    return Segment(saving=from_start - from_arrival, arrival_odds=[path[t] for t, path in arrivals])


def lift_value_report(traces, careers):
    """dK at m = 1, 2, 4 lifts: the plan's Cont beside the segment estimator's, and their paired difference with its
    bootstrap 90% interval (module notes)"""
    print(
        "\nthe value of a lift, K(p0) - K(p') at p' = p0 (p1 / p0)^m (reported, not fitted on): the plan's Cont / the"
        " traces' segment estimator, their difference [bootstrap 90%] (runs arriving of all; bootstrap draws that failed)"
    )
    print("| pool | A | " + " | ".join(f"m = {m}" for m in LIFT_MULTIPLES) + " |")
    print("|---|---|" + "---|" * len(LIFT_MULTIPLES))
    draws = random.Random(1)
    sides = {m: {"below": 0, "contains 0": 0, "above": 0} for m in LIFT_MULTIPLES}  # the difference's interval
    for trace in traces:
        if not trace.plan_cont:
            continue
        cont = PlanCont(*trace.plan_cont)
        _lam, phi, pinf, p0 = trace.plan_cont
        p1 = p0 * (pinf / p0) ** phi
        cells = []
        for m in LIFT_MULTIPLES:
            q = p0 * (p1 / p0) ** m
            found = segment(trace.odds, q, careers) if p1 > p0 else None
            if found is None:
                arrivals = arrivals_at(trace.odds, q) if p1 > p0 else []
                horizon = f", horizon {careers - max(t for t, _ in arrivals)}" if arrivals else ""
                cells.append(f"- ({len(arrivals)} of {len(trace.odds)} arrive{horizon})" if p1 > p0 else "- (no lift)")
                continue
            # each resample's segment and Cont's dK at that resample's own arrival odds, paired
            differences, failed = [], 0
            for _ in range(BOOTSTRAP_DRAWS):
                again = segment([draws.choice(trace.odds) for _ in trace.odds], q, careers)
                if again is None:
                    failed += 1
                else:
                    differences.append(cont.lift_value(again.arrival_odds) - again.saving)
            difference = cont.lift_value(found.arrival_odds) - found.saving
            cell = f"{cont.lift_value(found.arrival_odds):,.0f} / {found.saving:,.0f}, {difference:+,.0f}"
            if len(differences) > 20:
                differences.sort()
                low, high = differences[int(0.05 * len(differences))], differences[int(0.95 * len(differences)) - 1]
                sides[m]["below" if high < -ROUNDING else "above" if low > ROUNDING else "contains 0"] += 1
                cell += f" [{low:+,.0f}, {high:+,.0f}]"
            else:
                cell += " [-]"
            cells.append(cell + f" ({len(found.arrival_odds)} of {len(trace.odds)}; {failed}/{BOOTSTRAP_DRAWS} failed)")
        print(f"| {trace.pool} | {trace.trainee} | " + " | ".join(cells) + " |")
    print(
        "Cont's dK - the segment's, its interval below 0 / containing 0 / above 0: "
        + "; ".join(f"m = {m} {c['below']} / {c['contains 0']} / {c['above']}" for m, c in sides.items())
        + " (each interval conditional on the resamples that had 3 arriving runs and a horizon of 50)"
    )


def loss(traces, shape, careers):
    total = 0.0
    for trace in traces:
        for n in CHECKPOINTS:
            if n <= careers:
                total += (math.log(odds_after(trace, shape, n)) - mean_log_odds(trace, n)) ** 2
    return total


def cost_loss(traces, shape):
    """squared log error of Cont's cost against the split achievable cost, over the traces that have one: the quantity
    the plan reads (a step's cost), where `loss` fits the odds path"""
    return sum(math.log(cont_cost(t, shape) / achievable_split(t)) ** 2 for t in traces if achievable_split(t))


def best_shape(traces, careers, criterion="odds"):
    objective = (lambda s: loss(traces, s, careers)) if criterion == "odds" else (lambda s: cost_loss(traces, s))
    return min((Shape(share, decay) for share in SHARES for decay in DECAYS), key=objective)


def cost_errors(traces, shape):
    return [cont_cost(t, shape) / achievable_split(t) - 1 for t in traces if achievable_split(t)]


def cost_report(traces, current):
    """the shape fitted on cost, and each pool's cost error under the cost shape fitted without it, beside the current
    shape's (not fitted on these traces)"""
    fitted = best_shape(traces, 0, "cost")
    edge = fitted.share in (SHARES[0], SHARES[-1]) or fitted.decay in (DECAYS[0], DECAYS[-1])
    mean = lambda errors: sum(abs(e) for e in errors) / len(errors) if errors else float("nan")
    print(
        f"\ncost-fitted shape: CEIL_SHARE {fitted.share}, LAM_DECAY {fitted.decay}; cost loss "
        f"{cost_loss(traces, fitted):.3f} (current {cost_loss(traces, current):.3f})"
        + ("  (at the grid's edge)" if edge else "")
    )
    pools = sorted({trace.pool for trace in traces})
    print("| pool | cost shape without it | its cost errors there | under the current shape |")
    print("|---|---|---|---|")
    held, now = [], []
    for pool in pools:
        own = [t for t in traces if t.pool == pool]
        shape = best_shape([t for t in traces if t.pool != pool], 0, "cost")
        pool_held, pool_now = cost_errors(own, shape), cost_errors(own, current)
        held += pool_held
        now += pool_now
        print(
            f"| {pool} | {shape.share}, {shape.decay} | {', '.join(f'{e:+.1%}' for e in pool_held)} | "
            f"{', '.join(f'{e:+.1%}' for e in pool_now)} |"
        )
    print(f"mean |cost error|: held out {mean(held):.1%}; current shape {mean(now):.1%}")


def report(traces, careers):
    current = Shape(CEIL_SHARE, LAM_DECAY)
    fitted = best_shape(traces, careers)
    print(
        f"current shape: CEIL_SHARE {current.share}, LAM_DECAY {current.decay}; loss {loss(traces, current, careers):.3f}"
    )
    edge = fitted.share in (SHARES[0], SHARES[-1]) or fitted.decay in (DECAYS[0], DECAYS[-1])
    print(
        f"fitted shape:  CEIL_SHARE {fitted.share}, LAM_DECAY {fitted.decay}; loss {loss(traces, fitted, careers):.3f}"
        + ("  (at the grid's edge: widen SHARES / DECAYS)" if edge else "")
    )
    pools = sorted({trace.pool for trace in traces})
    held_out = {}
    if len(pools) > 1:
        print("\nheld out (each pool scored under the shape fitted without it):")
        print(
            "| pool | shape without it | its log-odds loss there (in the full fit) | Cont cost / split achievable - 1 |"
        )
        print("|---|---|---|---|")
        total_loss, errors = 0.0, []
        for pool in pools:
            own = [t for t in traces if t.pool == pool]
            shape = best_shape([t for t in traces if t.pool != pool], careers)
            held_out[pool] = shape
            pool_loss = loss(own, shape, careers)
            total_loss += pool_loss
            pool_errors = [cont_cost(t, shape) / achievable_split(t) - 1 for t in own if achievable_split(t)]
            errors += pool_errors
            print(
                f"| {pool} | {shape.share}, {shape.decay} | {pool_loss:.3f} ({loss(own, fitted, careers):.3f}) | "
                + ", ".join(f"{error:+.1%}" for error in pool_errors)
                + " |"
            )
        mean_error = sum(abs(error) for error in errors) / len(errors) if errors else float("nan")
        print(f"held-out loss {total_loss:.3f} over {len(pools)} pools; mean |cost error| {mean_error:.1%}")
        cost_report(traces, current)
    print(
        "\n| pool | A | p0 | C | odds at "
        + " / ".join(str(n) for n in CHECKPOINTS if n <= careers)
        + " (traces) | achievable (split) | the plan's Cont | Cont from the trace: now / fitted / held out |"
        + " lam = max(now, one lift later) |"
    )
    print("|---|---|---|---|---|---|---|---|---|")
    for trace in traces:
        observed = " / ".join(f"1/{math.exp(-mean_log_odds(trace, n)):,.0f}" for n in CHECKPOINTS if n <= careers)
        cost, k = achievable(trace)
        split = achievable_split(trace)
        variant_lam = max(trace.lam, trace.lam_after_lift) if trace.lam_after_lift else trace.lam
        plan_cont = f"{plan_cost(*trace.plan_cont, LAM_DECAY):,.0f}" if trace.plan_cont else "-"
        held = f"{cont_cost(trace, held_out[trace.pool]):,.0f}" if trace.pool in held_out else "-"
        print(
            f"| {trace.pool} | {trace.trainee} | 1/{1 / trace.p0:,.0f} | "
            f"{f'1/{1 / trace.ceiling:,.0f}' if trace.ceiling else 'p1 (unbound)'} | {observed} | "
            f"{cost:,.0f} at k {k} ({f'{split:,.0f}' if split else '-'}) | {plan_cont} | "
            f"{cont_cost(trace, current):,.0f} / {cont_cost(trace, fitted):,.0f} / {held} | "
            f"{cont_cost(trace, current, variant_lam):,.0f} |"
        )
    lift_value_report(traces, careers)


def main(args):
    def option(flag, default, kind):
        if flag not in args:
            return default
        i = args.index(flag)
        value = kind(args[i + 1])
        del args[i : i + 2]
        return value

    fit_only = "--fit-only" in args
    if fit_only:
        args.remove("--fit-only")
    settings = {
        "runs": option("--runs", 6, int),
        "careers": option("--careers", 400, int),
        "margin": option("--margin", 1e9, float),
        "fit_only": fit_only,
    }
    parallelism = option("-j", 2, int)
    binary, out_dir, wanted = os.path.abspath(args[0]), args[1], set(args[2:])
    os.makedirs(out_dir, exist_ok=True)
    entries = [
        e for e in json.load(open(os.path.join(SIM_DIR, "trace_pools.json"))) if not wanted or e["name"] in wanted
    ]
    with ThreadPoolExecutor(parallelism) as executor:
        traces = [t for pool in executor.map(lambda e: measure(binary, out_dir, e, settings), entries) for t in pool]
    if not traces:
        print("no traces")
        return 1
    report(traces, settings["careers"])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
