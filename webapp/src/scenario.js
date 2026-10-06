// ---------- scenario picker: a button showing the chosen scenario, opening a grid of the scenarios ----------
// Same pattern as the sort menus (sortmenu.js): role=menu, tiles are menuitemradio buttons. Opening focuses the chosen
// tile; arrows move between tiles (two columns), Escape or a click outside the button and the grid closes it, and so
// does focus leaving the picker. Picking a scenario also sets its guaranteed golds and its default lineage rank.
import { $ } from './dom.js';
import { SCENARIOS, SCEN_GOLDS, SK, RANKS } from './data.js';
import { ERA } from './settings.js';
import { state, writeSaved } from './state.js';
import { scenLogo, skillIcon, srcFor } from './images.js';
import { save } from './search.js';
import { renderTargets } from './targets.js';
import { renderCards } from './cards.js';
// the scenarios this era's data offers (Grand Masters only with the JP Grand Masters-era data)
const LIST = SCENARIOS.filter(s => !s.era || s.era === ERA);
export const scenarioOf = (v) => LIST.find(s => s.v === v) || LIST[0];
// the lineage rank control: a two-option toggle (B to S+ / SS and up) with the game's rank badges; data-value carries the
// chosen star mix, each option's tooltip its rank score range and star mix
export function paintRank() {
  const r = $('rank'); r.dataset.value = state.rank;
  r.querySelectorAll('button').forEach(b => { const on = b.dataset.v === state.rank; b.setAttribute('aria-checked', on); b.tabIndex = on ? 0 : -1; });
}
// switch to scenario v: its guaranteed golds and its default lineage rank (both can be changed by hand afterwards)
function setScenario(v) {
  const sc = scenarioOf(v); state.scen = sc.v;
  state.rank = sc.ss ? RANKS.ss : RANKS.below; paintRank();
}
// Switching to the JP Grand Masters-era data opens on its newest scenario, Grand Masters (with its golds and rank
// defaults); switching back to Global turns it into Unity Cup on load (initScenario).
export function eraScenario(era) {
  const sc = SCENARIOS.find(s => s.v === 'grandmasters');
  if (era !== 'gm' || !sc || state.scen === sc.v) return;
  state.scen = sc.v; state.rank = sc.ss ? RANKS.ss : RANKS.below;
}
const goldNames = (sc) => (SCEN_GOLDS[sc.v] || []).map(g => [g, (SK.get(g) || {}).gd]).filter(([, n]) => n);
// the tile's (and button's) accessible name: the logo is the only visible name
const label = (sc) => { const g = goldNames(sc).map(([, n]) => n);
  return sc.name + ', ' + sc.rate + (g.length ? ', ' + g.join(' and ') + ' guaranteed' : ''); };
// the button (the chosen scenario's logo) and the tiles' checked state follow state.scen (also after an import)
export function paintScen() {
  const sc = scenarioOf(state.scen), btn = $('x-scen');
  btn.dataset.value = sc.v; btn.setAttribute('aria-label', 'Scenario: ' + sc.name); btn.title = label(sc);
  btn.querySelector('.slogo').replaceWith(scenLogo(sc));
  $('scen-menu').querySelectorAll('.stile').forEach(t => t.setAttribute('aria-checked', String(t.dataset.v === sc.v)));
}
// the ? by Lineage rank: a small popup; Escape or a click elsewhere closes it
function initRankNote() { const q = $('rank-q'), n = $('rank-note'), set = (open) => { n.hidden = !open; q.setAttribute('aria-expanded', String(open)); };
  q.onclick = (e) => { e.stopPropagation(); set(n.hidden); };
  document.addEventListener('click', (e) => { if (!n.hidden && !n.contains(e.target)) set(false); });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && !n.hidden) { set(false); q.focus(); } }); }
