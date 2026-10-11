// ---------- roster: one tile per outfit ----------
// Owned / not owned, per outfit (solver: exclude + exclude_outfits).
import { $ } from './dom.js';
import { UMAS } from './data.js';
import { state } from './state.js';
import { umaAv } from './images.js';
import { save } from './save.js';
import { paintSort, sortMenu } from './sortmenu.js';
const ST_LABEL = { own: 'owned', out: 'not owned' };
export function umaState(u) { const os = u.outfits || []; const st = state.roster[u.id] || 'own';
  return st === 'own' && os.length && os.every(o => state.outfits[o.id] === false) ? 'out' : st; }
export const noOutfit = (u) => (u.outfits || []).length > 0 && u.outfits.every(o => state.outfits[o.id] === false);
const tileState = (u, o) => state.roster[u.id] === 'out' || (o && state.outfits[o.id] === false) ? 'out' : 'own';
// The outfit a result names for uma `id`: one, never "any outfit". `offered`: the solver's outfits for her (each covers a
// target the same way; empty when none of hers covers one). Among the offered, the first; with none offered, among your
// owned outfits her original (<id>01), else the first.
// Returns { list: the chosen first, then the other offered ones (equally good), any: no outfit of hers covers a target }
export function pickOutfit(id, offered = []) {
  const u = UMAS.find(x => x.id === id), mine = (u?.outfits || []).filter(o => state.outfits[o.id] !== false);
  if (offered.length) return { list: offered, any: false };
  const p = mine.find(o => o.id === id * 100 + 1) || mine[0];
  return { list: p ? [{ id: p.id, label: p.label }] : [], any: true };
}
// what her popup says about the outfit chosen: that any of hers would do, or the equally good others
export const outfitNote = (f) => f.any ? 'Any outfit works: none of hers covers a target.'
  : f.list.length > 1 ? 'Also works: ' + f.list.slice(1).map(o => o.label).join(', ') + '.' : '';
function rosterCount() {
  const out = UMAS.filter(u => state.roster[u.id] === 'out' || noOutfit(u)).length;
  const nO = UMAS.reduce((a, u) => a + Math.max(1, (u.outfits || []).length), 0), offO = UMAS.reduce((a, u) => a + ((u.outfits && u.outfits.length) ? u.outfits : [null]).filter(o => tileState(u, o) === 'out').length, 0);
  $('r-count').textContent = `${UMAS.length - out} of ${UMAS.length} umas · ${nO - offO} of ${nO} outfits`;
}
// after an ownership change: every tile's look (or her tiles only) and the count; the list itself stays (its order
// doesn't depend on ownership)
export function paintRoster(umaId = null) {
  for (const t of $('roster').children) if (umaId == null || +t.dataset.uma === umaId) t._paint();
  rosterCount();
}
// the filter hides tiles (name or outfit containing the text) instead of rebuilding the list on every keystroke
function filterRoster() {
  const f = $('r-filter').value.trim().toLowerCase();
  for (const t of $('roster').children) t.hidden = !!f && !t._nm.includes(f) && !t._ol.includes(f);
}
export function renderRoster() {
  const box = $('roster'); box.textContent = '';
  const tiles = [];
  for (const u of UMAS) for (const o of (u.outfits && u.outfits.length ? u.outfits : [null])) tiles.push({ u, o, rel: o ? o.rel : 0, key: u.name + ' ' + (o ? o.label : '') });
  const dir = state.rdir === 'desc' ? -1 : 1;
  tiles.sort((a, b) => dir * (state.rsort === 'name' ? a.key.localeCompare(b.key) : (a.rel - b.rel) || ((a.o ? a.o.id : 0) - (b.o ? b.o.id : 0))));
  const EAGER = 8;   // the first two rows are on screen at load (phone and desktop): their portraits load at once
  tiles.forEach(({ u, o }, i) => {
    const t = document.createElement('div'); t.className = 'ut'; t.dataset.uma = u.id;
    t._nm = u.name.toLowerCase(); t._ol = o ? o.label.toLowerCase() : '';
    const b = document.createElement('button'); b.className = 'up'; b.type = 'button';
    b.appendChild(umaAv(u.id, o ? o.id : null, 96, i < EAGER)); t.appendChild(b);
    const nm = document.createElement('div'); nm.className = 'un'; nm.textContent = u.name; t.appendChild(nm);
    if (o) { const ol = document.createElement('div'); ol.className = 'uol'; ol.textContent = o.label; ol.title = o.label; t.appendChild(ol); }   // two lines at most (CSS); the title has it whole
    t._paint = () => { const st = tileState(u, o); t.dataset.s = st; b.setAttribute('aria-label', u.name + (o ? ' ' + o.label : '') + ': ' + ST_LABEL[st]); b.title = u.name + (o ? ' ' + o.label : ''); };   // hover: who she is (the tile's look shows owned / not owned; the screen reader hears it)
    t._paint();
    b.onclick = () => {
      const st = tileState(u, o);
      if (st === 'own') { if (o) state.outfits[o.id] = false; else state.roster[u.id] = 'out'; }
      else if (!o) delete state.roster[u.id];
      else {   // a whole-uma 'not owned' becomes per outfit, so only this tile changes
        if (state.roster[u.id] === 'out') { delete state.roster[u.id]; for (const x of u.outfits) if (state.outfits[x.id] === undefined) state.outfits[x.id] = false; }
        delete state.outfits[o.id];
      }
      save(); box.querySelectorAll(`.ut[data-uma="${u.id}"]`).forEach(x => x._paint()); rosterCount(); };
    box.appendChild(t);
  });
  filterRoster();
  paintSort('rs', 'r');
  rosterCount();
}
export function initRoster() {
  sortMenu('rs', 'r', () => renderRoster());
  $('r-filter').oninput = filterRoster;
  $('r-all').onclick = () => { state.roster = {}; state.outfits = {}; save(); paintRoster(); };
  $('r-none').onclick = () => { for (const u of UMAS) state.roster[u.id] = 'out'; save(); paintRoster(); };
}
