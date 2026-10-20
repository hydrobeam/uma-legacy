# The model's mathematics

A model of how a career in Uma Musume's Independent Training acquires and passes on skills: what each part is, how it
was set from data, and how well it holds against real careers.

## Results

The model's odds rest on one prediction: the chance that a career acquires each target. Against
3,348 real Independent Training careers, each rebuilt from its own deck and lineage (section 13), that chance is
calibrated: observed / predicted 0.994 [0.98, 1.00] over 23,919 run-target rows. The chance of landing
all eight default targets reads 0.86 [0.69, 1.05] of the model: consistent with calibrated, but only
132 runs acquired every target.

The terms used below (observed / predicted, rows, nats, held out, the bracketed intervals) and the game's terms are
defined in *How to read*, after this summary.

| Part | How it is set | Main evidence | Section |
|---|---|---|---|
| Spark generation, $0.2$ / $0.4 \times 1.1^n$ | assumed | none: the runs show the sparks a career inherited, not the ones it produced | 1 |
| Star mix of sparks | mixes assumed; band per scenario derived | each scenario uses the band most of its careers reach (14,742 Grand Concert runs, 79% at 17,500+) | 2 |
| Inheritance rolls | skill-white bases tested, grandparents fitted | grandparent weight 1.01 [0.98, 1.03], affinity slope 0.98 [0.95, 1.01] on 69,840 + 30,368 trials; whole careers 0.989 [0.976, 1.001] | 3 |
| Affinity: +3 per shared G1 win; own grandparent adds no trio points | +3 assumed and tested; trio rule fitted | shared-win slope 0.97 [0.93, 1.01]; trio rule 19 nats better than the alternative | 3, 5 |
| Race win table | community table, tested | calibrated once grades include inherited aptitude sparks: 1.000 [0.999, 1.002] over 76,402 G1 attempts | 4 |
| Loop schedules for shared wins | derived | on the reference loops, no other schedule could raise the best loop's odds by more than 2.02% | 6 |
| Training hints, $\beta(1 + hf/100)(1 + d\,\mathit{sp}/100)$ | form fixed on the runs, slope $1/100$ assumed and tested, $\beta$ and $d$ fitted | 61,856 rows; specialty term fitted on earlier runs gains +12.9 nats on later ones; real careers 0.988 [0.95, 1.03] of the model | 7 |
| Chain events, Binomial(21, 0.35) per career | fitted | 3,424 runs; not rejected against a free distribution (p-value 0.06); no alternative predicts unseen runs better | 8 |
| Chain events spread over remaining links | fitted | 66 nats better than equal shares per card | 8 |
| Event choices: IT takes a prioritised skill's choice | assumed | event hints of prioritised targets 1.01 of the model | 9, 13 |
| Measured outcome shares | fitted | five events, e.g. Nakayama Festa's Nothing Ventured 0.61 [0.55, 0.66] (90%) | 9 |
| Random card events | not modelled | where they could hint a target, acquisition reads 1.26 [1.11, 1.41] of the model left out, 1.00 credited | 10 |
| Conditional character events | not modelled | where only they can hint a target, left out reads 1.20 [0.75, 1.64]; Vodka's own Nimble Navigator 25 of 27 runs | 11 |

### Known misses

Each measured and tested in its section:

- Chain events (8): in crowded decks early links run short, and more chains never start than predicted
  (0.34 against 0.20 with six chain cards). Final links fit pooled (0.352 against 0.357), but
  individual cards differ far beyond chance ($\chi^2$ = 229 on 42 cards).
- Deck hints (13) read 0.937 [0.92, 0.95] of the model, almost all from event hints of targets the
  player did not prioritise (0.71 of the model; the model assumes every target is prioritised).
- Trackblazer's hint base (13) over-predicts real careers: its training-pool hints read 0.777 [0.62, 0.93] of the model.
- Training hints (7): SR cards and decks with more pals depart from the adopted rate (+5.3 and +14.0 nats), and
  a skill also hinted by inspiration is hinted by training less than predicted (0.75 of the model).
- Left out of the model: deck cards' random events, so the targets they can hint are under-predicted
  (1.26 [1.11, 1.41] of the model on those rows; section 10); and Vodka's own Nimble Navigator (25 of 27 runs;
  section 11).
- Non-target skills (13): training-pool hints read 1.064 [1.05, 1.08] of the model.

### Not settled

Specialty priority predicts runs it was not fitted to (fitted on earlier runs, +12.9 nats on later
ones), but it is not shown to cause more hints: across limit breaks of the same card the estimate is the same size
($d$ = 0.41) but not significant (+1.5 nats). On the adopted model, a free hint-frequency divisor
does not predict unseen runs better than $1/100$ (section 7).

### Assumed and untested

the step from acquired to landed (spark generation, and that everything hinted is bought),
the star mixes, Grand Masters (no runs), hint levels, and the priority policy run by run.

## How to read this document

### Labels

Every quantity carries one:

- **fitted**: estimated from the runs; the sample and method are given.
- **derived**: follows from the other parts by computation.
- **assumed**: a chosen rule or an outside figure the runs cannot test. Its source is named, and no evidence is
  claimed for it.

### Intervals

$x$ [$a$, $b$] is an estimate with its 95% interval (90% where marked): the range of values the data
are consistent with. A prediction outside it is contradicted by the data at the 5% level. The method depends on the
kind of estimate:

- a rate with one observation per run (a share of runs): the Wilson score interval;
- observed / predicted, or a rate, over rows that come several to a run (targets, race attempts, inheritance
  trials): the ratio estimator, its variance taken from each run's residual (the run's observed total minus its
  predicted total), which stays valid when one run's rows are correlated;
