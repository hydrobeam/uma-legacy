# The model's mathematics

A model of how a career in Uma Musume's Independent Training acquires and passes on skills: what each part is, how it
was set from data, and how well it holds against real careers.

## Results

The model's odds rest on one prediction: the chance that a career acquires each target. Against
{{13.runs_used|,.0f}} real Independent Training careers, each rebuilt from its own deck and lineage (section 13), that chance is
calibrated where the model's priority assumption holds: observed / predicted {{13.3_acquired_any_source_prioritised_or_no_list_shown_obs_per_pred|.3f}} [{{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_high|.3f}}] over the
{{13.3_acquired_any_source_prioritised_or_no_list_shown_rows|,.0f}} run-target rows whose target was prioritised or whose run shows no priority list. Over all {{13.3_acquired_any_source_rows|,.0f}} rows it reads
{{13.3_acquired_any_source_obs_per_pred|.3f}} [{{13.3_acquired_any_source_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_ratio_95_pct_high|.3f}}], slightly optimistic: the model assumes every target prioritised, and the rows whose run shows it was not
read {{13.3_acquired_not_prioritised_obs_per_pred|.3f}} and carry {{13.3_acquired_shortfall_share_on_non_prioritised_rows|.0%}} of the shortfall. The chance of landing all eight default targets reads
{{13.p_all_eight_ratio_all|.2f}} [{{13.p_all_eight_ratio_all_95_pct_low|.2f}}, {{13.p_all_eight_ratio_all_95_pct_high|.2f}}] of the model: consistent with calibrated, but only {{13.runs_acquiring_every_open_target|.0f}} runs acquired every target.

The terms used below (observed / predicted, rows, nats, held out, the bracketed intervals) and the game's terms are
defined in *How to read*, after this summary.

| Part | How it is set | Main evidence | Section |
|---|---|---|---|
| Spark generation, $0.2$ / $0.4 \times 1.1^n$ | assumed | none: the runs show the sparks a career inherited, not the ones it produced | 1 |
| Star mix of sparks | mixes assumed; band per scenario derived | each scenario uses the band most of its careers reach ({{2.grand_concert_runs|,.0f}} Grand Concert runs, {{2.grand_concert_share_at_17_500_or_more|.0%}} at 17,500+) | 2 |
| Inheritance rolls | skill-white bases tested, grandparents fitted | grandparent weight {{3.grandparent_weight_fitted|.2f}} [{{3.grandparent_weight_95_pct_low|.2f}}, {{3.grandparent_weight_95_pct_high|.2f}}], affinity slope {{3.affinity_slope_fitted|.2f}} [{{3.affinity_slope_95_pct_low|.2f}}, {{3.affinity_slope_95_pct_high|.2f}}] on {{3.trials_grandparents|,.0f}} + {{3.trials_parents|,.0f}} trials, which predicts unseen runs no better than slope 1 ($z$ = {{3.affinity_slope_refitted_over_slope_1_held_out_z|.2f}}); whole careers {{3.inheritance_without_tail_held_high_obs_per_pred|.3f}} [{{3.inheritance_without_tail_held_high_ratio_95_pct_low|.3f}}, {{3.inheritance_without_tail_held_high_ratio_95_pct_high|.3f}}] apart from Tail Held High | 3 |
| Affinity: +3 per shared G1 win; own grandparent adds no trio points | +3 assumed and tested; trio rule fitted | shared-win slope {{3.race_part_slope_fitted|.2f}} [{{3.race_part_slope_95_pct_low|.2f}}, {{3.race_part_slope_95_pct_high|.2f}}], no better than 1 held out ($z$ = {{3.race_part_slope_refitted_over_slope_1_held_out_z|.2f}}); trio rule {{5.ll_0_ll_pair_nats|.0f}} nats better than the alternative | 3, 5 |
| Race win table | community table, tested | calibrated once grades include inherited aptitude sparks: {{4.race_obs_per_pred_pink|.3f}} [{{4.race_obs_per_pred_pink_95_pct_low|.3f}}, {{4.race_obs_per_pred_pink_95_pct_high|.3f}}] over {{4.race_attempts_at_the_34|,.0f}} G1 attempts | 4 |
| Loop schedules for shared wins | derived | on the reference loops, no other schedule could raise the best loop's odds by more than {{6.schedules_pb_ceiling_gain_max_pct|.2f}}% | 6 |
| Training hints, $\beta(1 + hf/100)(1 + d\,\mathit{sp}/100)$ | form fixed on the runs, slope $1/100$ assumed and tested, $\beta$ and $d$ fitted | {{7.specialty_rows|,.0f}} rows; specialty term fitted on earlier runs gains +{{7.specialty_held_out_gain_nats|.1f}} nats on later ones; real careers {{13.1_training_pool_only_obs_per_pred|.3f}} [{{13.1_training_pool_only_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_ratio_95_pct_high|.2f}}] of the model | 7 |
| Chain events, Binomial(21, 0.35) per career | fitted, kept | {{8.fit_of_n_runs|,.0f}} runs; rejected against a free distribution in-sample (p-value {{8.binomial_21_0_35_lr_p|.1e}}), a known miss; no alternative predicts unseen runs better | 8 |
| Chain events spread over remaining links | fitted | {{8.allocation_slot_over_card_nats|.0f}} nats better than equal shares per card | 8 |
| Event choices: IT takes a prioritised skill's choice | assumed | event hints of prioritised targets {{13.1_events_only_prioritised_obs_per_pred|.3f}} [{{13.1_events_only_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_prioritised_ratio_95_pct_high|.4f}}] of the model, of the others {{13.1_events_only_not_prioritised_obs_per_pred|.2f}} | 9, 13 |
| Measured outcome shares | fitted | five events, e.g. Nakayama Festa's Nothing Ventured {{9.festa_share|.2f}} [{{9.festa_share_90_pct_low|.2f}}, {{9.festa_share_90_pct_high|.2f}}] (90%) | 9 |
| Random card events | not modelled | where they could hint a target, acquisition reads {{10.calibration_obs_per_pred_left_out|.2f}} [{{10.calibration_ratio_95_pct_low_left_out|.2f}}, {{10.calibration_ratio_95_pct_high_left_out|.2f}}] of the model left out, {{10.calibration_obs_per_pred_credited|.2f}} credited | 10 |
| Conditional character events | not modelled | where only they can hint a target, left out reads {{11.calibration_conditional_only_obs_per_pred_left_out|.2f}} [{{11.calibration_conditional_only_ratio_95_pct_low_left_out|.2f}}, {{11.calibration_conditional_only_ratio_95_pct_high_left_out|.2f}}]; Vodka's own Nimble Navigator {{11.vodka_any_outfit_nimble_navigator_from_her_events|.0f}} of {{11.vodka_any_outfit_nimble_navigator_rows|.0f}} runs | 11 |

### Known misses

Each measured and tested in its section:

- Inheritance (3, 13): Tail Held High is inherited less often than its odds say ({{13.tail_held_high_inheritance|.3f}} [{{13.tail_held_high_inheritance_95_pct_low|.3f}}, {{13.tail_held_high_inheritance_95_pct_high|.3f}}]
  of the model on whole careers; cause unknown); parents at affinity 200 or more pass sparks less often than predicted
  ({{3.parents_at_affinity_200_and_above_obs_per_pred|.3f}} [{{3.parents_at_affinity_200_and_above_95_pct_low|.3f}}, {{3.parents_at_affinity_200_and_above_95_pct_high|.3f}}]); non-target skills are inherited slightly less often ({{3.non_target_skills_inheritance|.3f}} [{{3.non_target_skills_inheritance_95_pct_low|.3f}}, {{3.non_target_skills_inheritance_95_pct_high|.3f}}]).
- Races (4): chances of 70 or less win slightly less ({{4.race_pink_bp_lt_70_obs_per_pred|.2f}}); chosen races at the 1st and 2nd race in a row
  slightly less ({{4.race_pink_chosen_j_1_obs_per_pred|.3f}}, {{4.race_pink_chosen_j_2_obs_per_pred|.3f}}); goal races 7th or later in a row, which the
  planner scores as the 6th, win less than it predicts ({{4.race_pink_goal_j_ge_7_obs_per_pred|.2f}}, {{4.race_pink_goal_j_ge_7_attempts|,.0f}} attempts).
- Training hints (7): cards differ in hint rate beyond the adopted rate (observed / predicted {{7.per_card_obs_per_pred_min|.2f}}-{{7.per_card_obs_per_pred_max|.2f}} over
  {{7.per_card_cards|.0f}} cards, cause open); decks with more pals give slightly more hints (+{{7.covariate_gain_pals_per_deck_nats|.1f}} nats); a skill also hinted by
  inspiration is hinted by training less than predicted ({{7.spark_inspiration_ratio|.2f}} of the model).
- Chain events (8): Binomial(21, 0.35) is rejected against a free distribution of $N$ in-sample ($p$ = {{8.binomial_21_0_35_lr_p|.1e}}) and kept
  by choice: no alternative predicts unseen runs better, and the candidate Binomial(18, 0.405) fits {{8.candidate_binomial_18_0_405_over_binomial_21_0_35_in_sample_nats|.1f}} nats better
  in-sample. Decks with Sasami Anshinzawa [This Might Sting!]'s pal card reach more chain events ({{8.sasami_anshinzawa_s_card_mean_of_n|.2f}} against
  {{8.sasami_anshinzawa_s_card_mean_of_n_the_other_runs|.2f}}) and Inari One's careers fewer ({{8.inari_one_mean_of_n|.2f}} against {{8.inari_one_mean_of_n_the_other_runs|.2f}}); in crowded decks early links run short, and more chains never start than predicted
  ({{8.never_started_6_chain_cards_observed|.2f}} against {{8.never_started_6_chain_cards_model|.2f}} with six chain cards). Final links fit pooled ({{8.final_links_by_card_pooled_observed|.3f}} against {{8.final_links_by_card_pooled_model|.3f}}), but
  individual cards differ far beyond chance ($\chi^2$ = {{8.final_links_by_card_chi2|.0f}} on {{8.final_links_by_card_cards|.0f}} cards).
- Deck hints (13) read {{13.1_deck_hint_no_inspiration_obs_per_pred|.3f}} [{{13.1_deck_hint_no_inspiration_ratio_95_pct_low|.3f}}, {{13.1_deck_hint_no_inspiration_ratio_95_pct_high|.3f}}] of the model, almost all from event hints of targets the
  player did not prioritise ({{13.1_events_only_not_prioritised_obs_per_pred|.2f}} of the model; the model assumes every target is prioritised). Event hints of
  prioritised targets read just below the model ({{13.1_events_only_prioritised_obs_per_pred|.3f}} [{{13.1_events_only_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_prioritised_ratio_95_pct_high|.4f}}]; section 9).
- Trackblazer's hint base (13) over-predicts real careers: its training-pool hints read {{13.1_training_pool_only_trackblazer_obs_per_pred|.3f}} [{{13.1_training_pool_only_trackblazer_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_trackblazer_ratio_95_pct_high|.2f}}] of the model.
- Left out of the model: deck cards' random events, so the targets they can hint are under-predicted
  ({{10.calibration_obs_per_pred_left_out|.2f}} [{{10.calibration_ratio_95_pct_low_left_out|.2f}}, {{10.calibration_ratio_95_pct_high_left_out|.2f}}] of the model on those rows; their rate rises with the card's initial friendship gauge; section
  10); and Vodka's own Nimble Navigator ({{11.vodka_any_outfit_nimble_navigator_from_her_events|.0f}} of {{11.vodka_any_outfit_nimble_navigator_rows|.0f}} runs; section 11).
- Non-target skills (13): training-pool hints read {{13.other_skills_training_pool_hints|.3f}} [{{13.other_skills_training_pool_95_pct_low|.3f}}, {{13.other_skills_training_pool_95_pct_high|.3f}}] of the model.

### Not settled

Specialty priority predicts runs it was not fitted to (fitted on earlier runs, +{{7.specialty_held_out_gain_nats|.1f}} nats on later
ones), but it is not shown to cause more hints: on the cards whose limit breaks change only their specialty priority,
the estimate ($d$ = {{7.specialty_same_card_clean_d|u.2f}} [{{7.specialty_same_card_clean_d_95_pct_low|u.2f}}, {{7.specialty_same_card_clean_d_95_pct_high|.2f}}]) is consistent with no effect and with every per-scenario weight.
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
- **Grand Concert uses the higher mix**: most of its careers reach a rank score of 17,500 or more ({{2.grand_concert_share_at_17_500_or_more|.0%}}). URA Finale,
  Unity Cup and Trackblazer use the lower one; Unity Cup is close to even.
- **Averaging over stars overstates the inheritance chance slightly**: by at most {{2.q_overstatement_below_17_500_max|.2%}} for a target carried by
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

$$\bar b = 0.03\,(\pi_1 + 2\pi_2 + 3\pi_3) = {{2.base_per_roll_below_17_500|.4f}} \;(\text{below } 17{,}500), \quad {{2.base_per_roll_17_500_and_up|.3f}} \;(17{,}500+).$$

### Evidence

*Which mix does each scenario use?* (**derived** from the runs.) Each scenario takes the band most of its careers
reach. A career's rank score is estimated with every skill point spent, over every completed run in the run list:

| Scenario | Runs | Median rank score | Share at 17,500+ | Band |
|---|---|---|---|---|
| URA Finale | {{2.ura_finale_runs|,.0f}} | {{2.ura_finale_median_rank_score|,.0f}} | {{2.ura_finale_share_at_17_500_or_more|.3f}} [{{2.ura_finale_share_95_pct_low|.3f}}, {{2.ura_finale_share_95_pct_high|.3f}}] | below |
| Unity Cup | {{2.unity_cup_runs|,.0f}} | {{2.unity_cup_median_rank_score|,.0f}} | {{2.unity_cup_share_at_17_500_or_more|.3f}} [{{2.unity_cup_share_95_pct_low|.3f}}, {{2.unity_cup_share_95_pct_high|.3f}}] | below |
| Grand Concert | {{2.grand_concert_runs|,.0f}} | {{2.grand_concert_median_rank_score|,.0f}} | {{2.grand_concert_share_at_17_500_or_more|.3f}} [{{2.grand_concert_share_95_pct_low|.3f}}, {{2.grand_concert_share_95_pct_high|.3f}}] | 17,500+ |
| Trackblazer | {{2.trackblazer_runs|,.0f}} | {{2.trackblazer_median_rank_score|,.0f}} | {{2.trackblazer_share_at_17_500_or_more|.3f}} [{{2.trackblazer_share_95_pct_low|.3f}}, {{2.trackblazer_share_95_pct_high|.3f}}] | below |

Grand Masters has no runs and is **assumed** 17,500+.

*What does averaging over stars cost?* (**derived**.) $\bar b$ is exact for one roll but not for two: a legacy's stars
are the same in both rolls, and by convexity $E[(1 - r)^2] \ge (1 - E[r])^2$, so averaging overstates $q_k$. On
the six real affinities of each of the {{2.lineages|,.0f}} lineages in the runs, for a target carried by all six, it overstates $q$ by
at most {{2.q_overstatement_below_17_500_max|.2%}} below 17,500 (mean {{2.q_overstatement_below_17_500_mean|.2%}}) and {{2.q_overstatement_17_500_and_up_max|.2%}} at 17,500+.

## 3. Inheritance rolls

Computed by `research/s03_inheritance.py` (the roll's parts) and `research/s13_calibration.py` (the test on whole
careers).

Inheritance is one way a target gets hinted: at each inspiration, a legacy's spark of the skill may fire. This section
gives that chance, tests its parts on single-ancestor trials, and tests the whole on real careers.

### Findings

- **Each legacy carrying the target gets two rolls**, at the Classic and Senior inspirations. A roll's chance is a
  base set by the spark's stars, raised in proportion to the legacy's affinity.
- **Grandparents take the full base, as parents do**: a free weight on their roll fits {{3.grandparent_weight_fitted|.2f}} [{{3.grandparent_weight_95_pct_low|.2f}}, {{3.grandparent_weight_95_pct_high|.2f}}].
- **Affinity raises the roll as the formula says, not beaten held out**: a free slope fits {{3.affinity_slope_fitted|.2f}} [{{3.affinity_slope_95_pct_low|.2f}}, {{3.affinity_slope_95_pct_high|.2f}}],
  but it predicts unseen runs no better than slope 1 ($z$ = {{3.affinity_slope_refitted_over_slope_1_held_out_z|.2f}}), and neither does a free weight on either of its two
  parts, relation points and shared G1 wins.
- **On whole careers the chance is calibrated apart from one target**: without Tail Held High,
  {{3.inheritance_without_tail_held_high_obs_per_pred|.3f}} [{{3.inheritance_without_tail_held_high_ratio_95_pct_low|.3f}}, {{3.inheritance_without_tail_held_high_ratio_95_pct_high|.3f}}] of the model over the other default targets.
- **Known misses**: Tail Held High is inherited less often than predicted ({{13.tail_held_high_inheritance|.3f}} [{{13.tail_held_high_inheritance_95_pct_low|.3f}}, {{13.tail_held_high_inheritance_95_pct_high|.3f}}],
  cause unknown); parents at affinity 200 or more pass less often than predicted ({{3.parents_at_affinity_200_and_above_obs_per_pred|.3f}} [{{3.parents_at_affinity_200_and_above_95_pct_low|.3f}}, {{3.parents_at_affinity_200_and_above_95_pct_high|.3f}}]),
  though a refitted slope does not predict unseen runs better; non-target skills
  read {{3.non_target_skills_inheritance|.3f}} [{{3.non_target_skills_inheritance_95_pct_low|.3f}}, {{3.non_target_skills_inheritance_95_pct_high|.3f}}], the shortfall spread over many skills; and the band of predicted chance
  {{3.reliability_buckets_outside_their_own_95_pct_interval|s}} lies outside its own interval, though the bands hold jointly.

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

The roll's parts are **fitted** on {{3.trials_parents|,.0f}} parent and {{3.trials_grandparents|,.0f}} grandparent trials of ordinary ancestors (the
self-grandparent is section 5's).

*Do grandparents take the full base?* Yes. A free weight on the grandparents' roll fits {{3.grandparent_weight_fitted|.2f}} [{{3.grandparent_weight_95_pct_low|.2f}}, {{3.grandparent_weight_95_pct_high|.2f}}],
and the full base beats a halved one by {{3.grandparents_full_over_halved_nats|,.0f}} nats: {{3.grandparents_passes_observed|,.0f}} passes observed, {{3.grandparents_predicted_full_base|,.0f}} predicted with the full
base, {{3.grandparents_predicted_halved_base|,.0f}} with half. Parents read {{3.parents_obs_per_pred|.3f}} [{{3.parents_obs_per_pred_95_pct_low|.3f}}, {{3.parents_obs_per_pred_95_pct_high|.3f}}] of the model, the shortfall at high affinity (below).

