// The career run machine's shared parts: the run's record (`run`: its id makes an older run's work stale; main.js drives
// it), the solver call (`runJob`, also Try a run's), the plan job's `next`, and the robustness check on the plan on show
// (main.js runs the search and the plan, then this check on the plan they made). The check takes what it needs of the
// page as `CheckDeps`, so the node tests run it on fakes.
import { run as engineRun } from '../engine.ts';
import { pickOutfit, outfitNote } from '../roster.js';
import { unprovenWhy } from '../proofs.mjs';
import type { PlanProgress, SolverAnswer } from '../solver.ts';
import type { CareerJob, CareerSearch, CareerTrainee, PlanReport } from './job.js';
import type { DeepReadonly } from '../store.ts';
import { checkUnavailable, planOnShow, setPlanCheck } from './state.ts';
import type { PlanDone } from './state.ts';

/** what the check needs of the page */
export interface CheckDeps {
  /** the run the check belongs to is still the newest (no edit since: `run.id`) */
  runIsCurrent: () => boolean;
  /** a check job on the worker, in the background: its answer, or null when the worker was killed meanwhile */
  runCheck: (job: DeepReadonly<CareerJob>) => Promise<{ r: { next?: PlanReport; error?: string } } | null>;
  /** the check as far as it has run, drawn (the plan's table) */
  redraw: () => void;
  /** the plan with its check written, for what else keeps the same report (Compare's kept plan, analysis.js) */
  checked: (plan: DeepReadonly<PlanDone>) => void;
}

// a check that has nothing left to run: every variant done and the pair search in (`pairs`; absent where the solver has
// none)
export const checkComplete = (check: DeepReadonly<PlanReport['robust']> | undefined) =>
  check?.pending === 0 && check.pairs !== null;

// The robustness check on the pick, once it is shown: the plan job again with `next.robust: "next"`, one variant per
// job from the plan the worker keeps, as background jobs (an edit lets the variant under way finish and its job waits
// behind it, so the worker and what it keeps stay), until none is left, then one more for the within-trainee pair
// search (`next.pair_search`, sent on every check job: the report's `pairs`). Its report attaches only to the plan it was
// asked for: the same run, the plan on show still `planned` (by its id: each step's write replaces the plan's record),
// and the pick it names (`robust.plan`). Where the worker no longer keeps the plan (it was stopped or restarted: a
// stopped comparison, a solver error, a reload) the check is unavailable, and the table says so (`check_unavailable`).
export async function checkPhase(deps: CheckDeps, planned: DeepReadonly<PlanDone>) {
  const current = () => deps.runIsCurrent() && planOnShow(planned.id) !== null;
  // its report shows in the plan's table only: the tree and the veterans stay as they are (a redraw of the open
  // veterans dialog under a click in progress would lose the click, or land it on the dialog, which closes it)
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

// ---------- the run
// One run per edit, one answer per run. A run has phases: the search (the best careers by odds), then, with the
// trainee left to the planner, the plan (its pick among them and every other owned uma). The status line names the
// phase at work; the answer (career, tree places, veteran analysis) is drawn once, when the run ends. Until then the
// previous answer stays on screen, dimmed, its actions off. What the page draws of the run and its answer is the
// answer store's (career/state.ts: `runView`, `answer`); the run machine's own state, which nothing draws, is this
// record, written only by the functions that drive a run (main.js `schedule`, `search`, `planPhase`, `endRun`).
export const run: {
  id: number;
  inflight: string | null;
  tick: number | undefined;
  timer: number | undefined;
} = {
  id: 0, // the run under way or last made; a newer edit's run makes an older one's work stale
  inflight: null, // the run under way: its job and places (`runKey`); null when none is
  tick: undefined, // the status line's one-second redraw while a job runs (setInterval)
  timer: undefined, // the pause between an edit and its search (setTimeout)
};
// the plan's phase reports (career/plan/mod.rs `index_rule`: phase, k of n) in words
export const progText = ({ ph, k, n }: PlanProgress) =>
  ph === 1
    ? 'searching the candidates'
    : ph === 2
      ? `scoring ${k + 1} of ${n} umas`
      : ph === 3
        ? 'careers on two full veteran parents'
        : ph === 4
          ? `weighing ${n} careers`
          : '';
// a plan job's `next`: nothing of its own; Compare's plan job of the same pool sends the same, so a run can take its
// answer (`planKept`). The check's follow-up jobs add `robust` and `pair_search` (`checkPhase`), which the solver leaves
// out of the key it finds the kept check by (`check_key`)
export const PLAN_NEXT = Object.freeze({});
// every career a result names gets one outfit (never "any outfit"): its trainee's outfits are reordered with the chosen
// one first, filled with one of hers when none covers a target (`anyOutfit`); her popup says which
function oneOutfit(t: CareerTrainee | undefined) {
  if (!t || t.outfitNote != null) return;
  const f = pickOutfit(t.id, t.outfits || []);
  t.outfits = f.list;
  t.anyOutfit = f.any;
  t.outfitNote = outfitNote(f);
}
/** The solver's answer to a career job: its CareerSearch, or a refusal (`'error' in m.r`). */
export type CareerAnswer = SolverAnswer<CareerSearch>;
export const runJob = async (
  job: CareerJob,
  progress?: (p: PlanProgress) => void,
  background = false,
): Promise<CareerAnswer> => {
  // (a career job: its report is a CareerSearch)
  const m = (await engineRun(job, undefined, progress, background)) as CareerAnswer;
  if ('error' in m.r) return m; // (a refusal: nothing to fill in)
  const r = m.r;
  for (const x of [...(r.results || []), r.next?.choice, ...(r.next?.options || [])]) oneOutfit(x?.trainee);
  // (a plan's rows carry `proven` alone; why, the solver reports for the whole job: kept with the plan, so a plan
  // taken again from `planKept` or `answers` says the same)
  if (r.next?.options) {
    r.next.unproven_why = unprovenWhy(r);
    // (and the named assumptions any search of the job rests on: the status line's plan caveat, `planNote`)
    r.next.job_assumes = r.job_assumes || [];
  }
  return m;
};
// (the loop members kept per setup until the plan stopped running the loop planner's search: the old store goes)
try {
  localStorage.removeItem('llp:loops');
} catch {
  // storage blocked: nothing was kept to remove
}