- a fitted parameter (a slope, a weight): estimate ± 1.96 run-clustered standard errors (the sandwich estimate, each
  run's contribution to the fit taken as one unit) where a run gives several observations; the profile-likelihood
  interval (every value whose fit is within 1.92 nats of the best) where a run gives one, as for $n$ in section 8.

### Data terms


- *Run*: one career from the Independent Training runs from uma-it, pooled; every run record is in the pool.
- *Row*: one observation within a run, such as one target in one career (a *run-target row*), one race attempt or one
  inheritance trial.
- *Observed / predicted*: over a set of rows, the successes seen divided by the successes the model expects (the sum
  of its chances). 1 means calibrated; above 1 the model under-predicts (it is pessimistic), below 1 it over-predicts
  (it is optimistic).
- *Log-likelihood*, *nats*, *free fit*, *in-sample* and *held out*: see *Reading the fit comparisons* below.

### Game terms


- *IT*: Independent Training, the career mode modelled.
- *Career*: one training run of one uma, the *trainee*. It ends as a *veteran*. *Outfit*: which version of a
  character is trained; some skills come with the outfit.
- *Scenario*: the career's ruleset: URA Finale, Unity Cup, Grand Concert, Trackblazer or Grand Masters.
- *Legacy*: a veteran used as a parent or grandparent of a later career; a career has six (two parents, four
  grandparents).
- *Spark* (factor): what a veteran passes on, each with 1-3 stars: *white* sparks of skills, *pink* of aptitudes,
  *blue* of stats, *green* of the unique skill.
- *Inspiration*: Classic and Senior April (the career's second and third years), when each legacy's sparks can fire. A
  white spark that fires gives its skill a hint; a pink raises that aptitude. At the start of a career the legacies'
  blue and green sparks fire and their pinks raise her aptitudes (section 4).
- *Hint*: raises a skill's *hint level* (1-5). A skill is *hinted* once it has a level; only hinted skills can be
  bought. *Gold* skills are the stronger versions of white ones; the targets come as white / gold pairs.
- *Affinity*: a compatibility score between the trainee and each legacy; it raises the chance a legacy's sparks fire
  (section 5).
- *Deck*: the six support cards a career trains with. A card hints skills from its *training pool* and through its
  events.
- *Rarity*: SSR, SR or R (R cards are left out of measured results).
- *Chain card*: a card whose events form a sequence of 2 or 3 *links*. *Pal* and *group* cards have their own events
  and no chain.
- *Hint frequency*, *specialty priority*: card effects (section 7).
- *Limit break*: 0-4, written LB0-LB4; it sets a card's effect levels.
- *Card-run*: one card in one run's deck.
- *Priority list*: up to 10 skills the player marks at the start of an IT career for IT to favour (section 9).
- *Marker skill*: a skill only one particular event can give, so seeing it among a run's hints shows that event fired
  (sections 8 and 10).
- *Target*: a skill one wants the new veteran to carry as a spark. It *lands* when it becomes a spark on the new veteran.
- *G1*: the top race grade. *Aptitude grades* S to G for distance and surface set race win chances (section 4).

### Where the numbers come from

The scripts in `research/` compute every number this document states, from the
runs, the game data or the other parts. `./build.sh research` runs them; they write `research/results.json`, and
`research/render.py` fills `docs/RESEARCH.template.md` to produce this file. Each conclusion the text draws is also a
test in those scripts (a likelihood-ratio test, an interval covering or excluding a prediction), reported as holding
or failing; a miss the text states is tested as a miss. The only numbers not computed are the assumed inputs
themselves. The run records are not in this repository; a clone renders from the committed results and recomputes
what needs only game data.

### Reading the fit comparisons

A model's *log-likelihood* (LL) is the sum, over every observation, of the natural
log of the probability the model gave to what was actually seen. The higher (closer to 0), the better the model
explains the data. A difference of $\Delta$ in LL ("$\Delta$ *nats*") means the better model made the observed data
$e^{\Delta}$ times as likely: 1 nat is a factor of 2.7, 5 nats about 150, 30 nats about $10^{13}$.

A *free* fit estimates each quantity separately from the data, with no formula tying them together: a free
distribution of $N$ has its own probability for every value, a free win rate is each cell's own share of wins, a free
divisor is fitted instead of fixed at 100. It is the most flexible model of its kind and serves as a yardstick: a
formula that explains the data about as well as its free version leaves nothing detectable unexplained.

A model always explains the data it was fitted to (*in-sample*) at least as well as a simpler one, because its extra
freedom also fits chance. So each comparison is one of three kinds, each with its own rule:

1. **Models with the same number of fitted parameters** (two rules, two tables): compared directly by LL on the same
   data.
2. **Is an extra parameter detectable?** The likelihood-ratio test: with $k$ extra parameters in a model that contains
   the simpler one, $2\Delta$ is compared with a $\chi^2$ distribution on $k$ degrees of freedom (Wilks 1938). One
   extra parameter needs $\Delta > 1.92$ to be significant at 5%; four need $\Delta > 4.74$. A detectable effect is
   reported as detectable, not as a better model.
3. **Does a model predict better?** This alone decides whether a form is adopted or said to fit better. The runs are
   split into five groups by run id, and each group is scored by the model fitted on the other four, so every run is
   predicted by a model that never saw it (*held out*). Two models are compared run by run: the summed difference of
   their held-out LL, its standard error, and $z$ = difference / standard error (Vuong 1989). $z > 1.96$ means one
   predicts unseen runs better beyond chance.

Every test's criterion is a fixed rule of this kind (a threshold, or an interval covering or excluding a prediction),
written into the script. A miss is reported as a miss.

### Targets

The model is the same for every skill. For any skill it predicts the chance that a career *acquires* it
(hints it, so it could be bought) from that career's own deck, lineage and trainee, not from what the player was
aiming for. So a prediction can be checked on any skill in any run, whatever its owner targeted. "Acquired" means
hinted, not bought, and each prediction uses the run's own deck and lineage, so players choosing decks for their own
goals does not bias the comparison.

Intent enters in one place. The model assumes every target is prioritised, which steers IT's event choices, and a
player chasing other skills does not prioritise these. Section 13 measures that effect: event-only hints of
non-prioritised targets fall well below the model.

The tests that score a fixed set of skills use eight common targets (the default set of the Legacy Loop Planner,
which this model was built for), eight skill pairs (white / gold): Uma Stan / Superstan, Nimble Navigator / No Stopping Me!, Playtime's Over!
/ See Ya Later!, Ramp Up / It's On!, Tail Held High / Tail Nine, Racing Spirit: Stamina, Slipstream / Perfect Spot!,
Risky Business / Nothing Ventured. Unity Cup guarantees the No Stopping Me! and It's On! golds. Section 13 also checks
a sample of every other skill a deck card or spark could give.

## 0. What is computed

The model gives one number per career: the chance that every target lands. It is used in two ways:

- *For a planned loop*, whose veterans are not trained yet: their sparks' stars are averaged (section 2) and their
  race wins come from base aptitudes (section 4).
- *For a career with real legacies*: their actual stars, G1 wins and parents are used.

### Terms

- $T$: the trainee. $P_1, P_2$: her parents. With the parents' own parents (her grandparents) they make six legacies,
  $l = 1..6$.
- $k = 1..K$: the targets.

$$P(\text{all}) = P(\text{every target lands as a spark on the veteran this career produces}).$$

A target lands in three steps: the trainee acquires the skill (it is hinted, so it can be bought), she buys it, and it
then becomes a spark. The sections take the parts in turn:

- 1-3, the inheritance side: spark generation (1), the star mix of legacy sparks (2), inheritance rolls (3);
- 4, race wins; 5, affinity; 6, loop structures, ranking and schedules;
- 7-11, the deck and event side: training hints (7), chain events (8), event choices and outcomes (9), random card
  events (10), conditional character events (11);
- 12, the composition of the parts into $P(\text{all})$;
- 13, its test against real careers.

## 1. Spark generation (assumed)

A target lands only if the skill, once bought, becomes a spark on the new veteran. This section gives the chance of
that step.

### Findings

- **Nothing is measured here.** The curve is a community figure (README, sources), and the runs cannot test it: the
  run records show which sparks a career inherited, not which sparks it produced, and the veterans they do show were
  chosen for their sparks.
- **That everything acquired is bought** is also **assumed** and untested.

### Terms

- $g_k$: the chance that target $k$, once bought, becomes a spark on the veteran.
- $g_0$: its base, 0.2 when the white skill is learned and 0.4 when the gold is.
- $n_k$: the number of ancestors (of six) carrying a spark of $k$.

### Model

The chance a learned skill becomes a spark (**assumed**):

$$g_k = g_0 \cdot 1.1^{\,n_k}, \qquad g_0 = 0.2 \text{ (white learned)}, \quad g_0 = 0.4 \text{ (gold learned)}$$

A bought gold sparks its white. In a running loop every target spark sits on all six legacies (**assumed**: a loop
keeps veterans that landed everything), so $n_k = 6$, $1.1^6 = 1.771561$ and $g = 0.3543$ (white) / $0.7086$ (gold).

## 2. Star mix of legacy sparks

Computed by `research/s02_stars.py`.

A spark's stars set its inheritance base (section 3), but for a planned loop the stars of each legacy's sparks
are not known yet. This section sets the star mix it assumes, which mix each scenario uses, and what averaging over stars
costs.

### Findings

- **The star mix is assumed**, a community figure. The runs cannot measure it: players choose legacies for their
  sparks, so an unselected veteran's mix cannot be read from them.
- **Grand Concert uses the higher mix**: most of its careers reach a rank score of 17,500 or more (79%). URA Finale,
  Unity Cup and Trackblazer use the lower one; Unity Cup is close to even.
- **Averaging over stars overstates the inheritance chance slightly**: by at most 0.57% for a target carried by
  all six legacies. With real legacies their actual stars are used, so this does not arise.

### Terms

- *Rank score*: a veteran's end-of-career score. Its *band* is below 17,500, or 17,500 and up.
- $s \in \{1, 2, 3\}$: a white spark's stars. $\pi = (\pi_1, \pi_2, \pi_3)$: the share of sparks with each.
- $\bar b$: the inheritance base per roll, averaged over stars.
- $r$, $q_k$: the chance of one roll and of at least one inspiration in a career (section 3).

### Model

The star mix is **assumed** (community figures). A legacy's white spark has $s$ stars with shares $\pi$ set by her rank
score: $0.50 / 0.45 / 0.05$ below 17,500 and $0.20 / 0.70 / 0.10$ from 17,500 up. For a planned loop the model does not know a
legacy's stars and uses the expected base per roll (**derived** from section 3's bases):

$$\bar b = 0.03\,(\pi_1 + 2\pi_2 + 3\pi_3) = 0.0465 \;(\text{below } 17{,}500), \quad 0.057 \;(17{,}500+).$$

### Evidence

*Which mix does each scenario use?* (**derived** from the runs.) Each scenario takes the band most of its careers
reach. A career's rank score is estimated with every skill point spent, over every completed run in the run list:

| Scenario | Runs | Median rank score | Share at 17,500+ | Band |
|---|---|---|---|---|
| URA Finale | 1,357 | 12,761 | 0.000 [0.000, 0.003] | below |
| Unity Cup | 3,155 | 17,428 | 0.467 [0.450, 0.484] | below |
| Grand Concert | 14,742 | 18,094 | 0.794 [0.787, 0.800] | 17,500+ |
| Trackblazer | 282 | 12,386 | 0.000 [0.000, 0.013] | below |

Grand Masters has no runs and is **assumed** 17,500+.

*What does averaging over stars cost?* (**derived**.) $\bar b$ is exact for one roll but not for two: a legacy's stars
are the same in both rolls, and by convexity $E[(1 - r)^2] \ge (1 - E[r])^2$, so averaging overstates $q_k$. On
the six real affinities of each of the 3,607 lineages in the runs, for a target carried by all six, it overstates $q$ by
at most 0.57% below 17,500 (mean 0.50%) and 0.33% at 17,500+.

## 3. Inheritance rolls

Computed by `research/s03_inheritance.py` (the roll's parts) and `research/s13_calibration.py` (the test on whole
careers).

Inheritance is one way a target gets hinted: at each inspiration, a legacy's spark of the skill may fire. This section
gives that chance, tests its parts on single-ancestor trials, and tests the whole on real careers.

### Findings

- **Each legacy carrying the target gets two rolls**, at the Classic and Senior inspirations. A roll's chance is a
  base set by the spark's stars, raised in proportion to the legacy's affinity.
- **Grandparents take the full base, as parents do**: a free weight on their roll fits 1.01 [0.98, 1.03].
- **Affinity raises the roll as the formula says**: a free slope fits 0.98 [0.95, 1.01], and each of its
  two parts, relation points and shared G1 wins, carries its stated weight.
- **On whole careers the chance is calibrated**: 0.989 [0.976, 1.001] of the model over the default targets,
  and it holds on non-target skills too.
- **Known misses**: two affinity bands (150-199 and 200+) and one band of predicted chance (0.40-0.60) lie outside
  their own intervals, though each set of bands holds jointly.

### Terms

- *Roll*: one inspiration's chance for one legacy's spark to fire. $r_{lk}$: that chance for legacy $l$ and target
  $k$.
- $b(s)$: the base chance of a spark with $s$ stars; $b = 0$ without a spark.
- $A_l$: legacy $l$'s affinity (section 5).
- $q_k$: the chance of at least one inspiration of $k$ in the career, over both rolls.
- *Trial*: one (ancestor, white skill spark, year 2 or 3) where exactly one of the six legacies carries the skill, so
  that year's inspiration of it can only come from her. It *passes* when the spark fires that year.
- *Carriers*: the legacies with a spark of the target.
- *Self-grandparent*: a grandparent of the trainee's own character (sections 5 and 6).

### Model

Per legacy $l$, target $k$ and roll:

$$r_{lk} = \min\!\left\{1,\; b(s_{lk})\left(1 + \frac{A_l}{100}\right)\right\}, \qquad
q_k = 1 - \left[\prod_{l=1}^{6} (1 - r_{lk})\right]^{2}$$

The square is the two rolls, Classic and Senior April. The base by spark type and stars ($1/2/3\star$) is **assumed**
(community figures); the skill-white bases are tested below:

| Spark | Skill / scenario white | Race white | Green (unique) | Pink | Blue |
|---|---|---|---|---|---|
| $b$ | 3 / 6 / 9% | 1 / 2 / 3% | 5 / 10 / 15% | 1 / 3 / 5% | 70 / 80 / 90% |

The odds use skill whites only. Great-grandparents count for nothing (**assumed**, matching the game's two-generation
lineage; the runs show no great-grandparents). For a planned loop $b(s_{lk})$ is replaced by $\bar b$ (section 2).

### Evidence

The roll's parts are **fitted** on 30,368 parent and 69,840 grandparent trials of ordinary ancestors (the
self-grandparent is section 5's).

*Do grandparents take the full base?* Yes. A free weight on the grandparents' roll fits 1.01 [0.98, 1.03],
and the full base beats a halved one by 1,327 nats: 6,308 passes observed, 6,268 predicted with the full
base, 3,134 with half. Parents read 0.982 [0.96, 1.01] of the model.

*Does affinity scale the roll as $(1 + A/100)$?* A free slope on $A$ fits 0.98 [0.95, 1.01], and the term beats
no affinity by 3,760 nats. Its two parts, each fitted with the other fixed: relation points 0.96
[0.87, 1.05]; $3 \times$ shared G1 wins 0.97 [0.93, 1.01] (99,502 trials with shared wins).

*Do the bases by stars hold?* Observed / predicted is 0.980 [0.94, 1.02] at 1★ (27,826 trials),
0.995 [0.97, 1.01] at 2★ (63,864) and 1.013 [0.97, 1.06] at 3★ (8,518).

*Does it hold at every affinity?* Jointly, yes ($\chi^2$ = 10.0 on five bands, $p$ = 0.075), but two bands lie
outside their own 95% interval:

| Affinity | Trials | Observed / predicted |
|---|---|---|
| 0-49 | 9,666 | 1.003 [0.932, 1.073] |
| 50-99 | 60,546 | 1.006 [0.981, 1.032] |
| 100-149 | 1,180 | 0.994 [0.855, 1.132] |
| 150-199 | 4,040 | 1.079 [1.006, 1.152] |
| 200+ | 24,776 | 0.970 [0.944, 0.995] |

*Does the whole hold on real careers?* Observed / predicted inspiration on 3,348 careers (section 13):

- all 26,784 run-target rows: 0.989 [0.976, 1.001];
- by the number of carriers, 1 to 6: 0.956-1.001, every interval covering 1;
- per roll: years in which an inspiration of the skill fired, 0.988 of the model's expectation (1.69 standard
  deviations below it, run-clustered; inside its 95% band); legacy firings, against their expectation $2\sum_l r_l$,
  0.989;
