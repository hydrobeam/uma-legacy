# uma-legacy

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
- Every modelling assumption with its evidence: `docs/MODEL.md`; outside sources: `docs/SOURCES.md`

Inheritance chances use uma-it-optimizer's per-legacy affinity model, fitted to Independent Training receipts
([Acrith/uma-it-optimizer](https://github.com/Acrith/uma-it-optimizer)). Character icons and card art come from the
game's own files, © Cygames, and are not part of this repository.
