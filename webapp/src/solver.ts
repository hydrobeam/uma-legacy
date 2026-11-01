// Types only: the solver's jobs and reports as the worker and the page exchange them, and the worker protocol both
// ways (src/worker.ts, the worker; engine.ts, the page's side). Each command's own fields and report are typed where
// the page builds and reads them (search.js and results.js `LoopReport` for `deck`; career.js `CareerJob` and
// `CareerSearch` for `career`); here, what the worker and the engine read. The solver's side is
// pkg/loopopt/src/web.rs (`handle`: the commands).

/**
 * A job: its command and that command's fields. Named here, the fields the worker and the engine read.
 */
export interface SolverJob {
  /** `deck`, `career`, `state_in` or `state_out` (web.rs `handle`) */
  cmd: string;
  /** uma, outfit and card ids the search leaves out; the engine adds what has not started yet (`gate`) */
  exclude?: number[];
  exclude_outfits?: number[];
  exclude_cards?: number[];
  /** a deck search: false ranks by the product of per-target odds only (the worker's fast first pass); else the
   * worker runs that pass, then the exact one */
  exact?: boolean;
  /** `deck` runs as a deck search, as `cmd` does (no job the page builds sets it) */
  mode?: string;
  [field: string]: unknown;
}

/** A report: its command's own shape, or a refusal, `error`: why the job was not done. */
export interface SolverReply {
  error?: string;
}

/** `state_out`'s report: what the worker keeps of the career planner's work (career/state.rs `state_out`). */
export interface StateOutReply extends SolverReply {
  ok?: true;
  state?: unknown;
}

/** The answer to a job: the solver's report and its time in ms. */
export interface SolverAnswer<R extends SolverReply = SolverReply> {
  r: R;
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
  /** job `id`'s answer (`result`), or a deck search's fast first pass before it (`interim`) */
  | ({ type: 'interim' | 'result'; id: number } & SolverAnswer)
  /** a job failed in the worker (`id`), or the start did (`id` undefined); a refusal is a `result` with `r.error` */
  | { type: 'error'; id: number | undefined; message: string };
