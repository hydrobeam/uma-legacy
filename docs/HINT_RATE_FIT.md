# Training-hint rate in Independent Training: fit to 879 uma-it runs

- `hf_parse.py` parses the pages, `hf_build.py` builds the observations, `hf_cat.py` tags rows by other hint sources.
- `hf_lib.py` holds the likelihoods.
- `hf_fit3.py` and `hf_cov.py` do the fits; `hf_boot.py`, `hf_boot_u.py` and `hf_boot_s.py` do the bootstraps.
- `hf_valid.py`, `hf_ucv.py` and `hf_scv.py` do the held-out checks.
- `hf_cell_test.py`, `hf_extra.py`, `hf_excl.py`, `hf_skill.py` and `hf_spec.py` are the mechanism checks.
- `hf_impl2.py` computes the card odds tables.

numpy and scipy came from the uv cache through `hf_run.sh`; nothing was installed into the repo.

## Verdict

The planner's training-hint model is wrong in three ways, and a fourth term is now supported.

1. **The draw pool is the card's skills only (`nskill`).** The stat option does not compete with the skills.
2. **Hint frequency multiplies a base: `h0 × (1 + hf/100)`.** The planner's `1 + hf/25` has a slope-to-intercept
   ratio four times too steep and an intercept 2.5–2.7 times too low for Unity Cup.
3. **The base depends on the scenario.** Unity Cup is the highest at about 2.7, Grand Concert 2.07, URA 1.55.
4. **Specialty priority (support effect 19) also raises the count**, by about 2.5% per 10 points
   (90% interval 1.7–3.3%). This term is new in the 879-run fit. Without it, Unity's hint-frequency divisor fits at
   61 instead of 100. With it, the divisor goes back to 92 (81–103) pooled and 81 (64–105) for Unity alone.

Recommended formula (fitted on rows where no spark had hinted the skill):

```
P(card's training hints skill s) = 1 − (1 − 1/n)^h,      n = nskill (skills in the training-hint pool)
h = h0[scenario] × (1 + hf/100) × exp(0.0251 × spec/10)
    hf, spec = hint frequency and specialty priority at the card's level (max LB in the planner)
    h0: Unity Cup 2.50 (90% interval 2.40–2.63) · Grand Concert 1.91 (1.82–2.01) · URA 1.41 (1.32–1.67)
    0.0251 per 10 specialty points: 0.0165–0.0333
```

The shape (uniform draws with replacement over `n`, `h` draws per run) is derived from the data. The coefficients
are fitted. Nothing was checked against game code.

## 1. What a page shows per card per run

|---|---|---|
| Unity Cup | 1,130 | 51 |
| Grand Concert | 1,403 | 58 |
| URA | 223 | 9 |
| Trackblazer | 12 | 2 |

On `nskill`: the planner's `nskill` equals `opts − 1` for every card in the data. The 12 Global cards with
`nskill = opts` are all pals with zero options, so they never enter the fit. The fit reads `nskill` from
`web_cards.json`, and it matches the number of `single_mode_hint_gain` skill rows for every card in the data.

**The `Hints` column of `#contribs` counts distinct skills.** It equals the number of `#hints` entries that list
the card's bare character name as a source, in all 2,768 card-runs. Every such skill is in the card's pool.

**Hint levels show repeat draws.** Take the rows where the card's training is the only source. There, Lv is a
multiple of the per-hint increment `1 + (Hint Lv effect at the card's level)`, capped at 5:

| Hint-Lv effect | Lv distribution of single-source training hints |
|---|---|
| 0 (+1 per draw) | Lv1 1,674 · Lv2 228 · Lv3 12 |
| 1 (+2) | Lv2 77 · Lv4 12 |
| 2 (+3) | Lv3 1,209 · Lv5 240 · (Lv1 2: an R Matikanefukukitaru) |
| 3 (+4) | Lv4 1,371 · Lv5 277 · (Lv2 3, Lv3 1: Biwa Hayahide / Zenno Rob Roy, likely under-levelled) |
| 4 (+5) | Lv5 44 |

