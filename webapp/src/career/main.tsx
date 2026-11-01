// Career planner page: entry point. build.py bundles this and the modules it imports into dist/assets/career.<hash>.js,
// which dist/index.html loads.
// The lineage tree (trainee, two parents, their parents) is the page: each place is set by hand or left to the planner,
// which tries every owned uma and every pair of the player's veterans (veterans/) and fills the open places with its
// best pick (cmd "career"). The settings editors are the loop planner's (same modules, same saved settings).
// Here: the run machine's wiring (run.ts `createRun` on this page's solver, clock, store, job and views), the views'
// mount points, settings and start. The rest of career/:
//   state.ts           the answer store: the run on show, the answer, the last plan made, the answers kept
//   run.ts             the run machine: an edit's pause, the search, the plan job's `next`, the robustness check
//   job.ts             the career job from the settings, veterans and places (values in); the solver's types
//   current.ts         that job of the stores now; the solver call (runJob); the career on show and its parents
//   keys.ts            the keys a job, a plan job and a career are known by
//   io.ts              Export and Import of the veterans
//   roll.ts           a roll: a random result of a career (no page state; the suite runs it in node)
//   analysis/          the veteran analysis ("Try a run"): state.ts its store (her draft, her comparison), compare.ts
//                      the comparison's run machine, view.tsx Try a run's popup, Details.tsx her card
//   options.ts         the options table's figures and words ("Other careers": the plan's careers, Use this)
//   view/RunNote.tsx   the status line
//   view/Tree.tsx      the lineage tree, and the odds and Try a run beside the title
//   view/NodeDialog.tsx  the tree's node popup, and the veterans dialog's pane (VetPane)
//   view/ParentPicker.tsx  the parent picker
//   view/Sections.tsx  the career's sections in the tree: odds, deck and its popup, schedule, targets, counts
//   view/Setup.tsx     the Borrowed parent choice, and the setup panel's fold and summary
//   view/dialogs.ts    which popup is open, for what (modules that are not views open them through it)
//   view/Options.tsx   the results column (the options table) and its Saves popup
//   view/Pool.tsx      the veterans dialog: its tabs and order menu, lists and the pane beside them
//   view/Umamoe.tsx    uma.moe's button and popup (umamoesetup.ts keeps its listings)
//   view/SkillDetails.tsx  the Skill Details popup
//   view/Guard.tsx     the render guard each view draws inside
//   view/parts.ts      small parts the views share
import '../errorlog.ts'; // first: it listens before any other module runs
import '@preact/signals'; // its option hooks: a component that reads a signal draws again when it changes
import { render } from 'preact';
import { batch, computed } from '@preact/signals-core';
import type { ReadonlySignal } from '@preact/signals-core';
import type { VNode } from 'preact';
import { $, $dialog, $textarea, backdropCloses } from '../dom.ts';
import { ERA } from '../saved.ts';
import { loadData } from '../data.ts';
import { jobSetup, settings, switchEra } from '../settings.ts';
import { on } from '../store.ts';
import { restartEngine, engineBusy, enginePersist, engineSaveState, nextKill, stateKey } from '../engine.ts';
import { answerLoad, answerSave } from '../statestore.ts';
import { goldGiven } from '../job.ts';
import { initScenario, eraScenario } from '../scenario.ts';
import { initPopover } from '../popover.ts';
import '../agenda.ts'; // (its document listener is added before gmtip's)
import { cardsBadge, setupButtons, setupViews, umasBadge } from '../ui/Setup.tsx';
import { rosterCounts } from '../roster.ts';
import { cardsOwned } from '../cards.ts';
import '../gmtip.ts'; // the game-style tooltips (both planner pages)
import {
  vets,
  places,
  draft,
  persist,
  flushPersist,
  clearVets,
  dropMissing,
  handBackPlaces,
  setListings,
} from '../veterans/store.ts';
import { searchHeld } from '../veterans/pickers.ts';
import { AddList, VetEditors } from '../veterans/Editors.tsx';
import { ScheduleDialog, schedOpened } from '../ui/Schedule.tsx';
import { setRunner } from '../images.ts';
import { connectUmamoe, syncListings } from '../umamoesetup.ts';
import { UmButton, UmamoePanel } from './view/Umamoe.tsx';
import { renderGuarded, sayRenderErrors } from '../ui/RenderGuard.tsx';
import { placesKey, isFull, ROLL_K, umasNotOwned } from './job.ts';
import { currentJob, dropStaleChosen, shown, runJob } from './current.ts';
import { baseKey, refBase } from './keys.ts';
import { Analysis, draftOf } from './analysis/view.tsx';
import * as analysisStore from './analysis/state.ts';
import { createComparison } from './analysis/compare.ts';
import * as answerStore from './state.ts';
import { answer, dropAnswer, dropped, keepComparisonPlan, loadFails, runView } from './state.ts';
import { createRun } from './run.ts';
import { tName } from './view/parts.ts';
import { Guard } from './view/Guard.tsx';
import { RunNote, tickNote } from './view/RunNote.tsx';
import { Tree, Verdict } from './view/Tree.tsx';
import { NodeDialog } from './view/NodeDialog.tsx';
import { ParentPicker } from './view/ParentPicker.tsx';
import { DeckDialog } from './view/Sections.tsx';
import { SkillDetails } from './view/SkillDetails.tsx';
import { BorrowChoice, SetupSummary, initFold, sawVeterans } from './view/Setup.tsx';
import { nodeOpen, pickerFor } from './view/dialogs.ts';
import { Results, SavesPopup } from './view/Options.tsx';
import { PoolBody, PoolHead } from './view/Pool.tsx';
import { poolTab, inTab, openPool, poolClosed, selected } from './view/poolState.ts';
import { exportVets, copyExport, downloadExport, importText } from './io.ts';
import { searchCaveat } from '../proofs.ts';
import { must } from '../must.ts';
import type { Status } from './state.ts';
import type { CareerSearch } from './job.ts';

