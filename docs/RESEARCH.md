# The model's mathematics

How the planner's odds are built and how each piece was set. Every quantity carries a label: **assumed** (a chosen
rule, or a community figure without published data), **fitted** (estimated from data; sample and method given) or
**derived** (follows from the other parts). Intervals are 95% unless marked; log-likelihood (LL) differences are in
natural-log units ("nats"). "Runs" are Independent Training runs from uma-it, pooled.

The run records are not in this repository; the measured numbers are published here as aggregates. The scripts in
`research/` recompute the measured and derived numbers (`./build.sh research`); a clone without the records recomputes only
what comes from game data and the derived numbers. Community figures and external fits (sections 1-4) and solver
experiments (sections 5, 10, 11) are cited, not recomputed.

The default targets used in the tests are eight skill pairs (white / gold): Uma Stan / Superstan, Nimble Navigator /
No Stopping Me!, Playtime's Over! / See Ya Later!, Ramp Up / It's On!, Tail Held High / Tail Nine, Racing Spirit:
Stamina, Slipstream / Perfect Spot!, Risky Business / Nothing Ventured. Unity Cup guarantees the No Stopping Me! and
It's On! golds.

## 0. What is computed

A career has a trainee $T$, two parents $P_1, P_2$ and the parents' own parents: six legacies $l = 1..6$. For targets
$k = 1..K$ the planner ranks careers, loops and decks by

$$P(\text{all}) = P(\text{every target lands as a spark on the veteran this career produces}).$$

A target lands if the trainee acquires the skill (hinted, so it can be bought), buys it, and it then generates a spark.
Sections 1-4 give the inheritance side, 5 the loop structure, 6-10 the deck and event side, 11 the composition, 12 its
test against real careers.

## 1. Spark generation

$$g_k = g_0 \cdot 1.1^{\,n_k}, \qquad g_0 = 0.2 \text{ (white learned)}, \quad g_0 = 0.4 \text{ (gold learned)}$$

$n_k$ = ancestors (of six) carrying the spark of $k$. A bought gold sparks its white. In a running loop every target
spark sits on all six legacies (**assumed**: a loop keeps veterans that landed everything), so $n_k = 6$,
$1.1^6 = 1.771561$ and $g = 0.3543$ (white) / $0.7086$ (gold).

- $g_0$ and the $\times 1.1$ curve: a community curve, **fitted** externally on about 1.2M white/gold observations
  from Champions Meeting room data, $p \ge 0.84$ per tier. An additive alternative (+2.5 points per carrier) is
  rejected, $p \approx 10^{-14}$. The data cannot separate 0.35 from 0.354 at six carriers.
- That everything acquired is bought: **assumed**, untested (section 12).

## 2. Star mix of legacy sparks

Reproduced by `research/s02_star_mix.py`.

A legacy's white (and unique) spark has $s \in \{1, 2, 3\}$ stars with shares $\pi = (\pi_1, \pi_2, \pi_3)$ set by the
veteran's rank score $R$:

| Rank score | $\pi_1 / \pi_2 / \pi_3$ | Fit |
|---|---|---|
| below 6,500 | 0.90 / 0.10 / 0 | |
| 6,500-17,499 | 0.50 / 0.45 / 0.05 | 528,637 white sparks, $p = 0.13$ |
| 17,500+ | 0.20 / 0.70 / 0.10 | 196,646 white sparks, $p = 0.77$ |

**Fitted** on Champions Meeting room data (goodness-of-fit $p$ shown). The default band per scenario is set by the share of runs reaching 17,500 if
every skill point is spent: URA Finale 0%, Unity Cup 47% (median 17,428), Grand Concert 79% (median 18,094),
Trackblazer 0% (median 12,386; 282 runs). Hence 17,500+ for Grand Concert, below it for URA Finale, Unity Cup and
Trackblazer; Grand Masters has no data and takes 17,500+ (**assumed**).

The loop model does not know a legacy's stars and uses the expected base per roll (**derived** from section 3):

$$\bar b = 0.03\,(\pi_1 + 2\pi_2 + 3\pi_3) = 0.0465 \;(\text{below } 17{,}500), \quad 0.057 \;(17{,}500+).$$

## 3. Inheritance rolls

Reproduced by `research/s12_calibration.py` (the test).

Per legacy $l$, target $k$ and roll:

$$r_{lk} = \min\!\left\{1,\; b(s_{lk})\left(1 + \frac{A_l}{100}\right)\right\}, \qquad
q_k = 1 - \left[\prod_{l=1}^{6} (1 - r_{lk})\right]^{2}$$

$q_k$ = P(at least one inspiration of $k$ in the career); two rolls (Classic and Senior April); $A_l$ is the legacy's
affinity (section 4); $b(s) = 0$ without a spark. Base by spark type and stars ($1/2/3\star$), **assumed** from
community measurements (one of 1,000+ trials) and consistent across every source checked:

| Spark | Skill / scenario white | Race white | Green (unique) | Pink | Blue |
|---|---|---|---|---|---|
| $b$ | 3 / 6 / 9% | 1 / 2 / 3% | 5 / 10 / 15% | 1 / 3 / 5% | 70 / 80 / 90% |

