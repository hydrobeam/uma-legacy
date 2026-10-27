"""The career planner's next-plan jobs (val/plan/jobs.json): run them, then compare each output with the reference
strictly (the whole JSON, equal values; keys in SKIP and keys ending in _ms are ignored). For solver work that must
not move the planner's results; `val/runval.py` does not cover these jobs.

Then the proxy gate: each job again with `next.proxy_check`, which searches every builder exactly and prices each
builder row through its deck-class proxy beside its exact career. The plan admits a builder for an exact search by its
proxy efficiency (within PROXY_EFF 6% of a candidate's second best) and promotes a proxy row by its proxy saving + 1
(within PROXY_SAVE 6%), so a proxy that reads low can keep a better row out. The gate fails a job on any of:

- the plan moves: the plan as it runs (proxies on) and the all-exact plan differ in the pick (its career id), in V
  (`v0`, to the bit) or in the set of the top OPT_SHOW = 8 rows by score;
- an efficiency more than PROXY_GATE (3%, half the band) low: each candidate's, where the exact one is at least 1% of
  the row's best (well conditioned: the band compares efficiencies near the best);
- a saving more than SAVING_GATE (0.25 careers) low, in careers: proxy score against exact score. (A relative error on
  saving + 1 is ill-conditioned once honest savings sit near 0; it is reported, not failed on.) The two scores must
  be priced on the same deck cache for this to measure the proxy: `proxy_check` (rows.rs `check_plan_proxies`) scores
  proxies on the cache from before the builders' searches, so until that is changed a proxy that is the exact career
  itself can read low here, and the gate fails on the cache, not the proxy.

Then the first-step tripwire: the row that runs V's route first (`route_first_step`) should score about 0, since by
policy improvement the route's own next step saves nothing over V (a career costs 1 and V's route promises it saves 1:
for a building side Cont's lam (K(p) - K(p1)) = 1). It fails a job when that score is below FIRST_STEP_FLOOR (-0.1
careers): V's own step pricing below what V promises for it, as an Alternative side whose lift no route credits does.
It prints the split of the row's saving (score + 1): the builder's own full-veteran term (p_all x her own side's careers,
where she is a stage side without a real full veteran), the remainder (what the row credits to the side it builds), the
1 promised, and `routes_dropped` (routes the scorer's memory budget left out, which can also price the step low). A
score above FIXED_POINT_GAP (0.1) is reported, not failed on, as "V is not its own fixed point": the step saves more
than V's route promises (by-product and cross-trio credit V's additive sides lack, among others); it does not say which
way V errs against the true cost. The 0.1 is assumed: the gap between mean-lifted and per-outcome odds (Jensen) is not
measured. A plan with a finite V whose first step has no finite score (a missing row, a null or NaN score) fails; only
a plan with no V is not applicable.

Base rows only: a looped row (her career on two full veterans) is priced against her unconstrained proxy, which is her
base row's career; rows marked `looped` are dropped, or, from a solver that does not mark them, every row but each
trainee's best by exact odds. A high-side error (a proxy that reads high only costs a search) is reported, not failed
on.

usage: planval.py BIN TAG [REF] [-j N]
  writes val/plan/TAG__NAME.out, val/plan/TAG_proxy__NAME.out and val/plan/TAG_times.txt; compares with
  val/plan/REF__NAME.out (default REF: ref)
  -j N  jobs at a time (default 2; timings are only comparable at the same load)"""

import json
import math
import os
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from typing import NamedTuple

PROXY_GATE = 0.03
SAVING_GATE = 0.25  # careers
FIRST_STEP_FLOOR = -0.1  # careers: V's own first step scoring below this fails (module notes; assumed)
FIXED_POINT_GAP = 0.1  # careers: above this V's own first step is reported as V not its own fixed point
OPT_SHOW = 8  # rows the page shows (src/career/plan: OPT_SHOW)


class CapWarning(NamedTuple):
    class_index: int
    ratio: float
    cap: float


VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(VALIDATION_DIR)
PLAN_DIR = os.path.join(VALIDATION_DIR, "plan")
args = sys.argv[1:]
parallelism = 2
if "-j" in args:
    i = args.index("-j")
    parallelism = int(args[i + 1])
    del args[i : i + 2]
binary, tag = os.path.abspath(args[0]), args[1]
ref = args[2] if len(args) > 2 else "ref"
jobs = json.load(open(os.path.join(PLAN_DIR, "jobs.json")))
times = {}


def run(job_entry):
    era, name = job_entry
    started = time.time()
    result = subprocess.run(
        [binary, "data.json", os.path.join(PLAN_DIR, name + ".json")],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", era),
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"{name}: solver exited {result.returncode}\n{result.stderr}")
    open(os.path.join(PLAN_DIR, f"{tag}__{name}.out"), "w").write(result.stdout)
    times[name] = round(time.time() - started, 2)


with ThreadPoolExecutor(parallelism) as ex:
    list(ex.map(run, jobs))
open(os.path.join(PLAN_DIR, f"{tag}_times.txt"), "w").write(
    "".join(f"{name} {s}\n" for name, s in sorted(times.items()))
)
# This comparison skips work and bound diagnostics as well as timings; it does not enforce byte-identical outputs.
SKIP = {
    "timing_s",
    "work",
    "bounded",
    "solved",
    "scored",
    "skipped",
    "next_bound",
    "searches",
    "deck_searches",
    "decks_scored",
    "tables",
    "nodes",
}


def walk(reference, candidate, path, differences):
    if len(differences) >= 8:
        return
    if type(reference) != type(candidate):
        differences.append(f"{path}: type {type(reference).__name__} vs {type(candidate).__name__}")
        return
    if isinstance(reference, dict):
        for k in sorted(set(reference) | set(candidate)):
            if k in SKIP or k.endswith("_ms"):
                continue
            if k not in reference or k not in candidate:
                differences.append(f"{path}.{k}: only in {'new' if k in candidate else 'ref'}")
                continue
            walk(reference[k], candidate[k], f"{path}.{k}", differences)
    elif isinstance(reference, list):
        if len(reference) != len(candidate):
            differences.append(f"{path}: length {len(reference)} vs {len(candidate)}")
            return
        for i, (reference_item, candidate_item) in enumerate(zip(reference, candidate)):
            walk(reference_item, candidate_item, f"{path}[{i}]", differences)
    elif reference != candidate:
        differences.append(f"{path}: {str(reference)[:60]} vs {str(candidate)[:60]}")


# the deck structure classes of the exact caps (`cap_class`: chain-varying targets; a multi-target group counts as 4+)
CLS = ["0-1", "2", "3", "4+"]


def near_cap(ratio, cap):
    # a scored deck's exact / product r is near its class's assumed cap c: past three quarters of the cap's headroom
    # over 1 (1.1875 of 1.25; 1.0375 of 1.05).
    # The same rule as val/cmp2.py.
    return ratio > 1 + 0.75 * (cap - 1)


