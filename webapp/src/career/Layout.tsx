// ---------- the career page's one root: everything under the header (index.html's #app) ----------
// Drawn at build time with no views and no state (vite/prerender.mjs: the page's first paint), and hydrated by the entry
// (main.tsx), which fills in `views` (each view behind its guard) once the game data is in, and the state of what the
// layout draws itself: the setup bar's fold and summary, the status bar's colours, the dimmed results. Imports nothing
// but Preact and the shell (ui/Shell.tsx says why).
import type { ComponentChildren } from 'preact';
import { AddPop, EnhButtons, EraRow, Footer } from '../ui/Shell.tsx';
import type { Badge } from '../ui/Shell.tsx';
import type { Era } from '../eras.ts';
import { Button } from '../ui/Button.tsx';

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
  /** the setup bar's editors (ui/Setup.tsx) and the Borrowed parent choice */
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
export function CareerLayout({ version, note = null, views = null, state: s = {} }: Props) {
  const folded = !!s.folded;
  return (
    <>
      <div class="rb">
        {/* biome-ignore lint/a11y/useKeyWithClickEvents: the ribbon's button (#set-fold) is the keyboard's way; folded, a click anywhere on the bar is the pointer's shortcut */}
        <section class={folded ? 'rbbar folded' : 'rbbar'} aria-labelledby="h-set" onClick={s.onBar}>
          <h2 class="sethdr" id="h-set">
            <button
              type="button"
              class="setfold"
              id="set-fold"
              aria-expanded={folded ? 'false' : 'true'}
              aria-controls="set-body"
              title={s.folded === undefined ? undefined : folded ? 'Show the career setup' : 'Fold the career setup'}
            >
              Career setup
              <span class="foldi" aria-hidden="true" />
            </button>
          </h2>
          <span class="setsum" id="set-sum" role="note" hidden={!folded} aria-label={s.summaryLabel}>
            {views?.summary}
          </span>
          <div class="setlist">
            <EraRow era={s.era} onPick={s.onEra} />
            <div class="gctl rankctl" id="rank-ctl">
              {views?.rank}
            </div>
            <div class="scendd" id="scen-dd">
              {views?.scenario}
            </div>
            <div class="gctl setrow bpctl">
              <span class="glab" id="bp-l">
                Borrowed parent
              </span>
              {/* biome-ignore lint/a11y/useSemanticElements: a segmented control drawn as the game's pills; a fieldset's border and legend would fight that art */}
              <div class="seg" id="bp" role="group" aria-labelledby="bp-l">
                {views?.borrow}
              </div>
            </div>
          </div>
          <div class="setenh">
            <EnhButtons umas={s.umas} cards={s.cards} onUmas={s.onUmas} onCards={s.onCards} />
            <button
              class="enh wide"
              type="button"
              id="v-toggle"
              aria-haspopup="dialog"
              aria-label="Your veterans"
              onClick={s.onVeterans}
            >
              <span class="enhimg" aria-hidden="true" />
              <span class="enhl">
                <span>Veteran</span> <span>Umamusume</span>
              </span>
            </button>
          </div>
          <div class="lptgt">
            <section class="panel" id="skills-panel" aria-labelledby="h-skills">
              {views?.targets}
            </section>
          </div>
        </section>

        <main class="rbmain" id="rcol" tabIndex={-1} aria-labelledby="h-lin">
          <div class="rbtitle">
            <h2 id="h-lin">Next career</h2>
            <div class="lodds" id="lodds">
              {views?.verdict}
            </div>
            <span id="um-root">{views?.umamoe}</span>
            <div class="rhead" id="rhead" data-kind={s.kind ?? 'loading'}>
              <span class="runnote" id="run-note" role="status" aria-live="polite">
                {note}
              </span>
            </div>
          </div>
          <div class={s.treeStale ? 'lineup stale' : 'lineup'} id="tree">
            {views?.tree}
          </div>
          <div
            id="results"
            class={s.resultsStale ? 'stale' : undefined}
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