*Does affinity scale the roll as $(1 + A/100)$?* The term beats no affinity by {{3.affinity_over_none_nats|,.0f}} nats. A free slope on $A$ fits
{{3.affinity_slope_fitted|.2f}} [{{3.affinity_slope_95_pct_low|.2f}}, {{3.affinity_slope_95_pct_high|.2f}}], {{3.affinity_slope_refitted_over_slope_1_in_sample_nats|.1f}} nats above slope 1 in-sample; held out it predicts unseen runs no better
({{3.affinity_slope_refitted_over_slope_1_held_out_nats|.1f}} ± {{3.affinity_slope_refitted_over_slope_1_held_out_se|.1f}} nats, $z$ = {{3.affinity_slope_refitted_over_slope_1_held_out_z|.2f}}), so the slope stays 1. Its two parts, each fitted with the other fixed: relation
points {{3.relation_part_slope_fitted|.2f}} [{{3.relation_part_slope_95_pct_low|.2f}}, {{3.relation_part_slope_95_pct_high|.2f}}], held out {{3.relation_part_slope_refitted_over_slope_1_held_out_nats|.1f}} ± {{3.relation_part_slope_refitted_over_slope_1_held_out_se|.1f}} nats ($z$ = {{3.relation_part_slope_refitted_over_slope_1_held_out_z|.2f}}); $3 \times$ shared G1 wins {{3.race_part_slope_fitted|.2f}}
[{{3.race_part_slope_95_pct_low|.2f}}, {{3.race_part_slope_95_pct_high|.2f}}], held out {{3.race_part_slope_refitted_over_slope_1_held_out_nats|.1f}} ± {{3.race_part_slope_refitted_over_slope_1_held_out_se|.1f}} nats ($z$ = {{3.race_part_slope_refitted_over_slope_1_held_out_z|.2f}}) ({{3.trials_with_shared_wins|,.0f}} trials with shared wins). Neither
weight predicts unseen runs better than 1.

*Do the bases by stars hold?* Observed / predicted is {{3.1_star_obs_per_pred|.3f}} [{{3.1_star_95_pct_low|.2f}}, {{3.1_star_95_pct_high|.2f}}] at 1★ ({{3.1_star_trials|,.0f}} trials),
{{3.2_star_obs_per_pred|.3f}} [{{3.2_star_95_pct_low|.2f}}, {{3.2_star_95_pct_high|.2f}}] at 2★ ({{3.2_star_trials|,.0f}}) and {{3.3_star_obs_per_pred|.3f}} [{{3.3_star_95_pct_low|.2f}}, {{3.3_star_95_pct_high|.2f}}] at 3★ ({{3.3_star_trials|,.0f}}).

*Does it hold at every affinity?* Below 200, yes: the four bands are jointly consistent with the model ($\chi^2$ =
{{3.affinity_bands_below_200_chi2|.1f}}, $p$ = {{3.affinity_bands_below_200_chi2_p|.2f}}). The bands outside their own 95% interval are {{3.affinity_bands_outside_their_own_95_pct_interval|s}}, and with them the five
bands are not consistent ($\chi^2$ = {{3.affinity_bands_chi2|.1f}}, $p$ = {{3.affinity_bands_chi2_p|.3f}}). This is a known miss: parents at affinity 200 or more read
{{3.parents_at_affinity_200_and_above_obs_per_pred|.3f}} [{{3.parents_at_affinity_200_and_above_95_pct_low|.3f}}, {{3.parents_at_affinity_200_and_above_95_pct_high|.3f}}] of the model ({{3.parents_at_affinity_200_and_above_trials|,.0f}} trials), parents below it {{3.parents_at_affinity_below_200_obs_per_pred|.3f}} [{{3.parents_at_affinity_below_200_95_pct_low|.3f}}, {{3.parents_at_affinity_below_200_95_pct_high|.3f}}]
({{3.parents_at_affinity_below_200_trials|,.0f}}). A refitted slope does not predict unseen runs better (above), so the model keeps slope 1 and
this miss:

| Affinity | Trials | Observed / predicted |
|---|---|---|
| 0-49 | {{3.affinity_0_49_trials|,.0f}} | {{3.affinity_0_49_obs_per_pred|.3f}} [{{3.affinity_0_49_95_pct_low|.3f}}, {{3.affinity_0_49_95_pct_high|.3f}}] |
| 50-99 | {{3.affinity_50_99_trials|,.0f}} | {{3.affinity_50_99_obs_per_pred|.3f}} [{{3.affinity_50_99_95_pct_low|.3f}}, {{3.affinity_50_99_95_pct_high|.3f}}] |
| 100-149 | {{3.affinity_100_149_trials|,.0f}} | {{3.affinity_100_149_obs_per_pred|.3f}} [{{3.affinity_100_149_95_pct_low|.3f}}, {{3.affinity_100_149_95_pct_high|.3f}}] |
| 150-199 | {{3.affinity_150_199_trials|,.0f}} | {{3.affinity_150_199_obs_per_pred|.3f}} [{{3.affinity_150_199_95_pct_low|.3f}}, {{3.affinity_150_199_95_pct_high|.3f}}] |
| 200+ | {{3.affinity_200_trials|,.0f}} | {{3.affinity_200_obs_per_pred|.3f}} [{{3.affinity_200_95_pct_low|.3f}}, {{3.affinity_200_95_pct_high|.3f}}] |

*Does the whole hold on real careers?* Observed / predicted inspiration on {{3.runs_tested|,.0f}} careers (section 13):

- all {{3.run_target_rows|,.0f}} run-target rows: {{3.inheritance_obs_per_pred|.3f}} [{{3.inheritance_ratio_95_pct_low|.3f}}, {{3.inheritance_ratio_95_pct_high|.3f}}], short of the model because of one target;
- Tail Held High, a known miss: {{13.tail_held_high_inheritance|.3f}} [{{13.tail_held_high_inheritance_95_pct_low|.3f}}, {{13.tail_held_high_inheritance_95_pct_high|.3f}}], the only default target whose interval excludes 1;
  the cause is not known;
- the other {{3.inheritance_without_tail_held_high_rows|,.0f}} rows: {{3.inheritance_without_tail_held_high_obs_per_pred|.3f}} [{{3.inheritance_without_tail_held_high_ratio_95_pct_low|.3f}}, {{3.inheritance_without_tail_held_high_ratio_95_pct_high|.3f}}]; on them, by the number of carriers, 1 to 6:
  {{3.by_carriers_1_6_without_tail_held_high_lowest_ratio|.3f}}-{{3.by_carriers_1_6_without_tail_held_high_highest_ratio|.3f}}, every interval covering 1; per roll, years in which an inspiration of the skill fired,
  {{3.per_roll_without_tail_held_high_spark_years_obs_per_exp|.3f}} of the model's expectation ({{3.per_roll_without_tail_held_high_spark_years_sds_below_expectation|.2f}} standard deviations below it, run-clustered; inside its 95% band), and
  legacy firings, against their expectation $2\sum_l r_l$, {{3.per_roll_without_tail_held_high_ancestor_hits_obs_per_exp|.3f}};
- {{3.non_target_skills_inheritance_rows|,.0f}} rows of non-target skills, a known miss: {{3.non_target_skills_inheritance|.3f}} [{{3.non_target_skills_inheritance_95_pct_low|.3f}}, {{3.non_target_skills_inheritance_95_pct_high|.3f}}], the shortfall spread over
  many skills (no one skill carries more than {{13.other_skills_largest_one_skill_s_share_of_the_inheritance_shortfall|.0%}} of it).

*Does it hold at every predicted chance?* Grouped by predicted $q$, the groups are calibrated jointly ($\chi^2$ =
{{3.reliability_chi2|.1f}} on {{3.reliability_chi2_buckets|.0f}} groups, $p$ = {{3.reliability_chi2_p|.2f}}). One lies outside its own 95% interval, {{3.reliability_buckets_outside_their_own_95_pct_interval|s}}: {{3.reliability_q_0_40_0_60_observed|.3f}}
[{{3.reliability_q_0_40_0_60_observed_95_pct_low|.3f}}, {{3.reliability_q_0_40_0_60_observed_95_pct_high|.3f}}] observed against {{3.reliability_q_0_40_0_60_mean_predicted_three_places|.3f}} predicted.

| Predicted $q$ | Rows | Observed (mean predicted) |
|---|---|---|
| 0 | {{3.reliability_q_0_rows|,.0f}} | {{3.reliability_q_0_observed|.3f}} |
| above 0 to 0.20 | {{3.reliability_q_0_00_0_20_rows|,.0f}} | {{3.reliability_q_0_00_0_20_observed|.3f}} ({{3.reliability_q_0_00_0_20_mean_predicted|.2f}}) |
| 0.20-0.40 | {{3.reliability_q_0_20_0_40_rows|,.0f}} | {{3.reliability_q_0_20_0_40_observed|.3f}} ({{3.reliability_q_0_20_0_40_mean_predicted|.2f}}) |
| 0.40-0.60 | {{3.reliability_q_0_40_0_60_rows|,.0f}} | {{3.reliability_q_0_40_0_60_observed|.3f}} ({{3.reliability_q_0_40_0_60_mean_predicted|.2f}}) |
| 0.60-0.80 | {{3.reliability_q_0_60_0_80_rows|,.0f}} | {{3.reliability_q_0_60_0_80_observed|.3f}} ({{3.reliability_q_0_60_0_80_mean_predicted|.2f}}) |
| 0.80-0.95 | {{3.reliability_q_0_80_0_95_rows|,.0f}} | {{3.reliability_q_0_80_0_95_observed|.3f}} ({{3.reliability_q_0_80_0_95_mean_predicted|.2f}}) |

## 4. Race wins

Computed by `research/s04_races.py`.

Race wins matter through affinity: two umas that won the same G1s inherit more from each other (section 5). This
section measures how a trainee's chance of winning a race depends on her aptitudes, on how many races she has run in a
row and on whether her career requires the race, and tests a widely used community table of that chance.

### Findings

A race's win chance depends on the trainee's grades for its distance and surface (written as a pair, better grade
first) and on how many races she has run in a row. Of {{4.race_attempts_at_the_34|,.0f}} attempts at the 34 single-race G1s, the {{4.race_chosen_attempts|,.0f}} that IT
chose to run won at these rates (the community table's chance in parentheses; grades include those raised by
inherited aptitude sparks; S is shown as A, since both have the same chance; a dot where a cell has under 30 attempts;
grade pairs with 200 or more attempts):

{{4.race_grid_by_aptitude_and_streak|s}}

- **A goal race takes the penalty of the race before it.** Goal races (the races a career requires) win as if they
  stood one place earlier in the streak: no penalty up to the 3rd race in a row, then 5, 20 and 30. Scored that way
  they are calibrated at every position, and the table predicts unseen runs {{4.held_out_goal_shift_vs_community_table_nats|,.0f}} ± {{4.held_out_goal_shift_vs_community_table_se|.0f}} nats better than the
  community table, which charges a goal race its own position ($z$ = {{4.held_out_goal_shift_vs_community_table_z|.2f}}). A goal race still counts toward the
  position of the races after it: not counting goal races at all predicts {{4.held_out_goal_shift_vs_exclusion_nats|,.0f}} nats worse.
- **With that shift the table predicts these rates** for goal races at every position and for chosen races from the
  3rd race in a row on. Outside chosen races at the 1st and 2nd it is calibrated in every scenario.
- **Grades must include inherited aptitude sparks.** With the trainee's base grades the table predicts too few wins
  (observed / predicted {{4.race_obs_per_pred_base|.3f}} [{{4.race_obs_per_pred_base_95_pct_low|.3f}}, {{4.race_obs_per_pred_base_95_pct_high|.3f}}], {{4.race_base_bp_lt_70_obs_per_pred|.2f}} where base grades give a chance of 70 or less). Pink sparks
  raise grades at the start of a career and at each inspiration, from that April on.
- **Races in a row lower the chance; wins do not.** The penalty counts races entered. At the 2nd, 3rd and 4th or later
  race in a row the previous race's result does not change the next chance: after a loss the table's fit is
  {{4.race_in_a_streak_j_2_after_a_loss_per_after_a_win_obs_per_pred|.3f}}, {{4.race_in_a_streak_j_3_after_a_loss_per_after_a_win_obs_per_pred|.3f}} and {{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred|.3f}} of its fit after a win, each interval covering 1.
- **Known misses**: chances of 70 or less win slightly less than predicted ({{4.race_pink_bp_lt_70_obs_per_pred|.2f}}
  [{{4.race_pink_bp_lt_70_obs_per_pred_95_pct_low|.2f}}, {{4.race_pink_bp_lt_70_obs_per_pred_95_pct_high|.2f}}]); the planner counts a streak only to its 6th race, so the few goal races 7th or later
  in a row ({{4.race_pink_goal_j_ge_7_attempts|,.0f}}) win less than it predicts ({{4.race_pink_goal_j_ge_7_obs_per_pred|.2f}} [{{4.race_pink_goal_j_ge_7_obs_per_pred_95_pct_low|.2f}}, {{4.race_pink_goal_j_ge_7_obs_per_pred_95_pct_high|.2f}}]); chosen races
  at the 1st and 2nd race in a row win slightly less than predicted ({{4.race_pink_chosen_j_1_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_1_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_1_obs_per_pred_95_pct_high|.3f}}] and {{4.race_pink_chosen_j_2_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_2_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_2_obs_per_pred_95_pct_high|.3f}}]), which
  puts the table's overall observed / predicted at {{4.race_obs_per_pred_pink|.3f}} [{{4.race_obs_per_pred_pink_95_pct_low|.3f}}, {{4.race_obs_per_pred_pink_95_pct_high|.3f}}].

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

The runs record every race a career ran: {{4.race_attempts_at_the_34|,.0f}} attempts at the 34 single-race G1s ({{4.race_goal_attempts|,.0f}} of
them goals), of which {{4.race_win_rate_observed|.1%}} were won.

*Base aptitudes or raised grades?* Raised grades.

- On base aptitudes (as for a planned loop), the table predicts {{4.race_win_rate_predicted_base|.1%}}: observed / predicted
  {{4.race_obs_per_pred_base|.3f}} [{{4.race_obs_per_pred_base_95_pct_low|.3f}}, {{4.race_obs_per_pred_base_95_pct_high|.3f}}], and {{4.race_base_bp_lt_70_obs_per_pred|.2f}} where the table gives 70 or less ({{4.race_base_bp_lt_70_attempts|,.0f}} attempts).
- With her grades raised by the aptitude-spark rule (`docs/MODEL.md`: the legacies' pink stars at the
  start, at most to A; then each pink the run records firing at a Classic or Senior inspiration, from that April on),
  it predicts {{4.race_win_rate_predicted_pink|.1%}}: observed / predicted {{4.race_obs_per_pred_pink|.3f}} [{{4.race_obs_per_pred_pink_95_pct_low|.3f}}, {{4.race_obs_per_pred_pink_95_pct_high|.3f}}], {{4.race_ll_recalibrated_pink_table_recalibrated_base_table_nats|,.0f}} nats better than
  base aptitudes. Chosen races at the 1st and 2nd race in a row fall slightly short ({{4.race_pink_chosen_j_1_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_1_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_1_obs_per_pred_95_pct_high|.3f}}] and
  {{4.race_pink_chosen_j_2_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_2_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_2_obs_per_pred_95_pct_high|.3f}}]); outside them every scenario is calibrated, and over all attempts {{4.race_pink_scenarios_outside_their_interval|s}}
  fall outside their intervals.

*Is the streak penalty real?* Yes: without it the fit is {{4.race_ll_recalibrated_pink_table_recalibrated_pink_no_penalty_nats|,.0f}} nats worse.

Both comparisons need one adjustment. The table calls some races certain (0% or 100%), and a handful went the other
way, which gives the table as stated a log-likelihood of $-\infty$. So every model compared has its 0% and 100% pulled
in to a fitted floor and ceiling, the same two parameters in each; the comparisons are then between models with equal
numbers of fitted parameters (rule 1 of *How to read*).

*Do goal races pay the same penalty?* No: one position less. Under the community table, goal races from the 4th race in
a row win more than predicted ({{4.race_community_table_goal_j_ge_4_obs_per_pred|.3f}} [{{4.race_community_table_goal_j_ge_4_obs_per_pred_95_pct_low|.3f}}, {{4.race_community_table_goal_j_ge_4_obs_per_pred_95_pct_high|.3f}}], {{4.race_community_table_goal_j_ge_4_attempts|,.0f}} attempts), while chosen races there match.
Three forms, each with the same fitted floor and ceiling, scored on runs left out of the fit (five folds by run,
{{4.held_out_runs|,.0f}} runs):

| Form | Goal race at position $j$ pays | Races after it count it | Held out, against the adopted form |
|---|---|---|---|
| Adopted | $\mathrm{PEN}_{\max(j-1,1)}$ | yes | |
| Community table | $\mathrm{PEN}_j$ | yes | {{4.held_out_goal_shift_vs_community_table_nats|,.0f}} ± {{4.held_out_goal_shift_vs_community_table_se|.0f}} nats worse ($z$ = {{4.held_out_goal_shift_vs_community_table_z|.2f}}) |
| Exclusion | $\mathrm{PEN}$ at the chosen races before it | no | {{4.held_out_goal_shift_vs_exclusion_nats|,.0f}} ± {{4.held_out_goal_shift_vs_exclusion_se|.0f}} nats worse ($z$ = {{4.held_out_goal_shift_vs_exclusion_z|.2f}}) |

In-sample the adopted form fits {{4.race_ll_recalibrated_pink_table_recalibrated_community_table_nats|,.0f}} nats better than the community table. Adopted, goal races are calibrated at
each position, and chosen races from the 3rd on; chosen races at the 1st and 2nd fall slightly short, a known miss
(observed / predicted, attempts):

| Race in a row | Goal races | Chosen races |
|---|---|---|
| 1st | {{4.race_pink_goal_j_1_obs_per_pred|.3f}} [{{4.race_pink_goal_j_1_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_1_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_goal_j_1_attempts|,.0f}}) | {{4.race_pink_chosen_j_1_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_1_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_1_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_chosen_j_1_attempts|,.0f}}) |
| 2nd | {{4.race_pink_goal_j_2_obs_per_pred|.3f}} [{{4.race_pink_goal_j_2_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_2_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_goal_j_2_attempts|,.0f}}) | {{4.race_pink_chosen_j_2_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_2_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_2_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_chosen_j_2_attempts|,.0f}}) |
| 3rd | {{4.race_pink_goal_j_3_obs_per_pred|.3f}} [{{4.race_pink_goal_j_3_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_3_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_goal_j_3_attempts|,.0f}}) | {{4.race_pink_chosen_j_3_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_3_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_3_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_chosen_j_3_attempts|,.0f}}) |
| 4th | {{4.race_pink_goal_j_4_obs_per_pred|.3f}} [{{4.race_pink_goal_j_4_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_4_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_goal_j_4_attempts|,.0f}}) | {{4.race_pink_chosen_j_4_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_4_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_4_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_chosen_j_4_attempts|,.0f}}) |
| 5th | {{4.race_pink_goal_j_5_obs_per_pred|.3f}} [{{4.race_pink_goal_j_5_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_5_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_goal_j_5_attempts|,.0f}}) | {{4.race_pink_chosen_j_5_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_5_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_5_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_chosen_j_5_attempts|,.0f}}) |
| 6th or later | {{4.race_pink_goal_j_6_obs_per_pred|.3f}} [{{4.race_pink_goal_j_6_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_goal_j_6_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_goal_j_6_attempts|,.0f}}) | {{4.race_pink_chosen_j_6_obs_per_pred|.3f}} [{{4.race_pink_chosen_j_6_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_chosen_j_6_obs_per_pred_95_pct_high|.3f}}] ({{4.race_pink_chosen_j_6_attempts|,.0f}}) |

