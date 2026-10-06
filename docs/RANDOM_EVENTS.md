# Random character events triggered by support cards

## Verdict

A support card's random event (her character's "shared" event) fires in about **15% of careers** with the card in the
deck and someone else training: **21% when the deck offers 4 random events, 13% when it offers 14**, about 5% less
for each extra random event on offer. The strict competition model (a fixed number of random-event draws per career,
each picking uniformly among the events on offer) is rejected. Pals and chain links have no effect of their own once
that count is known. Scenario, limit break, initial friendship, friendship bonus and specialty priority have none
either. SSR cards may fire a little more often than SR and R cards (borderline).

Fitted form: P(a given event fires in a career) = 0.21 × (1 − 0.047)^(K − 4), K = random events on offer (every
shared event of the deck's characters, plus the trainee's own). Log-likelihood −1199.9 against −1205.5 for a
constant 0.148.

## Where the events are

GameTora `training_events_shared`: per character, events that can fire when she is the trainee or when her support
card is in the deck. Usually two per character, one of them without a skill. Of 270 such events: no skill on any
choice 136; skill on the top choice only 53; skill on the bottom choice only 77; skills on both 4. IT takes the top
choice unless a prioritised skill sits on another, so:

- top-choice skills hint whenever the event fires;
- bottom-choice skills hint only when prioritised. In 175 runs that show their prioritised skills, no bottom-choice
  skill with its card in the deck was ever prioritised (Hanshin Racecourse ○ 0/68, Long Shot ○ 0/42, ...), so in
  practice these never hint. Seeking the Pearl's Uma Stan (+3) is one: it needs Uma Stan prioritised;
- the 4 events with skills on both choices give what the priorities pick.

Character events belong to the character: an R, SR or SSR card of hers can trigger them.

## Measurement

| deck | hit rate |
|---|---|
| random events on offer 4 / 6 / 8 / 10 / 12 / 14 | 0.17 / 0.20 / 0.19 / 0.17 / 0.14 / 0.13 |
| pals 0 (chain links ≤11 / 12–14 / 15+) | 0.12 / 0.14 / 0.14 |
| pals 1 (≤11 / 12–14 / 15+) | 0.04 / 0.19 / 0.15 (the 0.04 cell is SR-heavy: SR Mayano Top Gun alone 0/53) |
| pals 2 / 3 / 4 / 5 | 0.17 / 0.19 / 0.20 / 0.17 |
| marker card SSR / SR / R | 0.17 / 0.11 / 0.12 |

Rejected forms (log-likelihood; constant −1205.5): strict competition 1 − (1 − 1/K)^S −1213.4 (S = 1.57); every
event slot competing (chain links + random events + pals) no better than constant, with chain links weighted 0;
power law c·(K/8)^−a −1200.7 with a = 0.39 (uniform competition would be a = 1).

## Same character, different cards

Agnes Tachyon SSR 0.18 (421 readings), SR 0.08 (124); Special Week SSR 0.18, R 0.07; Fine Motion SSR 0.14, R 0.13;
Air Shakur SSR 0.15 (one card), 0/26 (the other), R 0.12. Initial friendship does not explain it. Unknown whether it
is the card, page crediting, or chance in the smaller groups.

## Caveats

- Pages sometimes drop a source when the skill was already hinted another way: rates are slight underestimates.
- Vodka's Nimble Navigator as the trainee (23 of 25 runs) is a trainee event, not this; random events from the
  trainee's own side are not measured here beyond that.
- Only Global scenarios; Trackblazer has 28 readings.

## Rarity, training frequency and the target cards (2,264 runs, 2026-10-05)

- **Specialty priority** (a proxy for being trained with): +0.026 logit per 10 points, [−0.007, +0.065], likelihood
  ratio 1.5. Not adopted.
- **Random events on offer:** −0.043 logit per event [−0.062, −0.021]: 15.7% at 8 on offer, 12.5% at 14, a little
  flatter than the first fit.
- **The cards that hint the default targets:** Vodka (Nimble Navigator) 21/228 = 9.2% across R, SR and SSR (SR 10/104,
  SSR 9/107) against 15.0% predicted for those decks; Nice Nature's SSR (Ramp Up) 16/121 = 13.2% against 15.2%.
  Seeking the Pearl (Uma Stan, on the bottom choice) has no readings.

Impact on the planner, tested by crediting those events at 0.17 (`odds_events`, plus 0.9 for the trainee's own):
odds up by at most 1.7% (owner and all-owned rosters, Unity Cup, Grand Concert, URA Finale), no best loop changes,
and some decks swap one filler card for a Seeking the Pearl card. Not added to the model: the gain is within the
model's other uncertainties, and it would rest on Pearl's unmeasured rate and the assumption that Uma Stan is
prioritised.
