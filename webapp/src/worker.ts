// Solver worker: the Rust solver compiled to WebAssembly.
// The wasm's calling convention: the page's JSON request is copied into memory the solver allocates (`alloc`), `init`
// (the era's data) or `call` (a job) returns where its JSON answer starts and `result_len` its length, and the request's
// memory is freed (`dealloc`).
// Typed against the protocol in solver.ts, with the worker's own globals (tsconfig.worker.json).
import type { FromWorker, SolverReply, ToWorker } from './solver.ts';

/** `init` or `call`: the request's address and length in; where the JSON answer starts out */
type SolverEntry = (at: number, length: number) => number;
/** The solver's exports (pkg/loopopt/src/web.rs) */
interface SolverExports extends WebAssembly.Exports {
  memory: WebAssembly.Memory;
  alloc: (length: number) => number;
  dealloc: (at: number, length: number) => void;
  init: SolverEntry;
  call: SolverEntry;
  result_len: () => number;
}

let solver: SolverExports | null = null,
  runId: number | undefined; // solver: the instance's exports; runId: the job in the solver now (its progress reports name it)
const encoder = new TextEncoder(),
  decoder = new TextDecoder();
function callRaw(entry: SolverEntry, text: string): SolverReply {
  const request = encoder.encode(text);
  // @ts-expect-error solver is null before init: the strict commit passes the instance in
  const at = solver.alloc(request.length);
  // @ts-expect-error as above
  new Uint8Array(solver.memory.buffer, at, request.length).set(request);
  const answerAt = entry(at, request.length);
  // @ts-expect-error as above
  const answerLength = solver.result_len();
  // @ts-expect-error as above
  const answer = decoder.decode(new Uint8Array(solver.memory.buffer, answerAt, answerLength)); // a view, no copy: the memory is not shared and nothing runs in between
  // @ts-expect-error as above
  solver.dealloc(at, request.length);
  return JSON.parse(answer);
}
const post = (type: 'interim' | 'result', id: number, r: SolverReply, t0: number) =>
  self.postMessage({ type, id, r, ms: performance.now() - t0 } satisfies FromWorker);
self.onmessage = async (e: MessageEvent<ToWorker>) => {
  const message = e.data;
  try {
    if (message.type === 'init') {
      try {
        // env.clock_ms: the solver's wall clock (the career plan's timings, reported with it); env.progress: the career
        // plan's phase (k of n), posted to the page from inside the call
        const imports = {
          env: {
            clock_ms: () => performance.now(),
            progress: (ph: number, k: number, n: number) =>
              self.postMessage({
                type: 'progress',
                // @ts-expect-error runId is undefined before the first job: the strict commit starts it at 0
                id: runId,
                ph,
                k,
                n,
              } satisfies FromWorker),
          },
        };
        solver = (await WebAssembly.instantiate(message.module, imports)).exports as SolverExports; // compiled once by the page
      } catch {
        throw new Error(
          'This browser refused to run WebAssembly, which the solver needs. Use a current browser with WebAssembly enabled.',
        );
      }
      const r = callRaw(solver.init, message.data);
      if (r.error) throw new Error(r.error);
      self.postMessage({ type: 'ready' } satisfies FromWorker);
    } else if (message.type === 'run') {
      const t0 = performance.now(),
        job = message.job;
      runId = message.id;
      // a deck search runs twice: first by the product of per-target odds (fast, posted at once), then by exact odds
      if ((job.cmd === 'deck' || job.mode === 'deck') && job.exact !== false) {
        // @ts-expect-error solver is null before init: the strict commit asserts it (a job comes after `ready`)
        const quick = callRaw(solver.call, JSON.stringify(Object.assign({}, job, { exact: false })));
        post('interim', message.id, quick, t0);
        if (quick.error) {
          post('result', message.id, quick, t0);
          return;
        }
      }
      // @ts-expect-error as above
      post('result', message.id, callRaw(solver.call, JSON.stringify(job)), t0);
    }
  } catch (err) {
    self.postMessage({
      type: 'error',
      id: message.type === 'run' ? message.id : undefined,
      // @ts-expect-error err is unknown: the strict commit reads the message of an Error
      message: String(err?.message || err),
    } satisfies FromWorker);
  }
};
