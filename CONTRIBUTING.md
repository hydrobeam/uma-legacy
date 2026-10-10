# Contributing

Issues and pull requests are welcome.

- Build and test as in `docs/BUILD.md`: `./build.sh` then `./build.sh testq`; a change that can move results (the
  solver, the data, the model's constants) also runs `./build.sh valq TAG` and compares against the references.
  When results move on purpose, say why in the pull request and update `val/ref__*` / `val/refpu__*`.
- Game facts come from data (`master.mdb`, the exported JSON, GameTora's files), never from memory; cite where a
  number came from. Model changes update `docs/MODEL.md` and, for fitted values, `docs/RESEARCH.md`.
- Never commit game art, `master.mdb` files, bundles or keys. `.gitignore` is a whitelist: add new tracked paths to it.
- Statistics from run data are pooled aggregates only: never per-run records, never broken down by who ran a career.

By contributing you agree that your contribution is licensed under the GPL-3.0-or-later.
