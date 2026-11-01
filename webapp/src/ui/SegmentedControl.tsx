// ---------- the game's pill switch: a row of options, one chosen, as one control ----------
// `kind` is what it is to assistive tech and the keyboard: `radio` (a radiogroup of radios, aria-checked: one value of a
// setting), `tab` (a tablist of tabs, aria-selected: which pane shows) or `toggle` (a group of toggle buttons,
// aria-pressed). Radios and tabs are one Tab stop, the chosen option; an arrow key picks the next option (Right, and
// Down for radios) or the one before (Left, Up), wrapping, and focuses it; a tab list's Home and End pick its first and
// last. Toggle buttons are each a Tab stop. Each option is keyed by its value, so the focused one stays the same element
// after the pick.
// `shape`: `joined`, one pill track its options share, as wide as they need; `fill`, the track its row's width, its
// options sharing what their labels leave; `equal`, the same in equal shares; `apart`, separate pills that wrap (a pill
// with a portrait in it hugs it). The chosen option is the game's green, outlined white text, whichever ARIA state says
// so (the theme's `on:` variant). `size` is a joined track's (type, padding, height), by where it sits.
import type { ComponentChildren } from 'preact';
import { useRef } from 'preact/hooks';
import { tv } from 'tailwind-variants/lite';
import type { VariantProps } from 'tailwind-variants/lite';
import { around } from './arrows.ts';

const track = tv({
  variants: {
    shape: {
      joined: 'border-gm-tab-edge bg-gm-tab inset-shadow-well inline-flex overflow-hidden rounded-full border',
      fill: 'border-gm-tab-edge bg-gm-tab inset-shadow-well flex w-full overflow-hidden rounded-full border *:flex-auto',
      equal:
        'border-gm-tab-edge bg-gm-tab inset-shadow-well grid w-full auto-cols-fr grid-flow-col overflow-hidden rounded-full border',
      apart: 'flex flex-wrap items-center gap-1.5',
    },
  },
});
// a joined option: the track's line between it and the one before, the chosen one green with its rim (the text sits a
// little low: the game's pill has a darker foot, and Zen Maru sets Latin text low in its line)
const option = tv({
  base: 'group focus-visible:outline-accent not-on:text-gm-ink on:art-seg-on on:text-white on:inset-shadow-seg-on on:text-shadow-outline not-on:not-first:inset-shadow-divide inline-flex items-center justify-center gap-0.75 font-bold focus-visible:outline-3 focus-visible:-outline-offset-3',
  variants: {
    size: {
      /** the loop page's Results, in the results' corner */
      xs: 'text-note min-h-7 min-w-8 px-2.25 pt-0 pb-0.75 max-md:min-h-9 max-md:min-w-10',
      /** small type (a veteran's Yours or Guest) */
      sm: 'text-note px-2.5 pt-px pb-1.25 max-md:min-h-11',
      /** the loop page's setup rows */
      md: 'text-ui min-h-8.5 px-2.5 pt-px pb-1.25 leading-tight max-md:min-h-11',
      /** the career page's setup rows, a dialog's tabs */
      lg: 'text-body min-h-8.5 px-3.5 pt-px pb-1.25 max-md:min-h-11 max-sm:leading-tight',
      /** a card's own tabs (her details) */
      xl: 'text-body min-h-10.5 px-3.5 pt-px pb-1.25',
    },
  },
});
// a separate pill, its own size (a pill whose body holds elements, a portrait first, hugs it at the left)
const apart =
  'group inline-flex items-center gap-1.5 rounded-full border pt-0.75 pr-3 pb-1.25 text-ui font-bold focus-visible:outline-3 focus-visible:outline-offset-2 focus-visible:outline-accent not-has-[>span]:pl-3 has-[>span]:pl-1 not-on:border-gm-tab-edge not-on:bg-gm-tab not-on:text-gm-ink not-on:inset-shadow-well on:border-gm-rim on:art-seg-on on:text-white on:inset-shadow-seg-on on:text-shadow-outline max-md:min-h-11';
type Kind = 'radio' | 'tab' | 'toggle';
type Shape = NonNullable<VariantProps<typeof track>['shape']>;
type Size = NonNullable<VariantProps<typeof option>['size']>;
const GROUP = { radio: 'radiogroup', tab: 'tablist', toggle: 'group' } as const;

