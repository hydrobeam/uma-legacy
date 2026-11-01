// Her pink spark drawn (MODEL.md) from a career's `aptitudes` (the solver's report): her start grades, each legacy pink
// firing at each of the two April inspirations with its chance (+1 grade, at most S), then one aptitude uniform over those
// she ends at A or S, stars 20/70/10. The solver states the same model as each aptitude's chance of being the pink
// (`aptitudes.pink_gen`, `Env::pink_gen`); the browser suite's `career` group pins this draw against those chances on the
// reference outputs (val/ref__global__career_*.out). `rnd`: the random source (tests pass a seeded one).
// Grades are the solver's aptitude codes, 1..8 = G..S, one per aptitude in data.json's `apt` order; stars are 1 to 3.
import { gradeAt } from './grades.ts';
import { assert, must } from './must.ts';

const GRADE_A = 7,
  GRADE_S = 8;
// April inspirations in a career: each legacy pink gets a chance to fire at each
const INSPIRATIONS = 2;
/** One start of a career's pink draw: the grades, and each legacy pink, the aptitude it raises and its chance of firing
 *  at one inspiration */
interface PinkStart {
  start: readonly number[];
  fire: readonly { apt: number; p: number }[];
}
/** What the draw reads of the report's `aptitudes` (the solver's `Aptitudes`): per aptitude the pink spark's group, and
 *  her start, or, with a veteran before her result is known among her legacies, each start with its chance (a job's
 *  own veterans always have one: the page is never sent several) */
export type PinkAptitudes = {
  pink_gen: readonly { group: number }[];
  start: ({ kind: 'known' } & PinkStart) | { kind: 'outcomes'; outcomes: readonly { p: number; outcome: PinkStart }[] };
};
/** The pink she draws (group -> stars; empty when none) and her end grades (null without `aptitudes`). */
export interface PinkDraw {
  pinks: Record<number, number>;
  apt: number[] | null;
}
export function drawPink(ap: PinkAptitudes | null | undefined, rnd: () => number = Math.random): PinkDraw {
  if (!ap) return { pinks: {}, apt: null };
  // the random draws keep this order (the suite's seeded check replays it): with several starts, one for the start (by
  // their chances, as the solver's `draw_pink`); every fire chance per inspiration in `fire` order, then the aptitude,
  // then her stars
  const { start, fire } = ap.start.kind === 'outcomes' ? drawnStart(ap.start.outcomes, rnd()) : ap.start;
  const grades = start.slice();
  for (let inspiration = 0; inspiration < INSPIRATIONS; inspiration++) {
    for (const f of fire) if (rnd() < f.p) grades[f.apt] = Math.min(GRADE_S, gradeAt(grades, f.apt) + 1);
  }
  // the pinks of the aptitudes she ends at A or S (`pink_gen`: the solver's pink per aptitude, in their order)
  assert(ap.pink_gen.length === grades.length, 'the solver names a pink spark per aptitude');
  const atLeastA = ap.pink_gen.filter((_, k) => gradeAt(grades, k) >= GRADE_A);
  if (!atLeastA.length) return { pinks: {}, apt: grades };
  const pink = must(atLeastA[Math.floor(rnd() * atLeastA.length)], 'a uniform draw from a list lands in it'),
    u = rnd();
  const stars = u < 0.2 ? 1 : u < 0.9 ? 2 : 3;
  return { pinks: { [pink.group]: stars }, apt: grades };
}
// the start `u` (uniform in [0, 1)) falls in, by the starts' chances in order; the last where rounding leaves u past them
function drawnStart(outcomes: readonly { p: number; outcome: PinkStart }[], u: number): PinkStart {
  let remaining = u;
  for (const { p, outcome } of outcomes) {
    remaining -= p;
    if (remaining < 0) return outcome;
  }
  return must(outcomes[outcomes.length - 1], 'a career has a start').outcome;
}