def compare_job(name):
    reference_path, candidate_path = (
        os.path.join(PLAN_DIR, f"{ref}__{name}.out"),
        os.path.join(PLAN_DIR, f"{tag}__{name}.out"),
    )
    if not os.path.exists(reference_path):
        print(f"{name}: no reference ({ref})")
        return 1
    try:
        reference, candidate = json.load(open(reference_path)), json.load(open(candidate_path))
    except Exception as e:
        print(f"{name}: PARSE FAIL {e}")
        return 1
    differences = []
    walk(reference, candidate, "", differences)
    reference_times = (
        dict(l.split() for l in open(os.path.join(PLAN_DIR, f"{ref}_times.txt")))
        if os.path.exists(os.path.join(PLAN_DIR, f"{ref}_times.txt"))
        else {}
    )
    # the plan's own work (deck-search nodes, exact scorings): deterministic, so comparable at any machine load
    work_of = lambda j: (j.get("next") or {}).get("work") or {}
    reference_work, candidate_work = work_of(reference), work_of(candidate)
    work = " ".join(
        f"{k} {reference_work.get(k, '?'):,} -> {candidate_work.get(k, '?'):,}"
        if isinstance(reference_work.get(k), int) and isinstance(candidate_work.get(k), int)
        else f"{k} {reference_work.get(k, '?')} -> {candidate_work.get(k, '?')}"
        for k in ("nodes", "exact")
    )
    # Ratios near an assumed cap warn that pruning may discard a better deck.
    max_ratios, caps = candidate_work.get("class_max_ratio"), candidate_work.get("class_caps")
    if not isinstance(max_ratios, list):
        max_ratios, caps = [candidate_work.get("max_ratio")], [candidate_work.get("cap")]
    is_number = lambda candidate: isinstance(candidate, (int, float))
    near = [
        CapWarning(class_index=k, ratio=ratio, cap=cap)
        for k, (ratio, cap) in enumerate(zip(max_ratios, caps))
        if is_number(ratio) and is_number(cap) and near_cap(ratio, cap)
    ]
    print(
        f"{name}: {'identical' if not differences else 'DIFFERS'}  {reference_times.get(name, '?')} s -> {times[name]} s  {work}"
        + (
            f"  max ratio/cap by class {' '.join(f'{ratio:.3f}/{cap}' for ratio, cap in zip(max_ratios, caps))}"
            if all(is_number(ratio) for ratio in max_ratios)
            else ""
        )
    )
    for k, ratio, cap in near:
        print(
            f"    CAP: a scored deck of class {CLS[k] if len(max_ratios) == 4 else 'any'} reached exact/product {ratio:.4f}, near its assumed cap {cap}"
        )
    for o in differences:
        print("   ", o)
    return bool(differences) or bool(near)


class ProxyErrors(NamedTuple):
    """A job's proxy errors over its base builder rows: relative (proxy / exact - 1), and the saving's in careers."""

    rows: int
    other_deck: int
    efficiency_low: float
    efficiency_high: float
    saving_low: float  # saving + 1, relative (reported only)
    saving_high: float
    careers_low: float  # proxy saving - exact saving, careers


def base_rows(rows):
    """The proxy rows of base careers: unmarked rows' best by exact odds per trainee (module notes)"""
    if any("looped" in row for row in rows):
        return [row for row in rows if not row["looped"]]
    best = {}
    for row in rows:
        if row["trainee"] not in best or row["exact_val"] > best[row["trainee"]]["exact_val"]:
            best[row["trainee"]] = row
    return list(best.values())


def proxy_errors(output):
    rows = [row for row in base_rows(output["next"]["proxy_check"]["rows"]) if not row["cand"]]
    efficiency, saving, careers = [0.0], [0.0], [0.0]
    for row in rows:
        row_best = max(row["eff"], default=0.0)
        efficiency += [
            proxy / exact - 1
            for exact, proxy in zip(row["eff"], row["proxy_eff"])
            if row_best > 0 and exact >= 0.01 * row_best
        ]
        if row["score"] is not None and row["proxy_score"] is not None:
            careers.append(row["proxy_score"] - row["score"])
            if row["score"] + 1 > 0:
                saving.append((row["proxy_score"] + 1) / (row["score"] + 1) - 1)
    return ProxyErrors(
        rows=len(rows),
        other_deck=sum(not row["same_deck"] for row in rows),
        efficiency_low=min(efficiency),
        efficiency_high=max(efficiency),
        saving_low=min(saving),
        saving_high=max(saving),
        careers_low=min(careers),
    )


def proxy_gate(job_entry, job_dir):
    era, name = job_entry
    job = json.load(open(os.path.join(PLAN_DIR, name + ".json")))
    job["next"] = {**job.get("next", {}), "proxy_check": True}
    path = os.path.join(job_dir, name + ".json")
    json.dump(job, open(path, "w"))
    result = subprocess.run(
        [binary, "data.json", path],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", era),
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"{name} (proxy_check): solver exited {result.returncode}\n{result.stderr}")
    open(os.path.join(PLAN_DIR, f"{tag}_proxy__{name}.out"), "w").write(result.stdout)
    output = json.loads([line for line in result.stdout.splitlines() if line.strip()][-1])
    if "proxy_check" not in (output.get("next") or {}):
        return None
    return proxy_errors(output), plan_view(output["next"])


