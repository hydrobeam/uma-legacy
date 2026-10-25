# The model's mathematics

What this project measured, fitted, tested or derived for its model of how a career in Uma Musume's Independent
Training acquires and passes on skills. Every constant the model uses, with its source, is in `docs/MODEL.md`; this
document keeps the evidence.

## Results

On {{13.runs_used|,.0f}} real careers, each rebuilt from its own deck and lineage, the acquisition product approximation
has no detected pooled bias on unknown-priority rows and reads slightly low on known-prioritised ones (section 13).
Over all careers it is slightly optimistic, and landing every target at once comes out below its prediction.
Terms, labels and the limits of every test are in *How to read*.

| § | Conclusion | How it is set | Main evidence |
|---|---|---|---|
| 2 | Grand Concert defaults to the 17,500+ star mix; URA Finale, Unity Cup and Trackblazer to the lower one | default chosen from rank scores | Grand Concert: {{2.grand_concert_share_at_17_500_or_more|.0%}} of careers reach 17,500+; the largest relative overstatement from averaging stars was {{2.q_overstatement_below_17_500_max|.2%}} on sampled affinities under the assumed independent-slot star mixes |
| 3 | Grandparents pass sparks at the parents' full base rate | fitted | weight {{3.grandparent_weight_fitted|.2f}} [{{3.grandparent_weight_95_pct_low|.2f}}, {{3.grandparent_weight_95_pct_high|.2f}}] against the parents' 1 |
| 3 | The community affinity formula is kept, with +3 per shared G1 win | assumed, tested | held-out gains do not establish improvements over unit weights: slope $z$ = {{3.affinity_slope_refitted_over_slope_1_held_out_z|.2f}}, shared wins $z$ = {{3.race_part_slope_refitted_over_slope_1_held_out_z|.2f}} |
| 4 | The community win table holds on grades raised by pink sparks, with a goal race charged the streak position before it | community table, tested; goal shift fitted | wins {{4.race_obs_per_pred_pink|.3f}} [{{4.race_obs_per_pred_pink_95_pct_low|.3f}}, {{4.race_obs_per_pred_pink_95_pct_high|.3f}}] of predicted, over {{4.race_attempts_at_the_34|,.0f}} G1 attempts |
| 5 | A trainee who is her own grandparent gets no trio points from it | fitted | {{5.ll_0_ll_pair_nats|.0f}} nats better than counting the pair's points |
| 6 | Scheduling loops for total shared wins costs little | derived | with each loop's decks fixed, no schedule raises a reference loop's odds by more than {{6.schedules_pb_ceiling_gain_max_pct|.2f}}% |
| 7 | A card's training hints are uniform draws from its pool, a number per scenario raised by Hint Frequency and Specialty Priority | form fixed on the runs; base and specialty weight fitted | real careers' training-pool hints: {{13.1_training_pool_only_obs_per_pred|.3f}} [{{13.1_training_pool_only_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_ratio_95_pct_high|.2f}}] of the model |
| 8 | Chain events are Binomial(21, $p$) with $p$ rising with the deck's pals, each event going to a running chain card with equal chance | fitted | predicts unseen runs better than today's Binomial(21, 0.35) with links-left weighting: $z$ = {{8.held_out_pal_count_budget_over_today_s_model_z|.1f}}; than the same budget with links-left weighting: $z$ = {{8.held_out_pal_count_budget_over_the_same_with_the_slot_rule_z|.1f}} |
| 9 | IT takes a prioritised skill's choice; four outcome estimates replace equal shares | rule assumed, tested on careers; shares fitted | prioritised targets' event hints read at the model's rate: {{13.1_events_only_prioritised_obs_per_pred|.3f}} [{{13.1_events_only_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_prioritised_ratio_95_pct_high|.3f}}] |
| 10 | Random card events are left out | not modelled | the targets they can hint read {{10.calibration_obs_per_pred_left_out|.2f}} of the model |
| 11 | Conditional character events are left out | not modelled | sparse pooled comparison inconclusive; Vodka's own Nimble Navigator is a known miss |
| 12 | The parts compose into an exact $P(\text{all})$ under stated independence assumptions | derived | an independent implementation reproduces the solver's odds on every reference loop |
| 13 | The acquisition product approximation has no detected pooled bias on unknown-priority rows; known-prioritised rows read slightly above it | tested | unknown priority {{13.3_acquired_unknown_priority_obs_per_pred|.3f}} [{{13.3_acquired_unknown_priority_ratio_95_pct_low|.3f}}, {{13.3_acquired_unknown_priority_ratio_95_pct_high|.3f}}] on {{13.3_acquired_unknown_priority_rows|,.0f}} rows; prioritised {{13.3_acquired_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_prioritised_ratio_95_pct_high|.3f}}] on {{13.3_acquired_prioritised_rows|,.0f}} rows |
| 13 | Over all rows the model is slightly optimistic, mostly on targets shown not prioritised | tested | all rows {{13.3_acquired_any_source_obs_per_pred|.3f}} [{{13.3_acquired_any_source_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_ratio_95_pct_high|.3f}}]; targets shown not prioritised {{13.3_acquired_not_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_not_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_not_prioritised_ratio_95_pct_high|.3f}}], {{13.3_acquired_shortfall_share_on_non_prioritised_rows|.0%}} of the shortfall; the cause is not established |
| 13 | Landing every target at once comes out below the model: it is optimistic about $P(\text{all})$ | tested | spark-weighted all-target acquisition {{13.p_all_eight_ratio_all|.2f}} [{{13.p_all_eight_ratio_all_95_pct_low|.2f}}, {{13.p_all_eight_ratio_all_95_pct_high|.2f}}] of the model; runs showing no priority list {{13.p_all_eight_ratio_no_list_shown|.2f}}, runs showing one {{13.p_all_eight_ratio_list_shown|.2f}} [{{13.p_all_eight_ratio_list_shown_95_pct_low|.2f}}, {{13.p_all_eight_ratio_list_shown_95_pct_high|.2f}}]; only {{13.runs_acquiring_every_open_target|.0f}} runs acquired every target |

### Known misses

Each is measured in its section. *How to read* says why a listed miss can be partly chance.

| Part | Miss | Observed / predicted | Section |
|---|---|---|---|
| Inheritance | Tail Held High is inherited less often than predicted; cause unknown | {{13.tail_held_high_inheritance|.3f}} [{{13.tail_held_high_inheritance_95_pct_low|.3f}}, {{13.tail_held_high_inheritance_95_pct_high|.3f}}] | 3, 13 |
| Inheritance | parents at affinity 200 or more pass sparks less often | {{3.parents_at_affinity_200_and_above_obs_per_pred|.3f}} [{{3.parents_at_affinity_200_and_above_95_pct_low|.3f}}, {{3.parents_at_affinity_200_and_above_95_pct_high|.3f}}] | 3 |
| Inheritance | non-target skills, the shortfall spread over many skills | {{13.other_skills_inheritance|.3f}} [{{13.other_skills_inheritance_95_pct_low|.3f}}, {{13.other_skills_inheritance_95_pct_high|.3f}}] | 13 |
| Races | aptitude basis {{4.low_aptitude_basis_bucket|s}}, before streak penalties | {{4.race_pink_bp_lt_70_obs_per_pred|.2f}} [{{4.race_pink_bp_lt_70_obs_per_pred_95_pct_low|.2f}}, {{4.race_pink_bp_lt_70_obs_per_pred_95_pct_high|.2f}}] | 4 |
| Races | chosen races 1st and 2nd in a row | {{4.race_pink_chosen_j_1_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_1_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_1_obs_per_pred_95_pct_high|.3f}}]; {{4.race_pink_chosen_j_2_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_2_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_2_obs_per_pred_95_pct_high|.3f}}] | 4 |
| Races | goal races 7th or later in a row, which the planner charges as the 6th | {{4.race_pink_goal_j_ge_7_obs_per_pred|.2f}} [{{4.race_pink_goal_j_ge_7_obs_per_pred_95_pct_low|.2f}}, {{4.race_pink_goal_j_ge_7_obs_per_pred_95_pct_high|.2f}}] | 4 |
| Training hints | cards differ beyond their effects; cause open | per card {{7.per_card_obs_per_pred_min|.2f}}-{{7.per_card_obs_per_pred_max|.2f}} | 7 |
| Training hints | a skill also hinted by inspiration | {{7.spark_inspiration_ratio|.2f}} | 7 |
| Training hints | a skill another card's training also hinted | {{7.spark_other_card_training_ratio|.2f}} | 7 |
| Training hints | more pals per deck give slightly more hints | factor {{7.covariate_factor_pals_per_deck|.3f}} per pal | 7 |
| Training hints | Trackblazer's base over-predicts real careers | {{13.1_training_pool_only_trackblazer_obs_per_pred|.3f}} [{{13.1_training_pool_only_trackblazer_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_trackblazer_ratio_95_pct_high|.2f}}] | 13 |
| Training hints | non-target skills are under-predicted | {{13.other_skills_training_pool_hints|.3f}} [{{13.other_skills_training_pool_95_pct_low|.3f}}, {{13.other_skills_training_pool_95_pct_high|.3f}}] | 13 |
| Chain events | Inari One's careers reach fewer events than the pal-count budget predicts | own logit term {{8.inari_one_own_term_on_the_pal_count_budget|+.3f}}, LR $p$ = {{8.inari_one_own_term_on_the_pal_count_budget_lr_p|.3f}} | 8 |
| Chain events | early links run short at 3 pals and long at 1 | $z$ = {{8.early_link_shortfall_under_the_budget_3_pals_z|+.1f}} and {{8.early_link_shortfall_under_the_budget_1_pals_z|+.1f}} | 8 |
| Chain events | with five chain cards, chains that never start are slightly over-predicted | {{8.never_started_5_chain_cards_observed|.2f}} [{{8.never_started_5_chain_cards_observed_95_pct_low|.2f}}, {{8.never_started_5_chain_cards_observed_95_pct_high|.2f}}] against {{8.never_started_5_chain_cards_final_model|.2f}} | 8 |
| Loop ranking | a listed runner-up's pair-ceiling odds pass its winner's | {{6.schedules_runner_ups_whose_pb_ceiling_reaches_the_winner|.0f}} of {{6.schedules_runner_up_loops_checked|.0f}} runner-ups | 6 |
| Calibration | landing every target at once reads below the model | {{13.p_all_eight_ratio_all|.2f}} [{{13.p_all_eight_ratio_all_95_pct_low|.2f}}, {{13.p_all_eight_ratio_all_95_pct_high|.2f}}] | 13 |
| Calibration | prioritised targets are acquired slightly more often than predicted | {{13.3_acquired_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_prioritised_ratio_95_pct_high|.3f}}] | 13 |
| Inheritance | per roll, years with an inspiration of a target read slightly below the model | {{3.per_roll_without_tail_held_high_spark_years_obs_per_exp|.3f}} | 3, 13 |
| Event choices | five outcome shares the model leaves at the equal 0.5 read off it: Tazuna Hayakawa [Tracen Reception]'s finale, Agnes Tachyon [Q≠0] link 1, Marvelous Sunday [Dazzling Day in the Snow] link 3, Light Hello [From the Ground Up]'s finale, Gold Ship [That Time I Became the Strongest] link 3 | {{9.tazuna_finale_concentration|.2f}}, {{9.tachyon_link_1_choice_2_murmur|.2f}}, {{9.marvelous_sunday_link_3_position_pilfer|.2f}}, {{9.light_hello_finale_share|.2f}}, {{9.gold_ship_link_3_innate_experience|.2f}} | 9 |
| Left out | targets a deck card's random event can hint | {{10.calibration_obs_per_pred_left_out|.2f}} [{{10.calibration_ratio_95_pct_low_left_out|.2f}}, {{10.calibration_ratio_95_pct_high_left_out|.2f}}] | 10 |
| Left out | Vodka's own Nimble Navigator | her events give it in {{11.vodka_any_outfit_nimble_navigator_from_her_events|.0f}} of {{11.vodka_any_outfit_nimble_navigator_rows|.0f}} runs | 11 |

### Not settled

- Whether specialty priority causes more hints. Fitted on earlier runs, it predicts later ones better by
  {{7.specialty_held_out_gain_nats|.1f}} nats, but on cards whose limit breaks change only their specialty priority, no
  effect fits as well (section 7).
- The hint-frequency slope $1/100$ (**assumed**). On the adopted model a free slope (divisor
  {{7.specialty_model_free_divisor_c|.0f}}) predicts unseen runs marginally better, $z$ =
  {{7.held_out_specialty_model_free_divisor_vs_1_per_100_z|.1f}} at the 1.96 bound; without the specialty term one
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
  overstatement is {{2.q_overstatement_below_17_500_max|.2%}}, within the assumed mixes. It is no bound for every
  lineage and says nothing of joint landing odds.

### Evidence

*Which band?* A career's rank score is the run list's estimate with every skill point spent, over every completed run.

| Scenario | Runs | Share at 17,500+ [95%] | Band |
|---|---|---|---|
| URA Finale | {{2.ura_finale_runs|,.0f}} | {{2.ura_finale_share_at_17_500_or_more|.3f}} [{{2.ura_finale_share_95_pct_low|.3f}}, {{2.ura_finale_share_95_pct_high|.3f}}] | below |
| Unity Cup | {{2.unity_cup_runs|,.0f}} | {{2.unity_cup_share_at_17_500_or_more|.3f}} [{{2.unity_cup_share_95_pct_low|.3f}}, {{2.unity_cup_share_95_pct_high|.3f}}] | below |
| Grand Concert | {{2.grand_concert_runs|,.0f}} | {{2.grand_concert_share_at_17_500_or_more|.3f}} [{{2.grand_concert_share_95_pct_low|.3f}}, {{2.grand_concert_share_95_pct_high|.3f}}] | 17,500+ |
| Trackblazer | {{2.trackblazer_runs|,.0f}} | {{2.trackblazer_share_at_17_500_or_more|.3f}} [{{2.trackblazer_share_95_pct_low|.3f}}, {{2.trackblazer_share_95_pct_high|.3f}}] | below |

Grand Masters has no runs and is **assumed** 17,500+.

*What does averaging over stars cost?* For a planned loop the model gives each legacy her mix's expected base $\bar b$.
A legacy keeps her stars in both rolls. So her chance of firing in neither is $E[(1 - r_b)^2]$ over her base $b$, where
$r_b = \min\{1,\ b(1 + A/100)\}$ is her chance per roll at affinity $A$ (section 3). The model uses $(1 - r_{\bar b})^2$
instead. That miss is convex in $b$, cap included, so $E[(1 - r_b)^2] \ge (1 - r_{\bar b})^2$. Averaging therefore
understates the miss and overstates $q$, the chance of at least one inspiration of the skill.

