// ---------- a node's sparks as the game's Inspiration cells ----------
// Her blue (stat), pink (aptitude) and unique (green) sparks first, each a bar of its colour (a dashed one when she has
// none) that opens its picker; then her target skills' white sparks (click: stars 1 -> 2 -> 3 -> 1, the corner x
// removes), then "+" (a target skill she lacks). Sparks of skills that are not targets are kept but not shown
// (read-only: shown, after the targets, greyed). A spark's stars are two spans (the game-style card colours them
// apart), hidden from the screen reader (the control's label says the count). A chip is keyed by its group, so a click that
// changes its stars patches the chip and the focus stays on it. Sparks draws `nd` and reads no store: it draws again
// whenever its parent does (a new record from the veterans store, or a draft edited in place whose card draws again).
// Every edit goes through `at` (the node by where it is), from her record as it is when the edit lands. The colour
// picker and the "+" list are veterans/sparks.js's and pickers.js's (openColour, openAdd).
import { useRef } from 'preact/hooks';
import { NEW_STARS } from '../veterans/store.ts';
import { pinkName, skillName, uniqueName } from '../veterans/names.js';
import { addOpener, openAdd } from '../veterans/pickers.js';
import { KIND, currentSpark, openColour } from '../veterans/sparks.js';
import type { NodeRecord, NodeRef } from '../veterans/store.ts';
import { SkillIcon } from './Images.tsx';

