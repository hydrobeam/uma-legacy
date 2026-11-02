// ---------- the veteran card's parts the views share: the game's "Umamusume Details" screen ----------
// What Try a run's card (career/analysis/Details.tsx), the node popups and the decks share: the spark names' fit
// (`fitSparks`) and a deck's cards as they are drawn (`deckCards`). (Its Skill Details popup is
// career/view/SkillDetails.tsx.)
import { gameData } from './model/data.ts';
import type { Card } from './model/data.ts';
import type { ResultDeck } from './loop/types.ts';
import type { DeepReadonly } from './state/store.ts';

// a spark bar's name on one line, its type shrunk until it fits (the game's way), down to 9 px; panes on show only.
// The names are ui/Sparks.tsx's, marked data-spark-name.
// Measured, so it runs once the names are drawn and on show: from a layout effect of the component that draws them
// (after its dialog's own has opened it), and on a resize (the career page's entry).
// Each name takes the largest size f0 - 0.5k that fits, or the first one at or under 9 px (a step-by-step shrink). All
// names are sized together: every round writes each unsettled name's next size to try, then reads them all (one layout
// per round, not one per step per name), halving the range of steps left (the width only grows with the size).
export function fitSparks(root: ParentNode): void {
  const names = [...root.querySelectorAll<HTMLElement>('[data-spark-name]')];
  for (const e of names) e.style.fontSize = '';
  const fits = (e: HTMLElement): boolean => !(e.scrollWidth > e.clientWidth + 0.5);
  // per name that overflows (and is on show): base size f0 in px, the steps known not to fit (lo) and known to fit, or
  // the floor (hi); step k is the size f0 - 0.5k, and k = 0 is the CSS size; mid: the step being tried
  const shrinking: { e: HTMLElement; f0: number; lo: number; hi: number; mid: number }[] = names
    .filter((e) => e.clientWidth)
    .map((e) => ({ e, f0: parseFloat(getComputedStyle(e).fontSize) }))
    .filter((x) => !fits(x.e))
    .map((x) => {
      let floorStep = 0;
      while (x.f0 - 0.5 * floorStep > 9) floorStep++;
      return { ...x, lo: 0, hi: floorStep, mid: 0 };
    });
  // the answer is the smallest step that fits, else hi
  let open = shrinking.filter((x) => x.hi - x.lo > 1);
  while (open.length) {
    for (const x of open) {
      x.mid = (x.lo + x.hi) >> 1;
      x.e.style.fontSize = `${x.f0 - 0.5 * x.mid}px`;
    }
    for (const x of open) {
      if (fits(x.e)) x.hi = x.mid;
      else x.lo = x.mid;
    }
    open = open.filter((x) => x.hi - x.lo > 1);
  }
  for (const x of shrinking) if (x.hi > 0) x.e.style.fontSize = `${x.f0 - 0.5 * x.hi}px`;
}
/** One card of a deck report, as the views draw it (`deckCards`). */
export interface DeckSlot {
  /** its place in the report's `cards` */
  i: number;
  /** its name as the report writes it, with its notes (" (end chain at link N)", " (borrowed)") */
  named: string;
  /** its name without them */
  base: string;
  borrowed: boolean;
  /** its data (undefined when the page has no such card) */
  card: Card | undefined;
  /** the chain link to end its story at (0: none) */
  endAt: number;
  /** the skill the search chose to prioritise at its chain link ('': none) */
  choice: string;
}
// A deck report's cards (the solver's `PDeck::out`), in its order. The report lists the deck's cards with ids
// (card_ids) and then the plain ones (plain_ids, then any filler slot), in the order of `cards`. The per-card advice
// lists (`end_chain_at`, `choices`) hold one entry per card with an id, so the plain pals and fillers after them have
// none: no end link, no choice. (A report without the lists at all, which the type allows, reads the same.)
export function deckCards(deck: DeepReadonly<ResultDeck>): DeckSlot[] {
  const ids = [...deck.card_ids, ...(deck.plain_ids || [])];
  const slots = deck.cards.map((named, i): DeckSlot => {
    const base = named
      .replace(/ \(borrowed\)$/, '')
      .replace(/ \(end chain at link \d\)$/, '')
      .replace(/ \(link \d+: [^)]*\)$/, '');
    const id = ids[i]; // (an id the report lacks finds no card: the name finds it then)
    return {
      i,
      named,
      base,
      borrowed: /\(borrowed\)$/.test(named),
      card: (id !== undefined && gameData().cardById.get(id)) || gameData().cards.find((x) => x.name === base),
      endAt: deck.end_chain_at?.[i] ?? 0,
      choice: deck.choices?.[i] ?? '',
    };
  });
  return slots;
}
/** `slots` with the borrowed card last (the sixth slot, as in the game), the others in their order */
export const borrowedLast = (slots: readonly DeckSlot[]): DeckSlot[] => [
  ...slots.filter((s) => !s.borrowed),
  ...slots.filter((s) => s.borrowed),
];
