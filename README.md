# uma-legacy: the Legacy Loop Planner

Finds the best legacy loop (a three-uma rotation or a four-uma loop) and support decks (Uma Musume: Pretty Derby, Global) for landing a set of target
skills in Independent Training, with each uma's race schedule.

Use it at https://uma-legacy.pages.dev.

The career planner at `/career` picks the next career from your own veterans (imported from umadump, or pasted from an uma.moe lineage tree): trainee,
parents and deck, chosen by how many careers it brings a fully looped parent closer (an 8/8 whose parents are 8/8s):
exact over every result she can roll, valued against the cheapest trio of umas, each building her odds while it pays;
a career counts both as a shot at an 8/8 and as a parent for the others. It shows the trio and each step's state, her
race schedule and, for the veteran you actually got (or a random roll of the career), how many careers she saves. Both
pages are drawn in the game's own style (its Umamusume Details card, buttons and art, in light and dark). Each deck
borrows one friend's card when it helps. See `docs/USAGE.md`.

- Using the page: `docs/USAGE.md` (the page's own "How this works" explains the model in plain words)
- Building, testing, deploying, repository layout: `docs/BUILD.md`
- The solver (model, algorithms, job fields, command line): `docs/SOLVER.md`
- The maths behind each fitted value: `docs/RESEARCH.md`; every modelling assumption with its evidence:
  `docs/MODEL.md`; outside sources: `docs/SOURCES.md`

## Building it

Rust (pinned in `rust-toolchain.toml`), Node with npm, and [uv](https://docs.astral.sh/uv/):

```
./build.sh            # the solver with its tests, the wasm and the page (webapp/dist)
./build.sh serve      # http://localhost:8000 (the page needs http: it loads WebAssembly)
./build.sh testq      # the browser suite's fast tier (installs its browsers into webapp/node_modules once)
./build.sh valq TAG   # the validation jobs against the reference outputs
```

A clone builds and tests the page from the tracked data files. Refreshing the game data needs the game's
`master.mdb`, and the art steps need game files decrypted on your own
machine; without art the page shows initials. Details: `docs/BUILD.md`.

## Credits and licence

Inheritance chances use uma-it-optimizer's per-legacy affinity model, fitted to Independent Training receipts
([Acrith/uma-it-optimizer](https://github.com/Acrith/uma-it-optimizer)). Measured rates come from Independent
Training runs from [uma-it](https://training.umaladder.moe). Card events come from [GameTora](https://gametora.com)'s
data files, downloaded at build time.

The code is licensed under the GNU General Public License v3.0 or later (`LICENSE`). Uma Musume: Pretty Derby, its
names, game data and art are © Cygames, Inc.; they are not covered by this licence and the art is not part of this
repository. This is an unofficial fan tool, not affiliated with or endorsed by Cygames.
