# The model's mathematics

What this project measured, fitted, tested or derived for its model of how a career in Uma Musume's Independent
Training acquires and passes on skills. Every constant the model uses, with its source, is in `docs/MODEL.md`; this
document keeps the evidence.

## Results

On 5,198 real careers, each rebuilt from its own deck and lineage, the acquisition product approximation
has no detected pooled bias on known-prioritised and unknown-priority rows, reported separately in section 13.
Over all careers it is slightly optimistic.
Terms, labels and the limits of every test are in *How to read*.

| § | Conclusion | How it is set | Main evidence |
|---|---|---|---|
| 2 | Grand Concert defaults to the 17,500+ star mix; URA Finale, Unity Cup and Trackblazer to the lower one | default chosen from rank scores | Grand Concert: 79% of careers reach 17,500+; the largest relative overstatement from averaging stars was 0.57% on sampled affinities under the assumed independent-slot star mixes |
| 3 | Grandparents pass sparks at the parents' full base rate | fitted | weight 1.00 [0.98, 1.02] against the parents' 1 |
| 3 | The community affinity formula is kept, with +3 per shared G1 win | assumed, tested | held-out gains do not establish improvements over unit weights: slope $z$ = 0.86, shared wins $z$ = 0.81 |
| 4 | The community win table holds on grades raised by pink sparks, with a goal race charged the streak position before it | community table, tested; goal shift fitted | wins 0.998 [0.996, 0.999] of predicted, over 119,188 G1 attempts |
| 5 | A trainee who is her own grandparent gets no trio points from it | fitted | 29 nats better than counting the pair's points |
| 6 | Scheduling loops for total shared wins costs little | derived | with each loop's decks fixed, no schedule raises a reference loop's odds by more than 2.01% |
| 7 | A card's training hints are uniform draws from its pool, a number per scenario raised by Hint Frequency and Specialty Priority | form fixed on the runs; base and specialty weight fitted | real careers' training-pool hints: 0.994 [0.96, 1.03] of the model |
| 8 | The model uses Binomial(21, 0.35) chain events, allocated by remaining links | chosen binomial; allocation informed by fits | no alternative predicts unseen runs clearly better; remaining links score 102 nats above equal shares per card without marker allowance |
| 9 | IT takes a prioritised skill's choice; five outcome estimates replace equal shares | rule assumed, tested on careers; shares fitted | prioritised targets' event hints read just below the model, a known miss |
| 10 | Random card events are left out | not modelled | the targets they can hint read 1.24 of the model |
| 11 | Conditional character events are left out | not modelled | sparse pooled comparison inconclusive; Vodka's own Nimble Navigator is a known miss |
| 12 | The parts compose into an exact $P(\text{all})$ under stated independence assumptions | derived | an independent implementation reproduces the solver's odds on every reference loop |
| 13 | The acquisition product approximation has no detected pooled bias on known-prioritised and unknown-priority rows | tested | 0.998 [0.990, 1.005] on 32,848 pooled prioritised-or-unknown rows; the groups are reported separately in section 13 |
| 13 | Over all rows the model is slightly optimistic, mostly on targets shown not prioritised | tested | all rows 0.986 [0.980, 0.993]; targets shown not prioritised 0.911 [0.893, 0.929], 85% of the shortfall; the cause is not established |
| 13 | $P(\text{all})$ itself is checked only weakly | tested | spark-weighted all-target acquisition 0.93 [0.80, 1.05]; only 220 runs acquired every target; tests neither spark generation nor rankings |

### Known misses

Each is measured in its section. *How to read* says why a listed miss can be partly chance.

| Part | Miss | Observed / predicted | Section |
|---|---|---|---|
| Inheritance | Tail Held High is inherited less often than predicted; cause unknown | 0.953 [0.918, 0.988] | 3, 13 |
| Inheritance | parents at affinity 200 or more pass sparks less often | 0.966 [0.945, 0.987] | 3 |
| Inheritance | non-target skills, the shortfall spread over many skills | 0.986 [0.973, 0.999] | 13 |
| Races | aptitude basis <=70, before streak penalties | 0.95 [0.93, 0.97] | 4 |
| Races | chosen races 1st and 2nd in a row | 0.996 [0.994, 0.998]; 0.997 [0.994, 0.999] | 4 |
| Races | goal races 7th or later in a row, which the planner charges as the 6th | 0.68 [0.51, 0.84] | 4 |
| Training hints | cards differ beyond their effects; cause open | per card 0.80-1.17 | 7 |
| Training hints | a skill also hinted by inspiration | 0.73 | 7 |
| Training hints | more pals per deck give slightly more hints | factor 1.028 per pal | 7 |
| Training hints | Trackblazer's base over-predicts real careers | 0.779 [0.63, 0.93] | 13 |
| Training hints | non-target skills are under-predicted | 1.056 [1.045, 1.068] | 13 |
| Chain events | Binomial(21, 0.35) fits worse than the free distribution in-sample | 21.9 nats behind; nominal LR reference needs calibration | 8 |
| Chain events | decks with Sasami Anshinzawa [This Might Sting!]'s pal card have a higher fitted capped count | mean capped count 7.74 against 7.17 | 8 |
| Chain events | Inari One's careers have a lower fitted capped count | mean capped count 6.83 against 7.26 | 8 |
| Chain events | with six chain cards, absent first-link markers exceed predicted unstarted chains; marker allowance omitted | 0.34 against 0.20 | 8 |
| Chain events | individual cards' final-link markers differ from latent predictions; marker allowance omitted | $\chi^2$ = 386 on 50 cards | 8 |
| Event choices | prioritised targets' event hints read just below the model | 0.970 [0.940, 0.9997] | 9, 13 |
| Event choices | four outcome shares the model leaves at the equal 0.5 read off it: Tazuna Hayakawa [Tracen Reception]'s finale, Agnes Tachyon [Q≠0] link 1, Marvelous Sunday [Dazzling Day in the Snow] link 3, Light Hello [From the Ground Up]'s finale | 0.70, 0.70, 0.58, 0.71 | 9 |
| Left out | targets a deck card's random event can hint | 1.24 [1.11, 1.38] | 10 |
| Left out | Vodka's own Nimble Navigator | her events give it in 25 of 27 runs | 11 |

### Not settled

- Whether specialty priority causes more hints. Fitted on earlier runs, it predicts later ones better by
  10.1 nats, but on cards whose limit breaks change only their specialty priority, no
  effect fits as well (section 7).
- The hint-frequency slope $1/100$ (**assumed**). On the adopted model a free slope does not predict unseen runs
  better; without the specialty term one does (section 7).

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
5,527 lineages in the runs. It treats each legacy's stars as independent of the others'; a veteran
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
spark fires. There are 44,010 parent trials and 106,738 grandparent trials of ordinary ancestors; a trainee's own
character as a grandparent is section 5's.

### Findings

- **Grandparents pass sparks at the full base rate, as parents do.**
- **Affinity scales the roll as the formula says.** Held-out gains for a free slope on affinity or either part
  are too uncertain to establish an improvement over unit weights.
