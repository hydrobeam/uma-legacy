# The model's mathematics

How the planner computes its odds, how each part was set, and how well it holds against real careers.

## Results

Against 2,936 real Independent Training careers rebuilt from their own decks and lineages (section 12),
the chance the model gives each target of being acquired is calibrated: observed / predicted
0.995 [0.98, 1.01] over 20,923 run-target rows. Its chance of landing all eight default
targets reads 0.84 [0.65, 1.05] of the model, consistent with calibrated but on only 90 runs that
acquired every target.

| Part | How it is set | Main evidence | Section |
|---|---|---|---|
| Spark generation, $0.2$ / $0.4 \times 1.1^n$ | external fit | about 1.2M observations of Champions Meeting rooms (not ours) | 1 |
| Star mix of sparks | external fit | Champions Meeting room data (not ours) | 2 |
| Inheritance rolls | community bases, grandparents fitted | observed / predicted 0.985 [0.97, 1.00] on 23,488 rows | 3 |
| Affinity of a trainee's own grandparent: no trio points | fitted | 13 nats over the alternative, 101,536 trials | 4 |
| Loop ranking by the average over lineages | assumed; race weights derived | | 5 |
| Training hints, $\beta(1 + hf/100)(1 + d\,\mathit{sp}/100)$ | form derived, $\beta$ and $d$ fitted | 54,640 rows; real careers 0.978 [0.93, 1.03] of the model | 6 |
| Chain events, Binomial(21, 0.35) per career | fitted | 1,299 runs; 5.9 nats from a free distribution with 14 fewer parameters, Poisson 30.3 | 7 |
| Chain events spread over remaining links | assumed, preferred by the data | final links with 0 pals 0.15 against 0.13 observed (spread over cards: 0.22) | 7 |
| Event choices: IT takes a prioritised skill's choice | assumed | event hints of prioritised targets 1.02 of the model | 8, 12 |
| Measured outcome shares | fitted | five events, e.g. Nakayama Festa's Nothing Ventured 0.61 [0.55, 0.66] (90%) | 8 |
| Random card events | fitted, not modelled | 2,871 readings; crediting them moves the odds by at most 1.7% | 9 |
| Conditional character events | not modelled | | 10 |

**Known misses**, each measured in its section:

- Deck hints read 0.939 [0.91, 0.97] of the model, almost all from event hints of targets the
  player did not prioritise (0.65 of the model; the model assumes every target is prioritised).
- Trackblazer's hint base over-predicts real careers: its training-pool hints read 0.759 [0.59, 0.95] of the model.
- In crowded decks early chain links run short and more chains never start than predicted (0.35 against
  0.19 with six chain cards). Final links fit when pooled (0.355 against 0.358) but individual cards
  differ from the model far beyond chance ($\chi^2$ = 248 on 37 cards).
- A skill also hinted by inspiration is hinted by training less than predicted (0.78 of the model).
- Training-pool hints of non-target skills read 1.058 [1.04, 1.08] of the model.

**Not settled.** Specialty priority predicts runs it was not fitted to (+9.8 nats held out), but whether it
causes more hints is not shown: across limit breaks of the same card the estimate has the same size ($d$ =
0.42) without being significant (+1.3 nats). The hint-frequency slope $1/100$ is assumed; a free
divisor fits better.

**Untested:** the step from acquired to landed (spark generation, and that everything hinted is bought), Grand
Masters (no runs), hint levels, and the priority policy run by run.

## How to read this document

Every quantity carries a label: **assumed** (a chosen rule, or a community figure without published data), **fitted**
(estimated from data; sample and method given) or **derived** (follows from the other parts). Intervals are 95% unless
marked. "Runs" are Independent Training runs from uma-it, pooled.

**Where the numbers come from.** Every number this document states about the runs, the game data or a derived
quantity is computed by the scripts in `research/`, one per section, and written in when the document is rendered:
`./build.sh research` runs them, they write `research/results.json`, and `research/render.py` fills
`docs/RESEARCH.template.md` to produce this file. A number that is not computed is cited and says so: external fits
and community figures (sections 1-4) and solver experiments (sections 5, 9, 10, 11). Each script also tests the
conclusions its section draws (a likelihood-ratio test, an interval covering or excluding a prediction) and reports
each as holding or failing. The run records are not in this repository; a clone renders from the committed results
and recomputes what needs only game data.

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

## 1. Spark generation

$$g_k = g_0 \cdot 1.1^{\,n_k}, \qquad g_0 = 0.2 \text{ (white learned)}, \quad g_0 = 0.4 \text{ (gold learned)}$$

$n_k$ = ancestors (of six) carrying the spark of $k$. A bought gold sparks its white. In a running loop every target
spark sits on all six legacies (**assumed**: a loop keeps veterans that landed everything), so $n_k = 6$,
$1.1^6 = 1.771561$ and $g = 0.3543$ (white) / $0.7086$ (gold).

- $g_0$ and the $\times 1.1$ curve: a community curve, **fitted** externally on about 1.2M white/gold observations
  from Champions Meeting room data, $p \ge 0.84$ per tier. An additive alternative (+2.5 points per carrier) is
  rejected, $p \approx 10^{-14}$. The data cannot separate 0.35 from 0.354 at six carriers.
- That everything acquired is bought: **assumed**, untested (section 12).

## 2. Star mix of legacy sparks

A legacy's white (and unique) spark has $s \in \{1, 2, 3\}$ stars with shares $\pi = (\pi_1, \pi_2, \pi_3)$ set by the
veteran's rank score $R$:

