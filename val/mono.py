"""Monotonicity tripwire for the career planner: a veteran added to a pool never raises the true optimum (every plan
without her is still there), so a rise in the planner's V is the planner's error. Two checks per val/plan job:

- **last veteran:** plan the pool less its last veteran, then the full pool under that plan's fit (`next.fit`);
- **rolled veterans:** plan the full pool, then the pool with each of N veterans of its pick added, drawn as the page's
  Compare and Reroll draw them (career.js `draftOf` / `draftJob`: the landed set from the pick's `outcome_sets`, every
  set including the all-landed one, or each target on its own odds past 12 targets; each landed spark's stars from the
  job's `star_dist`; each scheduled G1 won with its chance; her pink drawn as pink.mjs `drawPink` draws it; her parents
  copied without their own parents; seeded per pool), each under the full pool's fit and refitted.

The fixed fit takes the continuation's refit out (`Cont` is refitted per pool); what remains is the plan's own
choice of route through the pool (which lineages its trios are bred from), so a fixed-fit rise is the plan's error. The
fixed-fit column is what the page's Compare shows (the plan of the pool with her under today's fit, career.js
`runComparison`); the refit column is what Add then shows (the pool with her, fitted to it). Refit rises are reported,
not failed on: a refit can move V either way.

A fixed-fit rise is re-planned with WIDE_LINEAGES fresh lineages per candidate (`next.fresh_lineages`, default 3),
the pool without her and with her under the same fit: if she no longer raises V there, the rise is the documented
truncation of the lineages a trio is bred from (docs/SOLVER.md: a veteran that opens more than `fresh_lineages`
better pairs can still raise V), reported as such; if she still raises it, it is a defect.

The gate fails on a fixed-fit rise that survives the wider lineages, and on any plan that should have a V but has none (a V lost when a veteran is
added is a rise to infinity), a malformed output, or a roll that could not be prepared.

Reported beside each roll, not failed on: the plan's quick answer for her (`next.roll` at her stars, from the kept
plan), flagged QUICK Y OFF where it differs from the fixed-fit saving by more than 1% of V; and on a refit rise, both
picks, each candidate's (lam, phi, pinf, p0) under both fits, and whether the builder pair her pinf comes from switched
(where the plan reports `pinf_builders`: {chara: [first, second]}).

usage: mono.py BIN [JOB ...] [-j N] [-n ROLLS]   (default: every job in val/plan/jobs.json; -j jobs at a time, default
  2; -n rolled veterans per pool, default 3, 0 for the last-veteran check alone)
  writes its job files and outputs to a temporary folder (printed at the end); exits 1 when a V rises by more than 1e-9 relative under a fixed fit, or a plan or roll
  fails (above)"""

import copy
import json
import math
import os
import random
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass

# pink.mjs: grades are aptitude codes 1..8 = G..S; each legacy pink fires at each of the two April inspirations
GRADE_A, GRADE_S, INSPIRATIONS = 7, 8, 2
DEFAULT_STAR_DIST = [0.5, 0.45, 0.05]  # the solver's default rank mix (src/skills.rs)
RISE = 1e-9


@dataclass(frozen=True)
class PoolComparison:
    """A pool's two plans under the same fit, or the reason they cannot be compared."""

    name: str
    without: dict | None = None
    with_veteran: dict | None = None
    unavailable: bool = False
    error: str | None = None


@dataclass(frozen=True)
class RolledComparison:
    """V with one rolled veteran of the pool's pick added, under the pool's fit and refitted."""

    name: str
    roll: int
    landed: int
    target_count: int
    fixed_fit_v: float | None
    refit_v: float | None
    truncated: bool = False  # a fixed-fit rise that WIDE_LINEAGES fresh lineages remove
    quick_y: float | None = None  # the pick's roll at her stars (`next.roll`, from the kept plan): her saving
    fixed: dict | None = None  # the fixed-fit plan (`plan_summary`)
    refit: dict | None = None  # the refit plan (`plan_summary`)


