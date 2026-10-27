#!/bin/bash
# One entry point for every build step. Usage: ./build.sh [step...]   (no step = native wasm art page)
#   data     Global + GM-era exports (needs game/global/master.mdb, game/jp/master.mdb)
#   gametora GameTora's data files at the pinned versions (pkg/loopopt/data/gametora.json) into cache/gametora
#   events   card event files from the GameTora files + manual entries (both eras); runs gametora first
#   native   release binary, clippy (as in lint), tests
#   lint     Rust: cargo fmt --check, clippy on every target and on the wasm lib, warnings denied; shellcheck on the scripts;
#            Python: ruff check and ruff format --check (pyproject.toml); the page's JS: Biome lint and formatting, warnings denied (webapp/biome.json), and tsc's checkJs (webapp/tsconfig.json)
#   hooks    use the tracked git hooks (.githooks: the pre-commit formatting check); once per clone
#   pb       pairwise affinity ceilings (exact, from the native binary) stored in both data.json files; a stale
#            table is ignored by the solver (fingerprint), so this only saves time: rerun after data or solver changes
#   wasm     wasm + wasm-opt + example.json
#   page     webapp/dist: index.html (webapp/src bundled with esbuild, both eras' data), loopopt.wasm, img/
#   runners  each uma's mini model running, a sprite strip per uma into webapp/img/run (webapp/chibi/chibi.py: the game's
#            files as for images; fetches its third-party tools into cache/chibi on first use)
#   images   character icons and card art from the game's decrypted bundles (game/) into webapp/img
#            (only new ids are extracted; pass --force through IMAGES_FORCE=1 to redo all)
#   browsers Playwright's Chromium and Firefox into webapp/node_modules (once; run by test, testq and runners)
#   test     browser regression suite on webapp/dist, served locally (webapp/nodebrowser.mjs; Chromium + Firefox): releases
#   testq    its fast tier (Chromium, each page's core path, under a minute): after each page change
#   serve    serve webapp/dist on http://localhost:8000 (PORT=...); the page fetches its wasm and images, so open it there
#   deploy   build the page and upload webapp/dist to Cloudflare Pages (CF_PAGES_PROJECT, default uma-legacy;
#            CF_PAGES_BRANCH=preview for a preview URL); refuses with uncommitted changes under webapp/ pkg/
#   val TAG  run the 24 validation jobs (20 deck, 4 career; the 20 deck jobs again with per-uma decks) in parallel with
#            the current binary into val/TAG__*.out and val/TAGpu__*.out, compared with val/ref__* and val/refpu__*
#   valq TAG the same without the three slow shared three-result jobs (val/slow.json): for iterating on a change
#   research compute the research numbers (research/run_all.py -> research/results.json) and render docs/RESEARCH.md
#            from docs/RESEARCH.template.md; without the run records (research/data/) the committed results are kept
#   fullcheck before a commit that stages solver, data, validation or page code (.githooks/fullcheck-paths): native
#            (every cargo test, plan_roll's fixture included), lint, wasm, page, the full browser suite, val and planval
#            against their references, mono where the career planner changed; then stamps the staged tree, which the
#            pre-commit hook requires. Re-pin references (val, val/plan, plan_roll) and run research first, as AGENTS.md says
#   art      images and runners when their game files are in game/global/ (skipped, with a note, when not)
#   all      data events native pb wasm art page
# Python runs in the project's uv environment (pyproject.toml, uv.lock; .venv is created on first use).
set -euo pipefail
R="$(cd "$(dirname "$0")" && pwd)"; C="$R/pkg/loopopt"; W="$R/webapp"
PY=(uv run --quiet --project "$R" python)
export PLAYWRIGHT_BROWSERS_PATH=0   # Playwright's browsers live in webapp/node_modules, not in the home directory
WFLAGS="--enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int --enable-mutable-globals"
# Requirements: each step checks what it needs first and names everything missing before it stops.
GLOBAL_MDB="game/global/master.mdb: the Global client's game database"
JP_MDB="game/jp/master.mdb: the JP client's game database (Grand Masters era)"
MISSING=()
need_cmd() { command -v "$1" >/dev/null 2>&1 || MISSING+=("$1 ($2)"); }
need_file() { [ -e "$R/$1" ] || MISSING+=("$1${2:+: $2}"); }
need() { local f; for f in "$@"; do need_file "${f%%:*}" "$(printf %s "$f" | sed -n 's/^[^:]*: //p')"; done; }
have() { local f; for f in "$@"; do [ -e "$R/${f%%:*}" ] || return 1; done; }
missing_of() { local f; for f in "$@"; do [ -e "$R/${f%%:*}" ] || printf '%s, ' "${f%%:*}"; done | sed 's/, $//'; }
check() {
  if [ ${#MISSING[@]} -gt 0 ]; then
    { echo "$1: cannot run, missing:"; printf '  - %s\n' "${MISSING[@]}"; echo "  (docs/BUILD.md, What is needed)"; } >&2; exit 1
  fi
}
TOOLS_RUST="https://rustup.rs"; TOOLS_NODE="Node.js with npm"; TOOLS_UV="https://docs.astral.sh/uv/"
step_data() {
  MISSING=(); need_cmd uv "$TOOLS_UV"; need "$GLOBAL_MDB" "$JP_MDB"; check data
  (cd "$C/export" && for era in global jp; do
     "${PY[@]}" build_base.py $era && "${PY[@]}" export_rs.py $era && "${PY[@]}" export_web.py $era \
       && "${PY[@]}" export_cards.py $era && "${PY[@]}" export_career.py $era || exit 1
   done && "${PY[@]}" upcoming.py global)
}
step_gametora() { MISSING=(); need_cmd uv "$TOOLS_UV"; check gametora; "${PY[@]}" "$C/export/gametora.py" ${GAMETORA_LATEST:+--latest}; }
step_events() {
  MISSING=(); need_cmd uv "$TOOLS_UV"; need "$GLOBAL_MDB" "$JP_MDB"; check events
  step_gametora
  (cd "$C/export" && "${PY[@]}" build_card_events.py global && "${PY[@]}" build_card_events.py jp)
}
step_native() { MISSING=(); need_cmd cargo "$TOOLS_RUST"; check native; (cd "$C" && cargo build --release 2>&1 | tail -1 && clippy_all && { rc=0; out=$(cargo test --release 2>&1) || rc=$?; printf '%s\n' "$out" | grep -E "^test result|FAILED|panicked|^error"; exit $rc; }); }   # every test binary's result line, any failure, and the tests' exit status
# clippy as ./build.sh lint runs it (lints chosen in pkg/loopopt/Cargo.toml): quiet unless it finds something
clippy_all() { cargo clippy --quiet --all-targets -- -D warnings && cargo clippy --quiet --target wasm32-unknown-unknown --lib -- -D warnings; }
step_lint() { MISSING=(); need_cmd cargo "$TOOLS_RUST"; need_cmd uv "$TOOLS_UV"; need_cmd node "$TOOLS_NODE"; need_cmd npm "$TOOLS_NODE"; check lint; step_npm
  (cd "$C" && cargo fmt --check && clippy_all) && uvx --quiet --from shellcheck-py shellcheck "$R/build.sh" "$R/.githooks/pre-commit" \
    && (cd "$R" && uv run --quiet ruff check && uv run --quiet ruff format --check) \
    && (cd "$W" && npx --no-install biome check --error-on-warnings --diagnostic-level=warn && npx --no-install tsc -p .) \
    && echo "lint: clean"; }
step_hooks() { git -C "$R" config core.hooksPath .githooks && echo "hooks: core.hooksPath = .githooks (every worktree of this clone)"; }
step_wasm() {
  MISSING=(); need_cmd cargo "$TOOLS_RUST"; need_cmd rustup "$TOOLS_RUST: adds the wasm target"; need_cmd node "$TOOLS_NODE"; need_cmd npm "$TOOLS_NODE"; check wasm
  step_npm
  (cd "$C" && RUSTC_BOOTSTRAP=1 RUSTFLAGS="-C panic=abort" cargo build --release --lib --target wasm32-unknown-unknown -Z build-std=std,panic_abort 2>&1 | tail -1)
  # -ocimfs=0: wasm-opt inlines no function for having one caller (by default it inlines any such function, whatever its
  # size, undoing #[inline(never)]): a loop split into its own function stays one, so V8 tiers it up (docs/PERF.md)
  # wasm-opt is nearly all of an unchanged rebuild (about a minute): skipped when the compiled wasm, the flags and
  # wasm-opt's version are those of the last optimised build (webapp/.loopopt.wasm.key, untracked); the example (1.5 s,
  # it reads the data and today's date) always runs
  local raw="$C/target/wasm32-unknown-unknown/release/loopopt.wasm" key
  key=$({ cat "$raw"; printf '%s|%s' "$WFLAGS" "$(node -p "require('$W/node_modules/binaryen/package.json').version")"; } | sha256sum | cut -d' ' -f1)
  if [ -f "$W/loopopt.opt.wasm" ] && [ "$(cat "$W/.loopopt.wasm.key" 2>/dev/null)" = "$key" ]; then echo "wasm-opt: the wasm is unchanged, skipped"
  else
    # shellcheck disable=SC2086  # WFLAGS holds several flags: split on purpose
    (cd "$W" && npx wasm-opt -ocimfs=0 -O3 $WFLAGS "$raw" -o loopopt.opt.wasm) && echo "$key" > "$W/.loopopt.wasm.key"
  fi
  (cd "$W" && node example.mjs | tail -1)
}
step_browsers() { step_npm; local B="$W/node_modules/playwright-core/.local-browsers"
  ls -d "$B"/chromium-* "$B"/firefox-* >/dev/null 2>&1 || (cd "$W" && npx playwright install chromium firefox); }
step_test() { MISSING=(); need "webapp/dist/index.html: the built page (./build.sh)"; check test; step_browsers; node "$W/nodebrowser.mjs"; }
step_testq() { MISSING=(); need "webapp/dist/index.html: the built page (./build.sh)"; check testq; step_browsers; node "$W/nodebrowser.mjs" quick; }
step_serve() { MISSING=(); need_cmd node "$TOOLS_NODE"; need "webapp/dist/index.html: the built page (./build.sh)"; check serve; node "$W/serve.mjs" "${PORT:-8000}"; }
# Cloudflare Pages, direct upload of webapp/dist (one-time setup: npx wrangler login; npx wrangler pages project create)
step_deploy() {
  local dirty; dirty=$(git -C "$R" status --porcelain --untracked-files=no -- webapp pkg research/results.json)
  if [ -n "$dirty" ] && [ -z "${DEPLOY_DIRTY:-}" ]; then echo "uncommitted changes under webapp/ pkg/ or research/results.json; commit first (or DEPLOY_DIRTY=1)"; echo "$dirty"; exit 1; fi
  step_wasm; step_page   # the solver it ships is built from this tree, never one left from another
  (cd "$W" && npx wrangler pages deploy dist --project-name "${CF_PAGES_PROJECT:-uma-legacy}" --branch "${CF_PAGES_BRANCH:-main}" \
     --commit-hash "$(git -C "$R" rev-parse HEAD)" --commit-message "$(git -C "$R" describe --tags --always)" --commit-dirty="$([ -n "${DEPLOY_DIRTY:-}" ] && echo true || echo false)")
}
step_npm() { MISSING=(); need_cmd node "$TOOLS_NODE"; need_cmd npm "$TOOLS_NODE"; check npm
  # reinstall when the lockfile is newer than what npm last installed (node_modules/.package-lock.json), so a pulled
  # dependency change (a new or bumped package) is never checked against a stale node_modules
  [ -x "$W/node_modules/.bin/esbuild" ] && [ ! "$W/package-lock.json" -nt "$W/node_modules/.package-lock.json" ] || (cd "$W" && npm ci --silent); }
step_page() { MISSING=(); need_cmd uv "$TOOLS_UV"; need_cmd node "$TOOLS_NODE"; need_cmd npm "$TOOLS_NODE"; need "webapp/loopopt.opt.wasm: the solver (./build.sh wasm)"; check page; step_npm; (cd "$W" && "${PY[@]}" build.py); }
step_pb() { MISSING=(); need_cmd uv "$TOOLS_UV"; need "pkg/loopopt/target/release/loopopt: the solver (./build.sh native)"; check pb; "${PY[@]}" "$C/export/add_pb.py" "$C/target/release/loopopt" "$C/data/global/data.json" "$C/data/jp/data.json"; }
step_runners() { MISSING=(); need_cmd uv "$TOOLS_UV"; [ -e "$R/cache/chibi/tools/xml/usr/lib/libxml2.so.2" ] || { need_cmd tar "to unpack libxml2 once"; need_cmd zstd "to unpack libxml2 once"; }; need "${RUNNER_FILES[@]}"; check runners; step_browsers; "${PY[@]}" "$W/chibi/chibi.py" all; }
step_images() { MISSING=(); need_cmd uv "$TOOLS_UV"; need "${IMAGE_FILES[0]}"; check images; art_notes; "${PY[@]}" "$W/images.py" ${IMAGES_FORCE:+--force}; }
IMAGE_FILES=("game/global/decrypted: the Global client's asset bundles, decrypted, by asset path")
RUNNER_FILES=("game/global/decrypted: the Global client's asset bundles, decrypted, by asset path" "game/global/meta.db: the Global client's asset index as plain SQLite" "$GLOBAL_MDB")
art_notes() {   # optional inputs of images: missing ones only lose their pictures
  [ -e "$R/game/global/resources.assets" ] || echo "images: no game/global/resources.assets (the install's built-in assets): the game's buttons and check box keep their drawn fallbacks"
  [ -d "$R/game/jp/bundles" ] || echo "images: no game/jp/bundles (the JP client's bundles): the Grand Masters art and 14 JP-only skill icons keep their fallbacks"
}
step_art() {   # in the default build: runs what the game files in game/ allow, says what it skips
  if have "${IMAGE_FILES[@]}"; then step_images; else echo "images: skipped, missing $(missing_of "${IMAGE_FILES[@]}") (the page shows initials)"; fi
  if have "${RUNNER_FILES[@]}"; then step_runners; else echo "runners: skipped, missing $(missing_of "${RUNNER_FILES[@]}") (the page shows a spinner)"; fi
}
step_val() { local tag="${1:?val needs a tag}"; shift || true
  MISSING=(); need_cmd uv "$TOOLS_UV"; need "pkg/loopopt/target/release/loopopt: the solver (./build.sh native)"; check val
  "${PY[@]}" "$R/val/runval.py" "$C/target/release/loopopt" "$tag" "$@"
  "${PY[@]}" "$R/val/cmp2.py" "$tag" ref; "${PY[@]}" "$R/val/cmp2.py" "${tag}pu" refpu; }
step_research() { uv run --quiet --project "$R" --group research python "$R/research/run_all.py" && uv run --quiet --project "$R" --group research python "$R/research/render.py"; }
step_fullcheck() {   # every check a commit staging .githooks/fullcheck-paths needs, then the staged tree's stamp for the hook
  MISSING=(); need_cmd uv "$TOOLS_UV"; check fullcheck
  local paths tree; mapfile -t paths < "$R/.githooks/fullcheck-paths"
  # (the stamp is of the staged tree, so what is checked must be exactly what is staged)
  if ! git -C "$R" diff --quiet -- "${paths[@]}" || [ -n "$(git -C "$R" ls-files --others --exclude-standard -- "${paths[@]}")" ]; then
    echo "fullcheck: stage or set aside the unstaged changes and untracked files under .githooks/fullcheck-paths first" >&2; exit 1
  fi
  tree=$(git -C "$R" write-tree)
  step_native; step_lint; step_wasm; step_page; step_test; step_val fullcheck
  "${PY[@]}" "$R/val/planval.py" "$C/target/release/loopopt" fullcheck
  # (mono re-plans every val pool with rolled veterans: only where the career planner changed)
  git -C "$R" diff --cached --quiet HEAD -- pkg/loopopt/src/career || "${PY[@]}" "$R/val/mono.py" "$C/target/release/loopopt"
  [ "$(git -C "$R" write-tree)" = "$tree" ] || { echo "fullcheck: the staged tree changed while it ran; run it again" >&2; exit 1; }
  printf '%s\n' "$tree" > "$(git -C "$R" rev-parse --git-path fullcheck)"
  echo "fullcheck: passed; staged tree $tree stamped for the pre-commit hook"
}
[ $# -eq 0 ] && set -- native wasm art page
while [ $# -gt 0 ]; do
  case "$1" in
    all) step_data; step_events; step_native; step_pb; step_wasm; step_art; step_page ;;
    val) step_val "${2:-}"; shift ;;
    valq) step_val "${2:-}" --quick; shift ;;
    gametora|data|events|native|lint|hooks|pb|wasm|page|art|images|runners|browsers|test|testq|serve|deploy|research|fullcheck) "step_$1" ;;
    *) echo "unknown step $1"; exit 1 ;;
  esac; shift
done
