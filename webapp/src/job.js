// ---------- job fields every search sends (the loop planner and the career page) ----------
// Each takes the settings it reads (settings.ts: `settings.value`, or the defaults for the example's job).
import { gameData, SCEN_GOLDS } from './data.js';
/** @typedef {import('./settings.ts').SettingsView} SettingsView */
// skills a career always gives an uma outside her outfit and the deck: Tokai Teio's Head-On (her Triple Crown event;
// all three races are her career goals) and Yamanin Zephyr's Slipstream (a goal race's; GameTora's event data lacks it).
// Each [character id, skill group, level (1 white)], the solver's `extra` field as it reads it.
export const EXTRA = [
  [1003, 20190, 1],
  [1078, 20165, 1],
];
// a target the scenario hands out as a gold whatever the deck (Unity Cup: No Stopping Me!, It's On!); nothing else is
// guaranteed outside the deck, so the player never sets it
/** @type {(scen: string, group: number) => boolean} */
export const goldGiven = (scen, group) => (SCEN_GOLDS[scen || 'unity'] || []).includes(group);
// only the scenario guarantees anything
/** @param {SettingsView} s */
export const jobTargets = (s) =>
  s.targets.map((t) => ({ group: t.group, guaranteed: goldGiven(s.scen, t.group) ? 2 : 0 }));
// the solver reads an empty list as "no restriction", so with every card not owned it gets [0], an id no card has
/** @param {SettingsView} s */
export function ownedCards(s) {
  const notOwned = gameData().cards.filter((c) => s.cardsOff[c.id] === false).length;
  if (!notOwned) return [];
  if (notOwned === gameData().cards.length) return [0];
  return gameData()
    .cards.filter((c) => s.cardsOff[c.id] !== false)
    .map((c) => c.id);
}
// R cards (not pals) stay out of every search: settings.ts `rcards`, which no setting turns on
/** @param {SettingsView} s */
export const rcardsOff = (s) =>
  s.rcards
    ? {}
    : {
        exclude_cards: gameData()
          .cards.filter((c) => c.rar === 1 && !c.pal)
          .map((c) => c.id),
      };
