// ---------- a node's sparks as the game's Inspiration cells ----------
// Her blue (stat), pink (aptitude) and unique (green) sparks first, each a bar of its colour (a dashed one when she has
// none) that opens its picker; then her target skills' white sparks (click: stars 1 -> 2 -> 3 -> 1, the corner x
// removes), then "+" (a target skill she lacks). Sparks of skills that are not targets are kept but not shown
// (read-only: shown, after the targets, greyed). A spark's stars are two rows, lit then unlit, hidden from the screen
// reader (the control's label says the count). A chip is keyed by its group, so a click that changes its stars patches
// the chip and the focus stays on it. Sparks draws `nd` and reads no store: it draws again whenever its parent does (a
// new record from the veterans store, or a draft edited in place whose card draws again).
// Every edit goes through `at` (the node by where it is), from her record as it is when the edit lands. The colour
// picker and the "+" list are veterans/sparks.ts's and pickers.ts's (openColour, openAdd).
// The look: two bars to a row, each the game's spark sprite (9-sliced over its gradient: the theme's nine-spark-* and
// art-spark-*), its dot at the left, its name cut short at the bar's end, its stars hanging off its foot; 28 px tall on
// a narrow phone, 34 px elsewhere. Each spark's cell is data-spark (white, pink, blue, green; data-none where she has
// none, data-off for a spark not a target), its bar data-bar, its name data-spark-name (details.ts fitSparks shrinks
// it to fit).
import { useRef } from 'preact/hooks';
import { NEW_STARS } from '../veterans/store.ts';
import { pinkName, skillName, starsOf, uniqueName } from '../veterans/names.ts';
import { addOpener, openAdd } from '../veterans/pickers.ts';
import { KIND, currentSpark, openColour } from '../veterans/sparks.ts';
import type { NodeRecord, NodeRef } from '../veterans/store.ts';
import { cx, tv } from 'tailwind-variants/lite';
import type { VariantProps } from 'tailwind-variants/lite';

type Kind = 'blue' | 'pink' | 'green';
/** a spark's cell (its bar, the stars hanging under it, its corner x): in a list, room under it for the stars; the
 *  colour picker's choices have none */
const SPARK_CELL = {
  list: 'group/spark relative block min-w-0 pb-3 max-sm:pb-2.5',
  pick: 'relative block min-w-0',
} as const;
/** a spark's bar: `look`, its colour (`none`, a dashed bar where she has no spark: blue's dashes, or `none-green` for
 *  the unique); `off`, greyed (a spark not a target); `use`, a control (`edit`), a parent's (`parent`: the plain
 *  cursor), read-only (`view`), or a colour picker's choice (`pick`, ringed in brown while it is hers) */
