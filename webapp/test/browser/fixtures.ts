// The browser flows' test and fixtures (Playwright Test; playwright.config.ts). Each flow gets a fresh context with:
// the solver (solver.ts: the scenario's canned reply for each kind of job, the wasm never run; a job of a kind without
// one fails the flow), the worker hooks (`window.__made`, `__killed`, `__runs`, `__results`), the stored state it
// starts from (`test.use({ scenario })`: a committed scenario's localStorage, test/fixtures/solver/<scenario>.seed.json,
// with `test.use({ seed })` over it; written before the page's first script, once per tab, so a reload keeps what the
// page saved), and its
// page's errors collected: a flow fails on any page error or error-level console line unless it takes them itself
// (`pageErrors`, emptied by the flow).
import fs from 'node:fs';
import path from 'node:path';
import { test as base, expect } from 'playwright/test';
import { FIXTURE_DIR, installSolver } from './solver.ts';

/** localStorage as a flow starts: key -> value (a string as it is, anything else as its JSON). */
export type Seed = Record<string, unknown>;

interface Options {
  /** a committed scenario's stored state (test/fixtures/solver/<scenario>.seed.json), or none */
  scenario: string | null;
  /** the stand-in answering with canned replies, or the page's own worker on the built wasm (worker.spec.ts) */
  solver: 'fixture' | 'real';
  seed: Seed;
}
interface Fixtures {
  /** the page's errors and error-level console lines so far; a flow that expects some reads and empties it */
  pageErrors: string[];
}

export const test = base.extend<Fixtures & Options>({
  scenario: [null, { option: true }],
  solver: ['fixture', { option: true }],
  seed: [{}, { option: true }],
  context: async ({ context, scenario, solver, seed }, use, testInfo) => {
    const tally = solver === 'fixture' ? await installSolver(context, scenario) : null;
    if (tally) await context.addInitScript(workerCounts);
    const seeded: Seed = scenario ? scenarioSeed(scenario) : {};
    const items = Object.entries({ ...seeded, ...seed }).map(([k, v]): [string, string] => [
      k,
      typeof v === 'string' ? v : JSON.stringify(v),
    ]);
    if (items.length) await context.addInitScript(seedStorage, items);
    await use(context);
    if (!tally) return;
    testInfo.annotations.push({ type: 'solver', description: `${tally.answered} answered with canned replies` });
    expect(tally.missing, "every solver job's kind has a canned reply in the flow's scenario").toEqual([]);
  },
  pageErrors: [
    async ({ page }, use) => {
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(String(e)));
      page.on('console', (m) => {
        // (web-font fetch aborts are network noise)
        if (m.type() === 'error' && !/downloadable font/.test(m.text())) errors.push(m.text());
      });
      await use(errors);
      expect(errors, 'the page logged no errors').toEqual([]);
    },
    { auto: true },
  ],
});
export { expect };
/** A committed scenario's stored state (test/fixtures/solver/<scenario>.seed.json). */
const scenarioSeed = (scenario: string): Seed =>
  JSON.parse(fs.readFileSync(path.join(FIXTURE_DIR, `${scenario}.seed.json`), 'utf8'));

// (runs in the page) counts of the workers made and terminated, the jobs sent and their results
function workerCounts() {
  window.__made = 0;
  window.__killed = 0;
  window.__runs = [];
  window.__results = [];
  const hooks = window.__workerHooks;
  hooks.made.push(() => {
    window.__made++;
  });
  hooks.terminate.push(() => {
    window.__killed++;
  });
  hooks.post.push((m) => {
    if (m.type === 'run') window.__runs.push(m.job);
    return undefined;
  });
  hooks.message.push((d) => {
    if (d.type === 'result') window.__results.push(d.r);
  });
}

// (runs in the page) the flow's stored state, written once per tab (a reload keeps what the page saved since)
function seedStorage(items: [string, string][]) {
  try {
    if (sessionStorage.getItem('llp-test:seeded')) return;
    for (const [k, v] of items) localStorage.setItem(k, v);
    sessionStorage.setItem('llp-test:seeded', '1');
  } catch {
    // storage blocked (a flow that blocks it seeds nothing)
  }
}