Why a goal race should pay one position less is not known; the runs only show that it does.

*Does the table hold at every chance?* On raised grades:

| Table's chance | Attempts | Observed / predicted |
|---|---|---|
| 110 | {{4.race_pink_bp_110_attempts|,.0f}} | {{4.race_pink_bp_110_obs_per_pred|.3f}} [{{4.race_pink_bp_110_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_110_obs_per_pred_95_pct_high|.3f}}] |
| 100 | {{4.race_pink_bp_100_attempts|,.0f}} | {{4.race_pink_bp_100_obs_per_pred|.3f}} [{{4.race_pink_bp_100_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_100_obs_per_pred_95_pct_high|.3f}}] |
| 90 | {{4.race_pink_bp_90_attempts|,.0f}} | {{4.race_pink_bp_90_obs_per_pred|.3f}} [{{4.race_pink_bp_90_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_90_obs_per_pred_95_pct_high|.3f}}] |
| 80 | {{4.race_pink_bp_80_attempts|,.0f}} | {{4.race_pink_bp_80_obs_per_pred|.3f}} [{{4.race_pink_bp_80_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_80_obs_per_pred_95_pct_high|.3f}}] |
| 70 or less | {{4.race_pink_bp_lt_70_attempts|,.0f}} | {{4.race_pink_bp_lt_70_obs_per_pred|.3f}} [{{4.race_pink_bp_lt_70_obs_per_pred_95_pct_low|.3f}}, {{4.race_pink_bp_lt_70_obs_per_pred_95_pct_high|.3f}}] |

*When does a pink spark take effect?* From the April of its inspiration. The records say which inspiration fired a
pink, and inspirations sit at fixed turns. On the {{4.proc_timing_attempts|,.0f}} Senior January-March attempts where the timing matters,
{{4.proc_timing_observed|.3f}} [{{4.proc_timing_observed_95_pct_low|.3f}}, {{4.proc_timing_observed_95_pct_high|.3f}}] were won, against {{4.proc_timing_predicted_from_april|.3f}} predicted with the pink counted from April and {{4.proc_timing_predicted_from_the_year_start|.3f}} counted
from the year's start.

*How many chosen attempts is each cell of the table in Findings?*

{{4.race_grid_attempts|s}}

*Does the previous race's result matter?* No. Each attempt's chance combines both effects in one formula: her grades
for the race's distance and surface, and her position in the current streak. The penalty counts races in a row, not
wins. The test is the contrast: observed / predicted after a loss divided by the same after a win, with a
run-clustered interval (observed / predicted, attempts):

| Race in a row | After a win | After a loss | Loss / win |
|---|---|---|---|
| 2nd | {{4.race_in_a_streak_j_2_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_2_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_2_after_a_win_obs_per_pred_95_pct_high|.3f}}] ({{4.race_in_a_streak_j_2_after_a_win_attempts|,.0f}}) | {{4.race_in_a_streak_j_2_after_a_loss_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_2_after_a_loss_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_2_after_a_loss_obs_per_pred_95_pct_high|.3f}}] ({{4.race_in_a_streak_j_2_after_a_loss_attempts|,.0f}}) | {{4.race_in_a_streak_j_2_after_a_loss_per_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_2_after_a_loss_per_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_2_after_a_loss_per_after_a_win_obs_per_pred_95_pct_high|.3f}}] |
| 3rd | {{4.race_in_a_streak_j_3_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_3_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_3_after_a_win_obs_per_pred_95_pct_high|.3f}}] ({{4.race_in_a_streak_j_3_after_a_win_attempts|,.0f}}) | {{4.race_in_a_streak_j_3_after_a_loss_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_3_after_a_loss_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_3_after_a_loss_obs_per_pred_95_pct_high|.3f}}] ({{4.race_in_a_streak_j_3_after_a_loss_attempts|,.0f}}) | {{4.race_in_a_streak_j_3_after_a_loss_per_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_3_after_a_loss_per_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_3_after_a_loss_per_after_a_win_obs_per_pred_95_pct_high|.3f}}] |
| 4th or later | {{4.race_in_a_streak_j_4_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_4_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_4_after_a_win_obs_per_pred_95_pct_high|.3f}}] ({{4.race_in_a_streak_j_4_after_a_win_attempts|,.0f}}) | {{4.race_in_a_streak_j_4_after_a_loss_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_4_after_a_loss_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_4_after_a_loss_obs_per_pred_95_pct_high|.3f}}] ({{4.race_in_a_streak_j_4_after_a_loss_attempts|,.0f}}) | {{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred|.3f}} [{{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred_95_pct_low|.3f}}, {{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred_95_pct_high|.3f}}] |

Every contrast covers 1 ($z$ = {{4.race_in_a_streak_j_4_after_a_loss_per_after_a_win_obs_per_pred_z|.2f}} from the 4th race on): the previous result does not move the next chance.

*Is one formula enough?* Yes, though not exactly. Giving every (chance, streak position, goal or chosen) cell its own
win rate, read straight from its attempts ({{4.race_pink_cells_bp_x_j_x_goal|.0f}} cells), explains the attempts {{4.race_ll_pink_cells_bp_x_j_x_goal_recalibrated_pink_table_nats|.0f}} nats better than the
table's one formula in-sample. That is beyond the {{4.race_ll_threshold_pink_cells_bp_x_j_x_goal|.0f}} nats chance allows for its {{4.race_ll_pink_cells_bp_x_j_x_goal_extra_parameters|.0f}} extra parameters if
attempts were independent, but they are not: one trainee's stats carry through her races. Held out, the per-cell rates
do not predict unseen runs better than the table ({{4.held_out_cells_vs_table_nats|.0f}} ± {{4.held_out_cells_vs_table_se|.0f}} nats, $z$ = {{4.held_out_cells_vs_table_z|.2f}}). So the table stands, with the
known misses in Findings.

The test is conditional on IT having chosen each non-goal race; stats are not modelled.

## 5. Affinity

Computed by `research/s05_affinity.py` (the self-grandparent fit).

A legacy's affinity $A_l$ scales her inheritance roll, $r = b\,(1 + A_l/100)$ (section 3). This section defines how
$A_l$ is computed and fits the one rule the game data leave open.

### Findings

- **Affinity adds the characters' relation points (game data) and 3 for each single-race G1 both umas
  won.** The 3 per shared win is **assumed** as the game's rule; in section 3 a free weight on it fits
  {{3.race_part_slope_fitted|.2f}} [{{3.race_part_slope_95_pct_low|.2f}}, {{3.race_part_slope_95_pct_high|.2f}}] but predicts unseen runs no better than 1 ($z$ = {{3.race_part_slope_refitted_over_slope_1_held_out_z|.2f}}).
- **A trainee who is her own grandparent gets no trio points from it.** The game data do not say what a trio with
  one character twice counts. Counting nothing predicts the self-grandparent's inspirations in the runs; counting it
  as the pair is about {{5.ll_0_ll_pair_nats|.0f}} nats worse.

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
{{5.trials_attributable_to_one_ancestor|,.0f}} inspiration trials attributable to a single ancestor. Self-grandparent passes: {{5.self_grandparent_passes_observed|.0f}} observed, {{5.predicted_tri_t_p_t_0|.1f}}
predicted with 0, {{5.predicted_tri_t_p_t_pair_t_p|.1f}} with $\mathrm{tri}(T,P,T) = \mathrm{pair}(T,P)$ (about {{5.ll_0_ll_pair_nats|.0f}} nats worse). Run-clustered, the observed count lies
{{5.self_grandparent_passes_z_with_0|u.2f}} standard deviations from the prediction with 0 and {{5.self_grandparent_passes_z_with_pair|u.2f}} from the one with the pair: 0 is kept and the pair
rejected. Allowing any multiplier $\kappa$, the best fit is $\hat\kappa = {{5.kappa_fitted|.0f}}$, at its lower bound.

The gap is not the ordinary trials' own misfit: with the affinity slope refitted on the ordinary ancestors
({{5.affinity_slope_refitted_on_the_ordinary_trials|.2f}}; section 3 keeps 1), the self-grandparent passes lie {{5.self_grandparent_passes_refitted_slope_z_with_0|u.2f}} standard deviations from the prediction with 0
and {{5.self_grandparent_passes_refitted_slope_z_with_pair|u.2f}} from the pair. At slope 1 ordinary parents read {{5.ordinary_parents_observed_per_predicted|.3f}} of prediction and grandparents
{{5.ordinary_grandparents_observed_per_predicted|.3f}} (section 3's known miss at high affinity).

*Does the formula as a whole hold?* Section 3 tests it on the runs' real lineages and shared wins: a free slope on this $A$
fits {{3.affinity_slope_fitted|.2f}} [{{3.affinity_slope_95_pct_low|.2f}}, {{3.affinity_slope_95_pct_high|.2f}}] but predicts unseen runs no better than 1 ($z$ = {{3.affinity_slope_refitted_over_slope_1_held_out_z|.2f}}); parents at affinity 200 or more
pass less often than predicted, a known miss.

## 6. Loop structures, ranking and schedules

Computed by `research/s06_loops.py`.

A breeding loop is three or four umas bred from each other in turn. This section defines a loop's lineages and how a
loop is scored, and bounds what a different choice of race schedules could gain.

### Findings

- **A four-uma loop has {{6.four_uma_loop_lineages|.0f}} lineages, a three-uma rotation {{6.three_uma_rotation_lineages|.0f}}.** In a rotation the trainee is her own grandparent,
  and that occurs in practice: {{6.a_grandparent_of_her_own_character|,.0f}} of {{6.runs|,.0f}} runs have a grandparent of the trainee's own character.
- **A structure is ranked by its lineages' average odds**: the long-run rate of a player who trains every member in
  turn. This is a definition of the score, not a claim.
- **Every pair of umas carries the same weight**, so a loop's race schedules can be chosen for the most expected shared
  wins over all its pairs.
- **Scheduling for shared wins instead of odds costs little**: no schedule could raise a reference loop's odds by
  more than {{6.schedules_pb_ceiling_gain_max_pct|.2f}}%, and no listed runner-up could overtake its winner.

### Terms

- *Lineage* $(T, P_1, P_2, X)$: a trainee, her parents, and $X$; each parent was bred from the other parent and $X$.
- *Structure* $S$: a loop or rotation; $\Lambda_S$: its set of lineages; $|\Lambda_S|$: their number.
- $P_\lambda(\text{all})$: section 0's $P(\text{all})$ for lineage $\lambda$; $\bar P_S$: the structure's score.
- $o(a,b)$: expected shared G1 wins (section 5); $o_{\max}(a,b)$: its value with every offering of every G1
  attempted and no streak penalty.
- *Pair ceiling*: the highest expected overlap a pair can reach over feasible schedules, found by search.
- *Reference loops*: the best loops and decks found for a fixed set of reference setups (rosters and scenarios).

### Model

A four-uma loop has {{6.four_uma_loop_lineages|.0f}} lineages (each member trains from two of the other three); a three-uma
rotation has {{6.three_uma_rotation_lineages|.0f}}, with $X = T$ (the trainee's own earlier veteran). A structure $S$ scores

$$\bar P_S = \frac{1}{|\Lambda_S|} \sum_{\lambda \in \Lambda_S} P_\lambda(\text{all}),$$

the long-run rate of a player who trains every member in turn. It is a definition of the score, not a
claim: a player who re-trains only the best trainee would score a structure by its best lineage.

*Race weights* (**derived**). In one lineage $o(P_1,P_2)$ enters the six affinities {{6.one_lineage_o_p1_p2_weight|.0f}} times, and $o(P_1,X)$ and
$o(P_2,X)$ twice each. Summed over the 12 loop lineages, every pair gets ${{6.one_lineage_o_p1_p2_weight|.0f}}\cdot 2 + {{6.one_lineage_o_p1_x_weight|.0f}}\cdot 4 = {{6.loop_weight_per_pair|.0f}}$; over the 3
rotation lineages, ${{6.one_lineage_o_p1_p2_weight|.0f}} + {{6.one_lineage_o_p1_x_weight|.0f}} + {{6.one_lineage_o_p1_x_weight|.0f}} = {{6.rotation_weight_per_pair|.0f}}$. Both are uniform over pairs, so a structure's schedules maximise
$\sum_{a<b} o(a,b)$.

### Evidence

*Does the rotation occur?* Yes: {{6.a_grandparent_of_her_own_character|,.0f}} of {{6.runs|,.0f}} runs have a grandparent of the trainee's own character, and
{{6.a_parent_of_her_own_character|.0f}} have a parent of her character.

*What could a different schedule gain?* (**derived**, on the reference loops.) Schedules are chosen for $\sum_{a<b} o(a,b)$, not for $\bar P_S$. $\bar P_S$ rises with every $o$: $o$ enters each legacy's affinity with
weight +3, and $r$, $q$ and every target's odds are nondecreasing in their inputs. So, with a loop's decks fixed, any
schedule (including one chosen for $\bar P_S$ directly) scores between the odds found and the odds with every pair at
its highest possible overlap. Over {{6.schedules_result_files|.0f}} reference results, whose odds an independent implementation reproduces to within
{{6.schedules_recomputed_odds_max_relative_error|.0e}}:

- the shared wins found are worth {{6.schedules_shared_win_value_min_pct|.0f}}-{{6.schedules_shared_win_value_max_pct|.0f}}% over no race affinity;
- the independent ceiling $o_{\max}(a,b) = \sum_r P^{\max}_a(r)\,P^{\max}_b(r)$ caps the gain from a different
  objective at {{6.schedules_ceiling_gain_max_pct|.1f}}%. It is far from reachable (the schedules found reach about {{6.schedules_overlap_found_as_pct_of_o_max_median|.0f}}% of it) and cannot
  settle the ranking: {{6.schedules_runner_ups_whose_ceiling_reaches_the_winner|.0f}} of {{6.schedules_runner_up_loops_checked|.0f}} listed runner-up loops reach their winner under it;
- the pair ceilings come from the same search code that chose the schedules, so they are not an independent check, but they sit between
  $o$ and $o_{\max}$ on every pair. They put the ceiling only {{6.schedules_pb_ceiling_gain_min_pct|.2f}}-{{6.schedules_pb_ceiling_gain_max_pct|.2f}}% above the odds found (median {{6.schedules_pb_ceiling_gain_median_pct|.2f}}%), and no listed runner-up's ceiling
  reaches its winner ({{6.schedules_runner_ups_whose_pb_ceiling_reaches_the_winner|.0f}} of {{6.schedules_runner_up_loops_checked|.0f}}).

## 7. Training hints

Computed by `research/s07_hints.py`.

A deck card's training is the main way targets get hinted. When a card joins a training session it can show a hint,
which raises one skill from the card's list of hintable skills (its *pool*) by a hint level or more.

### Findings

- **A card gives about the same number of training hints every career, each a random skill from its pool.** A typical card in
  the runs' decks (hint frequency +{{7.example_median_hint_frequency|.0f}}%, specialty priority {{7.example_median_specialty_priority|.0f}}) gives about {{7.example_hints_per_career_median_card_unity_cup|.1f}} hints per career in Unity
  Cup, {{7.example_hints_per_career_median_card_grand_concert|.1f}} in Grand Concert, {{7.example_hints_per_career_median_card_trackblazer|.1f}} in Trackblazer and {{7.example_hints_per_career_median_card_ura_finale|.1f}} in URA Finale. Each hint picks a skill uniformly from the pool;
  a skill already at hint level 5 is no longer picked.
- **Hint frequency scales that number directly**: a card with Hint Frequency +40% gives 40% more hints. The slope is
  **assumed**; on the adopted model a free one does not predict unseen careers better.
- **Specialty priority also raises it.** The game describes Specialty Priority only as making the character join her
  preferred training type more often; in the runs, cards with more of it give more hints, by an amount that differs
  between scenarios. The effect predicts careers it was not fitted to, but it is an association: on the cards whose
  limit breaks change only their specialty priority, the data fit no effect as well as the fitted one, so it is not
  shown to be a cause.
- **Scenarios differ**: each has its own base (table below). The specialty weight differs too: fitted, about
  {{7.specialty_d_unity_cup|.2f}} in Unity Cup and {{7.specialty_d_ura_finale|.2f}} in URA Finale against {{7.specialty_d_grand_concert|.2f}} in Grand Concert and {{7.specialty_d_trackblazer|.2f}} in Trackblazer, and a weight per scenario
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
  $c = \lceil 5 / \iota \rceil$ hints ({{7.draws_to_lv_5_1_card|.0f}}, {{7.draws_to_lv_5_2_card|.0f}}, {{7.draws_to_lv_5_3_card|.0f}}, {{7.draws_to_lv_5_4_card|.0f}}, {{7.draws_to_lv_5_5_card|.0f}} for +1 to +5).
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
run, on maximum-limit-break cards, rows where no inspiration hinted the skill, R cards out; {{7.specialty_rows|,.0f}} rows from
{{7.specialty_card_runs|,.0f}} card-runs; the Runs column counts the runs that add a row; *Adopted*: the values the model uses, fitted
on an earlier, smaller set of the runs:

| Scenario | $\beta$ | $d$ | Adopted $\beta$ / $d$ | Rows | Runs |
|---|---|---|---|---|---|
| Unity Cup | {{7.specialty_base_unity_cup|.3f}} | {{7.specialty_d_unity_cup|.3f}} | {{7.adopted_unity_cup|.2f}} / {{7.adopted_d_unity_cup|.2f}} | {{7.specialty_rows_unity_cup|,.0f}} | {{7.specialty_runs_unity_cup|.0f}} |
| Grand Concert | {{7.specialty_base_grand_concert|.3f}} | {{7.specialty_d_grand_concert|.3f}} | {{7.adopted_grand_concert|.2f}} / {{7.adopted_d_grand_concert|.2f}} | {{7.specialty_rows_grand_concert|,.0f}} | {{7.specialty_runs_grand_concert|,.0f}} |
| URA Finale | {{7.specialty_base_ura_finale|.3f}} | {{7.specialty_d_ura_finale|.3f}} | {{7.adopted_ura_finale|.2f}} / {{7.adopted_d_ura_finale|.2f}} | {{7.specialty_rows_ura_finale|,.0f}} | {{7.specialty_runs_ura_finale|.0f}} |
| Trackblazer | {{7.specialty_base_trackblazer|.3f}} | {{7.specialty_d_trackblazer|.3f}} | {{7.adopted_trackblazer|.2f}} / {{7.adopted_d_trackblazer|.2f}} | {{7.specialty_rows_trackblazer|,.0f}} | {{7.specialty_runs_trackblazer|.0f}} |
| Grand Masters | no runs | | {{7.adopted_grand_masters|.2f}} / {{7.adopted_d_grand_masters|.2f}} (**assumed**: one $\beta$ and one $d$ over every scenario) | | |

### Evidence

*Specialty priority.* Without the specialty term, cards with more specialty priority give more hints than predicted;
with it, the excess is gone:

| Specialty priority | Rows | Observed / predicted, without the term | with it |
|---|---|---|---|
| 0 | {{7.specialty_band_0_rows|,.0f}} | {{7.specialty_band_0_before|.3f}} | {{7.specialty_band_0_after|.3f}} |
| 1-29 | {{7.specialty_band_1_29_rows|,.0f}} | {{7.specialty_band_1_29_before|.3f}} | {{7.specialty_band_1_29_after|.3f}} |
| 30-59 | {{7.specialty_band_30_59_rows|,.0f}} | {{7.specialty_band_30_59_before|.3f}} | {{7.specialty_band_30_59_after|.3f}} |
| 60+ | {{7.specialty_band_60_rows|,.0f}} | {{7.specialty_band_60_before|.3f}} | {{7.specialty_band_60_after|.3f}} |

- It predicts careers it was not fitted to: held out, the term gains {{7.held_out_specialty_term_vs_none_nats|.1f}} ± {{7.held_out_specialty_term_vs_none_se|.1f}} nats, $z$ = {{7.held_out_specialty_term_vs_none_z|.1f}}; fitted on the earlier
  half of the runs, it predicts the later half better ({{7.held_out_later_runs_specialty_term_vs_none_nats|.1f}} ± {{7.held_out_later_runs_specialty_term_vs_none_se|.1f}} nats, $z$ = {{7.held_out_later_runs_specialty_term_vs_none_z|.1f}}).
- A weight per scenario predicts better than one shared weight ({{7.held_out_per_scenario_d_vs_one_d_nats|.1f}} ± {{7.held_out_per_scenario_d_vs_one_d_se|.1f}} nats, $z$ = {{7.held_out_per_scenario_d_vs_one_d_z|.1f}}).
- Split by scenario and specialty band, the {{7.specialty_cells_df|.0f}} cells with 200 or more rows are jointly consistent with the
  model (run-clustered $\chi^2$ = {{7.specialty_cells_joint_chi2|.1f}}, p-value {{7.specialty_cells_p|.2f}}).
- It is not one card's, one rarity's or one period's: one scale $k$ on the adopted per-scenario weights ($k$ = 1 on
  every row), refitted on either half of the runs, on SR or SSR cards alone, and without any one of the eight
  most-used cards, stays between {{7.specialty_scale_k_robustness_min|.2f}} and {{7.specialty_scale_k_robustness_max|.2f}}.
- Cause or association: the same card at different limit breaks has different specialty priority but is otherwise
  the same card. On most such cards hint frequency changes with limit break too, so their weight
  ({{7.specialty_same_card_d|.2f}} [{{7.specialty_same_card_d_95_pct_low|u.2f}}, {{7.specialty_same_card_d_95_pct_high|.2f}}] over {{7.specialty_same_card_cards|.0f}} cards, a base per card and scenario) also absorbs any misfit of the
  hint-frequency term. On the {{7.specialty_same_card_clean_cards|.0f}} clean cards, whose hint frequency is the same at every limit break, it is
  {{7.specialty_same_card_clean_d|u.2f}} [{{7.specialty_same_card_clean_d_95_pct_low|u.2f}}, {{7.specialty_same_card_clean_d_95_pct_high|.2f}}] ({{7.specialty_same_card_clean_gain_nats|+.2f}} nats over no term): consistent with no effect and with every per-scenario
  weight, so the data do not decide whether specialty priority itself causes the hints.

*Hint frequency's slope.* The $1/100$ in $(1 + hf/100)$ is **assumed** (a card's +40% read as 40% more hints) and
tested: on the adopted model a fitted slope ($hf/{{7.specialty_model_free_divisor_c|.0f}}$) is detectable in-sample but does not predict unseen careers
better ({{7.held_out_specialty_model_free_divisor_vs_1_per_100_nats|.1f}} ± {{7.held_out_specialty_model_free_divisor_vs_1_per_100_se|.1f}} nats, $z$ = {{7.held_out_specialty_model_free_divisor_vs_1_per_100_z|.1f}}). In the form study below, without the specialty term, a free divisor
($c$ = {{7.free_divisor_c|.0f}}) does predict unseen careers better ({{7.held_out_free_divisor_vs_1_per_100_nats|.1f}} ± {{7.held_out_free_divisor_vs_1_per_100_se|.1f}} nats, $z$ = {{7.held_out_free_divisor_vs_1_per_100_z|.1f}}). The adopted model keeps $1/100$: an
assumption that, with the specialty term in, unseen careers do not reject.

