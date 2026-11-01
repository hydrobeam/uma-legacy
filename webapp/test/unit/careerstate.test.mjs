// The career answer store (src/career/state.ts) and the robustness check on its plan (src/career/run.ts `checkPhase`),
// on made-up reports: each transition copies on write; the check knows its plan by id, so its own writes keep it
// running and a new plan or run stops it; the check reaches every place the plan is kept; an effect wakes only for
// the signals it names. node --test 'test/unit/*.test.*' (in webapp)
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { on } from '../../src/store.ts';
import {
  answer,
  answerCareer,
  answerFails,
  checkUnavailable,
  dropAnswer,
  keepAnswer,
  keepSavedAnswer,
  loadFails,
  newPlanId,
  planOnShow,
  planProgresses,
  planStarts,
  recallAnswer,
  reference,
  runEnds,
  runEndsWith,
  runView,
  runWaits,
  searchStarts,
  setPlanCheck,
  showAnswer,
  showKeptAnswer,
} from '../../src/career/state.ts';
import { checkPhase } from '../../src/career/run.ts';

const PLACES = '{"trainee":null,"parents":[null,null]}';
const career = (name) => ({ trainee: { id: 1001, name }, parents: [0, 1], career: name, p_all: 0.01 });
// an answer as a run makes it: the search's best career, and a plan (made now: a new id) whose pick is `pick`
const made = (pick = 'pick') => {
  const choice = career(pick);
  return {
    status: { kind: 'done', text: 'Weighed 2 careers in 0.1 s.' },
    last: { r: { results: [career('best'), choice], candidates: 2 }, job: { legacies: [] }, key: 'job', plan: PLACES },
    planState: {
      state: 'done',
      id: newPlanId(),
      r: { choice, options: [choice], v0: 9, outcomes: { career: pick } },
      ms: 5,
      job: { cmd: 'career', next: {}, legacies: [] },
    },
  };
};
const WHERE = { base: 'base', places: { trainee: null, parents: [null, null] } };
const check = (pending, pairs = null) => ({ plan: 'pick', pending, stable: true, contenders: [], pairs });

