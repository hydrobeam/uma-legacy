# Policy simulator (`val/policy_sim.mjs`)

What it measures: for a decision policy (which career to run next) on a fixed starting pool of veterans, the
expected number of careers until the first career whose veteran lands every target, under the solver's model.
That is the only number the career planner should minimise; this tool is the measuring stick for comparing
planners before one is changed.

## One run

Before career t the policy picks a career (trainee, two parents, deck) from the current pool; that is one native
solver call (`mode: career`). With chance p_t = the career's P(all) it lands everything and the process ends. The
ending is counted analytically, so the simulated path continues only through failures:

    E[N] = sum_{t >= 0} P(N > t) = sum_t S_t,   S_0 = 1,   S_{t+1} = S_t (1 - p_t)        (derived)

and each run's E is an unbiased estimate of the policy's expected careers (the path's draws are conditioned on
failure, the hit is weighted in through S). A failed career leaves a veteran, drawn as follows:

- **Landed set**: from the career's `outcome_sets` (the exact chance of every set of targets, guaranteed golds
  included) with the all-landed set removed and renormalised. Exact for the model.
- **Stars**: each landed spark independently from the job's `star_dist` (default 50/45/5). Assumed, as in the
  solver's look-ahead.
- **G1 wins**: per race, at least one win among the schedule rows she runs (`run !== false`, each won with its `p`,
  races independent), mapped to race indices through `data.json` `race_names`. The solver's rollout uses the same
  chances but as expected counts; here they are drawn.
- **Pinks**: none (they do not enter the skill odds). **Parents**: the career's two parents, recorded as she would be.

The new veteran joins the pool and the policy is asked again.

## Stopping and the tail

A run ends at the first of:

- **settled**: K decisions in a row (`--settle`, default 2) pick the pool's best career by P(all) (`results[0]`),
  with that best career unchanged. A pick that is not the best (the planner breeding a better veteran) never
  settles: the policy repeats it only while the pool stays as it is, so costing it forever would charge the
  planner for a plan it does not follow. K = 0: never settle.
- **H decisions** (`--steps`), counted in careers.
- **survival below eps** (`--eps`, default 0 = off).

The tail is S_t / p_t: the last career repeated forever. **Assumed**, and pessimistic for both policies: under the
model the pool keeps improving after the run stops (every failed career can still leave a better veteran), so the
real policy does better than its tail. For greedy, re-planning only ever moves to a higher P(all), so the tail is an
upper bound on its remaining careers (derived); for the planner it is approximately one. See *What the numbers
mean* below for how large this is.

Records keep the whole path, so `report DIR --steps H` recomputes every number at a smaller H (draws are keyed by
career index, so a path cut at H is the path a run with that H takes), and `--settle`/`--eps` likewise where the
recording reaches. Raising H on an existing output directory reruns only the runs that ended at the old H.

## Pruning dominated veterans (`--prune 1`, default)

