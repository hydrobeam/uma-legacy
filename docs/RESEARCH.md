# The model's mathematics

A model of how a career in Uma Musume's Independent Training acquires and passes on skills: what each part is, how it
was set from data, and how well it holds against real careers.

## Results

The model's odds rest on one prediction: the chance that a career acquires each target. Against
5,198 real Independent Training careers, each rebuilt from its own deck and lineage (section 13), that chance is
calibrated where the model's priority assumption holds: observed / predicted 0.999 [0.991, 1.006] over the
32,848 run-target rows whose target was prioritised or whose run shows no priority list. Over all 37,287 rows it reads
0.988 [0.981, 0.994], slightly optimistic: the model assumes every target prioritised, and the rows whose run shows it was not
read 0.912 and carry 92% of the shortfall. The chance of landing all eight default targets reads
0.93 [0.80, 1.05] of the model: consistent with calibrated, but only 220 runs acquired every target.

The terms used below (observed / predicted, rows, nats, held out, the bracketed intervals) and the game's terms are
defined in *How to read*, after this summary.

| Part | How it is set | Main evidence | Section |
|---|---|---|---|
| Spark generation, $0.2$ / $0.4 \times 1.1^n$ | assumed | none: the runs show the sparks a career inherited, not the ones it produced | 1 |
| Star mix of sparks | mixes assumed; band per scenario derived | each scenario uses the band most of its careers reach (14,742 Grand Concert runs, 79% at 17,500+) | 2 |
| Inheritance rolls | skill-white bases tested, grandparents fitted | grandparent weight 1.00 [0.98, 1.02], affinity slope 0.97 [0.95, 0.99] on 106,738 + 44,010 trials, which predicts unseen runs no better than slope 1 ($z$ = 0.86); whole careers 0.992 [0.982, 1.003] apart from Tail Held High | 3 |
| Affinity: +3 per shared G1 win; own grandparent adds no trio points | +3 assumed and tested; trio rule fitted | shared-win slope 0.96 [0.92, 0.99], no better than 1 held out ($z$ = 0.81); trio rule 29 nats better than the alternative | 3, 5 |
| Race win table | community table, tested | calibrated once grades include inherited aptitude sparks: 0.998 [0.996, 0.999] over 119,188 G1 attempts | 4 |
| Loop schedules for shared wins | derived | on the reference loops, no other schedule could raise the best loop's odds by more than 2.02% | 6 |
| Training hints, $\beta(1 + hf/100)(1 + d\,\mathit{sp}/100)$ | form fixed on the runs, slope $1/100$ assumed and tested, $\beta$ and $d$ fitted | 87,801 rows; specialty term fitted on earlier runs gains +10.1 nats on later ones; real careers 1.004 [0.97, 1.04] of the model | 7 |
| Chain events, Binomial(21, 0.35) per career | fitted, kept | 5,316 runs; rejected against a free distribution in-sample (p-value 6.5e-05), a known miss; no alternative predicts unseen runs better | 8 |
| Chain events spread over remaining links | fitted | 102 nats better than equal shares per card | 8 |
| Event choices: IT takes a prioritised skill's choice | assumed | event hints of prioritised targets 0.970 [0.940, 0.9997] of the model, of the others 0.65 | 9, 13 |
| Measured outcome shares | fitted | five events, e.g. Nakayama Festa's Nothing Ventured 0.61 [0.56, 0.66] (90%) | 9 |
| Random card events | not modelled | where they could hint a target, acquisition reads 1.25 [1.12, 1.38] of the model left out, 1.01 credited | 10 |
| Conditional character events | not modelled | where only they can hint a target, left out reads 1.10 [0.81, 1.40]; Vodka's own Nimble Navigator 25 of 27 runs | 11 |

### Known misses

Each measured and tested in its section:

- Inheritance (3, 13): Tail Held High is inherited less often than its odds say (0.953 [0.918, 0.988]
  of the model on whole careers; cause unknown); parents at affinity 200 or more pass sparks less often than predicted
  (0.966 [0.945, 0.987]); non-target skills are inherited slightly less often (0.986 [0.973, 0.999]).
- Races (4): chances of 70 or less win slightly less (0.95); chosen races at the 1st and 2nd race in a row
  slightly less (0.996, 0.997); goal races 7th or later in a row, which the
  planner scores as the 6th, win less than it predicts (0.68, 47 attempts).
- Training hints (7): cards differ in hint rate beyond the adopted rate (observed / predicted 0.80-1.17 over
  58 cards, cause open); decks with more pals give slightly more hints (+12.5 nats); a skill also hinted by
  inspiration is hinted by training less than predicted (0.74 of the model).
- Chain events (8): Binomial(21, 0.35) is rejected against a free distribution of $N$ in-sample ($p$ = 6.5e-05) and kept
  by choice: no alternative predicts unseen runs better, and the candidate Binomial(18, 0.405) fits 8.8 nats better
  in-sample. Decks with Sasami Anshinzawa [This Might Sting!]'s pal card reach more chain events (7.74 against
  7.17) and Inari One's careers fewer (6.83 against 7.26); in crowded decks early links run short, and more chains never start than predicted
  (0.34 against 0.20 with six chain cards). Final links fit pooled (0.348 against 0.353), but
  individual cards differ far beyond chance ($\chi^2$ = 386 on 50 cards).
- Deck hints (13) read 0.929 [0.915, 0.942] of the model, almost all from event hints of targets the
  player did not prioritise (0.65 of the model; the model assumes every target is prioritised). Event hints of
  prioritised targets read just below the model (0.970 [0.940, 0.9997]; section 9).
- Trackblazer's hint base (13) over-predicts real careers: its training-pool hints read 0.779 [0.63, 0.93] of the model.
- Left out of the model: deck cards' random events, so the targets they can hint are under-predicted
  (1.25 [1.12, 1.38] of the model on those rows; their rate rises with the card's initial friendship gauge; section
  10); and Vodka's own Nimble Navigator (25 of 27 runs; section 11).
- Non-target skills (13): training-pool hints read 1.065 [1.053, 1.076] of the model.

### Not settled

Specialty priority predicts runs it was not fitted to (fitted on earlier runs, +10.1 nats on later
ones), but it is not shown to cause more hints: on the cards whose limit breaks change only their specialty priority,
the estimate ($d$ = −0.06 [−0.44, 0.56]) is consistent with no effect and with every per-scenario weight.
The hint-frequency slope $1/100$ is assumed: on the adopted model a free divisor does not predict unseen runs better,
but in the form study, without the specialty term, one does (section 7).

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
the six real affinities of each of the 5,527 lineages in the runs, for a target carried by all six, it overstates $q$ by
at most 0.57% below 17,500 (mean 0.51%) and 0.33% at 17,500+.

## 3. Inheritance rolls

Computed by `research/s03_inheritance.py` (the roll's parts) and `research/s13_calibration.py` (the test on whole
careers).

Inheritance is one way a target gets hinted: at each inspiration, a legacy's spark of the skill may fire. This section
gives that chance, tests its parts on single-ancestor trials, and tests the whole on real careers.

### Findings

- **Each legacy carrying the target gets two rolls**, at the Classic and Senior inspirations. A roll's chance is a
  base set by the spark's stars, raised in proportion to the legacy's affinity.
- **Grandparents take the full base, as parents do**: a free weight on their roll fits 1.00 [0.98, 1.02].
- **Affinity raises the roll as the formula says, not beaten held out**: a free slope fits 0.97 [0.95, 0.99],
  but it predicts unseen runs no better than slope 1 ($z$ = 0.86), and neither does a free weight on either of its two
  parts, relation points and shared G1 wins.
- **On whole careers the chance is calibrated apart from one target**: without Tail Held High,
  0.992 [0.982, 1.003] of the model over the other default targets.
- **Known misses**: Tail Held High is inherited less often than predicted (0.953 [0.918, 0.988],
  cause unknown); parents at affinity 200 or more pass less often than predicted (0.966 [0.945, 0.987]),
  though a refitted slope does not predict unseen runs better; non-target skills
  read 0.986 [0.973, 0.999], the shortfall spread over many skills; and the band of predicted chance
  0.40-0.60 lies outside its own interval, though the bands hold jointly.

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

The roll's parts are **fitted** on 44,010 parent and 106,738 grandparent trials of ordinary ancestors (the
self-grandparent is section 5's).

*Do grandparents take the full base?* Yes. A free weight on the grandparents' roll fits 1.00 [0.98, 1.02],
and the full base beats a halved one by 1,975 nats: 9,607 passes observed, 9,630 predicted with the full
base, 4,815 with half. Parents read 0.975 [0.956, 0.995] of the model, the shortfall at high affinity (below).

*Does affinity scale the roll as $(1 + A/100)$?* The term beats no affinity by 5,427 nats. A free slope on $A$ fits
0.97 [0.95, 0.99], 2.9 nats above slope 1 in-sample; held out it predicts unseen runs no better
(2.1 ± 2.4 nats, $z$ = 0.86), so the slope stays 1. Its two parts, each fitted with the other fixed: relation
points 0.92 [0.84, 0.99], held out 1.8 ± 2.2 nats ($z$ = 0.85); $3 \times$ shared G1 wins 0.96
[0.92, 0.99], held out 2.0 ± 2.5 nats ($z$ = 0.81) (150,032 trials with shared wins). Neither
weight predicts unseen runs better than 1.