The table measures the overstatement for a target all six legacies carry, at the six real affinities of each of the
{{2.lineages|,.0f}} lineages in the runs. It treats each legacy's stars as independent of the others'; a veteran
appearing twice in a lineage would share hers.

| Band | Mean overstatement of $q$ | Largest |
|---|---|---|
| below 17,500 | {{2.q_overstatement_below_17_500_mean|.2%}} | {{2.q_overstatement_below_17_500_max|.2%}} |
| 17,500+ | {{2.q_overstatement_17_500_and_up_mean|.2%}} | {{2.q_overstatement_17_500_and_up_max|.2%}} |

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
spark fires. There are {{3.trials_parents|,.0f}} parent trials and {{3.trials_grandparents|,.0f}} grandparent trials of ordinary ancestors; a trainee's own
character as a grandparent is section 5's.

### Findings

- **Grandparents pass sparks at the full base rate, as parents do.**
- **Affinity scales the roll as the formula says.** Held-out gains for a free slope on affinity or either part
  are too uncertain to establish an improvement over unit weights.
- **Each star count's base matches its trials below affinity 200.** On all trials {{3.star_counts_outside_their_own_95_pct_interval_on_every_trial|.0f}} star count reads below its prediction, through its trials at 200 or more: the parents' known miss below.
- **Known miss: parents at affinity 200 or more pass sparks less often than predicted.** The bands below 200 are
  jointly consistent. The held-out gain from a refitted slope is inconclusive. The model keeps the formula and this
  miss.
- **Whole careers bear out $q$ apart from Tail Held High**, whose miss and the totals are in section 13. One band of
  predicted $q$ lies outside its pointwise interval; the joint test does not reject the model.

### Evidence

"Held out" is rule 3 of *How to read*: five folds by run, positive nats favouring the refit. Each part's weight is
fitted with the other part at 1. The carrier and spark-year checks exclude Tail Held High, leaving
{{3.inheritance_without_tail_held_high_rows|,.0f}} rows. The predicted-$q$ band check uses all
{{13.2_inheritance_rows|,.0f}} rows.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Grandparents take the full base | weight {{3.grandparent_weight_fitted|.2f}} [{{3.grandparent_weight_95_pct_low|.2f}}, {{3.grandparent_weight_95_pct_high|.2f}}] | full base beats half by {{3.grandparents_full_over_halved_nats|,.0f}} nats | full base |
| Affinity raises the roll | | beats no affinity by {{3.affinity_over_none_nats|,.0f}} nats | kept |
| Affinity slope is 1 | {{3.affinity_slope_fitted|.2f}} [{{3.affinity_slope_95_pct_low|.2f}}, {{3.affinity_slope_95_pct_high|.2f}}] | held out {{3.affinity_slope_refitted_over_slope_1_held_out_nats|.1f}} ± {{3.affinity_slope_refitted_over_slope_1_held_out_se|.1f}} nats, $z$ = {{3.affinity_slope_refitted_over_slope_1_held_out_z|.2f}} | 1 kept |
| Relation-point weight is 1 | {{3.relation_part_slope_fitted|.2f}} [{{3.relation_part_slope_95_pct_low|.2f}}, {{3.relation_part_slope_95_pct_high|.2f}}] | held out {{3.relation_part_slope_refitted_over_slope_1_held_out_nats|.1f}} ± {{3.relation_part_slope_refitted_over_slope_1_held_out_se|.1f}} nats, $z$ = {{3.relation_part_slope_refitted_over_slope_1_held_out_z|.2f}} | 1 kept |
| Weight of 3 per shared G1 win is 1 | {{3.race_part_slope_fitted|.2f}} [{{3.race_part_slope_95_pct_low|.2f}}, {{3.race_part_slope_95_pct_high|.2f}}] | held out {{3.race_part_slope_refitted_over_slope_1_held_out_nats|.1f}} ± {{3.race_part_slope_refitted_over_slope_1_held_out_se|.1f}} nats, $z$ = {{3.race_part_slope_refitted_over_slope_1_held_out_z|.2f}} | 1 kept |
| Affinity bands below 200 fit | | $\chi^2$ = {{3.affinity_bands_below_200_chi2|.1f}}, $p$ = {{3.affinity_bands_below_200_chi2_p|.2f}} over four bands | consistent |
| All five affinity bands fit | | $\chi^2$ = {{3.affinity_bands_chi2|.1f}}, $p$ = {{3.affinity_bands_chi2_p|.3f}} | known miss at 200+ |
| Careers with 1 to 6 carriers | {{3.by_carriers_1_6_without_tail_held_high_lowest_ratio|.3f}}-{{3.by_carriers_1_6_without_tail_held_high_highest_ratio|.3f}} of the model | every interval covers 1 | consistent |
| Years with an inspiration of the skill | {{3.per_roll_without_tail_held_high_spark_years_obs_per_exp|.3f}} of the model | {{3.per_roll_without_tail_held_high_spark_years_sds_below_expectation|.2f}} sd below | known miss, under 2% short; cause not established |
| $q$ by predicted band | | $\chi^2$ = {{3.reliability_chi2|.1f}}, $p$ = {{3.reliability_chi2_p|.2f}} over the {{3.reliability_chi2_buckets|.0f}} bands above 0 | consistent |

*By stars and by affinity.* Of these groups, affinity {{3.affinity_bands_outside_their_own_95_pct_interval|s}} lies outside its own interval, and
every trial there is a parent's; a star count that misses on all trials does so through those trials, and below
affinity 200 every star count covers 1.

| Group | Trials | Observed / predicted [95%] |
|---|---|---|
| 1★ | {{3.1_star_trials|,.0f}} | {{3.1_star_obs_per_pred|.3f}} [{{3.1_star_95_pct_low|.2f}}, {{3.1_star_95_pct_high|.2f}}] |
| 2★ | {{3.2_star_trials|,.0f}} | {{3.2_star_obs_per_pred|.3f}} [{{3.2_star_95_pct_low|.2f}}, {{3.2_star_95_pct_high|.2f}}] |
| 3★ | {{3.3_star_trials|,.0f}} | {{3.3_star_obs_per_pred|.3f}} [{{3.3_star_95_pct_low|.2f}}, {{3.3_star_95_pct_high|.2f}}] |
| 1★, affinity below 200 | {{3.1_star_below_200_trials|,.0f}} | {{3.1_star_below_200_obs_per_pred|.3f}} [{{3.1_star_below_200_95_pct_low|.2f}}, {{3.1_star_below_200_95_pct_high|.2f}}] |
| 2★, affinity below 200 | {{3.2_star_below_200_trials|,.0f}} | {{3.2_star_below_200_obs_per_pred|.3f}} [{{3.2_star_below_200_95_pct_low|.2f}}, {{3.2_star_below_200_95_pct_high|.2f}}] |
| 3★, affinity below 200 | {{3.3_star_below_200_trials|,.0f}} | {{3.3_star_below_200_obs_per_pred|.3f}} [{{3.3_star_below_200_95_pct_low|.2f}}, {{3.3_star_below_200_95_pct_high|.2f}}] |
| affinity 0-49 | {{3.affinity_0_49_trials|,.0f}} | {{3.affinity_0_49_obs_per_pred|.3f}} [{{3.affinity_0_49_95_pct_low|.3f}}, {{3.affinity_0_49_95_pct_high|.3f}}] |
| affinity 50-99 | {{3.affinity_50_99_trials|,.0f}} | {{3.affinity_50_99_obs_per_pred|.3f}} [{{3.affinity_50_99_95_pct_low|.3f}}, {{3.affinity_50_99_95_pct_high|.3f}}] |
| affinity 100-149 | {{3.affinity_100_149_trials|,.0f}} | {{3.affinity_100_149_obs_per_pred|.3f}} [{{3.affinity_100_149_95_pct_low|.3f}}, {{3.affinity_100_149_95_pct_high|.3f}}] |
| affinity 150-199 | {{3.affinity_150_199_trials|,.0f}} | {{3.affinity_150_199_obs_per_pred|.3f}} [{{3.affinity_150_199_95_pct_low|.3f}}, {{3.affinity_150_199_95_pct_high|.3f}}] |
| affinity 200+ | {{3.affinity_200_trials|,.0f}} | {{3.affinity_200_obs_per_pred|.3f}} [{{3.affinity_200_95_pct_low|.3f}}, {{3.affinity_200_95_pct_high|.3f}}] |

*Whole careers by predicted $q$.* Only the band {{3.reliability_buckets_outside_their_own_95_pct_interval|s}} lies outside its own 95% interval.

| Predicted $q$ | Rows | Observed | Mean predicted |
|---|---|---|---|
| 0 | {{3.reliability_q_0_rows|,.0f}} | {{3.reliability_q_0_observed|.3f}} | 0 |
| above 0 to 0.20 | {{3.reliability_q_0_00_0_20_rows|,.0f}} | {{3.reliability_q_0_00_0_20_observed|.3f}} | {{3.reliability_q_0_00_0_20_mean_predicted|.2f}} |
| 0.20-0.40 | {{3.reliability_q_0_20_0_40_rows|,.0f}} | {{3.reliability_q_0_20_0_40_observed|.3f}} | {{3.reliability_q_0_20_0_40_mean_predicted|.2f}} |
| 0.40-0.60 | {{3.reliability_q_0_40_0_60_rows|,.0f}} | {{3.reliability_q_0_40_0_60_observed|.3f}} [{{3.reliability_q_0_40_0_60_observed_95_pct_low|.3f}}, {{3.reliability_q_0_40_0_60_observed_95_pct_high|.3f}}] | {{3.reliability_q_0_40_0_60_mean_predicted_three_places|.3f}} |
| 0.60-0.80 | {{3.reliability_q_0_60_0_80_rows|,.0f}} | {{3.reliability_q_0_60_0_80_observed|.3f}} | {{3.reliability_q_0_60_0_80_mean_predicted|.2f}} |
| 0.80-0.95 | {{3.reliability_q_0_80_0_95_rows|,.0f}} | {{3.reliability_q_0_80_0_95_observed|.3f}} | {{3.reliability_q_0_80_0_95_mean_predicted|.2f}} |

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
- **Known misses.** Races with aptitude basis {{4.low_aptitude_basis_bucket|s}} (before streak penalties) and chosen races 1st or 2nd in a row win slightly less than predicted.
  Goal races 7th or later in a row win less than the planner predicts, since it charges them as the 6th.

IT's chosen races won at these rates, with the community table's chance in parentheses. Grades include those raised by
inherited aptitude sparks; S is shown as A, since both cost 0. A dot marks a cell with under 30 attempts. Only grade
pairs with 200 or more attempts are shown.

{{4.race_grid_by_aptitude_and_streak|s}}

### Evidence

The runs record {{4.race_attempts_at_the_34|,.0f}} attempts at the 34 single-race G1s, {{4.race_goal_attempts|,.0f}} of them goal races.

The table calls some races certain, 0% or 100%, and a handful went the other way, so its log-likelihood would be
$-\infty$. Every model compared therefore has its 0% and 100% pulled in to a fitted floor and ceiling. With the same
two parameters in each, the in-sample comparisons follow rule 1 of *How to read*.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Raised grades, not base | observed / predicted: base {{4.race_obs_per_pred_base|.3f}} [{{4.race_obs_per_pred_base_95_pct_low|.3f}}, {{4.race_obs_per_pred_base_95_pct_high|.3f}}], raised {{4.race_obs_per_pred_pink|.3f}} [{{4.race_obs_per_pred_pink_95_pct_low|.3f}}, {{4.race_obs_per_pred_pink_95_pct_high|.3f}}] | raised fits {{4.race_ll_recalibrated_pink_table_recalibrated_base_table_nats|,.0f}} nats better | raised grades |
| A pink counts from the April of its inspiration | won {{4.proc_timing_observed|.3f}} [{{4.proc_timing_observed_95_pct_low|.3f}}, {{4.proc_timing_observed_95_pct_high|.3f}}] of {{4.proc_timing_attempts|,.0f}} Senior January-March attempts where the timing matters | predicted {{4.proc_timing_predicted_from_april|.3f}} from April, {{4.proc_timing_predicted_from_the_year_start|.3f}} from the year's start | from April |
| The streak penalty is real | | without it, {{4.race_ll_recalibrated_pink_table_recalibrated_pink_no_penalty_nats|,.0f}} nats worse | kept |
| A loss changes the next race's chance | after a loss over after a win, 4th race on: {{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred_95_pct_high|.3f}}] | run-clustered; at the 2nd and 3rd the interval also covers 1 | not detected |
| Goal races pay one position less | community table, goal races 4th on: {{4.race_community_table_goal_j_ge_4_obs_per_pred|.3f}} [{{4.race_community_table_goal_j_ge_4_obs_per_pred_95_pct_low|.3f}}, {{4.race_community_table_goal_j_ge_4_obs_per_pred_95_pct_high|.3f}}] | held out, forms table below | shift adopted |
| One formula is enough | | a free rate per chance, streak position and race kind, {{4.race_pink_cells_bp_x_j_x_goal|.0f}} cells, held out: {{4.held_out_cells_vs_table_nats|.0f}} ± {{4.held_out_cells_vs_table_se|.0f}} nats, $z$ = {{4.held_out_cells_vs_table_z|.2f}} | table kept |
| Aptitude basis {{4.low_aptitude_basis_bucket|s}} wins less (grouped before streak penalties) | {{4.race_pink_bp_lt_70_obs_per_pred|.3f}} [{{4.race_pink_bp_lt_70_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_lt_70_obs_per_pred_95_pct_high|.3f}}] over {{4.race_pink_bp_lt_70_attempts|,.0f}} attempts | interval below 1 | known miss |

By scenario, every interval covers the prediction once chosen races 1st and 2nd in a row are left out.

*Goal races.* Under the community table, goal races from the 4th race in a row win more than predicted. Three forms
were scored on runs left out of the fit, five folds by run over {{4.held_out_runs|,.0f}} runs, each form with its own
fitted floor and ceiling:

| Form | Goal race at position $j$ pays | Races after it count it | Held out, against the adopted form |
|---|---|---|---|
| Adopted | $\mathrm{PEN}_{\max(j-1,1)}$ | yes | |
| Community table | $\mathrm{PEN}_j$ | yes | {{4.held_out_goal_shift_vs_community_table_nats|,.0f}} ± {{4.held_out_goal_shift_vs_community_table_se|.0f}} nats worse, $z$ = {{4.held_out_goal_shift_vs_community_table_z|.2f}} |
| Exclusion | $\mathrm{PEN}$ at the chosen races before it | no | {{4.held_out_goal_shift_vs_exclusion_nats|,.0f}} ± {{4.held_out_goal_shift_vs_exclusion_se|.0f}} nats worse, $z$ = {{4.held_out_goal_shift_vs_exclusion_z|.2f}} |