- **Each star count's base matches its trials.**
- **Known miss: parents at affinity 200 or more pass sparks less often than predicted.** The bands below 200 are
  jointly consistent. The held-out gain from a refitted slope is inconclusive. The model keeps the formula and this
  miss.
- **Whole careers bear out $q$ apart from Tail Held High**, whose miss and the totals are in section 13. One band of
  predicted $q$ lies outside its pointwise interval; the joint test does not reject the model.

### Evidence

"Held out" is rule 3 of *How to read*: five folds by run, positive nats favouring the refit. Each part's weight is
fitted with the other part at 1. The carrier and spark-year checks exclude Tail Held High, leaving
36,386 rows. The predicted-$q$ band check uses all
41,584 rows.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Grandparents take the full base | weight 1.00 [0.98, 1.02] | full base beats half by 1,975 nats | full base |
| Affinity raises the roll | | beats no affinity by 5,427 nats | kept |
| Affinity slope is 1 | 0.97 [0.95, 0.99] | held out 2.1 ± 2.4 nats, $z$ = 0.86 | 1 kept |
| Relation-point weight is 1 | 0.92 [0.84, 0.99] | held out 1.8 ± 2.2 nats, $z$ = 0.85 | 1 kept |
| Weight of 3 per shared G1 win is 1 | 0.96 [0.92, 0.99] | held out 2.0 ± 2.5 nats, $z$ = 0.81 | 1 kept |
| Affinity bands below 200 fit | | $\chi^2$ = 2.5, $p$ = 0.65 over four bands | consistent |
| All five affinity bands fit | | $\chi^2$ = 12.6, $p$ = 0.027 | known miss at 200+ |
| Careers with 1 to 6 carriers | 0.965-1.000 of the model | every interval covers 1 | consistent |
| Years with an inspiration of the skill | 0.989 of the model | 1.84 sd below | consistent |
| $q$ by predicted band | | $\chi^2$ = 9.2, $p$ = 0.10 over the 5 bands above 0 | consistent |

*By stars and by affinity.* Of these groups, only affinity 200+ lies outside its own interval, and
every trial there is a parent's.

| Group | Trials | Observed / predicted [95%] |
|---|---|---|
| 1★ | 41,970 | 0.985 [0.95, 1.02] |
| 2★ | 96,080 | 0.987 [0.97, 1.00] |
| 3★ | 12,698 | 0.995 [0.96, 1.03] |
| affinity 0-49 | 12,628 | 0.980 [0.919, 1.042] |
| affinity 50-99 | 94,696 | 0.999 [0.980, 1.019] |
| affinity 100-149 | 1,428 | 1.025 [0.899, 1.152] |
| affinity 150-199 | 5,316 | 1.045 [0.982, 1.107] |
| affinity 200+ | 36,680 | 0.966 [0.945, 0.987] |

*Whole careers by predicted $q$.* Only the band 0.40-0.60 lies outside its own 95% interval.

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

| Races in a row | AA | AB | AC | BB | AD | BC | AE | BD | CC | BE | CD | AF | AG |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 race | 100% (100%) | 100% (100%) | 90% (90%) | 91% (90%) | 80% (80%) | 76% (80%) | 68% (70%) | 70% (70%) | 63% (70%) | 50% (60%) | 59% (60%) | 46% (50%) | 19% (20%) |
| 2 races | 100% (100%) | 100% (100%) | 91% (90%) | 90% (90%) | 79% (80%) | 80% (80%) | 61% (70%) | 68% (70%) | 78% (70%) | 50% (60%) | 65% (60%) | 52% (50%) | 19% (20%) |
| 3 races | 100% (100%) | 95% (95%) | 85% (85%) | · (85%) | 75% (75%) | · (75%) | 59% (65%) | 60% (65%) | · (65%) | 43% (55%) | · (55%) | 56% (45%) | 11% (15%) |
| 4 races | 90% (90%) | 81% (80%) | 74% (70%) | · (70%) | 49% (60%) | 62% (60%) | 38% (50%) | 61% (50%) | · (50%) | · (40%) | 30% (40%) | · (30%) | · (0%) |
| 5 races | 81% (80%) | 66% (70%) | 54% (60%) | · (60%) | · (50%) | · (50%) | 35% (40%) | · (40%) | · (40%) | · (30%) | · (30%) | · (20%) | · (0%) |
| 6 or more | 56% (60%) | 58% (50%) | · (40%) | · (40%) | · (30%) | · (30%) | · (20%) | · (20%) | · (20%) | · (10%) | · (10%) | · (0%) | · (0%) |

### Evidence

The runs record 119,188 attempts at the 34 single-race G1s, 34,089 of them goal races.

The table calls some races certain, 0% or 100%, and a handful went the other way, so its log-likelihood would be
$-\infty$. Every model compared therefore has its 0% and 100% pulled in to a fitted floor and ceiling. With the same
two parameters in each, the in-sample comparisons follow rule 1 of *How to read*.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Raised grades, not base | observed / predicted: base 1.063 [1.060, 1.066], raised 0.998 [0.996, 0.999] | raised fits 2,921 nats better | raised grades |
| A pink counts from the April of its inspiration | won 0.737 [0.696, 0.778] of 502 Senior January-March attempts where the timing matters | predicted 0.754 from April, 0.864 from the year's start | from April |
| The streak penalty is real | | without it, 2,753 nats worse | kept |
| A loss changes the next race's chance | after a loss over after a win, 4th race on: 1.021 [0.982, 1.061] | run-clustered; at the 2nd and 3rd the interval also covers 1 | not detected |
| Goal races pay one position less | community table, goal races 4th on: 1.141 [1.125, 1.158] | held out, forms table below | shift adopted |
| One formula is enough | | a free rate per chance, streak position and race kind, 101 cells, held out: -11 ± 15 nats, $z$ = -0.73 | table kept |
| Aptitude basis <=70 wins less (grouped before streak penalties) | 0.949 [0.931, 0.968] over 9,574 attempts | interval below 1 | known miss |

By scenario, every interval covers the prediction once chosen races 1st and 2nd in a row are left out.

*Goal races.* Under the community table, goal races from the 4th race in a row win more than predicted. Three forms
were scored on runs left out of the fit, five folds by run over 5,470 runs, each form with its own
fitted floor and ceiling:

| Form | Goal race at position $j$ pays | Races after it count it | Held out, against the adopted form |
|---|---|---|---|
| Adopted | $\mathrm{PEN}_{\max(j-1,1)}$ | yes | |
| Community table | $\mathrm{PEN}_j$ | yes | 158 ± 34 nats worse, $z$ = 4.72 |
| Exclusion | $\mathrm{PEN}$ at the chosen races before it | no | 1,151 ± 62 nats worse, $z$ = 18.58 |

With the shift, the goal-race rows from the 1st to the 6th or later cover the prediction, as do chosen races from the
3rd on. Chosen races 1st and 2nd in a row fall slightly short, a known miss. Goal races 7th or later, part of the 6th
or later row, fall well short: the other known miss.

