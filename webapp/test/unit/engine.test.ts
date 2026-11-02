// @vitest-environment happy-dom
// The page's side of the solver's worker (src/engine.ts) on a stand-in Worker the test answers: a job that fails inside
// the worker (a wasm trap) answers as a refusal saying so, the worker goes, and the next job starts a fresh one; a
// job's answer reaches only its own caller. npx vitest run (in webapp)
import { afterEach, expect, test, vi } from 'vitest';
import { run } from '../../src/engine.ts';
import { loadData } from '../../src/data.ts';
import type { Request } from '../../src/gen/solver.ts';
import type { FromWorker, ToWorker } from '../../src/solver.ts';
import { flush } from './fakes.ts';

vi.mock('#page-data', () => import('./fakepage.ts'));
await loadData(null);

/** the workers the page made, in order: what each was sent, and its answer to the page */
const made: StandIn[] = [];
class StandIn {
  sent: ToWorker[] = [];
  terminated = false;
  onmessage: ((e: MessageEvent<FromWorker>) => void) | null = null;
  onerror: ((e: ErrorEvent) => void) | null = null;
  constructor() {
    made.push(this);
  }
  postMessage(message: ToWorker) {
    this.sent.push(message);
  }
  terminate() {
    this.terminated = true;
  }
  answer(data: FromWorker) {
    this.onmessage?.(new MessageEvent('message', { data }));
  }
  /** the id of the last job sent */
  lastRun(): number {
    const run = this.sent.findLast((m) => m.type === 'run');
    if (run?.type !== 'run') throw new Error('no job sent');
    return run.id;
  }
}
Object.defineProperty(globalThis, 'Worker', { value: StandIn, configurable: true, writable: true });
vi.spyOn(WebAssembly, 'compileStreaming').mockResolvedValue(
  new WebAssembly.Module(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])),
);
vi.spyOn(globalThis, 'fetch').mockImplementation(async () => new Response('{}'));
afterEach(() => {
  made.length = 0;
});

const JOB: Request = { cmd: 'state_out' };
/** the worker `made[i]`, started: init answered ready */
async function started(i: number) {
  await flush();
  const worker = made[i];
  if (!worker) throw new Error(`worker ${i} was not made`);
  expect(worker.sent[0]?.type).toBe('init');
  worker.answer({ type: 'ready' });
  await flush();
  return worker;
}

test('a job that fails inside the worker answers as a refusal saying so, the worker goes, and the next job gets a fresh one', async () => {
  const first = run(JOB);
  const worker = await started(0);
  worker.answer({ type: 'error', id: worker.lastRun(), message: 'unreachable executed' });
  expect((await first).r).toEqual({
    kind: 'refusal',
    error: 'the solver stopped with an error (unreachable executed)',
  });
  expect(worker.terminated).toBe(true);
  const next = run(JOB);
  const fresh = await started(1);
  expect(fresh).not.toBe(worker);
  fresh.answer({ type: 'result', id: fresh.lastRun(), r: { kind: 'state_out' }, ms: 5 });
  expect(await next).toEqual({ type: 'result', id: fresh.lastRun(), r: { kind: 'state_out' }, ms: 5 });
});
