// The career run machine's robustness check (career.js runs the search and the plan, then this on the plan they made).
// It takes what it needs of the page as `CheckDeps`, so the node tests run it on fakes.
import type { CareerJob, PlanReport } from '../career.js';
import type { DeepReadonly } from '../store.ts';
import { checkUnavailable, planOnShow, setPlanCheck } from './state.ts';
import type { PlanDone } from './state.ts';

/** what the check needs of the page */
export interface CheckDeps {
  /** the run the check belongs to is still the newest (no edit since: career.js `run.id`) */
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
