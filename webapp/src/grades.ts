// ---------- aptitude grades: the solver's and data.json's codes 1..8 = G..S ----------
// A list of grades has one per aptitude, in data.json's `apt` order: sprint, mile, medium, long, turf, dirt, then the
// four running styles. The lists come from the solver's reports, the veterans' records and data.json as plain numbers;
// `gradeAt` is where a number becomes a Grade, checked.
import { assert } from './must.ts';

export type Grade = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8;
/** each grade's letter */
export const GRADE_LETTER: Record<Grade, string> = { 1: 'G', 2: 'F', 3: 'E', 4: 'D', 5: 'C', 6: 'B', 7: 'A', 8: 'S' };
const isGrade = (x: unknown): x is Grade => typeof x === 'number' && Number.isInteger(x) && x >= 1 && x <= 8;
/** a list of grades, one per aptitude (a stored or pasted veteran's `apt` is checked with this where it is read) */
export const isGrades = (x: unknown): x is Grade[] => Array.isArray(x) && x.length === 10 && x.every(isGrade);
/** aptitude `k`'s grade in `apt` (a report's, a veteran's, data.json's): every list of grades holds one per aptitude,
 *  each a code 1..8 */
export function gradeAt(apt: readonly number[], k: number): Grade {
  const grade = apt[k];
  assert(isGrade(grade), `aptitude ${k} has a grade 1..8`);
  return grade;
}