*Do the bases by stars hold?* Observed / predicted is 0.985 [0.95, 1.02] at 1★ (41,970 trials),
0.987 [0.97, 1.00] at 2★ (96,080) and 0.995 [0.96, 1.03] at 3★ (12,698).

*Does it hold at every affinity?* Below 200, yes: the four bands are jointly consistent with the model ($\chi^2$ =
2.5, $p$ = 0.65). The bands outside their own 95% interval are 200+, and with them the five
bands are not consistent ($\chi^2$ = 12.6, $p$ = 0.027). This is a known miss: parents at affinity 200 or more read
0.966 [0.945, 0.987] of the model (36,680 trials), parents below it 1.040 [0.985, 1.094]
(7,330). A refitted slope does not predict unseen runs better (above), so the model keeps slope 1 and
this miss:

| Affinity | Trials | Observed / predicted |
|---|---|---|
| 0-49 | 12,628 | 0.980 [0.919, 1.042] |
| 50-99 | 94,696 | 0.999 [0.980, 1.019] |
| 100-149 | 1,428 | 1.025 [0.899, 1.152] |
| 150-199 | 5,316 | 1.045 [0.982, 1.107] |
| 200+ | 36,680 | 0.966 [0.945, 0.987] |

*Does the whole hold on real careers?* Observed / predicted inspiration on 5,198 careers (section 13):

- all 41,584 run-target rows: 0.987 [0.978, 0.997], short of the model because of one target;
- Tail Held High, a known miss: 0.953 [0.918, 0.988], the only default target whose interval excludes 1;
  the cause is not known;
- the other 36,386 rows: 0.992 [0.982, 1.003]; on them, by the number of carriers, 1 to 6:
  0.965-1.000, every interval covering 1; per roll, years in which an inspiration of the skill fired,
  0.989 of the model's expectation (1.84 standard deviations below it, run-clustered; inside its 95% band), and
  legacy firings, against their expectation $2\sum_l r_l$, 0.990;
- 66,598 rows of non-target skills, a known miss: 0.986 [0.973, 0.999], the shortfall spread over
  many skills (no one skill carries more than 9% of it).

*Does it hold at every predicted chance?* Grouped by predicted $q$, the groups are calibrated jointly ($\chi^2$ =
9.2 on 5 groups, $p$ = 0.10). One lies outside its own 95% interval, 0.40-0.60: 0.494
[0.481, 0.506] observed against 0.506 predicted.

| Predicted $q$ | Rows | Observed (mean predicted) |
|---|---|---|
| 0 | 13,734 | 0.001 |
| above 0 to 0.20 | 3,884 | 0.154 (0.16) |
| 0.20-0.40 | 4,688 | 0.294 (0.30) |
| 0.40-0.60 | 6,232 | 0.494 (0.51) |
| 0.60-0.80 | 9,066 | 0.697 (0.70) |
| 0.80-0.95 | 3,980 | 0.838 (0.84) |

## 4. Race wins

Computed by `research/s04_races.py`.

Race wins matter through affinity: two umas that won the same G1s inherit more from each other (section 5). This
section measures how a trainee's chance of winning a race depends on her aptitudes, on how many races she has run in a
row and on whether her career requires the race, and tests a widely used community table of that chance.

### Findings

A race's win chance depends on the trainee's grades for its distance and surface (written as a pair, better grade
first) and on how many races she has run in a row. Of 119,188 attempts at the 34 single-race G1s, the 85,099 that IT
chose to run won at these rates (the community table's chance in parentheses; grades include those raised by
inherited aptitude sparks; S is shown as A, since both have the same chance; a dot where a cell has under 30 attempts;
grade pairs with 200 or more attempts):

| Races in a row | AA | AB | AC | BB | AD | BC | AE | BD | CC | BE | CD | AF | AG |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 race | 100% (100%) | 100% (100%) | 90% (90%) | 91% (90%) | 80% (80%) | 76% (80%) | 68% (70%) | 70% (70%) | 63% (70%) | 50% (60%) | 59% (60%) | 46% (50%) | 19% (20%) |
| 2 races | 100% (100%) | 100% (100%) | 91% (90%) | 90% (90%) | 79% (80%) | 80% (80%) | 61% (70%) | 68% (70%) | 78% (70%) | 50% (60%) | 65% (60%) | 52% (50%) | 19% (20%) |
| 3 races | 100% (100%) | 95% (95%) | 85% (85%) | · (85%) | 75% (75%) | · (75%) | 59% (65%) | 60% (65%) | · (65%) | 43% (55%) | · (55%) | 56% (45%) | 11% (15%) |
| 4 races | 90% (90%) | 81% (80%) | 74% (70%) | · (70%) | 49% (60%) | 62% (60%) | 38% (50%) | 61% (50%) | · (50%) | · (40%) | 30% (40%) | · (30%) | · (0%) |
| 5 races | 81% (80%) | 66% (70%) | 54% (60%) | · (60%) | · (50%) | · (50%) | 35% (40%) | · (40%) | · (40%) | · (30%) | · (30%) | · (20%) | · (0%) |
| 6 or more | 56% (60%) | 58% (50%) | · (40%) | · (40%) | · (30%) | · (30%) | · (20%) | · (20%) | · (20%) | · (10%) | · (10%) | · (0%) | · (0%) |

- **A goal race takes the penalty of the race before it.** Goal races (the races a career requires) win as if they
  stood one place earlier in the streak: no penalty up to the 3rd race in a row, then 5, 20 and 30. Scored that way
  they are calibrated at every position, and the table predicts unseen runs 158 ± 34 nats better than the
  community table, which charges a goal race its own position ($z$ = 4.72). A goal race still counts toward the
  position of the races after it: not counting goal races at all predicts 1,151 nats worse.
- **With that shift the table predicts these rates** for goal races at every position and for chosen races from the
  3rd race in a row on. Outside chosen races at the 1st and 2nd it is calibrated in every scenario.
- **Grades must include inherited aptitude sparks.** With the trainee's base grades the table predicts too few wins
  (observed / predicted 1.063 [1.060, 1.066], 1.78 where base grades give a chance of 70 or less). Pink sparks
  raise grades at the start of a career and at each inspiration, from that April on.
- **Races in a row lower the chance; wins do not.** The penalty counts races entered. At the 2nd, 3rd and 4th or later
  race in a row the previous race's result does not change the next chance: after a loss the table's fit is
  0.993, 0.990 and 1.021 of its fit after a win, each interval covering 1.
- **Known misses**: chances of 70 or less win slightly less than predicted (0.95
  [0.93, 0.97]); the planner counts a streak only to its 6th race, so the few goal races 7th or later
  in a row (47) win less than it predicts (0.68 [0.51, 0.84]); chosen races
  at the 1st and 2nd race in a row win slightly less than predicted (0.996 [0.994, 0.998] and 0.997 [0.994, 0.999]), which
  puts the table's overall observed / predicted at 0.998 [0.996, 0.999].

### Terms

- *Grade*: the trainee's aptitude grade, S to G, for the race's distance and surface; $c_{\text{dist}}$ and
  $c_{\text{surf}}$ are what each grade costs.
- *Streak*: consecutive turns with a race; $j$ is a race's position in it, $s$ the position whose penalty it takes
  and $\mathrm{PEN}_s$ that penalty.
- *Table's chance*: $110 - c_{\text{dist}} - c_{\text{surf}} - \mathrm{PEN}_s$, in percent, before it is clamped to
  0-100.
- $p$: the chance of winning one race she enters. $P_a(r)$: uma $a$'s chance of winning race $r$ at least once over her
  schedule.
- *Attempt*: one race entered. *Goal race*: one her career requires; *chosen race*: one IT chose to enter.
- *Base aptitudes*: her grades before any spark. *Raised grades*: her grades after her legacies' pink sparks (the rule
  in the evidence below).
- *Community table*: the same formula with $s = j$ for every race.

### Model

A trainee's chance of winning a race she enters is a community table (**assumed**, tested below) with one
adjustment for goal races (**fitted**: chosen among three forms, below):

$$p = \mathrm{clamp}\big(110 - c_{\text{dist}} - c_{\text{surf}} - \mathrm{PEN}_s,\ 0,\ 100\big) / 100,$$

$c$ = 0 for S/A, 10 B, 20 C, 30 D, 40 E, 60 F, 90 G. $\mathrm{PEN}_s$ = 0, 0, 5, 20, 30, 50 for $s$ = 1 to 6 or more.
$j$ counts any race, goals included, and a turn without a race resets the streak. A chosen race takes its own
position, $s = j$; a goal race the one before it, $s = \max(j - 1, 1)$, so goal races pay 0, 0, 0, 5, 20, 30. The
planner counts $j$ up to 6 (6th or later), so a goal race 7th or later in a row pays 30. Stats and training are not
modelled. Over a planned schedule,

$$P_a(r) = 1 - \prod_{\text{attempts}} (1 - p);$$

section 5 turns these into race affinity.

### Evidence

The runs record every race a career ran: 119,188 attempts at the 34 single-race G1s (34,089 of
them goals), of which 93.7% were won.

*Base aptitudes or raised grades?* Raised grades.

- On base aptitudes (as for a planned loop), the table predicts 88.2%: observed / predicted
  1.063 [1.060, 1.066], and 1.78 where the table gives 70 or less (19,012 attempts).
