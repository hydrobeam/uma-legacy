// The career answer store (src/career/state.ts) and the robustness check on its plan (src/career/run.ts `checkPhase`),
// on made-up reports: each transition copies on write; the check knows its plan by id, so its own writes keep it
// running and a new plan or run stops it; the check reaches every place the plan is kept; an effect wakes only for
// the signals it names. npx vitest run (in webapp)
import assert from 'node:assert/strict';
import { test, vi } from 'vitest';
import { on } from '../../src/state/store.ts';
import * as store from '../../src/career/state.ts';
import {
  answer,
  answerCareer,
  answerFails,
  checkUnavailable,
  dropAnswer,
  keepAnswer,
  keepComparisonPlan,
  keptPlan,
  keepSavedAnswer,
  loadFails,
  newPlanId,
  planOnShow,
  planProgresses,
  planStarts,
  recallAnswer,
  reference,
  reviseKeptPlan,
  runEnds,
  runEndsUnsearched,
  runView,
  runWaits,
  searchStarts,
  setPlanCheck,
  showAnswer,
  showKeptAnswer,
} from '../../src/career/state.ts';
import { planKey } from '../../src/career/keys.ts';
import { type CheckDeps, checkPhase } from '../../src/career/run.ts';
import type { ReadonlySignal } from '@preact/signals';
import type { Answer, AnswerView, PlanDone } from '../../src/career/state.ts';
import type { CareerJob, PlanCheck } from '../../src/career/job.ts';
import type { CheckReply, PairSearch } from '../../src/gen/solver.ts';
import type { DeepReadonly } from '../../src/state/store.ts';
import { must } from '../../src/must.ts';
import { careerJob, careerReport, careerSearch, checkReport, pairSearch, planOption, planReport } from './reports.ts';

vi.mock('#page-data', () => import('./fakepage.ts')); // run.ts reads the solver's fingerprints from it (engine.ts stateKey)

const PLACES = '{"trainee":null,"parents":[null,null]}';
const career = (name: string) =>
  careerReport({
    trainee: { id: 1001, name, outfits: [], covers: [], schedule_outfit: 0 },
    parents: [0, 1],
    career: name,
    p_all: 0.01,
  });
// an answer as a run makes it: the search's best career, and a plan (made now: a new id) whose pick is `pick`
const made = (pick = 'pick'): Answer & { planState: PlanDone } => {
  const choice = career(pick);
  return {
    status: { kind: 'done', text: 'Weighed 2 careers in 0.1 s.' },
    last: {
      r: careerSearch({ results: [career('best'), choice], candidates: 2 }),
      job: careerJob(),
      key: 'job',
      plan: PLACES,
    },
    planState: {
      state: 'done',
      id: newPlanId(),
      r: planReport({
        choice,
        options: [planOption(choice)],
        v0: 9,
        outcomes: { trainee: 1001, parents: [0, 1], career: pick, uncv: [], fixed: [] },
      }),
      ms: 5,
      job: careerJob({ next: {} }),
    },
  };
};
// an answer's plan, made (throws: none)
const doneOf = (a: AnswerView | null) => {
  const plan = must(a, 'an answer').planState;
  assert.ok(plan.state === 'done', 'a plan');
  return plan;
};
// the plan on show
const shownPlan = () => doneOf(answer.value);
// a record's fields, by name
const keysOf = <T extends object>(o: T) => Object.keys(o).filter((k): k is keyof T & string => k in o);
const WHERE = { base: 'base', places: { trainee: null, parents: [null, null] } };
// the veterans the answers' jobs were built on (their ids by legacy index)
const POOL = ['a', 'b'];
const check = (pending: number, pairs: Partial<PairSearch> | null = null): PlanCheck =>
  checkReport({ plan: 'pick', pending, stable: true, contenders: [], pairs: pairs && pairSearch(pairs) });

