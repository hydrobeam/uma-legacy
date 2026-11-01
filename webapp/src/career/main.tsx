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
//   analysis/          the veteran analysis ("Try a run"): state.ts its store (her draft, her comparison), compare.ts
//                      the comparison's run machine, view.tsx Try a run's popup, Details.tsx her card
//   options.js         the options table ("Other careers": the plan's careers, Use this) and its Saves popup
//   view/RunNote.tsx   the status line
//   view/Tree.tsx      the lineage tree, and the odds and Try a run beside the title
//   view/NodeDialog.tsx  the tree's node popup, and the veterans dialog's pane (VeteranView)
//   view/ParentPicker.tsx  the parent picker
//   view/Sections.tsx  the career's sections in the tree: odds, deck and its popup, schedule, targets, counts
//   view/Setup.tsx     the Borrowed parent choice, and the setup panel's fold and summary
//   view/dialogs.ts    which popup is open, for what (the el() modules open them through it)
//   view/Results.js    the results column (the options table: el(), step 11)
//   view/Pool.tsx      the veterans dialog: its tabs, lists and the pane beside them
//   view/parts.js      small parts the el() views share
import '../errorlog.ts'; // first: it listens before any other module runs
import '@preact/signals'; // its option hooks: a component that reads a signal draws again when it changes
import { render } from 'preact';
import { batch, computed } from '@preact/signals-core';
import type { ReadonlySignal } from '@preact/signals-core';
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
  places,
  draft,
  persist,
  flushPersist,
  clearVets,
  dropMissing,
  handBackPlaces,
} from '../veterans/store.ts';
import { winsOpen } from '../veterans/pickers.js';
import { setRunner } from '../images.js';
import { initUmamoe, syncListings, umamoeButton } from '../umamoesetup.js';
import { sayRenderErrors } from '../ui/RenderGuard.tsx';
import {
  placesKey,
  buildJob,
  chosenDropped,
  dropStaleChosen,
  baseKey,
  refBase,
  shown,
  runJob,
  isFull,
  ROLL_K,
} from './job.js';
import { Analysis, draftOf, drawnHere } from './analysis/view.tsx';
import * as analysisStore from './analysis/state.ts';
import { createComparison } from './analysis/compare.ts';
import * as answerStore from './state.ts';
import { answer, dropAnswer, keepComparisonPlan, loadFails, runView } from './state.ts';
import { createRun } from './run.ts';
import { tName } from './view/parts.js';
import { Guard } from './view/Guard.tsx';
import { RunNote, tickNote } from './view/RunNote.tsx';
import { Tree, Verdict } from './view/Tree.tsx';
import { NodeDialog } from './view/NodeDialog.tsx';
import { ParentPicker } from './view/ParentPicker.tsx';
import { DeckDialog, drawnCard } from './view/Sections.tsx';
import { BorrowChoice, SetupSummary, initFold } from './view/Setup.tsx';
import { nodeOpen, openNode, pickerFor } from './view/dialogs.ts';
import { renderResults } from './view/Results.js';
import { PoolBody, PoolTabs, initVSort } from './view/Pool.tsx';
import { poolTab, inTab, openPool, selected } from './view/poolState.ts';
import { exportVets, copyExport, downloadExport, importText } from './io.js';
import { searchCaveat } from '../proofs.mjs';
import { must } from '../must.ts';
import type { Status } from './state.ts';
import type { CareerSearch } from './job.js';

// ---------- the run ----------
// The veteran analysis' comparison (analysis/compare.ts `createComparison`): the solver, the clock, the answer store,
// the analysis store, the career on show and her draft. The run machine below drives it through its hooks.
// Try a run draws her comparison and her draft
const simulationDrawn = computed(() => [analysisStore.simulation.value, draft.value]);
const comparison = createComparison({
  engine: { run: runJob, restart: restartEngine, saveState: engineSaveState },
  clock: { setTimer: (f, ms) => setTimeout(f, ms) },
  answers: { answer, keepComparisonPlan },
  store: analysisStore,
  page: {
    shown,
    searching: () => runView.peek().searching,
    targets: () => settings.peek().targets,
    baseKey,
    isFull,
    planTop: ROLL_K,
    draftOf,
    draft: () => draft.peek(),
  },
});
// The run machine (run.ts `createRun`), on this page: the solver, the clock, the answer store, the job of the
// settings, veterans and places, the veteran analysis and the redraws. The views that are components draw from the
// answer store's signals (career/state.ts) and the veterans store's (veterans/store.ts); the redraws here are the el()
// views'.
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
    places: () => placesKey(places.peek()),
    trainee: () => places.peek().trainee,
    refBase,
    baseKey,
    searched: searchStatus,
    loadAnswer: () => answerLoad(ERA),
    saveAnswer: (key, value) => answerSave(ERA, key, value),
  },
  analysis: comparison,
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
    dropped = chosenDropped?.places === placesKey(places.peek()) ? ` ${chosenDropped.text}` : '';
  if (m.r.results.length) return { kind: 'done', text: found + cert + dropped };
  const carried = new Set(
    vets
      .peek()
      .flatMap((v) => [v, ...v.parents])
      .flatMap((x) => Object.keys(x.sparks || {}).map(Number)),
  );
  const none = settings.value.targets
    .filter((t) => !goldGiven(settings.value.scen, t.group) && !carried.has(t.group))
    .map((t) => tName(t.group));
  const why = none.length
    ? `: none of your veterans or their parents carries ${none.join(', ')}, and no deck or outfit gives ${none.length > 1 ? 'them' : 'it'}.`
    : ' with these veterans.';
  return { kind: 'error', text: `No career can land all ${settings.value.targets.length} targets${why}${dropped}` };
}
// the views still built with el() after a change: the results column (step 11) and uma.moe's button (step 11); the
// components follow their signals
function drawUnported() {
  renderResults();
  umamoeButton();
}
// ---------- the views: each component into its mount point, behind a render guard (view/Guard.tsx) ----------
// a view that could not be drawn says so on the status line, as a failed search does (the answer on show stays)
sayRenderErrors(loadFails);
/** `what`: the view's name on the status line (none for the status line itself); `also`: what else it draws from */
const mount = (id: string, view: VNode, what?: string, also?: ReadonlySignal<unknown>) =>
  render(
    <Guard what={what} also={also}>
      {view}
    </Guard>,
    $(id),
  );
