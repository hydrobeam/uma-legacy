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
- *Loop*: *3 or 4 umas* (default: 3-uma rotations and 4-uma loops ranked together), *3 umas* or *4 umas* only.
  A 3-uma rotation holds at most three must-include umas.
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
that uma in the loop in that outfit (mark several of her outfits to allow any of them). Up to four umas can be must-include; with four, only four-uma loops qualify. In the results a must-include uma has a green ring round her portrait, and the card says *Must include* at its top right. **None owned**, then tapping
the ones you have, is quickest when you own few. Outfits decide most results through their skill lists (Agnes Digital
[Full-Color Fangirling] carries Superstan, Tamamo Cross and Inari One carry Tail Nine).

**Support deck.** Tap the cards you don't own; only cards that can give a target are listed, each with what it
gives. No uma runs a card of herself or a group card she belongs to. Group cards such as Progenitors and Guides count as pals.

**Results** update by themselves 0.55 s after any change (a change during a search restarts it); with *Search
automatically after a change* off in Settings, a change waits for the Search button. A loop is either four umas (each
trains from two of the other three) or a three-uma rotation (each trains from the other two, who were bred from each
other and from her own earlier veteran); both are ranked on the same odds, the average over the careers when every
member trains in turn, and a rotation usually comes first. For each loop:
- the odds that every target lands, that all but one land, and how many careers give a 90% chance;
- each uma with her outfit, the targets on her own skill list, and her odds as the trainee (click her name for her
  panel: owned switch, GameTora link);
- the deck, with pals and the borrowed card marked, and "prioritise …" on a card whose event needs that skill set as an IT priority (the search's choice at a chain link, or a pal finale such as Sasami Anshinzawa SSR's); the (i) button shows the chance of each hint it gives. With a
  deck per uma, the deck most of the loop shares comes first and each other uma's swaps follow;
- **Race schedule** (a tab per uma, then *All three* or *All four*: shared G1s build affinity; optional G1s are free extra race sparks; "A/B" means either race,
  but every runner must pick the same one) and **Affinity** (where each legacy's affinity comes from; in a rotation the trainee is her own
  grandparent twice, with no relation points).

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
1★, keep hers, or none; a new parent's own two parents get the same). The trainee shows the affinity her six
legacies hold (parents' and grandparents' parts).

When any place is left to the planner, it takes the six best careers by odds and shows the one whose veterans leave the
best next careers (a one-step rollout: per career, the exact chance of each set of targets her veteran could land and
the best next career with that veteran as a parent, today's best as the floor). When that is not the career with the
best odds now, one line under the odds says so: "Picked over <that career> (1 in X now): the veterans it leaves give
better next careers." With every place set by hand, the career is just scored.

Under the pedigree come the career's deck (with its **Race schedule** button) and odds, then *Veteran analysis*: her portrait beside her sparks. It starts as the most likely result of the career on show (the most
likely set of targets, at the rank's most likely star count, and the G1s she more likely wins than not). Set it to what
she actually landed (click a spark for its stars, × to remove, + to add; *G1 wins · edit* opens the G1 wins editor
described under *Your veterans*, where the G1s she could enter at a turn are offered, her schedule's race first with its
win odds, and a win in her schedule shows its odds in the turn bar) and press **Compare**: it counts careers until one
lands every target for both choices, running this career again or using her in the next career, the same way for both
(one more career, then the best setup it leaves, repeated), shows today's best and the next career with her (all
targets, all but one), that next career as portraits (trainee, then her two parents, *her* marked) and its deck behind
*Show its deck*. **Roll a random result** draws one from this career's odds (targets, stars by rank, G1 wins by their
odds) and compares at once; rolling (or pressing Compare) again while a comparison runs cancels it and starts the new one. An edit after Compare marks the comparison out of date. **Back to the most likely result**
resets her; **Add her to your veterans** saves her (with her parents, who become the next career's grandparents) and
hands every place back to the planner for the next generation.

Below: the odds (all targets land as sparks, all but one, targets expected), the deck with the *Race schedule* button
in its title row, the target skills (per target the chance the lineage passes it on, how many of the six carry it, the
chance it lands as a spark) and the aptitude (pink) sparks' chances of being passed on, and *After this career*. The
odds and the deck look as on the loop planner page: click a deck card for its panel (owned or not, GameTora); marking
it not owned searches again (with borrowing on, it may come back as the borrowed card). *Race schedule* opens the
planner's schedule window for the trainee: the G1s she runs, as banner cards with their win odds (a race a parent won
counts extra and is outlined as building affinity). *After this career* lists, as label and value: the chance she improves your best next
career (over every set of targets she could land and every star count; exact for the next careers checked, so a
floor), your best career today, what the next career averages when she improves it, the chance she lands every
target, the chance she improves it by how many targets she lands, and, when she can't, why (usually: today's best
career trains her own character, and no veteran parents her own character).

With any place left to the planner, *Other picks* lists its six candidates (only what it picked), each with its odds
now, the next career her veterans give on average (and how much better than today's best), the best next career with
her landing everything (as portraits), and **Use this**; the pick on show is marked. One line under them gives what
repeating today's best costs (careers on average, exact) and the fewest careers any plan could average (a proven
bound, from the best odds any pool could give).

**Your veterans** (the paddock) below: *Import umadump file* reads umadump's `trained_chara_data.json` (or
*Paste* its text): each veteran's skill and aptitude sparks with stars, single-race G1 wins and both parents; a
re-import replaces earlier imports and keeps veterans made here. *Export* shows your veterans, the plan and the
solver job the planner sends for them (to replay a search outside the page) as text, with *Copy* and *Download*;
*Paste* takes that text back, *Import* the downloaded file. In every node: click a skill to cycle its stars
1 → 2 → 3 → 1, hover it for the × that removes it, **+** lists the target skills she lacks and the aptitudes, click a
portrait to change who she is, the G1 button opens the G1 wins editor, *Yours / Borrowed* marks a borrowed veteran.

The G1 wins editor (a veteran's, and the veteran analysis') is the schedule window's calendar with her wins on it: each
win sits in its slot as a banner card, and a click removes it, leaving the slot empty. An empty slot where a G1 is held
shows a **+** on hover or keyboard focus (faintly at all times on a touch screen); a click opens a menu of the G1s held
at that turn, and picking one marks it won in that slot (Escape or a click elsewhere closes the menu). A race held in
Classic and Senior, such as the Arima Kinen, is one race and one win: it shows once, at the turn it was picked at, else
the turn her schedule runs it, else its first turn (an imported win carries no turn); the menu at its other turn offers
to move it there. The turn is only for display; the planner reads which races she won. The count of wins is in the
header; a veteran's edits are saved at once and the planner searches again when the editor closes.
