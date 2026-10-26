// ---------- the engine: a Web Worker running the WebAssembly solver (shared by the planner and the career page) ----------
import { $ } from './dom.js';
import { ERA } from './settings.js';
import { ASSETS, LATER, loadFail } from './data.js';
import { stateLoad, stateSave, stateDrop } from './statestore.js';
// A running wasm call cannot be interrupted, so a superseded search is stopped by terminating its worker.
/**
 * The worker's answer to a job: the solver's report, whose shape is the job command's (a deck search's
 * `results.js` LoopReport, a career search's `career.js` CareerSearch, ...), and the solver's time in ms.
 * @typedef {{r: any, ms: number}} SolverAnswer
 *
 * A job for the solver: its command and that command's fields.
 * @typedef {{cmd: string} & Record<string, unknown>} SolverJob
 *
 * A career plan's phase report (career/plan/mod.rs `index_rule`): phase `ph`, step k of n.
 * @typedef {{ph: number, k: number, n: number}} PlanProgress
 *
 * A job posted to the worker and not answered yet, by its id.
 * @typedef {object} PendingJob
 * @property {(answer: SolverAnswer) => void} done called with the solver's answer
 * @property {(message: {r: any}) => void} [interim] a deck search's fast first pass
 * @property {(message: PlanProgress) => void} [progress] a career plan's phase reports
 * @property {boolean} [background] a job an edit lets finish (the career plan's check, one short variant a job; the
 *   state store, `engineSaveState`): `engineBusy` leaves it out, so the worker and what it keeps stay
 */
let worker = null,
  ready = null,
  lastJobId = 0;
/** @type {Map<number, PendingJob>} */
const pending = new Map();
// The wasm (ASSETS.wasm, fingerprinted next to the page) is fetched and compiled once here and the compiled module handed to every
// worker, so a restart skips fetching and compiling it. Where a module can't be posted, the worker gets the bytes.
let wasmModule = null,
  engineGen = 0; // engineGen: a kill during the compile wait cancels that start
const eraKey = () => (ERA === 'gm' ? 'gm' : 'global');
// the solver's data for the active era (its own file, preloaded by the page head)
let engineData = null;
function eraEngineData() {
  if (!engineData) {
    engineData = fetch(ASSETS.engine[eraKey()])
      .then((response) => {
        if (!response.ok) throw 0;
        return response.text();
      })
      .catch(() => {
        throw new Error(loadFail('the solver data'));
      });
    engineData.catch(() => {
      engineData = null;
    });
  }
  return engineData;
}
function compiled() {
  if (!wasmModule)
    wasmModule = (async () => {
      // compiled as it streams in, so Chrome keeps the optimised code for the next visit (it caches only streaming compiles);
      // a clone of the response gives the bytes (for a worker that can't take the module, and for compiling them where the
      // streaming compile is missing or refuses, e.g. a host serving the wasm under another type)
      let response,
        streamed = null,
        bytes;
      try {
        response = await fetch(ASSETS.wasm);
        if (!response.ok) throw 0;
      } catch {
        throw new Error(loadFail('the solver'));
      }
      if (WebAssembly.compileStreaming) {
        const whole = response.clone();
        try {
          streamed = WebAssembly.compileStreaming(response).catch(() => null);
        } catch {
          streamed = null;
        }
        response = whole;
      }
      try {
        bytes = await response.arrayBuffer();
      } catch {
        throw new Error(loadFail('the solver'));
      }
      let module = streamed && (await streamed);
      if (!module)
        try {
          module = await WebAssembly.compile(bytes);
        } catch {
          module = null;
        }
      return { module, bytes };
    })();
  wasmModule.catch(() => {
    wasmModule = null;
  }); // a failed load is retried on the next search
  return wasmModule;
}
// whether a job other than a background one has been posted and not answered yet (a kill would cut it short; with
// none, the worker is idle or finishing a background job, which a new job waits behind)
export const engineBusy = () => [...pending.values()].some((job) => !job.background);
// the next kill (`killEngine`): a job in flight then never answers, so a background job's caller waits on this too
let nextKillAt = null;
export function nextKill() {
  if (!nextKillAt) {
    /** @type {(value?: unknown) => void} */
    let resolve = () => {};
    const promise = new Promise((r) => {
      resolve = r;
    });
    nextKillAt = { promise, resolve };
  }
  return nextKillAt.promise;
}
export function killEngine() {
  engineGen++;
  if (worker) worker.terminate();
  worker = null;
  ready = null;
  pending.clear();
  nextKillAt?.resolve();
  nextKillAt = null;
}
// the worker compiled and loaded without a job (a reload that shows a saved result still wants the next search quick)
export const warmEngine = () => startEngine().catch(() => {});
// a superseded or stopped job's worker goes and the next one starts loading at once, during the pause before the next search
export function restartEngine() {
  killEngine();
  warmEngine();
}
// A message from the worker. `start`: the start under way ({resolve, reject} of its promise, and the blob URL of the
// worker's script, released once it has loaded).
function onWorkerMessage(message, start) {
  if (message.type === 'ready') {
    URL.revokeObjectURL(start.url);
    start.resolve();
  } else if (message.type === 'error') {
    if (message.id === undefined) {
      start.reject(new Error(message.message));
      return;
    } // the start failed
    // a job failed inside the solver (a wasm trap leaves the instance unusable): every job waiting gets the error as
    // its answer and the worker goes (the next job starts a fresh one); without this the job never answered
    const waiting = [...pending.values()];
    killEngine();
    for (const job of waiting)
      job.done({ r: { error: `the solver stopped with an error (${message.message})` }, ms: 0 });
  } else if (message.type === 'interim') pending.get(message.id)?.interim?.(message);
  else if (message.type === 'progress') pending.get(message.id)?.progress?.(message);
  else if (message.type === 'result') {
    const job = pending.get(message.id);
    pending.delete(message.id);
    job?.done(message);
  }
}
function startEngine() {
  if (ready) return ready;
  const gen = engineGen;
  ready = Promise.all([compiled(), eraEngineData()]).then(
    ([{ module, bytes }, data]) =>
      new Promise((resolve, reject) => {
        if (gen !== engineGen) {
          reject(new Error('superseded'));
          return;
        }
        // the worker's script as a blob URL, released once the worker has loaded it (one per worker start otherwise stays)
        let url = null;
        try {
          const source = $('engine-worker').textContent;
          url = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
          worker = new Worker(url);
        } catch {
          if (url) URL.revokeObjectURL(url);
          reject(new Error('This browser blocked the background worker the solver runs in.'));
          return;
        }
        const start = { resolve, reject, url };
        worker.onmessage = (e) => onWorkerMessage(e.data, start);
        worker.onerror = (e) => reject(new Error(`The solver failed to start: ${e.message || 'unknown error'}`));
        try {
          if (!module) throw 0;
          worker.postMessage({ type: 'init', module, data });
        } catch {
          worker.postMessage({ type: 'init', bytes, data });
        } // module not transferable here
      }),
  );
  ready = ready.then(() => restore().catch(() => {})); // the last session's kept work, before the first job
  ready.catch(() => {
    if (gen === engineGen) ready = null;
  }); // a failed start is retried by the next search
  return ready;
}
// The career planner's kept work (each uma's best career per pool, the proofs) survives a page reload: a new worker gets
// what the last one held before its first job (`state_in`), and the page stores it after each plan (`state_out`, in the
// worker once it is idle). Only the page that asks for it (`enginePersist`) does this; its key is the solver's and the
// era data's fingerprints, so a release starts afresh. A stored state the solver refuses is dropped.
let persistOn = false,
  saveTimer = 0;