// The status line (its live region is the template's, there from load) and what it marks around it: the status bar's
// state (its colours), the results and the tree dimmed while a run replaces the answer on show, the runner (the trainee
// you chose, else the career on show). Drawn from the first paint the run machine asks for; then `paint` moves its clock.
function marks() {
  const { searching, started } = runView.peek(),
    { status } = answer.peek(),
    r = shown();
  setRunner(places.peek().trainee || r?.trainee?.id);
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
    on([runView, answer, places], marks);
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
  mount('bp', <BorrowChoice />, 'Borrowed parent');
  renderRoster();
  renderTargets();
  enginePersist(); // the solver's kept work survives a page reload
  initSetup();
  initFold();
  mount('set-sum', <SetupSummary />, 'The setup summary');
  // a change of setup (targets, umas, cards, scenario, rank, Borrowed parent): the tree shows the new targets, and the
  // search runs again
  on([jobSetup], () => {
    // a career taken from the other careers' table was for that setup: the planner picks again for this one
    if (places.peek().fromTable) handBackPlaces();
    syncListings(); // a changed target set: its listings (kept, or one request)
    run.schedule();
    drawUnported();
  });
}
// ---------- start ----------
// (the loop members kept per setup until the plan stopped running the loop planner's search: the old store goes)
try {
  localStorage.removeItem('llp:loops');
} catch {
  // storage blocked: nothing was kept to remove
}
// a save of the veterans still due goes out when the page is hidden or left (a reload or close loses nothing)
addEventListener('pagehide', flushPersist);
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'hidden') flushPersist();
});
document.addEventListener('click', (e) => {
  if (e.target instanceof HTMLDialogElement && e.target.open) e.target.close();
}); // a backdrop click closes
loadData().then(
  async () => {
    initSettings();
    initPopover({ drawn: (chip) => drawnCard(chip) || drawnHere(chip) });
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
    // places, so the answer on screen goes; an edit to a veteran or a place keeps it on screen, dimmed, until the new
    // one is ready. While a pool veteran's G1 wins window is open its edits are saved, and the search waits for it to
    // close.
    const idsOf = (list: typeof vets.value) => list.map((v) => v.id).join(',');
    let ids = idsOf(vets.peek());
    on([vets, places, winsOpen], (list, _places, editingWins) => {
      if (editingWins) return;
      if (idsOf(list) !== ids) {
        ids = idsOf(list);
        batch(() => {
          dropAnswer();
          comparison.clear();
        });
      }
      run.schedule();
      drawUnported();
    });
    // an edit of Try a run's draft: the comparison on show is for the veteran before it (a roll or a new career clears
    // the figures with her, so it marks nothing)
    on([draft], analysisStore.draftChanged);
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
      openPool();
      $dialog('set-vets').showModal();
    };
    $('v-clear').onclick = () => {
      // the open tab's veterans
      // (uma.moe's listings stay, unless one is a chosen parent: they are its answer for the targets, not yours)
      const parents = places.value.parents,
        gone = (v: (typeof vets.value)[number]) => inTab(v) && (v.src !== 'umamoe' || parents.includes(v.id)),
        n = vets.value.filter(gone).length,
        what = poolTab.peek() === 'own' ? `${n} of your veterans` : `${n} borrowed veteran${n === 1 ? '' : 's'}`;
      if (n && confirm(`Remove all ${what}?`)) {
        clearVets(gone);
        $('v-msg').textContent = `Removed ${what}.`;
      }
    };
    initVSort();
    mount('tree', <Tree />, 'The lineage tree');
    mount('lodds', <Verdict />, 'The odds');
    mount('v-tabs', <PoolTabs />, "The veterans' tabs", poolTab);
    mount('v-body', <PoolBody />, 'The veterans', selected);
    mount('node-root', <NodeDialog />, 'The node popup', nodeOpen);
    mount('deck-root', <DeckDialog />, 'The deck popup');
    mount('vpick-root', <ParentPicker />, 'The parent picker', pickerFor);
    mount('sim-top', <Analysis comparison={comparison} />, 'Try a run', simulationDrawn);
    drawUnported();
    await run.start(); // (the last visit's answer, when nothing changed since: drawn at once)
  },
  (e: Error) => {
    loadFails(e.message);
    paint();
  },
);
