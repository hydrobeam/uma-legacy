// ---------- a target as the game's skill tile: icon and name (the target skills editor's and the Presets view's) ----------
// A gold the chosen scenario guarantees shows as that gold (It's On! for Ramp Up), on the game's gold tile. The editor's
// tiles (`list`) are compact, to fit the setup's column, with a remove button (the game's close cross); the Presets
// view's (`view`) are the game's full size, the tiles alone.
import { fields } from '../settings.ts';
import { goldGiven } from '../job.ts';
import { namesOf } from '../targets.ts';
import { SkillIcon } from './Images.tsx';
import { cx } from 'tailwind-variants/lite';

// A list key per tile of `groups`: the group, and its count before when it repeats (only a hand-edited import repeats
// one), so each key is unique and a tile keeps its key when another is removed
export function groupKeys(groups: readonly number[]) {
  const seen = new Map<number, number>();
  return groups.map((group) => {
    const before = seen.get(group) ?? 0;
    seen.set(group, before + 1);
    return before ? `${group}:${before}` : String(group);
  });
}
// each size's tile padding, its name's gap and type (a long name smaller, as the game scales one down to fit) and its
// icon (the list's a little smaller on a narrow phone)
const SIZE = {
  list: {
    tile: 'min-h-9 pt-0.5 pr-0.75 pb-1.5 pl-1.5',
    name: 'text-ui max-sm:text-note gap-1.25 max-sm:gap-1',
    long: 'text-caption gap-1.25 max-sm:gap-1',
    icon: 'size-6 max-sm:size-5.5',
  },
  view: {
    tile: 'min-h-12 pt-1 pr-1.5 pb-2 pl-2.5',
    name: 'text-body gap-2',
    long: 'text-body gap-2',
    icon: 'size-7.5',
  },
} as const;
/** the tile's remove button: its name, and what a press does */
interface Remove {
  label: string;
  onClick: () => void;
}
export function TargetTile({
  group,
  id,
  size,
  remove,
}: {
  group: number;
  id?: string;
  size: keyof typeof SIZE;
  remove?: Remove;
}) {
  const names = namesOf(group),
    given = goldGiven(fields.scen.value, group),
    gold = given && !!names.gd,
    label = (gold && names.gd) || names.w,
    look = SIZE[size];
  return (
    // data-g: "2" for a gold the scenario guarantees (the checks read it)
    <div
      class={cx(
        'relative grid grid-cols-[minmax(0,1fr)_auto] items-center gap-1',
        given ? 'nine-skill-gold' : 'nine-skill-white',
        look.tile,
      )}
      id={id}
      data-g={given ? '2' : '0'}
    >
      {/* a plain target's name is on the tile: no hover. Keyed by gold: Preact sets `title` as a property, so a title
          taken away would stay as title="" */}
      <span
        key={gold ? 'gold' : 'white'}
        class={cx(
          'flex min-w-0 items-center leading-tight font-bold',
          given ? 'text-gm-gold-ink' : 'text-gm-ink',
          label.length > 18 ? look.long : look.name,
        )}
        title={gold ? `Gold of ${names.w}, guaranteed by the scenario` : undefined}
      >
        <SkillIcon group={group} gold={gold} cls={look.icon} />
        {label}
      </span>
      {remove && (
        // the game's close cross, 24 px, its touch area 36 px; on the dark white tile lightened
        <button
          class={cx(
            'after:art-close relative size-6 rounded-full before:absolute before:-inset-1.5 after:absolute after:inset-0 active:translate-y-px',
            given ? 'hover:bg-gm-gold-ink/12' : 'hover:bg-tile-hover dark:after:brightness-220 dark:after:saturate-50',
          )}
          type="button"
          aria-label={remove.label}
          title={remove.label}
          onClick={remove.onClick}
        />
      )}
    </div>
  );
}