- With her grades raised by the aptitude-spark rule (`docs/MODEL.md`: the legacies' pink stars at the
  start, at most to A; then each pink the run records firing at a Classic or Senior inspiration, from that April on),
  it predicts 94.0%: observed / predicted 0.998 [0.996, 0.999], 2,921 nats better than
  base aptitudes. Chosen races at the 1st and 2nd race in a row fall slightly short (0.996 [0.994, 0.998] and
  0.997 [0.994, 0.999]); outside them every scenario is calibrated, and over all attempts Grand Concert and Unity Cup
  fall outside their intervals.

*Is the streak penalty real?* Yes: without it the fit is 2,753 nats worse.

Both comparisons need one adjustment. The table calls some races certain (0% or 100%), and a handful went the other
way, which gives the table as stated a log-likelihood of $-\infty$. So every model compared has its 0% and 100% pulled
in to a fitted floor and ceiling, the same two parameters in each; the comparisons are then between models with equal
numbers of fitted parameters (rule 1 of *How to read*).

*Do goal races pay the same penalty?* No: one position less. Under the community table, goal races from the 4th race in
a row win more than predicted (1.141 [1.125, 1.158], 1,984 attempts), while chosen races there match.
Three forms, each with the same fitted floor and ceiling, scored on runs left out of the fit (five folds by run,
5,470 runs):

| Form | Goal race at position $j$ pays | Races after it count it | Held out, against the adopted form |
|---|---|---|---|
| Adopted | $\mathrm{PEN}_{\max(j-1,1)}$ | yes | |
| Community table | $\mathrm{PEN}_j$ | yes | 158 ± 34 nats worse ($z$ = 4.72) |
| Exclusion | $\mathrm{PEN}$ at the chosen races before it | no | 1,151 ± 62 nats worse ($z$ = 18.58) |

In-sample the adopted form fits 135 nats better than the community table. Adopted, goal races are calibrated at
each position, and chosen races from the 3rd on; chosen races at the 1st and 2nd fall slightly short, a known miss
(observed / predicted, attempts):

| Race in a row | Goal races | Chosen races |
|---|---|---|
| 1st | 1.001 [1.000, 1.001] (18,815) | 0.996 [0.994, 0.998] (41,537) |
| 2nd | 1.000 [0.999, 1.001] (9,378) | 0.997 [0.994, 0.999] (30,712) |
| 3rd | 1.001 [0.999, 1.003] (3,912) | 0.999 [0.995, 1.003] (8,065) |
| 4th | 0.996 [0.991, 1.001] (809) | 0.994 [0.981, 1.008] (3,525) |
| 5th | 0.999 [0.972, 1.026] (878) | 0.998 [0.959, 1.037] (988) |
| 6th or later | 0.972 [0.908, 1.036] (297) | 0.972 [0.865, 1.079] (272) |

Why a goal race should pay one position less is not known; the runs only show that it does.

*Does the table hold at every chance?* On raised grades:

| Table's chance | Attempts | Observed / predicted |
|---|---|---|
| 110 | 79,866 | 1.000 [0.999, 1.000] |
| 100 | 18,673 | 1.000 [0.998, 1.002] |
| 90 | 6,598 | 1.004 [0.996, 1.013] |
| 80 | 4,477 | 0.992 [0.977, 1.007] |
| 70 or less | 9,574 | 0.949 [0.931, 0.968] |

*When does a pink spark take effect?* From the April of its inspiration. The records say which inspiration fired a
pink, and inspirations sit at fixed turns. On the 502 Senior January-March attempts where the timing matters,
0.737 [0.696, 0.778] were won, against 0.754 predicted with the pink counted from April and 0.864 counted
from the year's start.

*How many chosen attempts is each cell of the table in Findings?*

| Races in a row | AA | AB | AC | BB | AD | BC | AE | BD | CC | BE | CD | AF | AG |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 race | 23,687 | 6,847 | 3,384 | 121 | 1,903 | 256 | 1,609 | 285 | 65 | 159 | 111 | 1,861 | 1,012 |
| 2 races | 19,408 | 4,539 | 1,676 | 177 | 1,417 | 342 | 784 | 309 | 127 | 342 | 191 | 587 | 322 |
| 3 races | 5,527 | 1,247 | 580 | 17 | 231 | 27 | 198 | 65 | 0 | 46 | 1 | 82 | 38 |
| 4 races | 2,461 | 465 | 196 | 29 | 82 | 73 | 37 | 36 | 27 | 21 | 44 | 15 | 8 |
| 5 races | 509 | 265 | 50 | 26 | 21 | 13 | 40 | 8 | 3 | 3 | 5 | 21 | 24 |
| 6 or more | 211 | 43 | 4 | 0 | 3 | 0 | 3 | 0 | 0 | 1 | 0 | 1 | 5 |

*Does the previous race's result matter?* No. Each attempt's chance combines both effects in one formula: her grades
for the race's distance and surface, and her position in the current streak. The penalty counts races in a row, not
wins. The test is the contrast: observed / predicted after a loss divided by the same after a win, with a
run-clustered interval (observed / predicted, attempts):

| Race in a row | After a win | After a loss | Loss / win |
|---|---|---|---|
| 2nd | 0.998 [0.996, 1.000] (37,863) | 0.991 [0.979, 1.003] (2,227) | 0.993 [0.981, 1.005] |
| 3rd | 1.000 [0.997, 1.003] (11,296) | 0.991 [0.972, 1.010] (681) | 0.990 [0.971, 1.010] |
| 4th or later | 0.992 [0.982, 1.003] (6,099) | 1.013 [0.975, 1.051] (670) | 1.021 [0.982, 1.061] |

Every contrast covers 1 ($z$ = 1.04 from the 4th race on): the previous result does not move the next chance.

*Is one formula enough?* Yes, though not exactly. Giving every (chance, streak position, goal or chosen) cell its own
win rate, read straight from its attempts (101 cells), explains the attempts 93 nats better than the
table's one formula in-sample. That is beyond the 62 nats chance allows for its 99 extra parameters if
attempts were independent, but they are not: one trainee's stats carry through her races. Held out, the per-cell rates
do not predict unseen runs better than the table (-11 ± 15 nats, $z$ = -0.73). So the table stands, with the
known misses in Findings.

The test is conditional on IT having chosen each non-goal race; stats are not modelled.

## 5. Affinity

Computed by `research/s05_affinity.py` (the self-grandparent fit).

A legacy's affinity $A_l$ scales her inheritance roll, $r = b\,(1 + A_l/100)$ (section 3). This section defines how
$A_l$ is computed and fits the one rule the game data leave open.

### Findings

- **Affinity adds the characters' relation points (game data) and 3 for each single-race G1 both umas
  won.** The 3 per shared win is **assumed** as the game's rule; in section 3 a free weight on it fits
  0.96 [0.92, 0.99] but predicts unseen runs no better than 1 ($z$ = 0.81).
- **A trainee who is her own grandparent gets no trio points from it.** The game data do not say what a trio with
  one character twice counts. Counting nothing predicts the self-grandparent's inspirations in the runs; counting it
  as the pair is about 29 nats worse.

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
174,056 inspiration trials attributable to a single ancestor. Self-grandparent passes: 1045 observed, 1074.7
predicted with 0, 1241.8 with $\mathrm{tri}(T,P,T) = \mathrm{pair}(T,P)$ (about 29 nats worse). Run-clustered, the observed count lies
−0.96 standard deviations from the prediction with 0 and −6.26 from the one with the pair: 0 is kept and the pair
rejected. Allowing any multiplier $\kappa$, the best fit is $\hat\kappa = 0$, at its lower bound.

