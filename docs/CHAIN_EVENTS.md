# Support-card chain events in Independent Training: the model and the evidence

## Verdict

Chain completion in the planner rests on three rules:

1. A run gets a random number N of chain events.
2. Each event advances one non-pal card's chain by one link. The card is picked at random, and a card with more links
   left is picked more often.
3. Pal cards sit outside this process.

How sure each part is:

- **The event count:** measured. It fits final-link completion at every pal count from 1 to 6 chain cards.
- **The allocation rule:** an assumption. It fits the data better than the obvious alternative, but it isn't a known
  game rule.
- **Whether the game spreads events uniformly over the remaining slots:** unknown. Spreading uniformly over the
  remaining chain slots fits the runs better than spreading uniformly over the cards. Neither is confirmed, and no
  mechanism that would explain the preference has been found.

## 1. How many chain events a run gets

N ~ Binomial(21, 0.35):

    P(N = e) = C(21, e) · 0.35^e · 0.65^(21−e)        mean 7.35, variance 4.8

N is the same whatever the deck and whatever the scenario, and pal events don't use any of it. The job field
`chain_events: [n, p]` overrides it.

**Fit.** A card's chain progress is read from which of its link events appear in the run's hint list (section 6). N is
the count that best explains the observed progress across all decks.

- **Distribution-free fit:** mean 7.30, variance 5.1.
- **Poisson(7.1),** the old model, is 27 log-likelihood units worse. Its variance equals its mean, which is too wide.
- **Binomial(15, 0.48)** is 3.5 units worse.
- **Scenario:** fitting the scenarios separately gains 15.7 log-likelihood for 32 extra parameters, which is no real
  difference.
- **Refit after the measurement fixes (section 6):** p stays at 0.35 with n = 21, whether fitted on all links or on
  final links only.

**What the numbers mean.** "21 trials at 35%" is a shape with this mean and spread. The spread being smaller than
Poisson's suggests a capped number of chances per run, for example a fixed set of turns that can each produce a chain
event. That is a reading of the shape, not something verified.

**Why it doesn't depend on the deck.** One N reproduces final-link completion at every pal count:

| Pals | Chain cards | Runs | Final link reached, observed / Binomial(21, 0.35) / old Poisson(7.1) |
|---|---|---|---|
| 5 | 1 | 120 | 0.98 / 0.98 / 0.96 |
| 4 | 2 | 285 | 0.88 / 0.86 / 0.79 |
| 3 | 3 | 391 | 0.58 / 0.53 / 0.48 |
| 2 | 4 | 212 | 0.29 / 0.30 / 0.29 |
| 1 | 5 | 168 | 0.19 / 0.16 / 0.17 |
| 0 | 6 | 123 | 0.14 / 0.15 / 0.15 |

If pal events used up chain events, 4-pal decks would complete less than predicted. They don't.

## 2. How events are shared between cards

**Who has chain slots.**

- **SR and SSR non-pal cards** have as many slots as their chain has links: Global SSRs have 3, SRs 2 (master.mdb
  `single_mode_story_data.show_progress_2`).
- **R cards** have no chain events, so no slots.
- **Pal and group cards** have none (section 3).
- **A deck slot left to a card that gives no targets ("filler")** counts as a 2-link SR, so it still takes events.

**The rule.** Each event goes to card i with probability

    P(i) = wᵢ / Σⱼ wⱼ,     wᵢ = Lᵢ − dᵢ while the card is running, 0 once it has stopped

Lᵢ is the chain length and dᵢ the links already fired. This is the same as picking one of the remaining chain slots
uniformly at random. Links fire in order, so the event always triggers the card's next link.

**When the allocation matters.** If N is at least the total number of slots, every chain completes whatever the rule.
For decks without stop risk:

    P(all chains complete) = P(N ≥ Σ Lᵢ)

For two 3-link cards that is P(N ≥ 6) = 0.80. The rule only decides who gets the events when N runs short. Events
beyond the total are lost.

**The rule against the alternative.** The obvious alternative is uniform over cards: each card's next event equally
likely, whatever its length. It is arguably the more natural mechanism, since the game presumably holds one "next
event" per card. Both rules were scored on the same runs:

| | Uniform over remaining slots (adopted) | Uniform over cards |
|---|---|---|
| Total log-likelihood | −2,397.2 | −2,396.7 |
| Final links, 2 pals (observed 0.27) | 0.29 | 0.36 |
| Final links, 0 pals (observed 0.13) | 0.15 | 0.22 |

Weighting by links left favours fresh cards, so more chains start and fewer finish. The card rule favours chains
already running, so more finish. The data reject the card rule's extra finishes in crowded decks. That is an
empirical preference. Things tested that don't explain it:

