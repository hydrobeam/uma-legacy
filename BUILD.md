# Build

`./build.sh [step...]` from this folder. No step = `native wasm page`.

| Step | Does | Needs |
|---|---|---|
| `data` | Global + GM-era exports (data.json, web_skills.json, web_cards.json) | `pkg/loopopt/export/master.mdb`, `jp/export/master.mdb` |
| `events` | card event caches from `pkg/loopopt/gametora/` + manual entries (Global, GM, GM without first-year dates) | Global master.mdb |
| `native` | release binary, tests | cargo |
| `wasm` | wasm32 build (build-std), wasm-opt -O3, wasm2js fallback, example.json | rust-src, node (`webapp/node_modules`) |
| `page` | `webapp/dist/index.html` with wasm, fallback and both eras embedded | |
| `val TAG` | 24 validation jobs into `val/TAG__*.out`, compared with `val/ref__*` | native |

Browser checks: `cd webapp && python3 pwdeck.py && python3 pwera.py` (Playwright, Chromium). WASM timings: `node nodebench.mjs loopopt.opt.wasm global pal_A2 pal_U pal_UF pal_F`.

Manual card entries: Global ones live in `pkg/loopopt/card_events.json` (entries whose `src` does not start with lowercase "gametora" survive `events`), GM ones in `jp/export/manual_gm*.json`.

`val/ref__*` holds the current reference outputs; when a data change moves results on purpose, rerun `val`, check the diff, then copy the new outputs over `ref__*` in the same commit. Releases are tagged `vN` to match the published page version.