A new veteran is not added when one already in the pool dominates her: same character, same two parents (by
their recorded sparks, wins and characters), owned, at least her stars on every target, every G1 she won. The
pool is then unchanged and the solver deterministic, so the policy's next pick is known without a call. For
P(all) this is exact (derived): stars and shared wins only raise inheritance chances, and two parents of one
character cannot pair (the solver's legality rule), so every career using her has a counterpart at least as
good. Not covered: the planner's top-K shortlist (her careers could have filled a place in it) and her value as a
later grandparent through wins her dominator lacks. Without pruning a repeated career's pool grows by one veteran
per career.

## Common random numbers and pairing

Every uniform is a hash of (seed, run index, career index, purpose, item): the landed set (inverse CDF over the
failed sets ordered fewest-landed first, so a high draw means many landed for any career), each target's stars,
each race's win. Two policies on the same run index face the same luck, so the per-run difference planner − greedy
has a far smaller spread than either policy's E; the report gives its mean and 95% CI (Student t) and how many runs
went each way.

## Running it

From the repository root (the solver runs in `pkg/loopopt`, so a pool's relative file names resolve there):

    node val/policy_sim.mjs --pool pkg/loopopt/jobs/career_U5.json --policy planner,greedy,repeat \
         --runs 48 --steps 20 --settle 2 --seed 1 -j 4 --out /tmp/u4
    node val/policy_sim.mjs report /tmp/u4 --steps 10      # the same runs cut at H = 10

`-j` caps parallel runs (default 4; each run is sequential solver calls). `--bin` picks the solver binary (default
`pkg/loopopt/target/release/loopopt`; copy it first if the solver is being rebuilt). `--first R` starts at run
index R, `--k` sets the planner's rollout width (default the pool's `rollout`, else 6), `--progress N` logs every N
decisions. A finished run is appended to `DIR/<policy>.jsonl` at once, so an interrupted batch resumes where it
stopped. Output: `<policy>.jsonl` (per run: E, careers simulated, solver calls and seconds, pruned veterans, the
final career and p, and `segs`: per decision the pick, its p, the pool's best and its p, the pool size, how many
careers ran under it, and the veteran that ended it: landed mask, stars per target, wins), `summary.json`, and the
printed table.

A policy is one function in `val/policy/policies.mjs`: `(job, {solve, opt}) => {pick, best, diag}`. `planner` is
the page's rollout pick (`rollout.choice`), `greedy` the best P(all) now (`results[0]` of a job with no rollout,
`top: 1`, `per_trainee: false`), `repeat` today's best forever (E = 1/p* exactly, computed once).

## Pools (`pools/`)

| File | What | Today's best (1/p*) |
|---|---|---|
| `owner4.json` | the owner's 4-veteran pool (Unity Cup, owned roster and cards, borrowing on), `rollout: 6` | 7,589 |
| `owner14.json` | the owner's 14-veteran pool, same settings | 817 |
| u4 (no copy here: `pkg/loopopt/jobs/career_U5.json`) | the `career_U4` test pool of 12 veterans with `rollout: 6` | 587 |

## Baselines (seed 1, settle K = 2 on the best career)

The raw runs behind these tables (`val/policy/baseline/`) are not tracked; `node val/policy_sim.mjs` regenerates them.

### Settle-on-best estimates: not a policy comparison

Read these as "the planner's breeding phase, then its settled career forever" against "today's best forever".
They are **unfair to greedy**: greedy always picks the pool's best career, so it settles as soon as one failed
career leaves the best unchanged (one or two careers in every run) and is then costed as repeating that career
forever, while under the model every later career could still leave a better veteran. Greedy here is therefore
almost exactly `repeat`. The planner is frozen too, but only after up to H decisions of breeding. The long-horizon
table below is the comparison.

| Pool | Runs | H | Planner, mean (95% CI) | Greedy | Repeat (exact) | Planner − greedy, paired (95% CI) | Planner lower / higher | Planner unsettled at H | Wall (-j 4) |
|---|---|---|---|---|---|---|---|---|---|
| u4 | 48 | 60 | 432 (400 to 465) | 576 (568 to 584) | 587 | −143 (−175 to −112) | 36 / 12 | 1 | 31 min (H 20, then 60) |
| owner4 | 48 | 20 | 5,446 (4,907 to 5,985) | 7,589 (every run) | 7,589 | −2,143 (−2,683 to −1,604) | 38 / 10 | 12 | 47 min |
| owner14 | 24 | 20 | 817 (every run) | 817 | 817 | 0 | 0 / 0 | 0 | 4 min |

Sensitivity to H (planner mean ± 95% half-width, runs unsettled at that H; the same paths cut short):

| Pool | H 4 | H 6 | H 10 | H 20 | H 50 | H 60 |
|---|---|---|---|---|---|---|
| u4 | 598 ±19 (45) | 564 ±19 (37) | 539 ±22 (31) | 485 ±30 (21) | 436 ±31 (5) | 432 ±33 (1) |
| owner4 | 6,812 ±465 (48) | 5,944 ±493 (24) | 5,814 ±517 (17) | 5,446 ±539 (12) | | |

On owner4 a quarter of the planner's runs are still breeding at H = 20 and the mean is still falling, so 5,446 is
an upper estimate of this (already pessimistic) quantity; raising H costs about 25 s per planner call on a pool
of 20+ veterans. On owner14 the planner's pick is today's best career (Agnes Digital [Full-Color Fangirling]
from legacies 8 and 10, 1 in 817) and the first failure leaves it best in every run, so all three coincide.

