# Using the page

The live page is https://uma-legacy.pages.dev. It has three pages: the **Loop planner**, the **Career planner** and
**How this works** (`/about`, the model in plain words). Settings are saved in your browser; nothing is uploaded.

## Loop planner

Finds the loop (a three-uma rotation or a four-uma loop) and support decks with the best odds of landing every target
skill as a spark, with each uma's race schedule. Results update by themselves 0.55 s after any change (switch
*Auto-search* off to search on demand).

**Settings**
- *Game data*: Global (umas, outfits and cards turn on at their release time, without a new build) or JP, Grand
  Masters (JP data cut at L'Arc's launch, 24 Aug 2023). Owned umas, outfits and cards are kept separately for each.
- *Scenario*: Unity Cup (default), Grand Concert, URA Finale, Trackblazer, and Grand Masters on the JP data. It sets the
  training-hint rate, the golds the scenario hands out (Unity Cup: No Stopping Me! and It's On!) and the cards it bans
  (Team Sirius in Unity Cup and Grand Concert). Grand Masters' hint rate is a placeholder.
- *Assumed lineage rank*: the star mix of your legacies' sparks, *B to S+* (50/45/5% for 1/2/3★) or *SS* (20/70/10%).
  Each scenario sets its usual rank; change it if yours differs.
- *Deck per uma* (on by default): each trainee gets her own best deck. Every deck may borrow one friend's card.
- *Results*: 1 or 3.
- **Umas**: one tile per outfit; tap to mark it owned or not owned.
- **Support cards**: tap the cards you don't own. Only cards that can give a target are listed.
- **Target skills**: type to add. **Presets** save, load, overwrite and rename sets of targets; *8/8 example* holds the
  eight default targets. Presets are shared by both pages and carried by Export.

**Results**, per loop: the odds that every target lands; each uma with her outfit and her odds as the trainee; the
deck, with pals, the borrowed card and "prioritise …" on a card whose event needs that skill set as an IT priority;
with a deck per uma, the *Swaps* each other trainee makes to the shared deck and what they do to the odds. Buttons:
**Odds** (the chance of landing each number of targets), **Race schedule** (all three years per uma; shared G1s build
affinity) and **Affinity** (each career's affinity, legacy by legacy).

The status line shows the search: a first result ranked on independent odds, then "Checking the exact odds…", then
"Checked N decks / M loops", or what keeps the settings from being searched, with the button that fixes it. **Stop**
keeps what was found.

## Career planner

Picks the next career from the veterans you have, aiming at a *fully looped parent* (an 8/8 whose parents are both
8/8s). It shares the loop planner's settings, plus **Veteran Umamusume**:
- *Import umadump file* reads umadump's `trained_chara_data.json` (sparks with stars, single-race G1 wins, both
  parents) or its `rental_chara_data.json`, the veterans the career start screen offers for borrowing (same fields,
  marked borrowed). Each replaces the earlier import of its kind, so both can be loaded; *Paste* takes the same text,
  an Export from this page, or an uma.moe lineage planner export.
- With no veterans, **Try sample veterans** adds six random mob umas carrying only the targets that inheritance alone
  gives, the minimum to start a loop.
- Each veteran's sparks, parents and G1 wins can be edited; *Yours / Borrowed* marks a friend's.

The pedigree shows the trainee, her two parents and their parents. Each place is set by hand (*Choose parent*) or left
to the planner, which tries every owned uma and every pair of your veterans (at most one borrowed). Every edit starts
one run; the previous answer stays dimmed until the new one arrives.

**Career choices** lists what the planner weighed, under its estimate of the careers you still need: per career its
odds and *Saves per run*, how many careers sooner it brings a fully looped parent, averaged over every result she could
roll, minus the run itself. *about the same* marks careers within 5% of the best (at least a quarter of a career); among
those the planner takes the better odds. **What do these mean?** explains the numbers on the page.

**Veteran analysis** shows the career's result as the game's *Umamusume Details* card (skills, inspiration, career
record). It starts as a random result of the career:
- **Reroll** draws another result (targets, stars, G1 wins, her pink spark) and reads its saving at once.
- Set it to what she actually landed and press **Compare** for the exact plan with her added: careers to a fully
  looped parent now and with her, and the best career with her as a parent.
- **Add to veterans** saves her (with her parents) and hands every place back to the planner for the next career.

Below it: the career's odds, its deck and **Race schedule**, each target's inheritance chance, and her aptitudes after
the legacies' pink sparks.

**Export** shows your veterans, the plan and the solver job as text (*Copy*, *Download*), to move them to another
browser or replay a search outside the page.
