# Building, testing and deploying

Everything goes through `./build.sh [step...]` at the repository root; no step means `native wasm page`. Python runs
in the project's uv environment (`pyproject.toml`, `uv.lock`; `.venv/` is local): `uv run path/to/script.py`, never
the system Python. Node tooling lives in `webapp/node_modules` (`npm ci` on first use).

## Steps

| Step | Does | Needs |
|---|---|---|
| `data` | Global and Grand Masters-era exports (`data.json`, `web_skills.json`, `web_cards.json` with the scenario card bans, `career.json`: the veteran import's spark-factor and win-saddle tables) | `pkg/loopopt/export/master.mdb`, `jp/export/master.mdb` |
| `events` | card event caches from `pkg/loopopt/gametora/` plus the manual entries (Global, GM, GM without first-year dates) | Global master.mdb |
| `native` | release binary and its tests | cargo |
| `pb` | exact pairwise affinity ceilings of every uma pair, stored as `pb` in both `data.json` (`export/add_pb.py` via `loopopt pbtable`); a typical search runs faster with it. It carries a fingerprint of the umas, goals and streak penalties; on a mismatch the solver ignores it and `cargo test` warns until `pb` is rerun | native |
| `wasm` | wasm32 build (build-std), `wasm-opt -O3`, then `example.json` (the result shown before the first search) | rust-src, node |
| `page` | `webapp/dist/` (see below) | node |
| `images` | game art into `webapp/img/{kind}/{id}.avif`: character icons, card art, skill icons, race banners, scenario logos, UI sprites (lossless WebP: the friend frame, card type icons, rank badges, and the career page's veteran card: aptitude grade letters `atlas/common utx_ico_statusrank_00..07`, spark bars and bullets `atlas/singlecommon utx_frm_factor_list_01..04_sl` / `utx_ico_factor_01..02`, skill tiles `atlas/common utx_btn_skill_base_00..03_sl`, the tab pill `utx_btn_square_00_sl`; the Enhance-menu buttons `home/ui/texture utx_btn_home_umamusume_00..02`; the game's buttons and check box from the install's `resources.assets`: `utx_btn_s_00..03_sl`, `utx_btn_checkbox_00..01`), dark-mode versions of the light ones (`*_dark`, made here: the game has no dark mode), and the scenario select art (`single/banner/bnr_chara_big_NNN`, kind `scenart`); only new ids unless `IMAGES_FORCE=1`; `img/manifest.json` lists sources and gaps. Never committed (© Cygames); without it the page shows initials | `$UMA_DECRYPTED` (default: the Steam/Proton install's `LocalLow/Cygames/Umamusume/decrypted`), `$UMA_GAME_DATA` (default: the Steam install's `UmamusumePrettyDerby_Data`) |
| `serve` | serves `webapp/dist` on http://localhost:8000 (`PORT=…`). The page fetches its solver and images, so it must be served, not opened as a file | node |
| `test` | the browser suite (below) | node, Playwright |
| `deploy` | builds the page, uploads `webapp/dist` to Cloudflare Pages with wrangler (project `uma-legacy` unless `CF_PAGES_PROJECT`; `CF_PAGES_BRANCH=preview` publishes to the preview URL; `SITE_URL` sets the link-preview tags' host). Refuses when `webapp/`, `pkg/` or `jp/` have uncommitted changes (`DEPLOY_DIRTY=1` overrides). One-time setup: `cd webapp && npx wrangler login`, then `npx wrangler pages project create uma-legacy --production-branch main` | node, a Cloudflare login |
| `val TAG` / `valq TAG` | validation (below) | native |

`uv run webapp/icons.py` renders the favicons and the link-preview image from `webapp/icons/favicon.svg` (committed).

## The page (`webapp/`)

`build.py` assembles `webapp/dist/`:
- `index.html`: `index.template.html` with `src/style.css`, the `src/*.js` modules bundled by esbuild into one inline
  script, the Global page data, the example result and the logo filled in. Short cache.
- `about.html` ("How this works", served at `/about`): `about.template.html` with the same styles; it holds the
  page's model text.
- `career.html` (the career planner, served at `/career`): `career.template.html` with the same styles, data and
  worker, its own bundle (`src/career.js`), the era's `career.json` tables and each G1's turns (`race_turns`, from
  `data.json`, for the G1-wins calendar).
- `assets/`: every file named with a fingerprint of its content and cached for a year (an update gets new names):
  the wasm, each era's solver data (preloaded by the page head), the GM era's page data (fetched when that era is
  chosen), the fonts (Latin subsets from the Fontsource packages: Zen Kaku Gothic New, IBM Plex Mono, and Zen Maru Gothic for the planner pages' game style) and `img.<hash>/` (copied from `webapp/img`).
- Favicons and `og.png` at the root, font licences in `licenses/`, `_headers` (the caches) and `robots.txt`.

The solver runs as WebAssembly in a Web Worker (`worker.template.js`); a browser that refuses WebAssembly gets a
message saying so. Source map of `src/`: `main.js` (entry), `settings.js` (saved settings), `state.js`, `data.js`,
`search.js` (auto-search), `save.js` (what a settings change does), `engine.js` (the worker), `job.js` (job fields both pages send), `roster.js`, `targets.js`, `scenario.js`, `cards.js`, `results.js`, `popover.js`
(card and member panels), `agenda.js` (race schedule), `affinity.js`, `io.js` (export/import), `images.js`,
`sortmenu.js`, `dom.js`; the career page: `career.js` (entry, search, results), `vets.js` (veterans and the tree
editor), `umadump.js` (import).

## Tests

- **Browser suite**: `./build.sh test` runs `webapp/nodebrowser.mjs` against `webapp/dist` (or `LLP_URL`), Chromium and
  Firefox at once with test groups in parallel (`LLP_JOBS` sets how many; `node webapp/nodebrowser.mjs chromium
  roster` runs a subset). Run it after any page change, and add checks for new behaviour (not for removals).
- **Solver**: `cargo test --release` in `pkg/loopopt` (also part of `native`).
- **Validation**: `./build.sh val TAG` runs the reference setups twice (shared deck, per-uma decks) in parallel and
  compares them with `val/ref__*` / `val/refpu__*` (`val/cmp2.py`); `valq TAG` skips the slow shared three-result
  jobs (`val/slow.json`) for iterating. When results move on purpose, check the diff and copy the new outputs over the
  references in the same commit; when every value should scale by one factor, compare with `val/cmp_ratio.py`.
- **Exactness**: `uv run val/owncheck.py BIN 3 "Yaeno Muteki"` and `uv run val/permcheck.py BIN 3` check the search
  against brute force on small owned sets; `uv run val/joint_check.py TAG` recomputes every deck result's exact odds
  independently.
- **Card events**: after touching the manual entries or the GameTora caches, `uv run val/event_audit.py`.
- **Speed work** on the solver follows `docs/PERF.md`: byte-identical outputs, one item per commit with its gain.
  `node webapp/nodebench.mjs <wasm> global pal_A2 …` times the wasm.

## Data

- Game data update: put the Global `master.mdb` (from the game's local data folder) in `pkg/loopopt/export/` and the
  JP one in `jp/export/`, then `./build.sh data events pb`. `master.mdb` files are never committed.
- Manual card entries (pal and group cards; the builder decodes only SR/SSR chains): Global in
  `pkg/loopopt/card_events.json` (entries whose `src` is not exactly "gametora training_events" survive `events`),
  GM in `jp/export/manual_gm*.json`. The card-event builders share one decoder, `pkg/loopopt/export/event_choices.py`.

## Releases

`./build.sh native wasm page` (plus `data` / `events` when inputs changed), tests pass, full `val` on anything that
can move results, commit, tag `vN` (the next after the latest tag), `./build.sh deploy`. The page footer shows the
build's `git describe` version.

## Layout

```
webapp/          the page: templates, src/ (styles and modules), worker, build.py, the browser suite
pkg/loopopt/     Rust solver (src/), data.json, web_skills.json, web_cards.json, card_events.json, jobs, tests
pkg/loopopt/export/  Python exporters (master.mdb -> data files)
jp/              Grand Masters-era data and exporters (JP master.mdb, cut at L'Arc's launch)
val/             validation jobs, reference outputs and checks
docs/            the documentation (this file, USAGE, SOLVER, MODEL, evidence notes, HANDOFF)
inputs/          reference material handed over by hand
build.sh         the one entry point for every build step
```
