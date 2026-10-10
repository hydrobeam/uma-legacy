#!/bin/bash
# One entry point for every build step. Usage: ./build.sh [step...]   (no step = native wasm page)
#   data     Global + GM-era exports (needs pkg/loopopt/export/master.mdb, jp/export/master.mdb)
#   gametora GameTora's data files at the pinned versions (pkg/loopopt/gametora.json) into cache/gametora
#   events   card event caches from the GameTora files + manual entries (Global and both GM variants); runs gametora first
#   native   release binary + tests
#   pb       pairwise affinity ceilings (exact, from the native binary) stored in both data.json files; a stale
#            table is ignored by the solver (fingerprint), so this only saves time: rerun after data or solver changes
#   wasm     wasm + wasm-opt + example.json
#   page     webapp/dist: index.html (webapp/src bundled with esbuild, both eras' data), loopopt.wasm, img/
#   runners  each uma's mini model running, a sprite strip per uma into webapp/img/run (webapp/chibi/chibi.py: the game's
#            files as for images; fetches its third-party tools into cache/chibi on first use)
#   images   character icons and card art from the game's decrypted bundles into webapp/img ($UMA_DECRYPTED;
#            only new ids are extracted; pass --force through IMAGES_FORCE=1 to redo all)
#   browsers Playwright's Chromium and Firefox into webapp/node_modules (once; run by test, testq and runners)
#   test     browser regression suite on webapp/dist, served locally (webapp/nodebrowser.mjs; Chromium + Firefox): releases
#   testq    its fast tier (Chromium, each page's core path, under a minute): after each page change
#   serve    serve webapp/dist on http://localhost:8000 (PORT=...); the page fetches its wasm and images, so open it there
#   deploy   build the page and upload webapp/dist to Cloudflare Pages (CF_PAGES_PROJECT, default uma-legacy;
#            CF_PAGES_BRANCH=preview for a preview URL); refuses with uncommitted changes under webapp/ pkg/ jp/
#   val TAG  run the 24 validation jobs (shared deck, and the 23 deck jobs again with per-uma decks) in parallel with
#            the current binary into val/TAG__*.out and val/TAGpu__*.out, compared with val/ref__* and val/refpu__*
#   valq TAG the same without the three slow shared three-result jobs (val/slow.json): for iterating on a change
#   all      data events native pb wasm page (not images: it needs the game install)
# Python runs in the project's uv environment (pyproject.toml, uv.lock; .venv is created on first use).
set -euo pipefail
R="$(cd "$(dirname "$0")" && pwd)"; C="$R/pkg/loopopt"; W="$R/webapp"
PY=(uv run --quiet --project "$R" python)
export PLAYWRIGHT_BROWSERS_PATH=0   # Playwright's browsers live in webapp/node_modules, not in the home directory
WFLAGS="--enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int --enable-mutable-globals"
step_data() {
  (cd "$C/export" && "${PY[@]}" build_base.py && "${PY[@]}" export_rs.py && "${PY[@]}" export_web.py && "${PY[@]}" export_cards.py && "${PY[@]}" export_career.py ../data.json ../career.json && "${PY[@]}" upcoming.py)
  (cd "$R/jp/export" && "${PY[@]}" build_base.py && "${PY[@]}" export_rs.py && "${PY[@]}" export_web.py && "${PY[@]}" export_cards.py && "${PY[@]}" "$C/export/export_career.py" ../data.json ../career.json)
}
step_gametora() { "${PY[@]}" "$C/export/gametora.py" ${GAMETORA_LATEST:+--latest}; }
step_events() {
  step_gametora
  (cd "$C/export" && "${PY[@]}" build_card_events.py)
  (cd "$R/jp/export" && "${PY[@]}" build_card_events.py && "${PY[@]}" build_card_events_nodates.py)
}
step_native() { (cd "$C" && cargo build --release 2>&1 | tail -1 && cargo test --release 2>&1 | grep -E "^test result" | head -3); }
step_wasm() {
  step_npm
  (cd "$C" && RUSTC_BOOTSTRAP=1 RUSTFLAGS="-C panic=abort" cargo build --release --lib --target wasm32-unknown-unknown -Z build-std=std,panic_abort 2>&1 | tail -1)
  (cd "$W" && npx wasm-opt -O3 $WFLAGS "$C/target/wasm32-unknown-unknown/release/loopopt.wasm" -o loopopt.opt.wasm \
           && node example.mjs | tail -1)
}
step_browsers() { step_npm; (cd "$W" && npx playwright install chromium firefox); }
step_test() { step_browsers; node "$W/nodebrowser.mjs"; }
step_testq() { step_browsers; node "$W/nodebrowser.mjs" quick; }
step_serve() { node "$W/serve.mjs" "${PORT:-8000}"; }
# Cloudflare Pages, direct upload of webapp/dist (one-time setup: npx wrangler login; npx wrangler pages project create)
step_deploy() {
  local dirty; dirty=$(git -C "$R" status --porcelain --untracked-files=no -- webapp pkg jp)
  if [ -n "$dirty" ] && [ -z "${DEPLOY_DIRTY:-}" ]; then echo "uncommitted changes under webapp/ pkg/ jp/; commit first (or DEPLOY_DIRTY=1)"; echo "$dirty"; exit 1; fi
  step_page
  (cd "$W" && npx wrangler pages deploy dist --project-name "${CF_PAGES_PROJECT:-uma-legacy}" --branch "${CF_PAGES_BRANCH:-main}" \
     --commit-hash "$(git -C "$R" rev-parse HEAD)" --commit-message "$(git -C "$R" describe --tags --always)" --commit-dirty=$([ -n "${DEPLOY_DIRTY:-}" ] && echo true || echo false))
}
step_npm() { [ -x "$W/node_modules/.bin/esbuild" ] || (cd "$W" && npm ci --silent); }
step_page() { step_npm; step_gametora; (cd "$W" && "${PY[@]}" build.py); }
step_pb() { "${PY[@]}" "$C/export/add_pb.py" "$C/target/release/loopopt" "$C/data.json" "$R/jp/data.json"; }
step_runners() { step_browsers; "${PY[@]}" "$W/chibi/chibi.py" all; }
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
    gametora|data|events|native|pb|wasm|page|images|runners|browsers|test|testq|serve|deploy) "step_$1" ;;
    *) echo "unknown step $1"; exit 1 ;;
  esac; shift
done
