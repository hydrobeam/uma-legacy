// ---------- support cards: the picker (owned or not, by rarity) and deck options ----------
import { $, $input, esc } from './dom.js';
import { CARDS, SK, SCENARIOS } from './data.js';
import { state, goldGiven } from './state.js';
import { cardAv } from './images.js';
import { save } from './save.js';
import { paintSort, sortMenu } from './sortmenu.js';
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
        skill: (SK.get(group) || { w: group }).w,
        note: 'hint',
        tip: 'Hinted when training with this card',
        gold: false,
      });
  for (const [group, level, link, p] of c.ev) {
    if (!groups.has(group)) continue;
    const names = SK.get(group) || { w: group, gd: group },
      part = share(p);
    const note = (link ? `${ordinal(link)} event` : 'event') + (part ? ` · ${part}` : '');
    const tip =
      (link ? `Its ${ordinal(link)} story event (they come in order)` : 'One of its events') +
      (part ? `, ${part} of the time` : '');
    out.push({ skill: level === 2 ? names.gd : names.w, note, tip, gold: level === 2 });
  }
  return out;
}
// the gifts as the game's small labels (the card list's tiles and a deck card's panel)
/** @type {(gifts: CardGift[]) => string} */
export const giftsHTML = (gifts) =>
  gifts
    .map(
      (x) =>
        `<span class="gv${x.gold ? ' g' : ''}" title="${esc(x.tip)}">${esc(x.skill)} <small>${esc(x.note)}</small></span>`,
    )
    .join('');
const scenarioName = (v) => SCENARIOS.find((s) => s.v === v)?.name;
// one card's tile in the picker: a toggle, pressed = owned; its name is its visible text (name, title, what it gives),
// so voice control matches it
function cardTile(c, gifts) {
  const title = cardTitle(c, false),
    tile = document.createElement('button');
  tile.className = 'ct';
  tile.type = 'button';
  tile._searchName = c.name.toLowerCase();
  tile.innerHTML = `<span class="cn">${esc(title.nm)}</span>${title.ti ? `<span class="cti">${esc(title.ti)}</span>` : ''}<span class="cg">${gifts.length ? giftsHTML(gifts) : ''}</span>`;
  tile.prepend(cardAv(c, 112));
  // a card the chosen scenario bans (Team Sirius in Unity Cup and Grand Concert): the search leaves it out
  const scenario = state.scen || 'unity';
  if ((c.ban || []).includes(scenario)) {
    tile.classList.add('ban');
    const note = document.createElement('span');
    note.className = 'cban';
    note.textContent = `Not allowed in ${scenarioName(scenario)}`;
    tile.appendChild(note);
  }
  const paint = () => {
    const owned = state.cardsOff[c.id] !== false;
    tile.setAttribute('aria-pressed', owned ? 'true' : 'false');
  };
  paint();
  tile._paint = paint;
  tile.onclick = () => {
    if (state.cardsOff[c.id] === false) delete state.cardsOff[c.id];
    else state.cardsOff[c.id] = false;
    save();
    paint();
    cardCount();
  };
  return tile;
}
export function renderCards() {
  const groups = new Set(state.targets.filter((t) => !goldGiven(t.group)).map((t) => t.group)); // what the deck still has to give
  const box = $('cards');
  box.textContent = '';
  /** @type {[string, (c: Card) => boolean][]} label and test of each rarity's section */
  const sections = [
    ['SSR', (c) => c.rar === 3],
    ['SR', (c) => c.rar === 2],
    ['R', (c) => c.rar === 1 && (c.pal || state.rcards)],
  ]; // R pals stay usable without R cards
  const dir = state.cdir === 'desc' ? -1 : 1,
    sortName = (c) => {
      const t = cardTitle(c);
      return `${t.nm} ${t.ti}`;
    };
  const order = (a, b) =>
    dir * (state.csort === 'name' ? sortName(a.c).localeCompare(sortName(b.c)) : a.c.rel - b.c.rel || a.c.id - b.c.id);
  for (const [label, inSection] of sections) {
    const list = CARDS.filter(inSection)
      .map((c) => ({ c, gifts: cardGives(c, groups) }))
      .filter((x) => x.c.pal || x.gifts.length)
      .sort(order);
    if (!list.length) continue;
    const section = document.createElement('div');
    section.className = 'cgroup';
    section.innerHTML = `<h3>${label}</h3><div class="ctiles"></div>`;
    const tiles = section.querySelector('.ctiles');
    for (const { c, gifts } of list) tiles.appendChild(cardTile(c, gifts));
    box.appendChild(section);
  }
  paintSort('cs', 'c');
  filterCards();
}
// after an ownership change: every card's pressed state and the count (the list doesn't depend on ownership)
export function paintCards() {
  for (const tile of $('cards').querySelectorAll('.ct')) tile._paint();
  cardCount();
}
// the filter hides cards (and a rarity section left empty) instead of rebuilding the list on every keystroke
function filterCards() {
  const filter = $input('c-filter').value.trim().toLowerCase();
  for (const section of $('cards').querySelectorAll('.cgroup')) {
    let any = false;
    for (const tile of section.querySelectorAll('.ct')) {
      tile.hidden = !!filter && !tile._searchName.includes(filter);
      any ||= !tile.hidden;
    }
    section.hidden = !any;
  }
  cardCount();
}
function cardCount() {
  // the cards listed (those that give a target, and pals, matching the filter) and how many of them are owned (beside All / None owned)
  const listed = [...$('cards').querySelectorAll('.ct:not([hidden])')],
    owned = listed.filter((c) => c.getAttribute('aria-pressed') === 'true').length;
  $('c-count').textContent = `${owned} of ${listed.length} owned`;
}
export function initCards() {
  sortMenu('cs', 'c', () => renderCards());
  $input('c-filter').oninput = filterCards;
  $('c-all').onclick = () => {
    state.cardsOff = {};
    save();
    paintCards();
  };
  $('c-none').onclick = () => {
    for (const c of CARDS) state.cardsOff[c.id] = false;
    save();
    paintCards();
  };
  if (!$('c-top')) return; // the career page has neither: one career, one deck
  // Results: 1 or 3, the game's pill switch (a radio group: the arrows move and select)
  const top = $('c-top'),
    pickTop = (v) => {
      if (v === state.decksTop) return;
      state.decksTop = v;
      paintTop();
      save();
    };
  top.querySelectorAll('button').forEach((b) => {
    b.onclick = () => pickTop(b.dataset.v);
  });
  top.addEventListener('keydown', (e) => {
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) return;
    e.preventDefault();
    const other = [...top.querySelectorAll('button')].find((b) => b.dataset.v !== state.decksTop); // two options: any arrow goes to the other
    pickTop(other.dataset.v);
    other.focus();
  });
  paintTop();
  $input('c-peruma').checked = state.perUma;
  $input('c-peruma').onchange = (/** @type {TargetedEvent<HTMLInputElement>} */ e) => {
    state.perUma = e.target.checked;
    save();
  };
}
export function paintTop() {
  const top = document.getElementById('c-top');
  if (!top) return;
  top.querySelectorAll('button').forEach((b) => {
    const on = b.dataset.v === state.decksTop;
    b.setAttribute('aria-checked', String(on));
    b.setAttribute('aria-pressed', String(on));
    b.tabIndex = on ? 0 : -1;
  });
}
