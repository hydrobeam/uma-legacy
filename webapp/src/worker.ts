// Solver worker: the Rust solver compiled to WebAssembly, called through wasm-bindgen's generated glue
// (src/gen/loopopt.js, written by ./build.sh wasm; pkg/loopopt/src/web.rs `Solver`): the page's module is instantiated
// (`initSync`), a `Solver` made from the era's data, and each job's JSON text passed to `call`, which answers its
// reply's.
// Typed against the protocol in solver.ts, with the worker's own globals (tsconfig.worker.json).
import { initSync, Solver } from './gen/loopopt.js';
import { must } from './must.ts';
import type { FromWorker, SolverReply, ToWorker } from './solver.ts';

// solver: the era's solver (null until init); runId: the job in the solver now (its progress reports name it; the
// page's ids start at 1)
let solver: Solver | null = null,
  runId = 0;
// the career plan's phase (k of n), which the solver reports from inside a call (career/clock.rs `progress`), posted to
// the page
Object.assign(self, {
  solverProgress: (ph: number, k: number, n: number) =>
    self.postMessage({ type: 'progress', id: runId, ph, k, n } satisfies FromWorker),
});
function call(job: string): SolverReply {
  return JSON.parse(must(solver, 'the solver, made by init (the page sends jobs once the worker is ready)').call(job));
}
const post = (type: 'interim' | 'result', id: number, r: SolverReply, t0: number) =>
  self.postMessage({ type, id, r, ms: performance.now() - t0 } satisfies FromWorker);
self.onmessage = (e: MessageEvent<ToWorker>) => {
  const message = e.data;
  try {
    if (message.type === 'init') {
      try {
        // (the module was compiled once by the page)
        initSync({ module: message.module });
      } catch {
        throw new Error(
          'This browser refused to run WebAssembly, which the solver needs. ' +
            'Use a current browser with WebAssembly enabled.',
        );
      }
      // (a request that does not parse throws, saying why)
      solver = new Solver(message.data);
      self.postMessage({ type: 'ready' } satisfies FromWorker);
    } else if (message.type === 'run') {
      const t0 = performance.now(),
        job = message.job;
      runId = message.id;
      // a deck search runs twice: first by the product of per-target odds (fast, posted at once), then by exact odds
      if (job.cmd === 'deck' && job.deck.exact !== false) {
        const quick = call(JSON.stringify({ ...job, deck: { ...job.deck, exact: false } }));
        post('interim', message.id, quick, t0);
        if (quick.kind === 'refusal') {
          post('result', message.id, quick, t0);
          return;
        }
      }
      post('result', message.id, call(JSON.stringify(job)), t0);
    }
  } catch (err) {
    self.postMessage({
      type: 'error',
      id: message.type === 'run' ? message.id : undefined,
      message: String((err instanceof Error && err.message) || err),
    } satisfies FromWorker);
  }
};