| Rank score | $\pi_1 / \pi_2 / \pi_3$ | Fit |
|---|---|---|
| below 6,500 | 0.90 / 0.10 / 0 | |
| 6,500-17,499 | 0.50 / 0.45 / 0.05 | 528,637 white sparks, $p = 0.13$ |
| 17,500+ | 0.20 / 0.70 / 0.10 | 196,646 white sparks, $p = 0.77$ |

**Fitted** on Champions Meeting room data (goodness-of-fit $p$ shown). The band per scenario is **assumed**: 17,500+
for Grand Concert and Grand Masters, below it for URA Finale, Unity Cup and Trackblazer (by the rank score a typical
career of each scenario reaches if every skill point is spent; not recomputed here).

The loop model does not know a legacy's stars and uses the expected base per roll (**derived** from section 3):

$$\bar b = 0.03\,(\pi_1 + 2\pi_2 + 3\pi_3) = 0.0465 \;(\text{below } 17{,}500), \quad 0.057 \;(17{,}500+).$$

## 3. Inheritance rolls

Computed by `research/s12_calibration.py` (the test).

Per legacy $l$, target $k$ and roll:

$$r_{lk} = \min\!\left\{1,\; b(s_{lk})\left(1 + \frac{A_l}{100}\right)\right\}, \qquad
q_k = 1 - \left[\prod_{l=1}^{6} (1 - r_{lk})\right]^{2}$$

$q_k$ = P(at least one inspiration of $k$ in the career); two rolls (Classic and Senior April); $A_l$ is the legacy's
affinity (section 4); $b(s) = 0$ without a spark. Base by spark type and stars ($1/2/3\star$), **assumed** from
community measurements (one of 1,000+ trials) and consistent across every source checked:

| Spark | Skill / scenario white | Race white | Green (unique) | Pink | Blue |
|---|---|---|---|---|---|
| $b$ | 3 / 6 / 9% | 1 / 2 / 3% | 5 / 10 / 15% | 1 / 3 / 5% | 70 / 80 / 90% |