- 43,244 rows of non-target skills: 0.993 [0.98, 1.01].

*Does it hold at every predicted chance?* Grouped by predicted $q$, the groups are calibrated jointly ($\chi^2$ =
5.7 on 5 groups, $p$ = 0.34). One lies outside its own 95% interval, 0.40-0.60: 0.489
[0.474, 0.504] observed against 0.505 predicted.

| Predicted $q$ | Rows | Observed (mean predicted) |
|---|---|---|
| 0 | 8,723 | 0.002 |
| above 0 to 0.20 | 2,490 | 0.156 (0.16) |
| 0.20-0.40 | 3,163 | 0.298 (0.31) |
| 0.40-0.60 | 4,098 | 0.489 (0.51) |
| 0.60-0.80 | 5,786 | 0.698 (0.70) |
| 0.80-0.95 | 2,524 | 0.834 (0.84) |

## 4. Race wins

Computed by `research/s04_races.py`.

Race wins matter through affinity: two umas that won the same G1s inherit more from each other (section 5). This
section measures how a trainee's chance of winning a race depends on her aptitudes and on how many races she has run
in a row, and tests a widely used community table of that chance.

### Findings

A race's win chance depends on the trainee's grades for its distance and surface (written as a pair, better grade
first) and on how many races she has run in a row. Over 76,402 attempts at the 34 single-race G1s, the observed win
rates are (the community table's chance in parentheses; grades include those raised by inherited aptitude sparks; S is
shown as A, since both have the same chance; a dot where a cell has under 30 attempts; grade pairs with 200 or more
attempts):

| Races in a row | AA | AB | AC | BB | AD | BC | AE | BD | BE | CD | AF | AG |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 race | 100% (100%) | 100% (100%) | 89% (90%) | 90% (90%) | 80% (80%) | 76% (80%) | 67% (70%) | 70% (70%) | 58% (60%) | 57% (60%) | 47% (50%) | 20% (20%) |
| 2 races | 100% (100%) | 100% (100%) | 91% (90%) | 91% (90%) | 81% (80%) | 81% (80%) | 62% (70%) | 70% (70%) | 51% (60%) | 67% (60%) | 54% (50%) | 20% (20%) |
| 3 races | 100% (100%) | 97% (95%) | 85% (85%) | · (85%) | 75% (75%) | · (75%) | 59% (65%) | · (65%) | · (55%) | · (55%) | 62% (45%) | 13% (15%) |
| 4 races | 92% (90%) | 81% (80%) | 77% (70%) | · (70%) | 49% (60%) | 62% (60%) | · (50%) | 62% (50%) | · (40%) | 21% (40%) | · (30%) | · (0%) |
| 5 races | 86% (80%) | 72% (70%) | 56% (60%) | · (60%) | 69% (50%) | · (50%) | 36% (40%) | · (40%) | · (30%) | · (30%) | · (20%) | · (0%) |
| 6 or more | 63% (60%) | 70% (50%) | · (40%) | · (40%) | · (30%) | · (30%) | · (20%) | · (20%) | · (10%) | · (10%) | · (0%) | · (0%) |

- **The community table predicts these rates**: observed / predicted 1.000 [0.999, 1.002] over all
  attempts, and calibrated within every scenario.
- **Grades must include inherited aptitude sparks.** With the trainee's base grades the table predicts too few wins
  (observed / predicted 1.079 [1.075, 1.083], 1.83 where base grades give a chance of 70 or less). Pink sparks
  raise grades at the start of a career and at each inspiration, from that April on.
- **Races in a row lower the chance; wins do not.** The penalty counts races entered, and the previous race's result
  does not change the next chance beyond the table.
- **Two departures**: goal races (the races a career requires) from the fourth race in a row win more than the
  penalty allows
  (1.13 [1.11, 1.15]), and chances of 70 or less win slightly less than predicted (0.96
  [0.94, 0.98]).

### Terms

- *Grade*: the trainee's aptitude grade, S to G, for the race's distance and surface; $c_{\text{dist}}$ and
  $c_{\text{surf}}$ are what each grade costs.
- *Streak*: consecutive turns with a race; $j$ is a race's position in it and $\mathrm{PEN}_j$ its penalty.
- *Table's chance*: $110 - c_{\text{dist}} - c_{\text{surf}} - \mathrm{PEN}_j$, in percent, before it is clamped to
  0-100.
- $p$: the chance of winning one race she enters. $P_a(r)$: uma $a$'s chance of winning race $r$ at least once over her
  schedule.
- *Attempt*: one race entered. *Goal race*: one her career requires; *chosen race*: one IT chose to enter.
- *Base aptitudes*: her grades before any spark. *Raised grades*: her grades after her legacies' pink sparks (the rule
  in the evidence below).

### Model

A trainee's chance of winning a race she enters is a community table (**assumed**, tested below):

$$p = \mathrm{clamp}\big(110 - c_{\text{dist}} - c_{\text{surf}} - \mathrm{PEN}_j,\ 0,\ 100\big) / 100,$$

$c$ = 0 for S/A, 10 B, 20 C, 30 D, 40 E, 60 F, 90 G. $\mathrm{PEN}_j$ = 0, 0, 5, 20, 30, 50 for the $j$-th race in a
row (any race, goals included; a turn without a race resets the streak). Stats and training are not modelled. Over a
planned schedule,

$$P_a(r) = 1 - \prod_{\text{attempts}} (1 - p);$$

section 5 turns these into race affinity.

### Evidence

The runs record every race a career ran: 76,402 attempts at the 34 single-race G1s (22,387 of
them goals), of which 93.1% were won.

*Base aptitudes or raised grades?* Raised grades.

- On base aptitudes (as for a planned loop), the table predicts 86.3%: observed / predicted
  1.079 [1.075, 1.083], and 1.83 where the table gives 70 or less (14,047 attempts).
- With her grades raised by the aptitude-spark rule (`docs/MODEL.md`: the legacies' pink stars at the
  start, at most to A; then each pink the run records firing at a Classic or Senior inspiration, from that April on),
  it predicts 93.1%: observed / predicted 1.000 [0.999, 1.002], 2,119 nats better than
  base aptitudes, and calibrated in every scenario.

*Is the streak penalty real?* Yes: without it the fit is 1,737 nats worse.

Both comparisons need one adjustment. The table calls some races certain (0% or 100%), and a handful went the other
way, which gives the table as stated a log-likelihood of $-\infty$. So every model compared has its 0% and 100% pulled
in to a fitted floor and ceiling, the same two parameters in each; the comparisons are then between models with equal
numbers of fitted parameters (rule 1 of *How to read*).

*Does the table hold at every chance?* On raised grades:

| Table's chance | Attempts | Observed / predicted |
|---|---|---|
| 110 | 50,681 | 1.002 [1.001, 1.002] |
| 100 | 11,102 | 1.004 [1.002, 1.007] |
| 90 | 4,674 | 1.002 [0.992, 1.012] |
| 80 | 3,169 | 1.002 [0.984, 1.020] |
| 70 or less | 6,776 | 0.959 [0.937, 0.982] |

*When does a pink spark take effect?* From the April of its inspiration. The records say which inspiration fired a
pink, and inspirations sit at fixed turns. On the 372 Senior January-March attempts where the timing matters,
0.739 [0.691, 0.787] were won, against 0.755 predicted with the pink counted from April and 0.865 counted
from the year's start.

*How many attempts is each cell of the table in Findings?*

| Races in a row | AA | AB | AC | BB | AD | BC | AE | BD | BE | CD | AF | AG |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 race | 24,982 | 5,875 | 2,499 | 87 | 1,407 | 192 | 1,268 | 134 | 97 | 76 | 1,196 | 827 |
| 2 races | 17,142 | 3,249 | 1,250 | 116 | 956 | 265 | 627 | 160 | 222 | 144 | 413 | 280 |
| 3 races | 5,508 | 1,273 | 479 | 13 | 179 | 18 | 204 | 23 | 19 | 1 | 79 | 31 |
| 4 races | 2,081 | 291 | 158 | 19 | 53 | 56 | 28 | 32 | 19 | 34 | 6 | 3 |
| 5 races | 707 | 335 | 41 | 8 | 35 | 4 | 45 | 7 | 3 | 3 | 12 | 10 |
| 6 or more | 261 | 79 | 4 | 0 | 4 | 0 | 2 | 0 | 1 | 0 | 6 | 3 |

*Does the previous race's result matter?* No. Each attempt's chance combines both effects in one formula: her grades
for the race's distance and surface, and her position in the current streak. The penalty counts races in a row, not
wins: within a streak, the previous race's result does not move the next chance beyond the table (observed /
predicted, attempts):

| Race in a row | After a win | After a loss |
|---|---|---|
| 2nd | 0.999 [0.997, 1.002] (23,686) | 0.996 [0.982, 1.010] (1,614) |
| 3rd | 1.003 [0.999, 1.007] (7,346) | 0.991 [0.969, 1.013] (487) |
| 4th or later | 1.025 [1.010, 1.039] (3,918) | 1.093 [1.041, 1.145] (472) |

From the 4th race on both read above 1: that is the goal-race excess below.

*Is one formula enough?* Yes, though not exactly. Giving every (chance, streak position) cell its own win rate, read
straight from its attempts (63 cells), explains the attempts 69 nats better than the table's one formula
in-sample. That is beyond the 40 nats chance allows for its 61 extra parameters if attempts were independent,
but they are not: one trainee's stats carry through her races. Held out, the per-cell rates do not predict unseen
runs better than the table (78 ± 43 nats, $z$ = 1.81). So the table stands, with two departures
clear in their own intervals:

- goal races from the fourth race in a row win more than the penalty allows (1.13 [1.11, 1.15], 1,339
  attempts), while chosen races there match (0.99 [0.97, 1.00]);
- chances of 70 or less win a little less than predicted (0.959 [0.937, 0.982]).

The test is conditional on IT having chosen each non-goal race; stats are not modelled.

## 5. Affinity

Computed by `research/s05_affinity.py` (the self-grandparent fit).

A legacy's affinity $A_l$ scales her inheritance roll, $r = b\,(1 + A_l/100)$ (section 3). This section defines how
$A_l$ is computed and fits the one rule the game data leave open.

### Findings

- **Affinity adds the characters' relation points (game data) and 3 for each single-race G1 both umas
  won.** The 3 per shared win is **assumed** as the game's rule; section 3 finds the roll scales with it as stated
  (slope 0.97 [0.93, 1.01]).
- **A trainee who is her own grandparent gets no trio points from it.** The game data do not say what a trio with
  one character twice counts. Counting nothing predicts the self-grandparent's inspirations in the runs; counting it
  as the pair is about 19 nats worse.

### Terms

- *Relation group*: a set of characters to which the game data give relation points.
- $\mathrm{pair}(a,b)$, $\mathrm{tri}(a,b,c)$: the summed relation points of the groups containing all the characters
  named.
