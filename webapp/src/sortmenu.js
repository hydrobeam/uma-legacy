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
export function sortMenu(id, key, render) {
  const btn = $(id), menu = $(id + '-menu'), open = (o) => { menu.hidden = !o; btn.setAttribute('aria-expanded', o); };
  btn.onclick = () => open(menu.hidden);
  menu.querySelectorAll('button').forEach(b => b.onclick = () => {
    if (b.dataset.k === state[key + 'sort']) state[key + 'dir'] = state[key + 'dir'] === 'desc' ? 'asc' : 'desc';
    else { state[key + 'sort'] = b.dataset.k; state[key + 'dir'] = b.dataset.k === 'rel' ? 'desc' : 'asc'; }
    writeSaved(); render(); });   // ordering only: saved, no new search
  document.addEventListener('click', (e) => { if (!e.target.closest('#' + id + '-dd')) open(false); });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && !menu.hidden) { open(false); btn.focus(); } });
}