export function initScenario() {
  initRankNote();
  const btn = $('x-scen'), menu = $('scen-menu'), dd = $('scen-dd');
  // a scenario this era does not offer (Grand Masters, saved while the GM-era data was active; the choice is shared by
  // both eras) becomes the first one offered, Unity Cup, as if picked
  if (scenarioOf(state.scen).v !== state.scen) { setScenario(scenarioOf(state.scen).v); writeSaved(); }
  const rk = $('rank'), pickRank = (v) => { if (v === state.rank) return; state.rank = v; paintRank(); save(); };
  rk.querySelectorAll('.rk').forEach(el => { const s = srcFor('ui', 'rank_' + el.dataset.r); if (!s) return;   // badge images (text stays without them)
    const im = new Image(); im.alt = ''; im.src = s; im.onerror = () => im.remove(); el.textContent = ''; el.classList.add('img'); el.appendChild(im); });
  rk.querySelectorAll('button').forEach(b => b.onclick = () => pickRank(b.dataset.v));
  rk.addEventListener('keydown', (e) => {   // radio group: arrows move and select
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) return; e.preventDefault();
    const bs = [...rk.querySelectorAll('button')], i = bs.findIndex(b => b.dataset.v === state.rank), n = bs[(i + (e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? bs.length - 1 : 1)) % bs.length];
    pickRank(n.dataset.v); n.focus(); });
  paintRank();
  menu.classList.toggle('five', LIST.length === 5);
  for (const sc of LIST) {
    const t = document.createElement('button'); t.type = 'button'; t.className = 'stile'; t.setAttribute('role', 'menuitemradio'); t.dataset.v = sc.v; t.tabIndex = -1;
    t.setAttribute('aria-label', label(sc)); t.title = label(sc);   // the same hover text as the button shows for the chosen one
    const sub = document.createElement('span'); sub.className = 'ssub'; sub.textContent = sc.mult;
    if (sc.est) {   // "(estimate)", shortened to "(est.)" when the tiles are narrow (phone) so the line stays one line
      const e = document.createElement('span'); e.className = 'sest'; e.innerHTML = '(<span class="sest-l">estimate</span><abbr class="sest-s" title="estimate">est.</abbr>)'; sub.appendChild(e); }
    // the golds the scenario hands out in every run: their icons on the same line, named by tooltip (the names as small
    // text only when the icons are missing), so every tile stays logo + one line
    const golds = goldNames(sc);
    if (golds.length) { const g = document.createElement('span'); g.className = 'sgolds'; g.setAttribute('role', 'group');
      g.setAttribute('aria-label', 'Guaranteed in every run'); g.title = 'Guaranteed in every run';
      for (const [id, n] of golds) { const ic = skillIcon(id, true);
        if (ic) { ic.alt = n; ic.title = n; ic.loading = 'eager'; g.appendChild(ic); }
        else { const x = document.createElement('span'); x.className = 'sgold'; x.textContent = n; x.title = n; g.appendChild(x); } }
      sub.appendChild(g); }
    t.append(scenLogo(sc), sub);
    t.onclick = () => pick(sc.v);
    menu.appendChild(t);
  }
  const tiles = () => [...menu.querySelectorAll('.stile')];
  const open = (o) => {
    menu.hidden = !o; btn.setAttribute('aria-expanded', String(o));
    if (o) (tiles().find(t => t.dataset.v === state.scen) || tiles()[0]).focus();
  };
  const close = (refocus) => { if (menu.hidden) return; open(false); if (refocus) btn.focus(); };
  function pick(v) {
    if (v !== state.scen) { setScenario(v); save(); renderTargets(); renderCards(); paintScen(); }
    close(true);
  }
  btn.onclick = () => (menu.hidden ? open(true) : close(false));
  btn.addEventListener('keydown', (e) => { if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && menu.hidden) { e.preventDefault(); open(true); } });
  menu.addEventListener('keydown', (e) => {
    const ts = tiles(), i = ts.indexOf(document.activeElement), cols = 2;
    const to = { ArrowRight: i + 1, ArrowLeft: i - 1, ArrowDown: i + cols, ArrowUp: i - cols, Home: 0, End: ts.length - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault(); ts[Math.max(0, Math.min(ts.length - 1, to))].focus();
  });
  // only the button and the open grid count as inside: a click anywhere else (the label, the row's empty space, the rank
  // control, the rest of the page) closes the picker
  document.addEventListener('click', (e) => { if (!e.target.closest('#x-scen, #scen-menu')) close(false); });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && !menu.hidden) close(true); });
  dd.addEventListener('focusout', (e) => { if (e.relatedTarget && !dd.contains(e.relatedTarget)) close(false); });
  paintScen();
}
