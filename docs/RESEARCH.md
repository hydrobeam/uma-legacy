# The model's mathematics

How the planner computes its odds, how each part was set, and how well it holds against real careers.

## Results

Against 3,348 real Independent Training careers rebuilt from their own decks and lineages (section 12),
the chance the model gives each target of being acquired is calibrated: observed / predicted
0.994 [0.98, 1.01] over 23,919 run-target rows. Its chance of landing all eight default
targets reads 0.86 [0.69, 1.05] of the model, consistent with calibrated but on only 132 runs that
acquired every target.

| Part | How it is set | Main evidence | Section |
|---|---|---|---|
| Spark generation, $0.2$ / $0.4 \times 1.1^n$ | assumed | none here: the runs show inherited sparks, not produced ones | 1 |
| Star mix of sparks | mixes assumed; band per scenario derived | each scenario's band is the one most of its careers reach (14,742 Grand Concert runs, 79% at 17,500+) | 2 |
| Inheritance rolls | skill-white bases tested, grandparents fitted | grandparent weight 1.01 [0.99, 1.03], affinity slope 0.98 [0.96, 1.01] on 69,840 + 30,368 trials; careers 0.989 [0.97, 1.00] | 3 |
| Affinity: +3 per shared G1 win; own grandparent adds no trio points | fitted | shared-win slope 0.97 [0.94, 1.01]; trio rule 19 nats over the alternative | 3, 4 |
| Race win table | community table, tested | calibrated on spark-raised aptitudes (as the career planner uses them), 1.000 [0.998, 1.002] over 76,402 G1 attempts | 4 |
| Loop schedules for shared wins | derived | no schedule could raise a validated winner by more than 2.0% (solver's pair ceilings) | 5 |
| Training hints, $\beta(1 + hf/100)(1 + d\,\mathit{sp}/100)$ | form derived, $\beta$ and $d$ fitted | 61,856 rows; held out +12.9 nats; real careers 0.988 [0.94, 1.03] of the model | 6 |
| Chain events, Binomial(21, 0.35) per career | fitted | 3,424 runs; not rejected against a free distribution ($p$ = 0.06) | 7 |
| Chain events spread over remaining links | fitted | 66 nats over uniform-by-card | 7 |
| Event choices: IT takes a prioritised skill's choice | assumed | event hints of prioritised targets 1.01 of the model | 8, 12 |
| Measured outcome shares | fitted | five events, e.g. Nakayama Festa's Nothing Ventured 0.61 [0.55, 0.66] (90%) | 8 |
| Random card events | not modelled | left out, acquisition on the rows they could hint reads 1.26 [1.10, 1.42] of the model; credited, 1.00 | 9 |
| Conditional character events | not modelled | where only they can hint a target, left out reads 1.20 [0.81, 1.66]; Vodka's own Nimble Navigator 25 of 27 | 10 |

**Known misses**, each measured and tested in its section:

- Chain events: a binomial with another $n$, Binomial(18, 0.405), fits 3.4 nats better; in crowded decks early links run
  short and more chains never start than predicted (0.34 against 0.20 with six chain cards); final links fit pooled
  (0.352 against 0.357) but individual cards differ far beyond chance ($\chi^2$ = 263 on 42 cards) (7).
- Deck hints read 0.937 [0.91, 0.96] of the model, almost all from event hints of targets the
  player did not prioritise (0.71 of the model; the model assumes every target is prioritised) (12).
- Trackblazer's hint base over-predicts real careers: training-pool hints read 0.777 [0.61, 0.97] of the model (12).
- Training hints: SR cards and decks with more pals depart from the adopted rate (+5.3 and +14.0 nats); a skill
  also hinted by inspiration is hinted by training less than predicted (0.75 of the model) (6).
- Left out of the model: deck cards' random events, which under-predicts the targets they can hint
  (1.26 [1.10, 1.42] of the model on those rows) (9), and Vodka's own Nimble Navigator (25 of 27 runs) (10).
- Training-pool hints of non-target skills read 1.064 [1.04, 1.09] of the model (12).

**Not settled.** Specialty priority predicts runs it was not fitted to (+12.9 nats held out), but whether it
causes more hints is not shown: across limit breaks of the same card the estimate has the same size ($d$ =
0.41) without being significant (+1.5 nats). The hint-frequency slope $1/100$ is assumed; a free
divisor fits better.

**Assumed and untested:** the step from acquired to landed (spark generation, and that everything hinted is bought),
the star mixes, Grand Masters (no runs), hint levels, and the priority policy run by run.

## How to read this document

Every quantity carries a label: **fitted** (estimated from the runs; sample and method given), **derived** (follows
from the other parts, computed) or **assumed** (a chosen rule or an outside figure the runs cannot test; its source is
named and no evidence is claimed for it). Intervals are 95% unless marked. "Runs" are Independent Training runs from
uma-it, pooled; every run record is in the pool.

**Where the numbers come from.** Every number this document states is computed from the runs, the game data or the
other parts by the scripts in `research/`, and written in when the document is rendered: `./build.sh research` runs
them, they write `research/results.json`, and `research/render.py` fills `docs/RESEARCH.template.md` to produce this
file. Each conclusion the text draws is also a test in those scripts (a likelihood-ratio test, an interval covering or
excluding a prediction), reported as holding or failing; a miss the text states is tested as a miss. The only numbers
not computed are the assumed inputs themselves. The run records are not in this repository; a clone renders from the
committed results and recomputes what needs only game data.

**Reading the fit comparisons.** A model's log-likelihood (LL) is the sum, over every observation, of the natural log
of the probability the model gave to what was actually seen; the higher (closer to 0), the better the model explains
the data. A difference of $\Delta$ in LL ("$\Delta$ nats") means the better model made the observed data $e^{\Delta}$
times as likely: 1 nat is a factor of 2.7, 5 nats about 150, 30 nats about $10^{13}$. A model with more parameters
always fits at least as well, so the gain has to be large enough to be more than chance. The rule used throughout is
the likelihood-ratio test: with $k$ extra parameters, $2\Delta$ is compared with a $\chi^2$ distribution on $k$
degrees of freedom, so one extra parameter needs $\Delta > 1.92$ to be significant at 5%, four need $\Delta > 4.74$.
A fit scored on the same runs it was fitted to (in-sample) overstates its gain; where it matters, the model is also
fitted on the earlier runs and scored on the later ones (held out).

**Targets.** The model is the same for every skill. Tests that need a fixed set of skills to score (mainly the
calibration of section 12) use the planner's default targets, eight skill pairs (white / gold): Uma Stan / Superstan,
Nimble Navigator / No Stopping Me!, Playtime's Over! / See Ya Later!, Ramp Up / It's On!, Tail Held High / Tail Nine,
Racing Spirit: Stamina, Slipstream / Perfect Spot!, Risky Business / Nothing Ventured (Unity Cup guarantees the No
Stopping Me! and It's On! golds). Section 12 also checks a sample of every other skill a deck card or spark could give.

## 0. What is computed

A career has a trainee $T$, two parents $P_1, P_2$ and the parents' own parents: six legacies $l = 1..6$. For targets
$k = 1..K$ the planner ranks careers, loops and decks by

$$P(\text{all}) = P(\text{every target lands as a spark on the veteran this career produces}).$$

A target lands if the trainee acquires the skill (hinted, so it can be bought), buys it, and it then generates a spark.
Sections 1-4 give the inheritance side, 5 the loop structure, 6-10 the deck and event side, 11 the composition, 12 its
test against real careers.

## 1. Spark generation (assumed)

$$g_k = g_0 \cdot 1.1^{\,n_k}, \qquad g_0 = 0.2 \text{ (white learned)}, \quad g_0 = 0.4 \text{ (gold learned)}$$

$n_k$ = ancestors (of six) carrying the spark of $k$. A bought gold sparks its white. In a running loop every target
spark sits on all six legacies (**assumed**: a loop keeps veterans that landed everything), so $n_k = 6$,
$1.1^6 = 1.771561$ and $g = 0.3543$ (white) / $0.7086$ (gold).

The curve is **assumed**, a community figure (README, sources), and not tested here: the run records show which
sparks a career inherited, not which sparks it produced, and the veterans they do show were chosen for their sparks.
That everything acquired is bought is also **assumed** and untested.

## 2. Star mix of legacy sparks

Computed by `research/s02_stars.py`.

A legacy's white spark has $s \in \{1, 2, 3\}$ stars with shares $\pi = (\pi_1, \pi_2, \pi_3)$ set by the veteran's
rank score: $0.50 / 0.45 / 0.05$ below 17,500 and $0.20 / 0.70 / 0.10$ from 17,500 up. The mixes are **assumed**
(community figures): the run records show only legacies their players chose, which are picked for their sparks, so the
mix of an unselected veteran cannot be read from them.

**Band per scenario** (**derived** from the runs). Each scenario takes the band most of its careers reach, by the
rank-score estimate with every skill point spent over every completed run in the run list:

| Scenario | Runs | Median rank score | Share at 17,500+ | Band |
|---|---|---|---|---|
| URA Finale | 1,357 | 12,761 | 0.000 [0.000, 0.003] | below |
| Unity Cup | 3,155 | 17,428 | 0.467 [0.450, 0.484] | below |
| Grand Concert | 14,742 | 18,094 | 0.794 [0.787, 0.800] | 17,500+ |
| Trackblazer | 282 | 12,386 | 0.000 [0.000, 0.013] | below |

Unity Cup is close to even. Grand Masters has no runs and is **assumed** 17,500+.

The loop model does not know a legacy's stars and uses the expected base per roll (**derived**, section 3's bases):

$$\bar b = 0.03\,(\pi_1 + 2\pi_2 + 3\pi_3) = 0.0465 \;(\text{below } 17{,}500), \quad 0.057 \;(17{,}500+).$$

That is exact for one roll but not for two: a legacy's stars are the same in both rolls, and by convexity
$E[(1 - r)^2] \ge (1 - E[r])^2$, so the loop model overstates $q_k$ (**derived**). On the six real affinities of each of
the 3,607 lineages in the runs, for a target carried by all six, it overstates $q$ by at most
0.57% (below 17,500; mean 0.50%) and 0.33% (17,500+). The career planner uses each
veteran's real stars and is not affected.

## 3. Inheritance rolls

Computed by `research/s03_inheritance.py` (the roll's parts) and `research/s12_calibration.py` (the test on whole
careers).

Per legacy $l$, target $k$ and roll:

$$r_{lk} = \min\!\left\{1,\; b(s_{lk})\left(1 + \frac{A_l}{100}\right)\right\}, \qquad
q_k = 1 - \left[\prod_{l=1}^{6} (1 - r_{lk})\right]^{2}$$

$q_k$ = P(at least one inspiration of $k$ in the career); two rolls (Classic and Senior April); $A_l$ is the legacy's
affinity (section 4); $b(s) = 0$ without a spark. Base by spark type and stars ($1/2/3\star$), **assumed** (community
figures); the skill-white bases are tested below:

| Spark | Skill / scenario white | Race white | Green (unique) | Pink | Blue |
|---|---|---|---|---|---|
| $b$ | 3 / 6 / 9% | 1 / 2 / 3% | 5 / 10 / 15% | 1 / 3 / 5% | 70 / 80 / 90% |

The odds use skill whites only. Great-grandparents count for nothing (**assumed**, matching the game's two-generation
lineage; the runs show no great-grandparents). In the loop model $b(s_{lk})$ is replaced by $\bar b$ (section 2).

**The roll's parts** (**fitted**). A trial is one (ancestor, white skill spark, year 2 or 3) where exactly one of the six
ancestors carries the skill, so that year's inspiration of it can only come from her; it passes when the factor shows
among the year's inspirations. Over 30,368 parent and 69,840 grandparent trials (ordinary ancestors; the
self-grandparent is section 4's):

- Grandparents take the full base: scaling their roll by a free weight fits 1.01 [0.99, 1.03], and the full
  base beats a halved one by 1,327 nats (6,308 passes observed, 6,268 predicted with the full base,
  3,134 halved). Parents read 0.982 [0.96, 1.01] of the model.
- Affinity scales the roll as $(1 + A/100)$: a free slope on $A$ fits 0.98 [0.96, 1.01], and the term beats no
  affinity by 3,760 nats. Its two parts apart, each with the other fixed: relation points 0.96
  [0.88, 1.05], $3 \times$ shared G1 wins 0.97 [0.94, 1.01] (99,502 trials with shared wins).
- The base by stars holds: observed / predicted 0.980 [0.94, 1.02] at 1★ (27,826 trials),
  0.995 [0.97, 1.01] at 2★ (63,864), 1.013 [0.97, 1.06] at 3★ (8,518).
- By affinity band the model holds jointly ($\chi^2$ = 10.3 on five bands, $p$ = 0.067), but two bands lie outside their
  own 95% interval:

| Affinity | Trials | Observed / predicted |
|---|---|---|
| 0-49 | 9,666 | 1.003 [0.93, 1.07] |
| 50-99 | 60,546 | 1.006 [0.98, 1.03] |
| 100-149 | 1,180 | 0.994 [0.84, 1.14] |
| 150-199 | 4,040 | 1.079 [1.01, 1.15] |
| 200+ | 24,776 | 0.970 [0.94, 0.99] |

**Test on whole careers** (section 12): observed / predicted inspiration on 3,348 real careers,

- all 26,784 run-target rows: 0.989 [0.97, 1.00];
- by the number of carriers, 1 to 6: 0.956-1.001, every interval covering 1;
- per roll: spark-years 0.988 (1.71 standard deviations below the model's expectation, at the edge of its
  95% band), ancestor hits ($2\sum_l r_l$) 0.989;
- 43,244 rows of non-target skills: 0.993 [0.97, 1.01].

By predicted chance the buckets are calibrated jointly ($\chi^2$ = 5.7 on 5 buckets, $p$ = 0.33); one lies
outside its own 95% interval, 0.40-0.60
(0.489 [0.474, 0.505] against 0.505):

| Predicted $q$ | Rows | Observed (mean predicted) |
|---|---|---|
| 0 | 8,723 | 0.002 |
| above 0 to 0.20 | 2,490 | 0.156 (0.16) |
| 0.20-0.40 | 3,163 | 0.298 (0.31) |
| 0.40-0.60 | 4,098 | 0.489 (0.51) |
| 0.60-0.80 | 5,786 | 0.698 (0.70) |
| 0.80-0.95 | 2,524 | 0.834 (0.84) |

## 4. Affinity

Computed by `research/s04_affinity.py` (the self-grandparent fit) and `research/s04_races.py` (the win table).

**Relation points** (exact, game data): $\mathrm{pair}(a,b)$ and $\mathrm{tri}(a,b,c)$ sum the relation points of the
relation groups containing all members. A group counts only distinct characters, so $\mathrm{tri}(T, P, T) = 0$.

**Race affinity**: +3 per single-race G1 both won, counted per race (relocated venues are the same race), multi-race
titles 0 (**assumed** as the game rule; the weight 3 per shared win is tested in section 3, slope
0.97 [0.94, 1.01]). With schedules fixed in advance and two careers independent (**assumed**), expected shared wins are

$$o(a,b) = \sum_r P_a(r)\,P_b(r), \qquad P_a(r) = 1 - \prod_{\text{attempts}} (1 - p),$$
$$p = \mathrm{clamp}\big(110 - c_{\text{dist}} - c_{\text{surf}} - \mathrm{PEN}_j,\ 0,\ 100\big) / 100,$$

$c$ = 0 for S/A, 10 B, 20 C, 30 D, 40 E, 60 F, 90 G; $\mathrm{PEN}_j$ = 0, 0, 5, 20, 30, 50 for the $j$-th race in a
row (any race, goals included; a rest resets). The win table is a community table, tested below. Stats and training
turns are not modelled.

**The win table, tested** (**fitted**; `research/s04_races.py`). The runs record every race a career ran. Over
76,402 attempts at the 34 single-race G1s (22,387 of them goals), 93.1% were won:

- On the trainee's base aptitudes, which is what the loop model uses, the table predicts 86.3%: observed / predicted
  1.079 [1.077, 1.081], and 1.83 where the table gives 70 or less (14,047 attempts): aptitude
  sparks raise her grades during the career. The loop model does not know its veterans' pink sparks and keeps base
  aptitudes (a simplification); the career planner, which plans with real veterans, raises them by the rule below.
- With her grades raised by the career planner's aptitude-spark rule (`docs/MODEL.md`: the legacies' pink stars at the
  start, then each April inspiration the run records), it predicts 93.1%: observed / predicted
  1.000 [0.998, 1.002], 2,119 nats better than base aptitudes, and calibrated in every scenario.