So 5,144 of 5,150 rows are consistent. The draw count k per skill is exact for +1 cards (up to 4). For the others it
reads as k = 1 or k ≥ 2. Where another source also lists the skill, only k ≥ 1 is known.

**Effect values interpolate linearly between milestone levels and are floored.** The hint-level data shows this.
King Halo at LB3 has Hint Lv 2.75, which floors to 2 (increment 3, as observed). Gold Ship at LB3 has 3.5, which
floors to 3 (increment 4, as observed); step values and rounding both fail on these cards. For hint frequency the
rule hardly matters: log-likelihood is −10,676.9 floored-linear and −10,677.4 step. Using max-LB hint frequency for
every card regardless of limit break is worse, at −10,683.8.

## 2. How the draws behave

**The stat option is not one of the uniform options.** If it were, T's variance would have a binomial floor. In 16
of 20 (card, hf) cells the observed variance is below that floor. Within setups, I gave both models the same
freedom: draw count t or t+1 with a free mixing weight per group. Draws over the n skills beat draws over n + 1
options by 843 log-likelihood units. All 147 groups with at least 5 runs favour skills-only.

**Within the pool, draws are uniform and with replacement.** The repeat rate P(k ≥ 2 | k ≥ 1) on single-source
rows (increment ≤ 4, n = 5,100) is observed at 0.151. Skills-only draws with T two-point around h predict 0.151;
Poisson T predicts 0.199.

| distinct skills per card-run | 0 | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| observed | 0.000 | 0.093 | 0.520 | 0.307 | 0.074 | 0.005 |
| skills-only, T two-point | 0.000 | 0.084 | 0.477 | 0.358 | 0.079 | 0.002 |
| skills-only, T Poisson | 0.072 | 0.218 | 0.297 | 0.236 | 0.120 | 0.043 |

Within-card uniformity fails clearly only for Special Week (χ² 113 on 12 df: some pool skills are never hinted).

| other source of the same skill | rows | observed | predicted | ratio |
|---|---|---|---|---|
| none | 14,949 | 0.345 | 0.346 | 1.00 |
| Inspiration (spark) | 1,696 | 0.269 | 0.348 | 0.78 |
| Inspiration + card event | 139 | 0.158 | 0.370 | 0.43 |
| Inspiration + other card | 81 | 0.173 | 0.356 | 0.48 |
| general Events | 820 | 0.373 | 0.363 | 1.03 |
| card event | 635 | 0.334 | 0.352 | 0.95 |
| other card's training | 572 | 0.372 | 0.375 | 0.99 |

Default-target whites come out 0.173 observed vs 0.352 predicted when a spark hinted them, and 0.331 vs 0.349 when
none did. This deficit does not change the planner's "hinted by any source" odds: the card only matters in runs
where no spark hinted the skill. The fits therefore use only the 17,079 rows where no spark hinted the skill.

## 3. Fits

Maximum likelihood of the per-skill Bernoulli model P = 1 − (1 − 1/n)^h(θ), which is the planner's own functional
form. The first table uses all 19,071 rows.

| form for h | parameters | log-lik | AIC |
|---|---|---|---|
| current: 1 + hf/25 over `opts` | none | −12,813.3 | 25,627 |
| 1 + hf/25 over n | none | −12,461.0 | 24,922 |
| constant over n | 2.71 | −12,048.0 | 24,098 |
| a + b·hf over `opts` | 2.52 + 0.0267·hf | −11,916.7 | 23,837 |
| a + b·hf over n | 2.11 + 0.0257·hf | −11,907.1 | 23,818 |
| base·(1 + hf/100) over n | 2.20 | −11,910.2 | 23,822 |
| base_scen·(1 + hf/100) | URA 1.50, Unity 2.64, GC 2.00, TB 1.74 | −11,817.8 | 23,644 |
| base_scen·(1 + hf/c) | c = 76 | −11,811.9 | 23,634 |
| a_scen + b_scen·hf | Unity b = 0.0387, others ≈ 0.020 | −11,806.4 | 23,629 |

