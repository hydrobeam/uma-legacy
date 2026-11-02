// ---------- the loop page's keys for its results: what identifies a loop, a deck and a result on show ----------
// Results.tsx keys each result card by them, so a redraw with the same loops keeps every element (an open panel, the
// focus); a key must be unique among the results drawn together.
import type { DeepReadonly } from '../state/store.ts';
import type * as Report from './types.ts';
import { loopMembers } from './members.ts';

type LoopResult = DeepReadonly<Report.LoopResult>;
type ResultDeck = DeepReadonly<Report.ResultDeck>;
type ResultLoop = DeepReadonly<Report.ResultLoop>;

/** a loop: its members in order, each with the outfits it offers */
export const loopKey = (loop: ResultLoop) =>
  loopMembers(loop)
    .map((m) => `${m.id}:${m.outfits.map((o) => o.id).join('/')}`)
    .join(' ');
/** a deck: its cards' names (each carries its borrowed and chain-end marks) */
export const deckKey = (deck: ResultDeck) => deck.cards.join('|');
/** each result's key: its loop and decks. Two results can share both: the solver keeps one result per deck and tells
 *  decks apart by their cards' chain choices too (loop_planner/search/mod.rs `SharedDeckKey`'s `variants`), which the
 *  report names only by the choice's label, and two choices can print the same label (sources.rs `label`). So a
 *  repeat is told apart by its place among the repeats ("…#2"): the first keeps the plain key, and a redraw keeps it. */
export function resultKeys(top: readonly LoopResult[]) {
  const seen = new Map<string, number>();
  return top.map((t) => {
    const key = [loopKey(t.loop), t.deck ? deckKey(t.deck) : '', ...(t.decks || []).map(deckKey)].join('#'),
      repeats = seen.get(key) ?? 0;
    seen.set(key, repeats + 1);
    return repeats ? `${key}#${repeats + 1}` : key;
  });
}
