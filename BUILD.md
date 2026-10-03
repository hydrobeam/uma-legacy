# Build

`./build.sh [step...]` from this folder. No step = `native wasm page`.

| Step | Does | Needs |
|---|---|---|
| `data` | Global + GM-era exports (data.json, web_skills.json, web_cards.json) | `pkg/loopopt/export/master.mdb`, `jp/export/master.mdb` |
| `events` | card event caches from `pkg/loopopt/gametora/` + manual entries (Global, GM, GM without first-year dates) | Global master.mdb |
| `native` | release binary, tests | cargo |
| `wasm` | wasm32 build (build-std), wasm-opt -O3, example.json | rust-src, node (`webapp/node_modules`) |
| `page` | `webapp/dist/index.html` with wasm and both eras embedded | |
| `val TAG` | 24 validation jobs, shared deck then per-uma decks, compared with `val/ref__*` and `val/refpu__*` | native |

Browser checks: `cd webapp && python3 pwdeck.py && python3 pwera.py` (Playwright, Chromium). WASM timings: `node nodebench.mjs loopopt.opt.wasm global pal_A2 pal_U pal_UF pal_F`.

Manual card entries (pal and group cards, written by hand; the build script decodes only SR/SSR chains): Global ones live in `pkg/loopopt/card_events.json` (every entry except the generated ones, `src` exactly "gametora training_events", survives `events`), GM ones in `jp/export/manual_gm*.json`.

`val/ref__*` holds the current reference outputs; when a data change moves results on purpose, rerun `val`, check the diff, then copy the new outputs over `ref__*` in the same commit. Releases are tagged `vN` to match the published page version.

Zips (used before the repo moved to GitHub) are a clone of this repository (history and tags included) plus the built `Legacy Loop Planner.html`.
Before zipping: clone, `git remote remove origin`, `git gc --aggressive`, copy in the built page.

Exactness checks against brute force on small owned sets: `python3 val/owncheck.py BIN 3 "Yaeno Muteki"` (character rule, shared deck) and `python3 val/permcheck.py BIN 3` (per-uma decks; `permcheck_big.py` is the 8-uma, 10-card version, ~25 min).

Audit the manual entries against GameTora after any change to them or to the GameTora caches: `python3 val/event_audit.py` (exit 1 lists every target hint a pal or group card can give that its entry does not account for).

When every value is expected to scale by one factor (a constant change), compare a run with `python3 val/cmp_ratio.py TAG ref` instead of `cmp2.py`.
