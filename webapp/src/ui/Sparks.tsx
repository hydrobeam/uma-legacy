// ---------- the component twin of veterans/sparks.js's spark editor (sparksEl): a node's sparks as the game's Inspiration cells ----------
// The markup sparksEl draws (veterans/sparks.js says what each part is). A chip is keyed by its group, so a click that changes its
// stars patches the chip and the focus stays on it. The record is edited in place, as the veterans' modules do (copy-on-write and
// its setters come with the veterans store, step 10); `onEdit` follows every edit. The colour picker and the "+" list
// stay veterans/sparks.js's and pickers.js's (openColour, openAdd). sparksEl stays for the views not yet components (the pool's cards, the
// veteran analysis).
import { useRef } from 'preact/hooks';
import { settings } from '../settings.ts';
import { NEW_STARS, vetsRev } from '../veterans/store.js';
import { pinkName, skillName, uniqueName } from '../veterans/names.js';
import { addOpener, openAdd } from '../veterans/pickers.js';
import { KIND, currentSpark, openColour } from '../veterans/sparks.js';
import type { LineageNode } from '../veterans/store.js';
import { SkillIcon } from './Images.tsx';

type Kind = 'blue' | 'pink' | 'green';
/** a spark's stars, filled then empty, out of 3 (sparks.js starsEl) */
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
  /** the control's name (data-k: the tests and the colour picker's return find it by it) */
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
/** one spark as a chip (sparks.js chip): click for the next star count, the corner x removes */
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
        {/* biome-ignore lint/a11y/useAriaPropsSupportedByRole: today's markup (sparks.js chip): the label names a read-only spark; a role for it is the SparkChips port's (step 10) */}
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
  nd: LineageNode;
  kind: Kind;
  k: string;
  onEdit: () => void;
  readOnly: boolean;
}
/** her blue, pink or unique spark as its coloured bar (sparks.js colourBar): a chip for a unique or a pink she has, else
 *  a bar that opens the picker (a missing unique's adds it) */
function ColourBar({ nd, kind, k, onEdit, readOnly }: BarProps) {
  const spec = KIND[kind],
    cur = currentSpark(nd, kind);
  if (kind === 'green') {
    if (cur) {
      const [group, stars] = cur,
        label = uniqueName(group) || 'Unique spark',
        greens = nd.greens as Record<string, number>;
      return (
        <Chip
          label={label}
          stars={stars}
          k={k}
          readOnly={readOnly}
          cls="vsb green"
          tag=" (unique)"
          onCycle={() => {
            greens[group] = (stars % 3) + 1;
            onEdit();
          }}
          onRemove={() => {
            delete greens[group];
            onEdit();
          }}
        />
      );
    }
    return (
      <span class="vsb green none">
        <button
          type="button"
          class="vskb"
          data-k={k}
          aria-label="Unique spark: none. Add it at 2 stars"
          onClick={() => {
            nd.greens = { [nd.outfit as number]: NEW_STARS };
            onEdit();
          }}
        >
          <span class="vskn">+ Unique spark</span>
        </button>
      </span>
    );
  }
  if (cur && kind === 'pink') {
    const [group, stars] = cur;
    return (
      <Chip
        label={pinkName(group)}
        stars={stars}
        pink
        k={k}
        readOnly={readOnly}
        popup
        aria={`${pinkName(group)} (aptitude), ${stars} star${stars > 1 ? 's' : ''}. Change`}
        onCycle={() => openColour(nd, kind, k, onEdit)}
      />
    );
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
        // biome-ignore lint/a11y/useAriaPropsSupportedByRole: today's markup (sparks.js colourBar): a read-only bar's label; its role is step 10's
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
          onClick={() => openColour(nd, kind, k, onEdit)}
        >
          {inner}
        </button>
      )}
    </span>
  );
}
/** "+": the target skills she lacks, in pickers.js's list; its aria-expanded drawn from `addOpener` */
function AddSpark({ nd, k, onEdit }: { nd: LineageNode; k: string; onEdit: () => void }) {
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
      onClick={(e) => openAdd(e.currentTarget, nd, onEdit, true)}
    >
      +
    </button>
  );
}
interface SparksProps {
  nd: LineageNode;
  /** names the controls (data-k) */
  k: string;
  onEdit: () => void;
  readOnly?: boolean;
}
/** her sparks (sparks.js sparksEl): her blue, pink and unique bars, her target skills' chips, then "+"; read-only, all her
 *  white sparks, the targets first, the rest grey */
export function Sparks({ nd, k, onEdit, readOnly = false }: SparksProps) {
  vetsRev.value; // (it reads a signal, so a parent's redraw with the same props skips it: the edits in place say when)
  const tg = settings.value.targets.map((t) => t.group),
    targets = new Set(tg),
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
        <ColourBar key={kind} nd={nd} kind={kind} k={`${k}:${kind}`} onEdit={onEdit} readOnly={readOnly} />
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
              onCycle={() => {
                sparks[g] = (stars % 3) + 1;
                onEdit();
              }}
              onRemove={() => {
                delete sparks[g];
                onEdit();
              }}
            />
          );
        })}
      {!readOnly && <AddSpark nd={nd} k={k} onEdit={onEdit} />}
    </div>
  );
}
