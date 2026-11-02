// ---------- support cards: a card's name and what it gives, as the page shows them; what you own of them ----------
// The Support cards panel that draws the picker is ui/Cards.tsx.
import { computed } from '@preact/signals';
import { gameData } from './model/data.ts';
import { fields, settings, setOwned } from './settings.ts';
import { must } from './must.ts';
import type { Card } from './model/data.ts';
const RARITY = { 1: 'R', 2: 'SR', 3: 'SSR' };
// a card's name split as the game shows it: {nm: the character (with the rarity, "Daitaku Helios SSR", unless `rar` is
// false: the card list leaves it out, its frame and section heading show it), ti: the card's title ('' when none)}
export const cardTitle = (c: Card, rar = true): { nm: string; ti: string } => {
  const [, title, character] = /^\[(.*)\] (.*)$/.exec(c.name) ?? [];
  // (a name without "[title] ": no title)
  return title !== undefined && character !== undefined
    ? { nm: character + (rar ? ` ${RARITY[c.rar]}` : ''), ti: title }
    : { nm: c.name, ti: '' };
};
// how a card gives a target, as a short note and a one-line tooltip: from training with it, from an event, or from its
// n-th story event (they come in order, so a later one needs the earlier ones first); "50%": the event gives it on
// some of its outcomes (one of two: 50%; Anshinzawa SSR's finale: 75%, measured)
const ORDINAL = ['', '1st', '2nd', '3rd', '4th', '5th'];
const ordinal = (n: number) => ORDINAL[n] || `${n}th`;
const share = (p: number) => (p >= 1 ? '' : `${Math.round(p * 100)}%`);
/** One way a card gives a target. */
export interface CardGift {
  /** the skill's name (its gold's when the event gives the gold) */
  skill: string;
  /** how, in short ("hint", "2nd event · 50%") */
  note: string;
  /** how, as a sentence for the tooltip */
  tip: string;
  /** the event gives the gold */
  gold: boolean;
}
// the ways card `c` gives any of the skill groups in `groups`
export function cardGives(c: Card, groups: ReadonlySet<number>): CardGift[] {
  const out: CardGift[] = [];
  for (const group of c.whites)
    if (groups.has(group))
      out.push({
        skill: skillName(group, 1),
        note: 'hint',
        tip: 'Hinted when training with this card',
        gold: false,
      });
  for (const [group, level, link, p] of c.ev) {
    if (!groups.has(group)) continue;
    const part = share(p);
    const note = (link ? `${ordinal(link)} event` : 'event') + (part ? ` · ${part}` : '');
    const tip =
      (link ? `Its ${ordinal(link)} story event (they come in order)` : 'One of its events') +
      (part ? `, ${part} of the time` : '');
    out.push({ skill: skillName(group, level), note, tip, gold: level === 2 });
  }
  return out;
}
/**
 * The choice that ends card `c`'s chain at story-event link `link`, for a deck card the search ends there. IT takes it
 * only when a skill on it is prioritised. In the run data IT took a prioritised skill's choice in all 312 readings where
 * only that choice held one, on card events (most of them Sasami Anshinzawa SSR's finale), carried over here to
 * chain-ending choices; with the skill already at hint Lv 5 from another source, in 16 of 28 (docs/RESEARCH.md section 9).
 */
export interface ChainEndAdvice {
  /** the ending choice's skills by name, '' when it names none the page knows */
  take: string;
  /** the skills on the event's other choices */
  others: { group: number; name: string }[];
}
export function chainEndAt(c: Card | undefined, link: number): ChainEndAdvice {
  const choice = c?.end?.find((x) => x.link === link),
    named = (skills: [number, number][] | undefined) =>
      (skills || []).map(([group, level]) => ({ group, name: skillName(group, level) }));
  return {
    take: named(choice?.end)
      .map((x) => x.name)
      .join(' or '),
    others: named(choice?.go),
  };
}
// a skill's name at a level (2: its gold's, which export_page.py makes sure the catalog has); a group missing from the
// catalog is named by its number
const skillName = (group: number, level: number): string => {
  const names = gameData().skillNames.get(group) || { w: String(group), gd: String(group) };
  return level === 2 ? must(names.gd, 'a skill given at gold has a gold name (export_page.py skills)') : names.w;
};
// a chain-ending deck card's caption: the skill that makes IT take the ending choice
export const chainEndNote = (c: Card | undefined, link: number) => {
  const { take } = chainEndAt(c, link);
  return take ? `prioritise ${take} (chain end)` : `chain end: link ${link}`;
};
// what is owned, counted over every card (the Support cards button says it): every card counts as owned until you say
// otherwise
export const cardsOwned = computed(() => {
  const off = fields.cardsOff.value;
  return { owned: gameData().cards.filter((c) => off[c.id] !== false).length, all: gameData().cards.length };
});
// a click on a card's tile: owned <-> not owned; saved
export function toggleCard(id: number) {
  const { [id]: notOwned, ...rest } = settings.value.cardsOff;
  setOwned({ cardsOff: notOwned === false ? rest : { ...rest, [id]: false } });
}
export const ownAllCards = () => setOwned({ cardsOff: {} });
export const ownNoCards = () =>
  setOwned({
    cardsOff: { ...settings.value.cardsOff, ...Object.fromEntries(gameData().cards.map((c) => [c.id, false])) },
  });
