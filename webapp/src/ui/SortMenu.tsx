// ---------- a menu button over a list of choices (P8: one keyboard and close behaviour for every such menu) ----------
// `SortMenu`: a sort button and its menu (`id`, `${id}-menu`, `${id}-dd`): pick an order; picking the checked one again
// reverses it (the caller says what a pick does). The menu stays open until a click outside it, Escape, or the focus
// leaving it. `useDropdown` and `menuKeys` are what every dropdown shares (the scenario picker too): opening focuses the
// checked item, the arrows (and Home/End) move between items, Escape closes and returns to the button.
// `SettingsSort` is the umas' and the cards' menu, Release or Name, kept in the settings.
import type { RefObject } from 'preact';
import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { settings, setSort } from '../settings.ts';
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

export interface SortItem {
  k: string;
  label: string;
  /** the item's tooltip while it is not the checked one */
  hint: string;
}
interface SortMenuProps {
  id: string;
  /** the button's class */
  cls: string;
  /** what the menu orders, after the button's text in its name ('sort umas') */
  what: string;
  items: SortItem[];
  /** the checked item's k */
  value: string;
  /** the direction, shown after the checked item's label */
  arrow: '↑' | '↓';
  /** the button's tooltip */
  title: string;
  onPick: (k: string) => void;
}
export function SortMenu({ id, cls, what, items, value, arrow, title, onPick }: SortMenuProps) {
  const [open, setOpen] = useState(false),
    button = useRef<HTMLButtonElement>(null),
    area = useRef<HTMLDivElement>(null),
    menu = useRef<HTMLDivElement>(null);
  const close = useRef(() => setOpen(false)).current,
    inside = useRef([area]).current;
  useDropdown(open, close, button, inside);
  useFocusChecked(open, menu);
  // (a value it doesn't list, as from a hand-edited storage: the first item)
  const checked = items.find((item) => item.k === value) ?? items[0];
  const text = `${checked.label} ${arrow}`;
  return (
    <div
      class="sortdd"
      id={`${id}-dd`}
      ref={area}
      onFocusOut={(e) => {
        const to = e.relatedTarget as Node | null;
        if (to && !e.currentTarget.contains(to)) setOpen(false);
      }}
    >
      <button
        class={cls}
        id={id}
        ref={button}
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={`${text}, ${what}`} // starts with the visible text (voice control)
        title={title}
        onClick={() => setOpen(!open)}
        onKeyDown={opensOnArrow(open, setOpen)}
      >
        {text}
      </button>
      <div class="sortmenu" id={`${id}-menu`} role="menu" ref={menu} hidden={!open} onKeyDown={(e) => menuKeys(e)}>
        {items.map(({ k, label, hint }) => {
          const on = k === value;
          return (
            <button
              key={k}
              type="button"
              role="menuitemradio"
              data-k={k}
              tabIndex={-1}
              aria-checked={on}
              title={on ? 'Click to reverse' : hint}
              onClick={() => onPick(k)}
            >
              {on ? `${label} ${arrow}` : label}
            </button>
          );
        })}
      </div>
    </div>
  );
}

// ---------- the umas' and the cards' order: Release (newest first) or Name (A to Z), kept in the settings
const RELEASE_NAME: SortItem[] = [
  { k: 'rel', label: 'Release', hint: 'Newest first' },
  { k: 'name', label: 'Name', hint: 'A to Z' },
];
const HOW: Record<string, string> = {
  'rel-desc': 'newest first',
  'rel-asc': 'oldest first',
  'name-asc': 'A to Z',
  'name-desc': 'Z to A',
};
/** `prefix`: the settings' sort keys ('r': rsort, rdir; 'c': csort, cdir); a pick is saved, and starts no search */
export function SettingsSort({ prefix, id, what }: { prefix: 'r' | 'c'; id: string; what: string }) {
  const s = settings.value,
    [savedSort, savedDir] = prefix === 'r' ? [s.rsort, s.rdir] : [s.csort, s.cdir];
  // read as the lists order by them: by name only for 'name', descending only for 'desc' (a hand-edited stored value is
  // neither)
  const sortBy = savedSort === 'name' ? 'name' : 'rel',
    direction = savedDir === 'desc' ? 'desc' : 'asc';
  const pick = (k: string) => {
    if (k === sortBy) setSort({ prefix, sort: sortBy, dir: direction === 'desc' ? 'asc' : 'desc' });
    else setSort({ prefix, sort: k, dir: k === 'rel' ? 'desc' : 'asc' });
  };
  return (
    <SortMenu
      id={id}
      cls="btn"
      what={what}
      items={RELEASE_NAME}
      value={sortBy}
      arrow={direction === 'desc' ? '↓' : '↑'}
      title={`Sorted ${HOW[`${sortBy}-${direction}`]}`}
      onPick={pick}
    />
  );
}
