// The loop page's run machine (src/loop/run.ts), its run state (loop/state.ts) and its job (loop/job.ts), on the fake
// page (fakepage.ts) with a fake engine (each job a promise the test settles; interim answers the test sends), a fake
// clock and timers, and a Map for localStorage. npx vitest run (in webapp)
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { test, vi } from 'vitest';
import { loadData } from '../../src/data.ts';
import { fmtIn } from '../../src/dom.ts';
import { gameData } from '../../src/data.ts';
import { deckPart, isOwned, skillsPart } from '../../src/job.ts';
import { blockedStatus, buildJob, jobKey } from '../../src/loop/job.ts';
import { PAUSE_MS, createRun } from '../../src/loop/run.ts';
import { resultKeys } from '../../src/loop/keys.ts';
import * as loopStore from '../../src/loop/state.ts';
import { headline, initLoopState, loopRun, loopView, resultsStale } from '../../src/loop/state.ts';
import {
  DEFAULTS,
  initStore,
  jobSetup,
  setAuto,
  setOwned,
  setPresets,
  setRank,
  setSort,
  setTargets,
  settings,
} from '../../src/settings.ts';
import { on } from '../../src/store.ts';
import { FIXTURES } from './loopfixtures.ts';
import { fakeClock, flush, memory } from './fakes.ts';
import { must } from '../../src/must.ts';
import type { Backing } from '../../src/storage.ts';
import type { SettingsView } from '../../src/settings.ts';
import type { SolverRefusal } from '../../src/solver.ts';
import type { DeckJob } from '../../src/loop/job.ts';
import type { DeckAnswer, LoopDeps } from '../../src/loop/run.ts';
import type { DrawnReport, LoopReport, LoopResult } from '../../src/loop/types.ts';
import { exactSummary, loopReport, resultDeck, resultLoop } from './reports.ts';

vi.mock('#page-data', () => import('./fakepage.ts'));

await loadData(null);
const GOLDEN = JSON.parse(fs.readFileSync(new URL('./loopgolden.json', import.meta.url), 'utf8'));
const LAST = 'llp:last:global';

// ---------- the fakes
const blocked = (): Backing => ({
  getItem: () => {
    throw new Error('blocked');
  },
  setItem: () => {
    throw new Error('blocked');
  },
  removeItem: () => {
    throw new Error('blocked');
  },
});
// the engine: each job's promise and interim callback, held for the test
interface HeldJob {
  job: DeckJob;
  interim: (answer: DeckAnswer) => void;
  resolve: (answer: DeckAnswer) => void;
  reject: (error: Error) => void;
}
function fakeEngine() {
  const jobs: HeldJob[] = [];
  const engine = {
    jobs,
    restarts: 0,
    run: (job: DeckJob, interim: (answer: DeckAnswer) => void) =>
      new Promise<DeckAnswer>((resolve, reject) => {
        jobs.push({ job, interim, resolve, reject });
      }),
    restart: () => {
      engine.restarts++;
    },
  };
  return engine;
}
// a loop report: its top loop's odds `p`
const report = (p: number, more: Partial<LoopReport> = {}) =>
  loopReport({
    top: [{ loop: resultLoop({ p_all: p }), deck: resultDeck() }],
    per_uma: true,
    stats: { ...loopReport().stats, decks_built: 1200 },
    foursomes: 30,
    proven: true,
    exact: exactSummary({ rho: 2, rho_target: 2, complete: true }),
    ...more,
  });
const answer = (r: LoopReport | SolverRefusal, ms = 1500): DeckAnswer => ({ r, ms });
const doneTextOf = (ms: number) =>
  `Checked ${(1200).toLocaleString()} decks / ${(30).toLocaleString()} loops in ${(ms / 1000).toFixed(1)} s`;