def plan_view(plan):
    """what a player sees of a plan: the pick's career, V, and the top OPT_SHOW rows by score"""
    career = lambda o: o.get("career") or [o["trainee"]["name"], o.get("parents")]
    chosen = next((o for o in plan["options"] if o.get("chosen")), None)
    scored = sorted((o for o in plan["options"] if o.get("score") is not None), key=lambda o: -o["score"])
    return (
        json.dumps(career(chosen)) if chosen else None,
        plan.get("v0"),
        sorted(json.dumps(career(o)) for o in scored[:OPT_SHOW]),
    )


class RouteStep(NamedTuple):
    """What V's route runs first and the row that prices it: exactly one of `row`, `not_applicable` (why the check does
    not apply, documented solver behaviour) and `failure` (why it cannot be checked) is set"""

    what: str
    row: dict | None = None
    not_applicable: str | None = None
    failure: str | None = None


# borrowed parents a runnable career may have, per `borrowed_parent` (`Pool::within_borrow_limit`)
BORROW_LIMIT = {"allow": 1, "later": 0, "never": 0}
# a row and a side are at the same odds within this relative tolerance (the row's P(all) and the side's odds come
# from different sums: gm16_nu8's differ in the last bit)
ODDS_MATCH = 1e-9


def trainee_ids(plan):
    """{trainee name: uma id} from the plan's rows and its choice"""
    rows = [*(plan.get("options") or []), *([plan["choice"]] if plan.get("choice") else [])]
    return {row["trainee"]["name"]: row["trainee"]["id"] for row in rows if "id" in row.get("trainee", {})}


def career_parents(career_id):
    """the two parents' legacy indices of a `kept::career_id` ("trainee:first,second:deck/pals/filler")"""
    first, second = career_id.split(":")[1].split(",")
    return int(first), int(second)


def borrowing_excludes(job, parents):
    """a career on these parents is no runnable option under the job's `borrowed_parent` rule"""
    legacies = job.get("legacies") or []
    borrowed = sum(bool(legacies[i].get("borrowed")) for i in parents if 0 <= i < len(legacies))
    return borrowed > BORROW_LIMIT.get(job.get("borrowed_parent", "allow"), 1)


def only_row(what, rows, missing):
    """the one row that matches, else a failure: none (`missing`) or more than one (ambiguous)"""
    if len(rows) == 1:
        return RouteStep(what, row=rows[0])
    if not rows:
        return RouteStep(what, failure=missing)
    return RouteStep(what, failure=f"{len(rows)} rows match it: ambiguous")


