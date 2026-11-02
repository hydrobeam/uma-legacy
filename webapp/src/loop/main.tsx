// Legacy Loop Planner page: entry point, loop.html's script (Vite bundles it and the modules it imports).
// The page's own modules (loop/): types.ts the report's types, state.ts the run state (signals), run.ts the search's
// run machine, job.ts its job, io.ts the export's text; Layout.tsx the page's one root; the views, components drawn from
// the state and the settings: RunNote.tsx the status line, Results.tsx the result cards and the 1 or 3 switch,
// Affinity.tsx and Export.tsx their dialogs. Each is drawn in its place in the layout, inside a ui/RenderGuard.tsx (a
// dialog's guard is inside it) that reports a throw while drawing on the status line.
import { started } from '../dom/errorlog.ts'; // first: it listens before any other module runs
import { createRef, hydrate } from 'preact';
import { signal } from '@preact/signals';
import { ERA } from '../state/saved.ts';
import type { Era } from '../model/eras.ts';
import { browserStorage } from '../state/storage.ts';
import { loadData } from '../model/data.ts';
import EXAMPLE from '../../example.json'; // the default settings' result, precomputed (example.mjs)
import { DEFAULTS, fields, jobSetup, settings, setAuto, setPerUma } from '../state/settings.ts';
import { on } from '../state/store.ts';
import { must } from '../must.ts';
import { beginSearch, headline, interim, loopDrawn, loopRun, loopView, resultsStale, settle, tick } from './state.ts';
import { createRun } from './run.ts';
import { RunAct, RunNote, noteShown, noteTitle, pillText } from './RunNote.tsx';
import { RenderGuard, sayRenderErrors } from '../ui/RenderGuard.tsx';
import { restartEngine, run, warmEngine } from '../solver/engine.ts';
import { initScenario } from '../state/scenario.ts';
import { Results, ResultsCount, drawResult } from './Results.tsx';
import { AffinityDialog } from './Affinity.tsx';
import { ExportDialog, openExport } from './Export.tsx';
import { OddsDialog } from './OddsDialog.tsx';
import { ScheduleDialog } from '../results/Schedule.tsx';
import { initPopover } from '../results/popover.ts';
import { CardPop } from '../results/CardPop.tsx';
import { loadExample } from '../state/presets.ts';
import { openSetupDialog, pickEra, setupBadges, setupViews } from '../setup/Setup.tsx';
import { useHydrated } from '../setup/Shell.tsx';
import { LoopLayout } from './Layout.tsx';
import type { LoopState, LoopViews } from './Layout.tsx';
import '../dom/gmtip.ts'; // the game-style tooltips (both planner pages)

// ---------- the search (run.ts), with what it reaches outside itself
const loop = createRun({
  run: (job, interim) => run(job, interim),
  restart: restartEngine,
  now: () => performance.now(),
  setTimer: (fn, ms) => window.setTimeout(fn, ms),
  every: (fn, ms) => window.setInterval(fn, ms),
  clearTimer: (timer) => clearTimeout(timer),
  storage: browserStorage,
  store: { loopRun, beginSearch, interim, settle, tick },
  settings: () => settings.peek(),
  render: drawResult,
  loadExample,
  example: ERA === 'global' ? { r: EXAMPLE, settings: DEFAULTS } : null,
});
// a view that could not be drawn says so as a failed search does
sayRenderErrors((text) => settle({ kind: 'error', text }));

// ---------- the page: one root (Layout.tsx), the build's markup hydrated ----------
// the results column, which the result cards move the focus to when a redraw removed it (Results.tsx)
const column = createRef<HTMLElement>();
// the views, made once (the same vnodes on every draw of the root, so a redraw of the layout leaves them be); drawn
// once the game data is in (`ready`), the status line from the first status worth showing on (`statusShown`: the GM
// era's loading, a failed load, the first search or the kept result; the example's status, between showFirst and the
// first search, is never shown)
const NOTE = (
  <RenderGuard input={loopView}>
    <RunNote />
  </RenderGuard>
);
const ACT = (
  <RenderGuard input={loopView}>
    <RunAct stop={loop.stopSearch} />
  </RenderGuard>
);
const setup = setupViews({ page: 'loop', fadeMore: true });
const VIEWS: LoopViews = {
  scenario: setup.scenario,
  rank: setup.rank,
  targets: setup.targets,
  count: (
    <RenderGuard input={fields.decksTop} what="The Results switch">
      <ResultsCount />
    </RenderGuard>
  ),
  results: (
    <RenderGuard input={loopDrawn} what="The result" fallback={<p>The result could not be drawn.</p>}>
      <Results column={column} />
    </RenderGuard>
  ),
  cardPop: <CardPop home={null} />, // (its slot in the results column)
  dialogs: {
    'set-umas-root': setup.umasDialog,
    'set-cards-root': setup.cardsDialog,
    'why-root': <AffinityDialog />,
    'sched-root': <ScheduleDialog />,
    'ex-root': <ExportDialog />,
    'odds-root': <OddsDialog />,
  },
};
const ready = signal(false),
  statusShown = signal(false);
