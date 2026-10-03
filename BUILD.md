# Build

`./build.sh [step...]` from this folder. No step = `native wasm page`.

Python runs in the project's uv environment (`pyproject.toml`, `uv.lock`, `.python-version`; `.venv/` is local and
created on first use). Run any script as `uv run path/to/script.py`; nothing goes into the system Python.

| Step | Does | Needs |
|---|---|---|
| `data` | Global + GM-era exports (data.json, web_skills.json, web_cards.json) | `pkg/loopopt/export/master.mdb`, `jp/export/master.mdb` |
| `events` | card event caches from `pkg/loopopt/gametora/` + manual entries (Global, GM, GM without first-year dates) | Global master.mdb |
| `native` | release binary, tests | cargo |
| `pb` | the exact pairwise affinity ceilings of every uma pair, stored as `pb` in both `data.json` files (`export/add_pb.py`, via `loopopt pbtable`); saves 15-25% of a typical search. The table carries a fingerprint of the umas, goals and streak penalties; when it does not match (data re-exported, `PB_VERSION` bumped) the solver ignores it and computes the ceilings itself, and `cargo test` warns until `pb` is rerun | native |
| `wasm` | wasm32 build (build-std), wasm-opt -O3, example.json | rust-src, node (`webapp/node_modules`) |
| `page` | `webapp/dist/`: `index.html` (the template, `webapp/src/style.css` and the `webapp/src/*.js` modules bundled by esbuild into one inline IIFE, unminified, plus the Global data), `loopopt.wasm`, `data-gm.json` + `engine-gm.json` (the GM era's page lists and solver data, fetched only when that era is selected), and `img/` (copied from `webapp/img`) | node (`webapp/node_modules`, installed by `npm ci` on first use) |
| `serve` | serves `webapp/dist` on http://localhost:8000 (`PORT=...`; `webapp/serve.mjs`). The page fetches its wasm and images, so it must be served, not opened as a file | node |
| `images` | character icons and card art into `webapp/img/{uma,outfit,card}/{id}.webp` (icons 96 px, cards 192 px: 2x their largest display) from the game's decrypted asset bundles; only new ids are extracted (`IMAGES_FORCE=1` redoes all); `img/manifest.json` lists each image's bundle and anything missing. `webapp/img/` is never committed (game art); a build without it shows initials | `$UMA_DECRYPTED` (default: the Steam/Proton install's `LocalLow/Cygames/Umamusume/decrypted`), already decrypted |
| `val TAG` | 24 validation jobs with their own settings plus the 23 deck jobs again with per-uma decks, run in parallel (`val/runval.py`; `VAL_J` sets how many at a time, default CPU count − 2), compared with `val/ref__*` and `val/refpu__*`; about 3.5 min, set by the three slow shared three-result jobs | native |
| `valq TAG` | the same without those three jobs (`val/slow.json`), about 40 s; for iterating on a change. The smaller three-result jobs still exercise the same code paths at small scale | native |

Browser checks: `./build.sh test` runs `webapp/nodebrowser.mjs` (Node Playwright, Chromium and Firefox, ~45 s; it serves `dist/` on a free local port, or tests `LLP_URL`): first search and odds, auto-search and restart, roster, card picker, panels, borrowed card, popups, phone layout, GM era, WCAG AAA contrast. `node webapp/nodebrowser.mjs chromium roster` runs a subset. Run it after any page change. WASM timings: `node nodebench.mjs loopopt.opt.wasm global pal_A2 pal_U pal_UF pal_F`.

Manual card entries (pal and group cards, written by hand; the build script decodes only SR/SSR chains): Global ones live in `pkg/loopopt/card_events.json` (every entry except the generated ones, `src` exactly "gametora training_events", survives `events`), GM ones in `jp/export/manual_gm*.json`.

Speeding up the solver: follow `docs/PERF.md` (protocol, measurement, pitfalls); results must stay byte-identical.

`val/ref__*` holds the current reference outputs; when a data change moves results on purpose, rerun `val`, check the diff, then copy the new outputs over `ref__*` in the same commit. Releases are tagged `vN` to match the published page version.

Zips (used before the repo moved to GitHub) are a clone of this repository (history and tags included) plus the built `Legacy Loop Planner.html`.
Before zipping: clone, `git remote remove origin`, `git gc --aggressive`, copy in the built page.

Exactness checks against brute force on small owned sets: `uv run val/owncheck.py BIN 3 "Yaeno Muteki"` (character rule, shared deck) and `uv run val/permcheck.py BIN 3` (per-uma decks; `permcheck_big.py` is the 8-uma, 10-card version, ~25 min).

Audit the manual entries against GameTora after any change to them or to the GameTora caches: `uv run val/event_audit.py` (exit 1 lists every target hint a pal or group card can give that its entry does not account for).

When every value is expected to scale by one factor (a constant change), compare a run with `uv run val/cmp_ratio.py TAG ref` instead of `cmp2.py`.