| Race in a row | Goal races: observed / predicted [95%] | Attempts | Chosen races: observed / predicted [95%] | Attempts |
|---|---|---|---|---|
| 1st | 1.001 [1.000, 1.001] | 18,815 | 0.996 [0.994, 0.998] | 41,537 |
| 2nd | 1.000 [0.999, 1.001] | 9,378 | 0.997 [0.994, 0.999] | 30,712 |
| 3rd | 1.001 [0.999, 1.003] | 3,912 | 0.999 [0.995, 1.003] | 8,065 |
| 4th | 0.996 [0.991, 1.001] | 809 | 0.994 [0.981, 1.008] | 3,525 |
| 5th | 0.999 [0.972, 1.026] | 878 | 0.998 [0.959, 1.037] | 988 |
| 6th or later | 0.972 [0.908, 1.036] | 297 | 0.972 [0.865, 1.079] | 272 |
| goal, 7th or later | 0.68 [0.51, 0.84] | 47 | | |

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
  tool does, fits 29 nats worse.

### Evidence

The fit uses 174,056 trials as section 3 defines them, self-grandparents included. In 1,045 of them a
self-grandparent passed her spark. The distance is observed minus predicted passes, in run-clustered standard
deviations.

| Claim | Estimate | Test | Verdict |
|---|---|---|---|
| $\mathrm{tri}(T,P,T) = 0$ | 1074.7 passes predicted | distance −0.96 | kept |
| $\mathrm{tri}(T,P,T) = \mathrm{pair}(T,P)$ | 1241.8 passes predicted | distance −6.26 | rejected |
| a fraction $\kappa$ of $\mathrm{pair}(T,P)$ | $\hat\kappa = 0$, its lower bound | | 0 kept |

The gap is not the ordinary trials' own misfit: refitting the affinity slope on them, to 0.97, leaves both verdicts
unchanged.

## 6. Loop structures, ranking and schedules

Computed by `research/s06_loops.py`. The structures, their lineages $(T, P_1, P_2, X)$ and their ranking by mean
$P(\text{all})$ are defined in `docs/MODEL.md` and `docs/SOLVER.md`. $X$ is the uma each parent was bred from with the
other parent; a rotation has $X = T$.

### Findings

- **The rotation occurs in practice**: 102 runs have its exact lineage shape.
  More broadly, 1,911 of 5,534 runs have a grandparent of the trainee's own character.
- **Every pair of umas carries the same weight in a structure's summed affinities**, so the schedules maximise the total
  expected shared wins. That total is a surrogate: equal weights do not make it equivalent to the nonlinear landing
  odds.
- **With each loop's reported decks fixed, scheduling for shared wins costs little**: on the reference results, no
  schedule could raise a loop's odds by more than 2.01%, and no listed runner-up could overtake its winner.

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

The bound is computed on the 40 reference results, the best loops and decks found for the validation setups
(`val/ref__*`, `val/refpu__*`). An independent implementation reproduces their odds to within 2e-15.

| On the reference results | Value |
|---|---|
| winners' odds gained by the shared wins found, over none | 31-85% |
| most any other schedule could add to a winner, by the pair ceilings | 0.43-2.01%, median 0.84% |
| listed runner-ups whose pair-ceiling odds reach their winner | 0 of 12 |

This bound does not certify the ranking when schedules and decks change jointly.

## 7. Training hints

Computed by `research/s07_hints.py`.

A card's training can hint the skills in its *pool*. $hf$ is the card's Hint Frequency in percent and $\mathit{sp}$ its
Specialty Priority, both at maximum limit break with unique effects included.

### Findings

- **A card makes a nearly fixed number of uniform draws from its pool.** A skill already at hint level 5 is no longer
  drawn.
- **Hint frequency scales that number directly**: Hint Frequency +40% gives 40% more hints. The slope is **assumed**;
  on the adopted model a free one does not predict unseen careers better.
- **Specialty priority also raises it**, by a weight that differs between scenarios. The effect predicts careers it was
  not fitted to, but it is not shown to be a cause, and why it differs by scenario is not tested.
- **Known misses**: cards differ in hint rate beyond what their effects predict, for reasons not known. Decks with more
  pals give slightly more hints. A skill also hinted by inspiration is hinted by training less than predicted.
- SR cards give slightly fewer hints than SSR cards with the same effects in-sample, but that does not predict unseen
  careers better, so the model ignores rarity.

### Model

A card gives, in scenario $s$, $h$ training hints per career:

$$h = \beta_s \left(1 + \frac{hf}{100}\right)\left(1 + d_s\,\frac{\mathit{sp}}{100}\right).$$

That is a base $\beta_s$ for a card with neither effect, raised in proportion to hint frequency and to specialty
priority. A career gets $\lfloor h \rfloor$ hints, or one more with chance $f = h - \lfloor h \rfloor$.

Each hint draws one skill uniformly from the pool and raises it by the card's hint increment $\iota$. So a skill reaches
level 5 and leaves the pool after $c = \lceil 5 / \iota \rceil$ hints. The runs bear out the increment: 37,333 of
37,456 levels that one card gave alone are multiples of $\iota$, or 5.

The chance that none of $k$ targets in a pool of $n$ skills is hinted is computed jointly, not target by target:

$$M(k, n, h) = (1 - f)\, m(k, n, \lfloor h \rfloor) + f\, m(k, n, \lfloor h \rfloor + 1), \qquad
m(k, n, t) = \left(1 - \frac{k}{n}\right)^{t} \ (t \le c).$$

Each of $t$ hints misses all $k$ targets with chance $1 - k/n$. When $t > c$, an exact Markov chain over the other
skills' hint counts replaces $m$.

### Fitted values

**Fitted** by maximum likelihood on 87,801 rows from 15,120 card-runs. A row is one pool skill of one card in
one run; a card-run is one card in one run's deck. The fit takes SR and SSR cards at maximum limit break and leaves out
rows where an inspiration hinted the skill. The Runs column counts the runs that add a row. The solver reads these
values from `research/results.json` when it is built (`pkg/loopopt/build.rs`), rounded to three decimals.

| Scenario | $\beta$ | $d$ | Rows | Runs | Typical card's hints per career |
|---|---|---|---|---|---|
| Unity Cup | 2.395 | 0.322 | 17,737 | 1211 | 3.2 |
| Grand Concert | 1.999 | 0.068 | 55,136 | 3,194 | 2.5 |
| URA Finale | 1.380 | 0.359 | 12,551 | 736 | 1.9 |
| Trackblazer | 1.702 | 0.107 | 2,377 | 100 | 2.1 |
| Grand Masters | 1.991 | 0.129 | no runs: **assumed**, one $\beta$ and one $d$ fitted over every scenario's rows | | |

The typical card is the runs' median: hint frequency +20% and specialty priority 35.

### Evidence

*The form.* The form was chosen before the specialty term was added, on a wider sample: every card at any limit break,
R cards included, 140,615 rows from 5,534 runs. That study reads the count as continuous and ignores the level-5 cap. On
runs left out of the fit, per-scenario $\beta(1 + hf/100)$ predicts better than either simpler form:

| Alternative | Per-scenario $\beta(1 + hf/100)$ better held out by |
|---|---|
| one constant count, no $hf$ | 1,457.8 ± 31.6 nats, $z$ = 46.1 |
| one $\beta(1 + hf/100)$ for every scenario | 556.4 ± 20.8 nats, $z$ = 26.7 |

*Hint frequency's slope.* The $1/100$ is **assumed**: a card's +40% is read as 40% more hints. A free divisor tests it,
with and without the specialty term:

| Model | Free divisor | Held out, against $1/100$ | Verdict |
|---|---|---|---|
| adopted, with the specialty term | 90 | 1.8 ± 2.0 nats, $z$ = 0.9 | $1/100$ kept |
| form study, without it | 83 | 16.7 ± 2.6 nats, $z$ = 6.4 | free divisor better |

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
| Fitted on the earlier half of the runs, it predicts the later half | | 10.1 ± 3.6 nats, $z$ = 2.8 | holds |
| A weight per scenario beats one shared weight | | held out 13.2 ± 3.0 nats, $z$ = 4.5 | per scenario |
| With the term, the scenario × specialty cells fit | 13 cells of 200+ rows | run-clustered $\chi^2$ = 16.0, $p$ = 0.25 | consistent |
| The same card, only specialty priority changing | $d$ = −0.06 [−0.44, 0.56], 27 cards | +0.03 nats over no term | cause undecided |

The last row compares each card with itself. On these cards, from one limit break to the next, specialty priority changes and hint
frequency does not; a base is fitted per card and scenario. The estimate is consistent with no effect and with every
per-scenario weight. A scale on the per-scenario weights, refitted on either half of the runs, on SR or SSR cards alone
and without each of the eight most-used cards, stays within 0.65-1.12.

*The level-5 cap.* Removing a skill at level 5 fits 22.1 nats better than drawing with replacement, with no
extra parameter. It matters most where one hint reaches level 5: with replacement those rows read 1.221 of prediction,
with the cap 0.968.

*The draws' shape.* Three checks bear on the shape of the draws, and none runs the adopted model. The repeat and pair
checks use the form study's rate, without the specialty term or the level-5 cap, and the pair check reads the count as
continuous. The stat-option check assumes a fixed count. They test the shape, not the fitted rate.

| Question | Observed | Predicted |
|---|---|---|
| A skill a card hinted at all, hinted again: $P(k \ge 2 \mid k \ge 1)$ | 0.139 | 0.139 with the two-point count, 0.191 with a Poisson count |
| Two skills of one pool both hinted, over 247,441 pairs, Trackblazer out | 0.0792 | 0.0794 by joint draws, 0.1094 independently |

That a draw never picks the card's stat option, the one non-skill option beside its pool, is **assumed**. In 71 of
91 (card, $hf$) cells the count of draws varies less than a drawn stat option would allow. The check keeps only
card-runs whose every count is exact, with no skill at level 5 or hinted by another source, and uses a sample
variance biased low. It is consistent with the assumption and does
not establish it.

*Covariates and cards.* Card type, initial bond and pool size add no nominally detectable gain to the adopted rate.
Three other terms, tested on 87,801 rows under the limits *How to read* sets out:

| Added term | In-sample gain | Held out | Verdict |
|---|---|---|---|
| rarity, SR against SSR | +2.9 nats | 1.8 ± 1.4, $z$ = 1.3 | not predictive, ignored; factor 0.971 |
| pals per deck | +12.6 nats | 12.2 ± 2.9, $z$ = 4.2 | known miss; factor 1.028 per pal |
| a factor for each of 58 cards with 200+ rows | | 48.2 ± 6.0, $z$ = 8.0 | known miss; cards read 0.80-1.17 |

*Other sources of the same skill.* The fit leaves out rows where an inspiration also hinted the skill. Against the
adopted rate, the card's own hint is seen:

| Other source of the skill | Rows | Observed / predicted |
|---|---|---|
| none | 77,728 | 1.005 [0.999, 1.011] |
| inspiration | 8,883 | 0.73 |
| card event | 2,191 | 0.99 |
| other card's training | 3,557 | 0.96 |

Removing both inspired skills for the whole run predicts a larger redistribution than observed. This comparison
does not exclude partial or time-dependent removal. The inspiration shortfall remains unmodelled. On card-runs where an inspiration hinted two pool skills, training hinted the rest at 0.371. With the
two kept in the pool the model predicts 0.340; with them removed, 0.462.

*Prioritised skills.* The model assumes a priority list does not change training hints. On the 1,201 runs that
show a list, 868 rows hold a prioritised pool skill. Training hinted them at 0.336, against 0.326
predicted: no sign of a rise, from a weak test.

## 8. Chain events

Computed by `research/s08_chains.py`.

### Findings

- **One $N$ serves every scenario.** Separate distributions per scenario gain nothing significant.
- **Remaining-link weighting scores above equal shares per card** (**fitted**). The composite score compares
  observed markers with latent completion probabilities and omits the marker allowance.
- **Known misses**, causes unknown. Decks with Sasami Anshinzawa [This Might Sting!]'s pal card have a higher fitted capped count,
  and Inari One's careers a lower one. In decks with many chain cards and few pals, early-link markers run below latent predictions.
  Final-link markers lie close to latent predictions pooled, but card residuals vary. These comparisons omit the
  marker allowance and do not validate the fitted observation model.

### Model

A run gets $N \sim \mathrm{Binomial}(21, 0.35)$ chain events (**assumed**, informed by fits), mean 7.35.
Each event advances one chain card by one link. The card is picked in proportion to its links not yet fired, its
remaining *slots*. A chain is 3 links on an SSR and 2 on an SR, and a stopped chain takes no more events.

Pal and group cards take no slot (**assumed**). The runs do not identify this: decks with more pals have fewer chain
cards, which finish early and leave larger counts unobserved.