The odds use skill whites only. Grandparents take the full base (**fitted**: with grandparents halved, every predicted
spark chance was about half the measured level). Great-grandparents count for nothing (**assumed**, matching the
game's two-generation lineage). In the loop model $b(s_{lk})$ is replaced by $\bar b$ in each roll. That is exact for
one roll but not for two: a legacy's stars are the same in both rolls, and by convexity
$E[(1 - r)^2] \ge (1 - E[r])^2$, so the loop model slightly overstates $q_k$ (**derived**). The career planner uses
each veteran's real stars and is not affected.

**Test** on 2,936 real careers (section 12), observed / predicted inspiration:

- all 23,488 run-target rows: 0.985 [0.97, 1.00];
- by the number of carriers, 1 to 6: 0.957-1.001, every interval covering 1;
- per roll: spark-years 0.985 (1.95 standard deviations below the model's expectation, at the edge of its
  95% band), ancestor hits ($2\sum_l r_l$) 0.987;
- 37,785 rows of non-target skills: 0.993 [0.97, 1.01].

By predicted chance, every bucket but one matches its prediction; 0.40-0.60 sits just below it (0.487
[0.471, 0.503] against 0.505):

| Predicted $q$ | Rows | Observed (mean predicted) |
|---|---|---|
| 0 | 7,785 | 0.002 |
| above 0 to 0.20 | 2,278 | 0.154 (0.16) |
| 0.20-0.40 | 2,868 | 0.299 (0.30) |
| 0.40-0.60 | 3,585 | 0.487 (0.50) |
| 0.60-0.80 | 4,851 | 0.691 (0.70) |
| 0.80-0.95 | 2,121 | 0.835 (0.84) |

## 4. Affinity

Computed by `research/s04_affinity.py` (the self-grandparent fit).

**Relation points** (exact, game data): $\mathrm{pair}(a,b)$ and $\mathrm{tri}(a,b,c)$ sum the relation points of the
relation groups containing all members. A group counts only distinct characters, so $\mathrm{tri}(T, P, T) = 0$.

**Race affinity** (**assumed**, the game rule as community tools implement it; two independent implementations
agree exactly): +3 per single-race G1 both won, counted per race (relocated venues are the same race), multi-race
titles 0. With schedules fixed in advance and two careers independent (**assumed**), expected shared wins are

$$o(a,b) = \sum_r P_a(r)\,P_b(r), \qquad P_a(r) = 1 - \prod_{\text{attempts}} (1 - p),$$
$$p = \mathrm{clamp}\big(110 - c_{\text{dist}} - c_{\text{surf}} - \mathrm{PEN}_j,\ 0,\ 100\big) / 100,$$

$c$ = 0 for S/A, 10 B, 20 C, 30 D, 40 E, 60 F, 90 G; $\mathrm{PEN}_j$ = 0, 0, 5, 20, 30, 50 for the $j$-th race in a
row (any race, goals included; a rest resets). The win table is a community table (**assumed**). Stats and training
turns are not modelled.

**Per-legacy affinity.** For parent $P$ with other parent $P'$ and own parents $\mathrm{par}(P)$, and grandparent
$G \in \mathrm{par}(P)$:

$$A_P = \mathrm{pair}(T,P) + \mathrm{pair}(P_1,P_2) + 3\,o(P,P') + \sum_{G \in \mathrm{par}(P)} \big[\mathrm{tri}(T,P,G) + 3\,o(P,G)\big]$$
$$A_G = \mathrm{tri}(T,P,G) + 3\,o(P,G)$$

In a loop $P'$ is also one of $P$'s own parents, so $o(P, P')$ counts twice in $A_P$. **Fitted** externally (not
recomputed here) to 8,155 Independent Training receipts (fired against predicted within 0.1% for parents, 2% for grandparents). A second
implementation gives identical numbers on one test loop's 12 veterans (lineage total 452 = 184 + 97 + 171), which
shows the implementations agree, not that the formula is the game's.

**Self-grandparent** ($G = T$, section 5). $\mathrm{tri}(T,P,T) = 0$ is **fitted** on 101,536 inspiration trials
attributable to a single ancestor in runs: self-grandparent passes 523 observed,
521.8 predicted with 0, 605.4 with $\mathrm{tri}(T,P,T) = \mathrm{pair}(T,P)$ (about 13 nats worse); the family
$\kappa\cdot\mathrm{pair}(T,P)$ fits $\hat\kappa = 0$, 95% upper limit about 0.33.

## 5. Loop structures, ranking and schedules

Computed by `research/s05_loops.py`.

A lineage is $(T, P_1, P_2, X)$: each parent bred from the other parent and $X$. A four-uma loop has 12 lineages (each
member trains from two of the other three); a three-uma rotation has 3, with $X = T$ (the trainee's own earlier
veteran). The rotation occurs in practice: 907 of 3,179 runs have a grandparent of
the trainee's own character (0 have a parent of her character). A structure $S$ with lineage set $\Lambda_S$ scores

$$\bar P_S = \frac{1}{|\Lambda_S|} \sum_{\lambda \in \Lambda_S} P_\lambda(\text{all}),$$

the long-run rate of a player who trains every member in turn (**assumed**: a player who re-trains only the best
trainee would score a structure by its best lineage).

**Race weights** (**derived**). In one lineage $o(P_1,P_2)$ enters the six affinities 6 times and $o(P_1,X)$,
$o(P_2,X)$ twice each. Summed over the 12 loop lineages every pair gets $6\cdot 2 + 2\cdot 4 = 20$; over the 3
rotation lineages $6 + 2 + 2 = 10$. Both are uniform over pairs, so a structure's schedules maximise
$\sum_{a<b} o(a,b)$.

**Schedules for shared wins, not odds** (**measured** by a solver experiment, not recomputed here). Re-optimising every solved loop's schedules for $\bar P_S$
directly gained at most +0.12% on any loop and +0.065% on a winner, and changed no top-1 or top-3 loop over 46
validation setups; the ceiling with every pair at its best possible overlap is only 0.56-2.83% above the odds found.

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
hinted the skill, R cards out. The form was fixed on 879 runs (19,071 rows, 17,079 without a spark); $\beta$ and $d$
are fitted on all runs (54,640 rows, 9,634 card-runs; the Runs column counts the runs that add a row):

| Scenario | $\beta$ | $d$ | Adopted $\beta$ / $d$ | Rows | Runs |
|---|---|---|---|---|---|
| Unity Cup | 2.356 | 0.358 | 2.36 / 0.36 | 12,967 | 825 |
| Grand Concert | 1.968 | 0.084 | 1.97 / 0.08 | 29,114 | 1,484 |
| URA Finale | 1.343 | 0.390 | 1.34 / 0.39 | 10,218 | 586 |
| Trackblazer | 1.699 | 0.111 | 1.70 / 0.11 | 2,341 | 98 |
| Grand Masters | no runs | | 1.86 / 0.26 (**assumed**: one $\beta$ and one $d$ over every scenario) | | |

What fixed the form (879 runs):

| Question | Evidence |
|---|---|
| Rate form, LL on 19,071 rows | $1 + hf/25$ over options −12,813.3; $1 + hf/25$ over skills −12,461.0; constant −12,048.0; $\beta(1 + hf/100)$ −11,910.2; per-scenario $\beta(1 + hf/100)$ −11,817.8; free divisor ($c = 76$) −11,811.9 |
| Skills only, not skills + stat option | in 16 of 19 (card, $hf$) cells the draw-count variance is below the binomial floor a drawn stat option implies |
| Uniform draws with repeats | repeat rate $P(k \ge 2 \mid k \ge 1)$: 0.151 observed, 0.151 two-point count, 0.199 Poisson count; distinct skills 1/2/3/4 per card-run 0.093 / 0.520 / 0.307 / 0.074 observed, 0.084 / 0.476 / 0.359 / 0.079 model |
| Covariates | per-scenario bases +92 nats; card type, rarity, limit break beyond $hf$, pals per deck, initial bond, pool size each at most 2.9 nats (factors 0.91-1.02) |
| Lv-5 cap | beats drawing with replacement by 7.8 nats with no parameter (17.5 on 2,402 runs); obs / pred by draws to Lv 5, replacement vs cap: 5 draws (5,347 rows) 0.968 / 0.979, 3 (259) 0.863 / 0.870, 2 (7,335) 1.025 / 1.024, 1 (94) 1.289 / 0.985 |
| Joint draws over targets | 38,927 pairs of no-spark pool skills on one card-run (Trackblazer left out): $P(\text{both})$ 0.0872 observed, 0.0862 joint, 0.1135 independent; $P(\text{neither})$ 0.4371 vs 0.4354 |
| Pool size | obs / pred 0.88-1.14 for every $n$ with at least 150 rows |
| Per-draw increment | 5,144 of 5,150 single-source levels are multiples of $\iota$, capped at 5 |
| $hf$ by card level | floored linear interpolation −10,676.9, step −10,677.4, max-limit-break $hf$ for all −10,683.8 |

The slope $1/100$ is **assumed**: a free divisor fits better ($c = 76$ in the table above) and is not adopted.

Reading $h$ as a floor/ceiling mixture against a two-point count differs by under 1% ($n = 5$, $h = 2.74$: miss
0.5426 vs 0.5453). A given setup can sit about half a draw off the smooth formula; $hf$ spans 0-60 in the data.

**Spark deficit** (not modelled; the fit conditions on no spark, the case in which a card's hint matters):

| Other source of the skill | Rows | Observed | Predicted | Ratio |
|---|---|---|---|---|
| none | 14,949 | 0.345 | 0.345 | 1.00 |
| inspiration | 1,696 | 0.269 | 0.347 | 0.78 |
| inspiration + card event | 139 | 0.158 | 0.369 | 0.43 |
| card event | 635 | 0.334 | 0.351 | 0.95 |
| other card's training | 572 | 0.372 | 0.376 | 0.99 |

Full drop-out of spark-hinted skills from the draws is rejected (with two pool skills spark-hinted, the rest are hinted
at 0.375 against 0.341 without drop-out and 0.455 with it).

**Specialty priority** (effect 19). On the fit's setup above, the rate without the specialty term leaves a residual
that rises with specialty: observed / predicted 0.951 at specialty 0 (15,281 rows), 0.967 at 1-29 (561), 1.003 at
30-59 (27,817), 1.067 at 60+ (10,981). The term gains 35.1 nats for 4 parameters (one $d$ for every scenario: 24.8
nats, $d = 0.195$; an exponential form $e^{g_s\,\mathit{sp}/10}$ fits the same, 35.2 nats). The bands then read 1.003 /
0.994 / 0.994 / 1.005, no (scenario, band) cell with 200+ rows is off by more than $|z| = 1.15$, and Unity Cup cards
with $hf = 0$ and specialty 0 go from 0.886 to 0.977. One pooled $d$ stays within 0.16-0.26 on either half of the runs,
on SR or SSR cards alone and without any one of the eight most-used cards. Fitted on the earlier runs and scored on
the later 31,023 rows, it gains 9.8 nats: it predicts runs it was not fitted to. Against real careers
(section 12) training-pool hints read 0.978 of the model with it.

The term is an association, not shown to be a cause: high-specialty cards might share another trait that raises
hints. The cleanest test compares the same card at different limit breaks, where specialty changes and the card does
not (46 cards, a base per card and scenario): $d$ = 0.42, the same size as across cards, but a gain of only
1.3 nats, not significant. The game's placement rule (each facility weighs 100, a card's specialty facility
100 + $\mathit{sp}$, absence 50) suggests a mechanism; it is not tested here.

Prioritised skills raising training hints: 101 runs show a priority list, prioritised skills hinted by training at 0.222
against 0.298 predicted (99 rows); no effect assumed.

## 7. Chain events

Computed by `research/s07_chains.py`.

A run gets $N$ chain events, the same for every deck and scenario (**fitted**):

$$N \sim \mathrm{Binomial}(21, 0.35), \qquad P(N = e) = \binom{21}{e}\, 0.35^{e}\, 0.65^{21-e}, \quad E N = 7.35,\ \mathrm{Var}\, N = 4.8.$$

Each event advances one non-pal chain card by one link (links fire in order). Card $i$ with chain length $L_i$ (SSR 3,
SR 2; a filler slot counts as a 2-link SR) and $d_i$ links fired is picked with

$$P(i) = \frac{w_i}{\sum_j w_j}, \qquad w_i = L_i - d_i \text{ while running, } 0 \text{ once stopped},$$

i.e. uniform over the remaining chain slots (**assumed**, preferred empirically). Pal and group cards take no slot
(**assumed**, supported below). Without stop risk $P(\text{all chains complete}) = P(N \ge \sum_i L_i)$; two 3-link
cards: $P(N \ge 6) = 0.80$.

**Stop risk** (**derived** from the event data and the choice rule, section 8): if IT's default choice at link $l$ ends
the chain on $k$ of its $n$ outcomes, $q_l = k/n$ and the stopping link $\sigma$ has
$P(\sigma = l) = q_l \prod_{l' < l} (1 - q_{l'})$; a stopped chain's slots return to the pool. The solver enumerates the
exact distribution of final chain states (no simulation); an independent recomputation agrees to about $10^{-16}$.

**Fit of $N$** (1,299 runs, the pal-count table below). Chain progress per card is read from marker skills: a link is
observable only if every choice and outcome of its event carries a skill appearing in no other event of that card. A
credit naming two sources ("A or B event") is unknown when B is another deck card whose events can give the marker, and
so is a marker listed only under inspiration. $N$'s distribution is the one that best explains progress over all decks.

The fit allows a fired link's marker to go unseen with chance 0.03 (**assumed**, a measurement allowance). Without
it, a run in which a card's later link shows while an earlier one does not has likelihood 0. Since links fire in
order, those runs measure the rate: of 581 earlier links implied fired by a later one, 13 show no marker, 0.022
[0.013, 0.038]. The cause is not known. The fit does not depend on it: with allowances from 0.01 to 0.045, or with
none on the 1,288 runs that show no such case, the best $p$ at $n = 21$ is 0.34-0.355.

| Form for $N$ | Result |
|---|---|
| distribution-free | mean 7.36, variance 4.93 |
| Binomial(21, 0.35) | 5.9 nats behind distribution-free with 14 fewer parameters; with $n = 21$, $p = 0.35$ fits best |
| Binomial(15, 0.48) | 14.1 nats behind distribution-free |
| Poisson(7.1) | 30.3 nats behind distribution-free (variance = mean, too wide) |
| per scenario | means Unity Cup 7.56, Grand Concert 7.29, URA Finale 6.79; +19.1 nats for 32 extra parameters |

The best binomial with $n$ free fits 0.0 nats better than (21, 0.35). A first fit on
879 runs, with markers read from IT's default choice only, chose the same Binomial(21, 0.35); so did a refit on 1,792
runs with those markers (means 7.52 / 6.84 / 7.20 / 7.92 for 1 / 2 / 3 / 4 pals, variances 4.0-5.3). The sub-Poisson
spread suggests a capped number of chances per run; a reading of the shape, not verified.

One $N$ reproduces final-link completion at every pal count, so pal events do not consume chain events:

| Pals | Chain cards | Runs | Final link reached: observed / Binomial(21, 0.35) / Poisson(7.1) |
|---|---|---|---|
| 5 | 1 | 120 | 0.98 / 0.98 / 0.96 |
| 4 | 2 | 285 | 0.88 / 0.86 / 0.79 |
| 3 | 3 | 391 | 0.56 / 0.53 / 0.48 |
| 2 | 4 | 212 | 0.29 / 0.30 / 0.29 |
| 1 | 5 | 168 | 0.19 / 0.16 / 0.17 |
| 0 | 6 | 123 | 0.14 / 0.15 / 0.15 |

**Allocation rule.** Against uniform over cards (each running card's next event equally likely), on 1,193 runs with
markers read from IT's default choice:

| | Uniform over remaining slots (adopted) | Uniform over cards |
|---|---|---|
| Total LL | −2,397.2 | −2,396.7 |
| Final links, 2 pals (observed 0.27) | 0.29 | 0.36 |
| Final links, 0 pals (observed 0.13) | 0.15 | 0.22 |

The total LL is level; the slot rule is preferred because the card rule's extra completions in crowded decks are not
seen. Exact consequences with Binomial(21, 0.35), P(reach link 1 / 2 / 3):

| Deck | Slots (adopted) | Cards |
|---|---|---|
| two 3-link SSRs | 1.00 / 0.97 / 0.87 | 0.99 / 0.96 / 0.87 |
| 3-link SSR + 2-link SR | SSR 1.00 / 0.99 / 0.93, SR 0.99 / 0.95 | SSR 1.00 / 0.98 / 0.93, SR 0.99 / 0.96 |
| three 3-link SSRs | 0.98 / 0.84 / 0.54 | 0.96 / 0.81 / 0.58 |
| four 3-link SSRs | 0.93 / 0.65 / 0.25 | 0.88 / 0.62 / 0.33 |

(With two 3-link cards and exactly 3 events, the slot rule gives $(3,0)$ with $\tfrac36 \cdot \tfrac25 \cdot \tfrac14 = 5\%$,
the card rule $(\tfrac12)^3 = 12.5\%$.)

**Known misses.** Two-chain decks (two 3-link SSRs, four pals) finish both chains in 130 of 157 runs, 0.83
[0.76, 0.88], against 0.80 predicted: within the interval. Early links in crowded decks run short; per run as a cluster, $z = \sum(\text{obs} - \text{pred}) / \sqrt{\sum(\text{obs} - \text{pred})^2}$:

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Early-link shortfall | 0.02 ± 0.02 | 0.07 ± 0.02 | 0.06 ± 0.02 | 0.05 ± 0.03 | 0.15 ± 0.03 |
| $z$ | 1.0 | 3.4 | 2.5 | 1.8 | 4.4 |

Final links fit when pooled (0.355 observed against 0.358) but individual cards differ
from the model far beyond chance: over the 37 cards with 60 or more final-link observations the squared
standardised differences sum to 248 (expected about 37). The cause is not known.

More chains never start in crowded decks than predicted: 0.19 against 0.13 with five chain cards, 0.35 against 0.19 with
six. The shortfall concentrates in a few cards (obs / pred): Special Week [Take Hold of Our Dreams!] 0.65 / 0.82, Yaeno
Muteki [Fiery Discipline] 0.78 / 0.90, Zenno Rob Roy ["Rhodonite and the Dreamstone"] 0.63 / 0.75, Hishi Akebono [Who
Wants the First Bite?] 0.86 / 0.94, Smart Falcon [My Umadol Way! ☆] 0.72 / 0.81, Silence Suzuka [Beyond This Shining
Moment] 0.56 / 0.77; their event data look like those of cards that fit (King Halo [Tonight, We Waltz] 0.90 / 0.91, n =
111). A per-card activation parameter (a chain card active with probability $a$) fitted three-card decks ($a = 0.82$) on
the first reading, which carried marker errors; on the corrected reading, fitted jointly with $N$, it gains 0.3 nats
($a = 0.97$): rejected.

## 8. Event choices and outcomes

Computed by `research/s08_outcomes.py` (the certainty checks: `research/s12_calibration.py`).

**Choice rule** (**assumed**): IT takes an event's top choice unless a prioritised skill (up to 10, set at career start)
sits on another choice, and the player is assumed to prioritise every target hint the model counts. Where targets sit
on different choices, a deck entry takes one choice (the solver offers one entry per useful choice, and an entry that
ends a chain early).

Evidence on the rule. Sasami Anshinzawa [This Might Sting!]'s finale: with Risky Business prioritised IT took choice 2
in about 95% of runs (3 of the 4 exceptions with Risky Business already at hint Lv 5); with neither skill prioritised, 0 of
35 runs took choice 2. Riko Kashimoto [Planned Perfection]: with Lone Wolf prioritised IT took the bottom choice
in 55 of 62 runs. Every run has a priority list, but the run pages show it for only 358 of 3,179 (the rest count as
unknown, never as an empty list), so the rule is tested mainly through its consequences (section 12: prioritised targets
1.02, non-prioritised 0.65 on event-only hints).

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
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante", Nothing Ventured (601 runs with the card, below) | 138/228 = 0.61 [0.55, 0.66] | 0.6 |

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

Nakayama Festa's share is measured on every research run with her card plus a separate batch of runs with her card
(601 in all; the batch is kept out of every other fit, since every run in it has her card). Her event is credited
to her alone with Nothing Ventured in 138 runs and with Risky Business in 68. In 39 more, Risky Business is credited to
"her or another card's event"; the other card gives it alone in 0.895 of runs without her (Team Sirius, 29 of the 39)
or 0.389 (Sasami Anshinzawa), she in 0.119 of hers, so a shared credit is hers with chance
$r_F / (1 - (1 - r_F)(1 - r_o))$: 6.4 of the 39. In 39 runs neither shows while Risky Business is already at hint Lv 5
(a Risky Business win cannot show, or the event never fired); they are split by the rate at which the event shows where
both outcomes can, 0.39. The share is then 138/228 = 0.61 [0.55, 0.66]; it stays within 0.525-0.625 whatever share of
the shared credits is hers, and shows no dependence on the trainee's final Wit.

**Pal and group cards.** Their events are entered per card with a probability (finales certain); first-year date
events are left out in IT (**assumed**). Certainty checks: Light Hello [From the Ground Up] credits See Ya Later! to her
event in 852 of 852 runs carrying her; Team Sirius gives Risky Business in 64 of 64.

## 9. Random card events (not modelled)

Computed by `research/s09_random_events.py`.

A support card's character's shared random event fires, per event per career with a card of hers in the deck (any rarity, R cards included) and another
uma training, with (**fitted**)

$$P(\text{fires}) = 0.21\,(1 - 0.047)^{K - 4},$$

$K$ = random events on offer (every shared event of the deck's characters plus the trainee's own): about 0.15 overall,
0.21 at $K = 4$, 0.13 at $K = 14$. Readings: top-choice marker skills credited to the character's event; 2,871
readings from 1,461 runs, 426 clean hits (58 ambiguous two-card credits counted as
misses; with them the rate is about 2 points higher).

| Form | LL |
|---|---|
| constant 0.148 | −1205.5 |
| $0.21(1 - 0.047)^{K-4}$ | −1199.9 |
| power law $c(K/8)^{-a}$, $a = 0.39$ (uniform competition would be $a = 1$) | −1200.7 |
| strict competition $1 - (1 - 1/K)^S$, $S = 1.57$ | −1213.4 |
| every event slot competing, $c / T$ with $T = K + 0.94 \cdot \text{pals}$ (chain links weigh 0), $c = 1.85$ | −1200.1 |

The slot form fits about as well as the adopted one (+5.4 nats over constant for two extra parameters): chain links play
no part, and a pal counts about as one more random event.

Observed hit rate by $K$ = 4 / 6 / 8 / 10 / 12 / 14: 0.17 / 0.20 / 0.19 / 0.17 / 0.14 / 0.13; within a run events fire
independently. On a larger sample (4,748 readings, 2,264 runs) the slope holds (15.7% at 8 on offer, 12.5% at 14) and
friendship, specialty priority and scenario show no effect distinguishable from zero, nor does rarity within a
character (pooled, SSR markers fire at 15.3% against SR 10.9%, mostly which characters have SR cards).

Only top-choice skills hint (bottom-choice ones need a priority; in 175 runs showing a priority list 0 were
prioritised). In a solver experiment (not recomputed here), crediting these events at 0.17 (0.9 for the trainee's
own) raised the odds by at most 1.7% across the rosters and scenarios tested and changed no best loop, so they stay out.

## 10. Conditional character events (not modelled)

Computed by `research/s10_conditional.py` (the counts).

Secret and race-result events: 168 events with hints for 81 umas (110 for 55 umas in Global), 11 touching the default
targets (Nimble Navigator, Ramp Up, Tail Held High), 0 of them giving a gold. In a solver experiment (not recomputed here), forcing every
target-relevant one to fire with certainty, even as a gold, left the best loop unchanged on every setup tested. One exception is modelled as always acquired: Tokai
Teio's Head-On (all three of its races are her career goals; the goals ask only for top 5, top 5 and top 3, but IT
won all three in 60 of 60 runs training her, and every run where her event could show credits Head-On), like Yamanin Zephyr's Slipstream.

## 11. The composed career model

Not recomputed in `research/`: the composition is derived; `val/joint_check.py` recomputes the solver's exact odds
independently; the exact / product ratios and the continuation fit come from solver and simulator runs.

For a target $k$ not guaranteed, with $G_k = 0.4\cdot 1.1^{n_k}$, $W_k = 0.2\cdot 1.1^{n_k}$:

$$p_k = d^{g}_k\, G_k + (1 - d^{g}_k)\, W_k \left[1 - (1 - d^{w}_k)(1 - e_k)(1 - q_k)\right]$$

$d^g_k$ = P(the deck hints the gold), $d^w_k$ = P(the deck hints the white), $e_k$ = P(the trainee's own events hint
it), $q_k$ from section 3. A skill on the trainee's outfit replaces the bracket by 1 (white) or gives $p_k = G_k$
(gold); a scenario-guaranteed gold gives $G_k$. Inheritance only ever supplies the white.

Targets are not independent: chain cards share one run's $N$, and one event's outcomes exclude each other. With $c$
the final chain state (and stopping links), sources are independent given $c$ except one card's training draws, which
are joint over its targets (section 6):

$$P(\text{all}) = \sum_{c} P(c) \prod_{k} p_k(c) \qquad (\text{exact, **derived**}).$$

The search prunes with $\prod_k p_k$ times a per-class cap (**assumed** from measurement: 1.05 for decks with at most two
chain-varying targets and no multi-target group, 1.25 otherwise). Exact / product ratios seen: up to 1.18 on Global deck
jobs and 1.17 on Grand Masters (both in the 1.25 class), at most 1.037 with two such targets (`pal_Fk3`), 1.127
with three. No data-free cap exists: a target whose only source is one card's last link makes the ratio
$1/P(\text{link})$.

**Career planner.** The same model with each veteran's real stars, real G1 wins and recorded parents (an unknown
parent brings no relation points; her sparks count); $n_k$ = legacies actually carrying $k$. Since $q_k$ and $e_k$
enter only as $(1 - e_k)(1 - q_k)$, $e_k$ is replaced by $1 - (1 - e_k)(1 - q_k)$ and $1.1^{n_k - 6}$ scales
$P(\text{all})$ (**derived**; checked to 9 digits against the loop model on a uniform-star case).

