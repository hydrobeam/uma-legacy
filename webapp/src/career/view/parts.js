// Small parts the career page's views share: a button, a section, a node's tile, a deck, a target's name, numbers.
import { el } from '../../dom.js';
import { careerTables, gameData } from '../../data.js';
import { settings } from '../../settings.ts';
import { goldGiven } from '../../job.js';
import { cardAv } from '../../images.js';
import { cardTitle, borLast, chainEndNote } from '../../cards.js';
import { tableCost, tableChance } from '../../agenda.js';
import { deckCardAt } from '../../details.js';
import { must } from '../../must.ts';

export const tName = (g) => {
  const s = gameData().skillNames.get(g) || { w: `Skill ${g}`, gd: null };
  return goldGiven(settings.value.scen, g) && s.gd ? s.gd : s.w;
};
export const btn = (label, cls, fn) => {
  const b = el('button', `btn${cls ? ` ${cls}` : ''}`, label);
  b.type = 'button';
  b.onclick = fn;
  return b;
};

export const fmtN = (x) => Math.round(x).toLocaleString();
// ---------- the career: odds, deck, targets, schedule, after this career, other picks ----------
// a section of the results, headed by its title
export const sectionEl = (cls, title) => {
  const s = el('section', `rbsec ${cls}`);
  s.appendChild(el('h3', '', title));
  return s;
};
// the deck as the loop planner shows it: card art and name; a click opens the card's panel (owned or not, GameTora)
export function deckEl(r, title = '') {
  const d = r.deck;
  const dk = el('div', 'deck');
  dk.innerHTML = `${title ? '<div class="dtitle"><b></b></div>' : ''}<div class="dcards"></div>`;
  if (title) must(dk.querySelector('.dtitle b'), 'the deck is made with its title').textContent = title;
  const box = must(dk.querySelector('.dcards'), 'the deck is made with its cards box');
  borLast(d.cards).forEach((i) => {
    const { base, borrowed: bor, card: c } = deckCardAt(d, i);
    const endAt = (d.end_chain_at || [])[i] || 0,
      pick = (d.choices || [])[i] || (c?.pri ? c.pri.join(', ') : '');
    const t = c ? cardTitle(c) : { nm: base, ti: '' },
      tag = bor ? 'borrowed' : c?.pal ? 'pal' : '';
    const full =
      t.nm +
      (t.ti ? ` · ${t.ti}` : '') +
      (tag ? ` (${tag})` : '') +
      (pick ? ` · prioritise ${pick}` : '') +
      (endAt ? ` · ${chainEndNote(c, endAt)}` : '');
    const b = el('button', `dc${bor ? ' bor' : c?.pal ? ' pal' : ''}`);
    b.type = 'button';
    if (c) b.dataset.cid = String(c.id);
    b.dataset.name = t.nm;
    if (endAt) b.dataset.end = String(endAt);
    b.title = full;
    b.setAttribute('aria-label', full);
    b.setAttribute('aria-haspopup', 'dialog');
    b.setAttribute('aria-expanded', 'false');
    b.append(cardAv(c || null, 96, false), el('span', 'dn', full));
    if (pick) b.appendChild(el('i', '', `prioritise ${pick}`));
    if (endAt) b.appendChild(el('i', '', chainEndNote(c, endAt)));
    box.appendChild(b);
  });
  return dk;
}
// her race schedule: the schedule window (the game's calendar), from a button under the deck (the races she runs: the
// G1s she could enter but doesn't aren't shown)
// her schedule's rows, each race's figure the community table's on her own grades (agenda.js `tableChance`; the
// planner's, which counts the pink sparks she may get, on hover)
export const tableRows = (r) =>
  (r.schedule || []).map((row) => {
    const tables = careerTables(),
      c = row.cells[0],
      ix = tables.races.indexOf(row.name),
      apt = r.aptitudes?.base;
    if (!c || ix < 0 || !apt || !tables.race_cat) return row;
    const ground = must(tables.race_ground, 'build.py writes race_ground with race_cat')[ix];
    const { cost, grades } = tableCost(apt, tables.race_cat[ix], ground);
    return { ...row, cells: [{ ...c, tbl: tableChance(cost, c.row), tapt: cost, grades }] };
  });