With the shift, goal races from the 1st to the 6th in a row and chosen races from the 3rd on are calibrated: each row
covers the prediction or, on the largest rows ({{4.race_pink_cells_outside_their_interval|s}}), misses it by under 1%,
a fifth of the table's smallest step. Goal races 4th to 6th in a row win {{4.race_pink_goal_j_4_6_obs_per_pred|.3f}}
[{{4.race_pink_goal_j_4_6_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_4_6_obs_per_pred_95_pct_high|.3f}}] of
the prediction, against {{4.race_community_table_goal_j_4_6_obs_per_pred|.3f}} under the community table. Chosen races
1st and 2nd in a row fall slightly short, a known miss. Goal races 7th or later fall well short: the other known miss.

| Race in a row | Goal races: observed / predicted [95%] | Attempts | Chosen races: observed / predicted [95%] | Attempts |
|---|---|---|---|---|
| 1st | {{4.race_pink_goal_j_1_obs_per_pred|.3f}} [{{4.race_pink_goal_j_1_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_1_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_1_attempts|,.0f}} | {{4.race_pink_chosen_j_1_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_1_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_1_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_1_attempts|,.0f}} |
| 2nd | {{4.race_pink_goal_j_2_obs_per_pred|.3f}} [{{4.race_pink_goal_j_2_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_2_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_2_attempts|,.0f}} | {{4.race_pink_chosen_j_2_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_2_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_2_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_2_attempts|,.0f}} |
| 3rd | {{4.race_pink_goal_j_3_obs_per_pred|.3f}} [{{4.race_pink_goal_j_3_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_3_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_3_attempts|,.0f}} | {{4.race_pink_chosen_j_3_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_3_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_3_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_3_attempts|,.0f}} |
| 4th | {{4.race_pink_goal_j_4_obs_per_pred|.3f}} [{{4.race_pink_goal_j_4_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_4_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_4_attempts|,.0f}} | {{4.race_pink_chosen_j_4_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_4_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_4_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_4_attempts|,.0f}} |
| 5th | {{4.race_pink_goal_j_5_obs_per_pred|.3f}} [{{4.race_pink_goal_j_5_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_5_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_5_attempts|,.0f}} | {{4.race_pink_chosen_j_5_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_5_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_5_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_5_attempts|,.0f}} |
| 6th (goal), 6th or later (chosen) | {{4.race_pink_goal_j_6_only_obs_per_pred|.3f}} [{{4.race_pink_goal_j_6_only_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_6_only_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_6_only_attempts|,.0f}} | {{4.race_pink_chosen_j_6_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_6_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_6_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_6_attempts|,.0f}} |
| goal, 7th or later | {{4.race_pink_goal_j_ge_7_obs_per_pred|.2f}} [{{4.race_pink_goal_j_ge_7_obs_per_pred_95_pct_low|.2f}}, {{4.race_pink_goal_j_ge_7_obs_per_pred_95_pct_high|.2f}}] | {{4.race_pink_goal_j_ge_7_attempts|,.0f}} | | |

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
  tool does, fits {{5.ll_0_ll_pair_nats|.0f}} nats worse.

### Evidence

The fit uses {{5.trials_attributable_to_one_ancestor|,.0f}} trials as section 3 defines them, self-grandparents included. In {{5.self_grandparent_passes_observed|,.0f}} of them a
self-grandparent passed her spark. The distance is observed minus predicted passes, in run-clustered standard
deviations.

| Claim | Estimate | Test | Verdict |
|---|---|---|---|
| $\mathrm{tri}(T,P,T) = 0$ | {{5.predicted_tri_t_p_t_0|.1f}} passes predicted | distance {{5.self_grandparent_passes_z_with_0|u.2f}} | kept |
| $\mathrm{tri}(T,P,T) = \mathrm{pair}(T,P)$ | {{5.predicted_tri_t_p_t_pair_t_p|.1f}} passes predicted | distance {{5.self_grandparent_passes_z_with_pair|u.2f}} | rejected |
| a fraction $\kappa$ of $\mathrm{pair}(T,P)$ | $\hat\kappa = {{5.kappa_fitted|.0f}}$, its lower bound | | 0 kept |

The gap is not the ordinary trials' own misfit: refitting the affinity slope on them, to {{5.affinity_slope_refitted_on_the_ordinary_trials|.2f}}, leaves both verdicts
unchanged.

## 6. Loop structures, ranking and schedules

Computed by `research/s06_loops.py`. The structures, their lineages $(T, P_1, P_2, X)$ and their ranking by mean
$P(\text{all})$ are defined in `docs/MODEL.md` and `docs/SOLVER.md`. $X$ is the uma each parent was bred from with the
other parent; a rotation has $X = T$.

### Findings

- **The rotation occurs in practice**: {{6.exact_rotation_lineages|,.0f}} runs have its exact lineage shape.
  More broadly, {{6.a_grandparent_of_her_own_character|,.0f}} of {{6.runs|,.0f}} runs have a grandparent of the trainee's own character.
- **Every pair of umas carries the same weight in a structure's summed affinities**, so the schedules maximise the total
  expected shared wins. That total is a surrogate: equal weights do not make it equivalent to the nonlinear landing
  odds.
- **With each loop's reported decks fixed, scheduling for shared wins costs little**: on the reference results, no
  schedule could raise a loop's odds by more than {{6.schedules_pb_ceiling_gain_max_pct|.2f}}%.
- **Known miss: the pair ceilings do not certify every listed top.** On {{6.schedules_result_files_with_such_a_runner_up|.0f}} results a listed runner-up's ceiling passes its winner's odds, by up to {{6.schedules_runner_up_pb_ceiling_over_its_winner_max_pct|.1f}}%; it would need {{6.schedules_such_a_runner_up_s_gain_needed_to_tie_min_pct|.2f}}-{{6.schedules_such_a_runner_up_s_gain_needed_to_tie_max_pct|.2f}}% over its schedule found to tie, which the search did not find and the bound cannot exclude.

### Evidence

*Race weights* (**derived** from section 5's $A_P$ and $A_G$). In one lineage, $o(P_1,P_2)$ enters the six affinities
{{6.one_lineage_o_p1_p2_weight|.0f}} times: twice in each $A_P$, since $P'$ is also one of $P$'s own parents. $o(P_1,X)$ and $o(P_2,X)$ enter
{{6.one_lineage_o_p1_x_weight|.0f}} times each. Summed over a loop's {{6.four_uma_loop_lineages|.0f}} lineages or a rotation's {{6.three_uma_rotation_lineages|.0f}}, every pair gets one weight:

$$\text{loop: } {{6.one_lineage_o_p1_p2_weight|.0f}}\cdot 2 + {{6.one_lineage_o_p1_x_weight|.0f}}\cdot 4 = {{6.loop_weight_per_pair|.0f}}, \qquad \text{rotation: } {{6.one_lineage_o_p1_p2_weight|.0f}} + {{6.one_lineage_o_p1_x_weight|.0f}} + {{6.one_lineage_o_p1_x_weight|.0f}} = {{6.rotation_weight_per_pair|.0f}}.$$

*The schedule bound* (**derived**). The odds are nondecreasing in every $o$: $o$ enters each affinity with weight +3,
and every later step is nondecreasing in its inputs. So, with a loop's decks fixed, no schedule can score above the
odds with every pair's overlap at an upper bound. The bound used is the solver's pair ceiling (`docs/SOLVER.md`,
`pair_ceilings`), with outfits reselected and decks fixed. Each pair's ceiling holds separately, and the pairs need not
reach theirs jointly, so the bound is conservative. The ceilings come from the search code that chose the schedules.
The script checks only that each lies between the overlap found and a loose independent ceiling: every G1 offering at
its best chance, with no streak penalty.

The bound is computed on the {{6.schedules_result_files|.0f}} reference results, the best loops and decks found for the validation setups
(`val/ref__*`, `val/refpu__*`). An independent implementation reproduces their odds to within {{6.schedules_recomputed_odds_max_relative_error|.0e}}.

| On the reference results | Value |
|---|---|
| winners' odds gained by the shared wins found, over none | {{6.schedules_shared_win_value_min_pct|.0f}}-{{6.schedules_shared_win_value_max_pct|.0f}}% |
| most any other schedule could add to a winner, by the pair ceilings | {{6.schedules_pb_ceiling_gain_min_pct|.2f}}-{{6.schedules_pb_ceiling_gain_max_pct|.2f}}%, median {{6.schedules_pb_ceiling_gain_median_pct|.2f}}% |
| listed runner-ups whose pair-ceiling odds reach their winner | {{6.schedules_runner_ups_whose_pb_ceiling_reaches_the_winner|.0f}} of {{6.schedules_runner_up_loops_checked|.0f}} |

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
  {{7.held_out_specialty_model_free_divisor_vs_1_per_100_z|.1f}}, at the 1.96 bound), a known miss.
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
level 5 and leaves the pool after $c = \lceil 5 / \iota \rceil$ hints. The runs bear out the increment: {{7.increment_multiples_of_iota|,.0f}} of
{{7.increment_single_source_levels|,.0f}} levels that one card gave alone are multiples of $\iota$, or 5.

The chance that none of $k$ targets in a pool of $n$ skills is hinted is computed jointly, not target by target:

$$M(k, n, h) = (1 - f)\, m(k, n, \lfloor h \rfloor) + f\, m(k, n, \lfloor h \rfloor + 1), \qquad
m(k, n, t) = \left(1 - \frac{k}{n}\right)^{t} \ (t \le c).$$

Each of $t$ hints misses all $k$ targets with chance $1 - k/n$. When $t > c$, an exact Markov chain over the other
skills' hint counts replaces $m$.

### Fitted values

**Fitted** by maximum likelihood on {{7.specialty_rows|,.0f}} rows from {{7.specialty_card_runs|,.0f}} card-runs. A row is one pool skill of one card in
one run; a card-run is one card in one run's deck. The fit takes SR and SSR cards at maximum limit break and leaves out
rows where an inspiration hinted the skill. The Runs column counts the runs that add a row. The solver reads these
values from `research/results.json` when it is built (`pkg/loopopt/build.rs`), rounded to three decimals.

| Scenario | $\beta$ | $d$ | Rows | Runs | Typical card's hints per career |
|---|---|---|---|---|---|
| Unity Cup | {{7.specialty_base_unity_cup|.3f}} | {{7.specialty_d_unity_cup|.3f}} | {{7.specialty_rows_unity_cup|,.0f}} | {{7.specialty_runs_unity_cup|.0f}} | {{7.example_hints_per_career_median_card_unity_cup|.1f}} |
| Grand Concert | {{7.specialty_base_grand_concert|.3f}} | {{7.specialty_d_grand_concert|.3f}} | {{7.specialty_rows_grand_concert|,.0f}} | {{7.specialty_runs_grand_concert|,.0f}} | {{7.example_hints_per_career_median_card_grand_concert|.1f}} |
| URA Finale | {{7.specialty_base_ura_finale|.3f}} | {{7.specialty_d_ura_finale|.3f}} | {{7.specialty_rows_ura_finale|,.0f}} | {{7.specialty_runs_ura_finale|.0f}} | {{7.example_hints_per_career_median_card_ura_finale|.1f}} |
| Trackblazer | {{7.specialty_base_trackblazer|.3f}} | {{7.specialty_d_trackblazer|.3f}} | {{7.specialty_rows_trackblazer|,.0f}} | {{7.specialty_runs_trackblazer|.0f}} | {{7.example_hints_per_career_median_card_trackblazer|.1f}} |
| Grand Masters | {{7.one_base_and_one_d_base|.3f}} | {{7.one_base_and_one_d_d|.3f}} | no runs: **assumed**, one $\beta$ and one $d$ fitted over every scenario's rows | | |

The typical card is the runs' median: hint frequency +{{7.example_median_hint_frequency|.0f}}% and specialty priority {{7.example_median_specialty_priority|.0f}}.

### Evidence

*The form.* The form was chosen before the specialty term was added, on a wider sample: every card at any limit break,
R cards included, {{7.form_rows|,.0f}} rows from {{7.form_runs|,.0f}} runs. That study reads the count as continuous and ignores the level-5 cap. On
runs left out of the fit, per-scenario $\beta(1 + hf/100)$ predicts better than either simpler form:

| Alternative | Per-scenario $\beta(1 + hf/100)$ better held out by |
|---|---|
| one constant count, no $hf$ | {{7.held_out_per_scenario_bases_vs_constant_nats|,.1f}} ± {{7.held_out_per_scenario_bases_vs_constant_se|.1f}} nats, $z$ = {{7.held_out_per_scenario_bases_vs_constant_z|.1f}} |
| one $\beta(1 + hf/100)$ for every scenario | {{7.held_out_per_scenario_bases_vs_one_base_nats|.1f}} ± {{7.held_out_per_scenario_bases_vs_one_base_se|.1f}} nats, $z$ = {{7.held_out_per_scenario_bases_vs_one_base_z|.1f}} |

*Hint frequency's slope.* The $1/100$ is **assumed**: a card's +40% is read as 40% more hints. A free divisor tests it,
with and without the specialty term:

| Model | Free divisor | Held out, against $1/100$ | Verdict |
|---|---|---|---|
| adopted, with the specialty term | {{7.specialty_model_free_divisor_c|.0f}} | {{7.held_out_specialty_model_free_divisor_vs_1_per_100_nats|.1f}} ± {{7.held_out_specialty_model_free_divisor_vs_1_per_100_se|.1f}} nats, $z$ = {{7.held_out_specialty_model_free_divisor_vs_1_per_100_z|.1f}} | free divisor marginally better (known miss); $1/100$ kept, assumed |
| form study, without it | {{7.free_divisor_c|.0f}} | {{7.held_out_free_divisor_vs_1_per_100_nats|.1f}} ± {{7.held_out_free_divisor_vs_1_per_100_se|.1f}} nats, $z$ = {{7.held_out_free_divisor_vs_1_per_100_z|.1f}} | free divisor better |

*Specialty priority.* Without the specialty term, cards with more specialty priority give more hints than predicted.
With it, the excess is gone:

| Specialty priority | Rows | Observed / predicted, without the term | With it |
|---|---|---|---|
| 0 | {{7.specialty_band_0_rows|,.0f}} | {{7.specialty_band_0_before|.3f}} | {{7.specialty_band_0_after|.3f}} |
| 1-29 | {{7.specialty_band_1_29_rows|,.0f}} | {{7.specialty_band_1_29_before|.3f}} | {{7.specialty_band_1_29_after|.3f}} |
| 30-59 | {{7.specialty_band_30_59_rows|,.0f}} | {{7.specialty_band_30_59_before|.3f}} | {{7.specialty_band_30_59_after|.3f}} |
| 60+ | {{7.specialty_band_60_rows|,.0f}} | {{7.specialty_band_60_before|.3f}} | {{7.specialty_band_60_after|.3f}} |

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| The term predicts unseen careers | | held out {{7.held_out_specialty_term_vs_none_nats|.1f}} ± {{7.held_out_specialty_term_vs_none_se|.1f}} nats, $z$ = {{7.held_out_specialty_term_vs_none_z|.1f}} | holds |
| Fitted on the earlier half of the runs, it predicts the later half | | {{7.held_out_later_runs_specialty_term_vs_none_nats|.1f}} ± {{7.held_out_later_runs_specialty_term_vs_none_se|.1f}} nats, $z$ = {{7.held_out_later_runs_specialty_term_vs_none_z|.1f}} | holds |
| A weight per scenario beats one shared weight | | held out {{7.held_out_per_scenario_d_vs_one_d_nats|.1f}} ± {{7.held_out_per_scenario_d_vs_one_d_se|.1f}} nats, $z$ = {{7.held_out_per_scenario_d_vs_one_d_z|.1f}} | per scenario |
| With the term, the scenario × specialty cells fit | {{7.specialty_cells_df|.0f}} cells of 200+ rows, {{7.specialty_cells_fitted_parameters|.0f}} parameters fitted on them | run-clustered $\chi^2$ = {{7.specialty_cells_joint_chi2|.1f}}: $p$ = {{7.specialty_cells_p_less_the_fitted_parameters|.2f}} on the cells less the fitted parameters (conservative), {{7.specialty_cells_p|.2f}} on the cells (liberal) | not shown |
| The same card, only specialty priority changing | $d$ = {{7.specialty_same_card_clean_d|u.2f}} [{{7.specialty_same_card_clean_d_95_pct_low|u.2f}}, {{7.specialty_same_card_clean_d_95_pct_high|.2f}}], {{7.specialty_same_card_clean_cards|.0f}} cards | {{7.specialty_same_card_clean_gain_nats|+.2f}} nats over no term | cause undecided |

The last row compares each card with itself. On these cards, from one limit break to the next, specialty priority changes and hint
frequency does not; a base is fitted per card and scenario. The estimate is consistent with no effect and with every
per-scenario weight. A scale on the per-scenario weights, refitted on either half of the runs, on SR or SSR cards alone
and without each of the eight most-used cards, stays within {{7.specialty_scale_k_robustness_min|.2f}}-{{7.specialty_scale_k_robustness_max|.2f}}.

*The level-5 cap.* Removing a skill at level 5 fits {{7.cap_gain_nats|.1f}} nats better than drawing with replacement, with no
extra parameter. It matters most where one hint reaches level 5: with replacement those rows read {{7.cap_table_1_draws_replacement|.3f}} of prediction,
with the cap {{7.cap_table_1_draws_cap|.3f}}.

*The draws' shape.* Three checks bear on the shape of the draws, and none runs the adopted model. The repeat and pair
checks use the form study's rate, without the specialty term or the level-5 cap, and the pair check reads the count as
continuous. The stat-option check assumes a fixed count. They test the shape, not the fitted rate.

| Question | Observed | Predicted |
|---|---|---|
| A skill a card hinted at all, hinted again: $P(k \ge 2 \mid k \ge 1)$ | {{7.repeat_rate_observed|.3f}} | {{7.repeat_rate_two_point_count|.3f}} with the two-point count, {{7.repeat_rate_poisson_count|.3f}} with a Poisson count |
| Two skills of one pool both hinted, over {{7.joint_pairs|,.0f}} pairs, Trackblazer out | {{7.p_both_observed|.4f}} | {{7.p_both_joint|.4f}} by joint draws, {{7.p_both_independent|.4f}} independently |

That a draw never picks the card's stat option, the one non-skill option beside its pool, is **assumed**. In {{7.cells_var_below_stat_option_floor|.0f}} of
{{7.cells_card_hf|.0f}} (card, $hf$) cells the count of draws varies less than a drawn stat option would allow. The check keeps only
card-runs whose every count is exact, with no skill at level 5 or hinted by another source, and uses a sample
variance biased low. It is consistent with the assumption and does
not establish it.

*Covariates and cards.* Card type, initial bond and pool size add no nominally detectable gain to the adopted rate.
Three other terms, tested on {{7.covariate_rows|,.0f}} rows under the limits *How to read* sets out:

| Added term | In-sample gain | Held out | Verdict |
|---|---|---|---|
| rarity, SR against SSR | +{{7.covariate_gain_sr_nats|.1f}} nats | {{7.held_out_covariate_sr_vs_none_nats|.1f}} ± {{7.held_out_covariate_sr_vs_none_se|.1f}}, $z$ = {{7.held_out_covariate_sr_vs_none_z|.1f}} | not predictive, ignored; factor {{7.covariate_factor_sr|.3f}} |
| pals per deck | +{{7.covariate_gain_pals_per_deck_nats|.1f}} nats | {{7.held_out_covariate_pals_per_deck_vs_none_nats|.1f}} ± {{7.held_out_covariate_pals_per_deck_vs_none_se|.1f}}, $z$ = {{7.held_out_covariate_pals_per_deck_vs_none_z|.1f}} | known miss; factor {{7.covariate_factor_pals_per_deck|.3f}} per pal |
| a factor for each of {{7.per_card_cards|.0f}} cards with 200+ rows | | {{7.held_out_a_factor_per_card_vs_none_nats|.1f}} ± {{7.held_out_a_factor_per_card_vs_none_se|.1f}}, $z$ = {{7.held_out_a_factor_per_card_vs_none_z|.1f}} | known miss; cards read {{7.per_card_obs_per_pred_min|.2f}}-{{7.per_card_obs_per_pred_max|.2f}} |

*Other sources of the same skill.* The fit leaves out rows where an inspiration also hinted the skill. Against the
adopted rate, the card's own hint is seen:

| Other source of the skill | Rows | Observed / predicted |
|---|---|---|
| none | {{7.spark_none_rows|,.0f}} | {{7.spark_none_obs_per_pred|.3f}} [{{7.spark_none_obs_per_pred_95_pct_low|.3f}}, {{7.spark_none_obs_per_pred_95_pct_high|.3f}}] |
| inspiration | {{7.spark_inspiration_rows|,.0f}} | {{7.spark_inspiration_ratio|.2f}} |
| card event | {{7.spark_card_event_rows|,.0f}} | {{7.spark_card_event_ratio|.2f}} |
| other card's training | {{7.spark_other_card_training_rows|,.0f}} | {{7.spark_other_card_training_ratio|.2f}} |

With no other source the adopted rate is calibrated to within 2%: it reads {{7.spark_none_obs_per_pred|.3f}}, an
excess just detectable on {{7.spark_none_rows|,.0f}} rows. A card's own event leaves it calibrated. A skill another
card's training also hinted reads {{7.spark_other_card_training_ratio|.2f}} of the model, a known miss smaller than the
inspiration's.

Removing both inspired skills for the whole run predicts a larger redistribution than observed. This comparison
does not exclude partial or time-dependent removal. The inspiration shortfall remains unmodelled. On card-runs where an inspiration hinted two pool skills, training hinted the rest at {{7.drop_out_test_observed|.3f}}. With the
two kept in the pool the model predicts {{7.drop_out_test_no_drop_out|.3f}}; with them removed, {{7.drop_out_test_drop_out|.3f}}.

*Prioritised skills.* The model assumes a priority list does not change training hints. On the {{7.priority_runs|,.0f}} runs that
show a list, {{7.priority_rows|.0f}} rows hold a prioritised pool skill. Training hinted them at {{7.priority_observed|.3f}}, against {{7.priority_predicted|.3f}}
predicted: no sign of a rise, from a weak test.

## 8. Chain events

Computed by `research/s08_chains.py`.

### Findings

- **The deck's pal cards raise the number of chain events** (**fitted**). $N \sim \mathrm{Binomial}(21, p)$ with
  $\operatorname{logit} p = a + b \times \text{pals}$, where group cards do not count as pals. $p$ runs from
  {{8.pal_count_budget_p_at_0_pals|.3f}} with no pals to {{8.pal_count_budget_p_at_5_pals|.3f}} with five.
- **Each chain event goes to a running chain card, all equally likely** (**fitted**). With the pal-count budget this card
  rule predicts unseen runs better than weighting cards by their links left, the rule the model used before.
- **A chain that ends early frees its events for the other chain cards** (**supported**, conditional on the rest of
  the model). Every one of {{8.chain_ends_synthetic_pools_per_hypothesis|.0f}} synthetic pools generated with an ended
  card still taking its picks scores below the real runs; the real runs fall among the pools generated with ends
  freeing their events. No randomised test (the same deck with and without the ending prioritised) has been run.
- **No scenario effect and no Sasami Anshinzawa [This Might Sting!] effect is detectable on top of the budget**: the
  extra events of decks with her are accounted for by the pals they carry. Not detecting an effect does not show there
  is none; the tests' resolution is in the evidence below.
- **Known misses**: Inari One's careers reach fewer events than the budget predicts; early links run short at 3 pals and
  long at 1; chains that never start with five chain cards are slightly over-predicted. Causes unknown.

### Model

A run gets $N \sim \mathrm{Binomial}(21, p)$ chain events, $\operatorname{logit} p = a + b \times \text{pals}$ (**fitted**:
$a$ = {{8.pal_count_budget_a|+.3f}}, $b$ = {{8.pal_count_budget_b|+.3f}}). Pals are the deck's pal-flagged cards
other than group cards. Each event advances one chain card by one link, the card picked uniformly among the chain cards
still running. A chain is 3 links on an SSR and 2 on an SR, and a stopped or finished chain takes no more events. Pal
and group cards take no events.

A chain can stop early. If IT's default choice at a link ends the chain on some of its outcomes, the chain stops there
on a share {{8.stop_share|.3f}} of that link's firings, the same for every
such link (**fitted**). A chain-ending choice the player prioritises stops it with the chance section 9's three-case
rule gives, read from each run's own priority list. A run showing no list takes {{8.no_list_scale|.2f}} times the list-showing runs' share (**fitted** on a grid). The model enumerates the exact
distribution of final chain states (section 12).

| Pals | 0 | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| $p$ | {{8.pal_count_budget_p_at_0_pals|.3f}} | {{8.pal_count_budget_p_at_1_pals|.3f}} | {{8.pal_count_budget_p_at_2_pals|.3f}} | {{8.pal_count_budget_p_at_3_pals|.3f}} | {{8.pal_count_budget_p_at_4_pals|.3f}} | {{8.pal_count_budget_p_at_5_pals|.3f}} |
| mean $N$ | {{8.pal_count_budget_mean_n_at_0_pals|.2f}} | {{8.pal_count_budget_mean_n_at_1_pals|.2f}} | {{8.pal_count_budget_mean_n_at_2_pals|.2f}} | {{8.pal_count_budget_mean_n_at_3_pals|.2f}} | {{8.pal_count_budget_mean_n_at_4_pals|.2f}} | {{8.pal_count_budget_mean_n_at_5_pals|.2f}} |

### Evidence

*Reading a chain's progress.* Progress is read from marker skills, on {{8.fit_of_n_runs|,.0f}} runs. A link is
observable only if every choice and outcome of its event carries a skill that no other event of the card gives. A link
is left out when another deck card's events can also give its marker, or when the marker can come from inspiration.
A marker can also appear without the link firing, from training hints and other sources: its background rate $\beta$ is
measured per skill and scenario on runs whose decks cannot give it, shrunk toward the trainee's own rate. A link is
then seen with chance $1 - \varepsilon(1 - \beta)$ if it fired and $\beta$ if it did not. Each fit is exact over the
chain states the run's deck can pass through.

*The marker allowance* $\varepsilon$ (**chosen**, inside its measured interval). The fits let a fired link's marker go
unseen with chance {{8.allowance_the_fits_value|.3f}}.
Links fire in order, so a later link seen on a clean marker ($\beta$ small) proves the earlier ones fired:

| Earlier links implied fired by a later one | Showing no marker | Share [95%] |
|---|---|---|
| {{8.allowance_earlier_links_implied_fired|,.0f}} | {{8.allowance_marker_not_shown|.0f}} | {{8.allowance_share|.3f}} [{{8.allowance_95_pct_low|.3f}}, {{8.allowance_95_pct_high|.3f}}] |

With none missing the upper limit counts each run as one trial. At $n = 21$ the best single $p$ moves from
{{8.allowance_sensitivity_lowest_best_p_at_n_21|.3f}} to {{8.allowance_sensitivity_highest_best_p_at_n_21|.3f}}
over allowances 0 to 0.02.

*Which model predicts unseen runs?* Each form is fitted on four of five groups of runs and scored on the fifth; the
per-run held-out log-likelihoods are compared pairwise (*How to read*). Today's model is the one this section
replaces: $N$ fixed at Binomial(21, 0.35), events weighted by links left; its stop risks, end chances and backgrounds
are the same fitted ones as every other form's. The free distribution fits $P(N = 0..17)$ and $P(N \ge 18)$ (no deck has
more than 18 links). All forms but today's use the card rule unless named:

| Form | Over | Held-out nats ± SE | $z$ |
|---|---|---|---|
| pal-count budget | today's model | {{8.held_out_pal_count_budget_over_today_s_model_nats|u+.1f}} ± {{8.held_out_pal_count_budget_over_today_s_model_se|.1f}} | {{8.held_out_pal_count_budget_over_today_s_model_z|u+.2f}} |
| pal-count budget | the free distribution | {{8.held_out_pal_count_budget_over_free_nats|u+.1f}} ± {{8.held_out_pal_count_budget_over_free_se|.1f}} | {{8.held_out_pal_count_budget_over_free_z|u+.2f}} |
| pal-count budget | the same with links-left weighting | {{8.held_out_pal_count_budget_over_the_same_with_the_slot_rule_nats|u+.1f}} ± {{8.held_out_pal_count_budget_over_the_same_with_the_slot_rule_se|.1f}} | {{8.held_out_pal_count_budget_over_the_same_with_the_slot_rule_z|u+.2f}} |
| pal-count budget | the same with an ended card still taking its picks | {{8.held_out_pal_count_budget_over_the_same_with_ends_wasted_nats|u+.1f}} ± {{8.held_out_pal_count_budget_over_the_same_with_ends_wasted_se|.1f}} | {{8.held_out_pal_count_budget_over_the_same_with_ends_wasted_z|u+.2f}} |
| pal-count budget, every nuisance (backgrounds, steering and no-list shares) fitted on the training folds | today's model | {{8.nested_held_out_pal_count_budget_over_today_s_model_nats|u+.1f}} ± {{8.nested_held_out_pal_count_budget_over_today_s_model_se|.1f}} | {{8.nested_held_out_pal_count_budget_over_today_s_model_z|u+.2f}} |
| the same | the same with an ended card still taking its picks | {{8.nested_held_out_pal_count_budget_over_the_same_with_ends_wasted_nats|u+.1f}} ± {{8.nested_held_out_pal_count_budget_over_the_same_with_ends_wasted_se|.1f}} | {{8.nested_held_out_pal_count_budget_over_the_same_with_ends_wasted_z|u+.2f}} |
| free distribution | today's model | {{8.held_out_free_over_today_s_model_nats|u+.1f}} ± {{8.held_out_free_over_today_s_model_se|.1f}} | {{8.held_out_free_over_today_s_model_z|u+.2f}} |
| binomial, $n$ and $p$ fitted | today's model | {{8.held_out_binomial_by_likelihood_over_today_s_model_nats|u+.1f}} ± {{8.held_out_binomial_by_likelihood_over_today_s_model_se|.1f}} | {{8.held_out_binomial_by_likelihood_over_today_s_model_z|u+.2f}} |

In-sample the budget's $b$ has likelihood-ratio $p$ = {{8.pal_count_budget_lr_p_against_one_p_at_n_21|.1e}} against
one $p$. With $N$ fixed at Binomial(21, 0.35), links-left weighting scores {{8.allocation_slot_over_card_nats|.0f}}
nats above the card rule (a composite score, each link scored on its own); with the budget the order reverses (the
table's third row).

*Freed or wasted?* When a chain ends early (a prioritised chain-ending choice, or a default-choice stop), either its
card leaves the running cards and later events go to the others (freed, the model), or it keeps taking its share of
picks until its full length and shows none (wasted). Each hypothesis gets its own stop share and no-list scale (wasted:
{{8.chain_ends_wasted_stop_share|.3f}} and {{8.chain_ends_wasted_no_list_scale|.2f}}). A direct likelihood ratio
has no ready reference distribution here, so the test is a parametric bootstrap: synthetic pools of the same runs are
generated under each hypothesis, at its own constants and budget, and scored with the budget re-fitted.
The real runs' freed - wasted log-likelihood is {{8.chain_ends_freed_wasted_log_likelihood_real_runs|+.1f}} nats;
{{8.chain_ends_synthetic_pools_if_wasted_scoring_below_the_real_runs|.0f}} of the wasted pools (mean
{{8.chain_ends_freed_wasted_synthetic_pools_if_wasted_mean|+.1f}}) and
{{8.chain_ends_synthetic_pools_if_freed_scoring_below_the_real_runs|.0f}} of the freed pools (mean
{{8.chain_ends_freed_wasted_synthetic_pools_if_freed_mean|+.1f}}) score below it. Limits: each hypothesis keeps its
real-data stop share and no-list scale in every pool instead of re-fitting them, so the spreads leave out that
selection; with this many pools only the ranks are informative, not tail probabilities; the backgrounds and choice
shares are fitted on the whole pool; and the result holds only as far as the budget law, the card rule, the
backgrounds and the choice rule do.

*The form of $N$.* Among one-count forms, the free fit's $\min(N, 18)$ is under-dispersed (variance / mean
{{8.distribution_free_variance_per_mean|.2f}}; Poisson(7.1) is {{8.poisson_7_1_nats_behind_distribution_free|.0f}} nats
behind), and the binomial beats Conway-Maxwell-Poisson by {{8.best_binomial_n_free_over_conway_maxwell_poisson_nats|.1f}}
nats. Binomial(21, 0.35) is {{8.binomial_21_0_35_nats_behind_distribution_free|.0f}} nats behind the free fit
in-sample. The best single binomial has $n$ = {{8.best_binomial_n_free_n|.0f}} (profile set
{{8.binomial_n_95_pct_profile_interval_low|.0f}}-{{8.binomial_n_95_pct_profile_interval_high|.0f}}); $n$ and $p$ trade
off along a ridge, so the budget keeps $n = 21$ and lets $p$ carry the deck.

*Groups.* Each group's own logit term on top of the budget:

| Group | Runs | Own term | Likelihood-ratio $p$ | Mean pal count, group / other runs |
|---|---|---|---|---|
| decks with Sasami Anshinzawa [This Might Sting!] | {{8.sasami_anshinzawa_s_card_runs|,.0f}} | {{8.sasami_anshinzawa_s_card_own_term_on_the_pal_count_budget|+.3f}} | {{8.sasami_anshinzawa_s_card_own_term_on_the_pal_count_budget_lr_p|.2f}} | {{8.sasami_anshinzawa_s_card_mean_pal_count|.2f}} / {{8.sasami_anshinzawa_s_card_mean_pal_count_the_other_runs|.2f}} |
| Inari One's careers | {{8.inari_one_runs|,.0f}} | {{8.inari_one_own_term_on_the_pal_count_budget|+.3f}} | {{8.inari_one_own_term_on_the_pal_count_budget_lr_p|.3f}} | {{8.inari_one_mean_pal_count|.2f}} / {{8.inari_one_mean_pal_count_the_other_runs|.2f}} |

Without the budget, Sasami's decks fit $p$ = {{8.sasami_anshinzawa_s_card_p_at_n_21|.3f}} against
{{8.sasami_anshinzawa_s_card_p_at_n_21_the_other_runs|.3f}}; with it, her own term is not distinguishable from zero.

*Scenarios.* No scenario effect is detectable: an intercept per scenario on the budget gains {{8.per_scenario_on_the_pal_count_budget_log_likelihood_gain|.1f}}
nats for {{8.per_scenario_on_the_pal_count_budget_extra_parameters|.0f}} parameters. A free distribution per scenario
gains {{8.per_scenario_log_likelihood_gain|.0f}} nats for {{8.per_scenario_extra_parameters|.0f}}, because the
scenarios' pal mixes differ.

*Stop share and no-list scale.* Each is profiled with the budget re-fitted at every grid value. The stop share
{{8.stop_share|.3f}} is the best grid value and the only one within 1.92 nats (0.4 is
{{8.stop_share_profile_0_4_nats_below_the_grid_s_best|.1f}} behind, 0.25 {{8.stop_share_profile_0_25_nats_below_the_grid_s_best|.0f}}).
The no-list scale {{8.no_list_scale|.2f}} is {{8.no_list_scale_profile_the_model_s_value_nats_below_the_grid_s_best|.1f}} nats below the grid's best; its 1.92-nat range is {{8.no_list_scale_profile_lowest_grid_value_within_1_92_nats|.2f}}-{{8.no_list_scale_profile_highest_grid_value_within_1_92_nats|.2f}};
1, the list-showing share itself, is {{8.no_list_scale_profile_1_nats_below_the_grid_s_best|.0f}} nats behind.

*Final links by pal count.* The final link's marker against each model's chance of reaching it, with the background and
allowance applied:

| Pals | Chain cards | Runs | Observed [95%] | Final model | Today's model |
|---|---|---|---|---|---|
| 5 | 1 | {{8.final_link_table_5_pals_runs|,.0f}} | {{8.final_link_table_5_pals_observed|.3f}} [{{8.final_link_table_5_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_5_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_5_pals_pal_count_budget|.3f}} | {{8.final_link_table_5_pals_binomial_21_0_35|.3f}} |
| 4 | 2 | {{8.final_link_table_4_pals_runs|,.0f}} | {{8.final_link_table_4_pals_observed|.3f}} [{{8.final_link_table_4_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_4_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_4_pals_pal_count_budget|.3f}} | {{8.final_link_table_4_pals_binomial_21_0_35|.3f}} |
| 3 | 3 | {{8.final_link_table_3_pals_runs|,.0f}} | {{8.final_link_table_3_pals_observed|.3f}} [{{8.final_link_table_3_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_3_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_3_pals_pal_count_budget|.3f}} | {{8.final_link_table_3_pals_binomial_21_0_35|.3f}} |
| 2 | 4 | {{8.final_link_table_2_pals_runs|,.0f}} | {{8.final_link_table_2_pals_observed|.3f}} [{{8.final_link_table_2_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_2_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_2_pals_pal_count_budget|.3f}} | {{8.final_link_table_2_pals_binomial_21_0_35|.3f}} |
| 1 | 5 | {{8.final_link_table_1_pals_runs|,.0f}} | {{8.final_link_table_1_pals_observed|.3f}} [{{8.final_link_table_1_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_1_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_1_pals_pal_count_budget|.3f}} | {{8.final_link_table_1_pals_binomial_21_0_35|.3f}} |
| 0 | 6 | {{8.final_link_table_0_pals_runs|,.0f}} | {{8.final_link_table_0_pals_observed|.3f}} [{{8.final_link_table_0_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_0_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_0_pals_pal_count_budget|.3f}} | {{8.final_link_table_0_pals_binomial_21_0_35|.3f}} |

The final model lies outside the interval at {{8.final_link_table_pal_counts_with_the_final_model_outside_the_interval|.0f}} pal counts; today's model at
{{8.final_link_table_pal_counts_with_today_s_model_outside_the_interval|.0f}}.

*Known miss: early links.* An early link precedes the final link. The shortfall is the share predicted to fire minus
the share seen; each run's links form one cluster, $z = \sum(\text{pred} - \text{obs}) / \sqrt{\sum(\text{pred} - \text{obs})^2}$
over runs:

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Final model: shortfall | {{8.early_link_shortfall_under_the_budget_4_pals|+.3f}} | {{8.early_link_shortfall_under_the_budget_3_pals|+.3f}} | {{8.early_link_shortfall_under_the_budget_2_pals|+.3f}} | {{8.early_link_shortfall_under_the_budget_1_pals|+.3f}} | {{8.early_link_shortfall_under_the_budget_0_pals|+.3f}} |
| $z$ | {{8.early_link_shortfall_under_the_budget_4_pals_z|+.1f}} | {{8.early_link_shortfall_under_the_budget_3_pals_z|+.1f}} | {{8.early_link_shortfall_under_the_budget_2_pals_z|+.1f}} | {{8.early_link_shortfall_under_the_budget_1_pals_z|+.1f}} | {{8.early_link_shortfall_under_the_budget_0_pals_z|+.1f}} |
| Today's model: $z$ | {{8.early_link_shortfall_4_pals_z|+.1f}} | {{8.early_link_shortfall_3_pals_z|+.1f}} | {{8.early_link_shortfall_2_pals_z|+.1f}} | {{8.early_link_shortfall_1_pals_z|+.1f}} | {{8.early_link_shortfall_0_pals_z|+.1f}} |

*Chains never started.* With five or six chain cards (zero or one pal), link 1 not reached:

| Chain cards | Observed [95%] | Final model | Today's model |
|---|---|---|---|
| 5 | {{8.never_started_5_chain_cards_observed|.2f}} [{{8.never_started_5_chain_cards_observed_95_pct_low|.2f}}, {{8.never_started_5_chain_cards_observed_95_pct_high|.2f}}] | {{8.never_started_5_chain_cards_final_model|.2f}} | {{8.never_started_5_chain_cards_model|.2f}} |
| 6 | {{8.never_started_6_chain_cards_observed|.2f}} [{{8.never_started_6_chain_cards_observed_95_pct_low|.2f}}, {{8.never_started_6_chain_cards_observed_95_pct_high|.2f}}] | {{8.never_started_6_chain_cards_final_model|.2f}} | {{8.never_started_6_chain_cards_model|.2f}} |

*Individual cards.* The {{8.final_links_by_card_cards|.0f}} cards with 60 or more final-link observations, against the
final model, clustered by run since one run's chain cards share its $N$: pooled {{8.final_links_by_card_pooled_observed|.3f}}
observed against {{8.final_links_by_card_pal_count_budget_pooled_model|.3f}}, $\chi^2$ =
{{8.final_links_by_card_pal_count_budget_chi2|.0f}} (nominal $p$ = {{8.final_links_by_card_pal_count_budget_wald_p|.2f}}),
no card beyond $|z|$ = {{8.final_links_by_card_pal_count_budget_largest_z|.1f}}. Against today's model the same test gives
$\chi^2$ = {{8.final_links_by_card_chi2|.0f}}.

## 9. Event choices and outcomes

Computed by `research/s09_outcomes.py` (the event-only rows and the certainty checks: `research/s13_calibration.py`).

Two rules are **assumed** (`docs/MODEL.md`). The *choice rule*: IT takes an event's top choice unless a prioritised
skill sits on another choice, and the player prioritises every target. The *equal rule*: a choice's outcomes are equally
likely unless measured, since master.mdb holds no outcome odds.

A target counts as *prioritised* when it is on the run's priority list and not outranked. It is outranked when a
higher-ranked prioritised skill sits on a rival choice of every event offering it. That affects {{9.event_only_hints_outranked_on_a_rival_choice_rows|.0f}} of section 13's
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
| one-sided: only that choice holds a prioritised skill | {{9.steer_one_sided_taken|.0f}} of {{9.steer_one_sided_runs|.0f}} | {{9.steer_one_sided_share|.2f}} [{{9.steer_one_sided_90_pct_low|.2f}}, {{9.steer_one_sided_90_pct_high|.2f}}] |
| maxed: the skill already at hint Lv 5 from another source | {{9.steer_maxed_taken|.0f}} of {{9.steer_maxed_runs|.0f}} | {{9.steer_maxed_share|.2f}} [{{9.steer_maxed_90_pct_low|.2f}}, {{9.steer_maxed_90_pct_high|.2f}}] |
| both: the default choice holds a prioritised skill too | {{9.steer_both_taken|.0f}} of {{9.steer_both_runs|.0f}} | {{9.steer_both_share|.2f}} [{{9.steer_both_90_pct_low|.2f}}, {{9.steer_both_90_pct_high|.2f}}] |

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
| Sasami Anshinzawa [This Might Sting!] finale, Risky Business prioritised: Nothing Ventured | {{9.anshinzawa_nv_with_rb_prioritised|.0f}}/{{9.anshinzawa_rb_prioritised_runs|.0f}} | {{9.anshinzawa_nv_share|.2f}} [{{9.anshinzawa_nv_share_90_pct_low|.2f}}, {{9.anshinzawa_nv_share_90_pct_high|.2f}}] | {{9.adopted_anshinzawa_finale_nothing_ventured|.2f}} |
| Riko Kashimoto [Planned Perfection] "Unexpected Side", top choice: Ramp Up, not Maverick ○ | {{9.riko_ramp_up_top_choice_taken|.0f}}/{{9.riko_readings|.0f}} | {{9.riko_share|.2f}} [{{9.riko_90_pct_low|.2f}}, {{9.riko_90_pct_high|.2f}}] | {{9.adopted_riko_unexpected_side_ramp_up|.2f}} |
| Seiun Sky [Foolproof Plan] link 3: Fast-Paced, not Escape Artist | {{9.seiun_sky_link_3_fast_paced|.0f}}/{{9.seiun_sky_link_3_readings|.0f}} | {{9.seiun_sky_share|.2f}} [{{9.seiun_sky_90_pct_low|.2f}}, {{9.seiun_sky_90_pct_high|.2f}}] | {{9.adopted_seiun_sky_link_3_fast_paced|.2f}} |
| Smart Falcon [My Umadol Way! ☆] link 3: Center Stage, not Prudent Positioning | {{9.smart_falcon_link_3_center_stage|.0f}}/{{9.smart_falcon_link_3_readings|.0f}} | {{9.smart_falcon_share|.2f}} [{{9.smart_falcon_90_pct_low|.2f}}, {{9.smart_falcon_90_pct_high|.2f}}] | 0.5, the equal rule |
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante": Nothing Ventured, not Risky Business | {{9.festa_nv_wins|.0f}} observed wins / {{9.festa_readings|.1f}} imputed trials | {{9.festa_share|.2f}} [{{9.festa_share_90_pct_low|.2f}}, {{9.festa_share_90_pct_high|.2f}}] | {{9.festa_adopted|.2f}} |

A share counts only decks where no other card's events can give an outcome's marker, seen or not: where one can,
the record credits "A or B event" and cannot say which card fired. Smart Falcon's share is consistent with the equal
rule, which the model now uses for it (its earlier 0.65 rested on readings that dropped such shared credits for one
outcome only). Riko Kashimoto's top-choice row counts each credit as a firing; Maverick ○ also comes from other events
(credited to them in {{9.riko_maverick_background_runs_crediting_events|.0f}} of {{9.riko_maverick_background_runs|.0f}}
runs whose deck cannot give it), so counting a run that credits both as one Ramp Up firing gives
{{9.riko_top_choice_runs_with_ramp_up|.0f}} of {{9.riko_top_choice_runs|.0f}} instead. Whether her top choice departs
from the equal rule depends on that reading. Her adopted value is a per-run chance, set out under *Evidence*.
Nakayama Festa's share rests on an attribution model; its interval has unestablished coverage. Every count uses the
full pool.

### Evidence

*Does IT follow the choice rule?* Where the runs show the choice taken, yes:

| Event | Condition | Choice taken |
|---|---|---|
| Sasami Anshinzawa's finale | Risky Business prioritised | choice 2 in about {{9.anshinzawa_rb_prioritised_took_choice_2|.0%}} of runs |
| Sasami Anshinzawa's finale | neither skill prioritised | identifiable choice-two wins in {{9.anshinzawa_neither_prioritised_choice_2_won|.0f}} of {{9.anshinzawa_neither_prioritised_runs|.0f}} runs |
| her date-start event "A Pleasant Practice Partner?", read by its extra Risky Business +1 | Risky Business prioritised | bottom choice in {{9.anshinzawa_her_extra_rb_in_choice_2_wins_prioritised|.0f}} of {{9.anshinzawa_choice_2_wins_prioritised|.0f}} runs winning finale choice 2 |
| the same | Risky Business not prioritised | bottom choice in {{9.anshinzawa_her_extra_rb_in_choice_2_wins_rb_not_prioritised|.0f}} of {{9.anshinzawa_choice_2_wins_rb_not_prioritised|.0f}} such runs |
| Riko Kashimoto [Planned Perfection]'s event | Lone Wolf prioritised | bottom choice in {{9.riko_lone_wolf_prioritised_bottom_choice|.0f}} of {{9.riko_lone_wolf_prioritised_runs|.0f}} runs |

Of the {{9.anshinzawa_choice_1_exceptions_rb_prioritised|.0f}} finale runs that took choice 1 with Risky Business prioritised, {{9.anshinzawa_choice_1_exceptions_with_rb_at_lv_5|.0f}} had it already at hint Lv 5.
The records show a priority list for only {{9.runs_showing_a_priority_list|,.0f}} of {{9.runs|,.0f}} runs. The rest count as unknown, never as an empty
list, so the rule is tested mainly through its consequences in section 13.

*Reading Sasami Anshinzawa's finale.* Nakayama Festa [Trusting the Dice]'s "Upping the Ante" also gives Nothing
Ventured and shares its credits. So these rows use the {{9.anshinzawa_runs_with_the_card_no_festa|.0f}} runs with Anshinzawa's card and without Festa's.
Hint levels identify the choice:

- her Nothing Ventured +3 is choice 1, which otherwise gives nothing, and +1 is choice 2;
- a loss on choice 2 shows as her Risky Business at level 2, from storyline choice 3 and the finale.

GameTora marks the date-start event's bottom choice as ending her event chain, which for a pal card means her dates.
Yet with Risky Business prioritised the finale shows its outcome in at least {{9.anshinzawa_rb_prioritised_runs_showing_the_finale|.0f}} of {{9.anshinzawa_rb_prioritised_runs|.0f}} runs. IT
apparently keeps her dates either way.

*Riko Kashimoto's adopted share.* The adopted value is the per-run chance of her Ramp Up hint. A run counts where
no other deck card's events can give Ramp Up and its level from other sources stays below 5. On runs whose shown
list prioritises Ramp Up and not Lone Wolf, as the model assumes of a target, it is
{{9.riko_per_run_ramp_up_prioritised_share|.3f}} [{{9.riko_per_run_ramp_up_prioritised_90_pct_low|.3f}},
{{9.riko_per_run_ramp_up_prioritised_90_pct_high|.3f}}], which covers the adopted
{{9.adopted_riko_unexpected_side_ramp_up|.2f}}. A prioritised Lone Wolf takes her bottom choice
({{9.riko_per_run_lone_wolf_prioritised_share|.2f}}), so the pooled chance is lower:

| Riko Kashimoto [Planned Perfection] | Count | Share [90%] |
|---|---|---|
| runs with her Ramp Up hint | {{9.riko_per_run_runs_with_ramp_up|.0f}} of {{9.riko_per_run_runs|.0f}} | {{9.riko_per_run_share|.3f}} [{{9.riko_per_run_90_pct_low|.3f}}, {{9.riko_per_run_90_pct_high|.3f}}] |
| on the {{9.riko_firings_clean_decks|.0f}} runs whose deck has no other card that can give any of her three skills, firings where IT took the top choice; the rest took the bottom choice, Lone Wolf | {{9.riko_firings_top_choice|.0f}} of {{9.riko_firings_all|.0f}} | {{9.riko_firings_top_choice_rate|.2f}} |
| runs with her Ramp Up hint, list shown with Ramp Up prioritised and not Lone Wolf | {{9.riko_per_run_ramp_up_prioritised_runs_with_ramp_up|.0f}} of {{9.riko_per_run_ramp_up_prioritised_runs|.0f}} | {{9.riko_per_run_ramp_up_prioritised_share|.3f}} [{{9.riko_per_run_ramp_up_prioritised_90_pct_low|.3f}}, {{9.riko_per_run_ramp_up_prioritised_90_pct_high|.3f}}] |

*Nakayama Festa's share.* It starts from the {{9.festa_runs_with_the_card|.0f}} runs with her card, less her own careers and decks with
Sasami Anshinzawa's SSR. The credits there:

| Credit | Runs |
|---|---|
| to her alone, Nothing Ventured | {{9.festa_nv_wins|.0f}} |
| to her alone, Risky Business | {{9.festa_rb_wins|.0f}} |
| Risky Business, to "her or another card's event" | {{9.festa_rb_shared_credits|.0f}}, Team Sirius in {{9.festa_shared_with_team_sirius|.0f}} |
| neither outcome, with Risky Business already at Lv 5 | {{9.festa_blank_risky_business_at_lv_5|.0f}} |

A shared credit is split by estimated source rates. Let $r_F$ be her exclusive Risky Business credit rate in runs with
her card, {{9.festa_her_rb_rate|.3f}}. Let $r_o$ be the other card's exclusive rate in runs without her: {{9.festa_team_sirius_alone_rb_rate|.3f}} for Team Sirius,
{{9.festa_anshinzawa_alone_rb_rate|.3f}} for Sasami Anshinzawa. A shared credit goes to her with weight $r_F / (1 - (1 - r_F)(1 - r_o))$, an attribution weight obtained by substituting exclusive-credit rates into an independent-source formula.
It is not an identified conditional firing probability. That gives her {{9.festa_shared_credits_hers|.1f}} of them.

In a blank run either a Risky Business win could not show or the event never fired. The blank runs are split by the
rate at which the event shows where both outcomes can, {{9.festa_firing_rate_where_both_can_show|.2f}}. Whatever part of the shared credits is hers, the share
stays within {{9.festa_share_shared_all_hers|.3f}}-{{9.festa_share_shared_none_hers|.3f}}.

So the share rests on an attribution model. Its rates are exclusive credit rates, not firing rates, and it assumes the
sources fire independently. The blank-run split ignores which outcome is missing. The interval rounds the imputed denominator and treats it as a binomial trial count. It omits attribution
uncertainty, so its coverage is unestablished. No joint observation model is
fitted or tested.

*Shares left at the equal rule.*

| Event: outcome | Share | Verdict |
|---|---|---|
| Tazuna Hayakawa [Tracen Reception] finale: Concentration | {{9.tazuna_finale_concentration|.2f}} | off 0.5: known miss |
| Agnes Tachyon [Q≠0] link 1 choice 2: Murmur | {{9.tachyon_link_1_choice_2_murmur|.2f}} | off 0.5: known miss |
| Marvelous Sunday [Dazzling Day in the Snow] link 3: Position Pilfer | {{9.marvelous_sunday_link_3_position_pilfer|.2f}} | off 0.5: known miss |
| Light Hello [From the Ground Up] finale: See Ya Later! +3 | {{9.light_hello_finale_share|.2f}}: {{9.light_hello_finale_see_ya_later_3|,.0f}} of {{9.light_hello_finale_readings|,.0f}} | off 0.5: known miss |
| Gold Ship [That Time I Became the Strongest] link 3 | {{9.gold_ship_link_3_innate_experience|.2f}} | off 0.5: known miss |
| King Halo [Tonight, We Waltz] link 3 | {{9.king_halo_link_3_gap_closer|.2f}} | consistent with 0.5 |
| Daitaku Helios [Sunlit Angel ♪] link 2, where link 3 visibly fired: Tail Held High | {{9.helios_link_2_tail_held_high_given_link_3|.2f}} | consistent with 0.5 |

*Pal and group finales.* The checks use runs where the card is the deck's one source of the skill and no inspiration
hinted it. Light Hello [From the Ground Up]'s event is credited with See Ya Later! in {{9.light_hello_credited_credited|.0f}} of {{9.light_hello_credited_runs_carrying_her|.0f}} such runs.
Team Sirius is credited with Risky Business, alone or shared, in {{9.team_sirius_credited_credited|.0f}} of {{9.team_sirius_credited_runs_carrying_her|.0f}}. These observed rates are
consistent with a finale that always fires; finite samples do not establish it.

## 10. Random card events (not modelled)

Computed by `research/s10_random_events.py`.

A deck card's character has *shared random events* that can fire in another uma's career; any rarity counts, R cards
included. $K$ is the number on offer: every shared event of the deck's characters plus the trainee's own. A *reading*
is one such event in one run, read through the marker skill on its top choice.

### Findings

- **Leaving them out under-predicts the targets they can hint**, a known miss: on real careers those targets read
  {{10.calibration_obs_per_pred_left_out|.2f}} of the model. Credited at the fitted rate, they read {{10.calibration_obs_per_pred_credited|.2f}}.
- **Two default targets are affected**: Vodka's Nimble Navigator and Nice Nature's Ramp Up, the only ones that are a
  character's top-choice marker.
- **A shared random event fires in about {{10.constant_p|.2f}} of the careers it can fire in** (**fitted**), a lower bound. The
  rate falls as more events are on offer, and the $K$ form predicts unseen runs better than a constant
  ($z$ = {{10.held_out_fitted_form_against_constant_z|u+.2f}}).
- **Known miss of the measured rate: it differs by scenario**, highest in Unity Cup and Trackblazer, next to $K$ and
  next to the card effects. It changes no prediction, since the model leaves these events out.
- **Crediting them has no clear held-out advantage** on the rows the model gives a positive chance.
- **Known miss of the measured rate: the card's initial friendship gauge raises it.** It changes no prediction, since
  the model leaves these events out.

### Evidence

*The rate.* A hit is a top-choice marker credited to the character's event. An *ambiguous credit* also names another
deck card whose events can give the marker; as in section 8, it is left out as unknown. The {{10.readings|,.0f}} readings come
from {{10.runs|,.0f}} runs. They give {{10.clean_hits|,.0f}} hits, with {{10.ambiguous_credits_unknown_left_out|.0f}} ambiguous credits left out.

A firing goes unseen when IT takes another choice for a prioritised skill, or when the marker is already at Lv 5 from
other sources. So the rate is a lower bound. Counting ambiguous credits as hits, over the readings where the event
could show, gives {{10.rate_where_it_could_show_ambiguous_as_hits|.3f}}.

The *$K$ form* lowers the rate by a fixed fraction for each extra event on offer. The card effects and scenario are
fitted beside $K$ in a logistic model.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| The rate falls as more events are on offer | $K$ form: {{10.fitted_p_at_k_4|.2f}} at $K$ = 4, {{10.fitted_p_at_k_14|.2f}} at 14 | run-clustered Wald $\lvert z \rvert$ = {{10.slope_in_k_robust_z|u.2f}} | detectable |
| The $K$ form predicts unseen runs better than a constant | constant {{10.constant_p|.3f}} | held out {{10.held_out_fitted_form_against_constant_nats|u+.1f}} ± {{10.held_out_fitted_form_against_constant_se|.1f}} nats, $z$ = {{10.held_out_fitted_form_against_constant_z|u+.2f}} | yes |
| The initial friendship gauge raises the rate | {{10.initial_friendship_gauge_logit_per_10_points|+.3f}} [{{10.initial_friendship_gauge_95_pct_low|+.3f}}, {{10.initial_friendship_gauge_95_pct_high|+.3f}}] logit per 10 points | Holm-adjusted run-clustered $p$ = {{10.initial_friendship_gauge_holm_p|.4f}} | known miss |
| Friendship bonus moves it | {{10.friendship_bonus_logit_per_10_points|u+.3f}} logit per 10 points | Holm-adjusted $p$ = {{10.friendship_bonus_holm_p|.2f}} | no effect |
| Specialty priority moves it | {{10.specialty_priority_logit_per_10_points|u+.3f}} logit per 10 points | Holm-adjusted $p$ = {{10.specialty_priority_holm_p|.2f}} | no effect |
| Scenario moves it | Unity Cup {{10.scenario_unity_cup_logit_against_ura_finale|u+.2f}}, Trackblazer {{10.scenario_trackblazer_logit_against_ura_finale|u+.2f}} logit against URA Finale | run-clustered $p$ = {{10.scenario_robust_p|.3f}}; next to the card effects {{10.scenario_next_to_the_card_effects_robust_p|.3f}} | known miss |
| Rarity moves it within a character | | run-clustered $z$ = {{10.rarity_within_a_character_robust_z|u+.2f}} | no effect: R and SR cards pooled with SSR |

Pairs of one run's readings that both hit are as frequent as independent firing predicts. Independent firing remains
an assumption.

*What does leaving them out cost on real careers?* The test uses section 13's rows, each on the run's own deck. It
scores whether the target was acquired from any source, so it does not depend on reading the event. Both baseline
and credited acquisition use a hybrid reconstruction: exact landing marginal $p$ and the report's approximate gold
chance $g$. For an unguaranteed target, it reconstructs acquisition as $p/W-g$, with $W$ the white-spark generation
factor from section 12. The baseline therefore includes the difference between exact and approximate gold chances.
These comparisons are neither exact acquisition calibration nor a consistent test of the report approximation.
Crediting uses the
$K$ form at each run's $K$, about {{10.careers_mean_fitted_rate_there|.2f}} on these rows.

| Rows where a deck card's random event could hint the target | Count |
|---|---|
| all | {{10.calibration_rows|.0f}} |
| target credited to that event | {{10.careers_acquired_through_that_event|.0f}} |
| the same, with no other source | {{10.careers_acquired_through_that_event_only|.0f}} |
| rows the model gives no chance | {{10.careers_rows_the_model_gives_no_chance|.0f}} |
| of those, hinted by the event | {{10.careers_of_them_acquired_through_that_event|.0f}} |

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Left out, these targets are under-predicted | {{10.calibration_obs_per_pred_left_out|.2f}} [{{10.calibration_ratio_95_pct_low_left_out|.2f}}, {{10.calibration_ratio_95_pct_high_left_out|.2f}}] | interval excludes 1 | known miss |
| Credited at the fitted rate, they are not | {{10.calibration_obs_per_pred_credited|.2f}} [{{10.calibration_ratio_95_pct_low_credited|.2f}}, {{10.calibration_ratio_95_pct_high_credited|.2f}}] | interval covers 1 | consistent |
| Crediting predicts unseen runs better | | held out {{10.held_out_credited_against_left_out_nats|u+.1f}} ± {{10.held_out_credited_against_left_out_se|.1f}} nats, $z$ = {{10.held_out_credited_against_left_out_z|u+.2f}} | not shown |

The held-out score refits the rate on the other folds' readings. It covers only the {{10.calibration_rows_with_a_positive_prediction|.0f}} rows with a positive
baseline prediction: a row the model gives no chance has no finite likelihood when acquired. So it leaves out the rows
where an omitted source matters most, and it cannot settle the value of adding the source.

Crediting raises the product of positive per-target landing marginals by a median
{{10.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_median|.0%}}. The largest rise, over {{10.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_runs|.0f}} affected runs, is {{10.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_max|.0%}}. This uses the hybrid reconstruction above. It does not recompute exact all-target odds.

## 11. Conditional character events (not modelled)

Computed by `research/s11_conditional.py`.

The trainee's secret and race-result events are left out of the model, except Tokai Teio's Head-On. A
*conditional-only row* is a run-target row where only such an event can give the target: none of the trainee's story,
date, outfit or shared random events can, nor the scenario's.

### Findings

- **The sparse pooled conditional-only comparison is inconclusive.** Its score excludes zero-baseline rows,
  including {{11.careers_of_them_from_her_events|.0f}} acquisitions. The {{11.careers_conditional_only_rows|.0f}} rows
  do not settle trainee-specific costs.
- **Known miss: Vodka's own Nimble Navigator.** Her events give it in most of her runs, and the model under-predicts
  it. Her random and conditional events both give it and cannot be told apart.
- **Tokai Teio's Head-On is modelled as always acquired**, like Yamanin Zephyr's Slipstream. All three of its races
  are her career goals. The goals ask only for top 5, top 5 and top 3, while the event needs wins, so certainty is
  **assumed** from observed wins: IT won all three in {{11.of_them_winning_all_three|.0f}} of {{11.runs_training_tokai_teio|.0f}} runs training her. Every run where her
  event could show credits Head-On.

### Evidence

| Rows | Her events gave the target | Rate [95%] |
|---|---|---|
| all conditional-only rows | {{11.careers_conditional_only_from_her_events|.0f}} of {{11.careers_conditional_only_rows|.0f}} | {{11.careers_conditional_only_rate|.3f}} [{{11.careers_conditional_only_rate_95_pct_low|.3f}}, {{11.careers_conditional_only_rate_95_pct_high|.3f}}] |
| Daiwa Scarlet, any outfit, Nimble Navigator | {{11.daiwa_scarlet_any_outfit_nimble_navigator_from_her_events|.0f}} of {{11.daiwa_scarlet_any_outfit_nimble_navigator_rows|.0f}} | [{{11.daiwa_scarlet_any_outfit_nimble_navigator_rate_95_pct_low|.2f}}, {{11.daiwa_scarlet_any_outfit_nimble_navigator_rate_95_pct_high|.2f}}] |
| Fine Motion, any outfit, Ramp Up | {{11.fine_motion_any_outfit_ramp_up_from_her_events|.0f}} of {{11.fine_motion_any_outfit_ramp_up_rows|.0f}} | [{{11.fine_motion_any_outfit_ramp_up_rate_95_pct_low|.2f}}, {{11.fine_motion_any_outfit_ramp_up_rate_95_pct_high|.2f}}] |
| Mejiro Ardan, any outfit, Ramp Up | {{11.mejiro_ardan_any_outfit_ramp_up_from_her_events|.0f}} of {{11.mejiro_ardan_any_outfit_ramp_up_rows|.0f}} | [{{11.mejiro_ardan_any_outfit_ramp_up_rate_95_pct_low|.2f}}, {{11.mejiro_ardan_any_outfit_ramp_up_rate_95_pct_high|.2f}}] |
| Vodka, any outfit, Nimble Navigator, not conditional-only | {{11.vodka_any_outfit_nimble_navigator_from_her_events|.0f}} of {{11.vodka_any_outfit_nimble_navigator_rows|.0f}} | [{{11.vodka_any_outfit_nimble_navigator_rate_95_pct_low|.2f}}, {{11.vodka_any_outfit_nimble_navigator_rate_95_pct_high|.2f}}] |

| Rows, model as it is | Acquisition, observed / predicted [95%] | Crediting the measured rates gains |
|---|---|---|
| conditional-only | {{11.calibration_conditional_only_obs_per_pred_left_out|.2f}} [{{11.calibration_conditional_only_ratio_95_pct_low_left_out|.2f}}, {{11.calibration_conditional_only_ratio_95_pct_high_left_out|.2f}}] | {{11.calibration_conditional_only_ll_credited_minus_left_out_positive_rows|.1f}} nats |
| with Vodka's rows | {{11.calibration_obs_per_pred_left_out|.2f}} [{{11.calibration_ratio_95_pct_low_left_out|.2f}}, {{11.calibration_ratio_95_pct_high_left_out|.2f}}] | {{11.calibration_ll_credited_minus_left_out_positive_rows|.1f}} nats |

Both sides use section 10's hybrid acquisition reconstruction, mixing exact landing marginals with approximate
gold chances. Each gain is in-sample and is set against half the $\chi^2$ 95% point for one degree of freedom per fitted rate: the
first falls short, the second exceeds it. As in section 10, the gains count only rows with a positive baseline
prediction. That leaves out {{11.careers_conditional_only_rows_the_model_gives_no_chance|.0f}} conditional-only rows the model gives no chance, and her events
hinted the target on {{11.careers_of_them_from_her_events|.0f}} of them.

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
| {{13.runs_used|,.0f}} | {{13.ura_finale_runs|.0f}} | {{13.unity_cup_runs|,.0f}} | {{13.grand_concert_runs|,.0f}} | {{13.trackblazer_runs|.0f}} | {{13.run_target_rows|,.0f}} | {{13.rows_on_her_outfit|,.0f}} | {{13.scenario_gold_rows|,.0f}} |

Rows on her outfit and scenario golds are *certain*, so tests 1 and 3 leave them out; the rest are *open targets*.
Unity Cup's guaranteed golds show in {{13.unity_cup_s_guaranteed_golds_runs_with_both|,.0f}} of its {{13.unity_cup_runs|,.0f}} runs. The runs end before skills are bought, so
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
| 1. Deck hint | {{13.1_deck_hint_no_inspiration_rows|,.0f}} | {{13.1_deck_hint_no_inspiration_predicted|.3f}} | {{13.1_deck_hint_no_inspiration_obs_per_pred|.3f}} [{{13.1_deck_hint_no_inspiration_ratio_95_pct_low|.3f}}, {{13.1_deck_hint_no_inspiration_ratio_95_pct_high|.3f}}] | optimistic |
| training pool only | {{13.1_training_pool_only_rows|,.0f}} | {{13.1_training_pool_only_predicted|.3f}} | {{13.1_training_pool_only_obs_per_pred|.3f}} [{{13.1_training_pool_only_ratio_95_pct_low|.3f}}, {{13.1_training_pool_only_ratio_95_pct_high|.3f}}] | covers 1 |
| events only | {{13.1_events_only_rows|,.0f}} | {{13.1_events_only_predicted|.3f}} | {{13.1_events_only_obs_per_pred|.3f}} [{{13.1_events_only_ratio_95_pct_low|.3f}}, {{13.1_events_only_ratio_95_pct_high|.3f}}] | optimistic |
| events only, target prioritised | {{13.1_events_only_prioritised_rows|.0f}} | {{13.1_events_only_prioritised_predicted|.3f}} | {{13.1_events_only_prioritised_obs_per_pred|.3f}} [{{13.1_events_only_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_prioritised_ratio_95_pct_high|.4f}}] | covers 1 |
| events only, not prioritised | {{13.1_events_only_not_prioritised_rows|.0f}} | {{13.1_events_only_not_prioritised_predicted|.3f}} | {{13.1_events_only_not_prioritised_obs_per_pred|.3f}} [{{13.1_events_only_not_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_not_prioritised_ratio_95_pct_high|.3f}}] | far below: the priority assumption fails |
| 2. Inheritance | {{13.2_inheritance_rows|,.0f}} | {{13.2_inheritance_predicted|.3f}} | {{13.2_inheritance_obs_per_pred|.3f}} [{{13.2_inheritance_ratio_95_pct_low|.3f}}, {{13.2_inheritance_ratio_95_pct_high|.3f}}] | below 1 through Tail Held High |
| without Tail Held High | {{3.inheritance_without_tail_held_high_rows|,.0f}} | | {{3.inheritance_without_tail_held_high_obs_per_pred|.3f}} [{{3.inheritance_without_tail_held_high_ratio_95_pct_low|.3f}}, {{3.inheritance_without_tail_held_high_ratio_95_pct_high|.3f}}] | covers 1 |
| Tail Held High | | | {{13.tail_held_high_inheritance|.3f}} [{{13.tail_held_high_inheritance_95_pct_low|.3f}}, {{13.tail_held_high_inheritance_95_pct_high|.3f}}] | known miss |
| 3. Acquired, any source | {{13.3_acquired_any_source_rows|,.0f}} | {{13.3_acquired_any_source_predicted|.3f}} | {{13.3_acquired_any_source_obs_per_pred|.3f}} [{{13.3_acquired_any_source_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_ratio_95_pct_high|.3f}}] | slightly optimistic |
| prioritised or unknown | {{13.3_acquired_any_source_prioritised_or_no_list_shown_rows|,.0f}} | {{13.3_acquired_any_source_prioritised_or_no_list_shown_predicted|.3f}} | {{13.3_acquired_any_source_prioritised_or_no_list_shown_obs_per_pred|.3f}} [{{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_high|.3f}}] | covers 1 |
| prioritised | {{13.3_acquired_prioritised_rows|,.0f}} | {{13.3_acquired_prioritised_predicted|.3f}} | {{13.3_acquired_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_prioritised_ratio_95_pct_high|.3f}}] | slightly above 1: known miss |
| not prioritised | {{13.3_acquired_not_prioritised_rows|,.0f}} | {{13.3_acquired_not_prioritised_predicted|.3f}} | {{13.3_acquired_not_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_not_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_not_prioritised_ratio_95_pct_high|.3f}}] | optimistic |
| unknown priority | {{13.3_acquired_unknown_priority_rows|,.0f}} | {{13.3_acquired_unknown_priority_predicted|.3f}} | {{13.3_acquired_unknown_priority_obs_per_pred|.3f}} [{{13.3_acquired_unknown_priority_ratio_95_pct_low|.3f}}, {{13.3_acquired_unknown_priority_ratio_95_pct_high|.3f}}] | covers unity |

### Evidence

*Acquisition reliability.* The product approximation misses acquisitions in its zero-prediction bucket.
Positive bands also show variation hidden by the pooled ratio. These are checks of the acquisition approximation
on open targets, not the exact engine. Each positive band's observed rate is followed by its mean prediction.

| Predicted acquisition | Rows, observed (mean prediction) |
|---|---|
| 0 | {{13.reliability_acquired_0_rows|,.0f}}, {{13.reliability_acquired_0_observed|.3f}} |
| above 0 to 0.20 | {{13.reliability_acquired_0_00_0_20_rows|,.0f}}, {{13.reliability_acquired_0_00_0_20_observed|.3f}} ({{13.reliability_acquired_0_00_0_20_mean_predicted|.2f}}) |
| 0.20-0.40 | {{13.reliability_acquired_0_20_0_40_rows|,.0f}}, {{13.reliability_acquired_0_20_0_40_observed|.3f}} ({{13.reliability_acquired_0_20_0_40_mean_predicted|.2f}}) |
| 0.40-0.60 | {{13.reliability_acquired_0_40_0_60_rows|,.0f}}, {{13.reliability_acquired_0_40_0_60_observed|.3f}} ({{13.reliability_acquired_0_40_0_60_mean_predicted|.2f}}) |
| 0.60-0.80 | {{13.reliability_acquired_0_60_0_80_rows|,.0f}}, {{13.reliability_acquired_0_60_0_80_observed|.3f}} ({{13.reliability_acquired_0_60_0_80_mean_predicted|.2f}}) |
| 0.80-0.95 | {{13.reliability_acquired_0_80_0_95_rows|,.0f}}, {{13.reliability_acquired_0_80_0_95_observed|.3f}} ({{13.reliability_acquired_0_80_0_95_mean_predicted|.2f}}) |
| 0.95-1 | {{13.reliability_acquired_0_95_1_00_rows|,.0f}}, {{13.reliability_acquired_0_95_1_00_observed|.3f}} ({{13.reliability_acquired_0_95_1_00_mean_predicted|.2f}}) |

*Where the deck loses its points.* The deck's rows predicted 0.6 or more hold {{13.deck_shortfall_in_buckets_predicted_0_6_and_above|.0%}} of its shortfall,
and {{13.their_rows_with_an_event_source|.0%}} of those rows have an event source. On prioritised-or-unknown rows, the deck and inspiration alone
read {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_obs_per_pred|.3f}} of the model. Unmodelled scenario and trainee events close the gap to the any-source row: they gave
{{13.acquisitions_from_unmodelled_events|.0f}} target acquisitions, {{13.acquisitions_from_unmodelled_events_share_of_rows|.1%}} of open rows. Agreement after adding them can hide errors in the parts, and it
does not validate candidate rankings.

*Landing.* Each open row's spark chance is recomputed from what the run acquired: $0.4 \cdot 1.1^n$ if the gold was
hinted, $0.2 \cdot 1.1^n$ if only the white. Summed, it reads {{13.4_landing_ratio|.3f}} of the model's $p$, in line with acquisition. It
does not test spark generation (section 1).

*Per target*, observed / predicted:

| Target | Deck hint [95%] | Inheritance | Acquired |
|---|---|---|---|
| Uma Stan / Superstan | {{13.uma_stan_deck_hint|.2f}} [{{13.uma_stan_deck_hint_95_pct_low|.2f}}, {{13.uma_stan_deck_hint_95_pct_high|.2f}}] | {{13.uma_stan_inheritance|.3f}} | {{13.uma_stan_acquired|.3f}} |
| Nimble Navigator / No Stopping Me! | {{13.nimble_navigator_deck_hint|.2f}} [{{13.nimble_navigator_deck_hint_95_pct_low|.2f}}, {{13.nimble_navigator_deck_hint_95_pct_high|.2f}}] | {{13.nimble_navigator_inheritance|.3f}} | {{13.nimble_navigator_acquired|.3f}} |
| Playtime's Over! / See Ya Later! | {{13.playtime_s_over_deck_hint|.3f}} [{{13.playtime_s_over_deck_hint_95_pct_low|.2f}}, {{13.playtime_s_over_deck_hint_95_pct_high|.2f}}] | {{13.playtime_s_over_inheritance|.3f}} | {{13.playtime_s_over_acquired|.3f}} |
| Ramp Up / It's On! | {{13.ramp_up_deck_hint|.3f}} [{{13.ramp_up_deck_hint_95_pct_low|.2f}}, {{13.ramp_up_deck_hint_95_pct_high|.2f}}] | {{13.ramp_up_inheritance|.3f}} | {{13.ramp_up_acquired|.3f}} [{{13.ramp_up_acquired_95_pct_low|.2f}}, {{13.ramp_up_acquired_95_pct_high|.2f}}] |
| Tail Held High / Tail Nine | {{13.tail_held_high_deck_hint|.3f}} [{{13.tail_held_high_deck_hint_95_pct_low|.2f}}, {{13.tail_held_high_deck_hint_95_pct_high|.2f}}] | {{13.tail_held_high_inheritance|.3f}} [{{13.tail_held_high_inheritance_95_pct_low|.2f}}, {{13.tail_held_high_inheritance_95_pct_high|.2f}}] | {{13.tail_held_high_acquired|.3f}} |
| Racing Spirit: Stamina | no deck source | {{13.racing_spirit_stamina_inheritance|.3f}} | {{13.racing_spirit_stamina_acquired|.3f}} |
| Slipstream / Perfect Spot! | {{13.slipstream_deck_hint|.3f}} [{{13.slipstream_deck_hint_95_pct_low|.2f}}, {{13.slipstream_deck_hint_95_pct_high|.2f}}] | {{13.slipstream_inheritance|.3f}} | {{13.slipstream_acquired|.3f}} |
| Risky Business / Nothing Ventured | {{13.risky_business_deck_hint|.3f}} [{{13.risky_business_deck_hint_95_pct_low|.2f}}, {{13.risky_business_deck_hint_95_pct_high|.2f}}] | {{13.risky_business_inheritance|.3f}} | {{13.risky_business_acquired|.3f}} [{{13.risky_business_acquired_95_pct_low|.2f}}, {{13.risky_business_acquired_95_pct_high|.2f}}] |

*Gold acquisition*, observed / predicted:

| Gold | Rows | Observed / predicted [95%] |
|---|---|---|
| Nothing Ventured | {{13.gold_acquisition_nothing_ventured_rows|,.0f}} | {{13.gold_acquisition_nothing_ventured|.3f}} [{{13.gold_acquisition_nothing_ventured_95_pct_low|.2f}}, {{13.gold_acquisition_nothing_ventured_95_pct_high|.2f}}] |
| Perfect Spot! | {{13.gold_acquisition_perfect_spot_rows|,.0f}} | {{13.gold_acquisition_perfect_spot|.3f}} [{{13.gold_acquisition_perfect_spot_95_pct_low|.2f}}, {{13.gold_acquisition_perfect_spot_95_pct_high|.2f}}] |
| It's On!, outside Unity Cup | {{13.gold_acquisition_it_s_on_rows|,.0f}} | {{13.gold_acquisition_it_s_on|.3f}} [{{13.gold_acquisition_it_s_on_95_pct_low|.2f}}, {{13.gold_acquisition_it_s_on_95_pct_high|.2f}}] |

*Training-pool hints by scenario* test $\beta$ and $d$ (section 7). Only Trackblazer's interval excludes 1: its $\beta$
over-predicts these runs. Split instead by the lowest limit break of the sourcing cards, the LB0, LB1 and LB2 intervals
each cover 1.

| Scenario | Grand Concert | Unity Cup | URA Finale | Trackblazer |
|---|---|---|---|---|
| Observed / predicted [95%] | {{13.1_training_pool_only_grand_concert_obs_per_pred|.3f}} [{{13.1_training_pool_only_grand_concert_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_grand_concert_ratio_95_pct_high|.2f}}] | {{13.1_training_pool_only_unity_cup_obs_per_pred|.3f}} [{{13.1_training_pool_only_unity_cup_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_unity_cup_ratio_95_pct_high|.2f}}] | {{13.1_training_pool_only_ura_finale_obs_per_pred|.3f}} [{{13.1_training_pool_only_ura_finale_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_ura_finale_ratio_95_pct_high|.2f}}] | {{13.1_training_pool_only_trackblazer_obs_per_pred|.3f}} [{{13.1_training_pool_only_trackblazer_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_trackblazer_ratio_95_pct_high|.2f}}] |

*Every open target in one run.* The share of runs acquiring all their open targets matches the mean product of their
per-target predictions:

| Runs | Product of the model's odds | Observed [95%] |
|---|---|---|
| all | {{13.per_run_all_open_acquired_product_of_odds|.4f}} | {{13.per_run_all_open_acquired_observed|.4f}} [{{13.per_run_all_open_acquired_95_pct_low|.4f}}, {{13.per_run_all_open_acquired_95_pct_high|.4f}}] |
| {{13.per_run_priority_assumption_fits_runs|,.0f}} with every open target prioritised or unknown | {{13.per_run_priority_assumption_fits_all_open_acquired_product_of_odds|.4f}} | {{13.per_run_priority_assumption_fits_all_open_acquired_observed|.4f}} [{{13.per_run_priority_assumption_fits_all_open_acquired_95_pct_low|.4f}}, {{13.per_run_priority_assumption_fits_all_open_acquired_95_pct_high|.4f}}] |

*Spark-weighted acquisition of all targets.* $P(\text{all eight})$ is recomputed from what each run acquired and
compared with the model's exact $P(\text{all})$; Results gives the ratio, its interval bootstrapping whole runs. The
shortfall sits in the runs showing no priority list, where the model assumes every target prioritised:
{{13.p_all_eight_ratio_no_list_shown|.2f}} [{{13.p_all_eight_ratio_no_list_shown_95_pct_low|.2f}}, {{13.p_all_eight_ratio_no_list_shown_95_pct_high|.2f}}];
runs showing a list read {{13.p_all_eight_ratio_list_shown|.2f}} [{{13.p_all_eight_ratio_list_shown_95_pct_low|.2f}}, {{13.p_all_eight_ratio_list_shown_95_pct_high|.2f}}], which covers 1. Only
{{13.runs_acquiring_every_open_target|.0f}} runs acquired every open target, too few to separate the exact joint from the product.

*Single cards.* Where one card is the deck's only source of a target, the deck test scores that card alone. The table
lists every such pair of 40 or more rows whose interval excludes 1.

| Card > target | Rows | Predicted | Obs / pred [95%] | Likely reason |
|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_rows|.0f}} | {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_predicted|.3f}} | {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio|.2f}} [{{13.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio_95_pct_low|.2f}}, {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio_95_pct_high|.2f}}] | the model assumes Risky Business prioritised, which takes finale choice 2 |
| Riko Kashimoto [Planned Perfection] > Ramp Up | {{13.planned_perfection_riko_kashimoto_gt_ramp_up_rows|.0f}} | {{13.planned_perfection_riko_kashimoto_gt_ramp_up_predicted|.3f}} | {{13.planned_perfection_riko_kashimoto_gt_ramp_up_ratio|.2f}} [{{13.planned_perfection_riko_kashimoto_gt_ramp_up_ratio_95_pct_low|.2f}}, {{13.planned_perfection_riko_kashimoto_gt_ramp_up_ratio_95_pct_high|.2f}}] | open; her Lone Wolf choice, when prioritised, accounts for part |
| King Halo [Tonight, We Waltz] > Uma Stan | {{13.tonight_we_waltz_king_halo_gt_uma_stan_rows|.0f}} | {{13.tonight_we_waltz_king_halo_gt_uma_stan_predicted|.3f}} | {{13.tonight_we_waltz_king_halo_gt_uma_stan_ratio|.2f}} [{{13.tonight_we_waltz_king_halo_gt_uma_stan_ratio_95_pct_low|.2f}}, {{13.tonight_we_waltz_king_halo_gt_uma_stan_ratio_95_pct_high|.2f}}] | Uma Stan sits on a choice that needs a priority |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_rows|.0f}} | {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_predicted|.3f}} | {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio|.2f}} [{{13.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio_95_pct_low|.2f}}, {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio_95_pct_high|.2f}}] | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_rows|.0f}} | {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_predicted|.3f}} | {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio|.2f}} [{{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio_95_pct_low|.2f}}, {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio_95_pct_high|.2f}}] | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | {{13.sunlit_angel_daitaku_helios_gt_slipstream_rows|.0f}} | {{13.sunlit_angel_daitaku_helios_gt_slipstream_predicted|.3f}} | {{13.sunlit_angel_daitaku_helios_gt_slipstream_ratio|.2f}} [{{13.sunlit_angel_daitaku_helios_gt_slipstream_ratio_95_pct_low|.2f}}, {{13.sunlit_angel_daitaku_helios_gt_slipstream_ratio_95_pct_high|.2f}}] | open |
| Tokai Teio [Step! Smile! Wink!] > Nimble Navigator | {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_rows|.0f}} | {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_predicted|.3f}} | {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_ratio|.2f}} [{{13.step_smile_wink_tokai_teio_gt_nimble_navigator_ratio_95_pct_low|.2f}}, {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_ratio_95_pct_high|.2f}}] | open |
| Yukino Bijin [Dancing Light into the Night] > Playtime's Over! | {{13.dancing_light_into_the_night_yukino_bijin_gt_playtime_s_over_rows|.0f}} | {{13.dancing_light_into_the_night_yukino_bijin_gt_playtime_s_over_predicted|.3f}} | {{13.dancing_light_into_the_night_yukino_bijin_gt_playtime_s_over_ratio|.2f}} [{{13.dancing_light_into_the_night_yukino_bijin_gt_playtime_s_over_ratio_95_pct_low|.2f}}, {{13.dancing_light_into_the_night_yukino_bijin_gt_playtime_s_over_ratio_95_pct_high|.2f}}] | the skill sits on a chain-ending choice 2, taken only when prioritised |

