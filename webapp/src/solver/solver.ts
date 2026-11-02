// Types only: the solver's jobs and reports as the worker and the page exchange them, and the worker protocol both
// ways (src/solver/worker.ts, the worker; solver/engine.ts, the page's side). Each command's own fields and report are
// typed where the page builds and reads them (loop/job.ts `DeckJob` and loop/types.ts `LoopReport` for `deck`;
// career/job.ts `CareerJob` and `CareerSearch` for `career`); here, what the worker and the engine read. The solver's
// side is pkg/loopopt/src/web.rs (`Solver::answer`: the commands).
import type { Request, ScheduleCell as SolverCell, ScheduleRow as SolverRow, StateOut } from '../gen/solver.ts';
import type { DeepReadonly } from '../state/store.ts';

/** A job: its command and that command's fields (the solver's `Request`, src/request.rs in pkg/loopopt: generated),
 *  read-only (the engine posts a copy; a career job shares the veterans store's arrays) */
export type SolverJob = DeepReadonly<Request>;
/** A reply as the worker reads it: every reply says what it is (`kind`); a refusal says why (`error`). */
export interface SolverReply {
  kind: string;
  error?: string;
}

/** A command's report: its own shape (the caller of `run` names it, R), its `kind` never "refusal". */
export type SolverReport = { kind: string };

/** A refusal: why the job was not done (web.rs `Solver::answer` and each command's checks), or, from the engine, that the
 * worker stopped with an error. */
export type SolverRefusal = { kind: 'refusal'; error: string };

/** `state_out`'s report: what the worker keeps of the career planner's work (career/state.rs `state_out`) */
export type StateOutReply = StateOut & { kind: 'state_out' };

/** The answer to a job: the solver's report R or a refusal (`r.kind` tells them apart), and its time in ms. */
export interface SolverAnswer<R extends SolverReport = SolverReport> {
  r: R | SolverRefusal;
  ms: number;
}

/** A career plan's phase report (career/plan/mod.rs `index_rule`): phase `ph`, step k of n. */
export interface PlanProgress {
  ph: number;
  k: number;
  n: number;
}

/** The page to the worker: `init` once, then a `run` per job. */
export type ToWorker =
  /** the compiled solver and the era's data (its JSON text: web.rs `InitReq`) */
  | { type: 'init'; module: WebAssembly.Module; data: string }
  /** a job, by the page's id for it */
  | { type: 'run'; id: number; job: SolverJob };

/** The worker to the page. */
export type FromWorker =
  /** `init` done: jobs may follow */
  | { type: 'ready' }
  /** a career plan's phase, while job `id` runs */
  | ({ type: 'progress'; id: number } & PlanProgress)
  /** job `id`'s answer (`result`), or a deck search's fast first pass before it (`interim`): its report */
  | { type: 'interim' | 'result'; id: number; r: SolverReport; ms: number }
  /** the same, refused: `r` holds only why */
  | { type: 'interim' | 'result'; id: number; r: SolverRefusal; ms: number }
  /** a job failed in the worker (`id`), or the start did (`id` undefined) */
  | { type: 'error'; id: number | undefined; message: string };

// ---------- the schedule report both commands carry (each G1 a member could run; model/agenda.ts reads it) ----------
/** One member's attempt at a scheduled race: the solver's cell, and on the career page the community table's figures
 * the page adds (career/view/parts.ts) */
export type ScheduleCell = SolverCell & {
  /** her win chance on the community table, on her own grades (`shownP` shows it in place of `p`, the planner's,
   *  which counts her pink sparks) */
  tbl?: number;
  /** the table's aptitude cost on her own grades, in percentage points */
  tapt?: number;
  /** those grades, as "Mile C, Turf A" */
  grades?: string;
};
/** One scheduled G1 as the page draws it: the solver's row, its cells as the page shows them; a career's row also
 * says which of her parents won it */
export type ScheduleRow = Omit<SolverRow, 'cells'> & {
  cells: (ScheduleCell | null)[];
  parents?: boolean[];
};
