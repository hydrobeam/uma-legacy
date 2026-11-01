// ---------- the loop page's one root: everything under the header (loop.html's #app) ----------
// Drawn at build time with no views and no state (vite/prerender.mjs: the page's first paint), and hydrated by the entry
// (main.tsx), which fills in `views` once the game data is in (the status line's two parts from the first status worth
// showing) and the state of what the layout draws itself: the status row's state and tooltip, the dimmed results, the
// switches, the phone's floating status button. Imports nothing but Preact, the shell and its class helpers
// (ui/Shell.tsx says why). The results column: on a phone (the columns stacked) it comes first, before the setup, in
// the reading and Tab order too (`phone`; loop.html's script after #app puts it there before the scripts run, so the
// first paint has it); on a desktop it scrolls with the page beside the setup.
import type { ComponentChildren, RefObject } from 'preact';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { cx } from 'tailwind-variants/lite';
import { EnhButtons, EraRow, Footer, Ribbon, setRow } from '../ui/Shell.tsx';
import { panel } from '../ui/Panel.tsx';
import type { Badge } from '../ui/Shell.tsx';
import type { Era } from '../eras.ts';
import { Button } from '../ui/Button.tsx';
import { Switch } from '../ui/Switch.tsx';

/** the dialogs' boxes at the end of the page, in order */
const LOOP_DIALOGS = ['set-umas-root', 'set-cards-root', 'why-root', 'sched-root', 'ex-root', 'odds-root'] as const;
/** the views, each drawn where the page's markup holds it */
export interface LoopViews {
  scenario: ComponentChildren;
  rank: ComponentChildren;
  targets: ComponentChildren;
  /** the Results switch above the results, the result cards, the deck card panel's slot */
  count: ComponentChildren;
  results: ComponentChildren;
  cardPop: ComponentChildren;
  /** the dialogs, each in its own box at the end of the page */
  dialogs: Record<(typeof LOOP_DIALOGS)[number], ComponentChildren>;
}
/** what the layout draws itself, from the stores (the entry's App); absent at build time and while hydrating */
export interface LoopState {
  era?: Era;
  umas?: Badge;
  cards?: Badge;
  /** the status row's state (data-kind: an error's line semibold) and the status line's busy mark and tooltip */
  kind?: string;
  busy?: boolean;
  noteTitle?: string;
  resultsStale?: boolean;
  /** the floating status button's text (phones; null: hidden) */
  pill?: string | null;
  auto?: boolean;
  perUma?: boolean;
  /** the columns stacked (phones): the results column first */
  phone?: boolean;
  onEra?: (era: Era) => void;
  onUmas?: () => void;
  onCards?: () => void;
  onAuto?: (on: boolean) => void;
  onPerUma?: (on: boolean) => void;
  onExport?: () => void;
}
interface Props {
  version: string;
  /** the status line's text and its button (drawn from the first status worth showing) */
  note?: ComponentChildren;
  act?: ComponentChildren;
  views?: LoopViews | null | undefined;
  state?: LoopState;
  /** the results column, for the views that reach it (the entry's; one of its own at build time) */
  column?: RefObject<HTMLElement | null>;
}
export function LoopLayout({ version, note = null, act = null, views = null, state: s = {}, column }: Props) {
  const own = useRef<HTMLElement>(null),
    rcol = column ?? own;
  // whether the results column is in view (html.rin: the floating status button hides)
  useLayoutEffect(() => {
    const box = rcol.current;
    if (!box || !window.IntersectionObserver) return;
    const seen = new IntersectionObserver((entries) => {
      // (#rcol's changes since the last call, the newest last)
      for (const entry of entries) document.documentElement.classList.toggle('rin', entry.isIntersecting);
    });
    seen.observe(box);
    return () => seen.disconnect();
  }, [rcol]);
  // the floating status button takes you back up to the results
  const toResults = () => {
    const box = rcol.current;
    if (!box) return;
    box.scrollIntoView({
      block: 'start',
      behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
    });
    box.focus({ preventScroll: true });
  };
  const side = (
    <div class="grid min-w-0 content-start gap-3.5" key="side">
      {/* the setup panel: its rows one under another, the Enhance buttons under them, the status row at its foot */}
      <section class={cx(panel, 'grid gap-y-2 self-start px-4.5 pb-3 max-sm:px-3')} aria-labelledby="h-set">
        <Ribbon>Loop setup</Ribbon>
        <div class="relative grid min-w-0 grid-cols-1 content-start gap-1.5">
          <EraRow era={s.era} page="loop" onPick={s.onEra} />
          {/* (its menu hangs under the whole list) */}
          <div class="contents" id="scen-dd">
            {views?.scenario}
          </div>
          <div class={cx(setRow.loop, 'relative')} id="rank-ctl">
            {views?.rank}
          </div>
        </div>
        <div class="grid grid-cols-[auto_auto] justify-center gap-x-3 gap-y-2 self-center pt-1.5">
          <EnhButtons page="loop" umas={s.umas} cards={s.cards} onUmas={s.onUmas} onCards={s.onCards} />
        </div>
        {/* the status line in small type over the switches; its colours stay the panel's (data-kind: an error's line
            semibold) */}
        <div
          class="group border-gm-line bg-gm-pane text-caption text-gm-ink mt-0.5 flex flex-wrap items-center gap-x-3.5 gap-y-1.5 rounded-lg border px-3 pt-2 pb-2.25"
          id="rhead"
          data-kind={s.kind}
        >
          {/* (a runner before the words: the words wrap beside it, never under it) */}
          <p
            class="text-ink leading-title m-0 flex grow basis-full flex-wrap items-center gap-x-2 gap-y-0.5 font-medium tabular-nums group-data-[kind=error]:font-semibold empty:hidden has-[>[aria-hidden]:first-child]:flex-nowrap has-[>[aria-hidden]:first-child]:items-start"
            id="run-note"
            role="status"
            aria-live="polite"
            title={s.noteTitle}
            data-busy={s.busy ? '' : undefined}
          >
            {note}
          </p>
          <span class="empty:hidden" id="r-act">
            {act}
          </span>
          <Switch
            size="sm"
            title="Automatically search 0.55s after a change"
            id="x-auto"
            checked={s.auto ?? true}
            onChange={s.onAuto && ((e) => s.onAuto?.(e.currentTarget.checked))}
          >
            Auto-search
          </Switch>
          <Switch
            size="sm"
            title="Each uma gets the best deck for her own outfit and targets: better odds than one deck for the whole loop"
            id="c-peruma"
            checked={s.perUma ?? true}
            onChange={s.onPerUma && ((e) => s.onPerUma?.(e.currentTarget.checked))}
          >
            Deck per uma
          </Switch>
        </div>
      </section>
      {/* Target skills in the open, under a green bar */}
      <section
        class={cx(panel, 'grid min-w-0 grid-cols-1 gap-3 overflow-hidden px-3 pb-3')}
        id="skills-panel"
        aria-labelledby="h-skills"
      >
        {views?.targets}
      </section>
    </div>
  );
  const results = (
    <main
      class="relative grid min-w-0 grid-cols-1 content-start gap-3 outline-none"
      id="rcol"
      tabIndex={-1}
      aria-label="Results"
      ref={rcol}
      key="results"
    >
      {/* Results (1 or 3), in the first result's top right corner (on a phone its label for the screen reader only) */}
      <span
        class="text-note text-gm-ink2 absolute top-3 right-3.5 z-2 m-0 inline-flex items-center gap-1.5 font-bold max-md:top-2.5 max-md:right-2.5 max-md:*:first:sr-only"
        id="rtop"
      >
        {views?.count}
      </span>
      {/* dimmed while a search runs (after a moment, so a quick one doesn't flicker) and while not these settings' */}
      <div
        class="grid min-w-0 gap-2.5 transition-opacity aria-busy:delay-800 aria-busy:duration-250 aria-busy:not-data-stale:opacity-50 data-stale:opacity-45 motion-reduce:transition-none"
        id="results"
        data-stale={s.resultsStale ? '' : undefined}
        {...(s.busy === undefined ? {} : { 'aria-busy': s.busy })}
      >
        {views?.results}
      </div>
      <div id="pop-root">{views?.cardPop}</div>
    </main>
  );
  return (
    <>
      {/* the page: the setup column beside the results; stacked on a phone, the results first */}
      <div
        class="max-w-page font-game text-gm-ink mx-auto grid grid-cols-[minmax(0,400px)_minmax(0,1fr)] items-start gap-4.5 max-xl:grid-cols-1"
        data-page="loop"
      >
        {s.phone ? [results, side] : [side, results]}
        {/* phone: the status line's copy floats at the foot of the screen, a button back to the results (hidden while
            they are in view) */}
        <button
          class="border-accent bg-panel text-body text-ink shadow-pop fixed inset-x-4 bottom-[calc(env(safe-area-inset-bottom,0px)+12px)] z-30 hidden min-h-11 items-center gap-2 rounded-lg border px-3.5 py-2 text-left font-semibold tabular-nums max-xl:not-in-[.rin]:flex"
          id="run-pill"
          type="button"
          hidden={s.pill == null}
          onClick={toResults}
        >
          {s.pill ?? ''}
        </button>
      </div>
      {LOOP_DIALOGS.map((id) => (
        <div key={id} id={id}>
          {views?.dialogs[id]}
        </div>
      ))}
      <Footer version={version}>
        <Button id="ex-open" onClick={s.onExport}>
          Export / import settings
        </Button>
      </Footer>
    </>
  );
}