// a page: the settings stored in `saved` (FIXTURES' shape), the run state fresh, the machine on the fakes
interface PageOptions {
  saved?: unknown;
  storage?: Backing;
  example?: LoopDeps['example'];
  settingsOf?: (s: SettingsView) => SettingsView;
}
function page({
  saved = FIXTURES.defaults,
  storage = memory(),
  example = null,
  settingsOf = (s) => s,
}: PageOptions = {}) {
  const settingsStorage = memory();
  settingsStorage.setItem('llp:settings', JSON.stringify(saved));
  initStore(() => settingsStorage);
  initLoopState();
  const clock = fakeClock(),
    engine = fakeEngine(),
    drawn: DrawnReport[] = [],
    examples: string[] = [];
  const loop = createRun({
    run: engine.run,
    restart: engine.restart,
    now: clock.now,
    setTimer: clock.setTimer,
    every: clock.every,
    clearTimer: clock.clearTimer,
    storage: () => storage,
    store: loopStore,
    settings: () => settingsOf(settings.peek()),
    render: (r) => {
      drawn.push(r);
    },
    loadExample: () => {
      examples.push('loaded');
    },
    example,
  });
  return { loop, clock, engine, drawn, storage, examples };
}
// a search started by the Search button and its job sent
type Page = ReturnType<typeof page>;
// the job sent `i`th (from the end when negative)
const held = (p: Page, i: number) => must(p.engine.jobs.at(i), `job ${i}`);
async function searching(p: Page) {
  p.loop.doSearch();
  await flush();
  return held(p, -1);
}
const snapshot = () => [loopRun.peek(), loopView.peek()];
// the status line's button, and pressing it
const act = () => must(loopView.peek().status.act, 'the status button');
const press = () => must(act().run, 'the button runs something')();

// ---------- stop
test('Stop during the pause: no job is sent, nothing is killed', () => {
  const p = page();
  p.loop.scheduleSearch();
  assert.equal(loopRun.peek().searching, true);
  p.loop.stopSearch();
  p.clock.advance(PAUSE_MS * 2);
  assert.equal(p.engine.jobs.length, 0);
  assert.equal(p.engine.restarts, 0);
  assert.equal(loopView.peek().status.kind, 'stopped');
  assert.equal(loopRun.peek().searching, false);
});

test('Stop during the search kills the worker once; its late answer changes nothing', async () => {
  const p = page();
  const job = await searching(p);
  p.loop.stopSearch();
  assert.equal(p.engine.restarts, 1);
  assert.deepEqual(
    { kind: loopView.peek().status.kind, text: loopView.peek().status.text, act: act().label },
    { kind: 'stopped', text: 'Stopped. The results below are from before your change.', act: 'Search again' },
  );
  const before = snapshot();
  job.interim(answer(report(0.01)));
  job.resolve(answer(report(0.02)));
  await flush();
  assert.deepEqual(snapshot(), before);
  assert.equal(p.drawn.length, 0);
  // its Search again searches the same settings at once
  press();
  await flush();
  assert.equal(p.engine.jobs.length, 2);
  assert.equal(loopRun.peek().searching, true);
});

test('Stop during the exact check keeps the fast result, said so', async () => {
  const p = page();
  const job = await searching(p);
  const fast = report(0.01);
  job.interim(answer(fast));
  assert.equal(loopRun.peek().phase, 'check');
  p.loop.stopSearch();
  const { status, lastResult } = loopView.peek();
  assert.equal(status.kind, 'done');
  assert.match(status.text, /^Stopped the exact check/);
  assert.equal(lastResult?.r, fast);
  assert.equal(p.engine.restarts, 1);
});

test('Stop when idle does nothing', () => {
  const p = page();
  const before = snapshot();
  p.loop.stopSearch();
  assert.deepEqual(snapshot(), before);
  assert.equal(p.engine.restarts, 0);
});

// ---------- superseded searches and edits
test("an edit during a search kills it, and the old search's late answers change nothing", async () => {
  const p = page();
  const old = await searching(p);
  p.loop.scheduleSearch(); // an edit
  assert.equal(p.engine.restarts, 1);
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.engine.jobs.length, 2);
  const before = snapshot();
  old.interim(answer(report(0.01)));
  old.resolve(answer(report(0.02)));
  await flush();
  assert.deepEqual(snapshot(), before);
  assert.equal(p.drawn.length, 0);
  // the new search's answer is drawn
  held(p, 1).resolve(answer(report(0.03)));
  await flush();
  assert.equal(loopView.peek().status.kind, 'done');
  assert.equal(loopView.peek().lastResult?.r.top[0].loop.p_all, 0.03);
});

