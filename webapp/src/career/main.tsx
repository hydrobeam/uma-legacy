// Career planner page: entry point. build.py bundles this and the modules it imports into dist/assets/career.<hash>.js,
// which dist/index.html loads.
// The lineage tree (trainee, two parents, their parents) is the page: each place is set by hand or left to the planner,
// which tries every owned uma and every pair of the player's veterans (veterans/) and fills the open places with its
// best pick (cmd "career"). The settings editors are the loop planner's (same modules, same saved settings).
// Here: the run machine's wiring (run.ts `createRun` on this page's solver, clock, store, job and views), the views'
// mount points, settings and start. The rest of career/:
//   state.ts           the answer store: the run on show, the answer, the last plan made, the answers kept
//   run.ts             the run machine: an edit's pause, the search, the plan job's `next`, the robustness check
//   job.js             the career job from the settings, veterans and places; the solver call (runJob); the career on
//                      show; the solver's types
//   keys.js            the keys a job, a plan job and a career are known by
//   io.js              Export and Import of the veterans
//   roll.mjs           a roll: a random result of a career (no page state; the suite runs it in node)
//   analysis.js        the veteran analysis ("Try a run"): her draft, her comparison, and the state they keep
//   options.js         the options table ("Other careers": the plan's careers, Use this) and its Saves popup
//   view/RunNote.tsx   the status line
//   view/Tree.tsx      the lineage tree, and the odds and Try a run beside the title
//   view/NodeDialog.tsx  the tree's node popup, and the veterans dialog's pane (VeteranView)
//   view/ParentPicker.tsx  the parent picker
//   view/Sections.tsx  the career's sections in the tree: odds, deck and its popup, schedule, targets, counts
//   view/Setup.tsx     the Borrowed parent choice
//   view/dialogs.ts    which popup is open, for what (the el() modules open them through it)
//   view/Results.js    the results column (the options table, Try a run: el(), steps 9 and 11)
//   view/Pool.js       the veterans dialog, and the setup panel's fold (el(), step 10)
//   view/parts.js      small parts the el() views share
import '../errorlog.ts'; // first: it listens before any other module runs
import '@preact/signals'; // its option hooks: a component that reads a signal draws again when it changes
import { render } from 'preact';
import type { VNode } from 'preact';
import { $, $dialog, $textarea, backdropCloses } from '../dom.js';
import { ERA } from '../saved.ts';
import { loadData } from '../data.js';
import { jobSetup, settings, switchEra } from '../settings.ts';
import { on } from '../store.ts';
import { restartEngine, engineBusy, enginePersist, engineSaveState, nextKill, stateKey } from '../engine.ts';
import { answerLoad, answerSave } from '../statestore.js';
import { goldGiven } from '../job.js';
import { initScenario, eraScenario } from '../scenario.js';
import { initRoster, renderRoster } from '../roster.js';
import { initCards } from '../cards.js';
import { initOwnedImport } from '../owned.js';
import { initPopover } from '../popover.js';
import '../agenda.js'; // (its document listener is added before gmtip's)
import { initSetup } from '../setup.js';
import '../gmtip.js'; // the game-style tooltips (both planner pages)
import { initTargets, renderTargets } from '../targets.js';
import {
  vets,
  vetsRev,
  plan,
  onVetsChange,
  persist,
  clearVets,
  dropMissing,
  handBackPlaces,
} from '../veterans/store.js';
import '../veterans/pickers.js'; // (the editors' dialog and document listeners are added with the veterans' load)
import '../veterans/sparks.js';
import { setRunner } from '../images.js';
import { initUmamoe, syncListings } from '../umamoesetup.js';
import { Boundary } from '../ui/Boundary.tsx';
import { placesKey, buildJob, chosenDropped, dropStaleChosen, baseKey, refBase, shown, runJob } from './job.js';
import { outdateComparison, stopComparison, showAnalysis, clearAnalysis, keepBaseline } from './analysis.js';
import * as answerStore from './state.ts';
import { answer, dropAnswer, loadFails, runView } from './state.ts';
import { createRun } from './run.ts';
import { tName } from './view/parts.js';
import { RunNote, tickNote } from './view/RunNote.tsx';
import { Tree, Verdict } from './view/Tree.tsx';
import { NodeDialog, VetPane } from './view/NodeDialog.tsx';
import { ParentPicker } from './view/ParentPicker.tsx';
import { DeckDialog, drawnCard } from './view/Sections.tsx';
import { BorrowChoice } from './view/Setup.tsx';
import { openNode } from './view/dialogs.ts';
import { renderResults } from './view/Results.js';
import { vTab, poolStale, inTab, setVTab, initVSort, renderPool, drawPool, paintFold, initFold } from './view/Pool.js';
import { exportVets, copyExport, downloadExport, importText } from './io.js';
import { searchCaveat } from '../proofs.mjs';
import { must } from '../must.ts';
import type { Status } from './state.ts';
import type { CareerSearch } from './job.js';

