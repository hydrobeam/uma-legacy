// The career run machine: `createRun(deps)` returns the functions that drive a run (an edit's pause, its search, its
// plan, the robustness check on the plan on show, the answer kept from the last visit). Its staleness tokens and timers
// are closure state: nothing draws them. What the page draws of the run and its answer is the answer store's
// (career/state.ts), changed only through the transitions `deps.store` hands in; everything else of the page (the
// solver, the clock, the job its settings and veterans make, the analysis, the redraws) comes in `deps` too, so the
// node tests run the machine on fakes. It reads no store itself (a model module: static.mjs checks it).
import type { PlanProgress } from '../solver.ts';
import type { CareerAnswer, CareerJob, CareerReport, CareerSearch, PlanReport } from './job.js';
import type { DeepReadonly } from '../store.ts';
import type * as AnswerStore from './state.ts';
import type { Answer, AnswerView, PlanDone, PlanState, Status } from './state.ts';
import { jobKey, planKey } from './keys.js';

/** the answer store's transitions and reads the machine uses (career/state.ts) */
export type RunStore = Pick<
  typeof AnswerStore,
  | 'answer'
  | 'runView'
  | 'runWaits'
  | 'searchStarts'
  | 'planStarts'
  | 'planProgresses'
  | 'runEnds'
  | 'runEndsUnsearched'
  | 'showAnswer'
  | 'showKeptAnswer'
  | 'answerFails'
  | 'newPlanId'
  | 'planOnShow'
  | 'setPlanCheck'
  | 'checkUnavailable'
  | 'keepAnswer'
  | 'recallAnswer'
  | 'keepSavedAnswer'
  | 'keptPlan'
  | 'reviseKeptPlan'
>;

/** what the run machine needs of the page, built once by the entry (main.js) */
export interface RunDeps {
  /** the solver (engine.ts, through job.js `runJob`) */
  engine: {
    /** a career job: its answer (rejects when the job waited on a start the engine's restart superseded) */
    run: (job: CareerJob, progress?: (p: PlanProgress) => void, background?: boolean) => Promise<CareerAnswer>;
    /** a job other than a background one is waiting or running */
    busy: () => boolean;
    /** kill the worker (a job in flight then never answers) and start a fresh one */
    restart: () => void;
    /** settles at the next kill */
    nextKill: () => Promise<unknown>;
    /** keep the worker's work for the next page load */
    saveState: () => void;
    /** the solver's and data's fingerprints (a kept answer is for them alone) */
    key: () => string;
  };
  /** the clock and timers (performance.now, setTimeout, setInterval) */
  clock: {
    now: () => number;
    setTimer: (run: () => void, ms: number) => number;
    every: (run: () => void, ms: number) => number;
    clearTimer: (id: number | undefined) => void;
  };
  /** the answer store's transitions */
  store: RunStore;
  /** the page's state as a run reads it */
  page: {
    /** the career job of the settings, veterans and places now (throws job.js `Blocked` when they make none) */
    job: () => CareerJob;
    /** the places as a key (job.js `placesKey`) */
    places: () => string;
    /** the trainee chosen by hand (null: the planner picks, so a plan runs) */
    trainee: () => number | null;
    /** job.js `refBase` and `baseKey` of a job */
    refBase: (job: CareerJob) => string;
    baseKey: (job: CareerJob) => string;
    /** the search's status line (main.js `searchStatus`): done with its caveats, or why no career lands every target */
    searched: (m: { r: CareerSearch; ms: number }, found: string) => Status;
    /** the answer the last visit kept, and keeping this one (statestore.js) */
    loadAnswer: () => Promise<{ key: string; value: AnswerView } | null>;
    saveAnswer: (key: string, value: AnswerView) => void;
  };
  /** the veteran analysis (analysis.js): its comparison follows the runs */
  analysis: {
    /** a new run: the comparison under way is stale */
    outdate: () => void;
    /** an edit: the comparison in flight stops */
    stop: () => void;
    /** an answer shows career `r` (null: none) */
    show: (r: DeepReadonly<CareerReport> | null) => void;
    /** no answer */
    clear: () => void;
    /** a run without places found today's best career: the comparisons' baseline */
    keepBaseline: (key: string, best: DeepReadonly<CareerReport>) => void;
  };
  /** the redraws */
  view: {
    /** the career on show (job.js `shown`) */
    shown: () => DeepReadonly<CareerReport> | null;
    /** the status line */
    paint: () => void;
    /** everything (the tree, the results, the pool), then the status line */
    drawAll: () => void;
    /** the plan's table, after a check step (the tree and the veterans stay: a redraw of the open veterans dialog
     *  under a click in progress would lose the click, or land it on the dialog, which closes it) */
    drawCheck: () => void;
  };
}

