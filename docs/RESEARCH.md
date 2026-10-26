# The model's mathematics

This is the evidence behind the Legacy Loop Planner's model of Uma Musume's Independent Training (IT): how a career
picks up skills and how it passes them on. For each part of the model it says what was measured, what was fitted to
real careers, what was only assumed, and where the model is known to be wrong. The constants themselves, with their
sources, are listed in `docs/MODEL.md`.

## Results

The model was checked against 8,373 real careers, each rebuilt from its own deck and lineage (section
13). The check is in-sample: sections 3, 5, 7, 8 and 9 fitted their constants on these same careers, so agreement
here is weaker evidence than a test on careers the fits never saw. Target skills were hinted
0.990 times as often as the model predicts, so it is slightly optimistic.
Prioritised targets come out slightly above the prediction (1.014), and
targets left off a shown priority list well below it (0.953); in the
arithmetic that second group accounts for 115% of the
shortfall, and why it falls short is not established. On runs showing no list, the model is optimistic about the
targets it rates likely, and the pooled ratio there reaches 1 only because events the model leaves out add hints.
Landing every target in one career is rarer than predicted (0.86), but that check rests
in effect on 386 careers. The per-target checks score the career report's
quick approximation, which multiplies the chances of separate cards; the all-target check uses the exact model. *How to read* explains the terms, the labels and what each
kind of test can show.

| § | Conclusion | How it is set | Main evidence |
|---|---|---|---|
| 2 | Grand Concert defaults to the 17,500+ star mix; URA Finale, Unity Cup and Trackblazer to the lower one | default chosen from rank-score ceilings, which are upper bounds | Grand Concert: 79% of careers reach 17,500+ on the ceiling, 29% if real scores sit 1,000 below it; averaging stars overstates $q$ by at most 1.61% on the runs' affinities, under the assumed independent-slot star mixes |
| 3 | Grandparents pass sparks at the full base rate | fitted | weight 0.99 [0.98, 1.01] against the model's 1 |
| 3 | The community affinity formula is kept by rule 3, with +3 per shared G1 win; parents pass sparks less often than it predicts | assumed, tested | parents read 0.970 [0.954, 0.985] of the model. In sample a parent weight or a lower affinity slope is detectable, and the trials cannot tell the two apart. Held out neither improvement is established, parent weight $z$ = 1.84 and slope $z$ = 1.58, where a real change of the in-sample size would be expected to give 1.76 and 1.65 |
| 4 | Overall, the community race-win table predicts wins to within 1% once grades include inherited aptitude sparks, with a goal race charged the streak penalty of the race before it; it misses more at the lowest aptitudes and in long streaks | community table, tested; goal shift fitted | wins 0.997 [0.996, 0.998] of predicted over 197,254 G1 attempts, slightly fewer than predicted; losses 1.047 [1.033, 1.061] of predicted |
| 5 | A trainee who is her own grandparent gets no trio points from it | fitted | 43 nats better than counting the pair's points |
| 5 | The loop model's expected shared wins, from base aptitudes, are too low, mostly in races the base table gives under 0.95 | tested | two careers running the same race both win it 1.118 [1.114, 1.122] times as often as the base table predicts, 1.0007 where both base chances are 0.95 or more. With the records' shared wins cut by these two ratios, parents read 1.020 and 0.970 of the model, grandparents 1.028 and 0.995: the sign of the planner's bias on parents depends on where its races fall |
| 6 | Planning a loop's race schedules for the most shared wins costs little | derived | with each loop's decks fixed, no schedule raises a reference loop's odds by more than 2.32% |
| 7 | A card's training hints are equally likely draws from its pool; how many depends on the scenario, rises with Hint Frequency (slope assumed) and is higher on cards with more Specialty Priority | form fixed on the runs; base and specialty weight fitted | real careers' training-pool hints: 0.994 [0.96, 1.02] of the model, mostly on the cards the fit used; on cards below their top limit break, outside the fit: LB0 0.98 [0.86, 1.09], LB1 1.02 [0.88, 1.16], LB2 0.95 [0.83, 1.06] |
| 8 | Chain events are Binomial(21, $p$) with $p$ rising with the deck's pals, each event going to a running chain card with equal chance | fitted | predicts unseen runs better than the old model, Binomial(21, 0.35) with events weighted by links left: $z$ = 11.5; than the same budget weighted by links left: $z$ = 6.8 |
| 8 | A chain that ends early frees its events for the other chain cards | supported, conditional on the rest of the model | held out, freed over wasted $z$ = 2.88, the mechanism alone $z$ = 3.31; the real runs score above all 20 pools simulated with ends wasted; no experiment run |
| 9 | IT takes the choice that holds a prioritised skill; four measured outcome shares replace the equal split | rule assumed, tested on careers; shares fitted | prioritised targets' event hints read at the model's rate: 0.997 [0.975, 1.019] |
| 10 | Random card events are left out | not modelled | the targets they can hint read 1.24 of the model; the same targets on decks without such a card read 0.96 |
| 11 | Conditional character events are left out | not modelled | sparse pooled comparison inconclusive; Vodka's own Nimble Navigator and Mejiro Ardan's own Ramp Up are known misses |
| 12 | The parts compose into an exact $P(\text{all})$ under stated independence assumptions, with a lineage's expected shared wins standing in for their chance outcome | derived | an independent implementation reproduces the solver's odds on the 42 reference loop results (2e-15 at worst); the career report's real-lineage path has no independent check |
| 13 | On targets of unknown priority the model is optimistic where it rates targets likely, and its pooled hint chance reaches 1 only through sources it leaves out; on prioritised targets it is slightly low | tested | unknown priority, predicted 0.6 or more: 0.983 [0.976, 0.990]; from modelled sources alone the bands depart jointly, $p$ = 5.8e-14; pooled 0.997 [0.989, 1.005] on 34,730 rows; prioritised 1.014 [1.005, 1.024] on 10,404 rows, by band $p$ = 0.08 |
| 13 | Over all targets the model is slightly optimistic, from targets the player left off the list | tested | all rows 0.990 [0.985, 0.995]; targets shown not prioritised 0.953 [0.943, 0.963], 115% of the shortfall; the cause is not established |
| 13 | The model overstates the chance of landing every target in one career, $P(\text{all})$ | tested | spark-weighted all-target acquisition 0.86 [0.77, 0.96] of the model, worth 386 equally weighted runs, 0.89 without the 50 runs of highest predicted chance; runs showing no priority list 0.78, runs showing one 0.89, a difference of −0.11 [−0.32, +0.12] that is not detectable; scaling each target's chance to its own measured miss brings the acquisition part back to about 1 (in-sample, so consistent with the per-target misses compounding, not a test of it); counting every open target equally, no miss is detected; only 357 runs acquired every open target |

### Known misses

A known miss is a measured gap between the model and real careers that the model keeps. Each is measured in its
section. *How to read* says why a listed miss can be partly chance.

| Part | Miss | Observed / predicted | Section |
|---|---|---|---|
| Inheritance | the eight targets are inherited slightly less often than predicted, with no difference between them detected, and per roll years with an inspiration read short too (Tail Held High, the one target whose own interval excludes 1, reads 0.972 [0.947, 0.997]); section 3's parent weight or its affinity slope removes both, in-sample, not held out | 0.990 [0.982, 0.997]; per roll 0.986 | 3, 13 |
| Inheritance | parents pass sparks less often, most of their trials at affinity 200 or more; a flat parent shortfall and a weaker affinity effect fit alike | parents 0.970 [0.954, 0.985]; at affinity 200+ 0.964 [0.948, 0.981] | 3 |
| Affinity | the loop model's expected shared wins use base aptitudes and fall short of real careers', mostly in races the base table gives under 0.95 | 1.118 [1.114, 1.122] | 4, 5 |
| Inheritance | non-target skills, the shortfall spread over many skills | 0.979 [0.969, 0.990] | 13 |
| Races | aptitude basis <=70 on raised grades, before streak penalties; an extra cost of 3 points there predicts unseen runs better; the career planner applies it | 0.94 [0.93, 0.96] | 4 |
| Races | goal races 4th in a row, chosen races 1st and 2nd in a row: each under 1% off in wins; goal races 4th in a row lose more often than predicted (exact $p$ = 0.0034) | wins 0.995, 0.995, 0.996; losses 2.35, 1.053, 1.057 | 4 |
| Races | goal races 7th or later in a row, short of the prediction as the planner charges them (as the 6th) and also at the full penalty of 50 | 0.60 [0.46, 0.74] | 4 |
| Races | Trackblazer's goal races 4th to 6th in a row, on few attempts | 0.84 [0.72, 0.96] over 46 | 4 |
| Training hints | cards differ beyond their effects; cause open | per card 0.84-1.18 | 7 |
| Training hints | a skill also hinted by inspiration | 0.74 [0.70, 0.77] | 7 |
| Training hints | a skill another card's training also hinted | 0.95 [0.91, 0.99] | 7 |
| Training hints | a card's other pool skills, when inspirations hinted some of its pool, are hinted more often | card-adjusted, none inspired 0.968, two 1.062 | 7 |
| Training hints | Trackblazer's base over-predicts real careers | 0.746 [0.60, 0.89] | 13 |
| Training hints | non-target skills are under-predicted | 1.038 [1.028, 1.047] | 13 |
| Chain events | Inari One's careers reach fewer events than the pal-count budget predicts | own logit term −0.073, LR $p$ = 0.007 | 8 |
| Chain events | early links run short at 3 pal or group cards and long at 1 | $z$ = +2.8 and −3.1 | 8 |
| Chain events | with five chain cards, chains that never start are slightly over-predicted | 0.215 [0.183, 0.246] against 0.248 | 8 |
| Calibration | landing every target at once reads below the model | 0.86 [0.77, 0.96] | 13 |
| Calibration | prioritised targets are acquired slightly more often than predicted | 1.014 [1.005, 1.024] | 13 |
| Calibration | Ramp Up / It's On! and Risky Business / Nothing Ventured are acquired less often than predicted | 0.930 [0.91, 0.95]; 0.926 [0.89, 0.96] | 13 |
| Calibration | on targets of unknown priority, those predicted 0.6 or more are acquired less often than predicted | 0.983 [0.976, 0.990]; from modelled sources, bands jointly $p$ = 5.8e-14 | 13 |
| Calibration | golds a deck can give, where the target is not shown prioritised, are hinted less often than predicted | golds not predicted certain 0.958 [0.944, 0.973]; unknown priority 0.932, not prioritised 0.951, prioritised 0.994 | 13 |
| Event choices | outcome shares the model leaves at the equal rule read off it on most choices the records can read, and on most of those read without hint levels | 15 of 20 choices; without hint levels 5 of 9 | 9 |
| Event choices | Sasami Anshinzawa [This Might Sting!]'s adopted Nothing Ventured 0.75 is above her per-run share on the planner's premise, both skills prioritised | 0.59 [0.48, 0.69] | 9 |
| Event choices | Riko Kashimoto [Planned Perfection]'s adopted Ramp Up 0.75 is just above her per-run share when only credits at a level her event gives count | 0.72 [0.69, 0.74] | 9 |
| Left out | targets a deck card's random event can hint | 1.24 [1.11, 1.38] | 10 |
| Left out | Vodka's own Nimble Navigator | her events give it in 28 of 30 runs | 11 |
| Left out | Mejiro Ardan's own Ramp Up | her events give it in 9 of 9 runs; the model gives 3 of them no chance | 11 |

### Search caveats

These concern the planner's search, not the model's fit to careers (section 6).

| Caveat | Count |
|---|---|
| a listed runner-up's pair-ceiling odds pass its winner's | 2 of 13 runner-ups |
| loops the reported search pass turned down after scheduling them, which no bound here excludes | 222 in 18 of 42 results |

### Not settled

- Whether specialty priority causes more hints, and whether its weight differs by scenario. The term predicts unseen
  careers better ($z$ = 2.1, bootstrap $p$ =
  0.03), but on unseen cards it sits at the edge
  of detection ($z$ = 1.8 to
  2.1), and on cards whose limit breaks change only their specialty
  priority no effect fits as well. A weight per scenario does not predict detectably better than one shared weight
  ($z$ = 1.3) (section 7).
- The hint-frequency slope $1/100$ (**assumed**). A free slope does not predict unseen careers detectably better, with
  the specialty term ($z$ = 0.7) or without it
  ($z$ = 1.5) (section 7).
- Whether skills are hinted more often in decks with more pals. In-sample the factor is 1.026 per
  pal, but it does not predict unseen careers detectably better ($z$ =
  1.8) (section 7).
- Why the model is optimistic about likely targets. The not-prioritised rows read short of it, and the runs showing
  no list may hold such targets; that this explains the unknown-priority miss is an inference, not tested (section
  13). Landing every target falls short through that miss, compounded over the targets, and on runs showing no list
  through the gold tier as well; this check rests on 386 effective runs.
- How far targets a shown list leaves out fall short. The model reads a shown list only at chain-ending choices and
  assumes every other choice favours the target, so some shortfall there is expected by construction; no script
  computes how much (section 13).
- Why Trackblazer's training-hint base, fitted on section 7's 130 runs, over-predicts
  section 13's 113 Trackblazer careers. The two count different rows (section 13 scores only
  targets, at every limit break); the cause is not tested (section 13).
- How firmly the goal-race shift beats the community table across characters. With the error clustered by character
  its $z$ is 1.93, just short of 1.96, and three
  characters carry 61% of its margin. Counted per
  character, 22 favour the shift and
  5 oppose it (section 4).

### Assumed and untested

Spark generation (section 1) and that every hinted target is bought; the star mixes, and that a loop's veterans were bred in the scenario being planned (section 2); Grand Masters, which
has no runs; that pal events use no chain budget (section 8); hint levels; which targets each run prioritised, since most lists are not shown;
that the race table, tested on the races IT chose (section 4), holds on the schedules a loop fixes in advance (section
5); that a planned loop's expected shared G1 wins can stand in for their chance outcome inside the inheritance chance
(section 12); that when a higher-ranked prioritised skill sits on a rival choice, IT takes it (section 9); that a hint
given to a skill already at Lv 5 is not credited (section 9); that pal and group finales always fire (section 9); that
Tokai Teio's Head-On is always acquired (section 11); the loss scaling section 6 uses to price base grades; a score
straight-line between its floor and ceiling (section 2); the one-nat allowance for fitting a parameter (section 3); and
dependence between a run's targets beyond what section 12 models, apart from the inspirations of different targets,
where sections 3 (within one inspiration) and 13 (over a career) test it and detect none.

## How to read this document

**Labels.** **fitted**: estimated from the runs, with the sample and method given. **derived**: computed from the
other parts. **assumed**: a chosen rule or an outside figure, with its source named; no evidence is claimed for it
unless the text says it was tested. A **known miss** is a measured discrepancy the model keeps.

**Terms.**

- *Run*: one Independent Training (IT) career from uma-it, pooled. There are 8,940. Each section uses the
  runs its test can read and says how many: for example section 8 the runs whose decks have readable chain links, and
  section 13 the runs that can be rebuilt as careers, so the counts differ. *Row*: one observation within a run; a
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

**Fit comparisons.** A model's *log-likelihood* (LL) adds up the log of the probability it gave to what actually
happened; the higher, the better the model accounts for the careers. Differences are in *nats*. A model ahead by
$\Delta$ nats made the observed careers $e^{\Delta}$ times as likely as its rival did: 1 nat is a factor of about 2.7,
5 nats about 150. Some scores add up one run's rows as if they were independent: the training-hint rows and the
chain-allocation link scores. Such a sum is a *composite* likelihood. Its differences rank models but are no
likelihood ratio for whole careers. A *free* fit gives each quantity its own value with no formula, such as a win rate
per cell; it is the yardstick a formula is measured against. Each comparison is one of three kinds:

1. **Same number of fitted parameters**: compared by LL on the same data. This is descriptive; no significance cutoff
   applies to models that are not nested.
2. **Is an extra parameter detectable?** A likelihood-ratio test compares $2\Delta$ with $\chi^2$ on the number of
   extra parameters. That reference holds only for regular nested fits of a joint likelihood. Tests on composite
   likelihoods, on integer parameters such as a binomial's $n$, or at a parameter's bound are reported unadjusted.
   Their p-values are nominal, and nominal significance alone does not establish an effect.
3. **Does a model predict better?** *Held out* means five-fold cross-validation by run id: the runs are split into
   five groups, each model is fitted on four of them and scored on the fifth, and this is done five times, so every run
   is scored by a fit that never saw it. The table gives the summed difference in nats, with a ± standard error that
   treats each run's score difference as independent, and $z$, the difference divided by its error. For one
   comparison, $|z|$ above about 2 is unlikely to be chance; this document makes many, so a few will pass by chance.
   The error ignores the overlap of the training sets and the choice among models, so $z$ is approximate. A $z$
   short of significance does not show an alternative equal or worse. Section 7's covariate and per-card checks keep
   the full-pool specialty weights in every fold, refitting only the per-scenario levels, so they are partly held out.
   Section 7 also treats the card as a unit, since every row of one card shares the card's own error; its verdicts
   read the $z$ that counts both careers and cards as units.

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
score of 17,500, and 20 / 70 / 10% from 17,500 up. The page picks the band by the scenario being planned. That
**assumes** a closed loop: the veterans a loop reuses were bred in that same scenario.

### Findings

- **Grand Concert defaults to the 17,500+ mix; URA Finale, Unity Cup and Trackblazer to the lower one.** Each takes the
  band most of its careers reach, Unity Cup by a small margin. This is a chosen default, not a fit of the stars of the
  veterans players keep in a loop.
- **The scores behind that default are upper bounds, and Grand Concert's default is fragile.** The run list's score
  assumes every skill point spent; its lowest estimate assumes none, and with none spent no career in any scenario
  reaches 17,500. Taking a career's score as rising in a straight line between the two (**assumed**), Grand Concert
  keeps its majority at 17,500+ only if its careers realise at least
  92% of what their skill points could add.
  How much they realise is not measured. The shares at fixed offsets below the ceiling are descriptive: the offsets
  were chosen after a first look at the data.
- **The runs' legacies carry more stars than the lower mix**, in every scenario. Players keep veterans for their
  sparks, and many were bred in a different scenario from the career they serve, so these legacies do not measure the
  mix of an unselected veteran. Even so, Grand Concert's own legacies read a mean base below the 17,500+ mix's that
  its default uses. Both this and the score threshold point the same way: the 17,500+ mix may overstate Grand Concert's
  stars.
