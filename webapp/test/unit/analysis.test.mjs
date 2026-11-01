// The veteran analysis' comparison (src/career/analysis/compare.ts `createComparison`) on the real analysis store
// (career/analysis/state.ts) with a fake engine (each job a promise the test settles; a restart leaves a job in flight
// unanswered, as engine.ts does), a fake clock, and a fake answer on show. What the store's one record goes through
// is read by an effect (`writes`): every late reply of a stale chain writes nothing, the 0.8 s timers start nothing
// after a stop, each phase is one write, the run machine's hooks and a comparison never interleave their writes, the
// same career keeps her draft (F6), quick and exact share one baseline (F5), and Compare keeps a plan only for a refit.
// node --test 'test/unit/*.test.*' (in webapp)
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { effect, signal } from '@preact/signals-core';
import { STAND_MS, createComparison } from '../../src/career/analysis/compare.ts';
import * as store from '../../src/career/analysis/state.ts';
import { fakeClock, flush } from './fakes.mjs';

const { simulation } = store;

// ---------- the fakes
function fakeEngine() {
  const engine = { jobs: [], restarts: 0, saves: 0 };
  engine.run = (job) => new Promise((resolve) => engine.jobs.push({ job, resolve, live: true }));
  engine.restart = () => {
    engine.restarts++;
    for (const j of engine.jobs) j.live = false; // (in flight: never answers)
  };
  engine.saveState = () => engine.saves++;
  // answer job n (from 0) with the report r; a killed job never answers
  engine.answer = (n, r, ms = 100) => {
    const j = engine.jobs[n];
    if (!j.live) return;
    j.live = false;
    j.resolve({ r, ms });
  };
  return engine;
}
const career = (name, parents = [0, 1], p_all = 0.01) => ({
  trainee: { id: 1001, name, outfits: [] },
  parents,
  career: name,
  p_all,
});
const node = (sparks = {}) => ({ chara: 7, sparks, pinks: {}, wins: [] });
// the page: the career on show `shown` (its search job `job`, `fix` set when a parent is fixed), and the plan for it
function page({ fix = [], planFor = 'shown' } = {}) {
  const clock = fakeClock(),
    engine = fakeEngine(),
    kept = [],
    shown = career('shown');
  const job = { cmd: 'career', targets: [{ group: 1 }], legacies: [{ v: 0 }, { v: 1 }], trainees: [], fix };
  const plan = {
    state: 'done',
    id: 1,
    ms: 10,
    job: { ...job, next: {} },
    r: { v0: 20, fit: [[1]], outcomes: { career: planFor }, options: [{ ...career('next'), chosen: true }] },
  };
  const answer = signal({
    status: { kind: 'done', text: '' },
    last: { job, r: { results: [shown] } },
    planState: plan,
  });
  const deps = {
    engine,
    clock,
    answers: { answer, keepComparisonPlan: (job, r, ms) => kept.push({ job, r, ms }) },
    store,
    page: {
      shown: () => shown,
      searching: () => deps.searching,
      targets: () => job.targets,
      baseKey: (j) => JSON.stringify({ ...j, fix: [] }),
      isFull: (nd) => (nd.sparks[1] || 0) > 0,
      planTop: 6,
      draftOf: () => ({ ...node({ 1: 1 }), borrowed: false, parents: [node(), node()], winAt: {} }),
      roll: () => ({ sparks: { 1: 3 }, bought: {}, wins: [], pinks: {}, apt: null, blues: {}, greens: {} }),
    },
    searching: false,
  };
  const c = createComparison(deps);
  c.show(shown);
  // every record the store publishes from here on
  const writes = [];
  const stop = effect(() => {
    writes.push(simulation.value);
  });
  writes.length = 0;
  return { c, deps, engine, clock, answer, kept, shown, writes, stop };
}
const sent = (p) => p.engine.jobs.length;
const states = (p) => p.writes.map((w) => w.state);
// the exact comparison's three jobs after the baseline: the best career with her, the plan with her (the plan on
// show is today's), answered
async function exactAnswers(p, from, { v0WithHer = 15 } = {}) {
  p.engine.answer(from, { results: [career('withHer', [0, 2], 0.02)] });
  await flush();
  p.engine.answer(from + 1, {
    results: [],
    next: { v0: v0WithHer, options: [{ ...career('next2', [0, 2]), chosen: true }] },
  });
  await flush();
}

