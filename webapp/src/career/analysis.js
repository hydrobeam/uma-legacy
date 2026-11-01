// The veteran analysis (the veteran you got): career.js's "Try a run" popup. The decision is per veteran: enter the one
// you got (she starts as a random result, drawn as Reroll draws, career/roll.mjs) and Compare runs both choices,
// running this career again or using her in the next one, on the plan's terms (expected careers to land).
// This module owns the analysis' state (`simulation`, the comparison under way `simulationId`, the comparisons'
// baseline `baselineCache`, the plan Compare kept `planKept`); career.js changes it only through the transitions
// below. It reads the answer and the run machine from career.js (an import cycle: this module's body only declares,
// so nothing of career.js is read while the bundle loads).
import { $, $dialog, el, fmtIn } from '../dom.js';
import { careerTables } from '../data.js';
import { state } from '../state.js';
import { restartEngine, engineSaveState } from '../engine.js';
import { umaAv, runnerHTML } from '../images.js';
import { pct } from '../odds.js';
import { vets, plan, changed, newVet, umaName, openWins } from '../vets.js';
import { detailsEl, fitSparks } from '../details.js';
import { scheduledRaces, rolledOutcome } from './roll.mjs';
import {
  last,
  planState,
  shown,
  run,
  runJob,
  keyOf,
  isFull,
  baseKey,
  planKey,
  PLAN_NEXT,
  ROLL_K,
  btn,
  sectionEl,
  deckEl,
  fmtN,
  tableRows,
} from '../career.js';
import { must } from '../must.ts';
import { openNode } from './view/Node.tsx';
/** @import { LineageNode } from '../vets.js' */
/** @import { CareerJob, CareerReport, CareerSearch, PlanDone, PlanOption, PlanReport } from '../career.js' */
/** @typedef {{results: CareerReport[], pending: true}} PendingSearch a stand-in while the quick comparison looks for the best career with her */

// the refit plan Compare ran for the pool with her, when today's plan had no V or no fit to price her under
// (`runComparison`):
// the same job a run sends once she is added (keys sorted, the comparison's flag aside), so Add to veterans takes it as
// it is instead of planning again (the solver is deterministic for its job). A plan under today's fit is never kept
// here: its job carries `next.fit`, which a run's does not
/** @type {{key: string, r: PlanReport, ms: number} | null} */
let planKept = null;

/**
 * The veteran analysis: the veteran this career leaves (the draft) and her comparison.
 * @typedef {object} Simulation
 * @property {'none'|'ready'|'busy'|'done'|'error'} state  none: no career on show; ready: no comparison yet
 * @property {Draft|null} [draft]  the entered veteran
 * @property {CareerSearch|PendingSearch|null} [r]  the comparison's search: the best career with her as a parent (`pending` while the
 *   quick comparison looks for it)
 * @property {CareerReport|null} [baseline]  today's best career, without her
 * @property {string|null} [error]
 * @property {string} [forKey]  `keyOf` the career she came from
 * @property {'quick'|null} [kind]  the running comparison is a roll's quick one
 * @property {boolean} [stale]  edited since the figures on show
 * @property {boolean} [refining]  an exact comparison runs behind a roll's quick figures
 * @property {boolean} [provisional]  the quick figures say no saving, but their bound allows one: checking
 * @property {boolean} [standing]  a roll's quick figures say it saves careers: the exact comparison starts once the
 *   roll has stood 0.8 s (shown as checking meanwhile)
 * @property {boolean} [quick]  the figures on show are the quick ones
 * @property {boolean} [full]  she lands every target
 * @property {{now: number|null, withHer: number|null}} [v0]  expected careers to a fully looped parent with your
 *   veterans now, and with her too
 * @property {boolean} [fixedFit]  with her is priced under today's plan's fit (Add to veterans then fits the plan to
 *   the pool with her, so its figures can differ); false: the refit itself (today's plan had no V or no fit)
 * @property {PlanOption|null} [pickA]  the planner's next career once she is added
 * @property {boolean} [samePick]  it is the same as without her
 * @property {boolean} [herParent]  she is one of its parents
 * @property {string} [where]  where her saving lands (`savedWhere`)
 *
 * The veteran a career leaves, as entered (a Veteran without id), with `apt` (her end grades) and `bought` (skill group
 * -> 'gold' | 'white') from her roll.
 * @typedef {LineageNode & {borrowed: boolean, parents: LineageNode[], apt?: number[] | null, bought?: Object<number, string>}} Draft
 */