*King Halo's Uma Stan follows the priority.* The expectation and the exact Poisson-binomial tail are the model's:

| Runs that list Uma Stan | Rows | Hinted by her card | Expected | $P(X \le \text{hinted})$ |
|---|---|---|---|---|
| prioritised | {{13.king_halo_gt_uma_stan_prioritised_rows|.0f}} | {{13.king_halo_gt_uma_stan_prioritised_hinted|.0f}} | {{13.king_halo_gt_uma_stan_prioritised_expected|.1f}} | {{13.king_halo_gt_uma_stan_prioritised_p_at_most_the_hinted|.3f}}, consistent |
| a list without it | {{13.king_halo_gt_uma_stan_on_a_list_not_prioritised_rows|.0f}} | {{13.king_halo_gt_uma_stan_on_a_list_not_prioritised_hinted|.0f}} | {{13.king_halo_gt_uma_stan_on_a_list_not_prioritised_expected|.1f}} | |

*Smaller pairs.* Below 40 rows, {{13.pairs_under_40_rows_excluding_1|.0f}} more pairs exclude 1. {{13.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources|.0f}} of them are
chain-ending event sources of Playtime's Over! (its three are Yukino Bijin [Dancing Light into the Night], Yaeno Muteki
[Fiery Discipline] and Sirius Symboli [Escorte Étoile]; one with 40 or more rows sits in the table above). They hinted
it in {{13.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources_hinted|.0f}} of {{13.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources_runs|.0f}} runs, where the model predicted at least {{13.pairs_under_40_rows_excluding_1_playtime_s_over_lowest_predicted|.2f}}. Their event puts the skill on
choice 2, which ends the chain before its last link, so IT takes it only when the skill is prioritised. Reading each
run's list accounts for the miss on runs that show one: prioritised rows match the model, and rows whose list leaves it
out are predicted none and show none. Runs showing no list stay below the model, which there assumes the end can be
taken.

*Other skills.* Every non-target skill a deck card or spark could give, on a 30% sample of {{13.other_skills_runs_sampled|,.0f}} runs:

| Test | Rows | Observed / predicted [95%] | Verdict |
|---|---|---|---|
| inheritance | {{13.other_skills_inheritance_rows|,.0f}} | {{13.other_skills_inheritance|.3f}} [{{13.other_skills_inheritance_95_pct_low|.3f}}, {{13.other_skills_inheritance_95_pct_high|.3f}}] | slightly short: known miss; no one skill carries more than {{13.other_skills_largest_one_skill_s_share_of_the_inheritance_shortfall|.0%}} of it |
| training-pool hints | {{13.other_skills_training_pool_rows|,.0f}} | {{13.other_skills_training_pool_hints|.3f}} [{{13.other_skills_training_pool_95_pct_low|.3f}}, {{13.other_skills_training_pool_95_pct_high|.3f}}] | under-predicted: known miss |
