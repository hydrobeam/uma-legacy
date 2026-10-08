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
sparks it expects on a new veteran. The pedigree reads left to right: the trainee, her two parents, each parent's own two
parents, joined by brackets; the six legacies carry gate numbers 1 to 6 and each shows her affinity in the career on
show. Each place is set by hand or left to the planner, which tries every owned uma and every pair of your veterans (at
most one borrowed) and shows its pick outlined in dashes. **Choose parent** opens the parent picker: one of *Your
veterans* (those that can't go there are greyed out, with the reason), *A new one (any uma)*, or *Let the planner
pick*. Picking a new uma, or changing who a grandparent is, asks whether to add the target skills (all at 2★, 3★ or
1★, keep hers, or none; a new parent's own two parents get the same). The trainee shows the affinity her six
legacies hold (parents' and grandparents' parts).

The search shows the career with the best odds first. Then the planner picks the next career (the status line says
"Planning…", then "Planned in X s"; nothing simulated). It values your veterans by the expected number of careers to a
*fully looped parent*: the cheapest plan A → B → C (an 8/8 of A, then of B with A's 8/8 as a parent, then of C from
both), each step repeated until its 8/8, an 8/8 you already have (or borrow) costing nothing. That number assumes
nothing improves along the way, so it leans high; it is shown above *Options*. Every owned uma's best career is
scored by how many careers its veteran saves on that count, on average over every result she can roll (exact), minus the career
itself, plus, for the few highest and the best odds, what her best rolls open for the career after (two careers
deep): the planner runs the highest and repeats it until a roll changes the plan (a roll that changes nobody's odds
leaves every score as it was). Under the odds a line says why the career on show is (or is not) the pick. With the
trainee chosen by hand there is nothing for it to pick, and with every place set by hand the career is just scored.

**The goal's steps** (a note above the career): the goal is a *fully looped parent*, a veteran with a spark of every
target (8/8) whose two parents are 8/8 too, so as a parent she brings three full legacies. A career's odds of landing
everything are exactly its chance of an 8/8 veteran, so with every place left to the planner: with no 8/8 yet the
planner builds the lineage (step 1, the pick above); with one 8/8 the career uses her as a parent, for an 8/8 of another uma
(step 2); with 8/8s of two umas the career uses both (step 3: its 8/8 is a fully looped parent; the pair with the most
stars). Once you have one, the note says so and the planner goes on for more stars.

Under the pedigree come the career's deck (with its **Race schedule** button) and odds, then *Veteran analysis*: her portrait beside her sparks. It starts as the most likely result of the career on show (the most
likely set of targets, at the rank's most likely star count, and the G1s she more likely wins than not). Set it to what
she actually landed (click a spark for its stars, × to remove, + to add; *G1 wins · edit* opens the G1 wins editor
described under *Your veterans*, where the G1s she could enter at a turn are offered, her schedule's race first with its
win odds, and a win in her schedule shows its odds in the turn bar) and press **Compare**: an 8/8 is step 1 of the goal done (with 8/8 parents, a fully looped parent), and the best career
with her as a parent is shown as step 2. Otherwise it gives the planner's expected careers to a fully looped parent once its best next career runs,
without and with her: *She saves N careers* (the two counts, and the next career with her), or *She changes nothing
yet*. A veteran can save careers without being anyone's better parent today: as a builder's parent she becomes a
grandparent of the best career's next lineage, which the planner's scores count. It shows both odds (all targets, all but one), that career as portraits (trainee, then her two parents, *her* marked)
and its deck behind *Show its deck*. *Show its deck*. **Roll a random result** draws one from this career's odds (targets, stars by rank, G1 wins by their
odds) and compares at once; rolling (or pressing Compare) again while a comparison runs cancels it and starts the new one. An edit after Compare marks the comparison out of date. **Back to the most likely result**
resets her; **Add her to your veterans** saves her (with her parents, who become the next career's grandparents) and
hands every place back to the planner for the next generation.

Below: the odds (all targets land as sparks, all but one, targets expected), the deck with the *Race schedule* button
in its title row, the target skills (per target the chance the lineage passes it on, how many of the six carry it, the
chance it lands as a spark) and the aptitude (pink) sparks' chances of being passed on. The
odds and the deck look as on the loop planner page: click a deck card for its panel (owned or not, GameTora); marking
it not owned searches again (with borrowing on, it may come back as the borrowed card). *Race schedule* opens the
planner's schedule window for the trainee: the G1s she runs, as banner cards with their win odds (a race a parent won
counts extra and is outlined as building affinity).

With any place left to the planner, *Options* is a table of what it weighed, under the planner's expected careers
to a fully looped parent and its plan (e.g. "about 1,905 (Agnes Digital → Special Week → Seiun Sky, each until her
8/8)"): per option "This career: 1 in X" and *Saves per run* (the careers its veteran saves on average, minus the run;
hover for her role, the average saving and how often her roll changes the plan), tags *planner's pick* and *best odds
now*, and **Use this**; the career on show is marked. The eight highest are listed (and always the best odds), with
one line for the rest ("41 more umas scored, +7.1 to +46.2 per run"). While the planner works, the six best by odds with
"planning…".

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
