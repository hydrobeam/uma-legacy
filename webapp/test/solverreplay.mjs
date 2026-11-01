// The solver's replies, recorded and replayed (docs/BUILD.md, *Tests*; .review/web/testperf.md). Most groups use the
// solver only to put the page in a state. In those (groups.spec.mjs REPLAY) a stand-in for the page's Worker answers
// each job from the store (cache/solver-replies/, a directory per solver and data: replaystore.mjs), and the page's
// wasm compile is handed an empty module, so the solver neither compiles nor runs. `LLP_SOLVER=live` runs every group
// on the real solver and records its replies (./build.sh test and testq do so when the store has none for this solver
// and data); a job the store lacks fails its group with the job named (no live fallback within a run).
//
// The key of a reply: SHA-256 over the wasm's fingerprinted name (ASSETS.wasm), the era data the worker was given (its
// SHA-256, which its fingerprint ASSETS.engine[era] truncates), every job that worker ran before, in order, and the
// job's own JSON (keys sorted; the message's per-call id left out). The solver keeps work between jobs (a plan's kept
// state, its caches, what `state_in` restores), so a reply is a function of the worker's whole history, not of its job
// alone. A replayed reply is therefore the one the real worker gave in the recording run, provided the solver is
// deterministic given its jobs: its only clock reads are timings it reports, and a live run prints and counts any reply
// that differs from the stored one (timings aside: `*_ms`, `ms`). A new wasm or new data changes every key, so a solver
// change fails every replaying group until a live run records it, and that run checks the solver's own replies.
//
// Every context also gets a seeded Math.random (the sample veterans' draw and the career rolls use it), so a page makes
// the same jobs in the recording run and in replay.
import fs from 'node:fs';
import path from 'node:path';
import { STORE_ROOT, storeName } from './replaystore.mjs';

/** 'live' (every group on the real solver, its replies recorded) or 'replay' (the default: REPLAY groups from the store). */
export const SOLVER = process.env.LLP_SOLVER || 'replay';
if (!['live', 'replay'].includes(SOLVER)) throw new Error(`LLP_SOLVER is live or replay, not ${SOLVER}`);

// a reply's content for the determinism check: every field but the timings (val/same.py's rule)
const untimed = (text) =>
  JSON.stringify(JSON.parse(text).msgs, (k, v) => (k === 'ms' || k.endsWith('_ms') ? undefined : v));

/**
 * How a group's contexts reach the solver; `tally` collects what happened, for the group's summary.
 * @typedef {{replay: boolean}} SolverMode  replay: answer from the store, else run live (recording under
 *   LLP_SOLVER=live)
 * @typedef {{replayed: number, recorded: number, differs: number, missing: string[], held: string[], stopped: string[]}} SolverTally
 */
/** @param {object} ctx  a browser context, before any other init script (the suite's worker hooks sit on top)
 * @param {SolverMode} mode @param {SolverTally} tally */
export async function installSolver(ctx, mode, tally) {
  await ctx.addInitScript(seedRandom);
  const record = SOLVER === 'live';
  if (!mode.replay && !record) return; // the real worker, untouched
  // assets: the page's data-assets JSON, which names the store's directory
  await ctx.exposeBinding('__llpSolver', async (_, op, assets, key, text) => {
    if (!/^[0-9a-f]{64}$/.test(key)) throw new Error(`bad solver reply key ${key}`);
    const dir = path.join(STORE_ROOT, storeName(JSON.parse(assets)));
    const file = path.join(dir, `${key}.json`);
    const stored = await fs.promises.readFile(file, 'utf8').catch(() => null);
    if (op === 'get') {
      if (stored !== null) tally.replayed++;
      return stored;
    }
    if (op === 'missing' || op === 'held' || op === 'stopped') {
      tally[op].push(text);
      return null;
    }
    tally.recorded++;
    // (a job that failed in the worker is left out: its message is the browser's, e.g. for states' deliberately null job)
    if (stored !== null && !/"type":"error"/.test(text) && untimed(stored) !== untimed(text)) {
      tally.differs++;
      console.log(`  solver reply differs from the one stored under ${key} (${JSON.parse(text).cmd})`);
    }
    await fs.promises.mkdir(dir, { recursive: true });
    const partial = `${file}.${process.pid}.part`; // written whole, then renamed: a concurrent reader sees all or none
    await fs.promises.writeFile(partial, text);
    await fs.promises.rename(partial, file);
  });
  await ctx.addInitScript(workerStandIn, !record);
  if (mode.replay) ctx.on('page', settleBeforeLeaving);
}
// A page stores the worker's kept work 100 ms after a plan (engine.js engineSaveState), and a new worker restores it
// before its first job (`state_in`), so whether a reload comes before or after that save decides the next worker's
// history. In a replaying group (when recorded and when replayed, alike) a reload or a navigation waits for that save:
// until no `state_out` is open and no foreground job ended in the last 300 ms, unless a foreground job is under way (the
// reload cuts it short, as the group means it to, and the page saves nothing while it runs). A robustness check under
// way is cut short as before: the stored state keeps none.
function settleBeforeLeaving(pg) {
  for (const k of ['reload', 'goto']) {
    const leave = pg[k].bind(pg);
    pg[k] = async (...args) => {
      await pg
        .evaluate(() => window.__llpSolverQuiet?.())
        .catch(() => {
          // a page already gone or not the planner's: nothing to settle
        });
      return leave(...args);
    };
  }
}

