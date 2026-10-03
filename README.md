# Legacy Loop Planner

Finds the best four-uma legacy loop and support decks (Uma Musume, Global) for landing a set of target skill sparks in Independent Training, and gives each uma's race schedule.

## Quick start: use the page

1. Unzip.
2. The page is the folder `webapp/dist/` (`index.html`, `loopopt.wasm`, `img/`). It fetches its solver and images, so serve the folder instead of opening the file: `./build.sh serve` (or any static server, e.g. `python3 -m http.server -d webapp/dist`), then open http://localhost:8000. Any modern browser; works offline once served locally.
3. Follow "Using the page" below.

The page is built from `webapp/index.template.html` (markup), `webapp/src/` (stylesheet and script modules) and the embedded data; the template on its own is not a working page.

## Using the page

1. **Umas.** One tile per outfit, sorted by release (or by name; either direction). Tap a tile to cycle *owned → not owned → must include*: owned is per outfit, must include is per uma. If you own few, press **None owned** and tap the ones you have.
2. **Outfits** decide most results through their skill lists (Gold Ship [Red Strife] = Uma Stan, Digital [Full-Color Fangirling] = Superstan, Tamamo / Inari = Tail Nine).
3. **Target skills.** Type to add a skill. For each one set *Guaranteed outside the deck* if the scenario hands you its
   white or gold anyway (Unity Cup: No Stopping Me!, It's On!). The deck's hints are worked out by the search.

   **Unity Cup example** loads the 8-skill set used in our runs.
4. **Lineage rank.** Star mix of the sparks on your legacies: *Below SS* (rank score 6,500–17,499, stars 50/45/5%) or *SS and up* (17,500+, 20/70/10%), from hakuraku.moe's room-match data. An Independent Training veteran reaches SS only if you spend nearly all its skill points.
5. **Results** update by themselves: any change starts a new search after a short pause (a change during a search restarts it). For each loop you get:
   - odds that all targets land, and all but one;
   - runs for a 90% chance;
   - each uma's outfit and the targets it covers;
   - each trainee's own odds;
   - the race schedule. Shared G1s build affinity; optional G1s are free extra race sparks. "A/B" means either race, same odds, but every runner must pick the same one.

**Support deck.** Tap the cards you don't own (only cards that can give a target are listed) and choose whether one friend card may be borrowed. R cards give training hints only and can be switched off under Game data. The search returns the best deck(s), pals and borrowed card marked, each with its own best loop and the odds each target gets from the deck. With "A deck per uma" on (the default), each trainee gets her own best deck: Digital, whose outfit already carries Superstan, drops King Halo for a card that helps her, say. The result shows the deck most of the loop shares and each other uma's swaps. No uma runs a card of herself or a group card she belongs to. One search usually takes 1–15 s, three results up to about 2 minutes.

**Game data.** "Global today" or "JP, Grand Masters era" (JP data cut at L'Arc's launch, 24 Aug 2023: 83 umas, group cards such as Progenitors and Guides count as pals). Switching reloads the page; owned umas, outfits and cards are kept separately for each. Pal first-year date events are left out (they don't fire in Unity Cup independent training). How often random events that hint a target fire is an option (unknown, so off by default). For a GM loop, set No Stopping Me! and It's On! to "None" and pick a higher lineage rank.

Settings are saved in your browser on that device. Nothing is uploaded.

Inheritance chances use uma-it-optimizer's per-legacy affinity model, fitted to Independent Training receipts ([Acrith/uma-it-optimizer](https://github.com/Acrith/uma-it-optimizer)).

## Host it (optional)

Upload the whole `webapp/dist/` folder anywhere that serves static files: Cloudflare Pages, GitHub Pages, Netlify. No special headers are needed.

## Command line (Rust)

```bash
cd pkg/loopopt
cargo build --release
target/release/loopopt data.json jobs/skills_unity8.json       # skill-target search (example job)
target/release/loopopt data.json jobs/best_full_roster.json   # affinity search
cargo test --release                                          # 29 tests
```

Job fields and the model are in `pkg/loopopt/README.md`. Saved result sheets are in `pkg/loopopt/results/`.

## Rebuilding

- **Everything** goes through `./build.sh [step...]` at the top level: `native` (binary + tests), `wasm` (wasm, wasm-opt, example), `page` (`webapp/dist/index.html`), `images` (character icons and card art from the game's decrypted files), `events` (card event caches), `data` (master.mdb exports), `val TAG` (24 validation jobs, shared deck then per-uma decks, compared with `val/ref__*` and `val/refpu__*`). No step = `native wasm page`. `serve` serves the result on http://localhost:8000.
- Step details for the wasm build are in `webapp/README.md`. Python runs through uv (`uv run script.py`), in the environment `pyproject.toml` and `uv.lock` define.
- **Game data update:** put your own copy of the Global `master.mdb` (from the game's local data folder) in `pkg/loopopt/export/`, then:

  ```bash
  cd pkg/loopopt/export
  uv run build_base.py && uv run export_rs.py && uv run export_web.py
  ```

  `master.mdb` is not included in this zip.

## Layout

```
webapp/                    page source: template, worker, build script
pkg/loopopt/               Rust solver, data.json, web_skills.json, job files, results, tests
pkg/loopopt/export/        Python exporters (master.mdb -> data.json, web_skills.json)
jp/                        Grand Masters-era data and exporters (JP master.mdb, cut at L'Arc's launch)
val/                       validation jobs and reference outputs (./build.sh val TAG)
build.sh, BUILD.md         one entry point for every build step
inputs/                    reference material handed over by hand (saved pages, tables, the owner's export)
.git/                      full history; releases are tagged vN to match the published page version
```

The page (`webapp/dist/`) is a build output and is not tracked by git.