// each transition: what it does to the record, and the fields it replaces (the rest stay the same objects)
const RUN_STEPS: [string, () => void, string[]][] = [
  ['runWaits', () => runWaits(10), ['searching', 'started', 'phase', 'lead', 'phaseStart']],
  ['searchStarts', () => searchStarts(20), ['searching', 'started', 'phase', 'lead', 'progress', 'phaseStart']],
  ['planStarts', () => planStarts('Weighed 2 careers.', 30), ['phase', 'lead', 'progress', 'phaseStart']],
  ['planProgresses', () => planProgresses('scoring 1 of 3 umas'), ['progress']],
  ['runEnds', () => runEnds(), ['searching']],
];
test('each run transition copies on write: the old record is unchanged and frozen, the new one changes its fields', () => {
  for (const [name, step, fields] of RUN_STEPS) {
    const before = runView.value,
      copy = structuredClone(before);
    step();
    const after = runView.value;
    assert.notEqual(after, before, `${name} replaces the record`);
    assert.deepEqual(before, copy, `${name} leaves the old record as it was`);
    assert.ok(Object.isFrozen(after), `${name}: the new record is frozen`);
    for (const key of keysOf(after))
      if (!fields.includes(key)) assert.equal(after[key], before[key], `${name} keeps ${key}`);
  }
  assert.deepEqual(runView.value, {
    searching: false,
    started: true,
    phase: 'plan',
    phaseStart: 30,
    lead: 'Weighed 2 careers.',
    progress: 'scoring 1 of 3 umas',
  });
});

test('each answer transition copies on write and freezes what it brings in, the jobs aside', () => {
  // [name, step, what the answer is after it]
  const first = made(),
    second = made('other');
  showAnswer(first, WHERE, POOL);
  const STEPS: [string, () => void, (a: AnswerView, b: AnswerView) => void][] = [
    ['showAnswer', () => showAnswer(second, WHERE, POOL), () => assert.equal(answer.value, second)],
    [
      'setPlanCheck',
      () => setPlanCheck(second.planState.id, check(2)),
      (a, b) => {
        assert.deepEqual(doneOf(a).r.robust, check(2));
        assert.equal(a.last, b.last, 'the search result stays the same object');
        assert.equal(a.status, b.status);
        assert.equal(doneOf(a).job, doneOf(b).job);
        assert.equal(doneOf(a).r.options, doneOf(b).r.options, 'only the plan report is copied, not its rows');
        assert.equal(doneOf(a).r.choice, doneOf(b).r.choice);
      },
    ],
    [
      'checkUnavailable',
      () => checkUnavailable(second.planState.id),
      (a, b) => {
        assert.equal(doneOf(a).r.check_unavailable, true);
        assert.equal(doneOf(a).r.robust, doneOf(b).r.robust);
      },
    ],
    [
      'dropAnswer',
      () => dropAnswer(),
      (a, b) => {
        assert.equal(a.last, null);
        assert.equal(a.planState.state, 'none');
        assert.equal(a.status, b.status, 'the status stays');
      },
    ],
    [
      'answerFails',
      () => answerFails('no worker'),
      (a) => assert.deepEqual(a.status, { kind: 'error', text: 'no worker' }),
    ],
    ['loadFails', () => loadFails('no data'), (a) => assert.equal(a.status.text, 'no data')],
    ['runEndsUnsearched', () => runEndsUnsearched(made()), (a) => assert.equal(a.last?.key, 'job')],
  ];
  for (const [name, step, after] of STEPS) {
    const before = answer.value,
      copy = structuredClone(before);
    step();
    assert.notEqual(answer.value, before, `${name} replaces the record`);
    assert.deepEqual(before, copy, `${name} leaves the old record as it was`);
    after(answer.value, before);
  }
  // frozen: the record, the search's report and the plan's, all the way down; a job is not (it holds the veterans'
  // own arrays until the veterans store copies on write)
  // (writes past the read-only type: Object.assign throws on a frozen target as an assignment does; a push by
  // Reflect.apply, an unchecked call)
  const now = answer.value;
  assert.throws(() => Object.assign(must(now.last?.r.results[0], 'a career'), { p_all: 1 }), TypeError);
  assert.throws(() => Reflect.apply(Array.prototype.push, doneOf(now).r.options, [planOption(career('x'))]), TypeError);
  assert.throws(() => Object.assign(now.status, { text: 'x' }), TypeError);
  assert.ok(!Object.isFrozen(now.last?.job), 'a job stays as the run built it');
});

