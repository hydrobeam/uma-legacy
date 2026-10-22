"""Write the simulator jobs behind val/sim/README.md into a directory.

    uv run val/sim/make_jobs.py OUT_DIR
    (cd pkg/loopopt/data/global && ../../target/release/loopopt data.json OUT_DIR/NAME.json > OUT_DIR/NAME.out)   # per job
    uv run val/sim/report.py runs OUT_DIR/*.out ; uv run val/sim/report.py audit OUT_DIR/*.out ; ... e1 OUT_DIR/e1_*.out

The greedy and loop policies on three pools (career_U5, pool4, pool14), with the surrogate's checks and audits, and
whole trajectories under the exact model; the trace job that refits Cont's shape is in val/sim/README.md.

Each job is a pool file (career mode) plus a `simulate` block (docs/SOLVER.md "Policy simulator"). Paths inside the
pool files are relative to the era's data folder (pkg/loopopt/data/global), so run the solver from there. Jobs split into run ranges run in parallel.
"""

import json
import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CAREER_POOL = f"{REPO}/val/jobs/career_U5.json"
FOUR_VETERAN_POOL = f"{REPO}/val/sim/pools/pool4.json"
FOURTEEN_VETERAN_POOL = f"{REPO}/val/sim/pools/pool14.json"
CAREER_LOOP = [1019, 1021, 1034]  # the loop planner's best for career_U5's settings (val/refpu__global__pal_U.out)
OTHER_LOOP = [1014, 1019, 1078]  # the loop planner's best for pool4/pool14's roster and cards
GREEDY_OPTIONS = {"margin": -1}  # exact-backed greedy
LOOP_OPTIONS = {"margin": 0.02}


def main(out):
    os.makedirs(out, exist_ok=True)
    jobs = []

    def write_job(source, name, **sim):
        job = json.load(open(source))
        job["simulate"] = sim
        json.dump(job, open(f"{out}/{name}.json", "w"))
        jobs.append(name)

    # E1 / E2: greedy with exact checks; margins
    for first_run in (0, 8):
        write_job(
            CAREER_POOL,
            f"e1_u5_{first_run}",
            policy="greedy",
            runs=8,
            first=first_run,
            margin=0,
            check=100,
            check_lineages=30,
        )
    write_job(FOURTEEN_VETERAN_POOL, "e1_o14", policy="greedy", runs=2, margin=0, check=250, check_lineages=30)
    write_job(FOUR_VETERAN_POOL, "e1_o4", policy="greedy", runs=1, margin=0, check=300, check_lineages=30)
    for margin in (0, 0.01, 0.05):
        write_job(CAREER_POOL, f"u5_g_m{margin}", policy="greedy", runs=48, margin=margin)
    # greedy and loop, audits on
    AUDIT_OPTIONS = {"audit": True}
    write_job(CAREER_POOL, "u5_g", policy="greedy", runs=48, **GREEDY_OPTIONS, **AUDIT_OPTIONS)
    write_job(CAREER_POOL, "u5_l", policy="loop", runs=48, loop_members=CAREER_LOOP, **LOOP_OPTIONS, **AUDIT_OPTIONS)
    write_job(FOURTEEN_VETERAN_POOL, "o14_g", policy="greedy", runs=16, **GREEDY_OPTIONS, **AUDIT_OPTIONS)
    write_job(
        FOURTEEN_VETERAN_POOL, "o14_l", policy="loop", runs=16, loop_members=OTHER_LOOP, **LOOP_OPTIONS, **AUDIT_OPTIONS
    )
    for first_run in (0, 8):
        write_job(
            FOUR_VETERAN_POOL,
            f"o4_g_{first_run}",
            policy="greedy",
            runs=8,
            first=first_run,
            **GREEDY_OPTIONS,
            **AUDIT_OPTIONS,
        )
    write_job(
        FOUR_VETERAN_POOL, "o4_l", policy="loop", runs=16, loop_members=OTHER_LOOP, **LOOP_OPTIONS, **AUDIT_OPTIONS
    )
    # whole trajectories under the exact model
    write_job(CAREER_POOL, "u5_gex", policy="greedy_exact", runs=2, first=4)
    write_job(CAREER_POOL, "u5_lx", policy="loop", runs=2, first=1, loop_members=CAREER_LOOP, margin=-1)
    write_job(FOURTEEN_VETERAN_POOL, "o14_lx", policy="loop", runs=2, loop_members=OTHER_LOOP, margin=-1)
    write_job(FOUR_VETERAN_POOL, "o4_lx", policy="loop", runs=2, loop_members=OTHER_LOOP, margin=-1)
    for source, pool_name in ((FOURTEEN_VETERAN_POOL, "o14"), (FOUR_VETERAN_POOL, "o4")):
        write_job(source, f"{pool_name}_gex", policy="greedy_exact", runs=1, max_careers=1000)
        write_job(source, f"{pool_name}_gh", policy="greedy", runs=1, max_careers=1000, margin=0)
    open(f"{out}/queue.txt", "w").write("\n".join(jobs) + "\n")
    print(f"{len(jobs)} jobs in {out} (queue.txt)")


if __name__ == "__main__":
    main(sys.argv[1])
