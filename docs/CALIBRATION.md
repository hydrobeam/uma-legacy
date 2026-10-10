# End-to-end calibration of the career model against real careers

```
```

## Verdict

On the default eight targets the composed acquisition odds the planner builds on are **calibrated**: pooled
observed / predicted 0.993 [0.98, 1.01] over 20,923 run-target rows. Inheritance is right (0.985 [0.97, 1.00]).
Deck hints read **about 7% optimistic** (0.932 [0.90, 0.96]), and almost all of that is the model's assumption that
the player prioritises the targets: in runs that show their priority list, event-sourced deck hints land at
1.04 [0.97, 1.08] × predicted for a prioritised target and 0.68 [0.60, 0.76] for one not prioritised. Unmodelled
scenario and trainee events (the page's "Events") supply the few points the deck loses, which is why the composed
number lands on 1.0; counting only the sources the model knows, it is 0.972 [0.96, 0.99].

Per-run P(all eight) at the acquisition stage: 0.85 [0.66, 1.05] × the model's (bootstrap over 2,936 runs), so
consistent with right, though the interval allows the model to be up to about 1.5× optimistic (1/0.66) or 5% pessimistic; the same gold/white weighting per target gives 0.990 pooled.

Weak spots worth a look: Trackblazer (deck hints 0.68, training-pool-only 0.75 [0.58, 0.94]); three gold hints
below the model (It's On! 0.46 on 60 rows, Perfect Spot! 0.80 [0.69, 0.91], Nothing Ventured 0.92); a handful of
cards (table below).

## What was rebuilt, and the definitions

Each run becomes a `career` job (`docs/SOLVER.md`, Career planner): the trainee in her outfit (receipt name
`_uma{card}`; other outfits excluded), the run's scenario (hint base), its six cards locked at **their own limit
breaks** (hint frequency and hint-level increment read at that limit break's level cap from master.mdb, as the
training-hint fit did; `cards_lb.py` adds a variant card per limit break and reproduces web_cards.json exactly at
max limit break, King Halo LB3 increment 3 and Gold Ship [Breakaway Battleship] LB3 increment 4 as in
`docs/HINT_RATE_FIT.md`), both parents with their recorded parents and every ancestor's white skill sparks at their
stars, and race wins laid out so every pair the model reads shares the page's shared-G1 count (the solver's race
affinity then equals the page's G1 bonus in all 2,936 runs). Relation points are the model's own, from master.mdb;
the page's "Base" column follows another convention (it is 0 for every parent-2 row in 1,000 runs) and was not used.
Targets: the owner's eight, Unity Cup's No Stopping Me! and It's On! guaranteed there; `extra` as the page sends it;
no trainee own-event hints (the page sends none). The solver picks event choices and early chain ends as the planner
would. A target with no source at all (no spark on any ancestor, no deck source, not her outfit's) is scored 0 and
not sent (the career search drops any career whose P(all) is 0).

Per (run, target) row, using the career output's per-target `buy` fields:

| Mechanism | Predicted | Observed on the page | Rows |
|---|---|---|---|
| 1. Deck hint | dg + (1 − dg)·dw, P(the deck hints the skill, gold or white) | a hint card of the skill credited to a deck card: its bare name (training), "X event", or "A or B event" naming one | target not on her outfit, not a scenario gold, and **no inspiration of it** (a spark often hides the card's credit: `docs/CHAIN_EVENTS.md` section 6) |
| 1b. Deck gold | dg | a gold-tier hint card credited to a deck card | same, dg > 0 |
| 2. Inheritance | q = 1 − Π(1 − min(1, 0.03·stars·(1 + affinity/100)))² | a skill factor of the skill fired in Year 2 or Year 3 (`#factors`; Year 1 is the start-of-career application) | every row |
| 3. Acquired | gold + white (1 for a scenario gold) | the skill hinted at the end from any source (any tier, any level), or owned | not on her outfit, not a scenario gold |
| 4. Landing at the acquisition stage | the solver's landing chance p | 0.4·1.1ⁿ if a gold was hinted, 0.2·1.1ⁿ if only the white, 0 if neither (n = carriers) | as 3 |

The pages record the career before skills are bought (`owned_skill_ids` holds only the unique skill), so "acquired"
means hinted and available to buy, the model's own meaning. Intervals are 95% Wilson on the observed rate; ratio
intervals are that interval over the mean prediction.

## Data

## Results, default eight targets

| Test | Rows | Predicted | Observed [95%] | Obs / pred [95%] |
|---|---|---|---|---|
| 1. Deck hint, no inspiration | 14,015 | 0.247 | 0.231 [0.224, 0.238] | 0.932 [0.90, 0.96] |
| 1. Deck hint, training pool only | 2,982 | 0.372 | 0.357 | 0.959 [0.91, 1.01] |
| 1. Deck hint, events only | 1,864 | 0.887 | 0.807 | 0.910 [0.89, 0.93] |
| 1. Events only, target prioritised (runs with a list) | 124 | 0.890 | 0.927 | 1.042 [0.97, 1.08] |
| 1. Events only, not prioritised (runs with a list) | 220 | 0.853 | 0.582 | 0.682 [0.60, 0.76] |
| 2. Inheritance | 23,488 | 0.350 | 0.345 [0.339, 0.351] | 0.985 [0.97, 1.00] |
| 3. Acquired, any source | 20,923 | 0.498 | 0.495 [0.488, 0.501] | 0.993 [0.98, 1.01] |
| 3. Acquired through deck or inspiration only | 20,923 | 0.498 | 0.485 | 0.972 [0.96, 0.99] |
| 3. Acquired, prioritised (runs with a list) | 946 | 0.685 | 0.717 | 1.046 [1.00, 1.09] |
| 3. Acquired, not prioritised (runs with a list) | 1,422 | 0.565 | 0.518 | 0.918 [0.87, 0.96] |
| 4. Landing, pooled (sums of p) | 20,923 | 3,669.8 | 3,631.5 | 0.990 |

Per target (obs / pred):

| Target | Deck hint | Inheritance | Acquired | Landing (4) |
|---|---|---|---|---|
| Uma Stan / Superstan | 0.79 [0.64, 0.96] | 1.005 | 0.996 | 0.997 |
| Nimble Navigator / No Stopping Me! | 0.95 [0.83, 1.08] | 0.977 | 1.057 | 1.035 |
| Playtime's Over! / See Ya Later! | 0.973 [0.94, 1.00] | 0.993 | 0.984 | 0.993 |
| Ramp Up / It's On! | 0.886 [0.83, 0.94] | 1.003 | 0.938 [0.90, 0.97] | 0.924 |
| Tail Held High / Tail Nine | 0.948 [0.88, 1.02] | 0.928 [0.88, 0.98] | 0.959 | 0.962 |
| Racing Spirit: Stamina | no deck source | 0.997 | 1.060 (Events) | 1.048 |
| Slipstream / Perfect Spot! | 1.004 [0.92, 1.10] | 0.993 | 1.018 | 0.976 |
| Risky Business / Nothing Ventured | 0.833 [0.75, 0.93] | 0.968 | 0.918 [0.85, 0.98] | 0.917 |

By scenario, deck hints from the training pool only (tests the hint bases): Grand Concert 1.000 [0.94, 1.06], Unity
Cup 0.935 [0.83, 1.05], URA Finale 0.893 [0.78, 1.02], Trackblazer 0.747 [0.58, 0.94]. By the lowest limit break
of the sourcing cards: LB4 0.960, LB3 0.953, LB0-2 0.83-1.06 (41-87 rows each, all intervals cover 1). Inheritance
by carriers 1-6: 0.957-1.001, every interval covering 1; per roll, spark-years 0.985 and ancestor hits (2Σp_l)
0.987. Unity Cup's guaranteed golds: 780 of 780 runs each.

Reliability (predicted bucket → observed):

| Predicted | Deck hint: n, obs | Inheritance: n, obs | Acquired: n, obs |
|---|---|---|---|
| 0 | 8,247, 0.001 | 7,785, 0.002 | 4,958, 0.015 |
| above 0 to 0.20 | 81, 0.247 (0.19) | 2,278, 0.154 (0.16) | 1,238, 0.164 (0.16) |
| 0.20-0.40 | 2,151, 0.300 (0.32) | 2,868, 0.299 (0.30) | 2,343, 0.316 (0.31) |
| 0.40-0.60 | 1,011, 0.465 (0.48) | 3,585, 0.487 (0.50) | 2,718, 0.501 (0.50) |
| 0.60-0.80 | 736, 0.632 (0.72) | 4,851, 0.691 (0.70) | 3,899, 0.703 (0.71) |
| 0.80-0.95 | 403, 0.722 (0.87) | 2,121, 0.835 (0.84) | 2,672, 0.826 (0.86) |
| 0.95-1 | 1,386, 0.957 (1.00) | | 3,095, 0.975 (1.00) |

(mean prediction in brackets; finer buckets in the report files.) The deck's over-prediction sits in its top buckets,
the event rewards; inheritance tracks the diagonal throughout.

Gold acquisition: Nothing Ventured 0.899 [0.82, 0.98] (449 rows), Perfect Spot! 0.833 [0.75, 0.92] (366), It's On!
outside Unity Cup 0.437 [0.28, 0.65] (109), No Stopping Me! outside Unity Cup 0.894 (62), See Ya Later! 1.000 (2,336).

Per run (2,936 runs, 7.13 open targets each): targets acquired, expected 3.551, observed 3.525; all open targets
acquired, product of the model's odds 0.035, observed 0.031 [0.025, 0.038]; landing P(all eight) recomputed from what
each run acquired against the solver's exact P(all): 0.847 [0.66, 1.05] (Unity Cup 0.834 [0.61, 1.07], Grand Concert
1.30 [0.78, 1.83], URA Finale 0.81 [0.23, 1.58]; Trackblazer's model value is 0).

Cards with one deck source of a target, furthest from the model (obs / pred, rows):

| Card > target | Pred | Obs | Ratio | Rows | Likely reason |
|---|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | 1.000 | 0.730 | 0.73 [0.66, 0.79] | 185 | the model assumes Risky Business prioritised (finale choice 2) |
| Riko Kashimoto [Planned Perfection] > Ramp Up | 0.750 | 0.596 | 0.80 [0.73, 0.86] | 359 | open (her Lone Wolf choice when prioritised accounts for part) |
| King Halo [Tonight, We Waltz] > Uma Stan | 0.937 | 0.726 | 0.77 [0.66, 0.87] | 73 | Uma Stan on a choice the run must prioritise |
| Gold Ship [That Time I Became the Strongest] > Uma Stan | 0.205 | 0.122 | 0.59 [0.32, 1.05] | 74 | open |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | 0.298 | 0.070 | 0.23 [0.08, 0.62] | 43 | open |
| Maruzensky [Sentimental Flare ♪] > Slipstream | 0.240 | 0.299 | 1.25 [1.01, 1.51] | 224 | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | 0.339 | 0.491 | 1.45 [1.08, 1.82] | 57 | open |

Light Hello [From the Ground Up] > See Ya Later! 852 of 852 and Team Sirius > Risky Business 64 of 64, as modelled.

## Other skills (wide set; a 30% seeded sample of runs)

Every other skill a deck card or an ancestor's spark could give, in jobs of at most 24 targets (the solver carries at
most 32), 881 runs sampled, 1,981 jobs, 8 jobs over a 2 s limit dropped (1,973 jobs analysed). Inheritance holds:
0.993 [0.97, 1.01] over 37,785 rows. Training-pool hints 1.051 [1.03, 1.07] (14,609 rows; Grand Concert 1.05, Unity
1.02, URA 1.12, Trackblazer 1.04). Event-only rewards 0.69 [0.66, 0.73], lower than for the default targets because the
model's choices and early chain ends here serve a 24-target objective nobody plays for, and almost no run prioritises
these skills. Acquired 1.18 × predicted: scenario and trainee events (the page's "Events") hint many non-target skills
the model never counts (1,682 of the 1,706 acquisitions with neither a deck credit nor inspiration). Two skills show a
deck credit 2.3-2.5× the model's (Straightaway Acceleration, Medium Corners ◎); unexplained, not targets. 7.6% of rows
whose skill no deck card lists still carry a deck-card credit (0.1% for the default targets): random card events or
events missing from the GameTora cache.

## What this does not test

- **Spark generation and buying.** The pages stop before skills are bought and show no veteran sparks, so the step
  from acquired to landed (0.2 / 0.4 × 1.1ⁿ, and that everything hinted is bought) is untested here; test 4 and the
  P(all) ratio apply the model's own generation to what was acquired.
- **The loop planner's carriers.** Career jobs use each run's real carriers; the loop planner's "all six carry every
  target" is a different input, not a different mechanism.
- **Trainee own events, random card events, conditional events:** not in the model, so not tested beyond showing their
  size ("Events" supplies 208 target acquisitions in 20,923 rows, 1.0%).
- **Correlations inside a run** beyond P(all): the exact chain joint is used for p and P(all), but P(all) has too few
  hits (90 runs with every open target acquired) to separate the exact model from the product.
- Grand Masters (no runs), R cards (left out), hint levels.

## Caveats

- Uploads skew to good runs; every comparison is per run on that run's own deck and lineage.
- Rows with inspiration are left out of the deck test because a spark often removes the card's credit; the deck test is
  then conditional on no spark, which the model treats as independent of the deck (the training-hint fit made the same
  choice).
- Limit break from the deck widget; the card level is assumed at that limit break's cap.
- The wide set's choices and early chain ends follow a 24-target objective; read its event numbers as indicative only.

CPU: parsing 13 s, card variants 1 s, default jobs 61 s (one process), wide jobs about 4.5 min (including a first
attempt stopped after a 38 s chunk), analysis about 30 s; one process throughout.
