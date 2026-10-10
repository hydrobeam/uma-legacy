# Legacy Loop Planner: instructions for coding agents

Read this file before doing anything. The docs: `docs/BUILD.md` (building, testing, the repository layout),
`docs/SOLVER.md` (the solver's model, algorithms and job fields), `docs/MODEL.md` (every modelling assumption and its
evidence), `docs/RESEARCH.md` (the maths behind each fitted value), `docs/USAGE.md` (the page) and
`docs/PERF.md` (the speed protocol and record). Read only the section you need.

## What this is

An optimizer for Uma Musume: Pretty Derby (Global server) that finds the best legacy loop (a three-uma rotation or a
four-uma loop) and support decks for landing a set of target skills in **Independent Training (IT)**, and plans the
next career from a player's veterans. It models IT only.

- Three pages: the loop planner (`/`), the career planner (`/career`: expected careers to a looped parent from the
  player's veterans, Compare and Reroll) and `/about`. Unity Cup is the default scenario; the JP Grand Masters-era data
  is a secondary mode.
- The page must be served over http (`./build.sh serve`), never opened as a file: WebAssembly is required (no JS
  fallback). Its game art (`webapp/img/`, extracted by `./build.sh images` from locally decrypted game files, the runner
  strips by `./build.sh runners`, © Cygames) is never committed; without it the page shows initials.
- Card event rewards come only from GameTora's data files (`./build.sh gametora` downloads them into `cache/gametora/`).
- The default targets (eight skill pairs) are listed in `docs/RESEARCH.md`.

When naming umas in results, give the outfit (e.g. "Agnes Digital [Full-Color Fangirling]", "Oguri Cap, any outfit").

## How to work in this repo

- **Python runs in the uv project environment**: `uv run script.py` (`pyproject.toml`, `uv.lock`). Add dependencies
  with `uv add`; never install into the system Python.
- **Ask before heavy compute.** Before any simulator, sampling, search batch or parallel fan-out that takes more than
  a few seconds of CPU, state its size (runs, cores, expected time) and wait for a go. Think first; run the smallest
  version that answers the question and report it before scaling up.
- **Never sit idle on a long job.** Validation and some searches take minutes. Run them in the background and
  report what you can meanwhile. Poll by counting output files; `pgrep -f` on a pattern that appears in your own
  command line matches itself and never ends. Kill processes by PID, never with `pkill -f` (it can match its own
  shell and kill the command).
- **Verify game facts against data, not memory.** Decode from `master.mdb`, `web_cards.json`/`web_skills.json` and
  the GameTora caches; cite where a number came from. Never build an example from remembered card or skill facts. Community figures are estimates unless
  measured; say which.
- **When results move on purpose**, rerun the affected validation, check the diff (`val/cmp2.py`, or
  `val/cmp_ratio.py` when everything should scale by one factor), copy the new outputs over `val/ref__*` /
  `val/refpu__*` in the same commit.
- **Releases**: `./build.sh native wasm page` (plus `events`/`data` when inputs changed), tests pass, full `val`
  on anything that can move results, update README/model text if behaviour changed, commit, tag `vN` (next after
  the latest tag). Then stop and say "ready to deploy": `./build.sh deploy` (Cloudflare Pages) is the maintainer's,
  run only on their explicit word each time (rollbacks are deploys too). Verify a deploy on its own hash URL; branch
  URLs cache HTML for 5 minutes.
- Keep docs current: README (user-facing), `docs/SOLVER.md` (model and job fields), `docs/BUILD.md`,
  `docs/MODEL.md`, `docs/RESEARCH.md`. Docs are long-term memory: keep them lean, one place per fact.
- **Shell safety.** No `rm` on paths built from variables or globs, even in scratch: write fresh output to a new
  directory at a literal path instead. Every `rm` targets a file you created, by literal path, for a stated reason.

## Agents

Delegated work gets a written brief that carries these rules and the verification for its kind of change.

## Git

- Maintainers work on `main` (no pull requests); outside contributors send pull requests.
- Commit tested work as you go, one logical commit per piece (model change, its fix, page change, docs), with
  messages that say what changed and why. Commits are GPG-signed; never use `--no-gpg-sign`.
- No AI-tool attribution in commits: no Co-Authored-By lines, session links or "Generated with" footers.
- Never rewrite history or force-push; others may work in the repo.
- Tags `vN` mark page versions and milestones (v16 onward; `git tag -n1` lists them).
- `master.mdb` files are never committed (`.gitignore` is a whitelist; add new tracked paths to it explicitly). Nor
  is anything from the game (`game/`, `webapp/img/`) or downloaded (`cache/`).

## Hard facts that are easy to get wrong

Every modelling assumption, with its evidence, is in `docs/MODEL.md`. The ones most often misremembered:

- A trainee cannot use a support card of her own character or a group card she belongs to.
- Race affinity: +3 per shared single-race G1 win, by race (relocated venues are the same race); multi-race title
  saddles count for nothing.
- Grandparents inherit at the full base rate; great-grandparents count for nothing.