// ---------- the phases: one write each
test('Compare: one write per phase, the baseline from the run (no places), the figures at the end', async () => {
  const p = page();
  p.c.compare();
  assert.deepEqual(states(p), ['busy'], 'the start: one write');
  await flush();
  assert.equal(p.writes.length, 2, 'the baseline is known: one write');
  assert.equal(simulation.peek().baseline.career, 'shown', "the run's own best (no places set): no job");
  assert.equal(sent(p), 1, 'the best career with her');
  assert.deepEqual(p.engine.jobs[0].job.fix, [2], 'her legacy index is after every veteran');
  await exactAnswers(p, 0);
  assert.deepEqual(states(p), ['busy', 'busy', 'done'], 'the end: one write');
  const done = simulation.peek();
  assert.deepEqual(done.v0, { now: 20, withHer: 15 });
  assert.equal(done.fixedFit, true, "priced under today's plan's fit");
  assert.equal(p.engine.jobs[1].job.next.fit[0][0], 1, "the plan's fit passed on");
  assert.equal(p.kept.length, 0, 'a plan under today’s fit is not kept for Add');
  assert.equal(p.engine.saves, 1);
  assert.ok(Object.isFrozen(done) && !Object.isFrozen(done.draft), 'the record frozen, her draft not (step 10)');
  p.stop();
});

test('Compare with no fit on show: the refit with her is kept, so Add takes it as it is', async () => {
  const p = page();
  p.answer.value = {
    ...p.answer.peek(),
    planState: { ...p.answer.peek().planState, r: { ...p.answer.peek().planState.r, fit: [] } },
  };
  p.c.compare();
  await flush();
  await exactAnswers(p, 0);
  assert.equal(p.kept.length, 1, 'the refit kept');
  assert.equal(p.engine.jobs[1].job.next.fit, undefined);
  assert.equal(simulation.peek().fixedFit, false);
  p.stop();
});

// ---------- staleness: late replies, timers, restarts
test("the run machine's hooks and a comparison never interleave: a stale chain's late replies write nothing", async () => {
  for (const hook of ['stop', 'outdate', 'show']) {
    const p = page({ fix: [0] }); // a parent fixed: the baseline is a job of its own
    p.c.compare();
    await flush();
    assert.equal(sent(p), 1, 'the baseline job');
    if (hook === 'show') {
      p.c.outdate(); // (a run outdates first, then shows its answer)
      p.c.show(career('other'));
    } else p.c[hook]();
    const before = p.writes.length,
      shownNow = simulation.peek();
    p.engine.answer(0, { results: [career('base')] });
    await flush();
    assert.equal(sent(p), 1, `${hook}: no job after the baseline`);
    assert.equal(p.writes.length, before, `${hook}: the late baseline writes nothing`);
    assert.equal(simulation.peek(), shownNow);
    p.stop();
  }
  // a roll's quick chain, the same after its baseline; the exact one after its first job
  for (const start of ['roll', 'compare']) {
    const p = page({ fix: start === 'roll' ? [0] : [] });
    p.c[start]();
    await flush();
    p.c.outdate();
    p.c.show(career('other'));
    const before = p.writes.length;
    p.engine.answer(0, { results: [career('x', [0, 2])] });
    await flush();
    assert.equal(sent(p), 1, `${start}: nothing sent after the stale reply`);
    assert.equal(p.writes.length, before, `${start}: and nothing written`);
    p.stop();
  }
  // a late answer of the exact comparison's last job, after a new answer is shown
  const p = page();
  p.c.compare();
  await flush();
  p.engine.answer(0, { results: [career('withHer', [0, 2], 0.02)] });
  await flush();
  p.c.outdate();
  p.c.show(career('other'));
  const before = p.writes.length;
  p.engine.answer(1, { results: [], next: { v0: 1, options: [] } });
  await flush();
  assert.equal(p.writes.length, before, "the old comparison's figures land nowhere");
  assert.equal(simulation.peek().state, 'ready');
  assert.equal(simulation.peek().forKey, 'other');
  p.stop();
});

