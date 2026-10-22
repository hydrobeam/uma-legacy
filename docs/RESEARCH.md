# The model's mathematics

What this project measured, fitted, tested or derived for its model of how a career in Uma Musume's Independent
Training acquires and passes on skills. Every constant the model uses, with its source, is in `docs/MODEL.md`; this
document keeps the evidence.

## Results

The model's odds rest on one prediction: the chance that a career acquires each target. Section 13 tests it on
5,198 real Independent Training careers, each rebuilt from its own deck and lineage. Aggregate acquisition is consistent with
the product approximation on rows whose target is prioritised or whose priority list is unknown. Unknown lists do not
establish the priority assumption. Over all rows the approximation is slightly optimistic.

| Rows | Rows (n) | Observed / predicted [95%] |
|---|---|---|
| target prioritised, or no priority list shown | 32,848 | 0.998 [0.990, 1.005] |
| all | 37,287 | 0.986 [0.980, 0.993] |
| target shown not prioritised | 4,439 | 0.911 [0.893, 0.929] |
| all eight default targets landed, per run | | 0.93 [0.80, 1.05] |

The not-prioritised rows carry 85% of the shortfall; this allocation does not establish its cause.
The last row is a spark-weighted acquisition check against the exact joint model, with only
220 runs acquiring every target. It tests neither spark generation nor the ranking of candidates.
The terms are defined in *How to read*.

### What each part rests on

The schedule ceiling below fixes each loop's reported decks. Scenario bands are chosen defaults informed by measured
rank distributions. The tests and their nominal labels have the limits set out in *How to read*.

| Part | How it is set | Main evidence | Section |
|---|---|---|---|
| Star band per scenario | derived | Grand Concert: 79% of careers reach 17,500+ | 2 |
| Grandparents' inheritance rate | fitted | weight 1.00 [0.98, 1.02] against the parents' 1 | 3 |
| Affinity's effect on inheritance | assumed, tested | a free slope predicts no better than 1: $z$ = 0.86 | 3 |
| +3 affinity per shared G1 win | assumed, tested | a free weight predicts no better than 1: $z$ = 0.81 | 3 |
| Own grandparent adds no trio points | fitted | 29 nats better than the alternative | 5 |
| Race win table | community table, tested; goal shift fitted | 0.998 [0.996, 0.999] over 119,188 G1 attempts | 4 |
| Loop schedules for shared wins | derived | no other schedule could raise the best loop's odds by more than 2.01% | 6 |
| Training hints, $\beta(1 + hf/100)(1 + d\,\mathit{sp}/100)$ | form fixed on the runs, $\beta$ and $d$ fitted | real careers: 0.994 [0.96, 1.03] of the model | 7 |
| Chain events, Binomial(21, 0.35) per career | fitted, kept by choice | no alternative predicts unseen runs better | 8 |
| Chain events spread over remaining links | fitted | 102 nats better than equal shares per card | 8 |
| IT takes a prioritised skill's choice | assumed, tested on careers | prioritised targets' event hints: 0.970 of the model | 9, 13 |
| Measured outcome shares | fitted | five events, e.g. Nakayama Festa's Nothing Ventured 0.61 | 9 |
| Random card events | not modelled | left out, those targets read 1.24 of the model | 10 |
| Conditional character events | not modelled | no significant cost, except Vodka's own Nimble Navigator | 11 |

### Known misses

Each is measured and tested in its section. These are exploratory discrepancies under the tests below; pointwise
intervals and nominal significance do not establish every listed miss after selection across many comparisons.

| Part | Miss | Observed / predicted | Section |
|---|---|---|---|
| Inheritance | Tail Held High is inherited less often than predicted; cause unknown | 0.953 [0.918, 0.988] | 3, 13 |
| Inheritance | parents at affinity 200 or more pass sparks less often | 0.966 [0.945, 0.987] | 3 |
| Inheritance | non-target skills, the shortfall spread over many skills | 0.986 [0.973, 0.999] | 13 |
| Races | chances of 70 or less | 0.95 [0.93, 0.97] | 4 |
| Races | chosen races, 1st in a row | 0.996 [0.994, 0.998] | 4 |
| Races | chosen races, 2nd in a row | 0.997 [0.994, 0.999] | 4 |
| Races | goal races 7th or later in a row, which the planner charges as the 6th | 0.68 [0.51, 0.84] | 4 |
| Training hints | cards differ beyond their effects; cause open | per card 0.80-1.17 | 7 |
| Training hints | a skill also hinted by inspiration | 0.73 | 7 |
| Training hints | more pals per deck give slightly more hints | factor 1.028 per pal | 7 |
| Training hints | Trackblazer's base over-predicts real careers | 0.779 [0.63, 0.93] | 13 |
| Training hints | non-target skills are under-predicted | 1.056 [1.045, 1.068] | 13 |
| Chain events | Binomial(21, 0.35) is rejected in-sample | p-value 6.5e-05 | 8 |
| Chain events | decks with Sasami Anshinzawa [This Might Sting!]'s pal card get more | mean 7.74 against 7.17 | 8 |
| Chain events | Inari One's careers get fewer | mean 6.83 against 7.26 | 8 |
| Chain events | with six chain cards, more chains never start | 0.34 against 0.20 | 8 |
| Chain events | individual cards' final links differ beyond chance | $\chi^2$ = 386 on 50 cards | 8 |
| Event choices | prioritised targets' event hints read just below the model | 0.970 [0.940, 0.9997] | 9, 13 |
| Left out | targets a deck card's random event can hint | 1.24 [1.11, 1.38] | 10 |
| Left out | Vodka's own Nimble Navigator | her events give it in 25 of 27 runs | 11 |

### Not settled

Specialty priority goes with more hints, but it is not shown to cause them. Fitted on earlier runs, it predicts later
ones better by 10.1 nats. On the cards whose limit breaks change only their specialty priority, the data fit
no effect as well as the fitted one (section 7).

The hint-frequency slope $1/100$ is assumed. On the adopted model a free divisor does not predict unseen runs better;
in the form study, without the specialty term, one does (section 7).

### Assumed and untested

The step from acquired to landed (spark generation, section 1, and that everything hinted is bought), the star mixes
(section 2), Grand Masters (no runs), hint levels, the priority policy run by run, and correlations inside a run beyond
$P(\text{all})$.

## How to read this document

**Labels.** **fitted**: estimated from the runs, with the sample and method given. **derived**: follows from the other
parts by computation. **assumed**: a chosen rule or an outside figure the runs cannot test; its source is named and no
evidence is claimed for it.

**Intervals.** $x$ [$a$, $b$] is an estimate with its 95% interval, or 90% where marked. Where a run gives one
observation, it is the Wilson interval. Where a run gives several rows, such as targets, race attempts or trials, the
interval is clustered by run, so a run is the unit.
Calibration ratio intervals treat predictions as fixed and runs as independent; they do not include uncertainty from fitting the
predictions or dependence between careers. Wilson divided by a mean prediction is a marginal rate interval, not a
conditional Poisson-binomial calibration test for heterogeneous predictions. Band tests are run-clustered Wald tests,
whose reference distribution requires enough independent runs and full-rank covariance. Unless stated otherwise,
intervals are pointwise and comparisons exploratory; covering the prediction does not establish equivalence.

**Terms.**

- *Run*: one Independent Training (IT) career from uma-it, pooled. *Row*: one observation within a run; a
  *run-target row* is one target in one career.
- *Observed / predicted*: the successes seen over a set of rows, divided by the successes the model expects. Above 1
  the model is pessimistic; below 1 it is optimistic.
- *Acquired*: hinted, so it could be bought. A target *lands* when it becomes a spark on the new veteran.
- A career's six *legacies*, two parents and four grandparents, carry *sparks* of 1-3 stars: *white* for skills,
  *pink* for aptitudes. At the Classic and Senior April *inspirations* a legacy's spark can fire, hinting its skill or
  raising an aptitude. The legacy's *affinity* with the trainee raises that chance.
- The *deck* is six support cards, each at a limit break from LB0 to LB4. *Pal* cards have their own events. A *chain
  card*'s events are a sequence of 2-3 *links*. A *marker skill* is one only one event can give, so its hint shows
  the event fired.
- The *priority list*: up to 10 skills the player marks at the start of an IT career for IT to favour.
- *Targets*: the Legacy Loop Planner's eight default white / gold pairs: Uma Stan / Superstan, Nimble Navigator / No
  Stopping Me!, Playtime's Over! / See Ya Later!, Ramp Up / It's On!, Tail Held High / Tail Nine, Racing Spirit:
  Stamina, Slipstream / Perfect Spot!, Risky Business / Nothing Ventured. The model is the same for every skill, so
  section 13 also checks a sample of every other skill.

**Fit comparisons.** A model's *log-likelihood* (LL) sums the log of the probability it gave to what was seen. For a
joint likelihood, a difference of $\Delta$ *nats* is a likelihood ratio of $e^{\Delta}$. Sums of dependent marginal
scores, including training-skill rows and the chain-allocation link scores, are composite likelihoods. Their differences
compare scores without giving that likelihood ratio for whole careers. A *free* fit gives each quantity its own value
with no formula, such as a win rate per cell. It is the yardstick a
formula is measured against. There are three kinds of comparison, each with its rule:

1. **Same number of fitted parameters**: compared descriptively by LL on the same data; no universal significance
   cutoff follows for nonnested models.
2. **Is an extra parameter detectable?** For regular nested joint-likelihood fits, Wilks' reference distribution
   compares $2\Delta$ with $\chi^2$ on the difference in parameter dimensions. The scripts' unadjusted tests on
   composite likelihoods, discrete count parameters or boundary fits need separate calibration. Their nominal
   significance alone does not establish detection.
3. **Does a model predict better?** The scripts compare folds grouped by run id. Each fold should refit the complete
   prediction pipeline on its training runs. The hint covariate and card-effect checks retain full-pool specialty
   weights, so those comparisons are only partly held out. Their per-scenario levels are refitted. The reported
   standard error treats per-run score differences as independent and ignores overlapping training folds and model
   selection. Its $z$ is an approximate comparison, and failure to cross the script's threshold does not show that
   alternatives are equivalent or worse.

Every number here is computed by the scripts in `research/`, which also test each conclusion the text draws
(`research/README.md`). A script's **holds** records its predicate, subject to these limitations. Only the assumed
inputs are typed in.

## 0. What is computed

The model gives one number per career, $P(\text{all})$: the chance that every target lands on the veteran it produces.
For a planned loop the legacies' stars are averaged (section 2) and race wins come from base aptitudes (section 4).
With real legacies, their own stars, wins and parents are used (section 12).

## 1. Spark generation (assumed)

**Assumed** (community figure; `docs/MODEL.md`): a bought skill becomes a spark with chance $g_0 \cdot 1.1^{n}$, where
$g_0$ is 0.2 for a white and 0.4 for a gold and $n$ counts the ancestors carrying it; $n = 6$ in a running loop. The
runs show the sparks a career inherited, not the ones it produced, so nothing here is tested.

