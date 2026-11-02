// The veteran analysis' comparison (src/career/analysis/compare.ts `createComparison`) on the real analysis store
// (career/analysis/state.ts) with a fake engine (each job a promise the test settles; a restart leaves a job in flight
// unanswered, as engine.ts does), a fake clock, and a fake answer on show. What the store's one record goes through
// is read by an effect (`writes`): every late reply of a stale chain writes nothing, the 0.8 s timers start nothing
// after a stop, each phase is one write, the run machine's hooks and a comparison never interleave their writes, the
// same career keeps her draft (F6), quick and exact share one baseline (F5), and Compare keeps a plan only for a refit.
// npx vitest run (in webapp)
import assert from 'node:assert/strict';
import { test, vi } from 'vitest';
import { effect, signal } from '@preact/signals';
import { STAND_MS, createComparison, savedWhere } from '../../src/career/analysis/compare.ts';
import * as store from '../../src/career/analysis/state.ts';
import * as vs from '../../src/veterans/store.ts';
import { on } from '../../src/store.ts';
import { fakeClock, flush, memory } from './fakes.ts';

vi.mock('#page-data', () => import('./fakepage.ts')); // run.ts reads the solver's fingerprints from it (engine.ts stateKey)

// the veterans store on a Map (Add writes into it)
vs.initVets({ storage: () => memory(), hidden: () => true, afterFrame: (f) => f() });

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
// her grades: A at every aptitude, so her roll draws a pink (every veteran has one), turf here
const allA = { distance: [7, 7, 7, 7], surface: [7, 7], style: [7, 7, 7, 7] };
const career = (name, parents = [0, 1], p_all = 0.01) => ({
  trainee: { id: 1001, name, outfits: [] },
  parents,
  career: name,
  p_all,
  aptitudes: { base: allA, pink_gen: [{ aptitude: 11, p: 1 }], start: { kind: 'known', start: allA, fire: [] } },
});
const node = (sparks = {}) => ({ chara: 7, sparks, pinks: { 11: 1 }, wins: [] });
// the page: the career on show `shown` (its search job `job`, `fix` set when a parent is fixed), and the plan for it
function page({ fix = [], planFor = 'shown' } = {}) {
  store.analysisClears(); // (the store is the module's: no test sees the last one's draft)
  const parents = [node(), node()], // her parents' records now (draftOf copies them)
    clock = fakeClock(),
    engine = fakeEngine(),
    kept = [],
    shown = career('shown');
  const job = {
    cmd: 'career',
    skills: { targets: [{ group: 1 }] },
    deck: {},
    career: { legacies: [{ v: 0 }, { v: 1 }], trainees: [], fix },
  };
  const plan = {
    state: 'done',
    id: 1,
    ms: 10,
    job: { ...job, career: { ...job.career, next: {} } },
    r: { v0: 20, fit: [[1]], outcomes: { career: planFor }, options: [{ ...career('next'), chosen: true }] },
  };
  const answer = signal({
    status: { kind: 'done', text: '' },
    last: { job, r: { kind: 'search', results: [shown] } },
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
      targets: () => job.skills.targets,
      baseKey: (j) => JSON.stringify({ ...j, career: { ...j.career, fix: [] } }),
      isFull: (nd) => (nd.sparks[1] || 0) > 0,
      planTop: 6,
      // her first result lands target 1 at 1 star, every later one (a roll) at 3
      draftOf: () => ({
        ...node({ 1: deps.draws++ ? 3 : 1 }),
        borrowed: false,
        parents: parents.map((x) => structuredClone(x)),
        winAt: {},
      }),
      draft: () => vs.draft.peek(),
    },
    searching: false,
    draws: 0,
  };
  const c = createComparison(deps);
  c.show(shown);
  // every record the store publishes from here on
  const writes = [];
  const stop = effect(() => {
    writes.push(simulation.value);
  });
  writes.length = 0;
  return { c, deps, engine, clock, answer, kept, shown, writes, stop, parents };
}
const sent = (p) => p.engine.jobs.length;
const states = (p) => p.writes.map((w) => w.state);
// the exact comparison's three jobs after the baseline: the best career with her, the plan with her (the plan on
// show is today's), answered
async function exactAnswers(p, from, { v0WithHer = 15 } = {}) {
  p.engine.answer(from, { kind: 'search', results: [career('withHer', [0, 2], 0.02)] });
  await flush();
  p.engine.answer(from + 1, {
    kind: 'search',
    results: [],
    next: { kind: 'plan', v0: v0WithHer, options: [{ ...career('next2', [0, 2]), chosen: true }] },
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
  assert.deepEqual(p.engine.jobs[0].job.career.fix, [2], 'her legacy index is after every veteran');
  await exactAnswers(p, 0);
  assert.deepEqual(states(p), ['busy', 'busy', 'done'], 'the end: one write');
  const done = simulation.peek();
  assert.deepEqual(done.v0, { now: 20, withHer: 15 });
  assert.equal(done.fixedFit, true, "priced under today's plan's fit");
  assert.equal(p.engine.jobs[1].job.career.next.fit[0][0], 1, "the plan's fit passed on");
  assert.equal(p.kept.length, 0, 'a plan under today’s fit is not kept for Add');
  assert.equal(p.engine.saves, 1);
  assert.ok(Object.isFrozen(done), 'the record frozen');
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
  assert.equal(p.engine.jobs[1].job.career.next.fit, undefined);
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
    p.engine.answer(0, { kind: 'search', results: [career('base')] });
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
    p.engine.answer(0, { kind: 'search', results: [career('x', [0, 2])] });
    await flush();
    assert.equal(sent(p), 1, `${start}: nothing sent after the stale reply`);
    assert.equal(p.writes.length, before, `${start}: and nothing written`);
    p.stop();
  }
  // a late answer of the exact comparison's last job, after a new answer is shown
  const p = page();
  p.c.compare();
  await flush();
  p.engine.answer(0, { kind: 'search', results: [career('withHer', [0, 2], 0.02)] });
  await flush();
  p.c.outdate();
  p.c.show(career('other'));
  const before = p.writes.length;
  p.engine.answer(1, { kind: 'search', results: [], next: { kind: 'plan', v0: 1, options: [] } });
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
    assert.equal(vs.draft.peek().sparks[1], 3, 'her draft rolled');
    assert.deepEqual(states(p), ['busy'], 'the roll and the quick start: one write');
    await flush(); // (the baseline: the run's own)
    assert.equal(sent(p), 1, "the roll's saving asked of the plan");
    assert.deepEqual(p.engine.jobs[0].job.career.next.roll, [3]);
    p.engine.answer(0, { kind: 'roll', roll: { kind: 'answer', outcome: { kind: 'saving', y: 2 } } });
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
  p.engine.answer(0, { kind: 'roll', roll: { kind: 'answer', outcome: { kind: 'saving', y: 0 } } });
  await flush();
  assert.equal(simulation.peek().standing, false);
  assert.equal(sent(p), 2, 'the best career with her');
  p.engine.answer(1, { kind: 'search', results: [career('withHer', [0, 2])] });
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
  q.engine.answer(0, { kind: 'roll', roll: { kind: 'answer', outcome: { kind: 'saving', y: 0 } } });
  await flush();
  q.engine.answer(1, { kind: 'search', results: [career('withHer', [0, 2])] });
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
      p.engine.answer(n, { kind: 'roll', roll: { kind: 'answer', outcome: { kind: 'saving', y } } });
      await flush();
      if (y === 0) {
        p.engine.answer(n + 1, { kind: 'search', results: [career('withHer', [0, 2])] });
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
  r.engine.answer(0, { kind: 'roll', roll: { kind: 'answer', outcome: { kind: 'saving', y: 2 } } });
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
  assert.deepEqual(p.engine.jobs[0].job.career.fix, [], 'the best over everything, no place fixed');
  p.engine.answer(0, { kind: 'search', results: [career('everything', [3, 4], 0.05)] });
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
    jobs.every((j) => j.career.fix.length === 1),
    'no second baseline job',
  );
  assert.equal(simulation.peek().baseline.career, 'everything');
  p.stop();
});