- **Averaging over stars slightly overstates the inheritance chance** (**derived**). On the runs' lineages, over every
  pattern of which legacies carry the target, the largest overstatement is
  1.61%, with 1 parent
  carrying it, under the assumed mixes. It is no bound for every lineage and says nothing of joint landing odds.

### Evidence

*Which band?* A career's rank score is the run list's estimate with every skill point spent, over every completed run.
That is the most the career could score. So the table also gives the share with every score taken 250, 500 and 1,000
points lower (offsets chosen after a first look at the data), and each scenario's median score.

| Scenario | Runs | Median score | Share at 17,500+ [95%] | 250 lower | 500 lower | 1,000 lower | Band |
|---|---|---|---|---|---|---|---|
| URA Finale | 1,357 | 12,761 | 0.000 [0.000, 0.003] | 0.000 | 0.000 | 0.000 | below |
| Unity Cup | 3,155 | 17,428 | 0.467 [0.450, 0.484] | 0.351 | 0.239 | 0.094 | below |
| Grand Concert | 14,742 | 18,094 | 0.794 [0.787, 0.800] | 0.684 | 0.554 | 0.292 | 17,500+ |
| Trackblazer | 282 | 12,386 | 0.000 [0.000, 0.013] | 0.000 | 0.000 | 0.000 | below |

Grand Masters has no runs and is **assumed** 17,500+. The run list's lowest estimate, with no skill point spent, puts
no career of any scenario at 17,500.

The two mixes give a legacy a mean base per roll of 0.0465 (below 17,500) and
0.0570 (17,500+). Mixing the two by each scenario's share at 17,500+ would give Unity
Cup 0.0514 and Grand Concert 0.0548.
Grand Concert's own legacies read 0.0564 [0.0563, 0.0565],
below the 0.0570 of the 17,500+ mix its default uses.

*What stars do the runs' legacies carry?* These are the skill-white sparks of the six legacies in the run records, by
the scenario of the career they serve. A spark counts once per run, so a veteran lent to many careers counts many
times. The interval is clustered by run.

| Scenario | White sparks | 1★ | 2★ | 3★ | Mean base [95%] |
|---|---|---|---|---|---|
| URA Finale | 55,725 | 34% | 58% | 7% | 0.0519 [0.0516, 0.0522] |
| Unity Cup | 126,473 | 32% | 60% | 8% | 0.0530 [0.0528, 0.0532] |
| Grand Concert | 357,316 | 23% | 67% | 11% | 0.0564 [0.0563, 0.0565] |
| Trackblazer | 6,755 | 26% | 65% | 9% | 0.0549 [0.0543, 0.0554] |

*What does averaging over stars cost?* A planned loop does not know each legacy's future stars, so the model gives
her the average base rate $\bar b$ of her star mix. That is slightly generous. A legacy keeps the same stars in both
inspiration rolls, so her chance of firing in neither is $E[(1 - r_b)^2]$, averaged over her possible base rates $b$.
Here $r_b = \min\{1,\ b(1 + A/100)\}$ is her chance per roll at affinity $A$ (section 3). The model uses
$(1 - r_{\bar b})^2$ instead. Because $(1 - r_b)^2$ is convex in $b$ (the cap at 1 keeps it so), its average is at
least its value at the average: $E[(1 - r_b)^2] \ge (1 - r_{\bar b})^2$. So averaging understates the chance she never
fires, and overstates $q$, the chance of at least one inspiration of the skill.

The table measures the overstatement at the six real affinities of each of the 8,926 lineages in the
runs. It does so first for a target all six legacies carry, then for the pattern of carriers, among every subset of the
six, that overstates most in that lineage. It treats each legacy's stars as independent of the others'; a veteran
appearing twice in a lineage would share hers.

| Band | All six carry: mean | All six carry: largest | Worst pattern: mean | Worst pattern: largest | Its carriers |
|---|---|---|---|---|---|
| below 17,500 | 0.51% | 0.57% | 1.28% | 1.61% | 1 parent |
| 17,500+ | 0.30% | 0.33% | 0.88% | 1.12% | 1 parent |

With real legacies the model uses their actual stars, so this does not arise.

## 3. Inheritance rolls

Computed by `research/s03_inheritance.py` (the roll's parts) and `research/s13_calibration.py` (whole careers).

The roll tested is the community formula (`docs/MODEL.md`). At each of the two inspirations, a legacy carrying the
skill passes it with chance $r = \min\{1,\ b(s)(1 + A/100)\}$. Here $b(s)$ is the base set by the spark's stars $s$,
3 / 6 / 9% for a skill white, and $A$ is the legacy's affinity (section 5). Over the career, the chance of at least one
inspiration of the skill is $q = 1 - [\prod_l (1 - r_l)]^2$ over the six legacies $l$. This assumes firings
independent across legacies and inspirations, given the stars and affinities. The odds use skill whites only.

The parts are **fitted** on *trials*. A trial is one ancestor's white spark in one year when she is the only one of the
six legacies carrying the skill, so an inspiration of it that year can only have come from her. The trial *passes*
when the spark fires. There are 70,506 parent trials and 166,540 grandparent trials of ordinary ancestors; a trainee's own
character as a grandparent is section 5's. The fits leave out the 15,380
trials of parents whose own parent is the trainee's character, since their affinity rests on section 5's rule; they
are reported on their own.

Affinity and generation barely overlap. No grandparent trial has an affinity above
97, while 85% of parent
trials sit at 200 or more (median 245). Below affinity 200,
94% of trials are grandparents'. So over all trials, a test of how
affinity scales the roll is mostly a comparison of parents with grandparents. Two one-parameter changes are tested
against the model: a *parent weight* $w_P$ multiplies every parent's roll, $r = \min\{1,\ w_P\, b(s)(1 + A/100)\}$, and
a *slope* $s$ scales affinity, $r = \min\{1,\ b(s)(1 + sA/100)\}$. The model has $w_P = s = 1$.

### Findings

- **Grandparents pass sparks at the model's full base rate.**
- **Within one inspiration, no per-ancestor trigger is detected.** A trigger would make an ancestor active in an
  inspiration with some chance $g$, her sparks then firing with chance $r/g$: each spark's own chance stays $r$, but a
  pair she carries shows together $1/g$ times as often as the product predicts, while pairs on two ancestors stay at
  the product (derived). Pairs of single-carrier skill whites in the same career year, each chance scaled by her
  generation's observed / predicted, show together 0.995 [0.969, 1.020]
  times the product on one ancestor and 0.996 [0.980, 1.012] on two
  (8,562 runs, run-clustered bootstrap). So a trigger with $g$ below
  0.981 is excluded. This tests one kind of dependence, through
  pairs; it does not show the firings independent in general.
- **Known miss: parents pass sparks less often than the model predicts, and the trials cannot say why.** In sample,
  two one-parameter changes are each detectable (rule 2): a parent weight below 1 and an affinity slope below 1. Over
  all trials the parent weight fits slightly better, since a slope also lowers the grandparents; on parents alone, the
  one generation whose affinities span a wide range, the slope fits slightly better (rule 1). Fitted together, on all
  trials or on parents alone, neither adds a detectable gain to the other, so a shortfall of parents and a weaker
  affinity effect cannot be told apart. The two differ for the planner: a slope makes the shortfall grow with affinity,
  a flat weight does not. Held out, neither improvement is established
  (rule 3), but that test lacks the power to confirm a change of this size. If either change were real at its
  in-sample size, a held-out test would be expected to give only $z$ = 1.76
  (parent weight) or 1.65 (slope), short of 1.96. That expectation is the
  in-sample gain less one nat for fitting one parameter (**assumed**), divided by the held-out error. The held-out
  gains found are close to it. The
  model keeps the formula and this miss.
- **Among grandparents alone, where generation does not vary, no slope away from 1 is detected.** Their affinities
  cover only 0 to 97.
- **Each part of affinity reads below its weight in sample, through the parents.** With the other part at 1, the
  relation points' and the shared wins' slopes each have an interval below 1. With a parent weight fitted, neither part
  adds a detectable gain. Held out, neither refit is established.
- **Parents whose own parent is the trainee's character read low too.** This is the structure of the planner's
  rotations, scored with section 5's rule.
- **Below affinity 200 the tests mostly re-test the grandparents.** Neither a common weight nor a slope fitted there
  moves detectably from 1, and each star count passes sparks at its base rate. These tests cannot tell the formula from
  the parents' shortfall: the slope's interval there also covers the slope fitted on all trials, and the parents below
  200, the only parents there, read 1.010
  [0.966, 1.055] over
  10,624 trials, which covers both 1 and the fitted parent weight. Over all trials,
  1 of the three star counts (2★, in the table
  below) reads low, and only through its trials at affinity 200 or more.
- **Whole careers read slightly short of $q$, with no difference between targets detected.** Over the eight targets
  the ratio, 0.990 [0.982, 0.997], lies below 1. Against one common ratio the targets do not differ
  detectably (run-clustered $\chi^2$, $p$ = 0.17); that test has little power against a single
  target off the rest, so it does not show the targets alike. Tail Held High is the one whose own interval excludes
  1; Risky Business reads 0.946 with a wider interval. Per roll, over every
  target, years with an inspiration read 0.986 of the model ($z$ =
  −3.25). One band of predicted $q$ lies outside its pointwise interval.
- **A parent shortfall accounts for the whole-career shortfall, in either form.** Each form is refitted on the
  trials above and scales the careers' own lineages: the flat parent weight (0.968
  [0.953, 0.984], the same estimate as above) scales each parent's roll, the affinity slope
  (0.963 [0.945, 0.982]) scales every legacy's affinity. The careers cannot tell the two
  apart either:

  | Whole-career check | The model | Parent weight | Affinity slope |
  |---|---|---|---|
  | every target | 0.990 | 0.999 [0.991, 1.006] | 1.002 [0.994, 1.009] |
  | targets with two or more carriers | 0.991 | 0.999 [0.992, 1.007] | 1.002 [0.995, 1.009] |
  | Tail Held High | 0.972 | 0.981 [0.956, 1.007] | 0.985 [0.959, 1.010] |
  | per-roll years with an inspiration, every target, $z$ | −3.25 | −0.23 | +0.64 |
  | $q$ band 0.40-0.60 (mean prediction) | 0.495 (0.507) | 0.503 (0.508) | 0.504 (0.508) |
  | non-target skills, two or more carriers | 0.982 | 0.993 [0.981, 1.005] | 0.996 [0.984, 1.008] |

  Both forms are fitted on careers where one legacy carries the skill. A row with one carrier is such a trial from the
  same career, so the rows with two or more carriers are the fairer check; they are still the same careers, so this is
  in-sample, not held out. The model keeps the formula.

### Evidence

"Held out" is rule 3 of *How to read*: five folds by run, positive nats favouring the refit. Each part's weight is
fitted with the other part at 1. The carrier, spark-year and predicted-$q$ band checks use every target, all
66,984 rows; leaving out Tail Held High, the one target whose own interval excludes 1, would
be a selection.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Grandparents take the full base | weight 0.99 [0.98, 1.01] | full base beats half by 3,073 nats | full base |
| Affinity raises the roll | | beats no affinity by 8,679 nats | kept |
| Affinity slope is 1 | 0.963 [0.945, 0.982] | in sample 7.4 nats, LR $p$ = 1e-04; held out 6.1 ± 3.9 nats, $z$ = 1.58 | below 1 in sample; not established held out |
| Parent weight is 1 | 0.968 [0.953, 0.984] | in sample 8.1 nats, LR $p$ = 5e-05; held out 7.5 ± 4.1 nats, $z$ = 1.84 | below 1 in sample; not established held out |
| Slope or parent weight | together: weight 0.978, slope 0.986 | the slope adds 0.27 nats to the parent weight, the weight 1.01 nats to the slope | cannot be told apart |
| Slope or parent weight, parents alone | slope 0.954 [0.932, 0.976]; together: weight 1.218, slope 0.709 | in sample: weight 8.1 nats, slope 8.5; the slope adds 1.67 nats to the weight, the weight 1.27 to the slope | cannot be told apart |
| Slope among grandparents alone | 0.989 [0.952, 1.026] | interval covers 1 | none detected |
| Relation-point weight is 1 | 0.89 [0.83, 0.95] | held out 5.8 ± 3.6 nats, $z$ = 1.58; with the parent weight fitted, 1.00, adding 0.00 nats | below 1 in sample, through the parents |
| Weight of 3 per shared G1 win is 1 | 0.95 [0.92, 0.97] | held out 5.9 ± 3.9 nats, $z$ = 1.52; with the parent weight fitted, 0.98, adding 0.45 nats | below 1 in sample, through the parents |
| Parents whose own parent is the trainee's character | 0.952 [0.919, 0.985] of the model, 15,380 trials | interval below 1 | short too |
| Below affinity 200: one weight on every roll | 0.996 [0.982, 1.011] | interval covers 1 | none detected; mostly the grandparents |
| Below affinity 200: a slope | 0.994 [0.961, 1.027] | interval covers 1 and the slope fitted on all trials | none detected; cannot separate |
| Parents below affinity 200 | 1.010 [0.966, 1.055] of the model, 10,624 trials | interval covers 1 and the fitted parent weight | cannot separate |
| Affinity bands below 200 fit | | $\chi^2$ = 0.8, $p$ = 0.94 over four bands; 94% of their trials are grandparents' | no band detected |
| All five affinity bands fit | | $\chi^2$ = 19.0, $p$ = 0.002 | known miss at 200+ |
| Careers with 1 to 6 carriers | 0.967-0.9997 of the model, every point estimate below 1 | 1 of 6 pointwise intervals exclude 1 | slightly short together |
| Years with an inspiration of the skill | 0.986 of the model | $z$ = −3.25 | known miss; the parent weight or the affinity slope removes it, in-sample |
| $q$ by predicted band | all rows 0.990 [0.982, 0.997] | $\chi^2$ = 10.5, $p$ = 0.06 over the 5 bands above 0, a test with no direction; the pooled ratio is the one-degree-of-freedom reading | slightly short; one band outside its pointwise interval |

*By stars and by affinity.* Over all trials two groups lie outside their own interval: the 2★ count, and affinity
200+, where every trial is a parent's. Below affinity 200 every star
count's interval covers 1, so the 2★ shortfall comes from its trials at 200 or more.

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
given, and stats are not modelled. Grades are each outfit's own, from the game data. The planner's data hold the first
outfit's per character, which differ from her own for three Global outfits, Vodka [Fiery Aqua Vitae], Daiwa Scarlet
[Nuit Étoilée de Scarlet] and Haru Urara [New Year ♪ New Urara!]: 4,976 attempts
here. The data also list each such outfit's own, which the career planner uses; the loop planner keeps the first's.

### Findings

- **A goal race takes the penalty of the race before it, and still counts toward the position of the races after it.**
  The form was chosen by looking at all the runs. Scored on runs left out of each fit, it predicts better than the
  community table and better than leaving goal races out of the count, and it holds in the Classic and Senior years
  alike. Most characters' runs favour it over the community table, but a few carry much of the margin, so with the
  error counted by character its edge is borderline. Why the game does this is not known.
- **Grades must include inherited aptitude sparks.** On base grades, which a planned loop uses, the table predicts too
  few wins. Pink sparks raise grades by the rule in `docs/MODEL.md`: at the start of a career, and from the April of
  each inspiration that fires one.
- **The streak penalty is kept.** The previous race's result shows no detectable further effect.
- **The table is kept, with a shortfall at the lowest aptitudes.** A free win rate for every cell has no clear held-out
  advantage. One extra aptitude cost on the lowest basis does, even allowing for that basis having been picked after
  its miss was seen. On raised grades, which the career planner uses, the planner charges that cost. On base grades,
  which a planned loop uses, the table errs the other way there, predicting too few wins, and the loop planner keeps
  the table.
- **Calibrated at every streak position to the 6th: inside the interval, or within 1%.** One rule is applied to every
  position, goal races 1st to 6th in a row and chosen races at every position, and every position meets it. Three meet
  it only through the 1% allowance, missing their interval by under 1% in wins: goal races 4th in a row, and chosen
  races 1st and 2nd in a row. They are listed as known misses. The 1% was chosen after these data were seen.
- **Known misses.** A race's *aptitude basis* is 110 less her two aptitude costs, its win chance before any streak
  penalty. Races with basis <=70 win less than
  predicted. Goal races 7th or later in a row win less than predicted, both as the planner charges them (as the 6th) and
  at the full penalty of 50: the planner's cap on the streak count adds to the miss but does not explain it.
  Trackblazer's goal races 4th to 6th in a row win less than predicted, on few attempts.

IT's chosen races won at these rates, with the community table's chance in parentheses. Grades include those raised by
inherited aptitude sparks; S is shown as A, since both cost 0. A dot marks a cell with under 30 attempts. Only grade
pairs with 200 or more attempts are shown.

| Races in a row | AA | AB | AC | BB | AD | BC | AE | BD | CC | BE | CD | AF | CE | DD | BF | DE | AG |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 race | 100% (100%) | 100% (100%) | 89% (90%) | 90% (90%) | 80% (80%) | 78% (80%) | 67% (70%) | 70% (70%) | 61% (70%) | 55% (60%) | 57% (60%) | 47% (50%) | 43% (50%) | 45% (50%) | 39% (40%) | 28% (40%) | 19% (20%) |
| 2 races | 100% (100%) | 100% (100%) | 90% (90%) | 91% (90%) | 79% (80%) | 79% (80%) | 61% (70%) | 70% (70%) | 74% (70%) | 50% (60%) | 63% (60%) | 52% (50%) | 41% (50%) | 60% (50%) | 34% (40%) | 31% (40%) | 19% (20%) |
| 3 races | 100% (100%) | 95% (95%) | 85% (85%) | 80% (85%) | 74% (75%) | 73% (75%) | 57% (65%) | 58% (65%) | · (65%) | 43% (55%) | · (55%) | 54% (45%) | · (45%) | · (45%) | · (35%) | · (35%) | 12% (15%) |
| 4 races | 90% (90%) | 80% (80%) | 73% (70%) | 77% (70%) | 61% (60%) | 60% (60%) | 45% (50%) | 50% (50%) | 61% (50%) | 26% (40%) | 31% (40%) | 23% (30%) | · (30%) | · (30%) | · (20%) | · (20%) | · (0%) |
| 5 races | 80% (80%) | 67% (70%) | 56% (60%) | 58% (60%) | 66% (50%) | 44% (50%) | 32% (40%) | · (40%) | · (40%) | · (30%) | · (30%) | 19% (20%) | · (20%) | · (20%) | · (10%) | · (10%) | 0% (0%) |
| 6 or more | 58% (60%) | 54% (50%) | 49% (40%) | · (40%) | · (30%) | · (30%) | · (20%) | · (20%) | · (20%) | · (10%) | · (10%) | · (0%) | · (0%) | · (0%) | · (0%) | · (0%) | · (0%) |