type Kind = 'blue' | 'pink' | 'green';
/** a spark's stars, filled then empty, out of 3 */
export function Stars({ n }: { n: number }) {
  return (
    <span class="stars" aria-hidden="true">
      <span class="sf" style={{ '--n': String(n) }}>
        {'★'.repeat(n)}
      </span>
      <span class="se" style={{ '--n': String(3 - n) }}>
        {'☆'.repeat(3 - n)}
      </span>
    </span>
  );
}
interface ChipProps {
  label: string;
  group?: number;
  stars: number;
  pink?: boolean;
  /** the control's name (data-k: the tests find it by it) */
  k: string;
  readOnly?: boolean;
  other?: boolean;
  /** after the label in its names: what kind of spark it is (a unique's " (unique)") */
  tag?: string;
  /** the face's label, when not the chip's own */
  aria?: string;
  cls?: string;
  /** a face that opens a picker */
  popup?: boolean;
  onCycle: () => void;
  /** its corner x (none without) */
  onRemove?: () => void;
}
/** one spark as a chip: click for the next star count, the corner x removes */
function Chip(props: ChipProps) {
  const {
      label,
      group,
      stars,
      pink = false,
      k,
      readOnly,
      other,
      tag = '',
      aria,
      cls,
      popup,
      onCycle,
      onRemove,
    } = props,
    named = `${label}${pink ? ' (aptitude)' : ''}${tag}`;
  const inner = (
    <>
      {group !== undefined && <SkillIcon group={group} />}
      <span class={`vskn${label.length > 18 ? ' long' : ''}`}>{label}</span>
      <Stars n={stars} />
    </>
  );
  const c = cls ?? `vsk${pink ? ' pink' : ''}${other ? ' off' : ''}`;
  if (readOnly)
    return (
      <span class={c}>
        {/* biome-ignore lint/a11y/useAriaPropsSupportedByRole: today's markup (the el() chip's before it): the label names a read-only spark; a role for it is the SparkChips port's (step 10) */}
        <span class="vskb" aria-label={`${named}, ${stars} stars${other ? ', not a target' : ''}`}>
          {inner}
        </span>
      </span>
    );
  return (
    <span class={c}>
      <button
        type="button"
        class="vskb"
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
          class="vskx"
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
}
/** her blue, pink or unique spark as its coloured bar: a chip for a unique or a pink she has, else
 *  a bar that opens the picker (a missing unique's adds it). The chip is drawn by calling Chip (it holds no hooks), not
 *  as a component of its own: the bar is then the same span and button whether she has the spark or not, so adding the
 *  unique back or setting a first pink keeps the button, and the focus on it (a <Chip> here would replace them) */
function ColourBar({ nd, kind, k, at, readOnly }: BarProps) {
  const spec = KIND[kind],
    cur = currentSpark(nd, kind);
  if (kind === 'green') {
    if (cur) {
      const [group, stars] = cur,
        label = uniqueName(group) || 'Unique spark';
      return Chip({
        label,
        stars,
        k,
        readOnly,
        cls: 'vsb green',
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
      <span class="vsb green none">
        <button
          type="button"
          class="vskb"
          data-k={k}
          aria-label="Unique spark: none. Add it at 2 stars"
          onClick={() => at.edit((now) => ({ greens: { [now.outfit as number]: NEW_STARS } }))}
        >
          <span class="vskn">+ Unique spark</span>
        </button>
      </span>
    );
  }
  if (cur && kind === 'pink') {
    const [group, stars] = cur;
    return Chip({
      label: pinkName(group),
      stars,
      pink: true,
      k,
      readOnly,
      popup: true,
      aria: `${pinkName(group)} (aptitude), ${stars} star${stars > 1 ? 's' : ''}. Change`,
      onCycle: () => openColour(at, kind),
    });
  }
  // a blue (she has one or not), or no pink yet
  const label = cur ? spec.name(cur[0]) || spec.title : `${spec.title}: none`,
    aria = cur
      ? `${label} (${kind === 'blue' ? 'stat' : 'unique'}), ${cur[1]} star${cur[1] > 1 ? 's' : ''}${readOnly ? '' : '. Change'}`
      : `${spec.title}: none. Set it`,
    inner = (
      <>
        <span class="vskn">{label}</span>
        {cur && <Stars n={cur[1]} />}
      </>
    );
  return (
    <span class={`vsb ${kind}${cur ? '' : ' none'}`}>
      {readOnly ? (
        // biome-ignore lint/a11y/useAriaPropsSupportedByRole: today's markup (the el() bar's before it): a read-only bar's label; its role is step 10's
        <span class="vskb" aria-label={aria}>
          {inner}
        </span>
      ) : (
        <button
          type="button"
          class="vskb"
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
/** "+": the target skills she lacks, in pickers.js's list; its aria-expanded drawn from `addOpener` */
function AddSpark({ k, at }: { k: string; at: NodeRef }) {
  const ref = useRef<HTMLButtonElement>(null),
    open = addOpener.value !== null && addOpener.value === ref.current;
  return (
    <button
      ref={ref}
      type="button"
      class="vskadd"
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
}
/** her sparks: her blue, pink and unique bars, her target skills' chips, then "+"; read-only, all her
 *  white sparks, the targets first, the rest grey */
export function Sparks({ nd, k, at, targets: tg, readOnly = false }: SparksProps) {
  const targets = new Set(tg),
    sparks = nd.sparks as Record<number, number>,
    groups = readOnly
      ? Object.keys(sparks)
          .map(Number)
          .sort((a, b) => Number(targets.has(b)) - Number(targets.has(a)))
      : tg;
  const bars = (['blue', 'pink', 'green'] as const).filter(
    (kind) => currentSpark(nd, kind) || (!readOnly && KIND[kind].groups(nd).length),
  );
  return (
    <div class="vsparks">
      {bars.map((kind) => (
        <ColourBar key={kind} nd={nd} kind={kind} k={`${k}:${kind}`} at={at} readOnly={readOnly} />
      ))}
      {groups
        .filter((g) => sparks[g])
        .map((g) => {
          const stars = sparks[g];
          return (
            <Chip
              key={g}
              label={skillName(g)}
              group={g}
              stars={stars}
              k={`${k}:s${g}`}
              readOnly={readOnly}
              other={readOnly && !targets.has(g)}
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
