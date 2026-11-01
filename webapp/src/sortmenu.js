// The el() menu button, for the two menus that are not components yet: the veterans' order (career/view/Pool.tsx) and
// uma.moe's (umamoesetup.js). Part 2 of step 11 moves them to ui/SortMenu.tsx, which is this behaviour as a
// component (P8); then this file goes.
import { $ } from './dom.js';
import { must } from './must.ts';
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
