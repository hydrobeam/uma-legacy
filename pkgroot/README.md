# Legacy Loop Planner

Finds the best four-uma legacy loop (Uma Musume, Global) for either affinity or a set of target skill sparks, and gives each uma's race schedule.

## Quick start: use the page

1. Unzip.
2. Double-click **`Legacy Loop Planner.html`**. It opens in any modern browser (Chrome, Edge, Firefox, Safari), works offline, and needs no install or server. The whole solver is inside that one file.
3. Follow the "How to use" steps at the top of the page.

`webapp/index.template.html` is the source the page is built from. Opened on its own it has no roster and no solver, and it says so.

## Using the page

1. **Umas.** Tap a name to cycle *owned → not owned → must include*. If you own few, press **None owned** and tap the ones you have.
2. **Outfits.** Open the Outfits list and switch off alt outfits you don't own. Outfit skill lists decide most results (Gold Ship [Red Strife] = Uma Stan, Digital [Full-Color Fangirling] = Superstan, Tamamo / Inari = Tail Nine).
3. **Target skills.** Type to add a skill. For each one set:
   - *Guaranteed source*: white or gold from the scenario or a card.
   - *Deck hint %*: chance your deck hands you the hint, e.g. 54% with 3 pals, 81% with 4.
   - Leave both at 0 for inherit-only.

   **Unity Cup example** loads the 8-skill set used in our runs.
4. **Lineage rank.** Star mix of the sparks on your legacies. Unity Cup tops out at A: use *B to SS*.
5. **Find best loops.** It returns the top 5 loops. For each loop you get:
   - odds that all targets land, and all but one;
   - runs for a 90% chance;
   - each uma's outfit and the targets it covers;
   - each trainee's own odds;
   - the race schedule. Shared G1s build affinity; optional G1s are free extra race sparks. "A/B" means either race, same odds, but every runner must pick the same one.

**Support deck.** With "Find best deck" on, tap the cards you don't own (pals, group cards, R cards and every SR/SSR that can give a target are listed; R cards have no chain events, so like pals they take no chain slot and can be switched off) and choose whether one friend card may be borrowed. The search returns the best deck(s), pals and borrowed card marked, each with its own best loop and the odds each target gets from the deck. One deck usually takes 1–15 s, three up to about 2 minutes. "Set odds by hand" uses the per-skill deck hint % instead.

**Game data.** "Global today" or "JP, Grand Masters era" (JP data cut at L'Arc's launch, 24 Aug 2023: 83 umas, group cards such as Progenitors and Guides count as pals). Switching reloads the page; owned umas, outfits and cards are kept separately for each. Two event options apply to both: whether pal first-year dates fire (they don't in Unity Cup independent training) and how often random events that hint a target fire (unknown, so off by default). For a GM loop, set No Stopping Me! and It's On! to "None" and pick a higher lineage rank.

Settings are saved in your browser on that device. Nothing is uploaded.

**Affinity only** mode ignores skills and maximises affinity, with grandparent slots at half weight.

## Host it (optional)

Upload `Legacy Loop Planner.html` anywhere that serves static files: GitHub Pages, Cloudflare Pages, Netlify. Rename it to `index.html` if you want it at the site root. No special headers are needed.

## Command line (Rust)

```bash
cd pkg/loopopt
cargo build --release
target/release/loopopt data.json jobs/skills_unity8.json       # skill-target search (example job)
target/release/loopopt data.json jobs/best_full_roster.json   # affinity search
cargo test --release                                          # 26 tests
```

Job fields and the model are in `pkg/loopopt/README.md`. Saved result sheets are in `pkg/loopopt/results/`.

## Rebuilding

- **Everything** goes through `./build.sh [step...]` at the top level: `native` (binary + tests), `wasm` (wasm, wasm-opt, wasm2js fallback, example), `page` (`webapp/dist/index.html`), `events` (card event caches), `data` (master.mdb exports), `val TAG` (24 validation jobs compared with `val/ref__*`). No step = `native wasm page`. Copy `webapp/dist/index.html` over `Legacy Loop Planner.html` afterwards.
- Step details for the wasm build are in `webapp/README.md`.
- **Game data update:** put your own copy of the Global `master.mdb` (from the game's local data folder) in `pkg/loopopt/export/`, then:

  ```bash
  cd pkg/loopopt/export
  python3 build_base.py && python3 export_rs.py && python3 export_web.py
  ```

  `master.mdb` is not included in this zip.

## Layout

```
Legacy Loop Planner.html   ready-to-use page (solver + data embedded)
webapp/                    page source: template, worker, build script
pkg/loopopt/               Rust solver, data.json, web_skills.json, job files, results, tests
pkg/loopopt/export/        Python exporters (master.mdb -> data.json, web_skills.json)
```