- The streak penalty is real: without it the fit is 1,737 nats worse.

| Table's chance (raised grades) | Attempts | Observed / predicted |
|---|---|---|
| 110 | 50,681 | 1.002 [1.001, 1.002] |
| 100 | 11,102 | 1.004 [1.001, 1.007] |
| 90 | 4,674 | 1.002 [0.992, 1.012] |
| 80 | 3,169 | 1.002 [0.983, 1.020] |
| 70 or less | 6,776 | 0.959 [0.936, 0.983] |

The table is not exact: a free win rate per (chance, streak position) cell (63 cells) beats it by
69 nats, more than chance allows (40). The excess sits in goal races from the fourth race in a row, which win more
than the penalty allows (1.13 [1.11, 1.15], 1,339 attempts), while chosen races there match
(0.99 [0.97, 1.01]); and low chances win a little less than predicted. The test is conditional on IT
having chosen each non-goal race; stats are not modelled.

**Per-legacy affinity.** For parent $P$ with other parent $P'$ and own parents $\mathrm{par}(P)$, and grandparent
$G \in \mathrm{par}(P)$:

$$A_P = \mathrm{pair}(T,P) + \mathrm{pair}(P_1,P_2) + 3\,o(P,P') + \sum_{G \in \mathrm{par}(P)} \big[\mathrm{tri}(T,P,G) + 3\,o(P,G)\big]$$
$$A_G = \mathrm{tri}(T,P,G) + 3\,o(P,G)$$

In a loop $P'$ is also one of $P$'s own parents, so $o(P, P')$ counts twice in $A_P$. Tested in section 3 on the runs'
real lineages and shared wins: the roll scales with this $A$ at slope 0.98 [0.96, 1.01].

**Self-grandparent** ($G = T$, section 5). $\mathrm{tri}(T,P,T) = 0$ is **fitted** on 114,240 inspiration trials
attributable to a single ancestor in runs: self-grandparent passes 610 observed,
621.6 predicted with 0, 721.2 with $\mathrm{tri}(T,P,T) = \mathrm{pair}(T,P)$ (about 19 nats worse); the family
$\kappa\cdot\mathrm{pair}(T,P)$ fits $\hat\kappa = 0$, 95% upper limit about 0.21.

## 5. Loop structures, ranking and schedules

Computed by `research/s05_loops.py`.

A lineage is $(T, P_1, P_2, X)$: each parent bred from the other parent and $X$. A four-uma loop has 12 lineages (each
member trains from two of the other three); a three-uma rotation has 3, with $X = T$ (the trainee's own earlier
veteran). The rotation occurs in practice: 1087 of 3,610 runs have a grandparent of
the trainee's own character (0 have a parent of her character). A structure $S$ with lineage set $\Lambda_S$ scores

$$\bar P_S = \frac{1}{|\Lambda_S|} \sum_{\lambda \in \Lambda_S} P_\lambda(\text{all}),$$

the long-run rate of a player who trains every member in turn (a definition of what the planner ranks, not a claim: a
player who re-trains only the best trainee would score a structure by its best lineage).

**Race weights** (**derived**). In one lineage $o(P_1,P_2)$ enters the six affinities 6 times and $o(P_1,X)$,
$o(P_2,X)$ twice each. Summed over the 12 loop lineages every pair gets $6\cdot 2 + 2\cdot 4 = 20$; over the 3
rotation lineages $6 + 2 + 2 = 10$. Both are uniform over pairs, so a structure's schedules maximise
$\sum_{a<b} o(a,b)$.

**Schedules for shared wins, not odds** (**derived**, computed on the validation results). The solver schedules each
structure for $\sum_{a<b} o(a,b)$, not for $\bar P_S$. $\bar P_S$ rises with every $o$: $o$ enters each legacy's
affinity with weight +3, and $r$, $q$ and every target's odds are nondecreasing in their inputs. So, with a loop's decks
fixed, any schedule (one chosen for $\bar P_S$ directly included) scores between the odds found and the odds with every
pair at its highest possible overlap. Over 40 validation results, recomputed independently to within
2e-15 of the solver:

- the shared wins found are worth 31-85% over no race affinity;
- the independent ceiling $o_{\max}(a,b) = \sum_r P^{\max}_a(r)\,P^{\max}_b(r)$ (every offering of every G1 attempted, no
  streak penalty) caps the gain from a different objective at 17.5%; it is far from reachable (the schedules
  found reach about 68% of it) and cannot settle the ranking: 8 of 12 listed runner-up loops reach their
  winner under it;
- the solver's exact pair ceilings (its best-response search over feasible schedules, so not independent; they sit
  between $o$ and $o_{\max}$ on every pair) put the ceiling only 0.54-2.02% above the odds found (median
  0.90%), and no listed runner-up's ceiling reaches its winner (0 of 12).

## 6. Training hints

Computed by `research/s06_hints.py`.

Each deck card makes $h$ training-hint draws per run, each uniform over its skill pool (size $n$; the stat option is
never drawn) among skills still below hint Lv 5; a skill leaves after $c = \lceil 5 / \iota \rceil$ draws, $\iota$ =
hint levels per draw ($c$ = 5, 3, 2, 2, 1 for +1 ... +5 cards).

$$h = \beta_{\text{scenario}}\,\left(1 + \frac{hf}{100}\right)\left(1 + d_{\text{scenario}}\,\frac{\mathit{sp}}{100}\right)$$

$hf$ = hint frequency and $\mathit{sp}$ = specialty priority at maximum limit break, unique effects included. For $k$
targets in one card's pool the miss chance is joint:

$$m(k, n, t) = \left(1 - \frac{k}{n}\right)^{t} \quad (t \le c), \qquad \text{exact Markov chain over the other skills' draw counts otherwise},$$
$$M(k, n, h) = (1 - f)\, m(k, n, \lfloor h \rfloor) + f\, m(k, n, \lfloor h \rfloor + 1), \qquad f = h - \lfloor h \rfloor.$$

The shape (uniform draws, $h$ per run, the cap) is **derived** from the data; $\beta$ and $d$ are **fitted**. Fit:
Bernoulli likelihood through $M$ per (card-run, pool skill) row, maximum-limit-break card-runs only, rows where no spark
hinted the skill, R cards out. The form study uses every card-run at any limit break, R cards included, scored without the specialty term
(93,928 rows from 3,610 runs, 84,418 without a spark); $\beta$ and $d$ are fitted (61,856 rows, 10,712 card-runs; the Runs column counts the runs that add a row):

| Scenario | $\beta$ | $d$ | Adopted $\beta$ / $d$ | Rows | Runs |
|---|---|---|---|---|---|
| Unity Cup | 2.377 | 0.349 | 2.36 / 0.36 | 14,047 | 915 |
| Grand Concert | 1.962 | 0.100 | 1.97 / 0.08 | 34,481 | 1,773 |
| URA Finale | 1.369 | 0.366 | 1.34 / 0.39 | 10,958 | 631 |
| Trackblazer | 1.700 | 0.108 | 1.70 / 0.11 | 2,370 | 99 |
| Grand Masters | no runs | | 1.86 / 0.26 (**assumed**: one $\beta$ and one $d$ over every scenario) | | |

What fixed the form (the form study):

| Question | Evidence |
|---|---|
| Rate form, LL on 93,928 rows | $1 + hf/25$ over options −61,672.2; $1 + hf/25$ over skills −60,105.9; constant −58,710.8; $\beta(1 + hf/100)$ −58,098.4; per-scenario $\beta(1 + hf/100)$ −57,642.8; free divisor ($c = 79$) −57,624.8 |
| Skills only, not skills + stat option | in 54 of 77 (card, $hf$) cells the draw-count variance is below the binomial floor a drawn stat option implies |
| Uniform draws with repeats | repeat rate $P(k \ge 2 \mid k \ge 1)$: 0.143 observed, 0.140 two-point count, 0.192 Poisson count; distinct skills 1/2/3/4 per card-run 0.152 / 0.555 / 0.253 / 0.038 observed, 0.126 / 0.554 / 0.274 / 0.044 model |
| Covariates | per-scenario bases +456 nats (form study). On top of the adopted rate (61,856 rows): card type +0.8 nats (4 parameters), initial bond +0.0, pool size +1.8, none significant; rarity (SR against SSR) +5.3 and pals per deck +14.0 are (known misses, factors 0.954 for SR and 1.033 per pal) |
| Lv-5 cap | beats drawing with replacement by 20.2 nats with no parameter; obs / pred by draws to Lv 5, replacement vs cap: 5 draws (31,144 rows) 0.963 / 0.972, 3 (2,711) 0.955 / 0.962, 2 (32,818) 1.035 / 1.033, 1 (409) 1.223 / 0.970 |
| Joint draws over targets | 176,927 pairs of no-spark pool skills on one card-run (Trackblazer left out): $P(\text{both})$ 0.0798 observed, 0.0794 joint, 0.1090 independent; $P(\text{neither})$ 0.4440 vs 0.4419 |
| Pool size | obs / pred 0.92-1.15 for every $n$ with at least 150 rows |
| Per-draw increment | 24,815 of 24,935 single-source levels are multiples of $\iota$, capped at 5 |
| $hf$ by card level | floored linear interpolation −52,233.7, step −52,240.7, max-limit-break $hf$ for all −52,255.1 |

The slope $1/100$ is **assumed**: a free divisor fits better ($c = 79$ in the table above) and is not adopted.

Reading $h$ as a floor/ceiling mixture against a two-point count differs by under 1% ($n = 5$, $h = 2.74$: miss
0.5426 vs 0.5453). $hf$ spans 0-70 in the data.

**Spark deficit** (not modelled; the fit conditions on no spark, the case in which a card's hint matters). Scored
against the adopted rate on maximum-limit-break card-runs, R cards out:

| Other source of the skill | Rows | Observed | Predicted | Ratio |
|---|---|---|---|---|
| none | 54,415 | 0.342 | 0.338 | 1.01 |
| inspiration | 6,194 | 0.265 | 0.352 | 0.75 |
| inspiration + card event | 316 | 0.196 | 0.343 | 0.57 |
| card event | 1,597 | 0.326 | 0.334 | 0.98 |
| other card's training | 2,836 | 0.343 | 0.359 | 0.96 |

Full drop-out of spark-hinted skills from the draws is rejected (with two pool skills spark-hinted, the rest are hinted
at 0.362 against 0.332 without drop-out and 0.450 with it).

**Specialty priority** (effect 19). On the fit's setup above, the rate without the specialty term leaves a residual
that rises with specialty: observed / predicted 0.951 at specialty 0 (18,870 rows), 0.965 at 1-29 (583), 1.005 at
30-59 (30,188), 1.067 at 60+ (12,215). The term gains 37.8 nats for 4 parameters (one $d$ for every scenario: 28.2
nats, $d = 0.193$; an exponential form $e^{g_s\,\mathit{sp}/10}$ fits the same, 37.9 nats). The bands then read 1.003 /
0.991 / 0.995 / 1.005, no (scenario, band) cell with 200+ rows is off by more than $|z| = 1.16$, and Unity Cup cards
with $hf = 0$ and specialty 0 go from 0.883 to 0.971. One pooled $d$ stays within 0.16-0.28 on either half of the runs,
on SR or SSR cards alone and without any one of the eight most-used cards. Fitted on the earlier runs and scored on
the later 32,675 rows, it gains 12.9 nats: it predicts runs it was not fitted to. Against real careers
(section 12) training-pool hints read 0.988 of the model with it.

The term is an association, not shown to be a cause: high-specialty cards might share another trait that raises
hints. The cleanest test compares the same card at different limit breaks, where specialty changes and the card does
not (50 cards, a base per card and scenario): $d$ = 0.41, the same size as across cards, but a gain of only
1.5 nats, not significant.

Prioritised skills raising training hints: 532 runs show a priority list, prioritised skills hinted by training at 0.343
against 0.318 predicted (470 rows); no effect assumed.

## 7. Chain events

Computed by `research/s07_chains.py`.

A run gets $N$ chain events, the same for every deck and scenario (**fitted**; a binomial with another $n$ fits better,
below):

$$N \sim \mathrm{Binomial}(21, 0.35), \qquad P(N = e) = \binom{21}{e}\, 0.35^{e}\, 0.65^{21-e}, \quad E N = 7.35,\ \mathrm{Var}\, N = 4.8.$$

Each event advances one non-pal chain card by one link (links fire in order). Card $i$ with chain length $L_i$ (SSR 3,
SR 2; a filler slot counts as a 2-link SR) and $d_i$ links fired is picked with

$$P(i) = \frac{w_i}{\sum_j w_j}, \qquad w_i = L_i - d_i \text{ while running, } 0 \text{ once stopped},$$

i.e. uniform over the remaining chain slots (**fitted** against the alternative, below). Pal and group cards take no slot
(**assumed**, supported below). Without stop risk $P(\text{all chains complete}) = P(N \ge \sum_i L_i)$; two 3-link
cards: $P(N \ge 6) = 0.80$.

**Stop risk** (**derived** from the event data and the choice rule, section 8): if IT's default choice at link $l$ ends
the chain on $k$ of its $n$ outcomes, $q_l = k/n$ and the stopping link $\sigma$ has
$P(\sigma = l) = q_l \prod_{l' < l} (1 - q_{l'})$; a stopped chain's slots return to the pool. The solver enumerates the
exact distribution of final chain states (no simulation); an independent recomputation of the solver's odds agrees to
2e-15 on every validated loop (section 5).

**Fit of $N$** (3,424 runs, the pal-count table below). Chain progress per card is read from marker skills: a link is
observable only if every choice and outcome of its event carries a skill appearing in no other event of that card. A
credit naming two sources ("A or B event") is unknown when B is another deck card whose events can give the marker, and
so is a marker listed only under inspiration. $N$'s distribution is the one that best explains progress over all decks.

The fit allows a fired link's marker to go unseen with chance 0.03 (**assumed**, a measurement allowance). Without
it, a run in which a card's later link shows while an earlier one does not has likelihood 0. Since links fire in
order, those runs measure the rate: of 1149 earlier links implied fired by a later one, 24 show no marker, 0.021
[0.014, 0.031]. The cause is not known. The fit does not depend on it: with allowances from 0.01 to 0.045, or with
none on the 3,402 runs that show no such case, the best $p$ at $n = 21$ is 0.34-0.350.

| Form for $N$ | Result |
|---|---|
| distribution-free | mean 7.28, variance 4.47 |
| Binomial(21, 0.35) | 11.4 nats behind distribution-free with 14 fewer parameters (not rejected, $p$ = 0.06) |
| Binomial(18, 0.405) | the best binomial with $n$ free: 8.0 nats behind distribution-free ($p$ = 0.31), 3.4 better than (21, 0.35) |
| Conway-Maxwell-Poisson | another under-dispersed two-parameter family: 10.1 nats behind distribution-free ($p$ = 0.12) |
| Binomial(15, 0.48) | 15.9 nats behind distribution-free |
| Poisson(7.1) | 87.8 nats behind distribution-free (variance = mean, too wide; rejected) |
| per scenario | means Unity Cup 7.57, Grand Concert 7.21, URA Finale 7.06, Trackblazer 7.03; +27.7 nats for 48 extra parameters (not significant) |

Why a binomial: the free fit's variance is 0.61 of its mean, so $N$ is under-dispersed, which rules out
the Poisson and every over-dispersed count. A binomial (a fixed number of chances, each taken with the same
probability) is the simplest under-dispersed count, but the data do not single it out: the Conway-Maxwell-Poisson fits
about as well. What the model uses is the distribution of $N$, not the family behind it.
The adopted (21, 0.35) is not rejected, but Binomial(18, 0.405) fits better by more than chance allows for its one
extra parameter (a known miss; the adopted form was chosen on fewer runs). Fitted per pal count, the means are
7.58 / 7.12 / 7.41 / 8.40 for 1 / 2 / 3 / 4 pals (2,918 runs): more pals do not lower $N$, so pal events do not consume chain events.

Final-link completion by pal count; the binomial lies inside every interval except at 2 pals, where it predicts more
completions than seen:

| Pals | Chain cards | Runs | Final link reached: observed / Binomial(21, 0.35) / Poisson(7.1) |
|---|---|---|---|
| 5 | 1 | 120 | 0.98 / 0.98 / 0.96 |
| 4 | 2 | 357 | 0.88 / 0.85 / 0.79 |
| 3 | 3 | 872 | 0.60 / 0.59 / 0.54 |
| 2 | 4 | 903 | 0.32 / 0.35 / 0.33 |
| 1 | 5 | 786 | 0.24 / 0.23 / 0.22 |
| 0 | 6 | 386 | 0.15 / 0.16 / 0.16 |

**Allocation rule.** Against uniform over cards (each running card's next event equally likely), on 3,424 runs:

| | Uniform over remaining slots (adopted) | Uniform over cards |
|---|---|---|
| Total LL | −7,318.4 | −7,384.9 |
| Final links, 2 pals (observed 0.32) | 0.35 | 0.41 |
| Final links, 0 pals (observed 0.15) | 0.16 | 0.23 |

The slot rule fits better by 66.4 nats. With no pals it lies inside the observed interval and the card rule
outside; with two pals both predict more final links than seen, the card rule further. Exact consequences with Binomial(21, 0.35), P(reach link 1 / 2 / 3):

| Deck | Slots (adopted) | Cards |
|---|---|---|
| two 3-link SSRs | 1.00 / 0.97 / 0.87 | 0.99 / 0.96 / 0.87 |
| 3-link SSR + 2-link SR | SSR 1.00 / 0.99 / 0.93, SR 0.99 / 0.95 | SSR 1.00 / 0.98 / 0.93, SR 0.99 / 0.96 |
| three 3-link SSRs | 0.98 / 0.84 / 0.54 | 0.96 / 0.81 / 0.58 |
| four 3-link SSRs | 0.93 / 0.65 / 0.25 | 0.88 / 0.62 / 0.33 |

(With two 3-link cards and exactly 3 events, the slot rule gives $(3,0)$ with $\tfrac36 \cdot \tfrac25 \cdot \tfrac14 = 5\%$,
the card rule $(\tfrac12)^3 = 12.5\%$.)

**Known misses.** Two-chain decks (two 3-link SSRs, four pals) finish both chains in 163 of 196 runs, 0.83
[0.77, 0.88], against 0.80 predicted: within the interval. Early links in crowded decks run short; per run as a cluster, $z = \sum(\text{obs} - \text{pred}) / \sqrt{\sum(\text{obs} - \text{pred})^2}$:

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Early-link shortfall | 0.01 ± 0.02 | 0.04 ± 0.01 | 0.07 ± 0.02 | 0.05 ± 0.02 | 0.14 ± 0.02 |
| $z$ | 0.8 | 3.4 | 4.7 | 2.9 | 5.6 |

Final links fit when pooled (0.352 observed against 0.357) but individual cards differ
from the model far beyond chance: over the 42 cards with 60 or more final-link observations the squared
standardised differences sum to 263 (expected about 42). The cause is not known.

More chains never start in crowded decks than predicted: 0.20 against 0.13 with five chain cards, 0.34 against 0.20 with
six. A per-card activation parameter (a chain card active with probability $a$), fitted jointly with $N$, gains
1.1 nats ($a = 0.97$): rejected.

## 8. Event choices and outcomes

Computed by `research/s08_outcomes.py` (the certainty checks: `research/s12_calibration.py`).

**Choice rule** (**assumed**): IT takes an event's top choice unless a prioritised skill (up to 10, set at career start)
sits on another choice, and the player is assumed to prioritise every target hint the model counts. Where targets sit
on different choices, a deck entry takes one choice (the solver offers one entry per useful choice, and an entry that
ends a chain early).

Evidence on the rule. Sasami Anshinzawa [This Might Sting!]'s finale: with Risky Business prioritised IT took choice 2
in about 95% of runs (3 of the 4 exceptions with Risky Business already at hint Lv 5); with neither skill prioritised, 0 of
35 runs took choice 2. Riko Kashimoto [Planned Perfection]: with Lone Wolf prioritised IT took the bottom choice
in 55 of 62 runs. Every run has a priority list, but the run records show it for only 532 of 3,610 (the rest count as
unknown, never as an empty list), so the rule is tested mainly through its consequences (section 12: prioritised targets
1.01, non-prioritised 0.71 on event-only hints).

**Outcomes** (**assumed** unless measured): one choice's outcomes exclude each other, one per firing, equally likely
unless measured (outcome odds are server-side: master.mdb holds none); a card's gold and white of one skill are
combined exactly. Measured shares (90% intervals; counted only where every outcome could show:
her hint credited, the skill's level from other sources below 5):

| Event, outcome | Observed | Adopted |
|---|---|---|
| Sasami Anshinzawa [This Might Sting!] finale, Nothing Ventured (Risky Business prioritised; 254 runs with the card) | 58/77 = 0.75 [0.66, 0.82] | 0.75 |
| same, choice 2 alone, no inherited Risky Business | 37/45 = 0.82 [0.71, 0.90] | |
| Riko Kashimoto [Planned Perfection] "Unexpected Side", Ramp Up (top choice taken) | 527/585 = 0.90 [0.88, 0.92] | 0.75 |
| Seiun Sky [Foolproof Plan] link 3, Fast-Paced | 33/40 = 0.82 [0.71, 0.90] | 0.75 |
| Smart Falcon [My Umadol Way! ☆] link 3, Center Stage | 37/51 = 0.73 [0.61, 0.81] | 0.75 |
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante", Nothing Ventured (603 runs with the card, below) | 138/228 = 0.61 [0.55, 0.66] | 0.6 |

Anshinzawa's finale is counted on runs without Nakayama Festa [Trusting the Dice] (whose "Upping the Ante" also gives
Nothing Ventured +1/+3, credited jointly). Hint levels identify the choice: her Nothing Ventured +3 = choice 1, +1 =
choice 2; a loss on choice 2 shows as her Risky Business at level 2 (storyline choice 3 +1, finale +1). With inherited
Risky Business levels mixed in, choice 2 reads 0.77-0.87. Choice 1 (Nothing Ventured +3 or nothing): 69/127 = 0.54 if
the finale always fires; Grand Concert lower (unexplained). With Risky Business prioritised IT also takes the bottom
choice of her date-start event "A Pleasant Practice Partner?" (Risky Business +1; GameTora marks it 'ee', which outside
IT declines her dates), yet the finale still fires (at least 75 of 77 such runs show its outcome; her extra Risky
Business +1 appears in 53 of 54 choice-2 wins with the priority, 1 of 20 without): IT apparently keeps her dates either
way. The other outcomes: Riko Kashimoto's alternative is Maverick ○, Seiun Sky's Escape Artist, Smart Falcon's Prudent
Positioning.

Riko Kashimoto's 0.75 is a per-run chance: a run gets her Ramp Up hint in 521 of 689 runs, 0.756 [0.728, 0.782], whichever
choice IT took (the top choice in 579 of 725 firings, 0.80; the bottom choice, Lone Wolf, in 146).

Off 0.5 but left at the equal rule: Tazuna Hayakawa SSR finale Concentration 0.70, Agnes Tachyon [Q≠0] link 1 choice 2
Murmur 0.70, Marvelous Sunday [Dazzling Day in the Snow] link 3 Position Pilfer 0.58, Light Hello SSR finale +3 See Ya
Later! 1,497 of 2,103 (0.71). Consistent with 0.5: Gold Ship [That Time I Became the Strongest] 0.61, King Halo
[Tonight, We Waltz] 0.57, Daitaku Helios [Sunlit Angel ♪] Tail Held High 0.47 (link 2, on runs where link 3 visibly fired).

Nakayama Festa's share is measured on every run with her card (603). Her event is credited
to her alone with Nothing Ventured in 138 runs and with Risky Business in 68. In 40 more, Risky Business is credited to
"her or another card's event"; the other card gives it alone in 0.895 of runs without her (Team Sirius, 30 of the 40)
or 0.390 (Sasami Anshinzawa), she in 0.119 of hers, so a shared credit is hers with chance
$r_F / (1 - (1 - r_F)(1 - r_o))$: 6.5 of the 40. In 39 runs neither shows while Risky Business is already at hint Lv 5
(a Risky Business win cannot show, or the event never fired); they are split by the rate at which the event shows where
both outcomes can, 0.39. The share is then 138/228 = 0.61 [0.55, 0.66]; it stays within 0.523-0.625 whatever share of
the shared credits is hers, and shows no dependence on the trainee's final Wit.

**Pal and group cards.** Their events are entered per card with a probability (finales certain); first-year date
events are left out in IT (**assumed**). Certainty checks: Light Hello [From the Ground Up] credits See Ya Later! to her
event in 972 of 972 runs carrying her; Team Sirius gives Risky Business in 64 of 64.

## 9. Random card events (not modelled)

Computed by `research/s09_random_events.py`.

A support card's character's shared random event fires, per event per career with a card of hers in the deck (any
rarity, R cards included) and another uma training, with (**fitted**)

$$P(\text{fires}) = 0.18\,(1 - 0.036)^{K - 4},$$

$K$ = random events on offer (every shared event of the deck's characters plus the trainee's own): about 0.14 overall,
0.18 at $K = 4$, 0.13 at $K = 14$. Readings: top-choice marker skills credited to the character's event; 5,596
readings from 2,884 runs, 786 clean hits (102 ambiguous two-card credits counted as
misses; with them the rate is about 2 points higher). A reading misses a firing when the marker was already at hint Lv 5
from other sources (106 readings): where the event could show, the rate is 0.143, or 0.162 counting the ambiguous
credits as hits. A firing is also missed when IT takes another choice for a prioritised skill (bottom-choice rates
below). The fitted rate is therefore a lower bound.

| Form | LL |
|---|---|
| constant 0.140 | −2270.8 |
| $0.18(1 - 0.036)^{K-4}$ | −2266.9 |
| power law $c(K/8)^{-a}$, $a = 0.34$ (uniform competition would be $a = 1$) | −2266.8 |
| strict competition $1 - (1 - 1/K)^S$, $S = 1.49$ | −2283.4 |
| every event slot competing, $c / T$ with $T = K + w \cdot \text{pals}$ (chain links weigh 0), $c = 1.77$ | −2266.8 |

The slot form fits about as well as the adopted one (+4.0 nats over constant for two extra parameters): chain links play
no part, and pals compete, a pal weighing $w$ = 1.09 [0.63, 1.69] random events.

Observed hit rate by $K$ = 4 / 6 / 8 / 10 / 12 / 14: 0.17 / 0.19 / 0.17 / 0.13 / 0.14 / 0.13; within a run events fire
independently. A logistic slope in $K$ gives 15.7% at 8 on offer and 12.6% at 14; friendship, specialty priority and
scenario show no effect distinguishable from zero, nor does rarity within a character (pooled, SSR markers fire at
15.3% against SR 11.2%, mostly which characters have SR cards).

Bottom-choice skills are rarely credited: in the 532 runs showing a priority list, 22 of 925 candidates were
prioritised; without a priority they were credited in 0.035 [0.025, 0.050] of runs, against 0.140 for
top-choice markers (3 of 22 with a priority).

**What leaving them out costs on real careers** (section 12's rows, each run's own deck; this test scores whether the
target was acquired from any source, so it does not depend on reading the event). Of the default targets only
two are a character's top-choice marker (Vodka's Nimble Navigator, Nice Nature's Ramp Up). On the 286 run-target rows
where a deck card's random event could hint the target, it was credited to that event 32 times, 21 with no other
source; 72 rows get no chance from the model, and the event hinted the target on 6 of them. Left out,
acquisition there reads 1.26 [1.10, 1.42] of the model, an under-prediction; credited at the fitted rate
(about 0.14 there) it reads 1.00 [0.87, 1.13] and the fit improves by 4.8 nats
(4.8 with the rate fitted without these rows' readings). Crediting raises an affected run's odds over the targets it
can land by a median factor 0.16 (at most 1.59; 214 runs). The trainee's own random event can be told
apart only for Nice Nature: 0 of 6 rows. The model leaves these events out; the data say it under-predicts
the two affected targets on decks with those characters.

## 10. Conditional character events (not modelled)

Computed by `research/s10_conditional.py`.

Secret and race-result events: 168 events with hints for 81 umas (110 for 55 umas in Global), 11 touching the default
targets (Nimble Navigator, Ramp Up, Tail Held High), 0 of them giving a gold.

**On real careers.** Where only a conditional event can give the target (none of the trainee's story, date, outfit or
shared random events, nor the scenario's, can), it did so on 4 of 56 rows, 0.071 [0.028, 0.170]:
Daiwa Scarlet, any outfit, 0 of 52 (Nimble Navigator); Mejiro Ardan, any outfit, 4 of 4 (Ramp Up). Left
out, acquisition there reads 1.20 [0.81, 1.66] of the model, and crediting the measured rates gains only
0.2 nats. The exception is Vodka, any outfit, whose own events give her Nimble Navigator in
25 of 27 runs ([0.77, 0.98]; her random and conditional events both give it and cannot be told
apart): with her rows, the model left as it is reads 1.45 [1.17, 1.72] and crediting gains
15.9 nats. Not modelled, so the model under-predicts Vodka's Nimble Navigator.

One exception is modelled as always acquired: Tokai Teio's Head-On (all three of its races are her career goals; the
goals ask only for top 5, top 5 and top 3, but IT won all three in 62 of 62 runs training her, and every run
where her event could show credits Head-On), like Yamanin Zephyr's Slipstream.

## 11. The composed career model

The composition is **derived** from sections 1-10; there is nothing to fit. How the solver searches it (pruning caps,
the career planner's continuation) is search design, in `docs/SOLVER.md`; `val/joint_check.py` recomputes the solver's
exact odds independently.

For a target $k$ not guaranteed, with $G_k = 0.4\cdot 1.1^{n_k}$, $W_k = 0.2\cdot 1.1^{n_k}$:

$$p_k = d^{g}_k\, G_k + (1 - d^{g}_k)\, W_k \left[1 - (1 - d^{w}_k)(1 - e_k)(1 - q_k)\right]$$

$d^g_k$ = P(the deck hints the gold), $d^w_k$ = P(the deck hints the white), $e_k$ = P(the trainee's own events hint
it), $q_k$ from section 3. A skill on the trainee's outfit replaces the bracket by 1 (white) or gives $p_k = G_k$
(gold); a scenario-guaranteed gold gives $G_k$. Inheritance only ever supplies the white.

Targets are not independent: chain cards share one run's $N$, and one event's outcomes exclude each other. With $c$
the final chain state (and stopping links), sources are independent given $c$ except one card's training draws, which
are joint over its targets (section 6):

$$P(\text{all}) = \sum_{c} P(c) \prod_{k} p_k(c) \qquad (\text{exact, **derived**}).$$

**Career planner.** The same model with each veteran's real stars, real G1 wins and recorded parents (an unknown
parent brings no relation points; her sparks count); $n_k$ = legacies actually carrying $k$. Since $q_k$ and $e_k$
enter only as $(1 - e_k)(1 - q_k)$, $e_k$ is replaced by $1 - (1 - e_k)(1 - q_k)$ and $1.1^{n_k - 6}$ scales
$P(\text{all})$ (**derived**).

## 12. Calibration against real careers

Computed by `research/s12_calibration.py`.

Each of 3,348 runs (URA Finale 645, Unity Cup 853, Grand Concert 1,768, Trackblazer
82) was rebuilt as a career: trainee in her outfit, scenario, the six cards at their own limit breaks (hint frequency
and increment at that limit break), both parents with their recorded parents and every ancestor's white sparks, and
race wins giving the recorded shared-G1 counts. Targets: the default eight; 26,784 run-target rows, of which 1,159 on
the trainee's outfit and 1,706 scenario golds sit outside tests 1 and 3. Intervals: Wilson 95% on the observed rate;
ratio intervals are that interval over the mean prediction. The runs end before skills are bought, so "acquired"
means hinted and buyable, the model's own meaning.

| Test | Predicted | Observed when |
|---|---|---|
| 1. Deck hint | $d^g + (1 - d^g)d^w$ | a deck card is credited with a hint of the skill; rows with no inspiration of it |
| 2. Inheritance | $q$ | a factor of the skill fired in Year 2 or 3 |
| 3. Acquired | from every modelled source | the skill was hinted from any source or owned |
| 4. Landing | the solver's $p$ | $0.4\cdot 1.1^n$ if a gold was hinted, $0.2\cdot 1.1^n$ if only the white |

| Test | Rows | Predicted | Observed [95%] | Obs / pred [95%] |
|---|---|---|---|---|
| 1. Deck hint, no inspiration | 15,774 | 0.260 | 0.244 [0.238, 0.251] | 0.937 [0.91, 0.96] |
| 1. Training pool only | 3,601 | 0.355 | 0.351 | 0.988 [0.94, 1.03] |
| 1. Events only | 2,082 | 0.884 | 0.804 | 0.909 [0.89, 0.93] |
| 1. Events only, target prioritised | 186 | 0.864 | 0.871 | 1.007 [0.94, 1.05] |
| 1. Events only, not prioritised | 234 | 0.860 | 0.611 | 0.711 [0.64, 0.78] |
| 2. Inheritance | 26,784 | 0.358 | 0.354 [0.349, 0.360] | 0.989 [0.97, 1.00] |
| 3. Acquired, any source | 23,919 | 0.514 | 0.511 [0.505, 0.517] | 0.994 [0.98, 1.01] |
| 3. Acquired, deck or inspiration only | 23,919 | 0.514 | 0.502 | 0.976 [0.96, 0.99] |
| 3. Acquired, prioritised | 1,587 | 0.727 | 0.747 | 1.028 [1.00, 1.06] |
| 3. Acquired, not prioritised | 1,888 | 0.556 | 0.519 | 0.932 [0.89, 0.97] |
| 4. Landing (sums of $p$) | 23,919 | 4,358.0 | 4,313.2 | 0.990 |

The prioritised / not prioritised rows use the runs that show a priority list.

**Verdict.** The composed acquisition odds are calibrated (0.994). Inheritance is right (one target of eight, Tail Held High, reads 0.946 [0.90, 0.99]). Deck hints read optimistic (0.937 of the model), almost all from the
priority assumption: event hints land at 1.01 × predicted for prioritised targets and 0.71 for others. Unmodelled scenario and
trainee events supply the few points the deck loses (220 target acquisitions in 23,919 rows, 0.9%).

Per target (obs / pred):

| Target | Deck hint | Inheritance | Acquired | Landing |
|---|---|---|---|---|
| Uma Stan / Superstan | 0.80 [0.68, 0.94] | 1.001 | 0.989 | 0.991 |
| Nimble Navigator / No Stopping Me! | 0.99 [0.88, 1.11] | 0.979 | 1.055 | 1.035 |
| Playtime's Over! / See Ya Later! | 0.976 [0.95, 1.00] | 0.998 | 0.986 | 0.994 |
| Ramp Up / It's On! | 0.884 [0.84, 0.93] | 1.004 | 0.939 [0.91, 0.97] | 0.929 |
| Tail Held High / Tail Nine | 0.994 [0.93, 1.06] | 0.946 [0.90, 0.99] | 0.984 | 0.986 |
| Racing Spirit: Stamina | no deck source | 1.003 | 1.060 | 1.049 |
| Slipstream / Perfect Spot! | 1.011 [0.93, 1.10] | 0.998 | 1.021 | 0.978 |
| Risky Business / Nothing Ventured | 0.830 [0.76, 0.90] | 0.944 | 0.897 [0.85, 0.95] | 0.888 |

Training-pool hints by scenario (tests $\beta$ and $d$): Grand Concert 1.009 [0.96, 1.06], Unity Cup 0.967 [0.86,
1.08], URA Finale 0.980 [0.86, 1.11], Trackblazer 0.777 [0.61, 0.97]: only Trackblazer's interval excludes 1, its
$\beta$ over-predicting these runs. By the lowest limit break of the sourcing cards: LB4 0.997, LB3 0.963, LB0-2 0.83-1.03 (72-118 rows each, every interval
covering 1). Unity Cup's guaranteed golds: 853 of 853 runs
each.

Gold acquisition (obs / pred): Nothing Ventured 0.847 [0.77, 0.92] (833 rows), Perfect Spot! 0.838 [0.76, 0.92] (439), It's
On! outside Unity Cup 0.456 [0.31, 0.66] (119), No Stopping Me! outside Unity Cup 0.885 (63), See Ya Later! 1.000
(2,723).

Reliability, deck hint and acquired (observed, mean prediction in brackets):

| Predicted | Deck hint: rows, observed | Acquired: rows, observed |
|---|---|---|
| 0 | 8,743, 0.001 | 5,246, 0.015 |
| above 0 to 0.20 | 102, 0.235 (0.17) | 1,286, 0.166 (0.16) |
| 0.20-0.40 | 2,620, 0.297 (0.31) | 2,715, 0.309 (0.30) |
| 0.40-0.60 | 1,328, 0.447 (0.47) | 3,147, 0.498 (0.50) |
| 0.60-0.80 | 910, 0.623 (0.72) | 4,609, 0.706 (0.71) |
| 0.80-0.95 | 515, 0.740 (0.87) | 3,310, 0.828 (0.86) |
| 0.95-1 | 1,556, 0.962 (1.00) | 3,606, 0.978 (1.00) |

The deck's over-prediction sits in its top buckets, the event rewards: those predicted 0.6 and above hold 82% of the
shortfall, and 97% of their rows have an event source. The 0.20-0.40 bucket, almost all training-pool rows, is within
its interval (0.297 [0.280, 0.315] against 0.308).

**Per run** (7.14 open targets each): targets acquired, expected 3.673, observed 3.650; all open targets acquired,
product of the model's odds 0.041, observed 0.039 [0.033, 0.047]. Landing $P(\text{all eight})$ recomputed from what
each run acquired against the solver's exact $P(\text{all})$: 0.863 [0.69, 1.05] (bootstrap over runs; Unity Cup 0.851
[0.67, 1.03], Grand Concert 1.27 [0.84, 1.71], URA Finale 0.88 [0.36, 1.60]; Trackblazer's model value is 0). Consistent
with calibrated, though the interval allows up to about 1.4× optimism (1/0.69). Only 132 runs acquired every open
target, too few to separate the exact joint from the product.

Single-source cards furthest from the model (every pair of 40 or more rows whose interval excludes 1, and Gold
Ship's):

| Card > target | Pred | Obs | Ratio | Rows | Likely reason |
|---|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | 1.000 | 0.731 | 0.73 [0.66, 0.79] | 186 | model assumes Risky Business prioritised (finale choice 2) |
| Riko Kashimoto [Planned Perfection] > Ramp Up | 0.750 | 0.579 | 0.77 [0.71, 0.83] | 420 | open (her Lone Wolf choice when prioritised accounts for part) |
| King Halo [Tonight, We Waltz] > Uma Stan | 0.937 | 0.726 | 0.77 [0.66, 0.87] | 73 | Uma Stan sits on a choice that needs a priority |
| Gold Ship [That Time I Became the Strongest] > Uma Stan | 0.196 | 0.133 | 0.68 [0.38, 1.17] | 75 | open |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | 0.301 | 0.068 | 0.23 [0.08, 0.60] | 44 | open |
| Maruzensky [Sentimental Flare ♪] > Slipstream | 0.237 | 0.301 | 1.27 [1.04, 1.53] | 236 | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | 0.346 | 0.493 | 1.43 [1.11, 1.75] | 75 | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | 0.710 | 0.638 | 0.90 [0.81, 0.98] | 235 | open |
| Sweep Tosho [Lamplit Training of a Witch-to-Be] > Slipstream | 0.337 | 0.282 | 0.84 [0.70, 0.994] | 305 | open |
| Nakayama Festa [Trusting the Dice] > Risky Business | 0.614 | 0.508 | 0.83 [0.75, 0.90] | 423 | open |

Below 40 rows 5 more pairs exclude 1; 3 of them are event sources of Playtime's Over! that hinted it in 1 of 46 runs
against 0.50-0.84 predicted (Yukino Bijin [Dancing Light into the Night], Yaeno Muteki [Fiery Discipline], Sirius
Symboli [Escorte Étoile]). Their event puts the skill on choice 2, which ends the chain early (before its last link), so IT
takes it only when the skill is prioritised; only 3 of those runs show a priority list (the skill prioritised in 1, hinted in 0 of the 3) and the
other 43 hinted it 1 time, so the priority cause is untested here.

**Other skills** (every non-target skill a deck card or spark could give, on a 30% sample of the runs, 1004): inheritance 0.993
[0.97, 1.01] over 43,244 rows, so inheritance holds beyond the eight targets; training-pool hints read 1.064
[1.04, 1.09] over 16,978, under-predicted.

**Untested here:** the step from acquired to landed (spark generation, and that everything hinted is bought); the
priority policy run by run; correlations inside a run beyond $P(\text{all})$; Grand Masters; hint levels. The runs skew
to good careers; every comparison is per run on that run's own deck and lineage. Rows with an inspiration are left out
of the deck test, which is then conditional on no spark, treated as independent of the deck.