def plan_summary(plan):
    """what a refit rise is explained by: the pick (its career id), each candidate's continuation from `fit` as
    {chara: (lam, phi, pinf, p0)}, and the builder pair each pinf comes from where the plan reports it
    (`pinf_builders`: {chara: [first, second]})"""
    choice = plan.get("choice") or {}
    return {
        "pick": choice.get("career") or [choice.get("trainee", {}).get("name"), choice.get("parents")],
        "fit": {entry[0]: tuple(entry[1:5]) for entry in plan.get("fit") or [] if len(entry) >= 4},
        "pinf_builders": {int(chara): pair for chara, pair in (plan.get("pinf_builders") or {}).items()},
    }


def her_stars(her, job):
    """her stars per job target (0 where she has no spark of it): a roll as the page's Reroll sends it"""
    return [
        max([stars for group, stars in her["sparks"] if group == target["group"]] or [0]) for target in job["targets"]
    ]


def quick_saves(era, job, name, veterans):
    """the plan's quick answer for each rolled veteran (`next.roll` at her stars, answered from the plan this process
    keeps: the plan job first, then one roll job per veteran), as each roll's `y`"""
    jobs = [dict(job, next={**job.get("next", {})})] + [
        dict(job, next={**job.get("next", {}), "roll": her_stars(her, job)}) for her in veterans
    ]
    paths = []
    for i, roll_job in enumerate(jobs):
        paths.append(os.path.join(temporary_dir, f"{name}_quick{i}.json"))
        json.dump(roll_job, open(paths[-1], "w"))
    result = subprocess.run(
        [binary, "data.json", *paths],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", era),
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"{name} (quick rolls): solver exited {result.returncode}\n{result.stderr}")
    open(os.path.join(temporary_dir, f"{name}_quick.out"), "w").write(result.stdout)
    outputs = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    return [((output.get("next") or {}).get("roll") or {}).get("y") for output in outputs[1:]]


def route_steps(plan):
    if not plan.get("v0_plan"):
        return "?"
    return " / ".join(
        f"{plan['v0_plan'][role]} {careers:,.1f}" for role, careers in zip("abc", plan["v0_plan"]["careers"])
    )


VALIDATION_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(VALIDATION_DIR)
PLAN_DIR = os.path.join(VALIDATION_DIR, "plan")


def option(args, flag, default):
    if flag not in args:
        return default
    i = args.index(flag)
    value = int(args[i + 1])
    del args[i : i + 2]
    return value


args = sys.argv[1:]
parallelism = option(args, "-j", 2)
roll_count = option(args, "-n", 3)
binary = os.path.abspath(args[0])
want = set(args[1:])
jobs = [(era, name) for era, name in json.load(open(os.path.join(PLAN_DIR, "jobs.json"))) if not want or name in want]
temporary_dir = tempfile.mkdtemp(prefix="mono_")


def solve(era, job, name):
    path = os.path.join(temporary_dir, name + ".json")
    json.dump(job, open(path, "w"))
    result = subprocess.run(
        [binary, "data.json", path],
        cwd=os.path.join(REPO, "pkg", "loopopt", "data", era),
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"{name}: solver exited {result.returncode}\n{result.stderr}")
    # (kept beside the job: val/sim/lam_regress.py reads the plans' fits)
    open(os.path.join(temporary_dir, name + ".out"), "w").write(result.stdout)
    return json.loads([line for line in result.stdout.splitlines() if line.strip()][-1])


def plan(era, job, name):
    """the output's `next` ({} when there is none)"""
    return solve(era, job, name).get("next") or {}


def finite(value):
    return isinstance(value, (int, float)) and math.isfinite(value)


