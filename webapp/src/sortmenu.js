// sort menus (umas, cards): pick Release or Name; picking the active one again reverses it (release starts newest
// first, name A to Z). The menu stays open until a click outside it or Escape. key: state prefix ('r' or 'c').
import { $ } from './dom.js';
import { state, writeSaved } from './state.js';
export function paintSort(id, key) {
  const k = state[key + 'sort'], d = state[key + 'dir'], arrow = d === 'desc' ? '↓' : '↑';
  const how = { 'rel-desc': 'newest first', 'rel-asc': 'oldest first', 'name-asc': 'A to Z', 'name-desc': 'Z to A' };
  const btn = $(id), what = btn.dataset.what || (btn.dataset.what = btn.getAttribute('aria-label').toLowerCase());   // 'sort umas'
  btn.textContent = (k === 'name' ? 'Name ' : 'Release ') + arrow; btn.title = 'Sorted ' + how[k + '-' + d];
  btn.setAttribute('aria-label', btn.textContent + ', ' + what);   // starts with the visible text (voice control)
  $(id + '-menu').querySelectorAll('button').forEach(b => { const on = b.dataset.k === k; b.setAttribute('aria-checked', on);
    b.textContent = (b.dataset.k === 'rel' ? 'Release' : 'Name') + (on ? ' ' + arrow : '');
    b.title = on ? 'Click to reverse' : (b.dataset.k === 'rel' ? 'Newest first' : 'A to Z'); });
}
// Keyboard (menu pattern, as the scenario picker): opening focuses the checked item, the arrows (and Home/End) move
// between items, Escape closes and returns to the button, focus leaving the menu closes it.
export function sortMenu(id, key, render) {
  const btn = $(id), menu = $(id + '-menu'), dd = $(id + '-dd'), items = () => [...menu.querySelectorAll('button')];
  const open = (o) => { menu.hidden = !o; btn.setAttribute('aria-expanded', o);
    if (o) (items().find(b => b.getAttribute('aria-checked') === 'true') || items()[0]).focus(); };
  items().forEach(b => b.tabIndex = -1);
  btn.onclick = () => open(menu.hidden);
  btn.addEventListener('keydown', (e) => { if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && menu.hidden) { e.preventDefault(); open(true); } });
  items().forEach(b => b.onclick = () => {
    if (b.dataset.k === state[key + 'sort']) state[key + 'dir'] = state[key + 'dir'] === 'desc' ? 'asc' : 'desc';
    else { state[key + 'sort'] = b.dataset.k; state[key + 'dir'] = b.dataset.k === 'rel' ? 'desc' : 'asc'; }
    writeSaved(); render(); });   // ordering only: saved, no new search
  menu.addEventListener('keydown', (e) => {
    const bs = items(), i = bs.indexOf(document.activeElement);
    const to = { ArrowDown: (i + 1) % bs.length, ArrowUp: (i - 1 + bs.length) % bs.length, Home: 0, End: bs.length - 1 }[e.key];
    if (to === undefined) return; e.preventDefault(); bs[to].focus(); });
  document.addEventListener('click', (e) => { if (!e.target.closest('#' + id + '-dd')) open(false); });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && !menu.hidden) { open(false); btn.focus(); } });
  dd.addEventListener('focusout', (e) => { if (e.relatedTarget && !dd.contains(e.relatedTarget)) open(false); });
}