- $P_a(r)$: uma $a$'s chance of winning race $r$ at least once (section 4).
- $o(a,b)$: the expected number of single-race G1s both $a$ and $b$ win.
- $P$: a parent; $P'$: the other parent; $\mathrm{par}(P)$: $P$'s own parents; $G$: one of them.
- $\kappa$: a free multiplier for the self-grandparent's trio, $\mathrm{tri}(T,P,T) = \kappa\cdot\mathrm{pair}(T,P)$.

### Model

*Relation points* (game data). When a character appears twice, as in $\mathrm{tri}(T, P, T)$, the data do not say what
the group counts; the model counts only distinct characters, so $\mathrm{tri}(T, P, T) = 0$ (**fitted**, below).

*Race affinity*: +3 per single-race G1 both won, counted per race (relocated venues are the same race); multi-race
titles count 0 (**assumed** as the game rule; the weight 3 is tested in section 3). With schedules fixed in advance and
two careers independent (**assumed**), the expected shared wins of $a$ and $b$ are

$$o(a,b) = \sum_r P_a(r)\,P_b(r),$$

with $P_a(r)$ from section 4 on the uma's base aptitudes.

*Per-legacy affinity.* For parent $P$ and grandparent $G \in \mathrm{par}(P)$:

$$A_P = \mathrm{pair}(T,P) + \mathrm{pair}(P_1,P_2) + 3\,o(P,P') + \sum_{G \in \mathrm{par}(P)} \big[\mathrm{tri}(T,P,G) + 3\,o(P,G)\big]$$
$$A_G = \mathrm{tri}(T,P,G) + 3\,o(P,G)$$

In a loop $P'$ is also one of $P$'s own parents, so $o(P, P')$ counts twice in $A_P$.

### Evidence

*What does a self-grandparent's trio count?* ($G = T$, section 6.) $\mathrm{tri}(T,P,T) = 0$ is **fitted** on
114,240 inspiration trials attributable to a single ancestor. Self-grandparent passes: 610 observed, 621.6
predicted with 0, 721.2 with $\mathrm{tri}(T,P,T) = \mathrm{pair}(T,P)$ (about 19 nats worse). Run-clustered, the observed count lies
−0.48 standard deviations from the prediction with 0 and −4.56 from the one with the pair: 0 is kept and the pair
rejected. Allowing any multiplier $\kappa$, the best fit is $\hat\kappa = 0$, at its lower bound.

*Does the formula as a whole hold?* Section 3 tests it on the runs' real lineages and shared wins: the roll scales with
this $A$ at slope 0.98 [0.95, 1.01].

## 6. Loop structures, ranking and schedules

Computed by `research/s06_loops.py`.

A breeding loop is three or four umas bred from each other in turn. This section defines a loop's lineages and how a
loop is scored, and bounds what a different choice of race schedules could gain.

### Findings

- **A four-uma loop has 12 lineages, a three-uma rotation 3.** In a rotation the trainee is her own grandparent,
  and that occurs in practice: 1,087 of 3,610 runs have a grandparent of the trainee's own character.
- **A structure is ranked by its lineages' average odds**: the long-run rate of a player who trains every member in
  turn. This is a definition of the score, not a claim.
- **Every pair of umas carries the same weight**, so a loop's race schedules can be chosen for the most expected shared
  wins over all its pairs.
- **Scheduling for shared wins instead of odds costs little**: no schedule could raise a reference loop's odds by
  more than 2.02%, and no listed runner-up could overtake its winner.

### Terms

- *Lineage* $(T, P_1, P_2, X)$: a trainee, her parents, and $X$; each parent was bred from the other parent and $X$.
- *Structure* $S$: a loop or rotation; $\Lambda_S$: its set of lineages; $|\Lambda_S|$: their number.
- $P_\lambda(\text{all})$: section 0's $P(\text{all})$ for lineage $\lambda$; $\bar P_S$: the structure's score.
- $o(a,b)$: expected shared G1 wins (section 5); $o_{\max}(a,b)$: its value with every offering of every G1
  attempted and no streak penalty.
- *Pair ceiling*: the highest expected overlap a pair can reach over feasible schedules, found by search.
- *Reference loops*: the best loops and decks found for a fixed set of reference setups (rosters and scenarios).

### Model

A four-uma loop has 12 lineages (each member trains from two of the other three); a three-uma
rotation has 3, with $X = T$ (the trainee's own earlier veteran). A structure $S$ scores

$$\bar P_S = \frac{1}{|\Lambda_S|} \sum_{\lambda \in \Lambda_S} P_\lambda(\text{all}),$$

the long-run rate of a player who trains every member in turn. It is a definition of the score, not a
claim: a player who re-trains only the best trainee would score a structure by its best lineage.

*Race weights* (**derived**). In one lineage $o(P_1,P_2)$ enters the six affinities 6 times, and $o(P_1,X)$ and
$o(P_2,X)$ twice each. Summed over the 12 loop lineages, every pair gets $6\cdot 2 + 2\cdot 4 = 20$; over the 3
rotation lineages, $6 + 2 + 2 = 10$. Both are uniform over pairs, so a structure's schedules maximise
$\sum_{a<b} o(a,b)$.

### Evidence

*Does the rotation occur?* Yes: 1,087 of 3,610 runs have a grandparent of the trainee's own character, and
0 have a parent of her character.

*What could a different schedule gain?* (**derived**, on the reference loops.) Schedules are chosen for $\sum_{a<b} o(a,b)$, not for $\bar P_S$. $\bar P_S$ rises with every $o$: $o$ enters each legacy's affinity with
weight +3, and $r$, $q$ and every target's odds are nondecreasing in their inputs. So, with a loop's decks fixed, any
schedule (including one chosen for $\bar P_S$ directly) scores between the odds found and the odds with every pair at
its highest possible overlap. Over 40 reference results, whose odds an independent implementation reproduces to within
2e-15:

- the shared wins found are worth 31-85% over no race affinity;
- the independent ceiling $o_{\max}(a,b) = \sum_r P^{\max}_a(r)\,P^{\max}_b(r)$ caps the gain from a different
  objective at 17.5%. It is far from reachable (the schedules found reach about 68% of it) and cannot
  settle the ranking: 8 of 12 listed runner-up loops reach their winner under it;
- the pair ceilings come from the same search code that chose the schedules, so they are not an independent check, but they sit between
  $o$ and $o_{\max}$ on every pair. They put the ceiling only 0.54-2.02% above the odds found (median 0.90%), and no listed runner-up's ceiling
  reaches its winner (0 of 12).

## 7. Training hints

Computed by `research/s07_hints.py`.

A deck card's training is the main way targets get hinted. When a card joins a training session it can show a hint,
which raises one skill from the card's list of hintable skills (its *pool*) by a hint level or more.

### Findings

- **A card gives about the same number of training hints every career, each a random skill from its pool.** A typical card in
  the runs' decks (hint frequency +20%, specialty priority 35) gives about 3.2 hints per career in Unity
  Cup, 2.4 in Grand Concert, 2.1 in Trackblazer and 1.8 in URA Finale. Each hint picks a skill uniformly from the pool;
  a skill already at hint level 5 is no longer picked.
- **Hint frequency scales that number directly**: a card with Hint Frequency +40% gives 40% more hints.
- **Specialty priority also raises it.** The game describes Specialty Priority only as making the character join her
  preferred training type more often; in the runs, cards with more of it give more hints, by an amount that differs
  between scenarios. The effect predicts careers it was not fitted to, but it is an association: across limit breaks
  of the same card it is the same size without being significant, so it is not shown to be a cause.
- **Scenarios differ**: each has its own base (table below). The specialty weight differs too: fitted, about
  0.35 in Unity Cup and 0.37 in URA Finale against 0.10 in Grand Concert and 0.11 in Trackblazer, and a weight per scenario
  predicts careers it was not fitted to better than one shared weight. Nothing in the game's description of Specialty
  Priority depends on the scenario, so this weight is an association measured in each scenario's careers, not a game
  constant; why it differs is not tested here.
- **Known misses**: SR cards give slightly fewer hints than SSR cards with the same effects, decks with more pals
  slightly more, and a skill also hinted by inheritance is hinted by training less than predicted.

### Terms

- *Hint level*: a skill's hints add up to level 5, the cap.
- *Pool*: the skills a card's training can hint; $n$ = their number.
- $\iota$: hint levels one hint gives (the card's hint increment, +1 to +5). A skill leaves the pool after
  $c = \lceil 5 / \iota \rceil$ hints (5, 3, 2, 2, 1 for +1 to +5).
- $hf$: the card's Hint Frequency in percent; $\mathit{sp}$: its Specialty Priority. Both at maximum limit break,
  unique effects included.
- $h$: training hints the card gives in one career.
- $\beta_s$, the *base*: hints per career in scenario $s$ for a card with neither effect.
- $d_s$, the *specialty weight*: how much more hinting goes with each point of specialty priority in scenario $s$'s
  careers, relative to the base. A fitted parameter, not a game value.

### Model

$$h = \beta_s \left(1 + \frac{hf}{100}\right)\left(1 + d_s\,\frac{\mathit{sp}}{100}\right).$$

$h$ is rarely a whole number; a career gets $\lfloor h \rfloor$ or $\lfloor h \rfloor + 1$ hints, the latter with chance
$f = h - \lfloor h \rfloor$. The chance that none of $k$ targets in one card's pool is hinted is joint over them:

$$M(k, n, h) = (1 - f)\, m(k, n, \lfloor h \rfloor) + f\, m(k, n, \lfloor h \rfloor + 1), \qquad
m(k, n, t) = \left(1 - \frac{k}{n}\right)^{t} \ (t \le c),$$

and an exact Markov chain over the other skills' hint counts once a skill can reach the cap ($t > c$).

### Fitted values

These are **fitted** by maximum likelihood on every hint row, a row being one pool skill of one card in one
run, on maximum-limit-break cards, rows where no inspiration hinted the skill, R cards out; 61,856 rows from
10,712 card-runs; the Runs column counts the runs that add a row; *Adopted*: the values the model uses:

| Scenario | $\beta$ | $d$ | Adopted $\beta$ / $d$ | Rows | Runs |
|---|---|---|---|---|---|
| Unity Cup | 2.377 | 0.349 | 2.36 / 0.36 | 14,047 | 915 |
| Grand Concert | 1.962 | 0.100 | 1.97 / 0.08 | 34,481 | 1,773 |
| URA Finale | 1.369 | 0.366 | 1.34 / 0.39 | 10,958 | 631 |
| Trackblazer | 1.700 | 0.108 | 1.70 / 0.11 | 2,370 | 99 |
| Grand Masters | no runs | | 1.86 / 0.26 (**assumed**: one $\beta$ and one $d$ over every scenario) | | |

### Evidence

*Specialty priority.* Without the specialty term, cards with more specialty priority give more hints than predicted;
with it, the excess is gone:

| Specialty priority | Rows | Observed / predicted, without the term | with it |
|---|---|---|---|
| 0 | 18,870 | 0.951 | 1.003 |
| 1-29 | 583 | 0.965 | 0.991 |
| 30-59 | 30,188 | 1.005 | 0.995 |
| 60+ | 12,215 | 1.067 | 1.005 |

- It predicts careers it was not fitted to: held out, the term gains 36.7 ± 4.9 nats, $z$ = 7.6; fitted on the earlier
  half of the runs, it predicts the later half better (12.9 ± 3.3 nats, $z$ = 3.9).
- A weight per scenario predicts better than one shared weight (8.3 ± 2.5 nats, $z$ = 3.4).
- Split by scenario and specialty band, the 13 cells with 200 or more rows are jointly consistent with the
  model (run-clustered $\chi^2$ = 7.4, p-value 0.88).
- One shared weight stays between 0.16 and 0.28 on either half of the runs, on SR or SSR cards alone, and
  without any one of the eight most-used cards.
- Cause or association: the same card at different limit breaks has different specialty priority but is otherwise
  the same card. Across 50 such cards (a base per card and scenario) the weight is 0.41, the same size as
  across cards, but the gain (1.5 nats) is not significant, so the data do not show the effect is caused by
  specialty priority itself.

*Hint frequency's slope.* The $1/100$ in $(1 + hf/100)$ is **assumed** (a card's +40% read as 40% more hints) and
tested: on the adopted model a fitted slope ($hf/89$) is detectable in-sample but does not predict unseen careers
better (1.1 ± 2.1 nats, $z$ = 0.5).

*The form.* Fixed on every card at any limit break, R cards included, without the specialty term
(93,928 rows from 3,610 runs):

| Question | Evidence |
|---|---|
| Rate form, LL on 93,928 rows | $1 + hf/25$ over options −61,672.2; $1 + hf/25$ over skills −60,105.9; constant −58,710.8; $\beta(1 + hf/100)$ −58,098.4; per-scenario $\beta(1 + hf/100)$ −57,642.8; free divisor ($c = 79$) −57,624.8. Held out, per-scenario $\beta(1 + hf/100)$ predicts unseen runs better than the constant (1067.3 ± 28.3, $z$ = 37.7) and the $hf/25$ forms (4027.6 ± 70.9, $z$ = 56.8; 2461.4 ± 52.6, $z$ = 46.8) |
| Skills only, not skills + stat option | in 54 of 77 (card, $hf$) cells the draw-count variance is below the binomial floor a drawn stat option implies |
| Uniform draws with repeats | repeat rate $P(k \ge 2 \mid k \ge 1)$: 0.143 observed, 0.140 two-point count, 0.192 Poisson count; distinct skills 1/2/3/4 per card-run 0.152 / 0.555 / 0.253 / 0.038 observed, 0.126 / 0.554 / 0.274 / 0.044 model |
| Covariates | per-scenario bases +456 nats (form study; held out 454.3 ± 19.0, $z$ = 23.9). On top of the adopted rate (61,856 rows): card type +0.8 nats (4 parameters), initial bond +0.0, pool size +1.8, none significant; rarity (SR against SSR) +5.3 and pals per deck +14.0 are detectable and each predicts unseen runs better (held out 4.7 ± 1.8, $z$ = 2.6; 13.8 ± 3.1, $z$ = 4.5): known misses, factors 0.954 for SR and 1.033 per pal |
| Lv-5 cap | beats drawing with replacement by 20.2 nats with no parameter; obs / pred by draws to Lv 5, replacement vs cap: 5 draws (31,144 rows) 0.963 / 0.972, 3 (2,711) 0.955 / 0.962, 2 (32,818) 1.035 / 1.033, 1 (409) 1.223 / 0.970 |
| Joint draws over targets | 176,927 pairs of no-spark pool skills on one card-run (Trackblazer left out): $P(\text{both})$ 0.0798 observed, 0.0794 joint, 0.1090 independent; $P(\text{neither})$ 0.4440 vs 0.4419 |
| Pool size | obs / pred 0.92-1.15 for every $n$ with at least 150 rows |
| Per-draw increment | 24,815 of 24,935 single-source levels are multiples of $\iota$, capped at 5 |
| $hf$ by card level | floored linear interpolation −52,233.7, step −52,240.7, max-limit-break $hf$ for all −52,255.1 |

*Other sources of the same skill.* The fit uses rows where no inspiration hinted the skill. Where something else also
hinted it, the card's own hint is seen this often (scored against the adopted rate on maximum-limit-break cards, R
cards out):

| Other source of the skill | Rows | Observed | Predicted | Ratio |
|---|---|---|---|---|
| none | 54,415 | 0.342 | 0.338 | 1.01 |
| inspiration | 6,194 | 0.265 | 0.352 | 0.75 |
| inspiration + card event | 316 | 0.196 | 0.343 | 0.57 |
| card event | 1,597 | 0.326 | 0.334 | 0.98 |
| other card's training | 2,836 | 0.343 | 0.359 | 0.96 |

A skill hinted by inspiration is hinted by training less than predicted (not modelled). It is not that such a skill
leaves the card's pool: with two pool skills hinted by inspiration, the rest are hinted at 0.362, against
0.332 if nothing leaves and 0.450 if they leave.

*Prioritised skills.* 532 runs show a priority list; prioritised skills are hinted by training at 0.343 against
0.318 predicted (470 rows): no effect, as the model assumes.

Against real careers (section 13), training-pool hints read 0.988 of the model.

## 8. Chain events

Computed by `research/s08_chains.py`.

A chain card hints its skills through a fixed sequence of events, so how far a chain gets depends on how many chain
events a career gets and how they are shared among the deck's chain cards. This section fits both.

### Findings

- **A career gets a random number of chain events, Binomial(21, 0.35)** (mean 7.35), the same for every deck and
  scenario. No alternative predicts unseen runs better. The data do not single out the binomial family; what the
  model uses is the distribution.
- **Each event goes to a chain card in proportion to its remaining links.** This fits 66 nats better than equal shares
  per card.
- **Pal events do not use up chain events**: decks with more pals do not get fewer.
- **Known misses, causes unknown**: in crowded decks early links run short and more chains never start than
  predicted; final links fit pooled, but individual cards differ far beyond chance.

### Terms

- $N$: the number of chain events in one career.
- *Link*: one event of a chain; links fire in order. The *final link* completes the chain; the others are *early
  links*.
- $L_i$: card $i$'s chain length (SSR 3, SR 2; a filler slot counts as a 2-link SR). $d_i$: its links fired so far.
- *Slot*: a link not yet fired. $w_i$: card $i$'s remaining slots while its chain runs.
- *Stopped chain*: one an outcome ends early. $q_l$: the chance link $l$ ends it; $\sigma$: the link where it stops.
- *Crowded deck*: one with many chain cards, so few pals.
- Binomial($n$, $p$): the number of successes in $n$ chances, each succeeding with probability $p$; mean $np$,
  variance $np(1-p)$.
- *Under-dispersed*: with variance below the mean. The *Conway-Maxwell-Poisson* is another two-parameter count
  distribution that can be under-dispersed; the *Poisson* has variance equal to its mean.
- *Marker allowance*: the chance that a fired link's marker skill goes unseen.

### Model

A run gets $N$ chain events, the same for every deck and scenario (**fitted**; tested against
alternatives in-sample and held out, below):

$$N \sim \mathrm{Binomial}(21, 0.35), \qquad P(N = e) = \binom{21}{e}\, 0.35^{e}\, 0.65^{21-e}, \quad E N = 7.35,\ \mathrm{Var}\, N = 4.78.$$

Each event advances one non-pal chain card by one link. Card $i$ is picked with

$$P(i) = \frac{w_i}{\sum_j w_j}, \qquad w_i = L_i - d_i \text{ while running, } 0 \text{ once stopped},$$

that is, uniformly over the remaining chain slots (**fitted** against the alternative, below). Pal and group cards take
no slot (**assumed**, supported below). Without stop risk, $P(\text{all chains complete}) = P(N \ge \sum_i L_i)$; for
two 3-link cards, $P(N \ge 6) = 0.80$.

*Stop risk* (**derived** from the event data and the choice rule, section 9). If IT's default choice at link $l$ ends
the chain on $k$ of its $n$ outcomes, $q_l = k/n$, and the stopping link $\sigma$ has
$P(\sigma = l) = q_l \prod_{l' < l} (1 - q_{l'})$. A stopped chain's slots return to the pool. The model enumerates
the exact distribution of final chain states (no simulation); an independent recomputation of the model's odds
agrees to 2e-15 on every reference loop (section 6).

### Evidence

*How is a chain's progress read?* From marker skills, on 3,424 runs (the pal-count table below). A link is observable
only if every choice and outcome of its event carries a skill appearing in no other event of that card. A credit
naming two sources ("A or B event") counts as unknown when B is another deck card whose events can give the marker, and
so does a marker listed only under inspiration. $N$'s distribution is the one that best explains progress over all
decks.

The fit allows a fired link's marker to go unseen with chance 0.03 (**assumed**, a measurement allowance). Without it,
a run in which a card's later link shows while an earlier one does not has likelihood 0. Since links fire in order,
those runs measure the rate: of 1,149 earlier links implied fired by a later one, 24 show no marker, 0.021
[0.012, 0.030]. The cause is not known. The fit does not depend on it: with allowances from 0.01 to 0.045, or
with none on the 3,402 runs that show no such case, the best $p$ at $n = 21$ is 0.34-0.35.

*Which distribution for $N$?* In-sample, against the free distribution:

| Form for $N$ | Result |
|---|---|
| free distribution | mean 7.28, variance 4.47 |
| Binomial(21, 0.35) | 11.4 nats behind the free distribution with 14 fewer parameters (not rejected, p-value 0.06) |
| Binomial($n$, $p$), both fitted | best at $n$ = 18, $p$ = 0.406: 8.0 nats behind (p-value 0.31); $n$ is only known to lie in 16-20 |
| Conway-Maxwell-Poisson | 10.1 nats behind (p-value 0.12) |
| Binomial(15, 0.48) | 15.9 nats behind |
| Poisson(7.1) | 87.8 nats behind (variance = mean, too wide; rejected) |
| per scenario | means Unity Cup 7.57, Grand Concert 7.21, URA Finale 7.06, Trackblazer 7.03; +27.7 nats for 48 extra parameters (not significant) |

Why a binomial: the free fit's variance is 0.61 of its mean, so $N$ is under-dispersed, which rules out the
Poisson and every over-dispersed count. A binomial (a fixed number of chances, each taken with the same probability)
is the simplest under-dispersed count, but the data do not single it out: the Conway-Maxwell-Poisson fits about as
well. What the model uses is the distribution of $N$, not the family behind it.

How $n$ and $p$ are read: fitting both $n$ and $p$, the runs fix the mean (7.30 at the best $n$) and, more
loosely, the spread, so $n$ and $p$ trade off along a ridge. Re-fitting $p$ at each $n$, every $n$ from
16 to 20 lies within 1.92 nats of the best (the 95% interval), so the best single pair is not itself a finding. The
adopted $n = 21$ lies just outside that interval in-sample (2.95 nats below the best $n$; variance
$21 \cdot 0.35 \cdot 0.65 = 4.78$ against 4.47 free).

*Does that in-sample preference survive held out?* No. Each of five groups of runs is scored by every form fitted on
the other four, and each form is compared run by run with Binomial(21, 0.35):

| Form, fitted on the other four groups | Held-out nats over (21, 0.35) ± SE | $z$ |
|---|---|---|
| free distribution | −1.3 ± 5.0 | −0.25 |
| binomial, $n$ and $p$ by likelihood | +0.0 ± 2.8 | +0.01 |
| binomial by moments | −1.4 ± 2.2 | −0.62 |
| Conway-Maxwell-Poisson | −2.2 ± 3.3 | −0.66 |
| Poisson | −69.6 ± 9.0 | −7.73 |

No form predicts unseen runs better than (21, 0.35) ($z$ above 1.96 would), and the Poisson predicts them worse. The
in-sample preference for another $n$ does not carry over to unseen runs, so there is no reason to change it.

*Do pals change $N$?* No. Fitted per pal count, the means are 7.58 / 7.12 / 7.41 / 8.40 for
1 / 2 / 3 / 4 pals (2,918 runs): more pals do not lower $N$, so pal events do not consume chain events. (Per
scenario, see the table above.)

*Are final links reached as often as predicted?* Yes, except at 2 pals: the binomial lies inside every observed
interval but that one, where it predicts more completions than seen.

| Pals | Chain cards | Runs | Final link reached: observed / Binomial(21, 0.35) / Poisson(7.1) |
|---|---|---|---|
| 5 | 1 | 120 | 0.98 / 0.98 / 0.96 |
| 4 | 2 | 357 | 0.88 / 0.85 / 0.79 |
| 3 | 3 | 872 | 0.60 / 0.59 / 0.54 |
| 2 | 4 | 903 | 0.32 / 0.35 / 0.33 |
| 1 | 5 | 786 | 0.24 / 0.23 / 0.22 |
| 0 | 6 | 386 | 0.15 / 0.16 / 0.16 |

*Spread over slots, or over cards?* Over slots. The alternative gives each running card's next event the same chance
(uniform over cards). On 3,424 runs:

| | Uniform over remaining slots (adopted) | Uniform over cards |
|---|---|---|
| Total LL | −7,318.4 | −7,384.9 |
| Final links, 2 pals (observed 0.32) | 0.35 | 0.41 |
| Final links, 0 pals (observed 0.15) | 0.16 | 0.23 |

The slot rule fits better by 66.4 nats. With no pals it lies inside the observed interval and the card rule
outside; with two pals both predict more final links than seen, the card rule further. What each rule implies with
Binomial(21, 0.35), as P(reach link 1 / 2 / 3):

| Deck | Slots (adopted) | Cards |
|---|---|---|
| two 3-link SSRs | 1.00 / 0.97 / 0.87 | 0.99 / 0.96 / 0.87 |
| 3-link SSR + 2-link SR | SSR 1.00 / 0.99 / 0.93, SR 0.99 / 0.95 | SSR 1.00 / 0.98 / 0.93, SR 0.99 / 0.96 |
| three 3-link SSRs | 0.98 / 0.84 / 0.54 | 0.96 / 0.81 / 0.58 |
| four 3-link SSRs | 0.93 / 0.65 / 0.25 | 0.88 / 0.62 / 0.33 |

For example, with two 3-link cards and exactly 3 events, the slot rule gives one card all three with chance
$\tfrac36 \cdot \tfrac25 \cdot \tfrac14 = 5\%$, the card rule $(\tfrac12)^3 = 12.5\%$.

*Do two-chain decks finish as predicted?* Yes. Decks with two 3-link SSRs and four pals finish both chains in
163 of 196 runs, 0.83 [0.77, 0.88], against 0.80 predicted.

*Known miss: early links in crowded decks run short.* The shortfall is the share of early links the model expects to
fire minus the share seen, ± its run-clustered standard error; $z = \sum(\text{pred} - \text{obs}) /
\sqrt{\sum(\text{pred} - \text{obs})^2}$, with each run's links summed as one cluster:

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Early-link shortfall | 0.01 ± 0.02 | 0.04 ± 0.01 | 0.07 ± 0.02 | 0.05 ± 0.02 | 0.14 ± 0.02 |
| $z$ | 0.8 | 3.4 | 4.7 | 2.9 | 5.6 |

*Known miss: individual cards' final links.* Pooled, final links fit (0.352 observed against 0.357). Card by
card they do not: over the 42 cards with 60 or more final-link observations, a joint test that every card
matches the model (clustered by run, since one run's chain cards share its $N$) gives $\chi^2$ = 229 on
42 degrees of freedom, where chance alone gives about 42. The cause is not known.

*Known miss: chains that never start.* More chains never start in crowded decks than predicted:
0.20 against 0.13 with five chain cards, 0.34 against 0.20 with six. A per-card activation parameter (a chain
card active with probability $a$), fitted jointly with $N$, gains 1.1 nats ($a = 0.97$): rejected.

## 9. Event choices and outcomes

Computed by `research/s09_outcomes.py` (the certainty checks: `research/s13_calibration.py`).

An event can put different skills on different choices, and an outcome can be random. This section states which
choice IT takes and measures the outcome shares that are not equal.

### Findings

- **The model assumes IT takes an event's top choice unless a prioritised skill sits on another**, and that the
  player prioritises every target. In the two events where priority lists show the choice taken, IT follows the rule, and its consequence
  shows on real careers: event-only hints of prioritised targets land at 1.01 of the model, of non-prioritised ones at 0.71.
- **Outcomes are equally likely unless measured.** Five measured outcomes depart from equal shares, and the model
  uses measured values for them, for example Nakayama Festa's Nothing Ventured at 0.61.
- **Some other outcomes are off the equal rule but left at it**: known misses.
- **Pal and group finales are modelled as certain**, and both checked hold: Light Hello and Team Sirius credit their finale
  skill in every run carrying them.

### Terms

- *Choice*: an option of an event. *Top* and *bottom choice*: the first and the last.
- *Outcome*: what a choice gives. *Firing*: one occurrence of an event in a run.
- *Prioritised*: on the run's priority list.
- *Credited*: the run record names an event as the source of a hint.
- *Share*: the fraction of firings with a given outcome. *Equal rule*: each of a choice's outcomes equally likely.
- *Reading*: a firing whose outcome could show.

### Model

Both rules are **assumed**.

*Choice rule.* IT takes an event's top choice unless a prioritised skill (up to 10, set at career start) sits on
another choice, and the player is assumed to prioritise every target hint the model counts. Where targets sit on
different choices, a deck entry takes one choice: the model treats each useful choice as a separate deck option, and an entry that ends
a chain early.

*Outcomes* (**assumed** unless measured). One choice's outcomes exclude each other, one per firing, and are equally
likely unless measured (outcome odds are server-side: master.mdb holds none). A card's gold and white of one skill are
combined exactly.

### Measured shares

These shares are **fitted**, with 90% intervals, counted only where every outcome could show: her hint credited, and
the skill's level from other sources below 5.

| Event, outcome | Observed | Adopted |
|---|---|---|
| Sasami Anshinzawa [This Might Sting!] finale, Nothing Ventured (Risky Business prioritised; 254 runs with the card) | 58/77 = 0.75 [0.66, 0.82] | 0.75 |
| same, choice 2 alone, no inherited Risky Business | 37/45 = 0.82 [0.71, 0.90] | |
| Riko Kashimoto [Planned Perfection] "Unexpected Side", Ramp Up (top choice taken) | 527/585 = 0.90 [0.88, 0.92] | 0.75 |
| Seiun Sky [Foolproof Plan] link 3, Fast-Paced | 33/40 = 0.82 [0.71, 0.90] | 0.75 |
| Smart Falcon [My Umadol Way! ☆] link 3, Center Stage | 37/51 = 0.73 [0.61, 0.81] | 0.75 |
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante", Nothing Ventured (603 runs with the card, below) | 138/228 = 0.61 [0.55, 0.66] | 0.60 |

The other outcomes: Riko Kashimoto's alternative is Maverick ○, Seiun Sky's Escape Artist, Smart Falcon's Prudent
Positioning.

### Evidence

*Does IT follow the choice rule?* Where it can be seen, yes. At Sasami Anshinzawa [This Might Sting!]'s finale, with
Risky Business prioritised IT took choice 2 in about 95% of runs (3 of the 4 exceptions had Risky
Business already at hint Lv 5); with neither skill prioritised, 0 of 35 runs took choice 2. At Riko Kashimoto
[Planned Perfection]'s event, with Lone Wolf prioritised IT took the bottom choice in 55 of 62 runs. Every run
has a priority list, but the run records show it for only 532 of 3,610 (the rest count as unknown, never as an
empty list). So the rule is tested mainly through its consequences (section 13): event-only hints read
1.01 of the model for prioritised targets and 0.71 for non-prioritised ones.

*How is Sasami Anshinzawa's finale read?* On runs without Nakayama Festa [Trusting the Dice], whose "Upping the Ante"
also gives Nothing Ventured +1/+3 and is credited jointly with it. Hint levels identify the choice: her Nothing
Ventured +3 is choice 1, +1 is choice 2, and a loss on choice 2 shows as her Risky Business at level 2 (storyline
choice 3 +1, finale +1). With inherited Risky Business levels mixed in, choice 2 reads 0.77-0.87. Choice 1
(Nothing Ventured +3 or nothing) reads 69/127 = 0.54 if the finale always fires; Grand Concert reads lower
(unexplained).

With Risky Business prioritised, IT also takes the bottom choice of her date-start event "A Pleasant Practice
Partner?" (Risky Business +1). GameTora marks that choice as ending her event chain, which for a pal card means her dates. Yet
the finale still fires: at least 75 of 77 such runs show its outcome, and her extra Risky Business +1 appears in
53 of 54 choice-2 wins with the priority against 1 of 20 without. IT apparently keeps her dates either
way.

*Why is Riko Kashimoto's adopted share lower than measured?* Because it is a per-run chance: a run gets her
Ramp Up hint in 521 of 689 runs, 0.756 [0.728, 0.782], whichever choice IT took. IT took the top choice in
579 of 725 firings (0.80) and the bottom choice, Lone Wolf, in 146.

*How is Nakayama Festa's share read?* On every run with her card (603). Her event is credited to her alone with
Nothing Ventured in 138 runs and with Risky Business in 68. In 40 more, Risky Business is credited to "her or
another card's event". The other card is the only one credited with it in 0.895 of runs without her (Team Sirius, 30 of
the 40) or 0.390 (Sasami Anshinzawa), and she gives it in 0.119 of hers. So a shared credit is hers with
chance $r_F / (1 - (1 - r_F)(1 - r_o))$, where $r_F$ is her rate and $r_o$ the other card's: 6.5 of the 40.
In 39 runs neither outcome shows while Risky Business is already at hint Lv 5 (a Risky Business win cannot
show, or the event never fired); they are split by the rate at which the event shows where both outcomes can,
0.39. The share is then 138/228 = 0.61 [0.55, 0.66]. It stays within 0.523-0.625 whatever share of the
shared credits is hers, and shows no dependence on the trainee's final Wit.

*Which shares are left at the equal rule?* Off 0.5 but left at the equal rule (known misses): Tazuna Hayakawa SSR
finale Concentration 0.70, Agnes Tachyon [Q≠0] link 1 choice 2 Murmur 0.70, Marvelous Sunday [Dazzling Day in
the Snow] link 3 Position Pilfer 0.58, Light Hello SSR finale +3 See Ya Later! 1,497 of 2,103
(0.71). Consistent with 0.5: Gold Ship [That Time I Became the Strongest] 0.61, King Halo [Tonight, We
Waltz] 0.57, Daitaku Helios [Sunlit Angel ♪] Tail Held High 0.47 (link 2, on runs where link 3 visibly fired).

*Are pal and group events certain?* Their events are entered per card with a probability, finales certain; first-year
date events are left out in IT (**assumed**). The finales check out: Light Hello [From the Ground Up] credits See Ya
Later! to her event in 972 of 972 runs carrying her, and Team Sirius is credited with Risky Business (alone or shared) in 64 of 64.

## 10. Random card events (not modelled)

Computed by `research/s10_random_events.py`.

The model leaves some events out. This section measures how often a deck character's shared random event fires and
what leaving it out costs on real careers.

### Findings

- **A deck character's shared random event fires in about 0.14 of the careers it can fire in**, a lower bound. The
  rate falls a little as more such events are on offer, but a constant rate predicts unseen runs as well.
- **Leaving them out under-predicts the targets they can hint**: acquisition there reads 1.26 [1.11, 1.41] of the
  model. Credited at the fitted rate, it reads 1.00.
- **Crediting them is not shown to predict unseen careers better** ($z$ = 1.72, short of 1.96).
- **Two default targets are affected**: Vodka's Nimble Navigator and Nice Nature's Ramp Up, the only ones that are a
  character's top-choice marker.

### Terms

- *Shared random event*: a character's random event that can fire in another uma's career when a card of hers is in
  the deck (any rarity, R cards included).
- $K$: the number of random events on offer: every shared event of the deck's characters plus the trainee's own.
- *Top-choice marker*: a marker skill on an event's top choice. *Bottom-choice skill*: a skill on its bottom choice.
- *Reading*: one shared event in one run, read through its top-choice marker. A *clean hit*: the marker credited to
  that character's event. An *ambiguous credit*: a credit naming two cards' events.
- *Slot form*: a form in which every event slot competes. $T = K + w \cdot \text{pals}$ counts them, $w$ being what a
  pal weighs in random events.

### Model

The model leaves these events out. The fitted rate (**fitted**) is about 0.14 per event
per career. It falls a little with $K$:

$$P(\text{fires}) = 0.18\,(1 - 0.036)^{K - 4},$$

which gives 0.18 at $K = 4$ and 0.13 at $K = 14$. The slope is detectable (run-clustered Wald test, $|z|$ =
2.90) but does not predict unseen runs better than the constant rate.

### Evidence

*How is the rate read?* From top-choice marker skills credited to the character's event: 5,596 readings from
2,884 runs, 786 clean hits. The 102 ambiguous credits count as misses; as hits, they would raise the rate by about
2 points. A reading also misses a firing when the marker was already at hint Lv 5 from other sources
(106 readings): where the event could show, the rate is 0.143, or 0.162 counting the ambiguous credits as hits.
A firing is missed too when IT takes another choice for a prioritised skill (bottom-choice rates below). The rate is
therefore a lower bound.

*Which form?* None beats the constant rate held out:

| Form | LL (in-sample) | Held out, against the line above it |
|---|---|---|
| constant 0.140 | −2,270.8 | |
| $0.18(1 - 0.036)^{K-4}$ | −2,266.9 | +0.9 ± 2.9 nats ($z$ = +0.31) |
| power law $c(K/8)^{-a}$, $a = 0.34$ (uniform competition would be $a = 1$) | −2,266.8 | +0.1 ± 0.5 against the $K$ form ($z$ = +0.11) |
| strict competition $1 - (1 - 1/K)^S$, $S = 1.49$ | −2,283.4 | −13.3 ± 5.8 against the $K$ form ($z$ = −2.30) |
| slot form, $c / T$ (chain links weigh 0), $c = 1.77$ | −2,266.8 | −0.1 ± 0.2 against the $K$ form ($z$ = −0.57) |

Held out (five groups of runs, each scored by the form fitted on the other four), no form predicts unseen runs better
than the constant rate, and strict competition predicts them worse. Within the slot form, chain links take no weight
(weight 1 is rejected, run-clustered score test) and pals compete: a pal weighs $w$ = 1.09 [0.57, 1.62] random
events (run-clustered interval), with 0 excluded.

*What else moves the rate?* Little. The observed hit rate by $K$ = 4 / 6 / 8 / 10 / 12 / 14 is 0.17 / 0.19 / 0.17 / 0.13 / 0.14 / 0.13, and
within a run events fire independently. A logistic slope in $K$ gives 15.7% at 8 on offer and 12.6% at 14.
Friendship and specialty priority show no effect (run-clustered Wald tests), nor does rarity within a character
(pooled, SSR markers fire at 15.3% against SR 11.2%).
Scenario is borderline detectable in-sample (p-value 0.048), but a rate per scenario does not predict unseen
runs better ($z$ = 0.30).

*Are bottom-choice skills credited?* Rarely. In the 532 runs showing a priority list, 22 of 925
candidates were prioritised. Without a priority they were credited in 0.035 [0.023, 0.048] of candidates, against
0.140 for top-choice markers; with a priority, 3 of 22.

*What does leaving them out cost on real careers?* Section 13's rows, each on the run's own deck. This test scores
whether the target was acquired from any source, so it does not depend on reading the event. Of the default targets
only two are a character's top-choice marker (Vodka's Nimble Navigator, Nice Nature's Ramp Up).

- On the 286 run-target rows where a deck card's random event could hint the target, it was credited to that
  event 32 times, 21 with no other source. The model gives 72 of these rows no chance, and the
  event hinted the target on 6 of them.
- Left out, acquisition there reads 1.26 [1.11, 1.41] of the model, an under-prediction. Credited at the
  fitted rate (about 0.14 there), it reads 1.00 [0.88, 1.11].
- Held out (the rate fitted on the other four groups' readings), crediting gains 4.8 ± 2.8 nats on
  these rows ($z$ = 1.72): positive, but short of the 1.96 that would show it predicts unseen careers better.
- Crediting raises an affected run's odds over the targets it can land by a median 16% (at most
  159%; 214 runs).
- The trainee's own random event can be told apart only for Nice Nature: 0 of 6 rows.

The model leaves these events out; the data say it under-predicts the two affected targets on decks with those
characters.

## 11. Conditional character events (not modelled)

Computed by `research/s11_conditional.py`.

The model also leaves out the trainee's secret and race-result events. This section counts them and measures what
leaving them out costs.

### Findings

- **168 secret and race-result events give hints**, for 81 umas. 11 touch the default targets, none with a gold.
- **Where only such an event can give a target, leaving them out shows no significant cost**: acquisition reads
  1.20 [0.75, 1.64] of the model, and crediting the measured rates gains only 0.2 nats.
- **The exception is Vodka's Nimble Navigator**: her own events give it in 25 of 27 runs, and the model
  under-predicts it.
- **Tokai Teio's Head-On is modelled as always acquired**, like Yamanin Zephyr's Slipstream.

### Terms

- *Conditional character event*: a trainee's secret or race-result event.
- *Conditional-only row*: a run-target row where only such an event can give the target: none of the trainee's story,
  date, outfit or shared random events can, nor the scenario's.

### Evidence

*How many are there?* 168 events with hints for 81 umas (110 for 55 umas in Global); 11 touch the default
targets (Nimble Navigator, Ramp Up, Tail Held High), 0 of them giving a gold.

*What does leaving them out cost?* On the conditional-only rows, her event gave the target on
4 of 56 rows, 0.071 [0.028, 0.170]: Daiwa Scarlet, any outfit, 0 of 52 (Nimble
Navigator); Mejiro Ardan, any outfit, 4 of 4 (Ramp Up). Left out, acquisition there reads
1.20 [0.75, 1.64] of the model, and crediting the measured rates gains only 0.2 nats.

*And Vodka?* Vodka, any outfit, gets Nimble Navigator from her own events in 25 of 27 runs
[0.77, 0.98]; her random and conditional events both give it and cannot be told apart. With her rows, the model
left as it is reads 1.45 [1.20, 1.71], and crediting gains 15.9 nats. Not modelled, so the model
under-predicts Vodka's Nimble Navigator.

*Why is Tokai Teio's Head-On certain?* All three of its races are her career goals. The goals ask only for top 5, top
5 and top 3, but IT won all three in 62 of 62 runs training her, and every run where her event could
show credits Head-On.

## 12. The composed career model

The composition is **derived** from sections 1-11; there is nothing to fit. How a planner searches over loops and decks
with it is outside this document (`docs/SOLVER.md`); an independent implementation (`val/joint_check.py`) reproduces
its exact odds.

### Findings

- **Each target's chance of landing combines its sources**: the deck's gold hint, then the white from the deck, the
  trainee's own events or inheritance, each followed by spark generation.
- **Targets are not independent**: chain cards share one run's $N$, and one event's outcomes exclude each other. The
  model sums over the final chain states exactly instead of multiplying the targets' chances.
- **With real legacies the model is the same**, using each veteran's real stars, G1 wins and parents.

### Terms

- $G_k = 0.4\cdot 1.1^{n_k}$, $W_k = 0.2\cdot 1.1^{n_k}$: the chance target $k$ becomes a spark when the gold, or only
  the white, is learned (section 1).
- $d^g_k$: the chance the deck hints the gold; $d^w_k$: that it hints the white.
- $e_k$: the chance the trainee's own events hint it; $q_k$: the chance of an inspiration of it (section 3).
- $p_k$: the chance target $k$ lands.
- $c$: the final chain state, with the links where chains stopped.

### Model

For a target $k$ not guaranteed:

$$p_k = d^{g}_k\, G_k + (1 - d^{g}_k)\, W_k \left[1 - (1 - d^{w}_k)(1 - e_k)(1 - q_k)\right]$$

A skill on the trainee's outfit replaces the bracket by 1 (white) or gives $p_k = G_k$ (gold); a scenario-guaranteed
gold gives $G_k$. Inheritance only ever supplies the white.

Given $c$, sources are independent except one card's training draws, which are joint over its targets (section 7):

$$P(\text{all}) = \sum_{c} P(c) \prod_{k} p_k(c) \qquad (\text{exact, **derived**}).$$

*Real legacies.* The same model with each veteran's real stars, real G1 wins and recorded parents (an unknown parent
brings no relation points; her sparks count); $n_k$ = the legacies actually carrying $k$. Since $q_k$ and $e_k$ enter
only as $(1 - e_k)(1 - q_k)$, $e_k$ is replaced by $1 - (1 - e_k)(1 - q_k)$, and $1.1^{n_k - 6}$ scales $P(\text{all})$
(**derived**).

