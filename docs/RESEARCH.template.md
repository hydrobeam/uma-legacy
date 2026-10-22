# The model's mathematics

What this project measured, fitted, tested or derived for its model of how a career in Uma Musume's Independent
Training acquires and passes on skills. Every constant the model uses, with its source, is in `docs/MODEL.md`; this
document keeps the evidence.

## Results

The model's odds rest on one prediction: the chance that a career acquires each target. Section 13 tests it on
{{13.runs_used|,.0f}} real Independent Training careers, each rebuilt from its own deck and lineage. Aggregate acquisition is consistent with
the product approximation on rows whose target is prioritised or whose priority list is unknown. Unknown lists do not
establish the priority assumption. Over all rows the approximation is slightly optimistic.

| Rows | Rows (n) | Observed / predicted [95%] |
|---|---|---|
| target prioritised, or no priority list shown | {{13.3_acquired_any_source_prioritised_or_no_list_shown_rows|,.0f}} | {{13.3_acquired_any_source_prioritised_or_no_list_shown_obs_per_pred|.3f}} [{{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_high|.3f}}] |
| all | {{13.3_acquired_any_source_rows|,.0f}} | {{13.3_acquired_any_source_obs_per_pred|.3f}} [{{13.3_acquired_any_source_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_ratio_95_pct_high|.3f}}] |
| target shown not prioritised | {{13.3_acquired_not_prioritised_rows|,.0f}} | {{13.3_acquired_not_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_not_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_not_prioritised_ratio_95_pct_high|.3f}}] |
| all eight default targets landed, per run | | {{13.p_all_eight_ratio_all|.2f}} [{{13.p_all_eight_ratio_all_95_pct_low|.2f}}, {{13.p_all_eight_ratio_all_95_pct_high|.2f}}] |

The not-prioritised rows carry {{13.3_acquired_shortfall_share_on_non_prioritised_rows|.0%}} of the shortfall; this allocation does not establish its cause.
The last row is a spark-weighted acquisition check against the exact joint model, with only
{{13.runs_acquiring_every_open_target|.0f}} runs acquiring every target. It tests neither spark generation nor the ranking of candidates.
The terms are defined in *How to read*.

### What each part rests on

The schedule ceiling below fixes each loop's reported decks. Scenario bands are chosen defaults informed by measured
rank distributions. The tests and their nominal labels have the limits set out in *How to read*.

| Part | How it is set | Main evidence | Section |
|---|---|---|---|
| Star band per scenario | derived | Grand Concert: {{2.grand_concert_share_at_17_500_or_more|.0%}} of careers reach 17,500+ | 2 |
| Grandparents' inheritance rate | fitted | weight {{3.grandparent_weight_fitted|.2f}} [{{3.grandparent_weight_95_pct_low|.2f}}, {{3.grandparent_weight_95_pct_high|.2f}}] against the parents' 1 | 3 |
| Affinity's effect on inheritance | assumed, tested | a free slope predicts no better than 1: $z$ = {{3.affinity_slope_refitted_over_slope_1_held_out_z|.2f}} | 3 |
| +3 affinity per shared G1 win | assumed, tested | a free weight predicts no better than 1: $z$ = {{3.race_part_slope_refitted_over_slope_1_held_out_z|.2f}} | 3 |
| Own grandparent adds no trio points | fitted | {{5.ll_0_ll_pair_nats|.0f}} nats better than the alternative | 5 |
| Race win table | community table, tested; goal shift fitted | {{4.race_obs_per_pred_pink|.3f}} [{{4.race_obs_per_pred_pink_95_pct_low|.3f}}, {{4.race_obs_per_pred_pink_95_pct_high|.3f}}] over {{4.race_attempts_at_the_34|,.0f}} G1 attempts | 4 |
| Loop schedules for shared wins | derived | no other schedule could raise the best loop's odds by more than {{6.schedules_pb_ceiling_gain_max_pct|.2f}}% | 6 |
| Training hints, $\beta(1 + hf/100)(1 + d\,\mathit{sp}/100)$ | form fixed on the runs, $\beta$ and $d$ fitted | real careers: {{13.1_training_pool_only_obs_per_pred|.3f}} [{{13.1_training_pool_only_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_ratio_95_pct_high|.2f}}] of the model | 7 |
| Chain events, Binomial(21, 0.35) per career | fitted, kept by choice | no alternative predicts unseen runs better | 8 |
| Chain events spread over remaining links | fitted | {{8.allocation_slot_over_card_nats|.0f}} nats better than equal shares per card | 8 |
| IT takes a prioritised skill's choice | assumed, tested on careers | prioritised targets' event hints: {{13.1_events_only_prioritised_obs_per_pred|.3f}} of the model | 9, 13 |
| Measured outcome shares | fitted | five events, e.g. Nakayama Festa's Nothing Ventured {{9.festa_share|.2f}} | 9 |
| Random card events | not modelled | left out, those targets read {{10.calibration_obs_per_pred_left_out|.2f}} of the model | 10 |
| Conditional character events | not modelled | no significant cost, except Vodka's own Nimble Navigator | 11 |

### Known misses

Each is measured and tested in its section. These are exploratory discrepancies under the tests below; pointwise
intervals and nominal significance do not establish every listed miss after selection across many comparisons.

| Part | Miss | Observed / predicted | Section |
|---|---|---|---|
| Inheritance | Tail Held High is inherited less often than predicted; cause unknown | {{13.tail_held_high_inheritance|.3f}} [{{13.tail_held_high_inheritance_95_pct_low|.3f}}, {{13.tail_held_high_inheritance_95_pct_high|.3f}}] | 3, 13 |
| Inheritance | parents at affinity 200 or more pass sparks less often | {{3.parents_at_affinity_200_and_above_obs_per_pred|.3f}} [{{3.parents_at_affinity_200_and_above_95_pct_low|.3f}}, {{3.parents_at_affinity_200_and_above_95_pct_high|.3f}}] | 3 |
| Inheritance | non-target skills, the shortfall spread over many skills | {{13.other_skills_inheritance|.3f}} [{{13.other_skills_inheritance_95_pct_low|.3f}}, {{13.other_skills_inheritance_95_pct_high|.3f}}] | 13 |
| Races | chances of 70 or less | {{4.race_pink_bp_lt_70_obs_per_pred|.2f}} [{{4.race_pink_bp_lt_70_obs_per_pred_95_pct_low|.2f}}, {{4.race_pink_bp_lt_70_obs_per_pred_95_pct_high|.2f}}] | 4 |
| Races | chosen races, 1st in a row | {{4.race_pink_chosen_j_1_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_1_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_1_obs_per_pred_95_pct_high|.3f}}] | 4 |
| Races | chosen races, 2nd in a row | {{4.race_pink_chosen_j_2_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_2_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_2_obs_per_pred_95_pct_high|.3f}}] | 4 |
| Races | goal races 7th or later in a row, which the planner charges as the 6th | {{4.race_pink_goal_j_ge_7_obs_per_pred|.2f}} [{{4.race_pink_goal_j_ge_7_obs_per_pred_95_pct_low|.2f}}, {{4.race_pink_goal_j_ge_7_obs_per_pred_95_pct_high|.2f}}] | 4 |
| Training hints | cards differ beyond their effects; cause open | per card {{7.per_card_obs_per_pred_min|.2f}}-{{7.per_card_obs_per_pred_max|.2f}} | 7 |
| Training hints | a skill also hinted by inspiration | {{7.spark_inspiration_ratio|.2f}} | 7 |
| Training hints | more pals per deck give slightly more hints | factor {{7.covariate_factor_pals_per_deck|.3f}} per pal | 7 |
| Training hints | Trackblazer's base over-predicts real careers | {{13.1_training_pool_only_trackblazer_obs_per_pred|.3f}} [{{13.1_training_pool_only_trackblazer_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_trackblazer_ratio_95_pct_high|.2f}}] | 13 |
| Training hints | non-target skills are under-predicted | {{13.other_skills_training_pool_hints|.3f}} [{{13.other_skills_training_pool_95_pct_low|.3f}}, {{13.other_skills_training_pool_95_pct_high|.3f}}] | 13 |
| Chain events | Binomial(21, 0.35) is rejected in-sample | p-value {{8.binomial_21_0_35_lr_p|.1e}} | 8 |
| Chain events | decks with Sasami Anshinzawa [This Might Sting!]'s pal card get more | mean {{8.sasami_anshinzawa_s_card_mean_of_n|.2f}} against {{8.sasami_anshinzawa_s_card_mean_of_n_the_other_runs|.2f}} | 8 |
| Chain events | Inari One's careers get fewer | mean {{8.inari_one_mean_of_n|.2f}} against {{8.inari_one_mean_of_n_the_other_runs|.2f}} | 8 |
| Chain events | with six chain cards, more chains never start | {{8.never_started_6_chain_cards_observed|.2f}} against {{8.never_started_6_chain_cards_model|.2f}} | 8 |
| Chain events | individual cards' final links differ beyond chance | $\chi^2$ = {{8.final_links_by_card_chi2|.0f}} on {{8.final_links_by_card_cards|.0f}} cards | 8 |
| Event choices | prioritised targets' event hints read just below the model | {{13.1_events_only_prioritised_obs_per_pred|.3f}} [{{13.1_events_only_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_prioritised_ratio_95_pct_high|.4f}}] | 9, 13 |
| Left out | targets a deck card's random event can hint | {{10.calibration_obs_per_pred_left_out|.2f}} [{{10.calibration_ratio_95_pct_low_left_out|.2f}}, {{10.calibration_ratio_95_pct_high_left_out|.2f}}] | 10 |
| Left out | Vodka's own Nimble Navigator | her events give it in {{11.vodka_any_outfit_nimble_navigator_from_her_events|.0f}} of {{11.vodka_any_outfit_nimble_navigator_rows|.0f}} runs | 11 |

### Not settled

Specialty priority goes with more hints, but it is not shown to cause them. Fitted on earlier runs, it predicts later
ones better by {{7.specialty_held_out_gain_nats|.1f}} nats. On the cards whose limit breaks change only their specialty priority, the data fit
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
  {{2.q_overstatement_below_17_500_max|.2%}} within the assumed mixes. This is no bound on every lineage or on joint landing odds.

### Evidence

*Which band?* (**chosen** from measured rank distributions.) Each measured scenario takes the majority band. A career's rank score
is estimated with every skill point spent, over every completed run in the run list.

| Scenario | Runs | Share at 17,500+ [95%] | Band |
|---|---|---|---|
| URA Finale | {{2.ura_finale_runs|,.0f}} | {{2.ura_finale_share_at_17_500_or_more|.3f}} [{{2.ura_finale_share_95_pct_low|.3f}}, {{2.ura_finale_share_95_pct_high|.3f}}] | below |
| Unity Cup | {{2.unity_cup_runs|,.0f}} | {{2.unity_cup_share_at_17_500_or_more|.3f}} [{{2.unity_cup_share_95_pct_low|.3f}}, {{2.unity_cup_share_95_pct_high|.3f}}] | below |
| Grand Concert | {{2.grand_concert_runs|,.0f}} | {{2.grand_concert_share_at_17_500_or_more|.3f}} [{{2.grand_concert_share_95_pct_low|.3f}}, {{2.grand_concert_share_95_pct_high|.3f}}] | 17,500+ |
| Trackblazer | {{2.trackblazer_runs|,.0f}} | {{2.trackblazer_share_at_17_500_or_more|.3f}} [{{2.trackblazer_share_95_pct_low|.3f}}, {{2.trackblazer_share_95_pct_high|.3f}}] | below |

Grand Masters has no runs and is **assumed** 17,500+.

*What does averaging over stars cost?* (**derived**.) For a planned loop the model uses each legacy's expected base per
roll. That is exact for one roll only when the cap is inactive. Across both rolls the same stars persist. With $r$ a
legacy's chance per roll, convexity gives $E[(1 - r)^2] \ge (1 - E[r])^2$. So averaging overstates $q$, the chance of
an inspiration over the career (section 3).
The table measures the overstatement on the six real affinities of each of the {{2.lineages|,.0f}} lineages in the runs, for a target
carried by all six. The calculation assumes independent unknown stars across slots; repeated appearances of the same
veteran would share her stars. With capping, the squared miss remains convex in the base, so the same direction holds
under this slot-independence assumption.

| Band | Mean overstatement of $q$ | Largest |
|---|---|---|
| below 17,500 | {{2.q_overstatement_below_17_500_mean|.2%}} | {{2.q_overstatement_below_17_500_max|.2%}} |
| 17,500+ | {{2.q_overstatement_17_500_and_up_mean|.2%}} | {{2.q_overstatement_17_500_and_up_max|.2%}} |

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
spark fires. There are {{3.trials_parents|,.0f}} parent trials and {{3.trials_grandparents|,.0f}} grandparent trials of ordinary ancestors; the trainee's own
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
| Grandparents take the full base | weight {{3.grandparent_weight_fitted|.2f}} [{{3.grandparent_weight_95_pct_low|.2f}}, {{3.grandparent_weight_95_pct_high|.2f}}] | full base beats half by {{3.grandparents_full_over_halved_nats|,.0f}} nats | full base |
| Affinity raises the roll at all | | beats no affinity by {{3.affinity_over_none_nats|,.0f}} nats | kept |
| Affinity slope is 1 | {{3.affinity_slope_fitted|.2f}} [{{3.affinity_slope_95_pct_low|.2f}}, {{3.affinity_slope_95_pct_high|.2f}}] | held out {{3.affinity_slope_refitted_over_slope_1_held_out_nats|.1f}} ± {{3.affinity_slope_refitted_over_slope_1_held_out_se|.1f}} nats, $z$ = {{3.affinity_slope_refitted_over_slope_1_held_out_z|.2f}} | 1 kept |
| Relation-point weight is 1 | {{3.relation_part_slope_fitted|.2f}} [{{3.relation_part_slope_95_pct_low|.2f}}, {{3.relation_part_slope_95_pct_high|.2f}}] | held out {{3.relation_part_slope_refitted_over_slope_1_held_out_nats|.1f}} ± {{3.relation_part_slope_refitted_over_slope_1_held_out_se|.1f}} nats, $z$ = {{3.relation_part_slope_refitted_over_slope_1_held_out_z|.2f}} | 1 kept |
| Weight of 3 per shared G1 win is 1 | {{3.race_part_slope_fitted|.2f}} [{{3.race_part_slope_95_pct_low|.2f}}, {{3.race_part_slope_95_pct_high|.2f}}] | held out {{3.race_part_slope_refitted_over_slope_1_held_out_nats|.1f}} ± {{3.race_part_slope_refitted_over_slope_1_held_out_se|.1f}} nats, $z$ = {{3.race_part_slope_refitted_over_slope_1_held_out_z|.2f}} | 1 kept |

