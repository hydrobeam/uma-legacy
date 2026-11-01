// ---------- the planner pages' shell: what both pages' layouts (career/Layout.tsx, loop/Layout.tsx) draw alike ----------
// The layouts are drawn twice: at build time into each page's HTML (vite/prerender.mjs, renderToString), so the setup's
// markup is there before any script runs, and on load by the entry, which hydrates that markup (one Preact root per
// page). So these modules import nothing but Preact: no store, no game data, no module with an effect at load. What
// the entry knows and the build does not (the era, the counts, the handlers) comes in as props once the entry has
// hydrated (`useHydrated`): until then each layout draws exactly what the build drew.
import type { ComponentChildren, Ref } from 'preact';
import { useLayoutEffect, useState } from 'preact/hooks';
import { cx } from 'tailwind-variants/lite';
import { ERA_NAMES } from '../eras.ts';
import type { Era } from '../eras.ts';
import { SegmentedControl } from './SegmentedControl.tsx';
import { popover } from './Popover.tsx';

/** false for the first draw (the hydration, which must draw what the build drew), true from the next */
export function useHydrated() {
  const [hydrated, setHydrated] = useState(false);
  useLayoutEffect(() => setHydrated(true), []);
  return hydrated;
}

/** the planner page a shared part sits on (each layout's root carries it as data-page, for the theme's `career:` and
 *  `loop:` variants) */
export type Page = 'career' | 'loop';
// ---------- the setup panel's rows: a label, then its control; the label column narrower on the loop page (and on a
// narrow phone, the career page's). As the game's Umamusume Details labels its rows (Track, Distance, Style): plain
// bold labels in a column, no tile behind them, no box round the row (the controls are the only framed things)
/** a setup row (its label, its control): Game data, Borrowed parent, the lineage rank, the scenario */
export const setRow = {
  career:
    'text-body grid grid-cols-[8.8em_minmax(0,1fr)] items-center justify-items-start gap-2.5 py-0.5 max-sm:grid-cols-[6.6em_minmax(0,1fr)] max-sm:gap-2',
  loop: 'text-body grid grid-cols-[8.4em_minmax(0,1fr)] items-center justify-items-start gap-2 py-0.5',
} as const;
/** a setup row's label (`stacked`: over its control, the row's width, on a narrow phone) */
export const setLabel = {
  beside:
    'text-body text-gm-ink flex items-center gap-1.5 self-stretch justify-self-stretch pr-1 pl-0.5 leading-tight font-bold',
  stacked:
    'text-body text-gm-ink flex items-center gap-1.5 self-stretch justify-self-stretch pr-1 pl-0.5 leading-tight font-bold max-sm:col-span-full max-sm:justify-self-start',
} as const;

/** the game data's era switch: the era on show pressed (Global at build time); a press of the other reloads into it */
export function EraRow({
  era = 'global',
  page,
  onPick,
}: {
  era?: Era | undefined;
  page: Page;
  onPick?: ((era: Era) => void) | undefined;
}) {
  return (
    <div class={setRow[page]}>
      <span class={setLabel.beside} id="era-l">
        Game data
      </span>
      <SegmentedControl<Era>
        kind="toggle"
        shape="fill"
        size={page === 'loop' ? 'md' : 'lg'}
        labelledBy="era-l"
        value={era}
        onPick={(v) => onPick?.(v)}
        options={[
          { v: 'global', id: 'era-global', title: "Global, today's data", body: ERA_NAMES.global },
          {
            v: 'gm',
            id: 'era-gm',
            title: "JP data from the Grand Masters era (cut at L'Arc's launch, 24 Aug 2023)",
            body: ERA_NAMES.gm,
          },
        ]}
      />
    </div>
  );
}

/** the setup panel's title: the game's pink ribbon (the Enhance menu's) along the panel's top left edge, its end cut on
 *  a slant with two white stripes near it; `fold`: it holds the button that folds the panel (career/Layout.tsx) */
export const Ribbon = ({ fold = false, children }: { fold?: boolean; children: ComponentChildren }) => (
  <h2
    class={cx(
      'art-ribbon clip-ribbon font-game text-display text-shadow-ribbon after:shadow-stripe relative -mt-px -ml-4.75 justify-self-start rounded-tl-2xl pt-1.5 pr-14 pb-1.75 pl-5 leading-tight font-black text-white after:absolute after:inset-y-0 after:right-6 after:w-1.75 after:-skew-x-16 after:bg-white/45 max-sm:-ml-3.25',
      fold && 'cursor-pointer after:pointer-events-none',
    )}
    id="h-set"
  >
    {children}
  </h2>
);