## 13. Calibration against real careers

Computed by `research/s13_calibration.py`.

This section rebuilds real careers as the model sees them and compares what the model predicted with what happened.

### Findings

- **The model's acquisition odds are calibrated**: observed / predicted 0.994 [0.98, 1.00].
- **Inheritance is calibrated.** Of the eight targets, only Tail Held High's interval excludes 1 (0.946 [0.90, 0.99]).
- **Deck hints read optimistic** (0.937 of the model), almost all from the priority assumption: event hints land at
  1.01 × predicted for prioritised targets and 0.71 for others. Unmodelled scenario and trainee events supply the
  few points the deck loses.
- **Trackblazer's training-hint base over-predicts** these runs (0.777 of the model); the other scenarios' intervals cover 1.
- **Landing all eight targets reads 0.86 of the model**: consistent with calibrated, though only 132 runs acquired
  every open target and the interval allows up to about 1.4× optimism.
- **Beyond the eight targets**, inheritance holds, and training-pool hints are under-predicted (1.064 of the model).

### Terms

- *Row*: one default target in one career.
- *Certain row*: a target the trainee's outfit gives, or one the scenario guarantees as a gold. *Open target*: one
  that is not certain.
- *Predicted*: the model's expected successes over a set of rows; *observed*: the successes seen. The table below
  shows both per row (as rates), except for landing, which shows sums.
