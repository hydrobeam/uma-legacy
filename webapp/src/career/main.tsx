// Career planner page: entry point, index.html's script (Vite bundles it and the modules it imports).
// The lineage tree (trainee, two parents, their parents) is the page: each place is set by hand or left to the planner,
// which tries every owned uma and every pair of the player's veterans (veterans/) and fills the open places with its
// best pick (cmd "career"). The settings editors are the loop planner's (same modules, same saved settings).
// Here: the run machine's wiring (run.ts `createRun` on this page's solver, clock, store, job and views), the page's
// one root (Layout.tsx, hydrated over the build's markup) with its views, settings and start. The rest of career/:
//   Layout.tsx         the page's one root: its markup, drawn at build time (vite/prerender.mjs) and hydrated here
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
import { started } from '../errorlog.ts'; // first: it listens before any other module runs
import '@preact/signals'; // its option hooks: a component that reads a signal draws again when it changes
import { hydrate } from 'preact';
import type { ComponentChildren } from 'preact';
import { signal } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import { ERA } from '../saved.ts';
import type { Era } from '../eras.ts';
import { must } from '../must.ts';
import { update } from '../storage.ts';
import { loadData } from '../data.ts';
import CAREER from '#career-data';
import { jobSetup, settings } from '../settings.ts';
import { on } from '../store.ts';
import { restartEngine, engineBusy, enginePersist, engineSaveState, nextKill } from '../engine.ts';
import { answerLoad, answerSave } from '../statestore.ts';
import { goldGiven } from '../job.ts';
import { initScenario } from '../scenario.ts';
import { initPopover } from '../popover.ts';
import { CardPop } from '../ui/CardPop.tsx';
import '../agenda.ts'; // (its document listener is added before gmtip's)
import { openSetupDialog, pickEra, setupBadges, setupViews } from '../ui/Setup.tsx';
import { useHydrated } from '../ui/Shell.tsx';
import '../gmtip.ts'; // the game-style tooltips (both planner pages)
import {
  vets,
  places,
  draft,
  persist,
  flushPersist,
  dropMissing,
  handBackPlaces,
  setListings,
  readyVets,
  parentsPlaced,
} from '../veterans/store.ts';
import { AddList, VetEditors } from '../veterans/Editors.tsx';
import { ScheduleDialog } from '../ui/Schedule.tsx';
import { setRunner } from '../images.ts';
import { connectUmamoe, syncListings } from '../umamoesetup.ts';
import { UmButton, UmamoeDialog } from './view/Umamoe.tsx';
import { sayRenderErrors } from '../ui/RenderGuard.tsx';
import { CareerLayout } from './Layout.tsx';
import type { CareerState, CareerViews } from './Layout.tsx';
import { placesKey, isFull, ROLL_K, umasNotOwned } from './job.ts';
import { currentJob, dropStaleChosen, followPool, shown, runJob } from './current.ts';
import { baseKey, refBase } from './keys.ts';
import { TryRunDialog, draftOf } from './analysis/view.tsx';
import * as analysisStore from './analysis/state.ts';
import { createComparison } from './analysis/compare.ts';
import * as answerStore from './state.ts';
import { answer, dropped, keepComparisonPlan, loadFails, runView } from './state.ts';
import { createRun } from './run.ts';
import { tName } from './view/parts.ts';
import { Guard } from './view/Guard.tsx';
import { RunNote, tickNote } from './view/RunNote.tsx';
import { Tree, Verdict } from './view/Tree.tsx';
import { NodeDialog } from './view/NodeDialog.tsx';
import { ParentPicker } from './view/ParentPicker.tsx';
import { DeckDialog } from './view/Sections.tsx';
import { SkillDetails } from './view/SkillDetails.tsx';
import { BorrowChoice, SetupSummary, foldClick, folded, sawVeterans, summaryLabel } from './view/Setup.tsx';
import { nodeOpen, pickerFor } from './view/dialogs.ts';
import { Results, SavesPopup } from './view/Options.tsx';
import { VetsDialog } from './view/Pool.tsx';
import { openPool } from './view/poolState.ts';
import { ExportDialog, PasteDialog, openExport } from './view/Io.tsx';
import { searchCaveat } from '../proofs.ts';
import { fitSparks } from '../details.ts';
import type { Status } from './state.ts';
import type { CareerSearch } from './job.ts';

// ---------- the run ----------
// uma.moe's sync (umamoesetup.ts): the targets it searches for, and the pool its listings join
connectUmamoe({ targets: () => settings.peek().targets, setListings });
// The veteran analysis' comparison (analysis/compare.ts `createComparison`): the solver, the clock, the answer store,
// the analysis store, the career on show and her draft. The run machine below drives it through its hooks.
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
    pool: () => readyVets.peek().map((v) => v.id),
    places: () => placesKey(places.peek()),
    trainee: () => places.peek().trainee,
    refBase: (job) => refBase(job, umasNotOwned(settings.peek())),
    baseKey: (job) => baseKey(job, umasNotOwned(settings.peek())),
    searched: searchStatus,
    loadAnswer: () => answerLoad(ERA),
    saveAnswer: (key, value) => answerSave(ERA, key, value),
  },
  analysis: comparison,
  view: { shown, paint },
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
      .flatMap((v) => [v, ...parentsPlaced(v.parents).map(([parent]) => parent)])
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
// ---------- the page: one root (Layout.tsx), the build's markup hydrated ----------
// a view that could not be drawn says so on the status line, as a failed search does (the answer on show stays)
sayRenderErrors(loadFails);
/** `view` behind the render guard (view/Guard.tsx): `what`, its name on the status line (none for the status line
 *  itself); `also`, what else it draws from */