// ---------- F6: the same career keeps her draft
test("F6: an answer showing the same career keeps the draft she was entered as; another career's resets it", () => {
  const p = page();
  vs.draftRef().edit((now) => ({ sparks: { ...now.sparks, 9: 2 } })); // entered by hand
  const draft = vs.draft.peek();
  p.c.show(p.shown);
  assert.equal(vs.draft.peek(), draft, 'the same career: the same draft');
  assert.equal(simulation.peek().state, 'ready', 'her comparison starts over');
  p.c.show(career('other'));
  assert.notEqual(vs.draft.peek(), draft);
  assert.equal(vs.draft.peek().sparks[9], undefined);
  p.stop();
});

test("her card's tab and the races' fold outlive her record: a new career, no career, her draft added", () => {
  const p = page();
  store.showTab('skills');
  store.toggleRaces();
  p.c.show(career('other'));
  p.c.clear();
  p.c.show(p.shown);
  store.draftJoins();
  assert.equal(store.detailsTab.peek(), 'skills');
  assert.equal(store.racesShown.peek(), true);
  store.toggleRaces();
  store.showTab('insp');
  assert.equal(store.racesShown.peek(), false);
  p.stop();
});

test("an edit of her draft (the veterans store's draftRef, the entry's effect): stale only with figures on show", async () => {
  const p = page();
  const marks = on([vs.draft], store.draftChanged);
  vs.draftRef().edit(() => ({ wins: [3] }));
  assert.deepEqual(vs.draft.peek().wins, [3], 'her draft, a new record');
  assert.equal(p.writes.length, 0, 'no figures: nothing to mark');
  p.c.compare();
  await flush();
  await exactAnswers(p, 0);
  assert.equal(simulation.peek().stale, false);
  vs.draftRef().edit(() => ({ wins: [] }));
  assert.equal(simulation.peek().stale, true, 'the figures are for her before the edit');
  p.c.stop();
  assert.equal(simulation.peek().stale, true, 'a stop leaves the figures, marked stale');
  const n = p.writes.length;
  p.c.stop();
  assert.equal(p.writes.length, n, 'a stop with nothing to stop writes nothing');
  // a roll replaces her and clears the figures in one batch: nothing marked
  p.c.roll();
  assert.equal(simulation.peek().r, null);
  assert.ok(!simulation.peek().stale);
  marks();
  p.stop();
});