def last_veteran(job_entry):
    era, name = job_entry
    job = json.load(open(os.path.join(PLAN_DIR, name + ".json")))
    if len(job.get("legacies", [])) < 3:
        return PoolComparison(name=name)
    less = dict(job, legacies=job["legacies"][:-1], next={**job.get("next", {})})
    without = plan(era=era, job=less, name=name + "_less")
    if not finite(without.get("v0")) or not without.get("fit"):
        return PoolComparison(name=name, unavailable=True, error=without.get("error"))
    with_veteran = plan(
        era=era, job=dict(job, next={**job.get("next", {}), "fit": without["fit"]}), name=name + "_with"
    )
    return PoolComparison(name=name, without=without, with_veteran=with_veteran)


# ---- a rolled veteran, as career.js draftOf / draftJob build her ----
def scheduled_races(report, race_names):
    """career.js scheduledRaces: per race index she runs, her chance of winning it at least once"""
    races = {}
    for row in report.get("schedule") or []:
        if row.get("run") is False:
            continue
        race = row.get("name") or row.get("race")
        if race not in race_names:
            continue
        index = race_names.index(race)
        races[index] = 1 - (1 - races.get(index, 0.0)) * (1 - row["p"])
    return races


def drawn_pink(aptitudes, rng):
    """pink.mjs drawPink: {group: stars}, its draws in the same order (fires per inspiration, aptitude, stars)"""
    if not aptitudes:
        return {}
    grades = list(aptitudes["start"])
    for _ in range(INSPIRATIONS):
        for fire in aptitudes["fire"]:
            if rng.random() < fire["p"]:
                grades[fire["apt"]] = min(GRADE_S, grades[fire["apt"]] + 1)
    at_least_a = [k for k, grade in enumerate(grades) if grade >= GRADE_A]
    if not at_least_a:
        return {}
    k = at_least_a[int(rng.random() * len(at_least_a))]
    u = rng.random()
    return {aptitudes["pink_gen"][k]["group"]: 1 if u < 0.2 else 2 if u < 0.9 else 3}


def drawn_veteran(report, job, race_names, rng):
    """(landed count, the veteran's job entry): draftOf's rolled outcome, then draftJob's shape"""
    star_dist = job.get("star_dist", DEFAULT_STAR_DIST)
    star = lambda: rng.choices([1, 2, 3], star_dist)[0]
    targets = job["targets"]
    if isinstance(report.get("outcome_sets"), list):
        u, cumulative, mask = rng.random(), 0.0, len(report["outcome_sets"]) - 1
        for landed_set, chance in enumerate(report["outcome_sets"]):
            cumulative += chance
            if u < cumulative:
                mask = landed_set
                break
        landed = [k for k in range(len(targets)) if mask >> k & 1]
    else:
        landed = [k for k, target in enumerate(report["targets"]) if rng.random() < target["p"]]
    sparks = [[targets[k]["group"], star()] for k in landed]
    wins = sorted(index for index, chance in scheduled_races(report, race_names).items() if rng.random() < chance)
    pinks = drawn_pink(report.get("aptitudes"), rng)
    node = lambda legacy: {
        "chara": legacy.get("chara", 0),
        "sparks": legacy.get("sparks", []),
        "pinks": legacy.get("pinks", []),
        "wins": list(legacy.get("wins", [])),
    }
    her = {
        "chara": report["trainee"]["id"],
        "sparks": sparks,
        "pinks": [[group, stars] for group, stars in pinks.items()],
        "wins": wins,
        "borrowed": False,
        "parents": [node(job["legacies"][parent]) for parent in report["parents"]],
    }
    return len(landed), her


def rolled_veterans(job_entry):
    """The pool's plan, then each rolled veteran of its pick under that plan's fit and refitted; an output that is not
    what the solver reports is a failed preparation"""
    try:
        return rolled_veterans_of(job_entry)
    except (KeyError, IndexError, TypeError, ValueError) as error:
        return {}, [], f"malformed output ({type(error).__name__}: {error})"