/** what the check needs of the page (`createRun` builds it from RunDeps; the node tests build their own) */
export interface CheckDeps {
  /** the run the check belongs to is still the newest (no edit since) */
  runIsCurrent: () => boolean;
  /** a check job on the worker, in the background: its answer, or null when the worker was killed meanwhile */
  runCheck: (job: DeepReadonly<CareerJob>) => Promise<{ r: { next?: PlanReport; error?: string } } | null>;
  /** the check as far as it has run, drawn (the plan's table) */
  redraw: () => void;
  /** the plan with its check written, for what else keeps the same report (Compare's kept plan, `reviseKeptPlan`) */
  checked: (plan: DeepReadonly<PlanDone>) => void;
  /** the answer store's plan transitions */
  store: Pick<RunStore, 'planOnShow' | 'setPlanCheck' | 'checkUnavailable'>;
}

/** the pause between an edit and its search, ms */
export const PAUSE_MS = 600;
// a plan job's `next`: nothing of its own; Compare's plan job of the same pool sends the same, so a run can take its
// answer (`keptPlan`). The check's follow-up jobs add `robust` and `pair_search` (`checkPhase`), which the solver leaves
// out of the key it finds the kept check by (`check_key`)
export const PLAN_NEXT = Object.freeze({});
// the plan's phase reports (career/plan/mod.rs `index_rule`: phase, k of n) in words
const progText = ({ ph, k, n }: PlanProgress) =>
  ph === 1
    ? 'searching the candidates'
    : ph === 2
      ? `scoring ${k + 1} of ${n} umas`
      : ph === 3
        ? 'careers on two full veteran parents'
        : ph === 4
          ? `weighing ${n} careers`
          : '';
// the search's lead line: how many careers it weighed, in what time
const searchedText = (m: { r: CareerSearch; ms: number }) => {
  const n = m.r.candidates,
    seconds = (m.ms / 1000).toFixed(1);
  return n > 1 ? `Weighed ${n.toLocaleString()} careers in ${seconds} s.` : `Scored in ${seconds} s.`;
};

// a check that has nothing left to run: every variant done and the pair search in (`pairs`; absent where the solver has
// none)
const checkComplete = (check: DeepReadonly<PlanReport['robust']> | undefined) =>
  check?.pending === 0 && check.pairs !== null;
// a plan whose check has more to run: not done, and not unavailable (the worker lost the plan)
const checkPending = (plan: DeepReadonly<PlanDone>) => !plan.r.check_unavailable && !checkComplete(plan.r.robust);

// The robustness check on the pick, once it is shown: the plan job again with `next.robust: "next"`, one variant per
// job from the plan the worker keeps, as background jobs (an edit lets the variant under way finish and its job waits
// behind it, so the worker and what it keeps stay), until none is left, then one more for the within-trainee pair
// search (`next.pair_search`, sent on every check job: the report's `pairs`). Its report attaches only to the plan it was
// asked for: the same run, the plan on show still `planned` (by its id: each step's write replaces the plan's record),
// and the pick it names (`robust.plan`). Where the worker no longer keeps the plan (it was stopped or restarted: a
// stopped comparison, a solver error, a reload) the check is unavailable, and the table says so (`check_unavailable`).
export async function checkPhase(deps: CheckDeps, planned: DeepReadonly<PlanDone>) {
  const { planOnShow, setPlanCheck, checkUnavailable } = deps.store;
  const current = () => deps.runIsCurrent() && planOnShow(planned.id) !== null;
  for (;;) {
    let m: Awaited<ReturnType<CheckDeps['runCheck']>> = null;
    try {
      const job = {
        ...planned.job,
        next: { ...planned.job.next, robust: 'next' as const, pair_search: true },
      };
      m = await deps.runCheck(job);
    } catch {
      m = null;
    }
    if (!current()) return;
    // (a kill loses the check: the stored state keeps no check, so the next worker could not run it; REPORT N7)
    const check = m?.r.next?.robust;
    if (!check || check.unavailable || check.error) {
      deps.checked(checkUnavailable(planned.id));
      deps.redraw();
      return;
    }
    if (check.plan !== planned.r.outcomes?.career) return;
    const variantsWereDone = planOnShow(planned.id)?.r.robust?.pending === 0;
    deps.checked(setPlanCheck(planned.id, check));
    if (check.pending > 0) continue;
    deps.redraw();
    // (done once the pair search is in; a step after the last variant that brought none has nothing more to bring)
    if (checkComplete(check) || variantsWereDone) return;
  }
}