const stateKey = () => `${ASSETS.wasm}|${ASSETS.engine[eraKey()]}`;
export function enginePersist() {
  persistOn = true;
}
// posts a job to the running worker as it is (no gate) and resolves with its answer
/** @param {SolverJob} job @param {PendingJob['interim']} [interim] @param {PendingJob['progress']} [progress] @returns {Promise<SolverAnswer>} */
function post(job, interim, progress, background = false) {
  return new Promise((done) => {
    const id = ++lastJobId;
    pending.set(id, { done, interim, progress, background });
    worker.postMessage({ type: 'run', id, job });
  });
}
async function restore() {
  if (!persistOn) return;
  const key = stateKey(),
    stored = await stateLoad(key);
  if (!stored || !worker) return;
  const answer = await post({ cmd: 'state_in', state: stored });
  if (!answer.r || answer.r.error) stateDrop(key);
}
// after a plan: store what the worker holds as soon as it is idle (a job in flight comes first, a background one
// excepted: the store waits behind it in the worker; a killed worker saves nothing). The store is a background job
// itself: waiting behind the plan's check, it would otherwise count as a job in flight, and an edit then would kill
// the worker and the plan it keeps (`engineBusy`). Promptly: a Reroll during an exact comparison kills the worker, and the next one restores this (a few ms)
// where it would otherwise plan again (most of a second); state_out takes about 10 ms
export function engineSaveState() {
  if (!persistOn) return;
  clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    if (!worker || !ready || engineBusy()) {
      engineSaveState();
      return;
    }
    try {
      await ready;
      const answer = await post({ cmd: 'state_out' }, undefined, undefined, true);
      if (answer.r?.state) stateSave(stateKey(), answer.r.state);
    } catch {
      /* the worker went: nothing to save */
    }
  }, 100);
}
// what has not started yet (data.js LATER) is left out of every search (a roll's plan job, `next.roll`, alike: the
// solver finds the plan's kept state by the job as it ran)
const gate = (job) =>
  !['deck', 'career'].includes(job.cmd)
    ? job
    : {
        ...job,
        exclude: [...(job.exclude || []), ...LATER.umas],
        exclude_outfits: [...(job.exclude_outfits || []), ...LATER.outfits],
        exclude_cards: [...(job.exclude_cards || []), ...LATER.cards],
      };
// interim: called with the fast first pass of a deck search, before the exact one resolves the promise; progress: called
// with the career plan's phase reports ({ph, k, n}: career/plan/mod.rs `index_rule`) while it runs
/**
 * @param {SolverJob} job
 * @param {PendingJob['interim']} [interim]
 * @param {PendingJob['progress']} [progress]
 * @param {boolean} [background]  `PendingJob['background']`
 * @returns {Promise<SolverAnswer>}
 */
export function run(job, interim, progress, background = false) {
  return startEngine().then(() => post(gate(job), interim, progress, background));
}
