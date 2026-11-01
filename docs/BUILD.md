# Building, testing and deploying

Every step goes through `./build.sh [step...]` at the repository root.

## Quick start

A fresh clone needs two commands:

```
./build.sh            # the solver and its tests, the wasm, the art (with game files), the page (webapp/dist)
./build.sh serve      # https://localhost:8000 over HTTP/2 (WebAssembly needs the page served, not opened as a file)
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
- Rust, via rustup: `rust-toolchain.toml` pins 1.98.1 with the `wasm32-unknown-unknown` target.
- Node 22 or newer, with npm (`engines` in `webapp/package.json`; CI reads it from there).
- [uv](https://docs.astral.sh/uv/) (it provides Python 3.14).
- bash, git; tar with zstd for `runners` only.

**Libraries, installed by the build**

| Where | What | Used by |
|---|---|---|
| `pkg/loopopt/Cargo.toml` | `serde`, `serde_json` | the solver |
| `webapp/package.json` (`npm ci` on first use) | `binaryen` (`wasm-opt`), `esbuild`, `@fontsource/*` (4 fonts), `three`, `playwright` (pinned), `wrangler`, `@biomejs/biome`, `typescript` | `wasm`, `page`, `runners`, the tests, `deploy`, `lint` |
| `pyproject.toml` (`.venv/`, created by uv) | `fonttools`, `brotli` | `page` (/about's math font subset; the page budgets' transfer estimate); the exporters use the standard library |
| `pyproject.toml`, group `art` | `UnityPy`, `Pillow` | `images`, `runners`, `webapp/icons.py` |
| `pyproject.toml`, group `dev` | `ruff`, `shellcheck-py` | `lint` |
| `pyproject.toml`, group `research` | `numpy`, `scipy` | `research` |
| `cache/ms-playwright/` (`./build.sh browsers`) | Playwright's Chromium and Firefox (`PLAYWRIGHT_BROWSERS_PATH`; outside `node_modules`, so `npm ci` keeps them) | `test`, `testq`, `runners` |
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
| `game/global/bundles/` | Global bundles the client downloads on demand (the legacy list's veteran portraits, `trained_chr_icon_*`), by asset path | `images` |

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

`./build.sh help` lists the steps with one line each, from the table `STEPS` at the top of `build.sh`, which also
says which steps the default build and `all` run; this section has the detail. Each step but `hooks` and `research`
checks its requirements first and, when something is missing, names every missing tool or file and what it is before
it stops; the `art` step instead says what it skipped.

| Step | Does |
|---|---|
| `all` | `data events native pb wasm art page` |
| `art` | `images` and `runners`, each when its game files are in `game/global/` (skipped with a note otherwise); part of the default build |
| `data` | the Global and Grand Masters-era exports from `master.mdb`: `base.json` (roster and aptitudes), `data.json`, `web_skills.json`, `web_cards.json` (with the scenario card bans), `career.json` (the veteran import's spark-factor and win-saddle tables), `upcoming.json` |
| `gametora` | GameTora's pinned data files into `cache/gametora/`; `GAMETORA_LATEST=1` moves the pins to the newest versions first; it also rewrites `pkg/loopopt/data/gametora_slugs.json`, the page's links to GameTora (tracked, so `page` downloads nothing). Run by `events` |
| `events` | each era's card event file from GameTora's files plus the hand-entered entries (pal first-year dates left out, as IT never fires them) |
| `native` | the release binary, clippy (as `lint` runs it), then `cargo test --release` |
| `lint` | every check below runs whatever the others found, and the step ends by naming the ones that failed: `cargo fmt --check` and clippy with warnings denied, on every target and on the wasm lib (see *Rust style*); shellcheck on `build.sh` and the hook (`shellcheck-py`, dev group); `ruff check` and `ruff format --check` on the Python (rules in `pyproject.toml`; ruff from the dev group, locked in `uv.lock`); `val/test_validation.py` (planval's and the comparisons' gates on a fake solver, 3 s; it had gone four days failing unnoticed when nothing ran it); `biome check` on the page's JS (lint and formatting in one pass: 2-space indent, single quotes, 120 columns) with warnings as errors (`webapp/biome.json`; reasoned `biome-ignore` comments where a rule is wrong for a line); TypeScript's checker on the page's plain JS (`webapp/tsconfig.json`: checkJs, not yet strict; the records are JSDoc typedefs); `webapp/test/static.mjs`, what the browser suite can't see going stale: CSS classes no script, template or `build.py` names (a name built from parts is listed in its `BUILT` with where), exports no other file uses, and `localStorage` writes of a copy not read from storage after the function's last `await` (one tab's write then erases another's) |
| `hooks` | `git config core.hooksPath .githooks`: the tracked pre-commit hook (see *Rust style*); once per clone |
| `pb` | the release solver (cargo's build, a no-op on an unchanged tree), then each uma pair's exact affinity ceiling (`loopopt pbtable`, via `export/add_pb.py`) stored in both `data.json` files; it only speeds searches up. The table carries a fingerprint of the umas, goals and streak penalties; on a mismatch the solver ignores it and `cargo test` warns. Rerun after data or solver changes |
| `wasm` | the wasm32 build (stable; rebuilding std with `-Z build-std` was measured on 2026-10-31 to change neither the output nor the speed, and under 2 KB of size), `wasm-opt -O3 -ocimfs=0` (no inlining of single-caller functions, so long loops stay in functions V8 optimises: `docs/PERF.md`, *In the page*), then `webapp/example.json` (the result shown before the first search). `wasm-opt` (about a minute) is skipped when the compiled wasm, its flags (`WASM_OPT_FLAGS`, the one list the command also reads) and wasm-opt's version match its last run (`webapp/.loopopt.wasm.key`); the example always runs |
| `page` | `webapp/dist/` (see *The page*) |
| `images` | game art into `webapp/img/` (see *Art*); only new ids unless `IMAGES_FORCE=1` |
| `runners` | each uma's running mini model as a sprite strip in `webapp/img/run/` (see *Art*) |
| `browsers` | Playwright's browsers at the revisions the pinned `playwright` package wants into `cache/ms-playwright` (`playwright install`: a download after a Playwright bump, a 0.6 s no-op when they are there) |
| `test` / `testq` | the page (`page`, 1.4 s), then the browser suite on it, full or its fast tier; live, recording the solver's replies, when none are recorded for this solver and data (see *Tests*) |
| `prune-replies` | the page, then deletes the recorded solver replies of every solver and data but its own (see *Tests*, *Recorded solver replies*) |
| `serve` | `webapp/dist` on https://localhost:8000 (`PORT=…`) over HTTP/2 with TLS, as the host serves it: the certificate is self-signed (`cache/tls/`), so a browser warns once |
| `val TAG` / `valq TAG` | the release solver, then validation: in full before landing a change that can move results, quick while iterating (see *Tests*) |
| `mono` | the release solver, then the career planner's monotonicity check (`val/mono.py`, 10-20 CPU-min: in the background; see *Tests*) |
| `deploy` | builds the solver (`wasm`) and the page, and uploads `webapp/dist` to Cloudflare Pages (see *Releases*) |
| `research` | `research/run_all.py` into `research/results.json`, then renders `docs/RESEARCH.md` from `docs/RESEARCH.template.md` (`research/README.md`) |
| `help` | the steps, from `build.sh`'s table |

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
bundles they come from is in `webapp/images.py`'s docstring. `uv run --group art webapp/icons.py` renders the favicons and the
link-preview image from `webapp/icons/favicon.svg`.

`runners` writes one sprite strip per uma to `webapp/img/run/{chara}.avif`, the uma being planned running while a
search works (a spinner when a build has none). How the pipeline works and what to check when it breaks:
`webapp/chibi/README.md`.

## The page

`webapp/build.py` assembles `webapp/dist/`:
- `index.html` (the career planner, the front page; an old `/career` link redirects there), `loop.html` (`/loop`, the
  loop planner) and `about.html` (`/about`, "How this works", which holds
  the model text), from their templates. Shared markup lives in `panels.template.html`, blocks marked
  `<!--panel:name-->` … `<!--/panel:name-->` that fill each `__PANEL_NAME__`. Both planner pages carry `class="game"`
  on `<html>`, which scopes the game-style rules in `style.css` (career-only layout under `.rb`, the loop planner's
  under `.lp`).
- `assets/`: every shared file named by a fingerprint of its content and cached for a year: the stylesheet (with the
  font faces and UI sprites as CSS variables), the shared data script (`self.__LLP_DATA`), the page bundles, the
  solver's worker script, the wasm, each era's solver data, the Grand Masters page data (loaded when that era is
  chosen), the fonts and `img.<hash>/`. A revisit fetches only the HTML (about 25 KB for the career planner, 35 KB for
  the loop planner).
- each page's first-visit cost, printed at the end of the build: the page, the local files it names and, on the planner
  pages, the worker script, the wasm and the Global solver data, raw and at brotli quality 4 (the host's transfer of the wasm to within
  0.1%). A page over its budget in `webapp/budgets.json` fails the build; a budget rises only in the commit that needs
  it, which says why.
- favicons, `og.png`, font licences in `licenses/`, `_headers` (the host caches the pages 5 minutes, `assets/` a
  year), `_redirects` (`/loop` to `/`) and `robots.txt`. `serve.mjs` caches `assets/` a year as the host does and
  serves the pages uncached.

The solver runs as WebAssembly in a Web Worker (`worker.template.js`, minified to `assets/worker.<hash>.js`). Saved settings are one versioned object,
`localStorage['llp:settings']`, migrated in `settings.js`. Shared page state is two exported objects (`state`: the
settings, `ui`: what is on screen). No framework.

In `src/`, `main.js` and `career.js` are the two pages' entries; `engine.js` talks to the worker, `job.js` builds the
job both pages send, `save.js` decides what a settings change does, and `statestore.js` keeps the solver's work in
IndexedDB.

## Tests

- **Solver:** `cargo test --release` in `pkg/loopopt` (part of `native`); `./build.sh lint` for formatting and clippy.
- **Browser suite:** `./build.sh test` builds the page and runs Playwright Test (`webapp/playwright.config.mjs`, from the pinned
  `playwright` package) against `webapp/dist`, served by `serve.mjs` over HTTP/2 with TLS as the host serves it (a
  self-signed certificate made once into `cache/tls/`; over HTTP/1.1 requests queue and timings mislead), or against
  `LLP_URL`, test groups in parallel (`LLP_JOBS` per browser, default 1: four each kept the laptop at its thermal
  limit, 96.9 C on average, where two groups at once averaged 82.8 C; `LLP_JOBS=4` is about 2.5 times faster, by estimate, and hot):
  every group in Chromium, and in Firefox the
  groups tagged `@both` in `groups.spec.mjs` (where its engine matters: the worker, wasm, storage, timing, layout and
  focus; the others test the page's logic, which Chromium covers, and took 420 of Firefox's 650 CPU-s a run). Each
  group is a module in `webapp/test/` and one test of `webapp/test/groups.spec.mjs`; each check is a step of its test,
  a failed one a soft failure (the group goes on), a throw ends the group. Shared helpers are in
  `webapp/test/helpers.mjs`. One group: `cd webapp && npx playwright test --project chromium -g roster` (`-g` is a
  regular expression over the titles). A failed test keeps its trace
  (`npx playwright show-trace webapp/test-results/<test>/trace.zip`); the run's report is
  `webapp/test-results/report.json`. `LLP_PROF=file` logs where the time goes.
- **Recorded solver replies:** the groups that use the solver only to put the page in a state, or check what the page
  draws from its replies (`REPLAY` in `groups.spec.mjs`), answer every solver job from the recorded replies
  (`webapp/test/solverreplay.mjs`): the page's worker is a stand-in and its wasm compile gets an empty module, so they
  cost about a third of their live CPU. The other groups run the real solver: they check its worker's live behaviour
  (kills, restarts, progress, races, the stored work), the wasm as served, or the deck solver's odds. A reply's key is
  the wasm's and era data's fingerprints, the jobs that worker ran before and the job, so a solver, data or page change
  that alters a job fails its group with the job named; nothing falls back to the live solver. The replies of one
  solver and data are one directory, `cache/solver-replies/WASM-GLOBAL-GM` (the fingerprints in the asset names of the
  wasm and the two eras' engine data: `webapp/test/replaystore.mjs`). `LLP_SOLVER=live ./build.sh test` runs every
  group on the real solver and records the replies (about 45 MB). `test` and `testq` do so by themselves, saying so in
  one line, when the current directory holds no passing live run of their tier (`.recorded-full`, written by a passing
  live `test`, serves both; `.recorded-quick` serves `testq`), so after a solver or data change the first run records.
  Run it by hand after a page change that makes a replaying group fail on a missing reply, and before a release.
  `./build.sh prune-replies` deletes the directories of every other solver and data, naming each, and leaves anything
  a recording does not write. A live run also prints any reply that differs from the one stored under its key, which would mean the solver is
  not deterministic given its jobs and replay unsound. Replaying groups wait, before a reload, for the page's 100 ms
  state save to land, so the next worker's history does not depend on timing.
- **Fast tier:** `./build.sh testq` (Chromium, the tests tagged `@quick`, with `LLP_QUICK=1` stopping career at its
  one-run answer; about 30 s, 14 s with `LLP_JOBS=4`; popups, phone and contrast replay, career, load and about run live): run it after each
  page change, and the full suite before a release.
- **Validation:** `./build.sh val TAG` builds the release solver (so it never validates a binary left from before a
  change) and runs every validation check, each whatever the others found, and fails if any
  does (about 7 CPU-min, summed from the recorded run times: `val/ref_times.txt`, `val/plan/*_times.txt` and the proxy
  runs' `timing_s`):
  - the 26 reference jobs in `val/jobs.json` (22 deck jobs, shared and per-uma decks, one of them in Grand Masters, and
    4 career jobs; every job leaves out the R cards that are not pals, as the page does), compared with
    `val/ref__*` / `val/refpu__*` by `val/cmp2.py` and `val/same.py`; a solver failure there stops the step before
    any comparison;
  - `val/joint_check.py TAG`, the deck odds recomputed independently, within 1e-3;
  - the policy simulator's jobs (`val/simval.py`) and the career planner's (`val/planval.py` with its proxy gate, then
    `val/same.py TAG --plan`), below.

  `./build.sh valq TAG` is the same while iterating, and on every commit that touches the solver (about 2.5 CPU-min):
  without the slow jobs (`val/slow.json`, `val/plan/slow.json`), `joint_check` and planval's proxy gate
  (`planval.py --quick`). `val/cmp2.py` says what moved: P(all) rounded to 14 decimals, rank by rank, and
  each rank that names another deck or career, which passes only as a confirmed tie (P(all) and P(all but one) equal
  to the bit); `val/same.py TAG` then compares the whole outputs (every field but the timings: the byte identity speed
  work keeps, `docs/PERF.md`), so a confirmed tie still fails until its reference is re-taken. When results move on
  purpose, check the diff and copy the new outputs over the references in the same commit; when every value should
  scale by one factor, compare with `val/cmp_ratio.py`. Solver process failures and comparison failures exit nonzero. `uv run val/test_validation.py` checks these failure
  paths with synthetic outputs and a stub solver, without running any searches.
- **Career planner:** `uv run val/planval.py pkg/loopopt/target/release/loopopt TAG` runs the plan jobs in
  `val/plan/jobs.json` and compares each whole output with `val/plan/ref__*.out`. Two of them take the page's own
  settings: `p16_borrow_check` asks for the robustness check and the pair search (`next.robust: "all"`, every
  variant in one job, where the page sends `"next"`, one variant per job), `p16_borrow_never` sets `borrowed_parent:
  "never"`. `p360_check` is the planner at a real roster's scale (360 veterans, 75 borrowed, one with every target,
  with the check and pair search), written by `uv run val/make_pool.py val/plan/p360_check.json` from the other
  pools' veterans under fresh characters (a scale test, not a model of real rosters; V's trio on one real side and one
  still building, as on a real roster; 10.6 s alone, so `valq` runs it too).
  An output without a reference fails (planval and `cmp2.py`) until its reference is taken. `val/mono.py BIN`
  checks that adding a veteran never raises the planner's estimate (on every plan job but the check's, whose plan
  `p16_borrow` already is; `./build.sh mono` runs it, about 10-20 CPU-min, estimated from the plan jobs' times);
  `val/capaudit.py BIN` checks the exact caps.
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
- **solver:** `lint`, `native` (release build, clippy, tests), then `val ci` (every output byte-identical to the
  references, work counters included; *Tests*) and `mono`.
- **page:** `wasm page` (no game art in a clone: the page shows initials) and `testq` (Chromium; live, since CI holds no
  recorded solver replies); the checks that need
  the art skip themselves.

## Rust style

`./build.sh lint` passes before every Rust commit. Its Rust and shell checks, in `pkg/loopopt` (ruff and Biome follow;
see *Steps*):

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
uv run shellcheck build.sh .githooks/pre-commit   # (from the repo root)
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
  and format on the staged files; the page's JS, Biome (lint and formatting) on the staged files and `tsc`, and for any
  page source, `webapp/test/static.mjs` (each under a second). It checks
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
`cd webapp && LLP_URL=https://<hash>.uma-legacy.pages.dev LLP_QUICK=1 npx playwright test --project chromium --grep
@quick`: the branch addresses cache HTML for 5 minutes and can serve the previous page until then.

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
cache/               downloads: GameTora's files, Playwright's browsers, the runner tools and exports (gitignored)
```
