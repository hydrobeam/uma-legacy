// ---------- what every dropdown shares (P8): the sort menus (SortMenu.tsx), the scenario picker ----------
// Opening focuses the checked item, the arrows (and Home/End) move between items, Escape closes and returns to the
// button, a click outside closes.
import type { RefObject } from 'preact';
import { useLayoutEffect } from 'preact/hooks';
import { must } from '../must.ts';

// While `open`: a click outside every element of `inside` closes it, and so does Escape, which also returns the focus
// to `button`. Escape is taken in the capture phase (before a dialog the menu sits in takes it) and goes no further.
// (The listeners exist only while open. The opening click is on the button, which counts as inside, so attaching them
// in a layout effect, inside that click's dispatch, cannot close the menu at once.)
export function useDropdown(
  open: boolean,
  close: () => void,
  button: RefObject<HTMLElement | null>,
  inside: RefObject<HTMLElement | null>[],
) {
  useLayoutEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      const target = e.target as Node;
      if (![button, ...inside].some((ref) => ref.current?.contains(target))) close();
    };
    const onEscape = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      e.preventDefault();
      e.stopPropagation();
      close();
      button.current?.focus();
    };
    document.addEventListener('click', onClick);
    document.addEventListener('keydown', onEscape, true);
    return () => {
      document.removeEventListener('click', onClick);
      document.removeEventListener('keydown', onEscape, true);
    };
  }, [open, close, button, inside]);
}
// on open: the focus to the checked item, else the first
export function useFocusChecked(open: boolean, menu: RefObject<HTMLElement | null>) {
  useLayoutEffect(() => {
    if (!open) return;
    const items = must(menu.current, 'the menu is mounted');
    (items.querySelector<HTMLElement>('[aria-checked="true"]') || (items.firstElementChild as HTMLElement)).focus();
  }, [open, menu]);
}
// The arrows over a menu's buttons: one column, wrapping (sort menus), or a grid of `columns`, clamped at the ends
// (the scenario tiles: Left/Right step one, Up/Down a row); Home and End go to the first and the last
export function menuKeys(e: KeyboardEvent & { currentTarget: HTMLElement }, columns = 1) {
  const all = [...e.currentTarget.querySelectorAll<HTMLElement>('button')],
    at = all.indexOf(document.activeElement as HTMLElement);
  const grid = columns > 1;
  const to = {
    ArrowDown: at + columns,
    ArrowUp: at - columns,
    Home: 0,
    End: all.length - 1,
    ...(grid ? { ArrowRight: at + 1, ArrowLeft: at - 1 } : {}),
  }[e.key];
  if (to === undefined) return;
  e.preventDefault();
  all[grid ? Math.max(0, Math.min(all.length - 1, to)) : (to + all.length) % all.length].focus();
}
// ArrowDown or ArrowUp on a closed menu's button opens it
export const opensOnArrow = (open: boolean, setOpen: (open: boolean) => void) => (e: KeyboardEvent) => {
  if (!open && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
    e.preventDefault();
    setOpen(true);
  }
};