// each transition: what it does to the record, and the fields it replaces (the rest stay the same objects)
const RUN_STEPS = [
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
    for (const key of Object.keys(after))
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
  showAnswer(first, WHERE);
  const STEPS = [
    ['showAnswer', () => showAnswer(second, WHERE), () => assert.equal(answer.value, second)],
    [
      'setPlanCheck',
      () => setPlanCheck(second.planState.id, check(2)),
      (a, b) => {
        assert.deepEqual(a.planState.r.robust, check(2));
        assert.equal(a.last, b.last, 'the search result stays the same object');
        assert.equal(a.status, b.status);
        assert.equal(a.planState.job, b.planState.job);
        assert.equal(a.planState.r.options, b.planState.r.options, 'only the plan report is copied, not its rows');
        assert.equal(a.planState.r.choice, b.planState.r.choice);
      },
    ],
    [
      'checkUnavailable',
      () => checkUnavailable(second.planState.id),
      (a, b) => {
        assert.equal(a.planState.r.check_unavailable, true);
        assert.equal(a.planState.r.robust, b.planState.r.robust);
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
    ['runEndsWith', () => runEndsWith(made()), (a) => assert.equal(a.last.key, 'job')],
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
  const now = answer.value;
  assert.throws(() => {
    now.last.r.results[0].p_all = 1;
  }, TypeError);
  assert.throws(() => {
    now.planState.r.options.push(career('x'));
  }, TypeError);
  assert.throws(() => {
    now.status.text = 'x';
  }, TypeError);
  assert.ok(!Object.isFrozen(now.last.job), 'a job stays as the run built it');
});

test('a kept answer comes back as it was kept, with the run over in the same batch; a saved one gets a new plan id', () => {
  const kept = made();
  showAnswer(kept, WHERE);
  keepAnswer('back');
  showAnswer(made('other'), WHERE);
  runWaits(1);
  let runs = 0;
  const stop = on([answer, runView], () => runs++);
  const hit = recallAnswer('back');
  assert.equal(hit, kept);
  showKeptAnswer(hit);
  assert.equal(answer.value, kept);
  assert.equal(runView.value.searching, false);
  assert.equal(runs, 1, 'one effect run for the answer and the run view together (batch)');
  stop();
  assert.equal(recallAnswer('none'), null);
  // an answer stored by the last visit, its plan id from that visit: a plan of this page may have the same id
  const saved = structuredClone(made());
  saved.planState.id = 1;
  keepSavedAnswer('saved', saved);
  const loaded = recallAnswer('saved');
  assert.ok(loaded.planState.id > kept.planState.id, 'a new id, past every one given before');
  assert.ok(Object.isFrozen(loaded.planState.r));
});

test('the check writes its result into the answer on show, the answers kept and the reference: wherever the plan is', () => {
  const kept = made(),
    other = made('other');
  showAnswer(other, WHERE);
  keepAnswer('other');
  showAnswer(kept, WHERE);
  keepAnswer('mine');
  assert.equal(reference.value.planId, kept.planState.id);
  setPlanCheck(kept.planState.id, check(0, { trainees: [] }));
  assert.deepEqual(recallAnswer('mine').planState.r.robust, check(0, { trainees: [] }), 'the answers cache has it');
  assert.deepEqual(reference.value.result.robust, check(0, { trainees: [] }), 'the reference has it');
  assert.equal(reference.value.result, answer.value.planState.r);
  assert.equal(recallAnswer('other'), other, "another plan's answer is untouched");
  // the kept answer comes back checked: nothing left to resume
  showAnswer(made('third'), WHERE);
  showKeptAnswer(recallAnswer('mine'));
  assert.deepEqual(answer.value.planState.r.robust, check(0, { trainees: [] }));
  checkUnavailable(kept.planState.id);
  assert.equal(recallAnswer('mine').planState.r.check_unavailable, true);
  assert.throws(() => setPlanCheck(other.planState.id, check(0)), /on show/, 'a plan not on show is not revised');
});

// the check's fakes: `replies` answered in turn, `during(call)` run while call n waits on the worker
const fakes = (replies, during = () => undefined) => {
  const seen = { jobs: [], redraws: 0, checked: [] };
  let current = true;
  return {
    seen,
    endRun: () => {
      current = false;
    },
    deps: {
      runIsCurrent: () => current,
      runCheck: async (job) => {
        seen.jobs.push(job);
        during(seen.jobs.length);
        return { r: { next: { robust: replies.shift() } } };
      },
      redraw: () => seen.redraws++,
      checked: (plan) => seen.checked.push(plan),
      store: { planOnShow, setPlanCheck, checkUnavailable },
    },
  };
};
const REPLIES = () => [check(2), check(1), check(0), check(0, { trainees: [] })];

test("the check runs every variant: its own writes replace the plan's record, and the plan's id keeps it current", async () => {
  const mine = made();
  showAnswer(mine, WHERE);
  const { seen, deps } = fakes(REPLIES());
  await checkPhase(deps, answer.value.planState);
  assert.equal(seen.jobs.length, 4, 'one job per variant, then the pair search');
  assert.deepEqual(seen.jobs[0].next, { robust: 'next', pair_search: true });
  assert.equal(seen.jobs[0].cmd, 'career');
  assert.deepEqual(answer.value.planState.r.robust, check(0, { trainees: [] }));
  assert.equal(answer.value.planState.id, mine.planState.id);
  assert.equal(seen.redraws, 2, 'drawn when the variants are done and when the pair search is in');
  assert.equal(seen.checked.length, 4);
  assert.equal(seen.checked.at(-1), answer.value.planState, 'each write handed on, the last the plan on show');
});

test('a new plan stops the check, even one like it; so do a new run and a worker that no longer keeps the plan', async () => {
  // a new plan (the same content, a new id) while the second variant runs: nothing more is written or run
  showAnswer(made(), WHERE);
  const first = answer.value.planState,
    next = made();
  let run = fakes(REPLIES(), (n) => n === 2 && showAnswer(next, WHERE));
  await checkPhase(run.deps, first);
  assert.equal(run.seen.jobs.length, 2);
  assert.equal(answer.value.planState.r.robust, undefined, 'the new plan has no check of the old one');
  assert.equal(run.seen.checked.length, 1);
  // a new run (an edit) while the first variant runs
  run = fakes(REPLIES(), (n) => n === 1 && run.endRun());
  await checkPhase(run.deps, answer.value.planState);
  assert.equal(run.seen.jobs.length, 1);
  assert.equal(answer.value.planState.r.robust, undefined);
  // the worker lost the plan: the check is unavailable, drawn, and stops
  run = fakes([{ ...check(0), unavailable: true }]);
  await checkPhase(run.deps, answer.value.planState);
  assert.equal(answer.value.planState.r.check_unavailable, true);
  assert.equal(run.seen.redraws, 1);
  assert.equal(run.seen.checked.length, 1);
  // a check of another pick (the plan changed under it in the worker) writes nothing
  showAnswer(made(), WHERE);
  run = fakes([{ ...check(0), plan: 'else' }]);
  await checkPhase(run.deps, answer.value.planState);
  assert.equal(answer.value.planState.r.robust, undefined);
  assert.equal(run.seen.checked.length, 0);
});

test('an effect wakes only for the signals it names: a check wakes the answer and the reference, not the career', () => {
  showAnswer(made(), WHERE);
  const counts = { view: 0, answer: 0, career: 0, reference: 0 };
  const stops = [
    on([runView], () => counts.view++),
    on([answer], () => counts.answer++),
    on([answerCareer], () => counts.career++),
    on([reference], () => counts.reference++),
  ];
  const expect = (what, step, view, answerRuns, careerRuns, referenceRuns) => {
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
  const id = answer.value.planState.id;
  expect('a progress report', () => planProgresses('scoring 2 of 3 umas'), 1, 0, 0, 0);
  expect('an edit', () => runWaits(5), 1, 0, 0, 0);
  expect("the check's step (the pick unchanged)", () => setPlanCheck(id, check(1)), 0, 1, 0, 1);
  expect('a new answer with a plan', () => showAnswer(made('new'), WHERE), 0, 1, 1, 1);
  expect('the answer dropped', () => dropAnswer(), 0, 1, 1, 0);
  expect('the run over, with an answer (one batch)', () => runEndsWith(made()), 1, 1, 1, 0);
  expect('a load failure (status only)', () => loadFails('x'), 0, 1, 0, 0);
  for (const stop of stops) stop();
});