The free slope gains {{3.affinity_slope_refitted_over_slope_1_in_sample_nats|.1f}} nats in-sample, which held out does not keep. Each part's weight is fitted with the
other fixed; {{3.trials_with_shared_wins|,.0f}} trials have shared wins.

*Grandparents.* A halved base falls far short of the passes seen:

| Grandparent passes | Count |
|---|---|
| observed | {{3.grandparents_passes_observed|,.0f}} |
| predicted with the full base | {{3.grandparents_predicted_full_base|,.0f}} |
| predicted with half | {{3.grandparents_predicted_halved_base|,.0f}} |

Parents read {{3.parents_obs_per_pred|.3f}} of the model, the shortfall coming from high affinity (below).

*The bases by stars* all cover 1:

| Stars | Trials | Observed / predicted [95%] |
|---|---|---|
| 1★ | {{3.1_star_trials|,.0f}} | {{3.1_star_obs_per_pred|.3f}} [{{3.1_star_95_pct_low|.2f}}, {{3.1_star_95_pct_high|.2f}}] |
| 2★ | {{3.2_star_trials|,.0f}} | {{3.2_star_obs_per_pred|.3f}} [{{3.2_star_95_pct_low|.2f}}, {{3.2_star_95_pct_high|.2f}}] |
| 3★ | {{3.3_star_trials|,.0f}} | {{3.3_star_obs_per_pred|.3f}} [{{3.3_star_95_pct_low|.2f}}, {{3.3_star_95_pct_high|.2f}}] |

*Every affinity?* Below 200, yes. The band outside its own 95% interval is {{3.affinity_bands_outside_their_own_95_pct_interval|s}}.

| Affinity | Trials | Observed / predicted [95%] |
|---|---|---|
| 0-49 | {{3.affinity_0_49_trials|,.0f}} | {{3.affinity_0_49_obs_per_pred|.3f}} [{{3.affinity_0_49_95_pct_low|.3f}}, {{3.affinity_0_49_95_pct_high|.3f}}] |
| 50-99 | {{3.affinity_50_99_trials|,.0f}} | {{3.affinity_50_99_obs_per_pred|.3f}} [{{3.affinity_50_99_95_pct_low|.3f}}, {{3.affinity_50_99_95_pct_high|.3f}}] |
| 100-149 | {{3.affinity_100_149_trials|,.0f}} | {{3.affinity_100_149_obs_per_pred|.3f}} [{{3.affinity_100_149_95_pct_low|.3f}}, {{3.affinity_100_149_95_pct_high|.3f}}] |
| 150-199 | {{3.affinity_150_199_trials|,.0f}} | {{3.affinity_150_199_obs_per_pred|.3f}} [{{3.affinity_150_199_95_pct_low|.3f}}, {{3.affinity_150_199_95_pct_high|.3f}}] |
| 200+ | {{3.affinity_200_trials|,.0f}} | {{3.affinity_200_obs_per_pred|.3f}} [{{3.affinity_200_95_pct_low|.3f}}, {{3.affinity_200_95_pct_high|.3f}}] |
| parents, below 200 | {{3.parents_at_affinity_below_200_trials|,.0f}} | {{3.parents_at_affinity_below_200_obs_per_pred|.3f}} [{{3.parents_at_affinity_below_200_95_pct_low|.3f}}, {{3.parents_at_affinity_below_200_95_pct_high|.3f}}] |
| parents, 200+ | {{3.parents_at_affinity_200_and_above_trials|,.0f}} | {{3.parents_at_affinity_200_and_above_obs_per_pred|.3f}} [{{3.parents_at_affinity_200_and_above_95_pct_low|.3f}}, {{3.parents_at_affinity_200_and_above_95_pct_high|.3f}}] |

| Joint test of the bands | $\chi^2$ | $p$ | Verdict |
|---|---|---|---|
| the four below 200 | {{3.affinity_bands_below_200_chi2|.1f}} | {{3.affinity_bands_below_200_chi2_p|.2f}} | consistent |
| all five | {{3.affinity_bands_chi2|.1f}} | {{3.affinity_bands_chi2_p|.3f}} | not consistent: a known miss |

*Whole careers.* Section 13 gives the totals and the Tail Held High discrepancy. Without it, the finer checks are consistent
over {{3.inheritance_without_tail_held_high_rows|,.0f}} rows:

| Check, without Tail Held High | Observed / expected | Verdict |
|---|---|---|
| by the number of carriers, 1 to 6 | {{3.by_carriers_1_6_without_tail_held_high_lowest_ratio|.3f}}-{{3.by_carriers_1_6_without_tail_held_high_highest_ratio|.3f}} | every interval covers 1 |
| years in which an inspiration of the skill fired | {{3.per_roll_without_tail_held_high_spark_years_obs_per_exp|.3f}} | {{3.per_roll_without_tail_held_high_spark_years_sds_below_expectation|.2f}} sd below, inside the 95% band |
| legacy firings, against $2\sum_l r_l$ | {{3.per_roll_without_tail_held_high_ancestor_hits_obs_per_exp|.3f}} | |

*Every predicted chance?* The rows are grouped by predicted $q$ into {{3.reliability_chi2_buckets|.0f}} groups, whose joint test does not reject the prediction.
Their joint test gives $\chi^2$ = {{3.reliability_chi2|.1f}}, $p$ = {{3.reliability_chi2_p|.2f}}. One group, {{3.reliability_buckets_outside_their_own_95_pct_interval|s}}, lies outside its own 95% interval.

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

{{4.race_grid_by_aptitude_and_streak|s}}

### Evidence

The runs record every race a career ran: {{4.race_attempts_at_the_34|,.0f}} attempts at the 34 single-race G1s. Of them, {{4.race_goal_attempts|,.0f}} were goal
races and {{4.race_chosen_attempts|,.0f}} chosen. {{4.race_win_rate_observed|.1%}} were won.

The table calls some races certain, 0% or 100%, and a handful went the other way. That would give the table a
log-likelihood of $-\infty$. So every model compared has its 0% and 100% pulled in to a fitted floor and ceiling, the
same two parameters in each, and the comparisons follow rule 1 of *How to read*.

| Claim | Estimate [95%] | Test | Verdict |
|---|---|---|---|
| Raised grades, not base | raised: {{4.race_obs_per_pred_pink|.3f}} [{{4.race_obs_per_pred_pink_95_pct_low|.3f}}, {{4.race_obs_per_pred_pink_95_pct_high|.3f}}] | {{4.race_ll_recalibrated_pink_table_recalibrated_base_table_nats|,.0f}} nats better than base | raised grades |
| The streak penalty is real | | without it, {{4.race_ll_recalibrated_pink_table_recalibrated_pink_no_penalty_nats|,.0f}} nats worse | kept |
| Goal races pay one position less | community table, goal races 4th+: {{4.race_community_table_goal_j_ge_4_obs_per_pred|.3f}} [{{4.race_community_table_goal_j_ge_4_obs_per_pred_95_pct_low|.3f}}, {{4.race_community_table_goal_j_ge_4_obs_per_pred_95_pct_high|.3f}}] | held out, forms table below | shift adopted |
| The previous result does not matter | every loss / win contrast covers 1 | $z$ = {{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred_z|.2f}} from the 4th race on | no effect |
| A pink counts from the April of its inspiration | won {{4.proc_timing_observed|.3f}} [{{4.proc_timing_observed_95_pct_low|.3f}}, {{4.proc_timing_observed_95_pct_high|.3f}}] | predicted {{4.proc_timing_predicted_from_april|.3f}} from April, {{4.proc_timing_predicted_from_the_year_start|.3f}} from the year's start | from April |
| One formula is enough | per-cell rates {{4.race_ll_pink_cells_bp_x_j_x_goal_recalibrated_pink_table_nats|.0f}} nats better in-sample | held out {{4.held_out_cells_vs_table_nats|.0f}} ± {{4.held_out_cells_vs_table_se|.0f}} nats, $z$ = {{4.held_out_cells_vs_table_z|.2f}} | table kept |

*Base or raised grades.* Base aptitudes are what a planned loop has. Raised grades follow the aptitude-spark rule in
`docs/MODEL.md`.

| Grades | Predicted win rate | Observed / predicted [95%] | Where the table gives 70 or less |
|---|---|---|---|
| base | {{4.race_win_rate_predicted_base|.1%}} | {{4.race_obs_per_pred_base|.3f}} [{{4.race_obs_per_pred_base_95_pct_low|.3f}}, {{4.race_obs_per_pred_base_95_pct_high|.3f}}] | {{4.race_base_bp_lt_70_obs_per_pred|.2f}} over {{4.race_base_bp_lt_70_attempts|,.0f}} attempts |
| raised | {{4.race_win_rate_predicted_pink|.1%}} | {{4.race_obs_per_pred_pink|.3f}} [{{4.race_obs_per_pred_pink_95_pct_low|.3f}}, {{4.race_obs_per_pred_pink_95_pct_high|.3f}}] | {{4.race_pink_bp_lt_70_obs_per_pred|.2f}} |

Outside chosen races at the 1st and 2nd race in a row, every scenario is consistent with the prediction. Over all attempts, the scenarios
outside their intervals are: {{4.race_pink_scenarios_outside_their_interval|s}}.

*Goal races.* Under the community table, goal races from the 4th race in a row win more than predicted, while chosen
races there match. Three forms were scored on runs left out of the fit, five folds by run over {{4.held_out_runs|,.0f}} runs, each with
the same fitted floor and ceiling:

| Form | Goal race at position $j$ pays | Races after it count it | Held out, against the adopted form |
|---|---|---|---|
| Adopted | $\mathrm{PEN}_{\max(j-1,1)}$ | yes | |
| Community table | $\mathrm{PEN}_j$ | yes | {{4.held_out_goal_shift_vs_community_table_nats|,.0f}} ± {{4.held_out_goal_shift_vs_community_table_se|.0f}} nats worse, $z$ = {{4.held_out_goal_shift_vs_community_table_z|.2f}} |
| Exclusion | $\mathrm{PEN}$ at the chosen races before it | no | {{4.held_out_goal_shift_vs_exclusion_nats|,.0f}} ± {{4.held_out_goal_shift_vs_exclusion_se|.0f}} nats worse, $z$ = {{4.held_out_goal_shift_vs_exclusion_z|.2f}} |

In-sample the adopted form fits {{4.race_ll_recalibrated_pink_table_recalibrated_community_table_nats|,.0f}} nats better than the community table. With the adopted shift, the grouped goal-race intervals cover the prediction, and chosen races from the 3rd on. Chosen races at the 1st and 2nd fall slightly short, a known miss.

| Race in a row | Goal races: observed / predicted [95%] | Attempts | Chosen races: observed / predicted [95%] | Attempts |
|---|---|---|---|---|
| 1st | {{4.race_pink_goal_j_1_obs_per_pred|.3f}} [{{4.race_pink_goal_j_1_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_1_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_1_attempts|,.0f}} | {{4.race_pink_chosen_j_1_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_1_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_1_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_1_attempts|,.0f}} |
| 2nd | {{4.race_pink_goal_j_2_obs_per_pred|.3f}} [{{4.race_pink_goal_j_2_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_2_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_2_attempts|,.0f}} | {{4.race_pink_chosen_j_2_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_2_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_2_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_2_attempts|,.0f}} |
| 3rd | {{4.race_pink_goal_j_3_obs_per_pred|.3f}} [{{4.race_pink_goal_j_3_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_3_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_3_attempts|,.0f}} | {{4.race_pink_chosen_j_3_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_3_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_3_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_3_attempts|,.0f}} |
| 4th | {{4.race_pink_goal_j_4_obs_per_pred|.3f}} [{{4.race_pink_goal_j_4_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_4_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_4_attempts|,.0f}} | {{4.race_pink_chosen_j_4_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_4_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_4_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_4_attempts|,.0f}} |
| 5th | {{4.race_pink_goal_j_5_obs_per_pred|.3f}} [{{4.race_pink_goal_j_5_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_5_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_5_attempts|,.0f}} | {{4.race_pink_chosen_j_5_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_5_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_5_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_5_attempts|,.0f}} |
| 6th or later | {{4.race_pink_goal_j_6_obs_per_pred|.3f}} [{{4.race_pink_goal_j_6_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_6_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_goal_j_6_attempts|,.0f}} | {{4.race_pink_chosen_j_6_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_6_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_6_obs_per_pred_95_pct_high|.3f}}] | {{4.race_pink_chosen_j_6_attempts|,.0f}} |
| goal, 7th or later | {{4.race_pink_goal_j_ge_7_obs_per_pred|.2f}} [{{4.race_pink_goal_j_ge_7_obs_per_pred_95_pct_low|.2f}}, {{4.race_pink_goal_j_ge_7_obs_per_pred_95_pct_high|.2f}}] | {{4.race_pink_goal_j_ge_7_attempts|,.0f}} | | |

*Every chance?* On raised grades, the pointwise intervals cover the prediction down to a chance of 80:

| Table's chance | Attempts | Observed / predicted [95%] |
|---|---|---|
| 110 | {{4.race_pink_bp_110_attempts|,.0f}} | {{4.race_pink_bp_110_obs_per_pred|.3f}} [{{4.race_pink_bp_110_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_110_obs_per_pred_95_pct_high|.3f}}] |
| 100 | {{4.race_pink_bp_100_attempts|,.0f}} | {{4.race_pink_bp_100_obs_per_pred|.3f}} [{{4.race_pink_bp_100_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_100_obs_per_pred_95_pct_high|.3f}}] |
| 90 | {{4.race_pink_bp_90_attempts|,.0f}} | {{4.race_pink_bp_90_obs_per_pred|.3f}} [{{4.race_pink_bp_90_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_90_obs_per_pred_95_pct_high|.3f}}] |
| 80 | {{4.race_pink_bp_80_attempts|,.0f}} | {{4.race_pink_bp_80_obs_per_pred|.3f}} [{{4.race_pink_bp_80_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_80_obs_per_pred_95_pct_high|.3f}}] |
| 70 or less | {{4.race_pink_bp_lt_70_attempts|,.0f}} | {{4.race_pink_bp_lt_70_obs_per_pred|.3f}} [{{4.race_pink_bp_lt_70_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_lt_70_obs_per_pred_95_pct_high|.3f}}] |

*A pink's timing.* The records say which inspiration fired a pink, and inspirations sit at fixed turns. The timing
matters on {{4.proc_timing_attempts|,.0f}} Senior January-March attempts; the claims table compares them.

*The previous race's result.* The test divides observed / predicted after a loss by the same after a win, with a
run-clustered interval. Every contrast covers 1; this supports retaining the race-count penalty without establishing absence of a win-history effect.

| Race in a row | After a win [95%] | Attempts | After a loss [95%] | Attempts | Loss / win [95%] |
|---|---|---|---|---|---|
| 2nd | {{4.race_in_a_streak_j_2_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_2_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_2_after_a_win_obs_per_pred_95_pct_high|.3f}}] | {{4.race_in_a_streak_j_2_after_a_win_attempts|,.0f}} | {{4.race_in_a_streak_j_2_after_a_loss_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_2_after_a_loss_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_2_after_a_loss_obs_per_pred_95_pct_high|.3f}}] | {{4.race_in_a_streak_j_2_after_a_loss_attempts|,.0f}} | {{4.race_in_a_streak_j_2_after_a_loss_per_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_2_after_a_loss_per_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_2_after_a_loss_per_after_a_win_obs_per_pred_95_pct_high|.3f}}] |
| 3rd | {{4.race_in_a_streak_j_3_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_3_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_3_after_a_win_obs_per_pred_95_pct_high|.3f}}] | {{4.race_in_a_streak_j_3_after_a_win_attempts|,.0f}} | {{4.race_in_a_streak_j_3_after_a_loss_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_3_after_a_loss_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_3_after_a_loss_obs_per_pred_95_pct_high|.3f}}] | {{4.race_in_a_streak_j_3_after_a_loss_attempts|,.0f}} | {{4.race_in_a_streak_j_3_after_a_loss_per_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_3_after_a_loss_per_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_3_after_a_loss_per_after_a_win_obs_per_pred_95_pct_high|.3f}}] |
| 4th or later | {{4.race_in_a_streak_j_4_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_4_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_4_after_a_win_obs_per_pred_95_pct_high|.3f}}] | {{4.race_in_a_streak_j_4_after_a_win_attempts|,.0f}} | {{4.race_in_a_streak_j_4_after_a_loss_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_4_after_a_loss_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_4_after_a_loss_obs_per_pred_95_pct_high|.3f}}] | {{4.race_in_a_streak_j_4_after_a_loss_attempts|,.0f}} | {{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred_95_pct_high|.3f}}] |