test('a roll: quick figures, the 0.8 s stand, then the exact check behind them; a stop before the stand starts none', async () => {
  for (const stopFirst of [false, true]) {
    const p = page();
    p.c.roll();
    assert.equal(simulation.peek().draft.sparks[1], 3, 'her draft rolled (in place)');
    assert.deepEqual(states(p), ['busy'], 'the roll and the quick start: one write');
    await flush(); // (the baseline: the run's own)
    assert.equal(sent(p), 1, "the roll's saving asked of the plan");
    assert.deepEqual(p.engine.jobs[0].job.next.roll, [3]);
    p.engine.answer(0, { results: [], next: { roll: { y: 2 } } });
    await flush();
    const quick = simulation.peek();
    assert.equal(quick.state, 'done');
    assert.equal(quick.standing, true, 'it saves careers: it stands, checking');
    assert.deepEqual(quick.v0, { now: 20, withHer: 18 });
    assert.deepEqual(p.clock.pending(), ['once']);
    if (stopFirst) {
      p.c.stop();
      p.clock.advance(STAND_MS);
      await flush();
      assert.equal(sent(p), 1, 'the stand after a stop starts nothing');
      assert.equal(simulation.peek().standing, false);
    } else {
      p.clock.advance(STAND_MS);
      assert.equal(simulation.peek().refining, true, 'the exact check runs behind the quick figures');
      assert.equal(simulation.peek().standing, false);
      assert.equal(simulation.peek().state, 'done');
      await flush(); // (the baseline: the run's own)
      assert.equal(sent(p), 2);
      await exactAnswers(p, 1, { v0WithHer: 17 });
      assert.equal(simulation.peek().quick, false);
      assert.equal(simulation.peek().refining, false);
      assert.deepEqual(simulation.peek().v0, { now: 20, withHer: 17 });
    }
    p.stop();
  }
});

test("a roll's zero: the best career with her, provisional, then checked after 0.8 s; a reroll meanwhile ignores the old", async () => {
  const p = page();
  p.c.roll();
  await flush();
  p.engine.answer(0, { results: [], next: { roll: { y: 0 } } });
  await flush();
  assert.equal(simulation.peek().standing, false);
  assert.equal(sent(p), 2, 'the best career with her');
  p.engine.answer(1, { results: [career('withHer', [0, 2])] });
  await flush();
  assert.equal(simulation.peek().provisional, true);
  assert.equal(simulation.peek().r.results[0].career, 'withHer');
  // a reroll before the stand: the old chain's timer finds the token moved
  p.c.roll();
  p.clock.advance(STAND_MS);
  await flush();
  assert.equal(sent(p), 3, "only the new roll's job");
  assert.ok(!simulation.peek().refining, 'no check started by the old timer');
  assert.equal(p.engine.restarts, 0, 'a quick comparison is never killed (its search finishes, ignored)');
  p.stop();
  // left standing, the zero is checked after 0.8 s
  const q = page();
  q.c.roll();
  await flush();
  q.engine.answer(0, { results: [], next: { roll: { y: 0 } } });
  await flush();
  q.engine.answer(1, { results: [career('withHer', [0, 2])] });
  await flush();
  q.clock.advance(STAND_MS);
  assert.equal(simulation.peek().refining, true);
  assert.equal(simulation.peek().provisional, true, 'shown as checking until the exact count lands');
  q.stop();
});

test("a timer of a roll rolled over: the next roll's figures are not checked before their own 0.8 s", async () => {
  for (const y of [2, 0]) {
    const p = page();
    const quick = async (n) => {
      p.c.roll();
      await flush();
      p.engine.answer(n, { results: [], next: { roll: { y } } });
      await flush();
      if (y === 0) {
        p.engine.answer(n + 1, { results: [career('withHer', [0, 2])] });
        await flush();
      }
    };
    await quick(0);
    p.clock.advance(STAND_MS / 2);
    await quick(y ? 1 : 2);
    p.clock.advance(STAND_MS / 2); // the first roll's timer
    assert.ok(!simulation.peek().refining, `y ${y}: the first roll's timer starts nothing`);
    p.clock.advance(STAND_MS / 2);
    assert.equal(simulation.peek().refining, true, `y ${y}: the second's own`);
    p.stop();
  }
});

test('cancel: Compare again kills an exact comparison in flight, never a quick search; a check behind a roll is killed too', async () => {
  const p = page();
  p.c.compare();
  await flush();
  p.c.compare();
  assert.equal(p.engine.restarts, 1, 'the exact comparison in flight: the worker goes');
  assert.equal(simulation.peek().state, 'busy');
  p.stop();
  // a quick search in flight: Compare waits behind it
  const q = page();
  q.c.roll();
  q.c.compare();
  assert.equal(q.engine.restarts, 0);
  q.stop();
  // a check behind a roll's quick figures: killed, the figures stand
  const r = page();
  r.c.roll();
  await flush();
  r.engine.answer(0, { results: [], next: { roll: { y: 2 } } });
  await flush();
  r.clock.advance(STAND_MS);
  assert.equal(simulation.peek().refining, true);
  r.c.roll();
  assert.equal(r.engine.restarts, 1);
  r.stop();
});