## 2. Star mix of legacy sparks

Computed by `research/s02_stars.py`.

The star mixes are **assumed** (community figures; `docs/MODEL.md`): 50 / 45 / 5% of sparks at 1 / 2 / 3★ below a rank
score of 17,500, and 20 / 70 / 10% from 17,500 up. The page chooses the majority band in each measured scenario.
That default is a rule informed by the rank scores, not a fitted mixture for veterans retained in a loop.

### Findings

- **Grand Concert uses the higher band; URA Finale, Unity Cup and Trackblazer the lower.** Unity Cup is close to even.
- **Averaging over stars overstates the single-target inheritance chance in the sampled configurations**, by at most
  0.57% within the assumed mixes. This is no bound on every lineage or on joint landing odds.

### Evidence

*Which band?* (**chosen** from measured rank distributions.) Each measured scenario takes the majority band. A career's rank score
is estimated with every skill point spent, over every completed run in the run list.

| Scenario | Runs | Share at 17,500+ [95%] | Band |
|---|---|---|---|
| URA Finale | 1,357 | 0.000 [0.000, 0.003] | below |
| Unity Cup | 3,155 | 0.467 [0.450, 0.484] | below |
| Grand Concert | 14,742 | 0.794 [0.787, 0.800] | 17,500+ |
| Trackblazer | 282 | 0.000 [0.000, 0.013] | below |

Grand Masters has no runs and is **assumed** 17,500+.

*What does averaging over stars cost?* (**derived**.) For a planned loop the model uses each legacy's expected base per
roll. That is exact for one roll only when the cap is inactive. Across both rolls the same stars persist. With $r$ a
legacy's chance per roll, convexity gives $E[(1 - r)^2] \ge (1 - E[r])^2$. So averaging overstates $q$, the chance of
an inspiration over the career (section 3).
The table measures the overstatement on the six real affinities of each of the 5,527 lineages in the runs, for a target
carried by all six. The calculation assumes independent unknown stars across slots; repeated appearances of the same
veteran would share her stars. With capping, the squared miss remains convex in the base, so the same direction holds
under this slot-independence assumption.

| Band | Mean overstatement of $q$ | Largest |
|---|---|---|
| below 17,500 | 0.51% | 0.57% |
| 17,500+ | 0.30% | 0.33% |

With real legacies their actual stars are used, so this does not arise.

## 3. Inheritance rolls

Computed by `research/s03_inheritance.py` (the roll's parts) and `research/s13_calibration.py` (the test on whole
careers).

The roll tested is the community formula (`docs/MODEL.md`). At each of the two inspirations, a legacy carrying the skill
passes it with chance $r = \min\{1,\ b(s)(1 + A/100)\}$: a base $b(s)$ set by the spark's stars $s$, 3 / 6 / 9% for a
skill white, raised in proportion to the legacy's affinity $A$. Over the career the chance of at least one inspiration
of the skill is $q = 1 - [\prod_l (1 - r_l)]^2$ over the six legacies $l$. The odds use skill whites only.
This assumes independent firings across legacies and inspirations conditional on the stars and affinities.

The parts are **fitted** on *trials*. A trial is one ancestor's white spark in one year where she is the only one of the
six legacies carrying the skill, so that year's inspiration of it can only come from her. The trial *passes* when the
spark fires. There are 44,010 parent trials and 106,738 grandparent trials of ordinary ancestors; the trainee's own
character as a grandparent is section 5's.

### Findings

- **Grandparents pass sparks at the full base rate, as parents do.**
- **The affinity formula is retained.** A free slope, or a free weight on either part of affinity, has no clear
  held-out advantage under the reported comparison.
- **The skill-white bases lie inside the pointwise intervals by stars.**
- **Known miss: parents at affinity 200 or more pass sparks less often than predicted.** A refitted slope does not
  predict unseen runs better, so the model keeps the formula and this miss.
- **One band of predicted chance lies outside its pointwise interval** on whole careers, though the joint test does
  not reject the prediction. Whole-career totals and Tail Held High are in section 13.

### Evidence

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Grandparents take the full base | weight 1.00 [0.98, 1.02] | full base beats half by 1,975 nats | full base |
| Affinity raises the roll at all | | beats no affinity by 5,427 nats | kept |
| Affinity slope is 1 | 0.97 [0.95, 0.99] | held out 2.1 ± 2.4 nats, $z$ = 0.86 | 1 kept |
| Relation-point weight is 1 | 0.92 [0.84, 0.99] | held out 1.8 ± 2.2 nats, $z$ = 0.85 | 1 kept |
| Weight of 3 per shared G1 win is 1 | 0.96 [0.92, 0.99] | held out 2.0 ± 2.5 nats, $z$ = 0.81 | 1 kept |

The free slope gains 2.9 nats in-sample, which held out does not keep. Each part's weight is fitted with the
other fixed; 150,032 trials have shared wins.

*Grandparents.* A halved base falls far short of the passes seen:

| Grandparent passes | Count |
|---|---|
| observed | 9,607 |
| predicted with the full base | 9,630 |
| predicted with half | 4,815 |

Parents read 0.975 of the model, the shortfall coming from high affinity (below).

*The bases by stars* all cover 1:

| Stars | Trials | Observed / predicted [95%] |
|---|---|---|
| 1★ | 41,970 | 0.985 [0.95, 1.02] |
| 2★ | 96,080 | 0.987 [0.97, 1.00] |
| 3★ | 12,698 | 0.995 [0.96, 1.03] |

*Every affinity?* Below 200, yes. The band outside its own 95% interval is 200+.

| Affinity | Trials | Observed / predicted [95%] |
|---|---|---|
| 0-49 | 12,628 | 0.980 [0.919, 1.042] |
| 50-99 | 94,696 | 0.999 [0.980, 1.019] |
| 100-149 | 1,428 | 1.025 [0.899, 1.152] |
| 150-199 | 5,316 | 1.045 [0.982, 1.107] |
| 200+ | 36,680 | 0.966 [0.945, 0.987] |
| parents, below 200 | 7,330 | 1.040 [0.985, 1.094] |
| parents, 200+ | 36,680 | 0.966 [0.945, 0.987] |

| Joint test of the bands | $\chi^2$ | $p$ | Verdict |
|---|---|---|---|
| the four below 200 | 2.5 | 0.65 | consistent |
| all five | 12.6 | 0.027 | not consistent: a known miss |

*Whole careers.* Section 13 gives the totals and the Tail Held High discrepancy. Without it, the finer checks are consistent
over 36,386 rows:

| Check, without Tail Held High | Observed / expected | Verdict |
|---|---|---|
| by the number of carriers, 1 to 6 | 0.965-1.000 | every interval covers 1 |
| years in which an inspiration of the skill fired | 0.989 | 1.84 sd below, inside the 95% band |
| legacy firings, against $2\sum_l r_l$ | 0.990 | |

*Every predicted chance?* The rows are grouped by predicted $q$ into 5 groups, whose joint test does not reject the prediction.
Their joint test gives $\chi^2$ = 9.2, $p$ = 0.10. One group, 0.40-0.60, lies outside its own 95% interval.

| Predicted $q$ | Rows | Observed | Mean predicted |
|---|---|---|---|
| 0 | 13,734 | 0.001 | 0 |
| above 0 to 0.20 | 3,884 | 0.154 | 0.16 |
| 0.20-0.40 | 4,688 | 0.294 | 0.30 |
| 0.40-0.60 | 6,232 | 0.494 [0.481, 0.506] | 0.506 |
| 0.60-0.80 | 9,066 | 0.697 | 0.70 |
| 0.80-0.95 | 3,980 | 0.838 | 0.84 |

## 4. Race wins

Computed by `research/s04_races.py`.

The table tested is the community one (`docs/MODEL.md`). A trainee's chance of winning a race she enters is
$p = \mathrm{clamp}(110 - c_{\text{dist}} - c_{\text{surf}} - \mathrm{PEN}_s,\ 0,\ 100)/100$. In words: 110%, less a
cost for each of her aptitude grades for the race's distance and surface, less a penalty for racing in a row. The
costs are 0 for S and A, rising to 90 for G. The penalty $\mathrm{PEN}_s$ is 0, 0, 5, 20, 30, 50 at streak position
$s$ = 1 to 6 or more.

A race's position $j$ counts consecutive race turns; any race counts, and a turn without one resets it. A *goal race*
is one the career requires; a *chosen race* is one IT chose. The community table charges every race its own position,
$s = j$. The adopted model (**fitted**, below) charges a goal race the position before it, $s = \max(j - 1, 1)$, so goal
races pay 0, 0, 0, 5, 20, 30. The planner counts $j$ up to 6.

### Findings

- **A goal race takes the penalty of the race before it.** It still counts toward the position of the races after
  it. Why is not known; the runs only show that it does.
- **Grades must include inherited aptitude sparks.** On base grades the table predicts too few wins. Pink sparks raise
  grades at the start of a career and at each inspiration, from that April on.
- **Races in a row lower the chance.** The previous race's result shows no detectable additional effect in these contrasts.
- **The table is retained.** Free cell rates have no clear held-out advantage.
- **Known misses**: chances of 70 or less, and chosen races at the 1st and 2nd race in a row, win slightly less than
  predicted. Goal races 7th or later in a row, which the planner charges as the 6th, win less than it predicts.

IT's chosen races won at these rates. The community table's chance is in parentheses. Grades include those raised by
inherited aptitude sparks, and S is shown as A, since both cost 0. A dot marks a cell with under 30 attempts; only
grade pairs with 200 or more attempts are shown.

| Races in a row | AA | AB | AC | BB | AD | BC | AE | BD | CC | BE | CD | AF | AG |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 race | 100% (100%) | 100% (100%) | 90% (90%) | 91% (90%) | 80% (80%) | 76% (80%) | 68% (70%) | 70% (70%) | 63% (70%) | 50% (60%) | 59% (60%) | 46% (50%) | 19% (20%) |
| 2 races | 100% (100%) | 100% (100%) | 91% (90%) | 90% (90%) | 79% (80%) | 80% (80%) | 61% (70%) | 68% (70%) | 78% (70%) | 50% (60%) | 65% (60%) | 52% (50%) | 19% (20%) |
| 3 races | 100% (100%) | 95% (95%) | 85% (85%) | · (85%) | 75% (75%) | · (75%) | 59% (65%) | 60% (65%) | · (65%) | 43% (55%) | · (55%) | 56% (45%) | 11% (15%) |
| 4 races | 90% (90%) | 81% (80%) | 74% (70%) | · (70%) | 49% (60%) | 62% (60%) | 38% (50%) | 61% (50%) | · (50%) | · (40%) | 30% (40%) | · (30%) | · (0%) |
| 5 races | 81% (80%) | 66% (70%) | 54% (60%) | · (60%) | · (50%) | · (50%) | 35% (40%) | · (40%) | · (40%) | · (30%) | · (30%) | · (20%) | · (0%) |
| 6 or more | 56% (60%) | 58% (50%) | · (40%) | · (40%) | · (30%) | · (30%) | · (20%) | · (20%) | · (20%) | · (10%) | · (10%) | · (0%) | · (0%) |