*One formula.* Giving each of the {{4.race_pink_cells_bp_x_j_x_goal|.0f}} (chance, streak position, goal or chosen) cells its own win rate fits better
in-sample. The gain exceeds the {{4.race_ll_threshold_pink_cells_bp_x_j_x_goal|.0f}} nats of the nominal independent-attempt threshold for {{4.race_ll_pink_cells_bp_x_j_x_goal_extra_parameters|.0f}} extra parameters if attempts were independent. The shared career state can make those attempts dependent. Held out, the per-cell rates predict no better, so the table stands.

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
  tool does, is about {{5.ll_0_ll_pair_nats|.0f}} nats worse.

### Evidence

The fit uses {{5.trials_attributable_to_one_ancestor|,.0f}} inspiration trials attributable to a single ancestor. In {{5.self_grandparent_passes_observed|.0f}} of them a self-grandparent passed her
spark. The distance of that count from each prediction is in run-clustered standard deviations:

| $\mathrm{tri}(T,P,T)$ | Passes predicted | Distance from observed, at slope 1 | At the slope refitted on ordinary ancestors |
|---|---|---|---|
| 0 (adopted) | {{5.predicted_tri_t_p_t_0|.1f}} | {{5.self_grandparent_passes_z_with_0|u.2f}} | {{5.self_grandparent_passes_refitted_slope_z_with_0|u.2f}} |
| $\mathrm{pair}(T,P)$ | {{5.predicted_tri_t_p_t_pair_t_p|.1f}} | {{5.self_grandparent_passes_z_with_pair|u.2f}} | {{5.self_grandparent_passes_refitted_slope_z_with_pair|u.2f}} |

So 0 is kept and the pair rejected. A free multiplier $\kappa$, with $\mathrm{tri}(T,P,T) = \kappa\cdot\mathrm{pair}(T,P)$,
fits best at $\hat\kappa = {{5.kappa_fitted|.0f}}$, its lower bound.

The gap is not the ordinary trials' own misfit. The last column refits the affinity slope on the ordinary ancestors, to
{{5.affinity_slope_refitted_on_the_ordinary_trials|.2f}}, and the verdict stands. At slope 1, ordinary parents read {{5.ordinary_parents_observed_per_predicted|.3f}} of prediction and grandparents {{5.ordinary_grandparents_observed_per_predicted|.3f}}.

## 6. Loop structures, ranking and schedules

Computed by `research/s06_loops.py`.

A four-uma loop has {{6.four_uma_loop_lineages|.0f}} lineages $(T, P_1, P_2, X)$: a trainee, her parents, and $X$, each parent bred from
the other parent and $X$. A three-uma rotation has {{6.three_uma_rotation_lineages|.0f}}, with $X = T$, so the trainee is her own grandparent. A
structure is ranked by its lineages' mean $P(\text{all})$, a definition (`docs/MODEL.md`).

### Findings

- **The rotation occurs in practice**: {{6.a_grandparent_of_her_own_character|,.0f}} of {{6.runs|,.0f}} runs have a grandparent of the trainee's own character.
- **Every pair of umas carries the same weight in the sum of affinities.** Total expected shared wins is the chosen
  scheduling objective; equal affinity weights do not make it equivalent to the nonlinear landing score.
- **With each loop's reported decks fixed, scheduling for shared wins costs little on the references**: no schedule
  could raise a reference loop's odds by more than {{6.schedules_pb_ceiling_gain_max_pct|.2f}}%, and no listed runner-up could overtake its winner.

### Evidence

