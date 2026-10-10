# Building, testing and deploying

Every step goes through `./build.sh [step...]` at the repository root.

## Quick start

```
./build.sh            # native (the solver and its tests), wasm, page -> webapp/dist
./build.sh serve      # http://localhost:8000 (the page loads WebAssembly, so it must be served, not opened as a file)
./build.sh testq      # the browser suite's fast tier
./build.sh valq TAG   # the validation jobs against their reference outputs
```

A fresh clone builds the page from the tracked data files and passes the solver tests and validation. Without game
art the page shows initials in place of pictures and a spinner in place of the runner; without art the browser
suite's five art checks fail (banners, skill icons, swap-row card art, initials contrast, scenario tiles).

## What is needed

Everything a build reads lives inside the repository folder; nothing is looked up elsewhere on the machine.

**Tools you install**
- Rust, via rustup: `rust-toolchain.toml` pins 1.98.1 with `rust-src` and the `wasm32-unknown-unknown` target.
- Node with npm.
- [uv](https://docs.astral.sh/uv/) (it provides Python 3.12).
- bash, git; tar with zstd for `runners` only.

**Libraries, installed into the folder by the build**

| Where | What | Used by |
|---|---|---|
| `pkg/loopopt/Cargo.toml` | `serde`, `serde_json` | the solver |
| `webapp/package.json` (`npm ci` on first use) | `binaryen` (`wasm-opt`), `esbuild`, `@fontsource/*` (4 fonts), `three`, `playwright` (pinned), `wrangler` | `wasm`, `page`, `runners`, the tests, `deploy` |
| `pyproject.toml` (`.venv/`, created by uv) | `UnityPy`, `Pillow` | `images`, `runners` (the exporters use the standard library) |
| `webapp/node_modules` (`./build.sh browsers`) | Playwright's Chromium and Firefox (`PLAYWRIGHT_BROWSERS_PATH=0`) | `test`, `testq`, `runners` |
| `cache/gametora/` (`./build.sh gametora`) | GameTora's data files at the versions pinned in `pkg/loopopt/gametora.json` | `events`, `page` |
| `cache/chibi/tools/` (`runners`, first use) | AssetStudioModCLI 0.19.0, a .NET 9 runtime, FBX2glTF 0.9.7, libxml2.so.2 (Arch's libxml2-legacy; always this copy) | `runners` |

**Game files you supply**, all in `game/` (gitignored; needed only to refresh the data or make the art)

| File | What it is | Needed by |
|---|---|---|
| `game/global/master.mdb` | the Global client's game database (plain SQLite) | `data`, `events` |
| `game/jp/master.mdb` | the JP client's game database (plain SQLite), for the Grand Masters era | `data`, `events` |
| `game/global/resources.assets` | the Global PC install's built-in assets file (the game's common buttons and check box) | `images` |
| `game/global/decrypted/` | the Global client's asset bundles, decrypted, laid out by asset path (`chara/chr1001/...`) | `images`, `runners` |
| `game/global/meta.db` | the Global client's asset index, decrypted to plain SQLite | `runners` |
| `game/jp/bundles/` | the JP client's bundles for JP-only art (the Grand Masters banner, 14 skill icons), by asset path | `images` |

The tool that decrypts the client's bundles and asset index is not published in this repository. Game files and
game art are never committed.

## Reproducing it

1. **The page, the solver and its checks** need only the tools above: `./build.sh`, then `./build.sh testq` (71 checks;
   the five art checks fail without art) and `./build.sh valq TAG` (every job matches its reference). This runs on
   the tracked data files, which were exported from the Global game database of 2026-10-10 and the JP one of
   2025-10-09.
2. **The data from the game:** with both `master.mdb` files in `game/`, `./build.sh data events pb`. With the same
   game databases the exports come out byte-identical, except `upcoming.json` (what starts after the build date) once
   a banner in it has started; a newer Global database adds its new umas, cards and races. Check with
   `uv run val/event_audit.py` and `./build.sh val TAG`.
3. **The art:** with the rest of `game/` in place, `./build.sh images runners page`.

## Steps

| Step | Does |
|---|---|
| `all` | `data events native pb wasm page` |
| `data` | the Global and Grand Masters-era exports from `master.mdb`: `data.json`, `web_skills.json`, `web_cards.json` (with the scenario card bans), `career.json` (the veteran import's spark-factor and win-saddle tables), `upcoming.json` |
| `gametora` | GameTora's pinned data files into `cache/gametora/`; `GAMETORA_LATEST=1` moves the pins to the newest versions first. Run by `events` and `page` |
| `events` | the card event files from GameTora's files plus the manual entries (Global, Grand Masters, Grand Masters without first-year dates) |
| `native` | the release binary and `cargo test --release` |
| `pb` | each uma pair's exact affinity ceiling (`loopopt pbtable`, via `export/add_pb.py`) stored in both `data.json` files; it only speeds searches up. The table carries a fingerprint of the umas, goals and streak penalties; on a mismatch the solver ignores it and `cargo test` warns. Rerun after data or solver changes |
| `wasm` | the wasm32 build (`-Z build-std` through `RUSTC_BOOTSTRAP=1`), `wasm-opt -O3`, then `webapp/example.json` (the result shown before the first search) |
| `page` | `webapp/dist/` (see *The page*) |
| `images` | game art into `webapp/img/` (see *Art*); only new ids unless `IMAGES_FORCE=1` |
| `runners` | each uma's running mini model as a sprite strip in `webapp/img/run/` (see *Art*) |
| `browsers` | Playwright's browsers into `webapp/node_modules` (once) |
| `test` / `testq` | the browser suite, full or its fast tier (see *Tests*) |
| `serve` | `webapp/dist` on http://localhost:8000 (`PORT=…`) |
| `val TAG` / `valq TAG` | validation (see *Tests*) |
| `deploy` | builds the page and uploads `webapp/dist` to Cloudflare Pages (see *Releases*) |

## Data updates

Put the new `master.mdb` files in `game/`, then `./build.sh data events pb`, validation, and a release.

- `data` keeps every uma, outfit and support card that `master.mdb` dates up to 60 days past the build
  (`pkg/loopopt/export/upcoming.py`; an outfit starts with its first gacha, else with its uma; content the game has
  not dated carries a 2050 placeholder and stays out). `pkg/loopopt/upcoming.json` lists what starts after the build.
  The page hides each entry until its start time and leaves it out of every search (`webapp/src/data.js` `LATER`,
  `engine.js`), so a banner turns on by itself; the precomputed example leaves the same entries out
  (`webapp/example.mjs`). Validation runs on everything in the data, upcoming included.
- Manual card entries (pal and group cards; the builder decodes only SR/SSR chains): Global in
  `pkg/loopopt/card_events.json` (entries whose `src` is not exactly "gametora training_events" survive `events`), Grand
  Masters in `jp/export/manual_gm*.json`. The builders share one decoder, `pkg/loopopt/export/event_choices.py`. After
  touching them, `uv run val/event_audit.py` should say "every target hint is accounted for".
- New characters' art comes from `./build.sh images` (and `runners`); without it they show initials.

## Art

`images` writes `webapp/img/{kind}/{id}.avif` (AVIF q55, loaded lazily) and `img/manifest.json` (sources and gaps):
character icons, card art, skill icons, race banners, scenario logos and select art, mob portraits, and the UI
sprites as lossless WebP with dark versions made here (the game has no dark mode). The full list of kinds and the
bundles they come from is in `webapp/images.py`'s docstring. The Global client indexes some content before Global
releases it, which is why Grand Masters-era art exists locally. `uv run webapp/icons.py` renders the favicons and the
link-preview image from `webapp/icons/favicon.svg`.

`runners` writes one sprite strip per uma to `webapp/img/run/{chara}.avif`, the uma being planned running while a
search works (a spinner when a build has none). How the pipeline works and what to check when it breaks:
`webapp/chibi/README.md`.

## The page

`webapp/build.py` assembles `webapp/dist/`:
- `index.html` (the loop planner), `career.html` (`/career`) and `about.html` (`/about`, "How this works", which holds
  the model text), from their templates. Shared markup lives in `panels.template.html`, blocks marked
  `<!--panel:name-->` … `<!--/panel:name-->` that fill each `__PANEL_NAME__`. Both planner pages carry `class="game"`
  on `<html>`, which scopes the game-style rules in `style.css` (career-only layout under `.rb`, the loop planner's
  under `.lp`).
- `assets/`: every shared file named by a fingerprint of its content and cached for a year: the stylesheet (with the
  font faces and UI sprites as CSS variables), the shared data script (`self.__LLP_DATA`), the page bundles, the wasm,
  each era's solver data, the Grand Masters page data (loaded when that era is chosen), the fonts and `img.<hash>/`.
  A revisit fetches only the HTML (about 33 KB).
- favicons, `og.png`, font licences in `licenses/`, `_headers` and `robots.txt`. `serve.mjs` serves with the same
  cache rules as the host.

The solver runs as WebAssembly in a Web Worker (`worker.template.js`). Saved settings are one versioned object,
`localStorage['llp:settings']`, migrated in `settings.js`. Shared page state is two exported objects (`state`: the
settings, `ui`: what is on screen). No framework.

`src/` at a glance: `main.js` (entry), `settings.js`, `state.js`, `data.js`, `search.js` (auto-search), `save.js` (what
a settings change does), `engine.js` (the worker), `job.js` (the job fields both pages send), `setup.js` (the shared
settings editors), `roster.js`, `targets.js`, `scenario.js`, `cards.js`, `results.js`, `popover.js`, `agenda.js` (race
schedule), `affinity.js`, `afftable.js`, `odds.js`, `io.js` (export and import), `images.js`, `sortmenu.js`, `dom.js`,
`gmtip.js` (tooltips); the career page: `career.js`, `vets.js` (veterans), `details.js` (the veteran card),
`statestore.js` (the solver's kept work in IndexedDB), `umadump.js` (import).

## Tests

- **Solver:** `cargo test --release` in `pkg/loopopt` (part of `native`).
- **Browser suite:** `./build.sh test` runs `webapp/nodebrowser.mjs` against `webapp/dist` (or `LLP_URL`) in Chromium
  and Firefox, test groups in parallel (`LLP_JOBS` per browser, default 4; `node webapp/nodebrowser.mjs chromium
  roster` runs one group). `LLP_PROF=file` logs where the time goes. `./build.sh testq` is the fast tier (Chromium,
  each page's core path, about 10 s): run it after each page change, and the full suite before a release.
- **Validation:** `./build.sh val TAG` runs the 24 reference jobs in `val/jobs.json` (20 deck jobs, shared and per-uma
  decks, and 4 career jobs) and compares them with `val/ref__*` / `val/refpu__*` (`val/cmp2.py`); `valq TAG` skips the
  three slow jobs in `val/slow.json`. When results move on purpose, check the diff and copy the new outputs over the
  references in the same commit; when every value should scale by one factor, compare with `val/cmp_ratio.py`.
- **Career planner:** `uv run val/planval.py pkg/loopopt/target/release/loopopt TAG` runs the seven plan jobs in
  `val/plan/` and compares each whole output with `val/plan/ref__*.out`; `val/mono.py` checks that adding a veteran
  never raises the planner's estimate; `val/capaudit.py` checks the exact caps.
- **Exactness:** `uv run val/owncheck.py BIN 3 "Yaeno Muteki"` and `uv run val/permcheck.py BIN 3` check the search
  against brute force on small owned sets; `uv run val/joint_check.py TAG` recomputes every deck result's odds
  independently.
- **Speed work** follows `docs/PERF.md` (byte-identical outputs, one item per commit with its gain);
  `docs/PERF_PLANNER.md` holds the planner's measurements. `node webapp/nodebench.mjs <wasm> global pal_A2 …` times
  the wasm.

## Releases

`./build.sh native wasm page` (plus `data` / `events` when inputs changed), tests pass, full `val` on anything that
can move results, commit, tag `vN` (the next after the latest), `./build.sh deploy`. The page footer shows the
build's `git describe` version.

Hosting is Cloudflare Pages by direct upload: project `uma-legacy` (`CF_PAGES_PROJECT` overrides), live at
https://uma-legacy.pages.dev. `deploy` refuses when `webapp/`, `pkg/` or `jp/` have uncommitted changes
(`DEPLOY_DIRTY=1` overrides); `CF_PAGES_BRANCH=preview` publishes to the preview address; `SITE_URL` sets the
link-preview tags' host. One-time setup: `cd webapp && npx wrangler login`, then `npx wrangler pages project create
uma-legacy --production-branch main --force` (`--force` keeps it on classic Pages; without it wrangler 4.147 delegated
to Workers-based Pages and failed).

Check a deploy on its own address (`https://<hash>.uma-legacy.pages.dev`, printed by wrangler), e.g.
`LLP_URL=https://<hash>.uma-legacy.pages.dev node webapp/nodebrowser.mjs quick`: the branch addresses cache HTML for 5
minutes and can serve the previous page until then.

## Layout

```
build.sh             the one entry point for every build step
webapp/              the page: templates, src/ (styles and modules), worker, build.py, the browser suite, chibi/
pkg/loopopt/         the Rust solver (src/, tests/), data.json, web_skills.json, web_cards.json, card_events.json, jobs/
pkg/loopopt/export/  Python exporters (master.mdb -> data files)
jp/                  Grand Masters-era data and exporters (JP master.mdb, cut at L'Arc's launch)
val/                 validation jobs, reference outputs and checks
docs/                USAGE, SOLVER, MODEL, RESEARCH, SOURCES, PERF, PERF_PLANNER, SECRET_EVENTS (made by
                     val/secret_events.py) and this file
game/                your game files (gitignored)
cache/               downloads: GameTora's files, the runner tools and exports (gitignored)
```
