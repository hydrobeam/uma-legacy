// The one flow on the real solver (./build.sh worker, in check merge): the built wasm in the page's own worker, no
// stand-in, on loop-small's loop page. Its first search's answer, as the page received it, is the one the fixtures
// hold for the same job, which the native build computed through the same `Solver` (web.rs `fixtures`), the wall
// clocks at 0. A difference is the wasm and the native build disagreeing, or a wasm older than the tree (./build.sh
// wasm).
import fs from 'node:fs';
import path from 'node:path';
import { expect, test } from './fixtures.ts';
import { loopSettled, openLoop } from './pages.ts';
import { FIXTURE_DIR } from './solver.ts';

test.use({ scenario: 'loop-small', solver: 'real' });

/** `value` with every wall clock (`timing_s`, `ms`, a key ending in `_ms`) at 0, as the fixtures write them. */
const zeroed = (value: unknown): unknown =>
  JSON.parse(JSON.stringify(value), (key, v: unknown) =>
    (key === 'timing_s' || key === 'ms' || key.endsWith('_ms')) && typeof v === 'number' ? 0 : v,
  );
// a job's JSON with every object's keys sorted
const sorted = (job: unknown) =>
  JSON.stringify(job, (_, v: unknown) =>
    v && typeof v === 'object' && !Array.isArray(v)
      ? Object.fromEntries(Object.entries(v).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)))
      : v,
  );
const read = (name: string) => JSON.parse(fs.readFileSync(path.join(FIXTURE_DIR, name), 'utf8'));

test("the built wasm in the page's worker answers its first search as the native solver does @worker", async ({
  page,
}) => {
  await page.addInitScript(recordWorker);
  await openLoop(page);
  await loopSettled(page);
  const [first] = await page.evaluate(() => window.__real);
  if (!first?.answer.result) throw new Error('the page sent its worker a job and had its answer');
  // (the scenario's deck job is the one its loop page sends first: the reply test answered that job)
  const jobs: { jobs: { kind: string; job: unknown }[] } = read('loop-small.jobs.json');
  expect(sorted(first.job), "loop-small's deck job").toBe(sorted(jobs.jobs.find((j) => j.kind === 'deck')?.job));
  expect(zeroed(first.answer)).toEqual((read('loop-small.snap.json') as Record<string, unknown>).deck);
});

// (runs in the page) the page's workers, recording each job one is sent and its answer
function recordWorker() {
  window.__real = [];
  const listened = new WeakSet<Worker>();
  const sent = new Map<number, { job: unknown; answer: { interim?: unknown; result?: unknown } }>();
  const post = Worker.prototype.postMessage;
  Worker.prototype.postMessage = function (this: Worker, message: WorkerPosted, ...rest: unknown[]) {
    if (!listened.has(this)) {
      listened.add(this);
      this.addEventListener('message', (e: MessageEvent<WorkerMessage>) => {
        const d = e.data;
        const run = d.type === 'interim' || d.type === 'result' ? sent.get(d.id) : undefined;
        if (run && (d.type === 'interim' || d.type === 'result')) run.answer[d.type] = d.r;
      });
    }
    if (message.type === 'run') {
      const run = { job: message.job, answer: {} };
      sent.set(message.id, run);
      window.__real.push(run);
    }
    Reflect.apply(post, this, [message, ...rest]);
  };
}
