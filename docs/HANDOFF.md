# Handoff: where the Legacy Loop Planner stands (2026-10-10)

Read `AGENTS.md` first. This file is the state of the work: current results, releases, what is open, the standing
decisions, and the lessons. The model and its evidence are in `docs/MODEL.md`, the solver in `docs/SOLVER.md`,
building in `docs/BUILD.md`, speed work in `docs/PERF.md` and `docs/PERF_PLANNER.md`. How the work got here is in git:
the tags (`git tag -n1`) and the commit messages.

## 1. Current results (val references, 2026-10-09; per-uma decks, star mix below SS)

All proven optimal for the model, read from `val/refpu__*.out` (`top[0].loop`; per-setup decks in `top[0].decks`, one
per member). The SS+ and own-roster rows of the v23 table were not regenerated.

| Setup (job) | Odds all 8 land | Loop |
|---|---|---|
| Unity Cup golds guaranteed, everything owned (`pal_U`) | 1 in 105 | Agnes Digital [Full-Color Fangirling], Tamamo Cross [Fast as Lightning], Inari One [Edomurasaki] |
| no guarantees, everything owned (`pal_UF`) | 1 in 241 | Agnes Digital [Full-Color Fangirling], Yukino Bijin [Darl'n Snowflake], Inari One [Edomurasaki] |
| restricted roster (`pal_F`) | 1 in 379 | Vodka [Fiery Aqua Vitae], Agnes Digital [Full-Color Fangirling], Kawakami Princess [Princess of Pink] |
| restricted roster (`pal_A2`) | 1 in 205 | Oguri Cap (any outfit), El Condor Pasa (any outfit), Agnes Digital [Full-Color Fangirling] |
| JP GM era, no guarantees (`jp/jobs/v_ng_k1`) | 1 in 142 | Agnes Digital [Full-Color Fangirling], Tamamo Cross [Fast as Lightning], Inari One [Edomurasaki] |
| JP GM era, Unity Cup golds (`jp/jobs/v_uc_k1`) | 1 in 91 | Agnes Digital [Full-Color Fangirling], Tamamo Cross [Fast as Lightning], Inari One [Edomurasaki] |

## 2. Releases

Live at https://uma-legacy.pages.dev (hosting: `docs/BUILD.md`, Releases): v49 in production, deployed 2026-10-10
(v49: an optional race's "x% win" legible in the race schedule; v48, page only: the runner, each uma's mini model running in place of the spinner, `./build.sh runners`; swap rows as the
outgoing card greyed -> the incoming one; a reload with unchanged settings shows the saved result as done; the status
line's words beside the runner; Career choices tooltips in plain words, "about the same" stated right (5%, at least a
quarter of a career); the affinity popup one pill per trainee with a Parents row in four-uma loops; Grand Masters'
banner art and 14 skill icons from the JP client's bundles). v45 (same day): Zenno Rob Roy.
Global data updates: a new `master.mdb`, then data, validation, deploy (`docs/BUILD.md`, Data).
Open: the policy simulator to the fully-looped goal to measure V against simulated play (`val/sim/README.md`); the
calibration's weak spots (`docs/CALIBRATION.md`).

Milestones since 2026-10-03 (v24; v20–v39 are lightweight tags, v40 on annotated):

| Tag | State |
|---|---|
| v24 | page rework (outfit tiles, card grid, agenda, auto-search), browser suite, versioned settings, ES modules |
| v25 | exporter fixes: split-outcome skills at k/n, unique-effect hint frequency |
| v26 | solver speed-ups (every search 42-55% of its old time, outputs unchanged; `docs/PERF.md`); shared compiled wasm module |
| v27 | chains that IT's default choice can end (`stop_risk`); secret-event list; parallel validation tiers |
| v28 | page as files (HTML 0.63 MB, wasm, lazy images, GM data on demand); relation groups in data.json; per-outfit must include |
| v29 | deck odds exact over event choices and outcomes, exact joint P(all); `./build.sh deploy` |
| v30 | chain events Binomial(21, 0.35) and the training-hint model, measured on uma-it runs |
| v31–v33 | scenario picker, Grand Masters, R cards off by default; page speed (Lighthouse mobile 69 -> about 88), accessibility, indexing allowed |
| v34–v37 | redesign and /about; scenario card bans; measured event outcome shares; dark mode; the model spec on /about |
| v38–v40 | ranking by exact odds with a certified cap (`EXACT_RHO`); the career planner (`/career`); three-uma rotations; both planner pages in the game's style |
| v41 | career planner speed (first plan about 12 s -> under 2 s), Reroll's per-outcome table, the career planner on /about |
| v42 | one run, one answer on the career page; sample veterans 3x faster; mobile pass; loop planner audit fixes |
| v43 | Reroll and Grand Masters speed: per-class exact caps, gm16 plan -27.5%; band-overflow ladder; deck and corner memos kept between jobs; `y_hi` tightened |
| v44 | target skill presets, dark-mode mirror, career page polish, Saves per run explained plainly |
| v45 | master.mdb 10008200 (Zenno Rob Roy); banners held ahead turn on at their start time |
| v46 | game-style header (page tabs, light / dark switch) and wordmark, /about Sources list, career presets popup fix, /loop |
| v47 | game-style status line and header switch, wordmark spacing, logo not a link, Career choices (i) |
| v48 | the runner, swap rows, reload cache, affinity by trainee, Grand Masters art |
| v49 | see above |

## 3. Open threads, most valuable first

0. **Career planner: consistency and speed.** Regression checks: `val/planval.py` (seven next-plan jobs, compared
   whole, deterministic work counters) and `val/mono.py` (a veteran added never raises V). Open:
   - **Lineage alternatives, parked** (each fresh 8/8 from her 3 best pairs; a parked branch,
     job field `next.alt_k`, not on main): V 0.6-1.5% better and the extra-wins rise fixed, but +20-40% planner
     work and still not monotone. To decide: merge off (recommended), on, or leave.
   - **A rise in V when a veteran is added** remains possible: the step model (Cont) is refitted per pool. Its error
     (`val/sim/README.md`, "E5 reread"): within 0-5% of achievable building on 2/11/19 veterans, 70% too pessimistic
     on 5 because lam rises there (builders improve). Proposed, not built: a one-lift lookahead for lam and a per-plan
     pick-sensitivity check.
   - The pass-2 replay is parked on a branch; further speed items: `docs/PERF_PLANNER.md`,
     Open.
   - Display: on a mob pool the search's six best careers are the same career with interchangeable pairs; group them
     on the page (display only; the plan's candidates depend on the list).
1. **Test the deck side against real careers.** The training-hint, chain-event and outcome-share fits were made part
   by part on uma-it runs; the end-to-end calibration (`docs/CALIBRATION.md`) tests the composed model. The chain
   outliers Agnes Digital [A Fan's Joy] and Seiun Sky [Foolproof Plan] noted at the hint refit are unresolved; check
   `docs/CHAIN_EVENTS.md` (section 5) before reopening them.
2. **Star-mix band for a player's veterans.** An IT veteran's score runs from "no skills bought" to "all affordable
   bought"; Unity Cup's median best case is 17,428 (just under SS), Grand Concert's 18,094. Whether loop veterans
   usually clear 17,500 decides the default.
3. **Measure spark generation and star mix on real veterans.** From a umadump snapshot's
   `trained_chara_data.json`: `uv run val/spark_gen_fit.py trained_chara_data.json`. Tested on synthetic data
   (recovers ×1.094 for a true ×1.1 at 24k observations).
4. **Hint levels from inheritance.** One run showed Ramp Up Lv 5 from a single 2★ proc; the per-proc level rule is
   unknown. Receipts would settle it. It matters only if the model ever uses hint levels.
5. **Per-lineage carrier counts in the loop planner.** The career planner already uses each legacy's real sparks and
   1.1ⁿ with the real carrier count; the loop planner assumes every target spark on all six ancestors (a running loop).
   Only worth changing if the loop planner is used to build loops from scratch.
6. **Three-uma rotations, follow-ups.** Whether to also offer the best single lineage (a player re-training only her
   best trainee); a five-member or mixed structure is not done.
7. **Loop planner, decisions open from the 2026-10-09 audit:** grey out the Deck
   (i) panel's hints its umas already have (recommended, as the swap rows now do); show a must-include uma's marked
   outfit instead of "any outfit" (recommended); "press Search" after an undo with Auto-search off (leave). Phone layout
   leftovers: a three-member result wraps 2+1; "prioritise Risky Business" wraps to three lines; the schedule's GOAL
   badge is clipped.
8. **Pending review:** the solver's affinity-only modes and jobs (off the page; not the aim)
   and the `skills` job (fixed odds, unused since "Set odds by hand" left the page) stay in the Rust code.
10. **Global asset sync (tabled 2026-10-10).** New umas' art and mini models still come from a local game install
   (`./build.sh images runners`); without them the page shows initials and the spinner.
11. **Runner follow-ups (2026-10-10).** Attachments (`attachment_model_id`) aren't drawn; `chibi.py`'s tool downloads
   were never run (the spike's tools were copied in); on the first search after a load the strip can be blank while it
   downloads (it is fetched when she is picked, so later searches are covered). Ideas offered, not requested: idle and
   cheer rows in the strip (other mini motions) for the end of a search; her running across the status bar as progress.
9. **Smaller ideas offered earlier, not requested:** reuse the deck search across shared-mode branches (slow case:
   `pal_Fk3` shared about 575 s historically); diff WASM vs native outputs; a "JP, latest" data mode (needs GameTora's
   support-card file for hint frequency); a "set these as prioritized skills" list on the page.

## 4. Standing decisions

- The tool is for Independent Training only. Unity Cup is the default scenario; the GM-era JP data is a secondary mode.
- The page is game-styled throughout, /about included (2026-10-10): one header on every page (the three pages as
  the game's tab bar, the current one green; the light / dark switch), the wordmark in the game's title style, /about's
  headings as the game's section titles. /about keeps AAA except the current tab. Copy: less is more, a cute touch.
- The busy sign is the runner, each uma's mini model running (2026-10-10; `docs/BUILD.md`, Runner strips).
- Deploys only on the maintainer's explicit word, each time.
- The Global asset sync is tabled (section 3, item 10).
- A reload whose settings match the saved last result (same solver, data, held-back entries and job) shows it as done
  and searches nothing (2026-10-10, reversing the earlier "rerun").
- The player is assumed to set prioritized skills so event choices reach the counted hints.
- Pal first-year dates don't fire in Unity Cup IT; always left out (the page option was removed 2026-10-03). If that
  changes in the game, the model changes with it.
- R cards stay out of the page (the Use R cards switch was removed in v36; the research behind their event rates is
  thin) and out of measured results until there is data.
- Secret and race-result character events: not worth modelling (`docs/MODEL.md`).
- Chain events: keep the binomial and revisit with more data.
- Loops and rotations are ranked by the rotation average (every member trains in turn), 2026-10-07.
- Career planner: builders go through their deck class (2026-10-08); the proxy margins 0.06 (2026-10-09); the 1.25
  cap stays as the top cap; parallel helper workers are a last resort.
- Per-class exact caps, 2026-10-09: 1.05 for at most two chain-varying targets (0-1 not proven: chain
  cards compete for events, which can correlate one target's feeders), 1.25 for three or more (the 1.10 proposed for three is not used: boot_mob scores 1.127). `val/capaudit.py` checks.
- uma-it statistics are pooled, never broken down or explained by who ran the career.
- Colours: both planner pages use the game's colours, held to a 3:1 contrast floor by the suite's audit; /about keeps
  WCAG AAA. No light colours on light backgrounds.
- Hosting on Cloudflare Pages, not GitHub Pages: the page is built locally (master.mdb, game art, the Rust
  toolchain), and no build artifact lands in git per release. Indexed since 2026-10-05 (it sent noindex
  before, over the game art); canonical https://uma-legacy.pages.dev/.
- Game assets and decryption keys are never committed.

## 5. Lessons

- A broad CSS descendant rule on one page (`.rb .lptgt .targets { grid-area: list }`) reached a dialog inside the panel
  and split it into two columns; scope page-layout rules to the panel's own children (`> .targets`).
- A `url()` inside a CSS custom property resolves against the stylesheet that uses the variable, not the element's
  page: set image URLs as the property itself.
- `gmtip.js` lifts a hovered element's title into `data-tip` while its balloon shows: a test reading `title` after a
  click can see it empty when layout moves the pointer onto it.
- Compare plans of two pools only under one fit, and know what a refit moves (V by several percent of itself).
- A model property such as "a veteran never costs careers" needs a tripwire (`val/mono.py`), not an assumption.
- Forks re-read the whole conversation; use general-purpose agents with briefs for self-contained work.
- On battery or a shared machine, measure speed with the deterministic work counters or instruction counts, not wall
  time.
- Verify a deploy against its own hash address: the branch addresses cache HTML for 5 minutes (`docs/BUILD.md`).
