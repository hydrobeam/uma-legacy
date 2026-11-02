// ---------- a node's colour sparks (blue, pink, unique): their kinds and their picker ----------
// (The sparks themselves are drawn by ui/Sparks.tsx, the picker by Editors.tsx.)
import { signal } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import { careerTables } from '../data.ts';
import { gradeOf, isAptitude } from '../grades.ts';
import { Grade } from '../gen/solver.ts';
import type { Aptitude } from '../gen/solver.ts';
import { SOLVER } from '../gen/constants.ts';
import { pinkName, blueName, uniqueName } from './names.ts';
import type { NodeRecord, NodeRef } from './store.ts';

/** Her blue / pink / unique spark: one each (the game gives every veteran one blue and one pink; a unique spark is hers
 *  or none). */
interface SparkKind {
  /** the node's field: blues {stat group: stars}, pinks {aptitude group: stars}, greens {outfit card id: stars} */
  field: 'blues' | 'pinks' | 'greens';
  /** the picker's title */
  title: string;
  /** a group's name */
  name: (group: number | string) => string | null;
  /** the groups she can have one of */
  groups: (nd: NodeRecord) => number[];
}
/** a colour spark's kind */
type Colour = 'blue' | 'pink' | 'green';
export const KIND: Record<Colour, SparkKind> = {
  blue: {
    field: 'blues',
    title: 'Blue spark',
    name: blueName,
    groups: () => Object.keys(careerTables().blues || { 1: 1, 2: 1, 3: 1, 4: 1, 5: 1 }).map(Number),
  },
  pink: { field: 'pinks', title: 'Pink spark', name: pinkName, groups: (nd) => pinkChoices(nd) },
  green: {
    field: 'greens',
    title: 'Unique spark',
    name: uniqueName,
    groups: (nd) => (nd.outfit && uniqueName(nd.outfit) ? [nd.outfit] : []),
  },
};
// the aptitudes a pink can come from: those she ends at A or S (a pink is drawn from them only). Her end-of-career
// grades are known for the veteran analysis' draft; a pool veteran's are not (her legacies' pinks raise her uma's base
// grades, two steps from C to A at most per legacy pink), so every aptitude is a choice for her. The pink she has always
// stays a choice. Every choice is an aptitude the solver knows, in code order (turf, dirt, the styles, the distances).
const BY_CODE: readonly Aptitude[] = Object.keys(SOLVER.aptitude_kinds)
  .map(Number)
  .filter(isAptitude)
  .sort((a, b) => a - b);
function pinkChoices(nd: NodeRecord): Aptitude[] {
  const grades = nd.apt;
  const have = Object.keys(nd.pinks || {})
    .map(Number)
    .filter(isAptitude);
  if (!grades) return [...BY_CODE];
  return BY_CODE.filter((aptitude) => gradeOf(grades, aptitude) >= Grade.A || have.includes(aptitude));
}
// her spark of one kind, as [group (an object key: a string), stars], or undefined (none)
export const currentSpark = (nd: NodeRecord, kind: Colour): [string, number] | undefined =>
  Object.entries(nd[KIND[kind].field] || {}).find(([, stars]) => stars > 0);
// the picker (game style, Editors.tsx ColourDialog): the choices as bars of the spark's colour, its stars (click one),
// Done (no remove: every veteran has one blue and one pink). Each pick lands on her record as it is then (`at`).
/** what an opener adds: whose spark it is (named in the title), and what runs once the picker has closed */
interface ColourOpts {
  who?: string;
  onClose?: () => void;
}
/** the colour picker's subject: her node, the kind, its opener's options, whether it is open, and its openings (`n`) */
export interface ColourEdit {
  at: NodeRef;
  kind: Colour;
  opts: ColourOpts;
  open: boolean;
  n: number;
}
const colour = signal<ColourEdit | null>(null);
/** the colour picker's node and kind (null: never opened) */
export const colourEditing: ReadonlySignal<ColourEdit | null> = colour;
export function openColour(at: NodeRef, kind: Colour, opts: ColourOpts = {}): void {
  colour.value = { at, kind, opts, open: true, n: (colour.peek()?.n ?? 0) + 1 };
}
/** the picker has closed (Done, Escape, the backdrop) */
export function colourClosed(): void {
  const now = colour.peek();
  if (!now?.open) return;
  colour.value = { ...now, open: false };
  now.opts.onClose?.();
}
