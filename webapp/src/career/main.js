// Career planner page: entry point. build.py bundles this and the modules it imports into dist/assets/career.<hash>.js,
// which dist/index.html loads.
// The lineage tree (trainee, two parents, their parents) is the page: each place is set by hand or left to the planner,
// which tries every owned uma and every pair of the player's veterans (vets.js) and fills the open places with its
// best pick (cmd "career"). The settings editors are the loop planner's (same modules, same saved settings).
// Here: the run machine (an edit's search and plan, drawn once as the answer), settings and start. The rest of career/:
//   state.ts           the answer store: the run on show, the answer, the last plan made, the answers kept
//   run.ts             the run's record, the solver call, the plan job's `next`, the robustness check on the plan on show
//   job.js             the career job from the settings, veterans and places; the career on show; the solver's types
//   keys.js            the keys a job, a plan job and a career are known by
//   io.js              Export and Import of the veterans
//   roll.mjs           a roll: a random result of a career (no page state; the suite runs it in node)
//   analysis.js        the veteran analysis ("Try a run"): her draft, her comparison, and the state they keep
//   options.js         the options table ("Other careers": the plan's careers, Use this) and its Saves popup
//   view/RunNote.js    the status line
//   view/Tree.js       the lineage tree
//   view/NodeDialog.js the tree's node popup (and the veterans dialog's pane)
//   view/ParentPicker.js  the parent picker
//   view/Sections.js   the career's sections and the results column
//   view/Pool.js       the veterans dialog, and the setup panel's fold
//   view/Setup.js      the Borrowed parent choice
//   view/parts.js      small parts the views share
// (The import order below keeps the bundle's load-time steps in their order: the options table's dialog, then the
// run's (run.ts), the node popup's, the deck popup's and the fold's. view/Sections.js imports options.js before
// analysis.js for it.)
import '../errorlog.ts'; // first: it listens before any other module runs
import { $, $dialog, $input, $textarea, backdropCloses } from '../dom.js';
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
import '../agenda.js'; // (load order: its document listener before gmtip's)
import { initSetup } from '../setup.js';
import '../gmtip.js'; // the game-style tooltips (both planner pages)
import '../odds.js'; // (load order: its dialog's step before the veterans load)
import { initTargets, renderTargets } from '../targets.js';
import { vets, plan, chosen, onVetsChange, changed, persist, clearVets, dropMissing, filterUmas } from '../vets.js';
import { initUmamoe, syncListings } from '../umamoesetup.js';
import { renderResults } from './view/Sections.js';
import { placesKey, buildJob, chosenDropped, dropStaleChosen, baseKey, refBase, shown, runJob } from './job.js';
import { outdateComparison, stopComparison, showAnalysis, clearAnalysis, keepBaseline } from './analysis.js';
import * as answerStore from './state.ts';
import { dropAnswer, loadFails } from './state.ts';
import { createRun } from './run.ts';
import { tName } from './view/parts.js';
import { paint } from './view/RunNote.js';
import { renderTree } from './view/Tree.js';
import { openNode } from './view/NodeDialog.js';
import { pickK, vpTab } from './view/ParentPicker.js';
import { vTab, poolStale, inTab, setVTab, initVSort, renderPool, drawPool, paintFold, initFold } from './view/Pool.js';
import { initBorrow } from './view/Setup.js';
import { exportVets, copyExport, downloadExport, importText } from './io.js';
import { searchCaveat } from '../proofs.mjs';
import { must } from '../must.ts';
/** @import { CareerJob, PlanReport } from './job.js' */
/** @import { Answer, PlanDone, Status } from './state.ts' */
/** @import { DeepReadonly } from '../store.ts' */

// ---------- the run ----------
// The run machine (run.ts `createRun`), on this page: the solver, the clock, the answer store, the job of the
// settings, veterans and places, the veteran analysis and the redraws. The status line and the answer are drawn from
// the answer store's signals (career/state.ts) by these explicit redraws.
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
      renderAll();
      paint();
    },
    drawCheck: () => {
      keepFocus(renderResults);
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
function searchStatus(m, found) {
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
// redraw everything, keeping the keyboard focus on the same control (every edit redraws)
function renderAll() {
  keepFocus(() => {
    renderTree();
    renderResults();
    renderPool();
  });
}
// a redraw that keeps the focus on the element of the same key (`data-k`) where it is drawn again
/** @param {() => void} draw */
function keepFocus(draw) {
  const ae = /** @type {HTMLElement} */ (document.activeElement),
    k = ae?.dataset ? ae.dataset.k : null;
  draw();
  if (k) {
    const e = document.querySelector(`[data-k="${CSS.escape(k)}"]`);
    if (e) e.focus();
  }
}

// ---------- settings ----------
function initSettings() {
  $('era-global').setAttribute('aria-pressed', String(ERA === 'global'));
  $('era-gm').setAttribute('aria-pressed', String(ERA === 'gm'));
  const setEra = (e) => {
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
  initBorrow();
  renderRoster();
  renderTargets();
  enginePersist(); // the solver's kept work survives a page reload
  initSetup();
  initFold();
  // a change of setup (targets, umas, cards, scenario, rank, Borrowed parent): the tree shows the new targets, and the
  // search runs again
  on([jobSetup], () => {
    // a career taken from the other careers' table was for that setup: the planner picks again for this one
    if (plan.fromTable) {
      plan.trainee = null;
      plan.parents = [null, null];
      plan.fromTable = false;
      chosen.career = null;
      persist();
    }
    syncListings(); // a changed target set: its listings (kept, or one request)
    run.schedule();
    renderAll();
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
    initPopover();
    // uma.moe's listings for the targets, from the kept answer, before the first search (a request otherwise)
    initUmamoe({ open: (v) => openNode({ kind: 'v', id: v.id }) });
    syncListings();
    dropMissing(); // (a place naming a veteran not here: an uma.moe one whose answer is gone, say)
    // Try a run's popup: Close and a backdrop click shut it
    const simDlg = $dialog('sim-dlg');
    for (const x of simDlg.querySelectorAll('.dclose')) /** @type {HTMLElement} */ (x).onclick = () => simDlg.close();
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
      renderAll();
    });
    for (const x of ['vets', 'new', 'auto']) $(`vp-tab-${x}`).onclick = () => vpTab(x);
    $input('vp-filter').oninput = () => filterUmas($('vp-umas'), $input('vp-filter').value);
    $('vp-auto-go').onclick = () => {
      plan.parents[pickK] = null;
      $dialog('vpick-dlg').close();
      changed();
    };
    $('v-file').onchange = async (/** @type {TargetedEvent<HTMLInputElement>} */ e) => {
      const f = must(e.target.files, 'a file input has its file list')[0];
      if (f) importText(await f.text());
      e.target.value = '';
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
      const gone = (v) => inTab(v) && (v.src !== 'umamoe' || plan.parents.includes(v.id)),
        n = vets.filter(gone).length,
        what = vTab === 'own' ? `${n} of your veterans` : `${n} borrowed veteran${n === 1 ? '' : 's'}`;
      if (n && confirm(`Remove all ${what}?`)) {
        clearVets(gone);
        $('v-msg').textContent = `Removed ${what}.`;
      }
    };
    for (const x of ['own', 'bor']) $(`v-tab-${x}`).onclick = () => setVTab(x);
    initVSort();
    renderAll();
    await run.start(); // (the last visit's answer, when nothing changed since: drawn at once)
  },
  (e) => {
    loadFails(e.message);
    paint();
  },
);
