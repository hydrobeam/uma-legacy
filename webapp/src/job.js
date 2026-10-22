// ---------- job fields every search sends (the loop planner and the career page) ----------
import { CARDS } from './data.js';
import { state, goldGiven } from './state.js';
// skills a career always gives an uma outside her outfit and the deck: Tokai Teio's Head-On (her Triple Crown event;
// all three races are her career goals) and Yamanin Zephyr's Slipstream (a goal race's; GameTora's event data lacks it).
// Each [character id, skill group, level (1 white)], the solver's `extra` field as it reads it.
export const EXTRA = [[1003, 20190, 1], [1078, 20165, 1]];
// only the scenario guarantees anything
export const jobTargets = () => state.targets.map(t => ({ group: t.group, guaranteed: goldGiven(t.group) ? 2 : 0 }));
// the solver reads an empty list as "no restriction", so with every card not owned it gets [0], an id no card has
export function ownedCards() {
  const notOwned = CARDS.filter(c => state.cardsOff[c.id] === false).length;
  if (!notOwned) return [];
  if (notOwned === CARDS.length) return [0];
  return CARDS.filter(c => state.cardsOff[c.id] !== false).map(c => c.id);
}
// R cards are opt-in
export const rcardsOff = () => state.rcards ? {} : { exclude_cards: CARDS.filter(c => c.rar === 1 && !c.pal).map(c => c.id) };
