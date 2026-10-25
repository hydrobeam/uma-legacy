# The model's mathematics

What this project measured, fitted, tested or derived for its model of how a career in Uma Musume's Independent
Training acquires and passes on skills. Every constant the model uses, with its source, is in `docs/MODEL.md`; this
document keeps the evidence.

## Results

On 8,373 real careers, each rebuilt from its own deck and lineage, the acquisition product approximation
has no detected pooled bias on unknown-priority rows and reads slightly low on known-prioritised ones (section 13).
Over all careers it is slightly optimistic, and landing every target at once comes out below its prediction.
Terms, labels and the limits of every test are in *How to read*.

| § | Conclusion | How it is set | Main evidence |
|---|---|---|---|
| 2 | Grand Concert defaults to the 17,500+ star mix; URA Finale, Unity Cup and Trackblazer to the lower one | default chosen from rank scores | Grand Concert: 79% of careers reach 17,500+; the largest relative overstatement from averaging stars was 0.57% on sampled affinities under the assumed independent-slot star mixes |
| 3 | Grandparents pass sparks at the parents' full base rate | fitted | weight 0.99 [0.98, 1.01] against the parents' 1 |
| 3 | The community affinity formula is kept, with +3 per shared G1 win | assumed, tested | held-out gains do not establish improvements over unit weights: slope $z$ = 1.58, shared wins $z$ = 1.52 |
| 4 | The community win table holds on grades raised by pink sparks, with a goal race charged the streak position before it | community table, tested; goal shift fitted | wins 0.997 [0.996, 0.998] of predicted, over 197,254 G1 attempts |
| 5 | A trainee who is her own grandparent gets no trio points from it | fitted | 43 nats better than counting the pair's points |
| 6 | Scheduling loops for total shared wins costs little | derived | with each loop's decks fixed, no schedule raises a reference loop's odds by more than 2.32% |
| 7 | A card's training hints are uniform draws from its pool, a number per scenario raised by Hint Frequency and Specialty Priority | form fixed on the runs; base and specialty weight fitted | real careers' training-pool hints: 0.994 [0.96, 1.02] of the model |
| 8 | Chain events are Binomial(21, $p$) with $p$ rising with the deck's pals, each event going to a running chain card with equal chance | fitted | predicts unseen runs better than today's Binomial(21, 0.35) with links-left weighting: $z$ = 11.5; than the same budget with links-left weighting: $z$ = 6.9 |
| 9 | IT takes a prioritised skill's choice; four outcome estimates replace equal shares | rule assumed, tested on careers; shares fitted | prioritised targets' event hints read at the model's rate: 0.997 [0.975, 1.019] |
| 10 | Random card events are left out | not modelled | the targets they can hint read 1.24 of the model |
| 11 | Conditional character events are left out | not modelled | sparse pooled comparison inconclusive; Vodka's own Nimble Navigator is a known miss |
| 12 | The parts compose into an exact $P(\text{all})$ under stated independence assumptions | derived | an independent implementation reproduces the solver's odds on every reference loop |
| 13 | The acquisition product approximation has no detected pooled bias on unknown-priority rows; known-prioritised rows read slightly above it | tested | unknown priority 0.997 [0.989, 1.005] on 34,730 rows; prioritised 1.014 [1.005, 1.024] on 10,404 rows |
| 13 | Over all rows the model is slightly optimistic, mostly on targets shown not prioritised | tested | all rows 0.990 [0.985, 0.995]; targets shown not prioritised 0.953 [0.943, 0.963], 115% of the shortfall; the cause is not established |
| 13 | Landing every target at once comes out below the model: it is optimistic about $P(\text{all})$ | tested | spark-weighted all-target acquisition 0.86 [0.76, 0.96] of the model; runs showing no priority list 0.78, runs showing one 0.89 [0.78, 1.00]; only 357 runs acquired every target |

### Known misses

Each is measured in its section. *How to read* says why a listed miss can be partly chance.

| Part | Miss | Observed / predicted | Section |
|---|---|---|---|
| Inheritance | Tail Held High is inherited less often than predicted; cause unknown | 0.972 [0.947, 0.997] | 3, 13 |
| Inheritance | parents at affinity 200 or more pass sparks less often | 0.964 [0.948, 0.981] | 3 |
| Inheritance | non-target skills, the shortfall spread over many skills | 0.979 [0.969, 0.990] | 13 |
| Races | aptitude basis <=70, before streak penalties | 0.95 [0.94, 0.96] | 4 |
| Races | chosen races 1st and 2nd in a row | 0.996 [0.994, 0.998]; 0.996 [0.994, 0.998] | 4 |
| Races | goal races 7th or later in a row, which the planner charges as the 6th | 0.60 [0.46, 0.74] | 4 |
| Training hints | cards differ beyond their effects; cause open | per card 0.84-1.18 | 7 |
| Training hints | a skill also hinted by inspiration | 0.74 | 7 |
| Training hints | a skill another card's training also hinted | 0.95 | 7 |
| Training hints | more pals per deck give slightly more hints | factor 1.026 per pal | 7 |
| Training hints | Trackblazer's base over-predicts real careers | 0.746 [0.60, 0.89] | 13 |
| Training hints | non-target skills are under-predicted | 1.038 [1.028, 1.047] | 13 |
| Chain events | Inari One's careers reach fewer events than the pal-count budget predicts | own logit term -0.072, LR $p$ = 0.008 | 8 |
| Chain events | early links run short at 3 pals and long at 1 | $z$ = +2.8 and -3.0 | 8 |
| Chain events | with five chain cards, chains that never start are slightly over-predicted | 0.21 [0.18, 0.25] against 0.25 | 8 |
| Loop ranking | a listed runner-up's pair-ceiling odds pass its winner's | 2 of 13 runner-ups | 6 |
| Calibration | landing every target at once reads below the model | 0.86 [0.76, 0.96] | 13 |
| Calibration | prioritised targets are acquired slightly more often than predicted | 1.014 [1.005, 1.024] | 13 |
| Inheritance | per roll, years with an inspiration of a target read slightly below the model | 0.987 | 3, 13 |
| Event choices | five outcome shares the model leaves at the equal 0.5 read off it: Tazuna Hayakawa [Tracen Reception]'s finale, Agnes Tachyon [Q≠0] link 1, Marvelous Sunday [Dazzling Day in the Snow] link 3, Light Hello [From the Ground Up]'s finale, Gold Ship [That Time I Became the Strongest] link 3 | 0.62, 0.62, 0.57, 0.70, 0.59 | 9 |
| Left out | targets a deck card's random event can hint | 1.24 [1.11, 1.38] | 10 |
| Left out | Vodka's own Nimble Navigator | her events give it in 28 of 30 runs | 11 |

### Not settled

- Whether specialty priority causes more hints. Fitted on earlier runs, it predicts later ones better by
  13.9 nats, but on cards whose limit breaks change only their specialty priority, no
  effect fits as well (section 7).
- The hint-frequency slope $1/100$ (**assumed**). On the adopted model a free slope (divisor
  93) predicts unseen runs marginally better, $z$ =
  2.0 at the 1.96 bound; without the specialty term one
  clearly does (section 7).

### Assumed and untested

Spark generation (section 1) and that every hinted target is bought; the star mixes (section 2); Grand Masters, which
has no runs; that pal events use no chain budget (section 8); hint levels; which targets each run prioritised, since most lists are not shown; and
dependence inside a run beyond what section 12 models.

## How to read this document

**Labels.** **fitted**: estimated from the runs, with the sample and method given. **derived**: computed from the
other parts. **assumed**: a chosen rule or an outside figure, with its source named; no evidence is claimed for it
unless the text says it was tested. A **known miss** is a measured discrepancy the model keeps.

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
  Stamina, Slipstream / Perfect Spot!, Risky Business / Nothing Ventured.

**Intervals.** $x$ [$a$, $b$] is an estimate with its 95% interval, or 90% where marked. Where a run gives one
observation, the interval is Wilson's. Where a run gives several rows, such as targets, race attempts or trials, the
interval is clustered by run, so the run is the unit. Four limits apply throughout:

- An observed / predicted interval treats the predictions as fixed and the runs as independent. It leaves out the
  uncertainty of the fitted constants and any dependence between careers.
- A Wilson interval divided by a mean prediction is an interval for the observed rate. It ignores that the rows'
  predictions differ.
- A joint test over a table's bands is a run-clustered Wald $\chi^2$. It needs many runs and a covariance that is not
  degenerate.
- Every interval is pointwise. One that covers the prediction shows no detected miss, not that the model is exact.
  Among this many intervals some exclude the prediction by chance, so a listed known miss can be partly selection.

**Fit comparisons.** A model's *log-likelihood* (LL) sums the log of the probability it gave to what was seen. For a
joint likelihood, a difference of $\Delta$ *nats* is a likelihood ratio of $e^{\Delta}$. Some scores sum one run's rows
as if they were independent: the training-hint rows and the chain-allocation link scores. Such a sum is a *composite*
likelihood. Its differences rank models but are no likelihood ratio for whole careers. A *free* fit gives each
quantity its own value with no formula, such as a win rate per cell; it is the yardstick a formula is measured
against. Each comparison is one of three kinds:

1. **Same number of fitted parameters**: compared by LL on the same data. This is descriptive; no significance cutoff
   applies to models that are not nested.
2. **Is an extra parameter detectable?** A likelihood-ratio test compares $2\Delta$ with $\chi^2$ on the number of
   extra parameters. That reference holds only for regular nested fits of a joint likelihood. Tests on composite
   likelihoods, on integer parameters such as a binomial's $n$, or at a parameter's bound are reported unadjusted.
   Their p-values are nominal, and nominal significance alone does not establish an effect.
3. **Does a model predict better?** Five-fold cross-validation by run id: each fold refits the prediction on four
   fifths of the runs and scores the rest. The ± is a standard error that treats each run's score difference as
   independent. It ignores the overlap of the training sets and the choice among models, so $z$ is approximate. A $z$
   short of significance does not show an alternative equal or worse. Section 7's covariate and per-card checks keep
   the full-pool specialty weights in every fold, refitting only the per-scenario levels, so they are partly held out.