// ---------- the run ----------
// The run machine (run.ts `createRun`), on this page: the solver, the clock, the answer store, the job of the
// settings, veterans and places, the veteran analysis and the redraws. The views that are components draw from the
// answer store's signals (career/state.ts) and the veterans' (`vetsRev`); the redraws here are the el() views'.
const run = createRun({
  engine: {
    run: runJob,
    busy: engineBusy,
    restart: restartEngine,
    nextKill,
    saveState: engineSaveState,
    key: stateKey,
  },
  clock: {
    now: () => performance.now(),
    setTimer: (f, ms) => setTimeout(f, ms),
    every: (f, ms) => setInterval(f, ms),
    clearTimer: (id) => clearTimeout(id),
  },
  store: answerStore,
  page: {
    job: () => buildJob(settings.peek()),
    places: () => placesKey(),
    trainee: () => plan.trainee,
    refBase,
    baseKey,
    searched: searchStatus,
    loadAnswer: () => answerLoad(ERA),
    saveAnswer: (key, value) => answerSave(ERA, key, value),
  },
  analysis: {
    outdate: outdateComparison,
    stop: stopComparison,
    show: showAnalysis,
    clear: clearAnalysis,
    keepBaseline,
  },
  view: {
    shown,
    paint,
    drawAll: () => {
      drawUnported();
      paint();
    },
    drawCheck: () => {
      renderResults();
      paint();
    },
  },
});
// the search's status: done, with a caveat (proofs.mjs `searchCaveat`) when the search could not prove the best odds'
// order (`proven_top`), or proved it only under a lower deck cap (`complete` false), saying what the report establishes
// of why, or proved it while a scored deck beat its class's assumed cap (the caps are assumptions the proofs rest on);
// or, when no career can land every target, which targets nothing can give (no veteran or parent carries their spark;
// no deck, outfit or event gives them either, or some career would have them)
/** @returns {Status} */
function searchStatus(m: { r: CareerSearch; ms: number }, found: string): Status {
  const cert = searchCaveat(m.r),
    dropped = chosenDropped?.places === placesKey() ? ` ${chosenDropped.text}` : '';
  if (m.r.results.length) return { kind: 'done', text: found + cert + dropped };
  const carried = new Set(
    vets.flatMap((v) => [v, ...v.parents]).flatMap((x) => Object.keys(x.sparks || {}).map(Number)),
  );
  const none = settings.value.targets
    .filter((t) => !goldGiven(settings.value.scen, t.group) && !carried.has(t.group))
    .map((t) => tName(t.group));
  const why = none.length
    ? `: none of your veterans or their parents carries ${none.join(', ')}, and no deck or outfit gives ${none.length > 1 ? 'them' : 'it'}.`
    : ' with these veterans.';
  return { kind: 'error', text: `No career can land all ${settings.value.targets.length} targets${why}${dropped}` };
}
// the views still built with el() after a change: the results column (steps 9, 11) and the pool (step 10); the
// components follow their signals
function drawUnported() {
  renderResults();
  renderPool();
}
// ---------- the views: each component into its mount point, behind an error boundary ----------
const mount = (id: string, view: VNode) => render(<Boundary>{view}</Boundary>, $(id));
// The status line (its live region is the template's, there from load) and what it marks around it: the status bar's
// state (its colours), the results and the tree dimmed while a run replaces the answer on show, the runner (the trainee
// you chose, else the career on show). Drawn from the first paint the run machine asks for; then `paint` moves its clock.
function marks() {
  const { searching, started } = runView.peek(),
    { status } = answer.peek(),
    r = shown();
  setRunner(plan.trainee || r?.trainee?.id);
  $('rhead').dataset.kind = searching ? 'busy' : status.kind;
  const dimmed = searching && started && !!r; // the previous answer, on screen until this run's
  $('results').classList.toggle('stale', dimmed || (!searching && ['error', 'empty'].includes(status.kind)));
  $('tree').classList.toggle('stale', dimmed);
  $('results').setAttribute('aria-busy', String(searching));
}
let noteShown = false;
function paint() {
  if (!noteShown) {
    noteShown = true;
    marks();
    on([runView, answer, vetsRev], marks);
    mount('run-note', <RunNote />);
  }
  tickNote();
}

