# The model's mathematics

How the planner computes its odds, how each part was set, and how well it holds against real careers.

## Results

Against {{12.runs_used|,.0f}} real Independent Training careers rebuilt from their own decks and lineages (section 12),
the chance the model gives each target of being acquired is calibrated: observed / predicted
{{12.3_acquired_any_source_obs_per_pred|.3f}} [{{12.3_acquired_any_source_ratio_95_pct_low|.2f}}, {{12.3_acquired_any_source_ratio_95_pct_high|.2f}}] over {{12.3_acquired_any_source_rows|,.0f}} run-target rows. Its chance of landing all eight default
targets reads {{12.p_all_eight_ratio_all|.2f}} [{{12.p_all_eight_ratio_all_95_pct_low|.2f}}, {{12.p_all_eight_ratio_all_95_pct_high|.2f}}] of the model, consistent with calibrated but on only {{12.runs_acquiring_every_open_target|.0f}} runs that
acquired every target.

| Part | How it is set | Main evidence | Section |
|---|---|---|---|
| Spark generation, $0.2$ / $0.4 \times 1.1^n$ | assumed | none here: the runs show inherited sparks, not produced ones | 1 |
| Star mix of sparks | mixes assumed; band per scenario derived | each scenario's band is the one most of its careers reach ({{2.grand_concert_runs|,.0f}} Grand Concert runs, {{2.grand_concert_share_at_17_500_or_more|.0%}} at 17,500+) | 2 |
| Inheritance rolls | skill-white bases tested, grandparents fitted | grandparent weight {{3.grandparent_weight_fitted|.2f}} [{{3.grandparent_weight_95_pct_low|.2f}}, {{3.grandparent_weight_95_pct_high|.2f}}], affinity slope {{3.affinity_slope_fitted|.2f}} [{{3.affinity_slope_95_pct_low|.2f}}, {{3.affinity_slope_95_pct_high|.2f}}] on {{3.trials_grandparents|,.0f}} + {{3.trials_parents|,.0f}} trials; careers {{3.inheritance_obs_per_pred|.3f}} [{{3.inheritance_ratio_95_pct_low|.2f}}, {{3.inheritance_ratio_95_pct_high|.2f}}] | 3 |
| Affinity: +3 per shared G1 win; own grandparent adds no trio points | fitted | shared-win slope {{3.race_part_slope_fitted|.2f}} [{{3.race_part_slope_95_pct_low|.2f}}, {{3.race_part_slope_95_pct_high|.2f}}]; trio rule {{4.ll_0_ll_pair_nats|.0f}} nats over the alternative | 3, 4 |
| Race win table | community table, tested | calibrated on spark-raised aptitudes (as the career planner uses them), {{4.race_obs_per_pred_pink|.3f}} [{{4.race_obs_per_pred_pink_95_pct_low|.3f}}, {{4.race_obs_per_pred_pink_95_pct_high|.3f}}] over {{4.race_attempts_at_the_34|,.0f}} G1 attempts | 4 |
| Loop schedules for shared wins | derived | no schedule could raise a validated winner by more than {{5.schedules_pb_ceiling_gain_max_pct|.1f}}% (solver's pair ceilings) | 5 |
| Training hints, $\beta(1 + hf/100)(1 + d\,\mathit{sp}/100)$ | form derived, $\beta$ and $d$ fitted | {{6.specialty_rows|,.0f}} rows; held out +{{6.specialty_held_out_gain_nats|.1f}} nats; real careers {{12.1_training_pool_only_obs_per_pred|.3f}} [{{12.1_training_pool_only_ratio_95_pct_low|.2f}}, {{12.1_training_pool_only_ratio_95_pct_high|.2f}}] of the model | 6 |
| Chain events, Binomial(21, 0.35) per career | fitted | {{7.fit_of_n_runs|,.0f}} runs; not rejected against a free distribution ($p$ = {{7.binomial_21_0_35_lr_p|.2f}}) | 7 |
| Chain events spread over remaining links | fitted | {{7.allocation_slot_over_card_nats|.0f}} nats over uniform-by-card | 7 |
| Event choices: IT takes a prioritised skill's choice | assumed | event hints of prioritised targets {{12.1_events_only_prioritised_obs_per_pred|.2f}} of the model | 8, 12 |
| Measured outcome shares | fitted | five events, e.g. Nakayama Festa's Nothing Ventured {{8.festa_share|.2f}} [{{8.festa_share_90_pct_low|.2f}}, {{8.festa_share_90_pct_high|.2f}}] (90%) | 8 |
| Random card events | not modelled | left out, acquisition on the rows they could hint reads {{9.calibration_obs_per_pred_left_out|.2f}} [{{9.calibration_ratio_95_pct_low_left_out|.2f}}, {{9.calibration_ratio_95_pct_high_left_out|.2f}}] of the model; credited, {{9.calibration_obs_per_pred_credited|.2f}} | 9 |
| Conditional character events | not modelled | where only they can hint a target, left out reads {{10.calibration_conditional_only_obs_per_pred_left_out|.2f}} [{{10.calibration_conditional_only_ratio_95_pct_low_left_out|.2f}}, {{10.calibration_conditional_only_ratio_95_pct_high_left_out|.2f}}]; Vodka's own Nimble Navigator {{10.vodka_any_outfit_nimble_navigator_from_her_events|.0f}} of {{10.vodka_any_outfit_nimble_navigator_rows|.0f}} | 10 |

**Known misses**, each measured and tested in its section:

- Chain events: a binomial with another $n$, Binomial({{7.best_binomial_n_free_n|.0f}}, {{7.best_binomial_n_free_p|.3f}}), fits {{7.best_binomial_n_free_nats_better_than_21_0_35|.1f}} nats better; in crowded decks early links run
  short and more chains never start than predicted ({{7.never_started_6_chain_cards_observed|.2f}} against {{7.never_started_6_chain_cards_model|.2f}} with six chain cards); final links fit pooled
  ({{7.final_links_by_card_pooled_observed|.3f}} against {{7.final_links_by_card_pooled_model|.3f}}) but individual cards differ far beyond chance ($\chi^2$ = {{7.final_links_by_card_chi2|.0f}} on {{7.final_links_by_card_cards|.0f}} cards) (7).
- Deck hints read {{12.1_deck_hint_no_inspiration_obs_per_pred|.3f}} [{{12.1_deck_hint_no_inspiration_ratio_95_pct_low|.2f}}, {{12.1_deck_hint_no_inspiration_ratio_95_pct_high|.2f}}] of the model, almost all from event hints of targets the
  player did not prioritise ({{12.1_events_only_not_prioritised_obs_per_pred|.2f}} of the model; the model assumes every target is prioritised) (12).
- Trackblazer's hint base over-predicts real careers: training-pool hints read {{12.1_training_pool_only_trackblazer_obs_per_pred|.3f}} [{{12.1_training_pool_only_trackblazer_ratio_95_pct_low|.2f}}, {{12.1_training_pool_only_trackblazer_ratio_95_pct_high|.2f}}] of the model (12).
- Training hints: SR cards and decks with more pals depart from the adopted rate (+{{6.covariate_gain_sr_nats|.1f}} and +{{6.covariate_gain_pals_per_deck_nats|.1f}} nats); a skill
  also hinted by inspiration is hinted by training less than predicted ({{6.spark_inspiration_ratio|.2f}} of the model) (6).
- Left out of the model: deck cards' random events, which under-predicts the targets they can hint
  ({{9.calibration_obs_per_pred_left_out|.2f}} [{{9.calibration_ratio_95_pct_low_left_out|.2f}}, {{9.calibration_ratio_95_pct_high_left_out|.2f}}] of the model on those rows) (9), and Vodka's own Nimble Navigator ({{10.vodka_any_outfit_nimble_navigator_from_her_events|.0f}} of {{10.vodka_any_outfit_nimble_navigator_rows|.0f}} runs) (10).
- Training-pool hints of non-target skills read {{12.other_skills_training_pool_hints|.3f}} [{{12.other_skills_training_pool_95_pct_low|.2f}}, {{12.other_skills_training_pool_95_pct_high|.2f}}] of the model (12).

**Not settled.** Specialty priority predicts runs it was not fitted to (+{{6.specialty_held_out_gain_nats|.1f}} nats held out), but whether it
causes more hints is not shown: across limit breaks of the same card the estimate has the same size ($d$ =
{{6.specialty_same_card_d|.2f}}) without being significant (+{{6.specialty_same_card_gain_nats|.1f}} nats). The hint-frequency slope $1/100$ is assumed; a free
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
| URA Finale | {{2.ura_finale_runs|,.0f}} | {{2.ura_finale_median_rank_score|,.0f}} | {{2.ura_finale_share_at_17_500_or_more|.3f}} [{{2.ura_finale_share_95_pct_low|.3f}}, {{2.ura_finale_share_95_pct_high|.3f}}] | below |
| Unity Cup | {{2.unity_cup_runs|,.0f}} | {{2.unity_cup_median_rank_score|,.0f}} | {{2.unity_cup_share_at_17_500_or_more|.3f}} [{{2.unity_cup_share_95_pct_low|.3f}}, {{2.unity_cup_share_95_pct_high|.3f}}] | below |
| Grand Concert | {{2.grand_concert_runs|,.0f}} | {{2.grand_concert_median_rank_score|,.0f}} | {{2.grand_concert_share_at_17_500_or_more|.3f}} [{{2.grand_concert_share_95_pct_low|.3f}}, {{2.grand_concert_share_95_pct_high|.3f}}] | 17,500+ |
| Trackblazer | {{2.trackblazer_runs|,.0f}} | {{2.trackblazer_median_rank_score|,.0f}} | {{2.trackblazer_share_at_17_500_or_more|.3f}} [{{2.trackblazer_share_95_pct_low|.3f}}, {{2.trackblazer_share_95_pct_high|.3f}}] | below |

Unity Cup is close to even. Grand Masters has no runs and is **assumed** 17,500+.

The loop model does not know a legacy's stars and uses the expected base per roll (**derived**, section 3's bases):

$$\bar b = 0.03\,(\pi_1 + 2\pi_2 + 3\pi_3) = {{2.base_per_roll_below_17_500|.4f}} \;(\text{below } 17{,}500), \quad {{2.base_per_roll_17_500_and_up|.3f}} \;(17{,}500+).$$

That is exact for one roll but not for two: a legacy's stars are the same in both rolls, and by convexity
$E[(1 - r)^2] \ge (1 - E[r])^2$, so the loop model overstates $q_k$ (**derived**). On the six real affinities of each of
the {{2.lineages|,.0f}} lineages in the runs, for a target carried by all six, it overstates $q$ by at most
{{2.q_overstatement_below_17_500_max|.2%}} (below 17,500; mean {{2.q_overstatement_below_17_500_mean|.2%}}) and {{2.q_overstatement_17_500_and_up_max|.2%}} (17,500+). The career planner uses each
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
among the year's inspirations. Over {{3.trials_parents|,.0f}} parent and {{3.trials_grandparents|,.0f}} grandparent trials (ordinary ancestors; the
self-grandparent is section 4's):

- Grandparents take the full base: scaling their roll by a free weight fits {{3.grandparent_weight_fitted|.2f}} [{{3.grandparent_weight_95_pct_low|.2f}}, {{3.grandparent_weight_95_pct_high|.2f}}], and the full
  base beats a halved one by {{3.grandparents_full_over_halved_nats|,.0f}} nats ({{3.grandparents_passes_observed|,.0f}} passes observed, {{3.grandparents_predicted_full_base|,.0f}} predicted with the full base,
  {{3.grandparents_predicted_halved_base|,.0f}} halved). Parents read {{3.parents_obs_per_pred|.3f}} [{{3.parents_obs_per_pred_95_pct_low|.2f}}, {{3.parents_obs_per_pred_95_pct_high|.2f}}] of the model.
- Affinity scales the roll as $(1 + A/100)$: a free slope on $A$ fits {{3.affinity_slope_fitted|.2f}} [{{3.affinity_slope_95_pct_low|.2f}}, {{3.affinity_slope_95_pct_high|.2f}}], and the term beats no
  affinity by {{3.affinity_over_none_nats|,.0f}} nats. Its two parts apart, each with the other fixed: relation points {{3.relation_part_slope_fitted|.2f}}
  [{{3.relation_part_slope_95_pct_low|.2f}}, {{3.relation_part_slope_95_pct_high|.2f}}], $3 \times$ shared G1 wins {{3.race_part_slope_fitted|.2f}} [{{3.race_part_slope_95_pct_low|.2f}}, {{3.race_part_slope_95_pct_high|.2f}}] ({{3.trials_with_shared_wins|,.0f}} trials with shared wins).
- The base by stars holds: observed / predicted {{3.1_star_obs_per_pred|.3f}} [{{3.1_star_95_pct_low|.2f}}, {{3.1_star_95_pct_high|.2f}}] at 1★ ({{3.1_star_trials|,.0f}} trials),
  {{3.2_star_obs_per_pred|.3f}} [{{3.2_star_95_pct_low|.2f}}, {{3.2_star_95_pct_high|.2f}}] at 2★ ({{3.2_star_trials|,.0f}}), {{3.3_star_obs_per_pred|.3f}} [{{3.3_star_95_pct_low|.2f}}, {{3.3_star_95_pct_high|.2f}}] at 3★ ({{3.3_star_trials|,.0f}}).