### Evidence

The runs record every race a career ran: 119,188 attempts at the 34 single-race G1s. Of them, 34,089 were goal
races and 85,099 chosen. 93.7% were won.

The table calls some races certain, 0% or 100%, and a handful went the other way. That would give the table a
log-likelihood of $-\infty$. So every model compared has its 0% and 100% pulled in to a fitted floor and ceiling, the
same two parameters in each, and the comparisons follow rule 1 of *How to read*.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Raised grades, not base | raised: 0.998 [0.996, 0.999] | 2,921 nats better than base | raised grades |
| The streak penalty is real | | without it, 2,753 nats worse | kept |
| Goal races pay one position less | community table, goal races 4th+: 1.141 [1.125, 1.158] | held out, forms table below | shift adopted |
| The previous result does not matter | every loss / win contrast covers 1 | $z$ = 1.04 from the 4th race on | no effect |
| A pink counts from the April of its inspiration | won 0.737 [0.696, 0.778] | predicted 0.754 from April, 0.864 from the year's start | from April |
| One formula is enough | per-cell rates 93 nats better in-sample | held out -11 ± 15 nats, $z$ = -0.73 | table kept |

*Base or raised grades.* Base aptitudes are what a planned loop has. Raised grades follow the aptitude-spark rule in
`docs/MODEL.md`.

| Grades | Predicted win rate | Observed / predicted [95%] | Where the table gives 70 or less |
|---|---|---|---|
| base | 88.2% | 1.063 [1.060, 1.066] | 1.78 over 19,012 attempts |
| raised | 94.0% | 0.998 [0.996, 0.999] | 0.95 |

Outside chosen races at the 1st and 2nd race in a row, every scenario is consistent with the prediction. Over all attempts, the scenarios
outside their intervals are: Grand Concert and Unity Cup.

*Goal races.* Under the community table, goal races from the 4th race in a row win more than predicted, while chosen
races there match. Three forms were scored on runs left out of the fit, five folds by run over 5,470 runs, each with
the same fitted floor and ceiling:

| Form | Goal race at position $j$ pays | Races after it count it | Held out, against the adopted form |
|---|---|---|---|
| Adopted | $\mathrm{PEN}_{\max(j-1,1)}$ | yes | |
| Community table | $\mathrm{PEN}_j$ | yes | 158 ± 34 nats worse, $z$ = 4.72 |
| Exclusion | $\mathrm{PEN}$ at the chosen races before it | no | 1,151 ± 62 nats worse, $z$ = 18.58 |

In-sample the adopted form fits 135 nats better than the community table. With the adopted shift, the grouped goal-race intervals cover the prediction, and chosen races from the 3rd on. Chosen races at the 1st and 2nd fall slightly short, a known miss.

| Race in a row | Goal races: observed / predicted [95%] | Attempts | Chosen races: observed / predicted [95%] | Attempts |
|---|---|---|---|---|
| 1st | 1.001 [1.000, 1.001] | 18,815 | 0.996 [0.994, 0.998] | 41,537 |
| 2nd | 1.000 [0.999, 1.001] | 9,378 | 0.997 [0.994, 0.999] | 30,712 |
| 3rd | 1.001 [0.999, 1.003] | 3,912 | 0.999 [0.995, 1.003] | 8,065 |
| 4th | 0.996 [0.991, 1.001] | 809 | 0.994 [0.981, 1.008] | 3,525 |
| 5th | 0.999 [0.972, 1.026] | 878 | 0.998 [0.959, 1.037] | 988 |
| 6th or later | 0.972 [0.908, 1.036] | 297 | 0.972 [0.865, 1.079] | 272 |
| goal, 7th or later | 0.68 [0.51, 0.84] | 47 | | |

*Every chance?* On raised grades, the pointwise intervals cover the prediction down to a chance of 80:

| Table's chance | Attempts | Observed / predicted [95%] |
|---|---|---|
| 110 | 79,866 | 1.000 [0.999, 1.000] |
| 100 | 18,673 | 1.000 [0.998, 1.002] |
| 90 | 6,598 | 1.004 [0.996, 1.013] |
| 80 | 4,477 | 0.992 [0.977, 1.007] |
| 70 or less | 9,574 | 0.949 [0.931, 0.968] |

*A pink's timing.* The records say which inspiration fired a pink, and inspirations sit at fixed turns. The timing
matters on 502 Senior January-March attempts; the claims table compares them.

*The previous race's result.* The test divides observed / predicted after a loss by the same after a win, with a
run-clustered interval. Every contrast covers 1; this supports retaining the race-count penalty without establishing absence of a win-history effect.

| Race in a row | After a win [95%] | Attempts | After a loss [95%] | Attempts | Loss / win [95%] |
|---|---|---|---|---|---|
| 2nd | 0.998 [0.996, 1.000] | 37,863 | 0.991 [0.979, 1.003] | 2,227 | 0.993 [0.981, 1.005] |
| 3rd | 1.000 [0.997, 1.003] | 11,296 | 0.991 [0.972, 1.010] | 681 | 0.990 [0.971, 1.010] |
| 4th or later | 0.992 [0.982, 1.003] | 6,099 | 1.013 [0.975, 1.051] | 670 | 1.021 [0.982, 1.061] |

*One formula.* Giving each of the 101 (chance, streak position, goal or chosen) cells its own win rate fits better
in-sample. The gain exceeds the 62 nats of the nominal independent-attempt threshold for 99 extra parameters if attempts were independent. The shared career state can make those attempts dependent. Held out, the per-cell rates predict no better, so the table stands.

The test is conditional on IT having chosen each non-goal race. Stats are not modelled.

## 5. Affinity

Computed by `research/s05_affinity.py` (the self-grandparent fit).

A legacy's affinity is the game's rule as community tools implement it (`docs/MODEL.md`). It adds the characters'
relation points from the game data and 3 for each single-race G1 both umas won. Trainee $T$ has parents $P_1$ and
$P_2$. For either parent $P$, $P'$ is the other parent and $G \in \mathrm{par}(P)$ are $P$'s own parents, the
grandparents:

