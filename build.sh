#!/bin/bash
# One entry point for every build step. Usage: ./build.sh [step...]   (no step = native wasm page)
#   data     Global + GM-era exports (needs pkg/loopopt/export/master.mdb, jp/export/master.mdb)
#   events   card event caches from the GameTora files + manual entries (Global and both GM variants)
#   native   release binary + tests
#   pb       pairwise affinity ceilings (exact, from the native binary) stored in both data.json files; a stale
#            table is ignored by the solver (fingerprint), so this only saves time: rerun after data or solver changes
#   wasm     wasm + wasm-opt + example.json
#   page     webapp/dist: index.html (webapp/src bundled with esbuild, both eras' data), loopopt.wasm, img/
#   images   character icons and card art from the game's decrypted bundles into webapp/img ($UMA_DECRYPTED;
#            only new ids are extracted; pass --force through IMAGES_FORCE=1 to redo all)
#   test     browser regression suite on webapp/dist, served locally (webapp/nodebrowser.mjs; Chromium + Firefox)
#   serve    serve webapp/dist on http://localhost:8000 (PORT=...); the page fetches its wasm and images, so open it there
#   val TAG  run the 24 validation jobs (shared deck, and the 23 deck jobs again with per-uma decks) in parallel with
#            the current binary into val/TAG__*.out and val/TAGpu__*.out, compared with val/ref__* and val/refpu__*
#   valq TAG the same without the three slow shared three-result jobs (val/slow.json): for iterating on a change
#   all      data events native pb wasm page (not images: it needs the game install)
# Python runs in the project's uv environment (pyproject.toml, uv.lock; .venv is created on first use).
set -euo pipefail
R="$(cd "$(dirname "$0")" && pwd)"; C="$R/pkg/loopopt"; W="$R/webapp"
PY=(uv run --quiet --project "$R" python)
WFLAGS="--enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int --enable-mutable-globals"
step_data() {
  (cd "$C/export" && "${PY[@]}" build_base.py && "${PY[@]}" export_rs.py && "${PY[@]}" export_web.py && "${PY[@]}" export_cards.py)
  (cd "$R/jp/export" && "${PY[@]}" build_base.py && "${PY[@]}" export_rs.py && "${PY[@]}" export_web.py && "${PY[@]}" export_cards.py)
}
step_events() {
  (cd "$C/export" && "${PY[@]}" build_card_events.py)
  (cd "$R/jp/export" && "${PY[@]}" build_card_events.py && "${PY[@]}" build_card_events_nodates.py)
}
step_native() { (cd "$C" && cargo build --release 2>&1 | tail -1 && cargo test --release 2>&1 | grep -E "^test result" | head -3); }
step_wasm() {
  (cd "$C" && RUSTC_BOOTSTRAP=1 RUSTFLAGS="-C panic=abort" cargo build --release --lib --target wasm32-unknown-unknown -Z build-std=std,panic_abort 2>&1 | tail -1)
  (cd "$W" && npx wasm-opt -O3 $WFLAGS "$C/target/wasm32-unknown-unknown/release/loopopt.wasm" -o loopopt.opt.wasm \
           && node example.mjs | tail -1)
}
step_test() { node "$W/nodebrowser.mjs"; }
step_serve() { node "$W/serve.mjs" "${PORT:-8000}"; }
step_page() { [ -x "$W/node_modules/.bin/esbuild" ] || (cd "$W" && npm ci --silent); (cd "$W" && "${PY[@]}" build.py); }
step_pb() { "${PY[@]}" "$C/export/add_pb.py" "$C/target/release/loopopt" "$C/data.json" "$R/jp/data.json"; }
step_images() { "${PY[@]}" "$W/images.py" ${IMAGES_FORCE:+--force}; }
step_val() { local tag="${1:?val needs a tag}"; shift || true
  "${PY[@]}" "$R/val/runval.py" "$C/target/release/loopopt" "$tag" "$@"
  "${PY[@]}" "$R/val/cmp2.py" "$tag" ref; "${PY[@]}" "$R/val/cmp2.py" "${tag}pu" refpu; }
[ $# -eq 0 ] && set -- native wasm page
while [ $# -gt 0 ]; do
  case "$1" in
    all) step_data; step_events; step_native; step_pb; step_wasm; step_page ;;
    val) step_val "${2:-}"; shift ;;
    valq) step_val "${2:-}" --quick; shift ;;
    data|events|native|pb|wasm|page|images|test|serve) "step_$1" ;;
    *) echo "unknown step $1"; exit 1 ;;
  esac; shift
done
