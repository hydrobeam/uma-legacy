// ---------- deck card panel: name, role, ownership (the same setting as the card picker): which one is open ----------
// A click on a deck card, a member's name or a portrait in a result opens its panel (ui/CardPop.tsx draws it); a click
// on the one already open, or outside every panel, closes it. The panel shows where its opener is: in the results
// column, or in the popup (a ui/Dialog) the opener sits in, whose slot draws it there (the page behind a modal popup is
// out of reach). What the panel shows is read from the opener's data-* at the click.
// Keyboard: opening moves the focus into the panel; Escape, or Tab past either end, closes it and returns the focus to
// the card or name that opened it.
import { computed, signal } from '@preact/signals-core';
import type { ReadonlySignal } from '@preact/signals-core';
import { must } from './must.ts';

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
// a click on what is already open closes it; on another card or name, its panel opens instead
function toggleFor(opener: HTMLElement, beside: boolean, subject: () => PopSubject) {
  if (open.peek()?.opener === opener) closePop();
  else open.value = { opener, home: opener.closest('dialog')?.id ?? null, beside, subject: subject() };
}
// a member's panel, from her member block (a portrait elsewhere in the result passes it)
const umaOf = (member: HTMLElement): PopSubject => ({
  kind: 'uma',
  uma: +must(member.dataset.uma, 'a member block names her'),
  outfit: member.dataset.outfit ? +member.dataset.outfit : null,
  outfitLabel: String(member.dataset.outfitLabel),
  outfitNote: member.dataset.outfitNote || '',
});
export function initPopover() {
  // (a click lands on an element; lib.dom types its target as a bare EventTarget, hence the casts)
  document.addEventListener('click', (e) => {
    const chip = (e.target as Element).closest<HTMLElement>('.deck .dc'),
      name = (e.target as Element).closest<HTMLElement>('.mem .memb');
    if (chip) {
      toggleFor(chip, true, () => ({
        kind: 'card',
        // (a chip without a card entry has no data-cid: no card)
        cid: chip.dataset.cid ? Number(chip.dataset.cid) : null,
        name: must(chip.dataset.name, 'a deck chip names its card'),
        borrowed: chip.classList.contains('bor'),
        end: chip.dataset.end ? +chip.dataset.end : 0,
      }));
      return;
    }
    if (name) {
      toggleFor(name, false, () =>
        umaOf(must(name.closest<HTMLElement>('.mem'), 'a name button is in its member block')),
      );
      return;
    }
    const face = (e.target as Element).closest<HTMLElement>('.loop .facebtn'); // a portrait in a deck title or swap row: the same panel
    if (face) {
      // a "Same loop as #1" card has no member blocks: #1's (the same umas and outfits) stand in
      const mem = `.mem[data-uma="${face.dataset.uma}"]`;
      toggleFor(face, false, () =>
        umaOf(
          must(
            must(face.closest('.loop'), 'a portrait button is in a loop card').querySelector<HTMLElement>(mem) ||
              document.querySelector<HTMLElement>(`.loop.first ${mem}`),
            "#1's loop has a member block for each of its umas",
          ),
        ),
      );
      return;
    }
    if (!(e.target as Element).closest('.cpop')) closePop();
  });
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && open.peek()) popBack();
  });
}
