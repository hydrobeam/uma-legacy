# Policy simulator results (2026-10-07)

The simulator (`simulate` on a career job, docs/SOLVER.md "Policy simulator", `pkg/loopopt/src/deck/career/sim.rs`)
measures what `val/policy_sim.mjs` measures, under the solver's model: the expected number of careers until the first
career whose veteran lands every target, for a decision policy on a fixed starting pool. Per run (derived):

    E = sum over careers t of S_t,  S_0 = 1,  S_{t+1} = S_t (1 - p_t),  stop at S_t < 0.001, plus the tail S_t / p_t

Every number below is a measurement of the model by simulation (seed 1, run indices 0..N-1 under every policy, so the
same run index faces the same uniforms), with a 95% Student-t interval over runs. "Paired" differences are per run index.
CRN pairing only holds while two policies make the same picks; once they diverge the paths draw different veterans, so
paired spreads stay large.

Pools: `career_U5` (`pkg/loopopt/jobs/career_U5.json`: 12 veterans, three each of Oguri Cap, Agnes Digital, Tamamo Cross and Inari One, everything owned, Unity Cup golds guaranteed, default 8 targets, star mix 50/45/5); `pool4` (Fine Motion,
Kitasan Black, Gold Ship, Special Week) and `pool14` (Gold Ship, Special Week, Agnes Digital, Seiun Sky) from
`val/policy/pools/`: a real player's roster, owned cards and veterans, Unity Cup, same targets and star mix.

## Reproduce

    cargo build --release --manifest-path pkg/loopopt/Cargo.toml
    uv run val/sim/make_jobs.py /tmp/sim          # writes the job files and queue.txt
    (cd pkg/loopopt && for n in $(cat /tmp/sim/queue.txt); do target/release/loopopt data.json /tmp/sim/$n.json > /tmp/sim/$n.out; done)
    uv run val/sim/report.py runs /tmp/sim/u5_*.out          # E per policy, paired differences
    uv run val/sim/report.py audit /tmp/sim/u5_*.out         # fidelity audits
    uv run val/sim/report.py e1 /tmp/sim/e1_*.out            # E1 checks

(Run the queue 5 at a time with `xargs -P 5`; it took about 4 hours of wall time on an i5-1340P under other load
using 4 cores. The rollout jobs dominate. The solver runs from `pkg/loopopt` because the pool files' paths are relative
to it.)

## Verdict

