// ---------- a menu button over a list of choices (P8: one keyboard and close behaviour for every such menu) ----------
// `SortMenu`: a sort button and its menu (`id`, `${id}-menu`, `${id}-dd`): pick an order; picking the checked one again
// reverses it (the caller says what a pick does). The menu stays open until a click outside it, Escape, or the focus
// leaving it; its keys and closing are dropdown.ts's, which the scenario picker shares.
// `SettingsSort` is the umas' and the cards' menu, Release or Name, kept in the settings.
import { useRef, useState } from 'preact/hooks';
import { must } from '../must.ts';
import { fields, setSort } from '../settings.ts';
import { Button } from './Button.tsx';
import type { ButtonSize } from './Button.tsx';
import { menuKeys, opensOnArrow, useDropdown, useFocusChecked } from './dropdown.ts';
import { popover } from './Popover.tsx';

// the menu under its button's right end, and its items (the checked one bold, in the accent)
const menuLook = popover({
  frame: 'plain',
  class: 'absolute top-[calc(100%+4px)] right-0 z-20 grid min-w-[140px] p-1',
});
const item =
  'hover:bg-accent-soft focus-visible:bg-accent-soft not-aria-checked:text-ink aria-checked:text-accent text-ui rounded-md px-2.5 py-1.75 text-left aria-checked:font-bold max-md:min-h-11';

export interface SortItem<K extends string = string> {
  k: K;
  label: string;
  /** the item's tooltip while it is not the checked one */
  hint: string;
}
interface SortMenuProps<K extends string> {
  id: string;
  /** the button's type size */
  size?: ButtonSize;
  /** what the menu orders, after the button's text in its name ('sort umas') */
  what: string;
  items: readonly SortItem<K>[];
  /** the checked item's k */
  value: K;
  /** the direction, shown after the checked item's label */
  arrow: '↑' | '↓';
  /** the button's tooltip */
  title: string;
  onPick: (k: K) => void;
}
export function SortMenu<K extends string>({ id, size, what, items, value, arrow, title, onPick }: SortMenuProps<K>) {
  const [open, setOpen] = useState(false),
    button = useRef<HTMLButtonElement>(null),
    area = useRef<HTMLDivElement>(null),
    menu = useRef<HTMLDivElement>(null);
  const close = useRef(() => setOpen(false)).current,
    inside = useRef([area]).current;
  useDropdown(open, close, button, inside);
  useFocusChecked(open, menu);
  // (a value it doesn't list, as from a hand-edited storage: the first item)
  const checked = must(items.find((item) => item.k === value) ?? items[0], 'a sort menu has items');
  const text = `${checked.label} ${arrow}`;
  return (
    <div
      class="relative flex-none"
      id={`${id}-dd`}
      ref={area}
      onFocusOut={(e) => {
        const to = e.relatedTarget as Node | null;
        if (to && !e.currentTarget.contains(to)) setOpen(false);
      }}
    >
      <Button
        size={size}
        id={id}
        ref={button}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={`${text}, ${what}`} // starts with the visible text (voice control)
        title={title}
        onClick={() => setOpen(!open)}
        onKeyDown={opensOnArrow(open, setOpen)}
      >
        {text}
      </Button>
      <div class={menuLook} id={`${id}-menu`} role="menu" ref={menu} hidden={!open} onKeyDown={(e) => menuKeys(e)}>
        {items.map(({ k, label, hint }) => {
          const on = k === value;
          return (
            <button
              key={k}
              type="button"
              class={item}
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
  const [savedSort, savedDir] =
    prefix === 'r' ? [fields.rsort.value, fields.rdir.value] : [fields.csort.value, fields.cdir.value];
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
      what={what}
      items={RELEASE_NAME}
      value={sortBy}
      arrow={direction === 'desc' ? '↓' : '↑'}
      title={`Sorted ${HOW[`${sortBy}-${direction}`]}`}
      onPick={pick}
    />
  );
}