The gap is not the ordinary trials' own misfit: with the affinity slope refitted on the ordinary ancestors
(0.97; section 3 keeps 1), the self-grandparent passes lie −0.61 standard deviations from the prediction with 0
and −5.78 from the pair. At slope 1 ordinary parents read 0.975 of prediction and grandparents
0.998 (section 3's known miss at high affinity).

*Does the formula as a whole hold?* Section 3 tests it on the runs' real lineages and shared wins: a free slope on this $A$
fits 0.97 [0.95, 0.99] but predicts unseen runs no better than 1 ($z$ = 0.86); parents at affinity 200 or more
pass less often than predicted, a known miss.

## 6. Loop structures, ranking and schedules

Computed by `research/s06_loops.py`.

A breeding loop is three or four umas bred from each other in turn. This section defines a loop's lineages and how a
loop is scored, and bounds what a different choice of race schedules could gain.

### Findings

- **A four-uma loop has 12 lineages, a three-uma rotation 3.** In a rotation the trainee is her own grandparent,
  and that occurs in practice: 1,911 of 5,534 runs have a grandparent of the trainee's own character.
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

*Does the rotation occur?* Yes: 1,911 of 5,534 runs have a grandparent of the trainee's own character, and
0 have a parent of her character.

*What could a different schedule gain?* (**derived**, on the reference loops.) Schedules are chosen for $\sum_{a<b} o(a,b)$, not for $\bar P_S$. $\bar P_S$ rises with every $o$: $o$ enters each legacy's affinity with
weight +3, and $r$, $q$ and every target's odds are nondecreasing in their inputs. So, with a loop's decks fixed, any
schedule (including one chosen for $\bar P_S$ directly) scores between the odds found and the odds with every pair at
its highest possible overlap. Over 40 reference results, whose odds an independent implementation reproduces to within
2e-15:

- the shared wins found are worth 31-85% over no race affinity;
- the independent ceiling $o_{\max}(a,b) = \sum_r P^{\max}_a(r)\,P^{\max}_b(r)$ caps the gain from a different
  objective at 17.5%. It is far from reachable (the schedules found reach about 69% of it) and cannot
  settle the ranking: 8 of 12 listed runner-up loops reach their winner under it;
- the pair ceilings come from the same search code that chose the schedules, so they are not an independent check, but they sit between
  $o$ and $o_{\max}$ on every pair. They put the ceiling only 0.43-2.02% above the odds found (median 0.84%), and no listed runner-up's ceiling
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
- **Hint frequency scales that number directly**: a card with Hint Frequency +40% gives 40% more hints. The slope is
  **assumed**; on the adopted model a free one does not predict unseen careers better.
- **Specialty priority also raises it.** The game describes Specialty Priority only as making the character join her
  preferred training type more often; in the runs, cards with more of it give more hints, by an amount that differs
  between scenarios. The effect predicts careers it was not fitted to, but it is an association: on the cards whose
  limit breaks change only their specialty priority, the data fit no effect as well as the fitted one, so it is not
  shown to be a cause.
- **Scenarios differ**: each has its own base (table below). The specialty weight differs too: fitted, about
  0.32 in Unity Cup and 0.36 in URA Finale against 0.07 in Grand Concert and 0.11 in Trackblazer, and a weight per scenario
  predicts careers it was not fitted to better than one shared weight. Nothing in the game's description of Specialty
  Priority depends on the scenario, so this weight is an association measured in each scenario's careers, not a game
  constant; why it differs is not tested here.
- **Known misses**: cards differ in hint rate beyond what their effects predict, and a factor per card predicts
  unseen careers better; which card properties explain it is open. Decks with more pals give slightly more hints, and a
  skill also hinted by inspiration is hinted by training less than predicted. SR cards give slightly fewer hints than
  SSR cards with the same effects in-sample, but that does not predict unseen careers better.

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
run, on maximum-limit-break cards, rows where no inspiration hinted the skill, R cards out; 87,801 rows from
15,120 card-runs; the Runs column counts the runs that add a row; *Adopted*: the values the model uses, fitted
on an earlier, smaller set of the runs:

| Scenario | $\beta$ | $d$ | Adopted $\beta$ / $d$ | Rows | Runs |
|---|---|---|---|---|---|
| Unity Cup | 2.395 | 0.322 | 2.36 / 0.36 | 17,737 | 1211 |
| Grand Concert | 1.999 | 0.068 | 1.97 / 0.08 | 55,136 | 3,194 |
| URA Finale | 1.380 | 0.359 | 1.34 / 0.39 | 12,551 | 736 |
| Trackblazer | 1.702 | 0.107 | 1.70 / 0.11 | 2,377 | 100 |
| Grand Masters | no runs | | 1.86 / 0.26 (**assumed**: one $\beta$ and one $d$ over every scenario) | | |

### Evidence

*Specialty priority.* Without the specialty term, cards with more specialty priority give more hints than predicted;
with it, the excess is gone:

| Specialty priority | Rows | Observed / predicted, without the term | with it |
|---|---|---|---|
| 0 | 29,193 | 0.965 | 1.003 |
| 1-29 | 691 | 0.966 | 0.986 |
| 30-59 | 40,466 | 1.001 | 0.993 |
| 60+ | 17,451 | 1.059 | 1.009 |

- It predicts careers it was not fitted to: held out, the term gains 38.6 ± 5.0 nats, $z$ = 7.8; fitted on the earlier
  half of the runs, it predicts the later half better (10.1 ± 3.6 nats, $z$ = 2.8).
- A weight per scenario predicts better than one shared weight (13.2 ± 3.0 nats, $z$ = 4.5).
- Split by scenario and specialty band, the 13 cells with 200 or more rows are jointly consistent with the
  model (run-clustered $\chi^2$ = 16.0, p-value 0.25).
- It is not one card's, one rarity's or one period's: one scale $k$ on the adopted per-scenario weights ($k$ = 1 on
  every row), refitted on either half of the runs, on SR or SSR cards alone, and without any one of the eight
  most-used cards, stays between 0.65 and 1.12.
- Cause or association: the same card at different limit breaks has different specialty priority but is otherwise
  the same card. On most such cards hint frequency changes with limit break too, so their weight
  (0.29 [−0.01, 0.71] over 64 cards, a base per card and scenario) also absorbs any misfit of the
  hint-frequency term. On the 27 clean cards, whose hint frequency is the same at every limit break, it is
  −0.06 [−0.44, 0.56] (+0.03 nats over no term): consistent with no effect and with every per-scenario
  weight, so the data do not decide whether specialty priority itself causes the hints.

*Hint frequency's slope.* The $1/100$ in $(1 + hf/100)$ is **assumed** (a card's +40% read as 40% more hints) and
tested: on the adopted model a fitted slope ($hf/90$) is detectable in-sample but does not predict unseen careers
better (1.8 ± 2.0 nats, $z$ = 0.9). In the form study below, without the specialty term, a free divisor
($c$ = 83) does predict unseen careers better (16.7 ± 2.6 nats, $z$ = 6.4). The adopted model keeps $1/100$: an
assumption that, with the specialty term in, unseen careers do not reject.

*The form.* Fixed on every card at any limit break, R cards included, without the specialty term
(140,615 rows from 5,534 runs):

| Question | Evidence |
|---|---|
| Rate form, LL on 140,615 rows | $1 + hf/25$ over options −92,742.3; $1 + hf/25$ over skills −90,327.3; constant −87,864.8; $\beta(1 + hf/100)$ −86,963.8; per-scenario $\beta(1 + hf/100)$ −86,406.6; free divisor ($c = 83$) −86,389.7. Held out, per-scenario $\beta(1 + hf/100)$ predicts unseen runs better than the constant (1457.8 ± 31.6, $z$ = 46.1) and the $hf/25$ forms (6334.1 ± 81.0, $z$ = 78.2; 3919.0 ± 60.8, $z$ = 64.5) |
| Skills only, not skills + stat option | in 71 of 91 (card, $hf$) cells the draw-count variance is below the binomial floor a drawn stat option implies |
| Uniform draws with repeats | repeat rate $P(k \ge 2 \mid k \ge 1)$: 0.139 observed, 0.139 two-point count, 0.191 Poisson count; distinct skills 1/2/3/4 per card-run 0.142 / 0.575 / 0.250 / 0.032 observed, 0.120 / 0.566 / 0.273 / 0.039 model |
| Covariates | per-scenario bases +557 nats (form study; held out 556.4 ± 20.8, $z$ = 26.7). On top of the adopted rate (87,801 rows): card type +2.8 nats (4 parameters), initial bond +0.3, pool size +0.6, none significant; rarity (SR against SSR) +3.1 and pals per deck +12.5 are detectable; pals per deck predicts unseen runs better (held out 12.2 ± 2.9, $z$ = 4.3; a known miss, factor 1.027 per pal), rarity does not (2.1 ± 1.4, $z$ = 1.5; factor 0.970 for SR) |
| Per card | on the adopted rate with a level per scenario, the 58 cards with 200 or more rows (83,075 rows) read 0.80-1.17 of prediction, jointly far from it (run-clustered $\chi^2$ = 445 on 58, $p$ = 4e-61); a factor per card predicts unseen runs better (48.7 ± 6.0, $z$ = 8.1): a known miss, not modelled; which card properties explain it is open |
| Lv-5 cap | beats drawing with replacement by 22.1 nats with no parameter; obs / pred by draws to Lv 5, replacement vs cap: 5 draws (44,737 rows) 0.967 / 0.975, 3 (3,011) 0.945 / 0.952, 2 (45,649) 1.032 / 1.031, 1 (465) 1.221 / 0.968 |
| Joint draws over targets | 247,441 pairs of no-spark pool skills on one card-run (Trackblazer left out): $P(\text{both})$ 0.0792 observed, 0.0794 joint, 0.1094 independent; $P(\text{neither})$ 0.4411 vs 0.4380 |
| Pool size | obs / pred 0.92-1.11 for every $n$ with at least 150 rows |
| Per-draw increment | 37,333 of 37,456 single-source levels are multiples of $\iota$, capped at 5 |
| $hf$ by card level | floored linear interpolation −78,362.7, step −78,371.3, max-limit-break $hf$ for all −78,396.3 |

*Other sources of the same skill.* The fit uses rows where no inspiration hinted the skill. Where something else also
hinted it, the card's own hint is seen this often (scored against the adopted rate on maximum-limit-break cards, R
cards out):

| Other source of the skill | Rows | Observed | Predicted | Ratio |
|---|---|---|---|---|
| none | 77,728 | 0.341 | 0.336 | 1.01 |
| inspiration | 8,883 | 0.262 | 0.354 | 0.74 |
| inspiration + card event | 409 | 0.213 | 0.348 | 0.61 |
| card event | 2,191 | 0.332 | 0.333 | 1.00 |
| other card's training | 3,557 | 0.344 | 0.355 | 0.97 |

With no other source the adopted rate slightly under-predicts (1.013 [1.007, 1.019]). A skill hinted by inspiration
is hinted by training less than predicted (not modelled). It is not that such a skill
leaves the card's pool: with two pool skills hinted by inspiration, the rest are hinted at 0.371, against
0.338 if nothing leaves and 0.459 if they leave.

