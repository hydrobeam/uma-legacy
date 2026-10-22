# Using the page

The live page is https://uma-legacy.pages.dev. It has three pages: the **Loop planner**, the **Career planner** and
**How this works** (`/about`, the model in plain words; its contents link each section, and a link to an "in detail"
section opens it). Settings are saved in your browser; nothing is uploaded.

## Loop planner

Finds the loop (a three-uma rotation or a four-uma loop) and support decks with the best odds of landing every target
skill as a spark, with each uma's race schedule. Results update by themselves 0.55 s after any change (switch
*Auto-search* off to search on demand).

**Loop setup**
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
- **Support cards**: tap the cards you don't own. Only cards that can give a target, and pals, are listed.
- **Target skills**: type to add. **Presets** save, load, overwrite and rename sets of targets; *8/8 example* holds the
  eight default targets. Presets are shared by both pages and carried by *Export / import settings*.

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
8/8s). It shares the loop planner's game data, scenario, rank, umas, cards and targets, plus *Borrowed parent* and
**Veteran Umamusume**. *Career setup* folds to a one-line summary
(click its ribbon); until you fold or open it yourself, it opens folded once you have veterans.
- *Import umadump file* reads umadump's `trained_chara_data.json` (sparks with stars, single-race G1 wins, both
  parents) or its `rental_chara_data.json`, the veterans the career start screen offers for borrowing (same fields,
  marked borrowed). The dialog's *Yours* and *Borrowed* tabs hold each kind. Your veterans merge by veteran (the
  file's version replaces the same one, nothing is removed); a borrow list replaces the earlier one, since it is who
  lends today; *Remove all* clears the open tab. *Paste* takes the same text, an Export from this page (which replaces
  every import), or an uma.moe lineage planner export.
- With no veterans, **Try sample veterans** adds six random mob umas carrying only the targets that inheritance alone
  gives, the minimum to start a loop.
- Each veteran's sparks, parents and G1 wins can be edited; *Yours / Borrowed* marks a friend's.

**Borrowed parent** (in Career setup, saved with the other settings): you can borrow only a few veterans a day.
*Allowed* (the default) lets a career use one borrowed parent, and the plan counts on borrowing in the careers after it
too. *None left today* (for when today's borrows are used up) picks a career on two of your own veterans and still plans
the later careers with a borrow, since borrows come back each day. *Never* leaves borrowed veterans out, now and in the plan. A borrowed veteran you set
as a parent by hand is then refused with a message (choose one of yours, or allow a borrowed parent), and the parent
picker greys borrowed veterans out.

The pedigree shows the trainee, her two parents and their parents. Each place is set by hand (*Choose trainee*, *Choose parent*) or left
to the planner, which tries every owned uma and every pair of your veterans (at most one borrowed, as *Borrowed parent*
allows). Every edit that
changes the job starts one run (one that leaves it as it is, such as renaming a preset, lets the run under way go on);
the previous answer stays dimmed until the new one arrives, and the status line says what the plan is at (*scoring 12
of 83 umas*).

**Career choices** lists what the planner weighed, under its estimate of the careers you still need: per career its
odds and *Saves per run*, how much it lowers that estimate of the careers to a fully looped parent, averaged over every
result she could roll, minus the run itself: a yardstick for comparing careers, not a forecast of careers saved. *about the same* marks careers within 5% of the best (at least a quarter of a career); among
those the planner takes the better odds. An uma can have two rows: her best odds, and her best career on two 8/8 parents
(*8/8 parents*), where an 8/8 of hers is a fully looped parent; the second can save more with lower odds. A line under
the table says when the search for a shown uma could not prove its answer (it ran out of its work budget, or a deck passed its cap):
her row is the best it found, and a better career may exist. The (i) by the title
(*What these numbers mean*) explains them on the page. **Use this** sets the career's places; the table stays (the last plan's,
while the veterans and settings are unchanged), with your career marked (*your choice* when the plan didn't list it),
and **Back to the planner's pick** returns to the places the plan ran with. The last few answers are kept, so going
back to one draws it again without a run.

**Veteran analysis** shows the career's result as the game's *Umamusume Details* card (Skills, Inspiration, Career
Info). It starts as a random result of the career:
- **Reroll** draws another result (targets, stars, G1 wins, her pink spark) and gets its saving from the plan at once
  (worked out by the solver for that roll); the best career with her as a parent follows from one quick search.
- Set it to what she actually landed and press **Compare** for the exact plan with her added: careers to a fully
  looped parent now and with her, and the best career with her as a parent.
- **Add to veterans** saves her (with her parents) and hands every place back to the planner for the next career.

The card's header holds the career's odds (click for the chance of each count); the card shows her aptitudes at the end
of that result (the legacies' pink sparks included). **Deck** and **Race schedule** sit under the trainee in the pedigree. Below the card, **Target
skills** gives per target the chance her legacies pass it on, how many of the six carry it, and the chance her veteran
gets its spark.

**Export** (in **Veteran Umamusume**) shows your veterans, the places you set and the solver job as text (*Copy*, *Download*), to move them to another
browser or replay a search outside the page.