const sparkBar = tv({
  base: 'text-body max-sm:text-note relative isolate grid h-8.5 w-full min-w-0 grid-cols-[20px_minmax(0,1fr)] items-center gap-x-1.5 rounded-md pr-2.5 pl-1.75 text-left leading-tight font-bold before:absolute before:inset-0 before:-z-1 after:col-start-1 after:row-start-1 after:size-4.5 after:rounded-full max-sm:h-7 max-sm:grid-cols-[12px_minmax(0,1fr)] max-sm:gap-x-1 max-sm:pr-1.25 max-sm:pl-1 max-sm:after:size-3',
  variants: {
    look: {
      white:
        'art-spark-white text-spark-ink inset-shadow-spark-white before:nine-spark-white max-sm:before:nine-spark-white-sm after:art-spark-dot',
      pink: 'art-spark-pink text-gm-pink-ink text-shadow-spark-pink inset-shadow-spark-pink before:nine-spark-pink max-sm:before:nine-spark-pink-sm after:art-spark-dot',
      blue: 'art-spark-blue text-shadow-spark-blue inset-shadow-spark-blue before:nine-spark-blue max-sm:before:nine-spark-blue-sm after:art-spark-dot text-white',
      green:
        'art-spark-green text-shadow-spark-green inset-shadow-spark-green before:nine-spark-green max-sm:before:nine-spark-green-sm after:art-spark-dot-gold text-white',
      none: 'bg-gm-cell text-gm-ink2 border-spark-none after:art-spark-dot border-2 border-dashed after:opacity-50',
      'none-green':
        'bg-gm-cell text-gm-ink2 border-spark-none-green after:art-spark-dot-gold border-2 border-dashed after:opacity-50',
    },
    off: { true: 'opacity-55 grayscale', false: '' },
    use: {
      edit: 'focus-visible:outline-accent cursor-pointer hover:brightness-104 focus-visible:outline-3 focus-visible:outline-offset-2',
      parent:
        'focus-visible:outline-accent cursor-default hover:brightness-104 focus-visible:outline-3 focus-visible:outline-offset-2',
      view: 'cursor-default',
      pick: 'focus-visible:not-aria-pressed:outline-accent aria-pressed:outline-gm-ink cursor-pointer hover:brightness-104 focus-visible:outline-3 focus-visible:not-aria-pressed:outline-offset-2 aria-pressed:outline-3 aria-pressed:outline-offset-1',
    },
  },
  defaultVariants: { look: 'white', off: false, use: 'edit' },
});
type BarUse = NonNullable<VariantProps<typeof sparkBar>['use']>;
/** a spark's name, after its dot, cut short at the bar's end; a long one (more than 18 letters) a step smaller */
const SPARK_NAME = {
  short: 'col-start-2 block truncate',
  long: 'text-ui max-sm:text-caption col-start-2 block truncate tracking-tight',
} as const;
// a row of lit or unlit stars, by how many: the game's spacing, a star every 18 px (15 on a phone), 3 px apart
const STAR_ROW = ['w-0', 'w-3.75 max-sm:w-3', 'w-8.25 max-sm:w-6.75', 'w-12.75 max-sm:w-10.5'] as const;
const starRow = (n: number) => STAR_ROW[Math.min(3, Math.max(0, n))];
/** a spark's stars, lit then unlit, out of 3, centred on the bar's foot */
function Stars({ n }: { n: number }) {
  return (
    <span
      class="absolute -bottom-1.25 left-1/2 z-1 flex h-3.75 -translate-x-1/2 max-sm:-bottom-1 max-sm:h-3"
      aria-hidden="true"
    >
      <span
        class={cx(
          'art-star-on max-sm:art-star-on-sm drop-shadow-star block h-3.75 max-sm:h-3',
          starRow(n),
          n > 0 && 'mr-0.75',
        )}
      />
      <span
        class={cx('art-star-off max-sm:art-star-off-sm drop-shadow-star block h-3.75 max-sm:h-3', starRow(3 - n))}
      />
    </span>
  );
}
// a white spark's corner x: under the pointer or with the focus in its cell (always on a screen without hover), a
// 38 px tap area round it
const REMOVE =
  'text-note text-out border-gm-edge bg-gm-cell focus-visible:outline-accent invisible absolute -top-1.5 -right-1 size-4.5 cursor-pointer rounded-full border p-0 leading-none group-focus-within/spark:visible group-hover/spark:visible touch:visible before:absolute before:-inset-2.5 focus-visible:outline-2 focus-visible:outline-offset-1';
// "+": a dashed cell as tall as a bar
const ADD =
  'border-gm-edge bg-gm-cell text-gm-ink text-title focus-visible:outline-accent h-8.5 cursor-pointer self-start rounded-md border-2 border-dashed px-2.25 font-black focus-visible:outline-2 focus-visible:outline-offset-1 max-sm:h-7';
interface ChipProps {
  label: string;
  stars: number;
  /** its colour: a white (skill), pink (aptitude) or green (unique) spark */
  spark: 'white' | 'pink' | 'green';
  /** the control's name (data-k: the tests find it by it) */
  k: string;
  readOnly?: boolean | undefined;
  other?: boolean;
  /** a parent's (the plain cursor) */
  parent?: boolean | undefined;
  /** after the label in its names: what kind of spark it is (a unique's " (unique)") */
  tag?: string;
  /** the face's label, when not the chip's own */
  aria?: string;
  /** a face that opens a picker */
  popup?: boolean;
  onCycle: () => void;
  /** its corner x (none without) */
  onRemove?: () => void;
}
const barUse = (readOnly: boolean | undefined, parent: boolean | undefined): BarUse =>
  readOnly ? 'view' : parent ? 'parent' : 'edit';