*Race weights* (**derived** from section 5's $A_P$ and $A_G$). In one lineage, $o(P_1,P_2)$ enters the six affinities
{{6.one_lineage_o_p1_p2_weight|.0f}} times. It counts twice in each $A_P$, since $P'$ is also one of $P$'s own parents. $o(P_1,X)$ and $o(P_2,X)$ enter
twice each. Summed over the lineages, every pair gets the same weight:

$$\text{loop: } {{6.one_lineage_o_p1_p2_weight|.0f}}\cdot 2 + {{6.one_lineage_o_p1_x_weight|.0f}}\cdot 4 = {{6.loop_weight_per_pair|.0f}}, \qquad \text{rotation: } {{6.one_lineage_o_p1_p2_weight|.0f}} + {{6.one_lineage_o_p1_x_weight|.0f}} + {{6.one_lineage_o_p1_x_weight|.0f}} = {{6.rotation_weight_per_pair|.0f}}.$$

The scheduler seeks a large $\sum_{a<b} o(a,b)$, the total expected shared wins, as a symmetric surrogate.

*The schedule bound* (**derived**, on the reference loops: the best loops and decks found for a fixed set of reference
setups). Schedules are chosen for shared wins, not for the score. The score is nondecreasing in every $o$: $o$ enters each
legacy's affinity with weight +3, and every later step is nondecreasing in its inputs. So, with a loop's decks fixed, no
schedule can score above the odds with every pair at a valid upper bound on its overlap. The best schedule value lies
between the feasible score found and this ceiling. Outfits are reselected at the ceiling, while decks remain fixed.
The bound is computed on {{6.schedules_result_files|.0f}} reference results, whose odds an independent implementation
reproduces to within {{6.schedules_recomputed_odds_max_relative_error|.0e}}.

Two ceilings bound the best possible overlap. $o_{\max}(a,b) = \sum_r P^{\max}_a(r)\,P^{\max}_b(r)$ is an independent
one: every offering of every G1 attempted, with no streak penalty. Each *pair ceiling* is the smaller of the bounds
obtained by optimising either member's feasible schedule against the other's componentwise best-case win chances.
The pair need not attain that ceiling jointly. These bounds come from the same search code that chose the schedules, so
they are not an independent check; they sit between $o$ and $o_{\max}$ on every pair.

| Quantity | Value | Runner-ups that reach their winner |
|---|---|---|
| shared wins found, worth over no race affinity | {{6.schedules_shared_win_value_min_pct|.0f}}-{{6.schedules_shared_win_value_max_pct|.0f}}% | |
| gain allowed by $o_{\max}$ | up to {{6.schedules_ceiling_gain_max_pct|.1f}}% | {{6.schedules_runner_ups_whose_ceiling_reaches_the_winner|.0f}} of {{6.schedules_runner_up_loops_checked|.0f}} |
| gain allowed by the pair ceilings | {{6.schedules_pb_ceiling_gain_min_pct|.2f}}-{{6.schedules_pb_ceiling_gain_max_pct|.2f}}%, median {{6.schedules_pb_ceiling_gain_median_pct|.2f}}% | {{6.schedules_runner_ups_whose_pb_ceiling_reaches_the_winner|.0f}} of {{6.schedules_runner_up_loops_checked|.0f}} |

$o_{\max}$ is far from reachable: the schedules found reach about {{6.schedules_overlap_found_as_pct_of_o_max_median|.0f}}% of it, so it cannot settle the ranking.
The pair ceilings settle this comparison with each loop's decks fixed: no listed runner-up reaches its winner under them.
They do not certify the ranking after jointly changing schedules and decks.

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
| adopted, with the specialty term | {{7.specialty_model_free_divisor_c|.0f}} | {{7.held_out_specialty_model_free_divisor_vs_1_per_100_nats|.1f}} ± {{7.held_out_specialty_model_free_divisor_vs_1_per_100_se|.1f}} nats, $z$ = {{7.held_out_specialty_model_free_divisor_vs_1_per_100_z|.1f}} | $1/100$ kept |
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
| With the term, the scenario × specialty cells fit | {{7.specialty_cells_df|.0f}} cells of 200+ rows | run-clustered $\chi^2$ = {{7.specialty_cells_joint_chi2|.1f}}, $p$ = {{7.specialty_cells_p|.2f}} | consistent |
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

The shortfall where an inspiration hinted the skill is not modelled. It is not because such a skill leaves the card's
pool. On card-runs where an inspiration hinted two pool skills, training hinted the rest at {{7.drop_out_test_observed|.3f}}. With the
two kept in the pool the model predicts {{7.drop_out_test_no_drop_out|.3f}}; with them removed, {{7.drop_out_test_drop_out|.3f}}.

*Prioritised skills.* The model assumes a priority list does not change training hints. On the {{7.priority_runs|,.0f}} runs that
show a list, {{7.priority_rows|.0f}} rows hold a prioritised pool skill. Training hinted them at {{7.priority_observed|.3f}}, against {{7.priority_predicted|.3f}}
predicted: no sign of a rise, from a weak test.

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

A run gets $N \sim \mathrm{Binomial}(21, 0.35)$ chain events, mean {{8.binomial_21_0_35_mean|.2f}}. Each event advances one non-pal chain card by one
link. The card is picked in proportion to its links not yet fired: uniformly over the remaining *slots*. A chain is 3
links on an SSR and 2 on an SR, and a stopped chain takes no more events. Pal and group cards take no slot
(**assumed**; the comparisons below are descriptive).

A chain can stop early (**assumed**, from section 9's equal outcomes). If IT's default choice at a link ends the chain
on $k$ of its $n$ outcomes, the chain stops there with chance $k/n$. Its unused slots return to the pool. The model
enumerates the exact distribution of final chain states (section 12).

### Evidence

*Reading a chain's progress.* Progress is read from marker skills, on {{8.fit_of_n_runs|,.0f}} runs. A link is observable only if every
choice and outcome of its event carries a skill appearing in no other event of that card. A credit naming two sources,
"A or B event", counts as unknown when B is another deck card whose events can give the marker. So does a marker listed
only under inspiration. The fitted distribution of $N$ is the one that best explains progress over all decks.

*The marker allowance.* The fit lets a fired link's marker go unseen with chance 0.02. Without an allowance, a run whose
later link shows while an earlier one does not has likelihood 0. Since links fire in order, those runs measure the
rate, and 0.02 is set to it:

| Earlier links implied fired by a later one | Showing no marker | Share [95%] |
|---|---|---|
| {{8.allowance_earlier_links_implied_fired|,.0f}} | {{8.allowance_marker_not_shown|.0f}} | {{8.allowance_share|.3f}} [{{8.allowance_95_pct_low|.3f}}, {{8.allowance_95_pct_high|.3f}}] |

The cause is not known, and the fit does not depend on it. Across allowances from 0.01 to 0.045, the best $p$ at
$n = 21$ stays within {{8.allowance_sensitivity_lowest_best_p_at_n_21|.2f}}-{{8.allowance_sensitivity_highest_best_p_at_n_21|.2f}}. That holds too with no allowance, on the {{8.allowance_0_runs_without_a_contradiction|,.0f}} runs that need none.

*Which distribution for $N$?* The table keeps the scripts' nominal tests. The fixed adopted binomial has no fitted
parameters, so subtracting the fitted binomial family's dimension is inappropriate. A free integer $n$ also lacks the
continuous parameter required by an ordinary Wilks profile interval. Latent-distribution tests need identifiability
and boundary checks before their reference distribution can be used. In-sample, against the free distribution:

| Form for $N$ | Nats behind the free distribution | p-value | Note |
|---|---|---|---|
| free distribution | 0 | | mean {{8.distribution_free_mean|.2f}}, variance {{8.distribution_free_variance|.2f}} |
| Binomial(21, 0.35) | {{8.binomial_21_0_35_nats_behind_distribution_free|.1f}} | {{8.binomial_21_0_35_lr_p|.1e}} | rejected; {{8.distribution_free_fit_parameters_more_than_the_binomial|.0f}} fewer parameters |
| Binomial($n$, $p$), both fitted | {{8.best_binomial_n_free_nats_behind_distribution_free|.1f}} | {{8.best_binomial_n_free_lr_p|.3f}} | best at $n$ = {{8.best_binomial_n_free_n|.0f}}, $p$ = {{8.best_binomial_n_free_p|.3f}} |
| Conway-Maxwell-Poisson | {{8.conway_maxwell_poisson_nats_behind_distribution_free|.1f}} | {{8.conway_maxwell_poisson_lr_p|.3f}} | |
| Binomial(15, 0.48) | {{8.binomial_15_0_48_nats_behind_distribution_free|.1f}} | | |
| Poisson(7.1) | {{8.poisson_7_1_nats_behind_distribution_free|.1f}} | | variance = mean, too wide; rejected |

The count capped at 18, $\min(N, 18)$, is under-dispersed: its variance is {{8.distribution_free_variance_per_mean|.2f}} of its mean. The runs cannot
tell counts above 18 apart, so the dispersion of $N$ itself is not identified. The fitted Poisson scores worse; these moments alone do not reject every
over-dispersed count distribution. The binomial is the simplest under-dispersed count. It fits better than the
Conway-Maxwell-Poisson, the other two-parameter family that can be under-dispersed, by {{8.best_binomial_n_free_over_conway_maxwell_poisson_nats|.1f}} nats.

The runs fix $N$'s mean well and its spread loosely, so $n$ and $p$ trade off along a ridge. The mean at the best $n$
is {{8.binomial_mean_n_p_at_the_best_n|.2f}}. Re-fitting $p$ at each $n$, every $n$ from {{8.binomial_n_95_pct_profile_interval_low|.0f}} to {{8.binomial_n_95_pct_profile_interval_high|.0f}} lies within 1.92 nats of the best: the 95%
nominal profile set, whose coverage is uncalibrated for integer $n$. So the best single pair is not itself a finding.
The adopted $n = 21$ lies just outside that set
in-sample, {{8.binomial_n_21_p_re_fitted_nats_below_the_best_n|.2f}} nats below the best $n$. Its variance is {{8.binomial_21_0_35_variance|.2f}}.

*Does the in-sample preference survive held out?* No alternative crosses the approximate threshold. Each form was fitted on four groups of runs and scored against
Binomial(21, 0.35) on the fifth:

| Form | Held-out nats over (21, 0.35) ± SE | $z$ |
|---|---|---|
| free distribution | {{8.held_out_free_over_binomial_21_0_35_nats|u+.1f}} ± {{8.held_out_free_over_binomial_21_0_35_se|.1f}} | {{8.held_out_free_over_binomial_21_0_35_z|u+.2f}} |
| binomial, $n$ and $p$ by likelihood | {{8.held_out_binomial_by_likelihood_over_binomial_21_0_35_nats|u+.1f}} ± {{8.held_out_binomial_by_likelihood_over_binomial_21_0_35_se|.1f}} | {{8.held_out_binomial_by_likelihood_over_binomial_21_0_35_z|u+.2f}} |
| binomial by moments | {{8.held_out_binomial_by_moments_over_binomial_21_0_35_nats|u+.1f}} ± {{8.held_out_binomial_by_moments_over_binomial_21_0_35_se|.1f}} | {{8.held_out_binomial_by_moments_over_binomial_21_0_35_z|u+.2f}} |
| Conway-Maxwell-Poisson | {{8.held_out_conway_maxwell_poisson_over_binomial_21_0_35_nats|u+.1f}} ± {{8.held_out_conway_maxwell_poisson_over_binomial_21_0_35_se|.1f}} | {{8.held_out_conway_maxwell_poisson_over_binomial_21_0_35_z|u+.2f}} |
| Poisson | {{8.held_out_poisson_over_binomial_21_0_35_nats|u+.1f}} ± {{8.held_out_poisson_over_binomial_21_0_35_se|.1f}} | {{8.held_out_poisson_over_binomial_21_0_35_z|u+.2f}} |

No alternative has a clear held-out advantage under the approximate comparison, and the Poisson scores worse. Keeping (21, 0.35) is a decision rather than
a fit. In-sample the candidate Binomial(18, 0.405) fits {{8.candidate_binomial_18_0_405_over_binomial_21_0_35_in_sample_nats|.1f}} nats better. It lies inside the profile interval, with
mean {{8.candidate_binomial_18_0_405_mean|.2f}} and variance {{8.candidate_binomial_18_0_405_variance|.2f}}.

*Does $N$ depend on the deck or trainee?* For two, yes. The model has one $N$ for every deck and trainee, so it
under-predicts the first and over-predicts the second, both known misses:

| Group | Runs | Mean $N$ | Mean $N$, other runs | $p$ at $n = 21$ | Same, other runs | Likelihood-ratio $p$ |
|---|---|---|---|---|---|---|
| decks with Sasami Anshinzawa [This Might Sting!]'s pal card | {{8.sasami_anshinzawa_s_card_runs|,.0f}} | {{8.sasami_anshinzawa_s_card_mean_of_n|.2f}} | {{8.sasami_anshinzawa_s_card_mean_of_n_the_other_runs|.2f}} | {{8.sasami_anshinzawa_s_card_p_at_n_21|.3f}} | {{8.sasami_anshinzawa_s_card_p_at_n_21_the_other_runs|.3f}} | {{8.sasami_anshinzawa_s_card_p_at_n_21_on_each_side_lr_p|.0e}} |
| Inari One's careers | {{8.inari_one_runs|,.0f}} | {{8.inari_one_mean_of_n|.2f}} | {{8.inari_one_mean_of_n_the_other_runs|.2f}} | {{8.inari_one_p_at_n_21|.3f}} | {{8.inari_one_p_at_n_21_the_other_runs|.3f}} | {{8.inari_one_p_at_n_21_on_each_side_lr_p|.3f}} |

On the runs with neither, Binomial(21, 0.35) is still rejected against a free distribution, $p$ = {{8.binomial_21_0_35_lr_p_runs_with_neither|.3f}}.

*Scenarios and pals.* A free distribution per scenario gains {{8.per_scenario_log_likelihood_gain|.1f}} nats for {{8.per_scenario_extra_parameters|.0f}} extra parameters, below the nominal threshold.
The fitted means by pal count do not decrease monotonically. Deck composition and weakly observed counts beyond
short decks' completion prevent this from identifying whether pal events consume chain budget. Over {{8.per_pal_count_runs_1_4_pals|,.0f}} runs with 1-4 pals:

| | Unity Cup | Grand Concert | URA Finale | Trackblazer | | 1 pal | 2 pals | 3 pals | 4 pals |
|---|---|---|---|---|---|---|---|---|---|
| Mean $N$ | {{8.per_scenario_mean_unity_cup|.2f}} | {{8.per_scenario_mean_grand_concert|.2f}} | {{8.per_scenario_mean_ura_finale|.2f}} | {{8.per_scenario_mean_trackblazer|.2f}} | | {{8.per_pal_count_mean_1_pals|.2f}} | {{8.per_pal_count_mean_2_pals|.2f}} | {{8.per_pal_count_mean_3_pals|.2f}} | {{8.per_pal_count_mean_4_pals|.2f}} |

*Final links by pal count.* These comparisons also use latent reach without the marker allowance. The binomial lies
inside the observed 95% interval at 3, 1 and 0 pals. It lies above it at
5 and 2 pals and below it at 4.

| Pals | Chain cards | Runs | Final link reached [95%] | Binomial(21, 0.35) | Poisson(7.1) |
|---|---|---|---|---|---|
| 5 | 1 | {{8.final_link_table_5_pals_runs|,.0f}} | {{8.final_link_table_5_pals_observed|.3f}} [{{8.final_link_table_5_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_5_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_5_pals_binomial_21_0_35|.3f}} | {{8.final_link_table_5_pals_poisson_7_1|.3f}} |
| 4 | 2 | {{8.final_link_table_4_pals_runs|,.0f}} | {{8.final_link_table_4_pals_observed|.3f}} [{{8.final_link_table_4_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_4_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_4_pals_binomial_21_0_35|.3f}} | {{8.final_link_table_4_pals_poisson_7_1|.3f}} |
| 3 | 3 | {{8.final_link_table_3_pals_runs|,.0f}} | {{8.final_link_table_3_pals_observed|.3f}} [{{8.final_link_table_3_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_3_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_3_pals_binomial_21_0_35|.3f}} | {{8.final_link_table_3_pals_poisson_7_1|.3f}} |
| 2 | 4 | {{8.final_link_table_2_pals_runs|,.0f}} | {{8.final_link_table_2_pals_observed|.3f}} [{{8.final_link_table_2_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_2_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_2_pals_binomial_21_0_35|.3f}} | {{8.final_link_table_2_pals_poisson_7_1|.3f}} |
| 1 | 5 | {{8.final_link_table_1_pals_runs|,.0f}} | {{8.final_link_table_1_pals_observed|.3f}} [{{8.final_link_table_1_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_1_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_1_pals_binomial_21_0_35|.3f}} | {{8.final_link_table_1_pals_poisson_7_1|.3f}} |
| 0 | 6 | {{8.final_link_table_0_pals_runs|,.0f}} | {{8.final_link_table_0_pals_observed|.3f}} [{{8.final_link_table_0_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_0_pals_observed_95_pct_high|.3f}}] | {{8.final_link_table_0_pals_binomial_21_0_35|.3f}} | {{8.final_link_table_0_pals_poisson_7_1|.3f}} |

*Slots or cards?* The slot rule has the higher sum of marginal link scores. The alternative gives each running card's next event the same chance. The slot rule scores
{{8.allocation_slot_over_card_nats|.1f}} nats better over {{8.allocation_runs|,.0f}} runs. With no pals it lies inside the observed interval and the card rule outside.
With two pals both predict more final links than seen, the card rule further.

| | Observed | Uniform over remaining slots, adopted | Uniform over cards |
|---|---|---|---|
| Total LL | | {{8.allocation_total_ll_slot_rule|u,.1f}} | {{8.allocation_total_ll_card_rule|u,.1f}} |
| Final links, 2 pals | {{8.allocation_final_links_2_pals_observed|.2f}} | {{8.allocation_final_links_2_pals_slot_rule|.2f}} | {{8.allocation_final_links_2_pals_card_rule|.2f}} |
| Final links, 0 pals | {{8.allocation_final_links_0_pals_observed|.2f}} | {{8.allocation_final_links_0_pals_slot_rule|.2f}} | {{8.allocation_final_links_0_pals_card_rule|.2f}} |

*Two-chain decks.* The prediction below is latent completion, while the observations require both markers to show.
The marker allowance must also be applied before this is a calibration check.
Decks with two 3-link SSRs and four pals need $N \ge 6$ to finish both chains:

| Runs | Both chains finished | Share [95%] | Predicted |
|---|---|---|---|
| {{8.two_chain_decks_runs|.0f}} | {{8.two_chain_decks_both_chains_finish|.0f}} | {{8.two_chain_decks_share|.2f}} [{{8.two_chain_decks_95_pct_low|.2f}}, {{8.two_chain_decks_95_pct_high|.2f}}] | {{8.two_chain_decks_predicted_p_n_ge_6|.2f}} |

*Known miss: early links in crowded decks run short.* An early link is one before a chain's final link. The shortfall
is the share of early links the model expects to fire minus the share seen, ± its run-clustered standard error. $z$
sums each run's links as one cluster: $z = \sum(\text{pred} - \text{obs}) / \sqrt{\sum(\text{pred} - \text{obs})^2}$.

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Early-link shortfall | {{8.early_link_shortfall_4_pals|.2f}} ± {{8.early_link_shortfall_4_pals_pm|.2f}} | {{8.early_link_shortfall_3_pals|.2f}} ± {{8.early_link_shortfall_3_pals_pm|.2f}} | {{8.early_link_shortfall_2_pals|.2f}} ± {{8.early_link_shortfall_2_pals_pm|.2f}} | {{8.early_link_shortfall_1_pals|.2f}} ± {{8.early_link_shortfall_1_pals_pm|.2f}} | {{8.early_link_shortfall_0_pals|.2f}} ± {{8.early_link_shortfall_0_pals_pm|.2f}} |
| $z$ | {{8.early_link_shortfall_4_pals_z|.1f}} | {{8.early_link_shortfall_3_pals_z|.1f}} | {{8.early_link_shortfall_2_pals_z|.1f}} | {{8.early_link_shortfall_1_pals_z|.1f}} | {{8.early_link_shortfall_0_pals_z|.1f}} |

*Known miss: individual cards' final links.* Pooled, final links fit: {{8.final_links_by_card_pooled_observed|.3f}} observed against {{8.final_links_by_card_pooled_model|.3f}}. Card by card they
do not. The test covers the {{8.final_links_by_card_cards|.0f}} cards with 60 or more final-link observations, clustered by run since one run's
chain cards share its $N$. It gives $\chi^2$ = {{8.final_links_by_card_chi2|.0f}}, where chance alone gives about {{8.final_links_by_card_cards|.0f}}. The cause is not known.

*Known miss: chains that never start.* More chains never start in crowded decks than predicted:

| Chain cards | Never started, observed | Model |
|---|---|---|
| 5 | {{8.never_started_5_chain_cards_observed|.2f}} | {{8.never_started_5_chain_cards_model|.2f}} |
| 6 | {{8.never_started_6_chain_cards_observed|.2f}} | {{8.never_started_6_chain_cards_model|.2f}} |

A per-card activation parameter remains unresolved. The disjoint coarse grids for activation and its null give
{{8.activation_corrected_reading_joint_with_n_gain_nats|u.1f}} nats at $a = {{8.activation_corrected_reading_joint_with_n_a|.2f}}$.
This is not a nested maximised comparison and cannot reject activation.

## 9. Event choices and outcomes

Computed by `research/s09_outcomes.py` (the certainty checks: `research/s13_calibration.py`).

Two rules are **assumed** (`docs/MODEL.md`). IT takes an event's top choice unless a prioritised skill sits on another
choice, and the player prioritises every target. A choice's outcomes are equally likely unless measured; master.mdb
holds no outcome odds.

A target counts as *prioritised* when it is on the run's priority list and not outranked. It is outranked when a
higher-ranked prioritised skill sits on a rival choice of every event offering it. That affects {{9.event_only_hints_outranked_on_a_rival_choice_rows|.0f}} event-only rows;
which way the rank runs is assumed, not tested. The classifier does not compare each choice's best-ranked skill,
so a higher-ranked companion on the target's own choice can invalidate its outranked label.

### Findings

- **IT follows the choice rule** in the two events where priority lists show the choice taken.
- **The rule's consequence shows on real careers.** Event-only hints of targets not prioritised read {{9.event_only_hints_non_prioritised_targets_obs_per_pred|.3f}} of the
  model, far below prioritised ones.
- **Known miss: event-only hints of prioritised targets read just below the model**, at {{9.event_only_hints_prioritised_targets_obs_per_pred|.3f}}.
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
| Sasami Anshinzawa [This Might Sting!] finale, Nothing Ventured, Risky Business prioritised | {{9.anshinzawa_nv_with_rb_prioritised|.0f}}/{{9.anshinzawa_rb_prioritised_runs|.0f}} | {{9.anshinzawa_nv_share|.2f}} [{{9.anshinzawa_nv_share_90_pct_low|.2f}}, {{9.anshinzawa_nv_share_90_pct_high|.2f}}] | {{9.adopted_anshinzawa_finale_nothing_ventured|.2f}} |
| same, choice 2 alone, no inherited Risky Business | {{9.anshinzawa_choice_2_no_inherited_rb_won|.0f}}/{{9.anshinzawa_choice_2_no_inherited_rb_runs|.0f}} | {{9.anshinzawa_choice_2_share|.2f}} [{{9.anshinzawa_choice_2_90_pct_low|.2f}}, {{9.anshinzawa_choice_2_90_pct_high|.2f}}] | |
| Riko Kashimoto [Planned Perfection] "Unexpected Side", Ramp Up, top choice taken | {{9.riko_ramp_up_top_choice_taken|.0f}}/{{9.riko_readings|.0f}} | {{9.riko_share|.2f}} [{{9.riko_90_pct_low|.2f}}, {{9.riko_90_pct_high|.2f}}] | {{9.adopted_riko_unexpected_side_ramp_up|.2f}} |
| Seiun Sky [Foolproof Plan] link 3, Fast-Paced | {{9.seiun_sky_link_3_fast_paced|.0f}}/{{9.seiun_sky_link_3_readings|.0f}} | {{9.seiun_sky_share|.2f}} [{{9.seiun_sky_90_pct_low|.2f}}, {{9.seiun_sky_90_pct_high|.2f}}] | {{9.adopted_seiun_sky_link_3_fast_paced|.2f}} |
| Smart Falcon [My Umadol Way! ☆] link 3, Center Stage | {{9.smart_falcon_link_3_center_stage|.0f}}/{{9.smart_falcon_link_3_readings|.0f}} | {{9.smart_falcon_share|.2f}} [{{9.smart_falcon_90_pct_low|.2f}}, {{9.smart_falcon_90_pct_high|.2f}}] | {{9.adopted_smart_falcon_link_3_center_stage|.2f}} |
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante", Nothing Ventured | {{9.festa_nv_wins|.0f}}/{{9.festa_readings|.0f}} | {{9.festa_share|.2f}} [{{9.festa_share_90_pct_low|.2f}}, {{9.festa_share_90_pct_high|.2f}}] | {{9.festa_adopted|.2f}} |

The other outcomes: Riko Kashimoto's alternative is Maverick ○, Seiun Sky's Escape Artist, Smart Falcon's Prudent
Positioning. Sasami Anshinzawa's rows use the {{9.anshinzawa_runs_with_the_card_no_festa|.0f}} runs with her card and without Nakayama Festa's; Nakayama Festa's
start from {{9.festa_runs_with_the_card|.0f}} runs with her card, then exclude her own careers and decks with Sasami Anshinzawa's SSR.
The split-choice, Sasami Anshinzawa and Daitaku Helios analyses use the earlier collection subset, not the full pool.

### Evidence

*Does IT follow the choice rule?* Where it can be seen, yes:

| Event | Condition | Choice taken |
|---|---|---|
| Sasami Anshinzawa's finale | Risky Business prioritised | choice 2 in about {{9.anshinzawa_rb_prioritised_took_choice_2|.0%}} of runs |
| Sasami Anshinzawa's finale | neither skill prioritised | choice 2 in {{9.anshinzawa_neither_prioritised_choice_2_won|.0f}} of {{9.anshinzawa_neither_prioritised_runs|.0f}} runs |
| Riko Kashimoto [Planned Perfection]'s event | Lone Wolf prioritised | bottom choice in {{9.riko_lone_wolf_prioritised_bottom_choice|.0f}} of {{9.riko_lone_wolf_prioritised_runs|.0f}} runs |

Of the {{9.anshinzawa_choice_1_exceptions_rb_prioritised|.0f}} exceptions in the first row, {{9.anshinzawa_choice_1_exceptions_with_rb_at_lv_5|.0f}} had Risky Business already at hint Lv 5. Every run has a priority
list, but the records show it for only {{9.runs_showing_a_priority_list|.0f}} of {{9.runs|,.0f}} runs. The rest count as unknown, never as an empty list.
So the rule is tested mainly through its consequences in section 13. There the prioritised rows read just below the
model, a known miss. Assuming every target prioritised over-predicts the rest by far more.

*Reading Sasami Anshinzawa's finale.* Nakayama Festa [Trusting the Dice]'s "Upping the Ante" also gives Nothing
Ventured and is credited jointly with it, so runs with Festa's card are left out. Hint levels identify the choice:

- her Nothing Ventured +3 is choice 1, and +1 is choice 2;
- a loss on choice 2 shows as her Risky Business at level 2, from storyline choice 3 and the finale.

With inherited Risky Business levels mixed in, choice 2 reads {{9.anshinzawa_choice_2_with_inherited_low|.2f}}-{{9.anshinzawa_choice_2_with_inherited_high|.2f}}. Choice 1 gives Nothing Ventured +3 or nothing.
If the finale always fires, choice 1 won in {{9.anshinzawa_choice_1_won|.0f}} of {{9.anshinzawa_choice_1_runs|.0f}} runs. Grand Concert reads lower, unexplained.

*IT keeps her dates.* With Risky Business prioritised, IT also takes the bottom choice of her date-start event "A
Pleasant Practice Partner?", which gives Risky Business +1. The extra +1 shows that it does:

| Choice-2 wins at her finale | With the extra Risky Business +1 |
|---|---|
| {{9.anshinzawa_choice_2_wins_prioritised|.0f}}, Risky Business prioritised | {{9.anshinzawa_her_extra_rb_in_choice_2_wins_prioritised|.0f}} |
| {{9.anshinzawa_choice_2_wins_rb_not_prioritised|.0f}}, not prioritised | {{9.anshinzawa_her_extra_rb_in_choice_2_wins_rb_not_prioritised|.0f}} |

GameTora marks that choice as ending her event chain, which for a pal card means her dates. Yet the finale still
fires: at least {{9.anshinzawa_rb_prioritised_runs_showing_the_finale|.0f}} of {{9.anshinzawa_rb_prioritised_runs|.0f}} such runs show its outcome. IT apparently keeps her dates either way.

*Why Riko Kashimoto's adopted share is lower than measured.* The adopted value is a per-run chance of her Ramp Up
hint, whichever choice IT took. Transferring it to a policy that prioritises Ramp Up is an assumption:

| Riko Kashimoto [Planned Perfection] | Count | Share [90%] |
|---|---|---|
| runs with her Ramp Up hint | {{9.riko_per_run_runs_with_ramp_up|.0f}} of {{9.riko_per_run_runs|.0f}} | {{9.riko_per_run_share|.3f}} [{{9.riko_per_run_90_pct_low|.3f}}, {{9.riko_per_run_90_pct_high|.3f}}] |
| firings where IT took the top choice | {{9.riko_firings_top_choice|.0f}} of {{9.riko_firings_all|.0f}} | {{9.riko_firings_top_choice_rate|.2f}} |
| firings where IT took the bottom choice, Lone Wolf | {{9.riko_firings_bottom_choice|.0f}} | |

*Reading Nakayama Festa's share.* The following credits are from the eligible subset of the
{{9.festa_runs_with_the_card|.0f}} runs with her card:

| Credit | Runs |
|---|---|
| to her alone, Nothing Ventured | {{9.festa_nv_wins|.0f}} |
| to her alone, Risky Business | {{9.festa_rb_wins|.0f}} |
| Risky Business, to "her or another card's event" | {{9.festa_rb_shared_credits|.0f}}, Team Sirius in {{9.festa_shared_with_team_sirius|.0f}} |
| neither outcome, with Risky Business already at Lv 5 | {{9.festa_blank_risky_business_at_lv_5|.0f}} |

A shared credit is split by estimated source rates. Let $r_F$ be her exclusive Risky Business credit rate in runs with
her card, {{9.festa_her_rb_rate|.3f}}. Let $r_o$ be the rate at which the other card alone is credited with it in runs without her:
{{9.festa_team_sirius_alone_rb_rate|.3f}} for Team Sirius, {{9.festa_anshinzawa_alone_rb_rate|.3f}} for Sasami Anshinzawa. The estimator assigns a shared credit to her with weight
$r_F / (1 - (1 - r_F)(1 - r_o))$, which gives her {{9.festa_shared_credits_hers|.1f}} of them.

In the blank runs a Risky Business win cannot show, or the event never fired. They are split by the rate at which the
event shows where both outcomes can, {{9.festa_firing_rate_where_both_can_show|.2f}}. That gives the share in the measured-shares table. Whatever share of
the shared credits is hers, it stays within {{9.festa_share_shared_all_hers|.3f}}-{{9.festa_share_shared_none_hers|.3f}} under the same blank-case correction.
This attribution assumes independent source firings but substitutes exclusive credits for marginal firing rates.
The blank-case correction does not condition on the missing gold outcome. Both corrections need a joint observation
model, and no final-Wit test is computed here.

*Shares left at the equal rule.*

| Event, outcome | Share | Verdict |
|---|---|---|
| Tazuna Hayakawa SSR finale, Concentration | {{9.tazuna_finale_concentration|.2f}} | off 0.5, left at the equal rule: known miss |
| Agnes Tachyon [Q≠0] link 1 choice 2, Murmur | {{9.tachyon_link_1_choice_2_murmur|.2f}} | off 0.5, left: known miss |
| Marvelous Sunday [Dazzling Day in the Snow] link 3, Position Pilfer | {{9.marvelous_sunday_link_3_position_pilfer|.2f}} | off 0.5, left: known miss |
| Light Hello SSR finale, +3 See Ya Later! | {{9.light_hello_finale_share|.2f}}: {{9.light_hello_finale_see_ya_later_3|,.0f}} of {{9.light_hello_finale_readings|,.0f}} | off 0.5, left: known miss |
| Gold Ship [That Time I Became the Strongest] link 3 | {{9.gold_ship_link_3_innate_experience|.2f}} | consistent with 0.5 |
| King Halo [Tonight, We Waltz] link 3 | {{9.king_halo_link_3_gap_closer|.2f}} | consistent with 0.5 |
| Daitaku Helios [Sunlit Angel ♪] link 2, Tail Held High, where link 3 visibly fired | {{9.helios_link_2_tail_held_high_given_link_3|.2f}} | consistent with 0.5 |

*Pal and group finales are assumed certain.* Light Hello [From the Ground Up] credits See Ya Later! to her event in {{9.light_hello_credited_credited|.0f}} of
{{9.light_hello_credited_runs_carrying_her|.0f}} runs carrying her. Team Sirius is credited with Risky Business, alone or shared, in {{9.team_sirius_credited_credited|.0f}} of {{9.team_sirius_credited_runs_carrying_her|.0f}}.

## 10. Random card events (not modelled)

Computed by `research/s10_random_events.py`.

A deck card's character has *shared random events* that can fire in another uma's career; any rarity counts, R cards
included. $K$ is the number on offer: every shared event of the deck's characters plus the trainee's own. A *reading*
is one such event in one run, read through the marker skill on its top choice.

### Findings

- **A shared random event fires in about {{10.constant_p|.2f}} of the careers it can fire in** (**fitted**). This is a lower bound.
  The rate falls a little as more events are on offer, but a constant rate predicts unseen runs as well.
- **Leaving them out under-predicts the targets they can hint**, which read {{10.calibration_obs_per_pred_left_out|.2f}} of the model.
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
| readings | {{10.readings|,.0f}}, from {{10.runs|,.0f}} runs |
| clean hits | {{10.clean_hits|,.0f}} |
| ambiguous credits, left out | {{10.ambiguous_credits_unknown_left_out|.0f}} |
| readings where the marker was already at Lv 5 from other sources | {{10.readings_where_the_event_could_not_show_marker_at_lv_5_from_other_sources|.0f}} |

| Way of counting | Rate |
|---|---|
| as fitted | {{10.constant_p|.3f}} |
| ambiguous credits as hits | about {{10.rate_with_ambiguous_points_higher|.0f}} points higher |
| only where the event could show | {{10.rate_where_the_event_could_show|.3f}} |
| the same, with ambiguous credits as hits | {{10.rate_where_it_could_show_ambiguous_as_hits|.3f}} |

A firing is also missed when IT takes another choice for a prioritised skill. So the rate is a lower bound.

*Which form?* None beats the constant rate held out:

| Form | LL in-sample | Held out, against the form named |
|---|---|---|
| constant {{10.constant_p|.3f}} | {{10.ll_constant|u,.1f}} | |
| $K$ form, ${{10.fitted_form_p_at_k_4|.2f}}(1 - {{10.fitted_form_1_ratio_per_event|.3f}})^{K-4}$ | {{10.ll_fitted_form|u,.1f}} | {{10.held_out_fitted_form_against_constant_nats|u+.1f}} ± {{10.held_out_fitted_form_against_constant_se|.1f}} nats against the constant, $z$ = {{10.held_out_fitted_form_against_constant_z|u+.2f}} |
| power law $c(K/8)^{-a}$, $a = {{10.power_law_a|.2f}}$ | {{10.ll_power_law|u,.1f}} | {{10.held_out_power_law_against_fitted_form_nats|u+.1f}} ± {{10.held_out_power_law_against_fitted_form_se|.1f}} against the $K$ form, $z$ = {{10.held_out_power_law_against_fitted_form_z|u+.2f}} |
| strict competition $1 - (1 - 1/K)^S$, $S = {{10.strict_competition_s|.2f}}$ | {{10.ll_strict_competition|u,.1f}} | {{10.held_out_strict_competition_against_fitted_form_nats|u+.1f}} ± {{10.held_out_strict_competition_against_fitted_form_se|.1f}} against the $K$ form, $z$ = {{10.held_out_strict_competition_against_fitted_form_z|u+.2f}} |
| slot form $c / T$, $c = {{10.every_slot_competing_c|.2f}}$ | {{10.ll_every_slot_competing|u,.1f}} | {{10.held_out_every_slot_competing_against_fitted_form_nats|u+.1f}} ± {{10.held_out_every_slot_competing_against_fitted_form_se|.1f}} against the $K$ form, $z$ = {{10.held_out_every_slot_competing_against_fitted_form_z|u+.2f}} |

The $K$ form says each extra event on offer lowers the rate by a fixed fraction. It gives {{10.fitted_p_at_k_4|.2f}} at $K = 4$ and {{10.fitted_p_at_k_14|.2f}} at
$K = 14$. Its slope is detectable, run-clustered Wald $|z|$ = {{10.slope_in_k_robust_z|u.2f}}, but does not predict unseen runs better. Uniform
competition would make the power law's $a = 1$. Strict competition predicts unseen runs worse.

The slot form lets every event slot compete: $T = K + w \cdot \text{pals}$, $w$ being what a pal weighs. Chain links
take no weight, and pals do compete:

| Slot weight | Estimate [95%, run-clustered] | Test |
|---|---|---|
| a chain link | {{10.every_slot_competing_chain_link_weight|.0f}} | weight 1 rejected, score test |
| a pal, $w$ | {{10.every_slot_competing_pal_weight|.2f}} [{{10.every_slot_competing_pal_weight_robust_95_pct_low|.2f}}, {{10.every_slot_competing_pal_weight_robust_95_pct_high|.2f}}] | 0 excluded |

*What else moves the rate?* The measured effects are below. The within-run check does not detect dependence;
independent firing remains an assumption.

| $K$ | 4 | 6 | 8 | 10 | 12 | 14 |
|---|---|---|---|---|---|---|
| Observed hit rate | {{10.hit_rate_k_4|.2f}} | {{10.hit_rate_k_6|.2f}} | {{10.hit_rate_k_8|.2f}} | {{10.hit_rate_k_10|.2f}} | {{10.hit_rate_k_12|.2f}} | {{10.hit_rate_k_14|.2f}} |

A logistic slope in $K$ gives {{10.logistic_rate_at_k_8|.1%}} at 8 on offer and {{10.logistic_rate_at_k_14|.1%}} at 14. The card effects were fitted together next to $K$:

| Effect, per 10 points | Logit [95%] | Holm-adjusted run-clustered $p$ | Verdict |
|---|---|---|---|
| initial friendship gauge | {{10.initial_friendship_gauge_logit_per_10_points|+.3f}} [{{10.initial_friendship_gauge_95_pct_low|+.3f}}, {{10.initial_friendship_gauge_95_pct_high|+.3f}}] | {{10.initial_friendship_gauge_holm_p|.4f}} | raises the rate; {{10.initial_friendship_gauge_cards_at_limit_break_4_logit_per_10_points|+.3f}} on LB4 cards alone |
| friendship bonus | {{10.friendship_bonus_logit_per_10_points|u+.3f}} | {{10.friendship_bonus_holm_p|.2f}} | none |
| specialty priority | {{10.specialty_priority_logit_per_10_points|+.3f}} | {{10.specialty_priority_holm_p|.2f}} | none |

The model leaves these events out, so the gauge changes no prediction; it is a known miss of the measured rate. Rarity
within a character shows no effect: pooled, SSR markers fire at {{10.pooled_ssr_marker_rate|.1%}} and SR at {{10.pooled_sr_marker_rate|.1%}}. Scenario is not detectable,
run-clustered $p$ = {{10.scenario_robust_p|.2f}}. A rate per scenario does not predict unseen runs better, $z$ = {{10.held_out_scenario_next_to_k_z|u.2f}}.

*Bottom-choice skills are rarely credited.* In the {{10.runs_showing_a_priority_list|.0f}} runs showing a priority list:

| Bottom-choice candidates | Count or rate [95%] |
|---|---|
| prioritised | {{10.bottom_choice_markers_prioritised_there|.0f}} of {{10.bottom_choice_candidates_there|.0f}} |
| credited, not prioritised | {{10.bottom_choice_not_prioritised_credited_rate|.3f}} [{{10.bottom_choice_not_prioritised_95_pct_low|.3f}}, {{10.bottom_choice_not_prioritised_95_pct_high|.3f}}] |
| credited, prioritised | {{10.bottom_choice_prioritised_credited_to_her_event|.0f}} of {{10.bottom_choice_markers_prioritised_there|.0f}} |
| top-choice markers credited, for comparison | {{10.top_choice_marker_rate|.3f}} |

*What does leaving them out cost on real careers?* The test uses section 13's rows, each on the run's own deck. It
scores whether the target was acquired from any source, so it does not depend on reading the event.

| Rows where a deck card's random event could hint the target | Count |
|---|---|
| all | {{10.calibration_rows|.0f}} |
| target credited to that event | {{10.careers_acquired_through_that_event|.0f}} |
| the same, with no other source | {{10.careers_acquired_through_that_event_only|.0f}} |
| rows the model gives no chance | {{10.careers_rows_the_model_gives_no_chance|.0f}} |
| of those, hinted by the event | {{10.careers_of_them_acquired_through_that_event|.0f}} |

| Events | Acquisition, observed / predicted [95%] |
|---|---|
| left out, as modelled | {{10.calibration_obs_per_pred_left_out|.2f}} [{{10.calibration_ratio_95_pct_low_left_out|.2f}}, {{10.calibration_ratio_95_pct_high_left_out|.2f}}] |
| credited at the fitted rate, about {{10.careers_mean_fitted_rate_there|.2f}} there | {{10.calibration_obs_per_pred_credited|.2f}} [{{10.calibration_ratio_95_pct_low_credited|.2f}}, {{10.calibration_ratio_95_pct_high_credited|.2f}}] |

Held out, on the {{10.calibration_rows_with_a_positive_prediction|.0f}} rows with positive baseline predictions and with the rate fitted on the other groups'
readings, crediting gains {{10.held_out_credited_against_left_out_nats|.1f}} ± {{10.held_out_credited_against_left_out_se|.1f}} nats. That is $z$ =
{{10.held_out_credited_against_left_out_z|.2f}}, short of 1.96. Crediting would raise an affected run's odds by a median {{10.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_median|.0%}}, over the targets it can land.
The largest rise is {{10.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_max|.0%}}, over {{10.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_runs|.0f}} affected runs. The trainee's own random event can be told apart only for Nice Nature: it
gave the target on {{10.careers_trainee_s_own_acquired_through_her_events|.0f}} of {{10.careers_rows_of_the_trainee_s_own_event_identifiable|.0f}} rows.

## 11. Conditional character events (not modelled)

Computed by `research/s11_conditional.py`.

The trainee's secret and race-result events are left out of the model. A *conditional-only row* is a run-target row
where only such an event can give the target: none of the trainee's story, date, outfit or shared random events can,
nor the scenario's.

### Findings

- **The conditional-only sample does not settle the cost of omission for individual trainees.** Crediting the measured
  rates on positive-baseline rows gains {{11.calibration_conditional_only_ll_credited_minus_left_out_positive_rows|.1f}} nats.
- **The exception is Vodka's Nimble Navigator.** Her own events give it in most of her runs, and the model
  under-predicts it. Her random and conditional events both give it and cannot be told apart.
- **Tokai Teio's Head-On is modelled as always acquired**, like Yamanin Zephyr's Slipstream. All three of its races
  are her career goals. The goals ask only for top 5, top 5 and top 3, but IT won all three in {{11.of_them_winning_all_three|.0f}} of {{11.runs_training_tokai_teio|.0f}}
  runs training her. Every run where her event could show credits Head-On. Certainty remains an assumption; her goals
  require placings, while the event requires wins.

### Evidence

| Rows | Her events gave the target | Rate [95%] |
|---|---|---|
| all conditional-only rows | {{11.careers_conditional_only_from_her_events|.0f}} of {{11.careers_conditional_only_rows|.0f}} | {{11.careers_conditional_only_rate|.3f}} [{{11.careers_conditional_only_rate_95_pct_low|.3f}}, {{11.careers_conditional_only_rate_95_pct_high|.3f}}] |
| Daiwa Scarlet, any outfit, Nimble Navigator | {{11.daiwa_scarlet_any_outfit_nimble_navigator_from_her_events|.0f}} of {{11.daiwa_scarlet_any_outfit_nimble_navigator_rows|.0f}} | [{{11.daiwa_scarlet_any_outfit_nimble_navigator_rate_95_pct_low|.2f}}, {{11.daiwa_scarlet_any_outfit_nimble_navigator_rate_95_pct_high|.2f}}] |
| Mejiro Ardan, any outfit, Ramp Up | {{11.mejiro_ardan_any_outfit_ramp_up_from_her_events|.0f}} of {{11.mejiro_ardan_any_outfit_ramp_up_rows|.0f}} | [{{11.mejiro_ardan_any_outfit_ramp_up_rate_95_pct_low|.2f}}, {{11.mejiro_ardan_any_outfit_ramp_up_rate_95_pct_high|.2f}}] |
| Vodka, any outfit, Nimble Navigator | {{11.vodka_any_outfit_nimble_navigator_from_her_events|.0f}} of {{11.vodka_any_outfit_nimble_navigator_rows|.0f}} | [{{11.vodka_any_outfit_nimble_navigator_rate_95_pct_low|.2f}}, {{11.vodka_any_outfit_nimble_navigator_rate_95_pct_high|.2f}}] |

| Rows, model as it is | Acquisition, observed / predicted [95%] | Crediting the measured rates gains |
|---|---|---|
| conditional-only | {{11.calibration_conditional_only_obs_per_pred_left_out|.2f}} [{{11.calibration_conditional_only_ratio_95_pct_low_left_out|.2f}}, {{11.calibration_conditional_only_ratio_95_pct_high_left_out|.2f}}] | {{11.calibration_conditional_only_ll_credited_minus_left_out_positive_rows|.1f}} nats |
| with Vodka's rows | {{11.calibration_obs_per_pred_left_out|.2f}} [{{11.calibration_ratio_95_pct_low_left_out|.2f}}, {{11.calibration_ratio_95_pct_high_left_out|.2f}}] | {{11.calibration_ll_credited_minus_left_out_positive_rows|.1f}} nats |

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
  over all rows, {{13.3_acquired_shortfall_share_on_non_prioritised_rows|.0%}}, sits on rows the run's list shows not prioritised.
- **Aggregate inheritance without Tail Held High is consistent with the model**: it reads {{3.inheritance_without_tail_held_high_obs_per_pred|.3f}} of the model.
- **Known miss: Tail Held High is inherited less often than predicted**, the only target of eight whose interval
  excludes 1. The cause is not known.
- **Deck hints read optimistic, with shortfalls concentrated in event sources.** Event hints of targets not prioritised land
  at {{13.1_events_only_not_prioritised_obs_per_pred|.2f}} of the model.
- **Known miss: prioritised targets' event hints read just below the model.**
- **Known miss: Trackblazer's training-hint base over-predicts these runs.** The other scenarios' intervals cover 1.
- **The spark-weighted acquisition check reads {{13.p_all_eight_ratio_all|.2f}} of the exact joint model**: a weak test that does not measure spark generation.
- **Beyond the eight targets**, inheritance reads slightly short of the model, spread over many skills: a known miss.
  Training-pool hints are under-predicted.

### What is compared

Each real career is rebuilt as the model sees one: the trainee in her outfit, the scenario, the six cards at their own
limit breaks, both parents with their recorded parents, every ancestor's white sparks, and race wins giving the
recorded shared-G1 counts. The model gives, for each default target, the chance of each step (sections 3-12). The run
record says what happened.

| Careers | URA Finale | Unity Cup | Grand Concert | Trackblazer | Run-target rows | On her outfit | Scenario golds |
|---|---|---|---|---|---|---|---|
| {{13.runs_used|,.0f}} | {{13.ura_finale_runs|.0f}} | {{13.unity_cup_runs|,.0f}} | {{13.grand_concert_runs|,.0f}} | {{13.trackblazer_runs|.0f}} | {{13.run_target_rows|,.0f}} | {{13.rows_on_her_outfit|,.0f}} | {{13.scenario_gold_rows|,.0f}} |

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
| 1. Deck hint, no inspiration | {{13.1_deck_hint_no_inspiration_rows|,.0f}} | {{13.1_deck_hint_no_inspiration_predicted|.3f}} | {{13.1_deck_hint_no_inspiration_observed|.3f}} [{{13.1_deck_hint_observed_95_pct_low|.3f}}, {{13.1_deck_hint_observed_95_pct_high|.3f}}] | {{13.1_deck_hint_no_inspiration_obs_per_pred|.3f}} [{{13.1_deck_hint_no_inspiration_ratio_95_pct_low|.3f}}, {{13.1_deck_hint_no_inspiration_ratio_95_pct_high|.3f}}] |
| 1. Training pool only | {{13.1_training_pool_only_rows|,.0f}} | {{13.1_training_pool_only_predicted|.3f}} | {{13.1_training_pool_only_observed|.3f}} | {{13.1_training_pool_only_obs_per_pred|.3f}} [{{13.1_training_pool_only_ratio_95_pct_low|.3f}}, {{13.1_training_pool_only_ratio_95_pct_high|.3f}}] |
| 1. Events only | {{13.1_events_only_rows|,.0f}} | {{13.1_events_only_predicted|.3f}} | {{13.1_events_only_observed|.3f}} | {{13.1_events_only_obs_per_pred|.3f}} [{{13.1_events_only_ratio_95_pct_low|.3f}}, {{13.1_events_only_ratio_95_pct_high|.3f}}] |
| 1. Events only, target prioritised | {{13.1_events_only_prioritised_rows|.0f}} | {{13.1_events_only_prioritised_predicted|.3f}} | {{13.1_events_only_prioritised_observed|.3f}} | {{13.1_events_only_prioritised_obs_per_pred|.3f}} [{{13.1_events_only_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_prioritised_ratio_95_pct_high|.4f}}] |
| 1. Events only, not prioritised | {{13.1_events_only_not_prioritised_rows|.0f}} | {{13.1_events_only_not_prioritised_predicted|.3f}} | {{13.1_events_only_not_prioritised_observed|.3f}} | {{13.1_events_only_not_prioritised_obs_per_pred|.3f}} [{{13.1_events_only_not_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_not_prioritised_ratio_95_pct_high|.3f}}] |
| 2. Inheritance | {{13.2_inheritance_rows|,.0f}} | {{13.2_inheritance_predicted|.3f}} | {{13.2_inheritance_observed|.3f}} [{{13.2_inheritance_observed_95_pct_low|.3f}}, {{13.2_inheritance_observed_95_pct_high|.3f}}] | {{13.2_inheritance_obs_per_pred|.3f}} [{{13.2_inheritance_ratio_95_pct_low|.3f}}, {{13.2_inheritance_ratio_95_pct_high|.3f}}] |
| 2. Inheritance, without Tail Held High | {{3.inheritance_without_tail_held_high_rows|,.0f}} | | | {{3.inheritance_without_tail_held_high_obs_per_pred|.3f}} [{{3.inheritance_without_tail_held_high_ratio_95_pct_low|.3f}}, {{3.inheritance_without_tail_held_high_ratio_95_pct_high|.3f}}] |
| 2. Inheritance, Tail Held High | | | | {{13.tail_held_high_inheritance|.3f}} [{{13.tail_held_high_inheritance_95_pct_low|.3f}}, {{13.tail_held_high_inheritance_95_pct_high|.3f}}] |
| 3. Acquired, any source | {{13.3_acquired_any_source_rows|,.0f}} | {{13.3_acquired_any_source_predicted|.3f}} | {{13.3_acquired_any_source_observed|.3f}} [{{13.3_acquired_any_source_observed_95_pct_low|.3f}}, {{13.3_acquired_any_source_observed_95_pct_high|.3f}}] | {{13.3_acquired_any_source_obs_per_pred|.3f}} [{{13.3_acquired_any_source_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_ratio_95_pct_high|.3f}}] |
| 3. Acquired, deck or inspiration only | {{13.3_acquired_deck_or_inspiration_only_rows|,.0f}} | {{13.3_acquired_deck_or_inspiration_only_predicted|.3f}} | {{13.3_acquired_deck_or_inspiration_only_observed|.3f}} | {{13.3_acquired_deck_or_inspiration_only_obs_per_pred|.3f}} [{{13.3_acquired_deck_or_inspiration_only_ratio_95_pct_low|.3f}}, {{13.3_acquired_deck_or_inspiration_only_ratio_95_pct_high|.3f}}] |
| 3. Acquired, any source, prioritised or no list shown | {{13.3_acquired_any_source_prioritised_or_no_list_shown_rows|,.0f}} | {{13.3_acquired_any_source_prioritised_or_no_list_shown_predicted|.3f}} | {{13.3_acquired_any_source_prioritised_or_no_list_shown_observed|.3f}} | {{13.3_acquired_any_source_prioritised_or_no_list_shown_obs_per_pred|.3f}} [{{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_high|.3f}}] |
| 3. Acquired, deck or inspiration only, prioritised or no list shown | {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_rows|,.0f}} | {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_predicted|.3f}} | {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_observed|.3f}} | {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_obs_per_pred|.3f}} [{{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_ratio_95_pct_low|.3f}}, {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_ratio_95_pct_high|.3f}}] |
| 3. Acquired, prioritised | {{13.3_acquired_prioritised_rows|,.0f}} | {{13.3_acquired_prioritised_predicted|.3f}} | {{13.3_acquired_prioritised_observed|.3f}} | {{13.3_acquired_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_prioritised_ratio_95_pct_high|.3f}}] |
| 3. Acquired, not prioritised | {{13.3_acquired_not_prioritised_rows|,.0f}} | {{13.3_acquired_not_prioritised_predicted|.3f}} | {{13.3_acquired_not_prioritised_observed|.3f}} | {{13.3_acquired_not_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_not_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_not_prioritised_ratio_95_pct_high|.3f}}] |
| 4. Landing, sums of $p$ | {{13.4_landing_rows|,.0f}} | {{13.4_landing_sum_of_p|,.1f}} | {{13.4_landing_sum_observed_equivalent|,.1f}} | {{13.4_landing_ratio|.3f}} |

