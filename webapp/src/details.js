// ---------- the veteran card's parts the views share: the game's "Umamusume Details" screen ----------
// What Try a run's card (career/analysis/Details.tsx) and the node popups share: the spark names' fit (`fitSparks`), the
// game-style Skill Details popup (`openSkill`), a deck's card by its index (`deckCardAt`), and the game's Inspiration view
// of a veteran built with el() (`inspView`: Try a run's Inspiration tab until the veterans store's draft setter lands,
// step 10). The game's own art where the build shipped it (images.py: spark bars and bullets, skill tiles), the same
// shapes in CSS without it. Her sparks stay editable (vets.js sparksEl: click for stars, x removes, + adds).
import { el, lazyDialog } from './dom.js';
import { gameData } from './data.js';
import { umaAv, cardAv } from './images.js';
import { umaName, sparksEl } from './vets.js';
/** @import { LineageNode } from './vets.js' */

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
const pctS = (p) => (p >= 0.995 ? 'always' : p < 0.005 ? '<1%' : `${Math.round(p * 100)}%`);
// the game-style skill popup (one dialog, refilled); skFrom: the tile that opened it, focused again on close
/** @type {HTMLElement | null} */
let skFrom = null;
const skillDialog = lazyDialog('gmdlg', (dialog) => {
  dialog.id = 'sk-dlg';
  dialog.setAttribute('aria-labelledby', 'sk-title');
  dialog.addEventListener('close', () => skFrom?.focus());
});
// b: the target's buying report; got: 'gold' | 'white' | null (not acquired), or undefined for a career not run yet (the
// popup then gives her chance to land it); icon() makes her skill icon; from: the tile that opened it (focused again on
// close). legacies: her six legacies, each {chara, outfit, carries}, shown as faces lit where one carries it; cards: the
// deck cards that give it, each {card, how} ("hint", "gold hint", "event at link 3")
/**
 * @param {{b: any, name: string, got?: 'gold'|'white'|null, icon: () => Node|null, gold?: boolean,
 *   legacies?: {chara: number, outfit?: number, carries: boolean}[], cards?: {card: any, how: string}[]}} spec
 * @param {HTMLElement} from
 */
export function openSkill({ b, name, got, icon, gold = false, legacies = [], cards = [] }, from) {
  const skDlg = skillDialog.get();
  skFrom = from;
  skDlg.textContent = '';
  const h = el('h3', 'gmdhdr', 'Skill Details');
  h.id = 'sk-title';
  skDlg.appendChild(h);
  const body = el('div', 'gmdbody');
  const ahead = got === undefined; // (a career not run yet: chances only)
  const tile = el('div', `udsk${got === 'gold' || (ahead && gold) ? ' gold' : ''}${got || ahead ? '' : ' off'}`);
  const ic = icon();
  if (ic) tile.appendChild(ic);
  tile.appendChild(el('span', 'udskn', name));
  body.appendChild(tile);
  body.appendChild(
    el(
      'p',
      `gmdstat${got || ahead ? ' on' : ''}`,
      ahead
        ? `Lands as a spark: ${pctS(b.land_gold + b.land_white)}${gold ? ' (through its gold)' : ''}`
        : got
          ? `Acquired${got === 'gold' ? ' (gold)' : ''} in this result`
          : 'Not acquired in this result',
    ),
  );
  // where it comes from, each with its chance (a source under half a percent isn't listed)
  const sources = [];
  if (b.scenario) sources.push({ label: 'The scenario', chance: 1 });
  if (b.outfit === 2) sources.push({ label: 'Her own skill (gold)', chance: 1 });
  else if (b.outfit === 1) sources.push({ label: 'Her own skill', chance: 1 });
  // the deck's gold, else its white (`deck_white` is given no gold): the two add up to its chance of a hint, `deckHint`
  const deckWhite = (1 - (b.deck_gold || 0)) * (b.deck_white || 0);
  if (b.deck_gold > 0.005) sources.push({ label: 'Deck (its gold)', chance: b.deck_gold });
  if (deckWhite > 0.005) sources.push({ label: 'Deck', chance: deckWhite });
  if (b.own_event > 0.005) sources.push({ label: 'Her story events', chance: b.own_event });
  if (b.inherit > 0.005) sources.push({ label: 'Legacies (inheritance)', chance: b.inherit });
  body.appendChild(el('div', 'gmdsub', 'Where it comes from'));
  const ul = el('ul', 'gmdlist');
  for (const { label, chance } of sources) {
    const li = el('li');
    li.append(el('span', '', label), el('b', '', pctS(chance)));
    ul.appendChild(li);
  }
  body.appendChild(ul);
  // which legacies carry it, which cards give it
  if (legacies.length) {
    const row = el('div', 'gmdfaces');
    for (const l of legacies) {
      const f = umaAv(l.chara || 0, l.outfit, 32);
      f.classList.toggle('dim', !l.carries);
      f.title = `${umaName(l.chara)}${l.carries ? ': carries it' : ': no spark of it'}`;
      row.appendChild(f);
    }
    body.append(el('div', 'gmdsub', 'Legacies carrying it'), row);
  }
  if (cards.length) {
    const list = el('ul', 'gmdlist gmdcards');
    for (const { card, how } of cards) {
      const li = el('li');
      const who = el('span', 'gmdcard');
      who.append(cardAv(card, 28), el('span', '', card.name.replace(/^\[[^\]]*\] /, '')));
      li.append(who, el('b', '', how));
      list.appendChild(li);
    }
    body.append(el('div', 'gmdsub', 'Cards that give it'), list);
  }
  const sum = el('div', 'gmdlist two');
  for (const { label, chance } of [
    { label: 'Acquired', chance: b.gold + b.white },
    { label: 'Lands as a spark', chance: b.land_gold + b.land_white },
  ]) {
    const x = el('div');
    x.append(el('span', '', label), el('b', '', pctS(chance)));
    sum.appendChild(x);
  }
  body.appendChild(sum);
  skDlg.appendChild(body);
  const close = el('button', 'btn gmclose', 'Close');
  close.type = 'button';
  close.onclick = () => skDlg.close();
  const f = el('div', 'gmdfoot');
  f.appendChild(close);
  skDlg.appendChild(f);
  skDlg.showModal();
  close.focus();
}