The simulator reproduces the exact model's trajectories bit for bit wherever it was compared (14 whole trajectories of
greedy and the loop on three pools, plus the planner's first careers), at 20-150 times the speed. On the policy question: on `pool4` and `pool14` a plain
3-uma rotation (`loop`) beats re-running the best career after every career (`greedy`) by a wide, significant margin
(pool14: 121 careers fewer, 95% CI 95-147; pool4: 2,600 fewer, CI 1,850-3,350); on `career_U5` the two are within
noise (greedy 12.6 fewer, CI -3.3 to +28.5). One-step rollouts over either base gain nothing measurable on any pool:
with 4-8 paired samples their candidate scores are noise-dominated, so they deviate from the base pick at random.
The rollout planner (the page's pick before `next` replaced it; the exact solver per decision) finished one run: 227.2 careers on run index 0,
where greedy took 273.5 and the loop 381.4. One run, no interval: suggestive, not a measurement of its mean. It costs
2.2 hours per run in process, which is why the other runs were stopped (three more were at 400-1,100 careers).

## E2: the simulator against the exact model (measured)

| Check | Simulator | Exact | Same? |
|---|---|---|---|
| `career_U5` greedy runs 0-3 vs val/policy_sim.mjs (frozen binary, two independent batches) | E 273.494, 242.377, 273.906, 254.189; 1,677 / 1,419 / 1,736 / 1,291 careers | same | bit-identical E (all 17 digits), careers and pick sequences; 2.6-36 s per run against 574-1,862 s |
| `career_U5` greedy runs 4-5 vs `greedy_exact` (the solver in process per decision) | 472.466, 306.027 | same | bit-identical E and picks (56 and 166 s against 2,212 and 7,447 s; the simulator times include its audits) |
| `pool14` greedy, 1,000 careers vs `greedy_exact` | 817.0797 | 817.0797 | bit-identical, same picks (7 s against 1,484 s) |
| `pool4` greedy, 1,000 careers vs `greedy_exact` | 1,225.1583 | 1,225.1583 | bit-identical, same picks (42 s against 6,089 s) |
| `loop` (margin 0.02) vs the exact loop (margin -1: a fresh full deck search for every career and every keep-or-replace term): U5 runs 1-2, pool14 runs 0-1, pool4 runs 0-1 | 288.17, 235.15; 754.48, 647.80; 837.18, 877.15 | same | bit-identical E and picks, all six (14-132 s against 169-1,509 s) |
| `planner` in process vs policy_sim's planner, U5 run 0, first 5 careers | picks 1034:2,6 / 1034:1,3 / 1034:2,3, same p | same | bit-identical |

Under `margin: -1` (used for every greedy-based E3 run) the greedy pick is exact by construction: every candidate no
proof rules out gets a fresh exact deck search, with the same proofs the career search uses. Margins 0, 0.01 and 0.05
gave bit-identical E on all 48 `career_U5` runs (refreshes 2,328 / 1,347 / 1,139).

The greedy E distribution: mean 279.7 over 48 runs (the three earlier exact long runs, 242, 254 and 274, sit inside it;
sd 55, range 205-472). E is far from normal: a few runs that stay on a weak best career for long pull the mean up.

## E1: surrogate fidelity (measured)

Exact checks inside simulated greedy runs (margin 0): every 100 / 250 / 300 new veterans, the exact career search's
pick, and up to 30 of the newest veteran's candidates (ceiling at least 0.3 x the best) with a full exact deck search.

| Pool | Argmax checks | Surrogate pick differs | Lineages | Trainee has a cached deck | Table vs exact, those | Landed-set TV |
|---|---|---|---|---|---|---|
| `career_U5` (16 runs, pools 12-1,012) | 122 | 0 | 3,765 | 3,432 | all within 6.5e-16 | 0 |
| `pool14` (2 runs, pools 14-1,014) | 10 | 0 | 308 | 26 | all within 4.1e-16 | 0 |
| `pool4` (1 run, pools 4-1,204) | 5 | 0 | 154 | 52 | 45 exact; 7 short by 3.9-4.0% (exact about 0.46 x the best) | up to 0.045 |

Audits inside every E3 experiment: at each greedy pick change the exact career search's pick,
plus the pick and 3 random candidates with the newest veteran (ceiling at least 0.3 x the best); at each rollout
decision the chosen and 3 random candidates; every 50 loop careers her deck and the keep-or-replace decision under
exact values. Rows compare the surrogate's table (her cached decks, no refresh) with a fresh exact deck search.

| Experiment | Greedy pick changes, differs from exact | Pick / loop-career / rollout-candidate rows: table vs exact | Random candidates: no cached deck (bounds only) | Random candidates with a deck: error | Loop keep-or-replace, same under exact |
|---|---|---|---|---|---|
| U5 greedy (48 runs) | 830, 0 | 830 picks, max 4.8e-16 | 2,447 of 2,490 (exact at most 0.53 x the pick) | 43, max 5e-16 | |
| U5 loop (48) | | 1,280 loop careers, max 5.9e-16 | | | 1,232 of 1,232 |
| U5 rollout_greedy, first career (16) | 274, 0 | 274 picks + 64 rollout candidates, max 4.7e-16 | 12 of 822 | 810: 7 above 1e-9, max 5.5%, TV 0.038 | |
| U5 rollout_greedy, every 100 (4) | 71, 0 | 71 + 252, max 5.4e-16 | 0 of 213 | 213: 1 above 1e-9 (1.2%) | |
| U5 rollout_loop (16) | | 415 loop careers + 64 rollout candidates, max 3.8e-16 | | | 415 of 415 |
| pool14 greedy (16) | 16, 0 (the pick never changes) | 16, 0 | 48 of 48 (exact at most 0.44 x the pick) | | |
| pool14 loop (16) | | 1,101, max 6.6e-16 | | | 1,085 of 1,085 |
| pool14 rollout_greedy (8) | 1, 0 | 1 + 32, max 4.1e-16 | 0 of 3 | 3, exact | |
| pool14 rollout_loop (8) | | 549 + 32, max 6.3e-16 | | | 548 of 548 |
| pool4 greedy (16) | 98, 0 (pools 4-2,935) | 98, max 6.3e-16 | 73 of 294 (exact at most 0.78 x the pick) | 221: median 2.4%, 99th pct 56%, max 76%; TV up to 0.31 | |
| pool4 loop (16) | | 1,125, max 6.2e-16 | | | 1,109 of 1,109 |
| pool4 rollout_greedy (2) | 9, 0 | 9 + 8, max 6.2e-16 | 1 of 27 | 26: 10 above 1e-9, max 1.6%, TV 0.034 | |
| pool4 rollout_loop (8) | | 545 + 32, max 6.2e-16 | | | 545 of 545 |

Reading: everything that feeds a decision (greedy picks, loop careers, rotation decisions, rollout candidates) was exact
to rounding in every audited case, and no candidate valued only by bounds was at or above the pick. The tables
themselves are not a general valuation: on `pool4` a trainee's cached decks miss the best deck of an arbitrary,
non-competitive lineage by a median 2.4% and up to 76%. The simulator never uses a table value without the refresh
test (bound above the table by more than the margin: search), which is why its decisions stay exact; anything that
reads tables as values without that test (for example the planner's f(S) for setups outside its shortlist) would not
be. With margin 0.02 (the loop runs) a career's deck is guaranteed within 2% of the best (derived); measured: exact.

## E3: policies (measured; expected careers to the first all-target veteran)

Policies: `greedy` = the best career by P(all), re-searched after every career (policy_sim's greedy); `loop` = the
3-uma rotation of the loop planner's best members, outfits as the career model picks them per career (career_U5:
Agnes Digital, Tamamo Cross, Inari One, the loop planner's best for its settings, `val/refpu__pkg_loopopt__pal_U.out`;
pool4 and pool14: El Condor Pasa, Agnes Digital, Yamanin Zephyr, none of whom has a veteran in either pool); `rollout_*` =
at the first career (or every 100 careers) score the base policy's career, the 6 best careers, the best career of the
4 best trainees and the best career with each of the 2 newest veterans by running each, then the base policy to
S < 0.001 (0.05 for the every-100 variant), averaged over 8 (U5) or 4 (pool4, pool14) paired samples; run the lowest.
`repeat` (today's best forever) is exact: 1/p*.

| Pool | Policy | Runs | E | 95% CI | Careers simulated | Final best P(all) | Paired vs greedy (policy - greedy) |
|---|---|---|---|---|---|---|---|
| career_U5 | repeat | exact | 587 | | | 1/587 | |
| | greedy | 48 | 279.7 | 263.6-295.8 | 1,513 | 1/182 | |
| | loop | 48 | 292.3 | 280.5-304.1 | 1,310 | 1/156 | +12.6 (-3.3 to +28.5); greedy lower in 31 of 48 |
| | rollout_greedy, first career | 16 | 305.9 | 257.9-353.8 | 1,602 | 1/187 | +15.9 (-28.4 to +60.2); greedy lower in 13 of 16 |
| | rollout_greedy, every 100 careers | 4 | 260.7 | 236.8-284.5 | 1,522 | 1/200 | -0.3 (-1.6 to +0.9) |
| | rollout_loop, first career | 16 | 298.7 | 268.7-328.6 | 1,322 | 1/156 | +8.7 (-34.0 to +51.5); vs loop +3.5 (-23.3 to +30.3) |
| | planner (the rollout pick of the time, exact) | 1 | 227.2 | (one run) | 1,286 | 1/161 | run 0 only: greedy 273.5 on the same index; stopped runs 1-3 not counted |
| pool14 | repeat | exact | 817 | | | 1/817 | |
| | greedy | 16 | 817.1 | (no spread) | 5,641 | 1/817 | |
| | loop | 16 | 696.1 | 669.9-722.2 | 3,418 | 1/449 | -121.0 (-147.2 to -94.9); loop lower in 16 of 16 |
| | rollout_greedy, first career | 8 | 817.1 | 817.0-817.3 | 5,641 | 1/817 | +0.1 (-0.1 to +0.2) |
| | rollout_loop, first career | 8 | 699.1 | 657.3-741.0 | 3,456 | 1/508 | -117.9 (-159.8 to -76.1); vs loop +3.7 (-54.6 to +61.9) |
| pool4 | repeat | exact | 7,589 | | | 1/7,589 | |
| | greedy | 16 | 3,361.9 | 2,611.5-4,112.3 | 19,513 | 1/2,088 | |
| | loop | 16 | 762.0 | 725.6-798.4 | 3,489 | 1/510 | -2,599.9 (-3,346.2 to -1,853.5); loop lower in 16 of 16 |
| | rollout_greedy, first career | 2 | 1,225.2; 2,970.2 | | 14,244 | 1/1,689 | runs 0-1: greedy 1,225.2 and 4,647.0 (too few runs for an interval) |
| | rollout_loop, first career | 8 | 709.0 | 669.4-748.6 | 3,430 | 1/444 | -3,033.3 (-4,156.8 to -1,909.9); vs loop -54.5 (-124.3 to +15.2), the base career kept in 4 of 8 |

The tail S/p at the stop (survival below 0.001) is small against E everywhere: mean 0.19 careers on U5 greedy (max
0.32), 0.16 for the loop, 0.82 on pool14 greedy, 2.5 on pool4 greedy (max 4.9), about 0.5 for the pool4 and pool14 loops.

What the numbers say:

- Greedy is myopic where the pool is thin. On pool14 its pick never changes in 5,641 careers in any of 16 runs: E is
  exactly 1/p* = 817. Reading (not measured separately): the best career's own veterans are of the trainee's
  character and cannot parent her, and no other career they enable beats 1/817. On pool4 it climbs from 1/7,589 to
  about 1/2,100 over 19,500 careers. A fixed rotation that trains every member in turn, even at P(all) near 1e-5 at
  first, builds lineages that reach 1/449 (pool14) and 1/510 (pool4) and wins by 15% and 77%.
- On `career_U5`, where the pool already holds the loop's three characters, greedy and the loop are within noise;
  greedy reaches its best (1/182) less high than the loop (1/156) but gets there with fewer careers spent at low odds.
- Rollouts over greedy cannot find the loop's gain: every continuation is greedy, which on pool14 never moves, so all
  candidates score the same. Over the loop they gain nothing measurable either.
- Winner's curse (measured on U5): with 8 paired samples the paired 95% CI of a candidate's score against the base pick
  is typically ±10 to ±60 careers, wider than any true difference a single career makes, so the rollout picks a
  non-base career in 15 of 16 runs on noise. More samples or a lower-variance score would be needed before a
  one-career rollout can show anything.

## E4: getting off the dominant trainee (measured, 2026-10-07)

`pool14` (the career page's export: Gold Ship, Special Week, Seiun Sky and Agnes Digital veterans; the
loop planner's loop El Condor Pasa → Agnes Digital → Yamanin Zephyr). Agnes Digital's best career (1 in 817) can't
improve itself: her lineage is her two parents and theirs, so her own veterans reach it only through another uma's
career. One-step measurements first (`index`, `index2`, 40 and 20 × 10 draws): only El Condor Pasa's veteran lifted
Digital's best (8 of 80 draws), as a parent, through affinity (225 against Zephyr's 174 in the parent slot, relation and
shared G1s); new Digital, Zephyr, Gold Ship or Special Week veterans lifted nothing, and the grandparent route was worth
0 or less. Then policies to the first 8/8, runs 0-3, candidates Digital, Pasa, Zephyr, Seiun Sky, Gold Ship, Special Week:

| Policy | Mean | Runs |
|---|---|---|
| `pick_index` (10 draws per candidate) | 537 | 524, 590, 553, 480 |
| `stale` | 538 | 486, 523, 551, 592 |
| `cycle` D-P-D-Z | 611 | 605, 690, 560, 588 |
| `next_noretry` (the former page rule) | 644 | 567, 636, 701, 670 |
| `loop` | 697 | 754, 648, 618, 768 |
| `cycle` D-D-P | 744 | 767, 745, 748, 718 |
| `greedy` | 817 | frozen at 1 in 817 |
| `cycle` D-P | 849 | 860, 866, 836, 834 |

`stale` beat the former rule in all four runs (78-150 careers) and matched the sampled index, with no sampling: it is
the page's rule then (since replaced by `next`). These runs predate one change (2026-10-08): the best career's trainee now counts as a
candidate too, so with no veteran of hers from her best parents she trains first (a pool with no Digital veteran ran
three builders before her); not re-measured. The retry variants of the former rule (retry when no other member uses the newest veteran)
measured worse on pool14 (766 over 16 runs against 673 over 11 runs for no retry): the retry fires almost always once a lineage
is saturated. Wall time 33 minutes on 2 cores (fixed cycles about 5 minutes a run).

## E5: how building actually goes (measured, 2026-10-08)

`trace` (docs/SOLVER.md): building for Agnes Digital with her five most efficient builders. Each career the most
efficient one (exact lift chance x odds gained) runs, her veteran joins; 6 runs x 400 careers per pool (2-, 5-,
11- and 19-veteran pools from the same player's roster; the pool files are not tracked). Digital's odds (mean over runs; "1 in"):

| Pool | Start | 25 careers | 100 | 400 | Full-loop odds |
|---|---|---|---|---|---|
| 2 veterans | 20,926 | 1,356 | 360 | 177 | 97 |
| 5 | 2,252 | 1,446 | 605 | 346 | 163 |
| 11 | 713 | 535 | 361 | 292 | 168 |
| 19 | 496 | 415 | 354 | 312 | 177 |

The lift chance fell far faster than the first assumption (linear in the remaining log-odds gap, with the full-loop
odds as ceiling): fitted exponent 4-5 on 11 and 19, against 1; that assumption overrated building (at 400 careers it
predicted 1 in 185-194 on 11 and 19). Every pool plateaued near 0.5-0.6 of the full-loop odds: building with the best builder doesn't reach the hypothetical all-8/8 lineage. One
shape over the four pools (least squares on log odds at 10-400 careers): effective ceiling 0.6 x the full-loop odds,
lift chance x (remaining log gap)^1.5 (`CEIL_SHARE`, `LAM_DECAY` in sim/plan.rs). It tracks 11 and 19 within 10-20%;
it underestimates building on 5 (626 predicted at 400, 346 actual: there builders first get more efficient) and
overestimates it early on 2. Leave-one-pool-out fits ranged 0.6-1.0 and 1-3: a calibration on these pools, not a law.
With it, the planner picks Agnes Digital outright on 7-, 10- and 11-veteran pools of the same roster (the first
assumption overrated building there). Reproduce: a career job with `"simulate": {"policy": "trace", "runs": 6, "max_careers":
400, "margin": 1e9, "loop_members": [1019, <five builders>]}`; about 5 s a run natively.

## E5 reread (2026-10-09): bounds, the step model's error, and why

No new simulation: the E5 traces (per career: odds before, builder, its lift chance, mean lifted odds, odds after;
6 runs x 400 careers, Agnes Digital) reread. "Achievable" is min over k of the mean over runs of k + 1/p(k): build
greedily k careers then grind, a concrete strategy, so an upper bound on the model's true expected careers.

| Pool | Grind 1/p0 | Build once then grind (1/lam + 1/p1) | Cont (fit) | Achievable (best k) | 1/full-loop odds |
|---|---|---|---|---|---|
| 2 veterans | 20,926 | 11,231 | 391 | 391 (174) | 97 |
| 5 | 2,252 | 2,023 | 995 | 587 (204) | 163 |
| 11 | 713 | 614 | 425 | 446 (73) | 168 |
| 19 | 496 | 439 | 392 | 414 (53) | 177 |

- Cont tracks achievable building within 0-5% on 2, 11 and 19 veterans and is provably 70% too pessimistic on 5 (995
  above an achievable 587). Constant-free bounds (the one-lift strategy, the full-loop odds) bracket by 2-100x: the value
  is all in compounding, so no analytic bound replaces Cont.
- Why 5 fails: Cont measures lam once and only lets it decay. On 5 veterans lam jumps 0.10 -> 0.27 after the first lift
  (a new veteran is also a parent for the other builders' careers) and stays near 0.2 for ten lifts; on 11 and 19 it
  falls faster than modelled (exponent about 1.7-2.3 against the remaining gap, against 1.5). Per lift the log-odds step
  is roughly constant on 5/11/19 (x1.08-1.1), shrinking on 2; a "constant factor" form fits 5 (570) but overshoots 2
  (214) and undershoots 11/19, so the step form is not settled by this data.
- lam = max(lam now, lam one lift later) fixes most of 5 (995 -> 734) and leaves 2/11/19 unchanged. Proposed (not
  built): a one-lift lookahead for lam in `cont_of` (the best builder's lifting veteran added, the top builders' careers
  re-searched with her fixed, their lift chance at the lifted odds), gated on builder headroom or logged first; and a
  per-plan check that the pick is the same under both step costs (`next.sensitive`). Evidence covers one candidate.

## Scope: where the surrogate is verified and where it is not

Verified (measured above): three pools (`career_U5`, `pool4`, `pool14`), Unity Cup, the default 8 targets with the two
Unity golds guaranteed (6 uncertain), star mix 50/45/5 (rank below 17,500), first-year pal dates off; `career_U5` with
everything owned and the default card set; pool4 and pool14 with their player's roster (16-17 umas excluded, 29-30 outfits
excluded), 243 owned cards and 80 excluded cards, `borrow` on; no borrowed veteran in any pool; every owned uma
as trainee; loops of three members; pools up to 3,750 veterans (loop) and 2,935 (greedy).

Not verified: other scenarios (the JP Grand Masters data, URA, no guarantees), other target sets (up to 10 uncertain
targets: the corner tables grow to 1,024 entries and nothing here measured them), the SS+ star mix, pools with
borrowed veterans, 4-member loops, and other rosters. The equality "cached deck = best deck" that made the tables exact
on U5 and pool14 does not hold in general (pool4's non-competitive lineages, above); the simulator's decisions do
not depend on it, its speed does (a pool where the best deck changes often would mean many more deck searches).

Model-level assumptions are the solver's and policy_sim's, not checked here: stars independent per target and of the
landed set, G1 wins drawn per race from the schedule's chances, the last career repeated forever as the tail (bounded
above), dominated veterans pruned (exact for P(all)), and the loop's own rules (bootstrap, keep-or-replace, each
career's own schedule; the loop planner coordinates schedules across members, this does not, so its loop is a lower
estimate of a coordinated loop).