A chain can stop early (**assumed**, from section 9's equal outcomes). If IT's default choice at a link ends the chain
on some of its outcomes, the chain stops there with the share of outcomes that end it. Its unused slots return to the
pool. The model enumerates the exact distribution of final chain states (section 12).

### Evidence

*Reading a chain's progress.* Progress is read from marker skills, on 5,316 runs. A link is
observable only if every choice and outcome of its event carries a skill that no other event of the card gives. A
credit naming two sources, "A or B event", counts as unknown when B is another deck card whose events can give the
marker. So does a marker credited only to inspiration. Each fit of $N$ is exact over the chain states the run's deck
can pass through.

*The marker allowance* (**fitted**). The fits let a fired link's marker go unseen with chance 0.02; without it, a run
whose later link shows while an earlier one does not has likelihood 0. Links fire in order, so those earlier links
measure the rate:

| Earlier links implied fired by a later one | Showing no marker | Share [95%] |
|---|---|---|
| 1,800 | 38 | 0.021 [0.014, 0.028] |

At fixed $n$, the best $p$ stays within the narrow range below across the tested allowances. The zero-allowance
check uses only runs without a contradiction. At $n = 21$, the range is 0.34-0.35.

*Which distribution for $N$?* The free distribution fits $P(N = 0..17)$ and $P(N \ge 18)$. No deck has more than 18
links, so the runs cannot tell larger counts apart, and every mean and variance of a free fit here is that of
$\min(N, 18)$. Against a fitted two-parameter family, the nominal dimension difference is
16. Against the fixed binomial, which estimates no
parameters, it is 18. Boundary and discrete-parameter issues still require
calibrating the reference distribution (*How to read*, rule 2). The likelihood deficit is descriptive evidence.
Held out, each form is fitted on four groups of runs and scored on the fifth.

| Claim | Estimate | Test | Verdict |
|---|---|---|---|
| $\min(N, 18)$ is under-dispersed | free fit: mean 7.22, variance 4.44, ratio 0.62 | Poisson(7.1) 136.2 nats behind the free fit | Poisson rejected; $N$'s own dispersion not identified |
| The binomial is the better under-dispersed two-parameter form | best at $n$ = 17, $p$ = 0.426 | 4.8 nats above Conway-Maxwell-Poisson | binomial |
| The fitted binomial fits in-sample | 12.1 nats behind the free fit | $p$ = 0.084 | not rejected |
| Binomial(21, 0.35) fits in-sample | variance 4.78; 21.9 nats behind the free fit | nominal $p$ = 6.2e-04 | fits worse; reference uncalibrated |
| $n = 21$ is in the binomial's profile set | $n$ from 16 to 19 | $n = 21$ is 6.42 nats below the best | just outside |
| The free distribution predicts unseen runs better than (21, 0.35) | | held out +12.6 ± 6.6 nats, $z$ = +1.90 | no clear advantage |
| The fitted binomial predicts unseen runs better | | held out +6.4 ± 4.8 nats, $z$ = +1.34 | no clear advantage |

Within the binomial family, the runs constrain the mean more tightly than the spread. The binomial's $n$ and $p$
trade off along a ridge. The nominal
95% profile set keeps every $n$ whose refitted $p$ comes within 1.92 nats of the best. So the runs favour a smaller $n$
in-sample, but no single pair is a finding, and keeping (21, 0.35) is a decision.

*Does $N$ depend on the deck or trainee?* For two groups, yes. The model has one $N$ for every deck and trainee, so it
under-predicts the first and over-predicts the second. The test refits $p$ at $n = 21$ on each side:

| Group | Runs | Mean capped count $\min(N, 18)$, group / other runs | $p$ at $n = 21$, group / other runs | Likelihood-ratio $p$ |
|---|---|---|---|---|
| decks with Sasami Anshinzawa [This Might Sting!]'s pal card | 639 | 7.74 / 7.17 | 0.374 / 0.342 | 3e-07 |
| Inari One's careers | 473 | 6.83 / 7.26 | 0.326 / 0.347 | 0.001 |

On the runs with neither group, the fixed-binomial comparison also gives a small nominal $p$:
0.013.

*Scenarios.* A free distribution per scenario gains 29.5 nats for
54 extra parameters, short of the nominal threshold.

*Slots or cards?* (**fitted**.) The slot rule scores 101.9 nats better than giving
each running card's next event the same chance. The score sums each link's own likelihood, a composite score
(*How to read*). Final links by pal count show where the rules differ. These checks score each link by the model's
latent chance of reaching it, without the marker allowance. They compare different observation models from the
fit of $N$. Their interval placements do not establish calibration of the marker model:

| Pals | Chain cards | Runs | Final-link marker observed [95%] | Latent completion prediction |
|---|---|---|---|---|
| 5 | 1 | 127 | 0.952 [0.899, 0.978] | 0.979 |
| 4 | 2 | 552 | 0.892 [0.869, 0.915] | 0.860 |
| 3 | 3 | 1,426 | 0.583 [0.565, 0.602] | 0.566 |
| 2 | 4 | 1,399 | 0.318 [0.304, 0.332] | 0.342 |
| 1 | 5 | 1,414 | 0.224 [0.213, 0.234] | 0.225 |
| 0 | 6 | 398 | 0.146 [0.130, 0.163] | 0.158 |

The latent prediction lies inside the marker interval at 3, 1 and 0 pals, above it at 5 and 2, and below it at 4. Equal
shares per card predict 0.41 at 2 pals, further above. At 0 pals they
predict 0.23, outside the interval.

*Known miss: early-link markers in crowded decks run below latent predictions.* An early link precedes the final
link. The shortfall is the share predicted to fire minus the share with a marker seen, ± its run-clustered standard error. Predictions
omit the marker allowance. Each run's links form one cluster: $z = \sum(\text{pred} - \text{obs}) / \sqrt{\sum(\text{pred} - \text{obs})^2}$, summed
over runs.

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Early-link shortfall | 0.02 ± 0.01 | 0.04 ± 0.01 | 0.08 ± 0.01 | 0.05 ± 0.01 | 0.14 ± 0.02 |
| $z$ | 1.3 | 4.7 | 6.5 | 3.7 | 5.8 |

*Known miss: absent first-link markers.* With five or six chain cards, more first-link markers are absent than
latent predictions of unstarted chains. The comparison omits the marker allowance:

| Chain cards | First-link marker absent [95%] | Latent unstarted prediction |
|---|---|---|
| 5 | 0.19 [0.16, 0.22] | 0.13 |
| 6 | 0.34 [0.29, 0.39] | 0.20 |

*Known miss: individual cards' final links.* Pooled, markers and latent predictions lie close: 0.348
observed against 0.353. Card residuals vary. The marker allowance is omitted. The test covers the
50 cards with 60 or more final-link observations, clustered by run, since one run's
chain cards share its $N$. It gives $\chi^2$ = 386, where chance alone gives about
50.

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

- **IT follows the choice rule** in the three events where the runs show which choice it took.
- **The event-hint calibration by priority is in section 13.**
- **Five adopted outcome estimates depart from the equal rule**, and the model uses them. Some other outcomes are off the
  equal rule but left at it: known misses.
- **Pal and group finales are assumed to fire every time.** They fired in every checked run: an observed rate on a
  finite sample, not a game rule.

### Measured shares

These shares are **fitted**. The table distinguishes observed counts from imputed ones. Festa's binomial interval
uses a partly imputed denominator. A *reading* is a firing where every outcome could show: the card's hint credited,
and each outcome's skill below level 5 from other sources.

| Event: outcome | Observed or attributed counts | Share [90%; Festa coverage unestablished] | Adopted |
|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] finale, Risky Business prioritised: Nothing Ventured | 58/77 | 0.75 [0.66, 0.82] | 0.75 |
| Riko Kashimoto [Planned Perfection] "Unexpected Side", top choice: Ramp Up, not Maverick ○ | 527/585 | 0.90 [0.88, 0.92] | 0.75 |
| Seiun Sky [Foolproof Plan] link 3: Fast-Paced, not Escape Artist | 33/40 | 0.82 [0.71, 0.90] | 0.75 |
| Smart Falcon [My Umadol Way! ☆] link 3: Center Stage, not Prudent Positioning | 37/51 | 0.73 [0.61, 0.81] | 0.75 |
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante": Nothing Ventured, not Risky Business | 151 observed wins / 246.7 imputed trials | 0.61 [0.56, 0.66] | 0.60 |

