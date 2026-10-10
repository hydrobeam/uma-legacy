"""Write the simulator jobs behind val/sim/README.md into a directory.

    uv run val/sim/make_jobs.py OUT_DIR
    (cd pkg/loopopt && target/release/loopopt data.json OUT_DIR/NAME.json > OUT_DIR/NAME.out)   # per job (one line of JSON)
    uv run val/sim/report.py runs OUT_DIR/*.out ; uv run val/sim/report.py audit OUT_DIR/*.out ; ... e1 OUT_DIR/e1_*.out

Each job is a pool file (career mode) plus a `simulate` block (docs/SOLVER.md "Policy simulator"). Paths inside the
pool files are relative to pkg/loopopt, so run the solver from there. Jobs split into run ranges run in parallel.
"""
import json, os, sys

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
U5 = f'{REPO}/pkg/loopopt/jobs/career_U5.json'
O4 = f'{REPO}/val/policy/pools/pool4.json'
O14 = f'{REPO}/val/policy/pools/pool14.json'
LU = [1019, 1021, 1034]   # the loop planner's best for career_U5's settings (val/refpu__pkg_loopopt__pal_U.out)
LO = [1014, 1019, 1078]   # the loop planner's best for pool4/pool14's roster and cards
G = dict(margin=-1)       # exact-backed greedy
L = dict(margin=0.02)


def main(out):
    os.makedirs(out, exist_ok=True)
    jobs = []

    def mk(src, name, **sim):
        j = json.load(open(src)); j['simulate'] = sim
        json.dump(j, open(f'{out}/{name}.json', 'w')); jobs.append(name)

    # E1 / E2: greedy with exact checks; margins
    for a in (0, 8):
        mk(U5, f'e1_u5_{a}', policy='greedy', runs=8, first=a, margin=0, check=100, check_lineages=30)
    mk(O14, 'e1_o14', policy='greedy', runs=2, margin=0, check=250, check_lineages=30)
    mk(O4, 'e1_o4', policy='greedy', runs=1, margin=0, check=300, check_lineages=30)
    for m in (0, 0.01, 0.05):
        mk(U5, f'u5_g_m{m}', policy='greedy', runs=48, margin=m)
    # E3 (audit on)
    A = dict(audit=True)
    mk(U5, 'u5_g', policy='greedy', runs=48, **G, **A)
    mk(U5, 'u5_l', policy='loop', runs=48, loop_members=LU, **L, **A)
    for a in (0, 8):
        mk(U5, f'u5_rg1_{a}', policy='rollout_greedy', runs=8, first=a, samples=8, **G, **A)
        mk(U5, f'u5_rl1_{a}', policy='rollout_loop', runs=8, first=a, samples=8, loop_members=LU, **L, **A)
    for a in (0, 2):
        mk(U5, f'u5_rgR_{a}', policy='rollout_greedy', runs=2, first=a, samples=4, redecide=100, cont_eps=0.05, **G, **A)
    mk(O14, 'o14_g', policy='greedy', runs=16, **G, **A)
    mk(O14, 'o14_l', policy='loop', runs=16, loop_members=LO, **L, **A)
    mk(O14, 'o14_rg1', policy='rollout_greedy', runs=8, samples=4, **G, **A)
    mk(O14, 'o14_rl1', policy='rollout_loop', runs=8, samples=4, loop_members=LO, **L, **A)
    for a in (0, 8):
        mk(O4, f'o4_g_{a}', policy='greedy', runs=8, first=a, **G, **A)
    mk(O4, 'o4_l', policy='loop', runs=16, loop_members=LO, **L, **A)
    for a in range(2):
        mk(O4, f'o4_rg1_{a}', policy='rollout_greedy', runs=1, first=a, samples=4, **G, **A)
    for a in range(8):
        mk(O4, f'o4_rl1_{a}', policy='rollout_loop', runs=1, first=a, samples=4, loop_members=LO, **L, **A)
    # the rollout planner (the page's pick before `next` replaced it; the solver in process per decision): slow, one run per job
    for a in range(4):
        mk(U5, f'u5_pl_{a}', policy='planner', runs=1, first=a, progress=100)
    # whole trajectories under the exact model
    mk(U5, 'u5_gex', policy='greedy_exact', runs=2, first=4)
    mk(U5, 'u5_lx', policy='loop', runs=2, first=1, loop_members=LU, margin=-1)
    mk(O14, 'o14_lx', policy='loop', runs=2, loop_members=LO, margin=-1)
    mk(O4, 'o4_lx', policy='loop', runs=2, loop_members=LO, margin=-1)
    for src, p in ((O14, 'o14'), (O4, 'o4')):
        mk(src, f'{p}_gex', policy='greedy_exact', runs=1, max_careers=1000)
        mk(src, f'{p}_gh', policy='greedy', runs=1, max_careers=1000, margin=0)
    open(f'{out}/queue.txt', 'w').write('\n'.join(jobs) + '\n')
    print(f'{len(jobs)} jobs in {out} (queue.txt)')


if __name__ == '__main__':
    main(sys.argv[1])