- how often the trainee trains with a card (the page's per-card stat contributions);
- the card's type;
- its position in the deck;
- the number of races run.

**Worked examples.** These are exact; "cards" means uniform over cards.

Two 3-link SSRs and exactly 3 events:

| Final state | Slots (model) | Cards |
|---|---|---|
| (2,1) or (1,2) | 45% each | 37.5% each |
| (3,0) or (0,3) | 5% each | 12.5% each |

Under the slot rule (3,0) has probability 3/6 · 2/5 · 1/4: a card that runs ahead has fewer slots left and is picked
less often, so the slot rule evens progress out. Under the card rule it is (1/2)³.

With N ~ Binomial(21, 0.35), P(reach link 1 / 2 / 3) per card:

| Deck | Slots (model) | Cards |
|---|---|---|
| two 3-link SSRs | 1.00 / 0.97 / 0.87 | 0.99 / 0.96 / 0.87 |
| 3-link SSR + 2-link SR | SSR 1.00 / 0.99 / 0.93, SR 0.99 / 0.95 | SSR 1.00 / 0.98 / 0.93, SR 0.99 / 0.96 |
| three 3-link SSRs | 0.98 / 0.84 / 0.54 | 0.96 / 0.81 / 0.58 |
| four 3-link SSRs | 0.93 / 0.65 / 0.25 | 0.88 / 0.62 / 0.33 |

With two chain cards the rule barely matters (N ≥ 6 in 80% of runs, and then every chain completes). As decks crowd,
the slot rule starts more chains and finishes fewer; the card rule does the opposite. Final-link data side with the
slot rule (section 2 table). The rule ignores a card's identity entirely: its bond, how often the trainee trains with
it, its type and specialty, and the timing of events.

## 3. Pal cards

A pal (friend) card's events follow their own track: a first meeting, outings, card-specific events and a finale.

**In the model.**

- **No chain slots:** pals neither take nor receive any of the N chain events.
- **Rewards as GameTora gives them,** entered by hand in `pkg/loopopt/card_events.json` (`always` = certain, used for
  the finales; `chance` = a fixed probability).
- **First-year date events are left out:** they don't fire in IT.
- **Group cards are treated as pals.**

**Evidence.**

- Light Hello [From the Ground Up] credits See Ya Later! to her event in 812 of 812 runs that carry her. "Certain"
  holds.
- Sasami Anshinzawa [This Might Sting!]: her finale is modelled as Nothing Ventured on success and Risky Business on
  failure, 50% each. Nothing Ventured is credited to her event in 23 of 32 runs (72%). This is the first evidence that
  some outcomes aren't equally likely.
  These 23 credits are exact, and no other card in those decks gives Nothing Ventured.
- Her first-year Uma Stan event, left out as bugged: no run credits Uma Stan to her unambiguously. Three runs show
  "King Halo or Sasami Anshinzawa event" (run 10775) or "Agnes Digital or Sasami Anshinzawa event" (runs 19855,
  19947), and the other card in each has its own Uma Stan event. The data are consistent with the exclusion. (An
  earlier draft read these three as her event: a substring match on "Sasami Anshinzawa event" counted the ambiguous
  credits, trap 2 in section 6.)
- Sasami Anshinzawa [Acupuncturist] (R pal): her Risky Business, modelled as certain, in 11 runs:
  - 2 credit it exactly to her event.
  - 4 credit it ambiguously with Nakayama Festa, whose events also give Risky Business.
  - 3 credit it only to inspiration or to Festa's training.
  - 2 credit her with nothing at all.
  - The sample is too small to judge her certainty.

**The consequence.** N is fixed, so the non-pal cards share it. Each pal you add removes a competitor, and the chains
that remain finish more often: final-link completion is 0.86 with two chain cards, 0.53 with three and 0.29 with four.
Pals buy their own certain rewards and better completion for the remaining chains. That is why the planner keeps
choosing 3–4 pal decks with two chain cards.

## 4. Things layered on top

**Stop risk.** If IT's default choice at link l ends the chain on k of its n outcomes, the chain stops there with
q_l = k/n (only choices that end it on some outcomes, `stop_risk` in `card_events.json`). The stopping link σ is
random:

    P(σ = l) = q_l · Π_{l' < l} (1 − q_{l'})

A stopped chain's remaining slots go back into the pool for the other cards. The solver averages over every stopping
combination exactly. A choice that always ends the chain is offered as a separate deck entry instead: "end the chain
here".

**Choices and outcomes.** IT takes an event's first choice unless one of the player's prioritised skills (up to 10)
sits on another choice. The planner assumes the player sets priorities that reach the targets it counts, and names
them with the "prioritise" label. Outcomes of one choice are taken as equally likely and as excluding each other.

**Exactness.** The solver doesn't simulate. It enumerates the exact distribution of final chain states
(`chain_states`). Given a state, sources are independent, so targets that depend on the same run's events are counted
jointly. `val/joint_check.py` recomputes this independently and agrees to about 1e-16.