Every number here is computed by the scripts in `research/`, which also test each conclusion the text draws
(`research/README.md`). A script's **holds** means its stated test passed, within the limits above. Only the assumed
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
score of 17,500, and 20 / 70 / 10% from 17,500 up.

### Findings

- **Grand Concert defaults to the 17,500+ mix; URA Finale, Unity Cup and Trackblazer to the lower one.** Each takes the
  band most of its careers reach, Unity Cup by a small margin. This is a chosen default, not a fit of the stars of the
  veterans players keep in a loop.
- **Averaging over stars slightly overstates the inheritance chance** (**derived**). On the runs' lineages the largest
  overstatement is 0.57%, within the assumed mixes. It is no bound for every
  lineage and says nothing of joint landing odds.

### Evidence

*Which band?* A career's rank score is the run list's estimate with every skill point spent, over every completed run.

| Scenario | Runs | Share at 17,500+ [95%] | Band |
|---|---|---|---|
| URA Finale | 1,357 | 0.000 [0.000, 0.003] | below |
| Unity Cup | 3,155 | 0.467 [0.450, 0.484] | below |
| Grand Concert | 14,742 | 0.794 [0.787, 0.800] | 17,500+ |
| Trackblazer | 282 | 0.000 [0.000, 0.013] | below |

Grand Masters has no runs and is **assumed** 17,500+.

*What does averaging over stars cost?* For a planned loop the model gives each legacy her mix's expected base $\bar b$.
A legacy keeps her stars in both rolls. So her chance of firing in neither is $E[(1 - r_b)^2]$ over her base $b$, where
$r_b = \min\{1,\ b(1 + A/100)\}$ is her chance per roll at affinity $A$ (section 3). The model uses $(1 - r_{\bar b})^2$
instead. That miss is convex in $b$, cap included, so $E[(1 - r_b)^2] \ge (1 - r_{\bar b})^2$. Averaging therefore
understates the miss and overstates $q$, the chance of at least one inspiration of the skill.

The table measures the overstatement for a target all six legacies carry, at the six real affinities of each of the
8,926 lineages in the runs. It treats each legacy's stars as independent of the others'; a veteran
appearing twice in a lineage would share hers.

| Band | Mean overstatement of $q$ | Largest |
|---|---|---|
| below 17,500 | 0.51% | 0.57% |
| 17,500+ | 0.30% | 0.33% |

With real legacies the model uses their actual stars, so this does not arise.

## 3. Inheritance rolls