/** one spark as a chip: click for the next star count, the corner x removes */
function Chip(props: ChipProps) {
  const { label, stars, spark, k, readOnly, other = false, parent, tag = '', aria, popup, onCycle, onRemove } = props,
    named = `${label}${spark === 'pink' ? ' (aptitude)' : ''}${tag}`;
  const inner = (
    <>
      <span class={label.length > 18 ? SPARK_NAME.long : SPARK_NAME.short} data-spark-name="">
        {label}
      </span>
      <Stars n={stars} />
    </>
  );
  const bar = sparkBar({ look: spark, off: other, use: barUse(readOnly, parent) });
  if (readOnly)
    return (
      <span class={SPARK_CELL.list} data-spark={spark} data-off={other ? '' : undefined}>
        {/* biome-ignore lint/a11y/useAriaPropsSupportedByRole: today's markup (the el() chip's before it): the label names a read-only spark; recorded, not changed: a role (img) would read it as a picture */}
        <span class={bar} data-bar="" aria-label={`${named}, ${stars} stars${other ? ', not a target' : ''}`}>
          {inner}
        </span>
      </span>
    );
  return (
    <span class={SPARK_CELL.list} data-spark={spark}>
      <button
        type="button"
        class={bar}
        data-bar=""
        aria-label={aria ?? `${named}, ${stars} star${stars > 1 ? 's' : ''}. Click for ${(stars % 3) + 1}`}
        aria-haspopup={popup ? 'dialog' : undefined}
        data-k={k}
        onClick={onCycle}
      >
        {inner}
      </button>
      {onRemove && (
        <button
          type="button"
          class={REMOVE}
          aria-label={`Remove ${label}`}
          title={`Remove ${label}`}
          data-k={`${k}x`}
          onClick={onRemove}
        >
          ×
        </button>
      )}
    </span>
  );
}
interface BarProps {
  nd: NodeRecord;
  kind: Kind;
  k: string;
  at: NodeRef;
  readOnly: boolean;
  parent: boolean;
}
/** her blue, pink or unique spark as its coloured bar: a chip for a unique or a pink she has, else
 *  a bar that opens the picker (a missing unique's adds it). The chip is drawn by calling Chip (it holds no hooks), not
 *  as a component of its own: the bar is then the same span and button whether she has the spark or not, so adding the
 *  unique back or setting a first pink keeps the button, and the focus on it (a <Chip> here would replace them) */
