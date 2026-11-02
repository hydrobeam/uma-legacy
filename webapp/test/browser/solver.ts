// The page's solver in the browser flows: a stand-in for the page's Worker answers each job from the replies recorded
// for this solver and data (cache/solver-replies/, a directory per solver and data: store.ts), and the page's wasm
// compile is handed an empty module, so the solver never runs (a page-only tree builds none: vite.config.mjs). A job
// the store lacks fails its flow, naming the job. Recording is a solver lane's: `LLP_SOLVER=live` (with the built
// solver, ./build.sh wasm) passes every job to the real worker and stores what it answers.
//
// The key of a reply, within its solver's directory: SHA-256 over the era data the worker was given (its SHA-256),
// every foreground job that worker ran before, in order, and the job's own JSON (keys sorted; the message's per-call id
// left out). The solver keeps work between jobs (a plan's kept state, its caches, what `state_in` restores), so a reply
// is a function of the worker's history, and a replay is sound because the solver is deterministic given its jobs (its
// only clock reads are the timings it reports).
//
// Every context also gets a seeded Math.random (the sample veterans' draw and the career rolls use it), so a page makes
// the same jobs when recorded and replayed, and the stand-in carries the suite's hooks on the page's workers
// (`window.__workerHooks`, `__hold`/`__release`: window.d.ts).
import fs from 'node:fs';
import path from 'node:path';
import type { BrowserContext, Page } from 'playwright/test';
import type { Request } from '../../src/gen/solver.ts';
import { type PageAssets, STORE_ROOT, storeName } from './store.ts';

/** replay (the default): answer from the store; live: every job to the real worker, its replies recorded. */
export type SolverMode = 'replay' | 'live';
export const SOLVER_MODE: SolverMode = process.env.LLP_SOLVER === 'live' ? 'live' : 'replay';
/** What a flow's solver did: replies replayed or recorded, and the jobs the store lacked. */
export interface SolverTally {
  replayed: number;
  recorded: number;
  missing: string[];
}

const storeDirs = new Map<string, Promise<string>>();
const storeDir = (assets: string): Promise<string> => {
  let dir = storeDirs.get(assets);
  if (!dir) {
    const page: PageAssets = JSON.parse(assets);
    const named = storeName(page).then((name) => path.join(STORE_ROOT, name));
    storeDirs.set(assets, named);
    dir = named;
  }
  return dir;
};

/** A context's solver, before any other init script (the suite's hooks register on the stand-in). */
export async function installSolver(ctx: BrowserContext, tally: SolverTally): Promise<void> {
  await ctx.addInitScript(seedRandom);
  await ctx.addInitScript(workerStandIn, SOLVER_MODE);
  await ctx.exposeBinding(
    '__llpSolver',
    async (_, op: 'get' | 'put' | 'missing', assets: string, key: string, text: string) => {
      if (!/^[0-9a-f]{64}$/.test(key)) throw new Error(`bad solver reply key ${key}`);
      const dir = await storeDir(assets);
      const file = path.join(dir, `${key}.json`);
      if (op === 'missing') {
        tally.missing.push(text);
        return null;
      }
      if (op === 'get') {
        const stored = await fs.promises.readFile(file, 'utf8').catch(() => null);
        if (stored !== null) tally.replayed++;
        return stored;
      }
      // (a job that failed in the worker is left out: its message is the browser's)
      if (/"type":"error"/.test(text)) return null;
      tally.recorded++;
      await fs.promises.mkdir(dir, { recursive: true });
      const partial = `${file}.${process.pid}.part`; // written whole, then renamed: a concurrent reader sees all or none
      await fs.promises.writeFile(partial, text);
      await fs.promises.rename(partial, file);
      return null;
    },
  );
}

/** A page stores the worker's kept work 100 ms after a plan (engine.ts engineSaveState), and a new worker restores it
 * before its first job (`state_in`), so whether a reload comes before or after that save decides the next worker's
 * history. This waits for that save: until no `state_out` is open and no foreground job ended in the last 300 ms,
 * unless a foreground job is under way. Call it before a reload in a replaying flow. */
