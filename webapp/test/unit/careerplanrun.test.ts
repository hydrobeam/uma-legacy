// The career run machine (career/run.ts `createRun`) and a roll (analysis/compare.ts) on the real answer store with a
// typed fake engine and clock: an edit that leaves the job as it is lets the plan under way go on (no new job, no
// restart, one plan answered); the plan's progress reads "scoring k of n umas"; a plan made asks the engine to keep the
// worker's work; a roll asks the plan on show, after a newer plan and back on the first answer alike.
// npx vitest run (in webapp)
import { expect, test, vi } from 'vitest';
import { PAUSE_MS, createRun } from '../../src/career/run.ts';
import * as store from '../../src/career/state.ts';
import * as analysisStore from '../../src/career/analysis/state.ts';
import { createComparison } from '../../src/career/analysis/compare.ts';
import { draft } from '../../src/veterans/store.ts';
import type { CareerAnswer, CareerJob } from '../../src/career/job.ts';
import type { PlanProgress } from '../../src/solver.ts';
import type { DeepReadonly } from '../../src/state/store.ts';
import { career, careerJob, loadVets, option, plan, search, showPlan, completeVet } from './careerplans.ts';
import { lineageNode } from './reports.ts';
import { fakeClock, flush } from './fakes.ts';

vi.mock('#page-data', () => import('./fakepage.ts')); // run.ts reads the solver's fingerprints from it (engine.ts stateKey)

/** a job the fake engine was sent, held until the test answers it */
interface Sent {
  job: CareerJob;
  progress: ((p: PlanProgress) => void) | undefined;
  background: boolean;
  resolve: (answer: CareerAnswer) => void;
  live: boolean;
}
function fakeEngine() {
  const jobs: Sent[] = [];
  const engine = {
    jobs,
    restarts: 0,
    saves: 0,
    run: (job: CareerJob, progress?: (p: PlanProgress) => void, background = false) =>
      new Promise<CareerAnswer>((resolve) => {
        jobs.push({ job, progress, background, resolve, live: true });
      }),
    busy: () => jobs.some((j) => j.live && !j.background),
    restart: () => {
      engine.restarts++;
      for (const j of jobs) j.live = false;
    },
    nextKill: () => new Promise<unknown>(() => undefined),
    saveState: () => {
      engine.saves++;
    },
    /** job n answered with `answer` */
    answer(n: number, answer: CareerAnswer) {
      const j = jobs[n];
      if (!j?.live) throw new Error(`job ${n} is not in flight`);
      j.live = false;
      j.resolve(answer);
    },
  };
  return engine;
}
// the comparison the run machine drives (its hooks), on the same engine and clock; `shown`: the career on show
function comparison(engine: ReturnType<typeof fakeEngine>, clock: ReturnType<typeof fakeClock>) {
  return createComparison({
    engine,
    clock,
    answers: store,
    store: analysisStore,
    page: {
      shown: () => store.answerCareer.peek(),
      searching: () => store.runView.peek().searching,
      baseKey: (job) => JSON.stringify(job),
      targets: () => [{ group: 20049 }],
      isFull: () => false,
      planTop: 5,
      // her draft: target 20049 at 3 stars
      draftOf: () => ({
        ...lineageNode(1003, { 20049: 3 }, { 11: 1 }),
        borrowed: false,
        parents: [lineageNode(1001, {}, { 11: 1 }), lineageNode(1002, {}, { 11: 1 })],
      }),
      draft: () => draft.peek(),
    },
  });
}