const guarded = (what: string | undefined, view: ComponentChildren, also?: ReadonlySignal<unknown>) => (
  <Guard what={what} also={also}>
    {view}
  </Guard>
);
// the views, made once (the same vnodes on every draw of the root, so a redraw of the layout leaves them be); drawn
// once the game data is in (`ready`), the status line from the first paint the run machine asks for (`noteShown`)
const NOTE = guarded(undefined, <RunNote />);
const setup = setupViews({ page: 'career' });
const VIEWS: CareerViews = {
  rank: setup.rank,
  scenario: setup.scenario,
  targets: setup.targets,
  borrow: guarded('Borrowed parent', <BorrowChoice />),
  summary: guarded('The setup summary', <SetupSummary />),
  verdict: guarded('The odds', <Verdict />),
  umamoe: guarded("uma.moe's button", <UmButton />),
  tree: guarded('The lineage tree', <Tree />),
  results: guarded('The other careers', <Results />),
  cardPop: guarded('The deck card panel', <CardPop home={null} />), // (its slot in the results column)
  dialogs: {
    'vets-root': guarded('The veterans dialog', <VetsDialog />),
    'um-dlg-root': guarded("uma.moe's popup", <UmamoeDialog />),
    'sim-root': guarded('Try a run', <TryRunDialog comparison={comparison} />),
    'set-umas-root': setup.umasDialog,
    'set-cards-root': setup.cardsDialog,
    'vpick-root': guarded('The parent picker', <ParentPicker />, pickerFor),
    'node-root': guarded('The node popup', <NodeDialog />, nodeOpen),
    'deck-root': guarded('The deck popup', <DeckDialog />),
    'skill-root': guarded('Skill Details', <SkillDetails />),
    'save-root': guarded('The Saves popup', <SavesPopup />),
    'sched-root': <ScheduleDialog />,
    'vet-editors': guarded("The veterans' editors", <VetEditors />), // (each editor has a guard of its own)
    'paste-root': guarded('The Paste popup', <PasteDialog />),
    'vx-root': guarded('The Export popup', <ExportDialog />),
  },
  addList: guarded('The "+" list', <AddList />),
};
const ready = signal(false),
  noteShown = signal(false);
/** The root: the build's markup on the first draw (the hydration), then the page. Around the views it draws the
 *  status bar's state (its colours), the results and the tree dimmed while a run replaces the answer on show, the
 *  setup bar's fold, the Enhance buttons' counts and the buttons' actions. */
function App() {
  const hydrated = useHydrated();
  if (!hydrated) return <CareerLayout version={__LLP_VERSION__} />;
  const isReady = ready.value,
    noting = noteShown.value;
  const state: CareerState = { era: ERA };
  if (noting) {
    const { searching, started } = runView.value,
      { status } = answer.value,
      dimmed = searching && started && !!shown(); // the previous answer, on screen until this run's
    Object.assign(state, {
      kind: searching ? 'busy' : status.kind,
      resultsStale: dimmed || (!searching && ['error', 'empty'].includes(status.kind)),
      treeStale: dimmed,
      busy: searching,
    });
  }
  if (isReady)
    Object.assign(state, {
      ...setupBadges(),
      folded: folded.value,
      summaryLabel: summaryLabel.value,
      onEra: (era: Era) => pickEra(era, persist),
      onUmas: () => openSetupDialog('umas'),
      onCards: () => openSetupDialog('cards'),
      onBar: foldClick,
      onVeterans: openPool,
      onExport: openExport,
    });
  return (
    <CareerLayout version={__LLP_VERSION__} note={noting ? NOTE : null} views={isReady ? VIEWS : null} state={state} />
  );
}
// the status line, drawn from the first paint the run machine asks for; then `paint` moves its clock. The runner: the
// trainee you chose, else the career on show.
function paint() {
  if (!noteShown.peek()) {
    noteShown.value = true;
    const runner = () => setRunner(places.peek().trainee || shown()?.trainee?.id);
    runner();
    on([runView, answer, places], runner);
  }
  tickNote();
}

// ---------- settings ----------
function initSettings() {
  initScenario(); // (first: the era's scenario swap is the setup the first search starts from)
  enginePersist(); // the solver's kept work survives a page reload
  // the first veterans fold the setup panel (until a fold is chosen)
  if (vets.peek().length) sawVeterans();
  on([vets], (list) => {
    if (list.length) sawVeterans();
  });
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
// (the loop members kept per setup until the plan stopped running the loop planner's search: the old store goes)
update('llp:loops', () => null); // (storage blocked: nothing was kept to remove)
// a resize: the spark names on show shrink to fit their new width (details.ts fitSparks)
addEventListener('resize', () => fitSparks(document));
// a save of the veterans still due goes out when the page is hidden or left (a reload or close loses nothing)
addEventListener('pagehide', flushPersist);
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'hidden') flushPersist();
});
hydrate(<App />, must(document.getElementById('app'), 'index.html has the root'));
loadData(CAREER).then(
  async () => {
    initSettings();
    initPopover();
    // uma.moe's listings for the targets, from the kept answer, before the first search (a request otherwise)
    syncListings();
    dropMissing(); // (a place naming a veteran not here: an uma.moe one whose answer is gone, say)
    dropStaleChosen();
    followPool(run, comparison); // (an edit of the veterans or places searches again: current.ts)
    // an edit of Try a run's draft: the comparison on show is for the veteran before it (a roll or a new career clears
    // the figures with her, so it marks nothing)
    on([draft], analysisStore.draftChanged);
    ready.value = true;
    await run.start(); // (the last visit's answer, when nothing changed since: drawn at once)
  },
  (e: Error) => {
    loadFails(e.message);
    paint();
  },
);
started(); // (the last statement: errorlog.ts)