**Next-career continuation** (**assumed** form, **fitted** parameters). The planner's target is $V$, the expected
careers to a fully looped parent. A trainee's building step: builder careers lift her odds with chance
$\lambda_j = \lambda\,\big((1 - \phi)^j\big)^{1.5}$ at the $j$-th success, each closing a share $\phi$ of the
remaining log-odds gap to $p_\infty = 0.6 \times$ her odds with a full loop lineage; the step costs
$\min_n \big[\sum_{j<n} 1/\lambda_j + 1/p_n\big]$. $\lambda, \phi$ are fitted to the current pool; the shape (0.6,
1.5) to simulated builder traces on four real veteran pools (2, 5, 11, 19 veterans): within 0-5% of achievable
building on 2, 11 and 19, 70% too pessimistic on 5 (builders improve after the first lift; the form only decays).
Leave-one-pool-out fits ranged 0.6-1.0 and 1-3: a calibration on these pools, not a law.

## 12. Calibration against real careers

Computed by `research/s12_calibration.py`.

Each of 2,936 runs (URA Finale 598, Unity Cup 780, Grand Concert 1,477, Trackblazer
81) was rebuilt as a career: trainee in her outfit, scenario, the six cards at their own limit breaks (hint frequency
and increment at that limit break), both parents with their recorded parents and every ancestor's white sparks, and
race wins giving the recorded shared-G1 counts. Targets: the default eight; 23,488 run-target rows, of which 1,005 on
the trainee's outfit and 1,560 scenario golds sit outside tests 1 and 3. Intervals: Wilson 95% on the observed rate;
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
| 1. Deck hint, no inspiration | 14,015 | 0.245 | 0.231 [0.224, 0.238] | 0.939 [0.91, 0.97] |
| 1. Training pool only | 2,982 | 0.365 | 0.357 | 0.978 [0.93, 1.03] |
| 1. Events only | 1,864 | 0.887 | 0.807 | 0.910 [0.89, 0.93] |
| 1. Events only, target prioritised | 151 | 0.889 | 0.907 | 1.021 [0.96, 1.06] |
| 1. Events only, not prioritised | 193 | 0.849 | 0.549 | 0.647 [0.56, 0.73] |
| 2. Inheritance | 23,488 | 0.350 | 0.345 [0.339, 0.351] | 0.985 [0.97, 1.00] |
| 3. Acquired, any source | 20,923 | 0.497 | 0.495 [0.488, 0.501] | 0.995 [0.98, 1.01] |
| 3. Acquired, deck or inspiration only | 20,923 | 0.497 | 0.485 | 0.975 [0.96, 0.99] |
| 3. Acquired, prioritised | 1,052 | 0.709 | 0.736 | 1.037 [1.00, 1.07] |
| 3. Acquired, not prioritised | 1,316 | 0.533 | 0.487 | 0.913 [0.86, 0.96] |
| 4. Landing (sums of $p$) | 20,923 | 3,665.8 | 3,631.5 | 0.991 |