function ColourBar({ nd, kind, k, at, readOnly, parent }: BarProps) {
  const spec = KIND[kind],
    cur = currentSpark(nd, kind);
  if (kind === 'green') {
    if (cur) {
      const [group, stars] = cur,
        label = uniqueName(group) || 'Unique spark';
      return Chip({
        label,
        stars,
        spark: 'green',
        k,
        readOnly,
        parent,
        tag: ' (unique)',
        onCycle: () => at.edit((now) => ({ greens: { ...now.greens, [group]: (stars % 3) + 1 } })),
        onRemove: () =>
          at.edit((now) => {
            const { [group]: _, ...rest } = (now.greens || {}) as Record<string, number>;
            return { greens: rest };
          }),
      });
    }
    return (
      <span class={SPARK_CELL.list} data-spark="green" data-none="">
        <button
          type="button"
          class={sparkBar({ look: 'none-green', use: barUse(false, parent) })}
          data-bar=""
          data-k={k}
          aria-label="Unique spark: none. Add it at 2 stars"
          onClick={() => at.edit((now) => ({ greens: { [now.outfit as number]: NEW_STARS } }))}
        >
          <span class={SPARK_NAME.short} data-spark-name="">
            + Unique spark
          </span>
        </button>
      </span>
    );
  }
  if (cur && kind === 'pink') {
    const [group, stars] = cur;
    return Chip({
      label: pinkName(group),
      stars,
      spark: 'pink',
      k,
      readOnly,
      parent,
      popup: true,
      aria: `${pinkName(group)} (aptitude), ${stars} star${stars > 1 ? 's' : ''}. Change`,
      onCycle: () => openColour(at, kind),
    });
  }
  // a blue (she has one or not), or no pink yet (drawn as a missing blue is: the old sheet's look)
  const label = cur ? spec.name(cur[0]) || spec.title : `${spec.title}: none`,
    aria = cur
      ? `${label} (${kind === 'blue' ? 'stat' : 'unique'}), ${cur[1]} star${cur[1] > 1 ? 's' : ''}${readOnly ? '' : '. Change'}`
      : `${spec.title}: none. Set it`,
    bar = sparkBar({ look: cur ? 'blue' : 'none', use: barUse(readOnly, parent) }),
    inner = (
      <>
        <span class={SPARK_NAME.short} data-spark-name="">
          {label}
        </span>
        {cur && <Stars n={cur[1]} />}
      </>
    );
  return (
    <span class={SPARK_CELL.list} data-spark={kind} data-none={cur ? undefined : ''}>
      {readOnly ? (
        // biome-ignore lint/a11y/useAriaPropsSupportedByRole: today's markup (the el() bar's before it): a read-only bar's label; its role is step 10's
        <span class={bar} data-bar="" aria-label={aria}>
          {inner}
        </span>
      ) : (
        <button
          type="button"
          class={bar}
          data-bar=""
          data-k={k}
          aria-haspopup="dialog"
          aria-label={aria}
          onClick={() => openColour(at, kind)}
        >
          {inner}
        </button>
      )}
    </span>
  );
}
/** "+": the target skills she lacks, in pickers.ts's list; its aria-expanded drawn from `addOpener` */
function AddSpark({ k, at }: { k: string; at: NodeRef }) {
  const ref = useRef<HTMLButtonElement>(null),
    open = addOpener.value !== null && addOpener.value === ref.current;
  return (
    <button
      ref={ref}
      type="button"
      class={ADD}
      aria-label="Add a target skill spark"
      title="Add a target skill spark"
      aria-haspopup="dialog"
      aria-expanded={open}
      data-k={`${k}:add`}
      onClick={(e) => openAdd(e.currentTarget, at)}
    >
      +
    </button>
  );
}
interface SparksProps {
  /** the node drawn */
  nd: NodeRecord;
  /** names the controls (data-k) */
  k: string;
  /** where her edits go */
  at: NodeRef;
  /** the target skills' groups */
  targets: readonly number[];
  readOnly?: boolean;
  /** a parent's sparks (her row under Legacy Origin): the plain cursor over them */
  parent?: boolean;
}
/** her sparks: her blue, pink and unique bars, her target skills' chips, then "+"; read-only, all her
 *  white sparks, the targets first, the rest grey */
export function Sparks({ nd, k, at, targets: tg, readOnly = false, parent = false }: SparksProps) {
  const targets = new Set(tg),
    groups = readOnly
      ? Object.keys(nd.sparks)
          .map(Number)
          .sort((a, b) => Number(targets.has(b)) - Number(targets.has(a)))
      : tg;
  const bars = (['blue', 'pink', 'green'] as const).filter(
    (kind) => currentSpark(nd, kind) || (!readOnly && KIND[kind].groups(nd).length),
  );
  return (
    <div class="grid grid-cols-2 gap-x-2 max-sm:gap-x-1.5">
      {bars.map((kind) => (
        <ColourBar key={kind} nd={nd} kind={kind} k={`${k}:${kind}`} at={at} readOnly={readOnly} parent={parent} />
      ))}
      {groups
        .map((g) => [g, starsOf(nd.sparks, g)] as const)
        .filter(([, stars]) => stars > 0)
        .map(([g, stars]) => {
          return (
            <Chip
              key={g}
              label={skillName(g)}
              stars={stars}
              spark="white"
              k={`${k}:s${g}`}
              readOnly={readOnly}
              other={readOnly && !targets.has(g)}
              parent={parent}
              onCycle={() => at.edit((now) => ({ sparks: { ...now.sparks, [g]: (stars % 3) + 1 } }))}
              onRemove={() =>
                at.edit((now) => {
                  const { [g]: _, ...rest } = now.sparks as Record<number, number>;
                  return { sparks: rest };
                })
              }
            />
          );
        })}
      {!readOnly && <AddSpark k={k} at={at} />}
    </div>
  );
}