// (runs in the page) Math.random as mulberry32 from a fixed seed, restarted on each page load
function seedRandom() {
  let s = 0x2f6b9d3a;
  Math.random = () => {
    s = (s + 0x6d2b79f5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// (runs in the page) window.Worker replaced by a stand-in that makes the real worker (its script loads, its load errors
// reach the page) and either records what it answers (replay false) or answers every job from the store and never
// starts it (replay true; the page's wasm compile then gets an empty module)
function workerStandIn(replay) {
  const STOP_GRACE_MS = 3000;
  const Real = window.Worker,
    encoder = new TextEncoder();
  if (replay) {
    // the wasm is still fetched, as the page does; compiling it is the cost replay saves (the 8 bytes of an empty module)
    const empty = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]);
    WebAssembly.compileStreaming = async (source) => {
      const response = await source;
      if (!response.ok) throw new TypeError(`wasm fetch failed: ${response.status}`);
      response.body?.cancel();
      return WebAssembly.compile(empty);
    };
  }
  const sha256 = async (text) =>
    [...new Uint8Array(await crypto.subtle.digest('SHA-256', encoder.encode(text)))]
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('');
  // a job's JSON with every object's keys sorted
  const canonical = (job) =>
    JSON.stringify(job, (_, v) =>
      v && typeof v === 'object' && !Array.isArray(v)
        ? Object.fromEntries(
            Object.keys(v)
              .sort()
              .map((k) => [k, v[k]]),
          )
        : v,
    );
  const assets = () =>
    self.__LLP_DATA?.['data-assets'] || JSON.parse(document.getElementById('data-assets').textContent);
  const store = (op, key, text) => window.__llpSolver(op, JSON.stringify(assets()), key, text);
  const sleep = (ms) =>
    new Promise((resolve) => {
      setTimeout(resolve, ms);
    });
  const background = (job) => job?.cmd === 'state_out' || !!job?.next?.robust;
  // the jobs under way in this page's workers (foreground ones, and state stores), and when a foreground one last ended
  const open = { foreground: 0, store: 0, ended: performance.now() };
  const kindOf = (job) => (job?.cmd === 'state_out' ? 'store' : background(job) ? null : 'foreground');
  window.__llpSolverQuiet = async () => {
    while (open.store > 0 || (open.foreground === 0 && performance.now() - open.ended < 300)) await sleep(50);
  };

  class SolverWorker extends EventTarget {
    #real;
    #dead = false;
    #base = null; // promise of the wasm's name and the era data's digest
    #key = Promise.resolve(''); // the last job's key, which stands for the worker's history
    #jobs = 0;
    #steps = new Map(); // a check job (canonical JSON) -> how many times this worker was sent it
    #queue = Promise.resolve(); // a replaying worker answers one job at a time, as the real one
    #recording = new Map(); // job id -> {messages, key, cmd} while the real worker runs it
    #open = new Map(); // job id -> its kind (`kindOf`), from its post to its answer
    #handlers = {};

    constructor(...args) {
      super();
      this.#real = new Real(...args);
      this.#real.addEventListener('message', (e) => this.#fromReal(e.data));
      this.#real.addEventListener('error', (e) =>
        this.dispatchEvent(new ErrorEvent('error', { message: e.message, filename: e.filename, lineno: e.lineno })),
      );
      this.#real.addEventListener('messageerror', () => this.dispatchEvent(new MessageEvent('messageerror')));
    }
    // onmessage and friends as on a Worker: a listener taking its place among the others when first set
    #handler(type, f) {
      if (!(type in this.#handlers)) this.addEventListener(type, (e) => this.#handlers[type]?.call(this, e));
      this.#handlers[type] = f;
    }
    get onmessage() {
      return this.#handlers.message ?? null;
    }
    set onmessage(f) {
      this.#handler('message', f);
    }
    get onerror() {
      return this.#handlers.error ?? null;
    }
    set onerror(f) {
      this.#handler('error', f);
    }
    get onmessageerror() {
      return this.#handlers.messageerror ?? null;
    }
    set onmessageerror(f) {
      this.#handler('messageerror', f);
    }
    terminate() {
      this.#dead = true;
      this.#real.terminate();
      for (const id of [...this.#open.keys()]) this.#ended(id);
    }
    #ended(id) {
      const kind = this.#open.get(id);
      this.#open.delete(id);
      if (!kind) return;
      open[kind]--;
      if (kind === 'foreground') open.ended = performance.now();
    }
    #emit(data) {
      if (this.#dead) return;
      this.dispatchEvent(new MessageEvent('message', { data }));
      if (data.type === 'result' || data.type === 'error') this.#ended(data.id);
    }

    postMessage(message, transfer) {
      if (this.#dead) return;
      if (message?.type === 'init') {
        this.#base = sha256(String(message.data)).then((digest) => `${assets().wasm}|${digest}`);
        if (replay) this.#queue = this.#queue.then(() => this.#emit({ type: 'ready' }));
        else this.#real.postMessage(message, transfer);
        return;
      }
      if (message?.type !== 'run' || !this.#base) {
        this.#real.postMessage(message, transfer);
        return;
      }
      const kind = kindOf(message.job);
      this.#open.set(message.id, kind);
      if (kind) open[kind]++;
      if (replay) this.#queue = this.#queue.then(() => this.#replay(message));
      else {
        this.#recording.set(message.id, { messages: [], key: this.#ran(message.job), cmd: message.job?.cmd });
        this.#real.postMessage(message, transfer);
      }
    }
    // the key of a job run next on this worker: its history so far, then the job. The jobs the page sends in the
    // background go out on a timer (the state store, `state_out`) and as each answer comes (the plan's robustness check,
    // `next.robust`, one variant a job), so how they interleave with the rest depends on timing: they are left out of
    // the history. The state store's answer is the history's kept work (it keeps no check). A check job's answer is the
    // next step of that plan's check, which the worker keeps and resumes when the page asks again: it is keyed by how
    // many times this worker was sent that same job before
    #ran(job) {
      const text = canonical(job);
      this.#jobs++;
      if (background(job)) {
        const step = (this.#steps.get(text) ?? 0) + 1;
        if (job.next?.robust) this.#steps.set(text, step);
        const tag = job.next?.robust ? `check step ${step}` : 'store';
        return Promise.all([this.#base, this.#key]).then(([base, history]) =>
          sha256(`${base}\n${history}\n${tag}\n${text}`),
        );
      }
      this.#key = Promise.all([this.#base, this.#key]).then(([base, history]) =>
        sha256(`${base}\n${history}\n${text}`),
      );
      return this.#key;
    }
    async #replay(message) {
      const n = this.#jobs + 1,
        key = await this.#ran(message.job);
      if (this.#dead) return;
      const job = message.job,
        what = `${job?.cmd}${job?.next ? '.next' : ''} (job ${n} of its worker, key ${key.slice(0, 12)}): ${canonical(job).slice(0, 160)}`;
      const stored = await store('get', key);
      if (stored === null && background(job)) {
        // a background job the recording run never sent: the page moved on before it there (the next check step goes
        // out only as the last one answers). It stays unanswered, as a job still running when the page moved on, whose
        // answer the page drops; a check that waits for it fails, with this named in the test's annotations
        store('held', key, what);
        this.#ended(message.id);
        return;
      }
      if (stored === null) {
        // a job the page stops before its answer (a change mid-search ends the worker) was never answered when recorded:
        // given a moment, the page stops it here too
        await sleep(STOP_GRACE_MS);
        if (this.#dead) {
          store('stopped', key, what);
          return;
        }
        // the worker's answer to a failed job: the page drops the worker and shows the error
        store('missing', key, what);
        this.#emit({ type: 'error', id: message.id, message: `no stored reply for ${what}` });
        return;
      }
      // at once, each message in a task of its own as the real worker's (a deck search's fast pass, then its result)
      for (const d of JSON.parse(stored).msgs) {
        await sleep(0);
        this.#emit({ ...d, id: message.id });
      }
    }
    #fromReal(data) {
      const job = this.#recording.get(data?.id);
      if (job) {
        const { id, ...d } = data;
        // copied now: the suite's hooks may change the message the page gets
        job.messages.push(JSON.parse(JSON.stringify(d)));
        if (d.type === 'result' || d.type === 'error') {
          this.#recording.delete(id);
          job.key.then((k) => store('put', k, JSON.stringify({ cmd: job.cmd, msgs: job.messages })));
        }
      }
      this.#emit(data);
    }
  }
  window.Worker = /** @type {any} */ (SolverWorker);
}
