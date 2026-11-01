// ---------- deck card panel: name, role, ownership (the same setting as the card picker): which one is open ----------
// A click on a deck card, a member's name or a portrait in a result opens its panel (ui/CardPop.tsx draws it); a click
// on the one already open, or outside every panel, closes it. The panel shows where its opener is: in the results
// column, or in the popup (a ui/Dialog) the opener sits in, whose slot draws it there (the page behind a modal popup is
// out of reach: a popover over a modal dialog is inert there, in Chromium and Firefox). What the panel shows, each
// opener says (its own props: `usePopOpener`).
// Keyboard: opening moves the focus into the panel; Escape, or Tab past either end, closes it and returns the focus to
// the card or name that opened it.
import { computed, signal } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';

/** what a panel is about: a deck card (by its card id; a chip with no card entry has none) or a loop's member */
export type PopSubject =
  | { kind: 'card'; cid: number | null; name: string; borrowed: boolean; end: number }
  | { kind: 'uma'; uma: number; outfit: number | null; outfitLabel: string; outfitNote: string };
/** the panel open: its opener, where it shows (the id of the popup its opener sits in; null: the results column), whether
 *  it opens beside its opener (a deck card) or under it, and what it is about */
export interface PopOpen {
  opener: HTMLElement;
  home: string | null;
  beside: boolean;
  subject: PopSubject;
}
const open = signal<PopOpen | null>(null);
/** the panel open, if any */
export const popOpen: ReadonlySignal<PopOpen | null> = open;
// Every opener is a component's: it draws its own aria-expanded from `popOpener`, and its own greying (`off`) from the
// settings, so nothing here writes into it.
/** the button whose panel is open (null: none) */
export const popOpener: ReadonlySignal<HTMLElement | null> = computed(() => open.value?.opener ?? null);
export function closePop() {
  open.value = null;
}
// the panel element drawn now (CardPop's), for whether it holds the focus
let panel: HTMLElement | null = null;
export const setPanel = (element: HTMLElement | null) => {
  panel = element;
};
export const popHasFocus = () => !!panel && panel.contains(document.activeElement);
/** close the panel and focus the button that opened it */
export function popBack() {
  const opener = open.peek()?.opener;
  closePop();
  if (opener?.isConnected) opener.focus();
}
/** an opener's click (ui/CardPop.tsx `usePopOpener`): on what is already open it closes the panel; on another card or
 *  name, its panel opens instead, with what its opener says it is about */
export function togglePop(opener: HTMLElement, beside: boolean, subject: PopSubject) {
  if (open.peek()?.opener === opener) closePop();
  else open.value = { opener, home: opener.closest('dialog')?.id ?? null, beside, subject };
}
/** a click outside the open panel and its opener closes it (the opener's own click toggles it); Escape closes it, back
 *  to its opener */
export function initPopover() {
  // (a click lands on an element; lib.dom types its target as a bare EventTarget, hence the casts)
  document.addEventListener('click', (e) => {
    const now = open.peek(),
      target = e.target as Element;
    if (now && !now.opener.contains(target) && !target.closest('[data-pop]')) closePop();
  });
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && open.peek()) popBack();
  });
}