### Evidence

*The priority list matters.* On the runs that show a list, prioritised targets are acquired at the model's rate and
the others less often: the two "3. Acquired" rows split by priority. The pooled test averages the two with the runs
that show no list.

*Where the deck loses its points.* The shortfall is concentrated in event sources: test 1's split.
Unmodelled scenario and trainee events supply the few points the deck loses: {{13.acquisitions_from_unmodelled_events|.0f}} target acquisitions, {{13.acquisitions_from_unmodelled_events_share_of_rows|.1%}} of
rows. On prioritised-or-unknown rows, the deck and inspiration alone read {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_obs_per_pred|.3f}} of the model.
Agreement after adding unmodelled acquisitions can conceal component errors and does not validate candidate rankings.

*Per target*, observed / predicted:

| Target | Deck hint [95%] | Inheritance | Acquired | Landing |
|---|---|---|---|---|
| Uma Stan / Superstan | {{13.uma_stan_deck_hint|.2f}} [{{13.uma_stan_deck_hint_95_pct_low|.2f}}, {{13.uma_stan_deck_hint_95_pct_high|.2f}}] | {{13.uma_stan_inheritance|.3f}} | {{13.uma_stan_acquired|.3f}} | {{13.uma_stan_landing|.3f}} |
| Nimble Navigator / No Stopping Me! | {{13.nimble_navigator_deck_hint|.2f}} [{{13.nimble_navigator_deck_hint_95_pct_low|.2f}}, {{13.nimble_navigator_deck_hint_95_pct_high|.2f}}] | {{13.nimble_navigator_inheritance|.3f}} | {{13.nimble_navigator_acquired|.3f}} | {{13.nimble_navigator_landing|.3f}} |
| Playtime's Over! / See Ya Later! | {{13.playtime_s_over_deck_hint|.3f}} [{{13.playtime_s_over_deck_hint_95_pct_low|.2f}}, {{13.playtime_s_over_deck_hint_95_pct_high|.2f}}] | {{13.playtime_s_over_inheritance|.3f}} | {{13.playtime_s_over_acquired|.3f}} | {{13.playtime_s_over_landing|.3f}} |
| Ramp Up / It's On! | {{13.ramp_up_deck_hint|.3f}} [{{13.ramp_up_deck_hint_95_pct_low|.2f}}, {{13.ramp_up_deck_hint_95_pct_high|.2f}}] | {{13.ramp_up_inheritance|.3f}} | {{13.ramp_up_acquired|.3f}} [{{13.ramp_up_acquired_95_pct_low|.2f}}, {{13.ramp_up_acquired_95_pct_high|.2f}}] | {{13.ramp_up_landing|.3f}} |
| Tail Held High / Tail Nine | {{13.tail_held_high_deck_hint|.3f}} [{{13.tail_held_high_deck_hint_95_pct_low|.2f}}, {{13.tail_held_high_deck_hint_95_pct_high|.2f}}] | {{13.tail_held_high_inheritance|.3f}} [{{13.tail_held_high_inheritance_95_pct_low|.2f}}, {{13.tail_held_high_inheritance_95_pct_high|.2f}}] | {{13.tail_held_high_acquired|.3f}} | {{13.tail_held_high_landing|.3f}} |
| Racing Spirit: Stamina | no deck source | {{13.racing_spirit_stamina_inheritance|.3f}} | {{13.racing_spirit_stamina_acquired|.3f}} | {{13.racing_spirit_stamina_landing|.3f}} |
| Slipstream / Perfect Spot! | {{13.slipstream_deck_hint|.3f}} [{{13.slipstream_deck_hint_95_pct_low|.2f}}, {{13.slipstream_deck_hint_95_pct_high|.2f}}] | {{13.slipstream_inheritance|.3f}} | {{13.slipstream_acquired|.3f}} | {{13.slipstream_landing|.3f}} |
| Risky Business / Nothing Ventured | {{13.risky_business_deck_hint|.3f}} [{{13.risky_business_deck_hint_95_pct_low|.2f}}, {{13.risky_business_deck_hint_95_pct_high|.2f}}] | {{13.risky_business_inheritance|.3f}} | {{13.risky_business_acquired|.3f}} [{{13.risky_business_acquired_95_pct_low|.2f}}, {{13.risky_business_acquired_95_pct_high|.2f}}] | {{13.risky_business_landing|.3f}} |

