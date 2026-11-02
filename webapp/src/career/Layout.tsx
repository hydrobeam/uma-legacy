// ---------- the career page's one root: everything under the header (index.html's #app) ----------
// Drawn at build time with no views and no state (vite/prerender.mjs: the page's first paint), and hydrated by the entry
// (main.tsx), which fills in `views` (each view behind its guard) once the game data is in, and the state of what the
// layout draws itself: the setup bar's fold and summary, the status bar's colours, the dimmed results. Imports nothing
// but Preact, the shell and its class helpers (setup/Shell.tsx says why).
import type { ComponentChildren } from 'preact';
import { cx } from 'tailwind-variants/lite';
import { AddPop, EnhButtons, EraRow, Footer, Ribbon, setLabel, setRow } from '../setup/Shell.tsx';
import { EnhButton } from '../ui/Button.tsx';
import type { Badge } from '../ui/Button.tsx';
import type { Era } from '../model/eras.ts';
import { Button } from '../ui/Button.tsx';
import { panel } from '../ui/Panel.tsx';

/** the dialogs' and popups' boxes at the end of the page, in order */
const CAREER_DIALOGS = [
  'vets-root',
  'um-dlg-root',
  'sim-root',
  'set-umas-root',
  'set-cards-root',
  'vpick-root',
  'node-root',
  'deck-root',
  'skill-root',
  'save-root',
  'sched-root',
  'vet-editors',
  'paste-root',
  'vx-root',
] as const;
/** the views, each drawn where the page's markup holds it */
export interface CareerViews {
  /** the setup bar's editors (setup/Setup.tsx) and the Borrowed parent choice */
  rank: ComponentChildren;
  scenario: ComponentChildren;
  borrow: ComponentChildren;
  targets: ComponentChildren;
  /** the folded setup's one-line summary */
  summary: ComponentChildren;
  /** the odds and Try a run beside the title, uma.moe's button, the lineage tree, the other careers, the deck card
   *  panel's slot in the results column */
  verdict: ComponentChildren;
  umamoe: ComponentChildren;
  tree: ComponentChildren;
  results: ComponentChildren;
  cardPop: ComponentChildren;
  /** the dialogs and popups, each in its own box at the end of the page */
  dialogs: Record<(typeof CAREER_DIALOGS)[number], ComponentChildren>;
  /** the "+" list beside a node's sparks (veterans/Editors.tsx AddList: an AddPop, here or in the open dialog) */
  addList: ComponentChildren;
}
/** what the layout draws itself, from the stores (the entry's App); absent at build time and while hydrating */
export interface CareerState {
  era?: Era;
  umas?: Badge;
  cards?: Badge;
  folded?: boolean;
  /** the folded summary, whole, for the screen reader */
  summaryLabel?: string;
  /** the status bar's state (its colours): a run at work, or its answer's status */
  kind?: string;
  /** the results dimmed: the answer on show is not this setup's */
  resultsStale?: boolean;
  treeStale?: boolean;
  busy?: boolean;
  onEra?: (era: Era) => void;
  onUmas?: () => void;
  onCards?: () => void;
  /** a click on the setup bar (its ribbon unfolds or folds it; folded, the whole bar unfolds it) */
  onBar?: (e: MouseEvent) => void;
  onVeterans?: () => void;
  onExport?: () => void;
}
interface Props {
  version: string;
  /** the status line (drawn from the first status worth showing) */
  note?: ComponentChildren;
  views?: CareerViews | null | undefined;
  state?: CareerState;
}
// the setup panel: the ribbon, the settings, the Enhance buttons and Target skills side by side from 1180 px (one
// column under that); folded, the ribbon and a one-line summary
const SETUP = cx(
  panel,
  "grid grid-cols-[max-content_max-content_minmax(0,1fr)] grid-rows-[auto_1fr] items-start gap-x-6 gap-y-2 px-4.5 pb-3 [grid-template-areas:'hdr_enh_tgt'_'list_enh_tgt'] max-2xl:grid-cols-1 max-2xl:grid-rows-none max-2xl:[grid-template-areas:'hdr'_'list'_'enh'_'tgt'] max-sm:px-3",
);
const FOLDED = cx(panel, 'flex cursor-pointer items-center gap-3.5 px-4.5 max-sm:px-3');
export function CareerLayout({ version, note = null, views = null, state: s = {} }: Props) {
  const folded = !!s.folded;
  return (
    <>
      {/* the page: the setup panel over the next career (on a phone, under it) */}
      <div class="max-w-page font-game text-gm-ink mx-auto grid gap-6.5" data-page="career">
        {/* biome-ignore lint/a11y/useKeyWithClickEvents: the ribbon's button (#set-fold) is the keyboard's way; folded, a click anywhere on the bar is the pointer's shortcut */}
        <section class={folded ? FOLDED : SETUP} aria-labelledby="h-set" onClick={s.onBar}>
          <Ribbon fold>
            <button
              type="button"
              class="text-shadow-ribbon inline-flex cursor-pointer items-center gap-2.5 rounded-sm focus-visible:outline-3 focus-visible:outline-offset-2 focus-visible:outline-white"
              id="set-fold"
              aria-expanded={folded ? 'false' : 'true'}
              aria-controls="set-body"
              title={s.folded === undefined ? undefined : folded ? 'Show the career setup' : 'Fold the career setup'}
            >
              Career setup
              {/* the chevron: up while open, down while folded */}
              <span
                class={cx(
                  'drop-shadow-ribbon size-2 border-r-3 border-b-3 border-white',
                  folded ? '-translate-y-0.75 rotate-45' : 'translate-y-0.75 -rotate-135',
                )}
                aria-hidden="true"
              />
            </button>
          </Ribbon>
          <span
            class="text-ui text-gm-ink2 inline-flex flex-wrap items-baseline gap-x-5.5 gap-y-1 font-bold"
            id="set-sum"
            role="note"
            hidden={!folded}
            aria-label={s.summaryLabel}
          >
            {views?.summary}
          </span>
          {/* what folding hides: its parts are the panel's own grid's */}
          <div class="contents" id="set-body" hidden={folded}>
            {/* the settings, spread down the panel's height (Target skills sets it) */}
            <div class="relative grid min-w-0 grid-cols-[max-content] content-evenly gap-y-1.5 self-stretch pb-1.5 [grid-area:list] max-md:grid-cols-1">
              <EraRow era={s.era} page="career" onPick={s.onEra} />
              <div class={cx(setRow.career, 'relative')} id="rank-ctl">
                {views?.rank}
              </div>
              {/* (its menu hangs under the whole list) */}
              <div class="contents" id="scen-dd">
                {views?.scenario}
              </div>
              {/* on a narrow phone its three choices share the row's width under the label */}
              <div class={cx(setRow.career, 'max-sm:*:data-seg:col-span-full')}>
                <span class={setLabel.stacked} id="bp-l">
                  Borrowed parent
                </span>
                {views?.borrow}
              </div>
            </div>
            {/* Umas and Support cards, Veteran Umamusume under them as wide as the two, together at the top */}
            <div class="grid grid-cols-[auto_auto] content-start justify-start gap-1.5 self-start pt-2.5 pb-1.5 [grid-area:enh]">
              <EnhButtons page="career" umas={s.umas} cards={s.cards} onUmas={s.onUmas} onCards={s.onCards} />
              <EnhButton
                id="v-toggle"
                art="vets"
                size="wide"
                name="Your veterans"
                label={
                  <>
                    <span>Veteran</span> <span>Umamusume</span>
                  </>
                }
                onClick={s.onVeterans}
              />
            </div>
            {/* Target skills in the open: its title bar, the add box and Presets on one row (the add box on its own
                row on a phone), the tiles under them */}
            <section
              class="bg-gm-panel border-gm-edge mt-2.5 grid min-w-0 grid-cols-[max-content_minmax(0,1fr)_max-content] items-center gap-2.5 self-start rounded-2xl border px-3 pb-3 [grid-area:tgt] [grid-template-areas:'h_add_btn'_'list_list_list'] max-md:grid-cols-[minmax(0,1fr)_max-content] max-md:[grid-template-areas:'h_btn'_'add_add'_'list_list']"
              id="skills-panel"
              aria-labelledby="h-skills"
            >
              {views?.targets}
            </section>
          </div>
        </section>

        <main class="grid min-w-0 gap-2.5 outline-none max-lg:-order-1" id="rcol" tabIndex={-1} aria-labelledby="h-lin">
          <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
            <h2
              class="before:art-bright before:inset-shadow-knob text-headline text-gm-ink flex items-center gap-2.5 font-black before:h-[1.1em] before:w-2 before:rounded-sm"
              id="h-lin"
            >
              Next career
            </h2>
            {/* the odds and Try a run: the title row's own items */}
            <div class="contents empty:hidden" id="lodds">
              {views?.verdict}
            </div>
            <span class="contents" id="um-root">
              {views?.umamoe}
            </span>
            {/* the status: the game's brown words, no box; only an error gets a frame (the game's pink-edged note), and
                a finished run's line is read out, not shown */}
            <div
              class="text-body text-gm-ink2 data-[kind=error]:bg-gm-cell data-[kind=error]:border-gm-pink2 data-[kind=error]:text-gm-ink flex flex-[1_1_24em] flex-wrap items-center gap-x-3 gap-y-1.5 px-1 py-0.5 font-bold data-[kind=done]:sr-only data-[kind=error]:rounded-full data-[kind=error]:border-2 data-[kind=error]:px-3.5 data-[kind=error]:py-1.25"
              id="rhead"
              data-kind={s.kind ?? 'loading'}
            >
              <span
                class="flex flex-[1_1_18em] flex-wrap items-center gap-x-2 gap-y-0.5 tabular-nums empty:hidden"
                id="run-note"
                role="status"
                aria-live="polite"
              >
                {note}
              </span>
            </div>
          </div>
          {/* the next career as the player sets it up in the game: the trainee over her legacies (in the first column
              with or without targets), her targets beside them from 760 px (one column under that) */}
          <div
            class="mt-1 grid min-w-0 grid-cols-1 items-start gap-x-4.5 gap-y-4 lg:grid-cols-[minmax(280px,0.8fr)_minmax(0,1.7fr)] lg:grid-rows-[auto_1fr] lg:*:col-start-1 lg:*:nth-3:col-start-2 lg:*:nth-3:row-span-2 lg:*:nth-3:row-start-1"
            id="tree"
            data-stale={s.treeStale ? '' : undefined}
          >
            {views?.tree}
          </div>
          {/* dimmed while a run works (after a moment, so a quick one doesn't flicker) and while it is not this setup's */}
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
      </div>

      {CAREER_DIALOGS.map((id) => (
        <div key={id} id={id}>
          {views?.dialogs[id]}
        </div>
      ))}
      {views ? views.addList : <AddPop />}

      <Footer version={version}>
        <Button
          id="v-export"
          title="Your veterans, setup and the planner's search as text to copy or download (bring it back with Paste in Veteran Umamusume)"
          onClick={s.onExport}
        >
          Export
        </Button>
      </Footer>
    </>
  );
}