The odds use skill whites only. Grandparents take the full base (**fitted**: with grandparents halved, every predicted
spark chance was about half the measured level). Great-grandparents count for nothing (**assumed**, matching the
game's two-generation lineage). In the loop model $b(s_{lk})$ is replaced by $\bar b$ in each roll. That is exact for
one roll but not for two: a legacy's stars are the same in both rolls, and by convexity
$E[(1 - r)^2] \ge (1 - E[r])^2$, so the loop model slightly overstates $q_k$ (**derived**). The career planner uses
each veteran's real stars and is not affected.

**Test** (2,936 runs, section 12): observed / predicted inspiration over 23,488
run-target rows 0.985 [0.97, 1.00]; by carriers 1-6, 0.957-1.001, every interval covering 1; per roll, spark-years
0.985 and ancestor hits ($2\sum_l r_l$) 0.987; on 37,785 rows of non-target skills 0.993 [0.97, 1.01]. Reliability:

| Predicted $q$ | Rows | Observed (mean predicted) |
|---|---|---|
| 0 | 7,785 | 0.002 |
| above 0 to 0.20 | 2,278 | 0.154 (0.16) |
| 0.20-0.40 | 2,868 | 0.299 (0.30) |
| 0.40-0.60 | 3,585 | 0.487 (0.50) |
| 0.60-0.80 | 4,851 | 0.691 (0.70) |
| 0.80-0.95 | 2,121 | 0.835 (0.84) |

## 4. Affinity

Reproduced by `research/s04_affinity.py` (the self-grandparent fit).

**Relation points** (exact, game data): $\mathrm{pair}(a,b)$ and $\mathrm{tri}(a,b,c)$ sum the relation points of the
relation groups containing all members. A group counts only distinct characters, so $\mathrm{tri}(T, P, T) = 0$.

**Race affinity** (**assumed**, the game rule as community tools implement it; two independent implementations
agree exactly): +3 per single-race G1 both won, counted per race (relocated venues are the same race), multi-race
titles 0. With schedules fixed in advance and two careers independent (**assumed**), expected shared wins are

$$o(a,b) = \sum_r P_a(r)\,P_b(r), \qquad P_a(r) = 1 - \prod_{\text{attempts}} (1 - p),$$
$$p = \mathrm{clamp}\big(110 - c_{\text{dist}} - c_{\text{surf}} - \mathrm{PEN}_j,\ 0,\ 100\big) / 100,$$

$c$ = 0 for S/A, 10 B, 20 C, 30 D, 40 E, 60 F, 90 G; $\mathrm{PEN}_j$ = 0, 0, 5, 20, 30, 50 for the $j$-th race in a
row (any race, goals included; a rest resets). The win table is a community table (**assumed**). Stats and training
turns are not modelled.

**Per-legacy affinity.** For parent $P$ with other parent $P'$ and own parents $\mathrm{par}(P)$, and grandparent
$G \in \mathrm{par}(P)$:

$$A_P = \mathrm{pair}(T,P) + \mathrm{pair}(P_1,P_2) + 3\,o(P,P') + \sum_{G \in \mathrm{par}(P)} \big[\mathrm{tri}(T,P,G) + 3\,o(P,G)\big]$$
$$A_G = \mathrm{tri}(T,P,G) + 3\,o(P,G)$$

In a loop $P'$ is also one of $P$'s own parents, so $o(P, P')$ counts twice in $A_P$. **Fitted** externally to 8,155
Independent Training receipts (fired against predicted within 0.1% for parents, 2% for grandparents). A second
implementation gives identical numbers on one test loop's 12 veterans (lineage total 452 = 184 + 97 + 171), which
shows the implementations agree, not that the formula is the game's.

**Self-grandparent** ($G = T$, section 5). $\mathrm{tri}(T,P,T) = 0$ is **fitted** on 101,536 inspiration trials
attributable to a single ancestor in runs: self-grandparent passes 523 observed,
521.8 predicted with 0, 605.4 with $\mathrm{tri}(T,P,T) = \mathrm{pair}(T,P)$ (about 13 nats worse); the family
$\kappa\cdot\mathrm{pair}(T,P)$ fits $\hat\kappa = 0$, 95% upper limit about 0.35.

## 5. Loop structures, ranking and schedules

Reproduced by `research/s05_loops.py`.

A lineage is $(T, P_1, P_2, X)$: each parent bred from the other parent and $X$. A four-uma loop has 12 lineages (each
member trains from two of the other three); a three-uma rotation has 3, with $X = T$ (the trainee's own earlier
veteran). The rotation occurs in practice: 907 of 3,179 runs have a grandparent of
the trainee's own character (none a parent). A structure $S$ with lineage set $\Lambda_S$ scores

$$\bar P_S = \frac{1}{|\Lambda_S|} \sum_{\lambda \in \Lambda_S} P_\lambda(\text{all}),$$

the long-run rate of a player who trains every member in turn (**assumed**: a player who re-trains only the best
trainee would score a structure by its best lineage).

**Race weights** (**derived**). In one lineage $o(P_1,P_2)$ enters the six affinities 6 times and $o(P_1,X)$,
$o(P_2,X)$ twice each. Summed over the 12 loop lineages every pair gets $6\cdot 2 + 2\cdot 4 = 20$; over the 3
rotation lineages $6 + 2 + 2 = 10$. Both are uniform over pairs, so a structure's schedules maximise
$\sum_{a<b} o(a,b)$.

**Schedules for shared wins, not odds** (**measured**). Re-optimising every solved loop's schedules for $\bar P_S$
directly gained at most +0.12% on any loop and +0.065% on a winner, and changed no top-1 or top-3 loop over 46
validation setups; the ceiling with every pair at its best possible overlap is only 0.56-2.83% above the odds found.

## 6. Training hints

Reproduced by `research/s06_hints.py`.

Each deck card makes $h$ training-hint draws per run, each uniform over its skill pool (size $n$; the stat option is
never drawn) among skills still below hint Lv 5; a skill leaves after $c = \lceil 5 / \iota \rceil$ draws, $\iota$ =
hint levels per draw ($c$ = 5, 3, 2, 2, 1 for +1 ... +5 cards).

$$h = \beta_{\text{scenario}}\,\left(1 + \frac{hf}{100}\right)$$

$hf$ = hint frequency at maximum limit break, unique effect included. For $k$ targets in one card's pool the miss
chance is joint:

$$m(k, n, t) = \left(1 - \frac{k}{n}\right)^{t} \quad (t \le c), \qquad \text{exact Markov chain over the other skills' draw counts otherwise},$$
$$M(k, n, h) = (1 - f)\, m(k, n, \lfloor h \rfloor) + f\, m(k, n, \lfloor h \rfloor + 1), \qquad f = h - \lfloor h \rfloor.$$

The shape (uniform draws, $h$ per run, the cap) is **derived** from the data; $\beta$ is **fitted**. Fit: Bernoulli
likelihood through $M$ per (card-run, pool skill) row, maximum-limit-break card-runs only, rows where no spark hinted
the skill. The form was fixed on 879 runs (19,071 rows, 17,079 without a spark); the
bases were refit with the cap on 2,402 runs (the table's Runs column counts the 2,373 of them that add a row to the fit).

| Scenario | Fitted $\beta$ | Adopted | Rows | Runs |
|---|---|---|---|---|
| Unity Cup | 2.708 | 2.70 | 8,754 | 490 |
| Grand Concert | 2.026 | 2.03 | 25,111 | 1,251 |
| URA Finale | 1.596 | 1.60 | 9,722 | 514 |
| Trackblazer | 1.814 | 1.81 | 3,349 | 118 |
| Grand Masters | no runs | 2.30 (**assumed**: the 879-run pooled base) | | |

The adopted bases are the fitted ones to two decimals, except Unity Cup's, set at 2.70. The refit includes R
card-runs; without them the bases are 2.667 / 2.026 / 1.569 / 1.772 in the table's order.

What fixed the form (879 runs):

| Question | Evidence |
|---|---|
| Rate form, LL on 19,071 rows | $1 + hf/25$ over options −12,813.3; $1 + hf/25$ over skills −12,461.0; constant −12,048.0; $\beta(1 + hf/100)$ −11,910.2; per-scenario $\beta(1 + hf/100)$ −11,817.8; free divisor ($c = 76$) −11,811.9 |
| Skills only, not skills + stat option | in 16 of 19 (card, $hf$) cells the draw-count variance is below the binomial floor a drawn stat option implies |
| Uniform draws with repeats | repeat rate $P(k \ge 2 \mid k \ge 1)$: 0.151 observed, 0.151 two-point count, 0.199 Poisson count; distinct skills 1/2/3/4 per card-run 0.093 / 0.520 / 0.307 / 0.074 observed, 0.084 / 0.476 / 0.359 / 0.079 model |
| Covariates | per-scenario bases +92 nats; card type, rarity, limit break beyond $hf$, pals per deck, initial bond, pool size each under 3.5 (factors 0.91-1.02) |
| Lv-5 cap | beats drawing with replacement by 7.8 nats with no parameter (17.5 on 2,402 runs); obs / pred by draws to Lv 5, replacement vs cap: 5 draws (5,347 rows) 0.968 / 0.979, 3 (259) 0.863 / 0.870, 2 (7,335) 1.025 / 1.024, 1 (94) 1.289 / 0.985 |
| Joint draws over targets | 38,927 pairs of no-spark pool skills on one card-run (Trackblazer left out): $P(\text{both})$ 0.0872 observed, 0.0862 joint, 0.1135 independent; $P(\text{neither})$ 0.4371 vs 0.4354 |
| Pool size | obs / pred 0.88-1.14 for every $n$ with at least 150 rows |
| Per-draw increment | 5,144 of 5,150 single-source levels are multiples of $\iota$, capped at 5 |
| $hf$ by card level | floored linear interpolation −10,676.9, step −10,677.4, max-limit-break $hf$ for all −10,683.8 |

Reading $h$ as a floor/ceiling mixture against a two-point count differs by under 1% ($n = 5$, $h = 2.74$: miss
0.5426 vs 0.5453). A given setup can sit about half a draw off the smooth formula; $hf$ spans 0-60 in the data.

**Spark deficit** (not modelled; the fit conditions on no spark, the case in which a card's hint matters):

| Other source of the skill | Rows | Observed | Predicted | Ratio |
|---|---|---|---|---|
| none | 14,949 | 0.345 | 0.345 | 1.00 |
| inspiration | 1,696 | 0.269 | 0.347 | 0.78 |
| inspiration + card event | 139 | 0.158 | 0.369 | 0.43 |
| card event | 635 | 0.334 | 0.351 | 0.95 |
| other card's training | 572 | 0.372 | 0.376 | 0.99 |

Full drop-out of spark-hinted skills from the draws is rejected (with two pool skills spark-hinted, the rest are hinted
at 0.375 against 0.341 without drop-out and 0.455 with it).

**Not adopted.** Specialty priority as $h \times e^{0.0251}$ per 10 points: +18 nats on 879 runs, Unity's base then
2.50 and the free divisor about 92, but it rests mostly on the newest Unity runs and Unity's $hf$ slope is set by three
cards (obs / pred in Unity 0.89, 0.86, 1.16; in Grand Concert 1.09, 0.96, 1.06). Without it, cards with $hf = 0$ and
specialty 0 are over-rated by about 13% in Unity Cup. Prioritised skills raising training hints: 101 runs show a
priority list, prioritised skills hinted by training at 0.222 against 0.298 predicted (99 rows); no effect assumed.

## 7. Chain events

Reproduced by `research/s07_chains.py`.

A run gets $N$ chain events, the same for every deck and scenario (**fitted**):

$$N \sim \mathrm{Binomial}(21, 0.35), \qquad P(N = e) = \binom{21}{e}\, 0.35^{e}\, 0.65^{21-e}, \quad E N = 7.35,\ \mathrm{Var}\, N = 4.8.$$

Each event advances one non-pal chain card by one link (links fire in order). Card $i$ with chain length $L_i$ (SSR 3,
SR 2; a filler slot counts as a 2-link SR) and $d_i$ links fired is picked with

$$P(i) = \frac{w_i}{\sum_j w_j}, \qquad w_i = L_i - d_i \text{ while running, } 0 \text{ once stopped},$$

i.e. uniform over the remaining chain slots (**assumed**, preferred empirically). Pal and group cards take no slot
(**assumed**, supported below). Without stop risk $P(\text{all chains complete}) = P(N \ge \sum_i L_i)$; two 3-link
cards: $P(N \ge 6) = 0.80$.

**Stop risk** (**derived** from the event data and the choice rule, section 8): if IT's default choice at link $l$ ends
the chain on $k$ of its $n$ outcomes, $q_l = k/n$ and the stopping link $\sigma$ has
$P(\sigma = l) = q_l \prod_{l' < l} (1 - q_{l'})$; a stopped chain's slots return to the pool. The solver enumerates the
exact distribution of final chain states (no simulation); an independent recomputation agrees to about $10^{-16}$.

**Fit of $N$** (1,299 runs, the pal-count table below). Chain progress per card is read from marker skills: a link is
observable only if every choice and outcome of its event carries a skill appearing in no other event of that card. A
credit naming two sources ("A or B event") is unknown when B is another deck card whose events can give the marker, and
so is a marker listed only under inspiration. $N$'s distribution is the one that best explains progress over all decks.

The fit allows a fired link's marker to go unseen with chance 0.03 (**assumed**, a measurement allowance). Without
it, a run in which a card's later link shows while an earlier one does not has likelihood 0. Since links fire in
order, those runs measure the rate: of 581 earlier links implied fired by a later one, 13 show no marker, 0.022
[0.013, 0.038]. The cause is not known. The fit does not depend on it: with allowances from 0.01 to 0.045, or with
none on the 1,288 runs that show no such case, the best $p$ at $n = 21$ is 0.34-0.355.

| Form for $N$ | Result |
|---|---|
| distribution-free | mean 7.36, variance 4.93 |
| Binomial(21, 0.35) | 5.9 nats behind distribution-free with 14 fewer parameters; with $n = 21$, $p = 0.35$ fits best |
| Binomial(15, 0.48) | 14.1 nats behind distribution-free |
| Poisson(7.1) | 30.3 nats behind distribution-free (variance = mean, too wide) |
| per scenario | means Unity Cup 7.56, Grand Concert 7.29, URA Finale 6.79; +19.1 nats for 32 extra parameters |

No binomial with $n$ free beats (21, 0.35) by 0.1 nats. A first fit on
879 runs, with markers read from IT's default choice only, chose the same Binomial(21, 0.35); so did a refit on 1,792
runs with those markers (means 7.52 / 6.84 / 7.20 / 7.92 for 1 / 2 / 3 / 4 pals, variances 4.0-5.3). The sub-Poisson
spread suggests a capped number of chances per run; a reading of the shape, not verified.

One $N$ reproduces final-link completion at every pal count, so pal events do not consume chain events:

| Pals | Chain cards | Runs | Final link reached: observed / Binomial(21, 0.35) / Poisson(7.1) |
|---|---|---|---|
| 5 | 1 | 120 | 0.98 / 0.98 / 0.96 |
| 4 | 2 | 285 | 0.88 / 0.86 / 0.79 |
| 3 | 3 | 391 | 0.56 / 0.53 / 0.48 |
| 2 | 4 | 212 | 0.29 / 0.30 / 0.29 |
| 1 | 5 | 168 | 0.19 / 0.16 / 0.17 |
| 0 | 6 | 123 | 0.14 / 0.15 / 0.15 |

**Allocation rule.** Against uniform over cards (each running card's next event equally likely), on 1,193 runs with
markers read from IT's default choice:

| | Uniform over remaining slots (adopted) | Uniform over cards |
|---|---|---|
| Total LL | −2,397.2 | −2,396.7 |
| Final links, 2 pals (observed 0.27) | 0.29 | 0.36 |
| Final links, 0 pals (observed 0.13) | 0.15 | 0.22 |

The total LL is level (0.5 nats); the slot rule is preferred because the card rule's extra completions in crowded
decks are not seen. Training frequency with the card, card type, deck position and races run do not explain the
preference. Exact consequences with Binomial(21, 0.35), P(reach link 1 / 2 / 3):

| Deck | Slots (adopted) | Cards |
|---|---|---|
| two 3-link SSRs | 1.00 / 0.97 / 0.87 | 0.99 / 0.96 / 0.87 |
| 3-link SSR + 2-link SR | SSR 1.00 / 0.99 / 0.93, SR 0.99 / 0.95 | SSR 1.00 / 0.98 / 0.93, SR 0.99 / 0.96 |
| three 3-link SSRs | 0.98 / 0.84 / 0.54 | 0.96 / 0.81 / 0.58 |
| four 3-link SSRs | 0.93 / 0.65 / 0.25 | 0.88 / 0.62 / 0.33 |

(With two 3-link cards and exactly 3 events, the slot rule gives $(3,0)$ with $\tfrac36 \cdot \tfrac25 \cdot \tfrac14 = 5\%$,
the card rule $(\tfrac12)^3 = 12.5\%$.)

**Known misses.** Two-chain decks (two 3-link SSRs, four pals) finish both chains in 130 of 157 runs, 0.83
[0.76, 0.88], against 0.80 predicted: within the interval. Early links in crowded decks run short while final links
fit; per run as a cluster, $z = \sum(\text{obs} - \text{pred}) / \sqrt{\sum(\text{obs} - \text{pred})^2}$:

| Pals | 4 | 3 | 2 | 1 | 0 |
|---|---|---|---|---|---|
| Early-link shortfall | 0.02 ± 0.02 | 0.07 ± 0.02 | 0.06 ± 0.02 | 0.05 ± 0.03 | 0.15 ± 0.03 |
| $z$ | 1.0 | 3.4 | 2.5 | 1.8 | 4.4 |

More chains never start in crowded decks than predicted: 0.19 against 0.13 with five chain cards, 0.35 against 0.19 with
six. The shortfall concentrates in a few cards (obs / pred): Special Week [Take Hold of Our Dreams!] 0.65 / 0.82, Yaeno
Muteki [Fiery Discipline] 0.78 / 0.90, Zenno Rob Roy ["Rhodonite and the Dreamstone"] 0.63 / 0.75, Hishi Akebono [Who
Wants the First Bite?] 0.86 / 0.94, Smart Falcon [My Umadol Way! ☆] 0.72 / 0.81, Silence Suzuka [Beyond This Shining
Moment] 0.56 / 0.77; their event data look like those of cards that fit (King Halo [Tonight, We Waltz] 0.90 / 0.91, n =
111). A per-card activation parameter (a chain card active with probability $a$) fitted three-card decks ($a = 0.82$) on
data later found to carry marker errors, has no mechanism and fails at 4-6 chain cards: rejected.

## 8. Event choices and outcomes

Reproduced by `research/s08_outcomes.py` (the certainty checks: `research/s12_calibration.py`).

**Choice rule** (**assumed**): IT takes an event's top choice unless a prioritised skill (up to 10, set at career start)
sits on another choice, and the player is assumed to prioritise every target hint the model counts. Where targets sit
on different choices, a deck entry takes one choice (the solver offers one entry per useful choice, and an entry that
ends a chain early).

Evidence on the rule. Sasami Anshinzawa [This Might Sting!]'s finale: with Risky Business prioritised IT took choice 2
in about 95% of runs (exceptions mostly with Risky Business already at hint Lv 5); with neither skill prioritised, 0 of
35 runs took choice 2. Riko Kashimoto [Planned Perfection]: with Lone Wolf prioritised IT took the bottom choice
in 55 of 62 runs. Only 358 of 3,179 runs show a priority list, so the rule is tested
mainly through its consequences (section 12: prioritised targets 1.02, non-prioritised 0.65 on event-only hints).

**Outcomes** (**assumed** unless measured): one choice's outcomes exclude each other, one per firing, equally likely
unless measured (outcome odds are server-side: master.mdb holds none); a card's gold and white of one skill are
combined exactly. Measured shares (uma-it runs, 2026-10-06; 90% intervals; counted only where every outcome could show:
her hint credited, the skill's level from other sources below 5):

| Event, outcome | Observed | Adopted |
|---|---|---|
| Sasami Anshinzawa [This Might Sting!] finale, Nothing Ventured (Risky Business prioritised; 254 runs with the card) | 58/77 = 0.75 [0.66, 0.82] | 0.75 |
| same, choice 2 alone, no inherited Risky Business | 37/45 = 0.82 [0.71, 0.90] | |
| Riko Kashimoto [Planned Perfection] "Unexpected Side", Ramp Up (top choice taken) | 527/585 = 0.90 [0.88, 0.92] | 0.75 |
| Seiun Sky [Foolproof Plan] link 3, Fast-Paced | 33/40 = 0.82 [0.71, 0.90] | 0.75 |
| Smart Falcon [My Umadol Way! ☆] link 3, Center Stage | 37/51 = 0.73 [0.61, 0.81] | 0.75 |

Anshinzawa's finale is counted on runs without Nakayama Festa [Trusting the Dice] (whose "Upping the Ante" also gives
Nothing Ventured +1/+3, credited jointly). Hint levels identify the choice: her Nothing Ventured +3 = choice 1, +1 =
choice 2; a loss on choice 2 shows as her Risky Business at level 2 (storyline choice 3 +1, finale +1). With inherited
Risky Business levels mixed in, choice 2 reads 0.77-0.87. Choice 1 (Nothing Ventured +3 or nothing): 69/127 = 0.54 if
the finale always fires; Grand Concert lower (unexplained). With Risky Business prioritised IT also takes the bottom
choice of her date-start event "A Pleasant Practice Partner?" (Risky Business +1; GameTora marks it 'ee', which outside
IT declines her dates), yet the finale still fires (at least 75 of 77 such runs show its outcome; her extra Risky
Business +1 appears in 53 of 54 choice-2 wins with the priority, 1 of 20 without): IT apparently keeps her dates either
way. The other outcomes: Riko Kashimoto's alternative is Maverick ○, Seiun Sky's Escape Artist, Smart Falcon's Prudent
Positioning.

Off 0.5 but left at the equal rule: Tazuna Hayakawa SSR finale Concentration 0.70, Agnes Tachyon [Q≠0] link 1 choice 2
Murmur 0.70, Marvelous Sunday [Dazzling Day in the Snow] link 3 Position Pilfer 0.58, Light Hello SSR finale +3 See Ya
Later! 1,497 of 2,103 (0.71). Consistent with 0.5: Gold Ship [That Time I Became the Strongest] 0.61, King Halo
[Tonight, We Waltz] 0.57, Daitaku Helios [Sunlit Angel ♪] Tail Held High 0.47 (given link 2), Nakayama Festa
[Trusting the Dice] Nothing Ventured 0.48-0.65 (37-50 readings).

**Pal and group cards.** Their events are entered per card with a probability (finales certain); first-year date
events are left out in IT (**assumed**). Certainty checks: Light Hello [From the Ground Up] credits See Ya Later! to her
event in 852 of 852 runs carrying her; Team Sirius gives Risky Business in 64 of 64.

## 9. Random card events (not modelled)

Reproduced by `research/s09_random_events.py`.

A support card's character's shared random event fires, per event per career with the card in the deck and another
uma training, with (**fitted**)

$$P(\text{fires}) = 0.21\,(1 - 0.047)^{K - 4},$$

$K$ = random events on offer (every shared event of the deck's characters plus the trainee's own): about 0.15 overall,
0.21 at $K = 4$, 0.13 at $K = 14$. Readings: top-choice marker skills credited to the character's event; 2,871
readings from 1,461 runs, 426 clean hits (58 ambiguous two-card credits counted as
misses; with them the rate is about 2 points higher).

| Form | LL |
|---|---|
| constant 0.148 | −1205.5 |
| $0.21(1 - 0.047)^{K-4}$ | −1199.9 |
| power law $c(K/8)^{-a}$, $a = 0.39$ (uniform competition would be $a = 1$) | −1200.7 |
| strict competition $1 - (1 - 1/K)^S$, $S = 1.57$ | −1213.4 |
| every event slot competing, $c / T$ with $T = K + 0.94 \cdot \text{pals}$ (chain links weigh 0), $c = 1.85$ | −1200.1 |

The slot form fits about as well as the adopted one (+5.4 nats over constant for two extra parameters): chain links play
no part, and a pal counts about as one more random event.

Observed hit rate by $K$ = 4 / 6 / 8 / 10 / 12 / 14: 0.17 / 0.20 / 0.19 / 0.17 / 0.14 / 0.13; within a run events fire
independently. On a larger sample (4,748 readings, 2,264 runs) the slope holds (15.7% at 8 on offer, 12.5% at 14) and
rarity, friendship, specialty priority and scenario show no effect distinguishable from zero.

Only top-choice skills hint (bottom-choice ones need a priority; in 175 runs showing a priority list none was ever
prioritised). Crediting these events at 0.17 (0.9 for the trainee's own) raised the odds by at most 1.7% across the
rosters and scenarios tested and changed no best loop, so they stay out.

## 10. Conditional character events (not modelled)

Reproduced by `research/s10_conditional.py` (the counts).

Secret and race-result events: 168 events with hints for 81 umas (110 for 55 umas in Global), 11 touching the default
targets (Nimble Navigator, Ramp Up, Tail Held High), all white hints. Forcing every target-relevant one to fire with certainty, even as
a gold, leaves the best loop unchanged on every setup tested. One exception is modelled as always acquired: Tokai
Teio's Head-On (all three of its races are her career goals), like Yamanin Zephyr's Slipstream.

## 11. The composed career model

Not recomputed in `research/`: the composition is derived; `val/joint_check.py` recomputes the solver's exact odds
independently; the exact / product ratios and the continuation fit come from solver and simulator runs.

For a target $k$ not guaranteed, with $G_k = 0.4\cdot 1.1^{n_k}$, $W_k = 0.2\cdot 1.1^{n_k}$:

$$p_k = d^{g}_k\, G_k + (1 - d^{g}_k)\, W_k \left[1 - (1 - d^{w}_k)(1 - e_k)(1 - q_k)\right]$$

$d^g_k$ = P(the deck hints the gold), $d^w_k$ = P(the deck hints the white), $e_k$ = P(the trainee's own events hint
it), $q_k$ from section 3. A skill on the trainee's outfit replaces the bracket by 1 (white) or gives $p_k = G_k$
(gold); a scenario-guaranteed gold gives $G_k$. Inheritance only ever supplies the white.

Targets are not independent: chain cards share one run's $N$, and one event's outcomes exclude each other. With $c$
the final chain state (and stopping links), sources are independent given $c$ except one card's training draws, which
are joint over its targets (section 6):

$$P(\text{all}) = \sum_{c} P(c) \prod_{k} p_k(c) \qquad (\text{exact, **derived**}).$$

The search prunes with $\prod_k p_k$ times a per-class cap (**assumed** from measurement: 1.05 for decks with at most two
chain-varying targets and no multi-target group, 1.25 otherwise). Exact / product ratios seen: up to 1.18 on Global deck
jobs and 1.17 on Grand Masters (both in the 1.25 class), at most 1.037 with two such targets (`pal_Fk3`), 1.127
with three. No data-free cap exists: a target whose only source is one card's last link makes the ratio
$1/P(\text{link})$.

**Career planner.** The same model with each veteran's real stars, real G1 wins and recorded parents (an unknown
parent brings no relation points; her sparks count); $n_k$ = legacies actually carrying $k$. Since $q_k$ and $e_k$
enter only as $(1 - e_k)(1 - q_k)$, $e_k$ is replaced by $1 - (1 - e_k)(1 - q_k)$ and $1.1^{n_k - 6}$ scales
$P(\text{all})$ (**derived**; checked to 9 digits against the loop model on a uniform-star case).

**Next-career continuation** (**assumed** form, **fitted** parameters). The planner's target is $V$, the expected
careers to a fully looped parent. A trainee's building step: builder careers lift her odds with chance
$\lambda_j = \lambda\,\big((1 - \phi)^j\big)^{1.5}$ at the $j$-th success, each closing a share $\phi$ of the
remaining log-odds gap to $p_\infty = 0.6 \times$ her odds with a full loop lineage; the step costs
$\min_n \big[\sum_{j<n} 1/\lambda_j + 1/p_n\big]$. $\lambda, \phi$ are fitted to the current pool; the shape (0.6,
1.5) to simulated builder traces on four real veteran pools (2, 5, 11, 19 veterans): within 0-5% of achievable
building on 2, 11 and 19, 70% too pessimistic on 5 (builders improve after the first lift; the form only decays).
Leave-one-pool-out fits ranged 0.6-1.0 and 1-3: a calibration on these pools, not a law.

## 12. Calibration against real careers

Reproduced by `research/s12_calibration.py`.

Each of 2,936 runs (URA Finale 598, Unity Cup 780, Grand Concert 1,477, Trackblazer
81) was rebuilt as a career: trainee in her outfit, scenario, the six cards at their own limit breaks (hint frequency
and increment at that limit break), both parents with their recorded parents and every ancestor's white sparks, and
race wins giving the recorded shared-G1 counts. Targets: the default eight; 23,488 run-target rows, of which 1,005 on
the trainee's outfit and 1,560 scenario golds sit outside tests 1 and 3. Intervals: Wilson 95% on the observed rate;
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
| 1. Deck hint, no inspiration | 14,015 | 0.247 | 0.231 [0.224, 0.238] | 0.932 [0.90, 0.96] |
| 1. Training pool only | 2,982 | 0.372 | 0.357 | 0.959 [0.91, 1.01] |
| 1. Events only | 1,864 | 0.887 | 0.807 | 0.910 [0.89, 0.93] |
| 1. Events only, target prioritised | 151 | 0.889 | 0.907 | 1.021 [0.96, 1.06] |
| 1. Events only, not prioritised | 193 | 0.849 | 0.549 | 0.647 [0.56, 0.73] |
| 2. Inheritance | 23,488 | 0.350 | 0.345 [0.339, 0.351] | 0.985 [0.97, 1.00] |
| 3. Acquired, any source | 20,923 | 0.498 | 0.495 [0.488, 0.501] | 0.993 [0.98, 1.01] |
| 3. Acquired, deck or inspiration only | 20,923 | 0.498 | 0.485 | 0.972 [0.96, 0.99] |
| 3. Acquired, prioritised | 1,052 | 0.711 | 0.736 | 1.036 [1.00, 1.07] |
| 3. Acquired, not prioritised | 1,316 | 0.535 | 0.487 | 0.910 [0.86, 0.96] |
| 4. Landing (sums of $p$) | 20,923 | 3,669.8 | 3,631.5 | 0.990 |

(The prioritised / not prioritised rows use the runs that show a priority list.) **Verdict:** the composed acquisition
odds are calibrated (0.993). Inheritance is right. Deck hints read about 7% optimistic, almost all from the priority
assumption: event hints land at 1.02 × predicted for prioritised targets and 0.65 for others. Unmodelled scenario and
trainee events supply the few points the deck loses (208 target acquisitions in 20,923 rows, 1.0%).

Per target (obs / pred):

| Target | Deck hint | Inheritance | Acquired | Landing |
|---|---|---|---|---|
| Uma Stan / Superstan | 0.79 [0.64, 0.96] | 1.005 | 0.996 | 0.997 |
| Nimble Navigator / No Stopping Me! | 0.95 [0.83, 1.08] | 0.977 | 1.057 | 1.035 |
| Playtime's Over! / See Ya Later! | 0.973 [0.94, 1.00] | 0.993 | 0.984 | 0.993 |
| Ramp Up / It's On! | 0.886 [0.83, 0.94] | 1.003 | 0.938 [0.90, 0.97] | 0.924 |
| Tail Held High / Tail Nine | 0.948 [0.88, 1.02] | 0.928 [0.88, 0.98] | 0.959 | 0.962 |
| Racing Spirit: Stamina | no deck source | 0.997 | 1.060 | 1.048 |
| Slipstream / Perfect Spot! | 1.004 [0.92, 1.10] | 0.993 | 1.018 | 0.976 |
| Risky Business / Nothing Ventured | 0.833 [0.75, 0.93] | 0.968 | 0.918 [0.85, 0.98] | 0.917 |

Training-pool hints by scenario (tests $\beta$): Grand Concert 1.000 [0.94, 1.06], Unity Cup 0.935 [0.83, 1.05], URA
Finale 0.893 [0.78, 1.02], Trackblazer 0.747 [0.58, 0.94]. By the lowest limit break of the sourcing cards: LB4 0.960,
LB3 0.953, LB0-2 0.83-1.06 (33-87 rows each, every interval covering 1). Unity Cup's guaranteed golds: 780 of 780 runs
each. Gold acquisition: Nothing Ventured 0.899 [0.82, 0.98] (449 rows), Perfect Spot! 0.833 [0.75, 0.92] (366), It's
On! outside Unity Cup 0.437 [0.28, 0.65] (109), No Stopping Me! outside Unity Cup 0.894 (62), See Ya Later! 1.000
(2,336).

Reliability, deck hint and acquired (observed, mean prediction in brackets):

| Predicted | Deck hint: rows, observed | Acquired: rows, observed |
|---|---|---|
| 0 | 8,247, 0.001 | 4,958, 0.015 |
| above 0 to 0.20 | 81, 0.247 (0.19) | 1,238, 0.164 (0.16) |
| 0.20-0.40 | 2,151, 0.300 (0.32) | 2,343, 0.316 (0.31) |
| 0.40-0.60 | 1,011, 0.465 (0.48) | 2,718, 0.501 (0.50) |
| 0.60-0.80 | 736, 0.632 (0.72) | 3,899, 0.703 (0.71) |
| 0.80-0.95 | 403, 0.722 (0.87) | 2,672, 0.826 (0.86) |
| 0.95-1 | 1,386, 0.957 (1.00) | 3,095, 0.975 (1.00) |

The deck's over-prediction sits in its top buckets, the event rewards.

**Per run** (7.13 open targets each): targets acquired, expected 3.551, observed 3.525; all open targets acquired,
product of the model's odds 0.035, observed 0.031 [0.025, 0.038]. Landing $P(\text{all eight})$ recomputed from what
each run acquired against the solver's exact $P(\text{all})$: 0.847 [0.66, 1.05] (bootstrap over runs; Unity Cup 0.834
[0.61, 1.07], Grand Concert 1.30 [0.78, 1.83], URA Finale 0.81 [0.23, 1.58]; Trackblazer's model value is 0). Consistent
with calibrated, though the interval allows up to about 1.5× optimism (1/0.66). Only 90 runs acquired every open
target, too few to separate the exact joint from the product.

Single-source cards furthest from the model:

| Card > target | Pred | Obs | Ratio | Rows | Likely reason |
|---|---|---|---|---|---|
| Sasami Anshinzawa [This Might Sting!] > Risky Business | 1.000 | 0.730 | 0.73 [0.66, 0.79] | 185 | model assumes Risky Business prioritised (finale choice 2) |
| Riko Kashimoto [Planned Perfection] > Ramp Up | 0.750 | 0.596 | 0.79 [0.73, 0.86] | 359 | open (her Lone Wolf choice when prioritised accounts for part) |
| King Halo [Tonight, We Waltz] > Uma Stan | 0.937 | 0.726 | 0.77 [0.66, 0.87] | 73 | Uma Stan sits on a choice that needs a priority |
| Gold Ship [That Time I Became the Strongest] > Uma Stan | 0.205 | 0.122 | 0.59 [0.32, 1.05] | 74 | open |
| Mayano Top Gun [Cute + Cute = ?] > Nimble Navigator | 0.298 | 0.070 | 0.23 [0.08, 0.62] | 43 | open |
| Maruzensky [Sentimental Flare ♪] > Slipstream | 0.240 | 0.299 | 1.25 [1.01, 1.51] | 224 | open |
| Daitaku Helios [Let's Get This Party Lit!] > Slipstream | 0.339 | 0.491 | 1.45 [1.08, 1.82] | 57 | open |

**Other skills** (a 30% sample of every non-target skill a deck card or spark could give): inheritance 0.993
[0.97, 1.01] over 37,785 rows and training-pool hints 1.051 [1.03, 1.07] over 14,609, so those parts hold beyond the
eight targets.

**Untested here:** the step from acquired to landed (spark generation, and that everything hinted is bought); the
priority policy run by run; correlations inside a run beyond $P(\text{all})$; Grand Masters; hint levels. The runs skew
to good careers; every comparison is per run on that run's own deck and lineage. Rows with an inspiration are left out
of the deck test, which is then conditional on no spark, treated as independent of the deck.

## Summary

| Component | Status | Data | Fit / test |
|---|---|---|---|
| Spark generation $0.2/0.4 \times 1.1^n$ | fitted externally | ~1.2M observations | $p \ge 0.84$ per tier; additive form $p \approx 10^{-14}$ |
| Star mix | fitted externally | 528,637 / 196,646 sparks | $p$ = 0.13 / 0.77 |
| Inheritance $q$ | assumed bases, fitted affinity | 23,488 rows | 0.985 [0.97, 1.00] |
| Self-grandparent trio 0 | fitted | 101,536 trials | 13 nats over the alternative |
| Training hints | form derived, bases fitted | 2,402 runs | training pool 0.959 [0.91, 1.01] |
| Chain events Binomial(21, 0.35) | fitted | 1,299 runs | 5.9 nats from distribution-free; two-chain completion 0.83 vs 0.80 |
| Allocation by slots | assumed, preferred empirically | | final links 0.29 / 0.15 vs observed 0.27 / 0.13 |
| Event choice / priority | assumed | 358 runs with lists | 1.02 prioritised, 0.65 not (event hints) |
| Random card events | fitted, not modelled | 4,748 readings | at most +1.7% on the odds |
| Composed acquisition | derived | 20,923 rows | 0.993 [0.98, 1.01] |
| Per-run $P(\text{all})$ | derived | 2,936 runs | 0.85 [0.66, 1.05] |
