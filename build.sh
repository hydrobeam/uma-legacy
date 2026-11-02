#!/bin/bash
# One entry point for every build step: ./build.sh [step...]. The steps are the table STEPS below, which
# `./build.sh help` prints with the default build and `all`; docs/BUILD.md, Steps, has the detail.
# Python runs in the project's uv environment (pyproject.toml, uv.lock; .venv is created on first use).
set -euo pipefail
R="$(cd "$(dirname "$0")" && pwd)"
C="$R/pkg/loopopt"
W="$R/webapp"
# A linked worktree (git worktree add) uses the main checkout's cache/: Playwright's browsers, the recorded solver
# replies (keyed by the solver's bytes, so lanes share what matches), GameTora's files, wasm-bindgen and the TLS
# certificate, through a symlink made on first use, so a new worktree downloads and records nothing twice
share_cache() {
  local common main
  common=$(git -C "$R" rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || return 0
  main=$(dirname "$common")
  if [ "$main" != "$R" ] && [ ! -e "$R/cache" ] && [ -d "$main/cache" ]; then
    ln -s "$main/cache" "$R/cache"
    echo "cache: this worktree uses the main checkout's ($main/cache)"
  fi
}
share_cache
BIN="$C/target/release/loopopt"
# the solver as the checks build it ([profile.checks], pkg/loopopt/Cargo.toml): what val, valq, mono and lint run
CHECK_BIN="$C/target/checks/loopopt"
RAW_WASM="$C/target/wasm32-unknown-unknown/release/loopopt.wasm"
# wasm-bindgen's output: the wasm with its glue's exports (BOUND_WASM, wasm-opt's input) and the glue itself, whose
# loopopt.js and loopopt.d.ts the wasm step copies into webapp/src/gen (tracked, so tsc and knip see them unbuilt)
BINDGEN="$C/target/bindgen"
BOUND_WASM="$BINDGEN/loopopt_bg.wasm"
WASM_BINDGEN="$R/cache/wasm-bindgen/bin/wasm-bindgen"
PY=(uv run --quiet --project "$R" python)
PYART=(uv run --quiet --project "$R" --group art python)   # the art steps: UnityPy and Pillow (group art)
# Playwright's browsers: in the repository, outside node_modules (npm ci empties it)
export PLAYWRIGHT_BROWSERS_PATH="$R/cache/ms-playwright"
# -ocimfs=0: wasm-opt inlines no function for having one caller (by default it inlines any such function, whatever its
# size, undoing #[inline(never)]): a loop split into its own function stays one, so V8 tiers it up (docs/PERF.md).
# wasm-opt reads the wasm's features from the target_features section rustc writes. Its skip key covers these flags.
WASM_OPT_FLAGS=(-ocimfs=0 -O3)

# name|argument|sets (all: in `all`; default: in a bare ./build.sh)|what it does
STEPS=(
  "data||all|the Global and GM-era exports from game/global/master.mdb and game/jp/master.mdb"
  "gametora|||GameTora's data files at the pinned versions into cache/gametora (GAMETORA_LATEST=1 moves the pins)"
  "events||all|each era's card event file from GameTora's files and the manual entries; runs gametora first"
  "native||all default|clippy (as lint runs it), then cargo test in the checks profile"
  "pb||all|the solver, then pairwise affinity ceilings into both data.json; rerun after data or solver changes"
  "wasm||all default|the wasm32 build, wasm-bindgen's glue, wasm-opt (skipped when its key matches), webapp/example.json"
  "art||all default|images and runners when their game files are in game/global/ (each skipped, with a note, if not)"
  "page||all default|webapp/dist (Vite): the pages, webapp/src bundled, both eras' data, the wasm, img/"
  "images|||icons and card art from the game's decrypted bundles into webapp/img (IMAGES_FORCE=1 redoes all)"
  "runners|||each uma's running mini model as a sprite strip into webapp/img/run (webapp/chibi/chibi.py)"
  "lint|||every formatter and linter, warnings denied: Rust, shell, Python, the page's JS; the generated TypeScript current (Rust only when its inputs differ from main's)"
  "check|[TIER]||the checks a change needs, by what differs from main: edit, handback (the default), merge or full (docs/BUILD.md, Tests)"
  "hooks|||use the tracked git hooks (.githooks: the pre-commit check); once per clone"
  "browsers|||Playwright's Chromium and Firefox at the pinned revisions into cache/ms-playwright"
  "test|||the page, then the browser suite on it (Chromium, and Firefox on the @both groups): releases"
  "testq|||the page, then the suite's fast tier (Chromium, the @quick tests, about 30 s): after each page change"
  "serve|||webapp/dist on https://localhost:8000 over HTTP/2 (vite preview; PORT=...; a self-signed certificate)"
  "deploy|||wasm and page, then webapp/dist to Cloudflare Pages; refuses with uncommitted changes"
  "val|TAG||the solver, then every validation check against the references (about 7 CPU-min)"
  "valq|TAG||val without the slow jobs, joint_check and planval's proxy gate (about 2.5 CPU-min)"
  "mono|||the solver, then the planner's monotonicity check (val/mono.py, 10-20 CPU-min: in the background)"
  "research|||research/run_all.py into research/results.json, then docs/RESEARCH.md from its template"
  "prune-replies|||delete the recorded solver replies (cache/solver-replies) of every solver and data but this tree's"
  "all|||the steps marked all, in the table's order"
  "help|||this list"
)

# ---- requirements: a step names everything it is missing before it stops ----
GLOBAL_MDB="game/global/master.mdb: the Global client's game database"
JP_MDB="game/jp/master.mdb: the JP client's game database (Grand Masters era)"
NEED_DIST="webapp/dist/index.html: the built page (./build.sh page)"
NEED_OPT_WASM="webapp/loopopt.opt.wasm: the solver (./build.sh wasm)"
DECRYPTED="game/global/decrypted: the Global client's asset bundles, decrypted, by asset path"
IMAGE_FILES=("$DECRYPTED")
RUNNER_FILES=("$DECRYPTED" "game/global/meta.db: the Global client's asset index as plain SQLite" "$GLOBAL_MDB")
MISSING=()
need_cmd() { command -v "$1" >/dev/null 2>&1 || MISSING+=("$1 ($2)"); }
need_uv() { need_cmd uv "https://docs.astral.sh/uv/"; }
need_cargo() { need_cmd cargo "https://rustup.rs"; }
need_node() { need_cmd node "Node.js with npm"; need_cmd npm "Node.js with npm"; }
need() {   # need "path: what it is"...
  local f
  for f in "$@"; do [ -e "$R/${f%%:*}" ] || MISSING+=("$f"); done
}
have() {
  local f
  for f in "$@"; do [ -e "$R/${f%%:*}" ] || return 1; done
}
missing_of() {
  local f
  for f in "$@"; do [ -e "$R/${f%%:*}" ] || printf '%s, ' "${f%%:*}"; done | sed 's/, $//'
}
check() {   # check STEP: stops when anything was missing, else clears the list for the next step
  if [ ${#MISSING[@]} -gt 0 ]; then
    echo "$1: cannot run, missing:" >&2
    printf '  - %s\n' "${MISSING[@]}" >&2
    echo "  (docs/BUILD.md, What is needed)" >&2
    exit 1
  fi
  MISSING=()
}

# Runs a command and shows its whole output only when it fails: a compile error is never cut to its last line
quiet() {
  local out rc=0
  out=$("$@" 2>&1) || rc=$?
  if [ "$rc" -ne 0 ]; then printf '%s\n' "$out" >&2; fi
  return "$rc"
}

# ---- the data ----
# the page's lists for both eras (export/export_page.py): run by every step that writes one of their inputs, so the
# tracked page.json never lags them (the page's build refuses one that does: webapp/vite/pagedata.mjs)
export_page() {
  (cd "$C/export" && "${PY[@]}" export_page.py global && "${PY[@]}" export_page.py jp)
}
step_data() {
  need_uv; need "$GLOBAL_MDB" "$JP_MDB"; check data
  local era script
  for era in global jp; do
    for script in build_base export_rs export_web export_cards export_career; do
      (cd "$C/export" && "${PY[@]}" "$script.py" "$era")
    done
  done
  (cd "$C/export" && "${PY[@]}" upcoming.py global)
  export_page
}
step_gametora() {
  need_uv; check gametora
  "${PY[@]}" "$C/export/gametora.py" ${GAMETORA_LATEST:+--latest}
  export_page
}
step_events() {
  need_uv; need "$GLOBAL_MDB" "$JP_MDB"; check events
  step_gametora
  (cd "$C/export" && "${PY[@]}" build_card_events.py global && "${PY[@]}" build_card_events.py jp)
  export_page
}
step_pb() {
  need_uv; need_cargo; check pb
  build_solver
  "${PY[@]}" "$C/export/add_pb.py" "$BIN" "$C/data/global/data.json" "$C/data/jp/data.json"
  export_page
}

# ---- the solver ----
step_native() {
  need_cargo; check native
  if passed native; then return 0; fi
  (cd "$C" && clippy_all)
  cargo_test
  mark_passed native
}
# The release binary (fat LTO, as the wasm): pb and research build it first, so they never run a binary left from
# before a change (cargo's own check makes this a no-op on an unchanged tree).
build_solver() { (cd "$C" && quiet cargo build --release); }
# The checks' binary (CHECK_BIN): val, valq, mono and lint build it first, for the same reason. Its outputs equal the
# release binary's to the byte (docs/BUILD.md, *Tests*), and it builds in a third of the time.
build_checked() { (cd "$C" && quiet cargo build --profile checks); }
# clippy as lint runs it (lints chosen in pkg/loopopt/Cargo.toml): quiet unless it finds something
clippy_all() {
  cargo clippy --quiet --all-targets -- -D warnings \
    && cargo clippy --quiet --target wasm32-unknown-unknown --lib -- -D warnings
}
# cargo test in the checks profile: every test binary's result line and any failure, with the tests' exit status
cargo_test() {
  local out rc=0
  out=$(cd "$C" && cargo test --profile checks 2>&1) || rc=$?
  printf '%s\n' "$out" | grep -E '^test result|FAILED|panicked|^error' || true
  return "$rc"
}
step_wasm() {
  need_cargo; need_cmd rustup "https://rustup.rs: adds the wasm target"; need_node; check wasm
  npm_deps
  # an optimised wasm and its glue built from these exact inputs, by this tree or another worktree, are copied, not
  # built again (cache/wasm/KEY: about 4 minutes of fat-LTO build and wasm-opt saved per worktree)
  local built
  built="$R/cache/wasm/$(inputs_key solver "${WASM_OPT_FLAGS[@]}")"
  if [ -f "$built/loopopt.opt.wasm" ]; then
    cp "$built/loopopt.opt.wasm" "$W/loopopt.opt.wasm"
    cp "$built/loopopt.js" "$built/loopopt.d.ts" "$W/src/gen/"
    echo "wasm: built before from these inputs (${built#"$R"/}), copied"
  else
    build_wasm
    mkdir -p "$built"
    cp "$W/loopopt.opt.wasm" "$BINDGEN/loopopt.js" "$BINDGEN/loopopt.d.ts" "$built/"
  fi
  (cd "$W" && node example.mjs)
}
build_wasm() {
  wasm_bindgen_cli
  (cd "$C" && quiet cargo build --release --lib --target wasm32-unknown-unknown)
  # --target web: an ES module the worker imports (Vite bundles it), instantiated from the page's compiled module by
  # initSync; --omit-default-module-path: no fallback fetch of the wasm by URL, which Vite would ship a second copy for
  "$WASM_BINDGEN" --target web --omit-default-module-path --out-dir "$BINDGEN" --out-name loopopt "$RAW_WASM"
  cp "$BINDGEN/loopopt.js" "$BINDGEN/loopopt.d.ts" "$W/src/gen/"
  # wasm-opt is nearly all of an unchanged rebuild (about a minute): skipped when its key matches the last optimised
  # build's (webapp/.loopopt.wasm.key, untracked); the example (1.5 s, it reads the data and today's date) always runs
  local key
  key=$(wasm_opt_key)
  if [ -f "$W/loopopt.opt.wasm" ] && [ "$(cat "$W/.loopopt.wasm.key" 2>/dev/null)" = "$key" ]; then
    echo "wasm-opt: the wasm, its flags and wasm-opt are unchanged, skipped"
  else
    (cd "$W" && npx wasm-opt "${WASM_OPT_FLAGS[@]}" "$BOUND_WASM" -o loopopt.opt.wasm)
    echo "$key" >"$W/.loopopt.wasm.key"
  fi
}
# everything wasm-opt's output depends on: the compiled wasm, the flags and binaryen's version
wasm_opt_key() {
  local binaryen
  binaryen=$(node -p "require('$W/node_modules/binaryen/package.json').version")
  { cat "$BOUND_WASM"; printf '%s\n' "${WASM_OPT_FLAGS[@]}" "$binaryen"; } | sha256sum | cut -d' ' -f1
}
# wasm-bindgen-cli at the version of the wasm-bindgen crate in Cargo.lock (the glue's format is the crate's), installed
# by cargo into cache/wasm-bindgen on first use and again when the crate's pin moves (a few CPU-minutes)
wasm_bindgen_cli() {
  local version
  version=$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p;}' "$C/Cargo.lock")
  [ -n "$version" ] || { echo "wasm: no wasm-bindgen in pkg/loopopt/Cargo.lock" >&2; exit 1; }
  if [ "$("$WASM_BINDGEN" --version 2>/dev/null)" != "wasm-bindgen $version" ]; then
    echo "wasm-bindgen-cli $version: installing into cache/wasm-bindgen (cargo install, once)"
    cargo install --quiet --locked --root "$R/cache/wasm-bindgen" wasm-bindgen-cli --version "=$version"
  fi
}

# ---- checks ----
# every linter runs, whatever the others found; the step lists the ones that failed
# Every linter of every area, whatever the others found; the step ends by naming the ones that failed. Rust's
# (rustfmt, clippy, the generated types) run only when a Rust input differs from main's (changed_in rust), which passed
# them at its merge: a page or Python change compiles no Rust.
step_lint() {
  need_cargo; need_uv; need_node; check lint
  npm_deps
  LINT_FAILED=()
  CHANGED=$(changed_paths)
  if changed_in rust; then rust_lints; else echo "  rust: inputs as on main, not linted"; fi
  shell_lints
  python_lints
  if changed_in rust research; then research_lints; fi
  page_lints
  lint_verdict lint
}
rust_lints() {
  lint rustfmt in_dir "$C" cargo fmt --check
  lint clippy in_dir "$C" clippy_all
  # (the page's generated TypeScript, webapp/src/gen: the test fails when it is stale; docs/BUILD.md, *Generated types*)
  lint "generated types" in_dir "$C" cargo test --quiet --profile checks --lib ts_gen
}
shell_lints() { lint shellcheck uv run --quiet --project "$R" shellcheck "$R/build.sh" "$R/.githooks/pre-commit"; }
python_lints() {
  lint "ruff check" in_dir "$R" uv run --quiet ruff check
  lint "ruff format" in_dir "$R" uv run --quiet ruff format --check
  lint test_validation in_dir "$R" quiet "${PY[@]}" val/test_validation.py
}
# (section 8 reads CHAIN_N from the solver, `loopopt constants`: the checks' binary, built first, a no-op when current)
research_lints() {
  lint test_s08_likelihood build_and_run env LOOPOPT_SOLVER="$CHECK_BIN" uv run --quiet --project "$R" --group research python "$R/research/test_s08_likelihood.py"
}
page_types() {
  lint tsc in_dir "$W" npx --no-install tsc -p .
  lint "tsc (worker)" in_dir "$W" npx --no-install tsc -p tsconfig.worker.json
}
page_lints() {
  lint biome in_dir "$W" npx --no-install biome check --error-on-warnings --diagnostic-level=warn
  page_types
  lint "Tailwind classes" in_dir "$W" npx --no-install oxlint -c .oxlintrc.json src
  lint knip in_dir "$W" npx --no-install knip
  lint dependency-cruiser in_dir "$W" npx --no-install depcruise --config .dependency-cruiser.mjs src
  lint "@ts-expect-error" no_ts_expect_error
  node_tests run
}
# the page's node tests: `node_tests run` all of them, `node_tests related FILE...` those that import the files.
# Capped: a runaway test (assert.equal on two fake-DOM nodes formats all of Preact's objects) dies at 1 GB, not the
# terminal, and a hung one at 5 minutes (each test's own limit, 60 s: vite.config.mjs)
node_tests() {
  lint "node tests" in_dir "$W" env NODE_OPTIONS=--max-old-space-size=1024 timeout 300 npx --no-install vitest "$@" --run --reporter=dot
}
lint_verdict() {   # lint_verdict NAME: fails, naming the linters that failed, if any did
  if [ ${#LINT_FAILED[@]} -gt 0 ]; then
    local list
    list=$(printf '%s, ' "${LINT_FAILED[@]}")
    echo "$1: failed: ${list%, }" >&2
    return 1
  fi
  echo "$1: clean"
}
# lint NAME COMMAND...: runs it, prints its wall time, and records a failure in LINT_FAILED
lint() {
  local name=$1 start=$EPOCHREALTIME
  shift
  "$@" || LINT_FAILED+=("$name")
  printf '  %-24s %6.1f s\n' "$name" "$(elapsed "$start")"
}
elapsed() { echo "$EPOCHREALTIME - $1" | bc; }   # elapsed START: seconds since START (an $EPOCHREALTIME)
# tsc accepts a `@ts-expect-error` and no Biome rule sees a comment: one in the page's source fails lint (and the hook)
no_ts_expect_error() { ! grep -rn --include='*.ts' --include='*.tsx' '@ts-expect-error' "$W/src"; }
in_dir() { (cd "$1" && shift && "$@"); }   # in_dir DIR COMMAND...
build_and_run() { build_checked && quiet "$@"; }   # build_and_run COMMAND...: the checks' binary first
step_hooks() {
  git -C "$R" config core.hooksPath .githooks
  echo "hooks: core.hooksPath = .githooks (every worktree of this clone)"
}
# val TAG [--quick] (docs/BUILD.md, Tests): each check runs whatever the others found, and the step fails if any does.
# The deck and career jobs stop it before any comparison when one fails (their comparisons would read the tag's older
# outputs); planval and simval stop on a solver failure before comparing, and fail the step.
step_val() {
  local tag="${1:?val needs a tag}" quick=() rc=0
  [ "${2:-}" = --quick ] && quick=(--quick)
  need_uv; need_cargo; check val
  local mode=full
  [ ${#quick[@]} -gt 0 ] && mode=quick
  if passed val "$mode" || { [ "$mode" = quick ] && passed val full; }; then return 0; fi
  if ! val_checks "$tag" "${quick[@]}"; then return 1; fi
  mark_passed val "$mode"
}
val_checks() {   # val_checks TAG [--quick]
  local tag=$1 quick=("${@:2}") rc=0
  build_checked
  "${PY[@]}" "$R/val/runval.py" "$CHECK_BIN" "$tag" "${quick[@]}" || return 1
  "${PY[@]}" "$R/val/cmp2.py" "$tag" ref || rc=1
  "${PY[@]}" "$R/val/cmp2.py" "${tag}pu" refpu || rc=1
  "${PY[@]}" "$R/val/same.py" "$tag" || rc=1
  [ ${#quick[@]} -gt 0 ] || "${PY[@]}" "$R/val/joint_check.py" "$tag" || rc=1
  "${PY[@]}" "$R/val/simval.py" "$CHECK_BIN" "$tag" || rc=1
  "${PY[@]}" "$R/val/planval.py" "$CHECK_BIN" "$tag" "${quick[@]}" || rc=1
  "${PY[@]}" "$R/val/same.py" "$tag" --plan || rc=1
  return "$rc"
}
step_valq() { step_val "$1" --quick; }
step_mono() {
  need_uv; need_cargo; check mono
  if passed mono; then return 0; fi
  build_checked
  "${PY[@]}" "$R/val/mono.py" "$CHECK_BIN"
  mark_passed mono
}
step_research() {
  local py=(uv run --quiet --project "$R" --group research python)
  need_cargo; check research
  build_solver   # section 13 runs the release binary (research/common.py SOLVER)
  "${py[@]}" "$R/research/run_all.py" "$@"
  "${py[@]}" "$R/research/render.py"
}

# ---- the page ----
# the page's packages (webapp/package.json); wrangler (webapp/deploy) and three (webapp/chibi) are packages of their
# own, installed only by the steps that use them (deploy, runners), so every other install goes without them
npm_deps() {
  need_node; check npm
  npm_ci "$W"
}
# npm ci in DIR when it has no install or its lockfile is newer than what npm last installed there
# (node_modules/.package-lock.json), so a pulled dependency change is never run against a stale node_modules
npm_ci() {   # npm_ci DIR
  [ -e "$1/node_modules/.package-lock.json" ] && [ ! "$1/package-lock.json" -nt "$1/node_modules/.package-lock.json" ] \
    || (cd "$1" && npm ci --silent --prefer-offline)
}
# Vite (webapp/vite.config.mjs)
step_page() {
  need_uv; need_node; need "$NEED_OPT_WASM"; check page
  npm_deps
  (cd "$W" && npx --no-install vite build --logLevel warn)
}
step_images() {
  need_uv; need "${IMAGE_FILES[@]}"; check images
  art_notes
  "${PYART[@]}" "$W/images.py" ${IMAGES_FORCE:+--force}
}
art_notes() {   # optional inputs of images: missing ones only lose their pictures
  [ -e "$R/game/global/resources.assets" ] || echo "images: no game/global/resources.assets (the install's" \
    "built-in assets): the game's buttons and check box keep their drawn fallbacks"
  [ -d "$R/game/jp/bundles" ] || echo "images: no game/jp/bundles (the JP client's bundles): the Grand Masters" \
    "art and 14 JP-only skill icons keep their fallbacks"
}
step_runners() {
  need_uv
  if [ ! -e "$R/cache/chibi/tools/xml/usr/lib/libxml2.so.2" ]; then
    need_cmd tar "to unpack libxml2 once"; need_cmd zstd "to unpack libxml2 once"
  fi
  need "${RUNNER_FILES[@]}"; check runners
  step_browsers
  npm_ci "$W/chibi"   # three.js (chibi/package.json)
  "${PYART[@]}" "$W/chibi/chibi.py" all
}
step_art() {   # in the default build: runs what the game files in game/ allow, says what it skips
  if have "${IMAGE_FILES[@]}"; then step_images
  else echo "images: skipped, missing $(missing_of "${IMAGE_FILES[@]}") (the page shows initials)"; fi
  if have "${RUNNER_FILES[@]}"; then step_runners
  else echo "runners: skipped, missing $(missing_of "${RUNNER_FILES[@]}") (the page shows a spinner)"; fi
}
# the revisions the pinned playwright package wants: a no-op (0.6 s) when they are in cache/ms-playwright, a download
# after a Playwright bump (a check for any browser folder passed with the old revisions there)
step_browsers() {
  npm_deps
  (cd "$W" && npx --no-install playwright install chromium firefox)
}
step_test() { browser_suite full; }
step_testq() { LLP_QUICK=1 browser_suite quick --project chromium --grep @quick; }
# The browser suite on the page this tree builds (1.4 s first), never a dist left from before. Its replaying groups
# answer from the solver replies recorded for this solver and data (webapp/test/replaystore.mjs); when this tier has
# none, the run is live (LLP_SOLVER=live), which records them, and a passing live run marks them recorded.
browser_suite() {   # browser_suite quick|full PLAYWRIGHT_ARGS...
  local tier=$1 store solver=${LLP_SOLVER:-replay}
  shift
  step_page
  step_browsers
  store=$(cd "$W" && node test/replaystore.mjs)
  if [ "$solver" = replay ] && ! replies_recorded "$store" "$tier"; then
    echo "solver replies: none recorded for this solver and data (${store#"$R"/}): this run is live and records them"
    solver=live
  fi
  (cd "$W" && LLP_SOLVER=$solver npx --no-install playwright test "$@")
  if [ "$solver" = live ]; then mkdir -p "$store"; touch "$store/.recorded-$tier"; fi
}
# replies_recorded STORE quick|full: a passing live run of this tier, or of the full suite, recorded into STORE
replies_recorded() { [ -e "$1/.recorded-full" ] || { [ "$2" = quick ] && [ -e "$1/.recorded-quick" ]; }; }
step_prune_replies() {
  need_uv; check prune-replies
  step_page   # the current solver and data are this tree's
  "${PY[@]}" "$W/test/prune_replies.py" "$(cd "$W" && node test/replaystore.mjs)"
}
step_serve() {
  need_node; need "$NEED_DIST"; check serve
  (cd "$W" && npx --no-install vite preview --port "${PORT:-8000}" --host localhost --strictPort)
}
# Cloudflare Pages, a direct upload of webapp/dist by wrangler, installed here from webapp/deploy's own lockfile (set up
# once, in webapp/deploy: npx wrangler login; npx wrangler pages project create)
step_deploy() {
  local dirty
  dirty=$(git -C "$R" status --porcelain --untracked-files=no -- webapp pkg research/results.json)
  if [ -n "$dirty" ] && [ -z "${DEPLOY_DIRTY:-}" ]; then
    echo "uncommitted changes under webapp/ pkg/ or research/results.json; commit first (or DEPLOY_DIRTY=1)"
    echo "$dirty"
    exit 1
  fi
  local dirty_flag=false
  if [ -n "$dirty" ]; then dirty_flag=true; fi
  step_wasm; step_page   # the solver it ships is built from this tree, never one left from another
  npm_ci "$W/deploy"
  (cd "$W/deploy" && npx --no-install wrangler pages deploy ../dist --project-name "${CF_PAGES_PROJECT:-uma-legacy}" \
    --branch "${CF_PAGES_BRANCH:-main}" --commit-hash "$(git -C "$R" rev-parse HEAD)" \
    --commit-message "$(git -C "$R" describe --tags --always)" --commit-dirty="$dirty_flag")
}

# ---- what can't have changed ----
# The inputs of each kind of check, as paths (git pathspecs) from the repository's root. A check that passed records
# its inputs' key (cache/passed/NAME-KEY, shared by every worktree through cache/), and runs again only when one of
# them changes: a page-only change never runs native, val or mono. build.sh is in every key (it holds the flags).
declare -A INPUTS=(
  [solver]="pkg/loopopt/src pkg/loopopt/build.rs pkg/loopopt/Cargo.toml pkg/loopopt/Cargo.lock rust-toolchain.toml research/results.json"
  [rust]="pkg/loopopt/src pkg/loopopt/tests pkg/loopopt/build.rs pkg/loopopt/Cargo.toml pkg/loopopt/Cargo.lock rust-toolchain.toml research/results.json webapp/src/gen"
  [native]="pkg/loopopt/src pkg/loopopt/tests pkg/loopopt/build.rs pkg/loopopt/Cargo.toml pkg/loopopt/Cargo.lock rust-toolchain.toml research/results.json pkg/loopopt/data val/jobs val/plan val/sim"
  [val]="pkg/loopopt/src pkg/loopopt/build.rs pkg/loopopt/Cargo.toml pkg/loopopt/Cargo.lock rust-toolchain.toml research/results.json pkg/loopopt/data val"
  [mono]="pkg/loopopt/src pkg/loopopt/build.rs pkg/loopopt/Cargo.toml pkg/loopopt/Cargo.lock rust-toolchain.toml research/results.json pkg/loopopt/data val/mono.py val/solver.py val/plan"
  [research]="research pkg/loopopt/src pkg/loopopt/data val/jobs val/jobs.json"
  [python]=":(glob)**/*.py pyproject.toml uv.lock"
  [shell]="build.sh .githooks"
  [page]="webapp"
)
# inputs_key AREA [EXTRA...]: 16 hex digits of the SHA-256 of the area's files (tracked and untracked, ignored ones
# left out: names and contents as they are on disk), build.sh, and EXTRA words
inputs_key() {
  local area=$1
  shift
  # shellcheck disable=SC2086 # the pathspecs are words
  { (cd "$R" && git ls-files -co --exclude-standard -- ${INPUTS[$area]} build.sh | LC_ALL=C sort -u \
      | while IFS= read -r f; do if [ -f "$f" ]; then printf '%s\0' "$f"; fi; done | xargs -0 -r sha256sum)
    printf '%s\n' "$@"; } | sha256sum | cut -c1-16
}
passed() {   # passed AREA [MODE]: this area's checks passed on its current inputs (says so)
  local stamp
  stamp="$R/cache/passed/$1${2:+-$2}-$(inputs_key "$1")"
  [ -e "$stamp" ] || return 1
  echo "$1${2:+ $2}: passed before on these inputs (${stamp#"$R"/}), not run again"
}
mark_passed() { mkdir -p "$R/cache/passed" && touch "$R/cache/passed/$1${2:+-$2}-$(inputs_key "$1")"; }

# The scope: what differs from main (the commits since this branch left it, staged, unstaged and untracked files; on
# main itself, from its upstream). changed_in AREA...: one of those paths is an input of one of the areas.
changed_paths() {
  if [ -z "${CHANGED+set}" ]; then
    local base
    base=$(git -C "$R" merge-base HEAD main 2>/dev/null || git -C "$R" rev-parse HEAD)
    if [ "$base" = "$(git -C "$R" rev-parse HEAD)" ]; then
      base=$(git -C "$R" rev-parse -q --verify '@{upstream}' 2>/dev/null || echo "$base")
    fi
    CHANGED=$({ git -C "$R" diff --name-only "$base"; git -C "$R" ls-files -o --exclude-standard; } | sort -u)
  fi
  printf '%s\n' "$CHANGED"
}
changed_in() {
  local area spec
  for area in "$@"; do
    for spec in ${INPUTS[$area]}; do
      case $spec in
        ':(glob)**/*.py') if changed_paths | grep -q '\.py$'; then return 0; fi ;;
        *) if changed_paths | grep -q -e "^$spec\$" -e "^$spec/"; then return 0; fi ;;
      esac
    done
  done
  return 1
}

# check [TIER]: the checks a change needs, chosen by what differs from main (changed_in) and skipped where their
# inputs already passed (passed); each prints its wall time, and the tier its total against its budget, which is
# reported, never enforced (docs/BUILD.md, Tests):
#   edit      every edit, 10 s: the page's types and the node tests of the changed files; clippy and the unit tests;
#             ruff on the changed Python
#   handback  hand-back of a piece of work, 5 min: edit's, then every linter of each changed area, the Rust tests
#             (checks profile), valq, research (its sections recompute only on their own inputs), the browser
#             suite's fast tier
#   merge     before a merge to main, 10 min: every area, changed or not (each skipped where it already passed):
#             lint, native, val, the full browser suite, and mono after a change to the career planner
#   full      by hand: merge's, then mono and research on every section (--force), whatever changed
TIERS=(edit handback merge full)
declare -A BUDGET=([edit]=10 [handback]=300 [merge]=600 [full]=0)
step_check() {
  local tier=${1:-handback} start=$EPOCHREALTIME rc=0
  [[ " ${TIERS[*]} " == *" $tier "* ]] || { echo "check: no tier $tier (${TIERS[*]})" >&2; exit 1; }
  need_cargo; need_uv; need_node; check check
  LINT_FAILED=()
  CHANGED=$(changed_paths)
  echo "check $tier: changed from main: $(changed_paths | sed 's#/.*##' | sort -u | xargs)"
  "check_$tier" || rc=1
  lint_verdict "check $tier" || rc=1
  local took
  took=$(elapsed "$start")
  if [ "${BUDGET[$tier]}" -gt 0 ] && [ "${took%.*}" -gt "${BUDGET[$tier]}" ]; then
    echo "check $tier: ${took%.*} s, over its ${BUDGET[$tier]} s budget"
  else
    echo "check $tier: ${took%.*} s"
  fi
  return "$rc"
}
with_wasm() { step_wasm && "$@"; }   # with_wasm STEP: the wasm first (copied when its inputs were built before)
check_edit() {
  local py
  if changed_in page; then
    npm_deps
    page_types
    local files
    mapfile -t files < <(changed_paths | sed -n 's#^webapp/##p' | grep -E '^(src|test)/.*\.(ts|tsx|mjs)$' || true)
    if [ ${#files[@]} -gt 0 ]; then node_tests related "${files[@]}"; fi
  fi
  if changed_in rust; then
    lint clippy in_dir "$C" cargo clippy --quiet --all-targets -- -D warnings
    lint "unit tests" in_dir "$C" quiet cargo test --profile checks --lib
  fi
  py=$(changed_paths | grep '\.py$' || true)
  if [ -n "$py" ]; then
    # shellcheck disable=SC2086 # one path per word: the repo's paths have no spaces
    lint ruff in_dir "$R" uv run --quiet ruff check $py
  fi
}
check_handback() {
  check_edit
  if changed_in shell; then shell_lints; fi
  if changed_in python; then python_lints; fi
  if changed_in rust; then rust_lints; fi
  if changed_in rust research; then research_lints; fi
  if changed_in page; then page_lints; fi
  if changed_in native; then lint "rust tests (checks)" step_native; fi
  if changed_in val; then lint valq step_valq check; fi
  if changed_in research; then lint research step_research; fi
  if changed_in page solver; then lint "browser, fast tier" with_wasm step_testq; fi
}
check_merge() {
  shell_lints
  python_lints
  research_lints
  rust_lints
  page_lints
  lint "rust tests (checks)" step_native
  lint val step_val check
  lint "browser suite" with_wasm step_test
  if changed_paths | grep -q '^pkg/loopopt/src/career/'; then lint mono step_mono; fi
}
check_full() {
  check_merge
  lint mono step_mono
  lint "research (every section)" step_research --force
}

# ---- the table ----
steps_in() {   # steps_in SET: the steps of a set (all, default), in the table's order
  local row name sets
  for row in "${STEPS[@]}"; do
    IFS='|' read -r name _ sets _ <<<"$row"
    if [[ " $sets " == *" $1 "* ]]; then printf '%s\n' "$name"; fi
  done
}
step_arg() {   # step_arg NAME: the step's argument (empty when it takes none); fails for a name not in the table
  local row name arg
  for row in "${STEPS[@]}"; do
    IFS='|' read -r name arg _ <<<"$row"
    if [ "$name" = "$1" ]; then printf '%s\n' "$arg"; return 0; fi
  done
  return 1
}
step_all() {
  local step
  for step in $(steps_in all); do "step_$step"; done
}
step_help() {
  local row name arg desc
  echo "Usage: ./build.sh [step...]   (no step: $(steps_in default | xargs))"
  for row in "${STEPS[@]}"; do
    IFS='|' read -r name arg _ desc <<<"$row"
    [ "$name" = all ] && desc="$desc: $(steps_in all | xargs)"
    printf '  %-14s %s\n' "$name${arg:+ $arg}" "$desc"
  done
}

if [ $# -eq 0 ]; then mapfile -t defaults < <(steps_in default); set -- "${defaults[@]}"; fi
while [ $# -gt 0 ]; do
  step=$1; shift
  if ! arg=$(step_arg "$step"); then echo "unknown step $step (./build.sh help)" >&2; exit 1; fi
  if [[ $arg == "[TIER]" ]]; then
    if [ $# -gt 0 ] && [[ " ${TIERS[*]} " == *" $1 "* ]]; then "step_${step//-/_}" "$1"; shift; else "step_${step//-/_}"; fi
  elif [ -n "$arg" ]; then
    "step_${step//-/_}" "${1:-}"
    shift
  else
    "step_${step//-/_}"
  fi
done
