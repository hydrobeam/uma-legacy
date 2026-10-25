# Research: the numbers behind docs/RESEARCH.md

`docs/RESEARCH.md` is rendered: its prose lives in `docs/RESEARCH.template.md`, and every number it states about the
runs, the game data or a derived quantity is a placeholder filled from what these scripts compute. Only the assumed
inputs are typed in, labelled assumed; a placeholder without a computed value stops the rendering.

```
./build.sh research                                   # compute everything, then render docs/RESEARCH.md
uv run --group research research/run_all.py [--quick] [--force] [--jobs=N] # compute (unchanged scripts reused, the rest in parallel), write results.json
uv run --group research research/render.py [--check]  # render (or check that docs/RESEARCH.md is current)
uv run --group research research/s08_chains.py        # one section, with its detail
```

Each script, one per section that has something to compute, returns:

- **Values**: every number its section uses, by name (`common.Value`). The key `section.name_as_slug` is how the
  template refers to it (`{{8.fit_of_n_runs|,.0f}}`).
- **Findings**: every conclusion the section draws (a form adopted, an alternative rejected, a miss acknowledged),
  tested on the computed values the way the section argues it: a likelihood-ratio test, an interval that covers or
  excludes a prediction, a comparison of fits. Each prints **holds**, **FAILS** or **needs data** with the numbers
  it rests on.

`run_all.py` writes both to `research/results.json` (aggregates only) and lists computed values the template does not
use; `render.py` turns the template into `docs/RESEARCH.md`. `--quick` skips section 13's run over every other skill.

| Script | Computes |
|---|---|
| `s02_stars.py` | §2 the band each scenario's careers reach (rank scores, and below the ceiling by chosen offsets), the legacies' observed star mix, the expected base per roll, what averaging the stars costs over every carrier pattern |
| `s03_inheritance.py` | §3 the roll's parts on single-ancestor trials: grandparent weight, affinity slope and a flat parent weight (all trials and parents alone, in-sample, held out and with a power check), affinity confounded with generation, relation points and shared wins apart, base by stars, affinity bands |
| `s04_races.py` | §4 the race win table against every G1 attempt in the runs (each outfit's own aptitudes): base and spark-raised aptitudes, one calibration rule for every streak cell with loss ratios, the streak penalty, the previous race's result, when a pink takes effect, the goal shift held out by run and by character, a targeted low-aptitude cost, the table in the community layout |
| `s05_affinity.py` | §5 the self-grandparent fit (trio points 0 against pair(T, P), κ); shared wins observed against the base-aptitude products |
| `s06_loops.py` | §6 the race weights; trainees with their own character as a grandparent; on the validation results, what shared wins are worth, how far any other schedule could raise the odds, loops rejected after scheduling, what base grades cost in odds |
| `s07_hints.py` | §7 training hints: the form study, bases and specialty weights per scenario with card-and-career bootstrap intervals, the Lv-5 cap, joint draws, the stat option, inspired pool skills and a removal share, held-out tests clustered by card and career, leave-cards-out folds and a wild card bootstrap, same-card tests |
| `s08_chains.py` | §8 chain events: the fit of N, the pal-count budget and its held-out, nested and profile tests, the allocation rule, freed against wasted chain ends (each rule's own constants, held out with constants searched per fold, the mechanism alone, a mixture, direct deck-mate contrasts, a re-fitting parametric bootstrap, background and grid checks), whether prioritising an ending skill ends the chain, the marker allowance, final links by deck and by card, known misses |
| `s09_outcomes.py` | §9 measured outcome shares, a scan of every readable choice against the equal rule, and the choice-rule evidence, its three cases included |
| `s10_random_events.py` | §10 random card events: the fitted rate, what leaving them out costs on real decks, and the same targets on decks without such a card |
| `s11_conditional.py` | §11 conditional character events: how often they fire in the runs, what leaving them out costs per trainee and target, Tokai Teio's Head-On |
| `s13_calibration.py` | §13 the calibration against real careers (by source, priority and predicted band, every gold, P(all) with a two-sample bootstrap), §3's test on whole careers with the parent forms rescored, §9's certainty checks |

§12 (the composed model) is derived and has no script. Comparisons follow the three rules in *How to read*: same
parameter count, compared directly; an extra parameter, by likelihood ratio (detectable or not); whether a model
predicts better, by 5-fold cross-validation by run id (`common.fold`, `common.compare_held_out`).

Analyses use the pooled runs subject to their stated eligibility restrictions. What the runs cannot test is labelled **assumed** in the document and
claims no evidence: §1's spark curve and §2's star mixes (community figures). The bases of the spark types the odds do
not use are listed only in `docs/MODEL.md`.

**Data.** The measured values rest on Independent Training runs from uma-it, read as run records
(`research/data/runs.jsonl`; every run record is part of the pool) and, for §2, the run list's rank-score estimates
(`research/data/summary.jsonl`). **The run records are not in this repository.** Without
them the scripts compute only what needs game data, and `results.json` keeps the committed values for the rest, so a
clone still renders the whole document. Everything else comes from the repository: `pkg/loopopt/data/global/`, the
GameTora files (through `pkg/loopopt/export/gametora.py`) and `game/global/master.mdb`. Section 6 recomputes the odds of
every validation result (`val/ref__*`, `val/refpu__*`) through `val/joint_check.py`: about a minute. Section 13 also runs the native
solver (`cd pkg/loopopt && cargo build --release`): a few minutes for the default targets, more for the other-skills
sample, one process; results are kept in `research/data/out/` and reused while nothing they read changes.
numpy and scipy come with the `research` dependency group.

The chain link checks score each observed marker against its link's reach probability with the marker's background
and the allowance applied, as the fits do. Sections 10 and 11 reconstruct acquisition from exact landing marginals and
approximate report gold chances on both sides of their comparisons. These checks do not establish exact acquisition
calibration. Section 13 reports acquisition reliability, including zero predictions, for the report approximation.

**Rules of the analysis.** Pooled counts; intervals and tests treat a run as the unit wherever a run gives several
rows (`common.cluster_ci`, `common.cluster_chi2`, run-clustered standard errors), Wilson intervals where it gives one;
nothing split or explained by who ran a career; R cards
left out of measured results unless a fit says it includes them; every comparison is against the model on the run's
own deck. A number comes out as the data give it, and the text follows the number, never the reverse.