| model | parameter | estimate | 90% interval |
|---|---|---|---|
| h0·(1 + hf/100), pooled | h0 | 2.28 | 2.18–2.38 |
| h0·(1 + hf/c), pooled | c | 86 | 73–100 |
| h0_scen·(1 + hf/100) | Unity | **2.74** | **2.69–2.81** |
| | Grand Concert | 2.07 | 2.02–2.13 |
| | URA | 1.55 | 1.48–1.84 |
| h0_scen·(1 + hf/c), common c | c | 79 | 69–90 |
| Unity only, h0·(1 + hf/c) | h0, c | 2.47, 61 | 2.34–2.59, 51–76 |
| Unity old 679-page subset only | c | 87 | 59–134 |
| Grand Concert only, h0·(1 + hf/c) | c | 110 | 99–121 |
| **h0_scen·(1 + hf/100)·exp(γ·spec/10)** | Unity | **2.50** | **2.40–2.63** |
| | Grand Concert | 1.91 | 1.82–2.01 |
| | URA | 1.41 | 1.32–1.67 |
| | γ | 0.0251 | 0.0165–0.0333 |
| same, common c free | c | 92 | 81–103 |
| Unity only, with specialty, c free | c | 81 | 64–105 |
| Grand Concert only, with specialty, c free | c | 115 | 102–129 |
| Unity only, planner's `opts` parametrisation | a + b·hf | 2.95 + 0.043·hf | |

**Why specialty priority enters.** In Unity, the 200 new pages want a much steeper hint-frequency slope than the
old ones. The pattern is card-shaped, not page-shaped:

- Marvelous Sunday [Dazzling Day in the Snow] (hf 0, spec 0) and Matikanefukukitaru (hf 0, spec 0) hint less than
  the hf-only model predicts.
- Agnes Tachyon (spec 57–80), King Halo (55–70) and Eishin Flash (65) hint more.

Specialty priority and hint frequency correlate (r = 0.35), so a hint-frequency-only model reads the specialty
effect as a steeper hint-frequency slope. Once specialty priority is in, the Unity divisor is consistent with 100.
The specialty model moves each of these heavily used cards toward its observed rate:

| Unity card | runs | observed per-skill rate | h0·(1 + hf/100) | + specialty |
|---|---|---|---|---|
| Marvelous Sunday [Dazzling Day] (hf 0, spec 0) | 172 | 0.408 | 0.457 | 0.428 |
| Matikanefukukitaru (hf 0, spec 0) | 165 | 0.295 | 0.344 | 0.320 |
| Agnes Tachyon (hf 42–50, spec 57–80) | 107 | 0.489 | 0.422 | 0.457 |
| King Halo (hf 40–50, spec 55–70) | 51 | 0.545 | 0.509 | 0.530 |
| Agnes Digital SR (hf 40, spec 0) | 25 | 0.435 | 0.503 | 0.472 |
| Eishin Flash (hf 40, spec 65) | 25 | 0.517 | 0.446 | 0.471 |

The mechanism is plausible: specialty priority raises how often a card shows up in its own training type. That is
inference, not observation.

h in Unity Cup, in skill draws per card per run:

| hf | 0 | 30 | 40 | 50 | 60 |
|---|---|---|---|---|---|
| current 1 + hf/25 | 1.0 | 2.2 | 2.6 | 3.0 | 3.4 |
| 2.74·(1 + hf/100) | 2.74 | 3.56 | 3.83 | 4.11 | 4.38 |
| 2.50·(1 + hf/100)·exp(0.0251·spec/10), spec 0 | 2.50 | 3.25 | 3.50 | 3.75 | 4.00 |
| same, spec 50 | 2.83 | 3.68 | 3.96 | 4.25 | 4.53 |