// ---------- the run ----------
// uma.moe's sync (umamoesetup.ts): the targets it searches for, and the pool its listings join
connectUmamoe({ targets: () => settings.peek().targets, setListings });
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
    baseKey: (job) => baseKey(job, umasNotOwned(settings.peek())),
    isFull: (nd) => isFull(nd, settings.peek().targets),
    planTop: ROLL_K,
    draftOf,
    draft: () => draft.peek(),
  },
});
// The run machine (run.ts `createRun`), on this page: the solver, the clock, the answer store, the job of the
// settings, veterans and places, the veteran analysis and the redraws. The views are components that draw from the
// answer store's signals (career/state.ts) and the veterans store's (veterans/store.ts): the redraws are the status
// line's clock.
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
    job: currentJob,
    places: () => placesKey(places.peek()),
    trainee: () => places.peek().trainee,
    refBase: (job) => refBase(job, umasNotOwned(settings.peek())),
    baseKey: (job) => baseKey(job, umasNotOwned(settings.peek())),
    searched: searchStatus,
    loadAnswer: () => answerLoad(ERA),
    saveAnswer: (key, value) => answerSave(ERA, key, value),
  },
  analysis: comparison,
  view: {
    shown,
    paint,
    // (every view is a component: each follows its signals)
    drawAll: paint,
    // (the check's report is in the answer store: the table follows it)
    drawCheck: paint,
  },
});
// the search's status: done, with a caveat (proofs.ts `searchCaveat`) when the search could not prove the best odds'
// order (`proven_top`), or proved it only under a lower deck cap (`complete` false), saying what the report establishes
// of why, or proved it while a scored deck beat its class's assumed cap (the caps are assumptions the proofs rest on);
// or, when no career can land every target, which targets nothing can give (no veteran or parent carries their spark;
// no deck, outfit or event gives them either, or some career would have them)
function searchStatus(m: { r: CareerSearch; ms: number }, found: string): Status {
  const cert = searchCaveat(m.r),
    note = dropped.peek(),
    droppedText = note?.places === placesKey(places.peek()) ? ` ${note.text}` : '';
  if (m.r.results.length) return { kind: 'done', text: found + cert + droppedText };
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
  return { kind: 'error', text: `No career can land all ${settings.value.targets.length} targets${why}${droppedText}` };
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
  initScenario(); // (first: the era's scenario swap is the setup the first search starts from)
  mount('bp', <BorrowChoice />, 'Borrowed parent');
  enginePersist(); // the solver's kept work survives a page reload
  // the setup editors both pages share (ui/Setup.tsx), each into its mount point; the Umas and Support cards buttons
  // open their dialogs, and their counts follow what you own
  for (const { id, what, view } of setupViews()) renderGuarded(view, $(id), what, settings);
  setupButtons();
  on([rosterCounts], umasBadge);
  on([cardsOwned], cardsBadge);
  initFold();
  // the first veterans fold the setup panel (until a fold is chosen)
  if (vets.peek().length) sawVeterans();
  on([vets], (list) => {
    if (list.length) sawVeterans();
  });
  mount('set-sum', <SetupSummary />, 'The setup summary');
  // a change of setup (targets, umas, cards, scenario, rank, Borrowed parent): the tree shows the new targets, and the
  // search runs again
  on([jobSetup], () => {
    // a career taken from the other careers' table was for that setup: the planner picks again for this one
    if (places.peek().fromTable) handBackPlaces();
    syncListings(); // a changed target set: its listings (kept, or one request)
    run.schedule();
  });
}
// ---------- start ----------
// the template's dialogs: Close (or Done) closes each
for (const d of document.querySelectorAll('dialog'))
  for (const b of d.querySelectorAll<HTMLElement>('.dclose')) b.onclick = () => d.close();
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
    initPopover();
    // uma.moe's listings for the targets, from the kept answer, before the first search (a request otherwise)
    syncListings();
    dropMissing(); // (a place naming a veteran not here: an uma.moe one whose answer is gone, say)
    // Try a run's popup: Close and a backdrop click shut it
    const simDlg = $dialog('sim-dlg');
    for (const x of simDlg.querySelectorAll<HTMLElement>('.dclose')) x.onclick = () => simDlg.close();
    backdropCloses(simDlg);
    dropStaleChosen();
    // A result names parents by their place in the veteran list: adding, removing or importing veterans changes the
    // places, so the answer on screen goes; an edit to a veteran or a place keeps it on screen, dimmed, until the new
    // one is ready. While a pool veteran's G1 wins window or a skills prompt is open its edits are saved, and the search
    // waits for it to close (veterans/pickers.ts `searchHeld`).
    const idsOf = (list: typeof vets.value) => list.map((v) => v.id).join(',');
    let ids = idsOf(vets.peek());
    on([vets, places, searchHeld], (list, _places, held) => {
      if (held) return;
      if (idsOf(list) !== ids) {
        ids = idsOf(list);
        batch(() => {
          dropAnswer();
          comparison.clear();
        });
      }
      run.schedule();
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
    $dialog('set-vets').addEventListener('close', poolClosed);
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
    mount('tree', <Tree />, 'The lineage tree');
    mount('lodds', <Verdict />, 'The odds');
    mount('v-head', <PoolHead />, "The veterans' tabs", poolTab);
    mount('v-body', <PoolBody />, 'The veterans', selected);
    mount('node-root', <NodeDialog />, 'The node popup', nodeOpen);
    mount('deck-root', <DeckDialog />, 'The deck popup');
    mount('skill-root', <SkillDetails />, 'Skill Details');
    mount('vet-editors', <VetEditors />, "The veterans' editors"); // (each editor has a guard of its own)
    mount('add-pop', <AddList host={$('add-pop')} />, 'The "+" list');
    mount('results', <Results />, 'The other careers');
    mount('um-root', <UmButton />, "uma.moe's button");
    mount('um-body', <UmamoePanel />, "uma.moe's popup");
    mount('save-root', <SavesPopup />, 'The Saves popup');
    mount('sched-root', <ScheduleDialog />, 'The race schedule', schedOpened);
    mount('vpick-root', <ParentPicker />, 'The parent picker', pickerFor);
    mount('sim-top', <Analysis comparison={comparison} />, 'Try a run', simulationDrawn);
    await run.start(); // (the last visit's answer, when nothing changed since: drawn at once)
  },
  (e: Error) => {
    loadFails(e.message);
    paint();
  },
);
