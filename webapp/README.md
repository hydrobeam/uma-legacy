# Legacy Loop Planner (web)

Single static page. The Rust solver (`../pkg/loopopt`) is compiled to WebAssembly and runs in a Web Worker.
A host whose security policy refuses to compile WebAssembly can't run it; the page then says so. (A pure-JS
wasm2js fallback existed until v23; it was 8.8 MB of the page and 3.6-6x slower, so it was dropped.)

## Build
```bash
# 1. Rust -> wasm (needs rust-src for build-std; RUSTC_BOOTSTRAP lets stable use -Z build-std)
cd ../pkg/loopopt
RUSTC_BOOTSTRAP=1 RUSTFLAGS="-C panic=abort" cargo build --release --lib \
  --target wasm32-unknown-unknown -Z build-std=std,panic_abort
# (with the wasm32 target installed via rustup you can drop RUSTC_BOOTSTRAP and -Z build-std)

# 2. optimise (binaryen from npm)
cd ../../webapp && npm i binaryen
npx wasm-opt -O3 --enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int --enable-mutable-globals \
  ../pkg/loopopt/target/wasm32-unknown-unknown/release/loopopt.wasm -o loopopt.opt.wasm

# 3. data (after a game update): re-export from the Global master.mdb
cd ../pkg/loopopt/export && uv run build_base.py && uv run export_rs.py && uv run export_web.py

# 4. example result shown on first load, then assemble dist/ (index.html, loopopt.wasm, img/); serve it with ./build.sh serve
cd ../../../webapp && npm ci && node example.mjs && uv run build.py
```

## Deploy (Cloudflare Pages)
`./build.sh deploy` uploads `dist/` with wrangler (pinned in `package.json`); see BUILD.md for the one-time login and
project creation. The build writes `dist/_headers` (noindex, 5-minute cache) and `dist/robots.txt`. No other headers
are needed: the page uses plain Web Workers, not SharedArrayBuffer.

## Files
- `index.template.html` the page's markup, with placeholders build.py fills
- `src/style.css` the stylesheet; `src/*.js` the page script as ES modules, bundled by esbuild into one inline script:
  `main.js` (entry, wiring), `settings.js` (saved settings, one versioned object), `data.js` (embedded data), `state.js`,
  `search.js` (auto-search, worker), `roster.js`, `targets.js`, `scenario.js` (scenario picker), `cards.js`, `sortmenu.js`, `results.js`, `agenda.js`
  (race schedule popup), `affinity.js`, `popover.js` (deck card / member panel), `io.js` (export/import), `images.js`, `dom.js`
- `worker.template.js` solver worker (WebAssembly)
- `build.py` bundles `src/` with esbuild (`node_modules/.bin/esbuild`; `npm ci`) and assembles `dist/`: `index.html` with
  the stylesheet, script, Global data and example inlined, `loopopt.wasm`, `data-gm.json` / `engine-gm.json` (GM era,
  fetched only when selected), and `img/`
- `serve.mjs` a minimal static server for `dist/` (`./build.sh serve`; also used by the browser suite)
- `images.py` extracts character icons, card art, skill icons, race banners and scenario logos from the game's decrypted asset bundles into `img/` (`./build.sh images`)
- `example.mjs` precomputes the example shown before the first search
