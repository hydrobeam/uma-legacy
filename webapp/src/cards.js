// ---------- support cards: a card's name and what it gives, as the page shows them; what you own of them ----------
// The Support cards panel that draws the picker is ui/Cards.tsx.
import { computed } from '@preact/signals';
import { esc } from './dom.js';
import { gameData } from './data.js';
import { fields, settings, setOwned } from './settings.ts';
/** @import { Card } from './data.js' */
const RARITY = { 1: 'R', 2: 'SR', 3: 'SSR' };
const BORROWED = /\(borrowed\)$/; // a deck names its borrowed card with this suffix
// a deck's card indices with the borrowed card last: the sixth slot, as in the game
/** @type {(cards: string[]) => number[]} card names (a deck's `cards`) -> their indices */
export const borLast = (cards) =>
  cards.map((_, i) => i).sort((a, b) => Number(BORROWED.test(cards[a])) - Number(BORROWED.test(cards[b])));
// a card's name split as the game shows it: {nm: the character (with the rarity, "Daitaku Helios SSR", unless `rar` is
// false: the card list leaves it out, its frame and section heading show it), ti: the card's title ('' when none)}
/** @type {(c: Card, rar?: boolean) => {nm: string, ti: string}} */
export const cardTitle = (c, rar = true) => {
  const parts = /^\[(.*)\] (.*)$/.exec(c.name);
  return parts ? { nm: parts[2] + (rar ? ` ${RARITY[c.rar]}` : ''), ti: parts[1] } : { nm: c.name, ti: '' };
};
// how a card gives a target, as a short note and a one-line tooltip: from training with it, from an event, or from its
// n-th story event (they come in order, so a later one needs the earlier ones first); "50%": the event gives it on
// some of its outcomes (one of two: 50%; Anshinzawa SSR's finale: 75%, measured)
const ORDINAL = ['', '1st', '2nd', '3rd', '4th', '5th'];
const ordinal = (n) => ORDINAL[n] || `${n}th`;
const share = (p) => (p >= 1 ? '' : `${Math.round(p * 100)}%`);
/**
 * One way a card gives a target.
 * @typedef {object} CardGift
 * @property {string} skill the skill's name (its gold's when the event gives the gold)
 * @property {string} note how, in short ("hint", "2nd event · 50%")
 * @property {string} tip how, as a sentence for the tooltip
 * @property {boolean} gold the event gives the gold
 */
/** @returns {CardGift[]} the ways card `c` gives any of the skill groups in `groups` */
export function cardGives(c, groups) {
  const out = [];
  for (const group of c.whites)
    if (groups.has(group))
      out.push({
        skill: (gameData().skillNames.get(group) || { w: group }).w,
        note: 'hint',
        tip: 'Hinted when training with this card',
        gold: false,
      });
  for (const [group, level, link, p] of c.ev) {
    if (!groups.has(group)) continue;
    const names = gameData().skillNames.get(group) || { w: group, gd: group },
      part = share(p);
    const note = (link ? `${ordinal(link)} event` : 'event') + (part ? ` · ${part}` : '');
    const tip =
      (link ? `Its ${ordinal(link)} story event (they come in order)` : 'One of its events') +
      (part ? `, ${part} of the time` : '');
    out.push({ skill: level === 2 ? names.gd : names.w, note, tip, gold: level === 2 });
  }
  return out;
}
/**
 * The choice that ends card `c`'s chain at story-event link `link`, for a deck card the search ends there. IT takes it
 * only when a skill on it is prioritised. In the run data IT took a prioritised skill's choice in all 312 readings where
 * only that choice held one, on card events (most of them Sasami Anshinzawa SSR's finale), carried over here to
 * chain-ending choices; with the skill already at hint Lv 5 from another source, in 16 of 28 (docs/RESEARCH.md section 9).
 * @typedef {object} ChainEndAdvice
 * @property {string} take the ending choice's skills by name, '' when it names none the page knows
 * @property {{group: number, name: string}[]} others the skills on the event's other choices
 */
/** @returns {ChainEndAdvice} */
export function chainEndAt(c, link) {
  const choice = c?.end?.find((x) => x.link === link),
    named = (skills) => (skills || []).map(([group, level]) => ({ group, name: skillName(group, level) }));
  return {
    take: named(choice?.end)
      .map((x) => x.name)
      .join(' or '),
    others: named(choice?.go),
  };
}
const skillName = (group, level) => {
  const names = gameData().skillNames.get(group) || { w: String(group), gd: String(group) };
  return level === 2 ? names.gd : names.w;
};
// a chain-ending deck card's caption: the skill that makes IT take the ending choice
export const chainEndNote = (c, link) => {
  const { take } = chainEndAt(c, link);
  return take ? `prioritise ${take} (chain end)` : `chain end: link ${link}`;
};
// the gifts as the game's small labels (the card list's tiles and a deck card's panel)
/** @type {(gifts: CardGift[]) => string} */
export const giftsHTML = (gifts) =>
  gifts
    .map(
      (x) =>
        `<span class="gv${x.gold ? ' g' : ''}" title="${esc(x.tip)}">${esc(x.skill)} <small>${esc(x.note)}</small></span>`,
    )
    .join('');
// what is owned, counted over every card (the Support cards button says it): every card counts as owned until you say
// otherwise
export const cardsOwned = computed(() => {
  const off = fields.cardsOff.value;
  return { owned: gameData().cards.filter((c) => off[c.id] !== false).length, all: gameData().cards.length };
});
/** a click on a card's tile: owned <-> not owned; saved @param {number} id */
export function toggleCard(id) {
  const { [id]: notOwned, ...rest } = settings.value.cardsOff;
  setOwned({ cardsOff: notOwned === false ? rest : { ...rest, [id]: false } });
}
export const ownAllCards = () => setOwned({ cardsOff: {} });
export const ownNoCards = () =>
  setOwned({
    cardsOff: { ...settings.value.cardsOff, ...Object.fromEntries(gameData().cards.map((c) => [c.id, false])) },
  });
// Kept for callers that drew the card picker before it followed the settings (part 2 of step 11 drops the calls):
// nothing to do, the picker redraws itself. career/main.tsx calls initCards, popover.js paintCards.
export const initCards = () => undefined;
export const paintCards = () => undefined;