test('a kept answer comes back as it was kept, with the run over in the same batch; a saved one gets a new plan id', () => {
  const kept = made();
  showAnswer(kept, WHERE, POOL);
  keepAnswer('back');
  showAnswer(made('other'), WHERE, POOL);
  runWaits(1);
  let runs = 0;
  const stop = on([answer, runView], () => runs++);
  const hit = recallAnswer('back');
  assert.equal(hit, kept);
  showKeptAnswer(must(hit, 'the kept answer'), POOL);
  assert.equal(answer.value, kept);
  assert.equal(runView.value.searching, false);
  assert.equal(runs, 1, 'one effect run for the answer and the run view together (batch)');
  stop();
  assert.equal(recallAnswer('none'), null);
  // an answer stored by the last visit, its plan id from that visit: a plan of this page may have the same id
  const saved = structuredClone(made());
  saved.planState.id = 1;
  keepSavedAnswer('saved', saved);
  const loaded = doneOf(recallAnswer('saved'));
  assert.ok(loaded.id > kept.planState.id, 'a new id, past every one given before');
  assert.ok(Object.isFrozen(loaded.r));
});

test('the check writes its result into the answer on show, the answers kept and the reference: wherever the plan is', () => {
  const kept = made(),
    other = made('other');
  showAnswer(other, WHERE, POOL);
  keepAnswer('other');
  showAnswer(kept, WHERE, POOL);
  keepAnswer('mine');
  const ref = () => must(reference.value, 'the reference');
  assert.equal(ref().planId, kept.planState.id);
  setPlanCheck(kept.planState.id, check(0, { trainees: [] }));
  assert.deepEqual(doneOf(recallAnswer('mine')).r.robust, check(0, { trainees: [] }), 'the answers cache has it');
  assert.deepEqual(ref().result.robust, check(0, { trainees: [] }), 'the reference has it');
  assert.equal(ref().result, shownPlan().r);
  assert.equal(recallAnswer('other'), other, "another plan's answer is untouched");
  // the kept answer comes back checked: nothing left to resume
  showAnswer(made('third'), WHERE, POOL);
  showKeptAnswer(must(recallAnswer('mine'), 'the kept answer'), POOL);
  assert.deepEqual(shownPlan().r.robust, check(0, { trainees: [] }));
  checkUnavailable(kept.planState.id);
  assert.equal(doneOf(recallAnswer('mine')).r.check_unavailable, true);
  assert.throws(() => setPlanCheck(other.planState.id, check(0)), /on show/, 'a plan not on show is not revised');
});

// the check's fakes: `replies` answered in turn, `during(call)` run while call n waits on the worker
const fakes = (replies: CheckReply[], during: (n: number) => unknown = () => undefined) => {
  const jobs: DeepReadonly<CareerJob>[] = [],
    checked: DeepReadonly<PlanDone>[] = [];
  const seen = { jobs, redraws: 0, checked };
  let current = true;
  const deps: CheckDeps = {
    runIsCurrent: () => current,
    runCheck: async (job) => {
      seen.jobs.push(job);
      during(seen.jobs.length);
      return { r: { kind: 'check', robust: must(replies.shift(), 'a reply for the job') }, ms: 0 };
    },
    redraw: () => {
      seen.redraws++;
    },
    checked: (plan) => {
      seen.checked.push(plan);
    },
    store: { planOnShow, setPlanCheck, checkUnavailable },
  };
  return {
    seen,
    newRun: () => {
      current = false;
    },
    deps,
  };
};
const REPLIES = () => [check(2), check(1), check(0), check(0, { trainees: [] })];

