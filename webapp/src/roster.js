// ---------- roster: one tile per outfit ----------
// Owned / not owned and "must include" are both per outfit: a must-include outfit makes its uma required and her
// unmarked outfits left out (solver: require + exclude_outfits).
import { $ } from './dom.js';
import { UMAS } from './data.js';
import { state, writeSaved } from './state.js';
import { srcFor } from './images.js';
import { ERA } from './settings.js';
import { rosterTiles, compareTiles, tileState as coreState, tileLabel, tileHTML, initials, EAGER } from './rostercore.js';
import { save } from './search.js';
import { paintSort, sortMenu } from './sortmenu.js';
export function umaState(u) { const os = u.outfits || []; const st = state.roster[u.id] || 'own';
  return st === 'own' && os.length && os.every(o => state.outfits[o.id] === false) ? 'out' : st; }
export const noOutfit = (u) => (u.outfits || []).length > 0 && u.outfits.every(o => state.outfits[o.id] === false);
const tileState = (u, o) => coreState(state.roster, state.outfits, u.id, o ? o.id : null);
// must include, per outfit: the uma is required and only her marked outfits are allowed (any of them). An uma
// without separate outfits is marked as a whole (roster 'req').
export const reqOutfits = (u) => (u.outfits || []).filter(o => state.outfits[o.id] === 'req');
export const isRequired = (u) => state.roster[u.id] === 'req' || reqOutfits(u).length > 0;
// settings saved when must-include was per uma: mark each of her owned outfits instead
export function migrateMustInclude() {
  let changed = false;
  for (const u of UMAS) if (state.roster[u.id] === 'req' && (u.outfits || []).length) {
    for (const o of u.outfits) if (state.outfits[o.id] !== false) state.outfits[o.id] = 'req';
    delete state.roster[u.id]; changed = true;
  }
  return changed;
}
function rosterCount() {
  const out = UMAS.filter(u => state.roster[u.id] === 'out' || noOutfit(u)).length, req = UMAS.filter(isRequired).length;
  const nO = UMAS.reduce((a, u) => a + Math.max(1, (u.outfits || []).length), 0), offO = UMAS.reduce((a, u) => a + ((u.outfits && u.outfits.length) ? u.outfits : [null]).filter(o => tileState(u, o) === 'out').length, 0);
  $('r-count').textContent = `${nO - offO} of ${nO} outfits owned · ${UMAS.length - out} of ${UMAS.length} umas` + (req ? ` · ${req} must be in the loop` : '');
}
// Tile elements by uma and outfit, kept across renders (a re-sort or filter moves them, images stay loaded). The first
// render takes over the pre-rendered Global roster (the build writes it into the page; rosterearly.js has already put
// it in the saved order and states), so the first screen's portraits are requested from the HTML itself.
const els = new Map(); let adopted = false;
const tkey = (uid, oid) => uid + ':' + (oid || '');
const portrait = (u, o) => (o && srcFor('outfit', o.id)) || srcFor('uma', u.id);
function wire(t, u, o, box) {
  const b = t.querySelector('.up');
  t._paint = () => { const st = tileState(u, o); t.dataset.s = st; const l = tileLabel(u.name, o && o.label, st); b.setAttribute('aria-label', l); b.title = l; };
  // a portrait that fails falls back to the uma's own icon, then to initials
  const im = t.querySelector('.av img');
  if (im) { const fail = () => { const alt = o && srcFor('uma', u.id);
      if (alt && !im.src.endsWith(alt)) im.src = alt; else { const av = im.parentElement; im.remove(); av.textContent = initials(u.name); } };
    im.addEventListener('error', fail); if (im.complete && im.naturalWidth === 0 && im.getAttribute('src')) fail(); }
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
    save(); box.querySelectorAll(`.ut[data-uma="${u.id}"]`).forEach(x => x._paint && x._paint()); rosterCount(); };
}
export function renderRoster() {
  const f = $('r-filter').value.trim().toLowerCase();
  const box = $('roster');
  if (!adopted) { adopted = true; if (ERA === 'global') for (const t of box.querySelectorAll('.ut')) els.set(tkey(t.dataset.uma, t.dataset.o), t); }
  const tiles = rosterTiles(UMAS).filter(({ u, o }) => !f || u.name.toLowerCase().includes(f) || (o && o.label.toLowerCase().includes(f)));
  tiles.sort(compareTiles(state.rsort, state.rdir));
  const out = tiles.map((t, i) => {
    const { u, o } = t, k = tkey(u.id, o && o.id); let el = els.get(k);
    if (!el) { const tpl = document.createElement('template'); tpl.innerHTML = tileHTML(t, { src: portrait(u, o), eager: i < EAGER, state: tileState(u, o) }); el = tpl.content.firstElementChild; els.set(k, el); }
    if (!el._paint) wire(el, u, o, box);
    el._paint(); return el; });
  if (out.length !== box.children.length || out.some((el, i) => box.children[i] !== el)) box.replaceChildren(...out);   // unchanged: left in place
  paintSort('rs', 'r');
  rosterCount();
}
export function initRoster() {
  sortMenu('rs', 'r', () => renderRoster());
  $('r-filter').oninput = renderRoster;
  if (migrateMustInclude()) writeSaved();
  $('r-all').onclick = () => { state.roster = Object.fromEntries(Object.entries(state.roster).filter(([, v]) => v === 'req'));
    state.outfits = Object.fromEntries(Object.entries(state.outfits).filter(([, v]) => v === 'req')); save(); renderRoster(); };
  $('r-none').onclick = () => { for (const u of UMAS) if (!isRequired(u)) state.roster[u.id] = 'out'; save(); renderRoster(); };
}
