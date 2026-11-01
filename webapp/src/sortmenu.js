// sort menus (umas, cards): pick Release or Name; picking the active one again reverses it (release starts newest
// first, name A to Z). The menu stays open until a click outside it or Escape. key: the settings' prefix ('r' or 'c').
// (`wireMenu` is the button and menu alone; the uma.moe popup's orders use it too.)
import { $ } from './dom.js';
import { settings, setSort } from './settings.ts';
import { must } from './must.ts';
const HOW = { 'rel-desc': 'newest first', 'rel-asc': 'oldest first', 'name-asc': 'A to Z', 'name-desc': 'Z to A' };
// a sort menu's choice and its direction ('r': rsort, rdir; 'c': csort, cdir)
/** @type {(key: 'r' | 'c') => [string, string]} */
const sortOf = (key) =>
  key === 'r' ? [settings.value.rsort, settings.value.rdir] : [settings.value.csort, settings.value.cdir];
/** @param {string} id the sort button's id @param {'r' | 'c'} key the settings' sort keys' prefix ('r': rsort, rdir) */
export function paintSort(id, key) {
  const [sortBy, direction] = sortOf(key),
    arrow = direction === 'desc' ? '↓' : '↑';
  const btn = $(id);
  if (!btn.dataset.what)
    btn.dataset.what = must(btn.getAttribute('aria-label'), 'a sort button has its label').toLowerCase(); // 'sort umas', kept from the first paint
  const what = btn.dataset.what;
  btn.textContent = `${sortBy === 'name' ? 'Name ' : 'Release '}${arrow}`;
  btn.title = `Sorted ${HOW[`${sortBy}-${direction}`]}`;
  btn.setAttribute('aria-label', `${btn.textContent}, ${what}`); // starts with the visible text (voice control)
  $(`${id}-menu`)
    .querySelectorAll('button')
    .forEach((b) => {
      const on = b.dataset.k === sortBy;
      b.setAttribute('aria-checked', String(on));
      b.textContent = (b.dataset.k === 'rel' ? 'Release' : 'Name') + (on ? ` ${arrow}` : '');
      b.title = on ? 'Click to reverse' : b.dataset.k === 'rel' ? 'Newest first' : 'A to Z';
    });
}
/** @param {string} id the sort button's id @param {'r' | 'c'} key the settings' sort keys' prefix @param {() => void} render */
export function sortMenu(id, key, render) {
  wireMenu(id, (k) => {
    const [sortBy, direction] = sortOf(key);
    if (k === sortBy) setSort(key, sortBy, direction === 'desc' ? 'asc' : 'desc');
    else setSort(key, k, k === 'rel' ? 'desc' : 'asc');
    render(); // ordering only: saved, no new search
  });
}
// A sort button and its menu (`id`, `${id}-menu`, `${id}-dd`), wired: `pick` gets the chosen item's data-k (the items
// are the menu's buttons when this runs). Keyboard (menu pattern, as the scenario picker): opening focuses the checked
// item, the arrows (and Home/End) move between items, Escape closes and returns to the button, focus leaving the menu
// closes it.
/** @param {string} id @param {(k: string) => void} pick */
export function wireMenu(id, pick) {
  const btn = $(id),
    menu = $(`${id}-menu`),
    dropdown = $(`${id}-dd`),
    items = () => [...menu.querySelectorAll('button')];
  const open = (isOpen) => {
    menu.hidden = !isOpen;
    btn.setAttribute('aria-expanded', isOpen);
    if (isOpen) (items().find((b) => b.getAttribute('aria-checked') === 'true') || items()[0]).focus();
  };
  items().forEach((b) => {
    b.tabIndex = -1;
  });
  btn.onclick = () => open(menu.hidden);
  btn.addEventListener('keydown', (e) => {
    if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && menu.hidden) {
      e.preventDefault();
      open(true);
    }
  });
  items().forEach((b) => {
    b.onclick = () => pick(must(b.dataset.k, 'a menu item names its key'));
  });
  menu.addEventListener('keydown', (e) => {
    const all = items(),
      i = all.indexOf(/** @type {HTMLButtonElement} */ (document.activeElement));
    const to = {
      ArrowDown: (i + 1) % all.length,
      ArrowUp: (i - 1 + all.length) % all.length,
      Home: 0,
      End: all.length - 1,
    }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    all[to].focus();
  });
  document.addEventListener('click', (/** @type {TargetedEvent} */ e) => {
    if (!e.target.closest(`#${id}-dd`)) open(false);
  });
  document.addEventListener(
    'keydown',
    (e) => {
      if (e.key === 'Escape' && !menu.hidden) {
        e.preventDefault();
        e.stopPropagation();
        open(false);
        btn.focus();
      }
    },
    true,
  ); // capture: before a dialog it sits in takes Escape
  dropdown.addEventListener('focusout', (/** @type {FocusEvent & {relatedTarget: Element | null}} */ e) => {
    if (e.relatedTarget && !dropdown.contains(e.relatedTarget)) open(false);
  });
}