The prioritised / not prioritised rows use the runs that show a priority list.

**Verdict.** The composed acquisition odds are calibrated (0.995). Inheritance is right (one target of eight, Tail Held High, reads 0.928 [0.88, 0.98]). Deck hints read optimistic (0.939 of the model), almost all from the
priority assumption: event hints land at 1.02 × predicted for prioritised targets and 0.65 for others. Unmodelled scenario and
trainee events supply the few points the deck loses (208 target acquisitions in 20,923 rows, 1.0%).

Per target (obs / pred):

| Target | Deck hint | Inheritance | Acquired | Landing |
|---|---|---|---|---|
| Uma Stan / Superstan | 0.80 [0.65, 0.98] | 1.005 | 0.998 | 0.998 |
| Nimble Navigator / No Stopping Me! | 0.96 [0.84, 1.10] | 0.977 | 1.060 | 1.038 |
| Playtime's Over! / See Ya Later! | 0.973 [0.94, 1.00] | 0.993 | 0.984 | 0.993 |
| Ramp Up / It's On! | 0.894 [0.84, 0.95] | 1.003 | 0.943 [0.91, 0.98] | 0.929 |
| Tail Held High / Tail Nine | 0.975 [0.91, 1.05] | 0.928 [0.88, 0.98] | 0.969 | 0.971 |
| Racing Spirit: Stamina | no deck source | 0.997 | 1.060 | 1.048 |
| Slipstream / Perfect Spot! | 0.999 [0.91, 1.09] | 0.993 | 1.016 | 0.975 |
| Risky Business / Nothing Ventured | 0.836 [0.75, 0.93] | 0.968 | 0.920 [0.86, 0.99] | 0.909 |

