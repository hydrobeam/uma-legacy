# Using the page

The live page is https://uma-legacy.pages.dev. Its own "How this works" page (`/about`) explains the model in plain
words; this file is the reference for the controls.

**Settings** (top left)
- *Game data*: "Global today", or "JP, Grand Masters era" (JP data cut at L'Arc's launch, 24 Aug 2023). Switching
  reloads the page; owned umas, outfits and cards are kept separately for each.
- *Scenario*: Unity Cup (default), Grand Concert, URA Finale, Trackblazer; with the Grand Masters-era data also Grand
  Masters. It sets how often support cards give training hints (the picker shows each as a multiple of URA Finale's
  rate), which golds it hands out whatever the deck (Unity Cup: No Stopping Me! and It's On!), and which cards the
  game bans from the deck (Team Sirius in Unity Cup and Grand Concert). Grand Masters has no measured hint rate and
  uses a placeholder.
- *Assumed lineage rank* (the planner doesn't work out the score a deck would reach): the star mix of the sparks on your legacies, *B to S+* (rank score 6,500–17,499: 50/45/5% for
  1/2/3 stars) or *SS* (17,500+: 20/70/10%), from hakuraku.moe's room-match data. Picking a scenario sets its usual
  rank (SS for Grand Concert and Grand Masters); change it if yours differs.
- *Borrow one friend's card*, *A separate deck
  for each uma* (on by default).
- The status line: searching; then "Checking the exact odds…" with the first result already shown (ranked by the
  odds with targets treated as independent); done ("Searched N decks and M loops in T s." and what the result was checked against:
  "nothing else does better" on exact odds, or every deck within 11% / 5% of it, by odds with targets treated as
  independent, when the full check ran out of time); or
  what keeps the settings from being searched, with the button that fixes it; **Stop** ends a long search and keeps the
  results (stopped during the exact check, the first result stays, marked as not checked). **Results to show**
  gives up to three, numbered.

**Target skills.** Type to add a skill; **8/8 example** loads the eight targets used in our runs. A gold the scenario
guarantees shows as that gold, in gold. Nothing else is set by hand: outfits, decks and events are worked out by the
search.

**Umas.** One tile per outfit. Tap to cycle *owned → not owned → must include*, per outfit: a must-include outfit puts
that uma in the loop in that outfit (mark several of her outfits to allow any of them). **None owned**, then tapping
the ones you have, is quickest when you own few. Outfits decide most results through their skill lists (Agnes Digital
[Full-Color Fangirling] carries Superstan, Tamamo Cross and Inari One carry Tail Nine).

**Support deck.** Tap the cards you don't own; only cards that can give a target are listed, each with what it
gives. No uma runs a card of herself or a group card she belongs to. Group cards such as Progenitors and Guides count as pals.

**Results** update by themselves 0.55 s after any change (a change during a search restarts it); with *Search
automatically after a change* off in Settings, a change waits for the Search button. For each loop:
- the odds that every target lands, that all but one land, and how many careers give a 90% chance;
- each uma with her outfit, the targets on her own skill list, and her odds as the trainee (click her name for her
  panel: owned switch, GameTora link);
- the deck, with pals and the borrowed card marked, and "prioritise …" on a card whose event needs that skill set as an IT priority (the search's choice at a chain link, or a pal finale such as Sasami Anshinzawa SSR's); the (i) button shows the chance of each hint it gives. With a
  deck per uma, the deck most of the loop shares comes first and each other uma's swaps follow;
- **Race schedule** (shared G1s build affinity; optional G1s are free extra race sparks; "A/B" means either race,
  but every runner must pick the same one) and **Affinity** (where each legacy's affinity comes from).

On a wide screen the results stay in view while you edit the settings; on a phone they come first, and a button at
the bottom of the screen takes you back to them.

**Export / import settings** (bottom of the page) copies your settings and the results on screen as text, or loads
someone else's. Settings are saved in your browser on that device; nothing is uploaded.

**Career planner** (`/career`, linked from the header). Picks the next career from the veterans you have. The
settings strip (game data, scenario, lineage rank, *Target skills*, *Umas*, *Support cards*, *Borrow one friend's card*)
opens the loop planner's own editors and shares their saved settings; the rank sets the stars of the new veteran's
sparks in the look ahead. The pedigree reads left to right: the trainee, her two parents, each parent's own two
parents, joined by brackets; the six legacies carry gate numbers 1 to 6 and each shows her affinity in the career on
show. Each place is set by hand or left to the planner, which tries every owned uma and every pair of your veterans (at
most one borrowed) and shows its pick outlined in dashes. **Choose parent** opens the parent picker: one of *Your
veterans* (those that can't go there are greyed out, with the reason), *A new one (any uma)*, or *Let the planner
pick*. Picking a new uma, or changing who a grandparent is, asks whether to add the target skills (all at 2★, 3★ or
1★, keep hers, or none; a new parent's own two parents get the same). Under the pedigree: the odds (all targets land as sparks, all but one, targets expected), the deck, the
target skills (per target the chance the lineage passes it on, how many of the six carry it, the chance it lands as a
spark) and the aptitude (pink) sparks' chances of being passed on, the race schedule, and *After this career*. The odds and the deck look as on the loop planner page: click a deck card
for its panel (owned or not, GameTora); marking it not owned searches again (with borrowing on, it may come back as
the borrowed card). *Race schedule* opens the planner's schedule window for the trainee: every G1 she can enter, the
ones she runs as banner cards with their win odds (a race a parent won counts extra and is outlined as building
affinity), the ones she skips listed in their slot with what she does instead. *After this career* gives the chance
she improves your best next career, over every set of targets she could land and every star count (exact for the
next careers checked), what the next career averages when she does, her best case (every target at 3 stars), and why
not when she can't (usually: today's best career trains her own character, and no veteran parents her own character).
Below it, *The veteran you got* starts as the most likely result (the most likely set of targets, at the rank's most
likely star count, and the G1s she more likely wins than not). Set it to what she actually landed (click a spark for
its stars, × to remove, + to add, tick her G1 wins) and press **Compare**: it counts careers until one lands every
target for both choices, running this career again or using her in the next career, the same way for both (one more
career, then the best setup it leaves, repeated), and shows the next career with her and its deck. **Roll a random
result** draws one from this career's odds (targets, stars by rank, G1 wins by their odds) and compares at once. An
edit after Compare marks the comparison out of date. **Back to the most likely
result** resets her; **Add her to your veterans** saves her (with her parents, who become the next career's
grandparents) and hands every place back to the planner for the next generation.
With any place left to the planner, its other picks follow (only what it picked), each with **Use this**.
**Your veterans** (the paddock) below: *Import umadump file* reads umadump's `trained_chara_data.json` (or
*Paste* its text): each veteran's skill and aptitude sparks with stars, single-race G1 wins and both parents; a
re-import replaces earlier imports and keeps veterans made here. In every node: click a skill to cycle its stars
1 → 2 → 3 → 1, hover it for the × that removes it, **+** lists the target skills she lacks and the aptitudes, click a
portrait to change who she is, the G1 button edits her wins, *Yours / Borrowed* marks a borrowed veteran.