def rolled_veterans_of(job_entry):
    era, name = job_entry
    job = json.load(open(os.path.join(PLAN_DIR, name + ".json")))
    base = plan(era=era, job=dict(job, next={**job.get("next", {})}), name=name + "_base")
    pick = base.get("outcomes")
    if not finite(base.get("v0")) or not base.get("fit") or not pick:
        return base, [], base.get("error") or "no plan"
    # her career's report (schedule, landed sets, aptitudes): the pick's trainee on its parents, as the page shows it
    search = copy.deepcopy(job)
    search.pop("next", None)
    search.update(trainees=[pick["trainee"]], fix=pick["parents"], top=1, no_reports=False)
    report = solve(era, search, name + "_pick")["results"][0]
    if sorted(report["parents"]) != sorted(pick["parents"]):
        return base, [], f"the pick's search gave parents {report['parents']}, not {pick['parents']}"
    race_names = json.load(open(os.path.join(REPO, "pkg", "loopopt", "data", era, "data.json")))["race_names"]
    rng = random.Random(f"mono-{name}")
    draws = [drawn_veteran(report, job, race_names, rng) for _ in range(roll_count)]
    quick = quick_saves(era, job, name, [her for _landed, her in draws])
    comparisons = []
    for roll, (landed, her) in enumerate(draws):
        with_her = dict(job, legacies=job["legacies"] + [her])
        fixed = plan(era, dict(with_her, next={**job.get("next", {}), "fit": base["fit"]}), f"{name}_r{roll}_fixed")
        refit = plan(era, dict(with_her, next={**job.get("next", {})}), f"{name}_r{roll}_refit")
        truncated = rises(fixed.get("v0"), base["v0"]) and truncation_only(
            era, f"{name}_r{roll}", job, base["fit"], her
        )
        comparisons.append(
            RolledComparison(
                name=name,
                roll=roll,
                landed=landed,
                target_count=len(job["targets"]),
                fixed_fit_v=fixed.get("v0"),
                refit_v=refit.get("v0"),
                truncated=truncated,
                quick_y=quick[roll] if roll < len(quick) else None,
                fixed=plan_summary(fixed),
                refit=plan_summary(refit),
            )
        )
    return base, comparisons, None


WIDE_LINEAGES = 8


def truncation_only(era, name, job, fit, her):
    """whether a fixed-fit rise is gone with WIDE_LINEAGES fresh lineages (both plans under the same fit)"""
    wide = {**job.get("next", {}), "fit": fit, "fresh_lineages": WIDE_LINEAGES}
    without = plan(era, dict(job, next=wide), f"{name}_wide_without")
    with_her = plan(era, dict(job, legacies=job["legacies"] + [her], next=wide), f"{name}_wide_with")
    return finite(without.get("v0")) and finite(with_her.get("v0")) and not rises(with_her["v0"], without["v0"])


def rises(value_with, value_without):
    """a rise past RISE, a lost V (none, or not finite) counting as one"""
    return not finite(value_with) or value_with > value_without * (1 + RISE)


def change(value_with, value_without):
    return "none" if not finite(value_with) else f"{value_with - value_without:+10,.2f}"


QUICK_GAP = 0.01  # of V: the quick y against the fixed-fit saving


def quick_note(c, value_without):
    """the quick y beside the fixed-fit saving (-dV), flagged when they differ by more than QUICK_GAP of V"""
    if not finite(c.quick_y) or not finite(c.fixed_fit_v):
        return "  quick y none"
    saving = value_without - c.fixed_fit_v
    flag = " QUICK Y OFF" if abs(c.quick_y - saving) > QUICK_GAP * value_without else ""
    return f"  quick y {c.quick_y:+,.2f} (fixed-fit saving {saving:+,.2f}){flag}"


