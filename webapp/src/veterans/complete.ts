// ---------- what a veteran lacks before the solver takes her ----------
// The game gives every veteran exactly one pink spark, and shows her two parents (her Legacy Origin), each a veteran
// with her own. So a veteran is complete when she has one pink and each of her parent places holds her parent (an uma
// and one pink), or is absent: the record she came from lacks that parent (a game export's, umadump.ts), which the
// solver reads as no one. One that lacks any of it is incomplete: the page shows her so and leaves her out of the
// solver's jobs (veterans/store.ts `readyVets`, career/job.ts `buildJob`) until her editors finish her. A parent place
// with no uma is one not entered: what a veteran entered by hand holds until her parent is entered (entry.ts), and what
// earlier pages made up for a hand-entered veteran's parents (no uma, her own sparks copied).
import type { DeepReadonly } from '../state/store.ts';
import { pinkRecord } from './records.ts';
import type { LineageNode, ParentNode, ParentPlace } from './store.ts';

/** What she lacks: her uma or her pink, a parent not entered, or an entered parent's pink. A veteran lacking nothing is
 *  one a job takes (records.ts `jobLegacy`: the same rule, read the same way). */
export type Lack = { kind: 'uma' | 'pink' } | { kind: 'parent' | 'parent pink'; j: ParentPlace };
type Node = DeepReadonly<Pick<LineageNode, 'chara' | 'pinks'>>;

/** a parent place not entered yet: no uma, no sparks */
export const notEntered = (): LineageNode => ({ chara: null, sparks: {}, pinks: {}, wins: [] });
// she has exactly one pink, of an aptitude the solver knows, at 1 to 3 stars (as a job reads it: records.ts)
const onePink = (node: Node) => pinkRecord(node.pinks) !== null;
/** what veteran `v` lacks, hers first, then each parent's in place order (none: she is complete) */
export function lacks(v: Node & { parents: DeepReadonly<readonly [ParentNode, ParentNode]> }): Lack[] {
  const lacking: Lack[] = [];
  if (!v.chara) lacking.push({ kind: 'uma' });
  if (!onePink(v)) lacking.push({ kind: 'pink' });
  for (const j of [0, 1] as const) {
    const parent = v.parents[j];
    if (parent === null) continue; // (absent: the record she came from lacks her)
    if (!parent.chara) lacking.push({ kind: 'parent', j });
    else if (!onePink(parent)) lacking.push({ kind: 'parent pink', j });
  }
  return lacking;
}
/** she lacks nothing: the solver can take her */
export const isComplete = (v: Parameters<typeof lacks>[0]): boolean => lacks(v).length === 0;
/** what she lacks, in a few words ("pink, parent 2"; '' for nothing) */
export const lacksSaid = (lacking: readonly Lack[]): string =>
  lacking
    .map((lack) => ('j' in lack ? `parent ${lack.j + 1}${lack.kind === 'parent pink' ? "'s pink" : ''}` : lack.kind))
    .join(', ');
