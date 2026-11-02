// ---------- a veteran as the solver's job takes her: her spark maps as the solver's records (gen/solver.ts) ----------
// The veterans store keeps sparks as maps (group -> stars, veterans/store.ts `Sparks`); a job sends them as the solver's
// `Spark` and `Pink` records. A pink map read from a file or a save keeps only the aptitudes the solver knows, with 1 to
// 3 stars (`knownPinks`), so a job never carries one the solver refuses. A job takes a veteran only when her record is
// whole (`jobLegacy`): her uma and her one pink, and each parent place entered with hers or absent (complete.ts `lacks`
// says what one that is not whole lacks).
import { isAptitude } from '../model/grades.ts';
import type { Ancestor, Legacy, Pink, Spark } from '../gen/solver.ts';
import type { DeepReadonly } from '../state/store.ts';

/** A veteran's parent as the solver's job takes her. */
export type JobNode = DeepReadonly<Required<Ancestor>>;
/** A veteran as the solver's job takes her, with her two parent places (null: a parent her record doesn't have). */
export type JobLegacy = DeepReadonly<Required<Legacy>>;
/** a spark map: group -> stars */
type SparkMap = DeepReadonly<Record<number, number>>;
/** a node of the veterans store, as far as a job reads it */
interface Node {
  chara: number | null;
  sparks: SparkMap;
  pinks: SparkMap;
  wins: readonly number[];
}

const isStars = (stars: unknown): stars is number =>
  typeof stars === 'number' && Number.isInteger(stars) && stars >= 1 && stars <= 3;
/** white sparks as the solver's records */
const sparkRecords = (sparks: SparkMap | undefined): Spark[] =>
  Object.entries(sparks || {}).map(([group, stars]) => ({ group: +group, stars }));
/** her pink spark as the solver's record: the one of an aptitude it knows, with 1 to 3 stars (null: none, or more than
 *  one: a trained uma has one) */
export function pinkRecord(pinks: SparkMap | undefined): Pink | null {
  const records: Pink[] = [];
  for (const [code, stars] of Object.entries(pinks || {})) {
    const aptitude = +code;
    if (isAptitude(aptitude) && isStars(stars)) records.push({ aptitude, stars });
  }
  return records.length === 1 ? (records[0] ?? null) : null;
}
/** a node of the veterans store as the job takes her (null: her uma or her pink not known) */
function jobNode(node: Node): JobNode | null {
  const pink = pinkRecord(node.pinks);
  if (!node.chara || !pink) return null;
  return { chara: node.chara, sparks: sparkRecords(node.sparks), wins: node.wins, pink };
}
/** a parent place as the job takes it: her record, or null where the record she came from lacks that parent (absent:
 *  veterans/store.ts `ParentNode`); undefined for one not entered (no uma) or without her one pink: not whole */
function jobParent(node: Node | null): JobNode | null | undefined {
  return node === null ? null : (jobNode(node) ?? undefined);
}
/** a veteran as the job takes her, with her parents (null: her record is not whole, so no job takes her) */
export function jobLegacy(v: Node & { borrowed?: boolean; parents: readonly (Node | null)[] }): JobLegacy | null {
  const her = jobNode(v),
    [first, second] = v.parents.map(jobParent);
  if (!her || first === undefined || second === undefined) return null;
  return { ...her, borrowed: !!v.borrowed, parents: [first, second] };
}
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
