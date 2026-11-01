// Solver worker: the Rust solver compiled to WebAssembly.
// The wasm's calling convention: the page's JSON request is copied into memory the solver allocates (`alloc`),
// `init` (the era's data) or `call` (a job) returns where its JSON answer starts and `result_len` its length, and the
// request's memory is freed (`dealloc`).
// Typed against the protocol in solver.ts, with the worker's own globals (tsconfig.worker.json).
import { must } from './must.ts';
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

// solver: the instance's exports (null until init); runId: the job in the solver now (its progress reports name it; the
// page's ids start at 1)
let solver: SolverExports | null = null,
  runId = 0;
const encoder = new TextEncoder(),
  decoder = new TextDecoder();
function callRaw(entry: 'init' | 'call', text: string): SolverReply {
  const instance = must(solver, 'the solver, instantiated by init (the page sends jobs once the worker is ready)');
  const request = encoder.encode(text);
  const at = instance.alloc(request.length);
  new Uint8Array(instance.memory.buffer, at, request.length).set(request);
  const answerAt = instance[entry](at, request.length);
  const answerLength = instance.result_len();
  // (a view, no copy: the memory is not shared and nothing runs in between)
  const answer = decoder.decode(new Uint8Array(instance.memory.buffer, answerAt, answerLength));
  instance.dealloc(at, request.length);
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
              self.postMessage({ type: 'progress', id: runId, ph, k, n } satisfies FromWorker),
          },
        };
        // (the module was compiled once by the page)
        solver = (await WebAssembly.instantiate(message.module, imports)).exports as SolverExports;
      } catch {
        throw new Error(
          'This browser refused to run WebAssembly, which the solver needs. ' +
            'Use a current browser with WebAssembly enabled.',
        );
      }
      const r = callRaw('init', message.data);
      if (r.error) throw new Error(r.error);
      self.postMessage({ type: 'ready' } satisfies FromWorker);
    } else if (message.type === 'run') {
      const t0 = performance.now(),
        job = message.job;
      runId = message.id;
      // a deck search runs twice: first by the product of per-target odds (fast, posted at once), then by exact odds
      if (job.cmd === 'deck' && job.deck.exact !== false) {
        const quick = callRaw('call', JSON.stringify({ ...job, deck: { ...job.deck, exact: false } }));
        post('interim', message.id, quick, t0);
        if (quick.error) {
          post('result', message.id, quick, t0);
          return;
        }
      }
      post('result', message.id, callRaw('call', JSON.stringify(job)), t0);
    }
  } catch (err) {
    self.postMessage({
      type: 'error',
      id: message.type === 'run' ? message.id : undefined,
      message: String((err instanceof Error && err.message) || err),
    } satisfies FromWorker);
  }
};
