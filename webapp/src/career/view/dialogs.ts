// Which of the career page's popups is open, and for what: the node popup (its node), the deck popup and the parent
// picker (its place). The views (Tree.tsx, NodeDialog.tsx, Sections.tsx, ParentPicker.tsx) draw from these; the other
// views (the pool, the veteran analysis, uma.moe's list) open them through the functions here, and import no component
// to do it.
import { signal } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import type { ParentPlace } from '../../veterans/store.ts';

/** a node of the tree or the pool: the trainee, parent place k, parent k's parent j, a veteran of the pool */
export type NodeSpec =
  | { kind: 't' }
  | { kind: 'p'; k: number }
  | { kind: 'g'; k: number; j: ParentPlace }
  | { kind: 'v'; id: string };
const node = signal<NodeSpec | null>(null),
  deck = signal(false),
  picker = signal<number | null>(null);
/** the node on show in the node popup (null: closed) */
export const nodeOpen: ReadonlySignal<NodeSpec | null> = node;
/** the deck popup is open */
export const deckOpen: ReadonlySignal<boolean> = deck;
/** the parent picker's place (null: closed) */
export const pickerFor: ReadonlySignal<number | null> = picker;

// where the focus goes when the node popup closes, when the element that opened it is gone (a pool card drawn again)
let nodeBack: (() => void) | null = null;
/** the node popup for `spec`; `back` puts the focus back when its opener has been drawn again by then */
export function openNode(spec: NodeSpec, back: (() => void) | null = null) {
  nodeBack = back;
  node.value = spec;
}
/** the node popup closes (its `back` is kept for `nodeClosed`) */
export const closeNode = () => {
  node.value = null;
};
/** the node popup has closed: it hands its fallback for the focus, once */
export function nodeClosed(): (() => void) | null {
  const back = nodeBack;
  nodeBack = null;
  node.value = null;
  return back;
}
export const openDeck = () => {
  deck.value = true;
};
export const deckClosed = () => {
  deck.value = false;
};
export const openParentPicker = (k: number) => {
  picker.value = k;
};
export const pickerClosed = () => {
  picker.value = null;
};