export async function quiet(page: Page): Promise<void> {
  await page.evaluate(() => window.__llpSolverQuiet?.());
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
// reach the page) and answers every job from the store, never starting it (replay), or passes every job to it and
// records what it answers (live).
// The suite's hooks on every worker the page makes are `window.__workerHooks` (window.d.ts). While `window.__hold` is
// set, the messages wait (a search stays running however fast it is) until `window.__release()`; a worker terminated
// meanwhile drops its own.
function workerStandIn(mode: SolverMode) {
  const replay = mode === 'replay';
  const STOP_GRACE_MS = 1000;
  const Real = window.Worker;
  const encoder = new TextEncoder();
  const hooks: Window['__workerHooks'] = { made: [], post: [], message: [], terminate: [] };
  const flushes: (() => void)[] = [];
  window.__workerHooks = hooks;
  window.__hold = false;
  window.__release = () => {
    window.__hold = false;
    for (const flush of flushes) flush();
  };
  if (replay) {
    // the wasm is still fetched, as the page does; compiling it is the cost replay saves (the 8 bytes of an empty module)
    const empty = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]);
    WebAssembly.compileStreaming = async (source) => {
      const response = await source;
      if (!response.ok) throw new TypeError(`wasm fetch failed: ${response.status}`);
      await response.body?.cancel();
      return WebAssembly.compile(empty);
    };
  }
  const sha256 = async (text: string) =>
    [...new Uint8Array(await crypto.subtle.digest('SHA-256', encoder.encode(text)))]
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('');
  // a job's JSON with every object's keys sorted
  const canonical = (job: unknown) =>
    JSON.stringify(job, (_, v: unknown) =>
      v && typeof v === 'object' && !Array.isArray(v)
        ? Object.fromEntries(Object.entries(v).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)))
        : v,
    );
  const assets = () => JSON.parse(document.getElementById('data-assets')?.textContent ?? 'null');
  const sleep = (ms: number) =>
    new Promise((resolve) => {
      setTimeout(resolve, ms);
    });
  // a job the page sends in the background: the state store, or a plan's robustness check (`next.robust`)
  const isCheck = (job: Request) => job.cmd === 'career' && !!job.career.next?.robust;
  const background = (job: Request) => job.cmd === 'state_out' || isCheck(job);
  type Kind = 'foreground' | 'store';
  const kindOf = (job: Request): Kind | null =>
    job.cmd === 'state_out' ? 'store' : background(job) ? null : 'foreground';
  // the jobs under way in this page's workers (foreground ones, and state stores), and when a foreground one last ended
  const open = { foreground: 0, store: 0, ended: performance.now() };
  window.__llpSolverQuiet = async () => {
    while (open.store > 0 || (open.foreground === 0 && performance.now() - open.ended < 300)) await sleep(50);
  };

  class SolverWorker extends EventTarget {
    private real: Worker;
    private dead = false;
    private base: Promise<string> | null = null; // the era data's digest
    private key: Promise<string> = Promise.resolve(''); // the last foreground job's key, which stands for the history
    private jobs = 0; // jobs sent to this worker
    private steps = new Map<string, number>(); // a check job (canonical JSON) -> how many times this worker was sent it
    private queue: Promise<void> = Promise.resolve(); // a replaying worker answers one job at a time, as the real one
    private recording = new Map<number, { key: Promise<string>; cmd: string; msgs: WorkerMessage[] }>(); // job id -> its record
    private open = new Map<number, Kind | null>(); // job id -> its kind, from its post to its answer
    private handlers: Partial<Record<'message' | 'error' | 'messageerror', ((e: Event) => void) | null>> = {};
    private held: WorkerMessage[] = []; // messages that arrived while window.__hold was set

    constructor(script: string | URL, options?: WorkerOptions) {
      super();
      this.real = new Real(script, options);
      flushes.push(() => {
        const held = this.held;
        this.held = [];
        for (const data of held) if (!this.dead) this.dispatchEvent(new MessageEvent('message', { data }));
      });
      for (const made of hooks.made) made(this);
      this.real.addEventListener('message', (e: MessageEvent<WorkerMessage>) => this.fromReal(e.data));
      this.real.addEventListener('error', (e) =>
        this.dispatchEvent(new ErrorEvent('error', { message: e.message, filename: e.filename, lineno: e.lineno })),
      );
      this.real.addEventListener('messageerror', () => this.dispatchEvent(new MessageEvent('messageerror')));
    }
    // onmessage and friends as on a Worker: a listener taking its place among the others when first set
    private handler(type: 'message' | 'error' | 'messageerror', f: ((e: Event) => void) | null) {
      if (!(type in this.handlers)) this.addEventListener(type, (e) => this.handlers[type]?.call(this, e));
      this.handlers[type] = f;
    }
    get onmessage() {
      return this.handlers.message ?? null;
    }
    set onmessage(f) {
      this.handler('message', f);
    }
    get onerror() {
      return this.handlers.error ?? null;
    }
    set onerror(f) {
      this.handler('error', f);
    }
    get onmessageerror() {
      return this.handlers.messageerror ?? null;
    }
    set onmessageerror(f) {
      this.handler('messageerror', f);
    }
    terminate() {
      for (const terminated of hooks.terminate) terminated(this);
      this.dead = true;
      this.real.terminate();
      for (const id of [...this.open.keys()]) this.ended(id);
    }
    private ended(id: number) {
      const kind = this.open.get(id);
      this.open.delete(id);
      if (!kind) return;
      open[kind]--;
      if (kind === 'foreground') open.ended = performance.now();
    }
    private emit(data: WorkerMessage) {
      if (this.dead) return;
      for (const arrived of hooks.message) arrived(data, this);
      if (window.__hold) this.held.push(data);
      else this.dispatchEvent(new MessageEvent('message', { data }));
      if ((data.type === 'result' || data.type === 'error') && data.id !== undefined) this.ended(data.id);
    }

    postMessage(sent: WorkerPosted, transfer?: Transferable[]) {
      let message = sent;
      for (const post of hooks.post.toReversed()) message = post(message, this) ?? message;
      if (this.dead) return;
      if (message.type === 'run') {
        const kind = kindOf(message.job);
        this.open.set(message.id, kind);
        if (kind) open[kind]++;
      }
      if (message.type === 'init') {
        this.base = sha256(message.data);
        if (replay) this.queue = this.queue.then(() => this.emit({ type: 'ready' }));
        else this.real.postMessage(message, transfer ?? []);
        return;
      }
      const run = message;
      if (replay) this.queue = this.queue.then(() => this.replay(run.id, run.job));
      else {
        this.recording.set(run.id, { key: this.ran(run.job), cmd: run.job.cmd, msgs: [] });
        this.real.postMessage(run, transfer ?? []);
      }
    }
    private fromReal(data: WorkerMessage) {
      const id = 'id' in data ? data.id : undefined;
      const record = id === undefined ? undefined : this.recording.get(id);
      if (id !== undefined && record) {
        // copied now: the suite's hooks may change the message the page gets
        record.msgs.push(JSON.parse(JSON.stringify(data)));
        if (data.type === 'result' || data.type === 'error') {
          this.recording.delete(id);
          const text = JSON.stringify({ cmd: record.cmd, msgs: record.msgs });
          void record.key.then((key) => window.__llpSolver('put', JSON.stringify(assets()), key, text));
        }
      }
      this.emit(data);
    }
    // the key of a job run next on this worker: its history so far, then the job. The background jobs go out on a timer
    // (the state store) and as each answer comes (the plan's check, one variant a job), so how they interleave with the
    // rest depends on timing: they are left out of the history. A check job's answer is the next step of that plan's
    // check, which the worker resumes when the page asks again: it is keyed by how many times this worker was sent it.
    private ran(job: Request): Promise<string> {
      const base = this.base ?? Promise.reject(new Error('a job before init'));
      const text = canonical(job);
      if (background(job)) {
        const step = (this.steps.get(text) ?? 0) + 1;
        if (isCheck(job)) this.steps.set(text, step);
        const tag = isCheck(job) ? `check step ${step}` : 'store';
        return Promise.all([base, this.key]).then(([b, history]) => sha256(`${b}\n${history}\n${tag}\n${text}`));
      }
      this.key = Promise.all([base, this.key]).then(([b, history]) => sha256(`${b}\n${history}\n${text}`));
      return this.key;
    }
    private async replay(id: number, job: Request) {
      const n = ++this.jobs;
      const key = await this.ran(job);
      if (this.dead) return;
      const page = JSON.stringify(assets());
      const stored = await window.__llpSolver('get', page, key, '');
      if (stored === null) {
        // a background job the recording never sent (the page moved on before it there) stays unanswered, as one still
        // running when the page moved on; a foreground one the page stops in a moment (a change mid-search ends the
        // worker) was never answered when recorded either. Any other is missing: the worker's answer to a failed job
        if (background(job)) {
          this.ended(id);
          return;
        }
        await sleep(STOP_GRACE_MS);
        if (this.dead) return;
        const what = `${job.cmd} (job ${n} of its worker, key ${key.slice(0, 12)}): ${canonical(job).slice(0, 160)}`;
        await window.__llpSolver('missing', page, key, what);
        this.emit({ type: 'error', id, message: `no stored reply for ${what}` });
        return;
      }
      // at once, each message in a task of its own as the real worker's (a deck search's fast pass, then its result)
      const msgs: Exclude<WorkerMessage, { type: 'ready' }>[] = JSON.parse(stored).msgs;
      for (const d of msgs) {
        await sleep(0);
        this.emit({ ...d, id });
      }
    }
  }
  Object.defineProperty(window, 'Worker', { value: SolverWorker, configurable: true, writable: true });
}
