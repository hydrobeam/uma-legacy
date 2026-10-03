#!/bin/bash
# One entry point for every build step. Usage: ./build.sh [step...]   (no step = native wasm page)
#   data     Global + GM-era exports (needs pkg/loopopt/export/master.mdb, jp/export/master.mdb)
#   events   card event caches from the GameTora files + manual entries (Global and both GM variants)
#   native   release binary + tests
#   wasm     wasm + wasm-opt + example.json
#   page     webapp/dist/index.html (embeds wasm and both eras' data)
#   val TAG  run the 24 validation jobs (shared deck, then per-uma decks) with the current binary into
#            val/TAG__*.out and val/TAGpu__*.out, compared with val/ref__* and val/refpu__*
#   all      data events native wasm page
set -euo pipefail
R="$(cd "$(dirname "$0")" && pwd)"; C="$R/pkg/loopopt"; W="$R/webapp"
WFLAGS="--enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int --enable-mutable-globals"
step_data() {
  (cd "$C/export" && python3 build_base.py && python3 export_rs.py && python3 export_web.py && python3 export_cards.py)
  (cd "$R/jp/export" && python3 build_base.py && python3 export_rs.py && python3 export_web.py && python3 export_cards.py)
}
step_events() {
  (cd "$C/export" && python3 build_card_events.py)
  (cd "$R/jp/export" && python3 build_card_events.py && python3 build_card_events_nodates.py)
}
step_native() { (cd "$C" && cargo build --release 2>&1 | tail -1 && cargo test --release 2>&1 | grep -E "^test result" | head -3); }
step_wasm() {
  (cd "$C" && RUSTC_BOOTSTRAP=1 RUSTFLAGS="-C panic=abort" cargo build --release --lib --target wasm32-unknown-unknown -Z build-std=std,panic_abort 2>&1 | tail -1)
  (cd "$W" && npx wasm-opt -O3 $WFLAGS "$C/target/wasm32-unknown-unknown/release/loopopt.wasm" -o loopopt.opt.wasm \
           && node example.mjs | tail -1)
}
step_page() { (cd "$W" && python3 build.py); }
step_val() { local tag="${1:?val needs a tag}"
  bash "$R/val/runall.sh" "$C/target/release/loopopt" "$tag" > "$R/val/${tag}_times.txt"; python3 "$R/val/cmp2.py" "$tag" ref
  python3 "$R/val/run_pu.py" "$C/target/release/loopopt" "${tag}pu" > "$R/val/${tag}pu_times.txt"; python3 "$R/val/cmp2.py" "${tag}pu" refpu; }
[ $# -eq 0 ] && set -- native wasm page
while [ $# -gt 0 ]; do
  case "$1" in
    all) step_data; step_events; step_native; step_wasm; step_page ;;
    val) step_val "${2:-}"; shift ;;
    data|events|native|wasm|page) "step_$1" ;;
    *) echo "unknown step $1"; exit 1 ;;
  esac; shift
done