$$A_P = \mathrm{pair}(T,P) + \mathrm{pair}(P_1,P_2) + 3\,o(P,P') + \sum_{G \in \mathrm{par}(P)} \big[\mathrm{tri}(T,P,G) + 3\,o(P,G)\big], \qquad A_G = \mathrm{tri}(T,P,G) + 3\,o(P,G).$$

Here pair and tri are the relation points of the groups holding all the characters named. $o(a,b) = \sum_r P_a(r)\,
P_b(r)$ is the expected number of single-race G1s both $a$ and $b$ win, where $P_a(r)$ is $a$'s chance of winning race
$r$ at least once over her schedule (section 4, base aptitudes). Schedules fixed in advance and careers independent are
**assumed**. The 3 per shared win is tested in section 3.
The loop model inserts expected shared wins into the nonlinear inheritance formula; it does not average inheritance
over joint future race outcomes.

### Findings

- **A trainee who is her own grandparent gets no trio points from it**: $\mathrm{tri}(T,P,T) = 0$ (**fitted**). The game
  data do not say what a group with one character twice counts. Counting it as $\mathrm{pair}(T,P)$, as one community
  tool does, is about 29 nats worse.

### Evidence

The fit uses 174,056 inspiration trials attributable to a single ancestor. In 1045 of them a self-grandparent passed her
spark. The distance of that count from each prediction is in run-clustered standard deviations:

| $\mathrm{tri}(T,P,T)$ | Passes predicted | Distance from observed, at slope 1 | At the slope refitted on ordinary ancestors |
|---|---|---|---|
| 0 (adopted) | 1074.7 | −0.96 | −0.61 |
| $\mathrm{pair}(T,P)$ | 1241.8 | −6.26 | −5.78 |

So 0 is kept and the pair rejected. A free multiplier $\kappa$, with $\mathrm{tri}(T,P,T) = \kappa\cdot\mathrm{pair}(T,P)$,
fits best at $\hat\kappa = 0$, its lower bound.

The gap is not the ordinary trials' own misfit. The last column refits the affinity slope on the ordinary ancestors, to
0.97, and the verdict stands. At slope 1, ordinary parents read 0.975 of prediction and grandparents 0.998.

## 6. Loop structures, ranking and schedules

Computed by `research/s06_loops.py`.

A four-uma loop has 12 lineages $(T, P_1, P_2, X)$: a trainee, her parents, and $X$, each parent bred from
the other parent and $X$. A three-uma rotation has 3, with $X = T$, so the trainee is her own grandparent. A
structure is ranked by its lineages' mean $P(\text{all})$, a definition (`docs/MODEL.md`).

### Findings

- **The rotation occurs in practice**: 1,911 of 5,534 runs have a grandparent of the trainee's own character.
- **Every pair of umas carries the same weight in the sum of affinities.** Total expected shared wins is the chosen
  scheduling objective; equal affinity weights do not make it equivalent to the nonlinear landing score.
- **With each loop's reported decks fixed, scheduling for shared wins costs little on the references**: no schedule
  could raise a reference loop's odds by more than 2.01%, and no listed runner-up could overtake its winner.

### Evidence

*Race weights* (**derived** from section 5's $A_P$ and $A_G$). In one lineage, $o(P_1,P_2)$ enters the six affinities
6 times. It counts twice in each $A_P$, since $P'$ is also one of $P$'s own parents. $o(P_1,X)$ and $o(P_2,X)$ enter
twice each. Summed over the lineages, every pair gets the same weight:

$$\text{loop: } 6\cdot 2 + 2\cdot 4 = 20, \qquad \text{rotation: } 6 + 2 + 2 = 10.$$

The scheduler seeks a large $\sum_{a<b} o(a,b)$, the total expected shared wins, as a symmetric surrogate.

*The schedule bound* (**derived**, on the reference loops: the best loops and decks found for a fixed set of reference
setups). Schedules are chosen for shared wins, not for the score. The score is nondecreasing in every $o$: $o$ enters each
legacy's affinity with weight +3, and every later step is nondecreasing in its inputs. So, with a loop's decks fixed, no
schedule can score above the odds with every pair at a valid upper bound on its overlap. The best schedule value lies
between the feasible score found and this ceiling. Outfits are reselected at the ceiling, while decks remain fixed.
The bound is computed on 40 reference results, whose odds an independent implementation
reproduces to within 2e-15.

Two ceilings bound the best possible overlap. $o_{\max}(a,b) = \sum_r P^{\max}_a(r)\,P^{\max}_b(r)$ is an independent
one: every offering of every G1 attempted, with no streak penalty. Each *pair ceiling* is the smaller of the bounds
obtained by optimising either member's feasible schedule against the other's componentwise best-case win chances.
The pair need not attain that ceiling jointly. These bounds come from the same search code that chose the schedules, so
they are not an independent check; they sit between $o$ and $o_{\max}$ on every pair.

| Quantity | Value | Runner-ups that reach their winner |
|---|---|---|
| shared wins found, worth over no race affinity | 31-85% | |
| gain allowed by $o_{\max}$ | up to 17.5% | 8 of 12 |
| gain allowed by the pair ceilings | 0.43-2.01%, median 0.84% | 0 of 12 |

$o_{\max}$ is far from reachable: the schedules found reach about 69% of it, so it cannot settle the ranking.
The pair ceilings settle this comparison with each loop's decks fixed: no listed runner-up reaches its winner under them.
They do not certify the ranking after jointly changing schedules and decks.

## 7. Training hints

Computed by `research/s07_hints.py`.

A card's training can hint the skills in its *pool*. $hf$ is the card's Hint Frequency in percent and $\mathit{sp}$ its
Specialty Priority, both at maximum limit break with unique effects included.

### Findings

- **The adopted form gives a nearly fixed count of uniform skill draws per card.**
  A skill already at hint level 5 is no longer picked. The fitted table below gives a typical card's hints per
  scenario.
- **Hint frequency scales that number directly**: Hint Frequency +40% gives 40% more hints. The slope is **assumed**;
  on the adopted model a free one does not predict unseen careers better.
- **Specialty priority also raises it**, by a weight that differs between scenarios. The game describes it only as
  making the character join her preferred training more often. The effect predicts careers it was not fitted to, but
  it is not shown to be a cause, and why it differs by scenario is not tested.
- **Known misses**: cards differ in hint rate beyond what their effects predict; which card properties explain it is
  open. Decks with more pals give slightly more hints. A skill also hinted by inspiration is hinted by training less
  than predicted.
- SR cards give slightly fewer hints than SSR cards with the same effects in-sample, but that does not predict unseen
  careers better, so the model ignores rarity.

### Model

A card gives, in scenario $s$, $h$ training hints per career:

$$h = \beta_s \left(1 + \frac{hf}{100}\right)\left(1 + d_s\,\frac{\mathit{sp}}{100}\right).$$

That is a base $\beta_s$ for a card with neither effect, raised in proportion to hint frequency and to specialty
priority. The specialty weight $d_s$ is a fitted association, not a game value. A career gets $\lfloor h \rfloor$ hints,
or one more with chance $f = h - \lfloor h \rfloor$.

The chance that none of $k$ targets in a pool of $n$ skills is hinted is computed jointly, not target by target:

$$M(k, n, h) = (1 - f)\, m(k, n, \lfloor h \rfloor) + f\, m(k, n, \lfloor h \rfloor + 1), \qquad
m(k, n, t) = \left(1 - \frac{k}{n}\right)^{t} \ (t \le c).$$

Each of $t$ hints misses all $k$ targets with chance $1 - k/n$. A card's hint raises a skill by its hint increment
$\iota$, so a skill reaches level 5 and leaves the pool after $c = \lceil 5 / \iota \rceil$ hints:

| Hint increment $\iota$ | +1 | +2 | +3 | +4 | +5 |
|---|---|---|---|---|---|
| Hints to level 5, $c$ | 5 | 3 | 2 | 2 | 1 |

When $t > c$, an exact Markov chain over the other skills' hint counts replaces $m$.

### Fitted values

**Fitted** by maximum likelihood on 87,801 hint rows from 15,120 card-runs. A row is one pool skill of one card in one
run; a card-run is one card in one run's deck. Only maximum-limit-break cards count, R cards are out, and so are rows
where an inspiration hinted the skill. The Runs column counts the runs that add a row. The solver reads these values
from `research/results.json` when it is built (`pkg/loopopt/build.rs`), rounded to three decimals.

| Scenario | $\beta$ | $d$ | Rows | Runs | Typical card's hints per career |
|---|---|---|---|---|---|
| Unity Cup | 2.395 | 0.322 | 17,737 | 1211 | 3.2 |
| Grand Concert | 1.999 | 0.068 | 55,136 | 3,194 | 2.5 |
| URA Finale | 1.380 | 0.359 | 12,551 | 736 | 1.9 |
| Trackblazer | 1.702 | 0.107 | 2,377 | 100 | 2.1 |
| Grand Masters | 1.991 | 0.129 | no runs: **assumed**, one $\beta$ and one $d$ fitted over every scenario's rows | | |

The typical card is the runs' median: hint frequency +20% and specialty priority 35.

### Evidence

*Specialty priority.* Without the specialty term, cards with more specialty priority give more hints than predicted.
With it, the excess is gone:

| Specialty priority | Rows | Observed / predicted, without the term | With it |
|---|---|---|---|
| 0 | 29,193 | 0.965 | 1.003 |
| 1-29 | 691 | 0.966 | 0.986 |
| 30-59 | 40,466 | 1.001 | 0.993 |
| 60+ | 17,451 | 1.059 | 1.009 |

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| The term predicts unseen careers | | held out 38.6 ± 5.0 nats, $z$ = 7.8 | holds |
| Fitted on the earlier half, it predicts the later half | | 10.1 ± 3.6 nats, $z$ = 2.8 | holds |
| A weight per scenario beats one shared weight | | held out 13.2 ± 3.0 nats, $z$ = 4.5 | per scenario |
| Scenario × specialty cells fit | 13 cells of 200+ rows | run-clustered $\chi^2$ = 16.0, $p$ = 0.25 | consistent |
| Not one card's, rarity's or period's | scale $k$ 0.65-1.12 | refitted on subsets | holds |
| Same card across limit breaks | $d$ = 0.29 [−0.01, 0.71], 64 cards | | absorbs hint-frequency misfit |
| Same card, hint frequency fixed | $d$ = −0.06 [−0.44, 0.56], 27 cards | +0.03 nats over no term | cause undecided |

The scale $k$ multiplies the adopted per-scenario weights and is 1 on every row. It is refitted on either half of the
runs, on SR or SSR cards alone, and without any one of the eight most-used cards. The last two rows are within-card
comparisons. The same card at different limit breaks has different specialty priority but is otherwise the same card. On most
such cards hint frequency changes too, so their weight, with a base per card and scenario, also absorbs any misfit of
the hint-frequency term. On the clean cards, whose hint frequency never changes, the estimate is consistent with no
effect and with every per-scenario weight.

*Hint frequency's slope.* The $1/100$ is **assumed**: a card's +40% is read as 40% more hints. A free divisor tests it:

| Model | Free divisor | Held out, against $1/100$ | Verdict |
|---|---|---|---|
| adopted, with the specialty term | 90 | 1.8 ± 2.0 nats, $z$ = 0.9 | $1/100$ kept |
| form study, without it | 83 | 16.7 ± 2.6 nats, $z$ = 6.4 | free divisor better |

On the adopted model the free divisor gains in-sample but has no clear held-out advantage. The nominal in-sample
likelihood-ratio test uses dependent skill marginals without adjustment.

*The form.* The form was fixed on every card at any limit break, R cards included, without the specialty term:
140,615 rows from 5,534 runs. Per-scenario $\beta(1 + hf/100)$ predicts unseen runs better than every alternative:

| Rate form | LL in-sample | Per-scenario $\beta(1 + hf/100)$ better held out by |
|---|---|---|
| $1 + hf/25$, over options | −92,742.3 | 6334.1 ± 81.0 nats, $z$ = 78.2 |
| $1 + hf/25$, over skills | −90,327.3 | 3919.0 ± 60.8 nats, $z$ = 64.5 |
| constant | −87,864.8 | 1457.8 ± 31.6 nats, $z$ = 46.1 |
| one $\beta(1 + hf/100)$ | −86,963.8 | 556.4 ± 20.8 nats, $z$ = 26.7 |
| per-scenario $\beta(1 + hf/100)$ | −86,406.6 | |
| free divisor, $c = 83$ | −86,389.7 | slope table above |

The other checks of the form are descriptive. The stat-option variance check selects uncensored card-runs and uses
a downward-biased sample variance; its sign-test reference probability is unjustified. It does not establish the
absence of a stat option. The pair check uses continuous draw powers at the earlier rate without specialty priority;
it does not test the adopted cap-aware integer mixture.

| Question | Result |
|---|---|
| Hints draw skills only, not a stat option too | in 71 of 91 (card, $hf$) cells the draw-count variance is below the floor a stat option implies |
| Repeats: $P(k \ge 2 \mid k \ge 1)$ | 0.139 observed; 0.139 with the two-point count, 0.191 with a Poisson count |
| Distinct skills per card-run, 1 / 2 / 3 / 4, observed | 0.142 / 0.575 / 0.250 / 0.032 |
| the same, model | 0.120 / 0.566 / 0.273 / 0.039 |
| Joint draws: $P(\text{both})$ for 247,441 pairs of pool skills, Trackblazer out | 0.0792 observed; 0.0794 joint, 0.1094 independent |
| Joint draws: $P(\text{neither})$ | 0.4411 observed; 0.4380 joint |
| Pool size | observed / predicted 0.92-1.11 for every $n$ with 150+ rows |
| Increment per hint | 37,333 of 37,456 single-source levels are multiples of $\iota$, capped at 5 |
| $hf$ by card level, LL | floored linear interpolation −78,362.7; step −78,371.3; maximum-limit-break $hf$ for all −78,396.3 |

*The level-5 cap.* Removing a skill at level 5 beats drawing with replacement by 22.1 nats, with no extra parameter:

| Hints to reach level 5 | Rows | Observed / predicted, with replacement | With the cap |
|---|---|---|---|
| 5 | 44,737 | 0.967 | 0.975 |
| 3 | 3,011 | 0.945 | 0.952 |
| 2 | 45,649 | 1.032 | 1.031 |
| 1 | 465 | 1.221 | 0.968 |

*Covariates and cards.* The in-sample significance labels below use unadjusted composite-likelihood tests. The
held-out comparisons retain full-pool specialty weights. On top of the adopted rate, over 87,801 rows:

| Added term | In-sample gain | Held out | Verdict |
|---|---|---|---|
| card type, 4 parameters | +2.8 nats | | not significant |
| initial bond | +0.2 nats | | not significant |
| pool size | +0.8 nats | | not significant |
| rarity, SR against SSR | +2.9 nats | 1.8 ± 1.4, $z$ = 1.3 | detectable, not predictive; factor 0.971 |
| pals per deck | +12.6 nats | 12.2 ± 2.9, $z$ = 4.2 | known miss; factor 1.028 per pal |
| a factor per card | | 48.2 ± 6.0, $z$ = 8.0 | known miss, not modelled |

Cards differ beyond chance. On the adopted rate with a level per scenario, the cards with 200 or more rows read:

| Per card | Value |
|---|---|
| cards, rows | 58, 83,075 |
| observed / predicted | 0.80-1.17 |
| joint run-clustered $\chi^2$, degrees of freedom | 445, 58 |
| $p$ | 4e-61 |

*Other sources of the same skill.* The fit uses rows where no inspiration hinted the skill. Where something else also
hinted it, the card's own hint is seen this often, against the adopted rate:

| Other source of the skill | Rows | Observed | Predicted | Ratio |
|---|---|---|---|---|
| none | 77,728 | 0.341 | 0.339 | 1.005 [0.999, 1.011] |
| inspiration | 8,883 | 0.262 | 0.356 | 0.73 |
| inspiration + card event | 409 | 0.213 | 0.351 | 0.61 |
| card event | 2,191 | 0.332 | 0.336 | 0.99 |
| other card's training | 3,557 | 0.344 | 0.358 | 0.96 |

With no other source the adopted rate is consistent with the aggregate interval. A skill hinted by inspiration is
hinted by training less than predicted, which is not modelled. The cause is not that such a skill leaves the card's
pool. With two pool skills hinted by inspiration, the rest are hinted at 0.371:

| If the two skills | The rest are hinted at |
|---|---|
| stay in the pool | 0.340 |
| leave the pool | 0.462 |

*Prioritised skills* are hinted by training at the model's rate, as it assumes:

| Runs showing a priority list | Prioritised pool-skill rows | Hinted by training | Predicted |
|---|---|---|---|
| 1201 | 868 | 0.336 | 0.326 |

## 8. Chain events

Computed by `research/s08_chains.py`.

### Findings

- **A career gets a random number $N$ of chain events, Binomial(21, 0.35)**, the same for every deck and scenario.
  The scenario comparison finds no significant difference under its nominal test.
- **Binomial(21, 0.35) is kept by choice.** No alternative crosses the approximate held-out threshold; that does not
  establish equivalence. The free distribution fits better in-sample, but its nominal rejection needs recalibration.
  Binomial(18, 0.405) is the candidate the runs now favour in-sample.
- **Each event goes to a chain card in proportion to its remaining links.** This fits better than equal shares per
  card.
- **Pal events are assumed to use no chain budget.** The fitted means by pal count do not establish that mechanism.
- **Known misses**: two deck and trainee effects on $N$, which the model's single $N$ does not have. Causes unknown: in
  crowded decks, with many chain cards and few pals, early links run short and more chains never start than
  predicted. Final links fit pooled, but individual cards differ far beyond chance.

### Model

A run gets $N \sim \mathrm{Binomial}(21, 0.35)$ chain events, mean 7.35. Each event advances one non-pal chain card by one
link. The card is picked in proportion to its links not yet fired: uniformly over the remaining *slots*. A chain is 3
links on an SSR and 2 on an SR, and a stopped chain takes no more events. Pal and group cards take no slot
(**assumed**; the comparisons below are descriptive).

A chain can stop early (**assumed**, from section 9's equal outcomes). If IT's default choice at a link ends the chain
on $k$ of its $n$ outcomes, the chain stops there with chance $k/n$. Its unused slots return to the pool. The model
enumerates the exact distribution of final chain states (section 12).

### Evidence

*Reading a chain's progress.* Progress is read from marker skills, on 5,316 runs. A link is observable only if every
choice and outcome of its event carries a skill appearing in no other event of that card. A credit naming two sources,
"A or B event", counts as unknown when B is another deck card whose events can give the marker. So does a marker listed
only under inspiration. The fitted distribution of $N$ is the one that best explains progress over all decks.

*The marker allowance.* The fit lets a fired link's marker go unseen with chance 0.02. Without an allowance, a run whose
later link shows while an earlier one does not has likelihood 0. Since links fire in order, those runs measure the
rate, and 0.02 is set to it:

| Earlier links implied fired by a later one | Showing no marker | Share [95%] |
|---|---|---|
| 1,800 | 38 | 0.021 [0.014, 0.028] |

The cause is not known, and the fit does not depend on it. Across allowances from 0.01 to 0.045, the best $p$ at
$n = 21$ stays within 0.34-0.35. That holds too with no allowance, on the 5,280 runs that need none.

*Which distribution for $N$?* The table keeps the scripts' nominal tests. The fixed adopted binomial has no fitted
parameters, so subtracting the fitted binomial family's dimension is inappropriate. A free integer $n$ also lacks the
continuous parameter required by an ordinary Wilks profile interval. Latent-distribution tests need identifiability
and boundary checks before their reference distribution can be used. In-sample, against the free distribution:

| Form for $N$ | Nats behind the free distribution | p-value | Note |
|---|---|---|---|
| free distribution | 0 | | mean 7.22, variance 4.44 |
| Binomial(21, 0.35) | 21.9 | 6.5e-05 | rejected; 14 fewer parameters |
| Binomial($n$, $p$), both fitted | 12.1 | 0.042 | best at $n$ = 17, $p$ = 0.426 |
| Conway-Maxwell-Poisson | 16.9 | 0.002 | |
| Binomial(15, 0.48) | 19.4 | | |
| Poisson(7.1) | 136.1 | | variance = mean, too wide; rejected |

$N$ is under-dispersed: the free fit's variance is 0.62 of its mean. The fitted Poisson scores worse; these moments alone do not reject every
over-dispersed count distribution. The binomial is the simplest under-dispersed count. It fits better than the
Conway-Maxwell-Poisson, the other two-parameter family that can be under-dispersed, by 4.7 nats.

The runs fix $N$'s mean well and its spread loosely, so $n$ and $p$ trade off along a ridge. The mean at the best $n$
is 7.25. Re-fitting $p$ at each $n$, every $n$ from 16 to 19 lies within 1.92 nats of the best: the 95%
nominal profile set, whose coverage is uncalibrated for integer $n$. So the best single pair is not itself a finding.
The adopted $n = 21$ lies just outside that set
in-sample, 6.41 nats below the best $n$. Its variance is 4.78.

*Does the in-sample preference survive held out?* No alternative crosses the approximate threshold. Each form was fitted on four groups of runs and scored against
Binomial(21, 0.35) on the fifth:

| Form | Held-out nats over (21, 0.35) ± SE | $z$ |
|---|---|---|
| free distribution | +12.6 ± 6.6 | +1.90 |
| binomial, $n$ and $p$ by likelihood | +6.4 ± 4.8 | +1.34 |
| binomial by moments | +4.7 ± 4.0 | +1.17 |
| Conway-Maxwell-Poisson | +2.0 ± 4.9 | +0.41 |
| Poisson | −108.8 ± 11.6 | −9.37 |

No alternative has a clear held-out advantage under the approximate comparison, and the Poisson scores worse. Keeping (21, 0.35) is a decision rather than
a fit. In-sample the candidate Binomial(18, 0.405) fits 8.8 nats better. It lies inside the profile interval, with
mean 7.29 and variance 4.34.

*Does $N$ depend on the deck or trainee?* For two, yes. The model has one $N$ for every deck and trainee, so it
under-predicts the first and over-predicts the second, both known misses:

| Group | Runs | Mean $N$ | Mean $N$, other runs | $p$ at $n = 21$ | Same, other runs | Likelihood-ratio $p$ |
|---|---|---|---|---|---|---|
| decks with Sasami Anshinzawa [This Might Sting!]'s pal card | 639 | 7.74 | 7.17 | 0.374 | 0.342 | 3e-07 |
| Inari One's careers | 473 | 6.83 | 7.26 | 0.326 | 0.347 | 0.001 |

On the runs with neither, Binomial(21, 0.35) is still rejected against a free distribution, $p$ = 0.002.

*Scenarios and pals.* A free distribution per scenario gains 29.6 nats for 48 extra parameters, below the nominal threshold.
The fitted means by pal count do not decrease monotonically. Deck composition and weakly observed counts beyond
short decks' completion prevent this from identifying whether pal events consume chain budget. Over 4,791 runs with 1-4 pals:

| | Unity Cup | Grand Concert | URA Finale | Trackblazer | | 1 pal | 2 pals | 3 pals | 4 pals |
|---|---|---|---|---|---|---|---|---|---|
| Mean $N$ | 7.60 | 7.14 | 7.02 | 6.99 | | 7.36 | 7.05 | 7.41 | 8.40 |

*Final links by pal count.* These comparisons also use latent reach without the marker allowance. The binomial lies
inside the observed 95% interval at 3, 1 and 0 pals. It lies above it at
5 and 2 pals and below it at 4.

| Pals | Chain cards | Runs | Final link reached [95%] | Binomial(21, 0.35) | Poisson(7.1) |
|---|---|---|---|---|---|
| 5 | 1 | 127 | 0.952 [0.899, 0.978] | 0.979 | 0.961 |
| 4 | 2 | 552 | 0.892 [0.869, 0.915] | 0.860 | 0.794 |
| 3 | 3 | 1,426 | 0.583 [0.565, 0.602] | 0.566 | 0.514 |
| 2 | 4 | 1,399 | 0.318 [0.304, 0.332] | 0.342 | 0.326 |
| 1 | 5 | 1,414 | 0.224 [0.213, 0.234] | 0.225 | 0.221 |
| 0 | 6 | 398 | 0.146 [0.130, 0.163] | 0.158 | 0.159 |

*Slots or cards?* The slot rule has the higher sum of marginal link scores. The alternative gives each running card's next event the same chance. The slot rule scores
101.9 nats better over 5,316 runs. With no pals it lies inside the observed interval and the card rule outside.
With two pals both predict more final links than seen, the card rule further.

| | Observed | Uniform over remaining slots, adopted | Uniform over cards |
|---|---|---|---|
| Total LL | | −11,046.4 | −11,148.4 |
| Final links, 2 pals | 0.32 | 0.34 | 0.41 |
| Final links, 0 pals | 0.15 | 0.16 | 0.23 |

*Two-chain decks.* The prediction below is latent completion, while the observations require both markers to show.
The marker allowance must also be applied before this is a calibration check.
Decks with two 3-link SSRs and four pals need $N \ge 6$ to finish both chains:

| Runs | Both chains finished | Share [95%] | Predicted |
|---|---|---|---|
| 259 | 217 | 0.84 [0.79, 0.88] | 0.80 |

*Known miss: early links in crowded decks run short.* An early link is one before a chain's final link. The shortfall
is the share of early links the model expects to fire minus the share seen, ± its run-clustered standard error. $z$
sums each run's links as one cluster: $z = \sum(\text{pred} - \text{obs}) / \sqrt{\sum(\text{pred} - \text{obs})^2}$.

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Early-link shortfall | 0.02 ± 0.01 | 0.04 ± 0.01 | 0.08 ± 0.01 | 0.05 ± 0.01 | 0.14 ± 0.02 |
| $z$ | 1.3 | 4.7 | 6.5 | 3.7 | 5.8 |

*Known miss: individual cards' final links.* Pooled, final links fit: 0.348 observed against 0.353. Card by card they
do not. The test covers the 50 cards with 60 or more final-link observations, clustered by run since one run's
chain cards share its $N$. It gives $\chi^2$ = 386, where chance alone gives about 50. The cause is not known.

*Known miss: chains that never start.* More chains never start in crowded decks than predicted:

| Chain cards | Never started, observed | Model |
|---|---|---|
| 5 | 0.19 | 0.13 |
| 6 | 0.34 | 0.20 |

A per-card activation parameter remains unresolved. The disjoint coarse grids for activation and its null give
−3.0 nats at $a = 0.97$.
This is not a nested maximised comparison and cannot reject activation.

## 9. Event choices and outcomes

Computed by `research/s09_outcomes.py` (the certainty checks: `research/s13_calibration.py`).

Two rules are **assumed** (`docs/MODEL.md`). IT takes an event's top choice unless a prioritised skill sits on another
choice, and the player prioritises every target. A choice's outcomes are equally likely unless measured; master.mdb
holds no outcome odds.

A target counts as *prioritised* when it is on the run's priority list and not outranked. It is outranked when a
higher-ranked prioritised skill sits on a rival choice of every event offering it. That affects 15 event-only rows;
which way the rank runs is assumed, not tested. The classifier does not compare each choice's best-ranked skill,
so a higher-ranked companion on the target's own choice can invalidate its outranked label.

### Findings

- **IT follows the choice rule** in the two events where priority lists show the choice taken.
- **The rule's consequence shows on real careers.** Event-only hints of targets not prioritised read 0.653 of the
  model, far below prioritised ones.
- **Known miss: event-only hints of prioritised targets read just below the model**, at 0.970.
- **Five measured outcomes depart from equal shares**, and the model uses the measured values. Some other outcomes are
  off the equal rule but left at it: known misses.
- **Pal and group finales are assumed certain.** Every checked run is consistent with that
  assumption; finite samples do not establish certainty.

### Measured shares

These shares are **fitted**, with 90% intervals. Riko's adopted value is a per-career approximation under the observed
choice policy. Festa's interval treats estimated latent assignments as binomial trials and omits their uncertainty. A *reading* is a firing where every outcome could show: her hint
credited, and the skill's level from other sources below 5.

| Event, outcome | Observed | Share [90%] | Adopted |
|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] finale, Nothing Ventured, Risky Business prioritised | 58/77 | 0.75 [0.66, 0.82] | 0.75 |
| same, choice 2 alone, no inherited Risky Business | 37/45 | 0.82 [0.71, 0.90] | |
| Riko Kashimoto [Planned Perfection] "Unexpected Side", Ramp Up, top choice taken | 527/585 | 0.90 [0.88, 0.92] | 0.75 |
| Seiun Sky [Foolproof Plan] link 3, Fast-Paced | 33/40 | 0.82 [0.71, 0.90] | 0.75 |
| Smart Falcon [My Umadol Way! ☆] link 3, Center Stage | 37/51 | 0.73 [0.61, 0.81] | 0.75 |
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante", Nothing Ventured | 151/247 | 0.61 [0.56, 0.66] | 0.60 |

The other outcomes: Riko Kashimoto's alternative is Maverick ○, Seiun Sky's Escape Artist, Smart Falcon's Prudent
Positioning. Sasami Anshinzawa's rows use the 254 runs with her card and without Nakayama Festa's; Nakayama Festa's
start from 671 runs with her card, then exclude her own careers and decks with Sasami Anshinzawa's SSR.
The split-choice, Sasami Anshinzawa and Daitaku Helios analyses use the earlier collection subset, not the full pool.

### Evidence

*Does IT follow the choice rule?* Where it can be seen, yes:

| Event | Condition | Choice taken |
|---|---|---|
| Sasami Anshinzawa's finale | Risky Business prioritised | choice 2 in about 95% of runs |
| Sasami Anshinzawa's finale | neither skill prioritised | choice 2 in 0 of 35 runs |
| Riko Kashimoto [Planned Perfection]'s event | Lone Wolf prioritised | bottom choice in 55 of 62 runs |

Of the 4 exceptions in the first row, 3 had Risky Business already at hint Lv 5. Every run has a priority
list, but the records show it for only 1201 of 5,534 runs. The rest count as unknown, never as an empty list.
So the rule is tested mainly through its consequences in section 13. There the prioritised rows read just below the
model, a known miss. Assuming every target prioritised over-predicts the rest by far more.

*Reading Sasami Anshinzawa's finale.* Nakayama Festa [Trusting the Dice]'s "Upping the Ante" also gives Nothing
Ventured and is credited jointly with it, so runs with Festa's card are left out. Hint levels identify the choice:

- her Nothing Ventured +3 is choice 1, and +1 is choice 2;
- a loss on choice 2 shows as her Risky Business at level 2, from storyline choice 3 and the finale.

With inherited Risky Business levels mixed in, choice 2 reads 0.77-0.87. Choice 1 gives Nothing Ventured +3 or nothing.
If the finale always fires, choice 1 won in 69 of 127 runs. Grand Concert reads lower, unexplained.

*IT keeps her dates.* With Risky Business prioritised, IT also takes the bottom choice of her date-start event "A
Pleasant Practice Partner?", which gives Risky Business +1. The extra +1 shows that it does:

| Choice-2 wins at her finale | With the extra Risky Business +1 |
|---|---|
| 54, Risky Business prioritised | 53 |
| 20, not prioritised | 1 |

GameTora marks that choice as ending her event chain, which for a pal card means her dates. Yet the finale still
fires: at least 75 of 77 such runs show its outcome. IT apparently keeps her dates either way.

*Why Riko Kashimoto's adopted share is lower than measured.* The adopted value is a per-run chance of her Ramp Up
hint, whichever choice IT took. Transferring it to a policy that prioritises Ramp Up is an assumption:

| Riko Kashimoto [Planned Perfection] | Count | Share [90%] |
|---|---|---|
| runs with her Ramp Up hint | 521 of 689 | 0.756 [0.728, 0.782] |
| firings where IT took the top choice | 579 of 725 | 0.80 |
| firings where IT took the bottom choice, Lone Wolf | 146 | |

*Reading Nakayama Festa's share.* The following credits are from the eligible subset of the
671 runs with her card:

| Credit | Runs |
|---|---|
| to her alone, Nothing Ventured | 151 |
| to her alone, Risky Business | 71 |
| Risky Business, to "her or another card's event" | 58, Team Sirius in 48 |
| neither outcome, with Risky Business already at Lv 5 | 43 |

A shared credit is split by estimated source rates. Let $r_F$ be her exclusive Risky Business credit rate in runs with
her card, 0.111. Let $r_o$ be the rate at which the other card alone is credited with it in runs without her:
0.848 for Team Sirius, 0.445 for Sasami Anshinzawa. The estimator assigns a shared credit to her with weight
$r_F / (1 - (1 - r_F)(1 - r_o))$, which gives her 8.3 of them.

In the blank runs a Risky Business win cannot show, or the event never fired. They are split by the rate at which the
event shows where both outcomes can, 0.38. That gives the share in the measured-shares table. Whatever share of
the shared credits is hers, it stays within 0.504-0.635 under the same blank-case correction.
This attribution assumes independent source firings but substitutes exclusive credits for marginal firing rates.
The blank-case correction does not condition on the missing gold outcome. Both corrections need a joint observation
model, and no final-Wit test is computed here.

*Shares left at the equal rule.*

| Event, outcome | Share | Verdict |
|---|---|---|
| Tazuna Hayakawa SSR finale, Concentration | 0.70 | off 0.5, left at the equal rule: known miss |
| Agnes Tachyon [Q≠0] link 1 choice 2, Murmur | 0.70 | off 0.5, left: known miss |
| Marvelous Sunday [Dazzling Day in the Snow] link 3, Position Pilfer | 0.58 | off 0.5, left: known miss |
| Light Hello SSR finale, +3 See Ya Later! | 0.71: 1,497 of 2,103 | off 0.5, left: known miss |
| Gold Ship [That Time I Became the Strongest] link 3 | 0.61 | consistent with 0.5 |
| King Halo [Tonight, We Waltz] link 3 | 0.57 | consistent with 0.5 |
| Daitaku Helios [Sunlit Angel ♪] link 2, Tail Held High, where link 3 visibly fired | 0.47 | consistent with 0.5 |

*Pal and group finales are assumed certain.* Light Hello [From the Ground Up] credits See Ya Later! to her event in 1679 of
1679 runs carrying her. Team Sirius is credited with Risky Business, alone or shared, in 65 of 65.

## 10. Random card events (not modelled)

Computed by `research/s10_random_events.py`.

A deck card's character has *shared random events* that can fire in another uma's career; any rarity counts, R cards
included. $K$ is the number on offer: every shared event of the deck's characters plus the trainee's own. A *reading*
is one such event in one run, read through the marker skill on its top choice.

### Findings

- **A shared random event fires in about 0.14 of the careers it can fire in** (**fitted**). This is a lower bound.
  The rate falls a little as more events are on offer, but a constant rate predicts unseen runs as well.
- **Leaving them out under-predicts the targets they can hint**, which read 1.24 of the model.
- **On rows with a positive baseline prediction, crediting them has no clear held-out advantage.** Rows predicted
  impossible are excluded from that score, so it cannot settle the value of adding a previously missing source.
- **The card's initial friendship gauge raises the rate.** This is a known miss of the measured rate; friendship bonus
  and specialty priority show no effect.
- **Two default targets are affected**: Vodka's Nimble Navigator and Nice Nature's Ramp Up, the only ones that are a
  character's top-choice marker.

### Evidence

*Reading the rate.* A hit is a top-choice marker credited to the character's event. An *ambiguous credit* names another
deck card whose events can also give the marker; as in section 8, it is left out as unknown.

| Count | Value |
|---|---|
| readings | 8,169, from 4,232 runs |
| clean hits | 1,128 |
| ambiguous credits, left out | 222 |
| readings where the marker was already at Lv 5 from other sources | 153 |

| Way of counting | Rate |
|---|---|
| as fitted | 0.138 |
| ambiguous credits as hits | about 2 points higher |
| only where the event could show | 0.141 |
| the same, with ambiguous credits as hits | 0.164 |

A firing is also missed when IT takes another choice for a prioritised skill. So the rate is a lower bound.

*Which form?* None beats the constant rate held out:

| Form | LL in-sample | Held out, against the form named |
|---|---|---|
| constant 0.138 | −3,279.6 | |
| $K$ form, $0.18(1 - 0.034)^{K-4}$ | −3,275.6 | +1.8 ± 2.9 nats against the constant, $z$ = +0.63 |
| power law $c(K/8)^{-a}$, $a = 0.32$ | −3,275.5 | +0.1 ± 0.5 against the $K$ form, $z$ = +0.28 |
| strict competition $1 - (1 - 1/K)^S$, $S = 1.47$ | −3,295.8 | −18.0 ± 6.4 against the $K$ form, $z$ = −2.82 |
| slot form $c / T$, $c = 1.75$ | −3,275.5 | −4.0 ± 2.2 against the $K$ form, $z$ = −1.80 |

The $K$ form says each extra event on offer lowers the rate by a fixed fraction. It gives 0.18 at $K = 4$ and 0.12 at
$K = 14$. Its slope is detectable, run-clustered Wald $|z|$ = 2.91, but does not predict unseen runs better. Uniform
competition would make the power law's $a = 1$. Strict competition predicts unseen runs worse.

The slot form lets every event slot compete: $T = K + w \cdot \text{pals}$, $w$ being what a pal weighs. Chain links
take no weight, and pals do compete:

| Slot weight | Estimate [95%, run-clustered] | Test |
|---|---|---|
| a chain link | 0 | weight 1 rejected, score test |
| a pal, $w$ | 1.15 [0.65, 1.65] | 0 excluded |

*What else moves the rate?* The measured effects are below. The within-run check does not detect dependence;
independent firing remains an assumption.

| $K$ | 4 | 6 | 8 | 10 | 12 | 14 |
|---|---|---|---|---|---|---|
| Observed hit rate | 0.17 | 0.18 | 0.16 | 0.14 | 0.13 | 0.13 |

A logistic slope in $K$ gives 15.3% at 8 on offer and 12.4% at 14. The card effects were fitted together next to $K$:

| Effect, per 10 points | Logit [95%] | Holm-adjusted run-clustered $p$ | Verdict |
|---|---|---|---|
| initial friendship gauge | +0.102 [+0.041, +0.164] | 0.0034 | raises the rate; +0.101 on LB4 cards alone |
| friendship bonus | −0.050 | 0.60 | none |
| specialty priority | +0.016 | 0.60 | none |

The model leaves these events out, so the gauge changes no prediction; it is a known miss of the measured rate. Rarity
within a character shows no effect: pooled, SSR markers fire at 14.6% and SR at 11.6%. Scenario is not detectable,
run-clustered $p$ = 0.19. A rate per scenario does not predict unseen runs better, $z$ = −0.75.

*Bottom-choice skills are rarely credited.* In the 1201 runs showing a priority list:

| Bottom-choice candidates | Count or rate [95%] |
|---|---|
| prioritised | 33 of 2247 |
| credited, not prioritised | 0.076 [0.065, 0.088] |
| credited, prioritised | 4 of 33 |
| top-choice markers credited, for comparison | 0.138 |

*What does leaving them out cost on real careers?* The test uses section 13's rows, each on the run's own deck. It
scores whether the target was acquired from any source, so it does not depend on reading the event.

| Rows where a deck card's random event could hint the target | Count |
|---|---|
| all | 305 |
| target credited to that event | 35 |
| the same, with no other source | 22 |
| rows the model gives no chance | 73 |
| of those, hinted by the event | 6 |

| Events | Acquisition, observed / predicted [95%] |
|---|---|
| left out, as modelled | 1.24 [1.11, 1.38] |
| credited at the fitted rate, about 0.14 there | 1.00 [0.90, 1.11] |

Held out, on the 232 rows with positive baseline predictions and with the rate fitted on the other groups'
readings, crediting gains 4.9 ± 2.8 nats. That is $z$ =
1.76, short of 1.96. Crediting would raise an affected run's odds by a median 13%, over the targets it can land.
The largest rise is 156%, over 232 affected runs. The trainee's own random event can be told apart only for Nice Nature: it
gave the target on 0 of 6 rows.

## 11. Conditional character events (not modelled)

Computed by `research/s11_conditional.py`.

The trainee's secret and race-result events are left out of the model. A *conditional-only row* is a run-target row
where only such an event can give the target: none of the trainee's story, date, outfit or shared random events can,
nor the scenario's.

### Findings

- **The conditional-only sample does not settle the cost of omission for individual trainees.** Crediting the measured
  rates on positive-baseline rows gains 0.5 nats.
- **The exception is Vodka's Nimble Navigator.** Her own events give it in most of her runs, and the model
  under-predicts it. Her random and conditional events both give it and cannot be told apart.
- **Tokai Teio's Head-On is modelled as always acquired**, like Yamanin Zephyr's Slipstream. All three of its races
  are her career goals. The goals ask only for top 5, top 5 and top 3, but IT won all three in 62 of 62
  runs training her. Every run where her event could show credits Head-On. Certainty remains an assumption; her goals
  require placings, while the event requires wins.

### Evidence

| Rows | Her events gave the target | Rate [95%] |
|---|---|---|
| all conditional-only rows | 5 of 89 | 0.056 [0.024, 0.125] |
| Daiwa Scarlet, any outfit, Nimble Navigator | 0 of 84 | [0.00, 0.04] |
| Mejiro Ardan, any outfit, Ramp Up | 4 of 4 | [0.51, 1.00] |
| Vodka, any outfit, Nimble Navigator | 25 of 27 | [0.77, 0.98] |

| Rows, model as it is | Acquisition, observed / predicted [95%] | Crediting the measured rates gains |
|---|---|---|
| conditional-only | 1.10 [0.81, 1.39] | 0.5 nats |
| with Vodka's rows | 1.33 [1.12, 1.53] | 16.2 nats |

## 12. The composed career model

The composition is **derived given the independence assumptions** in sections 1-11 and below; there is nothing further
to fit and no script. An independent implementation (`val/joint_check.py`) reproduces the solver's exact odds on every
reference loop (section 6).

A target lands if the deck hints the gold and it sparks, or else if any source hints the white and that sparks. For a
target $k$ not guaranteed:

$$p_k = (1 - m^{g}_k)\,G_k + W_k\left[m^{g}_k - m^{0}_k(1 - e_k)(1 - q_k)\right].$$

Here $G_k = 0.4\cdot 1.1^{n_k}$ and $W_k = 0.2\cdot 1.1^{n_k}$ are the spark chances (section 1). $m^g_k$ is the chance
the deck gives no gold hint and $m^0_k$ the chance it gives neither tier, jointly over its sources. $e_k$ is the chance
the trainee's own events hint the white, and $q_k$ that it is inherited (section 3). A skill on the trainee's outfit
replaces the bracket by $m^g_k$ for a white, or gives $p_k = G_k$ for a gold. A scenario-guaranteed gold gives $G_k$.
Inheritance only ever supplies the white. The miss product assumes the deck, own-event acquisition and inheritance
are independent conditional on the model inputs.

Targets are not independent. Chain cards share one run's $N$, a card's training draws are joint (section 7), and one
event's outcomes exclude each other (section 9). So the model fixes the final chain state $c$, with the links where
chains stopped, and the joint training and deck-event outcomes $H$. Independent training processes across cards and
independent event-outcome groups conditional on chain progress are assumed. Given $c,H$ and the lineage inputs,
the remaining inheritance, own-event and generation outcomes are assumed independent across targets, so the
per-target chances multiply:

$$P(\text{all}) = \sum_{c} P(c)\, E_{H\mid c}\!\left[\prod_{k} p_k(c,H)\right] \qquad (\text{exact under the assumptions}).$$

Multiplying the targets' marginal chances given $c$ alone would discard those dependencies. The implementation
enumerates connected target groups and multiplies independent groups (`joint_eval_in`).

*Real legacies.* The same model uses each veteran's real stars, real G1 wins and recorded parents. An unknown parent
brings no relation points, but her sparks count. $n_k$ is then the number of legacies actually carrying $k$. Since
$q_k$ and $e_k$ enter only as $(1 - e_k)(1 - q_k)$, $e_k$ is replaced by $1 - (1 - e_k)(1 - q_k)$.
Relative to the loop's generation constants, $1.1^{n_k - 6}$ scales target $k$'s landing factor; the total scale is
$\prod_k 1.1^{n_k - 6} = 1.1^{\sum_k(n_k - 6)}$, including guaranteed targets (**derived**).

## 13. Calibration against real careers

Computed by `research/s13_calibration.py`.

### Findings

- **Aggregate acquisition is consistent with the product approximation on prioritised-or-unknown rows** (Results). Most of the shortfall
  over all rows, 85%, sits on rows the run's list shows not prioritised.
- **Aggregate inheritance without Tail Held High is consistent with the model**: it reads 0.992 of the model.
- **Known miss: Tail Held High is inherited less often than predicted**, the only target of eight whose interval
  excludes 1. The cause is not known.
- **Deck hints read optimistic, with shortfalls concentrated in event sources.** Event hints of targets not prioritised land
  at 0.65 of the model.
- **Known miss: prioritised targets' event hints read just below the model.**
- **Known miss: Trackblazer's training-hint base over-predicts these runs.** The other scenarios' intervals cover 1.
- **The spark-weighted acquisition check reads 0.93 of the exact joint model**: a weak test that does not measure spark generation.
- **Beyond the eight targets**, inheritance reads slightly short of the model, spread over many skills: a known miss.
  Training-pool hints are under-predicted.

### What is compared

Each real career is rebuilt as the model sees one: the trainee in her outfit, the scenario, the six cards at their own
limit breaks, both parents with their recorded parents, every ancestor's white sparks, and race wins giving the
recorded shared-G1 counts. The model gives, for each default target, the chance of each step (sections 3-12). The run
record says what happened.

| Careers | URA Finale | Unity Cup | Grand Concert | Trackblazer | Run-target rows | On her outfit | Scenario golds |
|---|---|---|---|---|---|---|---|
| 5,198 | 754 | 1,148 | 3,213 | 83 | 41,584 | 2,001 | 2,296 |

Rows on her outfit and scenario golds are *certain*: they could only count as successes, so tests 1 and 3 leave them
out. The rest are *open targets*. Intervals are clustered by run, since a run's eight targets share its deck and
lineage. The runs end before skills are bought, so "acquired" means hinted, the model's own meaning.

The deck and acquisition tests use the career report's `buy` product approximation across cards. Its white chance is
conditional on no gold in that approximation; it is not an unconditional white marginal. Shared chain progress can
make these predictions differ from the joint engine's acquisition probabilities. The landing test uses exact spark
marginals, and the all-target spark-weighted check uses exact `p_all`.

1. **Deck hint**: does a deck card hint the skill, by training or by its events, as often as sections 7-9 predict? The
   prediction is $d^g + (1 - d^g)d^w$, with $d^g$ the gold chance and $d^w$ the white chance conditional on no gold in
   the product approximation; observed is a deck card credited with a hint of the skill. Only rows where no
   inspiration hinted the skill are used (section 7). The rows split by where the deck's chance comes from:
   **training pool only** or **events only**. Events only splits again by whether the target was **prioritised**
   (section 9), for runs that show a list.
2. **Inheritance**: does an inspiration of the skill fire as often as $q$ (section 3)?
3. **Acquired**: does the skill end up hinted, from any source, as often as all modelled sources together predict?
   This is what the odds rest on. It is also read on rows with the target prioritised or the run showing no list;
   the latter have unknown priority status.
4. **Landing**: the model's $p$ against the same chance computed from what the run acquired: $0.4 \cdot 1.1^n$ if the
   gold was hinted, $0.2 \cdot 1.1^n$ if only the white. It checks the acquired step through the spark step. It does
   not test spark generation (section 1).

| Test | Rows | Predicted | Observed [95%] | Obs / pred [95%] |
|---|---|---|---|---|
| 1. Deck hint, no inspiration | 24,526 | 0.255 | 0.236 [0.233, 0.239] | 0.926 [0.912, 0.939] |
| 1. Training pool only | 5,036 | 0.361 | 0.358 | 0.994 [0.959, 1.028] |
| 1. Events only | 3,538 | 0.896 | 0.798 | 0.891 [0.878, 0.903] |
| 1. Events only, target prioritised | 551 | 0.859 | 0.833 | 0.970 [0.940, 0.9997] |
| 1. Events only, not prioritised | 584 | 0.884 | 0.577 | 0.653 [0.613, 0.694] |
| 2. Inheritance | 41,584 | 0.359 | 0.354 [0.351, 0.358] | 0.987 [0.978, 0.997] |
| 2. Inheritance, without Tail Held High | 36,386 | | | 0.992 [0.982, 1.003] |
| 2. Inheritance, Tail Held High | | | | 0.953 [0.918, 0.988] |
| 3. Acquired, any source | 37,287 | 0.513 | 0.506 [0.502, 0.509] | 0.986 [0.980, 0.993] |
| 3. Acquired, deck or inspiration only | 37,287 | 0.513 | 0.497 | 0.971 [0.964, 0.977] |
| 3. Acquired, any source, prioritised or no list shown | 32,848 | 0.506 | 0.505 | 0.998 [0.990, 1.005] |
| 3. Acquired, deck or inspiration only, prioritised or no list shown | 32,848 | 0.506 | 0.497 | 0.981 [0.973, 0.988] |
| 3. Acquired, prioritised | 3,556 | 0.700 | 0.706 | 1.008 [0.993, 1.024] |
| 3. Acquired, not prioritised | 4,439 | 0.559 | 0.509 | 0.911 [0.893, 0.929] |
| 4. Landing, sums of $p$ | 37,287 | 6,933.9 | 6,824.9 | 0.984 |

### Evidence

*The priority list matters.* On the runs that show a list, prioritised targets are acquired at the model's rate and
the others less often: the two "3. Acquired" rows split by priority. The pooled test averages the two with the runs
that show no list.

*Where the deck loses its points.* The shortfall is concentrated in event sources: test 1's split.
Unmodelled scenario and trainee events supply the few points the deck loses: 299 target acquisitions, 0.8% of
rows. On prioritised-or-unknown rows, the deck and inspiration alone read 0.981 of the model.
Agreement after adding unmodelled acquisitions can conceal component errors and does not validate candidate rankings.

*Per target*, observed / predicted:

| Target | Deck hint [95%] | Inheritance | Acquired | Landing |
|---|---|---|---|---|
| Uma Stan / Superstan | 0.79 [0.69, 0.91] | 0.996 | 0.977 | 0.979 |
| Nimble Navigator / No Stopping Me! | 1.04 [0.95, 1.14] | 0.984 | 1.048 | 1.035 |
| Playtime's Over! / See Ya Later! | 0.985 [0.97, 1.00] | 1.000 | 0.991 | 0.996 |
| Ramp Up / It's On! | 0.823 [0.78, 0.86] | 1.009 | 0.927 [0.90, 0.95] | 0.921 |
| Tail Held High / Tail Nine | 0.979 [0.92, 1.03] | 0.953 [0.92, 0.99] | 0.983 | 0.983 |
| Racing Spirit: Stamina | no deck source | 0.999 | 1.042 | 1.035 |
| Slipstream / Perfect Spot! | 0.986 [0.92, 1.06] | 0.974 | 0.991 | 0.953 |
| Risky Business / Nothing Ventured | 0.828 [0.77, 0.89] | 0.928 | 0.891 [0.85, 0.94] | 0.882 |

*Training-pool hints by scenario* test $\beta$ and $d$ (section 7). Only Trackblazer's interval excludes 1: its $\beta$
over-predicts these runs.

| Scenario | Grand Concert | Unity Cup | URA Finale | Trackblazer |
|---|---|---|---|---|
| Observed / predicted [95%] | 1.014 [0.97, 1.05] | 0.928 [0.83, 1.03] | 0.995 [0.89, 1.10] | 0.779 [0.63, 0.93] |

By the lowest limit break of the sourcing cards, every interval covers 1:

| Lowest limit break | LB4 | LB3 | LB0-2 |
|---|---|---|---|
| Observed / predicted | 1.002 | 0.982 | 0.86-0.98, 148-242 rows each |

*Unity Cup's guaranteed golds* show in 1148 of 1148 Unity Cup runs each.

*Gold acquisition*, observed / predicted:

| Gold | Rows | Observed / predicted [95%] |
|---|---|---|
| Nothing Ventured | 1260 | 0.852 [0.80, 0.90] |
| Perfect Spot! | 684 | 0.835 [0.78, 0.89] |
| It's On!, outside Unity Cup | 161 | 0.456 [0.33, 0.62] |
| No Stopping Me!, outside Unity Cup | 77 | 0.958 |
| See Ya Later! | 4,488 | 1.000 |

*Reliability.* The rows are grouped by the model's predicted chance. If the model is calibrated, each bucket's observed
rate matches its mean prediction, shown in parentheses:

| Predicted | Deck hint: rows | Observed | Acquired: rows | Observed |
|---|---|---|---|---|
| 0 | 14,258 | 0.001 | 8,822 | 0.011 |
| above 0 to 0.20 | 110 | 0.227 (0.17) | 1,943 | 0.162 (0.15) |
| 0.20-0.40 | 3,655 | 0.309 [0.294, 0.323] (0.314) | 3,812 | 0.319 (0.30) |
| 0.40-0.60 | 1,840 | 0.452 (0.47) | 4,633 | 0.501 (0.50) |
| 0.60-0.80 | 1,308 | 0.567 (0.73) | 6,996 | 0.689 (0.71) |
| 0.80-0.95 | 674 | 0.726 (0.87) | 4,973 | 0.832 (0.86) |
| 0.95-1 | 2,681 | 0.954 (1.00) | 6,108 | 0.974 (1.00) |

The deck's over-prediction sits in its top buckets, the event rewards. The buckets predicted 0.6 and above hold
91% of the shortfall, and 97% of their rows have an event source. The 0.20-0.40 bucket, almost all
training-pool rows, is within its interval.

*Per run.* A run has 7.17 open targets on average.

| Per run | Runs | Model | Observed [95%] |
|---|---|---|---|
| targets acquired | all | 3.677 | 3.627 |
| targets acquired | 4,123 where the priority assumption fits | 3.489 | 3.475 |
| all open targets acquired, model as product of odds | all | 0.0449 | 0.0423 [0.0372, 0.0481] |
| all open targets acquired, model as product of odds | where the priority assumption fits | 0.0250 | 0.0250 [0.0206, 0.0302] |

The table's "priority assumption fits" group includes runs with no list, whose priorities are unknown.
The all-open acquisition predictions multiply the approximate per-target acquisition probabilities.

*Spark-weighted acquisition of all targets.* $P(\text{all eight})$ is recomputed from what each run acquired and compared with the model's exact
$P(\text{all})$. The intervals bootstrap whole runs:

| Scenarios | Observed / predicted [95%] |
|---|---|
| all | 0.926 [0.80, 1.05] |
| Unity Cup | 0.926 [0.80, 1.07] |
| Grand Concert | 1.05 [0.67, 1.51] |
| URA Finale | 0.82 [0.34, 1.41] |

Trackblazer's model value is 0. The interval allows up to about 1.2× optimism. Only 220 runs acquired every
open target, too few to separate the exact joint from the product.

*Single-source cards.* In a single-source pair a card is the deck's only source of the target, so the deck test scores
that card alone. The table lists every pair of 40 or more rows whose interval excludes 1. It adds two whose intervals
reach 1, Gold Ship's and Sweep Tosho's.

| Card > target | Pred | Obs | Ratio [95%] | Rows | Likely reason |
|---|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | 1.000 | 0.760 | 0.76 [0.72, 0.80] | 437 | the model assumes Risky Business prioritised, which takes finale choice 2 |
| Riko Kashimoto [Planned Perfection] > Ramp Up | 0.750 | 0.490 | 0.65 [0.60, 0.70] | 708 | open; her Lone Wolf choice, when prioritised, accounts for part |
| King Halo [Tonight, We Waltz] > Uma Stan | 0.947 | 0.752 | 0.79 [0.71, 0.87] | 125 | Uma Stan sits on a choice that needs a priority |
| Gold Ship [That Time I Became the Strongest] > Uma Stan | 0.199 | 0.143 | 0.72 [0.42, 1.17] | 84 | open |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | 0.307 | 0.195 | 0.63 [0.40, 0.97] | 77 | open |
| Maruzensky [Sentimental Flare ♪] > Slipstream | 0.244 | 0.291 | 1.19 [1.00, 1.40] | 330 | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | 0.340 | 0.479 | 1.41 [1.12, 1.70] | 96 | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | 0.725 | 0.641 | 0.88 [0.81, 0.95] | 345 | open |
| Sweep Tosho [Lamplit Training of a Witch-to-Be] > Slipstream | 0.334 | 0.293 | 0.88 [0.75, 1.018] | 396 | open |
| Nakayama Festa [Trusting the Dice] > Risky Business | 0.612 | 0.522 | 0.85 [0.78, 0.93] | 462 | open |
| Tokai Teio [Step! Smile! Wink!] > Nimble Navigator | 0.423 | 0.541 | 1.28 [1.09, 1.46] | 148 | open |

*King Halo's Uma Stan follows the priority.* The expectation and the exact Poisson-binomial tail are the model's:

| Runs that list Uma Stan | Rows | Hinted by her card | Expected | $P(X \le \text{hinted})$ |
|---|---|---|---|---|
| prioritised | 38 | 35 | 37.1 | 0.062, consistent |
| a list without it | 12 | 0 | 11.7 | |

*Smaller pairs.* Below 40 rows, 5 more pairs exclude 1. 3 of them are event sources of Playtime's Over!: Yukino Bijin
[Dancing Light into the Night], Yaeno Muteki [Fiery Discipline] and Sirius Symboli [Escorte Étoile]. They hinted it in
1 of 49 runs. The model predicted 0.51-0.84. Their event puts the skill on choice 2, which ends the chain
before its last link, so IT takes it only when the skill is prioritised. That cause is untested here:

| Those runs | Count | Skill prioritised | Skill hinted |
|---|---|---|---|
| showing a priority list | 5 | 2 | 0 |
| without a list | 44 | | 1 |

*Other skills.* Every non-target skill a deck card or spark could give, on a 30% sample of 1,559 runs:

| Test | Rows | Observed / predicted [95%] | Verdict |
|---|---|---|---|
| inheritance | 66,598 | 0.986 [0.973, 0.999] | slightly short: known miss; no one skill carries more than 9% of it |
| training-pool hints | 26,573 | 1.056 [1.045, 1.068] | under-predicted |

The deck test leaves out rows with an inspiration, so it is conditional on no spark, treated as independent of the
deck.
