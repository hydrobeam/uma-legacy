// ---------- the umas' and the cards' sort menu: Release (newest first) or Name (A to Z), in the settings ----------
// (the menu itself: ui/SortMenu.tsx)
import { fields, setSort } from '../state/settings.ts';
import { SortMenu } from '../ui/SortMenu.tsx';
import type { SortItem } from '../ui/SortMenu.tsx';

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
