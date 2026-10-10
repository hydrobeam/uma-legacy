# uma-legacy: the Legacy Loop Planner

Finds the best legacy loop (a three-uma rotation or a four-uma loop) and support decks (Uma Musume: Pretty Derby, Global) for landing a set of target
skills in Independent Training, with each uma's race schedule.

Use it at https://uma-legacy.pages.dev.

The career planner at `/career` picks the next career from your own veterans (imported from umadump or uma.moe): the
trainee, her parents and her deck, chosen by how many careers it brings you closer to a fully looped parent (an 8/8
whose parents are 8/8s). For the veteran you actually got, it shows how many careers she saves.

- Using the page: `docs/USAGE.md`; the page's own "How this works" explains the model in plain words
- Building, testing, deploying, repository layout: `docs/BUILD.md`
- The solver (model, algorithms, job fields, command line): `docs/SOLVER.md`
- The maths behind each fitted value: `docs/RESEARCH.md`; every modelling assumption with its evidence:
  `docs/MODEL.md`

## Building it

Rust (pinned in `rust-toolchain.toml`), Node with npm, and [uv](https://docs.astral.sh/uv/):

```
./build.sh            # build everything (fetches its packages and data the first time)
./build.sh serve      # http://localhost:8000
```

That is all a fresh clone needs; `docs/BUILD.md` has the checks and the other steps. Refreshing the game data and making the art need
game files in `game/` (`docs/BUILD.md` lists them); without art the page shows initials.

## Credits and licence

The model builds on community research and tools:
- [uma-it-optimizer](https://github.com/Acrith/uma-it-optimizer) (Acrith): the per-legacy affinity model, fitted on
  Independent Training receipts.
- [uma-it](https://training.umaladder.moe): the Independent Training runs behind the measured hint, chain-event and
  outcome rates and the calibration (pooled aggregates only).
- [hakuraku.moe](https://hakuraku.moe/notes/spark_generation): spark generation and the star mix by rank score.
- [umamusustation](https://umamusustation.com/factor/red_factor_analysis.html): pink spark generation.
- [GameTora](https://gametora.com): card event data (downloaded at build time) and its
  [legacies guide](https://gametora.com/umamusume/legacies).
- [umamusu.wiki](https://umamusu.wiki/Game:Inheritance), [uma.guide](https://uma.guide/guides/sparks) and
  [umareference](https://www.umareference.com/guide/legacies/chance-of-inheriting-sparks): inheritance and pink-spark
  rules and base rates; JP community notes (herohero's 1,000+ trial inheritance rates among them).
- [uma.moe](https://uma.moe): its lineage planner, which our affinity and odds match; its lineage exports import here.
- [umadump](https://github.com/Werseter/umadump) (Werseter): the veteran export the career planner imports.

The code is licensed under the GNU General Public License v3.0 or later (`LICENSE`). Uma Musume: Pretty Derby, its
names, game data and art are © Cygames, Inc.; they are not covered by this licence and the art is not part of this
repository. This is an unofficial fan tool, not affiliated with or endorsed by Cygames.