/** @type {Simulation} */
let simulation = { state: 'none', draft: null, r: null, baseline: null, error: null };
let simulationId = 0; // the comparison under way; a newer one (or a run) makes it stale
/** @type {{key: string, result: CareerReport} | null} key: baseKey(job); result: today's best career, the comparisons' baseline */
let baselineCache = null;

// ---------- the transitions the run machine (career.js) makes
// a new run: the comparison under way, if any, is stale (its answer is ignored)
export function outdateComparison() {
  simulationId++;
}
// an edit: the comparison in flight stops, its panel back to idle, or a roll's quick figures stand unchecked (an answer
// that stands keeps her)
export function stopComparison() {
  simulationId++;
  if (simulation.state === 'busy') Object.assign(simulation, { state: 'ready', r: null, kind: null });
  Object.assign(simulation, { refining: false, provisional: false, standing: false });
}
// an answer shows `r` (`simulationFor`)
export function showAnalysis(r) {
  simulation = simulationFor(r);
}
// no answer: no career to analyse
export function clearAnalysis() {
  simulation = { state: 'none' };
}
// a run searched today's best career (no place set): the comparisons' baseline (`baselineFor`), under `key` (`baseKey`)
export function keepBaseline(key, result) {
  baselineCache = { key, result };
}
// the plan Compare ran for the pool with her, when a run's plan job is the same (`key`: `planKey`), else null
export const keptPlan = (key) => (planKept && planKept.key === key ? planKept : null);

// ---------- the analysis
function comparisonJob() {
  return JSON.parse(baseKey(must(last, 'a comparison is of the career on show').job));
}

// the veteran analysis, ready for the career on show: she starts as a random result of it
/** @type {(r: CareerReport) => Simulation} */
const readySimulation = (r) => ({
  state: 'ready',
  draft: draftOf(r),
  r: null,
  baseline: null,
  error: null,
  forKey: keyOf(r),
});
// the veteran analysis once an answer shows `r`: a fresh one, except that the same career keeps the veteran entered for
// it (an edit elsewhere, or its undoing, leaves what the player typed); her comparison starts over on the new pool
/** @type {(r: CareerReport|null) => Simulation} */
const simulationFor = (r) => {
  if (!r) return { state: 'none' };
  const fresh = readySimulation(r);
  return simulation.draft && simulation.forKey === fresh.forKey ? { ...fresh, draft: simulation.draft } : fresh;
};
// today's best career without her, which a roll's figures and a comparison's both set hers against: the run's when it
// searched without places (a fixed trainee or parent makes it the best of those), else cached or searched now
/** @param {CareerJob} base `comparisonJob()` @returns {Promise<CareerReport>} */
async function baselineFor(base) {
  const shownNow = must(last, 'a comparison is of the career on show');
  const key = JSON.stringify(base),
    open = !shownNow.job.trainees.length && !shownNow.job.fix.length,
    cached = baselineCache?.key === key ? baselineCache.result : null;
  /** @type {{results: CareerReport[], error?: string}} */
  const before = cached ? { results: [cached] } : open ? shownNow.r : (await runJob(base)).r;
  if (before.error || !before.results?.length)
    throw new Error(before.error || 'No career found in the current roster.');
  baselineCache = { key, result: before.results[0] };
  return before.results[0];
}

// the veteran analysis' G1 wins: the G1 wins editor of the pool (a win in its slot, click to remove; an empty slot lists the
// G1s she could enter then). Her schedule's races keep their win odds in the turn bar; a win shows at the turn picked for
// it, else the turn her schedule runs it. A race held in Classic and Senior is one win saddle (one race index).
function openDraftWins(d, r) {
  // the G1s she could enter, by turn (heldAt), and those her schedule runs, by race (runs: the first turn and its cell)
  const rows = tableRows(r),
    runs = new Map(),
    heldAt = new Map();
  for (const w of rows) {
    const ix = careerTables().races.indexOf(w.name || w.race);
    if (ix < 0) continue;
    if (!heldAt.has(w.turn)) heldAt.set(w.turn, []);
    heldAt.get(w.turn).push(ix);
    if (w.run !== false && !runs.has(ix)) runs.set(ix, { turn: w.turn, cell: w.cells[0] });
  }
  const parentWon = (ix) => d.parents.some((p) => p.wins.includes(ix));
  openWins(d, umaName(d.chara), {
    offer: rows.length ? (t) => heldAt.get(t) || [] : null,
    info: (ix) => {
      const x = runs.get(ix);
      return x || parentWon(ix) ? { ...(x || {}), parent: parentWon(ix) } : null;
    },
    onEdit: editDraft,
    onClose: () => $('sim-wins')?.focus(),
  }); // the button was redrawn by the edits
}
// a career as portraits: trainee (outfit), then her two parents; parents: [{chara, outfit, label, her}]
/**
 * @param {number} tid @param {number | undefined} outfit
 * @param {{chara: number | null, outfit?: number, label: string, her?: boolean}[]} parents
 * @param {string | null} [outfitLabel]
 */