/** a count on an Enhance button: its badge and its tooltip (none until the entry has counted) */
export interface Badge {
  n: string;
  title: string;
}
// ---------- the game's Enhance menu buttons: the game's art (its frame and white lower band, the art spilling over its
// top edge; the drop shadow follows that shape), the label on the band with a ▶ beside its first line, the count owned
// under it in small type. Umas and Support cards are square, one size on both pages (the label's margins a little wider
// on the career page); Veteran Umamusume is wide, its two-line label right of the art, as wide as the row of Enhance
// buttons
const ENH_ART = { umas: 'art-enh-umas', cards: 'art-enh-cards', vets: 'art-enh-vets' } as const;
const ENH = {
  career: {
    button: 'h-28 w-35',
    label:
      'after:border-l-enh-ink text-body text-enh-ink absolute right-6 bottom-2.25 left-2 text-center leading-tight font-black whitespace-nowrap after:absolute after:top-0.875 after:-right-3.25 after:border-y-6 after:border-l-8 after:border-y-transparent',
  },
  loop: {
    button: 'h-28 w-35',
    label:
      'after:border-l-enh-ink text-body text-enh-ink absolute right-5 bottom-2.25 left-1.5 text-center leading-tight font-black whitespace-nowrap after:absolute after:top-0.875 after:-right-3.25 after:border-y-6 after:border-l-8 after:border-y-transparent',
  },
  wide: {
    button: 'col-span-full h-18 w-71.5',
    label:
      'after:border-l-enh-ink text-body text-enh-ink absolute top-1/2 right-8.5 left-[44%] -translate-y-1/2 text-center leading-tight font-black *:block after:absolute after:top-1/2 after:-right-5.5 after:-mt-1.5 after:border-y-6 after:border-l-8 after:border-y-transparent',
  },
} as const;
/** an Enhance button (it opens a dialog): `art` its picture, `size` the page's square one or the wide one */
export function EnhButton({
  id,
  art,
  size,
  label,
  name,
  badge,
  onClick,
}: {
  id: string;
  art: keyof typeof ENH_ART;
  size: keyof typeof ENH;
  label: ComponentChildren;
  /** its name for assistive tech, when the label's words are not */
  name?: string | undefined;
  badge?: Badge | undefined;
  onClick: (() => void) | undefined;
}) {
  return (
    <button
      class={cx(
        'group focus-visible:outline-gm-bright2 relative inline-grid rounded-xl focus-visible:outline-3 focus-visible:outline-offset-2 active:translate-y-0.5',
        ENH[size].button,
      )}
      type="button"
      id={id}
      aria-haspopup="dialog"
      aria-label={name}
      title={badge?.title}
      onClick={onClick}
    >
      <span
        class={cx(
          'drop-shadow-enh group-hover:drop-shadow-enh-glow absolute inset-0 group-hover:brightness-106',
          ENH_ART[art],
        )}
        aria-hidden="true"
      />
      <span class={ENH[size].label}>
        {label}
        {badge && <span class="text-caption block font-bold tabular-nums opacity-85">{badge.n}</span>}
      </span>
    </button>
  );
}
/** the Umas and Support cards buttons (ui/Setup.tsx: their dialogs and counts), at the page's size */
export function EnhButtons({
  page,
  umas,
  cards,
  onUmas,
  onCards,
}: {
  page: Page;
  umas?: Badge | undefined;
  cards?: Badge | undefined;
  onUmas?: (() => void) | undefined;
  onCards?: (() => void) | undefined;
}) {
  return (
    <>
      <EnhButton id="o-umas" art="umas" size={page} label="Umas" badge={umas} onClick={onUmas} />
      <EnhButton id="o-cards" art="cards" size={page} label="Support cards" badge={cards} onClick={onCards} />
    </>
  );
}

/** the "+" list's box beside a node's sparks (veterans/Editors.tsx AddList fills it and places it; hidden while shut) */
export const AddPop = ({
  hidden = true,
  boxRef,
  children,
}: {
  hidden?: boolean;
  boxRef?: Ref<HTMLDivElement>;
  children?: ComponentChildren;
}) => (
  <div
    ref={boxRef ?? null}
    class={popover({
      frame: 'game',
      class: 'text-ui absolute z-50 grid max-w-[300px] gap-1 p-2.5 focus:outline-none',
    })}
    id="add-pop"
    data-pop="add"
    role="dialog"
    aria-label="Add a spark"
    tabIndex={-1}
    hidden={hidden}
  >
    {children}
  </div>
);

/** the footer: the page's own button and the version the build stamps */
export function Footer({ version, children }: { version: string; children: ComponentChildren }) {
  return (
    <footer class="max-w-page text-note text-muted mx-auto mt-3.5 grid gap-1.5">
      <p class="flex max-w-[90ch] flex-wrap items-center gap-2">
        {children}
        <span class="text-gm-ink2" id="ver">{`Version ${version}`}</span>
      </p>
    </footer>
  );
}