// The game's Inspiration view of a veteran: "Sparks", her sparks as the game's spark bars (stars on their edge), then
// "Legacy Origin": her parents' (each a row with her portrait). Its component twin, ui/InspView.tsx, draws the node popups'.
/**
 * @typedef {object} InspViewSpec
 * @property {LineageNode & {parents?: LineageNode[]}} d  the veteran shown (her parents: none, or her two)
 * @property {string} key  her sparks' control names
 * @property {() => void} onEdit  after an edit of her sparks
 * @property {string} [meClass]  an extra class on her row
 * @property {((j: number) => string)|null} [parentLabel]  parent j's label (null: "Her parent")
 * @property {((j: number) => string)|null} [parentKey]  parent j's sparks' control names (null: her parents' read-only)
 * @property {() => void} [onEditParent]  after an edit of a parent's sparks
 * @property {boolean} [header]  the "Sparks" heading
 * @property {boolean} [readOnly]  nothing editable (a veteran listed on uma.moe: another player's)
 */
/** @param {InspViewSpec} spec */
export function inspView({
  d,
  key,
  onEdit,
  meClass = '',
  parentLabel = null,
  parentKey = null,
  onEditParent,
  header = true,
  readOnly = false,
}) {
  const p = el('div', 'udinsp');
  if (header) p.appendChild(el('div', 'udhdr', 'Sparks'));
  const lin = el('div', `udlin${d.parents?.length ? '' : ' solo'}`);
  const mine = el('div', `udrow me${meClass ? ` ${meClass}` : ''}`);
  const face = umaAv(d.chara || 0, d.outfit, 64);
  face.classList.add('udface');
  mine.appendChild(face);
  const sp = el('div', 'simskills');
  sp.setAttribute('role', 'group');
  sp.setAttribute('aria-label', 'Her sparks');
  sp.appendChild(sparksEl(d, key, onEdit, readOnly));
  mine.appendChild(sp);
  lin.appendChild(mine);
  if (d.parents?.length) {
    const lo = el('div', 'udorigin');
    lo.appendChild(el('span', '', 'Legacy Origin'));
    lin.appendChild(lo);
    d.parents.forEach((g, j) => {
      const row = el('div', 'udrow par');
      const f = umaAv(g.chara || 0, g.outfit, 64);
      f.classList.add('udface');
      f.setAttribute('role', 'img');
      f.setAttribute('aria-label', `${parentLabel ? `Parent ${parentLabel(j)}` : 'Her parent'}: ${umaName(g.chara)}`);
      row.appendChild(f);
      const s = el('div', 'udpsp');
      s.appendChild(
        parentKey && !readOnly ? sparksEl(g, parentKey(j), onEditParent) : sparksEl(g, `simp${j}`, undefined, true),
      );
      row.appendChild(s);
      lin.appendChild(row);
    });
  }
  p.appendChild(lin);
  return p;
}

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
