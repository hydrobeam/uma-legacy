# Using the page

The live page is https://uma-legacy.pages.dev. Its own "How this works" page (`/about`) explains the model in plain
words; this file is the reference for the controls.

Both planner pages are drawn in the game's style (the same rules, html class `game`): game panels, the game's buttons,
check boxes and switches, green section bars, pink setup ribbon. The loop planner has two columns. On the left, the
**Loop setup** panel with the same rows as the career's *Career setup* (built from the same markup,
`webapp/panels.template.html`): *Game data*, *Scenario*, *Assumed lineage rank*; the game's Enhance buttons
**Umas** and **Support cards**, which open their editors in dialogs; and at its foot, in small type, the status line
and the *Auto-search* and *Deck per uma* switches. *Results* (1 or 3, the game's pill switch) sits in the top-right
corner of the results. Under it, *Target skills* in the open. On the right, the results, the first
one level with the setup panel. On a phone the columns stack, results first.

**Settings** (the Loop setup panel)
- *Game data*: "Global" (today's data), or "JP, Grand Masters" (JP data from the Grand Masters era, cut at L'Arc's launch, 24 Aug 2023). Switching
  reloads the page; owned umas, outfits and cards are kept separately for each.
- *Scenario*: Unity Cup (default), Grand Concert, URA Finale, Trackblazer; with the Grand Masters-era data also Grand
  Masters. It sets how often support cards give training hints (the picker shows each as a multiple of URA Finale's
  rate), which golds it hands out whatever the deck (Unity Cup: No Stopping Me! and It's On!), and which cards the
  game bans from the deck (Team Sirius in Unity Cup and Grand Concert). Grand Masters has no measured hint rate and
  uses a placeholder.
- *Assumed lineage rank* (the planner doesn't work out the score a deck would reach): the star mix of the sparks on your legacies, *B to S+* (rank score 6,500–17,499: 50/45/5% for
  1/2/3 stars) or *SS* (17,500+: 20/70/10%), from hakuraku.moe's room-match data. Picking a scenario sets its usual
  rank (SS for Grand Concert and Grand Masters); change it if yours differs.
- *Deck per uma* (a switch in the status row, on by default): each uma gets her own best deck. Every deck may borrow one friend's card and does when it helps;
  there is no setting for it.
- No loop-size setting: the search ranks 3-uma rotations and 4-uma loops together and shows the best. A 3-uma rotation
  holds at most three must-include umas.
- The status line (at the foot of the setup panel): searching; then "Checking the exact odds…" with the first result already shown (ranked by the
  odds with targets treated as independent); done ("Checked N decks / M loops in T s"; the result is checked on exact odds, so nothing else does
  better; only when the exact check ran out of time a short caveat follows: "exact check: decks within 11% of it" or
  "not checked exactly"); or
  what keeps the settings from being searched, with the button that fixes it; **Stop** ends a long search and keeps the
  results (stopped during the exact check, the first result stays, marked as not checked). **Results** (1 or 3)
  gives up to three, numbered.

**Target skills** (in the open under the setup panel; on the career page a dialog from its *Target skills* button; each
target as the game's skill tile, gold for a scenario gold). Type to add a skill; **8/8 example** loads the eight targets used in our runs. A gold the scenario
guarantees shows as that gold, in gold. Nothing else is set by hand: outfits, decks and events are worked out by the
search.

**Umas** (a dialog from the setup panel's *Umas* button). One tile per outfit. Tap to cycle *owned → not owned → must include*, per outfit: a must-include outfit puts
that uma in the loop in that outfit (mark several of her outfits to allow any of them). Up to four umas can be must-include; with four, only four-uma loops qualify. In the results a must-include uma has a green ring round her portrait, and the card says *Must include* at its top right. **None owned**, then tapping
the ones you have, is quickest when you own few. Outfits decide most results through their skill lists (Agnes Digital
[Full-Color Fangirling] carries Superstan, Tamamo Cross and Inari One carry Tail Nine).

**Support deck** (a dialog from the setup panel's *Support cards* button). Tap the cards you don't own; only cards that can give a target are listed, each with what it
gives. No uma runs a card of herself or a group card she belongs to. Group cards such as Progenitors and Guides count as pals.

**Results** update by themselves 0.55 s after any change (a change during a search restarts it); with *Auto-search*
off, a change waits for the Search button. A loop is either four umas (each
trains from two of the other three) or a three-uma rotation (each trains from the other two, who were bred from each
other and from her own earlier veteran); both are ranked on the same odds, the average over the careers when every
member trains in turn, and a rotation usually comes first. For each loop:
- the odds that every target lands, that all but one land, and how many careers give a 90% chance;
- each uma as a tile with her outfit, the targets on her own skill list (as the game's gold skill tile), and her odds as the trainee (click her name for her
  panel: owned switch, GameTora link);
- the deck, with pals and the borrowed card marked, and "prioritise …" on a card whose event needs that skill set as an IT priority (the search's choice at a chain link, or a pal finale such as Sasami Anshinzawa SSR's); the (i) button shows the chance of each hint it gives. With a
  deck per uma, the deck most of the loop shares comes first and each other uma's swaps follow;
- *Swaps* under a green bar: each row "her face swaps card → card";
- **Race schedule** (the game's Scheduled Races calendar, all three years; a tab per uma, then *All three* or *All four*: shared G1s build affinity; optional G1s are free extra race sparks; "A/B" means either race,
  but every runner must pick the same one) and **Affinity** (as the career page's: the loop's careers as pill switches, each trainee with her six legacies'
  affinity, the most first; under them that career's table, each parent then her two parents with their relation
  points, G1 wins shared (expected, x3) and affinity, and the Total; in a rotation the trainee is her own grandparent
  twice, marked, with no relation points).

The settings editors are dialogs, so the results stay where they are while you edit; a button at the bottom of a phone's
screen takes you back to them when they are out of view.

**Export / import settings** (bottom of the page) copies your settings and the results on screen as text, or loads
someone else's. Settings are saved in your browser on that device; nothing is uploaded.

**Career planner** (`/career`, linked from the header). Picks the next career from the veterans you have. The
*Career setup* panel (game data, scenario, lineage rank, *Target skills*; then the game's Enhance-menu buttons *Umas*,
*Support cards* and *Veteran Umamusume*) opens the loop planner's own editors and shares their saved settings; the
rank sets the stars of the new veteran's sparks it expects on a new veteran. The scenario button and its menu show the
game's scenario-select art behind each logo. Every deck may borrow one friend's card and does when it helps (no
setting). The pedigree reads left to right: the trainee, her two parents, each parent's own two
parents, joined by brackets; each of the six legacies shows her affinity in the career on show. Each place is set by hand or left to the planner, which tries every owned uma and every pair of your veterans (at
most one borrowed) and shows its pick outlined in dashes. **Choose parent** opens the parent picker: one of *Your
veterans* (those that can't go there are greyed out, with the reason), *A new one (any uma)*, or *Let the planner
pick*. Picking a new uma, or changing who a grandparent is, asks whether to add the target skills (all at 2★, 3★ or
1★, keep hers, or none; a new parent's own two parents get the same). The trainee shows the affinity her six
legacies hold (parents' and grandparents' parts).

The search shows the career with the best odds first. Then the planner picks the next career (the status line says
"Planning…", then "Planned in X s"; nothing simulated). It values your veterans by the expected number of careers to a
*fully looped parent*: the cheapest trio, 8/8s of two umas (A and B) and then a third (C) from both. A's and B's
steps count building: each one's odds can rise, each lift coming from some career's veteran used as her parent, so a
step is "build while it pays, then run her" (rate and step fitted to your veterans now, the curve's shape to
simulations; lifts get rarer near the ceiling). An 8/8 you already have (or borrow) makes a step free, unless a fresh
one (what her best career now would leave) is so much the better parent for C that running her again is cheaper. C's
odds are her odds with those two 8/8s as parents. Every owned uma's
career is scored by how many careers it brings that goal closer, on average over every result she can roll (exact),
minus the run itself: her own 8/8 counts for her own step, and her veteran counts wherever she'd be a better parent
for a step still to run (so a strong uma's career is a stepping stone for her partners as well as a shot at her
8/8). For the few highest and the best odds, plus what her best rolls open for the next career (two careers deep).
Above *Options*: the trio and its expected careers, and per step her odds now and how many lifts building pays for
(or "your 8/8, no careers"). The planner runs the highest (near-ties: the better odds) and repeats it until a roll changes the
plan. Under the odds a line says why the career on show
is (or is not) the pick. With the trainee chosen by hand there is nothing for it to pick, and with every place set by
hand the career is just scored.

**No goal steps**: the planner runs every step itself (its trio counts every 8/8 you can use, borrowed ones too, one
per career); the page shows no step note. Once you have a fully looped parent, it goes for the next one, any uma.

Under the pedigree come the career's deck (with its **Race schedule** button) and odds, then *Veteran analysis*: her card, drawn as the game's *Umamusume Details* screen (the game's own art for the grade
letters, spark bars and skill tiles where the build has it; no stats, which the model doesn't have). Her portrait and
outfit, her aptitude grades at the end of this result (Track, Distance, Style: the start grades after the legacies'
pinks, plus a roll's April firings), then three tabs: **Skills** (every target skill she could acquire this career, as the game's tiles, name only:
acquired in this result bright, gold tile for the gold version, the others greyed; a tile opens the game-style *Skill
Details* popup: acquired or not, where it comes from with each chance (her own skill, a deck card's gold or white hint,
her story events, inspiration from the legacies, the scenario), and how likely it is acquired and lands. A roll
draws them in the game's order: the sparks that land first (exact), then which skills she bought: a landed one is
bought, another with the chance of buying it without its spark landing. Skills other than the targets aren't modelled),
**Inspiration** (first; her sparks as the game's spark bars with the game's stars on their edge, her pink first, then
*Legacy Origin*: her two parents' sparks) and **Career Info** (as the game's: *Support Cards*, the career's deck;
*Legacy Umamusume*, her two parents in the game's Legacy 1 / Legacy 2 boxes with their own parents, a borrowed one a
*Guest*; *Career Record*, G1 races and wins, the race-by-race record behind *Show the races*, and the G1 wins editor).
On a wide screen the card sits beside the comparison and its buttons. The tab you pick stays across redraws; arrow keys move between tabs. The rest of the career page
follows the same style (rounded game panels, the game's own white and green buttons and check box, green section bars), in light and dark mode: the pedigree is
portraits only, as the game's legacy display: the trainee, then Legacy 1 (blue) and Legacy 2 (pink), each the parent
with her own two parents, every portrait with its affinity as a heart and a number; a portrait opens its popup: her G1 wins,
*Choose parent* / *Let the planner pick* (*Change who she is* for a grandparent), and her Inspiration view as on the
card: her sparks as the game's spark bars, then *Legacy Origin* with her parents' (all editable: click a bar for its
stars, × removes, + adds). A veteran's sparks there and
in *Your veterans* are the game's icons (skill icon or aptitude badge, stars under it): a click opens the *Spark
Details* popup (set the stars, Remove). *Your veterans* open from the setup panel's **Veteran Umamusume** button, in a
dialog with *Import*, *Paste*, *Export* and *Remove all*. These game-style parts use the game's colours rather than WCAG AAA (the rest of the site keeps AAA).

**Layout (career page, wide screens):** the tree runs down the left (the trainee, then Legacy 1 and Legacy 2), the
*Veteran analysis* beside it, its top level with the tree's (its comparison a compact panel beside the card); *Target skills* and *Options* follow at full width. The
career's odds sit in the card's header (1 in N to land all n); a click opens *Odds of this career*: the chance
of landing exactly k of the n targets, for every k, in percent (exact over the landed sets; n/n is the solver's own
figure), and the targets expected. No prose: the planner's pick shows as the tree and the *planner's pick* tag in *Options*. *Options* has no prose under it: **What do these
mean?** opens a popup saying what *saves per run* means and why the best odds aren't the loop's best pick, with the plan
the numbers come from. The comparison sits in a panel beside her card headed *Comparison* ("Compare/reroll to see if she
improves your loop" until you do). Under *Target skills*: *Pinks passed on*, *Start aptitudes*, *Her pink*. In *Career
Info*, a support card opens its card panel. A click on the backdrop closes any popup. A legacy box (the tree's nodes, Career Info's Legacy 1/2) opens from
anywhere in it. A veteran's Inspiration cells start
with her blue (stat), pink (aptitude) and unique (green) sparks, a bar each in the game's colour (dashed when she has
none); the blue and pink bars open a picker (the five stats; the aptitudes a pink can come from, those she ends at A or S, or
her uma's base A/S ones when her end grades aren't known; the stars; no remove: every veteran has one of each; the
blue picker notes that a 3★ blue needs that stat at 600+); her unique spark is clicked like a white one (stars, × to
remove, or + to add it). **+** offers only the
target skills she lacks, as white spark cells. A roll draws her blue (a stat, stars by the rank band: stats aren't
modelled) and her unique spark (always: guaranteed for an outfit at 3★ or more; stars by the rank band). Imports keep blue and unique
sparks. Under the trainee, **Deck**
opens her career's deck there (a card opens its panel) and **Race schedule** its window. In the card's header, between
her portrait and her name: **Compare**, **Reroll**, **Add to veterans** (she starts as a random
result, drawn as Reroll draws; there is no default result). *Your veterans* are compact cards (portrait, name, targets, G1 wins, her
parents' faces); a card, or a legacy's portrait in *Career Info*, opens her popup: her sparks and her parents' as the
game's spark bars, edited in place (click a bar for its stars, × removes, + adds), her G1 wins, and *change who she is*
for her and her parents. The tree's popups show the same bars without the "Sparks" header. Every dialog on the page has
the game's look (green title bar, pill buttons) and a hovered or focused control's tooltip shows as the game's balloon. It starts as a random result of the career on show, drawn exactly as **Reroll** draws (below), and is drawn again
whenever a new search result arrives; it does not compare until you press Compare. Set it to what
she actually landed (click a spark for its stars, × to remove, + to add; *G1 wins · edit* opens the G1 wins editor
described under *Your veterans*, where the G1s she could enter at a turn are offered, her schedule's race first with its
win odds, and a win in her schedule shows its odds in the turn bar) and press **Compare**. The *Comparison* panel says it in one line, *If this hit:* then *saves N careers*, *a parent of
the planner's next career* (under half a career, but its pick uses her), *better odds, no careers saved*, *no change
yet*, *an 8/8, step 1 of the goal done* or *a fully looped parent*; then two figures, now → with her, green when she
makes them better and red when worse: *Careers to a fully looped parent* (the planner's expected careers once its next
career runs; hover for its next career with her) and *Best odds, all N* (today's best career → the best career with her
as a parent; hover for the percentages and the N−1 odds); then *Best career with her* as portraits (trainee, then her
two parents, *her* marked), its deck behind *Show its deck*. A veteran never raises the count (every plan without her
is still there): the two plans are fitted to their own pools, and a rise between them reads as no change. A veteran can
save careers without being anyone's better parent today: as a builder's parent she becomes a grandparent of the best
career's next lineage, which the planner's scores count. **Reroll** draws one from this career's odds (targets, stars by rank, G1 wins by their
odds, and her pink: each legacy pink fires or not at both April inspirations, then one aptitude she ends at A or S, stars
20/70/10; the G1 wins are drawn from their average odds, not from that roll's aptitudes) and compares at once; rolling (or pressing Compare) again while a comparison runs cancels it and starts the new one. An edit after Compare marks the comparison out of date. **Add to veterans** saves her (with her parents, who become the next career's grandparents) and
hands every place back to the planner for the next generation.

Below: the odds (all targets land as sparks, all but one, targets expected), the deck with the *Race schedule* button
in its title row, the target skills (per target the chance the lineage passes it on, how many of the six carry it, the
chance it lands as a spark) and the aptitude (pink) sparks' chances of being passed on; then her aptitudes the
legacies' pinks raise at the start (e.g. Dirt F → E: the six legacies' pink stars for it summed, 1/4/7/10★ give 1-4
grades, at most A) and the odds of her own pink spark (one aptitude she ends at A or S, uniformly; the April
inspirations' firings can lift an aptitude, A to S included). Her schedule's win chances count all of it. The
odds and the deck look as on the loop planner page: click a deck card for its panel (owned or not, GameTora); marking
it not owned searches again (it may come back as the borrowed card). *Race schedule* opens the
planner's schedule window for the trainee, drawn as the game's *Scheduled Races*: the three years side by side, sized to fit
the window, each turn the game's white slot with the race's banner and its win odds in the pink
*Scheduled* badge (grey when no parent won it, pink when a parent did: it builds affinity), Junior's April to June the
game's grey cells. The G1 wins editor uses the same calendar: its empty turns where a G1 is held show the game's green
+, opening the races held then as their banners in the game's slots.

With any place left to the planner, *Options* is a table of what it weighed, under the planner's expected careers
to a fully looped parent and its plan (e.g. "about 1,905 (Agnes Digital → Special Week → Seiun Sky, each until her
8/8)"): per option "This career: 1 in X" and *Saves per run* (hover for her role, how often she lifts the step-1 trainee and
to what odds, and the two-deep part), tags *planner's pick*, *about the same* and *best odds now*, and **Use this**; the career on show is marked. The eight highest are listed (and always the best odds), with
one line for the rest ("41 more umas scored, +7.1 to +46.2 per run"). While the planner works, the six best by odds with
"planning…".

**Your veterans** (the *Veteran Umamusume* dialog): *Import umadump file* reads umadump's `trained_chara_data.json` (or
*Paste* its text): each veteran's skill and aptitude sparks with stars, single-race G1 wins and both parents; a
re-import replaces earlier imports and keeps veterans made here. *Export* shows your veterans, the plan and the
solver job the planner sends for them (to replay a search outside the page) as text, with *Copy* and *Download*;
*Paste* takes that text back, *Import* the downloaded file. *Paste* also takes an uma.moe lineage planner export: each
top veteran (p1, p2, ...) with her two parents is added after yours, marked yours (her popup says *From uma.moe*) (switch it
to Borrowed if she is a friend's); uma.moe's spark id x 10 + its level is the game's factor id, and its win saddles are
the game's. In every node: a spark is the game's icon; click it for
its popup (set 1, 2 or 3 stars, Remove), **+** lists the target skills she lacks and the aptitudes, click a
portrait to change who she is, the G1 button opens the G1 wins editor, *Yours / Borrowed* marks a borrowed veteran.

The G1 wins editor (a veteran's, and the veteran analysis') is the schedule window's calendar with her wins on it: each
win sits in its slot as a banner card, and a click removes it, leaving the slot empty. An empty slot where a G1 is held
shows a **+** on hover or keyboard focus (faintly at all times on a touch screen); a click opens a menu of the G1s held
at that turn, and picking one marks it won in that slot (Escape or a click elsewhere closes the menu). A race held in
Classic and Senior, such as the Arima Kinen, is one race and one win: it shows once, at the turn it was picked at, else
the turn her schedule runs it, else its first turn (an imported win carries no turn); the menu at its other turn offers
to move it there. The turn is only for display; the planner reads which races she won. The count of wins is in the
header; a veteran's edits are saved at once and the planner searches again when the editor closes.
