// ---------- aptitude grades: a uma's grade at an aptitude, and runtime checks of grades and codes read from outside ----------
// Grades are the solver's (gen/solver.ts `Grades`: each kind's grades at their codes' places, `distance[Distance]`,
// `surface[Surface]`, `style[Style]`); where an aptitude's grade is comes from the solver too (gen/constants.ts
// `aptitude_kinds`). The checks are for what a file, a save or uma.moe hands the page, which no type covers at runtime.
import { Grade } from '../gen/solver.ts';
import type { Aptitude, Grades } from '../gen/solver.ts';
import { SOLVER } from '../gen/constants.ts';
import { assert } from '../must.ts';
import type { DeepReadonly } from '../state/store.ts';

/** her grade at `aptitude` */
export function gradeOf(grades: DeepReadonly<Grades>, aptitude: Aptitude): Grade {
  const at = SOLVER.aptitude_kinds[aptitude];
  switch (at.kind) {
    case 'distance':
      return grades.distance[at.distance];
    case 'surface':
      return grades.surface[at.surface];
    case 'style':
      return grades.style[at.style];
  }
}
/** `grades` with her grade at `aptitude` set to `grade` */
export function withGrade(grades: DeepReadonly<Grades>, aptitude: Aptitude, grade: Grade): Grades {
  const at = SOLVER.aptitude_kinds[aptitude];
  const next: Grades = { distance: [...grades.distance], surface: [...grades.surface], style: [...grades.style] };
  switch (at.kind) {
    case 'distance':
      next.distance[at.distance] = grade;
      break;
    case 'surface':
      next.surface[at.surface] = grade;
      break;
    case 'style':
      next.style[at.style] = grade;
      break;
  }
  return next;
}

// ---------- checks of what the page reads from outside
const isGrade = (x: unknown): x is Grade => typeof x === 'number' && Grade[x] !== undefined;
/** whether `code` (a pink spark's factor group, read from a file, a save or uma.moe) is an aptitude the solver knows */
export const isAptitude = (code: number): code is Aptitude => Object.hasOwn(SOLVER.aptitude_kinds, code);
/** every aptitude the solver knows, in code order (turf, dirt, the styles, the distances) */
export const APTITUDES: readonly Aptitude[] = Object.keys(SOLVER.aptitude_kinds)
  .map(Number)
  .filter(isAptitude)
  .sort((a, b) => a - b);
/** how many grades of each kind a `Grades` holds: as many as the solver has aptitudes of that kind */
const KIND_SIZES = { distance: 0, surface: 0, style: 0 };
for (const at of Object.values(SOLVER.aptitude_kinds)) KIND_SIZES[at.kind]++;
const isGradeList = (x: unknown, size: number): boolean => Array.isArray(x) && x.length === size && x.every(isGrade);
/** whether `x` (a stored or pasted veteran's `apt`) is a uma's grades */
export const isGrades = (x: unknown): x is Grades =>
  typeof x === 'object' &&
  x !== null &&
  'distance' in x &&
  'surface' in x &&
  'style' in x &&
  isGradeList(x.distance, KIND_SIZES.distance) &&
  isGradeList(x.surface, KIND_SIZES.surface) &&
  isGradeList(x.style, KIND_SIZES.style);
/** a uma's grades from a save written before grades had kinds: a list of ten codes, the distances, the surfaces and
 *  the styles in turn (null: not such a list) */
export function gradesOfList(x: unknown): Grades | null {
  if (!Array.isArray(x) || !isGradeList(x, KIND_SIZES.distance + KIND_SIZES.surface + KIND_SIZES.style)) return null;
  const at = (k: number): Grade => {
    const grade: unknown = x[k];
    assert(isGrade(grade), 'a checked list holds grades');
    return grade;
  };
  return { distance: [at(0), at(1), at(2), at(3)], surface: [at(4), at(5)], style: [at(6), at(7), at(8), at(9)] };
}
