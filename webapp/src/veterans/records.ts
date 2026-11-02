// ---------- a veteran as the solver's job takes her: her spark maps as the solver's records (gen/solver.ts) ----------
// The veterans store keeps sparks as maps (group -> stars, veterans/store.ts `Sparks`); a job sends them as the solver's
// `Spark` and `Pink` records. A pink map read from a file or a save keeps only the aptitudes the solver knows, with 1 to
// 3 stars (`knownPinks`), so a job never carries one the solver refuses.
import { isAptitude } from '../grades.ts';
import type { Ancestor, Legacy, Pink, Spark } from '../gen/solver.ts';
import type { DeepReadonly } from '../store.ts';

/** A veteran or a parent as the solver's job takes her (an unknown parent's character 0: no relation points). */
export type JobNode = DeepReadonly<Required<Ancestor>>;
/** A veteran as the solver's job takes her, with her recorded parents. */
export type JobLegacy = DeepReadonly<Required<Legacy>>;
/** a spark map: group -> stars */
type SparkMap = DeepReadonly<Record<number, number>>;

const isStars = (stars: unknown): stars is number =>
  typeof stars === 'number' && Number.isInteger(stars) && stars >= 1 && stars <= 3;
/** white sparks as the solver's records */
const sparkRecords = (sparks: SparkMap | undefined): Spark[] =>
  Object.entries(sparks || {}).map(([group, stars]) => ({ group: +group, stars }));
/** pink sparks as the solver's records: those of an aptitude it knows, with 1 to 3 stars */
function pinkRecords(pinks: SparkMap | undefined): Pink[] {
  const records: Pink[] = [];
  for (const [code, stars] of Object.entries(pinks || {})) {
    const aptitude = +code;
    if (isAptitude(aptitude) && isStars(stars)) records.push({ aptitude, stars });
  }
  return records;
}
/** a node of the veterans store as the job takes her */
export const jobNode = (node: {
  chara: number | null;
  sparks: SparkMap;
  pinks: SparkMap;
  wins: readonly number[];
}): JobNode => ({
  chara: node.chara || 0,
  sparks: sparkRecords(node.sparks),
  pinks: pinkRecords(node.pinks),
  wins: node.wins,
});
/** a pink map read from a file or a save (`raw`: anything): the aptitudes the solver knows, with 1 to 3 stars, and how
 *  many it left out */
export function knownPinks(raw: unknown): { pinks: Record<number, number>; unknown: number } {
  const pinks: Record<number, number> = {};
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return { pinks, unknown: 0 };
  let unknown = 0;
  for (const [code, stars] of Object.entries(raw)) {
    const aptitude = +code;
    if (isAptitude(aptitude) && isStars(stars)) pinks[aptitude] = stars;
    else unknown++;
  }
  return { pinks, unknown };
}
/** how many pink sparks of a stored or pasted veteran (`raw`: anything) and of her parents `knownPinks` leaves out */
export function unknownPinksIn(raw: unknown): number {
  if (typeof raw !== 'object' || raw === null) return 0;
  const own = 'pinks' in raw ? knownPinks(raw.pinks).unknown : 0;
  const parents: unknown[] = 'parents' in raw && Array.isArray(raw.parents) ? raw.parents : [];
  return own + parents.reduce((count: number, parent) => count + unknownPinksIn(parent), 0);
}
/** what an import says of the pink sparks it left out (`unknown`: how many; '' for none) */
export const unknownPinksSaid = (unknown: number): string =>
  unknown ? ` Left out ${unknown} pink spark${unknown === 1 ? '' : 's'} of an unknown aptitude.` : '';