test("a job rejected when its worker is killed during the start ('superseded') changes nothing: the newer search keeps the page", async () => {
  const p = page();
  const old = await searching(p); // waiting for the worker to start, say (engine.ts rejects it when that start is killed)
  p.loop.scheduleSearch(); // an edit kills it
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.engine.jobs.length, 2);
  const before = snapshot();
  old.reject(new Error('superseded'));
  await flush();
  assert.deepEqual(snapshot(), before);
  // the newer search still runs in the engine: the next edit kills it
  const restarts = p.engine.restarts;
  p.loop.scheduleSearch();
  assert.equal(p.engine.restarts, restarts + 1);
});

test('a burst of edits inside the pause sends one job; the clock starts at the job', async () => {
  const p = page();
  for (let k = 0; k < 5; k++) {
    p.loop.scheduleSearch();
    p.clock.advance(100);
  }
  assert.equal(loopRun.peek().searching, true);
  assert.equal(p.engine.jobs.length, 0);
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.engine.jobs.length, 1);
  assert.equal(loopRun.peek().startedAt, 400 + PAUSE_MS); // the last edit at 400 ms, then the pause
  p.clock.advance(2500);
  assert.equal(loopRun.peek().now - loopRun.peek().startedAt, 2000); // the clock ticks each second
});

test("the clock's ticker: one while a search runs, none in an edit's pause, none once the search ends", async () => {
  const p = page();
  await searching(p);
  assert.deepEqual(p.clock.pending(), ['every']);
  p.loop.doSearch(); // the Search button during a search: a new search, its own ticker
  await flush();
  assert.deepEqual(p.clock.pending(), ['every']);
  p.loop.scheduleSearch(); // an edit: its pause, no clock yet
  assert.deepEqual(p.clock.pending(), ['once']);
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.deepEqual(p.clock.pending(), ['every']);
  held(p, -1).resolve(answer(report(0.02)));
  await flush();
  assert.deepEqual(p.clock.pending(), []);
});

test('Auto-search off: an edit stops the search and asks for Search, which runs even for the job already done', async () => {
  const p = page();
  const job = await searching(p);
  job.resolve(answer(report(0.02)));
  await flush();
  await searching(p); // the same settings searched again, running
  setAuto(false);
  p.loop.scheduleSearch();
  assert.equal(p.engine.restarts, 1);
  const { status } = loopView.peek();
  assert.deepEqual(
    [status.kind, status.text, status.act?.label],
    ['pending', 'Settings changed: press Search.', 'Search'],
  );
  assert.equal(loopRun.peek().searching, false);
  p.clock.advance(PAUSE_MS * 2);
  assert.equal(p.engine.jobs.length, 2);
  must(status.act?.run, 'Search runs')();
  await flush();
  assert.equal(p.engine.jobs.length, 3, 'Search runs the done job again');
});

// ---------- the done-key shortcut
test('an automatic search of the finished job on show runs nothing; with other loops drawn since, it runs', async () => {
  const p = page();
  const first = await searching(p);
  first.resolve(answer(report(0.02), 1500));
  await flush();
  const done = loopView.peek().status;
  // an edit and its undo: back to the same job, nothing drawn meanwhile
  p.loop.scheduleSearch();
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.engine.jobs.length, 1, 'no job for the job on show');
  assert.equal(loopView.peek().status, done);
  // another job running (its settings), then back: the kill leaves the engine idle, the shortcut holds
  setRank('ss_and_up');
  p.loop.scheduleSearch();
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.engine.jobs.length, 2);
  setRank(DEFAULTS.rank);
  p.loop.scheduleSearch();
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.engine.jobs.length, 2);
  assert.equal(loopView.peek().status, done);
  // that other job's fast loops drawn: the result on show is not the done job's, so it searches
  setRank('ss_and_up');
  p.loop.scheduleSearch();
  p.clock.advance(PAUSE_MS);
  await flush();
  held(p, 2).interim(answer(report(0.05)));
  setRank(DEFAULTS.rank);
  p.loop.scheduleSearch();
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.engine.jobs.length, 4);
});

