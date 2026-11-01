// Legacy Loop Planner page: entry point. build.py bundles this and the modules it imports (esbuild, one IIFE) into
// dist/assets/main.<hash>.js, which dist/loop.html loads after its data script.
// The page's own modules (loop/): types.ts the report's types, state.ts the run state (signals), run.ts the search's
// run machine, job.ts its job, io.js the export's text; the views, components drawn from the state and the settings:
// RunNote.tsx the status line, Results.tsx the result cards and the 1 or 3 switch, Affinity.tsx and Export.tsx their
// dialogs. Each is rendered into its mount point here (#run-note, #r-act, #results, #rtop, #why-root, #ex-root), inside a
// ui/RenderGuard.tsx (a dialog's guard is inside it) that reports a throw while drawing on the status line.
import '../errorlog.ts'; // first: it listens before any other module runs
import { render } from 'preact';
import { $, $dialog, $input, backdropCloses } from '../dom.js';
import { ERA } from '../saved.ts';
import { EXAMPLE, loadData } from '../data.js';
import { DEFAULTS, fields, jobSetup, settings, setAuto, switchEra } from '../settings.ts';
import { on } from '../store.ts';
import { headline, loopDrawn, loopRun, loopView, resultsStale, settle } from './state.ts';
import type { LoopView, RunState } from './state.ts';
import type { DeepReadonly } from '../store.ts';
import { createRun } from './run.ts';
import { RunAct, RunNote, noteShown, noteTitle, pillText } from './RunNote.tsx';
import { RenderGuard, sayRenderErrors } from '../ui/RenderGuard.tsx';
import { restartEngine, run, warmEngine } from '../engine.ts';
import { initRoster, renderRoster } from '../roster.js';
import { initTargets, renderTargets } from '../targets.js';
import { initScenario, eraScenario } from '../scenario.js';
import { initCards } from '../cards.js';
import { initOwnedImport } from '../owned.js';
import { Results, ResultsCount, drawResult } from './Results.tsx';
import { AffinityDialog } from './Affinity.tsx';
import { ExportDialog, openExport } from './Export.tsx';
import { OddsDialog } from '../ui/OddsDialog.tsx';
import { initPopover } from '../popover.js';
import { loadExample } from '../presets.js';
import { initSetup } from '../setup.js';
import '../gmtip.js'; // the game-style tooltips (both planner pages)