test("the check runs every variant: its own writes replace the plan's record, and the plan's id keeps it current", async () => {
  const mine = made();
  showAnswer(mine, WHERE, POOL);
  const { seen, deps } = fakes(REPLIES());
  await checkPhase(deps, shownPlan());
  assert.equal(seen.jobs.length, 4, 'one job per variant, then the pair search');
  assert.deepEqual(seen.jobs[0]?.career.next, { robust: 'next', pair_search: true });
  assert.equal(seen.jobs[0]?.cmd, 'career');
  assert.deepEqual(shownPlan().r.robust, check(0, { trainees: [] }));
  assert.equal(shownPlan().id, mine.planState.id);
  assert.equal(seen.redraws, 2, 'drawn when the variants are done and when the pair search is in');
  assert.equal(seen.checked.length, 4);
  assert.equal(seen.checked.at(-1), shownPlan(), 'each write handed on, the last the plan on show');
});

test('a new plan stops the check, even one like it; so do a new run and a worker that no longer keeps the plan', async () => {
  // a new plan (the same content, a new id) while the second variant runs: nothing more is written or run
  showAnswer(made(), WHERE, POOL);
  const first = shownPlan(),
    next = made();
  let run = fakes(REPLIES(), (n) => n === 2 && showAnswer(next, WHERE, POOL));
  await checkPhase(run.deps, first);
  assert.equal(run.seen.jobs.length, 2);
  assert.equal(shownPlan().r.robust, undefined, 'the new plan has no check of the old one');
  assert.equal(run.seen.checked.length, 1);
  // a new run (an edit) while the first variant runs
  run = fakes(REPLIES(), (n) => n === 1 && run.newRun());
  await checkPhase(run.deps, shownPlan());
  assert.equal(run.seen.jobs.length, 1);
  assert.equal(shownPlan().r.robust, undefined);
  // the worker lost the plan: the check is unavailable, drawn, and stops
  run = fakes([{ kind: 'unavailable', error: 'no kept plan to check' }]);
  await checkPhase(run.deps, shownPlan());
  assert.equal(shownPlan().r.check_unavailable, true);
  assert.equal(run.seen.redraws, 1);
  assert.equal(run.seen.checked.length, 1);
  // a check of another pick (the plan changed under it in the worker) writes nothing
  showAnswer(made(), WHERE, POOL);
  run = fakes([{ ...check(0), plan: 'else' }]);
  await checkPhase(run.deps, shownPlan());
  assert.equal(shownPlan().r.robust, undefined);
  assert.equal(run.seen.checked.length, 0);
});

test('an effect wakes only for the signals it names: a check wakes the answer and the reference, not the career', () => {
  showAnswer(made(), WHERE, POOL);
  const counts = { view: 0, answer: 0, career: 0, reference: 0 };
  const stops = [
    on([runView], () => counts.view++),
    on([answer], () => counts.answer++),
    on([answerCareer], () => counts.career++),
    on([reference], () => counts.reference++),
  ];
  const expect = (
    what: string,
    step: () => unknown,
    view: number,
    answerRuns: number,
    careerRuns: number,
    referenceRuns: number,
  ) => {
    const before = { ...counts };
    step();
    assert.deepEqual(
      [
        counts.view - before.view,
        counts.answer - before.answer,
        counts.career - before.career,
        counts.reference - before.reference,
      ],
      [view, answerRuns, careerRuns, referenceRuns],
      what,
    );
  };
  const id = shownPlan().id;
  expect('a progress report', () => planProgresses('scoring 2 of 3 umas'), 1, 0, 0, 0);
  expect('an edit', () => runWaits(5), 1, 0, 0, 0);
  expect("the check's step (the pick unchanged)", () => setPlanCheck(id, check(1)), 0, 1, 0, 1);
  expect('a new answer with a plan', () => showAnswer(made('new'), WHERE, POOL), 0, 1, 1, 1);
  expect('the answer dropped', () => dropAnswer(), 0, 1, 1, 0);
  expect('the run over, with an answer (its batch: the one-batch test)', () => runEndsUnsearched(made()), 1, 1, 1, 0);
  expect('a load failure (status only)', () => loadFails('x'), 0, 1, 0, 0);
  for (const stop of stops) stop();
});