### Evidence

The runs record 197,254 attempts at the 34 single-race G1s, 55,323 of them goal races.

The table makes some races certain, 0% or 100%, yet a handful went the other way, which would give it a
log-likelihood of $-\infty$. So every model compared has its 0% and 100% pulled in to a fitted floor and ceiling. With the same
two parameters in each, the in-sample comparisons follow rule 1 of *How to read*.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Raised grades, not base | wins observed / predicted: base 1.067 [1.065, 1.069], raised 0.997 [0.996, 0.998]; losses: base 0.525 [0.515, 0.535], raised 1.047 [1.033, 1.061] | raised fits 4,958 nats better | raised grades |
| A pink counts from the April of its inspiration | won 0.748 [0.718, 0.777] of 892 Senior January-March attempts where the timing matters | predicted 0.758 from April, 0.868 from the year's start | from April |
| The streak penalty is real | | without it, 4,845 nats worse | kept |
| A loss changes the next race's chance | after a loss over after a win, 4th race on: 1.007 [0.977, 1.038] | run-clustered; at the 2nd and 3rd the interval also covers 1 | not detected |
| Goal races pay one position less | community table, goal races 4th on: 1.135 [1.123, 1.147] | held out, forms table below | shift adopted |
| No free rate per cell does better | | a free rate per chance, streak position and race kind, 111 cells, each pulled toward 1/2 by one attempt, held out: 44 ± 30 nats, $z$ = 1.46; pulled toward the table's chance instead, and kept off 0% and 100%: $z$ = 1.51 | table kept |
| Aptitude basis <=70 wins less (grouped before streak penalties) | 0.942 [0.928, 0.957] over 15,946 attempts | interval below 1 | known miss |
| That shortfall is real | an extra cost of 3 points on that basis, the same in every fold; fitted on all runs it brings the bucket to 0.999 [0.984, 1.014], in sample and so close to 1 by construction | held out against the table: 29 ± 8 nats, $z$ = 3.62. The bucket was picked after its miss was seen, so the bar is raised for the five buckets it could have been (Bonferroni): $z$ > 2.58 | known miss of the table; the career planner applies the cost |
| Where base grades fall short | wins observed / predicted on base grades by basis: 110 1.000, 100 1.006, 90 1.067, 80 1.160, 70 or less 1.77; losses 1.08, 0.75, 0.46, 0.47, 0.51 | | used by section 6 |

By scenario, every interval covers the prediction once chosen races 1st and 2nd in a row are left out.

*Goal races.* Under the community table, goal races from the 4th race in a row win more than predicted. The forms
below were written down after looking at all the runs. Each was then scored on runs left out of the fit, five folds by
run over 8,875 runs. The folds refit only each form's floor and ceiling, so this tests how well each
fixed form predicts new careers. It does not test the choice of form on fresh data.

| Form | Goal race at position $j$ pays | Races after it count it | Held out, against the adopted form |
|---|---|---|---|
| Adopted | $\mathrm{PEN}_{\max(j-1,1)}$ | yes | |
| Community table | $\mathrm{PEN}_j$ | yes | 249 ± 38 nats worse, $z$ = 6.55 |
| Exclusion | $\mathrm{PEN}$ at the chosen races before it | no | 1,987 ± 81 nats worse, $z$ = 24.58 |

Runs of one character share her goal calendar, and the goal races 4th or later in a row fall on few calendar slots.
So the forms were also scored with every run of a character held out together (65
characters), the standard error clustered by character. Nothing calendar-specific is fitted, so this mostly asks how
much of the margin rests on a few characters. The adopted form still beats exclusion,
2,002 ±
592 nats, $z$ =
3.38. Against the community table it gains
203 ±
105 nats, $z$ =
1.93, just short of 1.96. The margin is uneven: of the
by-run margin, three characters carry 61%. A sign
test asks whether most characters lean the same way, whatever their size: of the characters whose summed difference
exceeds one nat, 22 favour the shift and
5 oppose it, $p$ =
0.0015. Most characters favour the shift; a few carry its size. By career year the shift holds in both: goal races 4th to 6th in a row win
0.997 [0.980, 1.014]
of the prediction in Classic and 0.994 [0.982, 1.007]
in Senior, against 1.147 and
1.136 under the community table. By scenario it holds in all
but Trackblazer, whose 46 such attempts win
0.84 [0.72, 0.96]
of it, while the community table's interval there covers its prediction
(0.95 [0.81, 1.09]).
That is one cell of four, so it may be partly chance.

*Calibration by streak position.* One rule is applied to every position, goal races 1st to 6th in a row and chosen
races at every position: the prediction must lie inside the 95% interval of the observed wins, or within 1% of them.
Goal races 7th or later are a separate known miss. The 1% is a fifth of the table's smallest step, a streak penalty of
5 on a chance near 100%, and it was chosen after these data were seen. It is not small in odds: with every scheduled
win chance 1% lower, a reference winner's odds move by up to
0.92%, more than the smallest gap between a listed
runner-up and its winner, 0.46%. Applied to every listed loop
alike, it reorders none (section 6). Every position passes, the 6th goal race on its interval alone: it wins
0.989 of the prediction. Those outside their interval
(goal races 4th in a row; chosen races 1st in a row; chosen races 2nd in a row) miss it by under 1% in wins, a known miss. The overall interval also
misses 1, by under 1%.

Wins can hide a gap in losses. Most attempts are near-certain wins, so a win ratio barely moves even when the table
predicts the few losses badly. This section therefore also reports observed losses over predicted losses. They show the
size of a gap, not a separate test: a loss interval excludes 1 when the win interval does. Goal races 4th in a row lose
2.35 times as often as predicted, while their wins are
0.995 of it. An exact count test of those losses, treating attempts as independent,
gives $p$ = 0.0034: the shift is slightly too generous there. For the planner
the win ratio is the one that matters, since shared wins and affinity are built from win chances.

Goal races 4th to 6th in a row win 0.995
[0.985, 1.005] of
the prediction, against 1.140 under the community table. Goal races
7th or later fall well short as the planner charges them, as the 6th:
0.60 [0.46, 0.74].
At the full penalty of 50 they still fall short,
0.81 [0.63, 1.00],
so the cap adds to this known miss but does not explain it.

| Race in a row | Goal races: observed / predicted [95%] | Attempts | Chosen races: observed / predicted [95%] | Attempts |
|---|---|---|---|---|
| 1st | 1.000 [1.000, 1.001] | 29,999 | 0.995 [0.994, 0.997] | 68,730 |
| 2nd | 1.000 [1.000, 1.001] | 15,068 | 0.996 [0.994, 0.998] | 51,432 |
| 3rd | 0.999 [0.997, 1.001] | 6,463 | 0.998 [0.994, 1.001] | 13,362 |
| 4th | 0.995 [0.990, 0.999] | 1,602 | 0.998 [0.987, 1.008] | 6,071 |
| 5th | 0.996 [0.976, 1.017] | 1,623 | 0.994 [0.966, 1.023] | 1,817 |
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
inheritance over the joint race outcomes. Section 3 tests the 3 per shared win on the runs' recorded shared wins. Base
aptitudes predict too few wins (section 4); the Evidence below measures what that does to $o$.

### Findings

- **A trainee who is her own grandparent gets no trio points from it**: $\mathrm{tri}(T,P,T) = 0$ (**fitted**). The
  game data do not say how a group counts one character twice. Counting it as $\mathrm{pair}(T,P)$, as one community
  tool does, fits 43 nats worse.
- **On base aptitudes the expected shared wins are too low**, mostly in races the base table gives under 0.95
  (**tested**). Two careers of different characters running the same race both win it more often than their
  base-table chances multiply to. The shortfall grows as the base chance falls; where both chances are 0.95 or more it
  is small (band edges chosen). This error works against the parents' known miss of section 3, by an amount that
  depends on where the loop model's races fall. With each record's shared wins cut by the pooled ratio, parents and
  grandparents alike would read above the model. Cut by the ratio where both chances are 0.95 or more, parents read
  below it, as they do now. The loop model's own schedules were not measured. They are built to maximise the summed
  base products, so it is likely, but untested, that they favour races where both chances are high, nearer the second
  case. The legacies' own careers are not in the records, so whether their wins
  follow this ratio is not known. Section 13 does not test any of this, since it rebuilds each lineage from its
  recorded shared wins. The effect on a loop's $P(\text{all})$ is not computed.

### Evidence

The fit uses 271,842 trials as section 3 defines them, self-grandparents included. In 1,557 of them a
self-grandparent passed her spark. The distance is observed minus predicted passes, in run-clustered standard
deviations.

| Claim | Estimate | Test | Verdict |
|---|---|---|---|
| $\mathrm{tri}(T,P,T) = 0$ | 1,582.3 passes predicted | distance −0.68 | kept |
| $\mathrm{tri}(T,P,T) = \mathrm{pair}(T,P)$ | 1,827.2 passes predicted | distance −7.10 | rejected |
| a fraction $\kappa$ of $\mathrm{pair}(T,P)$ | $\hat\kappa = 0$, its lower bound | | 0 kept |

The gap is not the ordinary trials' own misfit: refitting the affinity slope on them, to 0.96, leaves both verdicts
unchanged.

*What do base aptitudes do to shared wins?* Each row is one career and one single-race G1 it ran, 188,033
rows. The prediction is section 4's adopted table on base aptitudes, the one the loop model uses. For a pair of careers
at the same race, $o$ adds the product of their two chances; the second row compares that product with how often both
won, over every such pair of careers of different characters. Its interval is a jackknife over groups of runs.

| What is compared | Observed / predicted [95%] |
|---|---|
| One career wins a race it ran, at least once | 1.069 |
| Two careers both win the same race | 1.118 [1.114, 1.122] |
| ... both base chances below 0.80 (34,680 rows) | 3.323 [3.199, 3.447] |
| ... both 0.80-0.95 (18,729 rows) | 1.141 [1.132, 1.150] |
| ... both 0.95 or more (134,624 rows) | 1.0007 [1.0004, 1.0010] |

Shared wins make up 67% of a parent's affinity in the runs' lineages and
77% of a grandparent's. Dividing each record's shared wins by the pooled
pair ratio, as the loop model would see them, lowers the predicted passes to
0.950 (parents) and
0.967 (grandparents) of their values. The
table gives the ordinary trials at both ends: the pooled ratio, and the ratio where both base chances are 0.95 or more.

| Ancestors | Observed / predicted, pooled ratio [95%] | Observed / predicted, 0.95+ ratio [95%] |
|---|---|---|
| Parents | 1.020 [1.004, 1.036] | 0.970 [0.955, 0.985] |
| Grandparents | 1.028 [1.013, 1.044] | 0.995 [0.980, 1.010] |

Each column applies one ratio to every pair in a lineage, and the races are those IT chose, not the loop model's
schedules. The intervals leave out the pair ratio's own uncertainty.

## 6. Loop structures, ranking and schedules

Computed by `research/s06_loops.py`. The structures, their lineages $(T, P_1, P_2, X)$ and their ranking by mean
$P(\text{all})$ are defined in `docs/MODEL.md` and `docs/SOLVER.md`. $X$ is the uma each parent was bred from with the
other parent; a rotation has $X = T$.

### Findings

- **The rotation occurs in practice**: 168 runs have its exact lineage shape.
  More broadly, 2,932 of 8,940 runs have a grandparent of the trainee's own character.
- **Every pair of umas carries the same weight in a structure's summed affinities**, so the schedules maximise the total
  expected shared wins. That total stands in for the landing odds, which depend on affinity nonlinearly; equal weights
  do not make the two equivalent.
- **With each loop's reported decks fixed, scheduling for shared wins costs little**: on the reference results, no
  schedule could raise a loop's odds by more than 2.32%. That rests on the
  solver's pair ceilings, which its own tests check against an independent calculation; the loose ceiling this
  section computes itself would allow up to 24.4%.
- **Base grades make the planner understate the odds; the losses tried reorder no listed top.** Section 4 finds that
  on base grades real careers lose 0.53 times as often as the table says:
  0.51 at basis 70 or less, 0.47
  at 80, 0.46 at 90, 0.75 at 100 and
  1.08 at 110.
  With every scheduled race losing 0.5 times as often as the chart says
  (**assumed**), the winners' odds rise by
  1.3-6.3%:
  if that scaling held, the planner's odds would be about that much too low. At basis 110, where many scheduled races
  may fall, base grades lose slightly more often than predicted, so the planner's real error there is not measured.
  Losses were scaled by 0.5 and by 0, on every race or only at the
  lowest basis, which moves loops unevenly. Under none of these does a listed runner-up pass its winner. Section 4's
  ratios by basis were not applied as measured, and loops outside the listed tops were not rescored.
- **Known miss: the pair ceilings do not certify every listed top.** On 2 results a listed runner-up's ceiling passes its winner's odds, by up to 1.8%; it would need 0.46-2.07% over its schedule found to tie, which the search did not find and the bound cannot exclude.
  In the search pass each result reports, the search also turned down
  222 loops in 18 of the
  42 results after building their schedules, up to
  51 in one result. Each failed against the last listed place as it stood
  at that point, which only rises, and not against the final listing or its winner. So some would be excluded anyway.
  No bound here excludes them, and how many a better schedule could have kept is not known.

### Evidence

