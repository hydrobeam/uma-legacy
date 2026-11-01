// Legacy Loop Planner page: entry point. build.py bundles this and the modules it imports (esbuild, one IIFE) into
// dist/assets/main.<hash>.js, which dist/loop.html loads after its data script.
// The page's own modules (loop/): state.ts the run state (signals), run.ts the search's run machine, job.ts its job,
// runnote.js the status line, results.js the result cards, affinity.js the affinity dialog, io.js Export and Import.
import '../errorlog.ts'; // first: it listens before any other module runs
import { $, $dialog, $input, backdropCloses } from '../dom.js';
import { ERA } from '../saved.ts';
import { EXAMPLE, loadData } from '../data.js';
import { DEFAULTS, jobSetup, settings, setAuto, switchEra } from '../settings.ts';
import { on } from '../store.ts';
import { loopRun, loopView, settle } from './state.ts';
import { createRun } from './run.ts';
import { paintRunNote } from './runnote.js';
import { restartEngine, run, warmEngine } from '../engine.ts';
import { initRoster, renderRoster } from '../roster.js';
import { initTargets, renderTargets } from '../targets.js';
import { initScenario, eraScenario } from '../scenario.js';
import { initCards } from '../cards.js';
import { initOwnedImport } from '../owned.js';
import { renderDeck } from './results.js';
import { initPopover, markOwn } from '../popover.js';
import { initIO } from './io.js';
import { loadExample } from '../presets.js';
import { initSetup } from '../setup.js';
import '../gmtip.js'; // the game-style tooltips (both planner pages)

// ---------- results column (desktop): sticky by its top when it fits the window, by its bottom when it is taller ----------
{
  const column = document.querySelector('.rcol');
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
const setEra = (era) => {
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
  render: (r, targets) => renderDeck(r, { targets }),
  loadExample,
  example: ERA === 'global' && EXAMPLE ? { r: EXAMPLE, settings: DEFAULTS } : null,
});
const paint = () => paintRunNote(loop.stopSearch);
// A change of setup (the effect on settings.ts jobSetup below, or Auto-search switched on with a change pending): it
// greys what you no longer own in the result on show at once, before the search that replaces it, and searches again
function settingsChanged() {
  markOwn();
  loop.scheduleSearch();
}
// everything below needs the era's data (Global's from the page's data script, the GM era's fetched)
if (ERA === 'gm') {
  settle({ kind: 'loading', text: 'Loading the JP Grand Masters-era data…' });
  paint();
} // fetched: about a second
loadData().then(start, (e) => {
  settle({ kind: 'error', text: e.message });
  paint();
});
function start() {
  $input('x-auto').checked = settings.value.auto !== false;
  $input('x-auto').onchange = (/** @type {TargetedEvent<HTMLInputElement>} */ e) => {
    setAuto(e.target.checked);
    if (settings.value.auto && loopView.value.status.kind === 'pending') settingsChanged();
  }; // the switch changes no result: switching it on searches only changes made while it was off
  initScenario(); // the scenario picker (sets the training-hint rate; Unity Cup's golds follow it)

  initRoster();
  initCards();
  initOwnedImport();
  initTargets();
  initIO();
  initPopover();
  initSetup();
  // every change of setup from here on searches again (made after initScenario: its swap of a scenario this era lacks is
  // the setup the first search starts from)
  on([jobSetup], settingsChanged);
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
  for (const id of ['why-dlg', 'sched-dlg']) backdropCloses($dialog(id));

  renderRoster();
  renderTargets(); // renderTargets draws the card picker too (it lists the cards that give a target)
  // before the first search finishes: the remembered result of these settings, or the example for the defaults, or nothing
  const first = loop.showFirst();
  // the status line redraws on each change of the run state from here on (made after the first result: the example's
  // status is never drawn, the first search's "Searching…" replaces it)
  on([loopRun, loopView], paint);
  // the first search, with the saved settings (also warms the engine); none when the result on show is these settings'
  // finished search from before (it shows as done; the engine still warms for the next edit)
  if (first === 'kept') {
    warmEngine();
    paint();
  } else setTimeout(loop.doSearch, 0);
}
