# Handoff: where the Legacy Loop Planner stands (2026-10-09, v41 plus unreleased work on main)

## 1. First session: set up and confirm

1. Toolchain: Rust (stable) with `rust-src` (the wasm build uses `-Z build-std` via `RUSTC_BOOTSTRAP=1`; with the
   `wasm32-unknown-unknown` target installed it also works without build-std), Node + npm (`cd webapp && npm ci`
   for binaryen's `wasm-opt`), uv (Python environment from `pyproject.toml`/`uv.lock`: UnityPy and Pillow for the
   image step, everything else is stdlib; Playwright + Chromium for the optional browser checks `webapp/pw*.py`).
2. Data: the handoff zip already has the Global `master.mdb` at `pkg/loopopt/export/master.mdb` and the JP one
   (SimpleSandman/UmaMusumeMetaMasterMDB `master/master.mdb`, 2026-09-30) at `jp/export/master.mdb`. Neither is
   tracked by git; keep them out of commits. After a game update, replace them from the same sources.
3. `./build.sh all` (data, events, native + tests, wasm, page). Expect 29 tests passing, "178 cards written" /
   "222 cards written" twice from `events`, and `webapp/dist/index.html` about 13.6 MB.
4. `uv run val/event_audit.py` → "every target hint is accounted for".
5. Optional full check: `./build.sh val check` (parallel, about 3.5 minutes; it runs `val/cmp2.py` itself)
   for both modes: everything should match the references.
6. Decide hosting (section 6) and push the repo to the owner's GitHub.

## 2. Current results (val references, 2026-10-09; per-uma decks, star mix below SS)

All proven optimal for the model, read from `val/refpu__*.out` (`top[0].loop`). Odds moved a lot since v23 (the
2026-10-05 training-hint refit; three-uma rotations, which now win most setups). The SS+ and owner-roster rows of
the v23 table were not regenerated.