*The form.* Fixed on every card at any limit break, R cards included, without the specialty term
({{7.form_rows|,.0f}} rows from {{7.form_runs|,.0f}} runs):

| Question | Evidence |
|---|---|
| Rate form, LL on {{7.form_rows|,.0f}} rows | $1 + hf/25$ over options {{7.ll_1_hf_per_25_over_options|u,.1f}}; $1 + hf/25$ over skills {{7.ll_1_hf_per_25_over_skills|u,.1f}}; constant {{7.ll_constant|u,.1f}}; $\beta(1 + hf/100)$ {{7.ll_base_1_hf_per_100|u,.1f}}; per-scenario $\beta(1 + hf/100)$ {{7.ll_per_scenario_base_1_hf_per_100|u,.1f}}; free divisor ($c = {{7.free_divisor_c|.0f}}$) {{7.ll_free_divisor|u,.1f}}. Held out, per-scenario $\beta(1 + hf/100)$ predicts unseen runs better than the constant ({{7.held_out_per_scenario_bases_vs_constant_nats|.1f}} ± {{7.held_out_per_scenario_bases_vs_constant_se|.1f}}, $z$ = {{7.held_out_per_scenario_bases_vs_constant_z|.1f}}) and the $hf/25$ forms ({{7.held_out_per_scenario_bases_vs_1_hf_per_25_over_options_nats|.1f}} ± {{7.held_out_per_scenario_bases_vs_1_hf_per_25_over_options_se|.1f}}, $z$ = {{7.held_out_per_scenario_bases_vs_1_hf_per_25_over_options_z|.1f}}; {{7.held_out_per_scenario_bases_vs_1_hf_per_25_over_skills_nats|.1f}} ± {{7.held_out_per_scenario_bases_vs_1_hf_per_25_over_skills_se|.1f}}, $z$ = {{7.held_out_per_scenario_bases_vs_1_hf_per_25_over_skills_z|.1f}}) |
| Skills only, not skills + stat option | in {{7.cells_var_below_stat_option_floor|.0f}} of {{7.cells_card_hf|.0f}} (card, $hf$) cells the draw-count variance is below the binomial floor a drawn stat option implies |
| Uniform draws with repeats | repeat rate $P(k \ge 2 \mid k \ge 1)$: {{7.repeat_rate_observed|.3f}} observed, {{7.repeat_rate_two_point_count|.3f}} two-point count, {{7.repeat_rate_poisson_count|.3f}} Poisson count; distinct skills 1/2/3/4 per card-run {{7.distinct_1_observed|.3f}} / {{7.distinct_2_observed|.3f}} / {{7.distinct_3_observed|.3f}} / {{7.distinct_4_observed|.3f}} observed, {{7.distinct_1_model|.3f}} / {{7.distinct_2_model|.3f}} / {{7.distinct_3_model|.3f}} / {{7.distinct_4_model|.3f}} model |
| Covariates | per-scenario bases +{{7.per_scenario_bases_gain_nats|.0f}} nats (form study; held out {{7.held_out_per_scenario_bases_vs_one_base_nats|.1f}} ± {{7.held_out_per_scenario_bases_vs_one_base_se|.1f}}, $z$ = {{7.held_out_per_scenario_bases_vs_one_base_z|.1f}}). On top of the adopted rate ({{7.covariate_rows|,.0f}} rows): card type +{{7.covariate_gain_card_type_nats|.1f}} nats (4 parameters), initial bond +{{7.covariate_gain_initial_bond_nats|.1f}}, pool size +{{7.covariate_gain_pool_size_nats|.1f}}, none significant; rarity (SR against SSR) +{{7.covariate_gain_sr_nats|.1f}} and pals per deck +{{7.covariate_gain_pals_per_deck_nats|.1f}} are detectable; pals per deck predicts unseen runs better (held out {{7.held_out_covariate_pals_per_deck_vs_none_nats|.1f}} ± {{7.held_out_covariate_pals_per_deck_vs_none_se|.1f}}, $z$ = {{7.held_out_covariate_pals_per_deck_vs_none_z|.1f}}; a known miss, factor {{7.covariate_factor_pals_per_deck|.3f}} per pal), rarity does not ({{7.held_out_covariate_sr_vs_none_nats|.1f}} ± {{7.held_out_covariate_sr_vs_none_se|.1f}}, $z$ = {{7.held_out_covariate_sr_vs_none_z|.1f}}; factor {{7.covariate_factor_sr|.3f}} for SR) |
| Per card | on the adopted rate with a level per scenario, the {{7.per_card_cards|.0f}} cards with 200 or more rows ({{7.per_card_rows|,.0f}} rows) read {{7.per_card_obs_per_pred_min|.2f}}-{{7.per_card_obs_per_pred_max|.2f}} of prediction, jointly far from it (run-clustered $\chi^2$ = {{7.per_card_wald_chi2|.0f}} on {{7.per_card_wald_df|.0f}}, $p$ = {{7.per_card_wald_p|.0e}}); a factor per card predicts unseen runs better ({{7.held_out_a_factor_per_card_vs_none_nats|.1f}} ± {{7.held_out_a_factor_per_card_vs_none_se|.1f}}, $z$ = {{7.held_out_a_factor_per_card_vs_none_z|.1f}}): a known miss, not modelled; which card properties explain it is open |
| Lv-5 cap | beats drawing with replacement by {{7.cap_gain_nats|.1f}} nats with no parameter; obs / pred by draws to Lv 5, replacement vs cap: 5 draws ({{7.cap_table_5_draws_rows|,.0f}} rows) {{7.cap_table_5_draws_replacement|.3f}} / {{7.cap_table_5_draws_cap|.3f}}, 3 ({{7.cap_table_3_draws_rows|,.0f}}) {{7.cap_table_3_draws_replacement|.3f}} / {{7.cap_table_3_draws_cap|.3f}}, 2 ({{7.cap_table_2_draws_rows|,.0f}}) {{7.cap_table_2_draws_replacement|.3f}} / {{7.cap_table_2_draws_cap|.3f}}, 1 ({{7.cap_table_1_draws_rows|.0f}}) {{7.cap_table_1_draws_replacement|.3f}} / {{7.cap_table_1_draws_cap|.3f}} |
| Joint draws over targets | {{7.joint_pairs|,.0f}} pairs of no-spark pool skills on one card-run (Trackblazer left out): $P(\text{both})$ {{7.p_both_observed|.4f}} observed, {{7.p_both_joint|.4f}} joint, {{7.p_both_independent|.4f}} independent; $P(\text{neither})$ {{7.p_neither_observed|.4f}} vs {{7.p_neither_joint|.4f}} |
| Pool size | obs / pred {{7.pool_size_obs_per_pred_min|.2f}}-{{7.pool_size_obs_per_pred_max|.2f}} for every $n$ with at least 150 rows |
| Per-draw increment | {{7.increment_multiples_of_iota|,.0f}} of {{7.increment_single_source_levels|,.0f}} single-source levels are multiples of $\iota$, capped at 5 |
| $hf$ by card level | floored linear interpolation {{7.ll_hf_floored_linear|u,.1f}}, step {{7.ll_hf_step|u,.1f}}, max-limit-break $hf$ for all {{7.ll_hf_max_lb_for_all|u,.1f}} |

*Other sources of the same skill.* The fit uses rows where no inspiration hinted the skill. Where something else also
hinted it, the card's own hint is seen this often (scored against the adopted rate on maximum-limit-break cards, R
cards out):

| Other source of the skill | Rows | Observed | Predicted | Ratio |
|---|---|---|---|---|
| none | {{7.spark_none_rows|,.0f}} | {{7.spark_none_observed|.3f}} | {{7.spark_none_predicted|.3f}} | {{7.spark_none_ratio|.2f}} |
| inspiration | {{7.spark_inspiration_rows|,.0f}} | {{7.spark_inspiration_observed|.3f}} | {{7.spark_inspiration_predicted|.3f}} | {{7.spark_inspiration_ratio|.2f}} |
| inspiration + card event | {{7.spark_inspiration_card_event_rows|,.0f}} | {{7.spark_inspiration_card_event_observed|.3f}} | {{7.spark_inspiration_card_event_predicted|.3f}} | {{7.spark_inspiration_card_event_ratio|.2f}} |
| card event | {{7.spark_card_event_rows|,.0f}} | {{7.spark_card_event_observed|.3f}} | {{7.spark_card_event_predicted|.3f}} | {{7.spark_card_event_ratio|.2f}} |
| other card's training | {{7.spark_other_card_training_rows|,.0f}} | {{7.spark_other_card_training_observed|.3f}} | {{7.spark_other_card_training_predicted|.3f}} | {{7.spark_other_card_training_ratio|.2f}} |

With no other source the adopted rate slightly under-predicts ({{7.spark_none_obs_per_pred|.3f}} [{{7.spark_none_obs_per_pred_95_pct_low|.3f}}, {{7.spark_none_obs_per_pred_95_pct_high|.3f}}]). A skill hinted by inspiration
is hinted by training less than predicted (not modelled). It is not that such a skill
leaves the card's pool: with two pool skills hinted by inspiration, the rest are hinted at {{7.drop_out_test_observed|.3f}}, against
{{7.drop_out_test_no_drop_out|.3f}} if nothing leaves and {{7.drop_out_test_drop_out|.3f}} if they leave.

*Prioritised skills.* {{7.priority_runs|.0f}} runs show a priority list; prioritised skills are hinted by training at {{7.priority_observed|.3f}} against
{{7.priority_predicted|.3f}} predicted ({{7.priority_rows|.0f}} rows): no effect, as the model assumes.

Against real careers (section 13), training-pool hints read {{13.1_training_pool_only_obs_per_pred|.3f}} of the model.

## 8. Chain events

Computed by `research/s08_chains.py`.

A chain card hints its skills through a fixed sequence of events, so how far a chain gets depends on how many chain
events a career gets and how they are shared among the deck's chain cards. This section fits both.

### Findings

- **A career gets a random number of chain events, Binomial(21, 0.35)** (mean {{8.binomial_21_0_35_mean|.2f}}), the same for every deck and
  scenario (scenarios show no difference; two deck and trainee effects are known misses). No alternative predicts unseen runs better, but in-sample a free distribution rejects it, a known miss;
  of the two-parameter under-dispersed families the binomial fits best. Binomial(21, 0.35) is kept by choice;
  Binomial(18, 0.405) is the candidate the runs now favour in-sample.
- **Each event goes to a chain card in proportion to its remaining links.** This fits {{8.allocation_slot_over_card_nats|.0f}} nats better than equal shares
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

$$N \sim \mathrm{Binomial}(21, 0.35), \qquad P(N = e) = \binom{21}{e}\, 0.35^{e}\, 0.65^{21-e}, \quad E N = {{8.binomial_21_0_35_mean|.2f}},\ \mathrm{Var}\, N = {{8.binomial_21_0_35_variance|.2f}}.$$

Each event advances one non-pal chain card by one link. Card $i$ is picked with

$$P(i) = \frac{w_i}{\sum_j w_j}, \qquad w_i = L_i - d_i \text{ while running, } 0 \text{ once stopped},$$

that is, uniformly over the remaining chain slots (**fitted** against the alternative, below). Pal and group cards take
no slot (**assumed**, supported below). Without stop risk, $P(\text{all chains complete}) = P(N \ge \sum_i L_i)$; for
two 3-link cards, $P(N \ge 6) = {{8.two_3_link_cards_p_n_ge_6_p_both_chains_complete|.2f}}$.

*Stop risk* (**derived** from the event data and the choice rule, section 9). If IT's default choice at link $l$ ends
the chain on $k$ of its $n$ outcomes, $q_l = k/n$, and the stopping link $\sigma$ has
$P(\sigma = l) = q_l \prod_{l' < l} (1 - q_{l'})$. A stopped chain's slots return to the pool. The model enumerates
the exact distribution of final chain states (no simulation); an independent recomputation of the model's odds
agrees to {{6.schedules_recomputed_odds_max_relative_error|.0e}} on every reference loop (section 6).

### Evidence

*How is a chain's progress read?* From marker skills, on {{8.fit_of_n_runs|,.0f}} runs (the pal-count table below). A link is observable
only if every choice and outcome of its event carries a skill appearing in no other event of that card. A credit
naming two sources ("A or B event") counts as unknown when B is another deck card whose events can give the marker, and
so does a marker listed only under inspiration. $N$'s distribution is the one that best explains progress over all
decks.

The fit allows a fired link's marker to go unseen with chance 0.02 (a measurement allowance, set to the measured rate). Without it,
a run in which a card's later link shows while an earlier one does not has likelihood 0. Since links fire in order,
those runs measure the rate: of {{8.allowance_earlier_links_implied_fired|,.0f}} earlier links implied fired by a later one, {{8.allowance_marker_not_shown|.0f}} show no marker, {{8.allowance_share|.3f}}
[{{8.allowance_95_pct_low|.3f}}, {{8.allowance_95_pct_high|.3f}}]. The cause is not known. The fit does not depend on it: with allowances from 0.01 to 0.045, or
with none on the {{8.allowance_0_runs_without_a_contradiction|,.0f}} runs that show no such case, the best $p$ at $n = 21$ is {{8.allowance_sensitivity_lowest_best_p_at_n_21|.2f}}-{{8.allowance_sensitivity_highest_best_p_at_n_21|.2f}}.

*Which distribution for $N$?* In-sample, against the free distribution:

| Form for $N$ | Result |
|---|---|
| free distribution | mean {{8.distribution_free_mean|.2f}}, variance {{8.distribution_free_variance|.2f}} |
| Binomial(21, 0.35) | {{8.binomial_21_0_35_nats_behind_distribution_free|.1f}} nats behind the free distribution with {{8.distribution_free_fit_parameters_more_than_the_binomial|.0f}} fewer parameters (rejected, p-value {{8.binomial_21_0_35_lr_p|.1e}}) |
| Binomial($n$, $p$), both fitted | best at $n$ = {{8.best_binomial_n_free_n|.0f}}, $p$ = {{8.best_binomial_n_free_p|.3f}}: {{8.best_binomial_n_free_nats_behind_distribution_free|.1f}} nats behind (p-value {{8.best_binomial_n_free_lr_p|.3f}}); $n$ is only known to lie in {{8.binomial_n_95_pct_profile_interval_low|.0f}}-{{8.binomial_n_95_pct_profile_interval_high|.0f}} |
| Conway-Maxwell-Poisson | {{8.conway_maxwell_poisson_nats_behind_distribution_free|.1f}} nats behind (p-value {{8.conway_maxwell_poisson_lr_p|.3f}}) |
| Binomial(15, 0.48) | {{8.binomial_15_0_48_nats_behind_distribution_free|.1f}} nats behind |
| Poisson(7.1) | {{8.poisson_7_1_nats_behind_distribution_free|.1f}} nats behind (variance = mean, too wide; rejected) |
| per scenario | means Unity Cup {{8.per_scenario_mean_unity_cup|.2f}}, Grand Concert {{8.per_scenario_mean_grand_concert|.2f}}, URA Finale {{8.per_scenario_mean_ura_finale|.2f}}, Trackblazer {{8.per_scenario_mean_trackblazer|.2f}}; +{{8.per_scenario_log_likelihood_gain|.1f}} nats for {{8.per_scenario_extra_parameters|.0f}} extra parameters (not significant) |