Riko Kashimoto's adopted share mixes her choice policy with her outcome. It is the share of runs with her Ramp Up hint
under the runs' own choices, so it sits below the top-choice share above. Nakayama Festa's share rests on an
attribution model. Its interval has unestablished coverage. Both are set out under *Evidence*. Except for Nakayama Festa's,
the counts in this section's share and choice tables use an earlier subset of the runs, not the full pool.

### Evidence

*Does IT follow the choice rule?* Where the runs show the choice taken, yes:

| Event | Condition | Choice taken |
|---|---|---|
| Sasami Anshinzawa's finale | Risky Business prioritised | choice 2 in about 95% of runs |
| Sasami Anshinzawa's finale | neither skill prioritised | identifiable choice-two wins in 0 of 35 runs |
| her date-start event "A Pleasant Practice Partner?", read by its extra Risky Business +1 | Risky Business prioritised | bottom choice in 53 of 54 runs winning finale choice 2 |
| the same | Risky Business not prioritised | bottom choice in 1 of 20 such runs |
| Riko Kashimoto [Planned Perfection]'s event | Lone Wolf prioritised | bottom choice in 55 of 62 runs |

Of the 4 finale runs that took choice 1 with Risky Business prioritised, 3 had it already at hint Lv 5.
The records show a priority list for only 1,201 of 5,534 runs. The rest count as unknown, never as an empty
list, so the rule is tested mainly through its consequences in section 13.

*Reading Sasami Anshinzawa's finale.* Nakayama Festa [Trusting the Dice]'s "Upping the Ante" also gives Nothing
Ventured and shares its credits. So these rows use the 254 runs with Anshinzawa's card and without Festa's.
Hint levels identify the choice:

- her Nothing Ventured +3 is choice 1, which otherwise gives nothing, and +1 is choice 2;
- a loss on choice 2 shows as her Risky Business at level 2, from storyline choice 3 and the finale.

GameTora marks the date-start event's bottom choice as ending her event chain, which for a pal card means her dates.
Yet with Risky Business prioritised the finale shows its outcome in at least 75 of 77 runs. IT
apparently keeps her dates either way.

*Riko Kashimoto's adopted share.* The adopted value is the share of runs with her Ramp Up hint, whichever choice IT
took. It folds the runs' choice policy into the outcome, and carrying it over to a player who prioritises Ramp Up is
assumed:

| Riko Kashimoto [Planned Perfection] | Count | Share [90%] |
|---|---|---|
| runs with her Ramp Up hint | 521 of 689 | 0.756 [0.728, 0.782] |
| firings where IT took the top choice; the rest took the bottom choice, Lone Wolf | 579 of 725 | 0.80 |

*Nakayama Festa's share.* It starts from the 671 runs with her card, less her own careers and decks with
Sasami Anshinzawa's SSR. The credits there:

| Credit | Runs |
|---|---|
| to her alone, Nothing Ventured | 151 |
| to her alone, Risky Business | 71 |
| Risky Business, to "her or another card's event" | 58, Team Sirius in 48 |
| neither outcome, with Risky Business already at Lv 5 | 43 |

A shared credit is split by estimated source rates. Let $r_F$ be her exclusive Risky Business credit rate in runs with
her card, 0.111. Let $r_o$ be the other card's exclusive rate in runs without her: 0.848 for Team Sirius,
0.445 for Sasami Anshinzawa. A shared credit goes to her with weight $r_F / (1 - (1 - r_F)(1 - r_o))$, an attribution weight obtained by substituting exclusive-credit rates into an independent-source formula.
It is not an identified conditional firing probability. That gives her 8.3 of them.

In a blank run either a Risky Business win could not show or the event never fired. The blank runs are split by the
rate at which the event shows where both outcomes can, 0.38. Whatever part of the shared credits is hers, the share
stays within 0.504-0.635.

So the share rests on an attribution model. Its rates are exclusive credit rates, not firing rates, and it assumes the
sources fire independently. The blank-run split ignores which outcome is missing. The interval rounds the imputed denominator and treats it as a binomial trial count. It omits attribution
uncertainty, so its coverage is unestablished. No joint observation model is
fitted or tested.

*Shares left at the equal rule.*

| Event: outcome | Share | Verdict |
|---|---|---|
| Tazuna Hayakawa [Tracen Reception] finale: Concentration | 0.70 | off 0.5: known miss |
| Agnes Tachyon [Q≠0] link 1 choice 2: Murmur | 0.70 | off 0.5: known miss |
| Marvelous Sunday [Dazzling Day in the Snow] link 3: Position Pilfer | 0.58 | off 0.5: known miss |
| Light Hello [From the Ground Up] finale: See Ya Later! +3 | 0.71: 1,497 of 2,103 | off 0.5: known miss |
| Gold Ship [That Time I Became the Strongest] link 3 | 0.61 | consistent with 0.5 |
| King Halo [Tonight, We Waltz] link 3 | 0.57 | consistent with 0.5 |
| Daitaku Helios [Sunlit Angel ♪] link 2, where link 3 visibly fired: Tail Held High | 0.47 | consistent with 0.5 |

*Pal and group finales.* The checks use runs where the card is the deck's one source of the skill and no inspiration
hinted it. Light Hello [From the Ground Up]'s event is credited with See Ya Later! in 1679 of 1679 such runs.
Team Sirius is credited with Risky Business, alone or shared, in 65 of 65. These observed rates are
consistent with a finale that always fires; finite samples do not establish it.

## 10. Random card events (not modelled)

Computed by `research/s10_random_events.py`.

A deck card's character has *shared random events* that can fire in another uma's career; any rarity counts, R cards
included. $K$ is the number on offer: every shared event of the deck's characters plus the trainee's own. A *reading*
is one such event in one run, read through the marker skill on its top choice.

### Findings

- **Leaving them out under-predicts the targets they can hint**, a known miss: on real careers those targets read
  1.24 of the model. Credited at the fitted rate, they read 1.00.
- **Two default targets are affected**: Vodka's Nimble Navigator and Nice Nature's Ramp Up, the only ones that are a
  character's top-choice marker.
- **A shared random event fires in about 0.14 of the careers it can fire in** (**fitted**), a lower bound. The
  rate falls slightly as more events are on offer, but a constant rate predicts unseen runs as well.
- **Crediting them has no clear held-out advantage** on the rows the model gives a positive chance.
- **Known miss of the measured rate: the card's initial friendship gauge raises it.** It changes no prediction, since
  the model leaves these events out.

### Evidence

*The rate.* A hit is a top-choice marker credited to the character's event. An *ambiguous credit* also names another
deck card whose events can give the marker; as in section 8, it is left out as unknown. The 8,169 readings come
from 4,232 runs. They give 1,128 hits, with 222 ambiguous credits left out.

A firing goes unseen when IT takes another choice for a prioritised skill, or when the marker is already at Lv 5 from
other sources. So the rate is a lower bound. Counting ambiguous credits as hits, over the readings where the event
could show, gives 0.164.