// ---------- results column (desktop): sticky by its top when it fits the window, by its bottom when it is taller ----------
{
  const column = document.querySelector<HTMLElement>('.rcol');
  if (column) {
    const MARGIN = 12; // px kept above it (and below it, when it is taller than the window)
    const fit = () =>
      column.style.setProperty('--rtop', `${Math.min(MARGIN, innerHeight - column.offsetHeight - MARGIN)}px`);
    new ResizeObserver(fit).observe(column);
    addEventListener('resize', fit);
  }
}
// ---------- game data ----------
$('era-global').setAttribute('aria-pressed', String(ERA === 'global'));
$('era-gm').setAttribute('aria-pressed', String(ERA === 'gm'));
const setEra = (era: 'global' | 'gm') => {
  if (era === ERA) return;
  switchEra(era, eraScenario(era));
  location.reload();
};
$('era-global').onclick = () => setEra('global');
$('era-gm').onclick = () => setEra('gm');
// ---------- the search (run.ts), with what it reaches outside itself
const loop = createRun({
  run: (job, interim) => run(job, interim),
  restart: restartEngine,
  now: () => performance.now(),
  setTimer: (fn, ms) => setTimeout(fn, ms),
  every: (fn, ms) => setInterval(fn, ms),
  clearTimer: (timer) => clearTimeout(timer),
  storage: () => localStorage,
  settings: () => settings.peek(),
  render: drawResult,
  loadExample,
  example: ERA === 'global' && EXAMPLE ? { r: EXAMPLE, settings: DEFAULTS } : null,
});
// ---------- the status line (RunNote.tsx), and what it marks around it: the status bar's state (its colours), the
// results' (busy or dimmed: from other settings than these), the status line's own (busy, its tooltip) and the phone's
// floating button. The status line (the template's live region, empty from load) is drawn from the first
// status worth showing on (the GM era's loading, a failed load, the first search or the kept result): the example's
// status, between showFirst and the first search, is never shown.
// a view that could not be drawn says so as a failed search does
sayRenderErrors((text) => settle({ kind: 'error', text }));
function marks(run: DeepReadonly<RunState>, view: DeepReadonly<LoopView>) {
  $('rhead').dataset.kind = run.searching ? 'busy' : view.status.kind;
  const note = $('run-note');
  note.classList.toggle('busy', run.searching);
  note.title = noteTitle(run);
  const results = $('results');
  results.setAttribute('aria-busy', String(run.searching));
  results.classList.toggle('stale', resultsStale.peek() && !run.searching);
  const pill = $('run-pill');
  pill.hidden = !noteShown(run, view.status);
  pill.textContent = pillText(run, view.status, headline.peek());
}
let statusShown = false;
function showStatus() {
  render(
    <RenderGuard input={loopView}>
      <RunNote />
    </RenderGuard>,
    $('run-note'),
  );
  render(
    <RenderGuard input={loopView}>
      <RunAct stop={loop.stopSearch} />
    </RenderGuard>,
    $('r-act'),
  );
  marks(loopRun.peek(), loopView.peek());
  if (!statusShown) on([loopRun, loopView], marks);
  statusShown = true;
}
// everything below needs the era's data (Global's from the page's data script, the GM era's fetched)
if (ERA === 'gm') {
  settle({ kind: 'loading', text: 'Loading the JP Grand Masters-era data…' });
  showStatus();
} // fetched: about a second
loadData().then(start, (e) => {
  settle({ kind: 'error', text: e.message });
  showStatus();
});
function start() {
  $input('x-auto').checked = settings.value.auto !== false;
  $input('x-auto').onchange = () => {
    setAuto($input('x-auto').checked);
    if (settings.value.auto && loopView.value.status.kind === 'pending') loop.scheduleSearch();
  }; // the switch changes no result: switching it on searches only changes made while it was off
  initScenario(); // the scenario picker (sets the training-hint rate; Unity Cup's golds follow it)

  initRoster();
  initCards();
  render(
    <RenderGuard input={fields.decksTop} what="The Results switch">
      <ResultsCount />
    </RenderGuard>,
    $('rtop'),
  );
  initOwnedImport();
  initTargets();
  render(<AffinityDialog />, $('why-root'));
  render(<ExportDialog />, $('ex-root'));
  // the odds popup: its own mount point at the end of <body> (the template has none)
  render(<OddsDialog />, document.body.appendChild(Object.assign(document.createElement('div'), { id: 'odds-root' })));
  $('ex-open').onclick = openExport;
  initPopover({ drawn: true });
  initSetup();
  // every change of setup from here on searches again (made after initScenario: its swap of a scenario this era lacks is
  // the setup the first search starts from); what the result on show no longer owns greys at once (Results.tsx)
  on([jobSetup], () => loop.scheduleSearch());
  // phone: the floating status button shows while the results are out of view, and takes you back up to them
  if (window.IntersectionObserver)
    new IntersectionObserver((entries) =>
      document.documentElement.classList.toggle('rin', entries[0].isIntersecting),
    ).observe($('rcol'));
  $('run-pill').onclick = () => {
    const column = $('rcol');
    column.scrollIntoView({
      block: 'start',
      behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
    });
    column.focus({ preventScroll: true });
  };
  // the roster and card boxes: .more while there is more to scroll to below (their bottom edge fades)
  for (const id of ['roster', 'cards']) {
    const box = $(id),
      update = () => box.classList.toggle('more', box.scrollTop + box.clientHeight < box.scrollHeight - 4); // 4 px: rounding slack
    box.addEventListener('scroll', update, { passive: true });
    new MutationObserver(update).observe(box, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['hidden'],
    }); // the filter hides tiles
    if (window.ResizeObserver) new ResizeObserver(update).observe(box); // also when its dialog opens (its size goes from 0)
  }
  backdropCloses($dialog('sched-dlg'));

  renderRoster();
  renderTargets(); // renderTargets draws the card picker too (it lists the cards that give a target)
  render(
    <RenderGuard input={loopDrawn} what="The result" fallback={<p>The result could not be drawn.</p>}>
      <Results />
    </RenderGuard>,
    $('results'),
  );
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
