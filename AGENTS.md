# Notes for coding agents

The Legacy Loop Planner finds the best legacy loop (a three-uma rotation or a four-uma loop) and support decks for
landing target skills in Uma Musume's Independent Training, and plans a player's next career. It models Independent
Training only.

Docs: `docs/BUILD.md` (building, testing, layout), `docs/SOLVER.md` (the solver), `docs/MODEL.md` (assumptions and
evidence), `docs/RESEARCH.md` (the maths), `docs/USAGE.md` (the page), `docs/PERF.md` (speed work).

- **Python** runs in the uv environment: `uv run script.py`; add dependencies with `uv add`.
- **Lint:** `./build.sh lint` passes before a commit; the pre-commit hook checks the staged Rust, Python and page JS
  (formatting and lints).
- **Code clarity:** follow `docs/BUILD.md`, *Code clarity*, in every language. Lint passing is not a readability review.
- **Clones:** never clone to appease the borrow checker; restructure the borrows instead (`docs/BUILD.md`, *Ownership and clones*).
- **Results are pinned.** A change that can move the solver's results runs `./build.sh valq TAG` while iterating and
  `./build.sh val TAG` before it lands; career-planner changes also run `val/planval.py` and `val/mono.py`. When results
  move on purpose, the new outputs replace `val/ref__*` / `val/refpu__*` (or `val/plan/ref__*`) in the same commit.
  Speed work keeps outputs byte-identical (`docs/PERF.md`). After data or solver changes, rerun `./build.sh pb`.
- **`/about` states the model's exact constants** (`webapp/about.template.html`). A change to a modelling constant
  updates it in the same change, along with `docs/MODEL.md` and `docs/RESEARCH.md`.
- **`docs/RESEARCH.md` is rendered** from `docs/RESEARCH.template.md` and `research/results.json` (`./build.sh research`):
  edit the template, never the rendered file; every number in it is a placeholder for a value a `research/` script
  computes, except the assumed inputs, which are typed in and labelled assumed, and each section's conclusions are
  tested there (`research/README.md`).
- **Game facts come from data**: `master.mdb`, the exported JSON and GameTora's files, never memory. GameTora's files
  are read and decoded only through `pkg/loopopt/export/gametora.py`.
- **Pages** recreate the game's UI: its colours with a 3:1 contrast floor on the planner pages and WCAG AAA on /about,
  both checked by the browser suite; phones are designed at 375 px wide first.
- **Never commit** anything from the game (`game/`, `webapp/img/`) or downloaded (`cache/`). `.gitignore` is a
  whitelist: a new tracked path needs its own line.

Easy to get wrong (the rest is in `docs/MODEL.md`): a trainee can't use a support card of her own character or of a
group she belongs to; race affinity is +3 per shared single-race G1 win, counted per race, and multi-race titles count
nothing; grandparents inherit at the full base rate, and great-grandparents count for nothing.