// phone: the columns stacked (the results column first: Layout.tsx)
const PHONE = matchMedia('(max-width: 960px)'),
  phone = signal(PHONE.matches);
PHONE.addEventListener('change', () => {
  phone.value = PHONE.matches;
});
/** The root: the build's markup on the first draw (the hydration), then the page. Around the views it draws what the
 *  status line marks: the status bar's state (its colours), the results' (busy, or dimmed: from other settings than
 *  these), the status line's own (busy, its tooltip) and the phone's floating button. */
function App() {
  const hydrated = useHydrated();
  if (!hydrated) return <LoopLayout version={__LLP_VERSION__} state={{ phone: phone.peek() }} />;
  const isReady = ready.value,
    noting = statusShown.value,
    s = settings.value;
  const state: LoopState = { era: ERA, phone: phone.value, auto: s.auto !== false, perUma: s.perUma !== false };
  if (noting) {
    const r = loopRun.value,
      view = loopView.value;
    Object.assign(state, {
      kind: r.searching ? 'busy' : view.status.kind,
      busy: r.searching,
      noteTitle: noteTitle(r),
      resultsStale: resultsStale.value && !r.searching,
      pill: noteShown(r, view.status) ? pillText(r, view.status, headline.value) : null,
    });
  }
  if (isReady)
    Object.assign(state, {
      ...setupBadges(),
      onEra: (era: Era) => pickEra(era),
      onUmas: () => openSetupDialog('umas'),
      onCards: () => openSetupDialog('cards'),
      // the switch changes no result: switching it on searches only changes made while it was off
      onAuto: (on: boolean) => {
        setAuto(on);
        if (settings.value.auto && loopView.value.status.kind === 'pending') loop.scheduleSearch();
      },
      onPerUma: setPerUma,
      onExport: openExport,
    });
  return (
    <LoopLayout
      version={__LLP_VERSION__}
      note={noting ? NOTE : null}
      act={noting ? ACT : null}
      views={isReady ? VIEWS : null}
      state={state}
      column={column}
    />
  );
}
const showStatus = () => {
  statusShown.value = true;
};
hydrate(<App />, must(document.getElementById('app'), 'loop.html has the root'));

// everything below needs the era's data (Global's from the page's data script, the GM era's fetched)
if (ERA === 'gm') {
  settle({ kind: 'loading', text: 'Loading the JP Grand Masters-era data…' });
  showStatus();
} // fetched: about a second
loadData(null).then(start, (e) => {
  settle({ kind: 'error', text: e.message });
  showStatus();
});
function start() {
  initScenario(); // the scenario picker (sets the training-hint rate; Unity Cup's golds follow it)
  initPopover();
  ready.value = true;
  // every change of setup from here on searches again (made after initScenario: its swap of a scenario this era lacks is
  // the setup the first search starts from); what the result on show no longer owns greys at once (Results.tsx)
  on([jobSetup], () => loop.scheduleSearch());
  // before the first search finishes: the remembered result of these settings, or the example for the defaults, or nothing
  const first = loop.showFirst();
  // the first search, with the saved settings (also warms the engine); none when the result on show is these settings'
  // finished search from before (it shows as done; the engine still warms for the next edit)
  if (first === 'kept') {
    void warmEngine();
    showStatus();
  } else
    setTimeout(() => {
      void loop.doSearch();
      showStatus();
    }, 0);
}
started(); // (the last statement: dom/errorlog.ts)
