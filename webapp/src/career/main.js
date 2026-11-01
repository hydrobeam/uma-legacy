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
import { placesKey, runKey, buildJob, chosenDropped, dropStaleChosen, baseKey, refBase, shown } from './job.js';
import { jobKey, planKey } from './keys.js';
import {
  outdateComparison,
  stopComparison,
  showAnalysis,
  clearAnalysis,
  keepBaseline,
  keptPlan,
  reviseKeptPlan,
} from './analysis.js';
import {
  answer,
  answerFails,
  dropAnswer,
  keepAnswer,
  keepSavedAnswer,
  loadFails,
  newPlanId,
  planProgresses,
  planStarts,
  recallAnswer,
  runEnds,
  runEndsWith,
  runView,
  runWaits,
  searchStarts,
  showAnswer,
  showKeptAnswer,
} from './state.ts';
import { checkComplete, checkPhase, run, progText, runJob, PLAN_NEXT } from './run.ts';
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
// every change: redraw now, search again after a short pause (a change during a search stops it)
function schedule() {
  // an edit that leaves the job as it is (a preset saved or renamed, a setting changed and back) asks for the answer
  // already coming or on show: a run under way goes on, and an answer on show stands (with her comparison, if one is
  // running); a job that can't be built (`Blocked`) goes on as any other edit
  /** @type {string | null} */
  let key = null;
  try {
    key = runKey(buildJob(settings.value));
  } catch {
    /* the search says why */
  }
  const { last, planState } = answer.value;
  if (
    key !== null &&
    (run.inflight
      ? run.inflight === key
      : !runView.value.searching && last && `${last.key}\n${last.plan}` === key && planState.state !== 'error')
  )
    return;
  clearTimeout(run.timer);
  run.id++;
  run.inflight = null;
  // only a job in flight is cut short: an idle worker keeps what it holds for the next jobs (the last search, every
  // uma's best career, the proofs); during the pause before a search nothing runs, so a second edit kills nothing
  if (engineBusy()) restartEngine();
  // a comparison in flight is stopped with it: its panel goes back to idle, or a roll's quick figures stand unchecked
  // (an answer that stands keeps her)
  stopComparison();
  runWaits(performance.now());
  paint();
  run.timer = setTimeout(search, 600);
}

