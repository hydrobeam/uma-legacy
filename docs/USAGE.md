# Using the page

The live page is https://uma-legacy.pages.dev. Its own "How this works" page (`/about`) explains the model in plain
words; this file is the reference for the controls.

**Settings** (top left)
- *Game data*: "Global today", or "JP, Grand Masters era" (JP data cut at L'Arc's launch, 24 Aug 2023). Switching
  reloads the page; owned umas, outfits and cards are kept separately for each. Group cards such as Progenitors and
  Guides count as pals in both.
- *Scenario*: Unity Cup (default), Grand Concert, URA Finale, Trackblazer; with the Grand Masters-era data also Grand
  Masters. It sets how often support cards give training hints (the picker shows each as a multiple of URA Finale's
  rate), which golds it hands out whatever the deck (Unity Cup: No Stopping Me! and It's On!), and which cards the
  game bans from the deck (Team Sirius in Unity Cup and Grand Concert). Grand Masters has no measured hint rate and
  uses a placeholder.
- *Lineage rank*: the star mix of the sparks on your legacies, *B to S+* (rank score 6,500–17,499: 50/45/5% for
  1/2/3 stars) or *SS* (17,500+: 20/70/10%), from hakuraku.moe's room-match data. Picking a scenario sets its usual
  rank (SS for Grand Concert and Grand Masters); change it if yours differs.
- *Use R cards* (off by default: their training-hint model is untested), *Borrow one friend's card*, *A separate deck
  for each uma* (on by default).
- The status line: searching, done ("Searched N decks and M loops in T s"), or what keeps the settings from being
  searched, with the button that fixes it; **Stop** ends a long search and keeps the results. **Results to show**
  gives up to three, numbered.

**Target skills.** Type to add a skill; **8/8 example** loads the eight targets used in our runs. A gold the scenario
guarantees shows as that gold, in gold. Nothing else is set by hand: outfits, decks and events are worked out by the
search.

**Umas.** One tile per outfit. Tap to cycle *owned → not owned → must include*, per outfit: a must-include outfit puts
that uma in the loop in that outfit (mark several of her outfits to allow any of them). **None owned**, then tapping
the ones you have, is quickest when you own few. Outfits decide most results through their skill lists (Agnes Digital
[Full-Color Fangirling] carries Superstan, Tamamo Cross and Inari One carry Tail Nine).

**Support deck.** Tap the cards you don't own; only cards that can give a target are listed, each with what it
gives. No uma runs a card of herself or a group card she belongs to.

**Results** update by themselves a second after any change (a change during a search restarts it); with *Search
automatically after a change* off in Settings, a change waits for the Search button. For each loop:
- the odds that every target lands, that all but one land, and how many careers give a 90% chance;
- each uma with her outfit, the targets on her own skill list, and her odds as the trainee (click her name for her
  panel: owned switch, GameTora link);
- the deck, with pals and the borrowed card marked; the (i) button shows the chance of each hint it gives. With a
  deck per uma, the deck most of the loop shares comes first and each other uma's swaps follow;
- **Race schedule** (shared G1s build affinity; optional G1s are free extra race sparks; "A/B" means either race,
  but every runner must pick the same one) and **Affinity** (where each legacy's affinity comes from).

On a wide screen the results stay in view while you edit the settings; on a phone they come first, and a button at
the bottom of the screen takes you back to them.

**Export / import settings** (bottom of the page) copies your settings and the results on screen as text, or loads
someone else's. Settings are saved in your browser on that device; nothing is uploaded.
