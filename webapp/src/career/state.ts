// ---------- the career answer store: the run on show and the answer, each a record in a signal ----------
// `runView` is what the page draws of the run under way (the status line, the dimmed answer); `answer` is the answer on
// show (the search's status, its result, the plan); `reference` is the last plan made, whose table stays while you weigh
// a career you chose. Each is handed out read-only; the transitions below build the next record copy-on-write and
// assign it (nothing here is saved: statestore.js keeps the answer, the run machine calls it). The run machine's
// tokens, timers and the job in flight are the run machine's closure state (career/run.ts `createRun`), not here:
// nothing draws them.
// A plan is known by its id (`PlanDone.id`, given when it is made or loaded), never by its object: its check's writes
// replace the record (`setPlanCheck`), and reach every place that keeps the plan (the answers cache, `reference`).
// Records are frozen as they come in, except the jobs: a job holds the veterans' own arrays (veterans/store.ts `legaciesJob`)
// until the veterans store copies on write. Nothing here subscribes: the entry makes the effects (store.ts `on`).
import { batch, computed, signal } from '@preact/signals-core';
import type { ReadonlySignal } from '@preact/signals-core';
import { must } from '../must.ts';
import { deepFreeze } from '../store.ts';
import type { DeepReadonly } from '../store.ts';
import type { CareerJob, CareerSearch, PlanReport } from './job.js';
import type { PlacesRecord } from '../veterans/store.ts';
import { planKey } from './keys.js';

/** what the page draws of the run under way */
export interface RunView {
  /** a run is at work (from the edit, through its pause, to its answer) */
  searching: boolean;
  /** a job of this run is under way (the pause before it dims nothing: an edit that ends where it started runs none) */
  started: boolean;
  /** the phase at work */
  phase: 'search' | 'plan';
  /** when the phase began (performance.now()) */
  phaseStart: number;
  /** the plan phase's lead: what the search found ("Weighed N careers in X s.") */
  lead: string;
  /** the plan's progress, as the solver reports it from inside the plan (career/run.ts `progText`) */
  progress: string;
}
/** The status line's message once no run is at work. */
export interface Status {
  kind: 'loading' | 'done' | 'empty' | 'error';
  text: string;
  /** a button after the text (e.g. Try sample veterans), or several */
  act?: { label: string; run: () => void } | { label: string; run: () => void }[] | null;
}
/** The search's result for a job and the places it ran with (the plan's is the answer's `planState`). */
export interface Searched {
  /** the solver's search report */
  r: CareerSearch;
  /** the job it answers */
  job: CareerJob;
  /** career/keys.js `jobKey(job)` */
  key: string;
  /** the places, as JSON (`plan` when it ran) */
  plan: string;
}
// The answer's plan: none (the trainee chosen, or no run), done (`r`: the solver's `next` report; `job`: the plan job
// it ran; `id`: the plan's, `newPlanId`), or failed.
export interface PlanNone {
  state: 'none';
  r?: null;
}
export interface PlanDone {
  state: 'done';
  id: number;
  r: PlanReport;
  ms: number;
  job: CareerJob;
}
export interface PlanFailed {
  state: 'error';
  text: string;
}
export type PlanState = PlanNone | PlanDone | PlanFailed;
/** the answer on show: the status line's message, the search's result (null: none), the plan */
export interface Answer {
  status: Status;
  last: Searched | null;
  planState: PlanState;
}
/** the last plan made: `base`, career/job.js `refBase` of its job; `places`, as it ran with; `result`, its `next` report;
 *  `planId`, its plan's */
export interface Reference {
  base: string;
  places: PlacesRecord;
  result: PlanReport;
  planId: number;
}
export type AnswerView = DeepReadonly<Answer>;
type Check = NonNullable<PlanReport['robust']>;

