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
/** What the draw reads of the report's `aptitudes` (career/job.ts `CareerReport`). */
export interface PinkAptitudes {
  start: readonly number[];
  /** each legacy pink: the aptitude it raises and its chance of firing at one inspiration */
  fire: readonly { apt: number; p: number }[];
  /** per aptitude, the pink spark's group */
  pink_gen: readonly { group: number }[];
}
/** The pink she draws (group -> stars; empty when none) and her end grades (null without `aptitudes`). */
export interface PinkDraw {
  pinks: Record<number, number>;
  apt: number[] | null;
}
export function drawPink(ap: PinkAptitudes | null | undefined, rnd: () => number = Math.random): PinkDraw {
  if (!ap) return { pinks: {}, apt: null };
  // the random draws keep this order (the suite's seeded check replays it): every fire chance per inspiration in
  // `ap.fire` order, then the aptitude, then her stars
  const grades = ap.start.slice();
  for (let inspiration = 0; inspiration < INSPIRATIONS; inspiration++) {
    for (const fire of ap.fire) if (rnd() < fire.p) grades[fire.apt] = Math.min(GRADE_S, gradeAt(grades, fire.apt) + 1);
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