*Training-pool hints by scenario* test $\beta$ and $d$ (section 7). Only Trackblazer's interval excludes 1: its $\beta$
over-predicts these runs.

| Scenario | Grand Concert | Unity Cup | URA Finale | Trackblazer |
|---|---|---|---|---|
| Observed / predicted [95%] | {{13.1_training_pool_only_grand_concert_obs_per_pred|.3f}} [{{13.1_training_pool_only_grand_concert_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_grand_concert_ratio_95_pct_high|.2f}}] | {{13.1_training_pool_only_unity_cup_obs_per_pred|.3f}} [{{13.1_training_pool_only_unity_cup_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_unity_cup_ratio_95_pct_high|.2f}}] | {{13.1_training_pool_only_ura_finale_obs_per_pred|.3f}} [{{13.1_training_pool_only_ura_finale_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_ura_finale_ratio_95_pct_high|.2f}}] | {{13.1_training_pool_only_trackblazer_obs_per_pred|.3f}} [{{13.1_training_pool_only_trackblazer_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_trackblazer_ratio_95_pct_high|.2f}}] |

By the lowest limit break of the sourcing cards, every interval covers 1:

| Lowest limit break | LB4 | LB3 | LB0-2 |
|---|---|---|---|
| Observed / predicted | {{13.1_training_pool_only_lowest_lb_4|.3f}} | {{13.1_training_pool_only_lowest_lb_3|.3f}} | {{13.1_training_pool_only_lb0_2_lowest_ratio|.2f}}-{{13.1_training_pool_only_lb0_2_highest_ratio|.2f}}, {{13.1_training_pool_only_lb0_2_fewest_rows|.0f}}-{{13.1_training_pool_only_lb0_2_most_rows|.0f}} rows each |