The *$K$ form* lowers the rate by a fixed fraction for each extra event on offer. The card effects and scenario are
fitted beside $K$ in a logistic model.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| The rate falls as more events are on offer | $K$ form: 0.18 at $K$ = 4, 0.12 at 14 | run-clustered Wald $\lvert z \rvert$ = 2.91 | detectable |
| The $K$ form predicts unseen runs better than a constant | constant 0.138 | held out +1.8 ± 2.9 nats, $z$ = +0.63 | no |
| The initial friendship gauge raises the rate | +0.102 [+0.041, +0.164] logit per 10 points | Holm-adjusted run-clustered $p$ = 0.0034 | known miss |
| Friendship bonus moves it | −0.050 logit per 10 points | Holm-adjusted $p$ = 0.60 | no effect |
| Specialty priority moves it | +0.016 logit per 10 points | Holm-adjusted $p$ = 0.60 | no effect |
| Scenario moves it | | run-clustered $p$ = 0.19 | not detectable |
| Rarity moves it within a character | | run-clustered $z$ = +1.13 | no effect: R and SR cards pooled with SSR |

Pairs of one run's readings that both hit are as frequent as independent firing predicts. Independent firing remains
an assumption.

*What does leaving them out cost on real careers?* The test uses section 13's rows, each on the run's own deck. It
scores whether the target was acquired from any source, so it does not depend on reading the event. Both baseline
and credited acquisition use a hybrid reconstruction: exact landing marginal $p$ and the report's approximate gold
chance $g$. For an unguaranteed target, it reconstructs acquisition as $p/W-g$, with $W$ the white-spark generation
factor from section 12. The baseline therefore includes the difference between exact and approximate gold chances.
These comparisons are neither exact acquisition calibration nor a consistent test of the report approximation.
Crediting uses the
$K$ form at each run's $K$, about 0.14 on these rows.

| Rows where a deck card's random event could hint the target | Count |
|---|---|
| all | 305 |
| target credited to that event | 35 |
| the same, with no other source | 22 |
| rows the model gives no chance | 73 |
| of those, hinted by the event | 6 |

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Left out, these targets are under-predicted | 1.24 [1.11, 1.38] | interval excludes 1 | known miss |
| Credited at the fitted rate, they are not | 1.00 [0.90, 1.11] | interval covers 1 | consistent |
| Crediting predicts unseen runs better | | held out +4.9 ± 2.8 nats, $z$ = +1.76 | not shown |

The held-out score refits the rate on the other folds' readings. It covers only the 232 rows with a positive
baseline prediction: a row the model gives no chance has no finite likelihood when acquired. So it leaves out the rows
where an omitted source matters most, and it cannot settle the value of adding the source.

Crediting raises the product of positive per-target landing marginals by a median
13%. The largest rise, over 232 affected runs, is 156%. This uses the hybrid reconstruction above. It does not recompute exact all-target odds.

## 11. Conditional character events (not modelled)

Computed by `research/s11_conditional.py`.

The trainee's secret and race-result events are left out of the model, except Tokai Teio's Head-On. A
*conditional-only row* is a run-target row where only such an event can give the target: none of the trainee's story,
date, outfit or shared random events can, nor the scenario's.

### Findings

- **The sparse pooled conditional-only comparison is inconclusive.** Its score excludes zero-baseline rows,
  including 3 acquisitions. The 89 rows
  do not settle trainee-specific costs.
- **Known miss: Vodka's own Nimble Navigator.** Her events give it in most of her runs, and the model under-predicts
  it. Her random and conditional events both give it and cannot be told apart.
- **Tokai Teio's Head-On is modelled as always acquired**, like Yamanin Zephyr's Slipstream. All three of its races
  are her career goals. The goals ask only for top 5, top 5 and top 3, while the event needs wins, so certainty is
  **assumed** from observed wins: IT won all three in 62 of 62 runs training her. Every run where her
  event could show credits Head-On.

### Evidence

| Rows | Her events gave the target | Rate [95%] |
|---|---|---|
| all conditional-only rows | 5 of 89 | 0.056 [0.024, 0.125] |
| Daiwa Scarlet, any outfit, Nimble Navigator | 0 of 84 | [0.00, 0.04] |
| Fine Motion, any outfit, Ramp Up | 1 of 1 | [0.21, 1.00] |
| Mejiro Ardan, any outfit, Ramp Up | 4 of 4 | [0.51, 1.00] |
| Vodka, any outfit, Nimble Navigator, not conditional-only | 25 of 27 | [0.77, 0.98] |

| Rows, model as it is | Acquisition, observed / predicted [95%] | Crediting the measured rates gains |
|---|---|---|
| conditional-only | 1.10 [0.81, 1.39] | 0.5 nats |
| with Vodka's rows | 1.33 [1.12, 1.53] | 16.2 nats |

Both sides use section 10's hybrid acquisition reconstruction, mixing exact landing marginals with approximate
gold chances. Each gain is in-sample and is set against half the $\chi^2$ 95% point for one degree of freedom per fitted rate: the
first falls short, the second exceeds it. As in section 10, the gains count only rows with a positive baseline
prediction. That leaves out 25 conditional-only rows the model gives no chance, and her events
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
recorded shared-G1 counts. The model gives each default target's chance at each step (sections 3-12), and the run
record shows what happened.

| Careers | URA Finale | Unity Cup | Grand Concert | Trackblazer | Run-target rows | On her outfit | Scenario golds |
|---|---|---|---|---|---|---|---|
| 5,198 | 754 | 1,148 | 3,213 | 83 | 41,584 | 2,001 | 2,296 |

Rows on her outfit and scenario golds are *certain*, so tests 1 and 3 leave them out; the rest are *open targets*.
Unity Cup's guaranteed golds show in 1,148 of its 1,148 runs. The runs end before skills are bought, so
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
| 1. Deck hint | 24,526 | 0.255 | 0.926 [0.912, 0.939] | optimistic |
| training pool only | 5,036 | 0.361 | 0.994 [0.959, 1.028] | covers 1 |
| events only | 3,538 | 0.896 | 0.891 [0.878, 0.903] | optimistic |
| events only, target prioritised | 551 | 0.859 | 0.970 [0.940, 0.9997] | known miss |
| events only, not prioritised | 584 | 0.884 | 0.653 [0.613, 0.694] | far below: the priority assumption fails |
| 2. Inheritance | 41,584 | 0.359 | 0.987 [0.978, 0.997] | below 1 through Tail Held High |
| without Tail Held High | 36,386 | | 0.992 [0.982, 1.003] | covers 1 |
| Tail Held High | | | 0.953 [0.918, 0.988] | known miss |
| 3. Acquired, any source | 37,287 | 0.513 | 0.986 [0.980, 0.993] | slightly optimistic |
| prioritised or unknown | 32,848 | 0.506 | 0.998 [0.990, 1.005] | covers 1 |
| prioritised | 3,556 | 0.700 | 1.008 [0.993, 1.024] | covers 1 |
| not prioritised | 4,439 | 0.559 | 0.911 [0.893, 0.929] | optimistic |
| unknown priority | 29,292 | 0.483 | 0.996 [0.987, 1.004] | covers unity |