- *Deck hint*, *training pool only*, *events only*, *prioritised*: test 1 below.
- *Bucket*: the rows whose predicted chance falls in one range.
- *Single-source pair*: a (card, target) pair where that card is the deck's only source of the target.

### What is compared

Each of 3,348 real careers (URA Finale 645, Unity Cup 853, Grand Concert 1,768,
Trackblazer 82) is rebuilt as the model sees a career: the trainee in her outfit, the scenario, the six cards at
their own limit breaks, both parents with their recorded parents and every ancestor's white sparks, and race wins
giving the recorded shared-G1 counts. The model then gives, for each default target of that career, the chance of
each step (sections 3-12), and the run record says what actually happened. There are 26,784 rows. Certain rows
(1,159 on her outfit, 1,706 scenario golds) could only count as successes, so they are left out of tests 1 and 3.

Each test reports observed / predicted over a set of rows: 1 means calibrated, above 1 the model is pessimistic, below
1 optimistic. Intervals are 95%, clustered by run, since a run's eight targets share its deck and lineage (*How to
read*). The runs end before skills are bought, so "acquired" means hinted and buyable, the model's own meaning.

The four tests, each isolating one part of the model:

1. **Deck hint**: does a deck card hint the skill (by training or by its events) as often as sections 7-9 predict?
   The prediction is $d^g + (1 - d^g)d^w$, the chance the deck hints the gold or else the white; observed is a deck
   card credited with a hint of the skill. Only rows where no inspiration hinted the skill are used: the hint rates are
   fitted on such rows, and with an inspiration a card's hint is seen less often (section 7). The rows are split by
   where the deck's chance comes from: **training pool only** (only cards' training can hint the skill) and **events
   only** (only cards' events can); events only is split again by whether the target was on the run's priority list
   (**prioritised**, for runs that show one).
2. **Inheritance**: does an inspiration of the skill fire as often as section 3 predicts ($q$)? Observed: a spark of
   the skill fired at the Classic or Senior inspiration.
3. **Acquired**: does the skill end up hinted, from any source, as often as all modelled sources together predict?
   This is what the model's odds rest on.
4. **Landing**: the model's chance that the target lands as a spark ($p$), against the same chance computed from
   what the run actually acquired ($0.4 \cdot 1.1^n$ if the gold was hinted, $0.2 \cdot 1.1^n$ if only the white). It
   checks the acquired step through the spark step; it does not test spark generation, which the runs do not show
   (section 1).

| Test | Rows | Predicted | Observed [95%] | Obs / pred [95%] |
|---|---|---|---|---|
| 1. Deck hint, no inspiration | 15,774 | 0.260 | 0.244 [0.240, 0.249] | 0.937 [0.92, 0.95] |
| 1. Training pool only | 3,601 | 0.355 | 0.351 | 0.988 [0.95, 1.03] |
| 1. Events only | 2,082 | 0.884 | 0.804 | 0.909 [0.89, 0.93] |
| 1. Events only, target prioritised | 186 | 0.864 | 0.871 | 1.007 [0.96, 1.05] |
| 1. Events only, not prioritised | 234 | 0.860 | 0.611 | 0.711 [0.65, 0.78] |
| 2. Inheritance | 26,784 | 0.358 | 0.354 [0.350, 0.359] | 0.989 [0.98, 1.00] |
| 3. Acquired, any source | 23,919 | 0.514 | 0.511 [0.506, 0.515] | 0.994 [0.98, 1.00] |
| 3. Acquired, deck or inspiration only | 23,919 | 0.514 | 0.502 | 0.976 [0.97, 0.98] |
| 3. Acquired, prioritised | 1,587 | 0.727 | 0.747 | 1.028 [1.005, 1.051] |
| 3. Acquired, not prioritised | 1,888 | 0.556 | 0.519 | 0.932 [0.903, 0.960] |
| 4. Landing (sums of $p$) | 23,919 | 4,358.0 | 4,313.2 | 0.990 |

### Evidence

*Does the priority list matter?* Yes. On the runs that show a priority list, prioritised targets are acquired slightly
more often than predicted and non-prioritised ones less; the pooled test averages the two.

*Where does the deck lose its points?* Almost all in event hints of targets the player did not prioritise (test 1's
split above). Unmodelled scenario and trainee events supply the few points the deck loses: 220 target acquisitions
in 23,919 rows, 0.9%, and the composed acquisition odds are calibrated (0.994).

*Per target* (observed / predicted):

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

*Training-pool hints by scenario* (a test of $\beta$ and $d$, section 7): Grand Concert 1.009 [0.96, 1.06], Unity Cup
0.967 [0.86, 1.08], URA Finale 0.980 [0.86, 1.10], Trackblazer 0.777 [0.62, 0.93]. Only
Trackblazer's interval excludes 1, its $\beta$ over-predicting these runs. By the lowest limit break of the sourcing
cards: LB4 0.997, LB3 0.963, LB0-2 0.83-1.03 (72-118 rows each, every interval covering 1).

*Unity Cup's guaranteed golds* show in 853 of 853 Unity Cup runs each.

*Gold acquisition* (observed / predicted): Nothing Ventured 0.847 [0.77, 0.92] (833 rows), Perfect Spot!
0.838 [0.76, 0.92] (439), It's On! outside Unity Cup 0.456 [0.31, 0.66] (119), No Stopping
Me! outside Unity Cup 0.885 (63), See Ya Later! 1.000 (2,723).

*Reliability.* Rows grouped by the model's predicted chance; if the model is calibrated, each bucket's observed rate
matches its mean prediction. Deck hint and acquired, observed (mean prediction in parentheses):

| Predicted | Deck hint: rows, observed | Acquired: rows, observed |
|---|---|---|
| 0 | 8,743, 0.001 | 5,246, 0.015 |
| above 0 to 0.20 | 102, 0.235 (0.17) | 1,286, 0.166 (0.16) |
| 0.20-0.40 | 2,620, 0.297 (0.31) | 2,715, 0.309 (0.30) |
| 0.40-0.60 | 1,328, 0.447 (0.47) | 3,147, 0.498 (0.50) |
| 0.60-0.80 | 910, 0.623 (0.72) | 4,609, 0.706 (0.71) |
| 0.80-0.95 | 515, 0.740 (0.87) | 3,310, 0.828 (0.86) |
| 0.95-1 | 1,556, 0.962 (1.00) | 3,606, 0.978 (1.00) |

The deck's over-prediction sits in its top buckets, the event rewards: the buckets predicted 0.6 and above hold
82% of the shortfall, and 97% of their rows have an event source. The 0.20-0.40 bucket, almost all training-pool
rows, is within its interval (0.297 [0.280, 0.314] against 0.308).

*Per run.* A run has 7.14 open targets on average.

- Targets acquired per run: expected 3.673, observed 3.650.
- All open targets acquired: product of the model's odds 0.041, observed 0.039 [0.033, 0.047].
- Landing $P(\text{all eight})$, recomputed from what each run acquired, against the model's exact $P(\text{all})$:
  0.863 [0.69, 1.05] (bootstrap over runs, resampling whole runs); Unity Cup 0.851 [0.67, 1.03],
  Grand Concert 1.27 [0.84, 1.71], URA Finale 0.88 [0.36, 1.60]; Trackblazer's model value is 0.

Consistent with calibrated, though the interval allows up to about 1.4× optimism (1/0.69). Only
132 runs acquired every open target, too few to separate the exact joint from the product.

*Single-source cards.* For a single-source pair the deck test scores that card alone. The pairs furthest from the model
(every pair of 40 or more rows whose interval excludes 1, and Gold Ship's):

| Card > target | Pred | Obs | Ratio | Rows | Likely reason |
|---|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | 1.000 | 0.731 | 0.73 [0.66, 0.79] | 186 | the model assumes Risky Business prioritised (finale choice 2) |
| Riko Kashimoto [Planned Perfection] > Ramp Up | 0.750 | 0.579 | 0.77 [0.71, 0.83] | 420 | open (her Lone Wolf choice, when prioritised, accounts for part) |
| King Halo [Tonight, We Waltz] > Uma Stan | 0.937 | 0.726 | 0.77 [0.66, 0.87] | 73 | Uma Stan sits on a choice that needs a priority |
| Gold Ship [That Time I Became the Strongest] > Uma Stan | 0.196 | 0.133 | 0.68 [0.38, 1.17] | 75 | open |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | 0.301 | 0.068 | 0.23 [0.08, 0.60] | 44 | open |
| Maruzensky [Sentimental Flare ♪] > Slipstream | 0.237 | 0.301 | 1.27 [1.04, 1.53] | 236 | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | 0.346 | 0.493 | 1.43 [1.11, 1.75] | 75 | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | 0.710 | 0.638 | 0.90 [0.81, 0.98] | 235 | open |
| Sweep Tosho [Lamplit Training of a Witch-to-Be] > Slipstream | 0.337 | 0.282 | 0.84 [0.70, 0.994] | 305 | open |
| Nakayama Festa [Trusting the Dice] > Risky Business | 0.614 | 0.508 | 0.83 [0.75, 0.90] | 423 | open |

Below 40 rows, 5 more pairs exclude 1. 3 of them are event sources of Playtime's Over! (Yukino Bijin [Dancing Light
into the Night], Yaeno Muteki [Fiery Discipline], Sirius Symboli [Escorte Étoile]) that hinted it in 1 of 46 runs
against 0.50-0.84 predicted. Their event puts the skill on choice 2, which ends the chain before its last link, so
IT takes it only when the skill is prioritised. Only 3 of those runs show a priority list (the skill prioritised in
1, hinted in 0 of the 3), and the other 43 hinted it 1 time, so the priority cause is
untested here.

*Other skills.* Every non-target skill a deck card or spark could give, on a 30% sample of the runs
(1,004): inheritance reads 0.993 [0.98, 1.01] over 43,244 rows, so it holds beyond the eight
targets; training-pool hints read 1.064 [1.05, 1.08] over 16,978, under-predicted.

### Untested here

the step from acquired to landed (spark generation, and that everything hinted is bought); the
priority policy run by run; correlations inside a run beyond $P(\text{all})$; Grand Masters; hint levels. The runs are
completed careers; every comparison is per run on that run's own deck and lineage. Rows with an inspiration are left out
of the deck test, which is then conditional on no spark, treated as independent of the deck.