*Unity Cup's guaranteed golds* show in {{13.unity_cup_s_guaranteed_golds_runs_with_both|.0f}} of {{13.unity_cup_runs|.0f}} Unity Cup runs each.

*Gold acquisition*, observed / predicted:

| Gold | Rows | Observed / predicted [95%] |
|---|---|---|
| Nothing Ventured | {{13.gold_acquisition_nothing_ventured_rows|.0f}} | {{13.gold_acquisition_nothing_ventured|.3f}} [{{13.gold_acquisition_nothing_ventured_95_pct_low|.2f}}, {{13.gold_acquisition_nothing_ventured_95_pct_high|.2f}}] |
| Perfect Spot! | {{13.gold_acquisition_perfect_spot_rows|.0f}} | {{13.gold_acquisition_perfect_spot|.3f}} [{{13.gold_acquisition_perfect_spot_95_pct_low|.2f}}, {{13.gold_acquisition_perfect_spot_95_pct_high|.2f}}] |
| It's On!, outside Unity Cup | {{13.gold_acquisition_it_s_on_rows|.0f}} | {{13.gold_acquisition_it_s_on|.3f}} [{{13.gold_acquisition_it_s_on_95_pct_low|.2f}}, {{13.gold_acquisition_it_s_on_95_pct_high|.2f}}] |
| No Stopping Me!, outside Unity Cup | {{13.gold_acquisition_no_stopping_me_rows|.0f}} | {{13.gold_acquisition_no_stopping_me|.3f}} |
| See Ya Later! | {{13.gold_acquisition_see_ya_later_rows|,.0f}} | {{13.gold_acquisition_see_ya_later|.3f}} |

*Reliability.* The rows are grouped by the model's predicted chance. If the model is calibrated, each bucket's observed
rate matches its mean prediction, shown in parentheses:

