// Her pink spark drawn (MODEL.md) from a career's `aptitudes` (the solver's report): her start grades, each legacy pink
// firing at each of the two April inspirations with its chance (+1 grade, at most S), then one aptitude uniform over those
// she ends at A or S, stars 20/70/10. The solver states the same model as each aptitude's chance of being the pink
// (`aptitudes.pink_gen`, `Env::pink_gen`); the browser suite's `career` group pins this draw against those chances on the
// reference outputs (val/ref__global__career_*.out). `rnd`: the random source (tests pass a seeded one).
// Grades are the solver's (gen/solver.ts `Grades`); stars are 1 to 3.
import { gradeOf, withGrade } from './grades.ts';
import { Grade } from './gen/solver.ts';
import type { Aptitude, Aptitudes, Grades, PinkOutcomeReport, WeightedPinkOutcome } from './gen/solver.ts';
import type { DeepReadonly } from './store.ts';
import { must } from './must.ts';

// April inspirations in a career: each legacy pink gets a chance to fire at each
const INSPIRATIONS = 2;
/** The pink she draws (aptitude -> stars; empty when none) and her end grades (null without `aptitudes`). */
export interface PinkDraw {
  pinks: Record<number, number>;
  apt: Grades | null;
}
export function drawPink(ap: DeepReadonly<Aptitudes> | null | undefined, rnd: () => number = Math.random): PinkDraw {
  if (!ap) return { pinks: {}, apt: null };
  // the random draws keep this order (the suite's seeded check replays it): with several starts, one for the start (by
  // their chances, as the solver's `draw_pink`); every fire chance per inspiration in `fire` order, then the aptitude,
  // then her stars
  const { start, fire } = ap.start.kind === 'outcomes' ? drawnStart(ap.start.outcomes, rnd()) : ap.start;
  let grades: Grades = { distance: [...start.distance], surface: [...start.surface], style: [...start.style] };
  for (let inspiration = 0; inspiration < INSPIRATIONS; inspiration++) {
    for (const f of fire) {
      if (rnd() < f.p) grades = withGrade(grades, f.aptitude, Math.min(Grade.S, gradeOf(grades, f.aptitude) + 1));
    }
  }
  // the pinks of the aptitudes she ends at A or S (`pink_gen`: the solver's pink per aptitude, in its order)
  const atLeastA = ap.pink_gen.filter(({ aptitude }) => gradeOf(grades, aptitude) >= Grade.A);
  return {
    pinks: pinkAmong(
      atLeastA.map(({ aptitude }) => aptitude),
      rnd,
    ),
    apt: grades,
  };
}
/** Her pink among `ends` (the aptitudes she ends at A or S, in the order the draw reads them): one aptitude uniform
 *  over them, then her stars, 20/70/10 (aptitude -> stars; empty when there are none). */
export function pinkAmong(ends: readonly Aptitude[], rnd: () => number): Record<number, number> {
  if (!ends.length) return {};
  const aptitude = must(ends[Math.floor(rnd() * ends.length)], 'a uniform draw from a list lands in it'),
    u = rnd();
  const stars = u < 0.2 ? 1 : u < 0.9 ? 2 : 3;
  return { [aptitude]: stars };
}
// the start `u` (uniform in [0, 1)) falls in, by the starts' chances in order; the last where rounding leaves u past them
function drawnStart(outcomes: DeepReadonly<WeightedPinkOutcome[]>, u: number): DeepReadonly<PinkOutcomeReport> {
  let remaining = u;
  for (const { p, outcome } of outcomes) {
    remaining -= p;
    if (remaining < 0) return outcome;
  }
  return must(outcomes[outcomes.length - 1], 'a career has a start').outcome;
}