export function createRun(deps: RunDeps) {
  const { engine, clock, store, page, analysis, view } = deps;
  // the machine's own state, which nothing draws
  let id = 0, // the run under way or last made; a newer edit's run makes an older one's work stale
    inflight: string | null = null, // the run under way: its job and places (`runKey`); null when none is
    tick: number | undefined, // the status line's one-second redraw while a job runs
    timer: number | undefined; // the pause between an edit and its search
  const runKey = (job: CareerJob) => `${jobKey(job)}\n${page.places()}`;

  // every change: redraw now, search again after a short pause (a change during a search stops it)
  function schedule() {
    // an edit that leaves the job as it is (a preset saved or renamed, a setting changed and back) asks for the answer
    // already coming or on show: a run under way goes on, and an answer on show stands (with her comparison, if one is
    // running); a job that can't be built (`Blocked`) goes on as any other edit
    let key: string | null = null;
    try {
      key = runKey(page.job());
    } catch {
      /* the search says why */
    }
    const { last, planState } = store.answer.peek();
    if (
      key !== null &&
      (inflight
        ? inflight === key
        : !store.runView.peek().searching && last && `${last.key}\n${last.plan}` === key && planState.state !== 'error')
    )
      return;
    clock.clearTimer(timer);
    id++;
    inflight = null;
    // only a job in flight is cut short: an idle worker keeps what it holds for the next jobs (the last search, every
    // uma's best career, the proofs); during the pause before a search nothing runs, so a second edit kills nothing
    if (engine.busy()) engine.restart();
    // a comparison in flight is stopped with it: its panel goes back to idle, or a roll's quick figures stand unchecked
    // (an answer that stands keeps her)
    analysis.stop();
    store.runWaits(clock.now());
    view.paint();
    timer = clock.setTimer(search, PAUSE_MS);
  }

  // the answer the last visit drew, when it was for this setup (the solver's and data's fingerprints, the job, the
  // places): put among the answers kept, so the first search draws it at once instead of planning (the solver is
  // deterministic for its job)
  async function preloadAnswer() {
    let job: CareerJob;
    try {
      job = page.job();
    } catch {
      return;
    }
    const key = runKey(job),
      saved = await page.loadAnswer();
    if (saved?.key === `${engine.key()}\n${key}` && saved.value?.last) store.keepSavedAnswer(key, saved.value);
  }

  // A run: the search, then (with the trainee's place left to the planner) the plan, both on the one worker; the
  // answer is drawn once, at the end. Two solver jobs: the plan's reuses the search's (the solver keeps its last
  // search, so the plan job does none of that work again; a second job costs a few ms), and the page knows which phase
  // is at work without the solver reporting it. A newer edit stops a run in either phase (`id`); nothing of it is drawn.
  async function search() {
    const runId = ++id;
    analysis.outdate();
    let job: CareerJob;
    try {
      job = page.job();
    } catch (e) {
      const blocked = e as Error & Partial<Pick<Status, 'act'>> & { kind?: Status['kind'] };
      endUnsearched({
        status: { kind: blocked.kind || 'error', text: blocked.message, act: blocked.act },
        last: null,
        planState: { state: 'none' },
      });
      return;
    }
    // edits that end where they started (a setting changed and back, a star clicked twice): the job is the one whose
    // answer is on show (era, settings, veterans as the job carries them, and the same places), so it stands and
    // nothing runs (her comparison, if the edits cut one short, is hers to start again); a failed plan makes it run
    // again
    const key = jobKey(job),
      places = page.places(),
      answerKey = `${key}\n${places}`;
    const { last, planState } = store.answer.peek();
    if (last && last.key === key && last.plan === places && planState.state !== 'error') {
      store.runEnds();
      clock.clearTimer(tick);
      view.drawAll();
      resumeCheck(runId);
      return;
    }
    const hit = store.recallAnswer(answerKey);
    if (hit) {
      store.showKeptAnswer(hit);
      analysis.show(view.shown());
      clock.clearTimer(tick);
      view.drawAll();
      resumeCheck(runId);
      return;
    }
    inflight = answerKey;
    store.searchStarts(clock.now());
    clock.clearTimer(tick);
    tick = clock.every(view.paint, 1000);
    view.paint();
    const fail = (text: string) => {
      store.answerFails(text);
      analysis.clear();
    };
    // the plan to check once it is shown (`checkPhase`)
    let checkFor: DeepReadonly<PlanDone> | null = null;
    try {
      const m = await engine.run(job);
      if (runId !== id) return;
      if ('error' in m.r) fail(`Not searched: ${m.r.error}`);
      else {
        const result = { r: m.r, ms: m.ms },
          found = searchedText(result),
          status = page.searched(result, found);
        const best = m.r.results[0];
        const planned = best && !page.trainee() ? await planPhase(runId, job, found) : { state: 'none' as const };
        if (planned === null) return; // a newer edit took over
        // the answer: the plan's pick (the best odds when no plan ran or it failed), and the veteran you got, drawn
        // once from it (a comparison on screen was for the old answer); a plan made becomes the reference (`refFor`)
        store.showAnswer(
          { status, last: { r: m.r, job, key, plan: places }, planState: planned },
          planned.state === 'done' ? { base: page.refBase(job), places: JSON.parse(places) } : null,
        );
        // (a plan Compare kept may hold a check an edit cut short: it resumes, as a kept answer's does)
        if (planned.state === 'done' && checkPending(planned)) checkFor = planned;
        if (status.kind === 'done' && planned.state !== 'error') {
          store.keepAnswer(answerKey);
          // kept across a reload too: the same setup draws it at once (`preloadAnswer`)
          page.saveAnswer(`${engine.key()}\n${answerKey}`, store.answer.peek());
        }
        analysis.show(view.shown());
        if (best && !job.trainees.length && !job.fix.length) analysis.keepBaseline(page.baseKey(job), best);
      }
    } catch (e) {
      if (runId !== id) return;
      fail((e as Error).message);
    }
    store.runEnds();
    inflight = null;
    clock.clearTimer(tick);
    view.drawAll();
    if (checkFor) checkPhase(checkDeps(runId), checkFor);
  }

  // what the robustness check of run `runId` needs: its report shows in the plan's table only (`view.drawCheck`); a
  // plan Compare kept for the same job is the same report, so it takes the check too (`reviseKeptPlan`)
  const checkDeps = (runId: number): CheckDeps => ({
    runIsCurrent: () => runId === id,
    // (the engine posts the job, a copy: it writes nothing into it)
    runCheck: (job) =>
      Promise.race([engine.run(job as CareerJob, undefined, true), engine.nextKill().then(() => null)]),
    redraw: view.drawCheck,
    checked: (plan) => store.reviseKeptPlan(plan.job, plan.r),
    store,
  });
  // an unfinished check of the plan on show, resumed where its answer is shown again (`checkPhase`; a check cut short
  // by an edit and its undoing, or by a newer answer): nothing where it is done or unavailable
  function resumeCheck(runId: number) {
    const { planState } = store.answer.peek();
    if (planState.state !== 'done' || !checkPending(planState)) return;
    checkPhase(checkDeps(runId), planState);
  }
  // a run that ends without searching (the job can't be built): its message, no answer
  function endUnsearched(next: Answer) {
    store.runEndsUnsearched(next);
    clock.clearTimer(tick);
    analysis.clear();
    view.drawAll();
  }

  // The plan (solver: career/plan/mod.rs `index_rule`, exact odds, no simulation): the same job with `next`, every
  // owned uma's best career scored by the careers it saves toward a fully looped parent (the cheapest trio). The page
  // reads `next` alone, so the solver skips the career reports. A trainee you chose leaves it nothing to pick (no plan
  // phase). Returns the plan's state, or null when a newer edit (run `runId` no longer current) took over.
  async function planPhase(runId: number, job: CareerJob, found: string): Promise<PlanState | null> {
    store.planStarts(found, clock.now());
    view.paint();
    const { rollout: _r, p_now: _p, ...plain } = job;
    let planned: PlanState;
    try {
      // Compare may have run this very plan job already (`keptPlan`): take its answer instead of planning again
      const jp = { ...plain, next: { ...PLAN_NEXT }, no_reports: true },
        kept = store.keptPlan(planKey(jp));
      const m2: { r: { next?: DeepReadonly<PlanReport>; error?: string }; ms: number } = kept
        ? { r: { next: kept.r }, ms: kept.ms }
        : await engine.run(jp, (p) => {
            if (runId === id) {
              store.planProgresses(progText(p));
              view.paint();
            }
          });
      if (runId !== id) return null;
      const x = m2.r.next;
      planned =
        x && !x.error && x.options?.length
          ? { state: 'done', id: store.newPlanId(), r: x as PlanReport, ms: m2.ms, job: jp }
          : { state: 'error', text: m2.r.error || x?.error || 'no option found.' };
    } catch (e) {
      if (runId !== id) return null;
      planned = { state: 'error', text: (e as Error).message };
    }
    if (planned.state === 'done') engine.saveState(); // the worker's kept work, for the next page load
    return planned;
  }

  // the first run of a page load: the last visit's answer, when nothing changed since, is drawn at once
  async function start() {
    await preloadAnswer();
    search();
  }
  return { schedule, start };
}
