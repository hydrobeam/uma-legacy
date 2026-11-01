// ---------- the veteran card's parts the views share: the game's "Umamusume Details" screen ----------
// What Try a run's card (career/analysis/Details.tsx), the node popups and the decks share: the spark names' fit
// (`fitSparks`) and a deck's card by its index (`deckCardAt`). (Its Skill Details popup is career/view/SkillDetails.tsx.)
import { gameData } from './data.js';

// a spark bar's name on one line, its type shrunk until it fits (the game's way), down to 9 px; panes on show only
// Each name takes the largest size f0 - 0.5k that fits, or the first one at or under 9 px (a step-by-step shrink). All
// names are sized together: every round writes each unsettled name's next size to try, then reads them all (one layout
// per round, not one per step per name), halving the range of steps left (the width only grows with the size).
const fitting = new Set(); // roots with a pass already queued (resize fires many times a frame)
/** @param {ParentNode} [root] */
export function fitSparks(root = document) {
  if (fitting.has(root)) return;
  fitting.add(root);
  requestAnimationFrame(() => {
    fitting.delete(root);
    const names = [...root.querySelectorAll('.ud .vskn')];
    for (const e of names) e.style.fontSize = '';
    const fits = (e) => !(e.scrollWidth > e.clientWidth + 0.5);
    // per name that overflows (and is on show): base size f0 in px, the steps known not to fit (lo) and known to fit, or
    // the floor (hi); step k is the size f0 - 0.5k, and k = 0 is the CSS size; mid: the step being tried
    /** @type {{e: HTMLElement, f0: number, lo: number, hi: number, mid: number}[]} */
    const shrinking = names
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
  });
}
addEventListener('resize', () => fitSparks());
// A deck's i-th card, from the solver's deck report: its name with the report's notes taken off (base), whether it is the
// borrowed card, and the card's data (null when unknown). The report lists the deck's cards with ids (card_ids) and
// then the plain ones (plain_ids), in the order of `cards`.
/** @param {import('./loop/types.ts').ResultDeck} deck @param {number} i @returns {{base: string, borrowed: boolean, card: import('./data.js').Card|undefined}} */
export function deckCardAt(deck, i) {
  const name = deck.cards[i];
  const borrowed = /\(borrowed\)$/.test(name),
    base = name
      .replace(/ \(borrowed\)$/, '')
      .replace(/ \(end chain at link \d\)$/, '')
      .replace(/ \(link \d+: [^)]*\)$/, '');
  const id = deck.card_ids[i] ?? deck.plain_ids?.[i - deck.card_ids.length];
  return { base, borrowed, card: gameData().cardById.get(id) || gameData().cards.find((x) => x.name === base) };
}