// a transition that writes two signals writes them in one batch: an effect naming both runs once (one per signal would
// run once each, batch or not)
test('each two-signal transition is one batch: an effect on both signals runs once', () => {
  showAnswer(made(), WHERE, POOL);
  const once = (what: string, signals: readonly ReadonlySignal<unknown>[], step: () => unknown) => {
    let runs = 0;
    const stop = on(signals, () => runs++);
    runs = 0;
    step();
    stop();
    assert.equal(runs, 1, what);
  };
  once('the run ends unsearched: the run view and the answer', [answer, runView], () => runEndsUnsearched(made()));
  once('an answer with a plan: the answer and the reference', [answer, reference], () =>
    showAnswer(made(), WHERE, POOL),
  );
  const id = shownPlan().id;
  once("a check step: the answer and the reference (the plan's)", [answer, reference], () =>
    setPlanCheck(id, check(1)),
  );
  once('the check unavailable: the answer and the reference', [answer, reference], () => checkUnavailable(id));
  runWaits(1);
  const kept = made();
  once('a kept answer drawn again: the answer and the run view', [answer, runView], () => showKeptAnswer(kept, POOL));
});

// the step after the last variant brings no pair search (`pairs: null`, the solver's lost-pairs case): the check stops
// there, judged from the plan's current record (its own write replaced the one it started from)
test('a check whose pair search brings nothing stops after one more job, not in a loop', async () => {
  showAnswer(made(), WHERE, POOL);
  const replies = Array.from({ length: 10 }, () => check(0));
  const { seen, deps } = fakes(replies);
  await checkPhase(deps, shownPlan());
  assert.equal(seen.jobs.length, 2, 'the last variant, then one step that brought no pairs');
  assert.deepEqual(shownPlan().r.robust, check(0));
  assert.equal(shownPlan().r.check_unavailable, undefined);
});

test("Compare's kept plan: found by its plan job's key, and a check of that job's plan reaches it, another job's not", () => {
  const plain = careerJob({ next: {}, no_reports: true });
  const job: CareerJob = { ...plain, skills: { ...plain.skills, targets: [{ group: 1 }] } },
    other: CareerJob = { ...job, skills: { ...job.skills, targets: [{ group: 2 }] } },
    plan = made().planState.r;
  keepComparisonPlan({ ...job, career: { ...job.career, compare: true } }, plan, 12);
  const key = planKey(job);
  assert.equal(keptPlan(key)?.r, plan, "found by the run's plan job (the comparison's flag aside)");
  assert.equal(keptPlan(planKey(other)), null);
  const checked = { ...plan, robust: check(1) };
  reviseKeptPlan(other, checked);
  assert.equal(keptPlan(key)?.r, plan, "another job's check leaves it");
  reviseKeptPlan(job, checked);
  assert.equal(keptPlan(key)?.r, checked, "its own job's check reaches it");
  assert.equal(keptPlan(key)?.ms, 12);
});

test('the answers kept: the most recent four, a recalled one counting as recent', () => {
  const kept = (key: string) => {
    showAnswer(made(key), WHERE, POOL);
    keepAnswer(key);
  };
  for (const key of ['a', 'b', 'c', 'd']) kept(key);
  assert.equal(doneOf(recallAnswer('a')).r.outcomes?.career, 'a', 'a recalled answer is the one kept under its key');
  kept('e'); // the fifth: the least recent (b: a was recalled since) goes
  assert.equal(recallAnswer('b'), null);
  for (const key of ['a', 'c', 'd', 'e']) assert.notEqual(recallAnswer(key), null, `${key} is kept`);
  assert.equal(recallAnswer('none'), null);
});

test("`searching` wakes its readers when a run starts or ends, never on a plan's progress (review-9 F4)", () => {
  const { searching } = store;
  let woke = 0;
  const stop = on([searching], () => woke++);
  runWaits(1);
  planProgresses('a candidate');
  planProgresses('another');
  runEnds();
  stop();
  assert.equal(woke, 2, 'the run starts, the run ends');
});