test('with the example on show (no search finished yet), the automatic search of its settings runs', async () => {
  const p = page({ example: { r: report(0.009), settings: DEFAULTS } });
  assert.equal(p.loop.showFirst(), 'example');
  p.loop.scheduleSearch();
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.engine.jobs.length, 1);
});

// ---------- interim answers
test('an interim answer with loops is drawn and the phase is check; one without loops is ignored', async () => {
  const p = page();
  const job = await searching(p);
  const before = snapshot();
  job.interim(answer(report(0.01, { top: [] })));
  job.interim(answer({ kind: 'refusal', error: 'trap' }));
  assert.deepEqual(snapshot(), before);
  const fast = report(0.01);
  job.interim(answer(fast));
  assert.equal(p.drawn.length, 1);
  assert.equal(loopRun.peek().phase, 'check');
  assert.equal(loopView.peek().lastResult?.r, fast);
  // the final answer with the same loops is not drawn again
  job.resolve(answer(report(0.01)));
  await flush();
  assert.equal(p.drawn.length, 1);
  assert.equal(loopRun.peek().phase, 'search');
});

// ---------- blocked settings and errors
test('blocked settings: the status says why, with its button, and a busy engine is stopped', async () => {
  const p = page();
  await searching(p);
  setTargets([]);
  p.loop.doSearch();
  await flush();
  assert.equal(p.engine.restarts, 1);
  const { status } = loopView.peek();
  assert.deepEqual(
    [status.kind, status.text, status.act?.label],
    ['empty', 'Add a target skill to search.', 'Load the 8/8 example targets'],
  );
  must(status.act?.run, 'its button runs')();
  assert.deepEqual(p.examples, ['loaded']);
  assert.equal(loopRun.peek().searching, false);
});

test('errors: an engine that throws, a refusal, a report without loops', async () => {
  const p = page();
  (await searching(p)).reject(new Error('the solver could not load'));
  await flush();
  assert.deepEqual(loopView.peek().status, { kind: 'error', text: 'the solver could not load' });
  (await searching(p)).resolve(answer({ kind: 'refusal', error: 'bad job' }));
  await flush();
  assert.deepEqual(loopView.peek().status, { kind: 'error', text: 'Not searched: bad job' });
  (await searching(p)).resolve(answer(report(0.01, { top: [] })));
  await flush();
  assert.deepEqual(loopView.peek().status, { kind: 'error', text: 'No result: no loop fits these settings.' });
  assert.equal(loopRun.peek().searching, false);
  assert.equal(p.drawn.length, 0);
});

// ---------- the first result
test('first result: the kept result of these settings, else the example for its settings; the settings untouched', async () => {
  // a finished search kept, then a reload with the same settings: done, no job
  const storage = memory();
  const before = page({ storage });
  (await searching(before)).resolve(answer(report(0.02), 2500));
  await flush();
  const p = page({ storage });
  const record = settings.peek();
  assert.equal(p.loop.showFirst(), 'kept');
  assert.equal(loopView.peek().status.text, doneTextOf(2500));
  assert.deepEqual(loopView.peek().lastResult, { kind: 'deck', r: report(0.02), meta: {} }, 'Export has it');
  assert.equal(p.drawn.length, 1);
  assert.equal(settings.peek(), record, 'showFirst writes no setting');
  // it is a finished search: an edit and its undo search nothing
  p.loop.scheduleSearch();
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.engine.jobs.length, 0);
  assert.equal(loopView.peek().status.text, doneTextOf(2500));
  // the defaults, nothing kept: the example
  const example = report(0.009);
  const q = page({ example: { r: example, settings: DEFAULTS } });
  assert.equal(q.loop.showFirst(), 'example');
  assert.deepEqual(loopView.peek().status, {
    kind: 'example',
    text: 'Showing an example: Unity Cup skills, everything owned',
  });
  assert.deepEqual(loopView.peek().lastResult, { kind: 'deck', r: example, meta: { example: true } });
  // other settings: nothing (the example would not be theirs)
  const r = page({ saved: FIXTURES.partlyOwned, example: { r: example, settings: DEFAULTS } });
  assert.equal(r.loop.showFirst(), null);
  assert.equal(r.drawn.length, 0);
  assert.equal(loopView.peek().status.kind, 'loading');
});