*Prioritised skills.* 1201 runs show a priority list; prioritised skills are hinted by training at 0.336 against
0.323 predicted (868 rows): no effect, as the model assumes.

Against real careers (section 13), training-pool hints read 1.004 of the model.

## 8. Chain events

Computed by `research/s08_chains.py`.

A chain card hints its skills through a fixed sequence of events, so how far a chain gets depends on how many chain
events a career gets and how they are shared among the deck's chain cards. This section fits both.

### Findings

- **A career gets a random number of chain events, Binomial(21, 0.35)** (mean 7.35), the same for every deck and
  scenario (scenarios show no difference; two deck and trainee effects are known misses). No alternative predicts unseen runs better, but in-sample a free distribution rejects it, a known miss;
  of the two-parameter under-dispersed families the binomial fits best. Binomial(21, 0.35) is kept by choice;
  Binomial(18, 0.405) is the candidate the runs now favour in-sample.
- **Each event goes to a chain card in proportion to its remaining links.** This fits 102 nats better than equal shares
  per card.
- **Pal events do not use up chain events**: decks with more pals do not get fewer.
- **Known misses**: decks with Sasami Anshinzawa [This Might Sting!]'s pal card reach more chain events and Inari
  One's careers fewer, which the model's single $N$ does not have; and, causes unknown, in crowded decks early links
  run short and more chains never start than predicted; final links fit pooled, but individual cards differ far beyond
  chance.

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

*How is a chain's progress read?* From marker skills, on 5,316 runs (the pal-count table below). A link is observable
only if every choice and outcome of its event carries a skill appearing in no other event of that card. A credit
naming two sources ("A or B event") counts as unknown when B is another deck card whose events can give the marker, and
so does a marker listed only under inspiration. $N$'s distribution is the one that best explains progress over all
decks.

The fit allows a fired link's marker to go unseen with chance 0.02 (a measurement allowance, set to the measured rate). Without it,
a run in which a card's later link shows while an earlier one does not has likelihood 0. Since links fire in order,
those runs measure the rate: of 1,800 earlier links implied fired by a later one, 38 show no marker, 0.021
[0.014, 0.028]. The cause is not known. The fit does not depend on it: with allowances from 0.01 to 0.045, or
with none on the 5,280 runs that show no such case, the best $p$ at $n = 21$ is 0.34-0.35.

*Which distribution for $N$?* In-sample, against the free distribution:

| Form for $N$ | Result |
|---|---|
| free distribution | mean 7.22, variance 4.44 |
| Binomial(21, 0.35) | 21.9 nats behind the free distribution with 14 fewer parameters (rejected, p-value 6.5e-05) |
| Binomial($n$, $p$), both fitted | best at $n$ = 17, $p$ = 0.426: 12.1 nats behind (p-value 0.042); $n$ is only known to lie in 16-19 |
| Conway-Maxwell-Poisson | 16.9 nats behind (p-value 0.002) |
| Binomial(15, 0.48) | 19.4 nats behind |
| Poisson(7.1) | 136.1 nats behind (variance = mean, too wide; rejected) |
| per scenario | means Unity Cup 7.60, Grand Concert 7.14, URA Finale 7.02, Trackblazer 6.99; +29.6 nats for 48 extra parameters (not significant) |

Why a binomial: the free fit's variance is 0.62 of its mean, so $N$ is under-dispersed, which rules out the
Poisson and every over-dispersed count. A binomial (a fixed number of chances, each taken with the same probability)
is the simplest under-dispersed count, and it fits better than the Conway-Maxwell-Poisson, the other two-parameter
family that can be under-dispersed (by 4.7 nats, same parameter count). What the model uses is the distribution of
$N$.

How $n$ and $p$ are read: fitting both $n$ and $p$, the runs fix the mean (7.25 at the best $n$) and, more
loosely, the spread, so $n$ and $p$ trade off along a ridge. Re-fitting $p$ at each $n$, every $n$ from
16 to 19 lies within 1.92 nats of the best (the 95% interval), so the best single pair is not itself a finding. The
adopted $n = 21$ lies just outside that interval in-sample (6.41 nats below the best $n$; variance
$21 \cdot 0.35 \cdot 0.65 = 4.78$ against 4.44 free).

*Does that in-sample preference survive held out?* No. Each of five groups of runs is scored by every form fitted on
the other four, and each form is compared run by run with Binomial(21, 0.35):

| Form, fitted on the other four groups | Held-out nats over (21, 0.35) ± SE | $z$ |
|---|---|---|
| free distribution | +12.6 ± 6.6 | +1.90 |
| binomial, $n$ and $p$ by likelihood | +6.4 ± 4.8 | +1.34 |
| binomial by moments | +4.7 ± 4.0 | +1.17 |
| Conway-Maxwell-Poisson | +2.0 ± 4.9 | +0.41 |
| Poisson | −108.8 ± 11.6 | −9.37 |

No form predicts unseen runs better than (21, 0.35) ($z$ above 1.96 would), and the Poisson predicts them worse. The
model keeps (21, 0.35), a decision rather than a fit: in-sample it is rejected, and the candidate Binomial(18, 0.405)
(mean 7.29, variance 4.34, inside the profile interval) fits the runs 8.8 nats better, while a binomial refitted
by likelihood on each four groups gains 6.4 ± 4.8 nats held out ($z$ = 1.34), short of significance.

*Does $N$ depend on the deck or trainee?* For two, yes. Decks with Sasami Anshinzawa [This Might Sting!]'s pal card
(639 runs) reach a mean of 7.74 chain events against 7.17 on the other runs ($p$ at $n = 21$ 0.374
against 0.342, likelihood-ratio $p$ = 3e-07); Inari One's careers (473) reach 6.83 against 7.26
($p$ 0.326 against 0.347, likelihood-ratio $p$ = 0.001). The model has one $N$ for every deck and
trainee, so it under-predicts the first and over-predicts the second (known misses). On the runs with neither,
Binomial(21, 0.35) is still rejected against a free distribution ($p$ = 0.002).

*Do pals change $N$?* No. Fitted per pal count, the means are 7.36 / 7.05 / 7.41 / 8.40 for
1 / 2 / 3 / 4 pals (4,791 runs): more pals do not lower $N$, so pal events do not consume chain events. (Per
scenario, see the table above.)

*Are final links reached as often as predicted?* Not everywhere: the binomial lies inside the observed 95% interval
at 3, 1 and 0 pals, above it at 5 and 2 pals and below it at 4 pals.

| Pals | Chain cards | Runs | Final link reached: observed [95%] / Binomial(21, 0.35) / Poisson(7.1) |
|---|---|---|---|
| 5 | 1 | 127 | 0.952 [0.899, 0.978] / 0.979 / 0.961 |
| 4 | 2 | 552 | 0.892 [0.869, 0.915] / 0.860 / 0.794 |
| 3 | 3 | 1,426 | 0.583 [0.565, 0.602] / 0.566 / 0.514 |
| 2 | 4 | 1,399 | 0.318 [0.304, 0.332] / 0.342 / 0.326 |
| 1 | 5 | 1,414 | 0.224 [0.213, 0.234] / 0.225 / 0.221 |
| 0 | 6 | 398 | 0.146 [0.130, 0.163] / 0.158 / 0.159 |

*Spread over slots, or over cards?* Over slots. The alternative gives each running card's next event the same chance
(uniform over cards). On 5,316 runs:

| | Uniform over remaining slots (adopted) | Uniform over cards |
|---|---|---|
| Total LL | −11,046.4 | −11,148.4 |
| Final links, 2 pals (observed 0.32) | 0.34 | 0.41 |
| Final links, 0 pals (observed 0.15) | 0.16 | 0.23 |

The slot rule fits better by 101.9 nats. With no pals it lies inside the observed interval and the card rule
outside; with two pals both predict more final links than seen, the card rule further. What each rule implies with
Binomial(21, 0.35), as P(reach link 1 / 2 / 3):

| Deck | Slots (adopted) | Cards |
|---|---|---|
| two 3-link SSRs | 1.00 / 0.97 / 0.87 | 0.99 / 0.96 / 0.87 |
| 3-link SSR + 2-link SR | SSR 1.00 / 0.99 / 0.93, SR 0.99 / 0.95 | SSR 1.00 / 0.98 / 0.93, SR 0.99 / 0.96 |
| three 3-link SSRs | 0.98 / 0.84 / 0.54 | 0.96 / 0.81 / 0.58 |
| four 3-link SSRs | 0.93 / 0.65 / 0.25 | 0.88 / 0.62 / 0.33 |

For example, with two 3-link cards and exactly 3 events, the slot rule gives one card all three with chance
$\tfrac36 \cdot \tfrac25 \cdot \tfrac14$ = 5.0%, the card rule $(\tfrac12)^3$ = 12.5%.

*Do two-chain decks finish as predicted?* Yes. Decks with two 3-link SSRs and four pals finish both chains in
217 of 259 runs, 0.84 [0.79, 0.88], against 0.80 predicted.

