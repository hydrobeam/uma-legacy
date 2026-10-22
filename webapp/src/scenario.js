// ---------- scenario picker: a button showing the chosen scenario, opening a grid of the scenarios ----------
// Same pattern as the sort menus (sortmenu.js): role=menu, tiles are menuitemradio buttons. Opening focuses the chosen
// tile; arrows move between tiles (two columns), Escape or a click outside the button and the grid closes it, and so
// does focus leaving the picker. Picking a scenario also sets its guaranteed golds and its default lineage rank.
import { $ } from './dom.js';
import { SCENARIOS, SCEN_GOLDS, SK, RANKS } from './data.js';
import { ERA } from './settings.js';
import { state, writeSaved } from './state.js';
import { scenLogo, skillIcon, srcFor } from './images.js';
import { save } from './save.js';
import { renderTargets } from './targets.js';
// the scenarios this era's data offers (Grand Masters only with the JP Grand Masters-era data)
const LIST = SCENARIOS.filter(s => !s.era || s.era === ERA);
export const scenarioOf = (v) => LIST.find(s => s.v === v) || LIST[0];
// the lineage rank control: a two-option toggle (B to S+ / SS and up) with the game's rank badges; data-value carries the
// chosen star mix, each option's tooltip its rank score range and star mix
export function paintRank() {
  const rank = $('rank'); if (!rank) return;   // the career page has no rank control (it uses each legacy's real stars)
  rank.dataset.value = state.rank;
  rank.querySelectorAll('button').forEach(b => { const on = b.dataset.v === state.rank; b.setAttribute('aria-checked', String(on)); b.tabIndex = on ? 0 : -1; });
}
// the lineage rank a scenario starts from (its `ss`)
const rankOf = (sc) => sc.ss ? RANKS.ss : RANKS.below;
// switch to scenario v: its guaranteed golds and its default lineage rank (both can be changed by hand afterwards)
function setScenario(v) {
  const sc = scenarioOf(v); state.scen = sc.v;
  state.rank = rankOf(sc); paintRank();
}
// Switching to the JP Grand Masters-era data opens on its newest scenario, Grand Masters (with its golds and rank
// defaults); switching back to Global turns it into Unity Cup on load (initScenario).
export function eraScenario(era) {
  const sc = SCENARIOS.find(s => s.v === 'grandmasters');
  if (era !== 'gm' || !sc || state.scen === sc.v) return;
  state.scen = sc.v; state.rank = rankOf(sc);
}
// the golds a scenario hands out: [skill group, gold's name] (those with a name)
const goldNames = (sc) => (SCEN_GOLDS[sc.v] || []).map(group => [group, SK.get(group)?.gd]).filter(([, name]) => name);
// the tile's (and button's) accessible name: the logo is the only visible name
const label = (sc) => {
  const golds = goldNames(sc).map(([, name]) => name);
  return `${sc.name}, ${sc.rate}${golds.length ? `, ${golds.join(' and ')} guaranteed` : ''}`;
};
// the scenario select art (styled on the career page) as a CSS url(); absolute: a url() in a custom property resolves
// against the stylesheet that uses it, not the page. null without the art.
const artUrl = (sc) => { const art = srcFor('scenart', sc.id); return art ? `url(${new URL(art, document.baseURI).href})` : null; };
// the button (the chosen scenario's logo) and the tiles' checked state follow state.scen (also after an import)
export function paintScen() {
  const sc = scenarioOf(state.scen), btn = $('x-scen');
  btn.dataset.value = sc.v; btn.setAttribute('aria-label', `Scenario: ${sc.name}`); btn.title = label(sc);
  btn.querySelector('.slogo').replaceWith(scenLogo(sc));
  btn.style.setProperty('--scen-art', artUrl(sc) || 'none');
  $('scen-menu').querySelectorAll('.stile').forEach(t => { t.setAttribute('aria-checked', String(t.dataset.v === sc.v)); });
}
// the ? by Lineage rank: a small popup; Escape or a click elsewhere closes it
function initRankNote() {
  const question = $('rank-q'), note = $('rank-note'), setOpen = (open) => { note.hidden = !open; question.setAttribute('aria-expanded', String(open)); };
  question.onclick = (e) => { e.stopPropagation(); setOpen(note.hidden); };
  document.addEventListener('click', (/** @type {TargetedEvent} */ e) => { if (!note.hidden && !note.contains(e.target)) setOpen(false); });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && !note.hidden) { setOpen(false); question.focus(); } });
}
// the lineage rank toggle: its badge images (the text stays without them), a click or the arrows pick
function initRank() {
  const rank = $('rank') || document.createElement('div'), pickRank = (v) => { if (v === state.rank) return; state.rank = v; paintRank(); save(); };
  rank.querySelectorAll('.rk').forEach(badge => {
    const src = srcFor('ui', `rank_${badge.dataset.r}`); if (!src) return;
    const image = new Image(); image.alt = ''; image.src = src; image.onerror = () => image.remove();
    badge.textContent = ''; badge.classList.add('img'); badge.appendChild(image);
  });
  rank.querySelectorAll('button').forEach(b => { b.onclick = () => pickRank(b.dataset.v); });
  rank.addEventListener('keydown', (e) => {   // radio group: arrows move and select
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) return;
    e.preventDefault();
    const buttons = [...rank.querySelectorAll('button')], i = buttons.findIndex(b => b.dataset.v === state.rank);
    const back = e.key === 'ArrowLeft' || e.key === 'ArrowUp';
    const next = buttons[(i + (back ? buttons.length - 1 : 1)) % buttons.length];
    pickRank(next.dataset.v); next.focus();
  });
  paintRank();
}
// one scenario's tile in the grid: its logo, hint rate and guaranteed golds
function scenarioTile(sc) {
  const tile = document.createElement('button'); tile.type = 'button'; tile.className = 'stile'; tile.setAttribute('role', 'menuitemradio'); tile.dataset.v = sc.v; tile.tabIndex = -1;
  const art = artUrl(sc); if (art) tile.style.setProperty('--scen-art', art);
  tile.setAttribute('aria-label', label(sc)); tile.title = label(sc);   // the same hover text as the button shows for the chosen one
  const sub = document.createElement('span'); sub.className = 'ssub'; sub.textContent = sc.mult;
  if (sc.est) {   // "(estimate)", shortened to "(est.)" when the tiles are narrow (phone) so the line stays one line
    const estimate = document.createElement('span'); estimate.className = 'sest';
    estimate.innerHTML = '(<span class="sest-l">estimate</span><abbr class="sest-s" title="estimate">est.</abbr>)'; sub.appendChild(estimate);
  }
  // the golds the scenario hands out in every run: their icons on the same line, named by tooltip (the names as small
  // text only when the icons are missing), so every tile stays logo + one line
  const golds = goldNames(sc);
  if (golds.length) {
    const box = document.createElement('span'); box.className = 'sgolds'; box.setAttribute('role', 'group');
    box.setAttribute('aria-label', 'Guaranteed in every run'); box.title = 'Guaranteed in every run';
    for (const [group, name] of golds) {
      const icon = skillIcon(group, true);
      if (icon) { icon.alt = name; icon.title = name; icon.loading = 'eager'; box.appendChild(icon); }
      else { const text = document.createElement('span'); text.className = 'sgold'; text.textContent = name; text.title = name; box.appendChild(text); }
    }
    sub.appendChild(box);
  }
  tile.append(scenLogo(sc), sub);
  return tile;
}
export function initScenario() {
  if ($('rank-q')) initRankNote();
  const btn = $('x-scen'), menu = $('scen-menu'), dropdown = $('scen-dd');
  // a scenario this era does not offer (Grand Masters, saved while the GM-era data was active; the choice is shared by
  // both eras) becomes the first one offered, Unity Cup, as if picked
  if (scenarioOf(state.scen).v !== state.scen) { setScenario(scenarioOf(state.scen).v); writeSaved(); }
  initRank();
  menu.classList.toggle('five', LIST.length === 5);
  for (const sc of LIST) { const tile = scenarioTile(sc); tile.onclick = () => pick(sc.v); menu.appendChild(tile); }
  const tiles = () => [...menu.querySelectorAll('.stile')];
  const open = (isOpen) => {
    menu.hidden = !isOpen; btn.setAttribute('aria-expanded', String(isOpen));
    if (isOpen) (tiles().find(t => t.dataset.v === state.scen) || tiles()[0]).focus();
  };
  const close = (refocus) => { if (menu.hidden) return; open(false); if (refocus) btn.focus(); };
  function pick(v) {
    if (v !== state.scen) { setScenario(v); save(); renderTargets(); paintScen(); }   // renderTargets redraws the card picker too (bans, what each card gives)
    close(true);
  }
  btn.onclick = () => (menu.hidden ? open(true) : close(false));
  btn.addEventListener('keydown', (e) => { if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && menu.hidden) { e.preventDefault(); open(true); } });
  const COLUMNS = 2;   // the grid's columns (the arrows Up and Down move a row)
  menu.addEventListener('keydown', (e) => {
    const all = tiles(), i = all.indexOf(/** @type {HTMLElement} */ (document.activeElement));
    const to = { ArrowRight: i + 1, ArrowLeft: i - 1, ArrowDown: i + COLUMNS, ArrowUp: i - COLUMNS, Home: 0, End: all.length - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault(); all[Math.max(0, Math.min(all.length - 1, to))].focus();
  });
  // only the button and the open grid count as inside: a click anywhere else (the label, the row's empty space, the rank
  // control, the rest of the page) closes the picker
  document.addEventListener('click', (/** @type {TargetedEvent} */ e) => { if (!e.target.closest('#x-scen, #scen-menu')) close(false); });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && !menu.hidden) close(true); });
  dropdown.addEventListener('focusout', (/** @type {FocusEvent & {relatedTarget: Element | null}} */ e) => { if (e.relatedTarget && !dropdown.contains(e.relatedTarget)) close(false); });
  paintScen();
}