function facesEl(tid, outfit, parents, outfitLabel = null) {
  const box = el('span', 'cfaces');
  const t = el('span', 'cface tr');
  t.append(umaAv(tid, outfit, 32), umaName(tid) + (outfitLabel ? ` ${outfitLabel}` : ''));
  box.appendChild(t);
  box.appendChild(el('span', 'cwith', 'with'));
  parents.forEach((p, j) => {
    const f = el('span', 'cface');
    f.append(umaAv(p.chara, p.outfit, 26), p.label);
    if (p.her) f.appendChild(el('span', 'her', 'her'));
    box.appendChild(f);
    if (!j) box.appendChild(el('span', 'cwith', '+'));
  });
  return box;
}
// a parent of a future career by legacy index (index = vets.length: the veteran she leaves, `who`)
const parentFace = (i, who) =>
  i >= vets.length
    ? { chara: who.chara, outfit: who.outfit, label: umaName(who.chara), her: true }
    : { chara: vets[i].chara, outfit: vets[i].outfit, label: `#${i + 1} ${umaName(vets[i].chara)}` };
const copyNode = (v) => ({
  chara: v.chara,
  ...(v.outfit ? { outfit: v.outfit } : {}),
  sparks: { ...v.sparks },
  pinks: { ...v.pinks },
  blues: { ...(v.blues || {}) },
  greens: { ...(v.greens || {}) },
  wins: [...v.wins],
});
// what a roll reads of the settings and data (career/roll.mjs)
/** @type {() => import('./roll.mjs').RollSetup} */
const rollSetup = () => ({ career: careerTables(), targets: state.targets, rank: state.rank });
function rollDraft() {
  const r = shown();
  if (!r || run.searching) return;
  Object.assign(must(simulation.draft, 'a roll redraws the entered veteran'), rolledOutcome(r, rollSetup()), {
    winAt: {},
  });
  simulation.r = null;
  simulation.stale = false;
  const request = rollReq(simulation.draft, r);
  if (request == null) {
    compare();
    return;
  }
  cancelCompare();
  quickCompare(request);
}
// A roll's careers saved, asked of the plan for the career on show (`roll`: the plan job and her stars per target; the
// solver evaluates the plan's pick at that one outcome from what it kept of the plan, career/plan/roll.rs `roll_kept`, so no
// plan runs again unless the worker kept none): y, exact for her sparks, her G1 wins at their chances, over the plan's
// routes. It is no verdict: a route the plan has not (another lineage of a candidate's
// whose last career she improves as a grandparent, without lifting the candidate's odds) can save careers too, so
// every roll is confirmed by the exact comparison, its quick figure shown meanwhile as provisional. null when the plan
// isn't for this career (then Reroll compares exactly).
function rollReq(d, r) {
  if (planState.state !== 'done') return null;
  const o = planState.r?.outcomes;
  if (!o || !planState.job || (o.career ?? `${o.trainee}:${o.parents.join(',')}`) !== keyOf(r)) return null;
  const sp = d.sparks || {};
  return { job: planState.job, stars: state.targets.map((t) => sp[t.group] || 0) };
}
// the reroll's comparison, in two solver calls: the roll's saving from the plan the worker keeps (`next.roll`, about a
// millisecond), shown at once as Careers to loop; then the best career with her as a parent (one quick search, most of
// a second), which fills in her odds and the verdict's "Better odds". A reroll meanwhile waits behind that search (the
// worker keeps it and its caches; its answer is ignored)
async function quickCompare(request) {
  const id = ++simulationId,
    source = run.id,
    gone = () => id !== simulationId || source !== run.id;
  simulation.state = 'busy';
  simulation.kind = 'quick';
  simulation.error = null;
  simulation.stale = false;
  simulation.provisional = false;
  simulation.standing = false;
  renderSimulation();
  /** @type {number | null} the roll's saving (careers), from the plan's routes (`rollReq`) */
  let y = null;
  // her legacy index is after every veteran's
  const base = comparisonJob(),
    legaciesWithHer = [...base.legacies, draftJob(simulation.draft)],
    herIndex = base.legacies.length;
  try {
    if (!simulation.baseline) {
      const before = await baselineFor(base);
      if (gone()) return;
      simulation.baseline = before;
    }
    const m = await runJob({ ...request.job, next: { ...request.job.next, roll: request.stars } });
    if (gone()) return;
    // (no saving from the plan: the exact comparison answers instead)
    const roll = m.r.next?.roll;
    if (m.r.error || typeof roll?.y !== 'number') {
      simulation.kind = null;
      runComparison();
      return;
    }
    y = roll.y;
    const v0 = /** @type {PlanDone} */ (planState).r.v0; // done: `request` came from its `rollReq`
    simulation.r = { results: [], pending: true };
    simulation.full = isFull(simulation.draft);
    simulation.v0 = { now: v0, withHer: v0 != null ? Math.max(0, v0 - y) : null }; // r: a stand-in, no search figures
    simulation.fixedFit = true; // (y is the plan's own scoring, under its fit)
    simulation.pickA = null;
    simulation.samePick = false;
    simulation.herParent = false;
    simulation.where = '';
    simulation.quick = true;
    simulation.state = 'done';
  } catch (e) {
    if (gone()) return;
    simulation.error = e.message;
    simulation.state = 'error';
  }
  simulation.kind = null;
  renderSimulation();
  if (simulation.state !== 'done' || gone()) return;
  // a roll that saves careers goes to the exact comparison (which finds the best career with her too) once it has stood
  // 0.8 s, as a zero does (rolling on starts none, so no running check is stopped, the worker with it): the plan's
  // saving can be off either way (it prices her in its own routes only: a few partners, one deck per lifted side, one C
  // per route), so the exact count replaces it, shown as checking meanwhile; a zero is checked too, after the best
  // career with her
  if (must(y, "a roll's done figures have its saving") >= 0.5) {
    simulation.standing = true;
    renderSimulation();
    setTimeout(() => {
      if (id === simulationId && source === run.id && simulation.standing && !simulation.refining) {
        simulation.standing = false;
        runComparison(true);
      }
    }, 800);
    return;
  }
  try {
    const m = await runJob({ ...base, fix: [herIndex], legacies: legaciesWithHer, top: 1, compare: true });
    if (gone()) return;
    if (m.r.error || !m.r.results?.length) throw new Error(m.r.error || 'No next career found.');
    simulation.r = m.r;
  } catch (e) {
    if (gone()) return;
    simulation.error = e.message;
    simulation.state = 'error';
  }
  renderSimulation();
  // a zero is checked by the exact comparison too (the plan's routes are not every route a replan can take), shown
  // meanwhile as provisional
  // (after the roll has stood a moment: rolling on starts none; a running check would be stopped, the worker with it)
  if (simulation.state === 'done' && !gone()) {
    simulation.provisional = true;
    renderSimulation();
    setTimeout(() => {
      if (id === simulationId && source === run.id && simulation.provisional && !simulation.refining)
        runComparison(true);
    }, 800);
  }
}
// A comparison in flight is for a veteran that is going (a roll, a reset, Compare pressed again): stop it. A wasm call
// can't be interrupted, so the worker goes (a fresh one starts loading at once). (No run is in flight: they wait for the answer.)
function cancelCompare() {
  if (simulation.refining) {
    simulationId++;
    simulation.refining = false;
    simulation.provisional = false;
    simulation.standing = false;
    restartEngine();
    return;
  }
  if (simulation.state !== 'busy') return;
  simulationId++;
  simulation.state = 'ready';
  if (simulation.kind === 'quick') return; // one quick search: it finishes, its answer ignored (the worker and its caches stay)
  restartEngine();
}
// where a comparison's saving lands: the next career when she is one of its parents; else a later career, named (the
// step of V's route, A, B or C, whose careers fell most) only where both plans' cheapest routes are the same trio, on
// the same sides (`sides`: her real veteran, or her best career now or another career of hers, each on its parents: a
// side whose parents changed is another route). The
// cheapest trios are often near-tied (`trios`, `near`), so between two different ones a per-uma step comparison names
// an arbitrary uma; what falls then is V, which the panel shows. planNow / planWithHer: the plans' `next` reports;
// pickA: the planner's next career with her; samePick: it is the one without her
function savedWhere({ planNow, planWithHer, pickA, samePick, herIndex }) {
  if (!pickA) return '';
  if (pickA.parents.includes(herIndex)) return `Improves the next career (${pickA.trainee.name}).`;
  const trioNow = planNow?.v0_plan,
    trioWithHer = planWithHer?.v0_plan;
  // (A and B are ordered by their careers, so the same trio may list them swapped; each side by its uma and its veteran,
  // `sides`: the same names on other veterans, or on other parents, are another route, whose per-step careers are not
  // comparable)
  /** @type {(v: NonNullable<PlanReport['v0_plan']>) => string} */
  const sidesOf = (v) =>
    [v.a, v.b]
      .map((trainee, i) => `${trainee}|${v.sides?.[i] ?? '?'}`)
      .sort()
      .join();
  const sameTrio =
    !!trioNow &&
    !!trioWithHer &&
    !!trioNow.sides &&
    !!trioWithHer.sides &&
    trioNow.c === trioWithHer.c &&
    sidesOf(trioNow) === sidesOf(trioWithHer);
  // the route's steps: [the uma of the step, its careers]
  /** @type {(v: NonNullable<PlanReport['v0_plan']>) => [string|null, number][]} */
  const steps = (v) => [v.a, v.b, v.c].map((trainee, i) => [trainee, v.careers[i]]);
  /** @type {string | null} */
  let best = null,
    gain = 0;
  if (sameTrio) {
    const careersNow = new Map(steps(trioNow));
    for (const [trainee, careers] of steps(trioWithHer)) {
      const fell = (careersNow.get(trainee) ?? careers) - careers;
      if (trainee && fell > gain) {
        gain = fell;
        best = trainee;
      }
    }
  }
  return `Improves a later ${best ? `${best} ` : ''}career; the next career ${samePick ? 'stays' : 'is'} ${pickA.trainee.name}.`;
}
function draftOf(r) {
  return {
    chara: r.trainee.id,
    outfit: r.trainee.outfits[0]?.id,
    borrowed: false,
    ...rolledOutcome(r, rollSetup()),
    winAt: {},
    parents: r.parents.map((i) => copyNode(vets[i])),
  };
}
function draftJob(v) {
  const pairs = (o) => Object.entries(o).map(([g, s]) => [+g, s]);
  const node = (v) => ({ chara: v.chara || 0, sparks: pairs(v.sparks), pinks: pairs(v.pinks), wins: [...v.wins] });
  return { ...node(v), borrowed: false, parents: v.parents.map(node) };
}
// an edit to the entered veteran: the comparison on screen is for the old one
function editDraft() {
  if (simulation.r) simulation.stale = true;
  renderSimulation();
}
// Compare (or a roll): a press while a comparison runs restarts it with the veteran as she is now
function compare() {
  if (run.searching || !shown()) return;
  cancelCompare();
  runComparison();
}
// refine: a roll's quick result stays on screen (an improvement shown at once, its figures held) while this works out
// the exact count; otherwise the panel waits on it
async function runComparison(refine = false) {
  const id = ++simulationId,
    source = run.id,
    gone = () => id !== simulationId || source !== run.id;
  const base = comparisonJob();
  if (refine) simulation.refining = true;
  else {
    simulation.state = 'busy';
    simulation.refining = false;
    simulation.provisional = false;
    simulation.standing = false;
  }
  simulation.error = null;
  simulation.stale = false;
  renderSimulation();
  try {
    if (!simulation.baseline) {
      const before = await baselineFor(base);
      if (gone()) return;
      simulation.baseline = before;
    }
    // the planner's question (exact odds, estimated careers): the best career with her as a parent, and the planner's
    // expected careers to a fully looped parent (each plan's `v0`, V) from the veterans now, without and with her
    const legaciesWithHer = [...base.legacies, draftJob(simulation.draft)],
      herIndex = base.legacies.length;
    const m = await runJob({ ...base, fix: [herIndex], legacies: legaciesWithHer, top: 1, compare: true });
    if (gone()) return;
    if (m.r.error || !m.r.results?.length) throw new Error(m.r.error || 'No next career found.');
    const full = isFull(simulation.draft);
    const planOf = (legacies) => ({
      ...base,
      top: ROLL_K,
      legacies,
      next: { ...PLAN_NEXT },
      compare: true,
      no_reports: true,
    }); // as a run's plan job, once she is added
    let planNow =
      planState.state === 'done' && !must(last, 'a comparison is of the career on show').job.fix.length
        ? planState.r
        : null;
    if (!planNow) {
      const mb = await runJob(planOf(base.legacies));
      if (gone()) return;
      planNow = mb.r.next || null;
    }
    // with her: her pool priced under today's plan's fit (`next.fit`, passed on as the plan wrote it), so the two Vs
    // differ only by what she changes, not by Cont fitted again to the pool with her (a refit moves V by hundreds of
    // careers on a weak pool, in either direction). The plan Add to veterans shows is that refit, run on Add. Only when
    // today's plan has no V to compare with (no route: `v0` null, though its `fit` may still list candidates) or no fit
    // is the refit run here, and kept so that Add takes it as it is
    const fit = planNow && Number.isFinite(planNow.v0) && planNow.fit?.length ? planNow.fit : null,
      jobWithHer = fit ? { ...planOf(legaciesWithHer), next: { fit } } : planOf(legaciesWithHer),
      ma = await runJob(jobWithHer);
    if (gone()) return;
    const planWithHer = ma.r.next || null;
    if (!fit && planWithHer && !planWithHer.error) planKept = { key: planKey(jobWithHer), r: planWithHer, ms: ma.ms };
    engineSaveState();
    const careersToLoop = (next) => (next && next.v0 != null ? next.v0 : null); // careers to a fully looped parent with the pool as it is
    const careersNow = careersToLoop(planNow),
      pickA = planWithHer?.options?.find((o) => o.chosen) || null,
      pickB = planNow?.options?.find((o) => o.chosen) || null;
    // not clamped: in the game a veteran never costs careers (you can leave her out), but V can rise with her even
    // under the same fit: each fresh side of V's route comes from one of a candidate's `fresh_lineages` best careers
    // now (her best and her next two by cached decks, each searched on its pair), so her arrival can push the lineage
    // the route used out of them, and a pair's search that runs out of budget can stop below the pair's best. The panel
    // shows such a rise as it is, with that said (`comparisonResult`)
    const careersWithHer = careersToLoop(planWithHer);
    // the planner's next career keeps its trainee and parents (her legacy index is after them, so they keep theirs)
    const samePick = !!(pickA && pickB && keyOf(pickA) === keyOf(pickB));
    simulation.r = m.r;
    simulation.full = full;
    simulation.v0 = { now: careersNow, withHer: careersWithHer };
    simulation.fixedFit = !!fit;
    simulation.pickA = pickA;
    simulation.samePick = samePick;
    simulation.herParent = !!pickA?.parents.includes(herIndex);
    simulation.where = savedWhere({ planNow, planWithHer, pickA, samePick, herIndex });
    simulation.quick = false;
    simulation.refining = false;
    simulation.provisional = false;
    simulation.standing = false;
    simulation.state = 'done';
  } catch (e) {
    if (gone()) return;
    if (simulation.refining) {
      simulation.refining = false;
      simulation.provisional = false;
      simulation.standing = false;
    } else {
      simulation.error = e.message;
      simulation.state = 'error';
    }
  }
  renderSimulation();
}
function addDraft() {
  if (run.searching || !simulation.draft) return;
  $dialog('sim-dlg').close(); // she joins your veterans: back to the next career
  const d = structuredClone(simulation.draft);
  delete d.apt;
  delete d.bought;
  const v = newVet(d.chara, d.sparks);
  Object.assign(v, d, { src: 'entered', borrowed: false });
  plan.trainee = null;
  plan.parents = [null, null];
  simulation.draft = null; // she is in the pool now: the next answer starts a new one even for the same career
  $('v-msg').textContent =
    `Added ${umaName(v.chara)} to your veterans. The planner now picks the next career from all of them.`;
  changed();
}
// The veteran analysis: her card (details.js; she starts as a random result of this career) with its actions, and the
// comparison panel beside it.
function simulationBody() {
  const box = sectionEl('simulation', 'Veteran analysis');
  box.id = 'simulation';
  const d = simulation.draft,
    r = shown();
  if (!d || !r) return box;
  box.appendChild(analysisCard(d, r));
  const side = el('div', 'simcol');
  side.appendChild(comparisonPanel(d, r));
  box.appendChild(side);
  return box;
}
// her card, as the game's Umamusume Details screen: aptitudes at the end of this result, Skills / Inspiration (her
// sparks, editable) / Career Info (her G1s, the wins editor); in its header the career's odds and her actions
function analysisCard(d, r) {
  const wb = btn(
    `${d.wins.length ? d.wins.length : 'No'} G1 win${d.wins.length === 1 ? '' : 's'} · edit`,
    'vwins',
    () => openDraftWins(d, r),
  );
  wb.id = 'sim-wins';
  wb.dataset.k = 'sim:wins';
  wb.setAttribute('aria-haspopup', 'dialog');
  const card = detailsEl({
    d,
    outfit: r.trainee.outfits[0]?.label || null,
    apt: d.apt || r.aptitudes?.start || null,
    races: scheduledRaces(r, careerTables().races),
    raceName: (ix) => careerTables().races[ix] || `G1 ${ix}`,
    onEdit: editDraft,
    winsBtn: wb,
    targets: r.targets,
    deck: r.deck,
    legacies: r.parents.map((i) => ({
      v: vets[i],
      label: `#${i + 1}`,
      open: () => openNode({ kind: 'v', id: vets[i].id }),
    })),
    parentLabel: (j) => `#${r.parents[j] + 1}`,
  });
  const actions = el('div', 'row');
  // Compare and a roll stay usable while a comparison runs: either one restarts it (not during a run, whose
  // answer replaces the career on show)
  const go = btn('Compare', 'primary', compare);
  go.id = 'sim-compare';
  go.dataset.k = 'sim:compare';
  go.disabled = run.searching;
  const roll = btn('Reroll', '', rollDraft);
  roll.id = 'sim-roll';
  roll.dataset.k = 'sim:roll';
  roll.disabled = run.searching;
  const add = btn('Add to veterans', '', addDraft);
  add.id = 'sim-add';
  add.disabled = run.searching;
  actions.append(go, roll, add);
  const mid = el('div', 'udmid');
  mid.append(actions); // (the career's odds lead the main view)
  card.querySelector('.udtop')?.insertBefore(mid, card.querySelector('.udname'));
  return card;
}
// the panel beside her card: a mini header carrying the chance a roll of this career improves your loop (the plan's
// `lift`: her outcomes that lower V), then the comparison (before any, a line saying what goes here)
function comparisonPanel(d, r) {
  const busy = simulation.state === 'busy';
  const result = el('div', `simresult${simulation.stale ? ' stale' : ''}`);
  result.id = 'sim-result';
  result.setAttribute('role', 'status');
  const hdr = el('div', 'simhdr');
  hdr.append(el('span', '', 'Comparison'));
  const opt = planState.state === 'done' ? planState.r.options?.find((o) => keyOf(o) === keyOf(r)) : null;
  if (opt && opt.lift != null) {
    const odds = el(
      'span',
      'simchance',
      opt.lift > 0 ? `1/${Math.max(1, Math.round(1 / opt.lift)).toLocaleString()} to improve` : 'none improve',
    );
    odds.id = 'sim-chance';
    odds.title = `The planner's chance that a roll of this career lowers the careers to a fully looped parent (${pct(opt.lift)})`;
    hdr.append(odds);
  }
  result.appendChild(hdr);
  if (!busy && !simulation.error && !simulation.r)
    result.appendChild(el('p', 'simidle', 'Compare or reroll to see if she improves your loop.'));
  if (busy) {
    const p = el('p', 'runnote');
    p.innerHTML = `${runnerHTML()}<span>Comparing…</span>`;
    result.appendChild(p);
  } else if (simulation.error) result.appendChild(el('p', 'note', simulation.error));
  else if (simulation.r) comparisonResult(result, d);
  return result;
}
// what she does for the loop if you enter her: one short line (`saves`: the careers she saves, null: not known). A
// roll's quick zero is the plan's routes' only (`rollReq`), always provisional until the exact comparison answers: it
// reads "no saving found yet"
function verdictText(d, next, saves) {
  const n = state.targets.length,
    fullyLooped = simulation.full && d.parents.length === 2 && d.parents.every(isFull),
    none = simulation.quick ? 'saving found yet' : 'careers saved';
  return fullyLooped
    ? 'Fully looped parent'
    : simulation.full
      ? `${n}/${n}: step 1 done`
      : (simulation.refining || simulation.standing) && !simulation.provisional
        ? 'Checking…'
        : saves != null && saves >= 0.5
          ? `Saves ${saves.toFixed(1)} careers`
          : simulation.herParent
            ? 'Parent of the next career'
            : next &&
                next.p_all >
                  must(simulation.baseline, 'figures with her best career have a baseline').p_all * (1 + 1e-9)
              ? `Better odds, no ${none}`
              : `No ${none}`;
}
// One figure per row: its label, then now → with her, green when she makes it better, red when worse (`direction`).
function figureEl({ label, now, withHer, direction, title }) {
  const c = el('div', 'tc');
  c.title = title;
  c.append(el('span', 'tl', label), el('b', `tv ${direction}`, `${now} → ${withHer}`));
  return c;
}
// a figure's direction: 'up' (better with her), 'down' (worse) or 'same' (within the estimate's noise, or unknown)
function direction(now, withHer, lowerIsBetter) {
  if (now == null || withHer == null) return 'same';
  const gain = lowerIsBetter ? now - withHer : withHer - now;
  return Math.abs(gain) < (lowerIsBetter ? 0.5 : 1e-9 * now) ? 'same' : gain > 0 ? 'up' : 'down';
}
// the comparison on show: the verdict in one line, then the two figures it rests on (now -> with her), then the best
// career with her
function comparisonResult(result, d) {
  if (simulation.stale)
    result.appendChild(
      el('p', 'note', 'These figures are for her result before your last edit. Compare to update them.'),
    );
  const before = simulation.baseline,
    n = state.targets.length,
    { now: careersNow, withHer: careersWithHer } = simulation.v0 || { now: null, withHer: null };
  const next = must(simulation.r, 'a result is drawn for a comparison').results[0]; // (none yet while the quick search for the best career with her runs)
  const saves = careersNow != null && careersWithHer != null ? careersNow - careersWithHer : null;
  const verdict = el('p', 'simverdict');
  verdict.id = 'sim-verdict';
  const head = verdictText(d, next, saves),
    rose = saves != null && saves <= -0.5;
  verdict.append(el('strong', '', head));
  if (/^Saves /.test(head) && simulation.where) verdict.append(simulation.where);
  // a rise (never from a roll's quick figures, which can't rise): shown as it is, and said to be the planner's
  if (rose)
    verdict.append(
      `The planner's estimate rises by ${(-saves).toFixed(1)} careers with her, a limit of its route (it offers each uma's few best careers as sides, and she can push the one it used out of them); leaving her out costs nothing.`,
    );
  result.appendChild(verdict);
  if (simulation.refining || simulation.standing || simulation.provisional) {
    const p = el('p', 'runnote simrefine');
    p.innerHTML =
      runnerHTML() +
      (simulation.provisional ? '<span>Checking…</span>' : '<span>Working out how many careers it saves…</span>');
    result.appendChild(p);
  }
  const figs = el('div', `simodds${simulation.refining || simulation.standing ? ' held' : ''}`);
  const nextWithHer = simulation.pickA
    ? `. With her the planner's next career is ${simulation.pickA.trainee.name} (${simulation.pickA.parents.map((i) => `#${i + 1}`).join(' + ')}).`
    : '.';
  const fromTable = simulation.quick
    ? " From the planner's table for this career (her sparks exact, her G1 wins at their chances; only the routes the plan kept); Compare checks this exact roll."
    : '';
  figs.appendChild(
    figureEl({
      label: 'Careers to loop',
      now: careersNow != null ? `about ${fmtN(careersNow)}` : '?',
      withHer: careersWithHer != null ? fmtN(careersWithHer) : '?',
      direction: rose ? 'same' : direction(careersNow, careersWithHer, true),
      title: `Expected careers to a fully looped parent: with your veterans now → with her too${nextWithHer}${fromTable}`,
    }),
  );
  // with her priced under today's fit (`fixedFit`): the plan after Add is fitted to the pool with her, so it can show
  // other figures (more careers or fewer), which this one line says
  const fitNote =
    simulation.fixedFit && careersNow != null
      ? el(
          'p',
          'simfit',
          "Counted with today's plan; after Add the planner refits it to your veterans with her, so its figures can differ.",
        )
      : null;
  // the best career with her as a parent: its odds against today's best, then folded its portraits and deck
  if (!next) {
    result.appendChild(figs);
    if (fitNote) result.appendChild(fitNote);
    const p = el('p', 'runnote simbest');
    p.innerHTML = `${runnerHTML()}<span>Finding the best career with her…</span>`;
    result.appendChild(p);
    return;
  }
  const dd = el('details', 'nextdeck');
  dd.appendChild(el('summary', '', 'Best career with her'));
  const nx = el('div', 'nextc');
  nx.append(
    facesEl(
      next.trainee.id,
      next.trainee.outfits[0]?.id,
      next.parents.map((i) => parentFace(i, d)),
      next.trainee.outfits[0]?.label || '',
    ),
  );
  const baseline = must(before, 'figures with her best career have a baseline');
  figs.appendChild(
    figureEl({
      label: `All ${n} odds`,
      now: fmtIn(baseline.p_all),
      withHer: fmtIn(next.p_all),
      direction: direction(baseline.p_all, next.p_all, false),
      title: `All ${n}: today's best career → the best career with her as a parent (${pct(baseline.p_all)} → ${pct(next.p_all)}; ${n - 1} or more: ${fmtIn(baseline.p_minus1)} → ${fmtIn(next.p_minus1)})`,
    }),
  );
  result.appendChild(figs);
  if (fitNote) result.appendChild(fitNote);
  dd.append(nx, deckEl(next, 'Deck'));
  result.appendChild(dd);
}
export function renderSimulation() {
  const ae = /** @type {HTMLElement} */ (document.activeElement),
    k = ae?.dataset?.k,
    top = $('sim-top');
  if (top) {
    top.replaceChildren(shown() ? simulationBody() : '');
    fitSparks(top);
  }
  if (k?.startsWith('sim:') && !ae.isConnected) document.querySelector(`[data-k="${CSS.escape(k)}"]`)?.focus();
}