def refit_detail(c):
    """a refit rise's lines: both picks, each candidate's (lam, phi, pinf, p0) under both fits, and whether her pinf's
    builder pair switched"""
    lines = [f"        picks: fixed fit {c.fixed['pick']}, refit {c.refit['pick']}"]
    for chara in sorted(set(c.fixed["fit"]) | set(c.refit["fit"])):
        fmt = lambda entry: "-" if entry is None else "(" + ", ".join(f"{value:.4g}" for value in entry) + ")"
        before, after = c.fixed["pinf_builders"].get(chara), c.refit["pinf_builders"].get(chara)
        pair = (
            "pinf pair not reported"
            if before is None or after is None
            else f"pinf pair {before} -> {after}{' SWITCHED' if sorted(before) != sorted(after) else ''}"
        )
        lines.append(
            f"        {chara}: fixed {fmt(c.fixed['fit'].get(chara))}  refit {fmt(c.refit['fit'].get(chara))}  {pair}"
        )
    return "\n".join(lines)


fixed_rises = refit_rises = failures = truncation_rises = 0
with ThreadPoolExecutor(parallelism) as ex:
    last_comparisons = list(ex.map(last_veteran, jobs))
    rolled = list(ex.map(rolled_veterans, jobs)) if roll_count else []
print("== the last veteran added, under the plan's fit without her")
for comparison in last_comparisons:
    name = comparison.name
    if comparison.without is None and not comparison.unavailable:
        print(f"{name}: under 3 veterans, skipped")
        continue
    if comparison.unavailable:
        # (no V without her: nothing she can raise)
        print(f"{name}: no plan without her {comparison.error or ''}")
        continue
    without, with_veteran = comparison.without, comparison.with_veteran
    value_without, value_with = without["v0"], with_veteran.get("v0")
    rise = rises(value_with, value_without)
    fixed_rises += rise
    print(
        f"{name}: V {value_without:,.2f} -> {change(value_with, value_without).strip()}  {'RISES' if rise else ''}\n"
        f"    without: {route_steps(without)}\n    with:    {route_steps(with_veteran)}"
    )
if roll_count:
    print(f"== {roll_count} rolled veterans of each pool's pick: dV = V with her - V without (+: V rises)")
    for (_era, name), (base, comparisons, error) in zip(jobs, rolled):
        if error:
            print(f"{name}: FAILED, no rolls ({error})")
            failures += 1
            continue
        value_without = base["v0"]
        print(
            f"{name}: V {value_without:,.2f}, pick {base['choice']['trainee']['name']} on {base['outcomes']['parents']}"
        )
        for c in comparisons:
            fixed_rise, refit_rise = rises(c.fixed_fit_v, value_without), rises(c.refit_v, value_without)
            refit_lost = not finite(c.refit_v)
            truncation_rises += fixed_rise and c.truncated
            fixed_rise = fixed_rise and not c.truncated
            fixed_rises += fixed_rise
            refit_rises += refit_rise and not refit_lost
            failures += refit_lost
            print(
                f"    roll {c.roll}: landed {c.landed}/{c.target_count}  fixed-fit dV {change(c.fixed_fit_v, value_without)}{' RISES' if fixed_rise else ' (rise: lineage truncation, gone at ' + str(WIDE_LINEAGES) + ')' if c.truncated else ''}"
                f"  refit dV {change(c.refit_v, value_without)}{' (no V: FAILED)' if refit_lost else ' (refit rise)' if refit_rise else ''}"
                + quick_note(c, value_without)
            )
            if refit_rise and not refit_lost:
                print(refit_detail(c))
rolls = sum(len(comparisons) for _base, comparisons, _error in rolled)
print(
    f"{len(last_comparisons)} pools, {rolls} rolled veterans: {fixed_rises} rise under a fixed fit, "
    f"{refit_rises} rise refitted, {failures} failed plans or rolls; {truncation_rises} fixed-fit rises from lineage "
    f"truncation (gone at {WIDE_LINEAGES} lineages)"
)
print(f"jobs and outputs: {temporary_dir}")
sys.exit(1 if fixed_rises or failures else 0)
