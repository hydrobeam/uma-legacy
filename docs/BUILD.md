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
| `pkg/loopopt/Cargo.toml` | `ts-rs` (pinned exactly; MIT): derives the TypeScript type of every report and job struct, so the page's types are generated from the solver's (*Generated types*); its derives are dead code in the wasm (none of the declarations' text is in it) | `types`, `lint` |
| `pkg/loopopt/Cargo.toml` | `clap` (pinned exactly, its derive API; MIT or Apache-2.0): the native `loopopt` command line's arguments, subcommands and errors; a native-only dependency, not in the wasm | `native`, `val`, `pb` |
| `pkg/loopopt/Cargo.toml` | `wasm-bindgen` (pinned exactly; MIT or Apache-2.0): the wasm's exports (`src/web.rs`, the `Solver` class the worker makes) and the host's imports (`career/clock.rs`: `performance.now()`, the worker's progress hook), through its generated glue; `console_error_panic_hook` (pinned exactly; MIT or Apache-2.0): a panic's message in the console before the trap. Both wasm-only | `wasm` |
| `webapp/package.json` (`npm ci` on first use) | the page's packages, below | `wasm`, `page`, `runners`, the tests, `deploy`, `lint` |
| `webapp/chibi/package.json` (`npm ci` there by `runners`) | `three` 0.170.0: three.js, which `chibi.html` draws the running mini models with | `runners` |
| `webapp/deploy/package.json` (`npm ci` there by `deploy`) | `wrangler` 4.147.0 (with its Cloudflare stack, about 210 MB, so the page's install goes without it) | `deploy` (Cloudflare Pages) |
| `pyproject.toml` (`.venv/`, created by uv) | `fonttools`, `brotli` | `page` (/about's math font subset, `webapp/vite/mathfont.py`; `brotli` is fonttools' woff2 writer); the exporters use the standard library |
| `pyproject.toml`, group `art` | `UnityPy`, `Pillow` | `images`, `runners`, `webapp/icons.py` |
| `pyproject.toml`, group `dev` | `ruff`, `shellcheck-py` | `lint` |
| `pyproject.toml`, group `research` | `numpy`, `scipy` | `research` |
| `cache/wasm-bindgen/` (`wasm`, first use) | `wasm-bindgen-cli` at the `wasm-bindgen` crate's version in `pkg/loopopt/Cargo.lock` (the glue's format is the crate's, so the two move together): `cargo install --locked --root cache/wasm-bindgen`, a few CPU-minutes, again whenever the crate's pin moves | `wasm` |
| `cache/ms-playwright/` (`./build.sh browsers`) | Playwright's Chromium and Firefox (`PLAYWRIGHT_BROWSERS_PATH`; outside `node_modules`, so `npm ci` keeps them) | `test`, `testq`, `runners` |
| `cache/gametora/` (`./build.sh gametora`) | GameTora's data files at the versions pinned in `pkg/loopopt/data/gametora.json`. GameTora does not keep old versions online (6 of the 13 pins answered 404 on 2026-10-21), so the pinned set exists only in a cache that already holds it; elsewhere `GAMETORA_LATEST=1` moves the pins, and the events may change | `events` |
| `cache/chibi/tools/` (`runners`, first use) | AssetStudioModCLI 0.19.0, a .NET 9 runtime, FBX2glTF 0.9.7, libxml2.so.2 (Arch's libxml2-legacy; always this copy) | `runners` |

The page's packages (`webapp/package.json`; build and dev tools pinned exactly), each for what it replaces or does:

| Package | Why |
|---|---|
| `preact`, `@preact/signals` | the page's views and stores (bundled into the page); signals are imported from `@preact/signals` alone (Biome's `noRestrictedImports` bans `@preact/signals-core`: one package, one signal graph) |
| `vite` 8.3.5 | builds `webapp/dist/` (`page`): bundles the entries and the worker, fingerprints every asset, minifies the stylesheet, serves the build (`serve`, the browser suite); in place of the hand-written `build.py` and `serve.mjs` |
| `vite-plugin-handlebars` 2.0.3 | the pages' shared head and header markup (`webapp/partials/`) and the values they show (version, site address, /about's research figures); in place of `build.py`'s own templating |
| `preact-render-to-string` 6.8.2 | each planner page's root drawn into its HTML at build time (`vite/prerender.mjs`), which the entry hydrates |
| `vite-plugin-static-copy` 4.1.1 | copies the fonts' licences into `dist/licenses/` |
| `@vitejs/plugin-basic-ssl` 2.3.0 | the self-signed certificate `vite preview` serves HTTP/2 with (`cache/tls/`), in place of `serve.mjs`'s openssl call |
| `@fontsource/*` (4 fonts) | the self-hosted fonts' Latin subsets, imported by the stylesheets' entries (`src/styles/`) |
| `tailwindcss`, `@tailwindcss/vite` 4.3.3 | the page's styles: every component's look as utilities, the tokens in `src/styles/theme.css` (*Styles*) |
| `tailwind-variants` 4.0.1 | a component's class lists by its options (`tv`) and conditional joins (`cx`), through its `lite` entry (no tailwind-merge) |
| `binaryen` | `wasm-opt` (`wasm`) |
| `knip` 6.44.1 | dead exports, files and dependencies, imports of nothing (`lint`) |
| `vitest` 5.0.5, `happy-dom` 21.0.4 | the node tests (`webapp/test/unit/`), compiled by the page's Vite config; happy-dom is the document of those that draw views, in place of the hand-written `fakedom.mjs` |
| `dependency-cruiser` 18.7.0, `@swc/core` 1.16.21 | the module layers (`lint`); swc parses the TypeScript, as dependency-cruiser reads it through TypeScript's JavaScript API, which TypeScript 7 no longer has |
| `oxlint` 1.90.0, `eslint-plugin-better-tailwindcss` 4.10.0 | the Tailwind class gate (`lint`, the hook): a class Tailwind doesn't know fails (`webapp/.oxlintrc.json`) |
| `@testing-library/dom` 10.4.2 | the node tests' role queries (`within(root).getByRole`) |
| `playwright` (pinned) | the browser suite, `icons.mjs`, `runners` |
| `@biomejs/biome`, `typescript` | `lint` |

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

**Worktrees** (`git worktree add`): `build.sh` links a new worktree's `cache/` to the main checkout's on first use, so
Playwright's browsers, the recorded solver replies, GameTora's files, wasm-bindgen and the TLS certificate are
downloaded or recorded once for every worktree (`prune-replies` from any of them prunes the shared store). `npm ci`
fills each worktree's own `node_modules` from npm's shared cache (`--prefer-offline`; about a minute): a symlinked
`node_modules` is emptied for every tree by the next `npm ci` in any of them. Rust builds per worktree: the crate's
dependencies are 7 s of a build, and the crate itself, nearly all of it, differs by tree, so a shared compile cache
(sccache, measured 2026-11-01: no hits across target folders) or a shared target folder (one lock, every lane
rebuilding the others' source) saves nothing. `game/` and `webapp/img/` stay per checkout (link them by hand
for the art).

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
| `data` | the Global and Grand Masters-era exports from `master.mdb`: `base.json` (roster and aptitudes), `data.json`, `web_skills.json`, `web_cards.json` (with the scenario card bans), `career.json` (the veteran import's spark-factor and win-saddle tables), `upcoming.json`, then the page's lists (`page.json`, below) |
| `gametora` | GameTora's pinned data files into `cache/gametora/`; `GAMETORA_LATEST=1` moves the pins to the newest versions first; it also rewrites `pkg/loopopt/data/gametora_slugs.json`, the page's links to GameTora (tracked, so `page` downloads nothing), then the page's lists. Run by `events` |
| `events` | each era's card event file from GameTora's files plus the hand-entered entries (pal first-year dates left out, as IT never fires them), then the page's lists |
| `native` | clippy (as `lint` runs it), then `cargo test --profile checks` (*Tests*) |
| `check [TIER]` | the checks a change needs, by what differs from main and what already passed: `edit`, `handback` (the default), `merge` or `full` (*Tests*, *Tiers*) |
| `lint` | every check below runs whatever the others found, and the step ends by naming the ones that failed, each with its wall time; Rust's (formatting, clippy, the generated types) only when a Rust input differs from main's: `cargo fmt --check` and clippy with warnings denied, on every target and on the wasm lib (see *Rust style*); the generated TypeScript current (`cargo test --lib ts_gen`, *Generated types*); shellcheck on `build.sh` and the hook (`shellcheck-py`, dev group); `ruff check` and `ruff format --check` on the Python (rules in `pyproject.toml`; ruff from the dev group, locked in `uv.lock`); `val/test_validation.py` (planval's and the comparisons' gates on a fake solver, 3 s; it had gone four days failing unnoticed when nothing ran it); `research/test_s08_likelihood.py` (§8's chain likelihood on synthetic cases, research group, about 5 s; it reads the chain count from `loopopt constants`, so the checks' binary is built first); `biome check` on the page's JS (lint and formatting in one pass: 2-space indent, single quotes, 120 columns) with warnings as errors (`webapp/biome.json`; reasoned `biome-ignore` comments where a rule is wrong for a line); TypeScript's checker, twice: the page's TypeScript, strict (`webapp/tsconfig.json`; every module is `.ts` or `.tsx`, and with `allowJs` off an import of a stray `.js` fails), and the solver's worker with a worker's globals (`webapp/tsconfig.worker.json`, strict); what the browser suite can't see going stale, by standard tools first: knip (`webapp/knip.jsonc`: files no entry reaches, exports no other file imports, dependencies nothing uses, imports of a package not listed or a file that does not exist), dependency-cruiser (`webapp/.dependency-cruiser.mjs`: a value import points to its own layer of `LAYERS` or one below, nothing outside a page imports into it, so no logic module (types, io, model) reaches a store; a `.ts` module imports no `.tsx` one, a type import included), the Tailwind class gate (oxlint running eslint-plugin-better-tailwindcss's `no-unknown-classes` over `webapp/src`, `webapp/.oxlintrc.json`: a class in a `class` attribute or a `cx()`/`tv()` argument that the theme and Tailwind don't define, a typo or a name nothing styles; class lists in plain constants are not checked), and Biome's rules (its Tailwind rules among them: classes sorted, shorthands, whole static class strings, no raw palette colour, no arbitrary value but layout, no legacy utility, no restyling of a component through its `class`) (`webapp/biome.json`; each allowance an override or a plugin's `includes`, its reason in the rule's message or the plugin's header): import cycles (`noImportCycles`); effects (`effect` imported anywhere but `store.ts`: `noRestrictedImports`; a `.subscribe()` or a namespace's `.effect()`: `lint/effects.grit`); exported mutable bindings (`lint/mutable-export.grit`); `@ts-ignore` (`noTsIgnore`); storage, written only by `storage.ts`'s read-modify-write `update` (the `localStorage` global by `noRestrictedGlobals`, a `.setItem`/`.removeItem` by `lint/storage-write.grit`); a store module's state made by `store.ts`'s `store()`, which hands it out `DeepReadonly` (`signal` banned there); signals from `@preact/signals` alone; `render`/`hydrate` in the two entries alone (each page is one root); a lookup in the document outside the entries (`lint/dom-lookup.grit`); a `<dialog>` or `showModal()` outside `ui/Dialog.tsx` (`lint/dialogs.grit`); a `requestAnimationFrame` outside `veterans/store.ts` (`noRestrictedGlobals`); a listener added at a module's top level outside the entries, `errorlog.ts` and `gmtip.ts` (`lint/load-listener.grit`); a view built from a markup string (`lint/markup-string.grit`); then a grep for `@ts-expect-error` in `webapp/src` (tsc accepts it and no linter sees a comment); the page's node tests (`webapp/test/unit/`, Vitest) |
| `hooks` | `git config core.hooksPath .githooks`: the tracked pre-commit hook (see *Rust style*); once per clone |
| `pb` | the release solver (cargo's build, a no-op on an unchanged tree), then each uma pair's exact affinity ceiling (`loopopt pbtable`, via `export/add_pb.py`) stored in both `data.json` files, then the page's lists (`data.json` is one of their inputs); it only speeds searches up. The table carries a fingerprint of the umas, goals and streak penalties; on a mismatch the solver ignores it and `cargo test` warns. Rerun after data or solver changes |
| `wasm` | the wasm32 build (stable; rebuilding std with `-Z build-std` was measured on 2026-10-31 to change neither the output nor the speed, and under 2 KB of size), `wasm-bindgen --target web --omit-default-module-path` (into `pkg/loopopt/target/bindgen/`: the wasm with the glue's exports, and the glue, `loopopt.js` and `loopopt.d.ts`, copied to `webapp/src/gen/`: an ES module the worker imports, instantiated from the page's compiled module by `initSync`, with no fallback fetch of the wasm by URL), `wasm-opt -O3 -ocimfs=0` (no inlining of single-caller functions, so long loops stay in functions V8 optimises: `docs/PERF.md`, *In the page*), then `webapp/example.json` (the result shown before the first search). `wasm-opt` (about a minute) is skipped when the bound wasm, its flags (`WASM_OPT_FLAGS`, the one list the command also reads) and wasm-opt's version match its last run (`webapp/.loopopt.wasm.key`); the example always runs |
| `page` | `webapp/dist/` (see *The page*) |
| `images` | game art into `webapp/img/` (see *Art*); only new ids unless `IMAGES_FORCE=1` |
| `runners` | each uma's running mini model as a sprite strip in `webapp/img/run/` (see *Art*) |
| `browsers` | Playwright's browsers at the revisions the pinned `playwright` package wants into `cache/ms-playwright` (`playwright install`: a download after a Playwright bump, a 0.6 s no-op when they are there) |
| `test` / `testq` | the page (`page`, 1.4 s), then the browser suite on it, full or its fast tier; live, recording the solver's replies, when none are recorded for this solver and data (see *Tests*) |
| `prune-replies` | the page, then deletes the recorded solver replies of every solver and data but its own (see *Tests*, *Recorded solver replies*) |
| `serve` | `webapp/dist` on https://localhost:8000 (`PORT=…`) over HTTP/2 with TLS, as the host serves it (`vite preview`): the certificate is self-signed (`cache/tls/`), so a browser warns once. Unlike the host, it revalidates `assets/` on each load (ETags) rather than caching them a year |
| `val TAG` / `valq TAG` | the release solver, then validation: in full before landing a change that can move results, quick while iterating (see *Tests*) |
| `mono` | the release solver, then the career planner's monotonicity check (`val/mono.py`, 10-20 CPU-min: in the background; see *Tests*) |
| `deploy` | builds the solver (`wasm`) and the page, and uploads `webapp/dist` to Cloudflare Pages (see *Releases*) |
| `research` | `research/run_all.py` into `research/results.json`, then renders `docs/RESEARCH.md` from `docs/RESEARCH.template.md` (`research/README.md`) |
| `help` | the steps, from `build.sh`'s table |

The solver's training-hint constants (each scenario's base and specialty weight, `pkg/loopopt/src/deck/hints.rs`) are
the research fit: `pkg/loopopt/build.rs` reads them from the tracked `research/results.json` (section 7's keys) at every
build, native or wasm, and rounds them to 3 decimals, so a `research` run that moves one at that precision moves the
solver's results (rerun validation, `pb`, and update `/about` and `docs/MODEL.md`, which state them by hand). The page's
scenario tiles take the bases the same way: `webapp/vite/research.mjs` (`hintBases`) reads build.rs's keys with its
rounding into the page's data. A missing key fails either build with its name.

## Data updates

Put the new `master.mdb` files in `game/`, then `./build.sh data events pb`, validation, and a release.

- `data` keeps every uma, outfit and support card that `master.mdb` dates up to 60 days past the build
  (`AHEAD` in `pkg/loopopt/export/era.py`; an outfit starts with its first gacha, else with its uma; content the game has
  not dated carries a 2050 placeholder and stays out). `pkg/loopopt/data/global/upcoming.json` lists what starts after the build.
  The page hides each entry until its start time and leaves it out of every search (`webapp/src/data.ts` `gameData().later`,
  `engine.ts`), so a banner turns on by itself; the precomputed example leaves the same entries out
  (`webapp/example.mjs`). Validation runs on everything in the data, upcoming included.
- Manual card entries (pal and group cards; the builder decodes only SR/SSR chains): each era's
  `pkg/loopopt/data/<era>/manual.json`, which `events` merges into that era's `card_events.json`. GameTora's files are
  read and decoded in one place, `pkg/loopopt/export/gametora.py` (its docstring holds their format); the builders turn
  events into card entries with `event_choices.py`, and `val/event_audit.py` and `val/secret_events.py` read them through
  it too. After touching them, `uv run val/event_audit.py` should say "every target hint is accounted for".
- The page's lists (the card picker's cards, the roster with release dates, the skill catalog, the career page's tables)
  are exported too: `pkg/loopopt/export/export_page.py` writes each era's `page.json` and the shared
  `page_raceids.json` (race names to race ids) from the other exports, `gametora_slugs.json` and `master.mdb` (the
  release dates; without it, `upcoming.json`'s and id order). `data`, `gametora`, `events` and `pb` end by running it,
  since each writes one of its inputs. `page.json` records the sha256 of every file it was made from, and the page
  build stops ("run ./build.sh data") when one has changed since; a `master.mdb` absent from the checkout is skipped.
  Without `master.mdb`, the export refuses to replace a `page.json` that was made from it (its dates would fall back to
  id order in a tracked file); `--fallback` does it anyway.
- New characters' art comes from `./build.sh images` (and `runners`); without it they show initials.

## Art

`images` writes `webapp/img/{kind}/{id}.avif` (AVIF q55, loaded lazily) and `img/manifest.json` (sources and gaps):
character icons, card art, skill icons, race banners, scenario logos and select art, mob portraits, and the UI
sprites as lossless WebP with dark versions made here (the game has no dark mode). The full list of kinds and the
bundles they come from is in `webapp/images.py`'s docstring. `uv run --group art webapp/icons.py` renders the favicons and the
link-preview image from `webapp/public/favicon.svg`.

`runners` writes one sprite strip per uma to `webapp/img/run/{chara}.avif`, the uma being planned running while a
search works (a spinner when a build has none). How the pipeline works and what to check when it breaks:
`webapp/chibi/README.md`.

## The page

Vite builds `webapp/dist/` (`webapp/vite.config.mjs`; its plugins in `webapp/vite/`). It prepares no game data: the
page's lists are each era's exported `page.json` (*Data updates*), refused when a file they were made from has changed
since (`vite/pagedata.mjs`); the build adds only what depends on the tree it runs in, the research constants
(`research/results.json`, read as `build.rs` reads them: `vite/research.mjs`) and the career tables' mobs cut to those
with a portrait in `webapp/img/`. It writes:
- `index.html` (the career planner, the front page; an old `/career` link redirects there), `loop.html` (`/loop`, the
  loop planner) and `about.html` (`/about`, "How this works", which holds the model text), from the pages of those names
  in `webapp/`. Shared head and header markup lives in `webapp/partials/` (the theme script, the header), each
  `{{> name}}` filled by `vite-plugin-handlebars`, which also fills the values a page shows (`{{version}}`, `{{site}}`,
  /about's figures; a name without a value stops the build). Everything under a planner page's header is one Preact
  root, `<div id="app">`: its layout (`src/career/Layout.tsx`, `src/loop/Layout.tsx`, which import Preact, `src/ui/Shell.tsx`
  and the `ui/` primitives, no store) is drawn into the HTML at build time by Preact's `renderToString` (`vite/prerender.mjs`,
  through Vite's `runnerImport`), so the first paint has the setup's markup, and the entry hydrates it.
  (@preact/preset-vite's prerender, vite-prerender-plugin, renders every route into the first HTML input as one
  template, so it can't fill two pages with heads of their own.) The footer's version reaches the entries as
  `__LLP_VERSION__` (Vite's `define`). The solver's preload script and asset table go into each planner page's head
  from `vite/pagedata.mjs`.
  Both planner pages carry `class="game"` on `<html>` (read by `ui/Agenda.tsx`); where the two pages' looks differ, a
  component takes the page as a prop or reads its root's `data-page` through the theme's `career:`/`loop:` variants.
- **Styles** are Tailwind v4's (`@tailwindcss/vite`): each component carries its look as utilities, so CSS exists only
  for what something names. `src/styles/app.css` is the planner pages' entry (the fonts, Preflight, the theme, the
  utilities); /about has its own (`src/styles/about.css`, sourcing only `about.html` and the partials, its prose's
  elements styled in one `@layer base` block); both import `src/styles/site.css` (`dark:` bound to the site's
  `data-theme` switch, and the few element rules every page shares in `@layer base`). The tokens
  are `src/styles/theme.css`: Tailwind's own colours, fonts, type sizes, radii, shadows, breakpoints and containers
  cleared, then the site's and the game's palettes (`--color-*`), redefined for dark mode in one
  `:root[data-theme="dark"]` block (no `dark:` for the art); the type scale by role (`text-caption` … `text-display`,
  not Tailwind's values); five breakpoints (written desktop-first, `max-md:` a phone, 620 px and under); five dialog
  widths; the game's sprites as `--nine-*` (9-slice border-images, their faces `--face-*` swapped by dark mode, drawn
  by the `nine-*` utility), layered backgrounds as `--art-*` (`art-*`; never with a `bg-*` on one element) and the
  bars' slant as `clip-*`; and named custom variants for state a root carries (`stale:`, `on:`, `career:`, ...).
  Arbitrary values are for layout only (Biome's `noTailwindArbitraryValue`). Primitives in `src/ui/` (Button,
  GameBar, Dialog and its parts, SegmentedControl, Checkbox, Switch, the panel and popover frames, AffTable, the
  image components) carry the shared looks; a caller's `class` on a component only places it. A single element with
  options is one `tv()` (prop types from `VariantProps`); a multi-part component is subcomponents with fixed class
  strings, an option that changes several parts a `data-*` attribute on its root read by a custom variant; `cx()` for
  one-off conditionals; class names are always written whole. Tests find elements by role, name, id or `data-*`,
  never by a style class.
- `assets/`: every shared file named by a fingerprint of its content and cached for a year: the stylesheet (with the
  font faces and the UI sprites as CSS variables, `vite/images.mjs`), the page scripts (each page's entry and the
  modules both share, with the Global era's data: `#page-data`, and the career tables on the career page:
  `#career-data`), the solver's worker script, the wasm, each era's solver data, the Grand Masters page data (loaded
  when that era is chosen), the fonts, /about's math font (`vite/mathfont.mjs`) and `img.<hash>/` (the game's images,
  loaded by id). A revisit fetches only the HTML (about 10 KB per planner page). Each planner page's head carries the
  asset table (`data-assets`), from which its first script starts the solver's downloads.
- favicons, `og.png`, `_headers` (the host caches the pages 5 minutes, `assets/` a year), `_redirects` (`/loop` to `/`)
  and `robots.txt`, from `webapp/public/` as they are, and the fonts' licences in `licenses/`.

The solver runs as WebAssembly in a Web Worker (`src/worker.ts`, built by Vite to `assets/worker-<hash>.js`; its messages both ways are typed in `src/solver.ts`). Saved settings are one versioned object,
`localStorage['llp:settings']`, read and written in `saved.ts`. The settings are a store (`settings.ts`, on
@preact/signals): one record in a signal, handed out read-only; named setters copy it on write, save it, then assign
it. Each page's entry reacts to a change of setup with one effect (`store.ts` `on`, the only place an effect is made);
the loop page's run state (the search as the status line draws it, the status and the result on show) is a store too
(`loop/state.ts`), changed only by the transitions its run machine (`loop/run.ts`, `createRun(deps)`: the engine, clock,
timers, storage and settings passed in) makes. The career page's run and answer are a third store (`career/state.ts`):
what the status line draws of the run, the answer on show and the last plan made, each a read-only signal that named
transitions replace. Its run machine (`career/run.ts`, `createRun(deps)`: the search, the plan and the robustness check,
with the store's transitions, the engine, clock, page and redraws passed in) imports no store, and node tests run it on
fakes. A plan is known by an id given when it is made, so the check, whose writes replace the plan's record, writes its
result wherever the plan is kept. The veterans and the next career's places are the fourth store
(`veterans/store.ts`): one record in a signal, handed out as read-only parts (`vets`, `places`, `chosen`); its mutators
build the next record copy-on-write (an edited veteran is a new record, the others stay the same objects), save it (once
after the next frame, at once while the page is hidden), then assign it, and its editors address a node by where it is
(a veteran's id, her parent's place), never by an object they hold. Try a run's veteran analysis is a fifth
(`career/analysis/state.ts`: her comparison, one record replaced by a transition per phase; her draft is the veterans
store's `draft`, edited by path through `draftRef`), changed by its own run machine (`career/analysis/compare.ts`,
`createComparison(deps)`), which the career run machine drives through its hooks. Views are Preact components that read the stores through @preact/signals: the loop
page's, and the career page's status line, tree, node popup, parent picker, deck popup, veterans dialog (its two tab
lists both kept mounted, a card drawn again only when her record changes), setup summary and Try a run, and the setup
editors both pages share (`ui/`: the scenario picker, the lineage rank, the Umas and Support cards panels, the target
skills and their presets, listed by `ui/Setup.tsx`'s `setupViews`), and the career page's results column, popups and
veteran editors, each drawn in its place in its page's one root (the layout; the entry makes the views once and hands
them in after the game data loads); the deck card panel too (`ui/CardPop.tsx`, opened by its openers' own clicks,
`usePopOpener`: a slot in the results column and in every dialog, since a popover over a modal dialog is inert in both
engines). Every modal dialog is `ui/Dialog.tsx`'s, which alone calls `showModal`/`close`. Measured placement and the
spark names' fit (`details.ts`, from the layout effect of the component it measures), the game tooltip (`gmtip.ts`)
and the run machines stay imperative by design. Logic modules (`.ts`) take values and import no component
(`.dependency-cruiser.mjs`), and a component reaches its DOM by refs: no lookup in the document outside the entries
(Biome, `lint/dom-lookup.grit`).

In `src/`, `loop/main.tsx` and `career/main.tsx` are the two pages' entries (each page's own modules are in its folder,
`loop/` and `career/`; `career/main.tsx`'s header maps its folder: the job, the run, the views in `career/view/`);
`engine.ts` talks to the worker (the messages both ways typed in `solver.ts`), `job.ts` holds the job fields both pages
send (from the settings it is given; `loop/job.ts` and `career/job.ts` build their jobs from them), and `statestore.ts`
keeps the solver's work in IndexedDB.

Every page module is `.ts` or `.tsx`, checked strict with `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`
and the four flags that cost nothing (`webapp/tsconfig.json`, the worker's too); none carries `@ts-expect-error` or
`@ts-ignore` (Biome's `noTsIgnore` fails one; `lint` and the hook grep for `@ts-expect-error`, which no linter sees). An index read is avoided where the data can take the shape it is read in
(records in place of parallel lists: `loop/members.ts`; a veteran by her id: `career/current.ts` `legacyVet`); a key
from data (a grade, a race's track, a sort) gets its type where it is first read, checked there (`grades.ts`
`gradeAt`, `data.ts`). What is left is `must(value, reason)` for a value provably there, `assert(cond, reason)` for a
fact such as two lists being as long, and a guard only where the missing case is a real state, named beside it.
Never `x!` (Biome bans it), an `as` cast past the check, or `?? 0` on what must not be missing.

## Generated types

Every reply the solver gives (each command's report and its refusal, the CLI's outputs) and every job it reads is a Rust
struct or enum deriving `serde::Serialize` or `Deserialize` and `ts_rs::TS`; nothing that leaves the crate is built as
an untyped `serde_json::Value`. The page's TypeScript for them is generated from those structs, field docs included, into
one file, `webapp/src/gen/solver.ts`, beside the model constants the solver uses (`webapp/src/gen/constants.ts`:
`SOLVER`, serialised from the consts themselves, with its type). Both are committed and never edited by hand, nor
formatted (Biome leaves `src/gen` alone): a test, `pkg/loopopt/src/ts_gen.rs` (in `cargo test`, and so in `./build.sh
lint`), generates them and fails while a committed file differs; change the struct or constant, then run `BLESS=1
cargo test --lib ts_gen` in `pkg/loopopt` to write them (as `tests/it/plan_roll.rs` re-takes its fixture). The page
imports the generated types, adding only the fields it sets itself (a type like `CareerReport & { anyOutfit?: boolean
}`).

The worker's glue to the wasm, `webapp/src/gen/loopopt.js` with its types in `loopopt.d.ts`, is wasm-bindgen's, written
by `./build.sh wasm` (*Steps*) and committed beside them, so tsc and knip check the worker without a wasm build. It
changes only with the wasm's exports and imports (`src/web.rs`, `career/clock.rs`) or wasm-bindgen's version: commit it
with that change. Biome neither formats nor lints it. The worker imports nothing else of the wasm's interface, and
declares none of it by hand.

How serde's encoding reads in the TypeScript: an `Option` written as null is `T | null`; a field left out where it does
not apply (`skip_serializing_if`) is `field?: T`; a reply with several shapes is an enum tagged by `kind`
(`#[serde(tag = "kind")]`), so the TypeScript is a discriminated union the page narrows by `kind` (a refusal is `{kind:
"refusal", error}`); a record inside another is a nested object (no `#[serde(flatten)]`); an f64 that can be infinite
is written as null (serde_json), and such a field is typed `number | null`. A choice a job makes is an enum written by
name (`RankClass`, `Scenario`, `PickRule`, ...); a game's integer code is an enum written as its integer (`serde_repr`,
src/codes.rs: `Grade`, `Distance`, `Surface`) with `#[ts(repr(enum))]`, so its TypeScript is an `export enum` of the
same integers (ts-rs reads no serde_repr: without it, it writes a union of the names for a value the wire carries as an
integer).

## Tests

**Tiers.** `./build.sh check [TIER]` runs the checks a change needs: it reads what differs from main (the commits
since the branch left it, staged, unstaged and untracked files; on main, from its upstream) and runs each area's
checks only when one of its inputs changed. Each check prints its wall time, and the tier its total against its
budget: an overrun is reported, never a failure.

| tier | when | budget | runs |
|---|---|---|---|
| `edit` | every edit | 10 s | the page's two `tsc` passes and the node tests of the changed files (`vitest related`); clippy and `cargo test --lib` (checks profile); ruff on the changed Python |
| hook | every commit | 30 s | formatting and types of the staged languages (*Pre-commit hook*) |
| `handback` (default) | a piece of work handed back | 5 min | `edit`'s, then every linter of each changed area; the Rust tests and `valq` (checks profile) when the solver, its tests, data or `val/` changed; `research` when its inputs changed (each section recomputes on its own inputs); the browser suite's fast tier when the page or the solver changed |
| `merge` | before a merge to main | 10 min | every area, changed or not: `lint`'s linters, `native`, full `val`, the full browser suite, and `mono` after a change under `src/career/` |
| `full` | by hand | none | `merge`'s, then `mono` and `research --force` whatever changed |

Nothing runs twice on the same inputs: `native`, `val`, `valq` and `mono` record a pass under the key of their inputs
(`cache/passed/NAME-KEY`: the SHA-256 of the files they read, tracked and untracked, and `build.sh`), shared by every
worktree, and on the same key say so and return; a page-only change never runs them. `lint` runs Rust's linters only
when a Rust input differs from main's. The wasm is shared the same way (`cache/wasm/KEY`): a worktree whose solver
inputs were built before copies it instead of four minutes of fat-LTO build and `wasm-opt`.

**Profiles.** Every test, `val`, `valq`, `mono` and lint's generated-types and section-8 tests build in
`[profile.checks]`, with `debug-assertions` and `overflow-checks` on (release hides both from the tests); the wasm and
`pb`'s and research's binary in `[profile.release]`; clippy checks without code generation. valq is byte-identical
with the assertions on (2026-11-01), which cost the solver 8-12% more instructions. The unit tests stay in the checks
profile: the dev profile (`opt-level` 1, dependencies 3) rebuilt them no faster after a one-line edit (31 s against
34) and ran them slower (31 s against 25), and a second profile compiles the crate twice. Neither meets the edit
tier's 10 s: one unit test, `deck::tests::chain_marginals_dense_matches_ref`, is most of their run.

**Type-only changes.** A change the type checker fully explains (a rename, a type in place of a bare number, moved
code; no change to arithmetic, control flow or data) runs the unit tests and clippy per commit and one `valq` at the
merge, not a `valq` per commit. A change to arithmetic, control flow or data follows AGENTS.md's pinned procedure.

- **Solver:** `cargo test --profile checks` in `pkg/loopopt` (part of `native`); `./build.sh lint` for formatting and
  clippy. The checks' profile (`[profile.checks]` in `pkg/loopopt/Cargo.toml`) is release's opt-level 3 without fat
  LTO and one codegen unit, which had made every test binary a minute or two of single-threaded linking; val, valq,
  mono and lint run its binary, `target/checks/loopopt`, too. Its outputs are byte-identical to the release binary's on
  every valq job (checked 2026-11-01); the wasm, `pb` and `research` keep `[profile.release]`.
  The integration tests are one crate, `pkg/loopopt/tests/it/` (a module per file, `main.rs` lists them: the
  crate links once and libtest runs all of them in parallel), and run the solver binary through one harness, `tests/it/common.rs`,
  which clears the kept-state variables (`LOOPOPT_STATE_IN`, `LOOPOPT_STATE_OUT`) of the shell running them. When
  results move on purpose, `BLESS=1 cargo test --profile checks --test it plan_roll::rolls_equal` re-takes the roll fixture
  (`tests/fixtures/plan_roll.json`) from the solver's own rolls, naming each roll that changed and any sample whose y
  crossed 0.5 (the fixture keeps samples on both sides; pick another there); check the diff and commit it.
- **Page logic in node:** `npx vitest run` in `webapp` (part of `lint` and the pre-commit hook; about 2 s) runs the
  page's node tests, `webapp/test/unit/`, with the modules compiled as `vite.config.mjs` compiles them (its `test`
  branch: none of the build's plugins). A test file that draws views or runs editors asks for happy-dom's document
  (`// @vitest-environment happy-dom`) and mounts them in it as the page does; one that reads the game data mocks the
  build's page data with a small made-up game's (`vi.mock('#page-data', () => import('./fakepage.mjs'))`). happy-dom
  fires a dialog's `close` at once where a browser queues it, so a test lets tasks run after a close before it reads
  what the close did.
- **Browser suite:** `./build.sh test` builds the page and runs Playwright Test (`webapp/playwright.config.mjs`, from the pinned
  `playwright` package) against `webapp/dist`, served by `vite preview` over HTTP/2 with TLS as the host serves it (a
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
  (kills, restarts, progress, races, the stored work), the wasm as served, or the deck solver's odds. The replies of one
  solver and data are one directory, `cache/solver-replies/WASM-GLOBAL-GM-WORKER`, named by the SHA-256 of those files'
  bytes (the wasm, the two eras' engine data and the worker script: `webapp/test/replaystore.mjs`), never by an asset's
  name, so a page change that leaves them alone (a view, the stylesheet, the bundler's file names) replays. Within it a
  reply's key is the era data's digest, the jobs that worker ran before and the job, so a page change that alters a job
  fails its group with the job named; nothing falls back to the live solver. `LLP_SOLVER=live ./build.sh test` runs every
  group on the real solver and records the replies (about 45 MB). `test` and `testq` do so by themselves, saying so in
  one line, when the current directory holds no passing live run of their tier (`.recorded-full`, written by a passing
  live `test`, serves both; `.recorded-quick` serves `testq`), so after a solver or data change the first run records.
  Run it by hand after a page change that makes a replaying group fail on a missing reply, and before a release.
  `./build.sh prune-replies` deletes the directories of every other solver and data, naming each, and leaves anything
  a recording does not write. A live run also prints any reply that differs from the one stored under its key, which would mean the solver is
  not deterministic given its jobs and replay unsound. Replaying groups wait, before a reload, for the page's 100 ms
  state save to land, so the next worker's history does not depend on timing.
- **Fast tier:** `./build.sh testq` (Chromium, the tests tagged `@quick`, with `LLP_QUICK=1` stopping career at its
  one-run answer; about 30 s, 14 s with `LLP_JOBS=4`; popups and phone replay, career, load and about run live): run it after each
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
- **Career planner:** `uv run val/planval.py pkg/loopopt/target/checks/loopopt TAG` runs the plan jobs in
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
- **Policy simulator:** `uv run val/simval.py pkg/loopopt/target/checks/loopopt TAG` runs the five short simulator
  jobs in `val/sim/jobs.json` (each policy on `val/sim/pools/pool4.json`, about 6 s) and compares each whole output,
  wall clocks aside, with `val/sim/ref__*.out` (`val/same.py TAG --sim`).
- **Exactness:** `uv run val/owncheck.py BIN 3 "Yaeno Muteki"` and `uv run val/permcheck.py BIN 3` check the search
  against brute force on small owned sets; `uv run val/joint_check.py TAG` recomputes every deck result's odds
  independently. `uv run --group research val/milp_fixtures.py` recomputes the exact DP's reference fixtures (`pkg/loopopt/tests/fixtures/`)
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
  denied there; an `#[allow]` in the code needs `reason = "..."`. `unsafe_code` is denied crate-wide, with no exception: the wasm's exports
  (`web.rs`) and host imports (`career/clock.rs`) go through wasm-bindgen. A lint fix that could change floating-point evaluation
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
  turns on `.githooks/pre-commit`, which checks the formatting and types of the languages the commit stages, nothing
  else (budget 30 s, *Tests*): Rust, `cargo fmt --check` and native clippy with warnings denied (about 5 s after a
  change; a minute the first time in a new worktree); Python, ruff check and format on the staged files; the page's
  JS, Biome (lint and formatting) on the staged files and both `tsc` passes (about 10 s). It checks the working tree's
  files, not only the staged parts. The Tailwind class gate, knip, dependency-cruiser, the `@ts-expect-error` grep
  and the node tests (about 50 s together, which the hook had spent on every page commit) run in `check handback` and
  `lint`; the wasm clippy in `lint`.
- **Blame:** commits that only reformat, move or rename code are listed in `.git-blame-ignore-revs`; to have `git blame` skip them, once
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
link-preview tags' host. One-time setup: `cd webapp/deploy && npm ci && npx wrangler login`, then `npx wrangler pages project create
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
webapp/              the page: index.html, loop.html, about.html, partials/, src/ (styles and modules), the Vite
                     build (vite.config.mjs, vite/), public/, the browser suite, chibi/
pkg/loopopt/         the Rust solver (src/, its module map in docs/SOLVER.md; tests/; build.rs takes the training-hint
                     constants from research/results.json)
pkg/loopopt/export/  Python exporters, one set for both eras (`global` or `jp` argument; era.py)
pkg/loopopt/data/    the solver's and the page's data: global/ and jp/ (the Grand Masters era: the JP game database
                     cut at L'Arc's launch), each with data.json, web_skills.json, web_cards.json, career.json,
                     card_events.json, base.json, manual.json (hand-entered card events) and page.json (the page's
                     lists, export_page.py), plus upcoming.json for Global; and the files both eras share,
                     event_shares.json, gametora.json (GameTora's pinned versions), gametora_slugs.json (the page's
                     links to GameTora) and page_raceids.json
val/                 validation: jobs/ (the job files), jobs.json and slow.json (which run), reference outputs, checks
                     (solver.py: what they share, running the solver and reading and comparing its outputs);
                     plan/ and sim/ (the career planner's and the policy simulator's jobs and references)
research/            scripts computing RESEARCH.md's numbers; RESEARCH.md is rendered from docs/RESEARCH.template.md (`./build.sh research`; research/README.md)
docs/                USAGE, SOLVER, MODEL, RESEARCH, PERF and this file
game/                your game files (gitignored)
cache/               downloads: GameTora's files, Playwright's browsers, the runner tools and exports (gitignored)
```
