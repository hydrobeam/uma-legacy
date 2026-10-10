# Building, testing and deploying

Everything goes through `./build.sh [step...]` at the repository root; no step means `native wasm page`. Python runs
in the project's uv environment (`pyproject.toml`, `uv.lock`; `.venv/` is local): `uv run path/to/script.py`, never
the system Python. Node tooling lives in `webapp/node_modules` (`npm ci` on first use).

## Toolchain and a fresh checkout

- Rust stable with `rust-src`: the wasm build uses `-Z build-std` via `RUSTC_BOOTSTRAP=1` (with the
  `wasm32-unknown-unknown` target installed it also works without build-std).
- Node + npm: `cd webapp && npm ci` (binaryen's `wasm-opt`, esbuild, wrangler, Playwright).
- uv: the Python environment (UnityPy and Pillow for the image step; everything else is stdlib). Playwright with
  Chromium and Firefox for the browser suite.
- Data: the Global `master.mdb` in `pkg/loopopt/export/`, the JP one in `jp/export/` (sources: `docs/SOURCES.md`).
- Check: `./build.sh all` (data, events, native with its tests, pb, wasm, page), `uv run val/event_audit.py` ("every
  target hint is accounted for"), `./build.sh val TAG` (everything should match the references).

## Steps

| Step | Does | Needs |
|---|---|---|
| `all` | `data events native pb wasm page` | everything below |
| `data` | Global and Grand Masters-era exports (`data.json`, `web_skills.json`, `web_cards.json` with the scenario card bans, `career.json`: the veteran import's spark-factor and win-saddle tables) | `pkg/loopopt/export/master.mdb`, `jp/export/master.mdb` |
| `events` | card event caches from `pkg/loopopt/gametora/` plus the manual entries (Global, GM, GM without first-year dates) | Global master.mdb |
| `native` | release binary and its tests | cargo |
| `pb` | exact pairwise affinity ceilings of every uma pair, stored as `pb` in both `data.json` (`export/add_pb.py` via `loopopt pbtable`); a typical search runs faster with it. It carries a fingerprint of the umas, goals and streak penalties; on a mismatch the solver ignores it and `cargo test` warns until `pb` is rerun | native |
| `wasm` | wasm32 build (build-std), `wasm-opt -O3`, then `example.json` (the result shown before the first search) | rust-src, node |
| `page` | `webapp/dist/` (see below) | node |
| `images` | game art into `webapp/img/{kind}/{id}.avif`: character icons, card art, skill icons, race banners, scenario logos, UI sprites (lossless WebP: the friend frame, card type icons, rank badges, and the career page's veteran card: aptitude grade letters `atlas/common utx_ico_statusrank_00..07`, spark bars and bullets `atlas/singlecommon utx_frm_factor_list_01..04_sl` / `utx_ico_factor_01..02`, skill tiles `atlas/common utx_btn_skill_base_00..03_sl`, the tab pill `utx_btn_square_00_sl`; the Enhance-menu buttons `home/ui/texture utx_btn_home_umamusume_00..02`; the game's buttons and check box from the install's `resources.assets`: `utx_btn_s_00..03_sl`, `utx_btn_checkbox_00..01`), dark-mode versions of the light ones (`*_dark`, made here: the game has no dark mode), and the scenario select art (`single/banner/bnr_chara_big_NNN`, kind `scenart`), and every named mob uma's portrait (`mob/mob_chr_icon_ID_*`, kind `mob`: the career planner's sample veterans); only new ids unless `IMAGES_FORCE=1`; `img/manifest.json` lists sources and gaps. Never committed (© Cygames); without it the page shows initials | `$UMA_DECRYPTED` (default: the Steam/Proton install's `LocalLow/Cygames/Umamusume/decrypted`), `$UMA_GAME_DATA` (default: the Steam install's `UmamusumePrettyDerby_Data`) |
| `serve` | serves `webapp/dist` on http://localhost:8000 (`PORT=…`). The page fetches its solver and images, so it must be served, not opened as a file | node |
| `test` | the browser suite (below) | node, Playwright |
| `testq` | the browser suite's fast tier (below) | node, Playwright |
| `deploy` | builds the page, uploads `webapp/dist` to Cloudflare Pages with wrangler (project `uma-legacy` unless `CF_PAGES_PROJECT`; `CF_PAGES_BRANCH=preview` publishes to the preview URL; `SITE_URL` sets the link-preview tags' host). Refuses when `webapp/`, `pkg/` or `jp/` have uncommitted changes (`DEPLOY_DIRTY=1` overrides). One-time setup: `cd webapp && npx wrangler login`, then `npx wrangler pages project create uma-legacy --production-branch main` (plus `--force` to stay on classic Pages, see *Releases*) | node, a Cloudflare login |
| `val TAG` / `valq TAG` | validation (below) | native |

`uv run webapp/icons.py` renders the favicons and the link-preview image from `webapp/icons/favicon.svg` (committed).

Images: character icons are keyed by dress id (`card_rarity_data.race_dress_id`), not outfit id. Global's client
pre-indexes upcoming content (its `meta` lists the 55 GM-era cards Global has not released), which is why GM-era art
exists locally. Images are AVIF q55 (the owner's pick from a side-by-side comparison; half the size of WebP q82) and load
lazily.

## The page (`webapp/`)

`build.py` assembles `webapp/dist/`:
- `index.html`: `index.template.html` with the example result, the asset table, the worker source and the logo filled
  in; everything shared and large is a fingerprinted file in `assets/` (below), so a revisit or a move between pages
  fetches only the HTML (about 33 KB raw). Short cache.
- `about.html` ("How this works", served at `/about`): `about.template.html` with the same stylesheet (plus its math
  font face inline); it holds the page's model text.
- `career.html` (the career planner, served at `/career`): `career.template.html` with the same stylesheet, data and
  worker, its own bundle (`src/career.js`) and its own data script: the era's `career.json` tables and each G1's turns
  (`race_turns`, from `data.json`, for the G1-wins calendar).
- `assets/`: every file named with a fingerprint of its content and cached for a year (an update gets new names):
  `style.<hash>.css` (`src/style.css` minified, the font faces and the UI sprites as CSS variables, `--btn-w` etc.,
  with paths relative to `assets/`: a `url()` in a custom property resolves against the stylesheet that uses it);
  `data-global.<hash>.js` (the planner pages' shared data: Global cards, umas, skills, image keys, skill icons, race
  ids, set as `self.__LLP_DATA` under the ids of the elements that once held it) and `career-data.<hash>.js`; the
  bundles `main.<hash>.js` and `career.<hash>.js` (each page preloads its scripts from the head and runs them at the
  end of the body, data first); the wasm, each era's solver data (preloaded by the page head), the GM era's page data (fetched when that era is
  chosen), the fonts (Latin subsets from the Fontsource packages: Zen Kaku Gothic New, IBM Plex Mono, and Zen Maru Gothic for the planner pages' game style) and `img.<hash>/` (copied from `webapp/img`).
- Favicons and `og.png` at the root, font licences in `licenses/`, `_headers` (the caches) and `robots.txt`.
  `serve.mjs` serves `assets/` with the same one-year immutable cache and the pages with `no-cache`, as the host does.

Page state: saved settings are one object, `localStorage['llp:settings']` (`{v, era, common, global, gm}`, versioned
and migrated in `settings.js`). Shared mutable page state lives in two exported objects (`state`: settings, `ui`: what
is on screen), because an importer cannot reassign another module's `let`; module cycles (search -> results ->
popover -> search) are runtime-only calls and safe. No framework, by choice: the page's problems were layout, not
architecture.

The solver runs as WebAssembly in a Web Worker (`worker.template.js`); a browser that refuses WebAssembly gets a
message saying so. Source map of `src/`: `main.js` (entry), `settings.js` (saved settings), `state.js`, `data.js`,
`search.js` (auto-search), `save.js` (what a settings change does), `engine.js` (the worker), `job.js` (job fields both pages send), `roster.js`, `targets.js`, `scenario.js`, `cards.js`, `results.js`, `popover.js`
(card and member panels), `agenda.js` (race schedule), `affinity.js`, `io.js` (export/import), `images.js`,
`sortmenu.js`, `dom.js`, `setup.js` (the settings editors both pages share), `afftable.js` (a career's affinity, legacy by
legacy), `odds.js` (the odds popup), `gmtip.js` (the game-style tooltips); the career page: `career.js` (entry, search,
results), `vets.js` (veterans and the tree editor), `details.js` (the veteran card, the game's Umamusume Details
screen), `statestore.js` (the solver's kept work in IndexedDB), `umadump.js` (import).

## Tests

- **Browser suite**: `./build.sh test` runs `webapp/nodebrowser.mjs` against `webapp/dist` (or `LLP_URL`), Chromium and
  Firefox at once with test groups in parallel (`LLP_JOBS` sets how many per browser, default 4; `node
  webapp/nodebrowser.mjs chromium roster` runs a subset). The career page's checks are four groups (`career`,
  `careerpool`, `careervets`, `careergen`), each from its own import of the umadump test file, so they run side by
  side; the longest groups start first. About 100 s on the owner's laptop (2026-10-09, shared with other jobs; it was
  about 230 s before the split). Add checks for new behaviour (not for removals).
- **Fast tier**: `./build.sh testq` (`node webapp/nodebrowser.mjs quick`, Chromium only unless a browser is named)
  runs each page's core path in about 10 seconds: the planner loads and its search returns, the result's dialogs
  (odds, schedule, affinity), the phone layout and its dialogs, the contrast audit in light mode, /about with its
  audit, and the career page up to its one-run answer (import, search, plan, the veterans dialog). Run it after each
  page change while iterating; run the full `./build.sh test` before committing a page change that is done, and for
  every release.
- **Where the time goes**: `LLP_PROF=file` appends one JSON line per group (new contexts, page loads, waits for
  searches and plans, fixed sleeps, contrast audits, the solver's own time per job kind) to `file`.
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
  The career planner's next-plan jobs are outside `val`: `uv run val/planval.py pkg/loopopt/target/release/loopopt TAG`
  runs the seven in `val/plan/` (p16, p16_ss, p16_ura, p10, p11, gm16, boot_mob) and compares each whole output with
  `val/plan/ref__*.out`, printing the plan's work (nodes, exact scorings) beside the reference's; `val/mono.py` checks
  that adding a veteran never raises the planner's V; `docs/PERF_PLANNER.md` holds the planner's measurements.
  `node webapp/nodebench.mjs <wasm> global pal_A2 …` times the wasm.

## Data

  - `data` keeps every uma, outfit and support card the master dates up to 60 days past the build
    (`pkg/loopopt/export/upcoming.py`; an outfit's start is its first gacha's, else its uma's start date; undated
    content carries a 2050 placeholder and stays out) and writes `pkg/loopopt/upcoming.json`, what starts after the
    build. The page hides each until its start time and leaves it out of every search (`webapp/src/data.js` `LATER`,
    `engine.js`), so a banner turns on by itself: with the next update's master in place, rebuild and deploy any
    time before its start. The precomputed example leaves the same entries out (`webapp/example.mjs`).
    Validation runs on everything in the data, upcoming included.
  - New characters' art still comes from the game install (`./build.sh images`); without it the page shows initials.
- Manual card entries (pal and group cards; the builder decodes only SR/SSR chains): Global in
  `pkg/loopopt/card_events.json` (entries whose `src` is not exactly "gametora training_events" survive `events`),
  GM in `jp/export/manual_gm*.json`. The card-event builders share one decoder, `pkg/loopopt/export/event_choices.py`.

## Releases

`./build.sh native wasm page` (plus `data` / `events` when inputs changed), tests pass, full `val` on anything that
can move results, commit, tag `vN` (the next after the latest tag), `./build.sh deploy`. The page footer shows the
build's `git describe` version.

Hosting: Cloudflare Pages by direct upload, project `uma-legacy`, live at https://uma-legacy.pages.dev (canonical;
indexing allowed since 2026-10-05). The project is on classic Pages: it was created with `wrangler pages project
create --force` because wrangler 4.147 otherwise delegated to the Workers-based Pages and failed; deploys need no flag.
Classic Pages gets no new features; moving to Workers static assets is a `wrangler.jsonc` away but changes the address.

Verify a deploy against its own address (the `https://<hash>.uma-legacy.pages.dev` wrangler prints), e.g.
`LLP_URL=https://<hash>.uma-legacy.pages.dev node webapp/nodebrowser.mjs quick`. The branch addresses
(`preview.…`, the main one) serve HTML with a 5-minute cache, so for up to 5 minutes after a deploy they can hand out
the previous page and its scripts: checks run against them then fail with the old behaviour (v42's preview check did).

## Layout

```
webapp/          the page: templates, src/ (styles and modules), worker, build.py, the browser suite
pkg/loopopt/     Rust solver (src/), data.json, web_skills.json, web_cards.json, card_events.json, jobs, tests
pkg/loopopt/export/  Python exporters (master.mdb -> data files)
jp/              Grand Masters-era data and exporters (JP master.mdb, cut at L'Arc's launch)
val/             validation jobs, reference outputs and checks
docs/            the documentation (this file, USAGE, SOLVER, MODEL, PERF, PERF_PLANNER, SOURCES, the evidence notes
                 CHAIN_EVENTS, HINT_RATE_FIT, RANDOM_EVENTS, SECRET_EVENTS (generated by val/secret_events.py), HANDOFF)
inputs/          reference material handed over by hand
build.sh         the one entry point for every build step
```