Why a binomial: the free fit's variance is {{8.distribution_free_variance_per_mean|.2f}} of its mean, so $N$ is under-dispersed, which rules out the
Poisson and every over-dispersed count. A binomial (a fixed number of chances, each taken with the same probability)
is the simplest under-dispersed count, and it fits better than the Conway-Maxwell-Poisson, the other two-parameter
family that can be under-dispersed (by {{8.best_binomial_n_free_over_conway_maxwell_poisson_nats|.1f}} nats, same parameter count). What the model uses is the distribution of
$N$.

How $n$ and $p$ are read: fitting both $n$ and $p$, the runs fix the mean ({{8.binomial_mean_n_p_at_the_best_n|.2f}} at the best $n$) and, more
loosely, the spread, so $n$ and $p$ trade off along a ridge. Re-fitting $p$ at each $n$, every $n$ from
{{8.binomial_n_95_pct_profile_interval_low|.0f}} to {{8.binomial_n_95_pct_profile_interval_high|.0f}} lies within 1.92 nats of the best (the 95% interval), so the best single pair is not itself a finding. The
adopted $n = 21$ lies just outside that interval in-sample ({{8.binomial_n_21_p_re_fitted_nats_below_the_best_n|.2f}} nats below the best $n$; variance
$21 \cdot 0.35 \cdot 0.65 = {{8.binomial_21_0_35_variance|.2f}}$ against {{8.distribution_free_variance|.2f}} free).

*Does that in-sample preference survive held out?* No. Each of five groups of runs is scored by every form fitted on
the other four, and each form is compared run by run with Binomial(21, 0.35):

| Form, fitted on the other four groups | Held-out nats over (21, 0.35) ± SE | $z$ |
|---|---|---|
| free distribution | {{8.held_out_free_over_binomial_21_0_35_nats|u+.1f}} ± {{8.held_out_free_over_binomial_21_0_35_se|.1f}} | {{8.held_out_free_over_binomial_21_0_35_z|u+.2f}} |
| binomial, $n$ and $p$ by likelihood | {{8.held_out_binomial_by_likelihood_over_binomial_21_0_35_nats|u+.1f}} ± {{8.held_out_binomial_by_likelihood_over_binomial_21_0_35_se|.1f}} | {{8.held_out_binomial_by_likelihood_over_binomial_21_0_35_z|u+.2f}} |
| binomial by moments | {{8.held_out_binomial_by_moments_over_binomial_21_0_35_nats|u+.1f}} ± {{8.held_out_binomial_by_moments_over_binomial_21_0_35_se|.1f}} | {{8.held_out_binomial_by_moments_over_binomial_21_0_35_z|u+.2f}} |
| Conway-Maxwell-Poisson | {{8.held_out_conway_maxwell_poisson_over_binomial_21_0_35_nats|u+.1f}} ± {{8.held_out_conway_maxwell_poisson_over_binomial_21_0_35_se|.1f}} | {{8.held_out_conway_maxwell_poisson_over_binomial_21_0_35_z|u+.2f}} |
| Poisson | {{8.held_out_poisson_over_binomial_21_0_35_nats|u+.1f}} ± {{8.held_out_poisson_over_binomial_21_0_35_se|.1f}} | {{8.held_out_poisson_over_binomial_21_0_35_z|u+.2f}} |

No form predicts unseen runs better than (21, 0.35) ($z$ above 1.96 would), and the Poisson predicts them worse. The
model keeps (21, 0.35), a decision rather than a fit: in-sample it is rejected, and the candidate Binomial(18, 0.405)
(mean {{8.candidate_binomial_18_0_405_mean|.2f}}, variance {{8.candidate_binomial_18_0_405_variance|.2f}}, inside the profile interval) fits the runs {{8.candidate_binomial_18_0_405_over_binomial_21_0_35_in_sample_nats|.1f}} nats better, while a binomial refitted
by likelihood on each four groups gains {{8.held_out_binomial_by_likelihood_over_binomial_21_0_35_nats|.1f}} ± {{8.held_out_binomial_by_likelihood_over_binomial_21_0_35_se|.1f}} nats held out ($z$ = {{8.held_out_binomial_by_likelihood_over_binomial_21_0_35_z|.2f}}), short of significance.

*Does $N$ depend on the deck or trainee?* For two, yes. Decks with Sasami Anshinzawa [This Might Sting!]'s pal card
({{8.sasami_anshinzawa_s_card_runs|,.0f}} runs) reach a mean of {{8.sasami_anshinzawa_s_card_mean_of_n|.2f}} chain events against {{8.sasami_anshinzawa_s_card_mean_of_n_the_other_runs|.2f}} on the other runs ($p$ at $n = 21$ {{8.sasami_anshinzawa_s_card_p_at_n_21|.3f}}
against {{8.sasami_anshinzawa_s_card_p_at_n_21_the_other_runs|.3f}}, likelihood-ratio $p$ = {{8.sasami_anshinzawa_s_card_p_at_n_21_on_each_side_lr_p|.0e}}); Inari One's careers ({{8.inari_one_runs|,.0f}}) reach {{8.inari_one_mean_of_n|.2f}} against {{8.inari_one_mean_of_n_the_other_runs|.2f}}
($p$ {{8.inari_one_p_at_n_21|.3f}} against {{8.inari_one_p_at_n_21_the_other_runs|.3f}}, likelihood-ratio $p$ = {{8.inari_one_p_at_n_21_on_each_side_lr_p|.3f}}). The model has one $N$ for every deck and
trainee, so it under-predicts the first and over-predicts the second (known misses). On the runs with neither,
Binomial(21, 0.35) is still rejected against a free distribution ($p$ = {{8.binomial_21_0_35_lr_p_runs_with_neither|.3f}}).

*Do pals change $N$?* No. Fitted per pal count, the means are {{8.per_pal_count_mean_1_pals|.2f}} / {{8.per_pal_count_mean_2_pals|.2f}} / {{8.per_pal_count_mean_3_pals|.2f}} / {{8.per_pal_count_mean_4_pals|.2f}} for
1 / 2 / 3 / 4 pals ({{8.per_pal_count_runs_1_4_pals|,.0f}} runs): more pals do not lower $N$, so pal events do not consume chain events. (Per
scenario, see the table above.)

*Are final links reached as often as predicted?* Not everywhere: the binomial lies inside the observed 95% interval
at 3, 1 and 0 pals, above it at 5 and 2 pals and below it at 4 pals.

| Pals | Chain cards | Runs | Final link reached: observed [95%] / Binomial(21, 0.35) / Poisson(7.1) |
|---|---|---|---|
| 5 | 1 | {{8.final_link_table_5_pals_runs|,.0f}} | {{8.final_link_table_5_pals_observed|.3f}} [{{8.final_link_table_5_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_5_pals_observed_95_pct_high|.3f}}] / {{8.final_link_table_5_pals_binomial_21_0_35|.3f}} / {{8.final_link_table_5_pals_poisson_7_1|.3f}} |
| 4 | 2 | {{8.final_link_table_4_pals_runs|,.0f}} | {{8.final_link_table_4_pals_observed|.3f}} [{{8.final_link_table_4_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_4_pals_observed_95_pct_high|.3f}}] / {{8.final_link_table_4_pals_binomial_21_0_35|.3f}} / {{8.final_link_table_4_pals_poisson_7_1|.3f}} |
| 3 | 3 | {{8.final_link_table_3_pals_runs|,.0f}} | {{8.final_link_table_3_pals_observed|.3f}} [{{8.final_link_table_3_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_3_pals_observed_95_pct_high|.3f}}] / {{8.final_link_table_3_pals_binomial_21_0_35|.3f}} / {{8.final_link_table_3_pals_poisson_7_1|.3f}} |
| 2 | 4 | {{8.final_link_table_2_pals_runs|,.0f}} | {{8.final_link_table_2_pals_observed|.3f}} [{{8.final_link_table_2_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_2_pals_observed_95_pct_high|.3f}}] / {{8.final_link_table_2_pals_binomial_21_0_35|.3f}} / {{8.final_link_table_2_pals_poisson_7_1|.3f}} |
| 1 | 5 | {{8.final_link_table_1_pals_runs|,.0f}} | {{8.final_link_table_1_pals_observed|.3f}} [{{8.final_link_table_1_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_1_pals_observed_95_pct_high|.3f}}] / {{8.final_link_table_1_pals_binomial_21_0_35|.3f}} / {{8.final_link_table_1_pals_poisson_7_1|.3f}} |
| 0 | 6 | {{8.final_link_table_0_pals_runs|,.0f}} | {{8.final_link_table_0_pals_observed|.3f}} [{{8.final_link_table_0_pals_observed_95_pct_low|.3f}}, {{8.final_link_table_0_pals_observed_95_pct_high|.3f}}] / {{8.final_link_table_0_pals_binomial_21_0_35|.3f}} / {{8.final_link_table_0_pals_poisson_7_1|.3f}} |

*Spread over slots, or over cards?* Over slots. The alternative gives each running card's next event the same chance
(uniform over cards). On {{8.allocation_runs|,.0f}} runs:

| | Uniform over remaining slots (adopted) | Uniform over cards |
|---|---|---|
| Total LL | {{8.allocation_total_ll_slot_rule|u,.1f}} | {{8.allocation_total_ll_card_rule|u,.1f}} |
| Final links, 2 pals (observed {{8.allocation_final_links_2_pals_observed|.2f}}) | {{8.allocation_final_links_2_pals_slot_rule|.2f}} | {{8.allocation_final_links_2_pals_card_rule|.2f}} |
| Final links, 0 pals (observed {{8.allocation_final_links_0_pals_observed|.2f}}) | {{8.allocation_final_links_0_pals_slot_rule|.2f}} | {{8.allocation_final_links_0_pals_card_rule|.2f}} |

The slot rule fits better by {{8.allocation_slot_over_card_nats|.1f}} nats. With no pals it lies inside the observed interval and the card rule
outside; with two pals both predict more final links than seen, the card rule further. What each rule implies with
Binomial(21, 0.35), as P(reach link 1 / 2 / 3):

| Deck | Slots (adopted) | Cards |
|---|---|---|
| two 3-link SSRs | {{8.two_3_link_ssrs_slots_rule_p_reach_link_1|.2f}} / {{8.two_3_link_ssrs_slots_rule_p_reach_link_2|.2f}} / {{8.two_3_link_ssrs_slots_rule_p_reach_link_3|.2f}} | {{8.two_3_link_ssrs_cards_rule_p_reach_link_1|.2f}} / {{8.two_3_link_ssrs_cards_rule_p_reach_link_2|.2f}} / {{8.two_3_link_ssrs_cards_rule_p_reach_link_3|.2f}} |
| 3-link SSR + 2-link SR | SSR {{8.3_link_ssr_2_link_sr_ssr_slots_rule_p_reach_link_1|.2f}} / {{8.3_link_ssr_2_link_sr_ssr_slots_rule_p_reach_link_2|.2f}} / {{8.3_link_ssr_2_link_sr_ssr_slots_rule_p_reach_link_3|.2f}}, SR {{8.3_link_ssr_2_link_sr_sr_slots_rule_p_reach_link_1|.2f}} / {{8.3_link_ssr_2_link_sr_sr_slots_rule_p_reach_link_2|.2f}} | SSR {{8.3_link_ssr_2_link_sr_ssr_cards_rule_p_reach_link_1|.2f}} / {{8.3_link_ssr_2_link_sr_ssr_cards_rule_p_reach_link_2|.2f}} / {{8.3_link_ssr_2_link_sr_ssr_cards_rule_p_reach_link_3|.2f}}, SR {{8.3_link_ssr_2_link_sr_sr_cards_rule_p_reach_link_1|.2f}} / {{8.3_link_ssr_2_link_sr_sr_cards_rule_p_reach_link_2|.2f}} |
| three 3-link SSRs | {{8.three_3_link_ssrs_slots_rule_p_reach_link_1|.2f}} / {{8.three_3_link_ssrs_slots_rule_p_reach_link_2|.2f}} / {{8.three_3_link_ssrs_slots_rule_p_reach_link_3|.2f}} | {{8.three_3_link_ssrs_cards_rule_p_reach_link_1|.2f}} / {{8.three_3_link_ssrs_cards_rule_p_reach_link_2|.2f}} / {{8.three_3_link_ssrs_cards_rule_p_reach_link_3|.2f}} |
| four 3-link SSRs | {{8.four_3_link_ssrs_slots_rule_p_reach_link_1|.2f}} / {{8.four_3_link_ssrs_slots_rule_p_reach_link_2|.2f}} / {{8.four_3_link_ssrs_slots_rule_p_reach_link_3|.2f}} | {{8.four_3_link_ssrs_cards_rule_p_reach_link_1|.2f}} / {{8.four_3_link_ssrs_cards_rule_p_reach_link_2|.2f}} / {{8.four_3_link_ssrs_cards_rule_p_reach_link_3|.2f}} |

For example, with two 3-link cards and exactly 3 events, the slot rule gives one card all three with chance
$\tfrac36 \cdot \tfrac25 \cdot \tfrac14$ = {{8.two_3_link_cards_exactly_3_events_slots_rule_p_final_state_3_0|.1%}}, the card rule $(\tfrac12)^3$ = {{8.two_3_link_cards_exactly_3_events_cards_rule_p_final_state_3_0|.1%}}.

*Do two-chain decks finish as predicted?* Yes. Decks with two 3-link SSRs and four pals finish both chains in
{{8.two_chain_decks_both_chains_finish|.0f}} of {{8.two_chain_decks_runs|.0f}} runs, {{8.two_chain_decks_share|.2f}} [{{8.two_chain_decks_95_pct_low|.2f}}, {{8.two_chain_decks_95_pct_high|.2f}}], against {{8.two_chain_decks_predicted_p_n_ge_6|.2f}} predicted.

*Known miss: early links in crowded decks run short.* The shortfall is the share of early links the model expects to
fire minus the share seen, ± its run-clustered standard error; $z = \sum(\text{pred} - \text{obs}) /
\sqrt{\sum(\text{pred} - \text{obs})^2}$, with each run's links summed as one cluster:

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Early-link shortfall | {{8.early_link_shortfall_4_pals|.2f}} ± {{8.early_link_shortfall_4_pals_pm|.2f}} | {{8.early_link_shortfall_3_pals|.2f}} ± {{8.early_link_shortfall_3_pals_pm|.2f}} | {{8.early_link_shortfall_2_pals|.2f}} ± {{8.early_link_shortfall_2_pals_pm|.2f}} | {{8.early_link_shortfall_1_pals|.2f}} ± {{8.early_link_shortfall_1_pals_pm|.2f}} | {{8.early_link_shortfall_0_pals|.2f}} ± {{8.early_link_shortfall_0_pals_pm|.2f}} |
| $z$ | {{8.early_link_shortfall_4_pals_z|.1f}} | {{8.early_link_shortfall_3_pals_z|.1f}} | {{8.early_link_shortfall_2_pals_z|.1f}} | {{8.early_link_shortfall_1_pals_z|.1f}} | {{8.early_link_shortfall_0_pals_z|.1f}} |