### Evidence

*Acquisition reliability.* The product approximation misses acquisitions in its zero-prediction bucket.
Positive bands also show variation hidden by the pooled ratio. These are checks of the acquisition approximation
on open targets, not the exact engine. Each positive band's observed rate is followed by its mean prediction.

| Predicted acquisition | Rows, observed (mean prediction) |
|---|---|
| 0 | 8,822, 0.011 |
| above 0 to 0.20 | 1,943, 0.162 (0.15) |
| 0.20-0.40 | 3,812, 0.319 (0.30) |
| 0.40-0.60 | 4,633, 0.501 (0.50) |
| 0.60-0.80 | 6,996, 0.689 (0.71) |
| 0.80-0.95 | 4,973, 0.832 (0.86) |
| 0.95-1 | 6,108, 0.974 (1.00) |

*Where the deck loses its points.* The deck's rows predicted 0.6 or more hold 91% of its shortfall,
and 97% of those rows have an event source. On prioritised-or-unknown rows, the deck and inspiration alone
read 0.981 of the model. Unmodelled scenario and trainee events close the gap to the any-source row: they gave
299 target acquisitions, 0.8% of open rows. Agreement after adding them can hide errors in the parts, and it
does not validate candidate rankings.

*Landing.* Each open row's spark chance is recomputed from what the run acquired: $0.4 \cdot 1.1^n$ if the gold was
hinted, $0.2 \cdot 1.1^n$ if only the white. Summed, it reads 0.984 of the model's $p$, in line with acquisition. It
does not test spark generation (section 1).

*Per target*, observed / predicted:

| Target | Deck hint [95%] | Inheritance | Acquired |
|---|---|---|---|
| Uma Stan / Superstan | 0.79 [0.69, 0.91] | 0.996 | 0.977 |
| Nimble Navigator / No Stopping Me! | 1.04 [0.95, 1.14] | 0.984 | 1.048 |
| Playtime's Over! / See Ya Later! | 0.985 [0.97, 1.00] | 1.000 | 0.991 |
| Ramp Up / It's On! | 0.823 [0.78, 0.86] | 1.009 | 0.927 [0.90, 0.95] |
| Tail Held High / Tail Nine | 0.979 [0.92, 1.03] | 0.953 [0.92, 0.99] | 0.983 |
| Racing Spirit: Stamina | no deck source | 0.999 | 1.042 |
| Slipstream / Perfect Spot! | 0.986 [0.92, 1.06] | 0.974 | 0.991 |
| Risky Business / Nothing Ventured | 0.828 [0.77, 0.89] | 0.928 | 0.891 [0.85, 0.94] |

*Gold acquisition*, observed / predicted:

| Gold | Rows | Observed / predicted [95%] |
|---|---|---|
| Nothing Ventured | 1,260 | 0.852 [0.80, 0.90] |
| Perfect Spot! | 684 | 0.835 [0.78, 0.89] |
| It's On!, outside Unity Cup | 161 | 0.456 [0.33, 0.62] |

*Training-pool hints by scenario* test $\beta$ and $d$ (section 7). Only Trackblazer's interval excludes 1: its $\beta$
over-predicts these runs. Split instead by the lowest limit break of the sourcing cards, the LB0, LB1 and LB2 intervals
each cover 1.

| Scenario | Grand Concert | Unity Cup | URA Finale | Trackblazer |
|---|---|---|---|---|
| Observed / predicted [95%] | 1.014 [0.97, 1.05] | 0.928 [0.83, 1.03] | 0.995 [0.89, 1.10] | 0.779 [0.63, 0.93] |

*Every open target in one run.* The share of runs acquiring all their open targets matches the mean product of their
per-target predictions:

| Runs | Product of the model's odds | Observed [95%] |
|---|---|---|
| all | 0.0449 | 0.0423 [0.0372, 0.0481] |
| 4,123 with every open target prioritised or unknown | 0.0250 | 0.0250 [0.0206, 0.0302] |

*Spark-weighted acquisition of all targets.* $P(\text{all eight})$ is recomputed from what each run acquired and
compared with the model's exact $P(\text{all})$; Results gives the ratio, its interval bootstrapping whole runs. Only
220 runs acquired every open target, too few to separate the exact joint from the product.

*Single cards.* Where one card is the deck's only source of a target, the deck test scores that card alone. The table
lists every such pair of 40 or more rows whose interval excludes 1.

| Card > target | Rows | Predicted | Obs / pred [95%] | Likely reason |
|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | 437 | 1.000 | 0.76 [0.72, 0.80] | the model assumes Risky Business prioritised, which takes finale choice 2 |
| Riko Kashimoto [Planned Perfection] > Ramp Up | 708 | 0.750 | 0.65 [0.60, 0.70] | open; her Lone Wolf choice, when prioritised, accounts for part |
| King Halo [Tonight, We Waltz] > Uma Stan | 125 | 0.947 | 0.79 [0.71, 0.87] | Uma Stan sits on a choice that needs a priority |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | 77 | 0.307 | 0.63 [0.40, 0.97] | open |
| Maruzensky [Sentimental Flare ♪] > Slipstream | 330 | 0.244 | 1.19 [1.00, 1.40] | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | 96 | 0.340 | 1.41 [1.12, 1.70] | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | 345 | 0.725 | 0.88 [0.81, 0.95] | open |
| Nakayama Festa [Trusting the Dice] > Risky Business | 462 | 0.612 | 0.85 [0.78, 0.93] | open |
| Tokai Teio [Step! Smile! Wink!] > Nimble Navigator | 148 | 0.423 | 1.28 [1.09, 1.46] | open |

*King Halo's Uma Stan follows the priority.* The expectation and the exact Poisson-binomial tail are the model's:

| Runs that list Uma Stan | Rows | Hinted by her card | Expected | $P(X \le \text{hinted})$ |
|---|---|---|---|---|
| prioritised | 38 | 35 | 37.1 | 0.062, consistent |
| a list without it | 12 | 0 | 11.7 | |

*Smaller pairs.* Below 40 rows, 5 more pairs exclude 1. 3 of them are event sources of Playtime's Over!: Yukino Bijin
[Dancing Light into the Night], Yaeno Muteki [Fiery Discipline] and Sirius Symboli [Escorte Étoile]. They hinted it in
1 of 49 runs. The model predicted 0.51-0.84. Their event puts the skill on choice 2, which ends the
chain before its last link, so IT takes it only when the skill is prioritised. Only 5 of those runs show a
priority list, too few to test that cause.

*Other skills.* Every non-target skill a deck card or spark could give, on a 30% sample of 1,559 runs:

| Test | Rows | Observed / predicted [95%] | Verdict |
|---|---|---|---|
| inheritance | 66,598 | 0.986 [0.973, 0.999] | slightly short: known miss; no one skill carries more than 9% of it |
| training-pool hints | 26,573 | 1.056 [1.045, 1.068] | under-predicted: known miss |
