# Training-hint rate in Independent Training: the adopted fit

## Result (adopted)

```
h = base[scenario] × (1 + hf/100)     training-hint draws per card per run
    hf = hint frequency at max limit break, the unique effect's included (web_cards.json)
each draw uniform over the card's skills (nskill; the stat option is never drawn) still below hint Lv 5;
a skill leaves the pool after ceil(5 / inc) draws, inc = hint levels per draw (hinc): 5 for +1 cards, 3 for +2,
2 for +3 and +4, 1 for +5
P(none of the k targets in a card's pool hinted) exact over the draws; (1 − k/n)^h when no skill can reach Lv 5
fractional h: floor and ceiling, weighted by the fraction
```

| Scenario | Fitted base (2,402 runs, Lv-5 cap) | Adopted | Rows | Runs |
|---|---|---|---|---|
| Unity Cup | 2.708 | 2.70 | 8,754 | 490 |
| Grand Concert | 2.026 | 2.03 | 25,111 | 1,251 |
| URA Finale | 1.596 | 1.60 | 9,722 | 514 |
| Trackblazer | 1.814 | 1.81 | 3,349 | 118 |
| Grand Masters (JP) | no runs | 2.30, placeholder (the 879-run pooled base) | | |

The shape (uniform draws over the skill pool, h draws per run, the cap) is derived from the data; the bases are fitted.
Nothing was checked against game code.

## Data and measurement

- Per card-run, the distinct skills its training hinted: the `#hints` entries whose sources list the card's bare
  character name. That equals the page's `#contribs` Hints column in all 2,768 non-pal card-runs of the 879-run set,
  and every such skill is in the card's master.mdb pool (`single_mode_hint_gain`); `nskill` equals that pool's size.
- Rows are (card-run, pool skill), max-limit-break card-runs only, Bernoulli likelihood through the draw model. Only
  rows where no spark hinted the skill are fitted: that is the case in which a card's hint matters to the odds (see
  *Spark deficit*). 879-run set: 19,071 rows, 17,079 without a spark.
- Hint frequency is effect 18 at the card's level plus the unique effect's. Effect values interpolate linearly between
  milestone levels and are floored (King Halo at LB3: Hint Lv effect 2.75 floors to 2, increment 3 as observed; Gold
  Ship at LB3: 3.5 to 3, increment 4). For hint frequency the rule barely matters (log-likelihood −10,676.9
  floored-linear, −10,677.4 step, −10,683.8 max-LB hf for every card).
- Single-source training hints confirm the per-draw increment: 5,144 of 5,150 levels are multiples of
  1 + Hint Lv effect, capped at 5.

## What fixed the form (879-run set)

| Finding | Evidence |
|---|---|
| Skills only, not skills plus the stat option | in 16 of 20 (card, hf) cells the variance of the draw count is below the binomial floor that a drawn stat option implies; with the draw count free per setup, skills-only beats n + 1 options by 843 log-likelihood units |
| The draw count is near-fixed per setup | runs of one setup show one draw count or two adjacent ones, never more: IT behaves as if it rounds an expected count |
| Uniform draws, with repeats until Lv 5 | repeat rate P(k ≥ 2 given k ≥ 1) on single-source rows 0.151 observed, 0.151 predicted with a two-point draw count, 0.199 with a Poisson count. Distinct skills per card-run, 1/2/3/4: observed 0.093 / 0.520 / 0.307 / 0.074, model 0.084 / 0.477 / 0.358 / 0.079 |
| Hint frequency multiplies a base, divisor 100 | log-likelihood on all 19,071 rows: old 1 + hf/25 over options −12,813.3; 1 + hf/25 over skills −12,461.0; constant −12,048.0; base × (1 + hf/100) −11,910.2; per-scenario base × (1 + hf/100) −11,817.8; free divisor (c = 76) −11,811.9 |
| Scenario is the covariate that matters | per-scenario bases gain 92 log-likelihood units; card type, rarity, limit break beyond hf, pals per deck, initial bond and pool size each gain under 3.5 (factors 0.91-1.02) |
| The Lv-5 cap (owner's rule: a skill at Lv 5 takes no more training hints) | beats drawing with replacement by 7.8 log-likelihood with no extra parameter (17.5 on the 2,402 runs). Observed / predicted by draws to Lv 5, replacement vs cap: 5 draws (+1 cards, 5,347 rows) 0.968 / 0.979; 3 (+2, 259) 0.863 / 0.870; 2 (+3, +4, 7,335) 1.025 / 1.024; 1 (+5, 94) 1.289 / 0.985 |
| One card's draws over several targets are joint | 38,927 pairs of no-spark pool skills on one card-run: P(both hinted) 0.0872 observed, 0.0861 from P(none of A) = (1 − \|A\|/n)^h, 0.1134 if independent; P(neither) 0.4371 vs 0.4355 |
| Calibration by pool size | within 0.88-1.11 for every n with at least 150 rows (Unity Cup and Grand Concert) |

The +5 cards (one draw per skill): Gold Ship [Breakaway Battleship], Yaeno Muteki [Fiery Discipline], Air Shakur,
Gold City, Hishi Akebono, Taiki Shuttle. Global counts by increment: 86 cards +1, 13 +2, 94 +3, 41 +4, 6 +5.

## Spark deficit (not modelled, irrelevant to the odds)

A card is credited less often for a skill a spark also hinted:

| Other source of the same skill | Rows | Observed | Predicted | Ratio |
|---|---|---|---|---|
| none | 14,949 | 0.345 | 0.346 | 1.00 |
| Inspiration (spark) | 1,696 | 0.269 | 0.348 | 0.78 |
| Inspiration + card event | 139 | 0.158 | 0.370 | 0.43 |
| card event | 635 | 0.334 | 0.352 | 0.95 |
| other card's training | 572 | 0.372 | 0.375 | 0.99 |

Full drop-out of spark-hinted skills from the draws is rejected: with two other pool skills spark-hinted, the card's
remaining skills are hinted at 0.386 against 0.351 with no drop-out and 0.465 with full drop-out. Likely cause: sparks
and events pushing a skill to Lv 5 first, which removes it from the card's pool and sends those draws to its other
skills. Not modelled; the fit conditions on no spark, the case where the card's hint counts.

## Not adopted

- **Prioritized skills** raising a card's hint: 101 runs show the list; a prioritized skill was hinted by training at
  0.222 against 0.297 predicted (99 rows). Weak evidence; assume no effect.

## Caveats

- Per-setup error: IT rounds a per-setup count, so a given setup can be off the smooth formula by about half a draw.
  The fractional-h reading against a two-point count differs by under 1% (n = 5, h = 2.74: P(miss) 0.5426 vs 0.5453).
- Skills-only draws, the cap and the spark deficit's cause are inferred from receipts, not game code.
- IT only; no data for the Grand Masters era.