*Known miss: individual cards' final links.* Pooled, final links fit ({{8.final_links_by_card_pooled_observed|.3f}} observed against {{8.final_links_by_card_pooled_model|.3f}}). Card by
card they do not: over the {{8.final_links_by_card_cards|.0f}} cards with 60 or more final-link observations, a joint test that every card
matches the model (clustered by run, since one run's chain cards share its $N$) gives $\chi^2$ = {{8.final_links_by_card_chi2|.0f}} on
{{8.final_links_by_card_cards|.0f}} degrees of freedom, where chance alone gives about {{8.final_links_by_card_cards|.0f}}. The cause is not known.

*Known miss: chains that never start.* More chains never start in crowded decks than predicted:
{{8.never_started_5_chain_cards_observed|.2f}} against {{8.never_started_5_chain_cards_model|.2f}} with five chain cards, {{8.never_started_6_chain_cards_observed|.2f}} against {{8.never_started_6_chain_cards_model|.2f}} with six. A per-card activation parameter (a chain
card active with probability $a$), fitted jointly with $N$, gains {{8.activation_corrected_reading_joint_with_n_gain_nats|u.1f}} nats ($a = {{8.activation_corrected_reading_joint_with_n_a|.2f}}$): rejected.

## 9. Event choices and outcomes

Computed by `research/s09_outcomes.py` (the certainty checks: `research/s13_calibration.py`).

An event can put different skills on different choices, and an outcome can be random. This section states which
choice IT takes and measures the outcome shares that are not equal.

### Findings

- **The model assumes IT takes an event's top choice unless a prioritised skill sits on another**, and that the
  player prioritises every target. In the two events where priority lists show the choice taken, IT follows the rule.
  Its consequence shows on real careers: event-only hints of targets not prioritised, or outranked (below), fall
  clearly below those of prioritised ones (*prioritised* in Terms) ({{9.event_only_hints_non_prioritised_targets_obs_per_pred|.3f}} [{{13.1_events_only_not_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_not_prioritised_ratio_95_pct_high|.3f}}] of the model against {{9.event_only_hints_prioritised_targets_obs_per_pred|.3f}}).
- **Known miss**: event-only hints of prioritised targets read just below the model, {{9.event_only_hints_prioritised_targets_obs_per_pred|.3f}}
  [{{13.1_events_only_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_prioritised_ratio_95_pct_high|.4f}}].
- **Outcomes are equally likely unless measured.** Five measured outcomes depart from equal shares, and the model
  uses measured values for them, for example Nakayama Festa's Nothing Ventured at {{9.festa_share|.2f}}.
- **Some other outcomes are off the equal rule but left at it**: known misses.
- **Pal and group finales are modelled as certain**, and both checked hold: Light Hello and Team Sirius credit their finale
  skill in every run carrying them.

### Terms

- *Choice*: an option of an event. *Top* and *bottom choice*: the first and the last.
- *Outcome*: what a choice gives. *Firing*: one occurrence of an event in a run.
- *Prioritised*: on the run's priority list, and not outranked: a listed target counts as not prioritised when a
  higher-ranked prioritised skill sits on a rival choice of every event offering it, since between two prioritised
  skills on rival choices IT is taken to choose the higher-ranked one ({{9.event_only_hints_outranked_on_a_rival_choice_rows|.0f}} event-only rows; which way the rank
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
| Sasami Anshinzawa [This Might Sting!] finale, Nothing Ventured (Risky Business prioritised; {{9.anshinzawa_runs_with_the_card_no_festa|.0f}} runs with the card) | {{9.anshinzawa_nv_with_rb_prioritised|.0f}}/{{9.anshinzawa_rb_prioritised_runs|.0f}} = {{9.anshinzawa_nv_share|.2f}} [{{9.anshinzawa_nv_share_90_pct_low|.2f}}, {{9.anshinzawa_nv_share_90_pct_high|.2f}}] | {{9.adopted_anshinzawa_finale_nothing_ventured|.2f}} |
| same, choice 2 alone, no inherited Risky Business | {{9.anshinzawa_choice_2_no_inherited_rb_won|.0f}}/{{9.anshinzawa_choice_2_no_inherited_rb_runs|.0f}} = {{9.anshinzawa_choice_2_share|.2f}} [{{9.anshinzawa_choice_2_90_pct_low|.2f}}, {{9.anshinzawa_choice_2_90_pct_high|.2f}}] | |
| Riko Kashimoto [Planned Perfection] "Unexpected Side", Ramp Up (top choice taken) | {{9.riko_ramp_up_top_choice_taken|.0f}}/{{9.riko_readings|.0f}} = {{9.riko_share|.2f}} [{{9.riko_90_pct_low|.2f}}, {{9.riko_90_pct_high|.2f}}] | {{9.adopted_riko_unexpected_side_ramp_up|.2f}} |
| Seiun Sky [Foolproof Plan] link 3, Fast-Paced | {{9.seiun_sky_link_3_fast_paced|.0f}}/{{9.seiun_sky_link_3_readings|.0f}} = {{9.seiun_sky_share|.2f}} [{{9.seiun_sky_90_pct_low|.2f}}, {{9.seiun_sky_90_pct_high|.2f}}] | {{9.adopted_seiun_sky_link_3_fast_paced|.2f}} |
| Smart Falcon [My Umadol Way! ☆] link 3, Center Stage | {{9.smart_falcon_link_3_center_stage|.0f}}/{{9.smart_falcon_link_3_readings|.0f}} = {{9.smart_falcon_share|.2f}} [{{9.smart_falcon_90_pct_low|.2f}}, {{9.smart_falcon_90_pct_high|.2f}}] | {{9.adopted_smart_falcon_link_3_center_stage|.2f}} |
| Nakayama Festa [Trusting the Dice] link 3 "Upping the Ante", Nothing Ventured ({{9.festa_runs_with_the_card|.0f}} runs with the card, below) | {{9.festa_nv_wins|.0f}}/{{9.festa_readings|.0f}} = {{9.festa_share|.2f}} [{{9.festa_share_90_pct_low|.2f}}, {{9.festa_share_90_pct_high|.2f}}] | {{9.festa_adopted|.2f}} |

The other outcomes: Riko Kashimoto's alternative is Maverick ○, Seiun Sky's Escape Artist, Smart Falcon's Prudent
Positioning.

### Evidence

*Does IT follow the choice rule?* Where it can be seen, yes. At Sasami Anshinzawa [This Might Sting!]'s finale, with
Risky Business prioritised IT took choice 2 in about {{9.anshinzawa_rb_prioritised_took_choice_2|.0%}} of runs ({{9.anshinzawa_choice_1_exceptions_with_rb_at_lv_5|.0f}} of the {{9.anshinzawa_choice_1_exceptions_rb_prioritised|.0f}} exceptions had Risky
Business already at hint Lv 5); with neither skill prioritised, {{9.anshinzawa_neither_prioritised_choice_2_won|.0f}} of {{9.anshinzawa_neither_prioritised_runs|.0f}} runs took choice 2. At Riko Kashimoto
[Planned Perfection]'s event, with Lone Wolf prioritised IT took the bottom choice in {{9.riko_lone_wolf_prioritised_bottom_choice|.0f}} of {{9.riko_lone_wolf_prioritised_runs|.0f}} runs. Every run
has a priority list, but the run records show it for only {{9.runs_showing_a_priority_list|.0f}} of {{9.runs|,.0f}} (the rest count as unknown, never as an
empty list). So the rule is tested mainly through its consequences (section 13): event-only hints read
{{9.event_only_hints_prioritised_targets_obs_per_pred|.3f}} [{{13.1_events_only_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_prioritised_ratio_95_pct_high|.4f}}] of the model for prioritised targets and {{9.event_only_hints_non_prioritised_targets_obs_per_pred|.3f}} for the others. The
prioritised rows read just below the model, a known miss; the model's assumption that every target is prioritised
over-predicts the rest by far more.

*How is Sasami Anshinzawa's finale read?* On runs without Nakayama Festa [Trusting the Dice], whose "Upping the Ante"
also gives Nothing Ventured +1/+3 and is credited jointly with it. Hint levels identify the choice: her Nothing
Ventured +3 is choice 1, +1 is choice 2, and a loss on choice 2 shows as her Risky Business at level 2 (storyline
choice 3 +1, finale +1). With inherited Risky Business levels mixed in, choice 2 reads {{9.anshinzawa_choice_2_with_inherited_low|.2f}}-{{9.anshinzawa_choice_2_with_inherited_high|.2f}}. Choice 1
(Nothing Ventured +3 or nothing) reads {{9.anshinzawa_choice_1_won|.0f}}/{{9.anshinzawa_choice_1_runs|.0f}} = {{9.anshinzawa_choice_1_share|.2f}} if the finale always fires; Grand Concert reads lower
(unexplained).

With Risky Business prioritised, IT also takes the bottom choice of her date-start event "A Pleasant Practice
Partner?" (Risky Business +1). GameTora marks that choice as ending her event chain, which for a pal card means her dates. Yet
the finale still fires: at least {{9.anshinzawa_rb_prioritised_runs_showing_the_finale|.0f}} of {{9.anshinzawa_rb_prioritised_runs|.0f}} such runs show its outcome, and her extra Risky Business +1 appears in
{{9.anshinzawa_her_extra_rb_in_choice_2_wins_prioritised|.0f}} of {{9.anshinzawa_choice_2_wins_prioritised|.0f}} choice-2 wins with the priority against {{9.anshinzawa_her_extra_rb_in_choice_2_wins_rb_not_prioritised|.0f}} of {{9.anshinzawa_choice_2_wins_rb_not_prioritised|.0f}} without. IT apparently keeps her dates either
way.

*Why is Riko Kashimoto's adopted share lower than measured?* Because it is a per-run chance: a run gets her
Ramp Up hint in {{9.riko_per_run_runs_with_ramp_up|.0f}} of {{9.riko_per_run_runs|.0f}} runs, {{9.riko_per_run_share|.3f}} [{{9.riko_per_run_90_pct_low|.3f}}, {{9.riko_per_run_90_pct_high|.3f}}], whichever choice IT took. IT took the top choice in
{{9.riko_firings_top_choice|.0f}} of {{9.riko_firings_all|.0f}} firings ({{9.riko_firings_top_choice_rate|.2f}}) and the bottom choice, Lone Wolf, in {{9.riko_firings_bottom_choice|.0f}}.

*How is Nakayama Festa's share read?* On every run with her card ({{9.festa_runs_with_the_card|.0f}}). Her event is credited to her alone with
Nothing Ventured in {{9.festa_nv_wins|.0f}} runs and with Risky Business in {{9.festa_rb_wins|.0f}}. In {{9.festa_rb_shared_credits|.0f}} more, Risky Business is credited to "her or
another card's event". The other card is the only one credited with it in {{9.festa_team_sirius_alone_rb_rate|.3f}} of runs without her (Team Sirius, {{9.festa_shared_with_team_sirius|.0f}} of
the {{9.festa_rb_shared_credits|.0f}}) or {{9.festa_anshinzawa_alone_rb_rate|.3f}} (Sasami Anshinzawa), and she gives it in {{9.festa_her_rb_rate|.3f}} of hers. So a shared credit is hers with
chance $r_F / (1 - (1 - r_F)(1 - r_o))$, where $r_F$ is her rate and $r_o$ the other card's: {{9.festa_shared_credits_hers|.1f}} of the {{9.festa_rb_shared_credits|.0f}}.
In {{9.festa_blank_risky_business_at_lv_5|.0f}} runs neither outcome shows while Risky Business is already at hint Lv 5 (a Risky Business win cannot
show, or the event never fired); they are split by the rate at which the event shows where both outcomes can,
{{9.festa_firing_rate_where_both_can_show|.2f}}. The share is then {{9.festa_nv_wins|.0f}}/{{9.festa_readings|.0f}} = {{9.festa_share|.2f}} [{{9.festa_share_90_pct_low|.2f}}, {{9.festa_share_90_pct_high|.2f}}]. It stays within {{9.festa_share_shared_all_hers|.3f}}-{{9.festa_share_shared_none_hers|.3f}} whatever share of the
shared credits is hers, and shows no dependence on the trainee's final Wit.

*Which shares are left at the equal rule?* Off 0.5 but left at the equal rule (known misses): Tazuna Hayakawa SSR
finale Concentration {{9.tazuna_finale_concentration|.2f}}, Agnes Tachyon [Q≠0] link 1 choice 2 Murmur {{9.tachyon_link_1_choice_2_murmur|.2f}}, Marvelous Sunday [Dazzling Day in
the Snow] link 3 Position Pilfer {{9.marvelous_sunday_link_3_position_pilfer|.2f}}, Light Hello SSR finale +3 See Ya Later! {{9.light_hello_finale_see_ya_later_3|,.0f}} of {{9.light_hello_finale_readings|,.0f}}
({{9.light_hello_finale_share|.2f}}). Consistent with 0.5: Gold Ship [That Time I Became the Strongest] {{9.gold_ship_link_3_innate_experience|.2f}}, King Halo [Tonight, We
Waltz] {{9.king_halo_link_3_gap_closer|.2f}}, Daitaku Helios [Sunlit Angel ♪] Tail Held High {{9.helios_link_2_tail_held_high_given_link_3|.2f}} (link 2, on runs where link 3 visibly fired).

*Are pal and group events certain?* Their events are entered per card with a probability, finales certain; first-year
date events are left out in IT (**assumed**). The finales check out: Light Hello [From the Ground Up] credits See Ya
Later! to her event in {{9.light_hello_credited_credited|.0f}} of {{9.light_hello_credited_runs_carrying_her|.0f}} runs carrying her, and Team Sirius is credited with Risky Business (alone or shared) in {{9.team_sirius_credited_credited|.0f}} of {{9.team_sirius_credited_runs_carrying_her|.0f}}.

## 10. Random card events (not modelled)

Computed by `research/s10_random_events.py`.

The model leaves some events out. This section measures how often a deck character's shared random event fires and
what leaving it out costs on real careers.

### Findings

- **A deck character's shared random event fires in about {{10.constant_p|.2f}} of the careers it can fire in**, a lower bound. The
  rate falls a little as more such events are on offer, but a constant rate predicts unseen runs as well.
- **Leaving them out under-predicts the targets they can hint**: acquisition there reads {{10.calibration_obs_per_pred_left_out|.2f}} [{{10.calibration_ratio_95_pct_low_left_out|.2f}}, {{10.calibration_ratio_95_pct_high_left_out|.2f}}] of the
  model. Credited at the fitted rate, it reads {{10.calibration_obs_per_pred_credited|.2f}}.
- **Crediting them is not shown to predict unseen careers better** ($z$ = {{10.held_out_credited_against_left_out_z|.2f}}, short of 1.96).
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

The model leaves these events out. The fitted rate (**fitted**) is about {{10.constant_p|.2f}} per event
per career. It falls a little with $K$:

$$P(\text{fires}) = {{10.fitted_p_at_k_4|.2f}}\,(1 - {{10.fitted_form_1_ratio_per_event|.3f}})^{K - 4},$$

which gives {{10.fitted_p_at_k_4|.2f}} at $K = 4$ and {{10.fitted_p_at_k_14|.2f}} at $K = 14$. The slope is detectable (run-clustered Wald test, $|z|$ =
{{10.slope_in_k_robust_z|u.2f}}) but does not predict unseen runs better than the constant rate.

### Evidence

*How is the rate read?* From top-choice marker skills credited to the character's event: {{10.readings|,.0f}} readings from
{{10.runs|,.0f}} runs, {{10.clean_hits|,.0f}} clean hits. Each of the {{10.ambiguous_credits_unknown_left_out|.0f}} ambiguous credits names another deck card whose events
can give the marker, so, as in section 8, they are left out as unknown; counted as hits, they would raise the rate by
about {{10.rate_with_ambiguous_points_higher|.0f}} points. A reading also misses a firing when the marker was already at hint Lv 5 from other sources
({{10.readings_where_the_event_could_not_show_marker_at_lv_5_from_other_sources|.0f}} readings): where the event could show, the rate is {{10.rate_where_the_event_could_show|.3f}}, or {{10.rate_where_it_could_show_ambiguous_as_hits|.3f}} counting the ambiguous credits as hits.
A firing is missed too when IT takes another choice for a prioritised skill (bottom-choice rates below). The rate is
therefore a lower bound.

*Which form?* None beats the constant rate held out:

| Form | LL (in-sample) | Held out, against the line above it |
|---|---|---|
| constant {{10.constant_p|.3f}} | {{10.ll_constant|u,.1f}} | |
| ${{10.fitted_form_p_at_k_4|.2f}}(1 - {{10.fitted_form_1_ratio_per_event|.3f}})^{K-4}$ | {{10.ll_fitted_form|u,.1f}} | {{10.held_out_fitted_form_against_constant_nats|u+.1f}} ± {{10.held_out_fitted_form_against_constant_se|.1f}} nats ($z$ = {{10.held_out_fitted_form_against_constant_z|u+.2f}}) |
| power law $c(K/8)^{-a}$, $a = {{10.power_law_a|.2f}}$ (uniform competition would be $a = 1$) | {{10.ll_power_law|u,.1f}} | {{10.held_out_power_law_against_fitted_form_nats|u+.1f}} ± {{10.held_out_power_law_against_fitted_form_se|.1f}} against the $K$ form ($z$ = {{10.held_out_power_law_against_fitted_form_z|u+.2f}}) |
| strict competition $1 - (1 - 1/K)^S$, $S = {{10.strict_competition_s|.2f}}$ | {{10.ll_strict_competition|u,.1f}} | {{10.held_out_strict_competition_against_fitted_form_nats|u+.1f}} ± {{10.held_out_strict_competition_against_fitted_form_se|.1f}} against the $K$ form ($z$ = {{10.held_out_strict_competition_against_fitted_form_z|u+.2f}}) |
| slot form, $c / T$ (chain links weigh {{10.every_slot_competing_chain_link_weight|.0f}}), $c = {{10.every_slot_competing_c|.2f}}$ | {{10.ll_every_slot_competing|u,.1f}} | {{10.held_out_every_slot_competing_against_fitted_form_nats|u+.1f}} ± {{10.held_out_every_slot_competing_against_fitted_form_se|.1f}} against the $K$ form ($z$ = {{10.held_out_every_slot_competing_against_fitted_form_z|u+.2f}}) |

Held out (five groups of runs, each scored by the form fitted on the other four), no form predicts unseen runs better
than the constant rate, and strict competition predicts them worse. Within the slot form, chain links take no weight
(weight 1 is rejected, run-clustered score test) and pals compete: a pal weighs $w$ = {{10.every_slot_competing_pal_weight|.2f}} [{{10.every_slot_competing_pal_weight_robust_95_pct_low|.2f}}, {{10.every_slot_competing_pal_weight_robust_95_pct_high|.2f}}] random
events (run-clustered interval), with 0 excluded.

*What else moves the rate?* Little. The observed hit rate by $K$ = 4 / 6 / 8 / 10 / 12 / 14 is {{10.hit_rate_k_4|.2f}} / {{10.hit_rate_k_6|.2f}} / {{10.hit_rate_k_8|.2f}} / {{10.hit_rate_k_10|.2f}} / {{10.hit_rate_k_12|.2f}} / {{10.hit_rate_k_14|.2f}}, and
within a run events fire independently. A logistic slope in $K$ gives {{10.logistic_rate_at_k_8|.1%}} at 8 on offer and {{10.logistic_rate_at_k_14|.1%}} at 14.
Of the card effects, fitted together next to $K$, only the initial friendship gauge is detectable, and it raises the
rate: {{10.initial_friendship_gauge_logit_per_10_points|+.3f}} [{{10.initial_friendship_gauge_95_pct_low|+.3f}}, {{10.initial_friendship_gauge_95_pct_high|+.3f}}] on the logit scale per 10 points (Holm-adjusted run-clustered $p$ = {{10.initial_friendship_gauge_holm_p|.4f}}; on
cards at LB4 alone, {{10.initial_friendship_gauge_cards_at_limit_break_4_logit_per_10_points|+.3f}}). Friendship bonus ({{10.friendship_bonus_logit_per_10_points|u+.3f}}, $p$ = {{10.friendship_bonus_holm_p|.2f}}) and specialty priority
({{10.specialty_priority_logit_per_10_points|+.3f}}, $p$ = {{10.specialty_priority_holm_p|.2f}}) show none. The model leaves these events out, so this changes no prediction; it
is a known miss of the measured rate. Rarity within a character shows no
effect (pooled, SSR markers fire at {{10.pooled_ssr_marker_rate|.1%}} against SR {{10.pooled_sr_marker_rate|.1%}}). Scenario is not detectable (run-clustered $p$ =
{{10.scenario_robust_p|.2f}}), and a rate per scenario does not predict unseen runs better ($z$ = {{10.held_out_scenario_next_to_k_z|u.2f}}).

*Are bottom-choice skills credited?* Rarely. In the {{10.runs_showing_a_priority_list|.0f}} runs showing a priority list, {{10.bottom_choice_markers_prioritised_there|.0f}} of {{10.bottom_choice_candidates_there|.0f}}
candidates were prioritised. Without a priority they were credited in {{10.bottom_choice_not_prioritised_credited_rate|.3f}} [{{10.bottom_choice_not_prioritised_95_pct_low|.3f}}, {{10.bottom_choice_not_prioritised_95_pct_high|.3f}}] of candidates, against
{{10.top_choice_marker_rate|.3f}} for top-choice markers; with a priority, {{10.bottom_choice_prioritised_credited_to_her_event|.0f}} of {{10.bottom_choice_markers_prioritised_there|.0f}}.

*What does leaving them out cost on real careers?* Section 13's rows, each on the run's own deck. This test scores
whether the target was acquired from any source, so it does not depend on reading the event. Of the default targets
only two are a character's top-choice marker (Vodka's Nimble Navigator, Nice Nature's Ramp Up).

- On the {{10.calibration_rows|.0f}} run-target rows where a deck card's random event could hint the target, it was credited to that
  event {{10.careers_acquired_through_that_event|.0f}} times, {{10.careers_acquired_through_that_event_only|.0f}} with no other source. The model gives {{10.careers_rows_the_model_gives_no_chance|.0f}} of these rows no chance, and the
  event hinted the target on {{10.careers_of_them_acquired_through_that_event|.0f}} of them.
- Left out, acquisition there reads {{10.calibration_obs_per_pred_left_out|.2f}} [{{10.calibration_ratio_95_pct_low_left_out|.2f}}, {{10.calibration_ratio_95_pct_high_left_out|.2f}}] of the model, an under-prediction. Credited at the
  fitted rate (about {{10.careers_mean_fitted_rate_there|.2f}} there), it reads {{10.calibration_obs_per_pred_credited|.2f}} [{{10.calibration_ratio_95_pct_low_credited|.2f}}, {{10.calibration_ratio_95_pct_high_credited|.2f}}].
- Held out (the rate fitted on the other four groups' readings), crediting gains {{10.held_out_credited_against_left_out_nats|.1f}} ± {{10.held_out_credited_against_left_out_se|.1f}} nats on
  these rows ($z$ = {{10.held_out_credited_against_left_out_z|.2f}}): positive, but short of the 1.96 that would show it predicts unseen careers better.
- Crediting raises an affected run's odds over the targets it can land by a median {{10.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_median|.0%}} (at most
  {{10.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_max|.0%}}; {{10.odds_change_targets_with_a_positive_chance_runs_with_a_credited_row_runs|.0f}} runs).
- The trainee's own random event can be told apart only for Nice Nature: {{10.careers_trainee_s_own_acquired_through_her_events|.0f}} of {{10.careers_rows_of_the_trainee_s_own_event_identifiable|.0f}} rows.

The model leaves these events out; the data say it under-predicts the two affected targets on decks with those
characters.

## 11. Conditional character events (not modelled)

Computed by `research/s11_conditional.py`.

The model also leaves out the trainee's secret and race-result events. This section counts them and measures what
leaving them out costs.

### Findings

- **{{11.events_with_hints|.0f}} secret and race-result events give hints**, for {{11.umas_with_a_hint_event|.0f}} umas. {{11.events_touching_a_default_target|.0f}} touch the default targets, none with a gold.
- **Where only such an event can give a target, leaving them out shows no significant cost**: acquisition reads
  {{11.calibration_conditional_only_obs_per_pred_left_out|.2f}} [{{11.calibration_conditional_only_ratio_95_pct_low_left_out|.2f}}, {{11.calibration_conditional_only_ratio_95_pct_high_left_out|.2f}}] of the model, and crediting the measured rates gains only {{11.calibration_conditional_only_ll_credited_minus_left_out_positive_rows|.1f}} nats.
- **The exception is Vodka's Nimble Navigator**: her own events give it in {{11.vodka_any_outfit_nimble_navigator_from_her_events|.0f}} of {{11.vodka_any_outfit_nimble_navigator_rows|.0f}} runs, and the model
  under-predicts it.
- **Tokai Teio's Head-On is modelled as always acquired**, like Yamanin Zephyr's Slipstream.

### Terms

- *Conditional character event*: a trainee's secret or race-result event.
- *Conditional-only row*: a run-target row where only such an event can give the target: none of the trainee's story,
  date, outfit or shared random events can, nor the scenario's.

### Evidence

*How many are there?* {{11.events_with_hints|.0f}} events with hints for {{11.umas_with_a_hint_event|.0f}} umas ({{11.events_with_hints_global|.0f}} for {{11.umas_with_a_hint_event_global|.0f}} umas in Global); {{11.events_touching_a_default_target|.0f}} touch the default
targets (Nimble Navigator, Ramp Up, Tail Held High), {{11.of_them_giving_a_gold|.0f}} of them giving a gold.

*What does leaving them out cost?* On the conditional-only rows, her event gave the target on
{{11.careers_conditional_only_from_her_events|.0f}} of {{11.careers_conditional_only_rows|.0f}} rows, {{11.careers_conditional_only_rate|.3f}} [{{11.careers_conditional_only_rate_95_pct_low|.3f}}, {{11.careers_conditional_only_rate_95_pct_high|.3f}}]: Daiwa Scarlet, any outfit, {{11.daiwa_scarlet_any_outfit_nimble_navigator_from_her_events|.0f}} of {{11.daiwa_scarlet_any_outfit_nimble_navigator_rows|.0f}} (Nimble
Navigator); Mejiro Ardan, any outfit, {{11.mejiro_ardan_any_outfit_ramp_up_from_her_events|.0f}} of {{11.mejiro_ardan_any_outfit_ramp_up_rows|.0f}} (Ramp Up). Left out, acquisition there reads
{{11.calibration_conditional_only_obs_per_pred_left_out|.2f}} [{{11.calibration_conditional_only_ratio_95_pct_low_left_out|.2f}}, {{11.calibration_conditional_only_ratio_95_pct_high_left_out|.2f}}] of the model, and crediting the measured rates gains only {{11.calibration_conditional_only_ll_credited_minus_left_out_positive_rows|.1f}} nats.

*And Vodka?* Vodka, any outfit, gets Nimble Navigator from her own events in {{11.vodka_any_outfit_nimble_navigator_from_her_events|.0f}} of {{11.vodka_any_outfit_nimble_navigator_rows|.0f}} runs
[{{11.vodka_any_outfit_nimble_navigator_rate_95_pct_low|.2f}}, {{11.vodka_any_outfit_nimble_navigator_rate_95_pct_high|.2f}}]; her random and conditional events both give it and cannot be told apart. With her rows, the model
left as it is reads {{11.calibration_obs_per_pred_left_out|.2f}} [{{11.calibration_ratio_95_pct_low_left_out|.2f}}, {{11.calibration_ratio_95_pct_high_left_out|.2f}}], and crediting gains {{11.calibration_ll_credited_minus_left_out_positive_rows|.1f}} nats. Not modelled, so the model
under-predicts Vodka's Nimble Navigator.

*Why is Tokai Teio's Head-On certain?* All three of its races are her career goals. The goals ask only for top 5, top
5 and top 3, but IT won all three in {{11.of_them_winning_all_three|.0f}} of {{11.runs_training_tokai_teio|.0f}} runs training her, and every run where her event could
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
  {{13.3_acquired_any_source_prioritised_or_no_list_shown_obs_per_pred|.3f}} [{{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_high|.3f}}] on the rows whose target is prioritised or whose run shows no priority list. On
  all rows they read {{13.3_acquired_any_source_obs_per_pred|.3f}} [{{13.3_acquired_any_source_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_ratio_95_pct_high|.3f}}]; {{13.3_acquired_shortfall_share_on_non_prioritised_rows|.0%}} of the shortfall is on rows the run's list shows not prioritised
  ({{13.3_acquired_not_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_not_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_not_prioritised_ratio_95_pct_high|.3f}}]), where the model's assumption does not hold.