test('an edit that leaves the job as it is lets the plan go on: no new job, no restart, one plan; its progress reads scoring k of n umas, and the plan made is kept for the next load', async () => {
  loadVets([completeVet('a', 1001), completeVet('b', 1002)]);
  const clock = fakeClock(),
    engine = fakeEngine(),
    job = careerJob({ legacies: [] });
  const run = createRun({
    engine,
    clock,
    store,
    page: {
      job: () => job,
      pool: () => ['a', 'b'],
      places: () => '{"trainee":null,"parents":[null,null]}',
      trainee: () => null,
      refBase: () => 'base',
      baseKey: () => 'basekey',
      searched: (_, found) => ({ kind: 'done', text: found }),
      loadAnswer: async () => null,
      saveAnswer: () => undefined,
    },
    analysis: comparison(engine, clock),
    view: { shown: () => store.answerCareer.peek(), paint: () => undefined },
  });
  run.schedule();
  clock.advance(PAUSE_MS);
  await flush();
  const pick = career(1003, 'Uma C', [0, 1]);
  engine.answer(0, { r: search([pick]), ms: 100 });
  await flush();
  expect(engine.jobs).toHaveLength(2);
  const planJob = engine.jobs[1];
  expect(planJob?.job.career.next).toBeDefined();
  // the plan's phase reports, in words, the counts rising
  for (const k of [0, 1, 2]) {
    planJob?.progress?.({ ph: 2, k, n: 3 });
    expect(store.runView.peek().progress).toBe(`scoring ${k + 1} of 3 umas`);
  }
  // a preset saved: the job as it was
  run.schedule();
  clock.advance(PAUSE_MS);
  await flush();
  expect(engine.restarts).toBe(0);
  expect(engine.jobs).toHaveLength(2);
  expect(store.runView.peek().searching).toBe(true);
  expect(store.runView.peek().phase).toBe('plan');
  const report = plan([option(pick, 0.5, { chosen: true })]);
  engine.answer(1, { r: { ...search([]), next: report }, ms: 900 });
  await flush();
  const { planState } = store.answer.peek();
  expect(planState.state).toBe('done');
  expect(store.runView.peek().searching).toBe(false);
  expect(engine.saves).toBe(1);
  // (the check follows in the background: no foreground job)
  expect(engine.jobs.slice(2).every((j) => j.background)).toBe(true);
});

test('a roll asks the plan on show: after a newer plan, the newer one; back on the first answer, the first one', async () => {
  loadVets([completeVet('a', 1001), completeVet('b', 1002), completeVet('c', 1004)]);
  const clock = fakeClock(),
    engine = fakeEngine(),
    c = comparison(engine, clock);
  const pick = career(1003, 'Uma C', [0, 1]);
  const jobOf = (rank: 'below_ss' | 'ss_and_up'): CareerJob => {
    const base = careerJob({ next: {} });
    return { ...base, skills: { ...base.skills, rank_class: rank } };
  };
  // the plan of `rank` shown, the roll's job: the plan job it carries, and the stars it asks about
  const rollOn = async (): Promise<CareerJob> => {
    c.show(store.answerCareer.peek());
    const n = engine.jobs.length;
    c.roll();
    await flush();
    const sent = engine.jobs.slice(n).find((j) => j.job.career.next?.roll);
    if (!sent) throw new Error('no roll job sent');
    return sent.job;
  };
  const withoutRoll = (job: DeepReadonly<CareerJob>) => {
    const { roll: _, ...next } = job.career.next ?? {};
    return { ...job, career: { ...job.career, next } };
  };
  const first = jobOf('below_ss');
  showPlan(plan([option(pick, 0.5, { chosen: true })]), first, ['a', 'b', 'c']);
  const kept = store.answer.peek();
  const atFirst = await rollOn();
  expect(withoutRoll(atFirst)).toEqual(first);
  expect(atFirst.career.next?.roll).toEqual([3]);
  const newer = jobOf('ss_and_up');
  showPlan(plan([option(pick, 0.4, { chosen: true })]), newer, ['a', 'b', 'c']);
  expect(withoutRoll(await rollOn())).toEqual(newer);
  store.showKeptAnswer(kept, ['a', 'b', 'c']);
  expect(withoutRoll(await rollOn())).toEqual(first);
});