test('first result: a job that throws (a bug) is reported, and the page goes on with nothing on show', () => {
  const reported: unknown[] = [],
    report0 = globalThis.reportError;
  globalThis.reportError = (e: unknown) => {
    reported.push(e);
  };
  try {
    // (settings with no target list, as a bug could make them: JSON.parse hands them over unchecked)
    const p = page({ settingsOf: (s) => JSON.parse(JSON.stringify({ ...s, targets: null })) });
    assert.equal(p.loop.showFirst(), null);
    assert.equal(reported.length, 1);
    assert.equal(loopView.peek().status.kind, 'loading');
  } finally {
    globalThis.reportError = report0;
  }
});

// ---------- storage
test('storage: a finished result is kept under its era; a blocked storage leaves the page working', async () => {
  const p = page();
  const r = report(0.02);
  (await searching(p)).resolve(answer(r, 1800));
  await flush();
  const kept = JSON.parse(p.storage.getItem(LAST) ?? 'null');
  assert.equal(kept.ms, 1800);
  assert.deepEqual(kept.r, r);
  const q = page({ storage: blocked(), example: { r: report(0.009), settings: DEFAULTS } });
  assert.equal(q.loop.showFirst(), 'example');
  (await searching(q)).resolve(answer(report(0.03)));
  await flush();
  assert.equal(loopView.peek().status.kind, 'done');
});

// ---------- what Export marks stale, the status text, the headline
test('resultsStale after stop, pending, blocked, error and while searching; not after done', async () => {
  const p = page();
  const job = await searching(p);
  assert.equal(resultsStale.value, true);
  job.resolve(answer(report(0.02)));
  await flush();
  assert.equal(resultsStale.value, false);
  p.loop.scheduleSearch();
  p.loop.stopSearch();
  assert.equal(resultsStale.value, true, 'stopped');
  setAuto(false);
  p.loop.scheduleSearch();
  assert.equal(resultsStale.value, true, 'pending');
  setTargets([]);
  p.loop.doSearch();
  assert.equal(resultsStale.value, true, 'blocked');
  setTargets(DEFAULTS.targets);
  (await searching(p)).resolve(answer({ kind: 'refusal', error: 'x' }));
  await flush();
  assert.equal(resultsStale.value, true, 'error');
});

test('status text: cut short, the exact check caveats; the headline from the result on show', async () => {
  const p = page();
  assert.equal(headline.value, '');
  const cases: [Partial<LoopReport>, string][] = [
    [{ proven: false }, `${doneTextOf(1500)} · cut short: not proven best`],
    [
      { exact: exactSummary({ rho: 1.25, rho_target: 2, complete: false }) },
      `${doneTextOf(1500)} · exact check: decks within 20% of it (ran out of time)`,
    ],
    [
      { exact: exactSummary({ rho: 1, rho_target: 2, complete: false }) },
      `${doneTextOf(1500)} · not checked exactly (ran out of time)`,
    ],
    [{}, doneTextOf(1500)],
  ];
  for (const [more, text] of cases) {
    (await searching(p)).resolve(answer(report(0.0123, more)));
    await flush();
    assert.equal(loopView.peek().status.text, text);
  }
  assert.equal(headline.value, fmtIn(0.0123));
});

// ---------- the job (P9's golden jobs, P12)
test("golden jobs: each fixture's job byte-identical to 9307950's, the blocked ones' statuses the same", () => {
  for (const [name, saved] of Object.entries(FIXTURES)) {
    page({ saved });
    const record = settings.peek();
    const built = buildJob(record);
    const golden = GOLDEN[name];
    if (golden.job) assert.equal(JSON.stringify('job' in built ? built.job : null), golden.job, name);
    else {
      assert.ok('blocked' in built, name);
      const status = blockedStatus(built, () => undefined);
      assert.deepEqual(
        {
          kind: status.kind,
          text: status.text,
          act: status.act?.label,
          click: status.act?.click,
          run: !!status.act?.run,
        },
        golden.blocked,
        name,
      );
    }
    assert.equal(settings.peek(), record, 'buildJob writes no setting');
  }
});