### Long horizon (u4, greedy only; stopped early)

Never settle; follow the policy until survival is below 0.001 (tail S/p about 0.2 careers). Planned as greedy
N = 16 and planner N = 8 (`u4_long/`); stopped by the owner after 4 greedy runs (indices 0 to 3), because a faster
surrogate simulator reproduces exact-model greedy runs. No long-horizon planner run finished, so there is no
long-horizon planner − greedy difference here.

| Run | E (careers) | Careers simulated | Solver calls | Veterans pruned | Final pool | Final best career (1/p; outfit not recorded) | Solver time |
|---|---|---|---|---|---|---|---|
| 0 | 273.5 | 1,677 | 722 | 956 | 733 | Agnes Digital, legacies 664 + 665 (214) | 31 min |
| 1 | 242.4 | 1,419 | 613 | 807 | 624 | Agnes Digital, 491 + 510 (186) | 21 min |
| 2 | 273.9 | 1,736 | 622 | 1,115 | 633 | Agnes Digital, 71 + 209 (247) | 25 min |
| 3 | 254.2 | 1,291 | 534 | 758 | 545 | Agnes Digital, 179 + 214 (172) | 13 min |

Mean 261, SD 15.4, 95% CI 237 to 285 (n = 4). The same four runs cut at H careers (greedy then frozen): H 10: 570,
H 20: 490, H 50: 431, H 100: 281, H 200: 269, H 500: 261. Wall time 31 min for the four (in parallel with other
jobs, so per-call times are inflated).

## What the numbers mean

Under the model, the pool keeps improving long after any reasonable H. Greedy on u4 starts at 1 in 587. Settled
after one career it scores 576; followed until the hit is all but certain it scores 261 (n = 4). Its pool's best
career rose to 1 in 172 to 1 in 247, and in two of four runs it was still rising after 1,000 careers. So a
settle-on-best figure is the cost of "breed for at most H decisions, then never improve again", which neither
policy actually does. The settle-on-best planner − greedy differences (−143 on u4, −2,143 on owner4) measure the
planner's head start over a greedy that has been frozen, not which policy reaches a hit sooner.

Where runs settled (settle-on-best files): on u4, 12 of 48 planner runs bred, gained nothing and returned to
today's best (Inari One [Edomurasaki] from legacies 2 and 3, 1 in 587). The rest settled between 1 in 198 and
1 in 510 (median 418 over all 48). Greedy settled at 1 in 587 (36 runs) or on Agnes Digital (outfit not recorded) from legacies 2 and
12 at 1 in 556 to 582 (12 runs). On owner4, 8 of 48 planner runs returned to Agnes Digital [Full-Color Fangirling]
from legacies 2 and 3 (1 in 7,589). The rest settled between 1 in 1,525 and 1 in 6,919. All 48 greedy runs stayed
at 1 in 7,589.

The model choices that make a policy hard to simulate faithfully:

- The solver's rollout values a veteran at averaged stars and expected G1 wins; the simulator draws them. Because
  q = 1 − (1 − min(1, 0.03 s (1 + A/100)))² is not linear in the wins (through A) or the stars, the rollout's f is
  not the expectation of the drawn outcomes. The simulator is the faithful side.
- `outcome_sets` and `schedule` come only on `results[0]` and `rollout.choice`. A policy that runs any other career
  needs a second solve with that career forced (`trainees` + `fix`).
- Each step is a full search, and its cost grows roughly with the number of veteran pairs (greedy on u4: 0.06 s at
  12 veterans, 1.1 s at 285; the planner about 8 s at 576). Long horizons need hundreds of calls per run even with
  pruning. A repeated career keeps producing undominated veterans (about 13% of draws after 10,000), because wins
  on uncertain races and stars make many incomparable veterans.
- Pruning cannot be exact for the planner: dominated veterans can still occupy its top-K shortlist, and a veteran's
  extra wins matter to her future children (as a grandparent), which no P(all) dominance sees.
