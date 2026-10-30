# Using the page

The live page is https://uma-legacy.pages.dev. It has three pages: the **Career planner** (the front page), the **Loop
planner** (`/loop`) and **How this works** (`/about`, the model in plain words; its contents link each section, and a link to an "in detail"
section opens it). Settings are saved in your browser; nothing is uploaded, except that the career planner's uma.moe
search sends your target skills and your uma.moe key to uma.moe.

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
- *Import umadump* (in either) reads umadump's `card_data.json` (your outfits) or `support_card_data.json` (your
  cards), or both at once. A file replaces the marks for the chosen game data: what it lists is owned, the rest not
  owned. Ids the data lacks are skipped; stars, levels and limit breaks are not read.
- **Target skills**: type to add. **Presets** save, load, overwrite and rename sets of targets; *full veteran example* holds the
  eight default targets. Presets are shared by both pages and carried by *Export / import settings*.

**Results**, per loop: the odds that every target lands; each uma with her outfit and her odds as the trainee; the
deck, with pals, the borrowed card and "prioritise …" on a card whose event needs that skill set as an IT priority
("(chain end)" when that skill is on a choice that ends the card's chain early; its panel says when IT may not take it);
with a deck per uma, the *Swaps* each other trainee makes to the shared deck and what they do to the odds. Buttons:
**Odds** (the chance of landing each number of targets), **Race schedule** (all three years per uma; shared G1s build
affinity) and **Affinity** (each career's affinity, legacy by legacy).

The status line shows the search: a first result ranked on independent odds, then "Checking the exact odds…", then
"Checked N decks / M loops", or what keeps the settings from being searched, with the button that fixes it. **Stop**
keeps what was found.

## Career planner

Picks the next career from the veterans you have, aiming at a *fully looped parent* (a full veteran whose parents are both
full veterans). It shares the loop planner's game data, scenario, rank, umas, cards and targets, plus *Borrowed parent* and
**Veteran Umamusume**. *Career setup* folds to a one-line summary
(click its ribbon); until you fold or open it yourself, it folds once you have veterans. On phones the next career
comes first and the setup under it.
- *Import umadump file* reads umadump's `trained_chara_data.json` (sparks with stars, single-race G1 wins, both
  parents) or its `rental_chara_data.json`, the veterans the career start screen offers for borrowing (same fields,
  marked borrowed). The dialog's tabs are the game's: *Veteran Umamusume* (yours) and *Guests* (the ones you can
  borrow: your borrow list, then uma.moe's listings, each under its heading), drawn as the game's legacy list draws
  veterans (portrait in its ring, rank, three main sparks, her number or her trainer; a *Parent 1* / *Parent 2* tab on
  the next career's parents), ordered by targets carried (the most first), as they were added to the page (uma.moe's in its own order) or by rank (the
  sort menu). The chosen one shows beside
  the list (on phones, tapping one opens her popup): her sparks, how many targets she carries as *x/y*, *Yours* /
  *Guest*, *Remove*. Your veterans merge by veteran (the
  file's version replaces the same one, nothing is removed); a borrow list replaces the earlier one, since it is who
  lends today; *Remove all* clears the open tab. *Paste* takes the same text, an Export from this page (which replaces
  every import), or an uma.moe lineage planner export.
- **Borrow from uma.moe** (beside *Next career*, once you have a veteran of your own; a popup listing what was found as the game's legacy list draws veterans:
  her portrait in its ring, her rank, her three main sparks, her trainer; ordered (the sort menu, as the umas' and cards')
  as uma.moe orders them, or by rank, affinity, wins, white sparks' stars or the latest updated; picking the chosen order
  again reverses it; in her popup her target sparks come first, the rest in the game's grey; a card opens her veteran popup, with her trainer's followers
  and *Copy ID*; *Include max borrows* asks again without the follower limit): paste your uma.moe API key once (made at uma.moe/settings; kept in
  your browser under its own entry, sent only to uma.moe as a header, never in an Export) and the veterans players list
  on uma.moe join the pool as borrowed parents, which the planner weighs like any other borrow. The page asks once per
  target set (on saving the key, and after the targets change) for the best covered 100 lineages whose parent carries
  all but at most one target herself, from followable trainers (under 1,000 followers); the answer is kept 24 hours,
  and *Refresh* asks again. Edits to veterans or settings never send a request. A parent from uma.moe shows her trainer;
  her popup has *Copy ID*, to follow them in the game. uma.moe's veterans are not in *Veteran
  Umamusume* or an Export; the parent picker lists them under *uma.moe*, and one you choose stays while chosen.
  *Disconnect* forgets the key. Global data only.
- With no veterans, **Import veterans** opens Veteran Umamusume and **Try sample veterans** adds six random mob umas
  carrying only the targets that inheritance alone gives, the minimum to start a loop; importing your own veterans
  retires the samples. The Umas and Support cards buttons show how many you own (all, until you say otherwise).
- Each veteran's sparks, parents and G1 wins can be edited; *Yours / Guest* marks a friend's.
- Enter a veteran's pink (aptitude) sparks too: imports carry them, a veteran added by hand starts without. The
  trainee's race chances use her grades raised by her legacies' pinks, with an extra cost at the lowest aptitudes that
  was fitted on raised grades; without the pinks her grades read low, and her chances at those races low with them.

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

**Other careers** lists what the planner weighed, under its estimate of the careers you still need: per career its
odds and *Extra careers*, how many more careers than the pick it costs on that estimate (each career's saving, averaged
over every result she could roll, minus the run itself, against the pick's): a yardstick for comparing careers, not a
forecast. The pick is the career with the most saving; better odds don't break a near-tie, since the saving already
counts the runs where she lands every target. A career within 0.25 careers of the pick (the planner's per-decision
tolerance) shows *≈0*: as good, so your own preference decides. Where no career you can run saves more than its own run,
a line above the table says so, and under *None left today* whether the estimate's next step needs a borrowed parent.
Each number's tooltip says what her run does: one of the estimate's two parents or its last run, else the side her
veteran's routes lift most often and the chance per run. A career taken with **Use this** holds for the setup it
was taken on: changing a setting (targets, cards, Borrowed parent, ...) hands its places back to the planner; places
chosen by hand stay. The table's notes (no career saves its own run, a check that did not finish, a search it could not
prove) are in its (i) popup, not under the table.
Once
the pick is shown, a check scores the shown careers again under the model
without its kept step caps, six tested variants of it (its building curve at the corners of its fitted range, every
all-target chance scaled to either end of its calibration interval) and up to three per-uma scenarios (the top-scoring
uma's all-target chance scaled down and one rival uma's up, by a split inferred from the measured spread between umas;
the numbers are on *How this works*); where they score different careers highest, *pick varies* marks each one (all are
in the table) and a line under the table names them: the pick varies under those variants, a sensitivity of the
choice, not a tie. The pick is still the rule's under the model as fitted, and the check covers only those rows, variants and scenarios. Then, for the
pick's uma, each uma a scenario scores highest and the four best-scoring umas, it scores her every pair of parents with
the best deck found so far for each, and the four best pairs again with every deck found for her and after a deck
search of each; the best pairs found are then tried for every uma not searched on all her pairs (those a fixed amount
of work left out included). Where a pair beats the pick's (or a scenario's uma's)
shown parents (the pick itself, for the pick's uma), a line under the table names it, its gain in *Saves per run* over her shown parents and over the best
other pair checked, and how many other pairs it was checked against; where the best uma and pair found is another
uma's, a line names it against the pick with how many umas each step covered: a better career, and the recommendation,
where it saves more as fitted, with the per-uma scenarios that would reverse that named as a sensitivity; else none
beats the pick (scored with the decks found so far, so not certified best). It runs behind the answer;
where it cannot finish (the solver was stopped or restarted before it was done) a line under the table says so, as one
does where the planner's memory budget left out a route or partner it could not weigh (its Saves per run may then be
too low). An uma can have two rows: her best odds, and her best career on two full veteran parents
(*n/n parents*, with n the number of targets), where a full veteran of hers is a fully looped parent; the second can save more with lower odds. A line under
the table says when the search for a shown uma could not prove its answer, with the causes the plan's searches
reported (a search ran out of its work budget, a deck band was too large to check in full, or a deck beat its class's
assumed cap, in this run or an earlier search for these umas): her row is the best it found, and a better career may
exist. The status line carries the same caveats for the search of the best odds, and one when a scored deck beat its
class's assumed cap (the caps are assumptions the check rests on) even where that search finished. It also names the
assumptions a proof rests on (the assumed class cap for two-feeder decks, the assumed bound for nested-chain decks
beside other feeders), the plan's searches included: a proof under them is still a proof, and the table adds no line
for them. The (i) by the title
(*What these numbers mean*) explains them on the page. **Use this** sets the career's places (a builder's other career, listed only where the plan's `builder_careers` is above 1, which the page leaves at its default 1, also keeps its deck until the places change); the table stays (the last plan's,
while the veterans and settings are unchanged), with your career marked (*your choice* when the plan didn't list it),
and **Back to the planner's pick** returns to the places the plan ran with. The last few answers are kept, so going
back to one draws it again without a run.

**The next career**: beside the title, its odds of landing every target (click for the chance of each count) and **Try
a run**. On wide screens the trainee sits over her two **Legacies** on the left (each parent with her veteran number and
her own two parents as faces; on phones only the parent, tap her for the rest; a parent borrowed from uma.moe shows her
trainer, and her popup has *Copy ID*, to follow them before the career starts), with **Deck** (a popup: the six cards with
their priority and chain notes) and **Race schedule** (the game's calendar of the G1s she runs, each race's win chance from the community table on her own
grades, the planner's own figure, which counts the pink sparks she may get, on hover; *Win table* shows the table) under
the trainee; the
**Targets** take the right side, two to a row, lowest first: each the game's skill tile with her chance to land its spark,
and under it her chance to get the skill on the game's bond gauge (gold: its gold skill; white: the white alone), the chance her legacies pass it on (three uma figures) and her deck's (the cards that hint it or give it by an
event, her outfit's portrait or the scenario's logo); on phones one to a row. Tapping a skill opens *Skill Details*: where it comes from with each chance, her six legacies'
faces (lit where one carries it) and each card that gives it.
The landing chances
multiply to the odds of all of them. The status line shows only while the planner works or when something needs saying. A reload with nothing changed draws
the last answer at once, without planning again.

**Try a run** opens a popup with the career's result as the game's *Umamusume Details* card (Skills, Inspiration,
Career Info). It starts as a random result of the career:
- **Reroll** draws another result (targets, stars, G1 wins, her pink spark) and gets its saving from the plan at once:
  the solver's evaluation of that roll's sparks (her stars per target; her G1 wins at their expected chances, not the
  ones drawn), pricing her in the plan's routes: as a parent of a candidate's career (beside the partners the plan
  keeps) and so the grandparent of the route's last career, and as a side herself when she lands every target. That
  figure is provisional (the plan's routes are not every route a replan can take) and shows as *No saving found yet*
  when it is zero (*Checking…* when it saves careers); the best career with her as a parent follows from one quick
  search, and every roll that stands for a moment (0.8 s) then runs Compare by itself, so rerolling quickly stops no
  running check. The plan takes at most 12 targets, 10 of them not
  guaranteed by the scenario; beyond that there is no plan, so no saving and no careers to loop, only the best career
  with her. Over 12 targets the solver also gives no joint odds of the targets, so Reroll draws each target on its own
  odds, independently, which ignores how they land together (shared hints and chains).
- Set it to what she actually landed and press **Compare** for the plan with her added as she stands (her sparks,
  stars and G1 wins as entered), priced under today's plan's fit: careers to a fully looped parent now and with her, and the best career with her as a parent. Add to veterans
  fits the plan again to your veterans with her, so its figures can differ (a line under the comparison says so). The
  estimate can rise with her, a limit of the planner's route (it offers each uma's few best careers as sides, and she
  can push the one it used out of them); the panel shows the rise and says so. In the game she never costs careers: you can leave her out.
- **Add to veterans** saves her (with her parents) and hands every place back to the planner for the next career,
  which plans the pool with her afresh.

The card shows her aptitudes at the end of that result (the legacies' pink sparks included). Add to veterans closes
the popup.

**Export** (at the foot of the page) shows your veterans, the places you set and the solver job as text (*Copy*, *Download*), to move them to another
browser or replay a search outside the page.