- **Inheritance is calibrated apart from one target**: without Tail Held High, {{3.inheritance_without_tail_held_high_obs_per_pred|.3f}} [{{3.inheritance_without_tail_held_high_ratio_95_pct_low|.3f}}, {{3.inheritance_without_tail_held_high_ratio_95_pct_high|.3f}}].
  Tail Held High, the only target of eight whose interval excludes 1, is a known miss ({{13.tail_held_high_inheritance|.3f}} [{{13.tail_held_high_inheritance_95_pct_low|.3f}}, {{13.tail_held_high_inheritance_95_pct_high|.3f}}];
  section 3).
- **Deck hints read optimistic** ({{13.1_deck_hint_no_inspiration_obs_per_pred|.3f}} of the model), almost all from the priority assumption: event hints land at
  {{13.1_events_only_not_prioritised_obs_per_pred|.2f}} of the model for targets not prioritised. Prioritised targets' event hints read just below it,
  {{13.1_events_only_prioritised_obs_per_pred|.3f}} [{{13.1_events_only_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_prioritised_ratio_95_pct_high|.4f}}], a known miss. Unmodelled scenario and trainee events supply the few
  points the deck loses.
- **Trackblazer's training-hint base over-predicts** these runs ({{13.1_training_pool_only_trackblazer_obs_per_pred|.3f}} of the model); the other scenarios' intervals cover 1.
- **Landing all eight targets reads {{13.p_all_eight_ratio_all|.2f}} of the model**: consistent with calibrated, though only {{13.runs_acquiring_every_open_target|.0f}} runs acquired
  every open target and the interval allows up to about {{13.p_all_eight_optimism_the_interval_allows_1_per_95_pct_low|.1f}}× optimism.
- **Beyond the eight targets**, inheritance reads slightly short of the model ({{13.other_skills_inheritance|.3f}} [{{13.other_skills_inheritance_95_pct_low|.3f}}, {{13.other_skills_inheritance_95_pct_high|.3f}}], spread
  over many skills: a known miss), and training-pool hints are under-predicted ({{13.other_skills_training_pool_hints|.3f}} of the model).

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

Each of {{13.runs_used|,.0f}} real careers (URA Finale {{13.ura_finale_runs|.0f}}, Unity Cup {{13.unity_cup_runs|,.0f}}, Grand Concert {{13.grand_concert_runs|,.0f}},
Trackblazer {{13.trackblazer_runs|.0f}}) is rebuilt as the model sees a career: the trainee in her outfit, the scenario, the six cards at
their own limit breaks, both parents with their recorded parents and every ancestor's white sparks, and race wins
giving the recorded shared-G1 counts. The model then gives, for each default target of that career, the chance of
each step (sections 3-12), and the run record says what actually happened. There are {{13.run_target_rows|,.0f}} rows. Certain rows
({{13.rows_on_her_outfit|,.0f}} on her outfit, {{13.scenario_gold_rows|,.0f}} scenario golds) could only count as successes, so they are left out of tests 1 and 3.

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
| 1. Deck hint, no inspiration | {{13.1_deck_hint_no_inspiration_rows|,.0f}} | {{13.1_deck_hint_no_inspiration_predicted|.3f}} | {{13.1_deck_hint_no_inspiration_observed|.3f}} [{{13.1_deck_hint_observed_95_pct_low|.3f}}, {{13.1_deck_hint_observed_95_pct_high|.3f}}] | {{13.1_deck_hint_no_inspiration_obs_per_pred|.3f}} [{{13.1_deck_hint_no_inspiration_ratio_95_pct_low|.3f}}, {{13.1_deck_hint_no_inspiration_ratio_95_pct_high|.3f}}] |
| 1. Training pool only | {{13.1_training_pool_only_rows|,.0f}} | {{13.1_training_pool_only_predicted|.3f}} | {{13.1_training_pool_only_observed|.3f}} | {{13.1_training_pool_only_obs_per_pred|.3f}} [{{13.1_training_pool_only_ratio_95_pct_low|.3f}}, {{13.1_training_pool_only_ratio_95_pct_high|.3f}}] |
| 1. Events only | {{13.1_events_only_rows|,.0f}} | {{13.1_events_only_predicted|.3f}} | {{13.1_events_only_observed|.3f}} | {{13.1_events_only_obs_per_pred|.3f}} [{{13.1_events_only_ratio_95_pct_low|.3f}}, {{13.1_events_only_ratio_95_pct_high|.3f}}] |
| 1. Events only, target prioritised | {{13.1_events_only_prioritised_rows|.0f}} | {{13.1_events_only_prioritised_predicted|.3f}} | {{13.1_events_only_prioritised_observed|.3f}} | {{13.1_events_only_prioritised_obs_per_pred|.3f}} [{{13.1_events_only_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_prioritised_ratio_95_pct_high|.4f}}] |
| 1. Events only, not prioritised | {{13.1_events_only_not_prioritised_rows|.0f}} | {{13.1_events_only_not_prioritised_predicted|.3f}} | {{13.1_events_only_not_prioritised_observed|.3f}} | {{13.1_events_only_not_prioritised_obs_per_pred|.3f}} [{{13.1_events_only_not_prioritised_ratio_95_pct_low|.3f}}, {{13.1_events_only_not_prioritised_ratio_95_pct_high|.3f}}] |
| 2. Inheritance | {{13.2_inheritance_rows|,.0f}} | {{13.2_inheritance_predicted|.3f}} | {{13.2_inheritance_observed|.3f}} [{{13.2_inheritance_observed_95_pct_low|.3f}}, {{13.2_inheritance_observed_95_pct_high|.3f}}] | {{13.2_inheritance_obs_per_pred|.3f}} [{{13.2_inheritance_ratio_95_pct_low|.3f}}, {{13.2_inheritance_ratio_95_pct_high|.3f}}] |
| 3. Acquired, any source | {{13.3_acquired_any_source_rows|,.0f}} | {{13.3_acquired_any_source_predicted|.3f}} | {{13.3_acquired_any_source_observed|.3f}} [{{13.3_acquired_any_source_observed_95_pct_low|.3f}}, {{13.3_acquired_any_source_observed_95_pct_high|.3f}}] | {{13.3_acquired_any_source_obs_per_pred|.3f}} [{{13.3_acquired_any_source_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_ratio_95_pct_high|.3f}}] |
| 3. Acquired, deck or inspiration only | {{13.3_acquired_deck_or_inspiration_only_rows|,.0f}} | {{13.3_acquired_deck_or_inspiration_only_predicted|.3f}} | {{13.3_acquired_deck_or_inspiration_only_observed|.3f}} | {{13.3_acquired_deck_or_inspiration_only_obs_per_pred|.3f}} [{{13.3_acquired_deck_or_inspiration_only_ratio_95_pct_low|.3f}}, {{13.3_acquired_deck_or_inspiration_only_ratio_95_pct_high|.3f}}] |
| 3. Acquired, any source, prioritised or no list shown | {{13.3_acquired_any_source_prioritised_or_no_list_shown_rows|,.0f}} | {{13.3_acquired_any_source_prioritised_or_no_list_shown_predicted|.3f}} | {{13.3_acquired_any_source_prioritised_or_no_list_shown_observed|.3f}} | {{13.3_acquired_any_source_prioritised_or_no_list_shown_obs_per_pred|.3f}} [{{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_high|.3f}}] |
| 3. Acquired, deck or inspiration only, prioritised or no list shown | {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_rows|,.0f}} | {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_predicted|.3f}} | {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_observed|.3f}} | {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_obs_per_pred|.3f}} [{{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_ratio_95_pct_low|.3f}}, {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_ratio_95_pct_high|.3f}}] |
| 3. Acquired, prioritised | {{13.3_acquired_prioritised_rows|,.0f}} | {{13.3_acquired_prioritised_predicted|.3f}} | {{13.3_acquired_prioritised_observed|.3f}} | {{13.3_acquired_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_prioritised_ratio_95_pct_high|.3f}}] |
| 3. Acquired, not prioritised | {{13.3_acquired_not_prioritised_rows|,.0f}} | {{13.3_acquired_not_prioritised_predicted|.3f}} | {{13.3_acquired_not_prioritised_observed|.3f}} | {{13.3_acquired_not_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_not_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_not_prioritised_ratio_95_pct_high|.3f}}] |
| 4. Landing (sums of $p$) | {{13.4_landing_rows|,.0f}} | {{13.4_landing_sum_of_p|,.1f}} | {{13.4_landing_sum_observed_equivalent|,.1f}} | {{13.4_landing_ratio|.3f}} |

### Evidence

*Does the priority list matter?* Yes. On the runs that show a priority list, prioritised targets are acquired at the
model's rate ({{13.3_acquired_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_prioritised_ratio_95_pct_high|.3f}}]) and the others less often ({{13.3_acquired_not_prioritised_obs_per_pred|.3f}} [{{13.3_acquired_not_prioritised_ratio_95_pct_low|.3f}}, {{13.3_acquired_not_prioritised_ratio_95_pct_high|.3f}}]); the
pooled test averages the two with the runs that show no list. Of the shortfall over all rows,
{{13.3_acquired_shortfall_share_on_non_prioritised_rows|.0%}} is on the non-prioritised rows.

*Where does the deck lose its points?* Almost all in event hints of targets the player did not prioritise (test 1's
split above). Unmodelled scenario and trainee events supply the few points the deck loses: {{13.acquisitions_from_unmodelled_events|.0f}} target acquisitions
in {{13.3_acquired_any_source_rows|,.0f}} rows, {{13.acquisitions_from_unmodelled_events_share_of_rows|.1%}}. On the rows the priority assumption fits, the deck and inspiration alone read
{{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_obs_per_pred|.3f}} [{{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_ratio_95_pct_low|.3f}}, {{13.3_acquired_deck_or_inspiration_only_prioritised_or_no_list_shown_ratio_95_pct_high|.3f}}] of the model, and every source together {{13.3_acquired_any_source_prioritised_or_no_list_shown_obs_per_pred|.3f}} [{{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_low|.3f}}, {{13.3_acquired_any_source_prioritised_or_no_list_shown_ratio_95_pct_high|.3f}}].

*Per target* (observed / predicted):

| Target | Deck hint | Inheritance | Acquired | Landing |
|---|---|---|---|---|
| Uma Stan / Superstan | {{13.uma_stan_deck_hint|.2f}} [{{13.uma_stan_deck_hint_95_pct_low|.2f}}, {{13.uma_stan_deck_hint_95_pct_high|.2f}}] | {{13.uma_stan_inheritance|.3f}} | {{13.uma_stan_acquired|.3f}} | {{13.uma_stan_landing|.3f}} |
| Nimble Navigator / No Stopping Me! | {{13.nimble_navigator_deck_hint|.2f}} [{{13.nimble_navigator_deck_hint_95_pct_low|.2f}}, {{13.nimble_navigator_deck_hint_95_pct_high|.2f}}] | {{13.nimble_navigator_inheritance|.3f}} | {{13.nimble_navigator_acquired|.3f}} | {{13.nimble_navigator_landing|.3f}} |
| Playtime's Over! / See Ya Later! | {{13.playtime_s_over_deck_hint|.3f}} [{{13.playtime_s_over_deck_hint_95_pct_low|.2f}}, {{13.playtime_s_over_deck_hint_95_pct_high|.2f}}] | {{13.playtime_s_over_inheritance|.3f}} | {{13.playtime_s_over_acquired|.3f}} | {{13.playtime_s_over_landing|.3f}} |
| Ramp Up / It's On! | {{13.ramp_up_deck_hint|.3f}} [{{13.ramp_up_deck_hint_95_pct_low|.2f}}, {{13.ramp_up_deck_hint_95_pct_high|.2f}}] | {{13.ramp_up_inheritance|.3f}} | {{13.ramp_up_acquired|.3f}} [{{13.ramp_up_acquired_95_pct_low|.2f}}, {{13.ramp_up_acquired_95_pct_high|.2f}}] | {{13.ramp_up_landing|.3f}} |
| Tail Held High / Tail Nine | {{13.tail_held_high_deck_hint|.3f}} [{{13.tail_held_high_deck_hint_95_pct_low|.2f}}, {{13.tail_held_high_deck_hint_95_pct_high|.2f}}] | {{13.tail_held_high_inheritance|.3f}} [{{13.tail_held_high_inheritance_95_pct_low|.2f}}, {{13.tail_held_high_inheritance_95_pct_high|.2f}}] | {{13.tail_held_high_acquired|.3f}} | {{13.tail_held_high_landing|.3f}} |
| Racing Spirit: Stamina | no deck source | {{13.racing_spirit_stamina_inheritance|.3f}} | {{13.racing_spirit_stamina_acquired|.3f}} | {{13.racing_spirit_stamina_landing|.3f}} |
| Slipstream / Perfect Spot! | {{13.slipstream_deck_hint|.3f}} [{{13.slipstream_deck_hint_95_pct_low|.2f}}, {{13.slipstream_deck_hint_95_pct_high|.2f}}] | {{13.slipstream_inheritance|.3f}} | {{13.slipstream_acquired|.3f}} | {{13.slipstream_landing|.3f}} |
| Risky Business / Nothing Ventured | {{13.risky_business_deck_hint|.3f}} [{{13.risky_business_deck_hint_95_pct_low|.2f}}, {{13.risky_business_deck_hint_95_pct_high|.2f}}] | {{13.risky_business_inheritance|.3f}} | {{13.risky_business_acquired|.3f}} [{{13.risky_business_acquired_95_pct_low|.2f}}, {{13.risky_business_acquired_95_pct_high|.2f}}] | {{13.risky_business_landing|.3f}} |