def route_first_step(plan, job):
    """V's route's first step (`RouteStep`): the first trio side without a real full veteran of hers (A, then B), her
    Cont's builder career where her step builds (`members[].builder`, matched by its career id) or else her own career
    (her rows on the side's parents at the side's odds, within ODDS_MATCH); with both sides real, the finish (C on the two real full
    veterans: her `finish` rows on parents of the two members' umas). `builder` null where she builds is documented when
    the job's `next.fit` gave her Cont (output.rs: no builder then), and a builder career missing from the rows is
    documented when the job's borrowing rule keeps one of its parents from running now; every other missing or
    ambiguous match fails"""
    options, ids = plan.get("options") or [], trainee_ids(plan)
    given_fit = {entry[0] for entry in (job.get("next") or {}).get("fit") or []}
    for member in plan.get("members") or []:
        if member.get("real"):
            continue
        name = member["trainee"]
        if member.get("building_careers", 0) > 0:
            builder = member.get("builder")
            if builder is None:
                if ids.get(name) in given_fit:
                    return RouteStep(f"{name}'s first builder", not_applicable="Cont given (the job's next.fit)")
                return RouteStep(f"{name}'s first builder", failure="`members[].builder` not reported")
            what = f"{name}'s first builder {builder}"
            rows = [o for o in options if o.get("career") == builder]
            if not rows and borrowing_excludes(job, career_parents(builder)):
                return RouteStep(
                    what,
                    not_applicable="her first builder has a borrowed parent the borrowing rule keeps from running now",
                )
            return only_row(what, rows, "no row has its career id")
        parents = sorted(member.get("parents") or [])
        rows = [
            o
            for o in options
            if o["trainee"]["name"] == name
            and sorted(o.get("parents") or []) == parents
            and math.isclose(o.get("p_all") or 0.0, member.get("p") or math.nan, rel_tol=ODDS_MATCH)
        ]
        return only_row(f"{name}'s own career {parents}", rows, "no row of hers on the side's parents at its odds")
    finisher = (plan.get("v0_plan") or {}).get("c")
    members = sorted(ids.get(m["trainee"], -1) for m in plan.get("members") or [])
    legacies = job.get("legacies") or []
    chara = lambda i: legacies[i]["chara"] if 0 <= i < len(legacies) else None
    rows = [
        o
        for o in options
        if o.get("role") == "finish"
        and o["trainee"]["name"] == finisher
        and sorted(chara(i) for i in o.get("parents") or []) == members
    ]
    return only_row("the finish on two real full veterans", rows, "no finish row of C's on the two members' veterans")


class FirstStep(NamedTuple):
    """V's own first step as its row prices it: score = saving - 1, saving = own_term + remainder"""

    score: float
    saving: float
    own_term: float  # the row's trainee's own full veteran: p_all x her side's careers (0 where she has no such side)
    remainder: float  # the rest of the saving: what the row credits to the side it builds, or to the trio
    routes_dropped: float


def first_step(plan, row):
    """the route's first step as `row` prices it (a FirstStep; its score may be NaN), None where the row has no score"""
    if row.get("score") is None:
        return None
    saving = row["score"] + 1
    side = next((m for m in plan.get("members") or [] if m["trainee"] == row["trainee"]["name"]), None)
    own_term = row["p_all"] * side["careers"] if side and not side.get("real") else 0.0
    return FirstStep(
        score=row["score"],
        saving=saving,
        own_term=own_term,
        remainder=saving - own_term,
        routes_dropped=plan.get("routes_dropped") or 0.0,
    )


def no_route(plan):
    """a plan with no V that shows no route either (not applicable): no trio (`v0_plan` null or no finite career in
    it) and no member with finite careers, both fields present"""
    if "v0_plan" not in plan or not isinstance(plan.get("members"), list):
        return False
    finite = lambda x: isinstance(x, (int, float)) and math.isfinite(x)
    trio = plan["v0_plan"]
    trio_finite = isinstance(trio, dict) and any(finite(x) for x in trio.get("careers") or [])
    return not trio_finite and not any(finite(member.get("careers")) for member in plan["members"])