test('nothing starts while a run is at work, or with no career on show', () => {
  const p = page();
  p.deps.searching = true;
  p.c.compare();
  p.c.roll();
  assert.equal(sent(p), 0);
  assert.equal(p.writes.length, 0);
  p.stop();
});

// ---------- F5: one baseline for quick and exact, with a parent fixed
test("F5: with a parent fixed, the roll's baseline and Compare's are one job's (today's best over everything)", async () => {
  const p = page({ fix: [0] });
  p.c.roll();
  await flush();
  assert.equal(sent(p), 1, 'the baseline first');
  assert.deepEqual(p.engine.jobs[0].job.fix, [], 'the best over everything, no place fixed');
  p.engine.answer(0, { results: [career('everything', [3, 4], 0.05)] });
  await flush();
  assert.equal(simulation.peek().baseline.career, 'everything');
  // a new answer for the same career, then Compare: the baseline cached under the same job, no job for it
  p.c.outdate();
  p.c.show(p.shown);
  assert.equal(simulation.peek().baseline, null);
  p.c.compare();
  await flush();
  const jobs = p.engine.jobs.slice(2).map((j) => j.job);
  assert.ok(
    jobs.every((j) => j.fix.length === 1),
    'no second baseline job',
  );
  assert.equal(simulation.peek().baseline.career, 'everything');
  p.stop();
});

// ---------- F6: the same career keeps her draft
test("F6: an answer showing the same career keeps the draft she was entered as; another career's resets it", () => {
  const p = page();
  const draft = simulation.peek().draft;
  store.draftRef.edit((now) => ({ sparks: { ...now.sparks, 9: 2 } })); // entered by hand
  p.c.show(p.shown);
  assert.equal(simulation.peek().draft, draft, 'the same career: the same draft');
  assert.equal(simulation.peek().state, 'ready', 'her comparison starts over');
  p.c.show(career('other'));
  assert.notEqual(simulation.peek().draft, draft);
  assert.equal(simulation.peek().draft.sparks[9], undefined);
  p.stop();
});

test("her card's tab and the races' fold outlive her record: a new career, no career, her draft added", () => {
  const p = page();
  store.showTab('skills');
  store.toggleRaces();
  p.c.show(career('other'));
  p.c.clear();
  p.c.show(p.shown);
  store.draftAdded();
  assert.equal(store.detailsTab.peek(), 'skills');
  assert.equal(store.racesShown.peek(), true);
  store.toggleRaces();
  store.showTab('insp');
  assert.equal(store.racesShown.peek(), false);
  p.stop();
});

test('an edit of her draft: a new record over the same draft, stale only with figures on show', async () => {
  const p = page();
  const draft = simulation.peek().draft;
  store.draftRef.edit(() => ({ wins: [3] }));
  assert.deepEqual(draft.wins, [3], 'written into her draft');
  assert.equal(simulation.peek().stale, false, 'no figures: nothing stale');
  assert.equal(simulation.peek().draft, draft);
  assert.equal(p.writes.length, 1, 'a write: the views draw her again');
  p.c.compare();
  await flush();
  await exactAnswers(p, 0);
  store.draftRef.edit(() => ({}));
  assert.equal(simulation.peek().stale, true);
  p.c.stop();
  assert.equal(simulation.peek().stale, true, 'a stop leaves the figures, marked stale');
  const n = p.writes.length;
  p.c.stop();
  assert.equal(p.writes.length, n, 'a stop with nothing to stop writes nothing');
  p.stop();
});

test('an editor opened on her draft edits the draft on show when the edit lands, never one it captured', () => {
  const p = page();
  const at = store.draftRef; // (an editor holds the ref, as the G1 wins window does while open)
  p.c.show(career('other')); // another career: a new draft
  const now = simulation.peek().draft;
  at.edit((d) => ({ sparks: { ...d.sparks, 5: 1 } }));
  assert.equal(at.node(), now);
  assert.equal(now.sparks[5], 1);
  store.draftAdded();
  const n = p.writes.length;
  at.edit(() => ({ wins: [1] }));
  assert.equal(p.writes.length, n, 'no draft: an edit lands nowhere');
  p.stop();
});