| Setup (job) | Odds all 8 land | Loop |
|---|---|---|
| Unity Cup golds guaranteed, everything owned (`pal_U`) | 1 in 105 | Agnes Digital [Full-Color Fangirling], Tamamo Cross [Fast as Lightning], Inari One [Edomurasaki] |
| no guarantees, everything owned (`pal_UF`) | 1 in 241 | Agnes Digital [Full-Color Fangirling], Yukino Bijin [Darl'n Snowflake], Inari One [Edomurasaki] |
| restricted roster (`pal_F`) | 1 in 379 | Vodka [Fiery Aqua Vitae], Agnes Digital [Full-Color Fangirling], Kawakami Princess [Princess of Pink] |
| restricted roster (`pal_A2`) | 1 in 205 | Oguri Cap (any outfit), El Condor Pasa (any outfit), Agnes Digital [Full-Color Fangirling] |
| JP GM era, no guarantees (`jp/jobs/v_ng_k1`) | 1 in 142 | Agnes Digital [Full-Color Fangirling], Tamamo Cross [Fast as Lightning], Inari One [Edomurasaki] |
| JP GM era, Unity Cup golds (`jp/jobs/v_uc_k1`) | 1 in 91 | Agnes Digital [Full-Color Fangirling], Tamamo Cross [Fast as Lightning], Inari One [Edomurasaki] |

Per-setup decks are in `val/refpu__*.out` (`top[0].decks`, one per member).

## 3. What changed in the last session, and why

- **v16–v18 (Oct 2):** git + `build.sh`; the character rule (no trainee runs her own card or a group card she
  belongs to: shared-deck mode branches lazily, exact); one deck per uma (default on; skyline per uma class, lazy
  legal-deck search); group cards as pals in Global; R Anshinzawa's Risky Business restored.
- **v19:** adopted uma-it-optimizer's per-legacy affinity (receipt fit). Under the old umaishow split every spark
  chance was about half the measured level. Grandparents no longer halved.
- **umadump / uma.moe cross-check:** `val/umadump_lineage.py` wrote the best Unity loop's 12 veterans in umadump
  format; the owner loaded them in uma.moe. Every number matched ours: per-legacy affinity, per-roll and per-career
  odds, the lineage total 452 = 184 + 97 + 171. Showed titles don't count for race affinity and great-grandparents
  count for nothing. Agreement shows our implementation matches theirs; it is not independent evidence the formula
  matches the game.
- **v20:** spark generation 20%/40% × 1.1⁶ instead of rounded 35%/70%. Every loop's odds × 1.10292 exactly, no
  ranking change. (The hakuraku data can't separate 0.35 from 0.354 at six carriers; the owner asked to adopt the
  curve anyway.)
- **v21:** star mix fixed: 50/45/5 below rank score 17,500 (default), 20/70/10 at 17,500+, as a page option. The old
  45/50/5 matched neither band. Below SS: odds −2–3%, two decks swap one card. SS+: the Unity loop swaps Oguri for
  Nakayama Festa.
- **v22:** Riko Kashimoto (SSR and R) Ramp Up 50% from "Unexpected Side"; R Light Hello Playtime's Over! from "At
  Rainbow Cove"; event builders now keep chance-only manual entries (they used to drop them silently). No
  validation result changed.

Mistakes worth not repeating: a quick event decoder that read choice labels as rewards and skipped pal sections
produced several wrong event claims in conversation (corrected; production code was never affected). A wait loop
with `pgrep -f` matched itself and idled for five minutes. A full 48-job validation was run for a change that
provably only rescaled odds.

## 4. Open threads, most valuable first

0. **Career planner: speed and consistency (2026-10-08/09).** First plan on the owner's pool about 12 s -> 1.4–2 s in
   the page; sample (mob) pools 3x faster again (identical deck problems solved once: `Twin`, `deck_memo`). Every
   step is in `docs/PERF_PLANNER.md`; `val/planval.py` (seven next-plan jobs, compared whole, deterministic work
   counters) and `val/mono.py` (a veteran added never raises V) are the regression checks. Open:
   - **In progress (agent, 2026-10-09): each fresh 8/8's breeding pair chosen jointly with the route** (her top 2–3
     pairs as alternative sides), so an added veteran can never make V worse through a changed lineage. Evidence: the
     owner's Inari One veteran (exact saving 8.7 vs table 12.9; with two more G1 wins V rose 4 careers).
   - **Owner decision: carry the Cont fit across plans.** Compare now prices a pool with her under the plan's own
     fit (`next.fit`), so its verdict can't flip; but the plan run after Add refits, and "Careers to loop" can still
     rise (1,083 -> 1,103 on the owner's pool). Carrying the fit is principled only if the step model is unbiased:
     decide after item 1.
   - Per-outcome planner arithmetic on mob pools (about 1.4 s): a skip-ahead in `Cont::plan` is valid on paper but
     needs a bit-identical proof first. The pass-2 replay is parked on branch `worktree-agent-a919b16f64bbaea00`.
   - Display: on a mob pool the search's six best careers are the same career with interchangeable pairs; group
     them on the page (display only; the plan's candidates depend on the list).
2. **Star-mix band for the owner's veterans.** uma-it's summary data: an IT veteran's score range runs from "no
   skills bought" to "all affordable bought"; Unity Cup median best case is 17,428 (just under SS), Grand Concert
   18,094. Whether the owner's loop veterans clear 17,500 decides the default; ask.
3. **Measure spark generation and star mix on the owner's own veterans.** Run umadump snapshot → send
   `trained_chara_data.json` → `uv run val/spark_gen_fit.py trained_chara_data.json`. Tested on synthetic data
   (recovers ×1.094 for a true ×1.1 at 24k observations).
4. **Hint levels from inheritance.** One run showed Ramp Up Lv 5 from a single 2★ proc; the per-proc level rule is
   unknown. Receipts would settle it. It matters only if the model ever uses hint levels.
5. **Training-hint rate: done.** Refit 2026-10-05 on 2,402 runs and adopted (base per scenario × (1 + hint
   frequency/100); see `docs/MODEL.md`, Training hints). The chain outliers noted then (Agnes Digital [A Fan's Joy],
   Seiun Sky [Foolproof Plan]) are not resolved here; check `docs/CHAIN_EVENTS.md` before reopening them.
6. **Per-lineage carrier counts.** The model assumes every target spark sits on all six ancestors (a running loop).
   For bootstrapping a loop or a rented parent without a spark, generation should use 1.1ⁿ with the lineage's real
   n. Only worth it if the owner uses the tool to build loops from scratch.
7. **Smaller ideas offered earlier, not requested:** a fast quick-check validation subset; reuse the deck search
   across shared-mode branches (slow case: `pal_Fk3` shared ≈ 575 s historically); diff WASM vs native outputs; a
   "JP, latest" data mode (needs GameTora's support-card file for hint frequency); a "set these as prioritized
   skills" list on the page.

8. **Loop planner, owner decisions from the 2026-10-09 audit:** treat a reload's cached result as done when the wasm,
   data and job match (it reruns the search today, up to 10–35 s on shared-deck jobs; recommended); grey out the
   Deck (i) panel's hints its umas already have (recommended, as the swap rows now do); show a must-include uma's
   marked outfit instead of "any outfit" (recommended); "press Search" after an undo with Auto-search off (leave).
   Phone layout leftovers: a three-member result wraps 2+1; "prioritise Risky Business" wraps to three lines; the
   schedule's GOAL badge is clipped.

## 5. Standing decisions and the owner's calls

- The tool is for Independent Training only. Unity Cup is the default scenario model; the GM-era JP data is a
  secondary mode.
- The player is assumed to set prioritized skills so event choices reach the counted hints.
- Pal first-year dates don't fire in Unity Cup IT (owner); always left out. The page option was removed (2026-10-03);
  if that changes in the game, the owner will ask for the app to be updated.

## 7. Not included, and why

- Scratch experiments from the old workspace (`_gpexp`, `_affexp`, `_hintexp`, `_allexp`, `_x_*`): superseded;
  their conclusions are in the commits for v19–v21.
