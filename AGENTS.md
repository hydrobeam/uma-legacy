# Legacy Loop Planner: instructions for coding agents

Read this file, then `docs/HANDOFF.md` (where the work stands and why), before doing anything. `docs/MODEL.md`
holds every modelling assumption with its evidence; `docs/RESEARCH.md` the maths behind each fitted
value; `docs/SOURCES.md` lists the outside sources and repos. `docs/BUILD.md` is the build reference (steps,
assets, tests, repository layout); `docs/SOLVER.md` the solver's model and job fields; `docs/USAGE.md` the page;
`docs/PERF.md` and `docs/PERF_PLANNER.md` the speed protocol and what was done or tried (read only the section you need:
these docs are long).

## What this is

An optimizer for Uma Musume: Pretty Derby (Global server) that finds the best four-uma legacy loop and support
deck for landing a set of target skills in **Independent Training (IT)**. It is used for IT only.

- Three pages: the loop planner (`/`), the career planner (`/career`: expected careers to a looped parent from the
  player's veterans, Compare and Reroll) and `/about`. Unity Cup is the default scenario; the JP Grand Masters-era data
  is a secondary mode.
- The page must be served over http (`./build.sh serve`), never opened as a file: WebAssembly is required (no JS
  fallback). Its game art (`webapp/img/`, extracted by `./build.sh images` from locally decrypted game files, the runner
  strips by `./build.sh runners`, © Cygames) is never committed; without it the page shows initials.
- Card event rewards come only from GameTora's data files (`./build.sh gametora` downloads them into `cache/gametora/`).
- Default targets (8): Uma Stan/Superstan, Nimble Navigator/No Stopping Me!, Playtime's Over!/See Ya
  Later!, Ramp Up/It's On!, Tail Held High/Tail Nine, Racing Spirit: Stamina, Slipstream/Perfect Spot!, Risky
  Business/Nothing Ventured. Unity Cup guarantees the No Stopping Me! and It's On! golds.

## How to communicate

Verdict first, then numbers; no padding. For this project:

- When naming umas in results, give the outfit used (e.g. "Agnes Digital [Full-Color Fangirling]", "Oguri Cap, any
  outfit"). Don't specify awakening levels. Don't describe goal races when describing a setup.

## How to work in this repo

- **Python runs in the uv project environment**: `uv run script.py` (`pyproject.toml`, `uv.lock`). Add dependencies
  with `uv add`; never install into the system Python.
- **Ask before heavy compute.** Before any simulator, sampling, search batch or parallel fan-out that takes more than
  a few seconds of CPU, state its size (runs, cores, expected time) and wait for a go. Think first; run the smallest
  version that answers the question and report it before scaling up.
- **Never sit idle on a long job.** Validation and some searches take minutes. Run them in the background and
  report what you can meanwhile. Poll by counting output files; `pgrep -f` on a pattern that appears in your own
  command line matches itself and never ends (this cost five minutes once). Kill processes by PID, never with
  `pkill -f` (it matched its own shell once and killed the command).
- **Verify game facts against data, not memory.** Decode from `master.mdb`, `web_cards.json`/`web_skills.json` and
  the GameTora caches; cite where a number came from. Never build an example from remembered card or skill facts
  (an invented example once had inheritance-only skills coming from cards). Community figures are estimates unless
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
  `docs/MODEL.md`, `docs/RESEARCH.md`. A change to a modelling constant updates the page's model text too
  (`webapp/about.template.html`, search "Model."). Docs are long-term memory: keep them lean, one place per fact.
- **Shell safety.** No `rm` on paths built from variables or globs, even in scratch: write fresh output to a new
  directory at a literal path instead. Every `rm` targets a file you created, by literal path, for a stated reason.

## Agents

- Use general-purpose agents with a written brief for self-contained work (forks re-read the whole conversation).
  Agents in a worktree start from the last commit: uncommitted edits, this file's included, don't reach them.
- Every brief carries the rules the agent can't infer: the verification above for its kind of change, go easy on
  compute (no parallel batches without a go), commit signed per item, and on a GPG signing failure save the item's
  patch and commit message to files and continue with the next item (never stop, never commit unsigned).
- Ask for a short report: verdict, numbers, what changed, what was unrun. Agent reports arrive on their own; don't
  poll.

## Git

- Maintainers work on `main` (no pull requests); outside contributors send pull requests.
- Commit tested work as you go, one logical commit per piece (model change, its fix, page change, docs), with
  messages that say what changed and why. Commits are GPG-signed; never use `--no-gpg-sign`.
- No AI-tool attribution in commits: no Co-Authored-By lines, session links or "Generated with" footers.
- Never rewrite history or force-push; other agents may work in the repo.
- Tags `vN` mark page versions and milestones (v16 onward; milestones since v24 are listed in HANDOFF section 2).
- `master.mdb` files are never committed (`.gitignore` is a whitelist; add new tracked paths to it explicitly). Nor
  are game assets (`webapp/img/`, `cache/`, bundles) or decryption keys.

## Compact instructions

When compacting, keep: decisions the maintainer made and their dates, measured numbers with the jobs they came from,
running or unmerged agents and branches with what each is for, pending maintainer decisions, and uncommitted work. Drop
file dumps, tool output and superseded hypotheses.

## Hard facts that are easy to get wrong

- A trainee cannot use a support card of her own character or a group card she belongs to.
- Per-uma decks are on by default: each trainee gets her own best deck.
- Race affinity: +3 per shared single-race G1 win, counted per race (by win-saddle group, relocated venues are the
  same race); multi-race title saddles count for nothing.
- Spark generation in a running loop: 20% white / 40% gold x 1.1^6 (all six ancestors carry every target spark).
- Star mix of legacy sparks depends on the veteran's rank score: 50/45/5% below 17,500, 20/70/10% at 17,500+.
- Inheritance: two rolls per career; per legacy base (3/6/9% by stars) x (1 + that legacy's affinity / 100);
  grandparents use the full base. Great-grandparents count for nothing.
- IT picks the top choice of an event unless a prioritized skill (up to 10, set at career start) is on another
  choice; the model assumes the player sets priorities that reach the hints it counts.