*Training-pool hints by scenario* (a test of $\beta$ and $d$, section 7): Grand Concert {{13.1_training_pool_only_grand_concert_obs_per_pred|.3f}} [{{13.1_training_pool_only_grand_concert_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_grand_concert_ratio_95_pct_high|.2f}}], Unity Cup
{{13.1_training_pool_only_unity_cup_obs_per_pred|.3f}} [{{13.1_training_pool_only_unity_cup_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_unity_cup_ratio_95_pct_high|.2f}}], URA Finale {{13.1_training_pool_only_ura_finale_obs_per_pred|.3f}} [{{13.1_training_pool_only_ura_finale_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_ura_finale_ratio_95_pct_high|.2f}}], Trackblazer {{13.1_training_pool_only_trackblazer_obs_per_pred|.3f}} [{{13.1_training_pool_only_trackblazer_ratio_95_pct_low|.2f}}, {{13.1_training_pool_only_trackblazer_ratio_95_pct_high|.2f}}]. Only
Trackblazer's interval excludes 1, its $\beta$ over-predicting these runs. By the lowest limit break of the sourcing
cards: LB4 {{13.1_training_pool_only_lowest_lb_4|.3f}}, LB3 {{13.1_training_pool_only_lowest_lb_3|.3f}}, LB0-2 {{13.1_training_pool_only_lb0_2_lowest_ratio|.2f}}-{{13.1_training_pool_only_lb0_2_highest_ratio|.2f}} ({{13.1_training_pool_only_lb0_2_fewest_rows|.0f}}-{{13.1_training_pool_only_lb0_2_most_rows|.0f}} rows each, every interval covering 1).

*Unity Cup's guaranteed golds* show in {{13.unity_cup_s_guaranteed_golds_runs_with_both|.0f}} of {{13.unity_cup_runs|.0f}} Unity Cup runs each.

*Gold acquisition* (observed / predicted): Nothing Ventured {{13.gold_acquisition_nothing_ventured|.3f}} [{{13.gold_acquisition_nothing_ventured_95_pct_low|.2f}}, {{13.gold_acquisition_nothing_ventured_95_pct_high|.2f}}] ({{13.gold_acquisition_nothing_ventured_rows|.0f}} rows), Perfect Spot!
{{13.gold_acquisition_perfect_spot|.3f}} [{{13.gold_acquisition_perfect_spot_95_pct_low|.2f}}, {{13.gold_acquisition_perfect_spot_95_pct_high|.2f}}] ({{13.gold_acquisition_perfect_spot_rows|.0f}}), It's On! outside Unity Cup {{13.gold_acquisition_it_s_on|.3f}} [{{13.gold_acquisition_it_s_on_95_pct_low|.2f}}, {{13.gold_acquisition_it_s_on_95_pct_high|.2f}}] ({{13.gold_acquisition_it_s_on_rows|.0f}}), No Stopping
Me! outside Unity Cup {{13.gold_acquisition_no_stopping_me|.3f}} ({{13.gold_acquisition_no_stopping_me_rows|.0f}}), See Ya Later! {{13.gold_acquisition_see_ya_later|.3f}} ({{13.gold_acquisition_see_ya_later_rows|,.0f}}).

*Reliability.* Rows grouped by the model's predicted chance; if the model is calibrated, each bucket's observed rate
matches its mean prediction. Deck hint and acquired, observed (mean prediction in parentheses):

| Predicted | Deck hint: rows, observed | Acquired: rows, observed |
|---|---|---|
| 0 | {{13.reliability_deck_hint_0_rows|,.0f}}, {{13.reliability_deck_hint_0_observed|.3f}} | {{13.reliability_acquired_0_rows|,.0f}}, {{13.reliability_acquired_0_observed|.3f}} |
| above 0 to 0.20 | {{13.reliability_deck_hint_0_00_0_20_rows|,.0f}}, {{13.reliability_deck_hint_0_00_0_20_observed|.3f}} ({{13.reliability_deck_hint_0_00_0_20_mean_predicted|.2f}}) | {{13.reliability_acquired_0_00_0_20_rows|,.0f}}, {{13.reliability_acquired_0_00_0_20_observed|.3f}} ({{13.reliability_acquired_0_00_0_20_mean_predicted|.2f}}) |
| 0.20-0.40 | {{13.reliability_deck_hint_0_20_0_40_rows|,.0f}}, {{13.reliability_deck_hint_0_20_0_40_observed|.3f}} ({{13.reliability_deck_hint_0_20_0_40_mean_predicted|.2f}}) | {{13.reliability_acquired_0_20_0_40_rows|,.0f}}, {{13.reliability_acquired_0_20_0_40_observed|.3f}} ({{13.reliability_acquired_0_20_0_40_mean_predicted|.2f}}) |
| 0.40-0.60 | {{13.reliability_deck_hint_0_40_0_60_rows|,.0f}}, {{13.reliability_deck_hint_0_40_0_60_observed|.3f}} ({{13.reliability_deck_hint_0_40_0_60_mean_predicted|.2f}}) | {{13.reliability_acquired_0_40_0_60_rows|,.0f}}, {{13.reliability_acquired_0_40_0_60_observed|.3f}} ({{13.reliability_acquired_0_40_0_60_mean_predicted|.2f}}) |
| 0.60-0.80 | {{13.reliability_deck_hint_0_60_0_80_rows|,.0f}}, {{13.reliability_deck_hint_0_60_0_80_observed|.3f}} ({{13.reliability_deck_hint_0_60_0_80_mean_predicted|.2f}}) | {{13.reliability_acquired_0_60_0_80_rows|,.0f}}, {{13.reliability_acquired_0_60_0_80_observed|.3f}} ({{13.reliability_acquired_0_60_0_80_mean_predicted|.2f}}) |
| 0.80-0.95 | {{13.reliability_deck_hint_0_80_0_95_rows|,.0f}}, {{13.reliability_deck_hint_0_80_0_95_observed|.3f}} ({{13.reliability_deck_hint_0_80_0_95_mean_predicted|.2f}}) | {{13.reliability_acquired_0_80_0_95_rows|,.0f}}, {{13.reliability_acquired_0_80_0_95_observed|.3f}} ({{13.reliability_acquired_0_80_0_95_mean_predicted|.2f}}) |
| 0.95-1 | {{13.reliability_deck_hint_0_95_1_00_rows|,.0f}}, {{13.reliability_deck_hint_0_95_1_00_observed|.3f}} ({{13.reliability_deck_hint_0_95_1_00_mean_predicted|.2f}}) | {{13.reliability_acquired_0_95_1_00_rows|,.0f}}, {{13.reliability_acquired_0_95_1_00_observed|.3f}} ({{13.reliability_acquired_0_95_1_00_mean_predicted|.2f}}) |

The deck's over-prediction sits in its top buckets, the event rewards: the buckets predicted 0.6 and above hold
{{13.deck_shortfall_in_buckets_predicted_0_6_and_above|.0%}} of the shortfall, and {{13.their_rows_with_an_event_source|.0%}} of their rows have an event source. The 0.20-0.40 bucket, almost all training-pool
rows, is within its interval ({{13.reliability_deck_hint_0_20_0_40_observed_sentence|.3f}} [{{13.reliability_deck_hint_0_20_0_40_observed_95_pct_low|.3f}}, {{13.reliability_deck_hint_0_20_0_40_observed_95_pct_high|.3f}}] against {{13.reliability_deck_hint_0_20_0_40_mean_predicted_three_places|.3f}}).

*Per run.* A run has {{13.per_run_open_targets|.2f}} open targets on average.

- Targets acquired per run: expected {{13.per_run_acquired_expected|.3f}}, observed {{13.per_run_acquired_observed|.3f}}; on the {{13.per_run_priority_assumption_fits_runs|,.0f}} runs the priority assumption fits
  (every open target prioritised, or no list shown), expected {{13.per_run_priority_assumption_fits_acquired_expected|.3f}}, observed {{13.per_run_priority_assumption_fits_acquired_observed|.3f}}.
- All open targets acquired: product of the model's odds {{13.per_run_all_open_acquired_product_of_odds|.4f}}, observed {{13.per_run_all_open_acquired_observed|.4f}} [{{13.per_run_all_open_acquired_95_pct_low|.4f}}, {{13.per_run_all_open_acquired_95_pct_high|.4f}}]; on
  the runs the priority assumption fits, {{13.per_run_priority_assumption_fits_all_open_acquired_product_of_odds|.4f}} against {{13.per_run_priority_assumption_fits_all_open_acquired_observed|.4f}} [{{13.per_run_priority_assumption_fits_all_open_acquired_95_pct_low|.4f}}, {{13.per_run_priority_assumption_fits_all_open_acquired_95_pct_high|.4f}}].
- Landing $P(\text{all eight})$, recomputed from what each run acquired, against the model's exact $P(\text{all})$:
  {{13.p_all_eight_ratio_all|.3f}} [{{13.p_all_eight_ratio_all_95_pct_low|.2f}}, {{13.p_all_eight_ratio_all_95_pct_high|.2f}}] (bootstrap over runs, resampling whole runs); Unity Cup {{13.p_all_eight_ratio_unity_cup|.3f}} [{{13.p_all_eight_ratio_unity_cup_95_pct_low|.2f}}, {{13.p_all_eight_ratio_unity_cup_95_pct_high|.2f}}],
  Grand Concert {{13.p_all_eight_ratio_grand_concert|.2f}} [{{13.p_all_eight_ratio_grand_concert_95_pct_low|.2f}}, {{13.p_all_eight_ratio_grand_concert_95_pct_high|.2f}}], URA Finale {{13.p_all_eight_ratio_ura_finale|.2f}} [{{13.p_all_eight_ratio_ura_finale_95_pct_low|.2f}}, {{13.p_all_eight_ratio_ura_finale_95_pct_high|.2f}}]; Trackblazer's model value is {{13.trackblazer_s_model_p_all|.0f}}.

Consistent with calibrated, though the interval allows up to about {{13.p_all_eight_optimism_the_interval_allows_1_per_95_pct_low|.1f}}× optimism (1/{{13.p_all_eight_ratio_all_95_pct_low|.2f}}). Only
{{13.runs_acquiring_every_open_target|.0f}} runs acquired every open target, too few to separate the exact joint from the product.

*Single-source cards.* For a single-source pair the deck test scores that card alone. The pairs furthest from the model
(every pair of 40 or more rows whose interval excludes 1, and two whose intervals reach 1, Gold Ship's and Sweep
Tosho's):

| Card > target | Pred | Obs | Ratio | Rows | Likely reason |
|---|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_predicted|.3f}} | {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_observed|.3f}} | {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio|.2f}} [{{13.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio_95_pct_low|.2f}}, {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_ratio_95_pct_high|.2f}}] | {{13.this_might_sting_sasami_anshinzawa_gt_risky_business_rows|.0f}} | the model assumes Risky Business prioritised (finale choice 2) |
| Riko Kashimoto [Planned Perfection] > Ramp Up | {{13.planned_perfection_riko_kashimoto_gt_ramp_up_predicted|.3f}} | {{13.planned_perfection_riko_kashimoto_gt_ramp_up_observed|.3f}} | {{13.planned_perfection_riko_kashimoto_gt_ramp_up_ratio|.2f}} [{{13.planned_perfection_riko_kashimoto_gt_ramp_up_ratio_95_pct_low|.2f}}, {{13.planned_perfection_riko_kashimoto_gt_ramp_up_ratio_95_pct_high|.2f}}] | {{13.planned_perfection_riko_kashimoto_gt_ramp_up_rows|.0f}} | open (her Lone Wolf choice, when prioritised, accounts for part) |
| King Halo [Tonight, We Waltz] > Uma Stan | {{13.tonight_we_waltz_king_halo_gt_uma_stan_predicted|.3f}} | {{13.tonight_we_waltz_king_halo_gt_uma_stan_observed|.3f}} | {{13.tonight_we_waltz_king_halo_gt_uma_stan_ratio|.2f}} [{{13.tonight_we_waltz_king_halo_gt_uma_stan_ratio_95_pct_low|.2f}}, {{13.tonight_we_waltz_king_halo_gt_uma_stan_ratio_95_pct_high|.2f}}] | {{13.tonight_we_waltz_king_halo_gt_uma_stan_rows|.0f}} | Uma Stan sits on a choice that needs a priority |
| Gold Ship [That Time I Became the Strongest] > Uma Stan | {{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_predicted|.3f}} | {{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_observed|.3f}} | {{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_ratio|.2f}} [{{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_ratio_95_pct_low|.2f}}, {{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_ratio_95_pct_high|.2f}}] | {{13.that_time_i_became_the_strongest_gold_ship_gt_uma_stan_rows|.0f}} | open |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_predicted|.3f}} | {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_observed|.3f}} | {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio|.2f}} [{{13.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio_95_pct_low|.2f}}, {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_ratio_95_pct_high|.2f}}] | {{13.cute_cute_mayano_top_gun_gt_nimble_navigator_rows|.0f}} | open |
| Maruzensky [Sentimental Flare ♪] > Slipstream | {{13.sentimental_flare_maruzensky_gt_slipstream_predicted|.3f}} | {{13.sentimental_flare_maruzensky_gt_slipstream_observed|.3f}} | {{13.sentimental_flare_maruzensky_gt_slipstream_ratio|.2f}} [{{13.sentimental_flare_maruzensky_gt_slipstream_ratio_95_pct_low|.2f}}, {{13.sentimental_flare_maruzensky_gt_slipstream_ratio_95_pct_high|.2f}}] | {{13.sentimental_flare_maruzensky_gt_slipstream_rows|.0f}} | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_predicted|.3f}} | {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_observed|.3f}} | {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio|.2f}} [{{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio_95_pct_low|.2f}}, {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_ratio_95_pct_high|.2f}}] | {{13.let_s_get_this_party_lit_daitaku_helios_gt_slipstream_rows|.0f}} | open |
| Daitaku Helios [Sunlit Angel ♪] > Slipstream | {{13.sunlit_angel_daitaku_helios_gt_slipstream_predicted|.3f}} | {{13.sunlit_angel_daitaku_helios_gt_slipstream_observed|.3f}} | {{13.sunlit_angel_daitaku_helios_gt_slipstream_ratio|.2f}} [{{13.sunlit_angel_daitaku_helios_gt_slipstream_ratio_95_pct_low|.2f}}, {{13.sunlit_angel_daitaku_helios_gt_slipstream_ratio_95_pct_high|.2f}}] | {{13.sunlit_angel_daitaku_helios_gt_slipstream_rows|.0f}} | open |
| Sweep Tosho [Lamplit Training of a Witch-to-Be] > Slipstream | {{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_predicted|.3f}} | {{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_observed|.3f}} | {{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_ratio|.2f}} [{{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_ratio_95_pct_low|.2f}}, {{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_ratio_95_pct_high|.3f}}] | {{13.lamplit_training_of_a_witch_to_be_sweep_tosho_gt_slipstream_rows|.0f}} | open |
| Nakayama Festa [Trusting the Dice] > Risky Business | {{13.trusting_the_dice_nakayama_festa_gt_risky_business_predicted|.3f}} | {{13.trusting_the_dice_nakayama_festa_gt_risky_business_observed|.3f}} | {{13.trusting_the_dice_nakayama_festa_gt_risky_business_ratio|.2f}} [{{13.trusting_the_dice_nakayama_festa_gt_risky_business_ratio_95_pct_low|.2f}}, {{13.trusting_the_dice_nakayama_festa_gt_risky_business_ratio_95_pct_high|.2f}}] | {{13.trusting_the_dice_nakayama_festa_gt_risky_business_rows|.0f}} | open |
| Tokai Teio [Step! Smile! Wink!] > Nimble Navigator | {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_predicted|.3f}} | {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_observed|.3f}} | {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_ratio|.2f}} [{{13.step_smile_wink_tokai_teio_gt_nimble_navigator_ratio_95_pct_low|.2f}}, {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_ratio_95_pct_high|.2f}}] | {{13.step_smile_wink_tokai_teio_gt_nimble_navigator_rows|.0f}} | open |

King Halo's Uma Stan follows the priority: where the run lists Uma Stan as prioritised, her card hinted it in
{{13.king_halo_gt_uma_stan_prioritised_hinted|.0f}} of {{13.king_halo_gt_uma_stan_prioritised_rows|.0f}} rows against {{13.king_halo_gt_uma_stan_prioritised_expected|.1f}} expected (exact Poisson-binomial $P(X \le {{13.king_halo_gt_uma_stan_prioritised_hinted|.0f}})$ = {{13.king_halo_gt_uma_stan_prioritised_p_at_most_the_hinted|.3f}},
consistent); on a list without it, {{13.king_halo_gt_uma_stan_on_a_list_not_prioritised_hinted|.0f}} of {{13.king_halo_gt_uma_stan_on_a_list_not_prioritised_rows|.0f}} against {{13.king_halo_gt_uma_stan_on_a_list_not_prioritised_expected|.1f}}.

Below 40 rows, {{13.pairs_under_40_rows_excluding_1|.0f}} more pairs exclude 1. {{13.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources|.0f}} of them are event sources of Playtime's Over! (Yukino Bijin [Dancing Light
into the Night], Yaeno Muteki [Fiery Discipline], Sirius Symboli [Escorte Étoile]) that hinted it in {{13.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources_hinted|.0f}} of {{13.pairs_under_40_rows_excluding_1_playtime_s_over_event_sources_runs|.0f}} runs
against {{13.pairs_under_40_rows_excluding_1_playtime_s_over_lowest_predicted|.2f}}-{{13.pairs_under_40_rows_excluding_1_playtime_s_over_highest_predicted|.2f}} predicted. Their event puts the skill on choice 2, which ends the chain before its last link, so
IT takes it only when the skill is prioritised. Only {{13.playtime_s_over_event_sources_runs_showing_a_priority_list|.0f}} of those runs show a priority list (the skill prioritised in
{{13.playtime_s_over_event_sources_prioritised|.0f}}, hinted in {{13.playtime_s_over_event_sources_hinted_with_a_list_shown|.0f}} of the {{13.playtime_s_over_event_sources_runs_showing_a_priority_list|.0f}}), and the other {{13.playtime_s_over_event_sources_runs_without_a_list|.0f}} hinted it {{13.playtime_s_over_event_sources_hinted_without_a_list|.0f}} time, so the priority cause is
untested here.

*Other skills.* Every non-target skill a deck card or spark could give, on a 30% sample of the runs
({{13.other_skills_runs_sampled|,.0f}}): inheritance reads {{13.other_skills_inheritance|.3f}} [{{13.other_skills_inheritance_95_pct_low|.3f}}, {{13.other_skills_inheritance_95_pct_high|.3f}}] over
{{13.other_skills_inheritance_rows|,.0f}} rows, slightly short of the model, a known miss spread over many skills (the largest one skill's share of
the shortfall is {{13.other_skills_largest_one_skill_s_share_of_the_inheritance_shortfall|.0%}}); training-pool hints read {{13.other_skills_training_pool_hints|.3f}} [{{13.other_skills_training_pool_95_pct_low|.3f}}, {{13.other_skills_training_pool_95_pct_high|.3f}}] over {{13.other_skills_training_pool_rows|,.0f}}, under-predicted.

### Untested here

the step from acquired to landed (spark generation, and that everything hinted is bought); the
priority policy run by run; correlations inside a run beyond $P(\text{all})$; Grand Masters; hint levels. The runs are
completed careers; every comparison is per run on that run's own deck and lineage. Rows with an inspiration are left out
of the deck test, which is then conditional on no spark, treated as independent of the deck.