// ---------- review-9 F1: an editor is bound to the draft it was opened on
test('an editor opened on her draft: once another draft replaces her (a new career, a roll, Add), its edits land nowhere', () => {
  for (const replace of ['show', 'roll', 'add']) {
    const p = page();
    const at = vs.draftRef(); // (opened now, as the G1 wins window or the "+" list is)
    if (replace === 'show') p.c.show(career('other'));
    else if (replace === 'roll') p.c.roll();
    else store.draftJoins();
    const now = vs.draft.peek();
    assert.equal(at.node(), undefined, `${replace}: her editor sees no node`);
    at.edit((d) => ({ sparks: { ...d.sparks, 5: 1 } }));
    assert.equal(vs.draft.peek(), now, `${replace}: the draft on show is untouched`);
    const fresh = vs.draftRef();
    if (now) {
      fresh.edit((d) => ({ sparks: { ...d.sparks, 5: 1 } }));
      assert.equal(vs.draft.peek().sparks[5], 1, `${replace}: an editor opened on her now edits her`);
    }
    p.stop();
  }
});

// ---------- review-9 F2: her parents follow their records
test("her parents follow their records while the same career is on show: a new answer, a roll, and Add's record", () => {
  const p = page();
  vs.draftRef().edit((d) => ({ sparks: { ...d.sparks, 9: 2 } })); // (entered by hand: kept for the same career)
  p.parents[0] = node({ 4: 3 }); // her parent #1 edited in the pool
  p.c.show(p.shown);
  assert.deepEqual(vs.draft.peek().parents[0].sparks, { 4: 3 }, 'a new answer for the same career: her parents now');
  assert.equal(vs.draft.peek().sparks[9], 2, 'what she was entered as stays');
  p.parents[1] = node({ 6: 1 });
  p.c.roll();
  assert.deepEqual(vs.draft.peek().parents[1].sparks, { 6: 1 }, 'a roll: her parents now');
  p.parents[0] = node({ 8: 2 });
  p.c.show(p.shown);
  const before = vs.vets.peek().length;
  assert.equal(store.draftJoins(), 7);
  const added = vs.vets.peek()[before];
  assert.deepEqual(
    added.parents.map((x) => x.sparks),
    [{ 8: 2 }, { 6: 1 }],
    'Add writes her parents as they are',
  );
  assert.equal(added.src, 'entered');
  assert.ok(!('apt' in added) && !('bought' in added));
  assert.equal(vs.draft.peek(), null, 'no draft after Add');
  p.stop();
});

