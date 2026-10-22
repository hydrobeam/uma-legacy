// ---------- roster: one tile per outfit ----------
// Owned / not owned, per outfit (solver: exclude + exclude_outfits).
import { $ } from './dom.js';
import { UMAS } from './data.js';
import { state } from './state.js';
import { umaAv } from './images.js';
import { save } from './save.js';
import { paintSort, sortMenu } from './sortmenu.js';
const STATE_LABEL = { own: 'owned', out: 'not owned' };
// her ownership: 'out' when marked not owned or every outfit of hers is, else her roster mark ('own' by default)
export function umaState(u) {
  const outfits = u.outfits || [], mark = state.roster[u.id] || 'own';
  return mark === 'own' && outfits.length && outfits.every(o => state.outfits[o.id] === false) ? 'out' : mark;
}
export const noOutfit = (u) => (u.outfits || []).length > 0 && u.outfits.every(o => state.outfits[o.id] === false);
// one tile's ownership: the uma's mark, else the outfit's (o null: an uma without outfits in the data)
const tileState = (u, o) => state.roster[u.id] === 'out' || (o && state.outfits[o.id] === false) ? 'out' : 'own';
// her tiles' outfits: one tile per outfit, or a single tile (null) when the data lists none
const tileOutfits = (u) => u.outfits?.length ? u.outfits : [null];
// The outfit a result names for uma `id`: one, never "any outfit". `offered`: the solver's outfits for her (each covers a
// target the same way; empty when none of hers covers one). Among the offered, the first; with none offered, among your
// owned outfits her original (<id>01), else the first.
// Returns { list: the chosen first, then the other offered ones (equally good), any: no outfit of hers covers a target }
export function pickOutfit(id, offered = []) {
  const u = UMAS.find(x => x.id === id), mine = (u?.outfits || []).filter(o => state.outfits[o.id] !== false);
  if (offered.length) return { list: offered, any: false };
  const chosen = mine.find(o => o.id === id * 100 + 1) || mine[0];
  return { list: chosen ? [{ id: chosen.id, label: chosen.label }] : [], any: true };
}
// what her popup says about the outfit chosen: that any of hers would do, or the equally good others
export const outfitNote = (pick) => pick.any ? 'Any outfit works: none of hers covers a target.'
  : pick.list.length > 1 ? `Also works: ${pick.list.slice(1).map(o => o.label).join(', ')}.` : '';
function rosterCount() {
  const umasOut = UMAS.filter(u => state.roster[u.id] === 'out' || noOutfit(u)).length;
  const outfits = UMAS.reduce((sum, u) => sum + Math.max(1, (u.outfits || []).length), 0);
  const outfitsOut = UMAS.reduce((sum, u) => sum + tileOutfits(u).filter(o => tileState(u, o) === 'out').length, 0);
  $('r-count').textContent = `${UMAS.length - umasOut} of ${UMAS.length} umas · ${outfits - outfitsOut} of ${outfits} outfits`;
}
// after an ownership change: every tile's look (or her tiles only) and the count; the list itself stays (its order
// doesn't depend on ownership)
export function paintRoster(umaId = null) {
  for (const tile of $('roster').children) if (umaId == null || +tile.dataset.uma === umaId) tile._paint();
  rosterCount();
}
// the filter hides tiles (name or outfit containing the text) instead of rebuilding the list on every keystroke
function filterRoster() {
  const filter = $('r-filter').value.trim().toLowerCase();
  for (const tile of $('roster').children) tile.hidden = !!filter && !tile._searchName.includes(filter) && !tile._outfitLabel.includes(filter);
}
// a click on a tile: owned <-> not owned, for its outfit (or the uma, on a tile without one)
function toggleTile(u, o) {
  if (tileState(u, o) === 'own') { if (o) state.outfits[o.id] = false; else state.roster[u.id] = 'out'; }
  else if (!o) delete state.roster[u.id];
  else {   // a whole-uma 'not owned' becomes per outfit, so only this tile changes
    if (state.roster[u.id] === 'out') { delete state.roster[u.id]; for (const x of u.outfits) if (state.outfits[x.id] === undefined) state.outfits[x.id] = false; }
    delete state.outfits[o.id];
  }
}
function rosterTile(box, u, o, eager) {
  const tile = document.createElement('div'); tile.className = 'ut'; tile.dataset.uma = u.id;
  tile._searchName = u.name.toLowerCase(); tile._outfitLabel = o ? o.label.toLowerCase() : '';
  const portrait = document.createElement('button'); portrait.className = 'up'; portrait.type = 'button';
  portrait.appendChild(umaAv(u.id, o ? o.id : null, 96, eager)); tile.appendChild(portrait);
  const name = document.createElement('div'); name.className = 'un'; name.textContent = u.name; tile.appendChild(name);
  if (o) { const label = document.createElement('div'); label.className = 'uol'; label.textContent = o.label; label.title = o.label; tile.appendChild(label); }   // two lines at most (CSS); the title has it whole
  // hover: who she is (the tile's look shows owned / not owned; the screen reader hears it)
  tile._paint = () => {
    const tileMark = tileState(u, o), who = u.name + (o ? ` ${o.label}` : '');
    tile.dataset.s = tileMark; portrait.setAttribute('aria-label', `${who}: ${STATE_LABEL[tileMark]}`); portrait.title = who;
  };
  tile._paint();
  portrait.onclick = () => {
    toggleTile(u, o);
    save(); box.querySelectorAll(`.ut[data-uma="${u.id}"]`).forEach(x => { x._paint(); }); rosterCount();
  };
  return tile;
}
export function renderRoster() {
  const box = $('roster'); box.textContent = '';
  const tiles = [];
  for (const u of UMAS) for (const o of tileOutfits(u)) tiles.push({ u, o, rel: o ? o.rel : 0, key: `${u.name} ${o ? o.label : ''}` });
  const dir = state.rdir === 'desc' ? -1 : 1;
  tiles.sort((a, b) => dir * (state.rsort === 'name' ? a.key.localeCompare(b.key) : (a.rel - b.rel) || ((a.o ? a.o.id : 0) - (b.o ? b.o.id : 0))));
  const EAGER = 8;   // the first two rows are on screen at load (phone and desktop): their portraits load at once
  tiles.forEach(({ u, o }, i) => { box.appendChild(rosterTile(box, u, o, i < EAGER)); });
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
