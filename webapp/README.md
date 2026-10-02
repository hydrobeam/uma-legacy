# Legacy Loop Planner (web)

Single static page. The Rust solver (`../pkg/loopopt`) is compiled to WebAssembly and runs in a Web Worker;
if a host's security policy refuses to compile WebAssembly, the worker falls back to a pure-JS build of the
same code (wasm2js), about 6x slower.

## Build
```bash
# 1. Rust -> wasm (needs rust-src for build-std; RUSTC_BOOTSTRAP lets stable use -Z build-std)
cd ../pkg/loopopt
RUSTC_BOOTSTRAP=1 RUSTFLAGS="-C panic=abort" cargo build --release --lib \
  --target wasm32-unknown-unknown -Z build-std=std,panic_abort
# (with the wasm32 target installed via rustup you can drop RUSTC_BOOTSTRAP and -Z build-std)

# 2. optimise + JS fallback (binaryen from npm)
cd ../../webapp && npm i binaryen
npx wasm-opt -O3 --enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int --enable-mutable-globals \
  ../pkg/loopopt/target/wasm32-unknown-unknown/release/loopopt.wasm -o loopopt.opt.wasm
npx wasm2js -O2 --enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int --enable-mutable-globals \
  loopopt.opt.wasm -o loopopt.wasm2js.mjs

# 3. data (after a game update): re-export from the Global master.mdb
cd ../pkg/loopopt/export && python3 build_base.py && python3 export_rs.py && python3 export_web.py

# 4. example result shown on first load, then assemble dist/index.html
cd ../../../webapp && node example.mjs && python3 build.py
```

## Deploy (Cloudflare Pages)
Upload `dist/` as the build output (or point Pages at this folder with build output `dist`). No special headers
are needed: the page uses plain Web Workers, not SharedArrayBuffer.

## Files
- `index.template.html` UI (roster, target skills, rank, results, schedules)
- `worker.template.js` solver worker (WebAssembly, JS fallback spliced in at build)
- `build.py` assembles `dist/index.html` with the wasm, fallback, data and example embedded
- `example.mjs` precomputes the example shown before the first search
