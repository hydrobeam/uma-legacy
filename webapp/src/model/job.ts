// ---------- job fields every search sends (the loop planner and the career page) ----------
// Each takes the settings it reads (settings.ts: `settings.value`, or the defaults for the example's job).
import { gameData, SCEN_GOLDS } from './data.ts';
import type { Uma } from './data.ts';
import type { Scenario } from '../gen/solver.ts';
import type { SettingsView } from '../settings.ts';

export type { SettingsView };
// skills a career always gives an uma outside her outfit and the deck: Tokai Teio's Head-On (her Triple Crown event;
// all three races are her career goals) and Yamanin Zephyr's Slipstream (a goal race's; GameTora's event data lacks it).
// Each [character id, skill group, level (1 white)], the solver's `extra` field as it reads it.
const EXTRA: [number, number, number][] = [
  [1003, 20190, 1],
  [1078, 20165, 1],
];
// a target the scenario hands out as a gold whatever the deck (Unity Cup: No Stopping Me!, It's On!); nothing else is
// guaranteed outside the deck, so the player never sets it
export const goldGiven = (scen: string, group: number): boolean => (SCEN_GOLDS[scen || 'unity'] || []).includes(group);
// only the scenario guarantees anything
export const jobTargets = (s: SettingsView) =>
  s.targets.map((t) => ({ group: t.group, guaranteed: goldGiven(s.scen, t.group) ? 2 : 0 }));
// the solver reads an empty list as "no restriction", so with every card not owned it gets [0], an id no card has
function ownedCards(s: SettingsView) {
  const notOwned = gameData().cards.filter((c) => s.cardsOff[c.id] === false).length;
  if (!notOwned) return [];
  if (notOwned === gameData().cards.length) return [0];
  return gameData()
    .cards.filter((c) => s.cardsOff[c.id] !== false)
    .map((c) => c.id);
}
// every outfit of hers is marked not owned in `outfits` (the settings' marks)
const noOutfit = (u: Uma, outfits: SettingsView['outfits']): boolean =>
  (u.outfits || []).length > 0 && u.outfits.every((o) => outfits[o.id] === false);
// she is owned under settings `s`: not marked out, and an outfit of hers owned (or none in the data)
export const isOwned = (u: Uma, s: Pick<SettingsView, 'roster' | 'outfits'>): boolean =>
  s.roster[u.id] !== 'out' && !noOutfit(u, s.outfits);
// The parts both pages' jobs share (each page adds its own fields): the targets and roster (the solver's `SkillJob`): the
// targets, the lineage's rank band, the umas left out (`exclude`, which each page decides), the outfits not owned, the
// skills a career always gives
export const skillsPart = (s: SettingsView, exclude: number[]) => ({
  targets: jobTargets(s),
  rank_class: s.rank,
  exclude,
  exclude_outfits: Object.entries(s.outfits)
    .filter(([, owned]) => owned === false)
    .map(([id]) => +id),
  extra: EXTRA,
});
// and the deck space (`DeckJob`): the cards owned, a borrowed card allowed, the scenario, the R cards left out
export const deckPart = (s: SettingsView) => ({
  owned_cards: ownedCards(s),
  borrow: true,
  scenario: (s.scen || 'unity') as Scenario,
  ...rcardsOff(s),
});
// R cards (not pals) stay out of every search: settings.ts `rcards`, which no setting turns on
const rcardsOff = (s: SettingsView) =>
  s.rcards
    ? {}
    : {
        exclude_cards: gameData()
          .cards.filter((c) => c.rar === 1 && !c.pal)
          .map((c) => c.id),
      };
