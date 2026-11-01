// ---------- a node's colour sparks (blue, pink, unique): their kinds and their picker ----------
// (The sparks themselves are drawn by ui/Sparks.tsx, the picker by Editors.tsx.)
import { signal } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import { careerTables, gameData } from '../data.ts';
import { gradeAt } from '../grades.ts';
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
// the aptitudes a pink can come from: those she ends at A or S (a pink is drawn from them only): her end-of-career
// grades when known (the veteran analysis), else her uma's base grades; the pink she has always stays a choice
const APT_OF: Record<number, number> = { 31: 0, 32: 1, 33: 2, 34: 3, 11: 4, 12: 5, 21: 6, 22: 7, 23: 8, 24: 9 };
function pinkChoices(nd: NodeRecord): number[] {
  const grades = nd.apt || gameData().umas.find((u) => u.id === nd.chara)?.apt;
  const all = Object.keys(careerTables().pinks).map(Number),
    have = Object.keys(nd.pinks || {}).map(Number);
  // (a pink of an aptitude the page doesn't map is no choice, unless she has it)
  const endsAtA = (g: number) => {
    const k = APT_OF[g];
    return k !== undefined && !!grades && gradeAt(grades, k) >= 7;
  };
  return grades ? all.filter((g) => endsAtA(g) || have.includes(g)) : all;
}
// her spark of one kind, as [group (an object key: a string), stars], or undefined (none)
export const currentSpark = (nd: NodeRecord, kind: Colour): [string, number] | undefined =>
  Object.entries(nd[KIND[kind].field] || {}).find(([, stars]) => stars > 0);
// the picker (game style, Editors.tsx ColourDialog): the choices as bars of the spark's colour, its stars (click one),
// Done (no remove: every veteran has one blue and one pink). Each pick lands on her record as it is then (`at`).
/** the colour picker's subject: her node, the kind, whether it is open, and its openings (`n`) */
export interface ColourEdit {
  at: NodeRef;
  kind: Colour;
  open: boolean;
  n: number;
}
const colour = signal<ColourEdit | null>(null);
/** the colour picker's node and kind (null: never opened) */
export const colourEditing: ReadonlySignal<ColourEdit | null> = colour;
export function openColour(at: NodeRef, kind: Colour): void {
  colour.value = { at, kind, open: true, n: (colour.peek()?.n ?? 0) + 1 };
}
/** the picker has closed (Done, Escape, the backdrop) */
export function colourClosed(): void {
  const now = colour.peek();
  if (now?.open) colour.value = { ...now, open: false };
}