def first_step_tripwire(name):
    """Print V's own first step and its split (module notes). True (fails) where it scores below FIRST_STEP_FLOOR, or
    where it cannot be checked: an unreadable or error output, a missing `v0`, a V that is not finite, a null V beside
    a route, a first step with no row, more than one matching row, or a null or NaN score. Not applicable (passes, says
    why): a null V with no route at all (`no_route`), a Cont the job gave, a builder the borrowing rule keeps out."""
    path = os.path.join(PLAN_DIR, f"{tag}__{name}.out")
    try:
        output = json.loads([line for line in open(path).read().splitlines() if line.strip()][-1])
        job = json.load(open(os.path.join(PLAN_DIR, name + ".json")))
    except (OSError, IndexError, ValueError) as error:
        print(f"{name}: route's first step: FAIL, no plan output or job to check ({error})")
        return True
    plan = output.get("next")
    if output.get("error") or not isinstance(plan, dict) or plan.get("error"):
        print(
            f"{name}: route's first step: FAIL, an error or no `next` ({output.get('error') or (plan or {}).get('error')})"
        )
        return True
    if "v0" not in plan:
        print(f"{name}: route's first step: FAIL, the plan has no `v0` field")
        return True
    v = plan["v0"]
    if v is None:
        if no_route(plan):
            print(
                f"{name}: route's first step: not applicable, the plan has no V and no route (no trio, no finite side)"
            )
            return False
        print(
            f"{name}: route's first step: FAIL, V is null beside a route (a trio or a side with finite careers), or a field is missing"
        )
        return True
    if not isinstance(v, (int, float)) or not math.isfinite(v):
        print(f"{name}: route's first step: FAIL, V is {v!r}")
        return True
    route = route_first_step(plan, job)
    if route.not_applicable:
        print(f"{name}: route's first step: {route.what}: not applicable, {route.not_applicable}")
        return False
    if route.failure:
        print(f"{name}: route's first step: {route.what}: FAIL, {route.failure}")
        return True
    step = first_step(plan, route.row)
    if step is None or not math.isfinite(step.score):
        score = "no score" if step is None else f"score {step.score}"
        print(
            f"{name}: route's first step: {route.what}, {score} (FAIL: V's route has a first step the plan does not price)"
        )
        return True
    low, high = step.score < FIRST_STEP_FLOOR, step.score > FIXED_POINT_GAP
    verdict = (
        f" BELOW {FIRST_STEP_FLOOR} (FAIL)"
        if low
        else " (V is not its own fixed point: the step saves more than V's route promises; not failed)"
        if high
        else ""
    )
    dropped = f"; routes_dropped {step.routes_dropped:,.0f}" + (" (FLAG)" if step.routes_dropped > 0 else "")
    print(
        f"{name}: route's first step: {route.what}, score {step.score:+,.3f}{verdict}\n"
        f"    saving {step.saving:,.4f} = own full-veteran term {step.own_term:,.4f} + remainder {step.remainder:+,.4f}, "
        f"against the 1 career V's route promises{dropped}"
    )
    return low


failures = sum(compare_job(name) for _era, name in jobs)
print(f"{len(jobs)} plan jobs, {failures} differ")
first_step_failures = sum(first_step_tripwire(name) for _era, name in jobs)
print(f"{len(jobs)} first steps, {first_step_failures} fail (below {FIRST_STEP_FLOOR} careers, or unpriced)")
with tempfile.TemporaryDirectory() as job_dir, ThreadPoolExecutor(parallelism) as ex:
    gates = list(ex.map(lambda job_entry: proxy_gate(job_entry, job_dir), jobs))
proxy_failures = 0
for (_era, name), gate in zip(jobs, gates):
    if gate is None:
        print(f"{name}: no proxy_check in the output")
        proxy_failures += 1
        continue
    errors, exact_view = gate
    run_output = os.path.join(PLAN_DIR, f"{tag}__{name}.out")
    lines = (
        [line for line in open(run_output).read().splitlines() if line.strip()] if os.path.exists(run_output) else []
    )
    moved = not lines or plan_view(json.loads(lines[-1])["next"]) != exact_view
    efficiency_low, saving_low = errors.efficiency_low < -PROXY_GATE, errors.careers_low < -SAVING_GATE
    proxy_failures += moved or efficiency_low or saving_low
    print(
        f"{name}: {'PROXIES MOVE THE PLAN; ' if moved else 'plan as all-exact; '}{errors.rows} rows "
        f"({errors.other_deck} on another deck): efficiency {errors.efficiency_low:+.2%} .. {errors.efficiency_high:+.2%}"
        f"{' LOW' if efficiency_low else ''}, saving {errors.careers_low:+.2f} careers at worst{' LOW' if saving_low else ''}"
        f" (saving + 1 {errors.saving_low:+.2%} .. {errors.saving_high:+.2%}, reported only)"
    )
print(
    f"{len(jobs)} proxy checks, {proxy_failures} fail (the plan moves, an efficiency over {PROXY_GATE:.0%} low, or a "
    f"saving over {SAVING_GATE} careers low)"
)
sys.exit(1 if failures or proxy_failures or first_step_failures or not jobs else 0)