// ---------- review-9 F5: no quick leftover on an exact comparison
test('an exact comparison after a quick one (cancelled or failed) is killed by the next Compare', async () => {
  // cancelled: Compare while the roll's quick search runs (it is left to finish), then Compare again
  const p = page();
  p.c.roll();
  await flush();
  p.c.compare();
  assert.equal(p.engine.restarts, 0, 'a quick search is never killed');
  await flush();
  p.c.compare();
  assert.equal(p.engine.restarts, 1, 'the exact comparison in flight goes, the worker with it');
  p.stop();
  // failed: the roll's baseline job refuses
  const q = page({ fix: [0] });
  q.c.roll();
  await flush();
  q.engine.answer(0, { kind: 'refusal', error: 'bad' });
  await flush();
  assert.equal(simulation.peek().state, 'error');
  q.c.compare();
  await flush();
  q.c.compare();
  assert.equal(q.engine.restarts, 1);
  q.stop();
});

// ---------- review-9's sabotages the tests missed (N1, N3-N6)
test('Compare while a comparison runs: the cancel and the new start are one write (N1)', async () => {
  const p = page();
  p.c.compare();
  await flush();
  const n = p.writes.length;
  p.c.compare();
  assert.equal(p.writes.length, n + 1);
  assert.equal(simulation.peek().state, 'busy');
  p.stop();
});

test('a roll draws her G1 wins anew: the wins placed on the last one go (N3)', () => {
  const p = page();
  vs.draftRef().edit(() => ({ wins: [3], winAt: { 3: 20 } }));
  p.c.roll();
  assert.deepEqual(vs.draft.peek().winAt, {});
  assert.deepEqual(vs.draft.peek().wins, []);
  p.stop();
});

test('an edit during the stand: the exact check behind the quick figures clears the stale mark (N5)', async () => {
  const p = page();
  const marks = on([vs.draft], store.draftChanged);
  p.c.roll();
  await flush();
  p.engine.answer(0, { kind: 'roll', roll: { kind: 'answer', outcome: { kind: 'saving', y: 2 } } });
  await flush();
  vs.draftRef().edit((d) => ({ sparks: { ...d.sparks, 9: 1 } }));
  assert.equal(simulation.peek().stale, true, 'edited under the quick figures');
  p.clock.advance(STAND_MS);
  assert.equal(simulation.peek().refining, true);
  assert.equal(simulation.peek().stale, false, 'the check works on her as she is now');
  await flush();
  await exactAnswers(p, 1);
  assert.equal(simulation.peek().stale, false, 'its figures are hers');
  marks();
  p.stop();
});

test('a stop of a running comparison: back to idle, the old figures gone (N6)', async () => {
  const p = page();
  p.c.compare();
  await flush();
  await exactAnswers(p, 0);
  assert.ok(simulation.peek().r);
  p.c.compare(); // a second comparison, with the first one's figures still in the record while busy
  p.c.stop();
  assert.equal(simulation.peek().state, 'ready');
  assert.equal(simulation.peek().r, null, 'idle: no figures');
  p.stop();
});

test('savedWhere: her as a parent of the next career; a later step named only on the same route (N4)', () => {
  const pick = (parents, name = 'Next Uma') => ({ trainee: { name }, parents });
  const route = (sides, careers) => ({ a: 'A', b: 'B', c: 'C', sides, careers });
  assert.equal(
    savedWhere({ planNow: null, planWithHer: null, pickA: pick([0, 2]), samePick: false, herIndex: 2 }),
    'Improves the next career (Next Uma).',
  );
  assert.equal(savedWhere({ planNow: null, planWithHer: null, pickA: null, samePick: false, herIndex: 2 }), '');
  const now = { v0_plan: route(['x', 'y'], [10, 8, 5]) };
  assert.equal(
    savedWhere({
      planNow: now,
      planWithHer: { v0_plan: route(['x', 'y'], [10, 3, 5]) },
      pickA: pick([0, 1]),
      samePick: true,
      herIndex: 2,
    }),
    'Improves a later B career; the next career stays Next Uma.',
    'the same trio on the same sides: the step that fell most',
  );
  assert.equal(
    savedWhere({
      planNow: now,
      planWithHer: { v0_plan: route(['x', 'z'], [10, 3, 5]) },
      pickA: pick([0, 1]),
      samePick: false,
      herIndex: 2,
    }),
    'Improves a later career; the next career is Next Uma.',
    'another side: another route, no step named',
  );
  // a step that cannot land is written as null (an infinite cost, serde's null): no step is named for it
  assert.equal(
    savedWhere({
      planNow: now,
      planWithHer: { v0_plan: route(['x', 'y'], [10, null, 5]) },
      pickA: pick([0, 1]),
      samePick: true,
      herIndex: 2,
    }),
    'Improves a later career; the next career stays Next Uma.',
    "a null step's careers: nothing fell",
  );
});