Training-pool hints by scenario (tests $\beta$ and $d$): Grand Concert 1.008 [0.95, 1.07], Unity Cup 0.970 [0.86,
1.09], URA Finale 0.947 [0.82, 1.08], Trackblazer 0.759 [0.59, 0.95]: only Trackblazer's interval excludes 1, its
$\beta$ over-predicting these runs. By the lowest limit break of the sourcing cards: LB4 0.980, LB3 0.967, LB0-2 0.85-1.08 (33-87 rows each, every interval
covering 1). Unity Cup's guaranteed golds: 780 of 780 runs
each.

Gold acquisition (obs / pred): Nothing Ventured 0.868 [0.79, 0.95] (449 rows), Perfect Spot! 0.833 [0.75, 0.92] (366), It's
On! outside Unity Cup 0.437 [0.28, 0.65] (109), No Stopping Me! outside Unity Cup 0.894 (62), See Ya Later! 1.000
(2,336).

Reliability, deck hint and acquired (observed, mean prediction in brackets):

| Predicted | Deck hint: rows, observed | Acquired: rows, observed |
|---|---|---|
| 0 | 8,247, 0.001 | 4,958, 0.015 |
| above 0 to 0.20 | 81, 0.247 (0.17) | 1,238, 0.164 (0.16) |
| 0.20-0.40 | 2,054, 0.295 (0.31) | 2,305, 0.315 (0.30) |
| 0.40-0.60 | 1,108, 0.459 (0.47) | 2,765, 0.502 (0.50) |
| 0.60-0.80 | 737, 0.634 (0.72) | 3,909, 0.703 (0.71) |
| 0.80-0.95 | 402, 0.721 (0.86) | 2,657, 0.826 (0.85) |
| 0.95-1 | 1,386, 0.957 (1.00) | 3,091, 0.975 (1.00) |