**Lv 5 (owner's rule).** Events still hint a skill that is already at hint Lv 5. Only training hints are capped: a
skill at Lv 5 leaves the card's training-hint draws.

## 5. Known misses

1. **Two-chain decks:** both chains finish in 82% of runs, against 75% predicted on the decks in the data (some carry
   stop risk). Not explained.
2. **Early links in crowded decks are slightly short.** With the corrected measurement (section 6):

   | Pals | Early-link shortfall | z |
   |---|---|---|
   | 4 | 0.02 ± 0.02 | 0.9 (not significant) |
   | 3 | 0.07 ± 0.02 | 3.3 |
   | 2 | 0.06 ± 0.02 | 2.5 |
   | 1 | 0.05 ± 0.03 | 1.8 |
   | 0 | 0.14 ± 0.03 | 4.3 |

   Final links fit throughout. The shortfall is concentrated in a few cards, observed / predicted:

   | Short | | Fit | |
   |---|---|---|---|
   | Special Week [Take Hold of Our Dreams!] | 0.65 / 0.82 | King Halo [Tonight, We Waltz] (Uma Stan at link 1) | 0.91 / 0.92, n 125 |
   | Yaeno Muteki [Fiery Discipline] | 0.78 / 0.90 | Yukino Bijin [Hometown Cheers] | 0.95 / 0.93 |
   | Zenno Rob Roy ["Rhodonite and the Dreamstone"] | 0.63 / 0.75 | Tamamo Cross [Split the Sky, White Lightning!] | 0.78 / 0.70 |
   | Hishi Akebono [Who Wants the First Bite?] | 0.86 / 0.93 | Seiun Sky [Foolproof Plan] | 0.94 / 0.89 |
   | Smart Falcon [My Umadol Way! ☆] | 0.73 / 0.81 | Gold Ship [That Time I Became the Strongest] | 0.89 / 0.93 |
   | Silence Suzuka [Beyond This Shining Moment] | 0.56 / 0.77 | Symboli Kris S [A Win Foreshadowed] | 0.77 / 0.72 |

   In master.mdb the short cards' chain events look the same as those of the cards that fit, so the difference isn't
   visible in the data tables. Whether it is real or another marker problem is open.
3. **Outcome probabilities** are assumed equal. Anshinzawa's finale is the first evidence against this.
4. **Rejected: a per-card "activation" parameter** (each chain card active in a run with probability a, otherwise no
   events). At 3 chain cards it fitted the uncorrected data (a = 0.82), but it has no mechanism and fails at 4–6 chain
   cards. Most of what it fitted turned out to be measurement error (section 6).

## 6. How chain progress is read from a page, and the traps

**Markers.** A link fired if a "marker" skill of that link's event is credited to "<character> event". A marker must
appear in no other event of the card: no other link and no non-chain event. For the corrected measurement, a link is
only observable if every choice and every outcome of its event carries a marker. Then detection doesn't depend on
which choice IT took.

**The traps** (each made crowded decks look stalled before it was fixed):

1. **Default-choice markers.** Markers taken only from IT's first choice miss events where IT took another choice.
   Among skills credited to chain cards' events, the share from a non-default choice only rises from 2% (4 pals) to
   10% (0–2 pals). Possibly players prioritise more skills when the deck has more chain cards; most pages don't show
   the priority list.
2. **Ambiguous credits.** When two deck cards could have given a skill, the page credits "A or B event", e.g.
   "Sasami Anshinzawa or King Halo event" for Uma Stan. An exact-name match counts these as misses. This alone caused
   most of King Halo's apparent shortfall: 14 of his 43 link-1 misses.
3. **Inspiration-only listings.** A marker skill hinted by inspiration is often listed with "Inspiration" as its only
   source, with no event credit. Such observations are now treated as unknown. 257 card-links fell into this, and
   they are commonly farmed sparks (Uma Stan, Tail Held High).

Other checks:

- **Masking at Lv 5:** a marker already at Lv 5 from inspiration accounts for only 37 of 417 early-link misses.
- **Order contradictions:** an early link missing while a later one shows is rare: 30 of 190 link-1 misses, 4 of 462
  link-2 misses. They cluster in Smart Falcon and King Halo before the ambiguity fix.
- **Unexplained credits:** 8–18% of skills credited to chain cards' events appear nowhere in that card's GameTora
  events. Not investigated: possibly missing data, possibly scenario-specific card events.

**Calibration method.** For each run, P(card reaches link k) under the model is the exact marginal over N and the
allocation, given the run's chain cards and their stop risks. Observed and predicted are compared per link and pal
group. Significance treats each run as one cluster (z = Σ(obs − pred) / √Σ(obs − pred)², summed per run).

## 7. Data

|---|---|---|
| 4-pal | 284 | two-chain completion |
| 3-pal | 300 + 74 | three-chain; the 74 chosen for early-link markers |
| 2-pal | 175 | four chain cards |
| 1-pal | 120 | five chain cards |
| 0-pal | 120 | six chain cards, chosen for early-link markers |
| Yukino Bijin / Yaeno Muteki | 107 | planner-relevant early links |
| Unity Cup | 200 | 3–5 chain cards; also used for the hint-rate fit |

## 8. What would settle the open questions

- **The game's event-selection logic,** from client decompilation (`docs/HANDOFF.md`, research notes). It would replace
  the allocation assumption with the actual rule and explain the per-card shortfalls.
- **Outcome probabilities:** test every multi-outcome event with enough runs, as was done for Anshinzawa's finale.
- **The unexplained credits:** list which skills they are and for which cards. Missing event data would show up here.
