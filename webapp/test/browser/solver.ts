// The page's solver in the browser flows: a stand-in for the page's Worker answers each job with its scenario's canned
// reply for that kind of job (test/fixtures/solver/<scenario>.snap.json: a deck search; a career search, plan,
// robustness-check step, comparison and its plan, roll, a career with places fixed and one with every place chosen (Use this: scored), per pool size; the kept
// work out and in), which web.rs
// `fixtures` computes natively through the worker's own `Solver` from one job of each kind (<scenario>.jobs.json). The
// page's wasm compile is handed an empty module, so the solver never runs here. A job of a kind the scenario has no
// reply for fails the flow, naming it; a background one (the state store, a check step) is left unanswered, as one
// still running. The page reads a reply against its job through the legacies it names, by their place in the job's
// pool (so a career kind is per pool size, `plan@4`), and through the plan a check, a roll and a comparison name (the
// scenario's one plan of that pool): every other edit of a flow is answered with that plan. The `worker` flow alone
// runs the real solver (test/browser/worker.spec.ts); `./build.sh fixtures` regenerates the replies.
//
// Every context also gets a seeded Math.random (the career rolls use it), and the stand-in carries the suite's hooks on
// the page's workers (`window.__workerHooks`, `__hold`/`__release`: window.d.ts).
import fs from 'node:fs';
import path from 'node:path';
import type { BrowserContext, Page } from 'playwright/test';
import type { Request } from '../../src/gen/solver.ts';

/** the fixtures' folder */
export const FIXTURE_DIR = path.resolve(import.meta.dirname, '../fixtures/solver');

/** A worker's answer to one job: a deck search's fast pass, then its result (web.rs `fixtures::answer`). */
export interface Answer {
  interim?: unknown;
  result: unknown;
}
/** What a flow's solver did, checked when it ends: the jobs of a kind its scenario has no reply for. */
export interface SolverTally {
  answered: number;
  missing: string[];
}

/** A job's kind, as the scenario's canned replies are named (the same in web.rs `fixtures`' jobs files). A career
 * job's names its pool's size too (`plan@4`): its reply names legacies by their place in the pool, which the page
 * checks, so a flow that removes a veteran or adds a borrowed one needs that pool's replies. */
export function jobKind(job: Request): string {
  // (a deck search by its rank class too: a flow that changes it sees another answer)
  if (job.cmd === 'deck') return `deck@${job.skills.rank_class}`;
  if (job.cmd !== 'career') return job.cmd;
  // (every place chosen, Use this: the trainee and both parents fixed, one career asked, as career/job.ts buildJob
  // writes it; `trainees` alone cannot tell, since a job with no trainee placed lists every owned uma)
  const { next, compare, legacies, fix, top } = job.career;
  const kind = next
    ? next.robust
      ? 'check'
      : next.roll
        ? 'roll'
        : compare
          ? 'compare-plan'
          : 'plan'
    : compare
      ? 'compare'
      : fix?.length === 2 && top === 1
        ? 'scored'
        : fix?.length
          ? 'fixed'
          : 'search';
  return `${kind}@${legacies.length}`;
}
/** A scenario's canned replies, by job kind (none for a flow without a scenario). */
export const cannedReplies = (scenario: string | null): Record<string, Answer> =>
  scenario ? JSON.parse(fs.readFileSync(path.join(FIXTURE_DIR, `${scenario}.snap.json`), 'utf8')) : {};