## 4. Other dependencies

Each was added as a factor exp(γ·x) on base_scen·(1 + hf/100), using all rows. The ΔLL values treat rows as
independent, which they are not.

| covariate | factor on h | ΔLL |
|---|---|---|
| card type (vs Speed): Stamina / Power / Guts / Wit | 1.00 / 0.95 / 1.02 / 0.91 | 2.9 (4 df) |
| SR vs SSR | 1.02 | 0.0 |
| limit break < 4, beyond hf | 0.97 | 0.6 |
| pals per deck | 1.001 per pal | 0.0 |
| Stamina preset × Stamina card | 1.00 | 0.0 |
| pool holds a default-target white | 0.94 | 3.3 |
| **specialty priority, per 10** | **1.028** | **18.0** (bootstrap interval excludes 0) |
| initial bond, per 10 | 1.02 | 0.8 |
| pool size n | 1.002 per skill | 0.1 |

## 5. Validation

| model | log-loss / row | Brier | RMSE of distinct per card-run |
|---|---|---|---|
| current 1 + hf/25 over `opts` | 0.6841 | 0.2385 | 1.045 |
| 1 + hf/25 over n | 0.6641 | 0.2318 | 0.911 |
| a + b·hf over `opts` (refit) | 0.6309 | 0.2200 | 0.656 |
| base·(1 + hf/100) over n | 0.6304 | 0.2198 | 0.645 |
| base_scen·(1 + hf/100) over n | 0.6253 | 0.2174 | 0.548 |
| + specialty priority | **0.6246** | **0.2171** | |

**Unity only** (51 folds):

| model | log-loss / row |
|---|---|
| current | 0.7572 |
| 2.74·(1 + hf/100) | 0.6613 |
| h0·(1 + hf/c) | 0.6601 |
| with specialty | **0.6592** |

The current model under-predicts distinct skills by about half at hf 0 (0.85 predicted vs 1.92 observed) and
in Unity (1.53 vs 2.71). Across the 161 (card, skill) pairs with at least 30 card-runs, held-out z-scores have
mean −0.23 and SD 1.31; 18 exceed |2| and 3 exceed |3|. The per-card observed/predicted ratio of distinct skills
has SD 0.074, ranging from 0.84 to 1.10.

## 6. What it means for the planner

Change `Sources::train` (`pkg/loopopt/src/deck.rs`) to use `n = nskill` and the h above. The specialty version
needs one more exported field per card: effect 19 at max limit break, interpolated the same way as hint frequency,
plus any unique-effect specialty priority. The page's model text and the MODEL.md row change with it. The full
`./build.sh val` with new references is needed.

Unity per-target probabilities at max limit break:

| card | hf | spec | n | now | 2.74·(1+hf/100) | + specialty |
|---|---|---|---|---|---|---|
| Sweep Tosho [It's All Mine!] (Slipstream) | 0 | 65 | 7 | 0.125 | 0.344 | 0.365 |
| Super Creek [Piece of Mind] (Ramp Up) | 0 | 55 | 5 | 0.167 | 0.457 | 0.473 |
| Marvelous Sunday [A MORE MARVELOUS WORLD! ☆] (Ramp Up, Tail Held High) | 0 | 50 | 5 | 0.167 | 0.457 | 0.469 |
| Marvelous Sunday [Dazzling Day in the Snow] (Ramp Up, Tail Held High) | 0 | 0 | 5 | 0.167 | 0.457 | 0.428 |
| Fine Motion [Wave of Gratitude] (Nimble Navigator) | 0 | 35 | 5 | 0.167 | 0.457 | 0.457 |
| Matikanetannhauser [Machitan☆Adventure] (Nimble Navigator) | 0 | 35 | 6 | 0.143 | 0.393 | 0.393 |
| Seiun Sky [Paint the Sky Red] (Tail Held High) | 30 | 20 | 6 | 0.288 | 0.477 | 0.464 |
| Maruzensky [Sentimental Flare ♪] (Slipstream, Playtime's Over!) | 30 | 35 | 10 | 0.189 | 0.313 | 0.312 |
| Daitaku Helios [Make! Some! NOISE!] (Ramp Up, Slipstream) | 40 | 35 | 7 | 0.293 | 0.446 | 0.446 |
| Daitaku Helios [Sunlit Angel ♪] (Slipstream) | 50 | 50 | 9 | 0.271 | 0.384 | 0.394 |
| Ines Fujin [Watch My Star Fly!] (Slipstream, Playtime's Over!) | 50 | 80 | 5 | 0.421 | 0.600 | 0.641 |
| Nakayama Festa [Trusting the Dice] (Nimble Navigator, Uma Stan, Risky Business) | 50 | 0 | 12 | 0.213 | 0.301 | 0.279 |
| Yaeno Muteki [Fiery Discipline] (Ramp Up, Tail Held High) | 60 | 20 | 5 | 0.462 | 0.624 | 0.609 |
| Gold Ship [Breakaway Battleship] (Uma Stan) | 60 | 0 | 14 | 0.209 | 0.277 | 0.257 |

Every card's training odds rise, and the gain is largest for hf 0 cards (about 2.5–2.9×). The specialty term
reorders cards that share a target:

- **Dazzling Day vs A MORE MARVELOUS WORLD!** Both hf 0, but Dazzling Day has spec 0, so the two Marvelous Sunday
  SSRs are no longer equal (0.428 vs 0.469).
- **Ramp Up.** Daitaku Helios [Make! Some! NOISE!] (0.446) now sits between the two Marvelous Sundays.
  The first pass's claim that both Marvelous Sundays beat it holds only for A MORE MARVELOUS WORLD!.
- **Who gains most.** High-specialty cards (Ines Fujin, Sweep Tosho [It's All Mine!]) gain beyond the hint-frequency
  formula.
- **Who loses.** Spec 0 cards (Gold Ship [Breakaway Battleship], Nakayama Festa [Trusting the Dice]) lose some of
  their gain.

**Joint odds within one card.** One card's T draws are shared across its pool, so two targets in one card's pool
are negatively correlated. The planner treats training hints as independent per target. Take Marvelous Sunday at
h = 2.75 (n = 5, T two-point on {2, 3}). P(one target) is 0.456 and P(both) is 0.164; independence gives 0.208,
27% too high.

## 7. What changed from the 679-page fit

- **Specialty priority:** it was not significant before (ΔLL 3.0) and now is (ΔLL 18, γ 0.0165–0.0333 per 10).
  With it, the divisor is consistent with 100 again: 92 (81–103) pooled, 81 (64–105) Unity.
- **Unchanged:** skills-only draws, near-fixed T per setup, uniform draws with replacement, spark exclusion and
  floored interpolation all replicate.

## Caveats

- **Specialty is the newest finding.** The specialty term rests mostly on the 200 new pages, and its estimate moved
  between passes. It is significant under the bootstrap, but it should be confirmed on further pages before it
  becomes load-bearing. Until then, the fallback 2.74·(1 + hf/100) is defensible.
- **Per-setup error.** IT rounds a per-setup count, so any single setup can be off by about half a draw from the
  smooth formula.
- **Inferred mechanism.** Skills-only draws, the spark exclusion and the specialty mechanism come from receipts,
  not from game code.
- **Scope.** These are IT-only numbers. There is no data for the GM (JP) era.

## Verification (2026-10-05, independent re-derivation)

- **Rows and parsing reproduce:** 879 runs, 19,071 (card-run, pool skill) rows, 17,079 without a spark hint. Every
  training hint a page credits to a card is in that card's master.mdb pool (0 unmatched names), nskill equals the
  distinct pool size on every row, scenario ids match master.mdb (1 URA, 2 Unity Cup, 3 Grand Concert,
  4 Trackblazer), and `web_cards.json` hf equals effect 18 at max level plus the unique effect's for every card.
- **The joint formula holds.** Over 38,927 pairs of no-spark pool skills on one card-run, P(both hinted) is 0.0872
  observed, 0.0861 from the solver's P(none of A) = (1 − |A|/n)^h, and 0.1134 if the two were independent.
  P(neither) is 0.4371 against 0.4355.
- **Calibration by pool size** (Unity + Grand Concert) is within 0.88–1.11 for every n with ≥150 rows.
- **Spark "drop-out" is not established.** If spark-hinted skills left the draws, a card's other pool skills would
  gain. With two other pool skills spark-hinted, they are hinted at 0.386 against 0.351 with no drop-out and 0.465
  with full drop-out. Full drop-out is rejected. Partial redirection of the draws after the inspiration, or a page
  crediting artifact, both fit. The planner is unaffected either way, because the fit conditions on no spark,
  which is the case where a card's hint matters.
- **The Unity hint-frequency slope rests on three cards.** With 2.74 × (1 + hf/100), Unity hf 0 rows come out 0.368
  observed against 0.398 predicted, and hf 46+ rows 0.461 against 0.420. Grand Concert is flat. Per card in Unity:
  Marvelous Sunday [Dazzling Day in the Snow] 0.89, Matikanefukukitaru [Touching Sleeves] 0.86, Agnes Tachyon [Q≠0]
  1.16, Shinko Windy 1.04, Daitaku Helios [Sunlit Angel] 1.04 (rough ±0.07–0.10). The same first three cards are
  1.09, 0.97 and 1.07 in Grand Concert. The specialty term and a steeper Unity slope are both fitted to this handful
  of cards, so neither is established. The adopted model over-rates hf-0, specialty-0 cards in Unity by about 10%.
- **Not re-checked:** the near-fixed draw count per setup (it rests on +1-level cards). Its effect on the solver's
  fractional-h formula is under 1% (n = 5, h = 2.74: P(miss) 0.5426 fractional vs 0.5453 two-point).

## Lv-5 cap (2026-10-05)

The owner's rule: a skill at hint Lv 5 can receive no more hints, so it leaves the card's draw pool. A card giving
`inc` levels per draw (1 + hint-level effect, 17) draws one skill at most ceil(5 / inc) times: 5 for +1, 3 for +2, 2
for +3 and +4, 1 for +5 (Global: 86, 13, 94 + 41 and 6 cards with skills; the +5 cards are Gold Ship [Breakaway
Battleship], Yaeno Muteki [Fiery Discipline], Air Shakur, Gold City, Hishi Akebono, Taiki Shuttle).

Fitted on the max-limit-break no-spark rows with T two-point around h (`hv/capfit.py`), the cap model beats draws
with replacement by 7.8 log-likelihood with no extra parameter (Unity 5.1, Grand Concert 2.7). Observed / predicted
by draws to Lv 5:

| draws to Lv 5 | rows | with replacement | with the cap |
|---|---|---|---|
| 5 (+1) | 5,347 | 0.968 | 0.979 |
| 3 (+2) | 259 | 0.863 | 0.870 |
| 2 (+3, +4) | 7,335 | 1.025 | 1.024 |
| 1 (+5) | 94 | 1.289 | 0.985 |

Bases with the cap (owner-clustered bootstrap, 90%): Unity 2.696 (2.66–2.75), Grand Concert 2.076 (2.03–2.13), URA
1.602 (1.48–1.73), pooled 2.296 (Trackblazer's placeholder). Adopted as 2.70 / 2.08 / 1.60 / 2.30.

Not modelled: other sources (sparks, events) pushing a skill to Lv 5 before the card draws it, which also removes it
from the card's pool and sends those draws to the card's other skills. That is the likely cause of the spark deficit
in section 2 (and of the partial redirection seen in the verification), and it makes a card's hints slightly more
likely to land on targets a spark did not already max.
