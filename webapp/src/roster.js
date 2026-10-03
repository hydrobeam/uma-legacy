// ---------- roster: one tile per outfit ----------
// Owned / not owned is per outfit (the solver's exclude_outfits); "must include" is per uma (the solver requires umas,
// not outfits), so it shows on all of her tiles.
import { $ } from './dom.js';
import { UMAS } from './data.js';
import { state } from './state.js';
import { umaAv } from './images.js';
import { save } from './search.js';
import { paintSort, sortMenu } from './sortmenu.js';
const ST_LABEL = { own: 'owned', out: 'not owned', req: 'must include' };
export function umaState(u) { const os = u.outfits || []; const st = state.roster[u.id] || 'own';
  return st === 'own' && os.length && os.every(o => state.outfits[o.id] === false) ? 'out' : st; }
export const noOutfit = (u) => (u.outfits || []).length > 0 && u.outfits.every(o => state.outfits[o.id] === false);
function tileState(u, o) {
  if (state.roster[u.id] === 'out' || (o && state.outfits[o.id] === false)) return 'out';
  return state.roster[u.id] === 'req' ? 'req' : 'own';
}
function rosterCount() {
  const out = UMAS.filter(u => state.roster[u.id] === 'out' || noOutfit(u)).length, req = Object.values(state.roster).filter(v => v === 'req').length;
  const nO = UMAS.reduce((a, u) => a + Math.max(1, (u.outfits || []).length), 0), offO = UMAS.reduce((a, u) => a + ((u.outfits && u.outfits.length) ? u.outfits : [null]).filter(o => tileState(u, o) === 'out').length, 0);
  $('r-count').textContent = `${nO - offO} of ${nO} outfits owned · ${UMAS.length - out} of ${UMAS.length} umas` + (req ? ` · ${req} must be in the loop` : '');
}
export function renderRoster() {
  const f = $('r-filter').value.trim().toLowerCase();
  const box = $('roster'); box.textContent = '';
  const tiles = [];
  for (const u of UMAS) for (const o of (u.outfits && u.outfits.length ? u.outfits : [null])) {
    if (f && !u.name.toLowerCase().includes(f) && !(o && o.label.toLowerCase().includes(f))) continue;
    tiles.push({ u, o, rel: o ? o.rel : 0, key: u.name + ' ' + (o ? o.label : '') });
  }
  const dir = state.rdir === 'desc' ? -1 : 1;
  tiles.sort((a, b) => dir * (state.rsort === 'name' ? a.key.localeCompare(b.key) : (a.rel - b.rel) || ((a.o ? a.o.id : 0) - (b.o ? b.o.id : 0))));
  for (const { u, o } of tiles) {
    const t = document.createElement('div'); t.className = 'ut'; t.dataset.uma = u.id;
    const b = document.createElement('button'); b.className = 'up'; b.type = 'button';
    b.appendChild(umaAv(u.id, o ? o.id : null, 96)); t.appendChild(b);
    const nm = document.createElement('div'); nm.className = 'un'; nm.textContent = u.name; t.appendChild(nm);
    if (o) { const ol = document.createElement('div'); ol.className = 'uol'; ol.textContent = o.label; t.appendChild(ol); }
    t._paint = () => { const st = tileState(u, o); t.dataset.s = st; b.setAttribute('aria-label', u.name + (o ? ' ' + o.label : '') + ': ' + ST_LABEL[st]); b.title = b.getAttribute('aria-label'); };
    t._paint();
    b.onclick = () => {
      const st = tileState(u, o);
      if (st === 'own') { if (o) state.outfits[o.id] = false; else state.roster[u.id] = 'out'; }
      else if (st === 'out') { if (o) delete state.outfits[o.id]; state.roster[u.id] = 'req'; }
      else delete state.roster[u.id];
      save(); box.querySelectorAll(`.ut[data-uma="${u.id}"]`).forEach(x => x._paint()); rosterCount(); };
    box.appendChild(t);
  }
  paintSort('rs', 'r');
  rosterCount();
}
export function initRoster() {
  sortMenu('rs', 'r', () => renderRoster());
  $('r-filter').oninput = renderRoster;
  $('r-all').onclick = () => { state.roster = Object.fromEntries(Object.entries(state.roster).filter(([, v]) => v === 'req')); state.outfits = {}; save(); renderRoster(); };
  $('r-none').onclick = () => { for (const u of UMAS) if (state.roster[u.id] !== 'req') state.roster[u.id] = 'out'; save(); renderRoster(); };
}