*Known miss: early links in crowded decks run short.* The shortfall is the share of early links the model expects to
fire minus the share seen, ± its run-clustered standard error; $z = \sum(\text{pred} - \text{obs}) /
\sqrt{\sum(\text{pred} - \text{obs})^2}$, with each run's links summed as one cluster:

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Early-link shortfall | 0.02 ± 0.01 | 0.04 ± 0.01 | 0.08 ± 0.01 | 0.05 ± 0.01 | 0.14 ± 0.02 |
| $z$ | 1.3 | 4.7 | 6.5 | 3.7 | 5.8 |

*Known miss: individual cards' final links.* Pooled, final links fit (0.348 observed against 0.353). Card by
card they do not: over the 50 cards with 60 or more final-link observations, a joint test that every card
matches the model (clustered by run, since one run's chain cards share its $N$) gives $\chi^2$ = 386 on
50 degrees of freedom, where chance alone gives about 50. The cause is not known.

*Known miss: chains that never start.* More chains never start in crowded decks than predicted:
0.19 against 0.13 with five chain cards, 0.34 against 0.20 with six. A per-card activation parameter (a chain
card active with probability $a$), fitted jointly with $N$, gains −3.0 nats ($a = 0.97$): rejected.

## 9. Event choices and outcomes

Computed by `research/s09_outcomes.py` (the certainty checks: `research/s13_calibration.py`).

An event can put different skills on different choices, and an outcome can be random. This section states which
choice IT takes and measures the outcome shares that are not equal.

### Findings

- **The model assumes IT takes an event's top choice unless a prioritised skill sits on another**, and that the
  player prioritises every target. In the two events where priority lists show the choice taken, IT follows the rule.
  Its consequence shows on real careers: event-only hints of targets not prioritised, or outranked (below), fall
  clearly below those of prioritised ones (*prioritised* in Terms) (0.653 [0.613, 0.694] of the model against 0.970).
- **Known miss**: event-only hints of prioritised targets read just below the model, 0.970
  [0.940, 0.9997].
- **Outcomes are equally likely unless measured.** Five measured outcomes depart from equal shares, and the model
  uses measured values for them, for example Nakayama Festa's Nothing Ventured at 0.61.
- **Some other outcomes are off the equal rule but left at it**: known misses.
- **Pal and group finales are modelled as certain**, and both checked hold: Light Hello and Team Sirius credit their finale
  skill in every run carrying them.

### Terms

- *Choice*: an option of an event. *Top* and *bottom choice*: the first and the last.
- *Outcome*: what a choice gives. *Firing*: one occurrence of an event in a run.
- *Prioritised*: on the run's priority list, and not outranked: a listed target counts as not prioritised when a
  higher-ranked prioritised skill sits on a rival choice of every event offering it, since between two prioritised
  skills on rival choices IT is taken to choose the higher-ranked one (15 event-only rows; which way the rank
  runs is assumed, not tested).
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
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante", Nothing Ventured (671 runs with the card, below) | 151/247 = 0.61 [0.56, 0.66] | 0.60 |

The other outcomes: Riko Kashimoto's alternative is Maverick ○, Seiun Sky's Escape Artist, Smart Falcon's Prudent
Positioning.

### Evidence

*Does IT follow the choice rule?* Where it can be seen, yes. At Sasami Anshinzawa [This Might Sting!]'s finale, with
Risky Business prioritised IT took choice 2 in about 95% of runs (3 of the 4 exceptions had Risky
Business already at hint Lv 5); with neither skill prioritised, 0 of 35 runs took choice 2. At Riko Kashimoto
[Planned Perfection]'s event, with Lone Wolf prioritised IT took the bottom choice in 55 of 62 runs. Every run
has a priority list, but the run records show it for only 1201 of 5,534 (the rest count as unknown, never as an
empty list). So the rule is tested mainly through its consequences (section 13): event-only hints read
0.970 [0.940, 0.9997] of the model for prioritised targets and 0.653 for the others. The
prioritised rows read just below the model, a known miss; the model's assumption that every target is prioritised
over-predicts the rest by far more.

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

*How is Nakayama Festa's share read?* On every run with her card (671). Her event is credited to her alone with
Nothing Ventured in 151 runs and with Risky Business in 71. In 58 more, Risky Business is credited to "her or
another card's event". The other card is the only one credited with it in 0.848 of runs without her (Team Sirius, 48 of
the 58) or 0.445 (Sasami Anshinzawa), and she gives it in 0.111 of hers. So a shared credit is hers with
chance $r_F / (1 - (1 - r_F)(1 - r_o))$, where $r_F$ is her rate and $r_o$ the other card's: 8.3 of the 58.
In 43 runs neither outcome shows while Risky Business is already at hint Lv 5 (a Risky Business win cannot
show, or the event never fired); they are split by the rate at which the event shows where both outcomes can,
0.38. The share is then 151/247 = 0.61 [0.56, 0.66]. It stays within 0.504-0.635 whatever share of the
shared credits is hers, and shows no dependence on the trainee's final Wit.

*Which shares are left at the equal rule?* Off 0.5 but left at the equal rule (known misses): Tazuna Hayakawa SSR
finale Concentration 0.70, Agnes Tachyon [Q≠0] link 1 choice 2 Murmur 0.70, Marvelous Sunday [Dazzling Day in
the Snow] link 3 Position Pilfer 0.58, Light Hello SSR finale +3 See Ya Later! 1,497 of 2,103
(0.71). Consistent with 0.5: Gold Ship [That Time I Became the Strongest] 0.61, King Halo [Tonight, We
Waltz] 0.57, Daitaku Helios [Sunlit Angel ♪] Tail Held High 0.47 (link 2, on runs where link 3 visibly fired).

*Are pal and group events certain?* Their events are entered per card with a probability, finales certain; first-year
date events are left out in IT (**assumed**). The finales check out: Light Hello [From the Ground Up] credits See Ya
Later! to her event in 1679 of 1679 runs carrying her, and Team Sirius is credited with Risky Business (alone or shared) in 65 of 65.

## 10. Random card events (not modelled)

Computed by `research/s10_random_events.py`.

The model leaves some events out. This section measures how often a deck character's shared random event fires and
what leaving it out costs on real careers.

### Findings

- **A deck character's shared random event fires in about 0.14 of the careers it can fire in**, a lower bound. The
  rate falls a little as more such events are on offer, but a constant rate predicts unseen runs as well.
- **Leaving them out under-predicts the targets they can hint**: acquisition there reads 1.25 [1.12, 1.38] of the
  model. Credited at the fitted rate, it reads 1.01.
- **Crediting them is not shown to predict unseen careers better** ($z$ = 1.80, short of 1.96).
- **The card's initial friendship gauge raises the rate** (a known miss of the measured rate; friendship bonus and
  specialty priority show no effect).
- **Two default targets are affected**: Vodka's Nimble Navigator and Nice Nature's Ramp Up, the only ones that are a
  character's top-choice marker.

### Terms

- *Shared random event*: a character's random event that can fire in another uma's career when a card of hers is in
  the deck (any rarity, R cards included).