| Predicted | Deck hint: rows | Observed | Acquired: rows | Observed |
|---|---|---|---|---|
| 0 | {{13.reliability_deck_hint_0_rows|,.0f}} | {{13.reliability_deck_hint_0_observed|.3f}} | {{13.reliability_acquired_0_rows|,.0f}} | {{13.reliability_acquired_0_observed|.3f}} |
| above 0 to 0.20 | {{13.reliability_deck_hint_0_00_0_20_rows|,.0f}} | {{13.reliability_deck_hint_0_00_0_20_observed|.3f}} ({{13.reliability_deck_hint_0_00_0_20_mean_predicted|.2f}}) | {{13.reliability_acquired_0_00_0_20_rows|,.0f}} | {{13.reliability_acquired_0_00_0_20_observed|.3f}} ({{13.reliability_acquired_0_00_0_20_mean_predicted|.2f}}) |
| 0.20-0.40 | {{13.reliability_deck_hint_0_20_0_40_rows|,.0f}} | {{13.reliability_deck_hint_0_20_0_40_observed_sentence|.3f}} [{{13.reliability_deck_hint_0_20_0_40_observed_95_pct_low|.3f}}, {{13.reliability_deck_hint_0_20_0_40_observed_95_pct_high|.3f}}] ({{13.reliability_deck_hint_0_20_0_40_mean_predicted_three_places|.3f}}) | {{13.reliability_acquired_0_20_0_40_rows|,.0f}} | {{13.reliability_acquired_0_20_0_40_observed|.3f}} ({{13.reliability_acquired_0_20_0_40_mean_predicted|.2f}}) |
| 0.40-0.60 | {{13.reliability_deck_hint_0_40_0_60_rows|,.0f}} | {{13.reliability_deck_hint_0_40_0_60_observed|.3f}} ({{13.reliability_deck_hint_0_40_0_60_mean_predicted|.2f}}) | {{13.reliability_acquired_0_40_0_60_rows|,.0f}} | {{13.reliability_acquired_0_40_0_60_observed|.3f}} ({{13.reliability_acquired_0_40_0_60_mean_predicted|.2f}}) |
| 0.60-0.80 | {{13.reliability_deck_hint_0_60_0_80_rows|,.0f}} | {{13.reliability_deck_hint_0_60_0_80_observed|.3f}} ({{13.reliability_deck_hint_0_60_0_80_mean_predicted|.2f}}) | {{13.reliability_acquired_0_60_0_80_rows|,.0f}} | {{13.reliability_acquired_0_60_0_80_observed|.3f}} ({{13.reliability_acquired_0_60_0_80_mean_predicted|.2f}}) |
| 0.80-0.95 | {{13.reliability_deck_hint_0_80_0_95_rows|,.0f}} | {{13.reliability_deck_hint_0_80_0_95_observed|.3f}} ({{13.reliability_deck_hint_0_80_0_95_mean_predicted|.2f}}) | {{13.reliability_acquired_0_80_0_95_rows|,.0f}} | {{13.reliability_acquired_0_80_0_95_observed|.3f}} ({{13.reliability_acquired_0_80_0_95_mean_predicted|.2f}}) |
| 0.95-1 | {{13.reliability_deck_hint_0_95_1_00_rows|,.0f}} | {{13.reliability_deck_hint_0_95_1_00_observed|.3f}} ({{13.reliability_deck_hint_0_95_1_00_mean_predicted|.2f}}) | {{13.reliability_acquired_0_95_1_00_rows|,.0f}} | {{13.reliability_acquired_0_95_1_00_observed|.3f}} ({{13.reliability_acquired_0_95_1_00_mean_predicted|.2f}}) |

The deck's over-prediction sits in its top buckets, the event rewards. The buckets predicted 0.6 and above hold
{{13.deck_shortfall_in_buckets_predicted_0_6_and_above|.0%}} of the shortfall, and {{13.their_rows_with_an_event_source|.0%}} of their rows have an event source. The 0.20-0.40 bucket, almost all
training-pool rows, is within its interval.

*Per run.* A run has {{13.per_run_open_targets|.2f}} open targets on average.

| Per run | Runs | Model | Observed [95%] |
|---|---|---|---|
| targets acquired | all | {{13.per_run_acquired_expected|.3f}} | {{13.per_run_acquired_observed|.3f}} |
| targets acquired | {{13.per_run_priority_assumption_fits_runs|,.0f}} where the priority assumption fits | {{13.per_run_priority_assumption_fits_acquired_expected|.3f}} | {{13.per_run_priority_assumption_fits_acquired_observed|.3f}} |
| all open targets acquired, model as product of odds | all | {{13.per_run_all_open_acquired_product_of_odds|.4f}} | {{13.per_run_all_open_acquired_observed|.4f}} [{{13.per_run_all_open_acquired_95_pct_low|.4f}}, {{13.per_run_all_open_acquired_95_pct_high|.4f}}] |
| all open targets acquired, model as product of odds | where the priority assumption fits | {{13.per_run_priority_assumption_fits_all_open_acquired_product_of_odds|.4f}} | {{13.per_run_priority_assumption_fits_all_open_acquired_observed|.4f}} [{{13.per_run_priority_assumption_fits_all_open_acquired_95_pct_low|.4f}}, {{13.per_run_priority_assumption_fits_all_open_acquired_95_pct_high|.4f}}] |

The table's "priority assumption fits" group includes runs with no list, whose priorities are unknown.
The all-open acquisition predictions multiply the approximate per-target acquisition probabilities.

*Spark-weighted acquisition of all targets.* $P(\text{all eight})$ is recomputed from what each run acquired and compared with the model's exact
$P(\text{all})$. The intervals bootstrap whole runs:

| Scenarios | Observed / predicted [95%] |
|---|---|
| all | {{13.p_all_eight_ratio_all|.3f}} [{{13.p_all_eight_ratio_all_95_pct_low|.2f}}, {{13.p_all_eight_ratio_all_95_pct_high|.2f}}] |
| Unity Cup | {{13.p_all_eight_ratio_unity_cup|.3f}} [{{13.p_all_eight_ratio_unity_cup_95_pct_low|.2f}}, {{13.p_all_eight_ratio_unity_cup_95_pct_high|.2f}}] |
| Grand Concert | {{13.p_all_eight_ratio_grand_concert|.2f}} [{{13.p_all_eight_ratio_grand_concert_95_pct_low|.2f}}, {{13.p_all_eight_ratio_grand_concert_95_pct_high|.2f}}] |
| URA Finale | {{13.p_all_eight_ratio_ura_finale|.2f}} [{{13.p_all_eight_ratio_ura_finale_95_pct_low|.2f}}, {{13.p_all_eight_ratio_ura_finale_95_pct_high|.2f}}] |

Trackblazer's model value is {{13.trackblazer_s_model_p_all|.0f}}. The interval allows up to about {{13.p_all_eight_optimism_the_interval_allows_1_per_95_pct_low|.1f}}× optimism. Only {{13.runs_acquiring_every_open_target|.0f}} runs acquired every
open target, too few to separate the exact joint from the product.

*Single-source cards.* In a single-source pair a card is the deck's only source of the target, so the deck test scores
that card alone. The table lists every pair of 40 or more rows whose interval excludes 1. It adds two whose intervals
reach 1, Gold Ship's and Sweep Tosho's.

| Card > target | Pred | Obs | Ratio [95%] | Rows | Likely reason |
|---|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_predicted|.3f}} | {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_observed|.3f}} | {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio|.2f}} [{{13.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio_95_pct_low|.2f}}, {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio_95_pct_high|.2f}}] | {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_rows|.0f}} | the model assumes Risky Business prioritised, which takes finale choice 2 |
| Riko Kashimoto [Planned Perfection] > Ramp Up | {{13.planned_perfection_riko_kashimoto_gt_ramp_up_predicted|.3f}} | {{13.planned_perfection_riko_kashimoto_gt_ramp_up_observed|.3f}} | {{13.planned_perfection_riko_kashimoto_gt_ramp_up_ratio|.2f}} [{{13.planned_perfection_riko_kashimoto_gt_ramp_up_ratio_95_pct_low|.2f}}, {{13.planned_perfection_riko_kashimoto_gt_ramp_up_ratio_95_pct_high|.2f}}] | {{13.planned_perfection_riko_kashimoto_gt_ramp_up_rows|.0f}} | open; her Lone Wolf choice, when prioritised, accounts for part |
| King Halo [Tonight, We Waltz] > Uma Stan | {{13.tonight_we_waltz_king_halo_gt_uma_stan_predicted|.3f}} | {{13.tonight_we_waltz_king_halo_gt_uma_stan_observed|.3f}} | {{13.tonight_we_waltz_king_halo_gt_uma_stan_ratio|.2f}} [{{13.tonight_we_waltz_king_halo_gt_uma_stan_ratio_95_pct_low|.2f}}, {{13.tonight_we_waltz_king_halo_gt_uma_stan_ratio_95_pct_high|.2f}}] | {{13.tonight_we_waltz_king_halo_gt_uma_stan_rows|.0f}} | Uma Stan sits on a choice that needs a priority |
| Gold Ship [That Time I Became the Strongest] > Uma Stan | {{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_predicted|.3f}} | {{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_observed|.3f}} | {{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_ratio|.2f}} [{{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_ratio_95_pct_low|.2f}}, {{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_ratio_95_pct_high|.2f}}] | {{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_rows|.0f}} | open |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_predicted|.3f}} | {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_observed|.3f}} | {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio|.2f}} [{{13.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio_95_pct_low|.2f}}, {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio_95_pct_high|.2f}}] | {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_rows|.0f}} | open |
| Maruzensky [Sentimental Flare ♪] > Slipstream | {{13.sentimental_flare_maruzensky_gt_slipstream_predicted|.3f}} | {{13.sentimental_flare_maruzensky_gt_slipstream_observed|.3f}} | {{13.sentimental_flare_maruzensky_gt_slipstream_ratio|.2f}} [{{13.sentimental_flare_maruzensky_gt_slipstream_ratio_95_pct_low|.2f}}, {{13.sentimental_flare_maruzensky_gt_slipstream_ratio_95_pct_high|.2f}}] | {{13.sentimental_flare_maruzensky_gt_slipstream_rows|.0f}} | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_predicted|.3f}} | {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_observed|.3f}} | {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio|.2f}} [{{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio_95_pct_low|.2f}}, {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio_95_pct_high|.2f}}] | {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_rows|.0f}} | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | {{13.sunlit_angel_daitaku_helios_gt_slipstream_predicted|.3f}} | {{13.sunlit_angel_daitaku_helios_gt_slipstream_observed|.3f}} | {{13.sunlit_angel_daitaku_helios_gt_slipstream_ratio|.2f}} [{{13.sunlit_angel_daitaku_helios_gt_slipstream_ratio_95_pct_low|.2f}}, {{13.sunlit_angel_daitaku_helios_gt_slipstream_ratio_95_pct_high|.2f}}] | {{13.sunlit_angel_daitaku_helios_gt_slipstream_rows|.0f}} | open |
| Sweep Tosho [Lamplit Training of a Witch-to-Be] > Slipstream | {{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_predicted|.3f}} | {{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_observed|.3f}} | {{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_ratio|.2f}} [{{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_ratio_95_pct_low|.2f}}, {{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_ratio_95_pct_high|.3f}}] | {{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_rows|.0f}} | open |
| Nakayama Festa [Trusting the Dice] > Risky Business | {{13.trusting_the_dice_nakayama_festa_gt_risky_business_predicted|.3f}} | {{13.trusting_the_dice_nakayama_festa_gt_risky_business_observed|.3f}} | {{13.trusting_the_dice_nakayama_festa_gt_risky_business_ratio|.2f}} [{{13.trusting_the_dice_nakayama_festa_gt_risky_business_ratio_95_pct_low|.2f}}, {{13.trusting_the_dice_nakayama_festa_gt_risky_business_ratio_95_pct_high|.2f}}] | {{13.trusting_the_dice_nakayama_festa_gt_risky_business_rows|.0f}} | open |
| Tokai Teio [Step! Smile! Wink!] > Nimble Navigator | {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_predicted|.3f}} | {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_observed|.3f}} | {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_ratio|.2f}} [{{13.step_smile_wink_tokai_teio_gt_nimble_navigator_ratio_95_pct_low|.2f}}, {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_ratio_95_pct_high|.2f}}] | {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_rows|.0f}} | open |

*King Halo's Uma Stan follows the priority.* The expectation and the exact Poisson-binomial tail are the model's:

| Runs that list Uma Stan | Rows | Hinted by her card | Expected | $P(X \le \text{hinted})$ |
|---|---|---|---|---|
| prioritised | {{13.king_halo_gt_uma_stan_prioritised_rows|.0f}} | {{13.king_halo_gt_uma_stan_prioritised_hinted|.0f}} | {{13.king_halo_gt_uma_stan_prioritised_expected|.1f}} | {{13.king_halo_gt_uma_stan_prioritised_p_at_most_the_hinted|.3f}}, consistent |
| a list without it | {{13.king_halo_gt_uma_stan_on_a_list_not_prioritised_rows|.0f}} | {{13.king_halo_gt_uma_stan_on_a_list_not_prioritised_hinted|.0f}} | {{13.king_halo_gt_uma_stan_on_a_list_not_prioritised_expected|.1f}} | |

*Smaller pairs.* Below 40 rows, {{13.pairs_under_40_rows_excluding_1|.0f}} more pairs exclude 1. {{13.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources|.0f}} of them are event sources of Playtime's Over!: Yukino Bijin
[Dancing Light into the Night], Yaeno Muteki [Fiery Discipline] and Sirius Symboli [Escorte Étoile]. They hinted it in
{{13.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources_hinted|.0f}} of {{13.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources_runs|.0f}} runs. The model predicted {{13.pairs_under_40_rows_excluding_1_playtime_s_over_lowest_predicted|.2f}}-{{13.pairs_under_40_rows_excluding_1_playtime_s_over_highest_predicted|.2f}}. Their event puts the skill on choice 2, which ends the chain
before its last link, so IT takes it only when the skill is prioritised. That cause is untested here:

| Those runs | Count | Skill prioritised | Skill hinted |
|---|---|---|---|
| showing a priority list | {{13.playtime_s_over_event_sources_runs_showing_a_priority_list|.0f}} | {{13.playtime_s_over_event_sources_prioritised|.0f}} | {{13.playtime_s_over_event_sources_hinted_with_a_list_shown|.0f}} |
| without a list | {{13.playtime_s_over_event_sources_runs_without_a_list|.0f}} | | {{13.playtime_s_over_event_sources_hinted_without_a_list|.0f}} |

*Other skills.* Every non-target skill a deck card or spark could give, on a 30% sample of {{13.other_skills_runs_sampled|,.0f}} runs:

| Test | Rows | Observed / predicted [95%] | Verdict |
|---|---|---|---|
| inheritance | {{13.other_skills_inheritance_rows|,.0f}} | {{13.other_skills_inheritance|.3f}} [{{13.other_skills_inheritance_95_pct_low|.3f}}, {{13.other_skills_inheritance_95_pct_high|.3f}}] | slightly short: known miss; no one skill carries more than {{13.other_skills_largest_one_skill_s_share_of_the_inheritance_shortfall|.0%}} of it |
| training-pool hints | {{13.other_skills_training_pool_rows|,.0f}} | {{13.other_skills_training_pool_hints|.3f}} [{{13.other_skills_training_pool_95_pct_low|.3f}}, {{13.other_skills_training_pool_95_pct_high|.3f}}] | under-predicted |

The deck test leaves out rows with an inspiration, so it is conditional on no spark, treated as independent of the
deck.
