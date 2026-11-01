// ---------- the loop page's one root: everything under the header (loop.html's #app) ----------
// Drawn at build time with no views and no state (vite/prerender.mjs: the page's first paint), and hydrated by the entry
// (main.tsx), which fills in `views` once the game data is in (the status line's two parts from the first status worth
// showing) and the state of what the layout draws itself: the status bar's colours and tooltip, the dimmed results, the
// switches, the phone's floating status button. Imports nothing but Preact and the shell (ui/Shell.tsx says why).
// The results column: on a phone (the columns stacked) it comes first, before the setup, in the reading and Tab order
// too (`phone`; loop.html's script after #app puts it there before the scripts run, so the first paint has it); on
// a desktop it sticks by its top when it fits the window, by its bottom when it is taller.
import type { ComponentChildren, RefObject } from 'preact';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { EnhButtons, EraRow, Footer } from '../ui/Shell.tsx';
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
  /** the status bar's state (its colours) and the status line's busy mark and tooltip */
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
const MARGIN = 12; // px kept above the sticky results column (and below it, when it is taller than the window)
export function LoopLayout({ version, note = null, act = null, views = null, state: s = {}, column }: Props) {
  const own = useRef<HTMLElement>(null),
    rcol = column ?? own;
  // the column's sticky offset, and whether it is in view (html.rin: the floating status button hides)
  useLayoutEffect(() => {
    const box = rcol.current;
    if (!box) return;
    const fit = () => box.style.setProperty('--rtop', `${Math.min(MARGIN, innerHeight - box.offsetHeight - MARGIN)}px`);
    const sized = new ResizeObserver(fit);
    sized.observe(box);
    addEventListener('resize', fit);
    const seen = window.IntersectionObserver
      ? new IntersectionObserver((entries) => {
          // (#rcol's changes since the last call, the newest last)
          for (const entry of entries) document.documentElement.classList.toggle('rin', entry.isIntersecting);
        })
      : null;
    seen?.observe(box);
    return () => {
      sized.disconnect();
      seen?.disconnect();
      removeEventListener('resize', fit);
    };
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
    <div class="lpside" key="side">
      <section class="rbbar" aria-labelledby="h-set">
        <h2 class="sethdr" id="h-set">
          Loop setup
        </h2>
        <div class="setlist">
          <EraRow era={s.era} compact onPick={s.onEra} />
          <div class="scendd" id="scen-dd">
            {views?.scenario}
          </div>
          <div class="gctl rankctl" id="rank-ctl">
            {views?.rank}
          </div>
        </div>
        <div class="setenh">
          <EnhButtons umas={s.umas} cards={s.cards} onUmas={s.onUmas} onCards={s.onCards} />
        </div>
        <div class="rhead" id="rhead" data-kind={s.kind}>
          <p
            class={s.busy ? 'runnote busy' : 'runnote'}
            id="run-note"
            role="status"
            aria-live="polite"
            title={s.noteTitle}
          >
            {note}
          </p>
          <span class="ract" id="r-act">
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
      <div class="lptgt">
        <section class="panel" id="skills-panel" aria-labelledby="h-skills">
          {views?.targets}
        </section>
      </div>
    </div>
  );
  const results = (
    <main class="rcol" id="rcol" tabIndex={-1} aria-label="Results" ref={rcol} key="results">
      <span class="rtop" id="rtop">
        {views?.count}
      </span>
      <div
        class={s.resultsStale ? 'results stale' : 'results'}
        id="results"
        {...(s.busy === undefined ? {} : { 'aria-busy': s.busy })}
      >
        {views?.results}
      </div>
      <div id="pop-root">{views?.cardPop}</div>
    </main>
  );
  return (
    <>
      <div class="wrap lp">
        {s.phone ? [results, side] : [side, results]}
        <button class="runpill" id="run-pill" type="button" hidden={s.pill == null} onClick={toResults}>
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