- $K$: the number of random events on offer: every shared event of the deck's characters plus the trainee's own.
- *Top-choice marker*: a marker skill on an event's top choice. *Bottom-choice skill*: a skill on its bottom choice.
- *Reading*: one shared event in one run, read through its top-choice marker. A *clean hit*: the marker credited to
  that character's event. An *ambiguous credit*: a credit naming two cards' events; when the other card's events can
  also give the marker, the reading is unknown and left out (section 8's rule).
- *Slot form*: a form in which every event slot competes. $T = K + w \cdot \text{pals}$ counts them, $w$ being what a
  pal weighs in random events.

### Model

The model leaves these events out. The fitted rate (**fitted**) is about 0.14 per event
per career. It falls a little with $K$:

$$P(\text{fires}) = 0.18\,(1 - 0.034)^{K - 4},$$

which gives 0.18 at $K = 4$ and 0.12 at $K = 14$. The slope is detectable (run-clustered Wald test, $|z|$ =
2.91) but does not predict unseen runs better than the constant rate.

### Evidence

*How is the rate read?* From top-choice marker skills credited to the character's event: 8,169 readings from
4,232 runs, 1,128 clean hits. Each of the 222 ambiguous credits names another deck card whose events
can give the marker, so, as in section 8, they are left out as unknown; counted as hits, they would raise the rate by
about 2 points. A reading also misses a firing when the marker was already at hint Lv 5 from other sources
(153 readings): where the event could show, the rate is 0.141, or 0.164 counting the ambiguous credits as hits.
A firing is missed too when IT takes another choice for a prioritised skill (bottom-choice rates below). The rate is
therefore a lower bound.

*Which form?* None beats the constant rate held out:

| Form | LL (in-sample) | Held out, against the line above it |
|---|---|---|
| constant 0.138 | −3,279.6 | |
| $0.18(1 - 0.034)^{K-4}$ | −3,275.6 | +1.8 ± 2.9 nats ($z$ = +0.63) |
| power law $c(K/8)^{-a}$, $a = 0.32$ (uniform competition would be $a = 1$) | −3,275.5 | +0.1 ± 0.5 against the $K$ form ($z$ = +0.28) |
| strict competition $1 - (1 - 1/K)^S$, $S = 1.47$ | −3,295.8 | −18.0 ± 6.4 against the $K$ form ($z$ = −2.82) |
| slot form, $c / T$ (chain links weigh 0), $c = 1.75$ | −3,275.5 | −4.0 ± 2.2 against the $K$ form ($z$ = −1.80) |

Held out (five groups of runs, each scored by the form fitted on the other four), no form predicts unseen runs better
than the constant rate, and strict competition predicts them worse. Within the slot form, chain links take no weight
(weight 1 is rejected, run-clustered score test) and pals compete: a pal weighs $w$ = 1.15 [0.65, 1.65] random
events (run-clustered interval), with 0 excluded.

*What else moves the rate?* Little. The observed hit rate by $K$ = 4 / 6 / 8 / 10 / 12 / 14 is 0.17 / 0.18 / 0.16 / 0.14 / 0.13 / 0.13, and
within a run events fire independently. A logistic slope in $K$ gives 15.3% at 8 on offer and 12.4% at 14.
Of the card effects, fitted together next to $K$, only the initial friendship gauge is detectable, and it raises the
rate: +0.102 [+0.041, +0.164] on the logit scale per 10 points (Holm-adjusted run-clustered $p$ = 0.0034; on
cards at LB4 alone, +0.101). Friendship bonus (−0.050, $p$ = 0.60) and specialty priority
(+0.016, $p$ = 0.60) show none. The model leaves these events out, so this changes no prediction; it
is a known miss of the measured rate. Rarity within a character shows no
effect (pooled, SSR markers fire at 14.6% against SR 11.6%). Scenario is not detectable (run-clustered $p$ =
0.19), and a rate per scenario does not predict unseen runs better ($z$ = −0.75).

*Are bottom-choice skills credited?* Rarely. In the 1201 runs showing a priority list, 33 of 2247
candidates were prioritised. Without a priority they were credited in 0.076 [0.065, 0.088] of candidates, against
0.138 for top-choice markers; with a priority, 4 of 33.

*What does leaving them out cost on real careers?* Section 13's rows, each on the run's own deck. This test scores
whether the target was acquired from any source, so it does not depend on reading the event. Of the default targets
only two are a character's top-choice marker (Vodka's Nimble Navigator, Nice Nature's Ramp Up).

- On the 305 run-target rows where a deck card's random event could hint the target, it was credited to that
  event 35 times, 22 with no other source. The model gives 73 of these rows no chance, and the
  event hinted the target on 6 of them.
- Left out, acquisition there reads 1.25 [1.12, 1.38] of the model, an under-prediction. Credited at the
  fitted rate (about 0.14 there), it reads 1.01 [0.90, 1.11].
- Held out (the rate fitted on the other four groups' readings), crediting gains 5.0 ± 2.8 nats on
  these rows ($z$ = 1.80): positive, but short of the 1.96 that would show it predicts unseen careers better.
- Crediting raises an affected run's odds over the targets it can land by a median 13% (at most
  156%; 232 runs).
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
  1.10 [0.81, 1.40] of the model, and crediting the measured rates gains only 0.5 nats.
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
5 of 89 rows, 0.056 [0.024, 0.125]: Daiwa Scarlet, any outfit, 0 of 84 (Nimble
Navigator); Mejiro Ardan, any outfit, 4 of 4 (Ramp Up). Left out, acquisition there reads
1.10 [0.81, 1.40] of the model, and crediting the measured rates gains only 0.5 nats.

*And Vodka?* Vodka, any outfit, gets Nimble Navigator from her own events in 25 of 27 runs
[0.77, 0.98]; her random and conditional events both give it and cannot be told apart. With her rows, the model
left as it is reads 1.33 [1.12, 1.54], and crediting gains 16.2 nats. Not modelled, so the model
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

Given chain state $c$, let $H$ describe the joint training and event outcomes. Once both $c$ and $H$ are fixed,
the remaining per-target landing factors multiply:

$$P(\text{all}) = \sum_{c} P(c)\, E_{H\mid c}\!\left[\prod_{k} p_k(c,H)\right] \qquad (\text{exact, **derived**}).$$

The expectation includes each card's joint training draws (section 7) and shared event outcomes (section 9).
Multiplying the targets' marginal chances given $c$ alone would discard those dependencies. The implementation
enumerates connected target groups and multiplies independent groups (`joint_eval_in`).

*Real legacies.* The same model with each veteran's real stars, real G1 wins and recorded parents (an unknown parent
brings no relation points; her sparks count); $n_k$ = the legacies actually carrying $k$. Since $q_k$ and $e_k$ enter
only as $(1 - e_k)(1 - q_k)$, $e_k$ is replaced by $1 - (1 - e_k)(1 - q_k)$, and $1.1^{n_k - 6}$ scales $P(\text{all})$
(**derived**).

## 13. Calibration against real careers

Computed by `research/s13_calibration.py`.

This section rebuilds real careers as the model sees them and compares what the model predicted with what happened.

### Findings

- **The model's acquisition odds are calibrated where its priority assumption holds**: observed / predicted
  0.999 [0.991, 1.006] on the rows whose target is prioritised or whose run shows no priority list. On
  all rows they read 0.988 [0.981, 0.994]; 92% of the shortfall is on rows the run's list shows not prioritised
  (0.912 [0.893, 0.930]), where the model's assumption does not hold.
- **Inheritance is calibrated apart from one target**: without Tail Held High, 0.992 [0.982, 1.003].
  Tail Held High, the only target of eight whose interval excludes 1, is a known miss (0.953 [0.918, 0.988];
  section 3).
- **Deck hints read optimistic** (0.929 of the model), almost all from the priority assumption: event hints land at
  0.65 of the model for targets not prioritised. Prioritised targets' event hints read just below it,
  0.970 [0.940, 0.9997], a known miss. Unmodelled scenario and trainee events supply the few
  points the deck loses.
- **Trackblazer's training-hint base over-predicts** these runs (0.779 of the model); the other scenarios' intervals cover 1.
- **Landing all eight targets reads 0.93 of the model**: consistent with calibrated, though only 220 runs acquired
  every open target and the interval allows up to about 1.2× optimism.
- **Beyond the eight targets**, inheritance reads slightly short of the model (0.986 [0.973, 0.999], spread
  over many skills: a known miss), and training-pool hints are under-predicted (1.065 of the model).

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

Each of 5,198 real careers (URA Finale 754, Unity Cup 1,148, Grand Concert 3,213,
Trackblazer 83) is rebuilt as the model sees a career: the trainee in her outfit, the scenario, the six cards at
their own limit breaks, both parents with their recorded parents and every ancestor's white sparks, and race wins
giving the recorded shared-G1 counts. The model then gives, for each default target of that career, the chance of
each step (sections 3-12), and the run record says what actually happened. There are 41,584 rows. Certain rows
(2,001 on her outfit, 2,296 scenario golds) could only count as successes, so they are left out of tests 1 and 3.

Each test reports observed / predicted over a set of rows: 1 means calibrated, above 1 the model is pessimistic, below
1 optimistic. Intervals are 95%, clustered by run, since a run's eight targets share its deck and lineage (*How to
read*). The runs end before skills are bought, so "acquired" means hinted and buyable, the model's own meaning.

The four tests, each isolating one part of the model:

1. **Deck hint**: does a deck card hint the skill (by training or by its events) as often as sections 7-9 predict?
   The prediction is $d^g + (1 - d^g)d^w$, the chance the deck hints the gold or else the white; observed is a deck
   card credited with a hint of the skill. Only rows where no inspiration hinted the skill are used: the hint rates are
   fitted on such rows, and with an inspiration a card's hint is seen less often (section 7). The rows are split by
   where the deck's chance comes from: **training pool only** (only cards' training can hint the skill) and **events
   only** (only cards' events can); events only is split again by whether the target was **prioritised** (on the
   run's list and not outranked, section 9), for runs that show a list.
2. **Inheritance**: does an inspiration of the skill fire as often as section 3 predicts ($q$)? Observed: a spark of
   the skill fired at the Classic or Senior inspiration.
3. **Acquired**: does the skill end up hinted, from any source, as often as all modelled sources together predict?
   This is what the model's odds rest on. It is also read on the rows the model's priority assumption fits: the
   target prioritised, or the run showing no list.
4. **Landing**: the model's chance that the target lands as a spark ($p$), against the same chance computed from
   what the run actually acquired ($0.4 \cdot 1.1^n$ if the gold was hinted, $0.2 \cdot 1.1^n$ if only the white). It
   checks the acquired step through the spark step; it does not test spark generation, which the runs do not show
   (section 1).

| Test | Rows | Predicted | Observed [95%] | Obs / pred [95%] |
|---|---|---|---|---|
| 1. Deck hint, no inspiration | 24,526 | 0.254 | 0.236 [0.233, 0.239] | 0.929 [0.915, 0.942] |
| 1. Training pool only | 5,036 | 0.357 | 0.358 | 1.004 [0.969, 1.038] |
| 1. Events only | 3,538 | 0.896 | 0.798 | 0.891 [0.878, 0.903] |
| 1. Events only, target prioritised | 551 | 0.859 | 0.833 | 0.970 [0.940, 0.9997] |
| 1. Events only, not prioritised | 584 | 0.884 | 0.577 | 0.653 [0.613, 0.694] |
| 2. Inheritance | 41,584 | 0.359 | 0.354 [0.351, 0.358] | 0.987 [0.978, 0.997] |
| 3. Acquired, any source | 37,287 | 0.512 | 0.506 [0.502, 0.509] | 0.988 [0.981, 0.994] |
| 3. Acquired, deck or inspiration only | 37,287 | 0.512 | 0.497 | 0.972 [0.965, 0.978] |
| 3. Acquired, any source, prioritised or no list shown | 32,848 | 0.506 | 0.505 | 0.999 [0.991, 1.006] |
| 3. Acquired, deck or inspiration only, prioritised or no list shown | 32,848 | 0.506 | 0.497 | 0.982 [0.974, 0.989] |
| 3. Acquired, prioritised | 3,556 | 0.700 | 0.706 | 1.009 [0.993, 1.025] |
| 3. Acquired, not prioritised | 4,439 | 0.558 | 0.509 | 0.912 [0.893, 0.930] |
| 4. Landing (sums of $p$) | 37,287 | 6,929.5 | 6,824.9 | 0.985 |

### Evidence

*Does the priority list matter?* Yes. On the runs that show a priority list, prioritised targets are acquired at the
model's rate (1.009 [0.993, 1.025]) and the others less often (0.912 [0.893, 0.930]); the
pooled test averages the two with the runs that show no list. Of the shortfall over all rows,
92% is on the non-prioritised rows.

*Where does the deck lose its points?* Almost all in event hints of targets the player did not prioritise (test 1's
split above). Unmodelled scenario and trainee events supply the few points the deck loses: 299 target acquisitions
in 37,287 rows, 0.8%. On the rows the priority assumption fits, the deck and inspiration alone read
0.982 [0.974, 0.989] of the model, and every source together 0.999 [0.991, 1.006].

*Per target* (observed / predicted):

| Target | Deck hint | Inheritance | Acquired | Landing |
|---|---|---|---|---|
| Uma Stan / Superstan | 0.80 [0.70, 0.91] | 0.996 | 0.977 | 0.980 |
| Nimble Navigator / No Stopping Me! | 1.05 [0.96, 1.16] | 0.984 | 1.051 | 1.037 |
| Playtime's Over! / See Ya Later! | 0.985 [0.97, 1.00] | 1.000 | 0.991 | 0.996 |
| Ramp Up / It's On! | 0.826 [0.79, 0.87] | 1.009 | 0.929 [0.90, 0.96] | 0.923 |
| Tail Held High / Tail Nine | 0.986 [0.93, 1.04] | 0.953 [0.92, 0.99] | 0.985 | 0.985 |
| Racing Spirit: Stamina | no deck source | 0.999 | 1.042 | 1.035 |
| Slipstream / Perfect Spot! | 0.990 [0.92, 1.06] | 0.974 | 0.993 | 0.954 |
| Risky Business / Nothing Ventured | 0.828 [0.77, 0.89] | 0.928 | 0.892 [0.85, 0.94] | 0.882 |

*Training-pool hints by scenario* (a test of $\beta$ and $d$, section 7): Grand Concert 1.024 [0.98, 1.06], Unity Cup
0.932 [0.83, 1.03], URA Finale 1.013 [0.90, 1.12], Trackblazer 0.779 [0.63, 0.93]. Only
Trackblazer's interval excludes 1, its $\beta$ over-predicting these runs. By the lowest limit break of the sourcing
cards: LB4 1.012, LB3 0.991, LB0-2 0.87-0.98 (148-242 rows each, every interval covering 1).

*Unity Cup's guaranteed golds* show in 1148 of 1148 Unity Cup runs each.

*Gold acquisition* (observed / predicted): Nothing Ventured 0.852 [0.80, 0.90] (1260 rows), Perfect Spot!
0.835 [0.78, 0.89] (684), It's On! outside Unity Cup 0.456 [0.33, 0.62] (161), No Stopping
Me! outside Unity Cup 0.958 (77), See Ya Later! 1.000 (4,488).

*Reliability.* Rows grouped by the model's predicted chance; if the model is calibrated, each bucket's observed rate
matches its mean prediction. Deck hint and acquired, observed (mean prediction in parentheses):

| Predicted | Deck hint: rows, observed | Acquired: rows, observed |
|---|---|---|
| 0 | 14,258, 0.001 | 8,822, 0.011 |
| above 0 to 0.20 | 110, 0.227 (0.17) | 1,943, 0.162 (0.15) |
| 0.20-0.40 | 3,664, 0.308 (0.31) | 3,817, 0.319 (0.30) |
| 0.40-0.60 | 1,833, 0.453 (0.47) | 4,638, 0.500 (0.50) |
| 0.60-0.80 | 1,308, 0.568 (0.73) | 7,006, 0.690 (0.71) |
| 0.80-0.95 | 672, 0.725 (0.87) | 4,958, 0.832 (0.86) |
| 0.95-1 | 2,681, 0.954 (1.00) | 6,103, 0.974 (1.00) |

The deck's over-prediction sits in its top buckets, the event rewards: the buckets predicted 0.6 and above hold
95% of the shortfall, and 97% of their rows have an event source. The 0.20-0.40 bucket, almost all training-pool
rows, is within its interval (0.308 [0.294, 0.323] against 0.311).

*Per run.* A run has 7.17 open targets on average.

- Targets acquired per run: expected 3.673, observed 3.627; on the 4,123 runs the priority assumption fits
  (every open target prioritised, or no list shown), expected 3.485, observed 3.475.
- All open targets acquired: product of the model's odds 0.0448, observed 0.0423 [0.0372, 0.0481]; on
  the runs the priority assumption fits, 0.0249 against 0.0250 [0.0206, 0.0302].
- Landing $P(\text{all eight})$, recomputed from what each run acquired, against the model's exact $P(\text{all})$:
  0.927 [0.80, 1.05] (bootstrap over runs, resampling whole runs); Unity Cup 0.926 [0.80, 1.07],
  Grand Concert 1.06 [0.67, 1.51], URA Finale 0.83 [0.35, 1.41]; Trackblazer's model value is 0.

Consistent with calibrated, though the interval allows up to about 1.2× optimism (1/0.80). Only
220 runs acquired every open target, too few to separate the exact joint from the product.

*Single-source cards.* For a single-source pair the deck test scores that card alone. The pairs furthest from the model
(every pair of 40 or more rows whose interval excludes 1, and two whose intervals reach 1, Gold Ship's and Sweep
Tosho's):

| Card > target | Pred | Obs | Ratio | Rows | Likely reason |
|---|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | 1.000 | 0.760 | 0.76 [0.72, 0.80] | 437 | the model assumes Risky Business prioritised (finale choice 2) |
| Riko Kashimoto [Planned Perfection] > Ramp Up | 0.750 | 0.490 | 0.65 [0.60, 0.70] | 708 | open (her Lone Wolf choice, when prioritised, accounts for part) |
| King Halo [Tonight, We Waltz] > Uma Stan | 0.947 | 0.752 | 0.79 [0.71, 0.87] | 125 | Uma Stan sits on a choice that needs a priority |
| Gold Ship [That Time I Became the Strongest] > Uma Stan | 0.196 | 0.143 | 0.73 [0.43, 1.19] | 84 | open |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | 0.305 | 0.195 | 0.64 [0.40, 0.97] | 77 | open |
| Maruzensky [Sentimental Flare ♪] > Slipstream | 0.241 | 0.291 | 1.21 [1.01, 1.42] | 330 | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | 0.338 | 0.479 | 1.42 [1.13, 1.71] | 96 | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | 0.724 | 0.641 | 0.88 [0.81, 0.95] | 345 | open |
| Sweep Tosho [Lamplit Training of a Witch-to-Be] > Slipstream | 0.332 | 0.293 | 0.88 [0.75, 1.022] | 396 | open |
| Nakayama Festa [Trusting the Dice] > Risky Business | 0.610 | 0.522 | 0.85 [0.78, 0.93] | 462 | open |
| Tokai Teio [Step! Smile! Wink!] > Nimble Navigator | 0.418 | 0.541 | 1.29 [1.10, 1.48] | 148 | open |

King Halo's Uma Stan follows the priority: where the run lists Uma Stan as prioritised, her card hinted it in
35 of 38 rows against 37.1 expected (exact Poisson-binomial $P(X \le 35)$ = 0.062,
consistent); on a list without it, 0 of 12 against 11.7.

Below 40 rows, 5 more pairs exclude 1. 3 of them are event sources of Playtime's Over! (Yukino Bijin [Dancing Light
into the Night], Yaeno Muteki [Fiery Discipline], Sirius Symboli [Escorte Étoile]) that hinted it in 1 of 49 runs
against 0.51-0.84 predicted. Their event puts the skill on choice 2, which ends the chain before its last link, so
IT takes it only when the skill is prioritised. Only 5 of those runs show a priority list (the skill prioritised in
2, hinted in 0 of the 5), and the other 44 hinted it 1 time, so the priority cause is
untested here.

*Other skills.* Every non-target skill a deck card or spark could give, on a 30% sample of the runs
(1,559): inheritance reads 0.986 [0.973, 0.999] over
66,598 rows, slightly short of the model, a known miss spread over many skills (the largest one skill's share of
the shortfall is 9%); training-pool hints read 1.065 [1.053, 1.076] over 26,573, under-predicted.

### Untested here

the step from acquired to landed (spark generation, and that everything hinted is bought); the
priority policy run by run; correlations inside a run beyond $P(\text{all})$; Grand Masters; hint levels. The runs are
completed careers; every comparison is per run on that run's own deck and lineage. Rows with an inspiration are left out
of the deck test, which is then conditional on no spark, treated as independent of the deck.
