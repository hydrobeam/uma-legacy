// What the browser flows' init scripts add to the page's window (solver.ts, fixtures.ts), typed for the functions that
// run in the page.
import type { CareerReply, DeckReply, Request } from '../../src/gen/solver.ts';

/** A reply as the page's worker posts it. */
type Reply = CareerReply | DeckReply;
/** What the page posts to its worker (src/solver.ts `ToWorker`), a job with its generated type. */
type Posted = { type: 'init'; module: WebAssembly.Module; data: string } | { type: 'run'; id: number; job: Request };
/** What the worker posts (src/solver.ts `FromWorker`), a reply with its generated type. */
type Message =
  | { type: 'ready' }
  | { type: 'progress'; id: number; ph: number; k: number; n: number }
  | { type: 'interim' | 'result'; id: number; r: Reply; ms: number }
  | { type: 'error'; id: number | undefined; message: string };

/** The suite's hooks on every worker the page makes (solver.ts `workerStandIn`). */
interface WorkerHooks {
  made: ((worker: EventTarget) => void)[];
  /** before the worker gets a message; may return one to send in its place (the last added runs first) */
  post: ((message: Posted, worker: EventTarget) => Posted | undefined)[];
  /** as each message arrives, before the page sees it (a hook may change it) */
  message: ((data: Message, worker: EventTarget) => void)[];
  terminate: ((worker: EventTarget) => void)[];
}

declare global {
  type WorkerPosted = Posted;
  type WorkerMessage = Message;
  type SolverReplyWire = Reply;
  interface Window {
    __workerHooks: WorkerHooks;
    /** while set, the workers' messages wait until `__release()` */
    __hold: boolean;
    __release: () => void;
    /** the node side of the stand-in (solver.ts `installSolver`): a job's canned answer (JSON), or null */
    __llpSolver: (job: string) => Promise<string | null>;
    /** resolves once no state store is open and no foreground job ended in the last 300 ms */
    __llpSolverQuiet: () => Promise<void>;
    /** workers made and terminated, jobs sent (in order) and their results */
    __made: number;
    __killed: number;
    __runs: Request[];
    __results: Reply[];
    /** the worker flow's record of its real worker: each job sent and its answer (worker.spec.ts) */
    __real: { job: unknown; answer: { interim?: unknown; result?: unknown } }[];
  }
}
