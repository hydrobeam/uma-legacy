# Research: reproducing docs/RESEARCH.md

**The run records are not in this repository.** The measured claims rest on Independent Training runs from uma-it,
read as run records (`research/data/runs.jsonl` and `summary.jsonl`), which are not published; `docs/RESEARCH.md`
gives their aggregates. Without the records these scripts say so and check only what comes from game data and the
derived numbers.

One script per section of `docs/RESEARCH.md` that has something to recompute. Each prints the section's numbers next
to the value the document states; `run_all.py` runs them all and prints one table: claim, stated, reproduced, verdict.

```
uv run --group research research/run_all.py           # or ./build.sh research
uv run --group research research/run_all.py --quick   # without section 12's run over every other skill
uv run --group research research/s07_chains.py        # one section, with its detail
```

| Script | Reproduces |
|---|---|
| `s02_star_mix.py` | §2 each scenario's share of runs reaching rank score 17,500 (which picks its star-mix band); the expected base per roll |
| `s04_affinity.py` | §4 the self-grandparent fit (trio points 0 against pair(T, P), κ) |
| `s05_loops.py` | §5 the race weights 20 / 10, by expanding every lineage's six affinities; trainees with their own character as a grandparent |
| `s06_hints.py` | §6 training hints: bases per scenario, the form comparison, the Lv-5 cap, joint draws, spark deficit |
| `s07_chains.py` | §7 chain events: the fit of N, the marker allowance, final links by pals, the allocation rule, the exact link tables, known misses |
| `s08_outcomes.py` | §8 measured outcome shares and the choice-rule evidence |
| `s09_random_events.py` | §9 random card events: the fitted rate and the form comparison |
| `s10_conditional.py` | §10 conditional character events: counts, targets touched, golds |
| `s12_calibration.py` | §12 the calibration against real careers, §3's inheritance test, §8's certainty checks |

Not covered, because there is nothing here to recompute them from: §1's spark curve and §2's star-mix shares (external
fits on Champions Meeting data), §3's base rates and §4's race win table (community figures), §0 and §11 (definitions
and derivations; `val/joint_check.py` recomputes the solver's exact odds independently), and the solver and simulator
experiments in §5, §10 and §11 (each listed as **needs solver experiment**).

Verdicts: **match** (equal at the stated precision; counts exactly), **within tolerance** (log-likelihoods within 0.1,
bootstrap bounds within 0.01), **differs**, **needs data**, and **needs solver experiment** or **not reproduced** where
a claim needs work the scripts don't do (the line says why).

**Data.** Besides the run records, everything comes from the repository: `pkg/loopopt/data/global/`, the GameTora
files (through `pkg/loopopt/export/gametora.py`) and `game/global/master.mdb` (section 12 and the card-level readings
need it). Section 12 also runs the native solver (`cd pkg/loopopt && cargo build --release`): about a minute for the
default targets, several more for the other-skills sample, one process; results are kept in `research/data/out/` and
reused while nothing they read changes. numpy and scipy come with the `research` dependency group.

**Rules of the analysis.** Pooled counts with Wilson 95% intervals; nothing split or explained by who ran a career; R
cards left out of measured results unless a stated fit included them; every comparison is against the model on the
run's own deck. A number that does not reproduce is reported as it comes out, never tuned toward the stated value.