// the answer the last visit drew, when it was for this setup (the solver's and data's fingerprints, the job, the places):
// put among the answers kept (career/state.ts), so the first search draws it at once instead of planning (the solver is
// deterministic for its job)
async function preloadAnswer() {
  let job;
  try {
    job = buildJob(settings.value);
  } catch {
    return;
  }
  const key = `${jobKey(job)}\n${placesKey()}`,
    saved = await answerLoad(ERA);
  if (saved?.key === `${stateKey()}\n${key}` && saved.value?.last) keepSavedAnswer(key, saved.value);
}
// A run: the search, then (with the trainee's place left to the planner) the plan, both on the one worker; the answer
// is drawn once, at the end. Two solver jobs: the plan's reuses the search's (the solver keeps its last search, so the
// plan job does none of that work again; a second job costs a few ms), and the page knows which phase is at work
// without the solver reporting it. A newer edit stops a run in either phase (`runId`); nothing of it is drawn.
async function search() {
  const id = ++run.id;
  outdateComparison();
  let job;
  try {
    job = buildJob(settings.value);
  } catch (e) {
    endRun({
      status: { kind: e.kind || 'error', text: e.message, act: e.act },
      last: null,
      planState: { state: 'none' },
    });
    return;
  }
  // edits that end where they started (a setting changed and back, a star clicked twice): the job is the one whose
  // answer is on show (era, settings, veterans as the job carries them, and the same places), so it stands and nothing
  // runs (her comparison, if the edits cut one short, is hers to start again); a failed plan makes it run again
  const key = jobKey(job),
    places = placesKey(),
    answerKey = `${key}\n${places}`;
  const { last, planState } = answer.value;
  if (last && last.key === key && last.plan === places && planState.state !== 'error') {
    runEnds();
    clearInterval(run.tick);
    renderAll();
    paint();
    resumeCheck(id);
    return;
  }
  const hit = recallAnswer(answerKey);
  if (hit) {
    showKeptAnswer(hit);
    const r = shown();
    showAnalysis(r);
    clearInterval(run.tick);
    renderAll();
    paint();
    resumeCheck(id);
    return;
  }
  run.inflight = answerKey;
  searchStarts(performance.now());
  clearInterval(run.tick);
  run.tick = setInterval(paint, 1000);
  paint();
  const fail = (text) => {
    answerFails(text);
    clearAnalysis();
  };
  /** @type {DeepReadonly<PlanDone>|null} the plan to check once it is shown (`checkPhase`) */
  let checkFor = null;
  try {
    const m = await runJob(job);
    if (id !== run.id) return;
    if ('error' in m.r) fail(`Not searched: ${m.r.error}`);
    else {
      const found = searchedText(m),
        searched = searchStatus(m, found);
      const best = m.r.results[0];
      const planned = best && !plan.trainee ? await planPhase(id, job, found) : { state: 'none' };
      if (planned === null) return; // a newer edit took over
      // the answer: the plan's pick (the best odds when no plan ran or it failed), and the veteran you got, drawn once
      // from it (a comparison on screen was for the old answer); a plan made becomes the reference (`refFor`)
      showAnswer(
        { status: searched, last: { r: m.r, job, key, plan: places }, planState: planned },
        planned.state === 'done' ? { base: refBase(job), places: JSON.parse(places) } : null,
      );
      if (planned.state === 'done' && !planned.r.robust) checkFor = planned;
      if (searched.kind === 'done' && planned.state !== 'error') {
        keepAnswer(answerKey);
        // kept across a reload too: the same setup draws it at once (`preloadAnswer`)
        answerSave(ERA, `${stateKey()}\n${answerKey}`, answer.peek());
      }
      const r = shown();
      showAnalysis(r);
      if (best && !job.trainees.length && !job.fix.length) keepBaseline(baseKey(job), best);
    }
  } catch (e) {
    if (id !== run.id) return;
    fail(e.message);
  }
  runEnds();
  run.inflight = null;
  clearInterval(run.tick);
  renderAll();
  paint();
  if (checkFor) checkPhase(checkDeps(id), checkFor);
}
// what the robustness check of run `id` needs of the page (career/run.ts `checkPhase`): its report shows in the plan's
// table only, the tree and the veterans staying as they are (a redraw of the open veterans dialog under a click in
// progress would lose the click, or land it on the dialog, which closes it); a plan Compare kept for the same job is the
// same report, so it takes the check too (`reviseKeptPlan`)
/** @type {(id: number) => import('./run.ts').CheckDeps} */
const checkDeps = (id) => ({
  runIsCurrent: () => id === run.id,
  // (the engine posts the job, a copy: it writes nothing into it)
  runCheck: (job) =>
    Promise.race([runJob(/** @type {CareerJob} */ (job), undefined, true), nextKill().then(() => null)]),
  redraw: () => {
    keepFocus(renderResults);
    paint();
  },
  checked: (plan) => reviseKeptPlan(plan.job, plan.r),
});
// an unfinished check of the plan on show, resumed where its answer is shown again (`checkPhase`; a check cut short by
// an edit and its undoing, or by a newer answer): nothing where it is done or unavailable
function resumeCheck(id) {
  const { planState } = answer.value;
  if (planState.state !== 'done' || planState.r.check_unavailable || checkComplete(planState.r.robust)) return;
  checkPhase(checkDeps(id), planState);
}
// a run that ends without searching (the job can't be built): its message, no answer
/** @param {Answer} next */
function endRun(next) {
  runEndsWith(next);
  clearInterval(run.tick);
  clearAnalysis();
  renderAll();
  paint();
}
// the search's lead line: how many careers it weighed, in what time
function searchedText(m) {
  const n = m.r.candidates,
    seconds = (m.ms / 1000).toFixed(1);
  return n > 1 ? `Weighed ${n.toLocaleString()} careers in ${seconds} s.` : `Scored in ${seconds} s.`;
}
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
// The plan (solver: career/plan/mod.rs `index_rule`, exact odds, no simulation): the same job with `next`, every owned uma's
// best career scored by the careers it saves toward a fully looped parent (the cheapest trio). The page reads `next`
// alone, so the solver skips the career reports. A trainee you chose leaves it nothing to pick (no plan phase).
// Returns the plan's state, or null when a newer edit (run `id` no longer current) took over.
async function planPhase(id, job, found) {
  planStarts(found, performance.now());
  paint();
  const { rollout: _r, p_now: _p, ...plain } = job;
  let planned;
  try {
    // Compare may have run this very plan job already (`keptPlan`): take its answer instead of planning again
    const jp = { ...plain, next: { ...PLAN_NEXT }, no_reports: true },
      kept = keptPlan(planKey(jp));
    /** @type {{r: {next?: DeepReadonly<PlanReport>, error?: string}, ms: number}} */
    const m2 = kept
      ? { r: { next: kept.r }, ms: kept.ms }
      : await runJob(jp, (p) => {
          if (id === run.id) {
            planProgresses(progText(p));
            paint();
          }
        });
    if (id !== run.id) return null;
    const x = m2.r.next;
    planned =
      x && !x.error && x.options?.length
        ? { state: 'done', id: newPlanId(), r: x, ms: m2.ms, job: jp }
        : { state: 'error', text: m2.r.error || x?.error || 'no option found.' };
  } catch (e) {
    if (id !== run.id) return null;
    planned = { state: 'error', text: e.message };
  }
  if (planned.state === 'done') engineSaveState(); // the worker's kept work, for the next page load
  return planned;
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
    schedule();
    renderAll();
    paintFold();
  });
}
// ---------- start ----------
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
      schedule();
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
    await preloadAnswer(); // the last visit's answer, when nothing changed since: drawn at once
    search();
  },
  (e) => {
    loadFails(e.message);
    paint();
  },
);