// the key a finished result is kept under (and the suite's replies are found by): the fingerprints, then the job; the
// Grand Masters era's under its own data's fingerprint
test("golden job keys: the defaults' job keyed as recorded, in both eras", () => {
  page();
  const built = buildJob(settings.peek());
  assert.ok('job' in built);
  const { job } = built;
  assert.equal(jobKey(job), GOLDEN.defaults.key);
  assert.equal(jobKey(job, 'gm'), GOLDEN.defaults.keyGm);
});

// the career page's job carries the same shared parts (job.ts: its targets and roster, its deck space; career/job.ts
// builds it; this pins what the helpers give it and their order: 4ef2a4b's fields, nested)
test("golden career fields: each fixture's shared career-job fields as recorded", () => {
  for (const [name, saved] of Object.entries(FIXTURES)) {
    if (!GOLDEN[name].career) continue;
    page({ saved });
    const s = settings.peek();
    const exclude = gameData()
      .umas.filter((u) => !isOwned(u, s))
      .map((u) => u.id);
    const fields = { skills: skillsPart(s, exclude), deck: deckPart(s) };
    assert.equal(JSON.stringify(fields), GOLDEN[name].career, name);
  }
});

// ---------- effects (loop/main.tsx's two, made the same way)
test('effects: the setup effect wakes for job settings only, once per setter; the status line once per transition', async () => {
  const p = page();
  const runs = { setup: 0, paint: 0 };
  const stopSetup = on([jobSetup], () => {
    runs.setup++;
    p.loop.scheduleSearch();
  });
  const stopPaint = on([loopRun, loopView], () => runs.paint++);
  // presets, sorts and the Auto switch change no job (P2)
  setPresets([{ name: 'mine', groups: [20049] }]);
  setSort({ prefix: 'r', sort: 'name', dir: 'asc' });
  setAuto(false);
  setAuto(true);
  assert.deepEqual(runs, { setup: 0, paint: 0 });
  // ownership and targets do, once per setter call; each schedules one search (its pause: one paint)
  setOwned({ cardsOff: { 30001: false } });
  assert.deepEqual(runs, { setup: 1, paint: 1 });
  setTargets([{ group: 20049 }]);
  assert.deepEqual(runs, { setup: 2, paint: 2 });
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.engine.jobs.length, 1, 'one search for the two edits');
  assert.equal(runs.paint, 3, 'the search starts');
  p.clock.advance(1000);
  assert.equal(runs.paint, 4, 'the clock ticks');
  held(p, 0).interim(answer(report(0.01)));
  assert.equal(runs.paint, 5, 'the interim answer: one transition over both records');
  held(p, 0).resolve(answer(report(0.01)));
  await flush();
  assert.equal(runs.paint, 6, 'the answer');
  assert.equal(runs.setup, 2);
  stopSetup();
  stopPaint();
});

// ---------- the results' keys (loop/keys.ts)
test('result keys: one per result drawn together, a repeated loop and decks told apart by its place; others plain', () => {
  const loop = (p: number) =>
      resultLoop({
        members: [1001, 1002, 1003].map((id) => ({ id, name: `Uma ${id}`, short: '' })),
        outfits: [100101, 100201, 100301].map((id) => ({ outfits: [{ id, label: '' }], covers: [] })),
        per_trainee: [p, p, p],
        p_all: p,
      }),
    deck = (cards: string[]) => resultDeck({ cards });
  // #2 and #3: the same loop and cards, told apart in the solver only by a chain choice whose label prints the same
  const top: [LoopResult, LoopResult, LoopResult] = [
    { loop: loop(0.3), deck: deck(['A', 'B']) },
    { loop: loop(0.2), deck: deck(['A', 'C (end chain at link 1)']) },
    { loop: loop(0.1), deck: deck(['A', 'C (end chain at link 1)']) },
  ];
  const keys = resultKeys(top);
  assert.equal(new Set(keys).size, 3);
  assert.deepEqual(keys.slice(0, 2), resultKeys([top[0], top[1]])); // the first of each keeps its plain key
});