The deck's over-prediction sits in its top buckets, the event rewards: those predicted 0.6 and above hold 85% of the
shortfall, and 96% of their rows have an event source. The 0.20-0.40 bucket, almost all training-pool rows, is within
its interval (0.295 [0.276, 0.315] against 0.314).

**Per run** (7.13 open targets each): targets acquired, expected 3.542, observed 3.525; all open targets acquired,
product of the model's odds 0.034, observed 0.031 [0.025, 0.038]. Landing $P(\text{all eight})$ recomputed from what
each run acquired against the solver's exact $P(\text{all})$: 0.840 [0.65, 1.05] (bootstrap over runs; Unity Cup 0.826
[0.61, 1.07], Grand Concert 1.27 [0.77, 1.79], URA Finale 0.83 [0.23, 1.61]; Trackblazer's model value is 0). Consistent
with calibrated, though the interval allows up to about 1.5× optimism (1/0.65). Only 90 runs acquired every open
target, too few to separate the exact joint from the product.

Single-source cards furthest from the model (every pair of 40 or more rows whose interval excludes 1, and Gold
Ship's):

| Card > target | Pred | Obs | Ratio | Rows | Likely reason |
|---|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | 1.000 | 0.730 | 0.73 [0.66, 0.79] | 185 | model assumes Risky Business prioritised (finale choice 2) |
| Riko Kashimoto [Planned Perfection] > Ramp Up | 0.750 | 0.596 | 0.79 [0.73, 0.86] | 359 | open (her Lone Wolf choice when prioritised accounts for part) |
| King Halo [Tonight, We Waltz] > Uma Stan | 0.937 | 0.726 | 0.77 [0.66, 0.87] | 73 | Uma Stan sits on a choice that needs a priority |
| Gold Ship [That Time I Became the Strongest] > Uma Stan | 0.196 | 0.122 | 0.62 [0.33, 1.10] | 74 | open |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | 0.301 | 0.070 | 0.23 [0.08, 0.62] | 43 | open |
| Maruzensky [Sentimental Flare ♪] > Slipstream | 0.237 | 0.299 | 1.26 [1.02, 1.53] | 224 | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | 0.348 | 0.491 | 1.41 [1.05, 1.77] | 57 | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | 0.710 | 0.625 | 0.88 [0.78, 0.97] | 192 | open |
| Sweep Tosho [Lamplit Training of a Witch-to-Be] > Slipstream | 0.338 | 0.282 | 0.84 [0.69, 0.996] | 294 | open |

Below 40 rows 6 more pairs exclude 1; 3 of them are event sources of Playtime's Over! that hinted it in 1 of 46 runs
against 0.50-0.84 predicted (Yukino Bijin [Dancing Light into the Night], Yaeno Muteki [Fiery Discipline], Sirius
Symboli [Escorte Étoile]). Their event puts the skill on choice 2, which ends the chain early (before its last link), so IT
takes it only when the skill is prioritised; only 3 of those runs show a priority list (the skill prioritised in 1, hinted in 0 of the 3) and the
other 43 hinted it 1 time, so the priority cause is untested here.

**Other skills** (every non-target skill a deck card or spark could give, on a 30% sample of the runs, 881): inheritance 0.993
[0.97, 1.01] over 37,785 rows, so inheritance holds beyond the eight targets; training-pool hints read 1.058
[1.04, 1.08] over 14,609, under-predicted.

**Untested here:** the step from acquired to landed (spark generation, and that everything hinted is bought); the
priority policy run by run; correlations inside a run beyond $P(\text{all})$; Grand Masters; hint levels. The runs skew
to good careers; every comparison is per run on that run's own deck and lineage. Rows with an inspiration are left out
of the deck test, which is then conditional on no spark, treated as independent of the deck.