// ---------- the signals
const view = signal<DeepReadonly<RunView>>(
  deepFreeze({ searching: false, started: false, phase: 'search', phaseStart: 0, lead: '', progress: '' }),
);
export const runView: ReadonlySignal<DeepReadonly<RunView>> = view;
const NO_PLAN: PlanNone = { state: 'none', r: null };
const shownAnswer = signal<AnswerView>(
  deepFreeze({ status: { kind: 'loading', text: 'Loading…' }, last: null, planState: NO_PLAN }),
);
export const answer: ReadonlySignal<AnswerView> = shownAnswer;
const lastPlan = signal<DeepReadonly<Reference> | null>(null);
export const reference: ReadonlySignal<DeepReadonly<Reference> | null> = lastPlan;
// the last few answers, by job and places (career/run.ts's `runKey`), the most recent last (not a signal: nothing draws
// it)
const answers = new Map<string, AnswerView>(),
  ANSWERS = 4;
/** the career the answer shows, the places it ran with aside (career/job.js `shown` holds it to the places on screen): the
 *  plan's pick once it is made, else the highest odds; null without a result */
export const answerCareer = computed(() => {
  const { last, planState } = shownAnswer.value,
    best = last?.r.results[0];
  if (!best) return null;
  return (planState.state === 'done' && planState.r.choice) || best;
});

// the answer `next` frozen in place, all but its jobs (the header says why), and returned: the run machine's own
// references to its parts (the plan it checks, the baseline it keeps) are frozen with it
function freezeInPlace(next: AnswerView): AnswerView {
  deepFreeze(next.status);
  if (next.last) deepFreeze(next.last.r);
  if (next.planState.state === 'done') deepFreeze(next.planState.r);
  for (const record of [next.last, next.planState, next]) if (record) Object.freeze(record);
  return next;
}

// ---------- the run view's transitions (the run machine's phases)
const runPatch = (patch: Partial<RunView>) => {
  view.value = deepFreeze({ ...view.peek(), ...patch });
};
/** an edit: a run waits out the pause before its search (nothing runs yet) */
export const runWaits = (now: number) =>
  runPatch({ searching: true, started: false, phase: 'search', lead: '', phaseStart: now });
/** the search's job is sent */
export const searchStarts = (now: number) =>
  runPatch({ searching: true, started: true, phase: 'search', lead: '', progress: '', phaseStart: now });
/** the plan's job is sent, after the search found `lead` */
export const planStarts = (lead: string, now: number) =>
  runPatch({ phase: 'plan', lead, progress: '', phaseStart: now });
/** the plan reports its progress */
export const planProgresses = (progress: string) => runPatch({ progress });
/** the run is over: its answer is drawn, or one on show stands */
export const runEnds = () => runPatch({ searching: false });

// ---------- the answer's transitions
/** the search's answer; a plan it made becomes the `reference`, with `where` it was made (`base`, `places`: given
 *  with a plan, null without) */
export function showAnswer(next: AnswerView, where: Pick<Reference, 'base' | 'places'> | null) {
  const shownNext = freezeInPlace(next),
    plan = shownNext.planState;
  batch(() => {
    shownAnswer.value = shownNext;
    if (plan.state === 'done' && where)
      lastPlan.value = Object.freeze({ ...where, places: deepFreeze(where.places), result: plan.r, planId: plan.id });
  });
}
/** the run ends unsearched (the job can't be built), with `next` (its message, no result) */
export function runEndsUnsearched(next: AnswerView) {
  const shownNext = freezeInPlace(next);
  batch(() => {
    runEnds();
    shownAnswer.value = shownNext;
  });
}
/** a run's search failed: `text` says why */
export function answerFails(text: string) {
  shownAnswer.value = freezeInPlace({ status: { kind: 'error', text }, last: null, planState: NO_PLAN });
}
/** veterans were added or removed: a result names parents by their place in the list, so it goes (the status stays) */
export function dropAnswer() {
  shownAnswer.value = freezeInPlace({ ...shownAnswer.peek(), last: null, planState: NO_PLAN });
}
/** the page could not load: `text` says why */
export function loadFails(text: string) {
  shownAnswer.value = freezeInPlace({ ...shownAnswer.peek(), status: { kind: 'error', text } });
}

