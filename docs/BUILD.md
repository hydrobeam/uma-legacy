# Building, testing and deploying

Every step goes through `./build.sh [step...]` at the repository root.

## Quick start

A fresh clone needs two commands:

```
./build.sh            # the solver and its tests, the wasm, the art (with game files), the page (webapp/dist)
./build.sh serve      # http://localhost:8000 (WebAssembly needs the page served, not opened as a file)
```

The first `./build.sh` installs what it needs: rustup fetches the pinned Rust toolchain, `npm ci` the npm packages
into `webapp/node_modules`, uv the Python environment into `.venv/`. With the game files in `game/` (below) it
extracts the art too; without them it says so and the page shows initials. To check a change, `./build.sh testq`
(the browser suite's fast tier; it installs its browsers the first time) and `./build.sh valq TAG` (the validation
jobs against their reference outputs).

A fresh clone builds the page from the tracked data files and passes the solver tests, validation and the browser
suite. Without game art the page shows initials in place of pictures and a spinner in place of the runner, and the
browser suite skips the checks of the art itself (it says which).

## What is needed

Everything a build reads lives inside the repository folder, apart from the tools below and their own stores
(rustup's toolchains, Cargo's crate downloads, uv's Python and cache, npm's cache).

**Tools you install**
- Rust, via rustup: `rust-toolchain.toml` pins 1.98.1 with `rust-src` and the `wasm32-unknown-unknown` target.
- Node 22 or newer, with npm (`engines` in `webapp/package.json`; CI reads it from there).
- [uv](https://docs.astral.sh/uv/) (it provides Python 3.14).
- bash, git; tar with zstd for `runners` only.

**Libraries, installed by the build**

| Where | What | Used by |
|---|---|---|
| `pkg/loopopt/Cargo.toml` | `serde`, `serde_json` | the solver |
| `webapp/package.json` (`npm ci` on first use) | `binaryen` (`wasm-opt`), `esbuild`, `@fontsource/*` (4 fonts), `three`, `playwright` (pinned), `wrangler`, `@biomejs/biome` | `wasm`, `page`, `runners`, the tests, `deploy`, `lint` |
| `pyproject.toml` (`.venv/`, created by uv) | `UnityPy`, `Pillow` | `images`, `runners` (the exporters use the standard library) |
| `pyproject.toml`, group `dev` | `ruff` | `lint` |
| `pyproject.toml`, group `research` | `numpy`, `scipy` | `research` |
| `webapp/node_modules` (`./build.sh browsers`) | Playwright's Chromium and Firefox (`PLAYWRIGHT_BROWSERS_PATH=0`) | `test`, `testq`, `runners` |
| `cache/gametora/` (`./build.sh gametora`) | GameTora's data files at the versions pinned in `pkg/loopopt/data/gametora.json`. GameTora does not keep old versions online (6 of the 13 pins answered 404 on 2026-10-21), so the pinned set exists only in a cache that already holds it; elsewhere `GAMETORA_LATEST=1` moves the pins, and the events may change | `events` |
| `cache/chibi/tools/` (`runners`, first use) | AssetStudioModCLI 0.19.0, a .NET 9 runtime, FBX2glTF 0.9.7, libxml2.so.2 (Arch's libxml2-legacy; always this copy) | `runners` |

**Game files you supply**, all in `game/` (gitignored; needed only to refresh the data or make the art)

| File | What it is | Needed by |
|---|---|---|
| `game/global/master.mdb` | the Global client's game database (plain SQLite) | `data`, `events`, `runners` |
| `game/jp/master.mdb` | the JP client's game database (plain SQLite), for the Grand Masters era | `data`, `events` |
| `game/global/resources.assets` | the Global PC install's built-in assets file (the game's common buttons and check box) | `images` |
| `game/global/decrypted/` | the Global client's asset bundles, decrypted, laid out by asset path (`chara/chr1001/...`) | `images`, `runners` |
| `game/global/meta.db` | the Global client's asset index, decrypted to plain SQLite | `runners` |
| `game/jp/bundles/` | the JP client's bundles for JP-only art (the Grand Masters banner, 14 skill icons), by asset path | `images` |

The tool that decrypts the client's bundles and asset index is not published in this repository. Game files and
game art are never committed.

## Reproducing it

1. **The page, the solver and its checks** need only the tools above: `./build.sh`, then `./build.sh testq` (the checks
   of the art skip themselves without art) and `./build.sh valq TAG` (every job matches its reference).
   This runs on the tracked data files, which were exported from the Global game database of 2026-10-10 and the JP
   one of 2025-10-09.
2. **The data from the game:** with both `master.mdb` files in `game/`, `./build.sh data events pb`. With the same
   game databases the exports come out byte-identical, except `upcoming.json` (what starts after the build date) once
   a banner in it has started; a newer Global database adds its new umas, cards and races. Check with
   `uv run val/event_audit.py` and `./build.sh val TAG`.
3. **The art:** with the rest of `game/` in place, `./build.sh` extracts it.

## Steps

Each step but `hooks` and `research` checks its requirements first and, when something is missing, names every
missing tool or file and what it is before it stops; the `art` step instead says what it skipped.

| Step | Does |
|---|---|
| `all` | `data events native pb wasm art page` |
| `art` | `images` and `runners`, each when its game files are in `game/global/` (skipped with a note otherwise); part of the default build |
| `data` | the Global and Grand Masters-era exports from `master.mdb`: `base.json` (roster and aptitudes), `data.json`, `web_skills.json`, `web_cards.json` (with the scenario card bans), `career.json` (the veteran import's spark-factor and win-saddle tables), `upcoming.json` |
| `gametora` | GameTora's pinned data files into `cache/gametora/`; `GAMETORA_LATEST=1` moves the pins to the newest versions first; it also rewrites `pkg/loopopt/data/gametora_slugs.json`, the page's links to GameTora (tracked, so `page` downloads nothing). Run by `events` |
| `events` | each era's card event file from GameTora's files plus the hand-entered entries (pal first-year dates left out, as IT never fires them) |
| `native` | the release binary, clippy (as `lint` runs it), then `cargo test --release` |
| `lint` | `cargo fmt --check` and clippy with warnings denied, on every target and on the wasm lib (see *Rust style*); shellcheck on `build.sh` and the hook (`uvx` fetches it on first use); `ruff check` and `ruff format --check` on the Python (rules in `pyproject.toml`; ruff from the dev group, locked in `uv.lock`); `biome check` on the page's JS (lint and formatting in one pass: 2-space indent, single quotes, 120 columns) with warnings as errors (`webapp/biome.json`; reasoned `biome-ignore` comments where a rule is wrong for a line); TypeScript's checker on the page's plain JS (`webapp/tsconfig.json`: checkJs, not yet strict; the records are JSDoc typedefs) |
| `hooks` | `git config core.hooksPath .githooks`: the tracked pre-commit hook (see *Rust style*); once per clone |
| `pb` | each uma pair's exact affinity ceiling (`loopopt pbtable`, via `export/add_pb.py`) stored in both `data.json` files; it only speeds searches up. The table carries a fingerprint of the umas, goals and streak penalties; on a mismatch the solver ignores it and `cargo test` warns. Rerun after data or solver changes |
| `wasm` | the wasm32 build (`-Z build-std` through `RUSTC_BOOTSTRAP=1`), `wasm-opt -O3 -ocimfs=0` (no inlining of single-caller functions, so long loops stay in functions V8 optimises: `docs/PERF.md`, *In the page*), then `webapp/example.json` (the result shown before the first search). `wasm-opt` (about a minute) is skipped when the compiled wasm, the flags and wasm-opt's version match its last run (`webapp/.loopopt.wasm.key`); the example always runs |
| `page` | `webapp/dist/` (see *The page*) |
| `images` | game art into `webapp/img/` (see *Art*); only new ids unless `IMAGES_FORCE=1` |
| `runners` | each uma's running mini model as a sprite strip in `webapp/img/run/` (see *Art*) |
| `browsers` | Playwright's browsers into `webapp/node_modules` (once) |
| `test` / `testq` | the browser suite, full or its fast tier (see *Tests*) |
| `serve` | `webapp/dist` on http://localhost:8000 (`PORT=…`) |
| `val TAG` / `valq TAG` | validation (see *Tests*) |
| `deploy` | builds the solver (`wasm`) and the page, and uploads `webapp/dist` to Cloudflare Pages (see *Releases*) |
| `research` | `research/run_all.py` into `research/results.json`, then renders `docs/RESEARCH.md` from `docs/RESEARCH.template.md` (`research/README.md`) |

The solver's training-hint constants (each scenario's base and specialty weight, `pkg/loopopt/src/deck/hints.rs`) are
the research fit: `pkg/loopopt/build.rs` reads them from the tracked `research/results.json` (section 7's keys) at every
build, native or wasm, and rounds them to 3 decimals, so a `research` run that moves one at that precision moves the
solver's results (rerun validation, `pb`, and update `/about` and `docs/MODEL.md`, which state them by hand). The page's
scenario tiles take the bases the same way: `webapp/build.py` (`hint_bases`) reads build.rs's keys with its rounding
into the page's data. A missing key fails either build with its name.

## Data updates

Put the new `master.mdb` files in `game/`, then `./build.sh data events pb`, validation, and a release.

- `data` keeps every uma, outfit and support card that `master.mdb` dates up to 60 days past the build
  (`AHEAD` in `pkg/loopopt/export/era.py`; an outfit starts with its first gacha, else with its uma; content the game has
  not dated carries a 2050 placeholder and stays out). `pkg/loopopt/data/global/upcoming.json` lists what starts after the build.
  The page hides each entry until its start time and leaves it out of every search (`webapp/src/data.js` `LATER`,
  `engine.js`), so a banner turns on by itself; the precomputed example leaves the same entries out
  (`webapp/example.mjs`). Validation runs on everything in the data, upcoming included.
- Manual card entries (pal and group cards; the builder decodes only SR/SSR chains): each era's
  `pkg/loopopt/data/<era>/manual.json`, which `events` merges into that era's `card_events.json`. GameTora's files are
  read and decoded in one place, `pkg/loopopt/export/gametora.py` (its docstring holds their format); the builders turn
  events into card entries with `event_choices.py`, and `val/event_audit.py` and `val/secret_events.py` read them through
  it too. After touching them, `uv run val/event_audit.py` should say "every target hint is accounted for".
- New characters' art comes from `./build.sh images` (and `runners`); without it they show initials.

## Art

`images` writes `webapp/img/{kind}/{id}.avif` (AVIF q55, loaded lazily) and `img/manifest.json` (sources and gaps):
character icons, card art, skill icons, race banners, scenario logos and select art, mob portraits, and the UI
sprites as lossless WebP with dark versions made here (the game has no dark mode). The full list of kinds and the
bundles they come from is in `webapp/images.py`'s docstring. `uv run webapp/icons.py` renders the favicons and the
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
- favicons, `og.png`, font licences in `licenses/`, `_headers` (the host caches the pages 5 minutes, `assets/` a
  year), `_redirects` (`/loop` to `/`) and `robots.txt`. `serve.mjs` caches `assets/` a year as the host does and
  serves the pages uncached.

The solver runs as WebAssembly in a Web Worker (`worker.template.js`). Saved settings are one versioned object,
`localStorage['llp:settings']`, migrated in `settings.js`. Shared page state is two exported objects (`state`: the
settings, `ui`: what is on screen). No framework.

In `src/`, `main.js` and `career.js` are the two pages' entries; `engine.js` talks to the worker, `job.js` builds the
job both pages send, `save.js` decides what a settings change does, and `statestore.js` keeps the solver's work in
IndexedDB.

## Tests

- **Solver:** `cargo test --release` in `pkg/loopopt` (part of `native`); `./build.sh lint` for formatting and clippy.
- **Browser suite:** `./build.sh test` runs `webapp/nodebrowser.mjs` against `webapp/dist` (or `LLP_URL`) in Chromium
  and Firefox, test groups in parallel (`LLP_JOBS` per browser, default 4; `node webapp/nodebrowser.mjs chromium
  roster` runs one group; each group is a module in `webapp/test/`, shared helpers in `webapp/test/helpers.mjs`).
  `LLP_PROF=file` logs where the time goes. `./build.sh testq` is the fast tier (Chromium,
  each page's core path, about 10 s): run it after each page change, and the full suite before a release.
- **Validation:** `./build.sh val TAG` runs the 25 reference jobs in `val/jobs.json` (21 deck jobs, shared and per-uma
  decks, and 4 career jobs; every job leaves out the R cards that are not pals, as the page does) and compares them with `val/ref__*` / `val/refpu__*` (`val/cmp2.py`); `valq TAG` skips the
  three slow jobs in `val/slow.json`. When results move on purpose, check the diff and copy the new outputs over the
  references in the same commit; when every value should scale by one factor, compare with `val/cmp_ratio.py`.
  `uv run val/same.py TAG` compares the whole outputs instead (every field but the timings: the byte identity speed
  work keeps, `docs/PERF.md`). Solver process failures and comparison failures exit nonzero. `uv run val/test_validation.py` checks these failure
  paths with synthetic outputs and a stub solver, without running any searches.
- **Career planner:** `uv run val/planval.py pkg/loopopt/target/release/loopopt TAG` runs the ten plan jobs in
  `val/plan/` and compares each whole output with `val/plan/ref__*.out`; `val/mono.py BIN` checks that adding a veteran
  never raises the planner's estimate; `val/capaudit.py BIN` checks the exact caps.
- **Policy simulator:** `uv run val/simval.py pkg/loopopt/target/release/loopopt TAG` runs the five short simulator
  jobs in `val/sim/jobs.json` (each policy on `val/sim/pools/pool4.json`, about 6 s) and compares each whole output,
  wall clocks aside, with `val/sim/ref__*.out` (`val/same.py TAG --sim`).
- **Exactness:** `uv run val/owncheck.py BIN 3 "Yaeno Muteki"` and `uv run val/permcheck.py BIN 3` check the search
  against brute force on small owned sets; `uv run val/joint_check.py TAG` recomputes every deck result's odds
  independently. `uv run val/milp_fixtures.py` recomputes the exact DP's reference fixtures (`pkg/loopopt/tests/fixtures/`)
  with an independent MILP after a change to the win model (about 12 minutes, one process; `--check` only compares).
- **Speed work** follows `docs/PERF.md` (byte-identical outputs, one item per commit with its gain),
  which also holds the planner's speed record. `node webapp/nodebench.mjs <wasm> global pal_A2 …`
  (era `global` or `gm`; it reads its paths relative to `webapp/`) times the wasm.

## Continuous integration

`.github/workflows/ci.yml` runs on every push to main and every pull request, with the same `./build.sh` steps:
- **solver:** `lint`, `native` (release build, clippy, tests), then `valq ci` with `val/same.py ci`, `val/planval.py`
  with `val/same.py ci --plan` (every output byte-identical to the references, work counters included), `val/mono.py`
  and `val/simval.py` (the policy simulator's five jobs against `val/sim/ref__*`).
- **page:** `wasm page` (no game art in a clone: the page shows initials) and `testq` (Chromium); the checks that need
  the art skip themselves.

## Rust style

`./build.sh lint` passes before every Rust commit. Its Rust and shell checks, in `pkg/loopopt` (ruff and Biome follow;
see *Steps*):

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
uvx --from shellcheck-py shellcheck build.sh .githooks/pre-commit   # (from the repo root)
```

- **Formatting:** `pkg/loopopt/rustfmt.toml` (stable options: 120 columns, `use_small_heuristics = "Max"`); `cargo fmt`
  in `pkg/loopopt` applies it. `#[rustfmt::skip]` is kept for tables whose layout carries meaning, each with its reason.
- **Lints:** clippy's default groups, with the few that fight this code deliberately allowed in the `[lints.clippy]`
  table of `pkg/loopopt/Cargo.toml`, each with its reason, and a chosen set of pedantic, nursery and restriction lints
  denied there; an `#[allow]` in the code needs `reason = "..."`. `unsafe_code` is denied crate-wide and allowed only
  for the wasm exports (`web.rs`) and host imports (`career/clock.rs`), each unsafe block with a `// SAFETY:` note. A lint fix that could change floating-point evaluation
  order, NaN handling or iteration order is a result change: it lands only with byte-identical validation (the
  `same.py` checks in *Tests*), otherwise the lint is allowed locally with a reason. `./build.sh native` runs the
  clippy part before its tests.
- **Ownership and clones:** the borrow checker is a guide to the design, not an obstacle to get around. When it refuses,
  it is pointing at a real overlap (a value read while something else may change it); answer it by restructuring: borrow
  the fields a function needs separately, keep the read-only context behind `&` and the one mutable owner behind `&mut`,
  shorten a borrow's scope, or move a value instead of sharing it. A `.clone()`, `.to_vec()` or `.to_owned()` that
  exists only to make a conflict go away is a silent cost (outputs stay identical, only speed suffers) and is not
  accepted. A clone stays when the copy is the point (a snapshot kept while the original changes, a value handed to a
  longer-lived owner) and it is off the hot path; say so in a comment where that is not obvious. A change that adds
  clones lists each in its description with its reason.
- **Pre-commit hook:** `./build.sh hooks` (once per clone; it sets `core.hooksPath`, so it applies in every worktree)
  turns on `.githooks/pre-commit`, which checks what the commit stages, by kind: Rust, `cargo fmt --check` and native
  clippy with warnings denied (about 5 s after a change; a minute the first time in a new worktree); Python, ruff check
  and format on the staged files; the page's JS, Biome (lint and formatting) on the staged files and `tsc` (each under a second). It checks
  the working tree's files, not only the staged parts. The wasm clippy and the whole-repo checks are left to `lint`.
- **Blame:** commits that only reformat are listed in `.git-blame-ignore-revs`; to have `git blame` skip them, once
  per clone: `git config blame.ignoreRevsFile .git-blame-ignore-revs`.

## Code clarity

These rules apply to new and changed code in every language. Review readability as well as formatting and lints;
passing the tools does not establish that a reader can understand the code. Keep cleanup within the task's scope.

- **Names:** name values by their meaning, especially fields, parameters and return values. Prefer `pick_pool`,
  `is_runnable` and `incomplete` to unexplained abbreviations. Short loop indices and mathematical names are useful
  where their meaning is clear locally or defined in the model; do not expand them mechanically throughout a kernel.
- **Records:** use named fields for values that belong together, especially results with several indices, flags or
  different units. A tuple should be understandable without looking up its positional convention. Parallel buffers
  can be appropriate in a hot loop; explain their indexing invariant rather than wrapping them just for appearance.
- **Functions:** give each function one coherent operation or phase. A long workflow should show its phases in order,
  with inputs and results explicit. Split a long function only where a block is one computation with a short signature
  (about five arguments, and no record invented only to carry them into that call); otherwise keep it whole and mark its
  phases with heading comments (`// Phase 2: ...`). A split that needs a long argument list or a one-off record moves
  the complexity into the call and sends the reader elsewhere for no gain.
- **Dependencies:** values that travel together through several functions are one value: a trainee and her parents are
  one record, and the read-only state a family of functions shares (data, pool, legacies, score base) is one context
  struct, borrowed once, with documented fields, named in full where it is used. Keep what a function may change
  separate from what it reads: a read-only context, a `&mut` scratch or output struct, and small `Copy` options (as in a
  parser that takes `&mut Parser`, a `Cursor` and `ParseOpts`). Group by concept, not by call site, and prefer an
  existing domain record over another overlapping bundle. A struct unpacked into a dozen short locals on entry means the
  concept has not been named yet. A context is built once, where its contents come into being, and borrowed downward: a
  phase's entry point builds it and the functions below are its methods or take `&Context` (and the one mutable owner,
  such as `&mut Sim`) plus small values; call sites do not assemble context structs from loose locals. What changes
  during the phase (a score base refitted between phases) stays out of the long-lived context, which hands out the
  short-lived view through a method. Records store the records they are about (a row keeps its `lineage`), so calls pass
  `row.lineage` rather than rebuilding it.
- **Calls:** a reader checks a call against the signature without counting arguments. Adjacent arguments of one type
  that could be swapped unnoticed (`k.x, k.a, k.b`), or a long argument list (past five or so, as a guide, not a limit),
  usually mean a record is missing (a trainee and her parents; a scoring context). A function asked for one value
  returns one value, not a vector read at `[0]`. Pack and unpack bit fields through a named helper or type (an outcome's
  index from its star levels and fixed-target mask), and parenthesize any expression that mixes shifts with `|`, `&` or
  arithmetic. A call that spans several lines inside a closure (`(y < 0.5).then(|| …)`) is an `if`.
- **Files:** one module per concept: a reader should find a phase by its file name. A file past about 1500 lines
  holds several concepts and splits into a module folder; moving code between files changes no behaviour, so
  splits land as pure moves, apart from visibility, before anything is renamed.
- **Module folders:** a module with children is a folder whose root is `name/mod.rs` (never `name.rs` beside
  `name/`), so the folder is the whole module. Its `mod.rs` is the front door: its header maps the children, one line
  each, and it holds the module's main records and entry point, not the bulk of the work. A reader opens one file to
  learn where everything in the folder is.
- **Control flow:** keep the main path easy to follow; use guards for cases with nothing to do. Use iterators where
  they clarify the operation and index loops where indices, visit order or appending to the traversed vector matter.
  Do not add allocation, copying or repeated work merely to make an expression shorter.
- **Comments:** explain invariants, units, assumptions and reasons a reader cannot infer from the statements. Identify
  state effects that look like reads: a planner scoring can search for decks that later scorings use. Keep comments
  with the code they explain; do not leave session history or a restatement of the code as documentation.
- **Refactors:** preserve arithmetic and evaluation order, ties, cache/proof effects and resource lifetimes unless a
  behavior change is explicitly intended. Apply the output and work-counter contract in `docs/PERF.md`, and run the
  checks in *Tests*. A smaller function or a clean lint run is not evidence of equivalent behavior or better speed.
- **Checks in parallel work:** each branch (lane) of a batch worked in parallel runs the build, clippy and
  `cargo test`; the byte-identity validation (`val`, `val/planval.py`, `val/same.py`) runs on each integration batch
  once its lanes are merged, and once at the end. Validation takes minutes of CPU, and what it certifies is the
  merged tree: lanes that each pass can still interact once merged.

## Releases

`./build.sh native wasm page` (plus `data` / `events` when inputs changed), tests pass, full `val` on anything that
can move results, commit, tag `vN` (the next after the latest), `./build.sh deploy`. The page footer shows the
build's `git describe` version.

Hosting is Cloudflare Pages by direct upload: project `uma-legacy` (`CF_PAGES_PROJECT` overrides), live at
https://uma-legacy.pages.dev. `deploy` refuses when `webapp/`, `pkg/` or `research/results.json` have uncommitted changes
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
.githooks/           the pre-commit hook (`./build.sh hooks` turns it on)
.github/             CI (`workflows/ci.yml`)
webapp/              the page: templates, src/ (styles and modules), worker, build.py, the browser suite, chibi/
pkg/loopopt/         the Rust solver (src/, its module map in docs/SOLVER.md; tests/; build.rs takes the training-hint
                     constants from research/results.json)
pkg/loopopt/export/  Python exporters, one set for both eras (`global` or `jp` argument; era.py)
pkg/loopopt/data/    the solver's data: global/ and jp/ (the Grand Masters era: the JP game database cut at L'Arc's
                     launch), each with data.json, web_skills.json, web_cards.json, career.json, card_events.json,
                     base.json and manual.json (hand-entered card events), plus upcoming.json for Global; and the
                     files both eras share, event_shares.json, gametora.json (GameTora's pinned versions) and
                     gametora_slugs.json (the page's links to GameTora)
val/                 validation: jobs/ (the job files), jobs.json and slow.json (which run), reference outputs, checks;
                     plan/ and sim/ (the career planner's and the policy simulator's jobs and references)
research/            scripts computing RESEARCH.md's numbers; RESEARCH.md is rendered from docs/RESEARCH.template.md (`./build.sh research`; research/README.md)
docs/                USAGE, SOLVER, MODEL, RESEARCH, PERF and this file
game/                your game files (gitignored)
cache/               downloads: GameTora's files, the runner tools and exports (gitignored)
```