/** A context's solver, before any other init script (the suite's hooks register on the stand-in). */
export async function installSolver(ctx: BrowserContext, scenario: string | null): Promise<SolverTally> {
  const canned = cannedReplies(scenario);
  const tally: SolverTally = { answered: 0, missing: [] };
  await ctx.addInitScript(seedRandom);
  await ctx.addInitScript(workerStandIn);
  await ctx.exposeBinding('__llpSolver', (_, text: string) => {
    const job: Request = JSON.parse(text);
    const answer = canned[jobKind(job)];
    if (answer) {
      tally.answered++;
      return JSON.stringify(answer);
    }
    // (a background job, the state store or a check step, may go unanswered: as one still running when the page moved
    // on; only the scenario's own pool has them canned)
    if (job.cmd === 'state_out' || (job.cmd === 'career' && job.career.next?.robust)) return null;
    tally.missing.push(`${jobKind(job)} in ${scenario ?? 'no scenario'}: ${text.slice(0, 300)}`);
    // (FIXTURES_DUMP=FILE: each such job written whole, one a line, to be added to its scenario's jobs file)
    if (process.env.FIXTURES_DUMP)
      fs.appendFileSync(process.env.FIXTURES_DUMP, `${scenario} ${jobKind(job)} ${text}\n`);
    return null;
  });
  return tally;
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

/** A page stores the worker's kept work 100 ms after a plan (solver/engine.ts engineSaveState), and a new worker restores it
 * before its first job (`state_in`), so whether a reload comes before or after that save decides the next worker's
 * jobs. This waits for that save: until no `state_out` is open and no foreground job ended in the last 300 ms, unless
 * a foreground job is under way. Call it before a reload. */
export async function quiet(page: Page): Promise<void> {
  await page.evaluate(() => window.__llpSolverQuiet?.());
}

// (runs in the page) window.Worker replaced by a stand-in that makes the real worker (its script loads, its load errors
// reach the page) and answers every job with its kind's canned reply, never starting the solver. The suite's hooks on every
// worker the page makes are `window.__workerHooks` (window.d.ts). While `window.__hold` is set, the messages wait (a
// search stays running however fast it is) until `window.__release()`; a worker terminated meanwhile drops its own.
function workerStandIn() {
  const Real = window.Worker;
  const hooks: Window['__workerHooks'] = { made: [], post: [], message: [], terminate: [] };
  const flushes: (() => void)[] = [];
  window.__workerHooks = hooks;
  window.__hold = false;
  window.__release = () => {
    window.__hold = false;
    for (const flush of flushes) flush();
  };
  // the wasm is still fetched, as the page does; compiling it is the cost the stand-in saves (an empty module's 8 bytes)
  const empty = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]);
  WebAssembly.compileStreaming = async (source) => {
    const response = await source;
    if (!response.ok) throw new TypeError(`wasm fetch failed: ${response.status}`);
    await response.body?.cancel();
    return WebAssembly.compile(empty);
  };
  const sleep = (ms: number) =>
    new Promise((resolve) => {
      setTimeout(resolve, ms);
    });
  const background = (job: Request) => job.cmd === 'state_out' || (job.cmd === 'career' && !!job.career.next?.robust);
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
    private queue: Promise<void> = Promise.resolve(); // one job at a time, as the real worker answers
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

    postMessage(sent: WorkerPosted) {
      let message = sent;
      for (const post of hooks.post.toReversed()) message = post(message, this) ?? message;
      if (this.dead) return;
      if (message.type === 'init') {
        this.queue = this.queue.then(() => this.emit({ type: 'ready' }));
        return;
      }
      const { id, job } = message;
      const kind = kindOf(job);
      this.open.set(id, kind);
      if (kind) open[kind]++;
      const answer = window.__llpSolver(JSON.stringify(job));
      this.queue = this.queue.then(() => this.answer(id, job, answer));
    }
    private async answer(id: number, job: Request, reply: Promise<string | null>) {
      const text = await reply;
      if (this.dead) return;
      if (text === null) {
        if (background(job)) this.ended(id);
        else
          this.emit({ type: 'error', id, message: `no canned reply for this ${job.cmd} job in the flow's scenario` });
        return;
      }
      // each message in a task of its own, as the real worker's (a deck search's fast pass, then its result)
      const answer: { interim?: SolverReplyWire; result: SolverReplyWire } = JSON.parse(text);
      if (answer.interim) {
        await sleep(0);
        this.emit({ type: 'interim', id, r: answer.interim, ms: 0 });
      }
      await sleep(0);
      this.emit({ type: 'result', id, r: answer.result, ms: 0 });
    }
  }
  Object.defineProperty(window, 'Worker', { value: SolverWorker, configurable: true, writable: true });
}
