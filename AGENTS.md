# Notes for coding agents

The Legacy Loop Planner finds the best legacy loop (a three-uma rotation or a four-uma loop) and support decks for
landing target skills in Uma Musume's Independent Training, and plans a player's next career. It models Independent
Training only.

Docs: `docs/BUILD.md` (building, testing, layout), `docs/SOLVER.md` (the solver), `docs/MODEL.md` (assumptions and
evidence), `docs/RESEARCH.md` (the maths), `research/README.md` (the scripts behind it), `docs/USAGE.md` (the page),
`docs/PERF.md` (speed work).

- **Python** runs in the uv environment: `uv run script.py`; add dependencies with `uv add`.
- **Lint:** `./build.sh lint` passes before a commit; the pre-commit hook checks the staged Rust, Python and page JS
  (formatting and lints).
- **Code clarity:** follow `docs/BUILD.md`, *Code clarity*, in every language. Lint passing is not a readability review.
- **Clones:** never clone to appease the borrow checker; restructure the borrows instead (`docs/BUILD.md`, *Ownership and clones*).
- **Scripted edits check their match.** A sed or Python replacement asserts that its old text occurs exactly once, or
  uses the edit tool, which fails loudly. A formatter that reflowed the target makes an unchecked replacement miss in
  silence.
- **Where the model's constants live.** The chain-event law is in `pkg/loopopt/src/deck/chains.rs` (`CHAIN_A`,
  `CHAIN_B`, `CHAIN_STOP_SHARE`). The training-hint constants are read from `research/results.json` when the solver is
  built (`pkg/loopopt/build.rs`), so recomputing the research can move the solver's results. Measured outcome shares
  are in `pkg/loopopt/data/event_shares.json` and `pkg/loopopt/data/*/manual.json`, exported by `./build.sh events`.
- **Results are pinned.** A change that can move the solver's results runs `./build.sh valq TAG` while iterating and
  `./build.sh val TAG` before it lands; career-planner changes also run `val/planval.py` and `val/mono.py`. When results
  move on purpose:
  - the new outputs replace `val/ref__*` / `val/refpu__*` (or `val/plan/ref__*`) in the same commit;
  - `val/joint_check.py TAG`, an independent recomputation of the odds, agrees within 1e-3;
  - the `plan_roll` fixture (`pkg/loopopt/tests/fixtures/plan_roll.json`) is re-taken from the solver's own rolls;
  - `./build.sh research` runs again, since §6 recomputes the references and §13 runs the solver.

  Speed work keeps outputs byte-identical (`val/same.py`, `docs/PERF.md`). After data or solver changes, rerun
  `./build.sh pb`. Then `./build.sh fullcheck` on the staged tree: the pre-commit hook refuses a commit staging
  solver, data, validation or page code without its stamp (`docs/BUILD.md`, *Steps*).
- **`/about` states the model's exact constants** (`webapp/about.template.html`), each in one place. A change to a
  modelling constant updates it in the same change, along with `docs/MODEL.md` and `docs/RESEARCH.md`.
- **`docs/RESEARCH.md` is rendered** from `docs/RESEARCH.template.md` and `research/results.json` (`./build.sh research`):
  edit the template, never the rendered file. Every number in it is a placeholder for a value a `research/` script
  computes; only the assumed inputs are typed in, and they are labelled assumed. Each conclusion the document draws
  is a finding tested in its section's script (`research/README.md`), and a sentence the scripts do not test does not
  go in.
- **Research findings follow the data.** Sections import shared code only from `research/common.py`, never from each
  other. When a finding fails on new data, it is restated to what the data now show (a known miss where it is one) or
  its test made robust; it is never tuned to pass. An undetected effect is "not detectable", not "no effect".
- **Game facts come from data**: `master.mdb`, the exported JSON and GameTora's files, never memory. GameTora's files
  are read and decoded only through `pkg/loopopt/export/gametora.py`.
- **Pages** recreate the game's UI: its colours with a 3:1 contrast floor on the planner pages and WCAG AAA on /about,
  both checked by the browser suite; phones are designed at 375 px wide first.
- **Never commit** anything from the game (`game/`, `webapp/img/`) or downloaded (`cache/`). `.gitignore` is a
  whitelist: a path outside the whitelisted directories (`docs/`, `research/`, `pkg/loopopt/src/`, …) needs its own
  line; `git check-ignore -v PATH` shows which rule applies.

**The career planner's goal and its bar.** Every change to `/career` and `src/career/plan/` is judged against this:
- **Goal:** the recommended next career (trainee, both parents, deck) minimises the expected careers to a fully looped
  parent under the model. Everything else in the planner serves that.
- **Tolerance:** total regret at most 1% of V under the model. A wrong pick repeats until something useful lands, so
  a per-decision bar must be budgeted per pool: ε_p = 0.01 × V × (the pick's lower-bounded acceptance chance per run),
  e.g. about 0.11 careers on a pool whose useful outcomes land at 0.76% per run. δ = 0.25 careers stays only as a cap
  and as the page's tie tolerance; no per-decision bar alone certifies the 1% total (a uniform bound over every career
  would need about 0.01).
- **Two guarantees, never conflated:**
  - *Search:* the recommendation is provably the best career under the planner's score. A search over trainees, parent
    pairs and decks stops only when nothing unscored can beat it; otherwise the output says what was left out and what
    it could cost.
  - *Valuation:* the score's error against the expected careers under the model, measured, never assumed.

  The page states which of the two it gives.
- **Evidence:** passing mono, planval, bit-equality or a review round is not evidence that a pick improved. Every claim
  about a pick is sized in careers per decision, labelled measured, derived, model-internal or assumed. No "small"
  without a number.
- **Time:** the page may take up to a few minutes per career, and every second goes to work that can change or
  certify the pick.

Easy to get wrong (the rest is in `docs/MODEL.md`):
- A trainee can't use a support card of her own character or of a group she belongs to.
- Race affinity is +3 per shared single-race G1 win, counted per race; multi-race titles count nothing.
- Grandparents inherit at the full base rate, and great-grandparents count for nothing.
- Group cards are pals for a deck's make-up but do not raise the chain-event rate.
- A chain that ends early leaves the running cards, so later chain events go to the others.
- A card-event stop risk ends its chain on 1/3 of its firings; a job's own `stop_risk` pin keeps its exact k of n.
- The planner assumes the player prioritises every target, and a chain-ending skill on its own.
