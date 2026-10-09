// ---------- roster: one tile per outfit ----------
// Owned / not owned and "must include" are both per outfit: a must-include outfit makes its uma required and her
// unmarked outfits left out (solver: require + exclude_outfits).
import { $ } from './dom.js';
import { UMAS } from './data.js';
import { state } from './state.js';
import { umaAv } from './images.js';
import { save } from './save.js';
import { paintSort, sortMenu } from './sortmenu.js';
const ST_LABEL = { own: 'owned', out: 'not owned', req: 'must include' };
export function umaState(u) { const os = u.outfits || []; const st = state.roster[u.id] || 'own';
  return st === 'own' && os.length && os.every(o => state.outfits[o.id] === false) ? 'out' : st; }
export const noOutfit = (u) => (u.outfits || []).length > 0 && u.outfits.every(o => state.outfits[o.id] === false);
function tileState(u, o) {
  if (state.roster[u.id] === 'out' || (o && state.outfits[o.id] === false)) return 'out';
  return (o ? state.outfits[o.id] === 'req' : state.roster[u.id] === 'req') ? 'req' : 'own';
}
// must include, per outfit: the uma is required and only her marked outfits are allowed (any of them). An uma
// without separate outfits is marked as a whole (roster 'req').
export const reqOutfits = (u) => (u.outfits || []).filter(o => state.outfits[o.id] === 'req');
export const isRequired = (u) => state.roster[u.id] === 'req' || reqOutfits(u).length > 0;
function rosterCount() {
  const out = UMAS.filter(u => state.roster[u.id] === 'out' || noOutfit(u)).length, req = UMAS.filter(isRequired).length;
  const nO = UMAS.reduce((a, u) => a + Math.max(1, (u.outfits || []).length), 0), offO = UMAS.reduce((a, u) => a + ((u.outfits && u.outfits.length) ? u.outfits : [null]).filter(o => tileState(u, o) === 'out').length, 0);
  $('r-count').textContent = `${UMAS.length - out} of ${UMAS.length} umas · ${nO - offO} of ${nO} outfits` + (req ? ` · ${req} must include` : '');
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
    t._paint = () => { const st = tileState(u, o); t.dataset.s = st; b.setAttribute('aria-label', u.name + (o ? ' ' + o.label : '') + ': ' + ST_LABEL[st]); b.title = u.name + (o ? ' ' + o.label : ''); };   // hover: who she is (the tile's look shows owned / not owned / must include; the screen reader hears it)
    t._paint();
    b.onclick = () => {
      const st = tileState(u, o);
      if (st === 'own') { if (o) state.outfits[o.id] = false; else state.roster[u.id] = 'out'; }
      else if (st === 'out') {
        if (!o) state.roster[u.id] = 'req';
        else {   // a whole-uma 'not owned' becomes per outfit, so only this tile changes
          if (state.roster[u.id] === 'out') { delete state.roster[u.id]; for (const x of u.outfits) if (state.outfits[x.id] === undefined) state.outfits[x.id] = false; }
          state.outfits[o.id] = 'req';
        }
      }
      else if (o) delete state.outfits[o.id]; else delete state.roster[u.id];
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
  $('r-all').onclick = () => { state.roster = Object.fromEntries(Object.entries(state.roster).filter(([, v]) => v === 'req'));
    state.outfits = Object.fromEntries(Object.entries(state.outfits).filter(([, v]) => v === 'req')); save(); paintRoster(); };
  $('r-none').onclick = () => { for (const u of UMAS) if (!isRequired(u)) state.roster[u.id] = 'out'; save(); paintRoster(); };
}
