# Handoff: where the Legacy Loop Planner stands (2026-10-03, v23)

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
5. Optional full check: `./build.sh val check` (about 6 minutes on 2 cores) and `uv run val/cmp2.py check ref`
   / `uv run val/cmp2.py checkpu refpu`: everything should match the references (they are v22/v23 outputs).
6. Decide hosting (section 6) and push the repo to the owner's GitHub.

## 2. Current results (v23, per-uma decks, star mix below SS)

All proven optimal for the model. Outfits as the solver chose them.

| Setup (job) | Odds all 8 land | Loop |
|---|---|---|
| Unity Cup golds guaranteed, everything owned (`pal_U`) | 1 in 142 | Oguri Cap (any outfit), Agnes Digital [Full-Color Fangirling], Tamamo Cross [Fast as Lightning], Inari One [Edomurasaki] |
| same at the SS+ star mix | 1 in 128 | Agnes Digital [Full-Color Fangirling], Tamamo Cross [Fast as Lightning], Inari One [Edomurasaki], Nakayama Festa [Desperate Measures] |
| no guarantees, everything owned (`pal_UF`) | 1 in 364 | Vodka [Fiery Aqua Vitae], Agnes Digital [Fanatic♡Jiangshi], Yukino Bijin [Darl'n Snowflake], Kawakami Princess [Princess of Pink] |
| restricted roster (`pal_F`) | 1 in 543 | Special Week [Hopp'n♪Happy Heart], Vodka [Fiery Aqua Vitae], Agnes Digital [Full-Color Fangirling], Kawakami Princess [Princess of Pink] |
| restricted roster (`pal_A2`) | 1 in 291 | Oguri Cap, Daiwa Scarlet, El Condor Pasa (all any outfit), Agnes Digital [Full-Color Fangirling] |
| JP GM era, no guarantees (`jp/jobs/v_ng_k1`) | 1 in 209 | Agnes Digital [Full-Color Fangirling], Tamamo Cross [Fast as Lightning], Inari One [Edomurasaki], Yaeno Muteki [Blazed Head, Covered Fists] |
| JP GM era, Unity Cup golds (`jp/jobs/v_uc_k1`) | 1 in 117 | Agnes Digital, Tamamo Cross, Inari One (as above), Nakayama Festa [Desperate Measures] |
| the owner's roster (`jobs/owner_2026-10.json`) | 1 in 302 | Taiki Shuttle (any), El Condor Pasa (any), Agnes Digital [Full-Color Fangirling], Yamanin Zephyr [Fluttertail Spirit] |

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

2. **Star-mix band for the owner's veterans.** uma-it's summary data: an IT veteran's score range runs from "no
   skills bought" to "all affordable bought"; Unity Cup median best case is 17,428 (just under SS), Grand Concert
   18,094. Whether the owner's loop veterans clear 17,500 decides the default; ask.
3. **Measure spark generation and star mix on the owner's own veterans.** Run umadump snapshot → send
   `trained_chara_data.json` → `uv run val/spark_gen_fit.py trained_chara_data.json`. Tested on synthetic data
   (recovers ×1.094 for a true ×1.1 at 24k observations).
4. **Hint levels from inheritance.** One run showed Ramp Up Lv 5 from a single 2★ proc; the per-proc level rule is
   unknown. Receipts would settle it. It matters only if the model ever uses hint levels.
5. **Adopt measured deck-side rates?** uma-it-optimizer measured Unity training hints ≈ 3.04 + 0.038·hint_frequency
   (we assume 1 + hf/25) and chain completion (final link ≈ 24.5%, pal multipliers ×0.22/0.69/1.06/1.69). Not
   adopted; would change rankings. Decide with receipt data in hand.
6. **Per-lineage carrier counts.** The model assumes every target spark sits on all six ancestors (a running loop).
   For bootstrapping a loop or a rented parent without a spark, generation should use 1.1ⁿ with the lineage's real
   n. Only worth it if the owner uses the tool to build loops from scratch.
7. **Smaller ideas offered earlier, not requested:** a fast quick-check validation subset; reuse the deck search
   across shared-mode branches (slow case: `pal_Fk3` shared ≈ 575 s historically); diff WASM vs native outputs; a
   "JP, latest" data mode (needs GameTora's support-card file for hint frequency); a "set these as prioritized
   skills" list on the page.

## 5. Standing decisions and the owner's calls

- The tool is for Independent Training only. Unity Cup is the default scenario model; the GM-era JP data is a
  secondary mode.
- The player is assumed to set prioritized skills so event choices reach the counted hints.
- Pal first-year dates don't fire in Unity Cup IT (owner); always left out. The page option was removed (2026-10-03);
  if that changes in the game, the owner will ask for the app to be updated.

## 7. Not included, and why

- Scratch experiments from the old workspace (`_gpexp`, `_affexp`, `_hintexp`, `_allexp`, `_x_*`): superseded;
  their conclusions are in the commits for v19–v21.