*Race weights* (**derived** from section 5's $A_P$ and $A_G$). Shared G1 wins raise affinity, and the schedules decide
which pairs of umas share them, so the question is how much each pair's shared wins count. In one lineage, the parents'
shared wins $o(P_1,P_2)$ enter the six affinities 6 times: twice in each $A_P$ (as
$o(P,P')$, and again in the sum over $P$'s own parents, since in a loop the other parent $P'$ is one of them) and once in
the $A_G$ of each parent's own parent who is the other parent. $o(P_1,X)$ and $o(P_2,X)$ enter
2 times each. Summed over a loop's 12 lineages or a
rotation's 3, every pair gets the same weight:

$$\text{loop: } 6\cdot 2 + 2\cdot 4 = 20, \qquad \text{rotation: } 6 + 2 + 2 = 10.$$

*The schedule bound* (**derived**). More shared wins never lower the odds: each $o$ enters affinity with weight +3,
and every later step only grows with its inputs. So, with a loop's decks fixed, no schedule can beat the odds computed
with every pair's shared wins set to an upper bound. The bound used is the solver's pair ceiling (`docs/SOLVER.md`,
`pair_ceilings`), with outfits reselected and decks fixed. Each pair's ceiling holds separately, and the pairs need not
reach theirs jointly, so the bound is conservative. The ceilings come from the search code that chose the schedules.
The script checks only that each lies between the overlap found and a loose independent ceiling: every G1 offering at
its best chance, with no streak penalty. That loose ceiling would allow gains of
6.9-24.4% on the winners, so it
cannot carry the bound. What carries it is the solver's own check: its test `pair_ceilings_match_milp`
(`pkg/loopopt/src/tests.rs`) compares the code that computes the ceilings with an independent integer program
(`val/milp_fixtures.py`) on 2,278 pairs. That test runs on frozen test
data, so it checks the method, not the current ceilings table, which the same code computes.

The bound is computed on the 42 reference results, the best loops and decks found for the validation setups
(`val/ref__*`, `val/refpu__*`). An independent implementation reproduces their odds to within 2e-15.

| On the reference results | Value |
|---|---|
| winners' odds gained by the shared wins found, over none | 31-107% |
| most any other schedule could add to a winner, by the pair ceilings | 0.15-2.32%, median 0.87% |
| listed runner-ups whose pair-ceiling odds reach their winner | 2 of 13 |
| most any other schedule could add to a winner, by the loose independent ceiling | 6.9-24.4% |
| loops the reported pass turned down after their schedule was built | 222, in 18 results |
| winners' odds with every scheduled loss chance scaled by 0.5 (assumed) | +1.3 to +6.3%, median 3.1%; no listed runner-up passes its winner under any of 4 loss scalings (0.5 and 0, on every race or at the lowest basis only) |
| winners' odds with every scheduled win chance 1% lower | change up to 0.92%, median 0.61%; the smallest gap from a listed runner-up to its winner is 0.46%; no listed top reordered |

This bound does not certify the ranking when schedules and decks change jointly. The loss scalings are assumed sizes;
section 4's ratios by basis are not applied as measured, and only the listed loops are rescored.

## 7. Training hints

Computed by `research/s07_hints.py`.

A card's training can hint the skills in its *pool*. $hf$ is the card's Hint Frequency in percent and $\mathit{sp}$ its
Specialty Priority, both at maximum limit break with unique effects included.

### Findings

- **A card makes a nearly fixed number of uniform draws from its pool.** A skill already at hint level 5 is no longer
  drawn. Draws that could also pick the card's stat option, as often as each skill, fit unseen careers and unseen
  cards worse, so the model leaves the stat option out.
- **The model assumes hint frequency scales that number directly**: it reads Hint Frequency +40% as 40% more hints.
  A free slope does not predict unseen careers detectably better.
- **Skills are hinted more often by cards with more specialty priority.** The association predicts careers it was not fitted to, but
  on cards it was not fitted to it sits at the edge of detection, and it is not shown to be a cause. The model fits a
  weight per scenario.
  The four weights differ by more than chance in the fitted data, but a weight per scenario does not predict unseen
  careers detectably better than one shared weight, and each weight is uncertain (see *Fitted values*).
- **Known misses**: cards differ in hint rate beyond what their effects predict, for reasons not known. A skill also
  hinted by inspiration is hinted by training less than predicted, and a skill another card's training also hinted
  reads slightly low. The more of a card's pool skills an inspiration hinted, the more often its training hints the rest.
  Letting inspired skills partly leave the draws (one fitted share) predicts unseen careers and unseen cards better;
  the solver does not do that.
- Rarity and the number of pals in the deck both fit the data in-sample, but neither predicts unseen careers
  detectably better, so the model ignores both.

### Model

A card gives, in scenario $s$, $h$ training hints per career:

$$h = \beta_s \left(1 + \frac{hf}{100}\right)\left(1 + d_s\,\frac{\mathit{sp}}{100}\right).$$

That is a base $\beta_s$ for a card with neither effect, raised in proportion to hint frequency and to specialty
priority. A career gets $\lfloor h \rfloor$ hints, or one more with chance $f = h - \lfloor h \rfloor$.

Each hint draws one skill uniformly from the pool and raises it by the card's hint increment $\iota$. So a skill reaches
level 5 and leaves the pool after $c = \lceil 5 / \iota \rceil$ hints. The runs bear out the increment: on cards with
$\iota$ of 2 or more, where a level could fail the check, 33,494 of
33,505 levels that one card gave alone are multiples of $\iota$, or 5.

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

| Scenario | $\beta$ [95%] | $d$ [95%] | Rows | Runs | Typical card's hints per career [95%] |
|---|---|---|---|---|---|
| Unity Cup | 2.428 [2.30, 2.58] | 0.299 [0.15, 0.44] | 24,412 | 1,875 | 3.5 [3.40, 3.58] |
| Grand Concert | 1.990 [1.89, 2.05] | 0.082 [0.02, 0.18] | 87,736 | 5,499 | 2.7 [2.60, 2.71] |
| URA Finale | 1.369 [1.26, 1.53] | 0.393 [0.11, 0.59] | 15,645 | 960 | 2.0 [1.93, 2.13] |
| Trackblazer | 1.579 [1.33, 1.98] | 0.162 [−0.23, 0.60] | 3,186 | 130 | 2.2 [2.00, 2.39] |
| Grand Masters | 1.991; the scenarios span 1.37-2.43 | 0.134; the scenarios span 0.08-0.39 | no runs: **assumed**, one $\beta$ and one $d$ fitted over every scenario's rows | | |

The typical card is the runs' median: hint frequency +30% and specialty priority 35.

The intervals show how far the fit could move on other careers and other cards. Each comes from refitting on
500 resamples of the data that draw both cards and careers at random with replacement, so a
card's rows move together and so do a career's. They are narrow for the typical card's hints and wide for $d$,
because $\beta$ and $d$ trade off: across the resamples their correlation is
−0.88 (Unity Cup), −0.89 (Grand
Concert), −0.88 (URA Finale) and −0.90
(Trackblazer). So the fit pins down what a typical card gives, and much less how far specialty priority raises a
card's hints. Grand Masters has no runs. Its values are pooled over the scenarios' rows, of which Grand Concert gives
67%, so they sit close to Grand Concert's; the spread of the four scenarios'
own values, given in its row, says more about how far off they could be than a sampling interval would. Every value
here averages over how many of each card's pool skills an inspiration hinted (*Inspired pool skills*, below).

### Evidence


*How the held-out tests are read here.* Hint frequency, specialty priority, rarity and pool size belong to the card,
and every row of one card shares that card's own departure from the model. Treating careers as the only units then
understates the noise. Each held-out comparison below is given three ways: $z$ with careers as the units, as elsewhere
in this document; $z$ with careers and cards both as units (*two-way*), which the verdicts read; and the range of
$z$ (two-way) over three random splits that hold out whole cards instead of whole careers (*unseen cards*).

Few cards carry most rows: the specialty fit's rows amount to about
23 equally used cards, the form study's to about
29. With that few, reading $z$ against the normal curve can
be too generous. So each gain is also checked by a *wild bootstrap* by card: each card's summed difference, centred so
that there is no gain, gets a random sign, many times over, and $p$ is the share of those resamples whose $z$ is as
large as the real one. A verdict of "holds" needs two-way $z$ above 1.96 and that $p$ below 0.05.

*The form.* The form was chosen before the specialty term was added, on a wider sample: SR and SSR cards at any limit
break, and rows an inspiration also hinted, 205,974 rows from 8,769 runs. That
study reads the count as continuous and ignores the level-5 cap. On careers and cards left out of the fit,
per-scenario $\beta(1 + hf/100)$ predicts better than either simpler form:

| Alternative | Per-scenario $\beta(1 + hf/100)$ better held out by | $z$ by career | $z$ two-way | bootstrap $p$ | $z$ on unseen cards |
|---|---|---|---|---|---|
| one constant count, no $hf$ | 2,201.9 nats | 58.4 | 4.4 | 0.001 | 4.2 to 4.3 |
| one $\beta(1 + hf/100)$ for every scenario | 738.2 nats | 32.2 | 4.9 | 0.001 | 4.8 to 4.8 |

*Hint frequency's slope.* The model **assumes** the $1/100$: it reads a card's +40% as 40% more hints. A free divisor tests it,
with and without the specialty term. Both free divisors fit better in-sample (nominally, with rows treated as
independent), and with careers as the only units the form study's one looks clearly better on unseen careers.
Counting cards as units too, neither is detectably better, on unseen careers or unseen cards:

| Model | Free divisor | Held out, against $1/100$ | $z$ by career | $z$ two-way | bootstrap $p$ | $z$ on unseen cards | Verdict |
|---|---|---|---|---|---|---|---|
| adopted, with the specialty term | 93 | 2.4 nats | 2.0 | 0.7 | 0.47 | −0.6 to 0.2 | not detectable; $1/100$ kept, assumed |
| form study, without it | 81 | 34.2 nats | 9.3 | 1.5 | 0.16 | 0.6 to 1.4 | not detectable |

*Specialty priority.* Scored on careers left out of each fit, skills are hinted less often by cards with no specialty
priority than the model without the term predicts, and more often by cards with 60 or more. With the term every band reads within 2% of
prediction, a tolerance set by hand. A card's other careers are still in those fits, though, so the last two columns
score each card by fits that never saw it, as the range over the three card splits. There too the two outer bands
miss without the term (no split's interval covers 1), and with it every band's interval covers 1 on every split. The
intervals count careers and cards as units; the bands rest on few cards in effect:

| Specialty priority | Rows | Cards (effective) | Observed / predicted, without the term | With it | Unseen cards, without | Unseen cards, with |
|---|---|---|---|---|---|---|
| 0 | 39,776 | 22 (4.7) | 0.964 [0.942, 0.986] | 1.004 [0.979, 1.028] | 0.957-0.961 | 1.003-1.015 |
| 1-29 | 1,777 | 6 (1.9) | 0.969 [0.949, 0.988] | 0.990 [0.970, 1.009] | 0.970-0.977 | 0.989-0.996 |
| 30-59 | 60,381 | 80 (19.3) | 1.000 [0.984, 1.015] | 0.992 [0.978, 1.006] | 1.000-1.003 | 0.992-0.994 |
| 60+ | 29,045 | 36 (3.9) | 1.055 [1.033, 1.076] | 1.010 [0.989, 1.030] | 1.059-1.062 | 1.009-1.022 |

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| The four weights are not all zero | | in-sample, against the resamples' spread: $\chi^2$ = 28.3 on 4, $p$ = 1.1e-05 | holds |
| The term predicts unseen careers | | held out 53.0 nats; $z$ = 9.4 by career, 2.1 two-way; bootstrap $p$ = 0.030 | holds |
| The term predicts unseen cards | | 51.2 to 67.0 nats; $z$ = 1.8 to 2.1 | gain on every split, at the edge of detection |
| Fitted on the earlier half of the runs, it predicts the later half: a check for drift over time, since the two halves use largely the same cards | | 13.9 nats; $z$ = 4.2 by career, 2.3 two-way; bootstrap $p$ = 0.016 | holds |
| The four weights differ | | in-sample, against the resamples' spread: $\chi^2$ = 10.1 on 3, $p$ = 0.018 | they differ |
| A weight per scenario predicts better than one shared weight | one $d$ = 0.146 [0.09, 0.22], with a $\beta$ per scenario | held out 16.2 nats; $z$ = 5.2 by career, 1.3 two-way, 1.1 to 1.5 on unseen cards; bootstrap $p$ = 0.23 | not detectable |
| With the term, the scenario × specialty cells fit | 14 cells of 200+ rows, scored held out | two-way $\chi^2$ = 7.0, $p$ = 0.94; by career alone $\chi^2$ = 20.4, $p$ = 0.12 | no misfit detected |
| The same card, only specialty priority changing | $d$ = −0.04: [−0.38, 0.46] rows independent, [−0.29, 0.20] by card; 32 cards | +0.02 nats over no term | cause undecided |

The weights' tests pull two ways. In the fitted data the four weights differ by more than their spread across
resamples. Fitted on some careers or some cards and scored on the rest, a weight per scenario is ahead of one shared
weight by less than chance could give. So the data show that the weights differ, and do not show that a weight per
scenario predicts better.

The last row compares each card with itself. On these cards, from one limit break to the next, specialty priority
changes and hint frequency does not; a base is fitted per card and scenario. The first interval treats rows as
independent; the second allows for rows of one card being related and comes out narrower, though with
32 cards it may be too narrow. Both cover no effect and the shared weight. The
second excludes the URA Finale and Unity Cup weights, but 66%
of these rows are Grand Concert's, so it does not test those scenarios' weights. Players choose their limit breaks,
so this is not a randomised comparison either. A scale on the per-scenario weights, refitted on either half of the
runs, on SR or SSR cards alone and without each of the eight most-used cards, stays within
0.87-1.07.

*Covariates and cards.* Card type, initial bond and pool size add no nominally detectable gain to the adopted rate.
The table tests three more terms on 130,979 rows, under the limits *How to read* sets out. Rarity and pals
per deck fit better in-sample, but neither predicts unseen careers detectably better once cards count as units. A
factor per card does; it cannot be scored on cards left out, since a card's own factor is unknown there:

| Added term | In-sample gain | Held out | $z$ by career | $z$ two-way | bootstrap $p$ | $z$ on unseen cards | Verdict |
|---|---|---|---|---|---|---|---|
| rarity, SR against SSR | +3.5 nats | 2.3 nats | 1.5 | 0.4 | 0.71 | −1.2 to −0.2 | not detectable, ignored; factor 0.974 |
| pals per deck | +18.7 nats | 18.4 nats | 5.2 | 1.8 | 0.07 | 1.1 to 1.6 | not detectable, ignored; factor 1.026 per pal |
| a factor for each of 68 cards with 200+ rows | | 52.2 nats | 8.4 | 3.3 | 0.001 | | known miss; cards read 0.84-1.18 |

Every deck has 6 cards, so one more pal is one fewer non-pal card: the pals term is
equally a term in the number of non-pal cards.

*Other sources of the same skill.* The fit leaves out rows where an inspiration also hinted the skill. Against the
adopted rate, which was fitted on the other rows here, the table compares the card's own hints, split by whether
another source also hinted the skill:

| Other source of the skill | Rows | Observed / predicted |
|---|---|---|
| none, on careers left out of the fit | 114,529 | 1.005 [0.993, 1.017] |
| none, on cards left out of the fit (three splits) | 114,529 | 1.005-1.010, widest interval [0.992, 1.025] |
| inspiration | 14,283 | 0.74 [0.70, 0.77] |
| an event the card's own events could give | 939 | 1.09 [0.95, 1.22] |
| an event only other cards' events could give | 3,046 | 0.99 [0.95, 1.04] |
| other card's training | 5,611 | 0.95 [0.91, 0.99] |

With no other source, scored on careers the fit did not see, the adopted rate reads within 2% of what is observed (a
tolerance set by hand), its interval counting careers and cards as units; scored on cards the fit did not see, the
interval covers 1 on every split. This average mixes card-runs with and without inspired pool skills, which read
differently (next). Where an event could also have given
the skill, whether one of the card's own events or only another card's, the interval covers 1: no shift is
detectable. (The records name an event source by the deck cards whose events could give the skill; which event fired
is unknown.) A skill another card's training also hinted reads 0.95 of the
model, a known miss smaller than the inspiration's.

*Inspired pool skills.* Does an inspired skill leave the card's pool, so the card's draws go to the rest? The table
sorts the fit's rows by how many of the card-run's pool skills an inspiration hinted. *Card-adjusted* divides out
each card's own observed / predicted in each scenario first, so a card that simply hints more cannot make the
pattern. The count includes inspired skills the card also hinted itself: splitting on that would sort rows by their
own outcome. The last two columns are scored on careers left out of the fit, without and with a removal share
(below). Intervals count careers and cards as units:

| Inspired pool skills | Rows | Observed / predicted | Card-adjusted | Held out, kept | Held out, with the removal share |
|---|---|---|---|---|---|
| 0 | 67,235 | 0.956 [0.945, 0.968] | 0.968 [0.958, 0.978] | 0.956 | 0.992 [0.980, 1.005] |
| 1 | 44,246 | 1.028 [1.017, 1.039] | 1.021 [1.014, 1.028] | 1.028 | 1.012 [1.003, 1.022] |
| 2 | 14,988 | 1.083 [1.053, 1.114] | 1.062 [1.037, 1.086] | 1.083 | 1.005 [0.988, 1.022] |
| 3+ | 4,510 | 1.102 [1.052, 1.152] | 1.076 [1.031, 1.122] | 1.102 | 0.969 [0.928, 1.009] |

The more of a card's pool skills an inspiration hinted, the more its other skills are hinted than the model predicts,
card-adjusted too. A placebo asks whether something about the whole career does it. On card-runs with no inspired
pool skill, sorted by how many inspired pool skills the career's other cards have, the card-adjusted ratio reads
0.962 (none), 0.972
(1-2), 0.964 (3-5) and 0.959
(6+): flat, so no career-wide cause is detected.

A removal share $r$ tests one mechanism: a card-run with $k$ inspired pool skills draws a share $r$ of the way from
its full pool to the pool less those $k$, so $r = 0$ keeps them, as the model does, and $r = 1$ removes them for the
whole run. Fitted with a base and weight per scenario, $r$ = 0.34, which gains
95.6 nats in-sample. It predicts unseen careers better by
95.2 nats ($z$ = 3.8 two-way,
bootstrap $p$ = 0.001) and unseen cards ($z$ =
3.7 to 3.8),
and on held-out predictions most of the rise across the table goes. On card-runs with exactly two inspired pool
skills alone, the share that fits is 0.24
[0.18, 0.30], each row
against its own prediction. That interval excludes the $r$ = 0.34 fitted over every card-run, so
one share does not fit every count of inspired skills equally well. When in a career the removal happens is not
tested, and the inspiration shortfall above may
be the same effect seen from the inspired skill's side, which is not tested either. The solver keeps $r = 0$ and the
bases above. Using $r$ would need each pool skill's chance of being inspired and bases refitted with it (Grand Concert's
would be 1.899 against 1.990).

### Details

Checks of the draws themselves: the level-5 cap, the shape of the draws and the stat option's place, and whether a
priority list changes training hints.

*The level-5 cap.* Removing a skill at level 5 fits 60.6 nats better than drawing with replacement, with no
extra parameter. It matters most where one hint reaches level 5: with replacement those rows read 1.206 of prediction,
with the cap 0.966.

*The draws' shape.* Three checks ask whether hints really are equally likely draws from the pool. None of them runs
the adopted model, so they test the shape of the draws, not the fitted rate. The repeat and pair checks use the form
study's rate, without the specialty term or the level-5 cap, and the pair check treats the count as continuous. The
stat-option check assumes a fixed count.

| Question | Observed | Predicted |
|---|---|---|
| A skill a card hinted at all, hinted again: $P(k \ge 2 \mid k \ge 1)$ | 0.143 | 0.142 with the two-point count (the model's: $\lfloor h \rfloor$ or one more), 0.194 with a Poisson count |
| Two skills of one pool both hinted, over 354,973 pairs, Trackblazer out | 0.0803 | 0.0810 by joint draws, 0.1104 independently |

Beside its skill pool, a card's hint can also be a stat option. The model assumes a draw never picks it. Two checks
test that. First, the adopted model is refitted with every draw also able to pick the stat option, as often as each
skill and never leaving the pool, with no extra parameter. In-sample that changes the fit by
−53.3 nats. On unseen careers the model without the stat option is ahead by
53.3 nats ($z$ =
3.4 two-way, bootstrap $p$ =
0.001), and on unseen cards $z$ =
2.8 to
3.4. That is evidence against a stat option drawn as
often as a skill; one drawn much more rarely is not tested. Second, if draws did pick it, the number of skill hints would vary
from career to career by at least a certain amount; in 83 of
98 (card, $hf$) cells it varies less. That check keeps only card-runs whose every count is exact,
with no skill at level 5 or hinted by another source, and its sample variance is biased low, so it is consistent with
the assumption without establishing it.

*Prioritised skills.* The model assumes a priority list does not change training hints. On the 3,817 runs that
show a list, 3,859 rows hold a prioritised pool skill. Training hinted them at 0.345, against 0.347
predicted: no sign of a rise, from a weak test.

## 8. Chain events

Computed by `research/s08_chains.py`.

A *chain card* is a support card whose events tell a short story in order: 3 links on an SSR, 2 on an SR, each link
able to hint skills. How many of these events a career fires, and which cards they go to, decides how many chain skills
a deck can hope for. Pal and group cards have events of their own and take no chain events. This section answers three
questions: how many chain events a career gets, which chain card each one goes to, and what happens to the events a
chain would have taken when it ends early.

### Findings

- **Decks with more pal cards get more chain events** (**fitted**). A career's number of chain events $N$ is
  Binomial(21, $p$) with $\operatorname{logit} p = a + b \times \text{pals}$, where group cards do not count as pals:
  the *pal-count budget*. $p$ runs from 0.251 with no pals to
  0.396 with four, the most any deck in the runs has, an average of
  5.3 to 8.3 chain events per career.
- **Each chain event goes to one of the chain cards still running, all equally likely** (**fitted**). With the
  pal-count budget, on careers the fit never saw, this *card rule* predicts better than the old model's rule, which
  weighted each card by its links left. Under the old fixed count the order is the other way.
- **Prioritising a skill on a chain-ending choice ends the chain** (**tested**). With the ending skill prioritised
  alone, the links after the choice show about as often as their markers' background gives on its own; with nothing
  on that choice prioritised, they show as often as the model predicts.
- **A chain that ends early frees its events for the other chain cards** (**supported**, conditional on the rest of the
  model). The alternative is that the ended card keeps drawing its share of events and they are lost. Freed predicts
  unseen careers better, and so it does with the difference in event counts taken out. In each kind of run where a
  chain can end early, the real careers look like simulated careers with ends freed and unlike every simulation with
  ends wasted; on runs showing no list that evidence comes almost entirely from Marvelous Sunday [Dazzling Day in the
  Snow]'s decks. A direct look at the deck-mates of an ended card lands between the two accounts and cannot decide. No
  experiment (the same deck played with and without the ending prioritised) has been run.
- **No scenario effect and no Sasami Anshinzawa [This Might Sting!] effect is detectable on top of the budget.** Her
  decks get more chain events, but they also carry more pals, and the pal count accounts for the difference. Not
  detecting an effect does not show there is none; the tests' numbers are in the evidence below.
- **Known misses.** Inari One's careers get fewer chain events than the budget predicts; early links run short at 3
  pals and long at 1; with five chain cards, chains that never start are slightly over-predicted. The causes are
  unknown.

### Model

A career gets $N \sim \mathrm{Binomial}(21, p)$ chain events, with
$\operatorname{logit} p = \log\frac{p}{1 - p} = a + b \times \text{pals}$ (**fitted**: $a$ = −1.095,
$b$ = +0.168). The 21 is kept from the old model, Binomial(21, 0.35). It lies just outside its
own 95% profile range (23-34),
but $n$ and $p$ trade off against each other (*The form of N* below), so the budget keeps $n = 21$ and lets $p$ carry
the deck. Pals are the
deck's pal cards other than group cards. Each event advances one chain card by one link, the card picked with equal
chance among the chain cards still running. A chain that has finished or stopped takes no more events, so later events
go to the others.

A chain can stop before its last link in two ways. At some links, IT's default choice (the one it takes when nothing on
another choice is prioritised) ends the chain on some of its outcomes. The model stops the chain there on a share
0.333 of that link's firings, one value **fitted** for every such link (the best point of a coarse grid;
a finer grid puts it near 0.35). Other cards have a separate
*chain-ending choice*: when the player prioritises a skill on it, IT takes it with the chance section 9's three cases
give, read from each run's own priority list. A run that shows no list takes 0.25 times the share
seen on runs that show one (**fitted** on a grid: a scale, not a measured behaviour; Marvelous Sunday [Dazzling Day in
the Snow]'s decks are 1,932 of the
2,485 no-list runs where it applies, so they carry most of its weight). The model computes the exact distribution of where each chain ends
up (section 12).

| Pals | 0 | 1 | 2 | 3 | 4 |
|---|---|---|---|---|---|
| $p$ | 0.251 | 0.284 | 0.319 | 0.357 | 0.396 |
| mean $N$ | 5.27 | 5.96 | 6.70 | 7.49 | 8.31 |

No deck in the runs has five pals by this count, so the formula is not used past four.

### Evidence

*Reading a chain's progress.* The records do not say which events fired. They list the skills a career was hinted,
each credited to a card. A link is read through its *marker* skills, so it is readable only if every choice and
outcome of its event gives a skill that no other event of the card gives. A link is left out, seen or not, when another
deck card's events could also give its marker, or when the marker could have come from inspiration. That leaves
8,722 runs. A marker can also turn up without its link firing, from training hints or the scenario's
and trainee's events. This *background* rate $\beta$ is measured for each skill, scenario and trainee on runs whose deck
cannot give the skill, with each trainee's own rate pulled toward the scenario's. A link that fired then counts as seen
with chance $1 - \varepsilon(1 - \beta)$ ($\varepsilon$, a small allowance for a marker that fails to show, is measured under
*Details*), and one that did not with chance $\beta$. Each fit is exact over the chain
states the run's deck can pass through.

*Which model predicts unseen runs?* Each model is fitted on four fifths of the runs and scored on the rest (*held out*,
*How to read*). The *old model* is the one this section replaced: $N$ fixed at Binomial(21, 0.35), events weighted by
links left. Its stop risks, end chances and backgrounds are the same fitted ones every other model uses. The *free
distribution* fits $P(N = 0..17)$ and $P(N \ge 18)$ directly (no deck has more than 18 links). Every model but the old
one uses the card rule unless the row says otherwise. Positive nats favour the first model.

| Model | Compared with | Held-out nats ± SE | $z$ |
|---|---|---|---|
| pal-count budget | the old model | +351.3 ± 30.6 | +11.49 |
| pal-count budget | the free distribution | +187.4 ± 20.3 | +9.24 |
| pal-count budget | the same budget, events weighted by links left | +156.3 ± 22.8 | +6.85 |
| pal-count budget, with the backgrounds, choice shares and no-list shares also measured on the training runs only | the old model | +352.4 ± 30.6 | +11.53 |
| free distribution | the old model | +163.9 ± 29.4 | +5.57 |
| binomial, $n$ and $p$ fitted | the old model | +151.5 ± 28.8 | +5.26 |

In-sample, the pal count's slope $b$ is far from zero: likelihood-ratio $p$ =
3.8e-91 against one $p$ for every deck. The card rule wins only together
with the budget. With $N$ fixed at Binomial(21, 0.35), weighting by links left scores
241 nats above it (a composite score, each link scored on its own); with the budget
the order reverses (the table's third row).

*Does prioritising the ending skill end the chain?* Some cards have a separate chain-ending choice before their last
link. When a skill on it is prioritised alone, IT should take that choice and the chain should stop, so the markers of
the links after it can show only through their background. On runs showing a list, the table counts those later links
and what each account of them would give: background alone (the chain always ended), the model, and the chain never
ending. The share of chains that went on is (seen - background) / (never ended - background), with a Poisson error.

| Card | Skill on the ending choice | Links after it read | Seen | Background alone | Model | Never ended | Share going on |
|---|---|---|---|---|---|---|---|
| every such card | prioritised alone | 623 | 74 | 68.8 | 68.8 | 265.2 | 0.03 ± 0.04 |
| every such card | prioritised, already at Lv 5 elsewhere | 71 | 32 | 16.1 | 32.7 | 32.7 | 0.96 ± 0.34 |
| every such card | not prioritised | 1,794 | 1,066 | 346.2 | 1,073.6 | 1,073.6 | 0.99 ± 0.04 |
| Marvelous Sunday [Dazzling Day in the Snow] | Cloudy Days ○, prioritised alone | 401 | 30 | 25.4 | 25.4 | 160.2 | 0.03 ± 0.04 |
|  | not prioritised | 410 | 195 | 30.9 | 189.1 | 189.1 | 1.04 ± 0.09 |
| Smart Falcon [My Umadol Way! ☆] | Focus, prioritised alone | 48 | 5 | 4.4 | 4.4 | 22.5 | 0.03 ± 0.12 |
|  | not prioritised | 430 | 292 | 31.8 | 301.6 | 301.6 | 0.96 ± 0.06 |

With the skill prioritised alone, the later links show about as often as background alone gives: the share of chains
that went on is 0.03 ±
0.04, and
0.09 or
0.00 with every background
lowered or raised by a fifth. The model column there follows from section 9's one-sided share,
1.00. With nothing on the choice prioritised, the later links show as often as the model predicts. When the
ending skill is prioritised but already at hint Lv 5 from elsewhere (section 9's maxed case: at Lv 5 at the end of the
run from another source, with no credit from the card's event), the share going on is 0.96 ±
0.34; the model takes the ending choice there with
chance 0.00. This tests the choice rule on chain-ending choices
directly; section 9's readings include only four of them.

*Does an ended chain free its events?* A chain can end early at a default-choice stop or when IT takes a prioritised
chain-ending choice. Two accounts of what happens next are possible:

- *freed* (the model): the ended card drops out, and later chain events are shared among the cards still running;
- *wasted*: the ended card keeps drawing its share of events until it would have finished, and those events do
  nothing.

Under freed, ending one chain early gives the deck's other chain cards more events; under wasted it gives them none.
Each account gets its own stop share and no-list scale from the same search over the grids, the budget re-fitted at
every point: freed 0.333 and 0.25, wasted
0.400 and 0.30. On the real runs the search reaches
each account's best point of the full grid. The runs fall into four groups by what can end one of their chains early:

| What can end a chain early | Runs | Freed over wasted, in-sample nats | Held out, nats ± SE |
|---|---|---|---|
| nothing | 3,731 | +2.7 | +2.6 ± 3.4 |
| a default-choice stop, nothing else | 1,872 | +9.8 | +6.9 ± 4.9 |
| a chain-ending choice, list shown | 634 | +6.4 | +6.3 ± 3.1 |
| a chain-ending choice, no list shown | 2,485 | +7.1 | +6.7 ± 4.1 |
| all runs | | +26.0 | +22.5 ± 7.8 |

A chain-ending choice counts where the model gives it a chance above zero. On runs where nothing can end, the two
accounts make the same prediction for a given budget. They still differ there because each fits its own budget on
all the runs, so that row carries the budgets' difference, not a fourth look at the mechanism.

*Held out.* With each account's constants searched on the training folds, freed predicts unseen runs better by
22.5 ± 7.8 nats ($z$ =
2.88). The *mechanism alone* takes the budget out of the difference: the
budget is fitted on the training runs where nothing can end, where the accounts agree, and held fixed; each account's
constants are searched on the training runs where a chain can end, and those runs are scored. Freed is ahead by
30.1 ±
9.1 nats ($z$ =
3.31). That budget is carried over to decks whose pal mix
can differ.

*A simulated reference.* To judge whether a 26-nat lead is large, the test
simulates careers: the real runs' decks and lists, with their markers drawn from the model,
20 pools with ends freed and as many with ends wasted, each at that
account's fitted constants and budget. Every pool then goes through the real runs' whole procedure, both accounts'
constants searched again and budgets re-fitted. If ends were wasted, the real runs should look like the wasted pools.

| Statistic | Real runs | Pools with ends freed: mean ± SD | Below the real runs | Pools with ends wasted: mean ± SD | Below the real runs |
|---|---|---|---|---|---|
| freed over wasted, all runs | +26.0 | +25.5 ± 5.0 | 12 | −27.2 ± 6.5 | 20 |
| the same, nothing can end | +2.7 | +4.4 ± 1.8 | 4 | −5.1 ± 2.2 | 20 |
| the same, a default-choice stop only | +9.8 | +10.1 ± 3.4 | 9 | −11.3 ± 3.6 | 20 |
| the same, a chain-ending choice, list shown | +6.4 | +5.3 ± 2.4 | 15 | −3.7 ± 2.0 | 20 |
| the same, a chain-ending choice, no list shown | +7.1 | +5.7 ± 3.1 | 12 | −7.0 ± 3.7 | 20 |
| the same, no list shown, without Marvelous Sunday [Dazzling Day in the Snow]'s decks | −0.6 | +0.8 ± 1.6 | 4 | −0.7 ± 1.3 | 12 |
| deck-mates' gain beside a default-choice stop | +0.028 | +0.049 ± 0.022 | 3 | −0.007 ± 0.021 | 20 |
| deck-mates' gain beside a prioritised chain-ending choice | +0.050 | +0.054 ± 0.020 | 8 | +0.001 ± 0.022 | 19 |
| deck-mates' gain beside a 2-link SR | +0.058 | +0.042 ± 0.017 | 15 | +0.039 ± 0.016 | 18 |

On every log-likelihood row but one the real runs score above every wasted pool and among the freed ones. The
exception is the no-list runs without Marvelous Sunday [Dazzling Day in the Snow]'s decks: there the real runs sit
between the two kinds of pool, so the no-list group's evidence comes almost entirely from her decks. With this many
pools
the ranking is coarse: if ends were wasted, the real runs would beat every wasted pool by luck about one time in
21, and the ranking cannot say how far into the tail they are. The three
groups where a chain can end are separate runs, so they are three looks at the mechanism; they share the fitted budget
and constants, and none is independent of the model that generated the pools.

*Deck-mates, directly.* The likelihood works through the whole model. A more direct look compares the deck-mates'
own progress. It takes decks whose other chain cards are all plain 3-link cards, plus one odd card: one whose chain a
default choice can stop, one whose chain-ending choice the run's list prioritises, or a 2-link SR, which finishes
early by design. Within cells of the same pal count, number of chain cards, link and scenario, it compares how often
the deck-mates' markers show, corrected for background as $\sum(\text{seen} - \beta) / \sum(1 - \beta)$, against decks
of plain 3-link cards only. Its error is clustered by run across every cell.

| Odd card | Deck-mate links | Gain ± SE | Freed predicts | Wasted predicts |
|---|---|---|---|---|
| 2-link SR (control) | 2,107 | +0.058 ± 0.015 | +0.041 | +0.042 |
| a default-choice stop | 701 | +0.028 ± 0.027 | +0.046 | +0.000 |
| a prioritised chain-ending choice | 504 | +0.050 ± 0.032 | +0.054 | +0.000 |

Both accounts agree a 2-link SR frees events, and its deck-mates gain what they predict, far from no gain at all ($z$ =
3.9): the comparison can see freed events, and a finished chain's events
go to the other cards rather than being lost with it. Beside a chain that ends early, at a default-choice stop or a
prioritised ending, the gain lies between the two predictions, within two errors of each, so the direct look cannot
decide. These are the same runs the likelihood uses, looked at directly, not new careers.

*How much of each?* A mixture lets each run's chains be freed with chance $\lambda$ and wasted otherwise, the budget
re-fitted at each $\lambda$. The best $\lambda$ is 0.95, the values within 1.92 nats
run 0.70-1.00,
and every run wasted is 26 nats below the best, while every
run freed is only 0.1 behind it, so the data do not point to
some chains being wasted. The range is nominal: it is a profile likelihood with $\lambda$ bounded at 1, and each
account's constants stay at their own fit.

*What this rests on.* The chain-end conclusions hold as far as the rest of the model does: the budget law, the card
rule, the backgrounds and the choice rule, whose shares are taken as measured (section 9). The backgrounds, choice
shares and no-list shares are measured on every run, test folds included; both accounts share them. The analysis was
designed after earlier looks at these runs. The held-out scores and the simulated reference protect the fitted
constants, and every statistic computed is reported. Freed and wasted are the only accounts tested; the mixture covers runs behaving
as either, not other fates for the events. No experiment has played the same deck with and without the ending
prioritised.

### Details

The checks behind the evidence above: how the markers are read, how the simulated pools compare with real
careers, how the conclusions move with the backgrounds and grids, and the fit of $N$ in full.

*The marker allowance* $\varepsilon$ (**chosen**, inside its measured interval). The fits allow a fired link's marker
to go missing with chance 0.002. Links fire in order, so a later link seen on a clean
marker ($\beta$ small) proves that the earlier links fired, and those earlier links show how often a marker goes
missing:

| Earlier links proved fired by a later one | Showing no marker | Share [95%] |
|---|---|---|
| 1,051 | 0 | 0.000 [0.000, 0.004] |

With none missing, the interval's upper limit counts each run as one trial. The allowance hardly matters: at $n = 21$
the best single $p$ moves from 0.325 to
0.330 over allowances 0 to 0.02.

*Do the pools vary as much as real careers would?* A pool is the same decks with fresh luck, so its spread leaves out
how much careers differ from one another. Within each pool, the runs' own differences give a second error, the one
that treats runs as independent; the table compares it with the real runs'. The two are close on every row, so
the pools' spread is not narrow for that reason:

| Runs | Real runs' per-run error | Pools with ends freed, mean | Pools with ends wasted, mean |
|---|---|---|---|
| all | 7.7 | 7.3 | 7.3 |
| nothing can end | 3.4 | 2.9 | 3.2 |
| a default-choice stop only | 4.8 | 4.6 | 4.5 |
| a chain-ending choice, list shown | 3.1 | 3.0 | 2.8 |
| a chain-ending choice, no list shown | 4.0 | 3.9 | 3.9 |

*Backgrounds.* The pools treat each marker's background as known. With every background halved, or raised by half,
and each account's constants and budget re-fitted, freed stays ahead by
23.1 and 26.1
nats.

*Credits at levels a link cannot give.* Some markers are credited to a card's event below every level that link's
outcomes give the skill, which that link cannot have done. The cause is not known. The low credits fall on one skill of a choice in every
scenario, which points to an event the outcome data do not list; the data could also list the wrong level. 14 links
have at least one such credit. Leaving those links out of every run
(6,829 readings) and running the whole comparison again gives a budget of
$a$ = −1.081, $b$ = +0.164
(against −1.095 and +0.168), and freed ahead by
16.0 nats, against 26.0
with them: a smaller lead, on fewer readings, but still a lead.

*The grids.* Freed's best stop share sits on a grid point and wasted's in a coarse gap. On a finer grid around each
account's best, freed stays ahead by 26.2 nats
(freed 0.350 and 0.225, wasted
0.383 and 0.275).

*The form of $N$.* Among single distributions for $N$, the free fit's $\min(N, 18)$ is under-dispersed: its variance is
0.79 times its mean, where a Poisson's would equal it, and Poisson(7.1) is
182 nats behind. The binomial beats the Conway-Maxwell-Poisson by
1.6 nats. Binomial(21, 0.35) is
378 nats behind the free fit in-sample. The best single binomial has
$n$ = 27 (95% profile range 23-34),
but $n$ and $p$ trade off along a ridge, so the budget keeps $n = 21$ and lets $p$ carry the deck.

*Groups.* Each group's own term on the logit, on top of the budget:

| Group | Runs | Own term | Likelihood-ratio $p$ | Mean pal count, group / other runs |
|---|---|---|---|---|
| decks with Sasami Anshinzawa [This Might Sting!] | 960 | +0.034 | 0.18 | 2.83 / 1.68 |
| Inari One's careers | 792 | −0.073 | 0.007 | 1.97 / 1.79 |

Without the budget, Sasami's decks fit $p$ = 0.351 against
0.305 for the other runs; with it, her own term is not
distinguishable from zero.

*Scenarios.* No scenario effect is detectable on top of the budget: an intercept per scenario gains
2.1 nats for
3 parameters. A free distribution per scenario does gain
48 nats for 54 parameters, because the
scenarios' decks carry different numbers of pals.

*Stop share and no-list scale.* Each is profiled with the budget re-fitted at every grid value. The stop share
0.333 is the best grid value and the only one within 1.92 nats, the 95% range for one parameter (0.4 is
1.95 nats behind, 0.25 is
14). The no-list scale 0.25 is
0.0 nats below the grid's best, and the grid
values within 1.92 nats run 0.10-0.30;
1, the list-showing share itself, is 56 nats behind.

*Final links by deck.* By the deck's pal and group cards together (unlike the budget, which leaves group cards out),
how often the final link's marker shows, against each model's chance of the chain reaching
it, with the background and allowance applied:

| Pal and group cards | Chain cards | Runs | Observed [95%] | Final model | Old model |
|---|---|---|---|---|---|
| 5 | 1 | 127 | 0.952 [0.899, 0.978] | 0.960 | 0.957 |
| 4 | 2 | 1,438 | 0.885 [0.869, 0.901] | 0.888 | 0.866 |
| 3 | 3 | 2,862 | 0.544 [0.530, 0.558] | 0.552 | 0.574 |
| 2 | 4 | 2,060 | 0.316 [0.305, 0.328] | 0.316 | 0.369 |
| 1 | 5 | 1,752 | 0.220 [0.211, 0.230] | 0.212 | 0.244 |
| 0 | 6 | 483 | 0.147 [0.132, 0.162] | 0.147 | 0.184 |

The final model lies outside the observed interval at
0 of the six rows, the old model at
5.

*Known miss: early links.* An early link is any link before the final one. The shortfall is the share the model
predicts to fire minus the share seen. Each run's links count as one cluster:
$z = \sum(\text{pred} - \text{obs}) / \sqrt{\sum(\text{pred} - \text{obs})^2}$ over runs.

| Pal and group cards | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Final model: shortfall | +0.006 | +0.022 | +0.013 | −0.041 | +0.004 |
| $z$ | +0.8 | +2.8 | +1.0 | −3.1 | +0.2 |
| Old model: $z$ | +1.1 | +8.4 | +9.1 | +6.6 | +7.3 |

*Chains never started.* With five or six chain cards (one pal or group card, or none), the share of chains that never reach link 1:

| Chain cards | Observed [95%] | Final model | Old model |
|---|---|---|---|
| 5 | 0.215 [0.183, 0.246] | 0.248 | 0.12 |
| 6 | 0.37 [0.31, 0.42] | 0.36 | 0.18 |

*Individual cards.* The 55 cards with 60 or more final-link readings, against the
final model, clustered by run since one run's chain cards share its $N$. Pooled,
0.369 observed against 0.368
predicted. Across cards $\chi^2$ = 77 (nominal $p$ =
0.03), and no card is beyond $|z|$ =
2.6. The old model gives $\chi^2$ =
244 on the same test.

## 9. Event choices and outcomes

Computed by `research/s09_outcomes.py` (the event-only rows and the certainty checks: `research/s13_calibration.py`).

Two rules are **assumed** (`docs/MODEL.md`). The *choice rule*: IT takes an event's top choice unless a prioritised
skill sits on another choice, and the player prioritises every target. The *equal rule*: a choice's outcomes are equally
likely unless measured, since master.mdb holds no outcome odds.

A target counts as *prioritised* when it is on the run's priority list and not outranked. It is outranked when every
event that offers it also has a rival choice holding a higher-ranked prioritised skill, which IT would take instead.
That affects 15 of section 13's event-only rows. That the
higher rank wins is assumed, not tested. The classifier also ignores a higher-ranked skill on the target's own choice,
which would cancel the outranking.

### Findings

- **IT follows the choice rule, in three cases** (table below). With a prioritised skill on one choice only, IT took
  that choice in every reading. With the skill already at hint Lv 5 from elsewhere before the event, it took it in
  none; with a prioritised skill on the default choice too, in 0.51 of them.
- **The event-hint calibration by priority is in section 13.**
- **Four adopted outcome estimates replace equal shares.** Seiun Sky's link 3 departs from the equal rule; Riko's Ramp
  Up and Anshinzawa's Nothing Ventured are per-run chances; Festa's rests on an attribution model. Two are known misses:
  Anshinzawa's sits above her share on the planner's premise, and Riko's sits just above hers when only credits at a
  level her event gives are counted.
- **The equal rule misses on most choices the records can read**: on 15 of
  20, the 90% interval excludes it, and on 5
  of the 9 read without hint levels, far more than chance either way. Those without an
  adopted value stay at the equal rule: known misses. Two verdicts depend on how a credit below its outcome's level is
  read (below).
- **Pal and group finales are assumed to fire every time.** They fired in every checked run: an observed rate on a
  finite sample, not a game rule.

### The choice rule's three cases

A reading is a run showing its priority list, where one event's choice holds a prioritised skill and the records show
which choice IT took. The readings come from a *generic scan* of every card event whose choices carry skills no other
choice or deck card gives, and from Anshinzawa's finale, read through its hint levels. A *steered* choice is one other
than the top that holds a prioritised skill. Most readings come from that finale;
4 are a chain-ending choice, all one-sided (Sweep Tosho
[Lamplit Training of a Witch-to-Be]). Section 8 tests the rule on chain-ending choices directly, through the links
after them.

| Case | Readings where IT took the prioritised choice | Share [90%] |
|---|---|---|
| one-sided: only that choice holds a prioritised skill | 309 of 309 | 1.00 [0.99, 1.00] |
| maxed: the skill already at hint Lv 5 from another source before the event | 0 of 12 | 0.00 [0.00, 0.18] |
| both: the default choice holds a prioritised skill too | 23 of 45 | 0.51 [0.39, 0.63] |

The records give a skill's level at the end of the run, which includes hints that came after the event. A hint
given at Lv 5 is taken not to be credited (**assumed**). It fits Anshinzawa's runs: IT takes her date-start event's
bottom choice in nearly every run with Risky Business prioritised, so a run at Lv 5 that never credits it either got
that hint uncredited or skipped the choice. So a skill credited to the card's event was below Lv 5 when that event
fired. The maxed
case therefore counts a skill only when it ends at Lv 5 from other sources and none of it is credited to the card's
event. For the generic scan that reading can never show IT taking the steered choice, since that choice is seen only
through the card's credit, so its maxed readings are left out. Anshinzawa's finale can be read: her date-start event
comes first, and a run whose Risky Business is never credited to her was at Lv 5 before it, and so before the finale,
while a choice-2 win still shows by Nothing Ventured.

- Of the 27 runs with Risky Business at Lv 5 before her finale,
  12 show choice 1 and
  0 a choice-2 win; the other
  15 show nothing. Choice 2 shows Nothing Ventured on
  0.85 of its firings, so if the finale fired in each run IT takes choice 2 there less
  than 0.10 of the time (90%).
- The 16 runs that end at Lv 5 with her Risky Business credited
  were below Lv 5 at the date-start and count as one-sided: IT took choice 2 in all
  7 whose choice shows.

The model applies the same reading to a chain-ending skill: maxed only when it ends at Lv 5 with no credit from the
card's event.

The both case's readings are all Anshinzawa's finale. Riko Kashimoto's event is read apart: the generic scan does
not read it, since her top choice's skills fail its marker tests, so her runs are not among the table's readings. With
Lone Wolf and Maverick ○ both prioritised, IT took her bottom choice in 111 of
199 runs (0.56), no detectably different from the both case.
No effect of the ranking is detectable either (Riko's Lone Wolf above or below Maverick ○: intervals overlap). The
model takes the chain end with these shares in research (section 8); the planner assumes the player prioritises the
ending skill alone, the one-sided case.

### Measured shares

These shares are **fitted**. The table distinguishes observed counts from imputed ones. Festa's binomial interval
uses a partly imputed denominator. A *reading* is a firing where every outcome could show: the card's hint credited,
and each outcome's skill below level 5 from other sources.

| Event: outcome | Observed or attributed counts | Share [90%; Festa coverage unestablished] | Adopted |
|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] finale, choice 2: Nothing Ventured, not Risky Business (Risky Business prioritised alone, her events its only source) | 181/214 | 0.85 [0.80, 0.88] | per run, next row |
| the same finale, per run: her Nothing Ventured on lists prioritising both skills (the planner's premise) | 36/61 | 0.59 [0.48, 0.69] | 0.75: known miss |
| Riko Kashimoto [Planned Perfection] "Unexpected Side", top choice: Ramp Up, not Maverick ○, each read at its own hint level; counting every credit | 141/200; 145/265 | 0.70 [0.65, 0.76]; 0.55 [0.50, 0.60] | per run: 0.75 |
| Seiun Sky [Foolproof Plan] link 3: Fast-Paced, not Escape Artist | 79/118 | 0.67 [0.60, 0.74] | 0.65 |
| Smart Falcon [My Umadol Way! ☆] link 3: Center Stage, not Prudent Positioning | 40/74 | 0.54 [0.45, 0.63] | 0.5, the equal rule |
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante": Nothing Ventured, not Risky Business | 307 observed wins / 494.6 imputed trials | 0.62 [0.58, 0.66] | 0.60 |

A share counts only decks where no other card's events can give an outcome's marker, seen or not: where one can,
the record credits "A or B event" and cannot say which card fired. Smart Falcon's share is consistent with the equal
rule, which the model now uses for it (its earlier 0.65 rested on readings that dropped such shared credits for one
outcome only).

A credit counts only at a level its outcome can give. The records credit Maverick ○ to Riko Kashimoto's event below
Lv 5 in 63 of the 267 runs whose
deck has no other card that can give her three skills, though the one event GameTora lists for her that gives it gives
+5. 41 of those runs also credit her Ramp Up, which the same
choice gives in place of Maverick ○, so these credits come from somewhere else. Such credits fall mostly on one skill
of a choice and not on its other outcomes' skills (below), which points to a source of that skill that GameTora does
not list rather than to noise in the records; master.mdb holds no event rewards and cannot name it. Read at its
level, her top choice departs from the equal rule; counting every credit, it does not. Which reading is right turns
on that unknown source. Her adopted value is a per-run chance, set out under *Evidence*. Nakayama Festa's
share rests on an attribution model; its interval has unestablished coverage. Every count uses the full pool.

### Evidence

*Does IT follow the choice rule?* Where the runs show the choice taken, yes:

| Event | Condition | Choice taken |
|---|---|---|
| Sasami Anshinzawa's finale | Risky Business prioritised | choice 2 in about 89% of the 323 runs whose choice the hint levels show |
| Sasami Anshinzawa's finale | neither skill prioritised | identifiable choice-two wins in 0 of 243 runs |
| her date-start event "A Pleasant Practice Partner?", read by its extra Risky Business +1 | Risky Business prioritised | bottom choice in 252 of 254 runs winning finale choice 2 |
| the same | Risky Business not prioritised | bottom choice in 1 of 48 such runs |
| Riko Kashimoto [Planned Perfection]'s event | Lone Wolf prioritised | bottom choice in 387 of 534 runs |

Of the 34 finale readings that took choice 1 with Risky Business prioritised, 12 are the maxed case's (Risky Business at hint Lv 5 before the finale, Nothing Ventured not prioritised) and the other 22 had Nothing Ventured prioritised too.
The records show a priority list for only 3,817 of 8,940 runs. The rest count as unknown, never as an empty
list, so the rule is tested mainly through its consequences in section 13.

*Reading Sasami Anshinzawa's finale.* Nakayama Festa [Trusting the Dice]'s "Upping the Ante" also gives Nothing
Ventured and shares its credits. So these rows use the 929 runs with Anshinzawa's card and without Festa's.
Hint levels identify the choice:

- her Nothing Ventured +3 is choice 1, which otherwise gives nothing, and +1 is choice 2. On these runs Nothing
  Ventured is credited to no source but her events, so its level is her finale's;
- without Nothing Ventured, her Risky Business is read only when her events are its only source. Her events give it
  twice, +1 each (GameTora): the date-start event's bottom choice, which IT takes when Risky Business is prioritised,
  and a loss on finale choice 2. With Risky Business prioritised a loss shows as Lv 2, and Lv 1 means choice 1 gave
  nothing. On a list without it, a loss shows as Lv 1. Mixed with other sources the level cannot be split, and the
  run is not read.

GameTora marks the date-start event's bottom choice as ending her event chain, which for a pal card means her dates.
Yet with Risky Business prioritised, 300
of 373 runs show both its +1 and a finale outcome, so the bottom choice does not
stop her finale, an event of her SSR card. Whether her later dates still fire is not tested. The finale's choice is read in 323 of those
runs and cannot be read in 30; the rest show nothing.
With Risky Business prioritised alone, her date-start +1 never shows without a finale outcome
(0 of
285 runs with it). That is why a lone +1 with both skills prioritised is
read as choice 1 giving nothing.

*Sasami Anshinzawa's adopted share.* Her finale offers two choices. Choice 1, the top one, gives Nothing Ventured
+3 on one of its two outcomes and nothing on the other. Choice 2 gives Nothing Ventured +1 on a win and her Risky
Business +1 on a loss. The model's deck entry for her card takes choice 2 and tells the player to prioritise Risky
Business, the one skill only that choice holds. It credits her Nothing Ventured hint with a per-run chance of
0.75, set from runs with Risky Business prioritised, and counts Risky
Business as certain whenever the gold misses.

The planner also assumes the player prioritises every target. Risky Business / Nothing Ventured is one target, so on
that premise the list holds both skills: Nothing Ventured on both choices and Risky Business on choice 2, which is
section 9's both case. How often her Nothing Ventured shows depends on which skills the list holds:

| List shown | Runs with her Nothing Ventured | Share [90%] |
|---|---|---|
| both prioritised (the planner's premise) | 36 of 61 | 0.59 [0.48, 0.69] |
| Risky Business prioritised, not Nothing Ventured | 245 of 312 | 0.79 [0.74, 0.82] |
| Nothing Ventured prioritised, not Risky Business | 85 of 108 | 0.79 [0.72, 0.84] |
| neither prioritised | 125 of 243 | 0.51 [0.46, 0.57] |

With exactly one of the two skills on the list, the share is about the same either way and covers the adopted
0.75. With both, it is lower, and the adopted value lies above its
interval: a known miss on the planner's own premise. The finale's readings show why, through which choice IT took:

| List shown | Choice 1, Nothing Ventured | Choice 1, nothing | Choice 2, Nothing Ventured | Choice 2, Risky Business | Unreadable | Nothing shown | Nothing shown, Risky Business at Lv 5 |
|---|---|---|---|---|---|---|---|
| both prioritised (the planner's premise) | 15 | 7 | 21 | 2 | 11 | 0 | 5 |
| Risky Business prioritised, not Nothing Ventured | 12 | 0 | 233 | 33 | 19 | 0 | 15 |
| Nothing Ventured prioritised, not Risky Business | 37 | 0 | 48 | 2 | 9 | 11 | 1 |
| neither prioritised | 125 | 0 | 0 | 1 | 5 | 106 | 6 |

With Risky Business alone IT takes choice 2 in nearly every reading, and choice 2 wins most of the time. With
Nothing Ventured alone IT splits between the two choices, but both of them offer Nothing Ventured. With both on the
list, IT takes choice 2 in only 0.51 of the readings, the both case's rate, and falls back on
choice 1 the rest of the time, where Nothing Ventured comes on one outcome of two. That rate is enough to account for
the lower share: with the finale shown in 0.93
of runs (measured where IT always takes choice 2), choice 2 giving Nothing Ventured at its read share and choice 1 on
half its outcomes (the equal rule, **assumed**), the expected share is
0.63, inside the observed interval. This is a check of
consistency, not a test of the cause. Of the 61 runs whose list
holds both, 61 rank Nothing Ventured
above Risky Business, so whether the ranking matters cannot be tested here. ("Unreadable" is a run whose Risky Business credit is
shared or mixed with other sources, so a choice-2 loss cannot be told from her date-start hint. On lists without Risky
Business, "nothing shown" includes runs where choice 1 gave nothing, which cannot be told from a finale that did not
show.)

What this means for the planner (**derived**): where her card is a loop's only source of the gold and the white is
otherwise certain, a target's landing chance is proportional to 1 plus the gold chance. At the both-lists share it is
0.91 of what the adopted value
gives, and the loop's $P(\text{all})$ falls by the same factor. The model and the players' practice agree in either of two
ways: the deck entry tells the player to prioritise Risky Business and leave Nothing Ventured off the list, which
keeps the adopted value, or the share drops to the both-lists value, which keeps the premise. Neither is adopted yet.
The lists are the players' own choices, not assigned, and the both-lists share rests on
61 runs.

*Riko Kashimoto's adopted share.* The adopted value is the per-run chance of her Ramp Up hint. A run counts where
no other deck card's events can give Ramp Up and its level from other sources stays below 5. On runs whose shown
list prioritises Ramp Up and not Lone Wolf, as the model assumes of a target, counting every credit of it to her, as
when the value was set, gives 0.741
[0.714,
0.765], which covers the adopted
0.75. Counting only credits at Lv 3 or more, the level her event gives, it
is 0.717 [0.690,
0.742], just below it: a known miss under that reading. The rows
below count credits at Lv 3 or more. A prioritised Lone Wolf takes her bottom choice, and her Ramp Up then shows in
only 0.20 of runs, so the pooled chance is lower:

| Riko Kashimoto [Planned Perfection] | Count | Share [90%] |
|---|---|---|
| runs with her Ramp Up hint | 2,509 of 4,146 | 0.605 [0.593, 0.618] |
| on the 252 runs whose deck has no other card that can give any of her three skills and where all three could show (each below Lv 5 from other sources), firings where IT took the top choice; the rest took the bottom choice, Lone Wolf | 199 of 254 | 0.78 |
| runs with her Ramp Up hint, list shown with Ramp Up prioritised and not Lone Wolf | 572 of 798 | 0.717 [0.690, 0.742] |

*Nakayama Festa's share.* It starts from the 1,141 runs with her card, less her own careers and decks with
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

*The equal rule on every readable choice.* A choice is read when at least two of its outcomes have a marker and
they show in at least 30 runs. 31
choices have only one marked outcome and 33 have fewer readings; they are not
read. A share is one outcome's readings among the marked outcomes' readings, so under the equal rule it is one over
the number of marked outcomes. Where the outcomes give one skill at different hint levels, a *level split*, they
are read by that level, credited to her alone; a choice with a level split is read that way for every outcome. The
row reads the larger hint. A choice is off the equal rule when its 90% interval excludes the equal share. Chance alone
would put about one interval in ten off; 15 of 20 are,
and 5 of the 9 read without
hint levels, where a level reading cannot bias the count.

| Card, event (link, choice) | Outcome read | Count | Share [90%] | Equal share | Verdict |
|---|---|---|---|---|---|
| Grass Wonder [Fairest Fleur] "Hidden Petals" (link 3, choice 1) | Furious Feat +3 | 22 of 41 | 0.54 [0.41, 0.66] | 0.50 | consistent |
| Seiun Sky [Foolproof Plan] "It's All to Win" (link 3, choice 1) | Fast-Paced +1 | 79 of 118 | 0.67 [0.60, 0.74] | 0.50 | off: adopted value above |
| Smart Falcon [My Umadol Way! ☆] "My Umadol Way! ☆" (link 3, choice 1) | Prudent Positioning +1 | 34 of 74 | 0.46 [0.37, 0.55] | 0.50 | consistent |
| Kitasan Black [Fire at My Heels] "We Walk Together" (link 3, choice 1) | Professor of Curvature +3 | 43 of 72 | 0.60 [0.50, 0.69] | 0.50 | off: known miss |
| Yaeno Muteki [Fiery Discipline] "Yaeno Muteki's Love Song Nightmare" (link 3, choice 1) | It's On! +3 | 243 of 353 | 0.69 [0.65, 0.73] | 0.50 | off: known miss |
| King Halo [Tonight, We Waltz] "Glittering Star of the Ball" (link 3, choice 1) | Gap Closer +3 | 130 of 242 | 0.54 [0.48, 0.59] | 0.50 | consistent |
| Gold Ship [That Time I Became the Strongest] "Assembling the Uma Lord's Minions" (link 3, choice 1) | Inside Scoop +3 | 90 of 224 | 0.40 [0.35, 0.46] | 0.50 | off: known miss |
| Marvelous Sunday [Dazzling Day in the Snow] "Pure White Marvelousness ☆" (link 3, choice 1) | Fast & Furious +3 | 380 of 736 | 0.52 [0.49, 0.55] | 0.50 | consistent; counting every credit off (0.43 [0.40, 0.46]) |
| Matikanefukukitaru [Touching Sleeves Is Good Luck! ♪] "Finding Miraculous Joy!" (link 3, choice 1) | Super Lucky Seven +3 | 70 of 169 | 0.41 [0.35, 0.48] | 0.33 | off: known miss |
| Tosen Jordan [My Way] "If You Got It, Own It" (link 3, choice 1) | Breath of Fresh Air +3 | 25 of 43 | 0.58 [0.46, 0.70] | 0.50 | consistent |
| Nakayama Festa [Trusting the Dice] "Upping the Ante" (link 3, choice 1) | Nothing Ventured +3 | 43 of 280 | 0.15 [0.12, 0.19] | 0.50 | off: known miss |
| Agnes Tachyon [Q≠0] "Researching the Limits of Running: Rapport" (link 1, choice 2) | Murmur +1 | 327 of 525 | 0.62 [0.59, 0.66] | 0.50 | off: known miss |
| Agnes Tachyon [Q≠0] "Researching the Limits of Running: Velocity" (link 3, choice 1) | Unstoppable +3 | 512 of 820 | 0.62 [0.60, 0.65] | 0.50 | off: known miss |
| Daitaku Helios [Sunlit Angel ♪] "Hype Heaven, Part Two!" (link 3, choice 1) | Perfect Spot! +3 | 504 of 892 | 0.57 [0.54, 0.59] | 0.50 | off: known miss |
| Eishin Flash [Danke Schön] "One Should Share Friends' Hometown Love" (link 3, choice 1) | Sharp Streak +3 | 25 of 35 | 0.71 [0.58, 0.82] | 0.50 | off: known miss |
| Tazuna Hayakawa [Tracen Reception] "A Surprise Breather" (link 5, choice 1) | Focus +1 | 409 of 1,077 | 0.38 [0.36, 0.40] | 0.50 | off: known miss |
| Aoi Kiryuin [Trainers' Teamwork] "Chasing Dreams" (link 5, choice 1) | Subdued Front Runners +3 | 279 of 281 | 0.99 [0.98, 1.00] | 0.50 | off: known miss |
| Riko Kashimoto [Planned Perfection] "Unexpected Side" (her card's own event outside the chain, choice 1) | Ramp Up +3 | 141 of 200 | 0.70 [0.65, 0.76] | 0.50 | off; counting every credit consistent (0.55 [0.50, 0.60]) |
| Riko Kashimoto [Planned Perfection] "A Peek at Her Heart" (link 5, choice 1) | Rushing Gale! +3 | 3,228 of 4,653 | 0.69 [0.68, 0.70] | 0.50 | off: known miss |
| Light Hello [From the Ground Up] "At Rainbow Cove" (link 5, choice 1) | See Ya Later! +3 | 4,683 of 6,661 | 0.70 [0.69, 0.71] | 0.50 | off: known miss |

The rows off the rule without an adopted value stay at the equal rule: known misses. Festa's row reads her Nothing
Ventured +3 against +1; her adopted share, Nothing Ventured against Risky Business, is above.
10 of the 11 level splits lean to the
larger hint; the records cannot say whether the game draws them that way or the reading favours it. A credit at a
level no event GameTora lists for her gives is left out (298 of
them over the scan; 147 are on Marvelous Sunday's link 3, below the +3 both its outcomes give).
Counting them instead changes 2 verdicts, Marvelous Sunday's link 3
and Riko's "Unexpected Side". In every choice with such credits they fall mostly on one marker skill, and in all but
one choice on that skill alone. The only stories master.mdb lists for these characters that GameTora does not decode
are ones it lists for most support characters, and master.mdb holds no event rewards, so the game data cannot name
the source.
Daitaku Helios [Sunlit Angel ♪]'s link 2,
read where link 3 visibly fired, gives Tail Held High in 309 of
603 (0.51), consistent with the equal
rule.

*Pal and group finales.* The checks use runs where the card is the deck's one source of the skill and no inspiration
hinted it. Light Hello [From the Ground Up]'s event is credited with See Ya Later! in 2,163 of 2,163 such runs.
Team Sirius is credited with Risky Business, alone or shared, in 107 of 107. These observed rates are
consistent with a finale that always fires; finite samples do not establish it.

## 10. Random card events (not modelled)

Computed by `research/s10_random_events.py`.

A deck card's character has *shared random events* that can fire in another uma's career, for any card of hers.
They differ from a card's own events outside its chain, such as Riko Kashimoto [Planned Perfection]'s "Unexpected
Side" in section 9, which belong to the card and are modelled. $K$ is the number on offer: every shared event of the
deck's characters, an R card's character included, plus the trainee's own. A *reading* is one such event in one run
with an SR or SSR card of hers in the deck, read through the marker skill on its top choice. R cards are left out of
the measured rate, as of every measured result.

### Findings

- **Leaving them out under-predicts the targets they can hint**, a known miss: on real careers those targets read
  1.24 of the model, against 0.96
  for the same targets on decks without such a card. They read above that control also when matched on predicted
  acquisition, and on decks where no card's training hints give the target. Credited at the fitted rate, they read
  0.99, as the control does.
- **Two default targets are a character's top-choice marker**: Vodka's Nimble Navigator and Nice Nature's Ramp Up.
  A third, Uma Stan, sits on choice 2 of Seeking the Pearl's shared event, which IT takes only when Uma Stan is
  prioritised. No run prioritising it with her card credits it to her
  (0 of
  11), and the crediting below covers only the
  first two.
- **A shared random event fires in about 0.15 of the careers it can fire in** (**fitted**, on SR and
  SSR cards), a lower bound. The rate falls as more events are on offer in sample, but the $K$ form is not shown to
  predict unseen runs better than a constant ($z$ = +1.72).
- **Known miss of the measured rate: it differs by scenario**, highest in Unity Cup and Trackblazer, next to $K$ and
  next to the card effects. It changes no prediction, since the model leaves these events out.
- **Crediting them has no clear held-out advantage** on the rows the model gives a positive chance.
- **Known miss of the measured rate: it differs between characters**, which may be how well each character's
  marker shows as much as how often her event fires. Across characters a higher initial friendship gauge goes with a
  higher rate; within a character no card effect is detectable, but that test is underpowered: its interval for the
  gauge still holds the effect across characters. It changes no prediction, since the model leaves these events out.

### Evidence

*The rate.* A hit is a top-choice marker credited to the character's event, alone or in a shared credit ("A or her
event") whose other card cannot give the marker. As in section 8, a reading is left out as unknown when another deck
card's events can give the marker, whatever was credited. The 10,534 readings come from
6,043 runs and give 1,596 hits; 730 more readings are left
out as unknown, and 443 readings of R cards are not read.

A firing goes unseen when IT takes another choice for a prioritised skill, or when the marker is already at Lv 5
from other sources (183 readings, counted as misses). So the
fitted rate is a lower bound. Leaving the Lv 5 readings out gives 0.154; counting every
unknown reading as a hit as well gives 0.210. That
is an upper bound for these readings only: firings hidden because IT took another choice stay unseen, so it does not
bound how often the event fires.

The *$K$ form* lowers the rate by a fixed fraction for each extra event on offer. The card effects and scenario are
fitted beside $K$ in a logistic model.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| The rate falls as more events are on offer | $K$ form: 0.21 at $K$ = 4, 0.13 at 14 | run-clustered Wald $\lvert z \rvert$ = 5.13 | detectable |
| The $K$ form predicts unseen runs better than a constant | constant 0.152 | held out +8.8 ± 5.1 nats, $z$ = +1.72 | not shown |
| Across characters, the initial friendship gauge goes with a higher rate | +0.127 [+0.074, +0.180] logit per 10 points | Holm-adjusted run-clustered $p$ = 0.0000 | detectable across characters |
| Friendship bonus moves it | −0.039 logit per 10 points | Holm-adjusted $p$ = 0.77 | not detectable |
| Specialty priority moves it | +0.007 logit per 10 points | Holm-adjusted $p$ = 0.77 | not detectable |
| The rate differs between characters (character intercepts next to $K$ and the card effects) | | run-clustered Wald $\chi^2$ = 853, 26 df | known miss |
| Within a character, the initial friendship gauge moves it | +0.045 [−0.044, +0.133] logit per 10 points | Holm-adjusted $p$ = 0.97 | not detectable; underpowered (the interval holds the effect across characters) |
| Within a character, the rate falls as more events are on offer | | run-clustered $z$ = −2.92 | detectable |
| Scenario moves it | Unity Cup +0.22, Trackblazer +0.24 logit against URA Finale | run-clustered $p$ = 0.008; next to the card effects 0.001 | known miss |
| SR against SSR within a character | | run-clustered $z$ = +1.23 | not detectable |

Pairs of one run's readings that both hit number 105, against
122.9 from one rate per run ($z$ = −1.51) and
123.2 from a rate per reading by $K$, card effects, character and
scenario ($z$ = −1.52). Neither departure is detectable, in either
direction. Independent firing remains an assumption.

*What does leaving them out cost on real careers?* The test uses section 13's rows, each on the run's own deck. It
scores whether the target was acquired from any source, so it does not depend on reading the event. The acquisition
chance is recovered from the exact landing chance. Since a gold sparks with twice a white's chance, section 12's formula
for a target that is not guaranteed reads $p = W(a + g)$, with $a$ the chance the target is hinted, $g$ the chance its
gold is and $W$ the white spark chance; so $a = p/W - g$. The exact model gives $p$, but $g$ is the career report's
approximate gold chance, so the baseline carries the gap between the two. These comparisons are therefore neither an
exact calibration of acquisition nor a clean test of the report's approximation. Crediting uses the $K$ form at each
run's $K$, about 0.15 on these rows.

| Rows where a deck card's random event could hint the target | Count |
|---|---|
| all | 312 |
| left out: the card is an R card | 0 |
| target credited to that event | 37 |
| the same, with no other source | 24 |
| rows the model gives no chance | 73 |
| of those, hinted by the event | 6 |

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Left out, these targets are under-predicted | 1.24 [1.11, 1.38] | interval excludes 1 | known miss |
| Credited at the fitted rate, they are not | 0.99 [0.89, 1.10] | interval covers 1 | consistent |
| Crediting predicts unseen runs better | | held out +5.2 ± 2.9 nats, $z$ = +1.78 | not shown |
| Without such a card, the same targets read as | 0.96 [0.95, 0.98] over 11,639 rows; 0.96 weighted to the same targets | | the comparison's baseline |
| Left out, they read above the control | left out minus control: +0.28 ± 0.07 | run-clustered $z$ = +4.12 | yes |
| The same, matched on target and predicted acquisition | 1.29 of the control's rate | run-clustered $z$ = +3.85 | yes |
| The same, on decks where no card's training hints give the target | 1.46 against 1.00 (174 and 6,334 rows); matched 1.45 | run-clustered $z$ = +3.21; matched +2.95 | yes |
| Credited at the fitted rate, they read as the control | credited minus control: +0.02 ± 0.05 | run-clustered $z$ = +0.41 | consistent |

The held-out score refits the rate on the other folds' readings. It covers only the 239 rows with a positive
baseline prediction: a row the model gives no chance has no finite likelihood when acquired. So it leaves out the rows
where an omitted source matters most, and it cannot settle the value of adding the source.

Crediting raises the product of positive per-target landing marginals by a median
14%. The largest rise, over the 239 runs with a credited row (a run, not a row: each run's product of odds), is 163%. This uses the hybrid reconstruction above. It does not recompute exact all-target odds.

## 11. Conditional character events (not modelled)

Computed by `research/s11_conditional.py`.

The trainee's secret and race-result events are left out of the model, except Tokai Teio's Head-On. A
*conditional-only row* is a run-target row where only such an event can give the target: none of the trainee's story,
date, outfit or shared random events can, nor the scenario's.

### Findings

- **The sparse pooled conditional-only comparison is inconclusive.** Its score excludes zero-baseline rows,
  including 3 acquisitions. Most of its 112
  rows are Daiwa Scarlet's (99), which read as the model
  predicts, so the pooled rows hide the trainees below.
- **Known miss: Mejiro Ardan's own Ramp Up.** Her events gave it in
  9 of 9 rows.
  The model gives 3 of those rows no chance,
  and all 3 were acquired.
- **Known miss: Vodka's own Nimble Navigator.** Her events give it in most of her runs, and on her own rows the model
  under-predicts it. Her random and conditional events both give it and cannot be told apart.
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

| Trainee and target, model as it is | Acquired / predicted | Observed / predicted [95%] | Rows the model gives no chance (acquired) |
|---|---|---|---|
| Daiwa Scarlet, any outfit, Nimble Navigator | 26 / 26.8 | 0.97 [0.70, 1.24] | 25 (0) |
| Mejiro Ardan, any outfit, Ramp Up | 9 / 4.6 | 1.96 [0.99, 2.93] | 3 (3) |
| Vodka, any outfit, Nimble Navigator | 30 / 17.0 | 1.76 [1.51, 2.02] | 0 (0) |

With so few rows Mejiro Ardan's interval is rough; the decisive part is the rows the model gives no chance, each of
them acquired. Fine Motion's 4 rows are too few to read.

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
fitted and no research script computes it. An independent implementation, `val/joint_check.py`, reproduces the solver's exact
odds on the 42 loop-planner reference results and their listed runner-ups, from the
reported shared wins and relation points (section 6). That covers the loop planner. The career report's path for real
legacies, described at the end of this section, is what section 13 scores; `val/career_check.py` recomputes its
reports independently from each career job and the data (slot affinities, $q_k$, carriers, spark scales, landing
factors and the joint $P(\text{all})$, the deck model restated by `val/joint_check.py`), a validation check outside
these scripts.

A target lands if the deck hints the gold and it sparks, or else if any source hints the white and that sparks. For a
target $k$ not guaranteed:

$$p_k = (1 - m^{g}_k)\,G_k + W_k\left[m^{g}_k - m^{0}_k(1 - e_k)(1 - q_k)\right].$$

$G_k = 0.4\cdot 1.1^{n_k}$ and $W_k = 0.2\cdot 1.1^{n_k}$ are the gold and white spark chances, with $n_k$ ancestors
carrying the target (section 1). $m^g_k$ is the chance the deck gives no gold hint, and $m^0_k$ the chance it gives no
hint of either tier, each joint over the deck's sources. $e_k$ is the chance the trainee's own events hint the white,
and $q_k$ the chance it is inherited (section 3). The bracket is the chance the white is hinted without the gold. Its
miss product assumes the deck, own events and inheritance independent given the model inputs. Inheritance only ever
supplies the white.

For a planned loop, a pair's shared G1 wins are not known in advance. The model puts their expected count into the
affinity, and so into $q_k$, in place of averaging over the counts the races could give (**assumed**). Since $q_k$ is
not linear in the wins and every target in a lineage shares them, the two can differ in either direction; no script
measures by how much. A recorded career uses its recorded wins, so section 13 does not test this.

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

- **On targets of unknown priority the model is optimistic where it rates targets likely**, a known miss. Counting
  only the sources the model has, the bands it rates 0.6 or more read below it, and the lower bands show no
  detectable miss; the pooled ratio reaches 1 because scenario and trainee events the model leaves out fill the gap.
  Prioritised targets are hinted slightly more often than predicted, a known miss, with no detectable pattern by band.
- **Over all targets the model is slightly optimistic**, through the targets a shown list leaves out.
- **Golds a deck can give are hinted less often than predicted where the target is not shown prioritised**, a known
  miss; prioritised golds show no detected miss.
- **Landing every target in one career is rarer than the model predicts.** Runs showing no priority list read lower
  than runs showing one, but the difference is not detectable. Scaling each target's acquisition chance to its
  measured miss, in-sample, brings the all-target ratio back to 1, so the acquisition part of the shortfall is the
  per-target optimism compounded over the targets. On runs showing no list the gold tier adds to it. The check rests
  on few careers, since most of the predicted chance sits in a few strong ones, and it does not measure spark
  generation.
- **Inheritance reads slightly below the model, 0.990 of it, with no difference
  between targets detected**, a known miss. Section 3's parent weight or its affinity slope removes it, in-sample.

These checks are in-sample. Sections 3, 5, 7, 8 and 9 fitted their constants on these same careers, so a check that
passes here is weaker evidence than one on careers the fits never saw. Section 7's fit reads only cards at their top
limit break, so the training-pool rows on lower limit breaks are outside it.

These checks test whether the model's chances match recorded careers, not whether it ranks candidate loops correctly.
Acquisition uses the career report's approximation, which multiplies chances across cards. Agreement in aggregate can
hide errors in the parts that cancel, and misses inside a band of predictions. The comparisons beyond the default
targets test a broader sample of skills under the same restrictions.

### What is compared

Each real career is rebuilt as the model sees one: the trainee in her outfit, the scenario, the six cards at their own
limit breaks, both parents with their recorded parents, every ancestor's white sparks, and race wins giving the
recorded shared-G1 counts. Each chain-ending choice follows the run's priority list where it shows one: a certain end
is pinned as a stop at its link, an impossible one is removed, and a fractional one, like every run showing no list, is
left to the solver, which may take the end as a player following its advice would. The model gives each default target's chance at each step (sections 3-12), and the run
record shows what happened.

| Careers | URA Finale | Unity Cup | Grand Concert | Trackblazer | Run-target rows | On her outfit | Scenario golds |
|---|---|---|---|---|---|---|---|
| 8,373 | 1,001 | 1,695 | 5,564 | 113 | 66,984 | 3,329 | 3,390 |

Rows on her outfit and scenario golds are *certain*, so tests 1 and 3 leave them out; the rest are *open targets*.
Unity Cup's guaranteed golds show in 1,695 of its 1,695 runs. The runs end before skills are bought, so
"acquired" means hinted, the model's own meaning.

Test 1 scores the career report's deck hint chances (`buy.deck_gold`, `buy.deck_white`), a product approximation that
multiplies chances across cards, not the exact joint engine of section 12; shared chain progress makes the two differ.
Test 3 scores its buying chances (`buy.gold`, `buy.white`), which are exact per target: the gold chance over the
deck's chain states, the white part the rest of the exact landing chance. The landing and all-targets checks use the
exact engine's $p_k$ and $P(\text{all})$.

1. **Deck hint**: does a deck card hint the skill, by training or by its events, as often as sections 7-9 predict? The
   prediction is $d^g + (1 - d^g)d^w$, with $d^g$ the gold chance and $d^w$ the white chance given no gold. Only rows
   where no inspiration hinted the skill count, which treats inspiration as independent of the deck (section 7). The
   rows split by where the deck's chance comes from, **training pool only** or **events only**, and events only by
   whether the run's list shows the target **prioritised** (section 9).
2. **Inheritance**: does an inspiration of the skill fire as often as $q$ (section 3)?
3. **Acquired**: does the skill end up hinted, from any source, as often as all modelled sources together predict?
   This is what the odds rest on. The **prioritised or unknown** rows pool prioritised targets with the runs that show
   no list, whose priorities are unknown.
4. **Gold acquired**: on open targets whose gold a deck card can give, is the gold hinted as often as predicted?

| Test | Rows | Predicted | Obs / pred [95%] | Verdict |
|---|---|---|---|---|
| 1. Deck hint | 38,605 | 0.259 | 0.946 [0.936, 0.956] | optimistic |
| training pool only | 7,239 | 0.358 | 0.994 [0.965, 1.022] | covers 1 |
| events only | 5,689 | 0.871 | 0.902 [0.892, 0.913] | optimistic |
| events only, target prioritised | 1,141 | 0.833 | 0.997 [0.975, 1.0188] | covers 1 |
| events only, not prioritised | 1,670 | 0.867 | 0.776 [0.753, 0.799] | far below: the priority assumption fails |
| 2. Inheritance | 66,984 | 0.374 | 0.990 [0.982, 0.997] | below 1: known miss; no difference between targets detected ($p$ = 0.17) |
| Tail Held High | | | 0.972 [0.947, 0.997] | the one target whose own interval excludes 1 |
| 3. Acquired, any source | 60,265 | 0.528 | 0.990 [0.985, 0.995] | slightly optimistic |
| prioritised or unknown | 45,134 | 0.529 | 1.002 [0.996, 1.008] | covers 1 |
| prioritised | 10,404 | 0.665 | 1.014 [1.005, 1.024] | slightly above 1: known miss |
| not prioritised | 15,131 | 0.526 | 0.953 [0.943, 0.963] | optimistic |
| unknown priority | 34,730 | 0.488 | 0.997 [0.989, 1.005] | covers 1 pooled, through unmodelled sources; optimistic at 0.6 and above: known miss |
| 4. Gold acquired, golds not predicted certain | 17,175 | 0.217 | 0.958 [0.944, 0.973] | optimistic: known miss |
| prioritised | 3,570 | 0.358 | 0.994 [0.967, 1.021] | covers 1 |
| unknown priority | 10,826 | 0.134 | 0.932 [0.910, 0.953] | optimistic |
| not prioritised | 2,779 | 0.359 | 0.951 [0.923, 0.979] | optimistic |

### Evidence


*Acquisition reliability.* Do targets the model rates at a given chance get hinted at that chance? The rows are
grouped into bands of predicted chance, and each band's observed rate is followed by its mean prediction. The product
approximation misses acquisitions in its zero-prediction bucket. These are checks of the acquisition approximation on
open targets, not the exact engine.

| Predicted acquisition | Rows, observed (mean prediction) |
|---|---|
| 0 | 13,813, 0.009 |
| above 0 to 0.20 | 2,807, 0.159 (0.16) |
| 0.20-0.40 | 6,167, 0.307 (0.30) |
| 0.40-0.60 | 6,645, 0.507 (0.50) |
| 0.60-0.80 | 11,667, 0.699 (0.72) |
| 0.80-0.95 | 9,136, 0.842 (0.86) |
| 0.95-1 | 10,030, 0.981 (1.00) |

Pooled over every row the bands look close, but that mixes three kinds of target. Split by priority, a joint test
asks whether every band with a positive prediction matches at once: a run-clustered Wald $\chi^2$, one degree of
freedom per band, nothing fitted. The model's prediction covers only the deck and inspiration, so on unknown-priority
rows the bands are also counted from those *modelled sources* alone. Counted that way the bands under 0.6 read
0.985 [0.961, 1.009] of the model, no detected
miss, and the bands of 0.6 or more 0.979 [0.972, 0.986]. The excess
the any-source column shows in the low bands is acquisitions the model leaves out. Prioritised rows show no pattern
by band.

| Predicted acquisition | Unknown priority, modelled sources | Unknown priority, any source | Prioritised | Not prioritised |
|---|---|---|---|---|
| above 0 to 0.20 | 0.157 | 0.164 (0.157) | 0.176 (0.153) | 0.131 (0.153) |
| 0.20-0.40 | 0.300 | 0.317 (0.306) | 0.292 (0.288) | 0.289 (0.290) |
| 0.40-0.60 | 0.491 | 0.505 (0.498) | 0.517 (0.513) | 0.505 (0.507) |
| 0.60-0.80 | 0.691 | 0.697 (0.710) | 0.748 (0.727) | 0.659 (0.727) |
| 0.80-0.95 | 0.832 | 0.835 (0.855) | 0.880 (0.868) | 0.820 (0.856) |
| 0.95-1 | 0.987 | 0.987 (0.998) | 0.992 (0.994) | 0.962 (0.998) |
| joint $p$ | 5.8e-14 | 2.0e-12 | 0.08 | 1.9e-34 |

Where the model rates targets 0.6 or more likely, prioritised rows read 1.012 of it and
not-prioritised rows 0.945; the unknown-priority rows' 0.983 is what a mix of
57% prioritised targets would give. That the runs showing no list hold such a mix is an inference; it is
not tested.

*Where the deck loses its points.* The deck's rows predicted 0.6 or more hold 98% of its shortfall,
and 95% of those rows have an event source. On prioritised-or-unknown rows, the deck and inspiration alone
read 0.988 of the model. Unmodelled scenario and trainee events close the gap to the any-source row: they gave
376 target acquisitions, 0.6% of open rows. Agreement after adding them can hide errors in the parts, and it
does not validate candidate rankings.

*Per target*, observed / predicted:

| Target | Deck hint [95%] | Inheritance | Acquired |
|---|---|---|---|
| Uma Stan / Superstan | 0.89 [0.81, 0.98] | 0.991 | 0.985 |
| Nimble Navigator / No Stopping Me! | 1.04 [0.96, 1.13] | 0.999 | 1.044 |
| Playtime's Over! / See Ya Later! | 0.993 [0.98, 1.01] | 1.003 | 0.996 |
| Ramp Up / It's On! | 0.864 [0.84, 0.89] | 0.990 | 0.930 [0.91, 0.95] |
| Tail Held High / Tail Nine | 1.017 [0.97, 1.06] | 0.972 [0.947, 0.997] | 1.002 |
| Racing Spirit: Stamina | no deck source | 0.992 | 1.025 |
| Slipstream / Perfect Spot! | 0.980 [0.92, 1.04] | 0.983 | 0.989 |
| Risky Business / Nothing Ventured | 0.873 [0.83, 0.92] | 0.946 | 0.926 [0.89, 0.96] |

*Gold acquisition.* On open targets whose gold a deck card can give, is the gold hinted as often as predicted? Every
such gold in the runs is listed. The joint test asks whether every gold
with an uncertain prediction matches at once (run-clustered Wald $\chi^2$, nothing fitted): $\chi^2$ =
1021.0 on 8
golds, $p$ = 4.3e-215. See Ya Later! is predicted certain on every row
and always shows, so the joint test leaves it out, and so does the pooled test of the other golds:
0.958 [0.944, 0.973]. Split by the target's priority, prioritised golds
read 0.994 [0.967, 1.021], no detected miss, while unknown-priority golds read
0.932 and not-prioritised ones 0.951, the same pattern as the event-only hints of test 1.

| Gold | Rows | Observed / predicted [95%] |
|---|---|---|
| No Stopping Me! | 1,659 | 0.985 [0.80, 1.20] |
| See Ya Later! | 7,964 | 1.000 [0.994, 1.005] |
| It's On!, outside Unity Cup | 2,577 | 0.921 [0.82, 1.03] |
| Perfect Spot! | 2,277 | 0.920 [0.86, 0.98] |
| Nothing Ventured | 2,301 | 0.923 [0.88, 0.97] |
| every gold | 23,005 | 0.984 [0.978, 0.989] |

*Training-pool hints by scenario* test $\beta$ and $d$ (section 7). Most of these rows come from cards at their top
limit break, the rows section 7 fitted on, so they check the fit in-sample. Only Trackblazer's interval excludes 1:
its $\beta$, fitted in section 7, over-predicts these 113 careers. The rows differ (here
only targets, at every limit break), and why it misses is not tested. Split instead by the lowest limit break of
the sourcing cards, the rows outside section 7's fit read LB0 0.977
[0.86, 1.09]
(370 rows), LB1 1.019
[0.88, 1.16]
(309) and LB2 0.946
[0.83, 1.06]
(310). Each covers 1, but each interval is wide.

| Scenario | Grand Concert | Unity Cup | URA Finale | Trackblazer |
|---|---|---|---|---|
| Observed / predicted [95%] | 1.011 [0.98, 1.04] | 0.931 [0.85, 1.01] | 1.014 [0.92, 1.10] | 0.746 [0.60, 0.89] |

*Every open target in one run.* Does the share of runs acquiring all their open targets match the mean product of
their per-target predictions? Over every run it reads 0.926 of it, and the interval reaches
the prediction, so no miss is detected; the same holds over the runs whose open targets are each prioritised or of
unknown priority. With 357 such runs this check is weak:

| Runs | Product of the model's odds | Observed [95%] |
|---|---|---|
| all | 0.0461 | 0.0426 [0.0385, 0.0472] |
| 4,927 with every open target prioritised or unknown | 0.0289 | 0.0276 [0.0234, 0.0326] |

*Spark-weighted acquisition of all targets.* $P(\text{all eight})$ is recomputed from what each run acquired and
compared with the model's exact $P(\text{all})$; Results gives the ratio, its interval from 10,000 resamples of whole
runs. The ratio weights each run by its predicted $P(\text{all})$, so a few strong careers carry it: its
8,373 runs are worth 386 equally
weighted ones (the *effective runs*, $(\sum w)^2 / \sum w^2$).

Two checks of how fragile that is. The *BCa* interval corrects the bootstrap's percentiles for its bias and skew
(Efron's bias-corrected and accelerated interval). Leaving out the runs the model gave the highest $P(\text{all})$
shows how much a handful of careers move the ratio. With so few effective runs, either interval's 95% coverage is
doubtful.

| Run set | Runs | Effective runs | Observed / predicted [95%] | BCa 95% | Without the top 10 / 25 / 50 |
|---|---|---|---|---|---|
| every run | 8,373 | 386 | 0.863 [0.766, 0.961] | [0.765, 0.960] | 0.858 / 0.866 / 0.890 |
| showing a priority list | 3,545 | 288 | 0.885 [0.774, 0.999] | [0.774, 0.999] | 0.882 / 0.893 / 0.912 |
| showing none | 4,828 | 103 | 0.776 [0.594, 0.979] | [0.600, 0.986] | 0.836 / 0.803 / 0.892 |

Both run sets read below 1; the list runs' upper bound sits just under 1, at 0.999, so how far they are from 1 is
not settled. Do they differ? Resampling each set on its own, the no-list ratio minus the list ratio is
−0.109 [−0.323, +0.120], so no difference is detectable.

Where does the shortfall come from? The same comparison under the career report's approximation (the product over
targets of its per-target buying chances, each exact, multiplied as if the targets were independent), counted once by tier
and once *tier-blind* (every acquisition scored as a white, on both sides), separates the gold tier from acquisition
itself. Does counting tier-blind change the ratio? Both are scored on the same resamples of runs, and the difference
is the test:

| Run set | By tier | Tier-blind | Tier-blind minus by tier [95%] | Tier-blind, each target recalibrated [95%] |
|---|---|---|---|---|
| every run | 0.873 | 0.894 | +0.021 [−0.037, +0.082] | 0.989 [0.90, 1.08] |
| showing a list | 0.896 | 0.888 | −0.008 [−0.078, +0.061] | 0.987 [0.88, 1.09] |
| showing none | 0.786 | 0.915 | +0.129 [+0.019, +0.242] | 0.996 [0.81, 1.19] |

The gold tier adds to the shortfall only on runs showing no list, where the unknown-priority golds read short (the
gold table above). Over every run and on runs showing a list it adds nothing detectable, and the tier-blind ratio
itself lies below 1. What is left is acquisition. The last column tests whether that is the per-target miss: each
open row's acquisition chance is scaled by the observed / predicted ratio of its cell, one of
18 cells of priority status and band of prediction, and the all-target ratio
is recomputed. It returns to about 1 on every run set, and also counting only modelled sources
(0.996). The cells are fitted on the same rows
they rescore, so this is in-sample and returns to 1 partly by construction. It is consistent with the acquisition
shortfall being the per-target misses compounded over the targets; it does not test section 12's joint. Only
357 runs acquired every open target, too few to separate the exact joint from
the product.

*Single cards.* Where one card is the deck's only source of a target, the deck test scores that card alone. The table
lists every such pair of 40 or more rows whose interval excludes 1.

| Card > target | Rows | Predicted | Obs / pred [95%] | Possible reason (untested) |
|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | 628 | 1.000 | 0.76 [0.73, 0.80] | the model assumes Risky Business prioritised, which takes finale choice 2 |
| Riko Kashimoto [Planned Perfection] > Ramp Up | 1,576 | 0.750 | 0.75 [0.71, 0.78] | open; her Lone Wolf choice, when prioritised, accounts for part |
| King Halo [Tonight, We Waltz] > Uma Stan | 189 | 0.870 | 0.84 [0.76, 0.91] | Uma Stan sits on a choice that needs a priority |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | 83 | 0.308 | 0.63 [0.40, 0.94] | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | 182 | 0.332 | 1.21 [1.00, 1.43] | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | 581 | 0.709 | 0.930 [0.875, 0.983] | open |
| Tokai Teio [Step! Smile! Wink!] > Nimble Navigator | 155 | 0.418 | 1.30 [1.11, 1.48] | open |
| Yukino Bijin [Dancing Light into the Night] > Playtime's Over! | 42 | 0.155 | 0.15 [0.03, 0.79] | the skill sits on a chain-ending choice 2, taken only when prioritised |

### Details

Further checks: landing on top of acquisition, dependence between targets, single cards with few rows, and the
skills beyond the default targets.

*Landing.* Each open row's spark chance is recomputed from what the run acquired: $0.4 \cdot 1.1^n$ if the gold was
hinted, $0.2 \cdot 1.1^n$ if only the white. Summed, it reads 0.990 of the model's $p$. Both
sides use the model's own spark chances, so this checks the mix of gold and white acquisitions, not spark generation
(section 1).

*Dependence between targets.* Section 12 takes a run's inspirations of different targets as independent given its
lineage. If two targets of one run tended to be inherited together, or one instead of the other, the product of their
misses against the model would average away from 0. Over 234,444
target pairs it reads $z$ = −0.16, so no such dependence is detectable. The mean product is
−0.00005 with a 95% half-width of 0.00056, so a residual covariance between two targets'
inspirations larger than that would have shown.

*King Halo's Uma Stan follows the priority.* The expectation and the exact Poisson-binomial tail are the model's:

| Runs that list Uma Stan | Rows | Hinted by her card | Expected | $P(X \le \text{hinted})$ |
|---|---|---|---|---|
| prioritised | 74 | 64 | 66.7 | 0.180, no miss detected on these few rows |
| a list without it | 25 | 0 | 23.0 | |

*Smaller pairs.* Below 40 rows, 4 more pairs exclude 1. 2 of them are
chain-ending event sources of Playtime's Over! (its three are Yukino Bijin [Dancing Light into the Night], Yaeno Muteki
[Fiery Discipline] and Sirius Symboli [Escorte Étoile]; one with 40 or more rows sits in the table above). They hinted
it in 4 of 49 runs, where the model predicted at least 0.29. Their event puts the skill on
choice 2, which ends the chain before its last link, so IT takes it only when the skill is prioritised. Reading each
run's list accounts for the miss on runs that show one: rows whose list leaves it out are predicted none and show none,
and the 11 prioritised rows (4 hinted, interval
[0.51, 2.16] of the model) are too few to show a miss either way. Runs showing no list stay below
the model, which there assumes the end can be taken.

*Other skills.* Every non-target skill a deck card or spark could give, on a 30% sample of 2,515 runs:

| Test | Rows | Observed / predicted [95%] | Verdict |
|---|---|---|---|
| inheritance | 104,219 | 0.979 [0.969, 0.990] | slightly short: known miss; no one skill carries more than 5% of it |
| training-pool hints | 39,123 | 1.038 [1.028, 1.047] | under-predicted: known miss |
