# Research: the numbers behind docs/RESEARCH.md

`docs/RESEARCH.md` is rendered: its prose lives in `docs/RESEARCH.template.md`, and every number it states about the
runs, the game data or a derived quantity is a placeholder filled from what these scripts compute. No number is typed
in by hand; a placeholder without a computed value stops the rendering.

```
./build.sh research                                   # compute everything, then render docs/RESEARCH.md
uv run --group research research/run_all.py [--quick] [--force] # compute (scripts whose code and inputs are unchanged are reused), write results.json
uv run --group research research/render.py [--check]  # render (or check that docs/RESEARCH.md is current)
uv run --group research research/s08_chains.py        # one section, with its detail
```

Each script, one per section that has something to compute, returns:

- **Values**: every number its section uses, by name (`common.Value`). The key `section.name_as_slug` is how the
  template refers to it (`{{7.fit_of_n_runs|,.0f}}`).
- **Findings**: every conclusion the section draws (a form adopted, an alternative rejected, a miss acknowledged),
  tested on the computed values the way the section argues it: a likelihood-ratio test, an interval that covers or
  excludes a prediction, a comparison of fits. Each prints **holds**, **FAILS** or **needs data** with the numbers
  it rests on.

`run_all.py` writes both to `research/results.json` (aggregates only) and lists computed values the template does not
use; `render.py` turns the template into `docs/RESEARCH.md`. `--quick` skips section 13's run over every other skill.

| Script | Computes |
|---|---|
| `s02_stars.py` | §2 the band each scenario's careers reach (rank scores), the expected base per roll, what averaging the stars costs |
| `s03_inheritance.py` | §3 the roll's parts on single-ancestor trials: grandparent weight, affinity slope (relation points and shared wins apart), base by stars, affinity bands |
| `s04_races.py` | §4 the race win table against every G1 attempt in the runs: base and spark-raised aptitudes, the streak penalty, the previous race's result, when a pink takes effect, the table in the community layout |
| `s05_affinity.py` | §5 the self-grandparent fit (trio points 0 against pair(T, P), κ) |
| `s06_loops.py` | §6 the race weights; trainees with their own character as a grandparent; on the validation results, what shared wins are worth and how far any other schedule could raise the odds |
| `s07_hints.py` | §7 training hints: the form study, bases and specialty weights per scenario, the Lv-5 cap, joint draws, spark deficit, held-out and same-card tests |
| `s08_chains.py` | §8 chain events: the fit of N and its held-out comparison, the marker allowance, final links by pals and by card, the allocation rule, the exact link tables, known misses |
| `s09_outcomes.py` | §9 measured outcome shares and the choice-rule evidence |
| `s10_random_events.py` | §10 random card events: the fitted rate, the form comparison, what leaving them out costs on real decks |
| `s11_conditional.py` | §11 conditional character events: counts, how often they fire in the runs, what leaving them out costs, Tokai Teio's Head-On |
| `s13_calibration.py` | §13 the calibration against real careers, §3's test on whole careers, §9's certainty checks |

§12 (the composed model) is derived and has no script. Comparisons follow the three rules in *How to read*: same
parameter count, compared directly; an extra parameter, by likelihood ratio (detectable or not); whether a model
predicts better, by 5-fold cross-validation by run id (`common.fold`, `common.compare_held_out`).

Every analysis runs on every run in the pool. What the runs cannot test is labelled **assumed** in the document and
claims no evidence: §1's spark curve, §2's star mixes and §3's base rates other than the skill whites (community
figures).

**Data.** The measured values rest on Independent Training runs from uma-it, read as run records
(`research/data/runs.jsonl`; every run record is part of the pool) and, for §2, the run list's rank-score estimates
(`research/data/summary.jsonl`). **The run records are not in this repository.** Without
them the scripts compute only what needs game data, and `results.json` keeps the committed values for the rest, so a
clone still renders the whole document. Everything else comes from the repository: `pkg/loopopt/data/global/`, the
GameTora files (through `pkg/loopopt/export/gametora.py`) and `game/global/master.mdb`. Section 6 recomputes the odds of
every validation result (`val/ref__*`, `val/refpu__*`) through `val/joint_check.py`: about a minute. Section 13 also runs the native
solver (`cd pkg/loopopt && cargo build --release`): about a minute for the default targets, several more for the
other-skills sample, one process; results are kept in `research/data/out/` and reused while nothing they read changes.
numpy and scipy come with the `research` dependency group.

**Rules of the analysis.** Pooled counts; intervals and tests treat a run as the unit wherever a run gives several
rows (`common.cluster_ci`, `common.cluster_chi2`, run-clustered standard errors), Wilson intervals where it gives one;
nothing split or explained by who ran a career; R cards
left out of measured results unless a fit says it includes them; every comparison is against the model on the run's
own deck. A number comes out as the data give it, and the text follows the number, never the reverse.