Computed by `research/s03_inheritance.py` (the roll's parts) and `research/s13_calibration.py` (whole careers).

The roll tested is the community formula (`docs/MODEL.md`). At each of the two inspirations, a legacy carrying the
skill passes it with chance $r = \min\{1,\ b(s)(1 + A/100)\}$. Here $b(s)$ is the base set by the spark's stars $s$,
3 / 6 / 9% for a skill white, and $A$ is the legacy's affinity (section 5). Over the career, the chance of at least one
inspiration of the skill is $q = 1 - [\prod_l (1 - r_l)]^2$ over the six legacies $l$. This assumes firings
independent across legacies and inspirations, given the stars and affinities. The odds use skill whites only.

The parts are **fitted** on *trials*. A trial is one ancestor's white spark in one year in which she is the only one of
the six legacies carrying the skill, so that year's inspiration of it can only come from her. It *passes* when the
spark fires. There are 70,506 parent trials and 166,540 grandparent trials of ordinary ancestors; a trainee's own
character as a grandparent is section 5's.

### Findings

- **Grandparents pass sparks at the full base rate, as parents do.**
- **Affinity scales the roll as the formula says.** Held-out gains for a free slope on affinity or either part
  are too uncertain to establish an improvement over unit weights.
- **Each star count's base matches its trials below affinity 200.** On all trials 1 star count reads below its prediction, through its trials at 200 or more: the parents' known miss below.
- **Known miss: parents at affinity 200 or more pass sparks less often than predicted.** The bands below 200 are
  jointly consistent. The held-out gain from a refitted slope is inconclusive. The model keeps the formula and this
  miss.
- **Whole careers bear out $q$ apart from Tail Held High**, whose miss and the totals are in section 13. One band of
  predicted $q$ lies outside its pointwise interval; the joint test does not reject the model.

### Evidence

"Held out" is rule 3 of *How to read*: five folds by run, positive nats favouring the refit. Each part's weight is
fitted with the other part at 1. The carrier and spark-year checks exclude Tail Held High, leaving
58,611 rows. The predicted-$q$ band check uses all
66,984 rows.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Grandparents take the full base | weight 0.99 [0.98, 1.01] | full base beats half by 3,073 nats | full base |
| Affinity raises the roll | | beats no affinity by 8,679 nats | kept |
| Affinity slope is 1 | 0.96 [0.94, 0.98] | held out 6.1 ± 3.9 nats, $z$ = 1.58 | 1 kept |
| Relation-point weight is 1 | 0.89 [0.83, 0.95] | held out 5.8 ± 3.6 nats, $z$ = 1.58 | 1 kept |
| Weight of 3 per shared G1 win is 1 | 0.95 [0.92, 0.97] | held out 5.9 ± 3.9 nats, $z$ = 1.52 | 1 kept |
| Affinity bands below 200 fit | | $\chi^2$ = 0.8, $p$ = 0.94 over four bands | consistent |
| All five affinity bands fit | | $\chi^2$ = 19.0, $p$ = 0.002 | known miss at 200+ |
| Careers with 1 to 6 carriers | 0.977-0.998 of the model | every interval covers 1 | consistent |
| Years with an inspiration of the skill | 0.987 of the model | 2.86 sd below | known miss, under 2% short; cause not established |
| $q$ by predicted band | | $\chi^2$ = 10.5, $p$ = 0.06 over the 5 bands above 0 | consistent |

*By stars and by affinity.* Of these groups, affinity 200+ lies outside its own interval, and
every trial there is a parent's; a star count that misses on all trials does so through those trials, and below
affinity 200 every star count covers 1.

| Group | Trials | Observed / predicted [95%] |
|---|---|---|
| 1★ | 65,562 | 0.995 [0.97, 1.02] |
| 2★ | 152,254 | 0.980 [0.97, 0.99] |
| 3★ | 19,230 | 0.989 [0.96, 1.02] |
| 1★, affinity below 200 | 48,702 | 0.992 [0.95, 1.03] |
| 2★, affinity below 200 | 113,550 | 0.993 [0.98, 1.01] |
| 3★, affinity below 200 | 14,912 | 1.016 [0.98, 1.05] |
| affinity 0-49 | 17,356 | 0.988 [0.935, 1.041] |
| affinity 50-99 | 149,828 | 0.995 [0.980, 1.011] |
| affinity 100-149 | 2,478 | 1.006 [0.910, 1.101] |
| affinity 150-199 | 7,502 | 1.012 [0.960, 1.065] |
| affinity 200+ | 59,882 | 0.964 [0.948, 0.981] |

*Whole careers by predicted $q$.* Only the band 0.40-0.60 lies outside its own 95% interval.

| Predicted $q$ | Rows | Observed | Mean predicted |
|---|---|---|---|
| 0 | 21,431 | 0.001 | 0 |
| above 0 to 0.20 | 5,735 | 0.156 | 0.16 |
| 0.20-0.40 | 7,395 | 0.300 | 0.31 |
| 0.40-0.60 | 9,886 | 0.495 [0.485, 0.505] | 0.507 |
| 0.60-0.80 | 15,505 | 0.702 | 0.71 |
| 0.80-0.95 | 7,032 | 0.836 | 0.84 |

## 4. Race wins

Computed by `research/s04_races.py`.

The table tested is the community one (`docs/MODEL.md`). A trainee wins a race she enters with chance
$p = \mathrm{clamp}(110 - c_{\text{dist}} - c_{\text{surf}} - \mathrm{PEN}_s,\ 0,\ 100)/100$. In words: 110%, less a
cost for each of her aptitude grades for the race's distance and surface, less a penalty for racing in a row. The
costs are 0 for S and A, rising to 90 for G. The penalty $\mathrm{PEN}_s$ is 0, 0, 5, 20, 30, 50 at streak position
$s$ = 1 to 6 or more.

A race's position $j$ counts consecutive race turns: any race counts, and a turn without one resets it. A *goal race*
is one the career requires; a *chosen race* is one IT chose. The community table charges every race its own position,
$s = j$. The adopted model (**fitted**, below) charges a goal race the position before it, $s = \max(j - 1, 1)$, so goal
races pay 0, 0, 0, 5, 20, 30. The planner counts $j$ up to 6. The tests take IT's choice of each chosen race as
given, and stats are not modelled.

### Findings

- **A goal race takes the penalty of the race before it, and still counts toward the position of the races after it.**
  On runs left out of the fit, this predicts better than the community table and better than leaving goal races out
  of the count. Why the game does this is not known.
- **Grades must include inherited aptitude sparks.** On base grades, which a planned loop uses, the table predicts too
  few wins. Pink sparks raise grades by the rule in `docs/MODEL.md`: at the start of a career, and from the April of
  each inspiration that fires one.
- **The streak penalty is kept.** The previous race's result shows no detectable further effect.
- **The table is kept.** A free win rate per cell has no clear held-out advantage.
- **Known misses.** Races with aptitude basis <=70 (before streak penalties) and chosen races 1st or 2nd in a row win slightly less than predicted.
  Goal races 7th or later in a row win less than the planner predicts, since it charges them as the 6th.

IT's chosen races won at these rates, with the community table's chance in parentheses. Grades include those raised by
inherited aptitude sparks; S is shown as A, since both cost 0. A dot marks a cell with under 30 attempts. Only grade
pairs with 200 or more attempts are shown.

| Races in a row | AA | AB | AC | BB | AD | BC | AE | BD | CC | BE | CD | AF | CE | DD | BF | DE | AG |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 race | 100% (100%) | 100% (100%) | 89% (90%) | 90% (90%) | 80% (80%) | 78% (80%) | 66% (70%) | 70% (70%) | 61% (70%) | 55% (60%) | 57% (60%) | 48% (50%) | 43% (50%) | 45% (50%) | 39% (40%) | 28% (40%) | 19% (20%) |
| 2 races | 100% (100%) | 100% (100%) | 90% (90%) | 91% (90%) | 79% (80%) | 79% (80%) | 61% (70%) | 70% (70%) | 74% (70%) | 50% (60%) | 63% (60%) | 52% (50%) | 41% (50%) | 60% (50%) | 34% (40%) | 31% (40%) | 19% (20%) |
| 3 races | 100% (100%) | 95% (95%) | 85% (85%) | 80% (85%) | 74% (75%) | 73% (75%) | 57% (65%) | 58% (65%) | · (65%) | 43% (55%) | · (55%) | 54% (45%) | · (45%) | · (45%) | · (35%) | · (35%) | 12% (15%) |
| 4 races | 90% (90%) | 80% (80%) | 73% (70%) | 77% (70%) | 61% (60%) | 59% (60%) | 44% (50%) | 50% (50%) | 61% (50%) | 27% (40%) | 31% (40%) | 23% (30%) | · (30%) | · (30%) | · (20%) | · (20%) | · (0%) |
| 5 races | 80% (80%) | 67% (70%) | 56% (60%) | 58% (60%) | 66% (50%) | 44% (50%) | 32% (40%) | · (40%) | · (40%) | · (30%) | · (30%) | 19% (20%) | · (20%) | · (20%) | · (10%) | · (10%) | 0% (0%) |
| 6 or more | 58% (60%) | 54% (50%) | 49% (40%) | · (40%) | · (30%) | · (30%) | · (20%) | · (20%) | · (20%) | · (10%) | · (10%) | · (0%) | · (0%) | · (0%) | · (0%) | · (0%) | · (0%) |

### Evidence

The runs record 197,254 attempts at the 34 single-race G1s, 55,323 of them goal races.

The table calls some races certain, 0% or 100%, and a handful went the other way, so its log-likelihood would be
$-\infty$. Every model compared therefore has its 0% and 100% pulled in to a fitted floor and ceiling. With the same
two parameters in each, the in-sample comparisons follow rule 1 of *How to read*.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Raised grades, not base | observed / predicted: base 1.068 [1.065, 1.070], raised 0.997 [0.996, 0.998] | raised fits 4,911 nats better | raised grades |
| A pink counts from the April of its inspiration | won 0.748 [0.718, 0.777] of 892 Senior January-March attempts where the timing matters | predicted 0.758 from April, 0.868 from the year's start | from April |
| The streak penalty is real | | without it, 4,847 nats worse | kept |
| A loss changes the next race's chance | after a loss over after a win, 4th race on: 1.007 [0.977, 1.039] | run-clustered; at the 2nd and 3rd the interval also covers 1 | not detected |
| Goal races pay one position less | community table, goal races 4th on: 1.135 [1.123, 1.147] | held out, forms table below | shift adopted |
| One formula is enough | | a free rate per chance, streak position and race kind, 112 cells, held out: 47 ± 30 nats, $z$ = 1.55 | table kept |
| Aptitude basis <=70 wins less (grouped before streak penalties) | 0.950 [0.936, 0.964] over 16,002 attempts | interval below 1 | known miss |

By scenario, every interval covers the prediction once chosen races 1st and 2nd in a row are left out.

*Goal races.* Under the community table, goal races from the 4th race in a row win more than predicted. Three forms
were scored on runs left out of the fit, five folds by run over 8,875 runs, each form with its own
fitted floor and ceiling:

| Form | Goal race at position $j$ pays | Races after it count it | Held out, against the adopted form |
|---|---|---|---|
| Adopted | $\mathrm{PEN}_{\max(j-1,1)}$ | yes | |
| Community table | $\mathrm{PEN}_j$ | yes | 253 ± 38 nats worse, $z$ = 6.68 |
| Exclusion | $\mathrm{PEN}$ at the chosen races before it | no | 1,986 ± 81 nats worse, $z$ = 24.56 |

With the shift, goal races from the 1st to the 6th in a row and chosen races from the 3rd on are calibrated: each row
covers the prediction or, on the largest rows (goal j 1, goal j 4), misses it by under 1%,
a fifth of the table's smallest step. Goal races 4th to 6th in a row win 0.995
[0.985, 1.005] of
the prediction, against 1.140 under the community table. Chosen races
1st and 2nd in a row fall slightly short, a known miss. Goal races 7th or later fall well short: the other known miss.

| Race in a row | Goal races: observed / predicted [95%] | Attempts | Chosen races: observed / predicted [95%] | Attempts |
|---|---|---|---|---|
| 1st | 1.001 [1.000, 1.002] | 29,999 | 0.996 [0.994, 0.998] | 68,730 |
| 2nd | 1.000 [1.000, 1.001] | 15,068 | 0.996 [0.994, 0.998] | 51,432 |
| 3rd | 1.001 [0.999, 1.003] | 6,463 | 0.998 [0.994, 1.001] | 13,362 |
| 4th | 0.995 [0.990, 0.999] | 1,602 | 0.998 [0.987, 1.008] | 6,071 |
| 5th | 0.997 [0.976, 1.017] | 1,623 | 0.995 [0.966, 1.023] | 1,817 |
| 6th (goal), 6th or later (chosen) | 0.989 [0.940, 1.037] | 487 | 1.008 [0.927, 1.088] | 519 |
| goal, 7th or later | 0.60 [0.46, 0.74] | 81 | | |

## 5. Affinity

Computed by `research/s05_affinity.py`.

A legacy's affinity follows the game's rule as community tools implement it (`docs/MODEL.md`): the characters'
relation points from the game data, plus 3 for each single-race G1 both umas won. Trainee $T$ has parents $P_1$ and
$P_2$. For either parent $P$, $P'$ is the other parent, and $G \in \mathrm{par}(P)$ are $P$'s own parents, the
grandparents:

$$A_P = \mathrm{pair}(T,P) + \mathrm{pair}(P_1,P_2) + 3\,o(P,P') + \sum_{G \in \mathrm{par}(P)} \big[\mathrm{tri}(T,P,G) + 3\,o(P,G)\big], \qquad A_G = \mathrm{tri}(T,P,G) + 3\,o(P,G).$$

pair and tri are the relation points of the groups holding every character named. $o(a,b) = \sum_r P_a(r)\,P_b(r)$ is
the expected number of single-race G1s both $a$ and $b$ win, where $P_a(r)$ is $a$'s chance of winning race $r$ at
least once over her schedule (section 4, base aptitudes). Schedules fixed in advance and independent careers are
**assumed**. The loop model puts these expected shared wins into the nonlinear inheritance formula; it does not average
inheritance over the joint race outcomes. Section 3 tests the 3 per shared win.

### Findings

- **A trainee who is her own grandparent gets no trio points from it**: $\mathrm{tri}(T,P,T) = 0$ (**fitted**). The
  game data do not say how a group counts one character twice. Counting it as $\mathrm{pair}(T,P)$, as one community
  tool does, fits 43 nats worse.

### Evidence

The fit uses 271,842 trials as section 3 defines them, self-grandparents included. In 1,557 of them a
self-grandparent passed her spark. The distance is observed minus predicted passes, in run-clustered standard
deviations.

| Claim | Estimate | Test | Verdict |
|---|---|---|---|
| $\mathrm{tri}(T,P,T) = 0$ | 1582.3 passes predicted | distance −0.68 | kept |
| $\mathrm{tri}(T,P,T) = \mathrm{pair}(T,P)$ | 1827.2 passes predicted | distance −7.10 | rejected |
| a fraction $\kappa$ of $\mathrm{pair}(T,P)$ | $\hat\kappa = 0$, its lower bound | | 0 kept |

The gap is not the ordinary trials' own misfit: refitting the affinity slope on them, to 0.96, leaves both verdicts
unchanged.

## 6. Loop structures, ranking and schedules

Computed by `research/s06_loops.py`. The structures, their lineages $(T, P_1, P_2, X)$ and their ranking by mean
$P(\text{all})$ are defined in `docs/MODEL.md` and `docs/SOLVER.md`. $X$ is the uma each parent was bred from with the
other parent; a rotation has $X = T$.

### Findings

- **The rotation occurs in practice**: 168 runs have its exact lineage shape.
  More broadly, 2,932 of 8,940 runs have a grandparent of the trainee's own character.
- **Every pair of umas carries the same weight in a structure's summed affinities**, so the schedules maximise the total
  expected shared wins. That total is a surrogate: equal weights do not make it equivalent to the nonlinear landing
  odds.
- **With each loop's reported decks fixed, scheduling for shared wins costs little**: on the reference results, no
  schedule could raise a loop's odds by more than 2.32%.
- **Known miss: the pair ceilings do not certify every listed top.** On 2 results a listed runner-up's ceiling passes its winner's odds, by up to 1.8%; it would need 0.46-2.07% over its schedule found to tie, which the search did not find and the bound cannot exclude.

### Evidence

*Race weights* (**derived** from section 5's $A_P$ and $A_G$). In one lineage, $o(P_1,P_2)$ enters the six affinities
6 times: twice in each $A_P$, since $P'$ is also one of $P$'s own parents. $o(P_1,X)$ and $o(P_2,X)$ enter
2 times each. Summed over a loop's 12 lineages or a rotation's 3, every pair gets one weight:

$$\text{loop: } 6\cdot 2 + 2\cdot 4 = 20, \qquad \text{rotation: } 6 + 2 + 2 = 10.$$

*The schedule bound* (**derived**). The odds are nondecreasing in every $o$: $o$ enters each affinity with weight +3,
and every later step is nondecreasing in its inputs. So, with a loop's decks fixed, no schedule can score above the
odds with every pair's overlap at an upper bound. The bound used is the solver's pair ceiling (`docs/SOLVER.md`,
`pair_ceilings`), with outfits reselected and decks fixed. Each pair's ceiling holds separately, and the pairs need not
reach theirs jointly, so the bound is conservative. The ceilings come from the search code that chose the schedules.
The script checks only that each lies between the overlap found and a loose independent ceiling: every G1 offering at
its best chance, with no streak penalty.

The bound is computed on the 42 reference results, the best loops and decks found for the validation setups
(`val/ref__*`, `val/refpu__*`). An independent implementation reproduces their odds to within 2e-15.

| On the reference results | Value |
|---|---|
| winners' odds gained by the shared wins found, over none | 31-107% |
| most any other schedule could add to a winner, by the pair ceilings | 0.15-2.32%, median 0.87% |
| listed runner-ups whose pair-ceiling odds reach their winner | 2 of 13 |

This bound does not certify the ranking when schedules and decks change jointly.

## 7. Training hints

Computed by `research/s07_hints.py`.

A card's training can hint the skills in its *pool*. $hf$ is the card's Hint Frequency in percent and $\mathit{sp}$ its
Specialty Priority, both at maximum limit break with unique effects included.

### Findings

- **A card makes a nearly fixed number of uniform draws from its pool.** A skill already at hint level 5 is no longer
  drawn.
- **Hint frequency scales that number directly**: Hint Frequency +40% gives 40% more hints. The slope is **assumed**;
  on the adopted model a free one predicts unseen careers only marginally better ($z$ =
  2.0, at the 1.96 bound), a known miss.
- **Specialty priority also raises it**, by a weight that differs between scenarios. The effect predicts careers it was
  not fitted to, but it is not shown to be a cause, and why it differs by scenario is not tested.
- **Known misses**: cards differ in hint rate beyond what their effects predict, for reasons not known. Decks with more
  pals give slightly more hints. A skill also hinted by inspiration is hinted by training less than predicted, and one
  another card's training also hinted slightly less.
- SR cards give slightly fewer hints than SSR cards with the same effects in-sample, but that does not predict unseen
  careers better, so the model ignores rarity.

### Model

A card gives, in scenario $s$, $h$ training hints per career:

$$h = \beta_s \left(1 + \frac{hf}{100}\right)\left(1 + d_s\,\frac{\mathit{sp}}{100}\right).$$

That is a base $\beta_s$ for a card with neither effect, raised in proportion to hint frequency and to specialty
priority. A career gets $\lfloor h \rfloor$ hints, or one more with chance $f = h - \lfloor h \rfloor$.

Each hint draws one skill uniformly from the pool and raises it by the card's hint increment $\iota$. So a skill reaches
level 5 and leaves the pool after $c = \lceil 5 / \iota \rceil$ hints. The runs bear out the increment: 56,011 of
56,138 levels that one card gave alone are multiples of $\iota$, or 5.

The chance that none of $k$ targets in a pool of $n$ skills is hinted is computed jointly, not target by target:

$$M(k, n, h) = (1 - f)\, m(k, n, \lfloor h \rfloor) + f\, m(k, n, \lfloor h \rfloor + 1), \qquad
m(k, n, t) = \left(1 - \frac{k}{n}\right)^{t} \ (t \le c).$$

Each of $t$ hints misses all $k$ targets with chance $1 - k/n$. When $t > c$, an exact Markov chain over the other
skills' hint counts replaces $m$.

### Fitted values

**Fitted** by maximum likelihood on 130,979 rows from 22,641 card-runs. A row is one pool skill of one card in
one run; a card-run is one card in one run's deck. The fit takes SR and SSR cards at maximum limit break and leaves out
rows where an inspiration hinted the skill. The Runs column counts the runs that add a row. The solver reads these
values from `research/results.json` when it is built (`pkg/loopopt/build.rs`), rounded to three decimals.

| Scenario | $\beta$ | $d$ | Rows | Runs | Typical card's hints per career |
|---|---|---|---|---|---|
| Unity Cup | 2.428 | 0.299 | 24,412 | 1875 | 3.5 |
| Grand Concert | 1.990 | 0.082 | 87,736 | 5,499 | 2.7 |
| URA Finale | 1.369 | 0.393 | 15,645 | 960 | 2.0 |
| Trackblazer | 1.579 | 0.162 | 3,186 | 130 | 2.2 |
| Grand Masters | 1.991 | 0.134 | no runs: **assumed**, one $\beta$ and one $d$ fitted over every scenario's rows | | |

The typical card is the runs' median: hint frequency +30% and specialty priority 35.

### Evidence

*The form.* The form was chosen before the specialty term was added, on a wider sample: every card at any limit break,
R cards included, 213,216 rows from 8,940 runs. That study reads the count as continuous and ignores the level-5 cap. On
runs left out of the fit, per-scenario $\beta(1 + hf/100)$ predicts better than either simpler form:

| Alternative | Per-scenario $\beta(1 + hf/100)$ better held out by |
|---|---|
| one constant count, no $hf$ | 2,248.1 ± 38.2 nats, $z$ = 58.8 |
| one $\beta(1 + hf/100)$ for every scenario | 774.9 ± 24.0 nats, $z$ = 32.3 |

*Hint frequency's slope.* The $1/100$ is **assumed**: a card's +40% is read as 40% more hints. A free divisor tests it,
with and without the specialty term:

| Model | Free divisor | Held out, against $1/100$ | Verdict |
|---|---|---|---|
| adopted, with the specialty term | 93 | 2.4 ± 1.2 nats, $z$ = 2.0 | free divisor marginally better (known miss); $1/100$ kept, assumed |
| form study, without it | 82 | 32.1 ± 3.6 nats, $z$ = 9.0 | free divisor better |

*Specialty priority.* Without the specialty term, cards with more specialty priority give more hints than predicted.
With it, the excess is gone:

| Specialty priority | Rows | Observed / predicted, without the term | With it |
|---|---|---|---|
| 0 | 39,776 | 0.964 | 1.004 |
| 1-29 | 1,777 | 0.969 | 0.990 |
| 30-59 | 60,381 | 1.000 | 0.992 |
| 60+ | 29,045 | 1.055 | 1.010 |

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| The term predicts unseen careers | | held out 53.0 ± 5.6 nats, $z$ = 9.4 | holds |
| Fitted on the earlier half of the runs, it predicts the later half | | 13.9 ± 3.3 nats, $z$ = 4.2 | holds |
| A weight per scenario beats one shared weight | | held out 16.2 ± 3.1 nats, $z$ = 5.2 | per scenario |
| With the term, the scenario × specialty cells fit | 14 cells of 200+ rows, 8 parameters fitted on them | run-clustered $\chi^2$ = 20.4: $p$ = 0.00 on the cells less the fitted parameters (conservative), 0.12 on the cells (liberal) | not shown |
| The same card, only specialty priority changing | $d$ = −0.04 [−0.38, 0.46], 32 cards | +0.02 nats over no term | cause undecided |

The last row compares each card with itself. On these cards, from one limit break to the next, specialty priority changes and hint
frequency does not; a base is fitted per card and scenario. The estimate is consistent with no effect and with every
per-scenario weight. A scale on the per-scenario weights, refitted on either half of the runs, on SR or SSR cards alone
and without each of the eight most-used cards, stays within 0.87-1.07.

*The level-5 cap.* Removing a skill at level 5 fits 61.6 nats better than drawing with replacement, with no
extra parameter. It matters most where one hint reaches level 5: with replacement those rows read 1.204 of prediction,
with the cap 0.964.

*The draws' shape.* Three checks bear on the shape of the draws, and none runs the adopted model. The repeat and pair
checks use the form study's rate, without the specialty term or the level-5 cap, and the pair check reads the count as
continuous. The stat-option check assumes a fixed count. They test the shape, not the fitted rate.

| Question | Observed | Predicted |
|---|---|---|
| A skill a card hinted at all, hinted again: $P(k \ge 2 \mid k \ge 1)$ | 0.144 | 0.143 with the two-point count, 0.195 with a Poisson count |
| Two skills of one pool both hinted, over 368,324 pairs, Trackblazer out | 0.0805 | 0.0811 by joint draws, 0.1103 independently |

That a draw never picks the card's stat option, the one non-skill option beside its pool, is **assumed**. In 89 of
107 (card, $hf$) cells the count of draws varies less than a drawn stat option would allow. The check keeps only
card-runs whose every count is exact, with no skill at level 5 or hinted by another source, and uses a sample
variance biased low. It is consistent with the assumption and does
not establish it.

*Covariates and cards.* Card type, initial bond and pool size add no nominally detectable gain to the adopted rate.
Three other terms, tested on 130,979 rows under the limits *How to read* sets out:

| Added term | In-sample gain | Held out | Verdict |
|---|---|---|---|
| rarity, SR against SSR | +3.5 nats | 2.3 ± 1.5, $z$ = 1.5 | not predictive, ignored; factor 0.974 |
| pals per deck | +18.7 nats | 18.4 ± 3.6, $z$ = 5.2 | known miss; factor 1.026 per pal |
| a factor for each of 68 cards with 200+ rows | | 52.2 ± 6.2, $z$ = 8.4 | known miss; cards read 0.84-1.18 |

*Other sources of the same skill.* The fit leaves out rows where an inspiration also hinted the skill. Against the
adopted rate, the card's own hint is seen:

| Other source of the skill | Rows | Observed / predicted |
|---|---|---|
| none | 114,529 | 1.005 [1.000, 1.010] |
| inspiration | 14,283 | 0.74 |
| card event | 3,985 | 1.01 |
| other card's training | 5,611 | 0.95 |

With no other source the adopted rate is calibrated to within 2%: it reads 1.005, an
excess just detectable on 114,529 rows. A card's own event leaves it calibrated. A skill another
card's training also hinted reads 0.95 of the model, a known miss smaller than the
inspiration's.

Removing both inspired skills for the whole run predicts a larger redistribution than observed. This comparison
does not exclude partial or time-dependent removal. The inspiration shortfall remains unmodelled. On card-runs where an inspiration hinted two pool skills, training hinted the rest at 0.365. With the
two kept in the pool the model predicts 0.337; with them removed, 0.454.

*Prioritised skills.* The model assumes a priority list does not change training hints. On the 3,817 runs that
show a list, 3859 rows hold a prioritised pool skill. Training hinted them at 0.345, against 0.347
predicted: no sign of a rise, from a weak test.

## 8. Chain events

Computed by `research/s08_chains.py`.

### Findings

- **The deck's pal cards raise the number of chain events** (**fitted**). $N \sim \mathrm{Binomial}(21, p)$ with
  $\operatorname{logit} p = a + b \times \text{pals}$, where group cards do not count as pals. $p$ runs from
  0.251 with no pals to 0.437 with five.
- **Each chain event goes to a running chain card, all equally likely** (**fitted**). With the pal-count budget this card
  rule predicts unseen runs better than weighting cards by their links left, the rule the model used before.
- **A chain that ends early frees its events for the other chain cards** (**supported**, conditional on the rest of
  the model). Every one of 8 synthetic pools generated with an ended
  card still taking its picks scores below the real runs; the real runs fall among the pools generated with ends
  freeing their events. No randomised test (the same deck with and without the ending prioritised) has been run.
- **No scenario effect and no Sasami Anshinzawa [This Might Sting!] effect is detectable on top of the budget**: the
  extra events of decks with her are accounted for by the pals they carry. Not detecting an effect does not show there
  is none; the tests' resolution is in the evidence below.
- **Known misses**: Inari One's careers reach fewer events than the budget predicts; early links run short at 3 pals and
  long at 1; chains that never start with five chain cards are slightly over-predicted. Causes unknown.

### Model

A run gets $N \sim \mathrm{Binomial}(21, p)$ chain events, $\operatorname{logit} p = a + b \times \text{pals}$ (**fitted**:
$a$ = -1.095, $b$ = +0.168). Pals are the deck's pal-flagged cards
other than group cards. Each event advances one chain card by one link, the card picked uniformly among the chain cards
still running. A chain is 3 links on an SSR and 2 on an SR, and a stopped or finished chain takes no more events. Pal
and group cards take no events.

A chain can stop early. If IT's default choice at a link ends the chain on some of its outcomes, the chain stops there
on a share 0.333 of that link's firings, the same for every
such link (**fitted**). A chain-ending choice the player prioritises stops it with the chance section 9's three-case
rule gives, read from each run's own priority list. A run showing no list takes 0.25 times the list-showing runs' share (**fitted** on a grid). The model enumerates the exact
distribution of final chain states (section 12).

| Pals | 0 | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| $p$ | 0.251 | 0.284 | 0.319 | 0.357 | 0.396 | 0.437 |
| mean $N$ | 5.27 | 5.96 | 6.70 | 7.49 | 8.31 | 9.17 |

### Evidence

*Reading a chain's progress.* Progress is read from marker skills, on 8,722 runs. A link is
observable only if every choice and outcome of its event carries a skill that no other event of the card gives. A link
is left out when another deck card's events can also give its marker, or when the marker can come from inspiration.
A marker can also appear without the link firing, from training hints and other sources: its background rate $\beta$ is
measured per skill and scenario on runs whose decks cannot give it, shrunk toward the trainee's own rate. A link is
then seen with chance $1 - \varepsilon(1 - \beta)$ if it fired and $\beta$ if it did not. Each fit is exact over the
chain states the run's deck can pass through.

*The marker allowance* $\varepsilon$ (**chosen**, inside its measured interval). The fits let a fired link's marker go
unseen with chance 0.002.
Links fire in order, so a later link seen on a clean marker ($\beta$ small) proves the earlier ones fired:

| Earlier links implied fired by a later one | Showing no marker | Share [95%] |
|---|---|---|
| 1,051 | 0 | 0.000 [0.000, 0.004] |

With none missing the upper limit counts each run as one trial. At $n = 21$ the best single $p$ moves from
0.325 to 0.330
over allowances 0 to 0.02.

*Which model predicts unseen runs?* Each form is fitted on four of five groups of runs and scored on the fifth; the
per-run held-out log-likelihoods are compared pairwise (*How to read*). Today's model is the one this section
replaces: $N$ fixed at Binomial(21, 0.35), events weighted by links left; its stop risks, end chances and backgrounds
are the same fitted ones as every other form's. The free distribution fits $P(N = 0..17)$ and $P(N \ge 18)$ (no deck has
more than 18 links). All forms but today's use the card rule unless named:

| Form | Over | Held-out nats ± SE | $z$ |
|---|---|---|---|
| pal-count budget | today's model | +352.5 ± 30.6 | +11.53 |
| pal-count budget | the free distribution | +188.7 ± 20.2 | +9.32 |
| pal-count budget | the same with links-left weighting | +157.5 ± 22.8 | +6.90 |
| pal-count budget | the same with an ended card still taking its picks | +26.7 ± 7.5 | +3.55 |
| pal-count budget, every nuisance (backgrounds, steering and no-list shares) fitted on the training folds | today's model | +353.5 ± 30.6 | +11.57 |
| the same | the same with an ended card still taking its picks | +26.8 ± 7.5 | +3.56 |
| free distribution | today's model | +163.8 ± 29.5 | +5.55 |
| binomial, $n$ and $p$ fitted | today's model | +152.8 ± 28.8 | +5.31 |

In-sample the budget's $b$ has likelihood-ratio $p$ = 4.2e-91 against
one $p$. With $N$ fixed at Binomial(21, 0.35), links-left weighting scores 240
nats above the card rule (a composite score, each link scored on its own); with the budget the order reverses (the
table's third row).

*Freed or wasted?* When a chain ends early (a prioritised chain-ending choice, or a default-choice stop), either its
card leaves the running cards and later events go to the others (freed, the model), or it keeps taking its share of
picks until its full length and shows none (wasted). Each hypothesis gets its own stop share and no-list scale (wasted:
0.400 and 0.30). A direct likelihood ratio
has no ready reference distribution here, so the test is a parametric bootstrap: synthetic pools of the same runs are
generated under each hypothesis, at its own constants and budget, and scored with the budget re-fitted.
The real runs' freed - wasted log-likelihood is +24.1 nats;
8 of the wasted pools (mean
-27.2) and
3 of the freed pools (mean
+28.8) score below it. Limits: each hypothesis keeps its
real-data stop share and no-list scale in every pool instead of re-fitting them, so the spreads leave out that
selection; with this many pools only the ranks are informative, not tail probabilities; the backgrounds and choice
shares are fitted on the whole pool; and the result holds only as far as the budget law, the card rule, the
backgrounds and the choice rule do.

*The form of $N$.* Among one-count forms, the free fit's $\min(N, 18)$ is under-dispersed (variance / mean
0.79; Poisson(7.1) is 182 nats
behind), and the binomial beats Conway-Maxwell-Poisson by 1.5
nats. Binomial(21, 0.35) is 378 nats behind the free fit
in-sample. The best single binomial has $n$ = 27 (profile set
23-34); $n$ and $p$ trade
off along a ridge, so the budget keeps $n = 21$ and lets $p$ carry the deck.

*Groups.* Each group's own logit term on top of the budget:

| Group | Runs | Own term | Likelihood-ratio $p$ | Mean pal count, group / other runs |
|---|---|---|---|---|
| decks with Sasami Anshinzawa [This Might Sting!] | 960 | +0.033 | 0.19 | 2.83 / 1.68 |
| Inari One's careers | 792 | -0.072 | 0.008 | 1.97 / 1.79 |

Without the budget, Sasami's decks fit $p$ = 0.351 against
0.305; with it, her own term is not distinguishable from zero.

*Scenarios.* No scenario effect is detectable: an intercept per scenario on the budget gains 2.1
nats for 3 parameters. A free distribution per scenario
gains 48 nats for 54, because the
scenarios' pal mixes differ.

*Stop share and no-list scale.* Each is profiled with the budget re-fitted at every grid value. The stop share
0.333 is the best grid value and the only one within 1.92 nats (0.4 is
2.0 behind, 0.25 14).
The no-list scale 0.25 is 0.0 nats below the grid's best; its 1.92-nat range is 0.20-0.30;
1, the list-showing share itself, is 56 nats behind.

*Final links by pal count.* The final link's marker against each model's chance of reaching it, with the background and
allowance applied:

| Pals | Chain cards | Runs | Observed [95%] | Final model | Today's model |
|---|---|---|---|---|---|
| 5 | 1 | 127 | 0.952 [0.899, 0.978] | 0.959 | 0.955 |
| 4 | 2 | 1,438 | 0.885 [0.868, 0.901] | 0.889 | 0.866 |
| 3 | 3 | 2,862 | 0.544 [0.530, 0.558] | 0.551 | 0.574 |
| 2 | 4 | 2,060 | 0.316 [0.305, 0.328] | 0.316 | 0.369 |
| 1 | 5 | 1,752 | 0.220 [0.211, 0.230] | 0.212 | 0.244 |
| 0 | 6 | 483 | 0.147 [0.132, 0.162] | 0.147 | 0.184 |

The final model lies outside the interval at 0 pal counts; today's model at
5.

*Known miss: early links.* An early link precedes the final link. The shortfall is the share predicted to fire minus
the share seen; each run's links form one cluster, $z = \sum(\text{pred} - \text{obs}) / \sqrt{\sum(\text{pred} - \text{obs})^2}$
over runs:

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Final model: shortfall | +0.006 | +0.022 | +0.013 | -0.040 | +0.004 |
| $z$ | +0.8 | +2.8 | +1.1 | -3.0 | +0.2 |
| Today's model: $z$ | +1.1 | +8.4 | +9.1 | +6.7 | +7.3 |

*Chains never started.* With five or six chain cards (zero or one pal), link 1 not reached:

| Chain cards | Observed [95%] | Final model | Today's model |
|---|---|---|---|
| 5 | 0.21 [0.18, 0.25] | 0.25 | 0.12 |
| 6 | 0.37 [0.31, 0.42] | 0.36 | 0.18 |

*Individual cards.* The 55 cards with 60 or more final-link observations, against the
final model, clustered by run since one run's chain cards share its $N$: pooled 0.369
observed against 0.368, $\chi^2$ =
76 (nominal $p$ = 0.03),
no card beyond $|z|$ = 2.6. Against today's model the same test gives
$\chi^2$ = 241.

## 9. Event choices and outcomes

Computed by `research/s09_outcomes.py` (the event-only rows and the certainty checks: `research/s13_calibration.py`).

Two rules are **assumed** (`docs/MODEL.md`). The *choice rule*: IT takes an event's top choice unless a prioritised
skill sits on another choice, and the player prioritises every target. The *equal rule*: a choice's outcomes are equally
likely unless measured, since master.mdb holds no outcome odds.

A target counts as *prioritised* when it is on the run's priority list and not outranked. It is outranked when a
higher-ranked prioritised skill sits on a rival choice of every event offering it. That affects 15 of section 13's
event-only rows. That the higher rank wins is assumed, not tested. The classifier also ignores a higher-ranked skill on
the target's own choice, which would cancel the outranking.

### Findings

- **IT follows the choice rule, in three cases** (table below). With a prioritised skill on one choice only, IT took
  that choice in every reading; with the skill already at hint Lv 5 from elsewhere, or a prioritised skill on the
  default choice too, less often.
- **The event-hint calibration by priority is in section 13.**
- **Four adopted outcome estimates replace equal shares**: Anshinzawa's finale and Seiun Sky's link 3 depart from the
  equal rule; Riko's Ramp Up is a per-run chance and Festa's rests on an attribution model. Some other outcomes are off
  the equal rule but left at it: known misses.
- **Pal and group finales are assumed to fire every time.** They fired in every checked run: an observed rate on a
  finite sample, not a game rule.

### The choice rule's three cases

A reading is a run showing its priority list, where one event's choice holds a prioritised skill and the records show
which choice IT took (every card event whose choices carry skills no other choice or deck card gives, and Anshinzawa's
finale through its hint levels). Most readings come from that finale; four are a chain-ending choice (Sweep Tosho
[Lamplit Training of a Witch-to-Be]), so carrying the rule over to chain endings is an extrapolation from steering in
general.

| Case | Readings where IT took the prioritised choice | Share [90%] |
|---|---|---|
| one-sided: only that choice holds a prioritised skill | 312 of 312 | 1.00 [0.99, 1.00] |
| maxed: the skill already at hint Lv 5 from another source | 16 of 28 | 0.57 [0.42, 0.71] |
| both: the default choice holds a prioritised skill too | 41 of 56 | 0.73 [0.63, 0.82] |

The maxed case reads the skill's level at the end of the run, a proxy for its level when the event fired. In the both
case no effect of the ranking is detectable (Riko's Lone Wolf above or below Maverick ○: intervals overlap). The model
takes the chain end with these shares in research (section 8); the planner assumes the player prioritises the ending
skill alone, the one-sided case.

### Measured shares

These shares are **fitted**. The table distinguishes observed counts from imputed ones. Festa's binomial interval
uses a partly imputed denominator. A *reading* is a firing where every outcome could show: the card's hint credited,
and each outcome's skill below level 5 from other sources.

| Event: outcome | Observed or attributed counts | Share [90%; Festa coverage unestablished] | Adopted |
|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] finale, Risky Business prioritised: Nothing Ventured | 281/373 | 0.75 [0.71, 0.79] | 0.75 |
| Riko Kashimoto [Planned Perfection] "Unexpected Side", top choice: Ramp Up, not Maverick ○ | 145/265 | 0.55 [0.50, 0.60] | 0.75 |
| Seiun Sky [Foolproof Plan] link 3: Fast-Paced, not Escape Artist | 79/118 | 0.67 [0.60, 0.74] | 0.65 |
| Smart Falcon [My Umadol Way! ☆] link 3: Center Stage, not Prudent Positioning | 40/74 | 0.54 [0.45, 0.63] | 0.5, the equal rule |
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante": Nothing Ventured, not Risky Business | 307 observed wins / 494.6 imputed trials | 0.62 [0.58, 0.66] | 0.60 |

A share counts only decks where no other card's events can give an outcome's marker, seen or not: where one can,
the record credits "A or B event" and cannot say which card fired. Smart Falcon's share is consistent with the equal
rule, which the model now uses for it (its earlier 0.65 rested on readings that dropped such shared credits for one
outcome only). Riko Kashimoto's top-choice row counts each credit as a firing; Maverick ○ also comes from other events
(credited to them in 151 of 643
runs whose deck cannot give it), so counting a run that credits both as one Ramp Up firing gives
141 of 197 instead. Whether her top choice departs
from the equal rule depends on that reading. Her adopted value is a per-run chance, set out under *Evidence*.
Nakayama Festa's share rests on an attribution model; its interval has unestablished coverage. Every count uses the
full pool.

### Evidence

*Does IT follow the choice rule?* Where the runs show the choice taken, yes:

| Event | Condition | Choice taken |
|---|---|---|
| Sasami Anshinzawa's finale | Risky Business prioritised | choice 2 in about 92% of runs |
| Sasami Anshinzawa's finale | neither skill prioritised | identifiable choice-two wins in 0 of 243 runs |
| her date-start event "A Pleasant Practice Partner?", read by its extra Risky Business +1 | Risky Business prioritised | bottom choice in 252 of 254 runs winning finale choice 2 |
| the same | Risky Business not prioritised | bottom choice in 1 of 48 such runs |
| Riko Kashimoto [Planned Perfection]'s event | Lone Wolf prioritised | bottom choice in 370 of 534 runs |

Of the 27 finale runs that took choice 1 with Risky Business prioritised, 16 had it already at hint Lv 5.
The records show a priority list for only 3,817 of 8,940 runs. The rest count as unknown, never as an empty
list, so the rule is tested mainly through its consequences in section 13.

*Reading Sasami Anshinzawa's finale.* Nakayama Festa [Trusting the Dice]'s "Upping the Ante" also gives Nothing
Ventured and shares its credits. So these rows use the 929 runs with Anshinzawa's card and without Festa's.
Hint levels identify the choice:

- her Nothing Ventured +3 is choice 1, which otherwise gives nothing, and +1 is choice 2;
- a loss on choice 2 shows as her Risky Business at level 2, from storyline choice 3 and the finale.

GameTora marks the date-start event's bottom choice as ending her event chain, which for a pal card means her dates.
Yet with Risky Business prioritised the finale shows its outcome in at least 353 of 373 runs. IT
apparently keeps her dates either way.

*Riko Kashimoto's adopted share.* The adopted value is the per-run chance of her Ramp Up hint. A run counts where
no other deck card's events can give Ramp Up and its level from other sources stays below 5. On runs whose shown
list prioritises Ramp Up and not Lone Wolf, as the model assumes of a target, it is
0.741 [0.714,
0.765], which covers the adopted
0.75. A prioritised Lone Wolf takes her bottom choice
(0.24), so the pooled chance is lower:

| Riko Kashimoto [Planned Perfection] | Count | Share [90%] |
|---|---|---|
| runs with her Ramp Up hint | 2586 of 4146 | 0.624 [0.611, 0.636] |
| on the 252 runs whose deck has no other card that can give any of her three skills, firings where IT took the top choice; the rest took the bottom choice, Lone Wolf | 264 of 319 | 0.83 |
| runs with her Ramp Up hint, list shown with Ramp Up prioritised and not Lone Wolf | 591 of 798 | 0.741 [0.714, 0.765] |

*Nakayama Festa's share.* It starts from the 1141 runs with her card, less her own careers and decks with
Sasami Anshinzawa's SSR. The credits there:

| Credit | Runs |
|---|---|
| to her alone, Nothing Ventured | 307 |
| to her alone, Risky Business | 140 |
| Risky Business, to "her or another card's event" | 77, Team Sirius in 53 |
| neither outcome, with Risky Business already at Lv 5 | 77 |

A shared credit is split by estimated source rates. Let $r_F$ be her exclusive Risky Business credit rate in runs with
her card, 0.128. Let $r_o$ be the other card's exclusive rate in runs without her: 0.887 for Team Sirius,
0.470 for Sasami Anshinzawa. A shared credit goes to her with weight $r_F / (1 - (1 - r_F)(1 - r_o))$, an attribution weight obtained by substituting exclusive-credit rates into an independent-source formula.
It is not an identified conditional firing probability. That gives her 13.2 of them.

In a blank run either a Risky Business win could not show or the event never fired. The blank runs are split by the
rate at which the event shows where both outcomes can, 0.45. Whatever part of the shared credits is hers, the share
stays within 0.545-0.639.

So the share rests on an attribution model. Its rates are exclusive credit rates, not firing rates, and it assumes the
sources fire independently. The blank-run split ignores which outcome is missing. The interval rounds the imputed denominator and treats it as a binomial trial count. It omits attribution
uncertainty, so its coverage is unestablished. No joint observation model is
fitted or tested.

*Shares left at the equal rule.*

| Event: outcome | Share | Verdict |
|---|---|---|
| Tazuna Hayakawa [Tracen Reception] finale: Concentration | 0.62 | off 0.5: known miss |
| Agnes Tachyon [Q≠0] link 1 choice 2: Murmur | 0.62 | off 0.5: known miss |
| Marvelous Sunday [Dazzling Day in the Snow] link 3: Position Pilfer | 0.57 | off 0.5: known miss |
| Light Hello [From the Ground Up] finale: See Ya Later! +3 | 0.70: 4,683 of 6,661 | off 0.5: known miss |
| Gold Ship [That Time I Became the Strongest] link 3 | 0.59 | off 0.5: known miss |
| King Halo [Tonight, We Waltz] link 3 | 0.54 | consistent with 0.5 |
| Daitaku Helios [Sunlit Angel ♪] link 2, where link 3 visibly fired: Tail Held High | 0.51 | consistent with 0.5 |

*Pal and group finales.* The checks use runs where the card is the deck's one source of the skill and no inspiration
hinted it. Light Hello [From the Ground Up]'s event is credited with See Ya Later! in 2163 of 2163 such runs.
Team Sirius is credited with Risky Business, alone or shared, in 107 of 107. These observed rates are
consistent with a finale that always fires; finite samples do not establish it.

## 10. Random card events (not modelled)

Computed by `research/s10_random_events.py`.

A deck card's character has *shared random events* that can fire in another uma's career; any rarity counts, R cards
included. $K$ is the number on offer: every shared event of the deck's characters plus the trainee's own. A *reading*
is one such event in one run, read through the marker skill on its top choice.

### Findings

- **Leaving them out under-predicts the targets they can hint**, a known miss: on real careers those targets read
  1.24 of the model. Credited at the fitted rate, they read 0.99.
- **Two default targets are affected**: Vodka's Nimble Navigator and Nice Nature's Ramp Up, the only ones that are a
  character's top-choice marker.
- **A shared random event fires in about 0.15 of the careers it can fire in** (**fitted**), a lower bound. The
  rate falls as more events are on offer, and the $K$ form predicts unseen runs better than a constant
  ($z$ = +2.21).
- **Known miss of the measured rate: it differs by scenario**, highest in Unity Cup and Trackblazer, next to $K$ and
  next to the card effects. It changes no prediction, since the model leaves these events out.
- **Crediting them has no clear held-out advantage** on the rows the model gives a positive chance.
- **Known miss of the measured rate: the card's initial friendship gauge raises it.** It changes no prediction, since
  the model leaves these events out.

### Evidence

*The rate.* A hit is a top-choice marker credited to the character's event. An *ambiguous credit* also names another
deck card whose events can give the marker; as in section 8, it is left out as unknown. The 10,977 readings come
from 6,181 runs. They give 1,647 hits, with 730 ambiguous credits left out.

A firing goes unseen when IT takes another choice for a prioritised skill, or when the marker is already at Lv 5 from
other sources. So the rate is a lower bound. Counting ambiguous credits as hits, over the readings where the event
could show, gives 0.206.

The *$K$ form* lowers the rate by a fixed fraction for each extra event on offer. The card effects and scenario are
fitted beside $K$ in a logistic model.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| The rate falls as more events are on offer | $K$ form: 0.21 at $K$ = 4, 0.12 at 14 | run-clustered Wald $\lvert z \rvert$ = 5.59 | detectable |
| The $K$ form predicts unseen runs better than a constant | constant 0.150 | held out +12.0 ± 5.4 nats, $z$ = +2.21 | yes |
| The initial friendship gauge raises the rate | +0.126 [+0.074, +0.178] logit per 10 points | Holm-adjusted run-clustered $p$ = 0.0000 | known miss |
| Friendship bonus moves it | −0.032 logit per 10 points | Holm-adjusted $p$ = 0.88 | no effect |
| Specialty priority moves it | +0.009 logit per 10 points | Holm-adjusted $p$ = 0.88 | no effect |
| Scenario moves it | Unity Cup +0.18, Trackblazer +0.24 logit against URA Finale | run-clustered $p$ = 0.026; next to the card effects 0.005 | known miss |
| Rarity moves it within a character | | run-clustered $z$ = +1.54 | no effect: R and SR cards pooled with SSR |

Pairs of one run's readings that both hit are as frequent as independent firing predicts. Independent firing remains
an assumption.

*What does leaving them out cost on real careers?* The test uses section 13's rows, each on the run's own deck. It
scores whether the target was acquired from any source, so it does not depend on reading the event. Both baseline
and credited acquisition use a hybrid reconstruction: exact landing marginal $p$ and the report's approximate gold
chance $g$. For an unguaranteed target, it reconstructs acquisition as $p/W-g$, with $W$ the white-spark generation
factor from section 12. The baseline therefore includes the difference between exact and approximate gold chances.
These comparisons are neither exact acquisition calibration nor a consistent test of the report approximation.
Crediting uses the
$K$ form at each run's $K$, about 0.15 on these rows.

| Rows where a deck card's random event could hint the target | Count |
|---|---|
| all | 312 |
| target credited to that event | 37 |
| the same, with no other source | 24 |
| rows the model gives no chance | 73 |
| of those, hinted by the event | 6 |

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Left out, these targets are under-predicted | 1.24 [1.11, 1.38] | interval excludes 1 | known miss |
| Credited at the fitted rate, they are not | 0.99 [0.89, 1.10] | interval covers 1 | consistent |
| Crediting predicts unseen runs better | | held out +5.2 ± 2.9 nats, $z$ = +1.78 | not shown |

The held-out score refits the rate on the other folds' readings. It covers only the 239 rows with a positive
baseline prediction: a row the model gives no chance has no finite likelihood when acquired. So it leaves out the rows
where an omitted source matters most, and it cannot settle the value of adding the source.

Crediting raises the product of positive per-target landing marginals by a median
14%. The largest rise, over 239 affected runs, is 162%. This uses the hybrid reconstruction above. It does not recompute exact all-target odds.

## 11. Conditional character events (not modelled)

Computed by `research/s11_conditional.py`.

The trainee's secret and race-result events are left out of the model, except Tokai Teio's Head-On. A
*conditional-only row* is a run-target row where only such an event can give the target: none of the trainee's story,
date, outfit or shared random events can, nor the scenario's.

### Findings

- **The sparse pooled conditional-only comparison is inconclusive.** Its score excludes zero-baseline rows,
  including 3 acquisitions. The 112 rows
  do not settle trainee-specific costs.
- **Known miss: Vodka's own Nimble Navigator.** Her events give it in most of her runs, and the model under-predicts
  it. Her random and conditional events both give it and cannot be told apart.
- **Tokai Teio's Head-On is modelled as always acquired**, like Yamanin Zephyr's Slipstream. All three of its races
  are her career goals. The goals ask only for top 5, top 5 and top 3, while the event needs wins, so certainty is
  **assumed** from observed wins: IT won all three in 80 of 80 runs training her. Every run where her
  event could show credits Head-On.

### Evidence

| Rows | Her events gave the target | Rate [95%] |
|---|---|---|
| all conditional-only rows | 10 of 112 | 0.089 [0.049, 0.157] |
| Daiwa Scarlet, any outfit, Nimble Navigator | 0 of 99 | [0.00, 0.04] |
| Fine Motion, any outfit, Ramp Up | 1 of 4 | [0.05, 0.70] |
| Mejiro Ardan, any outfit, Ramp Up | 9 of 9 | [0.70, 1.00] |
| Vodka, any outfit, Nimble Navigator, not conditional-only | 28 of 30 | [0.79, 0.98] |

| Rows, model as it is | Acquisition, observed / predicted [95%] | Crediting the measured rates gains |
|---|---|---|
| conditional-only | 1.14 [0.90, 1.37] | 2.0 nats |
| with Vodka's rows | 1.34 [1.16, 1.53] | 21.0 nats |

Both sides use section 10's hybrid acquisition reconstruction, mixing exact landing marginals with approximate
gold chances. Each gain is in-sample and is set against half the $\chi^2$ 95% point for one degree of freedom per fitted rate: the
first falls short, the second exceeds it. As in section 10, the gains count only rows with a positive baseline
prediction. That leaves out 28 conditional-only rows the model gives no chance, and her events
hinted the target on 3 of them.

## 12. The composed career model

The composition is **derived** from sections 1-11 under the independence assumptions stated here; nothing further is
fitted and no script computes it. An independent implementation, `val/joint_check.py`, reproduces the solver's exact
odds on every reference loop (section 6).

A target lands if the deck hints the gold and it sparks, or else if any source hints the white and that sparks. For a
target $k$ not guaranteed:

$$p_k = (1 - m^{g}_k)\,G_k + W_k\left[m^{g}_k - m^{0}_k(1 - e_k)(1 - q_k)\right].$$

$G_k = 0.4\cdot 1.1^{n_k}$ and $W_k = 0.2\cdot 1.1^{n_k}$ are the gold and white spark chances, with $n_k$ ancestors
carrying the target (section 1). $m^g_k$ is the chance the deck gives no gold hint, and $m^0_k$ the chance it gives no
hint of either tier, each joint over the deck's sources. $e_k$ is the chance the trainee's own events hint the white,
and $q_k$ the chance it is inherited (section 3). The bracket is the chance the white is hinted without the gold. Its
miss product assumes the deck, own events and inheritance independent given the model inputs. Inheritance only ever
supplies the white.

A white on the trainee's outfit replaces the bracket by $m^g_k$. A gold on her outfit, or one the scenario guarantees,
gives $p_k = G_k$.

Targets are not independent. Chain cards share one run's chain-event count $N$ (section 8), a card's training draws
are joint (section 7), and one event's outcomes exclude each other (section 9). So the model conditions on the final
chain state $c$, which records where each chain stopped, and on the joint training and deck-event outcomes $H$. Given
$c$, $H$ and the lineage inputs, the remaining inheritance, own-event and generation outcomes are assumed independent
across targets, so the per-target chances multiply:

$$P(\text{all}) = \sum_{c} P(c)\, E_{H\mid c}\!\left[\prod_{k} p_k(c,H)\right] \qquad (\text{exact under the assumptions}).$$

This also assumes training independent across cards, and event-outcome groups independent given chain progress.
Multiplying the targets' marginal chances given $c$ alone would discard the dependence through $H$. The implementation
enumerates connected groups of targets and multiplies independent groups (`joint_eval_in`).

*Real legacies.* The same model takes each veteran's real stars, G1 wins and recorded parents. An unknown parent brings
no relation points, but her sparks count. Since $e_k$ and $q_k$ enter only through $(1 - e_k)(1 - q_k)$, the real
lineage's inheritance is folded in by replacing $e_k$ with $1 - (1 - e_k)(1 - q_k)$. $n_k$ becomes the number of
legacies actually carrying $k$, so against the loop's $n_k = 6$ target $k$'s landing factor scales by $1.1^{n_k - 6}$.
The total scale is $\prod_k 1.1^{n_k - 6} = 1.1^{\sum_k(n_k - 6)}$, guaranteed targets included (**derived**).

## 13. Calibration against real careers

Computed by `research/s13_calibration.py`.

### Findings

**The checks assess calibration on recorded careers, not candidate rankings.** The table below reports the ratios
by source and priority. Acquisition uses the report's product approximation. Unknown priorities remain a separate
cohort. Agreement in aggregate can hide component errors and failures within prediction bands.

**The spark-weighted all-target check is weak.** It does not measure spark generation. The comparisons beyond the
default targets test a broader skill sample under the same restrictions.

### What is compared

Each real career is rebuilt as the model sees one: the trainee in her outfit, the scenario, the six cards at their own
limit breaks, both parents with their recorded parents, every ancestor's white sparks, and race wins giving the
recorded shared-G1 counts. Each chain-ending choice follows the run's priority list where it shows one: a certain end
is pinned as a stop at its link, an impossible one is removed, and a fractional one, like every run showing no list, is
left to the solver, which may take the end as a player following its advice would. The model gives each default target's chance at each step (sections 3-12), and the run
record shows what happened.

| Careers | URA Finale | Unity Cup | Grand Concert | Trackblazer | Run-target rows | On her outfit | Scenario golds |
|---|---|---|---|---|---|---|---|
| 8,373 | 1001 | 1,695 | 5,564 | 113 | 66,984 | 3,329 | 3,390 |

Rows on her outfit and scenario golds are *certain*, so tests 1 and 3 leave them out; the rest are *open targets*.
Unity Cup's guaranteed golds show in 1,695 of its 1,695 runs. The runs end before skills are bought, so
"acquired" means hinted, the model's own meaning.

Tests 1 and 3 score the career report's `buy` product approximation, which multiplies chances across cards, not the
exact joint engine of section 12; shared chain progress makes the two differ. The landing and all-targets checks use
the exact engine's $p_k$ and $P(\text{all})$.

1. **Deck hint**: does a deck card hint the skill, by training or by its events, as often as sections 7-9 predict? The
   prediction is $d^g + (1 - d^g)d^w$, with $d^g$ the gold chance and $d^w$ the white chance given no gold. Only rows
   where no inspiration hinted the skill count, which treats inspiration as independent of the deck (section 7). The
   rows split by where the deck's chance comes from, **training pool only** or **events only**, and events only by
   whether the run's list shows the target **prioritised** (section 9).
2. **Inheritance**: does an inspiration of the skill fire as often as $q$ (section 3)?
3. **Acquired**: does the skill end up hinted, from any source, as often as all modelled sources together predict?
   This is what the odds rest on. The **prioritised or unknown** rows pool prioritised targets with the runs that show
   no list, whose priorities are unknown.

| Test | Rows | Predicted | Obs / pred [95%] | Verdict |
|---|---|---|---|---|
| 1. Deck hint | 38,605 | 0.259 | 0.946 [0.936, 0.956] | optimistic |
| training pool only | 7,239 | 0.358 | 0.994 [0.965, 1.022] | covers 1 |
| events only | 5,689 | 0.871 | 0.902 [0.892, 0.913] | optimistic |
| events only, target prioritised | 1141 | 0.833 | 0.997 [0.975, 1.0188] | covers 1 |
| events only, not prioritised | 1670 | 0.867 | 0.776 [0.753, 0.799] | far below: the priority assumption fails |
| 2. Inheritance | 66,984 | 0.374 | 0.990 [0.982, 0.997] | below 1 through Tail Held High |
| without Tail Held High | 58,611 | | 0.993 [0.985, 1.000] | covers 1 |
| Tail Held High | | | 0.972 [0.947, 0.997] | known miss |
| 3. Acquired, any source | 60,265 | 0.528 | 0.990 [0.985, 0.995] | slightly optimistic |
| prioritised or unknown | 45,134 | 0.529 | 1.002 [0.996, 1.008] | covers 1 |
| prioritised | 10,404 | 0.666 | 1.014 [1.005, 1.024] | slightly above 1: known miss |
| not prioritised | 15,131 | 0.526 | 0.953 [0.943, 0.963] | optimistic |
| unknown priority | 34,730 | 0.488 | 0.997 [0.989, 1.005] | covers unity |

### Evidence

*Acquisition reliability.* The product approximation misses acquisitions in its zero-prediction bucket.
Positive bands also show variation hidden by the pooled ratio. These are checks of the acquisition approximation
on open targets, not the exact engine. Each positive band's observed rate is followed by its mean prediction.

| Predicted acquisition | Rows, observed (mean prediction) |
|---|---|
| 0 | 13,813, 0.009 |
| above 0 to 0.20 | 2,807, 0.159 (0.16) |
| 0.20-0.40 | 6,167, 0.307 (0.30) |
| 0.40-0.60 | 6,644, 0.507 (0.50) |
| 0.60-0.80 | 11,662, 0.699 (0.72) |
| 0.80-0.95 | 9,135, 0.842 (0.86) |
| 0.95-1 | 10,037, 0.981 (1.00) |

*Where the deck loses its points.* The deck's rows predicted 0.6 or more hold 98% of its shortfall,
and 95% of those rows have an event source. On prioritised-or-unknown rows, the deck and inspiration alone
read 0.988 of the model. Unmodelled scenario and trainee events close the gap to the any-source row: they gave
376 target acquisitions, 0.6% of open rows. Agreement after adding them can hide errors in the parts, and it
does not validate candidate rankings.

*Landing.* Each open row's spark chance is recomputed from what the run acquired: $0.4 \cdot 1.1^n$ if the gold was
hinted, $0.2 \cdot 1.1^n$ if only the white. Summed, it reads 0.990 of the model's $p$, in line with acquisition. It
does not test spark generation (section 1).

*Per target*, observed / predicted:

| Target | Deck hint [95%] | Inheritance | Acquired |
|---|---|---|---|
| Uma Stan / Superstan | 0.89 [0.81, 0.98] | 0.991 | 0.985 |
| Nimble Navigator / No Stopping Me! | 1.04 [0.96, 1.13] | 0.999 | 1.044 |
| Playtime's Over! / See Ya Later! | 0.993 [0.98, 1.01] | 1.003 | 0.996 |
| Ramp Up / It's On! | 0.863 [0.84, 0.89] | 0.990 | 0.929 [0.91, 0.95] |
| Tail Held High / Tail Nine | 1.017 [0.97, 1.06] | 0.972 [0.95, 1.00] | 1.002 |
| Racing Spirit: Stamina | no deck source | 0.992 | 1.025 |
| Slipstream / Perfect Spot! | 0.980 [0.92, 1.04] | 0.983 | 0.989 |
| Risky Business / Nothing Ventured | 0.873 [0.83, 0.92] | 0.946 | 0.926 [0.89, 0.96] |

*Gold acquisition*, observed / predicted:

| Gold | Rows | Observed / predicted [95%] |
|---|---|---|
| Nothing Ventured | 1,963 | 0.923 [0.88, 0.97] |
| Perfect Spot! | 1,168 | 0.920 [0.87, 0.97] |
| It's On!, outside Unity Cup | 649 | 0.895 [0.81, 0.98] |

*Training-pool hints by scenario* test $\beta$ and $d$ (section 7). Only Trackblazer's interval excludes 1: its $\beta$
over-predicts these runs. Split instead by the lowest limit break of the sourcing cards, the LB0, LB1 and LB2 intervals
each cover 1.

| Scenario | Grand Concert | Unity Cup | URA Finale | Trackblazer |
|---|---|---|---|---|
| Observed / predicted [95%] | 1.011 [0.98, 1.04] | 0.931 [0.85, 1.01] | 1.014 [0.92, 1.10] | 0.746 [0.60, 0.89] |

*Every open target in one run.* The share of runs acquiring all their open targets matches the mean product of their
per-target predictions:

| Runs | Product of the model's odds | Observed [95%] |
|---|---|---|
| all | 0.0461 | 0.0426 [0.0385, 0.0472] |
| 4,927 with every open target prioritised or unknown | 0.0289 | 0.0276 [0.0234, 0.0326] |

*Spark-weighted acquisition of all targets.* $P(\text{all eight})$ is recomputed from what each run acquired and
compared with the model's exact $P(\text{all})$; Results gives the ratio, its interval bootstrapping whole runs. The
shortfall sits in the runs showing no priority list, where the model assumes every target prioritised:
0.78 [0.59, 0.97];
runs showing a list read 0.89 [0.78, 1.00], which covers 1. Only
357 runs acquired every open target, too few to separate the exact joint from the product.

*Single cards.* Where one card is the deck's only source of a target, the deck test scores that card alone. The table
lists every such pair of 40 or more rows whose interval excludes 1.

| Card > target | Rows | Predicted | Obs / pred [95%] | Likely reason |
|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | 628 | 1.000 | 0.76 [0.73, 0.80] | the model assumes Risky Business prioritised, which takes finale choice 2 |
| Riko Kashimoto [Planned Perfection] > Ramp Up | 1576 | 0.750 | 0.75 [0.71, 0.78] | open; her Lone Wolf choice, when prioritised, accounts for part |
| King Halo [Tonight, We Waltz] > Uma Stan | 189 | 0.870 | 0.84 [0.76, 0.91] | Uma Stan sits on a choice that needs a priority |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | 83 | 0.308 | 0.63 [0.40, 0.94] | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | 182 | 0.332 | 1.21 [1.00, 1.43] | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | 581 | 0.708 | 0.93 [0.87, 0.98] | open |
| Tokai Teio [Step! Smile! Wink!] > Nimble Navigator | 155 | 0.418 | 1.30 [1.11, 1.48] | open |
| Yukino Bijin [Dancing Light into the Night] > Playtime's Over! | 42 | 0.155 | 0.15 [0.03, 0.79] | the skill sits on a chain-ending choice 2, taken only when prioritised |

*King Halo's Uma Stan follows the priority.* The expectation and the exact Poisson-binomial tail are the model's:

| Runs that list Uma Stan | Rows | Hinted by her card | Expected | $P(X \le \text{hinted})$ |
|---|---|---|---|---|
| prioritised | 74 | 64 | 66.7 | 0.180, consistent |
| a list without it | 25 | 0 | 23.0 | |

*Smaller pairs.* Below 40 rows, 4 more pairs exclude 1. 2 of them are
chain-ending event sources of Playtime's Over! (its three are Yukino Bijin [Dancing Light into the Night], Yaeno Muteki
[Fiery Discipline] and Sirius Symboli [Escorte Étoile]; one with 40 or more rows sits in the table above). They hinted
it in 4 of 49 runs, where the model predicted at least 0.29. Their event puts the skill on
choice 2, which ends the chain before its last link, so IT takes it only when the skill is prioritised. Reading each
run's list accounts for the miss on runs that show one: prioritised rows match the model, and rows whose list leaves it
out are predicted none and show none. Runs showing no list stay below the model, which there assumes the end can be
taken.

*Other skills.* Every non-target skill a deck card or spark could give, on a 30% sample of 2,515 runs:

| Test | Rows | Observed / predicted [95%] | Verdict |
|---|---|---|---|
| inheritance | 104,219 | 0.979 [0.969, 0.990] | slightly short: known miss; no one skill carries more than 5% of it |
| training-pool hints | 39,123 | 1.038 [1.028, 1.047] | under-predicted: known miss |
