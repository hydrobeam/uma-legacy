// ---------- a menu button over a list of choices (P8: one keyboard and close behaviour for every such menu) ----------
// `SortMenu`: a sort button and its menu (`id`, `${id}-menu`, `${id}-dd`): pick an order; picking the checked one again
// reverses it (the caller says what a pick does). The menu stays open until a click outside it, Escape, or the focus
// leaving it; its keys and closing are dropdown.ts's, which the scenario picker shares.
// `SettingsSort` is the umas' and the cards' menu, Release or Name, kept in the settings.
import { useRef, useState } from 'preact/hooks';
import { fields, setSort } from '../settings.ts';
import { menuKeys, opensOnArrow, useDropdown, useFocusChecked } from './dropdown.ts';

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