- By affinity band the model holds jointly ($\chi^2$ = {{3.affinity_bands_chi2|.1f}} on five bands, $p$ = {{3.affinity_bands_chi2_p|.3f}}), but two bands lie outside their
  own 95% interval:

| Affinity | Trials | Observed / predicted |
|---|---|---|
| 0-49 | {{3.affinity_0_49_trials|,.0f}} | {{3.affinity_0_49_obs_per_pred|.3f}} [{{3.affinity_0_49_95_pct_low|.2f}}, {{3.affinity_0_49_95_pct_high|.2f}}] |
| 50-99 | {{3.affinity_50_99_trials|,.0f}} | {{3.affinity_50_99_obs_per_pred|.3f}} [{{3.affinity_50_99_95_pct_low|.2f}}, {{3.affinity_50_99_95_pct_high|.2f}}] |
| 100-149 | {{3.affinity_100_149_trials|,.0f}} | {{3.affinity_100_149_obs_per_pred|.3f}} [{{3.affinity_100_149_95_pct_low|.2f}}, {{3.affinity_100_149_95_pct_high|.2f}}] |
| 150-199 | {{3.affinity_150_199_trials|,.0f}} | {{3.affinity_150_199_obs_per_pred|.3f}} [{{3.affinity_150_199_95_pct_low|.2f}}, {{3.affinity_150_199_95_pct_high|.2f}}] |
| 200+ | {{3.affinity_200_trials|,.0f}} | {{3.affinity_200_obs_per_pred|.3f}} [{{3.affinity_200_95_pct_low|.2f}}, {{3.affinity_200_95_pct_high|.2f}}] |

**Test on whole careers** (section 12): observed / predicted inspiration on {{3.runs_tested|,.0f}} real careers,

- all {{3.run_target_rows|,.0f}} run-target rows: {{3.inheritance_obs_per_pred|.3f}} [{{3.inheritance_ratio_95_pct_low|.2f}}, {{3.inheritance_ratio_95_pct_high|.2f}}];
- by the number of carriers, 1 to 6: {{3.by_carriers_1_6_lowest_ratio|.3f}}-{{3.by_carriers_1_6_highest_ratio|.3f}}, every interval covering 1;
- per roll: spark-years {{3.per_roll_spark_years_obs_per_exp|.3f}} ({{3.per_roll_spark_years_sds_below_expectation|.2f}} standard deviations below the model's expectation, at the edge of its
  95% band), ancestor hits ($2\sum_l r_l$) {{3.per_roll_ancestor_hits_obs_per_exp|.3f}};
- {{3.non_target_skills_inheritance_rows|,.0f}} rows of non-target skills: {{3.non_target_skills_inheritance|.3f}} [{{3.non_target_skills_inheritance_95_pct_low|.2f}}, {{3.non_target_skills_inheritance_95_pct_high|.2f}}].

By predicted chance the buckets are calibrated jointly ($\chi^2$ = {{3.reliability_chi2|.1f}} on {{3.reliability_chi2_buckets|.0f}} buckets, $p$ = {{3.reliability_chi2_p|.2f}}); one lies
outside its own 95% interval, 0.40-0.60
({{3.reliability_q_0_40_0_60_observed|.3f}} [{{3.reliability_q_0_40_0_60_observed_95_pct_low|.3f}}, {{3.reliability_q_0_40_0_60_observed_95_pct_high|.3f}}] against {{3.reliability_q_0_40_0_60_mean_predicted_three_places|.3f}}):

| Predicted $q$ | Rows | Observed (mean predicted) |
|---|---|---|
| 0 | {{3.reliability_q_0_rows|,.0f}} | {{3.reliability_q_0_observed|.3f}} |
| above 0 to 0.20 | {{3.reliability_q_0_00_0_20_rows|,.0f}} | {{3.reliability_q_0_00_0_20_observed|.3f}} ({{3.reliability_q_0_00_0_20_mean_predicted|.2f}}) |
| 0.20-0.40 | {{3.reliability_q_0_20_0_40_rows|,.0f}} | {{3.reliability_q_0_20_0_40_observed|.3f}} ({{3.reliability_q_0_20_0_40_mean_predicted|.2f}}) |
| 0.40-0.60 | {{3.reliability_q_0_40_0_60_rows|,.0f}} | {{3.reliability_q_0_40_0_60_observed|.3f}} ({{3.reliability_q_0_40_0_60_mean_predicted|.2f}}) |
| 0.60-0.80 | {{3.reliability_q_0_60_0_80_rows|,.0f}} | {{3.reliability_q_0_60_0_80_observed|.3f}} ({{3.reliability_q_0_60_0_80_mean_predicted|.2f}}) |
| 0.80-0.95 | {{3.reliability_q_0_80_0_95_rows|,.0f}} | {{3.reliability_q_0_80_0_95_observed|.3f}} ({{3.reliability_q_0_80_0_95_mean_predicted|.2f}}) |

## 4. Affinity

Computed by `research/s04_affinity.py` (the self-grandparent fit) and `research/s04_races.py` (the win table).

**Relation points** (exact, game data): $\mathrm{pair}(a,b)$ and $\mathrm{tri}(a,b,c)$ sum the relation points of the
relation groups containing all members. A group counts only distinct characters, so $\mathrm{tri}(T, P, T) = 0$.

**Race affinity**: +3 per single-race G1 both won, counted per race (relocated venues are the same race), multi-race
titles 0 (**assumed** as the game rule; the weight 3 per shared win is tested in section 3, slope
{{3.race_part_slope_fitted|.2f}} [{{3.race_part_slope_95_pct_low|.2f}}, {{3.race_part_slope_95_pct_high|.2f}}]). With schedules fixed in advance and two careers independent (**assumed**), expected shared wins are

$$o(a,b) = \sum_r P_a(r)\,P_b(r), \qquad P_a(r) = 1 - \prod_{\text{attempts}} (1 - p),$$
$$p = \mathrm{clamp}\big(110 - c_{\text{dist}} - c_{\text{surf}} - \mathrm{PEN}_j,\ 0,\ 100\big) / 100,$$

$c$ = 0 for S/A, 10 B, 20 C, 30 D, 40 E, 60 F, 90 G; $\mathrm{PEN}_j$ = 0, 0, 5, 20, 30, 50 for the $j$-th race in a
row (any race, goals included; a rest resets). The win table is a community table, tested below. Stats and training
turns are not modelled.

**The win table, tested** (**fitted**; `research/s04_races.py`). The runs record every race a career ran. Over
{{4.race_attempts_at_the_34|,.0f}} attempts at the 34 single-race G1s ({{4.race_goal_attempts|,.0f}} of them goals), {{4.race_win_rate_observed|.1%}} were won:

- On the trainee's base aptitudes, which is what the loop model uses, the table predicts {{4.race_win_rate_predicted_base|.1%}}: observed / predicted
  {{4.race_obs_per_pred_base|.3f}} [{{4.race_obs_per_pred_base_95_pct_low|.3f}}, {{4.race_obs_per_pred_base_95_pct_high|.3f}}], and {{4.race_base_bp_lt_70_obs_per_pred|.2f}} where the table gives 70 or less ({{4.race_base_bp_lt_70_attempts|,.0f}} attempts): aptitude
  sparks raise her grades during the career. The loop model does not know its veterans' pink sparks and keeps base
  aptitudes (a simplification); the career planner, which plans with real veterans, raises them by the rule below.
- With her grades raised by the career planner's aptitude-spark rule (`docs/MODEL.md`: the legacies' pink stars at the
  start, then each April inspiration the run records), it predicts {{4.race_win_rate_predicted_pink|.1%}}: observed / predicted
  {{4.race_obs_per_pred_pink|.3f}} [{{4.race_obs_per_pred_pink_95_pct_low|.3f}}, {{4.race_obs_per_pred_pink_95_pct_high|.3f}}], {{4.race_ll_recalibrated_pink_table_recalibrated_base_table_nats|,.0f}} nats better than base aptitudes, and calibrated in every scenario.
- The streak penalty is real: without it the fit is {{4.race_ll_recalibrated_pink_table_recalibrated_pink_no_penalty_nats|,.0f}} nats worse.

| Table's chance (raised grades) | Attempts | Observed / predicted |
|---|---|---|
| 110 | {{4.race_pink_bp_110_attempts|,.0f}} | {{4.race_pink_bp_110_obs_per_pred|.3f}} [{{4.race_pink_bp_110_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_110_obs_per_pred_95_pct_high|.3f}}] |
| 100 | {{4.race_pink_bp_100_attempts|,.0f}} | {{4.race_pink_bp_100_obs_per_pred|.3f}} [{{4.race_pink_bp_100_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_100_obs_per_pred_95_pct_high|.3f}}] |
| 90 | {{4.race_pink_bp_90_attempts|,.0f}} | {{4.race_pink_bp_90_obs_per_pred|.3f}} [{{4.race_pink_bp_90_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_90_obs_per_pred_95_pct_high|.3f}}] |
| 80 | {{4.race_pink_bp_80_attempts|,.0f}} | {{4.race_pink_bp_80_obs_per_pred|.3f}} [{{4.race_pink_bp_80_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_80_obs_per_pred_95_pct_high|.3f}}] |
| 70 or less | {{4.race_pink_bp_lt_70_attempts|,.0f}} | {{4.race_pink_bp_lt_70_obs_per_pred|.3f}} [{{4.race_pink_bp_lt_70_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_lt_70_obs_per_pred_95_pct_high|.3f}}] |

The table is not exact: a free win rate per (chance, streak position) cell ({{4.race_pink_cells_bp_x_j|.0f}} cells) beats it by
{{4.race_ll_pink_cells_bp_x_j_recalibrated_pink_table_nats|.0f}} nats, more than chance allows ({{4.race_ll_threshold_pink_cells_bp_x_j|.0f}}). The excess sits in goal races from the fourth race in a row, which win more
than the penalty allows ({{4.race_pink_goal_j_ge_4_obs_per_pred|.2f}} [{{4.race_pink_goal_j_ge_4_obs_per_pred_95_pct_low|.2f}}, {{4.race_pink_goal_j_ge_4_obs_per_pred_95_pct_high|.2f}}], {{4.race_pink_goal_j_ge_4_attempts|,.0f}} attempts), while chosen races there match
({{4.race_pink_chosen_j_ge_4_obs_per_pred|.2f}} [{{4.race_pink_chosen_j_ge_4_obs_per_pred_95_pct_low|.2f}}, {{4.race_pink_chosen_j_ge_4_obs_per_pred_95_pct_high|.2f}}]); and low chances win a little less than predicted. The test is conditional on IT
having chosen each non-goal race; stats are not modelled.

**Per-legacy affinity.** For parent $P$ with other parent $P'$ and own parents $\mathrm{par}(P)$, and grandparent
$G \in \mathrm{par}(P)$:

$$A_P = \mathrm{pair}(T,P) + \mathrm{pair}(P_1,P_2) + 3\,o(P,P') + \sum_{G \in \mathrm{par}(P)} \big[\mathrm{tri}(T,P,G) + 3\,o(P,G)\big]$$
$$A_G = \mathrm{tri}(T,P,G) + 3\,o(P,G)$$

In a loop $P'$ is also one of $P$'s own parents, so $o(P, P')$ counts twice in $A_P$. Tested in section 3 on the runs'
real lineages and shared wins: the roll scales with this $A$ at slope {{3.affinity_slope_fitted|.2f}} [{{3.affinity_slope_95_pct_low|.2f}}, {{3.affinity_slope_95_pct_high|.2f}}].

**Self-grandparent** ($G = T$, section 5). $\mathrm{tri}(T,P,T) = 0$ is **fitted** on {{4.trials_attributable_to_one_ancestor|,.0f}} inspiration trials
attributable to a single ancestor in runs: self-grandparent passes {{4.self_grandparent_passes_observed|.0f}} observed,
{{4.predicted_tri_t_p_t_0|.1f}} predicted with 0, {{4.predicted_tri_t_p_t_pair_t_p|.1f}} with $\mathrm{tri}(T,P,T) = \mathrm{pair}(T,P)$ (about {{4.ll_0_ll_pair_nats|.0f}} nats worse); the family
$\kappa\cdot\mathrm{pair}(T,P)$ fits $\hat\kappa = {{4.kappa_fitted|.0f}}$, 95% upper limit about {{4.kappa_95_pct_upper_limit|.2f}}.

## 5. Loop structures, ranking and schedules

Computed by `research/s05_loops.py`.

A lineage is $(T, P_1, P_2, X)$: each parent bred from the other parent and $X$. A four-uma loop has {{5.four_uma_loop_lineages|.0f}} lineages (each
member trains from two of the other three); a three-uma rotation has {{5.three_uma_rotation_lineages|.0f}}, with $X = T$ (the trainee's own earlier
veteran). The rotation occurs in practice: {{5.a_grandparent_of_her_own_character|.0f}} of {{5.runs|,.0f}} runs have a grandparent of
the trainee's own character ({{5.a_parent_of_her_own_character|.0f}} have a parent of her character). A structure $S$ with lineage set $\Lambda_S$ scores

$$\bar P_S = \frac{1}{|\Lambda_S|} \sum_{\lambda \in \Lambda_S} P_\lambda(\text{all}),$$

the long-run rate of a player who trains every member in turn (a definition of what the planner ranks, not a claim: a
player who re-trains only the best trainee would score a structure by its best lineage).

**Race weights** (**derived**). In one lineage $o(P_1,P_2)$ enters the six affinities {{5.one_lineage_o_p1_p2_weight|.0f}} times and $o(P_1,X)$,
$o(P_2,X)$ twice each. Summed over the 12 loop lineages every pair gets ${{5.one_lineage_o_p1_p2_weight|.0f}}\cdot 2 + {{5.one_lineage_o_p1_x_weight|.0f}}\cdot 4 = {{5.loop_weight_per_pair|.0f}}$; over the 3
rotation lineages ${{5.one_lineage_o_p1_p2_weight|.0f}} + {{5.one_lineage_o_p1_x_weight|.0f}} + {{5.one_lineage_o_p1_x_weight|.0f}} = {{5.rotation_weight_per_pair|.0f}}$. Both are uniform over pairs, so a structure's schedules maximise
$\sum_{a<b} o(a,b)$.

**Schedules for shared wins, not odds** (**derived**, computed on the validation results). The solver schedules each
structure for $\sum_{a<b} o(a,b)$, not for $\bar P_S$. $\bar P_S$ rises with every $o$: $o$ enters each legacy's
affinity with weight +3, and $r$, $q$ and every target's odds are nondecreasing in their inputs. So, with a loop's decks
fixed, any schedule (one chosen for $\bar P_S$ directly included) scores between the odds found and the odds with every
pair at its highest possible overlap. Over {{5.schedules_result_files|.0f}} validation results, recomputed independently to within
{{5.schedules_recomputed_odds_max_relative_error|.0e}} of the solver:

- the shared wins found are worth {{5.schedules_shared_win_value_min_pct|.0f}}-{{5.schedules_shared_win_value_max_pct|.0f}}% over no race affinity;
- the independent ceiling $o_{\max}(a,b) = \sum_r P^{\max}_a(r)\,P^{\max}_b(r)$ (every offering of every G1 attempted, no
  streak penalty) caps the gain from a different objective at {{5.schedules_ceiling_gain_max_pct|.1f}}%; it is far from reachable (the schedules
  found reach about {{5.schedules_overlap_found_as_pct_of_o_max_median|.0f}}% of it) and cannot settle the ranking: {{5.schedules_runner_ups_whose_ceiling_reaches_the_winner|.0f}} of {{5.schedules_runner_up_loops_checked|.0f}} listed runner-up loops reach their
  winner under it;
- the solver's exact pair ceilings (its best-response search over feasible schedules, so not independent; they sit
  between $o$ and $o_{\max}$ on every pair) put the ceiling only {{5.schedules_pb_ceiling_gain_min_pct|.2f}}-{{5.schedules_pb_ceiling_gain_max_pct|.2f}}% above the odds found (median
  {{5.schedules_pb_ceiling_gain_median_pct|.2f}}%), and no listed runner-up's ceiling reaches its winner ({{5.schedules_runner_ups_whose_pb_ceiling_reaches_the_winner|.0f}} of {{5.schedules_runner_up_loops_checked|.0f}}).

## 6. Training hints

Computed by `research/s06_hints.py`.

Each deck card makes $h$ training-hint draws per run, each uniform over its skill pool (size $n$; the stat option is
never drawn) among skills still below hint Lv 5; a skill leaves after $c = \lceil 5 / \iota \rceil$ draws, $\iota$ =
hint levels per draw ($c$ = {{6.draws_to_lv_5_1_card|.0f}}, {{6.draws_to_lv_5_2_card|.0f}}, {{6.draws_to_lv_5_3_card|.0f}}, {{6.draws_to_lv_5_4_card|.0f}}, {{6.draws_to_lv_5_5_card|.0f}} for +1 ... +5 cards).

$$h = \beta_{\text{scenario}}\,\left(1 + \frac{hf}{100}\right)\left(1 + d_{\text{scenario}}\,\frac{\mathit{sp}}{100}\right)$$

$hf$ = hint frequency and $\mathit{sp}$ = specialty priority at maximum limit break, unique effects included. For $k$
targets in one card's pool the miss chance is joint:

$$m(k, n, t) = \left(1 - \frac{k}{n}\right)^{t} \quad (t \le c), \qquad \text{exact Markov chain over the other skills' draw counts otherwise},$$
$$M(k, n, h) = (1 - f)\, m(k, n, \lfloor h \rfloor) + f\, m(k, n, \lfloor h \rfloor + 1), \qquad f = h - \lfloor h \rfloor.$$

The shape (uniform draws, $h$ per run, the cap) is **derived** from the data; $\beta$ and $d$ are **fitted**. Fit:
Bernoulli likelihood through $M$ per (card-run, pool skill) row, maximum-limit-break card-runs only, rows where no spark
hinted the skill, R cards out. The form study uses every card-run at any limit break, R cards included, scored without the specialty term
({{6.form_rows|,.0f}} rows from {{6.form_runs|,.0f}} runs, {{6.form_rows_without_a_spark|,.0f}} without a spark); $\beta$ and $d$ are fitted ({{6.specialty_rows|,.0f}} rows, {{6.specialty_card_runs|,.0f}} card-runs; the Runs column counts the runs that add a row):

| Scenario | $\beta$ | $d$ | Adopted $\beta$ / $d$ | Rows | Runs |
|---|---|---|---|---|---|
| Unity Cup | {{6.specialty_base_unity_cup|.3f}} | {{6.specialty_d_unity_cup|.3f}} | {{6.adopted_unity_cup|.2f}} / {{6.adopted_d_unity_cup|.2f}} | {{6.specialty_rows_unity_cup|,.0f}} | {{6.specialty_runs_unity_cup|.0f}} |
| Grand Concert | {{6.specialty_base_grand_concert|.3f}} | {{6.specialty_d_grand_concert|.3f}} | {{6.adopted_grand_concert|.2f}} / {{6.adopted_d_grand_concert|.2f}} | {{6.specialty_rows_grand_concert|,.0f}} | {{6.specialty_runs_grand_concert|,.0f}} |
| URA Finale | {{6.specialty_base_ura_finale|.3f}} | {{6.specialty_d_ura_finale|.3f}} | {{6.adopted_ura_finale|.2f}} / {{6.adopted_d_ura_finale|.2f}} | {{6.specialty_rows_ura_finale|,.0f}} | {{6.specialty_runs_ura_finale|.0f}} |
| Trackblazer | {{6.specialty_base_trackblazer|.3f}} | {{6.specialty_d_trackblazer|.3f}} | {{6.adopted_trackblazer|.2f}} / {{6.adopted_d_trackblazer|.2f}} | {{6.specialty_rows_trackblazer|,.0f}} | {{6.specialty_runs_trackblazer|.0f}} |
| Grand Masters | no runs | | {{6.adopted_grand_masters|.2f}} / {{6.adopted_d_grand_masters|.2f}} (**assumed**: one $\beta$ and one $d$ over every scenario) | | |

What fixed the form (the form study):

| Question | Evidence |
|---|---|
| Rate form, LL on {{6.form_rows|,.0f}} rows | $1 + hf/25$ over options {{6.ll_1_hf_per_25_over_options|u,.1f}}; $1 + hf/25$ over skills {{6.ll_1_hf_per_25_over_skills|u,.1f}}; constant {{6.ll_constant|u,.1f}}; $\beta(1 + hf/100)$ {{6.ll_base_1_hf_per_100|u,.1f}}; per-scenario $\beta(1 + hf/100)$ {{6.ll_per_scenario_base_1_hf_per_100|u,.1f}}; free divisor ($c = {{6.free_divisor_c|.0f}}$) {{6.ll_free_divisor|u,.1f}} |
| Skills only, not skills + stat option | in {{6.cells_var_below_stat_option_floor|.0f}} of {{6.cells_card_hf|.0f}} (card, $hf$) cells the draw-count variance is below the binomial floor a drawn stat option implies |
| Uniform draws with repeats | repeat rate $P(k \ge 2 \mid k \ge 1)$: {{6.repeat_rate_observed|.3f}} observed, {{6.repeat_rate_two_point_count|.3f}} two-point count, {{6.repeat_rate_poisson_count|.3f}} Poisson count; distinct skills 1/2/3/4 per card-run {{6.distinct_1_observed|.3f}} / {{6.distinct_2_observed|.3f}} / {{6.distinct_3_observed|.3f}} / {{6.distinct_4_observed|.3f}} observed, {{6.distinct_1_model|.3f}} / {{6.distinct_2_model|.3f}} / {{6.distinct_3_model|.3f}} / {{6.distinct_4_model|.3f}} model |
| Covariates | per-scenario bases +{{6.per_scenario_bases_gain_nats|.0f}} nats (form study). On top of the adopted rate ({{6.covariate_rows|,.0f}} rows): card type +{{6.covariate_gain_card_type_nats|.1f}} nats (4 parameters), initial bond +{{6.covariate_gain_initial_bond_nats|.1f}}, pool size +{{6.covariate_gain_pool_size_nats|.1f}}, none significant; rarity (SR against SSR) +{{6.covariate_gain_sr_nats|.1f}} and pals per deck +{{6.covariate_gain_pals_per_deck_nats|.1f}} are (known misses, factors {{6.covariate_factor_sr|.3f}} for SR and {{6.covariate_factor_pals_per_deck|.3f}} per pal) |
| Lv-5 cap | beats drawing with replacement by {{6.cap_gain_nats|.1f}} nats with no parameter; obs / pred by draws to Lv 5, replacement vs cap: 5 draws ({{6.cap_table_5_draws_rows|,.0f}} rows) {{6.cap_table_5_draws_replacement|.3f}} / {{6.cap_table_5_draws_cap|.3f}}, 3 ({{6.cap_table_3_draws_rows|,.0f}}) {{6.cap_table_3_draws_replacement|.3f}} / {{6.cap_table_3_draws_cap|.3f}}, 2 ({{6.cap_table_2_draws_rows|,.0f}}) {{6.cap_table_2_draws_replacement|.3f}} / {{6.cap_table_2_draws_cap|.3f}}, 1 ({{6.cap_table_1_draws_rows|.0f}}) {{6.cap_table_1_draws_replacement|.3f}} / {{6.cap_table_1_draws_cap|.3f}} |
| Joint draws over targets | {{6.joint_pairs|,.0f}} pairs of no-spark pool skills on one card-run (Trackblazer left out): $P(\text{both})$ {{6.p_both_observed|.4f}} observed, {{6.p_both_joint|.4f}} joint, {{6.p_both_independent|.4f}} independent; $P(\text{neither})$ {{6.p_neither_observed|.4f}} vs {{6.p_neither_joint|.4f}} |
| Pool size | obs / pred {{6.pool_size_obs_per_pred_min|.2f}}-{{6.pool_size_obs_per_pred_max|.2f}} for every $n$ with at least 150 rows |
| Per-draw increment | {{6.increment_multiples_of_iota|,.0f}} of {{6.increment_single_source_levels|,.0f}} single-source levels are multiples of $\iota$, capped at 5 |
| $hf$ by card level | floored linear interpolation {{6.ll_hf_floored_linear|u,.1f}}, step {{6.ll_hf_step|u,.1f}}, max-limit-break $hf$ for all {{6.ll_hf_max_lb_for_all|u,.1f}} |

The slope $1/100$ is **assumed**: a free divisor fits better ($c = {{6.free_divisor_c|.0f}}$ in the table above) and is not adopted.

Reading $h$ as a floor/ceiling mixture against a two-point count differs by under 1% ($n = 5$, $h = 2.74$: miss
{{6.miss_1_1_per_n_h_n_5_h_2_74|.4f}} vs {{6.miss_floor_per_ceiling_n_5_h_2_74|.4f}}). $hf$ spans 0-{{6.hf_max_in_the_data|.0f}} in the data.

**Spark deficit** (not modelled; the fit conditions on no spark, the case in which a card's hint matters). Scored
against the adopted rate on maximum-limit-break card-runs, R cards out:

| Other source of the skill | Rows | Observed | Predicted | Ratio |
|---|---|---|---|---|
| none | {{6.spark_none_rows|,.0f}} | {{6.spark_none_observed|.3f}} | {{6.spark_none_predicted|.3f}} | {{6.spark_none_ratio|.2f}} |
| inspiration | {{6.spark_inspiration_rows|,.0f}} | {{6.spark_inspiration_observed|.3f}} | {{6.spark_inspiration_predicted|.3f}} | {{6.spark_inspiration_ratio|.2f}} |
| inspiration + card event | {{6.spark_inspiration_card_event_rows|,.0f}} | {{6.spark_inspiration_card_event_observed|.3f}} | {{6.spark_inspiration_card_event_predicted|.3f}} | {{6.spark_inspiration_card_event_ratio|.2f}} |
| card event | {{6.spark_card_event_rows|,.0f}} | {{6.spark_card_event_observed|.3f}} | {{6.spark_card_event_predicted|.3f}} | {{6.spark_card_event_ratio|.2f}} |
| other card's training | {{6.spark_other_card_training_rows|,.0f}} | {{6.spark_other_card_training_observed|.3f}} | {{6.spark_other_card_training_predicted|.3f}} | {{6.spark_other_card_training_ratio|.2f}} |

Full drop-out of spark-hinted skills from the draws is rejected (with two pool skills spark-hinted, the rest are hinted
at {{6.drop_out_test_observed|.3f}} against {{6.drop_out_test_no_drop_out|.3f}} without drop-out and {{6.drop_out_test_drop_out|.3f}} with it).

**Specialty priority** (effect 19). On the fit's setup above, the rate without the specialty term leaves a residual
that rises with specialty: observed / predicted {{6.specialty_band_0_before|.3f}} at specialty 0 ({{6.specialty_band_0_rows|,.0f}} rows), {{6.specialty_band_1_29_before|.3f}} at 1-29 ({{6.specialty_band_1_29_rows|.0f}}), {{6.specialty_band_30_59_before|.3f}} at
30-59 ({{6.specialty_band_30_59_rows|,.0f}}), {{6.specialty_band_60_before|.3f}} at 60+ ({{6.specialty_band_60_rows|,.0f}}). The term gains {{6.specialty_gain_per_scenario_nats|.1f}} nats for 4 parameters (one $d$ for every scenario: {{6.specialty_gain_one_d_nats|.1f}}
nats, $d = {{6.specialty_d_pooled|.3f}}$; an exponential form $e^{g_s\,\mathit{sp}/10}$ fits the same, {{6.specialty_gain_exp_form_nats|.1f}} nats). The bands then read {{6.specialty_band_0_after|.3f}} /
{{6.specialty_band_1_29_after|.3f}} / {{6.specialty_band_30_59_after|.3f}} / {{6.specialty_band_60_after|.3f}}, no (scenario, band) cell with 200+ rows is off by more than $|z| = {{6.specialty_cells_max_z_after|.2f}}$, and Unity Cup cards
with $hf = 0$ and specialty 0 go from {{6.unity_hf_0_spec_0_before|.3f}} to {{6.unity_hf_0_spec_0_after|.3f}}. One pooled $d$ stays within {{6.specialty_d_robustness_min|.2f}}-{{6.specialty_d_robustness_max|.2f}} on either half of the runs,
on SR or SSR cards alone and without any one of the eight most-used cards. Fitted on the earlier runs and scored on
the later {{6.specialty_held_out_rows|,.0f}} rows, it gains {{6.specialty_held_out_gain_nats|.1f}} nats: it predicts runs it was not fitted to. Against real careers
(section 12) training-pool hints read {{12.1_training_pool_only_obs_per_pred|.3f}} of the model with it.

The term is an association, not shown to be a cause: high-specialty cards might share another trait that raises
hints. The cleanest test compares the same card at different limit breaks, where specialty changes and the card does
not ({{6.specialty_same_card_cards|.0f}} cards, a base per card and scenario): $d$ = {{6.specialty_same_card_d|.2f}}, the same size as across cards, but a gain of only
{{6.specialty_same_card_gain_nats|.1f}} nats, not significant.

Prioritised skills raising training hints: {{6.priority_runs|.0f}} runs show a priority list, prioritised skills hinted by training at {{6.priority_observed|.3f}}
against {{6.priority_predicted|.3f}} predicted ({{6.priority_rows|.0f}} rows); no effect assumed.

## 7. Chain events

Computed by `research/s07_chains.py`.

A run gets $N$ chain events, the same for every deck and scenario (**fitted**; a binomial with another $n$ fits better,
below):

$$N \sim \mathrm{Binomial}(21, 0.35), \qquad P(N = e) = \binom{21}{e}\, 0.35^{e}\, 0.65^{21-e}, \quad E N = {{7.binomial_21_0_35_mean|.2f}},\ \mathrm{Var}\, N = {{7.binomial_21_0_35_variance|.1f}}.$$

Each event advances one non-pal chain card by one link (links fire in order). Card $i$ with chain length $L_i$ (SSR 3,
SR 2; a filler slot counts as a 2-link SR) and $d_i$ links fired is picked with

$$P(i) = \frac{w_i}{\sum_j w_j}, \qquad w_i = L_i - d_i \text{ while running, } 0 \text{ once stopped},$$

i.e. uniform over the remaining chain slots (**fitted** against the alternative, below). Pal and group cards take no slot
(**assumed**, supported below). Without stop risk $P(\text{all chains complete}) = P(N \ge \sum_i L_i)$; two 3-link
cards: $P(N \ge 6) = {{7.two_3_link_cards_p_n_ge_6_p_both_chains_complete|.2f}}$.

**Stop risk** (**derived** from the event data and the choice rule, section 8): if IT's default choice at link $l$ ends
the chain on $k$ of its $n$ outcomes, $q_l = k/n$ and the stopping link $\sigma$ has
$P(\sigma = l) = q_l \prod_{l' < l} (1 - q_{l'})$; a stopped chain's slots return to the pool. The solver enumerates the
exact distribution of final chain states (no simulation); an independent recomputation of the solver's odds agrees to
{{5.schedules_recomputed_odds_max_relative_error|.0e}} on every validated loop (section 5).

**Fit of $N$** ({{7.fit_of_n_runs|,.0f}} runs, the pal-count table below). Chain progress per card is read from marker skills: a link is
observable only if every choice and outcome of its event carries a skill appearing in no other event of that card. A
credit naming two sources ("A or B event") is unknown when B is another deck card whose events can give the marker, and
so is a marker listed only under inspiration. $N$'s distribution is the one that best explains progress over all decks.

The fit allows a fired link's marker to go unseen with chance 0.03 (**assumed**, a measurement allowance). Without
it, a run in which a card's later link shows while an earlier one does not has likelihood 0. Since links fire in
order, those runs measure the rate: of {{7.allowance_earlier_links_implied_fired|.0f}} earlier links implied fired by a later one, {{7.allowance_marker_not_shown|.0f}} show no marker, {{7.allowance_share|.3f}}
[{{7.allowance_95_pct_low|.3f}}, {{7.allowance_95_pct_high|.3f}}]. The cause is not known. The fit does not depend on it: with allowances from 0.01 to 0.045, or with
none on the {{7.allowance_0_runs_without_a_contradiction|,.0f}} runs that show no such case, the best $p$ at $n = 21$ is {{7.allowance_sensitivity_lowest_best_p_at_n_21|.2f}}-{{7.allowance_sensitivity_highest_best_p_at_n_21|.3f}}.

| Form for $N$ | Result |
|---|---|
| distribution-free | mean {{7.distribution_free_mean|.2f}}, variance {{7.distribution_free_variance|.2f}} |
| Binomial(21, 0.35) | {{7.binomial_21_0_35_nats_behind_distribution_free|.1f}} nats behind distribution-free with {{7.distribution_free_fit_parameters_more_than_the_binomial|.0f}} fewer parameters (not rejected, $p$ = {{7.binomial_21_0_35_lr_p|.2f}}) |
| Binomial({{7.best_binomial_n_free_n|.0f}}, {{7.best_binomial_n_free_p|.3f}}) | the best binomial with $n$ free: {{7.best_binomial_n_free_nats_behind_distribution_free|.1f}} nats behind distribution-free ($p$ = {{7.best_binomial_n_free_lr_p|.2f}}), {{7.best_binomial_n_free_nats_better_than_21_0_35|.1f}} better than (21, 0.35) |
| Conway-Maxwell-Poisson | another under-dispersed two-parameter family: {{7.conway_maxwell_poisson_nats_behind_distribution_free|.1f}} nats behind distribution-free ($p$ = {{7.conway_maxwell_poisson_lr_p|.2f}}) |
| Binomial(15, 0.48) | {{7.binomial_15_0_48_nats_behind_distribution_free|.1f}} nats behind distribution-free |
| Poisson(7.1) | {{7.poisson_7_1_nats_behind_distribution_free|.1f}} nats behind distribution-free (variance = mean, too wide; rejected) |
| per scenario | means Unity Cup {{7.per_scenario_mean_unity_cup|.2f}}, Grand Concert {{7.per_scenario_mean_grand_concert|.2f}}, URA Finale {{7.per_scenario_mean_ura_finale|.2f}}, Trackblazer {{7.per_scenario_mean_trackblazer|.2f}}; +{{7.per_scenario_log_likelihood_gain|.1f}} nats for {{7.per_scenario_extra_parameters|.0f}} extra parameters (not significant) |

Why a binomial: the free fit's variance is {{7.distribution_free_variance_per_mean|.2f}} of its mean, so $N$ is under-dispersed, which rules out
the Poisson and every over-dispersed count. A binomial (a fixed number of chances, each taken with the same
probability) is the simplest under-dispersed count, but the data do not single it out: the Conway-Maxwell-Poisson fits
about as well. What the model uses is the distribution of $N$, not the family behind it.
The adopted (21, 0.35) is not rejected, but Binomial({{7.best_binomial_n_free_n|.0f}}, {{7.best_binomial_n_free_p|.3f}}) fits better by more than chance allows for its one
extra parameter (a known miss; the adopted form was chosen on fewer runs). Fitted per pal count, the means are
{{7.per_pal_count_mean_1_pals|.2f}} / {{7.per_pal_count_mean_2_pals|.2f}} / {{7.per_pal_count_mean_3_pals|.2f}} / {{7.per_pal_count_mean_4_pals|.2f}} for 1 / 2 / 3 / 4 pals ({{7.per_pal_count_runs_1_4_pals|,.0f}} runs): more pals do not lower $N$, so pal events do not consume chain events.

Final-link completion by pal count; the binomial lies inside every interval except at 2 pals, where it predicts more
completions than seen:

| Pals | Chain cards | Runs | Final link reached: observed / Binomial(21, 0.35) / Poisson(7.1) |
|---|---|---|---|
| 5 | 1 | {{7.final_link_table_5_pals_runs|,.0f}} | {{7.final_link_table_5_pals_observed|.2f}} / {{7.final_link_table_5_pals_binomial_21_0_35|.2f}} / {{7.final_link_table_5_pals_poisson_7_1|.2f}} |
| 4 | 2 | {{7.final_link_table_4_pals_runs|,.0f}} | {{7.final_link_table_4_pals_observed|.2f}} / {{7.final_link_table_4_pals_binomial_21_0_35|.2f}} / {{7.final_link_table_4_pals_poisson_7_1|.2f}} |
| 3 | 3 | {{7.final_link_table_3_pals_runs|,.0f}} | {{7.final_link_table_3_pals_observed|.2f}} / {{7.final_link_table_3_pals_binomial_21_0_35|.2f}} / {{7.final_link_table_3_pals_poisson_7_1|.2f}} |
| 2 | 4 | {{7.final_link_table_2_pals_runs|,.0f}} | {{7.final_link_table_2_pals_observed|.2f}} / {{7.final_link_table_2_pals_binomial_21_0_35|.2f}} / {{7.final_link_table_2_pals_poisson_7_1|.2f}} |
| 1 | 5 | {{7.final_link_table_1_pals_runs|,.0f}} | {{7.final_link_table_1_pals_observed|.2f}} / {{7.final_link_table_1_pals_binomial_21_0_35|.2f}} / {{7.final_link_table_1_pals_poisson_7_1|.2f}} |
| 0 | 6 | {{7.final_link_table_0_pals_runs|,.0f}} | {{7.final_link_table_0_pals_observed|.2f}} / {{7.final_link_table_0_pals_binomial_21_0_35|.2f}} / {{7.final_link_table_0_pals_poisson_7_1|.2f}} |

**Allocation rule.** Against uniform over cards (each running card's next event equally likely), on {{7.allocation_runs|,.0f}} runs:

| | Uniform over remaining slots (adopted) | Uniform over cards |
|---|---|---|
| Total LL | {{7.allocation_total_ll_slot_rule|u,.1f}} | {{7.allocation_total_ll_card_rule|u,.1f}} |
| Final links, 2 pals (observed {{7.allocation_final_links_2_pals_observed|.2f}}) | {{7.allocation_final_links_2_pals_slot_rule|.2f}} | {{7.allocation_final_links_2_pals_card_rule|.2f}} |
| Final links, 0 pals (observed {{7.allocation_final_links_0_pals_observed|.2f}}) | {{7.allocation_final_links_0_pals_slot_rule|.2f}} | {{7.allocation_final_links_0_pals_card_rule|.2f}} |

The slot rule fits better by {{7.allocation_slot_over_card_nats|.1f}} nats. With no pals it lies inside the observed interval and the card rule
outside; with two pals both predict more final links than seen, the card rule further. Exact consequences with Binomial(21, 0.35), P(reach link 1 / 2 / 3):

| Deck | Slots (adopted) | Cards |
|---|---|---|
| two 3-link SSRs | {{7.two_3_link_ssrs_slots_rule_p_reach_link_1|.2f}} / {{7.two_3_link_ssrs_slots_rule_p_reach_link_2|.2f}} / {{7.two_3_link_ssrs_slots_rule_p_reach_link_3|.2f}} | {{7.two_3_link_ssrs_cards_rule_p_reach_link_1|.2f}} / {{7.two_3_link_ssrs_cards_rule_p_reach_link_2|.2f}} / {{7.two_3_link_ssrs_cards_rule_p_reach_link_3|.2f}} |
| 3-link SSR + 2-link SR | SSR {{7.3_link_ssr_2_link_sr_ssr_slots_rule_p_reach_link_1|.2f}} / {{7.3_link_ssr_2_link_sr_ssr_slots_rule_p_reach_link_2|.2f}} / {{7.3_link_ssr_2_link_sr_ssr_slots_rule_p_reach_link_3|.2f}}, SR {{7.3_link_ssr_2_link_sr_sr_slots_rule_p_reach_link_1|.2f}} / {{7.3_link_ssr_2_link_sr_sr_slots_rule_p_reach_link_2|.2f}} | SSR {{7.3_link_ssr_2_link_sr_ssr_cards_rule_p_reach_link_1|.2f}} / {{7.3_link_ssr_2_link_sr_ssr_cards_rule_p_reach_link_2|.2f}} / {{7.3_link_ssr_2_link_sr_ssr_cards_rule_p_reach_link_3|.2f}}, SR {{7.3_link_ssr_2_link_sr_sr_cards_rule_p_reach_link_1|.2f}} / {{7.3_link_ssr_2_link_sr_sr_cards_rule_p_reach_link_2|.2f}} |
| three 3-link SSRs | {{7.three_3_link_ssrs_slots_rule_p_reach_link_1|.2f}} / {{7.three_3_link_ssrs_slots_rule_p_reach_link_2|.2f}} / {{7.three_3_link_ssrs_slots_rule_p_reach_link_3|.2f}} | {{7.three_3_link_ssrs_cards_rule_p_reach_link_1|.2f}} / {{7.three_3_link_ssrs_cards_rule_p_reach_link_2|.2f}} / {{7.three_3_link_ssrs_cards_rule_p_reach_link_3|.2f}} |
| four 3-link SSRs | {{7.four_3_link_ssrs_slots_rule_p_reach_link_1|.2f}} / {{7.four_3_link_ssrs_slots_rule_p_reach_link_2|.2f}} / {{7.four_3_link_ssrs_slots_rule_p_reach_link_3|.2f}} | {{7.four_3_link_ssrs_cards_rule_p_reach_link_1|.2f}} / {{7.four_3_link_ssrs_cards_rule_p_reach_link_2|.2f}} / {{7.four_3_link_ssrs_cards_rule_p_reach_link_3|.2f}} |

(With two 3-link cards and exactly 3 events, the slot rule gives $(3,0)$ with $\tfrac36 \cdot \tfrac25 \cdot \tfrac14 = 5\%$,
the card rule $(\tfrac12)^3 = 12.5\%$.)

**Known misses.** Two-chain decks (two 3-link SSRs, four pals) finish both chains in {{7.two_chain_decks_both_chains_finish|.0f}} of {{7.two_chain_decks_runs|.0f}} runs, {{7.two_chain_decks_share|.2f}}
[{{7.two_chain_decks_95_pct_low|.2f}}, {{7.two_chain_decks_95_pct_high|.2f}}], against {{7.two_chain_decks_predicted_p_n_ge_6|.2f}} predicted: within the interval. Early links in crowded decks run short; per run as a cluster, $z = \sum(\text{obs} - \text{pred}) / \sqrt{\sum(\text{obs} - \text{pred})^2}$:

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Early-link shortfall | {{7.early_link_shortfall_4_pals|.2f}} ± {{7.early_link_shortfall_4_pals_pm|.2f}} | {{7.early_link_shortfall_3_pals|.2f}} ± {{7.early_link_shortfall_3_pals_pm|.2f}} | {{7.early_link_shortfall_2_pals|.2f}} ± {{7.early_link_shortfall_2_pals_pm|.2f}} | {{7.early_link_shortfall_1_pals|.2f}} ± {{7.early_link_shortfall_1_pals_pm|.2f}} | {{7.early_link_shortfall_0_pals|.2f}} ± {{7.early_link_shortfall_0_pals_pm|.2f}} |
| $z$ | {{7.early_link_shortfall_4_pals_z|.1f}} | {{7.early_link_shortfall_3_pals_z|.1f}} | {{7.early_link_shortfall_2_pals_z|.1f}} | {{7.early_link_shortfall_1_pals_z|.1f}} | {{7.early_link_shortfall_0_pals_z|.1f}} |

Final links fit when pooled ({{7.final_links_by_card_pooled_observed|.3f}} observed against {{7.final_links_by_card_pooled_model|.3f}}) but individual cards differ
from the model far beyond chance: over the {{7.final_links_by_card_cards|.0f}} cards with 60 or more final-link observations the squared
standardised differences sum to {{7.final_links_by_card_chi2|.0f}} (expected about {{7.final_links_by_card_cards|.0f}}). The cause is not known.

More chains never start in crowded decks than predicted: {{7.never_started_5_chain_cards_observed|.2f}} against {{7.never_started_5_chain_cards_model|.2f}} with five chain cards, {{7.never_started_6_chain_cards_observed|.2f}} against {{7.never_started_6_chain_cards_model|.2f}} with
six. A per-card activation parameter (a chain card active with probability $a$), fitted jointly with $N$, gains
{{7.activation_corrected_reading_joint_with_n_gain_nats|.1f}} nats ($a = {{7.activation_corrected_reading_joint_with_n_a|.2f}}$): rejected.

## 8. Event choices and outcomes

Computed by `research/s08_outcomes.py` (the certainty checks: `research/s12_calibration.py`).

**Choice rule** (**assumed**): IT takes an event's top choice unless a prioritised skill (up to 10, set at career start)
sits on another choice, and the player is assumed to prioritise every target hint the model counts. Where targets sit
on different choices, a deck entry takes one choice (the solver offers one entry per useful choice, and an entry that
ends a chain early).

Evidence on the rule. Sasami Anshinzawa [This Might Sting!]'s finale: with Risky Business prioritised IT took choice 2
in about {{8.anshinzawa_rb_prioritised_took_choice_2|.0%}} of runs ({{8.anshinzawa_choice_1_exceptions_with_rb_at_lv_5|.0f}} of the {{8.anshinzawa_choice_1_exceptions_rb_prioritised|.0f}} exceptions with Risky Business already at hint Lv 5); with neither skill prioritised, {{8.anshinzawa_neither_prioritised_choice_2_won|.0f}} of
{{8.anshinzawa_neither_prioritised_runs|.0f}} runs took choice 2. Riko Kashimoto [Planned Perfection]: with Lone Wolf prioritised IT took the bottom choice
in {{8.riko_lone_wolf_prioritised_bottom_choice|.0f}} of {{8.riko_lone_wolf_prioritised_runs|.0f}} runs. Every run has a priority list, but the run records show it for only {{8.runs_showing_a_priority_list|.0f}} of {{8.runs|,.0f}} (the rest count as
unknown, never as an empty list), so the rule is tested mainly through its consequences (section 12: prioritised targets
{{8.event_only_hints_prioritised_targets_obs_per_pred|.2f}}, non-prioritised {{8.event_only_hints_non_prioritised_targets_obs_per_pred|.2f}} on event-only hints).

**Outcomes** (**assumed** unless measured): one choice's outcomes exclude each other, one per firing, equally likely
unless measured (outcome odds are server-side: master.mdb holds none); a card's gold and white of one skill are
combined exactly. Measured shares (90% intervals; counted only where every outcome could show:
her hint credited, the skill's level from other sources below 5):

| Event, outcome | Observed | Adopted |
|---|---|---|
| Sasami Anshinzawa [This Might Sting!] finale, Nothing Ventured (Risky Business prioritised; {{8.anshinzawa_runs_with_the_card_no_festa|.0f}} runs with the card) | {{8.anshinzawa_nv_with_rb_prioritised|.0f}}/{{8.anshinzawa_rb_prioritised_runs|.0f}} = {{8.anshinzawa_nv_share|.2f}} [{{8.anshinzawa_nv_share_90_pct_low|.2f}}, {{8.anshinzawa_nv_share_90_pct_high|.2f}}] | {{8.adopted_anshinzawa_finale_nothing_ventured|.2f}} |
| same, choice 2 alone, no inherited Risky Business | {{8.anshinzawa_choice_2_no_inherited_rb_won|.0f}}/{{8.anshinzawa_choice_2_no_inherited_rb_runs|.0f}} = {{8.anshinzawa_choice_2_share|.2f}} [{{8.anshinzawa_choice_2_90_pct_low|.2f}}, {{8.anshinzawa_choice_2_90_pct_high|.2f}}] | |
| Riko Kashimoto [Planned Perfection] "Unexpected Side", Ramp Up (top choice taken) | {{8.riko_ramp_up_top_choice_taken|.0f}}/{{8.riko_readings|.0f}} = {{8.riko_share|.2f}} [{{8.riko_90_pct_low|.2f}}, {{8.riko_90_pct_high|.2f}}] | {{8.adopted_riko_unexpected_side_ramp_up|.2f}} |
| Seiun Sky [Foolproof Plan] link 3, Fast-Paced | {{8.seiun_sky_link_3_fast_paced|.0f}}/{{8.seiun_sky_link_3_readings|.0f}} = {{8.seiun_sky_share|.2f}} [{{8.seiun_sky_90_pct_low|.2f}}, {{8.seiun_sky_90_pct_high|.2f}}] | {{8.adopted_seiun_sky_link_3_fast_paced|.2f}} |
| Smart Falcon [My Umadol Way! ☆] link 3, Center Stage | {{8.smart_falcon_link_3_center_stage|.0f}}/{{8.smart_falcon_link_3_readings|.0f}} = {{8.smart_falcon_share|.2f}} [{{8.smart_falcon_90_pct_low|.2f}}, {{8.smart_falcon_90_pct_high|.2f}}] | {{8.adopted_smart_falcon_link_3_center_stage|.2f}} |
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante", Nothing Ventured ({{8.festa_runs_with_the_card|.0f}} runs with the card, below) | {{8.festa_nv_wins|.0f}}/{{8.festa_readings|.0f}} = {{8.festa_share|.2f}} [{{8.festa_share_90_pct_low|.2f}}, {{8.festa_share_90_pct_high|.2f}}] | {{8.festa_adopted|.1f}} |

Anshinzawa's finale is counted on runs without Nakayama Festa [Trusting the Dice] (whose "Upping the Ante" also gives
Nothing Ventured +1/+3, credited jointly). Hint levels identify the choice: her Nothing Ventured +3 = choice 1, +1 =
choice 2; a loss on choice 2 shows as her Risky Business at level 2 (storyline choice 3 +1, finale +1). With inherited
Risky Business levels mixed in, choice 2 reads {{8.anshinzawa_choice_2_with_inherited_low|.2f}}-{{8.anshinzawa_choice_2_with_inherited_high|.2f}}. Choice 1 (Nothing Ventured +3 or nothing): {{8.anshinzawa_choice_1_won|.0f}}/{{8.anshinzawa_choice_1_runs|.0f}} = {{8.anshinzawa_choice_1_share|.2f}} if
the finale always fires; Grand Concert lower (unexplained). With Risky Business prioritised IT also takes the bottom
choice of her date-start event "A Pleasant Practice Partner?" (Risky Business +1; GameTora marks it 'ee', which outside
IT declines her dates), yet the finale still fires (at least {{8.anshinzawa_rb_prioritised_runs_showing_the_finale|.0f}} of {{8.anshinzawa_rb_prioritised_runs|.0f}} such runs show its outcome; her extra Risky
Business +1 appears in {{8.anshinzawa_her_extra_rb_in_choice_2_wins_prioritised|.0f}} of {{8.anshinzawa_choice_2_wins_prioritised|.0f}} choice-2 wins with the priority, {{8.anshinzawa_her_extra_rb_in_choice_2_wins_rb_not_prioritised|.0f}} of {{8.anshinzawa_choice_2_wins_rb_not_prioritised|.0f}} without): IT apparently keeps her dates either
way. The other outcomes: Riko Kashimoto's alternative is Maverick ○, Seiun Sky's Escape Artist, Smart Falcon's Prudent
Positioning.

Riko Kashimoto's {{8.adopted_riko_unexpected_side_ramp_up|.2f}} is a per-run chance: a run gets her Ramp Up hint in {{8.riko_per_run_runs_with_ramp_up|.0f}} of {{8.riko_per_run_runs|.0f}} runs, {{8.riko_per_run_share|.3f}} [{{8.riko_per_run_90_pct_low|.3f}}, {{8.riko_per_run_90_pct_high|.3f}}], whichever
choice IT took (the top choice in {{8.riko_firings_top_choice|.0f}} of {{8.riko_firings_all|.0f}} firings, {{8.riko_firings_top_choice_rate|.2f}}; the bottom choice, Lone Wolf, in {{8.riko_firings_bottom_choice|.0f}}).

Off 0.5 but left at the equal rule: Tazuna Hayakawa SSR finale Concentration {{8.tazuna_finale_concentration|.2f}}, Agnes Tachyon [Q≠0] link 1 choice 2
Murmur {{8.tachyon_link_1_choice_2_murmur|.2f}}, Marvelous Sunday [Dazzling Day in the Snow] link 3 Position Pilfer {{8.marvelous_sunday_link_3_position_pilfer|.2f}}, Light Hello SSR finale +3 See Ya
Later! {{8.light_hello_finale_see_ya_later_3|,.0f}} of {{8.light_hello_finale_readings|,.0f}} ({{8.light_hello_finale_share|.2f}}). Consistent with 0.5: Gold Ship [That Time I Became the Strongest] {{8.gold_ship_link_3_innate_experience|.2f}}, King Halo
[Tonight, We Waltz] {{8.king_halo_link_3_gap_closer|.2f}}, Daitaku Helios [Sunlit Angel ♪] Tail Held High {{8.helios_link_2_tail_held_high_given_link_3|.2f}} (link 2, on runs where link 3 visibly fired).

Nakayama Festa's share is measured on every run with her card ({{8.festa_runs_with_the_card|.0f}}). Her event is credited
to her alone with Nothing Ventured in {{8.festa_nv_wins|.0f}} runs and with Risky Business in {{8.festa_rb_wins|.0f}}. In {{8.festa_rb_shared_credits|.0f}} more, Risky Business is credited to
"her or another card's event"; the other card gives it alone in {{8.festa_team_sirius_alone_rb_rate|.3f}} of runs without her (Team Sirius, {{8.festa_shared_with_team_sirius|.0f}} of the {{8.festa_rb_shared_credits|.0f}})
or {{8.festa_anshinzawa_alone_rb_rate|.3f}} (Sasami Anshinzawa), she in {{8.festa_her_rb_rate|.3f}} of hers, so a shared credit is hers with chance
$r_F / (1 - (1 - r_F)(1 - r_o))$: {{8.festa_shared_credits_hers|.1f}} of the {{8.festa_rb_shared_credits|.0f}}. In {{8.festa_blank_risky_business_at_lv_5|.0f}} runs neither shows while Risky Business is already at hint Lv 5
(a Risky Business win cannot show, or the event never fired); they are split by the rate at which the event shows where
both outcomes can, {{8.festa_firing_rate_where_both_can_show|.2f}}. The share is then {{8.festa_nv_wins|.0f}}/{{8.festa_readings|.0f}} = {{8.festa_share|.2f}} [{{8.festa_share_90_pct_low|.2f}}, {{8.festa_share_90_pct_high|.2f}}]; it stays within {{8.festa_share_shared_all_hers|.3f}}-{{8.festa_share_shared_none_hers|.3f}} whatever share of
the shared credits is hers, and shows no dependence on the trainee's final Wit.

**Pal and group cards.** Their events are entered per card with a probability (finales certain); first-year date
events are left out in IT (**assumed**). Certainty checks: Light Hello [From the Ground Up] credits See Ya Later! to her
event in {{8.light_hello_credited_credited|.0f}} of {{8.light_hello_credited_runs_carrying_her|.0f}} runs carrying her; Team Sirius gives Risky Business in {{8.team_sirius_credited_credited|.0f}} of {{8.team_sirius_credited_runs_carrying_her|.0f}}.

## 9. Random card events (not modelled)

Computed by `research/s09_random_events.py`.

A support card's character's shared random event fires, per event per career with a card of hers in the deck (any
rarity, R cards included) and another uma training, with (**fitted**)

$$P(\text{fires}) = {{9.fitted_p_at_k_4|.2f}}\,(1 - {{9.fitted_form_1_ratio_per_event|.3f}})^{K - 4},$$

$K$ = random events on offer (every shared event of the deck's characters plus the trainee's own): about {{9.fitted_p_overall|.2f}} overall,
{{9.fitted_p_at_k_4|.2f}} at $K = 4$, {{9.fitted_p_at_k_14|.2f}} at $K = 14$. Readings: top-choice marker skills credited to the character's event; {{9.readings|,.0f}}
readings from {{9.runs|,.0f}} runs, {{9.clean_hits|.0f}} clean hits ({{9.ambiguous_credits|.0f}} ambiguous two-card credits counted as
misses; with them the rate is about {{9.rate_with_ambiguous_points_higher|.0f}} points higher). A reading misses a firing when the marker was already at hint Lv 5
from other sources ({{9.readings_where_the_event_could_not_show_marker_at_lv_5_from_other_sources|.0f}} readings): where the event could show, the rate is {{9.rate_where_the_event_could_show|.3f}}, or {{9.rate_where_it_could_show_ambiguous_as_hits|.3f}} counting the ambiguous
credits as hits. A firing is also missed when IT takes another choice for a prioritised skill (bottom-choice rates
below). The fitted rate is therefore a lower bound.

| Form | LL |
|---|---|
| constant {{9.constant_p|.3f}} | {{9.ll_constant|u.1f}} |
| ${{9.fitted_form_p_at_k_4|.2f}}(1 - {{9.fitted_form_1_ratio_per_event|.3f}})^{K-4}$ | {{9.ll_fitted_form|u.1f}} |
| power law $c(K/8)^{-a}$, $a = {{9.power_law_a|.2f}}$ (uniform competition would be $a = 1$) | {{9.ll_power_law|u.1f}} |
| strict competition $1 - (1 - 1/K)^S$, $S = {{9.strict_competition_s|.2f}}$ | {{9.ll_strict_competition|u.1f}} |
| every event slot competing, $c / T$ with $T = K + w \cdot \text{pals}$ (chain links weigh {{9.every_slot_competing_chain_link_weight|.0f}}), $c = {{9.every_slot_competing_c|.2f}}$ | {{9.ll_every_slot_competing|u.1f}} |

The slot form fits about as well as the adopted one ({{9.every_slot_competing_ll_over_constant|+.1f}} nats over constant for two extra parameters): chain links play
no part, and pals compete, a pal weighing $w$ = {{9.every_slot_competing_pal_weight|.2f}} [{{9.every_slot_competing_pal_weight_95_pct_low|.2f}}, {{9.every_slot_competing_pal_weight_95_pct_high|.2f}}] random events.

Observed hit rate by $K$ = 4 / 6 / 8 / 10 / 12 / 14: {{9.hit_rate_k_4|.2f}} / {{9.hit_rate_k_6|.2f}} / {{9.hit_rate_k_8|.2f}} / {{9.hit_rate_k_10|.2f}} / {{9.hit_rate_k_12|.2f}} / {{9.hit_rate_k_14|.2f}}; within a run events fire
independently. A logistic slope in $K$ gives {{9.logistic_rate_at_k_8|.1%}} at 8 on offer and {{9.logistic_rate_at_k_14|.1%}} at 14; friendship, specialty priority and
scenario show no effect distinguishable from zero, nor does rarity within a character (pooled, SSR markers fire at
{{9.pooled_ssr_marker_rate|.1%}} against SR {{9.pooled_sr_marker_rate|.1%}}, mostly which characters have SR cards).

Bottom-choice skills are rarely credited: in the {{9.runs_showing_a_priority_list|.0f}} runs showing a priority list, {{9.bottom_choice_markers_prioritised_there|.0f}} of {{9.bottom_choice_candidates_there|.0f}} candidates were
prioritised; without a priority they were credited in {{9.bottom_choice_not_prioritised_credited_rate|.3f}} [{{9.bottom_choice_not_prioritised_95_pct_low|.3f}}, {{9.bottom_choice_not_prioritised_95_pct_high|.3f}}] of runs, against {{9.top_choice_marker_rate|.3f}} for
top-choice markers ({{9.bottom_choice_prioritised_credited_to_her_event|.0f}} of {{9.bottom_choice_markers_prioritised_there|.0f}} with a priority).

**What leaving them out costs on real careers** (section 12's rows, each run's own deck; this test scores whether the
target was acquired from any source, so it does not depend on reading the event). Of the default targets only
two are a character's top-choice marker (Vodka's Nimble Navigator, Nice Nature's Ramp Up). On the {{9.calibration_rows|.0f}} run-target rows
where a deck card's random event could hint the target, it was credited to that event {{9.careers_acquired_through_that_event|.0f}} times, {{9.careers_acquired_through_that_event_only|.0f}} with no other
source; {{9.careers_rows_the_model_gives_no_chance|.0f}} rows get no chance from the model, and the event hinted the target on {{9.careers_of_them_acquired_through_that_event|.0f}} of them. Left out,
acquisition there reads {{9.calibration_obs_per_pred_left_out|.2f}} [{{9.calibration_ratio_95_pct_low_left_out|.2f}}, {{9.calibration_ratio_95_pct_high_left_out|.2f}}] of the model, an under-prediction; credited at the fitted rate
(about {{9.careers_mean_fitted_rate_there|.2f}} there) it reads {{9.calibration_obs_per_pred_credited|.2f}} [{{9.calibration_ratio_95_pct_low_credited|.2f}}, {{9.calibration_ratio_95_pct_high_credited|.2f}}] and the fit improves by {{9.calibration_ll_credited_minus_left_out_positive_rows|.1f}} nats
({{9.calibration_rate_held_out_ll_credited_minus_left_out_positive_rows|.1f}} with the rate fitted without these rows' readings). Crediting raises an affected run's odds over the targets it
can land by a median factor {{9.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_median|.2f}} (at most {{9.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_max|.2f}}; {{9.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_runs|.0f}} runs). The trainee's own random event can be told
apart only for Nice Nature: {{9.careers_trainee_s_own_acquired_through_her_events|.0f}} of {{9.careers_rows_of_the_trainee_s_own_event_identifiable|.0f}} rows. The model leaves these events out; the data say it under-predicts
the two affected targets on decks with those characters.

## 10. Conditional character events (not modelled)

Computed by `research/s10_conditional.py`.

Secret and race-result events: {{10.events_with_hints|.0f}} events with hints for {{10.umas_with_a_hint_event|.0f}} umas ({{10.events_with_hints_global|.0f}} for {{10.umas_with_a_hint_event_global|.0f}} umas in Global), {{10.events_touching_a_default_target|.0f}} touching the default
targets (Nimble Navigator, Ramp Up, Tail Held High), {{10.of_them_giving_a_gold|.0f}} of them giving a gold.

**On real careers.** Where only a conditional event can give the target (none of the trainee's story, date, outfit or
shared random events, nor the scenario's, can), it did so on {{10.careers_conditional_only_from_her_events|.0f}} of {{10.careers_conditional_only_rows|.0f}} rows, {{10.careers_conditional_only_rate|.3f}} [{{10.careers_conditional_only_rate_95_pct_low|.3f}}, {{10.careers_conditional_only_rate_95_pct_high|.3f}}]:
Daiwa Scarlet, any outfit, {{10.daiwa_scarlet_any_outfit_nimble_navigator_from_her_events|.0f}} of {{10.daiwa_scarlet_any_outfit_nimble_navigator_rows|.0f}} (Nimble Navigator); Mejiro Ardan, any outfit, {{10.mejiro_ardan_any_outfit_ramp_up_from_her_events|.0f}} of {{10.mejiro_ardan_any_outfit_ramp_up_rows|.0f}} (Ramp Up). Left
out, acquisition there reads {{10.calibration_conditional_only_obs_per_pred_left_out|.2f}} [{{10.calibration_conditional_only_ratio_95_pct_low_left_out|.2f}}, {{10.calibration_conditional_only_ratio_95_pct_high_left_out|.2f}}] of the model, and crediting the measured rates gains only
{{10.calibration_conditional_only_ll_credited_minus_left_out_positive_rows|.1f}} nats. The exception is Vodka, any outfit, whose own events give her Nimble Navigator in
{{10.vodka_any_outfit_nimble_navigator_from_her_events|.0f}} of {{10.vodka_any_outfit_nimble_navigator_rows|.0f}} runs ([{{10.vodka_any_outfit_nimble_navigator_rate_95_pct_low|.2f}}, {{10.vodka_any_outfit_nimble_navigator_rate_95_pct_high|.2f}}]; her random and conditional events both give it and cannot be told
apart): with her rows, the model left as it is reads {{10.calibration_obs_per_pred_left_out|.2f}} [{{10.calibration_ratio_95_pct_low_left_out|.2f}}, {{10.calibration_ratio_95_pct_high_left_out|.2f}}] and crediting gains
{{10.calibration_ll_credited_minus_left_out_positive_rows|.1f}} nats. Not modelled, so the model under-predicts Vodka's Nimble Navigator.

One exception is modelled as always acquired: Tokai Teio's Head-On (all three of its races are her career goals; the
goals ask only for top 5, top 5 and top 3, but IT won all three in {{10.of_them_winning_all_three|.0f}} of {{10.runs_training_tokai_teio|.0f}} runs training her, and every run
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

Each of {{12.runs_used|,.0f}} runs (URA Finale {{12.ura_finale_runs|.0f}}, Unity Cup {{12.unity_cup_runs|.0f}}, Grand Concert {{12.grand_concert_runs|,.0f}}, Trackblazer
{{12.trackblazer_runs|.0f}}) was rebuilt as a career: trainee in her outfit, scenario, the six cards at their own limit breaks (hint frequency
and increment at that limit break), both parents with their recorded parents and every ancestor's white sparks, and
race wins giving the recorded shared-G1 counts. Targets: the default eight; {{12.run_target_rows|,.0f}} run-target rows, of which {{12.rows_on_her_outfit|,.0f}} on
the trainee's outfit and {{12.scenario_gold_rows|,.0f}} scenario golds sit outside tests 1 and 3. Intervals: Wilson 95% on the observed rate;
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
| 1. Deck hint, no inspiration | {{12.1_deck_hint_no_inspiration_rows|,.0f}} | {{12.1_deck_hint_no_inspiration_predicted|.3f}} | {{12.1_deck_hint_no_inspiration_observed|.3f}} [{{12.1_deck_hint_observed_95_pct_low|.3f}}, {{12.1_deck_hint_observed_95_pct_high|.3f}}] | {{12.1_deck_hint_no_inspiration_obs_per_pred|.3f}} [{{12.1_deck_hint_no_inspiration_ratio_95_pct_low|.2f}}, {{12.1_deck_hint_no_inspiration_ratio_95_pct_high|.2f}}] |
| 1. Training pool only | {{12.1_training_pool_only_rows|,.0f}} | {{12.1_training_pool_only_predicted|.3f}} | {{12.1_training_pool_only_observed|.3f}} | {{12.1_training_pool_only_obs_per_pred|.3f}} [{{12.1_training_pool_only_ratio_95_pct_low|.2f}}, {{12.1_training_pool_only_ratio_95_pct_high|.2f}}] |
| 1. Events only | {{12.1_events_only_rows|,.0f}} | {{12.1_events_only_predicted|.3f}} | {{12.1_events_only_observed|.3f}} | {{12.1_events_only_obs_per_pred|.3f}} [{{12.1_events_only_ratio_95_pct_low|.2f}}, {{12.1_events_only_ratio_95_pct_high|.2f}}] |
| 1. Events only, target prioritised | {{12.1_events_only_prioritised_rows|.0f}} | {{12.1_events_only_prioritised_predicted|.3f}} | {{12.1_events_only_prioritised_observed|.3f}} | {{12.1_events_only_prioritised_obs_per_pred|.3f}} [{{12.1_events_only_prioritised_ratio_95_pct_low|.2f}}, {{12.1_events_only_prioritised_ratio_95_pct_high|.2f}}] |
| 1. Events only, not prioritised | {{12.1_events_only_not_prioritised_rows|.0f}} | {{12.1_events_only_not_prioritised_predicted|.3f}} | {{12.1_events_only_not_prioritised_observed|.3f}} | {{12.1_events_only_not_prioritised_obs_per_pred|.3f}} [{{12.1_events_only_not_prioritised_ratio_95_pct_low|.2f}}, {{12.1_events_only_not_prioritised_ratio_95_pct_high|.2f}}] |
| 2. Inheritance | {{12.2_inheritance_rows|,.0f}} | {{12.2_inheritance_predicted|.3f}} | {{12.2_inheritance_observed|.3f}} [{{12.2_inheritance_observed_95_pct_low|.3f}}, {{12.2_inheritance_observed_95_pct_high|.3f}}] | {{12.2_inheritance_obs_per_pred|.3f}} [{{12.2_inheritance_ratio_95_pct_low|.2f}}, {{12.2_inheritance_ratio_95_pct_high|.2f}}] |
| 3. Acquired, any source | {{12.3_acquired_any_source_rows|,.0f}} | {{12.3_acquired_any_source_predicted|.3f}} | {{12.3_acquired_any_source_observed|.3f}} [{{12.3_acquired_any_source_observed_95_pct_low|.3f}}, {{12.3_acquired_any_source_observed_95_pct_high|.3f}}] | {{12.3_acquired_any_source_obs_per_pred|.3f}} [{{12.3_acquired_any_source_ratio_95_pct_low|.2f}}, {{12.3_acquired_any_source_ratio_95_pct_high|.2f}}] |
| 3. Acquired, deck or inspiration only | {{12.3_acquired_deck_or_inspiration_only_rows|,.0f}} | {{12.3_acquired_deck_or_inspiration_only_predicted|.3f}} | {{12.3_acquired_deck_or_inspiration_only_observed|.3f}} | {{12.3_acquired_deck_or_inspiration_only_obs_per_pred|.3f}} [{{12.3_acquired_deck_or_inspiration_only_ratio_95_pct_low|.2f}}, {{12.3_acquired_deck_or_inspiration_only_ratio_95_pct_high|.2f}}] |
| 3. Acquired, prioritised | {{12.3_acquired_prioritised_rows|,.0f}} | {{12.3_acquired_prioritised_predicted|.3f}} | {{12.3_acquired_prioritised_observed|.3f}} | {{12.3_acquired_prioritised_obs_per_pred|.3f}} [{{12.3_acquired_prioritised_ratio_95_pct_low|.2f}}, {{12.3_acquired_prioritised_ratio_95_pct_high|.2f}}] |
| 3. Acquired, not prioritised | {{12.3_acquired_not_prioritised_rows|,.0f}} | {{12.3_acquired_not_prioritised_predicted|.3f}} | {{12.3_acquired_not_prioritised_observed|.3f}} | {{12.3_acquired_not_prioritised_obs_per_pred|.3f}} [{{12.3_acquired_not_prioritised_ratio_95_pct_low|.2f}}, {{12.3_acquired_not_prioritised_ratio_95_pct_high|.2f}}] |
| 4. Landing (sums of $p$) | {{12.4_landing_rows|,.0f}} | {{12.4_landing_sum_of_p|,.1f}} | {{12.4_landing_sum_observed_equivalent|,.1f}} | {{12.4_landing_ratio|.3f}} |

The prioritised / not prioritised rows use the runs that show a priority list.

**Verdict.** The composed acquisition odds are calibrated ({{12.3_acquired_any_source_obs_per_pred|.3f}}). Inheritance is right (one target of eight, Tail Held High, reads {{12.tail_held_high_inheritance|.3f}} [{{12.tail_held_high_inheritance_95_pct_low|.2f}}, {{12.tail_held_high_inheritance_95_pct_high|.2f}}]). Deck hints read optimistic ({{12.1_deck_hint_no_inspiration_obs_per_pred|.3f}} of the model), almost all from the
priority assumption: event hints land at {{12.1_events_only_prioritised_obs_per_pred|.2f}} × predicted for prioritised targets and {{12.1_events_only_not_prioritised_obs_per_pred|.2f}} for others. Unmodelled scenario and
trainee events supply the few points the deck loses ({{12.acquisitions_from_unmodelled_events|.0f}} target acquisitions in {{12.3_acquired_any_source_rows|,.0f}} rows, {{12.acquisitions_from_unmodelled_events_share_of_rows|.1%}}).

Per target (obs / pred):

| Target | Deck hint | Inheritance | Acquired | Landing |
|---|---|---|---|---|
| Uma Stan / Superstan | {{12.uma_stan_deck_hint|.2f}} [{{12.uma_stan_deck_hint_95_pct_low|.2f}}, {{12.uma_stan_deck_hint_95_pct_high|.2f}}] | {{12.uma_stan_inheritance|.3f}} | {{12.uma_stan_acquired|.3f}} | {{12.uma_stan_landing|.3f}} |
| Nimble Navigator / No Stopping Me! | {{12.nimble_navigator_deck_hint|.2f}} [{{12.nimble_navigator_deck_hint_95_pct_low|.2f}}, {{12.nimble_navigator_deck_hint_95_pct_high|.2f}}] | {{12.nimble_navigator_inheritance|.3f}} | {{12.nimble_navigator_acquired|.3f}} | {{12.nimble_navigator_landing|.3f}} |
| Playtime's Over! / See Ya Later! | {{12.playtime_s_over_deck_hint|.3f}} [{{12.playtime_s_over_deck_hint_95_pct_low|.2f}}, {{12.playtime_s_over_deck_hint_95_pct_high|.2f}}] | {{12.playtime_s_over_inheritance|.3f}} | {{12.playtime_s_over_acquired|.3f}} | {{12.playtime_s_over_landing|.3f}} |
| Ramp Up / It's On! | {{12.ramp_up_deck_hint|.3f}} [{{12.ramp_up_deck_hint_95_pct_low|.2f}}, {{12.ramp_up_deck_hint_95_pct_high|.2f}}] | {{12.ramp_up_inheritance|.3f}} | {{12.ramp_up_acquired|.3f}} [{{12.ramp_up_acquired_95_pct_low|.2f}}, {{12.ramp_up_acquired_95_pct_high|.2f}}] | {{12.ramp_up_landing|.3f}} |
| Tail Held High / Tail Nine | {{12.tail_held_high_deck_hint|.3f}} [{{12.tail_held_high_deck_hint_95_pct_low|.2f}}, {{12.tail_held_high_deck_hint_95_pct_high|.2f}}] | {{12.tail_held_high_inheritance|.3f}} [{{12.tail_held_high_inheritance_95_pct_low|.2f}}, {{12.tail_held_high_inheritance_95_pct_high|.2f}}] | {{12.tail_held_high_acquired|.3f}} | {{12.tail_held_high_landing|.3f}} |
| Racing Spirit: Stamina | no deck source | {{12.racing_spirit_stamina_inheritance|.3f}} | {{12.racing_spirit_stamina_acquired|.3f}} | {{12.racing_spirit_stamina_landing|.3f}} |
| Slipstream / Perfect Spot! | {{12.slipstream_deck_hint|.3f}} [{{12.slipstream_deck_hint_95_pct_low|.2f}}, {{12.slipstream_deck_hint_95_pct_high|.2f}}] | {{12.slipstream_inheritance|.3f}} | {{12.slipstream_acquired|.3f}} | {{12.slipstream_landing|.3f}} |
| Risky Business / Nothing Ventured | {{12.risky_business_deck_hint|.3f}} [{{12.risky_business_deck_hint_95_pct_low|.2f}}, {{12.risky_business_deck_hint_95_pct_high|.2f}}] | {{12.risky_business_inheritance|.3f}} | {{12.risky_business_acquired|.3f}} [{{12.risky_business_acquired_95_pct_low|.2f}}, {{12.risky_business_acquired_95_pct_high|.2f}}] | {{12.risky_business_landing|.3f}} |

Training-pool hints by scenario (tests $\beta$ and $d$): Grand Concert {{12.1_training_pool_only_grand_concert_obs_per_pred|.3f}} [{{12.1_training_pool_only_grand_concert_ratio_95_pct_low|.2f}}, {{12.1_training_pool_only_grand_concert_ratio_95_pct_high|.2f}}], Unity Cup {{12.1_training_pool_only_unity_cup_obs_per_pred|.3f}} [{{12.1_training_pool_only_unity_cup_ratio_95_pct_low|.2f}},
{{12.1_training_pool_only_unity_cup_ratio_95_pct_high|.2f}}], URA Finale {{12.1_training_pool_only_ura_finale_obs_per_pred|.3f}} [{{12.1_training_pool_only_ura_finale_ratio_95_pct_low|.2f}}, {{12.1_training_pool_only_ura_finale_ratio_95_pct_high|.2f}}], Trackblazer {{12.1_training_pool_only_trackblazer_obs_per_pred|.3f}} [{{12.1_training_pool_only_trackblazer_ratio_95_pct_low|.2f}}, {{12.1_training_pool_only_trackblazer_ratio_95_pct_high|.2f}}]: only Trackblazer's interval excludes 1, its
$\beta$ over-predicting these runs. By the lowest limit break of the sourcing cards: LB4 {{12.1_training_pool_only_lowest_lb_4|.3f}}, LB3 {{12.1_training_pool_only_lowest_lb_3|.3f}}, LB0-2 {{12.1_training_pool_only_lb0_2_lowest_ratio|.2f}}-{{12.1_training_pool_only_lb0_2_highest_ratio|.2f}} ({{12.1_training_pool_only_lb0_2_fewest_rows|.0f}}-{{12.1_training_pool_only_lb0_2_most_rows|.0f}} rows each, every interval
covering 1). Unity Cup's guaranteed golds: {{12.unity_cup_s_guaranteed_golds_runs_with_both|.0f}} of {{12.unity_cup_runs|.0f}} runs
each.

Gold acquisition (obs / pred): Nothing Ventured {{12.gold_acquisition_nothing_ventured|.3f}} [{{12.gold_acquisition_nothing_ventured_95_pct_low|.2f}}, {{12.gold_acquisition_nothing_ventured_95_pct_high|.2f}}] ({{12.gold_acquisition_nothing_ventured_rows|.0f}} rows), Perfect Spot! {{12.gold_acquisition_perfect_spot|.3f}} [{{12.gold_acquisition_perfect_spot_95_pct_low|.2f}}, {{12.gold_acquisition_perfect_spot_95_pct_high|.2f}}] ({{12.gold_acquisition_perfect_spot_rows|.0f}}), It's
On! outside Unity Cup {{12.gold_acquisition_it_s_on|.3f}} [{{12.gold_acquisition_it_s_on_95_pct_low|.2f}}, {{12.gold_acquisition_it_s_on_95_pct_high|.2f}}] ({{12.gold_acquisition_it_s_on_rows|.0f}}), No Stopping Me! outside Unity Cup {{12.gold_acquisition_no_stopping_me|.3f}} ({{12.gold_acquisition_no_stopping_me_rows|.0f}}), See Ya Later! {{12.gold_acquisition_see_ya_later|.3f}}
({{12.gold_acquisition_see_ya_later_rows|,.0f}}).

Reliability, deck hint and acquired (observed, mean prediction in brackets):

| Predicted | Deck hint: rows, observed | Acquired: rows, observed |
|---|---|---|
| 0 | {{12.reliability_deck_hint_0_rows|,.0f}}, {{12.reliability_deck_hint_0_observed|.3f}} | {{12.reliability_acquired_0_rows|,.0f}}, {{12.reliability_acquired_0_observed|.3f}} |
| above 0 to 0.20 | {{12.reliability_deck_hint_0_00_0_20_rows|.0f}}, {{12.reliability_deck_hint_0_00_0_20_observed|.3f}} ({{12.reliability_deck_hint_0_00_0_20_mean_predicted|.2f}}) | {{12.reliability_acquired_0_00_0_20_rows|,.0f}}, {{12.reliability_acquired_0_00_0_20_observed|.3f}} ({{12.reliability_acquired_0_00_0_20_mean_predicted|.2f}}) |
| 0.20-0.40 | {{12.reliability_deck_hint_0_20_0_40_rows|,.0f}}, {{12.reliability_deck_hint_0_20_0_40_observed|.3f}} ({{12.reliability_deck_hint_0_20_0_40_mean_predicted|.2f}}) | {{12.reliability_acquired_0_20_0_40_rows|,.0f}}, {{12.reliability_acquired_0_20_0_40_observed|.3f}} ({{12.reliability_acquired_0_20_0_40_mean_predicted|.2f}}) |
| 0.40-0.60 | {{12.reliability_deck_hint_0_40_0_60_rows|,.0f}}, {{12.reliability_deck_hint_0_40_0_60_observed|.3f}} ({{12.reliability_deck_hint_0_40_0_60_mean_predicted|.2f}}) | {{12.reliability_acquired_0_40_0_60_rows|,.0f}}, {{12.reliability_acquired_0_40_0_60_observed|.3f}} ({{12.reliability_acquired_0_40_0_60_mean_predicted|.2f}}) |
| 0.60-0.80 | {{12.reliability_deck_hint_0_60_0_80_rows|.0f}}, {{12.reliability_deck_hint_0_60_0_80_observed|.3f}} ({{12.reliability_deck_hint_0_60_0_80_mean_predicted|.2f}}) | {{12.reliability_acquired_0_60_0_80_rows|,.0f}}, {{12.reliability_acquired_0_60_0_80_observed|.3f}} ({{12.reliability_acquired_0_60_0_80_mean_predicted|.2f}}) |
| 0.80-0.95 | {{12.reliability_deck_hint_0_80_0_95_rows|.0f}}, {{12.reliability_deck_hint_0_80_0_95_observed|.3f}} ({{12.reliability_deck_hint_0_80_0_95_mean_predicted|.2f}}) | {{12.reliability_acquired_0_80_0_95_rows|,.0f}}, {{12.reliability_acquired_0_80_0_95_observed|.3f}} ({{12.reliability_acquired_0_80_0_95_mean_predicted|.2f}}) |
| 0.95-1 | {{12.reliability_deck_hint_0_95_1_00_rows|,.0f}}, {{12.reliability_deck_hint_0_95_1_00_observed|.3f}} ({{12.reliability_deck_hint_0_95_1_00_mean_predicted|.2f}}) | {{12.reliability_acquired_0_95_1_00_rows|,.0f}}, {{12.reliability_acquired_0_95_1_00_observed|.3f}} ({{12.reliability_acquired_0_95_1_00_mean_predicted|.2f}}) |

The deck's over-prediction sits in its top buckets, the event rewards: those predicted 0.6 and above hold {{12.deck_shortfall_in_buckets_predicted_0_6_and_above|.0%}} of the
shortfall, and {{12.their_rows_with_an_event_source|.0%}} of their rows have an event source. The 0.20-0.40 bucket, almost all training-pool rows, is within
its interval ({{12.reliability_deck_hint_0_20_0_40_observed_sentence|.3f}} [{{12.reliability_deck_hint_0_20_0_40_observed_95_pct_low|.3f}}, {{12.reliability_deck_hint_0_20_0_40_observed_95_pct_high|.3f}}] against {{12.reliability_deck_hint_0_20_0_40_mean_predicted_three_places|.3f}}).

**Per run** ({{12.per_run_open_targets|.2f}} open targets each): targets acquired, expected {{12.per_run_acquired_expected|.3f}}, observed {{12.per_run_acquired_observed|.3f}}; all open targets acquired,
product of the model's odds {{12.per_run_all_open_acquired_product_of_odds|.3f}}, observed {{12.per_run_all_open_acquired_observed|.3f}} [{{12.per_run_all_open_acquired_95_pct_low|.3f}}, {{12.per_run_all_open_acquired_95_pct_high|.3f}}]. Landing $P(\text{all eight})$ recomputed from what
each run acquired against the solver's exact $P(\text{all})$: {{12.p_all_eight_ratio_all|.3f}} [{{12.p_all_eight_ratio_all_95_pct_low|.2f}}, {{12.p_all_eight_ratio_all_95_pct_high|.2f}}] (bootstrap over runs; Unity Cup {{12.p_all_eight_ratio_unity_cup|.3f}}
[{{12.p_all_eight_ratio_unity_cup_95_pct_low|.2f}}, {{12.p_all_eight_ratio_unity_cup_95_pct_high|.2f}}], Grand Concert {{12.p_all_eight_ratio_grand_concert|.2f}} [{{12.p_all_eight_ratio_grand_concert_95_pct_low|.2f}}, {{12.p_all_eight_ratio_grand_concert_95_pct_high|.2f}}], URA Finale {{12.p_all_eight_ratio_ura_finale|.2f}} [{{12.p_all_eight_ratio_ura_finale_95_pct_low|.2f}}, {{12.p_all_eight_ratio_ura_finale_95_pct_high|.2f}}]; Trackblazer's model value is {{12.trackblazer_s_model_p_all|.0f}}). Consistent
with calibrated, though the interval allows up to about {{12.p_all_eight_optimism_the_interval_allows_1_per_95_pct_low|.1f}}× optimism (1/{{12.p_all_eight_ratio_all_95_pct_low|.2f}}). Only {{12.runs_acquiring_every_open_target|.0f}} runs acquired every open
target, too few to separate the exact joint from the product.

Single-source cards furthest from the model (every pair of 40 or more rows whose interval excludes 1, and Gold
Ship's):

| Card > target | Pred | Obs | Ratio | Rows | Likely reason |
|---|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | {{12.this_might_sting_sasami_anshinzawa_gt_risky_business_predicted|.3f}} | {{12.this_might_sting_sasami_anshinzawa_gt_risky_business_observed|.3f}} | {{12.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio|.2f}} [{{12.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio_95_pct_low|.2f}}, {{12.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio_95_pct_high|.2f}}] | {{12.this_might_sting_sasami_anshinzawa_gt_risky_business_rows|.0f}} | model assumes Risky Business prioritised (finale choice 2) |
| Riko Kashimoto [Planned Perfection] > Ramp Up | {{12.planned_perfection_riko_kashimoto_gt_ramp_up_predicted|.3f}} | {{12.planned_perfection_riko_kashimoto_gt_ramp_up_observed|.3f}} | {{12.planned_perfection_riko_kashimoto_gt_ramp_up_ratio|.2f}} [{{12.planned_perfection_riko_kashimoto_gt_ramp_up_ratio_95_pct_low|.2f}}, {{12.planned_perfection_riko_kashimoto_gt_ramp_up_ratio_95_pct_high|.2f}}] | {{12.planned_perfection_riko_kashimoto_gt_ramp_up_rows|.0f}} | open (her Lone Wolf choice when prioritised accounts for part) |
| King Halo [Tonight, We Waltz] > Uma Stan | {{12.tonight_we_waltz_king_halo_gt_uma_stan_predicted|.3f}} | {{12.tonight_we_waltz_king_halo_gt_uma_stan_observed|.3f}} | {{12.tonight_we_waltz_king_halo_gt_uma_stan_ratio|.2f}} [{{12.tonight_we_waltz_king_halo_gt_uma_stan_ratio_95_pct_low|.2f}}, {{12.tonight_we_waltz_king_halo_gt_uma_stan_ratio_95_pct_high|.2f}}] | {{12.tonight_we_waltz_king_halo_gt_uma_stan_rows|.0f}} | Uma Stan sits on a choice that needs a priority |
| Gold Ship [That Time I Became the Strongest] > Uma Stan | {{12.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_predicted|.3f}} | {{12.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_observed|.3f}} | {{12.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_ratio|.2f}} [{{12.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_ratio_95_pct_low|.2f}}, {{12.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_ratio_95_pct_high|.2f}}] | {{12.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_rows|.0f}} | open |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | {{12.cute_cute_mayano_top_gun_gt_nimble_navigator_predicted|.3f}} | {{12.cute_cute_mayano_top_gun_gt_nimble_navigator_observed|.3f}} | {{12.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio|.2f}} [{{12.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio_95_pct_low|.2f}}, {{12.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio_95_pct_high|.2f}}] | {{12.cute_cute_mayano_top_gun_gt_nimble_navigator_rows|.0f}} | open |
| Maruzensky [Sentimental Flare ♪] > Slipstream | {{12.sentimental_flare_maruzensky_gt_slipstream_predicted|.3f}} | {{12.sentimental_flare_maruzensky_gt_slipstream_observed|.3f}} | {{12.sentimental_flare_maruzensky_gt_slipstream_ratio|.2f}} [{{12.sentimental_flare_maruzensky_gt_slipstream_ratio_95_pct_low|.2f}}, {{12.sentimental_flare_maruzensky_gt_slipstream_ratio_95_pct_high|.2f}}] | {{12.sentimental_flare_maruzensky_gt_slipstream_rows|.0f}} | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | {{12.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_predicted|.3f}} | {{12.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_observed|.3f}} | {{12.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio|.2f}} [{{12.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio_95_pct_low|.2f}}, {{12.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio_95_pct_high|.2f}}] | {{12.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_rows|.0f}} | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | {{12.sunlit_angel_daitaku_helios_gt_slipstream_predicted|.3f}} | {{12.sunlit_angel_daitaku_helios_gt_slipstream_observed|.3f}} | {{12.sunlit_angel_daitaku_helios_gt_slipstream_ratio|.2f}} [{{12.sunlit_angel_daitaku_helios_gt_slipstream_ratio_95_pct_low|.2f}}, {{12.sunlit_angel_daitaku_helios_gt_slipstream_ratio_95_pct_high|.2f}}] | {{12.sunlit_angel_daitaku_helios_gt_slipstream_rows|.0f}} | open |
| Sweep Tosho [Lamplit Training of a Witch-to-Be] > Slipstream | {{12.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_predicted|.3f}} | {{12.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_observed|.3f}} | {{12.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_ratio|.2f}} [{{12.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_ratio_95_pct_low|.2f}}, {{12.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_ratio_95_pct_high|.3f}}] | {{12.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_rows|.0f}} | open |
| Nakayama Festa [Trusting the Dice] > Risky Business | {{12.trusting_the_dice_nakayama_festa_gt_risky_business_predicted|.3f}} | {{12.trusting_the_dice_nakayama_festa_gt_risky_business_observed|.3f}} | {{12.trusting_the_dice_nakayama_festa_gt_risky_business_ratio|.2f}} [{{12.trusting_the_dice_nakayama_festa_gt_risky_business_ratio_95_pct_low|.2f}}, {{12.trusting_the_dice_nakayama_festa_gt_risky_business_ratio_95_pct_high|.2f}}] | {{12.trusting_the_dice_nakayama_festa_gt_risky_business_rows|.0f}} | open |

Below 40 rows {{12.pairs_under_40_rows_excluding_1|.0f}} more pairs exclude 1; {{12.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources|.0f}} of them are event sources of Playtime's Over! that hinted it in {{12.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources_hinted|.0f}} of {{12.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources_runs|.0f}} runs
against {{12.pairs_under_40_rows_excluding_1_playtime_s_over_lowest_predicted|.2f}}-{{12.pairs_under_40_rows_excluding_1_playtime_s_over_highest_predicted|.2f}} predicted (Yukino Bijin [Dancing Light into the Night], Yaeno Muteki [Fiery Discipline], Sirius
Symboli [Escorte Étoile]). Their event puts the skill on choice 2, which ends the chain early (before its last link), so IT
takes it only when the skill is prioritised; only {{12.playtime_s_over_event_sources_runs_showing_a_priority_list|.0f}} of those runs show a priority list (the skill prioritised in {{12.playtime_s_over_event_sources_prioritised|.0f}}, hinted in {{12.playtime_s_over_event_sources_hinted_with_a_list_shown|.0f}} of the {{12.playtime_s_over_event_sources_runs_showing_a_priority_list|.0f}}) and the
other {{12.playtime_s_over_event_sources_runs_without_a_list|.0f}} hinted it {{12.playtime_s_over_event_sources_hinted_without_a_list|.0f}} time, so the priority cause is untested here.

**Other skills** (every non-target skill a deck card or spark could give, on a 30% sample of the runs, {{12.other_skills_runs_sampled|.0f}}): inheritance {{12.other_skills_inheritance|.3f}}
[{{12.other_skills_inheritance_95_pct_low|.2f}}, {{12.other_skills_inheritance_95_pct_high|.2f}}] over {{12.other_skills_inheritance_rows|,.0f}} rows, so inheritance holds beyond the eight targets; training-pool hints read {{12.other_skills_training_pool_hints|.3f}}
[{{12.other_skills_training_pool_95_pct_low|.2f}}, {{12.other_skills_training_pool_95_pct_high|.2f}}] over {{12.other_skills_training_pool_rows|,.0f}}, under-predicted.

**Untested here:** the step from acquired to landed (spark generation, and that everything hinted is bought); the
priority policy run by run; correlations inside a run beyond $P(\text{all})$; Grand Masters; hint levels. The runs skew
to good careers; every comparison is per run on that run's own deck and lineage. Rows with an inspiration are left out
of the deck test, which is then conditional on no spark, treated as independent of the deck.