// ---------- settings ----------
function initSettings() {
  $('era-global').setAttribute('aria-pressed', String(ERA === 'global'));
  $('era-gm').setAttribute('aria-pressed', String(ERA === 'gm'));
  const setEra = (e: 'global' | 'gm') => {
    if (e === ERA) return;
    switchEra(e, eraScenario(e));
    persist();
    location.reload();
  };
  $('era-global').onclick = () => setEra('global');
  $('era-gm').onclick = () => setEra('gm');
  initScenario();
  initRoster();
  initCards();
  initOwnedImport();
  initTargets();
  mount('bp', <BorrowChoice />);
  renderRoster();
  renderTargets();
  enginePersist(); // the solver's kept work survives a page reload
  initSetup();
  initFold();
  // a change of setup (targets, umas, cards, scenario, rank, Borrowed parent): the tree shows the new targets, and the
  // search runs again
  on([jobSetup], () => {
    // a career taken from the other careers' table was for that setup: the planner picks again for this one
    if (plan.fromTable) handBackPlaces();
    syncListings(); // a changed target set: its listings (kept, or one request)
    run.schedule();
    drawUnported();
    paintFold();
  });
}
// ---------- start ----------
// (the loop members kept per setup until the plan stopped running the loop planner's search: the old store goes)
try {
  localStorage.removeItem('llp:loops');
} catch {
  // storage blocked: nothing was kept to remove
}
document.addEventListener('click', (e) => {
  if (e.target instanceof HTMLDialogElement && e.target.open) e.target.close();
}); // a backdrop click closes
loadData().then(
  async () => {
    initSettings();
    initPopover({ drawn: drawnCard });
    // uma.moe's listings for the targets, from the kept answer, before the first search (a request otherwise)
    initUmamoe({ open: (v) => openNode({ kind: 'v', id: v.id }) });
    syncListings();
    dropMissing(); // (a place naming a veteran not here: an uma.moe one whose answer is gone, say)
    // Try a run's popup: Close and a backdrop click shut it
    const simDlg = $dialog('sim-dlg');
    for (const x of simDlg.querySelectorAll<HTMLElement>('.dclose')) x.onclick = () => simDlg.close();
    backdropCloses(simDlg);
    dropStaleChosen();
    // A result names parents by their place in the veteran list: adding, removing or importing veterans changes the
    // places, so the answer on screen goes; an edit to a veteran keeps it on screen, dimmed, until the new one is ready.
    const vetsSig = () => vets.map((v) => v.id).join(',');
    let sig = vetsSig();
    onVetsChange(() => {
      if (vetsSig() !== sig) {
        sig = vetsSig();
        dropAnswer();
        clearAnalysis();
      }
      run.schedule();
      drawUnported();
    });
    $('v-file').onchange = async (e: Event) => {
      const input = e.target as HTMLInputElement;
      const f = must(input.files, 'a file input has its file list')[0];
      if (f) importText(await f.text());
      input.value = '';
    };
    $('v-paste').onclick = () => {
      $textarea('paste-text').value = '';
      $dialog('paste-dlg').showModal();
    };
    $('v-export').onclick = exportVets;
    $('vx-copy').onclick = copyExport;
    $('vx-dl').onclick = downloadExport;
    $('paste-load').onclick = () => {
      if (importText($textarea('paste-text').value)) $dialog('paste-dlg').close();
    };
    $('v-toggle').onclick = () => {
      if (poolStale) drawPool();
      $dialog('set-vets').showModal();
    };
    // however the dialog opens, its cards are drawn before it paints (a mutation callback runs before the next frame)
    new MutationObserver(() => {
      if ($dialog('set-vets').open && poolStale) drawPool();
    }).observe($dialog('set-vets'), { attributes: true, attributeFilter: ['open'] });
    $('v-clear').onclick = () => {
      // the open tab's veterans
      // (uma.moe's listings stay, unless one is a chosen parent: they are its answer for the targets, not yours)
      const gone = (v: (typeof vets)[number]) => inTab(v) && (v.src !== 'umamoe' || plan.parents.includes(v.id)),
        n = vets.filter(gone).length,
        what = vTab === 'own' ? `${n} of your veterans` : `${n} borrowed veteran${n === 1 ? '' : 's'}`;
      if (n && confirm(`Remove all ${what}?`)) {
        clearVets(gone);
        $('v-msg').textContent = `Removed ${what}.`;
      }
    };
    for (const x of ['own', 'bor']) $(`v-tab-${x}`).onclick = () => setVTab(x);
    initVSort();
    mount('tree', <Tree />);
    mount('lodds', <Verdict />);
    mount('v-pane', <VetPane />);
    mount('node-root', <NodeDialog />);
    mount('deck-root', <DeckDialog />);
    mount('vpick-root', <ParentPicker />);
    drawUnported();
    await run.start(); // (the last visit's answer, when nothing changed since: drawn at once)
  },
  (e: Error) => {
    loadFails(e.message);
    paint();
  },
);