// ---------- plans: their ids and their check
let planIds = 0;
/** an id for a plan made or loaded now (unique on this page: an id stored by an earlier visit is replaced on load) */
export const newPlanId = () => ++planIds;
/** the plan on show when it is plan `id`, else null */
export function planOnShow(id: number) {
  const plan = shownAnswer.peek().planState;
  return plan.state === 'done' && plan.id === id ? plan : null;
}
// plan `id` (on show) with its report replaced by `revise`, wherever it is kept: the answer on show, the answers cache,
// the reference. Returns the plan's new record.
function revisePlan(id: number, revise: (r: DeepReadonly<PlanReport>) => DeepReadonly<PlanReport>) {
  const plan = must(planOnShow(id), 'a plan is revised while it is on show');
  const r = Object.freeze(revise(plan.r)),
    planState = Object.freeze({ ...plan, r });
  for (const [key, kept] of answers)
    if (kept.planState.state === 'done' && kept.planState.id === id)
      answers.set(key, Object.freeze({ ...kept, planState }));
  const ref = lastPlan.peek();
  batch(() => {
    shownAnswer.value = Object.freeze({ ...shownAnswer.peek(), planState });
    if (ref?.planId === id) lastPlan.value = Object.freeze({ ...ref, result: r });
  });
  return planState;
}
/** plan `id`'s robustness check, as far as it has run (career/run.ts `checkPhase`) */
export const setPlanCheck = (id: number, check: Check) => revisePlan(id, (r) => ({ ...r, robust: deepFreeze(check) }));
/** plan `id`'s check could not finish: the worker no longer keeps the plan */
export const checkUnavailable = (id: number) => revisePlan(id, (r) => ({ ...r, check_unavailable: true }));

// ---------- the answers kept: going back to one draws it again without a run
/** the answer on show, kept under `key` (the oldest kept goes past ANSWERS) */
export function keepAnswer(key: string) {
  answers.set(key, shownAnswer.peek());
  if (answers.size > ANSWERS) answers.delete(answers.keys().next().value as string);
}
/** the answer kept under `key`, now the most recent; null when none is */
export function recallAnswer(key: string) {
  const hit = answers.get(key);
  if (!hit) return null;
  answers.delete(key);
  answers.set(key, hit);
  return hit;
}
/** an answer kept since the last visit (statestore.js), under `key`: its plan gets an id of this page's */
export function keepSavedAnswer(key: string, saved: AnswerView) {
  const planState = saved.planState.state === 'done' ? { ...saved.planState, id: newPlanId() } : saved.planState;
  answers.set(key, freezeInPlace({ ...saved, planState }));
}
/** a kept answer drawn again: it is shown, and the run is over */
export function showKeptAnswer(hit: AnswerView) {
  batch(() => {
    shownAnswer.value = hit;
    runEnds();
  });
}

// ---------- the plan Compare kept
// the refit plan Compare ran for the pool with her, when today's plan had no V or no fit to price her under
// (analysis/compare.ts `runComparison`): the same job a run sends once she is added (keys sorted, the comparison's flag aside),
// so Add to veterans takes it as it is instead of planning again (the solver is deterministic for its job). A plan
// under today's fit is never kept here: its job carries `next.fit`, which a run's does not. (Not a signal: nothing
// draws it.)
let comparisonPlan: { key: string; r: DeepReadonly<PlanReport>; ms: number } | null = null;
/** Compare's plan of plan job `job`, made in `ms` */
export function keepComparisonPlan(job: DeepReadonly<CareerJob>, r: DeepReadonly<PlanReport>, ms: number) {
  comparisonPlan = { key: planKey(job), r, ms };
}
/** the plan Compare ran for plan job key `key` (keys.js `planKey`), else null */
export const keptPlan = (key: string) => (comparisonPlan && comparisonPlan.key === key ? comparisonPlan : null);
/** the plan of job `job` (a run's) has had its check (career/run.ts) write `r`: where it is the plan Compare kept (the
 *  run took it as it was, `keptPlan`), the two are one report, so the kept plan is `r` from now on (a run that takes
 *  it again finds its check done) */
export function reviseKeptPlan(job: DeepReadonly<CareerJob>, r: DeepReadonly<PlanReport>) {
  if (comparisonPlan && comparisonPlan.key === planKey(job)) comparisonPlan = { ...comparisonPlan, r };
}