export interface Segment<V> {
  v: V;
  body: ComponentChildren;
  /** its accessible name, when the body's text is not */
  label?: string | undefined;
  title?: string | undefined;
  id?: string | undefined;
  /** a tab's pane */
  controls?: string | undefined;
  /** a `data-v` attribute (a check names the option by it) */
  dataV?: string | undefined;
  /** a `data-k` attribute (a check names the option by it) */
  dataK?: string | undefined;
}
interface SegmentedProps<V> {
  kind: Kind;
  shape?: Shape;
  /** a joined track's options' size (`apart` has its own) */
  size?: Size;
  /** the group's element ('div' or 'span': where it sits) */
  tag?: 'div' | 'span';
  id?: string | undefined;
  /** the group's name: an element's id, or the name itself */
  labelledBy?: string | undefined;
  label?: string | undefined;
  hidden?: boolean | undefined;
  /** a `data-value` attribute on the group (a check reads the chosen value by it) */
  dataValue?: string | undefined;
  /** where it sits: layout utilities only */
  class?: string | undefined;
  /** shown before the options, inside the group (a visible label) */
  lead?: ComponentChildren | undefined;
  /** shown after them, inside the group (a key to the pane) */
  trail?: ComponentChildren | undefined;
  options: Segment<V>[];
  value: V;
  onPick: (v: V) => void;
}
const BACK = ['ArrowLeft', 'ArrowUp'],
  ON = ['ArrowRight', 'ArrowDown'];
// the option a key moves to from `i` (undefined: the key isn't one of the kind's)
function moveTo(kind: Kind, key: string, i: number, count: number) {
  if (kind === 'tab') return { ArrowLeft: i - 1, ArrowRight: i + 1, Home: 0, End: count - 1 }[key];
  if (BACK.includes(key)) return i - 1;
  if (ON.includes(key)) return i + 1;
  return undefined;
}
export function SegmentedControl<V extends string | number | boolean>(props: SegmentedProps<V>) {
  const { kind, shape = 'joined', size = 'lg', tag: Tag = 'div', options, value, onPick } = props;
  // The value as last picked, ahead of the re-render: a radio's second pick in the same task (a click and its undoing)
  // compares with the first pick, not with the value this render drew
  const picked = useRef(value);
  picked.current = value;
  const pick = (v: V) => {
    if (kind === 'radio' && v === picked.current) return;
    picked.current = v;
    onPick(v);
  };
  const roving = kind !== 'toggle';
  const step = (e: KeyboardEvent & { currentTarget: HTMLElement }) => {
    if (!roving) return;
    const to = moveTo(
      kind,
      e.key,
      options.findIndex((o) => o.v === picked.current),
      options.length,
    );
    if (to === undefined) return;
    e.preventDefault();
    pick(around(options, to).v);
    // the option is kept (keyed by its value): focus it now, before the re-render (one per option, in their order)
    around([...e.currentTarget.querySelectorAll<HTMLElement>('[role="radio"], [role="tab"]')], to).focus();
  };
  return (
    <Tag
      class={track({ shape, class: props.class })}
      id={props.id}
      role={GROUP[kind]}
      aria-labelledby={props.labelledBy}
      aria-label={props.label}
      hidden={props.hidden}
      data-value={props.dataValue}
      data-seg={shape}
      onKeyDown={step}
    >
      {props.lead}
      {options.map((o) => {
        const on = o.v === value;
        return (
          // biome-ignore lint/a11y/useAriaPropsSupportedByRole: each state goes with its kind's role (aria-checked a radio's, aria-selected a tab's, aria-pressed a plain button's), which Biome can't follow through `kind`
          <button
            key={String(o.v)}
            type="button"
            class={shape === 'apart' ? apart : option({ size })}
            role={kind === 'toggle' ? undefined : kind}
            id={o.id}
            data-v={o.dataV}
            data-k={o.dataK}
            aria-label={o.label}
            title={o.title}
            aria-checked={kind === 'radio' ? on : undefined}
            aria-selected={kind === 'tab' ? on : undefined}
            aria-pressed={kind === 'toggle' ? on : undefined}
            aria-controls={o.controls}
            tabIndex={roving ? (on ? 0 : -1) : undefined}
            onClick={() => pick(o.v)}
          >
            {o.body}
          </button>
        );
      })}
      {props.trail}
    </Tag>
  );
}
